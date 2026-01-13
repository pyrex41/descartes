use chrono::Utc;
/// Example demonstrating task retrieval and querying functionality
/// This example shows how to use the TaskQueries API to retrieve, filter, and manage tasks
use descartes_core::{
    SortOrder, SqliteStateStore, StateStore, Task, TaskComplexity, TaskPriority, TaskQueries,
    TaskQueryBuilder, TaskSortField, TaskStatus,
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Task Queries Example ===\n");

    // 1. Initialize the database
    let mut store = SqliteStateStore::new("/tmp/task_queries_example.db", false).await?;
    store.initialize().await?;
    println!("✓ Database initialized\n");

    // 2. Create TaskQueries instance
    let queries = TaskQueries::new(store.pool().clone());

    // 3. Populate with sample tasks
    println!("Creating sample tasks...");
    create_sample_tasks(&store).await?;
    println!("✓ Sample tasks created\n");

    // 4. Basic retrieval examples
    println!("--- Basic Retrieval ---");

    // Get all tasks
    let all_tasks = queries.get_all_tasks().await?;
    println!("Total tasks: {}", all_tasks.len());

    // Get tasks by status
    let todo_tasks = queries.get_tasks_by_status(TaskStatus::Todo).await?;
    println!("Todo tasks: {}", todo_tasks.len());

    let in_progress_tasks = queries.get_tasks_by_status(TaskStatus::InProgress).await?;
    println!("In Progress tasks: {}", in_progress_tasks.len());

    // Get tasks by priority
    let high_priority = queries.get_tasks_by_priority(TaskPriority::High).await?;
    println!("High priority tasks: {}", high_priority.len());

    // Get tasks by complexity
    let complex_tasks = queries
        .get_tasks_by_complexity(TaskComplexity::Complex)
        .await?;
    println!("Complex tasks: {}", complex_tasks.len());
    println!();

    // 5. Query Builder examples
    println!("--- Query Builder Examples ---");

    // Filter by multiple criteria
    let filtered_tasks = TaskQueryBuilder::new()
        .with_status(TaskStatus::Todo)
        .with_priority(TaskPriority::High)
        .sort_by(TaskSortField::Priority)
        .order(SortOrder::Descending)
        .limit(10)
        .execute(queries.pool())
        .await?;
    println!("High priority Todo tasks: {}", filtered_tasks.len());
    for task in &filtered_tasks {
        println!("  - {} (Priority: {:?})", task.title, task.priority);
    }
    println!();

    // Search tasks
    let search_results = TaskQueryBuilder::new()
        .search("API".to_string())
        .execute(queries.pool())
        .await?;
    println!("Tasks matching 'API': {}", search_results.len());
    for task in &search_results {
        println!("  - {}", task.title);
    }
    println!();

    // Filter by assignee
    let alice_tasks = TaskQueryBuilder::new()
        .assigned_to("alice".to_string())
        .execute(queries.pool())
        .await?;
    println!("Tasks assigned to Alice: {}", alice_tasks.len());
    for task in &alice_tasks {
        println!("  - {} ({:?})", task.title, task.status);
    }
    println!();

    // Get unassigned tasks
    let unassigned = queries.get_unassigned_tasks().await?;
    println!("Unassigned tasks: {}", unassigned.len());
    println!();

    // 6. Dependency resolution examples
    println!("--- Dependency Resolution ---");

    // Create tasks with dependencies
    let (task_a, task_b) = create_dependent_tasks(&store).await?;

    // Get dependencies
    let dependencies = queries.get_task_dependencies(&task_b.id).await?;
    println!("Task '{}' depends on:", task_b.title);
    for dep in &dependencies {
        println!("  - {} ({:?})", dep.title, dep.status);
    }

    // Get dependent tasks
    let dependents = queries.get_dependent_tasks(&task_a.id).await?;
    println!("\nTasks that depend on '{}':", task_a.title);
    for dep in &dependents {
        println!("  - {} ({:?})", dep.title, dep.status);
    }

    // Check if task is blocked
    let is_blocked = queries.check_if_task_is_blocked(&task_b.id).await?;
    println!("\nIs '{}' blocked? {}", task_b.title, is_blocked);
    println!();

    // 7. Kanban board view
    println!("--- Kanban Board View ---");
    let kanban = queries.get_kanban_tasks().await?;
    println!("Todo: {} tasks", kanban.todo.len());
    println!("In Progress: {} tasks", kanban.in_progress.len());
    println!("Done: {} tasks", kanban.done.len());
    println!("Blocked: {} tasks", kanban.blocked.len());
    println!();

    // 8. Task statistics
    println!("--- Task Statistics ---");
    let stats = queries.get_task_statistics().await?;
    println!("Total: {}", stats.total);
    println!("Todo: {}", stats.todo);
    println!("In Progress: {}", stats.in_progress);
    println!("Done: {}", stats.done);
    println!("Blocked: {}", stats.blocked);
    println!();

    // 9. Ready tasks (tasks that can be worked on)
    println!("--- Ready Tasks ---");
    let ready_tasks = queries.get_ready_tasks().await?;
    println!("Tasks ready to work on: {}", ready_tasks.len());
    for task in &ready_tasks {
        println!(
            "  - {} (Priority: {:?}, Complexity: {:?})",
            task.title, task.priority, task.complexity
        );
    }
    println!();

    // 10. Pagination example
    println!("--- Pagination Example ---");
    let page_size = 3;
    for page in 0..2 {
        let tasks = TaskQueryBuilder::new()
            .sort_by(TaskSortField::UpdatedAt)
            .order(SortOrder::Descending)
            .limit(page_size)
            .offset(page * page_size)
            .execute(queries.pool())
            .await?;

        println!("Page {} ({} tasks):", page + 1, tasks.len());
        for task in &tasks {
            println!("  - {}", task.title);
        }
    }
    println!();

    // 11. Complex query example
    println!("--- Complex Query Example ---");
    let complex_query_results = TaskQueryBuilder::new()
        .with_statuses(vec![TaskStatus::Todo, TaskStatus::InProgress])
        .with_priorities(vec![TaskPriority::High, TaskPriority::Critical])
        .assigned_to_any(vec!["alice".to_string(), "bob".to_string()])
        .sort_by(TaskSortField::Priority)
        .order(SortOrder::Descending)
        .limit(10)
        .execute(queries.pool())
        .await?;

    println!("High/Critical priority Todo/InProgress tasks for Alice or Bob:");
    for task in &complex_query_results {
        println!(
            "  - {} (Priority: {:?}, Status: {:?}, Assigned: {:?})",
            task.title, task.priority, task.status, task.assigned_to
        );
    }

    println!("\n=== Example completed successfully ===");
    Ok(())
}

/// Create sample tasks for demonstration
async fn create_sample_tasks(store: &SqliteStateStore) -> Result<(), Box<dyn std::error::Error>> {
    let now = Utc::now().timestamp();

    let sample_tasks = vec![
        Task {
            id: Uuid::new_v4(),
            title: "Implement user authentication".to_string(),
            description: Some("Add JWT-based authentication to the API".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![],
            created_at: now - 3600,
            updated_at: now - 1800,
            metadata: None,
        },
        Task {
            id: Uuid::new_v4(),
            title: "Design database schema".to_string(),
            description: Some("Create schema for user and task tables".to_string()),
            status: TaskStatus::Done,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("bob".to_string()),
            dependencies: vec![],
            created_at: now - 7200,
            updated_at: now - 3600,
            metadata: None,
        },
        Task {
            id: Uuid::new_v4(),
            title: "Write API documentation".to_string(),
            description: Some("Document all REST API endpoints".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Simple,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![],
            created_at: now - 1800,
            updated_at: now - 900,
            metadata: None,
        },
        Task {
            id: Uuid::new_v4(),
            title: "Set up CI/CD pipeline".to_string(),
            description: Some(
                "Configure GitHub Actions for automated testing and deployment".to_string(),
            ),
            status: TaskStatus::Todo,
            priority: TaskPriority::Critical,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("charlie".to_string()),
            dependencies: vec![],
            created_at: now - 900,
            updated_at: now - 450,
            metadata: None,
        },
        Task {
            id: Uuid::new_v4(),
            title: "Add error handling".to_string(),
            description: Some(
                "Implement comprehensive error handling across the application".to_string(),
            ),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("bob".to_string()),
            dependencies: vec![],
            created_at: now - 450,
            updated_at: now - 225,
            metadata: None,
        },
        Task {
            id: Uuid::new_v4(),
            title: "Create unit tests".to_string(),
            description: Some("Write comprehensive unit tests for core modules".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: now - 225,
            updated_at: now,
            metadata: None,
        },
        Task {
            id: Uuid::new_v4(),
            title: "Optimize database queries".to_string(),
            description: Some("Add indexes and optimize slow queries".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::Low,
            complexity: TaskComplexity::Moderate,
            assigned_to: None,
            dependencies: vec![],
            created_at: now,
            updated_at: now,
            metadata: None,
        },
    ];

    for task in sample_tasks {
        store.save_task(&task).await?;
    }

    Ok(())
}

/// Create tasks with dependencies
async fn create_dependent_tasks(
    store: &SqliteStateStore,
) -> Result<(Task, Task), Box<dyn std::error::Error>> {
    let now = Utc::now().timestamp();

    // Task A - prerequisite
    let task_a = Task {
        id: Uuid::new_v4(),
        title: "Create API endpoints".to_string(),
        description: Some("Implement REST API endpoints".to_string()),
        status: TaskStatus::InProgress,
        priority: TaskPriority::High,
        complexity: TaskComplexity::Complex,
        assigned_to: Some("alice".to_string()),
        dependencies: vec![],
        created_at: now - 3600,
        updated_at: now - 1800,
        metadata: None,
    };
    store.save_task(&task_a).await?;

    // Task B - depends on Task A
    let task_b = Task {
        id: Uuid::new_v4(),
        title: "Write integration tests".to_string(),
        description: Some("Test API integration".to_string()),
        status: TaskStatus::Todo,
        priority: TaskPriority::Medium,
        complexity: TaskComplexity::Moderate,
        assigned_to: Some("bob".to_string()),
        dependencies: vec![task_a.id],
        created_at: now - 1800,
        updated_at: now - 900,
        metadata: None,
    };
    store.save_task(&task_b).await?;

    Ok((task_a, task_b))
}
