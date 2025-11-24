# DAG (Directed Acyclic Graph) Quick Reference

## Quick Start

### Import

```rust
use descartes_core::dag::{DAG, DAGNode, DAGEdge, EdgeType};
use uuid::Uuid;
```

### Create a DAG

```rust
let mut dag = DAG::new("My Workflow");
dag.description = Some("Task dependency graph".to_string());
```

### Add Nodes

```rust
// Auto-generate UUID
let node = DAGNode::new_auto("Task Name")
    .with_description("What this task does")
    .with_position(100.0, 200.0)
    .with_metadata("priority", "high")
    .with_tag("important");

dag.add_node(node)?;

// Or with specific UUID
let node_id = Uuid::new_v4();
let node = DAGNode::new(node_id, "Specific Task");
dag.add_node(node)?;
```

### Add Edges (Dependencies)

```rust
// Simple dependency
let edge = DAGEdge::dependency(from_node_id, to_node_id);
dag.add_edge(edge)?;

// With metadata
let edge = DAGEdge::dependency(from_id, to_id)
    .with_label("needs_approval")
    .with_metadata("approvers", 2);
dag.add_edge(edge)?;
```

### Validate and Sort

```rust
// Check for cycles
dag.validate()?;

// Get execution order
let sorted = dag.topological_sort()?;
for node_id in sorted {
    let node = dag.get_node(node_id).unwrap();
    println!("Execute: {}", node.label);
}
```

## Common Operations

### Query Nodes

```rust
// Get a node
let node = dag.get_node(node_id)?;

// Get all start nodes (no dependencies)
let starts = dag.get_start_nodes();

// Get all end nodes (no dependents)
let ends = dag.get_end_nodes();

// Get successors (tasks that depend on this)
let successors = dag.get_successors(node_id);

// Get predecessors (tasks this depends on)
let predecessors = dag.get_predecessors(node_id);
```

### Query Edges

```rust
// Get outgoing edges
let outgoing = dag.get_outgoing_edges(node_id);

// Get incoming edges
let incoming = dag.get_incoming_edges(node_id);
```

### Graph Analysis

```rust
// Check if path exists
if dag.has_path(start_id, end_id) {
    println!("Path exists!");
}

// Find all paths
let paths = dag.find_all_paths(start_id, end_id)?;

// Get maximum depth
let depth = dag.max_depth();

// Get statistics
let stats = dag.statistics()?;
println!("Nodes: {}, Edges: {}", stats.node_count, stats.edge_count);
```

### Traversal

```rust
// Breadth-first
dag.bfs_from(start_id, |node_id, depth| {
    println!("Depth {}: {:?}", depth, node_id);
})?;

// Depth-first
dag.dfs_from(start_id, |node_id, depth| {
    println!("Depth {}: {:?}", depth, node_id);
})?;
```

## Edge Types

```rust
EdgeType::Dependency         // Hard dependency (default)
EdgeType::SoftDependency     // Can start independently
EdgeType::OptionalDependency // Can reference output
EdgeType::DataFlow           // Data passing
EdgeType::Trigger            // Triggered execution
EdgeType::Custom("type")     // User-defined
```

## TOML Serialization

### Save to File

```rust
use descartes_core::dag_toml::save_dag_to_toml;

save_dag_to_toml(&dag, Path::new("workflow.toml"))?;
```

### Load from File

```rust
use descartes_core::dag_toml::load_dag_from_toml;

let dag = load_dag_from_toml(Path::new("workflow.toml"))?;
```

### Manual Conversion

```rust
use descartes_core::dag_toml::TomlDAG;

// To TOML
let toml_dag = TomlDAG::from_dag(&dag);
let toml_string = toml_dag.to_toml_string()?;

// From TOML
let toml_dag = TomlDAG::from_toml_str(&toml_string)?;
let dag = toml_dag.to_dag()?;
```

## TOML Format

### Full Format

```toml
[dag]
name = "My Workflow"
description = "Task dependencies"

[[dag.nodes]]
node_id = "uuid-here"
label = "Task Name"
position = { x = 100.0, y = 200.0 }
tags = ["tag1", "tag2"]

[dag.nodes.metadata]
priority = "high"
estimated_hours = 8

[[dag.edges]]
from = "uuid1"
to = "uuid2"
edge_type = "dependency"
label = "requires"
```

### Simplified Format

```toml
[dag]
name = "My Workflow"

[[dag.dependencies]]
task = "task_uuid"
depends_on = ["dep1_uuid", "dep2_uuid"]
dependency_type = "dependency"
```

## Error Handling

```rust
use descartes_core::dag::DAGError;

match dag.add_edge(edge) {
    Ok(_) => println!("Edge added"),
    Err(DAGError::CycleDetected(msg)) => {
        eprintln!("Cycle: {}", msg);
    }
    Err(DAGError::NodeNotFound(id)) => {
        eprintln!("Node {} not found", id);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

## Common Patterns

### Linear Workflow

```rust
let mut dag = DAG::new("Linear");
let task1 = DAGNode::new_auto("Step 1");
let task2 = DAGNode::new_auto("Step 2");
let task3 = DAGNode::new_auto("Step 3");

let id1 = task1.node_id;
let id2 = task2.node_id;
let id3 = task3.node_id;

dag.add_node(task1)?;
dag.add_node(task2)?;
dag.add_node(task3)?;

dag.add_edge(DAGEdge::dependency(id1, id2))?;
dag.add_edge(DAGEdge::dependency(id2, id3))?;
```

### Parallel Tasks

```rust
let mut dag = DAG::new("Parallel");
let start = DAGNode::new_auto("Start");
let task_a = DAGNode::new_auto("Parallel A");
let task_b = DAGNode::new_auto("Parallel B");
let end = DAGNode::new_auto("End");

let s = start.node_id;
let a = task_a.node_id;
let b = task_b.node_id;
let e = end.node_id;

dag.add_node(start)?;
dag.add_node(task_a)?;
dag.add_node(task_b)?;
dag.add_node(end)?;

// Fork
dag.add_edge(DAGEdge::dependency(s, a))?;
dag.add_edge(DAGEdge::dependency(s, b))?;

// Join
dag.add_edge(DAGEdge::dependency(a, e))?;
dag.add_edge(DAGEdge::dependency(b, e))?;
```

### Diamond Pattern

```rust
let mut dag = DAG::new("Diamond");
let start = DAGNode::new_auto("Start");
let left = DAGNode::new_auto("Left Path");
let right = DAGNode::new_auto("Right Path");
let end = DAGNode::new_auto("End");

let s = start.node_id;
let l = left.node_id;
let r = right.node_id;
let e = end.node_id;

dag.add_node(start)?;
dag.add_node(left)?;
dag.add_node(right)?;
dag.add_node(end)?;

dag.add_edge(DAGEdge::dependency(s, l))?;
dag.add_edge(DAGEdge::dependency(s, r))?;
dag.add_edge(DAGEdge::dependency(l, e))?;
dag.add_edge(DAGEdge::dependency(r, e))?;
```

## Tips

1. **Always validate** before executing: `dag.validate()?`
2. **Use builder pattern** for cleaner code
3. **Store node IDs** before adding to DAG (they're moved)
4. **Check for cycles** after adding each edge if building dynamically
5. **Use metadata** for domain-specific properties
6. **Tags** are great for filtering and categorization
7. **Position** enables visual editor integration

## Performance

- Node/Edge lookup: O(1)
- Add/Remove: O(1) for nodes, O(E) for node removal with edges
- Topological sort: O(V + E)
- Cycle detection: O(V + E)
- Path finding: O(V + E)

## See Also

- Full documentation: `/docs/phase3/8.1-DAG-Implementation-Report.md`
- Examples: `/descartes/core/examples/dag_usage.rs`
- TOML examples: `/descartes/examples/dag_example.toml`
