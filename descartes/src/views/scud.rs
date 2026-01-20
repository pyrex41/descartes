//! SCUD display functions

use serde::Serialize;

use crate::{Config, Result};

/// Output format for waves command
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WavesOutputFormat {
    /// Human-readable table format (default)
    Human,
    /// Complete JSON output
    Json,
    /// Streaming JSON events (one per wave)
    JsonEvents,
}

/// JSON output structure for waves
#[derive(Debug, Serialize)]
pub struct WavesOutput {
    /// List of waves, each containing task IDs that can run in parallel
    pub waves: Vec<WaveInfo>,
    /// Total number of tasks across all waves
    pub total_tasks: usize,
    /// Total number of waves
    pub total_waves: usize,
}

/// Information about a single wave
#[derive(Debug, Serialize)]
pub struct WaveInfo {
    /// Wave number (1-indexed for human readability)
    pub wave_number: usize,
    /// Task IDs in this wave
    pub task_ids: Vec<String>,
    /// Number of tasks in this wave
    pub task_count: usize,
}

/// JSON event for streaming output
#[derive(Debug, Serialize)]
pub struct WaveEvent {
    /// Event type (always "wave" for now)
    #[serde(rename = "type")]
    pub event_type: &'static str,
    /// Wave number (1-indexed)
    pub wave_number: usize,
    /// Task IDs in this wave
    pub task_ids: Vec<String>,
    /// Number of tasks in this wave
    pub task_count: usize,
    /// Whether this is the last wave
    pub is_final: bool,
}

/// Show task waves with configurable output format
pub fn show_waves_with_format(config: &Config, format: WavesOutputFormat) -> Result<()> {
    let waves = crate::scud::waves(config)?;

    match format {
        WavesOutputFormat::Human => show_waves_human(&waves),
        WavesOutputFormat::Json => show_waves_json(&waves),
        WavesOutputFormat::JsonEvents => show_waves_json_events(&waves),
    }
}

/// Show task waves (default human-readable format)
pub fn show_waves(config: &Config) -> Result<()> {
    show_waves_with_format(config, WavesOutputFormat::Human)
}

/// Display waves in human-readable format
fn show_waves_human(waves: &[Vec<String>]) -> Result<()> {
    if waves.is_empty() {
        println!("No pending tasks found.");
        return Ok(());
    }

    let total_tasks: usize = waves.iter().map(|w| w.len()).sum();

    println!("Task Waves (Parallel Execution Potential)");
    println!("=========================================\n");

    for (idx, wave) in waves.iter().enumerate() {
        println!(
            "Wave {} ({} task{}):",
            idx + 1,
            wave.len(),
            if wave.len() == 1 { "" } else { "s" }
        );
        for task_id in wave {
            println!("  - {}", task_id);
        }
        println!();
    }

    println!(
        "Total: {} task{} in {} wave{}",
        total_tasks,
        if total_tasks == 1 { "" } else { "s" },
        waves.len(),
        if waves.len() == 1 { "" } else { "s" }
    );

    Ok(())
}

/// Display waves in JSON format
fn show_waves_json(waves: &[Vec<String>]) -> Result<()> {
    let total_tasks: usize = waves.iter().map(|w| w.len()).sum();

    let output = WavesOutput {
        waves: waves
            .iter()
            .enumerate()
            .map(|(idx, task_ids)| WaveInfo {
                wave_number: idx + 1,
                task_ids: task_ids.clone(),
                task_count: task_ids.len(),
            })
            .collect(),
        total_tasks,
        total_waves: waves.len(),
    };

    let json = serde_json::to_string_pretty(&output)?;
    println!("{}", json);

    Ok(())
}

/// Display waves as streaming JSON events (one line per wave)
fn show_waves_json_events(waves: &[Vec<String>]) -> Result<()> {
    let total_waves = waves.len();

    for (idx, task_ids) in waves.iter().enumerate() {
        let event = WaveEvent {
            event_type: "wave",
            wave_number: idx + 1,
            task_ids: task_ids.clone(),
            task_count: task_ids.len(),
            is_final: idx == total_waves - 1,
        };

        let json = serde_json::to_string(&event)?;
        println!("{}", json);
    }

    Ok(())
}
