/// Daemon configuration
use crate::errors::{DaemonError, DaemonResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Daemon configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub server: ServerConfig,
    pub auth: AuthConfig,
    pub pool: PoolConfig,
    pub logging: LoggingConfig,
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// HTTP server bind address
    pub http_addr: String,
    /// HTTP server port
    pub http_port: u16,
    /// WebSocket server bind address
    pub ws_addr: String,
    /// WebSocket server port
    pub ws_port: u16,
    /// Request timeout in seconds
    pub request_timeout_secs: u64,
    /// Max connections
    pub max_connections: usize,
    /// Enable metrics endpoint
    pub enable_metrics: bool,
    /// Metrics port
    pub metrics_port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            http_addr: "127.0.0.1".to_string(),
            http_port: 8080,
            ws_addr: "127.0.0.1".to_string(),
            ws_port: 8081,
            request_timeout_secs: 30,
            max_connections: 1000,
            enable_metrics: true,
            metrics_port: 9090,
        }
    }
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication
    pub enabled: bool,
    /// JWT secret key
    pub jwt_secret: String,
    /// Token expiry in seconds
    pub token_expiry_secs: u64,
    /// API key for basic auth
    pub api_key: Option<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        AuthConfig {
            enabled: false,
            jwt_secret: "default-secret-change-in-production".to_string(),
            token_expiry_secs: 3600,
            api_key: None,
        }
    }
}

/// Connection pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolConfig {
    /// Minimum pool size
    pub min_size: u32,
    /// Maximum pool size
    pub max_size: u32,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Idle timeout in seconds
    pub idle_timeout_secs: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        PoolConfig {
            min_size: 10,
            max_size: 100,
            connection_timeout_secs: 30,
            idle_timeout_secs: 300,
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log file path
    pub file: Option<PathBuf>,
    /// Log to stdout
    pub stdout: bool,
    /// Log format
    pub format: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            level: "info".to_string(),
            file: None,
            stdout: true,
            format: "json".to_string(),
        }
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        DaemonConfig {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            pool: PoolConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

impl DaemonConfig {
    /// Load configuration from file
    pub fn load(path: &str) -> DaemonResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| DaemonError::ConfigError(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| DaemonError::ConfigError(format!("Failed to parse config: {}", e)))
    }

    /// Load from TOML file or use defaults
    pub fn load_or_default(path: Option<&str>) -> DaemonResult<Self> {
        match path {
            Some(p) => Self::load(p),
            None => Ok(Self::default()),
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> DaemonResult<()> {
        if self.server.http_port == 0 && self.server.ws_port == 0 {
            return Err(DaemonError::ConfigError(
                "At least one of http_port or ws_port must be non-zero".to_string(),
            ));
        }

        if self.server.max_connections == 0 {
            return Err(DaemonError::ConfigError(
                "max_connections must be greater than 0".to_string(),
            ));
        }

        if self.pool.min_size > self.pool.max_size {
            return Err(DaemonError::ConfigError(
                "pool.min_size must be <= pool.max_size".to_string(),
            ));
        }

        if self.auth.enabled && self.auth.jwt_secret == "default-secret-change-in-production" {
            return Err(DaemonError::ConfigError(
                "JWT secret must be changed from default when auth is enabled".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_is_valid() {
        let config = DaemonConfig::default();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validation_port_check() {
        let mut config = DaemonConfig::default();
        config.server.http_port = 0;
        config.server.ws_port = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_pool_validation() {
        let mut config = DaemonConfig::default();
        config.pool.min_size = 100;
        config.pool.max_size = 50;
        assert!(config.validate().is_err());
    }
}
