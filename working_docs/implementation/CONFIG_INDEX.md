# Configuration Documentation Index

Complete index of all Descartes configuration documentation and resources.

## Quick Links

### For Quick Setup (5 minutes)
- **[CONFIG_QUICKSTART.md](CONFIG_QUICKSTART.md)** - 30-second setup + common configs

### For Complete Reference (30 minutes)
- **[CONFIGURATION.md](CONFIGURATION.md)** - All options explained in detail

### For Special Topics
- **[CONFIG_MIGRATION.md](CONFIG_MIGRATION.md)** - Upgrading versions
- **[CONFIG_VALIDATION.md](CONFIG_VALIDATION.md)** - Validation rules & errors
- **[.descartes/README.md](.descartes/README.md)** - File management & maintenance

### For Examples
- **[.descartes/config.toml.example](.descartes/config.toml.example)** - Complete example config

## Documentation Guide

### 1. CONFIG_QUICKSTART.md
**Best for:** Getting started quickly

Contents:
- 30-second setup
- Configuration by use case
- Environment variables setup
- Common troubleshooting
- Security checklist

When to use:
- First time setup
- Quick reference
- Common configurations

### 2. CONFIGURATION.md
**Best for:** Complete understanding

Contents:
- All configuration sections explained
- Provider setup instructions
- Default values
- Performance tuning
- Example configurations
- Troubleshooting guide
- Environment variables

When to use:
- Understanding all options
- Fine-tuning settings
- Performance optimization
- Troubleshooting issues

### 3. CONFIG_MIGRATION.md
**Best for:** Version upgrades

Contents:
- Version comparison
- Migration strategy
- Migration handlers
- Backward compatibility
- Deprecation policy
- Backup procedures
- Rollback instructions

When to use:
- Upgrading Descartes
- Understanding version changes
- Handling breaking changes
- Planning migrations

### 4. CONFIG_VALIDATION.md
**Best for:** Troubleshooting

Contents:
- Validation rules (13 documented)
- Common errors and fixes
- Pre-flight checks
- Health check script
- Test configurations
- Continuous validation

When to use:
- Configuration validation fails
- Troubleshooting errors
- Pre-deployment checks
- Testing configurations

### 5. .descartes/README.md
**Best for:** File and directory management

Contents:
- Directory structure
- File purposes
- Data file management
- Log file handling
- Database maintenance
- Backup procedures
- Security practices

When to use:
- Managing storage
- Maintaining database
- Handling logs
- Understanding directory structure

### 6. .descartes/config.toml.example
**Best for:** Configuration template

Contents:
- All configuration options
- Comments for each setting
- Recommended defaults
- Setup instructions
- Inline documentation

When to use:
- Creating config file
- Reference for all options
- Understanding defaults
- Setting up new environment

## By Use Case

### I want to get started quickly
1. Read: [CONFIG_QUICKSTART.md](CONFIG_QUICKSTART.md) (5 min)
2. Run: Copy config.toml.example
3. Set: Environment variables
4. Done!

### I want to understand all configuration options
1. Read: [CONFIGURATION.md](CONFIGURATION.md) (30 min)
2. Reference: [.descartes/config.toml.example](.descartes/config.toml.example)
3. Explore: Specific sections that interest you

### I need to troubleshoot an error
1. Check: [CONFIG_VALIDATION.md](CONFIG_VALIDATION.md) - Common errors
2. Find: Your error in the table
3. Apply: Suggested fix
4. Validate: Using provided checks

### I'm upgrading Descartes
1. Read: [CONFIG_MIGRATION.md](CONFIG_MIGRATION.md)
2. Backup: Current configuration
3. Review: Breaking changes
4. Test: Migration in staging
5. Apply: To production

### I need to manage storage/logs
1. Read: [.descartes/README.md](.descartes/README.md)
2. Check: Current usage
3. Plan: Cleanup strategy
4. Execute: Maintenance tasks

## Configuration Sections Overview

### Providers
Where to find: [CONFIGURATION.md - Providers](CONFIGURATION.md#1-providers-configuration)
- Anthropic
- OpenAI
- Ollama
- DeepSeek
- Groq
- Custom providers

### Agent Behavior
Where to find: [CONFIGURATION.md - Agent Behavior](CONFIGURATION.md#2-agent-behavior-configuration)
- Timeouts
- Concurrency
- Streaming
- Tools
- Memory/caching

### Storage
Where to find: [CONFIGURATION.md - Storage](CONFIGURATION.md#3-storage-configuration)
- Database
- State store
- Event store
- Cache

### Security
Where to find: [CONFIGURATION.md - Security](CONFIGURATION.md#4-security-configuration)
- Encryption
- File permissions
- Access control
- Audit logging

### Notifications
Where to find: [CONFIGURATION.md - Notifications](CONFIGURATION.md#5-notifications-configuration)
- Telegram
- Slack
- Email
- Webhooks

### Logging
Where to find: [CONFIGURATION.md - Logging](CONFIGURATION.md#7-logging-configuration)
- Log levels
- Formats
- Output targets
- File rotation

### Features
Where to find: [CONFIGURATION.md - Feature Flags](CONFIGURATION.md#6-feature-flags-configuration)
- Experimental features
- Debug mode
- Custom flags
- Beta features

## Common Tasks

### Set up Anthropic provider
See: [CONFIGURATION.md - Anthropic](CONFIGURATION.md#anthropic)
Also: [CONFIG_QUICKSTART.md - Essential Configuration](CONFIG_QUICKSTART.md#essential-configuration)

### Configure multiple providers
See: [CONFIGURATION.md - Providers](CONFIGURATION.md#1-providers-configuration)
Also: [CONFIG_QUICKSTART.md - Multiple Providers](CONFIG_QUICKSTART.md#multiple-providers)

### Setup notifications
See: [CONFIGURATION.md - Notifications](CONFIGURATION.md#5-notifications-configuration)
Also: [.descartes/config.toml.example - Notifications]((.descartes/config.toml.example)

### Encrypt sensitive data
See: [CONFIGURATION.md - Security](CONFIGURATION.md#4-security-configuration)
Also: [CONFIG_QUICKSTART.md - Security Checklist](CONFIG_QUICKSTART.md#security-checklist)

### Optimize for high throughput
See: [CONFIGURATION.md - Performance Tuning](CONFIGURATION.md#performance-tuning)
Also: [CONFIG_QUICKSTART.md - Performance Settings](CONFIG_QUICKSTART.md#performance-settings)

### Troubleshoot configuration errors
See: [CONFIG_VALIDATION.md - Common Validation Errors](CONFIG_VALIDATION.md#common-validation-errors)
Also: [CONFIGURATION.md - Troubleshooting](CONFIGURATION.md#troubleshooting)

### Manage database
See: [.descartes/README.md - Database Maintenance]((.descartes/README.md#database-maintenance)
Also: [CONFIGURATION.md - Storage](CONFIGURATION.md#3-storage-configuration)

### Handle log files
See: [.descartes/README.md - Logs]((.descartes/README.md#logs)
Also: [CONFIGURATION.md - Logging](CONFIGURATION.md#7-logging-configuration)

### Backup and restore
See: [.descartes/README.md - Regular Backups]((.descartes/README.md#regular-backups)
Also: [CONFIG_MIGRATION.md - Backup and Restore](CONFIG_MIGRATION.md#backup-and-restore)

## API Reference

### ConfigManager in Code
Where to find: [core/src/config.rs](core/src/config.rs)
Also: [CONFIGURATION.md - Configuration API](CONFIGURATION.md#configuration-api)

Methods:
- `ConfigManager::load()` - Load configuration
- `config()` - Get config reference
- `validate()` - Validate configuration
- `load_from_env()` - Load from environment variables
- `save()` - Save modified configuration

## Examples by Level

### Beginner (Just starting)
1. [CONFIG_QUICKSTART.md - Essential Configuration](CONFIG_QUICKSTART.md#essential-configuration)
2. [.descartes/config.toml.example](.descartes/config.toml.example)
3. Copy and modify example file

### Intermediate (Comfortable with basics)
1. [CONFIGURATION.md](CONFIGURATION.md) - Read relevant sections
2. [CONFIG_VALIDATION.md](CONFIG_VALIDATION.md) - Understand validation
3. [CONFIG_QUICKSTART.md - Example Configurations](CONFIG_QUICKSTART.md#common-configurations)

### Advanced (Full control)
1. [CONFIGURATION.md](CONFIGURATION.md) - Complete reference
2. [CONFIG_MIGRATION.md](CONFIG_MIGRATION.md) - Version management
3. [core/src/config.rs](core/src/config.rs) - Source implementation
4. [CONFIG_VALIDATION.md](CONFIG_VALIDATION.md) - Validation logic

## File Locations

```
descartes/
├── CONFIG_INDEX.md                         <- You are here
├── CONFIG_QUICKSTART.md                    <- Quick setup
├── CONFIGURATION.md                        <- Full reference
├── CONFIG_MIGRATION.md                     <- Version upgrades
├── CONFIG_VALIDATION.md                    <- Troubleshooting
├── core/src/config.rs                      <- Implementation
└── .descartes/
    ├── README.md                           <- Directory guide
    ├── config.toml.example                 <- Example config
    └── config.toml                         <- Your actual config (create this)
```

## Configuration File Hierarchy

When you have both example and actual config:

1. `~/.descartes/config.toml` - Your configuration (highest priority)
2. Environment variables - Override file settings
3. Defaults in code - Used for missing values

## Search Tips

### Find information about...
- **A specific provider:** See [CONFIGURATION.md - Providers](CONFIGURATION.md#1-providers-configuration)
- **A validation error:** See [CONFIG_VALIDATION.md - Common Validation Errors](CONFIG_VALIDATION.md#common-validation-errors)
- **A specific setting:** Search in [CONFIGURATION.md](CONFIGURATION.md)
- **Your error message:** Check [CONFIG_VALIDATION.md](CONFIG_VALIDATION.md)
- **How to do something:** See [CONFIG_QUICKSTART.md - Common Tasks](CONFIG_QUICKSTART.md#examples-by-use-case)
- **File management:** See [.descartes/README.md](.descartes/README.md)

## Quick Reference

### Configuration Validation
See: [CONFIG_VALIDATION.md - Validation Rules](CONFIG_VALIDATION.md#validation-rules)
- 13 validation rules documented
- Common errors explained
- Fixes provided

### Performance Tuning
See: [CONFIGURATION.md - Performance Tuning](CONFIGURATION.md#performance-tuning)
- High throughput settings
- Low resource settings
- High availability settings

### Security Best Practices
See: [CONFIGURATION.md - Security Best Practices](CONFIGURATION.md#security-best-practices)
- File permissions
- Secret management
- Encryption
- Audit logging

### Environment Variables
See: [CONFIGURATION.md - Environment Variables](CONFIGURATION.md#environment-variables)
- Complete list of variables
- How to set them
- Override behavior

## Need Help?

1. **Quick answer:** [CONFIG_QUICKSTART.md](CONFIG_QUICKSTART.md)
2. **Detailed explanation:** [CONFIGURATION.md](CONFIGURATION.md)
3. **Troubleshooting:** [CONFIG_VALIDATION.md](CONFIG_VALIDATION.md)
4. **File management:** [.descartes/README.md](.descartes/README.md)
5. **Upgrading:** [CONFIG_MIGRATION.md](CONFIG_MIGRATION.md)
6. **Source code:** [core/src/config.rs](core/src/config.rs)

## Document Sizes

| Document | Lines | Size | Read Time |
|----------|-------|------|-----------|
| CONFIG_QUICKSTART.md | 350 | 12 KB | 5 min |
| CONFIGURATION.md | 650 | 14 KB | 30 min |
| CONFIG_MIGRATION.md | 450 | 9.8 KB | 15 min |
| CONFIG_VALIDATION.md | 400 | 12 KB | 15 min |
| .descartes/README.md | 350 | 8.9 KB | 15 min |
| config.toml.example | 400 | 11 KB | 10 min |

Total: 2,600 lines, 68 KB documentation

## Version Information

Current configuration version: **1.0.0**

Features:
- Complete schema for all components
- Provider support (Anthropic, OpenAI, Ollama, DeepSeek, Groq)
- All required settings
- Security and encryption
- Notifications and alerts
- Feature flags and logging

## Related Documentation

- **[README.md](../README.md)** - Main project documentation
- **[PROVIDER_DESIGN.md](PROVIDER_DESIGN.md)** - Provider architecture
- **[PROVIDER_EXAMPLES.md](PROVIDER_EXAMPLES.md)** - Provider usage examples

## Navigation Tips

- **Use Ctrl+F** to search within documents
- **Follow links** between documents for related topics
- **Check tables of contents** at top of each document
- **Use this index** to find what you need
- **Start with quickstart** if new to Descartes

---

**Last Updated:** 2025-11-23
**Configuration Version:** 1.0.0
**Status:** Complete and ready to use
