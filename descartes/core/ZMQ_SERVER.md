# ZMQ Agent Server Documentation

## Overview

The ZMQ Agent Server provides server-side infrastructure for distributed agent orchestration. It handles incoming requests from ZMQ clients to spawn, control, and monitor agents running on remote machines.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      ZMQ Agent Server                        │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  ┌───────────────┐         ┌──────────────────┐            │
│  │ ZMQ Connection│         │  Agent Registry  │            │
│  │  (REP Socket) │         │    (DashMap)     │            │
│  └───────┬───────┘         └────────┬─────────┘            │
│          │                          │                        │
│          ├──────────────────────────┤                        │
│          │   Request Handler        │                        │
│          │                          │                        │
│  ┌───────▼────────┬─────────────────▼─────────┐            │
│  │ Spawn Request  │  Control Command          │            │
│  │ List Agents    │  Status Updates           │            │
│  │ Health Check   │  Lifecycle Monitoring     │            │
│  └────────────────┴───────────────────────────┘            │
│                          │                                   │
│                          ▼                                   │
│                ┌──────────────────┐                         │
│                │ LocalProcessRunner│                         │
│                │   (Agent Spawner) │                         │
│                └──────────────────┘                         │
│                                                               │
└─────────────────────────────────────────────────────────────┘
```

## Key Components

### ZmqAgentServer

The main server struct that orchestrates all server-side operations.

**Responsibilities:**
- Accept and process incoming ZMQ requests
- Spawn agents using LocalProcessRunner
- Track active agents in registry
- Handle control commands (pause, resume, stop, kill)
- Send status updates
- Manage server lifecycle

### Agent Registry

A thread-safe registry (using `DashMap`) that tracks all spawned agents.

**Stored Information:**
- Agent configuration and metadata
- Spawn timestamp and request ID
- Output buffers (stdout/stderr)
- Last status update timestamp

### Request Handlers

The server implements handlers for each message type:

1. **SpawnRequest** → `handle_spawn_request()`
2. **ControlCommand** → `handle_control_command()`
3. **ListAgentsRequest** → `handle_list_agents_request()`
4. **HealthCheckRequest** → `handle_health_check_request()`

## Usage

### Basic Server Setup

```rust
use descartes_core::{ZmqAgentServer, ZmqServerConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure the server
    let config = ZmqServerConfig {
        endpoint: "tcp://0.0.0.0:5555".to_string(),
        server_id: "server-01".to_string(),
        max_agents: 100,
        status_update_interval_secs: 10,
        enable_status_updates: true,
        ..Default::default()
    };

    // Create and start the server
    let server = ZmqAgentServer::new(config);
    server.start().await?;

    Ok(())
}
```

### Server Configuration

```rust
pub struct ZmqServerConfig {
    /// Server endpoint to bind to
    pub endpoint: String,

    /// Server identifier (for multi-server setups)
    pub server_id: String,

    /// Maximum concurrent agents
    pub max_agents: usize,

    /// Status update interval in seconds
    pub status_update_interval_secs: u64,

    /// Enable automatic status updates
    pub enable_status_updates: bool,

    /// Process runner configuration
    pub runner_config: ProcessRunnerConfig,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,
}
```

### Graceful Shutdown

```rust
use tokio::signal;

// Wait for Ctrl+C
signal::ctrl_c().await?;

// Stop the server gracefully
server.stop().await?;
```

### Monitoring Server Statistics

```rust
// Get current statistics
let stats = server.stats();

println!("Spawn requests: {}", stats.spawn_requests);
println!("Successful spawns: {}", stats.successful_spawns);
println!("Active agents: {}", server.active_agent_count());
println!("Uptime: {}s", server.uptime_secs().unwrap_or(0));
```

## Request Flow

### 1. Spawn Request Flow

```
Client                    Server                    LocalProcessRunner
  |                         |                              |
  |--- SpawnRequest ------->|                              |
  |                         |--- spawn(config) ----------->|
  |                         |                              |-- Create process
  |                         |                              |-- Setup I/O pipes
  |                         |<-- AgentHandle --------------|
  |                         |                              |
  |                         |-- Add to registry            |
  |<-- SpawnResponse -------|                              |
  |    (with AgentInfo)     |                              |
```

### 2. Control Command Flow

```
Client                    Server                    LocalProcessRunner
  |                         |                              |
  |--- ControlCommand ----->|                              |
  |    (Pause/Resume/Stop)  |                              |
  |                         |-- Check registry             |
  |                         |                              |
  |                         |--- execute_command --------->|
  |                         |<-- Result -------------------|
  |<-- CommandResponse -----|                              |
```

### 3. Status Update Flow

```
Server                          Monitoring Task
  |                                    |
  |<--- Periodic tick (every 10s) ----|
  |                                    |
  |-- Check all agents                |
  |                                    |
  |-- Send StatusUpdate (broadcast)   |
  |                                    |
  |-- Update last_status_update ------>|
```

## Lifecycle Management

### Agent Monitoring

The server automatically monitors agent lifecycle:

1. **Periodic Health Checks**: Verify agents are still running
2. **Status Updates**: Broadcast agent status changes
3. **Cleanup**: Remove completed/terminated agents from registry

### Graceful Shutdown

When stopping the server:

1. Send shutdown signal to all background tasks
2. Stop all running agents gracefully
3. Wait for agents to terminate (with timeout)
4. Force kill any remaining agents
5. Clear agent registry
6. Disconnect ZMQ socket

## Error Handling

### Spawn Failures

```rust
SpawnResponse {
    success: false,
    error: Some("Maximum agent limit reached"),
    agent_info: None,
    server_id: Some("server-01"),
}
```

### Control Command Failures

```rust
CommandResponse {
    success: false,
    error: Some("Agent not found"),
    status: None,
    data: None,
}
```

### Server Errors

All errors are logged with tracing:

```rust
tracing::error!("Error handling request: {}", e);
self.stats.write().errors += 1;
```

## Security Considerations

### Network Security

1. **Endpoint Binding**: Use `0.0.0.0` for all interfaces or specific IP for restricted access
2. **Firewall Rules**: Configure firewall to allow ZMQ port (default: 5555)
3. **TLS/Encryption**: Consider ZMQ CURVE security for production

### Resource Limits

1. **Max Agents**: Configurable limit to prevent resource exhaustion
2. **Message Size**: Enforced MAX_MESSAGE_SIZE (10 MB) to prevent DOS
3. **Timeouts**: Request timeouts prevent hanging connections

### Input Validation

1. **Message Validation**: All incoming messages are validated
2. **Agent ID Verification**: Commands verify agent exists before execution
3. **Configuration Sanitization**: Agent configs are validated before spawning

## Performance Tuning

### Configuration Options

```rust
ZmqServerConfig {
    max_agents: 100,                    // Adjust based on server capacity
    status_update_interval_secs: 10,    // Lower = more updates, higher CPU
    request_timeout_secs: 30,           // Adjust based on network latency
    runner_config: ProcessRunnerConfig {
        max_concurrent_agents: Some(50), // OS process limits
        enable_health_checks: true,      // Enable for production
        health_check_interval_secs: 30,  // Balance monitoring overhead
        ..Default::default()
    },
}
```

### Monitoring Recommendations

1. **Track Statistics**: Monitor spawn success/failure rates
2. **Agent Count**: Watch active agent count over time
3. **Error Rate**: Alert on elevated error counts
4. **Response Times**: Monitor request processing latency

## Production Deployment

### Systemd Service Example

```ini
[Unit]
Description=Descartes ZMQ Agent Server
After=network.target

[Service]
Type=simple
User=descartes
WorkingDirectory=/opt/descartes
ExecStart=/usr/local/bin/descartes-server --config /etc/descartes/server.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
```

### Docker Deployment

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin descartes-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libzmq5
COPY --from=builder /app/target/release/descartes-server /usr/local/bin/
EXPOSE 5555
CMD ["descartes-server"]
```

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: descartes-server
spec:
  replicas: 3
  selector:
    matchLabels:
      app: descartes-server
  template:
    metadata:
      labels:
        app: descartes-server
    spec:
      containers:
      - name: server
        image: descartes/server:latest
        ports:
        - containerPort: 5555
        env:
        - name: SERVER_ENDPOINT
          value: "tcp://0.0.0.0:5555"
        - name: MAX_AGENTS
          value: "100"
```

## Troubleshooting

### Server Won't Start

**Issue**: Server fails to bind to endpoint

```
Error: Failed to bind REP socket: Address already in use
```

**Solution**:
- Check if another process is using the port: `netstat -tulpn | grep 5555`
- Change endpoint in configuration
- Kill existing process

### Agents Not Spawning

**Issue**: Spawn requests fail

**Common Causes**:
1. Maximum agent limit reached
2. CLI tool not installed (e.g., `claude`, `opencode`)
3. Insufficient permissions
4. Resource constraints (memory, CPU)

**Solution**:
- Check server logs
- Verify CLI tools are available: `which claude`
- Increase max_agents limit
- Monitor system resources

### Memory Issues

**Issue**: Server memory usage grows over time

**Causes**:
- Agents not being cleaned up
- Output buffers accumulating

**Solution**:
- Ensure agent monitoring task is running
- Manually clean up completed agents
- Reduce output buffer sizes

## Testing

### Unit Tests

```rust
#[test]
fn test_server_creation() {
    let config = ZmqServerConfig::default();
    let server = ZmqAgentServer::new(config);
    assert!(!server.is_running());
}
```

### Integration Tests

```rust
#[tokio::test]
#[ignore] // Requires ZMQ setup
async fn test_server_spawn_agent() {
    let server = start_test_server("tcp://127.0.0.1:15557").await;
    let client = create_test_client("tcp://127.0.0.1:15557");

    let agent = client.spawn_remote(config, Some(30)).await.unwrap();
    assert_eq!(agent.name, "test-agent");
}
```

### Load Testing

Use `zmq_load_test` tool to simulate high load:

```bash
cargo run --example zmq_load_test -- \
    --endpoint tcp://localhost:5555 \
    --clients 10 \
    --requests 1000
```

## API Reference

### Server Methods

#### `new(config: ZmqServerConfig) -> Self`
Create a new server instance.

#### `start(&self) -> AgentResult<()>`
Start the server and begin accepting requests.

#### `stop(&self) -> AgentResult<()>`
Stop the server gracefully.

#### `is_running(&self) -> bool`
Check if the server is running.

#### `stats(&self) -> ServerStats`
Get current server statistics.

#### `active_agent_count(&self) -> usize`
Get the number of active agents.

#### `uptime_secs(&self) -> Option<u64>`
Get server uptime in seconds.

## Examples

See:
- `examples/zmq_server_example.rs` - Basic server usage
- `examples/zmq_client_example.rs` - Client connecting to server
- `tests/zmq_server_integration_tests.rs` - Integration tests

## Related Documentation

- [ZMQ Agent Runner](./ZMQ_AGENT_RUNNER.md) - Trait and message definitions
- [ZMQ Communication Layer](./ZMQ_COMMUNICATION_LAYER_SUMMARY.md) - Low-level socket management
- [Agent Runner](./AGENT_RUNNER.md) - Local process spawning
