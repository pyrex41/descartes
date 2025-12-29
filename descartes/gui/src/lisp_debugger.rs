//! Lisp Debugger Panel for Swank/SBCL Integration
//!
//! This module provides a UI panel that displays when DebuggerPaused events occur
//! from a Lisp agent using Swank. It shows:
//! - Error condition and message
//! - Available restarts with descriptions
//! - Buttons to invoke each restart

use iced::widget::{button, column, container, row, scrollable, text, Space};
use iced::{border, Color, Element, Length, Theme};
use serde::{Deserialize, Serialize};

// Reference theme from the crate root (main.rs)
use crate::theme::{button_styles, colors, container_styles};

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Represents a restart option from the Lisp debugger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LispRestart {
    /// Index of the restart (used when invoking)
    pub index: usize,
    /// Name of the restart (e.g., "ABORT", "CONTINUE", "USE-VALUE")
    pub name: String,
    /// Description of what the restart does
    pub description: String,
}

/// Represents a stack frame from the Lisp debugger
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LispFrame {
    /// Frame index
    pub index: usize,
    /// Frame description
    pub description: String,
}

/// State for the Lisp debugger panel
#[derive(Debug, Clone, Default)]
pub struct LispDebuggerState {
    /// Whether the debugger panel is visible
    pub visible: bool,
    /// Agent ID that triggered the debugger
    pub agent_id: Option<String>,
    /// Thread ID in the Lisp runtime
    pub thread: i64,
    /// Debug level (nested debugger depth)
    pub level: i64,
    /// Error condition/message
    pub condition: String,
    /// Available restarts
    pub restarts: Vec<LispRestart>,
    /// Stack frames
    pub frames: Vec<LispFrame>,
    /// Whether a restart is being invoked
    pub invoking_restart: bool,
    /// Last error from restart invocation
    pub error: Option<String>,
}

impl LispDebuggerState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Show the debugger with the given error info
    pub fn show(&mut self, agent_id: String, thread: i64, level: i64, condition: String, restarts: Vec<LispRestart>, frames: Vec<LispFrame>) {
        self.visible = true;
        self.agent_id = Some(agent_id);
        self.thread = thread;
        self.level = level;
        self.condition = condition;
        self.restarts = restarts;
        self.frames = frames;
        self.invoking_restart = false;
        self.error = None;
    }

    /// Hide the debugger panel
    pub fn hide(&mut self) {
        self.visible = false;
        self.invoking_restart = false;
    }

    /// Mark that a restart is being invoked
    pub fn set_invoking(&mut self) {
        self.invoking_restart = true;
        self.error = None;
    }

    /// Set an error message
    pub fn set_error(&mut self, error: String) {
        self.error = Some(error);
        self.invoking_restart = false;
    }

    /// Check if the debugger is currently active/visible
    pub fn is_active(&self) -> bool {
        self.visible
    }
}

// ============================================================================
// MESSAGES
// ============================================================================

/// Messages for the Lisp debugger panel
#[derive(Debug, Clone)]
pub enum LispDebuggerMessage {
    /// Debugger was activated with error info
    DebuggerActivated {
        agent_id: String,
        thread: i64,
        level: i64,
        condition: String,
        restarts: Vec<LispRestart>,
        frames: Vec<LispFrame>,
    },
    /// User selected a restart to invoke
    InvokeRestart(usize),
    /// Restart invocation completed
    RestartComplete,
    /// Error during restart invocation
    RestartError(String),
    /// Dismiss the debugger panel
    Dismiss,
    /// Toggle frame expansion (future feature)
    ToggleFrames,
}

// ============================================================================
// UPDATE
// ============================================================================

/// Update the Lisp debugger state based on a message
/// Returns Some(restart_index) if a restart should be invoked
pub fn update(state: &mut LispDebuggerState, message: LispDebuggerMessage) -> Option<(String, usize)> {
    match message {
        LispDebuggerMessage::DebuggerActivated {
            agent_id,
            thread,
            level,
            condition,
            restarts,
            frames,
        } => {
            state.show(agent_id, thread, level, condition, restarts, frames);
            None
        }
        LispDebuggerMessage::InvokeRestart(index) => {
            if let Some(agent_id) = state.agent_id.clone() {
                state.set_invoking();
                Some((agent_id, index))
            } else {
                None
            }
        }
        LispDebuggerMessage::RestartComplete => {
            state.hide();
            None
        }
        LispDebuggerMessage::RestartError(error) => {
            state.set_error(error);
            None
        }
        LispDebuggerMessage::Dismiss => {
            state.hide();
            None
        }
        LispDebuggerMessage::ToggleFrames => {
            // Future feature: expand/collapse frames
            None
        }
    }
}

// ============================================================================
// VIEW
// ============================================================================

/// View the Lisp debugger panel
pub fn view(state: &LispDebuggerState) -> Element<LispDebuggerMessage> {
    if !state.visible {
        return Space::new(0, 0).into();
    }

    // Header with error icon and level
    let header = container(
        row![
            text("⚠").size(24).color(colors::ERROR),
            Space::with_width(12),
            column![
                text("Lisp Debugger").size(18).color(colors::TEXT_PRIMARY),
                text(format!("Level {} | Thread {}", state.level, state.thread))
                    .size(12)
                    .color(colors::TEXT_MUTED),
            ],
            Space::with_width(Length::Fill),
            button(text("✕").size(16))
                .on_press(LispDebuggerMessage::Dismiss)
                .padding(8)
                .style(button_styles::secondary),
        ]
        .align_y(iced::alignment::Vertical::Center)
    )
    .padding(16)
    .width(Length::Fill)
    .style(container_styles::header);

    // Error condition
    let condition_section = container(
        column![
            text("Condition").size(12).color(colors::TEXT_MUTED),
            Space::with_height(8),
            container(
                scrollable(
                    text(&state.condition)
                        .size(14)
                        .color(colors::ERROR)
                )
                .height(80)
            )
            .padding(12)
            .width(Length::Fill)
            .style(|_theme: &Theme| container::Style {
                background: Some(Color::from_rgb8(40, 20, 20).into()),
                border: border::rounded(6),
                ..Default::default()
            }),
        ]
    )
    .padding(16)
    .width(Length::Fill);

    // Restarts section
    let restarts_section = {
        let restart_buttons: Vec<Element<LispDebuggerMessage>> = state
            .restarts
            .iter()
            .map(|restart| {
                let is_abort = restart.name.to_uppercase() == "ABORT";
                let btn_style = if is_abort {
                    button_styles::primary
                } else {
                    button_styles::secondary
                };

                container(
                    row![
                        button(
                            column![
                                text(&restart.name)
                                    .size(14)
                                    .color(if is_abort { colors::TEXT_PRIMARY } else { colors::TEXT_SECONDARY }),
                                text(&restart.description)
                                    .size(11)
                                    .color(colors::TEXT_MUTED),
                            ]
                            .spacing(2)
                        )
                        .on_press_maybe(
                            if state.invoking_restart {
                                None
                            } else {
                                Some(LispDebuggerMessage::InvokeRestart(restart.index))
                            }
                        )
                        .width(Length::Fill)
                        .padding([10, 16])
                        .style(btn_style),
                    ]
                )
                .width(Length::Fill)
                .into()
            })
            .collect();

        container(
            column![
                text("Available Restarts").size(12).color(colors::TEXT_MUTED),
                Space::with_height(8),
                scrollable(
                    column(restart_buttons)
                        .spacing(6)
                        .width(Length::Fill)
                )
                .height(Length::FillPortion(1)),
            ]
        )
        .padding(16)
        .width(Length::Fill)
    };

    // Stack frames section (collapsed summary)
    let frames_section = if !state.frames.is_empty() {
        container(
            column![
                text(format!("Stack Trace ({} frames)", state.frames.len()))
                    .size(12)
                    .color(colors::TEXT_MUTED),
                Space::with_height(8),
                container(
                    scrollable(
                        column(
                            state.frames.iter().take(5).map(|frame| {
                                text(format!("{}: {}", frame.index,
                                    if frame.description.len() > 60 {
                                        format!("{}...", &frame.description[..60])
                                    } else {
                                        frame.description.clone()
                                    }
                                ))
                                .size(11)
                                .color(colors::TEXT_SECONDARY)
                                .into()
                            }).collect::<Vec<Element<LispDebuggerMessage>>>()
                        )
                        .spacing(4)
                    )
                    .height(100)
                )
                .padding(12)
                .width(Length::Fill)
                .style(container_styles::card),
            ]
        )
        .padding(16)
        .width(Length::Fill)
    } else {
        container(Space::new(0, 0))
    };

    // Error message if any
    let error_section = if let Some(ref error) = state.error {
        container(
            row![
                text("⚠").size(14).color(colors::WARNING),
                Space::with_width(8),
                text(error).size(12).color(colors::WARNING),
            ]
        )
        .padding(12)
        .width(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(Color::from_rgb8(60, 40, 20).into()),
            border: border::rounded(6),
            ..Default::default()
        })
    } else {
        container(Space::new(0, 0))
    };

    // Loading indicator
    let loading_section = if state.invoking_restart {
        container(
            row![
                text("⏳").size(16),
                Space::with_width(8),
                text("Invoking restart...").size(14).color(colors::TEXT_SECONDARY),
            ]
            .align_y(iced::alignment::Vertical::Center)
        )
        .padding(12)
        .width(Length::Fill)
    } else {
        container(Space::new(0, 0))
    };

    // Combine all sections
    let content = column![
        header,
        condition_section,
        restarts_section,
        frames_section,
        error_section,
        loading_section,
    ]
    .spacing(0);

    // Modal container with backdrop
    container(
        container(content)
            .width(500)
            .style(container_styles::panel)
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center(Length::Fill)
    .style(|_theme: &Theme| container::Style {
        background: Some(Color::from_rgba8(0, 0, 0, 0.7).into()),
        ..Default::default()
    })
    .into()
}

/// Parse DebuggerPaused event data into LispDebuggerMessage
pub fn parse_debugger_event(agent_id: String, data: &serde_json::Value) -> Option<LispDebuggerMessage> {
    let thread = data.get("thread").and_then(|v| v.as_i64()).unwrap_or(0);
    let level = data.get("level").and_then(|v| v.as_i64()).unwrap_or(1);
    let condition = data.get("condition")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown error")
        .to_string();

    let restarts = data.get("restarts")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().enumerate().filter_map(|(i, r)| {
                Some(LispRestart {
                    index: r.get("index").and_then(|v| v.as_u64()).unwrap_or(i as u64) as usize,
                    name: r.get("name").and_then(|v| v.as_str()).unwrap_or("UNKNOWN").to_string(),
                    description: r.get("description").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                })
            }).collect()
        })
        .unwrap_or_else(|| vec![
            LispRestart {
                index: 0,
                name: "ABORT".to_string(),
                description: "Return to top level".to_string(),
            }
        ]);

    let frames = data.get("frames")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter().enumerate().filter_map(|(i, f)| {
                Some(LispFrame {
                    index: f.get("index").and_then(|v| v.as_u64()).unwrap_or(i as u64) as usize,
                    description: f.get("description").and_then(|v| v.as_str()).unwrap_or("???").to_string(),
                })
            }).collect()
        })
        .unwrap_or_default();

    Some(LispDebuggerMessage::DebuggerActivated {
        agent_id,
        thread,
        level,
        condition,
        restarts,
        frames,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_lisp_debugger_state_show_hide() {
        let mut state = LispDebuggerState::new();
        assert!(!state.visible);

        state.show(
            "agent-1".to_string(),
            1,
            1,
            "Division by zero".to_string(),
            vec![LispRestart {
                index: 0,
                name: "ABORT".to_string(),
                description: "Return to top level".to_string(),
            }],
            vec![],
        );

        assert!(state.visible);
        assert_eq!(state.agent_id, Some("agent-1".to_string()));
        assert_eq!(state.condition, "Division by zero");

        state.hide();
        assert!(!state.visible);
    }

    #[test]
    fn test_parse_debugger_event() {
        let data = json!({
            "thread": 1,
            "level": 1,
            "condition": "The variable X is unbound.",
            "restarts": [
                {"index": 0, "name": "ABORT", "description": "Return to top level"},
                {"index": 1, "name": "USE-VALUE", "description": "Use a value for X"}
            ],
            "frames": [
                {"index": 0, "description": "(EVAL X)"},
                {"index": 1, "description": "(SWANK:EVAL-AND-GRAB-OUTPUT \"X\")"}
            ]
        });

        let msg = parse_debugger_event("agent-1".to_string(), &data);
        assert!(msg.is_some());

        if let Some(LispDebuggerMessage::DebuggerActivated { condition, restarts, frames, .. }) = msg {
            assert_eq!(condition, "The variable X is unbound.");
            assert_eq!(restarts.len(), 2);
            assert_eq!(restarts[0].name, "ABORT");
            assert_eq!(restarts[1].name, "USE-VALUE");
            assert_eq!(frames.len(), 2);
        } else {
            panic!("Expected DebuggerActivated message");
        }
    }

    #[test]
    fn test_update_invoke_restart() {
        let mut state = LispDebuggerState::new();
        state.show(
            "agent-1".to_string(),
            1,
            1,
            "Error".to_string(),
            vec![LispRestart {
                index: 0,
                name: "ABORT".to_string(),
                description: "Return to top level".to_string(),
            }],
            vec![],
        );

        let result = update(&mut state, LispDebuggerMessage::InvokeRestart(0));
        assert!(result.is_some());
        assert!(state.invoking_restart);

        let (agent_id, restart_index) = result.unwrap();
        assert_eq!(agent_id, "agent-1");
        assert_eq!(restart_index, 0);
    }

    #[test]
    fn test_update_restart_complete() {
        let mut state = LispDebuggerState::new();
        state.show(
            "agent-1".to_string(),
            1,
            1,
            "Error".to_string(),
            vec![],
            vec![],
        );
        state.invoking_restart = true;

        update(&mut state, LispDebuggerMessage::RestartComplete);
        assert!(!state.visible);
        assert!(!state.invoking_restart);
    }
}
