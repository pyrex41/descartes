//! OpenCode TUI Attachment Handler
//!
//! This module implements server-side handling for OpenCode TUI attachments.
//! OpenCode is an alternative TUI that can attach to paused Descartes agents.
//!
//! The protocol is similar to Claude Code but may have specific extensions:
//! - Different client identification
//! - Potential additional capabilities
//! - Custom message types for OpenCode features

use crate::attach_session::{AttachSessionInfo, AttachSessionManager, ClientType};
use crate::claude_code_tui::{ClaudeCodeTuiConfig, OutputBuffer};
use crate::errors::{DaemonError, DaemonResult};
use descartes_core::attach_protocol::{
    AttachHandshake, AttachHandshakeResponse, AttachMessage, AttachMessageType, OutputData,
    StdinData, ATTACH_PROTOCOL_VERSION,
};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// OpenCode-specific configuration
#[derive(Debug, Clone)]
pub struct OpenCodeTuiConfig {
    /// Base configuration (shared with Claude Code)
    pub base: ClaudeCodeTuiConfig,
    /// OpenCode-specific features enabled
    pub enable_extended_protocol: bool,
    /// Allow raw terminal mode
    pub allow_raw_mode: bool,
}

impl Default for OpenCodeTuiConfig {
    fn default() -> Self {
        Self {
            base: ClaudeCodeTuiConfig::default(),
            enable_extended_protocol: true,
            allow_raw_mode: true,
        }
    }
}

impl OpenCodeTuiConfig {
    /// Create config with custom base settings
    pub fn with_base(base: ClaudeCodeTuiConfig) -> Self {
        Self {
            base,
            enable_extended_protocol: true,
            allow_raw_mode: true,
        }
    }
}

/// OpenCode TUI connection handler
pub struct OpenCodeTuiHandler {
    /// Configuration
    config: OpenCodeTuiConfig,
    /// Attach session manager reference
    session_manager: Arc<AttachSessionManager>,
    /// Agent ID this handler is for
    agent_id: Uuid,
    /// Agent name
    agent_name: String,
    /// Agent task
    agent_task: String,
    /// Output buffer for history
    output_buffer: Arc<RwLock<OutputBuffer>>,
    /// Channel for sending stdin to agent
    stdin_tx: mpsc::Sender<Vec<u8>>,
    /// Broadcast channel for stdout from agent
    stdout_rx: broadcast::Receiver<Vec<u8>>,
    /// Broadcast channel for stderr from agent
    stderr_rx: broadcast::Receiver<Vec<u8>>,
}

impl OpenCodeTuiHandler {
    /// Create a new OpenCode TUI handler
    pub fn new(
        config: OpenCodeTuiConfig,
        session_manager: Arc<AttachSessionManager>,
        agent_id: Uuid,
        agent_name: String,
        agent_task: String,
        stdin_tx: mpsc::Sender<Vec<u8>>,
        stdout_rx: broadcast::Receiver<Vec<u8>>,
        stderr_rx: broadcast::Receiver<Vec<u8>>,
    ) -> Self {
        let output_buffer = Arc::new(RwLock::new(OutputBuffer::new(
            config.base.max_history_bytes,
            config.base.max_history_lines,
        )));

        Self {
            config,
            session_manager,
            agent_id,
            agent_name,
            agent_task,
            output_buffer,
            stdin_tx,
            stdout_rx,
            stderr_rx,
        }
    }

    /// Get a reference to the output buffer
    pub fn output_buffer(&self) -> Arc<RwLock<OutputBuffer>> {
        Arc::clone(&self.output_buffer)
    }

    /// Handle a new Unix socket connection from OpenCode
    pub async fn handle_connection(&mut self, stream: UnixStream) -> DaemonResult<()> {
        let (read_half, write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        let mut writer = write_half;

        // Perform handshake (same protocol as Claude Code)
        let session_info = self.perform_handshake(&mut reader, &mut writer).await?;
        info!(
            "OpenCode TUI handshake successful for agent {}",
            self.agent_id
        );

        // Send historical output
        self.send_historical_output(&mut writer).await?;
        info!("Historical output sent to OpenCode client");

        // Start IO forwarding loop
        self.run_io_loop(&mut reader, &mut writer, session_info)
            .await?;

        Ok(())
    }

    /// Perform the protocol handshake
    async fn perform_handshake<R, W>(
        &self,
        reader: &mut BufReader<R>,
        writer: &mut W,
    ) -> DaemonResult<AttachSessionInfo>
    where
        R: AsyncReadExt + Unpin,
        W: AsyncWriteExt + Unpin,
    {
        let handshake_timeout = Duration::from_secs(self.config.base.connection_timeout_secs);
        let handshake_msg = match timeout(handshake_timeout, self.read_message(reader)).await {
            Ok(Ok(msg)) => msg,
            Ok(Err(e)) => {
                let response = AttachHandshakeResponse::failure(&format!("Read error: {}", e));
                self.send_message(writer, &response.to_message()).await?;
                return Err(e);
            }
            Err(_) => {
                let response = AttachHandshakeResponse::failure("Handshake timeout");
                self.send_message(writer, &response.to_message()).await?;
                return Err(DaemonError::Timeout);
            }
        };

        if handshake_msg.msg_type != AttachMessageType::Handshake {
            let response = AttachHandshakeResponse::failure("Expected handshake message");
            self.send_message(writer, &response.to_message()).await?;
            return Err(DaemonError::AttachError(
                "Invalid message type for handshake".to_string(),
            ));
        }

        let handshake: AttachHandshake = serde_json::from_value(handshake_msg.payload)
            .map_err(|e| DaemonError::AttachError(format!("Invalid handshake payload: {}", e)))?;

        // Verify this is actually an OpenCode client
        if handshake.client_type != "opencode" {
            warn!(
                "OpenCode handler received non-OpenCode client: {}",
                handshake.client_type
            );
            // We still allow it but log the mismatch
        }

        if handshake.version != ATTACH_PROTOCOL_VERSION {
            let response = AttachHandshakeResponse::failure(&format!(
                "Protocol version mismatch: expected {}, got {}",
                ATTACH_PROTOCOL_VERSION, handshake.version
            ));
            self.send_message(writer, &response.to_message()).await?;
            return Err(DaemonError::AttachError("Protocol version mismatch".to_string()));
        }

        let session_info = self
            .session_manager
            .validate_token(&handshake.token)
            .await
            .ok_or_else(|| {
                DaemonError::AuthenticationFailed("Invalid or expired token".to_string())
            })?;

        if session_info.agent_id != self.agent_id {
            let response = AttachHandshakeResponse::failure("Token not valid for this agent");
            self.send_message(writer, &response.to_message()).await?;
            return Err(DaemonError::AuthenticationFailed(
                "Token not valid for this agent".to_string(),
            ));
        }

        // Build response with OpenCode-specific capabilities
        let buffer = self.output_buffer.read().await;
        let mut response = AttachHandshakeResponse::success(
            self.agent_id.to_string(),
            self.agent_name.clone(),
            self.agent_task.clone(),
            buffer.total_lines(),
        );
        drop(buffer);

        // Add OpenCode-specific capabilities if enabled
        if self.config.enable_extended_protocol {
            response.server_capabilities.push("opencode_extended".to_string());
        }
        if self.config.allow_raw_mode {
            response.server_capabilities.push("raw_terminal".to_string());
        }

        self.send_message(writer, &response.to_message()).await?;

        Ok(session_info)
    }

    /// Send historical output to the client
    async fn send_historical_output<W>(&self, writer: &mut W) -> DaemonResult<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        let buffer = self.output_buffer.read().await;
        let history = buffer.to_historical_output();
        drop(buffer);

        let msg = history.to_message();
        self.send_message(writer, &msg).await
    }

    /// Run the main IO forwarding loop
    async fn run_io_loop<R, W>(
        &mut self,
        reader: &mut BufReader<R>,
        writer: &mut W,
        _session_info: AttachSessionInfo,
    ) -> DaemonResult<()>
    where
        R: AsyncReadExt + Unpin,
        W: AsyncWriteExt + Unpin,
    {
        let ping_interval = Duration::from_secs(self.config.base.ping_interval_secs);
        let mut ping_timer = tokio::time::interval(ping_interval);
        let mut seq: u64 = 0;

        loop {
            tokio::select! {
                result = self.read_message(reader) => {
                    match result {
                        Ok(msg) => {
                            if !self.handle_client_message(msg, writer).await? {
                                info!("OpenCode client disconnected gracefully");
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Error reading from OpenCode client: {}", e);
                            break;
                        }
                    }
                }

                result = self.stdout_rx.recv() => {
                    match result {
                        Ok(data) => {
                            {
                                let mut buffer = self.output_buffer.write().await;
                                buffer.push_stdout(data.clone());
                            }

                            let output_data = OutputData::from_bytes(&data);
                            let msg = output_data.to_stdout_message();
                            if let Err(e) = self.send_message(writer, &msg).await {
                                warn!("Error sending stdout to OpenCode client: {}", e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("OpenCode client lagged, missed {} stdout messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Agent stdout channel closed");
                            break;
                        }
                    }
                }

                result = self.stderr_rx.recv() => {
                    match result {
                        Ok(data) => {
                            {
                                let mut buffer = self.output_buffer.write().await;
                                buffer.push_stderr(data.clone());
                            }

                            let output_data = OutputData::from_bytes(&data);
                            let msg = output_data.to_stderr_message();
                            if let Err(e) = self.send_message(writer, &msg).await {
                                warn!("Error sending stderr to OpenCode client: {}", e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("OpenCode client lagged, missed {} stderr messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Agent stderr channel closed");
                            break;
                        }
                    }
                }

                _ = ping_timer.tick() => {
                    seq += 1;
                    let ping_msg = AttachMessage::with_seq(
                        AttachMessageType::Ping,
                        serde_json::json!({}),
                        seq,
                    );
                    if let Err(e) = self.send_message(writer, &ping_msg).await {
                        warn!("Error sending ping to OpenCode: {}", e);
                        break;
                    }
                    debug!("Sent ping to OpenCode seq={}", seq);
                }
            }
        }

        Ok(())
    }

    /// Handle a message from the OpenCode client
    async fn handle_client_message<W>(
        &self,
        msg: AttachMessage,
        writer: &mut W,
    ) -> DaemonResult<bool>
    where
        W: AsyncWriteExt + Unpin,
    {
        match msg.msg_type {
            AttachMessageType::Stdin => {
                let stdin_data: StdinData = serde_json::from_value(msg.payload)
                    .map_err(|e| DaemonError::AttachError(format!("Invalid stdin payload: {}", e)))?;

                let data = stdin_data
                    .to_bytes()
                    .map_err(|e| DaemonError::AttachError(format!("Invalid stdin data: {}", e)))?;

                self.stdin_tx
                    .send(data)
                    .await
                    .map_err(|e| DaemonError::AttachError(format!("Failed to send stdin: {}", e)))?;

                debug!("Forwarded {} bytes of stdin from OpenCode to agent", stdin_data.bytes);
                Ok(true)
            }

            AttachMessageType::Ping => {
                let pong_msg = AttachMessage::new(AttachMessageType::Pong, serde_json::json!({}));
                self.send_message(writer, &pong_msg).await?;
                Ok(true)
            }

            AttachMessageType::Pong => {
                debug!("Received pong from OpenCode client");
                Ok(true)
            }

            AttachMessageType::Disconnect => {
                info!("OpenCode client requested disconnect");
                Ok(false)
            }

            AttachMessageType::ReadOutput => {
                self.send_historical_output(writer).await?;
                Ok(true)
            }

            _ => {
                warn!("Unexpected message type from OpenCode client: {:?}", msg.msg_type);
                let error_msg = AttachMessage::error(&format!(
                    "Unexpected message type: {:?}",
                    msg.msg_type
                ));
                self.send_message(writer, &error_msg).await?;
                Ok(true)
            }
        }
    }

    /// Read a message from the socket
    async fn read_message<R>(&self, reader: &mut BufReader<R>) -> DaemonResult<AttachMessage>
    where
        R: AsyncReadExt + Unpin,
    {
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to read message length: {}", e))
        })?;

        let msg_len = u32::from_be_bytes(len_buf) as usize;
        if msg_len > 10 * 1024 * 1024 {
            return Err(DaemonError::AttachError("Message too large".to_string()));
        }

        let mut msg_buf = vec![0u8; msg_len];
        reader.read_exact(&mut msg_buf).await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to read message body: {}", e))
        })?;

        AttachMessage::from_bytes(&msg_buf)
            .map_err(|e| DaemonError::AttachError(format!("Failed to parse message: {}", e)))
    }

    /// Send a message to the socket
    async fn send_message<W>(&self, writer: &mut W, msg: &AttachMessage) -> DaemonResult<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        let bytes = msg
            .to_bytes()
            .map_err(|e| DaemonError::AttachError(format!("Failed to serialize message: {}", e)))?;

        let len_bytes = (bytes.len() as u32).to_be_bytes();
        writer.write_all(&len_bytes).await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to write message length: {}", e))
        })?;

        writer.write_all(&bytes).await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to write message body: {}", e))
        })?;

        writer.flush().await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to flush: {}", e))
        })?;

        Ok(())
    }
}

/// Start an OpenCode TUI attach server for an agent
pub async fn start_opencode_attach_server(
    socket_path: &std::path::Path,
    config: OpenCodeTuiConfig,
    session_manager: Arc<AttachSessionManager>,
    agent_id: Uuid,
    agent_name: String,
    agent_task: String,
    stdin_tx: mpsc::Sender<Vec<u8>>,
    stdout_tx: broadcast::Sender<Vec<u8>>,
    stderr_tx: broadcast::Sender<Vec<u8>>,
) -> DaemonResult<()> {
    if socket_path.exists() {
        std::fs::remove_file(socket_path).map_err(|e| {
            DaemonError::ServerError(format!("Failed to remove existing socket: {}", e))
        })?;
    }

    if let Some(parent) = socket_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DaemonError::ServerError(format!("Failed to create socket directory: {}", e))
            })?;
        }
    }

    let listener = tokio::net::UnixListener::bind(socket_path).map_err(|e| {
        DaemonError::ServerError(format!("Failed to bind OpenCode attach socket: {}", e))
    })?;

    info!(
        "OpenCode TUI attach server listening on {:?}",
        socket_path
    );

    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                info!("New OpenCode TUI connection");

                let mut handler = OpenCodeTuiHandler::new(
                    config.clone(),
                    Arc::clone(&session_manager),
                    agent_id,
                    agent_name.clone(),
                    agent_task.clone(),
                    stdin_tx.clone(),
                    stdout_tx.subscribe(),
                    stderr_tx.subscribe(),
                );

                tokio::spawn(async move {
                    if let Err(e) = handler.handle_connection(stream).await {
                        error!("OpenCode TUI connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept OpenCode connection: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opencode_config_default() {
        let config = OpenCodeTuiConfig::default();
        assert!(config.enable_extended_protocol);
        assert!(config.allow_raw_mode);
        assert_eq!(config.base.max_history_bytes, 1024 * 1024);
    }

    #[test]
    fn test_opencode_config_with_base() {
        let mut base = ClaudeCodeTuiConfig::default();
        base.max_history_lines = 5000;

        let config = OpenCodeTuiConfig::with_base(base);
        assert_eq!(config.base.max_history_lines, 5000);
        assert!(config.enable_extended_protocol);
    }
}
