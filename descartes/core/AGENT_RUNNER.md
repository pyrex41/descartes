# Agent Runner Implementation

## Overview

The Agent Runner is a production-ready system for spawning and managing AI agent processes in the Descartes orchestration framework. It provides comprehensive process lifecycle management, stdio streaming, signal handling, health monitoring, and graceful shutdown capabilities.

## Architecture

### Components

```
┌─────────────────────────────────────────────────────────────┐
│                    LocalProcessRunner                        │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Agent Registry (DashMap)                           │   │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐          │   │
│  │  │ Agent 1  │  │ Agent 2  │  │ Agent N  │          │   │
│  │  └──────────┘  └──────────┘  └──────────┘          │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  Configuration:                                              │
│  - Working directory                                         │
│  - JSON streaming mode                                       │
│  - Health check interval                                     │
│  - Max concurrent agents                                     │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    LocalAgentHandle                          │
│  ┌─────────────────────────────────────────────────────┐   │
│  │  Child Process (tokio::process::Child)              │   │
│  │  - stdin  (AsyncWrite)                              │   │
│  │  - stdout (AsyncRead → BufReader → Channel)         │   │
│  │  - stderr (AsyncRead → BufReader → Channel)         │   │
│  └─────────────────────────────────────────────────────┘   │
│                                                              │
│  Features:                                                   │
│  - Async stdio streaming                                     │
│  - Background reader tasks                                   │
│  - Signal handling (SIGINT/SIGTERM/SIGKILL)                 │
│  - Status tracking                                           │
│  - Exit code capture                                         │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│                    GracefulShutdown                          │
│  1. Send shutdown signal (SIGTERM)                          │
│  2. Wait for timeout                                         │
│  3. Force kill if needed (SIGKILL)                          │
└─────────────────────────────────────────────────────────────┘
```

## Features

### 1. Process Spawning

Spawn agent processes with full configuration:

```rust
use descartes_core::{AgentConfig, AgentRunner, LocalProcessRunner};
use std::collections::HashMap;

let runner = LocalProcessRunner::new();

let config = AgentConfig {
    name: "research-agent".to_string(),
    model_backend: "claude".to_string(),
    task: "Research AI developments".to_string(),
    context: Some("Focus on LLMs".to_string()),
    system_prompt: Some("You are a researcher".to_string()),
    environment: HashMap::new(),
};

let mut handle = runner.spawn(config).await?;
```

### 2. Lifecycle Management

Full control over agent lifecycle:

```rust
// Check status
match handle.status() {
    AgentStatus::Running => println!("Agent is running"),
    AgentStatus::Completed => println!("Agent completed"),
    AgentStatus::Failed => println!("Agent failed"),
    _ => {}
}

// Wait for completion
let exit_status = handle.wait().await?;
println!("Exit code: {:?}", exit_status.code);

// Kill immediately
handle.kill().await?;
```

### 3. Stdio Streaming

Bidirectional communication with agents:

```rust
// Write to stdin
handle.write_stdin(b"Hello, agent!\n").await?;

// Read from stdout (non-blocking)
if let Some(output) = handle.read_stdout().await? {
    println!("Agent says: {}", String::from_utf8_lossy(&output));
}

// Read from stderr
if let Some(error) = handle.read_stderr().await? {
    eprintln!("Agent error: {}", String::from_utf8_lossy(&error));
}
```

### 4. Signal Handling

Send signals to agents (Unix-compatible):

```rust
use descartes_core::AgentSignal;

// Send SIGINT (Ctrl+C)
runner.signal(&agent_id, AgentSignal::Interrupt).await?;

// Send SIGTERM (graceful shutdown)
runner.signal(&agent_id, AgentSignal::Terminate).await?;

// Send SIGKILL (force kill)
runner.signal(&agent_id, AgentSignal::Kill).await?;
```

### 5. Health Monitoring

Automatic health checks with configurable intervals:

```rust
use descartes_core::ProcessRunnerConfig;

let config = ProcessRunnerConfig {
    enable_health_checks: true,
    health_check_interval_secs: 30, // Check every 30 seconds
    ..Default::default()
};

let runner = LocalProcessRunner::with_config(config);
// Health checks run automatically in background
```

### 6. Graceful Shutdown

Coordinated shutdown with timeout and force kill:

```rust
use descartes_core::GracefulShutdown;

let shutdown = GracefulShutdown::new(5); // 5 second timeout

// Tries SIGTERM first, then SIGKILL after timeout
shutdown.shutdown(&mut handle).await?;
```

### 7. Multi-Agent Management

Manage multiple agents concurrently:

```rust
// Spawn multiple agents
for i in 1..=10 {
    let config = AgentConfig { /* ... */ };
    let handle = runner.spawn(config).await?;
    println!("Spawned agent: {}", handle.id());
}

// List all agents
let agents = runner.list_agents().await?;
for agent in agents {
    println!("{}: {:?}", agent.name, agent.status);
}

// Kill all agents
for agent in runner.list_agents().await? {
    runner.kill(&agent.id).await?;
}
```

## Configuration

### ProcessRunnerConfig

```rust
pub struct ProcessRunnerConfig {
    /// Working directory for spawned processes
    pub working_dir: Option<PathBuf>,

    /// Enable JSON streaming mode for stdout parsing
    pub enable_json_streaming: bool,

    /// Enable automatic health checks
    pub enable_health_checks: bool,

    /// Health check interval in seconds
    pub health_check_interval_secs: u64,

    /// Maximum concurrent agents (None = unlimited)
    pub max_concurrent_agents: Option<usize>,
}
```

**Defaults:**
- `working_dir`: None (inherit from parent)
- `enable_json_streaming`: true
- `enable_health_checks`: true
- `health_check_interval_secs`: 30
- `max_concurrent_agents`: None

### AgentConfig

```rust
pub struct AgentConfig {
    /// Human-readable agent name
    pub name: String,

    /// Model backend identifier (e.g., "claude", "openai", "ollama")
    pub model_backend: String,

    /// Primary task/prompt for the agent
    pub task: String,

    /// Optional context information
    pub context: Option<String>,

    /// Optional system prompt
    pub system_prompt: Option<String>,

    /// Environment variables
    pub environment: HashMap<String, String>,
}
```

## Supported Backends

### Claude Code CLI

```rust
let config = AgentConfig {
    model_backend: "claude".to_string(),
    task: "Your task here".to_string(),
    // ...
};
```

**Command spawned:** `claude "Your task here"`

### OpenCode CLI

```rust
let config = AgentConfig {
    model_backend: "opencode".to_string(),
    task: "Your task here".to_string(),
    // ...
};
```

**Command spawned:** `opencode --headless "Your task here"`

### Generic CLI

```rust
let config = AgentConfig {
    model_backend: "custom-cli".to_string(),
    task: "Your task here".to_string(),
    // ...
};
```

**Command spawned:** `custom "Your task here"`

## Implementation Details

### Async Stdio Streaming

The agent runner uses background tokio tasks to read stdout/stderr asynchronously:

```rust
// Background task reads lines from stdout
tokio::spawn(async move {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();

    loop {
        match reader.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                tx.send(line.as_bytes().to_vec()).ok();
                line.clear();
            }
            Err(_) => break,
        }
    }
});
```

This ensures:
- Non-blocking reads
- No deadlocks from full buffers
- Efficient buffering
- Clean shutdown

### Signal Handling (Unix)

On Unix systems, signals are sent using the `nix` crate:

```rust
#[cfg(unix)]
{
    use nix::sys::signal::{kill, Signal};
    use nix::unistd::Pid;

    if let Some(pid) = child.id() {
        kill(Pid::from_raw(pid as i32), Signal::SIGTERM)?;
    }
}
```

On Windows, signals are mapped to appropriate equivalents.

### Health Checks

Health checks run in background tasks:

```rust
tokio::spawn(async move {
    let mut interval = tokio::time::interval(Duration::from_secs(30));

    loop {
        interval.tick().await;

        // Check if process is alive
        let is_alive = child.try_wait().is_ok();

        if !is_alive {
            // Update status and exit
            break;
        }
    }
});
```

### Thread Safety

- **DashMap**: Lock-free concurrent hashmap for agent registry
- **Arc<RwLock>**: Shared ownership with read/write locking
- **Arc<Mutex>**: Exclusive access to stdio handles
- **mpsc channels**: Message passing for stdout/stderr

## Error Handling

All operations return `AgentResult<T>`:

```rust
pub type AgentResult<T> = Result<T, AgentError>;

pub enum AgentError {
    SpawnFailed(String),
    ExecutionError(String),
    CommunicationError(String),
    NotFound(String),
    Timeout(String),
    InvalidContext(String),
    IoError(std::io::Error),
    ProviderError(ProviderError),
}
```

### Common Error Scenarios

**Agent spawn fails:**
```rust
match runner.spawn(config).await {
    Err(AgentError::SpawnFailed(msg)) => {
        eprintln!("Failed to spawn: {}", msg);
        // Retry or fallback
    }
    Ok(handle) => { /* ... */ }
    _ => {}
}
```

**Agent not found:**
```rust
match runner.get_agent(&id).await {
    Ok(None) => eprintln!("Agent {} not found", id),
    Ok(Some(info)) => println!("Found: {:?}", info),
    Err(e) => eprintln!("Error: {}", e),
}
```

**Communication error:**
```rust
match handle.write_stdin(data).await {
    Err(AgentError::CommunicationError(msg)) => {
        eprintln!("Communication failed: {}", msg);
        // Handle broken pipe, etc.
    }
    Ok(_) => {}
    _ => {}
}
```

## Performance Considerations

### Memory Usage

- **Buffer size**: 16KB per stream (stdout/stderr)
- **Channel capacity**: Unbounded (consider bounded for production)
- **Agent registry**: O(1) lookup via DashMap

### Concurrency

- **Max agents**: Configurable via `max_concurrent_agents`
- **Background tasks**: 2 per agent (stdout + stderr readers)
- **Health checks**: 1 per agent (if enabled)

**Recommended limits:**
- Small deployment: 10-50 agents
- Medium deployment: 50-200 agents
- Large deployment: 200-1000 agents (tune buffer sizes)

### Optimization Tips

1. **Disable health checks** if not needed:
   ```rust
   config.enable_health_checks = false;
   ```

2. **Set max concurrent agents**:
   ```rust
   config.max_concurrent_agents = Some(100);
   ```

3. **Use bounded channels** for production:
   ```rust
   // Modify agent_runner.rs to use bounded channels
   let (tx, rx) = mpsc::channel(1000); // 1000 message buffer
   ```

4. **Batch agent operations**:
   ```rust
   // Spawn agents in batches
   for batch in configs.chunks(10) {
       let handles = batch.iter()
           .map(|c| runner.spawn(c.clone()))
           .collect::<Vec<_>>();
       futures::future::join_all(handles).await;
   }
   ```

## Testing

### Unit Tests

Run unit tests:
```bash
cargo test --lib agent_runner
```

### Integration Tests

Run integration tests:
```bash
cargo test --test agent_runner_tests
```

### Ignored Tests

Tests requiring actual CLI tools are marked `#[ignore]`:
```bash
cargo test -- --ignored
```

### Example

Run the example:
```bash
cargo run --example agent_runner_example
```

## Security Considerations

1. **Environment variables**: Sensitive data (API keys) passed via environment
2. **Working directory**: Configurable to isolate agents
3. **Signal handling**: Proper cleanup to prevent zombie processes
4. **Resource limits**: `max_concurrent_agents` prevents resource exhaustion

**Best practices:**
- Never log sensitive environment variables
- Use working directory isolation
- Set resource limits in production
- Monitor agent resource usage
- Implement agent authentication

## Future Enhancements

### Planned Features

1. **Containerization**: Docker/Podman support
2. **Resource limits**: CPU/memory limits per agent
3. **Structured logging**: JSON logs with tracing
4. **Metrics**: Prometheus metrics for monitoring
5. **Agent pools**: Pre-spawned agent pools for faster startup
6. **Sandboxing**: seccomp/AppArmor profiles
7. **Network isolation**: Network namespaces
8. **Checkpoint/restore**: CRIU support for migration

### API Extensions

```rust
// Future API ideas

// Resource limits
pub struct ResourceLimits {
    pub max_memory_mb: Option<usize>,
    pub max_cpu_percent: Option<u8>,
    pub max_runtime_secs: Option<u64>,
}

// Agent pools
pub struct AgentPool {
    pub min_agents: usize,
    pub max_agents: usize,
    pub warmup_config: AgentConfig,
}

// Metrics
pub struct AgentMetrics {
    pub cpu_usage: f64,
    pub memory_mb: usize,
    pub uptime_secs: u64,
    pub messages_sent: usize,
    pub messages_received: usize,
}
```

## Troubleshooting

### Agent fails to spawn

**Problem:** `AgentError::SpawnFailed`

**Solutions:**
1. Verify CLI tool is installed: `which claude`
2. Check PATH environment variable
3. Test command manually: `claude --version`
4. Review logs for detailed error

### Stdout/stderr not reading

**Problem:** No output from `read_stdout()`

**Solutions:**
1. Check if agent is actually producing output
2. Verify agent is still running: `handle.status()`
3. Wait longer before reading (agent may be processing)
4. Check stderr for errors: `handle.read_stderr()`

### Agent becomes zombie

**Problem:** Agent process remains after kill

**Solutions:**
1. Use `GracefulShutdown` instead of direct kill
2. Increase shutdown timeout
3. Check for signal handler in agent CLI
4. Monitor with `ps aux | grep <agent>`

### Memory leak

**Problem:** Memory grows over time

**Solutions:**
1. Check stdout/stderr buffer usage
2. Read from stdout/stderr regularly
3. Set `max_concurrent_agents` limit
4. Monitor with `top` or `htop`
5. Consider bounded channels

## License

Part of the Descartes orchestration system.
