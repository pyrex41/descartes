// Descartes: Composable AI Agent Orchestration System
// Core library providing traits, providers, and orchestration utilities

pub mod config;
pub mod errors;
pub mod lease;
pub mod lease_manager;
pub mod notifications;
pub mod notification_router_impl;
pub mod providers;
pub mod secrets;
pub mod secrets_crypto;
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

pub use notifications::{
    ChannelStats, NotificationAdapter, NotificationChannel, NotificationError,
    NotificationEventType, NotificationPayload, NotificationPayloadBuilder, NotificationSendResult,
    NotificationRouter, NotificationStats, RateLimitConfig, RoutingRule, RetryConfig, Severity,
    TemplateContext, EventTypeStats,
};

pub use notification_router_impl::DefaultNotificationRouter;

pub use lease::{
    Lease, LeaseStatus, LeaseAcquisitionRequest, LeaseAcquisitionResponse,
    LeaseRenewalRequest, LeaseRenewalResponse, LeaseReleaseRequest, LeaseReleaseResponse,
    LeaseManager,
};

pub use lease_manager::SqliteLeaseManager;

pub use config::{
    ConfigManager, DescaratesConfig, ProvidersConfig, AgentBehaviorConfig, StorageConfig,
    SecurityConfig, NotificationsConfig, FeaturesConfig, LoggingConfig,
    OpenAiConfig, AnthropicConfig, OllamaConfig, DeepSeekConfig, GroqConfig,
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
