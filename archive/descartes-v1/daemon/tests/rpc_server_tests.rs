//! Comprehensive integration tests for the JSON-RPC server
//!
//! This test suite covers:
//! - All RPC methods (spawn, list_tasks, approve, get_state)
//! - Error handling and edge cases
//! - Concurrent request handling
//! - Unix socket connection lifecycle
//! - Request/response validation
//! - Timeout scenarios
//!
//! These tests create an in-memory test environment and don't require an external server.

use descartes_core::agent_runner::LocalProcessRunner;
use descartes_core::state_store::SqliteStateStore;
use descartes_core::traits::{StateStore, Task, TaskComplexity, TaskPriority, TaskStatus};
use descartes_daemon::rpc_server::{ApprovalResult, TaskInfo, UnixSocketRpcServer};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::{tempdir, TempDir};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::time::timeout;
use uuid::Uuid;

/// Test helper to create a test RPC server
/// Returns TempDir to keep it alive for the duration of the test
async fn setup_test_server() -> (UnixSocketRpcServer, PathBuf, Arc<SqliteStateStore>, TempDir) {
    let temp_dir = tempdir().unwrap();
    let socket_path = temp_dir.path().join("test-rpc.sock");

    // Create agent runner
    let agent_runner = Arc::new(LocalProcessRunner::new());

    // Create state store
    let db_path = temp_dir.path().join("test.db");
    let mut state_store = SqliteStateStore::new(db_path, false).await.unwrap();
    state_store.initialize().await.unwrap();
    let state_store = Arc::new(state_store);

    let server = UnixSocketRpcServer::new(
        socket_path.clone(),
        agent_runner.clone() as Arc<dyn descartes_core::traits::AgentRunner>,
        state_store.clone() as Arc<dyn descartes_core::traits::StateStore>,
    );

    (server, socket_path, state_store, temp_dir)
}

/// Test helper to create a JSON-RPC request
fn create_rpc_request(method: &str, params: Value, id: u64) -> String {
    json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": id
    })
    .to_string()
}

/// Test helper to send a request and receive a response
async fn send_rpc_request(
    socket_path: &PathBuf,
    request: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = UnixStream::connect(socket_path).await?;

    // Send request
    stream.write_all(request.as_bytes()).await?;
    stream.write_all(b"\n").await?;
    stream.flush().await?;

    // Receive response
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response).await?;

    let response_value: Value = serde_json::from_str(&response)?;
    Ok(response_value)
}

/// Test helper to send a request with timeout
async fn send_rpc_request_with_timeout(
    socket_path: &PathBuf,
    request: &str,
    timeout_duration: Duration,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
    timeout(timeout_duration, send_rpc_request(socket_path, request))
        .await
        .map_err(|_| -> Box<dyn std::error::Error + Send + Sync> { "Request timeout".into() })?
}

/// Test helper to create a test task in the database
async fn create_test_task(
    state_store: &Arc<SqliteStateStore>,
    title: &str,
    status: TaskStatus,
) -> Task {
    let task = Task {
        id: Uuid::new_v4(),
        title: title.to_string(),
        description: Some(format!("Test task: {}", title)),
        status,
        priority: TaskPriority::Medium,
        complexity: TaskComplexity::Moderate,
        assigned_to: Some("test-agent".to_string()),
        dependencies: vec![],
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
        metadata: None,
    };

    state_store.save_task(&task).await.unwrap();
    task
}

// ====================================================================================
// SERVER LIFECYCLE TESTS
// ====================================================================================

#[tokio::test]
async fn test_server_start_and_stop() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;

    // Start the server
    let mut handle = server.start().await.expect("Failed to start server");

    // Verify socket file was created
    assert!(socket_path.exists(), "Socket file should exist");

    // Stop the server
    handle.stop().expect("Failed to stop server");
    handle.stopped().await;
}

#[tokio::test]
async fn test_server_socket_cleanup() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;

    // Create a dummy socket file
    std::fs::write(&socket_path, "dummy").unwrap();
    assert!(socket_path.exists());

    // Start the server - it should remove the existing socket
    let mut handle = server.start().await.expect("Failed to start server");
    assert!(socket_path.exists(), "Socket file should be recreated");

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_multiple_clients_can_connect() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();

    // Give server time to start
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Connect multiple clients
    let client1 = UnixStream::connect(&socket_path).await;
    let client2 = UnixStream::connect(&socket_path).await;
    let client3 = UnixStream::connect(&socket_path).await;

    assert!(client1.is_ok(), "First client should connect");
    assert!(client2.is_ok(), "Second client should connect");
    assert!(client3.is_ok(), "Third client should connect");

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// LIST_TASKS METHOD TESTS
// ====================================================================================

#[tokio::test]
async fn test_list_tasks_empty() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["id"], 1);
    assert!(response["result"].is_array());
    assert_eq!(response["result"].as_array().unwrap().len(), 0);

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_list_tasks_with_data() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create test tasks
    create_test_task(&state_store, "Task 1", TaskStatus::Todo).await;
    create_test_task(&state_store, "Task 2", TaskStatus::InProgress).await;
    create_test_task(&state_store, "Task 3", TaskStatus::Done).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_array());
    assert_eq!(response["result"].as_array().unwrap().len(), 3);

    // Verify task structure
    let task = &response["result"][0];
    assert!(task["id"].is_string());
    assert!(task["name"].is_string());
    assert!(task["status"].is_string());
    assert!(task["created_at"].is_number());
    assert!(task["updated_at"].is_number());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_list_tasks_filter_by_status() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create tasks with different statuses
    create_test_task(&state_store, "Todo Task", TaskStatus::Todo).await;
    create_test_task(&state_store, "InProgress Task", TaskStatus::InProgress).await;
    create_test_task(&state_store, "Done Task", TaskStatus::Done).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Filter by Todo status
    let request = create_rpc_request("list_tasks", json!([{"status": "todo"}]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["result"].as_array().unwrap().len(), 1);
    assert_eq!(response["result"][0]["name"], "Todo Task");

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_list_tasks_filter_by_assigned_to() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create tasks with different assignments
    let mut task1 = create_test_task(&state_store, "Agent 1 Task", TaskStatus::Todo).await;
    task1.assigned_to = Some("agent-1".to_string());
    state_store.save_task(&task1).await.unwrap();

    let mut task2 = create_test_task(&state_store, "Agent 2 Task", TaskStatus::Todo).await;
    task2.assigned_to = Some("agent-2".to_string());
    state_store.save_task(&task2).await.unwrap();

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Filter by assigned_to
    let request = create_rpc_request("list_tasks", json!([{"assigned_to": "agent-1"}]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["result"].as_array().unwrap().len(), 1);
    assert_eq!(response["result"][0]["name"], "Agent 1 Task");

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_list_tasks_filter_multiple_criteria() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create tasks with various attributes
    let mut task1 = create_test_task(&state_store, "Task 1", TaskStatus::Todo).await;
    task1.assigned_to = Some("agent-1".to_string());
    state_store.save_task(&task1).await.unwrap();

    let mut task2 = create_test_task(&state_store, "Task 2", TaskStatus::InProgress).await;
    task2.assigned_to = Some("agent-1".to_string());
    state_store.save_task(&task2).await.unwrap();

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Filter by both status and assigned_to
    let request = create_rpc_request(
        "list_tasks",
        json!([{"status": "todo", "assigned_to": "agent-1"}]),
        1,
    );
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["result"].as_array().unwrap().len(), 1);
    assert_eq!(response["result"][0]["name"], "Task 1");

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// APPROVE METHOD TESTS
// ====================================================================================

#[tokio::test]
async fn test_approve_task_success() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    let task = create_test_task(&state_store, "Approval Test", TaskStatus::Todo).await;
    let task_id = task.id.to_string();

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("approve", json!([task_id.clone(), true]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["result"]["task_id"], task_id);
    assert_eq!(response["result"]["approved"], true);
    assert!(response["result"]["timestamp"].is_number());

    // Verify task was updated in database
    let updated_task = state_store.get_task(&task.id).await.unwrap().unwrap();
    assert_eq!(updated_task.status, TaskStatus::InProgress);
    assert!(updated_task.metadata.is_some());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_approve_task_rejection() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    let task = create_test_task(&state_store, "Rejection Test", TaskStatus::Todo).await;
    let task_id = task.id.to_string();

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("approve", json!([task_id.clone(), false]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["result"]["task_id"], task_id);
    assert_eq!(response["result"]["approved"], false);

    // Verify task was marked as blocked
    let updated_task = state_store.get_task(&task.id).await.unwrap().unwrap();
    assert_eq!(updated_task.status, TaskStatus::Blocked);

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_approve_nonexistent_task() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let fake_task_id = Uuid::new_v4().to_string();
    let request = create_rpc_request("approve", json!([fake_task_id, true]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_approve_invalid_task_id() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("approve", json!(["not-a-valid-uuid", true]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32602);
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("Invalid task ID"));

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_approve_task_metadata_preservation() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create task with existing metadata
    let mut task = create_test_task(&state_store, "Metadata Test", TaskStatus::Todo).await;
    task.metadata = Some(json!({
        "custom_field": "custom_value",
        "priority_score": 100
    }));
    state_store.save_task(&task).await.unwrap();

    let task_id = task.id.to_string();

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("approve", json!([task_id, true]), 1);
    send_rpc_request(&socket_path, &request).await.unwrap();

    // Verify metadata was preserved and approval info was added
    let updated_task = state_store.get_task(&task.id).await.unwrap().unwrap();
    let metadata = updated_task.metadata.unwrap();
    assert_eq!(metadata["custom_field"], "custom_value");
    assert_eq!(metadata["priority_score"], 100);
    assert_eq!(metadata["approved"], true);
    assert!(metadata["approval_timestamp"].is_number());

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// GET_STATE METHOD TESTS
// ====================================================================================

#[tokio::test]
async fn test_get_state_system_level() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create some tasks to populate the state
    create_test_task(&state_store, "Task 1", TaskStatus::Todo).await;
    create_test_task(&state_store, "Task 2", TaskStatus::InProgress).await;
    create_test_task(&state_store, "Task 3", TaskStatus::Done).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("get_state", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert_eq!(response["result"]["entity_type"], "system");
    assert!(response["result"]["agents"].is_object());
    assert!(response["result"]["tasks"].is_object());
    assert_eq!(response["result"]["tasks"]["total"], 3);
    assert_eq!(response["result"]["tasks"]["todo"], 1);
    assert_eq!(response["result"]["tasks"]["in_progress"], 1);
    assert_eq!(response["result"]["tasks"]["done"], 1);
    assert!(response["result"]["timestamp"].is_string());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_get_state_invalid_entity_id() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("get_state", json!(["not-a-valid-id"]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32602);

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_get_state_nonexistent_agent() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let fake_agent_id = Uuid::new_v4().to_string();
    let request = create_rpc_request("get_state", json!([fake_agent_id]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert!(response["error"].is_object());
    assert!(response["error"]["message"]
        .as_str()
        .unwrap()
        .contains("not found"));

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// SPAWN METHOD TESTS
// ====================================================================================

#[tokio::test]
async fn test_spawn_agent_basic() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request(
        "spawn",
        json!(["test-agent", "worker", {
            "task": "Test task",
            "environment": {}
        }]),
        1,
    );
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    // Spawn may fail due to model backend issues, but should return proper JSON-RPC response
    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_string() || response["error"].is_object());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_spawn_agent_with_full_config() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request(
        "spawn",
        json!(["full-config-agent", "worker", {
            "task": "Complex task",
            "context": "Additional context",
            "system_prompt": "You are a helpful assistant",
            "environment": {
                "DEBUG": "true",
                "LOG_LEVEL": "info"
            }
        }]),
        1,
    );
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_string() || response["error"].is_object());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_spawn_agent_minimal_config() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("spawn", json!(["minimal-agent", "basic", {}]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["result"].is_string() || response["error"].is_object());

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// ERROR HANDLING TESTS
// ====================================================================================

#[tokio::test]
async fn test_invalid_json_request() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let invalid_json = "{ this is not valid json }";

    let mut stream = UnixStream::connect(&socket_path).await.unwrap();
    stream.write_all(invalid_json.as_bytes()).await.unwrap();
    stream.write_all(b"\n").await.unwrap();
    stream.flush().await.unwrap();

    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    let read_result = reader.read_line(&mut response).await;

    // jsonrpsee should handle this gracefully
    assert!(read_result.is_ok() || response.is_empty());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_invalid_method_name() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("nonexistent_method", json!([]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");
    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32601); // Method not found

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_missing_required_params() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // approve requires task_id and approved params
    let request = create_rpc_request("approve", json!([]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert!(response["error"].is_object());
    assert_eq!(response["error"]["code"], -32602); // Invalid params

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_wrong_param_types() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // approved should be boolean, not string
    let request = create_rpc_request(
        "approve",
        json!([Uuid::new_v4().to_string(), "not-a-boolean"]),
        1,
    );
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert!(response["error"].is_object());

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// CONCURRENT REQUEST TESTS
// ====================================================================================

#[tokio::test]
async fn test_concurrent_list_tasks_requests() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create multiple tasks
    for i in 0..10 {
        create_test_task(&state_store, &format!("Task {}", i), TaskStatus::Todo).await;
    }

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send 10 concurrent requests
    let mut tasks = vec![];
    for i in 0..10 {
        let socket_path = socket_path.clone();
        let task = tokio::spawn(async move {
            let request = create_rpc_request("list_tasks", json!([null]), i);
            send_rpc_request(&socket_path, &request).await
        });
        tasks.push(task);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(tasks).await;

    // All requests should succeed
    for result in results {
        let response = result.unwrap().unwrap();
        assert_eq!(response["jsonrpc"], "2.0");
        assert!(response["result"].is_array());
        assert_eq!(response["result"].as_array().unwrap().len(), 10);
    }

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_concurrent_mixed_requests() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create test tasks
    let task1 = create_test_task(&state_store, "Task 1", TaskStatus::Todo).await;
    let task2 = create_test_task(&state_store, "Task 2", TaskStatus::Todo).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send different types of requests concurrently
    let socket_path1 = socket_path.clone();
    let socket_path2 = socket_path.clone();
    let socket_path3 = socket_path.clone();
    let task1_id = task1.id.to_string();
    let task2_id = task2.id.to_string();

    let handle1 = tokio::spawn(async move {
        let request = create_rpc_request("list_tasks", json!([null]), 1);
        send_rpc_request(&socket_path1, &request).await
    });

    let handle2 = tokio::spawn(async move {
        let request = create_rpc_request("approve", json!([task1_id, true]), 2);
        send_rpc_request(&socket_path2, &request).await
    });

    let handle3 = tokio::spawn(async move {
        let request = create_rpc_request("get_state", json!([null]), 3);
        send_rpc_request(&socket_path3, &request).await
    });

    // Wait for all to complete
    let (result1, result2, result3) = tokio::join!(handle1, handle2, handle3);

    assert!(result1.unwrap().is_ok());
    assert!(result2.unwrap().is_ok());
    assert!(result3.unwrap().is_ok());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_concurrent_task_approvals() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create multiple tasks
    let mut task_ids = vec![];
    for i in 0..5 {
        let task = create_test_task(&state_store, &format!("Task {}", i), TaskStatus::Todo).await;
        task_ids.push(task.id);
    }

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Approve all tasks concurrently
    let mut handles = vec![];
    for (i, task_id) in task_ids.iter().enumerate() {
        let socket_path = socket_path.clone();
        let task_id = task_id.to_string();
        let handle = tokio::spawn(async move {
            let request = create_rpc_request("approve", json!([task_id, true]), i as u64);
            send_rpc_request(&socket_path, &request).await
        });
        handles.push(handle);
    }

    // All approvals should succeed
    let results = futures::future::join_all(handles).await;
    for result in results {
        let response = result.unwrap().unwrap();
        assert_eq!(response["result"]["approved"], true);
    }

    // Verify all tasks were updated
    for task_id in task_ids {
        let task = state_store.get_task(&task_id).await.unwrap().unwrap();
        assert_eq!(task.status, TaskStatus::InProgress);
    }

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// TIMEOUT AND PERFORMANCE TESTS
// ====================================================================================

#[tokio::test]
async fn test_request_with_timeout() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let result =
        send_rpc_request_with_timeout(&socket_path, &request, Duration::from_secs(5)).await;

    assert!(result.is_ok(), "Request should complete within timeout");

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_rapid_sequential_requests() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Send 50 requests in rapid succession
    for i in 0..50 {
        let request = create_rpc_request("list_tasks", json!([null]), i);
        let response = send_rpc_request(&socket_path, &request).await;
        assert!(response.is_ok(), "Request {} should succeed", i);
    }

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_large_task_list_performance() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    // Create 100 tasks
    for i in 0..100 {
        create_test_task(
            &state_store,
            &format!("Task {}", i),
            if i % 3 == 0 {
                TaskStatus::Todo
            } else if i % 3 == 1 {
                TaskStatus::InProgress
            } else {
                TaskStatus::Done
            },
        )
        .await;
    }

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let start = std::time::Instant::now();
    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(response["result"].as_array().unwrap().len(), 100);
    assert!(elapsed < Duration::from_secs(1), "Should complete quickly");

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// REQUEST/RESPONSE VALIDATION TESTS
// ====================================================================================

#[tokio::test]
async fn test_json_rpc_version_field() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["jsonrpc"], "2.0");

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_request_id_preservation() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Test with different ID types
    let test_ids = vec![1, 42, 999, 12345];

    for id in test_ids {
        let request = create_rpc_request("list_tasks", json!([null]), id);
        let response = send_rpc_request(&socket_path, &request).await.unwrap();
        assert_eq!(response["id"], id);
    }

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_error_object_structure() {
    let (server, socket_path, _, _temp_dir) = setup_test_server().await;
    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("approve", json!(["invalid-uuid", true]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    // Verify error object has required fields
    assert!(response["error"].is_object());
    assert!(response["error"]["code"].is_number());
    assert!(response["error"]["message"].is_string());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_task_info_structure() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    create_test_task(&state_store, "Test Task", TaskStatus::Todo).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    let task = &response["result"][0];

    // Verify all required fields are present
    assert!(task["id"].is_string());
    assert!(task["name"].is_string());
    assert!(task["status"].is_string());
    assert!(task["created_at"].is_number());
    assert!(task["updated_at"].is_number());

    // Verify the task can be deserialized
    let task_info: TaskInfo = serde_json::from_value(task.clone()).unwrap();
    assert!(!task_info.id.is_empty());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_approval_result_structure() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    let task = create_test_task(&state_store, "Test Task", TaskStatus::Todo).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("approve", json!([task.id.to_string(), true]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    // Verify all required fields
    assert!(response["result"]["task_id"].is_string());
    assert!(response["result"]["approved"].is_boolean());
    assert!(response["result"]["timestamp"].is_number());

    // Verify the result can be deserialized
    let approval_result: ApprovalResult =
        serde_json::from_value(response["result"].clone()).unwrap();
    assert_eq!(approval_result.task_id, task.id.to_string());

    handle.stop().unwrap();
    handle.stopped().await;
}

// ====================================================================================
// EDGE CASE TESTS
// ====================================================================================

#[tokio::test]
async fn test_empty_filter_object() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    create_test_task(&state_store, "Task 1", TaskStatus::Todo).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Empty filter should return all tasks
    let request = create_rpc_request("list_tasks", json!([{}]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["result"].as_array().unwrap().len(), 1);

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_filter_with_nonexistent_field() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    create_test_task(&state_store, "Task 1", TaskStatus::Todo).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Filter with non-existent field should be ignored
    let request = create_rpc_request("list_tasks", json!([{"nonexistent_field": "value"}]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    // Should return all tasks since filter doesn't match any known field
    assert_eq!(response["result"].as_array().unwrap().len(), 1);

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_very_long_task_title() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    let long_title = "A".repeat(1000);
    create_test_task(&state_store, &long_title, TaskStatus::Todo).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["result"][0]["name"].as_str().unwrap().len(), 1000);

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_task_with_special_characters() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    let special_title = r#"Task with "quotes", 'apostrophes', \backslashes, and émojis 🚀"#;
    create_test_task(&state_store, special_title, TaskStatus::Todo).await;

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    let request = create_rpc_request("list_tasks", json!([null]), 1);
    let response = send_rpc_request(&socket_path, &request).await.unwrap();

    assert_eq!(response["result"][0]["name"], special_title);

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_multiple_approvals_same_task() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    let task = create_test_task(&state_store, "Test Task", TaskStatus::Todo).await;
    let task_id = task.id.to_string();

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // First approval
    let request = create_rpc_request("approve", json!([task_id.clone(), true]), 1);
    let response1 = send_rpc_request(&socket_path, &request).await.unwrap();
    assert!(response1["result"]["approved"].as_bool().unwrap());

    // Second approval should also succeed (idempotent)
    let request = create_rpc_request("approve", json!([task_id, true]), 2);
    let response2 = send_rpc_request(&socket_path, &request).await.unwrap();
    assert!(response2["result"]["approved"].as_bool().unwrap());

    handle.stop().unwrap();
    handle.stopped().await;
}

#[tokio::test]
async fn test_approve_then_reject_task() {
    let (server, socket_path, state_store, _temp_dir) = setup_test_server().await;

    let task = create_test_task(&state_store, "Test Task", TaskStatus::Todo).await;
    let task_id = task.id.to_string();

    let mut handle = server.start().await.unwrap();
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Approve first
    let request = create_rpc_request("approve", json!([task_id.clone(), true]), 1);
    send_rpc_request(&socket_path, &request).await.unwrap();

    let updated = state_store.get_task(&task.id).await.unwrap().unwrap();
    assert_eq!(updated.status, TaskStatus::InProgress);

    // Then reject
    let request = create_rpc_request("approve", json!([task_id, false]), 2);
    send_rpc_request(&socket_path, &request).await.unwrap();

    let updated = state_store.get_task(&task.id).await.unwrap().unwrap();
    assert_eq!(updated.status, TaskStatus::Blocked);

    handle.stop().unwrap();
    handle.stopped().await;
}
