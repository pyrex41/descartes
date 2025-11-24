-- Migration: Create Migration Tracking Table
-- Version: 004
-- Description: Tracks which migrations have been applied to ensure proper schema initialization

-- Migration tracking table
CREATE TABLE IF NOT EXISTS migrations (
    -- Version number of the migration (e.g., 001, 002, etc.)
    version INTEGER PRIMARY KEY NOT NULL,

    -- Name of the migration file
    name TEXT NOT NULL UNIQUE,

    -- Description of what this migration does
    description TEXT,

    -- Timestamp when this migration was applied (Unix timestamp in seconds)
    applied_at INTEGER NOT NULL,

    -- Status of the migration (success, failed, rolled_back)
    status TEXT NOT NULL DEFAULT 'success',

    -- Error message if migration failed
    error_message TEXT
);

-- Create index for quick lookup by name
CREATE INDEX IF NOT EXISTS idx_migrations_name ON migrations(name);

-- Insert records for existing migrations
INSERT OR IGNORE INTO migrations (version, name, description, applied_at, status)
VALUES
    (1, '001_create_leases_table', 'Creates the schema for managing file leases with TTL semantics', strftime('%s', 'now'), 'success'),
    (2, '002_create_lease_history_table', 'Creates audit table for tracking lease lifecycle events', strftime('%s', 'now'), 'success'),
    (3, '003_create_lease_config_table', 'Stores system-wide and per-resource lease configuration settings', strftime('%s', 'now'), 'success'),
    (4, '004_create_migration_tracking_table', 'Tracks which migrations have been applied to ensure proper schema initialization', strftime('%s', 'now'), 'success');
