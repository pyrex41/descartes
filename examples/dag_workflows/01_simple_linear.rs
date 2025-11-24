/// Example 1: Simple Linear Workflow
///
/// This example demonstrates the most basic workflow pattern:
/// a linear sequence of tasks with simple dependencies.
///
/// Pattern: Start -> Process -> Finish
///
/// Use case: Simple data processing pipeline

use descartes_core::dag::{DAG, DAGNode, DAGEdge, EdgeType};
use descartes_core::dag_swarm_export::{export_dag_to_swarm_toml, SwarmExportConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create DAG
    let mut dag = DAG::new("Simple Linear Workflow");
    dag.description = Some("A basic linear workflow for data processing".to_string());

    // Create nodes
    let start = DAGNode::new_auto("LoadData")
        .with_description("Load data from source")
        .with_position(100.0, 200.0)
        .with_metadata("agents", serde_json::json!(["data_loader"]))
        .with_metadata("entry_actions", serde_json::json!(["connect_to_source"]))
        .with_metadata("exit_actions", serde_json::json!(["close_connection"]));

    let process = DAGNode::new_auto("ProcessData")
        .with_description("Process and transform data")
        .with_position(400.0, 200.0)
        .with_metadata("agents", serde_json::json!(["data_processor"]))
        .with_metadata("entry_actions", serde_json::json!(["validate_input"]))
        .with_metadata("required_resources", serde_json::json!(["compute_cluster"]));

    let finish = DAGNode::new_auto("SaveResults")
        .with_description("Save processed results")
        .with_position(700.0, 200.0)
        .with_metadata("agents", serde_json::json!(["data_saver"]))
        .with_metadata("entry_actions", serde_json::json!(["prepare_storage"]))
        .with_metadata("exit_actions", serde_json::json!(["verify_save", "cleanup"]));

    // Store node IDs
    let start_id = start.node_id;
    let process_id = process.node_id;
    let finish_id = finish.node_id;

    // Add nodes to DAG
    dag.add_node(start)?;
    dag.add_node(process)?;
    dag.add_node(finish)?;

    // Create edges
    let edge1 = DAGEdge::dependency(start_id, process_id)
        .with_label("data_loaded");

    let edge2 = DAGEdge::dependency(process_id, finish_id)
        .with_label("processing_complete");

    dag.add_edge(edge1)?;
    dag.add_edge(edge2)?;

    // Validate DAG
    dag.validate()?;

    println!("✓ DAG created successfully");
    println!("  Nodes: {}", dag.nodes.len());
    println!("  Edges: {}", dag.edges.len());

    // Get execution order
    let execution_order = dag.get_execution_order()?;
    println!("\nExecution order:");
    for (i, node_id) in execution_order.iter().enumerate() {
        let node = dag.get_node(*node_id).unwrap();
        println!("  {}. {}", i + 1, node.label);
    }

    // Export to Swarm.toml
    let config = SwarmExportConfig::default()
        .with_workflow_name("simple_linear_workflow")
        .with_description("Linear data processing workflow")
        .with_agent("data_loader", "claude-3-opus")
        .with_agent("data_processor", "claude-3-sonnet")
        .with_agent("data_saver", "claude-3-haiku")
        .with_author("Example Team")
        .with_timeout(1800);

    let toml = export_dag_to_swarm_toml(&dag, &config)?;

    // Print first 500 characters of TOML
    println!("\nExported Swarm.toml (preview):");
    println!("{}", &toml[..toml.len().min(500)]);
    println!("...\n");

    // Save to file
    use std::path::Path;
    let output_path = Path::new("examples/dag_workflows/output/simple_linear_workflow.toml");
    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_path, toml)?;

    println!("✓ Workflow exported to: {:?}", output_path);

    Ok(())
}
