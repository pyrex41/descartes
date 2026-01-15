//! Transcript System Tests (US-37 to US-38)
//!
//! Tests for transcript recording and replay:
//! - US-37: Transcript Recording
//! - US-38: Session Replay

use crate::e2e::fixtures::TestProject;
use descartes::transcript::{Transcript, TranscriptEntry, TranscriptMetrics};

// US-37: Transcript Recording

#[test]
fn test_us37_transcript_project_has_directory() {
    let project = TestProject::transcript_project();

    let transcripts_dir = project.path.join(".descartes/transcripts");
    assert!(transcripts_dir.exists());
    assert!(transcripts_dir.is_dir());
}

#[test]
fn test_us37_transcript_file_exists() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    assert!(transcript_file.exists());
}

#[test]
fn test_us37_transcript_contains_meta() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    let content = std::fs::read_to_string(&transcript_file).unwrap();

    assert!(content.contains("@meta"));
    assert!(content.contains("task_id"));
    assert!(content.contains("agent_type"));
    assert!(content.contains("status"));
}

#[test]
fn test_us37_transcript_contains_messages() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    let content = std::fs::read_to_string(&transcript_file).unwrap();

    assert!(content.contains("@messages"));
    // H = Human, A = Assistant
    assert!(content.contains("H |"));
    assert!(content.contains("A |"));
}

#[test]
fn test_us37_transcript_contains_metrics() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    let content = std::fs::read_to_string(&transcript_file).unwrap();

    assert!(content.contains("@metrics"));
    assert!(content.contains("tokens_"));
}

#[test]
fn test_us37_transcript_entry_user() {
    let entry = TranscriptEntry::User("Implement the feature".to_string());
    if let TranscriptEntry::User(msg) = entry {
        assert!(msg.contains("feature"));
    } else {
        panic!("Expected User variant");
    }
}

#[test]
fn test_us37_transcript_entry_assistant() {
    let entry = TranscriptEntry::Assistant("I'll implement that now.".to_string());
    if let TranscriptEntry::Assistant(msg) = entry {
        assert!(msg.contains("implement"));
    } else {
        panic!("Expected Assistant variant");
    }
}

#[test]
fn test_us37_transcript_entry_tool_call() {
    let entry = TranscriptEntry::ToolCall {
        name: "Edit".to_string(),
        arguments: serde_json::json!({"file": "src/main.rs"}),
        id: "call-123".to_string(),
    };

    if let TranscriptEntry::ToolCall { name, id, .. } = entry {
        assert_eq!(name, "Edit");
        assert_eq!(id, "call-123");
    } else {
        panic!("Expected ToolCall variant");
    }
}

#[test]
fn test_us37_transcript_entry_tool_result() {
    let entry = TranscriptEntry::ToolResult {
        tool_call_id: "call-123".to_string(),
        content: "File edited successfully".to_string(),
        success: true,
    };

    if let TranscriptEntry::ToolResult { success, content, .. } = entry {
        assert!(success);
        assert!(content.contains("successfully"));
    } else {
        panic!("Expected ToolResult variant");
    }
}

#[test]
fn test_us37_transcript_metrics_structure() {
    let metrics = TranscriptMetrics {
        tokens_in: 1500,
        tokens_out: 500,
        duration_ms: 5000,
        tools_called: 3,
    };

    assert_eq!(metrics.tokens_in, 1500);
    assert_eq!(metrics.tokens_out, 500);
    assert_eq!(metrics.tools_called, 3);
    assert_eq!(metrics.duration_ms, 5000);
}

// US-38: Session Replay

#[test]
fn test_us38_transcript_file_parseable() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    let content = std::fs::read_to_string(&transcript_file).unwrap();

    // Verify structure is parseable
    assert!(content.starts_with("# Descartes Transcript"));
    assert!(content.contains("@meta {"));
    assert!(content.contains("@messages"));
    assert!(content.contains("@metrics {"));
}

#[test]
fn test_us38_extract_task_id_from_transcript() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    let content = std::fs::read_to_string(&transcript_file).unwrap();

    // Extract task_id from @meta section
    let task_id_line = content.lines()
        .find(|line| line.trim().starts_with("task_id"))
        .expect("Should have task_id in meta");

    assert!(task_id_line.contains("1"));
}

#[test]
fn test_us38_extract_agent_type_from_transcript() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    let content = std::fs::read_to_string(&transcript_file).unwrap();

    let agent_line = content.lines()
        .find(|line| line.trim().starts_with("agent_type"))
        .expect("Should have agent_type in meta");

    assert!(agent_line.contains("Builder"));
}

#[test]
fn test_us38_extract_status_from_transcript() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    let content = std::fs::read_to_string(&transcript_file).unwrap();

    let status_line = content.lines()
        .find(|line| line.trim().starts_with("status"))
        .expect("Should have status in meta");

    assert!(status_line.contains("completed"));
}

#[test]
fn test_us38_count_messages_in_transcript() {
    let project = TestProject::transcript_project();

    let transcript_file = project.path.join(".descartes/transcripts/test-session-001.scg");
    let content = std::fs::read_to_string(&transcript_file).unwrap();

    // Count message lines (H | or A | format)
    let message_count = content.lines()
        .filter(|line| {
            let trimmed = line.trim();
            trimmed.starts_with("H |") || trimmed.starts_with("A |")
        })
        .count();

    assert!(message_count >= 2); // At least human and assistant messages
}

#[test]
fn test_us38_transcript_entry_error() {
    let entry = TranscriptEntry::Error("Connection timeout".to_string());
    if let TranscriptEntry::Error(msg) = entry {
        assert!(msg.contains("timeout"));
    } else {
        panic!("Expected Error variant");
    }
}

#[test]
fn test_us38_transcript_entry_subagent_spawn() {
    let entry = TranscriptEntry::SubagentSpawn {
        category: "searcher".to_string(),
        prompt: "Find all error handlers".to_string(),
    };

    if let TranscriptEntry::SubagentSpawn { category, prompt } = entry {
        assert_eq!(category, "searcher");
        assert!(prompt.contains("error"));
    } else {
        panic!("Expected SubagentSpawn variant");
    }
}
