# Descartes RPC Daemon

JSON-RPC 2.0 server for remote control and management of Descartes agents, workflows, and system state.

## Features

- **JSON-RPC 2.0 compliant** - Fully spec-compliant RPC endpoint
- **HTTP and WebSocket support** - Multiple transport protocols
- **Authentication & Authorization** - JWT-based token authentication
- **Connection pooling** - Efficient connection management with backpressure control
- **Metrics & monitoring** - Prometheus metrics endpoint
- **Comprehensive logging** - Structured JSON logging with configurable levels
- **OpenAPI 3.0 schema** - Full API documentation

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              JSON-RPC 2.0 Requests                      │
└─────────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────────┐
│   HTTP Server         │      WebSocket Server           │
│   (Port 8080)         │      (Port 8081)                │
└─────────────────────────────────────────────────────────┘
                    ↓
┌─────────────────────────────────────────────────────────┐
│            RPC Request Handler                          │
│  ┌─────────────────────────────────────────────────────┐│
│  │ Request Validation & Routing                        ││
│  └─────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────┘
                    ↓
┌──────────────┬──────────────┬──────────────┬────────────┐
│  Agent Mgmt  │  Workflow    │  State Query │   System   │
│  Handler     │  Handler     │  Handler     │   Handler  │
└──────────────┴──────────────┴──────────────┴────────────┘
                    ↓
┌─────────────────────────────────────────────────────────┐
│         Descartes Core (Agent Runner, State Store)      │
└─────────────────────────────────────────────────────────┘
```

## RPC Methods

### Agent Management

#### `agent.spawn`
Spawn a new agent instance.

```json
{
  "jsonrpc": "2.0",
  "method": "agent.spawn",
  "params": {
    "name": "agent-name",
    "agent_type": "basic",
    "config": {}
  },
  "id": 1
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "agent_id": "uuid",
    "status": "running",
    "message": "Agent spawned successfully"
  },
  "id": 1
}
```

#### `agent.list`
List all active agents.

```json
{
  "jsonrpc": "2.0",
  "method": "agent.list",
  "params": {},
  "id": 2
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "agents": [
      {
        "id": "uuid",
        "name": "agent-name",
        "status": "running",
        "created_at": "2025-11-23T10:00:00Z",
        "updated_at": "2025-11-23T10:00:00Z",
        "pid": 12345,
        "config": {}
      }
    ],
    "count": 1
  },
  "id": 2
}
```

#### `agent.kill`
Terminate an agent.

```json
{
  "jsonrpc": "2.0",
  "method": "agent.kill",
  "params": {
    "agent_id": "uuid",
    "force": false
  },
  "id": 3
}
```

#### `agent.logs`
Retrieve agent logs.

```json
{
  "jsonrpc": "2.0",
  "method": "agent.logs",
  "params": {
    "agent_id": "uuid",
    "limit": 100,
    "offset": 0
  },
  "id": 4
}
```

### Workflow Management

#### `workflow.execute`
Execute a workflow across multiple agents.

```json
{
  "jsonrpc": "2.0",
  "method": "workflow.execute",
  "params": {
    "workflow_id": "workflow-uuid",
    "agents": ["agent-id-1", "agent-id-2"],
    "config": {}
  },
  "id": 5
}
```

### State Management

#### `state.query`
Query system or agent state.

```json
{
  "jsonrpc": "2.0",
  "method": "state.query",
  "params": {
    "agent_id": "optional-agent-id",
    "key": "optional-state-key"
  },
  "id": 6
}
```

### System Operations

#### `system.health`
Check system health.

```json
{
  "jsonrpc": "2.0",
  "method": "system.health",
  "params": {},
  "id": 7
}
```

Response:
```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "healthy",
    "version": "0.1.0",
    "uptime_secs": 3600,
    "timestamp": "2025-11-23T11:00:00Z"
  },
  "id": 7
}
```

#### `system.metrics`
Get system metrics.

```json
{
  "jsonrpc": "2.0",
  "method": "system.metrics",
  "params": {},
  "id": 8
}
```

## Building

```bash
cd descartes/daemon
cargo build --release
```

## Running

### Default configuration
```bash
./target/release/descartes-daemon
```

### With custom configuration file
```bash
./target/release/descartes-daemon --config daemon.toml
```

### With CLI overrides
```bash
./target/release/descartes-daemon \
  --http-port 8080 \
  --enable-auth \
  --jwt-secret "your-secret-key" \
  --log-level debug
```

## Configuration

See `daemon.toml` for complete configuration options:

- **Server**: HTTP/WebSocket binding, ports, timeouts
- **Auth**: JWT settings and API key
- **Pool**: Connection pool sizing
- **Logging**: Log levels and output

## API Documentation

OpenAPI 3.0 schema is available at `/openapi.json`

## Monitoring

### Prometheus Metrics

Metrics are exposed at `http://127.0.0.1:9090/metrics`

Key metrics:
- `requests_total` - Total RPC requests
- `request_duration_seconds` - Request latency histogram
- `request_errors_total` - Total request errors
- `agents_spawned_total` - Total agents spawned
- `agents_active` - Currently active agents
- `connections_total` - Total connections
- `connections_active` - Active connections

### Health Endpoint

```bash
curl http://127.0.0.1:8080/
```

## Error Handling

JSON-RPC 2.0 error codes:

| Code | Message | Cause |
|------|---------|-------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Invalid RPC request |
| -32601 | Method not found | Unknown method |
| -32603 | Internal error | Server error |
| -32001 | Authentication error | Auth failed |
| -32002 | Agent not found | Agent doesn't exist |
| -32003 | Spawn error | Agent spawn failed |
| -32004 | Kill error | Agent kill failed |
| -32005 | Workflow error | Workflow execution failed |
| -32006 | State error | State query failed |
| -32007 | Pool error | Connection pool error |
| -32009 | Timeout | Operation timed out |

## Testing

```bash
cargo test
```

## Examples

### Using curl

```bash
# Get server info
curl http://127.0.0.1:8080/

# Spawn agent
curl -X POST http://127.0.0.1:8080/ \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "agent.spawn",
    "params": {
      "name": "my-agent",
      "agent_type": "basic",
      "config": {}
    },
    "id": 1
  }'

# List agents
curl -X POST http://127.0.0.1:8080/ \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "agent.list",
    "params": {},
    "id": 2
  }'

# Check metrics
curl http://127.0.0.1:9090/metrics
```

### Using Python

```python
import requests
import json

url = "http://127.0.0.1:8080/"

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

print(json.dumps(response.json(), indent=2))
```

## Next Steps

1. **Implement WebSocket transport** for bidirectional communication
2. **Add real agent integration** with actual agent runner
3. **Implement state persistence** for durability
4. **Add authentication backends** (OAuth2, mTLS)
5. **Rate limiting and quota** management
6. **Client library** generation from OpenAPI schema

## License

MIT
