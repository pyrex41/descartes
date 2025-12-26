//! Swank client for communicating with SBCL.

use crate::swank::codec::SwankCodec;
use futures::{SinkExt, StreamExt};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use thiserror::Error;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, RwLock};
use tokio_util::codec::Framed;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum SwankError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Send failed: {0}")]
    SendFailed(String),
    #[error("Receive failed: {0}")]
    ReceiveFailed(String),
    #[error("Parse error: {0}")]
    ParseError(String),
    #[error("Timeout waiting for response")]
    Timeout,
    #[error("Client disconnected")]
    Disconnected,
}

/// Async event from Swank (debugger, output, etc.)
#[derive(Debug, Clone)]
pub enum SwankMessage {
    /// Debugger was entered
    Debug {
        thread: i64,
        level: i64,
        condition: String,
        restarts: Vec<SwankRestart>,
        frames: Vec<SwankFrame>,
    },
    /// Output was written
    WriteString(String),
    /// Return value from evaluation
    Return { id: u64, value: String },
    /// Evaluation aborted
    Abort { id: u64, reason: String },
}

#[derive(Debug, Clone)]
pub struct SwankRestart {
    pub index: usize,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct SwankFrame {
    pub index: usize,
    pub description: String,
}

/// Client for communicating with a Swank server.
pub struct SwankClient {
    /// Agent ID this client is associated with
    agent_id: Uuid,
    /// Port the Swank server is running on
    port: u16,
    /// Channel to send messages to the write loop
    write_tx: mpsc::Sender<String>,
    /// Pending request callbacks
    pending: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<String, SwankError>>>>>,
    /// Channel for async events (debug, output)
    #[allow(dead_code)]
    event_tx: mpsc::Sender<SwankMessage>,
    /// Counter for request IDs
    next_id: AtomicU64,
    /// Whether the client is connected
    connected: Arc<RwLock<bool>>,
}

impl SwankClient {
    /// Connect to a Swank server.
    pub async fn connect(
        agent_id: Uuid,
        port: u16,
        event_tx: mpsc::Sender<SwankMessage>,
    ) -> Result<Arc<Self>, SwankError> {
        let addr = format!("127.0.0.1:{}", port);
        info!("Connecting to Swank server at {}", addr);

        let stream = TcpStream::connect(&addr)
            .await
            .map_err(|e| SwankError::ConnectionFailed(e.to_string()))?;

        let framed = Framed::new(stream, SwankCodec);
        let (mut sink, mut stream) = framed.split();

        let (write_tx, mut write_rx) = mpsc::channel::<String>(32);
        let pending: Arc<RwLock<HashMap<u64, oneshot::Sender<Result<String, SwankError>>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let connected = Arc::new(RwLock::new(true));

        let client = Arc::new(Self {
            agent_id,
            port,
            write_tx,
            pending: Arc::clone(&pending),
            event_tx: event_tx.clone(),
            next_id: AtomicU64::new(1),
            connected: Arc::clone(&connected),
        });

        // Spawn write loop
        let connected_write = Arc::clone(&connected);
        tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                if let Err(e) = sink.send(msg).await {
                    error!("Swank write error: {}", e);
                    *connected_write.write().await = false;
                    break;
                }
            }
        });

        // Spawn read loop
        let pending_read = Arc::clone(&pending);
        let connected_read = Arc::clone(&connected);
        let event_tx_read = event_tx;
        tokio::spawn(async move {
            while let Some(result) = stream.next().await {
                match result {
                    Ok(msg) => {
                        Self::handle_message(&msg, &pending_read, &event_tx_read).await;
                    }
                    Err(e) => {
                        error!("Swank read error: {}", e);
                        *connected_read.write().await = false;
                        break;
                    }
                }
            }
            info!("Swank read loop terminated");
        });

        // Send initial connection info request
        client
            .send_raw("(:emacs-rex (swank:connection-info) \"CL-USER\" t 0)")
            .await?;

        Ok(client)
    }

    /// Handle an incoming message from Swank.
    async fn handle_message(
        msg: &str,
        pending: &Arc<RwLock<HashMap<u64, oneshot::Sender<Result<String, SwankError>>>>>,
        event_tx: &mpsc::Sender<SwankMessage>,
    ) {
        debug!("Received Swank message: {}", msg);

        if msg.starts_with("(:return") {
            // Parse return message: (:return (:ok "value") id)
            if let Some(id) = Self::extract_return_id(msg) {
                let value = Self::extract_return_value(msg);
                let mut pending_guard = pending.write().await;
                if let Some(tx) = pending_guard.remove(&id) {
                    let _ = tx.send(Ok(value));
                }
            }
        } else if msg.starts_with("(:debug") {
            // Parse debug message
            if let Some(debug_msg) = Self::parse_debug_message(msg) {
                let _ = event_tx.send(debug_msg).await;
            }
        } else if msg.starts_with("(:write-string") {
            // Parse output: (:write-string "text")
            if let Some(text) = Self::extract_write_string(msg) {
                let _ = event_tx.send(SwankMessage::WriteString(text)).await;
            }
        } else if msg.starts_with("(:debug-activate") {
            // Debug activation - already sent with :debug
            debug!("Debug activated");
        } else if msg.starts_with("(:debug-return") {
            // Debug returned - debugger exited
            debug!("Debug returned");
        } else if msg.starts_with("(:indentation-update") {
            // Ignore indentation updates
        } else if msg.starts_with("(:new-features") {
            // Ignore feature updates
        } else {
            warn!("Unhandled Swank message type: {}", &msg[..msg.len().min(50)]);
        }
    }

    fn extract_return_id(msg: &str) -> Option<u64> {
        // Find the last number in the message (the ID)
        // Format: (:return (:ok ...) ID) or (:return (:abort ...) ID)
        let trimmed = msg.trim_end_matches(')');
        trimmed
            .rsplit_once(' ')
            .and_then(|(_, id_str)| id_str.trim().parse().ok())
    }

    fn extract_return_value(msg: &str) -> String {
        // Extract value between (:ok and the closing paren before ID
        // This is a simplified parser
        if let Some(start) = msg.find("(:ok ") {
            let rest = &msg[start + 5..];
            // Find matching paren - this is simplified
            if let Some(end) = rest.rfind(')') {
                let value = &rest[..end];
                // Remove outer quotes if present
                return value.trim().trim_matches('"').to_string();
            }
        }
        if let Some(start) = msg.find("(:abort ") {
            let rest = &msg[start + 8..];
            if let Some(end) = rest.rfind(')') {
                return format!("ABORT: {}", rest[..end].trim().trim_matches('"'));
            }
        }
        msg.to_string()
    }

    fn extract_write_string(msg: &str) -> Option<String> {
        // (:write-string "text" :repl-result) or (:write-string "text")
        let start = msg.find('"')?;
        let rest = &msg[start + 1..];
        // Find the closing quote, handling escapes
        let mut end = 0;
        let mut escaped = false;
        for (i, c) in rest.chars().enumerate() {
            if escaped {
                escaped = false;
                continue;
            }
            if c == '\\' {
                escaped = true;
                continue;
            }
            if c == '"' {
                end = i;
                break;
            }
        }
        if end > 0 {
            let text = &rest[..end];
            // Unescape common sequences
            let unescaped = text
                .replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\\"", "\"")
                .replace("\\\\", "\\");
            Some(unescaped)
        } else {
            None
        }
    }

    fn parse_debug_message(msg: &str) -> Option<SwankMessage> {
        // Full format: (:debug thread level (condition restarts...) ((restart-name restart-desc)...) (frames...) ...)
        // Use lexpr to parse the S-expression

        if let Ok(sexp) = lexpr::from_str(msg) {
            // Convert to vector for easier indexing
            let items = Self::sexp_to_vec(&sexp);
            if items.len() >= 5 {
                let thread = items.get(1).and_then(|v| Self::sexp_as_i64(v)).unwrap_or(0);
                let level = items.get(2).and_then(|v| Self::sexp_as_i64(v)).unwrap_or(1);

                // Extract condition from the condition list (index 3)
                let condition = items.get(3)
                    .and_then(|v| {
                        let cond_items = Self::sexp_to_vec(v);
                        cond_items.first().and_then(|first| Self::sexp_as_str(first))
                    })
                    .unwrap_or_else(|| "Unknown error condition".to_string());

                // Extract restarts (index 4)
                let restarts = items.get(4)
                    .map(|v| {
                        Self::sexp_to_vec(v).iter().enumerate().filter_map(|(idx, r)| {
                            let r_items = Self::sexp_to_vec(r);
                            let name = r_items.first().and_then(|v| Self::sexp_as_str(v)).unwrap_or_else(|| "UNKNOWN".to_string());
                            let desc = r_items.get(1).and_then(|v| Self::sexp_as_str(v)).unwrap_or_default();
                            Some(SwankRestart {
                                index: idx,
                                name,
                                description: desc,
                            })
                        }).collect()
                    })
                    .unwrap_or_default();

                // Extract frames (index 5)
                let frames = items.get(5)
                    .map(|v| {
                        Self::sexp_to_vec(v).iter().enumerate().filter_map(|(idx, f)| {
                            let f_items = Self::sexp_to_vec(f);
                            // Frame format: (index description ...)
                            let desc = f_items.get(1).and_then(|v| Self::sexp_as_str(v)).unwrap_or_else(|| "???".to_string());
                            Some(SwankFrame {
                                index: idx,
                                description: desc,
                            })
                        }).collect()
                    })
                    .unwrap_or_default();

                return Some(SwankMessage::Debug {
                    thread,
                    level,
                    condition,
                    restarts,
                    frames,
                });
            }
        }

        // Fallback: simplified parsing
        Some(SwankMessage::Debug {
            thread: 0,
            level: 1,
            condition: "Debug condition (parse pending)".to_string(),
            restarts: vec![SwankRestart {
                index: 0,
                name: "ABORT".to_string(),
                description: "Abort evaluation".to_string(),
            }],
            frames: vec![],
        })
    }

    /// Convert an S-expression to a vector of values.
    fn sexp_to_vec(sexp: &lexpr::Value) -> Vec<lexpr::Value> {
        match sexp {
            lexpr::Value::Cons(cons) => {
                // Iterate over cons cells, collecting car values
                let mut result = Vec::new();
                for cell in cons.iter() {
                    result.push(lexpr::Value::Cons(cell.clone()));
                }
                // Actually, we want the list elements - use a different approach
                // Cons cells can be iterated as a list
                result.clear();
                let mut current = Some(cons);
                while let Some(c) = current {
                    result.push(c.car().clone());
                    match c.cdr() {
                        lexpr::Value::Cons(next) => current = Some(next),
                        _ => break,
                    }
                }
                result
            }
            lexpr::Value::Vector(vec) => vec.iter().cloned().collect(),
            _ => vec![],
        }
    }

    /// Extract i64 from S-expression.
    fn sexp_as_i64(sexp: &lexpr::Value) -> Option<i64> {
        match sexp {
            lexpr::Value::Number(n) => n.as_i64(),
            _ => None,
        }
    }

    /// Extract string from S-expression.
    fn sexp_as_str(sexp: &lexpr::Value) -> Option<String> {
        match sexp {
            lexpr::Value::String(s) => Some(s.to_string()),
            lexpr::Value::Symbol(s) => Some(s.to_string()),
            _ => None,
        }
    }

    /// Send a raw message to Swank.
    async fn send_raw(&self, msg: &str) -> Result<(), SwankError> {
        if !*self.connected.read().await {
            return Err(SwankError::Disconnected);
        }
        self.write_tx
            .send(msg.to_string())
            .await
            .map_err(|e| SwankError::SendFailed(e.to_string()))
    }

    /// Evaluate code in the Lisp runtime.
    pub async fn eval(&self, code: &str, package: &str) -> Result<String, SwankError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let escaped = code.replace('\\', "\\\\").replace('"', "\\\"");
        let msg = format!(
            "(:emacs-rex (swank:eval-and-grab-output \"{}\") \"{}\" t {})",
            escaped, package, id
        );

        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(id, tx);

        self.send_raw(&msg).await?;

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| SwankError::Timeout)?
            .map_err(|_| SwankError::Disconnected)?
    }

    /// Compile a code string.
    pub async fn compile_string(&self, code: &str) -> Result<String, SwankError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let escaped = code.replace('\\', "\\\\").replace('"', "\\\"");
        let msg = format!(
            "(:emacs-rex (swank:compile-string-for-emacs \"{}\" \"repl\" '((:position 1) (:line 1 1))) \"CL-USER\" t {})",
            escaped, id
        );

        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(id, tx);

        self.send_raw(&msg).await?;

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| SwankError::Timeout)?
            .map_err(|_| SwankError::Disconnected)?
    }

    /// Inspect an expression.
    pub async fn inspect(&self, expr: &str) -> Result<String, SwankError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let escaped = expr.replace('\\', "\\\\").replace('"', "\\\"");
        let msg = format!(
            "(:emacs-rex (swank:init-inspector \"{}\") \"CL-USER\" t {})",
            escaped, id
        );

        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(id, tx);

        self.send_raw(&msg).await?;

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| SwankError::Timeout)?
            .map_err(|_| SwankError::Disconnected)?
    }

    /// Invoke a debugger restart.
    pub async fn invoke_restart(&self, restart_index: usize) -> Result<String, SwankError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let msg = format!(
            "(:emacs-rex (swank:invoke-nth-restart-for-emacs 1 {}) \"CL-USER\" t {})",
            restart_index, id
        );

        let (tx, rx) = oneshot::channel();
        self.pending.write().await.insert(id, tx);

        self.send_raw(&msg).await?;

        tokio::time::timeout(std::time::Duration::from_secs(30), rx)
            .await
            .map_err(|_| SwankError::Timeout)?
            .map_err(|_| SwankError::Disconnected)?
    }

    /// Disconnect from the Swank server.
    pub async fn disconnect(&self) -> Result<(), SwankError> {
        *self.connected.write().await = false;
        Ok(())
    }

    /// Get the agent ID.
    pub fn agent_id(&self) -> Uuid {
        self.agent_id
    }

    /// Get the port.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Check if connected.
    pub async fn is_connected(&self) -> bool {
        *self.connected.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_return_id() {
        assert_eq!(
            SwankClient::extract_return_id("(:return (:ok \"3\") 5)"),
            Some(5)
        );
        assert_eq!(
            SwankClient::extract_return_id("(:return (:abort \"error\") 42)"),
            Some(42)
        );
    }

    #[test]
    fn test_extract_return_value() {
        let value = SwankClient::extract_return_value("(:return (:ok \"hello world\") 1)");
        assert!(value.contains("hello world"));
    }

    #[test]
    fn test_extract_write_string() {
        assert_eq!(
            SwankClient::extract_write_string("(:write-string \"hello\")"),
            Some("hello".to_string())
        );
        assert_eq!(
            SwankClient::extract_write_string("(:write-string \"hello\\nworld\")"),
            Some("hello\nworld".to_string())
        );
        assert_eq!(
            SwankClient::extract_write_string("(:write-string \"test\\\"quoted\\\"\")"),
            Some("test\"quoted\"".to_string())
        );
    }
}
