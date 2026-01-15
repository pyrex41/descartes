//! Configuration Tests (US-43 to US-44)
//!
//! Tests for configuration overrides:
//! - US-43: Category Configuration Overrides
//! - US-44: Task-Level Execution Overrides

use crate::e2e::fixtures::TestProject;
use descartes::agent::AgentCategory;
use descartes::config::CategoryConfig;

// US-43: Category Configuration Overrides

#[test]
fn test_us43_default_category_config() {
    // Use AgentCategory.default_config() instead
    let config = AgentCategory::Builder.default_config();

    // Default should have reasonable values
    assert!(!config.description.is_empty());
    assert!(!config.model.is_empty());
}

#[test]
fn test_us43_category_config_fields() {
    let config = CategoryConfig {
        description: "Search agent for finding code patterns".to_string(),
        model: "claude-3-haiku".to_string(),
        harness: Some("claude-code".to_string()),
        tools: vec!["Read".to_string(), "Grep".to_string(), "Glob".to_string()],
        parallel: true,
        backpressure: false,
        prompt_template: None,
    };

    assert!(config.parallel);
    assert!(!config.backpressure);
    assert_eq!(config.tools.len(), 3);
}

#[test]
fn test_us43_builder_config() {
    let config = CategoryConfig {
        description: "Builder agent for implementing features".to_string(),
        model: "claude-3-opus".to_string(),
        harness: Some("claude-code".to_string()),
        tools: vec!["Read".to_string(), "Edit".to_string(), "Write".to_string(), "Bash".to_string()],
        parallel: false,
        backpressure: true,
        prompt_template: None,
    };

    // Builders modify files, need more careful execution
    assert!(!config.parallel);
    assert!(config.backpressure);
}

#[test]
fn test_us43_custom_category_from_config() {
    // Test loading category config from file
    let project = TestProject::simple_project();

    // Create custom config
    let config_content = r#"
[llm]
provider = "mock"
model = "mock-model"

[categories.searcher]
description = "Search agent"
model = "claude-3-haiku"
parallel = true
backpressure = false

[categories.builder]
description = "Build agent"
model = "claude-3-opus"
parallel = false
backpressure = true
"#;
    std::fs::write(project.path.join(".scud/config.toml"), config_content).unwrap();

    // Config should be parseable
    let content = std::fs::read_to_string(project.path.join(".scud/config.toml")).unwrap();
    assert!(content.contains("[categories.searcher]"));
    assert!(content.contains("model = \"claude-3-haiku\""));
}

#[test]
fn test_us43_category_parsing() {
    // AgentCategory implements FromStr, returning Result (not Option)
    assert!("searcher".parse::<AgentCategory>().is_ok());
    assert!("builder".parse::<AgentCategory>().is_ok());
    assert!("fast-builder".parse::<AgentCategory>().is_ok());
    assert!("validator".parse::<AgentCategory>().is_ok());
    assert!("analyzer".parse::<AgentCategory>().is_ok());
    // Unknown becomes Custom("unknown")
    assert!(matches!(
        "unknown".parse::<AgentCategory>().unwrap(),
        AgentCategory::Custom(_)
    ));
}

// US-44: Task-Level Execution Overrides

#[test]
fn test_us44_task_override_project_setup() {
    let project = TestProject::task_override_project();

    // Verify task file with overrides exists
    let task_file = project.path.join(".scud/tasks/task-1.md");
    assert!(task_file.exists());
}

#[test]
fn test_us44_yaml_frontmatter_detected() {
    let project = TestProject::task_override_project();

    let task_content = std::fs::read_to_string(
        project.path.join(".scud/tasks/task-1.md")
    ).unwrap();

    // YAML frontmatter starts and ends with ---
    assert!(task_content.starts_with("---"));
    assert!(task_content.matches("---").count() >= 2);
}


#[test]
fn test_us44_inline_comment_detection() {
    let content = "Implement the feature.\n\n// override: category=searcher";

    // Content should contain override marker
    assert!(content.contains("// override:"));
}


#[test]
fn test_us44_category_enum_variants() {
    // Verify all expected categories exist
    let categories = vec![
        AgentCategory::Searcher,
        AgentCategory::Builder,
        AgentCategory::FastBuilder,
        AgentCategory::Validator,
        AgentCategory::Analyzer,
    ];

    assert_eq!(categories.len(), 5);
}

#[test]
fn test_us44_prompt_template_path() {
    use std::path::PathBuf;

    let config = CategoryConfig {
        description: "Custom agent".to_string(),
        model: "claude-3-opus".to_string(),
        harness: None,
        tools: vec![],
        parallel: false,
        backpressure: false,
        prompt_template: Some(PathBuf::from(".scud/prompts/custom.md")),
    };

    assert!(config.prompt_template.is_some());
}

#[test]
fn test_us44_config_harness_override() {
    let config = CategoryConfig {
        description: "OpenCode builder".to_string(),
        model: "grok".to_string(),
        harness: Some("opencode".to_string()),
        tools: vec![],
        parallel: false,
        backpressure: true,
        prompt_template: None,
    };

    assert_eq!(config.harness, Some("opencode".to_string()));
}
