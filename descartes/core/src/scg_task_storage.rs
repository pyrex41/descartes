/// SCG-based task storage for Descartes
/// Wraps SCUD's file-based Storage with async operations and in-memory querying
///
/// This module provides an alternative to SQLite task storage that uses SCUD's
/// SCG (SCUD Graph) format for human-readable, git-friendly task files.
use crate::errors::{StateStoreError, StateStoreResult};
use crate::traits::{
    scud_to_task, task_to_scud, ScudPhase, ScudStorage,
    Task, TaskComplexity, TaskPriority, TaskStatus,
};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// SCG-based task storage that wraps SCUD's Storage
/// Provides async access and in-memory caching for efficient queries
pub struct ScgTaskStorage {
    /// Project root path (used to create new Storage instances for blocking ops)
    project_root: PathBuf,

    /// In-memory cache of phases (tag -> Phase)
    phase_cache: RwLock<HashMap<String, ScudPhase>>,

    /// Active phase tag cache
    active_phase: RwLock<Option<String>>,
}

impl ScgTaskStorage {
    /// Create a new SCG task storage at the given project root
    pub fn new<P: AsRef<Path>>(project_root: P) -> Self {
        let project_root = project_root.as_ref().to_path_buf();

        Self {
            project_root,
            phase_cache: RwLock::new(HashMap::new()),
            active_phase: RwLock::new(None),
        }
    }

    /// Initialize the storage directory structure
    /// Creates .taskmaster/ directory with necessary files
    pub async fn initialize(&self) -> StateStoreResult<()> {
        // Run synchronous SCUD storage init in blocking task
        let project_root = self.project_root.clone();
        tokio::task::spawn_blocking(move || {
            let storage = ScudStorage::new(Some(project_root));
            storage.initialize()
        })
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Join error: {}", e)))?
        .map_err(|e| StateStoreError::DatabaseError(format!("SCUD init error: {}", e)))?;

        // Load initial state into cache
        self.refresh_cache().await?;

        Ok(())
    }

    /// Refresh the in-memory cache from disk
    pub async fn refresh_cache(&self) -> StateStoreResult<()> {
        let project_root = self.project_root.clone();

        // Load phases from disk
        let phases = tokio::task::spawn_blocking(move || {
            let storage = ScudStorage::new(Some(project_root));
            storage.load_tasks()
        })
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Join error: {}", e)))?
        .map_err(|e| StateStoreError::DatabaseError(format!("Load error: {}", e)))?;

        // Update phase cache
        {
            let mut cache = self.phase_cache.write().await;
            *cache = phases;
        }

        // Load active phase (SCUD calls it "group")
        let project_root = self.project_root.clone();
        let active = tokio::task::spawn_blocking(move || {
            let storage = ScudStorage::new(Some(project_root));
            storage.get_active_group()
        })
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Join error: {}", e)))?
        .map_err(|e| StateStoreError::DatabaseError(format!("Load active error: {}", e)))?;

        {
            let mut active_cache = self.active_phase.write().await;
            *active_cache = active;
        }

        Ok(())
    }

    /// Get all phases
    pub async fn get_phases(&self) -> StateStoreResult<HashMap<String, ScudPhase>> {
        let cache = self.phase_cache.read().await;
        Ok(cache.clone())
    }

    /// Get a specific phase by tag
    pub async fn get_phase(&self, tag: &str) -> StateStoreResult<Option<ScudPhase>> {
        let cache = self.phase_cache.read().await;
        Ok(cache.get(tag).cloned())
    }

    /// Get the active phase tag
    pub async fn get_active_phase_tag(&self) -> StateStoreResult<Option<String>> {
        let cache = self.active_phase.read().await;
        Ok(cache.clone())
    }

    /// Get the active phase
    pub async fn get_active_phase(&self) -> StateStoreResult<Option<ScudPhase>> {
        let active_tag = self.get_active_phase_tag().await?;
        match active_tag {
            Some(tag) => self.get_phase(&tag).await,
            None => Ok(None),
        }
    }

    /// Set the active phase (SCUD calls it "group")
    pub async fn set_active_phase(&self, tag: &str) -> StateStoreResult<()> {
        let project_root = self.project_root.clone();
        let tag_owned = tag.to_string();

        tokio::task::spawn_blocking(move || {
            let storage = ScudStorage::new(Some(project_root));
            storage.set_active_group(&tag_owned)
        })
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Join error: {}", e)))?
        .map_err(|e| StateStoreError::DatabaseError(format!("Set active error: {}", e)))?;

        // Update cache
        {
            let mut active_cache = self.active_phase.write().await;
            *active_cache = Some(tag.to_string());
        }

        Ok(())
    }

    /// Save a phase (creates or updates) - uses SCUD's update_group
    pub async fn save_phase(&self, phase: &ScudPhase) -> StateStoreResult<()> {
        let project_root = self.project_root.clone();
        let phase_clone = phase.clone();
        let phase_name = phase.name.clone();

        tokio::task::spawn_blocking(move || {
            let storage = ScudStorage::new(Some(project_root));
            storage.update_group(&phase_name, &phase_clone)
        })
        .await
        .map_err(|e| StateStoreError::DatabaseError(format!("Join error: {}", e)))?
        .map_err(|e| StateStoreError::DatabaseError(format!("Save phase error: {}", e)))?;

        // Update cache
        {
            let mut cache = self.phase_cache.write().await;
            cache.insert(phase.name.clone(), phase.clone());
        }

        Ok(())
    }

    /// Get all tasks across all phases as Descartes Task objects
    pub async fn get_all_tasks(&self) -> StateStoreResult<Vec<Task>> {
        let cache = self.phase_cache.read().await;
        let mut tasks = Vec::new();

        for phase in cache.values() {
            for scud_task in &phase.tasks {
                match scud_to_task(scud_task) {
                    Ok(task) => tasks.push(task),
                    Err(e) => {
                        // Log but continue - some tasks may have non-UUID IDs
                        tracing::warn!(
                            "Failed to convert SCUD task {} to Descartes task: {}",
                            scud_task.id,
                            e
                        );
                    }
                }
            }
        }

        Ok(tasks)
    }

    /// Get all tasks from the active phase
    pub async fn get_active_phase_tasks(&self) -> StateStoreResult<Vec<Task>> {
        match self.get_active_phase().await? {
            Some(phase) => {
                let mut tasks = Vec::new();
                for scud_task in &phase.tasks {
                    match scud_to_task(scud_task) {
                        Ok(task) => tasks.push(task),
                        Err(e) => {
                            tracing::warn!(
                                "Failed to convert SCUD task {} to Descartes task: {}",
                                scud_task.id,
                                e
                            );
                        }
                    }
                }
                Ok(tasks)
            }
            None => Ok(Vec::new()),
        }
    }

    /// Get a task by ID from the active phase
    pub async fn get_task(&self, task_id: &Uuid) -> StateStoreResult<Option<Task>> {
        let id_str = task_id.to_string();
        match self.get_active_phase().await? {
            Some(phase) => {
                for scud_task in &phase.tasks {
                    if scud_task.id == id_str {
                        return scud_to_task(scud_task)
                            .map(Some)
                            .map_err(|e| StateStoreError::DatabaseError(format!("UUID parse error: {}", e)));
                    }
                }
                Ok(None)
            }
            None => Ok(None),
        }
    }

    /// Save a task to the active phase
    pub async fn save_task(&self, task: &Task) -> StateStoreResult<()> {
        let active_tag = self.get_active_phase_tag().await?
            .ok_or_else(|| StateStoreError::NotFound("No active phase set".to_string()))?;

        let mut phase = self.get_phase(&active_tag).await?
            .ok_or_else(|| StateStoreError::NotFound(format!("Phase {} not found", active_tag)))?;

        let scud_task = task_to_scud(task);
        let task_id = scud_task.id.clone();

        // Find and update existing task, or add new one
        let mut found = false;
        for existing in &mut phase.tasks {
            if existing.id == task_id {
                *existing = scud_task.clone();
                found = true;
                break;
            }
        }

        if !found {
            phase.tasks.push(scud_task);
        }

        self.save_phase(&phase).await
    }

    /// Get the next available task from the active phase
    /// Returns a task that is Pending and has all dependencies met
    pub async fn get_next_task(&self) -> StateStoreResult<Option<Task>> {
        match self.get_active_phase().await? {
            Some(phase) => {
                if let Some(scud_task) = phase.find_next_task() {
                    scud_to_task(scud_task)
                        .map(Some)
                        .map_err(|e| StateStoreError::DatabaseError(format!("UUID parse error: {}", e)))
                } else {
                    Ok(None)
                }
            }
            None => Ok(None),
        }
    }

    /// Get phase statistics
    pub async fn get_phase_stats(&self, tag: &str) -> StateStoreResult<Option<ScgPhaseStats>> {
        match self.get_phase(tag).await? {
            Some(phase) => {
                let stats = phase.get_stats();
                Ok(Some(ScgPhaseStats {
                    name: phase.name.clone(),
                    total: stats.total,
                    pending: stats.pending,
                    in_progress: stats.in_progress,
                    done: stats.done,
                    blocked: stats.blocked,
                    total_complexity: stats.total_complexity,
                }))
            }
            None => Ok(None),
        }
    }

    /// Get project root path
    pub fn project_root(&self) -> &Path {
        &self.project_root
    }
}

/// Phase statistics from SCUD
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScgPhaseStats {
    pub name: String,
    pub total: usize,
    pub pending: usize,
    pub in_progress: usize,
    pub done: usize,
    pub blocked: usize,
    pub total_complexity: u32,
}

/// Query builder for in-memory SCG task queries
/// Provides similar interface to TaskQueryBuilder but works with in-memory data
#[derive(Debug, Clone, Default)]
pub struct ScgTaskQueryBuilder {
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
    sort_by: ScgSortField,

    /// Sort direction
    sort_order: ScgSortOrder,

    /// Pagination offset
    offset: usize,

    /// Pagination limit
    limit: usize,

    /// Include unassigned tasks
    include_unassigned: bool,
}

/// Fields for sorting
#[derive(Debug, Clone, Copy, Default)]
pub enum ScgSortField {
    #[default]
    UpdatedAt,
    CreatedAt,
    Priority,
    Complexity,
    Title,
    Status,
}

/// Sort order
#[derive(Debug, Clone, Copy, Default)]
pub enum ScgSortOrder {
    Ascending,
    #[default]
    Descending,
}

impl ScgTaskQueryBuilder {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            limit: 100,
            include_unassigned: true,
            ..Default::default()
        }
    }

    /// Filter by task status
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status_filter = Some(vec![status]);
        self
    }

    /// Filter by multiple statuses
    pub fn with_statuses(mut self, statuses: Vec<TaskStatus>) -> Self {
        self.status_filter = Some(statuses);
        self
    }

    /// Filter by priority
    pub fn with_priority(mut self, priority: TaskPriority) -> Self {
        self.priority_filter = Some(vec![priority]);
        self
    }

    /// Filter by multiple priorities
    pub fn with_priorities(mut self, priorities: Vec<TaskPriority>) -> Self {
        self.priority_filter = Some(priorities);
        self
    }

    /// Filter by complexity
    pub fn with_complexity(mut self, complexity: TaskComplexity) -> Self {
        self.complexity_filter = Some(vec![complexity]);
        self
    }

    /// Filter by assignee
    pub fn assigned_to(mut self, assignee: String) -> Self {
        self.assigned_to_filter = Some(vec![assignee]);
        self
    }

    /// Filter unassigned only
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
    pub fn sort_by(mut self, field: ScgSortField) -> Self {
        self.sort_by = field;
        self
    }

    /// Set sort order
    pub fn order(mut self, order: ScgSortOrder) -> Self {
        self.sort_order = order;
        self
    }

    /// Set pagination offset
    pub fn offset(mut self, offset: usize) -> Self {
        self.offset = offset;
        self
    }

    /// Set pagination limit
    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Execute the query against in-memory task list
    pub fn execute(&self, tasks: &[Task]) -> Vec<Task> {
        let mut result: Vec<&Task> = tasks
            .iter()
            .filter(|t| self.matches(t))
            .collect();

        // Sort
        result.sort_by(|a, b| {
            let cmp = match self.sort_by {
                ScgSortField::UpdatedAt => a.updated_at.cmp(&b.updated_at),
                ScgSortField::CreatedAt => a.created_at.cmp(&b.created_at),
                ScgSortField::Priority => {
                    let pa: u8 = (&a.priority).into();
                    let pb: u8 = (&b.priority).into();
                    pa.cmp(&pb)
                }
                ScgSortField::Complexity => {
                    let ca: u32 = a.complexity.into();
                    let cb: u32 = b.complexity.into();
                    ca.cmp(&cb)
                }
                ScgSortField::Title => a.title.cmp(&b.title),
                ScgSortField::Status => format!("{:?}", a.status).cmp(&format!("{:?}", b.status)),
            };

            match self.sort_order {
                ScgSortOrder::Ascending => cmp,
                ScgSortOrder::Descending => cmp.reverse(),
            }
        });

        // Paginate
        result
            .into_iter()
            .skip(self.offset)
            .take(self.limit)
            .cloned()
            .collect()
    }

    /// Check if a task matches the query filters
    fn matches(&self, task: &Task) -> bool {
        // Status filter
        if let Some(statuses) = &self.status_filter {
            if !statuses.contains(&task.status) {
                return false;
            }
        }

        // Priority filter
        if let Some(priorities) = &self.priority_filter {
            if !priorities.contains(&task.priority) {
                return false;
            }
        }

        // Complexity filter
        if let Some(complexities) = &self.complexity_filter {
            if !complexities.contains(&task.complexity) {
                return false;
            }
        }

        // Assignee filter
        if let Some(assignees) = &self.assigned_to_filter {
            if assignees.is_empty() && self.include_unassigned {
                // Only show unassigned
                if task.assigned_to.is_some() {
                    return false;
                }
            } else if !assignees.is_empty() {
                match &task.assigned_to {
                    Some(assigned) => {
                        if !assignees.contains(assigned) && !(self.include_unassigned && task.assigned_to.is_none()) {
                            return false;
                        }
                    }
                    None => {
                        if !self.include_unassigned {
                            return false;
                        }
                    }
                }
            }
        }

        // Search filter
        if let Some(term) = &self.search_term {
            let term_lower = term.to_lowercase();
            let in_title = task.title.to_lowercase().contains(&term_lower);
            let in_desc = task
                .description
                .as_ref()
                .map(|d| d.to_lowercase().contains(&term_lower))
                .unwrap_or(false);

            if !in_title && !in_desc {
                return false;
            }
        }

        true
    }
}

/// Helper trait to convert TaskPriority to numeric value for sorting
impl From<&TaskPriority> for u8 {
    fn from(priority: &TaskPriority) -> Self {
        match priority {
            TaskPriority::Low => 0,
            TaskPriority::Medium => 1,
            TaskPriority::High => 2,
            TaskPriority::Critical => 3,
        }
    }
}

/// SCG-based task queries that work with ScgTaskStorage
pub struct ScgTaskQueries {
    storage: Arc<ScgTaskStorage>,
}

impl ScgTaskQueries {
    /// Create a new ScgTaskQueries instance
    pub fn new(storage: Arc<ScgTaskStorage>) -> Self {
        Self { storage }
    }

    /// Get a task by ID
    pub async fn get_task_by_id(&self, id: &Uuid) -> StateStoreResult<Option<Task>> {
        self.storage.get_task(id).await
    }

    /// Get all tasks from active phase
    pub async fn get_all_tasks(&self) -> StateStoreResult<Vec<Task>> {
        self.storage.get_active_phase_tasks().await
    }

    /// Get tasks by status
    pub async fn get_tasks_by_status(&self, status: TaskStatus) -> StateStoreResult<Vec<Task>> {
        let tasks = self.get_all_tasks().await?;
        Ok(ScgTaskQueryBuilder::new()
            .with_status(status)
            .limit(1000)
            .execute(&tasks))
    }

    /// Get tasks by priority
    pub async fn get_tasks_by_priority(&self, priority: TaskPriority) -> StateStoreResult<Vec<Task>> {
        let tasks = self.get_all_tasks().await?;
        Ok(ScgTaskQueryBuilder::new()
            .with_priority(priority)
            .limit(1000)
            .execute(&tasks))
    }

    /// Get ready tasks (Pending with all dependencies met)
    pub async fn get_ready_tasks(&self) -> StateStoreResult<Vec<Task>> {
        match self.storage.get_next_task().await? {
            Some(task) => Ok(vec![task]),
            None => Ok(Vec::new()),
        }
    }

    /// Get task statistics for active phase
    pub async fn get_task_statistics(&self) -> StateStoreResult<ScgPhaseStats> {
        let active_tag = self.storage.get_active_phase_tag().await?
            .ok_or_else(|| StateStoreError::NotFound("No active phase".to_string()))?;

        self.storage.get_phase_stats(&active_tag).await?
            .ok_or_else(|| StateStoreError::NotFound("Phase not found".to_string()))
    }

    /// Create a query builder
    pub fn query(&self) -> ScgTaskQueryBuilder {
        ScgTaskQueryBuilder::new()
    }

    /// Execute a query against active phase tasks
    pub async fn execute_query(&self, query: &ScgTaskQueryBuilder) -> StateStoreResult<Vec<Task>> {
        let tasks = self.get_all_tasks().await?;
        Ok(query.execute(&tasks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn create_test_task(title: &str, status: TaskStatus, priority: TaskPriority) -> Task {
        Task {
            id: Uuid::new_v4(),
            title: title.to_string(),
            description: Some(format!("Description for {}", title)),
            status,
            priority,
            complexity: TaskComplexity::Moderate,
            assigned_to: None,
            dependencies: vec![],
            created_at: Utc::now().timestamp(),
            updated_at: Utc::now().timestamp(),
            metadata: None,
        }
    }

    #[test]
    fn test_query_builder_status_filter() {
        let tasks = vec![
            create_test_task("Task 1", TaskStatus::Todo, TaskPriority::Medium),
            create_test_task("Task 2", TaskStatus::InProgress, TaskPriority::High),
            create_test_task("Task 3", TaskStatus::Todo, TaskPriority::Low),
        ];

        let result = ScgTaskQueryBuilder::new()
            .with_status(TaskStatus::Todo)
            .execute(&tasks);

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|t| t.status == TaskStatus::Todo));
    }

    #[test]
    fn test_query_builder_priority_filter() {
        let tasks = vec![
            create_test_task("Task 1", TaskStatus::Todo, TaskPriority::High),
            create_test_task("Task 2", TaskStatus::Todo, TaskPriority::Medium),
            create_test_task("Task 3", TaskStatus::Todo, TaskPriority::High),
        ];

        let result = ScgTaskQueryBuilder::new()
            .with_priority(TaskPriority::High)
            .execute(&tasks);

        assert_eq!(result.len(), 2);
        assert!(result.iter().all(|t| t.priority == TaskPriority::High));
    }

    #[test]
    fn test_query_builder_search() {
        let tasks = vec![
            create_test_task("Authentication system", TaskStatus::Todo, TaskPriority::High),
            create_test_task("Database setup", TaskStatus::Todo, TaskPriority::Medium),
            create_test_task("Auth middleware", TaskStatus::Todo, TaskPriority::Low),
        ];

        let result = ScgTaskQueryBuilder::new()
            .search("auth".to_string())
            .execute(&tasks);

        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_query_builder_pagination() {
        let tasks: Vec<Task> = (0..10)
            .map(|i| create_test_task(&format!("Task {}", i), TaskStatus::Todo, TaskPriority::Medium))
            .collect();

        let page1 = ScgTaskQueryBuilder::new()
            .limit(3)
            .offset(0)
            .execute(&tasks);

        let page2 = ScgTaskQueryBuilder::new()
            .limit(3)
            .offset(3)
            .execute(&tasks);

        assert_eq!(page1.len(), 3);
        assert_eq!(page2.len(), 3);
    }

    #[test]
    fn test_query_builder_combined_filters() {
        let tasks = vec![
            create_test_task("High priority todo", TaskStatus::Todo, TaskPriority::High),
            create_test_task("Medium priority todo", TaskStatus::Todo, TaskPriority::Medium),
            create_test_task("High priority done", TaskStatus::Done, TaskPriority::High),
        ];

        let result = ScgTaskQueryBuilder::new()
            .with_status(TaskStatus::Todo)
            .with_priority(TaskPriority::High)
            .execute(&tasks);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].title, "High priority todo");
    }
}
