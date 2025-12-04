//! Minimal tool definitions for Descartes agents.
//!
//! Following Pi's philosophy: if you don't need it, don't build it.
//! These 4 tools are sufficient for effective coding agents.
//!
//! Tool levels:
//! - `Minimal`: read, write, edit, bash (for sub-sessions)
//! - `Orchestrator`: minimal + spawn_session (for top-level agents)
//! - `ReadOnly`: read, bash (for exploration/planning)

mod definitions;
mod executors;
mod registry;

pub use definitions::*;
pub use executors::*;
pub use registry::*;
