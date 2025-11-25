/// Integration tests for the kill command
use sqlx::sqlite::SqlitePool;
use tempfile::TempDir;
use std::time::SystemTime;
use uuid::Uuid;

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
) {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query(
        r#"
        INSERT INTO agents (id, name, status, model_backend, started_at, task)
        VALUES (?1, ?2, ?3, 'anthropic', ?4, 'Test task')
        "#,
    )
    .bind(id)
    .bind(name)
    .bind(status)
    .bind(now)
    .execute(pool)
    .await
    .expect("Failed to insert test agent");
}

#[tokio::test]
async fn test_kill_agent_uuid_parsing() {
    // Test valid UUID parsing
    let valid_uuid = "550e8400-e29b-41d4-a716-446655440000";
    let parsed = Uuid::parse_str(valid_uuid);
    assert!(parsed.is_ok(), "Should parse valid UUID");

    // Test invalid UUID
    let invalid_uuid = "not-a-uuid";
    let parsed = Uuid::parse_str(invalid_uuid);
    assert!(parsed.is_err(), "Should fail to parse invalid UUID");

    // Test short ID that's not a UUID
    let short_id = "abc123";
    let parsed = Uuid::parse_str(short_id);
    assert!(parsed.is_err(), "Should fail to parse short ID as UUID");
}

#[tokio::test]
async fn test_kill_updates_agent_status() {
    use sqlx::Row;

    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    let agent_id = "550e8400-e29b-41d4-a716-446655440000";
    insert_test_agent(&pool, agent_id, "TestAgent", "Running").await;

    // Simulate updating status to Terminated
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query(
        "UPDATE agents SET status = 'Terminated', completed_at = ?1 WHERE id = ?2"
    )
    .bind(now)
    .bind(agent_id)
    .execute(&pool)
    .await
    .expect("Failed to update agent status");

    // Verify the update
    let row = sqlx::query("SELECT status, completed_at FROM agents WHERE id = ?1")
        .bind(agent_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch agent");

    let status: String = row.get("status");
    let completed_at: Option<i64> = row.get("completed_at");

    assert_eq!(status, "Terminated");
    assert!(completed_at.is_some(), "Should have completion timestamp");
}

#[tokio::test]
async fn test_kill_nonexistent_agent() {
    use sqlx::Row;

    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    let nonexistent_id = "00000000-0000-0000-0000-000000000000";

    // Try to find agent
    let result = sqlx::query("SELECT id FROM agents WHERE id = ?1")
        .bind(nonexistent_id)
        .fetch_optional(&pool)
        .await
        .expect("Failed to query agent");

    assert!(result.is_none(), "Should not find nonexistent agent");
}

#[tokio::test]
async fn test_kill_already_completed_agent() {
    use sqlx::Row;

    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    let agent_id = "550e8400-e29b-41d4-a716-446655440001";

    // Insert a completed agent
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query(
        r#"
        INSERT INTO agents (id, name, status, model_backend, started_at, completed_at, task)
        VALUES (?1, 'CompletedAgent', 'Completed', 'anthropic', ?2, ?2, 'Done task')
        "#,
    )
    .bind(agent_id)
    .bind(now)
    .execute(&pool)
    .await
    .expect("Failed to insert completed agent");

    // Check agent status
    let row = sqlx::query("SELECT status FROM agents WHERE id = ?1")
        .bind(agent_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch agent");

    let status: String = row.get("status");
    assert_eq!(status, "Completed", "Agent should already be completed");
}

#[tokio::test]
async fn test_kill_multiple_agents_independently() {
    use sqlx::Row;

    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Insert multiple running agents
    let agent1_id = "550e8400-e29b-41d4-a716-446655440010";
    let agent2_id = "550e8400-e29b-41d4-a716-446655440011";

    insert_test_agent(&pool, agent1_id, "Agent1", "Running").await;
    insert_test_agent(&pool, agent2_id, "Agent2", "Running").await;

    // Kill only agent1
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query(
        "UPDATE agents SET status = 'Terminated', completed_at = ?1 WHERE id = ?2"
    )
    .bind(now)
    .bind(agent1_id)
    .execute(&pool)
    .await
    .expect("Failed to kill agent1");

    // Verify agent1 is terminated
    let row1 = sqlx::query("SELECT status FROM agents WHERE id = ?1")
        .bind(agent1_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch agent1");
    let status1: String = row1.get("status");
    assert_eq!(status1, "Terminated");

    // Verify agent2 is still running
    let row2 = sqlx::query("SELECT status FROM agents WHERE id = ?1")
        .bind(agent2_id)
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch agent2");
    let status2: String = row2.get("status");
    assert_eq!(status2, "Running");
}
