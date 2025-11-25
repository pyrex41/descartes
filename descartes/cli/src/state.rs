/// Simple StateStore implementation for CLI
use async_trait::async_trait;
use descartes_core::{Event, StateStore, StateStoreResult, Task, TaskComplexity, TaskPriority};
use sqlx::sqlite::SqlitePool;
use uuid::Uuid;

pub struct SimpleStateStore {
    pool: SqlitePool,
}

impl SimpleStateStore {
    pub async fn new(database_url: &str) -> StateStoreResult<Self> {
        let pool = SqlitePool::connect(database_url)
            .await
            .map_err(|e| descartes_core::StateStoreError::DatabaseError(e.to_string()))?;

        Ok(Self { pool })
    }

    pub async fn from_config(config: &descartes_core::DescaratesConfig) -> StateStoreResult<Self> {
        let db_path = format!("{}/data/descartes.db", config.storage.base_path);
        let db_url = format!("sqlite://{}", db_path);
        Self::new(&db_url).await
    }
}

#[async_trait]
impl StateStore for SimpleStateStore {
    async fn initialize(&mut self) -> StateStoreResult<()> {
        // Tables are created during init command
        Ok(())
    }

    async fn save_event(&self, event: &Event) -> StateStoreResult<()> {
        let metadata_json = event
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).ok())
            .flatten();

        sqlx::query(
            r#"
            INSERT INTO events (id, event_type, timestamp, session_id, actor_type, actor_id, content, metadata, git_commit)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
            "#,
        )
        .bind(event.id.to_string())
        .bind(&event.event_type)
        .bind(event.timestamp)
        .bind(&event.session_id)
        .bind(format!("{:?}", event.actor_type))
        .bind(&event.actor_id)
        .bind(&event.content)
        .bind(metadata_json)
        .bind(&event.git_commit)
        .execute(&self.pool)
        .await
        .map_err(|e| descartes_core::StateStoreError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_events(&self, session_id: &str) -> StateStoreResult<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT id, event_type, timestamp, session_id, actor_type, actor_id, content, metadata, git_commit FROM events WHERE session_id = ?1 ORDER BY timestamp"
        )
        .bind(session_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| descartes_core::StateStoreError::DatabaseError(e.to_string()))?;

        let events = rows
            .into_iter()
            .map(|row| self.row_to_event(&row))
            .collect::<StateStoreResult<Vec<_>>>()?;

        Ok(events)
    }

    async fn get_events_by_type(&self, event_type: &str) -> StateStoreResult<Vec<Event>> {
        let rows = sqlx::query(
            "SELECT id, event_type, timestamp, session_id, actor_type, actor_id, content, metadata, git_commit FROM events WHERE event_type = ?1 ORDER BY timestamp DESC LIMIT 100"
        )
        .bind(event_type)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| descartes_core::StateStoreError::DatabaseError(e.to_string()))?;

        let events = rows
            .into_iter()
            .map(|row| self.row_to_event(&row))
            .collect::<StateStoreResult<Vec<_>>>()?;

        Ok(events)
    }

    async fn save_task(&self, task: &Task) -> StateStoreResult<()> {
        let metadata_json = task
            .metadata
            .as_ref()
            .map(|m| serde_json::to_string(m).ok())
            .flatten();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO tasks (id, title, description, status, assigned_to, created_at, updated_at, metadata)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            "#,
        )
        .bind(task.id.to_string())
        .bind(&task.title)
        .bind(&task.description)
        .bind(format!("{:?}", task.status))
        .bind(&task.assigned_to)
        .bind(task.created_at)
        .bind(task.updated_at)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| descartes_core::StateStoreError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn get_task(&self, task_id: &Uuid) -> StateStoreResult<Option<Task>> {
        let row = sqlx::query(
            "SELECT id, title, description, status, assigned_to, created_at, updated_at, metadata FROM tasks WHERE id = ?1"
        )
        .bind(task_id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| descartes_core::StateStoreError::DatabaseError(e.to_string()))?;

        if let Some(row) = row {
            Ok(Some(self.row_to_task(&row)?))
        } else {
            Ok(None)
        }
    }

    async fn get_tasks(&self) -> StateStoreResult<Vec<Task>> {
        let rows = sqlx::query(
            "SELECT id, title, description, status, assigned_to, created_at, updated_at, metadata FROM tasks ORDER BY created_at DESC"
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| descartes_core::StateStoreError::DatabaseError(e.to_string()))?;

        let tasks = rows
            .into_iter()
            .map(|row| self.row_to_task(&row))
            .collect::<StateStoreResult<Vec<_>>>()?;

        Ok(tasks)
    }

    async fn search_events(&self, query: &str) -> StateStoreResult<Vec<Event>> {
        let search_pattern = format!("%{}%", query);

        let rows = sqlx::query(
            r#"
            SELECT id, event_type, timestamp, session_id, actor_type, actor_id, content, metadata, git_commit
            FROM events
            WHERE content LIKE ?1 OR event_type LIKE ?1
            ORDER BY timestamp DESC
            LIMIT 100
            "#
        )
        .bind(&search_pattern)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| descartes_core::StateStoreError::DatabaseError(e.to_string()))?;

        let events = rows
            .into_iter()
            .map(|row| self.row_to_event(&row))
            .collect::<StateStoreResult<Vec<_>>>()?;

        Ok(events)
    }
}

impl SimpleStateStore {
    fn row_to_event(&self, row: &sqlx::sqlite::SqliteRow) -> StateStoreResult<Event> {
        use sqlx::Row;

        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| descartes_core::StateStoreError::SerializationError(e.to_string()))?;

        let actor_type_str: String = row.get("actor_type");
        let actor_type = match actor_type_str.as_str() {
            "User" => descartes_core::ActorType::User,
            "Agent" => descartes_core::ActorType::Agent,
            "System" => descartes_core::ActorType::System,
            _ => descartes_core::ActorType::System,
        };

        let metadata_str: Option<String> = row.get("metadata");
        let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

        Ok(Event {
            id,
            event_type: row.get("event_type"),
            timestamp: row.get("timestamp"),
            session_id: row.get("session_id"),
            actor_type,
            actor_id: row.get("actor_id"),
            content: row.get("content"),
            metadata,
            git_commit: row.get("git_commit"),
        })
    }

    fn row_to_task(&self, row: &sqlx::sqlite::SqliteRow) -> StateStoreResult<Task> {
        use sqlx::Row;

        let id_str: String = row.get("id");
        let id = Uuid::parse_str(&id_str)
            .map_err(|e| descartes_core::StateStoreError::SerializationError(e.to_string()))?;

        let status_str: String = row.get("status");
        let status = match status_str.as_str() {
            "Todo" => descartes_core::TaskStatus::Todo,
            "InProgress" => descartes_core::TaskStatus::InProgress,
            "Done" => descartes_core::TaskStatus::Done,
            "Blocked" => descartes_core::TaskStatus::Blocked,
            _ => descartes_core::TaskStatus::Todo,
        };

        let metadata_str: Option<String> = row.get("metadata");
        let metadata = metadata_str.and_then(|s| serde_json::from_str(&s).ok());

        Ok(Task {
            id,
            title: row.get("title"),
            description: row.get("description"),
            status,
            priority: TaskPriority::default(),
            complexity: TaskComplexity::default(),
            assigned_to: row.get("assigned_to"),
            dependencies: vec![],
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
            metadata,
        })
    }
}
