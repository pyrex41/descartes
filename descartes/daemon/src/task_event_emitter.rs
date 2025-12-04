//! Task Event Emitter - Real-time SQLite Change Detection and Event Emission
//!
//! This module provides a wrapper around the StateStore that detects changes
//! to tasks in the SQLite database and emits events to the EventBus for
//! real-time GUI updates.
//!
//! Features:
//! - Detects INSERT, UPDATE, DELETE operations on tasks
//! - Emits TaskEvent to EventBus for WebSocket subscribers
//! - Debouncing to prevent event flooding
//! - Includes task data in events
//! - Thread-safe with Arc/Mutex

use crate::events::{DescartesEvent, EventBus, TaskEvent, TaskEventType};
use chrono::Utc;
use descartes_core::errors::StateStoreResult;
use descartes_core::traits::{StateStore, Task};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use uuid::Uuid;

/// Configuration for task event emitter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskEventEmitterConfig {
    /// Enable debouncing to prevent event flooding
    pub enable_debouncing: bool,

    /// Debounce interval in milliseconds
    pub debounce_interval_ms: u64,

    /// Include full task data in events (vs just task ID)
    pub include_task_data: bool,

    /// Enable verbose logging
    pub verbose_logging: bool,
}

impl Default for TaskEventEmitterConfig {
    fn default() -> Self {
        Self {
            enable_debouncing: true,
            debounce_interval_ms: 100,
            include_task_data: true,
            verbose_logging: false,
        }
    }
}

/// Task change event with full details
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "change_type")]
pub enum TaskChangeEvent {
    /// Task was created
    Created {
        task_id: String,
        task: Option<Task>,
        timestamp: i64,
    },
    /// Task was updated
    Updated {
        task_id: String,
        task: Option<Task>,
        previous_status: Option<String>,
        new_status: String,
        timestamp: i64,
    },
    /// Task was deleted
    Deleted { task_id: String, timestamp: i64 },
}

/// Debounce state for a specific task
#[derive(Debug, Clone)]
struct DebounceState {
    last_event_time: Instant,
    pending_event: Option<TaskChangeEvent>,
}

/// Task Event Emitter - wraps StateStore and emits events on changes
pub struct TaskEventEmitter {
    /// Underlying state store
    state_store: Arc<dyn StateStore>,

    /// Event bus for emitting events
    event_bus: Arc<EventBus>,

    /// Configuration
    config: TaskEventEmitterConfig,

    /// Debounce state per task
    debounce_state: Arc<RwLock<HashMap<String, DebounceState>>>,

    /// Cache of previous task states for change detection
    task_cache: Arc<RwLock<HashMap<String, Task>>>,
}

impl TaskEventEmitter {
    /// Create a new task event emitter
    pub fn new(
        state_store: Arc<dyn StateStore>,
        event_bus: Arc<EventBus>,
        config: TaskEventEmitterConfig,
    ) -> Self {
        Self {
            state_store,
            event_bus,
            config,
            debounce_state: Arc::new(RwLock::new(HashMap::new())),
            task_cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create with default configuration
    pub fn with_defaults(state_store: Arc<dyn StateStore>, event_bus: Arc<EventBus>) -> Self {
        Self::new(state_store, event_bus, TaskEventEmitterConfig::default())
    }

    /// Save a task and emit appropriate events
    pub async fn save_task(&self, task: &Task) -> StateStoreResult<()> {
        let task_id = task.id.to_string();

        // Check if task exists in cache (determines if this is INSERT or UPDATE)
        let is_new_task = {
            let cache = self.task_cache.read().await;
            !cache.contains_key(&task_id)
        };

        // Get previous task state for comparison
        let previous_task = {
            let cache = self.task_cache.read().await;
            cache.get(&task_id).cloned()
        };

        // Save to database
        self.state_store.save_task(task).await?;

        // Update cache
        {
            let mut cache = self.task_cache.write().await;
            cache.insert(task_id.clone(), task.clone());
        }

        // Determine event type and emit
        let event = if is_new_task {
            // This is a new task (INSERT)
            TaskChangeEvent::Created {
                task_id: task_id.clone(),
                task: if self.config.include_task_data {
                    Some(task.clone())
                } else {
                    None
                },
                timestamp: Utc::now().timestamp(),
            }
        } else {
            // This is an update (UPDATE)
            let previous_status = previous_task.as_ref().map(|t| format!("{:?}", t.status));
            let new_status = format!("{:?}", task.status);

            TaskChangeEvent::Updated {
                task_id: task_id.clone(),
                task: if self.config.include_task_data {
                    Some(task.clone())
                } else {
                    None
                },
                previous_status,
                new_status,
                timestamp: Utc::now().timestamp(),
            }
        };

        // Emit event (with debouncing if enabled)
        self.emit_task_event(event).await;

        Ok(())
    }

    /// Delete a task and emit event
    pub async fn delete_task(&self, task_id: &Uuid) -> StateStoreResult<()> {
        let task_id_str = task_id.to_string();

        // Note: We don't have a delete_task method in the StateStore trait
        // This is a placeholder for when it's added
        // For now, we'll just emit the event

        // Remove from cache
        {
            let mut cache = self.task_cache.write().await;
            cache.remove(&task_id_str);
        }

        // Emit deletion event
        let event = TaskChangeEvent::Deleted {
            task_id: task_id_str,
            timestamp: Utc::now().timestamp(),
        };

        self.emit_task_event(event).await;

        Ok(())
    }

    /// Get task by ID (pass-through to state store)
    pub async fn get_task(&self, task_id: &Uuid) -> StateStoreResult<Option<Task>> {
        self.state_store.get_task(task_id).await
    }

    /// Get all tasks (pass-through to state store)
    pub async fn get_tasks(&self) -> StateStoreResult<Vec<Task>> {
        self.state_store.get_tasks().await
    }

    /// Initialize the cache with existing tasks
    pub async fn initialize_cache(&self) -> StateStoreResult<()> {
        let tasks = self.state_store.get_tasks().await?;

        let mut cache = self.task_cache.write().await;
        for task in tasks {
            cache.insert(task.id.to_string(), task);
        }

        if self.config.verbose_logging {
            tracing::info!(
                "Task event emitter cache initialized with {} tasks",
                cache.len()
            );
        }

        Ok(())
    }

    /// Emit a task event to the event bus
    async fn emit_task_event(&self, change_event: TaskChangeEvent) {
        // Apply debouncing if enabled
        if self.config.enable_debouncing {
            let task_id = match &change_event {
                TaskChangeEvent::Created { task_id, .. } => task_id.clone(),
                TaskChangeEvent::Updated { task_id, .. } => task_id.clone(),
                TaskChangeEvent::Deleted { task_id, .. } => task_id.clone(),
            };

            let should_emit = self
                .should_emit_after_debounce(&task_id, &change_event)
                .await;

            if !should_emit {
                if self.config.verbose_logging {
                    tracing::debug!("Event for task {} debounced", task_id);
                }
                return;
            }
        }

        // Convert to DescartesEvent
        let descartes_event = self.convert_to_descartes_event(change_event);

        // Publish to event bus
        self.event_bus.publish(descartes_event).await;

        if self.config.verbose_logging {
            tracing::debug!("Task event emitted to event bus");
        }
    }

    /// Check if event should be emitted after debouncing
    async fn should_emit_after_debounce(&self, task_id: &str, event: &TaskChangeEvent) -> bool {
        let mut debounce_state = self.debounce_state.write().await;
        let now = Instant::now();
        let debounce_duration = Duration::from_millis(self.config.debounce_interval_ms);

        if let Some(state) = debounce_state.get_mut(task_id) {
            let elapsed = now.duration_since(state.last_event_time);

            if elapsed < debounce_duration {
                // Update pending event and don't emit yet
                state.pending_event = Some(event.clone());
                return false;
            } else {
                // Enough time has passed, emit the event
                state.last_event_time = now;
                state.pending_event = None;
                return true;
            }
        } else {
            // First event for this task
            debounce_state.insert(
                task_id.to_string(),
                DebounceState {
                    last_event_time: now,
                    pending_event: None,
                },
            );
            return true;
        }
    }

    /// Convert TaskChangeEvent to DescartesEvent
    fn convert_to_descartes_event(&self, change_event: TaskChangeEvent) -> DescartesEvent {
        let (task_id, event_type, data) = match change_event {
            TaskChangeEvent::Created {
                task_id,
                task,
                timestamp,
            } => {
                let data = json!({
                    "change_type": "created",
                    "task": task,
                    "timestamp": timestamp,
                });
                (task_id, TaskEventType::Created, data)
            }
            TaskChangeEvent::Updated {
                task_id,
                task,
                previous_status,
                new_status,
                timestamp,
            } => {
                let data = json!({
                    "change_type": "updated",
                    "task": task,
                    "previous_status": previous_status,
                    "new_status": new_status,
                    "timestamp": timestamp,
                });
                (task_id, TaskEventType::Progress, data)
            }
            TaskChangeEvent::Deleted { task_id, timestamp } => {
                let data = json!({
                    "change_type": "deleted",
                    "timestamp": timestamp,
                });
                (task_id, TaskEventType::Cancelled, data)
            }
        };

        DescartesEvent::TaskEvent(TaskEvent {
            id: Uuid::new_v4().to_string(),
            task_id,
            agent_id: None,
            timestamp: Utc::now(),
            event_type,
            data,
        })
    }

    /// Flush pending debounced events
    pub async fn flush_debounced_events(&self) {
        let mut debounce_state = self.debounce_state.write().await;

        for (task_id, state) in debounce_state.iter_mut() {
            if let Some(pending_event) = state.pending_event.take() {
                let descartes_event = self.convert_to_descartes_event(pending_event);
                self.event_bus.publish(descartes_event).await;

                if self.config.verbose_logging {
                    tracing::debug!("Flushed pending event for task {}", task_id);
                }
            }
        }
    }

    /// Get statistics about the emitter
    pub async fn get_statistics(&self) -> TaskEmitterStatistics {
        let cache_size = self.task_cache.read().await.len();
        let debounce_state_size = self.debounce_state.read().await.len();
        let pending_events = self
            .debounce_state
            .read()
            .await
            .values()
            .filter(|s| s.pending_event.is_some())
            .count();

        TaskEmitterStatistics {
            cached_tasks: cache_size,
            debounce_entries: debounce_state_size,
            pending_debounced_events: pending_events,
            config: self.config.clone(),
        }
    }

    /// Get the underlying state store
    pub fn state_store(&self) -> &Arc<dyn StateStore> {
        &self.state_store
    }

    /// Get the event bus
    pub fn event_bus(&self) -> &Arc<EventBus> {
        &self.event_bus
    }
}

/// Statistics about the task event emitter
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskEmitterStatistics {
    pub cached_tasks: usize,
    pub debounce_entries: usize,
    pub pending_debounced_events: usize,
    pub config: TaskEventEmitterConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use descartes_core::state_store::SqliteStateStore;
    use descartes_core::traits::{StateStore, TaskComplexity, TaskPriority, TaskStatus};
    use tempfile::NamedTempFile;

    async fn setup_test_emitter() -> (
        TaskEventEmitter,
        Arc<SqliteStateStore>,
        Arc<EventBus>,
        NamedTempFile,
    ) {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap().to_string();

        let mut state_store = SqliteStateStore::new(&db_path, true)
            .await
            .expect("Failed to create state store");
        state_store
            .initialize()
            .await
            .expect("Failed to initialize");

        let state_store = Arc::new(state_store);
        let event_bus = Arc::new(EventBus::new());

        let config = TaskEventEmitterConfig {
            enable_debouncing: false, // Disable for tests
            include_task_data: true,
            verbose_logging: true,
            ..Default::default()
        };

        let emitter = TaskEventEmitter::new(
            state_store.clone() as Arc<dyn StateStore>,
            event_bus.clone(),
            config,
        );

        emitter
            .initialize_cache()
            .await
            .expect("Failed to initialize cache");

        (emitter, state_store, event_bus, temp_file)
    }

    #[tokio::test]
    async fn test_task_creation_event() {
        let (emitter, _, event_bus, _temp_file) = setup_test_emitter().await;

        // Subscribe to events
        let (_sub_id, mut rx) = event_bus.subscribe(None).await;

        // Create a task
        let task = Task {
            id: Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: Some("Test description".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };

        // Save task (should emit Created event)
        emitter.save_task(&task).await.expect("Failed to save task");

        // Receive event
        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("Timeout waiting for event")
            .expect("Failed to receive event");

        // Verify event type
        match event {
            DescartesEvent::TaskEvent(task_event) => {
                assert_eq!(task_event.task_id, task.id.to_string());
                assert_eq!(task_event.event_type, TaskEventType::Created);

                // Verify task data is included
                let change_type = task_event
                    .data
                    .get("change_type")
                    .unwrap()
                    .as_str()
                    .unwrap();
                assert_eq!(change_type, "created");
            }
            _ => panic!("Expected TaskEvent"),
        }
    }

    #[tokio::test]
    async fn test_task_update_event() {
        let (emitter, _, event_bus, _temp_file) = setup_test_emitter().await;

        // Subscribe to events
        let (_sub_id, mut rx) = event_bus.subscribe(None).await;

        // Create a task
        let mut task = Task {
            id: Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: Some("Test description".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };

        // Save task first time
        emitter.save_task(&task).await.expect("Failed to save task");

        // Consume creation event
        let _ = rx.recv().await;

        // Update task
        task.status = TaskStatus::InProgress;
        task.updated_at = Utc::now().timestamp();

        // Save task again (should emit Updated event)
        emitter.save_task(&task).await.expect("Failed to save task");

        // Receive update event
        let event = tokio::time::timeout(Duration::from_secs(1), rx.recv())
            .await
            .expect("Timeout waiting for event")
            .expect("Failed to receive event");

        // Verify event type
        match event {
            DescartesEvent::TaskEvent(task_event) => {
                assert_eq!(task_event.task_id, task.id.to_string());

                // Verify status change is captured
                let previous_status = task_event
                    .data
                    .get("previous_status")
                    .and_then(|v| v.as_str())
                    .unwrap();
                let new_status = task_event
                    .data
                    .get("new_status")
                    .and_then(|v| v.as_str())
                    .unwrap();

                assert_eq!(previous_status, "Todo");
                assert_eq!(new_status, "InProgress");
            }
            _ => panic!("Expected TaskEvent"),
        }
    }

    #[tokio::test]
    async fn test_debouncing() {
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let db_path = temp_file.path().to_str().unwrap().to_string();

        let mut store = SqliteStateStore::new(&db_path, true)
            .await
            .expect("Failed to create state store");
        store.initialize().await.expect("Failed to initialize");

        let state_store = Arc::new(store) as Arc<dyn StateStore>;
        let event_bus = Arc::new(EventBus::new());

        let config = TaskEventEmitterConfig {
            enable_debouncing: true,
            debounce_interval_ms: 100,
            include_task_data: true,
            verbose_logging: true,
        };

        let emitter = TaskEventEmitter::new(state_store, event_bus.clone(), config);
        emitter
            .initialize_cache()
            .await
            .expect("Failed to initialize cache");

        // Subscribe to events
        let (_sub_id, mut rx) = event_bus.subscribe(None).await;

        // Create a task
        let mut task = Task {
            id: Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: Some("Test description".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };

        // Save task multiple times rapidly
        for i in 0..5 {
            task.title = format!("Test Task {}", i);
            task.updated_at = Utc::now().timestamp();
            emitter.save_task(&task).await.expect("Failed to save task");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        // Should receive fewer events due to debouncing
        let mut event_count = 0;
        while let Ok(Ok(_)) = tokio::time::timeout(Duration::from_millis(200), rx.recv()).await {
            event_count += 1;
        }

        // Should be less than 5 events due to debouncing
        assert!(
            event_count < 5,
            "Expected debouncing to reduce events, got {}",
            event_count
        );

        // Keep temp_file alive until the end of the test
        drop(temp_file);
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let (emitter, _, _, _temp_file) = setup_test_emitter().await;

        // Create some tasks
        for i in 0..10 {
            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Test Task {}", i),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Moderate,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            };
            emitter.save_task(&task).await.expect("Failed to save task");
        }

        let stats = emitter.get_statistics().await;
        assert_eq!(stats.cached_tasks, 10);
    }
}
