// Descartes: Composable AI Agent Orchestration System
// Core library providing traits, providers, and orchestration utilities

pub mod agent_history;
pub mod agent_runner;
pub mod agent_state;
pub mod agent_stream_parser;
pub mod body_restore;
pub mod brain_restore;
pub mod config;
pub mod config_loader;
pub mod config_migration;
pub mod config_watcher;
pub mod dag;
pub mod dag_toml;
pub mod dag_swarm_export;
pub mod debugger;
pub mod errors;
pub mod ipc;
pub mod lease;
pub mod lease_manager;
pub mod notifications;
pub mod notification_router_impl;
pub mod providers;
pub mod secrets;
pub mod secrets_crypto;
pub mod state_machine;
pub mod state_machine_store;
pub mod state_store;
pub mod swarm_parser;
pub mod task_queries;
pub mod thoughts;
pub mod time_travel_integration;
pub mod traits;
pub mod zmq_agent_runner;
pub mod zmq_communication;
pub mod zmq_client;
pub mod zmq_server;

// Re-export commonly used types
pub use errors::{
    AgentError, AgentResult, ContextError, ContextResult, ProviderError, ProviderResult,
    StateStoreError, StateStoreResult,
};

pub use traits::{
    ActorType, AgentConfig, AgentHandle, AgentInfo, AgentRunner, AgentSignal, AgentStatus,
    ContextSyncer, Event, ExitStatus, FinishReason, Message, MessageRole, ModelBackend,
    ModelProviderMode, ModelRequest, ModelResponse, StateStore, Task, TaskStatus, Tool,
    ToolCall, ToolParameters,
};

pub use providers::{
    AnthropicProvider, ClaudeCodeAdapter, HeadlessCliAdapter, OllamaProvider, OpenAiProvider,
    ProviderFactory,
};

pub use agent_runner::{
    LocalProcessRunner, ProcessRunnerConfig, GracefulShutdown,
};

pub use agent_state::{
    AgentStatus as RuntimeAgentStatus, AgentRuntimeState, AgentProgress, AgentError as RuntimeAgentError,
    StatusTransition, AgentStreamMessage, OutputStream, LifecycleEvent,
    AgentStateCollection, AgentStatistics,
};

pub use agent_stream_parser::{
    AgentStreamParser, StreamHandler, ParserConfig, StreamParseError, StreamResult,
    ParserStatistics, LoggingHandler,
};

pub use zmq_agent_runner::{
    ZmqAgentRunner, ZmqMessage, ZmqRunnerConfig,
    SpawnRequest, SpawnResponse,
    ControlCommand, ControlCommandType, CommandResponse,
    StatusUpdate, StatusUpdateType,
    ListAgentsRequest, ListAgentsResponse,
    HealthCheckRequest, HealthCheckResponse,
    CustomActionRequest,
    BatchControlCommand, BatchControlResponse, BatchAgentResult,
    OutputQueryRequest, OutputQueryResponse, ZmqOutputStream,
    serialize_zmq_message, deserialize_zmq_message, validate_message_size,
    ZMQ_PROTOCOL_VERSION, MAX_MESSAGE_SIZE, DEFAULT_TIMEOUT_SECS,
};

pub use zmq_communication::{
    ZmqConnection, ZmqMessageRouter, SocketType, ConnectionState, ConnectionStats,
};

pub use zmq_client::{
    ZmqClient,
};

pub use zmq_server::{
    ZmqAgentServer, ZmqServerConfig, ServerStats,
};

pub use notifications::{
    ChannelStats, NotificationAdapter, NotificationChannel, NotificationError,
    NotificationEventType, NotificationPayload, NotificationPayloadBuilder, NotificationSendResult,
    NotificationRouter, NotificationStats, RateLimitConfig, RoutingRule, RetryConfig, Severity,
    TemplateContext, EventTypeStats,
};

pub use notification_router_impl::DefaultNotificationRouter;

pub use ipc::{
    BackpressureConfig, BackpressureController, DeadLetterQueue, IpcMessage, MessageBus,
    MessageBusConfig, MessageBusStats, MessageHandler, MessageRouter, MessageTransport,
    MessageType, MemoryTransport, RequestResponseTracker, UnixSocketTransport,
};

pub use lease::{
    Lease, LeaseStatus, LeaseAcquisitionRequest, LeaseAcquisitionResponse,
    LeaseRenewalRequest, LeaseRenewalResponse, LeaseReleaseRequest, LeaseReleaseResponse,
    LeaseManager,
};

pub use lease_manager::SqliteLeaseManager;

pub use state_store::{
    SqliteStateStore, AgentState, StateTransition, Migration,
};

pub use task_queries::{
    TaskQueries, TaskQueryBuilder, TaskSortField, SortOrder,
    KanbanBoard, TaskStatistics,
};

pub use config::{
    ConfigManager, DescaratesConfig, ProvidersConfig, AgentBehaviorConfig, StorageConfig,
    SecurityConfig, NotificationsConfig, FeaturesConfig, LoggingConfig,
    OpenAiConfig, AnthropicConfig, OllamaConfig, DeepSeekConfig, GroqConfig,
};

pub use config_loader::{
    ConfigLoader, ConfigDiscoveryStrategy, init_config, ensure_config_directories,
    ConfigValidator,
};

pub use config_migration::{
    ConfigMigration,
};

pub use config_watcher::{
    ConfigWatcher, HotReloadManager, ConfigChangeEvent, ConfigChangeListener,
};

pub use state_machine::{
    WorkflowState, WorkflowEvent, WorkflowStateMachine, WorkflowOrchestrator,
    StateMachineError, StateMachineResult, StateHandler, DefaultStateHandler,
    TransitionMetadata, StateHistoryEntry, WorkflowMetadata, HierarchicalState,
    SerializedWorkflow,
};

pub use state_machine_store::{
    SqliteWorkflowStore, StateStoreConfig, WorkflowRecord, TransitionRecord,
    WorkflowRecovery,
};

pub use swarm_parser::{
    SwarmConfig, SwarmParser, SwarmParseError, SwarmResult, WorkflowMetadata as SwarmWorkflowMetadata,
    AgentConfig as SwarmAgentConfig, ResourceConfig, Workflow, State, Handler, Contract,
    ValidatedWorkflow, ValidatedState,
};

pub use thoughts::{
    ThoughtsStorage, ThoughtsConfig, ThoughtMetadata, ThoughtsError, ThoughtsResult,
    StorageStatistics,
};

pub use debugger::{
    DebuggerState, DebuggerError, DebuggerResult, ExecutionState, ThoughtSnapshot,
    CallFrame, DebugContext, BreakpointLocation, Breakpoint, DebugCommand,
    DebugEvent, DebugSnapshot, DebugStatistics, DebuggerStateExt,
    // Core debugger logic (phase3:6.2)
    Debugger, CommandResult, DebuggableAgent, run_with_debugging,
};

pub use agent_history::{
    AgentHistoryEvent, AgentHistoryStore, HistoryEventType, HistoryQuery, HistorySnapshot,
    HistoryStatistics, SqliteAgentHistoryStore,
};

pub use body_restore::{
    BodyRestoreManager, CommitInfo, CoordinatedRestore, GitBodyRestoreManager,
    RepositoryBackup,
    RestoreOptions as BodyRestoreOptions,
    RestoreResult as BodyRestoreResult,
};

pub use brain_restore::{
    BrainRestore, BrainState, DefaultBrainRestore,
    RestoreOptions as BrainRestoreOptions,
    RestoreResult as BrainRestoreResult,
    ThoughtEntry, DecisionNode, ConversationState, MessageEntry,
    create_snapshot_from_state, compare_states,
};

pub use time_travel_integration::{
    RewindManager, DefaultRewindManager, RewindConfig, RewindPoint, RewindResult,
    RewindBackup, ValidationResult, ResumeContext, RewindProgress, RewindConfirmation,
    slider_to_rewind_point, describe_rewind,
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

pub use dag::{
    DAG, DAGNode, DAGEdge, DAGError, DAGResult, DAGStatistics, EdgeType, Position,
};

pub use dag_toml::{
    TomlDAG, TomlDAGNode, TomlDAGEdge, TomlTaskDependency, TomlPosition,
    load_dag_from_toml, save_dag_to_toml,
};

pub use dag_swarm_export::{
    SwarmExportConfig, export_dag_to_swarm_toml, import_swarm_toml_to_dag,
    save_dag_as_swarm_toml, load_dag_from_swarm_toml,
};
