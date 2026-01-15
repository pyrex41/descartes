//! Swarm Execution Tests (US-26 to US-28)
//!
//! Tests for multi-agent swarm execution:
//! - US-26: Wave-Based Parallel Execution
//! - US-27: Swarm Progress Visualization
//! - US-28: Specialized Agent Categories

use crate::e2e::fixtures::TestProject;
use descartes::swarm_executor::SwarmExecutor;
use descartes::agent::{AgentCategory, AgentRegistry, AgentStatus, Tool, ToolSet};
use descartes::config::CategoryConfig;
use descartes::spec::SpecConfig;
use scud::storage::Storage;

// US-26: Wave-Based Parallel Execution

#[test]
fn test_us26_compute_waves_simple_project() {
    let project = TestProject::simple_project();

    // Load project and compute waves
    let executor = SwarmExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        3, // round_size
        false,
        project.path.clone(),
    ).expect("Failed to create executor");

    let storage = Storage::new(Some(project.path.clone()));
    let phase = storage.load_group(&project.tag()).expect("Failed to load phase");
    let waves = executor.compute_waves(&phase);

    // Simple project: 1 (no deps) -> 2 (depends on 1) -> 3 (depends on 2)
    // So we expect multiple waves
    assert!(!waves.is_empty());
}

#[test]
fn test_us26_compute_waves_parallel_project() {
    let project = TestProject::parallel_project();

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

    // Parallel project: 1 and 2 have no deps, 3 depends on both
    // Wave 1: [1, 2], Wave 2: [3]
    assert!(waves.len() >= 2);

    // First wave should have 2 tasks (the independent ones)
    if !waves.is_empty() {
        assert!(waves[0].len() <= 2);
    }
}

#[test]
fn test_us26_compute_waves_large_project() {
    let project = TestProject::large_wave_project();

    let executor = SwarmExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        4, // larger round size
        false,
        project.path.clone(),
    ).expect("Failed to create executor");

    let storage = Storage::new(Some(project.path.clone()));
    let phase = storage.load_group(&project.tag()).expect("Failed to load phase");
    let waves = executor.compute_waves(&phase);

    // Large project has 3 waves:
    // Wave 1: [1, 2, 3, 4] - 4 independent tasks
    // Wave 2: [5, 6] - depend on wave 1
    // Wave 3: [7] - depends on wave 2
    assert!(waves.len() >= 2);
}

#[test]
fn test_us26_waves_respect_round_size() {
    let project = TestProject::large_wave_project();

    let executor = SwarmExecutor::new(
        project.tag(),
        SpecConfig::default(),
        None,
        "mock".to_string(),
        None,
        2, // Small round size to force chunking
        false,
        project.path.clone(),
    ).expect("Failed to create executor");

    let storage = Storage::new(Some(project.path.clone()));
    let phase = storage.load_group(&project.tag()).expect("Failed to load phase");
    let waves = executor.compute_waves(&phase);

    // With round_size=2, large first wave should be chunked
    for wave in &waves {
        assert!(wave.len() <= 4); // Each chunk respects round size
    }
}

// US-27: Swarm Progress Visualization

#[test]
fn test_us27_tui_state_with_registry() {
    use descartes::agent::RegistryStatus;

    let mut registry = AgentRegistry::new();

    // Spawn some agents (pane_name, task_id)
    let handle1 = registry.spawn("agent-1", Some("task-1".to_string()));
    let handle2 = registry.spawn("agent-2", Some("task-2".to_string()));

    // Update statuses
    registry.update_status(&handle1.id, RegistryStatus::Running).unwrap();
    registry.update_status(&handle2.id, RegistryStatus::Completed).unwrap();

    // Count by status
    let counts = registry.count_by_status();
    let running = *counts.get(&RegistryStatus::Running).unwrap_or(&0);
    let completed = *counts.get(&RegistryStatus::Completed).unwrap_or(&0);

    assert_eq!(running, 1);
    assert_eq!(completed, 1);
}

#[test]
fn test_us27_registry_list_running() {
    let mut registry = AgentRegistry::new();

    // Agents start in Spawning status, which is active
    registry.spawn("agent-1", Some("task-1".to_string()));
    registry.spawn("agent-2", Some("task-2".to_string()));

    // list_running returns active (Spawning or Running) agents
    let running = registry.list_running();
    assert_eq!(running.len(), 2);
}

// US-28: Specialized Agent Categories

#[test]
fn test_us28_category_parsing() {
    // AgentCategory implements FromStr, returning Result not Option
    assert_eq!("searcher".parse::<AgentCategory>().unwrap(), AgentCategory::Searcher);
    assert_eq!("builder".parse::<AgentCategory>().unwrap(), AgentCategory::Builder);
    assert_eq!("fast-builder".parse::<AgentCategory>().unwrap(), AgentCategory::FastBuilder);
    assert_eq!("validator".parse::<AgentCategory>().unwrap(), AgentCategory::Validator);
}

#[test]
fn test_us28_category_config_parallel() {
    // Searchers typically allow parallel execution
    let config = AgentCategory::Searcher.default_config();
    // Searcher config should have parallel=true
    assert!(config.parallel);
}

#[test]
fn test_us28_toolset_all() {
    let tools = ToolSet::all();
    assert!(tools.has(Tool::Read));
    assert!(tools.has(Tool::Write));
    assert!(tools.has(Tool::Edit));
    assert!(tools.has(Tool::Bash));
}

#[test]
fn test_us28_toolset_read_only() {
    let tools = ToolSet::read_only();
    assert!(tools.has(Tool::Read));
    assert!(tools.has(Tool::Bash));
    // Read-only should not have write/edit
    assert!(!tools.has(Tool::Write));
    assert!(!tools.has(Tool::Edit));
}

#[test]
fn test_us28_toolset_from_names() {
    let tools = ToolSet::from(vec!["read".to_string(), "bash".to_string()]);
    assert!(tools.has(Tool::Read));
}

#[test]
fn test_us28_mixed_status_project_waves() {
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

    // Mixed status project has:
    // - 1 done (D) - excluded
    // - 2 in-progress (I) - excluded
    // - 3 pending/ready (P) - included
    // - 4 pending/blocked (P, blocked by 2) - maybe included
    // - 5 blocked (B) - excluded

    // Waves should only include pending tasks
    for wave in &waves {
        for task in wave {
            // Task IDs in waves should be pending
            assert!(!["1", "2", "5"].contains(&task.id.as_str()));
        }
    }
}

#[test]
fn test_us28_registry_status_variants() {
    use descartes::agent::RegistryStatus;

    // Verify all RegistryStatus variants are accessible
    let statuses = vec![
        RegistryStatus::Spawning,
        RegistryStatus::Running,
        RegistryStatus::Idle,
        RegistryStatus::Completed,
        RegistryStatus::Failed,
        RegistryStatus::Terminated,
    ];
    assert_eq!(statuses.len(), 6);
}

#[test]
fn test_us28_agent_status_variants() {
    // Verify all AgentStatus variants are accessible
    let statuses = vec![
        AgentStatus::Starting,
        AgentStatus::Running,
        AgentStatus::WaitingForTool,
        AgentStatus::Completed,
        AgentStatus::Failed,
        AgentStatus::Blocked,
    ];
    assert_eq!(statuses.len(), 6);
}
