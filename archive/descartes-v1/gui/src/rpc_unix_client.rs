/// Unix Socket RPC client integration for Descartes GUI
///
///This module provides a wrapper around the Unix socket RPC client for use in the Iced GUI.
/// It handles background communication with the daemon via Unix sockets.
use descartes_daemon::{DaemonError, UnixSocketRpcClient};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// GUI-specific RPC client wrapper for Unix sockets
pub struct GuiUnixRpcClient {
    client: Arc<UnixSocketRpcClient>,
    connected: Arc<RwLock<bool>>,
}

impl GuiUnixRpcClient {
    /// Create a new GUI Unix RPC client
    pub fn new(socket_path: PathBuf) -> Result<Self, DaemonError> {
        let client = UnixSocketRpcClient::new(socket_path)?;

        Ok(GuiUnixRpcClient {
            client: Arc::new(client),
            connected: Arc::new(RwLock::new(false)),
        })
    }

    /// Create a new client with default socket path (/tmp/descartes-rpc.sock)
    pub fn with_defaults() -> Result<Self, DaemonError> {
        Self::new(PathBuf::from("/tmp/descartes-rpc.sock"))
    }

    /// Test connection to the daemon
    pub async fn connect(&self) -> Result<(), DaemonError> {
        self.client.test_connection().await?;
        *self.connected.write().await = true;
        Ok(())
    }

    /// Check if connected
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }

    /// Get the underlying RPC client
    pub fn client(&self) -> Arc<UnixSocketRpcClient> {
        Arc::clone(&self.client)
    }

    /// Disconnect
    pub async fn disconnect(&self) {
        *self.connected.write().await = false;
    }

    /// Spawn a new agent
    pub async fn spawn_agent(
        &self,
        name: &str,
        agent_type: &str,
        config: Value,
    ) -> Result<String, DaemonError> {
        self.client.spawn(name, agent_type, config).await
    }

    /// List all tasks
    pub async fn list_tasks(
        &self,
        filter: Option<Value>,
    ) -> Result<Vec<descartes_daemon::TaskInfo>, DaemonError> {
        self.client.list_tasks(filter).await
    }

    /// Approve a task
    pub async fn approve_task(
        &self,
        task_id: &str,
        approved: bool,
    ) -> Result<descartes_daemon::ApprovalResult, DaemonError> {
        self.client.approve(task_id, approved).await
    }

    /// Get system or agent state
    pub async fn get_state(&self, entity_id: Option<&str>) -> Result<Value, DaemonError> {
        self.client.get_state(entity_id).await
    }
}

impl Clone for GuiUnixRpcClient {
    fn clone(&self) -> Self {
        GuiUnixRpcClient {
            client: Arc::clone(&self.client),
            connected: Arc::clone(&self.connected),
        }
    }
}

/// Example usage in Iced GUI
///
/// ```rust,no_run
/// use iced::{Application, Command, Element};
/// use serde_json::json;
///
/// struct DescartesApp {
///     rpc: GuiUnixRpcClient,
///     status: String,
/// }
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     Connect,
///     Connected(Result<(), String>),
///     SpawnAgent,
///     AgentSpawned(Result<String, String>),
///     ListTasks,
///     TasksReceived(Result<Vec<descartes_daemon::TaskInfo>, String>),
/// }
///
/// impl Application for DescartesApp {
///     type Message = Message;
///     type Executor = iced::executor::Default;
///     type Flags = ();
///     type Theme = iced::Theme;
///
///     fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
///         let rpc = GuiUnixRpcClient::with_defaults().unwrap();
///         (
///             DescartesApp {
///                 rpc,
///                 status: "Disconnected".to_string(),
///             },
///             Command::none(),
///         )
///     }
///
///     fn title(&self) -> String {
///         "Descartes GUI".to_string()
///     }
///
///     fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
///         match message {
///             Message::Connect => {
///                 let rpc = self.rpc.clone();
///                 Command::perform(
///                     async move {
///                         rpc.connect().await.map_err(|e| e.to_string())
///                     },
///                     Message::Connected,
///                 )
///             }
///             Message::Connected(result) => {
///                 match result {
///                     Ok(_) => {
///                         self.status = "Connected".to_string();
///                     }
///                     Err(e) => {
///                         self.status = format!("Connection failed: {}", e);
///                     }
///                 }
///                 Command::none()
///             }
///             Message::SpawnAgent => {
///                 let rpc = self.rpc.clone();
///                 Command::perform(
///                     async move {
///                         let config = json!({
///                             "task": "Write a hello world program",
///                             "environment": {}
///                         });
///                         rpc.spawn_agent("my-agent", "worker", config)
///                             .await
///                             .map_err(|e| e.to_string())
///                     },
///                     Message::AgentSpawned,
///                 )
///             }
///             Message::AgentSpawned(result) => {
///                 match result {
///                     Ok(agent_id) => {
///                         self.status = format!("Agent spawned: {}", agent_id);
///                     }
///                     Err(e) => {
///                         self.status = format!("Spawn failed: {}", e);
///                     }
///                 }
///                 Command::none()
///             }
///             Message::ListTasks => {
///                 let rpc = self.rpc.clone();
///                 Command::perform(
///                     async move {
///                         rpc.list_tasks(None).await.map_err(|e| e.to_string())
///                     },
///                     Message::TasksReceived,
///                 )
///             }
///             Message::TasksReceived(result) => {
///                 match result {
///                     Ok(tasks) => {
///                         self.status = format!("Found {} tasks", tasks.len());
///                     }
///                     Err(e) => {
///                         self.status = format!("Failed to list tasks: {}", e);
///                     }
///                 }
///                 Command::none()
///             }
///         }
///     }
///
///     fn view(&self) -> Element<Self::Message> {
///         // GUI implementation here
///         unimplemented!()
///     }
/// }
/// ```

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client() {
        let client = GuiUnixRpcClient::new(PathBuf::from("/tmp/test.sock"));
        assert!(client.is_ok());
    }

    #[test]
    fn test_default_client() {
        let client = GuiUnixRpcClient::with_defaults();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_initial_not_connected() {
        let client = GuiUnixRpcClient::with_defaults().unwrap();
        assert!(!client.is_connected().await);
    }

    #[tokio::test]
    async fn test_clone() {
        let client = GuiUnixRpcClient::with_defaults().unwrap();
        let client2 = client.clone();
        assert!(!client2.is_connected().await);
    }
}
