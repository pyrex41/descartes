//! Chat interface state management
//!
//! This module provides state management for the chat interface that wraps
//! Claude Code CLI for full session capabilities. Supports real-time streaming
//! via ZMQ PUB/SUB from the daemon.

use descartes_core::StreamChunk;
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
    /// Current session/chat ID (local)
    pub session_id: Option<Uuid>,
    /// Working directory for Claude Code
    pub working_directory: Option<String>,

    // === Streaming state (daemon-managed sessions) ===
    /// Current streaming text (accumulated from daemon)
    pub streaming_text: String,
    /// Current thinking text (accumulated from daemon)
    pub streaming_thinking: String,
    /// Is currently streaming from daemon
    pub is_streaming: bool,
    /// Active session ID from daemon
    pub daemon_session_id: Option<Uuid>,
    /// ZMQ PUB endpoint for streaming
    pub pub_endpoint: Option<String>,
    /// Current mode: "chat" or "agent"
    pub mode: String,
    /// Pending prompt to send after subscription is ready
    pub pending_prompt: Option<String>,
}

/// A single chat message entry
#[derive(Debug, Clone)]
pub struct ChatMessageEntry {
    pub id: Uuid,
    pub role: ChatRole,
    pub content: String,
    /// Thinking/reasoning content (optional, for assistant messages)
    pub thinking: Option<String>,
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
    /// Response completed (legacy - direct CLI mode)
    ResponseComplete(String),
    /// Error occurred
    Error(String),
    /// Clear error
    ClearError,
    /// Clear conversation and start new session
    ClearConversation,

    // === Daemon streaming messages ===
    /// Daemon session created (CLI not yet started, ready for subscription)
    SessionCreated {
        session_id: Uuid,
        pub_endpoint: String,
        pending_prompt: String,
    },
    /// Daemon session started with initial prompt (legacy - has race condition)
    SessionStarted {
        session_id: Uuid,
        pub_endpoint: String,
    },
    /// Send the pending prompt (called after subscription is ready)
    SendPendingPrompt,
    /// Stream chunk received from daemon
    StreamChunk(StreamChunk),
    /// Stream ended
    StreamEnded,
    /// Prompt sent to daemon
    PromptSent,
    /// Request to upgrade to agent mode
    UpgradeToAgent,
    /// Successfully upgraded to agent mode
    UpgradedToAgent,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            session_id: Some(Uuid::new_v4()),
            mode: "chat".to_string(),
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
                    thinking: None,
                    timestamp: chrono::Utc::now(),
                });
                state.prompt_input.clear();
                state.loading = true;
                state.error = None;
            }
        }
        ChatMessage::ResponseComplete(response) => {
            state.loading = false;

            // Add assistant message with response (legacy mode)
            state.messages.push(ChatMessageEntry {
                id: Uuid::new_v4(),
                role: ChatRole::Assistant,
                content: response,
                thinking: None,
                timestamp: chrono::Utc::now(),
            });
        }
        ChatMessage::Error(err) => {
            state.error = Some(err);
            state.loading = false;
            state.is_streaming = false;
        }
        ChatMessage::ClearError => {
            state.error = None;
        }
        ChatMessage::ClearConversation => {
            state.messages.clear();
            state.session_id = Some(Uuid::new_v4());
            state.daemon_session_id = None;
            state.pub_endpoint = None;
            state.streaming_text.clear();
            state.streaming_thinking.clear();
            state.is_streaming = false;
            state.mode = "chat".to_string();
        }

        // === Daemon streaming message handlers ===
        ChatMessage::SessionCreated {
            session_id,
            pub_endpoint,
            pending_prompt,
        } => {
            // Store session info (triggers subscription) and pending prompt
            state.daemon_session_id = Some(session_id);
            state.pub_endpoint = Some(pub_endpoint);
            state.pending_prompt = Some(pending_prompt);
            state.loading = true;
            // Note: CLI not started yet - will be started by SendPendingPrompt
        }
        ChatMessage::SessionStarted {
            session_id,
            pub_endpoint,
        } => {
            state.daemon_session_id = Some(session_id);
            state.pub_endpoint = Some(pub_endpoint);
            state.is_streaming = true;
            state.loading = true;
        }
        ChatMessage::SendPendingPrompt => {
            // Clear pending prompt (it will be sent by main.rs handler)
            state.pending_prompt = None;
            state.is_streaming = true;
        }
        ChatMessage::StreamChunk(chunk) => {
            match chunk {
                StreamChunk::Text { content } => {
                    state.streaming_text.push_str(&content);
                }
                StreamChunk::Thinking { content } => {
                    state.streaming_thinking.push_str(&content);
                }
                StreamChunk::TurnComplete { .. } => {
                    // Finalize the message for this turn
                    if !state.streaming_text.is_empty() || !state.streaming_thinking.is_empty() {
                        state.messages.push(ChatMessageEntry {
                            id: Uuid::new_v4(),
                            role: ChatRole::Assistant,
                            content: std::mem::take(&mut state.streaming_text),
                            thinking: if state.streaming_thinking.is_empty() {
                                None
                            } else {
                                Some(std::mem::take(&mut state.streaming_thinking))
                            },
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    state.loading = false;
                }
                StreamChunk::Complete { .. } => {
                    // Session completed - finalize any remaining content
                    if !state.streaming_text.is_empty() || !state.streaming_thinking.is_empty() {
                        state.messages.push(ChatMessageEntry {
                            id: Uuid::new_v4(),
                            role: ChatRole::Assistant,
                            content: std::mem::take(&mut state.streaming_text),
                            thinking: if state.streaming_thinking.is_empty() {
                                None
                            } else {
                                Some(std::mem::take(&mut state.streaming_thinking))
                            },
                            timestamp: chrono::Utc::now(),
                        });
                    }
                    state.is_streaming = false;
                    state.loading = false;
                }
                StreamChunk::Error { message } => {
                    state.error = Some(message);
                    state.is_streaming = false;
                    state.loading = false;
                }
                StreamChunk::ToolUseStart { .. }
                | StreamChunk::ToolUseInput { .. }
                | StreamChunk::ToolResult { .. } => {
                    // Tool use events - could be displayed in UI later
                    // For now, just track that we're still active
                }
            }
        }
        ChatMessage::StreamEnded => {
            state.is_streaming = false;
            state.loading = false;
        }
        ChatMessage::PromptSent => {
            // Prompt was sent to daemon, now waiting for response
            state.loading = true;
            state.is_streaming = true;
        }
        ChatMessage::UpgradeToAgent => {
            // This is handled in main.rs - triggers RPC call
        }
        ChatMessage::UpgradedToAgent => {
            state.mode = "agent".to_string();
        }
    }
}
