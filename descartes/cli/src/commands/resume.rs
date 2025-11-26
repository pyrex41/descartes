use anyhow::Result;
use colored::Colorize;
use descartes_core::DescaratesConfig;
use tracing::info;
use uuid::Uuid;

pub async fn execute(config: &DescaratesConfig, id: &str) -> Result<()> {
    println!("{}", format!("Resuming agent: {}", id).yellow().bold());

    // Parse UUID
    let agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    // Connect to database
    let db_path = format!("{}/data/descartes.db", config.storage.base_path);
    let db_url = format!("sqlite://{}", db_path);

    let pool = sqlx::sqlite::SqlitePool::connect(&db_url).await?;

    // Check if agent exists and get status
    let row = sqlx::query("SELECT id, name, status, pause_mode FROM agents WHERE id = ?1")
        .bind(id)
        .fetch_optional(&pool)
        .await?;

    if row.is_none() {
        anyhow::bail!("Agent not found: {}", id);
    }

    let row = row.unwrap();
    let status: String = sqlx::Row::get(&row, "status");
    let name: String = sqlx::Row::get(&row, "name");
    let pause_mode: Option<String> = sqlx::Row::get(&row, "pause_mode");

    println!("  Agent: {}", name.cyan());
    println!("  Status: {}", status.yellow());

    // Check if agent is paused
    if status != "Paused" {
        anyhow::bail!(
            "Agent is not paused (status: {}). Cannot resume.",
            status
        );
    }

    let mode = pause_mode.as_deref().unwrap_or("Unknown");
    println!("  Pause mode: {}", mode.cyan());

    info!("Resuming agent {}", id);

    // Update agent status
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    sqlx::query(
        r#"
        UPDATE agents
        SET status = 'Running', paused_at = NULL, pause_mode = NULL
        WHERE id = ?1
        "#,
    )
    .bind(id)
    .execute(&pool)
    .await?;

    // Record resume event
    let event_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO events (id, event_type, timestamp, session_id, actor_type, actor_id, content)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(&event_id)
    .bind("agent_resumed")
    .bind(now)
    .bind(id)
    .bind("System")
    .bind("cli")
    .bind(format!(
        "Agent {} resumed from {} pause",
        name,
        mode.to_lowercase()
    ))
    .execute(&pool)
    .await?;

    println!("\n{}", "Agent resumed successfully.".green().bold());
    println!(
        "  Resumed at: {}",
        chrono::DateTime::from_timestamp(now, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "Unknown".to_string())
            .cyan()
    );

    if mode == "Forced" {
        println!(
            "\n{}",
            "Note: Agent was force-resumed using SIGCONT.".yellow()
        );
    }

    Ok(())
}
