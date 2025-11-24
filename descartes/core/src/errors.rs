/// Error types for the Descartes orchestration system.
use thiserror::Error;

/// Core error type for provider operations.
#[derive(Error, Debug)]
pub enum ProviderError {
    #[error("Model backend error: {0}")]
    BackendError(String),

    #[error("Provider initialization failed: {0}")]
    InitializationError(String),

    #[error("API request failed: {0}")]
    ApiError(String),

    #[error("Authentication failed: {0}")]
    AuthenticationError(String),

    #[error("Process spawning failed: {0}")]
    ProcessError(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("Reqwest error: {0}")]
    ReqwestError(#[from] reqwest::Error),

    #[error("Invalid configuration: {0}")]
    ConfigError(String),

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Provider not available: {0}")]
    ProviderUnavailable(String),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    #[error("Stream closed unexpectedly")]
    StreamClosed,

    #[error("Timeout waiting for response")]
    Timeout,

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),
}

/// Result type for provider operations.
pub type ProviderResult<T> = Result<T, ProviderError>;

/// Core error type for agent runner operations.
#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Agent spawn failed: {0}")]
    SpawnFailed(String),

    #[error("Agent execution error: {0}")]
    ExecutionError(String),

    #[error("Agent communication error: {0}")]
    CommunicationError(String),

    #[error("Agent not found: {0}")]
    NotFound(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Invalid context: {0}")]
    InvalidContext(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Provider error: {0}")]
    ProviderError(#[from] ProviderError),
}

/// Result type for agent operations.
pub type AgentResult<T> = Result<T, AgentError>;

/// Core error type for state store operations.
#[derive(Error, Debug)]
pub enum StateStoreError {
    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Migration error: {0}")]
    MigrationError(String),

    #[error("Record not found: {0}")]
    NotFound(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Result type for state store operations.
pub type StateStoreResult<T> = Result<T, StateStoreError>;

/// Core error type for context operations.
#[derive(Error, Debug)]
pub enum ContextError {
    #[error("Context loading failed: {0}")]
    LoadFailed(String),

    #[error("Context slicing failed: {0}")]
    SliceFailed(String),

    #[error("Invalid context path: {0}")]
    InvalidPath(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Git error: {0}")]
    GitError(String),
}

/// Result type for context operations.
pub type ContextResult<T> = Result<T, ContextError>;
