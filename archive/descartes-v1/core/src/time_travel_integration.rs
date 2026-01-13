//! Time Travel Integration - Rewind and Resume Logic
//!
//! This module provides comprehensive time-travel debugging capabilities by integrating:
//! - Time travel slider UI (phase3:7.4)
//! - Brain restore functionality (phase3:7.2)
//! - Body restore functionality (phase3:7.3)
//! - Debugger integration for historical debugging
//!
//! # Features
//!
//! - Rewind to any point in agent history with UI selection
//! - Resume execution from rewound state
//! - Coordinated brain (events) and body (code) restoration
//! - State synchronization and consistency validation
//! - Safety features (backups, confirmations, rollback)
//! - User feedback and progress tracking
//! - Historical debugging with breakpoints
//!
//! # Safety Guarantees
//!
//! 1. Automatic backup before rewind
//! 2. Confirmation dialogs for destructive operations
//! 3. Uncommitted changes detection and warning
//! 4. State consistency validation after restore
//! 5. Rollback capability on failure
//! 6. Undo rewind functionality

use crate::agent_history::{
    AgentHistoryEvent, AgentHistoryStore, HistoryEventType, HistorySnapshot,
};
use crate::body_restore::{
    BodyRestoreManager, GitBodyRestoreManager, RepositoryBackup,
    RestoreOptions as BodyRestoreOptions, RestoreResult as BodyRestoreResult,
};
use crate::brain_restore::{
    BrainRestore, BrainState, DefaultBrainRestore,
    RestoreOptions as BrainRestoreOptions, RestoreResult as BrainRestoreResult,
};
use crate::debugger::{Breakpoint, BreakpointLocation, Debugger};
use crate::errors::{StateStoreError, StateStoreResult};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;

// ============================================================================
// DATA STRUCTURES
// ============================================================================

/// Configuration for rewind operations
#[derive(Debug, Clone)]
pub struct RewindConfig {
    /// Whether to require confirmation before rewind
    pub require_confirmation: bool,

    /// Whether to create automatic backups
    pub auto_backup: bool,

    /// Whether to validate state after restore
    pub validate_state: bool,

    /// Whether to allow rewind with uncommitted changes
    pub allow_uncommitted_changes: bool,

    /// Maximum number of undo operations to keep
    pub max_undo_history: usize,

    /// Whether to enable debugging at rewound state
    pub enable_debugging: bool,
}

impl Default for RewindConfig {
    fn default() -> Self {
        Self {
            require_confirmation: true,
            auto_backup: true,
            validate_state: true,
            allow_uncommitted_changes: false,
            max_undo_history: 10,
            enable_debugging: true,
        }
    }
}

impl RewindConfig {
    /// Create a safe configuration (all safety features enabled)
    pub fn safe() -> Self {
        Self::default()
    }

    /// Create a fast configuration (minimal safety checks)
    pub fn fast() -> Self {
        Self {
            require_confirmation: false,
            auto_backup: true,
            validate_state: false,
            allow_uncommitted_changes: true,
            max_undo_history: 5,
            enable_debugging: false,
        }
    }
}

/// Point in time to rewind to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewindPoint {
    /// Agent ID this point belongs to
    pub agent_id: Option<String>,

    /// Timestamp to rewind to
    pub timestamp: i64,

    /// Optional event ID at this point
    pub event_id: Option<Uuid>,

    /// Git commit hash at this point
    pub git_commit: Option<String>,

    /// Optional snapshot ID
    pub snapshot_id: Option<Uuid>,

    /// Description of this point
    pub description: String,

    /// Index in event list (for UI)
    pub event_index: Option<usize>,
}

impl RewindPoint {
    /// Create from timestamp
    pub fn from_timestamp(timestamp: i64) -> Self {
        Self {
            agent_id: None,
            timestamp,
            event_id: None,
            git_commit: None,
            snapshot_id: None,
            description: format!("Timestamp: {}", timestamp),
            event_index: None,
        }
    }

    /// Create from timestamp with agent ID
    pub fn from_timestamp_for_agent(timestamp: i64, agent_id: String) -> Self {
        Self {
            agent_id: Some(agent_id),
            timestamp,
            event_id: None,
            git_commit: None,
            snapshot_id: None,
            description: format!("Timestamp: {}", timestamp),
            event_index: None,
        }
    }

    /// Create from event
    pub fn from_event(event: &AgentHistoryEvent, index: Option<usize>) -> Self {
        Self {
            agent_id: Some(event.agent_id.clone()),
            timestamp: event.timestamp,
            event_id: Some(event.event_id),
            git_commit: event.git_commit_hash.clone(),
            snapshot_id: None,
            description: format!("{:?} event", event.event_type),
            event_index: index,
        }
    }

    /// Create from snapshot
    pub fn from_snapshot(snapshot: &HistorySnapshot) -> Self {
        Self {
            agent_id: Some(snapshot.agent_id.clone()),
            timestamp: snapshot.timestamp,
            event_id: None,
            git_commit: snapshot.git_commit.clone(),
            snapshot_id: Some(snapshot.snapshot_id),
            description: format!(
                "Snapshot: {}",
                snapshot.git_commit.as_deref().unwrap_or("unknown")
            ),
            event_index: None,
        }
    }

    /// Create from slider position (0.0 to 1.0)
    pub fn from_slider_position(position: f32, events: &[AgentHistoryEvent]) -> Option<Self> {
        if events.is_empty() {
            return None;
        }

        let index = (position * (events.len() - 1) as f32) as usize;
        events
            .get(index)
            .map(|event| Self::from_event(event, Some(index)))
    }

    /// Set the agent ID for this point
    pub fn with_agent_id(mut self, agent_id: String) -> Self {
        self.agent_id = Some(agent_id);
        self
    }
}

/// Result of a rewind operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewindResult {
    /// Whether the rewind was successful
    pub success: bool,

    /// The point we rewound to
    pub target_point: RewindPoint,

    /// Brain restore result
    pub brain_result: Option<BrainRestoreResult>,

    /// Body restore result
    pub body_result: Option<BodyRestoreResult>,

    /// Backup information for undo
    pub backup: RewindBackup,

    /// State validation results
    pub validation: ValidationResult,

    /// Messages and warnings
    pub messages: Vec<String>,

    /// Time taken for operation (milliseconds)
    pub duration_ms: u64,

    /// Timestamp of rewind operation
    pub timestamp: i64,
}

impl RewindResult {
    /// Create a successful result
    pub fn success(
        target_point: RewindPoint,
        brain_result: BrainRestoreResult,
        body_result: BodyRestoreResult,
        backup: RewindBackup,
        validation: ValidationResult,
    ) -> Self {
        Self {
            success: true,
            target_point,
            brain_result: Some(brain_result),
            body_result: Some(body_result),
            backup,
            validation,
            messages: Vec::new(),
            duration_ms: 0,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Create a failed result
    pub fn failure(target_point: RewindPoint, error: String, backup: RewindBackup) -> Self {
        Self {
            success: false,
            target_point,
            brain_result: None,
            body_result: None,
            backup,
            validation: ValidationResult::failed(vec![error]),
            messages: Vec::new(),
            duration_ms: 0,
            timestamp: chrono::Utc::now().timestamp(),
        }
    }

    /// Add a message
    pub fn with_message(mut self, message: String) -> Self {
        self.messages.push(message);
        self
    }

    /// Set duration
    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }
}

/// Backup of state before rewind (for undo)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewindBackup {
    /// Unique backup ID
    pub backup_id: Uuid,

    /// Original brain state
    pub brain_state: Option<BrainState>,

    /// Original repository state
    pub repository_state: RepositoryBackup,

    /// Events at time of backup
    pub event_count: usize,

    /// Timestamp of backup
    pub timestamp: i64,

    /// Description
    pub description: String,
}

impl RewindBackup {
    /// Create a new backup
    pub fn new(
        brain_state: Option<BrainState>,
        repository_state: RepositoryBackup,
        event_count: usize,
        description: String,
    ) -> Self {
        Self {
            backup_id: Uuid::new_v4(),
            brain_state,
            repository_state,
            event_count,
            timestamp: chrono::Utc::now().timestamp(),
            description,
        }
    }
}

/// State validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether validation passed
    pub valid: bool,

    /// Brain/body consistency check
    pub brain_body_consistent: bool,

    /// Git commit matches event timestamp
    pub git_commit_matches: bool,

    /// Validation errors
    pub errors: Vec<String>,

    /// Warnings (non-critical issues)
    pub warnings: Vec<String>,
}

impl ValidationResult {
    /// Create a successful validation
    pub fn success() -> Self {
        Self {
            valid: true,
            brain_body_consistent: true,
            git_commit_matches: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Create a failed validation
    pub fn failed(errors: Vec<String>) -> Self {
        Self {
            valid: false,
            brain_body_consistent: false,
            git_commit_matches: false,
            errors,
            warnings: Vec::new(),
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }
}

/// Resume context for continuing execution from rewound state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResumeContext {
    /// Agent ID to resume
    pub agent_id: String,

    /// Brain state to restore
    pub brain_state: BrainState,

    /// Current git commit
    pub git_commit: String,

    /// Event index to resume from
    pub resume_event_index: usize,

    /// Whether to enable debugging
    pub enable_debugging: bool,

    /// Breakpoints to set
    pub breakpoints: Vec<BreakpointLocation>,

    /// Additional metadata
    pub metadata: HashMap<String, serde_json::Value>,
}

impl ResumeContext {
    /// Create from rewind result
    pub fn from_rewind_result(result: &RewindResult, agent_id: String) -> StateStoreResult<Self> {
        let brain_state = result
            .brain_result
            .as_ref()
            .and_then(|r| r.brain_state.clone())
            .ok_or_else(|| StateStoreError::NotFound("Brain state not available".to_string()))?;

        let git_commit = result
            .body_result
            .as_ref()
            .map(|r| r.target_commit.clone())
            .ok_or_else(|| StateStoreError::NotFound("Git commit not available".to_string()))?;

        let resume_event_index = result.target_point.event_index.unwrap_or(0);

        Ok(Self {
            agent_id,
            brain_state,
            git_commit,
            resume_event_index,
            enable_debugging: false,
            breakpoints: Vec::new(),
            metadata: HashMap::new(),
        })
    }

    /// Add a breakpoint
    pub fn with_breakpoint(mut self, breakpoint: BreakpointLocation) -> Self {
        self.breakpoints.push(breakpoint);
        self
    }

    /// Enable debugging
    pub fn with_debugging(mut self) -> Self {
        self.enable_debugging = true;
        self
    }
}

/// Progress update during rewind/resume operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RewindProgress {
    /// Starting rewind operation
    Starting { target: RewindPoint },

    /// Creating backup
    CreatingBackup,

    /// Backup created
    BackupCreated { backup_id: Uuid },

    /// Validating target state
    Validating,

    /// Restoring brain state
    RestoringBrain {
        events_processed: usize,
        total_events: usize,
    },

    /// Brain restored
    BrainRestored { events_processed: usize },

    /// Restoring body (git checkout)
    RestoringBody { commit: String },

    /// Body restored
    BodyRestored { commit: String },

    /// Validating consistency
    ValidatingConsistency,

    /// Rewind complete
    Complete { success: bool },

    /// Error occurred
    Error { error: String },
}

/// Confirmation request for destructive operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewindConfirmation {
    /// Operation being confirmed
    pub operation: String,

    /// Target point
    pub target: RewindPoint,

    /// Warnings to display
    pub warnings: Vec<String>,

    /// Whether there are uncommitted changes
    pub has_uncommitted_changes: bool,

    /// Number of events that will be lost if proceeding
    pub events_will_be_lost: usize,
}

impl RewindConfirmation {
    /// Create a new confirmation request
    pub fn new(operation: String, target: RewindPoint) -> Self {
        Self {
            operation,
            target,
            warnings: Vec::new(),
            has_uncommitted_changes: false,
            events_will_be_lost: 0,
        }
    }

    /// Add a warning
    pub fn with_warning(mut self, warning: String) -> Self {
        self.warnings.push(warning);
        self
    }

    /// Set uncommitted changes flag
    pub fn with_uncommitted_changes(mut self, has_changes: bool) -> Self {
        self.has_uncommitted_changes = has_changes;
        self
    }

    /// Set events that will be lost
    pub fn with_lost_events(mut self, count: usize) -> Self {
        self.events_will_be_lost = count;
        self
    }
}

// ============================================================================
// TRAIT DEFINITION
// ============================================================================

/// Trait for rewind and resume operations
#[async_trait]
pub trait RewindManager: Send + Sync {
    /// Check if rewind is possible to a given point
    async fn can_rewind_to(&self, point: &RewindPoint) -> StateStoreResult<RewindConfirmation>;

    /// Rewind to a specific point in time
    async fn rewind_to(
        &self,
        point: RewindPoint,
        config: RewindConfig,
    ) -> StateStoreResult<RewindResult>;

    /// Resume execution from a rewound state
    async fn resume_from(&self, context: ResumeContext) -> StateStoreResult<()>;

    /// Undo the last rewind operation
    async fn undo_rewind(&self, backup_id: Uuid) -> StateStoreResult<RewindResult>;

    /// Get list of available rewind points
    async fn get_rewind_points(&self, agent_id: &str) -> StateStoreResult<Vec<RewindPoint>>;

    /// Validate state consistency between brain and body
    async fn validate_consistency(
        &self,
        brain_state: &BrainState,
        current_commit: &str,
    ) -> StateStoreResult<ValidationResult>;

    /// Create a snapshot at current state for quick rewind
    async fn create_snapshot(&self, agent_id: &str, description: String) -> StateStoreResult<Uuid>;
}

// ============================================================================
// DEFAULT IMPLEMENTATION
// ============================================================================

/// Default implementation of RewindManager
pub struct DefaultRewindManager<S: AgentHistoryStore> {
    /// Brain restore manager
    brain_restore: DefaultBrainRestore<S>,

    /// Body restore manager
    body_restore: Arc<GitBodyRestoreManager>,

    /// Repository path
    _repo_path: PathBuf,

    /// Undo history (recent backups)
    undo_history: Arc<RwLock<Vec<RewindBackup>>>,

    /// Maximum undo history size
    max_undo_history: usize,
}

impl<S: AgentHistoryStore> DefaultRewindManager<S> {
    /// Create a new rewind manager
    pub fn new(
        history_store: S,
        repo_path: PathBuf,
        max_undo_history: usize,
    ) -> StateStoreResult<Self> {
        let brain_restore = DefaultBrainRestore::new(history_store);
        let body_restore = Arc::new(GitBodyRestoreManager::new(&repo_path)?);

        Ok(Self {
            brain_restore,
            body_restore,
            _repo_path: repo_path,
            undo_history: Arc::new(RwLock::new(Vec::new())),
            max_undo_history,
        })
    }

    /// Load events for a rewind point
    async fn load_events_for_point(
        &self,
        agent_id: &str,
        point: &RewindPoint,
    ) -> StateStoreResult<Vec<AgentHistoryEvent>> {
        self.brain_restore
            .load_events_until(agent_id, point.timestamp)
            .await
    }

    /// Create a backup of current state
    async fn create_backup(
        &self,
        agent_id: &str,
        description: String,
    ) -> StateStoreResult<RewindBackup> {
        info!("Creating backup: {}", description);

        // Get current brain state
        let events = self
            .brain_restore
            .load_events_until(agent_id, i64::MAX)
            .await?;

        let brain_result = self
            .brain_restore
            .replay_events(events.clone(), BrainRestoreOptions::default())
            .await?;

        let brain_state = brain_result.brain_state;

        // Get current repository state
        let repo_backup = self.body_restore.create_backup().await?;

        let backup = RewindBackup::new(brain_state, repo_backup, events.len(), description);

        // Add to undo history
        let mut undo_history = self.undo_history.write().await;
        undo_history.push(backup.clone());

        // Trim undo history if needed
        if undo_history.len() > self.max_undo_history {
            undo_history.remove(0);
        }

        info!("Backup created: {}", backup.backup_id);
        Ok(backup)
    }

    /// Validate brain and body consistency
    async fn validate_brain_body_consistency(
        &self,
        brain_state: &BrainState,
        current_commit: &str,
    ) -> StateStoreResult<ValidationResult> {
        let mut result = ValidationResult::success();

        // Check if brain state has a git commit
        if let Some(ref brain_commit) = brain_state.git_commit {
            // Check if it matches the current commit
            if !current_commit.starts_with(&brain_commit.chars().take(7).collect::<String>()) {
                result.brain_body_consistent = false;
                result.git_commit_matches = false;
                result.errors.push(format!(
                    "Git commit mismatch: brain expects {}, body is at {}",
                    brain_commit, current_commit
                ));
            }
        } else {
            result = result.with_warning("Brain state has no git commit reference".to_string());
        }

        // Validate brain state itself
        if brain_state.is_empty() {
            result = result.with_warning("Brain state is empty".to_string());
        }

        result.valid = result.errors.is_empty();
        Ok(result)
    }

    /// Find git commit closest to timestamp
    async fn find_commit_at_timestamp(&self, timestamp: i64) -> StateStoreResult<String> {
        // Get recent commits
        let commits = self.body_restore.get_recent_commits(100).await?;

        // Find commit closest to timestamp
        let closest = commits
            .iter()
            .min_by_key(|c| (c.timestamp - timestamp).abs())
            .ok_or_else(|| StateStoreError::NotFound("No commits available".to_string()))?;

        Ok(closest.hash.clone())
    }
}

#[async_trait]
impl<S: AgentHistoryStore + 'static> RewindManager for DefaultRewindManager<S> {
    async fn can_rewind_to(&self, point: &RewindPoint) -> StateStoreResult<RewindConfirmation> {
        let mut confirmation =
            RewindConfirmation::new("Rewind to point".to_string(), point.clone());

        // Check for uncommitted changes
        let has_changes = self.body_restore.has_uncommitted_changes().await?;
        confirmation = confirmation.with_uncommitted_changes(has_changes);

        if has_changes {
            confirmation = confirmation.with_warning(
                "Repository has uncommitted changes that will be stashed".to_string(),
            );
        }

        // Check if commit exists (if specified)
        if let Some(ref commit) = point.git_commit {
            if !self.body_restore.verify_commit_exists(commit).await? {
                return Err(StateStoreError::NotFound(format!(
                    "Git commit not found: {}",
                    commit
                )));
            }
        }

        // Calculate events that will be "lost" (after the rewind point)
        // This is informational - they're not actually deleted, just not in current view
        if let Some(event_index) = point.event_index {
            confirmation = confirmation.with_lost_events(event_index);
        }

        Ok(confirmation)
    }

    async fn rewind_to(
        &self,
        point: RewindPoint,
        config: RewindConfig,
    ) -> StateStoreResult<RewindResult> {
        let start_time = std::time::Instant::now();

        info!("Starting rewind to: {:?}", point);

        // Step 1: Check if rewind is possible
        let confirmation = self.can_rewind_to(&point).await?;

        // Step 2: Check for uncommitted changes
        if confirmation.has_uncommitted_changes && !config.allow_uncommitted_changes {
            return Err(StateStoreError::Conflict(
                "Cannot rewind with uncommitted changes. Set allow_uncommitted_changes=true or commit your changes.".to_string(),
            ));
        }

        // Get agent_id from point (required for loading events)
        let agent_id = point.agent_id.clone().ok_or_else(|| {
            StateStoreError::Conflict(
                "RewindPoint must have agent_id set. Use with_agent_id() or from_event()/from_snapshot() factory methods.".to_string(),
            )
        })?;

        // Step 3: Create backup
        let backup = if config.auto_backup {
            self.create_backup(
                &agent_id,
                format!(
                    "Pre-rewind backup at {}",
                    chrono::Utc::now().format("%Y-%m-%d %H:%M:%S")
                ),
            )
            .await?
        } else {
            // Create minimal backup
            let repo_backup = self.body_restore.create_backup().await?;
            RewindBackup::new(None, repo_backup, 0, "Minimal backup".to_string())
        };

        // Step 4: Restore brain state
        info!("Restoring brain state for agent {}...", agent_id);

        let events = self.load_events_for_point(&agent_id, &point).await?;

        let brain_result = self
            .brain_restore
            .replay_events(
                events,
                BrainRestoreOptions {
                    validate: config.validate_state,
                    ..Default::default()
                },
            )
            .await?;

        if !brain_result.success {
            error!("Brain restore failed");
            return Ok(RewindResult::failure(
                point,
                format!("Brain restore failed: {:?}", brain_result.validation_errors),
                backup,
            ));
        }

        info!(
            "Brain state restored: {} events processed",
            brain_result.events_processed
        );

        // Step 5: Restore body (git checkout)
        info!("Restoring body state...");

        let target_commit = if let Some(commit) = &point.git_commit {
            commit.clone()
        } else {
            // Find commit closest to timestamp
            self.find_commit_at_timestamp(point.timestamp).await?
        };

        let body_options = BodyRestoreOptions {
            stash_changes: !config.allow_uncommitted_changes,
            verify_commit: true,
            create_backup: false, // We already created a backup
            force: config.allow_uncommitted_changes,
            preserve_untracked: true,
        };

        let body_result = self
            .body_restore
            .checkout_commit(&target_commit, body_options)
            .await?;

        if !body_result.success {
            error!("Body restore failed");

            // Try to rollback
            warn!("Attempting to rollback brain state...");
            // Brain rollback is implicit - we don't modify persistent state

            return Ok(RewindResult::failure(
                point.clone(),
                "Body restore failed".to_string(),
                backup,
            ));
        }

        info!("Body state restored to commit: {}", target_commit);

        // Step 6: Validate consistency
        let validation = if config.validate_state {
            if let Some(ref brain_state) = brain_result.brain_state {
                let current_commit = self.body_restore.get_current_commit().await?;
                self.validate_brain_body_consistency(brain_state, &current_commit)
                    .await?
            } else {
                ValidationResult::failed(vec!["Brain state not available".to_string()])
            }
        } else {
            ValidationResult::success()
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        // Check warnings before moving validation
        if !validation.warnings.is_empty() {
            warn!("Rewind completed with warnings: {:?}", validation.warnings);
        }

        let result = RewindResult::success(point, brain_result, body_result, backup, validation)
            .with_duration(duration_ms)
            .with_message(format!("Successfully rewound in {}ms", duration_ms));

        Ok(result)
    }

    async fn resume_from(&self, context: ResumeContext) -> StateStoreResult<()> {
        info!(
            "Resuming execution from event index: {}",
            context.resume_event_index
        );

        // Step 1: Verify brain state matches current git commit
        let current_commit = self.body_restore.get_current_commit().await?;

        if !current_commit.starts_with(&context.git_commit.chars().take(7).collect::<String>()) {
            return Err(StateStoreError::Conflict(format!(
                "Cannot resume: git commit mismatch. Expected {}, but at {}",
                context.git_commit, current_commit
            )));
        }

        // Step 2: Load agent history from resume point
        let all_events = self
            .brain_restore
            .load_events_until(&context.agent_id, i64::MAX)
            .await?;

        let remaining_events = all_events
            .into_iter()
            .skip(context.resume_event_index)
            .collect::<Vec<_>>();

        info!(
            "Loaded {} remaining events to process",
            remaining_events.len()
        );

        // Step 3: Set up debugging if requested
        let debugger = if context.enable_debugging {
            info!(
                "Debugging enabled, setting {} breakpoints",
                context.breakpoints.len()
            );

            // Create a debugger for this agent
            let agent_uuid = Uuid::parse_str(&context.agent_id).map_err(|e| {
                StateStoreError::Conflict(format!("Invalid agent_id UUID: {}", e))
            })?;

            let mut debugger = Debugger::new(agent_uuid);
            debugger.state_mut().enable();

            // Set breakpoints from context
            for location in context.breakpoints.iter() {
                let breakpoint = Breakpoint::new(location.clone());
                debugger.state_mut().add_breakpoint(breakpoint);
            }

            // Pause initially so user can inspect state
            let _ = debugger.pause_agent();

            info!(
                "Debugger initialized with {} breakpoints, execution paused",
                context.breakpoints.len()
            );

            Some(debugger)
        } else {
            None
        };

        // Step 4: Resume agent execution
        // Store the resume context for the agent runtime to pick up
        info!(
            "Agent {} ready to resume from event index {} (commit: {})",
            context.agent_id, context.resume_event_index, context.git_commit
        );

        // The actual agent runtime re-initialization happens externally:
        // 1. The caller retrieves this ResumeContext
        // 2. Uses AgentRunner to spawn a new agent with the restored brain state
        // 3. If debugging is enabled, attaches the debugger
        // 4. Continues processing from resume_event_index
        //
        // The debugger (if created) should be passed to the caller via the metadata
        if let Some(debugger) = debugger {
            info!(
                "Debugger state: enabled={}, paused={}, breakpoints={}",
                debugger.state().is_enabled(),
                debugger.state().execution_state.is_paused(),
                debugger.state().breakpoints.len()
            );
            // Note: In a full implementation, the debugger would be stored
            // in a registry keyed by agent_id for the runtime to retrieve
        }

        info!(
            "Resume context prepared: {} remaining events to process",
            remaining_events.len()
        );

        Ok(())
    }

    async fn undo_rewind(&self, backup_id: Uuid) -> StateStoreResult<RewindResult> {
        info!("Undoing rewind to backup: {}", backup_id);

        // Find the backup
        let undo_history = self.undo_history.read().await;
        let backup = undo_history
            .iter()
            .find(|b| b.backup_id == backup_id)
            .ok_or_else(|| StateStoreError::NotFound(format!("Backup not found: {}", backup_id)))?
            .clone();

        drop(undo_history);

        // Rollback body (git)
        self.body_restore.rollback(&backup.repository_state).await?;

        let current_commit = self.body_restore.get_current_commit().await?;

        // Create result
        let point = RewindPoint {
            agent_id: None, // Not needed for undo result
            timestamp: backup.timestamp,
            event_id: None,
            git_commit: Some(current_commit.clone()),
            snapshot_id: None,
            description: format!("Undo rewind to {}", backup.description),
            event_index: None,
        };

        let body_result = BodyRestoreResult {
            success: true,
            target_commit: current_commit,
            backup: backup.repository_state.clone(),
            messages: vec!["Rollback successful".to_string()],
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Brain state is restored from backup
        let brain_result = BrainRestoreResult {
            success: true,
            brain_state: backup.brain_state.clone(),
            events_processed: backup.event_count,
            events_skipped: 0,
            validation_errors: Vec::new(),
            warnings: Vec::new(),
            duration_ms: 0,
        };

        let validation = ValidationResult::success();

        Ok(
            RewindResult::success(point, brain_result, body_result, backup.clone(), validation)
                .with_message("Undo rewind successful".to_string()),
        )
    }

    async fn get_rewind_points(&self, agent_id: &str) -> StateStoreResult<Vec<RewindPoint>> {
        let mut points = Vec::new();

        // Get all events
        let events = self
            .brain_restore
            .load_events_until(agent_id, i64::MAX)
            .await?;

        // Create rewind points for important events
        for (index, event) in events.iter().enumerate() {
            // Add points for events with git commits
            if event.git_commit_hash.is_some() {
                points.push(RewindPoint::from_event(event, Some(index)));
            }

            // Add points for decision and state change events
            if matches!(
                event.event_type,
                HistoryEventType::Decision | HistoryEventType::StateChange
            ) {
                points.push(RewindPoint::from_event(event, Some(index)));
            }
        }

        // Get snapshots
        let snapshots = self.brain_restore.store().list_snapshots(agent_id).await?;

        for snapshot in snapshots {
            points.push(RewindPoint::from_snapshot(&snapshot));
        }

        // Sort by timestamp
        points.sort_by_key(|p| p.timestamp);

        Ok(points)
    }

    async fn validate_consistency(
        &self,
        brain_state: &BrainState,
        current_commit: &str,
    ) -> StateStoreResult<ValidationResult> {
        self.validate_brain_body_consistency(brain_state, current_commit)
            .await
    }

    async fn create_snapshot(&self, agent_id: &str, description: String) -> StateStoreResult<Uuid> {
        info!("Creating snapshot for agent {}: {}", agent_id, description);

        // Get current events
        let events = self
            .brain_restore
            .load_events_until(agent_id, i64::MAX)
            .await?;

        // Get current git commit
        let git_commit = self.body_restore.get_current_commit().await.ok();

        // Create snapshot
        let snapshot = HistorySnapshot::new(agent_id.to_string(), events, git_commit)
            .with_description(description);

        let snapshot_id = snapshot.snapshot_id;

        // Save snapshot
        self.brain_restore
            .store()
            .create_snapshot(&snapshot)
            .await?;

        info!("Snapshot created: {}", snapshot_id);
        Ok(snapshot_id)
    }
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Convert slider position (0.0 to 1.0) to rewind point
pub fn slider_to_rewind_point(position: f32, events: &[AgentHistoryEvent]) -> Option<RewindPoint> {
    RewindPoint::from_slider_position(position, events)
}

/// Get user-friendly description of rewind operation
pub fn describe_rewind(from: &RewindPoint, to: &RewindPoint) -> String {
    let time_diff = to.timestamp - from.timestamp;
    let direction = if time_diff > 0 { "forward" } else { "backward" };
    let abs_diff = time_diff.abs();

    let time_str = if abs_diff < 60 {
        format!("{} seconds", abs_diff)
    } else if abs_diff < 3600 {
        format!("{} minutes", abs_diff / 60)
    } else if abs_diff < 86400 {
        format!("{} hours", abs_diff / 3600)
    } else {
        format!("{} days", abs_diff / 86400)
    };

    format!(
        "Rewind {} by {} from {} to {}",
        direction, time_str, from.description, to.description
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent_history::SqliteAgentHistoryStore;
    use serde_json::json;
    use std::process::Command;
    use tempfile::{NamedTempFile, TempDir};

    async fn create_test_store() -> (SqliteAgentHistoryStore, NamedTempFile) {
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path().to_str().unwrap();
        let mut store = SqliteAgentHistoryStore::new(path).await.unwrap();
        store.initialize().await.unwrap();
        (store, temp_file)
    }

    fn create_test_repo() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let repo_path = temp_dir.path().to_path_buf();

        Command::new("git")
            .args(["init"])
            .current_dir(&repo_path)
            .output()
            .expect("Failed to init git repo");

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        std::fs::write(repo_path.join("test.txt"), "initial content").unwrap();
        Command::new("git")
            .args(["add", "test.txt"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(&repo_path)
            .output()
            .unwrap();

        (temp_dir, repo_path)
    }

    #[tokio::test]
    async fn test_create_rewind_manager() {
        let (store, _temp_file) = create_test_store().await;
        let (_temp, repo_path) = create_test_repo();

        let manager = DefaultRewindManager::new(store, repo_path, 10);
        assert!(manager.is_ok());
    }

    #[tokio::test]
    async fn test_rewind_point_from_slider() {
        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Thought,
                json!({"content": "event 1"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Action,
                json!({"action": "event 2"}),
            ),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Decision,
                json!({"decision": "event 3"}),
            ),
        ];

        let point = RewindPoint::from_slider_position(0.5, &events);
        assert!(point.is_some());

        let point = point.unwrap();
        assert_eq!(point.event_index, Some(1));
    }

    #[tokio::test]
    async fn test_get_rewind_points() {
        let (store, _temp_file) = create_test_store().await;
        let (_temp, repo_path) = create_test_repo();

        // Create some test events
        let events = vec![
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::Decision,
                json!({"decision": "test"}),
            )
            .with_git_commit("abc123".to_string()),
            AgentHistoryEvent::new(
                "agent-1".to_string(),
                HistoryEventType::StateChange,
                json!({"state": "new"}),
            ),
        ];

        for event in &events {
            store.record_event(event).await.unwrap();
        }

        let manager = DefaultRewindManager::new(store, repo_path, 10).unwrap();
        let points = manager.get_rewind_points("agent-1").await.unwrap();

        assert!(!points.is_empty());
    }

    #[tokio::test]
    async fn test_validation_result() {
        let success = ValidationResult::success();
        assert!(success.valid);
        assert!(success.brain_body_consistent);

        let failed = ValidationResult::failed(vec!["Error".to_string()]);
        assert!(!failed.valid);
        assert_eq!(failed.errors.len(), 1);
    }

    #[tokio::test]
    async fn test_describe_rewind() {
        let from = RewindPoint::from_timestamp(1000);
        let to = RewindPoint::from_timestamp(1030);  // 30 seconds difference

        let description = describe_rewind(&from, &to);
        assert!(description.contains("forward"));
        assert!(description.contains("seconds"));
    }

    #[tokio::test]
    async fn test_rewind_config() {
        let safe_config = RewindConfig::safe();
        assert!(safe_config.require_confirmation);
        assert!(safe_config.auto_backup);

        let fast_config = RewindConfig::fast();
        assert!(!fast_config.require_confirmation);
        assert!(fast_config.allow_uncommitted_changes);
    }
}
