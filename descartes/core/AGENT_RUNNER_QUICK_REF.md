# Agent Runner Quick Reference

## Import

```rust
use descartes_core::{
    AgentConfig, AgentRunner, AgentSignal, AgentStatus,
    LocalProcessRunner, ProcessRunnerConfig, GracefulShutdown,
};
use std::collections::HashMap;
```

## Create Runner

```rust
// Default configuration
let runner = LocalProcessRunner::new();

// Custom configuration
let config = ProcessRunnerConfig {
    working_dir: Some(PathBuf::from("/tmp")),
    enable_json_streaming: true,
    enable_health_checks: true,
    health_check_interval_secs: 30,
    max_concurrent_agents: Some(10),
};
let runner = LocalProcessRunner::with_config(config);
```

## Spawn Agent

```rust
let config = AgentConfig {
    name: "my-agent".to_string(),
    model_backend: "claude".to_string(),  // or "opencode", etc.
    task: "Write a poem about Rust".to_string(),
    context: Some("Use modern programming themes".to_string()),
    system_prompt: Some("You are a creative poet".to_string()),
    environment: HashMap::new(),
};

let mut handle = runner.spawn(config).await?;
```

## Control Agent

```rust
// Get agent ID
let id = handle.id();

// Check status
match handle.status() {
    AgentStatus::Running => println!("Running"),
    AgentStatus::Completed => println!("Done"),
    _ => {}
}

// Wait for completion
let exit_status = handle.wait().await?;

// Kill immediately
handle.kill().await?;
```

## Stdio Communication

```rust
// Write to stdin
handle.write_stdin(b"Hello, agent!\n").await?;

// Read from stdout (non-blocking)
while let Ok(Some(output)) = handle.read_stdout().await {
    println!("Agent: {}", String::from_utf8_lossy(&output));
}

// Read from stderr
if let Ok(Some(error)) = handle.read_stderr().await {
    eprintln!("Error: {}", String::from_utf8_lossy(&error));
}
```

## Send Signals

```rust
// Interrupt (SIGINT)
runner.signal(&agent_id, AgentSignal::Interrupt).await?;

// Terminate (SIGTERM)
runner.signal(&agent_id, AgentSignal::Terminate).await?;

// Kill (SIGKILL)
runner.signal(&agent_id, AgentSignal::Kill).await?;
```

## Manage Multiple Agents

```rust
// List all agents
let agents = runner.list_agents().await?;
for agent in agents {
    println!("{}: {:?}", agent.name, agent.status);
}

// Get specific agent
if let Some(info) = runner.get_agent(&agent_id).await? {
    println!("Found: {}", info.name);
}

// Kill all agents
for agent in runner.list_agents().await? {
    runner.kill(&agent.id).await?;
}
```

## Graceful Shutdown

```rust
let shutdown = GracefulShutdown::new(5); // 5 sec timeout
shutdown.shutdown(&mut handle).await?;
```

## Error Handling

```rust
use descartes_core::AgentError;

match runner.spawn(config).await {
    Ok(handle) => { /* success */ }
    Err(AgentError::SpawnFailed(msg)) => {
        eprintln!("Spawn failed: {}", msg);
    }
    Err(AgentError::NotFound(msg)) => {
        eprintln!("Not found: {}", msg);
    }
    Err(e) => {
        eprintln!("Error: {}", e);
    }
}
```

## Common Patterns

### Spawn and Wait

```rust
let mut handle = runner.spawn(config).await?;
let exit_status = handle.wait().await?;
println!("Exit code: {:?}", exit_status.code);
```

### Interactive Session

```rust
let mut handle = runner.spawn(config).await?;

// Send input
handle.write_stdin(b"task 1\n").await?;

// Process output
tokio::time::sleep(Duration::from_millis(100)).await;
while let Ok(Some(output)) = handle.read_stdout().await {
    process_output(&output);
}

handle.kill().await?;
```

### Batch Processing

```rust
for i in 1..=10 {
    let config = AgentConfig {
        name: format!("agent-{}", i),
        model_backend: "claude".to_string(),
        task: format!("Process item {}", i),
        /* ... */
    };

    let handle = runner.spawn(config).await?;
    println!("Spawned: {}", handle.id());
}
```

## Supported Backends

| Backend | Command | Example |
|---------|---------|---------|
| `claude` | `claude` | Claude Code CLI |
| `opencode` | `opencode --headless` | OpenCode CLI |
| `custom-cli` | `custom` | Generic CLI tool |

## Configuration Defaults

| Setting | Default | Description |
|---------|---------|-------------|
| `working_dir` | None | Inherit from parent |
| `enable_json_streaming` | true | Parse JSON output |
| `enable_health_checks` | true | Monitor process health |
| `health_check_interval_secs` | 30 | Check every 30s |
| `max_concurrent_agents` | None | No limit |

## Files

| File | Location |
|------|----------|
| Implementation | `/descartes/core/src/agent_runner.rs` |
| Tests | `/descartes/core/tests/agent_runner_tests.rs` |
| Examples | `/descartes/core/examples/agent_runner_example.rs` |
| Docs | `/descartes/core/AGENT_RUNNER.md` |
| Demo CLI | `/descartes/core/src/bin/agent_runner_demo.rs` |

## Quick Commands

```bash
# Run tests
cargo test --lib agent_runner

# Run integration tests
cargo test --test agent_runner_tests

# Run example
cargo run --example agent_runner_example

# Run demo CLI
cargo run --bin agent_runner_demo help
```

## Tips

1. **Always check status** before operations
2. **Use graceful shutdown** instead of direct kill
3. **Read stdout/stderr regularly** to prevent buffer overflow
4. **Set max_concurrent_agents** in production
5. **Enable health checks** for long-running agents
6. **Handle errors** appropriately (retry, fallback)
7. **Clean up agents** on application exit

## Troubleshooting

| Problem | Solution |
|---------|----------|
| Agent won't spawn | Check CLI tool is installed (`which claude`) |
| No output | Wait longer, check if agent is running |
| Zombie process | Use `GracefulShutdown` |
| Memory leak | Read stdout/stderr regularly |
| Permission denied | Check file permissions and working directory |

## More Info

- Full documentation: `AGENT_RUNNER.md`
- Implementation summary: `AGENT_RUNNER_SUMMARY.md`
- Run examples: `cargo run --example agent_runner_example`
