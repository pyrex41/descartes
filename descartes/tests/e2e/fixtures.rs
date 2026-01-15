//! Test fixtures for e2e integration tests
//!
//! Provides utilities for creating temporary SCUD projects and complex scenarios.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// Sample PRD content for testing parse functionality
pub const SAMPLE_PRD: &str = r#"# Product Requirements Document: Test Feature

## Overview
Implement a test feature with multiple components.

## Features

### 1. User Authentication
Implement user login and registration system.

### 2. Data Processing
Build data ingestion and transformation pipeline.

### 3. API Endpoints
Create REST API endpoints for the feature.
"#;

/// A test project with SCUD initialized
pub struct TestProject {
    /// Temporary directory (dropped when TestProject is dropped)
    pub dir: TempDir,

    /// Path to the project root
    pub path: PathBuf,
}

impl TestProject {
    /// Create a minimal SCUD project
    pub fn new() -> Self {
        let dir = TempDir::new().expect("Failed to create temp dir");
        let path = dir.path().to_path_buf();

        // Create .scud directory structure
        fs::create_dir_all(path.join(".scud/tasks")).unwrap();

        // Create minimal config
        let config = r#"
[llm]
provider = "mock"
model = "mock-model"

[swarm]
round_size = 3

[swarm.backpressure]
commands = ["echo 'ok'"]
stop_on_failure = true
timeout_secs = 30
"#;
        fs::write(path.join(".scud/config.toml"), config).unwrap();

        Self { dir, path }
    }

    /// Create a project with specific tasks
    pub fn with_tasks(tasks: &str) -> Self {
        let project = Self::new();

        // Write tasks.scg
        fs::write(project.path.join(".scud/tasks/tasks.scg"), tasks).unwrap();

        project
    }

    /// Create a project with a simple 3-task, 2-wave structure
    pub fn simple_project() -> Self {
        let tasks = r#"# SCUD Graph v1
# Phase: test

@meta {
  name test
  id_format sequential
}

@nodes
# id | title | status | complexity | priority
1 | Setup environment | P | 2 | H
2 | Build core module | P | 3 | H
3 | Run tests | P | 2 | M

@edges
# dependent -> dependency
2 -> 1
3 -> 2

@details
1 | description |
  Initialize project structure and dependencies
2 | description |
  Compile core functionality
3 | description |
  Execute test suite
"#;
        Self::with_tasks(tasks)
    }

    /// Create a project with parallel tasks (for testing waves)
    pub fn parallel_project() -> Self {
        let tasks = r#"# SCUD Graph v1
# Phase: parallel

@meta {
  name parallel
  id_format sequential
}

@nodes
# id | title | status | complexity | priority
1 | Task A (independent) | P | 2 | H
2 | Task B (independent) | P | 2 | H
3 | Task C (depends on A and B) | P | 3 | H

@edges
3 -> 1
3 -> 2

@details
1 | description |
  Independent task A
2 | description |
  Independent task B
3 | description |
  Depends on both A and B
"#;
        Self::with_tasks(tasks)
    }

    /// Create a broken Rust project (for backpressure testing)
    pub fn broken_rust_project() -> Self {
        let project = Self::simple_project();

        // Create a Cargo.toml
        let cargo_toml = r#"
[package]
name = "broken"
version = "0.1.0"
edition = "2021"
"#;
        fs::write(project.path.join("Cargo.toml"), cargo_toml).unwrap();

        // Create broken src/main.rs
        fs::create_dir_all(project.path.join("src")).unwrap();
        fs::write(
            project.path.join("src/main.rs"),
            "fn main() { undefined_variable }",
        )
        .unwrap();

        // Update backpressure to actually run cargo build
        let config = r#"
[llm]
provider = "mock"
model = "mock-model"

[swarm]
round_size = 3

[swarm.backpressure]
commands = ["cargo build"]
stop_on_failure = true
timeout_secs = 60
"#;
        fs::write(project.path.join(".scud/config.toml"), config).unwrap();

        project
    }

    /// Get the SCUD tag for this project
    pub fn tag(&self) -> String {
        // Read from tasks.scg to find the phase name
        let tasks_path = self.path.join(".scud/tasks/tasks.scg");
        if tasks_path.exists() {
            let content = fs::read_to_string(&tasks_path).unwrap_or_default();
            // Parse "name <tag>" from @meta section
            for line in content.lines() {
                if line.trim().starts_with("name ") {
                    return line.trim().strip_prefix("name ").unwrap().to_string();
                }
            }
        }
        "test".to_string()
    }

    /// Create a project with a large wave structure (for stress testing)
    pub fn large_wave_project() -> Self {
        let tasks = r#"# SCUD Graph v1
# Phase: large

@meta {
  name large
  id_format sequential
}

@nodes
# id | title | status | complexity | priority
1 | Independent A | P | 2 | H
2 | Independent B | P | 2 | H
3 | Independent C | P | 2 | H
4 | Independent D | P | 2 | H
5 | Wave 2 - depends on 1,2 | P | 3 | H
6 | Wave 2 - depends on 3,4 | P | 3 | H
7 | Wave 3 - depends on all | P | 4 | H

@edges
5 -> 1
5 -> 2
6 -> 3
6 -> 4
7 -> 5
7 -> 6

@details
1 | description |
  Independent task A
2 | description |
  Independent task B
3 | description |
  Independent task C
4 | description |
  Independent task D
5 | description |
  Depends on 1 and 2
6 | description |
  Depends on 3 and 4
7 | description |
  Final task depending on all previous waves
"#;
        Self::with_tasks(tasks)
    }

    /// Create a project with mixed task statuses (for swarm testing)
    pub fn mixed_status_project() -> Self {
        let tasks = r#"# SCUD Graph v1
# Phase: mixed

@meta {
  name mixed
  id_format sequential
}

@nodes
# id | title | status | complexity | priority
1 | Already done task | D | 2 | H
2 | In progress task | I | 3 | H
3 | Pending - ready | P | 2 | M
4 | Pending - blocked | P | 3 | H
5 | Blocked task | B | 2 | L

@edges
4 -> 2
5 -> 4

@details
1 | description |
  This task is already completed
2 | description |
  This task is being worked on
3 | description |
  This task has no dependencies and is ready
4 | description |
  This task is blocked by in-progress task 2
5 | description |
  This task is explicitly blocked
"#;
        Self::with_tasks(tasks)
    }

    /// Create a project with task overrides for category testing
    pub fn task_override_project() -> Self {
        let project = Self::simple_project();

        // Create a task file with YAML frontmatter override
        let task_md = r#"---
category: fast-builder
disable_review: true
---

# Task 1: Setup environment

Initialize project structure and dependencies.

## Test Strategy
Unit tests for configuration loading.
"#;
        fs::create_dir_all(project.path.join(".scud/tasks")).unwrap();
        fs::write(project.path.join(".scud/tasks/task-1.md"), task_md).unwrap();

        project
    }

    /// Create a project configured for validation testing
    pub fn validation_project() -> Self {
        let project = Self::broken_rust_project();

        // Update config with validation settings
        let config = r#"
[llm]
provider = "mock"
model = "mock-model"

[swarm]
round_size = 3

[swarm.validation]
enabled = true
commands = ["cargo build", "cargo test"]
stop_on_failure = true

[swarm.backpressure]
commands = ["cargo build"]
stop_on_failure = true
timeout_secs = 60
"#;
        fs::write(project.path.join(".scud/config.toml"), config).unwrap();

        project
    }

    /// Create a project with git initialized (for commit testing)
    pub fn git_project() -> Self {
        let project = Self::simple_project();

        // Initialize git repo
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(&project.path)
            .output()
            .expect("Failed to init git");

        std::process::Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&project.path)
            .output()
            .expect("Failed to set git email");

        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&project.path)
            .output()
            .expect("Failed to set git name");

        // Create initial commit
        fs::write(project.path.join("README.md"), "# Test Project").unwrap();

        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&project.path)
            .output()
            .expect("Failed to git add");

        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&project.path)
            .output()
            .expect("Failed to git commit");

        project
    }

    /// Create a project with transcript directory for replay testing
    pub fn transcript_project() -> Self {
        let project = Self::simple_project();

        // Create transcripts directory with sample transcript
        fs::create_dir_all(project.path.join(".descartes/transcripts")).unwrap();

        let transcript = r#"# Descartes Transcript v1
# Session: test-session-001

@meta {
  task_id 1
  agent_type Builder
  started_at 2025-01-15T10:00:00Z
  ended_at 2025-01-15T10:05:00Z
  status completed
}

@messages
H | 2025-01-15T10:00:00Z | Implement the authentication module
A | 2025-01-15T10:01:00Z | I'll implement the authentication module with JWT tokens.
A | 2025-01-15T10:02:00Z | @tool_call Edit { file: "src/auth.rs", changes: "..." }
A | 2025-01-15T10:03:00Z | Implementation complete. Running tests...
A | 2025-01-15T10:04:00Z | All tests pass. Task completed.

@metrics {
  tokens_used 1500
  tool_calls 3
  iterations 1
}
"#;
        fs::write(
            project.path.join(".descartes/transcripts/test-session-001.scg"),
            transcript,
        )
        .unwrap();

        project
    }

    /// Create a multi-phase project for cross-phase dependency testing
    pub fn multi_phase_project() -> Self {
        let project = Self::new();

        // Create two phases with cross-phase dependency
        let phase1 = r#"# SCUD Graph v1
# Phase: phase1

@meta {
  name phase1
  id_format sequential
}

@nodes
1 | Core module setup | P | 3 | H
2 | Database schema | P | 4 | H

@edges

@details
1 | description |
  Set up core module structure
2 | description |
  Design and implement database schema
"#;

        let phase2 = r#"# SCUD Graph v1
# Phase: phase2

@meta {
  name phase2
  id_format sequential
}

@nodes
1 | API implementation | P | 5 | H
2 | Frontend integration | P | 4 | M

@edges
1 -> phase1:2
2 -> 1

@details
1 | description |
  Implement API endpoints (depends on phase1 database)
2 | description |
  Integrate frontend with API
"#;

        fs::write(project.path.join(".scud/tasks/phase1.scg"), phase1).unwrap();
        fs::write(project.path.join(".scud/tasks/phase2.scg"), phase2).unwrap();

        // Set active tag to phase2
        fs::write(project.path.join(".scud/active-tag"), "phase2").unwrap();

        project
    }
}

impl Default for TestProject {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_simple_project() {
        let project = TestProject::simple_project();

        assert!(project.path.join(".scud").exists());
        assert!(project.path.join(".scud/tasks/tasks.scg").exists());
        assert!(project.path.join(".scud/config.toml").exists());
    }

    #[test]
    fn test_project_tag() {
        let project = TestProject::simple_project();
        assert_eq!(project.tag(), "test");

        let parallel = TestProject::parallel_project();
        assert_eq!(parallel.tag(), "parallel");
    }

    #[test]
    fn test_broken_rust_project() {
        let project = TestProject::broken_rust_project();

        assert!(project.path.join("Cargo.toml").exists());
        assert!(project.path.join("src/main.rs").exists());

        // Verify the code is actually broken
        let content = fs::read_to_string(project.path.join("src/main.rs")).unwrap();
        assert!(content.contains("undefined_variable"));
    }
}
