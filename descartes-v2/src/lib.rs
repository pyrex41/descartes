//! Descartes: Visible Subagent Orchestration
//!
//! A tight Rust binary for AI agent orchestration that combines:
//! - **SCUD**: DAG-driven task management with token-efficient SCG format
//! - **Ralph Wiggum**: Deterministic loops with planning/building modes
//! - **Visible subagents**: Full transcript capture for every subagent
//!
//! # Core Philosophy
//!
//! Every subagent execution is fully visible. No black boxes.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────┐
//! │           Ralph Loop (outer)            │
//! │  while :; do descartes run ; done       │
//! └────────────────────┬────────────────────┘
//!                      ▼
//! ┌─────────────────────────────────────────┐
//! │           SCUD Task Graph               │
//! │  $ scud next → returns ready task       │
//! └────────────────────┬────────────────────┘
//!                      ▼
//! ┌─────────────────────────────────────────┐
//! │    Subagents (1 level, visible)         │
//! │  searcher → builder → validator         │
//! │  All transcripts saved in SCG format    │
//! └─────────────────────────────────────────┘
//! ```

pub mod agent;
pub mod config;
pub mod handoff;
pub mod harness;
pub mod interactive;
pub mod ralph_loop;
pub mod scud;
pub mod transcript;
pub mod workflow;

// Re-exports for convenience
pub use agent::{AgentCategory, SubagentResult};
pub use config::Config;
pub use handoff::Handoff;
pub use harness::{Harness, HarnessKind};
pub use interactive::{Session, SessionState, SkillRegistry};
pub use ralph_loop::{LoopConfig, LoopMode};
pub use transcript::{Transcript, TranscriptEntry};
pub use workflow::{WorkflowConfig, WorkflowRunner, WorkflowState};

/// Crate-level error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Harness error: {0}")]
    Harness(String),

    #[error("Subagent error: {0}")]
    Subagent(String),

    #[error("No tasks ready")]
    NoTasksReady,

    #[error("Validation failed: {0}")]
    ValidationFailed(String),

    #[error("SCG parse error: {0}")]
    ScgParse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Notification error: {0}")]
    Notification(String),

    #[error("Command error: {0}")]
    Command(String),

    #[error("Workflow error: {0}")]
    Workflow(String),
}

pub type Result<T> = std::result::Result<T, Error>;
