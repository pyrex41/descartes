//! Chat session management
//!
//! Manages chat sessions using CLI backends (Claude Code, etc.) and streams
//! output to GUI clients via ZMQ PUB socket.

use crate::zmq_publisher::ZmqPublisher;
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use descartes_core::{ChatSessionConfig, CliBackend, ClaudeBackend, StreamChunk};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// Active chat session info (returned by RPC)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionInfo {
    pub session_id: String,
    pub working_dir: String,
    pub created_at: DateTime<Utc>,
    pub is_active: bool,
    pub turn_count: u32,
    /// Mode: "chat" or "agent"
    pub mode: String,
}

/// Internal session tracking
struct SessionTracker {
    working_dir: String,
    created_at: DateTime<Utc>,
    is_active: bool,
    turn_count: u32,
    mode: String,
    /// Stored config for deferred CLI start
    config: Option<ChatSessionConfig>,
    /// Whether the CLI backend has been started
    cli_started: bool,
}

/// Chat session manager
///
/// Coordinates between CLI backends and ZMQ publisher for streaming output.
pub struct ChatManager {
    backend: Arc<dyn CliBackend>,
    publisher: Arc<ZmqPublisher>,
    sessions: Arc<DashMap<Uuid, SessionTracker>>,
}

impl ChatManager {
    /// Create a new chat manager with the given publisher
    pub fn new(publisher: Arc<ZmqPublisher>) -> Self {
        Self {
            backend: Arc::new(ClaudeBackend::new()),
            publisher,
            sessions: Arc::new(DashMap::new()),
        }
    }

    /// Start a new chat session
    ///
    /// Returns the session ID. Clients should subscribe to the ZMQ PUB socket
    /// with topic `chat/{session_id}` to receive stream chunks.
    pub async fn start_session(&self, config: ChatSessionConfig) -> Result<Uuid, String> {
        // Start the backend session
        let handle = self.backend.start_session(config.clone()).await?;
        let session_id = handle.session_id;

        // Store session info
        self.sessions.insert(
            session_id,
            SessionTracker {
                working_dir: config.working_dir.clone(),
                created_at: Utc::now(),
                is_active: true,
                turn_count: 0,
                mode: "chat".to_string(),
                config: None, // No stored config - CLI already started
                cli_started: true,
            },
        );

        // Spawn task to forward stream chunks to ZMQ publisher
        let publisher = self.publisher.clone();
        let sessions = self.sessions.clone();
        let mut stream_rx = handle.stream_rx;

        tokio::spawn(async move {
            tracing::info!("Started streaming task for session {}", session_id);

            while let Some(chunk) = stream_rx.recv().await {
                tracing::debug!("Received chunk for session {}: {:?}", session_id, chunk);

                // Update turn count on turn complete
                if matches!(chunk, StreamChunk::TurnComplete { .. }) {
                    if let Some(mut info) = sessions.get_mut(&session_id) {
                        info.turn_count += 1;
                    }
                }

                // Check for completion
                let is_complete = matches!(chunk, StreamChunk::Complete { .. });

                // Publish to ZMQ
                if let Err(e) = publisher.publish(session_id, &chunk).await {
                    tracing::error!("Failed to publish chunk for session {}: {}", session_id, e);
                }

                if is_complete {
                    if let Some(mut info) = sessions.get_mut(&session_id) {
                        info.is_active = false;
                    }
                    tracing::info!("Session {} completed", session_id);
                    break;
                }
            }

            tracing::info!("Streaming task ended for session {}", session_id);
        });

        Ok(session_id)
    }

    /// Create a new chat session without starting the CLI
    ///
    /// Returns the session ID. Clients should subscribe to the ZMQ PUB socket
    /// with topic `chat/{session_id}` BEFORE calling send_prompt.
    pub fn create_session(&self, config: ChatSessionConfig) -> Uuid {
        let session_id = Uuid::new_v4();

        // Store session info with deferred CLI start
        self.sessions.insert(
            session_id,
            SessionTracker {
                working_dir: config.working_dir.clone(),
                created_at: Utc::now(),
                is_active: false, // Not active until CLI starts
                turn_count: 0,
                mode: "chat".to_string(),
                config: Some(config), // Store config for later
                cli_started: false,
            },
        );

        tracing::info!("Created session {} (CLI not yet started)", session_id);
        session_id
    }

    /// Send a prompt to a session (starts CLI if not yet started)
    ///
    /// For sessions created with `create_session`, this will start the CLI
    /// with the prompt. For sessions created with `start_session`, this is a no-op
    /// since single-shot CLI mode doesn't support follow-up prompts.
    pub async fn send_prompt(&self, session_id: Uuid, prompt: String) -> Result<(), String> {
        // Check if CLI needs to be started
        let needs_start = {
            let session = self.sessions.get(&session_id)
                .ok_or("Session not found")?;
            !session.cli_started
        };

        if needs_start {
            // Get the stored config and mark CLI as started
            let config = {
                let mut session = self.sessions.get_mut(&session_id)
                    .ok_or("Session not found")?;
                session.cli_started = true;
                session.is_active = true;
                session.config.take()
                    .ok_or("Session config not found - session may have been started directly")?
            };

            // Build config with the prompt
            let config_with_prompt = ChatSessionConfig {
                initial_prompt: prompt,
                ..config
            };

            // Start the backend session (this spawns Claude CLI)
            let handle = self.backend.start_session(config_with_prompt).await?;

            // The backend generates its own session_id, but we want to use our pre-generated one
            // We need to forward the stream to our session_id
            let publisher = self.publisher.clone();
            let sessions = self.sessions.clone();
            let mut stream_rx = handle.stream_rx;

            tokio::spawn(async move {
                tracing::info!("Started streaming task for session {}", session_id);

                while let Some(chunk) = stream_rx.recv().await {
                    tracing::info!("Received chunk for session {}: {:?}", session_id, chunk);

                    // Update turn count on turn complete
                    if matches!(chunk, StreamChunk::TurnComplete { .. }) {
                        if let Some(mut info) = sessions.get_mut(&session_id) {
                            info.turn_count += 1;
                        }
                    }

                    // Check for completion
                    let is_complete = matches!(chunk, StreamChunk::Complete { .. });

                    // Publish to ZMQ
                    tracing::info!("Publishing chunk for session {}", session_id);
                    if let Err(e) = publisher.publish(session_id, &chunk).await {
                        tracing::error!("Failed to publish chunk for session {}: {}", session_id, e);
                    }
                    tracing::info!("Published chunk successfully for session {}", session_id);

                    if is_complete {
                        if let Some(mut info) = sessions.get_mut(&session_id) {
                            info.is_active = false;
                        }
                        tracing::info!("Session {} completed", session_id);
                        break;
                    }
                }

                tracing::info!("Streaming task ended for session {}", session_id);
            });

            Ok(())
        } else {
            // CLI already running - try to send prompt (won't work in single-shot mode)
            self.backend.send_prompt(session_id, prompt).await
        }
    }

    /// Get session info
    pub fn get_session(&self, session_id: Uuid) -> Option<ChatSessionInfo> {
        self.sessions.get(&session_id).map(|tracker| ChatSessionInfo {
            session_id: session_id.to_string(),
            working_dir: tracker.working_dir.clone(),
            created_at: tracker.created_at,
            is_active: tracker.is_active,
            turn_count: tracker.turn_count,
            mode: tracker.mode.clone(),
        })
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<ChatSessionInfo> {
        self.sessions
            .iter()
            .map(|entry| {
                let session_id = *entry.key();
                let tracker = entry.value();
                ChatSessionInfo {
                    session_id: session_id.to_string(),
                    working_dir: tracker.working_dir.clone(),
                    created_at: tracker.created_at,
                    is_active: tracker.is_active,
                    turn_count: tracker.turn_count,
                    mode: tracker.mode.clone(),
                }
            })
            .collect()
    }

    /// Stop a session gracefully
    pub async fn stop_session(&self, session_id: Uuid) -> Result<(), String> {
        self.backend.stop_session(session_id).await?;
        if let Some(mut info) = self.sessions.get_mut(&session_id) {
            info.is_active = false;
        }
        Ok(())
    }

    /// Upgrade session to agent mode
    ///
    /// This marks the session as being in agent mode, which enables
    /// sub-agent spawning and more complex orchestration. The underlying
    /// CLI process is the same - the difference is in how the frontend
    /// consumes the output.
    pub fn upgrade_to_agent(&self, session_id: Uuid) -> Result<(), String> {
        if let Some(mut info) = self.sessions.get_mut(&session_id) {
            info.mode = "agent".to_string();
            Ok(())
        } else {
            Err("Session not found".to_string())
        }
    }

    /// Get the ZMQ PUB endpoint for clients to subscribe
    pub fn pub_endpoint(&self) -> String {
        self.publisher.client_endpoint()
    }

    /// Check if backend is available
    pub async fn is_backend_available(&self) -> bool {
        self.backend.is_available().await
    }

    /// Get backend version
    pub async fn backend_version(&self) -> Result<String, String> {
        self.backend.version().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Full integration tests require a running ZMQ publisher
    // These are unit tests for the data structures

    #[test]
    fn test_chat_session_info_serialization() {
        let info = ChatSessionInfo {
            session_id: Uuid::new_v4().to_string(),
            working_dir: "/tmp/test".to_string(),
            created_at: Utc::now(),
            is_active: true,
            turn_count: 5,
            mode: "chat".to_string(),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("session_id"));
        assert!(json.contains("working_dir"));
    }
}
