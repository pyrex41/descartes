//! Claude Code TUI Attachment Handler
//!
//! This module implements the server-side handling for Claude Code TUI attachments.
//! When a Claude Code client connects to a paused agent, this handler manages:
//! - WebSocket/Unix socket connection lifecycle
//! - Protocol handshake and authentication
//! - Stdin/stdout/stderr forwarding between Claude Code and the agent
//! - Historical output replay
//! - Session timeout handling

use crate::attach_session::AttachSessionManager;
use crate::errors::{DaemonError, DaemonResult};
use descartes_core::attach_protocol::{
    AttachHandshake, AttachHandshakeResponse, AttachMessage, AttachMessageType, HistoricalOutput,
    OutputData, StdinData, ATTACH_PROTOCOL_VERSION,
};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{broadcast, mpsc, RwLock};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Configuration for Claude Code TUI handler
#[derive(Debug, Clone)]
pub struct ClaudeCodeTuiConfig {
    /// Maximum buffer size for historical output (bytes)
    pub max_history_bytes: usize,
    /// Maximum number of historical lines to keep
    pub max_history_lines: usize,
    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,
    /// Ping interval for keepalive in seconds
    pub ping_interval_secs: u64,
    /// Read timeout for individual messages in milliseconds
    pub read_timeout_ms: u64,
}

impl Default for ClaudeCodeTuiConfig {
    fn default() -> Self {
        Self {
            max_history_bytes: 1024 * 1024, // 1MB
            max_history_lines: 10000,
            connection_timeout_secs: 300, // 5 minutes
            ping_interval_secs: 30,
            read_timeout_ms: 5000,
        }
    }
}

/// Output buffer for storing historical output
#[derive(Debug, Clone)]
pub struct OutputBuffer {
    /// Buffered stdout lines (raw bytes, base64 when serialized)
    stdout: VecDeque<Vec<u8>>,
    /// Buffered stderr lines (raw bytes, base64 when serialized)
    stderr: VecDeque<Vec<u8>>,
    /// Total bytes in stdout buffer
    stdout_bytes: usize,
    /// Total bytes in stderr buffer
    stderr_bytes: usize,
    /// Maximum bytes to keep
    max_bytes: usize,
    /// Maximum lines to keep
    max_lines: usize,
    /// Timestamp of first buffered line
    first_timestamp: i64,
    /// Timestamp of last buffered line
    last_timestamp: i64,
}

impl OutputBuffer {
    /// Create a new output buffer with the given limits
    pub fn new(max_bytes: usize, max_lines: usize) -> Self {
        Self {
            stdout: VecDeque::new(),
            stderr: VecDeque::new(),
            stdout_bytes: 0,
            stderr_bytes: 0,
            max_bytes,
            max_lines,
            first_timestamp: chrono::Utc::now().timestamp(),
            last_timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Add stdout data to the buffer
    pub fn push_stdout(&mut self, data: Vec<u8>) {
        let now = chrono::Utc::now().timestamp();
        if self.stdout.is_empty() && self.stderr.is_empty() {
            self.first_timestamp = now;
        }
        self.last_timestamp = now;

        self.stdout_bytes += data.len();
        self.stdout.push_back(data);

        // Trim if needed
        self.trim();
    }

    /// Add stderr data to the buffer
    pub fn push_stderr(&mut self, data: Vec<u8>) {
        let now = chrono::Utc::now().timestamp();
        if self.stdout.is_empty() && self.stderr.is_empty() {
            self.first_timestamp = now;
        }
        self.last_timestamp = now;

        self.stderr_bytes += data.len();
        self.stderr.push_back(data);

        // Trim if needed
        self.trim();
    }

    /// Trim buffer to stay within limits
    fn trim(&mut self) {
        // Trim by line count
        while self.stdout.len() + self.stderr.len() > self.max_lines {
            // Remove oldest from whichever is larger
            if self.stdout.len() > self.stderr.len() {
                if let Some(data) = self.stdout.pop_front() {
                    self.stdout_bytes -= data.len();
                }
            } else if let Some(data) = self.stderr.pop_front() {
                self.stderr_bytes -= data.len();
            }
        }

        // Trim by byte count
        while self.stdout_bytes + self.stderr_bytes > self.max_bytes {
            if self.stdout_bytes > self.stderr_bytes {
                if let Some(data) = self.stdout.pop_front() {
                    self.stdout_bytes -= data.len();
                }
            } else if let Some(data) = self.stderr.pop_front() {
                self.stderr_bytes -= data.len();
            }
        }
    }

    /// Get total number of buffered lines
    pub fn total_lines(&self) -> usize {
        self.stdout.len() + self.stderr.len()
    }

    /// Convert to HistoricalOutput for sending to client
    pub fn to_historical_output(&self) -> HistoricalOutput {
        use base64::Engine;
        let encoder = base64::engine::general_purpose::STANDARD;

        HistoricalOutput {
            stdout: self.stdout.iter().map(|d| encoder.encode(d)).collect(),
            stderr: self.stderr.iter().map(|d| encoder.encode(d)).collect(),
            timestamp_start: self.first_timestamp,
            timestamp_end: self.last_timestamp,
            stdout_bytes: self.stdout_bytes,
            stderr_bytes: self.stderr_bytes,
        }
    }

    /// Clear the buffer
    pub fn clear(&mut self) {
        self.stdout.clear();
        self.stderr.clear();
        self.stdout_bytes = 0;
        self.stderr_bytes = 0;
    }
}

/// Claude Code TUI connection handler
pub struct ClaudeCodeTuiHandler {
    /// Configuration
    config: ClaudeCodeTuiConfig,
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

impl ClaudeCodeTuiHandler {
    /// Create a new Claude Code TUI handler
    pub fn new(
        config: ClaudeCodeTuiConfig,
        session_manager: Arc<AttachSessionManager>,
        agent_id: Uuid,
        agent_name: String,
        agent_task: String,
        stdin_tx: mpsc::Sender<Vec<u8>>,
        stdout_rx: broadcast::Receiver<Vec<u8>>,
        stderr_rx: broadcast::Receiver<Vec<u8>>,
    ) -> Self {
        let output_buffer = Arc::new(RwLock::new(OutputBuffer::new(
            config.max_history_bytes,
            config.max_history_lines,
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

    /// Handle a new Unix socket connection from Claude Code
    pub async fn handle_connection(&mut self, stream: UnixStream) -> DaemonResult<()> {
        let (read_half, write_half) = stream.into_split();
        let mut reader = BufReader::new(read_half);
        let mut writer = write_half;

        // Perform handshake
        let session_info = self.perform_handshake(&mut reader, &mut writer).await?;
        info!(
            "Claude Code TUI handshake successful for agent {}",
            self.agent_id
        );

        // Send historical output
        self.send_historical_output(&mut writer).await?;
        info!("Historical output sent to Claude Code client");

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
    ) -> DaemonResult<Uuid>
    where
        R: AsyncReadExt + Unpin,
        W: AsyncWriteExt + Unpin,
    {
        // Read handshake message with timeout
        let handshake_timeout = Duration::from_secs(self.config.connection_timeout_secs);
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

        // Verify message type
        if handshake_msg.msg_type != AttachMessageType::Handshake {
            let response = AttachHandshakeResponse::failure("Expected handshake message");
            self.send_message(writer, &response.to_message()).await?;
            return Err(DaemonError::AttachError(
                "Invalid message type for handshake".to_string(),
            ));
        }

        // Parse handshake payload
        let handshake: AttachHandshake = serde_json::from_value(handshake_msg.payload)
            .map_err(|e| DaemonError::AttachError(format!("Invalid handshake payload: {}", e)))?;

        // Verify protocol version
        if handshake.version != ATTACH_PROTOCOL_VERSION {
            let response = AttachHandshakeResponse::failure(&format!(
                "Protocol version mismatch: expected {}, got {}",
                ATTACH_PROTOCOL_VERSION, handshake.version
            ));
            self.send_message(writer, &response.to_message()).await?;
            return Err(DaemonError::AttachError("Protocol version mismatch".to_string()));
        }

        // Validate token
        let validated_agent_id = self
            .session_manager
            .validate_token(&handshake.token)
            .await
            .ok_or_else(|| {
                DaemonError::AuthenticationFailed("Invalid or expired token".to_string())
            })?;

        // Verify agent ID matches
        if validated_agent_id != self.agent_id {
            let response = AttachHandshakeResponse::failure("Token not valid for this agent");
            self.send_message(writer, &response.to_message()).await?;
            return Err(DaemonError::AuthenticationFailed(
                "Token not valid for this agent".to_string(),
            ));
        }

        // Send success response
        let buffer = self.output_buffer.read().await;
        let response = AttachHandshakeResponse::success(
            self.agent_id.to_string(),
            self.agent_name.clone(),
            self.agent_task.clone(),
            buffer.total_lines(),
        );
        drop(buffer);

        self.send_message(writer, &response.to_message()).await?;

        Ok(validated_agent_id)
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
        _validated_agent_id: Uuid,
    ) -> DaemonResult<()>
    where
        R: AsyncReadExt + Unpin,
        W: AsyncWriteExt + Unpin,
    {
        let ping_interval = Duration::from_secs(self.config.ping_interval_secs);
        let mut ping_timer = tokio::time::interval(ping_interval);
        let mut seq: u64 = 0;

        // Extract mutable receivers to avoid borrow conflicts in select!
        let stdout_rx = &mut self.stdout_rx;
        let stderr_rx = &mut self.stderr_rx;
        let output_buffer = &self.output_buffer;
        let stdin_tx = &self.stdin_tx;

        loop {
            tokio::select! {
                // Handle incoming messages from Claude Code
                result = Self::read_message_static(reader) => {
                    match result {
                        Ok(msg) => {
                            if !Self::handle_client_message_static(msg, writer, stdin_tx).await? {
                                // Client disconnected gracefully
                                info!("Claude Code client disconnected gracefully");
                                break;
                            }
                        }
                        Err(e) => {
                            warn!("Error reading from Claude Code client: {}", e);
                            break;
                        }
                    }
                }

                // Forward stdout from agent to Claude Code
                result = stdout_rx.recv() => {
                    match result {
                        Ok(data) => {
                            // Buffer for history
                            {
                                let mut buffer = output_buffer.write().await;
                                buffer.push_stdout(data.clone());
                            }

                            // Forward to client
                            let output_data = OutputData::from_bytes(&data);
                            let msg = output_data.to_stdout_message();
                            if let Err(e) = Self::send_message_static(writer, &msg).await {
                                warn!("Error sending stdout to client: {}", e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Claude Code client lagged, missed {} stdout messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Agent stdout channel closed");
                            break;
                        }
                    }
                }

                // Forward stderr from agent to Claude Code
                result = stderr_rx.recv() => {
                    match result {
                        Ok(data) => {
                            // Buffer for history
                            {
                                let mut buffer = output_buffer.write().await;
                                buffer.push_stderr(data.clone());
                            }

                            // Forward to client
                            let output_data = OutputData::from_bytes(&data);
                            let msg = output_data.to_stderr_message();
                            if let Err(e) = Self::send_message_static(writer, &msg).await {
                                warn!("Error sending stderr to client: {}", e);
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("Claude Code client lagged, missed {} stderr messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            info!("Agent stderr channel closed");
                            break;
                        }
                    }
                }

                // Send periodic pings
                _ = ping_timer.tick() => {
                    seq += 1;
                    let ping_msg = AttachMessage::with_seq(
                        AttachMessageType::Ping,
                        serde_json::json!({}),
                        seq,
                    );
                    if let Err(e) = Self::send_message_static(writer, &ping_msg).await {
                        warn!("Error sending ping: {}", e);
                        break;
                    }
                    debug!("Sent ping seq={}", seq);
                }
            }
        }

        Ok(())
    }

    /// Handle a message from the Claude Code client
    #[allow(dead_code)]
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
                // Parse stdin data
                let stdin_data: StdinData = serde_json::from_value(msg.payload)
                    .map_err(|e| DaemonError::AttachError(format!("Invalid stdin payload: {}", e)))?;

                // Decode and forward to agent
                let data = stdin_data
                    .to_bytes()
                    .map_err(|e| DaemonError::AttachError(format!("Invalid stdin data: {}", e)))?;

                self.stdin_tx
                    .send(data)
                    .await
                    .map_err(|e| DaemonError::AttachError(format!("Failed to send stdin: {}", e)))?;

                debug!("Forwarded {} bytes of stdin to agent", stdin_data.bytes);
                Ok(true)
            }

            AttachMessageType::Ping => {
                // Respond with pong
                let pong_msg = AttachMessage::new(AttachMessageType::Pong, serde_json::json!({}));
                self.send_message(writer, &pong_msg).await?;
                Ok(true)
            }

            AttachMessageType::Pong => {
                // Client responded to our ping
                debug!("Received pong from client");
                Ok(true)
            }

            AttachMessageType::Disconnect => {
                info!("Client requested disconnect");
                Ok(false)
            }

            AttachMessageType::ReadOutput => {
                // Client wants to read buffered output (shouldn't happen normally after history sent)
                self.send_historical_output(writer).await?;
                Ok(true)
            }

            _ => {
                warn!("Unexpected message type from client: {:?}", msg.msg_type);
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
        Self::read_message_static(reader).await
    }

    /// Static version of read_message for use in select! blocks
    async fn read_message_static<R>(reader: &mut BufReader<R>) -> DaemonResult<AttachMessage>
    where
        R: AsyncReadExt + Unpin,
    {
        // Read length prefix (4 bytes, big-endian)
        let mut len_buf = [0u8; 4];
        reader.read_exact(&mut len_buf).await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to read message length: {}", e))
        })?;

        let msg_len = u32::from_be_bytes(len_buf) as usize;
        if msg_len > 10 * 1024 * 1024 {
            // 10MB max
            return Err(DaemonError::AttachError("Message too large".to_string()));
        }

        // Read message body
        let mut msg_buf = vec![0u8; msg_len];
        reader.read_exact(&mut msg_buf).await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to read message body: {}", e))
        })?;

        // Parse message
        AttachMessage::from_bytes(&msg_buf)
            .map_err(|e| DaemonError::AttachError(format!("Failed to parse message: {}", e)))
    }

    /// Send a message to the socket
    async fn send_message<W>(&self, writer: &mut W, msg: &AttachMessage) -> DaemonResult<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        Self::send_message_static(writer, msg).await
    }

    /// Static version of send_message for use in select! blocks
    async fn send_message_static<W>(writer: &mut W, msg: &AttachMessage) -> DaemonResult<()>
    where
        W: AsyncWriteExt + Unpin,
    {
        let bytes = msg
            .to_bytes()
            .map_err(|e| DaemonError::AttachError(format!("Failed to serialize message: {}", e)))?;

        // Send length prefix
        let len_bytes = (bytes.len() as u32).to_be_bytes();
        writer.write_all(&len_bytes).await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to write message length: {}", e))
        })?;

        // Send message body
        writer.write_all(&bytes).await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to write message body: {}", e))
        })?;

        writer.flush().await.map_err(|e| {
            DaemonError::ConnectionError(format!("Failed to flush: {}", e))
        })?;

        Ok(())
    }

    /// Static version of handle_client_message for use in select! blocks
    async fn handle_client_message_static<W>(
        msg: AttachMessage,
        writer: &mut W,
        stdin_tx: &mpsc::Sender<Vec<u8>>,
    ) -> DaemonResult<bool>
    where
        W: AsyncWriteExt + Unpin,
    {
        match msg.msg_type {
            AttachMessageType::Stdin => {
                // Forward stdin to agent
                let stdin_data: StdinData = serde_json::from_value(msg.payload)
                    .map_err(|e| DaemonError::AttachError(format!("Invalid stdin payload: {}", e)))?;

                let data = stdin_data.to_bytes()
                    .map_err(|e| DaemonError::AttachError(format!("Failed to decode stdin: {}", e)))?;

                stdin_tx.send(data).await.map_err(|e| {
                    DaemonError::AttachError(format!("Failed to send stdin to agent: {}", e))
                })?;

                Ok(true)
            }
            AttachMessageType::Pong => {
                // Handle pong response
                debug!("Received pong from client");
                Ok(true)
            }
            AttachMessageType::Disconnect => {
                // Client is disconnecting
                info!("Client sent disconnect message");
                Ok(false)
            }
            _ => {
                warn!("Unexpected message type from client: {:?}", msg.msg_type);
                let error_msg = AttachMessage::error(
                    &format!("Unexpected message type: {:?}", msg.msg_type),
                );
                Self::send_message_static(writer, &error_msg).await?;
                Ok(true)
            }
        }
    }
}

/// Start a Claude Code TUI attach server for an agent
pub async fn start_attach_server(
    socket_path: &std::path::Path,
    config: ClaudeCodeTuiConfig,
    session_manager: Arc<AttachSessionManager>,
    agent_id: Uuid,
    agent_name: String,
    agent_task: String,
    stdin_tx: mpsc::Sender<Vec<u8>>,
    stdout_tx: broadcast::Sender<Vec<u8>>,
    stderr_tx: broadcast::Sender<Vec<u8>>,
) -> DaemonResult<()> {
    // Remove existing socket if present
    if socket_path.exists() {
        std::fs::remove_file(socket_path).map_err(|e| {
            DaemonError::ServerError(format!("Failed to remove existing socket: {}", e))
        })?;
    }

    // Create parent directory if needed
    if let Some(parent) = socket_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).map_err(|e| {
                DaemonError::ServerError(format!("Failed to create socket directory: {}", e))
            })?;
        }
    }

    // Bind to socket
    let listener = tokio::net::UnixListener::bind(socket_path).map_err(|e| {
        DaemonError::ServerError(format!("Failed to bind attach socket: {}", e))
    })?;

    info!(
        "Claude Code TUI attach server listening on {:?}",
        socket_path
    );

    // Accept connections
    loop {
        match listener.accept().await {
            Ok((stream, _)) => {
                info!("New Claude Code TUI connection");

                // Create handler for this connection
                let mut handler = ClaudeCodeTuiHandler::new(
                    config.clone(),
                    Arc::clone(&session_manager),
                    agent_id,
                    agent_name.clone(),
                    agent_task.clone(),
                    stdin_tx.clone(),
                    stdout_tx.subscribe(),
                    stderr_tx.subscribe(),
                );

                // Handle connection in background
                tokio::spawn(async move {
                    if let Err(e) = handler.handle_connection(stream).await {
                        error!("Claude Code TUI connection error: {}", e);
                    }
                });
            }
            Err(e) => {
                error!("Failed to accept connection: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_buffer_new() {
        let buffer = OutputBuffer::new(1000, 100);
        assert_eq!(buffer.total_lines(), 0);
        assert_eq!(buffer.stdout_bytes, 0);
        assert_eq!(buffer.stderr_bytes, 0);
    }

    #[test]
    fn test_output_buffer_push() {
        let mut buffer = OutputBuffer::new(1000, 100);
        buffer.push_stdout(vec![1, 2, 3]);
        buffer.push_stderr(vec![4, 5]);

        assert_eq!(buffer.total_lines(), 2);
        assert_eq!(buffer.stdout_bytes, 3);
        assert_eq!(buffer.stderr_bytes, 2);
    }

    #[test]
    fn test_output_buffer_trim_by_lines() {
        let mut buffer = OutputBuffer::new(10000, 5);

        for i in 0..10 {
            buffer.push_stdout(vec![i]);
        }

        assert_eq!(buffer.total_lines(), 5);
    }

    #[test]
    fn test_output_buffer_trim_by_bytes() {
        let mut buffer = OutputBuffer::new(10, 1000);

        for i in 0..10 {
            buffer.push_stdout(vec![i, i, i, i, i]); // 5 bytes each
        }

        // Should have trimmed to about 10 bytes
        assert!(buffer.stdout_bytes <= 10);
    }

    #[test]
    fn test_output_buffer_to_historical() {
        let mut buffer = OutputBuffer::new(1000, 100);
        buffer.push_stdout(b"hello".to_vec());
        buffer.push_stderr(b"world".to_vec());

        let history = buffer.to_historical_output();
        assert_eq!(history.stdout.len(), 1);
        assert_eq!(history.stderr.len(), 1);
        assert_eq!(history.stdout_bytes, 5);
        assert_eq!(history.stderr_bytes, 5);
    }

    #[test]
    fn test_config_default() {
        let config = ClaudeCodeTuiConfig::default();
        assert_eq!(config.max_history_bytes, 1024 * 1024);
        assert_eq!(config.max_history_lines, 10000);
        assert_eq!(config.connection_timeout_secs, 300);
        assert_eq!(config.ping_interval_secs, 30);
    }
}
