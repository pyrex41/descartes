//! Pause a running agent via daemon RPC.
//!
//! This command calls the daemon's `agent.pause` RPC method to pause
//! an agent. The daemon handles the actual process control (SIGSTOP
//! for forced pause, or cooperative pause via stdin notification).

use anyhow::Result;
use colored::Colorize;
use tracing::info;
use uuid::Uuid;

use crate::rpc;

pub async fn execute(id: &str, force: bool) -> Result<()> {
    let mode = if force { "forced (SIGSTOP)" } else { "cooperative" };
    println!(
        "{}",
        format!("Pausing agent: {} (mode: {})", id, mode)
            .yellow()
            .bold()
    );

    // Parse UUID to validate format
    let _agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    info!("Connecting to daemon to pause agent {}", id);

    // Connect to daemon (auto-starts if needed)
    let client = rpc::connect_with_autostart().await?;

    // Call pause RPC (daemon expects positional array: [agent_id, force])
    let result = rpc::call_method(
        &client,
        "agent.pause",
        serde_json::json!([id, force]),
    )
    .await?;

    // Parse result
    let paused_at = result["paused_at"].as_i64().unwrap_or(0);
    let pause_mode = result["pause_mode"]
        .as_str()
        .unwrap_or(if force { "Forced" } else { "Cooperative" });

    println!("\n{}", "Agent paused successfully.".green().bold());
    println!("  Mode: {}", pause_mode.cyan());
    println!(
        "  Paused at: {}",
        chrono::DateTime::from_timestamp(paused_at, 0)
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
