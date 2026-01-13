//! Session/Workspace Management for Descartes
//!
//! This module provides types and traits for managing Descartes workspaces/sessions.
//! Each session represents a workspace with its own daemon instance, configuration,
//! and task storage.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

/// Represents a Descartes workspace/session
///
/// Note: The daemon is now global (not per-session), but `daemon_info` is kept
/// for backwards compatibility when deserializing old session.json files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Unique session ID
    pub id: Uuid,
    /// Human-readable name
    pub name: String,
    /// Path to the workspace root directory
    pub path: PathBuf,
    /// When the session was created
    pub created_at: DateTime<Utc>,
    /// When the session was last accessed
    pub last_accessed: Option<DateTime<Utc>>,
    /// Session status
    pub status: SessionStatus,
    /// Daemon connection info (deprecated - kept for backwards compatibility)
    /// The daemon is now global; use `daemon_launcher` module for connection info.
    #[serde(skip_serializing, default)]
    pub daemon_info: Option<DaemonInfo>,
}

impl Session {
    /// Create a new session with default values
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            id: Uuid::new_v4(),
            name,
            path,
            created_at: Utc::now(),
            last_accessed: Some(Utc::now()),
            status: SessionStatus::Inactive,
            daemon_info: None,
        }
    }

    /// Check if the session is active
    ///
    /// Note: This now only checks session status, not daemon status.
    /// The daemon is global and managed separately via `daemon_launcher`.
    pub fn is_running(&self) -> bool {
        matches!(self.status, SessionStatus::Active)
    }

    /// Check if the session is archived
    pub fn is_archived(&self) -> bool {
        matches!(self.status, SessionStatus::Archived)
    }

    /// Get the path to the .descartes directory (primary session storage)
    pub fn descartes_path(&self) -> PathBuf {
        self.path.join(".descartes")
    }

    /// Get the path to the .scud directory (for SCUD CLI plugin compatibility)
    pub fn scud_path(&self) -> PathBuf {
        self.path.join(".scud")
    }

    /// Get the path to the session metadata file
    pub fn metadata_path(&self) -> PathBuf {
        self.descartes_path().join("session.json")
    }
}

/// Session lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SessionStatus {
    /// Session exists but daemon not running
    #[default]
    Inactive,
    /// Daemon is starting up
    Starting,
    /// Daemon is running and connected
    Active,
    /// Daemon is stopping
    Stopping,
    /// Session has been archived
    Archived,
    /// Session has errors
    Error,
}

impl std::fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionStatus::Inactive => write!(f, "Inactive"),
            SessionStatus::Starting => write!(f, "Starting"),
            SessionStatus::Active => write!(f, "Active"),
            SessionStatus::Stopping => write!(f, "Stopping"),
            SessionStatus::Archived => write!(f, "Archived"),
            SessionStatus::Error => write!(f, "Error"),
        }
    }
}

/// Information about a running daemon
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonInfo {
    /// Process ID of the daemon
    pub pid: Option<u32>,
    /// HTTP endpoint for RPC
    pub http_endpoint: String,
    /// WebSocket endpoint for events
    pub ws_endpoint: Option<String>,
    /// When the daemon was started
    pub started_at: DateTime<Utc>,
}

impl DaemonInfo {
    /// Create new daemon info with the given endpoints
    pub fn new(http_endpoint: String, ws_endpoint: Option<String>) -> Self {
        Self {
            pid: None,
            http_endpoint,
            ws_endpoint,
            started_at: Utc::now(),
        }
    }

    /// Create daemon info with a PID
    pub fn with_pid(mut self, pid: u32) -> Self {
        self.pid = Some(pid);
        self
    }
}

/// Configuration for session discovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDiscoveryConfig {
    /// Base directories to scan for workspaces
    pub search_paths: Vec<PathBuf>,
    /// Whether to scan subdirectories
    pub recursive: bool,
    /// Max depth for recursive search
    pub max_depth: usize,
}

impl Default for SessionDiscoveryConfig {
    fn default() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        Self {
            search_paths: vec![
                PathBuf::from(&home).join("projects"),
                PathBuf::from(&home).join(".descartes"),
            ],
            recursive: true,
            max_depth: 3,
        }
    }
}

impl SessionDiscoveryConfig {
    /// Create a config with a single search path
    pub fn with_path<P: Into<PathBuf>>(path: P) -> Self {
        Self {
            search_paths: vec![path.into()],
            recursive: true,
            max_depth: 3,
        }
    }

    /// Add a search path
    pub fn add_path<P: Into<PathBuf>>(&mut self, path: P) {
        self.search_paths.push(path.into());
    }

    /// Set the max depth for recursive search
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Disable recursive search
    pub fn non_recursive(mut self) -> Self {
        self.recursive = false;
        self
    }
}

/// Trait for session management operations
#[async_trait]
pub trait SessionManager: Send + Sync {
    /// Discover all sessions in configured search paths
    async fn discover_sessions(&self) -> Result<Vec<Session>, SessionError>;

    /// Get a session by ID
    async fn get_session(&self, id: &Uuid) -> Result<Option<Session>, SessionError>;

    /// Get a session by path
    async fn get_session_by_path(&self, path: &Path) -> Result<Option<Session>, SessionError>;

    /// Create a new session/workspace
    async fn create_session(&self, name: &str, path: &Path) -> Result<Session, SessionError>;

    /// Archive a session (mark as archived)
    async fn archive_session(&self, id: &Uuid) -> Result<(), SessionError>;

    /// Delete a session permanently
    async fn delete_session(&self, id: &Uuid, delete_files: bool) -> Result<(), SessionError>;

    /// Get the currently active session
    async fn get_active_session(&self) -> Result<Option<Session>, SessionError>;

    /// Set the active session
    async fn set_active_session(&self, id: &Uuid) -> Result<(), SessionError>;

    /// Refresh a single session's status
    async fn refresh_session(&self, id: &Uuid) -> Result<Option<Session>, SessionError>;
}

/// Session-related errors
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    #[error("Session not found: {0}")]
    NotFound(Uuid),

    #[error("Session already exists at path: {0}")]
    AlreadyExists(PathBuf),

    #[error("Failed to start daemon: {0}")]
    DaemonStartFailed(String),

    #[error("Failed to stop daemon: {0}")]
    DaemonStopFailed(String),

    #[error("Daemon not running for session: {0}")]
    DaemonNotRunning(Uuid),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Invalid session path: {0}")]
    InvalidPath(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let session = Session::new("test-project".to_string(), PathBuf::from("/tmp/test"));

        assert_eq!(session.name, "test-project");
        assert_eq!(session.path, PathBuf::from("/tmp/test"));
        assert_eq!(session.status, SessionStatus::Inactive);
        assert!(session.daemon_info.is_none());
        assert!(!session.is_running());
        assert!(!session.is_archived());
    }

    #[test]
    fn test_session_paths() {
        let session = Session::new("test".to_string(), PathBuf::from("/home/user/project"));

        assert_eq!(session.descartes_path(), PathBuf::from("/home/user/project/.descartes"));
        assert_eq!(session.scud_path(), PathBuf::from("/home/user/project/.scud"));
        assert_eq!(session.metadata_path(), PathBuf::from("/home/user/project/.descartes/session.json"));
    }

    #[test]
    fn test_session_status_display() {
        assert_eq!(SessionStatus::Active.to_string(), "Active");
        assert_eq!(SessionStatus::Inactive.to_string(), "Inactive");
        assert_eq!(SessionStatus::Archived.to_string(), "Archived");
    }

    #[test]
    fn test_daemon_info_creation() {
        let info = DaemonInfo::new("http://127.0.0.1:8080".to_string(), Some("ws://127.0.0.1:8081".to_string()))
            .with_pid(12345);

        assert_eq!(info.pid, Some(12345));
        assert_eq!(info.http_endpoint, "http://127.0.0.1:8080");
        assert_eq!(info.ws_endpoint, Some("ws://127.0.0.1:8081".to_string()));
    }

    #[test]
    fn test_discovery_config_default() {
        let config = SessionDiscoveryConfig::default();

        assert!(config.recursive);
        assert_eq!(config.max_depth, 3);
        assert!(!config.search_paths.is_empty());
    }

    #[test]
    fn test_discovery_config_builder() {
        let config = SessionDiscoveryConfig::with_path("/custom/path")
            .with_max_depth(5)
            .non_recursive();

        assert!(!config.recursive);
        assert_eq!(config.max_depth, 5);
        assert_eq!(config.search_paths.len(), 1);
        assert_eq!(config.search_paths[0], PathBuf::from("/custom/path"));
    }
}
