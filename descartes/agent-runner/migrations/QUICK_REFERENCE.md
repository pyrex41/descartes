# SQLite Schema Quick Reference

**Phase 2: Descartes - AST & Dependency Tracking**

## Common Operations

### Query Nodes

```sql
-- Find function by name
SELECT * FROM semantic_nodes
WHERE name = 'parse_function'
AND node_type = 'function'
AND language = 'rust';

-- Find all classes in file
SELECT * FROM semantic_nodes
WHERE file_path = 'src/parser.rs'
AND node_type IN ('class', 'struct');

-- Find public API
SELECT id, name, signature, file_path
FROM semantic_nodes
WHERE visibility = 'public'
AND language = 'rust'
ORDER BY name;

-- Count nodes by type
SELECT node_type, COUNT(*) as count
FROM semantic_nodes
GROUP BY node_type
ORDER BY count DESC;
```

### Query Dependencies

```sql
-- Files that import X
SELECT DISTINCT source_file_path
FROM file_dependencies
WHERE target_file_path = 'src/core.rs'
AND dependency_type = 'import';

-- All imports in a file
SELECT target_file_path, import_statement, line_number
FROM file_dependencies
WHERE source_file_path = 'src/main.rs'
ORDER BY line_number;

-- Find circular dependencies
SELECT cycle_path, cycle_length
FROM circular_dependencies
WHERE severity = 'critical';

-- Count dependencies by type
SELECT dependency_type, COUNT(*) as count
FROM file_dependencies
GROUP BY dependency_type;
```

### Query Relationships

```sql
-- All calls from function X
SELECT target_node_id, sn.name, cg.call_count
FROM node_call_graph cg
INNER JOIN semantic_nodes sn ON cg.callee_node_id = sn.id
WHERE cg.caller_node_id = ?
ORDER BY cg.call_count DESC;

-- All functions that call X
SELECT caller_node_id, sn.name
FROM node_call_graph cg
INNER JOIN semantic_nodes sn ON cg.caller_node_id = sn.id
WHERE cg.callee_node_id = ?;

-- Find recursive functions
SELECT DISTINCT caller_node_id, sn.name
FROM node_call_graph cg
INNER JOIN semantic_nodes sn ON cg.caller_node_id = sn.id
WHERE cg.is_recursive = 1;

-- All relationships involving a node
SELECT
    CASE WHEN source_node_id = ? THEN target_node_id ELSE source_node_id END as node_id,
    relationship_type,
    confidence_score
FROM semantic_relationships
WHERE source_node_id = ? OR target_node_id = ?
ORDER BY confidence_score DESC;
```

### Full-Text Search

```sql
-- Search node names
SELECT * FROM semantic_nodes_fts
WHERE name MATCH 'pars*'
LIMIT 20;

-- Search documentation
SELECT * FROM semantic_nodes_fts
WHERE documentation MATCH 'error handling'
LIMIT 10;

-- Complex FTS search
SELECT * FROM semantic_nodes_fts
WHERE semantic_nodes_fts MATCH '(parse OR lexer) AND NOT test'
ORDER BY rank
LIMIT 20;

-- Search with language filter
SELECT sn.* FROM semantic_nodes sn
INNER JOIN semantic_nodes_fts fts ON sn.id = fts.id
WHERE fts MATCH 'concurrent'
AND sn.language = 'rust';
```

### Analyze Code Structure

```sql
-- Complexity ranking
SELECT id, name, complexity_score
FROM semantic_nodes
WHERE complexity_score IS NOT NULL
AND node_type = 'function'
ORDER BY complexity_score DESC
LIMIT 20;

-- Nodes by file
SELECT file_path, COUNT(*) as node_count
FROM semantic_nodes
GROUP BY file_path
ORDER BY node_count DESC
LIMIT 10;

-- Dependency coupling
SELECT
    fm.file_path,
    fm.incoming_dependencies,
    fm.outgoing_dependencies,
    (fm.incoming_dependencies + fm.outgoing_dependencies) as total
FROM file_metadata fm
WHERE fm.file_path IS NOT NULL
ORDER BY total DESC
LIMIT 20;

-- Language distribution
SELECT
    language,
    COUNT(*) as nodes,
    COUNT(DISTINCT file_path) as files
FROM semantic_nodes
GROUP BY language
ORDER BY nodes DESC;
```

## Maintenance

### Health Check

```sql
-- Overall status
SELECT * FROM system_health_diagnostics;

-- Schema integrity
SELECT * FROM schema_integrity_check;

-- Find issues
SELECT * FROM invalid_dependencies;
SELECT * FROM files_missing_metadata;
```

### Performance Monitoring

```sql
-- Check query performance
SELECT * FROM query_performance_diagnostics;

-- Check RAG performance
SELECT * FROM rag_performance_stats
WHERE period_start > (strftime('%s', 'now') - 86400);

-- Index usage
PRAGMA index_list('semantic_nodes');
PRAGMA index_info('idx_semantic_nodes_file_type');
```

### Database Optimization

```sql
-- Analyze query plans
EXPLAIN QUERY PLAN
SELECT * FROM semantic_nodes
WHERE file_path = ? AND node_type = ?;

-- Update statistics (run regularly!)
ANALYZE;

-- Reclaim space
VACUUM;

-- Rebuild indexes if fragmented
REINDEX idx_semantic_nodes_file_type;
```

## Inserting Data

### Insert Node

```sql
INSERT INTO semantic_nodes (
    id, node_type, name, qualified_name,
    source_code, file_path, language,
    line_start, line_end, visibility,
    created_at, updated_at
)
VALUES (
    ?, 'function', 'parse', 'parser::parse',
    ?, 'src/parser.rs', 'rust',
    42, 88, 'public',
    strftime('%s', 'now'),
    strftime('%s', 'now')
);
```

### Insert Parameter

```sql
INSERT INTO semantic_node_parameters (
    id, node_id, param_name, param_type,
    position, has_default, is_variadic
)
VALUES (
    ?, ?, 'input', 'String',
    0, 0, 0
);
```

### Insert Dependency

```sql
INSERT INTO file_dependencies (
    id, source_file_path, target_file_path,
    dependency_type, import_statement, line_number
)
VALUES (
    ?, 'src/main.rs', 'src/parser.rs',
    'import', 'use parser::parse;', 5
);
```

### Insert Relationship

```sql
INSERT INTO semantic_relationships (
    id, source_node_id, target_node_id,
    relationship_type, context_file_path,
    confidence_score, is_direct
)
VALUES (
    ?, ?, ?,
    'calls', 'src/main.rs',
    1.0, 1
);
```

## Bulk Operations

### Import from JSON

```sql
-- Assuming JSON data in table json_nodes(data TEXT)
INSERT INTO semantic_nodes
SELECT
    json_extract(data, '$.id'),
    json_extract(data, '$.node_type'),
    json_extract(data, '$.name'),
    ...
FROM json_nodes;
```

### Export to JSON

```sql
SELECT json_object(
    'nodes', (
        SELECT json_group_array(
            json_object(
                'id', id,
                'name', name,
                'type', node_type,
                'file', file_path
            )
        ) FROM semantic_nodes
    ),
    'dependencies', (
        SELECT json_group_array(
            json_object(
                'source', source_file_path,
                'target', target_file_path,
                'type', dependency_type
            )
        ) FROM file_dependencies
    )
) as export;
```

### Bulk Delete

```sql
-- Delete old sessions (keep 90 days)
DELETE FROM ast_parsing_sessions
WHERE completed_at < (strftime('%s', 'now') - 7776000);

-- Delete stale cache
DELETE FROM semantic_search_cache
WHERE expires_at < strftime('%s', 'now');
```

## Useful Views

```sql
-- Public APIs
SELECT * FROM public_api_nodes;

-- Call chains
SELECT * FROM call_hierarchy LIMIT 20;

-- Dependency metrics
SELECT * FROM dependency_metrics
ORDER BY total_dependencies DESC
LIMIT 10;

-- Circular dependencies
SELECT * FROM circular_dependency_chains;

-- Code complexity
SELECT * FROM node_complexity_analysis
ORDER BY complexity_score DESC
LIMIT 20;

-- Parse summary
SELECT * FROM parsing_session_summary
ORDER BY started_at DESC
LIMIT 5;
```

## Table Summary

| Table | Purpose | Rows Est. |
|-------|---------|-----------|
| semantic_nodes | AST nodes | 100K-1M |
| semantic_node_parameters | Function params | 100K-500K |
| file_dependencies | Import tracking | 10K-100K |
| semantic_relationships | Node relationships | 50K-500K |
| node_call_graph | Call chains | 50K-500K |
| file_metadata | File stats | 1K-10K |
| circular_dependencies | Cycle detection | 10-1K |
| vector_metadata | Embedding metadata | 100K-1M |
| fts_index_metadata | Search metadata | 100K-1M |
| ast_parsing_sessions | Parse history | 100-1K |

## Column Index Cheat Sheet

```sql
-- Fast lookups
idx_semantic_nodes_file_type
idx_file_dependencies_source
idx_file_dependencies_target
idx_semantic_relationships_source
idx_node_call_graph_caller

-- Language/type filtering
idx_semantic_nodes_language
idx_semantic_nodes_type

-- Search optimization
idx_semantic_nodes_qualified_name
idx_semantic_nodes_name

-- Temporal queries
idx_semantic_nodes_created
idx_file_dependencies_updated

-- Circular detection
idx_file_dependencies_circular
idx_circular_dependencies_severity

-- RAG optimization
idx_semantic_nodes_embedding
idx_vector_metadata_indexed
idx_fts_index_metadata_indexed
```

## Performance Tips

1. **Always use WHERE clauses** - Even small filters help tremendously
2. **Use file_path early** - Most queries should filter by file first
3. **Join on IDs, not names** - Foreign key joins are much faster
4. **Limit result sets** - Use LIMIT for large result sets
5. **Check EXPLAIN QUERY PLAN** - Verify index usage
6. **Run ANALYZE monthly** - Keep statistics current
7. **Use partial indexes** - For filtered queries

## Common Patterns

### Find everything about a node
```sql
SELECT 'node' as type, * FROM semantic_nodes WHERE id = ?
UNION ALL
SELECT 'params', * FROM semantic_node_parameters WHERE node_id = ?
UNION ALL
SELECT 'calls', * FROM node_call_graph WHERE caller_node_id = ?
UNION ALL
SELECT 'called_by', * FROM node_call_graph WHERE callee_node_id = ?
UNION ALL
SELECT 'relationships', * FROM semantic_relationships WHERE source_node_id = ? OR target_node_id = ?;
```

### Find impact of file change
```sql
SELECT DISTINCT sn.id, sn.name
FROM semantic_nodes sn
WHERE sn.file_path = ?

UNION

SELECT DISTINCT sn.id, sn.name
FROM semantic_nodes sn
INNER JOIN file_dependencies fd ON sn.file_path = fd.source_file_path
WHERE fd.target_file_path = ?;
```

### Trace dependencies recursively
```sql
WITH RECURSIVE deps(path, file) AS (
  VALUES(?, ?)
  UNION
  SELECT path || '/' || fd.target_file_path,
         fd.target_file_path
  FROM file_dependencies fd
  WHERE fd.source_file_path = deps.file
)
SELECT DISTINCT file FROM deps;
```

---

**For detailed information, see [SCHEMA_DESIGN.md](SCHEMA_DESIGN.md) and [README.md](README.md)**
