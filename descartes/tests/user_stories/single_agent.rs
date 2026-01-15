//! Single-Agent Workflow Tests (US-23 to US-25)
//!
//! Tests for single-agent workflow commands:
//! - US-23: Interactive AI Session
//! - US-24: Task Implementation with AI
//! - US-25: Implementation Planning

use crate::e2e::fixtures::TestProject;
use crate::e2e::mock_harness::{MockHarness, MockResponse};
use descartes::harness::{Harness, SessionConfig};

// US-23: Interactive AI Session

#[tokio::test]
async fn test_us23_interactive_starts_session() {
    let harness = MockHarness::new();
    let config = SessionConfig::default();

    let session = harness.start_session(config).await;
    assert!(session.is_ok());

    let session = session.unwrap();
    assert!(!session.id.is_empty());
    assert_eq!(session.harness, "mock");
}

#[tokio::test]
async fn test_us23_session_maintains_state() {
    let harness = MockHarness::new();
    let session = harness.start_session(SessionConfig::default()).await.unwrap();

    // Send multiple messages
    let _stream1 = harness.send(&session, "First message").await.unwrap();
    let _stream2 = harness.send(&session, "Second message").await.unwrap();

    // Verify calls were logged
    let calls = harness.calls();
    assert_eq!(calls.len(), 2);
    assert!(calls[0].message.contains("First"));
    assert!(calls[1].message.contains("Second"));
}

#[tokio::test]
async fn test_us23_session_handles_errors_gracefully() {
    let mut harness = MockHarness::new();
    harness.when_task("error", MockResponse::Timeout);

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let stream = harness.send(&session, "Task error: trigger timeout").await;

    assert!(stream.is_ok()); // Stream is returned even for timeout responses
}

// US-24: Task Implementation with AI

#[tokio::test]
async fn test_us24_task_implementation_receives_task_context() {
    let _project = TestProject::simple_project();
    let harness = MockHarness::new();

    let session = harness.start_session(SessionConfig::default()).await.unwrap();

    // Simulate sending task context
    let task_prompt = "Task 1: Setup environment\nDescription: Initialize project structure";
    let _stream = harness.send(&session, task_prompt).await.unwrap();

    let calls = harness.calls();
    assert_eq!(calls.len(), 1);
    assert!(calls[0].message.contains("Task 1"));
    assert!(calls[0].message.contains("Setup environment"));
}

#[tokio::test]
async fn test_us24_task_success_response() {
    use futures::StreamExt;
    use descartes::harness::ResponseChunk;

    let mut harness = MockHarness::new();
    harness.when_task("1", MockResponse::Success {
        output: "Task completed successfully. All tests pass.".to_string(),
    });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Task 1: Implement feature").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    // Should have text response followed by done
    assert!(matches!(responses[0], ResponseChunk::Text(_)));
    if let ResponseChunk::Text(text) = &responses[0] {
        assert!(text.contains("completed successfully"));
    }
}

#[tokio::test]
async fn test_us24_task_blocked_response() {
    use futures::StreamExt;
    use descartes::harness::ResponseChunk;

    let mut harness = MockHarness::new();
    harness.when_task("2", MockResponse::Blocked {
        reason: "Missing API credentials".to_string(),
    });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Task 2: Call external API").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    if let ResponseChunk::Text(text) = &responses[0] {
        assert!(text.contains("TASK_BLOCKED"));
        assert!(text.contains("Missing API credentials"));
    }
}

// US-25: Implementation Planning

#[tokio::test]
async fn test_us25_plan_mode_request() {
    let harness = MockHarness::new();
    let session = harness.start_session(SessionConfig::default()).await.unwrap();

    // Simulate plan mode request
    let plan_prompt = "Plan the implementation of user authentication:\n\
        - Database schema\n\
        - API endpoints\n\
        - Frontend components";

    let _stream = harness.send(&session, plan_prompt).await.unwrap();

    let calls = harness.calls();
    assert!(calls[0].message.contains("Plan"));
}

#[tokio::test]
async fn test_us25_plan_response_structure() {
    use futures::StreamExt;
    use descartes::harness::ResponseChunk;

    let harness = MockHarness::new()
        .with_default_response(MockResponse::Success {
            output: r#"## Implementation Plan

### Phase 1: Database Setup
1. Create user table schema
2. Add authentication columns

### Phase 2: API Implementation
1. Create auth endpoints
2. Implement JWT tokens

### Risks
- Token expiration handling
- Session management complexity
"#.to_string(),
        });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Plan: User authentication").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    if let ResponseChunk::Text(text) = &responses[0] {
        assert!(text.contains("Implementation Plan"));
        assert!(text.contains("Phase 1"));
        assert!(text.contains("Risks"));
    }
}

#[tokio::test]
async fn test_us25_multiple_sessions_independent() {
    let harness = MockHarness::new();

    let session1 = harness.start_session(SessionConfig::default()).await.unwrap();
    let session2 = harness.start_session(SessionConfig::default()).await.unwrap();

    // Sessions should have different IDs
    assert_ne!(session1.id, session2.id);

    // Send to each session
    let _stream1 = harness.send(&session1, "Session 1 message").await.unwrap();
    let _stream2 = harness.send(&session2, "Session 2 message").await.unwrap();

    let calls = harness.calls();
    assert_eq!(calls.len(), 2);

    // Verify messages are associated with correct sessions
    assert_eq!(calls[0].session_id, session1.id);
    assert_eq!(calls[1].session_id, session2.id);
}
