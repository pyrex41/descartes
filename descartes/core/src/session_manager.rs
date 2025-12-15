//! Filesystem-based Session Manager Implementation
//!
//! This module implements the SessionManager trait using the filesystem for
//! session discovery and persistence.

use crate::session::{
    Session, SessionDiscoveryConfig, SessionError, SessionManager, SessionStatus,
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
/// Discovers sessions by scanning configured directories for `.descartes/` or `.scud/`
/// directories. Manages session metadata persistence.
///
/// Note: The daemon is now global and managed separately via `daemon_launcher`.
/// Sessions no longer spawn their own daemon instances.
pub struct FileSystemSessionManager {
    /// Configuration for session discovery
    config: SessionDiscoveryConfig,
    /// Cached sessions by ID
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    /// Currently active session ID
    active_session_id: Arc<RwLock<Option<Uuid>>>,
    /// Path to persist global session metadata
    metadata_path: PathBuf,
}

impl FileSystemSessionManager {
    /// Create a new filesystem session manager
    pub fn new(config: SessionDiscoveryConfig, metadata_path: PathBuf) -> Self {
        Self {
            config,
            sessions: Arc::new(RwLock::new(HashMap::new())),
            active_session_id: Arc::new(RwLock::new(None)),
            metadata_path,
        }
    }

    /// Check if a directory is a valid Descartes workspace
    fn is_workspace(&self, path: &Path) -> bool {
        // Check for .descartes/ directory (new format), .scud/ directory (backward compat),
        // or config.toml
        path.join(".descartes").exists()
            || path.join(".scud").exists()
            || path.join("config.toml").exists()
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
        // Try to load from .descartes/session.json first (new format)
        let descartes_session_file = path.join(".descartes/session.json");
        if descartes_session_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&descartes_session_file) {
                if let Ok(mut session) = serde_json::from_str::<Session>(&content) {
                    // Update path in case it was moved
                    session.path = path.to_path_buf();
                    return Some(session);
                }
            }
        }

        // Fall back to .scud/session.json for backward compatibility
        let scud_session_file = path.join(".scud/session.json");
        if scud_session_file.exists() {
            if let Ok(content) = std::fs::read_to_string(&scud_session_file) {
                if let Ok(mut session) = serde_json::from_str::<Session>(&content) {
                    // Update path in case it was moved
                    session.path = path.to_path_buf();
                    return Some(session);
                }
            }
        }

        // Create session metadata from directory info if workspace exists but no metadata
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let created_at = path
            .metadata()
            .and_then(|m| m.created())
            .map(DateTime::<Utc>::from)
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
        let descartes_dir = session.descartes_path();
        if !descartes_dir.exists() {
            std::fs::create_dir_all(&descartes_dir)?;
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

        // Create directory structure using .descartes (new format)
        std::fs::create_dir_all(path)?;
        std::fs::create_dir_all(path.join(".descartes/sessions"))?;
        std::fs::create_dir_all(path.join("data"))?;
        std::fs::create_dir_all(path.join("thoughts"))?;
        std::fs::create_dir_all(path.join("logs"))?;

        // Create empty sessions.json for session tracking
        let sessions_file = path.join(".descartes/sessions/sessions.json");
        std::fs::write(&sessions_file, "[]")?;

        // Create workflow-state.json in .descartes
        let workflow_file = path.join(".descartes/workflow-state.json");
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
        let cache = self.sessions.read().await;
        Ok(cache.get(id).cloned())
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
        assert!(session_path.join(".descartes").exists());
        assert!(session_path.join(".descartes/sessions/sessions.json").exists());
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

        // Create .descartes directory (new format)
        std::fs::create_dir_all(temp_dir.path().join(".descartes")).expect("Failed to create .descartes");

        // Now it's a workspace
        assert!(manager.is_workspace(temp_dir.path()));
    }

    #[test]
    fn test_is_workspace_backward_compat() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let config = SessionDiscoveryConfig::default();
        let manager =
            FileSystemSessionManager::new(config, temp_dir.path().join("sessions.json"));

        // Not a workspace initially
        assert!(!manager.is_workspace(temp_dir.path()));

        // Create .scud directory (old format - should still work for backward compatibility)
        std::fs::create_dir_all(temp_dir.path().join(".scud")).expect("Failed to create .scud");

        // Should be detected as a workspace for backward compatibility
        assert!(manager.is_workspace(temp_dir.path()));
    }
}
