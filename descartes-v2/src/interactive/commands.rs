//! Slash command system
//!
//! Commands are context injectors and controllers that can be invoked
//! with `/command` syntax, similar to Claude Code and OpenCode.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::{Error, Result};

/// A registered command
#[derive(Debug, Clone)]
pub struct Command {
    /// Command name (without the /)
    pub name: String,
    /// Short description
    pub description: String,
    /// Command type
    pub kind: CommandKind,
    /// Aliases for this command
    pub aliases: Vec<String>,
}

/// Types of commands
#[derive(Debug, Clone)]
pub enum CommandKind {
    /// Load a skill/prompt and optionally start an agent
    Skill {
        /// Path to skill prompt file
        prompt_file: PathBuf,
        /// Agent category to use (if auto-starting)
        category: Option<String>,
        /// Auto-start agent after loading
        auto_start: bool,
    },
    /// Workflow transition command
    Transition {
        /// Target stage
        to_stage: String,
        /// Generate handoff from current stage
        generate_handoff: bool,
    },
    /// Control command (pause, cancel, etc.)
    Control(ControlAction),
    /// Context injection
    Context {
        /// Type of context to inject
        context_type: ContextType,
    },
    /// Built-in command with custom handler
    Builtin(String),
}

/// Control actions for agents
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlAction {
    /// Pause the current agent
    Pause,
    /// Resume a paused agent
    Resume,
    /// Cancel the current agent
    Cancel,
    /// Show status
    Status,
    /// Clear context
    Clear,
    /// Exit the session
    Exit,
}

/// Types of context that can be injected
#[derive(Debug, Clone)]
pub enum ContextType {
    /// Load from a file
    File(PathBuf),
    /// SCUD tasks
    ScudTasks,
    /// SCUD waves
    ScudWaves,
    /// Git diff
    GitDiff,
    /// Git status
    GitStatus,
    /// Custom command output
    Command(String),
    /// Previous handoff
    Handoff,
}

/// Parsed command invocation
#[derive(Debug, Clone)]
pub struct CommandInvocation {
    /// Command name
    pub name: String,
    /// Arguments after the command
    pub args: Vec<String>,
    /// Raw argument string
    pub raw_args: String,
}

impl CommandInvocation {
    /// Parse a command from input
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();

        // Must start with /
        if !input.starts_with('/') {
            return None;
        }

        let input = &input[1..]; // Remove leading /

        // Split into command and args
        let mut parts = input.splitn(2, char::is_whitespace);
        let name = parts.next()?.to_lowercase();
        let raw_args = parts.next().unwrap_or("").to_string();

        // Parse args (simple space-separated for now)
        let args: Vec<String> = if raw_args.is_empty() {
            Vec::new()
        } else {
            shell_words::split(&raw_args).unwrap_or_else(|_| {
                raw_args.split_whitespace().map(String::from).collect()
            })
        };

        Some(Self { name, args, raw_args })
    }

    /// Get first argument
    pub fn arg(&self, index: usize) -> Option<&str> {
        self.args.get(index).map(|s| s.as_str())
    }

    /// Check if a flag is present
    pub fn has_flag(&self, flag: &str) -> bool {
        self.args.iter().any(|a| a == flag || a == &format!("--{}", flag))
    }
}

/// Command registry
pub struct CommandRegistry {
    commands: HashMap<String, Command>,
    aliases: HashMap<String, String>,
}

impl CommandRegistry {
    /// Create a new registry with default commands
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
            aliases: HashMap::new(),
        };

        registry.register_defaults();
        registry
    }

    /// Register default commands
    fn register_defaults(&mut self) {
        // Control commands
        self.register(Command {
            name: "pause".to_string(),
            description: "Pause the current agent".to_string(),
            kind: CommandKind::Control(ControlAction::Pause),
            aliases: vec!["p".to_string()],
        });

        self.register(Command {
            name: "resume".to_string(),
            description: "Resume a paused agent".to_string(),
            kind: CommandKind::Control(ControlAction::Resume),
            aliases: vec!["r".to_string(), "continue".to_string()],
        });

        self.register(Command {
            name: "cancel".to_string(),
            description: "Cancel the current agent".to_string(),
            kind: CommandKind::Control(ControlAction::Cancel),
            aliases: vec!["stop".to_string(), "abort".to_string()],
        });

        self.register(Command {
            name: "status".to_string(),
            description: "Show current status".to_string(),
            kind: CommandKind::Control(ControlAction::Status),
            aliases: vec!["s".to_string()],
        });

        self.register(Command {
            name: "clear".to_string(),
            description: "Clear context".to_string(),
            kind: CommandKind::Control(ControlAction::Clear),
            aliases: vec![],
        });

        self.register(Command {
            name: "exit".to_string(),
            description: "Exit the session".to_string(),
            kind: CommandKind::Control(ControlAction::Exit),
            aliases: vec!["quit".to_string(), "q".to_string()],
        });

        // Transition commands
        self.register(Command {
            name: "handoff".to_string(),
            description: "Generate handoff for next stage".to_string(),
            kind: CommandKind::Transition {
                to_stage: String::new(), // Determined by args
                generate_handoff: true,
            },
            aliases: vec!["ho".to_string()],
        });

        // Context commands
        self.register(Command {
            name: "context".to_string(),
            description: "Load additional context".to_string(),
            kind: CommandKind::Context {
                context_type: ContextType::File(PathBuf::new()), // Determined by args
            },
            aliases: vec!["ctx".to_string(), "load".to_string()],
        });

        self.register(Command {
            name: "scud".to_string(),
            description: "Load SCUD task context".to_string(),
            kind: CommandKind::Context {
                context_type: ContextType::ScudTasks,
            },
            aliases: vec!["tasks".to_string()],
        });

        self.register(Command {
            name: "waves".to_string(),
            description: "Load SCUD wave context".to_string(),
            kind: CommandKind::Context {
                context_type: ContextType::ScudWaves,
            },
            aliases: vec![],
        });

        self.register(Command {
            name: "diff".to_string(),
            description: "Load git diff context".to_string(),
            kind: CommandKind::Context {
                context_type: ContextType::GitDiff,
            },
            aliases: vec![],
        });

        // Built-in commands
        self.register(Command {
            name: "help".to_string(),
            description: "Show available commands".to_string(),
            kind: CommandKind::Builtin("help".to_string()),
            aliases: vec!["?".to_string(), "h".to_string()],
        });

        self.register(Command {
            name: "skill".to_string(),
            description: "Load a skill prompt".to_string(),
            kind: CommandKind::Builtin("skill".to_string()),
            aliases: vec!["sk".to_string()],
        });
    }

    /// Register a command
    pub fn register(&mut self, command: Command) {
        // Register aliases
        for alias in &command.aliases {
            self.aliases.insert(alias.clone(), command.name.clone());
        }

        self.commands.insert(command.name.clone(), command);
    }

    /// Register a skill command
    pub fn register_skill(&mut self, name: &str, prompt_file: PathBuf, description: &str) {
        self.register(Command {
            name: name.to_string(),
            description: description.to_string(),
            kind: CommandKind::Skill {
                prompt_file,
                category: None,
                auto_start: false,
            },
            aliases: vec![],
        });
    }

    /// Get a command by name or alias
    pub fn get(&self, name: &str) -> Option<&Command> {
        let name = name.to_lowercase();

        // Check direct match
        if let Some(cmd) = self.commands.get(&name) {
            return Some(cmd);
        }

        // Check aliases
        if let Some(real_name) = self.aliases.get(&name) {
            return self.commands.get(real_name);
        }

        None
    }

    /// Resolve a command invocation
    pub fn resolve(&self, invocation: &CommandInvocation) -> Option<ResolvedCommand> {
        let command = self.get(&invocation.name)?;

        Some(ResolvedCommand {
            command: command.clone(),
            args: invocation.args.clone(),
            raw_args: invocation.raw_args.clone(),
        })
    }

    /// List all commands
    pub fn list(&self) -> Vec<&Command> {
        let mut commands: Vec<_> = self.commands.values().collect();
        commands.sort_by(|a, b| a.name.cmp(&b.name));
        commands
    }

    /// Format help text
    pub fn help(&self) -> String {
        let mut lines = vec!["Available commands:".to_string(), String::new()];

        for cmd in self.list() {
            let aliases = if cmd.aliases.is_empty() {
                String::new()
            } else {
                format!(" ({})", cmd.aliases.join(", "))
            };

            lines.push(format!("  /{}{} - {}", cmd.name, aliases, cmd.description));
        }

        lines.join("\n")
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// A resolved command ready for execution
#[derive(Debug, Clone)]
pub struct ResolvedCommand {
    pub command: Command,
    pub args: Vec<String>,
    pub raw_args: String,
}

impl ResolvedCommand {
    /// Get the control action if this is a control command
    pub fn as_control(&self) -> Option<ControlAction> {
        match &self.command.kind {
            CommandKind::Control(action) => Some(*action),
            _ => None,
        }
    }

    /// Get skill info if this is a skill command
    pub fn as_skill(&self) -> Option<(&PathBuf, Option<&str>, bool)> {
        match &self.command.kind {
            CommandKind::Skill { prompt_file, category, auto_start } => {
                Some((prompt_file, category.as_deref(), *auto_start))
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_command() {
        let inv = CommandInvocation::parse("/help").unwrap();
        assert_eq!(inv.name, "help");
        assert!(inv.args.is_empty());

        let inv = CommandInvocation::parse("/skill create_plan").unwrap();
        assert_eq!(inv.name, "skill");
        assert_eq!(inv.args, vec!["create_plan"]);

        let inv = CommandInvocation::parse("/context ./README.md --verbose").unwrap();
        assert_eq!(inv.name, "context");
        assert_eq!(inv.args, vec!["./README.md", "--verbose"]);
        assert!(inv.has_flag("verbose"));
    }

    #[test]
    fn test_not_a_command() {
        assert!(CommandInvocation::parse("hello world").is_none());
        assert!(CommandInvocation::parse("").is_none());
    }

    #[test]
    fn test_registry_aliases() {
        let registry = CommandRegistry::new();

        assert!(registry.get("pause").is_some());
        assert!(registry.get("p").is_some());
        assert!(registry.get("P").is_some()); // Case insensitive

        let pause = registry.get("p").unwrap();
        assert_eq!(pause.name, "pause");
    }
}
