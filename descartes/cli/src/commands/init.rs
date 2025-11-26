use anyhow::Result;
use colored::Colorize;
use descartes_core::{ConfigManager, DescaratesConfig};
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::{info, warn};

pub async fn execute(name: Option<&str>, dir: Option<&Path>) -> Result<()> {
    let project_name = name.unwrap_or("descartes-project");

    println!(
        "{}",
        format!("Initializing Descartes project: {}", project_name)
            .green()
            .bold()
    );

    // Determine base directory
    let base_dir = if let Some(d) = dir {
        d.to_path_buf()
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join(".descartes")
    };

    info!("Base directory: {:?}", base_dir);

    // Create progress bar
    let pb = ProgressBar::new(5);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
            .unwrap()
            .progress_chars("=>-"),
    );

    // Step 1: Create directory structure
    pb.set_message("Creating directory structure...");
    create_directory_structure(&base_dir)?;
    pb.inc(1);

    // Step 2: Initialize SQLite database
    pb.set_message("Initializing SQLite database...");
    initialize_database(&base_dir).await?;
    pb.inc(1);

    // Step 3: Create default config
    pb.set_message("Creating default configuration...");
    create_default_config(&base_dir, project_name)?;
    pb.inc(1);

    // Step 4: Set up thoughts directory
    pb.set_message("Setting up thoughts directory...");
    setup_thoughts_directory(&base_dir)?;
    pb.inc(1);

    // Step 5: Create example files
    pb.set_message("Creating example files...");
    create_example_files(&base_dir)?;
    pb.inc(1);

    pb.finish_with_message("Initialization complete!");

    println!("\n{}", "Project initialized successfully!".green().bold());
    println!("\nNext steps:");
    println!(
        "  1. Edit the configuration: {}",
        format!("{}/config.toml", base_dir.display()).cyan()
    );
    println!("  2. Add your API keys to the config or set environment variables");
    println!(
        "  3. Run: {} to spawn your first agent",
        "descartes spawn --task \"your task\"".yellow()
    );
    println!("\nFor more information, run: {}", "descartes --help".cyan());

    Ok(())
}

fn create_directory_structure(base_dir: &Path) -> Result<()> {
    let dirs = vec![
        base_dir.to_path_buf(),
        base_dir.join("data"),
        base_dir.join("data/state"),
        base_dir.join("data/events"),
        base_dir.join("data/cache"),
        base_dir.join("thoughts"),
        base_dir.join("logs"),
        base_dir.join("backups"),
    ];

    for dir in dirs {
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
            info!("Created directory: {:?}", dir);
        } else {
            warn!("Directory already exists: {:?}", dir);
        }
    }

    Ok(())
}

async fn initialize_database(base_dir: &Path) -> Result<()> {
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePool};

    let db_path = base_dir.join("data/descartes.db");
    let db_url = format!("sqlite://{}", db_path.display());

    info!("Creating SQLite database at: {}", db_url);

    // Create database options
    let options = SqliteConnectOptions::new()
        .filename(&db_path)
        .create_if_missing(true);

    // Create database pool
    let pool = SqlitePool::connect_with(options).await?;

    // Run migrations - create basic tables
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
    .await?;

    info!("Database initialized successfully");
    Ok(())
}

fn create_default_config(base_dir: &Path, project_name: &str) -> Result<()> {
    let config_path = base_dir.join("config.toml");

    if config_path.exists() {
        warn!("Config file already exists: {:?}", config_path);
        return Ok(());
    }

    // Create default config
    let mut config = DescaratesConfig::default();

    // Customize with project-specific settings
    config.storage.base_path = base_dir.display().to_string();

    // Create config manager and save
    let mut manager = ConfigManager::load(Some(&config_path))
        .unwrap_or_else(|_| ConfigManager::load(None).expect("Failed to create config manager"));

    // Load from environment
    manager.load_from_env()?;

    // Save to file
    manager.save()?;

    info!("Created default config at: {:?}", config_path);
    Ok(())
}

fn setup_thoughts_directory(base_dir: &Path) -> Result<()> {
    let thoughts_dir = base_dir.join("thoughts");

    // Create subdirectories for thought organization
    let subdirs = vec!["sessions", "archived", "templates"];

    for subdir in subdirs {
        let path = thoughts_dir.join(subdir);
        fs::create_dir_all(&path)?;
    }

    // Create a README
    let readme = thoughts_dir.join("README.md");
    fs::write(
        readme,
        r#"# Thoughts Directory

This directory stores agent thoughts and reasoning chains.

## Structure

- `sessions/` - Active thought sessions
- `archived/` - Archived thought sessions
- `templates/` - Thought templates for common tasks

## Usage

Thoughts are automatically saved here during agent execution.
You can review them to understand agent reasoning and decision-making.
"#,
    )?;

    Ok(())
}

fn create_example_files(base_dir: &Path) -> Result<()> {
    // Create example system prompt
    let example_prompt = base_dir.join("example_system_prompt.txt");
    if !example_prompt.exists() {
        fs::write(
            &example_prompt,
            r#"You are a helpful AI assistant specialized in software development.
Your role is to help users with coding tasks, debugging, and architectural decisions.

Guidelines:
- Be concise and clear
- Provide working code examples
- Explain your reasoning
- Ask clarifying questions when needed
"#,
        )?;
    }

    // Create .gitignore
    let gitignore = base_dir.join(".gitignore");
    if !gitignore.exists() {
        fs::write(
            &gitignore,
            r#"# Descartes
data/
logs/
backups/
*.db
*.db-shm
*.db-wal

# Secrets
config.toml
secrets/

# Cache
cache/
*.cache
"#,
        )?;
    }

    Ok(())
}
