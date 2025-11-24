# Knowledge Graph Overlay - Quick Start Guide

## What is Knowledge Graph Overlay?

The Knowledge Graph Overlay system automatically extracts semantic entities (functions, classes, structs, etc.) from your codebase and builds a queryable knowledge graph that is linked to your file tree. This enables powerful semantic code navigation and analysis.

## Installation

The knowledge graph overlay is part of the `agent-runner` crate:

```toml
[dependencies]
agent-runner = { path = "../agent-runner" }
```

## Basic Usage

### 1. Build a File Tree

```rust
use agent_runner::{FileTreeBuilder, KnowledgeGraphOverlay};

// Build file tree from a directory
let mut builder = FileTreeBuilder::new();
let file_tree = builder.scan_directory("./src")?;

println!("Found {} files", file_tree.file_count);
```

### 2. Generate Knowledge Graph

```rust
// Create overlay manager
let mut overlay = KnowledgeGraphOverlay::new()?;

// Generate knowledge graph from file tree
let kg = overlay.generate_knowledge_overlay(&file_tree)?;

println!("Extracted {} entities", kg.nodes.len());
```

### 3. Link to File Tree (Optional but Recommended)

```rust
// Generate and establish bidirectional links
let mut file_tree = builder.scan_directory("./src")?;
let kg = overlay.generate_and_link(&mut file_tree)?;

// Now file tree nodes have knowledge_links
for file in file_tree.get_all_files() {
    if !file.knowledge_links.is_empty() {
        println!("{:?} contains {} entities",
            file.path, file.knowledge_links.len());
    }
}
```

## Query Operations

### Find All Functions

```rust
use agent_runner::KnowledgeNodeType;

let functions = overlay.find_by_type(KnowledgeNodeType::Function, &kg);
for func in functions {
    println!("Function: {}", func.qualified_name);
}
```

### Find Entities in a Specific File

```rust
use std::path::Path;

let file_path = Path::new("src/main.rs");
let entities = overlay.find_entities_in_file(file_path, &kg);

for entity in entities {
    println!("{} at line {}",
        entity.name,
        entity.file_references[0].line_range.0);
}
```

### Find Definition of an Entity

```rust
if let Some(file_ref) = overlay.find_definition("module::function_name", &kg) {
    println!("Defined in: {:?} at line {}",
        file_ref.file_path,
        file_ref.line_range.0);
}
```

### Search by Name Pattern

```rust
let results = overlay.find_by_name_pattern("process", &kg);
for entity in results {
    println!("Found: {} ({})",
        entity.qualified_name,
        entity.content_type.as_str());
}
```

### Find Callers and Callees

```rust
// Find all functions that call this function
let callers = overlay.find_callers("my_function", &kg);

// Find all functions that this function calls
let callees = overlay.find_callees("my_function", &kg);
```

### Traverse Call Graph

```rust
let paths = overlay.traverse_call_graph("main", &kg, 5);
for path in paths {
    println!("Call path: {}", path.join(" -> "));
}
```

## Configuration

### Custom Configuration

```rust
use agent_runner::{OverlayConfig, Language};
use std::time::Duration;

let config = OverlayConfig {
    // Only parse Rust files
    enabled_languages: vec![Language::Rust],

    // Extract relationships between entities
    extract_relationships: true,

    // Skip files larger than 5MB
    max_file_size: Some(5 * 1024 * 1024),

    // Enable caching
    enable_cache: true,
    cache_dir: None,
    cache_ttl: Duration::from_secs(3600), // 1 hour

    // Parse files in parallel
    parallel_parsing: true,
};

let mut overlay = KnowledgeGraphOverlay::with_config(config)?;
```

## Incremental Updates

### Update When a File Changes

```rust
use std::path::Path;

// When a file changes, update the knowledge graph
let changed_file = Path::new("src/main.rs");
overlay.update_file(changed_file, &file_tree, &mut kg)?;

println!("Knowledge graph updated");
```

## Caching

### Cache Management

```rust
// Get cache statistics
let stats = overlay.cache_stats();
println!("Cached {} files with {} entities",
    stats.total_entries,
    stats.total_nodes);

// Clear cache
overlay.clear_cache();
```

## Working with Entities

### Accessing Entity Details

```rust
for (node_id, node) in &kg.nodes {
    println!("\nEntity: {}", node.qualified_name);
    println!("  Type: {:?}", node.content_type);
    println!("  Language: {:?}", node.language);

    // Parameters (for functions)
    if !node.parameters.is_empty() {
        println!("  Parameters: {:?}", node.parameters);
    }

    // Return type
    if let Some(ret) = &node.return_type {
        println!("  Returns: {}", ret);
    }

    // Documentation
    if let Some(doc) = &node.description {
        println!("  Doc: {}", doc);
    }

    // Source code
    if let Some(source) = &node.source_code {
        println!("  Source: {} chars", source.len());
    }

    // File locations
    for file_ref in &node.file_references {
        println!("  Location: {:?} @ lines {:?}",
            file_ref.file_path.file_name(),
            file_ref.line_range);
    }
}
```

### Working with Relationships

```rust
use agent_runner::RelationshipType;

// Get all outgoing relationships from a node
let edges = kg.get_outgoing_edges(node_id);
for edge in edges {
    println!("Relationship: {:?}", edge.relationship_type);
    if let Some(target) = kg.get_node(&edge.to_node_id) {
        println!("  -> {}", target.qualified_name);
    }
}

// Get all incoming relationships to a node
let incoming = kg.get_incoming_edges(node_id);
```

## Entity Types

The system extracts the following entity types:

- `KnowledgeNodeType::Function` - Function definitions
- `KnowledgeNodeType::Method` - Methods (part of a class/struct)
- `KnowledgeNodeType::Class` - Class definitions
- `KnowledgeNodeType::Struct` - Struct definitions
- `KnowledgeNodeType::Enum` - Enum definitions
- `KnowledgeNodeType::Interface` - Interface/Trait definitions
- `KnowledgeNodeType::Module` - Module/Package definitions
- `KnowledgeNodeType::Constant` - Constants
- `KnowledgeNodeType::Variable` - Variables
- `KnowledgeNodeType::TypeAlias` - Type aliases
- `KnowledgeNodeType::Macro` - Macro definitions

## Relationship Types

Detected relationships include:

- `RelationshipType::Calls` - Function calls
- `RelationshipType::Imports` - Module imports
- `RelationshipType::Inherits` - Class inheritance
- `RelationshipType::Implements` - Interface implementation
- `RelationshipType::Uses` - Entity usage/reference
- `RelationshipType::DefinedIn` - Containment relationship
- `RelationshipType::Overrides` - Method overriding
- `RelationshipType::DependsOn` - Dependencies

## Complete Example

```rust
use agent_runner::{
    FileTreeBuilder,
    KnowledgeGraphOverlay,
    KnowledgeNodeType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Build file tree
    let mut builder = FileTreeBuilder::new();
    let mut file_tree = builder.scan_directory("./src")?;

    // 2. Generate knowledge graph with links
    let mut overlay = KnowledgeGraphOverlay::new()?;
    let kg = overlay.generate_and_link(&mut file_tree)?;

    // 3. Query: Find all functions
    let functions = overlay.find_by_type(KnowledgeNodeType::Function, &kg);
    println!("Found {} functions", functions.len());

    // 4. Query: Find entities in main.rs
    let main_path = std::path::Path::new("src/main.rs");
    let entities = overlay.find_entities_in_file(main_path, &kg);
    for entity in entities {
        println!("  - {}", entity.name);
    }

    // 5. Search by pattern
    let results = overlay.find_by_name_pattern("process", &kg);
    println!("\nEntities matching 'process': {}", results.len());

    // 6. Find definition
    if let Some(func) = functions.first() {
        if let Some(def) = overlay.find_definition(&func.qualified_name, &kg) {
            println!("\n{} defined at {:?}:{:?}",
                func.name,
                def.file_path.file_name(),
                def.line_range);
        }
    }

    Ok(())
}
```

## Performance Tips

1. **Enable caching** for repeated analysis of the same codebase
2. **Use parallel parsing** for large codebases
3. **Set file size limits** to skip very large files
4. **Incremental updates** instead of full re-generation
5. **Filter by language** if you only need specific languages

## Common Patterns

### Pattern 1: Code Navigation

```rust
// Find definition and all references of an entity
fn navigate_to_entity(name: &str, overlay: &KnowledgeGraphOverlay, kg: &KnowledgeGraph) {
    // Definition
    if let Some(def) = overlay.find_definition(name, kg) {
        println!("Definition: {:?}", def.file_path);
    }

    // References
    let refs = overlay.find_references(name, kg);
    println!("Found in {} locations", refs.len());
}
```

### Pattern 2: Impact Analysis

```rust
// Find what would be affected if a function changes
fn impact_analysis(func_name: &str, overlay: &KnowledgeGraphOverlay, kg: &KnowledgeGraph) {
    let callers = overlay.find_callers(func_name, kg);
    println!("Functions that would be affected: {}", callers.len());

    for caller in callers {
        println!("  - {}", caller.qualified_name);
    }
}
```

### Pattern 3: Code Quality Metrics

```rust
// Analyze code structure
fn analyze_structure(overlay: &KnowledgeGraphOverlay, kg: &KnowledgeGraph) {
    let functions = overlay.find_by_type(KnowledgeNodeType::Function, kg);
    let classes = overlay.find_by_type(KnowledgeNodeType::Class, kg);

    println!("Functions: {}", functions.len());
    println!("Classes: {}", classes.len());
    println!("Avg edges per node: {:.2}", kg.stats().avg_degree);
}
```

## Troubleshooting

### Issue: No entities extracted

**Cause:** File language not detected or not enabled
**Solution:** Check file extension and enable the language in config

### Issue: Slow parsing

**Cause:** Large codebase or disabled parallel parsing
**Solution:** Enable parallel parsing and set file size limits

### Issue: Cache not working

**Cause:** Cache disabled or files being modified
**Solution:** Enable caching and check file modification times

## Next Steps

- Explore the full example: `examples/knowledge_graph_overlay_example.rs`
- Read the comprehensive tests: `tests/knowledge_graph_overlay_test.rs`
- Check the implementation report: `PHASE3_9.3_IMPLEMENTATION_REPORT.md`

## API Reference

For complete API documentation:
```bash
cargo doc --open
```
