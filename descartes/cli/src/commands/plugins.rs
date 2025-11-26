use anyhow::Result;
use clap::Subcommand;
use colored::Colorize;
use descartes_core::plugins::manager::PluginManager;
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum PluginCommands {
    /// List installed plugins
    List,
    /// Install a plugin from a path
    Install {
        /// Path to the .wasm plugin file
        path: PathBuf,
    },
    /// Execute a plugin
    Exec {
        /// Name of the plugin
        name: String,
        /// Input string
        input: String,
    },
}

pub async fn execute(command: &PluginCommands) -> Result<()> {
    let home = dirs::home_dir().expect("Could not find home directory");
    let plugins_dir = home.join(".descartes").join("plugins");
    let mut manager = PluginManager::new(&plugins_dir)?;

    // Load existing plugins
    manager.load_all().await?;

    match command {
        PluginCommands::List => {
            println!("{}", "Installed Plugins:".green().bold());
            let plugins = manager.list_plugins();
            if plugins.is_empty() {
                println!("  (none)");
            } else {
                for plugin in plugins {
                    println!("  - {}", plugin);
                }
            }
        }
        PluginCommands::Install { path } => {
            if !path.exists() {
                anyhow::bail!("Plugin file not found: {:?}", path);
            }

            if !plugins_dir.exists() {
                tokio::fs::create_dir_all(&plugins_dir).await?;
            }

            let file_name = path.file_name().expect("Invalid file name");
            let dest = plugins_dir.join(file_name);
            tokio::fs::copy(path, &dest).await?;
            println!("{} {}", "Installed plugin:".green(), dest.display());
        }
        PluginCommands::Exec { name, input } => match manager.execute_plugin(name, input).await {
            Ok(output) => println!("{}", output),
            Err(e) => eprintln!("{} {}", "Error:".red(), e),
        },
    }

    Ok(())
}
