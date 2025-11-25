//! GUI Integration for Time Travel Rewind and Resume
//!
//! This module connects the time travel slider UI with the core rewind/resume logic,
//! providing user feedback, confirmations, and progress tracking.

use descartes_core::{
    AgentHistoryEvent, DefaultRewindManager, ResumeContext, RewindConfig, RewindConfirmation,
    RewindManager, RewindPoint, RewindProgress, RewindResult, ValidationResult,
};
use iced::widget::{button, column, container, row, text, Column, Space};
use iced::{alignment::Horizontal, Element, Length, Theme};
use std::sync::Arc;

// ============================================================================
// MESSAGES
// ============================================================================

/// Messages for rewind/resume operations in the GUI
#[derive(Debug, Clone)]
pub enum RewindMessage {
    /// User selected a point on timeline to rewind to
    SelectRewindPoint(RewindPoint),

    /// User confirmed rewind operation
    ConfirmRewind,

    /// User cancelled rewind operation
    CancelRewind,

    /// Rewind operation started
    RewindStarted,

    /// Rewind progress update
    RewindProgress(RewindProgress),

    /// Rewind completed
    RewindCompleted(RewindResult),

    /// Rewind failed
    RewindFailed(String),

    /// User requested resume from current state
    RequestResume,

    /// Resume operation started
    ResumeStarted,

    /// Resume completed
    ResumeCompleted,

    /// Resume failed
    ResumeFailed(String),

    /// User requested undo of last rewind
    RequestUndo,

    /// Undo completed
    UndoCompleted(RewindResult),

    /// User toggled debugging mode
    ToggleDebugging(bool),

    /// User requested snapshot creation
    CreateSnapshot(String),

    /// Snapshot created
    SnapshotCreated(uuid::Uuid),
}

// ============================================================================
// STATE
// ============================================================================

/// State for rewind/resume operations in the GUI
#[derive(Debug, Clone)]
pub struct RewindState {
    /// Current rewind point being considered
    pub pending_rewind: Option<RewindPoint>,

    /// Confirmation dialog state
    pub confirmation: Option<RewindConfirmation>,

    /// Whether a rewind is in progress
    pub rewind_in_progress: bool,

    /// Current progress
    pub current_progress: Option<RewindProgress>,

    /// Last rewind result
    pub last_result: Option<RewindResult>,

    /// Whether debugging is enabled
    pub debugging_enabled: bool,

    /// Error message if any
    pub error_message: Option<String>,

    /// Success message if any
    pub success_message: Option<String>,
}

impl Default for RewindState {
    fn default() -> Self {
        Self {
            pending_rewind: None,
            confirmation: None,
            rewind_in_progress: false,
            current_progress: None,
            last_result: None,
            debugging_enabled: false,
            error_message: None,
            success_message: None,
        }
    }
}

impl RewindState {
    /// Create a new rewind state
    pub fn new() -> Self {
        Self::default()
    }

    /// Check if rewind is available (not in progress)
    pub fn can_rewind(&self) -> bool {
        !self.rewind_in_progress
    }

    /// Get the last backup ID for undo
    pub fn last_backup_id(&self) -> Option<uuid::Uuid> {
        self.last_result.as_ref().map(|r| r.backup.backup_id)
    }
}

// ============================================================================
// UI COMPONENTS
// ============================================================================

/// Create the rewind confirmation dialog
pub fn view_rewind_confirmation(confirmation: &RewindConfirmation) -> Element<RewindMessage> {
    let mut warnings = Column::new().spacing(5);

    if confirmation.has_uncommitted_changes {
        warnings = warnings.push(
            text("⚠ Uncommitted changes will be stashed")
                .size(12)
                .color(iced::Color::from_rgb(1.0, 0.7, 0.0)),
        );
    }

    if confirmation.events_will_be_lost > 0 {
        warnings = warnings.push(
            text(format!(
                "⚠ Timeline will rewind past {} events",
                confirmation.events_will_be_lost
            ))
            .size(12)
            .color(iced::Color::from_rgb(1.0, 0.7, 0.0)),
        );
    }

    for warning in &confirmation.warnings {
        warnings = warnings.push(
            text(format!("⚠ {}", warning))
                .size(12)
                .color(iced::Color::from_rgb(1.0, 0.7, 0.0)),
        );
    }

    container(
        column![
            text("Confirm Rewind Operation")
                .size(20)
                .color(iced::Color::WHITE),
            Space::with_height(15),
            text(&confirmation.operation).size(14),
            Space::with_height(10),
            text(format!("Target: {}", confirmation.target.description)).size(12),
            Space::with_height(15),
            warnings,
            Space::with_height(20),
            row![
                button(text("Cancel"))
                    .on_press(RewindMessage::CancelRewind)
                    .padding(10),
                Space::with_width(10),
                button(text("Confirm Rewind"))
                    .on_press(RewindMessage::ConfirmRewind)
                    .padding(10)
                    .style(|theme: &Theme, status| {
                        button::Style {
                            background: Some(iced::Background::Color(iced::Color::from_rgb(
                                0.2, 0.6, 1.0,
                            ))),
                            text_color: iced::Color::WHITE,
                            ..button::primary(theme, status)
                        }
                    }),
            ]
            .spacing(10)
            .align_y(iced::alignment::Vertical::Center),
        ]
        .spacing(5)
        .align_x(Horizontal::Center),
    )
    .width(Length::Fixed(500.0))
    .padding(25)
    .style(|theme: &Theme| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(
            0.15, 0.15, 0.2,
        ))),
        border: iced::border::rounded(12),
        ..Default::default()
    })
    .into()
}

/// Create the progress display for rewind operation
pub fn view_rewind_progress(progress: &RewindProgress) -> Element<RewindMessage> {
    let (status_text, progress_text) = match progress {
        RewindProgress::Starting { target } => (
            "Starting rewind...".to_string(),
            format!("Target: {}", target.description),
        ),
        RewindProgress::CreatingBackup => ("Creating backup...".to_string(), "".to_string()),
        RewindProgress::BackupCreated { backup_id } => {
            ("Backup created".to_string(), format!("ID: {}", backup_id))
        }
        RewindProgress::Validating => ("Validating target state...".to_string(), "".to_string()),
        RewindProgress::RestoringBrain {
            events_processed,
            total_events,
        } => (
            "Restoring brain state...".to_string(),
            format!("{} / {} events", events_processed, total_events),
        ),
        RewindProgress::BrainRestored { events_processed } => (
            "Brain state restored".to_string(),
            format!("{} events processed", events_processed),
        ),
        RewindProgress::RestoringBody { commit } => (
            "Restoring body (git checkout)...".to_string(),
            format!("Commit: {}", &commit.chars().take(7).collect::<String>()),
        ),
        RewindProgress::BodyRestored { commit } => (
            "Body state restored".to_string(),
            format!("Commit: {}", &commit.chars().take(7).collect::<String>()),
        ),
        RewindProgress::ValidatingConsistency => {
            ("Validating consistency...".to_string(), "".to_string())
        }
        RewindProgress::Complete { success } => {
            if *success {
                ("Rewind complete!".to_string(), "✓ Success".to_string())
            } else {
                (
                    "Rewind failed".to_string(),
                    "✗ See error details".to_string(),
                )
            }
        }
        RewindProgress::Error { error } => ("Error".to_string(), error.clone()),
    };

    container(
        column![
            text("Rewind in Progress").size(18),
            Space::with_height(10),
            text(status_text).size(14),
            if !progress_text.is_empty() {
                Space::with_height(5)
            } else {
                Space::with_height(0)
            },
            if !progress_text.is_empty() {
                text(progress_text).size(12)
            } else {
                text("")
            },
        ]
        .spacing(5)
        .align_x(Horizontal::Center),
    )
    .width(Length::Fixed(400.0))
    .padding(20)
    .style(|theme: &Theme| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(
            0.15, 0.15, 0.2,
        ))),
        border: iced::border::rounded(8),
        ..Default::default()
    })
    .into()
}

/// Create the rewind result summary
pub fn view_rewind_result(result: &RewindResult) -> Element<RewindMessage> {
    let status_color = if result.success {
        iced::Color::from_rgb(0.2, 0.8, 0.2)
    } else {
        iced::Color::from_rgb(0.8, 0.2, 0.2)
    };

    let status_icon = if result.success { "✓" } else { "✗" };

    let mut details = Column::new().spacing(5);

    if let Some(ref brain_result) = result.brain_result {
        details = details.push(
            text(format!(
                "Brain: {} events processed",
                brain_result.events_processed
            ))
            .size(12),
        );
    }

    if let Some(ref body_result) = result.body_result {
        details = details.push(
            text(format!(
                "Body: commit {}",
                body_result
                    .target_commit
                    .chars()
                    .take(7)
                    .collect::<String>()
            ))
            .size(12),
        );
    }

    details = details.push(text(format!("Duration: {}ms", result.duration_ms)).size(12));

    // Validation info
    if !result.validation.errors.is_empty() {
        details = details.push(Space::with_height(10));
        details = details.push(text("Validation Errors:").size(12).color(status_color));
        for error in &result.validation.errors {
            details = details.push(
                text(format!("  • {}", error))
                    .size(11)
                    .color(iced::Color::from_rgb(0.8, 0.2, 0.2)),
            );
        }
    }

    if !result.validation.warnings.is_empty() {
        details = details.push(Space::with_height(5));
        details = details.push(text("Warnings:").size(12));
        for warning in &result.validation.warnings {
            details = details.push(
                text(format!("  • {}", warning))
                    .size(11)
                    .color(iced::Color::from_rgb(1.0, 0.7, 0.0)),
            );
        }
    }

    // Messages
    if !result.messages.is_empty() {
        details = details.push(Space::with_height(5));
        for message in &result.messages {
            details = details.push(text(message).size(11));
        }
    }

    container(
        column![
            row![
                text(status_icon).size(24).color(status_color),
                Space::with_width(10),
                text(if result.success {
                    "Rewind Successful"
                } else {
                    "Rewind Failed"
                })
                .size(18)
                .color(status_color),
            ]
            .align_y(iced::alignment::Vertical::Center),
            Space::with_height(15),
            details,
            Space::with_height(15),
            button(text("Close"))
                .on_press(RewindMessage::CancelRewind)
                .padding(8),
        ]
        .spacing(5)
        .align_x(Horizontal::Center),
    )
    .width(Length::Fixed(450.0))
    .padding(20)
    .style(|theme: &Theme| container::Style {
        background: Some(iced::Background::Color(iced::Color::from_rgb(
            0.15, 0.15, 0.2,
        ))),
        border: iced::border::rounded(8),
        ..Default::default()
    })
    .into()
}

/// Create the rewind controls panel
pub fn view_rewind_controls(state: &RewindState) -> Element<RewindMessage> {
    let mut controls = Column::new().spacing(10);

    // Resume button (only if we have a successful rewind)
    if state.last_result.as_ref().map_or(false, |r| r.success) {
        controls = controls.push(
            button(text("▶ Resume from Here"))
                .on_press_maybe(if state.can_rewind() {
                    Some(RewindMessage::RequestResume)
                } else {
                    None
                })
                .padding(10),
        );
    }

    // Undo button (only if we have a backup)
    if state.last_backup_id().is_some() {
        controls = controls.push(
            button(text("↶ Undo Rewind"))
                .on_press_maybe(if state.can_rewind() {
                    Some(RewindMessage::RequestUndo)
                } else {
                    None
                })
                .padding(10),
        );
    }

    // Debugging toggle
    controls = controls.push(
        button(text(if state.debugging_enabled {
            "🐛 Debugging: ON"
        } else {
            "🐛 Debugging: OFF"
        }))
        .on_press(RewindMessage::ToggleDebugging(!state.debugging_enabled))
        .padding(8),
    );

    // Create snapshot button
    controls = controls.push(
        button(text("📸 Create Snapshot"))
            .on_press(RewindMessage::CreateSnapshot("Manual snapshot".to_string()))
            .padding(8),
    );

    container(
        column![
            text("Rewind Controls").size(14),
            Space::with_height(10),
            controls,
        ]
        .spacing(5),
    )
    .padding(15)
    .style(|theme: &Theme| container::Style {
        background: Some(theme.palette().background.into()),
        border: iced::border::rounded(8),
        ..Default::default()
    })
    .into()
}

// ============================================================================
// UPDATE LOGIC
// ============================================================================

/// Update rewind state based on messages
pub fn update_rewind(state: &mut RewindState, message: RewindMessage) {
    match message {
        RewindMessage::SelectRewindPoint(point) => {
            if state.can_rewind() {
                state.pending_rewind = Some(point);
                // Confirmation will be shown by the view
            }
        }

        RewindMessage::ConfirmRewind => {
            state.confirmation = None;
            state.rewind_in_progress = true;
            state.error_message = None;
            state.success_message = None;
            // Actual rewind operation would be triggered here via async command
        }

        RewindMessage::CancelRewind => {
            state.pending_rewind = None;
            state.confirmation = None;
            state.current_progress = None;
        }

        RewindMessage::RewindStarted => {
            state.rewind_in_progress = true;
            state.current_progress =
                state
                    .pending_rewind
                    .as_ref()
                    .map(|point| RewindProgress::Starting {
                        target: point.clone(),
                    });
        }

        RewindMessage::RewindProgress(progress) => {
            state.current_progress = Some(progress);
        }

        RewindMessage::RewindCompleted(result) => {
            state.rewind_in_progress = false;
            state.current_progress = Some(RewindProgress::Complete {
                success: result.success,
            });
            state.last_result = Some(result.clone());

            if result.success {
                state.success_message = Some("Rewind completed successfully!".to_string());
            } else {
                state.error_message = Some("Rewind failed. See details below.".to_string());
            }

            state.pending_rewind = None;
        }

        RewindMessage::RewindFailed(error) => {
            state.rewind_in_progress = false;
            state.current_progress = Some(RewindProgress::Error {
                error: error.clone(),
            });
            state.error_message = Some(error);
        }

        RewindMessage::RequestResume => {
            // Resume operation would be triggered here
            state.success_message = Some("Resuming execution...".to_string());
        }

        RewindMessage::ResumeStarted => {
            // Update UI to show resume in progress
        }

        RewindMessage::ResumeCompleted => {
            state.success_message = Some("Resume completed successfully!".to_string());
        }

        RewindMessage::ResumeFailed(error) => {
            state.error_message = Some(format!("Resume failed: {}", error));
        }

        RewindMessage::RequestUndo => {
            // Undo operation would be triggered here
        }

        RewindMessage::UndoCompleted(result) => {
            state.last_result = Some(result);
            state.success_message = Some("Undo completed successfully!".to_string());
        }

        RewindMessage::ToggleDebugging(enabled) => {
            state.debugging_enabled = enabled;
        }

        RewindMessage::CreateSnapshot(description) => {
            // Snapshot creation would be triggered here
            state.success_message = Some(format!("Creating snapshot: {}", description));
        }

        RewindMessage::SnapshotCreated(snapshot_id) => {
            state.success_message = Some(format!("Snapshot created: {}", snapshot_id.to_string()));
        }
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert timeline slider selection to rewind point
pub fn slider_selection_to_rewind_point(
    slider_position: f32,
    events: &[AgentHistoryEvent],
) -> Option<RewindPoint> {
    descartes_core::slider_to_rewind_point(slider_position, events)
}

/// Check if rewind is safe based on current state
pub fn is_rewind_safe(point: &RewindPoint, current_events: &[AgentHistoryEvent]) -> Vec<String> {
    let mut warnings = Vec::new();

    // Check if we're rewinding forward (unusual)
    if let Some(last_event) = current_events.last() {
        if point.timestamp > last_event.timestamp {
            warnings.push("Warning: Rewinding forward in time".to_string());
        }
    }

    // Check if there's a git commit
    if point.git_commit.is_none() {
        warnings.push("Warning: No git commit associated with this point".to_string());
    }

    // Check if rewinding very far back
    if let Some(last_event) = current_events.last() {
        let time_diff = (last_event.timestamp - point.timestamp).abs();
        if time_diff > 86400 {
            // More than 1 day
            warnings.push(format!(
                "Warning: Rewinding {} days back",
                time_diff / 86400
            ));
        }
    }

    warnings
}
