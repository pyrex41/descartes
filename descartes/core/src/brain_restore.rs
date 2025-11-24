/// Brain Restore - Event Sourcing and State Restoration for Agent Brains
///
/// This module provides comprehensive brain restoration functionality, enabling agents to:
/// - Load and replay events from agent history
/// - Restore brain state to any point in time
/// - Reconstruct thought history, decision trees, memory, and conversation state
/// - Handle event dependencies and maintain causality
/// - Validate state consistency after restoration
///
/// The restore system enables:
/// - Time-travel debugging
/// - State recovery from snapshots
/// - Audit trail analysis
/// - Disaster recovery

use crate::agent_history::{
    AgentHistoryEvent, AgentHistoryStore, HistoryEventType, HistoryQuery, HistorySnapshot,
};
use crate::errors::{StateStoreError, StateStoreResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// ============================================================================
// BRAIN STATE MODELS
// ============================================================================

/// Represents the complete brain state of an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainState {
    /// Agent identifier
    pub agent_id: String,

    /// Timestamp when this state was captured/restored
    pub timestamp: i64,

    /// Thought history
    pub thought_history: Vec<ThoughtEntry>,

    /// Decision tree nodes
    pub decision_tree: Vec<DecisionNode>,

    /// Memory/context entries
    pub memory: HashMap<String, Value>,

    /// Conversation state
    pub conversation_state: ConversationState,

    /// Current session ID
    pub session_id: Option<String>,

    /// Metadata about the state
    pub metadata: HashMap<String, Value>,

    /// Git commit hash at time of state
    pub git_commit: Option<String>,
}

impl BrainState {
    /// Create a new empty brain state
    pub fn new(agent_id: String) -> Self {
        Self {
            agent_id,
            timestamp: chrono::Utc::now().timestamp(),
            thought_history: Vec::new(),
            decision_tree: Vec::new(),
            memory: HashMap::new(),
            conversation_state: ConversationState::default(),
            session_id: None,
            metadata: HashMap::new(),
            git_commit: None,
        }
    }

    /// Check if the brain state is empty
    pub fn is_empty(&self) -> bool {
        self.thought_history.is_empty()
            && self.decision_tree.is_empty()
            && self.memory.is_empty()
            && self.conversation_state.messages.is_empty()
    }
}

/// A single thought entry in the agent's thought history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtEntry {
    pub thought_id: Uuid,
    pub timestamp: i64,
    pub content: String,
    pub thought_type: String,
    pub parent_thought_id: Option<Uuid>,
    pub metadata: Option<Value>,
}

/// A node in the decision tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionNode {
    pub decision_id: Uuid,
    pub timestamp: i64,
    pub decision_type: String,
    pub context: Value,
    pub outcome: Option<String>,
    pub parent_decision_id: Option<Uuid>,
    pub children: Vec<Uuid>,
}

/// Conversation state tracking messages and context
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConversationState {
    pub messages: Vec<MessageEntry>,
    pub current_turn: i64,
    pub context: HashMap<String, Value>,
}

/// A message in the conversation history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEntry {
    pub message_id: Uuid,
    pub timestamp: i64,
    pub role: String,
    pub content: String,
    pub metadata: Option<Value>,
}

// ============================================================================
// RESTORE OPTIONS AND CONFIGURATION
// ============================================================================

/// Options for configuring the restore operation
#[derive(Debug, Clone)]
pub struct RestoreOptions {
    /// Whether to validate state consistency after restore
    pub validate: bool,

    /// Whether to handle missing events gracefully
    pub skip_missing_events: bool,

    /// Whether to maintain strict causality
    pub strict_causality: bool,

    /// Maximum number of events to process
    pub max_events: Option<usize>,

    /// Whether to include metadata in restored state
    pub include_metadata: bool,

    /// Custom event filters
    pub event_filters: Vec<HistoryEventType>,
}

impl Default for RestoreOptions {
    fn default() -> Self {
        Self {
            validate: true,
            skip_missing_events: false,
            strict_causality: true,
            max_events: None,
            include_metadata: true,
            event_filters: Vec::new(),
        }
    }
}

impl RestoreOptions {
    /// Create restore options with validation enabled
    pub fn with_validation() -> Self {
        Self {
            validate: true,
            ..Default::default()
        }
    }

    /// Create restore options that skip validation
    pub fn without_validation() -> Self {
        Self {
            validate: false,
            ..Default::default()
        }
    }

    /// Create restore options with lenient error handling
    pub fn lenient() -> Self {
        Self {
            skip_missing_events: true,
            strict_causality: false,
            validate: false,
            ..Default::default()
        }
    }

    /// Set event type filters
    pub fn with_event_filters(mut self, filters: Vec<HistoryEventType>) -> Self {
        self.event_filters = filters;
        self
    }
}

// ============================================================================
// RESTORE RESULT AND STATISTICS
// ============================================================================

/// Result of a restore operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreResult {
    /// Whether the restore was successful
    pub success: bool,

    /// The restored brain state
    pub brain_state: Option<BrainState>,

    /// Number of events processed
    pub events_processed: usize,

    /// Number of events skipped
    pub events_skipped: usize,

    /// Validation errors encountered
    pub validation_errors: Vec<String>,

    /// Warnings during restore
    pub warnings: Vec<String>,

    /// Time taken for restore operation (milliseconds)
    pub duration_ms: u64,
}

impl RestoreResult {
    /// Create a successful restore result
    pub fn success(brain_state: BrainState, events_processed: usize, duration_ms: u64) -> Self {
        Self {
            success: true,
            brain_state: Some(brain_state),
            events_processed,
            events_skipped: 0,
            validation_errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms,
        }
    }

    /// Create a failed restore result
    pub fn failure(error: String) -> Self {
        Self {
            success: false,
            brain_state: None,
            events_processed: 0,
            events_skipped: 0,
            validation_errors: vec![error],
            warnings: Vec::new(),
            duration_ms: 0,
        }
    }
}

// ============================================================================
// BRAIN RESTORE TRAIT
// ============================================================================

/// Trait for brain restoration functionality
#[async_trait]
pub trait BrainRestore: Send + Sync {
    /// Load all events up to a specific timestamp
    async fn load_events_until(
        &self,
        agent_id: &str,
        timestamp: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    /// Load events in a specific time range
    async fn load_events_range(
        &self,
        agent_id: &str,
        start: i64,
        end: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    /// Filter events by event type
    async fn filter_by_event_type(
        &self,
        agent_id: &str,
        event_types: Vec<HistoryEventType>,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    /// Replay events to rebuild state
    async fn replay_events(
        &self,
        events: Vec<AgentHistoryEvent>,
        options: RestoreOptions,
    ) -> StateStoreResult<RestoreResult>;

    /// Restore brain state from a snapshot
    async fn restore_brain_state(
        &self,
        snapshot_id: &Uuid,
        options: RestoreOptions,
    ) -> StateStoreResult<RestoreResult>;

    /// Apply a single event to an existing brain state
    fn apply_event(
        &self,
        state: &mut BrainState,
        event: &AgentHistoryEvent,
    ) -> StateStoreResult<()>;

    /// Validate state consistency
    fn validate_state(&self, state: &BrainState) -> StateStoreResult<Vec<String>>;

    /// Check event dependencies
    fn check_dependencies(
        &self,
        events: &[AgentHistoryEvent],
    ) -> StateStoreResult<Vec<String>>;
}

// ============================================================================
// DEFAULT BRAIN RESTORE IMPLEMENTATION
// ============================================================================

/// Default implementation of BrainRestore
pub struct DefaultBrainRestore<S: AgentHistoryStore> {
    store: S,
}

impl<S: AgentHistoryStore> DefaultBrainRestore<S> {
    /// Create a new brain restore instance
    pub fn new(store: S) -> Self {
        Self { store }
    }

    /// Get a reference to the history store
    pub fn store(&self) -> &S {
        &self.store
    }

    /// Sort events by timestamp and causality
    fn sort_events_by_causality(&self, events: &mut Vec<AgentHistoryEvent>) {
        // Build parent-child map
        let mut parent_map: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        let mut event_map: HashMap<Uuid, AgentHistoryEvent> = HashMap::new();

        for event in events.iter() {
            event_map.insert(event.event_id, event.clone());
            if let Some(parent_id) = event.parent_event_id {
                parent_map.entry(parent_id).or_default().push(event.event_id);
            }
        }

        // Topological sort respecting timestamp and causality
        events.sort_by(|a, b| {
            // If one is parent of the other, parent comes first
            if a.parent_event_id == Some(b.event_id) {
                return std::cmp::Ordering::Greater;
            }
            if b.parent_event_id == Some(a.event_id) {
                return std::cmp::Ordering::Less;
            }
            // Otherwise sort by timestamp
            a.timestamp.cmp(&b.timestamp)
        });
    }

    /// Extract thought entry from event data
    fn extract_thought(&self, event: &AgentHistoryEvent) -> Option<ThoughtEntry> {
        if event.event_type != HistoryEventType::Thought {
            return None;
        }

        let content = event.event_data.get("content")?.as_str()?.to_string();
        let thought_type = event
            .event_data
            .get("thought_type")
            .and_then(|v| v.as_str())
            .unwrap_or("general")
            .to_string();

        let thought_id = event
            .event_data
            .get("thought_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        Some(ThoughtEntry {
            thought_id,
            timestamp: event.timestamp,
            content,
            thought_type,
            parent_thought_id: event.parent_event_id,
            metadata: event.metadata.clone(),
        })
    }

    /// Extract decision node from event data
    fn extract_decision(&self, event: &AgentHistoryEvent) -> Option<DecisionNode> {
        if event.event_type != HistoryEventType::Decision {
            return None;
        }

        let decision_type = event
            .event_data
            .get("decision_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let context = event.event_data.get("context")?.clone();

        let outcome = event
            .event_data
            .get("outcome")
            .and_then(|v| v.as_str())
            .map(String::from);

        let decision_id = event
            .event_data
            .get("decision_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        Some(DecisionNode {
            decision_id,
            timestamp: event.timestamp,
            decision_type,
            context,
            outcome,
            parent_decision_id: event.parent_event_id,
            children: Vec::new(),
        })
    }

    /// Extract message from event data
    fn extract_message(&self, event: &AgentHistoryEvent) -> Option<MessageEntry> {
        if event.event_type != HistoryEventType::Communication {
            return None;
        }

        let role = event
            .event_data
            .get("role")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let content = event.event_data.get("content")?.as_str()?.to_string();

        let message_id = event
            .event_data
            .get("message_id")
            .and_then(|v| v.as_str())
            .and_then(|s| Uuid::parse_str(s).ok())
            .unwrap_or_else(Uuid::new_v4);

        Some(MessageEntry {
            message_id,
            timestamp: event.timestamp,
            role,
            content,
            metadata: event.metadata.clone(),
        })
    }

    /// Update memory from event data
    fn update_memory(&self, memory: &mut HashMap<String, Value>, event: &AgentHistoryEvent) {
        if let Some(key) = event.event_data.get("memory_key").and_then(|v| v.as_str()) {
            if let Some(value) = event.event_data.get("memory_value") {
                memory.insert(key.to_string(), value.clone());
            }
        }
    }
}

#[async_trait]
impl<S: AgentHistoryStore> BrainRestore for DefaultBrainRestore<S> {
    async fn load_events_until(
        &self,
        agent_id: &str,
        timestamp: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        let query = HistoryQuery {
            agent_id: Some(agent_id.to_string()),
            end_time: Some(timestamp),
            ascending: true,
            ..Default::default()
        };

        self.store.query_events(&query).await
    }

    async fn load_events_range(
        &self,
        agent_id: &str,
        start: i64,
        end: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        self.store
            .get_events_by_time_range(agent_id, start, end)
            .await
    }

    async fn filter_by_event_type(
        &self,
        agent_id: &str,
        event_types: Vec<HistoryEventType>,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        let mut all_events = Vec::new();

        for event_type in event_types {
            let events = self
                .store
                .get_events_by_type(agent_id, event_type, i64::MAX)
                .await?;
            all_events.extend(events);
        }

        // Sort by timestamp
        all_events.sort_by_key(|e| e.timestamp);

        Ok(all_events)
    }

    async fn replay_events(
        &self,
        mut events: Vec<AgentHistoryEvent>,
        options: RestoreOptions,
    ) -> StateStoreResult<RestoreResult> {
        let start_time = std::time::Instant::now();

        if events.is_empty() {
            return Ok(RestoreResult {
                success: true,
                brain_state: None,
                events_processed: 0,
                events_skipped: 0,
                validation_errors: Vec::new(),
                warnings: vec!["No events to replay".to_string()],
                duration_ms: start_time.elapsed().as_millis() as u64,
            });
        }

        // Get agent_id from first event
        let agent_id = events[0].agent_id.clone();

        // Sort events by causality
        if options.strict_causality {
            self.sort_events_by_causality(&mut events);
        } else {
            events.sort_by_key(|e| e.timestamp);
        }

        // Apply event filters
        if !options.event_filters.is_empty() {
            events.retain(|e| options.event_filters.contains(&e.event_type));
        }

        // Limit events if specified
        if let Some(max) = options.max_events {
            events.truncate(max);
        }

        // Check dependencies
        let mut warnings = Vec::new();
        if options.strict_causality {
            match self.check_dependencies(&events) {
                Ok(dep_warnings) => warnings.extend(dep_warnings),
                Err(e) if options.skip_missing_events => {
                    warnings.push(format!("Dependency check failed: {}", e));
                }
                Err(e) => return Err(e),
            }
        }

        // Initialize brain state
        let mut state = BrainState::new(agent_id);
        let mut events_processed = 0;
        let mut events_skipped = 0;

        // Replay events
        for event in &events {
            match self.apply_event(&mut state, event) {
                Ok(_) => {
                    events_processed += 1;
                    // Update git commit and session from latest event
                    if event.git_commit_hash.is_some() {
                        state.git_commit = event.git_commit_hash.clone();
                    }
                    if event.session_id.is_some() {
                        state.session_id = event.session_id.clone();
                    }
                }
                Err(e) if options.skip_missing_events => {
                    events_skipped += 1;
                    warnings.push(format!("Skipped event {}: {}", event.event_id, e));
                }
                Err(e) => {
                    return Ok(RestoreResult {
                        success: false,
                        brain_state: Some(state),
                        events_processed,
                        events_skipped,
                        validation_errors: vec![format!("Failed to apply event: {}", e)],
                        warnings,
                        duration_ms: start_time.elapsed().as_millis() as u64,
                    });
                }
            }
        }

        // Update final timestamp
        if let Some(last_event) = events.last() {
            state.timestamp = last_event.timestamp;
        }

        // Validate if requested
        let mut validation_errors = Vec::new();
        if options.validate {
            match self.validate_state(&state) {
                Ok(errors) => validation_errors = errors,
                Err(e) => validation_errors.push(format!("Validation failed: {}", e)),
            }
        }

        let duration_ms = start_time.elapsed().as_millis() as u64;

        Ok(RestoreResult {
            success: validation_errors.is_empty(),
            brain_state: Some(state),
            events_processed,
            events_skipped,
            validation_errors,
            warnings,
            duration_ms,
        })
    }

    async fn restore_brain_state(
        &self,
        snapshot_id: &Uuid,
        options: RestoreOptions,
    ) -> StateStoreResult<RestoreResult> {
        // Load snapshot
        let snapshot = self
            .store
            .get_snapshot(snapshot_id)
            .await?
            .ok_or_else(|| {
                StateStoreError::NotFound(format!("Snapshot {} not found", snapshot_id))
            })?;

        // Replay events from snapshot
        self.replay_events(snapshot.events, options).await
    }

    fn apply_event(
        &self,
        state: &mut BrainState,
        event: &AgentHistoryEvent,
    ) -> StateStoreResult<()> {
        match event.event_type {
            HistoryEventType::Thought => {
                if let Some(thought) = self.extract_thought(event) {
                    state.thought_history.push(thought);
                }
            }
            HistoryEventType::Decision => {
                if let Some(decision) = self.extract_decision(event) {
                    state.decision_tree.push(decision);
                }
            }
            HistoryEventType::Communication => {
                if let Some(message) = self.extract_message(event) {
                    state.conversation_state.messages.push(message);
                    state.conversation_state.current_turn += 1;
                }
            }
            HistoryEventType::Action => {
                // Update memory if action contains memory operations
                self.update_memory(&mut state.memory, event);
            }
            HistoryEventType::StateChange => {
                // Store state change in metadata
                state.metadata.insert(
                    format!("state_change_{}", event.timestamp),
                    event.event_data.clone(),
                );
            }
            HistoryEventType::System => {
                // Store system events in metadata
                state.metadata.insert(
                    format!("system_{}", event.timestamp),
                    event.event_data.clone(),
                );
            }
            HistoryEventType::ToolUse | HistoryEventType::Error => {
                // Store in metadata for reference
                state.metadata.insert(
                    format!("{}_{}", event.event_type, event.timestamp),
                    event.event_data.clone(),
                );
            }
        }

        Ok(())
    }

    fn validate_state(&self, state: &BrainState) -> StateStoreResult<Vec<String>> {
        let mut errors = Vec::new();

        // Check for orphaned thoughts
        let thought_ids: HashSet<Uuid> = state
            .thought_history
            .iter()
            .map(|t| t.thought_id)
            .collect();

        for thought in &state.thought_history {
            if let Some(parent_id) = thought.parent_thought_id {
                if !thought_ids.contains(&parent_id) {
                    errors.push(format!(
                        "Thought {} has missing parent {}",
                        thought.thought_id, parent_id
                    ));
                }
            }
        }

        // Check for orphaned decisions
        let decision_ids: HashSet<Uuid> = state
            .decision_tree
            .iter()
            .map(|d| d.decision_id)
            .collect();

        for decision in &state.decision_tree {
            if let Some(parent_id) = decision.parent_decision_id {
                if !decision_ids.contains(&parent_id) {
                    errors.push(format!(
                        "Decision {} has missing parent {}",
                        decision.decision_id, parent_id
                    ));
                }
            }
        }

        // Check conversation state consistency
        if state.conversation_state.current_turn as usize
            != state.conversation_state.messages.len()
        {
            errors.push(format!(
                "Conversation turn count mismatch: {} turns but {} messages",
                state.conversation_state.current_turn,
                state.conversation_state.messages.len()
            ));
        }

        Ok(errors)
    }

    fn check_dependencies(
        &self,
        events: &[AgentHistoryEvent],
    ) -> StateStoreResult<Vec<String>> {
        let mut warnings = Vec::new();
        let event_ids: HashSet<Uuid> = events.iter().map(|e| e.event_id).collect();

        for event in events {
            if let Some(parent_id) = event.parent_event_id {
                if !event_ids.contains(&parent_id) {
                    warnings.push(format!(
                        "Event {} references missing parent event {}",
                        event.event_id, parent_id
                    ));
                }
            }
        }

        Ok(warnings)
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Create a snapshot from current brain state
pub fn create_snapshot_from_state(
    brain_state: &BrainState,
    events: Vec<AgentHistoryEvent>,
) -> HistorySnapshot {
    HistorySnapshot::new(
        brain_state.agent_id.clone(),
        events,
        brain_state.git_commit.clone(),
    )
    .with_agent_state(serde_json::to_value(brain_state).unwrap_or(Value::Null))
}

/// Compare two brain states for differences
pub fn compare_states(state1: &BrainState, state2: &BrainState) -> Vec<String> {
    let mut differences = Vec::new();

    if state1.thought_history.len() != state2.thought_history.len() {
        differences.push(format!(
            "Thought history length differs: {} vs {}",
            state1.thought_history.len(),
            state2.thought_history.len()
        ));
    }

    if state1.decision_tree.len() != state2.decision_tree.len() {
        differences.push(format!(
            "Decision tree length differs: {} vs {}",
            state1.decision_tree.len(),
            state2.decision_tree.len()
        ));
    }

    if state1.memory.len() != state2.memory.len() {
        differences.push(format!(
            "Memory size differs: {} vs {}",
            state1.memory.len(),
            state2.memory.len()
        ));
    }

    if state1.conversation_state.messages.len() != state2.conversation_state.messages.len() {
        differences.push(format!(
            "Message count differs: {} vs {}",
            state1.conversation_state.messages.len(),
            state2.conversation_state.messages.len()
        ));
    }

    differences
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_history::{AgentHistoryStore, SqliteAgentHistoryStore};
    use serde_json::json;
    use tempfile::NamedTempFile;

    async fn create_test_store() -> SqliteAgentHistoryStore {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        let mut store = SqliteAgentHistoryStore::new(path).await.unwrap();
        store.initialize().await.unwrap();
        store
    }

    #[tokio::test]
    async fn test_load_events_until() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        // Create test events
        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                json!({"content": "thinking 1"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                json!({"content": "thinking 2"}),
            ),
        ];

        for event in &events {
            restore.store.record_event(event).await.unwrap();
        }

        let future_timestamp = chrono::Utc::now().timestamp() + 1000;
        let loaded = restore
            .load_events_until("agent-1", future_timestamp)
            .await
            .unwrap();

        assert_eq!(loaded.len(), 2);
    }

    #[tokio::test]
    async fn test_load_events_range() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let start_time = chrono::Utc::now().timestamp();

        let event = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Action,
            json!({"action": "test"}),
        );

        restore.store.record_event(&event).await.unwrap();

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let end_time = chrono::Utc::now().timestamp();

        let loaded = restore
            .load_events_range("agent-1", start_time, end_time)
            .await
            .unwrap();

        assert_eq!(loaded.len(), 1);
    }

    #[tokio::test]
    async fn test_filter_by_event_type() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                json!({"content": "thinking"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Action,
                json!({"action": "execute"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                json!({"content": "more thinking"}),
            ),
        ];

        for event in &events {
            restore.store.record_event(event).await.unwrap();
        }

        let filtered = restore
            .filter_by_event_type("agent-1", vec![HistoryEventType::Thought])
            .await
            .unwrap();

        assert_eq!(filtered.len(), 2);
        assert!(filtered.iter().all(|e| e.event_type == HistoryEventType::Thought));
    }

    #[tokio::test]
    async fn test_replay_events() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                json!({
                    "content": "analyzing problem",
                    "thought_type": "analysis"
                }),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Decision,
                json!({
                    "decision_type": "action_selection",
                    "context": {"options": ["A", "B"]},
                    "outcome": "A"
                }),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Communication,
                json!({
                    "role": "assistant",
                    "content": "I will proceed with option A"
                }),
            ),
        ];

        let result = restore
            .replay_events(events, RestoreOptions::default())
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.events_processed, 3);
        assert_eq!(result.events_skipped, 0);

        let state = result.brain_state.unwrap();
        assert_eq!(state.thought_history.len(), 1);
        assert_eq!(state.decision_tree.len(), 1);
        assert_eq!(state.conversation_state.messages.len(), 1);
    }

    #[tokio::test]
    async fn test_restore_brain_state_from_snapshot() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let event = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            json!({"content": "test thought"}),
        );

        restore.store.record_event(&event).await.unwrap();

        let snapshot = HistorySnapshot::new(
            "agent-1".to_string(),
            vec![event],
            Some("abc123".to_string()),
        );

        restore.store.create_snapshot(&snapshot).await.unwrap();

        let result = restore
            .restore_brain_state(&snapshot.snapshot_id, RestoreOptions::default())
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.events_processed, 1);
    }

    #[tokio::test]
    async fn test_apply_event_to_state() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let mut state = BrainState::new("agent-1".to_string());

        let thought_event = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            json!({
                "content": "test thought",
                "thought_type": "reasoning"
            }),
        );

        restore.apply_event(&mut state, &thought_event).unwrap();

        assert_eq!(state.thought_history.len(), 1);
        assert_eq!(state.thought_history[0].content, "test thought");
    }

    #[tokio::test]
    async fn test_validate_state() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let mut state = BrainState::new("agent-1".to_string());

        // Add valid thought
        state.thought_history.push(ThoughtEntry {
            thought_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().timestamp(),
            content: "test".to_string(),
            thought_type: "test".to_string(),
            parent_thought_id: None,
            metadata: None,
        });

        // Add orphaned thought with missing parent
        let missing_parent = Uuid::new_v4();
        state.thought_history.push(ThoughtEntry {
            thought_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().timestamp(),
            content: "orphaned".to_string(),
            thought_type: "test".to_string(),
            parent_thought_id: Some(missing_parent),
            metadata: None,
        });

        let errors = restore.validate_state(&state).unwrap();
        assert!(!errors.is_empty());
        assert!(errors[0].contains("missing parent"));
    }

    #[tokio::test]
    async fn test_check_dependencies() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let parent_event = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            json!({"content": "parent"}),
        );

        let child_event = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Action,
            json!({"action": "based on thought"}),
        )
        .with_parent(parent_event.event_id);

        // Only include child, not parent
        let warnings = restore.check_dependencies(&[child_event]).unwrap();

        assert!(!warnings.is_empty());
        assert!(warnings[0].contains("missing parent"));
    }

    #[tokio::test]
    async fn test_replay_with_causality() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let event1 = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            json!({"content": "first"}),
        );

        let event2 = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Action,
            json!({"action": "second"}),
        )
        .with_parent(event1.event_id);

        // Pass events in wrong order
        let events = vec![event2.clone(), event1.clone()];

        let options = RestoreOptions {
            strict_causality: true,
            ..Default::default()
        };

        let result = restore.replay_events(events, options).await.unwrap();

        // Should still succeed because events are sorted by causality
        assert!(result.success);
        assert_eq!(result.events_processed, 2);
    }

    #[tokio::test]
    async fn test_replay_with_event_filters() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                json!({"content": "thinking"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Action,
                json!({"action": "execute"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Error,
                json!({"error": "failure"}),
            ),
        ];

        let options = RestoreOptions::default()
            .with_event_filters(vec![HistoryEventType::Thought, HistoryEventType::Action]);

        let result = restore.replay_events(events, options).await.unwrap();

        assert!(result.success);
        // Only Thought and Action should be processed, Error filtered out
        assert_eq!(result.events_processed, 2);
    }

    #[tokio::test]
    async fn test_compare_states() {
        let state1 = BrainState::new("agent-1".to_string());
        let mut state2 = BrainState::new("agent-1".to_string());

        state2.thought_history.push(ThoughtEntry {
            thought_id: Uuid::new_v4(),
            timestamp: chrono::Utc::now().timestamp(),
            content: "test".to_string(),
            thought_type: "test".to_string(),
            parent_thought_id: None,
            metadata: None,
        });

        let differences = compare_states(&state1, &state2);

        assert!(!differences.is_empty());
        assert!(differences[0].contains("Thought history length differs"));
    }

    #[tokio::test]
    async fn test_replay_empty_events() {
        let store = create_test_store().await;
        let restore = DefaultBrainRestore::new(store);

        let result = restore
            .replay_events(vec![], RestoreOptions::default())
            .await
            .unwrap();

        assert!(result.success);
        assert_eq!(result.events_processed, 0);
        assert!(!result.warnings.is_empty());
    }
}
