# Configuration Validation and Testing

Guide to validating Descartes configuration and ensuring proper setup.

## Overview

The configuration system includes built-in validation to catch common issues early and provide clear error messages.

## Validation Process

### Automatic Validation

Validation runs automatically when:
1. Configuration is loaded from file
2. Configuration is modified via API
3. Application starts up
4. User explicitly calls `validate()`

### Validation Rules

#### Provider Settings

```rust
// At least one provider must be configured
if !self.config.providers.openai.api_key.is_some()
    && !self.config.providers.anthropic.api_key.is_some()
    && !self.config.providers.ollama.enabled
    && self.config.providers.custom.is_empty() {
    warn!("No providers configured");
}

// Primary provider must exist
if !self.is_provider_enabled(&self.config.providers.primary) {
    error!("Primary provider '{}' is not enabled", self.config.providers.primary);
}

// API endpoints must be valid URLs
if !is_valid_url(&self.config.providers.openai.endpoint) {
    error!("Invalid OpenAI endpoint URL");
}

// Temperature must be in valid range
for provider in [&openai, &anthropic, &ollama, &deepseek, &groq] {
    if provider.temperature < 0.0 || provider.temperature > 2.0 {
        error!("Temperature must be between 0.0 and 2.0");
    }
}

// Timeout values must be positive
if self.config.agent.default_timeout_secs == 0 {
    error!("default_timeout_secs must be greater than 0");
}
```

#### Storage Settings

```rust
// Database pool size must be positive
if self.config.storage.database.pool_size == 0 {
    error!("Database pool_size must be greater than 0");
}

// Database type must be supported
match self.config.storage.database.database_type.as_str() {
    "sqlite" | "postgres" | "mysql" => {},
    other => error!("Unsupported database type: {}", other),
}

// Storage paths must not be empty
if self.config.storage.base_path.is_empty() {
    error!("Storage base_path cannot be empty");
}

// Cache TTL must be positive
if self.config.storage.cache.ttl_secs == 0 {
    error!("Cache TTL must be greater than 0");
}
```

#### Agent Settings

```rust
// Max concurrent agents must be positive
if self.config.agent.max_concurrent_agents == 0 {
    error!("max_concurrent_agents must be greater than 0");
}

// Task queue size must be reasonable
if self.config.agent.task_queue_size == 0 {
    warn!("task_queue_size is 0, no tasks can be queued");
}

// Context size must not exceed model limits
for provider in [...] {
    if self.config.agent.max_context_tokens > provider.max_tokens {
        warn!("max_context_tokens exceeds provider max_tokens");
    }
}

// Worker threads should be positive
if self.config.agent.enable_background_tasks && self.config.agent.worker_threads == 0 {
    error!("worker_threads must be > 0 if background tasks enabled");
}
```

#### Security Settings

```rust
// Encryption key required if encryption enabled
if self.config.security.enable_encryption {
    if self.config.security.encryption_key.is_none()
        && std::env::var("DESCARTES_ENCRYPTION_KEY").is_err() {
        error!("Encryption enabled but no encryption key provided");
    }
}

// File permissions must be valid octal
if !is_valid_octal(&self.config.security.file_permissions) {
    error!("Invalid file permissions: {}", self.config.security.file_permissions);
}

// Session timeout must be positive
if self.config.security.session_timeout_secs == 0 {
    error!("session_timeout_secs must be greater than 0");
}
```

#### Notification Settings

```rust
// Telegram: both token and chat_id required
if let Some(telegram) = &self.config.notifications.channels.telegram {
    if telegram.enabled {
        if telegram.bot_token.is_empty() || telegram.chat_id.is_empty() {
            error!("Telegram enabled but missing bot_token or chat_id");
        }
    }
}

// Email: all SMTP fields required
if let Some(email) = &self.config.notifications.channels.email {
    if email.enabled {
        if email.smtp_server.is_empty()
            || email.smtp_user.is_empty()
            || email.smtp_password.is_empty() {
            error!("Email enabled but missing SMTP credentials");
        }
        if email.recipients.is_empty() {
            error!("Email enabled but no recipients specified");
        }
    }
}

// Webhooks: URL must be valid
for webhook in &self.config.notifications.channels.webhooks {
    if !is_valid_url(&webhook.url) {
        error!("Invalid webhook URL: {}", webhook.url);
    }
}

// Alert threshold must be 0-100
if self.config.notifications.alerts.high_token_usage_threshold > 100 {
    error!("Alert threshold must be 0-100, got {}",
           self.config.notifications.alerts.high_token_usage_threshold);
}
```

## Manual Validation

### Validate Configuration File

```bash
# Using the CLI (when available)
descartes config validate
descartes config validate --file ~/.descartes/config.toml
```

### Programmatic Validation

```rust
use descartes_core::ConfigManager;

let manager = ConfigManager::load(None)?;
match manager.validate() {
    Ok(_) => println!("Configuration is valid"),
    Err(e) => eprintln!("Configuration error: {}", e),
}
```

## Common Validation Errors

### Error: "No providers configured"

**Cause:** No provider API keys set and Ollama not enabled.

**Fix:**
```bash
# Option 1: Set API key environment variable
export ANTHROPIC_API_KEY="sk-ant-..."

# Option 2: Add to config file
[providers.anthropic]
enabled = true
api_key = "sk-ant-..."

# Option 3: Enable local Ollama
[providers.ollama]
enabled = true
endpoint = "http://localhost:11434"
```

### Error: "Primary provider 'anthropic' is not enabled"

**Cause:** Primary provider is disabled or not configured.

**Fix:**
```toml
[providers]
primary = "openai"  # Change to enabled provider

# OR enable the primary provider
[providers.anthropic]
enabled = true
api_key = "sk-ant-..."
```

### Error: "Temperature must be between 0.0 and 2.0"

**Cause:** Invalid temperature value.

**Fix:**
```toml
# Correct range
[providers.anthropic]
temperature = 0.7  # Must be between 0.0 and 2.0

# Or for OpenAI (0.0-2.0)
[providers.openai]
temperature = 1.5
```

### Error: "Database pool_size must be greater than 0"

**Cause:** Database connection pool size is zero.

**Fix:**
```toml
[storage.database]
pool_size = 10  # Must be > 0
```

### Error: "max_concurrent_agents must be greater than 0"

**Cause:** Concurrent agent limit is zero.

**Fix:**
```toml
[agent]
max_concurrent_agents = 10  # Must be > 0
```

### Error: "Encryption enabled but no encryption key provided"

**Cause:** Encryption is enabled but key not set.

**Fix:**
```bash
# Generate and set encryption key
export DESCARTES_ENCRYPTION_KEY=$(openssl rand -base64 32)

# OR disable encryption
[security]
enable_encryption = false
```

### Error: "Telegram enabled but missing bot_token or chat_id"

**Cause:** Telegram notification enabled but credentials incomplete.

**Fix:**
```toml
[notifications.channels.telegram]
enabled = true
bot_token = "YOUR_BOT_TOKEN"
chat_id = "YOUR_CHAT_ID"

# OR disable if not needed
[notifications.channels.telegram]
enabled = false
```

## Pre-Flight Checks

Before deploying to production, run these checks:

### 1. Configuration Syntax

```bash
# Validate TOML syntax
python3 -c "import tomli; tomli.load(open('~/.descartes/config.toml', 'rb'))"
```

### 2. File Permissions

```bash
# Check config file permissions
ls -l ~/.descartes/config.toml
# Should show: -rw------- (0600)

# Fix if needed
chmod 0600 ~/.descartes/config.toml
```

### 3. API Connectivity

```bash
# Test Anthropic
curl -i https://api.anthropic.com/v1/models \
  -H "x-api-key: $ANTHROPIC_API_KEY"

# Test OpenAI
curl https://api.openai.com/v1/models \
  -H "Authorization: Bearer $OPENAI_API_KEY"

# Test Ollama
curl http://localhost:11434/api/tags
```

### 4. Storage Paths

```bash
# Verify storage directory exists
ls -la ~/.descartes/

# Create if needed
mkdir -p ~/.descartes/data

# Check write permissions
touch ~/.descartes/data/test.txt && rm ~/.descartes/data/test.txt
```

### 5. Database Connectivity

```bash
# For SQLite (should work automatically)
ls -la ~/.descartes/data/descartes.db

# For PostgreSQL
psql -h localhost -U user -d descartes -c "SELECT version();"

# For MySQL
mysql -h localhost -u user -p descartes -e "SELECT VERSION();"
```

### 6. Environment Variables

```bash
# Check all required env vars are set
echo "ANTHROPIC_API_KEY=${ANTHROPIC_API_KEY:-(not set)}"
echo "DESCARTES_ENCRYPTION_KEY=${DESCARTES_ENCRYPTION_KEY:-(not set)}"
```

### 7. Notification Channels

```bash
# Test Slack webhook
curl -X POST $SLACK_WEBHOOK_URL \
  -H 'Content-type: application/json' \
  -d '{"text":"Test notification"}'

# Test Telegram
curl -i -X POST "https://api.telegram.org/bot$BOT_TOKEN/sendMessage" \
  -d "chat_id=$CHAT_ID" \
  -d "text=Test notification"
```

## Test Configurations

### Minimal Configuration

```toml
# ~/.descartes/config.toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
```

Requires: `ANTHROPIC_API_KEY` environment variable

### Development Configuration

```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true

[logging]
level = "debug"

[features]
enable_debug = true
```

### Production Configuration

```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
api_key = "..."
max_retries = 5

[storage.database]
database_type = "postgres"
postgres_url = "..."
pool_size = 20
enable_backups = true

[security]
enable_encryption = true
encrypt_api_keys = true
enable_audit_logging = true

[notifications]
enabled = true

[notifications.channels.slack]
enabled = true
webhook_url = "..."

[logging]
level = "info"
log_sensitive_data = false
```

## Configuration Health Check Script

```bash
#!/bin/bash
# check_config.sh - Validate Descartes configuration

echo "=== Descartes Configuration Health Check ==="

# Check file exists
if [ ! -f ~/.descartes/config.toml ]; then
    echo "ERROR: config.toml not found at ~/.descartes/config.toml"
    exit 1
fi
echo "✓ Config file found"

# Check permissions
perms=$(stat -f %A ~/.descartes/config.toml 2>/dev/null || stat -c %a ~/.descartes/config.toml)
if [ "$perms" != "0600" ] && [ "$perms" != "600" ]; then
    echo "WARNING: config.toml permissions are $perms (should be 0600)"
fi

# Check TOML syntax
if python3 -c "import tomli; tomli.load(open('$HOME/.descartes/config.toml', 'rb'))" 2>/dev/null; then
    echo "✓ TOML syntax valid"
else
    echo "ERROR: Invalid TOML syntax"
    exit 1
fi

# Check API keys
if [ -z "$ANTHROPIC_API_KEY" ]; then
    echo "WARNING: ANTHROPIC_API_KEY not set"
fi

# Check storage directory
if [ ! -d ~/.descartes/data ]; then
    echo "WARNING: Data directory not found, will be created on startup"
fi

echo "✓ Configuration health check passed"
exit 0
```

Usage:
```bash
chmod +x check_config.sh
./check_config.sh
```

## Continuous Validation

### Watch Configuration Changes

```bash
# Watch for config file changes
watch -n 5 'tail -5 ~/.descartes/data/descartes.log | grep -i "config\|error"'
```

### Monitor Validation Errors

```bash
# Show recent validation errors
grep "ValidationError\|config.*error" ~/.descartes/data/descartes.log | tail -20
```

## Testing Configuration Changes

### Dry Run Mode

```bash
# Test configuration without starting the full application
descartes --config ~/.descartes/config.test.toml --dry-run
```

### Staging Environment

```bash
# Use a separate staging config
cp ~/.descartes/config.toml ~/.descartes/config.staging.toml

# Edit staging config
vim ~/.descartes/config.staging.toml

# Test with staging config
export DESCARTES_CONFIG_PATH=~/.descartes/config.staging.toml
descartes
```

## See Also

- [CONFIGURATION.md](CONFIGURATION.md) - Complete configuration reference
- [CONFIG_MIGRATION.md](CONFIG_MIGRATION.md) - Version migration guide
- [README.md](README.md) - General project documentation
