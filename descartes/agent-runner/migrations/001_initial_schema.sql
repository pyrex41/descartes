-- Phase 2: Initial Schema Extension for AST and File Dependencies
-- Created: 2025-11-23
-- Purpose: Foundation tables for semantic code analysis and Tri-Store RAG Layer
-- Dependencies: Core tables (events, sessions, tasks) assumed to exist from Phase 1

-- ============================================================================
-- 1. SEMANTIC NODES TABLE - Stores parsed AST nodes from tree-sitter
-- ============================================================================
CREATE TABLE IF NOT EXISTS semantic_nodes (
    -- Primary Key
    id TEXT PRIMARY KEY NOT NULL,

    -- Node Identification
    node_type TEXT NOT NULL CHECK(
        node_type IN (
            'module', 'function', 'class', 'struct', 'enum', 'interface',
            'import', 'export', 'type_alias', 'constant', 'variable',
            'comment', 'type', 'macro', 'method', 'property', 'other'
        )
    ),
    name TEXT NOT NULL,
    qualified_name TEXT NOT NULL,

    -- Source Code
    source_code TEXT NOT NULL,
    documentation TEXT,

    -- Location Information
    file_path TEXT NOT NULL,
    line_start INTEGER NOT NULL CHECK(line_start >= 0),
    line_end INTEGER NOT NULL CHECK(line_end >= line_start),
    column_start INTEGER,
    column_end INTEGER,

    -- Language
    language TEXT NOT NULL CHECK(language IN ('rust', 'python', 'javascript', 'typescript')),

    -- Hierarchy
    parent_id TEXT REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Type and Signature Information
    signature TEXT,
    return_type TEXT,
    visibility TEXT CHECK(visibility IN ('public', 'private', 'protected', 'internal', 'package')),

    -- Metadata
    is_exported BOOLEAN DEFAULT 0,
    is_async BOOLEAN DEFAULT 0,
    is_generic BOOLEAN DEFAULT 0,
    complexity_score REAL,

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- Full-text search and RAG
    embedding_hash TEXT,  -- Hash of vector embedding for deduplication
    summary TEXT,         -- Brief summary for RAG ranking

    -- Additional Metadata (JSON)
    metadata TEXT         -- JSON object for extensibility
);

-- ============================================================================
-- 2. SEMANTIC NODE PARAMETERS TABLE - Stores function/method parameters
-- ============================================================================
CREATE TABLE IF NOT EXISTS semantic_node_parameters (
    id TEXT PRIMARY KEY NOT NULL,
    node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Parameter Information
    param_name TEXT NOT NULL,
    param_type TEXT NOT NULL,
    position INTEGER NOT NULL CHECK(position >= 0),

    -- Modifiers
    has_default BOOLEAN DEFAULT 0,
    is_variadic BOOLEAN DEFAULT 0,
    is_optional BOOLEAN DEFAULT 0,

    -- Default Value (if applicable)
    default_value TEXT,

    -- Documentation
    documentation TEXT,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- 3. SEMANTIC NODE GENERIC PARAMETERS TABLE - Stores generic/template info
-- ============================================================================
CREATE TABLE IF NOT EXISTS semantic_node_type_parameters (
    id TEXT PRIMARY KEY NOT NULL,
    node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Generic Parameter Information
    param_name TEXT NOT NULL,
    position INTEGER NOT NULL CHECK(position >= 0),

    -- Constraints
    constraint_bound TEXT,
    is_contravariant BOOLEAN DEFAULT 0,
    is_covariant BOOLEAN DEFAULT 0,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- 4. FILE DEPENDENCIES TABLE - Tracks file-to-file dependencies
-- ============================================================================
CREATE TABLE IF NOT EXISTS file_dependencies (
    id TEXT PRIMARY KEY NOT NULL,

    -- Relationship
    source_file_path TEXT NOT NULL,
    target_file_path TEXT NOT NULL,

    -- Dependency Type
    dependency_type TEXT NOT NULL CHECK(
        dependency_type IN (
            'import', 'require', 'include', 'reference',
            'inherit', 'trait', 'interface', 'export',
            'reexport', 'dynamic', 'conditional'
        )
    ),

    -- Import Information
    import_statement TEXT,
    is_relative_import BOOLEAN DEFAULT 0,
    import_path TEXT,

    -- Scope
    scope TEXT CHECK(scope IN ('module', 'function', 'class', 'conditional')),

    -- Strength/Severity
    is_circular BOOLEAN DEFAULT 0,
    is_weak BOOLEAN DEFAULT 0,  -- Optional dependency
    is_external BOOLEAN DEFAULT 0,  -- External package
    is_internal BOOLEAN DEFAULT 0,  -- Internal to project

    -- Line Information
    line_number INTEGER,
    column_number INTEGER,

    -- Metadata
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- JSON metadata for extensibility
    metadata TEXT,

    UNIQUE(source_file_path, target_file_path, dependency_type, import_path)
);

-- ============================================================================
-- 5. SEMANTIC RELATIONSHIPS TABLE - Records semantic code relationships
-- ============================================================================
CREATE TABLE IF NOT EXISTS semantic_relationships (
    id TEXT PRIMARY KEY NOT NULL,

    -- Entities
    source_node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,
    target_node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Relationship Type
    relationship_type TEXT NOT NULL CHECK(
        relationship_type IN (
            'calls', 'called_by', 'inherits', 'implements',
            'uses', 'used_by', 'defines', 'type_of',
            'returns', 'parameter_of', 'raises', 'catches',
            'overrides', 'overridden_by', 'contains', 'contained_by',
            'depends_on', 'dependency_of', 'references', 'referenced_by'
        )
    ),

    -- Context
    context_file_path TEXT NOT NULL,
    context_line_start INTEGER,
    context_line_end INTEGER,

    -- Strength/Confidence
    confidence_score REAL DEFAULT 1.0 CHECK(confidence_score >= 0.0 AND confidence_score <= 1.0),
    is_direct BOOLEAN DEFAULT 1,  -- Direct vs inferred
    is_dynamic BOOLEAN DEFAULT 0,  -- Runtime polymorphism

    -- Metadata
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    metadata TEXT,  -- JSON for extensibility

    UNIQUE(source_node_id, target_node_id, relationship_type)
);

-- ============================================================================
-- 6. NODE CALL GRAPH TABLE - Optimized for call chain queries
-- ============================================================================
CREATE TABLE IF NOT EXISTS node_call_graph (
    id TEXT PRIMARY KEY NOT NULL,

    -- Call Information
    caller_node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,
    callee_node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Call Details
    call_count INTEGER DEFAULT 1 CHECK(call_count > 0),
    call_sites TEXT,  -- CSV of line numbers
    call_type TEXT CHECK(call_type IN ('direct', 'indirect', 'virtual', 'async', 'callback')),

    -- Context
    file_path TEXT NOT NULL,
    line_number INTEGER NOT NULL,

    -- Analysis
    execution_order INTEGER,  -- For ordered calls
    is_recursive BOOLEAN DEFAULT 0,
    is_mutual_recursive BOOLEAN DEFAULT 0,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    UNIQUE(caller_node_id, callee_node_id, line_number)
);

-- ============================================================================
-- 7. AST PARSING SESSIONS TABLE - Track parsing operations
-- ============================================================================
CREATE TABLE IF NOT EXISTS ast_parsing_sessions (
    id TEXT PRIMARY KEY NOT NULL,

    -- Session Info
    session_name TEXT NOT NULL,
    started_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    completed_at INTEGER,

    -- Scope
    root_path TEXT NOT NULL,
    language TEXT CHECK(language IN ('rust', 'python', 'javascript', 'typescript', 'mixed')),

    -- Statistics
    total_files_processed INTEGER DEFAULT 0,
    total_nodes_extracted INTEGER DEFAULT 0,
    total_dependencies_found INTEGER DEFAULT 0,
    total_relationships_found INTEGER DEFAULT 0,

    -- Status
    status TEXT NOT NULL CHECK(status IN ('in_progress', 'completed', 'failed', 'cancelled'))
                   DEFAULT 'in_progress',
    error_message TEXT,

    -- Performance
    duration_ms INTEGER,

    metadata TEXT  -- JSON metadata
);

-- ============================================================================
-- 8. FILE METADATA TABLE - Cache for file information
-- ============================================================================
CREATE TABLE IF NOT EXISTS file_metadata (
    file_path TEXT PRIMARY KEY NOT NULL,

    -- File Info
    file_name TEXT NOT NULL,
    file_size_bytes INTEGER,
    language TEXT CHECK(language IN ('rust', 'python', 'javascript', 'typescript', 'unknown')),

    -- Parse Info
    last_parsed_at INTEGER,
    parse_hash TEXT,  -- Hash of file content at parse time

    -- Statistics
    total_lines INTEGER,
    code_lines INTEGER,
    comment_lines INTEGER,
    blank_lines INTEGER,

    -- Node Count
    total_semantic_nodes INTEGER DEFAULT 0,

    -- Timestamps
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- Dependencies
    outgoing_dependencies INTEGER DEFAULT 0,
    incoming_dependencies INTEGER DEFAULT 0
);

-- ============================================================================
-- 9. CIRCULAR DEPENDENCY DETECTION TABLE
-- ============================================================================
CREATE TABLE IF NOT EXISTS circular_dependencies (
    id TEXT PRIMARY KEY NOT NULL,

    -- Cycle Information
    cycle_path TEXT NOT NULL,  -- JSON array of file paths forming cycle
    cycle_length INTEGER NOT NULL CHECK(cycle_length >= 2),

    -- Details
    dependency_type TEXT NOT NULL,
    severity TEXT CHECK(severity IN ('critical', 'high', 'medium', 'low')) DEFAULT 'high',

    -- Metadata
    first_detected_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    last_detected_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    detected_count INTEGER DEFAULT 1,

    UNIQUE(cycle_path)
);

-- ============================================================================
-- 10. SEMANTIC SEARCH CACHE TABLE - FTS optimization for RAG
-- ============================================================================
CREATE TABLE IF NOT EXISTS semantic_search_cache (
    id TEXT PRIMARY KEY NOT NULL,

    -- Query Info
    query_text TEXT NOT NULL,
    query_hash TEXT NOT NULL UNIQUE,

    -- Results
    result_node_ids TEXT NOT NULL,  -- JSON array of node IDs
    result_count INTEGER NOT NULL,

    -- Ranking
    relevance_scores TEXT,  -- JSON array of scores
    search_context TEXT,  -- Context for personalized ranking

    -- Metadata
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    expires_at INTEGER,  -- TTL for cache invalidation

    hit_count INTEGER DEFAULT 0
);

-- ============================================================================
-- 11. CODE CHANGE TRACKING TABLE - For incremental parsing
-- ============================================================================
CREATE TABLE IF NOT EXISTS code_change_tracking (
    id TEXT PRIMARY KEY NOT NULL,

    -- Change Information
    file_path TEXT NOT NULL,
    change_type TEXT NOT NULL CHECK(change_type IN ('created', 'modified', 'deleted')),

    -- Details
    git_hash TEXT,  -- Git commit hash if available
    git_author TEXT,
    change_description TEXT,

    -- Impact Analysis
    affected_node_ids TEXT,  -- JSON array of node IDs affected
    affected_dependencies INTEGER DEFAULT 0,
    affected_relationships INTEGER DEFAULT 0,

    -- Timestamps
    detected_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    processed_at INTEGER,

    -- Status
    processed BOOLEAN DEFAULT 0,

    UNIQUE(file_path, git_hash, change_type)
);

-- ============================================================================
-- 12. RAG LAYER METADATA TABLE - Support Tri-Store integration
-- ============================================================================
CREATE TABLE IF NOT EXISTS rag_metadata (
    id TEXT PRIMARY KEY NOT NULL,

    -- Node Reference
    node_id TEXT NOT NULL UNIQUE REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Vector Store (LanceDB) Metadata
    lancedb_vector_id TEXT,
    vector_dimension INTEGER,
    vector_computed_at INTEGER,

    -- Full-Text Search (Tantivy) Metadata
    tantivy_doc_id INTEGER,
    fts_index_version TEXT,
    fts_computed_at INTEGER,

    -- SQLite Full-Text Search
    fts_rowid INTEGER,

    -- Ranking Metrics
    popularity_score REAL DEFAULT 0.0,
    recency_score REAL DEFAULT 0.0,
    quality_score REAL DEFAULT 0.0,
    combined_rank REAL DEFAULT 0.0,

    -- Cache Status
    needs_reindex BOOLEAN DEFAULT 0,
    last_indexed_at INTEGER,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- 13. SEMANTIC INDEX STATS TABLE - Monitoring and optimization
-- ============================================================================
CREATE TABLE IF NOT EXISTS semantic_index_stats (
    id TEXT PRIMARY KEY NOT NULL,

    -- Stats
    stat_name TEXT NOT NULL UNIQUE,
    stat_value TEXT NOT NULL,

    -- Computation
    computed_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    computation_duration_ms INTEGER,

    -- Trend
    previous_value TEXT,
    change_percentage REAL,

    metadata TEXT
);
