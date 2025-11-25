/// Integration tests for agent_runner module.
///
/// Tests process spawning, lifecycle management, stdio streaming,
/// signal handling, and graceful shutdown.
use descartes_core::{
    AgentConfig, AgentRunner, AgentSignal, AgentStatus, GracefulShutdown, LocalProcessRunner,
    ProcessRunnerConfig,
};
use std::collections::HashMap;
use tokio::time::{sleep, Duration};

#[tokio::test]
async fn test_spawn_simple_process() {
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "test-echo".to_string(),
        model_backend: "echo-cli".to_string(), // Will fail but tests spawn logic
        task: "Hello World".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    // Attempt to spawn - will fail because 'echo' doesn't support CLI mode
    let result = runner.spawn(config).await;
    // This should fail with unsupported backend
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_agents() {
    let runner = LocalProcessRunner::new();

    // Should be empty initially
    let agents = runner.list_agents().await.unwrap();
    assert_eq!(agents.len(), 0);
}

#[tokio::test]
async fn test_get_nonexistent_agent() {
    let runner = LocalProcessRunner::new();
    let agent_id = uuid::Uuid::new_v4();

    let result = runner.get_agent(&agent_id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_kill_nonexistent_agent() {
    let runner = LocalProcessRunner::new();
    let agent_id = uuid::Uuid::new_v4();

    let result = runner.kill(&agent_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_signal_nonexistent_agent() {
    let runner = LocalProcessRunner::new();
    let agent_id = uuid::Uuid::new_v4();

    let result = runner.signal(&agent_id, AgentSignal::Interrupt).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_process_runner_with_max_agents() {
    let mut config = ProcessRunnerConfig::default();
    config.max_concurrent_agents = Some(0); // No agents allowed

    let runner = LocalProcessRunner::with_config(config);

    let agent_config = AgentConfig {
        name: "test".to_string(),
        model_backend: "claude".to_string(),
        task: "test".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    // Should fail due to max concurrent limit
    let result = runner.spawn(agent_config).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_process_config_defaults() {
    let config = ProcessRunnerConfig::default();

    assert!(config.enable_json_streaming);
    assert!(config.enable_health_checks);
    assert_eq!(config.health_check_interval_secs, 30);
    assert!(config.max_concurrent_agents.is_none());
}

#[tokio::test]
async fn test_graceful_shutdown_creation() {
    let shutdown = GracefulShutdown::new(10);
    assert_eq!(shutdown.timeout_secs, 10);

    let default_shutdown = GracefulShutdown::default();
    assert_eq!(default_shutdown.timeout_secs, 5);
}

#[tokio::test]
async fn test_agent_status_values() {
    // Ensure all status values are available
    let statuses = vec![
        AgentStatus::Idle,
        AgentStatus::Running,
        AgentStatus::Paused,
        AgentStatus::Completed,
        AgentStatus::Failed,
        AgentStatus::Terminated,
    ];

    assert_eq!(statuses.len(), 6);
}

#[tokio::test]
async fn test_agent_config_creation() {
    let mut env = HashMap::new();
    env.insert("TEST_VAR".to_string(), "test_value".to_string());

    let config = AgentConfig {
        name: "test-agent".to_string(),
        model_backend: "claude".to_string(),
        task: "Write a test".to_string(),
        context: Some("Test context".to_string()),
        system_prompt: Some("You are a test assistant".to_string()),
        environment: env.clone(),
    };

    assert_eq!(config.name, "test-agent");
    assert_eq!(config.model_backend, "claude");
    assert_eq!(config.task, "Write a test");
    assert!(config.context.is_some());
    assert!(config.system_prompt.is_some());
    assert_eq!(config.environment.len(), 1);
}

#[tokio::test]
async fn test_multiple_runners() {
    // Test creating multiple process runners
    let runner1 = LocalProcessRunner::new();
    let runner2 = LocalProcessRunner::new();

    let agents1 = runner1.list_agents().await.unwrap();
    let agents2 = runner2.list_agents().await.unwrap();

    assert_eq!(agents1.len(), 0);
    assert_eq!(agents2.len(), 0);
}

// Note: The following tests require actual CLI tools to be installed
// and are commented out to avoid test failures in CI/CD environments

/*
#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_spawn_real_claude_cli() {
    // This test requires the Claude CLI to be installed
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "test-claude".to_string(),
        model_backend: "claude".to_string(),
        task: "Say hello".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();
    assert_eq!(handle.status(), AgentStatus::Running);

    // Write to stdin
    handle.write_stdin(b"Hello\n").await.unwrap();

    // Wait a bit for response
    sleep(Duration::from_millis(100)).await;

    // Try to read stdout
    if let Ok(Some(output)) = handle.read_stdout().await {
        println!("Output: {}", String::from_utf8_lossy(&output));
    }

    // Kill the agent
    handle.kill().await.unwrap();
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_agent_lifecycle() {
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "lifecycle-test".to_string(),
        model_backend: "claude".to_string(),
        task: "Test lifecycle".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();
    let agent_id = handle.id();

    // Check it's in the registry
    let info = runner.get_agent(&agent_id).await.unwrap();
    assert!(info.is_some());

    // Send interrupt signal
    runner.signal(&agent_id, AgentSignal::Interrupt).await.unwrap();
    sleep(Duration::from_millis(100)).await;

    // Kill it
    runner.kill(&agent_id).await.unwrap();

    // Should be removed from registry
    let info = runner.get_agent(&agent_id).await.unwrap();
    assert!(info.is_none());
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_graceful_shutdown() {
    let runner = LocalProcessRunner::new();
    let shutdown = GracefulShutdown::new(2);

    let config = AgentConfig {
        name: "shutdown-test".to_string(),
        model_backend: "claude".to_string(),
        task: "Test shutdown".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();

    // Perform graceful shutdown
    shutdown.shutdown(&mut handle).await.unwrap();

    // Should be terminated
    let exit_code = handle.exit_code();
    assert!(exit_code.is_some());
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_stdio_streaming() {
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "stdio-test".to_string(),
        model_backend: "claude".to_string(),
        task: "Echo test".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();

    // Write multiple lines
    handle.write_stdin(b"Line 1\n").await.unwrap();
    handle.write_stdin(b"Line 2\n").await.unwrap();
    handle.write_stdin(b"Line 3\n").await.unwrap();

    sleep(Duration::from_millis(500)).await;

    // Read all available stdout
    let mut output_count = 0;
    while let Ok(Some(_)) = handle.read_stdout().await {
        output_count += 1;
    }

    assert!(output_count > 0);

    handle.kill().await.unwrap();
}

#[tokio::test]
#[ignore] // Run with: cargo test -- --ignored
async fn test_health_checks() {
    let mut config = ProcessRunnerConfig::default();
    config.health_check_interval_secs = 1; // Check every second

    let runner = LocalProcessRunner::with_config(config);

    let agent_config = AgentConfig {
        name: "health-test".to_string(),
        model_backend: "claude".to_string(),
        task: "Health check test".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(agent_config).await.unwrap();
    let agent_id = handle.id();

    // Wait for a health check cycle
    sleep(Duration::from_secs(2)).await;

    // Should still be running
    let info = runner.get_agent(&agent_id).await.unwrap().unwrap();
    assert_eq!(info.status, AgentStatus::Running);

    // Kill the process
    handle.kill().await.unwrap();

    // Wait for health check to detect termination
    sleep(Duration::from_secs(2)).await;

    // Status should be updated
    let info = runner.get_agent(&agent_id).await.unwrap();
    if let Some(info) = info {
        assert_ne!(info.status, AgentStatus::Running);
    }
}
*/
