# Task Queries Documentation

## Overview

The `task_queries` module provides comprehensive task retrieval and querying functionality for the Descartes orchestration system. It extends the basic `StateStore` trait with advanced querying capabilities needed for Kanban views, task lists, and dependency management.

## Features

- **Basic Retrieval**: Get tasks by ID, status, priority, complexity, or assignee
- **Advanced Filtering**: Query builder pattern for complex multi-criteria filtering
- **Sorting & Pagination**: Flexible sorting and pagination support
- **Dependency Resolution**: Track and query task dependencies
- **Kanban Board Views**: Organize tasks by status for board visualization
- **Statistics**: Get aggregate task statistics
- **Performance Optimized**: Uses SQL indexes and efficient queries

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
descartes-core = "0.1.0"
```

## Quick Start

```rust
use descartes_core::{SqliteStateStore, StateStore, TaskQueries, TaskStatus};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize database
    let mut store = SqliteStateStore::new("tasks.db", false).await?;
    store.initialize().await?;

    // Create TaskQueries instance
    let queries = TaskQueries::new(store.pool().clone());

    // Get all todo tasks
    let todo_tasks = queries.get_tasks_by_status(TaskStatus::Todo).await?;

    println!("Todo tasks: {}", todo_tasks.len());
    Ok(())
}
```

## API Reference

### TaskQueries

Main interface for task retrieval operations.

#### Constructor

```rust
pub fn new(pool: SqlitePool) -> Self
```

Creates a new `TaskQueries` instance from a SQLite connection pool.

#### Basic Retrieval Methods

##### get_task_by_id

```rust
pub async fn get_task_by_id(&self, id: &Uuid) -> StateStoreResult<Option<Task>>
```

Retrieves a single task by its unique ID.

**Example:**
```rust
let task = queries.get_task_by_id(&task_id).await?;
if let Some(t) = task {
    println!("Found task: {}", t.title);
}
```

##### get_all_tasks

```rust
pub async fn get_all_tasks(&self) -> StateStoreResult<Vec<Task>>
```

Retrieves all tasks in the system (limited to 1000).

**Example:**
```rust
let all_tasks = queries.get_all_tasks().await?;
println!("Total tasks: {}", all_tasks.len());
```

##### get_tasks_by_status

```rust
pub async fn get_tasks_by_status(&self, status: TaskStatus) -> StateStoreResult<Vec<Task>>
```

Retrieves all tasks with a specific status.

**Example:**
```rust
let in_progress = queries.get_tasks_by_status(TaskStatus::InProgress).await?;
```

##### get_tasks_by_priority

```rust
pub async fn get_tasks_by_priority(&self, priority: TaskPriority) -> StateStoreResult<Vec<Task>>
```

Retrieves all tasks with a specific priority level.

**Example:**
```rust
let critical_tasks = queries.get_tasks_by_priority(TaskPriority::Critical).await?;
```

##### get_tasks_by_complexity

```rust
pub async fn get_tasks_by_complexity(&self, complexity: TaskComplexity) -> StateStoreResult<Vec<Task>>
```

Retrieves all tasks with a specific complexity level.

**Example:**
```rust
let simple_tasks = queries.get_tasks_by_complexity(TaskComplexity::Simple).await?;
```

##### get_tasks_by_assignee

```rust
pub async fn get_tasks_by_assignee(&self, assignee: &str) -> StateStoreResult<Vec<Task>>
```

Retrieves all tasks assigned to a specific user/agent.

**Example:**
```rust
let alice_tasks = queries.get_tasks_by_assignee("alice").await?;
```

##### get_unassigned_tasks

```rust
pub async fn get_unassigned_tasks(&self) -> StateStoreResult<Vec<Task>>
```

Retrieves all tasks that have no assignee.

**Example:**
```rust
let unassigned = queries.get_unassigned_tasks().await?;
```

#### Dependency Methods

##### get_task_dependencies

```rust
pub async fn get_task_dependencies(&self, task_id: &Uuid) -> StateStoreResult<Vec<Task>>
```

Gets all tasks that a specific task depends on (prerequisites).

**Example:**
```rust
let dependencies = queries.get_task_dependencies(&task_id).await?;
for dep in dependencies {
    println!("Depends on: {}", dep.title);
}
```

##### get_dependent_tasks

```rust
pub async fn get_dependent_tasks(&self, task_id: &Uuid) -> StateStoreResult<Vec<Task>>
```

Gets all tasks that depend on a specific task.

**Example:**
```rust
let dependents = queries.get_dependent_tasks(&task_id).await?;
for dep in dependents {
    println!("Blocks: {}", dep.title);
}
```

##### check_if_task_is_blocked

```rust
pub async fn check_if_task_is_blocked(&self, task_id: &Uuid) -> StateStoreResult<bool>
```

Checks if a task is blocked by unfinished dependencies.

**Example:**
```rust
let is_blocked = queries.check_if_task_is_blocked(&task_id).await?;
if is_blocked {
    println!("Task is blocked by dependencies");
}
```

##### get_blocked_tasks

```rust
pub async fn get_blocked_tasks(&self) -> StateStoreResult<Vec<Task>>
```

Gets all tasks that are currently blocked by dependencies.

##### get_ready_tasks

```rust
pub async fn get_ready_tasks(&self) -> StateStoreResult<Vec<Task>>
```

Gets all tasks that are ready to work on (status=Todo, no blocking dependencies).

**Example:**
```rust
let ready = queries.get_ready_tasks().await?;
println!("{} tasks ready to work on", ready.len());
```

#### View Methods

##### get_kanban_tasks

```rust
pub async fn get_kanban_tasks(&self) -> StateStoreResult<KanbanBoard>
```

Gets tasks organized by status for Kanban board visualization.

**Example:**
```rust
let kanban = queries.get_kanban_tasks().await?;
println!("Todo: {}", kanban.todo.len());
println!("In Progress: {}", kanban.in_progress.len());
println!("Done: {}", kanban.done.len());
println!("Blocked: {}", kanban.blocked.len());
```

##### get_task_statistics

```rust
pub async fn get_task_statistics(&self) -> StateStoreResult<TaskStatistics>
```

Gets aggregate statistics about tasks.

**Example:**
```rust
let stats = queries.get_task_statistics().await?;
println!("Total: {}, Todo: {}, Done: {}",
         stats.total, stats.todo, stats.done);
```

### TaskQueryBuilder

Builder pattern for constructing complex queries with multiple filters.

#### Constructor

```rust
pub fn new() -> Self
```

Creates a new query builder with default settings.

#### Filter Methods

##### with_status

```rust
pub fn with_status(mut self, status: TaskStatus) -> Self
```

Filter by a single status.

##### with_statuses

```rust
pub fn with_statuses(mut self, statuses: Vec<TaskStatus>) -> Self
```

Filter by multiple statuses.

##### with_priority

```rust
pub fn with_priority(mut self, priority: TaskPriority) -> Self
```

Filter by a single priority level.

##### with_priorities

```rust
pub fn with_priorities(mut self, priorities: Vec<TaskPriority>) -> Self
```

Filter by multiple priority levels.

##### with_complexity

```rust
pub fn with_complexity(mut self, complexity: TaskComplexity) -> Self
```

Filter by a single complexity level.

##### with_complexities

```rust
pub fn with_complexities(mut self, complexities: Vec<TaskComplexity>) -> Self
```

Filter by multiple complexity levels.

##### assigned_to

```rust
pub fn assigned_to(mut self, assignee: String) -> Self
```

Filter by a single assignee.

##### assigned_to_any

```rust
pub fn assigned_to_any(mut self, assignees: Vec<String>) -> Self
```

Filter by multiple assignees.

##### unassigned_only

```rust
pub fn unassigned_only(mut self) -> Self
```

Filter to show only unassigned tasks.

##### search

```rust
pub fn search(mut self, term: String) -> Self
```

Search in task title and description.

#### Sorting Methods

##### sort_by

```rust
pub fn sort_by(mut self, field: TaskSortField) -> Self
```

Set the field to sort by. Options:
- `TaskSortField::CreatedAt`
- `TaskSortField::UpdatedAt`
- `TaskSortField::Priority`
- `TaskSortField::Complexity`
- `TaskSortField::Title`
- `TaskSortField::Status`

##### order

```rust
pub fn order(mut self, order: SortOrder) -> Self
```

Set the sort order:
- `SortOrder::Ascending`
- `SortOrder::Descending`

#### Pagination Methods

##### limit

```rust
pub fn limit(mut self, limit: i64) -> Self
```

Set the maximum number of results to return.

##### offset

```rust
pub fn offset(mut self, offset: i64) -> Self
```

Set the number of results to skip.

#### Execution

##### execute

```rust
pub async fn execute(&self, pool: &SqlitePool) -> StateStoreResult<Vec<Task>>
```

Execute the query and return matching tasks.

**Example:**
```rust
let tasks = TaskQueryBuilder::new()
    .with_status(TaskStatus::Todo)
    .with_priority(TaskPriority::High)
    .assigned_to("alice".to_string())
    .sort_by(TaskSortField::Priority)
    .order(SortOrder::Descending)
    .limit(10)
    .execute(queries.pool())
    .await?;
```

## Complex Query Examples

### High Priority Tasks for Specific Users

```rust
let tasks = TaskQueryBuilder::new()
    .with_statuses(vec![TaskStatus::Todo, TaskStatus::InProgress])
    .with_priorities(vec![TaskPriority::High, TaskPriority::Critical])
    .assigned_to_any(vec!["alice".to_string(), "bob".to_string()])
    .sort_by(TaskSortField::Priority)
    .order(SortOrder::Descending)
    .execute(queries.pool())
    .await?;
```

### Search with Pagination

```rust
let page_size = 20;
let page = 2;

let results = TaskQueryBuilder::new()
    .search("authentication".to_string())
    .sort_by(TaskSortField::UpdatedAt)
    .order(SortOrder::Descending)
    .limit(page_size)
    .offset(page * page_size)
    .execute(queries.pool())
    .await?;
```

### Complex Tasks Assigned to Specific User

```rust
let complex_tasks = TaskQueryBuilder::new()
    .with_complexity(TaskComplexity::Complex)
    .assigned_to("alice".to_string())
    .with_status(TaskStatus::InProgress)
    .sort_by(TaskSortField::UpdatedAt)
    .order(SortOrder::Descending)
    .execute(queries.pool())
    .await?;
```

## Database Schema

The task queries use the following database tables and indexes:

### Tasks Table

```sql
CREATE TABLE tasks (
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
    metadata TEXT
);
```

### Task Dependencies Table

```sql
CREATE TABLE task_dependencies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id TEXT NOT NULL,
    depends_on_task_id TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    FOREIGN KEY (depends_on_task_id) REFERENCES tasks(id) ON DELETE CASCADE,
    UNIQUE(task_id, depends_on_task_id)
);
```

### Indexes

The following indexes are automatically created for optimal query performance:

```sql
CREATE INDEX idx_tasks_status ON tasks(status);
CREATE INDEX idx_tasks_assigned_to ON tasks(assigned_to);
CREATE INDEX idx_tasks_priority ON tasks(priority);
CREATE INDEX idx_tasks_complexity ON tasks(complexity);
CREATE INDEX idx_tasks_status_updated ON tasks(status, updated_at DESC);
CREATE INDEX idx_tasks_priority_status ON tasks(priority, status);
CREATE INDEX idx_tasks_complexity_status ON tasks(complexity, status);
CREATE INDEX idx_task_dependencies_task_id ON task_dependencies(task_id);
CREATE INDEX idx_task_dependencies_depends_on ON task_dependencies(depends_on_task_id);
```

## Performance Considerations

1. **Pagination**: Always use pagination for large result sets to avoid memory issues
2. **Indexes**: The module uses optimized indexes for common query patterns
3. **Dependency Queries**: Dependency resolution involves multiple queries, so use caching when appropriate
4. **Batch Operations**: For bulk operations, consider using transactions

## Testing

The module includes comprehensive tests covering:
- Basic retrieval operations
- Query builder functionality
- Filtering and sorting
- Pagination
- Dependency resolution
- Kanban board views
- Task statistics

Run tests with:
```bash
cargo test --package descartes-core task_queries
```

## Integration with GUI

The task queries API is designed to integrate seamlessly with GUI components:

### Kanban Board

```rust
let kanban = queries.get_kanban_tasks().await?;

// Render columns
render_column("To Do", &kanban.todo);
render_column("In Progress", &kanban.in_progress);
render_column("Done", &kanban.done);
render_column("Blocked", &kanban.blocked);
```

### Task List with Filters

```rust
let tasks = TaskQueryBuilder::new()
    .with_status(selected_status)
    .with_priority(selected_priority)
    .assigned_to(selected_user)
    .search(search_term)
    .sort_by(sort_field)
    .order(sort_order)
    .limit(page_size)
    .offset(current_page * page_size)
    .execute(queries.pool())
    .await?;

render_task_list(tasks);
```

### Dependency Graph

```rust
let task = queries.get_task_by_id(&task_id).await?.unwrap();
let dependencies = queries.get_task_dependencies(&task.id).await?;
let dependents = queries.get_dependent_tasks(&task.id).await?;

render_dependency_graph(task, dependencies, dependents);
```

## Future Enhancements

Potential future enhancements include:
- Full-text search using FTS5
- Task history tracking
- Custom field filtering
- Saved queries/views
- Export to various formats
- Real-time updates via WebSocket

## License

Part of the Descartes orchestration system.
