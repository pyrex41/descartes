/// RPC client integration for Descartes GUI
///
/// This module provides a wrapper around the RPC client for use in the Iced GUI.
/// It handles background communication with the daemon and provides a message-based interface.

use descartes_daemon::{RpcClient, RpcClientBuilder, DaemonError};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    pub fn default() -> Result<Self, DaemonError> {
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
}

/// Example usage in Iced GUI
///
/// ```rust,no_run
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
///         let rpc = GuiRpcClient::default().unwrap();
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
        let client = GuiRpcClient::default();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_initial_not_connected() {
        let client = GuiRpcClient::default().unwrap();
        assert!(!client.is_connected().await);
    }
}
