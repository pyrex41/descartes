-- Migration: Create state snapshots table
-- Version: 3
-- Description: Create table for storing state snapshots for recovery and rollback

CREATE TABLE IF NOT EXISTS state_snapshots (
    -- Unique snapshot identifier
    id TEXT PRIMARY KEY NOT NULL,

    -- Associated agent ID
    agent_id TEXT NOT NULL,

    -- Snapshot of state data
    state_data TEXT NOT NULL,

    -- Optional description for the snapshot
    description TEXT,

    -- When the snapshot was created
    created_at INTEGER NOT NULL,

    -- Optional expiration time
    expires_at INTEGER
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_state_snapshots_agent_id ON state_snapshots(agent_id);
CREATE INDEX IF NOT EXISTS idx_state_snapshots_created_at ON state_snapshots(created_at);
CREATE INDEX IF NOT EXISTS idx_state_snapshots_agent_created ON state_snapshots(agent_id, created_at DESC);
