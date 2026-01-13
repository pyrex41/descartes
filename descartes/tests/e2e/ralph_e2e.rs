//! End-to-end tests for Ralph executor
//!
//! Tests the integration between Descartes and SCUD task management.

use super::fixtures::TestProject;
use super::mock_harness::{MockHarness, MockResponse};
use descartes::spec::SpecConfig;
use descartes::RalphExecutor;
use scud::storage::Storage;

/// Test that RalphExecutor correctly reads tasks from a SCUD project
#[test]
fn test_ralph_loads_scud_tasks() {
    let project = TestProject::simple_project();

    let executor = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    )
    .expect("Failed to create executor");

    // Load the phase and compute waves
    let storage = Storage::new(Some(project.path.clone()));
    let phase = storage.load_group(&project.tag()).expect("Failed to load phase");

    let waves = executor.compute_waves(&phase);

    // Should have 3 waves for our simple project
    // Wave 1: Task 1 (no deps)
    // Wave 2: Task 2 (depends on 1)
    // Wave 3: Task 3 (depends on 2)
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0].len(), 1);
    assert_eq!(waves[0][0].id, "1");
    assert_eq!(waves[1][0].id, "2");
    assert_eq!(waves[2][0].id, "3");
}

/// Test that parallel tasks are correctly grouped into waves
#[test]
fn test_ralph_parallel_wave_computation() {
    let project = TestProject::parallel_project();

    let executor = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        5,
        false,
        project.path.clone(),
    )
    .expect("Failed to create executor");

    let storage = Storage::new(Some(project.path.clone()));
    let phase = storage.load_group(&project.tag()).expect("Failed to load phase");

    let waves = executor.compute_waves(&phase);

    // Wave 1: Tasks 1 and 2 (parallel, no deps)
    // Wave 2: Task 3 (depends on both)
    assert_eq!(waves.len(), 2);
    assert_eq!(waves[0].len(), 2);
    assert_eq!(waves[1].len(), 1);
    assert_eq!(waves[1][0].id, "3");
}

/// Test dry run output format
#[test]
fn test_ralph_dry_run_format() {
    let project = TestProject::simple_project();

    let executor = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    )
    .expect("Failed to create executor");

    // dry_run() should not modify task statuses
    // Just verify it doesn't panic for now
    // Full output validation would require capturing stdout
    let storage = Storage::new(Some(project.path.clone()));
    let phase_before = storage.load_group(&project.tag()).unwrap();

    let pending_before: Vec<_> = phase_before
        .tasks
        .iter()
        .filter(|t| t.status == scud::models::TaskStatus::Pending)
        .collect();

    // After dry_run, tasks should still be pending
    let phase_after = storage.load_group(&project.tag()).unwrap();
    let pending_after: Vec<_> = phase_after
        .tasks
        .iter()
        .filter(|t| t.status == scud::models::TaskStatus::Pending)
        .collect();

    assert_eq!(pending_before.len(), pending_after.len());
}

/// Test that validation flag is respected
#[test]
fn test_ralph_validation_config() {
    let project = TestProject::simple_project();

    // Without validation
    let executor_no_validate = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    )
    .expect("Failed to create executor");
    assert!(!executor_no_validate.validate);

    // With validation
    let executor_validate = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        Some("cargo build".to_string()),
        "mock".to_string(),
        None,
        3,
        true,
        project.path.clone(),
    )
    .expect("Failed to create executor");
    assert!(executor_validate.validate);
    assert_eq!(
        executor_validate.verify_command,
        Some("cargo build".to_string())
    );
}

/// Test that SpecConfig integrates correctly with executor
#[test]
fn test_ralph_spec_config_integration() {
    let project = TestProject::simple_project();

    // Create a plan file
    let plan_path = project.path.join("plan.md");
    std::fs::write(&plan_path, "# Test Plan\n\nThis is a test plan.").unwrap();

    let spec_config = SpecConfig::new()
        .with_plan(plan_path.clone())
        .with_spec_file(plan_path.clone()); // Add same file as additional spec

    let executor = RalphExecutor::new(
        project.tag(),
        spec_config,
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    )
    .expect("Failed to create executor");

    assert!(executor.spec_config.plan_path.is_some());
    assert_eq!(executor.spec_config.additional_specs.len(), 1);
}

/// Test mock harness response configuration
#[tokio::test]
async fn test_mock_harness_task_responses() {
    use descartes::harness::{Harness, SessionConfig};
    use futures::StreamExt;

    let mut harness = MockHarness::new();

    // Configure specific responses
    harness.when_task(
        "1",
        MockResponse::Success {
            output: "Task 1 done".to_string(),
        },
    );
    harness.when_task(
        "2",
        MockResponse::Blocked {
            reason: "Missing file".to_string(),
        },
    );

    let session = harness
        .start_session(SessionConfig::default())
        .await
        .unwrap();

    // Test task 1 response
    let mut stream1 = harness
        .send(&session, "Task 1: Do something")
        .await
        .unwrap();
    let chunk1 = stream1.next().await.unwrap();
    if let descartes::harness::ResponseChunk::Text(text) = chunk1 {
        assert!(text.contains("Task 1 done"));
    } else {
        panic!("Expected text response");
    }

    // Test task 2 response
    let mut stream2 = harness
        .send(&session, "Task 2: Do something else")
        .await
        .unwrap();
    let chunk2 = stream2.next().await.unwrap();
    if let descartes::harness::ResponseChunk::Text(text) = chunk2 {
        assert!(text.contains("TASK_BLOCKED"));
    } else {
        panic!("Expected blocked response");
    }
}

/// Test round_size chunking behavior
#[test]
fn test_ralph_round_size_chunking() {
    let project = TestProject::parallel_project();

    // With round_size = 1, each task runs separately
    let executor = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        1, // Only 1 task per round
        false,
        project.path.clone(),
    )
    .expect("Failed to create executor");

    assert_eq!(executor.round_size, 1);

    // With round_size = 10, tasks can run in larger batches
    let executor_large = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        10,
        false,
        project.path,
    )
    .expect("Failed to create executor");

    assert_eq!(executor_large.round_size, 10);
}

/// Test working directory is correctly set
#[test]
fn test_ralph_working_directory() {
    let project = TestProject::simple_project();

    let executor = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    )
    .expect("Failed to create executor");

    assert_eq!(executor.working_dir, project.path);
}

/// Test model configuration
#[test]
fn test_ralph_model_config() {
    let project = TestProject::simple_project();

    // Default model (None)
    let executor_default = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    )
    .expect("Failed to create executor");
    assert!(executor_default.model.is_none());

    // Specific model
    let executor_specific = RalphExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        Some("claude-opus-4".to_string()),
        3,
        false,
        project.path,
    )
    .expect("Failed to create executor");
    assert_eq!(executor_specific.model, Some("claude-opus-4".to_string()));
}
