-- Encrypted Secrets Storage Schema
-- SQLite database for secure secret management with AES-256-GCM encryption

-- Master key information table
-- Stores parameters for key derivation and master key metadata
CREATE TABLE IF NOT EXISTS master_keys (
    id TEXT PRIMARY KEY,
    algorithm TEXT NOT NULL,          -- 'pbkdf2' or 'argon2id'
    salt BLOB NOT NULL,               -- Random salt for key derivation
    iterations INTEGER,               -- For PBKDF2: number of iterations
    memory_cost INTEGER,              -- For Argon2: memory in KiB
    parallelism INTEGER,              -- For Argon2: parallelism factor
    key_length INTEGER NOT NULL,      -- Output key length in bytes
    password_hash BLOB NOT NULL,      -- Hash of master password (for verification)
    created_at INTEGER NOT NULL,      -- Unix timestamp
    last_rotated_at INTEGER,          -- Unix timestamp
    rotation_interval_days INTEGER DEFAULT 90,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    version INTEGER NOT NULL DEFAULT 1,
    created_by TEXT,                  -- User who created this key

    CHECK (algorithm IN ('pbkdf2', 'argon2id'))
);

-- Secret metadata table
-- Stores unencrypted metadata about secrets
CREATE TABLE IF NOT EXISTS secrets (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    secret_type TEXT NOT NULL,        -- 'api_key', 'oauth_token', 'database_password', 'private_key', 'custom'
    description TEXT,
    service TEXT,                     -- Application/service name
    current_version INTEGER NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    last_accessed_at INTEGER,
    expires_at INTEGER,               -- Optional expiration
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_by TEXT NOT NULL,
    updated_by TEXT,

    UNIQUE(name),
    CHECK (secret_type IN ('api_key', 'oauth_token', 'database_password', 'private_key', 'custom'))
);

-- Secret tags table
-- Supports organizing secrets by tags
CREATE TABLE IF NOT EXISTS secret_tags (
    secret_id TEXT NOT NULL,
    tag TEXT NOT NULL,
    created_at INTEGER NOT NULL,

    PRIMARY KEY (secret_id, tag),
    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE
);

-- Create index for tag searches
CREATE INDEX IF NOT EXISTS idx_secret_tags_tag ON secret_tags(tag);

-- Encrypted secret values table
-- Stores encrypted secret data with encryption metadata
CREATE TABLE IF NOT EXISTS secret_values (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    ciphertext BLOB NOT NULL,         -- AES-256-GCM encrypted value
    nonce BLOB NOT NULL,              -- Unique nonce/IV for this encryption
    tag BLOB NOT NULL,                -- GCM authentication tag
    encryption_version INTEGER NOT NULL DEFAULT 1,  -- Encryption scheme version
    created_at INTEGER NOT NULL,
    rotated_at INTEGER,               -- When this version was rotated
    is_valid BOOLEAN NOT NULL DEFAULT 1,

    UNIQUE(secret_id, version),
    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE,
    CHECK (encryption_version = 1)
);

-- Create index for version lookups
CREATE INDEX IF NOT EXISTS idx_secret_values_secret_id ON secret_values(secret_id);

-- Access control table
-- Stores granular permissions for secret access
CREATE TABLE IF NOT EXISTS access_control (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    principal_type TEXT NOT NULL,     -- 'user', 'role', 'service'
    permission_level INTEGER NOT NULL, -- 0=None, 1=ViewMetadata, 2=Read, 3=Update, 4=Delete, 5=Admin
    granted_at INTEGER NOT NULL,
    granted_by TEXT NOT NULL,
    expires_at INTEGER,               -- Optional expiration
    reason TEXT,

    UNIQUE(secret_id, principal_id, principal_type),
    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE,
    CHECK (principal_type IN ('user', 'role', 'service')),
    CHECK (permission_level BETWEEN 0 AND 5)
);

-- Create indices for access control queries
CREATE INDEX IF NOT EXISTS idx_access_control_secret_id ON access_control(secret_id);
CREATE INDEX IF NOT EXISTS idx_access_control_principal_id ON access_control(principal_id);

-- Audit log table
-- Comprehensive logging of all secret access and modifications
CREATE TABLE IF NOT EXISTS audit_logs (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL,
    principal_id TEXT NOT NULL,
    action TEXT NOT NULL,             -- 'create', 'read', 'update', 'delete', 'rotate', 'modify_metadata', 'change_permissions', 'export', 'access_denied'
    success BOOLEAN NOT NULL,
    error TEXT,
    timestamp INTEGER NOT NULL,
    source TEXT,                      -- IP address or identifier
    context TEXT,                     -- Additional context (JSON)

    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE,
    CHECK (action IN ('create', 'read', 'update', 'delete', 'rotate', 'modify_metadata', 'change_permissions', 'export', 'access_denied'))
);

-- Create indices for audit log queries
CREATE INDEX IF NOT EXISTS idx_audit_logs_secret_id ON audit_logs(secret_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_principal_id ON audit_logs(principal_id);
CREATE INDEX IF NOT EXISTS idx_audit_logs_timestamp ON audit_logs(timestamp);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs(action);

-- Sessions table
-- Tracks decryption context and user sessions for rate limiting
CREATE TABLE IF NOT EXISTS secret_sessions (
    id TEXT PRIMARY KEY,
    principal_id TEXT NOT NULL,
    session_token TEXT NOT NULL UNIQUE,
    master_key_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    last_activity INTEGER NOT NULL,
    ip_address TEXT,
    user_agent TEXT,

    FOREIGN KEY (master_key_id) REFERENCES master_keys(id)
);

-- Create index for session lookups
CREATE INDEX IF NOT EXISTS idx_secret_sessions_principal_id ON secret_sessions(principal_id);
CREATE INDEX IF NOT EXISTS idx_secret_sessions_token ON secret_sessions(session_token);

-- Secret rotation policy table
-- Define automatic rotation policies for secrets
CREATE TABLE IF NOT EXISTS rotation_policies (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL,
    rotation_interval_days INTEGER NOT NULL,
    rotation_strategy TEXT,            -- 'automatic', 'manual', 'on_access'
    last_rotated_at INTEGER,
    next_rotation_at INTEGER,
    is_active BOOLEAN NOT NULL DEFAULT 1,
    created_at INTEGER NOT NULL,
    created_by TEXT NOT NULL,

    UNIQUE(secret_id),
    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE
);

-- Create index for rotation lookups
CREATE INDEX IF NOT EXISTS idx_rotation_policies_secret_id ON rotation_policies(secret_id);
CREATE INDEX IF NOT EXISTS idx_rotation_policies_next_rotation ON rotation_policies(next_rotation_at);

-- Master key rotation tracking
CREATE TABLE IF NOT EXISTS master_key_rotations (
    id TEXT PRIMARY KEY,
    old_master_key_id TEXT NOT NULL,
    new_master_key_id TEXT NOT NULL,
    initiated_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL,             -- 'in_progress', 'completed', 'failed'
    error TEXT,
    rotated_secret_count INTEGER DEFAULT 0,

    FOREIGN KEY (old_master_key_id) REFERENCES master_keys(id),
    FOREIGN KEY (new_master_key_id) REFERENCES master_keys(id),
    CHECK (status IN ('in_progress', 'completed', 'failed'))
);

-- Create indices for master key rotation
CREATE INDEX IF NOT EXISTS idx_master_key_rotations_status ON master_key_rotations(status);
CREATE INDEX IF NOT EXISTS idx_master_key_rotations_old_key ON master_key_rotations(old_master_key_id);
CREATE INDEX IF NOT EXISTS idx_master_key_rotations_new_key ON master_key_rotations(new_master_key_id);

-- Encryption metadata table
-- Stores encryption parameters for recovery and migration
CREATE TABLE IF NOT EXISTS encryption_metadata (
    id TEXT PRIMARY KEY,
    secret_id TEXT NOT NULL,
    encryption_algorithm TEXT NOT NULL,      -- 'aes-256-gcm'
    key_derivation_algorithm TEXT NOT NULL,  -- 'pbkdf2', 'argon2id'
    master_key_id TEXT NOT NULL,
    encryption_timestamp INTEGER NOT NULL,

    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE,
    FOREIGN KEY (master_key_id) REFERENCES master_keys(id),
    CHECK (encryption_algorithm = 'aes-256-gcm')
);

-- Create index for encryption metadata
CREATE INDEX IF NOT EXISTS idx_encryption_metadata_secret_id ON encryption_metadata(secret_id);

-- Rate limiting/brute force protection
CREATE TABLE IF NOT EXISTS access_attempts (
    id TEXT PRIMARY KEY,
    principal_id TEXT NOT NULL,
    secret_id TEXT,                   -- NULL for general access attempts
    attempt_type TEXT NOT NULL,       -- 'decryption_failed', 'password_wrong', 'unauthorized_access'
    success BOOLEAN NOT NULL,
    timestamp INTEGER NOT NULL,
    ip_address TEXT,

    FOREIGN KEY (secret_id) REFERENCES secrets(id) ON DELETE CASCADE,
    CHECK (attempt_type IN ('decryption_failed', 'password_wrong', 'unauthorized_access'))
);

-- Create indices for access attempt tracking
CREATE INDEX IF NOT EXISTS idx_access_attempts_principal_id ON access_attempts(principal_id);
CREATE INDEX IF NOT EXISTS idx_access_attempts_timestamp ON access_attempts(timestamp);

-- Views for common queries

-- View: All active secrets with their current versions
CREATE VIEW IF NOT EXISTS active_secrets AS
SELECT
    s.id,
    s.name,
    s.secret_type,
    s.description,
    s.service,
    s.current_version,
    s.created_at,
    s.updated_at,
    s.expires_at,
    s.created_by,
    COUNT(DISTINCT st.tag) as tag_count
FROM secrets s
LEFT JOIN secret_tags st ON s.id = st.secret_id
WHERE s.is_active = 1
GROUP BY s.id;

-- View: Expired secrets
CREATE VIEW IF NOT EXISTS expired_secrets AS
SELECT
    s.id,
    s.name,
    s.expires_at,
    CASE
        WHEN s.expires_at < strftime('%s', 'now') THEN 'expired'
        ELSE 'expiring_soon'
    END as status
FROM secrets s
WHERE s.expires_at IS NOT NULL
  AND s.expires_at < strftime('%s', 'now') + (30 * 86400);  -- Within 30 days

-- View: Recent audit activity
CREATE VIEW IF NOT EXISTS recent_audit_activity AS
SELECT
    a.id,
    a.secret_id,
    s.name as secret_name,
    a.principal_id,
    a.action,
    a.success,
    a.timestamp,
    a.error
FROM audit_logs a
LEFT JOIN secrets s ON a.secret_id = s.id
ORDER BY a.timestamp DESC
LIMIT 1000;

-- View: Access control summary
CREATE VIEW IF NOT EXISTS access_control_summary AS
SELECT
    s.id,
    s.name,
    COUNT(DISTINCT ac.principal_id) as permission_count,
    GROUP_CONCAT(DISTINCT ac.principal_id, ', ') as principal_ids,
    MAX(ac.granted_at) as last_permission_change
FROM secrets s
LEFT JOIN access_control ac ON s.id = ac.secret_id
WHERE ac.id IS NOT NULL
GROUP BY s.id;

-- Pragma settings for security
PRAGMA foreign_keys = ON;
PRAGMA journal_mode = WAL;            -- Write-Ahead Logging for durability
PRAGMA synchronous = FULL;            -- Full synchronization for data integrity
PRAGMA temp_store = MEMORY;           -- Use memory for temp tables
PRAGMA query_only = OFF;              -- Allow writes

-- Ensure UTF-8 encoding
PRAGMA encoding = "UTF-8";
