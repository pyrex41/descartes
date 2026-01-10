//! BAML Runtime wrapper for Descartes
//!
//! Provides initialization and configuration of the BAML runtime,
//! including client setup and environment configuration.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::{Error, Result};

/// Configuration for the BAML runtime
#[derive(Debug, Clone)]
pub struct BamlConfig {
    /// Path to BAML source directory
    pub baml_src_dir: PathBuf,
    /// Environment variables to pass to BAML
    pub env_vars: HashMap<String, String>,
    /// Default client to use
    pub default_client: String,
    /// Enable verbose logging
    pub verbose: bool,
}

impl Default for BamlConfig {
    fn default() -> Self {
        Self {
            baml_src_dir: PathBuf::from("baml_src"),
            env_vars: HashMap::new(),
            default_client: "DecisionModel".to_string(),
            verbose: false,
        }
    }
}

impl BamlConfig {
    /// Create config from environment
    pub fn from_env() -> Self {
        let mut config = Self::default();

        // Load API keys from environment
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            config.env_vars.insert("OPENAI_API_KEY".to_string(), key);
        }
        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            config.env_vars.insert("ANTHROPIC_API_KEY".to_string(), key);
        }

        // Check for BAML_SRC_DIR override
        if let Ok(dir) = std::env::var("BAML_SRC_DIR") {
            config.baml_src_dir = PathBuf::from(dir);
        }

        config
    }

    /// Set the BAML source directory
    pub fn with_baml_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.baml_src_dir = dir.into();
        self
    }

    /// Set the default client
    pub fn with_default_client(mut self, client: impl Into<String>) -> Self {
        self.default_client = client.into();
        self
    }

    /// Add an environment variable
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env_vars.insert(key.into(), value.into());
        self
    }
}

/// BAML Runtime wrapper
///
/// This provides a simplified interface to the BAML runtime for Descartes.
/// Since native Rust code generation is still in development, this uses
/// the runtime API directly.
pub struct BamlRuntime {
    config: BamlConfig,
    initialized: bool,
}

impl BamlRuntime {
    /// Create a new BAML runtime with the given configuration
    pub fn new(config: BamlConfig) -> Self {
        Self {
            config,
            initialized: false,
        }
    }

    /// Create with default configuration from environment
    pub fn from_env() -> Self {
        Self::new(BamlConfig::from_env())
    }

    /// Initialize the runtime
    pub fn init(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Verify BAML source directory exists
        if !self.config.baml_src_dir.exists() {
            return Err(Error::Config(format!(
                "BAML source directory not found: {}",
                self.config.baml_src_dir.display()
            )));
        }

        // Verify required API keys
        let has_openai = self.config.env_vars.contains_key("OPENAI_API_KEY")
            || std::env::var("OPENAI_API_KEY").is_ok();
        let has_anthropic = self.config.env_vars.contains_key("ANTHROPIC_API_KEY")
            || std::env::var("ANTHROPIC_API_KEY").is_ok();

        if !has_openai && !has_anthropic {
            warn!("No API keys found. Set OPENAI_API_KEY or ANTHROPIC_API_KEY");
        }

        info!(
            "BAML runtime initialized with source dir: {}",
            self.config.baml_src_dir.display()
        );

        self.initialized = true;
        Ok(())
    }

    /// Check if the runtime is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get the configuration
    pub fn config(&self) -> &BamlConfig {
        &self.config
    }

    /// Get the BAML source directory
    pub fn baml_src_dir(&self) -> &PathBuf {
        &self.config.baml_src_dir
    }
}

/// Builder for creating prompts compatible with BAML's output format
pub struct PromptBuilder {
    sections: Vec<String>,
    output_format: Option<String>,
}

impl PromptBuilder {
    pub fn new() -> Self {
        Self {
            sections: Vec::new(),
            output_format: None,
        }
    }

    /// Add a section to the prompt
    pub fn section(mut self, title: &str, content: &str) -> Self {
        self.sections.push(format!("## {}\n{}", title, content));
        self
    }

    /// Add the output format instruction
    pub fn output_format(mut self, format: &str) -> Self {
        self.output_format = Some(format.to_string());
        self
    }

    /// Build the final prompt
    pub fn build(self) -> String {
        let mut prompt = self.sections.join("\n\n");
        if let Some(format) = self.output_format {
            prompt.push_str("\n\n");
            prompt.push_str(&format);
        }
        prompt
    }
}

impl Default for PromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baml_config_default() {
        let config = BamlConfig::default();
        assert_eq!(config.baml_src_dir, PathBuf::from("baml_src"));
        assert_eq!(config.default_client, "DecisionModel");
    }

    #[test]
    fn test_baml_config_builder() {
        let config = BamlConfig::default()
            .with_baml_dir("/custom/path")
            .with_default_client("ClaudeModel")
            .with_env("TEST_KEY", "test_value");

        assert_eq!(config.baml_src_dir, PathBuf::from("/custom/path"));
        assert_eq!(config.default_client, "ClaudeModel");
        assert_eq!(config.env_vars.get("TEST_KEY"), Some(&"test_value".to_string()));
    }

    #[test]
    fn test_prompt_builder() {
        let prompt = PromptBuilder::new()
            .section("Context", "Some context here")
            .section("Task", "Do something")
            .output_format("Respond in JSON")
            .build();

        assert!(prompt.contains("## Context"));
        assert!(prompt.contains("## Task"));
        assert!(prompt.contains("Respond in JSON"));
    }
}
