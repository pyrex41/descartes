//! Interactive session controller
//!
//! Manages a persistent CLI session with agent control, command processing,
//! and workflow integration.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, watch};
use tracing::{debug, info, warn};

use crate::agent::{AgentCategory, SubagentResult};
use crate::harness::Harness;
use crate::workflow::WorkflowConfig;
use crate::{Config, Error, Result};

use super::commands::{
    CommandInvocation, CommandKind, CommandRegistry, ControlAction, ResolvedCommand,
};
use super::skills::SkillRegistry;

/// Session state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionState {
    /// Waiting for user input
    Idle,
    /// Agent is running
    AgentRunning,
    /// Agent is paused
    AgentPaused,
    /// At a workflow gate
    AtGate,
    /// Session ending
    Exiting,
}

/// Messages from the session to the agent runner
#[derive(Debug, Clone)]
pub enum AgentControl {
    /// Start the agent with given prompt
    Start { category: AgentCategory, prompt: String },
    /// Pause the agent
    Pause,
    /// Resume a paused agent
    Resume,
    /// Cancel/abort the agent
    Cancel,
    /// Interrupt with new input
    Interrupt { message: String },
}

/// Messages from the agent runner to the session
#[derive(Debug, Clone)]
pub enum AgentEvent {
    /// Agent started
    Started { category: AgentCategory },
    /// Agent produced output
    Output { text: String },
    /// Agent is waiting for input
    Waiting,
    /// Agent completed
    Completed { result: SubagentResult },
    /// Agent was paused
    Paused,
    /// Agent was resumed
    Resumed,
    /// Agent was cancelled
    Cancelled,
    /// Agent encountered an error
    Error { message: String },
}

/// Interactive session controller
pub struct Session {
    /// Application config
    config: Config,
    /// Workflow config (if in workflow mode)
    workflow_config: Option<WorkflowConfig>,
    /// Command registry
    commands: CommandRegistry,
    /// Skill registry
    skills: SkillRegistry,
    /// Harness for running agents
    harness: Arc<dyn Harness>,
    /// Current session state
    state: SessionState,
    /// Current workflow stage (if in workflow mode)
    current_stage: Option<String>,
    /// Pending context to inject
    pending_context: Vec<String>,
    /// Session history (for context)
    history: Vec<HistoryEntry>,
    /// Control channel sender (to agent runner)
    control_tx: Option<mpsc::Sender<AgentControl>>,
    /// Event channel receiver (from agent runner)
    event_rx: Option<mpsc::Receiver<AgentEvent>>,
    /// Interrupt flag (set by signal handler)
    interrupt_flag: Arc<AtomicBool>,
    /// Shutdown flag
    shutdown_flag: Arc<AtomicBool>,
}

/// A history entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    /// Entry type
    pub kind: HistoryKind,
    /// Content
    pub content: String,
    /// Timestamp
    pub timestamp: Instant,
}

/// Types of history entries
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    UserInput,
    AgentOutput,
    SystemMessage,
    CommandResult,
}

impl Session {
    /// Create a new session
    pub fn new(
        config: Config,
        harness: Arc<dyn Harness>,
        workflow_config: Option<WorkflowConfig>,
    ) -> Self {
        let mut commands = CommandRegistry::new();
        let skills = SkillRegistry::new();

        // Register skill commands
        for (name, skill) in skills.list() {
            commands.register_skill(name, skill.prompt_file.clone(), &skill.description);
        }

        Self {
            config,
            workflow_config,
            commands,
            skills,
            harness,
            state: SessionState::Idle,
            current_stage: None,
            pending_context: Vec::new(),
            history: Vec::new(),
            control_tx: None,
            event_rx: None,
            interrupt_flag: Arc::new(AtomicBool::new(false)),
            shutdown_flag: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get the interrupt flag (for signal handler)
    pub fn interrupt_flag(&self) -> Arc<AtomicBool> {
        self.interrupt_flag.clone()
    }

    /// Get the shutdown flag (for signal handler)
    pub fn shutdown_flag(&self) -> Arc<AtomicBool> {
        self.shutdown_flag.clone()
    }

    /// Run the interactive session
    pub async fn run(&mut self) -> Result<()> {
        info!("Starting interactive session");
        self.print_welcome();

        // Set up input handling
        let stdin = tokio::io::stdin();
        let reader = BufReader::new(stdin);
        let mut lines = reader.lines();

        loop {
            // Check for shutdown
            if self.shutdown_flag.load(Ordering::SeqCst) {
                info!("Shutdown requested");
                break;
            }

            // Check for interrupt (Ctrl+C)
            if self.interrupt_flag.swap(false, Ordering::SeqCst) {
                self.handle_interrupt().await?;
            }

            // Handle any pending agent events
            let events: Vec<AgentEvent> = if let Some(ref mut rx) = self.event_rx {
                let mut events = Vec::new();
                while let Ok(event) = rx.try_recv() {
                    events.push(event);
                }
                events
            } else {
                Vec::new()
            };
            for event in events {
                self.handle_agent_event(event).await?;
            }

            // Display prompt based on state
            self.print_prompt();

            // Read input
            match lines.next_line().await {
                Ok(Some(line)) => {
                    let line = line.trim();
                    if !line.is_empty() {
                        self.handle_input(line).await?;
                    }
                }
                Ok(None) => {
                    // EOF
                    info!("EOF received, exiting");
                    break;
                }
                Err(e) => {
                    warn!("Input error: {}", e);
                }
            }

            if self.state == SessionState::Exiting {
                break;
            }
        }

        self.cleanup().await?;
        Ok(())
    }

    /// Handle user input
    async fn handle_input(&mut self, input: &str) -> Result<()> {
        self.add_history(HistoryKind::UserInput, input);

        // Check if it's a command
        if let Some(invocation) = CommandInvocation::parse(input) {
            return self.handle_command(invocation).await;
        }

        // Regular input - behavior depends on state
        match self.state {
            SessionState::Idle => {
                // Start an agent with this as the prompt
                self.start_agent(AgentCategory::Builder, input.to_string())
                    .await?;
            }
            SessionState::AgentRunning => {
                // Queue as interrupt/follow-up
                if let Some(ref tx) = self.control_tx {
                    let _ = tx
                        .send(AgentControl::Interrupt {
                            message: input.to_string(),
                        })
                        .await;
                }
            }
            SessionState::AgentPaused => {
                // Resume with this input
                self.pending_context.push(input.to_string());
                self.resume_agent().await?;
            }
            SessionState::AtGate => {
                // Process as gate response
                self.handle_gate_response(input).await?;
            }
            SessionState::Exiting => {}
        }

        Ok(())
    }

    /// Handle a parsed command
    async fn handle_command(&mut self, invocation: CommandInvocation) -> Result<()> {
        let resolved = match self.commands.resolve(&invocation) {
            Some(cmd) => cmd,
            None => {
                // Check if it might be a skill
                if let Some(skill) = self.skills.get(&invocation.name) {
                    self.execute_skill(&invocation.name, &invocation.raw_args)
                        .await?;
                    return Ok(());
                }

                self.print_error(&format!("Unknown command: /{}", invocation.name));
                self.print_system("Type /help for available commands");
                return Ok(());
            }
        };

        match &resolved.command.kind {
            CommandKind::Control(action) => {
                self.handle_control_action(*action).await?;
            }
            CommandKind::Skill {
                prompt_file,
                category,
                auto_start,
            } => {
                self.execute_skill(&resolved.command.name, &resolved.raw_args)
                    .await?;
            }
            CommandKind::Transition {
                to_stage,
                generate_handoff,
            } => {
                self.handle_transition(&resolved).await?;
            }
            CommandKind::Context { context_type } => {
                self.inject_context(&resolved).await?;
            }
            CommandKind::Builtin(name) => {
                self.handle_builtin(name, &resolved).await?;
            }
        }

        Ok(())
    }

    /// Handle a control action
    async fn handle_control_action(&mut self, action: ControlAction) -> Result<()> {
        match action {
            ControlAction::Pause => {
                if self.state == SessionState::AgentRunning {
                    if let Some(ref tx) = self.control_tx {
                        let _ = tx.send(AgentControl::Pause).await;
                    }
                    self.print_system("Pausing agent...");
                } else {
                    self.print_error("No agent running to pause");
                }
            }
            ControlAction::Resume => {
                if self.state == SessionState::AgentPaused {
                    self.resume_agent().await?;
                } else {
                    self.print_error("No paused agent to resume");
                }
            }
            ControlAction::Cancel => {
                if self.state == SessionState::AgentRunning
                    || self.state == SessionState::AgentPaused
                {
                    if let Some(ref tx) = self.control_tx {
                        let _ = tx.send(AgentControl::Cancel).await;
                    }
                    self.print_system("Cancelling agent...");
                } else {
                    self.print_error("No agent to cancel");
                }
            }
            ControlAction::Status => {
                self.print_status();
            }
            ControlAction::Clear => {
                self.pending_context.clear();
                self.print_system("Context cleared");
            }
            ControlAction::Exit => {
                self.state = SessionState::Exiting;
                self.print_system("Goodbye!");
            }
        }
        Ok(())
    }

    /// Handle builtin commands
    async fn handle_builtin(&mut self, name: &str, resolved: &ResolvedCommand) -> Result<()> {
        match name {
            "help" => {
                println!("{}", self.commands.help());
                println!("\nSkills:");
                for (name, skill) in self.skills.list() {
                    println!("  /{} - {}", name, skill.description);
                }
            }
            "skill" => {
                if let Some(skill_name) = resolved.args.first() {
                    let rest = resolved.args[1..].join(" ");
                    self.execute_skill(skill_name, &rest).await?;
                } else {
                    self.print_error("Usage: /skill <name> [args]");
                    println!("\nAvailable skills:");
                    for (name, skill) in self.skills.list() {
                        println!("  {} - {}", name, skill.description);
                    }
                }
            }
            _ => {
                self.print_error(&format!("Unknown builtin: {}", name));
            }
        }
        Ok(())
    }

    /// Execute a skill
    async fn execute_skill(&mut self, name: &str, args: &str) -> Result<()> {
        let skill = match self.skills.get(name) {
            Some(s) => s.clone(),
            None => {
                self.print_error(&format!("Unknown skill: {}", name));
                return Ok(());
            }
        };

        // Load the prompt
        let prompt = match skill.load_prompt(args) {
            Ok(p) => p,
            Err(e) => {
                self.print_error(&format!("Failed to load skill prompt: {}", e));
                return Ok(());
            }
        };

        // Add any pending context
        let full_prompt = if self.pending_context.is_empty() {
            prompt
        } else {
            let context = self.pending_context.join("\n\n");
            self.pending_context.clear();
            format!("{}\n\n---\n\n{}", context, prompt)
        };

        // Start the agent
        let category = skill
            .category
            .as_ref()
            .and_then(|c| c.parse().ok())
            .unwrap_or(AgentCategory::Builder);

        self.start_agent(category, full_prompt).await
    }

    /// Inject context
    async fn inject_context(&mut self, resolved: &ResolvedCommand) -> Result<()> {
        use super::commands::ContextType;

        let context = match &resolved.command.kind {
            CommandKind::Context { context_type } => match context_type {
                ContextType::File(path) => {
                    let path = if resolved.args.is_empty() {
                        path.clone()
                    } else {
                        std::path::PathBuf::from(&resolved.args[0])
                    };
                    std::fs::read_to_string(&path).map_err(|e| {
                        Error::Config(format!("Failed to read {}: {}", path.display(), e))
                    })?
                }
                ContextType::ScudTasks => {
                    self.run_command("scud", &["list", "--format", "markdown"])
                        .await?
                }
                ContextType::ScudWaves => {
                    self.run_command("scud", &["waves", "--format", "markdown"])
                        .await?
                }
                ContextType::GitDiff => self.run_command("git", &["diff", "--stat"]).await?,
                ContextType::GitStatus => self.run_command("git", &["status", "--short"]).await?,
                ContextType::Command(cmd) => {
                    let cmd = if resolved.args.is_empty() {
                        cmd.clone()
                    } else {
                        resolved.args.join(" ")
                    };
                    self.run_shell_command(&cmd).await?
                }
                ContextType::Handoff => {
                    // Load most recent handoff
                    "<!-- Previous handoff would be loaded here -->".to_string()
                }
            },
            _ => return Ok(()),
        };

        self.pending_context.push(context.clone());
        self.print_system(&format!("Added {} bytes of context", context.len()));
        Ok(())
    }

    /// Handle workflow transition
    async fn handle_transition(&mut self, resolved: &ResolvedCommand) -> Result<()> {
        let to_stage = if !resolved.args.is_empty() {
            resolved.args[0].clone()
        } else {
            // Determine next stage from workflow config
            if let Some(ref wf) = self.workflow_config {
                if let Some(current) = &self.current_stage {
                    wf.next_stage(current)
                        .map(|s| s.to_string())
                        .unwrap_or_default()
                } else {
                    wf.stages().first().cloned().unwrap_or_default()
                }
            } else {
                String::new()
            }
        };

        if to_stage.is_empty() {
            self.print_error("No target stage specified and couldn't determine next stage");
            return Ok(());
        }

        self.print_system(&format!("Transitioning to stage: {}", to_stage));
        self.current_stage = Some(to_stage);
        // Actual transition logic would go here
        Ok(())
    }

    /// Start an agent
    async fn start_agent(&mut self, category: AgentCategory, prompt: String) -> Result<()> {
        let (control_tx, mut control_rx) = mpsc::channel::<AgentControl>(10);
        let (event_tx, event_rx) = mpsc::channel::<AgentEvent>(100);

        self.control_tx = Some(control_tx);
        self.event_rx = Some(event_rx);
        self.state = SessionState::AgentRunning;

        let harness = self.harness.clone();
        let config = self.config.clone();

        // Spawn agent runner
        tokio::spawn(async move {
            let _ = event_tx
                .send(AgentEvent::Started {
                    category: category.clone(),
                })
                .await;

            // Run the agent
            match crate::agent::spawn_subagent(&*harness, category, prompt, None).await {
                Ok(result) => {
                    let _ = event_tx.send(AgentEvent::Completed { result }).await;
                }
                Err(e) => {
                    let _ = event_tx
                        .send(AgentEvent::Error {
                            message: e.to_string(),
                        })
                        .await;
                }
            }
        });

        Ok(())
    }

    /// Resume a paused agent
    async fn resume_agent(&mut self) -> Result<()> {
        if let Some(ref tx) = self.control_tx {
            let _ = tx.send(AgentControl::Resume).await;
        }
        self.state = SessionState::AgentRunning;
        self.print_system("Resuming agent...");
        Ok(())
    }

    /// Handle agent events
    async fn handle_agent_event(&mut self, event: AgentEvent) -> Result<()> {
        match event {
            AgentEvent::Started { category } => {
                self.print_system(&format!("Agent started: {:?}", category));
            }
            AgentEvent::Output { text } => {
                self.add_history(HistoryKind::AgentOutput, &text);
                print!("{}", text);
            }
            AgentEvent::Waiting => {
                self.print_system("Agent waiting for input...");
            }
            AgentEvent::Completed { result } => {
                self.state = SessionState::Idle;
                self.print_system(&format!("Agent completed: {}", result.summary()));
                self.add_history(HistoryKind::AgentOutput, &result.summary());
            }
            AgentEvent::Paused => {
                self.state = SessionState::AgentPaused;
                self.print_system("Agent paused. Use /resume to continue or /cancel to abort.");
            }
            AgentEvent::Resumed => {
                self.state = SessionState::AgentRunning;
                self.print_system("Agent resumed.");
            }
            AgentEvent::Cancelled => {
                self.state = SessionState::Idle;
                self.print_system("Agent cancelled.");
            }
            AgentEvent::Error { message } => {
                self.state = SessionState::Idle;
                self.print_error(&format!("Agent error: {}", message));
            }
        }
        Ok(())
    }

    /// Handle interrupt (Ctrl+C)
    async fn handle_interrupt(&mut self) -> Result<()> {
        match self.state {
            SessionState::AgentRunning => {
                self.print_system("\nInterrupt received. Pausing agent...");
                if let Some(ref tx) = self.control_tx {
                    let _ = tx.send(AgentControl::Pause).await;
                }
            }
            SessionState::AgentPaused => {
                self.print_system("\nAgent already paused. Use /cancel to abort or /resume to continue.");
            }
            _ => {
                self.print_system("\nUse /exit to quit");
            }
        }
        Ok(())
    }

    /// Handle gate response
    async fn handle_gate_response(&mut self, input: &str) -> Result<()> {
        let response = input.trim().to_lowercase();
        match response.as_str() {
            "y" | "yes" | "a" | "approve" | "" => {
                self.print_system("Approved. Continuing...");
                self.state = SessionState::Idle;
            }
            "n" | "no" | "r" | "reject" => {
                self.print_system("Rejected. Workflow paused.");
                self.state = SessionState::Idle;
            }
            "e" | "edit" => {
                self.print_system("Edit mode. Enter your changes:");
            }
            "s" | "skip" => {
                self.print_system("Skipping stage.");
                self.state = SessionState::Idle;
            }
            _ => {
                self.print_system("Options: [y]es, [n]o, [e]dit, [s]kip");
            }
        }
        Ok(())
    }

    /// Run a command and return output
    async fn run_command(&self, cmd: &str, args: &[&str]) -> Result<String> {
        let output = tokio::process::Command::new(cmd)
            .args(args)
            .output()
            .await
            .map_err(|e| Error::Command(format!("{} failed: {}", cmd, e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Run a shell command and return output
    async fn run_shell_command(&self, cmd: &str) -> Result<String> {
        let output = tokio::process::Command::new("sh")
            .args(["-c", cmd])
            .output()
            .await
            .map_err(|e| Error::Command(format!("shell command failed: {}", e)))?;

        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    /// Add entry to history
    fn add_history(&mut self, kind: HistoryKind, content: &str) {
        self.history.push(HistoryEntry {
            kind,
            content: content.to_string(),
            timestamp: Instant::now(),
        });

        // Keep history bounded
        if self.history.len() > 1000 {
            self.history.drain(0..100);
        }
    }

    /// Cleanup on exit
    async fn cleanup(&mut self) -> Result<()> {
        // Cancel any running agent
        if let Some(ref tx) = self.control_tx {
            let _ = tx.send(AgentControl::Cancel).await;
        }
        Ok(())
    }

    // UI helpers

    fn print_welcome(&self) {
        println!("╭─────────────────────────────────────────────╮");
        println!("│  Descartes Interactive Session              │");
        println!("│  Type /help for commands, /exit to quit     │");
        println!("╰─────────────────────────────────────────────╯");
        println!();
    }

    fn print_prompt(&self) {
        use std::io::Write;

        let prompt = match self.state {
            SessionState::Idle => {
                if let Some(ref stage) = self.current_stage {
                    format!("[{}] > ", stage)
                } else {
                    "> ".to_string()
                }
            }
            SessionState::AgentRunning => "● ".to_string(),
            SessionState::AgentPaused => "⏸ ".to_string(),
            SessionState::AtGate => "🚧 ".to_string(),
            SessionState::Exiting => "".to_string(),
        };

        print!("{}", prompt);
        let _ = std::io::stdout().flush();
    }

    fn print_status(&self) {
        println!("\n─── Status ───");
        println!("State: {:?}", self.state);
        if let Some(ref stage) = self.current_stage {
            println!("Stage: {}", stage);
        }
        println!("Pending context: {} items", self.pending_context.len());
        println!("History entries: {}", self.history.len());
        if let Some(ref wf) = self.workflow_config {
            println!("Workflow: {}", wf.workflow.name);
        }
        println!("──────────────\n");
    }

    fn print_system(&self, msg: &str) {
        println!("\x1b[90m{}\x1b[0m", msg);
    }

    fn print_error(&self, msg: &str) {
        println!("\x1b[31m{}\x1b[0m", msg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_state() {
        assert_eq!(SessionState::Idle, SessionState::Idle);
        assert_ne!(SessionState::Idle, SessionState::AgentRunning);
    }

    #[test]
    fn test_history_entry() {
        let entry = HistoryEntry {
            kind: HistoryKind::UserInput,
            content: "test".to_string(),
            timestamp: Instant::now(),
        };
        assert_eq!(entry.kind, HistoryKind::UserInput);
    }
}
