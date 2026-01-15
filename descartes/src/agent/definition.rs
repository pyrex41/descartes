//! Agent definition loading from AGENT.md files
//!
//! Agents are defined using Anthropic's skill format:
//! - YAML frontmatter with name, description, category, model, tools, skills
//! - Markdown body with instructions
//! - {{skills_frontmatter}} placeholder for skill injection

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::{Error, Result};

/// Agent definition loaded from an AGENT.md file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDefinition {
    /// Agent name (from YAML frontmatter)
    pub name: String,
    /// Description for selection purposes (from YAML frontmatter)
    pub description: String,
    /// Base category for defaults (e.g., "analyzer", "builder")
    pub category: String,
    /// Optional model override
    #[serde(default)]
    pub model: Option<String>,
    /// Optional tools override
    #[serde(default)]
    pub tools: Option<Vec<String>>,
    /// Available skill names for this agent
    #[serde(default)]
    pub skills: Vec<String>,
    /// Instructions body (markdown content after frontmatter)
    #[serde(skip)]
    pub instructions: String,
    /// Directory path for resources
    #[serde(skip)]
    pub path: PathBuf,
}

/// YAML frontmatter structure for parsing
#[derive(Debug, Deserialize)]
struct AgentFrontmatter {
    name: String,
    description: String,
    #[serde(default = "default_category")]
    category: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    tools: Option<Vec<String>>,
    #[serde(default)]
    skills: Vec<String>,
}

fn default_category() -> String {
    "builder".to_string()
}

impl AgentDefinition {
    /// Load an agent definition from an AGENT.md file
    pub fn load(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            Error::Config(format!("Failed to read agent file {}: {}", path.display(), e))
        })?;

        Self::parse(&content, path)
    }

    /// Parse agent definition from content
    pub fn parse(content: &str, path: &Path) -> Result<Self> {
        // Extract YAML frontmatter between --- delimiters
        let (frontmatter_str, body) = Self::split_frontmatter(content)?;

        // Parse the YAML frontmatter
        let frontmatter: AgentFrontmatter = serde_yaml::from_str(&frontmatter_str).map_err(|e| {
            Error::Config(format!(
                "Failed to parse agent frontmatter in {}: {}",
                path.display(),
                e
            ))
        })?;

        // Get the directory containing the AGENT.md
        let dir = path.parent().unwrap_or(path).to_path_buf();

        Ok(Self {
            name: frontmatter.name,
            description: frontmatter.description,
            category: frontmatter.category,
            model: frontmatter.model,
            tools: frontmatter.tools,
            skills: frontmatter.skills,
            instructions: body.trim().to_string(),
            path: dir,
        })
    }

    /// Split content into frontmatter and body
    fn split_frontmatter(content: &str) -> Result<(String, String)> {
        let content = content.trim();

        // Must start with ---
        if !content.starts_with("---") {
            return Err(Error::Config(
                "Agent file must start with YAML frontmatter (---)".to_string(),
            ));
        }

        // Find the closing ---
        let rest = &content[3..];
        let end_pos = rest.find("\n---").ok_or_else(|| {
            Error::Config("Agent file missing closing frontmatter delimiter (---)".to_string())
        })?;

        let frontmatter = rest[..end_pos].trim().to_string();
        let body = rest[end_pos + 4..].to_string(); // Skip \n---

        Ok((frontmatter, body))
    }

    /// Build full context with skills resolved
    ///
    /// Replaces {{skills_frontmatter}} placeholder with actual skill metadata
    pub fn build_context(&self, skills_frontmatter: &str) -> String {
        self.instructions
            .replace("{{skills_frontmatter}}", skills_frontmatter)
    }

    /// Get a resource file path relative to this agent's directory
    pub fn resource_path(&self, filename: &str) -> PathBuf {
        self.path.join(filename)
    }
}

/// Registry for agent definitions
pub struct AgentDefinitionRegistry {
    /// Agents by name
    agents: HashMap<String, AgentDefinition>,
    /// Search paths for agents (in priority order)
    search_paths: Vec<PathBuf>,
    /// Enabled agent names (if set, only these are available)
    enabled: Option<Vec<String>>,
    /// Disabled agent names (if set, all except these are available)
    disabled: Option<Vec<String>>,
}

impl AgentDefinitionRegistry {
    /// Create a new empty registry with a primary directory
    pub fn new(directory: PathBuf) -> Self {
        // Build search paths with cross-tool compatibility
        let mut search_paths = vec![
            // Primary Descartes path
            directory,
            // OpenCode agent path (project-local)
            PathBuf::from(".opencode/agent"),
            // Claude Code compatibility path
            PathBuf::from(".claude/agents"),
        ];

        // Add global paths
        if let Some(config_dir) = dirs::config_dir() {
            // OpenCode global agents
            search_paths.push(config_dir.join("opencode/agent"));
            // Descartes global agents
            search_paths.push(config_dir.join("descartes/agents"));
        }

        // Home directory paths
        if let Some(home) = dirs::home_dir() {
            // Claude Code home path
            search_paths.push(home.join(".claude/agents"));
        }

        Self {
            agents: HashMap::new(),
            search_paths,
            enabled: None,
            disabled: None,
        }
    }

    /// Set enabled filter (use either enabled OR disabled, not both)
    pub fn with_enabled(mut self, enabled: Option<Vec<String>>) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set disabled filter
    pub fn with_disabled(mut self, disabled: Option<Vec<String>>) -> Self {
        self.disabled = disabled;
        self
    }

    /// Add a custom search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.insert(0, path); // Higher priority
    }

    /// Load all agents from all search paths
    pub fn load(&mut self) -> Result<()> {
        // Warn if both enabled and disabled are set
        if self.enabled.is_some() && self.disabled.is_some() {
            warn!("Both 'enabled' and 'disabled' are set in agent config. Using 'enabled' only.");
        }

        for search_path in &self.search_paths.clone() {
            if search_path.exists() {
                self.load_from_directory(search_path)?;
            }
        }

        debug!("Loaded {} agent definitions", self.agents.len());
        Ok(())
    }

    /// Load agents from a directory, supporting multiple formats
    fn load_from_directory(&mut self, dir: &Path) -> Result<()> {
        let entries = match std::fs::read_dir(dir) {
            Ok(e) => e,
            Err(e) => {
                debug!("Cannot read agent directory {}: {}", dir.display(), e);
                return Ok(());
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();

            // Format 1: Subdirectory with AGENT.md (Descartes/Claude style)
            // .descartes/agents/<name>/AGENT.md
            if path.is_dir() {
                let agent_file = path.join("AGENT.md");
                if agent_file.exists() {
                    self.try_load_agent(&agent_file);
                    continue;
                }
            }

            // Format 2: Direct .md file (OpenCode style)
            // .opencode/agent/<name>.md - filename becomes agent name
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "md" {
                        self.try_load_opencode_agent(&path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Try to load an agent from AGENT.md format
    fn try_load_agent(&mut self, path: &Path) {
        match AgentDefinition::load(path) {
            Ok(agent) => {
                if self.is_agent_allowed(&agent.name) && !self.agents.contains_key(&agent.name) {
                    debug!("Loaded agent definition: {} from {}", agent.name, path.display());
                    self.agents.insert(agent.name.clone(), agent);
                } else if self.agents.contains_key(&agent.name) {
                    debug!("Agent {} already loaded, skipping {}", agent.name, path.display());
                }
            }
            Err(e) => {
                warn!("Failed to load agent from {}: {}", path.display(), e);
            }
        }
    }

    /// Try to load an agent from OpenCode .md format
    fn try_load_opencode_agent(&mut self, path: &Path) {
        // OpenCode format: filename is the agent name
        let name = match path.file_stem().and_then(|s| s.to_str()) {
            Some(n) => n.to_string(),
            None => return,
        };

        // Skip if already loaded or not allowed
        if self.agents.contains_key(&name) || !self.is_agent_allowed(&name) {
            return;
        }

        let content = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                debug!("Cannot read {}: {}", path.display(), e);
                return;
            }
        };

        // Try to parse as our format first, then as OpenCode format
        if let Ok(agent) = AgentDefinition::parse(&content, path) {
            debug!("Loaded OpenCode agent: {} from {}", agent.name, path.display());
            self.agents.insert(agent.name.clone(), agent);
        } else if let Some(agent) = self.parse_opencode_format(&name, &content, path) {
            debug!("Loaded OpenCode agent (native format): {} from {}", agent.name, path.display());
            self.agents.insert(agent.name.clone(), agent);
        }
    }

    /// Parse OpenCode native agent format
    fn parse_opencode_format(&self, name: &str, content: &str, path: &Path) -> Option<AgentDefinition> {
        // OpenCode frontmatter fields: description, mode, model, prompt, tools, etc.
        let content = content.trim();
        if !content.starts_with("---") {
            return None;
        }

        let rest = &content[3..];
        let end_pos = rest.find("\n---")?;
        let frontmatter_str = rest[..end_pos].trim();
        let body = rest[end_pos + 4..].trim();

        // Parse YAML - OpenCode uses different field names
        let yaml: serde_yaml::Value = serde_yaml::from_str(frontmatter_str).ok()?;

        let description = yaml.get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("OpenCode agent")
            .to_string();

        let model = yaml.get("model")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let mode = yaml.get("mode")
            .and_then(|v| v.as_str())
            .unwrap_or("subagent");

        // Map OpenCode mode to our category
        let category = match mode {
            "primary" => "builder",
            "subagent" => "searcher",
            _ => "builder",
        }.to_string();

        Some(AgentDefinition {
            name: name.to_string(),
            description,
            category,
            model,
            tools: None,
            skills: vec![],
            instructions: body.to_string(),
            path: path.parent().unwrap_or(path).to_path_buf(),
        })
    }

    /// Check if an agent is allowed by the enabled/disabled filters
    fn is_agent_allowed(&self, name: &str) -> bool {
        // If enabled is set, only those agents are allowed
        if let Some(ref enabled) = self.enabled {
            return enabled.iter().any(|e| e == name);
        }

        // If disabled is set, all except those are allowed
        if let Some(ref disabled) = self.disabled {
            return !disabled.iter().any(|d| d == name);
        }

        // No filter, all allowed
        true
    }

    /// Get an agent by name
    pub fn get(&self, name: &str) -> Option<&AgentDefinition> {
        self.agents.get(name)
    }

    /// Get an agent by name (mutable)
    pub fn get_mut(&mut self, name: &str) -> Option<&mut AgentDefinition> {
        self.agents.get_mut(name)
    }

    /// List all enabled agents
    pub fn list(&self) -> Vec<&AgentDefinition> {
        self.agents.values().collect()
    }

    /// List agents by category
    pub fn list_by_category(&self, category: &str) -> Vec<&AgentDefinition> {
        self.agents
            .values()
            .filter(|a| a.category == category)
            .collect()
    }

    /// Get agent names
    pub fn names(&self) -> Vec<&str> {
        self.agents.keys().map(|s| s.as_str()).collect()
    }

    /// Check if an agent exists
    pub fn contains(&self, name: &str) -> bool {
        self.agents.contains_key(name)
    }

    /// Get count of loaded agents
    pub fn len(&self) -> usize {
        self.agents.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }
}

impl Default for AgentDefinitionRegistry {
    fn default() -> Self {
        Self::new(PathBuf::from(".descartes/agents"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE_AGENT: &str = r#"---
name: code-analyzer
description: Expert code analysis agent. Use for deep architectural review.
category: analyzer
model: opus
skills:
  - research
  - review
---

# Code Analyzer

You are an expert code analyst.

## Available Skills
{{skills_frontmatter}}

## Approach
1. Start by understanding the structure
2. Identify patterns
"#;

    #[test]
    fn test_parse_agent_definition() {
        let agent = AgentDefinition::parse(EXAMPLE_AGENT, Path::new("/test/AGENT.md")).unwrap();

        assert_eq!(agent.name, "code-analyzer");
        assert_eq!(
            agent.description,
            "Expert code analysis agent. Use for deep architectural review."
        );
        assert_eq!(agent.category, "analyzer");
        assert_eq!(agent.model, Some("opus".to_string()));
        assert_eq!(agent.skills, vec!["research", "review"]);
        assert!(agent.instructions.contains("# Code Analyzer"));
        assert!(agent.instructions.contains("{{skills_frontmatter}}"));
    }

    #[test]
    fn test_build_context() {
        let agent = AgentDefinition::parse(EXAMPLE_AGENT, Path::new("/test/AGENT.md")).unwrap();

        let skills_frontmatter = "## Available Skills\n- /research: Research a topic\n- /review: Review code";
        let context = agent.build_context(skills_frontmatter);

        assert!(context.contains("## Available Skills"));
        assert!(context.contains("- /research: Research a topic"));
        assert!(!context.contains("{{skills_frontmatter}}"));
    }

    #[test]
    fn test_split_frontmatter() {
        let (fm, body) = AgentDefinition::split_frontmatter(EXAMPLE_AGENT).unwrap();

        assert!(fm.contains("name: code-analyzer"));
        assert!(body.contains("# Code Analyzer"));
    }

    #[test]
    fn test_split_frontmatter_no_start() {
        let result = AgentDefinition::split_frontmatter("No frontmatter here");
        assert!(result.is_err());
    }

    #[test]
    fn test_split_frontmatter_no_end() {
        let result = AgentDefinition::split_frontmatter("---\nname: test\nNo closing");
        assert!(result.is_err());
    }

    #[test]
    fn test_minimal_agent() {
        let content = r#"---
name: minimal
description: Minimal agent
---

Just instructions.
"#;
        let agent = AgentDefinition::parse(content, Path::new("/test/AGENT.md")).unwrap();

        assert_eq!(agent.name, "minimal");
        assert_eq!(agent.category, "builder"); // default
        assert!(agent.model.is_none());
        assert!(agent.skills.is_empty());
    }

    #[test]
    fn test_registry_filter_enabled() {
        let registry = AgentDefinitionRegistry::new(PathBuf::from("/test"))
            .with_enabled(Some(vec!["allowed".to_string()]));

        assert!(registry.is_agent_allowed("allowed"));
        assert!(!registry.is_agent_allowed("not-allowed"));
    }

    #[test]
    fn test_registry_filter_disabled() {
        let registry = AgentDefinitionRegistry::new(PathBuf::from("/test"))
            .with_disabled(Some(vec!["blocked".to_string()]));

        assert!(!registry.is_agent_allowed("blocked"));
        assert!(registry.is_agent_allowed("allowed"));
    }

    #[test]
    fn test_registry_no_filter() {
        let registry = AgentDefinitionRegistry::new(PathBuf::from("/test"));

        assert!(registry.is_agent_allowed("anything"));
    }
}
