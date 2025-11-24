// Descartes: Composable AI Agent Orchestration System
// Core library providing traits, providers, and orchestration utilities

pub mod agent_runner;
pub mod config;
pub mod config_loader;
pub mod config_migration;
pub mod config_watcher;
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
pub mod thoughts;
pub mod traits;

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
