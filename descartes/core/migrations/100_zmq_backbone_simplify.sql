-- Migration: ZMQ Backbone Simplification
-- Version: 100
-- Description: Drops RAG/semantic tables, establishes event sourcing model
-- Does NOT touch: secrets_*, master_keys, leases_*, lease_*

-- Drop semantic analysis tables (if they exist from agent-runner)
DROP TABLE IF EXISTS semantic_nodes;
DROP TABLE IF EXISTS semantic_node_parameters;
DROP TABLE IF EXISTS semantic_node_type_parameters;
DROP TABLE IF EXISTS file_dependencies;
DROP TABLE IF EXISTS semantic_relationships;
DROP TABLE IF EXISTS node_call_graph;
DROP TABLE IF EXISTS ast_parsing_sessions;
DROP TABLE IF EXISTS file_metadata;
DROP TABLE IF EXISTS circular_dependencies;
DROP TABLE IF EXISTS semantic_search_cache;
DROP TABLE IF EXISTS code_change_tracking;
DROP TABLE IF EXISTS rag_metadata;
DROP TABLE IF EXISTS semantic_index_stats;

-- Drop RAG layer tables
DROP TABLE IF EXISTS rag_store_state;
DROP TABLE IF EXISTS vector_metadata;
DROP TABLE IF EXISTS fts_index_metadata;
DROP TABLE IF EXISTS sqlite_index_metadata;
DROP TABLE IF EXISTS hybrid_search_config;
DROP TABLE IF EXISTS search_results_log;
DROP TABLE IF EXISTS document_chunks;
DROP TABLE IF EXISTS rag_context_windows;
DROP TABLE IF EXISTS embedding_cache;
DROP TABLE IF EXISTS relevance_feedback;
DROP TABLE IF EXISTS sync_audit_trail;
DROP TABLE IF EXISTS consistency_checks;
DROP TABLE IF EXISTS rag_performance_stats;

-- Drop old system tables we're replacing
DROP TABLE IF EXISTS summary_statistics;
DROP TABLE IF EXISTS query_statistics;

-- Core event sourcing tables (simplified)
-- These may already exist, so use IF NOT EXISTS

CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'idle',
    zmq_address TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    metadata TEXT DEFAULT '{}'
);

CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    event_type TEXT NOT NULL,
    timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    agent_id TEXT,
    session_id TEXT,
    content TEXT NOT NULL,
    metadata TEXT DEFAULT '{}',
    git_commit TEXT,
    FOREIGN KEY (agent_id) REFERENCES agents(id)
);

CREATE TABLE IF NOT EXISTS snapshots (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_id TEXT,
    git_commit_hash TEXT NOT NULL,
    last_event_id INTEGER NOT NULL,
    description TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    metadata TEXT DEFAULT '{}',
    FOREIGN KEY (agent_id) REFERENCES agents(id),
    FOREIGN KEY (last_event_id) REFERENCES events(id)
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_events_agent_id ON events(agent_id);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
CREATE INDEX IF NOT EXISTS idx_events_git ON events(git_commit);
CREATE INDEX IF NOT EXISTS idx_snapshots_agent ON snapshots(agent_id);
CREATE INDEX IF NOT EXISTS idx_snapshots_git ON snapshots(git_commit_hash);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
