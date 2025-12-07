//! Session selector and sessions panel components
//!
//! This module provides UI components for session management:
//! - Session selector dropdown in the header
//! - Full sessions panel/view for managing workspaces

use crate::session_state::{SessionMessage, SessionState};
use crate::theme::{button_styles, colors, container_styles, fonts};
use descartes_core::{Session, SessionStatus};
use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::widget::text_input::Id as TextInputId;
use iced::{Element, Length};

/// Render the session selector dropdown in the header
pub fn view_session_selector(state: &SessionState) -> Element<SessionMessage> {
    let active_session = state.active_session();

    let session_name = active_session
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "No Session".to_string());

    let (status_indicator, status_color) = active_session
        .map(|s| match s.status {
            SessionStatus::Active => ("\u{25CF}", colors::SUCCESS), // ●
            SessionStatus::Starting => ("\u{25D0}", colors::WARNING), // ◐
            SessionStatus::Stopping => ("\u{25D0}", colors::WARNING), // ◐
            SessionStatus::Inactive => ("\u{25CB}", colors::TEXT_MUTED), // ○
            SessionStatus::Error => ("\u{25CF}", colors::ERROR), // ●
            SessionStatus::Archived => ("\u{25CB}", colors::TEXT_MUTED), // ○
        })
        .unwrap_or(("\u{25CB}", colors::TEXT_MUTED));

    let selector_content = row![
        text(status_indicator).size(10).color(status_color),
        Space::with_width(6),
        text(session_name)
            .size(13)
            .font(fonts::MONO_MEDIUM)
            .color(colors::TEXT_PRIMARY),
        Space::with_width(8),
        text("\u{25BC}").size(8).color(colors::TEXT_MUTED), // ▼
    ]
    .align_y(Vertical::Center);

    let selector = button(selector_content)
        .on_press(SessionMessage::RefreshSessions)
        .padding([6, 12])
        .style(button_styles::secondary);

    container(selector).into()
}

/// Render the full sessions panel/view
pub fn view_sessions_panel(state: &SessionState) -> Element<SessionMessage> {
    // Header with title and actions
    let header = row![
        text("Sessions")
            .size(24)
            .font(fonts::MONO_BOLD)
            .color(colors::TEXT_PRIMARY),
        Space::with_width(Length::Fill),
        button(
            text("+ New")
                .size(13)
                .font(fonts::MONO_MEDIUM)
                .color(colors::PRIMARY)
        )
        .on_press(SessionMessage::ShowCreateDialog)
        .padding([8, 16])
        .style(button_styles::primary),
        Space::with_width(8),
        button(
            text("Refresh")
                .size(13)
                .font(fonts::MONO)
                .color(colors::TEXT_PRIMARY)
        )
        .on_press(SessionMessage::RefreshSessions)
        .padding([8, 16])
        .style(button_styles::secondary),
    ]
    .align_y(Vertical::Center);

    // Search filter
    let search_input = text_input("Search sessions...", &state.filter.search)
        .on_input(SessionMessage::UpdateSearch)
        .padding([8, 12])
        .size(13);

    let filter_row = row![
        container(search_input).width(Length::FillPortion(2)),
        Space::with_width(12),
        button(
            text(if state.filter.include_archived {
                "[x] Show Archived"
            } else {
                "[ ] Show Archived"
            })
            .size(12)
            .color(colors::TEXT_SECONDARY)
        )
        .on_press(SessionMessage::ToggleIncludeArchived)
        .padding([6, 12])
        .style(button_styles::nav),
    ]
    .align_y(Vertical::Center);

    // Status summary
    let (active_count, inactive_count, archived_count) = state.status_counts();
    let status_summary = row![
        text(format!("{} active", active_count))
            .size(12)
            .color(colors::SUCCESS),
        Space::with_width(16),
        text(format!("{} inactive", inactive_count))
            .size(12)
            .color(colors::TEXT_MUTED),
        Space::with_width(16),
        text(format!("{} archived", archived_count))
            .size(12)
            .color(colors::TEXT_MUTED),
    ];

    // Session list
    let visible_sessions = state.visible_sessions();
    let session_cards: Vec<Element<SessionMessage>> = visible_sessions
        .iter()
        .map(|session| {
            view_session_card(session, state.active_session_id == Some(session.id))
        })
        .collect();

    let session_list = if state.loading {
        column![text("Loading sessions...")
            .size(14)
            .color(colors::TEXT_MUTED),]
    } else if session_cards.is_empty() {
        column![
            text("No sessions found").size(14).color(colors::TEXT_MUTED),
            Space::with_height(8),
            text("Click '+ New' to create a workspace")
                .size(12)
                .color(colors::TEXT_MUTED),
        ]
    } else {
        column![scrollable(column(session_cards).spacing(8)).height(Length::Fill),]
    };

    // Error message
    let error_section = if let Some(ref error) = state.error {
        container(
            row![
                text("Error: ").size(12).color(colors::ERROR),
                text(error).size(12).color(colors::ERROR),
                Space::with_width(Length::Fill),
                button(text("X").size(12).color(colors::ERROR))
                    .on_press(SessionMessage::ClearError)
                    .padding([2, 8])
                    .style(button_styles::nav),
            ]
            .align_y(Vertical::Center),
        )
        .padding([8, 12])
        .style(container_styles::badge_error)
    } else {
        container(Space::with_height(0))
    };

    // Create dialog overlay
    let create_dialog: Element<SessionMessage> = if state.show_create_dialog {
        view_create_session_dialog(state)
    } else {
        container(Space::with_height(0)).into()
    };

    // Main content
    let content = column![
        header,
        Space::with_height(16),
        filter_row,
        Space::with_height(12),
        status_summary,
        Space::with_height(16),
        error_section,
        Space::with_height(8),
        session_list,
        create_dialog,
    ]
    .spacing(0);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .into()
}

/// Render a single session card
fn view_session_card(session: &Session, is_active: bool) -> Element<SessionMessage> {
    let (status_color, status_text) = match session.status {
        SessionStatus::Active => (colors::SUCCESS, "Running"),
        SessionStatus::Starting => (colors::WARNING, "Starting"),
        SessionStatus::Stopping => (colors::WARNING, "Stopping"),
        SessionStatus::Inactive => (colors::TEXT_MUTED, "Stopped"),
        SessionStatus::Error => (colors::ERROR, "Error"),
        SessionStatus::Archived => (colors::TEXT_MUTED, "Archived"),
    };

    let path_str = session.path.display().to_string();
    let truncated_path = if path_str.len() > 50 {
        format!("...{}", &path_str[path_str.len() - 47..])
    } else {
        path_str
    };

    // Last accessed timestamp
    let last_accessed = session
        .last_accessed
        .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|| "Never".to_string());

    // Action buttons - daemon is global, so we just show select/delete actions
    let actions = match session.status {
        SessionStatus::Inactive | SessionStatus::Active => {
            row![button(
                text("Select")
                    .size(11)
                    .font(fonts::MONO)
                    .color(colors::PRIMARY)
            )
            .on_press(SessionMessage::SelectSession(session.id))
            .padding([4, 8])
            .style(button_styles::secondary),]
        }
        SessionStatus::Archived => {
            row![button(
                text("Delete")
                    .size(11)
                    .font(fonts::MONO)
                    .color(colors::ERROR)
            )
            .on_press(SessionMessage::DeleteSession(session.id))
            .padding([4, 8])
            .style(button_styles::danger),]
        }
        _ => row![],
    };

    // More actions dropdown (archive, etc.)
    let more_actions = if session.status != SessionStatus::Archived {
        row![button(text("...").size(11).color(colors::TEXT_MUTED))
            .on_press(SessionMessage::ArchiveSession(session.id))
            .padding([4, 8])
            .style(button_styles::nav),]
    } else {
        row![]
    };

    let card_content = column![
        // Header row with name and status
        row![
            text("\u{25CF}").size(10).color(status_color), // ●
            Space::with_width(8),
            text(session.name.clone())
                .size(16)
                .font(fonts::MONO_MEDIUM)
                .color(colors::TEXT_PRIMARY),
            Space::with_width(Length::Fill),
            text(status_text).size(12).color(status_color),
        ]
        .align_y(Vertical::Center),
        Space::with_height(6),
        // Path
        text(truncated_path)
            .size(11)
            .font(fonts::MONO)
            .color(colors::TEXT_MUTED),
        Space::with_height(4),
        // Footer with last accessed and actions
        row![
            text(format!("Last accessed: {}", last_accessed))
                .size(10)
                .color(colors::TEXT_MUTED),
            Space::with_width(Length::Fill),
            actions,
            Space::with_width(4),
            more_actions,
        ]
        .align_y(Vertical::Center),
    ]
    .spacing(2);

    let session_id = session.id;
    let card = button(card_content)
        .on_press(SessionMessage::SelectSession(session_id))
        .width(Length::Fill)
        .padding(12)
        .style(if is_active {
            button_styles::nav_active
        } else {
            button_styles::secondary
        });

    container(card).width(Length::Fill).into()
}

/// Render the create session dialog
fn view_create_session_dialog(state: &SessionState) -> Element<SessionMessage> {
    let dialog_content = column![
        // Title
        row![
            text("Create New Session")
                .size(18)
                .font(fonts::MONO_BOLD)
                .color(colors::TEXT_PRIMARY),
            Space::with_width(Length::Fill),
            button(text("X").size(14).color(colors::TEXT_MUTED))
                .on_press(SessionMessage::HideCreateDialog)
                .padding([4, 8])
                .style(button_styles::nav),
        ]
        .align_y(Vertical::Center),
        Space::with_height(16),
        // Name input
        text("Session Name")
            .size(12)
            .font(fonts::MONO_MEDIUM)
            .color(colors::TEXT_SECONDARY),
        Space::with_height(4),
        text_input("my-project", &state.new_session_name)
            .id(TextInputId::new("session-name"))
            .on_input(SessionMessage::UpdateNewSessionName)
            .on_submit(SessionMessage::FocusPathInput)
            .padding([8, 12])
            .size(14),
        Space::with_height(12),
        // Path input
        text("Workspace Path")
            .size(12)
            .font(fonts::MONO_MEDIUM)
            .color(colors::TEXT_SECONDARY),
        Space::with_height(4),
        text_input("/path/to/workspace", &state.new_session_path)
            .id(TextInputId::new("session-path"))
            .on_input(SessionMessage::UpdateNewSessionPath)
            .on_submit(SessionMessage::CreateSession)
            .padding([8, 12])
            .size(14),
        Space::with_height(16),
        // Action buttons
        row![
            Space::with_width(Length::Fill),
            button(
                text("Cancel")
                    .size(13)
                    .font(fonts::MONO)
                    .color(colors::TEXT_PRIMARY)
            )
            .on_press(SessionMessage::HideCreateDialog)
            .padding([8, 16])
            .style(button_styles::secondary),
            Space::with_width(8),
            button(
                text("Create")
                    .size(13)
                    .font(fonts::MONO_MEDIUM)
                    .color(colors::PRIMARY)
            )
            .on_press(SessionMessage::CreateSession)
            .padding([8, 16])
            .style(button_styles::primary),
        ]
        .align_y(Vertical::Center),
    ]
    .spacing(0)
    .padding(20);

    container(
        container(dialog_content)
            .width(400)
            .style(container_styles::panel),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .style(|_theme| container::Style {
        background: Some(iced::Color::from_rgba(0.0, 0.0, 0.0, 0.7).into()),
        ..Default::default()
    })
    .into()
}
