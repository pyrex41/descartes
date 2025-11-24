# Configuration Quick Start Guide

Fast setup guide for Descartes configuration.

## 30-Second Setup

```bash
# 1. Create config directory
mkdir -p ~/.descartes

# 2. Copy example config
cp descartes/.descartes/config.toml.example ~/.descartes/config.toml

# 3. Set API key
export ANTHROPIC_API_KEY="sk-ant-..."

# 4. Done! Descartes is ready to use
```

## Configuration Files at a Glance

| File | Location | Purpose |
|------|----------|---------|
| Config | `~/.descartes/config.toml` | Main configuration (you edit this) |
| Example | `descartes/.descartes/config.toml.example` | Template with all options |
| Database | `~/.descartes/data/descartes.db` | SQLite database (auto-created) |
| Logs | `~/.descartes/data/descartes.log` | Application logs |

## Essential Configuration

### Minimal Setup (Anthropic)

```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
# Set ANTHROPIC_API_KEY environment variable instead
```

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

### With Custom Models

```toml
[providers.anthropic]
enabled = true
model = "claude-3-opus-20240229"  # Change default model
models = [
    "claude-3-5-sonnet-20241022",
    "claude-3-opus-20240229",
    "claude-3-haiku-20240307"
]
```

### Multiple Providers

```toml
[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true

[providers.openai]
enabled = true

[providers.ollama]
enabled = true
endpoint = "http://localhost:11434"  # If Ollama is running
```

## Environment Variables

Set these in your shell or `.env` file:

```bash
# Required API Keys
export ANTHROPIC_API_KEY="sk-ant-..."          # Anthropic
export OPENAI_API_KEY="sk-proj-..."             # OpenAI
export DEEPSEEK_API_KEY="sk-..."                # DeepSeek
export GROQ_API_KEY="gsk_..."                   # Groq

# Security (optional)
export DESCARTES_ENCRYPTION_KEY="$(openssl rand -base64 32)"
export DESCARTES_SECRET_KEY="$(openssl rand -base64 32)"

# Alternative config location (optional)
export DESCARTES_CONFIG_PATH="/custom/path/config.toml"
```

### Setting Environment Variables

#### Bash/Zsh
```bash
# Add to ~/.bashrc or ~/.zshrc
export ANTHROPIC_API_KEY="sk-ant-..."

# Reload shell
source ~/.bashrc  # or source ~/.zshrc
```

#### Permanently (Bash)
```bash
echo 'export ANTHROPIC_API_KEY="sk-ant-..."' >> ~/.bashrc
source ~/.bashrc
```

#### Using .env file
```bash
# Create ~/.descartes/.env
cat > ~/.descartes/.env << EOF
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-proj-..."
EOF

# Load before running
source ~/.descartes/.env
```

## Common Configurations

### Development
```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
temperature = 0.8

[logging]
level = "debug"

[features]
enable_debug = true
```

### Production
```toml
version = "1.0.0"

[providers]
primary = "anthropic"

[providers.anthropic]
enabled = true
max_retries = 5
rate_limit_rpm = 30

[storage.database]
pool_size = 20
enable_backups = true

[security]
enable_encryption = true
enable_audit_logging = true

[logging]
level = "info"

[notifications]
enabled = true
```

### Local Testing (Ollama)
```toml
version = "1.0.0"

[providers]
primary = "ollama"

[providers.ollama]
enabled = true
endpoint = "http://localhost:11434"
model = "llama2"
```

## Configuration Sections Explained

### [providers]
LLM provider settings. Choose one as `primary`.

Available providers:
- `anthropic` - Claude models
- `openai` - GPT models
- `ollama` - Local inference
- `deepseek` - DeepSeek models
- `groq` - Groq models
- `custom.*` - Custom providers

### [agent]
How agents execute and behave.

Key settings:
- `max_concurrent_agents` - How many agents run simultaneously
- `default_timeout_secs` - Execution timeout
- `enable_streaming` - Stream responses
- `enable_tools` - Allow function calls

### [storage]
Where data is stored.

Database options:
- `sqlite` - Default, no setup needed
- `postgres` - For production
- `mysql` - Alternative database

### [security]
Encryption and access control.

Key settings:
- `enable_encryption` - Encrypt sensitive data
- `encrypt_api_keys` - Encrypt stored API keys
- `enable_audit_logging` - Log security events

### [notifications]
Alerts and notifications.

Channels:
- `telegram` - Telegram bot
- `slack` - Slack webhooks
- `email` - SMTP email
- `webhooks` - Generic webhooks

### [logging]
Application logging.

Options:
- `level` - Log verbosity (trace, debug, info, warn, error)
- `format` - Log format (json, text, pretty)
- `targets.file` - Log to file with rotation

## Validation

Check configuration syntax:

```bash
# Using Python (if installed)
python3 -c "import tomllib; tomllib.load(open('$HOME/.descartes/config.toml', 'rb'))"

# Using cargo (if in project)
cd descartes
cargo run --bin descartes -- config validate
```

## Troubleshooting

### "API key not found"
```bash
# Check key is set
echo $ANTHROPIC_API_KEY

# Set it
export ANTHROPIC_API_KEY="sk-ant-..."

# Add to shell config for persistence
echo 'export ANTHROPIC_API_KEY="sk-ant-..."' >> ~/.bashrc
```

### "Config file not found"
```bash
# Create it
mkdir -p ~/.descartes
cp descartes/.descartes/config.toml.example ~/.descartes/config.toml
```

### "Connection refused"
```bash
# Check endpoint in config
grep endpoint ~/.descartes/config.toml

# Test connectivity
curl -i https://api.anthropic.com/v1/models \
  -H "x-api-key: $ANTHROPIC_API_KEY"
```

### "Permission denied"
```bash
# Fix file permissions
chmod 0600 ~/.descartes/config.toml
chmod 0700 ~/.descartes
```

## Security Checklist

- [ ] Config file has 0600 permissions: `chmod 0600 ~/.descartes/config.toml`
- [ ] API keys in environment variables (not in config file)
- [ ] Config directory has 0700 permissions: `chmod 0700 ~/.descartes`
- [ ] Don't commit config file to git
- [ ] Use encryption key for sensitive data
- [ ] Keep backups of config: `cp ~/.descartes/config.toml ~/.descartes/config.toml.bak`

## Full Configuration Reference

For complete configuration documentation, see:

1. **[CONFIGURATION.md](CONFIGURATION.md)** - All options explained
2. **[CONFIG_VALIDATION.md](CONFIG_VALIDATION.md)** - Validation rules
3. **[CONFIG_MIGRATION.md](CONFIG_MIGRATION.md)** - Version upgrades
4. **[.descartes/README.md](.descartes/README.md)** - Directory structure

## Next Steps

1. Create and configure `~/.descartes/config.toml`
2. Set environment variables for API keys
3. Validate configuration (if available)
4. Run health check (if available)
5. Start using Descartes
6. Review logs if issues: `tail -f ~/.descartes/data/descartes.log`

## Examples by Use Case

### Just getting started?
Use minimal Anthropic config (see "Minimal Setup" above)

### Testing locally?
Use Ollama config (see "Local Testing" above)

### Using multiple providers?
Configure multiple providers and set `primary` (see "Multiple Providers" above)

### Going to production?
Use production config (see "Production" above)

### Need help?
1. Check relevant documentation
2. Review config example: `descartes/.descartes/config.toml.example`
3. Check logs: `~/.descartes/data/descartes.log`
4. Validate config: `descartes config validate`

## Configuration Hierarchy

Settings are applied in this order (last wins):
1. Built-in defaults
2. Configuration file
3. Environment variables

Example:
```toml
# In config file
[providers.anthropic]
temperature = 0.7
```

```bash
# Override with environment (if implemented)
ANTHROPIC_TEMPERATURE=0.9
```

## API Documentation

Load configuration in your code:

```rust
use descartes_core::ConfigManager;

// Load from default location (~/.descartes/config.toml)
let manager = ConfigManager::load(None)?;

// Or specify custom path
let manager = ConfigManager::load(Some(Path::new("/path/to/config.toml")))?;

// Validate configuration
manager.validate()?;

// Access configuration
let config = manager.config();
let primary = &config.providers.primary;
let timeout = config.agent.default_timeout_secs;

// Load from environment variables
let mut manager = ConfigManager::load(None)?;
manager.load_from_env()?;

// Save modified configuration
manager.save()?;
```

## Performance Settings

### For high throughput:
```toml
[agent]
max_concurrent_agents = 50
worker_threads = 8

[storage.cache]
cache_type = "redis"
max_size_mb = 2048
```

### For low resources:
```toml
[agent]
max_concurrent_agents = 2
worker_threads = 1

[storage.cache]
enabled = false
```

## Support

- Configuration documentation: [CONFIGURATION.md](CONFIGURATION.md)
- Validation guide: [CONFIG_VALIDATION.md](CONFIG_VALIDATION.md)
- Migration guide: [CONFIG_MIGRATION.md](CONFIG_MIGRATION.md)
- Directory guide: [.descartes/README.md](.descartes/README.md)
