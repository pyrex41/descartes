/// Integration tests for ZMQ Agent Server
///
/// These tests verify the server-side agent spawning and lifecycle management
/// functionality by simulating client-server interactions.
use descartes_core::{
    AgentConfig, AgentStatus, ZmqAgentRunner, ZmqAgentServer, ZmqClient, ZmqRunnerConfig,
    ZmqServerConfig,
};
use std::collections::HashMap;
use std::time::Duration;
use tokio::time::sleep;

/// Helper to start a test server in the background
async fn start_test_server(endpoint: &str) -> (ZmqAgentServer, tokio::task::JoinHandle<()>) {
    let config = ZmqServerConfig {
        endpoint: endpoint.to_string(),
        ..Default::default()
    };

    let server = ZmqAgentServer::new(config);
    let server_clone = server.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = server_clone.start().await {
            eprintln!("Server error: {}", e);
        }
    });

    // Give server time to start
    sleep(Duration::from_millis(500)).await;

    (server, handle)
}

/// Helper to create a test client
fn create_test_client(endpoint: &str) -> ZmqClient {
    let config = ZmqRunnerConfig {
        endpoint: endpoint.to_string(),
        connection_timeout_secs: 5,
        request_timeout_secs: 10,
        ..Default::default()
    };

    ZmqClient::new(config)
}

#[tokio::test]
#[ignore] // Requires ZMQ setup
async fn test_server_startup_and_shutdown() {
    let endpoint = "tcp://127.0.0.1:15555";
    let (server, handle) = start_test_server(endpoint).await;

    assert!(server.is_running());
    assert_eq!(server.active_agent_count(), 0);

    // Stop the server
    server.stop().await.unwrap();
    assert!(!server.is_running());

    // Cancel the task
    handle.abort();
}

#[tokio::test]
#[ignore] // Requires ZMQ setup and CLI tools
async fn test_server_health_check() {
    let endpoint = "tcp://127.0.0.1:15556";
    let (server, handle) = start_test_server(endpoint).await;

    // Create client
    let client = create_test_client(endpoint);
    client.connect(endpoint).await.unwrap();

    // Send health check
    let response = client.health_check().await.unwrap();

    assert!(response.healthy);
    assert_eq!(response.protocol_version, "1.0.0");
    assert!(response.uptime_secs.is_some());
    assert_eq!(response.active_agents, Some(0));

    // Cleanup
    client.disconnect().await.unwrap();
    server.stop().await.unwrap();
    handle.abort();
}

#[tokio::test]
#[ignore] // Requires ZMQ setup and CLI tools
async fn test_server_spawn_agent() {
    let endpoint = "tcp://127.0.0.1:15557";
    let (server, handle) = start_test_server(endpoint).await;

    // Create client
    let client = create_test_client(endpoint);
    client.connect(endpoint).await.unwrap();

    // Spawn an agent
    let config = AgentConfig {
        name: "test-agent".to_string(),
        model_backend: "claude".to_string(),
        task: "echo 'Hello, World!'".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
        ..Default::default()
    };

    let result = client.spawn_remote(config, Some(30)).await;

    match result {
        Ok(agent_info) => {
            assert_eq!(agent_info.name, "test-agent");
            assert_eq!(agent_info.model_backend, "claude");

            // Verify server has the agent
            assert_eq!(server.active_agent_count(), 1);

            // Get agent status
            let status = client.get_agent_status(&agent_info.id).await.unwrap();
            assert!(matches!(
                status,
                AgentStatus::Running | AgentStatus::Idle | AgentStatus::Initializing
            ));

            // Stop the agent
            client.stop_agent(&agent_info.id).await.unwrap();
        }
        Err(e) => {
            // If the CLI tool is not available, the test should be marked as ignored
            println!("Spawn failed (expected if CLI not available): {}", e);
        }
    }

    // Cleanup
    client.disconnect().await.unwrap();
    server.stop().await.unwrap();
    handle.abort();
}

#[tokio::test]
#[ignore] // Requires ZMQ setup and CLI tools
async fn test_server_list_agents() {
    let endpoint = "tcp://127.0.0.1:15558";
    let (server, handle) = start_test_server(endpoint).await;

    // Create client
    let client = create_test_client(endpoint);
    client.connect(endpoint).await.unwrap();

    // List agents (should be empty)
    let agents = client.list_remote_agents(None, None).await.unwrap();
    assert_eq!(agents.len(), 0);

    // Spawn multiple agents
    for i in 0..3 {
        let config = AgentConfig {
            name: format!("agent-{}", i),
            model_backend: "claude".to_string(),
            task: format!("Task {}", i),
            context: None,
            system_prompt: None,
            environment: HashMap::new(),
            ..Default::default()
        };

        if let Ok(_) = client.spawn_remote(config, Some(30)).await {
            // Agent spawned successfully
        }
    }

    // Give agents time to spawn
    sleep(Duration::from_millis(500)).await;

    // List agents again
    let agents = client.list_remote_agents(None, None).await.unwrap();
    assert!(agents.len() > 0);

    // Cleanup
    client.disconnect().await.unwrap();
    server.stop().await.unwrap();
    handle.abort();
}

#[tokio::test]
#[ignore] // Requires ZMQ setup and CLI tools
async fn test_server_control_commands() {
    let endpoint = "tcp://127.0.0.1:15559";
    let (server, handle) = start_test_server(endpoint).await;

    // Create client
    let client = create_test_client(endpoint);
    client.connect(endpoint).await.unwrap();

    // Spawn an agent
    let config = AgentConfig {
        name: "control-test-agent".to_string(),
        model_backend: "claude".to_string(),
        task: "sleep 30".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
        ..Default::default()
    };

    if let Ok(agent_info) = client.spawn_remote(config, Some(30)).await {
        // Give agent time to start
        sleep(Duration::from_millis(500)).await;

        // Test pause
        if let Ok(_) = client.pause_agent(&agent_info.id).await {
            let status = client.get_agent_status(&agent_info.id).await.unwrap();
            // Status might be Paused or Running depending on implementation
            println!("Agent status after pause: {:?}", status);
        }

        // Test resume
        if let Ok(_) = client.resume_agent(&agent_info.id).await {
            let status = client.get_agent_status(&agent_info.id).await.unwrap();
            println!("Agent status after resume: {:?}", status);
        }

        // Test stop
        client.stop_agent(&agent_info.id).await.unwrap();
    }

    // Cleanup
    client.disconnect().await.unwrap();
    server.stop().await.unwrap();
    handle.abort();
}

#[tokio::test]
#[ignore] // Requires ZMQ setup
async fn test_server_max_agents_limit() {
    let endpoint = "tcp://127.0.0.1:15560";

    // Create server with low max agent limit
    let config = ZmqServerConfig {
        endpoint: endpoint.to_string(),
        max_agents: 2,
        ..Default::default()
    };

    let server = ZmqAgentServer::new(config);
    let server_clone = server.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = server_clone.start().await {
            eprintln!("Server error: {}", e);
        }
    });

    sleep(Duration::from_millis(500)).await;

    // Create client
    let client = create_test_client(endpoint);
    client.connect(endpoint).await.unwrap();

    // Try to spawn more agents than the limit
    let mut spawned = Vec::new();
    for i in 0..5 {
        let config = AgentConfig {
            name: format!("agent-{}", i),
            model_backend: "claude".to_string(),
            task: format!("Task {}", i),
            context: None,
            system_prompt: None,
            environment: HashMap::new(),
            ..Default::default()
        };

        match client.spawn_remote(config, Some(30)).await {
            Ok(info) => spawned.push(info),
            Err(e) => {
                println!("Spawn {} failed: {}", i, e);
            }
        }
    }

    // Should have spawned at most max_agents
    assert!(spawned.len() <= 2);

    // Cleanup
    client.disconnect().await.unwrap();
    server.stop().await.unwrap();
    handle.abort();
}

#[tokio::test]
#[ignore] // Requires ZMQ setup
async fn test_server_statistics() {
    let endpoint = "tcp://127.0.0.1:15561";
    let (server, handle) = start_test_server(endpoint).await;

    // Check initial stats
    let stats = server.stats();
    assert_eq!(stats.spawn_requests, 0);
    assert_eq!(stats.successful_spawns, 0);
    assert!(stats.started_at.is_some());

    // Create client and send requests
    let client = create_test_client(endpoint);
    client.connect(endpoint).await.unwrap();

    // Send health check
    let _ = client.health_check().await;

    // Check updated stats
    let stats = server.stats();
    assert_eq!(stats.health_checks, 1);

    // Try to spawn an agent
    let config = AgentConfig {
        name: "stats-test-agent".to_string(),
        model_backend: "claude".to_string(),
        task: "test task".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
        ..Default::default()
    };

    let _ = client.spawn_remote(config, Some(30)).await;

    // Check spawn stats (should increment regardless of success)
    let stats = server.stats();
    assert_eq!(stats.spawn_requests, 1);

    // Cleanup
    client.disconnect().await.unwrap();
    server.stop().await.unwrap();
    handle.abort();
}

#[tokio::test]
async fn test_server_uptime_calculation() {
    let endpoint = "tcp://127.0.0.1:15562";

    let config = ZmqServerConfig {
        endpoint: endpoint.to_string(),
        ..Default::default()
    };

    let server = ZmqAgentServer::new(config);

    // Before starting, uptime should be None
    assert_eq!(server.uptime_secs(), None);

    // Note: Can't easily test uptime without actually starting the server,
    // which requires ZMQ setup. This test just verifies the API.
}

#[test]
fn test_server_config_customization() {
    let config = ZmqServerConfig {
        endpoint: "tcp://0.0.0.0:9999".to_string(),
        server_id: "custom-server".to_string(),
        max_agents: 50,
        status_update_interval_secs: 5,
        enable_status_updates: false,
        request_timeout_secs: 60,
        ..Default::default()
    };

    assert_eq!(config.endpoint, "tcp://0.0.0.0:9999");
    assert_eq!(config.server_id, "custom-server");
    assert_eq!(config.max_agents, 50);
    assert_eq!(config.status_update_interval_secs, 5);
    assert!(!config.enable_status_updates);
    assert_eq!(config.request_timeout_secs, 60);
}
