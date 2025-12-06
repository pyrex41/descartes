/// Core trait definitions for the Descartes orchestration system.
use crate::errors::{AgentResult, StateStoreResult};
use async_trait::async_trait;
use serde_json::Value;
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

    /// Pause a running agent.
    ///
    /// If `force` is false, sends a cooperative pause notification via stdin.
    /// If `force` is true, uses SIGSTOP (Unix) to immediately freeze the process.
    async fn pause(&self, agent_id: &Uuid, force: bool) -> AgentResult<()>;

    /// Resume a paused agent.
    ///
    /// If the agent was force-paused, sends SIGCONT to unfreeze it.
    /// Sends a resume notification via stdin.
    async fn resume(&self, agent_id: &Uuid) -> AgentResult<()>;
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentSignal {
    Interrupt,  // SIGINT - cooperative pause request
    Terminate,  // SIGTERM - graceful shutdown
    Kill,       // SIGKILL - force kill
    ForcePause, // SIGSTOP - emergency freeze (Unix only)
    Resume,     // SIGCONT - resume from forced pause
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
    /// When the agent was paused (if currently paused)
    #[serde(default)]
    pub paused_at: Option<std::time::SystemTime>,
    /// How the agent was paused (cooperative or forced)
    #[serde(default)]
    pub pause_mode: Option<PauseMode>,
    /// Attachment info for connecting external TUIs
    #[serde(default)]
    pub attach_info: Option<AttachInfo>,
}

/// How an agent was paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PauseMode {
    /// Paused via stdin notification, agent acknowledged
    Cooperative,
    /// Paused via SIGSTOP, process frozen immediately
    Forced,
}

impl std::fmt::Display for PauseMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PauseMode::Cooperative => write!(f, "cooperative"),
            PauseMode::Forced => write!(f, "forced"),
        }
    }
}

/// Connection information for attaching an external TUI to a paused agent.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttachInfo {
    /// ZMQ endpoint URL for connecting (e.g., "ipc:///tmp/descartes-agent-xxx.sock")
    pub connect_url: String,
    /// Authentication token for the attach session
    pub token: String,
    /// Unix timestamp when the token expires
    pub expires_at: i64,
}

/// Status of an agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    /// Agent has been created but not yet started
    Idle,
    /// Agent is initializing (loading context, setting up environment)
    Initializing,
    /// Agent is actively executing tasks
    Running,
    /// Agent is actively thinking/reasoning (visible to monitoring UI)
    Thinking,
    /// Agent has been paused and can be resumed
    Paused,
    /// Agent has completed its task successfully
    Completed,
    /// Agent encountered an error and stopped
    Failed,
    /// Agent was externally terminated (killed)
    Terminated,
}

impl std::fmt::Display for AgentStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentStatus::Idle => write!(f, "idle"),
            AgentStatus::Initializing => write!(f, "initializing"),
            AgentStatus::Running => write!(f, "running"),
            AgentStatus::Thinking => write!(f, "thinking"),
            AgentStatus::Paused => write!(f, "paused"),
            AgentStatus::Completed => write!(f, "completed"),
            AgentStatus::Failed => write!(f, "failed"),
            AgentStatus::Terminated => write!(f, "terminated"),
        }
    }
}

impl AgentStatus {
    /// Return true when the agent has reached a terminal lifecycle state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            AgentStatus::Completed | AgentStatus::Failed | AgentStatus::Terminated
        )
    }

    /// Return true while the agent is still eligible to run work.
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            AgentStatus::Idle
                | AgentStatus::Initializing
                | AgentStatus::Running
                | AgentStatus::Thinking
                | AgentStatus::Paused
        )
    }
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

// ============================================================================
// SCUD Integration Types
// Re-export SCUD types for gradual migration to unified task model
// ============================================================================

/// Re-export SCUD task types with Scud prefix to avoid conflicts during migration
pub use scud::models::{Phase as ScudPhase, Priority as ScudPriority, Task as ScudTask, TaskStatus as ScudTaskStatus};

/// Re-export SCUD storage for SCG file operations
pub use scud::storage::Storage as ScudStorage;

/// Re-export SCUD format utilities
pub use scud::formats::{parse_scg, serialize_scg};

/// Convert Descartes TaskStatus to SCUD TaskStatus
impl From<TaskStatus> for ScudTaskStatus {
    fn from(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Todo => ScudTaskStatus::Pending,
            TaskStatus::InProgress => ScudTaskStatus::InProgress,
            TaskStatus::Done => ScudTaskStatus::Done,
            TaskStatus::Blocked => ScudTaskStatus::Blocked,
        }
    }
}

/// Convert SCUD TaskStatus to Descartes TaskStatus
impl From<ScudTaskStatus> for TaskStatus {
    fn from(status: ScudTaskStatus) -> Self {
        match status {
            ScudTaskStatus::Pending => TaskStatus::Todo,
            ScudTaskStatus::InProgress => TaskStatus::InProgress,
            ScudTaskStatus::Done => TaskStatus::Done,
            ScudTaskStatus::Blocked => TaskStatus::Blocked,
            ScudTaskStatus::Expanded => TaskStatus::Done, // Expanded tasks are parent containers
            ScudTaskStatus::Review => TaskStatus::InProgress, // Map review to in-progress
            ScudTaskStatus::Deferred => TaskStatus::Blocked, // Map deferred to blocked
            ScudTaskStatus::Cancelled => TaskStatus::Done, // Map cancelled to done
        }
    }
}

/// Convert Descartes TaskPriority to SCUD Priority
impl From<TaskPriority> for ScudPriority {
    fn from(priority: TaskPriority) -> Self {
        match priority {
            TaskPriority::Low => ScudPriority::Low,
            TaskPriority::Medium => ScudPriority::Medium,
            TaskPriority::High => ScudPriority::High,
            TaskPriority::Critical => ScudPriority::Critical,
        }
    }
}

/// Convert SCUD Priority to Descartes TaskPriority
impl From<ScudPriority> for TaskPriority {
    fn from(priority: ScudPriority) -> Self {
        match priority {
            ScudPriority::Low => TaskPriority::Low,
            ScudPriority::Medium => TaskPriority::Medium,
            ScudPriority::High => TaskPriority::High,
            ScudPriority::Critical => TaskPriority::Critical,
        }
    }
}

/// Convert Descartes TaskComplexity to SCUD complexity (Fibonacci u32)
impl From<TaskComplexity> for u32 {
    fn from(complexity: TaskComplexity) -> Self {
        match complexity {
            TaskComplexity::Trivial => 1,   // < 1 hour
            TaskComplexity::Simple => 2,    // 1-4 hours
            TaskComplexity::Moderate => 5,  // 1-2 days
            TaskComplexity::Complex => 13,  // 3-5 days
            TaskComplexity::Epic => 21,     // > 1 week
        }
    }
}

/// Convert SCUD complexity (Fibonacci u32) to Descartes TaskComplexity
impl From<u32> for TaskComplexity {
    fn from(complexity: u32) -> Self {
        match complexity {
            0 | 1 => TaskComplexity::Trivial,
            2 | 3 => TaskComplexity::Simple,
            5 | 8 => TaskComplexity::Moderate,
            13 => TaskComplexity::Complex,
            _ => TaskComplexity::Epic, // 21, 34, 55, 89
        }
    }
}

/// Convert a Descartes Task to a SCUD Task
pub fn task_to_scud(task: &Task) -> ScudTask {
    let now = chrono::Utc::now().to_rfc3339();
    ScudTask {
        id: task.id.to_string(),
        title: task.title.clone(),
        description: task.description.clone().unwrap_or_default(),
        status: task.status.clone().into(),
        priority: task.priority.into(),
        complexity: task.complexity.into(),
        dependencies: task.dependencies.iter().map(|id| id.to_string()).collect(),
        assigned_to: task.assigned_to.clone(),
        parent_id: None, // Descartes doesn't have parent task concept yet
        subtasks: Vec::new(),
        details: None,
        test_strategy: None,
        created_at: Some(chrono::DateTime::from_timestamp(task.created_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| now.clone())),
        updated_at: Some(chrono::DateTime::from_timestamp(task.updated_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or(now)),
    }
}

/// Convert a SCUD Task to a Descartes Task
/// Note: Some SCUD-specific fields (parent_id, subtasks, locked_by, details, test_strategy) are not preserved
pub fn scud_to_task(scud_task: &ScudTask) -> Result<Task, uuid::Error> {
    let id = Uuid::parse_str(&scud_task.id)?;
    let dependencies: Result<Vec<Uuid>, _> = scud_task
        .dependencies
        .iter()
        .map(|s| Uuid::parse_str(s))
        .collect();

    let now = chrono::Utc::now().timestamp();

    // Parse RFC3339 timestamps from SCUD, fallback to current time
    let created_at = scud_task.created_at.as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(now);

    let updated_at = scud_task.updated_at.as_ref()
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|dt| dt.timestamp())
        .unwrap_or(now);

    Ok(Task {
        id,
        title: scud_task.title.clone(),
        description: if scud_task.description.is_empty() {
            None
        } else {
            Some(scud_task.description.clone())
        },
        status: scud_task.status.clone().into(),
        priority: scud_task.priority.clone().into(),
        complexity: scud_task.complexity.into(),
        assigned_to: scud_task.assigned_to.clone(),
        dependencies: dependencies?,
        created_at,
        updated_at,
        metadata: None,
    })
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
