//! System health check command - displays status of all Descartes components

use anyhow::Result;
use colored::Colorize;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check status indicator
enum Status {
    Ok,
    Warning,
    Error,
    NotConfigured,
}

impl Status {
    fn symbol(&self) -> String {
        match self {
            Status::Ok => "✓".green().to_string(),
            Status::Warning => "!".yellow().to_string(),
            Status::Error => "✗".red().to_string(),
            Status::NotConfigured => "○".dimmed().to_string(),
        }
    }
}

fn print_check(status: Status, label: &str, value: &str) {
    println!("  {} {}: {}", status.symbol(), label, value);
}

fn print_section(title: &str) {
    println!("\n{}", title.bold());
}

/// Get human-readable file size
fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Mask an API key for display
fn mask_key(key: &str) -> String {
    if key.len() <= 8 {
        "***".to_string()
    } else {
        format!("{}...{}", &key[..4], &key[key.len() - 4..])
    }
}

/// Check Rust toolchain
fn check_rust() -> (Status, String) {
    match Command::new("rustc").arg("--version").output() {
        Ok(output) => {
            if output.status.success() {
                let version = String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .replace("rustc ", "");
                (Status::Ok, version)
            } else {
                (Status::Error, "failed to get version".to_string())
            }
        }
        Err(_) => (Status::Error, "not found".to_string()),
    }
}

/// Check database
fn check_database(base_dir: &Path) -> (Status, String) {
    let db_path = base_dir.join("data/descartes.db");
    if db_path.exists() {
        match fs::metadata(&db_path) {
            Ok(meta) => (Status::Ok, format!("{} ({})", db_path.display(), human_size(meta.len()))),
            Err(_) => (Status::Warning, format!("{} (unreadable)", db_path.display())),
        }
    } else {
        (Status::NotConfigured, "not initialized (run 'descartes init')".to_string())
    }
}

/// Check config file
fn check_config(base_dir: &Path) -> (Status, String) {
    let config_path = base_dir.join("config.toml");
    if config_path.exists() {
        (Status::Ok, config_path.display().to_string())
    } else {
        (Status::NotConfigured, "not found (run 'descartes init')".to_string())
    }
}

/// Check API key
fn check_api_key(env_var: &str) -> (Status, String) {
    match env::var(env_var) {
        Ok(key) if !key.is_empty() => (Status::Ok, mask_key(&key)),
        Ok(_) => (Status::NotConfigured, "empty".to_string()),
        Err(_) => (Status::NotConfigured, "not set".to_string()),
    }
}

/// Check daemon status
fn check_daemon() -> (Status, String) {
    // Try to connect to the daemon on default port
    match std::net::TcpStream::connect_timeout(
        &"127.0.0.1:8080".parse().unwrap(),
        std::time::Duration::from_millis(500),
    ) {
        Ok(_) => (Status::Ok, "running on port 8080".to_string()),
        Err(_) => (Status::NotConfigured, "not running".to_string()),
    }
}

/// Check sessions directory
fn check_sessions() -> (Status, String) {
    // Check both possible locations
    let locations = vec![
        PathBuf::from(".scud/sessions"),
        dirs::home_dir()
            .map(|h| h.join(".descartes/sessions"))
            .unwrap_or_default(),
    ];

    for sessions_dir in locations {
        if sessions_dir.exists() {
            match fs::read_dir(&sessions_dir) {
                Ok(entries) => {
                    let mut count = 0;
                    let mut total_size = 0u64;
                    for entry in entries.flatten() {
                        if entry.path().extension().is_some_and(|e| e == "json") {
                            count += 1;
                            if let Ok(meta) = entry.metadata() {
                                total_size += meta.len();
                            }
                        }
                    }
                    if count > 0 {
                        return (
                            Status::Ok,
                            format!("{} sessions ({})", count, human_size(total_size)),
                        );
                    }
                }
                Err(_) => continue,
            }
        }
    }

    (Status::NotConfigured, "no sessions yet".to_string())
}

/// Check skills directory
fn check_skills() -> (Status, String) {
    let skills_dir = PathBuf::from(".descartes/skills");
    if skills_dir.exists() {
        match fs::read_dir(&skills_dir) {
            Ok(entries) => {
                let count = entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_file())
                    .count();
                if count > 0 {
                    (Status::Ok, format!("{} skills available", count))
                } else {
                    (Status::NotConfigured, "no skills installed".to_string())
                }
            }
            Err(_) => (Status::Warning, "unreadable".to_string()),
        }
    } else {
        (Status::NotConfigured, "no skills directory".to_string())
    }
}

pub async fn execute() -> Result<()> {
    println!();
    println!("{}", "Descartes System Health Check".cyan().bold());
    println!("{}", "─".repeat(40).dimmed());

    // Determine base directory
    let base_dir = dirs::home_dir()
        .map(|h| h.join(".descartes"))
        .unwrap_or_else(|| PathBuf::from(".descartes"));

    // System
    print_section("System");
    let (status, version) = check_rust();
    print_check(status, "Rust toolchain", &version);

    // Storage
    print_section("Storage");
    let (status, db_info) = check_database(&base_dir);
    print_check(status, "Database", &db_info);

    let (status, config_info) = check_config(&base_dir);
    print_check(status, "Config", &config_info);

    let (status, sessions_info) = check_sessions();
    print_check(status, "Sessions", &sessions_info);

    // API Keys
    print_section("API Keys");
    let providers = [
        ("XAI_API_KEY", "Grok (xAI)"),
        ("ANTHROPIC_API_KEY", "Anthropic"),
        ("OPENAI_API_KEY", "OpenAI"),
        ("DEEPSEEK_API_KEY", "DeepSeek"),
        ("GROQ_API_KEY", "Groq"),
    ];

    let mut any_configured = false;
    for (env_var, name) in providers {
        let (status, value) = check_api_key(env_var);
        if matches!(status, Status::Ok) {
            any_configured = true;
        }
        print_check(status, name, &value);
    }

    // Check Ollama separately
    let ollama_status = match env::var("OLLAMA_ENDPOINT") {
        Ok(endpoint) => (Status::Ok, endpoint),
        Err(_) => {
            // Check if Ollama is running on default port
            match std::net::TcpStream::connect_timeout(
                &"127.0.0.1:11434".parse().unwrap(),
                std::time::Duration::from_millis(500),
            ) {
                Ok(_) => (Status::Ok, "localhost:11434 (auto-detected)".to_string()),
                Err(_) => (Status::NotConfigured, "not running".to_string()),
            }
        }
    };
    print_check(ollama_status.0, "Ollama", &ollama_status.1);

    // Services
    print_section("Services");
    let (status, daemon_info) = check_daemon();
    print_check(status, "Daemon", &daemon_info);

    let (status, skills_info) = check_skills();
    print_check(status, "Skills", &skills_info);

    // Summary
    println!("\n{}", "─".repeat(40).dimmed());

    if any_configured {
        println!(
            "\n{}",
            "At least one provider configured. Ready to spawn agents!".green()
        );
        println!(
            "\nTry: {}",
            "descartes spawn --task \"Hello, world!\"".cyan()
        );
    } else {
        println!(
            "\n{}",
            "No API keys configured.".yellow()
        );
        println!("\nTo get started:");
        println!(
            "  1. Set an API key: {}",
            "export XAI_API_KEY=xai-...".cyan()
        );
        println!(
            "  2. Or start Ollama: {}",
            "ollama serve".cyan()
        );
        println!(
            "  3. Then run: {}",
            "descartes spawn --task \"Hello, world!\"".cyan()
        );
    }

    println!();
    Ok(())
}
