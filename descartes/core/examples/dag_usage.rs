/// Example demonstrating DAG (Directed Acyclic Graph) usage for task dependencies
///
/// This example shows how to:
/// 1. Create a DAG representing task dependencies
/// 2. Add nodes and edges
/// 3. Validate the DAG (check for cycles)
/// 4. Perform topological sort for execution order
/// 5. Traverse the graph
/// 6. Serialize to/from TOML
/// 7. Analyze DAG statistics
use descartes_core::dag::{DAGEdge, DAGNode, EdgeType, DAG};
use descartes_core::dag_toml::{load_dag_from_toml, save_dag_to_toml, TomlDAG};
use uuid::Uuid;

fn main() {
    println!("=== DAG (Directed Acyclic Graph) Example ===\n");

    // Example 1: Build a simple software development workflow DAG
    println!("Example 1: Building a Software Development Workflow DAG");
    let dag = build_development_workflow();

    // Validate the DAG (check for cycles)
    println!("\nValidating DAG...");
    match dag.validate() {
        Ok(_) => println!("✓ DAG is valid (acyclic)"),
        Err(e) => println!("✗ DAG validation failed: {}", e),
    }

    // Get topological sort (execution order)
    println!("\nTopological Sort (Execution Order):");
    match dag.topological_sort() {
        Ok(sorted) => {
            for (i, node_id) in sorted.iter().enumerate() {
                if let Some(node) = dag.get_node(*node_id) {
                    println!("  {}. {} ({})", i + 1, node.label, node_id);
                }
            }
        }
        Err(e) => println!("Error: {}", e),
    }

    // Display statistics
    println!("\nDAG Statistics:");
    if let Ok(stats) = dag.statistics() {
        println!("  Total nodes: {}", stats.node_count);
        println!("  Total edges: {}", stats.edge_count);
        println!("  Start nodes: {}", stats.start_nodes);
        println!("  End nodes: {}", stats.end_nodes);
        println!("  Max depth: {}", stats.max_depth);
        println!("  Average in-degree: {:.2}", stats.average_in_degree);
        println!("  Average out-degree: {:.2}", stats.average_out_degree);
        println!("  Is acyclic: {}", stats.is_acyclic);
        println!("  Is connected: {}", stats.is_connected);
    }

    // Example 2: Breadth-first traversal
    println!("\nExample 2: Breadth-First Traversal");
    let start_nodes = dag.get_start_nodes();
    if let Some(start_id) = start_nodes.first() {
        println!("Starting from: {}", dag.get_node(*start_id).unwrap().label);
        let _ = dag.bfs_from(*start_id, |node_id, depth| {
            if let Some(node) = dag.get_node(node_id) {
                println!("  Depth {}: {}", depth, node.label);
            }
        });
    }

    // Example 3: Find all paths between two nodes
    println!("\nExample 3: Finding All Paths");
    let nodes: Vec<_> = dag.nodes.keys().copied().collect();
    if nodes.len() >= 2 {
        let start = nodes[0];
        let end = nodes[nodes.len() - 1];

        if let (Some(start_node), Some(end_node)) = (dag.get_node(start), dag.get_node(end)) {
            println!(
                "Finding paths from '{}' to '{}':",
                start_node.label, end_node.label
            );

            match dag.find_all_paths(start, end) {
                Ok(paths) => {
                    if paths.is_empty() {
                        println!("  No paths found");
                    } else {
                        for (i, path) in paths.iter().enumerate() {
                            print!("  Path {}: ", i + 1);
                            let path_names: Vec<_> = path
                                .iter()
                                .filter_map(|id| dag.get_node(*id).map(|n| &n.label))
                                .collect();
                            println!("{}", path_names.join(" -> "));
                        }
                    }
                }
                Err(e) => println!("  Error: {}", e),
            }
        }
    }

    // Example 4: Serialization to TOML
    println!("\nExample 4: Serialization to TOML");
    let toml_dag = TomlDAG::from_dag(&dag);
    match toml_dag.to_toml_string() {
        Ok(toml_str) => {
            println!("TOML representation (first 500 chars):");
            println!("{}", &toml_str[..toml_str.len().min(500)]);

            // Save to file
            let path = std::path::Path::new("/tmp/example_dag.toml");
            if let Err(e) = save_dag_to_toml(&dag, path) {
                println!("Error saving to file: {}", e);
            } else {
                println!("\n✓ Saved to {}", path.display());
            }
        }
        Err(e) => println!("Serialization error: {}", e),
    }

    // Example 5: Detect cycles (intentional failure)
    println!("\nExample 5: Cycle Detection");
    let mut cyclic_dag = DAG::new("Cyclic Graph (Invalid)");

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let node3 = DAGNode::new_auto("C");

    let id1 = node1.node_id;
    let id2 = node2.node_id;
    let id3 = node3.node_id;

    cyclic_dag.add_node(node1).unwrap();
    cyclic_dag.add_node(node2).unwrap();
    cyclic_dag.add_node(node3).unwrap();

    // Create a cycle: A -> B -> C -> A
    cyclic_dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();
    cyclic_dag.add_edge(DAGEdge::dependency(id2, id3)).unwrap();
    cyclic_dag.add_edge(DAGEdge::dependency(id3, id1)).unwrap();

    match cyclic_dag.validate() {
        Ok(_) => println!("  Unexpected: DAG validated (should have cycle)"),
        Err(e) => println!("  ✓ Correctly detected cycle: {}", e),
    }

    // Example 6: Different edge types
    println!("\nExample 6: Different Edge Types");
    let mut multi_edge_dag = DAG::new("Multi-Edge Type DAG");

    let task1 = DAGNode::new_auto("Core Implementation").with_metadata("priority", "critical");
    let task2 = DAGNode::new_auto("Documentation").with_metadata("priority", "low");
    let task3 = DAGNode::new_auto("Deployment").with_metadata("priority", "high");

    let t1 = task1.node_id;
    let t2 = task2.node_id;
    let t3 = task3.node_id;

    multi_edge_dag.add_node(task1).unwrap();
    multi_edge_dag.add_node(task2).unwrap();
    multi_edge_dag.add_node(task3).unwrap();

    // Hard dependency: deployment must wait for implementation
    multi_edge_dag
        .add_edge(DAGEdge::new(t1, t3, EdgeType::Dependency))
        .unwrap();

    // Soft dependency: docs should wait but can proceed independently
    multi_edge_dag
        .add_edge(DAGEdge::new(t1, t2, EdgeType::SoftDependency))
        .unwrap();

    println!("Edge types in use:");
    for (_, edge) in &multi_edge_dag.edges {
        let from = multi_edge_dag.get_node(edge.from_node_id).unwrap();
        let to = multi_edge_dag.get_node(edge.to_node_id).unwrap();
        println!("  {} -> {} [{:?}]", from.label, to.label, edge.edge_type);
    }

    println!("\n=== Example Complete ===");
}

/// Build a realistic software development workflow DAG
fn build_development_workflow() -> DAG {
    let mut dag = DAG::new("Software Development Workflow");
    dag.description = Some("Complete development workflow from design to deployment".to_string());

    // Create nodes representing development tasks
    let design = DAGNode::new_auto("Design Architecture")
        .with_description("Create architectural design and specifications")
        .with_position(100.0, 100.0)
        .with_metadata("estimated_hours", 8)
        .with_metadata("priority", "critical")
        .with_tag("planning");

    let backend = DAGNode::new_auto("Implement Backend")
        .with_description("Build backend API and business logic")
        .with_position(300.0, 50.0)
        .with_metadata("estimated_hours", 24)
        .with_metadata("priority", "high")
        .with_tag("implementation");

    let frontend = DAGNode::new_auto("Implement Frontend")
        .with_description("Build user interface components")
        .with_position(300.0, 150.0)
        .with_metadata("estimated_hours", 20)
        .with_metadata("priority", "high")
        .with_tag("implementation");

    let unit_tests = DAGNode::new_auto("Write Unit Tests")
        .with_description("Create comprehensive unit test coverage")
        .with_position(500.0, 100.0)
        .with_metadata("estimated_hours", 12)
        .with_metadata("priority", "critical")
        .with_tag("testing");

    let integration_tests = DAGNode::new_auto("Integration Testing")
        .with_description("Test system integration and E2E flows")
        .with_position(700.0, 100.0)
        .with_metadata("estimated_hours", 8)
        .with_metadata("priority", "high")
        .with_tag("testing");

    let deployment = DAGNode::new_auto("Deploy to Production")
        .with_description("Deploy the application to production")
        .with_position(900.0, 100.0)
        .with_metadata("estimated_hours", 4)
        .with_metadata("priority", "critical")
        .with_tag("deployment");

    // Save node IDs for creating edges
    let design_id = design.node_id;
    let backend_id = backend.node_id;
    let frontend_id = frontend.node_id;
    let unit_tests_id = unit_tests.node_id;
    let integration_tests_id = integration_tests.node_id;
    let deployment_id = deployment.node_id;

    // Add nodes to DAG
    dag.add_node(design).unwrap();
    dag.add_node(backend).unwrap();
    dag.add_node(frontend).unwrap();
    dag.add_node(unit_tests).unwrap();
    dag.add_node(integration_tests).unwrap();
    dag.add_node(deployment).unwrap();

    // Define dependencies (edges)
    // Design must be complete before implementation
    dag.add_edge(DAGEdge::dependency(design_id, backend_id).with_label("requires_design"))
        .unwrap();
    dag.add_edge(DAGEdge::dependency(design_id, frontend_id).with_label("requires_design"))
        .unwrap();

    // Implementation must be complete before testing
    dag.add_edge(DAGEdge::dependency(backend_id, unit_tests_id).with_label("needs_backend"))
        .unwrap();
    dag.add_edge(DAGEdge::dependency(frontend_id, unit_tests_id).with_label("needs_frontend"))
        .unwrap();

    // Unit tests before integration tests
    dag.add_edge(
        DAGEdge::dependency(unit_tests_id, integration_tests_id).with_label("needs_unit_tests"),
    )
    .unwrap();

    // Integration tests before deployment
    dag.add_edge(
        DAGEdge::dependency(integration_tests_id, deployment_id)
            .with_label("needs_integration_tests"),
    )
    .unwrap();

    dag
}
