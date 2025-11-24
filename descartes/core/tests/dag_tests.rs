/// Comprehensive Unit Tests for DAG Core Functionality
///
/// This test suite validates all core DAG operations including:
/// - Graph construction (add nodes, add edges)
/// - Topological sorting
/// - Cycle detection
/// - Path finding
/// - Node/edge validation
/// - Metadata management
/// - History and undo/redo
/// - Statistics and analysis

use descartes_core::dag::{
    DAG, DAGNode, DAGEdge, DAGWithHistory, DAGError, DAGHistory, DAGOperation,
    EdgeType, Position, DAGStatistics,
};
use std::collections::HashMap;
use uuid::Uuid;

// ============================================================================
// Graph Construction Tests
// ============================================================================

#[test]
fn test_create_empty_dag() {
    let dag = DAG::new("Test Workflow");

    assert_eq!(dag.name, "Test Workflow");
    assert_eq!(dag.nodes.len(), 0);
    assert_eq!(dag.edges.len(), 0);
    assert!(dag.description.is_none());
}

#[test]
fn test_create_dag_with_description() {
    let mut dag = DAG::new("Workflow");
    dag.description = Some("A test workflow".to_string());

    assert_eq!(dag.description.unwrap(), "A test workflow");
}

#[test]
fn test_add_single_node() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Task 1");
    let node_id = node.node_id;

    assert!(dag.add_node(node).is_ok());
    assert_eq!(dag.nodes.len(), 1);
    assert!(dag.get_node(node_id).is_some());
}

#[test]
fn test_add_multiple_nodes() {
    let mut dag = DAG::new("Test");

    let nodes = vec![
        DAGNode::new_auto("Task 1"),
        DAGNode::new_auto("Task 2"),
        DAGNode::new_auto("Task 3"),
    ];

    for node in nodes {
        assert!(dag.add_node(node).is_ok());
    }

    assert_eq!(dag.nodes.len(), 3);
}

#[test]
fn test_add_duplicate_node() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Task");
    let node_id = node.node_id;

    assert!(dag.add_node(node.clone()).is_ok());
    assert!(matches!(dag.add_node(node), Err(DAGError::DuplicateNode(_))));
}

#[test]
fn test_add_node_with_position() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Task")
        .with_position(100.0, 200.0);
    let node_id = node.node_id;

    dag.add_node(node).unwrap();

    let retrieved = dag.get_node(node_id).unwrap();
    assert_eq!(retrieved.position.x, 100.0);
    assert_eq!(retrieved.position.y, 200.0);
}

#[test]
fn test_add_node_with_metadata() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Task")
        .with_metadata("priority", "high")
        .with_metadata("owner", "team-a");
    let node_id = node.node_id;

    dag.add_node(node).unwrap();

    let retrieved = dag.get_node(node_id).unwrap();
    assert_eq!(retrieved.metadata.get("priority").unwrap().as_str().unwrap(), "high");
    assert_eq!(retrieved.metadata.get("owner").unwrap().as_str().unwrap(), "team-a");
}

#[test]
fn test_add_node_with_tags() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Task")
        .with_tag("backend")
        .with_tag("critical");
    let node_id = node.node_id;

    dag.add_node(node).unwrap();

    let retrieved = dag.get_node(node_id).unwrap();
    assert_eq!(retrieved.tags.len(), 2);
    assert!(retrieved.tags.contains(&"backend".to_string()));
    assert!(retrieved.tags.contains(&"critical".to_string()));
}

#[test]
fn test_remove_node() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Task");
    let node_id = node.node_id;

    dag.add_node(node).unwrap();
    assert_eq!(dag.nodes.len(), 1);

    assert!(dag.remove_node(node_id).is_ok());
    assert_eq!(dag.nodes.len(), 0);
    assert!(dag.get_node(node_id).is_none());
}

#[test]
fn test_remove_nonexistent_node() {
    let mut dag = DAG::new("Test");
    let fake_id = Uuid::new_v4();

    assert!(matches!(dag.remove_node(fake_id), Err(DAGError::NodeNotFound(_))));
}

#[test]
fn test_remove_node_removes_connected_edges() {
    let mut dag = DAG::new("Test");

    let node1 = DAGNode::new_auto("Task 1");
    let node2 = DAGNode::new_auto("Task 2");
    let node3 = DAGNode::new_auto("Task 3");

    let id1 = node1.node_id;
    let id2 = node2.node_id;
    let id3 = node3.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();
    dag.add_node(node3).unwrap();

    // Create edges: 1 -> 2 -> 3
    dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();
    dag.add_edge(DAGEdge::dependency(id2, id3)).unwrap();

    assert_eq!(dag.edges.len(), 2);

    // Remove middle node
    dag.remove_node(id2).unwrap();

    // Both edges should be removed
    assert_eq!(dag.edges.len(), 0);
}

#[test]
fn test_update_node() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Original");
    let node_id = node.node_id;

    dag.add_node(node).unwrap();

    let updated = DAGNode::new(node_id, "Updated")
        .with_description("New description")
        .with_position(500.0, 500.0);

    assert!(dag.update_node(node_id, updated).is_ok());

    let retrieved = dag.get_node(node_id).unwrap();
    assert_eq!(retrieved.label, "Updated");
    assert_eq!(retrieved.description.as_ref().unwrap(), "New description");
    assert_eq!(retrieved.position.x, 500.0);
}

#[test]
fn test_update_nonexistent_node() {
    let mut dag = DAG::new("Test");
    let fake_id = Uuid::new_v4();
    let node = DAGNode::new(fake_id, "Fake");

    assert!(matches!(dag.update_node(fake_id, node), Err(DAGError::NodeNotFound(_))));
}

#[test]
fn test_add_simple_edge() {
    let mut dag = DAG::new("Test");

    let node1 = DAGNode::new_auto("Task 1");
    let node2 = DAGNode::new_auto("Task 2");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();

    let edge = DAGEdge::dependency(id1, id2);
    assert!(dag.add_edge(edge).is_ok());
    assert_eq!(dag.edges.len(), 1);
}

#[test]
fn test_add_edge_with_label() {
    let mut dag = DAG::new("Test");

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();

    let edge = DAGEdge::dependency(id1, id2).with_label("on_success");
    let edge_id = edge.edge_id;

    dag.add_edge(edge).unwrap();

    let retrieved = dag.get_edge(edge_id).unwrap();
    assert_eq!(retrieved.label.as_ref().unwrap(), "on_success");
}

#[test]
fn test_add_edge_with_metadata() {
    let mut dag = DAG::new("Test");

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();

    let edge = DAGEdge::dependency(id1, id2)
        .with_metadata("condition", "x > 0");
    let edge_id = edge.edge_id;

    dag.add_edge(edge).unwrap();

    let retrieved = dag.get_edge(edge_id).unwrap();
    assert_eq!(retrieved.metadata.get("condition").unwrap().as_str().unwrap(), "x > 0");
}

#[test]
fn test_add_multiple_edge_types() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (0..4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::new(ids[0], ids[1], EdgeType::Dependency)).unwrap();
    dag.add_edge(DAGEdge::new(ids[1], ids[2], EdgeType::SoftDependency)).unwrap();
    dag.add_edge(DAGEdge::new(ids[2], ids[3], EdgeType::DataFlow)).unwrap();

    assert_eq!(dag.edges.len(), 3);

    let edges: Vec<_> = dag.edges.values().collect();
    assert!(edges.iter().any(|e| matches!(e.edge_type, EdgeType::Dependency)));
    assert!(edges.iter().any(|e| matches!(e.edge_type, EdgeType::SoftDependency)));
    assert!(edges.iter().any(|e| matches!(e.edge_type, EdgeType::DataFlow)));
}

#[test]
fn test_self_loop_rejected() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Task");
    let id = node.node_id;

    dag.add_node(node).unwrap();

    let edge = DAGEdge::dependency(id, id);
    assert!(matches!(dag.add_edge(edge), Err(DAGError::SelfLoop(_))));
}

#[test]
fn test_edge_to_nonexistent_node() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Task");
    let id = node.node_id;
    let fake_id = Uuid::new_v4();

    dag.add_node(node).unwrap();

    let edge = DAGEdge::dependency(id, fake_id);
    assert!(matches!(dag.add_edge(edge), Err(DAGError::NodeNotFound(_))));
}

#[test]
fn test_remove_edge() {
    let mut dag = DAG::new("Test");

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();

    let edge = DAGEdge::dependency(id1, id2);
    let edge_id = edge.edge_id;

    dag.add_edge(edge).unwrap();
    assert_eq!(dag.edges.len(), 1);

    assert!(dag.remove_edge(edge_id).is_ok());
    assert_eq!(dag.edges.len(), 0);
}

#[test]
fn test_get_edges_between() {
    let mut dag = DAG::new("Test");

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();

    // Add multiple edges between same nodes
    dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();
    dag.add_edge(DAGEdge::soft_dependency(id1, id2)).unwrap();

    let edges = dag.get_edges_between(id1, id2);
    assert_eq!(edges.len(), 2);

    // No edges in reverse
    let reverse = dag.get_edges_between(id2, id1);
    assert_eq!(reverse.len(), 0);
}

// ============================================================================
// Topological Sort Tests
// ============================================================================

#[test]
fn test_topological_sort_linear_chain() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=5).map(|i| DAGNode::new_auto(format!("T{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Create chain: 1 -> 2 -> 3 -> 4 -> 5
    for i in 0..4 {
        dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1])).unwrap();
    }

    let sorted = dag.topological_sort().unwrap();
    assert_eq!(sorted.len(), 5);

    // Verify order
    for i in 0..4 {
        let pos_i = sorted.iter().position(|&x| x == ids[i]).unwrap();
        let pos_next = sorted.iter().position(|&x| x == ids[i + 1]).unwrap();
        assert!(pos_i < pos_next);
    }
}

#[test]
fn test_topological_sort_diamond() {
    let mut dag = DAG::new("Test");

    //     1
    //    / \
    //   2   3
    //    \ /
    //     4
    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap(); // 1 -> 2
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap(); // 1 -> 3
    dag.add_edge(DAGEdge::dependency(ids[1], ids[3])).unwrap(); // 2 -> 4
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap(); // 3 -> 4

    let sorted = dag.topological_sort().unwrap();
    assert_eq!(sorted.len(), 4);

    // 1 must come before 2 and 3
    let pos1 = sorted.iter().position(|&x| x == ids[0]).unwrap();
    let pos2 = sorted.iter().position(|&x| x == ids[1]).unwrap();
    let pos3 = sorted.iter().position(|&x| x == ids[2]).unwrap();
    let pos4 = sorted.iter().position(|&x| x == ids[3]).unwrap();

    assert!(pos1 < pos2);
    assert!(pos1 < pos3);
    assert!(pos2 < pos4);
    assert!(pos3 < pos4);
}

#[test]
fn test_topological_sort_multiple_roots() {
    let mut dag = DAG::new("Test");

    //   1   2
    //    \ /
    //     3
    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap(); // 1 -> 3
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap(); // 2 -> 3

    let sorted = dag.topological_sort().unwrap();
    assert_eq!(sorted.len(), 3);

    // Both 1 and 2 must come before 3
    let pos3 = sorted.iter().position(|&x| x == ids[2]).unwrap();
    let pos1 = sorted.iter().position(|&x| x == ids[0]).unwrap();
    let pos2 = sorted.iter().position(|&x| x == ids[1]).unwrap();

    assert!(pos1 < pos3);
    assert!(pos2 < pos3);
}

#[test]
fn test_topological_sort_empty_dag() {
    let dag = DAG::new("Empty");
    let sorted = dag.topological_sort().unwrap();
    assert_eq!(sorted.len(), 0);
}

#[test]
fn test_topological_sort_single_node() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("Single");
    let id = node.node_id;

    dag.add_node(node).unwrap();

    let sorted = dag.topological_sort().unwrap();
    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0], id);
}

#[test]
fn test_get_execution_order() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("T{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    let order = dag.get_execution_order().unwrap();
    assert_eq!(order, vec![ids[0], ids[1], ids[2]]);
}

// ============================================================================
// Cycle Detection Tests
// ============================================================================

#[test]
fn test_validate_acyclic_dag() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    assert!(dag.validate().is_ok());
}

#[test]
fn test_detect_simple_cycle() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Create cycle: 1 -> 2 -> 3 -> 1
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[0])).unwrap();

    assert!(matches!(dag.validate(), Err(DAGError::CycleDetected(_))));
    assert!(dag.topological_sort().is_err());
}

#[test]
fn test_detect_self_cycle() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("N");
    let id = node.node_id;

    dag.add_node(node).unwrap();

    // Self-loop is rejected at add_edge
    assert!(matches!(
        dag.add_edge(DAGEdge::dependency(id, id)),
        Err(DAGError::SelfLoop(_))
    ));
}

#[test]
fn test_detect_complex_cycle() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=6).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 3 (cycle)
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[3], ids[4])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[4], ids[5])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[5], ids[2])).unwrap(); // Creates cycle

    assert!(dag.validate().is_err());
}

#[test]
fn test_detect_cycles_method() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (0..6).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Create two separate cycles
    // Cycle 1: 0 -> 1 -> 2 -> 0
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[0])).unwrap();

    // Cycle 2: 3 -> 4 -> 5 -> 3
    dag.add_edge(DAGEdge::dependency(ids[3], ids[4])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[4], ids[5])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[5], ids[3])).unwrap();

    let cycles = dag.detect_cycles();
    assert!(cycles.len() >= 2);
}

// ============================================================================
// Path Finding Tests
// ============================================================================

#[test]
fn test_has_path_direct() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=2).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();

    assert!(dag.has_path(ids[0], ids[1]));
    assert!(!dag.has_path(ids[1], ids[0]));
}

#[test]
fn test_has_path_indirect() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=5).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Chain: 1 -> 2 -> 3 -> 4 -> 5
    for i in 0..4 {
        dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1])).unwrap();
    }

    // Path from 1 to 5 exists
    assert!(dag.has_path(ids[0], ids[4]));

    // No path from 5 to 1
    assert!(!dag.has_path(ids[4], ids[0]));

    // Path from 2 to 5 exists
    assert!(dag.has_path(ids[1], ids[4]));
}

#[test]
fn test_has_path_self() {
    let mut dag = DAG::new("Test");
    let node = DAGNode::new_auto("N");
    let id = node.node_id;

    dag.add_node(node).unwrap();

    // A node always has a "path" to itself
    assert!(dag.has_path(id, id));
}

#[test]
fn test_find_all_paths_single() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Single path: 1 -> 2 -> 3
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    let paths = dag.find_all_paths(ids[0], ids[2]).unwrap();
    assert_eq!(paths.len(), 1);
    assert_eq!(paths[0], vec![ids[0], ids[1], ids[2]]);
}

#[test]
fn test_find_all_paths_multiple() {
    let mut dag = DAG::new("Test");

    //     1
    //    / \
    //   2   3
    //    \ /
    //     4
    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap(); // 1 -> 2
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap(); // 1 -> 3
    dag.add_edge(DAGEdge::dependency(ids[1], ids[3])).unwrap(); // 2 -> 4
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap(); // 3 -> 4

    let paths = dag.find_all_paths(ids[0], ids[3]).unwrap();
    assert_eq!(paths.len(), 2);

    // Two paths: 1 -> 2 -> 4 and 1 -> 3 -> 4
    assert!(paths.contains(&vec![ids[0], ids[1], ids[3]]));
    assert!(paths.contains(&vec![ids[0], ids[2], ids[3]]));
}

#[test]
fn test_find_dependencies() {
    let mut dag = DAG::new("Test");

    // Chain: 1 -> 2 -> 3 -> 4
    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    for i in 0..3 {
        dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1])).unwrap();
    }

    // Node 4 depends on 3, 2, and 1
    let deps = dag.find_dependencies(ids[3]);
    assert_eq!(deps.len(), 3);
    assert!(deps.contains(&ids[0]));
    assert!(deps.contains(&ids[1]));
    assert!(deps.contains(&ids[2]));

    // Node 1 has no dependencies
    let deps1 = dag.find_dependencies(ids[0]);
    assert_eq!(deps1.len(), 0);
}

#[test]
fn test_find_dependents() {
    let mut dag = DAG::new("Test");

    // Chain: 1 -> 2 -> 3 -> 4
    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    for i in 0..3 {
        dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1])).unwrap();
    }

    // Node 1 has 3 dependents: 2, 3, 4
    let deps = dag.find_dependents(ids[0]);
    assert_eq!(deps.len(), 3);
    assert!(deps.contains(&ids[1]));
    assert!(deps.contains(&ids[2]));
    assert!(deps.contains(&ids[3]));

    // Node 4 has no dependents
    let deps4 = dag.find_dependents(ids[3]);
    assert_eq!(deps4.len(), 0);
}

#[test]
fn test_find_critical_path() {
    let mut dag = DAG::new("Test");

    //     1
    //    / \
    //   2   3
    //    \ /
    //     4
    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[3])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap();

    let critical = dag.find_critical_path().unwrap();

    assert_eq!(critical.len(), 3);
    assert_eq!(critical[0], ids[0]); // Starts with node 1
    assert_eq!(critical[2], ids[3]); // Ends with node 4
}

// ============================================================================
// Graph Query Tests
// ============================================================================

#[test]
fn test_get_start_nodes() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    //   1   2
    //    \ /
    //     3
    //     |
    //     4
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap();

    let starts = dag.get_start_nodes();
    assert_eq!(starts.len(), 2);
    assert!(starts.contains(&ids[0]));
    assert!(starts.contains(&ids[1]));
}

#[test]
fn test_get_end_nodes() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    //     1
    //    / \
    //   2   3
    //       |
    //       4
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap();

    let ends = dag.get_end_nodes();
    assert_eq!(ends.len(), 2);
    assert!(ends.contains(&ids[1]));
    assert!(ends.contains(&ids[3]));
}

#[test]
fn test_find_roots_and_leaves() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    let roots = dag.find_roots();
    let leaves = dag.find_leaves();

    assert_eq!(roots.len(), 1);
    assert_eq!(leaves.len(), 1);
    assert_eq!(roots[0], ids[0]);
    assert_eq!(leaves[0], ids[2]);
}

#[test]
fn test_get_successors() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    //     1
    //    /|\
    //   2 3 4
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[0], ids[3])).unwrap();

    let successors = dag.get_successors(ids[0]);
    assert_eq!(successors.len(), 3);
    assert!(successors.contains(&ids[1]));
    assert!(successors.contains(&ids[2]));
    assert!(successors.contains(&ids[3]));
}

#[test]
fn test_get_predecessors() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    //   1 2 3
    //    \|/
    //     4
    dag.add_edge(DAGEdge::dependency(ids[0], ids[3])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[3])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap();

    let predecessors = dag.get_predecessors(ids[3]);
    assert_eq!(predecessors.len(), 3);
    assert!(predecessors.contains(&ids[0]));
    assert!(predecessors.contains(&ids[1]));
    assert!(predecessors.contains(&ids[2]));
}

#[test]
fn test_get_incoming_edges() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    let incoming = dag.get_incoming_edges(ids[2]);
    assert_eq!(incoming.len(), 2);
}

#[test]
fn test_get_outgoing_edges() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap();

    let outgoing = dag.get_outgoing_edges(ids[0]);
    assert_eq!(outgoing.len(), 2);
}

// ============================================================================
// Graph Analysis Tests
// ============================================================================

#[test]
fn test_max_depth_linear() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=5).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Chain of depth 4 (5 nodes, 4 edges)
    for i in 0..4 {
        dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1])).unwrap();
    }

    assert_eq!(dag.max_depth(), 4);
}

#[test]
fn test_max_depth_diamond() {
    let mut dag = DAG::new("Test");

    //     1
    //    / \
    //   2   3
    //    \ /
    //     4
    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[3])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap();

    assert_eq!(dag.max_depth(), 2);
}

#[test]
fn test_is_connected_true() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    assert!(dag.is_connected());
}

#[test]
fn test_is_connected_false() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Only connect 1 -> 2, leaving 3 isolated
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();

    assert!(!dag.is_connected());
}

#[test]
fn test_validate_connectivity() {
    let mut dag = DAG::new("Test");

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();
    dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();

    assert!(dag.validate_connectivity().is_ok());

    // Add disconnected node
    let node3 = DAGNode::new_auto("C");
    dag.add_node(node3).unwrap();

    assert!(matches!(dag.validate_connectivity(), Err(DAGError::UnreachableNodes(_))));
}

#[test]
fn test_statistics() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    let stats = dag.statistics().unwrap();

    assert_eq!(stats.node_count, 3);
    assert_eq!(stats.edge_count, 2);
    assert_eq!(stats.start_nodes, 1);
    assert_eq!(stats.end_nodes, 1);
    assert_eq!(stats.max_depth, 2);
    assert!(stats.is_acyclic);
    assert!(stats.is_connected);
}

#[test]
fn test_get_subgraph() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // 1 -> 2 -> 3 -> 4
    for i in 0..3 {
        dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1])).unwrap();
    }

    // Extract subgraph with nodes 2 and 3
    let subgraph = dag.get_subgraph(&[ids[1], ids[2]]).unwrap();

    assert_eq!(subgraph.nodes.len(), 2);
    assert_eq!(subgraph.edges.len(), 1); // Only edge 2 -> 3
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_json_serialization() {
    let mut dag = DAG::new("Test Workflow");
    dag.description = Some("Test description".to_string());

    let node1 = DAGNode::new_auto("Task 1")
        .with_position(100.0, 200.0)
        .with_metadata("priority", "high");
    let node2 = DAGNode::new_auto("Task 2")
        .with_position(300.0, 200.0);

    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();
    dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();

    // Serialize to JSON
    let json = serde_json::to_string(&dag).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("Test Workflow"));

    // Deserialize and verify
    let mut deserialized: DAG = serde_json::from_str(&json).unwrap();
    deserialized.rebuild_adjacency();

    assert_eq!(deserialized.name, "Test Workflow");
    assert_eq!(deserialized.nodes.len(), 2);
    assert_eq!(deserialized.edges.len(), 1);
}

#[test]
fn test_adjacency_rebuild() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    // Serialize and deserialize
    let json = serde_json::to_string(&dag).unwrap();
    let mut deserialized: DAG = serde_json::from_str(&json).unwrap();

    // Adjacency should be empty before rebuild
    deserialized.rebuild_adjacency();

    // Test that adjacency works
    assert_eq!(deserialized.get_successors(ids[0]).len(), 1);
    assert_eq!(deserialized.get_predecessors(ids[2]).len(), 1);
}

// ============================================================================
// History and Undo/Redo Tests
// ============================================================================

#[test]
fn test_dag_with_history_add_node() {
    let mut dag = DAGWithHistory::new("Test");

    let node = DAGNode::new_auto("Task 1");
    dag.add_node(node).unwrap();

    assert_eq!(dag.dag.nodes.len(), 1);
    assert!(dag.can_undo());
    assert!(!dag.can_redo());
}

#[test]
fn test_dag_with_history_undo_add_node() {
    let mut dag = DAGWithHistory::new("Test");

    let node = DAGNode::new_auto("Task 1");
    let node_id = node.node_id;
    dag.add_node(node).unwrap();

    assert_eq!(dag.dag.nodes.len(), 1);

    dag.undo().unwrap();

    assert_eq!(dag.dag.nodes.len(), 0);
    assert!(!dag.can_undo());
    assert!(dag.can_redo());
}

#[test]
fn test_dag_with_history_redo() {
    let mut dag = DAGWithHistory::new("Test");

    let node = DAGNode::new_auto("Task 1");
    dag.add_node(node).unwrap();

    dag.undo().unwrap();
    assert_eq!(dag.dag.nodes.len(), 0);

    dag.redo().unwrap();
    assert_eq!(dag.dag.nodes.len(), 1);
}

#[test]
fn test_dag_with_history_update_node() {
    let mut dag = DAGWithHistory::new("Test");

    let node = DAGNode::new_auto("Original");
    let node_id = node.node_id;
    dag.add_node(node).unwrap();

    let updated = DAGNode::new(node_id, "Updated");
    dag.update_node(node_id, updated).unwrap();

    assert_eq!(dag.dag.get_node(node_id).unwrap().label, "Updated");

    dag.undo().unwrap();
    assert_eq!(dag.dag.get_node(node_id).unwrap().label, "Original");

    dag.redo().unwrap();
    assert_eq!(dag.dag.get_node(node_id).unwrap().label, "Updated");
}

#[test]
fn test_dag_with_history_remove_node() {
    let mut dag = DAGWithHistory::new("Test");

    let node = DAGNode::new_auto("Task");
    let node_id = node.node_id;
    dag.add_node(node).unwrap();

    dag.remove_node(node_id).unwrap();
    assert_eq!(dag.dag.nodes.len(), 0);

    dag.undo().unwrap();
    assert_eq!(dag.dag.nodes.len(), 1);
    assert!(dag.dag.get_node(node_id).is_some());
}

#[test]
fn test_dag_with_history_add_edge() {
    let mut dag = DAGWithHistory::new("Test");

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();

    let edge = DAGEdge::dependency(id1, id2);
    dag.add_edge(edge).unwrap();

    assert_eq!(dag.dag.edges.len(), 1);

    dag.undo().unwrap();
    assert_eq!(dag.dag.edges.len(), 0);
}

#[test]
fn test_dag_with_history_clear() {
    let mut dag = DAGWithHistory::new("Test");

    let node = DAGNode::new_auto("Task");
    dag.add_node(node).unwrap();

    assert!(dag.can_undo());

    dag.clear_history();

    assert!(!dag.can_undo());
    assert!(!dag.can_redo());
}

#[test]
fn test_history_max_size() {
    let mut history = DAGHistory::with_max_size(3);

    for i in 1..=5 {
        let node = DAGNode::new_auto(format!("Task {}", i));
        history.record(DAGOperation::AddNode(node));
    }

    // Should only keep last 3
    assert_eq!(history.undo_count(), 3);
}

// ============================================================================
// Traversal Tests
// ============================================================================

#[test]
fn test_bfs_traversal() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=4).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    //     1
    //    / \
    //   2   3
    //       |
    //       4
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[0], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[3])).unwrap();

    let mut visited = Vec::new();
    dag.bfs_from(ids[0], |node_id, _depth| {
        visited.push(node_id);
    }).unwrap();

    assert_eq!(visited.len(), 4);
    assert_eq!(visited[0], ids[0]); // Start node visited first
}

#[test]
fn test_dfs_traversal() {
    let mut dag = DAG::new("Test");

    let nodes: Vec<_> = (1..=3).map(|i| DAGNode::new_auto(format!("N{}", i))).collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();

    let mut visited = Vec::new();
    dag.dfs_from(ids[0], |node_id, _depth| {
        visited.push(node_id);
    }).unwrap();

    assert_eq!(visited.len(), 3);
    assert_eq!(visited[0], ids[0]); // Start node visited first
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_empty_dag_operations() {
    let dag = DAG::new("Empty");

    assert!(dag.topological_sort().unwrap().is_empty());
    assert_eq!(dag.max_depth(), 0);
    assert!(dag.is_connected());
    assert!(dag.get_start_nodes().is_empty());
    assert!(dag.get_end_nodes().is_empty());
}

#[test]
fn test_single_node_operations() {
    let mut dag = DAG::new("Single");
    let node = DAGNode::new_auto("Only");
    let id = node.node_id;

    dag.add_node(node).unwrap();

    assert_eq!(dag.topological_sort().unwrap(), vec![id]);
    assert_eq!(dag.max_depth(), 0);
    assert!(dag.is_connected());
    assert_eq!(dag.get_start_nodes(), vec![id]);
    assert_eq!(dag.get_end_nodes(), vec![id]);
}

#[test]
fn test_large_dag() {
    let mut dag = DAG::new("Large");

    // Create 100 nodes
    let nodes: Vec<_> = (0..100)
        .map(|i| DAGNode::new_auto(format!("Node {}", i)))
        .collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Create linear chain
    for i in 0..99 {
        dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1])).unwrap();
    }

    assert_eq!(dag.nodes.len(), 100);
    assert_eq!(dag.edges.len(), 99);
    assert!(dag.validate().is_ok());

    let sorted = dag.topological_sort().unwrap();
    assert_eq!(sorted.len(), 100);
}

#[test]
fn test_position_distance() {
    let pos1 = Position::new(0.0, 0.0);
    let pos2 = Position::new(3.0, 4.0);

    let distance = pos1.distance_to(&pos2);
    assert!((distance - 5.0).abs() < 0.001);
}
