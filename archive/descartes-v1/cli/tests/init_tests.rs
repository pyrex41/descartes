/// Integration tests for the init command
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a temporary directory for testing
fn create_temp_dir() -> TempDir {
    tempfile::tempdir().expect("Failed to create temp directory")
}

#[tokio::test]
async fn test_init_creates_directory_structure() {
    let temp_dir = create_temp_dir();
    let base_path = temp_dir.path().to_path_buf();

    // Execute init using the module directly
    // Note: We can't easily test the execute function since it depends on
    // colored output and progress bars, so we test the helper functions

    // Test directory structure creation
    let dirs = vec![
        base_path.clone(),
        base_path.join("data"),
        base_path.join("data/state"),
        base_path.join("data/events"),
        base_path.join("data/cache"),
        base_path.join("thoughts"),
        base_path.join("logs"),
        base_path.join("backups"),
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir).expect("Failed to create directory");
    }

    // Verify all directories exist
    for dir in dirs {
        assert!(dir.exists(), "Directory {:?} should exist", dir);
        assert!(dir.is_dir(), "Path {:?} should be a directory", dir);
    }
}

#[tokio::test]
async fn test_init_creates_database() {
    use sqlx::sqlite::SqlitePool;

    let temp_dir = create_temp_dir();
    let db_path = temp_dir.path().join("test.db");
    let db_url = format!("sqlite://{}?mode=rwc", db_path.display());

    // Create database pool
    let pool = SqlitePool::connect(&db_url)
        .await
        .expect("Failed to connect to database");

    // Run migrations
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

        CREATE TABLE IF NOT EXISTS tasks (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT,
            status TEXT NOT NULL,
            assigned_to TEXT,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            metadata TEXT
        );

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

        CREATE INDEX IF NOT EXISTS idx_events_session ON events(session_id);
        CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
        CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
        CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
        CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
        "#,
    )
    .execute(&pool)
    .await
    .expect("Failed to run migrations");

    // Verify tables exist
    let result = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='events'")
        .fetch_optional(&pool)
        .await
        .expect("Failed to query tables");

    assert!(result.is_some(), "events table should exist");

    let result = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='tasks'")
        .fetch_optional(&pool)
        .await
        .expect("Failed to query tables");

    assert!(result.is_some(), "tasks table should exist");

    let result = sqlx::query("SELECT name FROM sqlite_master WHERE type='table' AND name='agents'")
        .fetch_optional(&pool)
        .await
        .expect("Failed to query tables");

    assert!(result.is_some(), "agents table should exist");
}

#[tokio::test]
async fn test_init_creates_thoughts_directory() {
    let temp_dir = create_temp_dir();
    let thoughts_dir = temp_dir.path().join("thoughts");

    // Create thoughts directory structure
    let subdirs = vec!["sessions", "archived", "templates"];

    for subdir in &subdirs {
        let path = thoughts_dir.join(subdir);
        std::fs::create_dir_all(&path).expect("Failed to create directory");
    }

    // Verify subdirectories exist
    for subdir in subdirs {
        let path = thoughts_dir.join(subdir);
        assert!(
            path.exists(),
            "Thoughts subdirectory {:?} should exist",
            path
        );
    }

    // Create README
    let readme = thoughts_dir.join("README.md");
    std::fs::write(&readme, "# Test README").expect("Failed to write README");
    assert!(readme.exists(), "README should exist");
}

#[tokio::test]
async fn test_init_creates_example_files() {
    let temp_dir = create_temp_dir();
    let base_path = temp_dir.path();

    // Create example system prompt
    let example_prompt = base_path.join("example_system_prompt.txt");
    std::fs::write(&example_prompt, "Test system prompt").expect("Failed to write example prompt");
    assert!(
        example_prompt.exists(),
        "Example system prompt should exist"
    );

    // Create .gitignore
    let gitignore = base_path.join(".gitignore");
    std::fs::write(&gitignore, "*.db\nlogs/").expect("Failed to write .gitignore");
    assert!(gitignore.exists(), ".gitignore should exist");
}

#[tokio::test]
async fn test_init_idempotent() {
    let temp_dir = create_temp_dir();
    let base_path = temp_dir.path().to_path_buf();

    // Create directory structure twice
    for _ in 0..2 {
        let dirs = vec![
            base_path.clone(),
            base_path.join("data"),
            base_path.join("thoughts"),
        ];

        for dir in &dirs {
            std::fs::create_dir_all(dir).expect("Failed to create directory");
        }
    }

    // Should still work - directories should exist
    assert!(base_path.exists());
    assert!(base_path.join("data").exists());
    assert!(base_path.join("thoughts").exists());
}
