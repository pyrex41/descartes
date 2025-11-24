# Configuration System - Usage Examples

## Quick Start

### Basic Usage

```bash
# Use default config discovery (.descartes/config.toml or ~/.descartes/config.toml)
descartes spawn --task "analyze this code"

# Specify custom config path
descartes --config /etc/descartes/config.toml spawn --task "analyze this code"

# Override log level for debugging
descartes --log-level debug spawn --task "analyze this code"
```

### Environment Variable Configuration

```bash
# Set API key
export ANTHROPIC_API_KEY="sk-ant-abcd1234"

# Set storage path
export DESCARTES_STORAGE_PATH="/var/lib/descartes"

# Set log level
export DESCARTES_LOG_LEVEL="debug"

# Run command
descartes spawn --task "my task"
```

## Configuration File Examples

### Minimal Configuration (in `.descartes/config.toml`)

```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
model = "claude-3-5-sonnet-20241022"
```

### Full Configuration

```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
api_key = "sk-ant-abcd1234"  # or set via ANTHROPIC_API_KEY
endpoint = "https://api.anthropic.com"
model = "claude-3-5-sonnet-20241022"
temperature = 0.7
max_tokens = 4096
timeout_secs = 120
max_retries = 3
retry_backoff_ms = 1000
rate_limit_rpm = 60

[providers.openai]
enabled = false
api_key = ""  # set via OPENAI_API_KEY
model = "gpt-4-turbo"

[providers.ollama]
enabled = false
endpoint = "http://localhost:11434"
model = "llama2"

[agent]
default_timeout_secs = 300
max_concurrent_agents = 10
task_queue_size = 1000
enable_streaming = true
enable_tools = true
max_tool_retries = 2
tool_timeout_secs = 60
enable_memory = true
memory_ttl_secs = 3600
max_context_tokens = 32000
enable_background_tasks = true
worker_threads = 4

[storage]
base_path = "~/.descartes"

[storage.database]
database_type = "sqlite"
sqlite_path = "data/descartes.db"
pool_size = 10
auto_migrate = true
enable_backups = true
backup_interval_hours = 24

[storage.state_store]
enabled = true
path = "data/state"
serialization_format = "json"
enable_compression = false

[storage.event_store]
enabled = true
path = "data/events"
retention_days = 0  # 0 = infinite
batch_size = 100
enable_indexing = true

[storage.cache]
enabled = true
cache_type = "in-memory"  # or "disk" or "redis"
disk_path = "data/cache"
ttl_secs = 3600
max_size_mb = 512

[security]
enable_encryption = true
encryption_algorithm = "aes-256-gcm"
encryption_key = ""  # set via DESCARTES_ENCRYPTION_KEY
encrypt_api_keys = true
file_permissions = "0600"
enable_rbac = false
secret_key = ""  # set via DESCARTES_SECRET_KEY
session_timeout_secs = 3600
enable_audit_logging = true
audit_log_path = "data/audit.log"

[logging]
level = "info"  # trace, debug, info, warn, error
format = "text"  # text, json, pretty

[logging.targets]
stdout = true

[logging.targets.file]
path = "logs/descartes.log"
max_size_mb = 100
max_backups = 10
compress = false

[features]
enable_experimental = false
enable_debug = false
enable_tracing = true
enable_profiling = false

[notifications]
enabled = false

[notifications.alerts]
high_token_usage_threshold = 80
alert_on_api_errors = true
alert_on_agent_failures = true
alert_on_timeouts = true
alert_on_rate_limit = true
```

## Environment Variable Examples

### API Keys

```bash
# Anthropic
export ANTHROPIC_API_KEY="sk-ant-abcd1234"

# OpenAI
export OPENAI_API_KEY="sk-abc123"

# DeepSeek
export DEEPSEEK_API_KEY="sk-abcd1234"

# Groq
export GROQ_API_KEY="gsk_abcd1234"
```

### Storage and Paths

```bash
# Custom storage directory
export DESCARTES_STORAGE_PATH="/data/descartes"

# Custom encryption key
export DESCARTES_ENCRYPTION_KEY="my-secret-key-32-chars-long"

# Custom session secret
export DESCARTES_SECRET_KEY="session-secret-key"
```

### Logging

```bash
# Set log level
export DESCARTES_LOG_LEVEL="debug"

# Options: trace, debug, info, warn, error
```

## Practical Examples

### Example 1: Development Setup

```bash
#!/bin/bash

# Set development configuration
export DESCARTES_LOG_LEVEL="debug"
export DESCARTES_STORAGE_PATH="/tmp/descartes-dev"
export ANTHROPIC_API_KEY="sk-ant-dev-key"

# Create config
mkdir -p ~/.descartes
cat > ~/.descartes/config.toml << 'EOF'
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
model = "claude-3-5-sonnet-20241022"
temperature = 0.7

[logging]
level = "debug"

[agent]
enable_background_tasks = false
EOF

# Run
descartes spawn --task "test task"
```

### Example 2: Production Setup

```bash
#!/bin/bash

# Set production configuration
export DESCARTES_LOG_LEVEL="warn"
export DESCARTES_STORAGE_PATH="/var/lib/descartes"
export ANTHROPIC_API_KEY="sk-ant-prod-key"
export DESCARTES_ENCRYPTION_KEY="prod-encryption-key-32-chars"

# Run with explicit config
descartes --config /etc/descartes/config.toml spawn --task "production task"
```

### Example 3: Using Multiple Providers

```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
model = "claude-3-5-sonnet-20241022"

[providers.openai]
enabled = true
model = "gpt-4-turbo"
temperature = 0.5

[providers.ollama]
enabled = true
endpoint = "http://localhost:11434"
model = "neural-chat"
```

Then select provider per command (future feature):
```bash
descartes spawn --task "task" --provider anthropic
descartes spawn --task "task" --provider openai
descartes spawn --task "task" --provider ollama
```

### Example 4: Custom Storage Path

```toml
[storage]
base_path = "/mnt/fast-storage/descartes"

[storage.database]
database_type = "sqlite"
sqlite_path = "data/descartes.db"
pool_size = 20

[storage.cache]
cache_type = "redis"
redis_url = "redis://localhost:6379"
```

### Example 5: Notification Setup

```toml
[notifications]
enabled = true

[notifications.channels.telegram]
bot_token = "telegram-bot-token"
chat_id = "telegram-chat-id"
enabled = true

[notifications.channels.slack]
webhook_url = "https://hooks.slack.com/services/..."
channel = "#descartes-alerts"
enabled = true

[notifications.alerts]
high_token_usage_threshold = 80
alert_on_api_errors = true
alert_on_agent_failures = true
alert_on_timeouts = true
alert_on_rate_limit = true
```

## Configuration Discovery Flow

The system looks for configuration in this order:

1. **Explicit Path**: If `--config` flag is provided
   ```bash
   descartes --config /path/to/config.toml spawn --task "..."
   ```

2. **Local Directory**: `.descartes/config.toml` in current directory
   ```bash
   cd /project
   # Looks for /project/.descartes/config.toml
   descartes spawn --task "..."
   ```

3. **User Home**: `~/.descartes/config.toml` in home directory
   ```bash
   # Looks for /home/user/.descartes/config.toml
   descartes spawn --task "..."
   ```

4. **Environment Variable**: `$DESCARTES_CONFIG`
   ```bash
   export DESCARTES_CONFIG="/etc/descartes/config.toml"
   descartes spawn --task "..."
   ```

5. **Defaults**: Built-in defaults if no file found
   ```bash
   # Uses all default values
   descartes spawn --task "..."
   ```

## Override Precedence

When a setting appears in multiple places, this is the precedence (highest to lowest):

1. **Environment Variables** (highest priority)
   ```bash
   export ANTHROPIC_API_KEY="env-value"
   ```

2. **Config File**
   ```toml
   # [providers.anthropic]
   # api_key = "config-value"
   ```

3. **Defaults** (lowest priority)
   - Built into code

Example with API key:
```bash
# This will use env var, not config file
export ANTHROPIC_API_KEY="sk-ant-env"
descartes spawn --task "..."
# Uses: sk-ant-env
```

## Debugging Configuration

### View Effective Configuration

```bash
# Enable debug logging to see what config was loaded
descartes --log-level debug spawn --task "..." 2>&1 | grep -i "configuration\|loading\|config"

# Output will show:
# [INFO] Loading Descartes configuration...
# [INFO] Configuration loaded from: "/home/user/.descartes/config.toml"
# [INFO] Applied environment variable overrides to configuration
# [INFO] Configuration loaded and validated successfully
# [INFO] All configuration directories verified/created
```

### Test Configuration Loading

```bash
# Create a test config
mkdir -p ~/.descartes
cat > ~/.descartes/config.toml << 'EOF'
version = "1.0.0"
[providers]
primary = "anthropic"
EOF

# Test with debug logging
descartes --log-level debug --config ~/.descartes/config.toml spawn --task "test" 2>&1 | head -20
```

### Validate Configuration

The system automatically validates:
- ✓ At least one provider is available
- ✓ Database pool size > 0
- ✓ Temperature ranges (0.0 to 2.0)
- ✓ Max concurrent agents > 0
- ✓ Storage paths exist or can be created

Validation errors will show:
```
[ERROR] Configuration loading failed: Database pool size must be greater than 0
```

## Common Issues and Solutions

### Issue: "Config file not found"

**Solution**: Create config file in expected location:
```bash
mkdir -p ~/.descartes
cat > ~/.descartes/config.toml << 'EOF'
version = "1.0.0"
[providers]
primary = "anthropic"
EOF
```

### Issue: "No providers configured"

**Solution**: Ensure at least one provider is enabled and has valid settings:
```toml
[providers.anthropic]
enabled = true
api_key = "sk-ant-..."  # or set ANTHROPIC_API_KEY env var
```

### Issue: "API key not found"

**Solution**: Set via environment variable or config file:
```bash
export ANTHROPIC_API_KEY="sk-ant-..."
# or
# In ~/.descartes/config.toml:
# [providers.anthropic]
# api_key = "sk-ant-..."
```

### Issue: Permissions error on storage directory

**Solution**: Ensure directory is writable:
```bash
mkdir -p ~/.descartes/data
chmod 700 ~/.descartes
chmod 700 ~/.descartes/data
```

### Issue: Can't find config in different directory

**Solution**: Use explicit path or DESCARTES_CONFIG env var:
```bash
# Option 1: Explicit flag
descartes --config /etc/descartes/config.toml spawn --task "..."

# Option 2: Environment variable
export DESCARTES_CONFIG="/etc/descartes/config.toml"
descartes spawn --task "..."
```

## Next Steps

1. **Create Initial Configuration**: Set up `~/.descartes/config.toml`
2. **Set API Keys**: Via environment variables or config file
3. **Verify Settings**: Run with `--log-level debug` to see config
4. **Start Using**: Run any Descartes command
