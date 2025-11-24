# Configuration Migration Guide

Guide for maintaining backward compatibility and migrating between configuration versions.

## Overview

The Descartes configuration system is versioned to support breaking changes while maintaining backward compatibility through automatic migrations.

## Current Version: 1.0.0

The current schema includes:
- Provider configuration (Anthropic, OpenAI, Ollama, DeepSeek, Groq)
- Agent behavior settings
- Storage configuration (database, state, events, cache)
- Security and encryption
- Notifications and alerts
- Feature flags
- Logging and observability

## Migration Strategy

### Design Principles

1. **Always support previous versions** - Old configs should work with new code
2. **Clear migration path** - Explicit steps for major versions
3. **Automatic upgrades** - Minor/patch updates automatically merge configs
4. **Validation** - Validate configs after migration
5. **Audit trail** - Log all configuration changes

### Version Comparison

When a new version is released:
1. Load existing configuration
2. Compare version numbers
3. Run appropriate migration handlers
4. Validate new configuration
5. Create backup of old config
6. Save migrated configuration

## Example: Version 1.0.0 to 1.1.0 (Hypothetical)

### What Changed

New section added: `[providers.custom_deployment]`

### Migration Steps

1. **Parse old config (1.0.0)**
   ```toml
   version = "1.0.0"
   [providers]
   primary = "anthropic"
   ```

2. **Apply migration handler**
   ```rust
   fn migrate_1_0_to_1_1(config: &mut DescaratesConfig) {
       // Add new field with default value
       if !config.providers.custom.contains_key("default") {
           config.providers.custom.insert(
               "default".to_string(),
               CustomProviderConfig::default()
           );
       }
       config.version = "1.1.0".to_string();
   }
   ```

3. **Save migrated config**
   ```toml
   version = "1.1.0"
   [providers]
   primary = "anthropic"
   [providers.custom.default]
   endpoint = "..."
   ```

## Implementation

### Config Loader with Migration

```rust
pub struct ConfigManager {
    config: DescaratesConfig,
    config_path: PathBuf,
}

impl ConfigManager {
    pub fn load(config_path: Option<&Path>) -> AgentResult<Self> {
        // 1. Load TOML file
        let content = std::fs::read_to_string(&path)?;
        let mut config: DescaratesConfig = toml::from_str(&content)?;

        // 2. Check version
        let current_version = "1.0.0";
        if config.version != current_version {
            // 3. Run migrations
            Self::migrate(&mut config)?;
        }

        // 4. Validate
        validate_config(&config)?;

        Ok(ConfigManager { config, config_path: path })
    }

    fn migrate(config: &mut DescaratesConfig) -> AgentResult<()> {
        match config.version.as_str() {
            "0.9.0" => {
                Self::migrate_0_9_to_1_0(config)?;
                Self::migrate_1_0_to_1_1(config)?;
            }
            "1.0.0" => {
                Self::migrate_1_0_to_1_1(config)?;
            }
            v => {
                warn!("Unknown config version: {}, attempting compatibility mode", v);
            }
        }
        Ok(())
    }
}
```

### Migration Handlers

Each major version should have a migration function:

```rust
fn migrate_0_9_to_1_0(config: &mut DescaratesConfig) -> AgentResult<()> {
    // Breaking changes from 0.9 to 1.0
    debug!("Migrating configuration from 0.9.0 to 1.0.0");

    // Example: rename field
    // config.old_field -> config.new_field

    config.version = "1.0.0".to_string();
    info!("Migration 0.9.0 -> 1.0.0 completed");
    Ok(())
}

fn migrate_1_0_to_1_1(config: &mut DescaratesConfig) -> AgentResult<()> {
    // Non-breaking additions in 1.1
    debug!("Migrating configuration from 1.0.0 to 1.1.0");

    // Example: add new provider
    if !config.providers.custom.contains_key("new_provider") {
        config.providers.custom.insert(
            "new_provider".to_string(),
            CustomProviderConfig {
                endpoint: "https://...".to_string(),
                model: "default".to_string(),
                ..Default::default()
            }
        );
    }

    config.version = "1.1.0".to_string();
    info!("Migration 1.0.0 -> 1.1.0 completed");
    Ok(())
}
```

## Versioning Policy

### Major Version (X.0.0)
- Breaking changes
- New required fields (but with sensible defaults)
- Removed deprecated features
- Manual migration may be needed
- Clear migration guide provided

**Example:** Switch from OpenAI to Claude as default provider

### Minor Version (X.Y.0)
- New optional features
- New configuration sections
- Backward compatible
- Automatic migration with defaults

**Example:** Add new provider configuration

### Patch Version (X.Y.Z)
- Bug fixes
- Default value changes
- No schema changes
- No migration needed

**Example:** Change default timeout from 120s to 180s

## Testing Migrations

### Unit Tests

```rust
#[cfg(test)]
mod migration_tests {
    use super::*;

    #[test]
    fn test_migrate_0_9_to_1_0() {
        let old_config = r#"
            version = "0.9.0"
            [providers]
            primary = "anthropic"
        "#;

        let mut config: DescaratesConfig = toml::from_str(old_config).unwrap();
        ConfigManager::migrate(&mut config).unwrap();

        assert_eq!(config.version, "1.0.0");
    }

    #[test]
    fn test_migrate_preserves_data() {
        let config_str = r#"
            version = "1.0.0"
            [providers]
            primary = "openai"
            [providers.openai]
            enabled = true
        "#;

        let mut config: DescaratesConfig = toml::from_str(config_str).unwrap();
        ConfigManager::migrate(&mut config).unwrap();

        assert_eq!(config.providers.primary, "openai");
        assert!(config.providers.openai.enabled);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_load_legacy_config() {
    let temp_dir = tempfile::tempdir().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    // Write legacy config
    std::fs::write(&config_path, LEGACY_CONFIG_CONTENT).unwrap();

    // Load and migrate
    let manager = ConfigManager::load(Some(&config_path)).unwrap();

    // Verify migration
    assert_eq!(manager.config().version, "1.0.0");
    assert!(manager.validate().is_ok());
}
```

## Backup and Restore

### Automatic Backups

Before migration, create backup:

```rust
impl ConfigManager {
    fn create_backup(&self) -> AgentResult<PathBuf> {
        let backup_path = self.config_path.with_extension("toml.bak");
        std::fs::copy(&self.config_path, &backup_path)?;
        info!("Configuration backup created: {:?}", backup_path);
        Ok(backup_path)
    }
}
```

### Manual Restore

```bash
# Backup current config
cp ~/.descartes/config.toml ~/.descartes/config.toml.backup

# Restore from backup if needed
cp ~/.descartes/config.toml.backup ~/.descartes/config.toml
```

## Deprecation Policy

When features are deprecated:

1. **Mark as deprecated** in code with warnings
2. **Document deprecation** in CONFIGURATION.md
3. **Provide migration path** (old field → new field)
4. **Support for 2 major versions** before removal

### Example

```rust
/// DEPRECATED: Use `new_field` instead. Kept for compatibility with v1.x configs.
/// Will be removed in v3.0.0.
#[serde(default)]
pub old_field: Option<String>,
```

## Communication

### When releasing new version

1. **Update CONFIGURATION.md** with new options
2. **Create migration guide** if breaking changes
3. **Update example config** with new sections
4. **Document in CHANGELOG.md**

```markdown
## [1.1.0] - 2025-01-15

### Added
- New `providers.custom_deployment` section for advanced deployments
- Support for configuration versioning and auto-migration
- Backup of configurations before migration

### Changed
- Default temperature changed from 0.7 to 0.8

### Deprecated
- `providers.ollama.experimental_features` (use feature flags instead)

### Migration
- See CONFIG_MIGRATION.md for upgrade instructions
```

## Rollback Procedure

If a migrated config causes issues:

1. **Identify the problem** from logs
2. **Restore from backup**
   ```bash
   cp ~/.descartes/config.toml.bak ~/.descartes/config.toml
   ```
3. **Downgrade application** if needed
4. **Report issue** with:
   - Old version number
   - Migration error
   - Config excerpt (sanitized)

## Future Considerations

### Potential Breaking Changes for v2.0.0

- Consolidate provider configs (all use same schema)
- Rename sections (e.g., `agent` → `agents`)
- Change database default (sqlite → postgres)
- Remove deprecated fields
- Restructure notification channels

### Potential Minor Additions for v1.1.0+

- New cache types (memcached support)
- New notification channels (Discord, Teams)
- New logging targets (Datadog, CloudWatch)
- Rate limiting policies
- A/B testing configuration

## Best Practices

1. **Always backup before updating**
   ```bash
   cp ~/.descartes/config.toml ~/.descartes/config.toml.$(date +%Y%m%d)
   ```

2. **Test migrations in staging first**
   ```bash
   export DESCARTES_CONFIG_PATH=/tmp/staging.toml
   descartes --dry-run
   ```

3. **Review changes after migration**
   ```bash
   diff -u ~/.descartes/config.toml.bak ~/.descartes/config.toml
   ```

4. **Monitor logs after updates**
   ```bash
   tail -f ~/.descartes/data/descartes.log
   ```

5. **Keep documentation up to date**
   - Update migration guides
   - Document API changes
   - Provide examples

## Support

For migration issues:
1. Check this guide (CONFIG_MIGRATION.md)
2. Review backup: `~/.descartes/config.toml.bak`
3. Check logs: `~/.descartes/data/descartes.log`
4. Manual migration: restore and edit config

## See Also

- [CONFIGURATION.md](CONFIGURATION.md) - Complete configuration reference
- [CHANGELOG.md](CHANGELOG.md) - Version release notes
- [README.md](README.md) - General project documentation
