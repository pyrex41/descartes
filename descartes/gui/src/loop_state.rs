//! GUI state for iterative loops

use chrono::{DateTime, Utc};
use descartes_core::{IterativeExitReason, IterativeLoopState};

/// GUI-friendly loop status
#[derive(Debug, Clone, Default)]
pub struct LoopViewState {
    /// Whether a loop is active
    pub active: bool,

    /// Current iteration (1-indexed for display)
    pub current_iteration: u32,

    /// Maximum iterations (if set)
    pub max_iterations: Option<u32>,

    /// Progress percentage (0.0 - 1.0)
    pub progress: f32,

    /// Current phase description
    pub phase: String,

    /// Command being run
    pub command: String,

    /// Prompt (truncated)
    pub prompt_preview: String,

    /// Recent output lines
    pub output_lines: Vec<String>,

    /// Error message if any
    pub error: Option<String>,

    /// When the loop started
    pub started_at: Option<DateTime<Utc>>,

    /// Exit reason if completed
    pub exit_reason: Option<IterativeExitReason>,
}

impl LoopViewState {
    pub fn from_state(state: &IterativeLoopState) -> Self {
        let progress = if let Some(max) = state.config.max_iterations {
            if max > 0 {
                state.iteration as f32 / max as f32
            } else {
                0.0
            }
        } else {
            0.0
        };

        Self {
            active: !state.completed,
            current_iteration: state.iteration + 1, // 1-indexed for display
            max_iterations: state.config.max_iterations,
            progress,
            phase: if state.completed {
                "Completed".to_string()
            } else {
                format!("Running iteration {}", state.iteration + 1)
            },
            command: state.config.command.clone(),
            prompt_preview: state.config.prompt.chars().take(100).collect(),
            output_lines: state
                .iteration_summaries
                .last()
                .map(|s| s.output_preview.lines().map(String::from).collect())
                .unwrap_or_default(),
            error: state.error.clone(),
            started_at: Some(state.started_at),
            exit_reason: state.exit_reason.clone(),
        }
    }
}

/// Messages for loop view
#[derive(Debug, Clone)]
pub enum LoopMessage {
    /// Start a new loop
    StartLoop {
        command: String,
        prompt: String,
        completion_promise: String,
        max_iterations: u32,
    },
    /// Cancel the running loop
    CancelLoop,
    /// Loop state updated
    StateUpdated(Box<IterativeLoopState>),
    /// Output line received
    OutputReceived(String),
    /// Loop completed
    LoopCompleted(IterativeExitReason),
    /// Error occurred
    Error(String),
    /// Clear error
    ClearError,
    /// Refresh state from file
    RefreshState,
}
