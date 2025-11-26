use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use std::convert::TryFrom;
use std::path::{Path, PathBuf};
use wasmtime::{Config, Engine, Linker, Memory, Module, Store};

pub mod manager;

#[async_trait]
pub trait Plugin: Send + Sync {
    fn name(&self) -> &str;
    async fn execute(&self, input: &str) -> Result<String>;
}

/// Runs WASM plugins that expose a very small interface:
/// - `memory` export for linear memory access.
/// - `alloc(size: i32) -> i32` used to reserve host-visible memory.
/// - `execute(ptr: i32, len: i32) -> i32` that returns a pointer to a
///   UTF-8 string terminated with `\0`.
/// Each invocation spins up a fresh `Store`, so plugins can do whatever they
/// need without worrying about state cleanup.
pub struct WasmPluginRunner {
    engine: Engine,
}

impl WasmPluginRunner {
    pub fn new() -> Result<Self> {
        let mut config = Config::new();
        config.async_support(true);
        let engine = Engine::new(&config)?;
        Ok(Self { engine })
    }

    pub async fn load_plugin<P: AsRef<Path>>(&self, path: P) -> Result<Box<dyn Plugin>> {
        let path = path.as_ref();
        let module = Module::from_file(&self.engine, path)?;
        let name = derive_plugin_name(path);
        Ok(Box::new(WasmPlugin {
            engine: self.engine.clone(),
            module,
            name,
            location: path.to_path_buf(),
        }))
    }
}

struct WasmPlugin {
    engine: Engine,
    module: Module,
    name: String,
    location: PathBuf,
}

#[async_trait]
impl Plugin for WasmPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    async fn execute(&self, input: &str) -> Result<String> {
        let linker: Linker<()> = Linker::new(&self.engine);
        let mut store = Store::new(&self.engine, ());
        let instance = linker
            .instantiate_async(&mut store, &self.module)
            .await
            .with_context(|| format!("Failed to instantiate plugin '{}'", self.name))?;

        let memory = instance.get_memory(&mut store, "memory").with_context(|| {
            format!("Plugin '{}' is missing a exported linear memory", self.name)
        })?;

        let alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .with_context(|| {
                format!(
                    "Plugin '{}' does not export the required alloc function",
                    self.name
                )
            })?;

        let execute = instance
            .get_typed_func::<(i32, i32), i32>(&mut store, "execute")
            .with_context(|| {
                format!(
                    "Plugin '{}' does not export the required execute function",
                    self.name
                )
            })?;

        let input_bytes = input.as_bytes();
        let input_len = i32::try_from(input_bytes.len()).context("Input exceeds 32-bit limit")?;
        let input_ptr = alloc
            .call_async(&mut store, input_len)
            .await
            .context("Plugin alloc failed")?;
        let input_offset = usize_from_i32(input_ptr)
            .with_context(|| "Plugin alloc returned a negative pointer")?;

        memory
            .write(&mut store, input_offset, input_bytes)
            .with_context(|| format!("Failed to copy input into plugin '{}' memory", self.name))?;

        let output_ptr = execute
            .call_async(&mut store, (input_ptr, input_len))
            .await
            .with_context(|| format!("Plugin '{}' execution trapped", self.name))?;

        read_zero_terminated_string(&store, &memory, output_ptr).with_context(|| {
            format!(
                "Plugin '{}' did not return a valid UTF-8 response. File: {}",
                self.name,
                self.location.display()
            )
        })
    }
}

const MAX_PLUGIN_OUTPUT: usize = 1024 * 1024; // 1 MiB safety guard

fn derive_plugin_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unnamed_plugin".to_string())
}

fn usize_from_i32(value: i32) -> Result<usize> {
    usize::try_from(value).context("Pointer is negative")
}

fn read_zero_terminated_string(store: &Store<()>, memory: &Memory, ptr: i32) -> Result<String> {
    let start = usize_from_i32(ptr)?;
    let data = memory.data(store);
    if start >= data.len() {
        bail!("Pointer {} is outside the plugin memory", start);
    }

    let mut end = start;
    let upper_bound = data.len().min(start + MAX_PLUGIN_OUTPUT + 1);
    while end < upper_bound {
        if data[end] == 0 {
            let slice = &data[start..end];
            return String::from_utf8(slice.to_vec()).context("Plugin returned non UTF-8 payload");
        }
        end += 1;
    }

    bail!(
        "Plugin response exceeded {} bytes or lacked a terminator",
        MAX_PLUGIN_OUTPUT
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::manager::PluginManager;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    fn sample_plugin() -> &'static str {
        r#"
        (module
            (memory (export "memory") 1)
            (global $heap (mut i32) (i32.const 0))

            (func $alloc (export "alloc") (param $size i32) (result i32)
                (local $old i32)
                (local.set $old (global.get $heap))
                (global.set $heap (i32.add (global.get $heap) (local.get $size)))
                (local.get $old)
            )

            (func (export "execute") (param $ptr i32) (param $len i32) (result i32)
                (local $out i32)
                (local $i i32)
                ;; "echo: " + input + nul
                (local.set $out (call $alloc (i32.add (local.get $len) (i32.const 7))))
                (i32.store8 (local.get $out) (i32.const 101)) ;; e
                (i32.store8 (i32.add (local.get $out) (i32.const 1)) (i32.const 99)) ;; c
                (i32.store8 (i32.add (local.get $out) (i32.const 2)) (i32.const 104)) ;; h
                (i32.store8 (i32.add (local.get $out) (i32.const 3)) (i32.const 111)) ;; o
                (i32.store8 (i32.add (local.get $out) (i32.const 4)) (i32.const 58)) ;; :
                (i32.store8 (i32.add (local.get $out) (i32.const 5)) (i32.const 32)) ;; space

                (local.set $i (i32.const 0))
                (block $done
                    (loop $loop
                        (br_if $done (i32.ge_u (local.get $i) (local.get $len)))
                        (i32.store8
                            (i32.add (local.get $out) (i32.add (local.get $i) (i32.const 6)))
                            (i32.load8_u (i32.add (local.get $ptr) (local.get $i))))
                        (local.set $i (i32.add (local.get $i) (i32.const 1)))
                        (br $loop)
                    )
                )

                (i32.store8
                    (i32.add (local.get $out) (i32.add (local.get $len) (i32.const 6)))
                    (i32.const 0))
                (local.get $out)
            )
        )
        "#
    }

    fn write_wasm_to(dir: &Path, name: &str) -> Result<PathBuf> {
        let wasm_bytes = wat::parse_str(sample_plugin())?;
        let path = dir.join(name);
        std::fs::write(&path, wasm_bytes)?;
        Ok(path)
    }

    #[tokio::test]
    async fn wasm_plugin_executes_real_module() -> Result<()> {
        let dir = tempdir()?;
        let wasm_path = write_wasm_to(dir.path(), "echo_plugin.wasm")?;

        let runner = WasmPluginRunner::new()?;
        let plugin = runner.load_plugin(&wasm_path).await?;
        assert_eq!(plugin.name(), "echo_plugin");

        let output = plugin.execute("ping").await?;
        assert_eq!(output, "echo: ping");
        Ok(())
    }

    #[tokio::test]
    async fn plugin_manager_loads_plugins_from_disk() -> Result<()> {
        let dir = tempdir()?;
        let wasm_path = write_wasm_to(dir.path(), "echo_plugin.wasm")?;
        // Additional noise file should be ignored
        std::fs::write(dir.path().join("README.txt"), b"ignore me")?;

        let mut manager = PluginManager::new(dir.path())?;
        manager.load_all().await?;
        let mut plugins = manager.list_plugins();
        plugins.sort();
        assert_eq!(plugins, vec!["echo_plugin".to_string()]);

        let output = manager.execute_plugin("echo_plugin", "pong").await?;
        assert_eq!(output, "echo: pong");

        // Loading same plugin again should replace existing entry without panic
        manager.load_plugin(&wasm_path).await?;

        Ok(())
    }
}
