//! SCUD Bridge - Async communication layer between Iced GUI and SCUD CLI
//!
//! Provides event-driven communication with SCUD through subprocess calls
//! and JSON parsing for real-time updates.

use serde::Deserialize;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::state::TaskInfo;

/// Events emitted by SCUD execution
///
/// These events are sent from the ScudBridge to the GUI to update state.
#[derive(Debug, Clone)]
pub enum ScudEvent {
    /// Tasks loaded from SCUD storage
    TasksLoaded(Vec<TaskInfo>),

    /// Waves computed from task dependencies
    WavesComputed(Vec<Vec<String>>),

    /// Swarm execution started
    SwarmStarted { tag: String, total_waves: usize },

    /// A wave of tasks started
    WaveStarted { wave: usize, tasks: Vec<String> },

    /// Individual task started execution
    TaskStarted { task_id: String },

    /// Task output received
    TaskOutput { task_id: String, text: String },

    /// Individual task completed
    TaskCompleted { task_id: String, success: bool },

    /// Validation started
    ValidationStarted,

    /// Validation completed
    ValidationCompleted { passed: bool, output: String },

    /// Wave completed
    WaveCompleted { wave: usize },

    /// Swarm execution completed
    SwarmCompleted { success: bool },

    /// Generic output (for streaming text)
    Output(String),

    /// Error occurred
    Error(String),
}

/// Commands to send to SCUD
///
/// These commands are sent from the GUI to the ScudBridge.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum ScudCommand {
    /// Load tasks from SCUD, optionally filtered by tag
    LoadTasks { tag: Option<String> },

    /// Compute execution waves for a tag
    ComputeWaves { tag: String },

    /// Start swarm execution
    StartSwarm {
        tag: String,
        harness: String,
        round_size: usize,
    },

    /// Stop the currently running swarm
    StopSwarm,

    /// Mark a task as complete
    CompleteTask { task_id: String },

    /// Mark a task as blocked
    BlockTask { task_id: String },
}

/// JSON event format from SCUD CLI when running with --json-events
#[derive(Debug, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
enum ScudJsonEvent {
    SwarmStarted {
        tag: String,
        total_waves: usize,
    },
    WaveStarted {
        wave: usize,
        tasks: Vec<String>,
    },
    TaskStarted {
        task_id: String,
    },
    TaskOutput {
        task_id: String,
        text: String,
    },
    TaskCompleted {
        task_id: String,
        success: bool,
    },
    ValidationStarted,
    ValidationCompleted {
        passed: bool,
        #[serde(default)]
        output: String,
    },
    WaveCompleted {
        wave: usize,
    },
    SwarmCompleted {
        success: bool,
    },
}

impl From<ScudJsonEvent> for ScudEvent {
    fn from(json_event: ScudJsonEvent) -> Self {
        match json_event {
            ScudJsonEvent::SwarmStarted { tag, total_waves } => {
                ScudEvent::SwarmStarted { tag, total_waves }
            }
            ScudJsonEvent::WaveStarted { wave, tasks } => ScudEvent::WaveStarted { wave, tasks },
            ScudJsonEvent::TaskStarted { task_id } => ScudEvent::TaskStarted { task_id },
            ScudJsonEvent::TaskOutput { task_id, text } => ScudEvent::TaskOutput { task_id, text },
            ScudJsonEvent::TaskCompleted { task_id, success } => {
                ScudEvent::TaskCompleted { task_id, success }
            }
            ScudJsonEvent::ValidationStarted => ScudEvent::ValidationStarted,
            ScudJsonEvent::ValidationCompleted { passed, output } => {
                ScudEvent::ValidationCompleted { passed, output }
            }
            ScudJsonEvent::WaveCompleted { wave } => ScudEvent::WaveCompleted { wave },
            ScudJsonEvent::SwarmCompleted { success } => ScudEvent::SwarmCompleted { success },
        }
    }
}

/// JSON task format from SCUD CLI when running with --json
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ScudJsonTask {
    id: String,
    title: String,
    status: String,
    #[serde(default)]
    dependencies: Vec<String>,
    #[serde(default)]
    priority: Option<String>,
    #[serde(default)]
    complexity: Option<usize>,
}

impl From<ScudJsonTask> for TaskInfo {
    fn from(task: ScudJsonTask) -> Self {
        TaskInfo {
            id: task.id,
            title: task.title,
            status: task.status,
        }
    }
}

/// Bridge between Iced GUI and SCUD CLI
///
/// Handles subprocess spawning, JSON parsing, and channel communication
/// for real-time updates during SCUD execution.
pub struct ScudBridge {
    /// Sender for events to GUI
    event_tx: mpsc::Sender<ScudEvent>,

    /// Receiver for commands from GUI
    command_rx: mpsc::Receiver<ScudCommand>,

    /// Handle to current swarm process (for cancellation)
    swarm_handle: Option<tokio::process::Child>,
}

impl ScudBridge {
    /// Create a new ScudBridge with the given channel endpoints
    pub fn new(
        event_tx: mpsc::Sender<ScudEvent>,
        command_rx: mpsc::Receiver<ScudCommand>,
    ) -> Self {
        Self {
            event_tx,
            command_rx,
            swarm_handle: None,
        }
    }

    /// Create a new ScudBridge and return the channel handles for the GUI
    ///
    /// Returns (bridge, command_sender, event_receiver)
    pub fn create() -> (
        Self,
        mpsc::Sender<ScudCommand>,
        mpsc::Receiver<ScudEvent>,
    ) {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (command_tx, command_rx) = mpsc::channel(100);

        let bridge = Self::new(event_tx, command_rx);
        (bridge, command_tx, event_rx)
    }

    /// Main run loop - processes commands from the GUI
    pub async fn run(mut self) {
        info!("ScudBridge started");

        while let Some(cmd) = self.command_rx.recv().await {
            debug!("ScudBridge received command: {:?}", cmd);

            match cmd {
                ScudCommand::LoadTasks { tag } => {
                    self.load_tasks(tag).await;
                }
                ScudCommand::ComputeWaves { tag } => {
                    self.compute_waves(&tag).await;
                }
                ScudCommand::StartSwarm {
                    tag,
                    harness,
                    round_size,
                } => {
                    self.run_swarm(&tag, &harness, round_size).await;
                }
                ScudCommand::StopSwarm => {
                    self.stop_swarm().await;
                }
                ScudCommand::CompleteTask { task_id } => {
                    self.complete_task(&task_id).await;
                }
                ScudCommand::BlockTask { task_id } => {
                    self.block_task(&task_id).await;
                }
            }
        }

        info!("ScudBridge shutting down");
    }

    /// Load tasks from SCUD storage
    ///
    /// Calls `scud list --json` and parses the output
    async fn load_tasks(&self, tag: Option<String>) {
        let mut args = vec!["list", "--json"];
        let tag_str;

        if let Some(ref t) = tag {
            tag_str = t.clone();
            args.push("--tag");
            args.push(&tag_str);
        }

        debug!("Running: scud {}", args.join(" "));

        match Command::new("scud").args(&args).output().await {
            Ok(output) => {
                if output.status.success() {
                    match serde_json::from_slice::<Vec<ScudJsonTask>>(&output.stdout) {
                        Ok(tasks) => {
                            let task_infos: Vec<TaskInfo> =
                                tasks.into_iter().map(TaskInfo::from).collect();
                            let _ = self.event_tx.send(ScudEvent::TasksLoaded(task_infos)).await;
                        }
                        Err(e) => {
                            // If JSON parsing fails, try to parse as newline-delimited JSON
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            let tasks: Vec<TaskInfo> = stdout
                                .lines()
                                .filter_map(|line| {
                                    serde_json::from_str::<ScudJsonTask>(line)
                                        .ok()
                                        .map(TaskInfo::from)
                                })
                                .collect();

                            if tasks.is_empty() {
                                warn!("Failed to parse SCUD JSON output: {}", e);
                                let _ = self
                                    .event_tx
                                    .send(ScudEvent::Error(format!(
                                        "Failed to parse task list: {}",
                                        e
                                    )))
                                    .await;
                            } else {
                                let _ = self.event_tx.send(ScudEvent::TasksLoaded(tasks)).await;
                            }
                        }
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("scud list failed: {}", stderr);
                    let _ = self
                        .event_tx
                        .send(ScudEvent::Error(format!("scud list failed: {}", stderr)))
                        .await;
                }
            }
            Err(e) => {
                error!("Failed to spawn scud: {}", e);
                let _ = self
                    .event_tx
                    .send(ScudEvent::Error(format!("Failed to run scud: {}", e)))
                    .await;
            }
        }
    }

    /// Compute execution waves for a tag
    ///
    /// Calls `scud waves --json --tag <tag>` and parses the output
    async fn compute_waves(&self, tag: &str) {
        let args = vec!["waves", "--json", "--tag", tag];

        debug!("Running: scud {}", args.join(" "));

        match Command::new("scud").args(&args).output().await {
            Ok(output) => {
                if output.status.success() {
                    match serde_json::from_slice::<Vec<Vec<String>>>(&output.stdout) {
                        Ok(waves) => {
                            let _ = self.event_tx.send(ScudEvent::WavesComputed(waves)).await;
                        }
                        Err(e) => {
                            warn!("Failed to parse waves JSON: {}", e);
                            let _ = self
                                .event_tx
                                .send(ScudEvent::Error(format!("Failed to parse waves: {}", e)))
                                .await;
                        }
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("scud waves failed: {}", stderr);
                    let _ = self
                        .event_tx
                        .send(ScudEvent::Error(format!("scud waves failed: {}", stderr)))
                        .await;
                }
            }
            Err(e) => {
                error!("Failed to spawn scud: {}", e);
                let _ = self
                    .event_tx
                    .send(ScudEvent::Error(format!("Failed to run scud: {}", e)))
                    .await;
            }
        }
    }

    /// Run swarm execution with event streaming
    ///
    /// Spawns `scud swarm --tag <tag> --harness <harness> --json-events`
    /// and streams events as they occur
    async fn run_swarm(&mut self, tag: &str, harness: &str, round_size: usize) {
        let round_size_str = round_size.to_string();
        let args = vec![
            "swarm",
            "--tag",
            tag,
            "--harness",
            harness,
            "--round-size",
            &round_size_str,
            "--json-events",
        ];

        info!("Starting swarm: scud {}", args.join(" "));

        match Command::new("scud")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                // Take stdout for event streaming
                if let Some(stdout) = child.stdout.take() {
                    let event_tx = self.event_tx.clone();
                    let reader = BufReader::new(stdout);
                    let mut lines = reader.lines();

                    // Stream events from stdout
                    while let Ok(Some(line)) = lines.next_line().await {
                        // Try to parse as JSON event
                        if let Ok(event) = serde_json::from_str::<ScudJsonEvent>(&line) {
                            let scud_event: ScudEvent = event.into();
                            if event_tx.send(scud_event).await.is_err() {
                                warn!("Event channel closed");
                                break;
                            }
                        } else {
                            // Non-JSON line - send as generic output
                            if !line.trim().is_empty() {
                                let _ = event_tx.send(ScudEvent::Output(line)).await;
                            }
                        }
                    }
                }

                // Wait for process to complete
                match child.wait().await {
                    Ok(status) => {
                        if status.success() {
                            info!("Swarm completed successfully");
                        } else {
                            warn!("Swarm exited with status: {}", status);
                        }
                    }
                    Err(e) => {
                        error!("Error waiting for swarm process: {}", e);
                        let _ = self
                            .event_tx
                            .send(ScudEvent::Error(format!("Swarm process error: {}", e)))
                            .await;
                    }
                }
            }
            Err(e) => {
                error!("Failed to spawn swarm: {}", e);
                let _ = self
                    .event_tx
                    .send(ScudEvent::Error(format!("Failed to start swarm: {}", e)))
                    .await;
            }
        }
    }

    /// Stop the currently running swarm
    async fn stop_swarm(&mut self) {
        if let Some(ref mut handle) = self.swarm_handle {
            info!("Stopping swarm process");
            if let Err(e) = handle.kill().await {
                warn!("Failed to kill swarm process: {}", e);
            }
            self.swarm_handle = None;
        }
    }

    /// Mark a task as complete
    async fn complete_task(&self, task_id: &str) {
        let args = vec!["set-status", task_id, "done"];

        debug!("Running: scud {}", args.join(" "));

        match Command::new("scud").args(&args).output().await {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("scud set-status failed: {}", stderr);
                    let _ = self
                        .event_tx
                        .send(ScudEvent::Error(format!(
                            "Failed to complete task: {}",
                            stderr
                        )))
                        .await;
                } else {
                    info!("Task {} marked as done", task_id);
                }
            }
            Err(e) => {
                error!("Failed to spawn scud: {}", e);
                let _ = self
                    .event_tx
                    .send(ScudEvent::Error(format!("Failed to run scud: {}", e)))
                    .await;
            }
        }
    }

    /// Mark a task as blocked
    async fn block_task(&self, task_id: &str) {
        let args = vec!["set-status", task_id, "blocked"];

        debug!("Running: scud {}", args.join(" "));

        match Command::new("scud").args(&args).output().await {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    error!("scud set-status failed: {}", stderr);
                    let _ = self
                        .event_tx
                        .send(ScudEvent::Error(format!(
                            "Failed to block task: {}",
                            stderr
                        )))
                        .await;
                } else {
                    info!("Task {} marked as blocked", task_id);
                }
            }
            Err(e) => {
                error!("Failed to spawn scud: {}", e);
                let _ = self
                    .event_tx
                    .send(ScudEvent::Error(format!("Failed to run scud: {}", e)))
                    .await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scud_json_event_parsing() {
        let swarm_started = r#"{"event": "swarm_started", "tag": "feature", "total_waves": 3}"#;
        let parsed: ScudJsonEvent = serde_json::from_str(swarm_started).unwrap();
        match parsed {
            ScudJsonEvent::SwarmStarted { tag, total_waves } => {
                assert_eq!(tag, "feature");
                assert_eq!(total_waves, 3);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_task_started_parsing() {
        let task_started = r#"{"event": "task_started", "task_id": "1.2"}"#;
        let parsed: ScudJsonEvent = serde_json::from_str(task_started).unwrap();
        match parsed {
            ScudJsonEvent::TaskStarted { task_id } => {
                assert_eq!(task_id, "1.2");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_task_completed_parsing() {
        let task_completed = r#"{"event": "task_completed", "task_id": "1", "success": true}"#;
        let parsed: ScudJsonEvent = serde_json::from_str(task_completed).unwrap();
        match parsed {
            ScudJsonEvent::TaskCompleted { task_id, success } => {
                assert_eq!(task_id, "1");
                assert!(success);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_validation_completed_parsing() {
        let validation =
            r#"{"event": "validation_completed", "passed": false, "output": "Build failed"}"#;
        let parsed: ScudJsonEvent = serde_json::from_str(validation).unwrap();
        match parsed {
            ScudJsonEvent::ValidationCompleted { passed, output } => {
                assert!(!passed);
                assert_eq!(output, "Build failed");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_wave_events_parsing() {
        let wave_started = r#"{"event": "wave_started", "wave": 0, "tasks": ["1", "2"]}"#;
        let parsed: ScudJsonEvent = serde_json::from_str(wave_started).unwrap();
        match parsed {
            ScudJsonEvent::WaveStarted { wave, tasks } => {
                assert_eq!(wave, 0);
                assert_eq!(tasks, vec!["1", "2"]);
            }
            _ => panic!("Wrong event type"),
        }

        let wave_completed = r#"{"event": "wave_completed", "wave": 0}"#;
        let parsed: ScudJsonEvent = serde_json::from_str(wave_completed).unwrap();
        match parsed {
            ScudJsonEvent::WaveCompleted { wave } => {
                assert_eq!(wave, 0);
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[test]
    fn test_task_info_from_json() {
        let json_task = ScudJsonTask {
            id: "1".to_string(),
            title: "Test task".to_string(),
            status: "Pending".to_string(),
            dependencies: vec!["0".to_string()],
            priority: Some("High".to_string()),
            complexity: Some(3),
        };

        let task_info: TaskInfo = json_task.into();
        assert_eq!(task_info.id, "1");
        assert_eq!(task_info.title, "Test task");
        assert_eq!(task_info.status, "Pending");
    }

    #[test]
    fn test_scud_event_conversion() {
        let json_event = ScudJsonEvent::TaskStarted {
            task_id: "test-123".to_string(),
        };

        let scud_event: ScudEvent = json_event.into();
        match scud_event {
            ScudEvent::TaskStarted { task_id } => {
                assert_eq!(task_id, "test-123");
            }
            _ => panic!("Wrong event type"),
        }
    }
}
