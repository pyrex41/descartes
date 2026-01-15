//! Harness Implementation Tests (US-32 to US-34)
//!
//! Tests for different LLM harness implementations:
//! - US-32: Claude Code Harness
//! - US-33: Codex Harness
//! - US-34: OpenCode Harness
//!
//! Most tests use mocks. Feature-gated tests run with real APIs.

use descartes::harness::{HarnessKind, ResponseChunk, ToolCall, SessionConfig, SessionHandle};

// US-32: Claude Code Harness

#[test]
fn test_us32_claude_code_harness_kind() {
    let kind = HarnessKind::ClaudeCode;
    assert!(matches!(kind, HarnessKind::ClaudeCode));
}

#[test]
fn test_us32_session_config_default() {
    let config = SessionConfig::default();
    assert!(!config.is_subagent);
}

#[test]
fn test_us32_session_config_subagent() {
    let parent = SessionHandle {
        id: "parent-123".to_string(),
        harness: "claude-code".to_string(),
        model: "claude-3-opus".to_string(),
        parent: None,
    };

    let config = SessionConfig {
        model: "claude-3-haiku".to_string(),
        tools: vec!["Read".to_string(), "Bash".to_string()],
        system_prompt: Some("You are a searcher agent".to_string()),
        append_system_prompt: None,
        agent_context: None,
        parent: Some(parent),
        is_subagent: true,
    };

    assert!(config.is_subagent);
    assert!(config.parent.is_some());
}

// US-33: Codex Harness

#[test]
fn test_us33_codex_harness_kind() {
    let kind = HarnessKind::Codex;
    assert!(matches!(kind, HarnessKind::Codex));
}

// US-34: OpenCode Harness

#[test]
fn test_us34_opencode_harness_kind() {
    let kind = HarnessKind::OpenCode;
    assert!(matches!(kind, HarnessKind::OpenCode));
}

// Common harness tests

#[test]
fn test_harness_kind_display() {
    assert_eq!(format!("{:?}", HarnessKind::ClaudeCode), "ClaudeCode");
    assert_eq!(format!("{:?}", HarnessKind::Codex), "Codex");
    assert_eq!(format!("{:?}", HarnessKind::OpenCode), "OpenCode");
}

#[test]
fn test_response_chunk_text() {
    let chunk = ResponseChunk::Text("Hello world".to_string());
    if let ResponseChunk::Text(text) = chunk {
        assert_eq!(text, "Hello world");
    } else {
        panic!("Expected Text variant");
    }
}

#[test]
fn test_response_chunk_done() {
    let chunk = ResponseChunk::Done;
    assert!(matches!(chunk, ResponseChunk::Done));
}

#[test]
fn test_response_chunk_error() {
    let chunk = ResponseChunk::Error("Something went wrong".to_string());
    if let ResponseChunk::Error(msg) = chunk {
        assert!(msg.contains("wrong"));
    } else {
        panic!("Expected Error variant");
    }
}

#[test]
fn test_response_chunk_tool_call() {
    let tool = ToolCall {
        id: "call-123".to_string(),
        name: "Edit".to_string(),
        arguments: serde_json::json!({
            "file": "src/main.rs",
            "changes": "Add error handling"
        }),
    };

    let chunk = ResponseChunk::ToolCall(tool);
    if let ResponseChunk::ToolCall(tc) = chunk {
        assert_eq!(tc.name, "Edit");
        assert_eq!(tc.id, "call-123");
    } else {
        panic!("Expected ToolCall variant");
    }
}

#[test]
fn test_session_handle_structure() {
    let handle = SessionHandle {
        id: "session-abc".to_string(),
        harness: "claude-code".to_string(),
        model: "claude-3-opus".to_string(),
        parent: Some("parent-xyz".to_string()),
    };

    assert_eq!(handle.id, "session-abc");
    assert_eq!(handle.harness, "claude-code");
    assert!(handle.parent.is_some());
}

#[test]
fn test_session_handle_no_parent() {
    let handle = SessionHandle {
        id: "session-root".to_string(),
        harness: "opencode".to_string(),
        model: "grok".to_string(),
        parent: None,
    };

    assert!(handle.parent.is_none());
}

// Feature-gated real API tests would go here with #[cfg(feature = "real-llm")]
