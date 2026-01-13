//! Get attach credentials for a paused agent via daemon RPC.
//!
//! This command calls the daemon's `agent.attach.request` RPC method
//! to generate attach credentials (token, connect URL) that can be
//! used by external TUI clients like Claude Code or OpenCode.

use anyhow::Result;
use colored::Colorize;
use std::process::Command;
use tracing::info;
use uuid::Uuid;

use crate::rpc;

pub async fn execute(
    id: &str,
    client_type: &str,
    output_json: bool,
    launch: bool,
) -> Result<()> {
    if !output_json {
        println!(
            "{}",
            format!("Requesting attach credentials for agent: {}", id)
                .yellow()
                .bold()
        );
    }

    // Parse UUID to validate format
    let _agent_id = Uuid::parse_str(id)
        .map_err(|_| anyhow::anyhow!("Invalid agent ID format. Expected UUID."))?;

    info!(
        "Connecting to daemon to request attach credentials for agent {}",
        id
    );

    // Connect to daemon (auto-starts if needed)
    let client = rpc::connect_with_autostart().await?;

    // Call attach.request RPC (daemon expects positional array: [agent_id, client_type])
    let result = rpc::call_method(
        &client,
        "agent.attach.request",
        serde_json::json!([id, client_type]),
    )
    .await?;

    // Parse result
    let token = result["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No token in response"))?;
    let connect_url = result["connect_url"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("No connect_url in response"))?;
    let expires_at = result["expires_at"].as_i64().unwrap_or(0);

    if output_json {
        // Output JSON for scripting
        let output = serde_json::json!({
            "agent_id": id,
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

        // Calculate TTL from expires_at
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let ttl = expires_at - now;
        if ttl > 0 {
            println!(
                "\n{}",
                format!("Note: Token expires in {} seconds.", ttl).yellow()
            );
        }
    }

    // Launch TUI if requested
    if launch {
        launch_tui(client_type, token, connect_url)?;
    }

    Ok(())
}

/// Launch the appropriate TUI client with attach credentials
fn launch_tui(client_type: &str, token: &str, connect_url: &str) -> Result<()> {
    println!(
        "\n{}",
        format!("Launching {} TUI...", client_type).green().bold()
    );

    match client_type {
        "claude-code" => {
            // Try to find claude command
            let claude_cmd = which_claude().ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not find 'claude' command. Please ensure Claude Code is installed and in your PATH."
                )
            })?;

            info!("Launching Claude Code TUI: {} --attach-token {} --connect {}", claude_cmd, token, connect_url);

            // Spawn the TUI process, replacing the current process
            let status = Command::new(&claude_cmd)
                .arg("--attach-token")
                .arg(token)
                .arg("--connect")
                .arg(connect_url)
                .status()?;

            if !status.success() {
                anyhow::bail!("Claude Code exited with status: {:?}", status.code());
            }
        }
        "opencode" => {
            // Try to find opencode command
            let opencode_cmd = which_opencode().ok_or_else(|| {
                anyhow::anyhow!(
                    "Could not find 'opencode' command. Please ensure OpenCode is installed and in your PATH."
                )
            })?;

            info!("Launching OpenCode TUI: {} attach --token {} --url {}", opencode_cmd, token, connect_url);

            let status = Command::new(&opencode_cmd)
                .arg("attach")
                .arg("--token")
                .arg(token)
                .arg("--url")
                .arg(connect_url)
                .status()?;

            if !status.success() {
                anyhow::bail!("OpenCode exited with status: {:?}", status.code());
            }
        }
        other => {
            anyhow::bail!(
                "Cannot auto-launch unknown client type '{}'. Use --json to get credentials for manual connection.",
                other
            );
        }
    }

    Ok(())
}

/// Find the claude command in PATH
fn which_claude() -> Option<String> {
    // Try common names for the Claude Code CLI
    for cmd in &["claude", "claude-code"] {
        if Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(cmd.to_string());
        }
    }
    None
}

/// Find the opencode command in PATH
fn which_opencode() -> Option<String> {
    if Command::new("which")
        .arg("opencode")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        return Some("opencode".to_string());
    }
    None
}
