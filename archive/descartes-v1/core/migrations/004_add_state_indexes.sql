-- Migration: Add performance indexes
-- Version: 4
-- Description: Add additional indexes for optimized query performance

-- Additional indexes on events table for better search performance
CREATE INDEX IF NOT EXISTS idx_events_session_timestamp ON events(session_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_events_actor_id ON events(actor_id);
CREATE INDEX IF NOT EXISTS idx_events_type_timestamp ON events(event_type, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_events_content_search ON events(content) WHERE content IS NOT NULL;

-- Additional indexes on tasks table
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_assigned_to ON tasks(assigned_to);
CREATE INDEX IF NOT EXISTS idx_tasks_status_updated ON tasks(status, updated_at DESC);

-- Additional indexes on sessions table
CREATE INDEX IF NOT EXISTS idx_sessions_agent_id ON sessions(agent_id);
CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status);

-- Full-text search support (virtual tables, if FTS5 is available)
-- Note: FTS5 requires compilation flag, so this is optional
-- CREATE VIRTUAL TABLE IF NOT EXISTS events_fts USING fts5(
--     content,
--     event_type,
--     session_id,
--     content=events,
--     content_rowid=id
-- );
