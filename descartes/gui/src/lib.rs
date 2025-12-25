#![allow(mismatched_lifetime_syntaxes)]
#![allow(dead_code)]

pub mod dag_canvas_interactions;
pub mod dag_editor;
pub mod debugger_ui;
pub mod event_handler;
pub mod lisp_debugger;
pub mod theme;
/// Descartes GUI - Native cross-platform interface using Iced
/// Phase 3 implementation
pub mod rpc_client;
pub mod rpc_unix_client; // Unix socket RPC client (preferred for local IPC)
pub mod swarm_handler; // Stream handler for swarm events
pub mod swarm_monitor; // Live swarm monitoring UI (phase 3:5.5)
pub mod zmq_subscriber; // ZMQ SUB client for chat streaming
pub mod chat_state; // Chat interface state management
pub mod task_board;
pub mod time_travel;
pub mod time_travel_integration;

pub use dag_editor::{
    update as dag_editor_update, view as dag_editor_view, DAGEditorMessage, DAGEditorState, Tool,
};
pub use event_handler::EventHandler;
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
pub use zmq_subscriber::{chat_subscription, subscribe_to_session};
pub use chat_state::{update as chat_state_update, ChatMessage, ChatMessageEntry, ChatRole, ChatState};
pub use lisp_debugger::{
    update as lisp_debugger_update, view as lisp_debugger_view, parse_debugger_event,
    LispDebuggerMessage, LispDebuggerState, LispFrame, LispRestart,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
