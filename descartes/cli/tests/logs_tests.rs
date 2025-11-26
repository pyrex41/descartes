/// Integration tests for the logs command
use sqlx::sqlite::SqlitePool;
use std::time::SystemTime;
use tempfile::TempDir;
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
        CREATE TABLE IF NOT EXISTS events (
            id TEXT PRIMARY KEY,
            event_type TEXT NOT NULL,
            timestamp INTEGER NOT NULL,
            session_id TEXT NOT NULL,
            actor_type TEXT NOT NULL,
            actor_id TEXT NOT NULL,
            content TEXT NOT NULL,
            metadata TEXT,
            git_commit TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
        CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to create events table");

    pool
}

/// Insert a test event into the database
async fn insert_test_event(
    pool: &SqlitePool,
    event_type: &str,
    session_id: &str,
    actor_type: &str,
    actor_id: &str,
    content: &str,
) -> String {
    let id = Uuid::new_v4().to_string();
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    sqlx::query(
        r#"
        INSERT INTO events (id, event_type, timestamp, session_id, actor_type, actor_id, content)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(&id)
    .bind(event_type)
    .bind(timestamp)
    .bind(session_id)
    .bind(actor_type)
    .bind(actor_id)
    .bind(content)
    .execute(pool)
    .await
    .expect("Failed to insert test event");

    id
}

#[tokio::test]
async fn test_logs_empty_events() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Query for events - should be empty
    let rows = sqlx::query("SELECT * FROM events ORDER BY timestamp DESC LIMIT 100")
        .fetch_all(&pool)
        .await
        .expect("Failed to query events");

    assert!(rows.is_empty(), "Should have no events initially");
}

#[tokio::test]
async fn test_logs_query_by_session() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    let session_id = "session-123";

    // Insert events for different sessions
    insert_test_event(
        &pool,
        "agent_started",
        session_id,
        "System",
        "system",
        "Agent started",
    )
    .await;
    insert_test_event(
        &pool,
        "agent_started",
        "other-session",
        "System",
        "system",
        "Other agent started",
    )
    .await;

    // Query by session
    let rows = sqlx::query("SELECT * FROM events WHERE session_id = ?1 ORDER BY timestamp DESC")
        .bind(session_id)
        .fetch_all(&pool)
        .await
        .expect("Failed to query events");

    assert_eq!(rows.len(), 1, "Should have one event for session");
}

#[tokio::test]
async fn test_logs_query_by_event_type() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    let session_id = "session-456";

    // Insert events of different types
    insert_test_event(
        &pool,
        "agent_started",
        session_id,
        "System",
        "system",
        "Started",
    )
    .await;
    insert_test_event(
        &pool,
        "error",
        session_id,
        "Agent",
        "agent-1",
        "Something failed",
    )
    .await;
    insert_test_event(
        &pool,
        "agent_completed",
        session_id,
        "System",
        "system",
        "Completed",
    )
    .await;

    // Query by event type
    let error_rows =
        sqlx::query("SELECT * FROM events WHERE event_type = ?1 ORDER BY timestamp DESC")
            .bind("error")
            .fetch_all(&pool)
            .await
            .expect("Failed to query events");

    assert_eq!(error_rows.len(), 1, "Should have one error event");
}

#[tokio::test]
async fn test_logs_limit() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    let session_id = "session-789";

    // Insert many events
    for i in 0..150 {
        insert_test_event(
            &pool,
            "info",
            session_id,
            "Agent",
            &format!("agent-{}", i),
            &format!("Event {}", i),
        )
        .await;
    }

    // Query with limit
    let rows = sqlx::query("SELECT * FROM events ORDER BY timestamp DESC LIMIT 100")
        .fetch_all(&pool)
        .await
        .expect("Failed to query events");

    assert_eq!(rows.len(), 100, "Should be limited to 100 events");
}

#[tokio::test]
async fn test_logs_event_data_integrity() {
    use sqlx::Row;

    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    let event_id = insert_test_event(
        &pool,
        "agent_started",
        "test-session",
        "User",
        "user-123",
        "Test content message",
    )
    .await;

    // Query and verify data
    let row = sqlx::query(
        "SELECT id, event_type, session_id, actor_type, actor_id, content FROM events WHERE id = ?1"
    )
    .bind(&event_id)
    .fetch_one(&pool)
    .await
    .expect("Failed to fetch event");

    let id: String = row.get("id");
    let event_type: String = row.get("event_type");
    let session_id: String = row.get("session_id");
    let actor_type: String = row.get("actor_type");
    let actor_id: String = row.get("actor_id");
    let content: String = row.get("content");

    assert_eq!(id, event_id);
    assert_eq!(event_type, "agent_started");
    assert_eq!(session_id, "test-session");
    assert_eq!(actor_type, "User");
    assert_eq!(actor_id, "user-123");
    assert_eq!(content, "Test content message");
}

#[tokio::test]
async fn test_logs_combined_filters() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Insert various events
    insert_test_event(&pool, "info", "session-a", "Agent", "agent-1", "Info 1").await;
    insert_test_event(&pool, "error", "session-a", "Agent", "agent-1", "Error 1").await;
    insert_test_event(&pool, "info", "session-b", "Agent", "agent-2", "Info 2").await;
    insert_test_event(&pool, "error", "session-b", "Agent", "agent-2", "Error 2").await;

    // Query with both session and event type filter
    let rows = sqlx::query(
        "SELECT * FROM events WHERE session_id = ?1 AND event_type = ?2 ORDER BY timestamp DESC",
    )
    .bind("session-a")
    .bind("error")
    .fetch_all(&pool)
    .await
    .expect("Failed to query events");

    assert_eq!(rows.len(), 1, "Should have one error event for session-a");
}

#[tokio::test]
async fn test_logs_ordering() {
    use sqlx::Row;

    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Insert events with small delay to ensure different timestamps
    insert_test_event(&pool, "first", "session", "System", "sys", "First event").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    insert_test_event(&pool, "second", "session", "System", "sys", "Second event").await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
    insert_test_event(&pool, "third", "session", "System", "sys", "Third event").await;

    // Query in descending order (newest first)
    let rows = sqlx::query("SELECT event_type FROM events ORDER BY timestamp DESC")
        .fetch_all(&pool)
        .await
        .expect("Failed to query events");

    assert_eq!(rows.len(), 3);

    let first_type: String = rows[0].get("event_type");
    let last_type: String = rows[2].get("event_type");

    assert_eq!(first_type, "third", "Newest event should be first");
    assert_eq!(last_type, "first", "Oldest event should be last");
}

#[tokio::test]
async fn test_logs_search_content() {
    let temp_dir = create_temp_dir();
    let pool = setup_test_db(&temp_dir).await;

    // Insert events with different content
    insert_test_event(
        &pool,
        "info",
        "session",
        "Agent",
        "agent-1",
        "Hello world message",
    )
    .await;
    insert_test_event(
        &pool,
        "info",
        "session",
        "Agent",
        "agent-2",
        "Goodbye message",
    )
    .await;
    insert_test_event(
        &pool,
        "info",
        "session",
        "Agent",
        "agent-3",
        "Another content",
    )
    .await;

    // Search in content
    let rows = sqlx::query("SELECT * FROM events WHERE content LIKE ?1")
        .bind("%message%")
        .fetch_all(&pool)
        .await
        .expect("Failed to query events");

    assert_eq!(
        rows.len(),
        2,
        "Should find two events with 'message' in content"
    );
}
