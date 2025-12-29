//! GUI state for iterative loops

use chrono::{DateTime, Utc};
use descartes_core::{IterativeExitReason, IterativeLoopState};

/// Status of an individual task in a wave
#[derive(Debug, Clone, Default)]
pub enum TaskStatus {
    #[default]
    Pending,
    InProgress,
    Done,
    Blocked,
}

/// Progress for a single task in the wave view
#[derive(Debug, Clone, Default)]
pub struct TaskProgress {
    pub id: u32,
    pub title: String,
    pub status: TaskStatus,
    pub complexity: u32,
}

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

    // SCUD integration fields
    /// SCUD tag if running a SCUD-aware loop
    pub scud_tag: Option<String>,

    /// Current wave number (1-indexed)
    pub current_wave: u32,

    /// Total number of waves
    pub total_waves: u32,

    /// Tasks in the current wave
    pub tasks_in_wave: Vec<TaskProgress>,

    /// Commit hashes for completed waves
    pub wave_commits: Vec<String>,

    /// Total SCUD tasks completed
    pub scud_tasks_done: u32,

    /// Total SCUD tasks
    pub scud_tasks_total: u32,
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
            // SCUD fields initialized to defaults - populated by SCUD-aware loops
            scud_tag: None,
            current_wave: 0,
            total_waves: 0,
            tasks_in_wave: Vec::new(),
            wave_commits: Vec::new(),
            scud_tasks_done: 0,
            scud_tasks_total: 0,
        }
    }

    /// Create a SCUD-aware view state
    pub fn from_scud_state(
        base_state: &IterativeLoopState,
        scud_tag: String,
        current_wave: u32,
        total_waves: u32,
        tasks_in_wave: Vec<TaskProgress>,
        wave_commits: Vec<String>,
        tasks_done: u32,
        tasks_total: u32,
    ) -> Self {
        let mut view = Self::from_state(base_state);

        // Override progress with SCUD-based progress
        if tasks_total > 0 {
            view.progress = tasks_done as f32 / tasks_total as f32;
        }

        // Update phase description for SCUD
        view.phase = if view.active {
            format!("Wave {}/{} - {} tasks", current_wave, total_waves, tasks_in_wave.len())
        } else {
            format!("Completed: {}/{} tasks", tasks_done, tasks_total)
        };

        // Set SCUD fields
        view.scud_tag = Some(scud_tag);
        view.current_wave = current_wave;
        view.total_waves = total_waves;
        view.tasks_in_wave = tasks_in_wave;
        view.wave_commits = wave_commits;
        view.scud_tasks_done = tasks_done;
        view.scud_tasks_total = tasks_total;

        view
    }

    /// Check if this is a SCUD-aware loop
    pub fn is_scud_loop(&self) -> bool {
        self.scud_tag.is_some()
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
