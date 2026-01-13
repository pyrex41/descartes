//! Workflow orchestration system
//!
//! Provides workflow management for multi-stage agent pipelines:
//!
//! - **Stages**: Distinct phases of work (research, plan, implement, validate)
//! - **Transitions**: Movement between stages with handoffs
//! - **Gates**: Control points for approval/review
//! - **Notifications**: Async alerts via Telegram, Slack, etc.
//! - **State**: Persistent tracking of workflow progress
//!
//! ## Example Workflow
//!
//! ```toml
//! [workflow]
//! name = "feature-development"
//! stages = ["research", "plan", "implement", "validate"]
//!
//! [gates.research_to_plan]
//! type = "notify"
//! timeout = "5m"
//! notify = ["telegram"]
//!
//! [gates.plan_to_implement]
//! type = "manual"
//!
//! [transitions.plan_to_implement]
//! command = "/implement_plan"
//! pre_hooks = ["scud parse"]
//! auto_context = ["scud_tasks", "scud_waves"]
//! ```
//!
//! ## Running Workflows
//!
//! ```bash
//! # Step-by-step mode
//! descartes workflow run --step-by-step
//!
//! # One-shot mode
//! descartes workflow run --one-shot
//!
//! # With configured gates (default)
//! descartes workflow run
//! ```

pub mod config;
pub mod gate;
pub mod notify;
pub mod runner;
pub mod state;

pub use config::{
    default_workflow, AutoContext, GateConfig, GateType, NotificationConfig, TimeoutAction,
    TransitionConfig, WorkflowConfig, WorkflowMeta,
};
pub use gate::{ApprovalMethod, CliGate, GateController, GateResult};
pub use notify::{
    create_channels, DesktopChannel, LogChannel, Notification, NotificationChannel,
    NotificationResponse, SlackChannel, TelegramChannel,
};
pub use runner::{quick_handoff, RunOptions, WorkflowRunner};
pub use state::{
    GateState, GateStatus, StageState, StageStatus, StateManager, WorkflowState, WorkflowStatus,
};
