# Descartes Configuration System

Complete guide to configuring the Descartes AI Agent Orchestration System.

## Overview

Descartes uses a comprehensive TOML configuration system stored at `~/.descartes/config.toml`. The configuration is organized into logical sections for easy management and maintenance.

## Quick Start

1. **Copy the example configuration:**
   ```bash
   mkdir -p ~/.descartes
   cp .descartes/config.toml.example ~/.descartes/config.toml
   ```

2. **Edit the configuration:**
   ```bash
   $EDITOR ~/.descartes/config.toml
   ```

3. **Set environment variables for API keys:**
   ```bash
   export ANTHROPIC_API_KEY="sk-ant-..."
   export OPENAI_API_KEY="sk-proj-..."
   ```

4. **Validate your configuration:**
   The system automatically validates on startup and logs any issues.

## Configuration Sections

### 1. Providers Configuration

Configure which LLM providers are available and how to connect to them.

#### Primary Provider
```toml
[providers]
primary = "anthropic"  # Default provider to use
```

#### Anthropic Claude
```toml
[providers.anthropic]
enabled = true
api_key = "sk-ant-..."  # Set via ANTHROPIC_API_KEY env var
endpoint = "https://api.anthropic.com"
model = "claude-3-5-sonnet-20241022"
models = [
    "claude-3-5-sonnet-20241022",
    "claude-3-opus-20240229",
    "claude-3-haiku-20240307"
]
timeout_secs = 120
max_retries = 3
retry_backoff_ms = 1000
rate_limit_rpm = 60
temperature = 0.7
max_tokens = 4096
```

**Supported Models:**
- `claude-3-5-sonnet-20241022` - Latest Sonnet (recommended for most tasks)
- `claude-3-opus-20240229` - Most capable, slower
- `claude-3-haiku-20240307` - Fastest, least capable

#### OpenAI
```toml
[providers.openai]
enabled = false
api_key = "sk-proj-..."  # Set via OPENAI_API_KEY env var
model = "gpt-4-turbo"
models = ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"]
```

#### Ollama (Local)
```toml
[providers.ollama]
enabled = false
endpoint = "http://localhost:11434"
model = "llama2"
```

#### DeepSeek
```toml
[providers.deepseek]
enabled = false
api_key = "sk-..."  # Set via DEEPSEEK_API_KEY env var
endpoint = "https://api.deepseek.com/v1"
model = "deepseek-chat"
```

#### Groq
```toml
[providers.groq]
enabled = false
api_key = "gsk_..."  # Set via GROQ_API_KEY env var
endpoint = "https://api.groq.com/openai/v1"
model = "mixtral-8x7b-32768"
```

#### Custom Providers
```toml
[providers.custom.my-provider]
endpoint = "https://custom.api.com/v1"
api_key = "custom-key"
model = "custom-model"
timeout_secs = 120
use_bearer_auth = true
```

### 2. Agent Behavior Configuration

Control how agents execute and behave.

```toml
[agent]
# Agent execution timeout (seconds)
default_timeout_secs = 300

# Maximum concurrent agents
max_concurrent_agents = 10

# Task queue buffer size
task_queue_size = 1000

# Enable streaming responses
enable_streaming = true

# Enable function/tool calling
enable_tools = true

# Tool execution settings
max_tool_retries = 2
tool_timeout_secs = 60

# Context caching
enable_memory = true
memory_ttl_secs = 3600
max_context_tokens = 32000

# Background task processing
enable_background_tasks = true
worker_threads = 4
```

**Key Settings:**

| Setting | Default | Description |
|---------|---------|-------------|
| `default_timeout_secs` | 300 | Maximum seconds to wait for agent completion |
| `max_concurrent_agents` | 10 | Limits resource usage and API quotas |
| `enable_streaming` | true | Enable Server-Sent Events for streaming |
| `enable_tools` | true | Allow agents to call external functions |
| `max_context_tokens` | 32000 | Maximum total tokens in context window |
| `worker_threads` | 4 | Threads for background task processing |

### 3. Storage Configuration

Configure where Descartes stores data, state, and events.

```toml
[storage]
base_path = "~/.descartes"
```

#### Database
```toml
[storage.database]
database_type = "sqlite"              # sqlite, postgres, mysql
sqlite_path = "data/descartes.db"
# postgres_url = "postgresql://..."  # If using PostgreSQL
pool_size = 10
auto_migrate = true
enable_backups = true
backup_interval_hours = 24
```

**Supported Databases:**
- **SQLite** (default) - Single file, no setup required
- **PostgreSQL** - For production use
- **MySQL** - Alternative relational database

#### State Store
```toml
[storage.state_store]
enabled = true
path = "data/state"
serialization_format = "json"        # json, msgpack, bincode
enable_compression = false
```

#### Event Store
```toml
[storage.event_store]
enabled = true
path = "data/events"
retention_days = 0                   # 0 = infinite retention
batch_size = 100
enable_indexing = true
```

#### Cache
```toml
[storage.cache]
enabled = true
cache_type = "in-memory"             # in-memory, redis, disk
disk_path = "data/cache"
# redis_url = "redis://localhost:6379"
ttl_secs = 3600
max_size_mb = 512
```

### 4. Security Configuration

Configure encryption, secrets management, and access control.

```toml
[security]
# Encryption settings
enable_encryption = true
encryption_algorithm = "aes-256-gcm"
# encryption_key = "..."             # Set via DESCARTES_ENCRYPTION_KEY
encrypt_api_keys = true

# File permissions
file_permissions = "0600"             # Octal notation

# Access control
enable_rbac = false
# secret_key = "..."                 # Set via DESCARTES_SECRET_KEY
session_timeout_secs = 3600

# Audit logging
enable_audit_logging = true
audit_log_path = "data/audit.log"
```

**Important Security Notes:**

1. **Encryption Keys** - Generate secure keys:
   ```bash
   openssl rand -base64 32
   ```

2. **Never commit secrets** - Always use environment variables:
   ```bash
   export DESCARTES_ENCRYPTION_KEY="$(openssl rand -base64 32)"
   export DESCARTES_SECRET_KEY="$(openssl rand -base64 32)"
   ```

3. **File Permissions** - Ensure config file has restricted permissions:
   ```bash
   chmod 0600 ~/.descartes/config.toml
   ```

4. **Audit Logging** - All security events are logged to enable compliance audits.

### 5. Notifications Configuration

Configure how Descartes notifies you about important events.

#### Enable Notifications
```toml
[notifications]
enabled = false
```

#### Alert Thresholds
```toml
[notifications.alerts]
high_token_usage_threshold = 80      # Alert at 80% token usage
alert_on_api_errors = true
alert_on_agent_failures = true
alert_on_timeouts = true
alert_on_rate_limit = true
```

#### Telegram
```toml
[notifications.channels.telegram]
enabled = true
bot_token = "YOUR_TELEGRAM_BOT_TOKEN"
chat_id = "YOUR_CHAT_ID"
```

#### Webhooks
```toml
[[notifications.channels.webhooks]]
name = "slack"
url = "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
method = "POST"
events = ["agent_failure", "high_token_usage"]
enable_retries = true
max_retries = 3

[notifications.channels.webhooks.headers]
X-Custom-Header = "value"
```

#### Email
```toml
[notifications.channels.email]
enabled = true
smtp_server = "smtp.gmail.com"
smtp_port = 587
smtp_user = "your-email@gmail.com"
smtp_password = "app-password"       # Not your Gmail password!
from_address = "your-email@gmail.com"
recipients = ["admin@example.com", "ops@example.com"]
use_tls = true
```

#### Slack
```toml
[notifications.channels.slack]
enabled = true
webhook_url = "https://hooks.slack.com/services/YOUR/WEBHOOK/URL"
channel = "#alerts"
username = "Descartes"
```

### 6. Feature Flags Configuration

Enable/disable experimental and beta features.

```toml
[features]
enable_experimental = false
enable_debug = false
enable_tracing = true
enable_profiling = false

[features.flags]
streaming_responses = true
persistent_context = true

beta_features = [
    "advanced_memory",
    "distributed_execution"
]
```

### 7. Logging Configuration

Configure logging output, level, and targets.

```toml
[logging]
level = "info"                        # trace, debug, info, warn, error
format = "text"                       # json, text, pretty
log_requests = true
log_responses = true
log_sensitive_data = false            # NEVER true in production
```

#### File Logging
```toml
[logging.targets.file]
path = "data/descartes.log"
max_size_mb = 100                     # For rotation
max_backups = 10
compress = false
```

**Log Levels:**

| Level | Description | Use Case |
|-------|-------------|----------|
| `trace` | Very detailed, every operation | Development debugging |
| `debug` | Detailed diagnostic info | Development, troubleshooting |
| `info` | General informational messages | Production (default) |
| `warn` | Warning messages for potential issues | All environments |
| `error` | Error conditions | All environments |

## Environment Variables

Descartes loads configuration from environment variables, which override file settings:

```bash
# API Keys
export OPENAI_API_KEY="sk-proj-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export DEEPSEEK_API_KEY="sk-..."
export GROQ_API_KEY="gsk_..."

# Security
export DESCARTES_ENCRYPTION_KEY="$(openssl rand -base64 32)"
export DESCARTES_SECRET_KEY="$(openssl rand -base64 32)"

# Optional
export DESCARTES_CONFIG_PATH="/path/to/config.toml"
export DESCARTES_LOG_LEVEL="debug"
```

## Configuration Loading Order

1. **Default values** - Built into the application
2. **config.toml file** - Loaded from `~/.descartes/config.toml` or path specified in code
3. **Environment variables** - Override all file settings

This allows flexibility: use config file for most settings, environment variables for secrets.

## Validation

The configuration is validated on startup. Common issues:

```
Error: Database pool size must be greater than 0
  ✓ Solution: Ensure pool_size > 0 in [storage.database]

Error: Temperature must be between 0.0 and 2.0
  ✓ Solution: Set temperature to 0.0 - 2.0

Error: Max concurrent agents must be greater than 0
  ✓ Solution: Ensure max_concurrent_agents > 0 in [agent]
```

## Migration and Backward Compatibility

The configuration version is tracked to support future migrations:

```toml
version = "1.0.0"
```

### Future Versions

When breaking changes are introduced:

1. Version number increments
2. Migration logic automatically handles old configs
3. Manual migration guides provided if needed
4. Fallback to defaults for missing new fields

## Performance Tuning

### High Throughput
```toml
[agent]
max_concurrent_agents = 50
worker_threads = 8
task_queue_size = 5000

[storage.cache]
cache_type = "redis"
redis_url = "redis://localhost:6379"
max_size_mb = 2048
```

### Low Resource Usage
```toml
[agent]
max_concurrent_agents = 2
worker_threads = 1
task_queue_size = 100

[storage.cache]
enabled = false
```

### High Availability
```toml
[storage.database]
database_type = "postgres"
postgres_url = "postgresql://user:pass@host/db"
pool_size = 20
enable_backups = true
backup_interval_hours = 6

[providers.anthropic]
max_retries = 5
retry_backoff_ms = 2000
```

## Example Configurations

### Development
```toml
[logging]
level = "debug"

[features]
enable_debug = true
enable_tracing = true

[providers.anthropic]
temperature = 0.8

[storage.cache]
ttl_secs = 300
```

### Production
```toml
[logging]
level = "info"

[features]
enable_debug = false

[security]
enable_encryption = true
enable_audit_logging = true

[storage.database]
pool_size = 20
auto_migrate = true
enable_backups = true

[notifications]
enabled = true
```

### Testing
```toml
[agent]
default_timeout_secs = 10
max_concurrent_agents = 2

[storage]
base_path = "/tmp/descartes-test"

[storage.cache]
enabled = true
ttl_secs = 60
```

## Troubleshooting

### "Config file not found"
- Default location: `~/.descartes/config.toml`
- Solution: Copy from example and customize
  ```bash
  mkdir -p ~/.descartes
  cp .descartes/config.toml.example ~/.descartes/config.toml
  ```

### "API Key not found"
- Ensure API key is in config OR environment variable
- Check variable name matches provider (ANTHROPIC_API_KEY, OPENAI_API_KEY)
- Verify no typos or trailing spaces

### "Connection refused"
- Check endpoint URLs are correct
- Verify API service is running/accessible
- Check network connectivity

### "Token limit exceeded"
- Reduce `max_tokens` in provider config
- Reduce context history
- Enable context compression: `enable_compression = true`

## Security Best Practices

1. **Never commit secrets**
   ```bash
   echo "api_key = " >> .gitignore
   ```

2. **Use environment variables for sensitive data**
   ```bash
   export ANTHROPIC_API_KEY=$(aws secretsmanager get-secret-value --query SecretString)
   ```

3. **Rotate encryption keys regularly**
   ```bash
   # Backup old key
   export OLD_KEY=$DESCARTES_ENCRYPTION_KEY
   # Generate new key
   export DESCARTES_ENCRYPTION_KEY=$(openssl rand -base64 32)
   ```

4. **Enable audit logging in production**
   ```toml
   [security]
   enable_audit_logging = true
   ```

5. **Restrict file permissions**
   ```bash
   chmod 0600 ~/.descartes/config.toml
   chmod -R 0700 ~/.descartes
   ```

## Support

For configuration issues:
1. Check CONFIGURATION.md (this file)
2. Review example configuration: `.descartes/config.toml.example`
3. Check logs: `~/.descartes/data/descartes.log`
4. Validate configuration: `descartes config validate`

## Configuration API

Load configuration programmatically:

```rust
use descartes_core::ConfigManager;

// Load from default location
let manager = ConfigManager::load(None)?;

// Access configuration
let config = manager.config();
println!("Primary provider: {}", config.providers.primary);

// Load from environment
let mut manager = ConfigManager::load(None)?;
manager.load_from_env()?;

// Validate configuration
manager.validate()?;

// Save modified configuration
manager.save()?;
```

## Version History

### Version 1.0.0 (Current)
- Complete configuration schema
- Provider support: Anthropic, OpenAI, Ollama, DeepSeek, Groq
- Agent behavior tuning
- Storage and cache configuration
- Security and encryption
- Notifications and alerts
- Feature flags and logging
