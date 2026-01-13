//! Resume a paused agent via daemon RPC.
//!
//! This command calls the daemon's `agent.resume` RPC method to resume
//! an agent. The daemon handles the actual process control (SIGCONT
//! for forced pause, or resume notification via stdin for cooperative pause).

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use uuid::Uuid;

use crate::rpc;

pub async fn execute(id: &str) -> Result<()> {
    println!("{}", format!("Resuming agent: {}", id).yellow().bold());

    // Parse UUID to validate format
    let _agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    info!("Connecting to daemon to resume agent {}", id);

    // Connect to daemon (auto-starts if needed)
    let client = rpc::connect_with_autostart().await?;

    // Call resume RPC (daemon expects positional array: [agent_id])
    let result = rpc::call_method(
        &client,
        "agent.resume",
        serde_json::json!([id]),
    )
    .await?;

    // Parse result
    let resumed_at = result["resumed_at"].as_i64().unwrap_or(0);

    println!("\n{}", "Agent resumed successfully.".green().bold());
    println!(
        "  Resumed at: {}",
        chrono::DateTime::from_timestamp(resumed_at, 0)
            .map(|dt| dt.to_rfc3339())
            .unwrap_or_else(|| "Unknown".to_string())
            .cyan()
    );

    Ok(())
}
