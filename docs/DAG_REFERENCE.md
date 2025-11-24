# DAG Reference Guide

Complete reference documentation for the Descartes DAG (Directed Acyclic Graph) system.

**Version:** 1.0
**Last Updated:** 2025-11-24
**Applies To:** Descartes Phase 3

---

## Table of Contents

1. [Overview](#overview)
2. [Data Model](#data-model)
3. [Core Operations](#core-operations)
4. [Graph Algorithms](#graph-algorithms)
5. [Swarm.toml Export/Import](#swarmtoml-exportimport)
6. [Visual Editor](#visual-editor)
7. [API Reference](#api-reference)
8. [Best Practices](#best-practices)
9. [Error Handling](#error-handling)
10. [Examples](#examples)

---

## Overview

The DAG system in Descartes provides a visual and programmatic way to define task dependencies and workflow orchestration. It serves as the foundation for creating complex multi-agent workflows that can be exported to Swarm.toml format for execution.

### Key Features

- **Visual Graph Editor**: Interactive canvas for building workflows
- **Topological Ordering**: Automatic dependency resolution
- **Cycle Detection**: Prevents invalid circular dependencies
- **Bidirectional Conversion**: DAG ↔ Swarm.toml translation
- **Metadata Support**: Rich annotation and configuration
- **Undo/Redo**: Full history tracking
- **Type-Safe API**: Compile-time guarantees

### When to Use DAGs

- **Complex Workflows**: Multiple tasks with dependencies
- **Visual Design**: When graphical representation aids understanding
- **Team Collaboration**: Visual workflows are easier to discuss
- **Iterative Development**: Build and test workflows incrementally

---

## Data Model

### DAG Structure

```rust
pub struct DAG {
    pub name: String,
    pub description: Option<String>,
    pub nodes: HashMap<Uuid, DAGNode>,
    pub edges: HashMap<Uuid, DAGEdge>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

**Fields:**
- `name`: Human-readable workflow name
- `description`: Optional detailed description
- `nodes`: All nodes indexed by unique ID
- `edges`: All edges indexed by unique ID
- `metadata`: Arbitrary key-value metadata
- `created_at`: Workflow creation timestamp
- `updated_at`: Last modification timestamp

### DAGNode

```rust
pub struct DAGNode {
    pub node_id: Uuid,
    pub task_id: Option<Uuid>,
    pub label: String,
    pub description: Option<String>,
    pub position: Position,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tags: Vec<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

**Fields:**
- `node_id`: Unique identifier for this node
- `task_id`: Optional reference to a Task entity
- `label`: Display name for the node
- `description`: Optional detailed description
- `position`: (x, y) coordinates for visual layout
- `metadata`: Configuration and annotations
- `tags`: Classification labels
- `created_at`: Node creation timestamp
- `updated_at`: Last modification timestamp

**Key Metadata Keys:**
- `agents`: Array of agent names to use
- `entry_actions`: Actions to run on state entry
- `exit_actions`: Actions to run on state exit
- `required_resources`: Resources needed
- `parallel_execution`: Boolean for parallel support
- `timeout_seconds`: State timeout in seconds
- `timeout_target`: State to transition to on timeout
- `parent`: Parent state for hierarchical workflows

### DAGEdge

```rust
pub struct DAGEdge {
    pub edge_id: Uuid,
    pub from_node_id: Uuid,
    pub to_node_id: Uuid,
    pub edge_type: EdgeType,
    pub label: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

**Fields:**
- `edge_id`: Unique identifier for this edge
- `from_node_id`: Source node
- `to_node_id`: Target node
- `edge_type`: Type of dependency relationship
- `label`: Optional event name or description
- `metadata`: Configuration for this edge
- `created_at`: Edge creation timestamp

**Key Metadata Keys:**
- `guards`: Array of guard conditions
- `event`: Custom event name for transition

### EdgeType

```rust
pub enum EdgeType {
    Dependency,           // Hard dependency - target waits for source
    SoftDependency,       // Soft dependency - target can proceed independently
    OptionalDependency,   // Optional - target can reference but doesn't wait
    DataFlow,             // Data passing from source to target
    Trigger,              // Source completion triggers target
    Custom(String),       // User-defined semantics
}
```

**When to Use Each Type:**
- **Dependency**: When task B cannot start until task A completes
- **SoftDependency**: When task B should wait for A but can proceed if needed
- **OptionalDependency**: When task B can use A's output if available
- **DataFlow**: When explicitly passing data between tasks
- **Trigger**: When A's completion should immediately start B
- **Custom**: For domain-specific relationships

### Position

```rust
pub struct Position {
    pub x: f64,
    pub y: f64,
}
```

**Methods:**
- `new(x, y)`: Create new position
- `distance_to(&other)`: Calculate Euclidean distance

---

## Core Operations

### Creating a DAG

```rust
use descartes_core::dag::DAG;

let mut dag = DAG::new("My Workflow");
dag.description = Some("A sample workflow".to_string());
```

### Adding Nodes

```rust
use descartes_core::dag::DAGNode;

// Simple node
let node = DAGNode::new_auto("Task 1");
dag.add_node(node)?;

// Node with configuration
let node = DAGNode::new_auto("Task 2")
    .with_position(300.0, 100.0)
    .with_description("Process data")
    .with_metadata("agents", serde_json::json!(["agent1"]))
    .with_metadata("entry_actions", serde_json::json!(["setup"]))
    .with_tag("critical");

dag.add_node(node)?;
```

### Adding Edges

```rust
use descartes_core::dag::{DAGEdge, EdgeType};

// Simple dependency
let edge = DAGEdge::dependency(node1_id, node2_id);
dag.add_edge(edge)?;

// Edge with label and metadata
let edge = DAGEdge::new(node1_id, node2_id, EdgeType::DataFlow)
    .with_label("on_success")
    .with_metadata("guards", serde_json::json!(["is_ready"]));

dag.add_edge(edge)?;
```

### Removing Nodes and Edges

```rust
// Remove node (also removes connected edges)
dag.remove_node(node_id)?;

// Remove specific edge
dag.remove_edge(edge_id)?;
```

### Querying the Graph

```rust
// Get node information
let node = dag.get_node(node_id).unwrap();
println!("Node: {}", node.label);

// Get connected nodes
let predecessors = dag.get_predecessors(node_id);
let successors = dag.get_successors(node_id);

// Get edges
let incoming = dag.get_incoming_edges(node_id);
let outgoing = dag.get_outgoing_edges(node_id);

// Find start and end nodes
let starts = dag.get_start_nodes();
let ends = dag.get_end_nodes();
```

---

## Graph Algorithms

### Topological Sort

Returns nodes in dependency order (dependencies before dependents).

```rust
let sorted_nodes = dag.topological_sort()?;

for node_id in sorted_nodes {
    let node = dag.get_node(node_id).unwrap();
    println!("Execute: {}", node.label);
}

// Alias for better semantics
let execution_order = dag.get_execution_order()?;
```

**Use Cases:**
- Determining task execution order
- Validating workflow structure
- Finding build/deployment sequences

### Cycle Detection

```rust
// Check if DAG is valid (no cycles)
match dag.validate() {
    Ok(()) => println!("Valid DAG"),
    Err(DAGError::CycleDetected(msg)) => println!("Cycle found: {}", msg),
    Err(e) => println!("Error: {}", e),
}

// Find all cycles
let cycles = dag.detect_cycles();
for cycle in cycles {
    println!("Cycle: {:?}", cycle);
}
```

**Preventing Cycles:**
```rust
// Check before adding edge
if would_create_cycle(&dag, from_id, to_id) {
    println!("Cannot add edge - would create cycle");
} else {
    dag.add_edge(edge)?;
}
```

### Path Finding

```rust
// Check if path exists
if dag.has_path(start_id, end_id) {
    println!("Path exists");
}

// Find all paths
let paths = dag.find_all_paths(start_id, end_id)?;
for path in paths {
    println!("Path: {:?}", path);
}

// Find dependencies (all ancestors)
let deps = dag.find_dependencies(node_id);
println!("Depends on: {:?}", deps);

// Find dependents (all descendants)
let dependents = dag.find_dependents(node_id);
println!("Depended on by: {:?}", dependents);
```

### Critical Path

Find the longest path through the DAG (minimum completion time).

```rust
let critical_path = dag.find_critical_path()?;
println!("Critical path length: {}", critical_path.len());

for node_id in critical_path {
    let node = dag.get_node(node_id).unwrap();
    println!("Critical: {}", node.label);
}
```

### Graph Statistics

```rust
let stats = dag.statistics()?;

println!("Nodes: {}", stats.node_count);
println!("Edges: {}", stats.edge_count);
println!("Start nodes: {}", stats.start_nodes);
println!("End nodes: {}", stats.end_nodes);
println!("Max depth: {}", stats.max_depth);
println!("Average in-degree: {:.2}", stats.average_in_degree);
println!("Average out-degree: {:.2}", stats.average_out_degree);
println!("Is acyclic: {}", stats.is_acyclic);
println!("Is connected: {}", stats.is_connected);
```

### Traversals

```rust
// Breadth-first traversal
dag.bfs_from(start_id, |node_id, depth| {
    let node = dag.get_node(node_id).unwrap();
    println!("Depth {}: {}", depth, node.label);
})?;

// Depth-first traversal
dag.dfs_from(start_id, |node_id, depth| {
    let node = dag.get_node(node_id).unwrap();
    println!("Depth {}: {}", depth, node.label);
})?;
```

### Subgraph Extraction

```rust
// Extract subgraph with specific nodes
let node_ids = vec![id1, id2, id3];
let subgraph = dag.get_subgraph(&node_ids)?;

println!("Subgraph has {} nodes", subgraph.nodes.len());
```

---

## Swarm.toml Export/Import

### Exporting to Swarm.toml

```rust
use descartes_core::dag_swarm_export::{export_dag_to_swarm_toml, SwarmExportConfig};

let config = SwarmExportConfig::default()
    .with_workflow_name("my_workflow")
    .with_description("A sample workflow")
    .with_agent("agent1", "claude-3-opus")
    .with_agent("agent2", "claude-3-sonnet")
    .with_author("Your Name")
    .with_timeout(3600)
    .with_retries(3, 60);

let toml_string = export_dag_to_swarm_toml(&dag, &config)?;
println!("{}", toml_string);
```

### Export Configuration Options

```rust
// Save to file
use std::path::Path;
use descartes_core::dag_swarm_export::save_dag_as_swarm_toml;

let path = Path::new("workflow.toml");
save_dag_as_swarm_toml(&dag, path, &config)?;
```

**Configuration Options:**
- `workflow_name`: Override DAG name
- `workflow_description`: Override DAG description
- `initial_state`: Specify starting state (default: first root node)
- `agents`: Map of agent names to configurations
- `resources`: Map of resource names to configurations
- `guards`: Global guard definitions
- `author`: Workflow author name
- `completion_timeout_seconds`: Global timeout
- `max_retries`: Maximum retry attempts
- `retry_backoff_seconds`: Retry delay
- `use_labels_as_state_names`: Use node labels instead of UUIDs
- `default_event_name`: Default event for transitions
- `include_header`: Include generation comment header

### Importing from Swarm.toml

```rust
use descartes_core::dag_swarm_export::{import_swarm_toml_to_dag, load_dag_from_swarm_toml};

// From string
let swarm_config: SwarmConfig = toml::from_str(&toml_string)?;
let dag = import_swarm_toml_to_dag(&swarm_config, 0)?; // workflow index

// From file
let path = Path::new("workflow.toml");
let dag = load_dag_from_swarm_toml(path, 0)?;
```

### Round-Trip Conversion

```rust
// Export
let toml = export_dag_to_swarm_toml(&original_dag, &config)?;

// Parse
let swarm_config: SwarmConfig = toml::from_str(&toml)?;

// Import
let imported_dag = import_swarm_toml_to_dag(&swarm_config, 0)?;

// Verify
assert_eq!(original_dag.nodes.len(), imported_dag.nodes.len());
assert_eq!(original_dag.edges.len(), imported_dag.edges.len());
```

---

## Visual Editor

### Editor State

The DAG editor maintains several state components:

- **DAG State**: The graph structure being edited
- **Canvas State**: Pan, zoom, and viewport
- **UI State**: Panel visibility and layout
- **Interaction State**: Selection, dragging, and active operations
- **History State**: Undo/redo stack

### Tools

**Select Tool**: Select and move nodes
- Click to select single node
- Ctrl+Click to multi-select
- Drag to move selected nodes
- Drag empty area for box selection

**Add Node Tool**: Add new nodes
- Click anywhere to add a node
- Snap to grid if enabled

**Add Edge Tool**: Create dependencies
- Click source node
- Drag to target node
- Release to create edge
- Cycle detection prevents invalid edges

**Delete Tool**: Remove nodes and edges
- Click node or edge to delete

**Pan Tool**: Move the canvas
- Drag to pan view
- Middle mouse button pans in any tool

### Keyboard Shortcuts

- **Ctrl+Z**: Undo
- **Ctrl+Shift+Z** or **Ctrl+Y**: Redo
- **Ctrl+A**: Select all nodes
- **Delete**: Delete selected nodes/edges
- **Escape**: Cancel current operation
- **Space**: Hold to temporarily enable pan
- **+/-**: Zoom in/out

### Coordinate System

```rust
// Screen coordinates (pixels on canvas)
let screen_point = Point::new(400.0, 300.0);

// World coordinates (logical positions)
let world_point = screen_to_world(screen_point, &canvas_state);

// Back to screen
let back = world_to_screen(world_point, &canvas_state);
```

**Transformation Formula:**
```
world.x = (screen.x - offset.x) / zoom
world.y = (screen.y - offset.y) / zoom

screen.x = world.x * zoom + offset.x
screen.y = world.y * zoom + offset.y
```

### Grid System

The editor uses a 20-pixel grid:

```rust
// Snap position to grid
let snapped = snap_to_grid(Point::new(123.0, 456.0));
// Result: Point { x: 120.0, y: 460.0 }
```

Enable snap-to-grid in the editor:
```rust
state.snap_to_grid = true;
```

---

## API Reference

### DAG Methods

#### Construction
- `new(name)` - Create new DAG
- `add_node(node)` - Add node to graph
- `add_edge(edge)` - Add edge to graph
- `remove_node(id)` - Remove node and connected edges
- `remove_edge(id)` - Remove edge
- `update_node(id, node)` - Replace node data

#### Queries
- `get_node(id)` - Get node by ID
- `get_node_mut(id)` - Get mutable node reference
- `get_edge(id)` - Get edge by ID
- `get_edges_between(from, to)` - Get all edges between nodes
- `get_incoming_edges(id)` - Get edges pointing to node
- `get_outgoing_edges(id)` - Get edges from node
- `get_successors(id)` - Get child nodes
- `get_predecessors(id)` - Get parent nodes
- `get_start_nodes()` - Get root nodes
- `get_end_nodes()` - Get leaf nodes
- `find_roots()` - Alias for get_start_nodes
- `find_leaves()` - Alias for get_end_nodes

#### Analysis
- `validate()` - Check for cycles
- `validate_connectivity()` - Check all nodes reachable
- `validate_acyclic()` - Full validation
- `topological_sort()` - Get dependency order
- `get_execution_order()` - Alias for topological_sort
- `detect_cycles()` - Find all cycles
- `find_dependencies(id)` - Get all ancestors
- `find_dependents(id)` - Get all descendants
- `find_all_paths(from, to)` - Find all paths between nodes
- `has_path(from, to)` - Check if path exists
- `find_critical_path()` - Find longest path
- `max_depth()` - Get maximum depth
- `is_connected()` - Check connectivity
- `statistics()` - Get graph statistics
- `get_subgraph(ids)` - Extract subgraph

#### Traversal
- `bfs_from(start, visitor)` - Breadth-first traversal
- `dfs_from(start, visitor)` - Depth-first traversal

#### Serialization
- `rebuild_adjacency()` - Rebuild after deserialization

### DAGNode Methods

#### Construction
- `new(id, label)` - Create node with specific ID
- `new_auto(label)` - Create node with generated ID
- `with_task_id(id)` - Set task reference
- `with_description(desc)` - Set description
- `with_position(x, y)` - Set position
- `with_metadata(key, value)` - Add metadata
- `with_tag(tag)` - Add tag

#### Modification
- `touch()` - Update timestamp

### DAGEdge Methods

#### Construction
- `new(from, to, type)` - Create edge
- `dependency(from, to)` - Create dependency edge
- `soft_dependency(from, to)` - Create soft dependency
- `with_label(label)` - Set label
- `with_metadata(key, value)` - Add metadata

#### Queries
- `is_hard_dependency()` - Check if hard dependency

### DAGWithHistory Methods

Wraps DAG with undo/redo support:

- `new(name)` - Create with history
- `add_node(node)` - Add node (recorded)
- `remove_node(id)` - Remove node (recorded)
- `update_node(id, node)` - Update node (recorded)
- `add_edge(edge)` - Add edge (recorded)
- `remove_edge(id)` - Remove edge (recorded)
- `undo()` - Undo last operation
- `redo()` - Redo last undone operation
- `can_undo()` - Check if undo available
- `can_redo()` - Check if redo available
- `clear_history()` - Clear undo/redo stacks

---

## Best Practices

### 1. Node Design

**DO:**
- Use descriptive, action-oriented labels
- Add descriptions for complex nodes
- Use metadata to store configuration
- Tag nodes for classification

**DON'T:**
- Create nodes with duplicate labels (unless intentional)
- Leave nodes without descriptions in complex workflows
- Store large data in metadata (reference by ID instead)

### 2. Edge Design

**DO:**
- Use appropriate edge types for semantics
- Add guards to edges for conditional logic
- Label edges when multiple edges exist between nodes
- Document custom edge types

**DON'T:**
- Create cycles (DAG validation will catch this)
- Use edges for non-dependency relationships
- Overuse custom edge types

### 3. Workflow Structure

**DO:**
- Keep workflows focused and modular
- Use clear start and end nodes
- Minimize branching complexity
- Document complex decision points

**DON'T:**
- Create disconnected components
- Build overly deep hierarchies (>10 levels)
- Mix different abstraction levels in one DAG

### 4. Metadata Usage

**DO:**
- Use consistent key names across nodes
- Validate metadata schemas
- Document metadata contracts
- Use typed metadata access

**DON'T:**
- Store sensitive data in metadata
- Use metadata for transient state
- Rely on undocumented metadata keys

### 5. Performance

**DO:**
- Use subgraphs for large workflows
- Cache topological sort results when stable
- Batch node/edge operations
- Use appropriate data structures for queries

**DON'T:**
- Repeatedly call topological_sort unnecessarily
- Perform deep traversals on every update
- Create excessive temporary DAGs

### 6. Error Handling

**DO:**
- Handle all DAGError variants
- Provide user-friendly error messages
- Validate before major operations
- Use Result types consistently

**DON'T:**
- Unwrap without checking
- Ignore validation errors
- Suppress cycle detection warnings

---

## Error Handling

### Error Types

```rust
pub enum DAGError {
    CycleDetected(String),
    NodeNotFound(Uuid),
    EdgeNotFound(Uuid),
    DuplicateNode(Uuid),
    DuplicateEdge(Uuid),
    InvalidEdge(Uuid, Uuid),
    SelfLoop(Uuid),
    ValidationError(String),
    SerializationError(String),
    DeserializationError(String),
    NoStartNodes,
    UnreachableNodes(Vec<Uuid>),
}
```

### Handling Patterns

```rust
use descartes_core::dag::DAGError;

match dag.add_edge(edge) {
    Ok(()) => println!("Edge added"),
    Err(DAGError::CycleDetected(msg)) => {
        eprintln!("Cannot add edge: would create cycle: {}", msg);
    }
    Err(DAGError::NodeNotFound(id)) => {
        eprintln!("Node {} not found", id);
    }
    Err(DAGError::SelfLoop(id)) => {
        eprintln!("Cannot create self-loop on node {}", id);
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Validation Strategy

```rust
// Validate before export
if let Err(e) = dag.validate_acyclic() {
    eprintln!("DAG validation failed: {}", e);
    return;
}

// Validate after import
let dag = import_swarm_toml_to_dag(&config, 0)?;
dag.validate()?;
dag.validate_connectivity()?;
```

---

## Examples

### Example 1: Simple Linear Workflow

```rust
use descartes_core::dag::{DAG, DAGNode, DAGEdge, EdgeType};

let mut dag = DAG::new("Linear Workflow");

let start = DAGNode::new_auto("Start")
    .with_description("Initialize")
    .with_metadata("agents", serde_json::json!(["init_agent"]));

let process = DAGNode::new_auto("Process")
    .with_description("Process data")
    .with_metadata("agents", serde_json::json!(["worker_agent"]));

let finish = DAGNode::new_auto("Finish")
    .with_description("Finalize")
    .with_metadata("agents", serde_json::json!(["cleanup_agent"]));

let start_id = start.node_id;
let process_id = process.node_id;
let finish_id = finish.node_id;

dag.add_node(start)?;
dag.add_node(process)?;
dag.add_node(finish)?;

dag.add_edge(DAGEdge::dependency(start_id, process_id))?;
dag.add_edge(DAGEdge::dependency(process_id, finish_id))?;

// Validate and get execution order
dag.validate()?;
let order = dag.get_execution_order()?;
println!("Execution order: {:?}", order);
```

### Example 2: Branching Workflow

```rust
let mut dag = DAG::new("Branching Workflow");

//       Start
//      /  |  \
//     A   B   C
//      \  |  /
//        End

let start = DAGNode::new_auto("Start");
let task_a = DAGNode::new_auto("TaskA");
let task_b = DAGNode::new_auto("TaskB");
let task_c = DAGNode::new_auto("TaskC");
let end = DAGNode::new_auto("End");

let start_id = start.node_id;
let a_id = task_a.node_id;
let b_id = task_b.node_id;
let c_id = task_c.node_id;
let end_id = end.node_id;

dag.add_node(start)?;
dag.add_node(task_a)?;
dag.add_node(task_b)?;
dag.add_node(task_c)?;
dag.add_node(end)?;

// Start branches to A, B, C
dag.add_edge(DAGEdge::dependency(start_id, a_id))?;
dag.add_edge(DAGEdge::dependency(start_id, b_id))?;
dag.add_edge(DAGEdge::dependency(start_id, c_id))?;

// A, B, C converge to End
dag.add_edge(DAGEdge::dependency(a_id, end_id))?;
dag.add_edge(DAGEdge::dependency(b_id, end_id))?;
dag.add_edge(DAGEdge::dependency(c_id, end_id))?;

// Find critical path
let critical = dag.find_critical_path()?;
println!("Critical path: {:?}", critical);
```

### Example 3: Export to Swarm.toml

```rust
use descartes_core::dag_swarm_export::*;

let config = SwarmExportConfig::default()
    .with_workflow_name("production_workflow")
    .with_description("Production data processing")
    .with_agent("data_loader", "claude-3-opus")
    .with_agent("data_processor", "claude-3-sonnet")
    .with_agent("data_validator", "claude-3-haiku")
    .with_resource("database", ResourceConfig {
        resource_type: "PostgreSQL".to_string(),
        connection_string: Some("postgresql://localhost/db".to_string()),
        config: HashMap::new(),
    })
    .with_guard("data_valid", "context.data.is_valid")
    .with_author("Data Team")
    .with_timeout(7200)
    .with_retries(5, 120);

let toml = export_dag_to_swarm_toml(&dag, &config)?;

// Save to file
use std::path::Path;
let path = Path::new("production_workflow.toml");
save_dag_as_swarm_toml(&dag, path, &config)?;
```

### Example 4: History and Undo/Redo

```rust
use descartes_core::dag::DAGWithHistory;

let mut dag = DAGWithHistory::new("Workflow with History");

// Add nodes
let node1 = DAGNode::new_auto("Task 1");
let node2 = DAGNode::new_auto("Task 2");
let id1 = node1.node_id;
let id2 = node2.node_id;

dag.add_node(node1)?;
dag.add_node(node2)?;

// Add edge
let edge = DAGEdge::dependency(id1, id2);
dag.add_edge(edge)?;

// Undo edge addition
dag.undo()?;
assert_eq!(dag.dag.edges.len(), 0);

// Redo
dag.redo()?;
assert_eq!(dag.dag.edges.len(), 1);

// Clear history
dag.clear_history();
```

---

## Further Reading

- [DAG Editor User Manual](phase3/DAG_EDITOR_USER_MANUAL.md) - Visual editor guide
- [Swarm Export Quickstart](phase3/SWARM_EXPORT_QUICKSTART.md) - Quick export guide
- [Phase 3 Overview](phase3/) - Parallel execution documentation
- [State Machine README](STATE_MACHINE_README.md) - State machine integration

---

## Support

For questions or issues:
1. Check the examples in `/examples/dag_workflows/`
2. Review test files in `/descartes/core/tests/`
3. Consult the API documentation
4. Reach out to the development team

**Last Updated:** 2025-11-24
**Version:** 1.0
