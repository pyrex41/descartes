//! History Graph Layout Algorithm
//!
//! Computes x/y positions for nodes in a time-based graph layout.
//! - X-axis: Time (left = older, right = newer)
//! - Y-axis: Agent hierarchy (each agent gets a horizontal swim lane)
//! Uses parent_event_id for causality edges.

use crate::history_graph_state::{
    HistoryGraphState, HistoryNodeType, AGENT_INDENT, HORIZONTAL_GAP, NODE_HEIGHT, NODE_WIDTH,
    PADDING, VERTICAL_GAP,
};
use uuid::Uuid;

/// Layout result containing computed dimensions
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub width: f32,
    pub height: f32,
}

/// Compute layout positions for all visible nodes in the history graph
pub fn compute_layout(state: &mut HistoryGraphState) -> LayoutResult {
    if state.nodes.is_empty() {
        return LayoutResult {
            width: 800.0,
            height: 600.0,
        };
    }

    // Calculate time scale
    let time_range = (state.max_timestamp - state.min_timestamp).max(1) as f32;
    let available_width = 2000.0; // Base width for time axis
    let time_scale = available_width / time_range;

    // Calculate Y positions for each agent (swim lanes)
    let mut agent_y: std::collections::HashMap<String, f32> = std::collections::HashMap::new();
    let mut current_y = PADDING;

    for agent_id in &state.agent_order {
        if !state.collapsed_agents.contains(agent_id) {
            agent_y.insert(agent_id.clone(), current_y);
            current_y += NODE_HEIGHT + AGENT_INDENT;
        }
    }

    let mut max_x: f32 = 0.0;
    let mut max_y: f32 = current_y;

    // Layout all visible nodes
    let visible_node_ids: Vec<Uuid> = state
        .visible_nodes()
        .iter()
        .map(|n| n.id)
        .collect();

    for node_id in visible_node_ids {
        if let Some(node) = state.nodes.get(&node_id) {
            let agent_id = node.agent_id.clone();
            let timestamp = node.timestamp;
            let node_type = node.node_type;

            // Compute X from timestamp
            let time_offset = (timestamp - state.min_timestamp) as f32;
            let x = PADDING + time_offset * time_scale;

            // Compute Y from agent lane
            let base_y = agent_y.get(&agent_id).copied().unwrap_or(PADDING);

            // Slight Y offset for different event types to reduce overlap
            let type_offset = match node_type {
                HistoryNodeType::AgentStart => 0.0,
                HistoryNodeType::Thought => 5.0,
                HistoryNodeType::Action => 10.0,
                HistoryNodeType::ToolUse => 15.0,
                HistoryNodeType::StateChange => 20.0,
                HistoryNodeType::Communication => 25.0,
                HistoryNodeType::Decision => 30.0,
                HistoryNodeType::Error => 35.0,
                HistoryNodeType::System => 40.0,
                HistoryNodeType::AgentComplete => 45.0,
            };

            let y = base_y + type_offset;

            // Update tracking
            max_x = max_x.max(x + NODE_WIDTH);
            max_y = max_y.max(y + NODE_HEIGHT + VERTICAL_GAP);

            // Store position
            if let Some(node) = state.nodes.get_mut(&node_id) {
                node.x = x;
                node.y = y;
            }
        }
    }

    let result = LayoutResult {
        width: max_x + PADDING,
        height: max_y + PADDING,
    };

    state.graph_width = result.width;
    state.graph_height = result.height;

    result
}

/// Alternative layout: Tree-style layout with time on X, depth on Y
/// This shows causality more clearly for dense event streams
pub fn compute_tree_layout(state: &mut HistoryGraphState) -> LayoutResult {
    if state.nodes.is_empty() {
        return LayoutResult {
            width: 800.0,
            height: 600.0,
        };
    }

    let mut current_y = PADDING;
    let mut max_x: f32 = 0.0;

    // Process agent roots in order
    let agent_order = state.agent_order.clone();
    for agent_id in agent_order {
        if state.collapsed_agents.contains(&agent_id) {
            continue;
        }

        if let Some(&root_id) = state.agent_roots.get(&agent_id) {
            // Layout this agent's tree
            layout_agent_tree(state, root_id, 0, PADDING, &mut current_y, &mut max_x);
            current_y += AGENT_INDENT; // Gap between agents
        }
    }

    let result = LayoutResult {
        width: max_x + PADDING,
        height: current_y + PADDING,
    };

    state.graph_width = result.width;
    state.graph_height = result.height;

    result
}

/// Recursively layout a node and its children in tree style
fn layout_agent_tree(
    state: &mut HistoryGraphState,
    node_id: Uuid,
    depth: usize,
    base_x: f32,
    current_y: &mut f32,
    max_x: &mut f32,
) {
    // Check if node is visible (within timeline)
    let (node_height, children, _timestamp, is_visible) = {
        if let Some(node) = state.nodes.get(&node_id) {
            let visible = node.timestamp <= state.timeline_position && state.filter.matches(node);
            (node.height(), node.children.clone(), node.timestamp, visible)
        } else {
            return;
        }
    };

    if !is_visible {
        return;
    }

    // Compute position
    let x = base_x + (depth as f32 * HORIZONTAL_GAP);
    let y = *current_y;

    // Update node position
    if let Some(node) = state.nodes.get_mut(&node_id) {
        node.x = x;
        node.y = y;
    }

    // Update tracking
    *max_x = max_x.max(x + NODE_WIDTH);
    *current_y += node_height + VERTICAL_GAP;

    // Sort children by timestamp for consistent ordering
    let mut sorted_children = children;
    sorted_children.sort_by_key(|&child_id| {
        state.nodes.get(&child_id).map(|n| n.timestamp).unwrap_or(0)
    });

    // Layout children
    for child_id in sorted_children {
        layout_agent_tree(state, child_id, depth + 1, base_x, current_y, max_x);
    }
}

/// Find the node closest to a given position (for hit testing)
pub fn find_node_at_position(
    state: &HistoryGraphState,
    x: f32,
    y: f32,
) -> Option<Uuid> {
    // Account for zoom and pan
    let world_x = (x - state.offset.x) / state.zoom;
    let world_y = (y - state.offset.y) / state.zoom;

    state
        .visible_nodes()
        .iter()
        .find(|node| {
            world_x >= node.x
                && world_x <= node.x + NODE_WIDTH
                && world_y >= node.y
                && world_y <= node.y + node.height()
        })
        .map(|n| n.id)
}

/// Find the node with the highest timestamp (most recent)
pub fn find_latest_node(state: &HistoryGraphState) -> Option<Uuid> {
    state
        .nodes
        .values()
        .max_by_key(|n| n.timestamp)
        .map(|n| n.id)
}

/// Compute scroll offset to show a specific node
pub fn compute_scroll_to_node(
    state: &HistoryGraphState,
    node_id: Uuid,
    viewport_width: f32,
    viewport_height: f32,
) -> iced::Vector {
    if let Some(node) = state.nodes.get(&node_id) {
        // Center the node in the viewport
        let target_x = node.x * state.zoom - (viewport_width * 0.5);
        let target_y = node.y * state.zoom - (viewport_height * 0.5);
        iced::Vector::new(-target_x.max(0.0), -target_y.max(0.0))
    } else {
        state.offset
    }
}

/// Get the edge paths between nodes (for drawing connections)
pub fn compute_edges(state: &HistoryGraphState) -> Vec<(iced::Point, iced::Point)> {
    let mut edges = Vec::new();

    for node in state.visible_nodes() {
        if let Some(parent_id) = node.parent {
            if let Some(parent) = state.nodes.get(&parent_id) {
                // Check if parent is visible
                if parent.timestamp <= state.timeline_position && state.filter.matches(parent) {
                    // Edge from parent right side to child left side
                    let parent_x = parent.x + NODE_WIDTH;
                    let parent_y = parent.y + parent.height() / 2.0;
                    let child_x = node.x;
                    let child_y = node.y + node.height() / 2.0;

                    edges.push((
                        iced::Point::new(parent_x, parent_y),
                        iced::Point::new(child_x, child_y),
                    ));
                }
            }
        }
    }

    edges
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::history_graph_state::HistoryGraphNode;

    #[test]
    fn test_empty_layout() {
        let mut state = HistoryGraphState::new();
        let result = compute_layout(&mut state);
        assert_eq!(result.width, 800.0);
        assert_eq!(result.height, 600.0);
    }

    #[test]
    fn test_single_agent_layout() {
        let mut state = HistoryGraphState::new();

        // Add a root node
        let root = HistoryGraphNode::agent_start("agent-1", 1000);
        let root_id = root.id;
        state.nodes.insert(root_id, root);
        state.agent_roots.insert("agent-1".to_string(), root_id);
        state.agent_order.push("agent-1".to_string());
        state.min_timestamp = 1000;
        state.max_timestamp = 1000;
        state.timeline_position = 1000;

        let result = compute_layout(&mut state);

        assert!(result.width > 0.0);
        assert!(result.height > 0.0);

        let node = state.get_node(root_id).unwrap();
        assert!(node.x >= PADDING);
        assert!(node.y >= PADDING);
    }
}
