//! Settings display functions

use crate::{Config, Result};

/// Show current configuration
pub fn show_config(config: &Config) -> Result<()> {
    println!("Descartes Configuration");
    println!("=======================\n");

    println!("Harness: {}", config.harness.kind);
    println!();

    println!("Categories:");
    for (name, cat) in &config.categories {
        println!("  {}:", name);
        println!("    description: {}", cat.description);
        println!("    model: {}", cat.model);
        if let Some(ref h) = cat.harness {
            println!("    harness: {}", h);
        }
        println!("    tools: {:?}", cat.tools);
        println!("    parallel: {}", cat.parallel);
        println!("    backpressure: {}", cat.backpressure);
    }
    println!();

    println!("SCUD:");
    println!("  task_file: {:?}", config.scud.task_file);
    println!("  embedded: {}", config.scud.embedded);
    println!();

    println!("Transcripts:");
    println!("  directory: {:?}", config.transcripts.directory);
    println!("  format: {}", config.transcripts.format);
    println!();

    Ok(())
}

/// Show active harness information
pub fn show_harness(config: &Config) -> Result<()> {
    println!("Active Harness: {}", config.harness.kind);
    println!();

    match config.harness.kind.as_str() {
        "claude-code" => {
            println!("Claude Code Settings:");
            if let Some(ref bin) = config.harness.claude_code.binary {
                println!("  binary: {}", bin);
            } else {
                println!("  binary: claude (default)");
            }
            println!("  model: {}", config.harness.claude_code.model);
            println!("  headless: {}", config.harness.claude_code.headless);
            println!(
                "  skip_permissions: {}",
                config.harness.claude_code.dangerously_skip_permissions
            );
        }
        "opencode" => {
            println!("OpenCode Settings:");
            if let Some(ref bin) = config.harness.opencode.binary {
                println!("  binary: {}", bin);
            } else {
                println!("  binary: opencode (default)");
            }
            println!("  model: {}", config.harness.opencode.model);
        }
        "codex" => {
            println!("Codex Settings:");
            if let Some(ref base) = config.harness.codex.api_base {
                println!("  api_base: {}", base);
            }
            println!("  model: {}", config.harness.codex.model);
        }
        other => {
            println!("Unknown harness: {}", other);
        }
    }

    Ok(())
}
