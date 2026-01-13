//! Minimal Tokio channel bridge for internal communication
//! ZMQ handles all agent communication; this is just for in-process coordination

use tokio::sync::mpsc;

/// Simple message for internal coordination
#[derive(Debug, Clone)]
pub struct InternalMessage {
    pub msg_type: String,
    pub payload: serde_json::Value,
}

/// Bridge between ZMQ and internal components
pub struct ChannelBridge {
    /// Sender for internal messages
    pub tx: mpsc::UnboundedSender<InternalMessage>,
    /// Receiver for internal messages
    pub rx: mpsc::UnboundedReceiver<InternalMessage>,
}

impl ChannelBridge {
    pub fn new() -> (mpsc::UnboundedSender<InternalMessage>, mpsc::UnboundedReceiver<InternalMessage>) {
        mpsc::unbounded_channel()
    }
}

impl Default for ChannelBridge {
    fn default() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }
}
