# File Tree and Knowledge Graph Architecture Diagram

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Code Repository                              │
│                                                                       │
│  ┌─────────────────────────┐       ┌────────────────────────────┐  │
│  │      File Tree          │◄─────►│    Knowledge Graph         │  │
│  │                         │       │                            │  │
│  │  Represents:            │       │  Represents:               │  │
│  │  - File system          │       │  - Code entities           │  │
│  │  - Directory structure  │       │  - Semantic relationships  │  │
│  │  - File metadata        │       │  - Dependencies            │  │
│  └─────────────────────────┘       └────────────────────────────┘  │
│              │                                  │                    │
│              │                                  │                    │
│              ▼                                  ▼                    │
│  ┌─────────────────────────┐       ┌────────────────────────────┐  │
│  │   FileTreeNode          │       │   KnowledgeNode            │  │
│  │   - node_id             │       │   - node_id                │  │
│  │   - path                │       │   - content_type           │  │
│  │   - name                │       │   - name                   │  │
│  │   - node_type           │       │   - qualified_name         │  │
│  │   - parent_id           │       │   - description            │  │
│  │   - children[]          │       │   - source_code            │  │
│  │   - metadata            │       │   - file_references[]      │  │
│  │   - knowledge_links[]   │──────►│   - parent_id              │  │
│  │   - indexed             │       │   - children[]             │  │
│  │   - depth               │       │   - signature              │  │
│  └─────────────────────────┘       │   - parameters[]           │  │
│                                     │   - tags                   │  │
│                                     └────────────────────────────┘  │
│                                                  │                   │
│                                                  │ connected by      │
│                                                  ▼                   │
│                                     ┌────────────────────────────┐  │
│                                     │   KnowledgeEdge            │  │
│                                     │   - edge_id                │  │
│                                     │   - from_node_id           │  │
│                                     │   - to_node_id             │  │
│                                     │   - relationship_type      │  │
│                                     │   - weight                 │  │
│                                     └────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

## Bidirectional Linking

```
FileTreeNode                          KnowledgeNode
┌─────────────────┐                  ┌──────────────────┐
│ node_id: "f1"   │                  │ node_id: "k1"    │
│ path: /src/main │                  │ name: "main"     │
│ knowledge_links │─────────────────►│ file_references  │
│   ["k1", "k2"]  │                  │   [{             │
│                 │                  │     file_id: "f1"│
│                 │◄─────────────────│     path: ...    │
│                 │                  │     line: (1,20) │
│                 │                  │   }]             │
└─────────────────┘                  └──────────────────┘
```

## Knowledge Graph Relationships

```
┌─────────────┐                           ┌─────────────┐
│ KnowledgeNode│                           │ KnowledgeNode│
│  Function   │                           │  Function   │
│  "main"     │        KnowledgeEdge      │ "init"      │
│             │────────────────────────────│             │
│             │  RelationshipType::Calls  │             │
└─────────────┘                           └─────────────┘
       │
       │ KnowledgeEdge
       │ RelationshipType::Uses
       ▼
┌─────────────┐
│ KnowledgeNode│
│   Struct    │
│  "Config"   │
│             │
└─────────────┘
```

## Complete Example Flow

```
1. File System
   └── /project
       ├── src/
       │   ├── lib.rs
       │   └── main.rs
       └── tests/
           └── test.rs

2. File Tree Representation
   FileTreeNode (root: /project)
   ├── FileTreeNode (src/)
   │   ├── FileTreeNode (lib.rs)
   │   │   └── knowledge_links: [module_node, func1_node, struct_node]
   │   └── FileTreeNode (main.rs)
   │       └── knowledge_links: [main_func_node]
   └── FileTreeNode (tests/)
       └── FileTreeNode (test.rs)
           └── knowledge_links: [test_func_node]

3. Knowledge Graph Representation
   KnowledgeNode (Module: "mylib")
   ├── defines ──► KnowledgeNode (Struct: "Config")
   │               └── file_ref: lib.rs, lines 10-15
   ├── defines ──► KnowledgeNode (Function: "load_config")
   │               ├── file_ref: lib.rs, lines 20-30
   │               └── uses ──► Config
   └── defines ──► KnowledgeNode (Function: "save_config")
                   ├── file_ref: lib.rs, lines 35-45
                   └── uses ──► Config

   KnowledgeNode (Function: "main")
   ├── file_ref: main.rs, lines 1-20
   ├── calls ──► load_config
   └── calls ──► save_config

   KnowledgeNode (Function: "test_config")
   ├── file_ref: test.rs, lines 5-20
   └── calls ──► load_config
```

## Data Flow: Parsing to Graph

```
┌──────────────┐
│ Source Files │
└──────┬───────┘
       │
       ▼
┌──────────────────┐
│ SemanticParser   │  (Tree-Sitter)
│ - Parse AST      │
│ - Extract nodes  │
└──────┬───────────┘
       │
       │ SemanticNode[]
       │
       ▼
┌──────────────────┐
│ Converter        │
│ - Create nodes   │
│ - Build edges    │
└──────┬───────────┘
       │
       ▼
┌──────────────────┐       ┌─────────────────┐
│ KnowledgeGraph   │       │ FileTree        │
│ - Store nodes    │◄─────►│ - Store files   │
│ - Store edges    │       │ - Link to nodes │
└──────────────────┘       └─────────────────┘
       │
       ▼
┌──────────────────┐
│ CodeRepository   │
│ - Combined view  │
└──────────────────┘
```

## Query Patterns

### 1. Find all code in a file
```
FileTreeNode (path)
    │
    └─► knowledge_links[]
         │
         └─► KnowledgeNode[]
              └─► Returns: All code entities in that file
```

### 2. Find where a function is defined
```
KnowledgeNode (function)
    │
    └─► file_references[]
         │
         └─► FileReference
              └─► file_node_id
                   └─► FileTreeNode (path, line_range)
```

### 3. Find dependencies of a function
```
KnowledgeNode (function)
    │
    └─► outgoing_edges[]
         │
         └─► KnowledgeEdge (relationship_type: Calls/Uses)
              │
              └─► to_node_id
                   └─► KnowledgeNode (dependency)
```

### 4. Find all functions in a module
```
KnowledgeNode (module)
    │
    └─► outgoing_edges[]
         │
         └─► KnowledgeEdge (relationship_type: Defines)
              │
              └─► to_node_id
                   └─► KnowledgeNode (content_type: Function)
```

## Integration with RAG System

```
┌─────────────────┐
│ KnowledgeGraph  │
│                 │
│ KnowledgeNode   │
│ - source_code   │
│ - description   │
│ - signature     │
└────────┬────────┘
         │
         │ Convert to chunks
         ▼
┌─────────────────┐
│ CodeChunk       │
│ - content       │
│ - file_path     │
│ - chunk_type    │
│ - metadata      │
└────────┬────────┘
         │
         │ Generate embedding
         ▼
┌─────────────────┐       ┌──────────────┐
│ VectorStore     │       │ FullTextIndex│
│ (LanceDB)       │       │ (Tantivy)    │
│ - Embeddings    │       │ - Keywords   │
└────────┬────────┘       └──────┬───────┘
         │                       │
         │                       │
         └───────────┬───────────┘
                     │
                     ▼
              ┌─────────────┐
              │ RAG System  │
              │ - Search    │
              │ - Retrieve  │
              └─────────────┘
```

## Indexing Pipeline

```
1. Scan File System
   └─► Build FileTree
        │
        └─► For each file:
             │
             ├─► Parse with SemanticParser
             │    └─► Extract SemanticNodes
             │
             ├─► Convert to KnowledgeNodes
             │    └─► Add to KnowledgeGraph
             │
             ├─► Analyze dependencies
             │    └─► Create KnowledgeEdges
             │
             ├─► Link FileTreeNode to KnowledgeNodes
             │    └─► Set knowledge_links[]
             │
             ├─► Link KnowledgeNodes to FileTreeNode
             │    └─► Set file_references[]
             │
             └─► Generate embeddings
                  └─► Store in VectorStore
```

## Statistics Collection

```
CodeRepository
├── FileTreeStats
│   ├── total_nodes
│   ├── file_count
│   ├── directory_count
│   ├── indexed_count
│   └── max_depth
│
└── KnowledgeGraphStats
    ├── total_nodes
    ├── total_edges
    ├── node_type_counts
    │   ├── function: 150
    │   ├── struct: 45
    │   ├── class: 30
    │   └── ...
    └── avg_degree
```

## Use Cases

### 1. Code Navigation
```
User: "Where is function X defined?"
    ↓
Query KnowledgeGraph by name
    ↓
Get KnowledgeNode
    ↓
Read file_references[]
    ↓
Return: File path and line number
```

### 2. Dependency Analysis
```
User: "What does module Y depend on?"
    ↓
Find KnowledgeNode (Module: Y)
    ↓
Get outgoing edges (Imports, Uses, DependsOn)
    ↓
Collect target nodes
    ↓
Return: List of dependencies
```

### 3. Impact Analysis
```
User: "What will break if I change struct Z?"
    ↓
Find KnowledgeNode (Struct: Z)
    ↓
Get incoming edges (all relationships)
    ↓
Find all dependent nodes
    ↓
Traverse dependency graph
    ↓
Return: List of affected code entities
```

### 4. Code Search
```
User: "Find all error handling code"
    ↓
Search KnowledgeGraph (tags, descriptions)
    ↓
Filter by tag: "error-handling"
    ↓
Get file_references[] for each match
    ↓
Return: Files and line numbers
```
