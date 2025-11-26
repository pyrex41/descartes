use anyhow::Result;
use colored::Colorize;
use descartes_core::DescaratesConfig;
use tracing::info;
use uuid::Uuid;

pub async fn execute(config: &DescaratesConfig, id: &str, force: bool) -> Result<()> {
    let mode = if force { "forced (SIGSTOP)" } else { "cooperative" };
    println!(
        "{}",
        format!("Pausing agent: {} (mode: {})", id, mode)
            .yellow()
            .bold()
    );

    // Parse UUID
    let agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    // Connect to database
    let db_path = format!("{}/data/descartes.db", config.storage.base_path);
    let db_url = format!("sqlite://{}", db_path);

    let pool = sqlx::sqlite::SqlitePool::connect(&db_url).await?;

    // Check if agent exists and get status
    let row = sqlx::query("SELECT id, name, status FROM agents WHERE id = ?1")
        .bind(id)
        .fetch_optional(&pool)
        .await?;

    if row.is_none() {
        anyhow::bail!("Agent not found: {}", id);
    }

    let row = row.unwrap();
    let status: String = sqlx::Row::get(&row, "status");
    let name: String = sqlx::Row::get(&row, "name");

    println!("  Agent: {}", name.cyan());
    println!("  Status: {}", status.yellow());

    // Check if agent is running
    if status != "Running" {
        anyhow::bail!("Agent is not running (status: {}). Cannot pause.", status);
    }

    info!("Pausing agent {} with mode {}", id, mode);

    // Update agent status
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    let pause_mode = if force { "Forced" } else { "Cooperative" };

    sqlx::query(
        r#"
        UPDATE agents
        SET status = 'Paused', paused_at = ?1, pause_mode = ?2
        WHERE id = ?3
        "#,
    )
    .bind(now)
    .bind(pause_mode)
    .bind(id)
    .execute(&pool)
    .await?;

    // Record pause event
    let event_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO events (id, event_type, timestamp, session_id, actor_type, actor_id, content)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(&event_id)
    .bind("agent_paused")
    .bind(now)
    .bind(id)
    .bind("System")
    .bind("cli")
    .bind(format!("Agent {} paused with mode {}", name, pause_mode))
    .execute(&pool)
    .await?;

    println!("\n{}", "Agent paused successfully.".green().bold());
    println!("  Mode: {}", pause_mode.cyan());
    println!(
        "  Paused at: {}",
        chrono::DateTime::from_timestamp(now, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "Unknown".to_string())
            .cyan()
    );

    if force {
        println!(
            "\n{}",
            "Note: Agent was force-paused using SIGSTOP. Use 'descartes resume' to continue."
                .yellow()
        );
    } else {
        println!(
            "\n{}",
            "Note: Agent received cooperative pause signal. It will pause at the next safe point."
                .yellow()
        );
    }

    // Show attach hint
    println!(
        "\n{}",
        "To attach an external TUI, run: descartes attach <agent-id>".cyan()
    );

    Ok(())
}
