use descartes_core::dag::{DAGEdge, DAGNode, EdgeType, Position};
use descartes_gui::dag_canvas_interactions::*;
/// Tests for DAG Editor Drag-and-Drop Interactions
///
/// This test suite validates all drag-and-drop functionality including:
/// - Node dragging (single and multi-select)
/// - Edge creation via drag
/// - Box selection
/// - Canvas panning
/// - Zoom to cursor
/// - Undo/redo operations
use descartes_gui::dag_editor::*;
use iced::mouse::ScrollDelta;
use iced::{keyboard, mouse, Point};
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a test DAG editor state with some nodes
fn create_test_state() -> DAGEditorState {
    let mut state = DAGEditorState::new();

    // Add some test nodes
    let node1 = DAGNode::new_auto("Node 1").with_position(100.0, 100.0);
    let node2 = DAGNode::new_auto("Node 2").with_position(300.0, 100.0);
    let node3 = DAGNode::new_auto("Node 3").with_position(200.0, 300.0);

    state.dag.add_node(node1).unwrap();
    state.dag.add_node(node2).unwrap();
    state.dag.add_node(node3).unwrap();

    state.update_statistics();
    state
}

/// Get all node IDs from the DAG
fn get_node_ids(state: &DAGEditorState) -> Vec<Uuid> {
    state.dag.nodes.keys().copied().collect()
}

// ============================================================================
// Node Dragging Tests
// ============================================================================

#[test]
fn test_single_node_drag() {
    let mut state = create_test_state();
    let node_ids = get_node_ids(&state);
    let node_id = node_ids[0];

    // Get initial position
    let initial_pos = state.dag.get_node(node_id).unwrap().position;

    // Click on node (offset by 30 pixels from node position to be inside the node)
    let click_pos = Point::new(initial_pos.x as f32 + 30.0, initial_pos.y as f32 + 30.0);
    let result = handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        click_pos,
        keyboard::Modifiers::default(),
    );

    assert!(matches!(result, Some(InteractionResult::NodeDragStarted)));
    assert!(state.interaction.selected_nodes.contains(&node_id));

    // Drag to new position
    let drag_pos = Point::new(200.0, 200.0);
    handle_mouse_move(&mut state, drag_pos);

    // Release
    handle_mouse_release(&mut state, mouse::Button::Left, drag_pos);

    // Check that node moved
    let final_pos = state.dag.get_node(node_id).unwrap().position;
    assert_ne!(initial_pos.x, final_pos.x);
    assert_ne!(initial_pos.y, final_pos.y);
}

#[test]
fn test_multi_node_drag() {
    let mut state = create_test_state();
    let node_ids = get_node_ids(&state);

    // Select first node
    let click_pos1 = Point::new(130.0, 130.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        click_pos1,
        keyboard::Modifiers::default(),
    );

    // Ctrl+click to add second node to selection
    let click_pos2 = Point::new(330.0, 130.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        click_pos2,
        keyboard::Modifiers::CTRL,
    );

    assert_eq!(state.interaction.selected_nodes.len(), 2);

    // Get initial positions
    let initial_positions: Vec<_> = node_ids
        .iter()
        .filter(|id| state.interaction.selected_nodes.contains(id))
        .map(|id| state.dag.get_node(*id).unwrap().position)
        .collect();

    // Drag
    let drag_pos = Point::new(400.0, 200.0);
    handle_mouse_move(&mut state, drag_pos);

    // Release
    handle_mouse_release(&mut state, mouse::Button::Left, drag_pos);

    // Check that both nodes moved
    let final_positions: Vec<_> = node_ids
        .iter()
        .filter(|id| state.interaction.selected_nodes.contains(id))
        .map(|id| state.dag.get_node(*id).unwrap().position)
        .collect();

    for (initial, final_pos) in initial_positions.iter().zip(final_positions.iter()) {
        assert_ne!(initial.x, final_pos.x);
        assert_ne!(initial.y, final_pos.y);
    }
}

#[test]
fn test_drag_with_snap_to_grid() {
    let mut state = create_test_state();
    state.snap_to_grid = true;
    let node_ids = get_node_ids(&state);
    let node_id = node_ids[0];

    // Click on node
    let click_pos = Point::new(130.0, 130.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        click_pos,
        keyboard::Modifiers::default(),
    );

    // Drag to position that's not on grid
    let drag_pos = Point::new(237.5, 183.7);
    handle_mouse_move(&mut state, drag_pos);
    handle_mouse_release(&mut state, mouse::Button::Left, drag_pos);

    // Check that position was snapped to grid
    let final_pos = state.dag.get_node(node_id).unwrap().position;
    let grid_size = 20.0;
    assert_eq!(final_pos.x % grid_size, 0.0);
    assert_eq!(final_pos.y % grid_size, 0.0);
}

// ============================================================================
// Edge Creation Tests
// ============================================================================

#[test]
fn test_edge_creation_via_drag() {
    let mut state = create_test_state();
    state.tool = Tool::AddEdge;

    // Find the nodes by their positions
    let from_node = state
        .dag
        .nodes
        .values()
        .find(|n| n.position.x == 100.0 && n.position.y == 100.0)
        .unwrap()
        .node_id;
    let to_node = state
        .dag
        .nodes
        .values()
        .find(|n| n.position.x == 300.0 && n.position.y == 100.0)
        .unwrap()
        .node_id;

    // Click on source node (at 100, 100)
    let from_pos = Point::new(130.0, 130.0);
    let result = handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        from_pos,
        keyboard::Modifiers::default(),
    );

    assert!(matches!(
        result,
        Some(InteractionResult::EdgeCreationStarted(_))
    ));
    assert!(state.extended_interaction.edge_creation.is_some());

    // Drag to target node (at 300, 100)
    let to_pos = Point::new(330.0, 130.0);
    handle_mouse_move(&mut state, to_pos);

    // Release on target node
    let initial_edge_count = state.dag.edges.len();
    handle_mouse_release(&mut state, mouse::Button::Left, to_pos);

    // Check that edge was created
    assert_eq!(state.dag.edges.len(), initial_edge_count + 1);

    // Verify edge connects the right nodes
    let edge = state.dag.edges.values().next().unwrap();
    assert_eq!(edge.from_node_id, from_node);
    assert_eq!(edge.to_node_id, to_node);
}

#[test]
fn test_edge_creation_cycle_prevention() {
    let mut state = create_test_state();
    state.tool = Tool::AddEdge;

    // Find the nodes by their positions
    let node_a = state
        .dag
        .nodes
        .values()
        .find(|n| n.position.x == 100.0 && n.position.y == 100.0)
        .unwrap()
        .node_id;
    let node_b = state
        .dag
        .nodes
        .values()
        .find(|n| n.position.x == 300.0 && n.position.y == 100.0)
        .unwrap()
        .node_id;

    // Create edge A -> B
    let edge = DAGEdge::new(node_a, node_b, EdgeType::Dependency);
    state.dag.add_edge(edge).unwrap();

    // Try to create edge B -> A (would create cycle)
    let from_pos = Point::new(330.0, 130.0); // Node B at (300, 100)
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        from_pos,
        keyboard::Modifiers::default(),
    );

    let to_pos = Point::new(130.0, 130.0); // Node A at (100, 100)
    handle_mouse_move(&mut state, to_pos);

    let initial_edge_count = state.dag.edges.len();
    let result = handle_mouse_release(&mut state, mouse::Button::Left, to_pos);

    // Edge should not be created (would create cycle)
    assert!(matches!(
        result,
        Some(InteractionResult::EdgeCreationFailed(_))
    ));
    assert_eq!(state.dag.edges.len(), initial_edge_count);
}

#[test]
fn test_edge_creation_self_loop_prevention() {
    let mut state = create_test_state();
    state.tool = Tool::AddEdge;
    let node_ids = get_node_ids(&state);

    // Try to create edge from node to itself
    let pos = Point::new(130.0, 130.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        pos,
        keyboard::Modifiers::default(),
    );

    handle_mouse_move(&mut state, pos);

    let initial_edge_count = state.dag.edges.len();
    handle_mouse_release(&mut state, mouse::Button::Left, pos);

    // Edge should not be created (self-loop)
    assert_eq!(state.dag.edges.len(), initial_edge_count);
}

// ============================================================================
// Selection Tests
// ============================================================================

#[test]
fn test_box_selection() {
    let mut state = create_test_state();

    // Start box selection in empty area
    let start_pos = Point::new(50.0, 50.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        start_pos,
        keyboard::Modifiers::default(),
    );

    assert!(state.extended_interaction.box_selection.is_some());

    // Drag to encompass multiple nodes
    let end_pos = Point::new(350.0, 250.0);
    handle_mouse_move(&mut state, end_pos);

    // Release
    handle_mouse_release(&mut state, mouse::Button::Left, end_pos);

    // Check that nodes within box were selected
    assert!(!state.interaction.selected_nodes.is_empty());
}

#[test]
fn test_ctrl_click_multi_select() {
    let mut state = create_test_state();

    // Find the nodes by their positions
    let node_at_100_100 = state
        .dag
        .nodes
        .values()
        .find(|n| n.position.x == 100.0 && n.position.y == 100.0)
        .unwrap()
        .node_id;
    let node_at_300_100 = state
        .dag
        .nodes
        .values()
        .find(|n| n.position.x == 300.0 && n.position.y == 100.0)
        .unwrap()
        .node_id;

    // Click first node (at 100, 100)
    let pos1 = Point::new(130.0, 130.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        pos1,
        keyboard::Modifiers::default(),
    );

    assert_eq!(state.interaction.selected_nodes.len(), 1);
    assert!(state.interaction.selected_nodes.contains(&node_at_100_100));

    // Ctrl+click second node (at 300, 100)
    let pos2 = Point::new(330.0, 130.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        pos2,
        keyboard::Modifiers::CTRL,
    );

    assert_eq!(state.interaction.selected_nodes.len(), 2);

    // Ctrl+click first node again to deselect
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        pos1,
        keyboard::Modifiers::CTRL,
    );

    assert_eq!(state.interaction.selected_nodes.len(), 1);
    assert!(!state.interaction.selected_nodes.contains(&node_at_100_100));
    assert!(state.interaction.selected_nodes.contains(&node_at_300_100));
}

#[test]
fn test_select_all_keyboard_shortcut() {
    let mut state = create_test_state();

    // Press Ctrl+A
    let result = handle_key_press(
        &mut state,
        keyboard::Key::Character("a".into()),
        keyboard::Modifiers::CTRL,
    );

    assert!(matches!(result, Some(InteractionResult::AllNodesSelected)));
    assert_eq!(
        state.interaction.selected_nodes.len(),
        state.dag.nodes.len()
    );
}

// ============================================================================
// Panning Tests
// ============================================================================

#[test]
fn test_middle_mouse_pan() {
    let mut state = create_test_state();

    let initial_offset = state.canvas_state.offset;

    // Press middle mouse button
    let start_pos = Point::new(400.0, 300.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Middle,
        start_pos,
        keyboard::Modifiers::default(),
    );

    assert!(state.interaction.pan_state.is_some());

    // Drag
    let drag_pos = Point::new(300.0, 200.0);
    handle_mouse_move(&mut state, drag_pos);

    // Check that offset changed
    assert_ne!(state.canvas_state.offset.x, initial_offset.x);
    assert_ne!(state.canvas_state.offset.y, initial_offset.y);

    // Release
    handle_mouse_release(&mut state, mouse::Button::Middle, drag_pos);

    assert!(state.interaction.pan_state.is_none());
}

#[test]
fn test_pan_tool() {
    let mut state = create_test_state();
    state.tool = Tool::Pan;

    let initial_offset = state.canvas_state.offset;

    // Click and drag with pan tool
    let start_pos = Point::new(400.0, 300.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        start_pos,
        keyboard::Modifiers::default(),
    );

    let drag_pos = Point::new(300.0, 200.0);
    handle_mouse_move(&mut state, drag_pos);

    // Offset should change
    assert_ne!(state.canvas_state.offset.x, initial_offset.x);
    assert_ne!(state.canvas_state.offset.y, initial_offset.y);
}

// ============================================================================
// Zoom Tests
// ============================================================================

#[test]
fn test_mouse_wheel_zoom() {
    let mut state = create_test_state();

    let initial_zoom = state.canvas_state.zoom;
    let cursor_pos = Point::new(400.0, 300.0);

    // Scroll up to zoom in
    let delta = ScrollDelta::Lines { x: 0.0, y: 1.0 };
    handle_mouse_scroll(&mut state, delta, cursor_pos);

    assert!(state.canvas_state.zoom > initial_zoom);

    // Scroll down to zoom out
    let delta = ScrollDelta::Lines { x: 0.0, y: -1.0 };
    handle_mouse_scroll(&mut state, delta, cursor_pos);

    assert!(state.canvas_state.zoom < initial_zoom + 0.2); // Account for both operations
}

#[test]
fn test_zoom_limits() {
    let mut state = create_test_state();
    let cursor_pos = Point::new(400.0, 300.0);

    // Try to zoom way out
    for _ in 0..100 {
        let delta = ScrollDelta::Lines { x: 0.0, y: -10.0 };
        handle_mouse_scroll(&mut state, delta, cursor_pos);
    }

    assert!(state.canvas_state.zoom >= MIN_ZOOM);

    // Try to zoom way in
    for _ in 0..100 {
        let delta = ScrollDelta::Lines { x: 0.0, y: 10.0 };
        handle_mouse_scroll(&mut state, delta, cursor_pos);
    }

    assert!(state.canvas_state.zoom <= MAX_ZOOM);
}

#[test]
fn test_zoom_to_cursor_position() {
    let mut state = create_test_state();

    let cursor_pos = Point::new(300.0, 200.0);
    let world_pos_before = screen_to_world(cursor_pos, &state.canvas_state);

    // Zoom in
    let delta = ScrollDelta::Lines { x: 0.0, y: 1.0 };
    handle_mouse_scroll(&mut state, delta, cursor_pos);

    let world_pos_after = screen_to_world(cursor_pos, &state.canvas_state);

    // World position under cursor should remain approximately the same
    let diff_x = (world_pos_before.x - world_pos_after.x).abs();
    let diff_y = (world_pos_before.y - world_pos_after.y).abs();

    assert!(
        diff_x < 5.0,
        "Cursor X position drifted too much: {}",
        diff_x
    );
    assert!(
        diff_y < 5.0,
        "Cursor Y position drifted too much: {}",
        diff_y
    );
}

// ============================================================================
// Undo/Redo Tests
// ============================================================================

#[test]
fn test_undo_node_addition() {
    let mut state = create_test_state();

    let initial_count = state.dag.nodes.len();

    // Add a node
    let position = Position::new(400.0, 400.0);
    update(
        &mut state,
        DAGEditorMessage::AddNode(position),
    );

    assert_eq!(state.dag.nodes.len(), initial_count + 1);

    // Undo
    update(&mut state, DAGEditorMessage::Undo);

    assert_eq!(state.dag.nodes.len(), initial_count);
}

#[test]
fn test_undo_node_deletion() {
    let mut state = create_test_state();
    let node_ids = get_node_ids(&state);
    let node_id = node_ids[0];

    let initial_count = state.dag.nodes.len();

    // Delete a node
    update(&mut state, DAGEditorMessage::RemoveNode(node_id));

    assert_eq!(state.dag.nodes.len(), initial_count - 1);

    // Undo
    update(&mut state, DAGEditorMessage::Undo);

    assert_eq!(state.dag.nodes.len(), initial_count);
    assert!(state.dag.get_node(node_id).is_some());
}

#[test]
fn test_undo_edge_creation() {
    let mut state = create_test_state();
    let node_ids = get_node_ids(&state);

    let initial_count = state.dag.edges.len();

    // Create an edge
    update(
        &mut state,
        DAGEditorMessage::CreateEdge(node_ids[0], node_ids[1], EdgeType::Dependency),
    );

    assert_eq!(state.dag.edges.len(), initial_count + 1);

    // Undo
    update(&mut state, DAGEditorMessage::Undo);

    assert_eq!(state.dag.edges.len(), initial_count);
}

#[test]
fn test_redo_operations() {
    let mut state = create_test_state();
    let node_ids = get_node_ids(&state);

    let initial_count = state.dag.nodes.len();

    // Add a node
    let position = Position::new(400.0, 400.0);
    update(
        &mut state,
        DAGEditorMessage::AddNode(position),
    );

    // Undo
    update(&mut state, DAGEditorMessage::Undo);
    assert_eq!(state.dag.nodes.len(), initial_count);

    // Redo
    update(&mut state, DAGEditorMessage::Redo);
    assert_eq!(state.dag.nodes.len(), initial_count + 1);
}

#[test]
fn test_keyboard_undo_redo_shortcuts() {
    let mut state = create_test_state();

    // Add a node
    let position = Position::new(400.0, 400.0);
    update(
        &mut state,
        DAGEditorMessage::AddNode(position),
    );

    let count_after_add = state.dag.nodes.len();

    // Press Ctrl+Z to undo
    let result = handle_key_press(
        &mut state,
        keyboard::Key::Character("z".into()),
        keyboard::Modifiers::CTRL,
    );

    assert!(matches!(result, Some(InteractionResult::UndoRequested)));

    // Press Ctrl+Shift+Z to redo
    let result = handle_key_press(
        &mut state,
        keyboard::Key::Character("z".into()),
        keyboard::Modifiers::CTRL | keyboard::Modifiers::SHIFT,
    );

    assert!(matches!(result, Some(InteractionResult::RedoRequested)));
}

// ============================================================================
// Keyboard Interaction Tests
// ============================================================================

#[test]
fn test_delete_key() {
    let mut state = create_test_state();
    let node_ids = get_node_ids(&state);

    // Select a node
    state.interaction.selected_nodes.insert(node_ids[0]);

    let initial_count = state.dag.nodes.len();

    // Press Delete key
    handle_key_press(
        &mut state,
        keyboard::Key::Named(keyboard::key::Named::Delete),
        keyboard::Modifiers::default(),
    );

    assert_eq!(state.dag.nodes.len(), initial_count - 1);
}

#[test]
fn test_escape_cancels_operations() {
    let mut state = create_test_state();

    // Start edge creation
    state.extended_interaction.edge_creation = Some(EdgeCreation {
        from_node: Uuid::new_v4(),
        current_pos: Point::new(0.0, 0.0),
        hover_target: None,
        edge_type: EdgeType::Dependency,
    });

    // Press Escape
    let result = handle_key_press(
        &mut state,
        keyboard::Key::Named(keyboard::key::Named::Escape),
        keyboard::Modifiers::default(),
    );

    assert!(matches!(
        result,
        Some(InteractionResult::OperationCancelled)
    ));
    assert!(state.extended_interaction.edge_creation.is_none());
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_drag_large_selection() {
    let mut state = DAGEditorState::new();

    // Add 50 nodes
    for i in 0..50 {
        let node = DAGNode::new_auto(format!("Node {}", i))
            .with_position((i % 10) as f64 * 100.0, (i / 10) as f64 * 100.0);
        state.dag.add_node(node).unwrap();
    }

    // Select all nodes
    state.interaction.selected_nodes = state.dag.nodes.keys().copied().collect();

    // Start drag
    let start_pos = Point::new(100.0, 100.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        start_pos,
        keyboard::Modifiers::default(),
    );

    // Perform drag operation
    let drag_pos = Point::new(200.0, 200.0);
    handle_mouse_move(&mut state, drag_pos);

    // All selected nodes should have moved
    assert!(state.interaction.drag_state.is_some());
    assert_eq!(
        state.interaction.drag_state.as_ref().unwrap().nodes.len(),
        50
    );
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_click_on_empty_canvas() {
    let mut state = create_test_state();

    // Click on empty area
    let pos = Point::new(1000.0, 1000.0);
    handle_mouse_press(
        &mut state,
        mouse::Button::Left,
        pos,
        keyboard::Modifiers::default(),
    );

    // Should start box selection
    assert!(state.extended_interaction.box_selection.is_some());
}

#[test]
fn test_node_at_position_detection() {
    let state = create_test_state();

    // Point clearly inside first node
    let inside = Point::new(130.0, 130.0);
    assert!(find_node_at_position(&state, inside).is_some());

    // Point clearly outside all nodes
    let outside = Point::new(1000.0, 1000.0);
    assert!(find_node_at_position(&state, outside).is_none());
}

#[test]
fn test_cycle_detection_complex() {
    let mut state = DAGEditorState::new();

    // Create nodes A, B, C, D
    let node_a = DAGNode::new_auto("A").with_position(100.0, 100.0);
    let node_b = DAGNode::new_auto("B").with_position(300.0, 100.0);
    let node_c = DAGNode::new_auto("C").with_position(300.0, 300.0);
    let node_d = DAGNode::new_auto("D").with_position(100.0, 300.0);

    let id_a = node_a.node_id;
    let id_b = node_b.node_id;
    let id_c = node_c.node_id;
    let id_d = node_d.node_id;

    state.dag.add_node(node_a).unwrap();
    state.dag.add_node(node_b).unwrap();
    state.dag.add_node(node_c).unwrap();
    state.dag.add_node(node_d).unwrap();

    // Create edges: A -> B -> C -> D
    state
        .dag
        .add_edge(DAGEdge::new(id_a, id_b, EdgeType::Dependency))
        .unwrap();
    state
        .dag
        .add_edge(DAGEdge::new(id_b, id_c, EdgeType::Dependency))
        .unwrap();
    state
        .dag
        .add_edge(DAGEdge::new(id_c, id_d, EdgeType::Dependency))
        .unwrap();

    // Verify D -> A would create cycle
    assert!(would_create_cycle(&state.dag, id_d, id_a));

    // Verify A -> D would NOT create cycle (would be fine)
    assert!(!would_create_cycle(&state.dag, id_a, id_d));
}
