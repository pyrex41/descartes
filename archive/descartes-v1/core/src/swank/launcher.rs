//! SBCL process launcher for Swank.

use std::process::Stdio;
use thiserror::Error;
use tokio::process::{Child, Command};
use tokio::time::{timeout, Duration};
use tracing::{debug, info};

#[derive(Error, Debug)]
pub enum LauncherError {
    #[error("SBCL not found in PATH")]
    SbclNotFound,
    #[error("Failed to spawn SBCL: {0}")]
    SpawnFailed(String),
    #[error("Swank server did not start on port {0} within timeout")]
    StartupTimeout(u16),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Launcher for SBCL with Swank server.
pub struct SwankLauncher;

impl SwankLauncher {
    /// Check if SBCL is installed.
    pub async fn check_sbcl() -> Result<(), LauncherError> {
        let output = Command::new("which")
            .arg("sbcl")
            .output()
            .await
            .map_err(|e| LauncherError::SpawnFailed(e.to_string()))?;

        if output.status.success() {
            Ok(())
        } else {
            Err(LauncherError::SbclNotFound)
        }
    }

    /// Get SBCL version string.
    pub async fn sbcl_version() -> Result<String, LauncherError> {
        let output = Command::new("sbcl")
            .arg("--version")
            .output()
            .await
            .map_err(|e| LauncherError::SpawnFailed(e.to_string()))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(LauncherError::SbclNotFound)
        }
    }

    /// Start SBCL with Swank server on the specified port.
    pub async fn start_sbcl(port: u16) -> Result<Child, LauncherError> {
        Self::check_sbcl().await?;

        // Lisp code to start Swank server
        // Uses ASDF to load swank if available, falls back to quicklisp
        let swank_init = format!(
            r#"
(handler-case
    (progn
      ;; Try ASDF first (works with most setups)
      (require :asdf)
      (handler-case
          (asdf:load-system :swank)
        (error ()
          ;; Fall back to quicklisp if available
          (when (find-package :ql)
            (funcall (find-symbol "QUICKLOAD" :ql) :swank))))
      (swank:create-server :port {} :dont-close t))
  (error (e)
    (format *error-output* "Failed to start Swank: ~A~%" e)
    (sb-ext:exit :code 1)))
"#,
            port
        );

        info!("Starting SBCL with Swank on port {}", port);

        let child = Command::new("sbcl")
            .arg("--noinform")
            .arg("--eval")
            .arg(&swank_init)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| LauncherError::SpawnFailed(e.to_string()))?;

        // Wait for port to become available
        Self::wait_for_port(port, Duration::from_secs(30)).await?;

        info!("SBCL Swank server ready on port {}", port);
        Ok(child)
    }

    /// Start SBCL with a custom init file.
    pub async fn start_sbcl_with_init(
        port: u16,
        init_file: Option<&std::path::Path>,
    ) -> Result<Child, LauncherError> {
        Self::check_sbcl().await?;

        let swank_init = format!(
            r#"(swank:create-server :port {} :dont-close t)"#,
            port
        );

        info!("Starting SBCL with Swank on port {}", port);

        let mut cmd = Command::new("sbcl");
        cmd.arg("--noinform");

        // Load init file if provided
        if let Some(init) = init_file {
            cmd.arg("--load").arg(init);
        }

        // Then start swank
        cmd.arg("--eval")
            .arg("(require :asdf)")
            .arg("--eval")
            .arg("(asdf:load-system :swank)")
            .arg("--eval")
            .arg(&swank_init)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);

        let child = cmd
            .spawn()
            .map_err(|e| LauncherError::SpawnFailed(e.to_string()))?;

        // Wait for port to become available
        Self::wait_for_port(port, Duration::from_secs(30)).await?;

        info!("SBCL Swank server ready on port {}", port);
        Ok(child)
    }

    /// Wait for a port to become available (accepting connections).
    async fn wait_for_port(port: u16, max_wait: Duration) -> Result<(), LauncherError> {
        use tokio::net::TcpStream;

        let start = std::time::Instant::now();
        let addr = format!("127.0.0.1:{}", port);

        while start.elapsed() < max_wait {
            match timeout(Duration::from_millis(500), TcpStream::connect(&addr)).await {
                Ok(Ok(_)) => {
                    debug!("Port {} is now available", port);
                    return Ok(());
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            }
        }

        Err(LauncherError::StartupTimeout(port))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_check_sbcl() {
        // This test depends on whether SBCL is installed
        let result = SwankLauncher::check_sbcl().await;
        // We don't assert success/failure since it depends on the environment
        // Just ensure it doesn't panic
        let _ = result;
    }

    #[tokio::test]
    async fn test_sbcl_version() {
        // This test depends on whether SBCL is installed
        if SwankLauncher::check_sbcl().await.is_ok() {
            let version = SwankLauncher::sbcl_version().await;
            if let Ok(v) = version {
                assert!(v.contains("SBCL") || v.contains("sbcl"));
            }
        }
    }
}
