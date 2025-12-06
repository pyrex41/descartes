---
date: 2025-12-06T17:55:48+0000
researcher: reuben
git_commit: cdf0687d77c6304e246052112698df964db06f92
branch: master
repository: cap
topic: "Chat Graph View Implementation Research for Descartes GUI"
tags: [research, codebase, descartes, gui, iced, canvas, chat-graph, visualization]
status: complete
last_updated: 2025-12-06
last_updated_by: reuben
---

# Research: Chat Graph View Implementation for Descartes GUI

**Date**: 2025-12-06T17:55:48+0000
**Researcher**: reuben
**Git Commit**: cdf0687d77c6304e246052112698df964db06f92
**Branch**: master
**Repository**: cap

## Research Question

How to add a Chat Graph View to Descartes GUI for observing conversation flow, modeled after the hl_quick prototype implementation (SVG-based, SolidJS), adapted for Iced framework with Canvas widget.

## Summary

This research documents the existing patterns in Descartes GUI that can be leveraged to build a new Chat Graph View component. The key findings are:

1. **Canvas Rendering**: The DAG Editor (`dag_editor.rs`) provides comprehensive canvas patterns including node/edge rendering, coordinate transformations, and zoom/pan support that can be directly adapted.

2. **Interaction Handling**: The `dag_canvas_interactions.rs` module provides a complete interaction system with mouse/keyboard handling, hit testing, and state management.

3. **Theming**: The theme module provides a "Space-age hacker" aesthetic with neon colors, font definitions, and reusable container/button styles.

4. **Data Structures**: No existing tree/hierarchy structures for conversations exist - the `ChatGraphNode` type must be created from scratch, but can draw from existing patterns like `ToolCall`, `TranscriptEntry`, and `AgentHistoryEvent`.

5. **Integration Pattern**: The `main.rs` shows the ViewMode enum pattern for adding new views and the message routing architecture.

## Detailed Findings

### 1. hl_quick Reference Implementation

The hl_quick prototype at `hl_quick.xml:10900-11930` provides the target implementation to mirror:

#### Data Model (`hl_quick.xml:10900-10920`)
```typescript
type GraphNodeType = 'user' | 'assistant' | 'tool' | 'subagent-root' | 'subagent-message'

interface GraphNode {
  id: string
  type: GraphNodeType
  x: number
  y: number
  label: string
  content?: string
  toolCall?: ToolCall
  subagentResult?: SubagentResult
  message?: Message
  children: GraphNode[]
  parent?: GraphNode
  expanded: boolean
  isLive: boolean
}
```

#### Layout Constants (`hl_quick.xml:11134-11144`)
```typescript
const GRAPH_LAYOUT = {
  nodeWidth: 200,
  nodeHeight: 60,
  toolNodeHeight: 36,
  horizontalGap: 40,
  verticalGap: 30,
  branchIndent: 60,
  padding: 40
}
```

#### Tree Layout Algorithm (`hl_quick.xml:11154-11188`)
- Depth-first traversal computing x/y positions
- Depth-based indentation using `branchIndent`
- Vertical stacking with `verticalGap`
- Tool nodes use smaller `toolNodeHeight`

### 2. Existing DAG Editor Canvas Patterns

**File**: `descartes/gui/src/dag_editor.rs`

#### Canvas Program Trait (`dag_editor.rs:876-935`)
```rust
struct DAGCanvas {
    state: DAGEditorState,
}

impl<Message> canvas::Program<Message> for DAGCanvas {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        // Draw grid, edges, then nodes
        vec![frame.into_geometry()]
    }
}
```

#### Node Rendering (`dag_editor.rs:970-1025`)
- `Path::rounded_rectangle()` for node shapes
- `frame.fill()` for solid color fill
- `frame.stroke()` with `Stroke::default().with_color().with_width()` for borders
- `frame.fill_text()` for labels with alignment options
- Font size scales with zoom: `(14.0 * zoom).max(8.0).min(24.0)`

#### Edge Rendering (`dag_editor.rs:1028-1131`)
- `Path::line()` for straight edges
- `Path::new(|builder| { ... })` for custom arrow shapes
- Color varies by edge type (Dependency, DataFlow, Trigger)
- Selection state uses gold color (255, 200, 0)

#### Coordinate Transformations (`dag_editor.rs:1138-1175`)
```rust
pub fn screen_to_world(point: Point, canvas_state: &CanvasState) -> Point {
    Point::new(
        (point.x - canvas_state.offset.x) / canvas_state.zoom,
        (point.y - canvas_state.offset.y) / canvas_state.zoom,
    )
}

pub fn world_to_screen(point: Point, canvas_state: &CanvasState) -> Point {
    Point::new(
        point.x * canvas_state.zoom + canvas_state.offset.x,
        point.y * canvas_state.zoom + canvas_state.offset.y,
    )
}
```

### 3. Interaction Patterns

**File**: `descartes/gui/src/dag_canvas_interactions.rs`

#### Interaction State Types (`dag_canvas_interactions.rs:78-103`)
```rust
pub struct ExtendedInteractionState {
    pub box_selection: Option<BoxSelection>,
    pub edge_creation: Option<EdgeCreation>,
    pub space_held: bool,
    pub pre_drag_positions: HashMap<Uuid, Position>,
}
```

#### Interaction Results (`dag_canvas_interactions.rs:614-654`)
Typed enum for operation outcomes:
- Node operations: `NodeDragStarted`, `NodeDragging`, `NodeDragEnded`, `NodeAdded`, etc.
- Edge operations: `EdgeCreationStarted`, `EdgeCreated`, etc.
- View operations: `PanStarted`, `Panning`, `PanEnded`, `Zoomed`
- Context menus: `ContextMenuRequested`, `CanvasContextMenuRequested`

#### Hit Testing (`dag_canvas_interactions.rs:584-601`)
```rust
pub fn find_node_at_position(state: &DAGEditorState, position: Point) -> Option<Uuid> {
    let mut nodes: Vec<_> = state.dag.nodes.values().collect();
    nodes.sort_by(|a, b| b.position.y.partial_cmp(&a.position.y).unwrap());
    nodes.iter().find(|node|
        point_in_node(position, node, &state.canvas_state)
    ).map(|n| n.node_id)
}
```

### 4. Theme and Styling

**File**: `descartes/gui/src/theme.rs`

#### Color Palette (`theme.rs:34-91`)
```rust
// Primary colors (neon cyan)
pub const PRIMARY: Color = Color::from_rgb(0.0, 0.9, 0.9);      // #00e6e6
pub const PRIMARY_DIM: Color = Color::from_rgb(0.0, 0.35, 0.4); // #005966

// Status colors
pub const SUCCESS: Color = Color::from_rgb(0.0, 1.0, 0.5);      // #00ff80 (neon green)
pub const WARNING: Color = Color::from_rgb(1.0, 0.8, 0.0);      // #ffcc00 (amber)
pub const ERROR: Color = Color::from_rgb(1.0, 0.2, 0.3);        // #ff334d (neon red)

// Graph colors
pub const NODE_DEFAULT: Color = SURFACE;
pub const NODE_SELECTED: Color = PRIMARY_DIM;
pub const EDGE_DEFAULT: Color = BORDER;
pub const EDGE_ACTIVE: Color = PRIMARY;
```

#### Fonts (`theme.rs:14-31`)
```rust
pub mod fonts {
    pub const MONO: Font = Font { family: Family::Name("JetBrains Mono"), weight: Weight::Normal };
    pub const MONO_MEDIUM: Font = Font { family: Family::Name("JetBrains Mono"), weight: Weight::Medium };
    pub const MONO_BOLD: Font = Font { family: Family::Name("JetBrains Mono"), weight: Weight::Bold };
}
```

### 5. Chat State and View

**File**: `descartes/gui/src/chat_state.rs`

#### Current State Model (`chat_state.rs:9-23`)
```rust
pub struct ChatState {
    pub prompt_input: String,
    pub messages: Vec<ChatMessageEntry>,
    pub loading: bool,
    pub error: Option<String>,
    pub session_id: Option<Uuid>,
    pub working_directory: Option<String>,
}

pub struct ChatMessageEntry {
    pub id: Uuid,
    pub role: ChatRole,  // User, Assistant, System
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

**Current Limitations**:
- No tool call information in messages
- No parent-child relationships
- No subagent tracking
- Linear list structure, not tree

### 6. Main Application Integration

**File**: `descartes/gui/src/main.rs`

#### ViewMode Pattern (`main.rs:104-115`)
```rust
enum ViewMode {
    Sessions,
    Dashboard,
    Chat,
    TaskBoard,
    SwarmMonitor,
    Debugger,
    DagEditor,
    FileBrowser,
    KnowledgeGraph,
}
```

To add a new view mode (e.g., `ChatGraph`), add to this enum.

#### Message Routing (`main.rs:118-161`)
```rust
enum Message {
    SwitchView(ViewMode),
    Session(SessionMessage),
    TaskBoard(TaskBoardMessage),
    DAGEditor(DAGEditorMessage),
    Chat(chat_state::ChatMessage),
    // ... etc
}
```

#### View Dispatch (`main.rs:1138-1156`)
```rust
fn view_content(&self) -> Element<Message> {
    match self.current_view {
        ViewMode::Chat => self.view_chat(),
        ViewMode::DagEditor => self.view_dag_editor(),
        // ... etc
    }
}
```

### 7. Existing Data Structures for Reference

#### ToolCall (`descartes/core/src/traits.rs:59-65`)
```rust
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}
```

#### AgentHistoryEvent (`descartes/core/src/agent_history.rs:86-121`)
```rust
pub struct AgentHistoryEvent {
    pub event_id: Uuid,
    pub agent_id: String,
    pub timestamp: i64,
    pub event_type: HistoryEventType,  // Thought, Action, ToolUse, etc.
    pub event_data: Value,
    pub parent_event_id: Option<Uuid>,  // For causality chains
    pub tags: Vec<String>,
}
```

#### TranscriptMetadata (`descartes/core/src/session_transcript.rs:25-39`)
```rust
pub struct TranscriptMetadata {
    pub session_id: Uuid,
    pub parent_session_id: Option<Uuid>,  // For sub-sessions
    pub is_sub_session: bool,
}
```

## Code References

### Primary Reference Files
- `descartes/gui/src/dag_editor.rs` - Canvas rendering patterns (1294 lines)
- `descartes/gui/src/dag_canvas_interactions.rs` - Interaction handling
- `descartes/gui/src/chat_view.rs:1-291` - Current chat view implementation
- `descartes/gui/src/chat_state.rs:1-111` - Chat state management
- `descartes/gui/src/main.rs:1-2272` - Application structure and view modes
- `descartes/gui/src/theme.rs` - Theme and styling definitions

### Iced Canvas API Usage
- `dag_editor.rs:876-935` - canvas::Program trait implementation
- `dag_editor.rs:970-1025` - Node drawing with rounded rectangles
- `dag_editor.rs:1028-1131` - Edge drawing with arrows
- `dag_editor.rs:1138-1175` - Coordinate transformation helpers
- `time_travel.rs:610-733` - Alternative timeline canvas example

### Interaction Patterns
- `dag_canvas_interactions.rs:109-577` - Event handlers (mouse, keyboard)
- `dag_canvas_interactions.rs:584-601` - Hit testing logic
- `dag_canvas_interactions.rs:614-654` - InteractionResult enum

## Architecture Documentation

### Recommended New File Structure

```
descartes/gui/src/
├── chat_graph_view.rs     # NEW: Main graph view component
├── chat_graph_state.rs    # NEW: State management for graph
├── chat_graph_layout.rs   # NEW: Tree layout algorithm
└── chat_graph_node.rs     # NEW: Node data types
```

### Proposed ChatGraphNode Type
```rust
#[derive(Debug, Clone)]
pub enum ChatGraphNodeType {
    User,
    Assistant,
    Tool,
    SubagentRoot,
    SubagentMessage,
}

#[derive(Debug, Clone)]
pub struct ChatGraphNode {
    pub id: Uuid,
    pub node_type: ChatGraphNodeType,
    pub x: f32,
    pub y: f32,
    pub label: String,
    pub content: Option<String>,
    pub tool_call: Option<ToolCallInfo>,
    pub children: Vec<Uuid>,
    pub parent: Option<Uuid>,
    pub expanded: bool,
    pub is_live: bool,
}
```

### Rendering Constants (matching hl_quick)
```rust
pub const NODE_WIDTH: f32 = 200.0;
pub const NODE_HEIGHT: f32 = 60.0;
pub const TOOL_NODE_HEIGHT: f32 = 36.0;
pub const HORIZONTAL_GAP: f32 = 40.0;
pub const VERTICAL_GAP: f32 = 30.0;
pub const BRANCH_INDENT: f32 = 60.0;
pub const PADDING: f32 = 40.0;
```

### Color Mapping for Node Types
```rust
fn node_color(node_type: &ChatGraphNodeType) -> (Color, Color) {
    match node_type {
        ChatGraphNodeType::User => (colors::PRIMARY_DIM, colors::PRIMARY),
        ChatGraphNodeType::Assistant => (colors::SURFACE, colors::BORDER),
        ChatGraphNodeType::Tool => (colors::SURFACE, colors::TEXT_MUTED),
        ChatGraphNodeType::SubagentRoot => (colors::SURFACE, colors::INFO),
        ChatGraphNodeType::SubagentMessage => (colors::SURFACE, colors::INFO),
    }
}
```

## Related Research

- Existing time travel debugger uses canvas for timeline visualization (`time_travel.rs`)
- Knowledge graph panel uses similar graph visualization patterns (`knowledge_graph_panel.rs`)

## Open Questions

1. **Daemon Event Integration**: How to subscribe to daemon events for live streaming updates? The current `main.rs` has a subscription stub but it's not fully implemented (`main.rs:952-978`).

2. **Subagent Tracking**: The current ChatState doesn't track subagent spawning. Need to extend the data model to capture tool calls that spawn subagents.

3. **Performance with Large Graphs**: May need virtualization or node culling for conversations with 100+ messages. Consider conditional rendering based on zoom level (pattern from `dag_editor.rs:1012`).

4. **Toggle Integration**: Where to place the graph/linear view toggle button? Options:
   - In chat view header (recommended, similar to hl_quick)
   - In navigation sidebar as sub-mode
   - As keyboard shortcut

5. **Detail Popup**: Iced doesn't have a native popup/modal. Options:
   - Overlay container on top of canvas
   - Side panel that expands
   - Use existing modal patterns from session selector
