-- Migration: Create Lease History Table for Audit Trail
-- Version: 002
-- Description: Creates audit table for tracking lease lifecycle events (acquisition, renewal, release)

-- Lease history/audit trail table
CREATE TABLE IF NOT EXISTS lease_history (
    -- Unique identifier for this history record
    id TEXT PRIMARY KEY NOT NULL,

    -- ID of the lease this record pertains to
    lease_id TEXT NOT NULL,

    -- ID of the agent involved in this event
    agent_id TEXT NOT NULL,

    -- Path to the file
    file_path TEXT NOT NULL,

    -- Type of event (acquired, renewed, released, expired, failed, cleanup)
    event_type TEXT NOT NULL,

    -- Status before the event
    status_before TEXT,

    -- Status after the event
    status_after TEXT,

    -- Number of renewals at the time of the event
    renewal_count INTEGER,

    -- Additional details about the event (JSON)
    details TEXT,

    -- Timestamp when this event occurred (Unix timestamp in seconds)
    event_at INTEGER NOT NULL,

    -- Reason for event (if applicable, e.g., error message)
    reason TEXT,

    -- Foreign key to leases table (soft reference, lease may be deleted)
    FOREIGN KEY (lease_id) REFERENCES leases(id)
);

-- Index for quick lookup by lease ID
CREATE INDEX IF NOT EXISTS idx_lease_history_lease_id ON lease_history(lease_id);

-- Index for quick lookup by agent ID
CREATE INDEX IF NOT EXISTS idx_lease_history_agent_id ON lease_history(agent_id);

-- Index for finding events by type
CREATE INDEX IF NOT EXISTS idx_lease_history_event_type ON lease_history(event_type);

-- Index for finding events by file path
CREATE INDEX IF NOT EXISTS idx_lease_history_file_path ON lease_history(file_path);

-- Index for time-based queries
CREATE INDEX IF NOT EXISTS idx_lease_history_event_at ON lease_history(event_at);

-- Composite index for finding all events for a lease
CREATE INDEX IF NOT EXISTS idx_lease_history_lease_event ON lease_history(lease_id, event_at DESC);

-- Composite index for finding all events for an agent by type
CREATE INDEX IF NOT EXISTS idx_lease_history_agent_type ON lease_history(agent_id, event_type, event_at DESC);
