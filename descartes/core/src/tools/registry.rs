//! Tool registry for managing tool sets by capability level.
//!
//! This module provides a way to get the appropriate tool set based on
//! the agent's role (orchestrator vs sub-session).

use super::definitions::*;
use crate::traits::Tool;

/// Tool capability levels for agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolLevel {
    /// Minimal tools: read, write, edit, bash
    /// Used for sub-sessions that cannot spawn further agents
    Minimal,
    /// Orchestrator tools: minimal + spawn_session
    /// Used for top-level agents that can delegate work
    Orchestrator,
    /// Read-only tools: read, bash (with restrictions)
    /// Used for exploration/planning without modifications
    ReadOnly,
}

/// Get the tools for a given capability level.
pub fn get_tools(level: ToolLevel) -> Vec<Tool> {
    match level {
        ToolLevel::Minimal => vec![read_tool(), write_tool(), edit_tool(), bash_tool()],
        ToolLevel::Orchestrator => vec![
            read_tool(),
            write_tool(),
            edit_tool(),
            bash_tool(),
            spawn_session_tool(),
        ],
        ToolLevel::ReadOnly => vec![
            read_tool(),
            bash_tool(), // For ls, grep, find, git status, etc.
        ],
    }
}

/// Get minimal system prompt for coding agents.
/// Pi-style: ~200 tokens, not 10,000.
pub fn minimal_system_prompt() -> &'static str {
    r#"You are an expert coding assistant. You help users with coding tasks by reading files, executing commands, editing code, and writing new files.

Available tools:
- read: Read file contents
- bash: Execute bash commands
- edit: Make surgical edits to files
- write: Create or overwrite files

Guidelines:
- Use bash for file operations like ls, grep, find
- Use read to examine files before editing
- Use edit for precise changes (old text must match exactly)
- Use write only for new files or complete rewrites
- Be concise in your responses
- Show file paths clearly when working with files"#
}

/// Get orchestrator system prompt (includes spawn_session).
pub fn orchestrator_system_prompt() -> &'static str {
    r#"You are an expert coding assistant with the ability to delegate tasks to sub-sessions.

Available tools:
- read: Read file contents
- bash: Execute bash commands
- edit: Make surgical edits to files
- write: Create or overwrite files
- spawn_session: Spawn a sub-session for focused tasks

Guidelines:
- Use bash for file operations like ls, grep, find
- Use read to examine files before editing
- Use edit for precise changes (old text must match exactly)
- Use write only for new files or complete rewrites
- Use spawn_session for code review, research, or focused sub-tasks
- Sub-sessions stream their output to you and save transcripts
- Be concise in your responses"#
}

/// Get read-only system prompt for exploration mode.
pub fn readonly_system_prompt() -> &'static str {
    r#"You are an expert coding assistant in exploration mode. You can read files and run read-only commands, but cannot modify files.

Available tools:
- read: Read file contents
- bash: Execute bash commands (read-only operations only)

Guidelines:
- Use bash for ls, grep, find, git status, git log, etc.
- Use read to examine files
- Do not suggest file modifications in this mode
- Focus on understanding and explaining the codebase
- Be concise in your responses"#
}

/// Get the appropriate system prompt for a tool level.
pub fn get_system_prompt(level: ToolLevel) -> &'static str {
    match level {
        ToolLevel::Minimal => minimal_system_prompt(),
        ToolLevel::Orchestrator => orchestrator_system_prompt(),
        ToolLevel::ReadOnly => readonly_system_prompt(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minimal_tools() {
        let tools = get_tools(ToolLevel::Minimal);
        assert_eq!(tools.len(), 4);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"write"));
        assert!(names.contains(&"edit"));
        assert!(names.contains(&"bash"));
        assert!(!names.contains(&"spawn_session"));
    }

    #[test]
    fn test_orchestrator_tools() {
        let tools = get_tools(ToolLevel::Orchestrator);
        assert_eq!(tools.len(), 5);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"write"));
        assert!(names.contains(&"edit"));
        assert!(names.contains(&"bash"));
        assert!(names.contains(&"spawn_session"));
    }

    #[test]
    fn test_readonly_tools() {
        let tools = get_tools(ToolLevel::ReadOnly);
        assert_eq!(tools.len(), 2);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"bash"));
        assert!(!names.contains(&"write"));
        assert!(!names.contains(&"edit"));
    }

    #[test]
    fn test_system_prompts_not_empty() {
        assert!(!minimal_system_prompt().is_empty());
        assert!(!orchestrator_system_prompt().is_empty());
        assert!(!readonly_system_prompt().is_empty());
    }

    #[test]
    fn test_system_prompt_for_level() {
        assert_eq!(get_system_prompt(ToolLevel::Minimal), minimal_system_prompt());
        assert_eq!(
            get_system_prompt(ToolLevel::Orchestrator),
            orchestrator_system_prompt()
        );
        assert_eq!(get_system_prompt(ToolLevel::ReadOnly), readonly_system_prompt());
    }

    #[test]
    fn test_orchestrator_prompt_mentions_spawn() {
        let prompt = orchestrator_system_prompt();
        assert!(prompt.contains("spawn_session"));
    }

    #[test]
    fn test_minimal_prompt_no_spawn() {
        let prompt = minimal_system_prompt();
        assert!(!prompt.contains("spawn_session"));
    }
}
