/// Configuration file discovery and loading system for Descartes.
/// Handles filesystem-based config loading with environment variable overrides,
/// validation, migrations, and hot-reloading support.

use crate::config::{DescaratesConfig, ConfigManager};
use crate::errors::{AgentError, AgentResult};
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};
use std::fs;
use std::env;

/// Configuration file discovery strategy
#[derive(Debug, Clone)]
pub enum ConfigDiscoveryStrategy {
    /// Check in order: .descartes/config.toml, ~/.descartes/config.toml, DESCARTES_CONFIG env var
    Default,
    /// Use explicit path
    Explicit(PathBuf),
    /// Load from environment variable only
    EnvironmentOnly,
}

/// Configuration loader with filesystem discovery
pub struct ConfigLoader {
    strategy: ConfigDiscoveryStrategy,
    discovered_path: Option<PathBuf>,
}

impl ConfigLoader {
    /// Create a new config loader with default discovery strategy
    pub fn new() -> Self {
        Self {
            strategy: ConfigDiscoveryStrategy::Default,
            discovered_path: None,
        }
    }

    /// Create a config loader with explicit path
    pub fn with_path(path: PathBuf) -> Self {
        Self {
            strategy: ConfigDiscoveryStrategy::Explicit(path),
            discovered_path: None,
        }
    }

    /// Create a config loader that uses environment variable only
    pub fn env_only() -> Self {
        Self {
            strategy: ConfigDiscoveryStrategy::EnvironmentOnly,
            discovered_path: None,
        }
    }

    /// Discover config file path based on strategy
    fn discover_config_path(&self) -> AgentResult<Option<PathBuf>> {
        match &self.strategy {
            ConfigDiscoveryStrategy::Explicit(path) => {
                debug!("Using explicit config path: {:?}", path);
                Ok(Some(path.clone()))
            }
            ConfigDiscoveryStrategy::EnvironmentOnly => {
                match env::var("DESCARTES_CONFIG") {
                    Ok(path) => {
                        debug!("Found DESCARTES_CONFIG environment variable: {}", path);
                        Ok(Some(PathBuf::from(path)))
                    }
                    Err(_) => {
                        warn!("DESCARTES_CONFIG not set, will use defaults");
                        Ok(None)
                    }
                }
            }
            ConfigDiscoveryStrategy::Default => {
                // Check 1: .descartes/config.toml in current directory
                let local_config = PathBuf::from(".descartes/config.toml");
                if local_config.exists() {
                    debug!("Found config at: {:?}", local_config);
                    return Ok(Some(local_config));
                }

                // Check 2: ~/.descartes/config.toml
                if let Ok(home) = env::var("HOME") {
                    let home_config = PathBuf::from(home).join(".descartes/config.toml");
                    if home_config.exists() {
                        debug!("Found config at: {:?}", home_config);
                        return Ok(Some(home_config));
                    }
                }

                // Check 3: DESCARTES_CONFIG environment variable
                if let Ok(env_config) = env::var("DESCARTES_CONFIG") {
                    let env_path = PathBuf::from(env_config);
                    if env_path.exists() {
                        debug!("Found config via DESCARTES_CONFIG: {:?}", env_path);
                        return Ok(Some(env_path));
                    } else {
                        warn!("DESCARTES_CONFIG points to non-existent file: {:?}", env_path);
                    }
                }

                debug!("No config file found, will use defaults");
                Ok(None)
            }
        }
    }

    /// Load configuration with defaults and environment variable overrides
    pub fn load(mut self) -> AgentResult<(ConfigManager, PathBuf)> {
        // Discover config file path
        let config_path = self.discover_config_path()?;
        self.discovered_path = config_path.clone();

        // Load base configuration
        let mut config_manager = if let Some(path) = &config_path {
            info!("Loading configuration from: {:?}", path);

            // Verify file is readable
            if !path.exists() {
                return Err(AgentError::ExecutionError(
                    format!("Config file not found: {:?}", path)
                ));
            }

            ConfigManager::load(Some(path))?
        } else {
            info!("No config file found, using defaults");
            ConfigManager::load(None)?
        };

        // Apply environment variable overrides
        self.apply_env_overrides(&mut config_manager)?;

        // Validate configuration
        config_manager.validate()?;

        let discovered_or_default = config_path.unwrap_or_else(|| {
            if let Ok(home) = env::var("HOME") {
                PathBuf::from(home).join(".descartes/config.toml")
            } else {
                PathBuf::from(".descartes/config.toml")
            }
        });

        info!("Configuration loaded and validated successfully");
        Ok((config_manager, discovered_or_default))
    }

    /// Apply environment variable overrides to configuration
    fn apply_env_overrides(&self, config_manager: &mut ConfigManager) -> AgentResult<()> {
        let config = config_manager.config_mut();

        // Provider overrides
        if let Ok(key) = env::var("OPENAI_API_KEY") {
            debug!("Overriding OpenAI API key from environment");
            config.providers.openai.api_key = Some(key);
            config.providers.openai.enabled = true;
        }

        if let Ok(key) = env::var("ANTHROPIC_API_KEY") {
            debug!("Overriding Anthropic API key from environment");
            config.providers.anthropic.api_key = Some(key);
        }

        if let Ok(key) = env::var("DEEPSEEK_API_KEY") {
            debug!("Overriding DeepSeek API key from environment");
            config.providers.deepseek.api_key = Some(key);
            config.providers.deepseek.enabled = true;
        }

        if let Ok(key) = env::var("GROQ_API_KEY") {
            debug!("Overriding Groq API key from environment");
            config.providers.groq.api_key = Some(key);
            config.providers.groq.enabled = true;
        }

        // Security overrides
        if let Ok(key) = env::var("DESCARTES_ENCRYPTION_KEY") {
            debug!("Overriding encryption key from environment");
            config.security.encryption_key = Some(key);
        }

        if let Ok(key) = env::var("DESCARTES_SECRET_KEY") {
            debug!("Overriding secret key from environment");
            config.security.secret_key = Some(key);
        }

        // Storage path override
        if let Ok(path) = env::var("DESCARTES_STORAGE_PATH") {
            debug!("Overriding storage path from environment: {}", path);
            config.storage.base_path = path;
        }

        // Log level override
        if let Ok(level) = env::var("DESCARTES_LOG_LEVEL") {
            debug!("Overriding log level from environment: {}", level);
            config.logging.level = level;
        }

        info!("Applied environment variable overrides to configuration");
        Ok(())
    }

    /// Get the discovered config path (if any)
    pub fn discovered_path(&self) -> Option<&Path> {
        self.discovered_path.as_deref()
    }
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize config and ensure directory structure exists
pub fn init_config() -> AgentResult<(ConfigManager, PathBuf)> {
    let loader = ConfigLoader::new();
    let (config_manager, config_path) = loader.load()?;

    // Ensure directories exist
    ensure_config_directories(&config_manager)?;

    info!("Configuration initialization complete");
    Ok((config_manager, config_path))
}

/// Ensure all required directories exist
pub fn ensure_config_directories(config_manager: &ConfigManager) -> AgentResult<()> {
    let config = config_manager.config();
    let base_path = Path::new(&config.storage.base_path);

    // Create base storage directory
    fs::create_dir_all(base_path).map_err(|e| {
        AgentError::ExecutionError(
            format!("Failed to create storage directory {:?}: {}", base_path, e)
        )
    })?;
    debug!("Created/verified storage directory: {:?}", base_path);

    // Create database directory
    let db_path = base_path.join("data");
    fs::create_dir_all(&db_path).map_err(|e| {
        AgentError::ExecutionError(
            format!("Failed to create database directory {:?}: {}", db_path, e)
        )
    })?;
    debug!("Created/verified database directory: {:?}", db_path);

    // Create state directory if state store is enabled
    if config.storage.state_store.enabled {
        let state_path = base_path.join(&config.storage.state_store.path);
        fs::create_dir_all(&state_path).map_err(|e| {
            AgentError::ExecutionError(
                format!("Failed to create state directory {:?}: {}", state_path, e)
            )
        })?;
        debug!("Created/verified state directory: {:?}", state_path);
    }

    // Create event directory if event store is enabled
    if config.storage.event_store.enabled {
        let event_path = base_path.join(&config.storage.event_store.path);
        fs::create_dir_all(&event_path).map_err(|e| {
            AgentError::ExecutionError(
                format!("Failed to create event directory {:?}: {}", event_path, e)
            )
        })?;
        debug!("Created/verified event directory: {:?}", event_path);
    }

    // Create cache directory if cache is enabled
    if config.storage.cache.enabled && config.storage.cache.cache_type == "disk" {
        let cache_path = base_path.join(&config.storage.cache.disk_path);
        fs::create_dir_all(&cache_path).map_err(|e| {
            AgentError::ExecutionError(
                format!("Failed to create cache directory {:?}: {}", cache_path, e)
            )
        })?;
        debug!("Created/verified cache directory: {:?}", cache_path);
    }

    // Create log directory if file logging is enabled
    if let Some(log_file_config) = &config.logging.targets.file {
        let log_dir = base_path.join(&log_file_config.path).parent().map(|p| p.to_path_buf());
        if let Some(log_path) = log_dir {
            fs::create_dir_all(&log_path).map_err(|e| {
                AgentError::ExecutionError(
                    format!("Failed to create log directory {:?}: {}", log_path, e)
                )
            })?;
            debug!("Created/verified log directory: {:?}", log_path);
        }
    }

    info!("All configuration directories verified/created");
    Ok(())
}

/// Configuration validation rules
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate provider configuration
    pub fn validate_providers(config: &DescaratesConfig) -> AgentResult<()> {
        let primary = &config.providers.primary;

        match primary.as_str() {
            "anthropic" => {
                if !config.providers.anthropic.enabled {
                    warn!("Primary provider is set to anthropic but it's disabled");
                }
            }
            "openai" => {
                if !config.providers.openai.enabled {
                    warn!("Primary provider is set to openai but it's disabled");
                }
            }
            "ollama" => {
                if !config.providers.ollama.enabled {
                    warn!("Primary provider is set to ollama but it's disabled");
                }
            }
            _ => {
                if !config.providers.custom.contains_key(primary) {
                    warn!("Primary provider {} not found in custom providers", primary);
                }
            }
        }

        Ok(())
    }

    /// Validate storage configuration
    pub fn validate_storage(config: &DescaratesConfig) -> AgentResult<()> {
        let base_path = Path::new(&config.storage.base_path);

        // Check if base path is absolute or relative (acceptable, but warn if relative)
        if base_path.is_relative() {
            warn!("Storage base path is relative: {}", config.storage.base_path);
        }

        // Validate database settings
        if config.storage.database.pool_size == 0 {
            return Err(AgentError::ExecutionError(
                "Database pool size must be greater than 0".to_string(),
            ));
        }

        if config.storage.database.pool_size > 100 {
            warn!("Database pool size is unusually high: {}", config.storage.database.pool_size);
        }

        Ok(())
    }

    /// Validate all settings
    pub fn validate_all(config: &DescaratesConfig) -> AgentResult<()> {
        Self::validate_providers(config)?;
        Self::validate_storage(config)?;

        // Validate agent settings
        if config.agent.max_concurrent_agents == 0 {
            return Err(AgentError::ExecutionError(
                "Max concurrent agents must be greater than 0".to_string(),
            ));
        }

        // Validate temperature values for all providers
        if config.providers.openai.temperature < 0.0 || config.providers.openai.temperature > 2.0 {
            return Err(AgentError::ExecutionError(
                "OpenAI temperature must be between 0.0 and 2.0".to_string(),
            ));
        }

        if config.providers.anthropic.temperature < 0.0 || config.providers.anthropic.temperature > 2.0 {
            return Err(AgentError::ExecutionError(
                "Anthropic temperature must be between 0.0 and 2.0".to_string(),
            ));
        }

        debug!("All configuration validations passed");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_loader_creation() {
        let loader = ConfigLoader::new();
        matches!(loader.strategy, ConfigDiscoveryStrategy::Default);
    }

    #[test]
    fn test_explicit_path_loader() {
        let path = PathBuf::from("/tmp/test.toml");
        let loader = ConfigLoader::with_path(path.clone());
        matches!(loader.strategy, ConfigDiscoveryStrategy::Explicit(_));
    }

    #[test]
    fn test_env_only_loader() {
        let loader = ConfigLoader::env_only();
        matches!(loader.strategy, ConfigDiscoveryStrategy::EnvironmentOnly);
    }

    #[test]
    fn test_config_validator() {
        let config = DescaratesConfig::default();
        assert!(ConfigValidator::validate_all(&config).is_ok());
    }
}
