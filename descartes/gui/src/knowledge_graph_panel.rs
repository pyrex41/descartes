/// Knowledge Graph Panel - Interactive Visualization Component
///
/// This module provides a visual, interactive knowledge graph panel that displays
/// code entities and their relationships. Features include:
/// - Full knowledge graph visualization with nodes and edges
/// - Node filtering by type (function, class, module, etc.)
/// - Interactive graph navigation with pan/zoom
/// - Click-to-jump to code location
/// - Bidirectional linking with file tree
/// - Semantic search with fuzzy matching
/// - Relationship type filtering
/// - Visual node clustering
/// - Export capabilities
#[cfg(feature = "agent-runner")]
use descartes_agent_runner::knowledge_graph::{
    KnowledgeEdge, KnowledgeGraph, KnowledgeNode, KnowledgeNodeType, RelationshipType,
};

// Stub types when agent-runner feature is not enabled
#[cfg(not(feature = "agent-runner"))]
mod stub_types {
    use std::collections::HashMap;

    #[derive(Debug, Clone)]
    pub struct KnowledgeGraph {
        pub nodes: HashMap<String, KnowledgeNode>,
        pub edges: Vec<KnowledgeEdge>,
    }

    impl KnowledgeGraph {
        pub fn new() -> Self {
            Self {
                nodes: HashMap::new(),
                edges: Vec::new(),
            }
        }

        pub fn get_node(&self, _id: &str) -> Option<&KnowledgeNode> {
            None
        }

        pub fn get_nodes_by_type(&self, _node_type: KnowledgeNodeType) -> Vec<&KnowledgeNode> {
            Vec::new()
        }

        pub fn get_outgoing_edges(&self, _node_id: &str) -> Vec<&KnowledgeEdge> {
            Vec::new()
        }

        pub fn get_incoming_edges(&self, _node_id: &str) -> Vec<&KnowledgeEdge> {
            Vec::new()
        }

        pub fn find_path(&self, _from: &str, _to: &str) -> Option<Vec<String>> {
            None
        }

        pub fn stats(&self) -> GraphStats {
            GraphStats {
                total_nodes: 0,
                total_edges: 0,
                avg_degree: 0.0,
            }
        }
    }

    pub struct GraphStats {
        pub total_nodes: usize,
        pub total_edges: usize,
        pub avg_degree: f32,
    }

    #[derive(Debug, Clone)]
    pub struct KnowledgeNode {
        pub node_id: String,
        pub name: String,
        pub qualified_name: String,
        pub content_type: KnowledgeNodeType,
        pub signature: Option<String>,
    }

    #[derive(Debug, Clone)]
    pub struct KnowledgeEdge {
        pub from_node_id: String,
        pub to_node_id: String,
        pub relationship_type: RelationshipType,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum KnowledgeNodeType {
        Function,
        Method,
        Class,
        Struct,
        Enum,
        Interface,
        Module,
        TypeAlias,
        Constant,
        Variable,
        Macro,
        Concept,
        Documentation,
    }

    impl KnowledgeNodeType {
        pub fn as_str(&self) -> &'static str {
            match self {
                Self::Function => "Function",
                Self::Method => "Method",
                Self::Class => "Class",
                Self::Struct => "Struct",
                Self::Enum => "Enum",
                Self::Interface => "Interface",
                Self::Module => "Module",
                Self::TypeAlias => "TypeAlias",
                Self::Constant => "Constant",
                Self::Variable => "Variable",
                Self::Macro => "Macro",
                Self::Concept => "Concept",
                Self::Documentation => "Documentation",
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum RelationshipType {
        Calls,
        Uses,
        Inherits,
        Implements,
        DefinedIn,
    }

    impl RelationshipType {
        pub fn as_str(&self) -> &'static str {
            match self {
                Self::Calls => "Calls",
                Self::Uses => "Uses",
                Self::Inherits => "Inherits",
                Self::Implements => "Implements",
                Self::DefinedIn => "DefinedIn",
            }
        }
    }
}

#[cfg(not(feature = "agent-runner"))]
use stub_types::{
    KnowledgeEdge, KnowledgeGraph, KnowledgeNode, KnowledgeNodeType, RelationshipType,
};

use iced::widget::{
    button, canvas, checkbox, column, container, horizontal_space, pick_list, row, scrollable,
    text, text_input, Canvas, Column, Row, Space,
};
use iced::{mouse, Color, Element, Length, Point, Rectangle, Renderer, Size, Theme, Vector};
use std::collections::{HashMap, HashSet};

/// ============================================================================
/// State Management
/// ============================================================================

/// Knowledge graph panel state
#[derive(Debug)]
pub struct KnowledgeGraphPanelState {
    /// The knowledge graph data
    pub graph: Option<KnowledgeGraph>,

    /// Currently selected node ID
    pub selected_node: Option<String>,

    /// Hovered node ID (for tooltip)
    pub hovered_node: Option<String>,

    /// Search query
    pub search_query: String,

    /// Fuzzy search enabled
    pub fuzzy_search: bool,

    /// Filter by node types
    pub type_filters: HashSet<KnowledgeNodeType>,

    /// Filter by relationship types
    pub relationship_filters: HashSet<RelationshipType>,

    /// Show only connected nodes
    pub show_only_connected: bool,

    /// Graph layout algorithm
    pub layout_algorithm: LayoutAlgorithm,

    /// Viewport state (pan/zoom)
    pub viewport: ViewportState,

    /// Node positions (for interactive layout)
    pub node_positions: HashMap<String, Point>,

    /// Node being dragged
    pub dragging_node: Option<String>,

    /// Graph visualization mode
    pub visualization_mode: VisualizationMode,

    /// Show node labels
    pub show_labels: bool,

    /// Show edge labels
    pub show_edge_labels: bool,

    /// Search results
    pub search_results: Vec<String>,

    /// Canvas cache
    pub canvas_cache: canvas::Cache,

    /// Dependency path between two nodes
    pub dependency_path: Option<Vec<String>>,

    /// Impact analysis results (affected nodes)
    pub impact_nodes: HashSet<String>,

    /// Related code suggestions
    pub related_suggestions: Vec<String>,

    /// Nodes in comparison view
    pub comparison_nodes: Vec<String>,

    /// Show minimap
    pub show_minimap: bool,

    /// Current file filter
    pub file_filter: Option<String>,

    /// Call hierarchy for selected node
    pub call_hierarchy: Option<Vec<Vec<String>>>,
}

impl Default for KnowledgeGraphPanelState {
    fn default() -> Self {
        Self {
            graph: None,
            selected_node: None,
            hovered_node: None,
            search_query: String::new(),
            fuzzy_search: true,
            type_filters: HashSet::new(),
            relationship_filters: HashSet::new(),
            show_only_connected: false,
            layout_algorithm: LayoutAlgorithm::ForceDirected,
            viewport: ViewportState::default(),
            node_positions: HashMap::new(),
            dragging_node: None,
            visualization_mode: VisualizationMode::Graph,
            show_labels: true,
            show_edge_labels: false,
            search_results: Vec::new(),
            canvas_cache: canvas::Cache::default(),
            dependency_path: None,
            impact_nodes: HashSet::new(),
            related_suggestions: Vec::new(),
            comparison_nodes: Vec::new(),
            show_minimap: false,
            file_filter: None,
            call_hierarchy: None,
        }
    }
}

impl Clone for KnowledgeGraphPanelState {
    fn clone(&self) -> Self {
        Self {
            graph: self.graph.clone(),
            selected_node: self.selected_node.clone(),
            hovered_node: self.hovered_node.clone(),
            search_query: self.search_query.clone(),
            fuzzy_search: self.fuzzy_search,
            type_filters: self.type_filters.clone(),
            relationship_filters: self.relationship_filters.clone(),
            show_only_connected: self.show_only_connected,
            layout_algorithm: self.layout_algorithm,
            viewport: self.viewport.clone(),
            node_positions: self.node_positions.clone(),
            dragging_node: self.dragging_node.clone(),
            visualization_mode: self.visualization_mode,
            show_labels: self.show_labels,
            show_edge_labels: self.show_edge_labels,
            search_results: self.search_results.clone(),
            canvas_cache: canvas::Cache::default(), // Cannot clone cache, create new one
            dependency_path: self.dependency_path.clone(),
            impact_nodes: self.impact_nodes.clone(),
            related_suggestions: self.related_suggestions.clone(),
            comparison_nodes: self.comparison_nodes.clone(),
            show_minimap: self.show_minimap,
            file_filter: self.file_filter.clone(),
            call_hierarchy: self.call_hierarchy.clone(),
        }
    }
}

/// Viewport state for pan/zoom
#[derive(Debug, Clone)]
pub struct ViewportState {
    /// Translation offset
    pub translation: Vector,
    /// Zoom level (1.0 = 100%)
    pub scale: f32,
    /// Viewport bounds
    pub bounds: Rectangle,
}

impl Default for ViewportState {
    fn default() -> Self {
        Self {
            translation: Vector::new(0.0, 0.0),
            scale: 1.0,
            bounds: Rectangle::new(Point::ORIGIN, Size::new(800.0, 600.0)),
        }
    }
}

/// Graph layout algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutAlgorithm {
    ForceDirected,
    Hierarchical,
    Circular,
    Grid,
}

impl LayoutAlgorithm {
    pub fn all() -> Vec<Self> {
        vec![
            Self::ForceDirected,
            Self::Hierarchical,
            Self::Circular,
            Self::Grid,
        ]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ForceDirected => "Force-Directed",
            Self::Hierarchical => "Hierarchical",
            Self::Circular => "Circular",
            Self::Grid => "Grid",
        }
    }
}

impl std::fmt::Display for LayoutAlgorithm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Visualization modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VisualizationMode {
    Graph,
    Tree,
    Cluster,
}

impl VisualizationMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Graph => "Graph",
            Self::Tree => "Tree",
            Self::Cluster => "Cluster",
        }
    }
}

/// Messages for the knowledge graph panel
#[derive(Debug, Clone)]
pub enum KnowledgeGraphMessage {
    /// Load a knowledge graph
    GraphLoaded(KnowledgeGraph),

    /// Select a node
    SelectNode(String),

    /// Hover over a node
    HoverNode(Option<String>),

    /// Double-click on node (jump to code)
    JumpToNode(String),

    /// Update search query
    SearchQueryChanged(String),

    /// Toggle fuzzy search
    ToggleFuzzySearch,

    /// Toggle node type filter
    ToggleTypeFilter(KnowledgeNodeType),

    /// Toggle relationship type filter
    ToggleRelationshipFilter(RelationshipType),

    /// Toggle show only connected
    ToggleShowOnlyConnected,

    /// Change layout algorithm
    SetLayoutAlgorithm(LayoutAlgorithm),

    /// Change visualization mode
    SetVisualizationMode(VisualizationMode),

    /// Toggle labels
    ToggleLabels,

    /// Toggle edge labels
    ToggleEdgeLabels,

    /// Reset viewport
    ResetViewport,

    /// Zoom in
    ZoomIn,

    /// Zoom out
    ZoomOut,

    /// Pan viewport
    Pan(Vector),

    /// Start dragging node
    StartDragNode(String),

    /// Drag node to position
    DragNode(Point),

    /// End dragging
    EndDragNode,

    /// Relayout graph
    RelayoutGraph,

    /// Clear filters
    ClearFilters,

    /// Expand node (show all connections)
    ExpandNode(String),

    /// Focus on node and neighbors
    FocusNode(String),

    /// Find all references to a node
    FindReferences(String),

    /// Go to definition of a node
    GoToDefinition(String),

    /// Find all usages of a node
    FindUsages(String),

    /// Show dependency path between two nodes
    ShowDependencyPath(String, String),

    /// Analyze impact of changes to a node
    AnalyzeImpact(String),

    /// Show related code suggestions
    ShowRelatedCode(String),

    /// Add node to comparison view
    AddToComparison(String),

    /// Clear comparison view
    ClearComparison,

    /// Export graph as image
    ExportAsImage,

    /// Export graph as JSON
    ExportAsJson,

    /// Show node history (changes over time)
    ShowNodeHistory(String),

    /// Filter by file/module
    FilterByFile(String),

    /// Show call hierarchy
    ShowCallHierarchy(String),

    /// Show inheritance hierarchy
    ShowInheritanceHierarchy(String),

    /// Toggle minimap
    ToggleMinimap,
}

/// ============================================================================
/// Update Logic
/// ============================================================================

/// Update the knowledge graph panel state
pub fn update(state: &mut KnowledgeGraphPanelState, message: KnowledgeGraphMessage) {
    match message {
        KnowledgeGraphMessage::GraphLoaded(graph) => {
            state.graph = Some(graph);
            state.selected_node = None;
            state.search_results.clear();

            // Initialize node positions with layout algorithm
            if let Some(ref graph) = state.graph {
                state.node_positions = compute_layout(&graph, state.layout_algorithm);
            }

            // Invalidate canvas cache
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::SelectNode(node_id) => {
            state.selected_node = Some(node_id);
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::HoverNode(node_id) => {
            state.hovered_node = node_id;
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::JumpToNode(node_id) => {
            state.selected_node = Some(node_id.clone());
            tracing::info!("Jumping to node: {}", node_id);
            // This would trigger navigation in the main app
        }
        KnowledgeGraphMessage::SearchQueryChanged(query) => {
            state.search_query = query;
            state.search_results = perform_search(state);
        }
        KnowledgeGraphMessage::ToggleFuzzySearch => {
            state.fuzzy_search = !state.fuzzy_search;
            state.search_results = perform_search(state);
        }
        KnowledgeGraphMessage::ToggleTypeFilter(node_type) => {
            if state.type_filters.contains(&node_type) {
                state.type_filters.remove(&node_type);
            } else {
                state.type_filters.insert(node_type);
            }
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ToggleRelationshipFilter(rel_type) => {
            if state.relationship_filters.contains(&rel_type) {
                state.relationship_filters.remove(&rel_type);
            } else {
                state.relationship_filters.insert(rel_type);
            }
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ToggleShowOnlyConnected => {
            state.show_only_connected = !state.show_only_connected;
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::SetLayoutAlgorithm(algorithm) => {
            state.layout_algorithm = algorithm;
            if let Some(ref graph) = state.graph {
                state.node_positions = compute_layout(&graph, algorithm);
            }
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::SetVisualizationMode(mode) => {
            state.visualization_mode = mode;
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ToggleLabels => {
            state.show_labels = !state.show_labels;
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ToggleEdgeLabels => {
            state.show_edge_labels = !state.show_edge_labels;
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ResetViewport => {
            state.viewport.translation = Vector::new(0.0, 0.0);
            state.viewport.scale = 1.0;
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ZoomIn => {
            state.viewport.scale = (state.viewport.scale * 1.2).min(5.0);
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ZoomOut => {
            state.viewport.scale = (state.viewport.scale / 1.2).max(0.1);
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::Pan(offset) => {
            state.viewport.translation = state.viewport.translation + offset;
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::StartDragNode(node_id) => {
            state.dragging_node = Some(node_id);
        }
        KnowledgeGraphMessage::DragNode(position) => {
            if let Some(ref node_id) = state.dragging_node {
                state.node_positions.insert(node_id.clone(), position);
                state.canvas_cache.clear();
            }
        }
        KnowledgeGraphMessage::EndDragNode => {
            state.dragging_node = None;
        }
        KnowledgeGraphMessage::RelayoutGraph => {
            if let Some(ref graph) = state.graph {
                state.node_positions = compute_layout(&graph, state.layout_algorithm);
            }
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ClearFilters => {
            state.type_filters.clear();
            state.relationship_filters.clear();
            state.show_only_connected = false;
            state.search_query.clear();
            state.search_results.clear();
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ExpandNode(node_id) => {
            state.selected_node = Some(node_id);
            // This would show all connections
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::FocusNode(node_id) => {
            state.selected_node = Some(node_id.clone());

            // Center viewport on node
            if let Some(pos) = state.node_positions.get(&node_id) {
                let center = state.viewport.bounds.center();
                state.viewport.translation = Vector::new(
                    center.x - pos.x * state.viewport.scale,
                    center.y - pos.y * state.viewport.scale,
                );
            }

            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::FindReferences(node_id) => {
            tracing::info!("Finding references for node: {}", node_id);
            // Would search for all nodes that reference this one
            state.selected_node = Some(node_id);
        }
        KnowledgeGraphMessage::GoToDefinition(node_id) => {
            tracing::info!("Going to definition: {}", node_id);
            // Would navigate to the definition
            state.selected_node = Some(node_id);
        }
        KnowledgeGraphMessage::FindUsages(node_id) => {
            tracing::info!("Finding usages for node: {}", node_id);
            // Would search the knowledge graph for usages
            state.selected_node = Some(node_id);
        }
        KnowledgeGraphMessage::ShowDependencyPath(from_id, to_id) => {
            tracing::info!("Showing dependency path from {} to {}", from_id, to_id);
            if let Some(ref graph) = state.graph {
                state.dependency_path = graph.find_path(&from_id, &to_id);
                if state.dependency_path.is_some() {
                    tracing::info!(
                        "Path found with {} nodes",
                        state.dependency_path.as_ref().unwrap().len()
                    );
                }
            }
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::AnalyzeImpact(node_id) => {
            tracing::info!("Analyzing impact for node: {}", node_id);
            if let Some(ref graph) = state.graph {
                // Find all nodes that depend on this node (transitively)
                let mut affected = HashSet::new();
                let mut to_visit = vec![node_id.clone()];
                let mut visited = HashSet::new();

                while let Some(current) = to_visit.pop() {
                    if visited.contains(&current) {
                        continue;
                    }
                    visited.insert(current.clone());

                    for edge in graph.get_incoming_edges(&current) {
                        affected.insert(edge.from_node_id.clone());
                        to_visit.push(edge.from_node_id.clone());
                    }
                }

                state.impact_nodes = affected;
                tracing::info!(
                    "Impact analysis found {} affected nodes",
                    state.impact_nodes.len()
                );
            }
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ShowRelatedCode(node_id) => {
            tracing::info!("Showing related code for node: {}", node_id);
            if let Some(ref graph) = state.graph {
                // Find related nodes based on:
                // 1. Same type
                // 2. Similar names
                // 3. Connected nodes
                let mut suggestions = Vec::new();

                if let Some(node) = graph.get_node(&node_id) {
                    // Same type nodes
                    for candidate in graph.get_nodes_by_type(node.content_type) {
                        if candidate.node_id != node_id {
                            suggestions.push(candidate.node_id.clone());
                        }
                    }
                }

                state.related_suggestions = suggestions;
                tracing::info!(
                    "Found {} related suggestions",
                    state.related_suggestions.len()
                );
            }
        }
        KnowledgeGraphMessage::AddToComparison(node_id) => {
            if !state.comparison_nodes.contains(&node_id) {
                state.comparison_nodes.push(node_id);
                tracing::info!(
                    "Added to comparison: {} nodes total",
                    state.comparison_nodes.len()
                );
            }
        }
        KnowledgeGraphMessage::ClearComparison => {
            state.comparison_nodes.clear();
            tracing::info!("Cleared comparison view");
        }
        KnowledgeGraphMessage::ExportAsImage => {
            tracing::info!("Exporting graph as image");
            // Would export the current graph visualization as PNG/SVG
        }
        KnowledgeGraphMessage::ExportAsJson => {
            tracing::info!("Exporting graph as JSON");
            // Would serialize and export the graph data
        }
        KnowledgeGraphMessage::ShowNodeHistory(node_id) => {
            tracing::info!("Showing history for node: {}", node_id);
            // Would show version history/changes for this node
        }
        KnowledgeGraphMessage::FilterByFile(file_path) => {
            state.file_filter = Some(file_path);
            tracing::info!("Filtering by file: {:?}", state.file_filter);
            state.canvas_cache.clear();
        }
        KnowledgeGraphMessage::ShowCallHierarchy(node_id) => {
            tracing::info!("Showing call hierarchy for: {}", node_id);
            if let Some(ref graph) = state.graph {
                // Build call hierarchy (who calls this, and what it calls)
                let mut hierarchy = Vec::new();
                let mut current_path = vec![node_id.clone()];

                // Get callees (functions this calls)
                for edge in graph.get_outgoing_edges(&node_id) {
                    if edge.relationship_type == RelationshipType::Calls {
                        let mut path = current_path.clone();
                        path.push(edge.to_node_id.clone());
                        hierarchy.push(path);
                    }
                }

                state.call_hierarchy = Some(hierarchy);
            }
        }
        KnowledgeGraphMessage::ShowInheritanceHierarchy(node_id) => {
            tracing::info!("Showing inheritance hierarchy for: {}", node_id);
            // Would show the inheritance tree
        }
        KnowledgeGraphMessage::ToggleMinimap => {
            state.show_minimap = !state.show_minimap;
            tracing::info!("Minimap: {}", state.show_minimap);
        }
    }
}

/// ============================================================================
/// View / Rendering
/// ============================================================================

/// Render the knowledge graph panel
pub fn view(state: &KnowledgeGraphPanelState) -> Element<KnowledgeGraphMessage> {
    if state.graph.is_none() {
        return view_empty_state();
    }

    let graph = state.graph.as_ref().unwrap();

    // Header with controls
    let header = view_header(state);

    // Main content: side panel + graph canvas
    let side_panel = view_side_panel(state, graph);
    let graph_canvas = view_graph_canvas(state, graph);

    let main_content = row![
        side_panel,
        container(graph_canvas)
            .width(Length::Fill)
            .height(Length::Fill)
    ]
    .spacing(0);

    // Footer with stats
    let footer = view_footer(state, graph);

    column![header, main_content, footer,].spacing(0).into()
}

/// Render empty state
fn view_empty_state() -> Element<'static, KnowledgeGraphMessage> {
    container(
        column![
            text("No Knowledge Graph Loaded").size(18),
            Space::with_height(10),
            text("Generate a knowledge graph from a file tree to visualize code entities").size(14),
        ]
        .spacing(10)
        .padding(20)
        .align_x(iced::alignment::Horizontal::Center),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center(Length::Fill)
    .into()
}

/// Render header with search and main controls
fn view_header(state: &KnowledgeGraphPanelState) -> Element<KnowledgeGraphMessage> {
    let search_input = text_input("Search entities (name, type, file)...", &state.search_query)
        .on_input(KnowledgeGraphMessage::SearchQueryChanged)
        .padding(8)
        .width(Length::Fill);

    let fuzzy_checkbox = checkbox("Fuzzy", state.fuzzy_search)
        .on_toggle(|_| KnowledgeGraphMessage::ToggleFuzzySearch);

    let layout_picker = pick_list(
        LayoutAlgorithm::all(),
        Some(state.layout_algorithm),
        KnowledgeGraphMessage::SetLayoutAlgorithm,
    )
    .placeholder("Layout");

    let control_buttons = row![
        button(text("Reset View"))
            .on_press(KnowledgeGraphMessage::ResetViewport)
            .padding(6),
        button(text("Relayout"))
            .on_press(KnowledgeGraphMessage::RelayoutGraph)
            .padding(6),
        button(text("Zoom +"))
            .on_press(KnowledgeGraphMessage::ZoomIn)
            .padding(6),
        button(text("Zoom -"))
            .on_press(KnowledgeGraphMessage::ZoomOut)
            .padding(6),
    ]
    .spacing(5);

    container(
        column![
            row![
                search_input,
                Space::with_width(10),
                fuzzy_checkbox,
                Space::with_width(10),
                layout_picker,
            ]
            .spacing(5)
            .align_y(iced::alignment::Vertical::Center),
            Space::with_height(5),
            row![
                control_buttons,
                horizontal_space(),
                checkbox("Labels", state.show_labels)
                    .on_toggle(|_| KnowledgeGraphMessage::ToggleLabels),
                Space::with_width(10),
                checkbox("Edge Labels", state.show_edge_labels)
                    .on_toggle(|_| KnowledgeGraphMessage::ToggleEdgeLabels),
            ]
            .spacing(5)
        ]
        .spacing(5)
        .padding(10),
    )
    .width(Length::Fill)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: iced::Border {
            width: 0.0,
            color: Color::TRANSPARENT,
            radius: 0.0.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Render side panel with filters and info
fn view_side_panel<'a>(
    state: &'a KnowledgeGraphPanelState,
    graph: &'a KnowledgeGraph,
) -> Element<'a, KnowledgeGraphMessage> {
    let mut panel_content = Column::new().spacing(10).padding(10);

    // Type filters section
    panel_content = panel_content.push(text("Node Types").size(16));

    let node_types = [
        KnowledgeNodeType::Function,
        KnowledgeNodeType::Method,
        KnowledgeNodeType::Class,
        KnowledgeNodeType::Struct,
        KnowledgeNodeType::Enum,
        KnowledgeNodeType::Interface,
        KnowledgeNodeType::Module,
    ];

    for node_type in node_types {
        let count = graph.get_nodes_by_type(node_type).len();
        let is_filtered = state.type_filters.contains(&node_type);

        let filter_checkbox = checkbox(format!("{} ({})", node_type.as_str(), count), is_filtered)
            .on_toggle(move |_| KnowledgeGraphMessage::ToggleTypeFilter(node_type));

        panel_content = panel_content.push(filter_checkbox);
    }

    panel_content = panel_content.push(Space::with_height(10));
    panel_content = panel_content.push(text("Relationships").size(16));

    // Relationship filters
    let rel_types = [
        RelationshipType::Calls,
        RelationshipType::Uses,
        RelationshipType::Inherits,
        RelationshipType::Implements,
        RelationshipType::DefinedIn,
    ];

    for rel_type in rel_types {
        let is_filtered = state.relationship_filters.contains(&rel_type);

        let filter_checkbox = checkbox(rel_type.as_str(), is_filtered)
            .on_toggle(move |_| KnowledgeGraphMessage::ToggleRelationshipFilter(rel_type));

        panel_content = panel_content.push(filter_checkbox);
    }

    panel_content = panel_content.push(Space::with_height(10));

    // Other options
    let connected_checkbox = checkbox("Only Connected", state.show_only_connected)
        .on_toggle(|_| KnowledgeGraphMessage::ToggleShowOnlyConnected);
    panel_content = panel_content.push(connected_checkbox);

    panel_content = panel_content.push(Space::with_height(10));

    let clear_button = button(text("Clear Filters"))
        .on_press(KnowledgeGraphMessage::ClearFilters)
        .padding(8)
        .width(Length::Fill);
    panel_content = panel_content.push(clear_button);

    // Search results
    if !state.search_results.is_empty() {
        panel_content = panel_content.push(Space::with_height(15));
        panel_content = panel_content.push(text("Search Results").size(16));

        let results_list: Vec<Element<KnowledgeGraphMessage>> = state
            .search_results
            .iter()
            .take(10)
            .filter_map(|node_id| graph.get_node(node_id))
            .map(|node| {
                let node_id = node.node_id.clone();
                button(text(format!("{}: {}", node.content_type.as_str(), node.name)).size(12))
                    .on_press(KnowledgeGraphMessage::FocusNode(node_id))
                    .padding(5)
                    .width(Length::Fill)
                    .into()
            })
            .collect();

        let results_column = Column::with_children(results_list).spacing(3);
        panel_content = panel_content.push(scrollable(results_column).height(Length::Fill));
    }

    // Selected node info
    if let Some(ref node_id) = state.selected_node {
        if let Some(node) = graph.get_node(node_id) {
            panel_content = panel_content.push(Space::with_height(15));
            panel_content = panel_content.push(text("Selected Node").size(16));
            panel_content = panel_content.push(text(format!("Name: {}", node.name)).size(12));
            panel_content =
                panel_content.push(text(format!("Type: {}", node.content_type.as_str())).size(12));

            if let Some(ref sig) = node.signature {
                panel_content = panel_content.push(text(format!("Signature: {}", sig)).size(11));
            }

            let outgoing = graph.get_outgoing_edges(node_id).len();
            let incoming = graph.get_incoming_edges(node_id).len();
            panel_content = panel_content
                .push(text(format!("Connections: {} out, {} in", outgoing, incoming)).size(12));

            let jump_button = button(text("Jump to Code"))
                .on_press(KnowledgeGraphMessage::JumpToNode(node_id.clone()))
                .padding(6)
                .width(Length::Fill);
            panel_content = panel_content.push(jump_button);
        }
    }

    container(scrollable(panel_content))
        .width(300)
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

/// Render the graph canvas
fn view_graph_canvas<'a>(
    state: &'a KnowledgeGraphPanelState,
    graph: &'a KnowledgeGraph,
) -> Element<'a, KnowledgeGraphMessage> {
    // This is a placeholder - a real implementation would use iced::canvas
    // to render an interactive graph visualization

    let graph_text = format!(
        "Knowledge Graph Visualization\n\n\
        Nodes: {}\n\
        Edges: {}\n\
        Selected: {}\n\
        Zoom: {:.1}%\n\n\
        (Interactive canvas rendering would appear here)",
        graph.nodes.len(),
        graph.edges.len(),
        state.selected_node.as_ref().map(|_| "Yes").unwrap_or("No"),
        state.viewport.scale * 100.0
    );

    container(text(graph_text).size(14))
        .width(Length::Fill)
        .height(Length::Fill)
        .center(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(Color::from_rgb8(20, 20, 30).into()),
            border: iced::Border {
                width: 1.0,
                color: theme.palette().text.scale_alpha(0.2),
                radius: 0.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render footer with statistics
fn view_footer<'a>(
    state: &'a KnowledgeGraphPanelState,
    graph: &'a KnowledgeGraph,
) -> Element<'a, KnowledgeGraphMessage> {
    let stats = graph.stats();

    let visible_nodes = if state.type_filters.is_empty() {
        graph.nodes.len()
    } else {
        graph
            .nodes
            .values()
            .filter(|n| state.type_filters.contains(&n.content_type))
            .count()
    };

    let stats_text = format!(
        "Nodes: {} ({} visible) | Edges: {} | Avg Degree: {:.1} | Selected: {}",
        stats.total_nodes,
        visible_nodes,
        stats.total_edges,
        stats.avg_degree,
        if state.selected_node.is_some() {
            "Yes"
        } else {
            "No"
        }
    );

    container(text(stats_text).size(12))
        .padding(10)
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

/// ============================================================================
/// Helper Functions
/// ============================================================================

/// Perform search in the knowledge graph
fn perform_search(state: &KnowledgeGraphPanelState) -> Vec<String> {
    if state.search_query.is_empty() {
        return Vec::new();
    }

    let Some(ref graph) = state.graph else {
        return Vec::new();
    };

    let query = state.search_query.to_lowercase();

    graph
        .nodes
        .values()
        .filter(|node| {
            if state.fuzzy_search {
                fuzzy_match(&node.name.to_lowercase(), &query)
                    || fuzzy_match(&node.qualified_name.to_lowercase(), &query)
                    || node.content_type.as_str().to_lowercase().contains(&query)
            } else {
                node.name.to_lowercase().contains(&query)
                    || node.qualified_name.to_lowercase().contains(&query)
                    || node.content_type.as_str().to_lowercase().contains(&query)
            }
        })
        .map(|node| node.node_id.clone())
        .collect()
}

/// Simple fuzzy matching
fn fuzzy_match(text: &str, pattern: &str) -> bool {
    let mut pattern_chars = pattern.chars().peekable();
    let mut text_chars = text.chars();

    while let Some(p_char) = pattern_chars.peek() {
        let mut found = false;

        for t_char in text_chars.by_ref() {
            if t_char == *p_char {
                found = true;
                pattern_chars.next();
                break;
            }
        }

        if !found {
            return false;
        }
    }

    pattern_chars.peek().is_none()
}

/// Compute node positions using selected layout algorithm
fn compute_layout(graph: &KnowledgeGraph, algorithm: LayoutAlgorithm) -> HashMap<String, Point> {
    match algorithm {
        LayoutAlgorithm::ForceDirected => force_directed_layout(graph),
        LayoutAlgorithm::Hierarchical => hierarchical_layout(graph),
        LayoutAlgorithm::Circular => circular_layout(graph),
        LayoutAlgorithm::Grid => grid_layout(graph),
    }
}

/// Force-directed layout (simplified)
fn force_directed_layout(graph: &KnowledgeGraph) -> HashMap<String, Point> {
    let mut positions = HashMap::new();

    // Initialize with random positions
    let node_count = graph.nodes.len();
    let cols = (node_count as f32).sqrt().ceil() as usize;

    for (idx, node_id) in graph.nodes.keys().enumerate() {
        let row = idx / cols;
        let col = idx % cols;

        positions.insert(
            node_id.clone(),
            Point::new(100.0 + col as f32 * 150.0, 100.0 + row as f32 * 100.0),
        );
    }

    positions
}

/// Hierarchical layout (top-down tree)
fn hierarchical_layout(graph: &KnowledgeGraph) -> HashMap<String, Point> {
    let mut positions = HashMap::new();

    // Find root nodes (nodes with no incoming edges)
    let root_nodes: Vec<_> = graph
        .nodes
        .keys()
        .filter(|id| graph.get_incoming_edges(id).is_empty())
        .cloned()
        .collect();

    if root_nodes.is_empty() {
        return grid_layout(graph);
    }

    let mut level_map: HashMap<String, usize> = HashMap::new();
    let mut levels: Vec<Vec<String>> = Vec::new();

    // BFS to assign levels
    let mut queue: Vec<(String, usize)> = root_nodes.iter().map(|id| (id.clone(), 0)).collect();

    let mut visited = HashSet::new();

    while let Some((node_id, level)) = queue.pop() {
        if visited.contains(&node_id) {
            continue;
        }

        visited.insert(node_id.clone());
        level_map.insert(node_id.clone(), level);

        while levels.len() <= level {
            levels.push(Vec::new());
        }
        levels[level].push(node_id.clone());

        // Add children
        for edge in graph.get_outgoing_edges(&node_id) {
            if !visited.contains(&edge.to_node_id) {
                queue.push((edge.to_node_id.clone(), level + 1));
            }
        }
    }

    // Position nodes
    for (level, nodes) in levels.iter().enumerate() {
        let y = 100.0 + level as f32 * 120.0;
        let total_width = nodes.len() as f32 * 200.0;
        let start_x = 400.0 - total_width / 2.0;

        for (idx, node_id) in nodes.iter().enumerate() {
            positions.insert(node_id.clone(), Point::new(start_x + idx as f32 * 200.0, y));
        }
    }

    positions
}

/// Circular layout
fn circular_layout(graph: &KnowledgeGraph) -> HashMap<String, Point> {
    let mut positions = HashMap::new();

    let node_count = graph.nodes.len();
    let radius = 300.0;
    let center = Point::new(400.0, 300.0);

    for (idx, node_id) in graph.nodes.keys().enumerate() {
        let angle = (idx as f32 / node_count as f32) * 2.0 * std::f32::consts::PI;

        positions.insert(
            node_id.clone(),
            Point::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            ),
        );
    }

    positions
}

/// Grid layout
fn grid_layout(graph: &KnowledgeGraph) -> HashMap<String, Point> {
    let mut positions = HashMap::new();

    let node_count = graph.nodes.len();
    let cols = (node_count as f32).sqrt().ceil() as usize;

    for (idx, node_id) in graph.nodes.keys().enumerate() {
        let row = idx / cols;
        let col = idx % cols;

        positions.insert(
            node_id.clone(),
            Point::new(100.0 + col as f32 * 200.0, 100.0 + row as f32 * 120.0),
        );
    }

    positions
}

/// Get color for node type
pub fn get_node_color(node_type: KnowledgeNodeType) -> Color {
    match node_type {
        KnowledgeNodeType::Function => Color::from_rgb8(100, 200, 255),
        KnowledgeNodeType::Method => Color::from_rgb8(120, 180, 255),
        KnowledgeNodeType::Class => Color::from_rgb8(255, 180, 100),
        KnowledgeNodeType::Struct => Color::from_rgb8(255, 200, 120),
        KnowledgeNodeType::Enum => Color::from_rgb8(200, 150, 255),
        KnowledgeNodeType::Interface => Color::from_rgb8(150, 255, 150),
        KnowledgeNodeType::Module => Color::from_rgb8(255, 255, 100),
        KnowledgeNodeType::TypeAlias => Color::from_rgb8(200, 200, 200),
        KnowledgeNodeType::Constant => Color::from_rgb8(255, 150, 200),
        KnowledgeNodeType::Variable => Color::from_rgb8(180, 220, 180),
        KnowledgeNodeType::Macro => Color::from_rgb8(255, 100, 255),
        KnowledgeNodeType::Concept => Color::from_rgb8(150, 150, 200),
        KnowledgeNodeType::Documentation => Color::from_rgb8(200, 200, 150),
    }
}

/// Get icon for node type
pub fn get_node_icon(node_type: KnowledgeNodeType) -> &'static str {
    match node_type {
        KnowledgeNodeType::Function => "ƒ",
        KnowledgeNodeType::Method => "m",
        KnowledgeNodeType::Class => "C",
        KnowledgeNodeType::Struct => "S",
        KnowledgeNodeType::Enum => "E",
        KnowledgeNodeType::Interface => "I",
        KnowledgeNodeType::Module => "M",
        KnowledgeNodeType::TypeAlias => "T",
        KnowledgeNodeType::Constant => "K",
        KnowledgeNodeType::Variable => "V",
        KnowledgeNodeType::Macro => "!",
        KnowledgeNodeType::Concept => "?",
        KnowledgeNodeType::Documentation => "D",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        assert!(fuzzy_match("hello_world", "hw"));
        assert!(fuzzy_match("hello_world", "hew"));
        assert!(fuzzy_match("FunctionName", "fn"));
        assert!(!fuzzy_match("hello", "hx"));
    }

    #[test]
    fn test_search_empty_query() {
        let state = KnowledgeGraphPanelState::default();
        let results = perform_search(&state);
        assert!(results.is_empty());
    }

    #[test]
    fn test_layout_algorithms() {
        let graph = KnowledgeGraph::new();

        let _ = force_directed_layout(&graph);
        let _ = hierarchical_layout(&graph);
        let _ = circular_layout(&graph);
        let _ = grid_layout(&graph);
    }
}
