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
pub mod dag_swarm_export;
pub mod dag_toml;
pub mod debugger;
pub mod errors;
pub mod ipc;
pub mod lease;
pub mod lease_manager;
pub mod notification_router_impl;
pub mod notifications;
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
pub mod zmq_client;
pub mod zmq_communication;
pub mod zmq_server;

// Re-export commonly used types
pub use errors::{
    AgentError, AgentResult, ContextError, ContextResult, ProviderError, ProviderResult,
    StateStoreError, StateStoreResult,
};

pub use traits::{
    ActorType, AgentConfig, AgentHandle, AgentInfo, AgentRunner, AgentSignal, AgentStatus,
    ContextSyncer, Event, ExitStatus, FinishReason, Message, MessageRole, ModelBackend,
    ModelProviderMode, ModelRequest, ModelResponse, StateStore, Task, TaskComplexity, TaskPriority,
    TaskStatus, Tool, ToolCall, ToolParameters,
};

pub use providers::{
    AnthropicProvider, ClaudeCodeAdapter, HeadlessCliAdapter, OllamaProvider, OpenAiProvider,
    ProviderFactory,
};

pub use agent_runner::{GracefulShutdown, LocalProcessRunner, ProcessRunnerConfig};

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
    ListAgentsResponse, OutputQueryRequest, OutputQueryResponse, SpawnRequest, SpawnResponse,
    StatusUpdate, StatusUpdateType, ZmqAgentRunner, ZmqMessage, ZmqOutputStream, ZmqRunnerConfig,
    DEFAULT_TIMEOUT_SECS, MAX_MESSAGE_SIZE, ZMQ_PROTOCOL_VERSION,
};

pub use zmq_communication::{
    ConnectionState, ConnectionStats, SocketType, ZmqConnection, ZmqMessageRouter,
};

pub use zmq_client::ZmqClient;

pub use zmq_server::{ServerStats, ZmqAgentServer, ZmqServerConfig};

pub use notifications::{
    ChannelStats, EventTypeStats, NotificationAdapter, NotificationChannel, NotificationError,
    NotificationEventType, NotificationPayload, NotificationPayloadBuilder, NotificationRouter,
    NotificationSendResult, NotificationStats, RateLimitConfig, RetryConfig, RoutingRule, Severity,
    TemplateContext,
};

pub use notification_router_impl::DefaultNotificationRouter;

pub use ipc::{
    BackpressureConfig, BackpressureController, DeadLetterQueue, IpcMessage, MemoryTransport,
    MessageBus, MessageBusConfig, MessageBusStats, MessageHandler, MessageRouter, MessageTransport,
    MessageType, RequestResponseTracker, UnixSocketTransport,
};

pub use lease::{
    Lease, LeaseAcquisitionRequest, LeaseAcquisitionResponse, LeaseManager, LeaseReleaseRequest,
    LeaseReleaseResponse, LeaseRenewalRequest, LeaseRenewalResponse, LeaseStatus,
};

pub use lease_manager::SqliteLeaseManager;

pub use state_store::{AgentState, Migration, SqliteStateStore, StateTransition};

pub use task_queries::{
    KanbanBoard, SortOrder, TaskQueries, TaskQueryBuilder, TaskSortField, TaskStatistics,
};

pub use config::{
    AgentBehaviorConfig, AnthropicConfig, ConfigManager, DeepSeekConfig, DescaratesConfig,
    FeaturesConfig, GroqConfig, LoggingConfig, NotificationsConfig, OllamaConfig, OpenAiConfig,
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
    StorageStatistics, ThoughtMetadata, ThoughtsConfig, ThoughtsError, ThoughtsResult,
    ThoughtsStorage,
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
