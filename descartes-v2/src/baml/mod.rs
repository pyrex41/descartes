//! BAML integration for structured LLM outputs
//!
//! This module provides the bridge between BAML's type-safe function calling
//! and the descartes agent orchestration system.

pub mod decision;
pub mod runtime;
pub mod types;

pub use decision::{Decision, DecisionContext, DecisionEngine};
pub use runtime::BamlRuntime;
pub use types::*;
