# Swarm.toml Export - Quick Start Guide

## Overview

Convert visual DAG representations to executable Swarm.toml workflow configurations in just a few lines of code.

## Quick Examples

### 1. Basic Export

```rust
use descartes_core::dag::{DAG, DAGNode, DAGEdge};
use descartes_core::dag_swarm_export::{export_dag_to_swarm_toml, SwarmExportConfig};

// Create your DAG
let mut dag = DAG::new("My Workflow");

// Add nodes
let start = DAGNode::new_auto("Start")
    .with_description("Initial state")
    .with_metadata("agents", serde_json::json!(["my_agent"]));

let end = DAGNode::new_auto("End")
    .with_description("Final state");

let start_id = start.node_id;
let end_id = end.node_id;

dag.add_node(start).unwrap();
dag.add_node(end).unwrap();

// Add edge
let mut edge = DAGEdge::dependency(start_id, end_id);
edge.label = Some("proceed".to_string());
dag.add_edge(edge).unwrap();

// Configure and export
let config = SwarmExportConfig::default()
    .with_agent("my_agent", "claude-3-sonnet");

let swarm_toml = export_dag_to_swarm_toml(&dag, &config).unwrap();
println!("{}", swarm_toml);
```

### 2. Save to File

```rust
use descartes_core::dag_swarm_export::save_dag_as_swarm_toml;
use std::path::Path;

let config = SwarmExportConfig::default()
    .with_workflow_name("my_workflow")
    .with_agent("default", "claude-3-haiku");

save_dag_as_swarm_toml(&dag, Path::new("workflow.toml"), &config).unwrap();
```

### 3. Load from File

```rust
use descartes_core::dag_swarm_export::load_dag_from_swarm_toml;

let dag = load_dag_from_swarm_toml(Path::new("workflow.toml"), 0).unwrap();
println!("Loaded {} nodes", dag.nodes.len());
```

## Configuration Options

### Add Agents

```rust
config.with_agent("architect", "claude-3-opus")
      .with_agent("developer", "claude-3-sonnet")
      .with_agent("tester", "claude-3-haiku")
```

### Add Custom Agent Config

```rust
use descartes_core::swarm_parser::AgentConfig;

config.with_agent_config("custom", AgentConfig {
    model: "claude-3-opus".to_string(),
    max_tokens: Some(8000),
    temperature: Some(0.9),
    tags: vec!["creative".to_string()],
})
```

### Add Resources

```rust
use descartes_core::swarm_parser::ResourceConfig;

config.with_resource("api", ResourceConfig::Http {
    endpoint: "https://api.example.com".to_string(),
    auth_required: Some(true),
    secret_key: Some("API_KEY".to_string()),
})
```

### Add Guards

```rust
config.with_guard("approved", "context.approval_count >= 2")
      .with_guard("no_errors", "context.errors.is_empty()")
```

### Set Timeouts and Retries

```rust
config.with_timeout(3600)           // 1 hour timeout
      .with_retries(3, 60)          // 3 retries, 60s backoff
```

## Node Metadata Format

Nodes can store Swarm-specific metadata:

```rust
let node = DAGNode::new_auto("MyState")
    .with_description("State description")

    // Assign agents
    .with_metadata("agents", serde_json::json!(["agent1", "agent2"]))

    // Entry actions
    .with_metadata("entry_actions", serde_json::json!([
        "initialize",
        "validate_input"
    ]))

    // Exit actions
    .with_metadata("exit_actions", serde_json::json!([
        "cleanup",
        "save_state"
    ]))

    // Required resources
    .with_metadata("required_resources", serde_json::json!([
        "database",
        "api_service"
    ]))

    // Parent state (for hierarchical workflows)
    .with_metadata("parent", "ParentState")

    // Parallel execution
    .with_metadata("parallel_execution", true)

    // Timeout configuration
    .with_metadata("timeout_seconds", 300)
    .with_metadata("timeout_target", "TimeoutState");
```

## Edge Metadata Format

Edges can specify event names and guards:

```rust
let mut edge = DAGEdge::dependency(from_id, to_id);

// Set event name
edge.label = Some("user_approved".to_string());

// Add guards
edge.metadata.insert(
    "guards".to_string(),
    serde_json::json!(["has_permissions", "quota_available"])
);

// Alternative: set event in metadata
edge.metadata.insert(
    "event".to_string(),
    serde_json::json!("custom_event")
);
```

## Common Patterns

### Pattern 1: Approval Workflow

```rust
let pending = DAGNode::new_auto("Pending")
    .with_metadata("agents", serde_json::json!(["approver"]))
    .with_metadata("entry_actions", serde_json::json!(["notify_approver"]));

let approved = DAGNode::new_auto("Approved")
    .with_metadata("entry_actions", serde_json::json!(["execute_task"]));

let rejected = DAGNode::new_auto("Rejected")
    .with_metadata("entry_actions", serde_json::json!(["notify_requester"]));

// Add approve transition
let mut approve = DAGEdge::dependency(pending.node_id, approved.node_id);
approve.label = Some("approve".to_string());

// Add reject transition
let mut reject = DAGEdge::dependency(pending.node_id, rejected.node_id);
reject.label = Some("reject".to_string());
```

### Pattern 2: Parallel Processing

```rust
let review = DAGNode::new_auto("ParallelReview")
    .with_metadata("agents", serde_json::json!([
        "security_reviewer",
        "code_reviewer",
        "performance_reviewer"
    ]))
    .with_metadata("parallel_execution", true)
    .with_metadata("entry_actions", serde_json::json!([
        "distribute_work",
        "start_timers"
    ]));
```

### Pattern 3: Hierarchical States

```rust
// Parent state
let in_progress = DAGNode::new_auto("InProgress")
    .with_metadata("entry_actions", serde_json::json!(["allocate_resources"]))
    .with_metadata("exit_actions", serde_json::json!(["cleanup_resources"]));

// Child states
let planning = DAGNode::new_auto("Planning")
    .with_metadata("parent", "InProgress")
    .with_metadata("agents", serde_json::json!(["planner"]));

let executing = DAGNode::new_auto("Executing")
    .with_metadata("parent", "InProgress")
    .with_metadata("agents", serde_json::json!(["executor"]));
```

### Pattern 4: Timeout Handling

```rust
let processing = DAGNode::new_auto("Processing")
    .with_metadata("timeout_seconds", 300)
    .with_metadata("timeout_target", "TimedOut");

let timed_out = DAGNode::new_auto("TimedOut")
    .with_metadata("entry_actions", serde_json::json!(["log_timeout", "notify_admin"]));
```

## Validation Checklist

Before exporting, ensure:

- [ ] DAG has no cycles
- [ ] All nodes are reachable from start nodes
- [ ] All edges reference existing nodes
- [ ] At least one agent configured
- [ ] Event names are unique per state
- [ ] Timeout targets exist
- [ ] Parent states exist (for hierarchical)

Run validation:
```rust
dag.validate().unwrap();
dag.validate_connectivity().unwrap();
```

## Troubleshooting

### Error: "DAG has no start nodes"

**Cause:** All nodes have incoming edges.

**Solution:** Ensure at least one node has no incoming edges:
```rust
// This node will be a start node (no incoming edges)
let start = DAGNode::new_auto("Start");
```

### Error: "Cycle detected"

**Cause:** DAG contains circular dependencies.

**Solution:** Remove cycles:
```rust
// Use DAG methods to find cycles
let cycles = dag.detect_cycles();
for cycle in cycles {
    println!("Cycle: {:?}", cycle);
}
```

### Error: "Node not found"

**Cause:** Edge references non-existent node.

**Solution:** Add all nodes before edges:
```rust
// 1. Add all nodes first
dag.add_node(node1).unwrap();
dag.add_node(node2).unwrap();

// 2. Then add edges
dag.add_edge(edge).unwrap();
```

### Warning: State name sanitization

**Issue:** State names with special characters get sanitized.

**Examples:**
- `"Task #1"` → `"Task__1"`
- `"Review-Code"` → `"Review_Code"`

**Solutions:**
1. Use alphanumeric names with underscores
2. Or use UUID-based naming:
```rust
config.use_labels(false);  // Use UUIDs instead
```

## Best Practices

1. **Always validate before export:**
   ```rust
   dag.validate()?;
   dag.validate_connectivity()?;
   ```

2. **Use descriptive state names:**
   ```rust
   DAGNode::new_auto("WaitingForApproval")  // Good
   DAGNode::new_auto("State1")              // Bad
   ```

3. **Add descriptions to all nodes:**
   ```rust
   node.with_description("Human-readable description")
   ```

4. **Configure appropriate agents:**
   ```rust
   // Match agent to task complexity
   config.with_agent("simple_task", "claude-3-haiku")
         .with_agent("complex_analysis", "claude-3-opus")
   ```

5. **Use guards for conditional transitions:**
   ```rust
   edge.metadata.insert("guards", serde_json::json!(["is_valid"]));
   config.with_guard("is_valid", "context.validation_passed");
   ```

6. **Set reasonable timeouts:**
   ```rust
   node.with_metadata("timeout_seconds", 300)  // 5 minutes
   ```

7. **Always specify a timeout target:**
   ```rust
   node.with_metadata("timeout_seconds", 300)
       .with_metadata("timeout_target", "HandleTimeout")
   ```

## Advanced Usage

### Customize Event Names

```rust
config.default_event_name = "proceed".to_string();  // Default: "next"
```

### Control Header Comments

```rust
config.include_header = false;  // Skip generation comments
```

### Set Workflow Metadata

```rust
config.with_workflow_name("production_deployment")
      .with_description("Automated deployment pipeline")
      .with_author("DevOps Team")
      .with_initial_state("ValidateCode");
```

### Multiple Workflows

Export different workflows:
```rust
// Export workflow A
let config_a = SwarmExportConfig::default()
    .with_workflow_name("workflow_a");
let toml_a = export_dag_to_swarm_toml(&dag_a, &config_a)?;

// Export workflow B
let config_b = SwarmExportConfig::default()
    .with_workflow_name("workflow_b");
let toml_b = export_dag_to_swarm_toml(&dag_b, &config_b)?;

// Combine manually or save separately
```

## Complete Example

```rust
use descartes_core::dag::{DAG, DAGNode, DAGEdge};
use descartes_core::dag_swarm_export::{
    export_dag_to_swarm_toml, SwarmExportConfig,
};
use descartes_core::swarm_parser::{AgentConfig, ResourceConfig};

fn create_review_workflow() -> Result<String, Box<dyn std::error::Error>> {
    // 1. Create DAG
    let mut dag = DAG::new("Code Review Workflow");
    dag.description = Some("Automated code review process".to_string());

    // 2. Create states
    let submitted = DAGNode::new_auto("Submitted")
        .with_description("Code submitted for review")
        .with_metadata("entry_actions", serde_json::json!(["validate_submission"]))
        .with_position(100.0, 100.0);

    let reviewing = DAGNode::new_auto("Reviewing")
        .with_description("Under review by agents")
        .with_metadata("agents", serde_json::json!(["security", "quality"]))
        .with_metadata("parallel_execution", true)
        .with_metadata("timeout_seconds", 600)
        .with_metadata("timeout_target", "ReviewTimeout")
        .with_position(300.0, 100.0);

    let approved = DAGNode::new_auto("Approved")
        .with_description("Review passed")
        .with_metadata("entry_actions", serde_json::json!(["merge_code"]))
        .with_position(500.0, 50.0);

    let rejected = DAGNode::new_auto("Rejected")
        .with_description("Changes requested")
        .with_metadata("entry_actions", serde_json::json!(["notify_author"]))
        .with_position(500.0, 150.0);

    let timeout = DAGNode::new_auto("ReviewTimeout")
        .with_description("Review timed out")
        .with_metadata("entry_actions", serde_json::json!(["escalate"]))
        .with_position(500.0, 250.0);

    let submitted_id = submitted.node_id;
    let reviewing_id = reviewing.node_id;
    let approved_id = approved.node_id;
    let rejected_id = rejected.node_id;

    // 3. Add nodes
    dag.add_node(submitted)?;
    dag.add_node(reviewing)?;
    dag.add_node(approved)?;
    dag.add_node(rejected)?;
    dag.add_node(timeout)?;

    // 4. Add transitions
    let mut start_review = DAGEdge::dependency(submitted_id, reviewing_id);
    start_review.label = Some("start_review".to_string());
    dag.add_edge(start_review)?;

    let mut approve = DAGEdge::dependency(reviewing_id, approved_id);
    approve.label = Some("approve".to_string());
    approve.metadata.insert(
        "guards".to_string(),
        serde_json::json!(["all_checks_pass"]),
    );
    dag.add_edge(approve)?;

    let mut reject = DAGEdge::dependency(reviewing_id, rejected_id);
    reject.label = Some("request_changes".to_string());
    dag.add_edge(reject)?;

    // 5. Validate
    dag.validate()?;
    dag.validate_connectivity()?;

    // 6. Configure export
    let config = SwarmExportConfig::default()
        .with_workflow_name("code_review")
        .with_description("Automated code review process")
        .with_author("Platform Team")
        .with_timeout(1800)
        .with_retries(2, 30)
        .with_agent_config(
            "security",
            AgentConfig {
                model: "claude-3-opus".to_string(),
                max_tokens: Some(4000),
                temperature: Some(0.5),
                tags: vec!["security".to_string()],
            },
        )
        .with_agent_config(
            "quality",
            AgentConfig {
                model: "claude-3-sonnet".to_string(),
                max_tokens: Some(3000),
                temperature: Some(0.5),
                tags: vec!["quality".to_string()],
            },
        )
        .with_guard("all_checks_pass", "context.issues.is_empty()");

    // 7. Export
    let swarm_toml = export_dag_to_swarm_toml(&dag, &config)?;

    Ok(swarm_toml)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let workflow = create_review_workflow()?;
    println!("{}", workflow);

    // Save to file
    std::fs::write("code_review.toml", workflow)?;
    println!("\nSaved to code_review.toml");

    Ok(())
}
```

## Next Steps

- Review [Full Implementation Report](8.5-Swarm-TOML-Export-Report.md)
- Explore [Example Workflows](/descartes/examples/swarm_toml/)
- Check [DAG Documentation](../../descartes/core/src/dag.rs)
- Run [Demo Examples](../../descartes/core/examples/swarm_export_demo.rs)
