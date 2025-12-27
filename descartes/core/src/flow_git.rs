//! Git operations for flow workflow checkpoints
//!
//! Provides git checkpoint operations for the flow workflow,
//! allowing automatic commits at phase boundaries and rollback capability.

use anyhow::Result;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Git checkpoint operations for flow workflow
pub struct FlowGit {
    working_dir: PathBuf,
}

impl FlowGit {
    /// Create a new FlowGit instance for the given working directory
    pub fn new(working_dir: impl AsRef<Path>) -> Self {
        Self {
            working_dir: working_dir.as_ref().to_path_buf(),
        }
    }

    /// Get current HEAD commit hash
    pub fn current_commit(&self) -> Result<String> {
        let output = Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(anyhow::anyhow!("Failed to get current commit"))
        }
    }

    /// Get current branch name
    pub fn current_branch(&self) -> Result<Option<String>> {
        let output = Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if branch == "HEAD" {
                Ok(None) // Detached HEAD
            } else {
                Ok(Some(branch))
            }
        } else {
            Ok(None)
        }
    }

    /// Check if working directory is clean (no uncommitted changes)
    pub fn is_clean(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.working_dir)
            .output()?;

        Ok(output.status.success() && output.stdout.is_empty())
    }

    /// Check if there are any staged or unstaged changes
    pub fn has_changes(&self) -> Result<bool> {
        let output = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&self.working_dir)
            .output()?;

        Ok(output.status.success() && !output.stdout.is_empty())
    }

    /// Create checkpoint commit for phase
    pub fn create_checkpoint(&self, phase: &str, tag: &str) -> Result<String> {
        // Check if there are changes to commit
        if !self.has_changes()? {
            // No changes, return current commit
            return self.current_commit();
        }

        // Stage all changes
        let add_output = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&self.working_dir)
            .output()?;

        if !add_output.status.success() {
            let stderr = String::from_utf8_lossy(&add_output.stderr);
            return Err(anyhow::anyhow!("Failed to stage changes: {}", stderr));
        }

        // Create commit
        let message = format!(
            "flow({}): checkpoint after {} phase\n\nAutomated checkpoint by Descartes flow workflow",
            tag, phase
        );

        let output = Command::new("git")
            .args(["commit", "-m", &message])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            self.current_commit()
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Check if "nothing to commit" (not really an error)
            if stderr.contains("nothing to commit") {
                self.current_commit()
            } else {
                Err(anyhow::anyhow!("Failed to create checkpoint: {}", stderr))
            }
        }
    }

    /// Rollback to a specific commit (hard reset)
    pub fn rollback(&self, commit: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["reset", "--hard", commit])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to rollback: {}", stderr))
        }
    }

    /// Stash current changes
    pub fn stash(&self, message: &str) -> Result<()> {
        let output = Command::new("git")
            .args(["stash", "push", "-m", message])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to stash: {}", stderr))
        }
    }

    /// Pop the most recent stash
    pub fn stash_pop(&self) -> Result<()> {
        let output = Command::new("git")
            .args(["stash", "pop"])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to pop stash: {}", stderr))
        }
    }

    /// Get diff between two commits
    pub fn diff(&self, from_commit: &str, to_commit: &str) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", from_commit, to_commit])
            .current_dir(&self.working_dir)
            .output()?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(anyhow::anyhow!("Failed to get diff: {}", stderr))
        }
    }

    /// Get short commit hash (first 8 characters)
    pub fn short_hash(commit: &str) -> &str {
        if commit.len() >= 8 {
            &commit[..8]
        } else {
            commit
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_git_repo() -> TempDir {
        let temp_dir = TempDir::new().unwrap();

        // Initialize git repo
        Command::new("git")
            .args(["init"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Configure user for commits
        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        // Create initial commit
        fs::write(temp_dir.path().join("README.md"), "# Test").unwrap();
        Command::new("git")
            .args(["add", "-A"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_dir.path())
            .output()
            .unwrap();

        temp_dir
    }

    #[test]
    fn test_current_commit() {
        let temp_dir = setup_git_repo();
        let git = FlowGit::new(temp_dir.path());

        let commit = git.current_commit().unwrap();
        assert!(!commit.is_empty());
        assert_eq!(commit.len(), 40); // Full SHA
    }

    #[test]
    fn test_current_branch() {
        let temp_dir = setup_git_repo();
        let git = FlowGit::new(temp_dir.path());

        let branch = git.current_branch().unwrap();
        // Could be "main" or "master" depending on git config
        assert!(branch.is_some());
    }

    #[test]
    fn test_is_clean() {
        let temp_dir = setup_git_repo();
        let git = FlowGit::new(temp_dir.path());

        // Should be clean initially
        assert!(git.is_clean().unwrap());

        // Create a new file
        fs::write(temp_dir.path().join("new_file.txt"), "content").unwrap();

        // Should not be clean
        assert!(!git.is_clean().unwrap());
    }

    #[test]
    fn test_create_checkpoint() {
        let temp_dir = setup_git_repo();
        let git = FlowGit::new(temp_dir.path());

        // Create a change
        fs::write(temp_dir.path().join("phase1.txt"), "phase 1 complete").unwrap();

        // Create checkpoint
        let commit = git.create_checkpoint("ingest", "test-flow").unwrap();
        assert!(!commit.is_empty());

        // Should be clean after checkpoint
        assert!(git.is_clean().unwrap());
    }

    #[test]
    fn test_create_checkpoint_no_changes() {
        let temp_dir = setup_git_repo();
        let git = FlowGit::new(temp_dir.path());

        let initial_commit = git.current_commit().unwrap();

        // Create checkpoint with no changes
        let commit = git.create_checkpoint("ingest", "test-flow").unwrap();

        // Should return same commit
        assert_eq!(commit, initial_commit);
    }

    #[test]
    fn test_rollback() {
        let temp_dir = setup_git_repo();
        let git = FlowGit::new(temp_dir.path());

        let initial_commit = git.current_commit().unwrap();

        // Create a change and commit
        fs::write(temp_dir.path().join("new_file.txt"), "content").unwrap();
        git.create_checkpoint("ingest", "test-flow").unwrap();

        // Rollback
        git.rollback(&initial_commit).unwrap();

        // File should be gone
        assert!(!temp_dir.path().join("new_file.txt").exists());
    }

    #[test]
    fn test_short_hash() {
        assert_eq!(FlowGit::short_hash("abc123def456"), "abc123de");
        assert_eq!(FlowGit::short_hash("abc"), "abc");
    }
}
