/// Descartes GUI - Native cross-platform interface using Iced
/// Phase 3 implementation

pub mod rpc_client;
pub mod event_handler;

pub use rpc_client::GuiRpcClient;
pub use event_handler::EventHandler;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Placeholder for Iced GUI implementation
pub struct DescarterGui;

impl DescarterGui {
    /// Create a new GUI instance
    pub fn new() -> Self {
        Self
    }
}

impl Default for DescarterGui {
    fn default() -> Self {
        Self::new()
    }
}
