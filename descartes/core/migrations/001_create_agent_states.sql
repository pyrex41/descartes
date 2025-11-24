-- Migration: Create agent states table
-- Version: 1
-- Description: Create the main agent_states table for storing agent state data

CREATE TABLE IF NOT EXISTS agent_states (
    -- Unique key (can be prefixed)
    key TEXT PRIMARY KEY NOT NULL,

    -- Agent identifier
    agent_id TEXT NOT NULL UNIQUE,

    -- Agent display name
    name TEXT NOT NULL,

    -- Current agent status (running, idle, paused, completed, failed)
    status TEXT NOT NULL DEFAULT 'idle',

    -- Agent metadata as JSON
    metadata TEXT NOT NULL DEFAULT '{}',

    -- Serialized state data (JSON or bincode)
    state_data TEXT NOT NULL,

    -- State version for evolution tracking
    version INTEGER NOT NULL DEFAULT 1,

    -- When the agent state was first created
    created_at INTEGER NOT NULL,

    -- When the agent state was last updated
    updated_at INTEGER NOT NULL,

    -- Soft delete flag
    is_deleted INTEGER NOT NULL DEFAULT 0
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_agent_states_agent_id ON agent_states(agent_id);
CREATE INDEX IF NOT EXISTS idx_agent_states_status ON agent_states(status) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_agent_states_updated_at ON agent_states(updated_at) WHERE is_deleted = 0;
CREATE INDEX IF NOT EXISTS idx_agent_states_created_at ON agent_states(created_at);
