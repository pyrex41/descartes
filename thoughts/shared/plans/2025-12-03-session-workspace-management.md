# Session/Workspace Management Implementation Plan

## Overview

Add session/workspace management to the Descartes GUI, enabling users to:
1. Discover and browse workspaces in a base directory
2. Navigate between sessions
3. Create new workspaces
4. Archive/delete workspaces
5. Connect to the appropriate daemon for each session

Also fix the `.taskmaster` → `.scud` directory migration in the Rust code.

## Current State Analysis

### What Exists Now:
- GUI connects to **hardcoded** `http://127.0.0.1:8080` (`gui/src/rpc_client.rs:34-37`)
- **No session concept** in GUI - just one daemon connection
- `descartes init` creates workspace structure with `data/`, `thoughts/`, `logs/`, etc.
- SCG task storage is project-aware but still references `.taskmaster/` (needs migration to `.scud/`)
- Daemon config has `storage.base_path` setting (defaults to `~/.descartes`)

### Key Discoveries:
- `daemon/src/scg_task_event_emitter.rs` still uses `.taskmaster/` paths (lines 3, 120, 137, 180-181, 402-411)
- `cli/src/commands/tasks.rs` references `.taskmaster/` in error message (line 93)
- `.scud/` is already gitignored and used by the SCUD command files

## Desired End State

After this implementation:
1. **GUI has a session selector** in the header/sidebar to switch workspaces
2. **Sessions panel** shows all discovered workspaces with status
3. **Each workspace runs its own daemon** (or daemon supports workspace switching)
4. **Create/archive workflows** from the GUI
5. **All `.taskmaster` references migrated to `.scud`**

### Verification:
- User can see list of workspaces in a configured base directory
- User can switch between workspaces and GUI updates to show correct data
- User can create new workspace from GUI
- User can archive workspace from GUI
- `.scud/` directory is used everywhere (not `.taskmaster`)

## What We're NOT Doing

- Docker/cloud deployment (future phase)
- Micro-VM per agent (future phase)
- Remote daemon management
- Multi-user authentication
- Workspace sharing/collaboration

## Implementation Approach

**Architecture Decision: Daemon per Session**

Each workspace/session will have its own daemon instance. The GUI will:
1. Discover workspaces by scanning a base directory for `.scud/` or `config.toml`
2. Manage daemon lifecycle (start/stop) for each workspace
3. Connect to the active workspace's daemon
4. Persist session state (last active, recently used, etc.)

This approach is cleaner for future cloud deployment where each workspace would run in isolation.

---

## Phase 1: Fix `.taskmaster` → `.scud` Migration

### Overview
Update all Rust code to use `.scud/` instead of `.taskmaster/`.

### Changes Required:

#### 1.1 Daemon Task Event Emitter

**File**: `descartes/daemon/src/scg_task_event_emitter.rs`
**Changes**: Replace all `.taskmaster` references with `.scud`

```rust
// Line 3: Update doc comment
//! This module watches the .scud/tasks/tasks.json file for changes and emits

// Line 120: Update path
let tasks_file = project_root.join(".scud/tasks/tasks.json");

// Line 137: Update path
let tasks_dir = project_root.join(".scud/tasks");

// Lines 180-181: Update path
// Watch the .scud directory instead and wait for tasks/ to be created
let scud_dir = project_root.join(".scud");

// Lines 402-403: Update test path
// Create .scud directory structure
let scud_dir = temp_dir.path().join(".scud/tasks");

// Line 411: Update test path
let workflow_file = temp_dir.path().join(".scud/workflow-state.json");
```

#### 1.2 CLI Tasks Command

**File**: `descartes/cli/src/commands/tasks.rs`
**Changes**: Update error message

```rust
// Line 93: Update message
"Run 'scud init' or ensure .scud/ directory exists.".dimmed()
```

#### 1.3 Core Task Storage (Comment Only)

**File**: `descartes/core/src/scg_task_storage.rs`
**Changes**: Update doc comment

```rust
// Line 43: Update comment
/// Creates .scud/ directory with necessary files
```

### Success Criteria:

#### Automated Verification:
- [ ] Code compiles: `cargo build --workspace`
- [ ] Tests pass: `cargo test --workspace`
- [ ] No references to `.taskmaster` in Rust code: `rg "\.taskmaster" descartes/`

#### Manual Verification:
- [ ] Daemon correctly watches `.scud/tasks/tasks.json` for changes
- [ ] Task events emit when tasks are modified in `.scud/`

**Implementation Note**: Complete Phase 1 before proceeding. This is a prerequisite for session management.

---

## Phase 2: Core Session/Workspace Types

### Overview
Define the core data types and traits for session/workspace management.

### Changes Required:

#### 2.1 Session Types

**File**: `descartes/core/src/session.rs` (NEW)
**Changes**: Define session-related types

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Represents a Descartes workspace/session
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
    /// Daemon connection info (if running)
    pub daemon_info: Option<DaemonInfo>,
}

/// Session lifecycle status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SessionStatus {
    /// Session exists but daemon not running
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
```

#### 2.2 Session Manager Trait

**File**: `descartes/core/src/session.rs` (continued)
**Changes**: Define session management interface

```rust
use async_trait::async_trait;

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

    /// Archive a session (stop daemon, mark as archived)
    async fn archive_session(&self, id: &Uuid) -> Result<(), SessionError>;

    /// Delete a session permanently
    async fn delete_session(&self, id: &Uuid, delete_files: bool) -> Result<(), SessionError>;

    /// Start daemon for a session
    async fn start_daemon(&self, id: &Uuid) -> Result<DaemonInfo, SessionError>;

    /// Stop daemon for a session
    async fn stop_daemon(&self, id: &Uuid) -> Result<(), SessionError>;

    /// Get the currently active session
    async fn get_active_session(&self) -> Result<Option<Session>, SessionError>;

    /// Set the active session
    async fn set_active_session(&self, id: &Uuid) -> Result<(), SessionError>;
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

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Config error: {0}")]
    ConfigError(String),
}
```

#### 2.3 Update Core lib.rs

**File**: `descartes/core/src/lib.rs`
**Changes**: Export session module

```rust
// Add to existing exports
mod session;
pub use session::*;
```

### Success Criteria:

#### Automated Verification:
- [ ] Core crate compiles: `cargo build -p descartes-core`
- [ ] Core tests pass: `cargo test -p descartes-core`

#### Manual Verification:
- [ ] Types are correctly exported from descartes-core

---

## Phase 3: Session Discovery and Management Implementation

### Overview
Implement the session manager with filesystem scanning and daemon lifecycle management.

### Changes Required:

#### 3.1 Filesystem Session Manager

**File**: `descartes/core/src/session_manager.rs` (NEW)
**Changes**: Implement SessionManager trait

```rust
use crate::session::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, info, warn, error};

/// Filesystem-based session manager
pub struct FileSystemSessionManager {
    config: SessionDiscoveryConfig,
    /// Cached sessions by ID
    sessions: Arc<RwLock<HashMap<Uuid, Session>>>,
    /// Currently active session ID
    active_session_id: Arc<RwLock<Option<Uuid>>>,
    /// Path to persist session metadata
    metadata_path: PathBuf,
}

impl FileSystemSessionManager {
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
        // Check for .scud/ directory or config.toml
        path.join(".scud").exists() || path.join("config.toml").exists()
    }

    /// Scan a directory for workspaces
    fn scan_directory(&self, path: &Path, depth: usize) -> Vec<PathBuf> {
        let mut workspaces = Vec::new();

        if depth > self.config.max_depth {
            return workspaces;
        }

        if self.is_workspace(path) {
            workspaces.push(path.to_path_buf());
        }

        if self.config.recursive && path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let entry_path = entry.path();
                    if entry_path.is_dir()
                       && !entry_path.file_name().map(|n| n.to_str().unwrap_or("").starts_with('.')).unwrap_or(false)
                    {
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
                if let Ok(session) = serde_json::from_str::<Session>(&content) {
                    return Some(session);
                }
            }
        }

        // Create session metadata from directory info
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        let created_at = path.metadata()
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
}

#[async_trait]
impl SessionManager for FileSystemSessionManager {
    async fn discover_sessions(&self) -> Result<Vec<Session>, SessionError> {
        let mut sessions = Vec::new();

        for search_path in &self.config.search_paths {
            if search_path.exists() {
                let workspace_paths = self.scan_directory(search_path, 0);
                for path in workspace_paths {
                    if let Some(session) = self.load_session_from_path(&path) {
                        sessions.push(session);
                    }
                }
            }
        }

        // Update cache
        let mut cache = self.sessions.write().await;
        for session in &sessions {
            cache.insert(session.id, session.clone());
        }

        Ok(sessions)
    }

    async fn create_session(&self, name: &str, path: &Path) -> Result<Session, SessionError> {
        if path.exists() && self.is_workspace(path) {
            return Err(SessionError::AlreadyExists(path.to_path_buf()));
        }

        // Create directory structure
        std::fs::create_dir_all(path)?;
        std::fs::create_dir_all(path.join(".scud/tasks"))?;
        std::fs::create_dir_all(path.join("data"))?;
        std::fs::create_dir_all(path.join("thoughts"))?;
        std::fs::create_dir_all(path.join("logs"))?;

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
        let session_file = path.join(".scud/session.json");
        let content = serde_json::to_string_pretty(&session)
            .map_err(|e| SessionError::ConfigError(e.to_string()))?;
        std::fs::write(&session_file, content)?;

        // Update cache
        let mut cache = self.sessions.write().await;
        cache.insert(session.id, session.clone());

        Ok(session)
    }

    // ... implement remaining trait methods
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Core crate compiles: `cargo build -p descartes-core`
- [ ] Session discovery tests pass: `cargo test -p descartes-core session`

#### Manual Verification:
- [ ] Sessions are correctly discovered from filesystem
- [ ] New sessions can be created

---

## Phase 4: GUI Session State and Components

### Overview
Add session state to the GUI and create session management components.

### Changes Required:

#### 4.1 Session State in GUI

**File**: `descartes/gui/src/session_state.rs` (NEW)
**Changes**: Define GUI session state

```rust
use descartes_core::{Session, SessionStatus, SessionManager, FileSystemSessionManager};
use std::sync::Arc;

/// State for session management in the GUI
#[derive(Debug, Clone)]
pub struct SessionState {
    /// All discovered sessions
    pub sessions: Vec<Session>,
    /// Currently active session ID
    pub active_session_id: Option<uuid::Uuid>,
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
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            sessions: Vec::new(),
            active_session_id: None,
            new_session_name: String::new(),
            new_session_path: String::new(),
            loading: false,
            error: None,
            show_create_dialog: false,
        }
    }
}

/// Messages for session management
#[derive(Debug, Clone)]
pub enum SessionMessage {
    /// Refresh/discover sessions
    RefreshSessions,
    /// Sessions were loaded
    SessionsLoaded(Vec<Session>),
    /// Select a session
    SelectSession(uuid::Uuid),
    /// Session was activated
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
    ArchiveSession(uuid::Uuid),
    /// Session was archived
    SessionArchived(uuid::Uuid),
    /// Delete a session
    DeleteSession(uuid::Uuid),
    /// Error occurred
    Error(String),
}
```

#### 4.2 Session Selector Component

**File**: `descartes/gui/src/session_selector.rs` (NEW)
**Changes**: Create session dropdown/selector widget

```rust
use crate::session_state::{SessionMessage, SessionState};
use crate::theme::{colors, button_styles, container_styles};
use descartes_core::{Session, SessionStatus};
use iced::widget::{button, column, container, pick_list, row, scrollable, text, Space};
use iced::{Element, Length};

/// Render the session selector dropdown in the header
pub fn view_session_selector(state: &SessionState) -> Element<SessionMessage> {
    let active_session = state.active_session_id
        .and_then(|id| state.sessions.iter().find(|s| s.id == id));

    let session_name = active_session
        .map(|s| s.name.clone())
        .unwrap_or_else(|| "No Session".to_string());

    let status_indicator = active_session
        .map(|s| match s.status {
            SessionStatus::Active => ("●", colors::SUCCESS),
            SessionStatus::Starting => ("◐", colors::WARNING),
            SessionStatus::Inactive => ("○", colors::TEXT_MUTED),
            SessionStatus::Error => ("●", colors::ERROR),
            _ => ("○", colors::TEXT_MUTED),
        })
        .unwrap_or(("○", colors::TEXT_MUTED));

    let selector_content = row![
        text(status_indicator.0).size(10).color(status_indicator.1),
        Space::with_width(6),
        text(&session_name).size(13).color(colors::TEXT_PRIMARY),
        Space::with_width(8),
        text("▼").size(10).color(colors::TEXT_MUTED),
    ]
    .align_y(iced::alignment::Vertical::Center);

    // For now, use a button that refreshes sessions
    // TODO: Implement proper dropdown with session list
    let selector = button(selector_content)
        .on_press(SessionMessage::RefreshSessions)
        .padding([6, 12])
        .style(button_styles::secondary);

    container(selector).into()
}

/// Render the full sessions panel/view
pub fn view_sessions_panel(state: &SessionState) -> Element<SessionMessage> {
    let header = row![
        text("Sessions").size(24).color(colors::TEXT_PRIMARY),
        Space::with_width(Length::Fill),
        button(text("+ New").size(13))
            .on_press(SessionMessage::ShowCreateDialog)
            .padding([8, 16])
            .style(button_styles::primary),
        Space::with_width(8),
        button(text("Refresh").size(13))
            .on_press(SessionMessage::RefreshSessions)
            .padding([8, 16])
            .style(button_styles::secondary),
    ]
    .align_y(iced::alignment::Vertical::Center);

    let session_list: Vec<Element<SessionMessage>> = state.sessions
        .iter()
        .map(|session| view_session_card(session, state.active_session_id == Some(session.id)))
        .collect();

    let content = if state.loading {
        column![
            header,
            Space::with_height(20),
            text("Loading sessions...").color(colors::TEXT_MUTED),
        ]
    } else if session_list.is_empty() {
        column![
            header,
            Space::with_height(20),
            text("No sessions found").color(colors::TEXT_MUTED),
            text("Click '+ New' to create a workspace").size(12).color(colors::TEXT_MUTED),
        ]
    } else {
        column![
            header,
            Space::with_height(20),
            scrollable(column(session_list).spacing(8)),
        ]
    };

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(20)
        .into()
}

/// Render a single session card
fn view_session_card(session: &Session, is_active: bool) -> Element<SessionMessage> {
    let status_color = match session.status {
        SessionStatus::Active => colors::SUCCESS,
        SessionStatus::Starting => colors::WARNING,
        SessionStatus::Inactive => colors::TEXT_MUTED,
        SessionStatus::Error => colors::ERROR,
        SessionStatus::Archived => colors::TEXT_MUTED,
        SessionStatus::Stopping => colors::WARNING,
    };

    let status_text = match session.status {
        SessionStatus::Active => "Running",
        SessionStatus::Starting => "Starting",
        SessionStatus::Inactive => "Stopped",
        SessionStatus::Error => "Error",
        SessionStatus::Archived => "Archived",
        SessionStatus::Stopping => "Stopping",
    };

    let path_str = session.path.display().to_string();
    let truncated_path = if path_str.len() > 50 {
        format!("...{}", &path_str[path_str.len()-47..])
    } else {
        path_str
    };

    let card_content = column![
        row![
            text("●").size(10).color(status_color),
            Space::with_width(8),
            text(&session.name).size(16).color(colors::TEXT_PRIMARY),
            Space::with_width(Length::Fill),
            text(status_text).size(12).color(status_color),
        ]
        .align_y(iced::alignment::Vertical::Center),
        Space::with_height(4),
        text(&truncated_path).size(11).color(colors::TEXT_MUTED),
    ]
    .spacing(2);

    let session_id = session.id;
    let card = button(card_content)
        .on_press(SessionMessage::SelectSession(session_id))
        .width(Length::Fill)
        .padding(12)
        .style(if is_active {
            button_styles::nav_active
        } else {
            button_styles::secondary
        });

    container(card).width(Length::Fill).into()
}
```

#### 4.3 Integrate into Main GUI

**File**: `descartes/gui/src/main.rs`
**Changes**: Add session state and navigation

```rust
// Add imports
mod session_state;
mod session_selector;

use session_state::{SessionMessage, SessionState};

// Add to ViewMode enum
enum ViewMode {
    Sessions,  // New view
    Dashboard,
    TaskBoard,
    // ... existing modes
}

// Add to DescartesGui struct
struct DescartesGui {
    session_state: SessionState,
    // ... existing fields
}

// Add to Message enum
enum Message {
    Session(SessionMessage),
    // ... existing messages
}

// Add view_sessions method
fn view_sessions(&self) -> Element<Message> {
    session_selector::view_sessions_panel(&self.session_state)
        .map(Message::Session)
}

// Add session selector to header
fn view_header(&self) -> Element<Message> {
    // ... existing header code
    // Add session selector between brand and status
    let session_selector = session_selector::view_session_selector(&self.session_state)
        .map(Message::Session);

    // Insert into header layout
}

// Add to navigation items
let nav_items = vec![
    (ViewMode::Sessions, "◈", "Sessions"),  // New nav item
    (ViewMode::Dashboard, "⌂", "Dashboard"),
    // ... existing items
];
```

### Success Criteria:

#### Automated Verification:
- [ ] GUI compiles: `cargo build -p descartes-gui`
- [ ] No new warnings in GUI code

#### Manual Verification:
- [ ] Sessions view appears in navigation
- [ ] Session selector appears in header
- [ ] Session cards display correctly

---

## Phase 5: Session Lifecycle and Daemon Management

### Overview
Implement daemon start/stop and session switching with proper connection handling.

### Changes Required:

#### 5.1 Daemon Launcher

**File**: `descartes/core/src/daemon_launcher.rs` (NEW)
**Changes**: Handle spawning and managing daemon processes

```rust
use std::path::Path;
use std::process::{Child, Command, Stdio};
use tokio::time::{sleep, Duration};
use tracing::{info, warn, error};

use crate::session::{DaemonInfo, SessionError};

/// Manages daemon process lifecycle
pub struct DaemonLauncher {
    /// Base port for daemon HTTP endpoints
    base_http_port: u16,
    /// Base port for daemon WebSocket endpoints
    base_ws_port: u16,
}

impl DaemonLauncher {
    pub fn new() -> Self {
        Self {
            base_http_port: 8080,
            base_ws_port: 8081,
        }
    }

    /// Start a daemon for a workspace
    pub async fn start_daemon(&self, workspace_path: &Path, port_offset: u16) -> Result<DaemonInfo, SessionError> {
        let http_port = self.base_http_port + port_offset;
        let ws_port = self.base_ws_port + port_offset;

        let config_path = workspace_path.join("config.toml");

        // Build daemon command
        let mut cmd = Command::new("descartes-daemon");
        cmd.arg("--http-port").arg(http_port.to_string())
           .arg("--ws-port").arg(ws_port.to_string())
           .arg("--workdir").arg(workspace_path)
           .stdout(Stdio::piped())
           .stderr(Stdio::piped());

        if config_path.exists() {
            cmd.arg("--config").arg(&config_path);
        }

        let child = cmd.spawn()
            .map_err(|e| SessionError::DaemonStartFailed(e.to_string()))?;

        let pid = child.id();

        // Wait for daemon to be ready
        let endpoint = format!("http://127.0.0.1:{}", http_port);
        let mut attempts = 0;
        while attempts < 30 {
            if self.check_daemon_health(&endpoint).await {
                return Ok(DaemonInfo {
                    pid: Some(pid),
                    http_endpoint: endpoint,
                    ws_endpoint: Some(format!("ws://127.0.0.1:{}", ws_port)),
                    started_at: chrono::Utc::now(),
                });
            }
            sleep(Duration::from_millis(100)).await;
            attempts += 1;
        }

        Err(SessionError::DaemonStartFailed("Daemon failed to become healthy".to_string()))
    }

    /// Check if daemon is responding
    async fn check_daemon_health(&self, endpoint: &str) -> bool {
        // Simple HTTP health check
        match reqwest::get(&format!("{}/health", endpoint)).await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }

    /// Stop a daemon by PID
    pub async fn stop_daemon(&self, pid: u32) -> Result<(), SessionError> {
        #[cfg(unix)]
        {
            use nix::sys::signal::{kill, Signal};
            use nix::unistd::Pid;

            kill(Pid::from_raw(pid as i32), Signal::SIGTERM)
                .map_err(|e| SessionError::DaemonStopFailed(e.to_string()))?;
        }

        #[cfg(windows)]
        {
            // Windows implementation would use TerminateProcess
            return Err(SessionError::DaemonStopFailed("Windows not yet supported".to_string()));
        }

        Ok(())
    }
}
```

#### 5.2 Session Switching in GUI

**File**: `descartes/gui/src/main.rs`
**Changes**: Handle session switching and reconnection

```rust
// In update() function, handle Session messages
Message::Session(session_msg) => {
    match session_msg {
        SessionMessage::SelectSession(id) => {
            // Disconnect from current daemon if connected
            if self.daemon_connected {
                // Trigger disconnect
                self.update(Message::DisconnectDaemon);
            }

            // Find the selected session
            if let Some(session) = self.session_state.sessions.iter().find(|s| s.id == id) {
                self.session_state.active_session_id = Some(id);

                // If session has daemon info, connect to it
                if let Some(daemon_info) = &session.daemon_info {
                    // Update RPC client endpoint and reconnect
                    // This requires modifying GuiRpcClient to support endpoint changes
                }
            }

            iced::Task::none()
        }
        // ... handle other session messages
    }
}
```

### Success Criteria:

#### Automated Verification:
- [ ] All crates compile: `cargo build --workspace`
- [ ] Integration tests pass: `cargo test --workspace`

#### Manual Verification:
- [ ] Daemon starts when session is activated
- [ ] GUI connects to the correct daemon
- [ ] Switching sessions reconnects to different daemon
- [ ] Daemon stops when session is deactivated

---

## Testing Strategy

### Unit Tests:
- Session discovery scans directories correctly
- Session creation creates proper directory structure
- Session status transitions are valid

### Integration Tests:
- Full session lifecycle: create → activate → use → deactivate → archive
- Multiple sessions can run concurrently
- GUI correctly reflects session state changes

### Manual Testing Steps:
1. Launch GUI and verify Sessions view is accessible
2. Click "Refresh" and verify workspaces are discovered
3. Click "+ New" and create a new session
4. Verify `.scud/` directory is created with proper structure
5. Select a session and verify daemon starts
6. Verify task board shows tasks from the selected session
7. Switch to a different session and verify data changes
8. Archive a session and verify it's marked as archived

## Performance Considerations

- Session discovery should be async and not block the UI
- Cache discovered sessions to avoid repeated filesystem scans
- Daemon health checks should have reasonable timeouts
- Consider lazy loading session details

## Migration Notes

- Existing `.taskmaster/` directories should be automatically migrated to `.scud/`
- Session metadata (`.scud/session.json`) will be created on first access
- Users with existing workspaces should see them discovered automatically

## References

- Current RPC client: `descartes/gui/src/rpc_client.rs`
- Daemon configuration: `descartes/daemon/src/config.rs`
- SCG task storage: `descartes/core/src/scg_task_storage.rs`
- Init command: `descartes/cli/src/commands/init.rs`
