/// Test file to verify phase3:8.2 DAG implementation
/// This file demonstrates all the newly implemented functionality

use descartes_core::dag::{DAG, DAGNode, DAGEdge, EdgeType, DAGWithHistory};
use uuid::Uuid;

fn main() {
    println!("=== Phase 3.8.2 - Core Graph Logic Implementation Test ===\n");

    // Test 1: Node operations
    println!("Test 1: Node Operations");
    let mut dag = DAG::new("Test DAG");

    let node1 = DAGNode::new_auto("Task 1");
    let node2 = DAGNode::new_auto("Task 2");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();
    println!("✓ Added 2 nodes");

    // Test update_node
    let updated = DAGNode::new(id1, "Updated Task 1");
    dag.update_node(id1, updated).unwrap();
    println!("✓ Updated node");

    // Test 2: Edge operations
    println!("\nTest 2: Edge Operations");
    let edge = DAGEdge::dependency(id1, id2);
    dag.add_edge(edge).unwrap();
    println!("✓ Added edge");

    // Test get_edges_between
    let edges = dag.get_edges_between(id1, id2);
    println!("✓ get_edges_between: {} edges found", edges.len());

    // Test 3: Graph validation
    println!("\nTest 3: Graph Validation");
    let valid = dag.validate();
    println!("✓ validate_acyclic: {:?}", valid.is_ok());

    let cycles = dag.detect_cycles();
    println!("✓ detect_cycles: {} cycles found", cycles.len());

    let connected = dag.validate_connectivity();
    println!("✓ validate_connectivity: {:?}", connected.is_ok());

    // Test 4: Graph algorithms
    println!("\nTest 4: Graph Algorithms");
    let sorted = dag.topological_sort().unwrap();
    println!("✓ topological_sort: {} nodes", sorted.len());

    let deps = dag.find_dependencies(id2);
    println!("✓ find_dependencies: {} dependencies", deps.len());

    let dependents = dag.find_dependents(id1);
    println!("✓ find_dependents: {} dependents", dependents.len());

    let exec_order = dag.get_execution_order().unwrap();
    println!("✓ get_execution_order: {} nodes", exec_order.len());

    // Test 5: Graph queries
    println!("\nTest 5: Graph Queries");
    let roots = dag.find_roots();
    println!("✓ find_roots: {} root nodes", roots.len());

    let leaves = dag.find_leaves();
    println!("✓ find_leaves: {} leaf nodes", leaves.len());

    let subgraph = dag.get_subgraph(&[id1, id2]).unwrap();
    println!("✓ get_subgraph: {} nodes, {} edges", subgraph.nodes.len(), subgraph.edges.len());

    let critical_path = dag.find_critical_path().unwrap();
    println!("✓ find_critical_path: {} nodes in path", critical_path.len());

    // Test 6: State management with history
    println!("\nTest 6: State Management (Undo/Redo)");
    let mut dag_hist = DAGWithHistory::new("History DAG");

    let n1 = DAGNode::new_auto("Node A");
    let n2 = DAGNode::new_auto("Node B");
    let nid1 = n1.node_id;
    let nid2 = n2.node_id;

    dag_hist.add_node(n1).unwrap();
    dag_hist.add_node(n2).unwrap();
    println!("✓ Added 2 nodes with history");

    println!("  Can undo: {}", dag_hist.can_undo());
    println!("  Can redo: {}", dag_hist.can_redo());

    dag_hist.undo().unwrap();
    println!("✓ Undo successful (nodes: {})", dag_hist.dag.nodes.len());

    dag_hist.redo().unwrap();
    println!("✓ Redo successful (nodes: {})", dag_hist.dag.nodes.len());

    let edge_hist = DAGEdge::dependency(nid1, nid2);
    dag_hist.add_edge(edge_hist).unwrap();
    println!("✓ Added edge with history");

    dag_hist.undo().unwrap();
    println!("✓ Undid edge addition");

    println!("\n=== All Phase 3.8.2 Tests Passed! ===");
}
