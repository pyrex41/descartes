/// Example 2: Branching Workflow
///
/// This example demonstrates parallel execution with branching and convergence.
///
/// Pattern:
///       Start
///      /  |  \
///     A   B   C
///      \  |  /
///        End
///
/// Use case: Parallel data processing with multiple independent tasks

use descartes_core::dag::{DAG, DAGNode, DAGEdge, EdgeType};
use descartes_core::dag_swarm_export::{export_dag_to_swarm_toml, SwarmExportConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut dag = DAG::new("Branching Workflow");
    dag.description = Some("Parallel task execution with convergence".to_string());

    // Create start node
    let start = DAGNode::new_auto("InitializeWorkflow")
        .with_description("Initialize and prepare for parallel execution")
        .with_position(400.0, 100.0)
        .with_metadata("agents", serde_json::json!(["orchestrator"]))
        .with_metadata("entry_actions", serde_json::json!(["setup_context", "allocate_resources"]));

    // Create parallel task nodes
    let task_a = DAGNode::new_auto("ProcessDatasetA")
        .with_description("Process first dataset")
        .with_position(200.0, 300.0)
        .with_metadata("agents", serde_json::json!(["dataset_processor"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["database_a"]))
        .with_tag("parallel")
        .with_tag("dataset");

    let task_b = DAGNode::new_auto("ProcessDatasetB")
        .with_description("Process second dataset")
        .with_position(400.0, 300.0)
        .with_metadata("agents", serde_json::json!(["dataset_processor"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["database_b"]))
        .with_tag("parallel")
        .with_tag("dataset");

    let task_c = DAGNode::new_auto("ProcessDatasetC")
        .with_description("Process third dataset")
        .with_position(600.0, 300.0)
        .with_metadata("agents", serde_json::json!(["dataset_processor"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("required_resources", serde_json::json!(["database_c"]))
        .with_tag("parallel")
        .with_tag("dataset");

    // Create convergence node
    let end = DAGNode::new_auto("MergeResults")
        .with_description("Merge results from all parallel tasks")
        .with_position(400.0, 500.0)
        .with_metadata("agents", serde_json::json!(["result_merger"]))
        .with_metadata("entry_actions", serde_json::json!(["validate_all_inputs", "merge_data"]))
        .with_metadata("exit_actions", serde_json::json!(["save_merged_results", "cleanup"]));

    // Store node IDs
    let start_id = start.node_id;
    let a_id = task_a.node_id;
    let b_id = task_b.node_id;
    let c_id = task_c.node_id;
    let end_id = end.node_id;

    // Add nodes
    dag.add_node(start)?;
    dag.add_node(task_a)?;
    dag.add_node(task_b)?;
    dag.add_node(task_c)?;
    dag.add_node(end)?;

    // Create branching edges (Start -> A, B, C)
    dag.add_edge(DAGEdge::dependency(start_id, a_id).with_label("start_task_a"))?;
    dag.add_edge(DAGEdge::dependency(start_id, b_id).with_label("start_task_b"))?;
    dag.add_edge(DAGEdge::dependency(start_id, c_id).with_label("start_task_c"))?;

    // Create convergence edges (A, B, C -> End)
    dag.add_edge(DAGEdge::dependency(a_id, end_id).with_label("task_a_complete"))?;
    dag.add_edge(DAGEdge::dependency(b_id, end_id).with_label("task_b_complete"))?;
    dag.add_edge(DAGEdge::dependency(c_id, end_id).with_label("task_c_complete"))?;

    // Validate
    dag.validate()?;

    println!("✓ Branching workflow created");
    println!("  Nodes: {}", dag.nodes.len());
    println!("  Edges: {}", dag.edges.len());

    // Analyze structure
    let stats = dag.statistics()?;
    println!("\nWorkflow statistics:");
    println!("  Start nodes: {}", stats.start_nodes);
    println!("  End nodes: {}", stats.end_nodes);
    println!("  Max depth: {}", stats.max_depth);
    println!("  Connected: {}", stats.is_connected);

    // Find critical path
    let critical_path = dag.find_critical_path()?;
    println!("\nCritical path ({} nodes):", critical_path.len());
    for node_id in critical_path {
        let node = dag.get_node(node_id).unwrap();
        println!("  - {}", node.label);
    }

    // Export configuration
    let config = SwarmExportConfig::default()
        .with_workflow_name("branching_workflow")
        .with_description("Parallel execution with branching")
        .with_agent("orchestrator", "claude-3-opus")
        .with_agent("dataset_processor", "claude-3-sonnet")
        .with_agent("result_merger", "claude-3-sonnet")
        .with_author("Parallel Processing Team")
        .with_timeout(3600);

    let toml = export_dag_to_swarm_toml(&dag, &config)?;

    // Save
    use std::path::Path;
    let output_path = Path::new("examples/dag_workflows/output/branching_workflow.toml");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, toml)?;

    println!("\n✓ Workflow exported to: {:?}", output_path);

    Ok(())
}
