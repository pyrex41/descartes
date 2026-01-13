# Descartes Configuration Directory

This directory contains the Descartes configuration and data files.

## Directory Structure

```
~/.descartes/
├── config.toml              # Main configuration file (create from config.toml.example)
├── config.toml.example      # Example configuration template
├── config.toml.bak          # Automatic backup before migrations
├── data/
│   ├── descartes.db         # Main SQLite database
│   ├── state/               # Agent state snapshots
│   ├── events/              # Event log files
│   ├── cache/               # Cache files
│   ├── descartes.log        # Application log file
│   └── audit.log            # Audit trail
├── thoughts/                # Persistent agent thoughts/memories
└── README.md                # This file
```

## Quick Start

### 1. Initialize Configuration

```bash
# Create the directory if it doesn't exist
mkdir -p ~/.descartes

# Copy example configuration
cp path/to/descartes/.descartes/config.toml.example ~/.descartes/config.toml

# Set proper permissions
chmod 0600 ~/.descartes/config.toml
```

### 2. Configure API Keys

Edit `~/.descartes/config.toml`:

```toml
[providers.anthropic]
enabled = true
api_key = "sk-ant-..."  # Or set ANTHROPIC_API_KEY env var
```

Or use environment variables (recommended):

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-proj-..."
```

### 3. Start Using Descartes

```bash
# Descartes will create data directories automatically
descartes --help
```

## Configuration Files

### config.toml (Required)

Main configuration file. Create from `config.toml.example`:

- Provider settings (API keys, endpoints, models)
- Agent behavior tuning
- Storage configuration
- Security settings
- Notification channels
- Feature flags

### config.toml.example

Template configuration file with all available options and documentation.

Copy this file to `config.toml` and customize for your setup.

### config.toml.bak

Automatic backup created before configuration migrations.

Restore if needed:
```bash
cp ~/.descartes/config.toml.bak ~/.descartes/config.toml
```

## Data Files

### descartes.db

Main SQLite database containing:
- Agent execution history
- Task state and results
- User sessions (if RBAC enabled)
- Configuration metadata

```bash
# View database
sqlite3 ~/.descartes/data/descartes.db ".tables"

# Backup database
cp ~/.descartes/data/descartes.db ~/.descartes/data/descartes.db.backup
```

### state/

Agent state snapshots for recovery and persistence:

- `state/agent-{id}.json` - Individual agent state
- `state/context-cache.json` - Context cache
- `state/tasks.json` - Task state

### events/

Event stream logs:

- `events/events-{date}.ndjson` - NDJSON event logs
- `events/index.db` - Event index for fast search

Events are automatically indexed for fast querying.

### cache/

Temporary cache files (in-memory cache by default):

- `cache/contexts/` - Cached context windows
- `cache/tokens/` - Token count cache
- `cache/responses/` - Response cache

## Logs

### descartes.log

Main application log file containing:
- Startup/shutdown messages
- Configuration loading
- Agent execution
- API calls and responses
- Errors and warnings

View logs:
```bash
# Last 50 lines
tail -50 ~/.descartes/data/descartes.log

# Follow logs in real-time
tail -f ~/.descartes/data/descartes.log

# Search logs
grep "ERROR\|WARN" ~/.descartes/data/descartes.log
```

### audit.log

Security audit trail (if enabled):
- Configuration changes
- API key access
- Session creation/termination
- Failed authentication attempts

## Maintenance

### Regular Backups

```bash
# Daily backup
cp -r ~/.descartes ~/.descartes.backup.$(date +%Y%m%d)

# Or automated backup
0 2 * * * cp -r ~/.descartes ~/.descartes.backup.$(date +\%Y\%m\%d)  # Daily at 2 AM
```

### Database Maintenance

```bash
# Optimize database (SQLite)
sqlite3 ~/.descartes/data/descartes.db "VACUUM;"
sqlite3 ~/.descartes/data/descartes.db "ANALYZE;"

# Backup before maintenance
cp ~/.descartes/data/descartes.db ~/.descartes/data/descartes.db.backup
```

### Log Rotation

Logs are automatically rotated based on `config.toml` settings:

```toml
[logging.targets.file]
max_size_mb = 100        # Rotate when 100 MB
max_backups = 10         # Keep 10 old logs
compress = false         # Compress old logs
```

Old logs are stored as:
- `descartes.log` - Current log
- `descartes.log.1` - Previous log
- `descartes.log.2` - Older log
- etc.

### Cleanup Old Data

```bash
# Remove old backups (keep last 7 days)
find ~/.descartes -name "*.backup.*" -mtime +7 -delete

# Remove old event logs (keep 30 days)
find ~/.descartes/data/events -name "*.ndjson" -mtime +30 -delete

# Archive logs older than 90 days
find ~/.descartes/data -name "*.log*" -mtime +90 -exec gzip {} \;
```

## Troubleshooting

### Configuration Issues

See [CONFIGURATION.md](../../CONFIGURATION.md) in the main project directory.

### Storage Issues

**Problem:** Disk space running out

```bash
# Check disk usage
du -sh ~/.descartes

# Find large files
find ~/.descartes -size +100M

# Clear old logs
rm ~/.descartes/data/*.log.* 2>/dev/null
find ~/.descartes/data/events -mtime +30 -delete
```

**Problem:** Database corruption

```bash
# Check database integrity (SQLite)
sqlite3 ~/.descartes/data/descartes.db "PRAGMA integrity_check;"

# Restore from backup if corrupted
cp ~/.descartes/data/descartes.db.backup ~/.descartes/data/descartes.db
```

### Permission Issues

**Problem:** Permission denied when accessing config

```bash
# Fix permissions
chmod 0600 ~/.descartes/config.toml
chmod 0700 ~/.descartes

# Verify permissions
ls -la ~/.descartes
```

### API Connectivity

**Problem:** Cannot connect to provider API

```bash
# Check configuration
grep "endpoint" ~/.descartes/config.toml

# Test connectivity
curl -i https://api.anthropic.com/v1/models \
  -H "x-api-key: $ANTHROPIC_API_KEY"

# Check logs for errors
grep "ApiError\|Connection" ~/.descartes/data/descartes.log
```

## Security

### Protecting Configuration

```bash
# Strict permissions
chmod 0600 ~/.descartes/config.toml

# Entire directory
chmod 0700 ~/.descartes

# Never commit to git
echo "config.toml" >> ~/.gitignore
```

### API Key Management

```bash
# Use environment variables (recommended)
export ANTHROPIC_API_KEY="sk-ant-..."

# Never hardcode in config files in production
# Use: api_key = "$ANTHROPIC_API_KEY" and set env var

# Rotate keys periodically
# 1. Generate new key in API dashboard
# 2. Update environment variable
# 3. Restart application
# 4. Revoke old key
```

### Encryption at Rest

Enable encryption for sensitive data:

```toml
[security]
enable_encryption = true
encrypt_api_keys = true
encryption_key = "..."  # Or set DESCARTES_ENCRYPTION_KEY env var
```

Generate encryption key:
```bash
openssl rand -base64 32
```

## Space Requirements

### Typical Disk Usage

- **Config + Data:** 10-100 MB (with SQLite)
- **Logs:** 50-500 MB (varies by activity)
- **Events:** 100 MB - 1 GB (depends on retention)
- **Cache:** 100 MB - 500 MB (configurable)

### Recommended Storage

- **Development:** 1 GB
- **Production:** 10-50 GB (depending on scale)

### Cleanup Strategy

```bash
# Aggressive cleanup (delete 90+ day old data)
./cleanup.sh --aggressive

# Conservative cleanup (delete 180+ day old data)
./cleanup.sh --conservative

# Custom retention (delete 60+ day old data)
./cleanup.sh --days 60
```

## Configuration Documentation

For detailed configuration documentation, see:

1. **[CONFIGURATION.md](../../CONFIGURATION.md)** - Complete reference
   - All configuration sections
   - Provider setup
   - Security settings
   - Performance tuning

2. **[CONFIG_MIGRATION.md](../../CONFIG_MIGRATION.md)** - Version upgrades
   - Backward compatibility
   - Migration procedures
   - Breaking changes

3. **[CONFIG_VALIDATION.md](../../CONFIG_VALIDATION.md)** - Validation guide
   - Validation rules
   - Error troubleshooting
   - Pre-flight checks

## Environment Variables

Set these to override config file settings:

```bash
# Providers
export ANTHROPIC_API_KEY="sk-ant-..."
export OPENAI_API_KEY="sk-proj-..."
export DEEPSEEK_API_KEY="sk-..."
export GROQ_API_KEY="gsk_..."

# Security
export DESCARTES_ENCRYPTION_KEY="$(openssl rand -base64 32)"
export DESCARTES_SECRET_KEY="$(openssl rand -base64 32)"

# Configuration
export DESCARTES_CONFIG_PATH="/path/to/config.toml"
```

## Related Documentation

- **[README.md](../../README.md)** - Main project documentation
- **[CONFIGURATION.md](../../CONFIGURATION.md)** - Detailed configuration guide
- **[CONFIG_MIGRATION.md](../../CONFIG_MIGRATION.md)** - Version migration guide
- **[CONFIG_VALIDATION.md](../../CONFIG_VALIDATION.md)** - Validation reference

## Support

For issues or questions:

1. Check relevant documentation above
2. Review `~/.descartes/data/descartes.log` for errors
3. Validate configuration with provided tools
4. Check example configuration: `config.toml.example`

## License

Configuration files and documentation are part of the Descartes project (MIT License).
