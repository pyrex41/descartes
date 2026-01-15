//! Swarm orchestrator TUI
//!
//! Terminal UI for monitoring wave progress and agent status using crossterm.
//! Provides real-time visualization of the Swarm loop execution.

use std::io::{self, Write};
use std::time::Duration;

use crossterm::{
    cursor,
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{self, ClearType},
};

use crate::agent::{AgentRegistry, RegistryStatus};
use crate::{Config, Result};

/// TUI action returned from keyboard input
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TuiAction {
    /// Attach to agent by index (1-9)
    Attach(usize),
    /// Trigger validation
    Validate,
    /// Quit the TUI
    Quit,
    /// No action (continue running)
    None,
}

/// Wave progress state for visualization
#[derive(Debug, Clone)]
pub struct WaveProgress {
    /// All waves (groups of task IDs)
    pub waves: Vec<Vec<String>>,
    /// Current wave index (0-based)
    pub current_wave: usize,
    /// Tasks completed in current wave
    pub completed_in_wave: usize,
}

impl WaveProgress {
    /// Create new wave progress
    pub fn new(waves: Vec<Vec<String>>) -> Self {
        Self {
            waves,
            current_wave: 0,
            completed_in_wave: 0,
        }
    }

    /// Get total number of waves
    pub fn total_waves(&self) -> usize {
        self.waves.len()
    }

    /// Get tasks in current wave
    pub fn current_wave_tasks(&self) -> &[String] {
        self.waves
            .get(self.current_wave)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Get total tasks across all waves
    pub fn total_tasks(&self) -> usize {
        self.waves.iter().map(|w| w.len()).sum()
    }

    /// Advance to next wave
    pub fn advance_wave(&mut self) {
        if self.current_wave < self.waves.len().saturating_sub(1) {
            self.current_wave += 1;
            self.completed_in_wave = 0;
        }
    }

    /// Mark a task as completed in current wave
    pub fn complete_task(&mut self) {
        self.completed_in_wave += 1;
    }
}

/// Swarm orchestrator TUI
pub struct SwarmTui {
    /// Wave progress state
    wave_progress: Option<WaveProgress>,
    /// Agent registry for status tracking
    registry: AgentRegistry,
    /// Whether TUI is running
    running: bool,
    /// Last render height (for clearing)
    last_render_height: u16,
}

impl SwarmTui {
    /// Create a new TUI instance
    pub fn new() -> Self {
        Self {
            wave_progress: None,
            registry: AgentRegistry::new(),
            running: false,
            last_render_height: 0,
        }
    }

    /// Create TUI with existing registry
    pub fn with_registry(registry: AgentRegistry) -> Self {
        Self {
            wave_progress: None,
            registry,
            running: false,
            last_render_height: 0,
        }
    }

    /// Set wave progress
    pub fn set_waves(&mut self, waves: Vec<Vec<String>>) {
        self.wave_progress = Some(WaveProgress::new(waves));
    }

    /// Get mutable reference to wave progress
    pub fn wave_progress_mut(&mut self) -> Option<&mut WaveProgress> {
        self.wave_progress.as_mut()
    }

    /// Get reference to wave progress
    pub fn wave_progress(&self) -> Option<&WaveProgress> {
        self.wave_progress.as_ref()
    }

    /// Get mutable reference to agent registry
    pub fn registry_mut(&mut self) -> &mut AgentRegistry {
        &mut self.registry
    }

    /// Get reference to agent registry
    pub fn registry(&self) -> &AgentRegistry {
        &self.registry
    }

    /// Run the TUI main loop
    ///
    /// Polls for input every 100ms and renders the display.
    /// Returns when quit is requested or an error occurs.
    pub fn run(&mut self) -> Result<()> {
        self.running = true;

        // Enable raw mode for keyboard input
        terminal::enable_raw_mode()?;

        // Hide cursor during TUI
        let mut stdout = io::stdout();
        execute!(stdout, cursor::Hide)?;

        let result = self.run_loop();

        // Cleanup: restore terminal state
        execute!(stdout, cursor::Show)?;
        terminal::disable_raw_mode()?;

        result
    }

    /// Internal run loop
    fn run_loop(&mut self) -> Result<()> {
        while self.running {
            // Render current state
            self.render()?;

            // Poll for input with 100ms timeout
            match self.poll_input(Duration::from_millis(100))? {
                TuiAction::Quit => {
                    self.running = false;
                }
                TuiAction::Attach(idx) => {
                    // Get agent at index and focus
                    let agents: Vec<_> = self.registry.list_running();
                    if idx > 0 && idx <= agents.len() {
                        let agent_id = agents[idx - 1].id.clone();
                        if let Ok(focus_cmd) = self.registry.focus(&agent_id) {
                            if focus_cmd.is_supported() {
                                // Execute focus command
                                let _ = std::process::Command::new("sh")
                                    .arg("-c")
                                    .arg(focus_cmd.to_shell_command())
                                    .spawn();
                            }
                        }
                    }
                }
                TuiAction::Validate => {
                    // Validation requested - caller should handle this
                    // For now, just indicate it was requested
                }
                TuiAction::None => {
                    // Continue polling
                }
            }
        }

        Ok(())
    }

    /// Run a single iteration (for external control)
    ///
    /// Returns the action to take based on keyboard input.
    /// Call this in your own loop if you need more control.
    pub fn tick(&mut self) -> Result<TuiAction> {
        self.render()?;
        self.poll_input(Duration::from_millis(100))
    }

    /// Poll for keyboard input with timeout
    fn poll_input(&self, timeout: Duration) -> Result<TuiAction> {
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                return Ok(self.handle_key(key));
            }
        }
        Ok(TuiAction::None)
    }

    /// Handle keyboard input
    fn handle_key(&self, key: KeyEvent) -> TuiAction {
        match key.code {
            // Quit on 'q' or Ctrl+C
            KeyCode::Char('q') => TuiAction::Quit,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => TuiAction::Quit,

            // Validate on 'v'
            KeyCode::Char('v') => TuiAction::Validate,

            // Attach to agent by number (1-9)
            KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
                let idx = c.to_digit(10).unwrap() as usize;
                TuiAction::Attach(idx)
            }

            _ => TuiAction::None,
        }
    }

    /// Render the TUI display
    pub fn render(&mut self) -> Result<()> {
        let mut stdout = io::stdout();

        // Move cursor up to overwrite previous render
        if self.last_render_height > 0 {
            execute!(
                stdout,
                cursor::MoveTo(
                    0,
                    cursor::position()?
                        .1
                        .saturating_sub(self.last_render_height)
                ),
                terminal::Clear(ClearType::FromCursorDown)
            )?;
        }

        let mut lines_rendered = 0u16;

        // Header
        execute!(
            stdout,
            SetForegroundColor(Color::Cyan),
            Print("=== Swarm Orchestrator ===\n"),
            ResetColor
        )?;
        lines_rendered += 1;

        // Wave progress
        if let Some(progress) = &self.wave_progress {
            let wave_str = format!(
                "Wave {}/{} | Tasks in wave: {}/{}\n",
                progress.current_wave + 1,
                progress.total_waves(),
                progress.completed_in_wave,
                progress.current_wave_tasks().len()
            );
            execute!(
                stdout,
                SetForegroundColor(Color::Yellow),
                Print(&wave_str),
                ResetColor
            )?;
            lines_rendered += 1;

            // Progress bar
            let bar = self.render_progress_bar(progress);
            execute!(stdout, Print(&bar), Print("\n"))?;
            lines_rendered += 1;
        }

        // Blank line
        execute!(stdout, Print("\n"))?;
        lines_rendered += 1;

        // Agent status list
        execute!(
            stdout,
            SetForegroundColor(Color::Green),
            Print("Agents:\n"),
            ResetColor
        )?;
        lines_rendered += 1;

        let agents = self.registry.list_all();
        if agents.is_empty() {
            execute!(stdout, Print("  (no agents spawned)\n"))?;
            lines_rendered += 1;
        } else {
            for (idx, agent) in agents.iter().enumerate() {
                let status_color = status_color(agent.status);
                let task_info = agent
                    .task_id
                    .as_ref()
                    .map(|t| format!(" [{}]", t))
                    .unwrap_or_default();
                let category_info = agent
                    .category
                    .as_ref()
                    .map(|c| format!(" ({})", c))
                    .unwrap_or_default();

                execute!(
                    stdout,
                    Print(format!("  [{}] ", idx + 1)),
                    SetForegroundColor(status_color),
                    Print(format!("{}", agent.status)),
                    ResetColor,
                    Print(format!(
                        " {}{}{}\n",
                        agent.pane_name, task_info, category_info
                    ))
                )?;
                lines_rendered += 1;
            }
        }

        // Blank line
        execute!(stdout, Print("\n"))?;
        lines_rendered += 1;

        // Controls help
        execute!(
            stdout,
            SetForegroundColor(Color::DarkGrey),
            Print("[1-9] attach  [v] validate  [q] quit\n"),
            ResetColor
        )?;
        lines_rendered += 1;

        stdout.flush()?;
        self.last_render_height = lines_rendered;

        Ok(())
    }

    /// Render a progress bar for wave completion
    fn render_progress_bar(&self, progress: &WaveProgress) -> String {
        let width = 30;
        let total = progress.total_tasks();
        let completed: usize = progress
            .waves
            .iter()
            .take(progress.current_wave)
            .map(|w| w.len())
            .sum::<usize>()
            + progress.completed_in_wave;

        let filled = if total > 0 {
            (completed * width) / total
        } else {
            0
        };
        let empty = width - filled;

        format!(
            "[{}{}] {}/{}",
            "=".repeat(filled),
            " ".repeat(empty),
            completed,
            total
        )
    }

    /// Stop the TUI
    pub fn stop(&mut self) {
        self.running = false;
    }

    /// Check if TUI is running
    pub fn is_running(&self) -> bool {
        self.running
    }
}

impl Default for SwarmTui {
    fn default() -> Self {
        Self::new()
    }
}

/// Get color for agent status
fn status_color(status: RegistryStatus) -> Color {
    match status {
        RegistryStatus::Spawning => Color::Yellow,
        RegistryStatus::Running => Color::Green,
        RegistryStatus::Idle => Color::Blue,
        RegistryStatus::Completed => Color::Cyan,
        RegistryStatus::Failed => Color::Red,
        RegistryStatus::Terminated => Color::DarkGrey,
    }
}

/// Create TUI from SCUD config (loads waves automatically)
pub fn create_tui_from_config(config: &Config) -> Result<SwarmTui> {
    let mut tui = SwarmTui::new();

    // Load waves from SCUD
    let waves = crate::scud::waves(config)?;
    if !waves.is_empty() {
        tui.set_waves(waves);
    }

    Ok(tui)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wave_progress() {
        let waves = vec![
            vec!["1".to_string()],
            vec!["2".to_string(), "3".to_string()],
            vec!["4".to_string()],
        ];
        let mut progress = WaveProgress::new(waves);

        assert_eq!(progress.total_waves(), 3);
        assert_eq!(progress.total_tasks(), 4);
        assert_eq!(progress.current_wave, 0);
        assert_eq!(progress.current_wave_tasks(), &["1".to_string()]);

        progress.complete_task();
        assert_eq!(progress.completed_in_wave, 1);

        progress.advance_wave();
        assert_eq!(progress.current_wave, 1);
        assert_eq!(progress.completed_in_wave, 0);
        assert_eq!(
            progress.current_wave_tasks(),
            &["2".to_string(), "3".to_string()]
        );
    }

    #[test]
    fn test_tui_action_from_key() {
        let tui = SwarmTui::new();

        // Test quit
        let key = KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
        assert_eq!(tui.handle_key(key), TuiAction::Quit);

        // Test validate
        let key = KeyEvent::new(KeyCode::Char('v'), KeyModifiers::NONE);
        assert_eq!(tui.handle_key(key), TuiAction::Validate);

        // Test attach
        let key = KeyEvent::new(KeyCode::Char('5'), KeyModifiers::NONE);
        assert_eq!(tui.handle_key(key), TuiAction::Attach(5));

        // Test Ctrl+C
        let key = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(tui.handle_key(key), TuiAction::Quit);

        // Test other key
        let key = KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE);
        assert_eq!(tui.handle_key(key), TuiAction::None);
    }

    #[test]
    fn test_progress_bar() {
        let waves = vec![
            vec!["1".to_string(), "2".to_string()],
            vec!["3".to_string(), "4".to_string()],
        ];
        let mut progress = WaveProgress::new(waves);
        let tui = SwarmTui::new();

        // 0/4 completed
        let bar = tui.render_progress_bar(&progress);
        assert!(bar.contains("0/4"));

        // Complete first wave
        progress.complete_task();
        progress.complete_task();
        progress.advance_wave();

        // 2/4 completed
        let bar = tui.render_progress_bar(&progress);
        assert!(bar.contains("2/4"));
    }

    #[test]
    fn test_tui_with_registry() {
        let mut registry = AgentRegistry::new();
        let handle = registry.spawn("test-pane", Some("task-1".to_string()));

        let tui = SwarmTui::with_registry(registry);
        assert_eq!(tui.registry().list_all().len(), 1);
        assert_eq!(tui.registry().list_all()[0].id, handle.id);
    }
}
