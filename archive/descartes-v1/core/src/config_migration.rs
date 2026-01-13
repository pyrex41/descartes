/// Configuration migration system for version upgrades
/// Handles migrations between different config versions to ensure backwards compatibility
use crate::config::DescaratesConfig;
use crate::errors::{AgentError, AgentResult};
use serde_json::{json, Value};
use tracing::{debug, info, warn};

/// Configuration migration handler
pub struct ConfigMigration;

impl ConfigMigration {
    /// Migrate configuration from older versions to current version
    pub fn migrate(
        mut config: DescaratesConfig,
        from_version: &str,
        to_version: &str,
    ) -> AgentResult<DescaratesConfig> {
        info!(
            "Migrating configuration from {} to {}",
            from_version, to_version
        );

        let from_major = parse_version(from_version)?;
        let to_major = parse_version(to_version)?;

        // No migration needed if versions match
        if from_major == to_major {
            debug!("Configuration version matches, no migration needed");
            return Ok(config);
        }

        // Execute migrations in sequence
        for version in from_major..to_major {
            config = Self::migrate_to_version(config, version)?;
        }

        config.version = to_version.to_string();
        info!("Configuration migration complete");
        Ok(config)
    }

    /// Migrate to a specific version
    fn migrate_to_version(
        mut config: DescaratesConfig,
        version: u32,
    ) -> AgentResult<DescaratesConfig> {
        match version {
            1 => {
                debug!("Applying migrations for v1.0.0");
                // Add any v1.0.0 specific migrations here
                Ok(config)
            }
            2 => {
                debug!("Applying migrations for v2.0.0");
                Self::migrate_v1_to_v2(&mut config)?;
                Ok(config)
            }
            3 => {
                debug!("Applying migrations for v3.0.0");
                Self::migrate_v2_to_v3(&mut config)?;
                Ok(config)
            }
            _ => {
                warn!("Unknown migration target version: {}", version);
                Ok(config)
            }
        }
    }

    /// Migration from v1 to v2: Add new security settings
    fn migrate_v1_to_v2(config: &mut DescaratesConfig) -> AgentResult<()> {
        info!("Migrating from v1.x to v2.x: Adding enhanced security settings");
        // Example: Add new encryption field if it doesn't exist
        if config.security.encryption_key.is_none() {
            debug!("Initializing encryption key from environment or defaults");
        }
        Ok(())
    }

    /// Migration from v2 to v3: Update storage configuration
    fn migrate_v2_to_v3(config: &mut DescaratesConfig) -> AgentResult<()> {
        info!("Migrating from v2.x to v3.x: Updating storage configuration");
        // Example: Update storage paths or database settings
        if config.storage.base_path.is_empty() {
            warn!("Storage base path is empty, using default");
            if let Ok(home) = std::env::var("HOME") {
                config.storage.base_path = format!("{}/.descartes", home);
            }
        }
        Ok(())
    }

    /// Create a migration from JSON (for config files)
    pub fn migrate_from_json(mut json: Value, from_version: &str) -> AgentResult<Value> {
        info!("Migrating JSON configuration from version {}", from_version);

        let from_major = parse_version(from_version)?;

        // Ensure version field exists
        if json.get("version").is_none() {
            json["version"] = Value::String(from_version.to_string());
        }

        // Apply JSON-level migrations
        for version in from_major..2 {
            match version {
                0 => {
                    debug!("Applying JSON migration for v0.x to v1.x");
                    Self::migrate_json_v0_to_v1(&mut json)?;
                }
                1 => {
                    debug!("Applying JSON migration for v1.x to v2.x");
                    Self::migrate_json_v1_to_v2(&mut json)?;
                }
                _ => {}
            }
        }

        Ok(json)
    }

    fn migrate_json_v0_to_v1(json: &mut Value) -> AgentResult<()> {
        // Example: Rename old field names
        if let Some(old_provider) = json.get("old_provider_field").cloned() {
            json["providers"]["primary"] = old_provider;
            json.as_object_mut().unwrap().remove("old_provider_field");
        }
        Ok(())
    }

    fn migrate_json_v1_to_v2(json: &mut Value) -> AgentResult<()> {
        // Example: Add new required fields with defaults
        if json.get("security").is_none() {
            json["security"] = json!({
                "enable_encryption": true,
                "encryption_algorithm": "aes-256-gcm"
            });
        }
        Ok(())
    }
}

/// Parse semantic version into major version number
fn parse_version(version: &str) -> AgentResult<u32> {
    version
        .split('.')
        .next()
        .and_then(|major| major.parse::<u32>().ok())
        .ok_or_else(|| AgentError::ExecutionError(format!("Invalid version format: {}", version)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_version() {
        assert_eq!(parse_version("1.0.0").unwrap(), 1);
        assert_eq!(parse_version("2.5.3").unwrap(), 2);
        assert_eq!(parse_version("10.0.0").unwrap(), 10);
    }

    #[test]
    fn test_parse_version_invalid() {
        assert!(parse_version("invalid").is_err());
        assert!(parse_version("").is_err());
    }

    #[test]
    fn test_no_migration_needed() {
        let config = DescaratesConfig::default();
        let result = ConfigMigration::migrate(config, "1.0.0", "1.0.0");
        assert!(result.is_ok());
    }
}
