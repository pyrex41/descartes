//! Agent Definition Loader
//!
//! Loads agent definitions from markdown files with YAML frontmatter.
//! Agent files are stored in `~/.descartes/agents/` and define:
//! - Agent name and description
//! - Model preferences
//! - Tool level (which tools the agent can use)
//! - System prompt (the markdown content after frontmatter)

use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;
use tracing::{debug, info, warn};

use crate::thoughts::parse_markdown_with_frontmatter;
use crate::tools::ToolLevel;

/// Errors that can occur during agent definition operations
#[derive(Error, Debug)]
pub enum AgentDefinitionError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to determine home directory")]
    NoHomeDirectory,

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Invalid agent definition: {0}")]
    InvalidDefinition(String),

    #[error("Parse error: {0}")]
    ParseError(String),
}

/// Result type for agent definition operations
pub type AgentDefinitionResult<T> = Result<T, AgentDefinitionError>;

/// Agent definition loaded from a markdown file.
///
/// The markdown file format is:
/// ```markdown
/// ---
/// name: agent-name
/// description: Short description
/// model: claude-3-sonnet
/// tool_level: readonly
/// tags: [tag1, tag2]
/// ---
///
/// System prompt content here...
/// ```
#[derive(Debug, Clone)]
pub struct AgentDefinition {
    /// Unique name for the agent (from filename or frontmatter)
    pub name: String,
    /// Short description of the agent's purpose
    pub description: String,
    /// Preferred model for this agent (optional)
    pub model: Option<String>,
    /// Tool level determining which tools the agent can use
    pub tool_level: ToolLevel,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// The system prompt (markdown content after frontmatter)
    pub system_prompt: String,
}

impl AgentDefinition {
    /// Parse an agent definition from markdown content.
    pub fn from_markdown(content: &str, default_name: &str) -> AgentDefinitionResult<Self> {
        let doc = parse_markdown_with_frontmatter(content)
            .map_err(|e| AgentDefinitionError::ParseError(e.to_string()))?;

        let name = doc
            .get("name")
            .cloned()
            .unwrap_or_else(|| default_name.to_string());

        let description = doc
            .get("description")
            .cloned()
            .unwrap_or_else(|| format!("{} agent", name));

        let model = doc.get("model").cloned();

        let tool_level = doc
            .get("tool_level")
            .map(|s| parse_tool_level(s))
            .unwrap_or(ToolLevel::ReadOnly);

        let tags = doc.get_list("tags");

        let system_prompt = doc.content;

        Ok(Self {
            name,
            description,
            model,
            tool_level,
            tags,
            system_prompt,
        })
    }
}

/// Parse a tool level from a string.
fn parse_tool_level(s: &str) -> ToolLevel {
    match s.to_lowercase().as_str() {
        "minimal" => ToolLevel::Minimal,
        "orchestrator" => ToolLevel::Orchestrator,
        "readonly" | "read_only" | "read-only" => ToolLevel::ReadOnly,
        "researcher" => ToolLevel::Researcher,
        "planner" => ToolLevel::Planner,
        "lisp-developer" | "lisp_developer" | "lispdeveloper" => ToolLevel::LispDeveloper,
        _ => {
            warn!("Unknown tool level '{}', defaulting to ReadOnly", s);
            ToolLevel::ReadOnly
        }
    }
}

// Default agent definitions bundled at compile time
const DEFAULT_AGENT_CODEBASE_LOCATOR: &str = include_str!("../../agents/codebase-locator.md");
const DEFAULT_AGENT_CODEBASE_ANALYZER: &str = include_str!("../../agents/codebase-analyzer.md");
const DEFAULT_AGENT_CODEBASE_PATTERN_FINDER: &str =
    include_str!("../../agents/codebase-pattern-finder.md");
const DEFAULT_AGENT_RESEARCHER: &str = include_str!("../../agents/researcher.md");
const DEFAULT_AGENT_PLANNER: &str = include_str!("../../agents/planner.md");
const DEFAULT_AGENT_LISP_DEVELOPER: &str = include_str!("../../agents/lisp-developer.md");

// Flow workflow agents
const DEFAULT_AGENT_FLOW_ORCHESTRATOR: &str = include_str!("../../agents/flow-orchestrator.md");
const DEFAULT_AGENT_FLOW_INGEST: &str = include_str!("../../agents/flow-ingest.md");
const DEFAULT_AGENT_FLOW_REVIEW_GRAPH: &str = include_str!("../../agents/flow-review-graph.md");
const DEFAULT_AGENT_FLOW_PLAN_TASKS: &str = include_str!("../../agents/flow-plan-tasks.md");
const DEFAULT_AGENT_FLOW_IMPLEMENT: &str = include_str!("../../agents/flow-implement.md");
const DEFAULT_AGENT_FLOW_QA: &str = include_str!("../../agents/flow-qa.md");
const DEFAULT_AGENT_FLOW_SUMMARIZE: &str = include_str!("../../agents/flow-summarize.md");

/// Default agents bundled with Descartes.
const DEFAULT_AGENTS: &[(&str, &str)] = &[
    ("codebase-locator.md", DEFAULT_AGENT_CODEBASE_LOCATOR),
    ("codebase-analyzer.md", DEFAULT_AGENT_CODEBASE_ANALYZER),
    (
        "codebase-pattern-finder.md",
        DEFAULT_AGENT_CODEBASE_PATTERN_FINDER,
    ),
    ("researcher.md", DEFAULT_AGENT_RESEARCHER),
    ("planner.md", DEFAULT_AGENT_PLANNER),
    ("lisp-developer.md", DEFAULT_AGENT_LISP_DEVELOPER),
    // Flow workflow agents
    ("flow-orchestrator.md", DEFAULT_AGENT_FLOW_ORCHESTRATOR),
    ("flow-ingest.md", DEFAULT_AGENT_FLOW_INGEST),
    ("flow-review-graph.md", DEFAULT_AGENT_FLOW_REVIEW_GRAPH),
    ("flow-plan-tasks.md", DEFAULT_AGENT_FLOW_PLAN_TASKS),
    ("flow-implement.md", DEFAULT_AGENT_FLOW_IMPLEMENT),
    ("flow-qa.md", DEFAULT_AGENT_FLOW_QA),
    ("flow-summarize.md", DEFAULT_AGENT_FLOW_SUMMARIZE),
];

/// Loader for agent definitions from the filesystem.
///
/// By default, agents are loaded from `~/.descartes/agents/`.
pub struct AgentDefinitionLoader {
    /// Directory containing agent definition files
    agents_dir: PathBuf,
}

impl AgentDefinitionLoader {
    /// Create a new loader with the default agents directory.
    ///
    /// This also ensures default agents are installed if not present.
    pub fn new() -> AgentDefinitionResult<Self> {
        let agents_dir = Self::default_agents_dir()?;
        let loader = Self { agents_dir };
        loader.ensure_directory()?;
        loader.ensure_default_agents()?;
        Ok(loader)
    }

    /// Create a new loader with a custom agents directory.
    ///
    /// Unlike `new()`, this does NOT install default agents.
    pub fn with_dir(agents_dir: PathBuf) -> AgentDefinitionResult<Self> {
        let loader = Self { agents_dir };
        loader.ensure_directory()?;
        Ok(loader)
    }

    /// Ensure default agents are installed to the agents directory.
    ///
    /// This copies bundled agent definitions to `~/.descartes/agents/` if they
    /// don't already exist. User-modified agents are not overwritten.
    fn ensure_default_agents(&self) -> AgentDefinitionResult<()> {
        for (filename, content) in DEFAULT_AGENTS {
            let file_path = self.agents_dir.join(filename);
            if !file_path.exists() {
                fs::write(&file_path, content)?;
                info!("Installed default agent: {}", filename);
            }
        }
        Ok(())
    }

    /// Get the default agents directory (~/.descartes/agents/).
    fn default_agents_dir() -> AgentDefinitionResult<PathBuf> {
        let home = dirs::home_dir().ok_or(AgentDefinitionError::NoHomeDirectory)?;
        Ok(home.join(".descartes").join("agents"))
    }

    /// Ensure the agents directory exists.
    fn ensure_directory(&self) -> AgentDefinitionResult<()> {
        if !self.agents_dir.exists() {
            fs::create_dir_all(&self.agents_dir)?;
            debug!("Created agents directory: {:?}", self.agents_dir);
        }
        Ok(())
    }

    /// Get the agents directory path.
    pub fn agents_dir(&self) -> &Path {
        &self.agents_dir
    }

    /// Load an agent definition by name.
    ///
    /// The name can be with or without the `.md` extension.
    pub fn load_agent(&self, name: &str) -> AgentDefinitionResult<AgentDefinition> {
        let filename = if name.ends_with(".md") {
            name.to_string()
        } else {
            format!("{}.md", name)
        };

        let file_path = self.agents_dir.join(&filename);

        if !file_path.exists() {
            return Err(AgentDefinitionError::AgentNotFound(name.to_string()));
        }

        let content = fs::read_to_string(&file_path)?;
        let default_name = name.trim_end_matches(".md");

        let definition = AgentDefinition::from_markdown(&content, default_name)?;
        info!("Loaded agent definition: {}", definition.name);
        Ok(definition)
    }

    /// List all available agent names.
    ///
    /// Returns the names without the `.md` extension.
    pub fn list_agents(&self) -> AgentDefinitionResult<Vec<String>> {
        let mut agents = Vec::new();

        if !self.agents_dir.exists() {
            return Ok(agents);
        }

        for entry in fs::read_dir(&self.agents_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        if let Some(stem) = path.file_stem() {
                            if let Some(name) = stem.to_str() {
                                agents.push(name.to_string());
                            }
                        }
                    }
                }
            }
        }

        agents.sort();
        debug!("Found {} agents in {:?}", agents.len(), self.agents_dir);
        Ok(agents)
    }

    /// Check if an agent exists.
    pub fn agent_exists(&self, name: &str) -> bool {
        let filename = if name.ends_with(".md") {
            name.to_string()
        } else {
            format!("{}.md", name)
        };
        self.agents_dir.join(filename).exists()
    }

    /// Save an agent definition to a file.
    pub fn save_agent(&self, definition: &AgentDefinition) -> AgentDefinitionResult<PathBuf> {
        self.ensure_directory()?;

        let filename = format!("{}.md", definition.name);
        let file_path = self.agents_dir.join(&filename);

        // Build the frontmatter
        let mut frontmatter = String::new();
        frontmatter.push_str("---\n");
        frontmatter.push_str(&format!("name: {}\n", definition.name));
        frontmatter.push_str(&format!("description: {}\n", definition.description));
        if let Some(model) = &definition.model {
            frontmatter.push_str(&format!("model: {}\n", model));
        }
        frontmatter.push_str(&format!(
            "tool_level: {}\n",
            tool_level_to_string(definition.tool_level)
        ));
        if !definition.tags.is_empty() {
            frontmatter.push_str(&format!("tags: [{}]\n", definition.tags.join(", ")));
        }
        frontmatter.push_str("---\n\n");

        let content = format!("{}{}", frontmatter, definition.system_prompt);
        fs::write(&file_path, content)?;

        info!("Saved agent definition: {} to {:?}", definition.name, file_path);
        Ok(file_path)
    }
}

impl Default for AgentDefinitionLoader {
    fn default() -> Self {
        Self::new().expect("Failed to create default AgentDefinitionLoader")
    }
}

/// Convert a ToolLevel to its string representation.
fn tool_level_to_string(level: ToolLevel) -> &'static str {
    match level {
        ToolLevel::Minimal => "minimal",
        ToolLevel::Orchestrator => "orchestrator",
        ToolLevel::ReadOnly => "readonly",
        ToolLevel::Researcher => "researcher",
        ToolLevel::Planner => "planner",
        ToolLevel::LispDeveloper => "lisp-developer",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_loader() -> (AgentDefinitionLoader, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let loader = AgentDefinitionLoader::with_dir(temp_dir.path().to_path_buf()).unwrap();
        (loader, temp_dir)
    }

    #[test]
    fn test_parse_agent_definition() {
        let content = r#"---
name: codebase-locator
description: Find files in the codebase
model: claude-3-sonnet
tool_level: readonly
tags: [research, codebase]
---

You are a codebase locator specialist.

Your job is to find files and understand the codebase structure.
"#;

        let definition = AgentDefinition::from_markdown(content, "fallback").unwrap();

        assert_eq!(definition.name, "codebase-locator");
        assert_eq!(definition.description, "Find files in the codebase");
        assert_eq!(definition.model, Some("claude-3-sonnet".to_string()));
        assert_eq!(definition.tool_level, ToolLevel::ReadOnly);
        assert_eq!(definition.tags, vec!["research", "codebase"]);
        assert!(definition.system_prompt.contains("codebase locator specialist"));
    }

    #[test]
    fn test_parse_agent_minimal() {
        let content = r#"---
name: simple-agent
---

Just a simple prompt.
"#;

        let definition = AgentDefinition::from_markdown(content, "fallback").unwrap();

        assert_eq!(definition.name, "simple-agent");
        assert_eq!(definition.description, "simple-agent agent");
        assert_eq!(definition.model, None);
        assert_eq!(definition.tool_level, ToolLevel::ReadOnly);
        assert!(definition.tags.is_empty());
    }

    #[test]
    fn test_parse_agent_uses_default_name() {
        let content = r#"---
description: No name provided
---

Prompt content.
"#;

        let definition = AgentDefinition::from_markdown(content, "my-default-name").unwrap();
        assert_eq!(definition.name, "my-default-name");
    }

    #[test]
    fn test_loader_save_and_load() {
        let (loader, _temp) = create_test_loader();

        let definition = AgentDefinition {
            name: "test-agent".to_string(),
            description: "A test agent".to_string(),
            model: Some("gpt-4".to_string()),
            tool_level: ToolLevel::Minimal,
            tags: vec!["test".to_string()],
            system_prompt: "You are a test agent.".to_string(),
        };

        // Save the agent
        loader.save_agent(&definition).unwrap();

        // Load it back
        let loaded = loader.load_agent("test-agent").unwrap();

        assert_eq!(loaded.name, definition.name);
        assert_eq!(loaded.description, definition.description);
        assert_eq!(loaded.model, definition.model);
        assert_eq!(loaded.tool_level, definition.tool_level);
        assert_eq!(loaded.tags, definition.tags);
        assert!(loaded.system_prompt.contains("test agent"));
    }

    #[test]
    fn test_loader_list_agents() {
        let (loader, _temp) = create_test_loader();

        // Save a few agents
        for name in ["alpha", "beta", "gamma"] {
            let definition = AgentDefinition {
                name: name.to_string(),
                description: format!("{} agent", name),
                model: None,
                tool_level: ToolLevel::ReadOnly,
                tags: vec![],
                system_prompt: format!("You are {}.", name),
            };
            loader.save_agent(&definition).unwrap();
        }

        let agents = loader.list_agents().unwrap();
        assert_eq!(agents.len(), 3);
        assert!(agents.contains(&"alpha".to_string()));
        assert!(agents.contains(&"beta".to_string()));
        assert!(agents.contains(&"gamma".to_string()));
    }

    #[test]
    fn test_loader_agent_not_found() {
        let (loader, _temp) = create_test_loader();

        let result = loader.load_agent("nonexistent");
        assert!(result.is_err());

        match result {
            Err(AgentDefinitionError::AgentNotFound(name)) => {
                assert_eq!(name, "nonexistent");
            }
            _ => panic!("Expected AgentNotFound error"),
        }
    }

    #[test]
    fn test_agent_exists() {
        let (loader, _temp) = create_test_loader();

        assert!(!loader.agent_exists("test-agent"));

        let definition = AgentDefinition {
            name: "test-agent".to_string(),
            description: "Test".to_string(),
            model: None,
            tool_level: ToolLevel::ReadOnly,
            tags: vec![],
            system_prompt: "Prompt".to_string(),
        };
        loader.save_agent(&definition).unwrap();

        assert!(loader.agent_exists("test-agent"));
        assert!(loader.agent_exists("test-agent.md"));
    }

    #[test]
    fn test_parse_tool_levels() {
        assert_eq!(parse_tool_level("minimal"), ToolLevel::Minimal);
        assert_eq!(parse_tool_level("Minimal"), ToolLevel::Minimal);
        assert_eq!(parse_tool_level("orchestrator"), ToolLevel::Orchestrator);
        assert_eq!(parse_tool_level("readonly"), ToolLevel::ReadOnly);
        assert_eq!(parse_tool_level("read_only"), ToolLevel::ReadOnly);
        assert_eq!(parse_tool_level("read-only"), ToolLevel::ReadOnly);
        assert_eq!(parse_tool_level("researcher"), ToolLevel::Researcher);
        assert_eq!(parse_tool_level("planner"), ToolLevel::Planner);
        assert_eq!(parse_tool_level("lisp-developer"), ToolLevel::LispDeveloper);
        assert_eq!(parse_tool_level("lisp_developer"), ToolLevel::LispDeveloper);
        assert_eq!(parse_tool_level("lispdeveloper"), ToolLevel::LispDeveloper);
        assert_eq!(parse_tool_level("unknown"), ToolLevel::ReadOnly); // default
    }
}
