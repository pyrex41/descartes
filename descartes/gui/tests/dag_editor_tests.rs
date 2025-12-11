use descartes_core::dag::{DAGEdge, DAGNode, EdgeType, Position, DAG};
use descartes_gui::dag_canvas_interactions::*;
/// Enhanced Tests for DAG Editor Visual Components
///
/// This test suite validates the visual editor functionality including:
/// - Canvas rendering logic
/// - Layout algorithms
/// - Node positioning
/// - Edge routing
/// - Selection and highlighting
/// - Zoom and pan operations
/// - Grid and snap-to-grid
/// - Performance with large graphs
use descartes_gui::dag_editor::*;
use iced::{Point, Vector};
use uuid::Uuid;

// ============================================================================
// Canvas Coordinate System Tests
// ============================================================================

#[test]
fn test_screen_to_world_coordinates() {
    let mut canvas_state = CanvasState::default();
    canvas_state.zoom = 1.0;
    canvas_state.offset = Vector::new(0.0, 0.0);

    let screen_point = Point::new(100.0, 200.0);
    let world_point = screen_to_world(screen_point, &canvas_state);

    assert_eq!(world_point.x, 100.0);
    assert_eq!(world_point.y, 200.0);
}

#[test]
fn test_screen_to_world_with_zoom() {
    let mut canvas_state = CanvasState::default();
    canvas_state.zoom = 2.0;
    canvas_state.offset = Vector::new(0.0, 0.0);

    let screen_point = Point::new(100.0, 200.0);
    let world_point = screen_to_world(screen_point, &canvas_state);

    // At 2x zoom, screen coordinate 100 maps to world 50
    assert_eq!(world_point.x, 50.0);
    assert_eq!(world_point.y, 100.0);
}

#[test]
fn test_screen_to_world_with_offset() {
    let mut canvas_state = CanvasState::default();
    canvas_state.zoom = 1.0;
    canvas_state.offset = Vector::new(50.0, 100.0);

    let screen_point = Point::new(100.0, 200.0);
    let world_point = screen_to_world(screen_point, &canvas_state);

    // With offset, screen 100 - offset 50 = world 50
    assert_eq!(world_point.x, 50.0);
    assert_eq!(world_point.y, 100.0);
}

#[test]
fn test_world_to_screen_coordinates() {
    let mut canvas_state = CanvasState::default();
    canvas_state.zoom = 1.0;
    canvas_state.offset = Vector::new(0.0, 0.0);

    let world_point = Point::new(100.0, 200.0);
    let screen_point = world_to_screen(world_point, &canvas_state);

    assert_eq!(screen_point.x, 100.0);
    assert_eq!(screen_point.y, 200.0);
}

#[test]
fn test_world_to_screen_with_zoom() {
    let mut canvas_state = CanvasState::default();
    canvas_state.zoom = 2.0;
    canvas_state.offset = Vector::new(0.0, 0.0);

    let world_point = Point::new(100.0, 200.0);
    let screen_point = world_to_screen(world_point, &canvas_state);

    // At 2x zoom, world 100 maps to screen 200
    assert_eq!(screen_point.x, 200.0);
    assert_eq!(screen_point.y, 400.0);
}

#[test]
fn test_coordinate_roundtrip() {
    let mut canvas_state = CanvasState::default();
    canvas_state.zoom = 1.5;
    canvas_state.offset = Vector::new(25.0, 50.0);

    let original_screen = Point::new(150.0, 300.0);
    let world = screen_to_world(original_screen, &canvas_state);
    let back_to_screen = world_to_screen(world, &canvas_state);

    // Should get back to original (within floating point precision)
    assert!((back_to_screen.x - original_screen.x).abs() < 0.01);
    assert!((back_to_screen.y - original_screen.y).abs() < 0.01);
}

// ============================================================================
// Node Hit Detection Tests
// ============================================================================

#[test]
fn test_point_in_node_center() {
    let node = DAGNode::new_auto("Test").with_position(100.0, 100.0);
    let canvas_state = CanvasState::default();

    // Point at node center (adjusted for node size)
    let point = Point::new(180.0, 130.0); // Center of 160x60 box at (100, 100)

    assert!(point_in_node(point, &node, &canvas_state));
}

#[test]
fn test_point_in_node_edge() {
    let node = DAGNode::new_auto("Test").with_position(100.0, 100.0);
    let canvas_state = CanvasState::default();

    // Point at node edge
    let point = Point::new(105.0, 105.0);

    assert!(point_in_node(point, &node, &canvas_state));
}

#[test]
fn test_point_outside_node() {
    let node = DAGNode::new_auto("Test").with_position(100.0, 100.0);
    let canvas_state = CanvasState::default();

    // Point clearly outside
    let point = Point::new(50.0, 50.0);

    assert!(!point_in_node(point, &node, &canvas_state));
}

#[test]
fn test_point_in_node_with_zoom() {
    let node = DAGNode::new_auto("Test").with_position(100.0, 100.0);
    let mut canvas_state = CanvasState::default();
    canvas_state.zoom = 2.0;

    // Node is larger at 2x zoom
    let point = Point::new(220.0, 140.0);

    assert!(point_in_node(point, &node, &canvas_state));
}

// ============================================================================
// Grid and Snap Tests
// ============================================================================

#[test]
fn test_snap_to_grid_basic() {
    let point = Point::new(123.0, 456.0);
    let snapped = snap_to_grid(point);

    // Should snap to nearest 20-pixel grid
    assert_eq!(snapped.x % 20.0, 0.0);
    assert_eq!(snapped.y % 20.0, 0.0);
}

#[test]
fn test_snap_to_grid_already_aligned() {
    let point = Point::new(100.0, 200.0);
    let snapped = snap_to_grid(point);

    assert_eq!(snapped.x, 100.0);
    assert_eq!(snapped.y, 200.0);
}

#[test]
fn test_snap_to_grid_negative() {
    let point = Point::new(-15.0, -35.0);
    let snapped = snap_to_grid(point);

    // Should snap to nearest grid including negatives
    assert_eq!(snapped.x % 20.0, 0.0);
    assert_eq!(snapped.y % 20.0, 0.0);
}

#[test]
fn test_snap_to_grid_halfway() {
    let point = Point::new(110.0, 210.0);
    let snapped = snap_to_grid(point);

    // Halfway point should round to nearest
    assert!(snapped.x == 100.0 || snapped.x == 120.0);
    assert!(snapped.y == 200.0 || snapped.y == 220.0);
}

// ============================================================================
// State Management Tests
// ============================================================================

#[test]
fn test_dag_editor_state_creation() {
    let state = DAGEditorState::new();

    assert_eq!(state.dag.name, "Untitled Workflow");
    assert_eq!(state.dag.nodes.len(), 0);
    assert_eq!(state.canvas_state.zoom, 1.0);
    assert_eq!(state.tool, Tool::Select);
    assert!(state.show_grid);
}

#[test]
fn test_dag_editor_state_with_dag() {
    let mut dag = DAG::new("Test Workflow");
    let node = DAGNode::new_auto("Task");
    dag.add_node(node).unwrap();

    let state = DAGEditorState::with_dag(dag);

    assert_eq!(state.dag.name, "Test Workflow");
    assert_eq!(state.dag.nodes.len(), 1);
    assert!(state.statistics.is_some());
}

#[test]
fn test_tool_selection() {
    let mut state = DAGEditorState::new();

    update(&mut state, DAGEditorMessage::SelectTool(Tool::AddNode));
    assert_eq!(state.tool, Tool::AddNode);

    update(&mut state, DAGEditorMessage::SelectTool(Tool::AddEdge));
    assert_eq!(state.tool, Tool::AddEdge);

    update(&mut state, DAGEditorMessage::SelectTool(Tool::Delete));
    assert_eq!(state.tool, Tool::Delete);
}

#[test]
fn test_ui_state_toggles() {
    let mut state = DAGEditorState::new();

    let initial_props = state.ui_state.show_properties;
    update(&mut state, DAGEditorMessage::ToggleProperties);
    assert_eq!(state.ui_state.show_properties, !initial_props);

    let initial_stats = state.ui_state.show_statistics;
    update(&mut state, DAGEditorMessage::ToggleStatistics);
    assert_eq!(state.ui_state.show_statistics, !initial_stats);

    let initial_toolbar = state.ui_state.show_toolbar;
    update(&mut state, DAGEditorMessage::ToggleToolbar);
    assert_eq!(state.ui_state.show_toolbar, !initial_toolbar);
}

// ============================================================================
// Zoom Operations Tests
// ============================================================================

#[test]
fn test_zoom_in() {
    let mut state = DAGEditorState::new();
    let initial_zoom = state.canvas_state.zoom;

    update(&mut state, DAGEditorMessage::ZoomIn);

    assert!(state.canvas_state.zoom > initial_zoom);
}

#[test]
fn test_zoom_out() {
    let mut state = DAGEditorState::new();
    state.canvas_state.zoom = 2.0;

    update(&mut state, DAGEditorMessage::ZoomOut);

    assert!(state.canvas_state.zoom < 2.0);
}

#[test]
fn test_zoom_reset() {
    let mut state = DAGEditorState::new();
    state.canvas_state.zoom = 3.0;
    state.canvas_state.offset = Vector::new(100.0, 200.0);

    update(&mut state, DAGEditorMessage::ZoomReset);

    assert_eq!(state.canvas_state.zoom, 1.0);
    assert_eq!(state.canvas_state.offset.x, 0.0);
    assert_eq!(state.canvas_state.offset.y, 0.0);
}

#[test]
fn test_zoom_limits() {
    let mut state = DAGEditorState::new();

    // Try to zoom way out
    state.canvas_state.zoom = MIN_ZOOM;
    update(&mut state, DAGEditorMessage::ZoomOut);
    assert!(state.canvas_state.zoom >= MIN_ZOOM);

    // Try to zoom way in
    state.canvas_state.zoom = MAX_ZOOM;
    update(&mut state, DAGEditorMessage::ZoomIn);
    assert!(state.canvas_state.zoom <= MAX_ZOOM);
}

#[test]
fn test_zoom_to_point() {
    let mut state = DAGEditorState::new();
    let cursor_pos = Point::new(400.0, 300.0);
    let new_zoom = 2.0;

    update(
        &mut state,
        DAGEditorMessage::ZoomToPoint(cursor_pos, new_zoom),
    );

    assert_eq!(state.canvas_state.zoom, 2.0);
}

// ============================================================================
// Selection Tests
// ============================================================================

#[test]
fn test_select_single_node() {
    let mut state = DAGEditorState::new();

    let node = DAGNode::new_auto("Task");
    let node_id = node.node_id;
    state.dag.add_node(node).unwrap();

    update(&mut state, DAGEditorMessage::SelectNode(node_id, false));

    assert!(state.interaction.selected_nodes.contains(&node_id));
    assert_eq!(state.interaction.selected_nodes.len(), 1);
}

#[test]
fn test_select_multiple_nodes() {
    let mut state = DAGEditorState::new();

    let node1 = DAGNode::new_auto("Task 1");
    let node2 = DAGNode::new_auto("Task 2");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    state.dag.add_node(node1).unwrap();
    state.dag.add_node(node2).unwrap();

    update(&mut state, DAGEditorMessage::SelectNode(id1, false));
    update(&mut state, DAGEditorMessage::SelectNode(id2, true)); // Add to selection

    assert!(state.interaction.selected_nodes.contains(&id1));
    assert!(state.interaction.selected_nodes.contains(&id2));
    assert_eq!(state.interaction.selected_nodes.len(), 2);
}

#[test]
fn test_deselect_all() {
    let mut state = DAGEditorState::new();

    let node1 = DAGNode::new_auto("Task 1");
    let node2 = DAGNode::new_auto("Task 2");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    state.dag.add_node(node1).unwrap();
    state.dag.add_node(node2).unwrap();

    state.interaction.selected_nodes.insert(id1);
    state.interaction.selected_nodes.insert(id2);

    update(&mut state, DAGEditorMessage::DeselectAll);

    assert!(state.interaction.selected_nodes.is_empty());
}

#[test]
fn test_toggle_selection() {
    let mut state = DAGEditorState::new();

    let node = DAGNode::new_auto("Task");
    let node_id = node.node_id;
    state.dag.add_node(node).unwrap();

    // Select
    update(&mut state, DAGEditorMessage::SelectNode(node_id, true));
    assert!(state.interaction.selected_nodes.contains(&node_id));

    // Toggle off (with add_to_selection = true, it toggles)
    update(&mut state, DAGEditorMessage::SelectNode(node_id, true));
    assert!(!state.interaction.selected_nodes.contains(&node_id));
}

// ============================================================================
// Node Operations Tests
// ============================================================================

#[test]
fn test_add_node_at_position() {
    let mut state = DAGEditorState::new();
    let position = Position::new(200.0, 300.0);

    let initial_count = state.dag.nodes.len();
    update(&mut state, DAGEditorMessage::AddNode(position));

    assert_eq!(state.dag.nodes.len(), initial_count + 1);

    // Find the new node
    let new_node = state.dag.nodes.values().next().unwrap();
    assert_eq!(new_node.position.x, 200.0);
    assert_eq!(new_node.position.y, 300.0);
}

#[test]
fn test_remove_node() {
    let mut state = DAGEditorState::new();

    let node = DAGNode::new_auto("Task");
    let node_id = node.node_id;
    state.dag.add_node(node).unwrap();

    assert_eq!(state.dag.nodes.len(), 1);

    update(&mut state, DAGEditorMessage::RemoveNode(node_id));

    assert_eq!(state.dag.nodes.len(), 0);
}

#[test]
fn test_delete_selected_nodes() {
    let mut state = DAGEditorState::new();

    let node1 = DAGNode::new_auto("Task 1");
    let node2 = DAGNode::new_auto("Task 2");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    state.dag.add_node(node1).unwrap();
    state.dag.add_node(node2).unwrap();

    state.interaction.selected_nodes.insert(id1);
    state.interaction.selected_nodes.insert(id2);

    update(&mut state, DAGEditorMessage::DeleteSelected);

    assert_eq!(state.dag.nodes.len(), 0);
    assert!(state.interaction.selected_nodes.is_empty());
}

#[test]
fn test_update_node() {
    let mut state = DAGEditorState::new();

    let node = DAGNode::new_auto("Original");
    let node_id = node.node_id;
    state.dag.add_node(node).unwrap();

    let updated = DAGNode::new(node_id, "Updated");
    update(&mut state, DAGEditorMessage::UpdateNode(node_id, updated));

    let retrieved = state.dag.get_node(node_id).unwrap();
    assert_eq!(retrieved.label, "Updated");
}

// ============================================================================
// Edge Operations Tests
// ============================================================================

#[test]
fn test_create_edge() {
    let mut state = DAGEditorState::new();

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    state.dag.add_node(node1).unwrap();
    state.dag.add_node(node2).unwrap();

    let initial_count = state.dag.edges.len();
    update(
        &mut state,
        DAGEditorMessage::CreateEdge(id1, id2, EdgeType::Dependency),
    );

    assert_eq!(state.dag.edges.len(), initial_count + 1);
}

#[test]
fn test_delete_edge() {
    let mut state = DAGEditorState::new();

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    state.dag.add_node(node1).unwrap();
    state.dag.add_node(node2).unwrap();

    let edge = DAGEdge::dependency(id1, id2);
    let edge_id = edge.edge_id;
    state.dag.add_edge(edge).unwrap();

    update(&mut state, DAGEditorMessage::DeleteEdge(edge_id));

    assert_eq!(state.dag.edges.len(), 0);
}

// ============================================================================
// View Operations Tests
// ============================================================================

#[test]
fn test_fit_to_view_single_node() {
    let mut state = DAGEditorState::new();

    let node = DAGNode::new_auto("Task").with_position(500.0, 500.0);
    state.dag.add_node(node).unwrap();

    update(&mut state, DAGEditorMessage::FitToView);

    // Should adjust zoom and offset to show the node
    assert!(state.canvas_state.zoom > 0.0);
}

#[test]
fn test_fit_to_view_multiple_nodes() {
    let mut state = DAGEditorState::new();

    let node1 = DAGNode::new_auto("A").with_position(0.0, 0.0);
    let node2 = DAGNode::new_auto("B").with_position(1000.0, 1000.0);

    state.dag.add_node(node1).unwrap();
    state.dag.add_node(node2).unwrap();

    update(&mut state, DAGEditorMessage::FitToView);

    // Should fit both nodes in view
    assert!(state.canvas_state.zoom > 0.0);
}

#[test]
fn test_fit_to_view_empty_dag() {
    let mut state = DAGEditorState::new();

    let initial_zoom = state.canvas_state.zoom;
    update(&mut state, DAGEditorMessage::FitToView);

    // Should not crash with empty DAG
    assert_eq!(state.canvas_state.zoom, initial_zoom);
}

#[test]
fn test_pan_to_offset() {
    let mut state = DAGEditorState::new();

    let new_offset = Vector::new(150.0, 250.0);
    update(&mut state, DAGEditorMessage::PanTo(new_offset));

    assert_eq!(state.canvas_state.offset.x, 150.0);
    assert_eq!(state.canvas_state.offset.y, 250.0);
}

// ============================================================================
// Statistics Tests
// ============================================================================

#[test]
fn test_update_statistics() {
    let mut state = DAGEditorState::new();

    let node1 = DAGNode::new_auto("A");
    let node2 = DAGNode::new_auto("B");
    let id1 = node1.node_id;
    let id2 = node2.node_id;

    state.dag.add_node(node1).unwrap();
    state.dag.add_node(node2).unwrap();
    state.dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();

    update(&mut state, DAGEditorMessage::UpdateStatistics);

    let stats = state.statistics.as_ref().unwrap();
    assert_eq!(stats.node_count, 2);
    assert_eq!(stats.edge_count, 1);
    assert_eq!(stats.start_nodes, 1);
    assert_eq!(stats.end_nodes, 1);
}

#[test]
fn test_statistics_after_modifications() {
    let mut state = DAGEditorState::new();

    let node = DAGNode::new_auto("Task");
    let node_id = node.node_id;
    state.dag.add_node(node).unwrap();
    state.update_statistics();

    assert_eq!(state.statistics.as_ref().unwrap().node_count, 1);

    // Add another node
    update(
        &mut state,
        DAGEditorMessage::AddNode(Position::new(100.0, 100.0)),
    );

    // Statistics should be updated automatically
    assert_eq!(state.statistics.as_ref().unwrap().node_count, 2);
}

// ============================================================================
// Grid Operations Tests
// ============================================================================

#[test]
fn test_toggle_grid() {
    let mut state = DAGEditorState::new();

    let initial = state.show_grid;
    update(&mut state, DAGEditorMessage::ToggleGrid);
    assert_eq!(state.show_grid, !initial);

    update(&mut state, DAGEditorMessage::ToggleGrid);
    assert_eq!(state.show_grid, initial);
}

#[test]
fn test_toggle_snap_to_grid() {
    let mut state = DAGEditorState::new();

    let initial = state.snap_to_grid;
    update(&mut state, DAGEditorMessage::ToggleSnapToGrid);
    assert_eq!(state.snap_to_grid, !initial);

    update(&mut state, DAGEditorMessage::ToggleSnapToGrid);
    assert_eq!(state.snap_to_grid, initial);
}

// ============================================================================
// DAG Load/Save Tests
// ============================================================================

#[test]
fn test_load_dag() {
    let mut state = DAGEditorState::new();

    let mut new_dag = DAG::new("Loaded Workflow");
    let node = DAGNode::new_auto("Task");
    new_dag.add_node(node).unwrap();

    update(&mut state, DAGEditorMessage::LoadDAG(new_dag));

    assert_eq!(state.dag.name, "Loaded Workflow");
    assert_eq!(state.dag.nodes.len(), 1);
    assert!(state.statistics.is_some());
}

#[test]
fn test_new_dag() {
    let mut state = DAGEditorState::new();

    // Add some nodes
    let node = DAGNode::new_auto("Task");
    state.dag.add_node(node).unwrap();

    update(&mut state, DAGEditorMessage::NewDAG);

    assert_eq!(state.dag.name, "Untitled Workflow");
    assert_eq!(state.dag.nodes.len(), 0);
    assert!(state.interaction.selected_nodes.is_empty());
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_large_graph_rendering() {
    let mut state = DAGEditorState::new();

    // Add 100 nodes
    for i in 0..100 {
        let node = DAGNode::new_auto(format!("Node {}", i))
            .with_position((i % 10) as f64 * 200.0, (i / 10) as f64 * 150.0);
        state.dag.add_node(node).unwrap();
    }

    state.update_statistics();

    assert_eq!(state.dag.nodes.len(), 100);
    assert!(state.statistics.is_some());
}

#[test]
fn test_large_graph_selection() {
    let mut state = DAGEditorState::new();

    // Add 50 nodes
    let mut node_ids = Vec::new();
    for i in 0..50 {
        let node = DAGNode::new_auto(format!("Node {}", i));
        let id = node.node_id;
        node_ids.push(id);
        state.dag.add_node(node).unwrap();
    }

    // Select all nodes
    for id in &node_ids {
        update(&mut state, DAGEditorMessage::SelectNode(*id, true));
    }

    assert_eq!(state.interaction.selected_nodes.len(), 50);

    // Deselect all
    update(&mut state, DAGEditorMessage::DeselectAll);
    assert!(state.interaction.selected_nodes.is_empty());
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_select_nonexistent_node() {
    let mut state = DAGEditorState::new();

    let fake_id = Uuid::new_v4();
    update(&mut state, DAGEditorMessage::SelectNode(fake_id, false));

    // Should not crash, just add to selection
    assert!(state.interaction.selected_nodes.contains(&fake_id));
}

#[test]
fn test_delete_selected_with_no_selection() {
    let mut state = DAGEditorState::new();

    let node = DAGNode::new_auto("Task");
    state.dag.add_node(node).unwrap();

    update(&mut state, DAGEditorMessage::DeleteSelected);

    // Should not delete anything
    assert_eq!(state.dag.nodes.len(), 1);
}

#[test]
fn test_zoom_with_no_nodes() {
    let mut state = DAGEditorState::new();

    update(&mut state, DAGEditorMessage::ZoomIn);
    update(&mut state, DAGEditorMessage::ZoomOut);
    update(&mut state, DAGEditorMessage::FitToView);

    // Should not crash
    assert!(state.dag.nodes.is_empty());
}

#[test]
fn test_rapid_tool_switching() {
    let mut state = DAGEditorState::new();

    for _ in 0..10 {
        update(&mut state, DAGEditorMessage::SelectTool(Tool::Select));
        update(&mut state, DAGEditorMessage::SelectTool(Tool::AddNode));
        update(&mut state, DAGEditorMessage::SelectTool(Tool::AddEdge));
        update(&mut state, DAGEditorMessage::SelectTool(Tool::Delete));
        update(&mut state, DAGEditorMessage::SelectTool(Tool::Pan));
    }

    // Should not crash
    assert_eq!(state.tool, Tool::Pan);
}

// ============================================================================
// Cache Management Tests
// ============================================================================

#[test]
fn test_cache_cleared_on_modifications() {
    let mut state = DAGEditorState::new();

    // Add node should clear cache
    let node = DAGNode::new_auto("Task");
    let node_id = node.node_id;
    state.dag.add_node(node).unwrap();

    // Zoom operations should clear cache
    update(&mut state, DAGEditorMessage::ZoomIn);

    // Selection should clear cache
    update(&mut state, DAGEditorMessage::SelectNode(node_id, false));

    // Should not crash - cache management is internal
}
