/// RPC client integration for Descartes GUI
///
/// This module provides a wrapper around the RPC client for use in the Iced GUI.
/// It handles background communication with the daemon and provides a message-based interface.
use descartes_daemon::{DaemonError, RpcClient, RpcClientBuilder};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// GUI RPC client wrapper
pub struct GuiRpcClient {
    client: Arc<RpcClient>,
    connected: Arc<RwLock<bool>>,
}

impl GuiRpcClient {
    /// Create a new GUI RPC client
    pub fn new(url: &str) -> Result<Self, DaemonError> {
        let client = RpcClientBuilder::new()
            .url(url)
            .timeout(30)
            .max_retries(3)
            .pool_size(10)
            .build()?;

        Ok(GuiRpcClient {
            client: Arc::new(client),
            connected: Arc::new(RwLock::new(false)),
        })
    }

    /// Create a new client with default settings
    pub fn with_defaults() -> Result<Self, DaemonError> {
        Self::new("http://127.0.0.1:8080")
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
    pub fn client(&self) -> Arc<RpcClient> {
        Arc::clone(&self.client)
    }

    /// Disconnect
    pub async fn disconnect(&self) {
        *self.connected.write().await = false;
    }

    /// Pause a running agent
    ///
    /// # Arguments
    /// * `agent_id` - The ID of the agent to pause
    /// * `force` - If true, use SIGSTOP (forced); otherwise use cooperative pause
    ///
    /// # Returns
    /// Pause confirmation with timestamp and mode
    pub async fn pause_agent(&self, agent_id: Uuid, force: bool) -> Result<PauseResult, DaemonError> {
        let params = json!([agent_id.to_string(), force]);
        let result = self.client.call("agent.pause", Some(params)).await?;

        serde_json::from_value(result)
            .map_err(|e| DaemonError::SerializationError(format!("Failed to parse pause result: {}", e)))
    }

    /// Resume a paused agent
    ///
    /// # Arguments
    /// * `agent_id` - The ID of the agent to resume
    ///
    /// # Returns
    /// Resume confirmation with timestamp
    pub async fn resume_agent(&self, agent_id: Uuid) -> Result<ResumeResult, DaemonError> {
        let params = json!([agent_id.to_string()]);
        let result = self.client.call("agent.resume", Some(params)).await?;

        serde_json::from_value(result)
            .map_err(|e| DaemonError::SerializationError(format!("Failed to parse resume result: {}", e)))
    }

    /// Request attach credentials for a paused agent
    ///
    /// # Arguments
    /// * `agent_id` - The ID of the agent to attach to
    /// * `client_type` - The type of client requesting attachment (e.g., "claude-code")
    ///
    /// # Returns
    /// Attach credentials including token and connect URL
    pub async fn attach_request(&self, agent_id: Uuid, client_type: &str) -> Result<AttachCredentialsResult, DaemonError> {
        let params = json!([agent_id.to_string(), client_type]);
        let result = self.client.call("agent.attach.request", Some(params)).await?;

        serde_json::from_value(result)
            .map_err(|e| DaemonError::SerializationError(format!("Failed to parse attach credentials result: {}", e)))
    }
}

/// Example usage in Iced GUI
///
/// ```rust,ignore
/// use iced::{Application, Command, Element};
///
/// struct DescartesApp {
///     rpc: GuiRpcClient,
///     status: String,
/// }
///
/// #[derive(Debug, Clone)]
/// enum Message {
///     Connect,
///     Connected(Result<(), String>),
///     CheckHealth,
///     HealthReceived(Result<String, String>),
/// }
///
/// impl Application for DescartesApp {
///     type Message = Message;
///     type Executor = iced::executor::Default;
///     type Flags = ();
///     type Theme = iced::Theme;
///
///     fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
///         let rpc = GuiRpcClient::with_defaults().unwrap();
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
///             Message::CheckHealth => {
///                 let client = self.rpc.client();
///                 Command::perform(
///                     async move {
///                         client.health().await
///                             .map(|h| h.status)
///                             .map_err(|e| e.to_string())
///                     },
///                     Message::HealthReceived,
///                 )
///             }
///             Message::HealthReceived(result) => {
///                 match result {
///                     Ok(status) => {
///                         self.status = format!("Health: {}", status);
///                     }
///                     Err(e) => {
///                         self.status = format!("Health check failed: {}", e);
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
/// Pause result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PauseResult {
    pub agent_id: String,
    pub paused_at: i64,
    pub pause_mode: String,
}

/// Resume result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeResult {
    pub agent_id: String,
    pub resumed_at: i64,
}

/// Attach credentials result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachCredentialsResult {
    pub agent_id: String,
    pub token: String,
    pub connect_url: String,
    pub expires_at: i64,
}

impl Clone for GuiRpcClient {
    fn clone(&self) -> Self {
        GuiRpcClient {
            client: Arc::clone(&self.client),
            connected: Arc::clone(&self.connected),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_client() {
        let client = GuiRpcClient::new("http://localhost:8080");
        assert!(client.is_ok());
    }

    #[test]
    fn test_default_client() {
        let client = GuiRpcClient::with_defaults();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_initial_not_connected() {
        let client = GuiRpcClient::with_defaults().unwrap();
        assert!(!client.is_connected().await);
    }
}
