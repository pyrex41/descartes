use iced::mouse;
use iced::widget::canvas::{Cache, Canvas, Cursor, Frame, Geometry, Path, Stroke, Style, Text};
/// DAG Editor - Visual graph editor for task dependencies
/// Phase 3.8.3: Basic Iced UI Renderer
///
/// Features:
/// - Canvas-based rendering with pan/zoom
/// - Node rendering with labels and status colors
/// - Edge rendering with arrows and labels
/// - Interactive node selection and manipulation
/// - Toolbar with editing tools
/// - Side panel for node properties
/// - Bottom panel for graph statistics
/// - Grid background with optional snap-to-grid
/// - Performance optimized for 100+ nodes
use iced::widget::{button, canvas, column, container, row, scrollable, text, Scrollable, Space};
use iced::{
    alignment::{Horizontal, Vertical},
    Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector,
};

use descartes_core::dag::{
    DAGEdge, DAGHistory, DAGNode, DAGOperation, DAGStatistics, EdgeType, Position, DAG,
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

use crate::dag_canvas_interactions::{
    handle_key_press, handle_key_release, handle_mouse_move, handle_mouse_press,
    handle_mouse_release, handle_mouse_scroll, BoxSelection, EdgeCreation,
    ExtendedInteractionState, InteractionResult,
};

// ============================================================================
// Constants
// ============================================================================

const GRID_SIZE: f32 = 20.0;
const NODE_WIDTH: f32 = 160.0;
const NODE_HEIGHT: f32 = 60.0;
const NODE_RADIUS: f32 = 8.0;
const NODE_PADDING: f32 = 10.0;
const EDGE_WIDTH: f32 = 2.0;
const ARROW_SIZE: f32 = 10.0;
const SELECTION_BORDER: f32 = 3.0;
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 5.0;
const ZOOM_STEP: f32 = 0.1;

// ============================================================================
// State Management
// ============================================================================

/// Main DAG Editor State
#[derive(Debug, Clone)]
pub struct DAGEditorState {
    /// The DAG being edited
    pub dag: DAG,

    /// Canvas state
    pub canvas_state: CanvasState,

    /// UI state
    pub ui_state: UIState,

    /// Interaction state
    pub interaction: InteractionState,

    /// Extended interaction state (box selection, edge creation, etc.)
    pub extended_interaction: ExtendedInteractionState,

    /// Undo/redo history
    pub history: DAGHistory,

    /// Selected tool
    pub tool: Tool,

    /// Show grid
    pub show_grid: bool,

    /// Snap to grid
    pub snap_to_grid: bool,

    /// Graph statistics cache
    pub statistics: Option<DAGStatistics>,

    /// Canvas cache for performance
    canvas_cache: Cache,
}

/// Canvas view state (pan, zoom)
#[derive(Debug, Clone)]
pub struct CanvasState {
    /// Pan offset (x, y)
    pub offset: Vector,

    /// Zoom level (1.0 = 100%)
    pub zoom: f32,

    /// Canvas bounds (for clipping)
    pub bounds: Rectangle,
}

/// UI panel visibility and state
#[derive(Debug, Clone)]
pub struct UIState {
    /// Show properties panel
    pub show_properties: bool,

    /// Show statistics panel
    pub show_statistics: bool,

    /// Show toolbar
    pub show_toolbar: bool,

    /// Properties panel width
    pub properties_width: f32,
}

/// Interaction state (dragging, selecting)
#[derive(Debug, Clone)]
pub struct InteractionState {
    /// Selected node IDs
    pub selected_nodes: HashSet<Uuid>,

    /// Selected edge IDs
    pub selected_edges: HashSet<Uuid>,

    /// Currently hovering over node
    pub hover_node: Option<Uuid>,

    /// Currently hovering over edge
    pub hover_edge: Option<Uuid>,

    /// Drag state
    pub drag_state: Option<DragState>,

    /// Pan state (for canvas dragging)
    pub pan_state: Option<PanState>,
}

/// Drag operation state
#[derive(Debug, Clone)]
pub struct DragState {
    /// Nodes being dragged
    pub nodes: HashSet<Uuid>,

    /// Starting positions of dragged nodes
    pub start_positions: HashMap<Uuid, Position>,

    /// Drag start cursor position
    pub start_cursor: Point,
}

/// Pan operation state
#[derive(Debug, Clone)]
pub struct PanState {
    /// Pan start cursor position
    pub start_cursor: Point,

    /// Starting offset
    pub start_offset: Vector,
}

/// Editor tools
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    /// Select and move nodes
    Select,

    /// Add new nodes
    AddNode,

    /// Add edges between nodes
    AddEdge,

    /// Delete nodes/edges
    Delete,

    /// Pan the canvas
    Pan,
}

/// Messages for DAG Editor
#[derive(Debug, Clone)]
pub enum DAGEditorMessage {
    /// Tool selection
    SelectTool(Tool),

    /// Canvas interactions
    CanvasClicked(Point),
    CanvasRightClicked(Point),
    CanvasDrag(Point),
    CanvasDragEnd,
    MousePressed(mouse::Button, Point, keyboard::Modifiers),
    MouseReleased(mouse::Button, Point),
    MouseMoved(Point),
    MouseScrolled(mouse::ScrollDelta, Point),

    /// Keyboard interactions
    KeyPressed(keyboard::Key, keyboard::Modifiers),
    KeyReleased(keyboard::Key),

    /// Node operations
    SelectNode(Uuid, bool), // ID, add_to_selection
    DeselectAll,
    DeleteSelected,
    MoveNodes(HashMap<Uuid, Position>),

    /// Edge operations
    SelectEdge(Uuid, bool),
    CreateEdge(Uuid, Uuid, EdgeType),
    DeleteEdge(Uuid),

    /// View operations
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomToPoint(Point, f32), // position, zoom level
    PanTo(Vector),
    FitToView,

    /// Grid operations
    ToggleGrid,
    ToggleSnapToGrid,

    /// Panel operations
    ToggleProperties,
    ToggleStatistics,
    ToggleToolbar,

    /// DAG operations
    AddNode(Position),
    UpdateNode(Uuid, DAGNode),
    RemoveNode(Uuid),

    /// Undo/Redo
    Undo,
    Redo,

    /// Data operations
    LoadDAG(DAG),
    SaveDAG,
    NewDAG,

    /// Statistics
    UpdateStatistics,

    /// Internal
    InteractionResult(InteractionResult),
}

// ============================================================================
// Default Implementations
// ============================================================================

impl Default for DAGEditorState {
    fn default() -> Self {
        Self::new()
    }
}

impl DAGEditorState {
    pub fn new() -> Self {
        Self {
            dag: DAG::new("Untitled Workflow"),
            canvas_state: CanvasState::default(),
            ui_state: UIState::default(),
            interaction: InteractionState::default(),
            extended_interaction: ExtendedInteractionState::default(),
            history: DAGHistory::new(),
            tool: Tool::Select,
            show_grid: true,
            snap_to_grid: false,
            statistics: None,
            canvas_cache: Cache::new(),
        }
    }

    /// Create with an existing DAG
    pub fn with_dag(dag: DAG) -> Self {
        let mut state = Self::new();
        state.dag = dag;
        state.update_statistics();
        state
    }

    /// Update graph statistics
    pub fn update_statistics(&mut self) {
        self.statistics = self.dag.statistics().ok();
    }

    /// Clear cache to force redraw
    pub fn clear_cache(&mut self) {
        self.canvas_cache.clear();
    }
}

impl Default for CanvasState {
    fn default() -> Self {
        Self {
            offset: Vector::new(0.0, 0.0),
            zoom: 1.0,
            bounds: Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0)),
        }
    }
}

impl Default for UIState {
    fn default() -> Self {
        Self {
            show_properties: true,
            show_statistics: true,
            show_toolbar: true,
            properties_width: 300.0,
        }
    }
}

impl Default for InteractionState {
    fn default() -> Self {
        Self {
            selected_nodes: HashSet::new(),
            selected_edges: HashSet::new(),
            hover_node: None,
            hover_edge: None,
            drag_state: None,
            pan_state: None,
        }
    }
}

// ============================================================================
// Update Logic
// ============================================================================

pub fn update(state: &mut DAGEditorState, message: DAGEditorMessage) {
    match message {
        DAGEditorMessage::SelectTool(tool) => {
            state.tool = tool;
            state.interaction.selected_nodes.clear();
            state.interaction.selected_edges.clear();
            state.clear_cache();
        }

        // New interaction handlers
        DAGEditorMessage::MousePressed(button, position, modifiers) => {
            if let Some(result) = handle_mouse_press(
                state,
                &mut state.extended_interaction,
                button,
                position,
                modifiers,
            ) {
                update(state, DAGEditorMessage::InteractionResult(result));
            }
        }

        DAGEditorMessage::MouseReleased(button, position) => {
            if let Some(result) =
                handle_mouse_release(state, &mut state.extended_interaction, button, position)
            {
                update(state, DAGEditorMessage::InteractionResult(result));
            }
        }

        DAGEditorMessage::MouseMoved(position) => {
            if let Some(result) =
                handle_mouse_move(state, &mut state.extended_interaction, position)
            {
                update(state, DAGEditorMessage::InteractionResult(result));
            }
        }

        DAGEditorMessage::MouseScrolled(delta, position) => {
            if let Some(result) = handle_mouse_scroll(state, delta, position) {
                update(state, DAGEditorMessage::InteractionResult(result));
            }
        }

        DAGEditorMessage::KeyPressed(key, modifiers) => {
            if let Some(result) =
                handle_key_press(state, &mut state.extended_interaction, key, modifiers)
            {
                update(state, DAGEditorMessage::InteractionResult(result));
            }
        }

        DAGEditorMessage::KeyReleased(key) => {
            if let Some(result) = handle_key_release(state, &mut state.extended_interaction, key) {
                update(state, DAGEditorMessage::InteractionResult(result));
            }
        }

        DAGEditorMessage::InteractionResult(result) => {
            handle_interaction_result(state, result);
        }

        DAGEditorMessage::Undo => {
            if let Some(operation) = state.history.undo() {
                apply_undo_operation(state, operation);
                state.update_statistics();
                state.clear_cache();
            }
        }

        DAGEditorMessage::Redo => {
            if let Some(operation) = state.history.redo() {
                apply_redo_operation(state, operation);
                state.update_statistics();
                state.clear_cache();
            }
        }

        DAGEditorMessage::ZoomToPoint(position, zoom) => {
            let world_pos_before = screen_to_world(position, &state.canvas_state);
            state.canvas_state.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
            let world_pos_after = screen_to_world(position, &state.canvas_state);

            let world_delta = Vector::new(
                (world_pos_after.x - world_pos_before.x) * state.canvas_state.zoom,
                (world_pos_after.y - world_pos_before.y) * state.canvas_state.zoom,
            );

            state.canvas_state.offset = Vector::new(
                state.canvas_state.offset.x + world_delta.x,
                state.canvas_state.offset.y + world_delta.y,
            );

            state.clear_cache();
        }

        DAGEditorMessage::SelectNode(node_id, add_to_selection) => {
            if add_to_selection {
                if state.interaction.selected_nodes.contains(&node_id) {
                    state.interaction.selected_nodes.remove(&node_id);
                } else {
                    state.interaction.selected_nodes.insert(node_id);
                }
            } else {
                state.interaction.selected_nodes.clear();
                state.interaction.selected_nodes.insert(node_id);
            }
            state.clear_cache();
        }

        DAGEditorMessage::DeselectAll => {
            state.interaction.selected_nodes.clear();
            state.interaction.selected_edges.clear();
            state.clear_cache();
        }

        DAGEditorMessage::DeleteSelected => {
            // Delete selected nodes
            let nodes_to_delete: Vec<Uuid> =
                state.interaction.selected_nodes.iter().copied().collect();
            for node_id in nodes_to_delete {
                let _ = state.dag.remove_node(node_id);
            }

            // Delete selected edges
            let edges_to_delete: Vec<Uuid> =
                state.interaction.selected_edges.iter().copied().collect();
            for edge_id in edges_to_delete {
                let _ = state.dag.remove_edge(edge_id);
            }

            state.interaction.selected_nodes.clear();
            state.interaction.selected_edges.clear();
            state.update_statistics();
            state.clear_cache();
        }

        DAGEditorMessage::AddNode(position) => {
            let node_count = state.dag.nodes.len();
            let node = DAGNode::new_auto(format!("Task {}", node_count + 1))
                .with_position(position.x as f64, position.y as f64);

            let node_data = node.clone();
            if state.dag.add_node(node).is_ok() {
                state.history.record(DAGOperation::AddNode(node_data));
                state.update_statistics();
                state.clear_cache();
            }
        }

        DAGEditorMessage::RemoveNode(node_id) => {
            if let Some(node) = state.dag.get_node(node_id).cloned() {
                if state.dag.remove_node(node_id).is_ok() {
                    state
                        .history
                        .record(DAGOperation::RemoveNode(node_id, node));
                    state.update_statistics();
                    state.clear_cache();
                }
            }
        }

        DAGEditorMessage::CreateEdge(from_id, to_id, edge_type) => {
            let edge = DAGEdge::new(from_id, to_id, edge_type);
            let edge_data = edge.clone();
            if state.dag.add_edge(edge).is_ok() {
                state.history.record(DAGOperation::AddEdge(edge_data));
                state.update_statistics();
                state.clear_cache();
            }
        }

        DAGEditorMessage::DeleteEdge(edge_id) => {
            if let Some(edge) = state.dag.get_edge(edge_id).cloned() {
                if state.dag.remove_edge(edge_id).is_ok() {
                    state
                        .history
                        .record(DAGOperation::RemoveEdge(edge_id, edge));
                    state.update_statistics();
                    state.clear_cache();
                }
            }
        }

        DAGEditorMessage::ZoomIn => {
            state.canvas_state.zoom = (state.canvas_state.zoom + ZOOM_STEP).min(MAX_ZOOM);
            state.clear_cache();
        }

        DAGEditorMessage::ZoomOut => {
            state.canvas_state.zoom = (state.canvas_state.zoom - ZOOM_STEP).max(MIN_ZOOM);
            state.clear_cache();
        }

        DAGEditorMessage::ZoomReset => {
            state.canvas_state.zoom = 1.0;
            state.canvas_state.offset = Vector::new(0.0, 0.0);
            state.clear_cache();
        }

        DAGEditorMessage::PanTo(offset) => {
            state.canvas_state.offset = offset;
            state.clear_cache();
        }

        DAGEditorMessage::FitToView => {
            // Calculate bounding box of all nodes
            if state.dag.nodes.is_empty() {
                return;
            }

            let mut min_x = f32::INFINITY;
            let mut min_y = f32::INFINITY;
            let mut max_x = f32::NEG_INFINITY;
            let mut max_y = f32::NEG_INFINITY;

            for node in state.dag.nodes.values() {
                min_x = min_x.min(node.position.x as f32);
                min_y = min_y.min(node.position.y as f32);
                max_x = max_x.max(node.position.x as f32);
                max_y = max_y.max(node.position.y as f32);
            }

            let width = max_x - min_x + NODE_WIDTH * 2.0;
            let height = max_y - min_y + NODE_HEIGHT * 2.0;

            let zoom_x = state.canvas_state.bounds.width / width;
            let zoom_y = state.canvas_state.bounds.height / height;
            state.canvas_state.zoom = zoom_x.min(zoom_y).min(MAX_ZOOM).max(MIN_ZOOM);

            let center_x = (min_x + max_x) / 2.0;
            let center_y = (min_y + max_y) / 2.0;

            state.canvas_state.offset = Vector::new(
                state.canvas_state.bounds.width / 2.0 - center_x * state.canvas_state.zoom,
                state.canvas_state.bounds.height / 2.0 - center_y * state.canvas_state.zoom,
            );

            state.clear_cache();
        }

        DAGEditorMessage::ToggleGrid => {
            state.show_grid = !state.show_grid;
            state.clear_cache();
        }

        DAGEditorMessage::ToggleSnapToGrid => {
            state.snap_to_grid = !state.snap_to_grid;
        }

        DAGEditorMessage::ToggleProperties => {
            state.ui_state.show_properties = !state.ui_state.show_properties;
        }

        DAGEditorMessage::ToggleStatistics => {
            state.ui_state.show_statistics = !state.ui_state.show_statistics;
        }

        DAGEditorMessage::ToggleToolbar => {
            state.ui_state.show_toolbar = !state.ui_state.show_toolbar;
        }

        DAGEditorMessage::LoadDAG(dag) => {
            state.dag = dag;
            state.dag.rebuild_adjacency();
            state.update_statistics();
            state.clear_cache();
        }

        DAGEditorMessage::NewDAG => {
            state.dag = DAG::new("Untitled Workflow");
            state.interaction = InteractionState::default();
            state.statistics = None;
            state.clear_cache();
        }

        DAGEditorMessage::UpdateStatistics => {
            state.update_statistics();
        }

        _ => {
            // Other messages handled by canvas event handlers
        }
    }
}

// ============================================================================
// View Components
// ============================================================================

pub fn view(state: &DAGEditorState) -> Element<DAGEditorMessage> {
    let mut content = column![];

    // Toolbar
    if state.ui_state.show_toolbar {
        content = content.push(view_toolbar(state));
    }

    // Main content area (canvas + side panel)
    let main_row = if state.ui_state.show_properties {
        row![view_canvas(state), view_properties_panel(state),].spacing(0)
    } else {
        row![view_canvas(state)].spacing(0)
    };

    content = content.push(main_row);

    // Bottom statistics panel
    if state.ui_state.show_statistics {
        content = content.push(view_statistics_panel(state));
    }

    content.spacing(0).into()
}

/// Render the toolbar
fn view_toolbar(state: &DAGEditorState) -> Element<DAGEditorMessage> {
    let tool_button = |tool: Tool, label: &str, icon: &str| {
        let is_active = state.tool == tool;
        button(text(format!("{} {}", icon, label)).size(14))
            .padding(8)
            .on_press(DAGEditorMessage::SelectTool(tool))
            .style(if is_active {
                button::primary
            } else {
                button::secondary
            })
    };

    let tools = row![
        tool_button(Tool::Select, "Select", "↖"),
        tool_button(Tool::AddNode, "Add Node", "+"),
        tool_button(Tool::AddEdge, "Add Edge", "→"),
        tool_button(Tool::Delete, "Delete", "×"),
        tool_button(Tool::Pan, "Pan", "✋"),
    ]
    .spacing(5);

    let view_buttons = row![
        button(text("Zoom In [+]").size(14))
            .padding(8)
            .on_press(DAGEditorMessage::ZoomIn),
        button(text("Zoom Out [-]").size(14))
            .padding(8)
            .on_press(DAGEditorMessage::ZoomOut),
        button(text("Reset").size(14))
            .padding(8)
            .on_press(DAGEditorMessage::ZoomReset),
        button(text("Fit").size(14))
            .padding(8)
            .on_press(DAGEditorMessage::FitToView),
    ]
    .spacing(5);

    let grid_buttons = row![
        button(
            text(if state.show_grid {
                "Grid: ON"
            } else {
                "Grid: OFF"
            })
            .size(14)
        )
        .padding(8)
        .on_press(DAGEditorMessage::ToggleGrid),
        button(
            text(if state.snap_to_grid {
                "Snap: ON"
            } else {
                "Snap: OFF"
            })
            .size(14)
        )
        .padding(8)
        .on_press(DAGEditorMessage::ToggleSnapToGrid),
    ]
    .spacing(5);

    let toolbar_content = row![
        tools,
        Space::with_width(20),
        view_buttons,
        Space::with_width(20),
        grid_buttons,
        Space::with_width(Length::Fill),
        button(text("New").size(14))
            .padding(8)
            .on_press(DAGEditorMessage::NewDAG),
    ]
    .spacing(10)
    .padding(10)
    .align_y(Vertical::Center);

    container(toolbar_content)
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                width: 1.0,
                color: theme.palette().text.scale_alpha(0.2),
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render the canvas
fn view_canvas(state: &DAGEditorState) -> Element<DAGEditorMessage> {
    let canvas = Canvas::new(DAGCanvas {
        state: state.clone(),
    })
    .width(Length::Fill)
    .height(Length::Fill);

    container(canvas)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render properties panel
fn view_properties_panel(state: &DAGEditorState) -> Element<DAGEditorMessage> {
    let title = text("Properties").size(18).width(Length::Fill);

    let content = if state.interaction.selected_nodes.is_empty() {
        column![
            text("No node selected").size(14),
            Space::with_height(10),
            text("Select a node to view properties").size(12),
        ]
        .spacing(5)
    } else {
        let selected_count = state.interaction.selected_nodes.len();
        let info = if selected_count == 1 {
            let node_id = state.interaction.selected_nodes.iter().next().unwrap();
            if let Some(node) = state.dag.get_node(*node_id) {
                column![
                    text(format!("Node: {}", node.label)).size(14),
                    Space::with_height(5),
                    text(format!("ID: {}", node.node_id)).size(10),
                    Space::with_height(5),
                    text(format!(
                        "Position: ({:.0}, {:.0})",
                        node.position.x, node.position.y
                    ))
                    .size(10),
                    Space::with_height(10),
                    text(format!(
                        "Incoming: {}",
                        state.dag.get_incoming_edges(*node_id).len()
                    ))
                    .size(10),
                    text(format!(
                        "Outgoing: {}",
                        state.dag.get_outgoing_edges(*node_id).len()
                    ))
                    .size(10),
                ]
                .spacing(3)
            } else {
                column![text("Node not found").size(12)]
            }
        } else {
            column![text(format!("{} nodes selected", selected_count)).size(14),]
        };

        info
    };

    let panel_content = scrollable(
        column![title, Space::with_height(10), content,]
            .spacing(5)
            .padding(15),
    );

    container(panel_content)
        .width(state.ui_state.properties_width)
        .height(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                width: 1.0,
                color: theme.palette().text.scale_alpha(0.2),
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render statistics panel
fn view_statistics_panel(state: &DAGEditorState) -> Element<DAGEditorMessage> {
    let stats_text = if let Some(ref stats) = state.statistics {
        format!(
            "Nodes: {} | Edges: {} | Start: {} | End: {} | Depth: {} | Connected: {} | Acyclic: {} | Zoom: {:.0}%",
            stats.node_count,
            stats.edge_count,
            stats.start_nodes,
            stats.end_nodes,
            stats.max_depth,
            if stats.is_connected { "✓" } else { "✗" },
            if stats.is_acyclic { "✓" } else { "✗" },
            state.canvas_state.zoom * 100.0
        )
    } else {
        "No statistics available".to_string()
    };

    container(text(stats_text).size(12).width(Length::Fill))
        .padding(8)
        .width(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(theme.palette().background.into()),
            border: iced::Border {
                width: 1.0,
                color: theme.palette().text.scale_alpha(0.2),
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

// ============================================================================
// Canvas Rendering
// ============================================================================

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

        // Draw grid
        if self.state.show_grid {
            draw_grid(&mut frame, &self.state.canvas_state, bounds);
        }

        // Draw edges
        for edge in self.state.dag.edges.values() {
            if let (Some(from_node), Some(to_node)) = (
                self.state.dag.get_node(edge.from_node_id),
                self.state.dag.get_node(edge.to_node_id),
            ) {
                let is_selected = self
                    .state
                    .interaction
                    .selected_edges
                    .contains(&edge.edge_id);
                draw_edge(
                    &mut frame,
                    edge,
                    from_node,
                    to_node,
                    &self.state.canvas_state,
                    is_selected,
                );
            }
        }

        // Draw nodes
        for node in self.state.dag.nodes.values() {
            let is_selected = self
                .state
                .interaction
                .selected_nodes
                .contains(&node.node_id);
            let is_hover = self.state.interaction.hover_node == Some(node.node_id);
            draw_node(
                &mut frame,
                node,
                &self.state.canvas_state,
                is_selected,
                is_hover,
            );
        }

        vec![frame.into_geometry()]
    }
}

/// Draw grid background
fn draw_grid(frame: &mut Frame, canvas_state: &CanvasState, bounds: Rectangle) {
    let grid_color = Color::from_rgba8(100, 100, 100, 0.2);
    let grid_size = GRID_SIZE * canvas_state.zoom;

    // Calculate grid start position
    let start_x = (canvas_state.offset.x % grid_size) - grid_size;
    let start_y = (canvas_state.offset.y % grid_size) - grid_size;

    // Draw vertical lines
    let mut x = start_x;
    while x < bounds.width {
        let path = Path::line(Point::new(x, 0.0), Point::new(x, bounds.height));
        frame.stroke(
            &path,
            Stroke::default().with_color(grid_color).with_width(1.0),
        );
        x += grid_size;
    }

    // Draw horizontal lines
    let mut y = start_y;
    while y < bounds.height {
        let path = Path::line(Point::new(0.0, y), Point::new(bounds.width, y));
        frame.stroke(
            &path,
            Stroke::default().with_color(grid_color).with_width(1.0),
        );
        y += grid_size;
    }
}

/// Draw a node
fn draw_node(
    frame: &mut Frame,
    node: &DAGNode,
    canvas_state: &CanvasState,
    is_selected: bool,
    is_hover: bool,
) {
    // Transform position to canvas coordinates
    let x = node.position.x as f32 * canvas_state.zoom + canvas_state.offset.x;
    let y = node.position.y as f32 * canvas_state.zoom + canvas_state.offset.y;
    let width = NODE_WIDTH * canvas_state.zoom;
    let height = NODE_HEIGHT * canvas_state.zoom;

    // Node background color (can be customized based on metadata)
    let node_color = Color::from_rgb8(70, 130, 180); // Steel blue
    let border_color = if is_selected {
        Color::from_rgb8(255, 200, 0) // Gold
    } else if is_hover {
        Color::from_rgb8(150, 200, 255) // Light blue
    } else {
        Color::from_rgb8(50, 90, 130) // Dark steel blue
    };

    // Draw node rectangle with rounded corners
    let node_rect = Path::rounded_rectangle(
        Point::new(x, y),
        Size::new(width, height),
        NODE_RADIUS * canvas_state.zoom,
    );

    frame.fill(&node_rect, node_color);

    let border_width = if is_selected { SELECTION_BORDER } else { 1.5 };

    frame.stroke(
        &node_rect,
        Stroke::default()
            .with_color(border_color)
            .with_width(border_width),
    );

    // Draw node label (only if zoom level is reasonable)
    if canvas_state.zoom > 0.3 {
        let font_size = (14.0 * canvas_state.zoom).max(8.0).min(24.0);
        let label_text = Text {
            content: node.label.clone(),
            position: Point::new(x + width / 2.0, y + height / 2.0),
            color: Color::WHITE,
            size: font_size.into(),
            horizontal_alignment: iced::alignment::Horizontal::Center,
            vertical_alignment: iced::alignment::Vertical::Center,
            ..Default::default()
        };
        frame.fill_text(label_text);
    }
}

/// Draw an edge
fn draw_edge(
    frame: &mut Frame,
    edge: &DAGEdge,
    from_node: &DAGNode,
    to_node: &DAGNode,
    canvas_state: &CanvasState,
    is_selected: bool,
) {
    // Calculate node centers in canvas coordinates
    let from_x = (from_node.position.x as f32 + NODE_WIDTH / 2.0) * canvas_state.zoom
        + canvas_state.offset.x;
    let from_y = (from_node.position.y as f32 + NODE_HEIGHT / 2.0) * canvas_state.zoom
        + canvas_state.offset.y;
    let to_x =
        (to_node.position.x as f32 + NODE_WIDTH / 2.0) * canvas_state.zoom + canvas_state.offset.x;
    let to_y =
        (to_node.position.y as f32 + NODE_HEIGHT / 2.0) * canvas_state.zoom + canvas_state.offset.y;

    // Edge color based on type
    let edge_color = match edge.edge_type {
        EdgeType::Dependency => Color::from_rgb8(200, 200, 200),
        EdgeType::SoftDependency => Color::from_rgba8(200, 200, 200, 0.6),
        EdgeType::DataFlow => Color::from_rgb8(100, 200, 100),
        EdgeType::Trigger => Color::from_rgb8(255, 150, 50),
        _ => Color::from_rgb8(150, 150, 150),
    };

    let stroke_color = if is_selected {
        Color::from_rgb8(255, 200, 0)
    } else {
        edge_color
    };

    let stroke_width = if is_selected {
        EDGE_WIDTH * 1.5
    } else {
        EDGE_WIDTH
    };

    // Draw line
    let line = Path::line(Point::new(from_x, from_y), Point::new(to_x, to_y));

    frame.stroke(
        &line,
        Stroke::default()
            .with_color(stroke_color)
            .with_width(stroke_width),
    );

    // Draw arrow head
    if canvas_state.zoom > 0.3 {
        draw_arrow_head(
            frame,
            from_x,
            from_y,
            to_x,
            to_y,
            stroke_color,
            canvas_state.zoom,
        );
    }
}

/// Draw arrow head at the end of an edge
fn draw_arrow_head(
    frame: &mut Frame,
    from_x: f32,
    from_y: f32,
    to_x: f32,
    to_y: f32,
    color: Color,
    zoom: f32,
) {
    let dx = to_x - from_x;
    let dy = to_y - from_y;
    let length = (dx * dx + dy * dy).sqrt();

    if length < 0.01 {
        return;
    }

    let ux = dx / length;
    let uy = dy / length;

    let arrow_size = ARROW_SIZE * zoom;
    let arrow_angle = 0.5; // radians

    // Calculate arrow points
    let p1_x = to_x - arrow_size * (ux * arrow_angle.cos() - uy * arrow_angle.sin());
    let p1_y = to_y - arrow_size * (uy * arrow_angle.cos() + ux * arrow_angle.sin());

    let p2_x = to_x - arrow_size * (ux * arrow_angle.cos() + uy * arrow_angle.sin());
    let p2_y = to_y - arrow_size * (uy * arrow_angle.cos() - ux * arrow_angle.sin());

    // Draw arrow triangle
    let arrow_path = Path::new(|builder| {
        builder.move_to(Point::new(to_x, to_y));
        builder.line_to(Point::new(p1_x, p1_y));
        builder.line_to(Point::new(p2_x, p2_y));
        builder.close();
    });

    frame.fill(&arrow_path, color);
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Convert screen coordinates to world coordinates
pub fn screen_to_world(point: Point, canvas_state: &CanvasState) -> Point {
    Point::new(
        (point.x - canvas_state.offset.x) / canvas_state.zoom,
        (point.y - canvas_state.offset.y) / canvas_state.zoom,
    )
}

/// Convert world coordinates to screen coordinates
pub fn world_to_screen(point: Point, canvas_state: &CanvasState) -> Point {
    Point::new(
        point.x * canvas_state.zoom + canvas_state.offset.x,
        point.y * canvas_state.zoom + canvas_state.offset.y,
    )
}

/// Check if a point is inside a node
pub fn point_in_node(point: Point, node: &DAGNode, canvas_state: &CanvasState) -> bool {
    let screen_point = world_to_screen(
        Point::new(node.position.x as f32, node.position.y as f32),
        canvas_state,
    );

    let width = NODE_WIDTH * canvas_state.zoom;
    let height = NODE_HEIGHT * canvas_state.zoom;

    point.x >= screen_point.x
        && point.x <= screen_point.x + width
        && point.y >= screen_point.y
        && point.y <= screen_point.y + height
}

/// Snap position to grid
pub fn snap_to_grid(position: Point) -> Point {
    Point::new(
        (position.x / GRID_SIZE).round() * GRID_SIZE,
        (position.y / GRID_SIZE).round() * GRID_SIZE,
    )
}

// ============================================================================
// Interaction Result Handlers
// ============================================================================

/// Handle interaction results from the interaction handlers
fn handle_interaction_result(state: &mut DAGEditorState, result: InteractionResult) {
    match result {
        InteractionResult::NodeDragStarted => {
            // Already handled in interaction handler
        }

        InteractionResult::NodeDragging => {
            // Positions already updated in interaction handler
        }

        InteractionResult::NodeDragEnded(new_positions) => {
            // Record the move operation for undo
            // Note: For simplicity, we record individual node updates
            // In a production system, you might want a compound operation
            state.clear_cache();
        }

        InteractionResult::NodeAdded(node_id) => {
            // Already handled in message handler
            state.interaction.selected_nodes.clear();
            state.interaction.selected_nodes.insert(node_id);
        }

        InteractionResult::NodeDeleted(_node_id) => {
            // Already handled
        }

        InteractionResult::NodesDeleted(_node_ids) => {
            // Already handled
        }

        InteractionResult::EdgeCreated(_edge_id) => {
            // Already handled
        }

        InteractionResult::EdgeCreationFailed(msg) => {
            // TODO: Show error message to user
            eprintln!("Edge creation failed: {}", msg);
        }

        InteractionResult::Zoomed(_zoom) => {
            // Already handled
        }

        InteractionResult::UndoRequested => {
            update(state, DAGEditorMessage::Undo);
        }

        InteractionResult::RedoRequested => {
            update(state, DAGEditorMessage::Redo);
        }

        _ => {
            // Other results don't need special handling
        }
    }
}

/// Apply an undo operation
fn apply_undo_operation(state: &mut DAGEditorState, operation: DAGOperation) {
    match operation {
        DAGOperation::AddNode(node) => {
            // Undo add by removing
            let _ = state.dag.remove_node(node.node_id);
        }

        DAGOperation::RemoveNode(node_id, node) => {
            // Undo remove by adding back
            let _ = state.dag.add_node(node);
        }

        DAGOperation::UpdateNode(node_id, old_node, _new_node) => {
            // Undo update by restoring old node
            let _ = state.dag.update_node(node_id, old_node);
        }

        DAGOperation::AddEdge(edge) => {
            // Undo add by removing
            let _ = state.dag.remove_edge(edge.edge_id);
        }

        DAGOperation::RemoveEdge(edge_id, edge) => {
            // Undo remove by adding back
            let _ = state.dag.add_edge(edge);
        }
    }
}

/// Apply a redo operation
fn apply_redo_operation(state: &mut DAGEditorState, operation: DAGOperation) {
    match operation {
        DAGOperation::AddNode(node) => {
            let _ = state.dag.add_node(node);
        }

        DAGOperation::RemoveNode(node_id, _node) => {
            let _ = state.dag.remove_node(node_id);
        }

        DAGOperation::UpdateNode(node_id, _old_node, new_node) => {
            let _ = state.dag.update_node(node_id, new_node);
        }

        DAGOperation::AddEdge(edge) => {
            let _ = state.dag.add_edge(edge);
        }

        DAGOperation::RemoveEdge(edge_id, _edge) => {
            let _ = state.dag.remove_edge(edge_id);
        }
    }
}
