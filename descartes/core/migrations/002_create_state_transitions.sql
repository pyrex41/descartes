-- Migration: Create state transitions table
-- Version: 2
-- Description: Create table for tracking state transitions and history

CREATE TABLE IF NOT EXISTS state_transitions (
    -- Unique transition identifier
    id TEXT PRIMARY KEY NOT NULL,

    -- Associated agent ID
    agent_id TEXT NOT NULL,

    -- Previous state (serialized)
    state_before TEXT NOT NULL,

    -- New state (serialized)
    state_after TEXT NOT NULL,

    -- Reason for transition
    reason TEXT,

    -- When the transition occurred
    timestamp INTEGER NOT NULL,

    -- Additional metadata about the transition
    metadata TEXT
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS idx_state_transitions_agent_id ON state_transitions(agent_id);
CREATE INDEX IF NOT EXISTS idx_state_transitions_timestamp ON state_transitions(timestamp);
CREATE INDEX IF NOT EXISTS idx_state_transitions_agent_timestamp ON state_transitions(agent_id, timestamp);
