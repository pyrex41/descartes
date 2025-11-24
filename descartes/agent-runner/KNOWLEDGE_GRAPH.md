# File Tree and Knowledge Graph Data Models

## Overview

This document describes the data models for representing file system hierarchies and code knowledge graphs in the Descartes agent-runner system. These models enable semantic understanding of code structure and relationships.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CodeRepository                          │
│  ┌───────────────────┐          ┌─────────────────────┐    │
│  │    File Tree      │◄────────►│  Knowledge Graph    │    │
│  │                   │  Links   │                     │    │
│  │  - FileTreeNode   │          │  - KnowledgeNode    │    │
│  │  - FileMetadata   │          │  - KnowledgeEdge    │    │
│  │  - Traversal      │          │  - Relationships    │    │
│  └───────────────────┘          └─────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

## File Tree Models

### FileTreeNode

Represents a file or directory in the file system hierarchy.

**Fields:**
- `node_id: String` - Unique identifier (UUID)
- `path: PathBuf` - Full file system path
- `name: String` - File/directory name (without path)
- `node_type: FileNodeType` - File, Directory, or Symlink
- `parent_id: Option<String>` - Parent node reference (None for root)
- `children: Vec<String>` - Child node IDs (for directories)
- `metadata: FileMetadata` - File metadata
- `knowledge_links: Vec<String>` - Links to knowledge graph nodes
- `indexed: bool` - Whether indexed for search
- `depth: usize` - Depth in tree (0 for root)

**Key Methods:**
- `new(path, node_type, parent_id, depth)` - Create new node
- `is_directory()` - Check if node is a directory
- `is_file()` - Check if node is a file
- `add_child(child_id)` - Add child node
- `add_knowledge_link(node_id)` - Link to knowledge graph node
- `extension()` - Get file extension

### FileMetadata

Contains metadata about a file.

**Fields:**
- `size: Option<u64>` - File size in bytes
- `modified: Option<i64>` - Last modified timestamp
- `created: Option<i64>` - Creation timestamp
- `permissions: Option<u32>` - Unix permissions
- `mime_type: Option<String>` - MIME type
- `language: Option<Language>` - Programming language
- `is_binary: bool` - Binary file flag
- `line_count: Option<usize>` - Number of lines
- `git_status: Option<String>` - Git status
- `custom: HashMap<String, String>` - Custom metadata

### FileTree

Container for the complete file tree structure.

**Fields:**
- `root_id: Option<String>` - Root node ID
- `nodes: HashMap<String, FileTreeNode>` - All nodes by ID
- `path_index: HashMap<PathBuf, String>` - Path to node_id mapping
- `file_count: usize` - Total number of files
- `directory_count: usize` - Total number of directories
- `base_path: PathBuf` - Base path of the tree

**Key Methods:**
- `new(base_path)` - Create new tree
- `add_node(node)` - Add node to tree
- `get_node(node_id)` - Get node by ID
- `get_node_by_path(path)` - Get node by path
- `get_children(node_id)` - Get all children of a node
- `get_all_files()` - Get all file nodes
- `get_all_directories()` - Get all directory nodes
- `traverse_depth_first(visitor)` - Depth-first traversal
- `traverse_breadth_first(visitor)` - Breadth-first traversal
- `find_nodes(predicate)` - Find nodes matching predicate
- `stats()` - Get tree statistics

### FileTreeStats

Statistics about the file tree.

**Fields:**
- `total_nodes: usize`
- `file_count: usize`
- `directory_count: usize`
- `indexed_count: usize`
- `max_depth: usize`

## Knowledge Graph Models

### KnowledgeNode

Represents a code entity (function, class, struct, etc.) in the knowledge graph.

**Fields:**
- `node_id: String` - Unique identifier (UUID)
- `content_type: KnowledgeNodeType` - Type of code entity
- `name: String` - Entity name
- `qualified_name: String` - Fully qualified name (e.g., "module::class::method")
- `description: Option<String>` - Documentation/description
- `source_code: Option<String>` - Source code content
- `language: Option<Language>` - Programming language
- `signature: Option<String>` - Function/method signature
- `return_type: Option<String>` - Return type
- `parameters: Vec<String>` - Parameter list
- `type_parameters: Vec<String>` - Generic/template parameters
- `file_references: Vec<FileReference>` - File locations
- `parent_id: Option<String>` - Parent entity
- `children: Vec<String>` - Child entities
- `visibility: Option<String>` - Public, private, etc.
- `tags: HashSet<String>` - Categorization tags
- `metadata: HashMap<String, String>` - Additional metadata

**Key Methods:**
- `new(content_type, name, qualified_name)` - Create new node
- `add_file_reference(file_ref)` - Add file location
- `add_child(child_id)` - Add child entity
- `add_tag(tag)` - Add categorization tag

### KnowledgeNodeType

Types of code entities:
- `Function` - Function definition
- `Method` - Method (part of class/struct)
- `Class` - Class definition
- `Struct` - Struct definition
- `Enum` - Enum definition
- `Interface` - Interface/Trait definition
- `Module` - Module/Package
- `TypeAlias` - Type alias
- `Constant` - Constant definition
- `Variable` - Variable declaration
- `Macro` - Macro definition
- `Concept` - High-level abstraction
- `Documentation` - Comment/Documentation

### FileReference

Reference to a location in a file.

**Fields:**
- `file_node_id: String` - File tree node ID
- `file_path: PathBuf` - File path
- `line_range: (usize, usize)` - Line range [start, end]
- `column_range: Option<(usize, usize)>` - Column range
- `is_definition: bool` - Primary definition flag

### KnowledgeEdge

Represents a relationship between knowledge nodes.

**Fields:**
- `edge_id: String` - Unique identifier (UUID)
- `from_node_id: String` - Source node ID
- `to_node_id: String` - Target node ID
- `relationship_type: RelationshipType` - Type of relationship
- `weight: f32` - Relationship strength (0.0-1.0)
- `metadata: HashMap<String, String>` - Additional metadata

**Key Methods:**
- `new(from_id, to_id, relationship_type)` - Create new edge
- `with_weight(weight)` - Set relationship weight

### RelationshipType

Types of relationships between entities:
- `Calls` - Function A calls function B
- `Imports` - Module A imports module B
- `Inherits` - Class A inherits from class B
- `Implements` - Class A implements interface B
- `Uses` - Entity A uses/references entity B
- `Defines` - Entity A defines entity B
- `DefinedIn` - Entity A is defined in entity B
- `Overrides` - Entity A overrides entity B
- `Extends` - Entity A extends entity B
- `DependsOn` - Entity A depends on entity B
- `SimilarTo` - Entity A is similar to entity B
- `Related` - Generic relationship

### KnowledgeGraph

Container for the complete knowledge graph.

**Fields:**
- `nodes: HashMap<String, KnowledgeNode>` - All nodes by ID
- `edges: HashMap<String, KnowledgeEdge>` - All edges by ID
- `name_index: HashMap<String, String>` - Qualified name to node_id mapping
- `outgoing_edges: HashMap<String, Vec<String>>` - Outgoing edge index
- `incoming_edges: HashMap<String, Vec<String>>` - Incoming edge index
- `type_index: HashMap<KnowledgeNodeType, Vec<String>>` - Node type index

**Key Methods:**
- `new()` - Create new graph
- `add_node(node)` - Add node to graph
- `add_edge(edge)` - Add edge to graph
- `get_node(node_id)` - Get node by ID
- `get_node_by_name(qualified_name)` - Get node by name
- `get_nodes_by_type(node_type)` - Get all nodes of a type
- `get_outgoing_edges(node_id)` - Get outgoing edges
- `get_incoming_edges(node_id)` - Get incoming edges
- `get_neighbors(node_id)` - Get all connected nodes
- `get_neighbors_by_relationship(node_id, relationship)` - Get neighbors by relationship type
- `find_path(from_id, to_id)` - Find shortest path between nodes
- `find_nodes(predicate)` - Find nodes matching predicate
- `stats()` - Get graph statistics

### KnowledgeGraphStats

Statistics about the knowledge graph.

**Fields:**
- `total_nodes: usize`
- `total_edges: usize`
- `node_type_counts: HashMap<String, usize>`
- `avg_degree: f32`

## Combined Container

### CodeRepository

Combines file tree and knowledge graph with bidirectional links.

**Fields:**
- `file_tree: FileTree` - File system structure
- `knowledge_graph: KnowledgeGraph` - Code entity graph
- `metadata: HashMap<String, String>` - Repository metadata

**Key Methods:**
- `new(base_path)` - Create new repository
- `stats()` - Get combined statistics

## Linking File Tree and Knowledge Graph

The models are designed for bidirectional linking:

1. **File Tree → Knowledge Graph:**
   - `FileTreeNode.knowledge_links` contains IDs of knowledge nodes found in that file
   - Enables "What code entities are in this file?" queries

2. **Knowledge Graph → File Tree:**
   - `KnowledgeNode.file_references` contains file locations where the entity appears
   - Enables "Where is this function defined?" queries

### Example Linking Pattern

```rust
// Create file node
let mut file_node = FileTreeNode::new(path, FileNodeType::File, parent, depth);
let file_id = file_tree.add_node(file_node);

// Create knowledge node
let mut func_node = KnowledgeNode::new(
    KnowledgeNodeType::Function,
    "my_function".to_string(),
    "module::my_function".to_string(),
);

// Add file reference to knowledge node
func_node.add_file_reference(FileReference::new(
    file_id.clone(),
    path.clone(),
    (10, 20), // line range
));
let func_id = knowledge_graph.add_node(func_node);

// Link file node to knowledge node
if let Some(file) = file_tree.get_node_mut(&file_id) {
    file.add_knowledge_link(func_id.clone());
}
```

## Traversal Patterns

### File Tree Traversal

**Depth-First:**
```rust
file_tree.traverse_depth_first(|node| {
    println!("{}", node.path.display());
});
```

**Breadth-First:**
```rust
file_tree.traverse_breadth_first(|node| {
    println!("{}", node.path.display());
});
```

**Filtered Search:**
```rust
let rust_files = file_tree.find_nodes(|node| {
    node.metadata.language == Some(Language::Rust)
});
```

### Knowledge Graph Traversal

**Find Dependencies:**
```rust
let dependencies = knowledge_graph.get_neighbors_by_relationship(
    node_id,
    RelationshipType::Calls,
);
```

**Find Path Between Nodes:**
```rust
if let Some(path) = knowledge_graph.find_path(from_id, to_id) {
    for node_id in path {
        let node = knowledge_graph.get_node(node_id).unwrap();
        println!("{}", node.name);
    }
}
```

**Query by Type:**
```rust
let functions = knowledge_graph.get_nodes_by_type(KnowledgeNodeType::Function);
```

## Integration with RAG and Semantic Parsing

The knowledge graph models integrate with existing components:

1. **SemanticParser → KnowledgeGraph:**
   - Parse source files with SemanticParser
   - Convert SemanticNode to KnowledgeNode
   - Build edges from dependency analysis

2. **KnowledgeGraph → RAG:**
   - Convert KnowledgeNode to CodeChunk for embedding
   - Use graph structure for context-aware retrieval
   - Enable semantic search over code entities

3. **FileTree → Parser:**
   - Use FileTree to enumerate files for parsing
   - Track parsing progress with `indexed` flag
   - Store parsing results in knowledge graph

## Serialization

All models implement `Serialize` and `Deserialize` traits for:
- JSON export/import
- Database storage (via SQLite/PostgreSQL)
- Network transmission
- Caching and persistence

## Example Usage

See `/home/user/descartes/descartes/agent-runner/examples/knowledge_graph_example.rs` for a complete example demonstrating:
- Building a file tree
- Creating knowledge graph nodes
- Establishing relationships
- Linking file tree to knowledge graph
- Querying and traversing both structures

## Future Enhancements

Potential future additions:
1. **Incremental Updates:** Efficiently update graphs when files change
2. **Persistence Layer:** SQLite/PostgreSQL storage backends
3. **Graph Algorithms:** PageRank, community detection, centrality metrics
4. **Visualization:** Export to GraphViz, D3.js formats
5. **Query Language:** DSL for complex graph queries
6. **Diff/Merge:** Compare and merge knowledge graphs
7. **Version Tracking:** Track knowledge graph evolution over time
