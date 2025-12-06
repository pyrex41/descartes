//! Chat interface state management
//!
//! This module provides state management for the chat interface that wraps
//! Claude Code CLI for full session capabilities.

use uuid::Uuid;

/// State for the chat interface
#[derive(Debug, Clone, Default)]
pub struct ChatState {
    /// Current prompt input
    pub prompt_input: String,
    /// Conversation history
    pub messages: Vec<ChatMessageEntry>,
    /// Is currently waiting for response
    pub loading: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Current session/chat ID
    pub session_id: Option<Uuid>,
    /// Working directory for Claude Code
    pub working_directory: Option<String>,
}

/// A single chat message entry
#[derive(Debug, Clone)]
pub struct ChatMessageEntry {
    pub id: Uuid,
    pub role: ChatRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Message role in chat
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

/// Messages for chat operations
#[derive(Debug, Clone)]
pub enum ChatMessage {
    /// Update prompt input text
    UpdatePrompt(String),
    /// Submit the current prompt
    SubmitPrompt,
    /// Response completed
    ResponseComplete(String),
    /// Error occurred
    Error(String),
    /// Clear error
    ClearError,
    /// Clear conversation and start new session
    ClearConversation,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            session_id: Some(Uuid::new_v4()),
            ..Default::default()
        }
    }
}

/// Update chat state based on messages
pub fn update(state: &mut ChatState, message: ChatMessage) {
    match message {
        ChatMessage::UpdatePrompt(text) => {
            state.prompt_input = text;
        }
        ChatMessage::SubmitPrompt => {
            if !state.prompt_input.trim().is_empty() {
                // Add user message to history
                state.messages.push(ChatMessageEntry {
                    id: Uuid::new_v4(),
                    role: ChatRole::User,
                    content: state.prompt_input.clone(),
                    timestamp: chrono::Utc::now(),
                });
                state.prompt_input.clear();
                state.loading = true;
                state.error = None;
            }
        }
        ChatMessage::ResponseComplete(response) => {
            state.loading = false;

            // Add assistant message with response
            state.messages.push(ChatMessageEntry {
                id: Uuid::new_v4(),
                role: ChatRole::Assistant,
                content: response,
                timestamp: chrono::Utc::now(),
            });
        }
        ChatMessage::Error(err) => {
            state.error = Some(err);
            state.loading = false;
        }
        ChatMessage::ClearError => {
            state.error = None;
        }
        ChatMessage::ClearConversation => {
            state.messages.clear();
            state.session_id = Some(Uuid::new_v4());
        }
    }
}
