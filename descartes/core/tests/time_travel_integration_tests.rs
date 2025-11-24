//! Integration Tests for Time Travel Rewind and Resume
//!
//! These tests verify the complete rewind/resume workflow including:
//! - Brain and body coordination
//! - State consistency validation
//! - Safety features (backups, confirmations)
//! - Error handling and rollback
//! - Undo functionality

use descartes_core::{
    agent_history::{AgentHistoryEvent, HistoryEventType, SqliteAgentHistoryStore},
    body_restore::GitBodyRestoreManager,
    time_travel_integration::{
        DefaultRewindManager, RewindConfig, RewindManager, RewindPoint, ResumeContext,
    },
};
use serde_json::json;
use std::process::Command;
use std::sync::Arc;
use tempfile::{NamedTempFile, TempDir};
use tokio;

// ============================================================================
// TEST HELPERS
// ============================================================================

async fn create_test_history_store() -> SqliteAgentHistoryStore {
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path().to_str().unwrap();
    let mut store = SqliteAgentHistoryStore::new(path).await.unwrap();
    store.initialize().await.unwrap();
    store
}

fn create_test_git_repo() -> (TempDir, std::path::PathBuf) {
    let temp_dir = TempDir::new().unwrap();
    let repo_path = temp_dir.path().to_path_buf();

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to init git repo");

    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Test User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "test@example.com"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create initial commit
    std::fs::write(repo_path.join("test.txt"), "initial content").unwrap();
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial commit"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    (temp_dir, repo_path)
}

fn create_git_commit(repo_path: &std::path::Path, content: &str, message: &str) -> String {
    std::fs::write(repo_path.join("test.txt"), content).unwrap();
    Command::new("git")
        .args(["add", "test.txt"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Get commit hash
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

async fn create_test_events(
    store: &SqliteAgentHistoryStore,
    agent_id: &str,
    commit_hash: Option<String>,
) -> Vec<AgentHistoryEvent> {
    let mut events = vec![
        AgentHistoryEvent::new(
            agent_id.to_string(),
            HistoryEventType::Thought,
            json!({"content": "Analyzing problem", "thought_type": "analysis"}),
        ),
        AgentHistoryEvent::new(
            agent_id.to_string(),
            HistoryEventType::Decision,
            json!({"decision_type": "action_selection", "context": {"options": ["A", "B"]}, "outcome": "A"}),
        ),
        AgentHistoryEvent::new(
            agent_id.to_string(),
            HistoryEventType::Action,
            json!({"action": "execute_plan", "parameters": {}}),
        ),
    ];

    // Add commit hash to last event if provided
    if let Some(commit) = commit_hash {
        events.last_mut().unwrap().git_commit_hash = Some(commit);
    }

    // Record events
    for event in &events {
        store.record_event(event).await.unwrap();
    }

    events
}

// ============================================================================
// BASIC FUNCTIONALITY TESTS
// ============================================================================

#[tokio::test]
async fn test_create_rewind_manager() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let manager = DefaultRewindManager::new(store, repo_path, 10);
    assert!(manager.is_ok());
}

#[tokio::test]
async fn test_get_rewind_points() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    // Create events with git commits
    let commit = create_git_commit(&repo_path, "content 1", "Commit 1");
    create_test_events(&store, "agent-1", Some(commit.clone())).await;

    // Add more commits
    let commit2 = create_git_commit(&repo_path, "content 2", "Commit 2");
    let event = AgentHistoryEvent::new(
        "agent-1".to_string(),
        HistoryEventType::StateChange,
        json!({"state": "new_state"}),
    )
    .with_git_commit(commit2);
    store.record_event(&event).await.unwrap();

    let manager = DefaultRewindManager::new(store, repo_path, 10).unwrap();
    let points = manager.get_rewind_points("agent-1").await.unwrap();

    assert!(!points.is_empty(), "Should have rewind points");
    assert!(
        points.iter().any(|p| p.git_commit.is_some()),
        "Should have points with git commits"
    );
}

#[tokio::test]
async fn test_can_rewind_to() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let commit = create_git_commit(&repo_path, "content", "Test commit");
    create_test_events(&store, "agent-1", Some(commit.clone())).await;

    let manager = DefaultRewindManager::new(store, repo_path, 10).unwrap();

    let point = RewindPoint {
        timestamp: chrono::Utc::now().timestamp(),
        event_id: None,
        git_commit: Some(commit),
        snapshot_id: None,
        description: "Test point".to_string(),
        event_index: Some(0),
    };

    let confirmation = manager.can_rewind_to(&point).await;
    assert!(confirmation.is_ok(), "Should be able to rewind to valid point");
}

// ============================================================================
// REWIND WORKFLOW TESTS
// ============================================================================

#[tokio::test]
async fn test_rewind_to_point() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    // Create first commit with events
    let commit1 = create_git_commit(&repo_path, "content 1", "Commit 1");
    let events1 = create_test_events(&store, "agent-1", Some(commit1.clone())).await;

    // Create second commit with more events
    let commit2 = create_git_commit(&repo_path, "content 2", "Commit 2");
    let event = AgentHistoryEvent::new(
        "agent-1".to_string(),
        HistoryEventType::Action,
        json!({"action": "new_action"}),
    )
    .with_git_commit(commit2.clone());
    store.record_event(&event).await.unwrap();

    // Verify we're at commit 2
    let body_manager = GitBodyRestoreManager::new(&repo_path).unwrap();
    let current = body_manager.get_current_commit().await.unwrap();
    assert!(
        current.starts_with(&commit2.chars().take(7).collect::<String>()),
        "Should be at commit 2"
    );

    // Rewind to commit 1
    let manager = DefaultRewindManager::new(Arc::clone(&store), repo_path.clone(), 10).unwrap();

    let point = RewindPoint {
        timestamp: events1[0].timestamp,
        event_id: Some(events1[0].event_id),
        git_commit: Some(commit1.clone()),
        snapshot_id: None,
        description: "Rewind to commit 1".to_string(),
        event_index: Some(0),
    };

    let config = RewindConfig {
        require_confirmation: false,
        auto_backup: true,
        validate_state: true,
        allow_uncommitted_changes: true,
        max_undo_history: 10,
        enable_debugging: false,
    };

    let result = manager.rewind_to(point, config).await;
    assert!(result.is_ok(), "Rewind should succeed");

    let result = result.unwrap();
    assert!(result.success, "Rewind should be successful");
    assert!(result.brain_result.is_some(), "Should have brain result");
    assert!(result.body_result.is_some(), "Should have body result");

    // Verify we're now at commit 1
    let current = body_manager.get_current_commit().await.unwrap();
    assert!(
        current.starts_with(&commit1.chars().take(7).collect::<String>()),
        "Should be at commit 1 after rewind"
    );
}

#[tokio::test]
async fn test_rewind_with_validation() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let commit = create_git_commit(&repo_path, "content", "Test commit");
    let events = create_test_events(&store, "agent-1", Some(commit.clone())).await;

    let manager = DefaultRewindManager::new(store, repo_path, 10).unwrap();

    let point = RewindPoint {
        timestamp: events[0].timestamp,
        event_id: Some(events[0].event_id),
        git_commit: Some(commit),
        snapshot_id: None,
        description: "Test".to_string(),
        event_index: Some(0),
    };

    let config = RewindConfig {
        validate_state: true,
        allow_uncommitted_changes: true,
        ..RewindConfig::safe()
    };

    let result = manager.rewind_to(point, config).await.unwrap();

    assert!(result.success);
    assert!(
        result.validation.valid,
        "Validation should pass: {:?}",
        result.validation.errors
    );
}

#[tokio::test]
async fn test_rewind_creates_backup() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let commit = create_git_commit(&repo_path, "content", "Test commit");
    let events = create_test_events(&store, "agent-1", Some(commit.clone())).await;

    let manager = DefaultRewindManager::new(store, repo_path, 10).unwrap();

    let point = RewindPoint {
        timestamp: events[0].timestamp,
        event_id: Some(events[0].event_id),
        git_commit: Some(commit),
        snapshot_id: None,
        description: "Test".to_string(),
        event_index: Some(0),
    };

    let config = RewindConfig {
        auto_backup: true,
        allow_uncommitted_changes: true,
        ..Default::default()
    };

    let result = manager.rewind_to(point, config).await.unwrap();

    assert!(result.success);
    assert!(
        !result.backup.backup_id.is_nil(),
        "Should have backup ID"
    );
    assert!(
        result.backup.repository_state.head_commit.len() > 0,
        "Should have repository backup"
    );
}

// ============================================================================
// UNDO TESTS
// ============================================================================

#[tokio::test]
async fn test_undo_rewind() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    // Create two commits
    let commit1 = create_git_commit(&repo_path, "content 1", "Commit 1");
    let events1 = create_test_events(&store, "agent-1", Some(commit1.clone())).await;

    let commit2 = create_git_commit(&repo_path, "content 2", "Commit 2");
    let event = AgentHistoryEvent::new(
        "agent-1".to_string(),
        HistoryEventType::Action,
        json!({"action": "action"}),
    )
    .with_git_commit(commit2.clone());
    store.record_event(&event).await.unwrap();

    let manager = DefaultRewindManager::new(Arc::clone(&store), repo_path.clone(), 10).unwrap();

    // Record original commit
    let body_manager = GitBodyRestoreManager::new(&repo_path).unwrap();
    let original_commit = body_manager.get_current_commit().await.unwrap();

    // Rewind to commit 1
    let point = RewindPoint {
        timestamp: events1[0].timestamp,
        event_id: Some(events1[0].event_id),
        git_commit: Some(commit1.clone()),
        snapshot_id: None,
        description: "Rewind".to_string(),
        event_index: Some(0),
    };

    let config = RewindConfig::fast();
    let rewind_result = manager.rewind_to(point, config).await.unwrap();
    assert!(rewind_result.success);

    let backup_id = rewind_result.backup.backup_id;

    // Verify we're at commit 1
    let current = body_manager.get_current_commit().await.unwrap();
    assert!(current.starts_with(&commit1.chars().take(7).collect::<String>()));

    // Undo the rewind
    let undo_result = manager.undo_rewind(backup_id).await.unwrap();
    assert!(undo_result.success, "Undo should succeed");

    // Verify we're back at the original commit
    let current = body_manager.get_current_commit().await.unwrap();
    assert_eq!(
        current, original_commit,
        "Should be back at original commit after undo"
    );
}

// ============================================================================
// RESUME TESTS
// ============================================================================

#[tokio::test]
async fn test_resume_context_creation() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let commit = create_git_commit(&repo_path, "content", "Test commit");
    let events = create_test_events(&store, "agent-1", Some(commit.clone())).await;

    let manager = DefaultRewindManager::new(store, repo_path, 10).unwrap();

    let point = RewindPoint {
        timestamp: events[0].timestamp,
        event_id: Some(events[0].event_id),
        git_commit: Some(commit),
        snapshot_id: None,
        description: "Test".to_string(),
        event_index: Some(1),
    };

    let config = RewindConfig::fast();
    let result = manager.rewind_to(point, config).await.unwrap();

    // Create resume context from result
    let resume_ctx = ResumeContext::from_rewind_result(&result, "agent-1".to_string());
    assert!(resume_ctx.is_ok(), "Should create resume context");

    let ctx = resume_ctx.unwrap();
    assert_eq!(ctx.agent_id, "agent-1");
    assert_eq!(ctx.resume_event_index, 1);
    assert!(!ctx.git_commit.is_empty());
}

#[tokio::test]
async fn test_resume_from_context() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let commit = create_git_commit(&repo_path, "content", "Test commit");
    let events = create_test_events(&store, "agent-1", Some(commit.clone())).await;

    let manager = DefaultRewindManager::new(Arc::clone(&store), repo_path, 10).unwrap();

    let point = RewindPoint {
        timestamp: events[0].timestamp,
        event_id: Some(events[0].event_id),
        git_commit: Some(commit),
        snapshot_id: None,
        description: "Test".to_string(),
        event_index: Some(0),
    };

    let config = RewindConfig::fast();
    let result = manager.rewind_to(point, config).await.unwrap();

    let ctx = ResumeContext::from_rewind_result(&result, "agent-1".to_string()).unwrap();

    // Resume should work without errors
    let resume_result = manager.resume_from(ctx).await;
    assert!(resume_result.is_ok(), "Resume should succeed");
}

// ============================================================================
// SNAPSHOT TESTS
// ============================================================================

#[tokio::test]
async fn test_create_snapshot() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let commit = create_git_commit(&repo_path, "content", "Test commit");
    create_test_events(&store, "agent-1", Some(commit)).await;

    let manager = DefaultRewindManager::new(store, repo_path, 10).unwrap();

    let snapshot_id = manager
        .create_snapshot("agent-1", "Test snapshot".to_string())
        .await;

    assert!(snapshot_id.is_ok(), "Should create snapshot");
    assert!(!snapshot_id.unwrap().is_nil(), "Should have valid snapshot ID");
}

#[tokio::test]
async fn test_rewind_to_snapshot() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let commit = create_git_commit(&repo_path, "content", "Test commit");
    create_test_events(&store, "agent-1", Some(commit)).await;

    let manager = DefaultRewindManager::new(Arc::clone(&store), repo_path, 10).unwrap();

    // Create snapshot
    let snapshot_id = manager
        .create_snapshot("agent-1", "Test snapshot".to_string())
        .await
        .unwrap();

    // Get rewind points should include the snapshot
    let points = manager.get_rewind_points("agent-1").await.unwrap();

    let snapshot_point = points
        .iter()
        .find(|p| p.snapshot_id == Some(snapshot_id));

    assert!(
        snapshot_point.is_some(),
        "Should find snapshot in rewind points"
    );
}

// ============================================================================
// ERROR HANDLING TESTS
// ============================================================================

#[tokio::test]
async fn test_rewind_to_nonexistent_commit() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    create_test_events(&store, "agent-1", None).await;

    let manager = DefaultRewindManager::new(store, repo_path, 10).unwrap();

    let point = RewindPoint {
        timestamp: chrono::Utc::now().timestamp(),
        event_id: None,
        git_commit: Some("0000000000000000000000000000000000000000".to_string()),
        snapshot_id: None,
        description: "Invalid commit".to_string(),
        event_index: Some(0),
    };

    let config = RewindConfig::fast();
    let result = manager.rewind_to(point, config).await;

    assert!(result.is_err(), "Should fail with nonexistent commit");
}

#[tokio::test]
async fn test_validation_catches_inconsistencies() {
    let store = Arc::new(create_test_history_store().await);
    let (_temp, repo_path) = create_test_git_repo();

    let commit1 = create_git_commit(&repo_path, "content 1", "Commit 1");
    let commit2 = create_git_commit(&repo_path, "content 2", "Commit 2");

    // Create events with commit1, but we're actually at commit2
    let events = create_test_events(&store, "agent-1", Some(commit1.clone())).await;

    let manager = DefaultRewindManager::new(store, repo_path.clone(), 10).unwrap();

    // Get brain state (which references commit1)
    use descartes_core::brain_restore::{BrainRestore, DefaultBrainRestore, RestoreOptions};
    let brain_restore = DefaultBrainRestore::new(Arc::new(create_test_history_store().await));
    let brain_result = brain_restore
        .replay_events(events, RestoreOptions::default())
        .await
        .unwrap();

    let brain_state = brain_result.brain_state.unwrap();

    // Validate consistency (should catch mismatch)
    let validation = manager
        .validate_consistency(&brain_state, &commit2)
        .await
        .unwrap();

    // Should detect inconsistency
    assert!(
        !validation.brain_body_consistent || !validation.errors.is_empty(),
        "Should detect brain/body inconsistency"
    );
}

// ============================================================================
// SLIDER INTEGRATION TESTS
// ============================================================================

#[tokio::test]
async fn test_slider_to_rewind_point() {
    use descartes_core::slider_to_rewind_point;

    let events = vec![
        AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            json!({"content": "event 1"}),
        ),
        AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Action,
            json!({"action": "event 2"}),
        ),
        AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Decision,
            json!({"decision": "event 3"}),
        ),
    ];

    // Test beginning
    let point = slider_to_rewind_point(0.0, &events);
    assert!(point.is_some());
    assert_eq!(point.unwrap().event_index, Some(0));

    // Test middle
    let point = slider_to_rewind_point(0.5, &events);
    assert!(point.is_some());
    assert_eq!(point.unwrap().event_index, Some(1));

    // Test end
    let point = slider_to_rewind_point(1.0, &events);
    assert!(point.is_some());
    assert_eq!(point.unwrap().event_index, Some(2));

    // Test empty
    let point = slider_to_rewind_point(0.5, &[]);
    assert!(point.is_none());
}
