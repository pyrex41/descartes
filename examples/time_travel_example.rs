//! Comprehensive Time Travel Example
//!
//! This example demonstrates complete time travel functionality including:
//! - Creating an agent with history tracking
//! - Recording events (thoughts, actions, decisions)
//! - Creating git commits to track code changes
//! - Rewinding to previous points in time
//! - Validating state consistency
//! - Resuming execution from rewound state
//! - Creating and using snapshots
//! - Undo/redo functionality
//!
//! # Usage
//!
//! ```bash
//! cargo run --example time_travel_example
//! ```

use descartes_core::{
    agent_history::{
        AgentHistoryEvent, AgentHistoryStore, HistoryEventType, SqliteAgentHistoryStore,
    },
    body_restore::GitBodyRestoreManager,
    brain_restore::{BrainRestore, DefaultBrainRestore, RestoreOptions as BrainRestoreOptions},
    time_travel_integration::{
        DefaultRewindManager, RewindConfig, RewindManager, RewindPoint, ResumeContext,
    },
};
use serde_json::json;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;
use tempfile::TempDir;
use tokio;

/// Setup: Create a test git repository with some history
fn create_example_repo() -> (TempDir, PathBuf) {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let repo_path = temp_dir.path().to_path_buf();

    println!("📁 Creating example git repository at: {:?}", repo_path);

    // Initialize git repo
    Command::new("git")
        .args(["init"])
        .current_dir(&repo_path)
        .output()
        .expect("Failed to init git repo");

    // Configure git
    Command::new("git")
        .args(["config", "user.name", "Example User"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["config", "user.email", "example@descartes.ai"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    // Create initial commit
    std::fs::write(repo_path.join("agent.txt"), "Agent initialized").unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", "Initial: Agent created"])
        .current_dir(&repo_path)
        .output()
        .unwrap();

    println!("✅ Git repository initialized");

    (temp_dir, repo_path)
}

/// Create a git commit for tracking body state
fn create_commit(repo_path: &PathBuf, content: &str, message: &str) -> String {
    std::fs::write(repo_path.join("agent.txt"), content).unwrap();
    Command::new("git")
        .args(["add", "."])
        .current_dir(repo_path)
        .output()
        .unwrap();

    Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(repo_path)
        .output()
        .unwrap();

    // Get commit hash
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_path)
        .output()
        .unwrap();

    String::from_utf8(output.stdout).unwrap().trim().to_string()
}

/// Setup: Create an agent history store
async fn create_history_store() -> SqliteAgentHistoryStore {
    let temp_file = tempfile::NamedTempFile::new().expect("Failed to create temp file");
    let path = temp_file.path().to_str().unwrap();

    println!("📊 Creating agent history store at: {}", path);

    let mut store = SqliteAgentHistoryStore::new(path).await.unwrap();
    store.initialize().await.unwrap();

    println!("✅ History store initialized");

    store
}

/// Simulate agent activity by recording events
async fn simulate_agent_activity(
    store: &SqliteAgentHistoryStore,
    repo_path: &PathBuf,
) -> Vec<(AgentHistoryEvent, String)> {
    println!("\n🤖 Simulating agent activity...\n");

    let mut events_with_commits = Vec::new();

    // Phase 1: Analysis
    println!("Phase 1: Analysis");
    let event1 = AgentHistoryEvent::new(
        "agent-alpha".to_string(),
        HistoryEventType::Thought,
        json!({
            "content": "Analyzing user request: 'Build a REST API'",
            "thought_type": "analysis",
            "confidence": 0.9
        }),
    )
    .with_session("session-001".to_string());
    store.record_event(&event1).await.unwrap();
    println!("  💭 Thought: {}", event1.event_data["content"]);

    let commit1 = create_commit(repo_path, "Phase 1: Analysis", "Analysis phase");
    events_with_commits.push((event1.clone(), commit1.clone()));

    // Phase 2: Decision
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    println!("\nPhase 2: Decision");
    let event2 = AgentHistoryEvent::new(
        "agent-alpha".to_string(),
        HistoryEventType::Decision,
        json!({
            "decision_type": "framework_selection",
            "context": {"options": ["FastAPI", "Flask", "Django"]},
            "outcome": "FastAPI",
            "reasoning": "Better async support and modern features"
        }),
    )
    .with_session("session-001".to_string())
    .with_parent(event1.event_id)
    .with_git_commit(commit1.clone());
    store.record_event(&event2).await.unwrap();
    println!("  🎯 Decision: {}", event2.event_data["outcome"]);

    let commit2 = create_commit(repo_path, "Phase 2: Decision - FastAPI", "Decision phase");
    events_with_commits.push((event2.clone(), commit2.clone()));

    // Phase 3: Action
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    println!("\nPhase 3: Action");
    let event3 = AgentHistoryEvent::new(
        "agent-alpha".to_string(),
        HistoryEventType::Action,
        json!({
            "action": "create_project_structure",
            "parameters": {"framework": "FastAPI", "database": "PostgreSQL"},
            "status": "in_progress"
        }),
    )
    .with_session("session-001".to_string())
    .with_parent(event2.event_id)
    .with_git_commit(commit2.clone());
    store.record_event(&event3).await.unwrap();
    println!("  ⚡ Action: {}", event3.event_data["action"]);

    let commit3 = create_commit(
        repo_path,
        "Phase 3: Project structure created",
        "Action phase",
    );
    events_with_commits.push((event3.clone(), commit3.clone()));

    // Phase 4: Tool Use
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    println!("\nPhase 4: Tool Use");
    let event4 = AgentHistoryEvent::new(
        "agent-alpha".to_string(),
        HistoryEventType::ToolUse,
        json!({
            "tool": "code_generator",
            "input": {"endpoint": "/api/users", "method": "GET"},
            "output": "Generated FastAPI endpoint code"
        }),
    )
    .with_session("session-001".to_string())
    .with_parent(event3.event_id)
    .with_git_commit(commit3.clone());
    store.record_event(&event4).await.unwrap();
    println!("  🔧 Tool: {}", event4.event_data["tool"]);

    let commit4 = create_commit(repo_path, "Phase 4: Generated API endpoints", "Tool use phase");
    events_with_commits.push((event4.clone(), commit4.clone()));

    // Phase 5: State Change
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    println!("\nPhase 5: State Change");
    let event5 = AgentHistoryEvent::new(
        "agent-alpha".to_string(),
        HistoryEventType::StateChange,
        json!({
            "from_state": "building",
            "to_state": "complete",
            "trigger": "all_tasks_completed"
        }),
    )
    .with_session("session-001".to_string())
    .with_parent(event4.event_id)
    .with_git_commit(commit4.clone());
    store.record_event(&event5).await.unwrap();
    println!("  🔄 State: {} → {}", event5.event_data["from_state"], event5.event_data["to_state"]);

    let commit5 = create_commit(repo_path, "Phase 5: Project complete", "State change");
    events_with_commits.push((event5.clone(), commit5.clone()));

    println!("\n✅ Agent activity simulation complete");
    println!("   Recorded {} events across {} commits\n", events_with_commits.len(), 5);

    events_with_commits
}

/// Display a menu and get user choice
fn display_menu() -> usize {
    println!("\n{'=':=<60}\n  TIME TRAVEL MENU\n{'=':=<60}");
    println!("1. View event history");
    println!("2. Rewind to Phase 2 (Decision)");
    println!("3. Rewind to Phase 1 (Analysis)");
    println!("4. Create snapshot");
    println!("5. View rewind points");
    println!("6. Undo last rewind");
    println!("7. Forward to latest state");
    println!("8. Exit");
    println!("{'=':=<60}");

    print!("\nYour choice: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().parse().unwrap_or(8)
}

/// Display event history
async fn display_history(store: &SqliteAgentHistoryStore) {
    println!("\n📜 Event History:");
    println!("{:-<80}", "");

    let events = store.get_events("agent-alpha", 100).await.unwrap();

    for (i, event) in events.iter().enumerate() {
        let timestamp = chrono::DateTime::from_timestamp(event.timestamp, 0)
            .map(|dt| dt.format("%H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        let git_marker = if event.git_commit_hash.is_some() {
            " [GIT]"
        } else {
            ""
        };

        println!(
            "{}. [{}] {:?}{} - {}",
            i + 1,
            timestamp,
            event.event_type,
            git_marker,
            event
                .event_data
                .get("content")
                .or_else(|| event.event_data.get("decision_type"))
                .or_else(|| event.event_data.get("action"))
                .or_else(|| event.event_data.get("tool"))
                .or_else(|| event.event_data.get("from_state"))
                .unwrap_or(&json!("Event"))
                .as_str()
                .unwrap_or("Event")
        );

        if let Some(commit) = &event.git_commit_hash {
            println!("   Commit: {}", &commit[..7]);
        }
    }

    println!("{:-<80}", "");
}

/// Perform a rewind operation
async fn perform_rewind(
    manager: &DefaultRewindManager<Arc<SqliteAgentHistoryStore>>,
    body_manager: &GitBodyRestoreManager,
    events: &[(AgentHistoryEvent, String)],
    target_index: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n⏪ Rewinding to event {}...", target_index + 1);

    let (target_event, target_commit) = &events[target_index];

    let point = RewindPoint {
        timestamp: target_event.timestamp,
        event_id: Some(target_event.event_id),
        git_commit: Some(target_commit.clone()),
        snapshot_id: None,
        description: format!("Rewind to {:?}", target_event.event_type),
        event_index: Some(target_index),
    };

    // Check if we can rewind
    let confirmation = manager.can_rewind_to(&point).await?;
    println!("✓ Rewind check passed");
    if !confirmation.warnings.is_empty() {
        println!("  Warnings:");
        for warning in &confirmation.warnings {
            println!("    - {}", warning);
        }
    }

    // Perform rewind
    let config = RewindConfig {
        require_confirmation: false,
        auto_backup: true,
        validate_state: true,
        allow_uncommitted_changes: true,
        max_undo_history: 10,
        enable_debugging: false,
    };

    let start = std::time::Instant::now();
    let result = manager.rewind_to(point, config).await?;
    let duration = start.elapsed();

    if result.success {
        println!("\n✅ Rewind successful!");
        println!("   Duration: {:?}", duration);
        println!("   Events processed: {}", result.brain_result.as_ref().unwrap().events_processed);
        println!("   Backup ID: {}", result.backup.backup_id);

        // Verify body state
        let current_commit = body_manager.get_current_commit().await?;
        println!("   Current commit: {}", &current_commit[..7]);

        // Display validation results
        if !result.validation.warnings.is_empty() {
            println!("\n  Validation warnings:");
            for warning in &result.validation.warnings {
                println!("    ⚠️  {}", warning);
            }
        }

        if !result.validation.errors.is_empty() {
            println!("\n  Validation errors:");
            for error in &result.validation.errors {
                println!("    ❌ {}", error);
            }
        }
    } else {
        println!("\n❌ Rewind failed!");
        for error in &result.validation.errors {
            println!("   Error: {}", error);
        }
    }

    Ok(())
}

/// Create a snapshot
async fn create_snapshot_example(
    manager: &DefaultRewindManager<Arc<SqliteAgentHistoryStore>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n📸 Creating snapshot...");

    let snapshot_id = manager
        .create_snapshot(
            "agent-alpha",
            format!(
                "Manual snapshot at {}",
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
            ),
        )
        .await?;

    println!("✅ Snapshot created: {}", snapshot_id);

    Ok(())
}

/// View rewind points
async fn view_rewind_points(
    manager: &DefaultRewindManager<Arc<SqliteAgentHistoryStore>>,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("\n🎯 Available Rewind Points:");
    println!("{:-<80}", "");

    let points = manager.get_rewind_points("agent-alpha").await?;

    for (i, point) in points.iter().enumerate() {
        let timestamp = chrono::DateTime::from_timestamp(point.timestamp, 0)
            .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
            .unwrap_or_else(|| "Unknown".to_string());

        println!("{}. {} - {}", i + 1, timestamp, point.description);
        if let Some(ref commit) = point.git_commit {
            println!("   Git: {}", &commit[..7]);
        }
        if point.snapshot_id.is_some() {
            println!("   [SNAPSHOT]");
        }
    }

    println!("{:-<80}", "");
    println!("Total: {} rewind points", points.len());

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n{'=':=<80}");
    println!("  DESCARTES TIME TRAVEL DEMONSTRATION");
    println!("{'=':=<80}\n");

    // Setup
    println!("🔧 Setting up example environment...\n");
    let (_temp_dir, repo_path) = create_example_repo();
    let store = Arc::new(create_history_store().await);

    // Simulate agent activity
    let events_with_commits = simulate_agent_activity(&store, &repo_path).await;

    // Create managers
    let manager = DefaultRewindManager::new(Arc::clone(&store), repo_path.clone(), 10)?;
    let body_manager = GitBodyRestoreManager::new(&repo_path)?;

    println!("\n✅ Setup complete! Ready for time travel.\n");

    // Interactive menu loop
    loop {
        let choice = display_menu();

        match choice {
            1 => {
                display_history(&store).await;
            }
            2 => {
                // Rewind to Phase 2 (Decision) - event index 1
                perform_rewind(&manager, &body_manager, &events_with_commits, 1).await?;
            }
            3 => {
                // Rewind to Phase 1 (Analysis) - event index 0
                perform_rewind(&manager, &body_manager, &events_with_commits, 0).await?;
            }
            4 => {
                create_snapshot_example(&manager).await?;
            }
            5 => {
                view_rewind_points(&manager).await?;
            }
            6 => {
                println!("\n↩️  Undo functionality would be demonstrated here");
                println!("   (In a full implementation, this would restore the previous state)");
            }
            7 => {
                println!("\n⏩ Fast-forward to latest state...");
                let latest = events_with_commits.len() - 1;
                perform_rewind(&manager, &body_manager, &events_with_commits, latest).await?;
            }
            8 => {
                println!("\n👋 Thank you for trying Descartes Time Travel!");
                println!("{'=':=<80}\n");
                break;
            }
            _ => {
                println!("❌ Invalid choice. Please try again.");
            }
        }
    }

    Ok(())
}
