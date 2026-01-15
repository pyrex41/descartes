//! Validation Pipeline Tests (US-35 to US-36)
//!
//! Tests for validation and review features:
//! - US-35: Backpressure Validation
//! - US-36: Code Review for Fast Builds

use crate::e2e::fixtures::TestProject;
use crate::e2e::mock_harness::{MockHarness, MockResponse};
use descartes::harness::{Harness, SessionConfig, ResponseChunk};
use descartes::config::CategoryConfig;
use descartes::agent::AgentCategory;

// US-35: Backpressure Validation

#[test]
fn test_us35_validation_project_config() {
    let project = TestProject::validation_project();

    // Read config to verify validation is enabled
    let config_path = project.path.join(".scud/config.toml");
    let config = std::fs::read_to_string(&config_path).unwrap();

    assert!(config.contains("[swarm.validation]"));
    assert!(config.contains("enabled = true"));
    assert!(config.contains("stop_on_failure = true"));
}

#[test]
fn test_us35_broken_project_detectable() {
    let project = TestProject::broken_rust_project();

    // The broken project should have invalid Rust code
    let main_rs = project.path.join("src/main.rs");
    let content = std::fs::read_to_string(&main_rs).unwrap();

    assert!(content.contains("undefined_variable"));
}

#[tokio::test]
async fn test_us35_validation_failed_response() {
    use futures::StreamExt;

    let mut harness = MockHarness::new();
    harness.when_task("build", MockResponse::ValidationFailed {
        errors: vec![
            "cargo build failed".to_string(),
            "error[E0425]: cannot find value `undefined_variable`".to_string(),
        ],
    });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Task build: Implement feature").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    if let ResponseChunk::Text(text) = &responses[0] {
        assert!(text.contains("VALIDATION_FAILED"));
        assert!(text.contains("cargo build failed"));
    }
}

#[tokio::test]
async fn test_us35_validation_success_allows_completion() {
    use futures::StreamExt;

    let mut harness = MockHarness::new();
    harness.when_task("1", MockResponse::Success {
        output: "Implementation complete. All validation checks passed.".to_string(),
    });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Task 1: Implement feature").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    if let ResponseChunk::Text(text) = &responses[0] {
        assert!(text.contains("complete"));
        assert!(text.contains("passed") || text.contains("complete"));
    }
}

#[test]
fn test_us35_backpressure_commands_configured() {
    let project = TestProject::validation_project();

    let config_path = project.path.join(".scud/config.toml");
    let config = std::fs::read_to_string(&config_path).unwrap();

    assert!(config.contains("[swarm.backpressure]"));
    assert!(config.contains("commands = "));
    assert!(config.contains("timeout_secs"));
}

// US-36: Code Review for Fast Builds

#[tokio::test]
async fn test_us36_review_required_response() {
    use futures::StreamExt;

    let mut harness = MockHarness::new();
    harness.when_task("fast", MockResponse::ReviewRequired {
        changes: "Modified auth.rs: Added JWT validation".to_string(),
    });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Task fast: Quick auth fix").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    if let ResponseChunk::Text(text) = &responses[0] {
        assert!(text.contains("REVIEW_REQUIRED"));
        assert!(text.contains("Modified auth.rs"));
    }
}

#[test]
fn test_us36_category_config_has_backpressure_field() {
    // Use default_config() from AgentCategory instead
    let config = AgentCategory::Validator.default_config();
    // Validator config should have backpressure=true
    assert!(config.backpressure);
}

#[test]
fn test_us36_category_has_parallel_field() {
    let config = AgentCategory::Searcher.default_config();
    // Searcher should have parallel=true
    assert!(config.parallel);
}

#[tokio::test]
async fn test_us36_tool_call_triggers_review() {
    use futures::StreamExt;

    let mut harness = MockHarness::new();
    harness.when_task("edit", MockResponse::ToolCall {
        tool_name: "Edit".to_string(),
        arguments: serde_json::json!({
            "file": "src/main.rs",
            "changes": "Add error handling"
        }),
    });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Task edit: Add error handling").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    // Should have tool call response
    assert!(responses.iter().any(|r| matches!(r, ResponseChunk::ToolCall(_))));
}

#[test]
fn test_us36_task_override_project_has_disable_review() {
    let project = TestProject::task_override_project();

    let task_md = project.path.join(".scud/tasks/task-1.md");
    let content = std::fs::read_to_string(&task_md).unwrap();

    assert!(content.contains("disable_review: true"));
}

#[test]
fn test_us36_validation_stops_on_failure() {
    let project = TestProject::validation_project();

    let config_path = project.path.join(".scud/config.toml");
    let config = std::fs::read_to_string(&config_path).unwrap();

    // Both validation and backpressure should stop on failure
    assert!(config.contains("stop_on_failure = true"));
}

#[test]
fn test_us36_category_from_string() {
    // AgentCategory implements FromStr, returning Result
    assert!("fast-builder".parse::<AgentCategory>().is_ok());
    assert!("builder".parse::<AgentCategory>().is_ok());
    // "invalid" becomes Custom("invalid"), so it's also Ok
    assert!("invalid".parse::<AgentCategory>().is_ok());
    // Verify it becomes Custom
    assert!(matches!(
        "invalid".parse::<AgentCategory>().unwrap(),
        AgentCategory::Custom(_)
    ));
}
