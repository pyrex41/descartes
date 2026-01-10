//! Notification channels for workflow gates
//!
//! Supports multiple notification methods:
//! - Telegram bot
//! - Slack webhook
//! - Email
//! - Desktop notifications
//! - Log output (for testing)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use super::config::NotificationConfig;
use super::gate::ApprovalMethod;
use crate::{Error, Result};

/// A notification to send to the user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    /// Workflow name
    pub workflow: String,
    /// Current stage
    pub from_stage: String,
    /// Next stage
    pub to_stage: String,
    /// Notification title
    pub title: String,
    /// Summary of what happened
    pub summary: Option<String>,
    /// Next command to run
    pub next_command: Option<String>,
    /// Handoff preview (truncated)
    pub handoff_preview: Option<String>,
    /// Full handoff (for edit operations)
    pub handoff_full: Option<String>,
    /// Timeout if applicable
    pub timeout: Option<Duration>,
}

impl Notification {
    /// Create a new notification
    pub fn new(workflow: &str, from_stage: &str, to_stage: &str) -> Self {
        Self {
            workflow: workflow.to_string(),
            from_stage: from_stage.to_string(),
            to_stage: to_stage.to_string(),
            title: format!("Workflow: {} ({} → {})", workflow, from_stage, to_stage),
            summary: None,
            next_command: None,
            handoff_preview: None,
            handoff_full: None,
            timeout: None,
        }
    }

    /// Set the summary
    pub fn with_summary(mut self, summary: &str) -> Self {
        self.summary = Some(summary.to_string());
        self
    }

    /// Set the next command
    pub fn with_command(mut self, command: &str) -> Self {
        self.next_command = Some(command.to_string());
        self
    }

    /// Set the handoff
    pub fn with_handoff(mut self, handoff: &str) -> Self {
        // Truncate for preview
        let preview = if handoff.len() > 500 {
            format!("{}...", &handoff[..500])
        } else {
            handoff.to_string()
        };
        self.handoff_preview = Some(preview);
        self.handoff_full = Some(handoff.to_string());
        self
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Format for Telegram
    pub fn format_telegram(&self) -> String {
        let mut msg = format!("🔄 *{}*\n", self.title);
        msg.push_str(&format!("Stage: {} → {}\n\n", self.from_stage, self.to_stage));

        if let Some(summary) = &self.summary {
            msg.push_str(&format!("{}\n\n", summary));
        }

        if let Some(cmd) = &self.next_command {
            msg.push_str(&format!("📎 Next: `{}`\n\n", cmd));
        }

        msg.push_str("Reply:\n");
        msg.push_str("• `go` or `y` to continue\n");
        msg.push_str("• `edit` to modify handoff\n");
        msg.push_str("• `wait` to extend timeout\n");
        msg.push_str("• `stop` to pause workflow\n");

        if let Some(timeout) = self.timeout {
            msg.push_str(&format!(
                "\n⏱ Auto-continuing in {}",
                humantime::format_duration(timeout)
            ));
        }

        msg
    }

    /// Format for Slack
    pub fn format_slack(&self) -> serde_json::Value {
        let mut blocks = vec![
            serde_json::json!({
                "type": "header",
                "text": {
                    "type": "plain_text",
                    "text": format!("🔄 {}", self.title)
                }
            }),
            serde_json::json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("*Stage:* {} → {}", self.from_stage, self.to_stage)
                }
            }),
        ];

        if let Some(summary) = &self.summary {
            blocks.push(serde_json::json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": summary
                }
            }));
        }

        if let Some(cmd) = &self.next_command {
            blocks.push(serde_json::json!({
                "type": "section",
                "text": {
                    "type": "mrkdwn",
                    "text": format!("📎 *Next:* `{}`", cmd)
                }
            }));
        }

        blocks.push(serde_json::json!({
            "type": "actions",
            "elements": [
                {
                    "type": "button",
                    "text": { "type": "plain_text", "text": "✅ Continue" },
                    "style": "primary",
                    "value": "approve"
                },
                {
                    "type": "button",
                    "text": { "type": "plain_text", "text": "✏️ Edit" },
                    "value": "edit"
                },
                {
                    "type": "button",
                    "text": { "type": "plain_text", "text": "⏸️ Pause" },
                    "style": "danger",
                    "value": "pause"
                }
            ]
        }));

        serde_json::json!({ "blocks": blocks })
    }
}

/// Response from a notification channel
#[derive(Debug, Clone)]
pub enum NotificationResponse {
    /// Approve and continue
    Approve {
        source: ApprovalMethod,
        message: Option<String>,
    },
    /// Reject/cancel
    Reject {
        source: ApprovalMethod,
        reason: String,
    },
    /// Request to edit handoff
    Edit {
        source: ApprovalMethod,
    },
    /// Skip this stage
    Skip {
        source: ApprovalMethod,
    },
    /// Extend the timeout
    ExtendTimeout {
        source: ApprovalMethod,
        duration: Duration,
    },
}

/// Trait for notification channels
#[async_trait]
pub trait NotificationChannel: Send + Sync {
    /// Channel name
    fn name(&self) -> &str;

    /// Send a notification
    async fn send(
        &self,
        notification: &Notification,
        response_tx: mpsc::Sender<NotificationResponse>,
    ) -> Result<()>;
}

/// Telegram notification channel
pub struct TelegramChannel {
    bot_token: String,
    chat_id: String,
    client: reqwest::Client,
}

impl TelegramChannel {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            bot_token,
            chat_id,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_config(config: &NotificationConfig) -> Option<Self> {
        match config {
            NotificationConfig::Telegram { bot_token, chat_id } => {
                Some(Self::new(
                    Self::resolve_env(bot_token),
                    Self::resolve_env(chat_id),
                ))
            }
            _ => None,
        }
    }

    fn resolve_env(value: &str) -> String {
        if value.starts_with("${") && value.ends_with("}") {
            let var_name = &value[2..value.len() - 1];
            std::env::var(var_name).unwrap_or_else(|_| value.to_string())
        } else {
            value.to_string()
        }
    }
}

#[async_trait]
impl NotificationChannel for TelegramChannel {
    fn name(&self) -> &str {
        "telegram"
    }

    async fn send(
        &self,
        notification: &Notification,
        response_tx: mpsc::Sender<NotificationResponse>,
    ) -> Result<()> {
        let url = format!(
            "https://api.telegram.org/bot{}/sendMessage",
            self.bot_token
        );

        let message = notification.format_telegram();

        let response = self
            .client
            .post(&url)
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "text": message,
                "parse_mode": "Markdown"
            }))
            .send()
            .await
            .map_err(|e| Error::Notification(format!("Telegram send failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Notification(format!("Telegram error: {}", body)));
        }

        info!("Telegram notification sent");

        // TODO: Set up webhook or polling for responses
        // For now, we just send the notification without waiting for response
        // In a full implementation, you'd set up a webhook server or use long polling

        Ok(())
    }
}

/// Slack notification channel
pub struct SlackChannel {
    webhook_url: String,
    channel: Option<String>,
    client: reqwest::Client,
}

impl SlackChannel {
    pub fn new(webhook_url: String, channel: Option<String>) -> Self {
        Self {
            webhook_url,
            channel,
            client: reqwest::Client::new(),
        }
    }

    pub fn from_config(config: &NotificationConfig) -> Option<Self> {
        match config {
            NotificationConfig::Slack { webhook_url, channel } => {
                Some(Self::new(
                    Self::resolve_env(webhook_url),
                    channel.as_ref().map(|c| Self::resolve_env(c)),
                ))
            }
            _ => None,
        }
    }

    fn resolve_env(value: &str) -> String {
        if value.starts_with("${") && value.ends_with("}") {
            let var_name = &value[2..value.len() - 1];
            std::env::var(var_name).unwrap_or_else(|_| value.to_string())
        } else {
            value.to_string()
        }
    }
}

#[async_trait]
impl NotificationChannel for SlackChannel {
    fn name(&self) -> &str {
        "slack"
    }

    async fn send(
        &self,
        notification: &Notification,
        _response_tx: mpsc::Sender<NotificationResponse>,
    ) -> Result<()> {
        let mut payload = notification.format_slack();

        if let Some(channel) = &self.channel {
            payload["channel"] = serde_json::json!(channel);
        }

        let response = self
            .client
            .post(&self.webhook_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| Error::Notification(format!("Slack send failed: {}", e)))?;

        if !response.status().is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(Error::Notification(format!("Slack error: {}", body)));
        }

        info!("Slack notification sent");
        Ok(())
    }
}

/// Log notification channel (for testing/debugging)
pub struct LogChannel;

#[async_trait]
impl NotificationChannel for LogChannel {
    fn name(&self) -> &str {
        "log"
    }

    async fn send(
        &self,
        notification: &Notification,
        _response_tx: mpsc::Sender<NotificationResponse>,
    ) -> Result<()> {
        info!(
            "📬 Notification: {} ({} → {})",
            notification.title, notification.from_stage, notification.to_stage
        );
        if let Some(summary) = &notification.summary {
            debug!("Summary: {}", summary);
        }
        if let Some(cmd) = &notification.next_command {
            debug!("Next command: {}", cmd);
        }
        Ok(())
    }
}

/// Desktop notification channel
pub struct DesktopChannel;

#[async_trait]
impl NotificationChannel for DesktopChannel {
    fn name(&self) -> &str {
        "desktop"
    }

    async fn send(
        &self,
        notification: &Notification,
        _response_tx: mpsc::Sender<NotificationResponse>,
    ) -> Result<()> {
        // Use notify-send on Linux, osascript on macOS
        #[cfg(target_os = "linux")]
        {
            let _ = tokio::process::Command::new("notify-send")
                .arg(&notification.title)
                .arg(notification.summary.as_deref().unwrap_or("Workflow gate"))
                .spawn();
        }

        #[cfg(target_os = "macos")]
        {
            let script = format!(
                "display notification \"{}\" with title \"{}\"",
                notification.summary.as_deref().unwrap_or("Workflow gate"),
                notification.title
            );
            let _ = tokio::process::Command::new("osascript")
                .arg("-e")
                .arg(&script)
                .spawn();
        }

        info!("Desktop notification sent");
        Ok(())
    }
}

/// Create notification channels from configuration
pub fn create_channels(
    configs: &std::collections::HashMap<String, NotificationConfig>,
    enabled: &[String],
) -> Vec<Box<dyn NotificationChannel>> {
    let mut channels: Vec<Box<dyn NotificationChannel>> = Vec::new();

    for name in enabled {
        if let Some(config) = configs.get(name) {
            match config {
                NotificationConfig::Telegram { .. } => {
                    if let Some(channel) = TelegramChannel::from_config(config) {
                        channels.push(Box::new(channel));
                    }
                }
                NotificationConfig::Slack { .. } => {
                    if let Some(channel) = SlackChannel::from_config(config) {
                        channels.push(Box::new(channel));
                    }
                }
                NotificationConfig::Desktop => {
                    channels.push(Box::new(DesktopChannel));
                }
                NotificationConfig::Log => {
                    channels.push(Box::new(LogChannel));
                }
                NotificationConfig::Email { .. } => {
                    // TODO: Implement email channel
                    debug!("Email notifications not yet implemented");
                }
            }
        }
    }

    channels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_format_telegram() {
        let notification = Notification::new("test", "research", "plan")
            .with_summary("Found 3 relevant patterns")
            .with_command("/create_plan")
            .with_timeout(Duration::from_secs(300));

        let formatted = notification.format_telegram();
        assert!(formatted.contains("test"));
        assert!(formatted.contains("research → plan"));
        assert!(formatted.contains("3 relevant patterns"));
        assert!(formatted.contains("/create_plan"));
    }

    #[test]
    fn test_notification_format_slack() {
        let notification = Notification::new("test", "plan", "implement")
            .with_summary("Plan complete")
            .with_command("/implement_plan");

        let formatted = notification.format_slack();
        assert!(formatted.get("blocks").is_some());
    }
}
