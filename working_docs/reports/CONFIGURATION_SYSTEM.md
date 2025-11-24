# Descartes Configuration System Implementation

## Overview

This implementation provides a complete, production-grade configuration loading system for the Descartes orchestration platform. It includes filesystem discovery, environment variable overrides, validation, migrations, and hot-reloading capabilities.

## Components

### 1. ConfigLoader (`config_loader.rs`)

**Purpose**: Discovers and loads configuration files with environment variable overrides

**Features**:
- **Configuration Discovery Strategy**:
  - Default: Checks `.descartes/config.toml` → `~/.descartes/config.toml` → `DESCARTES_CONFIG` env var
  - Explicit: Uses provided file path
  - Environment-only: Uses only `DESCARTES_CONFIG` env var

- **Environment Variable Overrides**:
  - `OPENAI_API_KEY` - OpenAI API key
  - `ANTHROPIC_API_KEY` - Anthropic API key
  - `DEEPSEEK_API_KEY` - DeepSeek API key
  - `GROQ_API_KEY` - Groq API key
  - `DESCARTES_ENCRYPTION_KEY` - Master encryption key
  - `DESCARTES_SECRET_KEY` - Session secret key
  - `DESCARTES_STORAGE_PATH` - Base storage directory
  - `DESCARTES_LOG_LEVEL` - Log level (trace/debug/info/warn/error)

- **Directory Initialization**:
  - Creates storage base path
  - Creates database directory
  - Creates state store directory (if enabled)
  - Creates event store directory (if enabled)
  - Creates cache directory (if enabled)
  - Creates log directory (if enabled)

- **Configuration Validation**:
  - Validates provider settings
  - Validates storage paths
  - Validates agent settings
  - Validates temperature ranges
  - Warns on unusual values

**Usage**:
```rust
use descartes_core::ConfigLoader;

// Default discovery
let loader = ConfigLoader::new();
let (config_manager, config_path) = loader.load()?;

// Explicit path
let loader = ConfigLoader::with_path(PathBuf::from("/etc/descartes/config.toml"));
let (config_manager, config_path) = loader.load()?;

// Initialize config and directories
let (config_manager, config_path) = init_config()?;
```

### 2. ConfigMigration (`config_migration.rs`)

**Purpose**: Handles configuration version migrations for backwards compatibility

**Features**:
- **Version Parsing**: Parses semantic versions (e.g., "1.0.0" → major version 1)
- **Migration Pipeline**: Executes migrations in sequence from old to new version
- **JSON Migrations**: Supports JSON-level migrations for config file format changes
- **Extensible Architecture**: Easy to add new migrations for future versions

**Migrations Supported**:
- v1.0.0: Base configuration
- v2.0.0: Enhanced security settings
- v3.0.0: Updated storage configuration

**Usage**:
```rust
use descartes_core::ConfigMigration;

// Migrate configuration
let migrated_config = ConfigMigration::migrate(
    config,
    "1.0.0",
    "2.0.0"
)?;

// Migrate JSON
let migrated_json = ConfigMigration::migrate_from_json(json, "1.0.0")?;
```

### 3. ConfigWatcher (`config_watcher.rs`)

**Purpose**: Monitors configuration file changes and supports hot-reloading

**Features**:
- **File Change Detection**: Polls config file modification time
- **Configurable Check Interval**: Default 5 seconds, customizable
- **Change Notifications**: Notifies listeners when config changes
- **Disabled/Enabled Control**: Can be toggled on/off
- **Change Events**: Includes old/new config and timestamp

**Types**:
- `ConfigChangeEvent`: Contains path, old_config, new_config, timestamp
- `ConfigChangeListener`: Trait for custom change handlers
- `ConfigWatcher`: Core watcher implementation
- `HotReloadManager`: Manages multiple listeners

**Usage**:
```rust
use descartes_core::{ConfigWatcher, HotReloadManager, ConfigChangeListener, ConfigChangeEvent};

let mut watcher = ConfigWatcher::new(PathBuf::from("/etc/descartes/config.toml"));
watcher.set_check_interval(Duration::from_secs(10));

// Check if changed and reload
if let Some(new_config) = watcher.load_if_changed()? {
    // Use new_config
}

// Or use hot-reload manager
let manager = HotReloadManager::new(config_path.clone());
manager.on_change(Box::new(MyListener));

// Check and notify listeners
if let Some(new_config) = manager.check_and_reload(current_config)? {
    // Config was reloaded and listeners notified
}
```

### 4. Integration with CLI (`cli/src/main.rs`)

**Changes Made**:
- Added `--config` global flag to specify config path
- Added `--log-level` global flag to override log level
- Configuration is loaded before any command execution
- All subsystems receive the loaded config
- Directory initialization happens automatically

**Flow**:
1. Parse CLI arguments
2. Initialize tracing with custom log level
3. Load configuration using ConfigLoader
4. Ensure directories exist
5. Pass config to command handlers

**Example**:
```bash
# Use default discovery
descartes spawn --task "my task"

# Use explicit config
descartes --config /etc/descartes/config.toml spawn --task "my task"

# Override log level
descartes --log-level debug spawn --task "my task"

# Override provider
ANTHROPIC_API_KEY=sk-... descartes spawn --task "my task"
```

### 5. Configuration Files

The system looks for config files in this order:
1. `.descartes/config.toml` (current directory)
2. `~/.descartes/config.toml` (user home)
3. `$DESCARTES_CONFIG` (environment variable)

**Default Config Structure**:
```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
api_key = "from-env"
endpoint = "https://api.anthropic.com"
model = "claude-3-5-sonnet-20241022"
temperature = 0.7
max_tokens = 4096

[storage]
base_path = "~/.descartes"

[storage.database]
database_type = "sqlite"
sqlite_path = "data/descartes.db"
pool_size = 10

[agent]
default_timeout_secs = 300
max_concurrent_agents = 10
enable_streaming = true
enable_tools = true

[logging]
level = "info"
format = "text"

[security]
enable_encryption = true
```

## Architecture

### Initialization Sequence

```
CLI Main
├─ Parse Arguments
├─ Initialize Tracing
├─ ConfigLoader::new() / with_path()
├─ loader.load()
│  ├─ Discover config path
│  ├─ ConfigManager::load(path)
│  │  ├─ Read TOML file
│  │  └─ Parse to DescaratesConfig
│  ├─ Apply environment overrides
│  ├─ Run validation
│  └─ Return (ConfigManager, path)
├─ ensure_config_directories()
│  ├─ Create base_path
│  ├─ Create database directory
│  ├─ Create state/event/cache dirs
│  └─ Create log directory
└─ Execute Command with config
```

### Configuration Flow

```
Environment Variables (highest priority)
    ↓
Config File (.descartes/config.toml)
    ↓
Defaults (built-in)

Provider API Keys: ENV > Config > Defaults
Paths: ENV > Config > Defaults
Timeouts: Config > Defaults
Feature Flags: Config > Defaults
```

## Features

### 1. Filesystem Loading
- **Smart Discovery**: Checks multiple locations automatically
- **Error Handling**: Clear error messages if config invalid
- **Flexible Paths**: Supports absolute and relative paths

### 2. Environment Variable Overrides
- **Priority System**: Environment variables override config files
- **Provider Keys**: All API keys can be set via env vars
- **Storage Paths**: Paths can be overridden via env vars
- **Log Level**: Can be set via env var for debugging

### 3. Config Validation
- **Pre-flight Checks**: Validates before use
- **Range Validation**: Checks temperature, pool sizes, etc.
- **Warnings**: Alerts on unusual values (e.g., high pool size)
- **Requirement Checks**: Ensures at least one provider is available

### 4. Configuration Migration
- **Version Tracking**: Tracks config file versions
- **Auto-migration**: Can automatically upgrade config formats
- **Backwards Compatible**: Old configs work with new code
- **Extensible**: Easy to add new migrations

### 5. Hot-Reloading
- **Change Detection**: Monitors config file for changes
- **Configurable Polling**: Adjustable check interval
- **Listener Pattern**: Multiple handlers can be notified
- **Optional**: Can be disabled for static configs

## API Reference

### ConfigLoader
```rust
pub fn new() -> Self
pub fn with_path(path: PathBuf) -> Self
pub fn env_only() -> Self
pub fn load(self) -> AgentResult<(ConfigManager, PathBuf)>
pub fn discovered_path(&self) -> Option<&Path>
```

### ConfigMigration
```rust
pub fn migrate(config: DescaratesConfig, from: &str, to: &str) -> AgentResult<DescaratesConfig>
pub fn migrate_from_json(json: Value, from_version: &str) -> AgentResult<Value>
```

### ConfigWatcher
```rust
pub fn new(config_path: PathBuf) -> Self
pub fn set_check_interval(&mut self, interval: Duration)
pub fn check_for_changes(&self) -> AgentResult<Option<SystemTime>>
pub fn load_if_changed(&self) -> AgentResult<Option<DescaratesConfig>>
pub fn set_enabled(&self, enabled: bool)
pub fn is_enabled(&self) -> bool
```

### HotReloadManager
```rust
pub fn new(config_path: PathBuf) -> Self
pub fn on_change(&self, listener: Box<dyn ConfigChangeListener>)
pub fn check_and_reload(&self, current_config: DescaratesConfig) -> AgentResult<Option<DescaratesConfig>>
pub fn set_enabled(&self, enabled: bool)
pub fn watcher(&self) -> &ConfigWatcher
```

### Helper Functions
```rust
pub fn init_config() -> AgentResult<(ConfigManager, PathBuf)>
pub fn ensure_config_directories(config_manager: &ConfigManager) -> AgentResult<()>
```

## Error Handling

All configuration operations return `AgentResult<T>` which provides detailed error messages:

```rust
pub enum AgentError {
    ExecutionError(String),
    IoError(std::io::Error),
    // ... other variants
}
```

Common errors:
- "Config file not found at ..." - File path doesn't exist
- "Failed to read config file: ..." - Permission or IO error
- "Failed to parse config file: ..." - Invalid TOML syntax
- "Database pool size must be greater than 0" - Validation failure
- "No providers configured" - Warning about configuration

## Testing

The implementation includes tests for:
- Configuration discovery strategies
- Environment variable overrides
- Validation rules
- Migration paths
- File watcher functionality
- Hot-reload manager

**Run tests**:
```bash
cargo test --lib config_loader
cargo test --lib config_migration
cargo test --lib config_watcher
```

## Security Considerations

1. **API Keys**: Can be stored in environment variables instead of config files
2. **File Permissions**: Config files should be readable only by descartes user (0600)
3. **Encryption**: Sensitive fields can be encrypted using security.encryption_key
4. **Audit Logging**: Changes to configuration can be logged via audit_log_path

## Migration Guide

### From Old System
If the system previously loaded config manually:

**Before**:
```rust
let manager = ConfigManager::load(config_path)?;
```

**After**:
```rust
let loader = ConfigLoader::new();
let (manager, path) = loader.load()?;
ensure_config_directories(&manager)?;
```

### Environment Variables
To use environment variables for API keys:

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-..."
descartes spawn --task "my task"
```

No code changes needed - overrides are automatic.

## Future Enhancements

1. **Config File Watching**: Automatic reload when file changes
2. **Config Validation Schema**: JSON schema validation
3. **Config Generation**: Generate default config interactively
4. **Config Encryption**: Encrypt sensitive fields at rest
5. **Config Backup**: Automatic backups before changes
6. **Config Merge**: Merge multiple config files
7. **Profile Support**: Named configuration profiles (dev/prod/test)

## Files Modified/Created

### New Files
- `/Users/reuben/gauntlet/cap/descartes/core/src/config_loader.rs` - Configuration loading
- `/Users/reuben/gauntlet/cap/descartes/core/src/config_migration.rs` - Migration system
- `/Users/reuben/gauntlet/cap/descartes/core/src/config_watcher.rs` - Hot-reload system

### Modified Files
- `/Users/reuben/gauntlet/cap/descartes/core/src/lib.rs` - Added module exports
- `/Users/reuben/gauntlet/cap/descartes/cli/src/main.rs` - Integrated config loading

## Summary

This configuration system provides:

✅ **Smart Discovery**: Automatically finds config in expected locations
✅ **Environment Overrides**: All settings can be overridden via env vars
✅ **Validation**: Pre-flight checks before config is used
✅ **Migration System**: Version upgrades for config format changes
✅ **Hot-Reloading**: Monitor file changes and notify listeners
✅ **Error Messages**: Clear, actionable error messages
✅ **Directory Init**: Automatically creates all needed directories
✅ **Extensible**: Easy to add new features and migrations
✅ **Tested**: Includes unit tests for all components
✅ **Documented**: Comprehensive API documentation

The configuration is now fully functional and integrated with the CLI!
