//! Skills system for loadable prompts
//!
//! Skills are prompt templates that can be loaded and executed.
//! They can define variables, agent categories, and auto-context.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{Error, Result};

/// A skill definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Skill name (command name without /)
    pub name: String,
    /// Short description
    pub description: String,
    /// Path to the prompt file
    pub prompt_file: PathBuf,
    /// Agent category to use
    #[serde(default)]
    pub category: Option<String>,
    /// Auto-start agent after loading
    #[serde(default)]
    pub auto_start: bool,
    /// Variables that can be substituted
    #[serde(default)]
    pub variables: Vec<SkillVariable>,
    /// Auto-context to include
    #[serde(default)]
    pub auto_context: Vec<String>,
    /// Aliases for this skill
    #[serde(default)]
    pub aliases: Vec<String>,
}

/// A variable in a skill prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillVariable {
    /// Variable name
    pub name: String,
    /// Description
    #[serde(default)]
    pub description: Option<String>,
    /// Default value
    #[serde(default)]
    pub default: Option<String>,
    /// Required (no default)
    #[serde(default)]
    pub required: bool,
}

impl Skill {
    /// Load the prompt content with variable substitution
    pub fn load_prompt(&self, args: &str) -> Result<String> {
        let content = std::fs::read_to_string(&self.prompt_file).map_err(|e| {
            Error::Config(format!(
                "Failed to read skill prompt {}: {}",
                self.prompt_file.display(),
                e
            ))
        })?;

        // Parse args into variable values
        let values = self.parse_args(args);

        // Substitute variables
        let mut output = content;
        for var in &self.variables {
            let placeholder = format!("{{{{{}}}}}", var.name);
            let value = values
                .get(&var.name)
                .cloned()
                .or_else(|| var.default.clone())
                .unwrap_or_default();

            if var.required && value.is_empty() {
                return Err(Error::Config(format!(
                    "Required variable '{}' not provided",
                    var.name
                )));
            }

            output = output.replace(&placeholder, &value);
        }

        // Also substitute $1, $2, etc. style positional args
        let positional: Vec<&str> = args.split_whitespace().collect();
        for (i, arg) in positional.iter().enumerate() {
            output = output.replace(&format!("${}", i + 1), arg);
        }

        // Substitute $* for all args
        output = output.replace("$*", args);

        Ok(output)
    }

    /// Parse arguments into variable map
    fn parse_args(&self, args: &str) -> HashMap<String, String> {
        let mut values = HashMap::new();

        // Parse --key=value and --key value style args
        let parts: Vec<&str> = args.split_whitespace().collect();
        let mut i = 0;
        while i < parts.len() {
            let part = parts[i];
            if let Some(key) = part.strip_prefix("--") {
                if let Some((k, v)) = key.split_once('=') {
                    values.insert(k.to_string(), v.to_string());
                } else if i + 1 < parts.len() && !parts[i + 1].starts_with("--") {
                    values.insert(key.to_string(), parts[i + 1].to_string());
                    i += 1;
                }
            } else if let Some(key) = part.strip_prefix("-") {
                // Short flags: -f value
                if i + 1 < parts.len() && !parts[i + 1].starts_with("-") {
                    values.insert(key.to_string(), parts[i + 1].to_string());
                    i += 1;
                }
            }
            i += 1;
        }

        // First positional arg is often the main target
        if let Some(first) = parts.first() {
            if !first.starts_with('-') {
                values.insert("target".to_string(), first.to_string());
                values.insert("file".to_string(), first.to_string());
            }
        }

        values
    }
}

/// Skill registry for loading and managing skills
pub struct SkillRegistry {
    /// Loaded skills by name
    skills: HashMap<String, Skill>,
    /// Aliases mapping to skill names
    aliases: HashMap<String, String>,
    /// Search paths for skills
    search_paths: Vec<PathBuf>,
}

impl SkillRegistry {
    /// Create a new skill registry with default search paths
    pub fn new() -> Self {
        let mut registry = Self {
            skills: HashMap::new(),
            aliases: HashMap::new(),
            search_paths: vec![
                PathBuf::from(".descartes/skills"),
                PathBuf::from(".claude/skills"),
                dirs::config_dir()
                    .map(|p| p.join("descartes/skills"))
                    .unwrap_or_else(|| PathBuf::from("/etc/descartes/skills")),
            ],
        };

        // Register built-in skills
        registry.register_builtins();

        // Load skills from search paths
        registry.load_from_paths();

        registry
    }

    /// Register built-in skills
    fn register_builtins(&mut self) {
        // Create_plan skill
        self.register(Skill {
            name: "create_plan".to_string(),
            description: "Create an implementation plan from research".to_string(),
            prompt_file: PathBuf::from(".descartes/skills/create_plan.md"),
            category: Some("analyzer".to_string()),
            auto_start: true,
            variables: vec![
                SkillVariable {
                    name: "target".to_string(),
                    description: Some("Target file or feature to plan".to_string()),
                    default: None,
                    required: false,
                },
            ],
            auto_context: vec!["scud_tasks".to_string()],
            aliases: vec!["plan".to_string(), "cp".to_string()],
        });

        // Implement_plan skill
        self.register(Skill {
            name: "implement_plan".to_string(),
            description: "Implement tasks from a plan".to_string(),
            prompt_file: PathBuf::from(".descartes/skills/implement_plan.md"),
            category: Some("builder".to_string()),
            auto_start: true,
            variables: vec![
                SkillVariable {
                    name: "plan".to_string(),
                    description: Some("Path to the plan file".to_string()),
                    default: None,
                    required: false,
                },
            ],
            auto_context: vec!["scud_tasks".to_string(), "scud_waves".to_string()],
            aliases: vec!["implement".to_string(), "ip".to_string()],
        });

        // Research skill
        self.register(Skill {
            name: "research".to_string(),
            description: "Research a topic or codebase area".to_string(),
            prompt_file: PathBuf::from(".descartes/skills/research.md"),
            category: Some("searcher".to_string()),
            auto_start: true,
            variables: vec![
                SkillVariable {
                    name: "topic".to_string(),
                    description: Some("Topic or area to research".to_string()),
                    default: None,
                    required: true,
                },
            ],
            auto_context: vec![],
            aliases: vec!["r".to_string()],
        });

        // Commit skill
        self.register(Skill {
            name: "commit".to_string(),
            description: "Create a git commit with a good message".to_string(),
            prompt_file: PathBuf::from(".descartes/skills/commit.md"),
            category: Some("builder".to_string()),
            auto_start: true,
            variables: vec![],
            auto_context: vec!["git_diff".to_string(), "git_status".to_string()],
            aliases: vec!["c".to_string()],
        });

        // Review skill
        self.register(Skill {
            name: "review".to_string(),
            description: "Review code changes or PR".to_string(),
            prompt_file: PathBuf::from(".descartes/skills/review.md"),
            category: Some("validator".to_string()),
            auto_start: true,
            variables: vec![
                SkillVariable {
                    name: "target".to_string(),
                    description: Some("File, PR, or branch to review".to_string()),
                    default: None,
                    required: false,
                },
            ],
            auto_context: vec!["git_diff".to_string()],
            aliases: vec!["rv".to_string()],
        });

        // Fix skill
        self.register(Skill {
            name: "fix".to_string(),
            description: "Fix an issue or bug".to_string(),
            prompt_file: PathBuf::from(".descartes/skills/fix.md"),
            category: Some("builder".to_string()),
            auto_start: true,
            variables: vec![
                SkillVariable {
                    name: "issue".to_string(),
                    description: Some("Issue description or ID".to_string()),
                    default: None,
                    required: true,
                },
            ],
            auto_context: vec![],
            aliases: vec!["f".to_string()],
        });

        // Test skill
        self.register(Skill {
            name: "test".to_string(),
            description: "Run tests and fix failures".to_string(),
            prompt_file: PathBuf::from(".descartes/skills/test.md"),
            category: Some("validator".to_string()),
            auto_start: true,
            variables: vec![
                SkillVariable {
                    name: "target".to_string(),
                    description: Some("Test target or pattern".to_string()),
                    default: None,
                    required: false,
                },
            ],
            auto_context: vec![],
            aliases: vec!["t".to_string()],
        });
    }

    /// Load skills from search paths
    fn load_from_paths(&mut self) {
        for path in &self.search_paths.clone() {
            if path.exists() {
                self.load_from_directory(path);
            }
        }
    }

    /// Load skills from a directory
    fn load_from_directory(&mut self, dir: &Path) {
        // Load skill manifest if it exists
        let manifest_path = dir.join("skills.toml");
        if manifest_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&manifest_path) {
                if let Ok(skills) = toml::from_str::<SkillsManifest>(&content) {
                    for skill in skills.skills {
                        // Resolve relative paths
                        let mut skill = skill;
                        if skill.prompt_file.is_relative() {
                            skill.prompt_file = dir.join(&skill.prompt_file);
                        }
                        self.register(skill);
                    }
                }
            }
        }

        // Also scan for individual skill files
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "skill").unwrap_or(false) {
                    self.load_skill_file(&path);
                }
            }
        }
    }

    /// Load a single skill file (.skill.toml or .skill)
    fn load_skill_file(&mut self, path: &Path) {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(skill) = toml::from_str::<Skill>(&content) {
                let mut skill = skill;
                // If prompt_file is relative, resolve it relative to skill file
                if skill.prompt_file.is_relative() {
                    if let Some(parent) = path.parent() {
                        skill.prompt_file = parent.join(&skill.prompt_file);
                    }
                }
                self.register(skill);
            }
        }
    }

    /// Register a skill
    pub fn register(&mut self, skill: Skill) {
        // Register aliases
        for alias in &skill.aliases {
            self.aliases.insert(alias.clone(), skill.name.clone());
        }
        self.skills.insert(skill.name.clone(), skill);
    }

    /// Get a skill by name or alias
    pub fn get(&self, name: &str) -> Option<&Skill> {
        let name = name.to_lowercase();

        // Check direct match
        if let Some(skill) = self.skills.get(&name) {
            return Some(skill);
        }

        // Check aliases
        if let Some(real_name) = self.aliases.get(&name) {
            return self.skills.get(real_name);
        }

        None
    }

    /// List all skills
    pub fn list(&self) -> Vec<(&str, &Skill)> {
        let mut skills: Vec<_> = self.skills.iter().map(|(k, v)| (k.as_str(), v)).collect();
        skills.sort_by(|a, b| a.0.cmp(b.0));
        skills
    }

    /// Add a search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path.clone());
        if path.exists() {
            self.load_from_directory(&path);
        }
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Skill manifest file format
#[derive(Debug, Deserialize)]
struct SkillsManifest {
    #[serde(default)]
    skills: Vec<Skill>,
}

/// Create default skill prompt files
pub fn create_default_skills(dir: &Path) -> Result<()> {
    std::fs::create_dir_all(dir)?;

    // Create default skill prompts
    let skills = [
        (
            "create_plan.md",
            r#"# Create Implementation Plan

Based on the research and requirements, create a detailed implementation plan.

## Input
{{target}}

## Requirements
1. Break down the work into discrete, testable tasks
2. Identify dependencies between tasks
3. Estimate complexity (low/medium/high) for each task
4. Note any risks or unknowns

## Output Format
Create a plan in markdown with:
- Summary of approach
- Task breakdown with IDs (for SCUD)
- Dependencies
- Success criteria
"#,
        ),
        (
            "implement_plan.md",
            r#"# Implement Plan

Work through the implementation plan, completing tasks in dependency order.

## Plan
{{plan}}

## Instructions
1. Check SCUD for next available task
2. Implement the task
3. Run tests
4. Mark task complete in SCUD
5. Continue to next task

Focus on one task at a time. Commit after each completed task.
"#,
        ),
        (
            "research.md",
            r#"# Research

Conduct thorough research on the specified topic.

## Topic
{{topic}}

## Instructions
1. Search the codebase for relevant files
2. Understand existing patterns and conventions
3. Identify dependencies and constraints
4. Note any gaps or issues

## Output
Provide a summary with:
- Key findings
- Relevant files and code
- Recommendations
- Open questions
"#,
        ),
        (
            "commit.md",
            r#"# Create Commit

Create a well-formed git commit for the current changes.

## Current Changes
$*

## Instructions
1. Review the staged changes
2. Write a clear, conventional commit message
3. Follow the format: type(scope): description

Types: feat, fix, docs, style, refactor, test, chore
"#,
        ),
        (
            "review.md",
            r#"# Code Review

Review the specified code for quality and correctness.

## Target
{{target}}

## Review Checklist
- [ ] Code correctness
- [ ] Error handling
- [ ] Security considerations
- [ ] Performance implications
- [ ] Test coverage
- [ ] Documentation

## Output
Provide feedback organized by:
- Critical issues (must fix)
- Suggestions (should consider)
- Nitpicks (minor style issues)
"#,
        ),
        (
            "fix.md",
            r#"# Fix Issue

Diagnose and fix the specified issue.

## Issue
{{issue}}

## Instructions
1. Reproduce the issue if possible
2. Identify root cause
3. Implement fix
4. Add/update tests
5. Verify fix doesn't cause regressions
"#,
        ),
        (
            "test.md",
            r#"# Run Tests

Run tests and address any failures.

## Target
{{target}}

## Instructions
1. Run the test suite
2. Analyze any failures
3. Fix failing tests or code
4. Ensure all tests pass
5. Report coverage if available
"#,
        ),
    ];

    for (filename, content) in skills {
        let path = dir.join(filename);
        if !path.exists() {
            std::fs::write(&path, content)?;
            debug!("Created skill prompt: {}", path.display());
        }
    }

    // Create skills manifest
    let manifest_path = dir.join("skills.toml");
    if !manifest_path.exists() {
        let manifest = r#"# Descartes Skills Manifest
# Add custom skills here

# Example:
# [[skills]]
# name = "my_skill"
# description = "My custom skill"
# prompt_file = "my_skill.md"
# category = "builder"
# auto_start = true
# aliases = ["ms"]
"#;
        std::fs::write(&manifest_path, manifest)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skill_registry() {
        let registry = SkillRegistry::new();
        assert!(registry.get("create_plan").is_some());
        assert!(registry.get("cp").is_some()); // alias
        assert!(registry.get("plan").is_some()); // alias
    }

    #[test]
    fn test_skill_parse_args() {
        let skill = Skill {
            name: "test".to_string(),
            description: "Test".to_string(),
            prompt_file: PathBuf::from("test.md"),
            category: None,
            auto_start: false,
            variables: vec![],
            auto_context: vec![],
            aliases: vec![],
        };

        let args = skill.parse_args("file.rs --verbose --count=5");
        assert_eq!(args.get("target"), Some(&"file.rs".to_string()));
        assert_eq!(args.get("count"), Some(&"5".to_string()));
    }

    #[test]
    fn test_skill_list() {
        let registry = SkillRegistry::new();
        let skills = registry.list();
        assert!(!skills.is_empty());

        // Should be sorted
        let names: Vec<_> = skills.iter().map(|(name, _)| *name).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
    }
}
