//! Agent display functions

use crate::{Config, Result};

/// Show list of available agents
pub fn show_agents_list(config: &Config) -> Result<()> {
    let agents_dir = &config.agents.directory;

    if !agents_dir.exists() {
        println!("No agents directory found at {:?}", agents_dir);
        println!("Run 'descartes agents init' to create default agents.");
        return Ok(());
    }

    println!("Available Agents");
    println!("================\n");

    let mut found = false;
    for entry in std::fs::read_dir(agents_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let agent_file = path.join("AGENT.md");
            if agent_file.exists() {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                // Try to extract description from first line
                let description = std::fs::read_to_string(&agent_file)
                    .ok()
                    .and_then(|content| {
                        content
                            .lines()
                            .find(|l| !l.trim().is_empty() && !l.starts_with('#'))
                            .map(|s| s.trim().to_string())
                    })
                    .unwrap_or_default();

                println!("  {} - {}", name, description);
                found = true;
            }
        }
    }

    if !found {
        println!("No agents found in {:?}", agents_dir);
    }

    Ok(())
}

/// Show details of a specific agent
pub fn show_agent_detail(config: &Config, name: &str) -> Result<()> {
    let agent_file = config.agents.directory.join(name).join("AGENT.md");

    if !agent_file.exists() {
        println!("Agent not found: {}", name);
        return Ok(());
    }

    let content = std::fs::read_to_string(&agent_file)?;
    println!("{}", content);

    Ok(())
}
