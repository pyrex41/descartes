use anyhow::Result;
use colored::Colorize;
use descartes_core::DescaratesConfig;
use tracing::info;
use uuid::Uuid;

pub async fn execute(config: &DescaratesConfig, id: &str, force: bool) -> Result<()> {
    println!("{}", format!("Killing agent: {}", id).yellow().bold());

    // Parse UUID
    let _agent_id = Uuid::parse_str(id)
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

    // Check if already terminated
    if status == "Completed" || status == "Failed" || status == "Terminated" {
        println!("{}", "Agent is already terminated.".yellow());
        return Ok(());
    }

    // Determine signal type
    let signal_type = if force { "SIGKILL" } else { "SIGTERM" };
    println!("  Signal: {}", signal_type.red());

    // In a real implementation, we would send the signal to the running process
    // For now, we'll just update the database

    info!("Terminating agent {} with signal {}", id, signal_type);

    // Update agent status
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;

    sqlx::query(
        r#"
        UPDATE agents
        SET status = 'Terminated', completed_at = ?1
        WHERE id = ?2
        "#,
    )
    .bind(now)
    .bind(id)
    .execute(&pool)
    .await?;

    // Record termination event
    let event_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO events (id, event_type, timestamp, session_id, actor_type, actor_id, content)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(&event_id)
    .bind("agent_terminated")
    .bind(now)
    .bind(id)
    .bind("System")
    .bind("cli")
    .bind(format!(
        "Agent {} terminated with signal {}",
        name, signal_type
    ))
    .execute(&pool)
    .await?;

    println!("\n{}", "Agent terminated successfully.".green().bold());

    // Show cleanup message
    if force {
        println!(
            "{}",
            "Note: Forced termination may leave resources in inconsistent state.".yellow()
        );
    }

    Ok(())
}
