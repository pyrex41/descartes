//! Test fixtures for e2e integration tests
//!
//! Provides utilities for creating temporary SCUD projects.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

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
