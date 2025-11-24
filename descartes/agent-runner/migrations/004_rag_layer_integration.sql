-- Phase 2: Tri-Store RAG Layer Integration
-- Created: 2025-11-23
-- Purpose: Enable seamless integration with LanceDB (vectors), Tantivy (FTS), and SQLite (relational)

-- ============================================================================
-- RAG STORE METADATA AND SYNCHRONIZATION
-- ============================================================================

-- Table to track RAG store state and consistency
CREATE TABLE IF NOT EXISTS rag_store_state (
    id TEXT PRIMARY KEY NOT NULL,
    store_type TEXT NOT NULL CHECK(store_type IN ('sqlite', 'lancedb', 'tantivy')),

    -- Synchronization state
    is_synchronized BOOLEAN DEFAULT 0,
    last_sync_at INTEGER,

    -- Index statistics
    total_indexed_documents INTEGER DEFAULT 0,
    total_indexed_vectors INTEGER DEFAULT 0,
    total_indexed_tokens INTEGER DEFAULT 0,

    -- Performance metrics
    avg_query_time_ms REAL DEFAULT 0,
    max_query_time_ms INTEGER DEFAULT 0,
    total_queries INTEGER DEFAULT 0,

    -- Status
    status TEXT CHECK(status IN ('healthy', 'degraded', 'failed', 'rebuilding')) DEFAULT 'healthy',
    error_message TEXT,

    -- Configuration
    configuration TEXT,  -- JSON with store-specific config

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- LANCEDB VECTOR STORE INTEGRATION
-- ============================================================================

-- Vector metadata and indexing
CREATE TABLE IF NOT EXISTS vector_metadata (
    id TEXT PRIMARY KEY NOT NULL,

    -- Node reference
    node_id TEXT NOT NULL UNIQUE REFERENCES semantic_nodes(id) ON DELETE CASCADE,
    file_path TEXT NOT NULL,

    -- Vector information
    vector_id TEXT UNIQUE,
    embedding_model TEXT,  -- e.g., "sentence-transformers/all-MiniLM-L6-v2"
    vector_dimension INTEGER,
    vector_hash TEXT,  -- Hash for change detection

    -- Vector content descriptor
    content_type TEXT CHECK(content_type IN ('code', 'documentation', 'signature', 'mixed')),
    content_chunks INTEGER,  -- Number of chunks if chunked

    -- Ranking and relevance
    popularity_score REAL DEFAULT 0.0,
    recency_boost REAL DEFAULT 0.0,
    quality_score REAL DEFAULT 0.0,

    -- Indexing state
    is_indexed BOOLEAN DEFAULT 0,
    indexed_at INTEGER,
    last_queried_at INTEGER,
    query_count INTEGER DEFAULT 0,

    -- Performance metrics
    search_latency_ms REAL DEFAULT 0,
    similarity_threshold REAL DEFAULT 0.5,

    -- Metadata for multi-modal embeddings
    has_code_embedding BOOLEAN DEFAULT 0,
    has_doc_embedding BOOLEAN DEFAULT 0,
    has_combined_embedding BOOLEAN DEFAULT 0,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    metadata TEXT  -- JSON for additional vector metadata
);

-- Indexes for vector searches
CREATE INDEX IF NOT EXISTS idx_vector_metadata_node
ON vector_metadata(node_id);

CREATE INDEX IF NOT EXISTS idx_vector_metadata_vector_id
ON vector_metadata(vector_id);

CREATE INDEX IF NOT EXISTS idx_vector_metadata_indexed
ON vector_metadata(is_indexed, indexed_at DESC);

CREATE INDEX IF NOT EXISTS idx_vector_metadata_popularity
ON vector_metadata(popularity_score DESC);

-- ============================================================================
-- TANTIVY FULL-TEXT SEARCH INTEGRATION
-- ============================================================================

-- Full-text search index metadata
CREATE TABLE IF NOT EXISTS fts_index_metadata (
    id TEXT PRIMARY KEY NOT NULL,

    -- Node reference
    node_id TEXT NOT NULL UNIQUE REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- FTS document information
    doc_id INTEGER UNIQUE,
    segment_id TEXT,  -- Tantivy segment identifier

    -- Field presence
    has_name_field BOOLEAN DEFAULT 1,
    has_qualified_name_field BOOLEAN DEFAULT 1,
    has_documentation_field BOOLEAN DEFAULT 1,
    has_code_field BOOLEAN DEFAULT 1,
    has_signature_field BOOLEAN DEFAULT 1,
    has_summary_field BOOLEAN DEFAULT 0,

    -- Tokenization info
    total_tokens INTEGER DEFAULT 0,
    unique_terms INTEGER DEFAULT 0,
    average_term_frequency REAL DEFAULT 0,

    -- Ranking scores
    bm25_score REAL DEFAULT 0.0,
    tf_idf_score REAL DEFAULT 0.0,
    custom_relevance_score REAL DEFAULT 0.0,

    -- Index state
    is_indexed BOOLEAN DEFAULT 0,
    indexed_at INTEGER,
    last_updated_at INTEGER,

    -- Performance
    query_count INTEGER DEFAULT 0,
    last_queried_at INTEGER,
    avg_query_rank INTEGER,  -- Average position in search results

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    metadata TEXT  -- JSON for additional FTS metadata
);

-- Indexes for FTS queries
CREATE INDEX IF NOT EXISTS idx_fts_metadata_node
ON fts_index_metadata(node_id);

CREATE INDEX IF NOT EXISTS idx_fts_metadata_doc_id
ON fts_index_metadata(doc_id);

CREATE INDEX IF NOT EXISTS idx_fts_metadata_indexed
ON fts_index_metadata(is_indexed, indexed_at DESC);

CREATE INDEX IF NOT EXISTS idx_fts_metadata_bm25
ON fts_index_metadata(bm25_score DESC);

-- ============================================================================
-- SQLITE RELATIONAL INDEX METADATA
-- ============================================================================

-- SQLite-specific index metadata
CREATE TABLE IF NOT EXISTS sqlite_index_metadata (
    id TEXT PRIMARY KEY NOT NULL,

    -- Node reference
    node_id TEXT NOT NULL UNIQUE REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- SQLite table references
    primary_table TEXT NOT NULL,

    -- Index coverage
    indexed_in_fts_table BOOLEAN DEFAULT 1,
    indexed_in_relationship_table BOOLEAN DEFAULT 1,
    indexed_in_dependency_table BOOLEAN DEFAULT 1,

    -- Query statistics
    query_count INTEGER DEFAULT 0,
    sequential_scans INTEGER DEFAULT 0,
    index_scans INTEGER DEFAULT 0,
    total_rows_returned INTEGER DEFAULT 0,

    -- Performance
    avg_query_time_ms REAL DEFAULT 0,
    last_queried_at INTEGER,

    -- Optimization status
    needs_index_rebuild BOOLEAN DEFAULT 0,
    last_analyzed_at INTEGER,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- HYBRID SEARCH STRATEGY TABLE
-- ============================================================================

-- Configuration for hybrid search combining multiple stores
CREATE TABLE IF NOT EXISTS hybrid_search_config (
    id TEXT PRIMARY KEY NOT NULL,

    -- Search strategy
    strategy_name TEXT NOT NULL UNIQUE,
    description TEXT,

    -- Weight distribution (must sum to 1.0)
    sqlite_weight REAL NOT NULL DEFAULT 0.3,
    vector_weight REAL NOT NULL DEFAULT 0.4,
    fts_weight REAL NOT NULL DEFAULT 0.3,

    -- Performance thresholds
    min_sqlite_score REAL DEFAULT 0.5,
    min_vector_similarity REAL DEFAULT 0.5,
    min_fts_score REAL DEFAULT 0.5,

    -- Query optimization
    max_results_sqlite INTEGER DEFAULT 100,
    max_results_vector INTEGER DEFAULT 50,
    max_results_fts INTEGER DEFAULT 100,

    -- Reranking
    use_reranker BOOLEAN DEFAULT 1,
    reranker_model TEXT,

    -- Caching
    use_result_cache BOOLEAN DEFAULT 1,
    cache_ttl_seconds INTEGER DEFAULT 3600,

    -- Features
    boost_recent BOOLEAN DEFAULT 1,
    boost_popular BOOLEAN DEFAULT 1,
    boost_high_quality BOOLEAN DEFAULT 1,

    is_active BOOLEAN DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- SEARCH RESULT RANKING TABLE
-- ============================================================================

-- Store and analyze search results for ranking optimization
CREATE TABLE IF NOT EXISTS search_results_log (
    id TEXT PRIMARY KEY NOT NULL,

    -- Search context
    query_text TEXT NOT NULL,
    search_timestamp INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),

    -- Result details
    result_node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,
    result_rank INTEGER NOT NULL CHECK(result_rank > 0),
    result_score REAL NOT NULL,

    -- Source store
    source_store TEXT CHECK(source_store IN ('sqlite', 'vector', 'fts', 'hybrid')),

    -- User feedback (optional)
    clicked BOOLEAN DEFAULT 0,
    dwell_time_seconds INTEGER,
    marked_relevant BOOLEAN,
    marked_irrelevant BOOLEAN,

    -- Analysis
    is_optimal_result BOOLEAN DEFAULT NULL,  -- NULL = unknown, 1 = yes, 0 = no

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for search analysis
CREATE INDEX IF NOT EXISTS idx_search_results_query
ON search_results_log(query_text);

CREATE INDEX IF NOT EXISTS idx_search_results_node
ON search_results_log(result_node_id);

CREATE INDEX IF NOT EXISTS idx_search_results_feedback
ON search_results_log(marked_relevant, marked_irrelevant);

-- ============================================================================
-- CHUNKING STRATEGY FOR LARGE DOCUMENTS
-- ============================================================================

-- Store chunked document information
CREATE TABLE IF NOT EXISTS document_chunks (
    id TEXT PRIMARY KEY NOT NULL,

    -- Reference
    node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Chunk information
    chunk_index INTEGER NOT NULL CHECK(chunk_index >= 0),
    chunk_text TEXT NOT NULL,
    chunk_start_line INTEGER NOT NULL,
    chunk_end_line INTEGER NOT NULL,

    -- Vector embeddings
    vector_id TEXT UNIQUE,  -- Reference to LanceDB vector
    chunk_summary TEXT,

    -- FTS index
    fts_doc_id INTEGER,  -- Reference to Tantivy index

    -- Metadata
    is_indexed BOOLEAN DEFAULT 0,
    indexed_at INTEGER,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    updated_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for chunking
CREATE INDEX IF NOT EXISTS idx_chunks_node
ON document_chunks(node_id);

CREATE INDEX IF NOT EXISTS idx_chunks_indexed
ON document_chunks(is_indexed, indexed_at DESC);

-- ============================================================================
-- RAG CONTEXT WINDOW MANAGEMENT
-- ============================================================================

-- Store context windows for RAG operations
CREATE TABLE IF NOT EXISTS rag_context_windows (
    id TEXT PRIMARY KEY NOT NULL,

    -- Context
    session_id TEXT NOT NULL,
    query_node_id TEXT REFERENCES semantic_nodes(id) ON DELETE SET NULL,

    -- Retrieved documents
    retrieved_node_ids TEXT NOT NULL,  -- JSON array
    context_size_bytes INTEGER NOT NULL,
    context_token_count INTEGER,

    -- Reranking
    reranked_order TEXT,  -- JSON array of reranked node IDs
    final_scores TEXT,  -- JSON array of final scores

    -- Performance
    retrieval_latency_ms INTEGER,
    reranking_latency_ms INTEGER,
    total_latency_ms INTEGER,

    -- Quality metrics
    context_coherence_score REAL,
    context_relevance_score REAL,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
    expires_at INTEGER  -- TTL for context windows
);

-- Indexes for context window queries
CREATE INDEX IF NOT EXISTS idx_context_windows_session
ON rag_context_windows(session_id);

CREATE INDEX IF NOT EXISTS idx_context_windows_expires
ON rag_context_windows(expires_at);

-- ============================================================================
-- EMBEDDING CACHE FOR PERFORMANCE
-- ============================================================================

-- Cache computed embeddings to avoid recomputation
CREATE TABLE IF NOT EXISTS embedding_cache (
    id TEXT PRIMARY KEY NOT NULL,

    -- Content reference
    node_id TEXT NOT NULL UNIQUE REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Content hash (for cache invalidation)
    content_hash TEXT NOT NULL UNIQUE,

    -- Embedding information
    embedding_type TEXT CHECK(embedding_type IN ('code', 'documentation', 'combined')),
    embedding_model TEXT NOT NULL,
    embedding_dimension INTEGER NOT NULL,

    -- Cache status
    is_valid BOOLEAN DEFAULT 1,
    computed_at INTEGER NOT NULL,
    cache_expires_at INTEGER,

    -- Metadata
    computation_duration_ms INTEGER,
    cache_hit_count INTEGER DEFAULT 0,
    last_used_at INTEGER,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for cache management
CREATE INDEX IF NOT EXISTS idx_embedding_cache_node
ON embedding_cache(node_id);

CREATE INDEX IF NOT EXISTS idx_embedding_cache_valid
ON embedding_cache(is_valid, cache_expires_at);

-- ============================================================================
-- RELEVANCE FEEDBACK AND LEARNING
-- ============================================================================

-- Track user feedback to improve ranking
CREATE TABLE IF NOT EXISTS relevance_feedback (
    id TEXT PRIMARY KEY NOT NULL,

    -- Feedback context
    query_text TEXT NOT NULL,
    search_result_node_id TEXT NOT NULL REFERENCES semantic_nodes(id) ON DELETE CASCADE,

    -- Feedback
    feedback_type TEXT NOT NULL CHECK(feedback_type IN ('relevant', 'irrelevant', 'partially_relevant')),
    feedback_source TEXT CHECK(feedback_source IN ('user', 'system', 'automatic')),

    -- Confidence
    confidence_score REAL CHECK(confidence_score >= 0.0 AND confidence_score <= 1.0),

    -- Impact (computed)
    impact_on_ranking REAL,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- Indexes for feedback analysis
CREATE INDEX IF NOT EXISTS idx_relevance_feedback_query
ON relevance_feedback(query_text);

CREATE INDEX IF NOT EXISTS idx_relevance_feedback_node
ON relevance_feedback(search_result_node_id);

-- ============================================================================
-- SYNC AUDIT TRAIL
-- ============================================================================

-- Track synchronization between stores
CREATE TABLE IF NOT EXISTS sync_audit_trail (
    id TEXT PRIMARY KEY NOT NULL,

    -- Sync operation
    sync_type TEXT NOT NULL CHECK(sync_type IN ('push', 'pull', 'reconcile')),
    source_store TEXT NOT NULL CHECK(source_store IN ('sqlite', 'lancedb', 'tantivy')),
    target_store TEXT NOT NULL CHECK(target_store IN ('sqlite', 'lancedb', 'tantivy')),

    -- Operation details
    total_documents INTEGER,
    successful_syncs INTEGER,
    failed_syncs INTEGER,

    -- Performance
    start_time INTEGER NOT NULL,
    end_time INTEGER,
    duration_ms INTEGER,

    -- Status
    status TEXT CHECK(status IN ('in_progress', 'completed', 'failed', 'partial'))
           DEFAULT 'in_progress',
    error_message TEXT,

    -- Details
    details TEXT  -- JSON with detailed sync information
);

-- Indexes for audit trail
CREATE INDEX IF NOT EXISTS idx_sync_audit_stores
ON sync_audit_trail(source_store, target_store);

CREATE INDEX IF NOT EXISTS idx_sync_audit_status
ON sync_audit_trail(status, end_time DESC);

-- ============================================================================
-- CONSISTENCY CHECKING
-- ============================================================================

-- Monitor consistency across stores
CREATE TABLE IF NOT EXISTS consistency_checks (
    id TEXT PRIMARY KEY NOT NULL,

    -- Check information
    check_type TEXT NOT NULL,
    store_type TEXT CHECK(store_type IN ('sqlite', 'lancedb', 'tantivy', 'cross_store')),

    -- Results
    total_checked INTEGER,
    total_inconsistencies INTEGER,

    -- Details
    inconsistency_details TEXT,  -- JSON with details

    -- Status and remediation
    status TEXT CHECK(status IN ('ok', 'warning', 'error', 'resolved')),
    remediation_action TEXT,
    remediation_completed BOOLEAN DEFAULT 0,

    -- Timing
    checked_at INTEGER NOT NULL,
    remediation_at INTEGER,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- PERFORMANCE STATISTICS FOR MONITORING
-- ============================================================================

-- Track RAG layer performance
CREATE TABLE IF NOT EXISTS rag_performance_stats (
    id TEXT PRIMARY KEY NOT NULL,

    -- Time period
    measurement_period TEXT NOT NULL,  -- e.g., 'hourly', 'daily'
    period_start INTEGER NOT NULL,
    period_end INTEGER NOT NULL,

    -- Query statistics
    total_queries INTEGER DEFAULT 0,
    avg_query_time_ms REAL DEFAULT 0,
    p95_query_time_ms REAL,
    p99_query_time_ms REAL,

    -- Store-specific stats
    sqlite_avg_time_ms REAL,
    vector_avg_time_ms REAL,
    fts_avg_time_ms REAL,

    -- Result quality
    avg_result_rank_position REAL,
    click_through_rate REAL,
    user_satisfaction_score REAL,

    -- Indexing health
    documents_pending_indexing INTEGER DEFAULT 0,
    indexing_lag_minutes INTEGER,

    -- Cache effectiveness
    cache_hit_rate REAL,
    cache_miss_rate REAL,

    created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
);

-- ============================================================================
-- TRIGGER FOR RAG CONSISTENCY
-- ============================================================================

-- Mark vectors/FTS as needing reindex when node is updated
CREATE TRIGGER IF NOT EXISTS mark_rag_stale_on_node_update
AFTER UPDATE OF source_code, documentation, summary
ON semantic_nodes
FOR EACH ROW
WHEN NEW.source_code != OLD.source_code
  OR NEW.documentation != OLD.documentation
  OR NEW.summary != OLD.summary
BEGIN
  UPDATE vector_metadata
  SET vector_hash = NULL, is_indexed = 0, updated_at = strftime('%s', 'now')
  WHERE node_id = NEW.id;

  UPDATE fts_index_metadata
  SET is_indexed = 0, updated_at = strftime('%s', 'now')
  WHERE node_id = NEW.id;

  UPDATE embedding_cache
  SET is_valid = 0, cache_expires_at = strftime('%s', 'now')
  WHERE node_id = NEW.id;

  UPDATE rag_metadata
  SET needs_reindex = 1, updated_at = strftime('%s', 'now')
  WHERE node_id = NEW.id;
END;
