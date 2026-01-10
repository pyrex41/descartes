//! Interactive session system
//!
//! Provides a persistent CLI session with:
//! - Agent lifecycle control (pause, resume, cancel)
//! - Slash commands for context injection and control
//! - Skills/prompts loading system
//! - Signal handling (Ctrl+C for interrupt)
//! - Workflow stage integration

pub mod commands;
pub mod session;
pub mod signals;
pub mod skills;

pub use commands::{
    Command, CommandInvocation, CommandKind, CommandRegistry, ContextType, ControlAction,
    ResolvedCommand,
};
pub use session::{AgentControl, AgentEvent, HistoryEntry, HistoryKind, Session, SessionState};
pub use signals::SignalHandler;
pub use skills::{Skill, SkillRegistry, SkillVariable};
