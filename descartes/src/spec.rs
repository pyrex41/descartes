//! Spec configuration for Swarm loop
//!
//! Implements Geoff's "fixed spec allocation" pattern:
//! ~5k tokens of persistent context at the start of each prompt.
//!
//! # Features
//!
//! - **Codebase Context Injection**: Auto-include relevant file snippets based on task description
//! - **Dependency Context**: Include outputs/summaries from completed dependency tasks
//! - **Verification Commands**: Configurable verification commands for each spec
//! - **Template System**: Per-project customizable prompt templates

use std::fs;
use std::path::{Path, PathBuf};

use regex::Regex;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::scud::Task;
use crate::Result;

// ============================================================================
// Codebase Context Injection
// ============================================================================

/// Configuration for codebase context injection
///
/// Allows automatic inclusion of relevant file snippets in task specs
/// based on file patterns, keywords, or explicit paths.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodebaseContext {
    /// Glob patterns for files to include (e.g., "src/**/*.rs")
    #[serde(default)]
    pub include_patterns: Vec<String>,

    /// Keywords to search for in the codebase
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Explicit file paths to include
    #[serde(default)]
    pub explicit_files: Vec<PathBuf>,

    /// Maximum number of files to include
    #[serde(default = "default_max_files")]
    pub max_files: usize,

    /// Maximum lines per file
    #[serde(default = "default_max_lines_per_file")]
    pub max_lines_per_file: usize,

    /// Working directory for resolving relative paths
    #[serde(skip)]
    pub working_dir: Option<PathBuf>,
}

fn default_max_files() -> usize {
    5
}

fn default_max_lines_per_file() -> usize {
    100
}

impl Default for CodebaseContext {
    fn default() -> Self {
        Self {
            include_patterns: Vec::new(),
            keywords: Vec::new(),
            explicit_files: Vec::new(),
            max_files: default_max_files(),
            max_lines_per_file: default_max_lines_per_file(),
            working_dir: None,
        }
    }
}

impl CodebaseContext {
    /// Create a new CodebaseContext with defaults
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a glob pattern for files to include
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.include_patterns.push(pattern.into());
        self
    }

    /// Add a keyword to search for
    pub fn with_keyword(mut self, keyword: impl Into<String>) -> Self {
        self.keywords.push(keyword.into());
        self
    }

    /// Add an explicit file path
    pub fn with_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.explicit_files.push(path.into());
        self
    }

    /// Set the working directory
    pub fn with_working_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.working_dir = Some(dir.into());
        self
    }

    /// Set maximum files to include
    pub fn with_max_files(mut self, max: usize) -> Self {
        self.max_files = max;
        self
    }

    /// Check if any context is configured
    pub fn is_empty(&self) -> bool {
        self.include_patterns.is_empty()
            && self.keywords.is_empty()
            && self.explicit_files.is_empty()
    }

    /// Load file contents based on configuration
    ///
    /// Returns a formatted string with file contents suitable for inclusion in a spec.
    pub fn load_context(&self) -> Result<Option<String>> {
        if self.is_empty() {
            return Ok(None);
        }

        let mut files_content = Vec::new();
        let working_dir = self
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Load explicit files first
        for file_path in &self.explicit_files {
            let full_path = if file_path.is_absolute() {
                file_path.clone()
            } else {
                working_dir.join(file_path)
            };

            if let Ok(content) = self.read_file_truncated(&full_path) {
                files_content.push(format!(
                    "### {}\n\n```\n{}\n```",
                    file_path.display(),
                    content
                ));
            }
        }

        // Process glob patterns
        for pattern in &self.include_patterns {
            let full_pattern = working_dir.join(pattern);
            if let Ok(entries) = glob::glob(full_pattern.to_string_lossy().as_ref()) {
                for entry in entries.flatten() {
                    if files_content.len() >= self.max_files {
                        break;
                    }
                    if let Ok(content) = self.read_file_truncated(&entry) {
                        let rel_path = entry
                            .strip_prefix(&working_dir)
                            .unwrap_or(&entry);
                        files_content.push(format!(
                            "### {}\n\n```\n{}\n```",
                            rel_path.display(),
                            content
                        ));
                    }
                }
            }
        }

        if files_content.is_empty() {
            return Ok(None);
        }

        Ok(Some(format!(
            "## Codebase Context\n\n{}",
            files_content.join("\n\n")
        )))
    }

    /// Read a file with truncation
    fn read_file_truncated(&self, path: &Path) -> Result<String> {
        let content = fs::read_to_string(path)?;
        let lines: Vec<&str> = content.lines().take(self.max_lines_per_file).collect();
        let truncated = lines.join("\n");

        if content.lines().count() > self.max_lines_per_file {
            Ok(format!("{}\n... (truncated)", truncated))
        } else {
            Ok(truncated)
        }
    }
}

// ============================================================================
// Dependency Context
// ============================================================================

/// Context from completed dependency tasks
///
/// Allows including outputs or summaries from tasks that this task depends on.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DependencyContext {
    /// Summaries from completed dependency tasks
    #[serde(default)]
    pub summaries: Vec<DependencySummary>,

    /// Whether to auto-load dependency transcripts
    #[serde(default)]
    pub auto_load_transcripts: bool,

    /// Path to transcripts directory
    #[serde(skip)]
    pub transcripts_dir: Option<PathBuf>,
}

/// Summary of a completed dependency task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencySummary {
    /// Task ID of the dependency
    pub task_id: String,

    /// Task title
    pub title: String,

    /// Summary of what was accomplished
    pub summary: String,

    /// Key outputs or artifacts produced
    #[serde(default)]
    pub outputs: Vec<String>,
}

impl DependencyContext {
    /// Create a new DependencyContext
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a dependency summary
    pub fn with_summary(mut self, summary: DependencySummary) -> Self {
        self.summaries.push(summary);
        self
    }

    /// Enable auto-loading of transcripts
    pub fn with_auto_load_transcripts(mut self, dir: PathBuf) -> Self {
        self.auto_load_transcripts = true;
        self.transcripts_dir = Some(dir);
        self
    }

    /// Check if any context is available
    pub fn is_empty(&self) -> bool {
        self.summaries.is_empty()
    }

    /// Format dependency context for inclusion in spec
    pub fn format(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let summaries: Vec<String> = self
            .summaries
            .iter()
            .map(|s| {
                let outputs_str = if s.outputs.is_empty() {
                    String::new()
                } else {
                    format!("\n  - Outputs: {}", s.outputs.join(", "))
                };
                format!(
                    "- **Task {}** ({}): {}{}",
                    s.task_id, s.title, s.summary, outputs_str
                )
            })
            .collect();

        Some(format!(
            "## Dependency Context\n\nCompleted dependencies:\n\n{}",
            summaries.join("\n")
        ))
    }
}

// ============================================================================
// Verification Commands
// ============================================================================

/// Configuration for verification commands
///
/// Specifies commands to run after implementation to verify correctness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationConfig {
    /// Primary verification command (e.g., "cargo test")
    #[serde(default)]
    pub primary: Option<String>,

    /// Additional validation commands
    #[serde(default)]
    pub additional: Vec<String>,

    /// Whether all commands must pass
    #[serde(default = "default_require_all")]
    pub require_all: bool,

    /// Custom success criteria description
    #[serde(default)]
    pub success_criteria: Option<String>,
}

fn default_require_all() -> bool {
    true
}

impl Default for VerificationConfig {
    fn default() -> Self {
        Self {
            primary: None,
            additional: Vec::new(),
            require_all: default_require_all(),
            success_criteria: None,
        }
    }
}

impl VerificationConfig {
    /// Create a new VerificationConfig
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the primary verification command
    pub fn with_primary(mut self, command: impl Into<String>) -> Self {
        self.primary = Some(command.into());
        self
    }

    /// Add an additional verification command
    pub fn with_additional(mut self, command: impl Into<String>) -> Self {
        self.additional.push(command.into());
        self
    }

    /// Set custom success criteria
    pub fn with_success_criteria(mut self, criteria: impl Into<String>) -> Self {
        self.success_criteria = Some(criteria.into());
        self
    }

    /// Get all verification commands as a slice
    pub fn all_commands(&self) -> Vec<&str> {
        let mut commands = Vec::new();
        if let Some(ref primary) = self.primary {
            commands.push(primary.as_str());
        }
        commands.extend(self.additional.iter().map(|s| s.as_str()));
        commands
    }

    /// Check if any verification is configured
    pub fn is_empty(&self) -> bool {
        self.primary.is_none() && self.additional.is_empty()
    }

    /// Format verification commands for inclusion in spec
    pub fn format(&self) -> Option<String> {
        if self.is_empty() {
            return None;
        }

        let commands = self.all_commands();
        let commands_formatted: Vec<String> = commands
            .iter()
            .map(|cmd| format!("{}  # required", cmd))
            .collect();

        let criteria_section = self
            .success_criteria
            .as_ref()
            .map(|c| format!("\n\n**Success Criteria**: {}", c))
            .unwrap_or_default();

        let require_note = if self.require_all && commands.len() > 1 {
            "\n\nAll commands must pass for the task to be complete."
        } else {
            ""
        };

        Some(format!(
            "## Verification Commands\n\n```bash\n{}\n```{}{}",
            commands_formatted.join("\n"),
            require_note,
            criteria_section
        ))
    }
}

// ============================================================================
// Template System
// ============================================================================

/// Per-project spec template configuration
///
/// Allows projects to customize the structure of generated specs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpecTemplate {
    /// Template name
    pub name: String,

    /// Template content with placeholders
    /// Available: {task}, {plan}, {codebase}, {dependencies}, {verification}, {custom}
    pub content: String,

    /// Description of when to use this template
    #[serde(default)]
    pub description: Option<String>,

    /// Task types this template applies to (e.g., ["builder", "planner"])
    #[serde(default)]
    pub applies_to: Vec<String>,
}

impl Default for SpecTemplate {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            content: DEFAULT_SPEC_TEMPLATE.to_string(),
            description: Some("Default spec template".to_string()),
            applies_to: Vec::new(),
        }
    }
}

impl SpecTemplate {
    /// Create a new template with the given name and content
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            description: None,
            applies_to: Vec::new(),
        }
    }

    /// Set description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Add an applicable task type
    pub fn applies_to(mut self, task_type: impl Into<String>) -> Self {
        self.applies_to.push(task_type.into());
        self
    }
}

/// Registry of project-specific templates
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TemplateRegistry {
    /// Available templates by name
    #[serde(default)]
    pub templates: std::collections::HashMap<String, SpecTemplate>,

    /// Default template name to use
    #[serde(default = "default_template_name")]
    pub default_template: String,
}

fn default_template_name() -> String {
    "default".to_string()
}

impl TemplateRegistry {
    /// Create a new registry with the built-in default template
    pub fn new() -> Self {
        let mut templates = std::collections::HashMap::new();
        templates.insert("default".to_string(), SpecTemplate::default());
        Self {
            templates,
            default_template: "default".to_string(),
        }
    }

    /// Add a template to the registry
    pub fn add_template(&mut self, template: SpecTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Get a template by name, falling back to default
    pub fn get(&self, name: &str) -> &SpecTemplate {
        self.templates
            .get(name)
            .or_else(|| self.templates.get(&self.default_template))
            .unwrap_or_else(|| {
                static DEFAULT: once_cell::sync::Lazy<SpecTemplate> =
                    once_cell::sync::Lazy::new(SpecTemplate::default);
                &DEFAULT
            })
    }

    /// Get template for a specific task type
    pub fn get_for_type(&self, task_type: &str) -> &SpecTemplate {
        // Find a template that applies to this task type
        for template in self.templates.values() {
            if template.applies_to.iter().any(|t| t == task_type) {
                return template;
            }
        }
        // Fall back to default
        self.get(&self.default_template)
    }

    /// Load templates from a directory
    ///
    /// Looks for `.toml` files in the specified directory and loads them as templates.
    pub fn load_from_directory(&mut self, dir: &Path) -> Result<usize> {
        let mut loaded = 0;

        if !dir.exists() {
            debug!("Templates directory does not exist: {:?}", dir);
            return Ok(0);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().map(|e| e == "toml").unwrap_or(false) {
                match fs::read_to_string(&path) {
                    Ok(content) => {
                        match toml::from_str::<SpecTemplate>(&content) {
                            Ok(template) => {
                                debug!("Loaded template: {} from {:?}", template.name, path);
                                self.add_template(template);
                                loaded += 1;
                            }
                            Err(e) => {
                                warn!("Failed to parse template {:?}: {}", path, e);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to read template file {:?}: {}", path, e);
                    }
                }
            }
        }

        Ok(loaded)
    }
}

// ============================================================================
// Enhanced SpecConfig
// ============================================================================

/// Configuration for spec/context loading
#[derive(Debug, Clone)]
pub struct SpecConfig {
    /// Include SCUD task details in spec (default: true)
    pub include_task: bool,

    /// Path to implementation plan document
    pub plan_path: Option<PathBuf>,

    /// Additional spec files to include
    pub additional_specs: Vec<PathBuf>,

    /// Max tokens for combined spec (warn if exceeded)
    pub max_spec_tokens: Option<usize>,

    /// Custom template for combining specs
    /// Placeholders: {task}, {plan}, {custom}, {verification}, {codebase}, {dependencies}
    pub spec_template: Option<String>,

    /// Codebase context injection configuration
    pub codebase_context: Option<CodebaseContext>,

    /// Dependency context configuration
    pub dependency_context: Option<DependencyContext>,

    /// Verification commands configuration
    pub verification: Option<VerificationConfig>,

    /// Path to templates directory
    pub templates_dir: Option<PathBuf>,

    /// Template registry (loaded from templates_dir)
    pub template_registry: Option<TemplateRegistry>,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            include_task: true,
            plan_path: None,
            additional_specs: Vec::new(),
            max_spec_tokens: Some(5000),
            spec_template: None,
            codebase_context: None,
            dependency_context: None,
            verification: None,
            templates_dir: None,
            template_registry: None,
        }
    }
}

impl SpecConfig {
    /// Create a new SpecConfig with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a plan document path
    pub fn with_plan(mut self, path: PathBuf) -> Self {
        self.plan_path = Some(path);
        self
    }

    /// Add an additional spec file
    pub fn with_spec_file(mut self, path: PathBuf) -> Self {
        self.additional_specs.push(path);
        self
    }

    /// Set codebase context configuration
    pub fn with_codebase_context(mut self, context: CodebaseContext) -> Self {
        self.codebase_context = Some(context);
        self
    }

    /// Set dependency context
    pub fn with_dependency_context(mut self, context: DependencyContext) -> Self {
        self.dependency_context = Some(context);
        self
    }

    /// Set verification configuration
    pub fn with_verification(mut self, verification: VerificationConfig) -> Self {
        self.verification = Some(verification);
        self
    }

    /// Set templates directory and load templates
    pub fn with_templates_dir(mut self, dir: PathBuf) -> Self {
        self.templates_dir = Some(dir.clone());
        let mut registry = TemplateRegistry::new();
        if let Err(e) = registry.load_from_directory(&dir) {
            warn!("Failed to load templates from {:?}: {}", dir, e);
        }
        self.template_registry = Some(registry);
        self
    }

    /// Set a custom spec template
    pub fn with_template(mut self, template: String) -> Self {
        self.spec_template = Some(template);
        self
    }

    /// Get the template to use, considering registry and custom template
    pub fn get_template(&self, task_type: Option<&str>) -> &str {
        // Custom template takes precedence
        if let Some(ref template) = self.spec_template {
            return template;
        }

        // Try registry
        if let Some(ref registry) = self.template_registry {
            if let Some(task_type) = task_type {
                return &registry.get_for_type(task_type).content;
            }
            return &registry.get(&registry.default_template).content;
        }

        DEFAULT_SPEC_TEMPLATE
    }
}

/// Default template for combining spec sections
///
/// Placeholders:
/// - `{task}`: Task details (ID, title, description, status, dependencies)
/// - `{plan}`: Plan section extracted from plan document
/// - `{codebase}`: Codebase context (relevant file snippets)
/// - `{dependencies}`: Dependency context (completed task summaries)
/// - `{verification}`: Verification commands
/// - `{custom}`: Additional spec files content
const DEFAULT_SPEC_TEMPLATE: &str = r#"# Task Specification

{task}

{plan}

{codebase}

{dependencies}

{custom}

{verification}
"#;

/// Estimate token count from text (~4 chars per token)
fn estimate_tokens(text: &str) -> usize {
    text.len() / 4
}

/// Format a SCUD task as a spec section
pub fn format_task_spec(task: &Task) -> String {
    let deps_str = if task.dependencies.is_empty() {
        String::new()
    } else {
        format!("\n**Dependencies**: {}", task.dependencies.join(", "))
    };

    format!(
        "## Task: {}\n\n**ID**: {}\n**Status**: {:?}\n**Priority**: {:?}{}\n\n### Description\n\n{}",
        task.title,
        task.id,
        task.status,
        task.priority,
        deps_str,
        task.description
    )
}

/// Extract the plan section for a specific task from a plan document
///
/// Supports multiple heading patterns:
/// - `## Task X:` (e.g., "## Task 1.2: Implement feature")
/// - `### X.` (e.g., "### 1.2. Feature name")
/// - `#### Task X` (e.g., "#### Task 1.2")
/// - `## X` (e.g., "## 1.2")
pub fn extract_plan_section(plan_content: &str, task_id: &str) -> Option<String> {
    // Escape task_id for regex (handle dots in task IDs like "1.2")
    let escaped_id = regex::escape(task_id);

    // Build pattern to match section headings
    // Patterns: ## Task X:, ### X., #### Task X, ## X
    let pattern = format!(
        r"(?m)^(##\s+Task\s+{id}[:\s]|###\s+{id}\.\s|####\s+Task\s+{id}[:\s\n]|##\s+{id}[:\s\n])",
        id = escaped_id
    );

    let section_re = match Regex::new(&pattern) {
        Ok(re) => re,
        Err(_) => return None,
    };

    // Find the start of the section
    let section_match = section_re.find(plan_content)?;
    let section_start = plan_content[..section_match.start()]
        .rfind('\n')
        .map(|i| i + 1)
        .unwrap_or(0);

    // Find the end of the section (next heading at same or higher level)
    // Look for ## or ### (not ####) as section terminators
    let next_section_re = Regex::new(r"(?m)^#{2,3}\s").ok()?;
    let section_end = next_section_re
        .find_at(plan_content, section_match.end())
        .map(|m| m.start())
        .unwrap_or(plan_content.len());

    let section = plan_content[section_start..section_end].trim();
    if section.is_empty() {
        None
    } else {
        Some(section.to_string())
    }
}

/// Template context for spec generation
#[derive(Debug, Default)]
pub struct TemplateContext<'a> {
    /// Task details section
    pub task: Option<&'a str>,
    /// Plan section
    pub plan: Option<&'a str>,
    /// Codebase context section
    pub codebase: Option<&'a str>,
    /// Dependency context section
    pub dependencies: Option<&'a str>,
    /// Verification commands section
    pub verification: Option<&'a str>,
    /// Custom/additional content section
    pub custom: Option<&'a str>,
}

impl<'a> TemplateContext<'a> {
    /// Create a new template context
    pub fn new() -> Self {
        Self::default()
    }

    /// Set task section
    pub fn with_task(mut self, task: &'a str) -> Self {
        self.task = Some(task);
        self
    }

    /// Set plan section
    pub fn with_plan(mut self, plan: &'a str) -> Self {
        self.plan = Some(plan);
        self
    }

    /// Set codebase context section
    pub fn with_codebase(mut self, codebase: &'a str) -> Self {
        self.codebase = Some(codebase);
        self
    }

    /// Set dependencies section
    pub fn with_dependencies(mut self, deps: &'a str) -> Self {
        self.dependencies = Some(deps);
        self
    }

    /// Set verification section
    pub fn with_verification(mut self, verification: &'a str) -> Self {
        self.verification = Some(verification);
        self
    }

    /// Set custom section
    pub fn with_custom(mut self, custom: &'a str) -> Self {
        self.custom = Some(custom);
        self
    }
}

/// Apply a template with placeholder substitution
///
/// Placeholders: {task}, {plan}, {codebase}, {dependencies}, {verification}, {custom}
pub fn apply_spec_template(
    template: &str,
    task: Option<&str>,
    plan: Option<&str>,
    custom: Option<&str>,
    verification: Option<&str>,
) -> String {
    apply_template_with_context(
        template,
        &TemplateContext {
            task,
            plan,
            custom,
            verification,
            codebase: None,
            dependencies: None,
        },
    )
}

/// Apply a template with full context
pub fn apply_template_with_context(template: &str, ctx: &TemplateContext) -> String {
    template
        .replace("{task}", ctx.task.unwrap_or(""))
        .replace("{plan}", ctx.plan.unwrap_or(""))
        .replace("{codebase}", ctx.codebase.unwrap_or(""))
        .replace("{dependencies}", ctx.dependencies.unwrap_or(""))
        .replace("{verification}", ctx.verification.unwrap_or(""))
        .replace("{custom}", ctx.custom.unwrap_or(""))
        // Clean up multiple blank lines
        .lines()
        .fold((String::new(), false), |(mut acc, was_blank), line| {
            let is_blank = line.trim().is_empty();
            if !(is_blank && was_blank) {
                if !acc.is_empty() {
                    acc.push('\n');
                }
                acc.push_str(line);
            }
            (acc, is_blank)
        })
        .0
}

/// Build a complete task spec from configuration
///
/// Combines:
/// - Task details (if `include_task` is true)
/// - Plan section for the task (if `plan_path` is set)
/// - Codebase context (if `codebase_context` is configured)
/// - Dependency context (if `dependency_context` has summaries)
/// - Verification commands (if `verification` is configured)
/// - Additional spec files
///
/// Logs a warning if the combined spec exceeds `max_spec_tokens`.
pub fn build_task_spec(config: &SpecConfig, task: &Task) -> Result<String> {
    // Format task section
    let task_section = if config.include_task {
        Some(format_task_spec(task))
    } else {
        None
    };

    // Extract plan section if configured
    let plan_section = if let Some(plan_path) = &config.plan_path {
        match fs::read_to_string(plan_path) {
            Ok(plan_content) => extract_plan_section(&plan_content, &task.id),
            Err(e) => {
                warn!("Failed to read plan file {:?}: {}", plan_path, e);
                None
            }
        }
    } else {
        None
    };

    // Load codebase context
    let codebase_section = if let Some(ref ctx) = config.codebase_context {
        match ctx.load_context() {
            Ok(content) => content,
            Err(e) => {
                warn!("Failed to load codebase context: {}", e);
                None
            }
        }
    } else {
        None
    };

    // Format dependency context
    let dependency_section = config
        .dependency_context
        .as_ref()
        .and_then(|ctx| ctx.format());

    // Format verification commands
    let verification_section = config.verification.as_ref().and_then(|v| v.format());

    // Load and combine additional spec files
    let custom_section = if config.additional_specs.is_empty() {
        None
    } else {
        let mut custom_parts = Vec::new();
        for spec_path in &config.additional_specs {
            match fs::read_to_string(spec_path) {
                Ok(content) => {
                    custom_parts.push(format!(
                        "### {}\n\n{}",
                        spec_path.file_name().unwrap_or_default().to_string_lossy(),
                        content.trim()
                    ));
                }
                Err(e) => {
                    warn!("Failed to read spec file {:?}: {}", spec_path, e);
                }
            }
        }
        if custom_parts.is_empty() {
            None
        } else {
            Some(custom_parts.join("\n\n"))
        }
    };

    // Get template (considering task type if using registry)
    let template = config.get_template(task.agent_type.as_deref());

    // Apply template with full context
    let ctx = TemplateContext {
        task: task_section.as_deref(),
        plan: plan_section.as_deref(),
        codebase: codebase_section.as_deref(),
        dependencies: dependency_section.as_deref(),
        verification: verification_section.as_deref(),
        custom: custom_section.as_deref(),
    };
    let spec = apply_template_with_context(template, &ctx);

    // Check token budget
    if let Some(max_tokens) = config.max_spec_tokens {
        let estimated_tokens = estimate_tokens(&spec);
        if estimated_tokens > max_tokens {
            warn!(
                "Spec exceeds token budget: ~{} tokens (max: {})",
                estimated_tokens, max_tokens
            );
        }
    }

    Ok(spec)
}

/// Build the full prompt for a Ralph task execution.
///
/// This combines:
/// - User guidance (global + context-specific)
/// - Task spec (from `build_task_spec()`)
/// - SCUD tag context
/// - Verification command (from backpressure config or explicit --verify)
/// - Instructions including TASK_BLOCKED output format for blocked scenarios
///
/// # Arguments
///
/// * `spec` - The task spec content (from `build_task_spec()`)
/// * `task` - The SCUD task being executed
/// * `scud_tag` - The SCUD tag this task belongs to
/// * `verify_command` - Optional explicit verification command (overrides backpressure)
/// * `backpressure_commands` - Commands from backpressure config (used if verify_command is None)
/// * `guidance` - Optional user guidance to prepend to the prompt
///
/// # Returns
///
/// A formatted prompt string ready to send to the agent.
pub fn build_prompt(
    spec: &str,
    task: &Task,
    scud_tag: &str,
    verify_command: Option<&str>,
    backpressure_commands: &[String],
    guidance: Option<&str>,
) -> String {
    // Determine verification command: explicit > backpressure > fallback
    let verification = verify_command
        .map(|s| s.to_string())
        .or_else(|| backpressure_commands.first().cloned())
        .unwrap_or_else(|| "echo 'No verification configured'".to_string());

    // Format verification section with all backpressure commands if available
    let verification_section = if verify_command.is_some() {
        // Explicit command takes precedence
        format!(
            r#"After implementation, run:
```bash
{}
```"#,
            verification
        )
    } else if backpressure_commands.len() > 1 {
        // Multiple backpressure commands - show all
        let commands_formatted = backpressure_commands
            .iter()
            .map(|cmd| format!("{}  # required", cmd))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"After implementation, run these validation commands:
```bash
{}
```

All commands must pass for the task to be complete."#,
            commands_formatted
        )
    } else {
        // Single command or fallback
        format!(
            r#"After implementation, run:
```bash
{}
```"#,
            verification
        )
    };

    // Build guidance section if provided
    let guidance_section = guidance
        .map(|g| format!("## User Guidance\n\n{}\n\n", g))
        .unwrap_or_default();

    format!(
        r#"{guidance}You are implementing SCUD task {task_id} for tag '{tag}' using the Swarm technique.

## Spec

{spec}

## Verification Command

{verification}

## Instructions

1. Implement the task described in the spec
2. Follow existing code patterns in the codebase
3. Run the verification command(s)
4. If verification passes, you're done
5. If blocked after 3 attempts, output exactly: TASK_BLOCKED: <reason>

### Blocked Task Protocol

If you cannot complete this task after making a genuine attempt, output a single line:

```
TASK_BLOCKED: <concise reason>
```

Valid reasons for blocking:
- Missing dependencies not available in the codebase
- Requires clarification on ambiguous requirements
- Blocked by another task that must complete first
- External service or API unavailable
- Verification command fails with unresolvable errors

Do NOT use TASK_BLOCKED for:
- Normal implementation challenges (keep trying)
- Test failures you can fix
- Compilation errors you can debug

Begin implementation."#,
        guidance = guidance_section,
        task_id = task.id,
        tag = scud_tag,
        spec = spec,
        verification = verification_section
    )
}

/// Build a general spec for guidance (without task-specific content)
///
/// This combines:
/// - Plan document content (if `plan_path` is set)
/// - Additional spec files
///
/// Used by SCUD delegation to write context to `.scud/guidance/descartes-spec.md`
pub fn build_general_spec(config: &SpecConfig) -> Result<String> {
    let mut sections = Vec::new();

    // Add plan content if configured
    if let Some(plan_path) = &config.plan_path {
        match fs::read_to_string(plan_path) {
            Ok(plan_content) => {
                let filename = plan_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Plan".to_string());
                sections.push(format!("## {}\n\n{}", filename, plan_content.trim()));
            }
            Err(e) => {
                warn!("Failed to read plan file {:?}: {}", plan_path, e);
            }
        }
    }

    // Add additional spec files
    for spec_path in &config.additional_specs {
        match fs::read_to_string(spec_path) {
            Ok(content) => {
                let filename = spec_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Spec".to_string());
                sections.push(format!("## {}\n\n{}", filename, content.trim()));
            }
            Err(e) => {
                warn!("Failed to read spec file {:?}: {}", spec_path, e);
            }
        }
    }

    if sections.is_empty() {
        return Ok(String::new());
    }

    let spec = format!(
        "# Descartes Task Context\n\n{}\n",
        sections.join("\n\n---\n\n")
    );

    // Check token budget
    if let Some(max_tokens) = config.max_spec_tokens {
        let estimated_tokens = estimate_tokens(&spec);
        if estimated_tokens > max_tokens {
            warn!(
                "General spec exceeds token budget: ~{} tokens (max: {})",
                estimated_tokens, max_tokens
            );
        }
    }

    Ok(spec)
}

// ============================================================================
// Enriched Spec Generation (Task 9.2)
// ============================================================================

/// Build an enriched SpecConfig by integrating with SCUD and backpressure
///
/// This function enhances a basic SpecConfig with:
/// - Verification commands from backpressure config
/// - Dependency summaries from completed SCUD tasks
/// - Templates from the templates directory
/// - Optional codebase context patterns
///
/// # Arguments
///
/// * `base_config` - The basic SpecConfig with plan_path and additional_specs
/// * `working_dir` - The working directory containing `.scud/` and `.descartes/`
/// * `scud_tag` - The SCUD tag being executed (for loading dependency context)
///
/// # Returns
///
/// An enriched SpecConfig with all integrated features
pub fn enrich_spec_config(
    mut base_config: SpecConfig,
    working_dir: &Path,
    scud_tag: Option<&str>,
) -> Result<SpecConfig> {
    // 1. Load verification commands from backpressure config
    base_config.verification = load_verification_from_backpressure(working_dir)?;

    // 2. Load dependency context from SCUD (if tag provided)
    if let Some(tag) = scud_tag {
        base_config.dependency_context = load_dependency_context(working_dir, tag)?;
    }

    // 3. Load templates from .descartes/templates/ if it exists
    let templates_dir = working_dir.join(".descartes/templates");
    if templates_dir.exists() {
        base_config = base_config.with_templates_dir(templates_dir);
    }

    // 4. Set working directory for codebase context (if configured)
    if let Some(ref mut ctx) = base_config.codebase_context {
        ctx.working_dir = Some(working_dir.to_path_buf());
    }

    debug!(
        "Enriched spec config: verification={}, deps={}, templates={}",
        base_config.verification.is_some(),
        base_config.dependency_context.as_ref().map(|d| !d.is_empty()).unwrap_or(false),
        base_config.template_registry.is_some()
    );

    Ok(base_config)
}

/// Load verification config from backpressure.toml
fn load_verification_from_backpressure(working_dir: &Path) -> Result<Option<VerificationConfig>> {
    use scud::backpressure::BackpressureConfig;

    match BackpressureConfig::load(Some(&working_dir.to_path_buf())) {
        Ok(bp_config) if !bp_config.commands.is_empty() => {
            let mut verification = VerificationConfig::new();

            // First command is primary, rest are additional
            let mut commands = bp_config.commands.into_iter();
            if let Some(primary) = commands.next() {
                verification.primary = Some(primary);
            }
            verification.additional = commands.collect();
            verification.require_all = bp_config.stop_on_failure;

            debug!(
                "Loaded {} verification commands from backpressure config",
                1 + verification.additional.len()
            );

            Ok(Some(verification))
        }
        Ok(_) => {
            debug!("No verification commands in backpressure config");
            Ok(None)
        }
        Err(e) => {
            warn!("Failed to load backpressure config: {}", e);
            Ok(None)
        }
    }
}

/// Load dependency context from completed SCUD tasks
fn load_dependency_context(working_dir: &Path, scud_tag: &str) -> Result<Option<DependencyContext>> {
    use scud::models::TaskStatus;
    use scud::storage::Storage;

    let storage = Storage::new(Some(working_dir.to_path_buf()));

    // Load tasks for the tag
    let phases = match storage.load_tasks() {
        Ok(p) => p,
        Err(e) => {
            debug!("Failed to load SCUD tasks: {}", e);
            return Ok(None);
        }
    };

    let phase = match phases.get(scud_tag) {
        Some(p) => p,
        None => {
            debug!("SCUD tag '{}' not found", scud_tag);
            return Ok(None);
        }
    };

    // Find completed tasks to use as dependency summaries
    let completed_tasks: Vec<&Task> = phase
        .tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .collect();

    if completed_tasks.is_empty() {
        return Ok(None);
    }

    let mut dep_context = DependencyContext::new();

    for task in completed_tasks {
        let summary = DependencySummary {
            task_id: task.id.clone(),
            title: task.title.clone(),
            summary: truncate_description(&task.description, 200),
            outputs: Vec::new(), // Could be enhanced to track file outputs
        };
        dep_context.summaries.push(summary);
    }

    debug!(
        "Loaded {} completed task summaries for dependency context",
        dep_context.summaries.len()
    );

    Ok(Some(dep_context))
}

/// Truncate a description to a maximum length, preserving word boundaries
fn truncate_description(desc: &str, max_len: usize) -> String {
    if desc.len() <= max_len {
        return desc.to_string();
    }

    // Find the last space before max_len
    let truncated = &desc[..max_len];
    match truncated.rfind(' ') {
        Some(pos) if pos > max_len / 2 => format!("{}...", &desc[..pos]),
        _ => format!("{}...", truncated),
    }
}

/// Create a SpecConfig with codebase context for a specific task
///
/// Analyzes the task description to extract keywords and configure
/// relevant file patterns for codebase context injection.
///
/// # Arguments
///
/// * `task` - The task to analyze for codebase context
/// * `working_dir` - The working directory
///
/// # Returns
///
/// A CodebaseContext configured based on task analysis
pub fn create_codebase_context_for_task(task: &Task, working_dir: &Path) -> CodebaseContext {
    let mut ctx = CodebaseContext::new()
        .with_working_dir(working_dir.to_path_buf());

    // Extract potential keywords from task title and description
    let text = format!("{} {}", task.title, task.description);

    // Look for common code-related terms to build patterns
    let keywords: Vec<&str> = vec![
        "test", "config", "spec", "module", "component", "service",
        "handler", "controller", "model", "view", "api", "endpoint",
    ];

    for keyword in keywords {
        if text.to_lowercase().contains(keyword) {
            ctx.keywords.push(keyword.to_string());
        }
    }

    // Add common patterns based on file extension mentions
    if text.contains(".rs") || text.contains("rust") || text.contains("Cargo") {
        ctx.include_patterns.push("src/**/*.rs".to_string());
    }
    if text.contains(".ts") || text.contains("typescript") {
        ctx.include_patterns.push("src/**/*.ts".to_string());
    }
    if text.contains(".py") || text.contains("python") {
        ctx.include_patterns.push("**/*.py".to_string());
    }

    // Limit to reasonable defaults
    ctx.max_files = 3;
    ctx.max_lines_per_file = 50;

    ctx
}

/// Write spec content to SCUD guidance directory
///
/// Creates `.scud/guidance/descartes-spec.md` with the combined spec content
/// from plan files and additional spec files.
///
/// # Arguments
///
/// * `config` - The spec configuration with plan_path and additional_specs
/// * `working_dir` - The working directory containing `.scud/`
///
/// # Returns
///
/// The path to the written guidance file
pub fn write_spec_to_guidance(config: &SpecConfig, working_dir: &std::path::Path) -> Result<PathBuf> {
    let spec_content = build_general_spec(config)?;

    // If no content, skip writing
    if spec_content.is_empty() {
        return Ok(working_dir.join(".scud/guidance/descartes-spec.md"));
    }

    // Ensure guidance directory exists
    let guidance_dir = working_dir.join(".scud/guidance");
    fs::create_dir_all(&guidance_dir).map_err(|e| {
        crate::Error::Config(format!(
            "Failed to create guidance directory {:?}: {}",
            guidance_dir, e
        ))
    })?;

    // Write spec to guidance file
    let guidance_path = guidance_dir.join("descartes-spec.md");
    fs::write(&guidance_path, &spec_content).map_err(|e| {
        crate::Error::Config(format!(
            "Failed to write guidance file {:?}: {}",
            guidance_path, e
        ))
    })?;

    tracing::info!("Wrote spec to {:?} ({} bytes)", guidance_path, spec_content.len());

    Ok(guidance_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_task() -> Task {
        let mut task = Task::new(
            "1.2".to_string(),
            "Implement feature X".to_string(),
            "This is the task description.\nIt has multiple lines.".to_string(),
        );
        task.dependencies = vec!["1.1".to_string()];
        task
    }

    #[test]
    fn test_format_task_spec() {
        let task = sample_task();
        let spec = format_task_spec(&task);

        assert!(spec.contains("## Task: Implement feature X"));
        assert!(spec.contains("**ID**: 1.2"));
        assert!(spec.contains("**Dependencies**: 1.1"));
        assert!(spec.contains("This is the task description."));
    }

    #[test]
    fn test_format_task_spec_no_deps() {
        let task = Task::new(
            "1".to_string(),
            "Root task".to_string(),
            "Description".to_string(),
        );
        let spec = format_task_spec(&task);

        assert!(!spec.contains("Dependencies"));
    }

    #[test]
    fn test_extract_plan_section_double_hash() {
        let plan = r#"# Implementation Plan

## 1.1

First task details.

## 1.2: Feature X

Implementation details for 1.2.
More content here.

## 1.3

Next task.
"#;

        let section = extract_plan_section(plan, "1.2").unwrap();
        assert!(section.contains("## 1.2: Feature X"));
        assert!(section.contains("Implementation details for 1.2"));
        assert!(!section.contains("## 1.3"));
    }

    #[test]
    fn test_extract_plan_section_triple_hash() {
        let plan = r#"## Features

### 2.1. First feature

Details for 2.1.

### 2.2. Second feature

Details for 2.2.
"#;

        let section = extract_plan_section(plan, "2.1").unwrap();
        assert!(section.contains("### 2.1. First feature"));
        assert!(section.contains("Details for 2.1"));
        assert!(!section.contains("2.2"));
    }

    #[test]
    fn test_extract_plan_section_task_prefix() {
        let plan = r#"# Plan

## Task 3.1: Setup

Setup instructions.

## Task 3.2: Build

Build instructions.
"#;

        let section = extract_plan_section(plan, "3.1").unwrap();
        assert!(section.contains("## Task 3.1: Setup"));
        assert!(section.contains("Setup instructions"));
        assert!(!section.contains("3.2"));
    }

    #[test]
    fn test_extract_plan_section_not_found() {
        let plan = "## 1.1\n\nSome content.";
        let section = extract_plan_section(plan, "9.9");
        assert!(section.is_none());
    }

    #[test]
    fn test_apply_spec_template() {
        let template = "{task}\n\n{plan}\n\n{custom}";
        let result = apply_spec_template(
            template,
            Some("Task content"),
            Some("Plan content"),
            Some("Custom content"),
            None,
        );

        assert!(result.contains("Task content"));
        assert!(result.contains("Plan content"));
        assert!(result.contains("Custom content"));
    }

    #[test]
    fn test_apply_spec_template_missing_sections() {
        let template = "{task}\n\n{plan}\n\n{custom}";
        let result = apply_spec_template(template, Some("Task only"), None, None, None);

        assert!(result.contains("Task only"));
        // Should not have excessive blank lines
        assert!(!result.contains("\n\n\n"));
    }

    #[test]
    fn test_estimate_tokens() {
        // 20 chars should be ~5 tokens
        assert_eq!(estimate_tokens("12345678901234567890"), 5);
        // Empty string
        assert_eq!(estimate_tokens(""), 0);
    }

    #[test]
    fn test_build_task_spec_basic() {
        let task = sample_task();
        let config = SpecConfig::default();

        let spec = build_task_spec(&config, &task).unwrap();

        assert!(spec.contains("## Task: Implement feature X"));
        assert!(spec.contains("**ID**: 1.2"));
    }

    #[test]
    fn test_build_task_spec_no_task() {
        let task = sample_task();
        let mut config = SpecConfig::default();
        config.include_task = false;

        let spec = build_task_spec(&config, &task).unwrap();

        assert!(!spec.contains("## Task:"));
    }

    // ============ build_prompt tests ============

    #[test]
    fn test_build_prompt_with_explicit_verify() {
        let task = sample_task();
        let spec = "# Task Spec\n\nThis is the spec content.";

        let prompt = build_prompt(spec, &task, "test-tag", Some("cargo test"), &[], None);

        assert!(prompt.contains("task 1.2"));
        assert!(prompt.contains("tag 'test-tag'"));
        assert!(prompt.contains("cargo test"));
        assert!(prompt.contains("TASK_BLOCKED"));
        assert!(prompt.contains("Swarm technique"));
    }

    #[test]
    fn test_build_prompt_with_backpressure_commands() {
        let task = sample_task();
        let spec = "# Subtask Spec";

        let prompt = build_prompt(
            spec,
            &task,
            "feature",
            None,
            &["cargo build".to_string(), "cargo test".to_string()],
            None,
        );

        assert!(prompt.contains("task 1.2"));
        assert!(prompt.contains("cargo build"));
        assert!(prompt.contains("cargo test"));
        assert!(prompt.contains("All commands must pass"));
    }

    #[test]
    fn test_build_prompt_explicit_overrides_backpressure() {
        let task = sample_task();
        let spec = "# Spec";

        let prompt = build_prompt(
            spec,
            &task,
            "tag",
            Some("npm test"),
            &["cargo build".to_string()],
            None,
        );

        assert!(prompt.contains("npm test"));
        assert!(!prompt.contains("cargo build")); // Should not appear
    }

    #[test]
    fn test_build_prompt_fallback_when_no_commands() {
        let task = sample_task();
        let spec = "# Spec";

        let prompt = build_prompt(spec, &task, "empty", None, &[], None);

        assert!(prompt.contains("No verification configured"));
    }

    #[test]
    fn test_build_prompt_contains_blocked_protocol() {
        let task = sample_task();
        let spec = "# Spec";

        let prompt = build_prompt(spec, &task, "tag", Some("verify"), &[], None);

        // Check for blocked protocol documentation
        assert!(prompt.contains("Blocked Task Protocol"));
        assert!(prompt.contains("Missing dependencies"));
        assert!(prompt.contains("Requires clarification"));
        assert!(prompt.contains("Do NOT use TASK_BLOCKED"));
    }

    #[test]
    fn test_build_prompt_single_backpressure_command() {
        let task = sample_task();
        let spec = "# Spec";

        let prompt = build_prompt(spec, &task, "single", None, &["cargo test".to_string()], None);

        assert!(prompt.contains("cargo test"));
        // Should NOT show "All commands must pass" for single command
        assert!(!prompt.contains("All commands must pass"));
    }

    #[test]
    fn test_build_prompt_includes_spec_content() {
        let task = sample_task();
        let spec = "## Custom Task Details\n\nThis is custom spec content with **markdown**.";

        let prompt = build_prompt(spec, &task, "test", Some("make test"), &[], None);

        // Spec content should be included verbatim
        assert!(prompt.contains("## Custom Task Details"));
        assert!(prompt.contains("This is custom spec content with **markdown**."));
    }

    #[test]
    fn test_build_prompt_with_guidance() {
        let task = sample_task();
        let spec = "# Spec";
        let guidance = "Always use Rust best practices.\nPrefer composition over inheritance.";

        let prompt = build_prompt(spec, &task, "tag", Some("cargo test"), &[], Some(guidance));

        // Guidance should appear at the start
        assert!(prompt.contains("## User Guidance"));
        assert!(prompt.contains("Always use Rust best practices"));
        assert!(prompt.contains("Prefer composition over inheritance"));
        // Should still contain normal prompt elements
        assert!(prompt.contains("Swarm technique"));
    }

    // ============ build_general_spec tests ============

    #[test]
    fn test_build_general_spec_empty() {
        let config = SpecConfig::default();
        let spec = build_general_spec(&config).unwrap();
        assert!(spec.is_empty(), "Empty config should produce empty spec");
    }

    #[test]
    fn test_build_general_spec_with_plan() {
        // Create a temp file for the plan
        let temp_dir = std::env::temp_dir();
        let plan_path = temp_dir.join("test-plan.md");
        std::fs::write(&plan_path, "# Test Plan\n\nThis is the plan content.").unwrap();

        let config = SpecConfig::new().with_plan(plan_path.clone());
        let spec = build_general_spec(&config).unwrap();

        assert!(spec.contains("# Descartes Task Context"));
        assert!(spec.contains("## test-plan.md"));
        assert!(spec.contains("This is the plan content"));

        // Cleanup
        let _ = std::fs::remove_file(&plan_path);
    }

    #[test]
    fn test_build_general_spec_with_multiple_files() {
        let temp_dir = std::env::temp_dir();
        let plan_path = temp_dir.join("test-plan2.md");
        let spec_path = temp_dir.join("test-spec.md");

        std::fs::write(&plan_path, "# Plan\n\nPlan content.").unwrap();
        std::fs::write(&spec_path, "# Spec\n\nSpec content.").unwrap();

        let config = SpecConfig::new()
            .with_plan(plan_path.clone())
            .with_spec_file(spec_path.clone());
        let spec = build_general_spec(&config).unwrap();

        assert!(spec.contains("Plan content"));
        assert!(spec.contains("Spec content"));
        assert!(spec.contains("---"), "Should have separator between sections");

        // Cleanup
        let _ = std::fs::remove_file(&plan_path);
        let _ = std::fs::remove_file(&spec_path);
    }

    // ============ write_spec_to_guidance tests ============

    #[test]
    fn test_write_spec_to_guidance_creates_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let working_dir = temp_dir.path();

        // Create a plan file
        let plan_path = temp_dir.path().join("plan.md");
        std::fs::write(&plan_path, "# Test Plan\n\nContent here.").unwrap();

        let config = SpecConfig::new().with_plan(plan_path);
        let result = write_spec_to_guidance(&config, working_dir).unwrap();

        assert!(result.exists(), "Guidance file should exist");
        assert!(result.ends_with("descartes-spec.md"));

        let content = std::fs::read_to_string(&result).unwrap();
        assert!(content.contains("# Descartes Task Context"));
        assert!(content.contains("Content here"));
    }

    #[test]
    fn test_write_spec_to_guidance_creates_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let working_dir = temp_dir.path();

        // Create a plan file
        let plan_path = temp_dir.path().join("plan.md");
        std::fs::write(&plan_path, "# Plan").unwrap();

        // .scud/guidance/ doesn't exist yet
        let guidance_dir = working_dir.join(".scud/guidance");
        assert!(!guidance_dir.exists());

        let config = SpecConfig::new().with_plan(plan_path);
        write_spec_to_guidance(&config, working_dir).unwrap();

        assert!(guidance_dir.exists(), "Should create guidance directory");
    }

    #[test]
    fn test_write_spec_to_guidance_empty_config() {
        let temp_dir = tempfile::tempdir().unwrap();
        let config = SpecConfig::default();

        let result = write_spec_to_guidance(&config, temp_dir.path()).unwrap();

        // Should return path but not create file when empty
        assert!(result.ends_with("descartes-spec.md"));
        // File might or might not exist, but if it doesn't, that's fine for empty config
    }

    // ============ CodebaseContext tests ============

    #[test]
    fn test_codebase_context_empty() {
        let ctx = CodebaseContext::new();
        assert!(ctx.is_empty());

        let content = ctx.load_context().unwrap();
        assert!(content.is_none());
    }

    #[test]
    fn test_codebase_context_with_explicit_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.rs");
        std::fs::write(&file_path, "fn main() {\n    println!(\"Hello\");\n}").unwrap();

        let ctx = CodebaseContext::new()
            .with_file(file_path.clone())
            .with_working_dir(temp_dir.path().to_path_buf());

        assert!(!ctx.is_empty());

        let content = ctx.load_context().unwrap();
        assert!(content.is_some());
        let content = content.unwrap();
        assert!(content.contains("## Codebase Context"));
        assert!(content.contains("fn main()"));
    }

    #[test]
    fn test_codebase_context_truncation() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("long.txt");

        // Create a file with many lines
        let lines: Vec<String> = (0..200).map(|i| format!("Line {}", i)).collect();
        std::fs::write(&file_path, lines.join("\n")).unwrap();

        let ctx = CodebaseContext::new()
            .with_file(file_path)
            .with_working_dir(temp_dir.path().to_path_buf())
            .with_max_files(5); // default max_lines_per_file is 100

        let content = ctx.load_context().unwrap().unwrap();
        assert!(content.contains("... (truncated)"));
        assert!(content.contains("Line 99")); // Should have line 99
        assert!(!content.contains("Line 150")); // Should not have line 150
    }

    // ============ DependencyContext tests ============

    #[test]
    fn test_dependency_context_empty() {
        let ctx = DependencyContext::new();
        assert!(ctx.is_empty());
        assert!(ctx.format().is_none());
    }

    #[test]
    fn test_dependency_context_with_summary() {
        let summary = DependencySummary {
            task_id: "1.1".to_string(),
            title: "Setup project".to_string(),
            summary: "Created initial project structure".to_string(),
            outputs: vec!["Cargo.toml".to_string(), "src/main.rs".to_string()],
        };

        let ctx = DependencyContext::new().with_summary(summary);

        assert!(!ctx.is_empty());
        let formatted = ctx.format().unwrap();
        assert!(formatted.contains("## Dependency Context"));
        assert!(formatted.contains("Task 1.1"));
        assert!(formatted.contains("Setup project"));
        assert!(formatted.contains("Created initial project structure"));
        assert!(formatted.contains("Cargo.toml"));
    }

    #[test]
    fn test_dependency_context_multiple_summaries() {
        let ctx = DependencyContext::new()
            .with_summary(DependencySummary {
                task_id: "1.1".to_string(),
                title: "First task".to_string(),
                summary: "Done first".to_string(),
                outputs: vec![],
            })
            .with_summary(DependencySummary {
                task_id: "1.2".to_string(),
                title: "Second task".to_string(),
                summary: "Done second".to_string(),
                outputs: vec!["output.txt".to_string()],
            });

        let formatted = ctx.format().unwrap();
        assert!(formatted.contains("Task 1.1"));
        assert!(formatted.contains("Task 1.2"));
        assert!(formatted.contains("Done first"));
        assert!(formatted.contains("Done second"));
    }

    // ============ VerificationConfig tests ============

    #[test]
    fn test_verification_config_empty() {
        let config = VerificationConfig::new();
        assert!(config.is_empty());
        assert!(config.format().is_none());
    }

    #[test]
    fn test_verification_config_primary_only() {
        let config = VerificationConfig::new().with_primary("cargo test");

        assert!(!config.is_empty());
        assert_eq!(config.all_commands(), vec!["cargo test"]);

        let formatted = config.format().unwrap();
        assert!(formatted.contains("## Verification Commands"));
        assert!(formatted.contains("cargo test"));
    }

    #[test]
    fn test_verification_config_multiple_commands() {
        let config = VerificationConfig::new()
            .with_primary("cargo build")
            .with_additional("cargo test")
            .with_additional("cargo clippy");

        assert_eq!(config.all_commands().len(), 3);

        let formatted = config.format().unwrap();
        assert!(formatted.contains("cargo build"));
        assert!(formatted.contains("cargo test"));
        assert!(formatted.contains("cargo clippy"));
        assert!(formatted.contains("All commands must pass"));
    }

    #[test]
    fn test_verification_config_with_success_criteria() {
        let config = VerificationConfig::new()
            .with_primary("cargo test")
            .with_success_criteria("All tests pass with no warnings");

        let formatted = config.format().unwrap();
        assert!(formatted.contains("**Success Criteria**: All tests pass with no warnings"));
    }

    // ============ TemplateContext tests ============

    #[test]
    fn test_template_context_all_fields() {
        let ctx = TemplateContext::new()
            .with_task("Task content")
            .with_plan("Plan content")
            .with_codebase("Codebase content")
            .with_dependencies("Dependencies content")
            .with_verification("Verification content")
            .with_custom("Custom content");

        let template = "{task}\n{plan}\n{codebase}\n{dependencies}\n{verification}\n{custom}";
        let result = apply_template_with_context(template, &ctx);

        assert!(result.contains("Task content"));
        assert!(result.contains("Plan content"));
        assert!(result.contains("Codebase content"));
        assert!(result.contains("Dependencies content"));
        assert!(result.contains("Verification content"));
        assert!(result.contains("Custom content"));
    }

    // ============ SpecTemplate tests ============

    #[test]
    fn test_spec_template_default() {
        let template = SpecTemplate::default();
        assert_eq!(template.name, "default");
        assert!(template.content.contains("{task}"));
        assert!(template.content.contains("{codebase}"));
        assert!(template.content.contains("{dependencies}"));
    }

    #[test]
    fn test_spec_template_custom() {
        let template = SpecTemplate::new("builder", "# Builder Task\n\n{task}\n\n{verification}")
            .with_description("Template for builder tasks")
            .applies_to("builder")
            .applies_to("fast-builder");

        assert_eq!(template.name, "builder");
        assert!(template.applies_to.contains(&"builder".to_string()));
        assert!(template.applies_to.contains(&"fast-builder".to_string()));
    }

    // ============ TemplateRegistry tests ============

    #[test]
    fn test_template_registry_default() {
        let registry = TemplateRegistry::new();
        assert_eq!(registry.default_template, "default");

        let template = registry.get("default");
        assert_eq!(template.name, "default");
    }

    #[test]
    fn test_template_registry_add_and_get() {
        let mut registry = TemplateRegistry::new();
        registry.add_template(
            SpecTemplate::new("builder", "Builder template")
                .applies_to("builder"),
        );

        let template = registry.get("builder");
        assert_eq!(template.name, "builder");

        // Should return default for unknown template
        let unknown = registry.get("unknown");
        assert_eq!(unknown.name, "default");
    }

    #[test]
    fn test_template_registry_get_for_type() {
        let mut registry = TemplateRegistry::new();
        registry.add_template(
            SpecTemplate::new("builder-template", "Builder content")
                .applies_to("builder")
                .applies_to("fast-builder"),
        );

        let template = registry.get_for_type("builder");
        assert_eq!(template.name, "builder-template");

        let template = registry.get_for_type("fast-builder");
        assert_eq!(template.name, "builder-template");

        // Unknown type should return default
        let template = registry.get_for_type("unknown");
        assert_eq!(template.name, "default");
    }

    // ============ Integration tests for build_task_spec with new features ============

    #[test]
    fn test_build_task_spec_with_verification() {
        let task = sample_task();
        let verification = VerificationConfig::new()
            .with_primary("cargo test")
            .with_additional("cargo clippy");

        let config = SpecConfig::default().with_verification(verification);
        let spec = build_task_spec(&config, &task).unwrap();

        assert!(spec.contains("## Verification Commands"));
        assert!(spec.contains("cargo test"));
        assert!(spec.contains("cargo clippy"));
    }

    #[test]
    fn test_build_task_spec_with_dependency_context() {
        let task = sample_task();
        let dep_ctx = DependencyContext::new().with_summary(DependencySummary {
            task_id: "1.1".to_string(),
            title: "Prerequisites".to_string(),
            summary: "Set up the project foundation".to_string(),
            outputs: vec![],
        });

        let config = SpecConfig::default().with_dependency_context(dep_ctx);
        let spec = build_task_spec(&config, &task).unwrap();

        assert!(spec.contains("## Dependency Context"));
        assert!(spec.contains("Task 1.1"));
        assert!(spec.contains("Prerequisites"));
    }

    #[test]
    fn test_build_task_spec_with_all_features() {
        let temp_dir = tempfile::tempdir().unwrap();
        let plan_path = temp_dir.path().join("plan.md");
        std::fs::write(
            &plan_path,
            "# Plan\n\n## 1.2: Feature X\n\nImplement the feature.\n\n## 1.3\n\nNext task.",
        )
        .unwrap();

        let task = sample_task();
        let config = SpecConfig::default()
            .with_plan(plan_path)
            .with_verification(
                VerificationConfig::new()
                    .with_primary("cargo test"),
            )
            .with_dependency_context(
                DependencyContext::new().with_summary(DependencySummary {
                    task_id: "1.1".to_string(),
                    title: "Setup".to_string(),
                    summary: "Initial setup done".to_string(),
                    outputs: vec![],
                }),
            );

        let spec = build_task_spec(&config, &task).unwrap();

        // Check all sections are present
        assert!(spec.contains("## Task: Implement feature X")); // Task section
        assert!(spec.contains("## 1.2: Feature X")); // Plan section
        assert!(spec.contains("## Dependency Context")); // Dependency section
        assert!(spec.contains("## Verification Commands")); // Verification section
    }

    // ============ Task 9.2: Enrichment function tests ============

    #[test]
    fn test_truncate_description_short() {
        let desc = "Short description";
        assert_eq!(truncate_description(desc, 100), desc);
    }

    #[test]
    fn test_truncate_description_long() {
        let desc = "This is a very long description that needs to be truncated at word boundaries";
        let truncated = truncate_description(desc, 30);
        assert!(truncated.ends_with("..."));
        assert!(truncated.len() <= 35); // 30 + "..."
    }

    #[test]
    fn test_truncate_description_no_space() {
        let desc = "NoSpacesInThisVeryLongString";
        let truncated = truncate_description(desc, 10);
        assert_eq!(truncated, "NoSpacesIn...");
    }

    #[test]
    fn test_create_codebase_context_for_task_rust() {
        let task = Task::new(
            "1".to_string(),
            "Implement Rust module".to_string(),
            "Add new .rs file for the feature".to_string(),
        );
        let ctx = create_codebase_context_for_task(&task, std::path::Path::new("."));

        assert!(ctx.include_patterns.iter().any(|p| p.contains(".rs")));
        assert_eq!(ctx.max_files, 3);
        assert_eq!(ctx.max_lines_per_file, 50);
    }

    #[test]
    fn test_create_codebase_context_for_task_typescript() {
        let task = Task::new(
            "1".to_string(),
            "Add TypeScript component".to_string(),
            "Create new .ts component".to_string(),
        );
        let ctx = create_codebase_context_for_task(&task, std::path::Path::new("."));

        assert!(ctx.include_patterns.iter().any(|p| p.contains(".ts")));
    }

    #[test]
    fn test_create_codebase_context_for_task_keywords() {
        let task = Task::new(
            "1".to_string(),
            "Add API endpoint".to_string(),
            "Create new endpoint handler for the service".to_string(),
        );
        let ctx = create_codebase_context_for_task(&task, std::path::Path::new("."));

        assert!(ctx.keywords.contains(&"api".to_string()));
        assert!(ctx.keywords.contains(&"endpoint".to_string()));
        assert!(ctx.keywords.contains(&"handler".to_string()));
        assert!(ctx.keywords.contains(&"service".to_string()));
    }

    #[test]
    fn test_enrich_spec_config_basic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let base_config = SpecConfig::new();

        // Should not fail even without backpressure config
        let enriched = enrich_spec_config(
            base_config,
            temp_dir.path(),
            None,
        ).unwrap();

        // Templates dir doesn't exist, so registry should be None
        assert!(enriched.template_registry.is_none());
    }

    #[test]
    fn test_enrich_spec_config_with_templates_dir() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create templates directory
        let templates_dir = temp_dir.path().join(".descartes/templates");
        std::fs::create_dir_all(&templates_dir).unwrap();

        // Write a template file
        std::fs::write(
            templates_dir.join("builder.toml"),
            r#"name = "builder"
content = """
# Builder Task

{task}

{verification}
"""
description = "Template for builder tasks"
applies_to = ["builder"]
"#,
        ).unwrap();

        let base_config = SpecConfig::new();
        let enriched = enrich_spec_config(
            base_config,
            temp_dir.path(),
            None,
        ).unwrap();

        // Templates should be loaded
        assert!(enriched.template_registry.is_some());
        let registry = enriched.template_registry.unwrap();
        assert!(registry.templates.contains_key("builder"));
    }
}
