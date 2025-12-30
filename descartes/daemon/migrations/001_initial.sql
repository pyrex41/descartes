-- Initial schema for descartes-daemon
-- Projects table
CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    owner_id TEXT NOT NULL,
    prd_content TEXT,
    scud_tag TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_projects_owner ON projects(owner_id);

-- Agents table (for tracking cloud agents)
CREATE TABLE IF NOT EXISTS agents (
    id TEXT PRIMARY KEY,
    project_id TEXT REFERENCES projects(id),
    fly_machine_id TEXT,
    status TEXT NOT NULL,
    task_id TEXT,
    created_at TEXT NOT NULL,
    completed_at TEXT,
    cost_compute_seconds INTEGER DEFAULT 0,
    cost_tokens INTEGER DEFAULT 0
);

CREATE INDEX IF NOT EXISTS idx_agents_project ON agents(project_id);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);

-- Cost tracking table
CREATE TABLE IF NOT EXISTS cost_entries (
    id TEXT PRIMARY KEY,
    agent_id TEXT REFERENCES agents(id),
    project_id TEXT REFERENCES projects(id),
    compute_seconds INTEGER NOT NULL,
    tokens INTEGER NOT NULL,
    cost_usd REAL NOT NULL,
    created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cost_project ON cost_entries(project_id);

-- Sessions table (for user sessions)
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    created_at TEXT NOT NULL,
    expires_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sessions_user ON sessions(user_id);
CREATE INDEX IF NOT EXISTS idx_sessions_expires ON sessions(expires_at);
