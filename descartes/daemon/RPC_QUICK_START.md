# RPC Quick Start Guide

## For CLI Developers (like scud)

### 1. Add dependency

```toml
[dependencies]
descartes-daemon = { path = "../daemon" }
```

### 2. Create client

```rust
use descartes_daemon::UnixSocketRpcClient;
use std::path::PathBuf;

let client = UnixSocketRpcClient::default_client()?;
// Or with custom socket:
// let client = UnixSocketRpcClient::new(PathBuf::from("/path/to/socket"))?;
```

### 3. Make RPC calls

```rust
// Spawn an agent
let config = serde_json::json!({
    "task": "Write hello world",
    "environment": {}
});
let agent_id = client.spawn("my-agent", "worker", config).await?;

// List tasks
let tasks = client.list_tasks(None).await?;
println!("Found {} tasks", tasks.len());

// Filter tasks
let filter = serde_json::json!({ "status": "todo" });
let todo_tasks = client.list_tasks(Some(filter)).await?;

// Approve a task
let result = client.approve(&task_id, true).await?;

// Get system state
let state = client.get_state(None).await?;
```

### 4. Handle errors

```rust
use descartes_daemon::DaemonError;

match client.spawn("agent", "type", config).await {
    Ok(id) => println!("Spawned: {}", id),
    Err(DaemonError::ConnectionError(e)) => {
        eprintln!("Failed to connect: {}", e);
        eprintln!("Make sure daemon is running");
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

### Complete CLI Example

```rust
use descartes_daemon::{UnixSocketRpcClient, DaemonError};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect
    let client = UnixSocketRpcClient::default_client()?;
    client.test_connection().await?;

    // Spawn agent
    let config = json!({
        "task": "Create a web server",
        "environment": { "PORT": "3000" }
    });
    let agent_id = client.spawn("web-server-agent", "rust-dev", config).await?;
    println!("Agent spawned: {}", agent_id);

    // List tasks
    let tasks = client.list_tasks(None).await?;
    for task in tasks {
        println!("Task: {} [{}]", task.name, task.status);
    }

    Ok(())
}
```

## For GUI Developers (Iced)

### 1. Add dependencies

```toml
[dependencies]
descartes-gui = { path = "../gui" }
descartes-daemon = { path = "../daemon" }
iced = "0.12"
```

### 2. Create GUI with RPC client

```rust
use descartes_gui::GuiUnixRpcClient;
use iced::{Application, Command, Element};

struct MyApp {
    rpc: GuiUnixRpcClient,
    status: String,
}

#[derive(Debug, Clone)]
enum Message {
    Connect,
    Connected(Result<(), String>),
    SpawnAgent,
    AgentSpawned(Result<String, String>),
}

impl Application for MyApp {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Flags = ();
    type Theme = iced::Theme;

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        let rpc = GuiUnixRpcClient::default().unwrap();
        let app = MyApp {
            rpc,
            status: "Disconnected".to_string(),
        };
        (app, Command::none())
    }

    fn title(&self) -> String {
        "My Descartes App".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Connect => {
                let rpc = self.rpc.clone();
                Command::perform(
                    async move {
                        rpc.connect().await.map_err(|e| e.to_string())
                    },
                    Message::Connected,
                )
            }

            Message::Connected(result) => {
                match result {
                    Ok(_) => self.status = "Connected".to_string(),
                    Err(e) => self.status = format!("Error: {}", e),
                }
                Command::none()
            }

            Message::SpawnAgent => {
                let rpc = self.rpc.clone();
                Command::perform(
                    async move {
                        let config = serde_json::json!({
                            "task": "Build a GUI app",
                            "environment": {}
                        });
                        rpc.spawn_agent("gui-agent", "rust-dev", config)
                            .await
                            .map_err(|e| e.to_string())
                    },
                    Message::AgentSpawned,
                )
            }

            Message::AgentSpawned(result) => {
                match result {
                    Ok(id) => self.status = format!("Agent spawned: {}", id),
                    Err(e) => self.status = format!("Error: {}", e),
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> Element<Self::Message> {
        // Build your GUI here
        unimplemented!()
    }
}
```

## Running the Server

### Development

```bash
cargo run --bin descartes-daemon
```

The server will listen on `/tmp/descartes-rpc.sock` by default.

### Production

```bash
# Custom socket path
DESCARTES_SOCKET_PATH=/var/run/descartes/rpc.sock \
  cargo run --release --bin descartes-daemon

# Set permissions
chmod 660 /var/run/descartes/rpc.sock
chown descartes:descartes /var/run/descartes/rpc.sock
```

## Testing

### Run integration tests

```bash
cd descartes/daemon
cargo test --test rpc_compatibility_test
```

### Run examples

```bash
# Terminal 1: Start server
cargo run --bin descartes-daemon

# Terminal 2: Run CLI example
cargo run --example cli_rpc_integration

# Terminal 3: Run GUI example
cargo run --example gui_rpc_integration

# Terminal 4: Run multi-client test
cargo run --example multi_client_test
```

## Common Patterns

### Connection Retry

```rust
use tokio::time::{sleep, Duration};

async fn connect_with_retry(max_retries: u32) -> Result<UnixSocketRpcClient, DaemonError> {
    let client = UnixSocketRpcClient::default_client()?;

    for attempt in 1..=max_retries {
        match client.test_connection().await {
            Ok(_) => return Ok(client),
            Err(e) if attempt == max_retries => return Err(e),
            Err(_) => {
                eprintln!("Connection attempt {} failed, retrying...", attempt);
                sleep(Duration::from_secs(1)).await;
            }
        }
    }

    unreachable!()
}
```

### Polling for Updates

```rust
use tokio::time::{interval, Duration};

async fn poll_tasks(client: &UnixSocketRpcClient) {
    let mut interval = interval(Duration::from_secs(5));

    loop {
        interval.tick().await;

        match client.list_tasks(None).await {
            Ok(tasks) => {
                println!("Current tasks: {}", tasks.len());
                // Update UI or process tasks
            }
            Err(e) => eprintln!("Failed to fetch tasks: {}", e),
        }
    }
}
```

### Batch Operations

```rust
// Execute multiple operations concurrently
use tokio::try_join;

let (state, tasks, _) = try_join!(
    client.get_state(None),
    client.list_tasks(None),
    client.test_connection(),
)?;

println!("State: {:?}", state);
println!("Tasks: {} found", tasks.len());
```

## Troubleshooting

### Connection Refused

**Problem:** `ConnectionError: Failed to connect to Unix socket`

**Solution:**
1. Check if daemon is running: `ps aux | grep descartes-daemon`
2. Start daemon: `cargo run --bin descartes-daemon`
3. Check socket exists: `ls -la /tmp/descartes-rpc.sock`

### Permission Denied

**Problem:** `ConnectionError: Permission denied`

**Solution:**
```bash
# Fix socket permissions
chmod 600 /tmp/descartes-rpc.sock

# Or make it group-accessible
chmod 660 /tmp/descartes-rpc.sock
chgrp descartes /tmp/descartes-rpc.sock
```

### Timeout Errors

**Problem:** Requests timing out

**Solution:**
```rust
// Increase timeout
let client = UnixSocketRpcClientBuilder::new()
    .timeout(60)  // 60 seconds
    .build()?;
```

## API Reference

### Client Methods

| Method | Parameters | Returns | Description |
|--------|-----------|---------|-------------|
| `spawn` | `name: &str, agent_type: &str, config: Value` | `String` | Spawn new agent |
| `list_tasks` | `filter: Option<Value>` | `Vec<TaskInfo>` | List tasks with filter |
| `approve` | `task_id: &str, approved: bool` | `ApprovalResult` | Approve/reject task |
| `get_state` | `entity_id: Option<&str>` | `Value` | Get system or agent state |
| `test_connection` | - | `()` | Test connection to server |

### Data Structures

```rust
pub struct TaskInfo {
    pub id: String,
    pub name: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

pub struct ApprovalResult {
    pub task_id: String,
    pub approved: bool,
    pub timestamp: i64,
}
```

## Further Reading

- **Full Documentation:** [RPC_CLIENT_GUIDE.md](./RPC_CLIENT_GUIDE.md)
- **Unix Socket Details:** [UNIX_SOCKET_RPC.md](./UNIX_SOCKET_RPC.md)
- **Compatibility Report:** [PHASE3_1.3_RPC_COMPATIBILITY_REPORT.md](../../PHASE3_1.3_RPC_COMPATIBILITY_REPORT.md)
- **Examples:** See `examples/` directory
- **Tests:** See `tests/rpc_compatibility_test.rs`

## Support

For issues or questions:
1. Check the comprehensive guides above
2. Review example code in `examples/`
3. Run tests to verify your setup
4. Check server logs for errors
