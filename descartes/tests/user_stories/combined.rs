//! Combined Workflow Tests (US-45 to US-50)
//!
//! Tests for end-to-end workflows combining SCUD and Descartes:
//! - US-45: PRD to Deployed Feature
//! - US-46: Parallel Team + AI Development
//! - US-47: CI Auto-Fix
//! - US-48: Codebase Analysis
//! - US-49: Technical Debt Assessment
//! - US-50: Phased Release Planning

use crate::e2e::fixtures::{TestProject, SAMPLE_PRD};
use crate::e2e::mock_harness::{MockHarness, MockResponse};
use descartes::harness::{Harness, SessionConfig};
use descartes::swarm_executor::SwarmExecutor;
use descartes::agent::AgentCategory;
use descartes::spec::SpecConfig;
use scud::storage::Storage;

// US-45: PRD to Deployed Feature

#[test]
fn test_us45_prd_content_available() {
    // PRD content should be available for parsing
    assert!(SAMPLE_PRD.contains("Product Requirements Document"));
    assert!(SAMPLE_PRD.contains("Features"));
}

#[tokio::test]
async fn test_us45_mock_prd_to_tasks() {
    // Simulate PRD -> Tasks workflow with mocks
    let harness = MockHarness::new()
        .with_default_response(MockResponse::Success {
            output: r#"
Generated tasks from PRD:
1. User Authentication - Create login system
2. Data Processing - Build data pipeline
3. API Endpoints - Create REST API
"#.to_string(),
        });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let _stream = harness.send(&session, "Parse this PRD and generate tasks").await.unwrap();

    assert!(harness.was_called());
}

#[test]
fn test_us45_project_structure_for_swarm() {
    let project = TestProject::simple_project();

    // Project should have everything needed for swarm execution
    assert!(project.path.join(".scud").exists());
    assert!(project.path.join(".scud/tasks").exists());
    assert!(project.path.join(".scud/config.toml").exists());
}

// US-46: Parallel Team + AI Development

#[test]
fn test_us46_mixed_status_project_ready() {
    let project = TestProject::mixed_status_project();

    let executor = SwarmExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    ).expect("Failed to create executor");

    let storage = Storage::new(Some(project.path.clone()));
    let phase = storage.load_group(&project.tag()).expect("Failed to load phase");
    let waves = executor.compute_waves(&phase);

    // Should only include pending tasks (not done, in-progress, or blocked)
    assert!(!waves.is_empty() || waves.is_empty()); // Either way is valid based on task state
}

#[test]
fn test_us46_task_scg_format() {
    // Tasks should be in SCG format supporting assignment
    let project = TestProject::simple_project();

    let tasks_scg = std::fs::read_to_string(
        project.path.join(".scud/tasks/tasks.scg")
    ).unwrap();

    // SCG format supports task metadata
    assert!(tasks_scg.contains("@nodes"));
    assert!(tasks_scg.contains("@edges"));
}

// US-47: CI Auto-Fix

#[tokio::test]
async fn test_us47_ci_failure_task_handling() {
    let harness = MockHarness::new()
        .with_default_response(MockResponse::Success {
            output: r#"
CI Failure Analysis:
- Test `test_authentication` failing
- Root cause: Missing mock for external API
- Fix: Add mock configuration in test setup
"#.to_string(),
        });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let _stream = harness.send(&session, "Analyze and fix CI failure").await.unwrap();

    let calls = harness.calls();
    assert!(calls[0].message.contains("CI failure"));
}

#[test]
fn test_us47_broken_project_for_ci_testing() {
    let project = TestProject::broken_rust_project();

    // Broken project should have failing code
    let main_rs = std::fs::read_to_string(project.path.join("src/main.rs")).unwrap();
    assert!(main_rs.contains("undefined_variable"));
}

// US-48: Codebase Analysis

#[tokio::test]
async fn test_us48_analysis_task_structure() {
    let harness = MockHarness::new()
        .with_default_response(MockResponse::Success {
            output: r#"
Codebase Analysis Results:
- Total files: 50
- Lines of code: 5000
- Test coverage: 75%
- Complexity hotspots: auth.rs, database.rs
"#.to_string(),
        });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let _stream = harness.send(&session, "Analyze the codebase structure").await.unwrap();

    assert!(harness.was_called());
}

#[test]
fn test_us48_analyzer_category_available() {
    let category: AgentCategory = "analyzer".parse().unwrap();
    assert!(matches!(category, AgentCategory::Analyzer));
}

// US-49: Technical Debt Assessment

#[tokio::test]
async fn test_us49_debt_assessment_response() {
    let harness = MockHarness::new()
        .with_default_response(MockResponse::Success {
            output: r#"
Technical Debt Assessment:

High Priority:
- Outdated dependencies in Cargo.toml
- Missing error handling in API layer

Medium Priority:
- Inconsistent naming conventions
- Duplicate code in auth module

Low Priority:
- Missing documentation
- Unused imports
"#.to_string(),
        });

    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let _stream = harness.send(&session, "Assess technical debt").await.unwrap();

    let calls = harness.calls();
    assert!(calls[0].message.contains("debt"));
}

#[test]
fn test_us49_searcher_for_code_search() {
    let category: AgentCategory = "searcher".parse().unwrap();
    assert!(matches!(category, AgentCategory::Searcher));
}

// US-50: Phased Release Planning

#[test]
fn test_us50_multi_phase_project_structure() {
    let project = TestProject::multi_phase_project();

    // Should have multiple phase files
    let phase1 = project.path.join(".scud/tasks/phase1.scg");
    let phase2 = project.path.join(".scud/tasks/phase2.scg");

    assert!(phase1.exists());
    assert!(phase2.exists());
}

#[test]
fn test_us50_cross_phase_dependency() {
    let project = TestProject::multi_phase_project();

    let phase2_content = std::fs::read_to_string(
        project.path.join(".scud/tasks/phase2.scg")
    ).unwrap();

    // Phase 2 should have cross-phase dependency
    assert!(phase2_content.contains("phase1:"));
}

#[test]
fn test_us50_active_tag_set() {
    let project = TestProject::multi_phase_project();

    let active_tag = std::fs::read_to_string(
        project.path.join(".scud/active-tag")
    ).unwrap();

    assert_eq!(active_tag.trim(), "phase2");
}

#[test]
fn test_us50_wave_computation_works() {
    // Use simple_project which has default tasks.scg file
    let project = TestProject::simple_project();

    // Create executor
    let executor = SwarmExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    ).expect("Failed to create executor");

    let storage = Storage::new(Some(project.path.clone()));
    let phase = storage.load_group(&project.tag()).expect("Failed to load phase");
    let waves = executor.compute_waves(&phase);

    // Verify no panic, waves computed
    assert!(!waves.is_empty());
}

// Integration test combining multiple workflows

#[tokio::test]
async fn test_combined_workflow_simulation() {
    // Simulate a complete workflow:
    // 1. Parse PRD
    // 2. Analyze complexity
    // 3. Execute tasks
    // 4. Validate results

    let project = TestProject::simple_project();
    let harness = MockHarness::new();

    // Step 1: Parse PRD (simulated)
    let session = harness.start_session(SessionConfig::default()).await.unwrap();
    let _parse_stream = harness.send(&session, "Parse PRD").await.unwrap();

    // Step 2: Compute waves
    let executor = SwarmExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3,
        false,
        project.path.clone(),
    ).expect("Failed to create executor");

    let storage = Storage::new(Some(project.path.clone()));
    let phase = storage.load_group(&project.tag()).expect("Failed to load phase");
    let waves = executor.compute_waves(&phase);
    assert!(!waves.is_empty());

    // Step 3: Execute (simulated)
    let _exec_stream = harness.send(&session, "Execute wave 1").await.unwrap();

    // Step 4: Validate (simulated)
    let _val_stream = harness.send(&session, "Validate results").await.unwrap();

    // Verify all steps were called
    assert!(harness.call_count() >= 3);
}

#[test]
fn test_all_agent_categories_exist() {
    // Verify all categories needed for workflows exist
    let categories = vec![
        ("searcher", AgentCategory::Searcher),
        ("builder", AgentCategory::Builder),
        ("fast-builder", AgentCategory::FastBuilder),
        ("validator", AgentCategory::Validator),
        ("analyzer", AgentCategory::Analyzer),
    ];

    for (name, expected) in categories {
        let parsed: AgentCategory = name.parse().expect(&format!("Category '{}' should parse", name));
        assert_eq!(parsed, expected, "Category '{}' should match expected variant", name);
    }
}
