/// Integration tests for RPC client
///
/// These tests require a running daemon server.
/// Run the daemon first: `cargo run --bin descartes-daemon`
use descartes_daemon::{RpcClient, RpcClientBuilder, RpcClientConfig};
use serde_json::json;

#[tokio::test]
#[ignore] // Requires running server
async fn test_client_health_check() {
    let client = RpcClient::with_url("http://127.0.0.1:8080").unwrap();

    let health = client.health().await;
    assert!(health.is_ok(), "Health check should succeed: {:?}", health);

    let health = health.unwrap();
    assert_eq!(health.status, "healthy");
}

#[tokio::test]
#[ignore] // Requires running server
async fn test_client_list_agents() {
    let client = RpcClient::with_url("http://127.0.0.1:8080").unwrap();

    let agents = client.list_agents().await;
    assert!(agents.is_ok(), "List agents should succeed: {:?}", agents);
}

#[tokio::test]
#[ignore] // Requires running server
async fn test_client_spawn_and_kill_agent() {
    let client = RpcClient::with_url("http://127.0.0.1:8080").unwrap();

    // Spawn agent
    let spawn_result = client.spawn_agent("test-agent", "basic", json!({})).await;

    assert!(
        spawn_result.is_ok(),
        "Spawn should succeed: {:?}",
        spawn_result
    );

    let spawn_response = spawn_result.unwrap();
    let agent_id = spawn_response.agent_id;

    // Kill agent
    let kill_result = client.kill_agent(&agent_id, false).await;
    assert!(
        kill_result.is_ok(),
        "Kill should succeed: {:?}",
        kill_result
    );
}

#[tokio::test]
#[ignore] // Requires running server
async fn test_client_metrics() {
    let client = RpcClient::with_url("http://127.0.0.1:8080").unwrap();

    let metrics = client.metrics().await;
    assert!(metrics.is_ok(), "Metrics should succeed: {:?}", metrics);
}

#[tokio::test]
#[ignore] // Requires running server
async fn test_client_state_query() {
    let client = RpcClient::with_url("http://127.0.0.1:8080").unwrap();

    let state = client.query_state(None, None).await;
    assert!(state.is_ok(), "State query should succeed: {:?}", state);
}

#[tokio::test]
#[ignore] // Requires running server
async fn test_client_batch_requests() {
    let client = RpcClient::with_url("http://127.0.0.1:8080").unwrap();

    let requests = vec![
        ("system.health", None),
        ("agent.list", Some(json!({}))),
        ("system.metrics", None),
    ];

    let results = client.batch_call(requests).await;
    assert!(results.is_ok(), "Batch call should succeed: {:?}", results);

    let results = results.unwrap();
    assert_eq!(results.len(), 3, "Should have 3 results");
}

#[tokio::test]
#[ignore] // Requires running server
async fn test_client_connection_test() {
    let client = RpcClient::with_url("http://127.0.0.1:8080").unwrap();

    let result = client.test_connection().await;
    assert!(
        result.is_ok(),
        "Connection test should succeed: {:?}",
        result
    );
}

#[tokio::test]
#[ignore] // Requires running server with auth
async fn test_client_with_authentication() {
    let client = RpcClientBuilder::new()
        .url("http://127.0.0.1:8080")
        .auth_token("test-token-123".to_string())
        .build()
        .unwrap();

    assert!(client.is_authenticated());

    // This will fail if auth is not configured on server
    let _health = client.health().await;
}

#[tokio::test]
async fn test_client_connection_failure() {
    // Try to connect to non-existent server
    let client = RpcClient::with_url("http://127.0.0.1:19999").unwrap();

    let result = client.health().await;
    assert!(
        result.is_err(),
        "Should fail to connect to non-existent server"
    );
}

#[tokio::test]
async fn test_client_builder() {
    let client = RpcClientBuilder::new()
        .url("http://localhost:8080")
        .timeout(60)
        .max_retries(5)
        .retry_delay(200)
        .pool_size(20)
        .build();

    assert!(client.is_ok());
    let client = client.unwrap();
    assert_eq!(client.url(), "http://localhost:8080");
    assert!(!client.is_authenticated());
}

#[tokio::test]
async fn test_client_config() {
    let config = RpcClientConfig::new("http://localhost:9000")
        .with_auth("token".to_string())
        .with_timeout(45)
        .with_retries(2);

    let client = RpcClient::new(config).unwrap();
    assert!(client.is_authenticated());
}
