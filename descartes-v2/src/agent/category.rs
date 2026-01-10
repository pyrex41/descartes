//! Agent categories define the role and capabilities of agents

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::{Error, Result};

/// Built-in agent categories plus custom user-defined categories
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCategory {
    /// Fast parallel code search (Sonnet, read-only)
    Searcher,
    /// Deep code analysis (Sonnet, read-only)
    Analyzer,
    /// Code implementation (Opus, full tools)
    Builder,
    /// Test runner with backpressure (Sonnet, bash only)
    Validator,
    /// Task planning and breakdown (Opus, read + bash)
    Planner,
    /// User-defined custom category
    Custom(String),
}

impl AgentCategory {
    /// Check if this category can run in parallel
    pub fn is_parallel(&self) -> bool {
        matches!(self, AgentCategory::Searcher | AgentCategory::Analyzer)
    }

    /// Check if this category acts as a backpressure gate
    pub fn is_backpressure(&self) -> bool {
        matches!(self, AgentCategory::Validator)
    }

    /// Get the recommended model tier for this category
    pub fn model_tier(&self) -> ModelTier {
        match self {
            AgentCategory::Searcher | AgentCategory::Analyzer | AgentCategory::Validator => {
                ModelTier::Fast // Sonnet-tier
            }
            AgentCategory::Builder | AgentCategory::Planner => {
                ModelTier::Strong // Opus-tier
            }
            AgentCategory::Custom(_) => ModelTier::Fast, // Default to cheaper
        }
    }

    /// Get the name of this category
    pub fn name(&self) -> &str {
        match self {
            AgentCategory::Searcher => "searcher",
            AgentCategory::Analyzer => "analyzer",
            AgentCategory::Builder => "builder",
            AgentCategory::Validator => "validator",
            AgentCategory::Planner => "planner",
            AgentCategory::Custom(name) => name,
        }
    }
}

impl fmt::Display for AgentCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl FromStr for AgentCategory {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "searcher" | "search" => Ok(AgentCategory::Searcher),
            "analyzer" | "analyse" | "analyze" => Ok(AgentCategory::Analyzer),
            "builder" | "build" | "implement" | "implementer" => Ok(AgentCategory::Builder),
            "validator" | "validate" | "test" | "tester" => Ok(AgentCategory::Validator),
            "planner" | "plan" | "planning" => Ok(AgentCategory::Planner),
            other => Ok(AgentCategory::Custom(other.to_string())),
        }
    }
}

/// Model tier for cost/capability tradeoff
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelTier {
    /// Fast, cheaper models (Sonnet, GPT-4o-mini)
    Fast,
    /// Strong reasoning models (Opus, GPT-4)
    Strong,
}

impl ModelTier {
    /// Get the default model name for this tier
    pub fn default_model(&self) -> &str {
        match self {
            ModelTier::Fast => "sonnet",
            ModelTier::Strong => "opus",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_category_parsing() {
        assert_eq!(
            "searcher".parse::<AgentCategory>().unwrap(),
            AgentCategory::Searcher
        );
        assert_eq!(
            "search".parse::<AgentCategory>().unwrap(),
            AgentCategory::Searcher
        );
        assert_eq!(
            "builder".parse::<AgentCategory>().unwrap(),
            AgentCategory::Builder
        );
        assert_eq!(
            "implement".parse::<AgentCategory>().unwrap(),
            AgentCategory::Builder
        );

        // Custom categories
        assert_eq!(
            "security_reviewer".parse::<AgentCategory>().unwrap(),
            AgentCategory::Custom("security_reviewer".to_string())
        );
    }

    #[test]
    fn test_parallel_categories() {
        assert!(AgentCategory::Searcher.is_parallel());
        assert!(AgentCategory::Analyzer.is_parallel());
        assert!(!AgentCategory::Builder.is_parallel());
        assert!(!AgentCategory::Validator.is_parallel());
    }

    #[test]
    fn test_backpressure() {
        assert!(!AgentCategory::Searcher.is_backpressure());
        assert!(AgentCategory::Validator.is_backpressure());
    }
}
