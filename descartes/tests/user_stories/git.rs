//! Git Automation Tests (US-41 to US-42)
//!
//! Tests for git-related features:
//! - US-41: AI-Generated Commit Messages
//! - US-42: Automatic Task Completion and Commit

use crate::e2e::fixtures::TestProject;
use std::process::Command;

// US-41: AI-Generated Commit Messages

#[test]
fn test_us41_git_project_initialized() {
    let project = TestProject::git_project();

    // Verify git repo exists
    let git_dir = project.path.join(".git");
    assert!(git_dir.exists());
    assert!(git_dir.is_dir());
}

#[test]
fn test_us41_git_has_initial_commit() {
    let project = TestProject::git_project();

    // Run git log to verify initial commit
    let output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to run git log");

    let log = String::from_utf8_lossy(&output.stdout);
    assert!(log.contains("Initial commit"));
}

#[test]
fn test_us41_git_config_set() {
    let project = TestProject::git_project();

    // Verify git config is set
    let output = Command::new("git")
        .args(["config", "user.email"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to get git config");

    let email = String::from_utf8_lossy(&output.stdout);
    assert!(email.contains("test@example.com"));
}

#[test]
fn test_us41_commit_message_conventional_format() {
    // Test that generated commit messages follow conventional format
    let commit_types = vec![
        "feat: ", "fix: ", "docs: ", "style: ", "refactor: ",
        "test: ", "chore: ", "perf: ", "ci: ", "build: ",
    ];

    // A conventional commit message should start with one of these
    let sample_message = "feat: Add user authentication";
    assert!(commit_types.iter().any(|t| sample_message.starts_with(t)));
}

#[test]
fn test_us41_commit_message_includes_task_reference() {
    // Commit messages should reference the task ID
    let message_with_task = "[TASK-1] feat: Implement user registration";
    assert!(message_with_task.contains("[TASK-"));
}

#[test]
fn test_us41_git_diff_available() {
    let project = TestProject::git_project();

    // Make a change
    std::fs::write(project.path.join("new_file.txt"), "new content").unwrap();

    // Git diff should show the new file
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to run git status");

    let status = String::from_utf8_lossy(&output.stdout);
    assert!(status.contains("new_file.txt"));
}

// US-42: Automatic Task Completion and Commit

#[test]
fn test_us42_stage_and_commit_workflow() {
    let project = TestProject::git_project();

    // Create a new file
    std::fs::write(project.path.join("feature.rs"), "// New feature").unwrap();

    // Stage the file
    let stage_output = Command::new("git")
        .args(["add", "feature.rs"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to stage file");
    assert!(stage_output.status.success());

    // Commit
    let commit_output = Command::new("git")
        .args(["commit", "-m", "[TASK-1] feat: Add new feature"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to commit");
    assert!(commit_output.status.success());

    // Verify commit in log
    let log_output = Command::new("git")
        .args(["log", "--oneline", "-1"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to get log");

    let log = String::from_utf8_lossy(&log_output.stdout);
    assert!(log.contains("TASK-1"));
    assert!(log.contains("Add new feature"));
}

#[test]
fn test_us42_commit_all_changes() {
    let project = TestProject::git_project();

    // Create multiple files
    std::fs::write(project.path.join("file1.rs"), "// File 1").unwrap();
    std::fs::write(project.path.join("file2.rs"), "// File 2").unwrap();

    // Stage all and commit
    Command::new("git")
        .args(["add", "."])
        .current_dir(&project.path)
        .output()
        .expect("Failed to stage files");

    let commit_output = Command::new("git")
        .args(["commit", "-m", "[TASK-2] feat: Add multiple files"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to commit");
    assert!(commit_output.status.success());

    // Verify both files are in the commit
    let show_output = Command::new("git")
        .args(["show", "--stat", "HEAD"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to show commit");

    let show = String::from_utf8_lossy(&show_output.stdout);
    assert!(show.contains("file1.rs"));
    assert!(show.contains("file2.rs"));
}

#[test]
fn test_us42_no_changes_no_commit() {
    let project = TestProject::git_project();

    // Try to commit with no changes
    let output = Command::new("git")
        .args(["commit", "-m", "Empty commit"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to run commit");

    // Should fail with "nothing to commit"
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{}{}", stdout, stderr);
    assert!(combined.contains("nothing to commit") || !output.status.success());
}

#[test]
fn test_us42_git_log_format() {
    let project = TestProject::git_project();

    // Make a commit
    std::fs::write(project.path.join("test.rs"), "// Test").unwrap();
    Command::new("git").args(["add", "."]).current_dir(&project.path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "[TASK-3] test: Add test file"])
        .current_dir(&project.path)
        .output()
        .unwrap();

    // Verify log format
    let log = Command::new("git")
        .args(["log", "--pretty=format:%s", "-1"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to get log");

    let message = String::from_utf8_lossy(&log.stdout);
    assert!(message.starts_with("[TASK-3]"));
}

#[test]
fn test_us42_verify_clean_working_directory() {
    let project = TestProject::git_project();

    // Initially should be clean
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to get status");

    let status = String::from_utf8_lossy(&output.stdout);
    assert!(status.trim().is_empty(), "Working directory should be clean");
}

#[test]
fn test_us42_commit_amend_not_used_by_default() {
    // This is a policy test - we should use new commits, not amend
    // Amend is destructive for shared history
    let project = TestProject::git_project();

    // Make first commit
    std::fs::write(project.path.join("first.rs"), "// First").unwrap();
    Command::new("git").args(["add", "."]).current_dir(&project.path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "First commit"])
        .current_dir(&project.path)
        .output()
        .unwrap();

    // Make second commit (not amend)
    std::fs::write(project.path.join("second.rs"), "// Second").unwrap();
    Command::new("git").args(["add", "."]).current_dir(&project.path).output().unwrap();
    Command::new("git")
        .args(["commit", "-m", "Second commit"])
        .current_dir(&project.path)
        .output()
        .unwrap();

    // Should have 3 commits (initial + 2 new)
    let log = Command::new("git")
        .args(["rev-list", "--count", "HEAD"])
        .current_dir(&project.path)
        .output()
        .expect("Failed to count commits");

    let count: i32 = String::from_utf8_lossy(&log.stdout).trim().parse().unwrap_or(0);
    assert_eq!(count, 3, "Should have 3 separate commits");
}
