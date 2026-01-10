//! SCG (SCUD Compact Graph) format for transcripts
//!
//! Token-efficient format that's ~75% smaller than JSON while remaining
//! human-readable and grep-friendly.
//!
//! # Format
//!
//! ```scg
//! @transcript
//! id: "session_2025-01-10_001"
//! harness: "claude-code"
//! model: "opus"
//! started: 2025-01-10T14:30:00Z
//! parent: null
//!
//! @messages
//! 1:user "Find all rate limiting implementations"
//! 2:assistant "I'll search for rate limiting code."
//! 3:tool:bash "rg 'rate.?limit' --type rust"
//! 4:result:bash "src/middleware/rate_limit.rs:15:..."
//!
//! @subagents
//! sub1:searcher "find tests" -> session_002
//!
//! @metrics
//! tokens_in: 1240
//! tokens_out: 856
//! duration_ms: 3420
//! ```

use chrono::{DateTime, Utc};

use super::{Transcript, TranscriptEntry, TranscriptMetrics, SubagentRef};
use crate::{Error, Result};

/// Convert a transcript to SCG format
pub fn to_scg(transcript: &Transcript) -> String {
    let mut out = String::new();

    // Header section
    out.push_str("@transcript\n");
    out.push_str(&format!("id: \"{}\"\n", transcript.id()));
    out.push_str(&format!("harness: \"{}\"\n", transcript.harness));
    out.push_str(&format!("model: \"{}\"\n", transcript.model));
    out.push_str(&format!("started: {}\n", transcript.started.to_rfc3339()));

    if let Some(ref parent) = transcript.parent {
        out.push_str(&format!("parent: \"{}\"\n", parent));
    } else {
        out.push_str("parent: null\n");
    }

    if let Some(ref category) = transcript.category {
        out.push_str(&format!("category: \"{}\"\n", category));
    }

    out.push('\n');

    // Messages section
    out.push_str("@messages\n");
    for (i, entry) in transcript.entries.iter().enumerate() {
        let line_num = i + 1;
        match entry {
            TranscriptEntry::User(msg) => {
                out.push_str(&format!("{}:user {}\n", line_num, quote_string(msg)));
            }
            TranscriptEntry::Assistant(msg) => {
                out.push_str(&format!("{}:assistant {}\n", line_num, quote_string(msg)));
            }
            TranscriptEntry::ToolCall { name, arguments, id } => {
                let args_str = serde_json::to_string(arguments).unwrap_or_default();
                out.push_str(&format!(
                    "{}:tool:{} {} # {}\n",
                    line_num, name, quote_string(&args_str), id
                ));
            }
            TranscriptEntry::ToolResult {
                tool_call_id,
                content,
                success,
            } => {
                let status = if *success { "ok" } else { "err" };
                out.push_str(&format!(
                    "{}:result:{} {} # {}\n",
                    line_num,
                    status,
                    quote_string(content),
                    tool_call_id
                ));
            }
            TranscriptEntry::SubagentSpawn { category, prompt } => {
                out.push_str(&format!(
                    "{}:spawn:{} {}\n",
                    line_num,
                    category,
                    quote_string(prompt)
                ));
            }
            TranscriptEntry::Error(e) => {
                out.push_str(&format!("{}:error {}\n", line_num, quote_string(e)));
            }
        }
    }

    out.push('\n');

    // Subagents section
    if !transcript.subagents.is_empty() {
        out.push_str("@subagents\n");
        for (i, sub) in transcript.subagents.iter().enumerate() {
            let status = if sub.completed { "done" } else { "pending" };
            out.push_str(&format!(
                "sub{}:{}:{} {} -> {}\n",
                i + 1,
                sub.category,
                status,
                quote_string(&sub.prompt),
                sub.session_id
            ));
        }
        out.push('\n');
    }

    // Metrics section
    out.push_str("@metrics\n");
    out.push_str(&format!("tokens_in: {}\n", transcript.metrics.tokens_in));
    out.push_str(&format!("tokens_out: {}\n", transcript.metrics.tokens_out));
    out.push_str(&format!("duration_ms: {}\n", transcript.metrics.duration_ms));
    out.push_str(&format!("tools_called: {}\n", transcript.metrics.tools_called));

    out
}

/// Quote a string for SCG format (handles multiline)
fn quote_string(s: &str) -> String {
    if s.contains('\n') || s.contains('"') {
        // Use triple-quote for multiline
        format!("\"\"\"{}\"\"\"", s.replace("\"\"\"", "\\\"\\\"\\\""))
    } else {
        format!("\"{}\"", s)
    }
}

/// Parser for SCG format
pub struct ScgParser<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> ScgParser<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn peek(&self) -> Option<char> {
        self.input[self.pos..].chars().next()
    }

    fn advance(&mut self) {
        if let Some(c) = self.peek() {
            self.pos += c.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(' ' | '\t')) {
            self.advance();
        }
    }

    fn skip_line(&mut self) {
        while matches!(self.peek(), Some(c) if c != '\n') {
            self.advance();
        }
        if self.peek() == Some('\n') {
            self.advance();
        }
    }

    fn parse_until(&mut self, end: char) -> String {
        let start = self.pos;
        while matches!(self.peek(), Some(c) if c != end) {
            self.advance();
        }
        self.input[start..self.pos].to_string()
    }

    fn parse_quoted_string(&mut self) -> Result<String> {
        self.skip_whitespace();

        if self.peek() != Some('"') {
            return Err(Error::ScgParse("Expected quoted string".to_string()));
        }
        self.advance();

        // Check for triple-quote
        if self.input[self.pos..].starts_with("\"\"") {
            self.advance();
            self.advance();
            // Parse until closing triple-quote
            let start = self.pos;
            while !self.input[self.pos..].starts_with("\"\"\"") && self.pos < self.input.len() {
                self.advance();
            }
            let content = self.input[start..self.pos].to_string();
            // Skip closing quotes
            self.advance();
            self.advance();
            self.advance();
            Ok(content)
        } else {
            // Single line string
            let content = self.parse_until('"');
            self.advance(); // Skip closing quote
            Ok(content)
        }
    }

    fn parse_key_value(&mut self) -> Result<(String, String)> {
        let key = self.parse_until(':');
        if self.peek() != Some(':') {
            return Err(Error::ScgParse("Expected ':'".to_string()));
        }
        self.advance();
        self.skip_whitespace();

        let value = if self.peek() == Some('"') {
            self.parse_quoted_string()?
        } else {
            self.parse_until('\n').trim().to_string()
        };

        Ok((key, value))
    }
}

/// Parse SCG format into a Transcript
pub fn parse_scg(input: &str) -> Result<Transcript> {
    let mut transcript = Transcript::new();
    let mut current_section = "";

    for line in input.lines() {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Section headers
        if line.starts_with('@') {
            current_section = line.trim_start_matches('@');
            continue;
        }

        match current_section {
            "transcript" => {
                // Parse header key-value pairs
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim();
                    let value = value.trim().trim_matches('"');
                    match key {
                        "id" => transcript.id = value.to_string(),
                        "harness" => transcript.harness = value.to_string(),
                        "model" => transcript.model = value.to_string(),
                        "started" => {
                            if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                                transcript.started = dt.with_timezone(&Utc);
                            }
                        }
                        "parent" => {
                            if value != "null" {
                                transcript.parent = Some(value.to_string());
                            }
                        }
                        "category" => {
                            transcript.category = Some(value.to_string());
                        }
                        _ => {}
                    }
                }
            }
            "messages" => {
                // Parse message entries: N:type "content"
                if let Some((prefix, rest)) = line.split_once(' ') {
                    let parts: Vec<&str> = prefix.split(':').collect();
                    if parts.len() >= 2 {
                        let msg_type = parts[1];
                        let content = rest.trim().trim_matches('"');

                        match msg_type {
                            "user" => {
                                transcript.entries.push(TranscriptEntry::User(content.to_string()));
                            }
                            "assistant" => {
                                transcript
                                    .entries
                                    .push(TranscriptEntry::Assistant(content.to_string()));
                            }
                            "error" => {
                                transcript.entries.push(TranscriptEntry::Error(content.to_string()));
                            }
                            t if t.starts_with("tool:") => {
                                let tool_name = t.trim_start_matches("tool:");
                                let (args_str, id) = if let Some((a, comment)) = rest.split_once('#') {
                                    (a.trim().trim_matches('"'), comment.trim())
                                } else {
                                    (rest.trim().trim_matches('"'), "")
                                };
                                let arguments: serde_json::Value =
                                    serde_json::from_str(args_str).unwrap_or(serde_json::Value::Null);
                                transcript.entries.push(TranscriptEntry::ToolCall {
                                    name: tool_name.to_string(),
                                    arguments,
                                    id: id.to_string(),
                                });
                            }
                            t if t.starts_with("result:") => {
                                let status = t.trim_start_matches("result:");
                                let (content_str, id) = if let Some((c, comment)) = rest.split_once('#') {
                                    (c.trim().trim_matches('"'), comment.trim())
                                } else {
                                    (rest.trim().trim_matches('"'), "")
                                };
                                transcript.entries.push(TranscriptEntry::ToolResult {
                                    tool_call_id: id.to_string(),
                                    content: content_str.to_string(),
                                    success: status == "ok",
                                });
                            }
                            t if t.starts_with("spawn:") => {
                                let category = t.trim_start_matches("spawn:");
                                transcript.entries.push(TranscriptEntry::SubagentSpawn {
                                    category: category.to_string(),
                                    prompt: content.to_string(),
                                });
                            }
                            _ => {}
                        }
                    }
                }
            }
            "subagents" => {
                // Parse subagent refs: subN:category:status "prompt" -> session_id
                if let Some((prefix, rest)) = line.split_once(' ') {
                    let parts: Vec<&str> = prefix.split(':').collect();
                    if parts.len() >= 3 {
                        let category = parts[1];
                        let status = parts[2];

                        if let Some((prompt_part, session_id)) = rest.split_once("->") {
                            let prompt = prompt_part.trim().trim_matches('"');
                            transcript.subagents.push(SubagentRef {
                                session_id: session_id.trim().to_string(),
                                category: category.to_string(),
                                prompt: prompt.to_string(),
                                completed: status == "done",
                            });
                        }
                    }
                }
            }
            "metrics" => {
                // Parse metrics key-value pairs
                if let Some((key, value)) = line.split_once(':') {
                    let key = key.trim();
                    let value = value.trim();
                    match key {
                        "tokens_in" => {
                            transcript.metrics.tokens_in = value.parse().unwrap_or(0);
                        }
                        "tokens_out" => {
                            transcript.metrics.tokens_out = value.parse().unwrap_or(0);
                        }
                        "duration_ms" => {
                            transcript.metrics.duration_ms = value.parse().unwrap_or(0);
                        }
                        "tools_called" => {
                            transcript.metrics.tools_called = value.parse().unwrap_or(0);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    Ok(transcript)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip() {
        let mut transcript = Transcript::new()
            .with_harness("claude-code")
            .with_model("opus");

        transcript.record_user_message("Hello, world!");
        transcript.record_assistant_message("Hi there!");
        transcript.finalize();

        let scg = to_scg(&transcript);
        let parsed = parse_scg(&scg).unwrap();

        assert_eq!(parsed.harness, "claude-code");
        assert_eq!(parsed.model, "opus");
        assert_eq!(parsed.entries.len(), 2);
    }

    #[test]
    fn test_multiline_string() {
        let s = "line1\nline2\nline3";
        let quoted = quote_string(s);
        assert!(quoted.starts_with("\"\"\""));
        assert!(quoted.ends_with("\"\"\""));
    }
}
