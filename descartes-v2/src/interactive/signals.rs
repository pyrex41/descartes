//! Signal handling for interactive sessions
//!
//! Handles Ctrl+C (SIGINT) to pause/interrupt agents rather than exit.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::Result;

/// Signal handler for the interactive session
pub struct SignalHandler {
    /// Interrupt flag - set on first Ctrl+C
    interrupt_flag: Arc<AtomicBool>,
    /// Shutdown flag - set on second Ctrl+C
    shutdown_flag: Arc<AtomicBool>,
    /// Count of consecutive interrupts
    interrupt_count: Arc<AtomicUsize>,
}

impl SignalHandler {
    /// Create a new signal handler
    pub fn new(interrupt_flag: Arc<AtomicBool>, shutdown_flag: Arc<AtomicBool>) -> Self {
        Self {
            interrupt_flag,
            shutdown_flag,
            interrupt_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    /// Install signal handlers
    pub fn install(&self) -> Result<()> {
        let interrupt_flag = self.interrupt_flag.clone();
        let shutdown_flag = self.shutdown_flag.clone();
        let interrupt_count = self.interrupt_count.clone();

        ctrlc::set_handler(move || {
            let count = interrupt_count.fetch_add(1, Ordering::SeqCst);

            if count == 0 {
                // First Ctrl+C: set interrupt flag
                debug!("First interrupt received");
                interrupt_flag.store(true, Ordering::SeqCst);

                // Reset count after a delay (so double Ctrl+C must be quick)
                let count_clone = interrupt_count.clone();
                std::thread::spawn(move || {
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    count_clone.store(0, Ordering::SeqCst);
                });
            } else {
                // Second Ctrl+C within 2 seconds: shutdown
                info!("Second interrupt received, shutting down");
                shutdown_flag.store(true, Ordering::SeqCst);
            }
        })
        .map_err(|e| crate::Error::Config(format!("Failed to set signal handler: {}", e)))?;

        Ok(())
    }

    /// Get the interrupt flag
    pub fn interrupt_flag(&self) -> Arc<AtomicBool> {
        self.interrupt_flag.clone()
    }

    /// Get the shutdown flag
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }

    /// Check if interrupted
    pub fn is_interrupted(&self) -> bool {
        self.interrupt_flag.load(Ordering::SeqCst)
    }

    /// Check if shutdown requested
    pub fn is_shutdown(&self) -> bool {
        self.shutdown_flag.load(Ordering::SeqCst)
    }

    /// Clear the interrupt flag
    pub fn clear_interrupt(&self) {
        self.interrupt_flag.store(false, Ordering::SeqCst);
    }

    /// Reset all flags
    pub fn reset(&self) {
        self.interrupt_flag.store(false, Ordering::SeqCst);
        self.shutdown_flag.store(false, Ordering::SeqCst);
        self.interrupt_count.store(0, Ordering::SeqCst);
    }
}

/// Install a simple panic handler that provides better output
pub fn install_panic_handler() {
    std::panic::set_hook(Box::new(|info| {
        let msg = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_string()
        };

        let location = if let Some(loc) = info.location() {
            format!(" at {}:{}", loc.file(), loc.line())
        } else {
            String::new()
        };

        eprintln!("\n\x1b[31m╭────────────────────────────────────────╮\x1b[0m");
        eprintln!("\x1b[31m│  Descartes crashed unexpectedly        │\x1b[0m");
        eprintln!("\x1b[31m╰────────────────────────────────────────╯\x1b[0m");
        eprintln!("\n\x1b[33mError:\x1b[0m {}{}\n", msg, location);
        eprintln!("Please report this issue at:");
        eprintln!("  https://github.com/yourusername/descartes/issues\n");
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signal_handler_flags() {
        let interrupt = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));
        let handler = SignalHandler::new(interrupt.clone(), shutdown.clone());

        assert!(!handler.is_interrupted());
        assert!(!handler.is_shutdown());

        interrupt.store(true, Ordering::SeqCst);
        assert!(handler.is_interrupted());

        handler.clear_interrupt();
        assert!(!handler.is_interrupted());
    }

    #[test]
    fn test_signal_handler_reset() {
        let interrupt = Arc::new(AtomicBool::new(true));
        let shutdown = Arc::new(AtomicBool::new(true));
        let handler = SignalHandler::new(interrupt.clone(), shutdown.clone());

        handler.reset();

        assert!(!handler.is_interrupted());
        assert!(!handler.is_shutdown());
    }
}
