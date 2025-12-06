//! Chat Graph Layout Algorithm
//!
//! Computes x/y positions for nodes in a tree layout.
//! Uses depth-first traversal with depth-based indentation.

use crate::chat_graph_state::{
    ChatGraphState, BRANCH_INDENT, NODE_WIDTH, PADDING, VERTICAL_GAP,
};
use uuid::Uuid;

/// Layout result containing computed dimensions
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub width: f32,
    pub height: f32,
}

/// Compute layout positions for all nodes in the graph
pub fn compute_layout(state: &mut ChatGraphState) -> LayoutResult {
    let mut current_y = PADDING;
    let mut max_x: f32 = 0.0;

    // Process root nodes in order
    let root_ids: Vec<Uuid> = state.root_nodes.clone();
    for root_id in root_ids {
        layout_node_recursive(state, root_id, 0, PADDING, &mut current_y, &mut max_x);
    }

    let result = LayoutResult {
        width: max_x + PADDING,
        height: current_y + PADDING,
    };

    state.graph_width = result.width;
    state.graph_height = result.height;

    result
}

/// Recursively layout a node and its children
fn layout_node_recursive(
    state: &mut ChatGraphState,
    node_id: Uuid,
    depth: usize,
    base_x: f32,
    current_y: &mut f32,
    max_x: &mut f32,
) {
    // Get node info first to avoid borrow issues
    let (node_height, is_expanded, children) = {
        if let Some(node) = state.nodes.get(&node_id) {
            (node.height(), node.expanded, node.children.clone())
        } else {
            return;
        }
    };

    // Compute position
    let x = base_x + (depth as f32 * BRANCH_INDENT);
    let y = *current_y;

    // Update node position
    if let Some(node) = state.nodes.get_mut(&node_id) {
        node.x = x;
        node.y = y;
    }

    // Update tracking
    *max_x = max_x.max(x + NODE_WIDTH);
    *current_y += node_height + VERTICAL_GAP;

    // Layout children if expanded
    if is_expanded {
        for child_id in children {
            layout_node_recursive(state, child_id, depth + 1, base_x, current_y, max_x);
        }
    }
}

/// Find the node with the highest Y position (most recent)
#[allow(dead_code)]
pub fn find_latest_node(state: &ChatGraphState) -> Option<Uuid> {
    state
        .nodes
        .values()
        .max_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
        .map(|n| n.id)
}

/// Compute scroll offset to show a specific node
#[allow(dead_code)]
pub fn compute_scroll_to_node(
    state: &ChatGraphState,
    node_id: Uuid,
    viewport_height: f32,
) -> iced::Vector {
    if let Some(node) = state.nodes.get(&node_id) {
        // Position node at 2/3 down the viewport
        let target_y = node.y - (viewport_height * 0.66);
        iced::Vector::new(state.offset.x, -target_y.max(0.0))
    } else {
        state.offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_graph_state::{ChatGraphNode, PADDING};

    #[test]
    fn test_single_node_layout() {
        let mut state = ChatGraphState::new();
        let node = ChatGraphNode::user("Hello".to_string());
        state.add_node(node);

        let result = compute_layout(&mut state);

        assert!(result.width > 0.0);
        assert!(result.height > 0.0);

        let root_id = state.root_nodes[0];
        let node = state.get_node(root_id).unwrap();
        assert_eq!(node.x, PADDING);
        assert_eq!(node.y, PADDING);
    }

    #[test]
    fn test_parent_child_layout() {
        let mut state = ChatGraphState::new();

        let user = ChatGraphNode::user("Hello".to_string());
        let user_id = user.id;
        state.add_node(user);

        let mut assistant = ChatGraphNode::assistant("Hi there".to_string());
        assistant.parent = Some(user_id);
        state.add_node(assistant);

        compute_layout(&mut state);

        // Child should be indented
        let user_node = state.nodes.values().find(|n| n.parent.is_none()).unwrap();
        let child_node = state.nodes.values().find(|n| n.parent.is_some()).unwrap();

        assert!(child_node.x > user_node.x);
        assert!(child_node.y > user_node.y);
    }
}
