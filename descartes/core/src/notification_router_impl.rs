/// Reference implementation of the NotificationRouter trait.
/// This provides a complete, production-ready notification routing system.
use crate::notifications::*;
use async_trait::async_trait;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Default implementation of NotificationRouter with full feature support.
pub struct DefaultNotificationRouter {
    /// Registered adapters for each channel
    adapters: Arc<RwLock<HashMap<NotificationChannel, Arc<dyn NotificationAdapter>>>>,
    /// Routing rules for automatic channel selection
    routing_rules: Arc<RwLock<Vec<RoutingRule>>>,
    /// Recent notifications for rate limiting
    recent_notifications: Arc<RwLock<VecDeque<NotificationPayload>>>,
    /// Rate limiting configuration
    rate_limit_config: Arc<RwLock<RateLimitConfig>>,
    /// Retry configuration
    retry_config: Arc<RwLock<RetryConfig>>,
    /// Notification statistics
    stats: Arc<RwLock<NotificationStats>>,
}

impl DefaultNotificationRouter {
    /// Create a new notification router with default configuration.
    pub fn new() -> Self {
        Self {
            adapters: Arc::new(RwLock::new(HashMap::new())),
            routing_rules: Arc::new(RwLock::new(Vec::new())),
            recent_notifications: Arc::new(RwLock::new(VecDeque::new())),
            rate_limit_config: Arc::new(RwLock::new(RateLimitConfig::default())),
            retry_config: Arc::new(RwLock::new(RetryConfig::default())),
            stats: Arc::new(RwLock::new(NotificationStats::default())),
        }
    }

    /// Helper method to prune old notifications from the recent queue.
    async fn prune_old_notifications(&self) {
        let config = self.rate_limit_config.read().await;
        let cutoff_time = std::time::SystemTime::now() - config.window_duration;

        let mut recent = self.recent_notifications.write().await;
        while let Some(front) = recent.front() {
            if front.timestamp < cutoff_time {
                recent.pop_front();
            } else {
                break;
            }
        }
    }

    /// Helper method to check if a notification is a duplicate.
    async fn is_duplicate(&self, payload: &NotificationPayload) -> bool {
        let config = self.rate_limit_config.read().await;
        if config.deduplication_window.is_none() {
            return false;
        }

        let dedup_window = config.deduplication_window.unwrap();
        let cutoff_time = std::time::SystemTime::now() - dedup_window;

        let recent = self.recent_notifications.read().await;
        for notif in recent.iter() {
            if notif.timestamp > cutoff_time
                && notif.event_type == payload.event_type
                && notif.title == payload.title
                && notif.source == payload.source
            {
                return true;
            }
        }
        false
    }

    /// Send with retry logic.
    async fn send_with_retry(
        &self,
        adapter: Arc<dyn NotificationAdapter>,
        payload: &NotificationPayload,
    ) -> (bool, usize, Option<String>) {
        let retry_config = self.retry_config.read().await;
        let mut attempts = 0;
        let mut last_error: Option<String> = None;

        for attempt in 0..retry_config.max_attempts {
            attempts += 1;
            match adapter.send(payload).await {
                Ok(()) => return (true, attempts, None),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < retry_config.max_attempts - 1 {
                        let backoff = retry_config.backoff_for_attempt(attempt);
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        }

        (false, attempts, last_error)
    }

    /// Update statistics for a channel.
    async fn update_channel_stats(&self, channel: NotificationChannel, success: bool) {
        let channel_name = channel.to_string();
        let mut stats = self.stats.write().await;

        if success {
            stats.total_sent += 1;
        } else {
            stats.total_failed += 1;
        }

        let channel_stats = stats
            .per_channel
            .entry(channel_name)
            .or_insert_with(ChannelStats::default);

        if success {
            channel_stats.successful += 1;
        } else {
            channel_stats.failed += 1;
        }
    }

    /// Update statistics for an event type.
    async fn update_event_stats(&self, payload: &NotificationPayload) {
        let event_name = payload.event_type.to_string();
        let mut stats = self.stats.write().await;

        let event_stats = stats
            .per_event_type
            .entry(event_name)
            .or_insert_with(EventTypeStats::default);

        event_stats.count += 1;
        // Update average severity (simplified calculation)
        let severity_level = match payload.severity {
            Severity::Info => 1.0,
            Severity::Warning => 2.0,
            Severity::Error => 3.0,
            Severity::Critical => 4.0,
        };

        let total = event_stats.count as f32;
        event_stats.avg_severity_level =
            (event_stats.avg_severity_level * (total - 1.0) + severity_level) / total;
    }
}

impl Default for DefaultNotificationRouter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationRouter for DefaultNotificationRouter {
    async fn register_adapter(&self, adapter: Arc<dyn NotificationAdapter>) -> Result<(), String> {
        // Validate the adapter first
        adapter.validate().await?;

        let channel = adapter.channel();
        let mut adapters = self.adapters.write().await;
        adapters.insert(channel, adapter);

        Ok(())
    }

    async fn unregister_adapter(&self, channel: NotificationChannel) -> Result<(), String> {
        let mut adapters = self.adapters.write().await;
        adapters.remove(&channel);
        Ok(())
    }

    async fn add_routing_rule(&self, rule: RoutingRule) -> Result<(), String> {
        let mut rules = self.routing_rules.write().await;
        rules.push(rule);
        Ok(())
    }

    async fn remove_routing_rule(&self, rule_id: &str) -> Result<(), String> {
        let mut rules = self.routing_rules.write().await;
        rules.retain(|r| r.id != rule_id);
        Ok(())
    }

    async fn update_routing_rule(&self, rule: RoutingRule) -> Result<(), String> {
        let mut rules = self.routing_rules.write().await;
        if let Some(existing) = rules.iter_mut().find(|r| r.id == rule.id) {
            *existing = rule;
            Ok(())
        } else {
            Err(format!("Rule not found: {}", rule.id))
        }
    }

    async fn set_rate_limit(&self, config: RateLimitConfig) -> Result<(), String> {
        let mut rate_limit = self.rate_limit_config.write().await;
        *rate_limit = config;
        Ok(())
    }

    async fn set_retry_config(&self, config: RetryConfig) -> Result<(), String> {
        let mut retry_config = self.retry_config.write().await;
        *retry_config = config;
        Ok(())
    }

    async fn send_notification(
        &self,
        _payload: NotificationPayload,
    ) -> Result<Vec<NotificationSendResult>, String> {
        // Check rate limit
        self.check_rate_limit(&_payload).await?;

        // Check for duplicates
        if self.is_duplicate(&_payload).await {
            return Ok(Vec::new());
        }

        // Find matching routing rules
        let rules = self.routing_rules.read().await;
        let matching_rules: Vec<_> = rules.iter().filter(|r| r.matches(&_payload)).collect();

        let mut channels = Vec::new();
        for rule in matching_rules {
            channels.extend(rule.channels.iter().copied());
        }

        // Remove duplicates
        channels.sort();
        channels.dedup();

        // If no rules matched, don't send
        if channels.is_empty() {
            return Ok(Vec::new());
        }

        self.send_to_channels(_payload, channels).await
    }

    async fn send_to_channels(
        &self,
        payload: NotificationPayload,
        channels: Vec<NotificationChannel>,
    ) -> Result<Vec<NotificationSendResult>, String> {
        let adapters = self.adapters.read().await;
        let mut results = Vec::new();

        for channel in channels {
            if let Some(adapter) = adapters.get(&channel) {
                let (success, attempts, error) =
                    self.send_with_retry(adapter.clone(), &payload).await;

                self.update_channel_stats(channel, success).await;

                results.push(NotificationSendResult {
                    notification_id: payload.id.clone(),
                    channel,
                    success,
                    error,
                    attempts,
                });
            }
        }

        // Record the notification
        let mut recent = self.recent_notifications.write().await;
        recent.push_back(payload.clone());

        // Prune old notifications
        self.prune_old_notifications().await;

        // Update event statistics
        self.update_event_stats(&payload).await;

        Ok(results)
    }

    async fn get_recent_notifications(
        &self,
        since: std::time::Duration,
    ) -> Vec<NotificationPayload> {
        let cutoff_time = std::time::SystemTime::now() - since;
        let recent = self.recent_notifications.read().await;

        recent
            .iter()
            .filter(|n| n.timestamp > cutoff_time)
            .cloned()
            .collect()
    }

    async fn clear_history(&self) -> Result<(), String> {
        let mut recent = self.recent_notifications.write().await;
        recent.clear();
        Ok(())
    }

    async fn get_statistics(&self) -> Result<NotificationStats, String> {
        let stats = self.stats.read().await;
        Ok(stats.clone())
    }

    async fn check_rate_limit(&self, _payload: &NotificationPayload) -> Result<(), String> {
        self.prune_old_notifications().await;

        let config = self.rate_limit_config.read().await;
        let recent = self.recent_notifications.read().await;

        if recent.len() >= config.max_per_window {
            return Err("Rate limit exceeded".to_string());
        }

        Ok(())
    }

    async fn validate_adapters(&self) -> Result<(), String> {
        let adapters = self.adapters.read().await;

        for (channel, adapter) in adapters.iter() {
            adapter
                .validate()
                .await
                .map_err(|e| format!("Adapter validation failed for {}: {}", channel, e))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_creation() {
        let router = DefaultNotificationRouter::new();
        assert_eq!(router.get_statistics().await.unwrap().total_sent, 0);
    }

    #[tokio::test]
    async fn test_rate_limit_configuration() {
        let router = DefaultNotificationRouter::new();
        let config = RateLimitConfig {
            max_per_window: 10,
            window_duration: std::time::Duration::from_secs(60),
            deduplication_window: None,
        };

        router.set_rate_limit(config.clone()).await.unwrap();
        let retrieved = router.rate_limit_config.read().await;
        assert_eq!(retrieved.max_per_window, 10);
    }

    #[tokio::test]
    async fn test_routing_rule_management() {
        let router = DefaultNotificationRouter::new();

        let rule = RoutingRule {
            id: "test-rule-1".to_string(),
            event_types: vec![NotificationEventType::Alert],
            min_severity: Severity::Warning,
            channels: vec![NotificationChannel::Telegram],
            required_tags: vec![],
            enabled: true,
        };

        router.add_routing_rule(rule.clone()).await.unwrap();

        let rules = router.routing_rules.read().await;
        assert_eq!(rules.len(), 1);
        assert_eq!(rules[0].id, "test-rule-1");
    }

    #[test]
    fn test_payload_builder_integration() {
        let payload = NotificationPayload::builder()
            .event_type(NotificationEventType::AgentStarted)
            .severity(Severity::Info)
            .title("Agent Started".to_string())
            .message("Agent is running".to_string())
            .source("agent-system".to_string())
            .tag("startup".to_string())
            .build();

        assert_eq!(payload.event_type, NotificationEventType::AgentStarted);
        assert_eq!(payload.severity, Severity::Info);
        assert!(payload.tags.contains(&"startup".to_string()));
    }
}
