# Task Queries Quick Reference

## Setup

```rust
use descartes_core::{
    SqliteStateStore, TaskQueries, TaskQueryBuilder,
    TaskStatus, TaskPriority, TaskComplexity,
    TaskSortField, SortOrder
};

// Initialize
let mut store = SqliteStateStore::new("tasks.db", false).await?;
store.initialize().await?;
let queries = TaskQueries::new(store.pool().clone());
```

## Basic Queries

### Get Task by ID
```rust
let task = queries.get_task_by_id(&task_id).await?;
```

### Get All Tasks
```rust
let all_tasks = queries.get_all_tasks().await?;
```

### Get by Status
```rust
let todo = queries.get_tasks_by_status(TaskStatus::Todo).await?;
let in_progress = queries.get_tasks_by_status(TaskStatus::InProgress).await?;
let done = queries.get_tasks_by_status(TaskStatus::Done).await?;
let blocked = queries.get_tasks_by_status(TaskStatus::Blocked).await?;
```

### Get by Priority
```rust
let critical = queries.get_tasks_by_priority(TaskPriority::Critical).await?;
let high = queries.get_tasks_by_priority(TaskPriority::High).await?;
let medium = queries.get_tasks_by_priority(TaskPriority::Medium).await?;
let low = queries.get_tasks_by_priority(TaskPriority::Low).await?;
```

### Get by Complexity
```rust
let epic = queries.get_tasks_by_complexity(TaskComplexity::Epic).await?;
let complex = queries.get_tasks_by_complexity(TaskComplexity::Complex).await?;
let moderate = queries.get_tasks_by_complexity(TaskComplexity::Moderate).await?;
let simple = queries.get_tasks_by_complexity(TaskComplexity::Simple).await?;
let trivial = queries.get_tasks_by_complexity(TaskComplexity::Trivial).await?;
```

### Get by Assignee
```rust
let alice_tasks = queries.get_tasks_by_assignee("alice").await?;
let unassigned = queries.get_unassigned_tasks().await?;
```

## Advanced Queries

### Filter by Multiple Statuses
```rust
let tasks = TaskQueryBuilder::new()
    .with_statuses(vec![TaskStatus::Todo, TaskStatus::InProgress])
    .execute(queries.pool())
    .await?;
```

### Filter by Multiple Priorities
```rust
let tasks = TaskQueryBuilder::new()
    .with_priorities(vec![TaskPriority::High, TaskPriority::Critical])
    .execute(queries.pool())
    .await?;
```

### High Priority Todo Tasks
```rust
let tasks = TaskQueryBuilder::new()
    .with_status(TaskStatus::Todo)
    .with_priority(TaskPriority::High)
    .execute(queries.pool())
    .await?;
```

### Complex Tasks for Specific User
```rust
let tasks = TaskQueryBuilder::new()
    .with_complexity(TaskComplexity::Complex)
    .assigned_to("alice".to_string())
    .execute(queries.pool())
    .await?;
```

### Search Tasks
```rust
let tasks = TaskQueryBuilder::new()
    .search("authentication".to_string())
    .execute(queries.pool())
    .await?;
```

### Tasks for Multiple Users
```rust
let tasks = TaskQueryBuilder::new()
    .assigned_to_any(vec!["alice".to_string(), "bob".to_string()])
    .execute(queries.pool())
    .await?;
```

## Sorting

### Sort by Priority (Descending)
```rust
let tasks = TaskQueryBuilder::new()
    .sort_by(TaskSortField::Priority)
    .order(SortOrder::Descending)
    .execute(queries.pool())
    .await?;
```

### Sort by Updated Date (Most Recent First)
```rust
let tasks = TaskQueryBuilder::new()
    .sort_by(TaskSortField::UpdatedAt)
    .order(SortOrder::Descending)
    .execute(queries.pool())
    .await?;
```

### Sort by Title (Alphabetical)
```rust
let tasks = TaskQueryBuilder::new()
    .sort_by(TaskSortField::Title)
    .order(SortOrder::Ascending)
    .execute(queries.pool())
    .await?;
```

## Pagination

### First Page (20 items)
```rust
let tasks = TaskQueryBuilder::new()
    .limit(20)
    .offset(0)
    .execute(queries.pool())
    .await?;
```

### Second Page
```rust
let tasks = TaskQueryBuilder::new()
    .limit(20)
    .offset(20)
    .execute(queries.pool())
    .await?;
```

### Generic Pagination
```rust
let page_size = 20;
let page_number = 2; // 0-indexed

let tasks = TaskQueryBuilder::new()
    .limit(page_size)
    .offset(page_number * page_size)
    .execute(queries.pool())
    .await?;
```

## Complex Queries

### High Priority Tasks for Team
```rust
let tasks = TaskQueryBuilder::new()
    .with_statuses(vec![TaskStatus::Todo, TaskStatus::InProgress])
    .with_priorities(vec![TaskPriority::High, TaskPriority::Critical])
    .assigned_to_any(vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()])
    .sort_by(TaskSortField::Priority)
    .order(SortOrder::Descending)
    .limit(50)
    .execute(queries.pool())
    .await?;
```

### Searchable Task List with Filters
```rust
let tasks = TaskQueryBuilder::new()
    .search("API".to_string())
    .with_status(TaskStatus::Todo)
    .with_priority(TaskPriority::High)
    .sort_by(TaskSortField::UpdatedAt)
    .order(SortOrder::Descending)
    .limit(10)
    .execute(queries.pool())
    .await?;
```

### Unassigned High Priority Tasks
```rust
let tasks = TaskQueryBuilder::new()
    .unassigned_only()
    .with_priorities(vec![TaskPriority::High, TaskPriority::Critical])
    .sort_by(TaskSortField::Priority)
    .order(SortOrder::Descending)
    .execute(queries.pool())
    .await?;
```

## Dependencies

### Get Task Prerequisites
```rust
let dependencies = queries.get_task_dependencies(&task_id).await?;
for dep in dependencies {
    println!("Depends on: {} ({:?})", dep.title, dep.status);
}
```

### Get Tasks Depending on This One
```rust
let dependents = queries.get_dependent_tasks(&task_id).await?;
for dep in dependents {
    println!("Blocks: {} ({:?})", dep.title, dep.status);
}
```

### Check if Task is Blocked
```rust
let is_blocked = queries.check_if_task_is_blocked(&task_id).await?;
if is_blocked {
    println!("Task is blocked by unfinished dependencies");
}
```

### Get All Blocked Tasks
```rust
let blocked_tasks = queries.get_blocked_tasks().await?;
```

### Get Ready Tasks (Can Start Working)
```rust
let ready_tasks = queries.get_ready_tasks().await?;
```

## Views

### Kanban Board
```rust
let kanban = queries.get_kanban_tasks().await?;

println!("📋 To Do: {}", kanban.todo.len());
for task in &kanban.todo {
    println!("  - {}", task.title);
}

println!("🔄 In Progress: {}", kanban.in_progress.len());
for task in &kanban.in_progress {
    println!("  - {}", task.title);
}

println!("✅ Done: {}", kanban.done.len());
for task in &kanban.done {
    println!("  - {}", task.title);
}

println!("🚫 Blocked: {}", kanban.blocked.len());
for task in &kanban.blocked {
    println!("  - {}", task.title);
}
```

### Task Statistics
```rust
let stats = queries.get_task_statistics().await?;

println!("Total Tasks: {}", stats.total);
println!("Todo: {} ({:.1}%)", stats.todo, (stats.todo as f64 / stats.total as f64) * 100.0);
println!("In Progress: {} ({:.1}%)", stats.in_progress, (stats.in_progress as f64 / stats.total as f64) * 100.0);
println!("Done: {} ({:.1}%)", stats.done, (stats.done as f64 / stats.total as f64) * 100.0);
println!("Blocked: {} ({:.1}%)", stats.blocked, (stats.blocked as f64 / stats.total as f64) * 100.0);
```

## Common Patterns

### Sprint Planning (High Priority, Not Done)
```rust
let sprint_tasks = TaskQueryBuilder::new()
    .with_statuses(vec![TaskStatus::Todo, TaskStatus::InProgress])
    .with_priorities(vec![TaskPriority::High, TaskPriority::Critical])
    .sort_by(TaskSortField::Priority)
    .order(SortOrder::Descending)
    .execute(queries.pool())
    .await?;
```

### User Dashboard (My Tasks)
```rust
let my_tasks = TaskQueryBuilder::new()
    .assigned_to("alice".to_string())
    .with_statuses(vec![TaskStatus::Todo, TaskStatus::InProgress])
    .sort_by(TaskSortField::Priority)
    .order(SortOrder::Descending)
    .execute(queries.pool())
    .await?;
```

### Backlog (Unassigned, Low Priority)
```rust
let backlog = TaskQueryBuilder::new()
    .unassigned_only()
    .with_status(TaskStatus::Todo)
    .with_priorities(vec![TaskPriority::Low, TaskPriority::Medium])
    .sort_by(TaskSortField::CreatedAt)
    .order(SortOrder::Ascending)
    .execute(queries.pool())
    .await?;
```

### Next Up (Ready to Start)
```rust
let next_up = queries.get_ready_tasks().await?;

// Further filter by priority
let high_priority_ready = next_up.into_iter()
    .filter(|t| t.priority >= TaskPriority::High)
    .collect::<Vec<_>>();
```

### Recently Updated
```rust
let recent = TaskQueryBuilder::new()
    .sort_by(TaskSortField::UpdatedAt)
    .order(SortOrder::Descending)
    .limit(10)
    .execute(queries.pool())
    .await?;
```

### Overdue or Stuck (In Progress for Long Time)
```rust
use chrono::Utc;

let all_in_progress = queries.get_tasks_by_status(TaskStatus::InProgress).await?;
let now = Utc::now().timestamp();
let one_week_ago = now - (7 * 24 * 3600);

let stuck_tasks = all_in_progress.into_iter()
    .filter(|t| t.updated_at < one_week_ago)
    .collect::<Vec<_>>();
```

## Performance Tips

1. **Always use indexes**: The module creates indexes automatically
2. **Use pagination**: Don't load all tasks at once for large datasets
3. **Cache results**: For frequently accessed data (e.g., Kanban board)
4. **Batch operations**: Group multiple queries when possible
5. **Filter early**: Apply filters in the query, not in application code

## Error Handling

```rust
match queries.get_task_by_id(&task_id).await {
    Ok(Some(task)) => println!("Found: {}", task.title),
    Ok(None) => println!("Task not found"),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Integration with GUI

### Reactive Filtering
```rust
// User changes filters in GUI
let tasks = TaskQueryBuilder::new()
    .with_status(selected_status)
    .with_priority(selected_priority)
    .assigned_to(selected_assignee)
    .search(search_term)
    .sort_by(sort_field)
    .order(sort_order)
    .limit(page_size)
    .offset(current_page * page_size)
    .execute(queries.pool())
    .await?;

// Update GUI with filtered tasks
update_task_list(tasks);
```

### Drag-and-Drop Update
```rust
// User drags task from "Todo" to "In Progress"
let mut task = queries.get_task_by_id(&task_id).await?.unwrap();
task.status = TaskStatus::InProgress;
task.updated_at = Utc::now().timestamp();
store.save_task(&task).await?;

// Refresh Kanban view
let kanban = queries.get_kanban_tasks().await?;
update_kanban_board(kanban);
```
