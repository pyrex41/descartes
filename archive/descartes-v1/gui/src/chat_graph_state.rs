//! Chat Graph View state management
//!
//! Provides data structures for visualizing conversation flow as a tree graph.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Types of nodes in the chat graph
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChatGraphNodeType {
    /// User message
    User,
    /// Assistant response
    Assistant,
    /// Tool call (child of Assistant)
    Tool,
    /// Root of a subagent conversation branch
    SubagentRoot,
    /// Message within a subagent conversation
    SubagentMessage,
}

/// Information about a tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub arguments: String,
    pub output: Option<String>,
    pub status: ToolCallStatus,
}

/// Status of a tool call execution
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolCallStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// Information about a subagent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubagentInfo {
    pub task_id: String,
    pub role: String,
    pub description: String,
    pub status: SubagentStatus,
}

/// Status of a subagent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubagentStatus {
    Running,
    Completed,
    Failed,
}

/// A node in the chat graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGraphNode {
    /// Unique identifier
    pub id: Uuid,
    /// Type of node
    pub node_type: ChatGraphNodeType,
    /// Computed X position (set by layout)
    pub x: f32,
    /// Computed Y position (set by layout)
    pub y: f32,
    /// Short label for display (truncated content)
    pub label: String,
    /// Full content for detail view
    pub content: Option<String>,
    /// Tool call info (if node_type == Tool)
    pub tool_call: Option<ToolCallInfo>,
    /// Subagent info (if node_type == SubagentRoot)
    pub subagent: Option<SubagentInfo>,
    /// Child node IDs
    pub children: Vec<Uuid>,
    /// Parent node ID
    pub parent: Option<Uuid>,
    /// Whether subagent branch is expanded
    pub expanded: bool,
    /// Whether node is receiving live updates
    pub is_live: bool,
    /// Timestamp for ordering
    pub timestamp: i64,
}

impl ChatGraphNode {
    /// Create a new user message node
    pub fn user(content: String) -> Self {
        let label = truncate_label(&content, 50);
        Self {
            id: Uuid::new_v4(),
            node_type: ChatGraphNodeType::User,
            x: 0.0,
            y: 0.0,
            label,
            content: Some(content),
            tool_call: None,
            subagent: None,
            children: Vec::new(),
            parent: None,
            expanded: true,
            is_live: false,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a new assistant message node
    pub fn assistant(content: String) -> Self {
        let label = truncate_label(&content, 50);
        Self {
            id: Uuid::new_v4(),
            node_type: ChatGraphNodeType::Assistant,
            x: 0.0,
            y: 0.0,
            label,
            content: Some(content),
            tool_call: None,
            subagent: None,
            children: Vec::new(),
            parent: None,
            expanded: true,
            is_live: false,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a tool call node
    #[allow(dead_code)]
    pub fn tool(tool_call: ToolCallInfo, parent_id: Uuid) -> Self {
        Self {
            id: Uuid::new_v4(),
            node_type: ChatGraphNodeType::Tool,
            x: 0.0,
            y: 0.0,
            label: tool_call.name.clone(),
            content: None,
            tool_call: Some(tool_call),
            subagent: None,
            children: Vec::new(),
            parent: Some(parent_id),
            expanded: true,
            is_live: false,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a subagent root node
    #[allow(dead_code)]
    pub fn subagent_root(subagent: SubagentInfo, parent_id: Uuid) -> Self {
        let label = format!(
            "{}: {}",
            subagent.role,
            truncate_label(&subagent.description, 30)
        );
        Self {
            id: Uuid::new_v4(),
            node_type: ChatGraphNodeType::SubagentRoot,
            x: 0.0,
            y: 0.0,
            label,
            content: None,
            tool_call: None,
            subagent: Some(subagent),
            children: Vec::new(),
            parent: Some(parent_id),
            expanded: false, // Collapsed by default
            is_live: false,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Get the height of this node type
    pub fn height(&self) -> f32 {
        match self.node_type {
            ChatGraphNodeType::Tool => TOOL_NODE_HEIGHT,
            _ => NODE_HEIGHT,
        }
    }
}

/// Truncate a string for label display
fn truncate_label(s: &str, max_len: usize) -> String {
    let s = s.lines().next().unwrap_or(s); // First line only
    if s.len() > max_len {
        format!("{}...", &s[..max_len])
    } else {
        s.to_string()
    }
}

// Layout constants (matching hl_quick)
pub const NODE_WIDTH: f32 = 200.0;
pub const NODE_HEIGHT: f32 = 60.0;
pub const TOOL_NODE_HEIGHT: f32 = 36.0;
#[allow(dead_code)]
pub const HORIZONTAL_GAP: f32 = 40.0;
pub const VERTICAL_GAP: f32 = 30.0;
pub const BRANCH_INDENT: f32 = 60.0;
pub const PADDING: f32 = 40.0;
pub const NODE_RADIUS: f32 = 8.0;

/// State for the chat graph view
#[derive(Debug, Clone)]
pub struct ChatGraphState {
    /// All nodes in the graph (keyed by ID)
    pub nodes: std::collections::HashMap<Uuid, ChatGraphNode>,
    /// Root node IDs (top-level conversation nodes)
    pub root_nodes: Vec<Uuid>,
    /// Currently selected node
    pub selected_node: Option<Uuid>,
    /// Set of expanded subagent node IDs
    pub expanded_subagents: std::collections::HashSet<Uuid>,
    /// Whether graph view is active (vs linear view)
    pub show_graph_view: bool,
    /// Canvas pan offset
    pub offset: iced::Vector,
    /// Canvas zoom level
    pub zoom: f32,
    /// Computed graph dimensions
    pub graph_width: f32,
    pub graph_height: f32,
}

impl Default for ChatGraphState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChatGraphState {
    pub fn new() -> Self {
        Self {
            nodes: std::collections::HashMap::new(),
            root_nodes: Vec::new(),
            selected_node: None,
            expanded_subagents: std::collections::HashSet::new(),
            show_graph_view: false,
            offset: iced::Vector::new(0.0, 0.0),
            zoom: 1.0,
            graph_width: 800.0,
            graph_height: 600.0,
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: ChatGraphNode) {
        let id = node.id;
        let parent = node.parent;
        self.nodes.insert(id, node);

        if parent.is_none() {
            self.root_nodes.push(id);
        } else if let Some(parent_id) = parent {
            if let Some(parent_node) = self.nodes.get_mut(&parent_id) {
                parent_node.children.push(id);
            }
        }
    }

    /// Get a node by ID
    pub fn get_node(&self, id: Uuid) -> Option<&ChatGraphNode> {
        self.nodes.get(&id)
    }

    /// Get a mutable node by ID
    #[allow(dead_code)]
    pub fn get_node_mut(&mut self, id: Uuid) -> Option<&mut ChatGraphNode> {
        self.nodes.get_mut(&id)
    }

    /// Toggle expansion of a subagent node
    pub fn toggle_subagent(&mut self, node_id: Uuid) {
        if self.expanded_subagents.contains(&node_id) {
            self.expanded_subagents.remove(&node_id);
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.expanded = false;
            }
        } else {
            self.expanded_subagents.insert(node_id);
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.expanded = true;
            }
        }
    }

    /// Clear all nodes
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.root_nodes.clear();
        self.selected_node = None;
    }
}

/// Messages for chat graph operations
#[derive(Debug, Clone)]
pub enum ChatGraphMessage {
    /// Toggle between graph and linear view
    ToggleView,
    /// Select a node (for detail popup)
    SelectNode(Option<Uuid>),
    /// Toggle subagent expansion
    ToggleSubagent(Uuid),
    /// Update node live status
    #[allow(dead_code)]
    SetNodeLive(Uuid, bool),
    /// Pan the canvas
    Pan(iced::Vector),
    /// Zoom the canvas
    #[allow(dead_code)]
    Zoom(f32),
    /// Zoom to point (for scroll wheel)
    ZoomToPoint(iced::Point, f32),
    /// Rebuild graph from chat state
    RebuildGraph,
    /// Scroll to latest node
    #[allow(dead_code)]
    ScrollToLatest,
}

/// Update chat graph state
pub fn update(state: &mut ChatGraphState, message: ChatGraphMessage) {
    match message {
        ChatGraphMessage::ToggleView => {
            state.show_graph_view = !state.show_graph_view;
        }
        ChatGraphMessage::SelectNode(node_id) => {
            state.selected_node = node_id;
        }
        ChatGraphMessage::ToggleSubagent(node_id) => {
            state.toggle_subagent(node_id);
        }
        ChatGraphMessage::SetNodeLive(node_id, is_live) => {
            if let Some(node) = state.nodes.get_mut(&node_id) {
                node.is_live = is_live;
            }
        }
        ChatGraphMessage::Pan(delta) => {
            state.offset = iced::Vector::new(state.offset.x + delta.x, state.offset.y + delta.y);
        }
        ChatGraphMessage::Zoom(zoom) => {
            state.zoom = zoom.clamp(0.1, 5.0);
        }
        ChatGraphMessage::ZoomToPoint(position, new_zoom) => {
            let old_zoom = state.zoom;
            state.zoom = new_zoom.clamp(0.1, 5.0);

            // Adjust offset to keep point stationary
            let world_x = (position.x - state.offset.x) / old_zoom;
            let world_y = (position.y - state.offset.y) / old_zoom;

            state.offset = iced::Vector::new(
                position.x - world_x * state.zoom,
                position.y - world_y * state.zoom,
            );
        }
        ChatGraphMessage::RebuildGraph => {
            // Will be handled in integration phase
        }
        ChatGraphMessage::ScrollToLatest => {
            // Will be implemented in interaction phase
        }
    }
}
