/// Advanced task retrieval and querying functionality
/// Provides flexible querying, filtering, sorting, and dependency resolution for tasks
///
/// This module extends the basic StateStore trait with more sophisticated task retrieval
/// capabilities needed for Kanban views, task lists, and dependency management.
use crate::errors::{StateStoreError, StateStoreResult};
use crate::traits::{Task, TaskComplexity, TaskPriority, TaskStatus};
use sqlx::sqlite::SqlitePool;
use sqlx::Row;
use std::str::FromStr;
use uuid::Uuid;

/// Task query builder for constructing complex queries
#[derive(Debug, Clone)]
pub struct TaskQueryBuilder {
    /// Filter by status
    status_filter: Option<Vec<TaskStatus>>,

    /// Filter by priority
    priority_filter: Option<Vec<TaskPriority>>,

    /// Filter by complexity
    complexity_filter: Option<Vec<TaskComplexity>>,

    /// Filter by assigned_to
    assigned_to_filter: Option<Vec<String>>,

    /// Search term for title/description
    search_term: Option<String>,

    /// Sort field
    sort_by: TaskSortField,

    /// Sort direction
    sort_order: SortOrder,

    /// Pagination offset
    offset: i64,

    /// Pagination limit
    limit: i64,

    /// Include tasks with no assignee
    include_unassigned: bool,
}

/// Fields that can be used for sorting tasks
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSortField {
    CreatedAt,
    UpdatedAt,
    Priority,
    Complexity,
    Title,
    Status,
}

/// Sort order for query results
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortOrder {
    Ascending,
    Descending,
}

impl Default for TaskQueryBuilder {
    fn default() -> Self {
        Self {
            status_filter: None,
            priority_filter: None,
            complexity_filter: None,
            assigned_to_filter: None,
            search_term: None,
            sort_by: TaskSortField::UpdatedAt,
            sort_order: SortOrder::Descending,
            offset: 0,
            limit: 100,
            include_unassigned: true,
        }
    }
}

impl TaskQueryBuilder {
    /// Create a new query builder with default settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Filter by task status
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status_filter = Some(vec![status]);
        self
    }

    /// Filter by multiple task statuses
    pub fn with_statuses(mut self, statuses: Vec<TaskStatus>) -> Self {
        self.status_filter = Some(statuses);
        self
    }

    /// Filter by task priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority_filter = Some(vec![priority]);
        self
    }

    /// Filter by multiple priorities
    pub fn with_priorities(mut self, priorities: Vec<TaskPriority>) -> Self {
        self.priority_filter = Some(priorities);
        self
    }

    /// Filter by task complexity
    pub fn with_complexity(mut self, complexity: TaskComplexity) -> Self {
        self.complexity_filter = Some(vec![complexity]);
        self
    }

    /// Filter by multiple complexities
    pub fn with_complexities(mut self, complexities: Vec<TaskComplexity>) -> Self {
        self.complexity_filter = Some(complexities);
        self
    }

    /// Filter by assignee
    pub fn assigned_to(mut self, assignee: String) -> Self {
        self.assigned_to_filter = Some(vec![assignee]);
        self
    }

    /// Filter by multiple assignees
    pub fn assigned_to_any(mut self, assignees: Vec<String>) -> Self {
        self.assigned_to_filter = Some(assignees);
        self
    }

    /// Filter unassigned tasks only
    pub fn unassigned_only(mut self) -> Self {
        self.assigned_to_filter = Some(vec![]);
        self.include_unassigned = true;
        self
    }

    /// Search in title and description
    pub fn search(mut self, term: String) -> Self {
        self.search_term = Some(term);
        self
    }

    /// Set sort field
    pub fn sort_by(mut self, field: TaskSortField) -> Self {
        self.sort_by = field;
        self
    }

    /// Set sort order
    pub fn order(mut self, order: SortOrder) -> Self {
        self.sort_order = order;
        self
    }

    /// Set pagination offset
    pub fn offset(mut self, offset: i64) -> Self {
        self.offset = offset;
        self
    }

    /// Set pagination limit
    pub fn limit(mut self, limit: i64) -> Self {
        self.limit = limit;
        self
    }

    /// Build the SQL query and parameters
    fn build_query(&self) -> (String, Vec<String>) {
        let mut query = String::from(
            "SELECT id, title, description, status, priority, complexity, assigned_to, dependencies, created_at, updated_at, metadata FROM tasks"
        );

        let mut where_clauses = Vec::new();
        let mut params = Vec::new();

        // Status filter
        if let Some(statuses) = &self.status_filter {
            if !statuses.is_empty() {
                let placeholders: Vec<_> = statuses.iter().map(|_| "?").collect();
                where_clauses.push(format!("status IN ({})", placeholders.join(", ")));
                for status in statuses {
                    params.push(format!("{:?}", status));
                }
            }
        }

        // Priority filter
        if let Some(priorities) = &self.priority_filter {
            if !priorities.is_empty() {
                let placeholders: Vec<_> = priorities.iter().map(|_| "?").collect();
                where_clauses.push(format!("priority IN ({})", placeholders.join(", ")));
                for priority in priorities {
                    params.push(priority.to_string());
                }
            }
        }

        // Complexity filter
        if let Some(complexities) = &self.complexity_filter {
            if !complexities.is_empty() {
                let placeholders: Vec<_> = complexities.iter().map(|_| "?").collect();
                where_clauses.push(format!("complexity IN ({})", placeholders.join(", ")));
                for complexity in complexities {
                    params.push(complexity.to_string());
                }
            }
        }

        // Assigned_to filter
        if let Some(assignees) = &self.assigned_to_filter {
            if assignees.is_empty() && self.include_unassigned {
                where_clauses.push("assigned_to IS NULL".to_string());
            } else if !assignees.is_empty() {
                let placeholders: Vec<_> = assignees.iter().map(|_| "?").collect();
                let clause = if self.include_unassigned {
                    format!(
                        "(assigned_to IN ({}) OR assigned_to IS NULL)",
                        placeholders.join(", ")
                    )
                } else {
                    format!("assigned_to IN ({})", placeholders.join(", "))
                };
                where_clauses.push(clause);
                for assignee in assignees {
                    params.push(assignee.clone());
                }
            }
        }

        // Search term
        if let Some(term) = &self.search_term {
            where_clauses.push("(title LIKE ? OR description LIKE ?)".to_string());
            let search_pattern = format!("%{}%", term);
            params.push(search_pattern.clone());
            params.push(search_pattern);
        }

        // Add WHERE clause if needed
        if !where_clauses.is_empty() {
            query.push_str(" WHERE ");
            query.push_str(&where_clauses.join(" AND "));
        }

        // Add ORDER BY clause
        let order_str = match self.sort_order {
            SortOrder::Ascending => "ASC",
            SortOrder::Descending => "DESC",
        };

        let sort_column = match self.sort_by {
            TaskSortField::CreatedAt => "created_at",
            TaskSortField::UpdatedAt => "updated_at",
            TaskSortField::Priority => "priority",
            TaskSortField::Complexity => "complexity",
            TaskSortField::Title => "title",
            TaskSortField::Status => "status",
        };

        query.push_str(&format!(" ORDER BY {} {}", sort_column, order_str));

        // Add pagination
        query.push_str(&format!(" LIMIT {} OFFSET {}", self.limit, self.offset));

        (query, params)
    }

    /// Execute the query against the database
    pub async fn execute(&self, pool: &SqlitePool) -> StateStoreResult<Vec<Task>> {
        let (query_str, params) = self.build_query();

        // Build the query dynamically with parameters
        let mut query = sqlx::query(&query_str);
        for param in params {
            query = query.bind(param);
        }

        let rows = query.fetch_all(pool).await.map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to execute task query: {}", e))
        })?;

        let tasks = rows
            .iter()
            .map(parse_task_row)
            .collect::<StateStoreResult<Vec<Task>>>()?;

        Ok(tasks)
    }
}

/// Parse a database row into a Task
fn parse_task_row(row: &sqlx::sqlite::SqliteRow) -> StateStoreResult<Task> {
    let status_str: String = row.get("status");
    let status = match status_str.as_str() {
        "Todo" => TaskStatus::Todo,
        "InProgress" => TaskStatus::InProgress,
        "Done" => TaskStatus::Done,
        "Blocked" => TaskStatus::Blocked,
        _ => TaskStatus::Todo,
    };

    let priority_str: String = row.get("priority");
    let priority = TaskPriority::from_str(&priority_str).unwrap_or_default();

    let complexity_str: String = row.get("complexity");
    let complexity = TaskComplexity::from_str(&complexity_str).unwrap_or_default();

    let dependencies_str: String = row.get("dependencies");
    let dependencies: Vec<Uuid> = serde_json::from_str(&dependencies_str).unwrap_or_default();

    let metadata_str: Option<String> = row.get("metadata");
    let metadata = metadata_str.and_then(|m| serde_json::from_str(&m).ok());

    Ok(Task {
        id: Uuid::parse_str(&row.get::<String, _>("id"))
            .map_err(|e| StateStoreError::DatabaseError(format!("Invalid UUID: {}", e)))?,
        title: row.get("title"),
        description: row.get("description"),
        status,
        priority,
        complexity,
        assigned_to: row.get("assigned_to"),
        dependencies,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        metadata,
    })
}

/// Task retrieval and query functions
pub struct TaskQueries {
    pool: SqlitePool,
}

impl TaskQueries {
    /// Create a new TaskQueries instance
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get a task by ID
    pub async fn get_task_by_id(&self, id: &Uuid) -> StateStoreResult<Option<Task>> {
        let row = sqlx::query(
            "SELECT id, title, description, status, priority, complexity, assigned_to, dependencies, created_at, updated_at, metadata FROM tasks WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Failed to fetch task: {}", e)))?;

        row.map(|r| parse_task_row(&r)).transpose()
    }

    /// Get all tasks
    pub async fn get_all_tasks(&self) -> StateStoreResult<Vec<Task>> {
        TaskQueryBuilder::new()
            .limit(1000)
            .execute(&self.pool)
            .await
    }

    /// Get tasks by status
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> StateStoreResult<Vec<Task>> {
        TaskQueryBuilder::new()
            .with_status(status)
            .execute(&self.pool)
            .await
    }

    /// Get tasks by priority
    pub async fn get_tasks_by_priority(
        &self,
        priority: TaskPriority,
    ) -> StateStoreResult<Vec<Task>> {
        TaskQueryBuilder::new()
            .with_priority(priority)
            .execute(&self.pool)
            .await
    }

    /// Get tasks by complexity
    pub async fn get_tasks_by_complexity(
        &self,
        complexity: TaskComplexity,
    ) -> StateStoreResult<Vec<Task>> {
        TaskQueryBuilder::new()
            .with_complexity(complexity)
            .execute(&self.pool)
            .await
    }

    /// Get tasks assigned to a specific user/agent
    pub async fn get_tasks_by_assignee(&self, assignee: &str) -> StateStoreResult<Vec<Task>> {
        TaskQueryBuilder::new()
            .assigned_to(assignee.to_string())
            .execute(&self.pool)
            .await
    }

    /// Get unassigned tasks
    pub async fn get_unassigned_tasks(&self) -> StateStoreResult<Vec<Task>> {
        TaskQueryBuilder::new()
            .unassigned_only()
            .execute(&self.pool)
            .await
    }

    /// Get tasks that a specific task depends on (prerequisites)
    pub async fn get_task_dependencies(&self, task_id: &Uuid) -> StateStoreResult<Vec<Task>> {
        // First get the task to retrieve its dependencies list
        let task = self.get_task_by_id(task_id).await?;

        match task {
            Some(t) if !t.dependencies.is_empty() => {
                // Build a query to fetch all dependency tasks
                let placeholders: Vec<_> = t.dependencies.iter().map(|_| "?").collect();
                let query_str = format!(
                    "SELECT id, title, description, status, priority, complexity, assigned_to, dependencies, created_at, updated_at, metadata FROM tasks WHERE id IN ({})",
                    placeholders.join(", ")
                );

                let mut query = sqlx::query(&query_str);
                for dep_id in &t.dependencies {
                    query = query.bind(dep_id.to_string());
                }

                let rows = query.fetch_all(&self.pool).await.map_err(|e| {
                    StateStoreError::DatabaseError(format!("Failed to fetch dependencies: {}", e))
                })?;

                rows.iter().map(parse_task_row).collect()
            }
            _ => Ok(vec![]),
        }
    }

    /// Get tasks that depend on a specific task (dependents)
    pub async fn get_dependent_tasks(&self, task_id: &Uuid) -> StateStoreResult<Vec<Task>> {
        // Query the task_dependencies table to find tasks that depend on this one
        let rows = sqlx::query(
            r#"
            SELECT t.id, t.title, t.description, t.status, t.priority, t.complexity,
                   t.assigned_to, t.dependencies, t.created_at, t.updated_at, t.metadata
            FROM tasks t
            INNER JOIN task_dependencies td ON t.id = td.task_id
            WHERE td.depends_on_task_id = ?
            ORDER BY t.updated_at DESC
            "#,
        )
        .bind(task_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            StateStoreError::DatabaseError(format!("Failed to fetch dependent tasks: {}", e))
        })?;

        rows.iter().map(parse_task_row).collect()
    }

    /// Check if a task is blocked (has unfinished dependencies)
    pub async fn check_if_task_is_blocked(&self, task_id: &Uuid) -> StateStoreResult<bool> {
        let dependencies = self.get_task_dependencies(task_id).await?;

        // A task is blocked if any of its dependencies are not done
        Ok(dependencies
            .iter()
            .any(|dep| dep.status != TaskStatus::Done))
    }

    /// Get all tasks that are currently blocked
    pub async fn get_blocked_tasks(&self) -> StateStoreResult<Vec<Task>> {
        // Get all tasks with dependencies
        let all_tasks = self.get_all_tasks().await?;

        let mut blocked_tasks = Vec::new();

        for task in all_tasks {
            if !task.dependencies.is_empty() {
                let is_blocked = self.check_if_task_is_blocked(&task.id).await?;
                if is_blocked {
                    blocked_tasks.push(task);
                }
            }
        }

        Ok(blocked_tasks)
    }

    /// Get tasks ready to be worked on (status=Todo, no blocking dependencies)
    pub async fn get_ready_tasks(&self) -> StateStoreResult<Vec<Task>> {
        let todo_tasks = self.get_tasks_by_status(TaskStatus::Todo).await?;

        let mut ready_tasks = Vec::new();

        for task in todo_tasks {
            if task.dependencies.is_empty() {
                ready_tasks.push(task);
            } else {
                let is_blocked = self.check_if_task_is_blocked(&task.id).await?;
                if !is_blocked {
                    ready_tasks.push(task);
                }
            }
        }

        Ok(ready_tasks)
    }

    /// Get tasks for Kanban board view (organized by status)
    pub async fn get_kanban_tasks(&self) -> StateStoreResult<KanbanBoard> {
        Ok(KanbanBoard {
            todo: self.get_tasks_by_status(TaskStatus::Todo).await?,
            in_progress: self.get_tasks_by_status(TaskStatus::InProgress).await?,
            done: self.get_tasks_by_status(TaskStatus::Done).await?,
            blocked: self.get_tasks_by_status(TaskStatus::Blocked).await?,
        })
    }

    /// Get task statistics
    pub async fn get_task_statistics(&self) -> StateStoreResult<TaskStatistics> {
        let total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| StateStoreError::DatabaseError(format!("Failed to count tasks: {}", e)))?;

        let todo = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks WHERE status = 'Todo'")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to count todo tasks: {}", e))
            })?;

        let in_progress =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks WHERE status = 'InProgress'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    StateStoreError::DatabaseError(format!(
                        "Failed to count in progress tasks: {}",
                        e
                    ))
                })?;

        let done = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks WHERE status = 'Done'")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| {
                StateStoreError::DatabaseError(format!("Failed to count done tasks: {}", e))
            })?;

        let blocked =
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM tasks WHERE status = 'Blocked'")
                .fetch_one(&self.pool)
                .await
                .map_err(|e| {
                    StateStoreError::DatabaseError(format!("Failed to count blocked tasks: {}", e))
                })?;

        Ok(TaskStatistics {
            total: total as usize,
            todo: todo as usize,
            in_progress: in_progress as usize,
            done: done as usize,
            blocked: blocked as usize,
        })
    }

    /// Create a query builder
    pub fn query(&self) -> TaskQueryBuilder {
        TaskQueryBuilder::new()
    }

    /// Get connection pool for advanced operations
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

/// Kanban board representation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct KanbanBoard {
    pub todo: Vec<Task>,
    pub in_progress: Vec<Task>,
    pub done: Vec<Task>,
    pub blocked: Vec<Task>,
}

/// Task statistics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TaskStatistics {
    pub total: usize,
    pub todo: usize,
    pub in_progress: usize,
    pub done: usize,
    pub blocked: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_store::SqliteStateStore;
    use crate::traits::StateStore;
    use chrono::Utc;

    async fn setup_test_db() -> (SqliteStateStore, TaskQueries) {
        let db_path = format!(
            "/tmp/test_task_queries_{}.db",
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let mut store = SqliteStateStore::new(&db_path, false)
            .await
            .expect("Failed to create state store");
        store
            .initialize()
            .await
            .expect("Failed to initialize store");

        let queries = TaskQueries::new(store.pool().clone());
        (store, queries)
    }

    #[tokio::test]
    async fn test_get_task_by_id() {
        let (store, queries) = setup_test_db().await;

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

        store.save_task(&task).await.expect("Failed to save task");

        let fetched = queries
            .get_task_by_id(&task.id)
            .await
            .expect("Failed to get task");
        assert!(fetched.is_some());
        assert_eq!(fetched.unwrap().title, "Test Task");
    }

    #[tokio::test]
    async fn test_get_tasks_by_status() {
        let (store, queries) = setup_test_db().await;

        // Create tasks with different statuses
        for i in 0..5 {
            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Task {}", i),
                description: None,
                status: if i < 3 {
                    TaskStatus::Todo
                } else {
                    TaskStatus::InProgress
                },
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Moderate,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            };
            store.save_task(&task).await.expect("Failed to save task");
        }

        let todo_tasks = queries
            .get_tasks_by_status(TaskStatus::Todo)
            .await
            .expect("Failed to get tasks");
        assert_eq!(todo_tasks.len(), 3);

        let in_progress_tasks = queries
            .get_tasks_by_status(TaskStatus::InProgress)
            .await
            .expect("Failed to get tasks");
        assert_eq!(in_progress_tasks.len(), 2);
    }

    #[tokio::test]
    async fn test_get_tasks_by_priority() {
        let (store, queries) = setup_test_db().await;

        // Create tasks with different priorities
        for priority in &[
            TaskPriority::Low,
            TaskPriority::Medium,
            TaskPriority::High,
            TaskPriority::Critical,
        ] {
            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Task {:?}", priority),
                description: None,
                status: TaskStatus::Todo,
                priority: *priority,
                complexity: TaskComplexity::Moderate,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            };
            store.save_task(&task).await.expect("Failed to save task");
        }

        let high_priority_tasks = queries
            .get_tasks_by_priority(TaskPriority::High)
            .await
            .expect("Failed to get tasks");
        assert_eq!(high_priority_tasks.len(), 1);
    }

    #[tokio::test]
    async fn test_task_dependencies() {
        let (store, queries) = setup_test_db().await;

        // Create prerequisite task
        let prereq_task = Task {
            id: Uuid::new_v4(),
            title: "Prerequisite Task".to_string(),
            description: None,
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };
        store
            .save_task(&prereq_task)
            .await
            .expect("Failed to save task");

        // Create dependent task
        let dependent_task = Task {
            id: Uuid::new_v4(),
            title: "Dependent Task".to_string(),
            description: None,
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Moderate,
            assigned_to: None,
            dependencies: vec![prereq_task.id],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };
        store
            .save_task(&dependent_task)
            .await
            .expect("Failed to save task");

        // Test get_task_dependencies
        let deps = queries
            .get_task_dependencies(&dependent_task.id)
            .await
            .expect("Failed to get dependencies");
        assert_eq!(deps.len(), 1);
        assert_eq!(deps[0].id, prereq_task.id);

        // Test get_dependent_tasks
        let dependents = queries
            .get_dependent_tasks(&prereq_task.id)
            .await
            .expect("Failed to get dependents");
        assert_eq!(dependents.len(), 1);
        assert_eq!(dependents[0].id, dependent_task.id);

        // Test check_if_task_is_blocked
        let is_blocked = queries
            .check_if_task_is_blocked(&dependent_task.id)
            .await
            .expect("Failed to check if blocked");
        assert!(is_blocked); // Should be blocked because prereq is not done
    }

    #[tokio::test]
    async fn test_query_builder() {
        let (store, queries) = setup_test_db().await;

        // Create diverse set of tasks
        let tasks_data = vec![
            (TaskStatus::Todo, TaskPriority::High, "Alice"),
            (TaskStatus::InProgress, TaskPriority::Medium, "Bob"),
            (TaskStatus::Done, TaskPriority::Low, "Alice"),
            (TaskStatus::Todo, TaskPriority::Critical, "Charlie"),
        ];

        for (i, (status, priority, assignee)) in tasks_data.iter().enumerate() {
            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Task {}", i),
                description: Some(format!("Description {}", i)),
                status: status.clone(),
                priority: *priority,
                complexity: TaskComplexity::Moderate,
                assigned_to: Some(assignee.to_string()),
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            };
            store.save_task(&task).await.expect("Failed to save task");
        }

        // Test query builder with multiple filters
        let results = queries
            .query()
            .with_status(TaskStatus::Todo)
            .with_priority(TaskPriority::High)
            .assigned_to("Alice".to_string())
            .execute(queries.pool())
            .await
            .expect("Failed to execute query");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, TaskStatus::Todo);
        assert_eq!(results[0].priority, TaskPriority::High);
    }

    #[tokio::test]
    async fn test_search_tasks() {
        let (store, queries) = setup_test_db().await;

        let task = Task {
            id: Uuid::new_v4(),
            title: "Implement authentication".to_string(),
            description: Some("Add JWT authentication to API".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Complex,
            assigned_to: None,
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        };
        store.save_task(&task).await.expect("Failed to save task");

        let results = queries
            .query()
            .search("authentication".to_string())
            .execute(queries.pool())
            .await
            .expect("Failed to search tasks");

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Implement authentication");
    }

    #[tokio::test]
    async fn test_pagination() {
        let (store, queries) = setup_test_db().await;

        // Create 10 tasks
        for i in 0..10 {
            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Task {}", i),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Moderate,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp() + i,
                updated_at: Utc::now().timestamp() + i,
                metadata: None,
            };
            store.save_task(&task).await.expect("Failed to save task");
        }

        // Get first page
        let page1 = queries
            .query()
            .limit(5)
            .offset(0)
            .execute(queries.pool())
            .await
            .expect("Failed to get page 1");
        assert_eq!(page1.len(), 5);

        // Get second page
        let page2 = queries
            .query()
            .limit(5)
            .offset(5)
            .execute(queries.pool())
            .await
            .expect("Failed to get page 2");
        assert_eq!(page2.len(), 5);
    }

    #[tokio::test]
    async fn test_kanban_board() {
        let (store, queries) = setup_test_db().await;

        // Create tasks for each status
        for status in &[
            TaskStatus::Todo,
            TaskStatus::InProgress,
            TaskStatus::Done,
            TaskStatus::Blocked,
        ] {
            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Task {:?}", status),
                description: None,
                status: status.clone(),
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Moderate,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            };
            store.save_task(&task).await.expect("Failed to save task");
        }

        let kanban = queries
            .get_kanban_tasks()
            .await
            .expect("Failed to get kanban board");
        assert_eq!(kanban.todo.len(), 1);
        assert_eq!(kanban.in_progress.len(), 1);
        assert_eq!(kanban.done.len(), 1);
        assert_eq!(kanban.blocked.len(), 1);
    }

    #[tokio::test]
    async fn test_task_statistics() {
        let (store, queries) = setup_test_db().await;

        // Create tasks
        for i in 0..10 {
            let status = match i % 4 {
                0 => TaskStatus::Todo,
                1 => TaskStatus::InProgress,
                2 => TaskStatus::Done,
                _ => TaskStatus::Blocked,
            };

            let task = Task {
                id: Uuid::new_v4(),
                title: format!("Task {}", i),
                description: None,
                status,
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Moderate,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            };
            store.save_task(&task).await.expect("Failed to save task");
        }

        let stats = queries
            .get_task_statistics()
            .await
            .expect("Failed to get statistics");
        assert_eq!(stats.total, 10);
        assert!(stats.todo > 0);
        assert!(stats.in_progress > 0);
        assert!(stats.done > 0);
    }
}
