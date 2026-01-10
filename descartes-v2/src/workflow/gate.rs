//! Gate logic for workflow transitions
//!
//! Gates control how transitions between stages are handled:
//! - Auto: Continue immediately
//! - Manual: Wait for explicit approval
//! - Notify: Notify user and wait for response or timeout

use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use super::config::{GateConfig, GateType, TimeoutAction};
use super::notify::{Notification, NotificationChannel, NotificationResponse};
use crate::Result;

/// Result of a gate check
#[derive(Debug, Clone)]
pub enum GateResult {
    /// Approved to continue
    Approved {
        /// How the approval was obtained
        method: ApprovalMethod,
        /// Any message from the approver
        message: Option<String>,
    },
    /// Rejected/cancelled
    Rejected {
        /// Reason for rejection
        reason: String,
    },
    /// Waiting for response
    Waiting {
        /// When the gate was entered
        started: Instant,
        /// When it will timeout (if applicable)
        timeout_at: Option<Instant>,
    },
    /// Edit requested - user wants to modify the handoff
    EditRequested,
    /// Skip this stage
    Skip,
}

/// How an approval was obtained
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMethod {
    /// Auto-approved (no gate)
    Auto,
    /// Approved via CLI interaction
    Cli,
    /// Approved via Telegram response
    Telegram,
    /// Approved via Slack response
    Slack,
    /// Approved via email response
    Email,
    /// Approved due to timeout
    Timeout,
}

/// Gate controller for managing transition approvals
pub struct GateController {
    /// Gate configuration
    config: GateConfig,
    /// Notification channels
    channels: Vec<Box<dyn NotificationChannel>>,
    /// Response receiver
    response_rx: Option<mpsc::Receiver<NotificationResponse>>,
}

impl GateController {
    /// Create a new gate controller
    pub fn new(config: GateConfig, channels: Vec<Box<dyn NotificationChannel>>) -> Self {
        Self {
            config,
            channels,
            response_rx: None,
        }
    }

    /// Check the gate and return the result
    pub async fn check(
        &mut self,
        notification: &Notification,
    ) -> Result<GateResult> {
        match self.config.gate_type {
            GateType::Auto => {
                debug!("Gate is auto, continuing immediately");
                Ok(GateResult::Approved {
                    method: ApprovalMethod::Auto,
                    message: None,
                })
            }
            GateType::Manual => {
                info!("Gate is manual, waiting for explicit approval");
                self.wait_for_approval(notification, None).await
            }
            GateType::Notify => {
                let timeout = self.config.timeout;
                info!("Gate is notify, sending notifications");

                // Send notifications to all configured channels
                self.send_notifications(notification).await?;

                // Wait for response or timeout
                self.wait_for_approval(notification, timeout).await
            }
        }
    }

    /// Send notifications to all configured channels
    async fn send_notifications(&mut self, notification: &Notification) -> Result<()> {
        let (tx, rx) = mpsc::channel(10);
        self.response_rx = Some(rx);

        for channel in &self.channels {
            if let Err(e) = channel.send(notification, tx.clone()).await {
                warn!("Failed to send notification via {}: {}", channel.name(), e);
            } else {
                debug!("Notification sent via {}", channel.name());
            }
        }

        Ok(())
    }

    /// Wait for approval (either explicit or via timeout)
    async fn wait_for_approval(
        &mut self,
        notification: &Notification,
        timeout: Option<Duration>,
    ) -> Result<GateResult> {
        let started = Instant::now();
        let timeout_at = timeout.map(|d| started + d);

        // If we have a response channel, wait for responses
        if let Some(ref mut rx) = self.response_rx {
            loop {
                let remaining = timeout_at.map(|t| {
                    t.checked_duration_since(Instant::now())
                        .unwrap_or(Duration::ZERO)
                });

                // Check for timeout
                if let Some(remaining) = remaining {
                    if remaining.is_zero() {
                        return self.handle_timeout();
                    }
                }

                // Wait for response with timeout
                let wait_duration = remaining.unwrap_or(Duration::from_secs(60));

                match tokio::time::timeout(wait_duration, rx.recv()).await {
                    Ok(Some(response)) => {
                        return self.handle_response(response);
                    }
                    Ok(None) => {
                        // Channel closed
                        if timeout.is_some() {
                            return self.handle_timeout();
                        } else {
                            // Manual gate with no responses - keep waiting
                            continue;
                        }
                    }
                    Err(_) => {
                        // Timeout on recv
                        if timeout_at.is_some() {
                            return self.handle_timeout();
                        }
                        // For manual gates, keep waiting
                    }
                }
            }
        }

        // No response channel - this is a CLI-only flow
        // Return waiting status so CLI can handle it
        Ok(GateResult::Waiting { started, timeout_at })
    }

    /// Handle a response from a notification channel
    fn handle_response(&self, response: NotificationResponse) -> Result<GateResult> {
        match response {
            NotificationResponse::Approve { source, message } => {
                info!("Gate approved via {:?}", source);
                Ok(GateResult::Approved {
                    method: source,
                    message,
                })
            }
            NotificationResponse::Reject { source, reason } => {
                info!("Gate rejected via {:?}: {}", source, reason);
                Ok(GateResult::Rejected { reason })
            }
            NotificationResponse::Edit { source } => {
                info!("Edit requested via {:?}", source);
                Ok(GateResult::EditRequested)
            }
            NotificationResponse::Skip { source } => {
                info!("Skip requested via {:?}", source);
                Ok(GateResult::Skip)
            }
            NotificationResponse::ExtendTimeout { source, duration } => {
                // This should be handled by the caller
                debug!("Timeout extension requested via {:?}: {:?}", source, duration);
                Ok(GateResult::Waiting {
                    started: Instant::now(),
                    timeout_at: Some(Instant::now() + duration),
                })
            }
        }
    }

    /// Handle timeout based on configuration
    fn handle_timeout(&self) -> Result<GateResult> {
        match self.config.timeout_action {
            TimeoutAction::Continue => {
                info!("Gate timed out, auto-continuing");
                Ok(GateResult::Approved {
                    method: ApprovalMethod::Timeout,
                    message: Some("Auto-approved after timeout".to_string()),
                })
            }
            TimeoutAction::Pause => {
                info!("Gate timed out, staying paused");
                Ok(GateResult::Waiting {
                    started: Instant::now(),
                    timeout_at: None, // No more timeout
                })
            }
        }
    }
}

/// CLI gate interaction
pub struct CliGate;

impl CliGate {
    /// Prompt the user for gate approval via CLI
    pub async fn prompt(notification: &Notification) -> Result<GateResult> {
        println!("\n{}", "─".repeat(60));
        println!("📋 {}", notification.title);
        println!("{}", "─".repeat(60));

        if let Some(summary) = &notification.summary {
            println!("\n{}\n", summary);
        }

        if let Some(next_command) = &notification.next_command {
            println!("Next: {}", next_command);
        }

        println!("\n[a]pprove  [e]dit  [s]kip  [r]eject  [?]help");
        print!("> ");

        // Read input
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        let input = input.trim().to_lowercase();

        match input.as_str() {
            "a" | "approve" | "y" | "yes" | "go" | "" => {
                Ok(GateResult::Approved {
                    method: ApprovalMethod::Cli,
                    message: None,
                })
            }
            "e" | "edit" => Ok(GateResult::EditRequested),
            "s" | "skip" => Ok(GateResult::Skip),
            "r" | "reject" | "n" | "no" | "cancel" => {
                Ok(GateResult::Rejected {
                    reason: "User rejected".to_string(),
                })
            }
            _ => {
                println!("Commands:");
                println!("  a/approve/y/yes/go - Continue to next stage");
                println!("  e/edit            - Edit the handoff before continuing");
                println!("  s/skip            - Skip this stage");
                println!("  r/reject/n/no     - Cancel the workflow");
                // Recurse to get valid input
                Box::pin(Self::prompt(notification)).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_result_approved() {
        let result = GateResult::Approved {
            method: ApprovalMethod::Auto,
            message: None,
        };
        assert!(matches!(result, GateResult::Approved { .. }));
    }

    #[test]
    fn test_handle_timeout_continue() {
        let config = GateConfig {
            gate_type: GateType::Notify,
            timeout: Some(Duration::from_secs(60)),
            timeout_action: TimeoutAction::Continue,
            notify: vec![],
            message: None,
        };
        let controller = GateController::new(config, vec![]);
        let result = controller.handle_timeout().unwrap();
        assert!(matches!(
            result,
            GateResult::Approved {
                method: ApprovalMethod::Timeout,
                ..
            }
        ));
    }

    #[test]
    fn test_handle_timeout_pause() {
        let config = GateConfig {
            gate_type: GateType::Notify,
            timeout: Some(Duration::from_secs(60)),
            timeout_action: TimeoutAction::Pause,
            notify: vec![],
            message: None,
        };
        let controller = GateController::new(config, vec![]);
        let result = controller.handle_timeout().unwrap();
        assert!(matches!(result, GateResult::Waiting { .. }));
    }
}
