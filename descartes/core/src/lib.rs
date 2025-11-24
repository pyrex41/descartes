// Descartes: Composable AI Agent Orchestration System
// Core library providing traits, providers, and orchestration utilities

pub mod errors;
pub mod providers;
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
