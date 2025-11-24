/// Configuration file watcher and hot-reloading system
/// Monitors config file changes and notifies subscribers of updates

use crate::config::{DescaratesConfig, ConfigManager};
use crate::errors::AgentResult;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};
use tracing::{debug, info, warn};

/// Configuration change event
#[derive(Debug, Clone)]
pub struct ConfigChangeEvent {
    /// Path to the changed config file
    pub path: PathBuf,
    /// Previous configuration
    pub old_config: DescaratesConfig,
    /// New configuration
    pub new_config: DescaratesConfig,
    /// Timestamp of the change
    pub timestamp: SystemTime,
}

/// Configuration change listener
pub trait ConfigChangeListener: Send + Sync {
    /// Called when configuration changes
    fn on_config_change(&self, event: ConfigChangeEvent);
}

/// Configuration file watcher
pub struct ConfigWatcher {
    config_path: PathBuf,
    last_modified: Arc<Mutex<SystemTime>>,
    check_interval: Duration,
    enabled: Arc<Mutex<bool>>,
}

impl ConfigWatcher {
    /// Create a new configuration watcher
    pub fn new(config_path: PathBuf) -> Self {
        let last_modified = match std::fs::metadata(&config_path) {
            Ok(metadata) => metadata.modified().unwrap_or_else(|_| SystemTime::now()),
            Err(_) => SystemTime::now(),
        };

        Self {
            config_path,
            last_modified: Arc::new(Mutex::new(last_modified)),
            check_interval: Duration::from_secs(5),
            enabled: Arc::new(Mutex::new(true)),
        }
    }

    /// Set the check interval for file changes
    pub fn set_check_interval(&mut self, interval: Duration) {
        self.check_interval = interval;
    }

    /// Check if configuration file has changed
    pub fn check_for_changes(&self) -> AgentResult<Option<SystemTime>> {
        if !self.is_enabled() {
            return Ok(None);
        }

        let current_modified = match std::fs::metadata(&self.config_path) {
            Ok(metadata) => metadata.modified().ok(),
            Err(e) => {
                warn!("Failed to check config file metadata: {}", e);
                return Ok(None);
            }
        };

        if let Some(current_time) = current_modified {
            let mut last = self.last_modified.lock().unwrap();
            if current_time > *last {
                debug!("Configuration file has been modified");
                *last = current_time;
                return Ok(Some(current_time));
            }
        }

        Ok(None)
    }

    /// Load configuration if it has changed
    pub fn load_if_changed(&self) -> AgentResult<Option<DescaratesConfig>> {
        if self.check_for_changes()?.is_some() {
            debug!("Reloading configuration from: {:?}", self.config_path);
            let manager = ConfigManager::load(Some(&self.config_path))?;
            manager.validate()?;
            info!("Configuration reloaded successfully");
            Ok(Some(manager.config().clone()))
        } else {
            Ok(None)
        }
    }

    /// Enable/disable the watcher
    pub fn set_enabled(&self, enabled: bool) {
        *self.enabled.lock().unwrap() = enabled;
        if enabled {
            debug!("Configuration watcher enabled");
        } else {
            debug!("Configuration watcher disabled");
        }
    }

    /// Check if the watcher is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock().unwrap()
    }

    /// Get the config file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }

    /// Get the check interval
    pub fn check_interval(&self) -> Duration {
        self.check_interval
    }
}

/// Hot-reload handler for managing configuration reloads
pub struct HotReloadManager {
    watcher: Arc<ConfigWatcher>,
    listeners: Arc<Mutex<Vec<Box<dyn ConfigChangeListener>>>>,
}

impl HotReloadManager {
    /// Create a new hot-reload manager
    pub fn new(config_path: PathBuf) -> Self {
        Self {
            watcher: Arc::new(ConfigWatcher::new(config_path)),
            listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Register a change listener
    pub fn on_change(&self, listener: Box<dyn ConfigChangeListener>) {
        let mut listeners = self.listeners.lock().unwrap();
        listeners.push(listener);
        debug!("Registered config change listener");
    }

    /// Check for changes and notify listeners if needed
    pub fn check_and_reload(&self, current_config: DescaratesConfig) -> AgentResult<Option<DescaratesConfig>> {
        if let Some(new_config) = self.watcher.load_if_changed()? {
            // Create change event
            let event = ConfigChangeEvent {
                path: self.watcher.config_path().to_path_buf(),
                old_config: current_config.clone(),
                new_config: new_config.clone(),
                timestamp: SystemTime::now(),
            };

            // Notify all listeners
            let listeners = self.listeners.lock().unwrap();
            for listener in listeners.iter() {
                listener.on_config_change(event.clone());
            }

            info!("Configuration reloaded and listeners notified");
            Ok(Some(new_config))
        } else {
            Ok(None)
        }
    }

    /// Enable/disable hot-reloading
    pub fn set_enabled(&self, enabled: bool) {
        self.watcher.set_enabled(enabled);
    }

    /// Get the watcher
    pub fn watcher(&self) -> &ConfigWatcher {
        &self.watcher
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_config_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        // Create empty config file
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(b"[providers]\nprimary = \"anthropic\"").unwrap();

        let watcher = ConfigWatcher::new(config_path);
        assert!(watcher.is_enabled());
    }

    #[test]
    fn test_watcher_enable_disable() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(b"[providers]\nprimary = \"anthropic\"").unwrap();

        let watcher = ConfigWatcher::new(config_path);

        watcher.set_enabled(false);
        assert!(!watcher.is_enabled());

        watcher.set_enabled(true);
        assert!(watcher.is_enabled());
    }

    #[test]
    fn test_hot_reload_manager_creation() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");

        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(b"[providers]\nprimary = \"anthropic\"").unwrap();

        let manager = HotReloadManager::new(config_path);
        assert!(manager.watcher().is_enabled());
    }
}
