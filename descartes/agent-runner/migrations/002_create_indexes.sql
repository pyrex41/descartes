-- Phase 2: Indexes for Efficient Querying
-- Created: 2025-11-23
-- Purpose: Optimize query performance for semantic analysis and RAG operations

-- ============================================================================
-- SEMANTIC NODES INDEXES
-- ============================================================================

-- Primary lookup by file and node type
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_file_type
ON semantic_nodes(file_path, node_type);

-- Hierarchy traversal
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_parent
ON semantic_nodes(parent_id);

-- Search by qualified name
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_qualified_name
ON semantic_nodes(qualified_name);

-- Location-based queries
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_location
ON semantic_nodes(file_path, line_start, line_end);

-- Language-specific queries
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_language
ON semantic_nodes(language);

-- For export/visibility filtering
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_visibility
ON semantic_nodes(visibility);

-- Complex node type filtering
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_type
ON semantic_nodes(node_type);

-- Name-based search
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_name
ON semantic_nodes(name COLLATE NOCASE);

-- For RAG embeddings
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_embedding
ON semantic_nodes(embedding_hash);

-- Timestamp queries
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_created
ON semantic_nodes(created_at DESC);

CREATE INDEX IF NOT EXISTS idx_semantic_nodes_updated
ON semantic_nodes(updated_at DESC);

-- ============================================================================
-- SEMANTIC NODE PARAMETERS INDEXES
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_semantic_node_params_node
ON semantic_node_parameters(node_id);

CREATE INDEX IF NOT EXISTS idx_semantic_node_params_name
ON semantic_node_parameters(param_name COLLATE NOCASE);

-- ============================================================================
-- SEMANTIC NODE TYPE PARAMETERS INDEXES
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_semantic_node_type_params_node
ON semantic_node_type_parameters(node_id);

-- ============================================================================
-- FILE DEPENDENCIES INDEXES
-- ============================================================================

-- Forward and backward dependency queries
CREATE INDEX IF NOT EXISTS idx_file_deps_source
ON file_dependencies(source_file_path);

CREATE INDEX IF NOT EXISTS idx_file_deps_target
ON file_dependencies(target_file_path);

-- Bidirectional dependency lookups
CREATE INDEX IF NOT EXISTS idx_file_deps_source_target
ON file_dependencies(source_file_path, target_file_path);

-- Circular dependency detection
CREATE INDEX IF NOT EXISTS idx_file_deps_circular
ON file_dependencies(is_circular, source_file_path, target_file_path);

-- Dependency type filtering
CREATE INDEX IF NOT EXISTS idx_file_deps_type
ON file_dependencies(dependency_type);

-- External vs internal filtering
CREATE INDEX IF NOT EXISTS idx_file_deps_internal
ON file_dependencies(is_internal, source_file_path);

-- Import path lookup
CREATE INDEX IF NOT EXISTS idx_file_deps_import_path
ON file_dependencies(import_path);

-- Temporal queries
CREATE INDEX IF NOT EXISTS idx_file_deps_updated
ON file_dependencies(updated_at DESC);

-- ============================================================================
-- SEMANTIC RELATIONSHIPS INDEXES
-- ============================================================================

-- Forward and backward relationship queries
CREATE INDEX IF NOT EXISTS idx_semantic_rels_source
ON semantic_relationships(source_node_id);

CREATE INDEX IF NOT EXISTS idx_semantic_rels_target
ON semantic_relationships(target_node_id);

-- Bidirectional relationship lookup
CREATE INDEX IF NOT EXISTS idx_semantic_rels_source_target
ON semantic_relationships(source_node_id, target_node_id);

-- Relationship type filtering
CREATE INDEX IF NOT EXISTS idx_semantic_rels_type
ON semantic_relationships(relationship_type);

-- Call graph optimization
CREATE INDEX IF NOT EXISTS idx_semantic_rels_calls
ON semantic_relationships(
    relationship_type,
    source_node_id,
    target_node_id
) WHERE relationship_type IN ('calls', 'called_by');

-- File-based relationship queries
CREATE INDEX IF NOT EXISTS idx_semantic_rels_context
ON semantic_relationships(context_file_path);

-- Confidence filtering
CREATE INDEX IF NOT EXISTS idx_semantic_rels_confidence
ON semantic_relationships(confidence_score DESC);

-- Direct vs inferred filtering
CREATE INDEX IF NOT EXISTS idx_semantic_rels_direct
ON semantic_relationships(is_direct);

-- ============================================================================
-- NODE CALL GRAPH INDEXES
-- ============================================================================

-- Forward and backward call queries
CREATE INDEX IF NOT EXISTS idx_call_graph_caller
ON node_call_graph(caller_node_id);

CREATE INDEX IF NOT EXISTS idx_call_graph_callee
ON node_call_graph(callee_node_id);

-- Bidirectional call lookup
CREATE INDEX IF NOT EXISTS idx_call_graph_caller_callee
ON node_call_graph(caller_node_id, callee_node_id);

-- Recursive call detection
CREATE INDEX IF NOT EXISTS idx_call_graph_recursive
ON node_call_graph(is_recursive, caller_node_id);

-- File-based call queries
CREATE INDEX IF NOT EXISTS idx_call_graph_file
ON node_call_graph(file_path);

-- Call type filtering
CREATE INDEX IF NOT EXISTS idx_call_graph_type
ON node_call_graph(call_type);

-- ============================================================================
-- AST PARSING SESSIONS INDEXES
-- ============================================================================

-- Session queries
CREATE INDEX IF NOT EXISTS idx_parsing_sessions_root
ON ast_parsing_sessions(root_path);

CREATE INDEX IF NOT EXISTS idx_parsing_sessions_status
ON ast_parsing_sessions(status);

-- Temporal queries
CREATE INDEX IF NOT EXISTS idx_parsing_sessions_started
ON ast_parsing_sessions(started_at DESC);

-- Language-specific sessions
CREATE INDEX IF NOT EXISTS idx_parsing_sessions_language
ON ast_parsing_sessions(language);

-- ============================================================================
-- FILE METADATA INDEXES
-- ============================================================================

-- Language-based queries
CREATE INDEX IF NOT EXISTS idx_file_metadata_language
ON file_metadata(language);

-- Dependency metrics
CREATE INDEX IF NOT EXISTS idx_file_metadata_incoming
ON file_metadata(incoming_dependencies DESC);

CREATE INDEX IF NOT EXISTS idx_file_metadata_outgoing
ON file_metadata(outgoing_dependencies DESC);

-- Parse status
CREATE INDEX IF NOT EXISTS idx_file_metadata_last_parsed
ON file_metadata(last_parsed_at DESC);

-- Temporal queries
CREATE INDEX IF NOT EXISTS idx_file_metadata_updated
ON file_metadata(updated_at DESC);

-- ============================================================================
-- CIRCULAR DEPENDENCIES INDEXES
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_circular_deps_detected
ON circular_dependencies(last_detected_at DESC);

CREATE INDEX IF NOT EXISTS idx_circular_deps_severity
ON circular_dependencies(severity);

-- ============================================================================
-- SEMANTIC SEARCH CACHE INDEXES
-- ============================================================================

-- Cache lookup
CREATE INDEX IF NOT EXISTS idx_search_cache_hash
ON semantic_search_cache(query_hash);

-- TTL-based cleanup
CREATE INDEX IF NOT EXISTS idx_search_cache_expires
ON semantic_search_cache(expires_at);

-- Hit tracking
CREATE INDEX IF NOT EXISTS idx_search_cache_hits
ON semantic_search_cache(hit_count DESC);

-- ============================================================================
-- CODE CHANGE TRACKING INDEXES
-- ============================================================================

-- File-based change queries
CREATE INDEX IF NOT EXISTS idx_code_changes_file
ON code_change_tracking(file_path);

-- Change type filtering
CREATE INDEX IF NOT EXISTS idx_code_changes_type
ON code_change_tracking(change_type);

-- Processing status
CREATE INDEX IF NOT EXISTS idx_code_changes_processed
ON code_change_tracking(processed);

-- Temporal queries
CREATE INDEX IF NOT EXISTS idx_code_changes_detected
ON code_change_tracking(detected_at DESC);

-- Git hash lookup
CREATE INDEX IF NOT EXISTS idx_code_changes_git_hash
ON code_change_tracking(git_hash);

-- ============================================================================
-- RAG METADATA INDEXES
-- ============================================================================

-- Vector store lookup
CREATE INDEX IF NOT EXISTS idx_rag_lancedb_vector
ON rag_metadata(lancedb_vector_id);

-- Full-text search lookup
CREATE INDEX IF NOT EXISTS idx_rag_tantivy_doc
ON rag_metadata(tantivy_doc_id);

-- Ranking queries
CREATE INDEX IF NOT EXISTS idx_rag_combined_rank
ON rag_metadata(combined_rank DESC);

-- Reindexing queries
CREATE INDEX IF NOT EXISTS idx_rag_needs_reindex
ON rag_metadata(needs_reindex, last_indexed_at DESC);

-- ============================================================================
-- SEMANTIC INDEX STATS INDEXES
-- ============================================================================

CREATE INDEX IF NOT EXISTS idx_index_stats_computed
ON semantic_index_stats(computed_at DESC);

-- ============================================================================
-- COMPOSITE INDEXES FOR COMPLEX QUERIES
-- ============================================================================

-- Common RAG queries: find nodes by language, type, and relevance
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_rag_query
ON semantic_nodes(language, node_type, embedding_hash)
WHERE is_exported = 1 OR visibility = 'public';

-- Call graph analysis
CREATE INDEX IF NOT EXISTS idx_call_graph_analysis
ON node_call_graph(caller_node_id, callee_node_id, call_type, is_recursive);

-- Dependency analysis
CREATE INDEX IF NOT EXISTS idx_file_deps_analysis
ON file_dependencies(source_file_path, target_file_path, is_circular, dependency_type);

-- Change impact analysis
CREATE INDEX IF NOT EXISTS idx_change_impact_analysis
ON code_change_tracking(file_path, change_type, processed)
WHERE processed = 0;

-- ============================================================================
-- FUNCTIONAL INDEXES FOR ADVANCED QUERIES
-- ============================================================================

-- Case-insensitive name search
CREATE INDEX IF NOT EXISTS idx_semantic_nodes_name_lower
ON semantic_nodes(LOWER(name));

-- File path patterns
CREATE INDEX IF NOT EXISTS idx_file_deps_source_dirname
ON file_dependencies(source_file_path)
WHERE source_file_path LIKE '%.rs' OR source_file_path LIKE '%.py';

-- Recent changes (last 7 days)
CREATE INDEX IF NOT EXISTS idx_recent_code_changes
ON code_change_tracking(detected_at DESC)
WHERE detected_at > (strftime('%s', 'now') - 604800);

-- ============================================================================
-- PERFORMANCE OPTIMIZATION: PARTIAL INDEXES
-- ============================================================================

-- Only index active/public nodes for faster RAG queries
CREATE INDEX IF NOT EXISTS idx_public_semantic_nodes
ON semantic_nodes(file_path, node_type, qualified_name)
WHERE visibility = 'public' AND is_exported = 1;

-- Only index direct relationships for faster traversal
CREATE INDEX IF NOT EXISTS idx_direct_relationships
ON semantic_relationships(source_node_id, target_node_id, relationship_type)
WHERE is_direct = 1;

-- Only index valid circular dependencies
CREATE INDEX IF NOT EXISTS idx_active_circular_deps
ON circular_dependencies(cycle_length, severity)
WHERE severity IN ('critical', 'high');

-- Only index unprocessed changes for faster change detection
CREATE INDEX IF NOT EXISTS idx_unprocessed_changes
ON code_change_tracking(file_path, change_type)
WHERE processed = 0;

-- ============================================================================
-- FULL-TEXT SEARCH VIRTUAL TABLE
-- ============================================================================

-- Create FTS5 virtual table for semantic node content search
CREATE VIRTUAL TABLE IF NOT EXISTS semantic_nodes_fts
USING fts5(
    id UNINDEXED,
    name,
    qualified_name,
    documentation,
    source_code,
    summary,
    -- Metadata columns for filtering
    file_path UNINDEXED,
    node_type UNINDEXED,
    language UNINDEXED,
    visibility UNINDEXED,
    -- Ranking column
    rank UNINDEXED
);

-- Create FTS5 for file dependencies
CREATE VIRTUAL TABLE IF NOT EXISTS file_dependencies_fts
USING fts5(
    id UNINDEXED,
    source_file_path UNINDEXED,
    target_file_path UNINDEXED,
    import_statement,
    dependency_type UNINDEXED,
    import_path
);

-- ============================================================================
-- STATISTICS AND ANALYSIS
-- ============================================================================

-- Analyze tables for query planning optimization
ANALYZE;
