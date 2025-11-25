/// Notification system for multi-channel alerts and messaging.
/// Supports Telegram, Slack, Email, Webhooks with retry logic, templating, and rate limiting.
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use uuid::Uuid;

/// Severity levels for notifications.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational messages
    Info,
    /// Warning messages
    Warning,
    /// Error messages
    Error,
    /// Critical issues requiring immediate attention
    Critical,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Info => write!(f, "INFO"),
            Severity::Warning => write!(f, "WARNING"),
            Severity::Error => write!(f, "ERROR"),
            Severity::Critical => write!(f, "CRITICAL"),
        }
    }
}

/// Supported notification channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd, Serialize, Deserialize)]
pub enum NotificationChannel {
    /// Telegram bot notifications
    Telegram,
    /// Slack webhook notifications
    Slack,
    /// Email notifications
    Email,
    /// Generic webhook notifications
    Webhook,
}

impl std::fmt::Display for NotificationChannel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationChannel::Telegram => write!(f, "Telegram"),
            NotificationChannel::Slack => write!(f, "Slack"),
            NotificationChannel::Email => write!(f, "Email"),
            NotificationChannel::Webhook => write!(f, "Webhook"),
        }
    }
}

/// Event types for notifications.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NotificationEventType {
    /// Agent started
    AgentStarted,
    /// Agent completed successfully
    AgentCompleted,
    /// Agent failed
    AgentFailed,
    /// Task started
    TaskStarted,
    /// Task completed
    TaskCompleted,
    /// Task failed
    TaskFailed,
    /// High-priority alert
    Alert,
    /// Performance metric or report
    Metric,
    /// Arbitrage opportunity detected
    ArbitrageOpportunity,
    /// Custom event type
    Custom(String),
}

impl std::fmt::Display for NotificationEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationEventType::AgentStarted => write!(f, "AgentStarted"),
            NotificationEventType::AgentCompleted => write!(f, "AgentCompleted"),
            NotificationEventType::AgentFailed => write!(f, "AgentFailed"),
            NotificationEventType::TaskStarted => write!(f, "TaskStarted"),
            NotificationEventType::TaskCompleted => write!(f, "TaskCompleted"),
            NotificationEventType::TaskFailed => write!(f, "TaskFailed"),
            NotificationEventType::Alert => write!(f, "Alert"),
            NotificationEventType::Metric => write!(f, "Metric"),
            NotificationEventType::ArbitrageOpportunity => write!(f, "ArbitrageOpportunity"),
            NotificationEventType::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Message payload for notifications.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// Unique identifier for this notification
    pub id: String,
    /// Event type triggering the notification
    pub event_type: NotificationEventType,
    /// Severity level of the notification
    pub severity: Severity,
    /// Title/subject of the notification
    pub title: String,
    /// Detailed message content
    pub message: String,
    /// Optional structured data
    pub data: HashMap<String, serde_json::Value>,
    /// Timestamp when notification was created
    pub timestamp: SystemTime,
    /// Source/originating component
    pub source: String,
    /// Optional tags for filtering and categorization
    pub tags: Vec<String>,
}

impl Default for NotificationPayload {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_type: NotificationEventType::Custom("default".to_string()),
            severity: Severity::Info,
            title: String::new(),
            message: String::new(),
            data: HashMap::new(),
            timestamp: SystemTime::now(),
            source: "system".to_string(),
            tags: Vec::new(),
        }
    }
}

impl NotificationPayload {
    /// Create a new notification payload builder.
    pub fn builder() -> NotificationPayloadBuilder {
        NotificationPayloadBuilder::default()
    }

    /// Check if this payload matches severity filter.
    pub fn matches_severity(&self, min_severity: Severity) -> bool {
        self.severity >= min_severity
    }

    /// Check if this payload has any of the given tags.
    pub fn matches_tags(&self, required_tags: &[String]) -> bool {
        if required_tags.is_empty() {
            return true;
        }
        required_tags.iter().any(|tag| self.tags.contains(tag))
    }
}

/// Builder for creating NotificationPayload instances.
#[derive(Default, Debug, Clone)]
pub struct NotificationPayloadBuilder {
    id: Option<String>,
    event_type: Option<NotificationEventType>,
    severity: Option<Severity>,
    title: Option<String>,
    message: Option<String>,
    data: HashMap<String, serde_json::Value>,
    timestamp: Option<SystemTime>,
    source: Option<String>,
    tags: Vec<String>,
}

impl NotificationPayloadBuilder {
    /// Set the notification ID.
    pub fn id(mut self, id: String) -> Self {
        self.id = Some(id);
        self
    }

    /// Set the event type.
    pub fn event_type(mut self, event_type: NotificationEventType) -> Self {
        self.event_type = Some(event_type);
        self
    }

    /// Set the severity level.
    pub fn severity(mut self, severity: Severity) -> Self {
        self.severity = Some(severity);
        self
    }

    /// Set the title.
    pub fn title(mut self, title: String) -> Self {
        self.title = Some(title);
        self
    }

    /// Set the message content.
    pub fn message(mut self, message: String) -> Self {
        self.message = Some(message);
        self
    }

    /// Add a data field.
    pub fn data(mut self, key: String, value: serde_json::Value) -> Self {
        self.data.insert(key, value);
        self
    }

    /// Set timestamp.
    pub fn timestamp(mut self, timestamp: SystemTime) -> Self {
        self.timestamp = Some(timestamp);
        self
    }

    /// Set the source component.
    pub fn source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    /// Add a tag.
    pub fn tag(mut self, tag: String) -> Self {
        self.tags.push(tag);
        self
    }

    /// Add multiple tags.
    pub fn tags(mut self, tags: Vec<String>) -> Self {
        self.tags.extend(tags);
        self
    }

    /// Build the NotificationPayload.
    pub fn build(self) -> NotificationPayload {
        NotificationPayload {
            id: self.id.unwrap_or_else(|| Uuid::new_v4().to_string()),
            event_type: self
                .event_type
                .unwrap_or(NotificationEventType::Custom("unknown".to_string())),
            severity: self.severity.unwrap_or(Severity::Info),
            title: self.title.unwrap_or_default(),
            message: self.message.unwrap_or_default(),
            data: self.data,
            timestamp: self.timestamp.unwrap_or_else(SystemTime::now),
            source: self.source.unwrap_or_else(|| "system".to_string()),
            tags: self.tags,
        }
    }
}

/// Template context for message formatting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateContext {
    /// Variables available in the template
    pub variables: HashMap<String, String>,
}

impl TemplateContext {
    /// Create a new template context.
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
        }
    }

    /// Add a variable to the context.
    pub fn add_variable(mut self, key: String, value: String) -> Self {
        self.variables.insert(key, value);
        self
    }

    /// Format a template string with variables.
    /// Supports simple {{variable}} placeholders.
    pub fn format(&self, template: &str) -> String {
        let mut result = template.to_string();
        for (key, value) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }
        result
    }
}

impl Default for TemplateContext {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for notification routing and delivery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRule {
    /// Rule identifier
    pub id: String,
    /// Event types this rule applies to
    pub event_types: Vec<NotificationEventType>,
    /// Minimum severity level to trigger this rule
    pub min_severity: Severity,
    /// Channels to send to
    pub channels: Vec<NotificationChannel>,
    /// Tags that must be present (empty = all match)
    pub required_tags: Vec<String>,
    /// Whether this rule is enabled
    pub enabled: bool,
}

impl Default for RoutingRule {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            event_types: vec![],
            min_severity: Severity::Info,
            channels: vec![],
            required_tags: vec![],
            enabled: true,
        }
    }
}

impl RoutingRule {
    /// Check if a notification matches this routing rule.
    pub fn matches(&self, payload: &NotificationPayload) -> bool {
        if !self.enabled {
            return false;
        }

        // Check severity
        if !payload.matches_severity(self.min_severity) {
            return false;
        }

        // Check event type
        if !self.event_types.is_empty() && !self.event_types.contains(&payload.event_type) {
            return false;
        }

        // Check tags
        if !payload.matches_tags(&self.required_tags) {
            return false;
        }

        true
    }
}

/// Rate limiting configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// Maximum notifications per time window
    pub max_per_window: usize,
    /// Time window duration
    pub window_duration: Duration,
    /// Enable deduplication within this duration
    pub deduplication_window: Option<Duration>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_per_window: 100,
            window_duration: Duration::from_secs(60),
            deduplication_window: Some(Duration::from_secs(300)),
        }
    }
}

/// Result of sending a notification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSendResult {
    /// Notification ID
    pub notification_id: String,
    /// Channel this was sent to
    pub channel: NotificationChannel,
    /// Whether sending succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
    /// Number of retry attempts
    pub attempts: usize,
}

/// Configuration for retries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: usize,
    /// Initial backoff duration
    pub initial_backoff: Duration,
    /// Exponential backoff multiplier
    pub backoff_multiplier: f32,
    /// Maximum backoff duration
    pub max_backoff: Duration,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_backoff: Duration::from_secs(1),
            backoff_multiplier: 2.0,
            max_backoff: Duration::from_secs(60),
        }
    }
}

impl RetryConfig {
    /// Calculate the backoff duration for a given attempt number.
    pub fn backoff_for_attempt(&self, attempt: usize) -> Duration {
        let backoff = Duration::from_secs_f32(
            self.initial_backoff.as_secs_f32() * self.backoff_multiplier.powi(attempt as i32),
        );
        backoff.min(self.max_backoff)
    }
}

/// Trait for notification provider adapters.
/// Implementations handle sending notifications to specific channels.
#[async_trait]
pub trait NotificationAdapter: Send + Sync {
    /// Get the channel this adapter handles.
    fn channel(&self) -> NotificationChannel;

    /// Send a notification through this adapter.
    /// Returns true if successful, false otherwise.
    async fn send(&self, payload: &NotificationPayload) -> Result<(), String>;

    /// Format a message using a template (optional override per adapter).
    fn format_message(
        &self,
        _template: &str,
        _context: &TemplateContext,
    ) -> Result<String, String> {
        Err("Formatting not supported by this adapter".to_string())
    }

    /// Validate that the adapter is properly configured.
    async fn validate(&self) -> Result<(), String>;

    /// Get the name/identifier of this adapter instance.
    fn adapter_name(&self) -> &str;
}

/// Main notification router trait.
/// Orchestrates routing, filtering, retrying, and rate limiting for notifications.
#[async_trait]
pub trait NotificationRouter: Send + Sync {
    /// Register a notification adapter for a specific channel.
    async fn register_adapter(&self, adapter: Arc<dyn NotificationAdapter>) -> Result<(), String>;

    /// Unregister an adapter for a channel.
    async fn unregister_adapter(&self, channel: NotificationChannel) -> Result<(), String>;

    /// Add a routing rule.
    async fn add_routing_rule(&self, rule: RoutingRule) -> Result<(), String>;

    /// Remove a routing rule by ID.
    async fn remove_routing_rule(&self, rule_id: &str) -> Result<(), String>;

    /// Update a routing rule.
    async fn update_routing_rule(&self, rule: RoutingRule) -> Result<(), String>;

    /// Set the rate limiting configuration.
    async fn set_rate_limit(&self, config: RateLimitConfig) -> Result<(), String>;

    /// Set the retry configuration.
    async fn set_retry_config(&self, config: RetryConfig) -> Result<(), String>;

    /// Send a notification through the router.
    /// This will apply routing rules, rate limiting, and retries automatically.
    async fn send_notification(
        &self,
        payload: NotificationPayload,
    ) -> Result<Vec<NotificationSendResult>, String>;

    /// Send a notification to specific channels directly (bypasses routing rules).
    async fn send_to_channels(
        &self,
        payload: NotificationPayload,
        channels: Vec<NotificationChannel>,
    ) -> Result<Vec<NotificationSendResult>, String>;

    /// Get notifications sent in the last duration.
    /// Used for rate limiting checks.
    async fn get_recent_notifications(&self, since: Duration) -> Vec<NotificationPayload>;

    /// Clear the notification history.
    async fn clear_history(&self) -> Result<(), String>;

    /// Get statistics about notifications.
    async fn get_statistics(&self) -> Result<NotificationStats, String>;

    /// Check if a notification would be rate limited.
    async fn check_rate_limit(&self, payload: &NotificationPayload) -> Result<(), String>;

    /// Validate all registered adapters.
    async fn validate_adapters(&self) -> Result<(), String>;
}

/// Statistics about notification delivery.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NotificationStats {
    /// Total notifications sent
    pub total_sent: usize,
    /// Total notifications failed
    pub total_failed: usize,
    /// Statistics per channel
    pub per_channel: HashMap<String, ChannelStats>,
    /// Statistics per event type
    pub per_event_type: HashMap<String, EventTypeStats>,
}

/// Statistics for a specific channel.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ChannelStats {
    /// Number of successful sends
    pub successful: usize,
    /// Number of failed sends
    pub failed: usize,
    /// Average retry attempts per notification
    pub avg_attempts: f32,
}

/// Statistics for a specific event type.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventTypeStats {
    /// Number of notifications sent
    pub count: usize,
    /// Average severity level
    pub avg_severity_level: f32,
}

/// Error types for notification operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NotificationError {
    /// No adapter registered for a channel
    NoAdapterForChannel(NotificationChannel),
    /// Adapter failed to send
    AdapterError(String),
    /// Rate limit exceeded
    RateLimitExceeded,
    /// Invalid configuration
    ConfigError(String),
    /// Template formatting failed
    TemplateError(String),
    /// Maximum retries exceeded
    MaxRetriesExceeded,
    /// Other error
    Other(String),
}

impl std::fmt::Display for NotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NotificationError::NoAdapterForChannel(channel) => {
                write!(f, "No adapter registered for channel: {}", channel)
            }
            NotificationError::AdapterError(msg) => write!(f, "Adapter error: {}", msg),
            NotificationError::RateLimitExceeded => write!(f, "Rate limit exceeded"),
            NotificationError::ConfigError(msg) => write!(f, "Configuration error: {}", msg),
            NotificationError::TemplateError(msg) => write!(f, "Template error: {}", msg),
            NotificationError::MaxRetriesExceeded => write!(f, "Maximum retries exceeded"),
            NotificationError::Other(msg) => write!(f, "Notification error: {}", msg),
        }
    }
}

impl std::error::Error for NotificationError {}

/// Result type for notification operations.
pub type NotificationResult<T> = Result<T, NotificationError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_payload_builder() {
        let payload = NotificationPayload::builder()
            .event_type(NotificationEventType::Alert)
            .severity(Severity::Critical)
            .title("Test Alert".to_string())
            .message("This is a test".to_string())
            .source("test".to_string())
            .tag("urgent".to_string())
            .build();

        assert_eq!(payload.severity, Severity::Critical);
        assert_eq!(payload.title, "Test Alert");
        assert!(payload.tags.contains(&"urgent".to_string()));
    }

    #[test]
    fn test_severity_comparison() {
        assert!(Severity::Critical > Severity::Error);
        assert!(Severity::Error > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
    }

    #[test]
    fn test_routing_rule_matching() {
        let rule = RoutingRule {
            enabled: true,
            min_severity: Severity::Warning,
            event_types: vec![NotificationEventType::Alert],
            ..Default::default()
        };

        let matching_payload = NotificationPayload {
            severity: Severity::Critical,
            event_type: NotificationEventType::Alert,
            ..Default::default()
        };

        let non_matching_payload = NotificationPayload {
            severity: Severity::Info,
            event_type: NotificationEventType::Alert,
            ..Default::default()
        };

        assert!(rule.matches(&matching_payload));
        assert!(!rule.matches(&non_matching_payload));
    }

    #[test]
    fn test_template_formatting() {
        let context = TemplateContext::new()
            .add_variable("name".to_string(), "Alice".to_string())
            .add_variable("status".to_string(), "active".to_string());

        let template = "User {{name}} is {{status}}";
        let formatted = context.format(template);
        assert_eq!(formatted, "User Alice is active");
    }

    #[test]
    fn test_retry_backoff_calculation() {
        let config = RetryConfig::default();
        let backoff_0 = config.backoff_for_attempt(0);
        let backoff_1 = config.backoff_for_attempt(1);
        let backoff_2 = config.backoff_for_attempt(2);

        assert!(backoff_1 > backoff_0);
        assert!(backoff_2 > backoff_1);
        assert!(backoff_2 <= config.max_backoff);
    }

    #[test]
    fn test_rate_limit_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_per_window, 100);
        assert_eq!(config.window_duration, Duration::from_secs(60));
    }

    #[test]
    fn test_notification_channels_display() {
        assert_eq!(NotificationChannel::Telegram.to_string(), "Telegram");
        assert_eq!(NotificationChannel::Slack.to_string(), "Slack");
        assert_eq!(NotificationChannel::Email.to_string(), "Email");
        assert_eq!(NotificationChannel::Webhook.to_string(), "Webhook");
    }

    #[test]
    fn test_severity_display() {
        assert_eq!(Severity::Info.to_string(), "INFO");
        assert_eq!(Severity::Warning.to_string(), "WARNING");
        assert_eq!(Severity::Error.to_string(), "ERROR");
        assert_eq!(Severity::Critical.to_string(), "CRITICAL");
    }

    #[test]
    fn test_payload_tag_matching() {
        let payload = NotificationPayload {
            tags: vec!["urgent".to_string(), "performance".to_string()],
            ..Default::default()
        };

        let required_tags = vec!["urgent".to_string()];
        assert!(payload.matches_tags(&required_tags));

        let non_matching_tags = vec!["debug".to_string()];
        assert!(!payload.matches_tags(&non_matching_tags));

        assert!(payload.matches_tags(&[]));
    }
}
