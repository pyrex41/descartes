/// SQLite-backed implementation of the StateStore trait
/// Provides persistent agent state management with history tracking and migrations
use crate::errors::{StateStoreError, StateStoreResult};
use crate::traits::{ActorType, Event, StateStore, Task, TaskStatus};
use async_trait::async_trait;
use chrono::Utc;
use serde_json::{json, Value};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use uuid::Uuid;

/// SQLite-backed state store implementation
pub struct SqliteStateStore {
    /// Connection pool to SQLite database
    pool: SqlitePool,

    /// Path to the SQLite database file
    _db_path: PathBuf,

    /// Optional prefix for agent state keys
    key_prefix: Option<String>,

    /// Enable state compression
    _enable_compression: bool,
}

/// Agent state snapshot for persistence
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgentState {
    /// Unique agent identifier
    pub agent_id: String,

    /// Agent name
    pub name: String,

    /// Current agent status
    pub status: String,

    /// Agent metadata
    pub metadata: Value,

    /// Serialized state data
    pub state_data: String,

    /// Creation timestamp
    pub created_at: i64,

    /// Last update timestamp
    pub updated_at: i64,

    /// Version number for state evolution
    pub version: i32,
}

/// State transition record for history tracking
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StateTransition {
    /// Unique transition identifier
    pub id: String,

    /// Associated agent ID
    pub agent_id: String,

    /// State before transition
    pub state_before: String,

    /// State after transition
    pub state_after: String,

    /// Reason for transition
    pub reason: Option<String>,

    /// Transition timestamp
    pub timestamp: i64,

    /// Metadata about the transition
    pub metadata: Option<Value>,
}

/// Database migration record
#[derive(Debug, Clone)]
pub struct Migration {
    pub version: i32,
    pub name: String,
    pub description: Option<String>,
    pub applied_at: i64,
}

impl SqliteStateStore {
    /// Create a new SQLite state store
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    /// * `enable_compression` - Whether to compress state data
    pub async fn new<P: AsRef<Path>>(
        db_path: P,
        enable_compression: bool,
    ) -> StateStoreResult<Self> {
        let db_path = db_path.as_ref().to_path_buf();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    StateStoreError::DatabaseError(format!("Failed to create directory: {}", e))
                })?;
            }
        }

        // Create connection options with foreign keys enabled
        let connect_options = SqliteConnectOptions::from_str(db_path.to_string_lossy().as_ref())
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to parse database path: {}", e))
            })?
            .create_if_missing(true)
            .foreign_keys(true);

        // Create connection pool with reasonable defaults
        let pool = SqlitePoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .connect_with(connect_options)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to create database pool: {}", e))
            })?;

        Ok(SqliteStateStore {
            pool,
            _db_path: db_path,
            key_prefix: None,
            _enable_compression: enable_compression,
        })
    }

    /// Set a key prefix for all state keys
    pub fn with_prefix(mut self, prefix: String) -> Self {
        self.key_prefix = Some(prefix);
        self
    }

    /// Get the full key with optional prefix
    fn get_full_key(&self, key: &str) -> String {
        match &self.key_prefix {
            Some(prefix) => format!("{}:{}", prefix, key),
            None => key.to_string(),
        }
    }

    /// Apply all pending migrations
    async fn apply_migrations(&self) -> StateStoreResult<()> {
        // Create migrations table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS migrations (
                version INTEGER PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                applied_at INTEGER NOT NULL,
                rollback_script TEXT
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::MigrationError(format!("Failed to create migrations table: {}", e))
        })?;

        // Get current schema version
        let max_version: i32 =
            sqlx::query_scalar("SELECT COALESCE(MAX(version), 0) FROM migrations")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    StateStoreError::MigrationError(format!(
                        "Failed to query migration version: {}",
                        e
                    ))
                })?;

        // Define migrations as inline SQL strings
        let migrations: Vec<(i32, &str, &str, Vec<&str>)> = vec![
            (
                1,
                "create_agent_states",
                "Create agent states table",
                vec![
                    r#"CREATE TABLE IF NOT EXISTS agent_states (
                        key TEXT PRIMARY KEY NOT NULL,
                        agent_id TEXT NOT NULL UNIQUE,
                        name TEXT NOT NULL,
                        status TEXT NOT NULL DEFAULT 'idle',
                        metadata TEXT NOT NULL DEFAULT '{}',
                        state_data TEXT NOT NULL,
                        version INTEGER NOT NULL DEFAULT 1,
                        created_at INTEGER NOT NULL,
                        updated_at INTEGER NOT NULL,
                        is_deleted INTEGER NOT NULL DEFAULT 0
                    )"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_agent_states_agent_id ON agent_states(agent_id)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_agent_states_status ON agent_states(status) WHERE is_deleted = 0"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_agent_states_updated_at ON agent_states(updated_at) WHERE is_deleted = 0"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_agent_states_created_at ON agent_states(created_at)"#,
                ],
            ),
            (
                2,
                "create_state_transitions",
                "Create state transition history table",
                vec![
                    r#"CREATE TABLE IF NOT EXISTS state_transitions (
                        id TEXT PRIMARY KEY NOT NULL,
                        agent_id TEXT NOT NULL,
                        state_before TEXT NOT NULL,
                        state_after TEXT NOT NULL,
                        reason TEXT,
                        timestamp INTEGER NOT NULL,
                        metadata TEXT
                    )"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_state_transitions_agent_id ON state_transitions(agent_id)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_state_transitions_timestamp ON state_transitions(timestamp)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_state_transitions_agent_timestamp ON state_transitions(agent_id, timestamp)"#,
                ],
            ),
            (
                3,
                "create_state_snapshots",
                "Create state snapshots table",
                vec![
                    r#"CREATE TABLE IF NOT EXISTS state_snapshots (
                        id TEXT PRIMARY KEY NOT NULL,
                        agent_id TEXT NOT NULL,
                        state_data TEXT NOT NULL,
                        description TEXT,
                        created_at INTEGER NOT NULL,
                        expires_at INTEGER
                    )"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_state_snapshots_agent_id ON state_snapshots(agent_id)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_state_snapshots_created_at ON state_snapshots(created_at)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_state_snapshots_agent_created ON state_snapshots(agent_id, created_at DESC)"#,
                ],
            ),
            (
                4,
                "add_state_indexes",
                "Add performance indexes",
                vec![
                    r#"CREATE INDEX IF NOT EXISTS idx_events_session_timestamp ON events(session_id, timestamp DESC)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_events_actor_id ON events(actor_id)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_events_type_timestamp ON events(event_type, timestamp DESC)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_events_content_search ON events(content) WHERE content IS NOT NULL"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_tasks_assigned_to ON tasks(assigned_to)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_tasks_status_updated ON tasks(status, updated_at DESC)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_sessions_agent_id ON sessions(agent_id)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_sessions_started_at ON sessions(started_at DESC)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_sessions_status ON sessions(status)"#,
                ],
            ),
            (
                5,
                "enhance_task_model",
                "Add priority, complexity, and dependencies to tasks",
                vec![
                    r#"CREATE TABLE IF NOT EXISTS task_dependencies (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        task_id TEXT NOT NULL,
                        depends_on_task_id TEXT NOT NULL,
                        created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now')),
                        FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
                        FOREIGN KEY (depends_on_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
                        UNIQUE(task_id, depends_on_task_id)
                    )"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_tasks_priority ON tasks(priority)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_tasks_complexity ON tasks(complexity)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_tasks_priority_status ON tasks(priority, status)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_tasks_complexity_status ON tasks(complexity, status)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_task_dependencies_task_id ON task_dependencies(task_id)"#,
                    r#"CREATE INDEX IF NOT EXISTS idx_task_dependencies_depends_on ON task_dependencies(depends_on_task_id)"#,
                ],
            ),
        ];

        // Apply pending migrations
        for (version, name, desc, statements) in migrations {
            if version > max_version {
                for statement in statements {
                    sqlx::query(statement)
                        .execute(&self.pool)
                        .await
                        .map_err(|e| {
                            StateStoreError::MigrationError(format!(
                                "Failed to apply migration {}: {}",
                                name, e
                            ))
                        })?;
                }

                // Record migration
                let now = Utc::now().timestamp();
                sqlx::query(
                    "INSERT INTO migrations (version, name, description, applied_at) VALUES (?, ?, ?, ?)"
                )
                .bind(version)
                .bind(name)
                .bind(Some(desc))
                .bind(now)
                .execute(&self.pool)
                .await
                .map_err(|e| StateStoreError::MigrationError(
                    format!("Failed to record migration {}: {}", name, e)
                ))?;
            }
        }

        Ok(())
    }
}

#[async_trait]
impl StateStore for SqliteStateStore {
    async fn initialize(&mut self) -> StateStoreResult<()> {
        // Create base tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS events (
                id TEXT PRIMARY KEY NOT NULL,
                event_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                session_id TEXT NOT NULL,
                actor_type TEXT NOT NULL,
                actor_id TEXT NOT NULL,
                content TEXT NOT NULL,
                metadata TEXT,
                git_commit TEXT,
                created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to create events table: {}", e))
        })?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY NOT NULL,
                title TEXT NOT NULL,
                description TEXT,
                status TEXT NOT NULL,
                priority TEXT NOT NULL DEFAULT 'medium',
                complexity TEXT NOT NULL DEFAULT 'moderate',
                assigned_to TEXT,
                dependencies TEXT DEFAULT '[]',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                metadata TEXT,
                created_timestamp INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to create tasks table: {}", e))
        })?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY NOT NULL,
                agent_id TEXT NOT NULL,
                started_at INTEGER NOT NULL,
                ended_at INTEGER,
                status TEXT NOT NULL,
                metadata TEXT,
                created_at INTEGER NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to create sessions table: {}", e))
        })?;

        // Apply schema migrations
        self.apply_migrations().await?;

        Ok(())
    }

    async fn save_event(&self, event: &Event) -> StateStoreResult<()> {
        let metadata = event.metadata.as_ref().map(|m| m.to_string());

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO events
            (id, event_type, timestamp, session_id, actor_type, actor_id, content, metadata, git_commit)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(event.id.to_string())
        .bind(&event.event_type)
        .bind(event.timestamp)
        .bind(&event.session_id)
        .bind(format!("{:?}", event.actor_type))
        .bind(&event.actor_id)
        .bind(&event.content)
        .bind(metadata)
        .bind(&event.git_commit)
        .execute(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to save event: {}", e)))?;

        Ok(())
    }

    async fn get_events(&self, session_id: &str) -> StateStoreResult<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT id, event_type, timestamp, session_id, actor_type, actor_id, content, metadata, git_commit
             FROM events WHERE session_id = ? ORDER BY timestamp DESC"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch events: {}", e)))?;

        let events = rows
            .iter()
            .map(|row| {
                let actor_type_str: String = row.get("actor_type");
                let actor_type = match actor_type_str.as_str() {
                    "User" => ActorType::User,
                    "Agent" => ActorType::Agent,
                    "System" => ActorType::System,
                    _ => ActorType::System,
                };

                let metadata_str: Option<String> = row.get("metadata");
                let metadata = metadata_str.and_then(|m| serde_json::from_str(&m).ok());

                Event {
                    id: Uuid::parse_str(&row.get::<String, _>("id"))
                        .unwrap_or_else(|_| Uuid::new_v4()),
                    event_type: row.get("event_type"),
                    timestamp: row.get("timestamp"),
                    session_id: row.get("session_id"),
                    actor_type,
                    actor_id: row.get("actor_id"),
                    content: row.get("content"),
                    metadata,
                    git_commit: row.get("git_commit"),
                }
            })
            .collect();

        Ok(events)
    }

    async fn get_events_by_type(&self, event_type: &str) -> StateStoreResult<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT id, event_type, timestamp, session_id, actor_type, actor_id, content, metadata, git_commit
             FROM events WHERE event_type = ? ORDER BY timestamp DESC"
        )
        .bind(event_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch events by type: {}", e)))?;

        let events = rows
            .iter()
            .map(|row| {
                let actor_type_str: String = row.get("actor_type");
                let actor_type = match actor_type_str.as_str() {
                    "User" => ActorType::User,
                    "Agent" => ActorType::Agent,
                    "System" => ActorType::System,
                    _ => ActorType::System,
                };

                let metadata_str: Option<String> = row.get("metadata");
                let metadata = metadata_str.and_then(|m| serde_json::from_str(&m).ok());

                Event {
                    id: Uuid::parse_str(&row.get::<String, _>("id"))
                        .unwrap_or_else(|_| Uuid::new_v4()),
                    event_type: row.get("event_type"),
                    timestamp: row.get("timestamp"),
                    session_id: row.get("session_id"),
                    actor_type,
                    actor_id: row.get("actor_id"),
                    content: row.get("content"),
                    metadata,
                    git_commit: row.get("git_commit"),
                }
            })
            .collect();

        Ok(events)
    }

    async fn save_task(&self, task: &Task) -> StateStoreResult<()> {
        let metadata = task.metadata.as_ref().map(|m| m.to_string());
        let dependencies_json = serde_json::to_string(&task.dependencies).map_err(|e| {
            StateStoreError::SerializationError(format!("Failed to serialize dependencies: {}", e))
        })?;

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO tasks
            (id, title, description, status, priority, complexity, assigned_to, dependencies, created_at, updated_at, metadata)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(task.id.to_string())
        .bind(&task.title)
        .bind(&task.description)
        .bind(format!("{:?}", task.status))
        .bind(task.priority.to_string())
        .bind(task.complexity.to_string())
        .bind(&task.assigned_to)
        .bind(&dependencies_json)
        .bind(task.created_at)
        .bind(task.updated_at)
        .bind(metadata)
        .execute(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to save task: {}", e)))?;

        // Update task_dependencies table
        // First, delete existing dependencies for this task
        sqlx::query("DELETE FROM task_dependencies WHERE task_id = ?")
            .bind(task.id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to delete old dependencies: {}", e))
            })?;

        // Insert new dependencies
        for dep_id in &task.dependencies {
            sqlx::query(
                "INSERT INTO task_dependencies (task_id, depends_on_task_id, created_at) VALUES (?, ?, ?)"
            )
            .bind(task.id.to_string())
            .bind(dep_id.to_string())
            .bind(chrono::Utc::now().timestamp())
            .execute(&self.pool)
            .await
            .map_err(|e| StateStoreError::DatabaseError(format!("Failed to insert dependency: {}", e)))?;
        }

        Ok(())
    }

    async fn get_task(&self, task_id: &Uuid) -> StateStoreResult<Option<Task>> {
        let row = sqlx::query(
            "SELECT id, title, description, status, priority, complexity, assigned_to, dependencies, created_at, updated_at, metadata FROM tasks WHERE id = ?"
        )
        .bind(task_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch task: {}", e)))?;

        Ok(row.map(|r| {
            use crate::traits::{TaskComplexity, TaskPriority};
            use std::str::FromStr;

            let status_str: String = r.get("status");
            let status = match status_str.as_str() {
                "Todo" => TaskStatus::Todo,
                "InProgress" => TaskStatus::InProgress,
                "Done" => TaskStatus::Done,
                "Blocked" => TaskStatus::Blocked,
                _ => TaskStatus::Todo,
            };

            let priority_str: String = r.get("priority");
            let priority = TaskPriority::from_str(&priority_str).unwrap_or_default();

            let complexity_str: String = r.get("complexity");
            let complexity = TaskComplexity::from_str(&complexity_str).unwrap_or_default();

            let dependencies_str: String = r.get("dependencies");
            let dependencies: Vec<Uuid> =
                serde_json::from_str(&dependencies_str).unwrap_or_default();

            let metadata_str: Option<String> = r.get("metadata");
            let metadata = metadata_str.and_then(|m| serde_json::from_str(&m).ok());

            Task {
                id: Uuid::parse_str(&r.get::<String, _>("id")).unwrap_or_else(|_| Uuid::new_v4()),
                title: r.get("title"),
                description: r.get("description"),
                status,
                priority,
                complexity,
                assigned_to: r.get("assigned_to"),
                dependencies,
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                metadata,
            }
        }))
    }

    async fn get_tasks(&self) -> StateStoreResult<Vec<Task>> {
        let rows = sqlx::query(
            "SELECT id, title, description, status, priority, complexity, assigned_to, dependencies, created_at, updated_at, metadata FROM tasks ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch tasks: {}", e)))?;

        let tasks = rows
            .iter()
            .map(|r| {
                use crate::traits::{TaskComplexity, TaskPriority};
                use std::str::FromStr;

                let status_str: String = r.get("status");
                let status = match status_str.as_str() {
                    "Todo" => TaskStatus::Todo,
                    "InProgress" => TaskStatus::InProgress,
                    "Done" => TaskStatus::Done,
                    "Blocked" => TaskStatus::Blocked,
                    _ => TaskStatus::Todo,
                };

                let priority_str: String = r.get("priority");
                let priority = TaskPriority::from_str(&priority_str).unwrap_or_default();

                let complexity_str: String = r.get("complexity");
                let complexity = TaskComplexity::from_str(&complexity_str).unwrap_or_default();

                let dependencies_str: String = r.get("dependencies");
                let dependencies: Vec<Uuid> =
                    serde_json::from_str(&dependencies_str).unwrap_or_default();

                let metadata_str: Option<String> = r.get("metadata");
                let metadata = metadata_str.and_then(|m| serde_json::from_str(&m).ok());

                Task {
                    id: Uuid::parse_str(&r.get::<String, _>("id"))
                        .unwrap_or_else(|_| Uuid::new_v4()),
                    title: r.get("title"),
                    description: r.get("description"),
                    status,
                    priority,
                    complexity,
                    assigned_to: r.get("assigned_to"),
                    dependencies,
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                    metadata,
                }
            })
            .collect();

        Ok(tasks)
    }

    async fn search_events(&self, query: &str) -> StateStoreResult<Vec<Event>> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            r#"
            SELECT id, event_type, timestamp, session_id, actor_type, actor_id, content, metadata, git_commit
            FROM events
            WHERE content LIKE ? OR event_type LIKE ? OR session_id LIKE ?
            ORDER BY timestamp DESC
            LIMIT 100
            "#,
        )
        .bind(&search_pattern)
        .bind(&search_pattern)
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to search events: {}", e)))?;

        let events = rows
            .iter()
            .map(|row| {
                let actor_type_str: String = row.get("actor_type");
                let actor_type = match actor_type_str.as_str() {
                    "User" => ActorType::User,
                    "Agent" => ActorType::Agent,
                    "System" => ActorType::System,
                    _ => ActorType::System,
                };

                let metadata_str: Option<String> = row.get("metadata");
                let metadata = metadata_str.and_then(|m| serde_json::from_str(&m).ok());

                Event {
                    id: Uuid::parse_str(&row.get::<String, _>("id"))
                        .unwrap_or_else(|_| Uuid::new_v4()),
                    event_type: row.get("event_type"),
                    timestamp: row.get("timestamp"),
                    session_id: row.get("session_id"),
                    actor_type,
                    actor_id: row.get("actor_id"),
                    content: row.get("content"),
                    metadata,
                    git_commit: row.get("git_commit"),
                }
            })
            .collect();

        Ok(events)
    }
}

// Additional agent state management methods (not part of StateStore trait but useful)
impl SqliteStateStore {
    /// Save agent state
    pub async fn save_agent_state(&self, agent_state: &AgentState) -> StateStoreResult<()> {
        let key = self.get_full_key(&agent_state.agent_id);

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO agent_states
            (key, agent_id, name, status, metadata, state_data, version, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&key)
        .bind(&agent_state.agent_id)
        .bind(&agent_state.name)
        .bind(&agent_state.status)
        .bind(agent_state.metadata.to_string())
        .bind(&agent_state.state_data)
        .bind(agent_state.version)
        .bind(agent_state.created_at)
        .bind(Utc::now().timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to save agent state: {}", e))
        })?;

        Ok(())
    }

    /// Load agent state by ID
    pub async fn load_agent_state(&self, agent_id: &str) -> StateStoreResult<Option<AgentState>> {
        let key = self.get_full_key(agent_id);

        let row = sqlx::query(
            "SELECT agent_id, name, status, metadata, state_data, created_at, updated_at, version FROM agent_states WHERE key = ?"
        )
        .bind(&key)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to load agent state: {}", e)))?;

        Ok(row.map(|r| {
            let metadata_str: String = r.get("metadata");
            let metadata = serde_json::from_str(&metadata_str).unwrap_or(json!({}));

            AgentState {
                agent_id: r.get("agent_id"),
                name: r.get("name"),
                status: r.get("status"),
                metadata,
                state_data: r.get("state_data"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                version: r.get("version"),
            }
        }))
    }

    /// List all agent states
    pub async fn list_agents(&self) -> StateStoreResult<Vec<AgentState>> {
        let rows = sqlx::query(
            "SELECT agent_id, name, status, metadata, state_data, created_at, updated_at, version FROM agent_states ORDER BY updated_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to list agents: {}", e)))?;

        let agents = rows
            .iter()
            .map(|r| {
                let metadata_str: String = r.get("metadata");
                let metadata = serde_json::from_str(&metadata_str).unwrap_or(json!({}));

                AgentState {
                    agent_id: r.get("agent_id"),
                    name: r.get("name"),
                    status: r.get("status"),
                    metadata,
                    state_data: r.get("state_data"),
                    created_at: r.get("created_at"),
                    updated_at: r.get("updated_at"),
                    version: r.get("version"),
                }
            })
            .collect();

        Ok(agents)
    }

    /// Update agent status
    pub async fn update_agent_status(
        &self,
        agent_id: &str,
        new_status: &str,
    ) -> StateStoreResult<()> {
        let key = self.get_full_key(agent_id);

        sqlx::query("UPDATE agent_states SET status = ?, updated_at = ? WHERE key = ?")
            .bind(new_status)
            .bind(Utc::now().timestamp())
            .bind(&key)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to update agent status: {}", e))
            })?;

        Ok(())
    }

    /// Delete agent state
    pub async fn delete_agent(&self, agent_id: &str) -> StateStoreResult<()> {
        let key = self.get_full_key(agent_id);

        sqlx::query("DELETE FROM agent_states WHERE key = ?")
            .bind(&key)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to delete agent: {}", e))
            })?;

        Ok(())
    }

    /// Record a state transition
    pub async fn record_state_transition(
        &self,
        agent_id: &str,
        state_before: &str,
        state_after: &str,
        reason: Option<String>,
    ) -> StateStoreResult<()> {
        let transition_id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO state_transitions
            (id, agent_id, state_before, state_after, reason, timestamp)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&transition_id)
        .bind(agent_id)
        .bind(state_before)
        .bind(state_after)
        .bind(reason)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to record state transition: {}", e))
        })?;

        Ok(())
    }

    /// Get state history for an agent
    pub async fn get_state_history(
        &self,
        agent_id: &str,
        limit: i32,
    ) -> StateStoreResult<Vec<StateTransition>> {
        let rows = sqlx::query(
            "SELECT id, agent_id, state_before, state_after, reason, timestamp FROM state_transitions WHERE agent_id = ? ORDER BY timestamp DESC LIMIT ?"
        )
        .bind(agent_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch state history: {}", e)))?;

        let transitions = rows
            .iter()
            .map(|r| StateTransition {
                id: r.get("id"),
                agent_id: r.get("agent_id"),
                state_before: r.get("state_before"),
                state_after: r.get("state_after"),
                reason: r.get("reason"),
                timestamp: r.get("timestamp"),
                metadata: None,
            })
            .collect();

        Ok(transitions)
    }

    /// Create a state snapshot
    pub async fn create_snapshot(
        &self,
        agent_id: &str,
        description: Option<String>,
    ) -> StateStoreResult<String> {
        let snapshot_id = Uuid::new_v4().to_string();
        let now = Utc::now().timestamp();

        // Get current agent state
        if let Some(agent_state) = self.load_agent_state(agent_id).await? {
            sqlx::query(
                r#"
                INSERT INTO state_snapshots
                (id, agent_id, state_data, description, created_at)
                VALUES (?, ?, ?, ?, ?)
                "#,
            )
            .bind(&snapshot_id)
            .bind(agent_id)
            .bind(&agent_state.state_data)
            .bind(description)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to create snapshot: {}", e))
            })?;

            Ok(snapshot_id)
        } else {
            Err(StateStoreError::NotFound(format!(
                "Agent {} not found",
                agent_id
            )))
        }
    }

    /// Restore agent state from a snapshot
    pub async fn restore_snapshot(&self, snapshot_id: &str) -> StateStoreResult<()> {
        let row = sqlx::query("SELECT agent_id, state_data FROM state_snapshots WHERE id = ?")
            .bind(snapshot_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to fetch snapshot: {}", e))
            })?;

        if let Some(r) = row {
            let agent_id: String = r.get("agent_id");
            let state_data: String = r.get("state_data");

            // Update agent state with snapshot data
            sqlx::query(
                "UPDATE agent_states SET state_data = ?, updated_at = ? WHERE agent_id = ?",
            )
            .bind(&state_data)
            .bind(Utc::now().timestamp())
            .bind(&agent_id)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to restore snapshot: {}", e))
            })?;

            Ok(())
        } else {
            Err(StateStoreError::NotFound(format!(
                "Snapshot {} not found",
                snapshot_id
            )))
        }
    }

    /// List all snapshots for an agent
    pub async fn list_snapshots(
        &self,
        agent_id: &str,
    ) -> StateStoreResult<Vec<(String, Option<String>, i64)>> {
        let rows = sqlx::query("SELECT id, description, created_at FROM state_snapshots WHERE agent_id = ? ORDER BY created_at DESC")
            .bind(agent_id)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StateStoreError::DatabaseError(format!("Failed to list snapshots: {}", e)))?;

        let snapshots = rows
            .iter()
            .map(|r| {
                (
                    r.get::<String, _>("id"),
                    r.get::<Option<String>, _>("description"),
                    r.get::<i64, _>("created_at"),
                )
            })
            .collect();

        Ok(snapshots)
    }

    /// Get migration history
    pub async fn get_migration_history(&self) -> StateStoreResult<Vec<Migration>> {
        let rows = sqlx::query(
            "SELECT version, name, description, applied_at FROM migrations ORDER BY version ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::MigrationError(format!("Failed to fetch migrations: {}", e))
        })?;

        let migrations = rows
            .iter()
            .map(|r| Migration {
                version: r.get("version"),
                name: r.get("name"),
                description: r.get("description"),
                applied_at: r.get("applied_at"),
            })
            .collect();

        Ok(migrations)
    }

    /// Execute a transaction with multiple operations
    pub async fn transact<F, T>(&self, f: F) -> StateStoreResult<T>
    where
        F: for<'a> std::future::Future<Output = StateStoreResult<T>>,
    {
        // Start transaction
        let tx = self.pool.begin().await.map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to start transaction: {}", e))
        })?;

        // Execute operations (note: we can't use the transaction here without refactoring)
        // For now, operations auto-commit. A full implementation would need transaction support in queries
        let result = f.await;

        // Commit or rollback
        match result {
            Ok(val) => {
                tx.commit().await.map_err(|e| {
                    StateStoreError::DatabaseError(format!("Failed to commit transaction: {}", e))
                })?;
                Ok(val)
            }
            Err(e) => {
                let _ = tx.rollback().await;
                Err(e)
            }
        }
    }

    /// Get connection pool for advanced operations
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn create_test_store() -> SqliteStateStore {
        // Use in-memory SQLite for faster tests
        let connect_options = SqliteConnectOptions::from_str(":memory:")
            .unwrap()
            .create_if_missing(true)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(connect_options)
            .await
            .unwrap();

        let mut store = SqliteStateStore {
            pool,
            _db_path: std::path::PathBuf::from(":memory:"),
            key_prefix: None,
            _enable_compression: false,
        };
        store.initialize().await.unwrap();
        store
    }

    #[tokio::test]
    async fn test_state_store_creation() {
        let _store = create_test_store().await;
    }

    #[tokio::test]
    async fn test_save_and_load_event() {
        let store = create_test_store().await;

        let event = Event {
            id: Uuid::new_v4(),
            event_type: "test".to_string(),
            timestamp: Utc::now().timestamp(),
            session_id: "session_1".to_string(),
            actor_type: ActorType::Agent,
            actor_id: "agent_1".to_string(),
            content: "Test event".to_string(),
            metadata: None,
            git_commit: None,
        };

        store
            .save_event(&event)
            .await
            .expect("Failed to save event");
        let events = store
            .get_events("session_1")
            .await
            .expect("Failed to get events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "test");
    }

    #[tokio::test]
    async fn test_save_and_load_task() {
        use crate::traits::{TaskComplexity, TaskPriority};

        let store = create_test_store().await;

        let task = Task {
            id: Uuid::new_v4(),
            title: "Test Task".to_string(),
            description: Some("A test task".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("agent_1".to_string()),
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };

        store.save_task(&task).await.expect("Failed to save task");
        let fetched = store.get_task(&task.id).await.expect("Failed to get task");

        assert!(fetched.is_some());
        let fetched_task = fetched.unwrap();
        assert_eq!(fetched_task.title, "Test Task");
        assert_eq!(fetched_task.priority, TaskPriority::High);
        assert_eq!(fetched_task.complexity, TaskComplexity::Moderate);
    }

    #[tokio::test]
    async fn test_search_events() {
        let store = create_test_store().await;

        let event = Event {
            id: Uuid::new_v4(),
            event_type: "search_test".to_string(),
            timestamp: Utc::now().timestamp(),
            session_id: "session_1".to_string(),
            actor_type: ActorType::Agent,
            actor_id: "agent_1".to_string(),
            content: "Searchable content".to_string(),
            metadata: None,
            git_commit: None,
        };

        store
            .save_event(&event)
            .await
            .expect("Failed to save event");
        let results = store
            .search_events("Searchable")
            .await
            .expect("Failed to search");

        assert_eq!(results.len(), 1);
    }
}
