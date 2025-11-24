/// Agent History - Event Sourcing and History Tracking for Agent Brain & Body
///
/// This module provides comprehensive history tracking for agents, combining:
/// - Brain State: Event logs tracking thoughts, actions, tool usage, and state changes
/// - Body State: Git commit references tracking code changes and artifacts
///
/// The history system enables:
/// - Time-travel debugging and replay
/// - Audit trails for agent actions
/// - Performance analysis and optimization
/// - Recovery and restoration from any point in history
use crate::errors::{StateStoreError, StateStoreResult};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use sqlx::{ConnectOptions, Row};
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use uuid::Uuid;

// ============================================================================
// EVENT TYPES AND ENUMS
// ============================================================================

/// Types of history events that can be recorded for an agent
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HistoryEventType {
    /// Cognitive events - agent's internal reasoning
    Thought,
    /// Action events - agent performing operations
    Action,
    /// Tool usage events - agent using external tools
    ToolUse,
    /// State transitions - changes in agent state machine
    StateChange,
    /// Communication events - messages sent/received
    Communication,
    /// Decision events - choices made by the agent
    Decision,
    /// Error events - failures and exceptions
    Error,
    /// System events - lifecycle and metadata changes
    System,
}

impl std::fmt::Display for HistoryEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryEventType::Thought => write!(f, "thought"),
            HistoryEventType::Action => write!(f, "action"),
            HistoryEventType::ToolUse => write!(f, "tool_use"),
            HistoryEventType::StateChange => write!(f, "state_change"),
            HistoryEventType::Communication => write!(f, "communication"),
            HistoryEventType::Decision => write!(f, "decision"),
            HistoryEventType::Error => write!(f, "error"),
            HistoryEventType::System => write!(f, "system"),
        }
    }
}

impl FromStr for HistoryEventType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "thought" => Ok(HistoryEventType::Thought),
            "action" => Ok(HistoryEventType::Action),
            "tool_use" => Ok(HistoryEventType::ToolUse),
            "state_change" => Ok(HistoryEventType::StateChange),
            "communication" => Ok(HistoryEventType::Communication),
            "decision" => Ok(HistoryEventType::Decision),
            "error" => Ok(HistoryEventType::Error),
            "system" => Ok(HistoryEventType::System),
            _ => Err(format!("Unknown event type: {}", s)),
        }
    }
}

// ============================================================================
// CORE DATA MODELS
// ============================================================================

/// A single event in an agent's history (brain state)
///
/// Represents a discrete moment in time when the agent performed an action,
/// had a thought, used a tool, or underwent a state change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHistoryEvent {
    /// Unique identifier for this event
    pub event_id: Uuid,

    /// ID of the agent that generated this event
    pub agent_id: String,

    /// Timestamp when the event occurred (Unix timestamp in seconds)
    pub timestamp: i64,

    /// Type of event (Thought, Action, ToolUse, StateChange, etc.)
    pub event_type: HistoryEventType,

    /// Event-specific data (flexible JSON structure)
    pub event_data: Value,

    /// Optional git commit hash associated with this event (body state)
    pub git_commit_hash: Option<String>,

    /// Optional session ID to group related events
    pub session_id: Option<String>,

    /// Optional parent event ID for causality tracking
    pub parent_event_id: Option<Uuid>,

    /// Optional tags for categorization and filtering
    pub tags: Vec<String>,

    /// Additional metadata
    pub metadata: Option<Value>,
}

impl AgentHistoryEvent {
    /// Create a new history event
    pub fn new(agent_id: String, event_type: HistoryEventType, event_data: Value) -> Self {
        Self {
            event_id: Uuid::new_v4(),
            agent_id,
            timestamp: Utc::now().timestamp(),
            event_type,
            event_data,
            git_commit_hash: None,
            session_id: None,
            parent_event_id: None,
            tags: Vec::new(),
            metadata: None,
        }
    }

    /// Create a new event with git commit reference
    pub fn with_git_commit(mut self, commit_hash: String) -> Self {
        self.git_commit_hash = Some(commit_hash);
        self
    }

    /// Create a new event with session ID
    pub fn with_session(mut self, session_id: String) -> Self {
        self.session_id = Some(session_id);
        self
    }

    /// Create a new event with parent event for causality
    pub fn with_parent(mut self, parent_id: Uuid) -> Self {
        self.parent_event_id = Some(parent_id);
        self
    }

    /// Add tags to the event
    pub fn with_tags(mut self, tags: Vec<String>) -> Self {
        self.tags = tags;
        self
    }

    /// Add metadata to the event
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// A snapshot combining brain and body state at a specific point in time
///
/// This represents the complete state of an agent, including:
/// - Events (brain): the cognitive and action history
/// - Git commit (body): the code and artifact state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySnapshot {
    /// Unique identifier for this snapshot
    pub snapshot_id: Uuid,

    /// ID of the agent this snapshot belongs to
    pub agent_id: String,

    /// Timestamp when the snapshot was created
    pub timestamp: i64,

    /// Events included in this snapshot (brain state)
    pub events: Vec<AgentHistoryEvent>,

    /// Git commit hash at the time of snapshot (body state)
    pub git_commit: Option<String>,

    /// Optional description of this snapshot
    pub description: Option<String>,

    /// Metadata about the snapshot
    pub metadata: Option<Value>,

    /// Agent state data at the time of snapshot
    pub agent_state: Option<Value>,
}

impl HistorySnapshot {
    /// Create a new history snapshot
    pub fn new(
        agent_id: String,
        events: Vec<AgentHistoryEvent>,
        git_commit: Option<String>,
    ) -> Self {
        Self {
            snapshot_id: Uuid::new_v4(),
            agent_id,
            timestamp: Utc::now().timestamp(),
            events,
            git_commit,
            description: None,
            metadata: None,
            agent_state: None,
        }
    }

    /// Create a snapshot with description
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }

    /// Create a snapshot with agent state
    pub fn with_agent_state(mut self, state: Value) -> Self {
        self.agent_state = Some(state);
        self
    }

    /// Create a snapshot with metadata
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = Some(metadata);
        self
    }
}

/// Query parameters for retrieving history events
#[derive(Debug, Clone, Default)]
pub struct HistoryQuery {
    /// Filter by agent ID
    pub agent_id: Option<String>,

    /// Filter by session ID
    pub session_id: Option<String>,

    /// Filter by event type
    pub event_type: Option<HistoryEventType>,

    /// Filter by tags (events must have all specified tags)
    pub tags: Vec<String>,

    /// Start timestamp (inclusive)
    pub start_time: Option<i64>,

    /// End timestamp (inclusive)
    pub end_time: Option<i64>,

    /// Maximum number of results to return
    pub limit: Option<i64>,

    /// Offset for pagination
    pub offset: Option<i64>,

    /// Sort order (true = ascending, false = descending)
    pub ascending: bool,
}

/// Statistics about agent history
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryStatistics {
    /// Total number of events
    pub total_events: i64,

    /// Events by type
    pub events_by_type: std::collections::HashMap<String, i64>,

    /// Total number of snapshots
    pub total_snapshots: i64,

    /// Time range of history
    pub earliest_event: Option<i64>,
    pub latest_event: Option<i64>,

    /// Number of unique sessions
    pub unique_sessions: i64,

    /// Git commits referenced
    pub git_commits: Vec<String>,
}

// ============================================================================
// STORAGE TRAIT
// ============================================================================

/// Trait for agent history storage implementations
#[async_trait]
pub trait AgentHistoryStore: Send + Sync {
    /// Initialize the history store (create tables, run migrations)
    async fn initialize(&mut self) -> StateStoreResult<()>;

    /// Record a new event in the agent's history
    async fn record_event(&self, event: &AgentHistoryEvent) -> StateStoreResult<()>;

    /// Record multiple events in a batch
    async fn record_events(&self, events: &[AgentHistoryEvent]) -> StateStoreResult<()>;

    /// Get events for a specific agent
    async fn get_events(
        &self,
        agent_id: &str,
        limit: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    /// Query events with filters
    async fn query_events(&self, query: &HistoryQuery) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    /// Get events by type
    async fn get_events_by_type(
        &self,
        agent_id: &str,
        event_type: HistoryEventType,
        limit: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    /// Get events in a time range
    async fn get_events_by_time_range(
        &self,
        agent_id: &str,
        start_time: i64,
        end_time: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    /// Get events by session
    async fn get_events_by_session(
        &self,
        session_id: &str,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>>;

    /// Create a snapshot of agent history
    async fn create_snapshot(&self, snapshot: &HistorySnapshot) -> StateStoreResult<()>;

    /// Get a specific snapshot by ID
    async fn get_snapshot(&self, snapshot_id: &Uuid) -> StateStoreResult<Option<HistorySnapshot>>;

    /// List snapshots for an agent
    async fn list_snapshots(&self, agent_id: &str) -> StateStoreResult<Vec<HistorySnapshot>>;

    /// Delete old events (for cleanup/archival)
    async fn delete_events_before(&self, timestamp: i64) -> StateStoreResult<i64>;

    /// Get history statistics
    async fn get_statistics(&self, agent_id: &str) -> StateStoreResult<HistoryStatistics>;

    /// Get the event chain (follow parent references)
    async fn get_event_chain(&self, event_id: &Uuid) -> StateStoreResult<Vec<AgentHistoryEvent>>;
}

// ============================================================================
// SQLITE IMPLEMENTATION
// ============================================================================

/// SQLite-backed implementation of AgentHistoryStore
pub struct SqliteAgentHistoryStore {
    pool: SqlitePool,
}

impl SqliteAgentHistoryStore {
    /// Create a new SQLite history store
    pub async fn new<P: AsRef<Path>>(db_path: P) -> StateStoreResult<Self> {
        let db_path = db_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    StateStoreError::DatabaseError(format!("Failed to create directory: {}", e))
                })?;
            }
        }

        // Create connection options
        let connect_options =
            sqlx::sqlite::SqliteConnectOptions::from_str(db_path.to_string_lossy().as_ref())
                .map_err(|e| {
                    StateStoreError::DatabaseError(format!("Failed to parse database path: {}", e))
                })?
                .create_if_missing(true)
                .foreign_keys(true);

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect_with(connect_options)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to create database pool: {}", e))
            })?;

        Ok(Self { pool })
    }

    /// Get reference to the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl AgentHistoryStore for SqliteAgentHistoryStore {
    async fn initialize(&mut self) -> StateStoreResult<()> {
        // Create agent_history_events table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS agent_history_events (
                event_id TEXT PRIMARY KEY NOT NULL,
                agent_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                event_type TEXT NOT NULL,
                event_data TEXT NOT NULL,
                git_commit_hash TEXT,
                session_id TEXT,
                parent_event_id TEXT,
                tags TEXT,
                metadata TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );

            CREATE INDEX IF NOT EXISTS idx_history_agent_id ON agent_history_events(agent_id);
            CREATE INDEX IF NOT EXISTS idx_history_timestamp ON agent_history_events(timestamp);
            CREATE INDEX IF NOT EXISTS idx_history_event_type ON agent_history_events(event_type);
            CREATE INDEX IF NOT EXISTS idx_history_session_id ON agent_history_events(session_id);
            CREATE INDEX IF NOT EXISTS idx_history_git_commit ON agent_history_events(git_commit_hash);
            CREATE INDEX IF NOT EXISTS idx_history_agent_timestamp ON agent_history_events(agent_id, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_history_agent_type ON agent_history_events(agent_id, event_type);
            CREATE INDEX IF NOT EXISTS idx_history_parent ON agent_history_events(parent_event_id);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to create events table: {}", e))
        })?;

        // Create history_snapshots table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS history_snapshots (
                snapshot_id TEXT PRIMARY KEY NOT NULL,
                agent_id TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                git_commit TEXT,
                description TEXT,
                metadata TEXT,
                agent_state TEXT,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            );

            CREATE INDEX IF NOT EXISTS idx_snapshots_agent_id ON history_snapshots(agent_id);
            CREATE INDEX IF NOT EXISTS idx_snapshots_timestamp ON history_snapshots(timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_snapshots_agent_timestamp ON history_snapshots(agent_id, timestamp DESC);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to create snapshots table: {}", e))
        })?;

        // Create snapshot_events junction table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS snapshot_events (
                snapshot_id TEXT NOT NULL,
                event_id TEXT NOT NULL,
                PRIMARY KEY (snapshot_id, event_id),
                FOREIGN KEY (snapshot_id) REFERENCES history_snapshots(snapshot_id) ON DELETE CASCADE,
                FOREIGN KEY (event_id) REFERENCES agent_history_events(event_id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_snapshot_events_snapshot ON snapshot_events(snapshot_id);
            CREATE INDEX IF NOT EXISTS idx_snapshot_events_event ON snapshot_events(event_id);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!(
                "Failed to create snapshot_events table: {}",
                e
            ))
        })?;

        Ok(())
    }

    async fn record_event(&self, event: &AgentHistoryEvent) -> StateStoreResult<()> {
        let tags_json = serde_json::to_string(&event.tags).map_err(|e| {
            StateStoreError::SerializationError(format!("Failed to serialize tags: {}", e))
        })?;

        let event_data_json = serde_json::to_string(&event.event_data).map_err(|e| {
            StateStoreError::SerializationError(format!("Failed to serialize event_data: {}", e))
        })?;

        let metadata_json = event.metadata.as_ref().map(|m| m.to_string());

        sqlx::query(
            r#"
            INSERT INTO agent_history_events
            (event_id, agent_id, timestamp, event_type, event_data, git_commit_hash,
             session_id, parent_event_id, tags, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.event_id.to_string())
        .bind(&event.agent_id)
        .bind(event.timestamp)
        .bind(event.event_type.to_string())
        .bind(event_data_json)
        .bind(&event.git_commit_hash)
        .bind(&event.session_id)
        .bind(event.parent_event_id.map(|id| id.to_string()))
        .bind(tags_json)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to record event: {}", e)))?;

        Ok(())
    }

    async fn record_events(&self, events: &[AgentHistoryEvent]) -> StateStoreResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to begin transaction: {}", e))
        })?;

        for event in events {
            let tags_json = serde_json::to_string(&event.tags).map_err(|e| {
                StateStoreError::SerializationError(format!("Failed to serialize tags: {}", e))
            })?;

            let event_data_json = serde_json::to_string(&event.event_data).map_err(|e| {
                StateStoreError::SerializationError(format!(
                    "Failed to serialize event_data: {}",
                    e
                ))
            })?;

            let metadata_json = event.metadata.as_ref().map(|m| m.to_string());

            sqlx::query(
                r#"
                INSERT INTO agent_history_events
                (event_id, agent_id, timestamp, event_type, event_data, git_commit_hash,
                 session_id, parent_event_id, tags, metadata)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(event.event_id.to_string())
            .bind(&event.agent_id)
            .bind(event.timestamp)
            .bind(event.event_type.to_string())
            .bind(event_data_json)
            .bind(&event.git_commit_hash)
            .bind(&event.session_id)
            .bind(event.parent_event_id.map(|id| id.to_string()))
            .bind(tags_json)
            .bind(metadata_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to record event: {}", e))
            })?;
        }

        tx.commit().await.map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn get_events(
        &self,
        agent_id: &str,
        limit: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, agent_id, timestamp, event_type, event_data, git_commit_hash,
                   session_id, parent_event_id, tags, metadata
            FROM agent_history_events
            WHERE agent_id = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch events: {}", e)))?;

        self.rows_to_events(rows)
    }

    async fn query_events(&self, query: &HistoryQuery) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        let mut sql = String::from(
            "SELECT event_id, agent_id, timestamp, event_type, event_data, git_commit_hash,
                    session_id, parent_event_id, tags, metadata
             FROM agent_history_events WHERE 1=1",
        );

        let mut conditions = Vec::new();

        if query.agent_id.is_some() {
            conditions.push("agent_id = ?");
        }

        if query.session_id.is_some() {
            conditions.push("session_id = ?");
        }

        if query.event_type.is_some() {
            conditions.push("event_type = ?");
        }

        if query.start_time.is_some() {
            conditions.push("timestamp >= ?");
        }

        if query.end_time.is_some() {
            conditions.push("timestamp <= ?");
        }

        for condition in conditions {
            sql.push_str(" AND ");
            sql.push_str(condition);
        }

        sql.push_str(if query.ascending {
            " ORDER BY timestamp ASC"
        } else {
            " ORDER BY timestamp DESC"
        });

        if let Some(limit) = query.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }

        if let Some(offset) = query.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let mut query_builder = sqlx::query(&sql);

        if let Some(ref agent_id) = query.agent_id {
            query_builder = query_builder.bind(agent_id);
        }

        if let Some(ref session_id) = query.session_id {
            query_builder = query_builder.bind(session_id);
        }

        if let Some(ref event_type) = query.event_type {
            query_builder = query_builder.bind(event_type.to_string());
        }

        if let Some(start_time) = query.start_time {
            query_builder = query_builder.bind(start_time);
        }

        if let Some(end_time) = query.end_time {
            query_builder = query_builder.bind(end_time);
        }

        let rows = query_builder.fetch_all(&self.pool).await.map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to query events: {}", e))
        })?;

        self.rows_to_events(rows)
    }

    async fn get_events_by_type(
        &self,
        agent_id: &str,
        event_type: HistoryEventType,
        limit: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, agent_id, timestamp, event_type, event_data, git_commit_hash,
                   session_id, parent_event_id, tags, metadata
            FROM agent_history_events
            WHERE agent_id = ? AND event_type = ?
            ORDER BY timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(agent_id)
        .bind(event_type.to_string())
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to fetch events by type: {}", e))
        })?;

        self.rows_to_events(rows)
    }

    async fn get_events_by_time_range(
        &self,
        agent_id: &str,
        start_time: i64,
        end_time: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, agent_id, timestamp, event_type, event_data, git_commit_hash,
                   session_id, parent_event_id, tags, metadata
            FROM agent_history_events
            WHERE agent_id = ? AND timestamp >= ? AND timestamp <= ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(agent_id)
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to fetch events by time range: {}", e))
        })?;

        self.rows_to_events(rows)
    }

    async fn get_events_by_session(
        &self,
        session_id: &str,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, agent_id, timestamp, event_type, event_data, git_commit_hash,
                   session_id, parent_event_id, tags, metadata
            FROM agent_history_events
            WHERE session_id = ?
            ORDER BY timestamp ASC
            "#,
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to fetch events by session: {}", e))
        })?;

        self.rows_to_events(rows)
    }

    async fn create_snapshot(&self, snapshot: &HistorySnapshot) -> StateStoreResult<()> {
        let mut tx = self.pool.begin().await.map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to begin transaction: {}", e))
        })?;

        let metadata_json = snapshot.metadata.as_ref().map(|m| m.to_string());
        let agent_state_json = snapshot.agent_state.as_ref().map(|s| s.to_string());

        // Insert snapshot
        sqlx::query(
            r#"
            INSERT INTO history_snapshots
            (snapshot_id, agent_id, timestamp, git_commit, description, metadata, agent_state)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(snapshot.snapshot_id.to_string())
        .bind(&snapshot.agent_id)
        .bind(snapshot.timestamp)
        .bind(&snapshot.git_commit)
        .bind(&snapshot.description)
        .bind(metadata_json)
        .bind(agent_state_json)
        .execute(&mut *tx)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to create snapshot: {}", e)))?;

        // Link events to snapshot
        for event in &snapshot.events {
            sqlx::query("INSERT INTO snapshot_events (snapshot_id, event_id) VALUES (?, ?)")
                .bind(snapshot.snapshot_id.to_string())
                .bind(event.event_id.to_string())
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    StateStoreError::DatabaseError(format!(
                        "Failed to link event to snapshot: {}",
                        e
                    ))
                })?;
        }

        tx.commit().await.map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to commit transaction: {}", e))
        })?;

        Ok(())
    }

    async fn get_snapshot(&self, snapshot_id: &Uuid) -> StateStoreResult<Option<HistorySnapshot>> {
        let row = sqlx::query(
            r#"
            SELECT snapshot_id, agent_id, timestamp, git_commit, description, metadata, agent_state
            FROM history_snapshots
            WHERE snapshot_id = ?
            "#,
        )
        .bind(snapshot_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch snapshot: {}", e)))?;

        if let Some(row) = row {
            let snapshot_id_str: String = row.get("snapshot_id");
            let agent_id: String = row.get("agent_id");
            let timestamp: i64 = row.get("timestamp");
            let git_commit: Option<String> = row.get("git_commit");
            let description: Option<String> = row.get("description");
            let metadata_str: Option<String> = row.get("metadata");
            let agent_state_str: Option<String> = row.get("agent_state");

            let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());
            let agent_state = agent_state_str.and_then(|s| serde_json::from_str(&s).ok());

            // Get events for this snapshot
            let event_rows = sqlx::query(
                r#"
                SELECT e.event_id, e.agent_id, e.timestamp, e.event_type, e.event_data,
                       e.git_commit_hash, e.session_id, e.parent_event_id, e.tags, e.metadata
                FROM agent_history_events e
                INNER JOIN snapshot_events se ON e.event_id = se.event_id
                WHERE se.snapshot_id = ?
                ORDER BY e.timestamp ASC
                "#,
            )
            .bind(&snapshot_id_str)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to fetch snapshot events: {}", e))
            })?;

            let events = self.rows_to_events(event_rows)?;

            Ok(Some(HistorySnapshot {
                snapshot_id: Uuid::parse_str(&snapshot_id_str).unwrap_or_else(|_| Uuid::new_v4()),
                agent_id,
                timestamp,
                events,
                git_commit,
                description,
                metadata,
                agent_state,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_snapshots(&self, agent_id: &str) -> StateStoreResult<Vec<HistorySnapshot>> {
        let rows = sqlx::query(
            r#"
            SELECT snapshot_id, agent_id, timestamp, git_commit, description, metadata, agent_state
            FROM history_snapshots
            WHERE agent_id = ?
            ORDER BY timestamp DESC
            "#,
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to list snapshots: {}", e)))?;

        let mut snapshots = Vec::new();

        for row in rows {
            let snapshot_id_str: String = row.get("snapshot_id");
            let snapshot_id = Uuid::parse_str(&snapshot_id_str).unwrap_or_else(|_| Uuid::new_v4());

            if let Some(snapshot) = self.get_snapshot(&snapshot_id).await? {
                snapshots.push(snapshot);
            }
        }

        Ok(snapshots)
    }

    async fn delete_events_before(&self, timestamp: i64) -> StateStoreResult<i64> {
        let result = sqlx::query("DELETE FROM agent_history_events WHERE timestamp < ?")
            .bind(timestamp)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to delete old events: {}", e))
            })?;

        Ok(result.rows_affected() as i64)
    }

    async fn get_statistics(&self, agent_id: &str) -> StateStoreResult<HistoryStatistics> {
        // Get total events
        let total_events: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM agent_history_events WHERE agent_id = ?")
                .bind(agent_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    StateStoreError::DatabaseError(format!("Failed to get total events: {}", e))
                })?;

        // Get events by type
        let type_rows = sqlx::query(
            "SELECT event_type, COUNT(*) as count FROM agent_history_events WHERE agent_id = ? GROUP BY event_type",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to get events by type: {}", e))
        })?;

        let mut events_by_type = std::collections::HashMap::new();
        for row in type_rows {
            let event_type: String = row.get("event_type");
            let count: i64 = row.get("count");
            events_by_type.insert(event_type, count);
        }

        // Get total snapshots
        let total_snapshots: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM history_snapshots WHERE agent_id = ?")
                .bind(agent_id)
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    StateStoreError::DatabaseError(format!("Failed to get total snapshots: {}", e))
                })?;

        // Get time range
        let time_range = sqlx::query(
            "SELECT MIN(timestamp) as earliest, MAX(timestamp) as latest FROM agent_history_events WHERE agent_id = ?",
        )
        .bind(agent_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to get time range: {}", e))
        })?;

        let (earliest_event, latest_event) = if let Some(row) = time_range {
            (row.get("earliest"), row.get("latest"))
        } else {
            (None, None)
        };

        // Get unique sessions
        let unique_sessions: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT session_id) FROM agent_history_events WHERE agent_id = ? AND session_id IS NOT NULL",
        )
        .bind(agent_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to get unique sessions: {}", e))
        })?;

        // Get git commits
        let commit_rows = sqlx::query(
            "SELECT DISTINCT git_commit_hash FROM agent_history_events WHERE agent_id = ? AND git_commit_hash IS NOT NULL",
        )
        .bind(agent_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to get git commits: {}", e))
        })?;

        let git_commits: Vec<String> = commit_rows
            .into_iter()
            .filter_map(|row| row.get("git_commit_hash"))
            .collect();

        Ok(HistoryStatistics {
            total_events,
            events_by_type,
            total_snapshots,
            earliest_event,
            latest_event,
            unique_sessions,
            git_commits,
        })
    }

    async fn get_event_chain(&self, event_id: &Uuid) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        let mut chain = Vec::new();
        let mut current_id = Some(*event_id);

        while let Some(id) = current_id {
            let row = sqlx::query(
                r#"
                SELECT event_id, agent_id, timestamp, event_type, event_data, git_commit_hash,
                       session_id, parent_event_id, tags, metadata
                FROM agent_history_events
                WHERE event_id = ?
                "#,
            )
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch event: {}", e)))?;

            if let Some(row) = row {
                let parent_id_str: Option<String> = row.get("parent_event_id");
                let event = self.row_to_event(&row)?;
                chain.push(event);

                current_id = parent_id_str.and_then(|s| Uuid::parse_str(&s).ok());
            } else {
                break;
            }
        }

        Ok(chain)
    }
}

// Blanket implementation for Arc<T> where T: AgentHistoryStore
// This allows Arc-wrapped stores to be used directly without dereferencing,
// while still enforcing that initialization happens before sharing clones.
#[async_trait]
impl<T: AgentHistoryStore> AgentHistoryStore for Arc<T> {
    async fn initialize(&mut self) -> StateStoreResult<()> {
        if let Some(inner) = Arc::get_mut(self) {
            inner.initialize().await
        } else {
            Err(StateStoreError::DatabaseError(
                "Cannot initialize Arc-cloned AgentHistoryStore; call initialize() before cloning the Arc"
                    .to_string(),
            ))
        }
    }

    async fn record_event(&self, event: &AgentHistoryEvent) -> StateStoreResult<()> {
        (**self).record_event(event).await
    }

    async fn record_events(&self, events: &[AgentHistoryEvent]) -> StateStoreResult<()> {
        (**self).record_events(events).await
    }

    async fn get_events(
        &self,
        agent_id: &str,
        limit: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        (**self).get_events(agent_id, limit).await
    }

    async fn query_events(&self, query: &HistoryQuery) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        (**self).query_events(query).await
    }

    async fn get_events_by_type(
        &self,
        agent_id: &str,
        event_type: HistoryEventType,
        limit: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        (**self)
            .get_events_by_type(agent_id, event_type, limit)
            .await
    }

    async fn get_events_by_time_range(
        &self,
        agent_id: &str,
        start_time: i64,
        end_time: i64,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        (**self)
            .get_events_by_time_range(agent_id, start_time, end_time)
            .await
    }

    async fn get_events_by_session(
        &self,
        session_id: &str,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        (**self).get_events_by_session(session_id).await
    }

    async fn create_snapshot(&self, snapshot: &HistorySnapshot) -> StateStoreResult<()> {
        (**self).create_snapshot(snapshot).await
    }

    async fn get_snapshot(&self, snapshot_id: &Uuid) -> StateStoreResult<Option<HistorySnapshot>> {
        (**self).get_snapshot(snapshot_id).await
    }

    async fn list_snapshots(&self, agent_id: &str) -> StateStoreResult<Vec<HistorySnapshot>> {
        (**self).list_snapshots(agent_id).await
    }

    async fn delete_events_before(&self, timestamp: i64) -> StateStoreResult<i64> {
        (**self).delete_events_before(timestamp).await
    }

    async fn get_statistics(&self, agent_id: &str) -> StateStoreResult<HistoryStatistics> {
        (**self).get_statistics(agent_id).await
    }

    async fn get_event_chain(&self, event_id: &Uuid) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        (**self).get_event_chain(event_id).await
    }
}

// Helper methods for SqliteAgentHistoryStore
impl SqliteAgentHistoryStore {
    fn rows_to_events(
        &self,
        rows: Vec<sqlx::sqlite::SqliteRow>,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        rows.iter().map(|row| self.row_to_event(row)).collect()
    }

    fn row_to_event(&self, row: &sqlx::sqlite::SqliteRow) -> StateStoreResult<AgentHistoryEvent> {
        let event_id_str: String = row.get("event_id");
        let event_type_str: String = row.get("event_type");
        let event_data_str: String = row.get("event_data");
        let tags_str: String = row.get("tags");
        let metadata_str: Option<String> = row.get("metadata");
        let parent_event_id_str: Option<String> = row.get("parent_event_id");

        let event_id = Uuid::parse_str(&event_id_str).map_err(|e| {
            StateStoreError::SerializationError(format!("Failed to parse event_id: {}", e))
        })?;

        let event_type = HistoryEventType::from_str(&event_type_str).map_err(|e| {
            StateStoreError::SerializationError(format!("Failed to parse event_type: {}", e))
        })?;

        let event_data: Value = serde_json::from_str(&event_data_str).map_err(|e| {
            StateStoreError::SerializationError(format!("Failed to parse event_data: {}", e))
        })?;

        let tags: Vec<String> = serde_json::from_str(&tags_str).map_err(|e| {
            StateStoreError::SerializationError(format!("Failed to parse tags: {}", e))
        })?;

        let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

        let parent_event_id = parent_event_id_str.and_then(|s| Uuid::parse_str(&s).ok());

        Ok(AgentHistoryEvent {
            event_id,
            agent_id: row.get("agent_id"),
            timestamp: row.get("timestamp"),
            event_type,
            event_data,
            git_commit_hash: row.get("git_commit_hash"),
            session_id: row.get("session_id"),
            parent_event_id,
            tags,
            metadata,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{NamedTempFile, TempDir};

    async fn create_uninitialized_store() -> (SqliteAgentHistoryStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("agent_history.db");
        let store = SqliteAgentHistoryStore::new(&db_path).await.unwrap();
        (store, temp_dir)
    }

    async fn create_test_store() -> SqliteAgentHistoryStore {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        let mut store = SqliteAgentHistoryStore::new(path).await.unwrap();
        store.initialize().await.unwrap();
        store
    }

    #[tokio::test]
    async fn test_record_and_retrieve_event() {
        let store = create_test_store().await;

        let event = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            serde_json::json!({"content": "thinking about the problem"}),
        );

        store.record_event(&event).await.unwrap();

        let events = store.get_events("agent-1", 10).await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, HistoryEventType::Thought);
    }

    #[tokio::test]
    async fn test_record_events_batch() {
        let store = create_test_store().await;

        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Action,
                serde_json::json!({"action": "read_file"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::ToolUse,
                serde_json::json!({"tool": "grep"}),
            ),
        ];

        store.record_events(&events).await.unwrap();

        let retrieved = store.get_events("agent-1", 10).await.unwrap();
        assert_eq!(retrieved.len(), 2);
    }

    #[tokio::test]
    async fn test_query_events_by_type() {
        let store = create_test_store().await;

        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                serde_json::json!({"content": "thinking"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Action,
                serde_json::json!({"action": "execute"}),
            ),
        ];

        store.record_events(&events).await.unwrap();

        let thoughts = store
            .get_events_by_type("agent-1", HistoryEventType::Thought, 10)
            .await
            .unwrap();

        assert_eq!(thoughts.len(), 1);
        assert_eq!(thoughts[0].event_type, HistoryEventType::Thought);
    }

    #[tokio::test]
    async fn test_create_and_retrieve_snapshot() {
        let store = create_test_store().await;

        let event = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::StateChange,
            serde_json::json!({"from": "idle", "to": "running"}),
        );

        store.record_event(&event).await.unwrap();

        let snapshot = HistorySnapshot::new(
            "agent-1".to_string(),
            vec![event],
            Some("abc123".to_string()),
        )
        .with_description("Test snapshot".to_string());

        store.create_snapshot(&snapshot).await.unwrap();

        let retrieved = store.get_snapshot(&snapshot.snapshot_id).await.unwrap();
        assert!(retrieved.is_some());
        let retrieved = retrieved.unwrap();
        assert_eq!(retrieved.agent_id, "agent-1");
        assert_eq!(retrieved.events.len(), 1);
        assert_eq!(retrieved.git_commit, Some("abc123".to_string()));
    }

    #[tokio::test]
    async fn test_get_statistics() {
        let store = create_test_store().await;

        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                serde_json::json!({"content": "thinking"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Action,
                serde_json::json!({"action": "execute"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                serde_json::json!({"content": "more thinking"}),
            ),
        ];

        store.record_events(&events).await.unwrap();

        let stats = store.get_statistics("agent-1").await.unwrap();
        assert_eq!(stats.total_events, 3);
        assert_eq!(stats.events_by_type.get("thought"), Some(&2));
        assert_eq!(stats.events_by_type.get("action"), Some(&1));
    }

    #[tokio::test]
    async fn test_event_chain() {
        let store = create_test_store().await;

        let event1 = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            serde_json::json!({"content": "first thought"}),
        );

        store.record_event(&event1).await.unwrap();

        let event2 = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Action,
            serde_json::json!({"action": "based on thought"}),
        )
        .with_parent(event1.event_id);

        store.record_event(&event2).await.unwrap();

        let chain = store.get_event_chain(&event2.event_id).await.unwrap();
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].event_id, event2.event_id);
        assert_eq!(chain[1].event_id, event1.event_id);
    }

    #[tokio::test]
    async fn test_time_range_query() {
        let store = create_test_store().await;

        let start_time = Utc::now().timestamp();

        let event1 = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            serde_json::json!({"content": "early thought"}),
        );

        store.record_event(&event1).await.unwrap();

        // Sleep to ensure different timestamp
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        let mid_time = Utc::now().timestamp();

        let event2 = AgentHistoryEvent::new(
            "agent-1".to_string(),
            HistoryEventType::Thought,
            serde_json::json!({"content": "later thought"}),
        );

        store.record_event(&event2).await.unwrap();

        let end_time = Utc::now().timestamp();

        // Query full range
        let all_events = store
            .get_events_by_time_range("agent-1", start_time, end_time)
            .await
            .unwrap();
        assert_eq!(all_events.len(), 2);

        // Query first half
        let early_events = store
            .get_events_by_time_range("agent-1", start_time, mid_time)
            .await
            .unwrap();
        assert_eq!(early_events.len(), 1);
    }

    #[tokio::test]
    async fn arc_store_initialize_succeeds_when_unique() {
        let (store, _temp_dir) = create_uninitialized_store().await;
        let mut arc_store = Arc::new(store);

        AgentHistoryStore::initialize(&mut arc_store)
            .await
            .expect("initialize should succeed for unique Arc");

        let event = AgentHistoryEvent::new(
            "agent-unique".to_string(),
            HistoryEventType::Thought,
            serde_json::json!({"content": "initialized via Arc"}),
        );

        arc_store.record_event(&event).await.unwrap();
        let events = arc_store.get_events("agent-unique", 10).await.unwrap();
        assert_eq!(events.len(), 1);
    }

    #[tokio::test]
    async fn arc_store_initialize_fails_when_cloned() {
        let (store, _temp_dir) = create_uninitialized_store().await;
        let mut arc_store = Arc::new(store);
        let _clone = Arc::clone(&arc_store);

        let err = AgentHistoryStore::initialize(&mut arc_store)
            .await
            .expect_err("initialize should fail when Arc has multiple owners");

        match err {
            StateStoreError::DatabaseError(msg) => {
                assert!(
                    msg.contains("Arc-cloned"),
                    "unexpected error message: {}",
                    msg
                );
            }
            other => panic!("expected DatabaseError, got {:?}", other),
        }
    }
}
