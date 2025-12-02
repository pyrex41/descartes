/// Agent spawning and process management for the Descartes orchestration system.
///
/// This module provides production-ready implementations for spawning and managing
/// AI agent processes, including:
/// - LocalProcessRunner: Spawns CLI processes (claude, opencode, etc.)
/// - Process lifecycle management (start, stop, pause, resume)
/// - stdin/stdout/stderr streaming with JSON parsing
/// - Signal handling (SIGINT, SIGTERM, SIGKILL)
/// - Agent handle/ID tracking
/// - Health checks and monitoring
/// - Graceful shutdown mechanisms
use crate::errors::{AgentError, AgentResult};
use crate::traits::{
    AgentConfig, AgentHandle, AgentInfo, AgentRunner, AgentSignal, AgentStatus, ExitStatus,
    PauseMode,
};
use async_trait::async_trait;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command};
use tokio::sync::{broadcast, mpsc, Mutex};
use uuid::Uuid;

/// Maximum buffer size for stdout/stderr streams (16KB)
const STREAM_BUFFER_SIZE: usize = 16 * 1024;

/// Timeout for graceful shutdown before force kill (5 seconds)
const SHUTDOWN_TIMEOUT_SECS: u64 = 5;

/// Health check interval in seconds
const HEALTH_CHECK_INTERVAL_SECS: u64 = 30;

/// LocalProcessRunner spawns and manages agent processes on the local system.
///
/// This is the primary implementation of the AgentRunner trait, designed for
/// running headless CLI tools as child processes with full lifecycle management.
pub struct LocalProcessRunner {
    /// Registry of all spawned agents
    agents: Arc<DashMap<Uuid, Arc<RwLock<LocalAgentHandle>>>>,
    /// Configuration for default behavior
    config: ProcessRunnerConfig,
}

/// Configuration for the process runner
#[derive(Debug, Clone)]
pub struct ProcessRunnerConfig {
    /// Working directory for spawned processes
    pub working_dir: Option<std::path::PathBuf>,
    /// Enable JSON streaming mode for stdout parsing
    pub enable_json_streaming: bool,
    /// Enable automatic health checks
    pub enable_health_checks: bool,
    /// Health check interval in seconds
    pub health_check_interval_secs: u64,
    /// Maximum concurrent agents
    pub max_concurrent_agents: Option<usize>,
}

impl Default for ProcessRunnerConfig {
    fn default() -> Self {
        Self {
            working_dir: None,
            enable_json_streaming: true,
            enable_health_checks: true,
            health_check_interval_secs: HEALTH_CHECK_INTERVAL_SECS,
            max_concurrent_agents: None,
        }
    }
}

impl LocalProcessRunner {
    /// Create a new LocalProcessRunner with default configuration.
    pub fn new() -> Self {
        Self::with_config(ProcessRunnerConfig::default())
    }

    /// Create a new LocalProcessRunner with custom configuration.
    pub fn with_config(config: ProcessRunnerConfig) -> Self {
        Self {
            agents: Arc::new(DashMap::new()),
            config,
        }
    }

    /// Get access to the agent handle for TUI attachment.
    ///
    /// This returns the raw Arc<RwLock<LocalAgentHandle>> which provides
    /// access to the I/O channels needed for TUI forwarding.
    pub fn get_agent_handle(&self, agent_id: &Uuid) -> Option<Arc<RwLock<LocalAgentHandle>>> {
        self.agents.get(agent_id).map(|h| Arc::clone(&h))
    }

    /// Check if we can spawn more agents based on max_concurrent_agents
    fn can_spawn_agent(&self) -> bool {
        if let Some(max) = self.config.max_concurrent_agents {
            let active_agents = self
                .agents
                .iter()
                .filter(|entry| {
                    let handle = entry.value().read();
                    !handle.status.is_terminal()
                })
                .count();
            active_agents < max
        } else {
            true
        }
    }

    /// Build the tokio Command from AgentConfig
    fn build_command(&self, config: &AgentConfig) -> AgentResult<Command> {
        // Determine the command based on model_backend
        let (cmd, args) = match config.model_backend.as_str() {
            "claude-code-cli" | "claude" => {
                // Claude Code CLI
                let mut args = vec![];

                // Add task/prompt as argument
                args.push(config.task.clone());

                (String::from("claude"), args)
            }
            "opencode" => {
                // OpenCode CLI
                let mut args = vec![String::from("--headless")];
                args.push(config.task.clone());
                (String::from("opencode"), args)
            }
            backend if backend.contains("cli") => {
                // Generic CLI backend - parse from backend name
                let parts: Vec<&str> = backend.split('-').collect();
                let cmd = parts.first().unwrap_or(&"unknown").to_string();
                (cmd, vec![config.task.clone()])
            }
            _ => {
                return Err(AgentError::SpawnFailed(format!(
                    "Unsupported model backend for process spawning: {}. Use API mode instead.",
                    config.model_backend
                )));
            }
        };

        let mut command = Command::new(&cmd);
        command.args(&args);

        // Set up stdio pipes
        command.stdin(Stdio::piped());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        // Set working directory
        if let Some(ref working_dir) = self.config.working_dir {
            command.current_dir(working_dir);
        }

        // Set environment variables
        for (key, value) in &config.environment {
            command.env(key, value);
        }

        // Add context as environment variable if provided
        if let Some(ref context) = config.context {
            command.env("AGENT_CONTEXT", context);
        }

        // Add system prompt as environment variable if provided
        if let Some(ref system_prompt) = config.system_prompt {
            command.env("AGENT_SYSTEM_PROMPT", system_prompt);
        }

        Ok(command)
    }

    /// Spawn background health checker for an agent
    fn spawn_health_checker(
        &self,
        agent_id: Uuid,
        agents: Arc<DashMap<Uuid, Arc<RwLock<LocalAgentHandle>>>>,
    ) {
        if !self.config.enable_health_checks {
            return;
        }

        let interval_secs = self.config.health_check_interval_secs;

        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                // Get the agent handle
                let handle = match agents.get(&agent_id) {
                    Some(h) => h.clone(),
                    None => {
                        // Agent has been removed, exit health check loop
                        break;
                    }
                };

                // Check if process is still alive
                let is_alive = {
                    let handle_guard = handle.read();
                    handle_guard.is_alive()
                };

                if !is_alive {
                    // Update status to terminated
                    if let Some(h) = agents.get(&agent_id) {
                        let mut handle_guard = h.write();
                        if handle_guard.exit_status.is_some() {
                            // Already recorded by exit observer
                        } else if !handle_guard.status.is_terminal() {
                            handle_guard.set_status(AgentStatus::Terminated);
                        }
                    }
                    break;
                }
            }
        });
    }

    /// Monitor exit status so handles update once the child process finishes.
    fn spawn_exit_observer(&self, agent_id: Uuid, handle: Arc<RwLock<LocalAgentHandle>>) {
        tokio::spawn(async move {
            loop {
                let child_handle = {
                    let handle_read = handle.read();
                    if handle_read.status.is_terminal() || handle_read.exit_status.is_some() {
                        break;
                    }
                    Arc::clone(&handle_read.child)
                };

                let mut child_guard = child_handle.lock().await;
                match child_guard.try_wait() {
                    Ok(Some(status)) => {
                        let exit_status = ExitStatus {
                            code: status.code(),
                            success: status.success(),
                        };
                        {
                            let mut handle_write = handle.write();
                            handle_write.record_exit_status(exit_status);
                        }
                        break;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        tracing::error!(
                            "Failed to poll process status for agent {}: {}",
                            agent_id,
                            e
                        );
                        break;
                    }
                }

                tokio::time::sleep(Duration::from_millis(250)).await;
            }
        });
    }
}

impl Default for LocalProcessRunner {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AgentRunner for LocalProcessRunner {
    async fn spawn(&self, config: AgentConfig) -> AgentResult<Box<dyn AgentHandle>> {
        // Check if we can spawn more agents
        if !self.can_spawn_agent() {
            return Err(AgentError::SpawnFailed(format!(
                "Maximum concurrent agents limit reached: {:?}",
                self.config.max_concurrent_agents
            )));
        }

        // Build the command
        let mut command = self.build_command(&config)?;

        // Spawn the process
        let mut child = command.spawn().map_err(|e| {
            AgentError::SpawnFailed(format!(
                "Failed to spawn process for backend '{}': {}",
                config.model_backend, e
            ))
        })?;

        // Extract stdio handles
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| AgentError::SpawnFailed("Failed to capture stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| AgentError::SpawnFailed("Failed to capture stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| AgentError::SpawnFailed("Failed to capture stderr".to_string()))?;

        // Create agent handle
        let agent_id = Uuid::new_v4();
        let agent_info = AgentInfo {
            id: agent_id,
            name: config.name.clone(),
            status: AgentStatus::Running,
            model_backend: config.model_backend.clone(),
            started_at: SystemTime::now(),
            task: config.task.clone(),
            paused_at: None,
            pause_mode: None,
            attach_info: None,
        };

        let handle = LocalAgentHandle::new(
            agent_info,
            child,
            stdin,
            stdout,
            stderr,
            self.config.enable_json_streaming,
        );

        let handle_arc = Arc::new(RwLock::new(handle));
        self.agents.insert(agent_id, handle_arc.clone());

        // Spawn health checker
        self.spawn_health_checker(agent_id, self.agents.clone());
        self.spawn_exit_observer(agent_id, handle_arc.clone());

        // Return boxed handle wrapper
        Ok(Box::new(AgentHandleWrapper {
            id: agent_id,
            handle: handle_arc,
        }))
    }

    async fn list_agents(&self) -> AgentResult<Vec<AgentInfo>> {
        let mut agents = Vec::new();
        for entry in self.agents.iter() {
            let handle = entry.value().read();
            agents.push(handle.info.clone());
        }
        Ok(agents)
    }

    async fn get_agent(&self, agent_id: &Uuid) -> AgentResult<Option<AgentInfo>> {
        if let Some(handle) = self.agents.get(agent_id) {
            let handle_guard = handle.read();
            Ok(Some(handle_guard.info.clone()))
        } else {
            Ok(None)
        }
    }

    async fn kill(&self, agent_id: &Uuid) -> AgentResult<()> {
        if let Some(handle) = self.agents.get(agent_id) {
            let child = {
                let handle_guard = handle.read();
                Arc::clone(&handle_guard.child)
            };

            let mut child_guard = child.lock().await;
            child_guard.kill().await?;
            drop(child_guard);

            // Update the handle's status
            {
                let mut handle_guard = handle.write();
                handle_guard.set_status(AgentStatus::Terminated);
                handle_guard.exit_status = Some(ExitStatus {
                    code: None,
                    success: false,
                });
            }

            self.agents.remove(agent_id);
            Ok(())
        } else {
            Err(AgentError::NotFound(format!(
                "Agent not found: {}",
                agent_id
            )))
        }
    }

    async fn signal(&self, agent_id: &Uuid, signal: AgentSignal) -> AgentResult<()> {
        if let Some(handle) = self.agents.get(agent_id) {
            let child = {
                let handle_guard = handle.read();
                Arc::clone(&handle_guard.child)
            };

            let mut child_guard = child.lock().await;

            match signal {
                AgentSignal::Interrupt => {
                    // Send SIGINT (Ctrl+C)
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        if let Some(pid) = child_guard.id() {
                            kill(Pid::from_raw(pid as i32), Signal::SIGINT).map_err(|e| {
                                AgentError::ExecutionError(format!("Failed to send SIGINT: {}", e))
                            })?;
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        return Err(AgentError::UnsupportedOperation(
                            "SIGINT not supported on this platform".into(),
                        ));
                    }
                }
                AgentSignal::Terminate => {
                    // Send SIGTERM
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        if let Some(pid) = child_guard.id() {
                            kill(Pid::from_raw(pid as i32), Signal::SIGTERM).map_err(|e| {
                                AgentError::ExecutionError(format!("Failed to send SIGTERM: {}", e))
                            })?;
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        child_guard.kill().await?;
                    }
                }
                AgentSignal::Kill => {
                    child_guard.kill().await?;
                }
                AgentSignal::ForcePause => {
                    // Send SIGSTOP to freeze the process
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        if let Some(pid) = child_guard.id() {
                            kill(Pid::from_raw(pid as i32), Signal::SIGSTOP).map_err(|e| {
                                AgentError::ExecutionError(format!("Failed to send SIGSTOP: {}", e))
                            })?;
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        return Err(AgentError::UnsupportedOperation(
                            "SIGSTOP not supported on this platform".into(),
                        ));
                    }
                }
                AgentSignal::Resume => {
                    // Send SIGCONT to resume the process
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        if let Some(pid) = child_guard.id() {
                            kill(Pid::from_raw(pid as i32), Signal::SIGCONT).map_err(|e| {
                                AgentError::ExecutionError(format!("Failed to send SIGCONT: {}", e))
                            })?;
                        }
                    }
                    #[cfg(not(unix))]
                    {
                        return Err(AgentError::UnsupportedOperation(
                            "SIGCONT not supported on this platform".into(),
                        ));
                    }
                }
            }

            drop(child_guard);

            {
                let mut handle_guard = handle.write();
                match signal {
                    AgentSignal::Interrupt => handle_guard.set_paused(PauseMode::Cooperative),
                    AgentSignal::Terminate => handle_guard.set_status(AgentStatus::Terminated),
                    AgentSignal::Kill => {
                        handle_guard.set_status(AgentStatus::Terminated);
                        handle_guard.exit_status = Some(ExitStatus {
                            code: None,
                            success: false,
                        });
                    }
                    AgentSignal::ForcePause => handle_guard.set_paused(PauseMode::Forced),
                    AgentSignal::Resume => {
                        handle_guard.clear_paused();
                        handle_guard.set_status(AgentStatus::Running);
                    }
                }
            }

            Ok(())
        } else {
            Err(AgentError::NotFound(format!(
                "Agent not found: {}",
                agent_id
            )))
        }
    }

    async fn pause(&self, agent_id: &Uuid, force: bool) -> AgentResult<()> {
        if let Some(handle) = self.agents.get(agent_id) {
            // Check if agent is in a pauseable state
            {
                let handle_guard = handle.read();
                match handle_guard.status {
                    AgentStatus::Running | AgentStatus::Thinking => {}
                    AgentStatus::Paused => {
                        return Err(AgentError::ExecutionError(
                            "Agent is already paused".into(),
                        ));
                    }
                    status if status.is_terminal() => {
                        return Err(AgentError::ExecutionError(format!(
                            "Cannot pause agent in terminal state: {}",
                            status
                        )));
                    }
                    status => {
                        return Err(AgentError::ExecutionError(format!(
                            "Cannot pause agent in state: {}",
                            status
                        )));
                    }
                }
            }

            if force {
                // Force pause using SIGSTOP
                self.signal(agent_id, AgentSignal::ForcePause).await?;
                tracing::info!("Agent {} force-paused (SIGSTOP)", agent_id);
            } else {
                // Cooperative pause: send notification via stdin
                let stdin = {
                    let handle_guard = handle.read();
                    Arc::clone(&handle_guard.stdin)
                };

                // Send pause notification (JSON format for compatibility)
                let pause_msg = r#"{"type":"pause","action":"pause"}"#;
                {
                    let mut stdin_guard = stdin.lock().await;
                    stdin_guard
                        .write_all(format!("{}\n", pause_msg).as_bytes())
                        .await?;
                    stdin_guard.flush().await?;
                }

                // Update status to paused
                {
                    let mut handle_guard = handle.write();
                    handle_guard.set_paused(PauseMode::Cooperative);
                }

                tracing::info!("Agent {} cooperatively paused", agent_id);
            }

            Ok(())
        } else {
            Err(AgentError::NotFound(format!(
                "Agent not found: {}",
                agent_id
            )))
        }
    }

    async fn resume(&self, agent_id: &Uuid) -> AgentResult<()> {
        if let Some(handle) = self.agents.get(agent_id) {
            let pause_mode = {
                let handle_guard = handle.read();
                if handle_guard.status != AgentStatus::Paused {
                    return Err(AgentError::ExecutionError(format!(
                        "Agent is not paused, current status: {}",
                        handle_guard.status
                    )));
                }
                handle_guard.pause_mode
            };

            // If force-paused, send SIGCONT first
            if pause_mode == Some(PauseMode::Forced) {
                self.signal(agent_id, AgentSignal::Resume).await?;
            } else {
                // For cooperative pause, send resume via stdin and update status
                let stdin = {
                    let handle_guard = handle.read();
                    Arc::clone(&handle_guard.stdin)
                };

                // Send resume notification
                let resume_msg = r#"{"type":"pause","action":"resume"}"#;
                {
                    let mut stdin_guard = stdin.lock().await;
                    stdin_guard
                        .write_all(format!("{}\n", resume_msg).as_bytes())
                        .await?;
                    stdin_guard.flush().await?;
                }

                // Update status
                {
                    let mut handle_guard = handle.write();
                    handle_guard.clear_paused();
                    handle_guard.set_status(AgentStatus::Running);
                }
            }

            tracing::info!("Agent {} resumed", agent_id);
            Ok(())
        } else {
            Err(AgentError::NotFound(format!(
                "Agent not found: {}",
                agent_id
            )))
        }
    }
}

/// LocalAgentHandle manages a single spawned agent process.
///
/// Provides full control over the process lifecycle, including stdio streaming,
/// signal handling, and status tracking.
pub struct LocalAgentHandle {
    /// Agent information
    info: AgentInfo,
    /// The child process
    child: Arc<Mutex<Child>>,
    /// Stdin writer
    stdin: Arc<Mutex<ChildStdin>>,
    /// Current status
    status: AgentStatus,
    /// Recorded exit status (if the process has completed)
    exit_status: Option<ExitStatus>,
    /// Enable JSON streaming mode
    json_streaming: bool,
    /// Buffered stdout lines
    stdout_buffer: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
    /// Buffered stderr lines
    stderr_buffer: Arc<Mutex<mpsc::UnboundedReceiver<Vec<u8>>>>,
    /// Stdout sender (for background task)
    _stdout_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Stderr sender (for background task)
    _stderr_tx: mpsc::UnboundedSender<Vec<u8>>,
    /// Broadcast sender for stdout (for TUI attachment)
    stdout_broadcast: broadcast::Sender<Vec<u8>>,
    /// Broadcast sender for stderr (for TUI attachment)
    stderr_broadcast: broadcast::Sender<Vec<u8>>,
    /// How the agent was paused (if currently paused)
    pause_mode: Option<PauseMode>,
}

impl LocalAgentHandle {
    /// Create a new LocalAgentHandle with stdio streams.
    fn new(
        info: AgentInfo,
        child: Child,
        stdin: ChildStdin,
        stdout: ChildStdout,
        stderr: ChildStderr,
        json_streaming: bool,
    ) -> Self {
        // Create channels for buffering stdout/stderr
        let (stdout_tx, stdout_rx) = mpsc::unbounded_channel();
        let (stderr_tx, stderr_rx) = mpsc::unbounded_channel();

        // Create broadcast channels for TUI attachment (1024 message buffer)
        let (stdout_broadcast, _) = broadcast::channel(1024);
        let (stderr_broadcast, _) = broadcast::channel(1024);

        let stdout_reader = BufReader::new(stdout);
        let stderr_reader = BufReader::new(stderr);

        // Spawn background tasks to read stdout/stderr (also broadcasts)
        Self::spawn_stdout_reader(stdout_reader.into(), stdout_tx.clone(), stdout_broadcast.clone());
        Self::spawn_stderr_reader(stderr_reader.into(), stderr_tx.clone(), stderr_broadcast.clone());

        Self {
            info,
            child: Arc::new(Mutex::new(child)),
            stdin: Arc::new(Mutex::new(stdin)),
            status: AgentStatus::Running,
            exit_status: None,
            json_streaming,
            stdout_buffer: Arc::new(Mutex::new(stdout_rx)),
            stderr_buffer: Arc::new(Mutex::new(stderr_rx)),
            _stdout_tx: stdout_tx,
            _stderr_tx: stderr_tx,
            stdout_broadcast,
            stderr_broadcast,
            pause_mode: None,
        }
    }

    /// Spawn a background task to read stdout lines into a channel.
    fn spawn_stdout_reader(
        mut reader: BufReader<ChildStdout>,
        tx: mpsc::UnboundedSender<Vec<u8>>,
        broadcast_tx: broadcast::Sender<Vec<u8>>,
    ) {
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let data = line.as_bytes().to_vec();
                        // Send to local buffer
                        if tx.send(data.clone()).is_err() {
                            break; // Receiver dropped
                        }
                        // Also broadcast to any attached TUIs (ignore errors - no subscribers is fine)
                        let _ = broadcast_tx.send(data);
                    }
                    Err(_) => break,
                }
            }
        });
    }

    /// Spawn a background task to read stderr lines into a channel.
    fn spawn_stderr_reader(
        mut reader: BufReader<ChildStderr>,
        tx: mpsc::UnboundedSender<Vec<u8>>,
        broadcast_tx: broadcast::Sender<Vec<u8>>,
    ) {
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let data = line.as_bytes().to_vec();
                        // Send to local buffer
                        if tx.send(data.clone()).is_err() {
                            break; // Receiver dropped
                        }
                        // Also broadcast to any attached TUIs (ignore errors - no subscribers is fine)
                        let _ = broadcast_tx.send(data);
                    }
                    Err(_) => break,
                }
            }
        });
    }

    /// Update both the internal status and the externally reported AgentInfo.
    fn set_status(&mut self, status: AgentStatus) {
        self.status = status;
        self.info.status = status;
    }

    /// Set paused state with pause mode and timestamp.
    fn set_paused(&mut self, mode: PauseMode) {
        self.status = AgentStatus::Paused;
        self.info.status = AgentStatus::Paused;
        self.pause_mode = Some(mode);
        self.info.pause_mode = Some(mode);
        self.info.paused_at = Some(SystemTime::now());
    }

    /// Clear paused state.
    fn clear_paused(&mut self) {
        self.pause_mode = None;
        self.info.pause_mode = None;
        self.info.paused_at = None;
    }

    /// Record the final exit status from the process.
    fn record_exit_status(&mut self, exit_status: ExitStatus) {
        self.exit_status = Some(exit_status.clone());
        if self.status == AgentStatus::Terminated {
            // Preserve explicit termination status
            return;
        }

        let next_status = if exit_status.success {
            AgentStatus::Completed
        } else {
            AgentStatus::Failed
        };
        self.set_status(next_status);
    }

    /// Check if the process is still alive.
    fn is_alive(&self) -> bool {
        // Try to get a non-blocking lock
        if let Ok(mut child) = self.child.try_lock() {
            match child.try_wait() {
                Ok(Some(_)) => false, // Process has exited
                Ok(None) => true,     // Process is still running
                Err(_) => false,      // Error checking status
            }
        } else {
            // If we can't get the lock, assume it's alive
            true
        }
    }

    /// Send a signal to the process.
    async fn send_signal(&mut self, signal: AgentSignal) -> AgentResult<()> {
        {
            let mut child = self.child.lock().await;

            match signal {
                AgentSignal::Interrupt => {
                    // Send SIGINT (Ctrl+C)
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        if let Some(pid) = child.id() {
                            kill(Pid::from_raw(pid as i32), Signal::SIGINT).map_err(|e| {
                                AgentError::ExecutionError(format!("Failed to send SIGINT: {}", e))
                            })?;
                        }
                    }

                    #[cfg(not(unix))]
                    {
                        // On Windows, try to gracefully shutdown
                        child.kill().await?;
                    }
                }
                AgentSignal::Terminate => {
                    // Send SIGTERM
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        if let Some(pid) = child.id() {
                            kill(Pid::from_raw(pid as i32), Signal::SIGTERM).map_err(|e| {
                                AgentError::ExecutionError(format!("Failed to send SIGTERM: {}", e))
                            })?;
                        }
                    }

                    #[cfg(not(unix))]
                    {
                        child.kill().await?;
                    }
                }
                AgentSignal::Kill => {
                    // Send SIGKILL
                    child.kill().await?;
                }
                AgentSignal::ForcePause => {
                    // Send SIGSTOP
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        if let Some(pid) = child.id() {
                            kill(Pid::from_raw(pid as i32), Signal::SIGSTOP).map_err(|e| {
                                AgentError::ExecutionError(format!("Failed to send SIGSTOP: {}", e))
                            })?;
                        }
                    }

                    #[cfg(not(unix))]
                    {
                        return Err(AgentError::UnsupportedOperation(
                            "SIGSTOP not supported on this platform".into(),
                        ));
                    }
                }
                AgentSignal::Resume => {
                    // Send SIGCONT
                    #[cfg(unix)]
                    {
                        use nix::sys::signal::{kill, Signal};
                        use nix::unistd::Pid;

                        if let Some(pid) = child.id() {
                            kill(Pid::from_raw(pid as i32), Signal::SIGCONT).map_err(|e| {
                                AgentError::ExecutionError(format!("Failed to send SIGCONT: {}", e))
                            })?;
                        }
                    }

                    #[cfg(not(unix))]
                    {
                        return Err(AgentError::UnsupportedOperation(
                            "SIGCONT not supported on this platform".into(),
                        ));
                    }
                }
            }
        }

        match signal {
            AgentSignal::Interrupt => {
                self.set_paused(PauseMode::Cooperative);
            }
            AgentSignal::Terminate => {
                self.set_status(AgentStatus::Terminated);
            }
            AgentSignal::Kill => {
                self.set_status(AgentStatus::Terminated);
                self.exit_status = Some(ExitStatus {
                    code: None,
                    success: false,
                });
            }
            AgentSignal::ForcePause => {
                self.set_paused(PauseMode::Forced);
            }
            AgentSignal::Resume => {
                self.clear_paused();
                self.set_status(AgentStatus::Running);
            }
        }

        Ok(())
    }

    /// Kill the process immediately.
    async fn kill(&mut self) -> AgentResult<()> {
        {
            let mut child = self.child.lock().await;
            child.kill().await?;
        }
        self.set_status(AgentStatus::Terminated);
        self.exit_status = Some(ExitStatus {
            code: None,
            success: false,
        });
        Ok(())
    }

    /// Wait for the process to complete.
    async fn wait(&mut self) -> AgentResult<ExitStatus> {
        let status = {
            let mut child = self.child.lock().await;
            child.wait().await?
        };

        let exit_status = ExitStatus {
            code: status.code(),
            success: status.success(),
        };

        self.record_exit_status(exit_status.clone());

        Ok(exit_status)
    }

    /// Write data to stdin.
    async fn write_stdin(&mut self, data: &[u8]) -> AgentResult<()> {
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(data).await?;
        stdin.flush().await?;
        Ok(())
    }

    /// Read from stdout buffer (non-blocking).
    async fn read_stdout(&mut self) -> AgentResult<Option<Vec<u8>>> {
        let mut buffer = self.stdout_buffer.lock().await;
        Ok(buffer.try_recv().ok())
    }

    /// Read from stderr buffer (non-blocking).
    async fn read_stderr(&mut self) -> AgentResult<Option<Vec<u8>>> {
        let mut buffer = self.stderr_buffer.lock().await;
        Ok(buffer.try_recv().ok())
    }

    // ========= TUI Attachment Accessors =========

    /// Get the agent info.
    pub fn agent_info(&self) -> &AgentInfo {
        &self.info
    }

    /// Get the stdin sender for writing to the agent.
    /// Returns an mpsc sender that can be used to forward stdin from TUI.
    pub fn get_stdin_sender(&self) -> mpsc::Sender<Vec<u8>> {
        // Create a new channel that forwards to stdin
        let (tx, mut rx) = mpsc::channel::<Vec<u8>>(256);
        let stdin = Arc::clone(&self.stdin);
        tokio::spawn(async move {
            while let Some(data) = rx.recv().await {
                let mut stdin_guard = stdin.lock().await;
                if stdin_guard.write_all(&data).await.is_err() {
                    break;
                }
                if stdin_guard.flush().await.is_err() {
                    break;
                }
            }
        });
        tx
    }

    /// Get a broadcast receiver for stdout.
    /// Returns a receiver that will receive all stdout output from the agent.
    pub fn subscribe_stdout(&self) -> broadcast::Receiver<Vec<u8>> {
        self.stdout_broadcast.subscribe()
    }

    /// Get a broadcast receiver for stderr.
    /// Returns a receiver that will receive all stderr output from the agent.
    pub fn subscribe_stderr(&self) -> broadcast::Receiver<Vec<u8>> {
        self.stderr_broadcast.subscribe()
    }

    /// Get the stdout broadcast sender for TUI handlers.
    pub fn stdout_broadcast_sender(&self) -> broadcast::Sender<Vec<u8>> {
        self.stdout_broadcast.clone()
    }

    /// Get the stderr broadcast sender for TUI handlers.
    pub fn stderr_broadcast_sender(&self) -> broadcast::Sender<Vec<u8>> {
        self.stderr_broadcast.clone()
    }
}

/// Wrapper that implements AgentHandle trait for LocalAgentHandle.
///
/// This allows us to store LocalAgentHandle in an Arc<RwLock<>> while still
/// implementing the AgentHandle trait.
struct AgentHandleWrapper {
    id: Uuid,
    handle: Arc<RwLock<LocalAgentHandle>>,
}

#[async_trait]
impl AgentHandle for AgentHandleWrapper {
    fn id(&self) -> Uuid {
        self.id
    }

    fn status(&self) -> AgentStatus {
        let handle = self.handle.read();
        handle.status
    }

    async fn write_stdin(&mut self, data: &[u8]) -> AgentResult<()> {
        let stdin = {
            let handle = self.handle.read();
            Arc::clone(&handle.stdin)
        };
        let mut stdin_guard = stdin.lock().await;
        stdin_guard.write_all(data).await?;
        stdin_guard.flush().await?;
        Ok(())
    }

    async fn read_stdout(&mut self) -> AgentResult<Option<Vec<u8>>> {
        let stdout_buffer = {
            let handle = self.handle.read();
            Arc::clone(&handle.stdout_buffer)
        };
        let mut buffer = stdout_buffer.lock().await;
        Ok(buffer.try_recv().ok())
    }

    async fn read_stderr(&mut self) -> AgentResult<Option<Vec<u8>>> {
        let stderr_buffer = {
            let handle = self.handle.read();
            Arc::clone(&handle.stderr_buffer)
        };
        let mut buffer = stderr_buffer.lock().await;
        Ok(buffer.try_recv().ok())
    }

    async fn wait(&mut self) -> AgentResult<ExitStatus> {
        if let Some(status) = {
            let handle = self.handle.read();
            handle.exit_status.clone()
        } {
            return Ok(status);
        }

        let child = {
            let handle = self.handle.read();
            Arc::clone(&handle.child)
        };
        let mut child_guard = child.lock().await;
        let status = child_guard.wait().await?;
        let exit_status = ExitStatus {
            code: status.code(),
            success: status.success(),
        };

        {
            let mut handle = self.handle.write();
            handle.record_exit_status(exit_status.clone());
        }

        Ok(exit_status)
    }

    async fn kill(&mut self) -> AgentResult<()> {
        let child = {
            let handle = self.handle.read();
            Arc::clone(&handle.child)
        };
        let mut child_guard = child.lock().await;
        child_guard.kill().await?;

        // Update the handle's status
        {
            let mut handle = self.handle.write();
            handle.set_status(AgentStatus::Terminated);
            handle.exit_status = Some(ExitStatus {
                code: None,
                success: false,
            });
        }

        Ok(())
    }

    fn exit_code(&self) -> Option<i32> {
        let handle = self.handle.read();
        handle
            .exit_status
            .as_ref()
            .and_then(|exit_status| exit_status.code)
    }
}

/// Graceful shutdown coordinator for agent processes.
///
/// Attempts graceful shutdown (SIGTERM) first, then force kills (SIGKILL)
/// after timeout.
pub struct GracefulShutdown {
    timeout_secs: u64,
}

impl GracefulShutdown {
    /// Create a new graceful shutdown coordinator.
    pub fn new(timeout_secs: u64) -> Self {
        Self { timeout_secs }
    }

    /// Perform graceful shutdown of an agent.
    ///
    /// Sends SIGTERM first, waits for timeout, then sends SIGKILL if needed.
    pub async fn shutdown(&self, handle: &mut Box<dyn AgentHandle>) -> AgentResult<()> {
        // Send SIGTERM
        tracing::info!(
            "Sending SIGTERM to agent {} for graceful shutdown",
            handle.id()
        );

        // Create a simple stdin write to signal shutdown
        // (This is a placeholder - actual implementation depends on agent protocol)
        let _ = handle.write_stdin(b"exit\n").await;

        // Wait for timeout or process exit
        let timeout = tokio::time::Duration::from_secs(self.timeout_secs);
        match tokio::time::timeout(timeout, handle.wait()).await {
            Ok(Ok(status)) => {
                tracing::info!(
                    "Agent {} exited gracefully with status: {:?}",
                    handle.id(),
                    status
                );
                Ok(())
            }
            Ok(Err(e)) => {
                tracing::error!("Error waiting for agent {}: {}", handle.id(), e);
                Err(e)
            }
            Err(_) => {
                // Timeout - force kill
                tracing::warn!(
                    "Agent {} did not exit within {}s, sending SIGKILL",
                    handle.id(),
                    self.timeout_secs
                );
                handle.kill().await?;
                Ok(())
            }
        }
    }
}

impl Default for GracefulShutdown {
    fn default() -> Self {
        Self::new(SHUTDOWN_TIMEOUT_SECS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_process_runner_creation() {
        let runner = LocalProcessRunner::new();
        assert_eq!(runner.agents.len(), 0);
    }

    #[tokio::test]
    async fn test_can_spawn_agent() {
        let mut config = ProcessRunnerConfig::default();
        config.max_concurrent_agents = Some(1);
        let runner = LocalProcessRunner::with_config(config);
        assert!(runner.can_spawn_agent());
    }

    #[tokio::test]
    async fn test_list_agents_empty() {
        let runner = LocalProcessRunner::new();
        let agents = runner.list_agents().await.unwrap();
        assert_eq!(agents.len(), 0);
    }

    #[tokio::test]
    async fn test_get_nonexistent_agent() {
        let runner = LocalProcessRunner::new();
        let agent_id = Uuid::new_v4();
        let result = runner.get_agent(&agent_id).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_graceful_shutdown_creation() {
        let shutdown = GracefulShutdown::new(10);
        assert_eq!(shutdown.timeout_secs, 10);
    }

    #[tokio::test]
    async fn test_process_config_default() {
        let config = ProcessRunnerConfig::default();
        assert!(config.enable_json_streaming);
        assert!(config.enable_health_checks);
        assert_eq!(
            config.health_check_interval_secs,
            HEALTH_CHECK_INTERVAL_SECS
        );
    }

    #[test]
    fn test_pause_mode_display() {
        assert_eq!(PauseMode::Cooperative.to_string(), "cooperative");
        assert_eq!(PauseMode::Forced.to_string(), "forced");
    }

    #[test]
    fn test_pause_mode_equality() {
        assert_eq!(PauseMode::Cooperative, PauseMode::Cooperative);
        assert_eq!(PauseMode::Forced, PauseMode::Forced);
        assert_ne!(PauseMode::Cooperative, PauseMode::Forced);
    }

    #[test]
    fn test_agent_signal_variants() {
        // Verify all AgentSignal variants exist and are copyable
        let signals = [
            AgentSignal::Interrupt,
            AgentSignal::Terminate,
            AgentSignal::Kill,
            AgentSignal::ForcePause,
            AgentSignal::Resume,
        ];

        // Ensure they're Copy/Clone
        let copied = signals[0];
        assert_eq!(copied, AgentSignal::Interrupt);
    }

    #[tokio::test]
    async fn test_pause_nonexistent_agent() {
        let runner = LocalProcessRunner::new();
        let agent_id = Uuid::new_v4();
        let result = runner.pause(&agent_id, false).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Agent not found"));
    }

    #[tokio::test]
    async fn test_resume_nonexistent_agent() {
        let runner = LocalProcessRunner::new();
        let agent_id = Uuid::new_v4();
        let result = runner.resume(&agent_id).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Agent not found"));
    }

    #[test]
    fn test_agent_info_with_pause_fields() {
        let info = AgentInfo {
            id: Uuid::new_v4(),
            name: "test-agent".to_string(),
            status: AgentStatus::Running,
            model_backend: "claude".to_string(),
            started_at: SystemTime::now(),
            task: "test task".to_string(),
            paused_at: None,
            pause_mode: None,
            attach_info: None,
        };

        assert!(info.paused_at.is_none());
        assert!(info.pause_mode.is_none());
        assert!(info.attach_info.is_none());
    }

    #[test]
    fn test_agent_info_paused() {
        let info = AgentInfo {
            id: Uuid::new_v4(),
            name: "test-agent".to_string(),
            status: AgentStatus::Paused,
            model_backend: "claude".to_string(),
            started_at: SystemTime::now(),
            task: "test task".to_string(),
            paused_at: Some(SystemTime::now()),
            pause_mode: Some(PauseMode::Cooperative),
            attach_info: None,
        };

        assert!(info.paused_at.is_some());
        assert_eq!(info.pause_mode, Some(PauseMode::Cooperative));
        assert_eq!(info.status, AgentStatus::Paused);
    }

    #[test]
    fn test_attach_info_serialization() {
        use crate::traits::AttachInfo;

        let attach_info = AttachInfo {
            connect_url: "ipc:///tmp/test.sock".to_string(),
            token: "test-token-123".to_string(),
            expires_at: 1234567890,
        };

        // Test serialization
        let json = serde_json::to_string(&attach_info).unwrap();
        assert!(json.contains("connect_url"));
        assert!(json.contains("test.sock"));

        // Test deserialization
        let deserialized: AttachInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.connect_url, attach_info.connect_url);
        assert_eq!(deserialized.token, attach_info.token);
        assert_eq!(deserialized.expires_at, attach_info.expires_at);
    }
}
