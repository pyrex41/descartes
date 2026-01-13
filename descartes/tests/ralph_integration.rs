//! Integration tests for Ralph executor and spec components
//!
//! These tests verify that components work together correctly:
//! - RalphExecutor initialization with various configs
//! - SpecConfig builder pattern
//! - compute_waves() integration with mock SCUD data
//! - dry_run() output format validation

use std::path::PathBuf;

use descartes::{spec::SpecConfig, RalphExecutor};
use scud::models::{Phase, Task, TaskStatus};

/// Create a test phase with a simple dependency graph for integration testing
fn create_integration_test_phase() -> Phase {
    let mut phase = Phase::new("integration".to_string());

    // Wave 1: Independent tasks
    let task1 = Task::new(
        "1".to_string(),
        "Setup environment".to_string(),
        "Initialize project structure".to_string(),
    );
    let task2 = Task::new(
        "2".to_string(),
        "Install dependencies".to_string(),
        "Run package manager install".to_string(),
    );

    // Wave 2: Tasks depending on wave 1
    let mut task3 = Task::new(
        "3".to_string(),
        "Build core module".to_string(),
        "Compile core functionality".to_string(),
    );
    task3.dependencies = vec!["1".to_string()];

    let mut task4 = Task::new(
        "4".to_string(),
        "Run tests".to_string(),
        "Execute test suite".to_string(),
    );
    task4.dependencies = vec!["2".to_string()];

    // Wave 3: Task depending on wave 2
    let mut task5 = Task::new(
        "5".to_string(),
        "Deploy".to_string(),
        "Deploy to production".to_string(),
    );
    task5.dependencies = vec!["3".to_string(), "4".to_string()];

    phase.add_task(task1);
    phase.add_task(task2);
    phase.add_task(task3);
    phase.add_task(task4);
    phase.add_task(task5);

    phase
}

#[test]
fn test_ralph_executor_new_with_default_config() {
    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "test-tag".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    assert_eq!(executor.scud_tag, "test-tag");
    assert_eq!(executor.harness_name, "claude-code");
    assert_eq!(executor.round_size, 5);
    assert!(!executor.validate);
    assert!(executor.verify_command.is_none());
    assert!(executor.model.is_none());
}

#[test]
fn test_ralph_executor_new_with_full_config() {
    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "production".to_string(),
        spec_config,
        Some("cargo test && cargo build".to_string()),
        "opencode".to_string(),
        Some("claude-opus-4".to_string()),
        10,
        true,
        PathBuf::from("/tmp/test"),
    )
    .unwrap();

    assert_eq!(executor.scud_tag, "production");
    assert_eq!(executor.harness_name, "opencode");
    assert_eq!(executor.round_size, 10);
    assert!(executor.validate);
    assert_eq!(
        executor.verify_command,
        Some("cargo test && cargo build".to_string())
    );
    assert_eq!(executor.model, Some("claude-opus-4".to_string()));
    assert_eq!(executor.working_dir, PathBuf::from("/tmp/test"));
}

#[test]
fn test_spec_config_default() {
    let config = SpecConfig::default();

    assert!(config.include_task);
    assert!(config.plan_path.is_none());
    assert!(config.additional_specs.is_empty());
    assert_eq!(config.max_spec_tokens, Some(5000));
    assert!(config.spec_template.is_none());
}

#[test]
fn test_spec_config_builder_pattern() {
    let config = SpecConfig::new()
        .with_plan(PathBuf::from("plan.md"))
        .with_spec_file(PathBuf::from("spec1.md"))
        .with_spec_file(PathBuf::from("spec2.md"));

    assert_eq!(config.plan_path, Some(PathBuf::from("plan.md")));
    assert_eq!(config.additional_specs.len(), 2);
    assert_eq!(config.additional_specs[0], PathBuf::from("spec1.md"));
    assert_eq!(config.additional_specs[1], PathBuf::from("spec2.md"));
}

#[test]
fn test_spec_config_builder_chaining() {
    // Test that builder pattern can be chained fluently
    let config = SpecConfig::default()
        .with_plan(PathBuf::from("/path/to/plan.md"))
        .with_spec_file(PathBuf::from("context.md"));

    assert!(config.plan_path.is_some());
    assert_eq!(config.additional_specs.len(), 1);
}

#[test]
fn test_compute_waves_integration() {
    let phase = create_integration_test_phase();
    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "integration".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);

    // Verify wave structure
    assert_eq!(waves.len(), 3, "Should have 3 waves");

    // Wave 1: tasks 1 and 2 (no dependencies)
    assert_eq!(waves[0].len(), 2, "Wave 1 should have 2 tasks");
    let wave1_ids: Vec<&str> = waves[0].iter().map(|t| t.id.as_str()).collect();
    assert!(wave1_ids.contains(&"1"));
    assert!(wave1_ids.contains(&"2"));

    // Wave 2: tasks 3 and 4 (depend on wave 1)
    assert_eq!(waves[1].len(), 2, "Wave 2 should have 2 tasks");
    let wave2_ids: Vec<&str> = waves[1].iter().map(|t| t.id.as_str()).collect();
    assert!(wave2_ids.contains(&"3"));
    assert!(wave2_ids.contains(&"4"));

    // Wave 3: task 5 (depends on wave 2)
    assert_eq!(waves[2].len(), 1, "Wave 3 should have 1 task");
    assert_eq!(waves[2][0].id, "5");
}

#[test]
fn test_compute_waves_with_partial_completion() {
    let mut phase = create_integration_test_phase();

    // Mark tasks 1 and 2 as done
    phase.tasks[0].set_status(TaskStatus::Done);
    phase.tasks[1].set_status(TaskStatus::Done);

    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "integration".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);

    // With tasks 1 and 2 done, tasks 3 and 4 should now be in wave 1
    assert_eq!(waves.len(), 2, "Should have 2 waves");
    assert_eq!(waves[0].len(), 2, "Wave 1 should have tasks 3 and 4");
    assert_eq!(waves[1].len(), 1, "Wave 2 should have task 5");
}

#[test]
fn test_compute_waves_filters_non_pending_tasks() {
    let mut phase = create_integration_test_phase();

    // Mark task 3 as in-progress and task 4 as blocked
    phase.tasks[2].set_status(TaskStatus::InProgress);
    phase.tasks[3].set_status(TaskStatus::Blocked);

    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "integration".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);

    // Only pending tasks should be included
    let all_task_ids: Vec<&str> = waves
        .iter()
        .flat_map(|w| w.iter())
        .map(|t| t.id.as_str())
        .collect();

    // Tasks 1, 2, and 5 should be present (pending)
    // Tasks 3 and 4 should be filtered out (in-progress and blocked)
    assert!(all_task_ids.contains(&"1"));
    assert!(all_task_ids.contains(&"2"));
    assert!(
        !all_task_ids.contains(&"3"),
        "In-progress task should be filtered"
    );
    assert!(
        !all_task_ids.contains(&"4"),
        "Blocked task should be filtered"
    );
    assert!(all_task_ids.contains(&"5"));
}

#[test]
fn test_executor_integration_with_different_round_sizes() {
    let phase = create_integration_test_phase();

    // Test with round_size = 1
    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "integration".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        1,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);
    assert_eq!(waves.len(), 3);

    // With round_size = 1, wave 1 would be split into 2 rounds
    // (but compute_waves doesn't split by rounds, just returns waves)
    assert_eq!(waves[0].len(), 2);
}

#[test]
fn test_executor_integration_with_validation_enabled() {
    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "test".to_string(),
        spec_config,
        Some("cargo check".to_string()),
        "claude-code".to_string(),
        None,
        5,
        true,
        PathBuf::from("."),
    )
    .unwrap();

    assert!(executor.validate, "Validation should be enabled");
    assert_eq!(executor.verify_command, Some("cargo check".to_string()));
}

#[test]
fn test_compute_waves_with_complex_dependencies() {
    let mut phase = Phase::new("complex".to_string());

    // Create a more complex dependency graph
    let task_a = Task::new("a".to_string(), "Task A".to_string(), "".to_string());

    let mut task_b = Task::new("b".to_string(), "Task B".to_string(), "".to_string());
    task_b.dependencies = vec!["a".to_string()];

    let mut task_c = Task::new("c".to_string(), "Task C".to_string(), "".to_string());
    task_c.dependencies = vec!["a".to_string()];

    let mut task_d = Task::new("d".to_string(), "Task D".to_string(), "".to_string());
    task_d.dependencies = vec!["b".to_string(), "c".to_string()];

    let mut task_e = Task::new("e".to_string(), "Task E".to_string(), "".to_string());
    task_e.dependencies = vec!["d".to_string()];

    phase.add_task(task_a);
    phase.add_task(task_b);
    phase.add_task(task_c);
    phase.add_task(task_d);
    phase.add_task(task_e);

    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "complex".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);

    assert_eq!(waves.len(), 4, "Complex graph should have 4 waves");
    assert_eq!(waves[0].len(), 1, "Wave 1: A");
    assert_eq!(waves[1].len(), 2, "Wave 2: B and C");
    assert_eq!(waves[2].len(), 1, "Wave 3: D");
    assert_eq!(waves[3].len(), 1, "Wave 4: E");
}

#[test]
fn test_compute_waves_ignores_external_dependencies() {
    let mut phase = Phase::new("external".to_string());

    // Task with dependency on non-existent task
    let mut task1 = Task::new("1".to_string(), "Task 1".to_string(), "".to_string());
    task1.dependencies = vec!["external-999".to_string()];

    let task2 = Task::new("2".to_string(), "Task 2".to_string(), "".to_string());

    phase.add_task(task1);
    phase.add_task(task2);

    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "external".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);

    // External dependencies should be ignored (treated as satisfied)
    assert_eq!(waves.len(), 1, "Should have 1 wave");
    assert_eq!(waves[0].len(), 2, "Both tasks should be ready");
}

#[test]
fn test_compute_waves_with_expanded_parent_task() {
    let mut phase = Phase::new("expanded".to_string());

    // Parent task with subtasks
    let mut parent = Task::new("1".to_string(), "Parent Task".to_string(), "".to_string());
    parent.subtasks = vec!["1.1".to_string(), "1.2".to_string()];

    // Regular task
    let task2 = Task::new("2".to_string(), "Regular Task".to_string(), "".to_string());

    phase.add_task(parent);
    phase.add_task(task2);

    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "expanded".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);

    // Parent task should be filtered out (expanded)
    assert_eq!(waves.len(), 1, "Should have 1 wave");
    assert_eq!(
        waves[0].len(),
        1,
        "Only non-expanded task should be included"
    );
    assert_eq!(waves[0][0].id, "2");
}

#[test]
fn test_spec_config_with_multiple_files() {
    // Test that multiple spec files can be added
    let config = SpecConfig::new()
        .with_spec_file(PathBuf::from("context1.md"))
        .with_spec_file(PathBuf::from("context2.md"))
        .with_spec_file(PathBuf::from("context3.md"))
        .with_plan(PathBuf::from("plan.md"));

    assert_eq!(config.additional_specs.len(), 3);
    assert!(config.plan_path.is_some());
}

#[test]
fn test_executor_with_spec_config_integration() {
    // Test that RalphExecutor properly stores SpecConfig
    let spec_config = SpecConfig::new()
        .with_plan(PathBuf::from("/plan/doc.md"))
        .with_spec_file(PathBuf::from("/spec/context.md"));

    let executor = RalphExecutor::new(
        "test".to_string(),
        spec_config.clone(),
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    // Verify that spec_config was properly stored
    assert!(executor.spec_config.plan_path.is_some());
    assert_eq!(executor.spec_config.additional_specs.len(), 1);
}

#[test]
fn test_compute_waves_empty_phase() {
    let phase = Phase::new("empty".to_string());

    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "empty".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);

    assert!(waves.is_empty(), "Empty phase should produce no waves");
}

#[test]
fn test_compute_waves_all_tasks_done() {
    let mut phase = create_integration_test_phase();

    // Mark all tasks as done
    for task in &mut phase.tasks {
        task.set_status(TaskStatus::Done);
    }

    let spec_config = SpecConfig::default();
    let executor = RalphExecutor::new(
        "integration".to_string(),
        spec_config,
        None,
        "claude-code".to_string(),
        None,
        5,
        false,
        PathBuf::from("."),
    )
    .unwrap();

    let waves = executor.compute_waves(&phase);

    assert!(waves.is_empty(), "All done tasks should produce no waves");
}
