# Unix Socket RPC Server - Phase 3.1 Implementation

## Overview

This document describes the jsonrpsee-based RPC server implementation that listens on a Unix socket for inter-process communication (IPC). This is part of Phase 3.1 of the Descartes project.

## Architecture

### Key Components

1. **RPC Server** (`src/rpc_server.rs`)
   - Built on jsonrpsee library (modern, async-first RPC framework)
   - Listens on Unix socket for local IPC
   - Implements JSON-RPC 2.0 protocol
   - Provides methods for agent management and workflow control

2. **RPC Interface** (`DescartesRpc` trait)
   - Defines the public RPC API
   - Four core methods: `spawn`, `list_tasks`, `approve`, `get_state`
   - Type-safe with automatic serialization/deserialization

3. **Error Handling**
   - Proper error types for socket operations
   - Graceful handling of connection failures
   - Automatic socket cleanup on server start

## RPC Methods

### 1. spawn
Spawn a new agent with the given configuration.

**Parameters:**
- `name`: String - The name of the agent
- `agent_type`: String - The type of agent to spawn
- `config`: Value - Additional configuration parameters (JSON object)

**Returns:**
- String - The ID of the spawned agent

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "spawn",
  "params": {
    "name": "my-agent",
    "agent_type": "worker",
    "config": {
      "max_iterations": 10,
      "timeout": 300
    }
  },
  "id": 1
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": "agent-uuid-here",
  "id": 1
}
```

### 2. list_tasks
List all tasks in the system.

**Parameters:**
- `filter`: Option<Value> - Optional filter criteria (JSON object)

**Returns:**
- Vec<TaskInfo> - List of tasks matching the filter

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "list_tasks",
  "params": {
    "filter": {
      "status": "pending"
    }
  },
  "id": 2
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": [
    {
      "id": "task-1",
      "name": "Build feature X",
      "status": "pending",
      "created_at": 1700000000,
      "updated_at": 1700000000
    }
  ],
  "id": 2
}
```

### 3. approve
Approve or reject a pending task or action.

**Parameters:**
- `task_id`: String - The ID of the task to approve
- `approved`: bool - Whether to approve (true) or reject (false)

**Returns:**
- ApprovalResult - Confirmation of the approval

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "approve",
  "params": {
    "task_id": "task-1",
    "approved": true
  },
  "id": 3
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "task_id": "task-1",
    "approved": true,
    "timestamp": 1700000100
  },
  "id": 3
}
```

### 4. get_state
Get the current state of the system or a specific entity.

**Parameters:**
- `entity_id`: Option<String> - Optional ID of a specific entity to query

**Returns:**
- Value - The current state (JSON object)

**Example:**
```json
{
  "jsonrpc": "2.0",
  "method": "get_state",
  "params": {
    "entity_id": "agent-123"
  },
  "id": 4
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "result": {
    "status": "healthy",
    "entity_id": "agent-123",
    "timestamp": "2025-11-24T05:00:00Z"
  },
  "id": 4
}
```

## Usage

### Starting the Server

```rust
use descartes_daemon::{UnixSocketRpcServer, DaemonResult};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> DaemonResult<()> {
    // Create socket path
    let socket_path = PathBuf::from("/tmp/descartes-rpc.sock");

    // Create and start the RPC server
    let server = UnixSocketRpcServer::new(socket_path);
    let handle = server.start().await?;

    // Wait for server to stop
    handle.stopped().await;

    Ok(())
}
```

### Connecting as a Client

Since jsonrpsee doesn't have built-in Unix socket client support, you'll need to implement a custom transport layer. Here's a conceptual example:

```rust
use tokio::net::UnixStream;
use serde_json::json;

async fn connect_to_server() -> anyhow::Result<()> {
    // Connect to the Unix socket
    let stream = UnixStream::connect("/tmp/descartes-rpc.sock").await?;

    // Create a JSON-RPC request
    let request = json!({
        "jsonrpc": "2.0",
        "method": "get_state",
        "params": {},
        "id": 1
    });

    // Send and receive (simplified - actual implementation needs buffering)
    // ... send request, receive response ...

    Ok(())
}
```

## Socket Management

### Socket Path
- Default: `/tmp/descartes-rpc.sock`
- Configurable via constructor
- Automatically cleaned up on server start if exists

### Permissions
- Unix socket inherits filesystem permissions
- Recommended: restrict access to specific users/groups
- Example: `chmod 600 /tmp/descartes-rpc.sock`

### Error Handling
The server handles various socket-related errors:
- **Socket already exists**: Automatically removed before starting
- **Permission denied**: Returns error with details
- **Directory doesn't exist**: Creates parent directory automatically
- **Invalid path**: Returns error immediately

## Integration with Existing Code

The new Unix socket RPC server coexists with the existing HTTP-based RPC server:

- **Old RPC server** (`src/rpc.rs`): Uses jsonrpc-core over HTTP
- **New RPC server** (`src/rpc_server.rs`): Uses jsonrpsee over Unix socket

Both can run simultaneously if needed, serving different use cases:
- HTTP: External access, web clients, remote connections
- Unix socket: Local IPC, CLI tools, GUI applications

## Testing

### Unit Tests
```bash
cd descartes/daemon
cargo test --lib rpc_server
```

### Integration Tests
Run the example server and client:

```bash
# Terminal 1: Start server
cargo run --example unix_socket_server

# Terminal 2: Test with netcat (if socket supports it)
nc -U /tmp/descartes-rpc.sock
```

### Manual Testing
Use a JSON-RPC client tool or implement a simple test client.

## Performance Characteristics

### Advantages of Unix Sockets
- **Lower latency**: No TCP/IP stack overhead
- **Higher throughput**: Direct kernel IPC
- **Better security**: Filesystem-based access control
- **Local only**: No network exposure

### Benchmarks (Expected)
- **Latency**: < 1ms per call (vs 5-10ms for HTTP)
- **Throughput**: 10,000+ RPS (vs 1,000-2,000 for HTTP)
- **Memory**: Minimal overhead per connection

## Security Considerations

### Access Control
1. **Filesystem permissions**: Use chmod to restrict access
2. **User/Group ownership**: Set appropriate owner
3. **Socket directory**: Place in secure location

### Recommendations
- Development: `/tmp/descartes-rpc.sock` with mode 600
- Production: `/var/run/descartes/rpc.sock` with mode 660, group descartes

## Future Enhancements

### Phase 3.2
- [ ] Implement authentication/authorization
- [ ] Add rate limiting per client
- [ ] Support for subscriptions (server-push events)
- [ ] Metrics and monitoring

### Phase 3.3
- [ ] Multiple socket support
- [ ] Abstract socket names (Linux)
- [ ] Windows named pipe support
- [ ] ZMQ transport option

## Troubleshooting

### Socket already in use
```bash
# Check if socket exists
ls -la /tmp/descartes-rpc.sock

# Remove manually if needed
rm /tmp/descartes-rpc.sock

# Or let the server remove it automatically on start
```

### Permission denied
```bash
# Check socket permissions
ls -la /tmp/descartes-rpc.sock

# Fix permissions
chmod 600 /tmp/descartes-rpc.sock

# Or change ownership
chown $USER /tmp/descartes-rpc.sock
```

### Cannot connect
```bash
# Verify server is running
ps aux | grep descartes

# Check socket exists
[ -S /tmp/descartes-rpc.sock ] && echo "Socket exists" || echo "Socket not found"

# Test with socat (if available)
echo '{"jsonrpc":"2.0","method":"get_state","params":{},"id":1}' | socat - UNIX-CONNECT:/tmp/descartes-rpc.sock
```

## References

- [jsonrpsee Documentation](https://docs.rs/jsonrpsee/)
- [JSON-RPC 2.0 Specification](https://www.jsonrpc.org/specification)
- [Unix Socket Programming](https://man7.org/linux/man-pages/man7/unix.7.html)
- [Tokio Unix Sockets](https://docs.rs/tokio/latest/tokio/net/struct.UnixListener.html)

## License

MIT
