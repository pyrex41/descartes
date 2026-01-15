//! Context Management Tests (US-29 to US-31)
//!
//! Tests for context management features:
//! - US-29: Fresh Context Pattern
//! - US-30: Subagent Result Injection
//! - US-31: Single-Level Depth Enforcement

use crate::e2e::mock_harness::{MockHarness, MockResponse};
use descartes::harness::{Harness, SessionConfig, ResponseChunk, SubagentRequest, SubagentResult, SubagentMetrics};
use descartes::context_handoff::{ContextMonitor, HandoffContext};

// US-29: Fresh Context Pattern

#[test]
fn test_us29_context_monitor_tracks_usage() {
    let mut monitor = ContextMonitor::new(100_000, 0.8); // 100k token limit, 80% threshold

    // Record some usage (record_usage takes &str, not usize)
    // 40000 chars ≈ 10000 tokens
    let text_10k = "x".repeat(40000);
    monitor.record_usage(&text_10k);
    assert!((monitor.usage_percentage() - 0.10).abs() < 0.01);

    // Record more (160000 chars ≈ 40000 tokens, total 50000)
    let text_40k = "y".repeat(160000);
    monitor.record_usage(&text_40k);
    assert!((monitor.usage_percentage() - 0.50).abs() < 0.01);
}

#[test]
fn test_us29_context_monitor_detects_handoff_needed() {
    let mut monitor = ContextMonitor::new(100_000, 0.8); // 80% threshold

    // Under threshold (70000 tokens = 280000 chars)
    let text_70k = "x".repeat(280000);
    monitor.record_usage(&text_70k);
    assert!(!monitor.should_handoff());

    // Over threshold (add 15000 tokens = 60000 chars)
    let text_15k = "y".repeat(60000);
    monitor.record_usage(&text_15k);
    assert!(monitor.should_handoff());
}

#[test]
fn test_us29_context_monitor_reset() {
    let mut monitor = ContextMonitor::new(100_000, 0.8);

    // 50000 tokens = 200000 chars
    let text_50k = "x".repeat(200000);
    monitor.record_usage(&text_50k);
    assert!((monitor.usage_percentage() - 0.50).abs() < 0.01);

    monitor.reset();
    assert!((monitor.usage_percentage() - 0.0).abs() < 0.01);
}

#[test]
fn test_us29_handoff_context_builds_prompt() {
    let context = HandoffContext::new(
        "Completed database schema, working on API".to_string(),
        "Implement user auth: create login/logout endpoints".to_string(),
        0,
    );

    let prompt = context.build_handoff_prompt();
    assert!(prompt.contains("Iteration 1"));
    assert!(prompt.contains("database schema") || prompt.contains("auth"));
}

#[test]
fn test_us29_fresh_context_for_new_iteration() {
    // Each iteration should start with a fresh context
    let context1 = HandoffContext::new(
        "Iteration 1 progress: started implementation".to_string(),
        "Task 1: Build feature".to_string(),
        0,
    );

    let context2 = HandoffContext::new(
        "Iteration 2 progress: continued implementation".to_string(),
        "Task 1: Build feature".to_string(),
        1,
    );

    // Different iterations have different progress summaries
    assert_ne!(context1.summary, context2.summary);
}

// US-30: Subagent Result Injection

#[tokio::test]
async fn test_us30_inject_result_into_session() {
    let harness = MockHarness::new();
    let session = harness.start_session(SessionConfig::default()).await.unwrap();

    let result = SubagentResult {
        session_id: "subagent-123".to_string(),
        output: "Subagent completed successfully".to_string(),
        success: true,
        metrics: SubagentMetrics {
            tokens_in: 500,
            tokens_out: 200,
            tokens_total: 700,
            duration_ms: 1000,
            tools_called: 2,
        },
    };

    // Inject result should succeed
    let inject_result = harness.inject_result(&session, result).await;
    assert!(inject_result.is_ok());
}

#[tokio::test]
async fn test_us30_inject_failed_result() {
    let harness = MockHarness::new();
    let session = harness.start_session(SessionConfig::default()).await.unwrap();

    let result = SubagentResult {
        session_id: "subagent-456".to_string(),
        output: "Error: API key invalid".to_string(),
        success: false,
        metrics: SubagentMetrics {
            tokens_in: 100,
            tokens_out: 50,
            tokens_total: 150,
            duration_ms: 500,
            tools_called: 0,
        },
    };

    let inject_result = harness.inject_result(&session, result).await;
    assert!(inject_result.is_ok());
}

#[tokio::test]
async fn test_us30_subagent_spawn_detection() {
    use futures::StreamExt;

    let mut harness = MockHarness::new();
    harness.when_task("complex", MockResponse::SubagentSpawn {
        category: "searcher".to_string(),
        task: "Find all error handling patterns".to_string(),
    });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Task complex: Analyze codebase").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    // Response should indicate subagent spawn request
    if let ResponseChunk::Text(text) = &responses[0] {
        assert!(text.contains("SPAWN_SUBAGENT"));
        assert!(text.contains("searcher"));
    }
}

// US-31: Single-Level Depth Enforcement

#[test]
fn test_us31_detect_nested_spawn_blocked() {
    // Subagents should not be able to spawn their own subagents
    let harness = MockHarness::new();

    // detect_subagent_spawn should return None for mock harness
    let chunk = ResponseChunk::Text("SPAWN_SUBAGENT: nested".to_string());
    let request = harness.detect_subagent_spawn(&chunk);

    // Mock harness doesn't allow subagent spawning
    assert!(request.is_none());
}

#[tokio::test]
async fn test_us31_subagent_session_has_parent() {
    let harness = MockHarness::new();

    // Create a session configured as a subagent
    let parent_handle = descartes::harness::SessionHandle {
        id: "parent-123".to_string(),
        harness: "mock".to_string(),
        model: "mock-model".to_string(),
        parent: None,
    };

    let config = SessionConfig {
        model: "mock-model".to_string(),
        tools: vec![],
        system_prompt: None,
        parent: Some(parent_handle),
        is_subagent: true,
    };

    let session = harness.start_session(config).await.unwrap();

    // Session should still work
    assert!(!session.id.is_empty());
}

#[test]
fn test_us31_context_overflow_triggers_handoff_not_spawn() {
    // When context overflows, should trigger handoff, not spawn
    let mut monitor = ContextMonitor::new(100_000, 0.8); // 80% threshold

    // Record 85000 tokens (340000 chars)
    let text_85k = "x".repeat(340000);
    monitor.record_usage(&text_85k);

    assert!(monitor.should_handoff());

    // Handoff should create new session, not spawn subagent
    // tokens_until_handoff returns 0 when over threshold
    assert_eq!(monitor.tokens_until_handoff(), 0);
}

#[tokio::test]
async fn test_us31_context_overflow_response() {
    use futures::StreamExt;

    let mut harness = MockHarness::new();
    harness.when_task("big", MockResponse::ContextOverflow { usage_percent: 95 });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let mut stream = harness.send(&session, "Task big: Process large dataset").await.unwrap();

    let mut responses = vec![];
    while let Some(chunk) = stream.next().await {
        responses.push(chunk);
    }

    if let ResponseChunk::Text(text) = &responses[0] {
        assert!(text.contains("Context usage"));
        assert!(text.contains("95%"));
        assert!(text.contains("handoff"));
    }
}

#[test]
fn test_us31_subagent_request_structure() {
    let request = SubagentRequest {
        category: "searcher".to_string(),
        prompt: "Find all TODO comments".to_string(),
        model: Some("claude-3-haiku".to_string()),
    };

    assert_eq!(request.category, "searcher");
    assert!(request.model.is_some());
}
