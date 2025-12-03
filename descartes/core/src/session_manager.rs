//! Filesystem-based Session Manager Implementation
//!
//! This module implements the SessionManager trait using the filesystem for
//! session discovery and persistence.

use crate::session::{
    DaemonInfo, Session, SessionDiscoveryConfig, SessionError, SessionManager, SessionStatus,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn};
use uuid::Uuid;

/// Filesystem-based session manager
///
/// Discovers sessions by scanning configured directories for `.scud/` directories
/// or `config.toml` files. Manages session metadata and daemon lifecycle.
pub struct FileSystemSessionManager {
    /// Configuration for session discovery
    config: SessionDiscoveryConfig,
    /// Cached sessions by ID
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    /// Currently active session ID
    active_session_id: Arc<RwLock<Option<Uuid>>>,
    /// Path to persist global session metadata
    metadata_path: PathBuf,
    /// Base port for daemon HTTP endpoints
    base_http_port: u16,
    /// Base port for daemon WebSocket endpoints
    base_ws_port: u16,
    /// Port offset counter for multiple daemons
    port_offset: Arc<RwLock<u16>>,
}

impl FileSystemSessionManager {
    /// Create a new filesystem session manager
    pub fn new(config: SessionDiscoveryConfig, metadata_path: PathBuf) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            active_session_id: Arc::new(RwLock::new(None)),
            metadata_path,
            base_http_port: 8080,
            base_ws_port: 8081,
            port_offset: Arc::new(RwLock::new(0)),
        }
    }

    /// Create with custom port configuration
    pub fn with_ports(mut self, http_port: u16, ws_port: u16) -> Self {
        self.base_http_port = http_port;
        self.base_ws_port = ws_port;
        self
    }

    /// Check if a directory is a valid Descartes workspace
    fn is_workspace(&self, path: &Path) -> bool {
        // Check for .scud/ directory or config.toml
        path.join(".scud").exists() || path.join("config.toml").exists()
    }

    /// Scan a directory for workspaces
    fn scan_directory(&self, path: &Path, depth: usize) -> Vec<PathBuf> {
        let mut workspaces = Vec::new();

        if depth > self.config.max_depth {
            return workspaces;
        }

        // Check if this directory is a workspace
        if self.is_workspace(path) {
            workspaces.push(path.to_path_buf());
            // Don't scan inside a workspace for nested workspaces
            return workspaces;
        }

        // Recursively scan subdirectories
        if self.config.recursive && path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();
                    // Skip hidden directories (except we check .scud inside is_workspace)
                    let is_hidden = entry_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with('.'))
                        .unwrap_or(false);

                    if entry_path.is_dir() && !is_hidden {
                        workspaces.extend(self.scan_directory(&entry_path, depth + 1));
                    }
                }
            }
        }

        workspaces
    }

    /// Load session metadata from a workspace directory
    fn load_session_from_path(&self, path: &Path) -> Option<Session> {
        // Try to load from .scud/session.json if it exists
        let session_file = path.join(".scud/session.json");
        if session_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&session_file) {
                if let Ok(mut session) = serde_json::from_str::<Session>(&content) {
                    // Update path in case it was moved
                    session.path = path.to_path_buf();
                    return Some(session);
                }
            }
        }

        // Create session metadata from directory info
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let created_at = path
            .metadata()
            .and_then(|m| m.created())
            .map(|t| DateTime::<Utc>::from(t))
            .unwrap_or_else(|_| Utc::now());

        Some(Session {
            id: Uuid::new_v4(),
            name,
            path: path.to_path_buf(),
            created_at,
            last_accessed: None,
            status: SessionStatus::Inactive,
            daemon_info: None,
        })
    }

    /// Save session metadata to disk
    fn save_session_metadata(&self, session: &Session) -> Result<(), SessionError> {
        let scud_dir = session.scud_path();
        if !scud_dir.exists() {
            std::fs::create_dir_all(&scud_dir)?;
        }

        let session_file = session.metadata_path();
        let content = serde_json::to_string_pretty(session)
            .map_err(|e| SessionError::SerializationError(e.to_string()))?;
        std::fs::write(&session_file, content)?;

        debug!("Saved session metadata to {:?}", session_file);
        Ok(())
    }

    /// Load global session state (active session, etc.)
    async fn load_global_state(&self) -> Result<(), SessionError> {
        if self.metadata_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&self.metadata_path) {
                if let Ok(state) = serde_json::from_str::<GlobalSessionState>(&content) {
                    let mut active = self.active_session_id.write().await;
                    *active = state.active_session_id;
                }
            }
        }
        Ok(())
    }

    /// Save global session state
    async fn save_global_state(&self) -> Result<(), SessionError> {
        let active = self.active_session_id.read().await;
        let state = GlobalSessionState {
            active_session_id: *active,
        };

        if let Some(parent) = self.metadata_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let content = serde_json::to_string_pretty(&state)
            .map_err(|e| SessionError::SerializationError(e.to_string()))?;
        std::fs::write(&self.metadata_path, content)?;

        Ok(())
    }

    /// Get the next available port offset
    async fn get_next_port_offset(&self) -> u16 {
        let mut offset = self.port_offset.write().await;
        let current = *offset;
        *offset += 1;
        current
    }

    /// Check if a daemon is healthy
    async fn check_daemon_health(&self, endpoint: &str) -> bool {
        match reqwest::get(&format!("{}/health", endpoint)).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}

/// Global session state persisted across runs
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GlobalSessionState {
    active_session_id: Option<Uuid>,
}

#[async_trait]
impl SessionManager for FileSystemSessionManager {
    async fn discover_sessions(&self) -> Result<Vec<Session>, SessionError> {
        let mut sessions = Vec::new();

        for search_path in &self.config.search_paths {
            if search_path.exists() {
                debug!("Scanning directory for workspaces: {:?}", search_path);
                let workspace_paths = self.scan_directory(search_path, 0);

                for path in workspace_paths {
                    if let Some(session) = self.load_session_from_path(&path) {
                        info!("Discovered session: {} at {:?}", session.name, session.path);
                        sessions.push(session);
                    }
                }
            } else {
                debug!("Search path does not exist: {:?}", search_path);
            }
        }

        // Update cache
        let mut cache = self.sessions.write().await;
        for session in &sessions {
            cache.insert(session.id, session.clone());
        }

        // Load global state to restore active session
        let _ = self.load_global_state().await;

        Ok(sessions)
    }

    async fn get_session(&self, id: &Uuid) -> Result<Option<Session>, SessionError> {
        let cache = self.sessions.read().await;
        Ok(cache.get(id).cloned())
    }

    async fn get_session_by_path(&self, path: &Path) -> Result<Option<Session>, SessionError> {
        let cache = self.sessions.read().await;
        Ok(cache.values().find(|s| s.path == path).cloned())
    }

    async fn create_session(&self, name: &str, path: &Path) -> Result<Session, SessionError> {
        // Check if path already exists as a workspace
        if path.exists() && self.is_workspace(path) {
            return Err(SessionError::AlreadyExists(path.to_path_buf()));
        }

        // Create directory structure
        std::fs::create_dir_all(path)?;
        std::fs::create_dir_all(path.join(".scud/tasks"))?;
        std::fs::create_dir_all(path.join("data"))?;
        std::fs::create_dir_all(path.join("thoughts"))?;
        std::fs::create_dir_all(path.join("logs"))?;

        // Create empty tasks.json
        let tasks_file = path.join(".scud/tasks/tasks.json");
        std::fs::write(&tasks_file, "{}")?;

        // Create workflow-state.json
        let workflow_file = path.join(".scud/workflow-state.json");
        std::fs::write(
            &workflow_file,
            r#"{"active_epic": null, "updated_at": null}"#,
        )?;

        let session = Session {
            id: Uuid::new_v4(),
            name: name.to_string(),
            path: path.to_path_buf(),
            created_at: Utc::now(),
            last_accessed: Some(Utc::now()),
            status: SessionStatus::Inactive,
            daemon_info: None,
        };

        // Save session metadata
        self.save_session_metadata(&session)?;

        // Update cache
        let mut cache = self.sessions.write().await;
        cache.insert(session.id, session.clone());

        info!("Created new session: {} at {:?}", name, path);
        Ok(session)
    }

    async fn archive_session(&self, id: &Uuid) -> Result<(), SessionError> {
        // Stop daemon if running
        let _ = self.stop_daemon(id).await;

        // Update session status
        let mut cache = self.sessions.write().await;
        if let Some(session) = cache.get_mut(id) {
            session.status = SessionStatus::Archived;
            session.daemon_info = None;
            self.save_session_metadata(session)?;
            info!("Archived session: {}", session.name);
            Ok(())
        } else {
            Err(SessionError::NotFound(*id))
        }
    }

    async fn delete_session(&self, id: &Uuid, delete_files: bool) -> Result<(), SessionError> {
        // Stop daemon first
        let _ = self.stop_daemon(id).await;

        let mut cache = self.sessions.write().await;
        if let Some(session) = cache.remove(id) {
            if delete_files {
                warn!(
                    "Deleting session files at {:?} (this is permanent!)",
                    session.path
                );
                std::fs::remove_dir_all(&session.path)?;
            } else {
                // Just remove the session.json to "unregister" it
                let metadata_path = session.metadata_path();
                if metadata_path.exists() {
                    std::fs::remove_file(&metadata_path)?;
                }
            }
            info!("Deleted session: {}", session.name);
            Ok(())
        } else {
            Err(SessionError::NotFound(*id))
        }
    }

    async fn start_daemon(&self, id: &Uuid) -> Result<DaemonInfo, SessionError> {
        let mut cache = self.sessions.write().await;
        let session = cache
            .get_mut(id)
            .ok_or_else(|| SessionError::NotFound(*id))?;

        // Check if already running
        if let Some(ref daemon_info) = session.daemon_info {
            if self.check_daemon_health(&daemon_info.http_endpoint).await {
                return Ok(daemon_info.clone());
            }
        }

        // Get next port offset
        let port_offset = self.get_next_port_offset().await;
        let http_port = self.base_http_port + port_offset;
        let ws_port = self.base_ws_port + port_offset;

        // Update session status
        session.status = SessionStatus::Starting;
        drop(cache); // Release lock before spawning

        // Spawn daemon process
        let workspace_path = {
            let cache = self.sessions.read().await;
            cache.get(id).unwrap().path.clone()
        };

        let result = self
            .spawn_daemon(&workspace_path, http_port, ws_port)
            .await;

        // Update session based on result
        let mut cache = self.sessions.write().await;
        if let Some(session) = cache.get_mut(id) {
            match result {
                Ok(daemon_info) => {
                    session.status = SessionStatus::Active;
                    session.daemon_info = Some(daemon_info.clone());
                    session.last_accessed = Some(Utc::now());
                    self.save_session_metadata(session)?;
                    Ok(daemon_info)
                }
                Err(e) => {
                    session.status = SessionStatus::Error;
                    self.save_session_metadata(session)?;
                    Err(e)
                }
            }
        } else {
            Err(SessionError::NotFound(*id))
        }
    }

    async fn stop_daemon(&self, id: &Uuid) -> Result<(), SessionError> {
        let mut cache = self.sessions.write().await;
        let session = cache
            .get_mut(id)
            .ok_or_else(|| SessionError::NotFound(*id))?;

        if let Some(ref daemon_info) = session.daemon_info {
            session.status = SessionStatus::Stopping;

            // Try graceful shutdown via HTTP
            if let Err(e) = self.graceful_shutdown(&daemon_info.http_endpoint).await {
                warn!("Graceful shutdown failed: {}, trying kill", e);

                // Fall back to kill if we have PID
                if let Some(pid) = daemon_info.pid {
                    self.kill_daemon(pid)?;
                }
            }

            session.status = SessionStatus::Inactive;
            session.daemon_info = None;
            self.save_session_metadata(session)?;
            info!("Stopped daemon for session: {}", session.name);
            Ok(())
        } else {
            Err(SessionError::DaemonNotRunning(*id))
        }
    }

    async fn get_active_session(&self) -> Result<Option<Session>, SessionError> {
        let active_id = self.active_session_id.read().await;
        if let Some(id) = *active_id {
            self.get_session(&id).await
        } else {
            Ok(None)
        }
    }

    async fn set_active_session(&self, id: &Uuid) -> Result<(), SessionError> {
        // Verify session exists
        let cache = self.sessions.read().await;
        if !cache.contains_key(id) {
            return Err(SessionError::NotFound(*id));
        }
        drop(cache);

        // Update active session
        let mut active = self.active_session_id.write().await;
        *active = Some(*id);
        drop(active);

        // Persist
        self.save_global_state().await?;

        // Update last accessed
        let mut cache = self.sessions.write().await;
        if let Some(session) = cache.get_mut(id) {
            session.last_accessed = Some(Utc::now());
            let _ = self.save_session_metadata(session);
        }

        Ok(())
    }

    async fn refresh_session(&self, id: &Uuid) -> Result<Option<Session>, SessionError> {
        let mut cache = self.sessions.write().await;
        if let Some(session) = cache.get_mut(id) {
            // Check if daemon is still running
            if let Some(ref daemon_info) = session.daemon_info {
                if !self.check_daemon_health(&daemon_info.http_endpoint).await {
                    session.status = SessionStatus::Inactive;
                    session.daemon_info = None;
                }
            }
            Ok(Some(session.clone()))
        } else {
            Ok(None)
        }
    }
}

impl FileSystemSessionManager {
    /// Spawn a daemon process for a workspace
    async fn spawn_daemon(
        &self,
        workspace_path: &Path,
        http_port: u16,
        ws_port: u16,
    ) -> Result<DaemonInfo, SessionError> {
        use std::process::{Command, Stdio};
        use tokio::time::{sleep, Duration};

        let config_path = workspace_path.join("config.toml");

        // Build daemon command
        let mut cmd = Command::new("descartes-daemon");
        cmd.arg("--http-port")
            .arg(http_port.to_string())
            .arg("--ws-port")
            .arg(ws_port.to_string())
            .arg("--workdir")
            .arg(workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if config_path.exists() {
            cmd.arg("--config").arg(&config_path);
        }

        let child = cmd
            .spawn()
            .map_err(|e| SessionError::DaemonStartFailed(e.to_string()))?;

        let pid = child.id();

        // Wait for daemon to be ready
        let endpoint = format!("http://127.0.0.1:{}", http_port);
        let mut attempts = 0;
        while attempts < 30 {
            if self.check_daemon_health(&endpoint).await {
                info!("Daemon started on port {} (PID: {})", http_port, pid);
                return Ok(DaemonInfo {
                    pid: Some(pid),
                    http_endpoint: endpoint,
                    ws_endpoint: Some(format!("ws://127.0.0.1:{}", ws_port)),
                    started_at: Utc::now(),
                });
            }
            sleep(Duration::from_millis(100)).await;
            attempts += 1;
        }

        Err(SessionError::DaemonStartFailed(
            "Daemon failed to become healthy within timeout".to_string(),
        ))
    }

    /// Send graceful shutdown request to daemon
    async fn graceful_shutdown(&self, endpoint: &str) -> Result<(), SessionError> {
        let client = reqwest::Client::new();
        client
            .post(&format!("{}/shutdown", endpoint))
            .send()
            .await
            .map_err(|e| SessionError::DaemonStopFailed(e.to_string()))?;
        Ok(())
    }

    /// Kill daemon by PID
    fn kill_daemon(&self, pid: u32) -> Result<(), SessionError> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
                .map_err(|e| SessionError::DaemonStopFailed(e.to_string()))?;
        }

        #[cfg(not(unix))]
        {
            return Err(SessionError::DaemonStopFailed(
                "Process termination not yet supported on this platform".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_manager() -> (FileSystemSessionManager, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = SessionDiscoveryConfig::with_path(temp_dir.path());
        let metadata_path = temp_dir.path().join("sessions.json");
        let manager = FileSystemSessionManager::new(config, metadata_path);
        (manager, temp_dir)
    }

    #[tokio::test]
    async fn test_create_session() {
        let (manager, temp_dir) = create_test_manager();
        let session_path = temp_dir.path().join("test-project");

        let session = manager
            .create_session("Test Project", &session_path)
            .await
            .expect("Failed to create session");

        assert_eq!(session.name, "Test Project");
        assert_eq!(session.status, SessionStatus::Inactive);
        assert!(session_path.join(".scud").exists());
        assert!(session_path.join(".scud/tasks/tasks.json").exists());
    }

    #[tokio::test]
    async fn test_discover_sessions() {
        let (manager, temp_dir) = create_test_manager();

        // Create a couple of sessions
        let session1_path = temp_dir.path().join("project1");
        let session2_path = temp_dir.path().join("project2");

        manager
            .create_session("Project 1", &session1_path)
            .await
            .expect("Failed to create session 1");
        manager
            .create_session("Project 2", &session2_path)
            .await
            .expect("Failed to create session 2");

        // Clear cache
        {
            let mut cache = manager.sessions.write().await;
            cache.clear();
        }

        // Discover
        let sessions = manager
            .discover_sessions()
            .await
            .expect("Failed to discover sessions");

        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_archive_session() {
        let (manager, temp_dir) = create_test_manager();
        let session_path = temp_dir.path().join("archive-test");

        let session = manager
            .create_session("Archive Test", &session_path)
            .await
            .expect("Failed to create session");

        manager
            .archive_session(&session.id)
            .await
            .expect("Failed to archive session");

        let archived = manager
            .get_session(&session.id)
            .await
            .expect("Failed to get session")
            .expect("Session should exist");

        assert_eq!(archived.status, SessionStatus::Archived);
    }

    #[tokio::test]
    async fn test_active_session() {
        let (manager, temp_dir) = create_test_manager();
        let session_path = temp_dir.path().join("active-test");

        let session = manager
            .create_session("Active Test", &session_path)
            .await
            .expect("Failed to create session");

        manager
            .set_active_session(&session.id)
            .await
            .expect("Failed to set active session");

        let active = manager
            .get_active_session()
            .await
            .expect("Failed to get active session")
            .expect("Should have active session");

        assert_eq!(active.id, session.id);
    }

    #[test]
    fn test_is_workspace() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = SessionDiscoveryConfig::default();
        let manager =
            FileSystemSessionManager::new(config, temp_dir.path().join("sessions.json"));

        // Not a workspace initially
        assert!(!manager.is_workspace(temp_dir.path()));

        // Create .scud directory
        std::fs::create_dir_all(temp_dir.path().join(".scud")).expect("Failed to create .scud");

        // Now it's a workspace
        assert!(manager.is_workspace(temp_dir.path()));
    }
}
