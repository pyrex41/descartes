# SQLite Schema Design for AST and File Dependencies

**Phase 2: Descartes Agent Orchestration System**

## Table of Contents

1. [Architecture Overview](#architecture-overview)
2. [Core Data Structures](#core-data-structures)
3. [Relationships and Constraints](#relationships-and-constraints)
4. [Tri-Store Integration](#tri-store-integration)
5. [Query Patterns](#query-patterns)
6. [Performance Optimization](#performance-optimization)
7. [Migration Strategy](#migration-strategy)

---

## Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────┐
│         Descartes Agent Orchestration System                │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  Phase 1: Core Framework                                  │
│  ├── ModelBackend Trait (API, CLI, Local)                │
│  ├── AgentRunner (Process Management)                     │
│  ├── StateStore (Events, Sessions, Tasks)                │
│  └── ContextSyncer (File/Git Context)                    │
│                                                             │
│  Phase 2: Semantic Analysis (THIS)                        │
│  ├── Semantic Code Parser (tree-sitter)                  │
│  ├── Dependency Analyzer                                  │
│  ├── Relationship Graph                                   │
│  └── Tri-Store RAG Layer                                  │
│      ├── SQLite: Relational + FTS                        │
│      ├── LanceDB: Vector Embeddings                      │
│      └── Tantivy: Full-Text Search                       │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

### Schema Layers

**Layer 1: Semantic Analysis Layer**
- Stores AST nodes and their metadata
- Tracks function/class hierarchies
- Records type information and signatures

**Layer 2: Dependency Layer**
- Maps file-to-file dependencies
- Tracks semantic relationships between nodes
- Detects circular dependencies

**Layer 3: Indexing Layer**
- Optimized indexes for common queries
- Full-text search capabilities
- Vector metadata for semantic search

**Layer 4: RAG Integration Layer**
- Bridges SQLite with LanceDB and Tantivy
- Manages synchronization state
- Tracks performance metrics

---

## Core Data Structures

### 1. Semantic Nodes (AST Storage)

**Purpose**: Store parsed AST nodes from tree-sitter

**Key Tables**:
- `semantic_nodes` - Main AST node storage
- `semantic_node_parameters` - Function/method parameters
- `semantic_node_type_parameters` - Generic/template parameters

**Structure**:
```
semantic_nodes
├── Identification
│   ├── id (PRIMARY KEY): Unique node identifier (hash-based)
│   ├── node_type: Function, Class, Struct, Enum, etc.
│   ├── name: Simple name
│   └── qualified_name: Full path (module::class::method)
│
├── Location
│   ├── file_path: Source file location
│   ├── line_start, line_end: Line range
│   ├── column_start, column_end: Column range
│   └── language: Rust, Python, JavaScript, TypeScript
│
├── Content
│   ├── source_code: Full source text
│   ├── documentation: Preceding comments/docs
│   ├── summary: Brief description for RAG
│   └── metadata: JSON for extensibility
│
├── Code Information
│   ├── signature: Method/function signature
│   ├── return_type: Return type (if applicable)
│   ├── visibility: public, private, protected, etc.
│   ├── is_exported: Public API marker
│   ├── is_async: Async function marker
│   └── is_generic: Has generic parameters
│
├── Hierarchy
│   ├── parent_id: Reference to parent node
│   └── complexity_score: Cyclomatic/LOC complexity
│
├── RAG Integration
│   ├── embedding_hash: Vector embedding hash
│   └── created_at, updated_at: Timestamps
└── Relationships
    └── Parameter & Type Parameter records (1:M)
```

**Indexes**:
- File + Type (common query)
- Qualified Name (search)
- Language (filtering)
- Parent (hierarchy)
- Timestamps (recent)

### 2. File Dependencies

**Purpose**: Track imports, includes, and external references

**Key Table**: `file_dependencies`

**Structure**:
```
file_dependencies
├── Identification
│   ├── id (PRIMARY KEY): Unique dependency record
│   ├── source_file_path: Importing file
│   ├── target_file_path: Imported file
│   └── dependency_type: import, require, include, etc.
│
├── Classification
│   ├── is_relative_import: Relative vs absolute path
│   ├── is_external: External package vs local
│   ├── is_internal: Internal to project
│   ├── is_weak: Optional dependency
│   └── is_circular: Part of a cycle
│
├── Details
│   ├── import_statement: Original import text
│   ├── import_path: Resolved path
│   ├── line_number: Where import appears
│   ├── column_number: Column position
│   └── scope: Module, function, class level
│
└── Metadata
    ├── created_at, updated_at: Timestamps
    └── metadata: JSON extensions
```

**Constraints**:
- UNIQUE(source_file_path, target_file_path, dependency_type, import_path)
- Prevents duplicate dependency records
- Foreign keys prevent orphaned references

**Indexes**:
- Source file (forward deps)
- Target file (backward deps)
- Source + Target (bidirectional)
- Circular flag (cycle detection)
- Dependency type (filtering)

### 3. Semantic Relationships

**Purpose**: Record connections between semantic nodes

**Key Table**: `semantic_relationships`

**Relationship Types**:
```
Function A
├── calls ──────────> Function B
├── called_by ──────> Function C
├── uses ───────────> Type X
├── parameter_of ───> Function D
└── returns ───────> Class E

Class X
├── inherits ──────> Class Y
├── implements ────> Interface Z
├── contains ──────> Method M
└── overrides ─────> Method M (parent)

Interface A
├── implemented_by > Class X
├── contains ──────> Method M
└── extends ──────> Interface B
```

**Structure**:
```
semantic_relationships
├── Entities
│   ├── id (PRIMARY KEY)
│   ├── source_node_id (FOREIGN KEY)
│   └── target_node_id (FOREIGN KEY)
│
├── Relationship
│   ├── relationship_type: calls, inherits, uses, etc.
│   ├── context_file_path: Where relationship is defined
│   ├── context_line_start, context_line_end: Location range
│   └── is_dynamic: Runtime polymorphism flag
│
├── Confidence
│   ├── confidence_score: 0.0 - 1.0
│   ├── is_direct: Direct vs inferred
│   └── metadata: JSON extensions
│
└── Timestamps
    ├── created_at
    └── updated_at
```

**Constraint**: UNIQUE(source_node_id, target_node_id, relationship_type)

**Indexes**:
- Source node (outgoing)
- Target node (incoming)
- Relationship type (filtering)
- Confidence (ranking)
- Context file (location-based)

### 4. Call Graph (Performance Optimization)

**Purpose**: Fast call chain queries without relationship table traversal

**Key Table**: `node_call_graph`

**Structure**:
```
node_call_graph
├── Entities
│   ├── id (PRIMARY KEY)
│   ├── caller_node_id (FOREIGN KEY)
│   └── callee_node_id (FOREIGN KEY)
│
├── Call Details
│   ├── call_count: Number of times called
│   ├── call_sites: CSV of line numbers
│   ├── call_type: direct, indirect, virtual, async
│   ├── file_path: Where call occurs
│   ├── line_number: Call location
│   └── execution_order: For sequential calls
│
├── Analysis
│   ├── is_recursive: Direct recursion
│   ├── is_mutual_recursive: Mutual recursion
│   └── updated_at: Last change timestamp
│
└── Constraint
    └── UNIQUE(caller_node_id, callee_node_id, line_number)
```

**Indexes**:
- Caller (call chains from)
- Callee (call chains to)
- Recursive (find recursion)
- File + Line (location lookup)

---

## Relationships and Constraints

### Foreign Key Relationships

```
semantic_nodes
├── Parent (self-reference)
│   └── parent_id -> semantic_nodes.id
│
└── Parameters
    └── semantic_node_parameters.node_id -> id
    └── semantic_node_type_parameters.node_id -> id

file_dependencies
├── Source: source_file_path -> file_metadata.file_path
└── Target: target_file_path -> file_metadata.file_path

semantic_relationships
├── Source: source_node_id -> semantic_nodes.id
└── Target: target_node_id -> semantic_nodes.id

node_call_graph
├── Caller: caller_node_id -> semantic_nodes.id
└── Callee: callee_node_id -> semantic_nodes.id
```

### Cascade Behaviors

**ON DELETE CASCADE**:
- Deleting a semantic_node cascades to:
  - semantic_node_parameters
  - semantic_node_type_parameters
  - semantic_relationships (source & target)
  - node_call_graph (caller & callee)
  - vector_metadata
  - fts_index_metadata
  - rag_metadata

**Effect**: Clean deletion of node and all associated data

### Uniqueness Constraints

1. `semantic_nodes(id)` - PRIMARY KEY
2. `file_dependencies(source_file_path, target_file_path, dependency_type, import_path)` - UNIQUE
3. `semantic_relationships(source_node_id, target_node_id, relationship_type)` - UNIQUE
4. `node_call_graph(caller_node_id, callee_node_id, line_number)` - UNIQUE

---

## Tri-Store Integration

### Store Architecture

```
┌──────────────────────────────────────┐
│     Application Code (Rust)          │
└──────────────┬───────────────────────┘
               │
        ┌──────┴──────────────────────────────────────┐
        │         SQLx Query Layer                    │
        └──────┬──────────────────────────────────────┘
               │
        ┌──────┴────────────────────────────────┐
        │                                       │
    ┌───▼────────┐    ┌──────────┐    ┌──────▼───────┐
    │   SQLite   │    │ LanceDB  │    │   Tantivy    │
    │            │    │          │    │              │
    │ Relational │    │ Vectors  │    │ Full-Text    │
    │ +FTS5      │    │ Search   │    │ Search       │
    └────────────┘    └──────────┘    └──────────────┘
         │                 │                 │
         │                 │                 │
    ┌────▼─────────────────▼────────────────▼─────┐
    │         Hybrid Search Layer                  │
    │  (Combining results from all three stores)   │
    └─────────────────────────────────────────────┘
```

### Store Responsibilities

**SQLite (Relational)**:
- Primary data storage
- Exact match queries
- Relationship traversal
- Full-text search (FTS5)
- Local caching

**LanceDB (Vector)**:
- Code embeddings
- Semantic similarity
- Multi-modal queries
- Fast approximate search

**Tantivy (Full-Text Search)**:
- Keyword indexing
- Ranking (BM25)
- Complex text queries
- Boost/penalty operations

### Synchronization Metadata

**Vector Store Sync**:
```
vector_metadata
├── vector_id: Reference to LanceDB
├── is_indexed: Synchronization flag
├── indexed_at: Last sync timestamp
├── vector_dimension: Model info
├── embedding_model: Which model produced vector
└── query_count: Usage statistics
```

**FTS Store Sync**:
```
fts_index_metadata
├── doc_id: Tantivy document ID
├── segment_id: Tantivy segment
├── is_indexed: Synchronization flag
├── indexed_at: Last sync timestamp
├── bm25_score: Cached ranking score
└── query_count: Usage statistics
```

**RAG Metadata**:
```
rag_metadata
├── node_id: Reference to semantic_nodes
├── lancedb_vector_id: LanceDB sync
├── tantivy_doc_id: Tantivy sync
├── needs_reindex: Invalidation flag
├── combined_rank: Hybrid ranking
└── last_indexed_at: Sync timestamp
```

---

## Query Patterns

### Pattern 1: Node Lookup
```sql
-- Find node by qualified name
SELECT * FROM semantic_nodes
WHERE qualified_name = 'module::Class::method'
AND language = 'rust';

-- Find all nodes in file
SELECT * FROM semantic_nodes
WHERE file_path = 'src/main.rs'
ORDER BY line_start;

-- Find public API
SELECT * FROM semantic_nodes
WHERE visibility = 'public'
AND is_exported = 1
AND language = ?;
```

### Pattern 2: Dependency Traversal
```sql
-- Direct dependencies
SELECT DISTINCT target_file_path FROM file_dependencies
WHERE source_file_path = ?
AND is_circular = 0;

-- Transitive dependencies (2 levels)
WITH deps AS (
  SELECT DISTINCT target_file_path AS dep
  FROM file_dependencies
  WHERE source_file_path = ?
  UNION ALL
  SELECT DISTINCT fd.target_file_path
  FROM file_dependencies fd
  INNER JOIN deps ON fd.source_file_path = deps.dep
)
SELECT * FROM deps;

-- Find all files that import this file
SELECT DISTINCT source_file_path FROM file_dependencies
WHERE target_file_path = ?;
```

### Pattern 3: Call Chain Analysis
```sql
-- All functions called by X
SELECT cg.callee_node_id, sn.name, sn.signature
FROM node_call_graph cg
INNER JOIN semantic_nodes sn ON cg.callee_node_id = sn.id
WHERE cg.caller_node_id = ?
ORDER BY cg.call_count DESC;

-- Call depth analysis
WITH RECURSIVE call_depth(id, depth) AS (
  SELECT callee_node_id, 1
  FROM node_call_graph
  WHERE caller_node_id = ?

  UNION ALL

  SELECT cg.callee_node_id, depth + 1
  FROM node_call_graph cg
  INNER JOIN call_depth ON cg.caller_node_id = call_depth.id
  WHERE depth < 10
)
SELECT id, MIN(depth) FROM call_depth GROUP BY id;
```

### Pattern 4: Full-Text Search
```sql
-- Simple name search
SELECT * FROM semantic_nodes_fts
WHERE name MATCH 'parser*'
LIMIT 20;

-- Complex FTS query
SELECT * FROM semantic_nodes_fts
WHERE semantic_nodes_fts MATCH 'name:parse AND documentation:regex'
AND file_path LIKE '%.rs'
ORDER BY rank DESC
LIMIT 50;

-- Combine with relational filters
SELECT sn.* FROM semantic_nodes sn
INNER JOIN semantic_nodes_fts fts ON sn.id = fts.id
WHERE fts MATCH 'error handling'
AND sn.language = 'rust'
AND sn.visibility = 'public'
LIMIT 20;
```

### Pattern 5: Circular Dependency Detection
```sql
-- All circular dependencies
SELECT cycle_path, cycle_length, severity
FROM circular_dependencies
WHERE severity IN ('critical', 'high')
ORDER BY cycle_length DESC;

-- Detect new cycles
SELECT source_file_path, target_file_path
FROM file_dependencies fd
WHERE is_circular = 1
AND updated_at > (strftime('%s', 'now') - 86400);
```

### Pattern 6: RAG-Optimized Queries
```sql
-- Find similar semantic nodes
SELECT sn.* FROM semantic_nodes sn
INNER JOIN vector_metadata vm ON sn.id = vm.node_id
WHERE vm.is_indexed = 1
AND sn.language = ?
AND sn.node_type = ?
AND sn.visibility = 'public'
ORDER BY vm.popularity_score DESC
LIMIT 10;

-- Cache-aware search
SELECT * FROM semantic_search_cache
WHERE query_hash = ?
AND expires_at > strftime('%s', 'now')
LIMIT 1;
```

---

## Performance Optimization

### Index Strategy

**Hot Indexes** (Created First):
```sql
-- Frequently used lookups
idx_semantic_nodes_file_type
idx_file_dependencies_source
idx_file_dependencies_target
idx_semantic_relationships_source
idx_node_call_graph_caller
```

**Warm Indexes** (Created Next):
```sql
-- Secondary access patterns
idx_semantic_nodes_language
idx_semantic_nodes_qualified_name
idx_file_dependencies_type
idx_semantic_relationships_type
```

**Cold Indexes** (Created Last):
```sql
-- Rare or analytics queries
idx_circular_dependencies_severity
idx_semantic_search_cache_expires
idx_code_change_tracking_processed
```

### Partial Indexes (for specific queries)

```sql
-- Only public/exported nodes for API discovery
idx_public_semantic_nodes
ON semantic_nodes(file_path, node_type, qualified_name)
WHERE visibility = 'public' AND is_exported = 1;

-- Only direct relationships for common graphs
idx_direct_relationships
ON semantic_relationships(source_node_id, target_node_id)
WHERE is_direct = 1;

-- Only unprocessed changes
idx_unprocessed_changes
ON code_change_tracking(file_path, change_type)
WHERE processed = 0;
```

### Query Optimization Techniques

**1. Covering Indexes**:
```sql
-- Covers typical node query
CREATE INDEX idx_semantic_nodes_coverage
ON semantic_nodes(
  file_path,
  node_type,
  line_start
) INCLUDE (name, qualified_name, visibility);
```

**2. Statistics**:
```sql
ANALYZE;  -- Updates query planner statistics
```

**3. PRAGMA Settings**:
```sql
PRAGMA cache_size = -64000;      -- 64MB cache
PRAGMA synchronous = NORMAL;      -- Balance speed/safety
PRAGMA journal_mode = WAL;        -- Better concurrency
PRAGMA query_only = ON;           -- Read-only mode for replicas
```

### Storage Optimization

**Compression for Large Content**:
```sql
-- For rarely-accessed source code
ALTER TABLE semantic_nodes
ADD COLUMN source_code_compressed BLOB;
```

**Partitioning Strategy** (for very large datasets):
```sql
-- Partition by language
CREATE TABLE semantic_nodes_rust AS
SELECT * FROM semantic_nodes
WHERE language = 'rust';

-- Add constraints and indexes per partition
```

---

## Migration Strategy

### Phase 1 → Phase 2 Data Migration

1. **Verify Phase 1 Schema**:
   - Check events, sessions, tasks tables exist
   - Validate data integrity

2. **Run Migration Scripts in Order**:
   ```bash
   001_initial_schema.sql      # Create all tables
   002_create_indexes.sql       # Create indexes
   003_fts_and_optimization.sql # Triggers & FTS
   004_rag_layer_integration.sql # RAG tables
   005_initialization_procedures.sql # Helpers
   ```

3. **Import Existing Data**:
   - Link Phase 1 events to parsing sessions
   - Associate tasks with parsing operations
   - Update session metadata with AST stats

4. **Validate Migration**:
   ```sql
   SELECT * FROM required_tables_check;
   SELECT * FROM schema_integrity_check;
   ```

### Rollback Procedure

```sql
-- Store rollback point
INSERT INTO rollback_points(id, migration_version, description)
VALUES (?, 4, 'Before RAG layer migration');

-- Rollback specific migration
DROP TABLE IF EXISTS vector_metadata;
DROP TABLE IF EXISTS fts_index_metadata;
-- ... etc

-- Update version tracking
UPDATE schema_versions
SET status = 'rolled_back',
    rolled_back_at = strftime('%s', 'now')
WHERE version = 4;
```

---

## Schema Extension Examples

### Adding a New Node Type

```sql
-- Add to node type checking
ALTER TABLE semantic_nodes
MODIFY node_type TEXT NOT NULL
CHECK(node_type IN (
    'module', 'function', ..., 'newtype'
));

-- Create indexes if query pattern changed
CREATE INDEX idx_semantic_nodes_newtype
ON semantic_nodes(file_path, name)
WHERE node_type = 'newtype';
```

### Adding Custom Metadata

```sql
-- Use JSON in existing metadata column
INSERT INTO semantic_nodes(
    ..., metadata
) VALUES (
    ..., json_object(
        'custom_field', 'value',
        'analysis_results', json_array(1,2,3)
    )
);

-- Query custom metadata
SELECT * FROM semantic_nodes
WHERE json_extract(metadata, '$.custom_field') = 'value';
```

### Adding New Indexes

```sql
-- For new query pattern
CREATE INDEX idx_new_pattern
ON semantic_nodes(column1, column2)
WHERE filter_condition;

-- Update statistics
ANALYZE;

-- Verify improvement
EXPLAIN QUERY PLAN SELECT ...;
```

---

## Monitoring and Maintenance

### Regular Maintenance Tasks

**Weekly**:
- Run `ANALYZE` to update statistics
- Check `stale_records` view
- Monitor `system_health_diagnostics`

**Monthly**:
- Run `VACUUM` to reclaim space
- Check `unused_semantic_nodes`
- Review `query_statistics` for slow queries

**Quarterly**:
- Rebuild fragmented indexes: `REINDEX`
- Archive old parsing sessions
- Analyze growth trends

### Performance Baselines

**Expected Performance**:
- Simple lookup: < 1ms
- File scan: 1-5ms
- FTS search: 5-50ms
- Complex relationship query: 10-100ms
- Circular dependency detection: 50-500ms (first run)

**Warning Thresholds**:
- Query time > 1s for common queries
- Cache hit rate < 50%
- Index scan ratio > 20% sequential scans
- Database size growth > 50% per month

---

## References

- [SQLite Query Optimizer](https://www.sqlite.org/optoverview.html)
- [FTS5 Full Text Search](https://www.sqlite.org/fts5.html)
- [Schema Best Practices](https://www.sqlite.org/bestpractice.html)
- [VACUUM and ANALYZE](https://www.sqlite.org/lang_vacuum.html)
