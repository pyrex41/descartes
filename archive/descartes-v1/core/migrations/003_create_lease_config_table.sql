-- Migration: Create Lease Configuration Table
-- Version: 003
-- Description: Stores system-wide and per-resource lease configuration settings

-- Lease configuration table
CREATE TABLE IF NOT EXISTS lease_configs (
    -- Unique identifier for this configuration
    id TEXT PRIMARY KEY NOT NULL,

    -- Configuration key (e.g., 'default_ttl', 'max_retries', 'cleanup_interval')
    config_key TEXT NOT NULL UNIQUE,

    -- Configuration value (can be integer, float, boolean, or string)
    config_value TEXT NOT NULL,

    -- Data type of the value (int, float, bool, string, json)
    value_type TEXT NOT NULL DEFAULT 'string',

    -- Description of what this configuration does
    description TEXT,

    -- Whether this is a global or per-resource config
    scope TEXT NOT NULL DEFAULT 'global',

    -- Resource ID if scope is per-resource
    resource_id TEXT,

    -- Whether changes to this config require restart
    requires_restart INTEGER NOT NULL DEFAULT 0,

    -- Timestamp when this config was created (Unix timestamp in seconds)
    created_at INTEGER NOT NULL,

    -- Timestamp when this config was last modified (Unix timestamp in seconds)
    updated_at INTEGER NOT NULL
);

-- Create index for quick lookup of configurations
CREATE INDEX IF NOT EXISTS idx_lease_configs_key ON lease_configs(config_key);

-- Create index for scope queries
CREATE INDEX IF NOT EXISTS idx_lease_configs_scope ON lease_configs(scope, resource_id);

-- Insert default configurations
INSERT OR IGNORE INTO lease_configs (id, config_key, config_value, value_type, description, scope, requires_restart, created_at, updated_at)
VALUES
    -- Default TTL for new leases (in seconds)
    ('001', 'default_ttl_seconds', '3600', 'int', 'Default time-to-live for newly acquired leases in seconds', 'global', 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Maximum number of times a lease can be renewed
    ('002', 'max_renewals', '-1', 'int', 'Maximum number of renewals per lease (-1 for unlimited)', 'global', 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Timeout for acquiring a lease (in milliseconds)
    ('003', 'acquisition_timeout_ms', '30000', 'int', 'Default timeout for acquiring a lease in milliseconds', 'global', 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Interval for cleaning up expired leases (in seconds)
    ('004', 'cleanup_interval_seconds', '300', 'int', 'Interval between cleanup operations for expired leases in seconds', 'global', 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Enable automatic expiration of leases
    ('005', 'auto_expire_enabled', 'true', 'bool', 'Whether to automatically mark leases as expired when they exceed TTL', 'global', 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Enable blocking/waiting mode for lease acquisition
    ('006', 'blocking_mode_enabled', 'true', 'bool', 'Whether to allow blocking/waiting mode when acquiring leases', 'global', 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- Keep lease history for auditing
    ('007', 'keep_history', 'true', 'bool', 'Whether to maintain historical records of lease operations', 'global', 0, strftime('%s', 'now'), strftime('%s', 'now')),

    -- History retention period (in days)
    ('008', 'history_retention_days', '30', 'int', 'Number of days to retain lease history records', 'global', 0, strftime('%s', 'now'), strftime('%s', 'now'));
