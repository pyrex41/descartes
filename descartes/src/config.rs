//! Configuration loading and management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Harness configuration
    #[serde(default)]
    pub harness: HarnessConfig,

    /// Agent category configurations
    #[serde(default)]
    pub categories: HashMap<String, CategoryConfig>,

    /// Agent definition configuration
    #[serde(default)]
    pub agents: AgentsConfig,

    /// Swarm orchestration settings
    #[serde(default)]
    pub swarm: SwarmConfig,

    /// SCUD integration settings
    #[serde(default)]
    pub scud: ScudConfig,

    /// Transcript settings
    #[serde(default)]
    pub transcripts: TranscriptConfig,

    /// User guidance configuration
    #[serde(default)]
    pub guidance: GuidanceConfig,

    /// Path to prompts directory
    #[serde(default = "default_prompts_dir")]
    pub prompts_dir: PathBuf,
}

fn default_prompts_dir() -> PathBuf {
    PathBuf::from("prompts")
}

/// Swarm orchestration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwarmConfig {
    /// Whether to try fast-builder first for applicable tasks
    #[serde(default)]
    pub use_fast_first: bool,

    /// Whether to always review fast-builder changes
    #[serde(default)]
    pub always_review: bool,

    /// Heuristic for orchestration decisions
    #[serde(default = "default_heuristic")]
    pub heuristic: String,
}

fn default_heuristic() -> String {
    "prefer_speed".to_string()
}

impl Default for SwarmConfig {
    fn default() -> Self {
        Self {
            use_fast_first: true,
            always_review: false,
            heuristic: default_heuristic(),
        }
    }
}

/// Agent definition configuration
///
/// Controls loading of agent definitions from `.descartes/agents/<name>/AGENT.md`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsConfig {
    /// Directory containing agent definitions
    #[serde(default = "default_agents_dir")]
    pub directory: PathBuf,

    /// Explicitly enabled agents (if set, only these are available)
    /// Use either 'enabled' OR 'disabled', not both
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabled: Option<Vec<String>>,

    /// Explicitly disabled agents (if set, all others are available)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub disabled: Option<Vec<String>>,
}

fn default_agents_dir() -> PathBuf {
    PathBuf::from(".descartes/agents")
}

impl Default for AgentsConfig {
    fn default() -> Self {
        Self {
            directory: default_agents_dir(),
            enabled: None,
            disabled: None,
        }
    }
}

// Environment variable helpers for category defaults
fn default_fast_model() -> String {
    std::env::var("DESCARTES_FAST_MODEL").unwrap_or_else(|_| "xai/grok-code-fast-1".to_string())
}

fn default_smart_model() -> String {
    std::env::var("DESCARTES_SMART_MODEL").unwrap_or_else(|_| "opus".to_string())
}

impl Default for Config {
    fn default() -> Self {
        let mut categories = HashMap::new();

        // Fast categories use opencode (default harness) with grok-code-fast-1
        categories.insert(
            "searcher".to_string(),
            CategoryConfig {
                description: "Fast parallel code search".to_string(),
                model: default_fast_model(),
                harness: None, // uses global default (opencode)
                tools: vec!["read".to_string(), "bash".to_string()],
                parallel: true,
                backpressure: false,
                prompt_template: None,
            },
        );

        categories.insert(
            "analyzer".to_string(),
            CategoryConfig {
                description: "Deep code analysis".to_string(),
                model: default_fast_model(),
                harness: None,
                tools: vec!["read".to_string()],
                parallel: true,
                backpressure: false,
                prompt_template: None,
            },
        );

        // Smart categories use claude-code with opus
        categories.insert(
            "builder".to_string(),
            CategoryConfig {
                description: "Code implementation".to_string(),
                model: default_smart_model(),
                harness: Some("claude-code".to_string()), // smart tasks use claude-code
                tools: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "edit".to_string(),
                    "bash".to_string(),
                ],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
        );

        categories.insert(
            "validator".to_string(),
            CategoryConfig {
                description: "Test runner (backpressure gate)".to_string(),
                model: default_fast_model(),
                harness: None, // fast
                tools: vec!["bash".to_string()],
                parallel: false,
                backpressure: true,
                prompt_template: None,
            },
        );

        categories.insert(
            "planner".to_string(),
            CategoryConfig {
                description: "Task planning and breakdown".to_string(),
                model: default_smart_model(),
                harness: Some("claude-code".to_string()), // smart tasks use claude-code
                tools: vec!["read".to_string(), "bash".to_string()],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
        );

        categories.insert(
            "fast-builder".to_string(),
            CategoryConfig {
                description: "Fast code implementation via OpenCode".to_string(),
                model: default_fast_model(),
                harness: None, // uses global default (opencode)
                tools: vec![
                    "read".to_string(),
                    "write".to_string(),
                    "edit".to_string(),
                    "bash".to_string(),
                ],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
        );

        categories.insert(
            "builder-reviewer".to_string(),
            CategoryConfig {
                description: "Deep review and fixes".to_string(),
                model: default_smart_model(),
                harness: Some("claude-code".to_string()), // smart tasks use claude-code
                tools: vec!["read".to_string(), "edit".to_string(), "bash".to_string()],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
        );

        categories.insert(
            "orchestrator".to_string(),
            CategoryConfig {
                description: "Loop and subagent orchestration".to_string(),
                model: default_smart_model(),
                harness: Some("claude-code".to_string()), // smart tasks use claude-code
                tools: vec!["read".to_string()],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
        );

        Self {
            harness: HarnessConfig::default(),
            categories,
            agents: AgentsConfig::default(),
            swarm: SwarmConfig::default(),
            scud: ScudConfig::default(),
            transcripts: TranscriptConfig::default(),
            guidance: GuidanceConfig::default(),
            prompts_dir: default_prompts_dir(),
        }
    }
}

impl Config {
    /// Load configuration from file or default locations
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = path.map(PathBuf::from).or_else(|| {
            // Try .descartes/config.toml in current directory
            let local = PathBuf::from(".descartes/config.toml");
            if local.exists() {
                return Some(local);
            }

            // Try ~/.descartes/config.toml
            dirs::home_dir().map(|h| h.join(".descartes/config.toml"))
        });

        match config_path {
            Some(p) if p.exists() => {
                let content = std::fs::read_to_string(&p)?;
                let config: Config = toml::from_str(&content)?;
                Ok(config)
            }
            _ => Ok(Config::default()),
        }
    }

    /// Get category configuration by name
    pub fn get_category(&self, name: &str) -> Option<&CategoryConfig> {
        self.categories.get(name)
    }

    /// Get transcript directory path
    pub fn transcript_dir(&self) -> PathBuf {
        self.transcripts.directory.clone()
    }

    /// Get SCUD task file path
    pub fn scud_path(&self) -> PathBuf {
        self.scud.task_file.clone()
    }
}

/// Harness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessConfig {
    /// Which harness to use
    #[serde(default = "default_harness_kind")]
    pub kind: String,

    /// Claude Code specific settings
    #[serde(default)]
    pub claude_code: ClaudeCodeConfig,

    /// OpenCode specific settings
    #[serde(default)]
    pub opencode: OpenCodeConfig,

    /// Codex specific settings
    #[serde(default)]
    pub codex: CodexConfig,
}

fn default_harness_kind() -> String {
    std::env::var("DESCARTES_HARNESS").unwrap_or_else(|_| "opencode".to_string())
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            kind: default_harness_kind(),
            claude_code: ClaudeCodeConfig::default(),
            opencode: OpenCodeConfig::default(),
            codex: CodexConfig::default(),
        }
    }
}

/// Claude Code harness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeCodeConfig {
    /// Path to claude binary (defaults to "claude" in PATH)
    #[serde(default)]
    pub binary: Option<String>,

    /// Default model to use
    #[serde(default = "default_claude_model")]
    pub model: String,

    /// Whether to use headless mode
    #[serde(default = "default_true")]
    pub headless: bool,

    /// Skip permission prompts (dangerous but needed for loops)
    #[serde(default)]
    pub dangerously_skip_permissions: bool,
}

impl Default for ClaudeCodeConfig {
    fn default() -> Self {
        Self {
            binary: None,
            model: default_claude_model(),
            headless: default_true(),
            dangerously_skip_permissions: false,
        }
    }
}

fn default_claude_model() -> String {
    std::env::var("DESCARTES_CLAUDE_MODEL").unwrap_or_else(|_| "opus".to_string())
}

fn default_true() -> bool {
    true
}

/// OpenCode harness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCodeConfig {
    /// Path to opencode binary (defaults to "opencode" in PATH)
    #[serde(default)]
    pub binary: Option<String>,

    /// Default model (e.g., "anthropic/claude-sonnet")
    #[serde(default = "default_opencode_model")]
    pub model: String,
}

fn default_opencode_model() -> String {
    std::env::var("DESCARTES_OPENCODE_MODEL").unwrap_or_else(|_| "xai/grok-code-fast-1".to_string())
}

impl Default for OpenCodeConfig {
    fn default() -> Self {
        Self {
            binary: None,
            model: default_opencode_model(),
        }
    }
}

/// Codex harness configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodexConfig {
    /// API base URL
    #[serde(default)]
    pub api_base: Option<String>,

    /// API key (can also be in environment)
    #[serde(default)]
    pub api_key: Option<String>,

    /// Default model to use
    #[serde(default = "default_codex_model")]
    pub model: String,
}

fn default_codex_model() -> String {
    std::env::var("DESCARTES_CODEX_MODEL").unwrap_or_else(|_| "gpt-4o".to_string())
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            api_base: None,
            api_key: None,
            model: default_codex_model(),
        }
    }
}

/// Agent category configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryConfig {
    /// Human-readable description
    pub description: String,

    /// Default model for this category
    pub model: String,

    /// Harness to use for this category (claude-code, opencode, codex)
    /// If not set, uses the global harness.kind setting
    #[serde(default)]
    pub harness: Option<String>,

    /// Tools available to agents in this category
    pub tools: Vec<String>,

    /// Whether multiple agents of this category can run in parallel
    #[serde(default)]
    pub parallel: bool,

    /// Whether this category acts as a backpressure gate
    #[serde(default)]
    pub backpressure: bool,

    /// Optional prompt template file
    #[serde(default)]
    pub prompt_template: Option<PathBuf>,
}

/// SCUD integration configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScudConfig {
    /// Path to SCUD task file
    #[serde(default = "default_scud_task_file")]
    pub task_file: PathBuf,

    /// Whether to use embedded SCUD or shell out to binary
    #[serde(default)]
    pub embedded: bool,

    /// Path to scud binary (if not embedded)
    #[serde(default)]
    pub binary: Option<String>,

    /// SCUD LLM provider (passed as SCUD_PROVIDER env var)
    #[serde(default)]
    pub provider: Option<String>,

    /// SCUD LLM model (passed as SCUD_MODEL env var)
    #[serde(default)]
    pub model: Option<String>,

    /// SCUD smart provider (passed as SCUD_SMART_PROVIDER env var)
    #[serde(default)]
    pub smart_provider: Option<String>,

    /// SCUD smart model (passed as SCUD_SMART_MODEL env var)
    #[serde(default)]
    pub smart_model: Option<String>,

    /// SCUD fast provider (passed as SCUD_FAST_PROVIDER env var)
    #[serde(default)]
    pub fast_provider: Option<String>,

    /// SCUD fast model (passed as SCUD_FAST_MODEL env var)
    #[serde(default)]
    pub fast_model: Option<String>,

    /// SCUD max tokens (passed as SCUD_MAX_TOKENS env var)
    #[serde(default)]
    pub max_tokens: Option<u32>,
}

impl ScudConfig {
    /// Get environment variables to pass to SCUD subprocess
    pub fn env_vars(&self) -> Vec<(String, String)> {
        let mut vars = Vec::new();
        if let Some(ref v) = self.provider {
            vars.push(("SCUD_PROVIDER".to_string(), v.clone()));
        }
        if let Some(ref v) = self.model {
            vars.push(("SCUD_MODEL".to_string(), v.clone()));
        }
        if let Some(ref v) = self.smart_provider {
            vars.push(("SCUD_SMART_PROVIDER".to_string(), v.clone()));
        }
        if let Some(ref v) = self.smart_model {
            vars.push(("SCUD_SMART_MODEL".to_string(), v.clone()));
        }
        if let Some(ref v) = self.fast_provider {
            vars.push(("SCUD_FAST_PROVIDER".to_string(), v.clone()));
        }
        if let Some(ref v) = self.fast_model {
            vars.push(("SCUD_FAST_MODEL".to_string(), v.clone()));
        }
        if let Some(v) = self.max_tokens {
            vars.push(("SCUD_MAX_TOKENS".to_string(), v.to_string()));
        }
        vars
    }
}

fn default_scud_task_file() -> PathBuf {
    PathBuf::from(".scud/scud.scg")
}

impl Default for ScudConfig {
    fn default() -> Self {
        Self {
            task_file: default_scud_task_file(),
            embedded: false,
            binary: None,
            provider: None,
            model: None,
            smart_provider: None,
            smart_model: None,
            fast_provider: None,
            fast_model: None,
            max_tokens: None,
        }
    }
}

/// Transcript configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptConfig {
    /// Directory to store transcripts
    #[serde(default = "default_transcript_dir")]
    pub directory: PathBuf,

    /// Format to use (scg or json)
    #[serde(default = "default_transcript_format")]
    pub format: String,

    /// Maximum transcripts to keep (0 = unlimited)
    #[serde(default)]
    pub max_keep: usize,
}

fn default_transcript_dir() -> PathBuf {
    PathBuf::from(".descartes/transcripts")
}

fn default_transcript_format() -> String {
    "scg".to_string()
}

impl Default for TranscriptConfig {
    fn default() -> Self {
        Self {
            directory: default_transcript_dir(),
            format: default_transcript_format(),
            max_keep: 0,
        }
    }
}

/// User guidance configuration
///
/// Allows users to inject custom context into agent prompts without modifying code.
/// Each field contains markdown text that gets prepended to relevant prompts.
///
/// # Example
///
/// ```toml
/// [guidance]
/// global = "Always follow existing code patterns. Prefer small, focused changes."
/// builder = "Run tests after making changes. Use cargo check before cargo test."
/// review = "Check for security issues and edge cases."
/// validator = "Use cargo test --all-features for full coverage."
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GuidanceConfig {
    /// Global guidance included in all prompts
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub global: Option<String>,

    /// Guidance specific to builder agents (implementation tasks)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub builder: Option<String>,

    /// Guidance specific to reviewer agents
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub review: Option<String>,

    /// Guidance specific to validator agents (test running)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validator: Option<String>,
}

impl GuidanceConfig {
    /// Get combined guidance for a specific context
    ///
    /// Returns global guidance combined with context-specific guidance.
    pub fn for_context(&self, context: &str) -> Option<String> {
        let specific = match context {
            "builder" | "fast-builder" => self.builder.as_deref(),
            "review" | "builder-reviewer" => self.review.as_deref(),
            "validator" => self.validator.as_deref(),
            _ => None,
        };

        match (self.global.as_deref(), specific) {
            (Some(g), Some(s)) => Some(format!("{}\n\n{}", g, s)),
            (Some(g), None) => Some(g.to_string()),
            (None, Some(s)) => Some(s.to_string()),
            (None, None) => None,
        }
    }

    /// Check if any guidance is configured
    pub fn has_any(&self) -> bool {
        self.global.is_some()
            || self.builder.is_some()
            || self.review.is_some()
            || self.validator.is_some()
    }
}

/// Initialize .descartes directory
pub fn init() -> Result<()> {
    let descartes_dir = PathBuf::from(".descartes");

    if !descartes_dir.exists() {
        std::fs::create_dir_all(&descartes_dir)?;
    }

    // Create transcripts directory
    let transcripts_dir = descartes_dir.join("transcripts");
    if !transcripts_dir.exists() {
        std::fs::create_dir_all(&transcripts_dir)?;
    }

    // Create default config if it doesn't exist
    let config_path = descartes_dir.join("config.toml");
    if !config_path.exists() {
        let default_config = Config::default();
        let config_str =
            toml::to_string_pretty(&default_config).map_err(|e| Error::Config(e.to_string()))?;
        std::fs::write(&config_path, config_str)?;
    }

    // Create prompts directory
    let prompts_dir = PathBuf::from("prompts");
    if !prompts_dir.exists() {
        std::fs::create_dir_all(&prompts_dir)?;

        // Write default prompts
        std::fs::write(
            prompts_dir.join("plan.md"),
            include_str!("../prompts/plan.md"),
        )?;
        std::fs::write(
            prompts_dir.join("build.md"),
            include_str!("../prompts/build.md"),
        )?;
    }

    Ok(())
}
