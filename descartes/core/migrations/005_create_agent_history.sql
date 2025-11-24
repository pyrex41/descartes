-- Migration: Create Agent History Tables
-- Version: 5
-- Description: Create tables for agent history tracking including event logs (brain)
--              and git commit references (body) for comprehensive agent state history

-- ============================================================================
-- AGENT HISTORY EVENTS TABLE (Brain State)
-- ============================================================================
-- Stores individual events in an agent's history including thoughts, actions,
-- tool usage, state changes, and other significant moments

CREATE TABLE IF NOT EXISTS agent_history_events (
    -- Unique identifier for this event
    event_id TEXT PRIMARY KEY NOT NULL,

    -- Agent that generated this event
    agent_id TEXT NOT NULL,

    -- When the event occurred (Unix timestamp in seconds)
    timestamp INTEGER NOT NULL,

    -- Type of event: thought, action, tool_use, state_change, communication, decision, error, system
    event_type TEXT NOT NULL,

    -- Event-specific data (JSON format for flexibility)
    event_data TEXT NOT NULL,

    -- Optional git commit hash linking to body state
    git_commit_hash TEXT,

    -- Optional session ID for grouping related events
    session_id TEXT,

    -- Optional parent event ID for causality tracking
    parent_event_id TEXT,

    -- Tags for categorization (JSON array)
    tags TEXT NOT NULL DEFAULT '[]',

    -- Additional metadata (JSON format)
    metadata TEXT,

    -- When this record was created in the database
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- Foreign key to parent event (if exists)
    FOREIGN KEY (parent_event_id) REFERENCES agent_history_events(event_id) ON DELETE SET NULL
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_history_agent_id
    ON agent_history_events(agent_id);

CREATE INDEX IF NOT EXISTS idx_history_timestamp
    ON agent_history_events(timestamp);

CREATE INDEX IF NOT EXISTS idx_history_event_type
    ON agent_history_events(event_type);

CREATE INDEX IF NOT EXISTS idx_history_session_id
    ON agent_history_events(session_id)
    WHERE session_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_history_git_commit
    ON agent_history_events(git_commit_hash)
    WHERE git_commit_hash IS NOT NULL;

-- Composite indexes for common query patterns
CREATE INDEX IF NOT EXISTS idx_history_agent_timestamp
    ON agent_history_events(agent_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_history_agent_type
    ON agent_history_events(agent_id, event_type);

CREATE INDEX IF NOT EXISTS idx_history_agent_session
    ON agent_history_events(agent_id, session_id)
    WHERE session_id IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_history_parent
    ON agent_history_events(parent_event_id)
    WHERE parent_event_id IS NOT NULL;

-- ============================================================================
-- HISTORY SNAPSHOTS TABLE
-- ============================================================================
-- Stores point-in-time snapshots of agent state, combining brain and body state

CREATE TABLE IF NOT EXISTS history_snapshots (
    -- Unique identifier for this snapshot
    snapshot_id TEXT PRIMARY KEY NOT NULL,

    -- Agent this snapshot belongs to
    agent_id TEXT NOT NULL,

    -- When the snapshot was created
    timestamp INTEGER NOT NULL,

    -- Git commit hash at time of snapshot (body state)
    git_commit TEXT,

    -- Human-readable description
    description TEXT,

    -- Snapshot metadata (JSON format)
    metadata TEXT,

    -- Agent state data at time of snapshot (JSON format)
    agent_state TEXT,

    -- When this record was created in the database
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for snapshot queries
CREATE INDEX IF NOT EXISTS idx_snapshots_agent_id
    ON history_snapshots(agent_id);

CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp
    ON history_snapshots(timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_snapshots_agent_timestamp
    ON history_snapshots(agent_id, timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_snapshots_git_commit
    ON history_snapshots(git_commit)
    WHERE git_commit IS NOT NULL;

-- ============================================================================
-- SNAPSHOT EVENTS JUNCTION TABLE
-- ============================================================================
-- Links events to snapshots (many-to-many relationship)

CREATE TABLE IF NOT EXISTS snapshot_events (
    -- References to snapshot and event
    snapshot_id TEXT NOT NULL,
    event_id TEXT NOT NULL,

    -- Composite primary key
    PRIMARY KEY (snapshot_id, event_id),

    -- Foreign keys with cascade delete
    FOREIGN KEY (snapshot_id) REFERENCES history_snapshots(snapshot_id) ON DELETE CASCADE,
    FOREIGN KEY (event_id) REFERENCES agent_history_events(event_id) ON DELETE CASCADE
);

-- Indexes for junction table
CREATE INDEX IF NOT EXISTS idx_snapshot_events_snapshot
    ON snapshot_events(snapshot_id);

CREATE INDEX IF NOT EXISTS idx_snapshot_events_event
    ON snapshot_events(event_id);

-- ============================================================================
-- VIEWS FOR CONVENIENT QUERYING
-- ============================================================================

-- View: Recent agent activity
CREATE VIEW IF NOT EXISTS v_recent_agent_activity AS
SELECT
    agent_id,
    event_type,
    COUNT(*) as event_count,
    MAX(timestamp) as last_event_time,
    MIN(timestamp) as first_event_time
FROM agent_history_events
WHERE timestamp > strftime('%s', 'now') - 86400  -- Last 24 hours
GROUP BY agent_id, event_type
ORDER BY last_event_time DESC;

-- View: Agent event summary
CREATE VIEW IF NOT EXISTS v_agent_event_summary AS
SELECT
    agent_id,
    COUNT(*) as total_events,
    COUNT(DISTINCT session_id) as unique_sessions,
    COUNT(DISTINCT git_commit_hash) as unique_commits,
    MIN(timestamp) as earliest_event,
    MAX(timestamp) as latest_event,
    COUNT(DISTINCT event_type) as event_types_used
FROM agent_history_events
GROUP BY agent_id;

-- View: Event chains (parent-child relationships)
CREATE VIEW IF NOT EXISTS v_event_chains AS
SELECT
    e1.event_id as child_event_id,
    e1.event_type as child_event_type,
    e1.timestamp as child_timestamp,
    e2.event_id as parent_event_id,
    e2.event_type as parent_event_type,
    e2.timestamp as parent_timestamp,
    e1.agent_id
FROM agent_history_events e1
LEFT JOIN agent_history_events e2 ON e1.parent_event_id = e2.event_id
WHERE e1.parent_event_id IS NOT NULL
ORDER BY e1.timestamp DESC;

-- ============================================================================
-- COMMENTS FOR DOCUMENTATION
-- ============================================================================

-- The agent history system provides comprehensive tracking of agent behavior:
--
-- 1. BRAIN STATE (agent_history_events):
--    - Records all cognitive and action events
--    - Supports causality tracking via parent_event_id
--    - Flexible event_data field for event-specific information
--    - Tagging system for categorization and filtering
--
-- 2. BODY STATE (git_commit_hash):
--    - Links events to code/artifact state
--    - Enables correlation between thoughts and changes
--    - Supports time-travel debugging
--
-- 3. SNAPSHOTS (history_snapshots + snapshot_events):
--    - Point-in-time captures of complete agent state
--    - Combines brain (events) and body (git commits)
--    - Supports recovery and restoration
--    - Enables performance analysis
--
-- QUERY PATTERNS:
--
-- Get all events for an agent:
--   SELECT * FROM agent_history_events WHERE agent_id = ? ORDER BY timestamp DESC;
--
-- Get events by type:
--   SELECT * FROM agent_history_events WHERE agent_id = ? AND event_type = ?;
--
-- Get events in time range:
--   SELECT * FROM agent_history_events
--   WHERE agent_id = ? AND timestamp BETWEEN ? AND ?;
--
-- Get event chain (follow parent references):
--   WITH RECURSIVE chain AS (
--     SELECT * FROM agent_history_events WHERE event_id = ?
--     UNION ALL
--     SELECT e.* FROM agent_history_events e
--     INNER JOIN chain c ON e.event_id = c.parent_event_id
--   )
--   SELECT * FROM chain;
--
-- Get snapshot with events:
--   SELECT s.*, e.* FROM history_snapshots s
--   LEFT JOIN snapshot_events se ON s.snapshot_id = se.snapshot_id
--   LEFT JOIN agent_history_events e ON se.event_id = e.event_id
--   WHERE s.snapshot_id = ?;
