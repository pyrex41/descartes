/// Event bus system for real-time event streaming to GUI clients
///
/// This module provides:
/// - Event types for agent, task, and system events
/// - Event bus for publishing and subscribing to events
/// - WebSocket-based event streaming
/// - Event filtering and routing
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Maximum number of events to buffer in the broadcast channel
const EVENT_CHANNEL_CAPACITY: usize = 1000;

/// System-wide event that can be subscribed to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum DescartesEvent {
    /// Agent-related events
    AgentEvent(AgentEvent),
    /// Task-related events
    TaskEvent(TaskEvent),
    /// Workflow events
    WorkflowEvent(WorkflowEvent),
    /// System events
    SystemEvent(SystemEvent),
    /// State change events
    StateEvent(StateEvent),
}

/// Agent lifecycle and status events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub id: String,
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: AgentEventType,
    pub data: serde_json::Value,
}

/// Types of agent events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventType {
    /// Agent was spawned
    Spawned,
    /// Agent started running
    Started,
    /// Agent status changed
    StatusChanged,
    /// Agent paused
    Paused,
    /// Agent resumed
    Resumed,
    /// Agent completed successfully
    Completed,
    /// Agent failed
    Failed,
    /// Agent was killed
    Killed,
    /// Agent log message
    Log,
    /// Agent metric update
    Metric,
}

/// Task execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub id: String,
    pub task_id: String,
    pub agent_id: Option<String>,
    pub timestamp: DateTime<Utc>,
    pub event_type: TaskEventType,
    pub data: serde_json::Value,
}

/// Types of task events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskEventType {
    /// Task created
    Created,
    /// Task started
    Started,
    /// Task progress update
    Progress,
    /// Task completed
    Completed,
    /// Task failed
    Failed,
    /// Task cancelled
    Cancelled,
}

/// Workflow execution events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowEvent {
    pub id: String,
    pub workflow_id: String,
    pub execution_id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: WorkflowEventType,
    pub data: serde_json::Value,
}

/// Types of workflow events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowEventType {
    /// Workflow started
    Started,
    /// Workflow step completed
    StepCompleted,
    /// Workflow completed
    Completed,
    /// Workflow failed
    Failed,
    /// Workflow paused
    Paused,
    /// Workflow resumed
    Resumed,
}

/// System-level events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemEvent {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: SystemEventType,
    pub data: serde_json::Value,
}

/// Types of system events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SystemEventType {
    /// Daemon started
    DaemonStarted,
    /// Daemon stopping
    DaemonStopping,
    /// Health check status
    HealthCheck,
    /// Metrics update
    MetricsUpdate,
    /// Connection established
    ConnectionEstablished,
    /// Connection closed
    ConnectionClosed,
    /// Error occurred
    Error,
}

/// State change events
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEvent {
    pub id: String,
    pub agent_id: Option<String>,
    pub key: String,
    pub timestamp: DateTime<Utc>,
    pub event_type: StateEventType,
    pub data: serde_json::Value,
}

/// Types of state events
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StateEventType {
    /// State created
    Created,
    /// State updated
    Updated,
    /// State deleted
    Deleted,
}

/// Event filter for subscribing to specific events
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventFilter {
    /// Filter by agent IDs (empty = all)
    #[serde(default)]
    pub agent_ids: Vec<String>,
    /// Filter by task IDs (empty = all)
    #[serde(default)]
    pub task_ids: Vec<String>,
    /// Filter by workflow IDs (empty = all)
    #[serde(default)]
    pub workflow_ids: Vec<String>,
    /// Filter by event categories
    #[serde(default)]
    pub event_categories: Vec<EventCategory>,
}

/// High-level event categories for filtering
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EventCategory {
    Agent,
    Task,
    Workflow,
    System,
    State,
}

impl EventFilter {
    /// Create a filter that matches all events
    pub fn all() -> Self {
        Self::default()
    }

    /// Create a filter for specific agent
    pub fn for_agent(agent_id: String) -> Self {
        Self {
            agent_ids: vec![agent_id],
            ..Default::default()
        }
    }

    /// Create a filter for specific task
    pub fn for_task(task_id: String) -> Self {
        Self {
            task_ids: vec![task_id],
            ..Default::default()
        }
    }

    /// Check if an event matches this filter
    pub fn matches(&self, event: &DescartesEvent) -> bool {
        // Check category filter
        if !self.event_categories.is_empty() {
            let category = match event {
                DescartesEvent::AgentEvent(_) => EventCategory::Agent,
                DescartesEvent::TaskEvent(_) => EventCategory::Task,
                DescartesEvent::WorkflowEvent(_) => EventCategory::Workflow,
                DescartesEvent::SystemEvent(_) => EventCategory::System,
                DescartesEvent::StateEvent(_) => EventCategory::State,
            };
            if !self.event_categories.contains(&category) {
                return false;
            }
        }

        // Check specific ID filters
        match event {
            DescartesEvent::AgentEvent(e) => {
                if !self.agent_ids.is_empty() && !self.agent_ids.contains(&e.agent_id) {
                    return false;
                }
            }
            DescartesEvent::TaskEvent(e) => {
                if !self.task_ids.is_empty() && !self.task_ids.contains(&e.task_id) {
                    return false;
                }
            }
            DescartesEvent::WorkflowEvent(e) => {
                if !self.workflow_ids.is_empty() && !self.workflow_ids.contains(&e.workflow_id) {
                    return false;
                }
            }
            _ => {}
        }

        true
    }
}

/// Subscription handle for managing event subscriptions
#[derive(Debug, Clone)]
pub struct EventSubscription {
    pub id: String,
    pub filter: EventFilter,
    pub created_at: DateTime<Utc>,
}

/// Event bus for publishing and subscribing to events
pub struct EventBus {
    /// Broadcast channel for events
    tx: broadcast::Sender<DescartesEvent>,
    /// Active subscriptions
    subscriptions: Arc<RwLock<HashMap<String, EventSubscription>>>,
    /// Event statistics
    stats: Arc<RwLock<EventBusStats>>,
}

/// Statistics about event bus usage
#[derive(Debug, Clone, Default)]
pub struct EventBusStats {
    pub total_events_published: u64,
    pub total_subscriptions: u64,
    pub active_subscriptions: usize,
    pub events_by_type: HashMap<String, u64>,
}

impl EventBus {
    /// Create a new event bus
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(EVENT_CHANNEL_CAPACITY);
        Self {
            tx,
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(EventBusStats::default())),
        }
    }

    /// Publish an event to all subscribers
    pub async fn publish(&self, event: DescartesEvent) {
        // Update statistics
        let mut stats = self.stats.write().await;
        stats.total_events_published += 1;

        let event_type = match &event {
            DescartesEvent::AgentEvent(e) => format!("agent:{:?}", e.event_type),
            DescartesEvent::TaskEvent(e) => format!("task:{:?}", e.event_type),
            DescartesEvent::WorkflowEvent(e) => format!("workflow:{:?}", e.event_type),
            DescartesEvent::SystemEvent(e) => format!("system:{:?}", e.event_type),
            DescartesEvent::StateEvent(e) => format!("state:{:?}", e.event_type),
        };
        *stats.events_by_type.entry(event_type).or_insert(0) += 1;
        drop(stats);

        // Publish to broadcast channel (ignore send errors if no subscribers)
        let _ = self.tx.send(event);
    }

    /// Subscribe to events with optional filter
    pub async fn subscribe(
        &self,
        filter: Option<EventFilter>,
    ) -> (String, broadcast::Receiver<DescartesEvent>) {
        let subscription_id = Uuid::new_v4().to_string();
        let filter = filter.unwrap_or_default();

        let subscription = EventSubscription {
            id: subscription_id.clone(),
            filter,
            created_at: Utc::now(),
        };

        self.subscriptions
            .write()
            .await
            .insert(subscription_id.clone(), subscription);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.total_subscriptions += 1;
        stats.active_subscriptions = self.subscriptions.read().await.len();
        drop(stats);

        let rx = self.tx.subscribe();
        (subscription_id, rx)
    }

    /// Unsubscribe from events
    pub async fn unsubscribe(&self, subscription_id: &str) {
        self.subscriptions.write().await.remove(subscription_id);

        // Update stats
        let mut stats = self.stats.write().await;
        stats.active_subscriptions = self.subscriptions.read().await.len();
    }

    /// Get current statistics
    pub async fn stats(&self) -> EventBusStats {
        self.stats.read().await.clone()
    }

    /// Get active subscription count
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper functions for creating events
impl AgentEvent {
    pub fn spawned(agent_id: String, data: serde_json::Value) -> DescartesEvent {
        DescartesEvent::AgentEvent(AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id,
            timestamp: Utc::now(),
            event_type: AgentEventType::Spawned,
            data,
        })
    }

    pub fn status_changed(agent_id: String, status: String) -> DescartesEvent {
        DescartesEvent::AgentEvent(AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id,
            timestamp: Utc::now(),
            event_type: AgentEventType::StatusChanged,
            data: serde_json::json!({ "status": status }),
        })
    }

    pub fn completed(agent_id: String, data: serde_json::Value) -> DescartesEvent {
        DescartesEvent::AgentEvent(AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id,
            timestamp: Utc::now(),
            event_type: AgentEventType::Completed,
            data,
        })
    }

    pub fn failed(agent_id: String, error: String) -> DescartesEvent {
        DescartesEvent::AgentEvent(AgentEvent {
            id: Uuid::new_v4().to_string(),
            agent_id,
            timestamp: Utc::now(),
            event_type: AgentEventType::Failed,
            data: serde_json::json!({ "error": error }),
        })
    }
}

impl TaskEvent {
    pub fn started(task_id: String, agent_id: Option<String>) -> DescartesEvent {
        DescartesEvent::TaskEvent(TaskEvent {
            id: Uuid::new_v4().to_string(),
            task_id,
            agent_id,
            timestamp: Utc::now(),
            event_type: TaskEventType::Started,
            data: serde_json::json!({}),
        })
    }

    pub fn progress(task_id: String, progress: f32, message: String) -> DescartesEvent {
        DescartesEvent::TaskEvent(TaskEvent {
            id: Uuid::new_v4().to_string(),
            task_id,
            agent_id: None,
            timestamp: Utc::now(),
            event_type: TaskEventType::Progress,
            data: serde_json::json!({
                "progress": progress,
                "message": message,
            }),
        })
    }

    pub fn completed(task_id: String, result: serde_json::Value) -> DescartesEvent {
        DescartesEvent::TaskEvent(TaskEvent {
            id: Uuid::new_v4().to_string(),
            task_id,
            agent_id: None,
            timestamp: Utc::now(),
            event_type: TaskEventType::Completed,
            data: result,
        })
    }
}

impl SystemEvent {
    pub fn daemon_started() -> DescartesEvent {
        DescartesEvent::SystemEvent(SystemEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: SystemEventType::DaemonStarted,
            data: serde_json::json!({}),
        })
    }

    pub fn metrics_update(metrics: serde_json::Value) -> DescartesEvent {
        DescartesEvent::SystemEvent(SystemEvent {
            id: Uuid::new_v4().to_string(),
            timestamp: Utc::now(),
            event_type: SystemEventType::MetricsUpdate,
            data: metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_filter_matches() {
        let filter = EventFilter::for_agent("agent-1".to_string());

        let event1 = AgentEvent::spawned("agent-1".to_string(), serde_json::json!({}));
        assert!(filter.matches(&event1));

        let event2 = AgentEvent::spawned("agent-2".to_string(), serde_json::json!({}));
        assert!(!filter.matches(&event2));
    }

    #[test]
    fn test_category_filter() {
        let filter = EventFilter {
            event_categories: vec![EventCategory::Agent],
            ..Default::default()
        };

        let agent_event = AgentEvent::spawned("agent-1".to_string(), serde_json::json!({}));
        assert!(filter.matches(&agent_event));

        let system_event = SystemEvent::daemon_started();
        assert!(!filter.matches(&system_event));
    }

    #[tokio::test]
    async fn test_event_bus_publish_subscribe() {
        let bus = EventBus::new();

        let (_sub_id, mut rx) = bus.subscribe(None).await;

        let event = AgentEvent::spawned("test-agent".to_string(), serde_json::json!({}));

        bus.publish(event.clone()).await;

        let received = rx.recv().await.unwrap();
        match received {
            DescartesEvent::AgentEvent(e) => {
                assert_eq!(e.agent_id, "test-agent");
            }
            _ => panic!("Wrong event type"),
        }
    }

    #[tokio::test]
    async fn test_event_bus_stats() {
        let bus = EventBus::new();

        bus.publish(AgentEvent::spawned(
            "agent-1".to_string(),
            serde_json::json!({}),
        ))
        .await;

        bus.publish(SystemEvent::daemon_started()).await;

        let stats = bus.stats().await;
        assert_eq!(stats.total_events_published, 2);
    }

    #[tokio::test]
    async fn test_subscription_management() {
        let bus = EventBus::new();

        let (sub_id, _rx) = bus.subscribe(None).await;
        assert_eq!(bus.subscription_count().await, 1);

        bus.unsubscribe(&sub_id).await;
        assert_eq!(bus.subscription_count().await, 0);
    }
}
