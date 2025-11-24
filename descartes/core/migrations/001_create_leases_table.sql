-- Migration: Create Leases Table for TTL-Based File Locking
-- Version: 001
-- Description: Creates the schema for managing file leases with TTL semantics

-- Main leases table
CREATE TABLE IF NOT EXISTS leases (
    -- Unique identifier for the lease
    id TEXT PRIMARY KEY NOT NULL,

    -- Path to the file being locked
    file_path TEXT NOT NULL,

    -- ID of the agent holding this lease
    agent_id TEXT NOT NULL,

    -- Timestamp when the lease was created (Unix timestamp in seconds)
    created_at INTEGER NOT NULL,

    -- Timestamp when the lease will expire (Unix timestamp in seconds)
    expires_at INTEGER NOT NULL,

    -- Time-to-live for the lease in seconds
    ttl_seconds INTEGER NOT NULL,

    -- Current status of the lease (active, expired, released, pending, failed)
    status TEXT NOT NULL DEFAULT 'pending',

    -- Number of times this lease has been renewed
    renewal_count INTEGER NOT NULL DEFAULT 0,

    -- Maximum number of renewals allowed (-1 for unlimited)
    max_renewals INTEGER NOT NULL DEFAULT -1,

    -- Timestamp when the lease was last modified (Unix timestamp in seconds)
    updated_at INTEGER NOT NULL
);

-- Index for quick lookup by file path
CREATE INDEX IF NOT EXISTS idx_leases_file_path ON leases(file_path);

-- Index for quick lookup by agent ID
CREATE INDEX IF NOT EXISTS idx_leases_agent_id ON leases(agent_id);

-- Index for finding active leases by expiration time (for cleanup)
CREATE INDEX IF NOT EXISTS idx_leases_expires_at ON leases(expires_at) WHERE status = 'active';

-- Composite index for checking if a file is locked
CREATE INDEX IF NOT EXISTS idx_leases_file_status ON leases(file_path, status) WHERE status IN ('active', 'pending');

-- Composite index for finding active leases for an agent
CREATE INDEX IF NOT EXISTS idx_leases_agent_status ON leases(agent_id, status) WHERE status = 'active';
