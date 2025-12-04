//! Tool definitions for Descartes agents.
//!
//! These follow Pi's minimal approach: 4 core tools that are sufficient
//! for effective coding agents.

use crate::traits::{Tool, ToolParameters};
use serde_json::json;
use std::collections::HashMap;

/// Create the `read` tool definition.
/// Reads file contents. Supports text files and images.
pub fn read_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert(
        "path".to_string(),
        json!({
            "type": "string",
            "description": "Path to the file to read (relative or absolute)"
        }),
    );
    properties.insert(
        "offset".to_string(),
        json!({
            "type": "integer",
            "description": "Line number to start reading from (1-indexed, optional)"
        }),
    );
    properties.insert(
        "limit".to_string(),
        json!({
            "type": "integer",
            "description": "Maximum number of lines to read (optional)"
        }),
    );

    Tool {
        name: "read".to_string(),
        description: "Read the contents of a file. Supports text files and images (jpg, png, gif, webp). For text files, defaults to first 2000 lines. Use offset/limit for large files.".to_string(),
        parameters: ToolParameters {
            required: vec!["path".to_string()],
            properties,
        },
    }
}

/// Create the `write` tool definition.
/// Writes content to a file, creating directories as needed.
pub fn write_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert(
        "path".to_string(),
        json!({
            "type": "string",
            "description": "Path to the file to write (relative or absolute)"
        }),
    );
    properties.insert(
        "content".to_string(),
        json!({
            "type": "string",
            "description": "Content to write to the file"
        }),
    );

    Tool {
        name: "write".to_string(),
        description: "Write content to a file. Creates the file if it doesn't exist, overwrites if it does. Automatically creates parent directories.".to_string(),
        parameters: ToolParameters {
            required: vec!["path".to_string(), "content".to_string()],
            properties,
        },
    }
}

/// Create the `edit` tool definition.
/// Makes surgical edits by replacing exact text.
pub fn edit_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert(
        "path".to_string(),
        json!({
            "type": "string",
            "description": "Path to the file to edit (relative or absolute)"
        }),
    );
    properties.insert(
        "old_text".to_string(),
        json!({
            "type": "string",
            "description": "Exact text to find and replace (must match exactly including whitespace)"
        }),
    );
    properties.insert(
        "new_text".to_string(),
        json!({
            "type": "string",
            "description": "New text to replace the old text with"
        }),
    );

    Tool {
        name: "edit".to_string(),
        description: "Edit a file by replacing exact text. The old_text must match exactly (including whitespace). Use this for precise, surgical edits.".to_string(),
        parameters: ToolParameters {
            required: vec![
                "path".to_string(),
                "old_text".to_string(),
                "new_text".to_string(),
            ],
            properties,
        },
    }
}

/// Create the `bash` tool definition.
/// Executes bash commands in the working directory.
pub fn bash_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert(
        "command".to_string(),
        json!({
            "type": "string",
            "description": "Bash command to execute"
        }),
    );
    properties.insert(
        "timeout".to_string(),
        json!({
            "type": "integer",
            "description": "Timeout in seconds (optional, no default timeout)"
        }),
    );

    Tool {
        name: "bash".to_string(),
        description: "Execute a bash command in the current working directory. Returns stdout and stderr. Use for git, npm, make, and other CLI operations.".to_string(),
        parameters: ToolParameters {
            required: vec!["command".to_string()],
            properties,
        },
    }
}

/// Create the `spawn_session` tool definition.
/// Only available to orchestrator agents, NOT to spawned sub-sessions.
pub fn spawn_session_tool() -> Tool {
    let mut properties = HashMap::new();
    properties.insert(
        "task".to_string(),
        json!({
            "type": "string",
            "description": "The task/prompt to give to the spawned session"
        }),
    );
    properties.insert(
        "provider".to_string(),
        json!({
            "type": "string",
            "description": "Provider to use: 'claude', 'opencode', 'anthropic', 'openai', 'ollama'",
            "default": "claude"
        }),
    );
    properties.insert(
        "output_file".to_string(),
        json!({
            "type": "string",
            "description": "Optional path to save the session transcript"
        }),
    );
    properties.insert(
        "attachable".to_string(),
        json!({
            "type": "boolean",
            "description": "If true, creates an attach socket for TUI connection",
            "default": false
        }),
    );

    Tool {
        name: "spawn_session".to_string(),
        description: "Spawn a sub-session to handle a specific task. The sub-session's output streams to this session. Sub-sessions cannot spawn their own sub-sessions (no recursive agents). Use for code review, research, or delegating focused tasks.".to_string(),
        parameters: ToolParameters {
            required: vec!["task".to_string()],
            properties,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_tool() {
        let tool = read_tool();
        assert_eq!(tool.name, "read");
        assert!(tool.parameters.required.contains(&"path".to_string()));
        assert!(tool.parameters.properties.contains_key("path"));
        assert!(tool.parameters.properties.contains_key("offset"));
        assert!(tool.parameters.properties.contains_key("limit"));
    }

    #[test]
    fn test_write_tool() {
        let tool = write_tool();
        assert_eq!(tool.name, "write");
        assert!(tool.parameters.required.contains(&"path".to_string()));
        assert!(tool.parameters.required.contains(&"content".to_string()));
    }

    #[test]
    fn test_edit_tool() {
        let tool = edit_tool();
        assert_eq!(tool.name, "edit");
        assert!(tool.parameters.required.contains(&"path".to_string()));
        assert!(tool.parameters.required.contains(&"old_text".to_string()));
        assert!(tool.parameters.required.contains(&"new_text".to_string()));
    }

    #[test]
    fn test_bash_tool() {
        let tool = bash_tool();
        assert_eq!(tool.name, "bash");
        assert!(tool.parameters.required.contains(&"command".to_string()));
        assert!(tool.parameters.properties.contains_key("timeout"));
    }

    #[test]
    fn test_spawn_session_tool() {
        let tool = spawn_session_tool();
        assert_eq!(tool.name, "spawn_session");
        assert!(tool.parameters.required.contains(&"task".to_string()));
        assert!(tool.parameters.properties.contains_key("provider"));
        assert!(tool.parameters.properties.contains_key("output_file"));
        assert!(tool.parameters.properties.contains_key("attachable"));
    }
}
