use crate::plugins::{Plugin, WasmPluginRunner};
use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::fs;

pub struct PluginManager {
    runner: WasmPluginRunner,
    plugins: HashMap<String, Box<dyn Plugin>>,
    plugins_dir: PathBuf,
}

impl PluginManager {
    pub fn new<P: AsRef<Path>>(plugins_dir: P) -> Result<Self> {
        Ok(Self {
            runner: WasmPluginRunner::new()?,
            plugins: HashMap::new(),
            plugins_dir: plugins_dir.as_ref().to_path_buf(),
        })
    }

    pub async fn load_all(&mut self) -> Result<()> {
        if !self.plugins_dir.exists() {
            fs::create_dir_all(&self.plugins_dir).await?;
        }

        let mut entries = fs::read_dir(&self.plugins_dir).await?;
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map_or(false, |ext| ext == "wasm") {
                if let Err(e) = self.load_plugin(&path).await {
                    tracing::warn!("Failed to load plugin {:?}: {}", path, e);
                }
            }
        }
        Ok(())
    }

    pub async fn load_plugin(&mut self, path: &Path) -> Result<()> {
        let plugin = self
            .runner
            .load_plugin(path)
            .await
            .context(format!("Failed to load plugin from {:?}", path))?;
        let name = plugin.name().to_string();
        if self.plugins.insert(name.clone(), plugin).is_some() {
            tracing::warn!(
                "Plugin '{}' already existed – replaced with the newest copy from {:?}",
                name,
                path
            );
        }
        Ok(())
    }

    pub fn list_plugins(&self) -> Vec<String> {
        self.plugins.keys().cloned().collect()
    }

    pub async fn execute_plugin(&self, name: &str, input: &str) -> Result<String> {
        if let Some(plugin) = self.plugins.get(name) {
            plugin.execute(input).await
        } else {
            Err(anyhow!("Plugin '{}' not found", name))
        }
    }
}
