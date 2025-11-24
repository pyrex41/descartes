/// Descartes GUI - Native cross-platform interface using Iced
/// Phase 3 implementation

pub mod rpc_client;
pub mod rpc_unix_client; // Unix socket RPC client (preferred for local IPC)
pub mod event_handler;
pub mod time_travel;
pub mod time_travel_integration;
pub mod task_board;
pub mod dag_editor;
pub mod file_tree_view;

pub use rpc_client::GuiRpcClient;
pub use rpc_unix_client::GuiUnixRpcClient;
pub use event_handler::EventHandler;
pub use time_travel::{
    TimeTravelState, TimeTravelMessage, PlaybackState, TimelineSettings,
    view as time_travel_view, update as time_travel_update,
};
pub use time_travel_integration::{
    RewindMessage, RewindState, view_rewind_confirmation, view_rewind_progress,
    view_rewind_result, view_rewind_controls, update_rewind,
    slider_selection_to_rewind_point, is_rewind_safe,
};
pub use task_board::{
    TaskBoardState, TaskBoardMessage, TaskFilters, TaskSort, KanbanBoard,
    view as task_board_view, update as task_board_update,
};
pub use dag_editor::{
    DAGEditorState, DAGEditorMessage, Tool,
    view as dag_editor_view, update as dag_editor_update,
};
pub use file_tree_view::{
    FileTreeState, FileTreeMessage, SortOrder,
    view as file_tree_view, update as file_tree_update,
    get_selected_node, get_selected_path, is_node_visible,
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
