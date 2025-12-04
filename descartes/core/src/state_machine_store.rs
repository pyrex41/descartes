//! SQLite-based state machine persistence using the StateStore trait
//!
//! Provides production-ready persistent storage for workflow states with:
//! - Async SQLite operations via sqlx
//! - History retention policies
//! - Checkpoint and recovery support
//! - State snapshots for recovery

use crate::state_machine::*;
use chrono::Utc;
use serde_json::json;
use sqlx::sqlite::SqlitePool;
use std::sync::Arc;

// ============================================================================
// PERSISTENCE MODELS
// ============================================================================

/// Persistent record of a workflow in SQLite
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WorkflowRecord {
    pub id: i64,
    pub workflow_id: String,
    pub current_state: String,
    pub context: String, // JSON
    pub created_at: String,
    pub updated_at: String,
    pub is_terminal: bool,
}

/// Persistent record of a state transition in history
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TransitionRecord {
    pub id: i64,
    pub workflow_id: String,
    pub transition_id: String,
    pub from_state: String,
    pub to_state: String,
    pub event: String,
    pub duration_ms: i64,
    pub error_message: Option<String>,
    pub handler_details: Option<String>,
    pub context_snapshot: String, // JSON
    pub created_at: String,
}

/// Configuration for state store
#[derive(Debug, Clone)]
pub struct StateStoreConfig {
    /// Maximum history entries to retain per workflow
    pub max_history_per_workflow: usize,
    /// Enable automatic cleanup of old history
    pub enable_history_cleanup: bool,
    /// Days of history to retain
    pub history_retention_days: u32,
    /// Checkpoint interval (number of transitions)
    pub checkpoint_interval: u32,
}

impl Default for StateStoreConfig {
    fn default() -> Self {
        Self {
            max_history_per_workflow: 10000,
            enable_history_cleanup: true,
            history_retention_days: 90,
            checkpoint_interval: 100,
        }
    }
}

// ============================================================================
// SQLITE STATE STORE IMPLEMENTATION
// ============================================================================

/// SQLite-based implementation of workflow state persistence
pub struct SqliteWorkflowStore {
    pool: SqlitePool,
    config: StateStoreConfig,
}

impl SqliteWorkflowStore {
    /// Create a new SQLite workflow store
    pub async fn new(database_url: &str, config: StateStoreConfig) -> Result<Self, sqlx::Error> {
        let pool = SqlitePool::connect(database_url).await?;

        let store = Self { pool, config };
        store.initialize_schema().await?;

        Ok(store)
    }

    /// Initialize database schema
    pub async fn initialize_schema(&self) -> Result<(), sqlx::Error> {
        // Create workflows table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflows (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workflow_id TEXT NOT NULL UNIQUE,
                current_state TEXT NOT NULL,
                context TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                is_terminal INTEGER NOT NULL DEFAULT 0
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create transitions history table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS state_transitions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workflow_id TEXT NOT NULL,
                transition_id TEXT NOT NULL UNIQUE,
                from_state TEXT NOT NULL,
                to_state TEXT NOT NULL,
                event TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                error_message TEXT,
                handler_details TEXT,
                context_snapshot TEXT NOT NULL DEFAULT '{}',
                created_at TEXT NOT NULL,
                FOREIGN KEY (workflow_id) REFERENCES workflows(workflow_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        // Create indexes separately for SQLite compatibility
        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_workflow_id ON state_transitions(workflow_id)"#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_created_at ON state_transitions(created_at)"#,
        )
        .execute(&self.pool)
        .await?;

        // Create checkpoints table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS workflow_checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                workflow_id TEXT NOT NULL UNIQUE,
                transition_count INTEGER NOT NULL,
                serialized_state TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY (workflow_id) REFERENCES workflows(workflow_id)
            )
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Save or update a workflow state
    pub async fn save_workflow(&self, workflow: &WorkflowStateMachine) -> Result<(), sqlx::Error> {
        let serialized = workflow.serialize().await.map_err(|e| {
            sqlx::Error::Io(std::io::Error::new(
                std::io::ErrorKind::Other,
                e.to_string(),
            ))
        })?;

        let now = Utc::now().to_rfc3339();
        let is_terminal = serialized.current_state.is_terminal();
        let context_json = serde_json::to_string(&serialized.context).unwrap_or_default();

        // Upsert workflow record
        sqlx::query(
            r#"
            INSERT INTO workflows (workflow_id, current_state, context, created_at, updated_at, is_terminal)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(workflow_id) DO UPDATE SET
                current_state = excluded.current_state,
                context = excluded.context,
                updated_at = excluded.updated_at,
                is_terminal = excluded.is_terminal
            "#,
        )
        .bind(&serialized.workflow_id)
        .bind(format!("{:?}", serialized.current_state))
        .bind(context_json)
        .bind(&serialized.metadata.created_at)
        .bind(now)
        .bind(is_terminal as i32)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load a workflow state from storage
    pub async fn load_workflow(
        &self,
        workflow_id: &str,
    ) -> Result<SerializedWorkflow, sqlx::Error> {
        let record =
            sqlx::query_as::<_, WorkflowRecord>("SELECT * FROM workflows WHERE workflow_id = ?")
                .bind(workflow_id)
                .fetch_one(&self.pool)
                .await?;

        let history = sqlx::query_as::<_, TransitionRecord>(
            r#"
            SELECT * FROM state_transitions
            WHERE workflow_id = ?
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(workflow_id)
        .bind(self.config.max_history_per_workflow as i64)
        .fetch_all(&self.pool)
        .await?;

        let state = match record.current_state.as_str() {
            "Running" => WorkflowState::Running,
            "Paused" => WorkflowState::Paused,
            "Completed" => WorkflowState::Completed,
            "Failed" => WorkflowState::Failed,
            _ => WorkflowState::Idle,
        };

        let context: serde_json::Value = serde_json::from_str(&record.context).unwrap_or(json!({}));

        let history_entries: Vec<StateHistoryEntry> = history
            .into_iter()
            .filter_map(|rec| {
                let from_state = match rec.from_state.as_str() {
                    "Running" => WorkflowState::Running,
                    "Paused" => WorkflowState::Paused,
                    "Completed" => WorkflowState::Completed,
                    "Failed" => WorkflowState::Failed,
                    _ => WorkflowState::Idle,
                };

                let to_state = match rec.to_state.as_str() {
                    "Running" => WorkflowState::Running,
                    "Paused" => WorkflowState::Paused,
                    "Completed" => WorkflowState::Completed,
                    "Failed" => WorkflowState::Failed,
                    _ => WorkflowState::Idle,
                };

                let context_snapshot =
                    serde_json::from_str(&rec.context_snapshot).unwrap_or(json!({}));

                Some(StateHistoryEntry {
                    transition: TransitionMetadata {
                        transition_id: rec.transition_id,
                        timestamp: rec.created_at,
                        from_state,
                        to_state,
                        event: rec.event,
                        duration_ms: rec.duration_ms as u64,
                        error: rec.error_message,
                        handler_details: rec.handler_details,
                    },
                    context_snapshot,
                })
            })
            .collect();

        let metadata = WorkflowMetadata {
            workflow_id: record.workflow_id.clone(),
            current_state: state,
            created_at: record.created_at.clone(),
            last_transition_at: record.updated_at.clone(),
            history_size: history_entries.len(),
        };

        Ok(SerializedWorkflow {
            workflow_id: record.workflow_id,
            current_state: state,
            history: history_entries,
            context,
            metadata,
        })
    }

    /// Save a state transition to history
    pub async fn save_transition(
        &self,
        workflow_id: &str,
        transition: &TransitionMetadata,
        context_snapshot: &serde_json::Value,
    ) -> Result<(), sqlx::Error> {
        let context_json = serde_json::to_string(context_snapshot).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO state_transitions
            (workflow_id, transition_id, from_state, to_state, event, duration_ms,
             error_message, handler_details, context_snapshot, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(workflow_id)
        .bind(&transition.transition_id)
        .bind(format!("{:?}", transition.from_state))
        .bind(format!("{:?}", transition.to_state))
        .bind(&transition.event)
        .bind(transition.duration_ms as i64)
        .bind(&transition.error)
        .bind(&transition.handler_details)
        .bind(context_json)
        .bind(&transition.timestamp)
        .execute(&self.pool)
        .await?;

        // Cleanup old history if needed
        if self.config.enable_history_cleanup {
            self.cleanup_old_history(workflow_id).await.ok();
        }

        Ok(())
    }

    /// Get workflow history
    pub async fn get_workflow_history(
        &self,
        workflow_id: &str,
    ) -> Result<Vec<StateHistoryEntry>, sqlx::Error> {
        let records = sqlx::query_as::<_, TransitionRecord>(
            r#"
            SELECT * FROM state_transitions
            WHERE workflow_id = ?
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(workflow_id)
        .bind(self.config.max_history_per_workflow as i64)
        .fetch_all(&self.pool)
        .await?;

        let entries = records
            .into_iter()
            .filter_map(|rec| {
                let from_state = match rec.from_state.as_str() {
                    "Running" => WorkflowState::Running,
                    "Paused" => WorkflowState::Paused,
                    "Completed" => WorkflowState::Completed,
                    "Failed" => WorkflowState::Failed,
                    _ => WorkflowState::Idle,
                };

                let to_state = match rec.to_state.as_str() {
                    "Running" => WorkflowState::Running,
                    "Paused" => WorkflowState::Paused,
                    "Completed" => WorkflowState::Completed,
                    "Failed" => WorkflowState::Failed,
                    _ => WorkflowState::Idle,
                };

                let context_snapshot =
                    serde_json::from_str(&rec.context_snapshot).unwrap_or(json!({}));

                Some(StateHistoryEntry {
                    transition: TransitionMetadata {
                        transition_id: rec.transition_id,
                        timestamp: rec.created_at,
                        from_state,
                        to_state,
                        event: rec.event,
                        duration_ms: rec.duration_ms as u64,
                        error: rec.error_message,
                        handler_details: rec.handler_details,
                    },
                    context_snapshot,
                })
            })
            .collect();

        Ok(entries)
    }

    /// Create a checkpoint of current workflow state
    pub async fn create_checkpoint(
        &self,
        workflow_id: &str,
        transition_count: u32,
        serialized_state: &SerializedWorkflow,
    ) -> Result<(), sqlx::Error> {
        let state_json = serde_json::to_string(serialized_state).unwrap_or_default();

        sqlx::query(
            r#"
            INSERT INTO workflow_checkpoints
            (workflow_id, transition_count, serialized_state, created_at)
            VALUES (?, ?, ?, ?)
            ON CONFLICT(workflow_id) DO UPDATE SET
                transition_count = excluded.transition_count,
                serialized_state = excluded.serialized_state,
                created_at = excluded.created_at
            "#,
        )
        .bind(workflow_id)
        .bind(transition_count as i64)
        .bind(state_json)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// List all workflows
    pub async fn list_workflows(&self) -> Result<Vec<WorkflowRecord>, sqlx::Error> {
        sqlx::query_as::<_, WorkflowRecord>("SELECT * FROM workflows ORDER BY updated_at DESC")
            .fetch_all(&self.pool)
            .await
    }

    /// Delete a workflow and its history
    pub async fn delete_workflow(&self, workflow_id: &str) -> Result<(), sqlx::Error> {
        // Delete history first (due to foreign key constraint)
        sqlx::query("DELETE FROM state_transitions WHERE workflow_id = ?")
            .bind(workflow_id)
            .execute(&self.pool)
            .await?;

        // Delete checkpoints
        sqlx::query("DELETE FROM workflow_checkpoints WHERE workflow_id = ?")
            .bind(workflow_id)
            .execute(&self.pool)
            .await?;

        // Delete workflow
        sqlx::query("DELETE FROM workflows WHERE workflow_id = ?")
            .bind(workflow_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ============================================================================
    // PRIVATE HELPERS
    // ============================================================================

    async fn cleanup_old_history(&self, workflow_id: &str) -> Result<(), sqlx::Error> {
        // Remove entries beyond max_history_per_workflow limit
        sqlx::query(
            r#"
            DELETE FROM state_transitions
            WHERE workflow_id = ? AND id NOT IN (
                SELECT id FROM state_transitions
                WHERE workflow_id = ?
                ORDER BY created_at DESC
                LIMIT ?
            )
            "#,
        )
        .bind(workflow_id)
        .bind(workflow_id)
        .bind(self.config.max_history_per_workflow as i64)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

// ============================================================================
// RECOVERY UTILITIES
// ============================================================================

/// Helpers for workflow recovery and restoration
pub struct WorkflowRecovery;

impl WorkflowRecovery {
    /// Recover a workflow from persistent storage
    pub async fn recover_workflow(
        store: &SqliteWorkflowStore,
        workflow_id: &str,
    ) -> Result<Arc<WorkflowStateMachine>, Box<dyn std::error::Error>> {
        let serialized = store.load_workflow(workflow_id).await?;
        let sm = WorkflowStateMachine::deserialize(serialized).await?;
        Ok(sm)
    }

    /// Recover all workflows from storage
    pub async fn recover_all_workflows(
        store: &SqliteWorkflowStore,
    ) -> Result<Vec<Arc<WorkflowStateMachine>>, Box<dyn std::error::Error>> {
        let records = store.list_workflows().await?;
        let mut workflows = Vec::new();

        for record in records {
            if let Ok(sm) = Self::recover_workflow(store, &record.workflow_id).await {
                workflows.push(sm);
            }
        }

        Ok(workflows)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_store() -> SqliteWorkflowStore {
        // Use in-memory SQLite for faster tests
        SqliteWorkflowStore::new("sqlite::memory:", StateStoreConfig::default())
            .await
            .expect("Failed to create store")
    }

    #[tokio::test]
    async fn test_schema_initialization() {
        let store = create_test_store().await;

        // Verify tables exist
        let workflows = store.list_workflows().await.expect("Failed to list");
        assert_eq!(workflows.len(), 0);
    }

    #[tokio::test]
    async fn test_save_and_load_workflow() {
        let store = create_test_store().await;

        // Create and save workflow
        let sm = Arc::new(WorkflowStateMachine::new("test-workflow".to_string()));
        sm.process_event(WorkflowEvent::Start).await.unwrap();
        sm.set_context("key", serde_json::json!("value"))
            .await
            .unwrap();

        store.save_workflow(&sm).await.expect("Failed to save");

        // Load and verify
        let loaded = store
            .load_workflow("test-workflow")
            .await
            .expect("Failed to load");

        assert_eq!(loaded.workflow_id, "test-workflow");
        assert_eq!(loaded.current_state, WorkflowState::Running);
        assert_eq!(loaded.context.get("key"), Some(&serde_json::json!("value")));
    }
}
