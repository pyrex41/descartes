//! Swank protocol client for SBCL integration.
//!
//! This module provides TCP communication with a Swank server running
//! in an SBCL process, enabling Lisp code evaluation, compilation,
//! inspection, and debugger interaction.

mod client;
mod codec;
mod launcher;
mod registry;

pub use client::{SwankClient, SwankError, SwankMessage, SwankRestart, SwankFrame};
pub use launcher::{SwankLauncher, LauncherError};
pub use registry::SwankSessionRegistry;

/// Default Swank port (used as starting point for allocation).
pub const DEFAULT_SWANK_PORT: u16 = 4005;

/// Find an available port starting from the given port.
pub async fn find_available_port(start_port: u16) -> std::io::Result<u16> {
    use tokio::net::TcpListener;

    for port in start_port..start_port + 100 {
        match TcpListener::bind(("127.0.0.1", port)).await {
            Ok(_listener) => return Ok(port),
            Err(_) => continue,
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrNotAvailable,
        "No available ports in range",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_find_available_port() {
        // Should find an available port
        let port = find_available_port(40000).await.unwrap();
        assert!(port >= 40000 && port < 40100);
    }
}
