//! Integration tests for RPC server compatibility
//!
//! These tests verify that the RPC server correctly integrates with:
//! - Unix socket client
//! - Multiple concurrent clients
//! - scud CLI expectations
//! - descartes GUI expectations

use descartes_core::agent_runner::LocalProcessRunner;
use descartes_core::state_store::SqliteStateStore;
use descartes_core::traits::{StateStore, Task, TaskComplexity, TaskPriority, TaskStatus};
use descartes_daemon::{UnixSocketRpcClient, UnixSocketRpcServer};
use std::path::PathBuf;
use std::sync::Arc;
use tempfile::{tempdir, TempDir};
use tokio::time::{sleep, Duration};
use uuid::Uuid;

/// Helper to create test dependencies
/// Returns temp directories to keep them alive for the duration of the test
async fn create_test_server() -> (
    UnixSocketRpcServer,
    Arc<dyn descartes_core::traits::StateStore>,
    PathBuf,
    TempDir, // Keep db directory alive
    TempDir, // Keep socket directory alive
) {
    let agent_runner =
        Arc::new(LocalProcessRunner::new()) as Arc<dyn descartes_core::traits::AgentRunner>;

    let temp_db = tempdir().unwrap();
    let db_path = temp_db.path().join("test.db");
    let mut state_store = SqliteStateStore::new(db_path, false).await.unwrap();
    state_store.initialize().await.unwrap();
    let state_store = Arc::new(state_store) as Arc<dyn descartes_core::traits::StateStore>;

    let socket_dir = tempdir().unwrap();
    let socket_path = socket_dir.path().join("test-rpc.sock");

    let server =
        UnixSocketRpcServer::new(socket_path.clone(), agent_runner, Arc::clone(&state_store));

    (server, state_store, socket_path, temp_db, socket_dir)
}

#[tokio::test]
async fn test_spawn_method_compatibility() {
    let (server, _state_store, socket_path, _temp_db, _temp_socket) = create_test_server().await;

    // Start server in background
    let mut handle = server.start().await.unwrap();
    sleep(Duration::from_millis(100)).await; // Give server time to start

    // Create client
    let client = UnixSocketRpcClient::new(socket_path.clone()).unwrap();

    // Test spawn method with invalid backend - should return proper RPC error
    // Valid backends are: "claude", "opencode", or "*cli"
    let config = serde_json::json!({
        "task": "Write a hello world program",
        "environment": {},
        "system_prompt": "You are a helpful assistant"
    });

    let result = client.spawn("test-agent", "invalid-backend", config).await;

    // Verify RPC layer properly returns errors for unsupported backends
    assert!(result.is_err(), "Spawn with invalid backend should fail");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Unsupported model backend"),
        "Error should mention unsupported backend: {}",
        err
    );

    // Cleanup
    handle.stop().unwrap();
}

#[tokio::test]
async fn test_list_tasks_with_filters() {
    let (server, state_store, socket_path, _temp_db, _temp_socket) = create_test_server().await;

    // Create test tasks
    let task1 = Task {
        id: Uuid::new_v4(),
        title: "Task 1".to_string(),
        description: Some("Description 1".to_string()),
        status: TaskStatus::Todo,
        priority: TaskPriority::High,
        complexity: TaskComplexity::Simple,
        assigned_to: Some("agent-1".to_string()),
        dependencies: vec![],
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    let task2 = Task {
        id: Uuid::new_v4(),
        title: "Task 2".to_string(),
        description: Some("Description 2".to_string()),
        status: TaskStatus::InProgress,
        priority: TaskPriority::Medium,
        complexity: TaskComplexity::Moderate,
        assigned_to: Some("agent-2".to_string()),
        dependencies: vec![],
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    state_store.save_task(&task1).await.unwrap();
    state_store.save_task(&task2).await.unwrap();

    // Start server
    let mut handle = server.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Create client
    let client = UnixSocketRpcClient::new(socket_path).unwrap();

    // Test 1: List all tasks
    let all_tasks = client.list_tasks(None).await.unwrap();
    assert_eq!(all_tasks.len(), 2, "Should find 2 tasks");

    // Test 2: Filter by status
    let filter = serde_json::json!({ "status": "todo" });
    let filtered_tasks = client.list_tasks(Some(filter)).await.unwrap();
    assert_eq!(filtered_tasks.len(), 1, "Should find 1 todo task");
    assert_eq!(filtered_tasks[0].status, "Todo");

    // Test 3: Filter by assigned_to
    let filter = serde_json::json!({ "assigned_to": "agent-2" });
    let filtered_tasks = client.list_tasks(Some(filter)).await.unwrap();
    assert_eq!(filtered_tasks.len(), 1, "Should find 1 task for agent-2");
    assert_eq!(filtered_tasks[0].name, "Task 2");

    // Cleanup
    handle.stop().unwrap();
}

#[tokio::test]
async fn test_approve_workflow() {
    let (server, state_store, socket_path, _temp_db, _temp_socket) = create_test_server().await;

    // Create a test task
    let task = Task {
        id: Uuid::new_v4(),
        title: "Pending Task".to_string(),
        description: Some("Needs approval".to_string()),
        status: TaskStatus::Todo,
        priority: TaskPriority::High,
        complexity: TaskComplexity::Complex,
        assigned_to: Some("agent-1".to_string()),
        dependencies: vec![],
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    let task_id = task.id;
    state_store.save_task(&task).await.unwrap();

    // Start server
    let mut handle = server.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Create client
    let client = UnixSocketRpcClient::new(socket_path).unwrap();

    // Test approval
    let result = client.approve(&task_id.to_string(), true).await.unwrap();

    assert_eq!(result.task_id, task_id.to_string());
    assert!(result.approved);
    assert!(result.timestamp > 0);

    // Verify task status was updated
    let updated_task = state_store.get_task(&task_id).await.unwrap().unwrap();
    assert_eq!(updated_task.status, TaskStatus::InProgress);

    // Test rejection
    let task2 = Task {
        id: Uuid::new_v4(),
        title: "Another Task".to_string(),
        description: Some("Will be rejected".to_string()),
        status: TaskStatus::Todo,
        priority: TaskPriority::Low,
        complexity: TaskComplexity::Simple,
        assigned_to: None,
        dependencies: vec![],
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    let task2_id = task2.id;
    state_store.save_task(&task2).await.unwrap();

    let result = client.approve(&task2_id.to_string(), false).await.unwrap();
    assert!(!result.approved);

    let updated_task2 = state_store.get_task(&task2_id).await.unwrap().unwrap();
    assert_eq!(updated_task2.status, TaskStatus::Blocked);

    // Cleanup
    handle.stop().unwrap();
}

#[tokio::test]
async fn test_get_state_system_and_agent() {
    let (server, state_store, socket_path, _temp_db, _temp_socket) = create_test_server().await;

    // Create some test data
    let task1 = Task {
        id: Uuid::new_v4(),
        title: "Task 1".to_string(),
        description: None,
        status: TaskStatus::Todo,
        priority: TaskPriority::Medium,
        complexity: TaskComplexity::Simple,
        assigned_to: None,
        dependencies: vec![],
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    let task2 = Task {
        id: Uuid::new_v4(),
        title: "Task 2".to_string(),
        description: None,
        status: TaskStatus::InProgress,
        priority: TaskPriority::High,
        complexity: TaskComplexity::Moderate,
        assigned_to: None,
        dependencies: vec![],
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    state_store.save_task(&task1).await.unwrap();
    state_store.save_task(&task2).await.unwrap();

    // Start server
    let mut handle = server.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Create client
    let client = UnixSocketRpcClient::new(socket_path).unwrap();

    // Test system state query
    let state = client.get_state(None).await.unwrap();
    assert_eq!(state["entity_type"], "system");
    assert!(state["agents"].is_object());
    assert!(state["tasks"].is_object());
    assert_eq!(state["tasks"]["total"], 2);
    assert_eq!(state["tasks"]["todo"], 1);
    assert_eq!(state["tasks"]["in_progress"], 1);

    // Cleanup
    handle.stop().unwrap();
}

#[tokio::test]
async fn test_multiple_concurrent_clients() {
    let (server, state_store, socket_path, _temp_db, _temp_socket) = create_test_server().await;

    // Create test tasks
    for i in 0..5 {
        let task = Task {
            id: Uuid::new_v4(),
            title: format!("Task {}", i),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: chrono::Utc::now().timestamp(),
            updated_at: chrono::Utc::now().timestamp(),
            metadata: None,
        };
        state_store.save_task(&task).await.unwrap();
    }

    // Start server
    let mut handle = server.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Create multiple clients and make concurrent requests
    let socket_path_clone = socket_path.clone();
    let client1_task = tokio::spawn(async move {
        let client = UnixSocketRpcClient::new(socket_path_clone).unwrap();
        client.list_tasks(None).await
    });

    let socket_path_clone = socket_path.clone();
    let client2_task = tokio::spawn(async move {
        let client = UnixSocketRpcClient::new(socket_path_clone).unwrap();
        client.get_state(None).await
    });

    let socket_path_clone = socket_path.clone();
    let client3_task = tokio::spawn(async move {
        let client = UnixSocketRpcClient::new(socket_path_clone).unwrap();
        let filter = serde_json::json!({ "status": "todo" });
        client.list_tasks(Some(filter)).await
    });

    // Wait for all requests to complete
    let result1 = client1_task.await.unwrap();
    let result2 = client2_task.await.unwrap();
    let result3 = client3_task.await.unwrap();

    // Verify all requests succeeded
    assert!(result1.is_ok(), "Client 1 request should succeed");
    assert!(result2.is_ok(), "Client 2 request should succeed");
    assert!(result3.is_ok(), "Client 3 request should succeed");

    let tasks = result1.unwrap();
    assert_eq!(tasks.len(), 5, "Should find all 5 tasks");

    // Cleanup
    handle.stop().unwrap();
}

#[tokio::test]
async fn test_error_handling() {
    let (server, _state_store, socket_path, _temp_db, _temp_socket) = create_test_server().await;

    // Start server
    let mut handle = server.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Create client
    let client = UnixSocketRpcClient::new(socket_path).unwrap();

    // Test 1: Invalid task ID format
    let result = client.approve("invalid-uuid", true).await;
    assert!(result.is_err(), "Should fail with invalid UUID");

    // Test 2: Nonexistent task
    let fake_id = Uuid::new_v4().to_string();
    let result = client.approve(&fake_id, true).await;
    assert!(result.is_err(), "Should fail for nonexistent task");

    // Test 3: Invalid entity ID in get_state
    let result = client.get_state(Some("not-a-uuid")).await;
    assert!(result.is_err(), "Should fail with invalid entity ID");

    // Cleanup
    handle.stop().unwrap();
}

#[tokio::test]
async fn test_json_rpc_compliance() {
    let (server, _state_store, socket_path, _temp_db, _temp_socket) = create_test_server().await;

    // Start server
    let mut handle = server.start().await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Create client
    let client = UnixSocketRpcClient::new(socket_path.clone()).unwrap();

    // Test JSON-RPC 2.0 compliance
    let result = client.list_tasks(None).await;
    assert!(result.is_ok(), "Request should succeed");

    // Test connection test
    let client2 = UnixSocketRpcClient::new(socket_path).unwrap();
    let result = client2.test_connection().await;
    assert!(result.is_ok(), "Connection test should succeed");

    // Cleanup
    handle.stop().unwrap();
}
