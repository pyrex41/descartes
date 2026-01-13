//! GUI RPC Integration Example
//!
//! This example demonstrates how a GUI application can integrate with
//! the Descartes RPC server via Unix sockets.
//!
//! Usage:
//!   1. Start the RPC server: cargo run --bin descartes-daemon
//!   2. Run this example: cargo run --example gui_rpc_integration

use descartes_daemon::{TaskInfo, UnixSocketRpcClient};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{sleep, Duration};

/// Simulated GUI state
struct GuiState {
    client: Arc<UnixSocketRpcClient>,
    connected: Arc<RwLock<bool>>,
    tasks: Arc<RwLock<Vec<TaskInfo>>>,
    system_status: Arc<RwLock<String>>,
}

impl GuiState {
    fn new(client: UnixSocketRpcClient) -> Self {
        Self {
            client: Arc::new(client),
            connected: Arc::new(RwLock::new(false)),
            tasks: Arc::new(RwLock::new(Vec::new())),
            system_status: Arc::new(RwLock::new("Disconnected".to_string())),
        }
    }

    /// Connect to the daemon
    async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.client.test_connection().await?;
        *self.connected.write().await = true;
        *self.system_status.write().await = "Connected".to_string();
        Ok(())
    }

    /// Refresh task list (simulates GUI polling)
    async fn refresh_tasks(&self) -> Result<(), Box<dyn std::error::Error>> {
        let tasks = self.client.list_tasks(None).await?;
        *self.tasks.write().await = tasks;
        Ok(())
    }

    /// Spawn agent from GUI form
    async fn spawn_agent_from_form(
        &self,
        name: &str,
        agent_type: &str,
        task: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let config = json!({
            "task": task,
            "environment": {}
        });
        let agent_id = self.client.spawn(name, agent_type, config).await?;
        Ok(agent_id)
    }

    /// Handle task approval from GUI button
    async fn handle_task_approval(
        &self,
        task_id: &str,
        approved: bool,
    ) -> Result<(), Box<dyn std::error::Error>> {
        self.client.approve(task_id, approved).await?;
        // Refresh task list after approval
        self.refresh_tasks().await?;
        Ok(())
    }

    /// Update status display
    async fn update_status(&self) -> Result<(), Box<dyn std::error::Error>> {
        let state = self.client.get_state(None).await?;
        let status = format!(
            "Agents: {} | Tasks: {}",
            state["agents"]["total"], state["tasks"]["total"]
        );
        *self.system_status.write().await = status;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("=== GUI RPC Integration Example ===\n");

    // Create RPC client
    let socket_path = PathBuf::from("/tmp/descartes-rpc.sock");
    let client = UnixSocketRpcClient::new(socket_path)?;
    let gui_state = GuiState::new(client);

    println!("Simulating GUI application workflow...\n");

    // Step 1: Connect to daemon
    println!("1. Connecting to daemon...");
    match gui_state.connect().await {
        Ok(_) => {
            let status = gui_state.system_status.read().await;
            println!("   Status: {}\n", status);
        }
        Err(e) => {
            eprintln!("   Failed to connect: {}", e);
            eprintln!("\n   Make sure the RPC server is running:");
            eprintln!("     cargo run --bin descartes-daemon\n");
            return Err(e);
        }
    }

    // Step 2: Load initial data
    println!("2. Loading initial task list...");
    gui_state.refresh_tasks().await?;
    let tasks = gui_state.tasks.read().await;
    println!("   Loaded {} tasks\n", tasks.len());
    drop(tasks);

    // Step 3: Update status display
    println!("3. Updating status display...");
    gui_state.update_status().await?;
    let status = gui_state.system_status.read().await;
    println!("   {}\n", status);
    drop(status);

    // Step 4: Simulate user spawning an agent from GUI form
    println!("4. User clicks 'Spawn Agent' button...");
    println!("   Form input:");
    println!("     Name: gui-test-agent");
    println!("     Type: worker");
    println!("     Task: Create a test file");

    match gui_state
        .spawn_agent_from_form("gui-test-agent", "worker", "Create a test file")
        .await
    {
        Ok(agent_id) => {
            println!("   ✓ Agent spawned: {}\n", agent_id);
        }
        Err(e) => {
            println!("   ✗ Failed to spawn: {}\n", e);
        }
    }

    // Step 5: Simulate periodic refresh (GUI polling loop)
    println!("5. Simulating GUI refresh loop (3 iterations)...");
    for i in 1..=3 {
        sleep(Duration::from_millis(500)).await;

        gui_state.refresh_tasks().await?;
        gui_state.update_status().await?;

        let tasks = gui_state.tasks.read().await;
        let status = gui_state.system_status.read().await;

        println!("   Refresh {}: {} | {} tasks", i, status, tasks.len());

        drop(tasks);
        drop(status);
    }
    println!();

    // Step 6: Simulate user filtering tasks
    println!("6. User selects 'Show TODO tasks only' filter...");
    let filter = json!({ "status": "todo" });
    match gui_state.client.list_tasks(Some(filter)).await {
        Ok(filtered_tasks) => {
            println!("   Showing {} TODO tasks:", filtered_tasks.len());
            for task in filtered_tasks.iter().take(3) {
                println!("     - {}: {}", task.id, task.name);
            }
            println!();
        }
        Err(e) => {
            println!("   Failed to filter: {}\n", e);
        }
    }

    // Step 7: Simulate task approval interaction
    println!("7. User approves/rejects tasks (simulation)...");
    println!("   (Skipped - requires actual pending tasks)");
    println!("   Usage: gui_state.handle_task_approval(task_id, true).await\n");

    // Step 8: Show how GUI handles errors
    println!("8. GUI error handling demonstration...");
    println!("   User attempts to approve invalid task...");
    match gui_state.handle_task_approval("invalid-id", true).await {
        Ok(_) => println!("   Unexpected success"),
        Err(e) => {
            println!("   ✓ Error caught and displayed to user:");
            println!("     \"{}\"", e);
        }
    }
    println!();

    println!("=== GUI Integration Examples Complete ===\n");
    println!("Key GUI Integration Patterns:");
    println!("  • Use Arc<RpcClient> for shared state across GUI");
    println!("  • Wrap network calls in async commands (Iced Command::perform)");
    println!("  • Handle errors gracefully with user-friendly messages");
    println!("  • Poll periodically or use event subscriptions for updates");
    println!("  • Maintain local state cache for better UX");
    println!();

    Ok(())
}
