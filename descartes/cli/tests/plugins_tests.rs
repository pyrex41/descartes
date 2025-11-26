use anyhow::Result;
use assert_cmd::cargo::cargo_bin_cmd;
use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

const PLUGIN_NAME: &str = "echo_plugin";

fn sample_plugin_wat() -> &'static str {
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

fn write_sample_plugin(dir: &Path) -> Result<PathBuf> {
    let wasm_bytes = wat::parse_str(sample_plugin_wat())?;
    let path = dir.join(format!("{PLUGIN_NAME}.wasm"));
    fs::write(&path, wasm_bytes)?;
    Ok(path)
}

fn cli_command(home: &Path) -> Result<Command> {
    let mut cmd = cargo_bin_cmd!("descartes");
    cmd.env("HOME", home)
        .env("NO_COLOR", "1")
        .env("CLICOLOR", "0");
    Ok(cmd)
}

#[test]
fn plugins_install_list_and_exec_flow() -> Result<()> {
    let home = TempDir::new()?;
    let plugin_source = TempDir::new()?;
    let plugin_path = write_sample_plugin(plugin_source.path())?;
    let plugin_path_str = plugin_path.to_string_lossy().to_string();

    // Install plugin from local path.
    cli_command(home.path())?
        .args(["plugins", "install", &plugin_path_str])
        .assert()
        .success();

    let installed_plugin = home
        .path()
        .join(".descartes")
        .join("plugins")
        .join(format!("{PLUGIN_NAME}.wasm"));
    assert!(
        installed_plugin.exists(),
        "Installed plugin should exist at {}",
        installed_plugin.display()
    );

    // Listing should surface the plugin name.
    let list_output = cli_command(home.path())?
        .args(["plugins", "list"])
        .output()?;
    assert!(
        list_output.status.success(),
        "List command should succeed: {:?}",
        list_output
    );
    let stdout = String::from_utf8_lossy(&list_output.stdout);
    assert!(
        stdout.contains("Installed Plugins"),
        "List output should include heading, got: {stdout}"
    );
    assert!(
        stdout.contains(PLUGIN_NAME),
        "List output should include plugin name, got: {stdout}"
    );

    // Exec should print the plugin execution result.
    let exec_output = cli_command(home.path())?
        .args(["plugins", "exec", PLUGIN_NAME, "ping"])
        .output()?;
    assert!(
        exec_output.status.success(),
        "Exec command should succeed: {:?}",
        exec_output
    );
    let exec_stdout = String::from_utf8_lossy(&exec_output.stdout);
    assert_eq!(exec_stdout.trim(), "echo: ping");

    Ok(())
}
