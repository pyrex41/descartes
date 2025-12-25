# Phase 3: RPC Server Implementation Guide

## Overview

Phase 3 introduces the **Descartes RPC Daemon** - a JSON-RPC 2.0 compliant server that enables remote control and management of Descartes agents, workflows, and system state.

## Architecture

### Components

```
┌────────────────────────────────────────────────────────────┐
│                    RPC Clients                             │
│         (HTTP, WebSocket, CLI, GUI, Python SDK)            │
└────────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────────┐
│                  HTTP/WebSocket Servers                    │
│        Port 8080 (HTTP)  │  Port 8081 (WebSocket)          │
└────────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────────┐
│              JSON-RPC 2.0 Request Handler                  │
│           (Validation, Routing, Authentication)            │
└────────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────────┐
│                   RPC Method Handlers                      │
│  ┌──────────┬──────────┬──────────┬──────────┐            │
│  │ Agent    │ Workflow │ State    │ System   │            │
│  │ Mgmt     │ Exec     │ Query    │ Health   │            │
│  └──────────┴──────────┴──────────┴──────────┘            │
└────────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────────┐
│              Descartes Core Services                       │
│  ┌──────────┬──────────┬──────────┬──────────┐            │
│  │ Agent    │ Workflow │ State    │ Lease    │            │
│  │ Runner   │ Engine   │ Store    │ Manager  │            │
│  └──────────┴──────────┴──────────┴──────────┘            │
└────────────────────────────────────────────────────────────┘
                           ↓
┌────────────────────────────────────────────────────────────┐
│                  System Resources                          │
│       Processes, Memory, Network, File System              │
└────────────────────────────────────────────────────────────┘
```

## Implemented Features

### 1. Core RPC Server (`src/rpc.rs`)
- **JSON-RPC 2.0 compliance**
  - Request/response protocol
  - Error handling with standard error codes
  - Batch request support
  - Notification support (requests without ID)

- **Method dispatch**
  - `agent.spawn` - Create new agent instance
  - `agent.list` - List all active agents
  - `agent.kill` - Terminate agent
  - `agent.logs` - Retrieve agent logs
  - `workflow.execute` - Run workflow
  - `state.query` - Query state
  - `system.health` - Health check
  - `system.metrics` - Get metrics

### 2. HTTP Server (`src/server.rs`)
- **Hyper-based HTTP server**
  - Async request handling
  - Connection pooling
  - Graceful shutdown
  - CORS support (ready to add)

- **Endpoints**
  - `POST /` - JSON-RPC endpoint
  - `GET /` - Server info
  - `GET /metrics` - Prometheus metrics (Port 9090)

### 3. Authentication & Authorization (`src/auth.rs`)
- **JWT support**
  - Token generation and verification
  - Claims-based authorization
  - Scope/permission checking

- **API Key support**
  - Basic API key authentication
  - Per-endpoint permission validation

- **AuthContext**
  - User identification
  - Scope management
  - Permission checking

### 4. Connection Pool (`src/pool.rs`)
- **Connection lifecycle management**
  - Register/unregister connections
  - Track active connections
  - Idle timeout handling
  - Pool statistics

- **Backpressure control**
  - Max connection limit
  - Graceful rejection when full
  - Idle connection cleanup

### 5. Metrics Collection (`src/metrics.rs`)
- **Prometheus metrics**
  - Request count and duration
  - Error tracking
  - Agent lifecycle metrics
  - Connection statistics

- **Metrics endpoint**
  - Prometheus text format
  - Histogram and counter types
  - Time-series compatible

### 6. Configuration Management (`src/config.rs`)
- **TOML configuration**
  - Server settings (port, address, timeouts)
  - Authentication settings
  - Connection pool tuning
  - Logging configuration

- **CLI overrides**
  - Command-line argument support
  - Environment variable support
  - Default values

### 7. Type Definitions (`src/types.rs`)
- **RPC types**
  - RpcRequest, RpcResponse, RpcError
  - JSON-RPC 2.0 compliant structures

- **Domain types**
  - AgentInfo, AgentStatus
  - WorkflowExecution, StateQuery
  - HealthCheck, Metrics

### 8. OpenAPI Schema (`src/openapi.rs`)
- **Full OpenAPI 3.0 specification**
  - All RPC methods documented
  - Request/response schemas
  - Error responses
  - Server information

## File Structure

```
descartes/daemon/
├── Cargo.toml                    # Package manifest with dependencies
├── daemon.toml                   # Example configuration file
├── README.md                     # User documentation
├── src/
│   ├── lib.rs                   # Library root with module exports
│   ├── main.rs                  # Binary entry point
│   ├── auth.rs                  # JWT and API key authentication
│   ├── config.rs                # Configuration management
│   ├── errors.rs                # Error types and conversions
│   ├── handlers.rs              # RPC method implementations
│   ├── metrics.rs               # Prometheus metrics collection
│   ├── openapi.rs               # OpenAPI 3.0 schema generation
│   ├── pool.rs                  # Connection pooling
│   ├── rpc.rs                   # JSON-RPC 2.0 server logic
│   ├── server.rs                # HTTP/WebSocket server
│   └── types.rs                 # Type definitions
└── examples/
    └── client.rs                # Example RPC client
```

## Key Design Decisions

### 1. JSON-RPC 2.0
- **Why**: Industry standard, widely supported, language-agnostic
- **Benefit**: Simple to integrate with existing tools
- **Trade-off**: Not REST-style, but more flexible

### 2. Async/Await with Tokio
- **Why**: Rust's async runtime, excellent performance
- **Benefit**: Handle 1000s of concurrent connections efficiently
- **Trade-off**: More complex code, learning curve

### 3. Prometheus Metrics
- **Why**: Industry standard, works with Grafana
- **Benefit**: Easy monitoring and alerting
- **Trade-off**: Text format, not binary

### 4. Connection Pooling
- **Why**: Control resource usage, prevent DOS
- **Benefit**: Predictable resource consumption
- **Trade-off**: Additional complexity

## Integration Points

### With descartes-core
The daemon integrates with core services:

```rust
// Agent runner integration
let agent_runner = AgentRunner::new(config)?;
let agents = agent_runner.list_agents()?;

// State store integration
let state_store = StateStore::open(path)?;
let state = state_store.query("agent-id", "key")?;

// Notification router integration
let notifications = NotificationRouter::new(config)?;
notifications.send(event)?;
```

### With external systems
The daemon can be extended to integrate with:
- Kubernetes for container orchestration
- etcd for distributed configuration
- Kafka for event streaming
- Jaeger for distributed tracing

## Usage Examples

### Starting the daemon
```bash
# With defaults
./descartes-daemon

# With custom config
./descartes-daemon --config /etc/descartes/daemon.toml

# With auth enabled
./descartes-daemon --enable-auth --jwt-secret "my-secret"
```

### Using the Python client
```python
import requests
import json

url = "http://127.0.0.1:8080"

# Spawn agent
response = requests.post(url, json={
    "jsonrpc": "2.0",
    "method": "agent.spawn",
    "params": {
        "name": "my-agent",
        "agent_type": "basic",
        "config": {}
    },
    "id": 1
})

print(response.json())
```

### Using curl
```bash
# List agents
curl -X POST http://127.0.0.1:8080/ \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "agent.list",
    "params": {},
    "id": 1
  }'

# Get metrics
curl http://127.0.0.1:9090/metrics
```

## Performance Characteristics

### Benchmarks (Expected)
- **Request latency**: <10ms (P50), <50ms (P95)
- **Throughput**: 1000+ RPS per core
- **Connection overhead**: ~1KB per connection
- **Memory**: ~100MB baseline + per-connection overhead

### Scaling
- **Horizontal**: Multiple daemon instances behind load balancer
- **Vertical**: Increase pool size and request timeout
- **Threading**: Tokio works-stealing scheduler adapts automatically

## Security Considerations

### 1. Authentication
- **Default**: No authentication (dev mode)
- **Recommended**: JWT with strong secret
- **Best practice**: mTLS for production

### 2. Authorization
- **Scope-based**: Fine-grained permissions per method
- **Role-based**: Map users to roles
- **Resource-based**: Restrict agent access per user

### 3. Transport
- **Development**: HTTP only
- **Production**: HTTPS with valid certificates
- **Optional**: mTLS for client authentication

### 4. Input Validation
- **All RPC parameters validated**
- **JSON schema checking**
- **Type coercion prevention**

## Testing

### Unit Tests
All modules include comprehensive unit tests:
```bash
cargo test --lib
```

### Integration Tests
Create integration tests in `tests/` directory:
```bash
cargo test --test '*'
```

### Manual Testing
Use the provided client example:
```bash
cargo run --example client
```

## Future Enhancements

### Phase 3.1: WebSocket Support
- Bidirectional communication
- Server-push notifications
- Lower latency for real-time updates

### Phase 3.2: gRPC Support
- High-performance protocol
- Protobuf serialization
- Language-specific code generation

### Phase 3.3: GraphQL API
- Flexible querying
- Introspection support
- Better tooling integration

### Phase 3.4: Advanced Features
- Event subscription and streaming
- Rate limiting per client
- Quota management
- Request batching
- Caching layer

## Deployment

### Docker
```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo build --release -p descartes-daemon

FROM debian:bookworm-slim
COPY --from=builder /app/target/release/descartes-daemon /usr/local/bin/
EXPOSE 8080 8081 9090
ENTRYPOINT ["descartes-daemon"]
```

### Kubernetes
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: descartes-daemon
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: daemon
        image: descartes-daemon:latest
        ports:
        - containerPort: 8080  # HTTP
        - containerPort: 8081  # WebSocket
        - containerPort: 9090  # Metrics
        env:
        - name: RUST_LOG
          value: "info"
```

### Systemd
```ini
[Unit]
Description=Descartes RPC Daemon
After=network.target

[Service]
Type=simple
User=descartes
ExecStart=/usr/local/bin/descartes-daemon --config /etc/descartes/daemon.toml
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

## Monitoring and Observability

### Prometheus Metrics
```bash
# CPU and memory
process_cpu_seconds_total
process_resident_memory_bytes

# RPC requests
requests_total
request_duration_seconds
request_errors_total

# Agents
agents_spawned_total
agents_active
agents_killed_total

# Connections
connections_total
connections_active
```

### Logging
```json
{
  "timestamp": "2025-11-23T12:00:00Z",
  "level": "info",
  "message": "RPC request received",
  "method": "agent.spawn",
  "request_id": "uuid",
  "user_id": "user123",
  "duration_ms": 125
}
```

### Tracing (Future)
```rust
// With OpenTelemetry
#[tracing::instrument]
async fn handle_agent_spawn(...) -> DaemonResult<Value> {
    // Spans and events automatically recorded
}
```

## Troubleshooting

### Connection refused
```bash
# Check if daemon is running
curl http://127.0.0.1:8080/

# Check port binding
lsof -i :8080
```

### Authentication errors
```bash
# Verify JWT secret is set
echo $JWT_SECRET

# Check token expiry
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:8080/
```

### Pool exhausted
```bash
# Increase pool size in config
[pool]
max_size = 200  # Increase from default 100

# Check active connections
curl http://127.0.0.1:9090/metrics | grep connections_active
```

### High latency
```bash
# Check request duration metrics
curl http://127.0.0.1:9090/metrics | grep request_duration

# Increase thread pool
TOKIO_WORKER_THREADS=8 ./descartes-daemon
```

## Contributing

To extend the daemon:

1. **Add new RPC method**:
   ```rust
   // In handlers.rs
   pub async fn handle_new_method(...) -> DaemonResult<Value> {
       // Implementation
   }

   // In rpc.rs
   "new.method" => self.call_new_method(...).await,
   ```

2. **Update types**:
   ```rust
   // In types.rs
   pub struct NewMethodRequest { ... }
   pub struct NewMethodResponse { ... }
   ```

3. **Update OpenAPI schema**:
   ```rust
   // In openapi.rs
   "components": {
       "schemas": {
           "NewMethodRequest": { ... }
       }
   }
   ```

4. **Add tests**:
   ```rust
   #[tokio::test]
   async fn test_new_method() { ... }
   ```

## References

- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [OpenAPI 3.0 Specification](https://spec.openapis.org/oas/v3.0.3)
- [Hyper Documentation](https://hyper.rs/)
- [Tokio Documentation](https://tokio.rs/)
- [Prometheus Metrics](https://prometheus.io/docs/introduction/overview/)

## License

MIT
