//! Spec configuration for Swarm loop
//!
//! Implements Geoff's "fixed spec allocation" pattern:
//! ~5k tokens of persistent context at the start of each prompt.

use std::fs;
use std::path::PathBuf;

use regex::Regex;
use tracing::warn;

use crate::scud::Task;
use crate::Result;

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
    /// Placeholders: {task}, {plan}, {custom}, {verification}
    pub spec_template: Option<String>,
}

impl Default for SpecConfig {
    fn default() -> Self {
        Self {
            include_task: true,
            plan_path: None,
            additional_specs: Vec::new(),
            max_spec_tokens: Some(5000),
            spec_template: None,
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
}

/// Default template for combining spec sections
const DEFAULT_SPEC_TEMPLATE: &str = r#"# Task Specification

{task}

{plan}

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

/// Apply a template with placeholder substitution
///
/// Placeholders: {task}, {plan}, {custom}, {verification}
pub fn apply_spec_template(
    template: &str,
    task: Option<&str>,
    plan: Option<&str>,
    custom: Option<&str>,
    verification: Option<&str>,
) -> String {
    template
        .replace("{task}", task.unwrap_or(""))
        .replace("{plan}", plan.unwrap_or(""))
        .replace("{custom}", custom.unwrap_or(""))
        .replace("{verification}", verification.unwrap_or(""))
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

    // Apply template
    let template = config
        .spec_template
        .as_deref()
        .unwrap_or(DEFAULT_SPEC_TEMPLATE);

    let spec = apply_spec_template(
        template,
        task_section.as_deref(),
        plan_section.as_deref(),
        custom_section.as_deref(),
        None, // verification placeholder reserved for future use
    );

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
}
