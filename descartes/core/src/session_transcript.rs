//! Session transcript management for Descartes agents.
//!
//! Transcripts capture the full conversation history including user messages,
//! assistant responses, and tool calls for later review and debugging.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use uuid::Uuid;

/// A session transcript entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub timestamp: DateTime<Utc>,
    pub role: String, // "user", "assistant", "tool_call", "tool_result"
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_id: Option<String>,
}

/// Session transcript metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptMetadata {
    pub session_id: Uuid,
    pub started_at: DateTime<Utc>,
    pub ended_at: Option<DateTime<Utc>>,
    pub provider: String,
    pub model: String,
    pub task: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<Uuid>,
    pub is_sub_session: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_level: Option<String>,
}

/// Writer for session transcripts.
pub struct TranscriptWriter {
    path: PathBuf,
    metadata: TranscriptMetadata,
    entries: Vec<TranscriptEntry>,
}

impl TranscriptWriter {
    /// Create a new transcript writer.
    ///
    /// # Arguments
    /// * `sessions_dir` - Directory to store transcripts (created if needed)
    /// * `provider` - Provider name (e.g., "anthropic")
    /// * `model` - Model name (e.g., "claude-3-5-sonnet")
    /// * `task` - The task/prompt given to the agent
    /// * `parent_session_id` - Parent session ID if this is a sub-session
    /// * `tool_level` - Tool level used ("minimal", "orchestrator", "readonly")
    pub fn new(
        sessions_dir: &PathBuf,
        provider: &str,
        model: &str,
        task: &str,
        parent_session_id: Option<Uuid>,
        tool_level: Option<&str>,
    ) -> std::io::Result<Self> {
        let session_id = Uuid::new_v4();
        let started_at = Utc::now();
        let is_sub_session = parent_session_id.is_some();

        // Create sessions directory if needed
        fs::create_dir_all(sessions_dir)?;

        // Generate filename: YYYY-MM-DD-HH-MM-SS-{short_id}.json
        let filename = format!(
            "{}-{}.json",
            started_at.format("%Y-%m-%d-%H-%M-%S"),
            &session_id.to_string()[..8]
        );
        let path = sessions_dir.join(filename);

        let metadata = TranscriptMetadata {
            session_id,
            started_at,
            ended_at: None,
            provider: provider.to_string(),
            model: model.to_string(),
            task: task.to_string(),
            parent_session_id,
            is_sub_session,
            tool_level: tool_level.map(|s| s.to_string()),
        };

        Ok(Self {
            path,
            metadata,
            entries: Vec::new(),
        })
    }

    /// Add a user message to the transcript.
    pub fn add_user_message(&mut self, content: &str) {
        self.entries.push(TranscriptEntry {
            timestamp: Utc::now(),
            role: "user".to_string(),
            content: content.to_string(),
            tool_name: None,
            tool_id: None,
        });
    }

    /// Add an assistant message to the transcript.
    pub fn add_assistant_message(&mut self, content: &str) {
        self.entries.push(TranscriptEntry {
            timestamp: Utc::now(),
            role: "assistant".to_string(),
            content: content.to_string(),
            tool_name: None,
            tool_id: None,
        });
    }

    /// Add a tool call to the transcript.
    pub fn add_tool_call(&mut self, tool_name: &str, tool_id: &str, arguments: &str) {
        self.entries.push(TranscriptEntry {
            timestamp: Utc::now(),
            role: "tool_call".to_string(),
            content: arguments.to_string(),
            tool_name: Some(tool_name.to_string()),
            tool_id: Some(tool_id.to_string()),
        });
    }

    /// Add a tool result to the transcript.
    pub fn add_tool_result(&mut self, tool_id: &str, result: &str) {
        self.entries.push(TranscriptEntry {
            timestamp: Utc::now(),
            role: "tool_result".to_string(),
            content: result.to_string(),
            tool_name: None,
            tool_id: Some(tool_id.to_string()),
        });
    }

    /// Add a generic entry to the transcript.
    pub fn add_entry(
        &mut self,
        role: &str,
        content: &str,
        tool_name: Option<&str>,
        tool_id: Option<&str>,
    ) {
        self.entries.push(TranscriptEntry {
            timestamp: Utc::now(),
            role: role.to_string(),
            content: content.to_string(),
            tool_name: tool_name.map(|s| s.to_string()),
            tool_id: tool_id.map(|s| s.to_string()),
        });
    }

    /// Save the transcript to disk.
    pub fn save(&mut self) -> std::io::Result<PathBuf> {
        // Update ended_at
        self.metadata.ended_at = Some(Utc::now());

        let file = File::create(&self.path)?;
        let mut writer = BufWriter::new(file);

        // Write as JSON with metadata and entries
        let output = serde_json::json!({
            "metadata": self.metadata,
            "entries": self.entries,
        });

        serde_json::to_writer_pretty(&mut writer, &output)?;
        writer.flush()?;

        Ok(self.path.clone())
    }

    /// Get the session ID.
    pub fn session_id(&self) -> Uuid {
        self.metadata.session_id
    }

    /// Get the transcript path.
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get the number of entries.
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

/// Get the default sessions directory path.
pub fn default_sessions_dir() -> PathBuf {
    // Use .scud/sessions in current directory, or ~/.descartes/sessions
    let scud_sessions = PathBuf::from(".scud/sessions");
    if scud_sessions.parent().is_some_and(|p| p.exists()) {
        scud_sessions
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".descartes/sessions")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_transcript_writer_creation() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let writer = TranscriptWriter::new(
            &sessions_dir,
            "anthropic",
            "claude-3-5-sonnet",
            "test task",
            None,
            Some("minimal"),
        )
        .unwrap();

        assert!(!writer.session_id().is_nil());
        assert!(writer.path().to_string_lossy().contains(".json"));
        assert_eq!(writer.entry_count(), 0);
    }

    #[test]
    fn test_transcript_entries() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let mut writer = TranscriptWriter::new(
            &sessions_dir,
            "anthropic",
            "claude-3-5-sonnet",
            "test task",
            None,
            None,
        )
        .unwrap();

        writer.add_user_message("Hello");
        writer.add_assistant_message("Hi there!");
        writer.add_tool_call("bash", "call_1", r#"{"command": "ls"}"#);
        writer.add_tool_result("call_1", "file1.txt\nfile2.txt");

        assert_eq!(writer.entry_count(), 4);
    }

    #[test]
    fn test_transcript_save() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");

        let mut writer = TranscriptWriter::new(
            &sessions_dir,
            "anthropic",
            "claude-3-5-sonnet",
            "test task",
            None,
            Some("orchestrator"),
        )
        .unwrap();

        writer.add_user_message("Hello");
        writer.add_assistant_message("Hi!");

        let path = writer.save().unwrap();
        assert!(path.exists());

        // Verify JSON structure
        let content = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert!(json.get("metadata").is_some());
        assert!(json.get("entries").is_some());
        assert_eq!(json["entries"].as_array().unwrap().len(), 2);
        assert_eq!(json["metadata"]["provider"], "anthropic");
        assert_eq!(json["metadata"]["tool_level"], "orchestrator");
    }

    #[test]
    fn test_sub_session_transcript() {
        let temp_dir = TempDir::new().unwrap();
        let sessions_dir = temp_dir.path().join("sessions");
        let parent_id = Uuid::new_v4();

        let mut writer = TranscriptWriter::new(
            &sessions_dir,
            "claude",
            "claude-3-5-sonnet",
            "sub task",
            Some(parent_id),
            Some("minimal"),
        )
        .unwrap();

        writer.add_user_message("Do something");
        let path = writer.save().unwrap();

        let content = std::fs::read_to_string(&path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        assert_eq!(json["metadata"]["is_sub_session"], true);
        assert_eq!(
            json["metadata"]["parent_session_id"],
            parent_id.to_string()
        );
    }
}
