use anyhow::Result;
use colored::Colorize;
use descartes_core::DescaratesConfig;
use tracing::info;
use uuid::Uuid;

/// Default TTL for attach tokens in seconds (5 minutes)
const DEFAULT_TOKEN_TTL_SECS: i64 = 300;

pub async fn execute(
    config: &DescaratesConfig,
    id: &str,
    client_type: &str,
    output_json: bool,
) -> Result<()> {
    if !output_json {
        println!(
            "{}",
            format!("Requesting attach credentials for agent: {}", id)
                .yellow()
                .bold()
        );
    }

    // Parse UUID
    let agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    // Connect to database
    let db_path = format!("{}/data/descartes.db", config.storage.base_path);
    let db_url = format!("sqlite://{}", db_path);

    let pool = sqlx::sqlite::SqlitePool::connect(&db_url).await?;

    // Check if agent exists and get status
    let row = sqlx::query("SELECT id, name, status, task FROM agents WHERE id = ?1")
        .bind(id)
        .fetch_optional(&pool)
        .await?;

    if row.is_none() {
        anyhow::bail!("Agent not found: {}", id);
    }

    let row = row.unwrap();
    let status: String = sqlx::Row::get(&row, "status");
    let name: String = sqlx::Row::get(&row, "name");
    let task: Option<String> = sqlx::Row::get(&row, "task");

    if !output_json {
        println!("  Agent: {}", name.cyan());
        println!("  Status: {}", status.yellow());
    }

    // Check if agent is paused (attach only works for paused agents)
    if status != "Paused" {
        anyhow::bail!(
            "Agent is not paused (status: {}). Pause the agent first with 'descartes pause'.",
            status
        );
    }

    info!(
        "Generating attach credentials for agent {} (client: {})",
        id, client_type
    );

    // Generate attach token
    let token = generate_attach_token();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs() as i64;
    let expires_at = now + DEFAULT_TOKEN_TTL_SECS;

    // Generate connect URL (Unix socket path for local connections)
    let socket_path = format!("{}/run/attach-{}.sock", config.storage.base_path, id);
    let connect_url = format!("unix://{}", socket_path);

    // Store attach session in database
    let session_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO attach_sessions (id, agent_id, token, client_type, connect_url, created_at, expires_at)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(&session_id)
    .bind(id)
    .bind(&token)
    .bind(client_type)
    .bind(&connect_url)
    .bind(now)
    .bind(expires_at)
    .execute(&pool)
    .await
    .or_else(|e| {
        // Table might not exist yet, just log warning and continue
        info!("Could not store attach session (table may not exist): {}", e);
        Ok::<_, sqlx::Error>(sqlx::sqlite::SqliteQueryResult::default())
    })?;

    // Record attach request event
    let event_id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"
        INSERT INTO events (id, event_type, timestamp, session_id, actor_type, actor_id, content)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
    )
    .bind(&event_id)
    .bind("attach_requested")
    .bind(now)
    .bind(id)
    .bind("System")
    .bind("cli")
    .bind(format!(
        "Attach credentials generated for agent {} (client: {})",
        name, client_type
    ))
    .execute(&pool)
    .await?;

    if output_json {
        // Output JSON for scripting/piping to other tools
        let output = serde_json::json!({
            "agent_id": id,
            "agent_name": name,
            "task": task,
            "token": token,
            "connect_url": connect_url,
            "expires_at": expires_at,
            "client_type": client_type
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("\n{}", "Attach credentials generated:".green().bold());
        println!("  Token: {}", token.cyan());
        println!("  Connect URL: {}", connect_url.cyan());
        println!(
            "  Expires at: {}",
            chrono::DateTime::from_timestamp(expires_at, 0)
                .map(|dt| dt.to_rfc3339())
                .unwrap_or_else(|| "Unknown".to_string())
                .cyan()
        );
        println!("  Client type: {}", client_type.cyan());

        println!("\n{}", "Usage:".yellow().bold());
        match client_type {
            "claude-code" => {
                println!(
                    "  claude --attach-token {} --connect {}",
                    token, connect_url
                );
            }
            "opencode" => {
                println!("  opencode attach --token {} --url {}", token, connect_url);
            }
            _ => {
                println!("  Pass the token and connect_url to your TUI client");
            }
        }

        println!(
            "\n{}",
            format!(
                "Note: Token expires in {} seconds.",
                DEFAULT_TOKEN_TTL_SECS
            )
            .yellow()
        );
    }

    Ok(())
}

/// Generate a secure random attach token
fn generate_attach_token() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    const TOKEN_LEN: usize = 32;

    let mut rng = rand::thread_rng();
    (0..TOKEN_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}
