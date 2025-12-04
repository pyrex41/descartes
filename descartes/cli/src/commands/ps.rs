use anyhow::Result;
use chrono::{DateTime, Local};
use colored::Colorize;
use descartes_core::DescaratesConfig;
use serde_json::json;
use std::time::SystemTime;

pub async fn execute(config: &DescaratesConfig, format: &str, show_all: bool) -> Result<()> {
    // Connect to database
    let db_path = format!("{}/data/descartes.db", config.storage.base_path);
    let db_url = format!("sqlite://{}", db_path);

    let pool = sqlx::sqlite::SqlitePool::connect(&db_url).await?;

    // Query agents
    let query = if show_all {
        "SELECT id, name, status, model_backend, started_at, completed_at, task FROM agents ORDER BY started_at DESC"
    } else {
        "SELECT id, name, status, model_backend, started_at, completed_at, task FROM agents WHERE status IN ('Running', 'Idle', 'Paused') ORDER BY started_at DESC"
    };

    let rows = sqlx::query(query).fetch_all(&pool).await?;

    if rows.is_empty() {
        println!("{}", "No agents found.".yellow());
        return Ok(());
    }

    match format {
        "json" => print_json(&rows)?,
        "table" | _ => print_table(&rows)?,
    }

    Ok(())
}

fn print_table(rows: &[sqlx::sqlite::SqliteRow]) -> Result<()> {
    use sqlx::Row;

    println!("\n{}", "Running Agents".green().bold());
    println!("{}", "─".repeat(120).dimmed());

    // Header
    println!(
        "{:<36} {:<20} {:<12} {:<15} {:<20} {:<15}",
        "ID".bold(),
        "NAME".bold(),
        "STATUS".bold(),
        "PROVIDER".bold(),
        "STARTED".bold(),
        "RUNTIME".bold()
    );
    println!("{}", "─".repeat(120).dimmed());

    // Rows
    for row in rows {
        let id: String = row.get("id");
        let name: String = row.get("name");
        let status: String = row.get("status");
        let backend: String = row.get("model_backend");
        let started_at: i64 = row.get("started_at");
        let completed_at: Option<i64> = row.get("completed_at");

        let started = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(started_at as u64);
        let started_str = format_time(started);

        let runtime = if let Some(completed) = completed_at {
            let end = SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(completed as u64);
            format_duration(started, end)
        } else {
            format_duration(started, SystemTime::now())
        };

        let status_colored = match status.as_str() {
            "Running" => status.green(),
            "Idle" => status.yellow(),
            "Completed" => status.blue(),
            "Failed" => status.red(),
            "Terminated" => status.red().dimmed(),
            _ => status.white(),
        };

        println!(
            "{:<36} {:<20} {:<12} {:<15} {:<20} {:<15}",
            id.cyan(),
            name,
            status_colored.to_string(),
            backend.yellow(),
            started_str.dimmed(),
            runtime
        );
    }

    println!("{}", "─".repeat(120).dimmed());
    println!("\nTotal: {}", rows.len().to_string().cyan());

    Ok(())
}

fn print_json(rows: &[sqlx::sqlite::SqliteRow]) -> Result<()> {
    use sqlx::Row;

    let agents: Vec<_> = rows
        .iter()
        .map(|row| {
            json!({
                "id": row.get::<String, _>("id"),
                "name": row.get::<String, _>("name"),
                "status": row.get::<String, _>("status"),
                "model_backend": row.get::<String, _>("model_backend"),
                "started_at": row.get::<i64, _>("started_at"),
                "completed_at": row.get::<Option<i64>, _>("completed_at"),
                "task": row.get::<String, _>("task"),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&agents)?);
    Ok(())
}

fn format_time(time: SystemTime) -> String {
    let datetime: DateTime<Local> = time.into();
    datetime.format("%Y-%m-%d %H:%M:%S").to_string()
}

fn format_duration(start: SystemTime, end: SystemTime) -> String {
    if let Ok(duration) = end.duration_since(start) {
        let secs = duration.as_secs();
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, mins, secs)
        } else if mins > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}s", secs)
        }
    } else {
        "N/A".to_string()
    }
}
