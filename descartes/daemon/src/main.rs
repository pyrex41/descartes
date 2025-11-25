/// Descartes RPC Daemon - Main entry point
/// Starts the JSON-RPC 2.0 server for remote agent control
use clap::Parser;
use descartes_daemon::{DaemonConfig, RpcServer};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "descartes-daemon",
    about = "RPC server for managing Descartes agents and workflows",
    version = env!("CARGO_PKG_VERSION")
)]
struct Args {
    /// Configuration file path
    #[arg(
        short,
        long,
        value_name = "PATH",
        help = "Path to configuration file (TOML)"
    )]
    config: Option<PathBuf>,

    /// HTTP server port
    #[arg(
        short,
        long,
        value_name = "PORT",
        help = "HTTP server port (default: 8080)"
    )]
    http_port: Option<u16>,

    /// WebSocket server port
    #[arg(
        short,
        long,
        value_name = "PORT",
        help = "WebSocket server port (default: 8081)"
    )]
    ws_port: Option<u16>,

    /// Enable authentication
    #[arg(long, help = "Enable JWT authentication")]
    enable_auth: bool,

    /// JWT secret
    #[arg(
        long,
        value_name = "SECRET",
        help = "JWT secret (required if auth enabled)"
    )]
    jwt_secret: Option<String>,

    /// Log level
    #[arg(
        short,
        long,
        value_name = "LEVEL",
        default_value = "info",
        help = "Log level (trace, debug, info, warn, error)"
    )]
    log_level: String,

    /// Enable verbose logging
    #[arg(short, long, help = "Enable verbose output")]
    verbose: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Setup logging
    let log_level = if args.verbose {
        "debug"
    } else {
        &args.log_level
    };
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(log_level.parse()?))
        .with_target(false)
        .with_thread_ids(false)
        .with_line_number(true)
        .init();

    info!(
        "Starting Descartes RPC Daemon v{}",
        descartes_daemon::VERSION
    );

    // Load configuration
    let mut config = match args.config {
        Some(path) => {
            info!("Loading configuration from: {}", path.display());
            DaemonConfig::load(path.to_str().unwrap())?
        }
        None => {
            info!("Using default configuration");
            DaemonConfig::default()
        }
    };

    // Apply CLI overrides
    if let Some(port) = args.http_port {
        config.server.http_port = port;
    }
    if let Some(port) = args.ws_port {
        config.server.ws_port = port;
    }

    if args.enable_auth {
        config.auth.enabled = true;
        if let Some(secret) = args.jwt_secret {
            config.auth.jwt_secret = secret;
        } else {
            eprintln!("Error: JWT secret required when auth is enabled");
            std::process::exit(1);
        }
    }

    // Validate configuration
    config.validate()?;

    info!(
        "Server configuration: HTTP {}:{}, WebSocket {}:{}",
        config.server.http_addr,
        config.server.http_port,
        config.server.ws_addr,
        config.server.ws_port
    );

    if config.auth.enabled {
        info!("Authentication: ENABLED");
    } else {
        info!("Authentication: DISABLED");
    }

    // Create and start RPC server
    let server = RpcServer::new(config)?;

    // Setup signal handling for graceful shutdown
    let (tx, rx) = tokio::sync::mpsc::channel(1);

    let signal_handler = tokio::spawn(async move {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("Failed to install CTRL+C signal handler");
        };

        #[cfg(unix)]
        let terminate = async {
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("Failed to install SIGTERM handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            () = ctrl_c => {
                info!("Received CTRL+C signal");
            },
            () = terminate => {
                info!("Received SIGTERM signal");
            },
        }

        let _ = tx.send(()).await;
    });

    // Run server in background
    let server_handle = tokio::spawn(async move {
        if let Err(e) = server.run().await {
            eprintln!("Server error: {:?}", e);
        }
    });

    // Wait for signal
    tokio::select! {
        _ = signal_handler => {
            info!("Shutting down daemon...");
        }
        _ = rx.recv() => {
            info!("Shutting down daemon...");
        }
        _ = server_handle => {
            info!("Server terminated unexpectedly");
        }
    }

    info!("Descartes RPC Daemon stopped");
    Ok(())
}
