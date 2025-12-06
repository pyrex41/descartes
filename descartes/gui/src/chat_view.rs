//! Chat interface view component
//!
//! Provides a chat UI for interacting with Claude Code in full session mode.

use crate::chat_state::{ChatMessage, ChatMessageEntry, ChatRole, ChatState};
use crate::theme::{button_styles, colors, container_styles, fonts};
use iced::alignment::Vertical;
use iced::widget::text_input::Id as TextInputId;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Element, Length};

/// Render the chat view
pub fn view(state: &ChatState) -> Element<ChatMessage> {
    let title = text("Agent Chat")
        .size(24)
        .font(fonts::MONO_BOLD)
        .color(colors::TEXT_PRIMARY);

    let subtitle = text("Full-session Claude Code integration")
        .size(14)
        .color(colors::TEXT_SECONDARY);

    // Status row showing session info
    let session_status = if state.loading {
        row![
            text("●").size(10).color(colors::PRIMARY),
            Space::with_width(6),
            text("Processing...").size(12).color(colors::PRIMARY),
        ]
        .align_y(Vertical::Center)
    } else if !state.messages.is_empty() {
        row![
            text("●").size(10).color(colors::SUCCESS),
            Space::with_width(6),
            text("Session active").size(12).color(colors::SUCCESS),
        ]
        .align_y(Vertical::Center)
    } else {
        row![
            text("○").size(10).color(colors::TEXT_MUTED),
            Space::with_width(6),
            text("Ready").size(12).color(colors::TEXT_MUTED),
        ]
        .align_y(Vertical::Center)
    };

    // Working directory display
    let workdir_display = if let Some(ref dir) = state.working_directory {
        row![
            text("📁").size(12),
            Space::with_width(6),
            text(dir).size(11).color(colors::TEXT_MUTED),
        ]
        .align_y(Vertical::Center)
    } else {
        row![
            text("📁").size(12),
            Space::with_width(6),
            text("No working directory set").size(11).color(colors::TEXT_MUTED),
        ]
        .align_y(Vertical::Center)
    };

    let controls_row = row![
        session_status,
        Space::with_width(24),
        workdir_display,
        Space::with_width(Length::Fill),
        button(
            text("Clear")
                .size(12)
                .font(fonts::MONO)
                .color(colors::TEXT_MUTED)
        )
        .on_press(ChatMessage::ClearConversation)
        .padding([4, 8])
        .style(button_styles::nav),
    ]
    .align_y(Vertical::Center);

    // Messages area
    let messages_content: Vec<Element<ChatMessage>> = state
        .messages
        .iter()
        .map(|msg| view_message(msg))
        .collect();

    let messages_area = if messages_content.is_empty() {
        container(
            column![
                text("◎").size(48).color(colors::TEXT_MUTED),
                Space::with_height(16),
                text("No messages yet")
                    .size(16)
                    .font(fonts::MONO_MEDIUM)
                    .color(colors::TEXT_MUTED),
                Space::with_height(8),
                text("Type a task below and press Enter to start a Claude Code session")
                    .size(12)
                    .color(colors::TEXT_MUTED),
                Space::with_height(16),
                text("Claude Code provides full agentic capabilities:")
                    .size(12)
                    .color(colors::TEXT_SECONDARY),
                Space::with_height(4),
                text("• File editing and creation")
                    .size(11)
                    .color(colors::TEXT_MUTED),
                text("• Bash command execution")
                    .size(11)
                    .color(colors::TEXT_MUTED),
                text("• Web search and research")
                    .size(11)
                    .color(colors::TEXT_MUTED),
                text("• Multi-turn conversation context")
                    .size(11)
                    .color(colors::TEXT_MUTED),
            ]
            .align_x(iced::alignment::Horizontal::Center),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
    } else {
        container(
            scrollable(column(messages_content).spacing(12))
                .height(Length::Fill)
                .anchor_bottom(),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .style(container_styles::card)
    };

    // Error display
    let error_section = if let Some(ref error) = state.error {
        container(
            row![
                text("⚠").size(12).color(colors::ERROR),
                Space::with_width(8),
                text(error).size(12).color(colors::ERROR),
                Space::with_width(Length::Fill),
                button(
                    text("✕")
                        .size(12)
                        .font(fonts::MONO)
                        .color(colors::ERROR)
                )
                .on_press(ChatMessage::ClearError)
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

    // Loading indicator
    let loading_indicator = if state.loading {
        container(
            row![
                text("◐").size(14).color(colors::PRIMARY),
                Space::with_width(8),
                text("Claude is thinking...")
                    .size(12)
                    .font(fonts::MONO)
                    .color(colors::PRIMARY),
            ]
            .align_y(Vertical::Center),
        )
        .padding([8, 0])
    } else {
        container(Space::with_height(0))
    };

    // Input area
    let input_row = row![
        container(
            text_input("Type your task here...", &state.prompt_input)
                .id(TextInputId::new("chat-prompt"))
                .on_input(ChatMessage::UpdatePrompt)
                .on_submit(ChatMessage::SubmitPrompt)
                .padding([12, 16])
                .size(14)
        )
        .width(Length::Fill),
        Space::with_width(12),
        button(
            text(if state.loading { "..." } else { "Send" })
                .size(14)
                .font(fonts::MONO_MEDIUM)
                .color(if state.loading {
                    colors::TEXT_MUTED
                } else {
                    colors::PRIMARY
                })
        )
        .on_press_maybe(
            if state.loading || state.prompt_input.trim().is_empty() {
                None
            } else {
                Some(ChatMessage::SubmitPrompt)
            }
        )
        .padding([12, 24])
        .style(button_styles::primary),
    ]
    .align_y(Vertical::Center);

    // Hint text
    let hint_text = text("Press Enter to send • Claude Code runs with full tool access")
        .size(10)
        .color(colors::TEXT_MUTED);

    column![
        title,
        Space::with_height(4),
        subtitle,
        Space::with_height(16),
        controls_row,
        Space::with_height(12),
        messages_area,
        Space::with_height(8),
        error_section,
        loading_indicator,
        Space::with_height(8),
        input_row,
        Space::with_height(4),
        hint_text,
    ]
    .spacing(0)
    .into()
}

/// Render a single chat message
fn view_message(msg: &ChatMessageEntry) -> Element<ChatMessage> {
    let (icon, name) = match msg.role {
        ChatRole::User => ("◆", "You"),
        ChatRole::Assistant => ("◎", "Claude"),
        ChatRole::System => ("⚙", "System"),
    };

    let role_color = match msg.role {
        ChatRole::User => colors::PRIMARY,
        ChatRole::Assistant => colors::SUCCESS,
        ChatRole::System => colors::TEXT_MUTED,
    };

    let timestamp = msg.timestamp.format("%H:%M:%S").to_string();

    // Format content - handle potential markdown/code blocks
    let content_text = text(&msg.content)
        .size(14)
        .font(fonts::MONO)
        .color(colors::TEXT_PRIMARY);

    let content = column![
        row![
            text(icon).size(12).color(role_color),
            Space::with_width(8),
            text(name)
                .size(12)
                .font(fonts::MONO_MEDIUM)
                .color(role_color),
            Space::with_width(Length::Fill),
            text(timestamp).size(10).color(colors::TEXT_MUTED),
        ]
        .align_y(Vertical::Center),
        Space::with_height(8),
        content_text,
    ];

    // Use different styles for different roles
    match msg.role {
        ChatRole::Assistant => container(content)
            .padding(12)
            .width(Length::Fill)
            .style(container_styles::panel)
            .into(),
        _ => container(content)
            .padding(12)
            .width(Length::Fill)
            .style(container_styles::card)
            .into(),
    }
}
