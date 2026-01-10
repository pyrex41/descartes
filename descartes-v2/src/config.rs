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

    /// SCUD integration settings
    #[serde(default)]
    pub scud: ScudConfig,

    /// Transcript settings
    #[serde(default)]
    pub transcripts: TranscriptConfig,

    /// Path to prompts directory
    #[serde(default = "default_prompts_dir")]
    pub prompts_dir: PathBuf,
}

fn default_prompts_dir() -> PathBuf {
    PathBuf::from("prompts")
}

impl Default for Config {
    fn default() -> Self {
        let mut categories = HashMap::new();

        categories.insert(
            "searcher".to_string(),
            CategoryConfig {
                description: "Fast parallel code search".to_string(),
                model: "sonnet".to_string(),
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
                model: "sonnet".to_string(),
                tools: vec!["read".to_string()],
                parallel: true,
                backpressure: false,
                prompt_template: None,
            },
        );

        categories.insert(
            "builder".to_string(),
            CategoryConfig {
                description: "Code implementation".to_string(),
                model: "opus".to_string(),
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
                model: "sonnet".to_string(),
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
                model: "opus".to_string(),
                tools: vec!["read".to_string(), "bash".to_string()],
                parallel: false,
                backpressure: false,
                prompt_template: None,
            },
        );

        Self {
            harness: HarnessConfig::default(),
            categories,
            scud: ScudConfig::default(),
            transcripts: TranscriptConfig::default(),
            prompts_dir: default_prompts_dir(),
        }
    }
}

impl Config {
    /// Load configuration from file or default locations
    pub fn load(path: Option<&Path>) -> Result<Self> {
        let config_path = path
            .map(PathBuf::from)
            .or_else(|| {
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
    "claude-code".to_string()
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
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
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

fn default_claude_model() -> String {
    "opus".to_string()
}

fn default_true() -> bool {
    true
}

/// OpenCode harness configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OpenCodeConfig {
    /// Socket path for IPC
    #[serde(default)]
    pub socket_path: Option<PathBuf>,

    /// Default model
    #[serde(default)]
    pub model: Option<String>,
}

/// Codex harness configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CodexConfig {
    /// API base URL
    #[serde(default)]
    pub api_base: Option<String>,

    /// API key (can also be in environment)
    #[serde(default)]
    pub api_key: Option<String>,
}

/// Agent category configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryConfig {
    /// Human-readable description
    pub description: String,

    /// Default model for this category
    pub model: String,

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
        let config_str = toml::to_string_pretty(&default_config)
            .map_err(|e| Error::Config(e.to_string()))?;
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
