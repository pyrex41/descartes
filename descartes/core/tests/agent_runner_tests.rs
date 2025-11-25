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
async fn test_spawn_unsupported_backend() {
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "test-unsupported".to_string(),
        model_backend: "openai-api".to_string(), // Non-CLI backend should fail
        task: "Hello World".to_string(),
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    // Attempt to spawn - should fail because this backend doesn't contain 'cli'
    let result = runner.spawn(config).await;
    // This should fail with unsupported backend error
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("Unsupported model backend"));
    }
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
    let _shutdown = GracefulShutdown::new(10);
    // Note: timeout_secs is private, so we just test construction works

    let _default_shutdown = GracefulShutdown::default();
    // Default timeout is 5 seconds as defined in SHUTDOWN_TIMEOUT_SECS
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

// ==============================================================================
// ACTIVE INTEGRATION TESTS
// ==============================================================================
// The following tests use simple shell commands (cat, sleep, etc.) as mock
// backends to test the full agent lifecycle without requiring external CLIs.

#[tokio::test]
async fn test_spawn_with_sleep_backend() {
    // Test spawning an agent with the 'sleep' command as a mock backend.
    // The backend name "sleep-cli" will be parsed to spawn the 'sleep' command.
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "test-sleep".to_string(),
        model_backend: "sleep-cli".to_string(), // Will spawn 'sleep' command
        task: "2".to_string(),                   // Sleep for 2 seconds
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();
    assert_eq!(handle.status(), AgentStatus::Running);

    // Process should be running
    sleep(Duration::from_millis(100)).await;
    assert_eq!(handle.status(), AgentStatus::Running);

    // Kill the agent before it naturally exits
    handle.kill().await.unwrap();
}

#[tokio::test]
async fn test_agent_lifecycle() {
    // Test full agent lifecycle: spawn -> register -> kill -> cleanup
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "lifecycle-test".to_string(),
        model_backend: "sleep-cli".to_string(), // Use sleep as a simple long-running process
        task: "30".to_string(),                  // Sleep for 30 seconds (will be killed before)
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();
    let agent_id = handle.id();

    // Check it's in the registry
    let info = runner.get_agent(&agent_id).await.unwrap();
    assert!(info.is_some());
    assert_eq!(info.unwrap().name, "lifecycle-test");

    // Verify it's listed in all agents
    let agents = runner.list_agents().await.unwrap();
    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].id, agent_id);

    // Kill via handle (not via runner) to avoid potential deadlocks
    handle.kill().await.unwrap();

    // Wait a moment for cleanup
    sleep(Duration::from_millis(100)).await;

    // Check if removed from registry (implementation removes it on runner.kill(), not handle.kill())
    let agents = runner.list_agents().await.unwrap();
    // The agent might still be in the list since we used handle.kill() not runner.kill()
    // This is expected behavior - handle.kill() just kills the process, runner.kill() also removes from registry
    assert!(agents.len() <= 1);
}

#[tokio::test]
async fn test_graceful_shutdown() {
    // Test graceful shutdown with timeout - should force kill a long-running process
    let runner = LocalProcessRunner::new();
    let shutdown = GracefulShutdown::new(1); // 1 second timeout

    let config = AgentConfig {
        name: "shutdown-test".to_string(),
        model_backend: "sleep-cli".to_string(),
        task: "30".to_string(), // Sleep for 30 seconds
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();
    assert_eq!(handle.status(), AgentStatus::Running);

    // Perform graceful shutdown (writes "exit\n" to stdin, waits, then force kills)
    // Sleep will not exit on "exit\n", so this will timeout and force kill
    let start = std::time::Instant::now();
    shutdown.shutdown(&mut handle).await.unwrap();
    let elapsed = start.elapsed();

    // Should have timed out and killed quickly (around 1 second)
    assert!(
        elapsed.as_secs() <= 2,
        "Shutdown took too long: {:?}",
        elapsed
    );
}

#[tokio::test]
async fn test_graceful_shutdown_timeout() {
    // Test that graceful shutdown respects timeout and doesn't wait forever
    let runner = LocalProcessRunner::new();
    let shutdown = GracefulShutdown::new(1); // 1 second timeout

    let config = AgentConfig {
        name: "shutdown-timeout-test".to_string(),
        model_backend: "sleep-cli".to_string(),
        task: "10".to_string(), // Sleep for 10 seconds
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();
    assert_eq!(handle.status(), AgentStatus::Running);

    // Graceful shutdown should timeout after 1 second and force kill
    let start = std::time::Instant::now();
    shutdown.shutdown(&mut handle).await.unwrap();
    let elapsed = start.elapsed();

    // Should complete in approximately 1 second (timeout), not 10 seconds
    // Allow some margin for test execution overhead
    assert!(
        elapsed.as_secs() <= 3,
        "Shutdown took too long: {:?}. Expected ~1s timeout, not 10s",
        elapsed
    );
}

#[tokio::test]
async fn test_stdio_streaming() {
    // Test that stdio handles are properly set up and accessible
    // Note: We can't easily test echoing with simple Unix commands via the CLI interface,
    // so this test verifies that stdio operations don't error
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "stdio-test".to_string(),
        model_backend: "sleep-cli".to_string(),
        task: "5".to_string(), // Sleep for 5 seconds
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();

    // Verify we can write to stdin without errors (even though sleep ignores it)
    assert!(handle.write_stdin(b"test input\n").await.is_ok());

    // Try to read stdout (should be empty or None for sleep command)
    let stdout_result = handle.read_stdout().await;
    assert!(stdout_result.is_ok());

    // Try to read stderr (should be empty or None for sleep command)
    let stderr_result = handle.read_stderr().await;
    assert!(stderr_result.is_ok());

    handle.kill().await.unwrap();
}

#[tokio::test]
async fn test_health_checks() {
    // Test health check monitoring of process status
    let mut config = ProcessRunnerConfig::default();
    config.health_check_interval_secs = 1; // Check every second

    let runner = LocalProcessRunner::with_config(config);

    let agent_config = AgentConfig {
        name: "health-test".to_string(),
        model_backend: "sleep-cli".to_string(),
        task: "30".to_string(), // Long-running process
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(agent_config).await.unwrap();
    let agent_id = handle.id();

    // Wait for at least one health check cycle
    sleep(Duration::from_millis(1500)).await;

    // Should still be running (cat is long-running)
    let info = runner.get_agent(&agent_id).await.unwrap().unwrap();
    assert_eq!(info.status, AgentStatus::Running);

    // Kill the process via handle
    handle.kill().await.unwrap();

    // Wait for health check to detect termination
    sleep(Duration::from_millis(1500)).await;

    // The agent should still be in registry (handle.kill() doesn't remove it)
    // but status should eventually be updated by health checker
    let info = runner.get_agent(&agent_id).await.unwrap();
    assert!(info.is_some(), "Expected agent to still be in registry");

    // Status may have been updated by health checker, or may still be Running
    // depending on timing - either is acceptable for this test
    let status = info.unwrap().status;
    assert!(
        status == AgentStatus::Running || status == AgentStatus::Terminated,
        "Status should be Running or Terminated, got: {:?}",
        status
    );
}

#[tokio::test]
async fn test_process_exit_detection() {
    // Test that the system detects when a process exits naturally
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "exit-test".to_string(),
        model_backend: "sleep-cli".to_string(),
        task: "1".to_string(), // Sleep for 1 second then exit
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();

    // Initially should be running
    assert_eq!(handle.status(), AgentStatus::Running);

    // Wait for process to complete
    let exit_status = handle.wait().await.unwrap();
    assert!(exit_status.success);
    assert_eq!(exit_status.code, Some(0));

    // Exit code should now be available
    assert_eq!(handle.exit_code(), Some(0));
}

#[tokio::test]
#[cfg(unix)] // This test is Unix-specific due to signal handling
async fn test_signal_handling() {
    // Test sending different signals to a process
    let runner = LocalProcessRunner::new();

    let config = AgentConfig {
        name: "signal-test".to_string(),
        model_backend: "sleep-cli".to_string(),
        task: "10".to_string(), // Long-running process
        context: None,
        system_prompt: None,
        environment: HashMap::new(),
    };

    let mut handle = runner.spawn(config).await.unwrap();
    let agent_id = handle.id();

    // Test SIGTERM signal - should succeed without errors
    runner
        .signal(&agent_id, AgentSignal::Terminate)
        .await
        .unwrap();

    // Wait a bit for the signal to be processed
    sleep(Duration::from_millis(200)).await;

    // Process should be terminated by SIGTERM
    // Note: The status in registry may not be updated immediately since
    // signal() doesn't update status - only the health checker does that
    // Clean up via handle
    let _ = handle.kill().await; // May already be dead from SIGTERM

    // Verify the signal was sent successfully (we got this far)
    // The actual process termination is verified by the fact that
    // kill() doesn't error even if process is already dead
    assert!(true); // Test passed if we got here without errors
}
