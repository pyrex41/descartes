//! Session state and messages for the GUI
//!
//! This module defines the session-related state and message types
//! for managing workspaces/sessions in the Descartes GUI.

use descartes_core::{Session, SessionStatus};
use uuid::Uuid;

/// State for session management in the GUI
#[derive(Debug, Clone, Default)]
pub struct SessionState {
    /// All discovered sessions
    pub sessions: Vec<Session>,
    /// Currently active session ID
    pub active_session_id: Option<Uuid>,
    /// Session being created (name input)
    pub new_session_name: String,
    /// Session being created (path input)
    pub new_session_path: String,
    /// Loading state
    pub loading: bool,
    /// Error message
    pub error: Option<String>,
    /// Show create session dialog
    pub show_create_dialog: bool,
    /// Filter for session list
    pub filter: SessionFilter,
}

/// Filter options for the session list
#[derive(Debug, Clone, Default)]
pub struct SessionFilter {
    /// Search text
    pub search: String,
    /// Include archived sessions
    pub include_archived: bool,
}

impl SessionState {
    /// Get the currently active session
    pub fn active_session(&self) -> Option<&Session> {
        self.active_session_id
            .and_then(|id| self.sessions.iter().find(|s| s.id == id))
    }

    /// Get all non-archived sessions
    pub fn visible_sessions(&self) -> Vec<&Session> {
        self.sessions
            .iter()
            .filter(|s| {
                // Filter by archived status
                if !self.filter.include_archived && s.status == SessionStatus::Archived {
                    return false;
                }

                // Filter by search text
                if !self.filter.search.is_empty() {
                    let search_lower = self.filter.search.to_lowercase();
                    if !s.name.to_lowercase().contains(&search_lower)
                        && !s.path.to_string_lossy().to_lowercase().contains(&search_lower)
                    {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Count sessions by status
    pub fn status_counts(&self) -> (usize, usize, usize) {
        let active = self
            .sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Active)
            .count();
        let inactive = self
            .sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Inactive)
            .count();
        let archived = self
            .sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Archived)
            .count();
        (active, inactive, archived)
    }
}

/// Messages for session management
#[derive(Debug, Clone)]
pub enum SessionMessage {
    /// Refresh/discover sessions
    RefreshSessions,
    /// Sessions were loaded
    SessionsLoaded(Vec<Session>),
    /// Select a session (switch to it)
    SelectSession(Uuid),
    /// Session was activated (daemon started, connected)
    SessionActivated(Session),
    /// Start creating a new session
    ShowCreateDialog,
    /// Hide create dialog
    HideCreateDialog,
    /// Update new session name
    UpdateNewSessionName(String),
    /// Update new session path
    UpdateNewSessionPath(String),
    /// Create the new session
    CreateSession,
    /// Session was created
    SessionCreated(Session),
    /// Archive a session
    ArchiveSession(Uuid),
    /// Session was archived
    SessionArchived(Uuid),
    /// Delete a session
    DeleteSession(Uuid),
    /// Session was deleted
    SessionDeleted(Uuid),
    /// Start daemon for a session
    StartDaemon(Uuid),
    /// Stop daemon for a session
    StopDaemon(Uuid),
    /// Update search filter
    UpdateSearch(String),
    /// Toggle include archived
    ToggleIncludeArchived,
    /// Error occurred
    Error(String),
    /// Clear error
    ClearError,
}

/// Update session state based on messages
pub fn update(state: &mut SessionState, message: SessionMessage) {
    match message {
        SessionMessage::RefreshSessions => {
            state.loading = true;
            state.error = None;
        }
        SessionMessage::SessionsLoaded(sessions) => {
            state.sessions = sessions;
            state.loading = false;
        }
        SessionMessage::SelectSession(id) => {
            state.active_session_id = Some(id);
        }
        SessionMessage::SessionActivated(session) => {
            // Update session in list
            if let Some(existing) = state.sessions.iter_mut().find(|s| s.id == session.id) {
                *existing = session;
            }
        }
        SessionMessage::ShowCreateDialog => {
            state.show_create_dialog = true;
            state.new_session_name = String::new();
            state.new_session_path = String::new();
        }
        SessionMessage::HideCreateDialog => {
            state.show_create_dialog = false;
        }
        SessionMessage::UpdateNewSessionName(name) => {
            state.new_session_name = name;
        }
        SessionMessage::UpdateNewSessionPath(path) => {
            state.new_session_path = path;
        }
        SessionMessage::CreateSession => {
            state.loading = true;
        }
        SessionMessage::SessionCreated(session) => {
            state.sessions.push(session.clone());
            state.active_session_id = Some(session.id);
            state.show_create_dialog = false;
            state.loading = false;
        }
        SessionMessage::ArchiveSession(_id) => {
            state.loading = true;
        }
        SessionMessage::SessionArchived(id) => {
            if let Some(session) = state.sessions.iter_mut().find(|s| s.id == id) {
                session.status = SessionStatus::Archived;
            }
            state.loading = false;
        }
        SessionMessage::DeleteSession(_id) => {
            state.loading = true;
        }
        SessionMessage::SessionDeleted(id) => {
            state.sessions.retain(|s| s.id != id);
            if state.active_session_id == Some(id) {
                state.active_session_id = None;
            }
            state.loading = false;
        }
        SessionMessage::StartDaemon(_id) => {
            state.loading = true;
        }
        SessionMessage::StopDaemon(_id) => {
            state.loading = true;
        }
        SessionMessage::UpdateSearch(search) => {
            state.filter.search = search;
        }
        SessionMessage::ToggleIncludeArchived => {
            state.filter.include_archived = !state.filter.include_archived;
        }
        SessionMessage::Error(err) => {
            state.error = Some(err);
            state.loading = false;
        }
        SessionMessage::ClearError => {
            state.error = None;
        }
    }
}
