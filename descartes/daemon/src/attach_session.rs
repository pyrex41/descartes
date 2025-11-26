//! Attach session management for paused agents.
//!
//! This module provides session lifecycle management for external TUI clients
//! (Claude Code, OpenCode, etc.) attaching to paused agents.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────┐     ┌─────────────────┐     ┌─────────────────┐
//! │   Claude Code   │     │    OpenCode     │     │  Custom Client  │
//! │      TUI        │     │      TUI        │     │                 │
//! └────────┬────────┘     └────────┬────────┘     └────────┬────────┘
//!          │                       │                        │
//!          │ ZMQ DEALER           │ ZMQ DEALER             │ ZMQ DEALER
//!          ▼                       ▼                        ▼
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                     AttachSessionManager                           │
//! │  ┌─────────────────────────────────────────────────────────────┐  │
//! │  │                   Active Sessions                            │  │
//! │  │  session_id → AttachSession { agent_id, token, client, ... }│  │
//! │  └─────────────────────────────────────────────────────────────┘  │
//! │                              │                                     │
//! │  ┌─────────────────────────────────────────────────────────────┐  │
//! │  │                   AttachTokenStore                          │  │
//! │  │  token → AttachToken { agent_id, expires_at, ... }          │  │
//! │  └─────────────────────────────────────────────────────────────┘  │
//! └────────────────────────────────────────────────────────────────────┘
//!                              │
//!                              ▼
//! ┌────────────────────────────────────────────────────────────────────┐
//! │                        Agent Runner                                │
//! │          pause() / resume() / write_stdin() / read_stdout()       │
//! └────────────────────────────────────────────────────────────────────┘
//! ```

use chrono::{DateTime, Utc};
use descartes_core::{AttachToken, AttachTokenStore};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::errors::{DaemonError, DaemonResult};
use crate::events::{AgentEvent, AgentEventType, DescartesEvent, EventBus};

/// Client type for attach sessions
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClientType {
    /// Claude Code TUI
    ClaudeCode,
    /// OpenCode TUI
    OpenCode,
    /// Custom/unknown client
    Custom(String),
}

impl std::fmt::Display for ClientType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientType::ClaudeCode => write!(f, "claude-code"),
            ClientType::OpenCode => write!(f, "opencode"),
            ClientType::Custom(name) => write!(f, "custom:{}", name),
        }
    }
}

impl From<&str> for ClientType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "claude-code" | "claude_code" | "claudecode" => ClientType::ClaudeCode,
            "opencode" | "open-code" | "open_code" => ClientType::OpenCode,
            other => ClientType::Custom(other.to_string()),
        }
    }
}

/// An active attach session for a paused agent.
#[derive(Debug, Clone)]
pub struct AttachSession {
    /// Unique session identifier
    pub session_id: Uuid,
    /// The agent being attached to
    pub agent_id: Uuid,
    /// The token used for this session
    pub token: String,
    /// When the session was created
    pub connected_at: DateTime<Utc>,
    /// Type of client attached
    pub client_type: ClientType,
    /// Client version string
    pub client_version: String,
    /// ZMQ endpoint for this session
    pub zmq_endpoint: String,
    /// Total stdin bytes sent
    pub stdin_bytes: usize,
    /// Total stdout bytes received
    pub stdout_bytes: usize,
    /// Total stderr bytes received
    pub stderr_bytes: usize,
    /// Last activity timestamp
    pub last_activity: DateTime<Utc>,
}

impl AttachSession {
    /// Create a new attach session.
    pub fn new(
        agent_id: Uuid,
        token: String,
        client_type: ClientType,
        client_version: String,
        zmq_endpoint: String,
    ) -> Self {
        let now = Utc::now();
        Self {
            session_id: Uuid::new_v4(),
            agent_id,
            token,
            connected_at: now,
            client_type,
            client_version,
            zmq_endpoint,
            stdin_bytes: 0,
            stdout_bytes: 0,
            stderr_bytes: 0,
            last_activity: now,
        }
    }

    /// Update last activity timestamp.
    pub fn touch(&mut self) {
        self.last_activity = Utc::now();
    }

    /// Record stdin bytes.
    pub fn record_stdin(&mut self, bytes: usize) {
        self.stdin_bytes += bytes;
        self.touch();
    }

    /// Record stdout bytes.
    pub fn record_stdout(&mut self, bytes: usize) {
        self.stdout_bytes += bytes;
        self.touch();
    }

    /// Record stderr bytes.
    pub fn record_stderr(&mut self, bytes: usize) {
        self.stderr_bytes += bytes;
        self.touch();
    }

    /// Get session duration in seconds.
    pub fn duration_secs(&self) -> i64 {
        (Utc::now() - self.connected_at).num_seconds()
    }

    /// Convert to serializable info.
    pub fn to_info(&self) -> AttachSessionInfo {
        AttachSessionInfo {
            session_id: self.session_id.to_string(),
            agent_id: self.agent_id.to_string(),
            client_type: self.client_type.to_string(),
            client_version: self.client_version.clone(),
            connected_at: self.connected_at.to_rfc3339(),
            duration_secs: self.duration_secs(),
            stdin_bytes: self.stdin_bytes,
            stdout_bytes: self.stdout_bytes,
            stderr_bytes: self.stderr_bytes,
        }
    }
}

/// Serializable session info for RPC responses.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttachSessionInfo {
    pub session_id: String,
    pub agent_id: String,
    pub client_type: String,
    pub client_version: String,
    pub connected_at: String,
    pub duration_secs: i64,
    pub stdin_bytes: usize,
    pub stdout_bytes: usize,
    pub stderr_bytes: usize,
}

/// Configuration for the attach session manager.
#[derive(Debug, Clone)]
pub struct AttachSessionConfig {
    /// Default TTL for attach tokens in seconds
    pub token_ttl_secs: i64,
    /// Interval for token cleanup in seconds
    pub cleanup_interval_secs: u64,
    /// Maximum concurrent sessions per agent
    pub max_sessions_per_agent: usize,
    /// Base path for ZMQ IPC sockets
    pub zmq_socket_path: String,
}

impl Default for AttachSessionConfig {
    fn default() -> Self {
        Self {
            token_ttl_secs: 300, // 5 minutes
            cleanup_interval_secs: 60,
            max_sessions_per_agent: 1, // Single attach initially
            zmq_socket_path: "/tmp/descartes-attach".to_string(),
        }
    }
}

/// Manager for active attach sessions.
///
/// Coordinates token validation, session lifecycle, and event emission.
pub struct AttachSessionManager {
    /// Active sessions by session_id
    sessions: Arc<RwLock<HashMap<Uuid, AttachSession>>>,
    /// Sessions indexed by agent_id for quick lookup
    sessions_by_agent: Arc<RwLock<HashMap<Uuid, Vec<Uuid>>>>,
    /// Token store for validation
    token_store: Arc<AttachTokenStore>,
    /// Event bus for notifications
    event_bus: Arc<EventBus>,
    /// Configuration
    config: AttachSessionConfig,
}

impl AttachSessionManager {
    /// Create a new attach session manager.
    pub fn new(
        token_store: Arc<AttachTokenStore>,
        event_bus: Arc<EventBus>,
        config: AttachSessionConfig,
    ) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            sessions_by_agent: Arc::new(RwLock::new(HashMap::new())),
            token_store,
            event_bus,
            config,
        }
    }

    /// Create with default configuration.
    pub fn with_defaults(token_store: Arc<AttachTokenStore>, event_bus: Arc<EventBus>) -> Self {
        Self::new(token_store, event_bus, AttachSessionConfig::default())
    }

    /// Get the token store.
    pub fn token_store(&self) -> Arc<AttachTokenStore> {
        Arc::clone(&self.token_store)
    }

    /// Request attach credentials for a paused agent.
    ///
    /// Generates a token and returns connection info.
    pub async fn request_attach(
        &self,
        agent_id: Uuid,
        client_type: ClientType,
    ) -> DaemonResult<AttachCredentials> {
        // Check if we've exceeded max sessions for this agent
        let current_sessions = self.get_sessions_for_agent(&agent_id).await.len();
        if current_sessions >= self.config.max_sessions_per_agent {
            return Err(DaemonError::ResourceExhausted(format!(
                "Maximum concurrent sessions ({}) reached for agent {}",
                self.config.max_sessions_per_agent, agent_id
            )));
        }

        // Generate token
        let token = self.token_store.generate(agent_id).await;

        // Create ZMQ endpoint URL
        let zmq_endpoint = format!(
            "ipc://{}-{}.sock",
            self.config.zmq_socket_path, agent_id
        );

        // Emit event
        self.emit_attach_requested(&agent_id, &client_type).await;

        tracing::info!(
            agent_id = %agent_id,
            client_type = %client_type,
            expires_in_secs = token.remaining_secs(),
            "Attach credentials requested"
        );

        Ok(AttachCredentials {
            agent_id: agent_id.to_string(),
            token: token.token,
            connect_url: zmq_endpoint,
            expires_at: token.expires_at_unix(),
        })
    }

    /// Create a new attach session after successful handshake.
    pub async fn create_session(
        &self,
        agent_id: Uuid,
        token: String,
        client_type: ClientType,
        client_version: String,
    ) -> DaemonResult<AttachSession> {
        // Validate token
        let token_agent_id = self
            .token_store
            .validate(&token)
            .await
            .ok_or_else(|| DaemonError::AuthenticationFailed("Invalid or expired token".into()))?;

        if token_agent_id != agent_id {
            return Err(DaemonError::AuthenticationFailed(
                "Token does not match agent".into(),
            ));
        }

        // Create session
        let zmq_endpoint = format!(
            "ipc://{}-{}.sock",
            self.config.zmq_socket_path, agent_id
        );

        let session = AttachSession::new(
            agent_id,
            token.clone(),
            client_type.clone(),
            client_version,
            zmq_endpoint,
        );

        let session_id = session.session_id;

        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id, session.clone());
        }

        {
            let mut by_agent = self.sessions_by_agent.write().await;
            by_agent
                .entry(agent_id)
                .or_insert_with(Vec::new)
                .push(session_id);
        }

        // Emit event
        self.emit_attach_connected(&agent_id, &session_id, &client_type)
            .await;

        tracing::info!(
            session_id = %session_id,
            agent_id = %agent_id,
            client_type = %client_type,
            "Attach session created"
        );

        Ok(session)
    }

    /// Validate a token and get the associated agent_id.
    pub async fn validate_token(&self, token: &str) -> Option<Uuid> {
        self.token_store.validate(token).await
    }

    /// Get a session by ID.
    pub async fn get_session(&self, session_id: &Uuid) -> Option<AttachSession> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).cloned()
    }

    /// Get all sessions for an agent.
    pub async fn get_sessions_for_agent(&self, agent_id: &Uuid) -> Vec<AttachSession> {
        let session_ids: Vec<Uuid> = {
            let by_agent = self.sessions_by_agent.read().await;
            by_agent.get(agent_id).cloned().unwrap_or_default()
        };

        let sessions = self.sessions.read().await;
        session_ids
            .iter()
            .filter_map(|sid| sessions.get(sid).cloned())
            .collect()
    }

    /// Terminate a session.
    pub async fn terminate_session(&self, session_id: &Uuid) -> bool {
        let session = {
            let mut sessions = self.sessions.write().await;
            sessions.remove(session_id)
        };

        if let Some(session) = session {
            // Remove from agent index
            {
                let mut by_agent = self.sessions_by_agent.write().await;
                if let Some(session_list) = by_agent.get_mut(&session.agent_id) {
                    session_list.retain(|s| s != session_id);
                    if session_list.is_empty() {
                        by_agent.remove(&session.agent_id);
                    }
                }
            }

            // Revoke the token
            self.token_store.revoke(&session.token).await;

            // Emit event
            self.emit_attach_disconnected(
                &session.agent_id,
                session_id,
                &session.client_type,
                session.duration_secs(),
            )
            .await;

            tracing::info!(
                session_id = %session_id,
                agent_id = %session.agent_id,
                duration_secs = session.duration_secs(),
                "Attach session terminated"
            );

            true
        } else {
            false
        }
    }

    /// Terminate all sessions for an agent.
    pub async fn terminate_sessions_for_agent(&self, agent_id: &Uuid) -> usize {
        let session_ids: Vec<Uuid> = {
            let by_agent = self.sessions_by_agent.read().await;
            by_agent.get(agent_id).cloned().unwrap_or_default()
        };

        let mut count = 0;
        for session_id in session_ids {
            if self.terminate_session(&session_id).await {
                count += 1;
            }
        }

        count
    }

    /// Get count of active sessions.
    pub async fn active_session_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Get all active sessions as info.
    pub async fn list_sessions(&self) -> Vec<AttachSessionInfo> {
        self.sessions
            .read()
            .await
            .values()
            .map(|s| s.to_info())
            .collect()
    }

    /// Update session statistics (stdin/stdout bytes).
    pub async fn record_activity(
        &self,
        session_id: &Uuid,
        stdin_bytes: usize,
        stdout_bytes: usize,
        stderr_bytes: usize,
    ) {
        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(session_id) {
            if stdin_bytes > 0 {
                session.record_stdin(stdin_bytes);
            }
            if stdout_bytes > 0 {
                session.record_stdout(stdout_bytes);
            }
            if stderr_bytes > 0 {
                session.record_stderr(stderr_bytes);
            }
        }
    }

    // Event emission helpers

    async fn emit_attach_requested(&self, agent_id: &Uuid, client_type: &ClientType) {
        let event = AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            event_type: AgentEventType::AttachRequested,
            data: serde_json::json!({
                "client_type": client_type.to_string(),
            }),
        };
        self.event_bus.publish(DescartesEvent::AgentEvent(event));
    }

    async fn emit_attach_connected(
        &self,
        agent_id: &Uuid,
        session_id: &Uuid,
        client_type: &ClientType,
    ) {
        let event = AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            event_type: AgentEventType::AttachConnected,
            data: serde_json::json!({
                "session_id": session_id.to_string(),
                "client_type": client_type.to_string(),
            }),
        };
        self.event_bus.publish(DescartesEvent::AgentEvent(event));
    }

    async fn emit_attach_disconnected(
        &self,
        agent_id: &Uuid,
        session_id: &Uuid,
        client_type: &ClientType,
        duration_secs: i64,
    ) {
        let event = AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id: agent_id.to_string(),
            timestamp: Utc::now(),
            event_type: AgentEventType::AttachDisconnected,
            data: serde_json::json!({
                "session_id": session_id.to_string(),
                "client_type": client_type.to_string(),
                "duration_secs": duration_secs,
            }),
        };
        self.event_bus.publish(DescartesEvent::AgentEvent(event));
    }
}

/// Attach credentials returned to clients.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AttachCredentials {
    /// Agent ID
    pub agent_id: String,
    /// Authentication token
    pub token: String,
    /// ZMQ endpoint URL
    pub connect_url: String,
    /// Token expiration timestamp (Unix)
    pub expires_at: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_manager() -> AttachSessionManager {
        let token_store = Arc::new(AttachTokenStore::new());
        let event_bus = Arc::new(EventBus::new());
        AttachSessionManager::with_defaults(token_store, event_bus)
    }

    #[test]
    fn test_client_type_display() {
        assert_eq!(ClientType::ClaudeCode.to_string(), "claude-code");
        assert_eq!(ClientType::OpenCode.to_string(), "opencode");
        assert_eq!(
            ClientType::Custom("test".to_string()).to_string(),
            "custom:test"
        );
    }

    #[test]
    fn test_client_type_from_str() {
        assert_eq!(ClientType::from("claude-code"), ClientType::ClaudeCode);
        assert_eq!(ClientType::from("opencode"), ClientType::OpenCode);
        assert_eq!(
            ClientType::from("custom-client"),
            ClientType::Custom("custom-client".to_string())
        );
    }

    #[test]
    fn test_attach_session_creation() {
        let session = AttachSession::new(
            Uuid::new_v4(),
            "token123".to_string(),
            ClientType::ClaudeCode,
            "1.0.0".to_string(),
            "ipc:///tmp/test.sock".to_string(),
        );

        assert_eq!(session.stdin_bytes, 0);
        assert_eq!(session.stdout_bytes, 0);
        assert!(session.duration_secs() >= 0);
    }

    #[test]
    fn test_attach_session_recording() {
        let mut session = AttachSession::new(
            Uuid::new_v4(),
            "token123".to_string(),
            ClientType::ClaudeCode,
            "1.0.0".to_string(),
            "ipc:///tmp/test.sock".to_string(),
        );

        session.record_stdin(100);
        session.record_stdout(200);
        session.record_stderr(50);

        assert_eq!(session.stdin_bytes, 100);
        assert_eq!(session.stdout_bytes, 200);
        assert_eq!(session.stderr_bytes, 50);
    }

    #[tokio::test]
    async fn test_request_attach() {
        let manager = create_test_manager();
        let agent_id = Uuid::new_v4();

        let creds = manager
            .request_attach(agent_id, ClientType::ClaudeCode)
            .await
            .unwrap();

        assert_eq!(creds.agent_id, agent_id.to_string());
        assert!(!creds.token.is_empty());
        assert!(creds.connect_url.contains(&agent_id.to_string()));
        assert!(creds.expires_at > 0);
    }

    #[tokio::test]
    async fn test_create_session_with_valid_token() {
        let manager = create_test_manager();
        let agent_id = Uuid::new_v4();

        // Request attach to get a valid token
        let creds = manager
            .request_attach(agent_id, ClientType::ClaudeCode)
            .await
            .unwrap();

        // Create session with the token
        let session = manager
            .create_session(
                agent_id,
                creds.token,
                ClientType::ClaudeCode,
                "1.0.0".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(session.agent_id, agent_id);
        assert_eq!(session.client_type, ClientType::ClaudeCode);
    }

    #[tokio::test]
    async fn test_create_session_with_invalid_token() {
        let manager = create_test_manager();
        let agent_id = Uuid::new_v4();

        let result = manager
            .create_session(
                agent_id,
                "invalid-token".to_string(),
                ClientType::ClaudeCode,
                "1.0.0".to_string(),
            )
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_terminate_session() {
        let manager = create_test_manager();
        let agent_id = Uuid::new_v4();

        let creds = manager
            .request_attach(agent_id, ClientType::ClaudeCode)
            .await
            .unwrap();

        let session = manager
            .create_session(
                agent_id,
                creds.token,
                ClientType::ClaudeCode,
                "1.0.0".to_string(),
            )
            .await
            .unwrap();

        let session_id = session.session_id;

        // Session should exist
        assert!(manager.get_session(&session_id).await.is_some());

        // Terminate
        let terminated = manager.terminate_session(&session_id).await;
        assert!(terminated);

        // Session should not exist
        assert!(manager.get_session(&session_id).await.is_none());
    }

    #[tokio::test]
    async fn test_get_sessions_for_agent() {
        let manager = create_test_manager();
        let agent_id = Uuid::new_v4();

        // Create multiple sessions (need to increase max_sessions_per_agent for this test)
        let token_store = manager.token_store();
        let token = token_store.generate(agent_id).await;

        // Manually create a session since we have single-session limit
        let session = AttachSession::new(
            agent_id,
            token.token,
            ClientType::ClaudeCode,
            "1.0.0".to_string(),
            "ipc:///tmp/test.sock".to_string(),
        );

        {
            let mut sessions = manager.sessions.write().await;
            sessions.insert(session.session_id, session.clone());
        }
        {
            let mut by_agent = manager.sessions_by_agent.write().await;
            by_agent
                .entry(agent_id)
                .or_insert_with(Vec::new)
                .push(session.session_id);
        }

        let sessions = manager.get_sessions_for_agent(&agent_id).await;
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].agent_id, agent_id);
    }

    #[tokio::test]
    async fn test_record_activity() {
        let manager = create_test_manager();
        let agent_id = Uuid::new_v4();

        let creds = manager
            .request_attach(agent_id, ClientType::ClaudeCode)
            .await
            .unwrap();

        let session = manager
            .create_session(
                agent_id,
                creds.token,
                ClientType::ClaudeCode,
                "1.0.0".to_string(),
            )
            .await
            .unwrap();

        let session_id = session.session_id;

        manager.record_activity(&session_id, 100, 200, 50).await;

        let updated = manager.get_session(&session_id).await.unwrap();
        assert_eq!(updated.stdin_bytes, 100);
        assert_eq!(updated.stdout_bytes, 200);
        assert_eq!(updated.stderr_bytes, 50);
    }
}
