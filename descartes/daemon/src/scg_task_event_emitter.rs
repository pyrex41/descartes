//! SCG-based Task Event Emitter - File watching and event emission for SCG task files
//!
//! This module watches the .scud/tasks/tasks.scg file for changes and emits
//! events when tasks are created, updated, or deleted.
//!
//! Features:
//! - File-system watching using notify crate
//! - Detects changes by comparing with cached state
//! - Emits TaskEvent to EventBus for WebSocket subscribers
//! - Debouncing to handle rapid file saves
//! - Thread-safe with Arc/RwLock

use crate::events::{DescartesEvent, EventBus, TaskEvent, TaskEventType};
use chrono::Utc;
use descartes_core::{ScgTaskStorage, Task};
use notify::{Config, Event as NotifyEvent, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, RwLock};
use uuid::Uuid;

/// Configuration for SCG task event emitter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScgTaskEventEmitterConfig {
    /// Debounce interval in milliseconds
    pub debounce_interval_ms: u64,

    /// Include full task data in events
    pub include_task_data: bool,

    /// Enable verbose logging
    pub verbose_logging: bool,
}

impl Default for ScgTaskEventEmitterConfig {
    fn default() -> Self {
        Self {
            debounce_interval_ms: 100,
            include_task_data: true,
            verbose_logging: false,
        }
    }
}

/// SCG-based Task Event Emitter
/// Watches SCG files and emits events when tasks change
pub struct ScgTaskEventEmitter {
    /// SCG task storage
    storage: Arc<ScgTaskStorage>,

    /// Event bus for emitting events
    event_bus: Arc<EventBus>,

    /// Configuration
    config: ScgTaskEventEmitterConfig,

    /// Cache of previous task states for change detection
    task_cache: Arc<RwLock<HashMap<String, Task>>>,

    /// File watcher handle (kept alive)
    _watcher_handle: Arc<RwLock<Option<RecommendedWatcher>>>,

    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl ScgTaskEventEmitter {
    /// Create a new SCG task event emitter
    pub fn new(
        storage: Arc<ScgTaskStorage>,
        event_bus: Arc<EventBus>,
        config: ScgTaskEventEmitterConfig,
    ) -> Self {
        Self {
            storage,
            event_bus,
            config,
            task_cache: Arc::new(RwLock::new(HashMap::new())),
            _watcher_handle: Arc::new(RwLock::new(None)),
            shutdown_tx: None,
        }
    }

    /// Create with default configuration
    pub fn with_defaults(storage: Arc<ScgTaskStorage>, event_bus: Arc<EventBus>) -> Self {
        Self::new(storage, event_bus, ScgTaskEventEmitterConfig::default())
    }

    /// Initialize cache with current tasks
    pub async fn initialize_cache(&self) -> anyhow::Result<()> {
        // Refresh storage from disk
        self.storage.refresh_cache().await.map_err(|e| anyhow::anyhow!("{}", e))?;

        // Load all tasks
        let tasks = self.storage.get_all_tasks().await.map_err(|e| anyhow::anyhow!("{}", e))?;

        // Populate cache
        let mut cache = self.task_cache.write().await;
        for task in tasks {
            cache.insert(task.id.to_string(), task);
        }

        if self.config.verbose_logging {
            tracing::info!(
                "SCG task event emitter cache initialized with {} tasks",
                cache.len()
            );
        }

        Ok(())
    }

    /// Start watching for file changes
    /// Returns a handle that must be kept alive
    pub async fn start_watching(&mut self) -> anyhow::Result<()> {
        let project_root = self.storage.project_root().to_path_buf();
        let tasks_file = project_root.join(".scud/tasks/tasks.scg");

        if !tasks_file.exists()
            && self.config.verbose_logging {
                tracing::warn!(
                    "Tasks file does not exist: {}. Watching parent directory.",
                    tasks_file.display()
                );
            }

        // Create channel for file events
        let (tx, mut rx) = mpsc::channel::<()>(10);
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel::<()>(1);
        self.shutdown_tx = Some(shutdown_tx);

        // Set up watcher
        let tasks_dir = project_root.join(".scud/tasks");
        let tx_clone = tx.clone();

        let watcher = RecommendedWatcher::new(
            move |res: Result<NotifyEvent, notify::Error>| {
                if let Ok(event) = res {
                    // Filter for modify/create/remove events
                    if matches!(
                        event.kind,
                        notify::EventKind::Modify(_)
                            | notify::EventKind::Create(_)
                            | notify::EventKind::Remove(_)
                    ) {
                        // Check if it's the tasks.scg file
                        if event.paths.iter().any(|p| {
                            p.file_name()
                                .map(|n| n == "tasks.scg")
                                .unwrap_or(false)
                        }) {
                            let _ = tx_clone.blocking_send(());
                        }
                    }
                }
            },
            Config::default().with_poll_interval(Duration::from_millis(200)),
        )?;

        // Store watcher handle
        {
            let mut handle = self._watcher_handle.write().await;
            *handle = Some(watcher);
        }

        // Watch the directory
        {
            let mut handle = self._watcher_handle.write().await;
            if let Some(ref mut watcher) = *handle {
                if tasks_dir.exists() {
                    Watcher::watch(watcher, &tasks_dir, RecursiveMode::NonRecursive)?;
                    if self.config.verbose_logging {
                        tracing::info!("Watching directory: {}", tasks_dir.display());
                    }
                } else {
                    // Watch the .scud directory instead and wait for tasks/ to be created
                    let scud_dir = project_root.join(".scud");
                    if scud_dir.exists() {
                        Watcher::watch(watcher, &scud_dir, RecursiveMode::Recursive)?;
                        if self.config.verbose_logging {
                            tracing::info!("Watching directory: {}", scud_dir.display());
                        }
                    }
                }
            }
        }

        // Spawn event processing task
        let storage = self.storage.clone();
        let event_bus = self.event_bus.clone();
        let task_cache = self.task_cache.clone();
        let config = self.config.clone();
        let debounce_ms = self.config.debounce_interval_ms;

        tokio::spawn(async move {
            let mut last_process = std::time::Instant::now();

            loop {
                tokio::select! {
                    Some(()) = rx.recv() => {
                        // Debounce: wait for debounce interval before processing
                        let now = std::time::Instant::now();
                        if now.duration_since(last_process) < Duration::from_millis(debounce_ms) {
                            continue;
                        }
                        last_process = now;

                        // Small delay to let file writes complete
                        tokio::time::sleep(Duration::from_millis(50)).await;

                        // Process file changes
                        if let Err(e) = process_file_changes(
                            &storage,
                            &event_bus,
                            &task_cache,
                            &config,
                        ).await {
                            tracing::error!("Error processing SCG file changes: {}", e);
                        }
                    }
                    Some(()) = shutdown_rx.recv() => {
                        if config.verbose_logging {
                            tracing::info!("SCG file watcher shutting down");
                        }
                        break;
                    }
                    else => {
                        // All senders dropped
                        break;
                    }
                }
            }
        });

        Ok(())
    }

    /// Stop watching for file changes
    pub async fn stop_watching(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(()).await;
        }

        let mut handle = self._watcher_handle.write().await;
        *handle = None;
    }

    /// Manually trigger a refresh and emit events
    pub async fn refresh_and_emit(&self) -> anyhow::Result<()> {
        process_file_changes(
            &self.storage,
            &self.event_bus,
            &self.task_cache,
            &self.config,
        ).await
    }

    /// Get the event bus
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }

    /// Get the storage
    pub fn storage(&self) -> &Arc<ScgTaskStorage> {
        &self.storage
    }
}

/// Process file changes and emit events
async fn process_file_changes(
    storage: &Arc<ScgTaskStorage>,
    event_bus: &Arc<EventBus>,
    task_cache: &Arc<RwLock<HashMap<String, Task>>>,
    config: &ScgTaskEventEmitterConfig,
) -> anyhow::Result<()> {
    // Refresh storage from disk
    storage.refresh_cache().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    // Load current tasks
    let current_tasks = storage.get_all_tasks().await.map_err(|e| anyhow::anyhow!("{}", e))?;

    // Build current map
    let current_map: HashMap<String, Task> = current_tasks
        .into_iter()
        .map(|t| (t.id.to_string(), t))
        .collect();

    // Get previous state
    let mut previous_map = task_cache.write().await;

    // Detect changes
    let mut events = Vec::new();
    let timestamp = Utc::now().timestamp();

    // Check for created and updated tasks
    for (id, task) in &current_map {
        match previous_map.get(id) {
            None => {
                // New task
                events.push(create_task_event(
                    id.clone(),
                    TaskEventType::Created,
                    if config.include_task_data { Some(task.clone()) } else { None },
                    None,
                    format!("{:?}", task.status),
                    timestamp,
                ));
                if config.verbose_logging {
                    tracing::debug!("Task created: {}", id);
                }
            }
            Some(prev) => {
                // Check if updated
                if prev.status != task.status
                    || prev.title != task.title
                    || prev.priority != task.priority
                    || prev.updated_at != task.updated_at
                {
                    events.push(create_task_event(
                        id.clone(),
                        TaskEventType::Progress,
                        if config.include_task_data { Some(task.clone()) } else { None },
                        Some(format!("{:?}", prev.status)),
                        format!("{:?}", task.status),
                        timestamp,
                    ));
                    if config.verbose_logging {
                        tracing::debug!("Task updated: {} ({:?} -> {:?})", id, prev.status, task.status);
                    }
                }
            }
        }
    }

    // Check for deleted tasks
    for (id, _task) in previous_map.iter() {
        if !current_map.contains_key(id) {
            events.push(create_task_event(
                id.clone(),
                TaskEventType::Cancelled,
                None,
                None,
                "Deleted".to_string(),
                timestamp,
            ));
            if config.verbose_logging {
                tracing::debug!("Task deleted: {}", id);
            }
        }
    }

    // Update cache
    *previous_map = current_map;

    // Emit events
    for event in events {
        event_bus.publish(event).await;
    }

    Ok(())
}

/// Create a TaskEvent wrapped in DescartesEvent
fn create_task_event(
    task_id: String,
    event_type: TaskEventType,
    task: Option<Task>,
    previous_status: Option<String>,
    new_status: String,
    timestamp: i64,
) -> DescartesEvent {
    let data = json!({
        "task": task,
        "previous_status": previous_status,
        "new_status": new_status,
        "timestamp": timestamp,
    });

    DescartesEvent::TaskEvent(TaskEvent {
        id: Uuid::new_v4().to_string(),
        task_id,
        agent_id: None,
        timestamp: Utc::now(),
        event_type,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use descartes_core::TaskStatus;
    use tempfile::TempDir;

    async fn setup_test_emitter() -> (ScgTaskEventEmitter, Arc<EventBus>, TempDir) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");

        // Create .scud directory structure
        let scud_dir = temp_dir.path().join(".scud/tasks");
        std::fs::create_dir_all(&scud_dir).expect("Failed to create scud dir");

        // Create empty tasks.scg
        let tasks_file = scud_dir.join("tasks.scg");
        std::fs::write(&tasks_file, "").expect("Failed to create tasks.scg");

        // Create workflow-state.json
        let workflow_file = temp_dir.path().join(".scud/workflow-state.json");
        std::fs::write(
            &workflow_file,
            r#"{"active_epic": null, "updated_at": null}"#,
        )
        .expect("Failed to create workflow-state.json");

        let storage = Arc::new(ScgTaskStorage::new(temp_dir.path()));
        let event_bus = Arc::new(EventBus::new());

        let config = ScgTaskEventEmitterConfig {
            debounce_interval_ms: 10,
            include_task_data: true,
            verbose_logging: true,
        };

        let emitter = ScgTaskEventEmitter::new(storage, event_bus.clone(), config);

        (emitter, event_bus, temp_dir)
    }

    #[tokio::test]
    async fn test_emitter_creation() {
        let (emitter, _event_bus, _temp_dir) = setup_test_emitter().await;

        // Initialize cache (should work even with empty tasks)
        let result = emitter.initialize_cache().await;
        assert!(result.is_ok());
    }
}
