//! Chat interface view component
//!
//! Provides a chat UI for interacting with Claude Code in full session mode.
//! Supports streaming output with thinking blocks displayed distinctively.

use crate::chat_state::{ChatMessage, ChatMessageEntry, ChatRole, ChatState, SubAgentInfo};
use crate::theme::{button_styles, colors, container_styles, fonts};
use iced::alignment::Vertical;
use iced::widget::text_input::Id as TextInputId;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::{Element, Length};

/// Color for thinking blocks (semi-transparent purple/blue)
const THINKING_COLOR: iced::Color = iced::Color {
    r: 0.6,
    g: 0.5,
    b: 0.8,
    a: 1.0,
};

/// Background color for thinking blocks
const THINKING_BG: iced::Color = iced::Color {
    r: 0.1,
    g: 0.1,
    b: 0.15,
    a: 1.0,
};

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

    // Mode indicator and upgrade button
    let mode_section = if state.daemon_session_id.is_some() {
        if state.mode == "agent" {
            // Already in agent mode
            container(
                row![
                    text("⚡").size(10).color(colors::SUCCESS),
                    Space::with_width(4),
                    text("Agent Mode").size(11).color(colors::SUCCESS),
                ]
                .align_y(Vertical::Center),
            )
            .padding([4, 8])
        } else {
            // In chat mode, offer upgrade
            container(
                button(
                    row![
                        text("⚡").size(10).color(colors::PRIMARY),
                        Space::with_width(4),
                        text("Upgrade to Agent").size(11).color(colors::PRIMARY),
                    ]
                    .align_y(Vertical::Center),
                )
                .on_press(ChatMessage::UpgradeToAgent)
                .padding([4, 8])
                .style(button_styles::secondary),
            )
        }
    } else {
        container(Space::with_width(0))
    };

    let controls_row = row![
        session_status,
        Space::with_width(24),
        workdir_display,
        Space::with_width(Length::Fill),
        mode_section,
        Space::with_width(12),
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
    let loading_indicator = if state.loading && !state.is_streaming {
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

    // Streaming content display (real-time output)
    let streaming_section = if state.is_streaming
        && (!state.streaming_text.is_empty() || !state.streaming_thinking.is_empty())
    {
        let mut content: Vec<Element<ChatMessage>> = vec![];

        // Show thinking block first if present
        if !state.streaming_thinking.is_empty() {
            content.push(
                container(
                    column![
                        row![
                            text("💭").size(12),
                            Space::with_width(6),
                            text("Thinking...").size(11).color(THINKING_COLOR),
                        ]
                        .align_y(Vertical::Center),
                        Space::with_height(4),
                        text(&state.streaming_thinking)
                            .size(12)
                            .font(fonts::MONO)
                            .color(THINKING_COLOR),
                    ]
                    .spacing(4),
                )
                .padding(8)
                .width(Length::Fill)
                .style(container_styles::panel)
                .into(),
            );
        }

        // Show streaming text if present
        if !state.streaming_text.is_empty() {
            content.push(
                container(
                    column![
                        row![
                            text("◎").size(12).color(colors::SUCCESS),
                            Space::with_width(6),
                            text("Claude").size(11).color(colors::SUCCESS),
                            Space::with_width(6),
                            text("▌").size(12).color(colors::PRIMARY), // Cursor
                        ]
                        .align_y(Vertical::Center),
                        Space::with_height(4),
                        text(&state.streaming_text)
                            .size(14)
                            .font(fonts::MONO)
                            .color(colors::TEXT_PRIMARY),
                    ]
                    .spacing(4),
                )
                .padding(12)
                .width(Length::Fill)
                .style(container_styles::panel)
                .into(),
            );
        }

        container(column(content).spacing(8))
            .padding([8, 0])
            .width(Length::Fill)
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

    // Sub-agents section (shows active sub-agents spawned by the main agent)
    let sub_agents_section = if !state.sub_agents.is_empty() {
        let agent_items: Vec<Element<ChatMessage>> = state
            .sub_agents
            .iter()
            .map(|agent| view_sub_agent(agent))
            .collect();

        container(
            column![
                row![
                    text("🔀").size(12),
                    Space::with_width(6),
                    text(format!("Sub-agents ({})", state.sub_agents.len()))
                        .size(11)
                        .font(fonts::MONO_MEDIUM)
                        .color(colors::TEXT_SECONDARY),
                ]
                .align_y(Vertical::Center),
                Space::with_height(8),
                column(agent_items).spacing(6),
            ]
            .spacing(4),
        )
        .padding(10)
        .width(Length::Fill)
        .style(container_styles::panel)
    } else {
        container(Space::with_height(0))
    };

    column![
        title,
        Space::with_height(4),
        subtitle,
        Space::with_height(16),
        controls_row,
        Space::with_height(8),
        sub_agents_section,
        Space::with_height(8),
        messages_area,
        Space::with_height(8),
        streaming_section,
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

    // Build content column with optional thinking block
    let mut content_parts: Vec<Element<ChatMessage>> = vec![];

    // Header row
    content_parts.push(
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
        .align_y(Vertical::Center)
        .into(),
    );

    content_parts.push(Space::with_height(8).into());

    // Thinking block (if present for assistant messages)
    if let Some(ref thinking) = msg.thinking {
        if !thinking.is_empty() {
            content_parts.push(
                container(
                    column![
                        row![
                            text("💭").size(10),
                            Space::with_width(4),
                            text("Thinking").size(10).color(THINKING_COLOR),
                        ]
                        .align_y(Vertical::Center),
                        Space::with_height(4),
                        text(thinking)
                            .size(11)
                            .font(fonts::MONO)
                            .color(THINKING_COLOR),
                    ]
                    .spacing(2),
                )
                .padding(8)
                .width(Length::Fill)
                .style(container_styles::panel)
                .into(),
            );
            content_parts.push(Space::with_height(8).into());
        }
    }

    // Main content
    content_parts.push(
        text(&msg.content)
            .size(14)
            .font(fonts::MONO)
            .color(colors::TEXT_PRIMARY)
            .into(),
    );

    let content = column(content_parts);

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

/// Render a single sub-agent info card
fn view_sub_agent(agent: &SubAgentInfo) -> Element<ChatMessage> {
    let type_label = agent
        .subagent_type
        .as_ref()
        .map(|t| t.as_str())
        .unwrap_or("general");

    // Truncate prompt for display (first 60 chars)
    let prompt_preview = if agent.prompt.len() > 60 {
        format!("{}...", &agent.prompt[..60])
    } else {
        agent.prompt.clone()
    };

    let timestamp = agent.spawned_at.format("%H:%M:%S").to_string();

    container(
        row![
            // Agent type badge
            container(
                text(type_label)
                    .size(9)
                    .font(fonts::MONO)
                    .color(colors::PRIMARY)
            )
            .padding([2, 6])
            .style(container_styles::panel),
            Space::with_width(8),
            // Agent ID
            text(&agent.agent_id)
                .size(10)
                .font(fonts::MONO)
                .color(colors::TEXT_MUTED),
            Space::with_width(12),
            // Prompt preview
            text(prompt_preview)
                .size(11)
                .font(fonts::MONO)
                .color(colors::TEXT_SECONDARY),
            Space::with_width(Length::Fill),
            // Timestamp
            text(timestamp)
                .size(9)
                .color(colors::TEXT_MUTED),
        ]
        .align_y(Vertical::Center),
    )
    .padding([6, 8])
    .width(Length::Fill)
    .style(container_styles::card)
    .into()
}
