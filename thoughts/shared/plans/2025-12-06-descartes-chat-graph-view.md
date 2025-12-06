# Chat Graph View Implementation Plan

## Overview

Add a Chat Graph View to the Descartes GUI that visualizes conversation flow as an interactive tree structure. This provides an alternative to the linear chat view, allowing users to observe agent conversations including tool calls, subagent spawning, and live streaming content. The implementation mirrors the hl_quick prototype (SolidJS/SVG) but uses Iced's Canvas widget.

## Current State Analysis

### What Exists:
- **DAG Editor** (`dag_editor.rs`) - Canvas-based graph editor for task workflows with full pan/zoom, node/edge rendering
- **Chat View** (`chat_view.rs`) - Linear chat interface displaying messages as a list
- **Chat State** (`chat_state.rs`) - Simple state with `messages: Vec<ChatMessageEntry>` containing only role/content/timestamp
- **Interaction Patterns** (`dag_canvas_interactions.rs`) - Comprehensive mouse/keyboard handling, hit testing, coordinate transforms

### What's Missing:
- No tree/hierarchy structure for conversations
- No tool call tracking in chat messages
- No subagent/spawned task tracking
- No graph visualization of chat flow
- No live streaming node indicators

### Key Discoveries:
- Canvas Program trait pattern at `dag_editor.rs:876-935`
- Coordinate transforms at `dag_editor.rs:1138-1175`
- Conditional rendering pattern at `dag_editor.rs:1012` (zoom > 0.3)
- Theme colors at `theme.rs:34-91` including `NODE_*` and `EDGE_*`
- ViewMode enum at `main.rs:104-115` for adding new views

## Desired End State

After implementation:

1. **Toggle Button** in chat view header switches between linear and graph views
2. **Graph View** renders conversation as a tree with:
   - User messages as cyan-bordered nodes
   - Assistant messages as default nodes
   - Tool calls as smaller dashed-border nodes (children of assistant)
   - Subagent branches as collapsible purple-bordered subtrees
3. **Auto-layout** positions nodes in depth-first tree order
4. **Interactions**: Click node to show detail popup, expand/collapse subagents
5. **Live Updates**: Pulsing indicator on nodes receiving streaming content
6. **Pan/Zoom**: Standard canvas controls for navigation

### Verification:
- Send a message with tool calls → see assistant node with tool children
- Toggle view → switches between linear and graph
- Click node → popup shows full content
- Zoom out → labels hidden for performance
- Large conversations (50+ messages) render smoothly

## What We're NOT Doing

- **NOT** implementing drag-and-drop node repositioning (auto-layout only)
- **NOT** implementing edge creation/deletion (read-only visualization)
- **NOT** implementing search/filter functionality
- **NOT** implementing graph export (PNG/SVG)
- **NOT** implementing minimap for large graphs
- **NOT** persisting view preference across sessions
- **NOT** implementing keyboard navigation between nodes

## Implementation Approach

1. **Phase 1**: Create data model and state management (`chat_graph_state.rs`)
2. **Phase 2**: Implement tree layout algorithm (`chat_graph_layout.rs`)
3. **Phase 3**: Build canvas renderer (`chat_graph_view.rs`)
4. **Phase 4**: Add interactions (click, expand/collapse, pan/zoom)
5. **Phase 5**: Integrate with main.rs and add view toggle
6. **Phase 6**: Add live updates and streaming indicators

---

## Phase 1: Data Model and State Management

### Overview
Create the ChatGraphNode data structure and ChatGraphState to manage the graph view state. This phase establishes the foundation for all subsequent phases.

### Changes Required:

#### 1.1 Create ChatGraphNode Types

**File**: `descartes/gui/src/chat_graph_state.rs` (NEW)
**Changes**: Create new module with data types

```rust
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
    pub fn subagent_root(subagent: SubagentInfo, parent_id: Uuid) -> Self {
        let label = format!("{}: {}", subagent.role, truncate_label(&subagent.description, 30));
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
pub const HORIZONTAL_GAP: f32 = 40.0;
pub const VERTICAL_GAP: f32 = 30.0;
pub const BRANCH_INDENT: f32 = 60.0;
pub const PADDING: f32 = 40.0;
pub const NODE_RADIUS: f32 = 8.0;

/// State for the chat graph view
#[derive(Debug, Clone, Default)]
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
    SetNodeLive(Uuid, bool),
    /// Pan the canvas
    Pan(iced::Vector),
    /// Zoom the canvas
    Zoom(f32),
    /// Zoom to point (for scroll wheel)
    ZoomToPoint(iced::Point, f32),
    /// Rebuild graph from chat state
    RebuildGraph,
    /// Scroll to latest node
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
            state.offset = iced::Vector::new(
                state.offset.x + delta.x,
                state.offset.y + delta.y,
            );
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
```

#### 1.2 Register Module

**File**: `descartes/gui/src/main.rs`
**Changes**: Add module declaration after line 29

```rust
mod chat_graph_state;
```

### Success Criteria:

#### Automated Verification:
- [x] Code compiles: `cd descartes/gui && cargo check`
- [x] No clippy warnings: `cd descartes/gui && cargo clippy`
- [x] Module is accessible from main.rs

#### Manual Verification:
- [ ] Data structures match the hl_quick GraphNode interface
- [ ] All node types are represented
- [ ] Constants match hl_quick layout values

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the review was successful before proceeding to the next phase.

---

## Phase 2: Tree Layout Algorithm

### Overview
Implement the layout algorithm that computes x/y positions for all nodes in a depth-first tree traversal.

### Changes Required:

#### 2.1 Create Layout Module

**File**: `descartes/gui/src/chat_graph_layout.rs` (NEW)
**Changes**: Create layout computation logic

```rust
//! Chat Graph Layout Algorithm
//!
//! Computes x/y positions for nodes in a tree layout.
//! Uses depth-first traversal with depth-based indentation.

use crate::chat_graph_state::{
    ChatGraphNode, ChatGraphState, BRANCH_INDENT, NODE_HEIGHT, NODE_WIDTH, PADDING,
    TOOL_NODE_HEIGHT, VERTICAL_GAP,
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
pub fn find_latest_node(state: &ChatGraphState) -> Option<Uuid> {
    state
        .nodes
        .values()
        .max_by(|a, b| a.y.partial_cmp(&b.y).unwrap_or(std::cmp::Ordering::Equal))
        .map(|n| n.id)
}

/// Compute scroll offset to show a specific node
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
    use crate::chat_graph_state::ChatGraphNode;

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

        let mut user = ChatGraphNode::user("Hello".to_string());
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
```

#### 2.2 Register Layout Module

**File**: `descartes/gui/src/main.rs`
**Changes**: Add module declaration

```rust
mod chat_graph_layout;
```

### Success Criteria:

#### Automated Verification:
- [ ] Code compiles: `cd descartes/gui && cargo check`
- [ ] Tests pass: `cd descartes/gui && cargo test chat_graph_layout`
- [ ] No clippy warnings

#### Manual Verification:
- [ ] Layout algorithm matches hl_quick behavior (depth-based indentation)
- [ ] Tool nodes use smaller height
- [ ] Collapsed subagents hide their children

**Implementation Note**: After completing this phase, pause for human confirmation before proceeding.

---

## Phase 3: Canvas Renderer

### Overview
Build the Iced Canvas-based renderer that draws nodes, edges, and handles the visual presentation.

### Changes Required:

#### 3.1 Create Graph View Module

**File**: `descartes/gui/src/chat_graph_view.rs` (NEW)
**Changes**: Create canvas rendering component

```rust
//! Chat Graph View - Canvas-based conversation visualization
//!
//! Renders chat conversations as an interactive tree graph using Iced Canvas.

use crate::chat_graph_layout::{compute_layout, find_latest_node};
use crate::chat_graph_state::{
    ChatGraphMessage, ChatGraphNode, ChatGraphNodeType, ChatGraphState, NODE_HEIGHT, NODE_RADIUS,
    NODE_WIDTH, TOOL_NODE_HEIGHT,
};
use crate::theme::{colors, container_styles, fonts};
use iced::mouse::{self, Cursor};
use iced::widget::canvas::{Cache, Frame, Geometry, Path, Stroke, Text};
use iced::widget::{button, canvas, column, container, row, scrollable, text, Space};
use iced::{
    alignment::Vertical, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
};
use uuid::Uuid;

/// Canvas program for rendering the chat graph
pub struct ChatGraphCanvas {
    state: ChatGraphState,
    cache: Cache,
}

impl ChatGraphCanvas {
    pub fn new(state: ChatGraphState) -> Self {
        Self {
            state,
            cache: Cache::new(),
        }
    }
}

impl<Message> canvas::Program<Message> for ChatGraphCanvas {
    type State = ();

    fn draw(
        &self,
        _interaction_state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());

        // Draw background
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            colors::BACKGROUND,
        );

        // Draw edges first (behind nodes)
        for root_id in &self.state.root_nodes {
            self.draw_edges_recursive(&mut frame, *root_id);
        }

        // Draw nodes on top
        for root_id in &self.state.root_nodes {
            self.draw_node_recursive(&mut frame, *root_id);
        }

        vec![frame.into_geometry()]
    }
}

impl ChatGraphCanvas {
    /// Draw a node and its visible children recursively
    fn draw_node_recursive(&self, frame: &mut Frame, node_id: Uuid) {
        if let Some(node) = self.state.nodes.get(&node_id) {
            self.draw_node(frame, node);

            // Draw children if expanded
            if node.expanded {
                for child_id in &node.children {
                    self.draw_node_recursive(frame, *child_id);
                }
            }
        }
    }

    /// Draw a single node
    fn draw_node(&self, frame: &mut Frame, node: &ChatGraphNode) {
        let zoom = self.state.zoom;
        let offset = self.state.offset;

        // Transform to screen coordinates
        let x = node.x * zoom + offset.x;
        let y = node.y * zoom + offset.y;
        let width = NODE_WIDTH * zoom;
        let height = node.height() * zoom;
        let radius = NODE_RADIUS * zoom;

        // Skip if outside viewport (simple culling)
        // TODO: Get actual bounds from draw context

        // Get colors based on node type and state
        let (fill_color, border_color) = self.node_colors(node);
        let is_selected = self.state.selected_node == Some(node.id);

        // Draw node rectangle
        let node_rect = Path::rounded_rectangle(
            Point::new(x, y),
            Size::new(width, height),
            radius.into(),
        );

        frame.fill(&node_rect, fill_color);

        // Border
        let border_width = if is_selected { 3.0 } else { 1.5 };
        let actual_border = if is_selected { colors::PRIMARY } else { border_color };

        // Dashed border for tool nodes
        if node.node_type == ChatGraphNodeType::Tool {
            // Iced doesn't support dashed strokes natively, use double stroke
            frame.stroke(
                &node_rect,
                Stroke::default()
                    .with_color(actual_border)
                    .with_width(border_width),
            );
        } else {
            frame.stroke(
                &node_rect,
                Stroke::default()
                    .with_color(actual_border)
                    .with_width(border_width),
            );
        }

        // Draw label (only if zoom > 0.3)
        if zoom > 0.3 {
            let font_size = (12.0 * zoom).max(8.0).min(18.0);
            let label_x = x + 12.0 * zoom;
            let label_y = y + height / 2.0;

            frame.fill_text(Text {
                content: node.label.clone(),
                position: Point::new(label_x, label_y),
                color: colors::TEXT_PRIMARY,
                size: font_size.into(),
                horizontal_alignment: iced::alignment::Horizontal::Left,
                vertical_alignment: iced::alignment::Vertical::Center,
                ..Default::default()
            });
        }

        // Draw live indicator (pulsing dot)
        if node.is_live && zoom > 0.3 {
            let indicator_x = x + width - 12.0 * zoom;
            let indicator_y = y + 12.0 * zoom;
            let indicator_radius = 4.0 * zoom;

            frame.fill(
                &Path::circle(Point::new(indicator_x, indicator_y), indicator_radius),
                colors::WARNING, // Yellow for live
            );
        }

        // Draw expand/collapse indicator for subagent roots
        if node.node_type == ChatGraphNodeType::SubagentRoot && !node.children.is_empty() {
            let btn_x = x + 8.0 * zoom;
            let btn_y = y + height / 2.0 - 8.0 * zoom;
            let btn_size = 16.0 * zoom;

            let btn_rect = Path::rounded_rectangle(
                Point::new(btn_x, btn_y),
                Size::new(btn_size, btn_size),
                (3.0 * zoom).into(),
            );

            frame.fill(&btn_rect, colors::SURFACE_HOVER);
            frame.stroke(
                &btn_rect,
                Stroke::default()
                    .with_color(colors::BORDER)
                    .with_width(1.0),
            );

            // +/- icon
            if zoom > 0.3 {
                let icon = if node.expanded { "−" } else { "+" };
                frame.fill_text(Text {
                    content: icon.to_string(),
                    position: Point::new(btn_x + btn_size / 2.0, btn_y + btn_size / 2.0),
                    color: colors::TEXT_MUTED,
                    size: (12.0 * zoom).into(),
                    horizontal_alignment: iced::alignment::Horizontal::Center,
                    vertical_alignment: iced::alignment::Vertical::Center,
                    ..Default::default()
                });
            }
        }
    }

    /// Get fill and border colors for a node
    fn node_colors(&self, node: &ChatGraphNode) -> (Color, Color) {
        match node.node_type {
            ChatGraphNodeType::User => (colors::PRIMARY_DIM, colors::PRIMARY),
            ChatGraphNodeType::Assistant => (colors::SURFACE, colors::BORDER),
            ChatGraphNodeType::Tool => (colors::SURFACE, colors::TEXT_MUTED),
            ChatGraphNodeType::SubagentRoot => (colors::SURFACE, colors::INFO),
            ChatGraphNodeType::SubagentMessage => (colors::SURFACE, Color::from_rgb(0.4, 0.3, 0.6)),
        }
    }

    /// Draw edges from a node to its children
    fn draw_edges_recursive(&self, frame: &mut Frame, node_id: Uuid) {
        if let Some(node) = self.state.nodes.get(&node_id) {
            if node.expanded {
                for child_id in &node.children {
                    self.draw_edge(frame, node, *child_id);
                    self.draw_edges_recursive(frame, *child_id);
                }
            }
        }
    }

    /// Draw a bezier curve edge from parent to child
    fn draw_edge(&self, frame: &mut Frame, parent: &ChatGraphNode, child_id: Uuid) {
        if let Some(child) = self.state.nodes.get(&child_id) {
            let zoom = self.state.zoom;
            let offset = self.state.offset;

            // Start from bottom center of parent
            let start_x = (parent.x + NODE_WIDTH / 2.0) * zoom + offset.x;
            let start_y = (parent.y + parent.height()) * zoom + offset.y;

            // End at top center of child
            let end_x = (child.x + NODE_WIDTH / 2.0) * zoom + offset.x;
            let end_y = child.y * zoom + offset.y;

            // Edge color based on child type
            let edge_color = match child.node_type {
                ChatGraphNodeType::Tool => colors::TEXT_MUTED,
                ChatGraphNodeType::SubagentRoot | ChatGraphNodeType::SubagentMessage => colors::INFO,
                _ => colors::BORDER,
            };

            // Draw curved line using quadratic bezier approximation
            // Iced Path doesn't have bezier, so use line for now
            // TODO: Implement proper bezier curve
            let mid_y = (start_y + end_y) / 2.0;

            let edge_path = Path::new(|builder| {
                builder.move_to(Point::new(start_x, start_y));
                builder.line_to(Point::new(start_x, mid_y));
                builder.line_to(Point::new(end_x, mid_y));
                builder.line_to(Point::new(end_x, end_y));
            });

            let edge_width = if child.is_live { 2.0 } else { 1.5 };

            frame.stroke(
                &edge_path,
                Stroke::default()
                    .with_color(edge_color)
                    .with_width(edge_width),
            );
        }
    }
}

/// Coordinate transformation: screen to world
pub fn screen_to_world(point: Point, state: &ChatGraphState) -> Point {
    Point::new(
        (point.x - state.offset.x) / state.zoom,
        (point.y - state.offset.y) / state.zoom,
    )
}

/// Coordinate transformation: world to screen
pub fn world_to_screen(point: Point, state: &ChatGraphState) -> Point {
    Point::new(
        point.x * state.zoom + state.offset.x,
        point.y * state.zoom + state.offset.y,
    )
}

/// Check if a point is inside a node
pub fn point_in_node(point: Point, node: &ChatGraphNode, state: &ChatGraphState) -> bool {
    let screen_pos = world_to_screen(Point::new(node.x, node.y), state);
    let width = NODE_WIDTH * state.zoom;
    let height = node.height() * state.zoom;

    point.x >= screen_pos.x
        && point.x <= screen_pos.x + width
        && point.y >= screen_pos.y
        && point.y <= screen_pos.y + height
}

/// Find the node at a given screen position
pub fn find_node_at_position(state: &ChatGraphState, position: Point) -> Option<Uuid> {
    // Check in reverse order (top-most first)
    for node in state.nodes.values() {
        if point_in_node(position, node, state) {
            return Some(node.id);
        }
    }
    None
}

/// Render the complete chat graph view
pub fn view(state: &ChatGraphState, chat_state: &crate::chat_state::ChatState) -> Element<ChatGraphMessage> {
    if !state.show_graph_view {
        // Return empty if not showing graph view
        return Space::with_height(0).into();
    }

    let canvas = canvas::Canvas::new(ChatGraphCanvas::new(state.clone()))
        .width(Length::Fill)
        .height(Length::Fill);

    // Wrap in container
    let graph_container = container(canvas)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(container_styles::panel);

    // Node detail popup (if selected)
    let popup = if let Some(node_id) = state.selected_node {
        if let Some(node) = state.get_node(node_id) {
            view_node_detail(node)
        } else {
            container(Space::with_height(0))
        }
    } else {
        container(Space::with_height(0))
    };

    // Stack popup over graph
    column![
        graph_container,
    ]
    .into()
}

/// Render node detail popup
fn view_node_detail(node: &ChatGraphNode) -> Element<ChatGraphMessage> {
    let type_label = match node.node_type {
        ChatGraphNodeType::User => "User Message",
        ChatGraphNodeType::Assistant => "Assistant",
        ChatGraphNodeType::Tool => "Tool Call",
        ChatGraphNodeType::SubagentRoot => "Subagent",
        ChatGraphNodeType::SubagentMessage => "Subagent Message",
    };

    let type_color = match node.node_type {
        ChatGraphNodeType::User => colors::PRIMARY,
        ChatGraphNodeType::Assistant => colors::SUCCESS,
        ChatGraphNodeType::Tool => colors::WARNING,
        ChatGraphNodeType::SubagentRoot | ChatGraphNodeType::SubagentMessage => colors::INFO,
    };

    let content = if let Some(ref tool) = node.tool_call {
        format!(
            "Tool: {}\n\nInput:\n{}\n\nOutput:\n{}",
            tool.name,
            tool.arguments,
            tool.output.as_deref().unwrap_or("(pending)")
        )
    } else if let Some(ref subagent) = node.subagent {
        format!(
            "Task: {}\nRole: {}\nStatus: {:?}",
            subagent.description, subagent.role, subagent.status
        )
    } else {
        node.content.clone().unwrap_or_default()
    };

    let close_btn = button(text("×").size(18))
        .on_press(ChatGraphMessage::SelectNode(None))
        .padding([4, 8])
        .style(crate::theme::button_styles::icon);

    let popup_content = column![
        row![
            container(text(type_label).size(10).color(type_color))
                .padding([3, 8])
                .style(container_styles::badge_success),
            Space::with_width(Length::Fill),
            close_btn,
        ]
        .align_y(Vertical::Center),
        Space::with_height(12),
        scrollable(
            text(&content)
                .size(13)
                .font(fonts::MONO)
                .color(colors::TEXT_PRIMARY)
        )
        .height(Length::Fill),
    ]
    .spacing(0)
    .padding(16);

    container(popup_content)
        .width(400)
        .height(300)
        .style(container_styles::panel)
        .into()
}
```

#### 3.2 Register View Module

**File**: `descartes/gui/src/main.rs`
**Changes**: Add module declaration

```rust
mod chat_graph_view;
```

### Success Criteria:

#### Automated Verification:
- [ ] Code compiles: `cd descartes/gui && cargo check`
- [ ] No clippy warnings

#### Manual Verification:
- [ ] Nodes render with correct colors by type
- [ ] Edges connect parent nodes to children
- [ ] Labels visible when zoomed in, hidden when zoomed out
- [ ] Selected node has highlighted border

**Implementation Note**: After completing this phase, pause for human confirmation before proceeding.

---

## Phase 4: Interactions

### Overview
Add mouse interactions: click to select, click expand button, pan/zoom canvas.

### Changes Required:

#### 4.1 Add Canvas Event Handling

**File**: `descartes/gui/src/chat_graph_view.rs`
**Changes**: Implement mouse/keyboard event handling in the canvas program

Add to the `canvas::Program` implementation:

```rust
impl canvas::Program<ChatGraphMessage> for ChatGraphCanvas {
    type State = InteractionState;

    fn update(
        &self,
        interaction: &mut Self::State,
        event: canvas::Event,
        bounds: Rectangle,
        cursor: Cursor,
    ) -> (canvas::event::Status, Option<ChatGraphMessage>) {
        let cursor_position = cursor.position_in(bounds);

        match event {
            canvas::Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)) => {
                if let Some(pos) = cursor_position {
                    // Check if clicking on a node
                    if let Some(node_id) = find_node_at_position(&self.state, pos) {
                        // Check if clicking expand button on subagent
                        if let Some(node) = self.state.nodes.get(&node_id) {
                            if node.node_type == ChatGraphNodeType::SubagentRoot {
                                // Check if within expand button bounds
                                let btn_bounds = expand_button_bounds(node, &self.state);
                                if pos.x >= btn_bounds.x
                                    && pos.x <= btn_bounds.x + btn_bounds.width
                                    && pos.y >= btn_bounds.y
                                    && pos.y <= btn_bounds.y + btn_bounds.height
                                {
                                    return (
                                        canvas::event::Status::Captured,
                                        Some(ChatGraphMessage::ToggleSubagent(node_id)),
                                    );
                                }
                            }
                        }
                        return (
                            canvas::event::Status::Captured,
                            Some(ChatGraphMessage::SelectNode(Some(node_id))),
                        );
                    } else {
                        // Clicking on empty space - deselect and start pan
                        interaction.panning = true;
                        interaction.pan_start = pos;
                        return (
                            canvas::event::Status::Captured,
                            Some(ChatGraphMessage::SelectNode(None)),
                        );
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)) => {
                interaction.panning = false;
            }
            canvas::Event::Mouse(mouse::Event::CursorMoved { .. }) => {
                if interaction.panning {
                    if let Some(pos) = cursor_position {
                        let delta = Vector::new(
                            pos.x - interaction.pan_start.x,
                            pos.y - interaction.pan_start.y,
                        );
                        interaction.pan_start = pos;
                        return (
                            canvas::event::Status::Captured,
                            Some(ChatGraphMessage::Pan(delta)),
                        );
                    }
                }
            }
            canvas::Event::Mouse(mouse::Event::WheelScrolled { delta }) => {
                if let Some(pos) = cursor_position {
                    let zoom_delta = match delta {
                        mouse::ScrollDelta::Lines { y, .. } => y * 0.1,
                        mouse::ScrollDelta::Pixels { y, .. } => y * 0.001,
                    };
                    let new_zoom = (self.state.zoom + zoom_delta).clamp(0.1, 5.0);
                    return (
                        canvas::event::Status::Captured,
                        Some(ChatGraphMessage::ZoomToPoint(pos, new_zoom)),
                    );
                }
            }
            _ => {}
        }

        (canvas::event::Status::Ignored, None)
    }

    fn draw(
        &self,
        _interaction: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        // ... existing draw implementation
    }
}

/// Interaction state for the canvas
#[derive(Debug, Clone, Default)]
pub struct InteractionState {
    pub panning: bool,
    pub pan_start: Point,
}

/// Calculate expand button bounds for a subagent node
fn expand_button_bounds(node: &ChatGraphNode, state: &ChatGraphState) -> Rectangle {
    let zoom = state.zoom;
    let screen_pos = world_to_screen(Point::new(node.x, node.y), state);
    let height = node.height() * zoom;

    Rectangle::new(
        Point::new(
            screen_pos.x + 8.0 * zoom,
            screen_pos.y + height / 2.0 - 8.0 * zoom,
        ),
        Size::new(16.0 * zoom, 16.0 * zoom),
    )
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Code compiles: `cd descartes/gui && cargo check`

#### Manual Verification:
- [ ] Click on node → selects and shows popup
- [ ] Click on empty space → deselects
- [ ] Drag on empty space → pans canvas
- [ ] Scroll wheel → zooms in/out centered on cursor
- [ ] Click +/- button on subagent → toggles expansion

**Implementation Note**: After completing this phase, pause for human confirmation before proceeding.

---

## Phase 5: Main Integration

### Overview
Integrate the chat graph view with main.rs, add view toggle button, and build graph from chat state.

### Changes Required:

#### 5.1 Add Graph State to Main

**File**: `descartes/gui/src/main.rs`
**Changes**: Add ChatGraphState to DescartesGui struct (around line 91)

```rust
/// Chat graph view state
chat_graph_state: chat_graph_state::ChatGraphState,
```

Initialize in `DescartesGui::new()` (around line 176):
```rust
chat_graph_state: chat_graph_state::ChatGraphState::new(),
```

#### 5.2 Add Message Variant

**File**: `descartes/gui/src/main.rs`
**Changes**: Add to Message enum (around line 144)

```rust
/// Chat graph view message
ChatGraph(chat_graph_state::ChatGraphMessage),
```

#### 5.3 Add Message Handler

**File**: `descartes/gui/src/main.rs`
**Changes**: Add handler in update() match (around line 427)

```rust
Message::ChatGraph(msg) => {
    // Handle rebuild specially
    if matches!(msg, chat_graph_state::ChatGraphMessage::RebuildGraph) {
        self.rebuild_chat_graph();
    }
    chat_graph_state::update(&mut self.chat_graph_state, msg);
    iced::Task::none()
}
```

#### 5.4 Add Graph Builder Method

**File**: `descartes/gui/src/main.rs`
**Changes**: Add method to DescartesGui impl

```rust
/// Rebuild the chat graph from current chat state
fn rebuild_chat_graph(&mut self) {
    use chat_graph_state::{ChatGraphNode, ChatGraphNodeType};

    self.chat_graph_state.clear();

    let mut last_node_id: Option<uuid::Uuid> = None;

    for msg in &self.chat_state.messages {
        let node = match msg.role {
            chat_state::ChatRole::User => ChatGraphNode::user(msg.content.clone()),
            chat_state::ChatRole::Assistant => ChatGraphNode::assistant(msg.content.clone()),
            chat_state::ChatRole::System => continue, // Skip system messages
        };

        let node_id = node.id;
        self.chat_graph_state.add_node(node);
        last_node_id = Some(node_id);
    }

    // Compute layout
    chat_graph_layout::compute_layout(&mut self.chat_graph_state);
}
```

#### 5.5 Update Chat View with Toggle

**File**: `descartes/gui/src/chat_view.rs`
**Changes**: Add toggle button to header and conditional rendering

Add import at top:
```rust
use crate::chat_graph_state::ChatGraphState;
```

Modify `view` function signature:
```rust
pub fn view(state: &ChatState, graph_state: &ChatGraphState) -> Element<ChatViewMessage>
```

Add toggle button in controls_row (around line 78):
```rust
button(
    text(if graph_state.show_graph_view { "≡ List" } else { "◇ Graph" })
        .size(12)
        .font(fonts::MONO)
        .color(colors::TEXT_SECONDARY)
)
.on_press(ChatViewMessage::ToggleGraphView)
.padding([4, 8])
.style(button_styles::secondary),
Space::with_width(8),
```

Add new message variant:
```rust
pub enum ChatMessage {
    // ... existing variants
    /// Toggle between linear and graph view
    ToggleGraphView,
}
```

#### 5.6 Update Chat View Call

**File**: `descartes/gui/src/main.rs`
**Changes**: Update view_chat() to pass graph state and handle toggle

```rust
fn view_chat(&self) -> Element<Message> {
    if self.chat_graph_state.show_graph_view {
        // Show graph view
        chat_graph_view::view(&self.chat_graph_state, &self.chat_state)
            .map(Message::ChatGraph)
    } else {
        // Show linear view
        chat_view::view(&self.chat_state, &self.chat_graph_state)
            .map(|msg| {
                match msg {
                    chat_state::ChatMessage::ToggleGraphView => {
                        Message::ChatGraph(chat_graph_state::ChatGraphMessage::ToggleView)
                    }
                    other => Message::Chat(other)
                }
            })
    }
}
```

#### 5.7 Rebuild Graph on Chat Changes

**File**: `descartes/gui/src/main.rs`
**Changes**: Trigger rebuild when chat state changes

In the `Message::Chat` handler, after updating state:
```rust
// Rebuild graph if visible
if self.chat_graph_state.show_graph_view {
    self.rebuild_chat_graph();
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Code compiles: `cd descartes/gui && cargo check`
- [ ] No warnings

#### Manual Verification:
- [ ] Toggle button appears in chat header
- [ ] Clicking toggle switches between views
- [ ] Graph shows messages from chat history
- [ ] Sending a new message updates graph
- [ ] State persists when switching views

**Implementation Note**: After completing this phase, pause for human confirmation before proceeding.

---

## Phase 6: Live Updates and Polish

### Overview
Add live streaming indicators, auto-scroll to latest, and visual polish.

### Changes Required:

#### 6.1 Add Streaming State Tracking

**File**: `descartes/gui/src/chat_state.rs`
**Changes**: Add streaming indicator

```rust
pub struct ChatState {
    // ... existing fields
    /// Currently streaming response content
    pub streaming_content: Option<String>,
    /// ID of the streaming node (if any)
    pub streaming_node_id: Option<Uuid>,
}
```

#### 6.2 Update Graph on Streaming

**File**: `descartes/gui/src/main.rs`
**Changes**: Update live node during streaming

In chat message handler when streaming starts:
```rust
// Mark node as live
if let Some(node_id) = self.chat_graph_state.selected_node {
    chat_graph_state::update(
        &mut self.chat_graph_state,
        chat_graph_state::ChatGraphMessage::SetNodeLive(node_id, true),
    );
}
```

When streaming ends:
```rust
// Mark node as no longer live
if let Some(node_id) = streaming_node_id {
    chat_graph_state::update(
        &mut self.chat_graph_state,
        chat_graph_state::ChatGraphMessage::SetNodeLive(node_id, false),
    );
}
```

#### 6.3 Auto-scroll to Latest

**File**: `descartes/gui/src/chat_graph_view.rs`
**Changes**: Implement scroll to latest

Add to the view function:
```rust
// Auto-scroll when new nodes are added
if let Some(latest_id) = find_latest_node(state) {
    if Some(latest_id) != state.last_scrolled_to {
        let new_offset = compute_scroll_to_node(state, latest_id, viewport_height);
        // Return message to update offset
    }
}
```

#### 6.4 Add Pulsing Animation for Live Nodes

**File**: `descartes/gui/src/chat_graph_view.rs`
**Changes**: Use Iced's time subscription for animation

Note: Iced doesn't have built-in animation support like CSS. For true pulsing, we'd need to:
1. Add a time subscription that ticks every 100ms
2. Track animation phase (0.0 to 1.0)
3. Modulate the live indicator opacity/size

For simplicity, we'll use a static bright indicator initially.

#### 6.5 Visual Polish

**File**: `descartes/gui/src/chat_graph_view.rs`
**Changes**: Improve visual appearance

- Add subtle shadow effect to nodes (darker fill below)
- Improve edge routing to avoid overlapping nodes
- Add node type icons (◆ user, ◎ assistant, ⚙ tool)

### Success Criteria:

#### Automated Verification:
- [ ] Code compiles: `cd descartes/gui && cargo check`
- [ ] Full test suite passes: `cd descartes && cargo test`

#### Manual Verification:
- [ ] Live nodes show yellow indicator
- [ ] Graph auto-scrolls to show new messages
- [ ] Node icons differentiate types at a glance
- [ ] Large conversations (50+ messages) render smoothly
- [ ] Pan/zoom feels responsive

**Implementation Note**: This is the final phase. After completion, do comprehensive manual testing.

---

## Testing Strategy

### Unit Tests:

**File**: `descartes/gui/src/chat_graph_layout.rs`
- Test single node layout
- Test parent-child indentation
- Test collapsed subagent hides children
- Test find_latest_node

**File**: `descartes/gui/src/chat_graph_state.rs`
- Test node creation helpers
- Test add_node parent-child linking
- Test toggle_subagent state changes

### Integration Tests:
- Build graph from chat history with various message patterns
- Verify layout dimensions for known configurations

### Manual Testing Steps:
1. Start with empty chat, send messages, verify nodes appear
2. Toggle between linear and graph views multiple times
3. Click nodes to verify detail popup content
4. Expand/collapse subagent branches
5. Zoom in/out and verify label visibility threshold
6. Pan around a large graph
7. Verify scroll-to-latest works when adding messages
8. Test with 50+ message conversation for performance

## Performance Considerations

1. **Viewport Culling**: Only render nodes visible in current viewport
2. **Conditional Detail Rendering**: Hide labels/icons when zoom < 0.3
3. **Cache Usage**: Leverage Iced's Canvas cache for static content
4. **Lazy Layout**: Only recompute layout when nodes change
5. **Node Limit**: For 100+ nodes, consider pagination or virtualization

## Migration Notes

- No data migration needed - graph is built from existing ChatState
- Feature is purely additive, no breaking changes
- Default view remains linear chat (graph view opt-in via toggle)

## References

- Research document: `thoughts/shared/research/2025-12-06-descartes-chat-graph-view.md`
- hl_quick prototype: `hl_quick.xml:10900-11930`
- DAG Editor patterns: `descartes/gui/src/dag_editor.rs`
- Canvas interactions: `descartes/gui/src/dag_canvas_interactions.rs`
- Theme definitions: `descartes/gui/src/theme.rs`
