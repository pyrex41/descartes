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

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // HELPER FUNCTIONS
    // ============================================================================

    fn create_default_state() -> ChatState {
        ChatState::new()
    }

    // ============================================================================
    // INITIALIZATION TESTS
    // ============================================================================

    #[test]
    fn test_chat_state_new() {
        let state = ChatState::new();
        assert!(state.session_id.is_some());
        assert_eq!(state.mode, "chat");
        assert!(state.messages.is_empty());
        assert!(!state.loading);
        assert!(state.error.is_none());
        assert!(!state.is_streaming);
    }

    #[test]
    fn test_chat_state_default() {
        let state = ChatState::default();
        assert!(state.session_id.is_none()); // Default doesn't set session_id
        assert!(state.mode.is_empty()); // Default string is empty
    }

    // ============================================================================
    // PROMPT UPDATE TESTS
    // ============================================================================

    #[test]
    fn test_update_prompt() {
        let mut state = create_default_state();
        update(&mut state, ChatMessage::UpdatePrompt("Hello world".to_string()));
        assert_eq!(state.prompt_input, "Hello world");
    }

    #[test]
    fn test_update_prompt_multiple_times() {
        let mut state = create_default_state();
        update(&mut state, ChatMessage::UpdatePrompt("First".to_string()));
        assert_eq!(state.prompt_input, "First");
        update(&mut state, ChatMessage::UpdatePrompt("Second".to_string()));
        assert_eq!(state.prompt_input, "Second");
    }

    // ============================================================================
    // SUBMIT PROMPT TESTS
    // ============================================================================

    #[test]
    fn test_submit_prompt_adds_user_message() {
        let mut state = create_default_state();
        state.prompt_input = "Test prompt".to_string();

        update(&mut state, ChatMessage::SubmitPrompt);

        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, ChatRole::User);
        assert_eq!(state.messages[0].content, "Test prompt");
        assert!(state.prompt_input.is_empty());
        assert!(state.loading);
        assert!(state.error.is_none());
    }

    #[test]
    fn test_submit_empty_prompt_does_nothing() {
        let mut state = create_default_state();
        state.prompt_input = "   ".to_string(); // Whitespace only

        update(&mut state, ChatMessage::SubmitPrompt);

        assert!(state.messages.is_empty());
        assert!(!state.loading);
    }

    #[test]
    fn test_submit_clears_previous_error() {
        let mut state = create_default_state();
        state.prompt_input = "Test".to_string();
        state.error = Some("Previous error".to_string());

        update(&mut state, ChatMessage::SubmitPrompt);

        assert!(state.error.is_none());
    }

    // ============================================================================
    // RESPONSE COMPLETE TESTS (LEGACY MODE)
    // ============================================================================

    #[test]
    fn test_response_complete() {
        let mut state = create_default_state();
        state.loading = true;

        update(&mut state, ChatMessage::ResponseComplete("Assistant response".to_string()));

        assert!(!state.loading);
        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, ChatRole::Assistant);
        assert_eq!(state.messages[0].content, "Assistant response");
    }

    // ============================================================================
    // ERROR HANDLING TESTS
    // ============================================================================

    #[test]
    fn test_error_message() {
        let mut state = create_default_state();
        state.loading = true;
        state.is_streaming = true;

        update(&mut state, ChatMessage::Error("Test error".to_string()));

        assert_eq!(state.error, Some("Test error".to_string()));
        assert!(!state.loading);
        assert!(!state.is_streaming);
    }

    #[test]
    fn test_clear_error() {
        let mut state = create_default_state();
        state.error = Some("Existing error".to_string());

        update(&mut state, ChatMessage::ClearError);

        assert!(state.error.is_none());
    }

    // ============================================================================
    // CLEAR CONVERSATION TESTS
    // ============================================================================

    #[test]
    fn test_clear_conversation() {
        let mut state = create_default_state();
        let original_session_id = state.session_id;

        // Add some state
        state.messages.push(ChatMessageEntry {
            id: Uuid::new_v4(),
            role: ChatRole::User,
            content: "Test".to_string(),
            thinking: None,
            timestamp: chrono::Utc::now(),
        });
        state.daemon_session_id = Some(Uuid::new_v4());
        state.pub_endpoint = Some("tcp://127.0.0.1:19480".to_string());
        state.streaming_text = "Some text".to_string();
        state.streaming_thinking = "Some thinking".to_string();
        state.is_streaming = true;
        state.mode = "agent".to_string();

        update(&mut state, ChatMessage::ClearConversation);

        assert!(state.messages.is_empty());
        assert!(state.session_id.is_some());
        assert_ne!(state.session_id, original_session_id); // New session ID
        assert!(state.daemon_session_id.is_none());
        assert!(state.pub_endpoint.is_none());
        assert!(state.streaming_text.is_empty());
        assert!(state.streaming_thinking.is_empty());
        assert!(!state.is_streaming);
        assert_eq!(state.mode, "chat");
    }

    // ============================================================================
    // DAEMON STREAMING TESTS - SESSION CREATED
    // ============================================================================

    #[test]
    fn test_session_created() {
        let mut state = create_default_state();
        let session_id = Uuid::new_v4();
        let pub_endpoint = "tcp://127.0.0.1:19480".to_string();
        let pending_prompt = "Hello".to_string();

        update(&mut state, ChatMessage::SessionCreated {
            session_id,
            pub_endpoint: pub_endpoint.clone(),
            pending_prompt: pending_prompt.clone(),
        });

        assert_eq!(state.daemon_session_id, Some(session_id));
        assert_eq!(state.pub_endpoint, Some(pub_endpoint));
        assert_eq!(state.pending_prompt, Some(pending_prompt));
        assert!(state.loading);
    }

    // ============================================================================
    // DAEMON STREAMING TESTS - SESSION STARTED (LEGACY)
    // ============================================================================

    #[test]
    fn test_session_started() {
        let mut state = create_default_state();
        let session_id = Uuid::new_v4();
        let pub_endpoint = "tcp://127.0.0.1:19480".to_string();

        update(&mut state, ChatMessage::SessionStarted {
            session_id,
            pub_endpoint: pub_endpoint.clone(),
        });

        assert_eq!(state.daemon_session_id, Some(session_id));
        assert_eq!(state.pub_endpoint, Some(pub_endpoint));
        assert!(state.is_streaming);
        assert!(state.loading);
    }

    // ============================================================================
    // DAEMON STREAMING TESTS - SEND PENDING PROMPT
    // ============================================================================

    #[test]
    fn test_send_pending_prompt() {
        let mut state = create_default_state();
        state.pending_prompt = Some("Test prompt".to_string());

        update(&mut state, ChatMessage::SendPendingPrompt);

        assert!(state.pending_prompt.is_none());
        assert!(state.is_streaming);
    }

    // ============================================================================
    // STREAM CHUNK TESTS - TEXT
    // ============================================================================

    #[test]
    fn test_stream_chunk_text() {
        let mut state = create_default_state();

        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Text {
            content: "Hello ".to_string(),
        }));
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Text {
            content: "world!".to_string(),
        }));

        assert_eq!(state.streaming_text, "Hello world!");
    }

    // ============================================================================
    // STREAM CHUNK TESTS - THINKING
    // ============================================================================

    #[test]
    fn test_stream_chunk_thinking() {
        let mut state = create_default_state();

        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Thinking {
            content: "Let me think...".to_string(),
        }));

        assert_eq!(state.streaming_thinking, "Let me think...");
    }

    // ============================================================================
    // STREAM CHUNK TESTS - TURN COMPLETE
    // ============================================================================

    #[test]
    fn test_stream_chunk_turn_complete_with_content() {
        let mut state = create_default_state();
        state.loading = true;
        state.streaming_text = "Response text".to_string();
        state.streaming_thinking = "Thinking text".to_string();

        update(&mut state, ChatMessage::StreamChunk(StreamChunk::TurnComplete {
            turn_number: 0,
        }));

        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].role, ChatRole::Assistant);
        assert_eq!(state.messages[0].content, "Response text");
        assert_eq!(state.messages[0].thinking, Some("Thinking text".to_string()));
        assert!(state.streaming_text.is_empty());
        assert!(state.streaming_thinking.is_empty());
        assert!(!state.loading);
    }

    #[test]
    fn test_stream_chunk_turn_complete_empty_thinking() {
        let mut state = create_default_state();
        state.streaming_text = "Response text".to_string();

        update(&mut state, ChatMessage::StreamChunk(StreamChunk::TurnComplete {
            turn_number: 0,
        }));

        assert_eq!(state.messages.len(), 1);
        assert!(state.messages[0].thinking.is_none());
    }

    #[test]
    fn test_stream_chunk_turn_complete_no_content() {
        let mut state = create_default_state();
        // No streaming text or thinking

        update(&mut state, ChatMessage::StreamChunk(StreamChunk::TurnComplete {
            turn_number: 0,
        }));

        // No message added if both are empty
        assert!(state.messages.is_empty());
    }

    // ============================================================================
    // STREAM CHUNK TESTS - COMPLETE
    // ============================================================================

    #[test]
    fn test_stream_chunk_complete() {
        let mut state = create_default_state();
        state.loading = true;
        state.is_streaming = true;
        state.streaming_text = "Final response".to_string();

        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Complete {
            exit_code: 0,
        }));

        assert_eq!(state.messages.len(), 1);
        assert_eq!(state.messages[0].content, "Final response");
        assert!(!state.is_streaming);
        assert!(!state.loading);
    }

    #[test]
    fn test_stream_chunk_complete_no_remaining_content() {
        let mut state = create_default_state();
        state.is_streaming = true;
        // No streaming content

        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Complete {
            exit_code: 0,
        }));

        // No message added if no content
        assert!(state.messages.is_empty());
        assert!(!state.is_streaming);
    }

    // ============================================================================
    // STREAM CHUNK TESTS - ERROR
    // ============================================================================

    #[test]
    fn test_stream_chunk_error() {
        let mut state = create_default_state();
        state.loading = true;
        state.is_streaming = true;

        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Error {
            message: "API error".to_string(),
        }));

        assert_eq!(state.error, Some("API error".to_string()));
        assert!(!state.is_streaming);
        assert!(!state.loading);
    }

    // ============================================================================
    // STREAM CHUNK TESTS - TOOL USE EVENTS
    // ============================================================================

    #[test]
    fn test_stream_chunk_tool_use_events() {
        let mut state = create_default_state();

        // Tool events should not change state significantly
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::ToolUseStart {
            tool_name: "read_file".to_string(),
            tool_id: "tool-123".to_string(),
        }));
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::ToolUseInput {
            tool_id: "tool-123".to_string(),
            input: serde_json::json!({"path": "/tmp/test.txt"}),
        }));
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::ToolResult {
            tool_id: "tool-123".to_string(),
            result: "File contents".to_string(),
            is_error: false,
        }));

        // No messages or streaming text changes from tool events
        assert!(state.messages.is_empty());
        assert!(state.streaming_text.is_empty());
    }

    // ============================================================================
    // OTHER MESSAGE HANDLERS
    // ============================================================================

    #[test]
    fn test_stream_ended() {
        let mut state = create_default_state();
        state.is_streaming = true;
        state.loading = true;

        update(&mut state, ChatMessage::StreamEnded);

        assert!(!state.is_streaming);
        assert!(!state.loading);
    }

    #[test]
    fn test_prompt_sent() {
        let mut state = create_default_state();

        update(&mut state, ChatMessage::PromptSent);

        assert!(state.loading);
        assert!(state.is_streaming);
    }

    #[test]
    fn test_upgraded_to_agent() {
        let mut state = create_default_state();
        assert_eq!(state.mode, "chat");

        update(&mut state, ChatMessage::UpgradedToAgent);

        assert_eq!(state.mode, "agent");
    }

    // ============================================================================
    // FULL CONVERSATION FLOW TESTS
    // ============================================================================

    #[test]
    fn test_full_streaming_conversation_flow() {
        let mut state = create_default_state();
        let session_id = Uuid::new_v4();

        // 1. User enters prompt
        update(&mut state, ChatMessage::UpdatePrompt("What is Rust?".to_string()));

        // 2. User submits prompt
        update(&mut state, ChatMessage::SubmitPrompt);
        assert_eq!(state.messages.len(), 1);
        assert!(state.loading);

        // 3. Session is created
        update(&mut state, ChatMessage::SessionCreated {
            session_id,
            pub_endpoint: "tcp://127.0.0.1:19480".to_string(),
            pending_prompt: "What is Rust?".to_string(),
        });

        // 4. Pending prompt sent
        update(&mut state, ChatMessage::SendPendingPrompt);
        assert!(state.is_streaming);

        // 5. Receive thinking chunks
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Thinking {
            content: "The user is asking about Rust programming language.".to_string(),
        }));

        // 6. Receive text chunks
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Text {
            content: "Rust is a ".to_string(),
        }));
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Text {
            content: "systems programming language.".to_string(),
        }));

        // 7. Turn completes
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::TurnComplete {
            turn_number: 0,
        }));
        assert_eq!(state.messages.len(), 2); // User + Assistant

        // 8. Session completes
        update(&mut state, ChatMessage::StreamChunk(StreamChunk::Complete {
            exit_code: 0,
        }));
        assert!(!state.is_streaming);
        assert!(!state.loading);

        // Verify final state
        assert_eq!(state.messages.len(), 2);
        assert_eq!(state.messages[0].role, ChatRole::User);
        assert_eq!(state.messages[1].role, ChatRole::Assistant);
        assert_eq!(state.messages[1].content, "Rust is a systems programming language.");
        assert!(state.messages[1].thinking.is_some());
    }

    // ============================================================================
    // CHAT ROLE EQUALITY TEST
    // ============================================================================

    #[test]
    fn test_chat_role_equality() {
        assert_eq!(ChatRole::User, ChatRole::User);
        assert_eq!(ChatRole::Assistant, ChatRole::Assistant);
        assert_eq!(ChatRole::System, ChatRole::System);
        assert_ne!(ChatRole::User, ChatRole::Assistant);
    }
}
