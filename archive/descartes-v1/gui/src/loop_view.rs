//! Loop view component for GUI

use crate::loop_state::{LoopMessage, LoopViewState, TaskStatus};
use crate::theme::{colors, fonts};
use iced::alignment::Vertical;
use iced::widget::{button, column, container, progress_bar, row, scrollable, text, Space};
use iced::{Element, Length, Theme};

/// Render the loop view
pub fn view(state: &LoopViewState) -> Element<LoopMessage> {
    let title = text("Iterative Loop")
        .size(24)
        .font(fonts::MONO_BOLD)
        .color(colors::TEXT_PRIMARY);

    let subtitle = text("Ralph-style iterative agent execution")
        .size(14)
        .color(colors::TEXT_SECONDARY);

    // Status indicator
    let status_indicator = if state.active {
        row![
            text("\u{25CF}").size(10).color(colors::PRIMARY), // Filled circle
            Space::with_width(6),
            text(&state.phase).size(12).color(colors::PRIMARY),
        ]
        .align_y(Vertical::Center)
    } else if state.exit_reason.is_some() {
        let (icon, color, label) = match &state.exit_reason {
            Some(descartes_core::IterativeExitReason::CompletionPromiseDetected) => {
                ("\u{2713}", colors::SUCCESS, "Completed successfully")
            }
            Some(descartes_core::IterativeExitReason::MaxIterationsReached) => {
                ("!", colors::WARNING, "Max iterations reached")
            }
            Some(descartes_core::IterativeExitReason::UserCancelled) => {
                ("\u{2717}", colors::TEXT_MUTED, "Cancelled")
            }
            Some(descartes_core::IterativeExitReason::Error { .. }) => {
                ("\u{26A0}", colors::ERROR, "Error")
            }
            _ => ("\u{25CB}", colors::TEXT_MUTED, "Idle"),
        };
        row![
            text(icon).size(10).color(color),
            Space::with_width(6),
            text(label).size(12).color(color),
        ]
        .align_y(Vertical::Center)
    } else {
        row![
            text("\u{25CB}").size(10).color(colors::TEXT_MUTED), // Empty circle
            Space::with_width(6),
            text("Ready").size(12).color(colors::TEXT_MUTED),
        ]
        .align_y(Vertical::Center)
    };

    // Progress section
    let progress_section: Element<LoopMessage> = if state.active || state.max_iterations.is_some() {
        let progress_text = if state.is_scud_loop() {
            format!(
                "Tasks: {}/{} | Wave {}/{}",
                state.scud_tasks_done, state.scud_tasks_total,
                state.current_wave, state.total_waves
            )
        } else if let Some(max) = state.max_iterations {
            format!("Iteration {} of {}", state.current_iteration, max)
        } else {
            format!("Iteration {}", state.current_iteration)
        };

        container(
            column![
                row![
                    text(progress_text.clone())
                        .size(12)
                        .font(fonts::MONO)
                        .color(colors::TEXT_SECONDARY),
                    Space::with_width(Length::Fill),
                    text(format!("{:.0}%", state.progress * 100.0))
                        .size(12)
                        .font(fonts::MONO)
                        .color(colors::TEXT_MUTED),
                ],
                Space::with_height(8),
                progress_bar(0.0..=1.0, state.progress)
                    .height(4)
                    .style(progress_bar_style),
            ]
            .spacing(4),
        )
        .padding(12)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(colors::SURFACE)),
            border: iced::Border {
                color: colors::BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    } else {
        container(text("No active loop").size(12).color(colors::TEXT_MUTED))
            .padding(12)
            .into()
    };

    // SCUD Wave section (only shown for SCUD-aware loops)
    let wave_section: Element<LoopMessage> = if state.is_scud_loop() && !state.tasks_in_wave.is_empty() {
        let tag_row = row![
            text("Tag: ").size(10).color(colors::TEXT_MUTED),
            text(state.scud_tag.as_deref().unwrap_or(""))
                .size(10)
                .font(fonts::MONO)
                .color(colors::PRIMARY),
        ];

        let wave_header = row![
            text(format!("Wave {} Tasks", state.current_wave))
                .size(11)
                .font(fonts::MONO_BOLD)
                .color(colors::TEXT_PRIMARY),
            Space::with_width(Length::Fill),
            text(format!("{} remaining", state.tasks_in_wave.iter().filter(|t| matches!(t.status, TaskStatus::Pending | TaskStatus::InProgress)).count()))
                .size(10)
                .color(colors::TEXT_MUTED),
        ];

        let task_list: Vec<Element<LoopMessage>> = state.tasks_in_wave.iter().map(|task| {
            let (icon, color) = match task.status {
                TaskStatus::Pending => ("\u{25CB}", colors::TEXT_MUTED),    // Empty circle
                TaskStatus::InProgress => ("\u{25CF}", colors::PRIMARY),     // Filled circle
                TaskStatus::Done => ("\u{2713}", colors::SUCCESS),           // Checkmark
                TaskStatus::Blocked => ("\u{26A0}", colors::WARNING),        // Warning
            };

            row![
                text(icon).size(10).color(color),
                Space::with_width(6),
                text(format!("#{}", task.id))
                    .size(10)
                    .font(fonts::MONO)
                    .color(colors::TEXT_MUTED),
                Space::with_width(6),
                text(&task.title)
                    .size(11)
                    .color(if matches!(task.status, TaskStatus::Done) { colors::TEXT_MUTED } else { colors::TEXT_PRIMARY }),
                Space::with_width(Length::Fill),
                text(format!("[{}]", task.complexity))
                    .size(9)
                    .font(fonts::MONO)
                    .color(colors::TEXT_MUTED),
            ]
            .align_y(Vertical::Center)
            .into()
        }).collect();

        let commits_section: Element<LoopMessage> = if !state.wave_commits.is_empty() {
            column![
                Space::with_height(8),
                text("Wave Commits:")
                    .size(10)
                    .color(colors::TEXT_MUTED),
                text(state.wave_commits.iter().map(|c| format!("{}", &c[..7.min(c.len())])).collect::<Vec<_>>().join(" → "))
                    .size(10)
                    .font(fonts::MONO)
                    .color(colors::SUCCESS),
            ]
            .into()
        } else {
            Space::with_height(0).into()
        };

        container(
            column![
                tag_row,
                Space::with_height(8),
                wave_header,
                Space::with_height(6),
            ]
            .push(column(task_list).spacing(4))
            .push(commits_section),
        )
        .padding(12)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(colors::SURFACE)),
            border: iced::Border {
                color: colors::PRIMARY,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    } else {
        Space::with_height(0).into()
    };

    // Command info section
    let command_section: Element<LoopMessage> = if !state.command.is_empty() {
        container(
            column![
                text("Command")
                    .size(10)
                    .color(colors::TEXT_MUTED),
                text(&state.command)
                    .size(12)
                    .font(fonts::MONO)
                    .color(colors::TEXT_PRIMARY),
                Space::with_height(8),
                text("Prompt")
                    .size(10)
                    .color(colors::TEXT_MUTED),
                text(&state.prompt_preview)
                    .size(11)
                    .font(fonts::MONO)
                    .color(colors::TEXT_SECONDARY),
            ]
            .spacing(4),
        )
        .padding(12)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(colors::SURFACE)),
            border: iced::Border {
                color: colors::BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    } else {
        Space::with_height(0).into()
    };

    // Output section
    let output_section: Element<LoopMessage> = if !state.output_lines.is_empty() {
        let output_text: String = state.output_lines.join("\n");
        container(
            column![
                text("Recent Output")
                    .size(10)
                    .color(colors::TEXT_MUTED),
                scrollable(
                    text(output_text.clone())
                        .size(11)
                        .font(fonts::MONO)
                        .color(colors::TEXT_SECONDARY),
                )
                .height(100),
            ]
            .spacing(8),
        )
        .padding(12)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(colors::SURFACE)),
            border: iced::Border {
                color: colors::BORDER,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    } else {
        Space::with_height(0).into()
    };

    // Error section
    let error_section: Element<LoopMessage> = if let Some(ref error) = state.error {
        container(
            row![
                text("\u{26A0}").size(14).color(colors::ERROR),
                Space::with_width(8),
                text(error).size(12).color(colors::ERROR),
            ]
            .align_y(Vertical::Center),
        )
        .padding(12)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(colors::ERROR_DIM)),
            border: iced::Border {
                color: colors::ERROR,
                width: 1.0,
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
    } else {
        Space::with_height(0).into()
    };

    // Action buttons
    let action_buttons = if state.active {
        row![
            button(text("Cancel Loop").size(12))
                .on_press(LoopMessage::CancelLoop)
                .style(|_theme: &Theme, _status| button::Style {
                    background: Some(iced::Background::Color(colors::ERROR_DIM)),
                    text_color: colors::ERROR,
                    border: iced::Border {
                        color: colors::ERROR,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
                .padding([8, 16]),
        ]
    } else {
        row![
            button(text("Refresh").size(12))
                .on_press(LoopMessage::RefreshState)
                .style(|_theme: &Theme, _status| button::Style {
                    background: Some(iced::Background::Color(colors::SURFACE)),
                    text_color: colors::TEXT_SECONDARY,
                    border: iced::Border {
                        color: colors::BORDER,
                        width: 1.0,
                        radius: 4.0.into(),
                    },
                    ..Default::default()
                })
                .padding([8, 16]),
        ]
    };

    // Main layout
    let content = column![
        row![title, Space::with_width(Length::Fill), status_indicator].align_y(Vertical::Center),
        subtitle,
        Space::with_height(16),
        progress_section,
        Space::with_height(12),
        wave_section,
        Space::with_height(12),
        command_section,
        Space::with_height(12),
        output_section,
        error_section,
        Space::with_height(16),
        action_buttons,
    ]
    .spacing(4)
    .padding(20);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|_theme: &Theme| container::Style {
            background: Some(iced::Background::Color(colors::BACKGROUND)),
            ..Default::default()
        })
        .into()
}

/// Custom progress bar style matching the theme
fn progress_bar_style(_: &iced::Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: iced::Background::Color(colors::SURFACE),
        bar: iced::Background::Color(colors::PRIMARY),
        border: iced::Border::default(),
    }
}
