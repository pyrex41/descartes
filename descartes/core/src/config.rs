/// Configuration management for Descartes orchestration system.
/// Handles loading, parsing, validation, and migration of .descartes/config.toml
use crate::errors::{AgentError, AgentResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Top-level configuration structure for Descartes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DescaratesConfig {
    /// Configuration file version (for future migrations)
    #[serde(default = "default_version")]
    pub version: String,

    /// Provider-specific settings (OpenAI, Anthropic, Ollama, etc.)
    #[serde(default)]
    pub providers: ProvidersConfig,

    /// Agent behavior and execution settings
    #[serde(default)]
    pub agent: AgentBehaviorConfig,

    /// Storage and persistence settings
    #[serde(default)]
    pub storage: StorageConfig,

    /// Security and encryption settings
    #[serde(default)]
    pub security: SecurityConfig,

    /// Notification and alerting settings
    #[serde(default)]
    pub notifications: NotificationsConfig,

    /// Feature flags and experimental options
    #[serde(default)]
    pub features: FeaturesConfig,

    /// Logging and observability settings
    #[serde(default)]
    pub logging: LoggingConfig,
}

impl Default for DescaratesConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            providers: ProvidersConfig::default(),
            agent: AgentBehaviorConfig::default(),
            storage: StorageConfig::default(),
            security: SecurityConfig::default(),
            notifications: NotificationsConfig::default(),
            features: FeaturesConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

fn default_version() -> String {
    "1.0.0".to_string()
}

/// Provider configuration settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvidersConfig {
    /// Default provider to use
    #[serde(default = "default_primary_provider")]
    pub primary: String,

    /// OpenAI provider settings
    #[serde(default)]
    pub openai: OpenAiConfig,

    /// Anthropic provider settings
    #[serde(default)]
    pub anthropic: AnthropicConfig,

    /// Ollama local provider settings
    #[serde(default)]
    pub ollama: OllamaConfig,

    /// DeepSeek provider settings
    #[serde(default)]
    pub deepseek: DeepSeekConfig,

    /// Groq provider settings
    #[serde(default)]
    pub groq: GroqConfig,

    /// Custom provider endpoints (for proxy, self-hosted, etc.)
    #[serde(default)]
    pub custom: HashMap<String, CustomProviderConfig>,
}

fn default_primary_provider() -> String {
    "anthropic".to_string()
}

impl Default for ProvidersConfig {
    fn default() -> Self {
        Self {
            primary: default_primary_provider(),
            openai: OpenAiConfig::default(),
            anthropic: AnthropicConfig::default(),
            ollama: OllamaConfig::default(),
            deepseek: DeepSeekConfig::default(),
            groq: GroqConfig::default(),
            custom: HashMap::new(),
        }
    }
}

/// OpenAI provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAiConfig {
    /// Whether OpenAI provider is enabled
    #[serde(default)]
    pub enabled: bool,

    /// API key (can be read from env: OPENAI_API_KEY)
    #[serde(default)]
    pub api_key: Option<String>,

    /// API endpoint (defaults to OpenAI's endpoint)
    #[serde(default = "default_openai_endpoint")]
    pub endpoint: String,

    /// Default model to use
    #[serde(default = "default_openai_model")]
    pub model: String,

    /// Available models
    #[serde(default = "default_openai_models")]
    pub models: Vec<String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Max retries on transient failures
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry backoff in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,

    /// Rate limit (requests per minute)
    #[serde(default = "default_rate_limit")]
    pub rate_limit_rpm: u32,

    /// Temperature for generation (0.0 to 2.0)
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Max tokens for generation
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

impl Default for OpenAiConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            endpoint: default_openai_endpoint(),
            model: default_openai_model(),
            models: default_openai_models(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            rate_limit_rpm: default_rate_limit(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_openai_endpoint() -> String {
    "https://api.openai.com/v1".to_string()
}

fn default_openai_model() -> String {
    "gpt-4-turbo".to_string()
}

fn default_openai_models() -> Vec<String> {
    vec![
        "gpt-4".to_string(),
        "gpt-4-turbo".to_string(),
        "gpt-3.5-turbo".to_string(),
    ]
}

/// Anthropic provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// Whether Anthropic provider is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// API key (can be read from env: ANTHROPIC_API_KEY)
    #[serde(default)]
    pub api_key: Option<String>,

    /// API endpoint
    #[serde(default = "default_anthropic_endpoint")]
    pub endpoint: String,

    /// Default model to use
    #[serde(default = "default_anthropic_model")]
    pub model: String,

    /// Available models
    #[serde(default = "default_anthropic_models")]
    pub models: Vec<String>,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Max retries on transient failures
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry backoff in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,

    /// Rate limit (requests per minute)
    #[serde(default = "default_rate_limit")]
    pub rate_limit_rpm: u32,

    /// Temperature for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Max tokens for generation
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: None,
            endpoint: default_anthropic_endpoint(),
            model: default_anthropic_model(),
            models: default_anthropic_models(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            rate_limit_rpm: default_rate_limit(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_anthropic_endpoint() -> String {
    "https://api.anthropic.com".to_string()
}

fn default_anthropic_model() -> String {
    "claude-3-5-sonnet-20241022".to_string()
}

fn default_anthropic_models() -> Vec<String> {
    vec![
        "claude-3-5-sonnet-20241022".to_string(),
        "claude-3-opus-20240229".to_string(),
        "claude-3-haiku-20240307".to_string(),
    ]
}

/// Ollama local provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Whether Ollama provider is enabled
    #[serde(default)]
    pub enabled: bool,

    /// Ollama server endpoint
    #[serde(default = "default_ollama_endpoint")]
    pub endpoint: String,

    /// Default model to use
    #[serde(default = "default_ollama_model")]
    pub model: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Max retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry backoff in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,

    /// Temperature for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Max tokens for generation
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            endpoint: default_ollama_endpoint(),
            model: default_ollama_model(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_ollama_endpoint() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "llama2".to_string()
}

/// DeepSeek provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeepSeekConfig {
    /// Whether DeepSeek provider is enabled
    #[serde(default)]
    pub enabled: bool,

    /// API key
    #[serde(default)]
    pub api_key: Option<String>,

    /// API endpoint
    #[serde(default = "default_deepseek_endpoint")]
    pub endpoint: String,

    /// Default model to use
    #[serde(default = "default_deepseek_model")]
    pub model: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Max retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry backoff in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,

    /// Temperature for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Max tokens for generation
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

impl Default for DeepSeekConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            endpoint: default_deepseek_endpoint(),
            model: default_deepseek_model(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_deepseek_endpoint() -> String {
    "https://api.deepseek.com/v1".to_string()
}

fn default_deepseek_model() -> String {
    "deepseek-chat".to_string()
}

/// Groq provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroqConfig {
    /// Whether Groq provider is enabled
    #[serde(default)]
    pub enabled: bool,

    /// API key
    #[serde(default)]
    pub api_key: Option<String>,

    /// API endpoint
    #[serde(default = "default_groq_endpoint")]
    pub endpoint: String,

    /// Default model to use
    #[serde(default = "default_groq_model")]
    pub model: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Max retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Retry backoff in milliseconds
    #[serde(default = "default_retry_backoff_ms")]
    pub retry_backoff_ms: u64,

    /// Temperature for generation
    #[serde(default = "default_temperature")]
    pub temperature: f32,

    /// Max tokens for generation
    #[serde(default = "default_max_tokens")]
    pub max_tokens: usize,
}

impl Default for GroqConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_key: None,
            endpoint: default_groq_endpoint(),
            model: default_groq_model(),
            timeout_secs: default_timeout(),
            max_retries: default_max_retries(),
            retry_backoff_ms: default_retry_backoff_ms(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
        }
    }
}

fn default_groq_endpoint() -> String {
    "https://api.groq.com/openai/v1".to_string()
}

fn default_groq_model() -> String {
    "mixtral-8x7b-32768".to_string()
}

/// Custom provider configuration (for proxies, self-hosted, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomProviderConfig {
    /// API endpoint URL
    pub endpoint: String,

    /// API key (if required)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Default model
    pub model: String,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,

    /// Whether to use Bearer token auth
    #[serde(default = "default_true")]
    pub use_bearer_auth: bool,

    /// Custom headers as key-value pairs
    #[serde(default)]
    pub custom_headers: HashMap<String, String>,
}

// Common default values
fn default_timeout() -> u64 {
    120
}

fn default_max_retries() -> u32 {
    3
}

fn default_retry_backoff_ms() -> u64 {
    1000
}

fn default_rate_limit() -> u32 {
    60
}

fn default_temperature() -> f32 {
    0.7
}

fn default_max_tokens() -> usize {
    4096
}

/// Agent behavior configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentBehaviorConfig {
    /// Default agent execution timeout in seconds
    #[serde(default = "default_agent_timeout")]
    pub default_timeout_secs: u64,

    /// Maximum concurrent agents
    #[serde(default = "default_max_agents")]
    pub max_concurrent_agents: usize,

    /// Task queue size
    #[serde(default = "default_queue_size")]
    pub task_queue_size: usize,

    /// Enable agent streaming responses
    #[serde(default = "default_true")]
    pub enable_streaming: bool,

    /// Enable tool use (function calling)
    #[serde(default = "default_true")]
    pub enable_tools: bool,

    /// Maximum tool call retries
    #[serde(default = "default_tool_retries")]
    pub max_tool_retries: u32,

    /// Tool execution timeout in seconds
    #[serde(default = "default_tool_timeout")]
    pub tool_timeout_secs: u64,

    /// Enable agent memory/context caching
    #[serde(default = "default_true")]
    pub enable_memory: bool,

    /// Context caching TTL in seconds
    #[serde(default = "default_cache_ttl")]
    pub memory_ttl_secs: u64,

    /// Maximum context size (in tokens)
    #[serde(default = "default_context_size")]
    pub max_context_tokens: usize,

    /// Enable background task processing
    #[serde(default = "default_true")]
    pub enable_background_tasks: bool,

    /// Background worker thread count
    #[serde(default = "default_worker_threads")]
    pub worker_threads: usize,
}

impl Default for AgentBehaviorConfig {
    fn default() -> Self {
        Self {
            default_timeout_secs: default_agent_timeout(),
            max_concurrent_agents: default_max_agents(),
            task_queue_size: default_queue_size(),
            enable_streaming: true,
            enable_tools: true,
            max_tool_retries: default_tool_retries(),
            tool_timeout_secs: default_tool_timeout(),
            enable_memory: true,
            memory_ttl_secs: default_cache_ttl(),
            max_context_tokens: default_context_size(),
            enable_background_tasks: true,
            worker_threads: default_worker_threads(),
        }
    }
}

fn default_agent_timeout() -> u64 {
    300
}

fn default_max_agents() -> usize {
    10
}

fn default_queue_size() -> usize {
    1000
}

fn default_tool_retries() -> u32 {
    2
}

fn default_tool_timeout() -> u64 {
    60
}

fn default_cache_ttl() -> u64 {
    3600
}

fn default_context_size() -> usize {
    32000
}

fn default_worker_threads() -> usize {
    4
}

/// Storage and persistence configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Base storage directory (defaults to ~/.descartes)
    #[serde(default = "default_storage_path")]
    pub base_path: String,

    /// Database configuration
    #[serde(default)]
    pub database: DatabaseConfig,

    /// State store configuration
    #[serde(default)]
    pub state_store: StateStoreConfig,

    /// Event store configuration
    #[serde(default)]
    pub event_store: EventStoreConfig,

    /// Cache configuration
    #[serde(default)]
    pub cache: CacheConfig,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            base_path: default_storage_path(),
            database: DatabaseConfig::default(),
            state_store: StateStoreConfig::default(),
            event_store: EventStoreConfig::default(),
            cache: CacheConfig::default(),
        }
    }
}

fn default_storage_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{}/.descartes", home)
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database type (sqlite, postgres, mysql)
    #[serde(default = "default_db_type")]
    pub database_type: String,

    /// SQLite database path (relative to storage.base_path)
    #[serde(default = "default_db_path")]
    pub sqlite_path: String,

    /// PostgreSQL connection string
    #[serde(default)]
    pub postgres_url: Option<String>,

    /// Connection pool size
    #[serde(default = "default_pool_size")]
    pub pool_size: u32,

    /// Enable database migrations on startup
    #[serde(default = "default_true")]
    pub auto_migrate: bool,

    /// Enable database backups
    #[serde(default = "default_true")]
    pub enable_backups: bool,

    /// Backup interval in hours
    #[serde(default = "default_backup_interval")]
    pub backup_interval_hours: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            database_type: default_db_type(),
            sqlite_path: default_db_path(),
            postgres_url: None,
            pool_size: default_pool_size(),
            auto_migrate: true,
            enable_backups: true,
            backup_interval_hours: default_backup_interval(),
        }
    }
}

fn default_db_type() -> String {
    "sqlite".to_string()
}

fn default_db_path() -> String {
    "data/descartes.db".to_string()
}

fn default_pool_size() -> u32 {
    10
}

fn default_backup_interval() -> u32 {
    24
}

/// State store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateStoreConfig {
    /// Enable state persistence
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// State directory (relative to storage.base_path)
    #[serde(default = "default_state_path")]
    pub path: String,

    /// State serialization format (json, msgpack, bincode)
    #[serde(default = "default_serialization_format")]
    pub serialization_format: String,

    /// Enable state compression
    #[serde(default)]
    pub enable_compression: bool,
}

impl Default for StateStoreConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_state_path(),
            serialization_format: default_serialization_format(),
            enable_compression: false,
        }
    }
}

fn default_state_path() -> String {
    "data/state".to_string()
}

fn default_serialization_format() -> String {
    "json".to_string()
}

/// Event store configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventStoreConfig {
    /// Enable event persistence
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Event directory (relative to storage.base_path)
    #[serde(default = "default_event_path")]
    pub path: String,

    /// Event retention days (0 = infinite)
    #[serde(default)]
    pub retention_days: u32,

    /// Event batch size for writes
    #[serde(default = "default_event_batch_size")]
    pub batch_size: usize,

    /// Enable event indexing for search
    #[serde(default = "default_true")]
    pub enable_indexing: bool,
}

impl Default for EventStoreConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            path: default_event_path(),
            retention_days: 0,
            batch_size: default_event_batch_size(),
            enable_indexing: true,
        }
    }
}

fn default_event_path() -> String {
    "data/events".to_string()
}

fn default_event_batch_size() -> usize {
    100
}

/// Cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Cache type (in-memory, redis, disk)
    #[serde(default = "default_cache_type")]
    pub cache_type: String,

    /// Cache directory (for disk cache)
    #[serde(default = "default_cache_path")]
    pub disk_path: String,

    /// Redis connection URL (for redis cache)
    #[serde(default)]
    pub redis_url: Option<String>,

    /// Cache TTL in seconds
    #[serde(default = "default_cache_ttl_secs")]
    pub ttl_secs: u64,

    /// Max cache size in MB
    #[serde(default = "default_max_cache_size")]
    pub max_size_mb: u64,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_type: default_cache_type(),
            disk_path: default_cache_path(),
            redis_url: None,
            ttl_secs: default_cache_ttl_secs(),
            max_size_mb: default_max_cache_size(),
        }
    }
}

fn default_cache_type() -> String {
    "in-memory".to_string()
}

fn default_cache_path() -> String {
    "data/cache".to_string()
}

fn default_cache_ttl_secs() -> u64 {
    3600
}

fn default_max_cache_size() -> u64 {
    512
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityConfig {
    /// Enable encryption for sensitive data
    #[serde(default = "default_true")]
    pub enable_encryption: bool,

    /// Encryption algorithm (aes-256-gcm, etc.)
    #[serde(default = "default_encryption_algo")]
    pub encryption_algorithm: String,

    /// Encryption key (can be read from env: DESCARTES_ENCRYPTION_KEY)
    #[serde(default)]
    pub encryption_key: Option<String>,

    /// Enable field-level encryption
    #[serde(default)]
    pub encrypt_api_keys: bool,

    /// File permissions for sensitive files (octal)
    #[serde(default = "default_file_permissions")]
    pub file_permissions: String,

    /// Enable RBAC (Role-Based Access Control)
    #[serde(default)]
    pub enable_rbac: bool,

    /// Secret key for session management
    #[serde(default)]
    pub secret_key: Option<String>,

    /// Session timeout in seconds
    #[serde(default = "default_session_timeout")]
    pub session_timeout_secs: u64,

    /// Enable audit logging
    #[serde(default = "default_true")]
    pub enable_audit_logging: bool,

    /// Audit log path
    #[serde(default = "default_audit_path")]
    pub audit_log_path: String,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_encryption: true,
            encryption_algorithm: default_encryption_algo(),
            encryption_key: None,
            encrypt_api_keys: true,
            file_permissions: default_file_permissions(),
            enable_rbac: false,
            secret_key: None,
            session_timeout_secs: default_session_timeout(),
            enable_audit_logging: true,
            audit_log_path: default_audit_path(),
        }
    }
}

fn default_encryption_algo() -> String {
    "aes-256-gcm".to_string()
}

fn default_file_permissions() -> String {
    "0600".to_string()
}

fn default_session_timeout() -> u64 {
    3600
}

fn default_audit_path() -> String {
    "data/audit.log".to_string()
}

/// Notifications and alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationsConfig {
    /// Enable notifications
    #[serde(default)]
    pub enabled: bool,

    /// Notification channels
    #[serde(default)]
    pub channels: NotificationChannels,

    /// Alert thresholds
    #[serde(default)]
    pub alerts: AlertConfig,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            channels: NotificationChannels::default(),
            alerts: AlertConfig::default(),
        }
    }
}

/// Notification channels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationChannels {
    /// Telegram notifications
    #[serde(default)]
    pub telegram: Option<TelegramConfig>,

    /// Webhook notifications
    #[serde(default)]
    pub webhooks: Option<Vec<WebhookConfig>>,

    /// Email notifications
    #[serde(default)]
    pub email: Option<EmailConfig>,

    /// Slack notifications
    #[serde(default)]
    pub slack: Option<SlackConfig>,
}

impl Default for NotificationChannels {
    fn default() -> Self {
        Self {
            telegram: None,
            webhooks: None,
            email: None,
            slack: None,
        }
    }
}

/// Telegram notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    /// Telegram bot token
    pub bot_token: String,

    /// Telegram chat ID
    pub chat_id: String,

    /// Enable Telegram notifications
    #[serde(default = "default_true")]
    pub enabled: bool,
}

/// Webhook notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook name/identifier
    pub name: String,

    /// Webhook URL
    pub url: String,

    /// Webhook HTTP method
    #[serde(default = "default_webhook_method")]
    pub method: String,

    /// Events to trigger on
    pub events: Vec<String>,

    /// Custom headers
    #[serde(default)]
    pub headers: HashMap<String, String>,

    /// Enable retries
    #[serde(default = "default_true")]
    pub enable_retries: bool,

    /// Max retries
    #[serde(default = "default_webhook_retries")]
    pub max_retries: u32,
}

fn default_webhook_method() -> String {
    "POST".to_string()
}

fn default_webhook_retries() -> u32 {
    3
}

/// Email notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailConfig {
    /// SMTP server
    pub smtp_server: String,

    /// SMTP port
    #[serde(default = "default_smtp_port")]
    pub smtp_port: u16,

    /// SMTP username
    pub smtp_user: String,

    /// SMTP password
    pub smtp_password: String,

    /// From email address
    pub from_address: String,

    /// List of recipient emails
    pub recipients: Vec<String>,

    /// Enable TLS
    #[serde(default = "default_true")]
    pub use_tls: bool,

    /// Enable notifications
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_smtp_port() -> u16 {
    587
}

/// Slack notification configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackConfig {
    /// Slack webhook URL
    pub webhook_url: String,

    /// Slack channel
    pub channel: String,

    /// Bot username
    #[serde(default = "default_slack_username")]
    pub username: String,

    /// Enable Slack notifications
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_slack_username() -> String {
    "Descartes".to_string()
}

/// Alert configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertConfig {
    /// Alert on high token usage (threshold percentage, 0-100)
    #[serde(default = "default_alert_threshold")]
    pub high_token_usage_threshold: u32,

    /// Alert on API errors
    #[serde(default = "default_true")]
    pub alert_on_api_errors: bool,

    /// Alert on agent failures
    #[serde(default = "default_true")]
    pub alert_on_agent_failures: bool,

    /// Alert on timeouts
    #[serde(default = "default_true")]
    pub alert_on_timeouts: bool,

    /// Alert on rate limiting
    #[serde(default = "default_true")]
    pub alert_on_rate_limit: bool,
}

impl Default for AlertConfig {
    fn default() -> Self {
        Self {
            high_token_usage_threshold: default_alert_threshold(),
            alert_on_api_errors: true,
            alert_on_agent_failures: true,
            alert_on_timeouts: true,
            alert_on_rate_limit: true,
        }
    }
}

fn default_alert_threshold() -> u32 {
    80
}

/// Feature flags and experimental options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturesConfig {
    /// Enable experimental features
    #[serde(default)]
    pub enable_experimental: bool,

    /// Enable debug mode
    #[serde(default)]
    pub enable_debug: bool,

    /// Enable tracing
    #[serde(default = "default_true")]
    pub enable_tracing: bool,

    /// Enable profiling
    #[serde(default)]
    pub enable_profiling: bool,

    /// Feature flags as key-value pairs
    #[serde(default)]
    pub flags: HashMap<String, bool>,

    /// Beta features to enable
    #[serde(default)]
    pub beta_features: Vec<String>,
}

impl Default for FeaturesConfig {
    fn default() -> Self {
        Self {
            enable_experimental: false,
            enable_debug: false,
            enable_tracing: true,
            enable_profiling: false,
            flags: HashMap::new(),
            beta_features: Vec::new(),
        }
    }
}

/// Logging and observability configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Log format (json, text, pretty)
    #[serde(default = "default_log_format")]
    pub format: String,

    /// Log output targets
    #[serde(default)]
    pub targets: LogTargets,

    /// Enable request logging
    #[serde(default = "default_true")]
    pub log_requests: bool,

    /// Enable response logging
    #[serde(default = "default_true")]
    pub log_responses: bool,

    /// Log sensitive data (API keys, etc.)
    #[serde(default)]
    pub log_sensitive_data: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            format: default_log_format(),
            targets: LogTargets::default(),
            log_requests: true,
            log_responses: true,
            log_sensitive_data: false,
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

fn default_log_format() -> String {
    "text".to_string()
}

/// Log output targets
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogTargets {
    /// Enable stdout logging
    #[serde(default = "default_true")]
    pub stdout: bool,

    /// Enable file logging
    #[serde(default)]
    pub file: Option<LogFileConfig>,

    /// Enable syslog
    #[serde(default)]
    pub syslog: bool,
}

impl Default for LogTargets {
    fn default() -> Self {
        Self {
            stdout: true,
            file: None,
            syslog: false,
        }
    }
}

/// Log file configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileConfig {
    /// Log file path (relative to storage.base_path)
    pub path: String,

    /// Max log file size in MB (for rotation)
    #[serde(default = "default_max_log_size")]
    pub max_size_mb: u64,

    /// Max log file backups to keep
    #[serde(default = "default_max_backups")]
    pub max_backups: u32,

    /// Log file compression
    #[serde(default)]
    pub compress: bool,
}

fn default_max_log_size() -> u64 {
    100
}

fn default_max_backups() -> u32 {
    10
}

/// Configuration loader and manager
pub struct ConfigManager {
    config: DescaratesConfig,
    config_path: PathBuf,
}

impl ConfigManager {
    /// Load configuration from file or use defaults
    pub fn load(config_path: Option<&Path>) -> AgentResult<Self> {
        let path = if let Some(p) = config_path {
            p.to_path_buf()
        } else {
            // Look for .descartes/config.toml in current dir or home
            if let Ok(home) = std::env::var("HOME") {
                let default_path = PathBuf::from(home).join(".descartes/config.toml");
                if default_path.exists() {
                    default_path
                } else {
                    PathBuf::from(".descartes/config.toml")
                }
            } else {
                PathBuf::from(".descartes/config.toml")
            }
        };

        let config = if path.exists() {
            info!("Loading config from {:?}", path);
            let content = std::fs::read_to_string(&path).map_err(|e| {
                AgentError::ExecutionError(format!("Failed to read config file: {}", e))
            })?;
            toml::from_str(&content).map_err(|e| {
                AgentError::ExecutionError(format!("Failed to parse config file: {}", e))
            })?
        } else {
            warn!("Config file not found at {:?}, using defaults", path);
            DescaratesConfig::default()
        };

        debug!("Configuration loaded successfully");
        Ok(ConfigManager {
            config,
            config_path: path,
        })
    }

    /// Get configuration reference
    pub fn config(&self) -> &DescaratesConfig {
        &self.config
    }

    /// Get mutable configuration reference
    pub fn config_mut(&mut self) -> &mut DescaratesConfig {
        &mut self.config
    }

    /// Save configuration to file
    pub fn save(&self) -> AgentResult<()> {
        // Create directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AgentError::ExecutionError(format!("Failed to create config directory: {}", e))
            })?;
        }

        let content = toml::to_string_pretty(&self.config).map_err(|e| {
            AgentError::ExecutionError(format!("Failed to serialize config: {}", e))
        })?;

        std::fs::write(&self.config_path, content).map_err(|e| {
            AgentError::ExecutionError(format!("Failed to write config file: {}", e))
        })?;

        info!("Configuration saved to {:?}", self.config_path);
        Ok(())
    }

    /// Validate configuration
    pub fn validate(&self) -> AgentResult<()> {
        // Validate provider settings
        if !self.config.providers.openai.api_key.is_some()
            && !self.config.providers.anthropic.api_key.is_some()
            && !self.config.providers.ollama.enabled
            && self.config.providers.custom.is_empty()
        {
            warn!("No providers configured - ensure at least one provider is enabled");
        }

        // Validate storage paths
        if self.config.storage.database.pool_size == 0 {
            return Err(AgentError::ExecutionError(
                "Database pool size must be greater than 0".to_string(),
            ));
        }

        // Validate agent settings
        if self.config.agent.max_concurrent_agents == 0 {
            return Err(AgentError::ExecutionError(
                "Max concurrent agents must be greater than 0".to_string(),
            ));
        }

        // Validate temperature values
        if self.config.providers.openai.temperature < 0.0
            || self.config.providers.openai.temperature > 2.0
        {
            return Err(AgentError::ExecutionError(
                "Temperature must be between 0.0 and 2.0".to_string(),
            ));
        }

        debug!("Configuration validation passed");
        Ok(())
    }

    /// Load configuration from environment variables
    pub fn load_from_env(&mut self) -> AgentResult<()> {
        // Load API keys from environment
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            self.config.providers.openai.api_key = Some(key);
            self.config.providers.openai.enabled = true;
        }

        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            self.config.providers.anthropic.api_key = Some(key);
        }

        if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            self.config.providers.deepseek.api_key = Some(key);
        }

        if let Ok(key) = std::env::var("GROQ_API_KEY") {
            self.config.providers.groq.api_key = Some(key);
        }

        if let Ok(key) = std::env::var("DESCARTES_ENCRYPTION_KEY") {
            self.config.security.encryption_key = Some(key);
        }

        if let Ok(key) = std::env::var("DESCARTES_SECRET_KEY") {
            self.config.security.secret_key = Some(key);
        }

        info!("Configuration loaded from environment variables");
        Ok(())
    }

    /// Get the configuration file path
    pub fn config_path(&self) -> &Path {
        &self.config_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = DescaratesConfig::default();
        assert_eq!(config.version, "1.0.0");
        assert_eq!(config.providers.primary, "anthropic");
    }

    #[test]
    fn test_anthropic_config_defaults() {
        let config = AnthropicConfig::default();
        assert!(config.enabled);
        assert_eq!(config.timeout_secs, 120);
    }

    #[test]
    fn test_agent_behavior_defaults() {
        let config = AgentBehaviorConfig::default();
        assert!(config.enable_streaming);
        assert!(config.enable_tools);
        assert!(config.enable_memory);
    }

    #[test]
    fn test_validation_pool_size() {
        let mut config = DescaratesConfig::default();
        config.storage.database.pool_size = 0;

        let manager = ConfigManager {
            config,
            config_path: PathBuf::from("/tmp/test.toml"),
        };

        assert!(manager.validate().is_err());
    }

    #[test]
    fn test_temperature_validation() {
        let mut config = DescaratesConfig::default();
        config.providers.openai.temperature = 2.5;

        let manager = ConfigManager {
            config,
            config_path: PathBuf::from("/tmp/test.toml"),
        };

        assert!(manager.validate().is_err());
    }
}
