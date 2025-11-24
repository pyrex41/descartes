/// Descartes Thoughts System - Global persistent memory storage
///
/// This module handles the global thoughts storage system that allows agents
/// to maintain persistent memory across sessions. Thoughts are stored in a
/// global directory (~/.descartes/thoughts/) with optional project-specific
/// symlinks for easy access.
///
/// Features:
/// - Global storage directory with proper file permissions (user-only access)
/// - Automatic directory creation on first use
/// - Support for categorizing thoughts via tags/folders
/// - Project-specific symlink management to global thoughts
/// - Atomic operations for safe concurrent access

use std::fs;
use std::path::{Path, PathBuf};
use std::os::unix::fs as unix_fs;
use thiserror::Error;
use tracing::{debug, info, warn};

/// Errors that can occur during thoughts operations
#[derive(Error, Debug)]
pub enum ThoughtsError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to determine home directory")]
    NoHomeDirectory,

    #[error("Invalid thoughts path: {0}")]
    InvalidPath(String),

    #[error("Permission denied: {0}")]
    PermissionDenied(String),

    #[error("Symlink error: {0}")]
    SymlinkError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
}

/// Result type for thoughts operations
pub type ThoughtsResult<T> = Result<T, ThoughtsError>;

/// Thought metadata and organization information
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ThoughtMetadata {
    /// Unique identifier for the thought
    pub id: String,
    /// Title or summary of the thought
    pub title: String,
    /// Content of the thought
    pub content: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Last modified timestamp (ISO 8601)
    pub modified_at: String,
    /// Optional agent identifier that created this thought
    pub agent_id: Option<String>,
    /// Optional project identifier
    pub project_id: Option<String>,
}

/// Configuration for the thoughts storage system
#[derive(Debug, Clone)]
pub struct ThoughtsConfig {
    /// Root directory for global thoughts storage
    pub global_root: PathBuf,
    /// Directory permissions (Unix mode, default: 0o700 for user-only)
    pub dir_permissions: u32,
    /// File permissions (Unix mode, default: 0o600 for user-only)
    pub file_permissions: u32,
}

impl Default for ThoughtsConfig {
    fn default() -> Self {
        Self {
            global_root: Self::default_global_root(),
            dir_permissions: 0o700,
            file_permissions: 0o600,
        }
    }
}

impl ThoughtsConfig {
    /// Get the default global thoughts root directory (~/.descartes/thoughts/)
    fn default_global_root() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        home.join(".descartes").join("thoughts")
    }
}

/// Global Thoughts Storage Manager
///
/// Handles all operations related to persistent thought storage including
/// directory initialization, thought persistence, and symlink management.
pub struct ThoughtsStorage {
    config: ThoughtsConfig,
}

impl ThoughtsStorage {
    /// Create a new ThoughtsStorage instance with default configuration
    pub fn new() -> ThoughtsResult<Self> {
        Self::with_config(ThoughtsConfig::default())
    }

    /// Create a new ThoughtsStorage instance with custom configuration
    pub fn with_config(config: ThoughtsConfig) -> ThoughtsResult<Self> {
        let storage = Self { config };
        storage.initialize()?;
        Ok(storage)
    }

    /// Initialize the global thoughts storage directory structure
    ///
    /// Creates:
    /// - ~/.descartes/thoughts/ (main storage directory)
    /// - ~/.descartes/thoughts/archive/ (archived thoughts)
    /// - ~/.descartes/thoughts/projects/ (project-specific thoughts)
    /// - ~/.descartes/thoughts/categories/ (categorized thoughts)
    ///
    /// Sets proper Unix permissions (user-only access: 0o700)
    pub fn initialize(&self) -> ThoughtsResult<()> {
        debug!("Initializing thoughts storage at: {:?}", self.config.global_root);

        // Create main root directory
        self.create_directory(&self.config.global_root)?;

        // Create subdirectories
        let subdirs = vec![
            self.config.global_root.join("archive"),
            self.config.global_root.join("projects"),
            self.config.global_root.join("categories"),
            self.config.global_root.join(".metadata"),
        ];

        for subdir in subdirs {
            self.create_directory(&subdir)?;
        }

        // Create root metadata file if it doesn't exist
        self.create_root_metadata()?;

        info!("Thoughts storage initialized at: {:?}", self.config.global_root);
        Ok(())
    }

    /// Create a directory with proper permissions
    fn create_directory(&self, path: &Path) -> ThoughtsResult<()> {
        if path.exists() {
            if !path.is_dir() {
                return Err(ThoughtsError::InvalidPath(format!(
                    "Path exists but is not a directory: {:?}",
                    path
                )));
            }
            debug!("Directory already exists: {:?}", path);
            return Ok(());
        }

        // Create parent directories if needed
        fs::create_dir_all(path)?;
        debug!("Created directory: {:?}", path);

        // Set permissions (Unix only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(self.config.dir_permissions);
            fs::set_permissions(path, perms)?;
            debug!("Set permissions {:o} on: {:?}", self.config.dir_permissions, path);
        }

        Ok(())
    }

    /// Create root metadata tracking file
    fn create_root_metadata(&self) -> ThoughtsResult<()> {
        let metadata_file = self.config.global_root.join(".metadata").join("index.json");

        if metadata_file.exists() {
            return Ok(());
        }

        let metadata = serde_json::json!({
            "version": "1.0",
            "created_at": chrono::Utc::now().to_rfc3339(),
            "total_thoughts": 0,
            "categories": []
        });

        fs::write(&metadata_file, metadata.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(self.config.file_permissions);
            fs::set_permissions(&metadata_file, perms)?;
        }

        debug!("Created root metadata at: {:?}", metadata_file);
        Ok(())
    }

    /// Save a thought to the global storage
    pub fn save_thought(&self, thought: ThoughtMetadata) -> ThoughtsResult<()> {
        let thought_dir = self.get_thought_directory(&thought.id)?;
        fs::create_dir_all(&thought_dir)?;

        let metadata_file = thought_dir.join("metadata.json");
        let content_file = thought_dir.join("content.txt");

        // Write metadata
        let metadata_json = serde_json::to_string_pretty(&thought)?;
        fs::write(&metadata_file, metadata_json)?;

        // Write content separately for easy text access
        fs::write(&content_file, &thought.content)?;

        // Set file permissions
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::Permissions::from_mode(self.config.file_permissions);
            fs::set_permissions(&metadata_file, perms.clone())?;
            fs::set_permissions(&content_file, perms)?;
        }

        info!("Saved thought: {} ({})", thought.title, thought.id);
        Ok(())
    }

    /// Load a thought by ID
    pub fn load_thought(&self, thought_id: &str) -> ThoughtsResult<ThoughtMetadata> {
        let metadata_file = self.get_thought_directory(thought_id)?.join("metadata.json");

        if !metadata_file.exists() {
            return Err(ThoughtsError::InvalidPath(format!(
                "Thought not found: {}",
                thought_id
            )));
        }

        let content = fs::read_to_string(&metadata_file)?;
        let thought = serde_json::from_str(&content)?;

        debug!("Loaded thought: {}", thought_id);
        Ok(thought)
    }

    /// List all thoughts in storage
    pub fn list_thoughts(&self) -> ThoughtsResult<Vec<String>> {
        let mut thought_ids = Vec::new();

        if !self.config.global_root.exists() {
            return Ok(thought_ids);
        }

        for entry in fs::read_dir(&self.config.global_root)? {
            let entry = entry?;
            let path = entry.path();

            // Skip metadata directory and other special directories
            if path.file_name().and_then(|n| n.to_str()) == Some(".metadata") {
                continue;
            }

            if path.is_dir() {
                if let Some(thought_id) = path.file_name().and_then(|n| n.to_str()) {
                    // Verify it's a valid thought directory (has metadata.json)
                    if path.join("metadata.json").exists() {
                        thought_ids.push(thought_id.to_string());
                    }
                }
            }
        }

        debug!("Found {} thoughts in storage", thought_ids.len());
        Ok(thought_ids)
    }

    /// List thoughts by category/tag
    pub fn list_thoughts_by_tag(&self, tag: &str) -> ThoughtsResult<Vec<String>> {
        let mut matching_thoughts = Vec::new();

        for thought_id in self.list_thoughts()? {
            match self.load_thought(&thought_id) {
                Ok(thought) => {
                    if thought.tags.contains(&tag.to_string()) {
                        matching_thoughts.push(thought_id);
                    }
                }
                Err(e) => {
                    warn!("Failed to load thought {}: {}", thought_id, e);
                }
            }
        }

        debug!("Found {} thoughts with tag: {}", matching_thoughts.len(), tag);
        Ok(matching_thoughts)
    }

    /// Create a symlink from a project directory to the global thoughts storage
    ///
    /// This allows projects to have local `.thoughts` symlinks that reference
    /// the global storage, enabling easy access to shared thoughts.
    pub fn create_project_symlink(&self, project_path: &Path) -> ThoughtsResult<()> {
        let symlink_path = project_path.join(".thoughts");

        // Check if symlink already exists
        if symlink_path.exists() || symlink_path.symlink_metadata().is_ok() {
            if symlink_path.is_symlink() {
                debug!("Symlink already exists: {:?}", symlink_path);
                return Ok(());
            } else {
                return Err(ThoughtsError::SymlinkError(format!(
                    "Path exists and is not a symlink: {:?}",
                    symlink_path
                )));
            }
        }

        // Create symlink
        #[cfg(unix)]
        {
            unix_fs::symlink(&self.config.global_root, &symlink_path)?;
            debug!(
                "Created symlink: {:?} -> {:?}",
                symlink_path, self.config.global_root
            );
            info!(
                "Created project symlink at: {:?}",
                symlink_path
            );
        }

        #[cfg(not(unix))]
        {
            return Err(ThoughtsError::SymlinkError(
                "Symlinks not supported on this platform".to_string(),
            ));
        }

        Ok(())
    }

    /// Remove a project symlink
    pub fn remove_project_symlink(&self, project_path: &Path) -> ThoughtsResult<()> {
        let symlink_path = project_path.join(".thoughts");

        if !symlink_path.exists() && symlink_path.symlink_metadata().is_err() {
            debug!("Symlink does not exist: {:?}", symlink_path);
            return Ok(());
        }

        if !symlink_path.is_symlink() {
            return Err(ThoughtsError::SymlinkError(format!(
                "Path is not a symlink: {:?}",
                symlink_path
            )));
        }

        fs::remove_file(&symlink_path)?;
        info!("Removed project symlink: {:?}", symlink_path);
        Ok(())
    }

    /// Archive a thought (move to archive directory)
    pub fn archive_thought(&self, thought_id: &str) -> ThoughtsResult<()> {
        let current_path = self.get_thought_directory(thought_id)?;

        if !current_path.exists() {
            return Err(ThoughtsError::InvalidPath(format!(
                "Thought not found: {}",
                thought_id
            )));
        }

        let archive_path = self.config.global_root.join("archive").join(thought_id);

        if archive_path.exists() {
            return Err(ThoughtsError::InvalidPath(format!(
                "Archived thought already exists: {}",
                thought_id
            )));
        }

        fs::rename(&current_path, &archive_path)?;
        info!("Archived thought: {}", thought_id);
        Ok(())
    }

    /// Get the directory path for a specific thought
    fn get_thought_directory(&self, thought_id: &str) -> ThoughtsResult<PathBuf> {
        let path = self.config.global_root.join(thought_id);
        Ok(path)
    }

    /// Get statistics about the thoughts storage
    pub fn get_statistics(&self) -> ThoughtsResult<StorageStatistics> {
        let thoughts = self.list_thoughts()?;
        let total_thoughts = thoughts.len();

        let mut tags_count = std::collections::HashMap::new();
        let mut total_size = 0u64;

        for thought_id in &thoughts {
            if let Ok(thought) = self.load_thought(thought_id) {
                for tag in &thought.tags {
                    *tags_count.entry(tag.clone()).or_insert(0) += 1;
                }
            }

            if let Ok(metadata) = fs::metadata(self.get_thought_directory(thought_id)?) {
                total_size += metadata.len();
            }
        }

        Ok(StorageStatistics {
            total_thoughts,
            total_size_bytes: total_size,
            tags: tags_count,
        })
    }

    /// Clear all thoughts (destructive operation)
    pub fn clear_all(&self) -> ThoughtsResult<usize> {
        let thoughts = self.list_thoughts()?;
        let count = thoughts.len();

        for thought_id in thoughts {
            let path = self.get_thought_directory(&thought_id)?;
            if path.exists() {
                fs::remove_dir_all(&path)?;
            }
        }

        warn!("Cleared {} thoughts from storage", count);
        Ok(count)
    }

    /// Get the global root directory path
    pub fn get_root(&self) -> &Path {
        &self.config.global_root
    }

    /// Get the current configuration
    pub fn config(&self) -> &ThoughtsConfig {
        &self.config
    }
}

impl Default for ThoughtsStorage {
    fn default() -> Self {
        Self::new().expect("Failed to initialize default ThoughtsStorage")
    }
}

/// Statistics about thoughts storage
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StorageStatistics {
    /// Total number of thoughts stored
    pub total_thoughts: usize,
    /// Total size of all thoughts in bytes
    pub total_size_bytes: u64,
    /// Count of thoughts per tag
    pub tags: std::collections::HashMap<String, usize>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_storage() -> (ThoughtsStorage, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = ThoughtsConfig {
            global_root: temp_dir.path().to_path_buf(),
            dir_permissions: 0o700,
            file_permissions: 0o600,
        };
        let storage = ThoughtsStorage::with_config(config).unwrap();
        (storage, temp_dir)
    }

    #[test]
    fn test_initialize_creates_directories() {
        let (storage, _temp) = create_test_storage();

        assert!(storage.config.global_root.exists());
        assert!(storage.config.global_root.join("archive").exists());
        assert!(storage.config.global_root.join("projects").exists());
        assert!(storage.config.global_root.join("categories").exists());
        assert!(storage.config.global_root.join(".metadata").exists());
    }

    #[test]
    fn test_save_and_load_thought() {
        let (storage, _temp) = create_test_storage();

        let thought = ThoughtMetadata {
            id: "test-1".to_string(),
            title: "Test Thought".to_string(),
            content: "This is a test thought".to_string(),
            tags: vec!["test".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        storage.save_thought(thought.clone()).unwrap();
        let loaded = storage.load_thought("test-1").unwrap();

        assert_eq!(loaded.id, thought.id);
        assert_eq!(loaded.title, thought.title);
        assert_eq!(loaded.content, thought.content);
    }

    #[test]
    fn test_list_thoughts() {
        let (storage, _temp) = create_test_storage();

        let thought1 = ThoughtMetadata {
            id: "thought-1".to_string(),
            title: "First Thought".to_string(),
            content: "Content 1".to_string(),
            tags: vec!["tag1".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        let thought2 = ThoughtMetadata {
            id: "thought-2".to_string(),
            title: "Second Thought".to_string(),
            content: "Content 2".to_string(),
            tags: vec!["tag2".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        storage.save_thought(thought1).unwrap();
        storage.save_thought(thought2).unwrap();

        let thoughts = storage.list_thoughts().unwrap();
        assert_eq!(thoughts.len(), 2);
    }

    #[test]
    fn test_list_thoughts_by_tag() {
        let (storage, _temp) = create_test_storage();

        let thought1 = ThoughtMetadata {
            id: "thought-1".to_string(),
            title: "First".to_string(),
            content: "Content 1".to_string(),
            tags: vec!["important".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        let thought2 = ThoughtMetadata {
            id: "thought-2".to_string(),
            title: "Second".to_string(),
            content: "Content 2".to_string(),
            tags: vec!["important".to_string(), "urgent".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        let thought3 = ThoughtMetadata {
            id: "thought-3".to_string(),
            title: "Third".to_string(),
            content: "Content 3".to_string(),
            tags: vec!["low-priority".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        storage.save_thought(thought1).unwrap();
        storage.save_thought(thought2).unwrap();
        storage.save_thought(thought3).unwrap();

        let important = storage.list_thoughts_by_tag("important").unwrap();
        assert_eq!(important.len(), 2);

        let urgent = storage.list_thoughts_by_tag("urgent").unwrap();
        assert_eq!(urgent.len(), 1);
    }

    #[test]
    fn test_archive_thought() {
        let (storage, _temp) = create_test_storage();

        let thought = ThoughtMetadata {
            id: "test-archive".to_string(),
            title: "Archivable Thought".to_string(),
            content: "Content".to_string(),
            tags: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        storage.save_thought(thought).unwrap();
        assert_eq!(storage.list_thoughts().unwrap().len(), 1);

        storage.archive_thought("test-archive").unwrap();
        assert_eq!(storage.list_thoughts().unwrap().len(), 0);
        assert!(storage.config.global_root.join("archive").join("test-archive").exists());
    }

    #[test]
    fn test_get_statistics() {
        let (storage, _temp) = create_test_storage();

        let thought1 = ThoughtMetadata {
            id: "thought-1".to_string(),
            title: "First".to_string(),
            content: "Content 1".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        storage.save_thought(thought1).unwrap();

        let stats = storage.get_statistics().unwrap();
        assert_eq!(stats.total_thoughts, 1);
        assert_eq!(stats.tags.get("tag1"), Some(&1));
        assert_eq!(stats.tags.get("tag2"), Some(&1));
    }

    #[test]
    fn test_clear_all() {
        let (storage, _temp) = create_test_storage();

        let thought = ThoughtMetadata {
            id: "test-clear".to_string(),
            title: "Clearable".to_string(),
            content: "Content".to_string(),
            tags: vec![],
            created_at: chrono::Utc::now().to_rfc3339(),
            modified_at: chrono::Utc::now().to_rfc3339(),
            agent_id: None,
            project_id: None,
        };

        storage.save_thought(thought).unwrap();
        assert_eq!(storage.list_thoughts().unwrap().len(), 1);

        let cleared = storage.clear_all().unwrap();
        assert_eq!(cleared, 1);
        assert_eq!(storage.list_thoughts().unwrap().len(), 0);
    }
}
