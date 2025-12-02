/// Task management commands for Descartes CLI
/// Uses SCG-based storage for task operations
use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use descartes_core::{
    ScgTaskQueries, ScgTaskQueryBuilder, ScgTaskStorage, TaskPriority, TaskStatus,
};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Subcommand)]
pub enum TaskCommands {
    /// List tasks in the active phase
    List {
        /// Filter by status (pending, in-progress, done, blocked)
        #[arg(short, long)]
        status: Option<String>,

        /// Filter by priority (low, medium, high, critical)
        #[arg(short, long)]
        priority: Option<String>,

        /// Search in title and description
        #[arg(long)]
        search: Option<String>,

        /// Output format (table, json, scg)
        #[arg(short, long, default_value = "table")]
        format: String,

        /// Limit number of results
        #[arg(short, long, default_value = "50")]
        limit: usize,
    },

    /// Show details of a specific task
    Show {
        /// Task ID
        id: String,

        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Show the next available task (ready to work on)
    Next {
        /// Only output the task ID
        #[arg(long)]
        id_only: bool,
    },

    /// Show task statistics for the active phase
    Stats {
        /// Output format (text, json)
        #[arg(short, long, default_value = "text")]
        format: String,
    },

    /// Set the active phase/epic
    Use {
        /// Phase/epic tag to activate
        tag: String,
    },

    /// List available phases/epics
    Phases {
        /// Output format (table, json)
        #[arg(short, long, default_value = "table")]
        format: String,
    },
}

/// Execute a task command
pub async fn execute(cmd: &TaskCommands, project_root: Option<PathBuf>) -> Result<()> {
    // Determine project root (current directory or specified)
    let root = project_root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    // Create SCG storage
    let storage = Arc::new(ScgTaskStorage::new(&root));

    // Refresh cache from disk
    if let Err(e) = storage.refresh_cache().await {
        println!(
            "{} {}",
            "Warning:".yellow(),
            format!("Could not load tasks: {}", e).dimmed()
        );
        println!(
            "{}",
            "Run 'scud init' or ensure .taskmaster/ directory exists.".dimmed()
        );
        return Ok(());
    }

    match cmd {
        TaskCommands::List {
            status,
            priority,
            search,
            format,
            limit,
        } => {
            list_tasks(&storage, status.as_deref(), priority.as_deref(), search.as_deref(), format, *limit).await
        }
        TaskCommands::Show { id, format } => show_task(&storage, id, format).await,
        TaskCommands::Next { id_only } => next_task(&storage, *id_only).await,
        TaskCommands::Stats { format } => show_stats(&storage, format).await,
        TaskCommands::Use { tag } => use_phase(&storage, tag).await,
        TaskCommands::Phases { format } => list_phases(&storage, format).await,
    }
}

/// List tasks with optional filters
async fn list_tasks(
    storage: &Arc<ScgTaskStorage>,
    status_filter: Option<&str>,
    priority_filter: Option<&str>,
    search_term: Option<&str>,
    format: &str,
    limit: usize,
) -> Result<()> {
    let tasks = storage.get_active_phase_tasks().await?;

    if tasks.is_empty() {
        println!("{}", "No tasks found in active phase.".yellow());
        return Ok(());
    }

    // Build query
    let mut query = ScgTaskQueryBuilder::new().limit(limit);

    if let Some(status_str) = status_filter {
        if let Some(status) = parse_status(status_str) {
            query = query.with_status(status);
        }
    }

    if let Some(priority_str) = priority_filter {
        if let Some(priority) = parse_priority(priority_str) {
            query = query.with_priority(priority);
        }
    }

    if let Some(term) = search_term {
        query = query.search(term.to_string());
    }

    let filtered = query.execute(&tasks);

    match format {
        "json" => print_tasks_json(&filtered)?,
        "scg" => print_tasks_scg(&filtered)?,
        "table" | _ => print_tasks_table(&filtered)?,
    }

    Ok(())
}

fn print_tasks_table(tasks: &[descartes_core::Task]) -> Result<()> {
    println!("\n{}", "Tasks".green().bold());
    println!("{}", "─".repeat(100).dimmed());

    // Header
    println!(
        "{:<38} {:<30} {:<12} {:<10} {:<8}",
        "ID".bold(),
        "TITLE".bold(),
        "STATUS".bold(),
        "PRIORITY".bold(),
        "CMPLX".bold()
    );
    println!("{}", "─".repeat(100).dimmed());

    for task in tasks {
        let id = task.id.to_string();
        let title = if task.title.len() > 28 {
            format!("{}...", &task.title[..25])
        } else {
            task.title.clone()
        };

        let status_str = format!("{:?}", task.status);
        let status_colored = match task.status {
            TaskStatus::Todo => status_str.white(),
            TaskStatus::InProgress => status_str.cyan(),
            TaskStatus::Done => status_str.green(),
            TaskStatus::Blocked => status_str.red(),
        };

        let priority_str = format!("{:?}", task.priority);
        let priority_colored = match task.priority {
            TaskPriority::Critical => priority_str.red().bold(),
            TaskPriority::High => priority_str.yellow(),
            TaskPriority::Medium => priority_str.white(),
            TaskPriority::Low => priority_str.dimmed(),
        };

        let complexity: u32 = task.complexity.into();

        println!(
            "{:<38} {:<30} {:<12} {:<10} {:<8}",
            id.cyan(),
            title,
            status_colored.to_string(),
            priority_colored.to_string(),
            complexity.to_string().dimmed()
        );
    }

    println!("{}", "─".repeat(100).dimmed());
    println!("\nTotal: {}", tasks.len().to_string().cyan());

    Ok(())
}

fn print_tasks_json(tasks: &[descartes_core::Task]) -> Result<()> {
    let json_tasks: Vec<_> = tasks
        .iter()
        .map(|t| {
            let complexity: u32 = t.complexity.into();
            json!({
                "id": t.id.to_string(),
                "title": t.title,
                "description": t.description,
                "status": format!("{:?}", t.status).to_lowercase(),
                "priority": format!("{:?}", t.priority).to_lowercase(),
                "complexity": complexity,
                "assigned_to": t.assigned_to,
                "dependencies": t.dependencies.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&json_tasks)?);
    Ok(())
}

fn print_tasks_scg(tasks: &[descartes_core::Task]) -> Result<()> {
    // SCG-style output
    println!("# id | title | status | complexity | priority");
    for task in tasks {
        let status_code = match task.status {
            TaskStatus::Todo => "P",
            TaskStatus::InProgress => "I",
            TaskStatus::Done => "D",
            TaskStatus::Blocked => "B",
        };
        let priority_code = match task.priority {
            TaskPriority::Low => "L",
            TaskPriority::Medium => "M",
            TaskPriority::High => "H",
            TaskPriority::Critical => "C",
        };
        let complexity: u32 = task.complexity.into();

        println!(
            "{} | {} | {} | {} | {}",
            task.id, task.title, status_code, complexity, priority_code
        );
    }
    Ok(())
}

/// Show details of a specific task
async fn show_task(storage: &Arc<ScgTaskStorage>, id: &str, format: &str) -> Result<()> {
    let tasks = storage.get_active_phase_tasks().await?;

    // Find task by ID (support both UUID and short ID matching)
    let task = tasks.iter().find(|t| {
        let task_id = t.id.to_string();
        task_id == id || task_id.starts_with(id)
    });

    match task {
        Some(t) => {
            let complexity: u32 = t.complexity.into();
            match format {
                "json" => {
                    let json_task = json!({
                        "id": t.id.to_string(),
                        "title": t.title,
                        "description": t.description,
                        "status": format!("{:?}", t.status).to_lowercase(),
                        "priority": format!("{:?}", t.priority).to_lowercase(),
                        "complexity": complexity,
                        "assigned_to": t.assigned_to,
                        "dependencies": t.dependencies.iter().map(|d| d.to_string()).collect::<Vec<_>>(),
                        "created_at": t.created_at,
                        "updated_at": t.updated_at,
                    });
                    println!("{}", serde_json::to_string_pretty(&json_task)?);
                }
                "text" | _ => {
                    println!("\n{}", "Task Details".green().bold());
                    println!("{}", "─".repeat(60).dimmed());
                    println!("{:<15} {}", "ID:".bold(), t.id.to_string().cyan());
                    println!("{:<15} {}", "Title:".bold(), t.title);
                    println!(
                        "{:<15} {}",
                        "Status:".bold(),
                        format!("{:?}", t.status)
                    );
                    println!(
                        "{:<15} {}",
                        "Priority:".bold(),
                        format!("{:?}", t.priority)
                    );
                    println!("{:<15} {}", "Complexity:".bold(), complexity);

                    if let Some(desc) = &t.description {
                        println!("\n{}", "Description:".bold());
                        println!("{}", desc.dimmed());
                    }

                    if let Some(assignee) = &t.assigned_to {
                        println!("\n{:<15} {}", "Assigned to:".bold(), assignee);
                    }

                    if !t.dependencies.is_empty() {
                        println!("\n{}", "Dependencies:".bold());
                        for dep in &t.dependencies {
                            println!("  - {}", dep);
                        }
                    }

                    println!("{}", "─".repeat(60).dimmed());
                }
            }
        }
        None => {
            println!("{}", format!("Task '{}' not found.", id).red());
        }
    }

    Ok(())
}

/// Show the next available task
async fn next_task(storage: &Arc<ScgTaskStorage>, id_only: bool) -> Result<()> {
    match storage.get_next_task().await? {
        Some(task) => {
            if id_only {
                println!("{}", task.id);
            } else {
                println!("\n{}", "Next Available Task".green().bold());
                println!("{}", "─".repeat(60).dimmed());
                println!("{:<15} {}", "ID:".bold(), task.id.to_string().cyan());
                println!("{:<15} {}", "Title:".bold(), task.title);
                println!(
                    "{:<15} {}",
                    "Priority:".bold(),
                    format!("{:?}", task.priority)
                );
                let complexity: u32 = task.complexity.into();
                println!("{:<15} {}", "Complexity:".bold(), complexity);

                if let Some(desc) = &task.description {
                    println!("\n{}", "Description:".bold());
                    let preview = if desc.len() > 200 {
                        format!("{}...", &desc[..200])
                    } else {
                        desc.clone()
                    };
                    println!("{}", preview.dimmed());
                }
                println!("{}", "─".repeat(60).dimmed());
            }
        }
        None => {
            if id_only {
                // Exit silently with no output
            } else {
                println!("{}", "No tasks ready to work on.".yellow());
                println!(
                    "{}",
                    "All pending tasks may have unmet dependencies or are blocked.".dimmed()
                );
            }
        }
    }

    Ok(())
}

/// Show task statistics
async fn show_stats(storage: &Arc<ScgTaskStorage>, format: &str) -> Result<()> {
    let active_tag = storage.get_active_phase_tag().await?;

    match active_tag {
        Some(tag) => {
            if let Some(stats) = storage.get_phase_stats(&tag).await? {
                match format {
                    "json" => {
                        let json_stats = json!({
                            "phase": stats.name,
                            "total": stats.total,
                            "pending": stats.pending,
                            "in_progress": stats.in_progress,
                            "done": stats.done,
                            "blocked": stats.blocked,
                            "total_complexity": stats.total_complexity,
                            "completion_percent": if stats.total > 0 {
                                (stats.done as f64 / stats.total as f64 * 100.0).round()
                            } else {
                                0.0
                            }
                        });
                        println!("{}", serde_json::to_string_pretty(&json_stats)?);
                    }
                    "text" | _ => {
                        let completion = if stats.total > 0 {
                            (stats.done as f64 / stats.total as f64 * 100.0).round() as usize
                        } else {
                            0
                        };

                        println!("\n{} {}", "Phase:".bold(), stats.name.cyan());
                        println!("{}", "─".repeat(40).dimmed());

                        // Progress bar
                        let bar_width = 30;
                        let filled = (completion * bar_width) / 100;
                        let empty = bar_width - filled;
                        let progress_bar = format!(
                            "[{}{}] {}%",
                            "█".repeat(filled).green(),
                            "░".repeat(empty).dimmed(),
                            completion
                        );
                        println!("{}", progress_bar);

                        println!();
                        println!("{:<15} {}", "Total:".bold(), stats.total);
                        println!(
                            "{:<15} {}",
                            "Pending:".bold(),
                            stats.pending.to_string().white()
                        );
                        println!(
                            "{:<15} {}",
                            "In Progress:".bold(),
                            stats.in_progress.to_string().cyan()
                        );
                        println!(
                            "{:<15} {}",
                            "Done:".bold(),
                            stats.done.to_string().green()
                        );
                        println!(
                            "{:<15} {}",
                            "Blocked:".bold(),
                            stats.blocked.to_string().red()
                        );
                        println!();
                        println!(
                            "{:<15} {}",
                            "Complexity:".bold(),
                            stats.total_complexity.to_string().dimmed()
                        );
                        println!("{}", "─".repeat(40).dimmed());
                    }
                }
            }
        }
        None => {
            println!("{}", "No active phase set.".yellow());
            println!(
                "{}",
                "Use 'descartes tasks use <tag>' to set active phase.".dimmed()
            );
        }
    }

    Ok(())
}

/// Set the active phase
async fn use_phase(storage: &Arc<ScgTaskStorage>, tag: &str) -> Result<()> {
    match storage.set_active_phase(tag).await {
        Ok(()) => {
            println!("{} {}", "Activated phase:".green(), tag.cyan());
        }
        Err(e) => {
            println!("{} {}", "Failed to set phase:".red(), e);
        }
    }
    Ok(())
}

/// List available phases
async fn list_phases(storage: &Arc<ScgTaskStorage>, format: &str) -> Result<()> {
    let phases = storage.get_phases().await?;
    let active = storage.get_active_phase_tag().await?.unwrap_or_default();

    if phases.is_empty() {
        println!("{}", "No phases found.".yellow());
        return Ok(());
    }

    match format {
        "json" => {
            let json_phases: Vec<_> = phases
                .iter()
                .map(|(tag, phase)| {
                    let stats = phase.get_stats();
                    json!({
                        "tag": tag,
                        "name": phase.name,
                        "active": tag == &active,
                        "total_tasks": stats.total,
                        "done": stats.done,
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&json_phases)?);
        }
        "table" | _ => {
            println!("\n{}", "Available Phases".green().bold());
            println!("{}", "─".repeat(60).dimmed());

            println!(
                "{:<3} {:<20} {:<10} {:<10}",
                "".bold(),
                "TAG".bold(),
                "TASKS".bold(),
                "DONE".bold()
            );
            println!("{}", "─".repeat(60).dimmed());

            for (tag, phase) in &phases {
                let stats = phase.get_stats();
                let marker = if tag == &active { "►" } else { " " };
                let tag_display = if tag == &active {
                    tag.cyan().bold().to_string()
                } else {
                    tag.to_string()
                };

                println!(
                    "{:<3} {:<20} {:<10} {:<10}",
                    marker.green(),
                    tag_display,
                    stats.total,
                    format!("{}/{}", stats.done, stats.total).dimmed()
                );
            }

            println!("{}", "─".repeat(60).dimmed());
        }
    }

    Ok(())
}

/// Parse status string to TaskStatus
fn parse_status(s: &str) -> Option<TaskStatus> {
    match s.to_lowercase().as_str() {
        "pending" | "todo" | "p" => Some(TaskStatus::Todo),
        "in-progress" | "inprogress" | "progress" | "i" => Some(TaskStatus::InProgress),
        "done" | "completed" | "d" => Some(TaskStatus::Done),
        "blocked" | "b" => Some(TaskStatus::Blocked),
        _ => None,
    }
}

/// Parse priority string to TaskPriority
fn parse_priority(s: &str) -> Option<TaskPriority> {
    match s.to_lowercase().as_str() {
        "low" | "l" => Some(TaskPriority::Low),
        "medium" | "m" => Some(TaskPriority::Medium),
        "high" | "h" => Some(TaskPriority::High),
        "critical" | "c" => Some(TaskPriority::Critical),
        _ => None,
    }
}
