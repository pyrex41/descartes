// Descartes: Composable AI Agent Orchestration System
// Core library providing traits, providers, and orchestration utilities
#![allow(mismatched_lifetime_syntaxes)]

pub mod agent_definitions;
pub mod agent_history;
pub mod attach;
pub mod attach_protocol;
pub mod agent_runner;
pub mod agent_state;
pub mod agent_stream_parser;
pub mod body_restore;
pub mod brain_restore;
pub mod channel_bridge;
pub mod claude_backend;
pub mod cli_backend;
pub mod config;
pub mod config_loader;
pub mod config_migration;
pub mod config_watcher;
pub mod dag;
pub mod dag_swarm_export;
pub mod dag_toml;
pub mod debugger;
pub mod errors;
pub mod expression_eval;
pub mod lease;
pub mod lease_manager;
pub mod providers;
pub mod secrets;
pub mod secrets_crypto;
pub mod state_machine;
pub mod state_machine_store;
pub mod state_store;
pub mod swarm_parser;
pub mod task_queries;
pub mod scg_task_storage;
pub mod scud_plugin;
pub mod thoughts;
pub mod time_travel_integration;
pub mod traits;
pub mod workflow_commands;
pub mod workflow_executor;
pub mod zmq_agent_runner;
pub mod zmq_client;
pub mod zmq_communication;
pub mod zmq_server;

// Session/workspace management
pub mod session;
pub mod session_manager;
pub mod session_transcript;

// Daemon lifecycle management
pub mod daemon_launcher;

// Minimal tool definitions (Pi-style)
pub mod tools;

// Re-export commonly used types
pub use errors::{
    AgentError, AgentResult, ContextError, ContextResult, ProviderError, ProviderResult,
    StateStoreError, StateStoreResult,
};

pub use traits::{
    ActorType, AgentConfig, AgentHandle, AgentInfo, AgentRunner, AgentSignal, AgentStatus,
    AttachInfo, ContextSyncer, Event, ExitStatus, FinishReason, Message, MessageRole, ModelBackend,
    ModelProviderMode, ModelRequest, ModelResponse, PauseMode, StateStore, Task, TaskComplexity,
    TaskPriority, TaskStatus, Tool, ToolCall, ToolParameters,
    // SCUD integration types (for gradual migration to unified task model)
    ScudPhase, ScudPriority, ScudStorage, ScudTask, ScudTaskStatus,
    parse_scg, serialize_scg, scud_to_task, task_to_scud,
};

pub use providers::{
    AnthropicProvider, ClaudeCodeAdapter, HeadlessCliAdapter, OllamaProvider, OpenAiProvider,
    ProviderFactory,
};

pub use agent_runner::{GracefulShutdown, LocalAgentHandle, LocalProcessRunner, ProcessRunnerConfig};

pub use attach::{AttachToken, AttachTokenStore, DEFAULT_TOKEN_TTL_SECS};

pub use attach_protocol::{
    AttachHandshake, AttachHandshakeResponse, AttachMessage, AttachMessageType, HistoricalOutput,
};

pub use agent_state::{
    AgentError as RuntimeAgentError, AgentProgress, AgentRuntimeState, AgentStateCollection,
    AgentStatistics, AgentStatus as RuntimeAgentStatus, AgentStreamMessage, LifecycleEvent,
    OutputStream, StatusTransition,
};

pub use agent_stream_parser::{
    AgentStreamParser, LoggingHandler, ParserConfig, ParserStatistics, StreamHandler,
    StreamParseError, StreamResult,
};

pub use zmq_agent_runner::{
    deserialize_zmq_message, serialize_zmq_message, validate_message_size, BatchAgentResult,
    BatchControlCommand, BatchControlResponse, CommandResponse, ControlCommand, ControlCommandType,
    CustomActionRequest, HealthCheckRequest, HealthCheckResponse, ListAgentsRequest,
    ListAgentsResponse, LogStreamMessage, LogStreamType, OutputQueryRequest, OutputQueryResponse,
    SpawnRequest, SpawnResponse, StatusUpdate, StatusUpdateType, ZmqAgentRunner, ZmqMessage,
    ZmqOutputStream, ZmqRunnerConfig, DEFAULT_TIMEOUT_SECS, MAX_MESSAGE_SIZE, ZMQ_PROTOCOL_VERSION,
};

pub use zmq_communication::{
    ConnectionState, ConnectionStats, SocketType, ZmqConnection, ZmqMessageRouter,
};

pub use zmq_client::ZmqClient;

pub use zmq_server::{ServerStats, ZmqAgentServer, ZmqServerConfig};

pub use channel_bridge::{ChannelBridge, InternalMessage};

pub use cli_backend::{ChatSessionConfig, ChatSessionHandle, CliBackend, StreamChunk};
pub use claude_backend::ClaudeBackend;

pub use lease::{
    Lease, LeaseAcquisitionRequest, LeaseAcquisitionResponse, LeaseManager, LeaseReleaseRequest,
    LeaseReleaseResponse, LeaseRenewalRequest, LeaseRenewalResponse, LeaseStatus,
};

pub use lease_manager::SqliteLeaseManager;

pub use state_store::{AgentState, Migration, SqliteStateStore, StateTransition};

pub use task_queries::{
    KanbanBoard, SortOrder, TaskQueries, TaskQueryBuilder, TaskSortField, TaskStatistics,
};

pub use scg_task_storage::{
    ScgPhaseStats, ScgSortField, ScgSortOrder, ScgTaskQueries, ScgTaskQueryBuilder, ScgTaskStorage,
};

pub use config::{
    AgentBehaviorConfig, AnthropicConfig, ConfigManager, DeepSeekConfig, DescaratesConfig,
    FeaturesConfig, GroqConfig, LoggingConfig, OllamaConfig, OpenAiConfig,
    ProvidersConfig, SecurityConfig, StorageConfig,
};

pub use config_loader::{
    ensure_config_directories, init_config, ConfigDiscoveryStrategy, ConfigLoader, ConfigValidator,
};

pub use config_migration::ConfigMigration;

pub use config_watcher::{
    ConfigChangeEvent, ConfigChangeListener, ConfigWatcher, HotReloadManager,
};

pub use state_machine::{
    DefaultStateHandler, HierarchicalState, SerializedWorkflow, StateHandler, StateHistoryEntry,
    StateMachineError, StateMachineResult, TransitionMetadata, WorkflowEvent, WorkflowMetadata,
    WorkflowOrchestrator, WorkflowState, WorkflowStateMachine,
};

pub use state_machine_store::{
    SqliteWorkflowStore, StateStoreConfig, TransitionRecord, WorkflowRecord, WorkflowRecovery,
};

pub use swarm_parser::{
    AgentConfig as SwarmAgentConfig, Contract, Handler, ResourceConfig, State, SwarmConfig,
    SwarmParseError, SwarmParser, SwarmResult, ValidatedState, ValidatedWorkflow, Workflow,
    WorkflowMetadata as SwarmWorkflowMetadata,
};

pub use thoughts::{
    parse_markdown_with_frontmatter, MarkdownDocument, StorageStatistics, ThoughtMetadata,
    ThoughtsConfig, ThoughtsError, ThoughtsResult, ThoughtsStorage, PLANS_DIR, RESEARCH_DIR,
};

pub use agent_definitions::{
    AgentDefinition, AgentDefinitionError, AgentDefinitionLoader, AgentDefinitionResult,
};

pub use workflow_commands::{
    get_workflow, list_workflows, prepare_workflow, StepResult, WorkflowCommand, WorkflowContext,
    WorkflowError, WorkflowExecutionResult, WorkflowRegistry, WorkflowResult, WorkflowStep,
};

pub use workflow_executor::{
    execute_step, execute_workflow, StepExecutionResult, WorkflowExecutionError,
    WorkflowExecutorConfig,
};

pub use scud_plugin::{
    detect_workspace_type, ensure_scud_dir, has_scud, is_dual_workspace, read_scud_tasks,
    read_scud_workflow_state, scud_available, scud_dir, scud_tasks_file, scud_version,
    scud_workflow_state_file, sync_tasks_to_scud, WorkspaceType,
};

pub use debugger::{
    run_with_debugging,
    Breakpoint,
    BreakpointLocation,
    CallFrame,
    CommandResult,
    DebugCommand,
    DebugContext,
    DebugEvent,
    DebugSnapshot,
    DebugStatistics,
    DebuggableAgent,
    // Core debugger logic (phase3:6.2)
    Debugger,
    DebuggerError,
    DebuggerResult,
    DebuggerState,
    DebuggerStateExt,
    ExecutionState,
    ThoughtSnapshot,
};

pub use expression_eval::{
    context_from_json, evaluate, evaluate_bool, EvalContext, EvalError, EvalResult, Expr,
    ExpressionEvaluator,
};

pub use agent_history::{
    AgentHistoryEvent, AgentHistoryStore, HistoryEventType, HistoryQuery, HistorySnapshot,
    HistoryStatistics, SqliteAgentHistoryStore,
};

pub use body_restore::{
    BodyRestoreManager, CommitInfo, CoordinatedRestore, GitBodyRestoreManager, RepositoryBackup,
    RestoreOptions as BodyRestoreOptions, RestoreResult as BodyRestoreResult,
};

pub use brain_restore::{
    compare_states, create_snapshot_from_state, BrainRestore, BrainState, ConversationState,
    DecisionNode, DefaultBrainRestore, MessageEntry, RestoreOptions as BrainRestoreOptions,
    RestoreResult as BrainRestoreResult, ThoughtEntry,
};

pub use time_travel_integration::{
    describe_rewind, slider_to_rewind_point, DefaultRewindManager, ResumeContext, RewindBackup,
    RewindConfig, RewindConfirmation, RewindManager, RewindPoint, RewindProgress, RewindResult,
    ValidationResult,
};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}

#[cfg(test)]
mod providers_test;

pub use dag::{DAGEdge, DAGError, DAGNode, DAGResult, DAGStatistics, EdgeType, Position, DAG};

pub use dag_toml::{
    load_dag_from_toml, save_dag_to_toml, TomlDAG, TomlDAGEdge, TomlDAGNode, TomlPosition,
    TomlTaskDependency,
};

pub use dag_swarm_export::{
    export_dag_to_swarm_toml, import_swarm_toml_to_dag, load_dag_from_swarm_toml,
    save_dag_as_swarm_toml, SwarmExportConfig,
};

pub use session::{
    DaemonInfo, Session, SessionDiscoveryConfig, SessionError, SessionManager, SessionStatus,
};

pub use session_manager::FileSystemSessionManager;

pub use daemon_launcher::{
    daemon_http_endpoint, daemon_socket_path, daemon_ws_endpoint, ensure_daemon_running,
    is_daemon_running, DEFAULT_HTTP_PORT, DEFAULT_WS_PORT,
};

pub use tools::{
    bash_tool, edit_tool, execute_bash, execute_edit, execute_read, execute_spawn_session,
    execute_tool, execute_write, get_system_prompt, get_tools, minimal_system_prompt,
    orchestrator_system_prompt, planner_system_prompt, read_tool, readonly_system_prompt,
    researcher_system_prompt, spawn_session_tool, write_tool, ToolLevel, ToolResult,
};

pub use session_transcript::{
    default_sessions_dir, TranscriptEntry, TranscriptMetadata, TranscriptWriter,
};
