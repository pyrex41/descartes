/// Integration tests for the ps command
use sqlx::sqlite::SqlitePool;
use tempfile::TempDir;
use std::time::SystemTime;

/// Helper to create a temporary directory for testing
fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

/// Set up a test database with the required schema
async fn setup_test_db(temp_dir: &TempDir) -> SqlitePool {
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    let pool = SqlitePool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS agents (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            model_backend TEXT NOT NULL,
            started_at INTEGER NOT NULL,
            completed_at INTEGER,
            task TEXT NOT NULL,
            metadata TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create agents table");

    pool
}

/// Insert a test agent into the database
async fn insert_test_agent(
    pool: &SqlitePool,
    id: &str,
    name: &str,
    status: &str,
    backend: &str,
    task: &str,
) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query(
        r#"
        INSERT INTO agents (id, name, status, model_backend, started_at, task)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(status)
    .bind(backend)
    .bind(now)
    .bind(task)
    .execute(pool)
    .await
    .expect("Failed to insert test agent");
}

#[tokio::test]
async fn test_ps_empty_agents() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Query for agents - should be empty
    let rows = sqlx::query(
        "SELECT id, name, status, model_backend, started_at, completed_at, task FROM agents ORDER BY started_at DESC"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to query agents");

    assert!(rows.is_empty(), "Should have no agents initially");
}

#[tokio::test]
async fn test_ps_with_running_agent() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Insert a running agent
    insert_test_agent(
        &pool,
        "test-agent-1",
        "TestAgent",
        "Running",
        "anthropic",
        "Test task",
    )
    .await;

    // Query for running agents
    let rows = sqlx::query(
        "SELECT id, name, status, model_backend, started_at, completed_at, task FROM agents WHERE status IN ('Running', 'Idle', 'Paused') ORDER BY started_at DESC"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to query agents");

    assert_eq!(rows.len(), 1, "Should have one running agent");
}

#[tokio::test]
async fn test_ps_show_all_agents() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Insert multiple agents with different statuses
    insert_test_agent(
        &pool,
        "agent-1",
        "RunningAgent",
        "Running",
        "anthropic",
        "Task 1",
    )
    .await;
    insert_test_agent(
        &pool,
        "agent-2",
        "CompletedAgent",
        "Completed",
        "openai",
        "Task 2",
    )
    .await;
    insert_test_agent(
        &pool,
        "agent-3",
        "FailedAgent",
        "Failed",
        "ollama",
        "Task 3",
    )
    .await;

    // Query without filter (show all)
    let all_rows = sqlx::query(
        "SELECT id, name, status, model_backend, started_at, completed_at, task FROM agents ORDER BY started_at DESC"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to query all agents");

    assert_eq!(all_rows.len(), 3, "Should have three agents total");

    // Query with filter (running only)
    let running_rows = sqlx::query(
        "SELECT id, name, status, model_backend, started_at, completed_at, task FROM agents WHERE status IN ('Running', 'Idle', 'Paused') ORDER BY started_at DESC"
    )
    .fetch_all(&pool)
    .await
    .expect("Failed to query running agents");

    assert_eq!(running_rows.len(), 1, "Should have one running agent");
}

#[tokio::test]
async fn test_ps_agent_data_integrity() {
    use sqlx::Row;

    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Insert an agent with specific data
    insert_test_agent(
        &pool,
        "uuid-12345",
        "MyTestAgent",
        "Running",
        "deepseek",
        "Important task",
    )
    .await;

    // Query and verify data
    let rows = sqlx::query(
        "SELECT id, name, status, model_backend, task FROM agents WHERE id = ?1"
    )
    .bind("uuid-12345")
    .fetch_all(&pool)
    .await
    .expect("Failed to query agent");

    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    let id: String = row.get("id");
    let name: String = row.get("name");
    let status: String = row.get("status");
    let backend: String = row.get("model_backend");
    let task: String = row.get("task");

    assert_eq!(id, "uuid-12345");
    assert_eq!(name, "MyTestAgent");
    assert_eq!(status, "Running");
    assert_eq!(backend, "deepseek");
    assert_eq!(task, "Important task");
}
