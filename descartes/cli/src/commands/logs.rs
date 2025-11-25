use anyhow::Result;
use chrono::{DateTime, Local};
use colored::Colorize;
use descartes_core::{ActorType, DescaratesConfig, Event};
use serde_json::json;
use std::time::{Duration, SystemTime};
use tracing::info;

pub async fn execute(
    config: &DescaratesConfig,
    agent_id: Option<&str>,
    follow: bool,
    event_type: Option<&str>,
    limit: usize,
    format: &str,
) -> Result<()> {
    // Connect to database
    let db_path = format!("{}/data/descartes.db", config.storage.base_path);
    let db_url = format!("sqlite://{}", db_path);

    let pool = sqlx::sqlite::SqlitePool::connect(&db_url).await?;

    // Build query
    let mut query = String::from("SELECT * FROM events WHERE 1=1");
    let mut params: Vec<String> = Vec::new();

    if let Some(id) = agent_id {
        query.push_str(" AND session_id = ?");
        params.push(id.to_string());
    }

    if let Some(etype) = event_type {
        query.push_str(&format!(" AND event_type = ?"));
        params.push(etype.to_string());
    }

    query.push_str(" ORDER BY timestamp DESC");

    if !follow {
        query.push_str(&format!(" LIMIT {}", limit));
    }

    if follow {
        println!("{}", "Following logs (Ctrl+C to stop)...".green().bold());
        follow_logs(&pool, &query, &params, format).await?;
    } else {
        print_logs(&pool, &query, &params, format).await?;
    }

    Ok(())
}

async fn print_logs(
    pool: &sqlx::sqlite::SqlitePool,
    query: &str,
    params: &[String],
    format: &str,
) -> Result<()> {
    let mut sql_query = sqlx::query(query);
    for param in params {
        sql_query = sql_query.bind(param);
    }

    let rows = sql_query.fetch_all(pool).await?;

    if rows.is_empty() {
        println!("{}", "No logs found.".yellow());
        return Ok(());
    }

    match format {
        "json" => print_logs_json(&rows)?,
        "text" | _ => print_logs_text(&rows)?,
    }

    Ok(())
}

async fn follow_logs(
    pool: &sqlx::sqlite::SqlitePool,
    query: &str,
    params: &[String],
    format: &str,
) -> Result<()> {
    let mut last_timestamp: i64 = 0;

    loop {
        // Query for new events since last timestamp
        let follow_query = format!("{} AND timestamp > ?", query);
        let mut sql_query = sqlx::query(&follow_query);

        for param in params {
            sql_query = sql_query.bind(param);
        }
        sql_query = sql_query.bind(last_timestamp);

        let rows = sql_query.fetch_all(pool).await?;

        if !rows.is_empty() {
            match format {
                "json" => print_logs_json(&rows)?,
                "text" | _ => print_logs_text(&rows)?,
            }

            // Update last timestamp
            if let Some(row) = rows.last() {
                last_timestamp = sqlx::Row::get(row, "timestamp");
            }
        }

        // Sleep for a short interval
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

fn print_logs_text(rows: &[sqlx::sqlite::SqliteRow]) -> Result<()> {
    use sqlx::Row;

    for row in rows {
        let timestamp: i64 = row.get("timestamp");
        let event_type: String = row.get("event_type");
        let actor_type: String = row.get("actor_type");
        let actor_id: String = row.get("actor_id");
        let content: String = row.get("content");

        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(timestamp as u64);
        let datetime: DateTime<Local> = time.into();
        let time_str = datetime.format("%Y-%m-%d %H:%M:%S");

        // Color code by event type
        let event_colored = match event_type.as_str() {
            "agent_started" => event_type.green(),
            "agent_completed" => event_type.blue(),
            "agent_failed" => event_type.red(),
            "agent_terminated" => event_type.yellow(),
            "error" => event_type.red().bold(),
            "warning" => event_type.yellow(),
            _ => event_type.white(),
        };

        // Format actor
        let actor_colored = match actor_type.as_str() {
            "User" => format!("{}:{}", actor_type, actor_id).cyan(),
            "Agent" => format!("{}:{}", actor_type, actor_id).magenta(),
            "System" => format!("{}:{}", actor_type, actor_id).blue(),
            _ => format!("{}:{}", actor_type, actor_id).white(),
        };

        println!(
            "{} {} {} {}",
            time_str.to_string().dim(),
            event_colored,
            actor_colored,
            content
        );
    }

    Ok(())
}

fn print_logs_json(rows: &[sqlx::sqlite::SqliteRow]) -> Result<()> {
    use sqlx::Row;

    let events: Vec<_> = rows
        .iter()
        .map(|row| {
            let metadata: Option<String> = row.get("metadata");
            let metadata_json =
                metadata.and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok());

            json!({
                "id": row.get::<String, _>("id"),
                "event_type": row.get::<String, _>("event_type"),
                "timestamp": row.get::<i64, _>("timestamp"),
                "session_id": row.get::<String, _>("session_id"),
                "actor_type": row.get::<String, _>("actor_type"),
                "actor_id": row.get::<String, _>("actor_id"),
                "content": row.get::<String, _>("content"),
                "metadata": metadata_json,
                "git_commit": row.get::<Option<String>, _>("git_commit"),
            })
        })
        .collect();

    println!("{}", serde_json::to_string_pretty(&events)?);
    Ok(())
}
