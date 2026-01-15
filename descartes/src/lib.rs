//! Descartes: Visible Subagent Orchestration
//!
//! A tight Rust binary for AI agent orchestration that combines:
//! - **SCUD**: DAG-driven task management with token-efficient SCG format
//! - **Swarm**: Fresh-context-per-task loops inspired by Ralph Wiggum principles
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
//! │           Swarm Loop (outer)            │
//! │  descartes swarm --scud-tag <tag>       │
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
pub mod context_handoff;
pub mod handoff;
pub mod harness;
pub mod interactive;
pub mod swarm_executor;
pub mod swarm_tui;
pub mod scud;
pub mod spec;
pub mod transcript;

// Re-exports for convenience
pub use agent::{
    AgentCategory, AgentHandle, AgentRegistry, RegistryStatus, SubagentResult, TerminalType,
};
pub use config::Config;
pub use context_handoff::{
    estimate_tokens, summarize_agent_progress, ContextMonitor, HandoffContext,
};
pub use handoff::Handoff;
pub use harness::{Harness, HarnessKind};
pub use interactive::{Session, SessionState, SkillRegistry};
pub use swarm_executor::{SwarmExecutor, TaskResult};
pub use swarm_tui::{SwarmTui, TuiAction, WaveProgress};
pub use spec::{
    apply_spec_template, build_prompt, build_task_spec, extract_plan_section, format_task_spec,
    SpecConfig,
};
pub use transcript::{Transcript, TranscriptEntry};

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
}

pub type Result<T> = std::result::Result<T, Error>;
