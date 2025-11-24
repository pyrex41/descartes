-- Phase 2: Full-Text Search Optimization and Performance Tuning
-- Created: 2025-11-23
-- Purpose: Enhance RAG layer with FTS capabilities and query optimization

-- ============================================================================
-- PRAGMA SETTINGS FOR OPTIMIZATION
-- ============================================================================

-- Enable foreign key constraints for data integrity
PRAGMA foreign_keys = ON;

-- Set journal mode to WAL for concurrent access
PRAGMA journal_mode = WAL;

-- Increase cache size for better performance
PRAGMA cache_size = -64000;  -- 64MB

-- Synchronous mode balance: NORMAL for better performance
PRAGMA synchronous = NORMAL;

-- Enable query optimizer
PRAGMA optimize;

-- ============================================================================
-- SEMANTIC NODES FTS5 EXTENSIONS
-- ============================================================================

-- Create trigger to keep FTS index synchronized with semantic_nodes table
CREATE TRIGGER IF NOT EXISTS semantic_nodes_fts_insert
AFTER INSERT ON semantic_nodes BEGIN
  INSERT INTO semantic_nodes_fts(id, name, qualified_name, documentation, source_code, summary, file_path, node_type, language, visibility, rank)
  VALUES (
    new.id,
    new.name,
    new.qualified_name,
    new.documentation,
    new.source_code,
    new.summary,
    new.file_path,
    new.node_type,
    new.language,
    new.visibility,
    0  -- rank computed at query time
  );
END;

CREATE TRIGGER IF NOT EXISTS semantic_nodes_fts_delete
AFTER DELETE ON semantic_nodes BEGIN
  DELETE FROM semantic_nodes_fts WHERE id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS semantic_nodes_fts_update
AFTER UPDATE ON semantic_nodes BEGIN
  DELETE FROM semantic_nodes_fts WHERE id = old.id;
  INSERT INTO semantic_nodes_fts(id, name, qualified_name, documentation, source_code, summary, file_path, node_type, language, visibility, rank)
  VALUES (
    new.id,
    new.name,
    new.qualified_name,
    new.documentation,
    new.source_code,
    new.summary,
    new.file_path,
    new.node_type,
    new.language,
    new.visibility,
    0
  );
END;

-- ============================================================================
-- FILE DEPENDENCIES FTS5 EXTENSIONS
-- ============================================================================

CREATE TRIGGER IF NOT EXISTS file_dependencies_fts_insert
AFTER INSERT ON file_dependencies BEGIN
  INSERT INTO file_dependencies_fts(id, source_file_path, target_file_path, import_statement, dependency_type, import_path)
  VALUES (
    new.id,
    new.source_file_path,
    new.target_file_path,
    new.import_statement,
    new.dependency_type,
    new.import_path
  );
END;

CREATE TRIGGER IF NOT EXISTS file_dependencies_fts_delete
AFTER DELETE ON file_dependencies BEGIN
  DELETE FROM file_dependencies_fts WHERE id = old.id;
END;

CREATE TRIGGER IF NOT EXISTS file_dependencies_fts_update
AFTER UPDATE ON file_dependencies BEGIN
  DELETE FROM file_dependencies_fts WHERE id = old.id;
  INSERT INTO file_dependencies_fts(id, source_file_path, target_file_path, import_statement, dependency_type, import_path)
  VALUES (
    new.id,
    new.source_file_path,
    new.target_file_path,
    new.import_statement,
    new.dependency_type,
    new.import_path
  );
END;

-- ============================================================================
-- TIMESTAMP UPDATE TRIGGERS
-- ============================================================================

-- Auto-update updated_at timestamps
CREATE TRIGGER IF NOT EXISTS semantic_nodes_update_timestamp
AFTER UPDATE ON semantic_nodes
FOR EACH ROW
BEGIN
  UPDATE semantic_nodes SET updated_at = strftime('%s', 'now')
  WHERE id = NEW.id AND updated_at = OLD.updated_at;
END;

CREATE TRIGGER IF NOT EXISTS file_dependencies_update_timestamp
AFTER UPDATE ON file_dependencies
FOR EACH ROW
BEGIN
  UPDATE file_dependencies SET updated_at = strftime('%s', 'now')
  WHERE id = NEW.id AND updated_at = OLD.updated_at;
END;

CREATE TRIGGER IF NOT EXISTS semantic_relationships_update_timestamp
AFTER UPDATE ON semantic_relationships
FOR EACH ROW
BEGIN
  UPDATE semantic_relationships SET updated_at = strftime('%s', 'now')
  WHERE id = NEW.id AND updated_at = OLD.updated_at;
END;

CREATE TRIGGER IF NOT EXISTS node_call_graph_update_timestamp
AFTER UPDATE ON node_call_graph
FOR EACH ROW
BEGIN
  UPDATE node_call_graph SET updated_at = strftime('%s', 'now')
  WHERE id = NEW.id AND updated_at = OLD.updated_at;
END;

CREATE TRIGGER IF NOT EXISTS rag_metadata_update_timestamp
AFTER UPDATE ON rag_metadata
FOR EACH ROW
BEGIN
  UPDATE rag_metadata SET updated_at = strftime('%s', 'now')
  WHERE id = NEW.id AND updated_at = OLD.updated_at;
END;

CREATE TRIGGER IF NOT EXISTS file_metadata_update_timestamp
AFTER UPDATE ON file_metadata
FOR EACH ROW
BEGIN
  UPDATE file_metadata SET updated_at = strftime('%s', 'now')
  WHERE file_path = NEW.file_path AND updated_at = OLD.updated_at;
END;

-- ============================================================================
-- CIRCULAR DEPENDENCY DETECTION TRIGGERS
-- ============================================================================

-- Trigger to update circular dependency tracking when a circular relationship is detected
CREATE TRIGGER IF NOT EXISTS file_deps_circular_update
AFTER UPDATE ON file_dependencies
FOR EACH ROW
WHEN NEW.is_circular = 1 AND OLD.is_circular = 0
BEGIN
  UPDATE circular_dependencies
  SET last_detected_at = strftime('%s', 'now'),
      detected_count = detected_count + 1
  WHERE cycle_path = (
    SELECT '[' || NEW.source_file_path || ',' || NEW.target_file_path || ']'
  );
END;

-- ============================================================================
-- FILE METADATA SYNCHRONIZATION TRIGGERS
-- ============================================================================

-- Update file metadata when nodes are added/removed
CREATE TRIGGER IF NOT EXISTS update_file_metadata_on_node_insert
AFTER INSERT ON semantic_nodes
FOR EACH ROW
BEGIN
  INSERT INTO file_metadata(file_path, file_name, language, total_semantic_nodes)
  VALUES (
    NEW.file_path,
    SUBSTR(NEW.file_path, INSTR(NEW.file_path, '/', -1) + 1),
    NEW.language,
    1
  )
  ON CONFLICT(file_path) DO UPDATE SET
    total_semantic_nodes = total_semantic_nodes + 1,
    updated_at = strftime('%s', 'now');
END;

CREATE TRIGGER IF NOT EXISTS update_file_metadata_on_node_delete
AFTER DELETE ON semantic_nodes
FOR EACH ROW
BEGIN
  UPDATE file_metadata
  SET total_semantic_nodes = MAX(0, total_semantic_nodes - 1),
      updated_at = strftime('%s', 'now')
  WHERE file_path = OLD.file_path;
END;

-- Update incoming/outgoing dependency counts
CREATE TRIGGER IF NOT EXISTS update_file_metadata_on_dep_insert
AFTER INSERT ON file_dependencies
FOR EACH ROW
BEGIN
  UPDATE file_metadata
  SET outgoing_dependencies = outgoing_dependencies + 1,
      updated_at = strftime('%s', 'now')
  WHERE file_path = NEW.source_file_path;

  UPDATE file_metadata
  SET incoming_dependencies = incoming_dependencies + 1,
      updated_at = strftime('%s', 'now')
  WHERE file_path = NEW.target_file_path;
END;

CREATE TRIGGER IF NOT EXISTS update_file_metadata_on_dep_delete
AFTER DELETE ON file_dependencies
FOR EACH ROW
BEGIN
  UPDATE file_metadata
  SET outgoing_dependencies = MAX(0, outgoing_dependencies - 1),
      updated_at = strftime('%s', 'now')
  WHERE file_path = OLD.source_file_path;

  UPDATE file_metadata
  SET incoming_dependencies = MAX(0, incoming_dependencies - 1),
      updated_at = strftime('%s', 'now')
  WHERE file_path = OLD.target_file_path;
END;

-- ============================================================================
-- RAG METADATA SYNCHRONIZATION TRIGGERS
-- ============================================================================

-- Update RAG metadata when nodes are updated
CREATE TRIGGER IF NOT EXISTS rag_metadata_mark_reindex_on_node_update
AFTER UPDATE OF source_code, documentation, summary, name
ON semantic_nodes
FOR EACH ROW
WHEN NEW.source_code != OLD.source_code
  OR NEW.documentation != OLD.documentation
  OR NEW.summary != OLD.summary
  OR NEW.name != OLD.name
BEGIN
  UPDATE rag_metadata
  SET needs_reindex = 1,
      updated_at = strftime('%s', 'now')
  WHERE node_id = NEW.id;
END;

-- ============================================================================
-- VIEWS FOR COMMON QUERIES
-- ============================================================================

-- View: All public API surfaces
CREATE VIEW IF NOT EXISTS public_api_nodes AS
SELECT
    id, name, qualified_name, node_type, file_path,
    line_start, line_end, language, signature, documentation
FROM semantic_nodes
WHERE visibility = 'public' AND is_exported = 1
ORDER BY file_path, line_start;

-- View: Call hierarchy for debugging
CREATE VIEW IF NOT EXISTS call_hierarchy AS
SELECT
    cg.caller_node_id,
    cg.callee_node_id,
    caller.name AS caller_name,
    callee.name AS callee_name,
    caller.file_path AS caller_file,
    callee.file_path AS callee_file,
    cg.call_count,
    cg.call_type,
    cg.is_recursive,
    cg.line_number
FROM node_call_graph cg
LEFT JOIN semantic_nodes caller ON cg.caller_node_id = caller.id
LEFT JOIN semantic_nodes callee ON cg.callee_node_id = callee.id
ORDER BY cg.updated_at DESC;

-- View: Dependency metrics
CREATE VIEW IF NOT EXISTS dependency_metrics AS
SELECT
    fm.file_path,
    fm.file_name,
    fm.language,
    fm.total_semantic_nodes AS nodes_count,
    fm.incoming_dependencies,
    fm.outgoing_dependencies,
    (fm.incoming_dependencies + fm.outgoing_dependencies) AS total_dependencies,
    CAST(fm.incoming_dependencies AS REAL) / (fm.outgoing_dependencies + 1) AS coupling_ratio,
    fm.last_parsed_at
FROM file_metadata fm
WHERE fm.file_path IS NOT NULL
ORDER BY total_dependencies DESC;

-- View: Circular dependency chains
CREATE VIEW IF NOT EXISTS circular_dependency_chains AS
SELECT
    id,
    cycle_path,
    cycle_length,
    severity,
    detected_count,
    first_detected_at,
    last_detected_at,
    (last_detected_at - first_detected_at) AS duration_seconds
FROM circular_dependencies
ORDER BY severity DESC, cycle_length DESC;

-- View: Node complexity analysis
CREATE VIEW IF NOT EXISTS node_complexity_analysis AS
SELECT
    sn.id,
    sn.name,
    sn.node_type,
    sn.file_path,
    sn.complexity_score,
    COUNT(DISTINCT snp.id) AS parameter_count,
    COUNT(DISTINCT sr.target_node_id) AS dependency_count,
    COUNT(DISTINCT cg.callee_node_id) AS call_count
FROM semantic_nodes sn
LEFT JOIN semantic_node_parameters snp ON sn.id = snp.node_id
LEFT JOIN semantic_relationships sr ON sn.id = sr.source_node_id
LEFT JOIN node_call_graph cg ON sn.id = cg.caller_node_id
GROUP BY sn.id
ORDER BY sn.complexity_score DESC;

-- View: Import statistics
CREATE VIEW IF NOT EXISTS import_statistics AS
SELECT
    fd.dependency_type,
    COUNT(*) AS import_count,
    COUNT(DISTINCT fd.source_file_path) AS source_files,
    COUNT(DISTINCT fd.target_file_path) AS target_files,
    COUNT(DISTINCT CASE WHEN fd.is_circular = 1 THEN fd.id END) AS circular_imports
FROM file_dependencies fd
GROUP BY fd.dependency_type
ORDER BY import_count DESC;

-- View: Language distribution
CREATE VIEW IF NOT EXISTS language_distribution AS
SELECT
    sn.language,
    COUNT(*) AS node_count,
    COUNT(DISTINCT sn.file_path) AS file_count,
    COUNT(DISTINCT CASE WHEN sn.node_type = 'function' THEN sn.id END) AS function_count,
    COUNT(DISTINCT CASE WHEN sn.node_type = 'class' THEN sn.id END) AS class_count,
    SUM(sn.line_end - sn.line_start) AS total_lines
FROM semantic_nodes sn
GROUP BY sn.language
ORDER BY node_count DESC;

-- View: Parsing session summary
CREATE VIEW IF NOT EXISTS parsing_session_summary AS
SELECT
    aps.id,
    aps.session_name,
    aps.root_path,
    aps.language,
    aps.status,
    aps.total_files_processed,
    aps.total_nodes_extracted,
    aps.total_dependencies_found,
    aps.total_relationships_found,
    aps.duration_ms,
    CASE
        WHEN aps.duration_ms > 0
        THEN CAST(aps.total_nodes_extracted AS REAL) / (aps.duration_ms / 1000.0)
        ELSE 0
    END AS nodes_per_second,
    aps.started_at,
    aps.completed_at
FROM ast_parsing_sessions aps
ORDER BY aps.started_at DESC;

-- ============================================================================
-- MATERIALIZED VIEW SUMMARY TABLE
-- ============================================================================

-- Summary statistics table (updated periodically)
CREATE TABLE IF NOT EXISTS summary_statistics (
    id TEXT PRIMARY KEY NOT NULL,
    stat_key TEXT NOT NULL UNIQUE,
    stat_value INTEGER NOT NULL,
    stat_value_float REAL,
    stat_description TEXT,
    computed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Index for summary statistics
CREATE INDEX IF NOT EXISTS idx_summary_stats_key
ON summary_statistics(stat_key);

-- ============================================================================
-- QUERY STATISTICS TABLE FOR OPTIMIZATION
-- ============================================================================

CREATE TABLE IF NOT EXISTS query_statistics (
    id TEXT PRIMARY KEY NOT NULL,
    query_name TEXT NOT NULL UNIQUE,
    query_pattern TEXT,
    execution_count INTEGER DEFAULT 0,
    total_duration_ms INTEGER DEFAULT 0,
    average_duration_ms REAL DEFAULT 0,
    min_duration_ms INTEGER,
    max_duration_ms INTEGER,
    last_executed_at INTEGER,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- CLEANUP AND OPTIMIZATION PROCEDURES
-- ============================================================================

-- Procedure to vacuum and optimize database (should be called periodically)
-- Run: VACUUM;
-- Run: ANALYZE;

-- Procedure to cleanup stale cache entries (old sessions, expired caches)
CREATE TRIGGER IF NOT EXISTS cleanup_stale_search_cache
AFTER INSERT ON semantic_search_cache
WHEN (SELECT COUNT(*) FROM semantic_search_cache WHERE expires_at < strftime('%s', 'now')) > 1000
BEGIN
  DELETE FROM semantic_search_cache
  WHERE expires_at < strftime('%s', 'now')
  LIMIT 500;
END;

-- ============================================================================
-- CONSTRAINT VALIDATION VIEWS
-- ============================================================================

-- Validate referential integrity
CREATE VIEW IF NOT EXISTS orphaned_semantic_nodes AS
SELECT sn.id, sn.file_path, sn.name
FROM semantic_nodes sn
LEFT JOIN ast_parsing_sessions aps ON sn.file_path LIKE aps.root_path || '%'
WHERE aps.id IS NULL;

-- Detect missing file metadata
CREATE VIEW IF NOT EXISTS files_missing_metadata AS
SELECT DISTINCT sn.file_path
FROM semantic_nodes sn
LEFT JOIN file_metadata fm ON sn.file_path = fm.file_path
WHERE fm.file_path IS NULL;

-- Detect inconsistent dependency references
CREATE VIEW IF NOT EXISTS invalid_dependencies AS
SELECT fd.id, fd.source_file_path, fd.target_file_path
FROM file_dependencies fd
LEFT JOIN file_metadata fm_src ON fd.source_file_path = fm_src.file_path
LEFT JOIN file_metadata fm_tgt ON fd.target_file_path = fm_tgt.file_path
WHERE fm_src.file_path IS NULL OR fm_tgt.file_path IS NULL;
