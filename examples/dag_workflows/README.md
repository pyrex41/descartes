# DAG Workflow Examples

This directory contains example workflows demonstrating various DAG patterns and use cases.

## Overview

Each example showcases a different workflow pattern:

1. **Simple Linear** - Basic sequential workflow
2. **Branching** - Parallel execution with convergence
3. **Complex Multi-Agent** - Realistic multi-agent system
4. **Hierarchical** - Nested workflows with sub-states

## Running Examples

### Prerequisites

```bash
# Ensure Descartes is installed
cd /home/user/descartes

# Build the project
cargo build --release
```

### Execute Examples

```bash
# Example 1: Simple Linear Workflow
cargo run --example 01_simple_linear --manifest-path examples/dag_workflows/Cargo.toml

# Example 2: Branching Workflow
cargo run --example 02_branching_workflow --manifest-path examples/dag_workflows/Cargo.toml

# Example 3: Complex Multi-Agent
cargo run --example 03_complex_multiagent --manifest-path examples/dag_workflows/Cargo.toml

# Example 4: Hierarchical Workflow
cargo run --example 04_hierarchical_workflow --manifest-path examples/dag_workflows/Cargo.toml
```

Alternatively, run from the examples directory:

```bash
cd examples/dag_workflows

cargo run --bin 01_simple_linear
cargo run --bin 02_branching_workflow
cargo run --bin 03_complex_multiagent
cargo run --bin 04_hierarchical_workflow
```

## Example Descriptions

### 1. Simple Linear Workflow

**Pattern:**
```
Start → Process → Finish
```

**Key Concepts:**
- Basic node creation
- Simple dependency edges
- Linear execution flow
- Metadata configuration

**Use Case:** Simple data processing pipeline

**Code:** [`01_simple_linear.rs`](01_simple_linear.rs)

---

### 2. Branching Workflow

**Pattern:**
```
       Start
      /  |  \
     A   B   C
      \  |  /
        End
```

**Key Concepts:**
- Parallel task execution
- Branching and convergence
- Independent task execution
- Result merging

**Use Case:** Parallel data processing with multiple independent tasks

**Code:** [`02_branching_workflow.rs`](02_branching_workflow.rs)

---

### 3. Complex Multi-Agent Workflow

**Pattern:**
```
Ingest → Classify → [Extract Text, Extract Metadata, Analyze Sentiment]
                          ↓
                      Validate → [Success → Store, Failure → Handle Errors]
                                    ↓
                                  Notify
```

**Key Concepts:**
- Multiple specialized agents
- Different edge types (dependency, soft dependency)
- Guards and conditional logic
- Resource dependencies
- Error handling paths
- Retry mechanisms

**Use Case:** Complex document processing pipeline

**Code:** [`03_complex_multiagent.rs`](03_complex_multiagent.rs)

---

### 4. Hierarchical Workflow

**Pattern:**
```
Initialize → Build [Compile → Test → Package]
          → Deploy [Staging, Canary → Production]
          → Verify → [Success → Finalize, Failure → Rollback]
```

**Key Concepts:**
- Parent-child state relationships
- Sub-workflows
- Hierarchical organization
- Composite states
- Multi-environment deployment

**Use Case:** Large-scale application deployment workflow

**Code:** [`04_hierarchical_workflow.rs`](04_hierarchical_workflow.rs)

---

## Output Files

Each example generates a Swarm.toml file in the `output/` directory:

```
output/
├── simple_linear_workflow.toml
├── branching_workflow.toml
├── complex_multiagent.toml
└── hierarchical_workflow.toml
```

These files can be used directly with the Descartes runtime.

## Learning Path

### Beginner

1. Start with `01_simple_linear.rs`
   - Understand basic DAG structure
   - Learn node and edge creation
   - See simple export to Swarm.toml

2. Move to `02_branching_workflow.rs`
   - Learn parallel execution
   - Understand branching and convergence
   - See critical path analysis

### Intermediate

3. Study `03_complex_multiagent.rs`
   - Multiple agent types
   - Conditional logic with guards
   - Error handling patterns
   - Resource management

### Advanced

4. Explore `04_hierarchical_workflow.rs`
   - Hierarchical state composition
   - Complex deployment scenarios
   - Multi-stage workflows
   - Sub-state relationships

## Key Patterns

### Sequential Flow

```rust
dag.add_edge(DAGEdge::dependency(task1_id, task2_id))?;
dag.add_edge(DAGEdge::dependency(task2_id, task3_id))?;
```

### Parallel Execution

```rust
// Branch from start
dag.add_edge(DAGEdge::dependency(start_id, task_a_id))?;
dag.add_edge(DAGEdge::dependency(start_id, task_b_id))?;
dag.add_edge(DAGEdge::dependency(start_id, task_c_id))?;

// Converge to end
dag.add_edge(DAGEdge::dependency(task_a_id, end_id))?;
dag.add_edge(DAGEdge::dependency(task_b_id, end_id))?;
dag.add_edge(DAGEdge::dependency(task_c_id, end_id))?;
```

### Conditional Logic

```rust
let mut success_edge = DAGEdge::dependency(validate_id, success_id);
success_edge.metadata.insert(
    "guards".to_string(),
    serde_json::json!(["validation_passed"])
);
dag.add_edge(success_edge)?;

let mut failure_edge = DAGEdge::dependency(validate_id, failure_id);
failure_edge.metadata.insert(
    "guards".to_string(),
    serde_json::json!(["validation_failed"])
);
dag.add_edge(failure_edge)?;
```

### Hierarchical States

```rust
let parent = DAGNode::new_auto("ParentState")
    .with_metadata("parallel_execution", serde_json::json!(true));

let child = DAGNode::new_auto("ChildState")
    .with_metadata("parent", serde_json::json!("ParentState"));
```

## Common Operations

### Validation

```rust
// Validate DAG structure
dag.validate()?;

// Get statistics
let stats = dag.statistics()?;
println!("Nodes: {}, Edges: {}", stats.node_count, stats.edge_count);
```

### Analysis

```rust
// Get execution order
let order = dag.get_execution_order()?;

// Find critical path
let critical = dag.find_critical_path()?;

// Check connectivity
if dag.is_connected() {
    println!("All nodes are reachable");
}
```

### Export

```rust
let config = SwarmExportConfig::default()
    .with_workflow_name("my_workflow")
    .with_agent("agent1", "claude-3-opus")
    .with_author("Your Name");

let toml = export_dag_to_swarm_toml(&dag, &config)?;
save_dag_as_swarm_toml(&dag, path, &config)?;
```

## Troubleshooting

### "Cycle detected" error
- Check that you haven't created circular dependencies
- Use `dag.detect_cycles()` to find the cycles
- Remove or redirect problematic edges

### "Node not found" error
- Ensure node IDs are stored before creating edges
- Verify nodes are added to DAG before edges

### Export validation fails
- Run `dag.validate()` before export
- Check that all nodes have required metadata
- Verify agent configurations are complete

## Further Reading

- [DAG Reference Guide](../../docs/DAG_REFERENCE.md) - Complete API documentation
- [DAG Editor User Manual](../../docs/phase3/DAG_EDITOR_USER_MANUAL.md) - Visual editor guide
- [Swarm Export Guide](../../docs/phase3/SWARM_EXPORT_QUICKSTART.md) - Export walkthrough
- [Phase 3 Documentation](../../docs/phase3/) - System architecture

## Contributing

To add new examples:

1. Create a new `.rs` file in this directory
2. Follow the naming pattern: `NN_descriptive_name.rs`
3. Include comprehensive comments
4. Update this README with your example
5. Test the example before committing

## Support

For questions or issues:
- Review the documentation links above
- Check existing examples for patterns
- Examine the test files in `/descartes/core/tests/`
- Reach out to the development team

---

**Last Updated:** 2025-11-24
**Examples Version:** 1.0
