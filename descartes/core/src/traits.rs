/// Core trait definitions for the Descartes orchestration system.
use crate::errors::{AgentResult, StateStoreResult};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use uuid::Uuid;

/// Represents a message/context to be sent to a model provider.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelRequest {
    pub messages: Vec<Message>,
    pub model: String,
    pub max_tokens: Option<usize>,
    pub temperature: Option<f32>,
    pub system_prompt: Option<String>,
    pub tools: Option<Vec<Tool>>,
}

/// A single message in a conversation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

/// The role of a message sender.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

/// A tool/function that the model can call.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: ToolParameters,
}

/// Parameters for a tool.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolParameters {
    pub required: Vec<String>,
    pub properties: std::collections::HashMap<String, Value>,
}

/// Response from a model provider.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelResponse {
    pub content: String,
    pub finish_reason: FinishReason,
    pub tokens_used: Option<usize>,
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool call made by the model.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub arguments: Value,
}

/// Why the model stopped responding.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum FinishReason {
    Stop,
    MaxTokens,
    ToolUse,
    Error,
}

/// Configuration for model provider mode.
#[derive(Debug, Clone)]
pub enum ModelProviderMode {
    /// Direct HTTP API calls to providers like OpenAI, Anthropic
    Api { endpoint: String, api_key: String },
    /// Spawn CLI as child process (e.g., claude, opencode)
    Headless { command: String, args: Vec<String> },
    /// Connect to local service like Ollama
    Local { endpoint: String, timeout_secs: u64 },
}

/// The unified trait for all LLM model backends.
/// Supports API, headless CLI, and local modes.
#[async_trait]
pub trait ModelBackend: Send + Sync {
    /// Get the name/identifier of this backend.
    fn name(&self) -> &str;

    /// Get the current mode of operation.
    fn mode(&self) -> &ModelProviderMode;

    /// Initialize the backend with configuration.
    async fn initialize(&mut self) -> AgentResult<()>;

    /// Check if the backend is available and healthy.
    async fn health_check(&self) -> AgentResult<bool>;

    /// Send a request to the model and get a response.
    async fn complete(&self, request: ModelRequest) -> AgentResult<ModelResponse>;

    /// Stream responses from the model (returns JSON lines).
    async fn stream(
        &self,
        request: ModelRequest,
    ) -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<ModelResponse>> + Unpin + Send>>;

    /// Get list of available models.
    async fn list_models(&self) -> AgentResult<Vec<String>>;

    /// Get token count estimate for a given text.
    async fn estimate_tokens(&self, text: &str) -> AgentResult<usize>;

    /// Shutdown the backend gracefully.
    async fn shutdown(&mut self) -> AgentResult<()>;
}

/// Agent Runner trait - spawns and manages agent execution environments.
#[async_trait]
pub trait AgentRunner: Send + Sync {
    /// Spawn an agent with the given configuration.
    async fn spawn(&self, config: AgentConfig) -> AgentResult<Box<dyn AgentHandle>>;

    /// List running agents.
    async fn list_agents(&self) -> AgentResult<Vec<AgentInfo>>;

    /// Get information about a specific agent.
    async fn get_agent(&self, agent_id: &Uuid) -> AgentResult<Option<AgentInfo>>;

    /// Kill an agent.
    async fn kill(&self, agent_id: &Uuid) -> AgentResult<()>;

    /// Send a signal to an agent.
    async fn signal(&self, agent_id: &Uuid, signal: AgentSignal) -> AgentResult<()>;
}

/// Configuration for spawning an agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub model_backend: String, // e.g., "anthropic", "openai", "ollama"
    pub task: String,
    pub context: Option<String>,
    pub system_prompt: Option<String>,
    pub environment: std::collections::HashMap<String, String>,
}

/// Signal to send to an agent.
#[derive(Debug, Clone, Copy)]
pub enum AgentSignal {
    Interrupt, // SIGINT
    Terminate, // SIGTERM
    Kill,      // SIGKILL
}

/// Information about a running agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentInfo {
    pub id: Uuid,
    pub name: String,
    pub status: AgentStatus,
    pub model_backend: String,
    pub started_at: std::time::SystemTime,
    pub task: String,
}

/// Status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AgentStatus {
    Idle,
    Running,
    Paused,
    Completed,
    Failed,
    Terminated,
}

/// Handle to control a running agent.
#[async_trait]
pub trait AgentHandle: Send + Sync {
    /// Get the agent's unique ID.
    fn id(&self) -> Uuid;

    /// Get the agent's current status.
    fn status(&self) -> AgentStatus;

    /// Write to the agent's stdin.
    async fn write_stdin(&mut self, data: &[u8]) -> AgentResult<()>;

    /// Read from the agent's stdout.
    async fn read_stdout(&mut self) -> AgentResult<Option<Vec<u8>>>;

    /// Read from the agent's stderr.
    async fn read_stderr(&mut self) -> AgentResult<Option<Vec<u8>>>;

    /// Wait for the agent to complete.
    async fn wait(&mut self) -> AgentResult<ExitStatus>;

    /// Kill the agent.
    async fn kill(&mut self) -> AgentResult<()>;

    /// Get the agent's exit code (if completed).
    fn exit_code(&self) -> Option<i32>;
}

/// Exit status of an agent.
#[derive(Debug, Clone)]
pub struct ExitStatus {
    pub code: Option<i32>,
    pub success: bool,
}

/// StateStore trait - persistence layer.
#[async_trait]
pub trait StateStore: Send + Sync {
    /// Initialize the store with schema and migrations.
    async fn initialize(&mut self) -> StateStoreResult<()>;

    /// Save an event to the store.
    async fn save_event(&self, event: &Event) -> StateStoreResult<()>;

    /// Get events by session ID.
    async fn get_events(&self, session_id: &str) -> StateStoreResult<Vec<Event>>;

    /// Get events by type.
    async fn get_events_by_type(&self, event_type: &str) -> StateStoreResult<Vec<Event>>;

    /// Save task state.
    async fn save_task(&self, task: &Task) -> StateStoreResult<()>;

    /// Get task by ID.
    async fn get_task(&self, task_id: &Uuid) -> StateStoreResult<Option<Task>>;

    /// Get all tasks.
    async fn get_tasks(&self) -> StateStoreResult<Vec<Task>>;

    /// Search events by full-text query.
    async fn search_events(&self, query: &str) -> StateStoreResult<Vec<Event>>;
}

/// An event in the Descartes system.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub event_type: String,
    pub timestamp: i64,
    pub session_id: String,
    pub actor_type: ActorType,
    pub actor_id: String,
    pub content: String,
    pub metadata: Option<Value>,
    pub git_commit: Option<String>,
}

/// Type of actor in an event.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ActorType {
    User,
    Agent,
    System,
}

/// A task in the global task manager.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub complexity: TaskComplexity,
    pub assigned_to: Option<String>,
    pub dependencies: Vec<Uuid>, // IDs of tasks this task depends on
    pub created_at: i64,
    pub updated_at: i64,
    pub metadata: Option<Value>,
}

/// Status of a task.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
}

/// Priority level of a task.
#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Default for TaskPriority {
    fn default() -> Self {
        TaskPriority::Medium
    }
}

impl std::fmt::Display for TaskPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskPriority::Low => write!(f, "low"),
            TaskPriority::Medium => write!(f, "medium"),
            TaskPriority::High => write!(f, "high"),
            TaskPriority::Critical => write!(f, "critical"),
        }
    }
}

impl std::str::FromStr for TaskPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(TaskPriority::Low),
            "medium" => Ok(TaskPriority::Medium),
            "high" => Ok(TaskPriority::High),
            "critical" => Ok(TaskPriority::Critical),
            _ => Err(format!("Invalid priority: {}", s)),
        }
    }
}

/// Complexity/effort estimate of a task.
#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "snake_case")]
pub enum TaskComplexity {
    Trivial,  // < 1 hour
    Simple,   // 1-4 hours
    Moderate, // 1-2 days
    Complex,  // 3-5 days
    Epic,     // > 1 week
}

impl Default for TaskComplexity {
    fn default() -> Self {
        TaskComplexity::Moderate
    }
}

impl std::fmt::Display for TaskComplexity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskComplexity::Trivial => write!(f, "trivial"),
            TaskComplexity::Simple => write!(f, "simple"),
            TaskComplexity::Moderate => write!(f, "moderate"),
            TaskComplexity::Complex => write!(f, "complex"),
            TaskComplexity::Epic => write!(f, "epic"),
        }
    }
}

impl std::str::FromStr for TaskComplexity {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trivial" => Ok(TaskComplexity::Trivial),
            "simple" => Ok(TaskComplexity::Simple),
            "moderate" => Ok(TaskComplexity::Moderate),
            "complex" => Ok(TaskComplexity::Complex),
            "epic" => Ok(TaskComplexity::Epic),
            _ => Err(format!("Invalid complexity: {}", s)),
        }
    }
}

/// ContextSyncer trait - loads and streams context.
#[async_trait]
pub trait ContextSyncer: Send + Sync {
    /// Load context from various sources (files, git, etc).
    async fn load_context(&self, path: &str) -> AgentResult<String>;

    /// Stream context in chunks.
    async fn stream_context(
        &self,
        path: &str,
    ) -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<String>> + Unpin + Send>>;

    /// Slice context by patterns or relevance.
    async fn slice_context(&self, path: &str, patterns: &[String]) -> AgentResult<String>;

    /// Sync context to a specific location.
    async fn sync_to(&self, source: &str, destination: &str) -> AgentResult<()>;
}
