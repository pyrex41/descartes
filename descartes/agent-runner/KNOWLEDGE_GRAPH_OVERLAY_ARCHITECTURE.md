# Knowledge Graph Overlay Architecture

## System Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                    Knowledge Graph Overlay System                │
│                                                                   │
│  ┌──────────────┐         ┌──────────────┐        ┌──────────┐ │
│  │  File Tree   │────────▶│   Overlay    │───────▶│Knowledge │ │
│  │   Builder    │         │   Manager    │        │  Graph   │ │
│  │              │         │              │        │          │ │
│  └──────────────┘         └──────────────┘        └──────────┘ │
│         │                        │                      │        │
│         │                        │                      │        │
│         ▼                        ▼                      ▼        │
│  ┌──────────────┐         ┌──────────────┐        ┌──────────┐ │
│  │  File Tree   │◀────────│  Semantic    │        │  Query   │ │
│  │   Nodes      │         │   Parser     │        │   API    │ │
│  └──────────────┘         └──────────────┘        └──────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Component Architecture

### 1. Knowledge Graph Overlay Manager

The central orchestrator that coordinates all operations:

```
KnowledgeGraphOverlay
├── Configuration (OverlayConfig)
├── Semantic Parser (multi-language)
├── Cache Manager (file-based caching)
└── Query Engine (semantic queries)
```

### 2. Data Flow

```
Source Files
    │
    ▼
┌─────────────────┐
│ File Tree       │
│ Builder         │  Scans directory structure
│                 │  Collects file metadata
└────────┬────────┘  Detects languages
         │
         ▼
┌─────────────────┐
│ Knowledge       │
│ Graph Overlay   │  Parses source code
│                 │  Extracts entities
└────────┬────────┘  Detects relationships
         │
         ▼
┌─────────────────┐
│ Knowledge       │
│ Graph           │  Stores nodes & edges
│                 │  Provides queries
└─────────────────┘  Enables navigation
```

### 3. Entity Extraction Pipeline

```
Source File → Tree-Sitter Parser → AST → Semantic Extractor → Knowledge Node
                                                                      │
                                                                      ▼
                                                              ┌───────────────┐
                                                              │  Node Details │
                                                              ├───────────────┤
                                                              │ • Name        │
                                                              │ • Type        │
                                                              │ • Signature   │
                                                              │ • Parameters  │
                                                              │ • Return Type │
                                                              │ • Source Code │
                                                              │ • Location    │
                                                              └───────────────┘
```

### 4. Bidirectional Linking

```
FileTreeNode                                KnowledgeNode
┌──────────────┐                          ┌──────────────┐
│ path         │                          │ qualified    │
│ name         │                          │ _name        │
│ metadata     │                          │ content_type │
│              │                          │ source_code  │
│ knowledge_   │──────────────────────────│ file_        │
│ links: []    │                          │ references[] │
└──────────────┘                          └──────────────┘
      │                                          │
      └──────────── Bidirectional ──────────────┘
                      References
```

### 5. Relationship Graph

```
                    ┌─────────────┐
                    │  Function A │
                    └─────┬───────┘
                          │ calls
                          ▼
                    ┌─────────────┐
                    │  Function B │
                    └─────┬───────┘
                          │ uses
                          ▼
                    ┌─────────────┐
                    │   Struct C  │
                    └─────┬───────┘
                          │ defined_in
                          ▼
                    ┌─────────────┐
                    │   Module D  │
                    └─────────────┘
```

## Layer Architecture

### Presentation Layer (Query API)

```
┌────────────────────────────────────────────────────────┐
│                     Query Operations                    │
├────────────────────────────────────────────────────────┤
│ • find_by_type(type)                                   │
│ • find_entities_in_file(path)                          │
│ • find_definition(name)                                │
│ • find_references(name)                                │
│ • traverse_call_graph(name, depth)                     │
│ • find_callers(name) / find_callees(name)             │
│ • find_by_name_pattern(pattern)                        │
└────────────────────────────────────────────────────────┘
```

### Business Logic Layer

```
┌────────────────────────────────────────────────────────┐
│                  Core Logic Components                  │
├────────────────────────────────────────────────────────┤
│ ┌──────────────────┐   ┌──────────────────┐           │
│ │ Entity Extraction│   │ Relationship     │           │
│ │ • Parse files    │   │ Detection        │           │
│ │ • Extract nodes  │   │ • Calls          │           │
│ │ • Collect meta   │   │ • Imports        │           │
│ └──────────────────┘   │ • Inheritance    │           │
│                        └──────────────────┘           │
│ ┌──────────────────┐   ┌──────────────────┐           │
│ │ Linking Logic    │   │ Update Manager   │           │
│ │ • File→Knowledge │   │ • Incremental    │           │
│ │ • Knowledge→File │   │ • Remove old     │           │
│ │ • Consistency    │   │ • Add new        │           │
│ └──────────────────┘   └──────────────────┘           │
└────────────────────────────────────────────────────────┘
```

### Data Layer

```
┌────────────────────────────────────────────────────────┐
│                      Data Storage                       │
├────────────────────────────────────────────────────────┤
│ ┌──────────────────┐   ┌──────────────────┐           │
│ │ Knowledge Graph  │   │  File Tree       │           │
│ │ • nodes: HashMap │   │ • nodes: HashMap │           │
│ │ • edges: HashMap │   │ • path_index     │           │
│ │ • indices        │   │ • metadata       │           │
│ └──────────────────┘   └──────────────────┘           │
│ ┌──────────────────┐   ┌──────────────────┐           │
│ │ Cache            │   │  Parser State    │           │
│ │ • entries        │   │ • parsers        │           │
│ │ • timestamps     │   │ • grammars       │           │
│ └──────────────────┘   └──────────────────┘           │
└────────────────────────────────────────────────────────┘
```

## Processing Flow

### Full Generation

```
1. Input: FileTree
   │
   ▼
2. Filter parseable files
   │ (by language & size)
   ▼
3. For each file:
   ├─ Check cache
   ├─ Parse source code
   ├─ Extract entities
   ├─ Create knowledge nodes
   └─ Link to file tree node
   │
   ▼
4. Extract relationships
   │ (between all nodes)
   ▼
5. Build graph indices
   │ (for fast queries)
   ▼
6. Output: KnowledgeGraph
```

### Incremental Update

```
1. Input: Changed file path
   │
   ▼
2. Remove old entities
   │ (from this file)
   ▼
3. Re-parse file
   │
   ▼
4. Extract new entities
   │
   ▼
5. Re-build relationships
   │ (affected edges)
   ▼
6. Update indices
   │
   ▼
7. Invalidate cache
   │
   ▼
8. Output: Updated KnowledgeGraph
```

### Query Processing

```
1. Query request
   │ (e.g., find_by_type)
   ▼
2. Check indices
   │ (for fast lookup)
   ▼
3. Filter nodes
   │ (by criteria)
   ▼
4. Collect results
   │
   ▼
5. Return references
   │ (to nodes)
   ▼
6. Output: Vec<&KnowledgeNode>
```

## Module Dependencies

```
knowledge_graph_overlay
    │
    ├─── knowledge_graph (data models)
    │    ├─ FileTree
    │    ├─ FileTreeNode
    │    ├─ KnowledgeGraph
    │    ├─ KnowledgeNode
    │    └─ KnowledgeEdge
    │
    ├─── parser (code parsing)
    │    └─ SemanticParser
    │
    ├─── semantic (entity extraction)
    │    └─ SemanticExtractor
    │
    ├─── types (language definitions)
    │    ├─ Language
    │    └─ SemanticNode
    │
    └─── errors (error handling)
         ├─ ParserError
         └─ ParserResult
```

## Configuration Options

```
OverlayConfig
    │
    ├─ enabled_languages: Vec<Language>
    │  └─ Which languages to parse
    │
    ├─ extract_relationships: bool
    │  └─ Whether to detect relationships
    │
    ├─ max_file_size: Option<u64>
    │  └─ Skip files larger than this
    │
    ├─ enable_cache: bool
    │  └─ Enable result caching
    │
    ├─ cache_dir: Option<PathBuf>
    │  └─ Where to store cache
    │
    ├─ cache_ttl: Duration
    │  └─ How long cache is valid
    │
    └─ parallel_parsing: bool
       └─ Parse files in parallel
```

## Memory Layout

```
KnowledgeGraphOverlay
    │
    ├─ config: OverlayConfig (stack)
    │
    ├─ parser: SemanticParser (heap)
    │   └─ parsers: HashMap<Language, Parser>
    │
    └─ cache: HashMap<PathBuf, CacheEntry> (heap)
        └─ entries with timestamps

KnowledgeGraph
    │
    ├─ nodes: HashMap<String, KnowledgeNode> (heap)
    │   └─ ~1KB per node
    │
    ├─ edges: HashMap<String, KnowledgeEdge> (heap)
    │   └─ ~200B per edge
    │
    └─ indices: Multiple HashMaps (heap)
        ├─ name_index
        ├─ type_index
        ├─ outgoing_edges
        └─ incoming_edges
```

## Performance Profile

```
Operation              Time Complexity    Space Complexity
────────────────────────────────────────────────────────────
generate_overlay       O(n * m)           O(n + e)
find_by_type          O(k)               O(1)
find_entities_in_file O(n)               O(1)
find_definition       O(1)               O(1)
update_file           O(m + e')          O(n')
traverse_call_graph   O(e * d)           O(d)

where:
  n = number of nodes
  e = number of edges
  m = file size (lines)
  k = nodes of type
  d = max depth
  e' = affected edges
  n' = new nodes
```

## Thread Safety

```
KnowledgeGraphOverlay
    │
    ├─ Not thread-safe by default
    │  (contains &mut references)
    │
    ├─ Can be wrapped in Arc<Mutex<>>
    │  for multi-threaded access
    │
    └─ Internal parallel parsing
       (via rayon for file processing)

KnowledgeGraph
    │
    ├─ Read operations: Thread-safe
    │  (uses & references)
    │
    └─ Write operations: Requires &mut
       (not thread-safe)
```

## Extension Points

```
1. Custom Entity Types
   └─ Extend KnowledgeNodeType enum

2. Custom Relationships
   └─ Extend RelationshipType enum

3. New Languages
   └─ Add tree-sitter grammar
   └─ Update language detection

4. Advanced Analysis
   └─ Implement on top of query API
   └─ Use graph traversal methods

5. Persistent Storage
   └─ Serialize KnowledgeGraph
   └─ Save to database or file
```

## Error Handling Flow

```
Operation
    │
    ├─ Success
    │  └─ Return Ok(result)
    │
    └─ Failure
       ├─ I/O Error
       │  └─ ParserError::IoError(e)
       │
       ├─ Parse Error
       │  └─ ParserError::ParseError(msg)
       │
       ├─ Invalid Language
       │  └─ ParserError::InvalidLanguage(lang)
       │
       └─ Other
          └─ Log warning
          └─ Continue with partial results
```

## Integration Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    External Systems                      │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  IDE / Editor          CLI Tool          Web Service    │
│       │                   │                    │         │
│       └───────────────────┴────────────────────┘         │
│                           │                              │
└───────────────────────────┼──────────────────────────────┘
                            │
                            ▼
        ┌───────────────────────────────────┐
        │   Knowledge Graph Overlay API     │
        └───────────────────────────────────┘
                            │
        ┌───────────────────┴───────────────┐
        │                                   │
        ▼                                   ▼
┌──────────────┐                    ┌──────────────┐
│  File Tree   │                    │  Knowledge   │
│  (phase 9.2) │◀──────────────────▶│   Graph      │
│              │    Bidirectional   │  (phase 9.1) │
└──────────────┘                    └──────────────┘
```

---

This architecture provides a solid foundation for semantic code analysis and navigation, with clear separation of concerns, efficient data structures, and extensibility for future enhancements.
