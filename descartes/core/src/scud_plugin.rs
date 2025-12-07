//! SCUD CLI Plugin Integration
//!
//! Allows the external SCUD CLI tool to work alongside Descartes.
//! SCUD manages its own state in `.scud/` while Descartes uses `.descartes/`.
//!
//! This module provides utilities for:
//! - Detecting if SCUD CLI is available
//! - Reading/writing SCUD-compatible files
//! - Syncing tasks between Descartes and SCUD

use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if SCUD CLI is available in PATH
pub fn scud_available() -> bool {
    Command::new("scud")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Get the SCUD directory for a workspace
pub fn scud_dir(workspace: &Path) -> PathBuf {
    workspace.join(".scud")
}

/// Check if a workspace has SCUD initialized
pub fn has_scud(workspace: &Path) -> bool {
    scud_dir(workspace).exists()
}

/// Get the SCUD tasks file path
pub fn scud_tasks_file(workspace: &Path) -> PathBuf {
    scud_dir(workspace).join("tasks").join("tasks.json")
}

/// Get the SCUD workflow state file path
pub fn scud_workflow_state_file(workspace: &Path) -> PathBuf {
    scud_dir(workspace).join("workflow-state.json")
}

/// Sync Descartes tasks to SCUD (write-only plugin)
///
/// This writes tasks in SCUD-compatible JSON format to the SCUD tasks file.
/// This is a one-way sync - Descartes writes, SCUD reads.
pub fn sync_tasks_to_scud(workspace: &Path, tasks_json: &str) -> std::io::Result<()> {
    let scud_tasks = scud_tasks_file(workspace);
    if let Some(parent) = scud_tasks.parent() {
        if parent.exists() {
            std::fs::write(scud_tasks, tasks_json)?;
        }
    }
    Ok(())
}

/// Read SCUD workflow state (read-only)
///
/// Returns the workflow state JSON if it exists, None otherwise.
pub fn read_scud_workflow_state(workspace: &Path) -> Option<String> {
    let state_file = scud_workflow_state_file(workspace);
    std::fs::read_to_string(state_file).ok()
}

/// Read SCUD tasks (read-only)
///
/// Returns the tasks JSON if it exists, None otherwise.
pub fn read_scud_tasks(workspace: &Path) -> Option<String> {
    let tasks_file = scud_tasks_file(workspace);
    std::fs::read_to_string(tasks_file).ok()
}

/// Create a SCUD directory structure if it doesn't exist
///
/// This is useful when Descartes needs to write to SCUD-compatible paths
/// but SCUD hasn't been initialized yet.
pub fn ensure_scud_dir(workspace: &Path) -> std::io::Result<()> {
    let scud = scud_dir(workspace);
    std::fs::create_dir_all(scud.join("tasks"))?;
    Ok(())
}

/// Get SCUD CLI version if available
pub fn scud_version() -> Option<String> {
    Command::new("scud")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}

/// Check if both Descartes and SCUD are initialized in a workspace
pub fn is_dual_workspace(workspace: &Path) -> bool {
    let has_descartes = workspace.join(".descartes").exists();
    let has_scud = has_scud(workspace);
    has_descartes && has_scud
}

/// Workspace type detection
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceType {
    /// Only Descartes (.descartes/) is present
    DescartesOnly,
    /// Only SCUD (.scud/) is present
    ScudOnly,
    /// Both are present
    DualWorkspace,
    /// Neither is present
    NotInitialized,
}

impl std::fmt::Display for WorkspaceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkspaceType::DescartesOnly => write!(f, "Descartes"),
            WorkspaceType::ScudOnly => write!(f, "SCUD"),
            WorkspaceType::DualWorkspace => write!(f, "Descartes+SCUD"),
            WorkspaceType::NotInitialized => write!(f, "Not Initialized"),
        }
    }
}

/// Detect the workspace type for a given path
pub fn detect_workspace_type(workspace: &Path) -> WorkspaceType {
    let has_descartes = workspace.join(".descartes").exists();
    let has_scud = has_scud(workspace);

    match (has_descartes, has_scud) {
        (true, true) => WorkspaceType::DualWorkspace,
        (true, false) => WorkspaceType::DescartesOnly,
        (false, true) => WorkspaceType::ScudOnly,
        (false, false) => WorkspaceType::NotInitialized,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_scud_dir() {
        let workspace = PathBuf::from("/home/user/project");
        assert_eq!(scud_dir(&workspace), PathBuf::from("/home/user/project/.scud"));
    }

    #[test]
    fn test_has_scud() {
        let temp = TempDir::new().unwrap();
        assert!(!has_scud(temp.path()));

        std::fs::create_dir(temp.path().join(".scud")).unwrap();
        assert!(has_scud(temp.path()));
    }

    #[test]
    fn test_detect_workspace_type() {
        let temp = TempDir::new().unwrap();

        // Not initialized
        assert_eq!(detect_workspace_type(temp.path()), WorkspaceType::NotInitialized);

        // SCUD only
        std::fs::create_dir(temp.path().join(".scud")).unwrap();
        assert_eq!(detect_workspace_type(temp.path()), WorkspaceType::ScudOnly);

        // Dual workspace
        std::fs::create_dir(temp.path().join(".descartes")).unwrap();
        assert_eq!(detect_workspace_type(temp.path()), WorkspaceType::DualWorkspace);
    }

    #[test]
    fn test_is_dual_workspace() {
        let temp = TempDir::new().unwrap();

        assert!(!is_dual_workspace(temp.path()));

        std::fs::create_dir(temp.path().join(".scud")).unwrap();
        assert!(!is_dual_workspace(temp.path()));

        std::fs::create_dir(temp.path().join(".descartes")).unwrap();
        assert!(is_dual_workspace(temp.path()));
    }

    #[test]
    fn test_scud_tasks_file() {
        let workspace = PathBuf::from("/home/user/project");
        assert_eq!(
            scud_tasks_file(&workspace),
            PathBuf::from("/home/user/project/.scud/tasks/tasks.json")
        );
    }

    #[test]
    fn test_scud_workflow_state_file() {
        let workspace = PathBuf::from("/home/user/project");
        assert_eq!(
            scud_workflow_state_file(&workspace),
            PathBuf::from("/home/user/project/.scud/workflow-state.json")
        );
    }

    #[test]
    fn test_workspace_type_display() {
        assert_eq!(format!("{}", WorkspaceType::DescartesOnly), "Descartes");
        assert_eq!(format!("{}", WorkspaceType::ScudOnly), "SCUD");
        assert_eq!(format!("{}", WorkspaceType::DualWorkspace), "Descartes+SCUD");
        assert_eq!(format!("{}", WorkspaceType::NotInitialized), "Not Initialized");
    }

    #[test]
    fn test_ensure_scud_dir() {
        let temp = TempDir::new().unwrap();

        assert!(!temp.path().join(".scud").exists());

        ensure_scud_dir(temp.path()).unwrap();

        assert!(temp.path().join(".scud").exists());
        assert!(temp.path().join(".scud/tasks").exists());
    }
}
