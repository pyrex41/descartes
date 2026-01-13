# ZMQ Agent Runner - Remote Agent Orchestration

## Overview

The ZMQ Agent Runner provides a robust, distributed agent spawning and control system using ZeroMQ as the transport layer. It enables spawning and managing AI agents across remote machines, supporting massive parallel agent swarms in distributed environments.

## Table of Contents

- [Architecture](#architecture)
- [Message Schemas](#message-schemas)
- [Trait Interface](#trait-interface)
- [Usage Examples](#usage-examples)
- [Configuration](#configuration)
- [Serialization](#serialization)
- [Error Handling](#error-handling)
- [Best Practices](#best-practices)

## Architecture

### Communication Patterns

The ZMQ Agent Runner supports multiple ZeroMQ patterns:

1. **REQ/REP (Request-Reply)**: For client-server communication
   - Client sends spawn/control requests
   - Server responds with results
   - Synchronous, reliable delivery

2. **DEALER/ROUTER**: For async multi-client scenarios
   - Load balancing across multiple servers
   - Async request/response
   - Scalable for high throughput

3. **PUB/SUB**: For status updates
   - Server publishes agent status updates
   - Clients subscribe to updates
   - One-to-many broadcast

### Message Flow

```text
┌─────────────┐                    ┌─────────────┐
│   Client    │                    │   Server    │
│  (Runner)   │                    │  (Runner)   │
└──────┬──────┘                    └──────┬──────┘
       │                                  │
       │  1. SpawnRequest                 │
       ├─────────────────────────────────>│
       │     {agent_config, timeout}      │
       │                                  │
       │                                  │  2. Spawn
       │                                  │     Local
       │                                  │     Agent
       │                                  │
       │  3. SpawnResponse                │
       │<─────────────────────────────────┤
       │     {agent_info, success}        │
       │                                  │
       │                                  │
       │  4. ControlCommand               │
       ├─────────────────────────────────>│
       │     {pause/resume/stop}          │
       │                                  │
       │                                  │  5. Execute
       │                                  │     Command
       │                                  │
       │  6. CommandResponse              │
       │<─────────────────────────────────┤
       │     {success, status}            │
       │                                  │
       │                                  │
       │  7. StatusUpdate (PUB/SUB)       │
       │<─────────────────────────────────┤
       │     {status_changed, ...}        │
       │                                  │
```

## Message Schemas

All messages use **MessagePack** for efficient binary serialization. Each message type is wrapped in a `ZmqMessage` envelope for multiplexing.

### Spawn Request

Request to spawn a new agent on a remote server.

```rust
pub struct SpawnRequest {
    pub request_id: String,           // Unique request identifier
    pub config: AgentConfig,          // Agent configuration
    pub timeout_secs: Option<u64>,    // Optional spawn timeout
    pub metadata: Option<HashMap<String, String>>, // Optional metadata
}
```

**Example**:
```rust
use descartes_core::{SpawnRequest, AgentConfig};

let request = SpawnRequest {
    request_id: uuid::Uuid::new_v4().to_string(),
    config: AgentConfig {
        name: "remote-agent-1".to_string(),
        model_backend: "claude".to_string(),
        task: "Analyze codebase".to_string(),
        context: Some("project context...".to_string()),
        system_prompt: None,
        environment: HashMap::new(),
    },
    timeout_secs: Some(300),
    metadata: Some([
        ("priority".to_string(), "high".to_string()),
        ("project".to_string(), "descartes".to_string()),
    ].into_iter().collect()),
};
```

### Spawn Response

Response to a spawn request.

```rust
pub struct SpawnResponse {
    pub request_id: String,           // Request ID this responds to
    pub success: bool,                // Whether spawn succeeded
    pub agent_info: Option<AgentInfo>, // Agent info if successful
    pub error: Option<String>,        // Error message if failed
    pub server_id: Option<String>,    // Server that spawned the agent
}
```

### Control Command

Command to control a running agent.

```rust
pub struct ControlCommand {
    pub request_id: String,           // Unique request identifier
    pub agent_id: Uuid,               // Agent to control
    pub command_type: ControlCommandType, // Command type
    pub payload: Option<serde_json::Value>, // Optional payload
}

pub enum ControlCommandType {
    Pause,          // Pause the agent
    Resume,         // Resume a paused agent
    Stop,           // Stop gracefully
    Kill,           // Kill immediately
    WriteStdin,     // Write to stdin
    ReadStdout,     // Read from stdout
    ReadStderr,     // Read from stderr
    GetStatus,      // Get current status
    Signal,         // Send custom signal
}
```

**Example**:
```rust
use descartes_core::{ControlCommand, ControlCommandType};

// Pause an agent
let pause = ControlCommand {
    request_id: uuid::Uuid::new_v4().to_string(),
    agent_id: agent_id,
    command_type: ControlCommandType::Pause,
    payload: None,
};

// Write to stdin
let stdin = ControlCommand {
    request_id: uuid::Uuid::new_v4().to_string(),
    agent_id: agent_id,
    command_type: ControlCommandType::WriteStdin,
    payload: Some(serde_json::json!({
        "data": "Execute task step 2\n"
    })),
};
```

### Command Response

Response to a control command.

```rust
pub struct CommandResponse {
    pub request_id: String,           // Request ID this responds to
    pub agent_id: Uuid,               // Agent ID
    pub success: bool,                // Whether command succeeded
    pub status: Option<AgentStatus>,  // Current agent status
    pub data: Option<serde_json::Value>, // Optional response data
    pub error: Option<String>,        // Error message if failed
}
```

### Status Update

Asynchronous status update pushed from server to client.

```rust
pub struct StatusUpdate {
    pub agent_id: Uuid,               // Agent this update is about
    pub update_type: StatusUpdateType, // Type of update
    pub status: Option<AgentStatus>,  // Current status
    pub message: Option<String>,      // Optional message
    pub data: Option<serde_json::Value>, // Optional data
    pub timestamp: SystemTime,        // Update timestamp
}

pub enum StatusUpdateType {
    StatusChanged,     // Agent status changed
    OutputAvailable,   // Agent output available
    Error,             // Agent error occurred
    Completed,         // Agent completed
    Terminated,        // Agent terminated
    Heartbeat,         // Heartbeat/keepalive
}
```

### List Agents Request

Request to list agents on a remote server.

```rust
pub struct ListAgentsRequest {
    pub request_id: String,           // Unique request identifier
    pub filter_status: Option<AgentStatus>, // Optional status filter
    pub limit: Option<usize>,         // Optional result limit
}
```

### Health Check

Health check to verify server is responsive.

```rust
pub struct HealthCheckRequest {
    pub request_id: String,           // Unique request identifier
}

pub struct HealthCheckResponse {
    pub request_id: String,           // Request ID this responds to
    pub healthy: bool,                // Whether server is healthy
    pub protocol_version: String,     // Protocol version
    pub uptime_secs: Option<u64>,     // Server uptime
    pub active_agents: Option<usize>, // Number of active agents
    pub metadata: Option<HashMap<String, String>>, // Server metadata
}
```

## Trait Interface

The `ZmqAgentRunner` trait defines the interface for remote agent management.

```rust
#[async_trait]
pub trait ZmqAgentRunner: Send + Sync {
    // Connection management
    async fn connect(&self, endpoint: &str) -> AgentResult<()>;
    async fn disconnect(&self) -> AgentResult<()>;
    fn is_connected(&self) -> bool;

    // Agent lifecycle
    async fn spawn_remote(&self, config: AgentConfig, timeout_secs: Option<u64>)
        -> AgentResult<AgentInfo>;
    async fn list_remote_agents(&self, filter_status: Option<AgentStatus>, limit: Option<usize>)
        -> AgentResult<Vec<AgentInfo>>;
    async fn get_remote_agent(&self, agent_id: &Uuid)
        -> AgentResult<Option<AgentInfo>>;

    // Agent control
    async fn get_agent_status(&self, agent_id: &Uuid) -> AgentResult<AgentStatus>;
    async fn pause_agent(&self, agent_id: &Uuid) -> AgentResult<()>;
    async fn resume_agent(&self, agent_id: &Uuid) -> AgentResult<()>;
    async fn stop_agent(&self, agent_id: &Uuid) -> AgentResult<()>;
    async fn kill_agent(&self, agent_id: &Uuid) -> AgentResult<()>;

    // I/O operations
    async fn write_agent_stdin(&self, agent_id: &Uuid, data: &[u8]) -> AgentResult<()>;
    async fn read_agent_stdout(&self, agent_id: &Uuid) -> AgentResult<Option<Vec<u8>>>;
    async fn read_agent_stderr(&self, agent_id: &Uuid) -> AgentResult<Option<Vec<u8>>>;

    // Monitoring
    async fn health_check(&self) -> AgentResult<HealthCheckResponse>;
    async fn subscribe_status_updates(&self, agent_id: Option<Uuid>)
        -> AgentResult<Box<dyn futures::Stream<Item = AgentResult<StatusUpdate>> + Unpin + Send>>;
}
```

## Usage Examples

### Basic Remote Agent Spawning

```rust
use descartes_core::{ZmqAgentRunner, AgentConfig};
use std::collections::HashMap;

async fn spawn_remote_agent(runner: &impl ZmqAgentRunner) -> Result<(), Box<dyn std::error::Error>> {
    // Connect to remote server
    runner.connect("tcp://192.168.1.100:5555").await?;

    // Configure agent
    let config = AgentConfig {
        name: "data-analyzer".to_string(),
        model_backend: "claude".to_string(),
        task: "Analyze CSV data and generate insights".to_string(),
        context: Some("data.csv".to_string()),
        system_prompt: Some("You are a data analysis expert".to_string()),
        environment: HashMap::new(),
    };

    // Spawn agent with 5 minute timeout
    let agent = runner.spawn_remote(config, Some(300)).await?;
    println!("Spawned agent: {} on remote server", agent.id);

    // Get status
    let status = runner.get_agent_status(&agent.id).await?;
    println!("Agent status: {:?}", status);

    Ok(())
}
```

### Controlling Remote Agents

```rust
use descartes_core::{ZmqAgentRunner, AgentStatus};
use uuid::Uuid;

async fn control_agent(
    runner: &impl ZmqAgentRunner,
    agent_id: Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    // Write to agent's stdin
    let input = b"Process file: data.txt\n";
    runner.write_agent_stdin(&agent_id, input).await?;

    // Read output
    if let Some(output) = runner.read_agent_stdout(&agent_id).await? {
        println!("Agent output: {}", String::from_utf8_lossy(&output));
    }

    // Pause for inspection
    runner.pause_agent(&agent_id).await?;
    println!("Agent paused");

    // Resume
    runner.resume_agent(&agent_id).await?;
    println!("Agent resumed");

    // Stop gracefully when done
    runner.stop_agent(&agent_id).await?;
    println!("Agent stopped");

    Ok(())
}
```

### Monitoring Agent Status

```rust
use descartes_core::{ZmqAgentRunner, StatusUpdate, StatusUpdateType};
use futures::StreamExt;

async fn monitor_agents(runner: &impl ZmqAgentRunner) -> Result<(), Box<dyn std::error::Error>> {
    // Subscribe to all status updates
    let mut updates = runner.subscribe_status_updates(None).await?;

    // Process updates
    while let Some(update) = updates.next().await {
        let update = update?;

        match update.update_type {
            StatusUpdateType::StatusChanged => {
                println!("Agent {} status changed to {:?}",
                    update.agent_id, update.status);
            }
            StatusUpdateType::OutputAvailable => {
                println!("Agent {} has new output", update.agent_id);
            }
            StatusUpdateType::Error => {
                eprintln!("Agent {} error: {:?}",
                    update.agent_id, update.message);
            }
            StatusUpdateType::Completed => {
                println!("Agent {} completed", update.agent_id);
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
```

### Multi-Server Setup

```rust
use descartes_core::{ZmqAgentRunner, AgentConfig};
use std::collections::HashMap;

async fn spawn_distributed_swarm(
    servers: Vec<&str>,
    num_agents_per_server: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut handles = vec![];

    for (i, server_endpoint) in servers.iter().enumerate() {
        for j in 0..num_agents_per_server {
            let endpoint = server_endpoint.to_string();
            let agent_name = format!("agent-{}-{}", i, j);

            let handle = tokio::spawn(async move {
                // Create runner for this agent
                let runner = create_zmq_runner(); // Your implementation

                // Connect to server
                runner.connect(&endpoint).await?;

                // Spawn agent
                let config = AgentConfig {
                    name: agent_name.clone(),
                    model_backend: "claude".to_string(),
                    task: "Process task".to_string(),
                    context: None,
                    system_prompt: None,
                    environment: HashMap::new(),
                };

                let agent = runner.spawn_remote(config, Some(600)).await?;
                println!("Spawned {} on {}", agent_name, endpoint);

                Ok::<_, Box<dyn std::error::Error>>(agent)
            });

            handles.push(handle);
        }
    }

    // Wait for all spawns to complete
    for handle in handles {
        handle.await??;
    }

    println!("Spawned {} agents across {} servers",
        servers.len() * num_agents_per_server, servers.len());

    Ok(())
}
```

## Configuration

### ZmqRunnerConfig

```rust
pub struct ZmqRunnerConfig {
    pub endpoint: String,                 // ZMQ endpoint
    pub connection_timeout_secs: u64,     // Connection timeout
    pub request_timeout_secs: u64,        // Request timeout
    pub auto_reconnect: bool,             // Auto-reconnect on failure
    pub max_reconnect_attempts: u32,      // Max reconnection attempts
    pub reconnect_delay_secs: u64,        // Delay between reconnects
    pub enable_heartbeat: bool,           // Enable heartbeat
    pub heartbeat_interval_secs: u64,     // Heartbeat interval
    pub server_id: Option<String>,        // Server identifier
}
```

**Default Configuration**:
```rust
ZmqRunnerConfig {
    endpoint: "tcp://localhost:5555",
    connection_timeout_secs: 30,
    request_timeout_secs: 30,
    auto_reconnect: true,
    max_reconnect_attempts: 3,
    reconnect_delay_secs: 5,
    enable_heartbeat: true,
    heartbeat_interval_secs: 30,
    server_id: None,
}
```

### TOML Configuration

```toml
[zmq_runner]
endpoint = "tcp://192.168.1.100:5555"
connection_timeout_secs = 30
request_timeout_secs = 60
auto_reconnect = true
max_reconnect_attempts = 5
reconnect_delay_secs = 10
enable_heartbeat = true
heartbeat_interval_secs = 30
server_id = "server-01"
```

## Serialization

### MessagePack Format

All messages use **MessagePack** (via `rmp-serde`) for efficient binary serialization:

- **Space efficient**: ~30-50% smaller than JSON
- **Fast**: 2-5x faster serialization/deserialization
- **Type safe**: Preserves type information
- **Binary safe**: Can encode binary data

### Serialization Utilities

```rust
use descartes_core::{
    serialize_zmq_message, deserialize_zmq_message,
    ZmqMessage, SpawnRequest
};

// Serialize
let request = SpawnRequest { /* ... */ };
let msg = ZmqMessage::SpawnRequest(request);
let bytes = serialize_zmq_message(&msg)?;

// Deserialize
let deserialized = deserialize_zmq_message(&bytes)?;
match deserialized {
    ZmqMessage::SpawnRequest(req) => {
        println!("Got spawn request: {:?}", req);
    }
    _ => {}
}
```

### Message Size Validation

```rust
use descartes_core::{validate_message_size, MAX_MESSAGE_SIZE};

// Validate before sending
validate_message_size(bytes.len())?; // Fails if > 10MB
```

## Error Handling

All ZMQ operations return `AgentResult<T>`, which is an alias for `Result<T, AgentError>`.

### Common Errors

```rust
use descartes_core::{AgentError, AgentResult};

match runner.spawn_remote(config, None).await {
    Ok(agent) => println!("Spawned: {:?}", agent),
    Err(AgentError::SpawnFailed(msg)) => {
        eprintln!("Failed to spawn: {}", msg);
    }
    Err(AgentError::NotFound(msg)) => {
        eprintln!("Agent not found: {}", msg);
    }
    Err(AgentError::ExecutionError(msg)) => {
        eprintln!("Execution error: {}", msg);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

### Retry Logic

```rust
use tokio::time::{sleep, Duration};

async fn spawn_with_retry(
    runner: &impl ZmqAgentRunner,
    config: AgentConfig,
    max_retries: u32,
) -> AgentResult<AgentInfo> {
    let mut attempts = 0;

    loop {
        match runner.spawn_remote(config.clone(), Some(300)).await {
            Ok(agent) => return Ok(agent),
            Err(e) if attempts < max_retries => {
                attempts += 1;
                eprintln!("Spawn failed (attempt {}): {}", attempts, e);
                sleep(Duration::from_secs(5)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Best Practices

### 1. Connection Management

```rust
// Use connection pooling for multiple requests
let runner = create_zmq_runner();
runner.connect("tcp://server:5555").await?;

// Reuse the connection
for i in 0..10 {
    let agent = runner.spawn_remote(config.clone(), None).await?;
    println!("Spawned agent {}: {}", i, agent.id);
}

// Clean disconnect
runner.disconnect().await?;
```

### 2. Timeout Handling

```rust
use tokio::time::{timeout, Duration};

// Set timeout for spawn operation
let result = timeout(
    Duration::from_secs(60),
    runner.spawn_remote(config, Some(300))
).await;

match result {
    Ok(Ok(agent)) => println!("Spawned: {:?}", agent),
    Ok(Err(e)) => eprintln!("Spawn error: {}", e),
    Err(_) => eprintln!("Timeout after 60 seconds"),
}
```

### 3. Health Monitoring

```rust
// Periodic health checks
use tokio::time::{interval, Duration};

let mut health_check_interval = interval(Duration::from_secs(30));

loop {
    health_check_interval.tick().await;

    match runner.health_check().await {
        Ok(health) if health.healthy => {
            println!("Server healthy: {} agents",
                health.active_agents.unwrap_or(0));
        }
        Ok(health) => {
            eprintln!("Server unhealthy!");
        }
        Err(e) => {
            eprintln!("Health check failed: {}", e);
            // Attempt reconnection
            runner.disconnect().await?;
            runner.connect("tcp://server:5555").await?;
        }
    }
}
```

### 4. Graceful Shutdown

```rust
use tokio::signal;

async fn run_with_graceful_shutdown(
    runner: impl ZmqAgentRunner,
) -> Result<(), Box<dyn std::error::Error>> {
    // Spawn agents
    let agents = spawn_agents(&runner).await?;

    // Wait for CTRL+C
    signal::ctrl_c().await?;
    println!("Shutting down gracefully...");

    // Stop all agents
    for agent in agents {
        if let Err(e) = runner.stop_agent(&agent.id).await {
            eprintln!("Failed to stop agent {}: {}", agent.id, e);
        }
    }

    // Disconnect
    runner.disconnect().await?;
    println!("Shutdown complete");

    Ok(())
}
```

### 5. Error Recovery

```rust
// Implement automatic recovery
async fn resilient_spawn(
    runner: &impl ZmqAgentRunner,
    config: AgentConfig,
) -> AgentResult<AgentInfo> {
    const MAX_RETRIES: u32 = 3;

    for attempt in 1..=MAX_RETRIES {
        match runner.spawn_remote(config.clone(), Some(300)).await {
            Ok(agent) => return Ok(agent),
            Err(e) => {
                eprintln!("Attempt {}/{} failed: {}", attempt, MAX_RETRIES, e);

                if attempt == MAX_RETRIES {
                    return Err(e);
                }

                // Exponential backoff
                let delay = 2u64.pow(attempt - 1);
                tokio::time::sleep(Duration::from_secs(delay)).await;

                // Try reconnecting
                let _ = runner.disconnect().await;
                runner.connect("tcp://server:5555").await?;
            }
        }
    }

    unreachable!()
}
```

## Protocol Version

Current protocol version: **1.0.0**

The protocol version is included in health check responses to ensure compatibility between client and server.

```rust
use descartes_core::ZMQ_PROTOCOL_VERSION;

assert_eq!(ZMQ_PROTOCOL_VERSION, "1.0.0");
```

## Performance Considerations

### Message Size

- Maximum message size: **10 MB**
- Typical spawn request: ~1-10 KB
- Typical status update: ~100-500 bytes
- Use MessagePack compression for large payloads

### Throughput

Expected performance on modern hardware:

- **Spawn rate**: 100-500 agents/sec (single server)
- **Control commands**: 1000+ ops/sec
- **Status updates**: 10000+ msgs/sec (PUB/SUB)

### Scaling

For high-scale deployments:

1. **Horizontal scaling**: Run multiple ZMQ servers
2. **Load balancing**: Use DEALER/ROUTER pattern
3. **Sharding**: Partition agents across servers
4. **Monitoring**: Track metrics per server

## Security

### Network Security

- Use **IPC sockets** (`ipc://`) for same-machine communication
- Use **TCP with TLS** for remote communication
- Use **CurveZMQ** for encryption (requires libzmq with curve support)

### Authentication

- Implement authentication at application layer
- Use JWT tokens in metadata fields
- Validate requests on server side

### Best Practices

1. Validate all incoming messages
2. Limit message sizes (enforced at 10MB)
3. Rate limit spawn requests
4. Use encryption for sensitive data
5. Implement access control lists

## Next Steps

This implementation provides the foundation for ZMQ-based agent communication. The next steps are:

1. **phase3:2.2**: Implement the actual ZMQ communication layer
2. **phase3:2.3**: Implement server-side agent spawning
3. **phase3:2.4**: Implement client-side agent control

## References

- [ZeroMQ Guide](https://zguide.zeromq.org/)
- [MessagePack Specification](https://msgpack.org/)
- [Descartes Architecture Docs](../../docs/architecture/)
