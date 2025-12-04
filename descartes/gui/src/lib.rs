#![allow(mismatched_lifetime_syntaxes)]
#![allow(dead_code)]

pub mod code_preview_panel;
pub mod dag_canvas_interactions;
pub mod dag_editor;
pub mod debugger_ui;
pub mod event_handler;
#[cfg(feature = "agent-runner")]
pub mod file_tree_view;
#[cfg(feature = "agent-runner")]
pub mod knowledge_graph_panel;
/// Descartes GUI - Native cross-platform interface using Iced
/// Phase 3 implementation
pub mod rpc_client;
pub mod rpc_unix_client; // Unix socket RPC client (preferred for local IPC)
pub mod swarm_handler; // Stream handler for swarm events
pub mod swarm_monitor; // Live swarm monitoring UI (phase 3:5.5)
pub mod task_board;
pub mod time_travel;
pub mod time_travel_integration;

pub use code_preview_panel::{
    get_visible_range, is_line_visible, load_file, next_bookmark, previous_bookmark,
    update as code_preview_update, view as code_preview_view, CodePreviewMessage, CodePreviewState,
    ViewMode,
};
pub use dag_editor::{
    update as dag_editor_update, view as dag_editor_view, DAGEditorMessage, DAGEditorState, Tool,
};
pub use event_handler::EventHandler;
#[cfg(feature = "agent-runner")]
pub use file_tree_view::{
    get_selected_node, get_selected_path, is_node_visible, update as file_tree_update,
    view as file_tree_view, FileTreeMessage, FileTreeState, SortOrder,
};
#[cfg(feature = "agent-runner")]
pub use knowledge_graph_panel::{
    get_node_color, get_node_icon, update as knowledge_graph_panel_update,
    view as knowledge_graph_panel_view, KnowledgeGraphMessage, KnowledgeGraphPanelState,
    LayoutAlgorithm, VisualizationMode,
};
pub use rpc_client::GuiRpcClient;
pub use rpc_unix_client::GuiUnixRpcClient;
pub use swarm_handler::{generate_sample_agents, GuiStreamHandler};
pub use swarm_monitor::{
    subscription as swarm_monitor_subscription, update as swarm_monitor_update,
    view as swarm_monitor_view, AgentEvent, AgentFilter as SwarmAgentFilter, ConnectionStatus,
    PerformanceStats, SwarmMonitorMessage, SwarmMonitorState,
};
pub use task_board::{
    update as task_board_update, view as task_board_view, KanbanBoard, TaskBoardMessage,
    TaskBoardState, TaskFilters, TaskSort,
};
pub use time_travel::{
    update as time_travel_update, view as time_travel_view, PlaybackState, TimeTravelMessage,
    TimeTravelState, TimelineSettings,
};
pub use time_travel_integration::{
    is_rewind_safe, slider_selection_to_rewind_point, update_rewind, view_rewind_confirmation,
    view_rewind_controls, view_rewind_progress, view_rewind_result, RewindMessage, RewindState,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Placeholder for Iced GUI implementation
pub struct DescarterGui;

impl DescarterGui {
    /// Create a new GUI instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for DescarterGui {
    fn default() -> Self {
        Self::new()
    }
}
