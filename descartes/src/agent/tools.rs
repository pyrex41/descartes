//! Tool definitions for agents
//!
//! The 4-tool philosophy: read, write, edit, bash
//! Bash is the universal escape hatch.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Available tools for agents
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tool {
    /// Read file contents
    Read,
    /// Write new files
    Write,
    /// Edit existing files
    Edit,
    /// Execute shell commands (universal escape hatch)
    Bash,
}

impl Tool {
    /// Get the tool name as used in tool calls
    pub fn name(&self) -> &str {
        match self {
            Tool::Read => "read",
            Tool::Write => "write",
            Tool::Edit => "edit",
            Tool::Bash => "bash",
        }
    }

    /// Get a description of the tool
    pub fn description(&self) -> &str {
        match self {
            Tool::Read => "Read the contents of a file",
            Tool::Write => "Write content to a new file",
            Tool::Edit => "Edit an existing file with search/replace",
            Tool::Bash => "Execute a shell command",
        }
    }

    /// Parse a tool from its name
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "read" | "read_file" | "cat" => Some(Tool::Read),
            "write" | "write_file" | "create" => Some(Tool::Write),
            "edit" | "edit_file" | "modify" | "patch" => Some(Tool::Edit),
            "bash" | "shell" | "exec" | "run" | "command" => Some(Tool::Bash),
            _ => None,
        }
    }
}

impl std::fmt::Display for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl std::str::FromStr for Tool {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Tool::from_name(s).ok_or_else(|| format!("Unknown tool: {}", s))
    }
}

/// A set of tools available to an agent
#[derive(Debug, Clone, Default)]
pub struct ToolSet {
    tools: HashSet<Tool>,
}

impl ToolSet {
    /// Create an empty tool set
    pub fn new() -> Self {
        Self {
            tools: HashSet::new(),
        }
    }

    /// Create a tool set with all tools (orchestrator level)
    pub fn all() -> Self {
        let mut tools = HashSet::new();
        tools.insert(Tool::Read);
        tools.insert(Tool::Write);
        tools.insert(Tool::Edit);
        tools.insert(Tool::Bash);
        Self { tools }
    }

    /// Create a read-only tool set (searcher/analyzer level)
    pub fn read_only() -> Self {
        let mut tools = HashSet::new();
        tools.insert(Tool::Read);
        tools.insert(Tool::Bash); // For grep, find, etc.
        Self { tools }
    }

    /// Create a bash-only tool set (validator level)
    pub fn bash_only() -> Self {
        let mut tools = HashSet::new();
        tools.insert(Tool::Bash);
        Self { tools }
    }

    /// Add a tool to the set
    pub fn add(&mut self, tool: Tool) -> &mut Self {
        self.tools.insert(tool);
        self
    }

    /// Remove a tool from the set
    pub fn remove(&mut self, tool: Tool) -> &mut Self {
        self.tools.remove(&tool);
        self
    }

    /// Check if a tool is in the set
    pub fn has(&self, tool: Tool) -> bool {
        self.tools.contains(&tool)
    }

    /// Get all tools as a slice of names
    pub fn names(&self) -> Vec<String> {
        self.tools.iter().map(|t| t.name().to_string()).collect()
    }

    /// Check if this tool set can write files
    pub fn can_write(&self) -> bool {
        self.has(Tool::Write) || self.has(Tool::Edit)
    }

    /// Check if this tool set is read-only
    pub fn is_read_only(&self) -> bool {
        !self.can_write()
    }
}

impl FromIterator<Tool> for ToolSet {
    fn from_iter<I: IntoIterator<Item = Tool>>(iter: I) -> Self {
        Self {
            tools: iter.into_iter().collect(),
        }
    }
}

impl From<Vec<String>> for ToolSet {
    fn from(names: Vec<String>) -> Self {
        names
            .iter()
            .filter_map(|n| Tool::from_name(n))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_parsing() {
        assert_eq!(Tool::from_name("read"), Some(Tool::Read));
        assert_eq!(Tool::from_name("bash"), Some(Tool::Bash));
        assert_eq!(Tool::from_name("shell"), Some(Tool::Bash));
        assert_eq!(Tool::from_name("unknown"), None);
    }

    #[test]
    fn test_toolset_all() {
        let set = ToolSet::all();
        assert!(set.has(Tool::Read));
        assert!(set.has(Tool::Write));
        assert!(set.has(Tool::Edit));
        assert!(set.has(Tool::Bash));
        assert!(set.can_write());
    }

    #[test]
    fn test_toolset_readonly() {
        let set = ToolSet::read_only();
        assert!(set.has(Tool::Read));
        assert!(set.has(Tool::Bash));
        assert!(!set.has(Tool::Write));
        assert!(!set.has(Tool::Edit));
        assert!(set.is_read_only());
    }

    #[test]
    fn test_toolset_from_names() {
        let names = vec!["read".to_string(), "bash".to_string()];
        let set = ToolSet::from(names);
        assert!(set.has(Tool::Read));
        assert!(set.has(Tool::Bash));
        assert!(!set.has(Tool::Write));
    }
}
