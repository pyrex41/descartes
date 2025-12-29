//! Tool registry for managing tool sets by capability level.
//!
//! This module provides a way to get the appropriate tool set based on
//! the agent's role (orchestrator vs sub-session).

use super::definitions::*;
use crate::traits::Tool;

/// Tool capability levels for agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// Researcher tools: read, bash (read-only)
    /// Specialized for codebase research with focused prompts
    Researcher,
    /// Planner tools: read, bash, write (to thoughts only)
    /// Used for planning and documentation tasks
    Planner,
    /// Lisp developer tools: swank_eval, swank_compile, swank_inspect, swank_restart + read, bash
    /// Used for live Lisp development with SBCL/Swank
    LispDeveloper,
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
        ToolLevel::Researcher => vec![
            read_tool(),
            bash_tool(), // Same as ReadOnly but with researcher-focused prompt
        ],
        ToolLevel::Planner => vec![
            read_tool(),
            write_tool(), // Can write to thoughts/plans
            bash_tool(),
        ],
        ToolLevel::LispDeveloper => vec![
            swank_eval_tool(),
            swank_compile_tool(),
            swank_inspect_tool(),
            swank_restart_tool(),
            read_tool(),  // For reading Lisp source files
            bash_tool(),  // For running shell commands
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

/// Get researcher system prompt for codebase research tasks.
pub fn researcher_system_prompt() -> &'static str {
    r#"You are a codebase researcher. Your role is to explore and understand code structure, patterns, and implementations.

Available tools:
- read: Read file contents
- bash: Execute bash commands (read-only: ls, grep, find, git)

Guidelines:
- Use bash for file discovery: ls, find, grep, git log
- Use read to examine file contents in detail
- Focus on finding patterns, understanding architecture
- Document your findings clearly
- Report file locations and code snippets when relevant
- Be thorough but efficient in your exploration"#
}

/// Get planner system prompt for planning and documentation tasks.
pub fn planner_system_prompt() -> &'static str {
    r#"You are a planning assistant. Your role is to create implementation plans and documentation.

Available tools:
- read: Read file contents
- write: Write files (use for plans and documentation)
- bash: Execute bash commands (read-only operations)

Guidelines:
- Use bash for understanding current state: ls, grep, find, git
- Use read to examine existing code and documentation
- Use write to create plans and documentation files
- Focus on clear, actionable implementation steps
- Consider dependencies and order of operations
- Be specific about file locations and changes needed"#
}

/// Get Lisp developer system prompt for live Lisp development.
pub fn lisp_developer_system_prompt() -> &'static str {
    r#"You are a Lisp developer with access to a live SBCL runtime via Swank.

Available tools:
- swank_eval: Evaluate Lisp expressions in the live runtime
- swank_compile: Compile Lisp code (better diagnostics for defun, defclass, etc.)
- swank_inspect: Inspect Lisp objects to see their structure
- swank_restart: Invoke a debugger restart when errors occur
- read: Read source files
- bash: Execute shell commands

Guidelines:
- Use swank_eval for interactive exploration and testing
- Use swank_compile for defining functions, classes, and macros
- When an error occurs, you'll see available restarts - use swank_restart to choose one
- Restart index 0 is typically ABORT (return to top level)
- Use read to examine Lisp source files before modifying
- The runtime persists state between evaluations - defined functions remain available
- Package defaults to CL-USER, specify :package for other packages"#
}

/// Get the appropriate system prompt for a tool level.
pub fn get_system_prompt(level: ToolLevel) -> &'static str {
    match level {
        ToolLevel::Minimal => minimal_system_prompt(),
        ToolLevel::Orchestrator => orchestrator_system_prompt(),
        ToolLevel::ReadOnly => readonly_system_prompt(),
        ToolLevel::Researcher => researcher_system_prompt(),
        ToolLevel::Planner => planner_system_prompt(),
        ToolLevel::LispDeveloper => lisp_developer_system_prompt(),
    }
}

/// Parse a tool level from a string.
/// Supports various formats: "minimal", "read-only", "readonly", "orchestrator", etc.
pub fn parse_tool_level(s: &str) -> Option<ToolLevel> {
    match s.to_lowercase().replace('-', "").replace('_', "").as_str() {
        "minimal" => Some(ToolLevel::Minimal),
        "orchestrator" | "full" => Some(ToolLevel::Orchestrator),
        "readonly" | "read" => Some(ToolLevel::ReadOnly),
        "researcher" | "research" => Some(ToolLevel::Researcher),
        "planner" | "plan" => Some(ToolLevel::Planner),
        "lispdeveloper" | "lisp" => Some(ToolLevel::LispDeveloper),
        _ => None,
    }
}

/// Convert a tool level to Claude Code's --allowedTools format.
/// This returns the tool names that should be allowed for the given level.
pub fn tool_level_to_allowed_tools(level: ToolLevel) -> String {
    let tools = get_tools(level);
    let tool_names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
    tool_names.join(",")
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
    fn test_researcher_tools() {
        let tools = get_tools(ToolLevel::Researcher);
        assert_eq!(tools.len(), 2);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"bash"));
        assert!(!names.contains(&"write"));
    }

    #[test]
    fn test_planner_tools() {
        let tools = get_tools(ToolLevel::Planner);
        assert_eq!(tools.len(), 3);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"read"));
        assert!(names.contains(&"write"));
        assert!(names.contains(&"bash"));
        assert!(!names.contains(&"edit"));
    }

    #[test]
    fn test_lisp_developer_tools() {
        let tools = get_tools(ToolLevel::LispDeveloper);
        assert_eq!(tools.len(), 6);

        let names: Vec<&str> = tools.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"swank_eval"));
        assert!(names.contains(&"swank_compile"));
        assert!(names.contains(&"swank_inspect"));
        assert!(names.contains(&"swank_restart"));
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
        assert!(!researcher_system_prompt().is_empty());
        assert!(!planner_system_prompt().is_empty());
        assert!(!lisp_developer_system_prompt().is_empty());
    }

    #[test]
    fn test_system_prompt_for_level() {
        assert_eq!(get_system_prompt(ToolLevel::Minimal), minimal_system_prompt());
        assert_eq!(
            get_system_prompt(ToolLevel::Orchestrator),
            orchestrator_system_prompt()
        );
        assert_eq!(get_system_prompt(ToolLevel::ReadOnly), readonly_system_prompt());
        assert_eq!(get_system_prompt(ToolLevel::Researcher), researcher_system_prompt());
        assert_eq!(get_system_prompt(ToolLevel::Planner), planner_system_prompt());
        assert_eq!(get_system_prompt(ToolLevel::LispDeveloper), lisp_developer_system_prompt());
    }

    #[test]
    fn test_lisp_developer_prompt_mentions_swank() {
        let prompt = lisp_developer_system_prompt();
        assert!(prompt.contains("swank_eval"));
        assert!(prompt.contains("SBCL"));
        assert!(prompt.contains("restart"));
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

    #[test]
    fn test_researcher_prompt_mentions_codebase() {
        let prompt = researcher_system_prompt();
        assert!(prompt.contains("codebase"));
    }

    #[test]
    fn test_planner_prompt_mentions_plans() {
        let prompt = planner_system_prompt();
        assert!(prompt.contains("plan"));
    }
}
