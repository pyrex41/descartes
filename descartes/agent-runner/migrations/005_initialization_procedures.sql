-- Phase 2: Initialization Procedures and Backward Compatibility
-- Created: 2025-11-23
-- Purpose: Setup procedures, initialization scripts, and migration utilities

-- ============================================================================
-- INITIALIZATION STORED PROCEDURES (sqlite stored procedures)
-- NOTE: SQLite doesn't support stored procedures in the traditional sense.
-- Use CREATE VIEW + trigger pattern or separate application code.
-- ============================================================================

-- View for initialization status
CREATE VIEW IF NOT EXISTS initialization_status AS
SELECT
    'semantic_nodes' as table_name,
    (SELECT COUNT(*) FROM semantic_nodes) as record_count,
    (SELECT COUNT(DISTINCT file_path) FROM semantic_nodes) as unique_files,
    (SELECT MIN(created_at) FROM semantic_nodes) as first_record,
    (SELECT MAX(updated_at) FROM semantic_nodes) as last_updated
UNION ALL
SELECT
    'file_dependencies',
    COUNT(*),
    COUNT(DISTINCT source_file_path),
    MIN(created_at),
    MAX(updated_at)
FROM file_dependencies
UNION ALL
SELECT
    'semantic_relationships',
    COUNT(*),
    0,
    MIN(created_at),
    MAX(updated_at)
FROM semantic_relationships
UNION ALL
SELECT
    'node_call_graph',
    COUNT(*),
    COUNT(DISTINCT file_path),
    MIN(created_at),
    MAX(updated_at)
FROM node_call_graph;

-- ============================================================================
-- INITIALIZATION HELPER VIEWS AND FUNCTIONS
-- ============================================================================

-- View for schema version tracking
CREATE TABLE IF NOT EXISTS schema_versions (
    version INTEGER PRIMARY KEY NOT NULL,
    migration_name TEXT NOT NULL UNIQUE,
    description TEXT,
    applied_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    rolled_back_at INTEGER,
    status TEXT CHECK(status IN ('applied', 'rolled_back')) DEFAULT 'applied'
);

-- Index for version tracking
CREATE INDEX IF NOT EXISTS idx_schema_versions_applied
ON schema_versions(status, applied_at DESC);

-- ============================================================================
-- BACKWARD COMPATIBILITY: PHASE 1 TABLES
-- ============================================================================

-- Ensure Phase 1 tables exist (assuming they were created in Phase 1)
-- These are referenced by foreign keys in Phase 2 tables

-- Placeholder for Phase 1 events table structure
-- CREATE TABLE IF NOT EXISTS events (
--     id TEXT PRIMARY KEY NOT NULL,
--     event_type TEXT NOT NULL,
--     timestamp INTEGER NOT NULL,
--     session_id TEXT NOT NULL,
--     actor_type TEXT NOT NULL,
--     actor_id TEXT NOT NULL,
--     content TEXT NOT NULL,
--     metadata TEXT,
--     git_commit TEXT,
--     created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
-- );

-- Placeholder for Phase 1 sessions table
-- CREATE TABLE IF NOT EXISTS sessions (
--     id TEXT PRIMARY KEY NOT NULL,
--     user_id TEXT NOT NULL,
--     started_at INTEGER NOT NULL,
--     ended_at INTEGER,
--     status TEXT NOT NULL,
--     metadata TEXT,
--     created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
-- );

-- Placeholder for Phase 1 tasks table
-- CREATE TABLE IF NOT EXISTS tasks (
--     id TEXT PRIMARY KEY NOT NULL,
--     title TEXT NOT NULL,
--     description TEXT,
--     status TEXT NOT NULL,
--     assigned_to TEXT,
--     created_at INTEGER NOT NULL,
--     updated_at INTEGER NOT NULL,
--     metadata TEXT
-- );

-- ============================================================================
-- MIGRATION VALIDATION AND INTEGRITY CHECKS
-- ============================================================================

-- View to validate schema integrity
CREATE VIEW IF NOT EXISTS schema_integrity_check AS
SELECT
    'semantic_nodes' as entity,
    COUNT(*) as total_records,
    COUNT(DISTINCT file_path) as unique_files,
    COUNT(CASE WHEN parent_id IS NULL THEN 1 END) as root_nodes,
    COUNT(CASE WHEN is_exported = 1 THEN 1 END) as exported_nodes,
    CASE
        WHEN COUNT(*) = 0 THEN 'OK'
        WHEN COUNT(CASE WHEN file_path IS NULL THEN 1 END) > 0 THEN 'ERROR: Missing file paths'
        WHEN COUNT(CASE WHEN qualified_name IS NULL THEN 1 END) > 0 THEN 'ERROR: Missing qualified names'
        ELSE 'OK'
    END as status
FROM semantic_nodes
UNION ALL
SELECT
    'file_dependencies',
    COUNT(*),
    COUNT(DISTINCT source_file_path),
    0,
    COUNT(CASE WHEN is_external = 1 THEN 1 END),
    CASE
        WHEN COUNT(*) = 0 THEN 'OK'
        WHEN COUNT(CASE WHEN source_file_path = target_file_path THEN 1 END) > 0 THEN 'WARNING: Self-dependencies'
        ELSE 'OK'
    END
FROM file_dependencies
UNION ALL
SELECT
    'semantic_relationships',
    COUNT(*),
    COUNT(DISTINCT source_node_id),
    0,
    COUNT(CASE WHEN is_direct = 1 THEN 1 END),
    CASE
        WHEN COUNT(*) = 0 THEN 'OK'
        WHEN COUNT(CASE WHEN source_node_id = target_node_id THEN 1 END) > 0 THEN 'WARNING: Self-relationships'
        ELSE 'OK'
    END
FROM semantic_relationships;

-- ============================================================================
-- DATA MIGRATION AND IMPORT HELPERS
-- ============================================================================

-- Table for tracking import/migration operations
CREATE TABLE IF NOT EXISTS migration_operations (
    id TEXT PRIMARY KEY NOT NULL,

    -- Operation details
    operation_type TEXT NOT NULL CHECK(operation_type IN ('import', 'sync', 'migrate', 'bulk_insert')),
    source_type TEXT NOT NULL,  -- e.g., 'json', 'csv', 'database'

    -- Statistics
    total_records_attempted INTEGER,
    total_records_succeeded INTEGER,
    total_records_failed INTEGER,

    -- Errors
    error_details TEXT,  -- JSON array of errors

    -- Status and timing
    started_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    completed_at INTEGER,
    duration_ms INTEGER,
    status TEXT CHECK(status IN ('in_progress', 'completed', 'failed', 'partial'))
           DEFAULT 'in_progress',

    -- Verification
    checksum TEXT,  -- For data integrity verification

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- EXPORT HELPERS FOR BACKUP AND ANALYSIS
-- ============================================================================

-- View for exporting semantic structure
CREATE VIEW IF NOT EXISTS export_semantic_structure AS
SELECT
    sn.id,
    sn.node_type,
    sn.name,
    sn.qualified_name,
    sn.file_path,
    sn.language,
    sn.line_start,
    sn.line_end,
    sn.visibility,
    sn.signature,
    sn.documentation,
    sn.parent_id,
    -- Count children
    (SELECT COUNT(*) FROM semantic_nodes WHERE parent_id = sn.id) as child_count
FROM semantic_nodes
ORDER BY sn.file_path, sn.line_start;

-- View for exporting dependencies
CREATE VIEW IF NOT EXISTS export_dependencies_structure AS
SELECT
    fd.id,
    fd.source_file_path,
    fd.target_file_path,
    fd.dependency_type,
    fd.import_statement,
    fd.is_circular,
    fd.is_external,
    fd.line_number,
    fd.created_at
FROM file_dependencies
ORDER BY fd.source_file_path, fd.line_number;

-- ============================================================================
-- CLEANUP AND HOUSEKEEPING VIEWS
-- ============================================================================

-- View to identify stale records
CREATE VIEW IF NOT EXISTS stale_records AS
SELECT
    'semantic_nodes' as table_name,
    id as record_id,
    updated_at,
    (strftime('%s', 'now') - updated_at) / 86400.0 as days_since_update
FROM semantic_nodes
WHERE updated_at < (strftime('%s', 'now') - 2592000)  -- Older than 30 days
UNION ALL
SELECT
    'ast_parsing_sessions',
    id,
    completed_at,
    (strftime('%s', 'now') - completed_at) / 86400.0
FROM ast_parsing_sessions
WHERE completed_at < (strftime('%s', 'now') - 7776000);  -- Older than 90 days

-- View for unused semantic nodes
CREATE VIEW IF NOT EXISTS unused_semantic_nodes AS
SELECT
    sn.id,
    sn.name,
    sn.file_path,
    sn.node_type,
    sn.created_at,
    COUNT(DISTINCT sr.id) as relationship_count,
    COUNT(DISTINCT cg.id) as call_count
FROM semantic_nodes sn
LEFT JOIN semantic_relationships sr ON sn.id = sr.source_node_id OR sn.id = sr.target_node_id
LEFT JOIN node_call_graph cg ON sn.id = cg.caller_node_id OR sn.id = cg.callee_node_id
GROUP BY sn.id
HAVING relationship_count = 0 AND call_count = 0
ORDER BY sn.created_at DESC;

-- ============================================================================
-- CONFIGURATION AND DEFAULTS
-- ============================================================================

-- Table for storing configuration parameters
CREATE TABLE IF NOT EXISTS configuration (
    key TEXT PRIMARY KEY NOT NULL,
    value TEXT NOT NULL,
    value_type TEXT CHECK(value_type IN ('string', 'integer', 'float', 'boolean', 'json')),
    description TEXT,
    is_user_overridable BOOLEAN DEFAULT 1,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Insert default configuration
INSERT OR IGNORE INTO configuration(key, value, value_type, description) VALUES
    ('max_parse_depth', '100', 'integer', 'Maximum AST traversal depth'),
    ('chunk_size', '1000', 'integer', 'Size of document chunks for embeddings'),
    ('embedding_model', 'sentence-transformers/all-MiniLM-L6-v2', 'string', 'Default embedding model'),
    ('fts_ranking_algorithm', 'bm25', 'string', 'FTS ranking algorithm'),
    ('vector_similarity_threshold', '0.5', 'float', 'Minimum vector similarity for results'),
    ('enable_circular_detection', 'true', 'boolean', 'Enable circular dependency detection'),
    ('enable_async_indexing', 'true', 'boolean', 'Enable asynchronous indexing'),
    ('cache_ttl_seconds', '3600', 'integer', 'Cache time-to-live in seconds'),
    ('max_recursive_depth', '20', 'integer', 'Maximum recursion depth for call graphs'),
    ('enable_complexity_analysis', 'true', 'boolean', 'Enable code complexity scoring');

-- Index for configuration
CREATE INDEX IF NOT EXISTS idx_configuration_key
ON configuration(key);

-- ============================================================================
-- ROLLBACK SUPPORT TABLES
-- ============================================================================

-- Table to track rollback points
CREATE TABLE IF NOT EXISTS rollback_points (
    id TEXT PRIMARY KEY NOT NULL,
    migration_version INTEGER NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    description TEXT,
    backup_path TEXT
);

-- ============================================================================
-- DIAGNOSTIC AND HEALTH CHECK VIEWS
-- ============================================================================

-- View for system health diagnostics
CREATE VIEW IF NOT EXISTS system_health_diagnostics AS
SELECT
    'Database Size' as metric,
    CAST((SELECT page_count * page_size FROM pragma_page_count(), pragma_page_size) / 1024 / 1024 as TEXT) || ' MB' as value
UNION ALL
SELECT
    'Total Semantic Nodes',
    CAST(COUNT(*) as TEXT)
FROM semantic_nodes
UNION ALL
SELECT
    'Total Dependencies',
    CAST(COUNT(*) as TEXT)
FROM file_dependencies
UNION ALL
SELECT
    'Circular Dependencies',
    CAST(COUNT(*) as TEXT)
FROM circular_dependencies
UNION ALL
SELECT
    'Parsing Sessions',
    CAST(COUNT(*) as TEXT)
FROM ast_parsing_sessions
WHERE status = 'completed'
UNION ALL
SELECT
    'Languages Indexed',
    CAST(COUNT(DISTINCT language) as TEXT)
FROM semantic_nodes;

-- View for query performance diagnostics
CREATE VIEW IF NOT EXISTS query_performance_diagnostics AS
SELECT
    'Nodes with Embeddings' as metric,
    CAST(COUNT(*) as TEXT) as value
FROM vector_metadata
WHERE is_indexed = 1
UNION ALL
SELECT
    'Nodes with FTS Index',
    CAST(COUNT(*) as TEXT)
FROM fts_index_metadata
WHERE is_indexed = 1
UNION ALL
SELECT
    'Cache Hit Rate',
    CAST(ROUND(CAST(SUM(CASE WHEN hit_count > 0 THEN 1 ELSE 0 END) as REAL) / COUNT(*) * 100, 2) as TEXT) || '%'
FROM semantic_search_cache
UNION ALL
SELECT
    'Avg Query Time (ms)',
    CAST(ROUND(AVG(total_duration_ms), 2) as TEXT)
FROM query_statistics;

-- ============================================================================
-- DOCUMENTATION AND REFERENCES
-- ============================================================================

-- Table for schema documentation
CREATE TABLE IF NOT EXISTS schema_documentation (
    id TEXT PRIMARY KEY NOT NULL,
    table_name TEXT NOT NULL UNIQUE,
    table_description TEXT NOT NULL,
    purpose TEXT,
    related_tables TEXT,  -- JSON array of related table names
    key_columns TEXT,  -- JSON array of key column names
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Insert schema documentation
INSERT OR IGNORE INTO schema_documentation(id, table_name, table_description, purpose) VALUES
    ('1', 'semantic_nodes', 'Parsed AST nodes from tree-sitter', 'Store semantic code elements extracted from source code'),
    ('2', 'file_dependencies', 'File-to-file dependencies', 'Track import and dependency relationships between files'),
    ('3', 'semantic_relationships', 'Semantic code relationships', 'Record relationships between semantic nodes (calls, uses, etc.)'),
    ('4', 'node_call_graph', 'Optimized call graph', 'Fast lookup for function call relationships'),
    ('5', 'vector_metadata', 'Vector embeddings metadata', 'Integration with LanceDB for vector similarity search'),
    ('6', 'fts_index_metadata', 'Full-text search metadata', 'Integration with Tantivy for text search'),
    ('7', 'rag_context_windows', 'RAG context for inference', 'Store retrieved context for RAG operations'),
    ('8', 'circular_dependencies', 'Circular dependency tracking', 'Detect and monitor circular dependencies in code');

-- ============================================================================
-- FINAL VALIDATION
-- ============================================================================

-- Verify all required tables exist
CREATE VIEW IF NOT EXISTS required_tables_check AS
SELECT
    'semantic_nodes' as table_name,
    EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='semantic_nodes') as exists
UNION ALL
SELECT 'file_dependencies', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='file_dependencies')
UNION ALL
SELECT 'semantic_relationships', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='semantic_relationships')
UNION ALL
SELECT 'node_call_graph', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='node_call_graph')
UNION ALL
SELECT 'vector_metadata', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='vector_metadata')
UNION ALL
SELECT 'fts_index_metadata', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='fts_index_metadata')
UNION ALL
SELECT 'rag_metadata', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='rag_metadata')
UNION ALL
SELECT 'ast_parsing_sessions', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='ast_parsing_sessions')
UNION ALL
SELECT 'file_metadata', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='file_metadata')
UNION ALL
SELECT 'circular_dependencies', EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='circular_dependencies');

-- ============================================================================
-- INITIALIZATION COMPLETION MARKER
-- ============================================================================

-- Mark initialization as complete
INSERT OR IGNORE INTO schema_versions(version, migration_name, description, status)
VALUES
    (1, '001_initial_schema', 'Initial AST and dependency schema', 'applied'),
    (2, '002_create_indexes', 'Performance optimization indexes', 'applied'),
    (3, '003_fts_and_optimization', 'Full-text search and triggers', 'applied'),
    (4, '004_rag_layer_integration', 'Tri-Store RAG integration', 'applied'),
    (5, '005_initialization_procedures', 'Initialization and helpers', 'applied');
