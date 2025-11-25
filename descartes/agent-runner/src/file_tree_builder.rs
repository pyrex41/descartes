/// File Tree Builder: File System Scanning and Tree Construction
///
/// This module provides functionality for:
/// - Scanning directory structures recursively
/// - Building file trees from paths
/// - Collecting file metadata (size, timestamps, language detection)
/// - Git status integration
/// - Incremental tree updates
use crate::knowledge_graph::{FileMetadata, FileNodeType, FileTree, FileTreeNode};
use crate::types::Language;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

/// Configuration for file tree building
#[derive(Debug, Clone)]
pub struct FileTreeBuilderConfig {
    /// Maximum depth to scan (None = unlimited)
    pub max_depth: Option<usize>,

    /// Follow symbolic links
    pub follow_symlinks: bool,

    /// Collect file metadata
    pub collect_metadata: bool,

    /// Detect programming languages
    pub detect_languages: bool,

    /// Count lines in text files
    pub count_lines: bool,

    /// Track git status
    pub track_git_status: bool,

    /// Patterns to ignore (gitignore-style)
    pub ignore_patterns: Vec<String>,

    /// Maximum file size to process (in bytes)
    pub max_file_size: Option<u64>,
}

impl Default for FileTreeBuilderConfig {
    fn default() -> Self {
        Self {
            max_depth: None,
            follow_symlinks: false,
            collect_metadata: true,
            detect_languages: true,
            count_lines: true,
            track_git_status: true,
            ignore_patterns: vec![
                ".git".to_string(),
                "node_modules".to_string(),
                "target".to_string(),
                "__pycache__".to_string(),
                "*.pyc".to_string(),
                ".DS_Store".to_string(),
            ],
            max_file_size: Some(10 * 1024 * 1024), // 10 MB
        }
    }
}

/// File tree builder for scanning and constructing file trees
pub struct FileTreeBuilder {
    config: FileTreeBuilderConfig,
    git_status_cache: Option<HashMap<PathBuf, String>>,
}

impl FileTreeBuilder {
    /// Create a new file tree builder with default configuration
    pub fn new() -> Self {
        Self {
            config: FileTreeBuilderConfig::default(),
            git_status_cache: None,
        }
    }

    /// Create a builder with custom configuration
    pub fn with_config(config: FileTreeBuilderConfig) -> Self {
        Self {
            config,
            git_status_cache: None,
        }
    }

    /// Scan a directory and build a file tree
    ///
    /// This recursively walks the directory tree, creating FileTreeNode
    /// instances for each file and directory encountered.
    ///
    /// # Arguments
    /// * `path` - Root path to scan
    ///
    /// # Returns
    /// * `Result<FileTree, io::Error>` - The constructed file tree
    ///
    /// # Example
    /// ```ignore
    /// let builder = FileTreeBuilder::new();
    /// let tree = builder.scan_directory("/path/to/project")?;
    /// ```
    pub fn scan_directory<P: AsRef<Path>>(&mut self, path: P) -> io::Result<FileTree> {
        let path = path.as_ref();
        let canonical_path = fs::canonicalize(path)?;

        // Initialize git status cache if enabled
        if self.config.track_git_status {
            self.git_status_cache = Some(self.load_git_status(&canonical_path));
        }

        let mut tree = FileTree::new(canonical_path.clone());

        // Create root node
        let root_node = self.create_node(&canonical_path, None, 0)?;
        let root_id = tree.add_node(root_node);

        // Scan recursively
        if canonical_path.is_dir() {
            self.scan_directory_recursive(&canonical_path, &root_id, 0, &mut tree)?;
        }

        Ok(tree)
    }

    /// Build a file tree from a list of paths
    ///
    /// This constructs a tree structure from an explicit list of paths,
    /// creating intermediate directory nodes as needed.
    ///
    /// # Arguments
    /// * `paths` - List of file paths to include
    /// * `base_path` - Base path for the tree
    ///
    /// # Returns
    /// * `Result<FileTree, io::Error>` - The constructed file tree
    pub fn build_from_paths<P: AsRef<Path>>(
        &mut self,
        paths: &[PathBuf],
        base_path: P,
    ) -> io::Result<FileTree> {
        let base_path = base_path.as_ref();
        let canonical_base = if base_path.exists() {
            fs::canonicalize(base_path)?
        } else {
            base_path.to_path_buf()
        };

        let mut tree = FileTree::new(canonical_base.clone());
        let mut path_to_node_id: HashMap<PathBuf, String> = HashMap::new();

        // Initialize git status cache if enabled
        if self.config.track_git_status {
            self.git_status_cache = Some(self.load_git_status(&canonical_base));
        }

        // Sort paths to ensure parents are created before children
        let mut sorted_paths: Vec<PathBuf> = paths.to_vec();
        sorted_paths.sort();
        sorted_paths.dedup();

        for path in sorted_paths {
            // Skip if already processed
            if path_to_node_id.contains_key(&path) {
                continue;
            }

            // Ensure all parent directories exist in the tree
            self.ensure_parent_nodes(&path, &mut tree, &mut path_to_node_id)?;

            // Create node for this path
            let parent_path = path.parent();
            let parent_id = parent_path.and_then(|p| path_to_node_id.get(p).cloned());

            let depth = path.components().count();
            let node = self.create_node(&path, parent_id.clone(), depth)?;
            let node_id = tree.add_node(node);
            path_to_node_id.insert(path.clone(), node_id.clone());

            // Link to parent
            if let Some(parent_id) = parent_id {
                if let Some(parent) = tree.get_node_mut(&parent_id) {
                    parent.add_child(node_id);
                }
            }
        }

        Ok(tree)
    }

    /// Recursively scan a directory
    fn scan_directory_recursive(
        &self,
        path: &Path,
        parent_id: &str,
        depth: usize,
        tree: &mut FileTree,
    ) -> io::Result<()> {
        // Check depth limit
        if let Some(max_depth) = self.config.max_depth {
            if depth >= max_depth {
                return Ok(());
            }
        }

        // Read directory entries
        let entries = fs::read_dir(path)?;

        for entry in entries {
            let entry = entry?;
            let entry_path = entry.path();

            // Check ignore patterns
            if self.should_ignore(&entry_path) {
                continue;
            }

            // Create node for this entry
            let node = self.create_node(&entry_path, Some(parent_id.to_string()), depth + 1)?;
            let is_dir = node.is_directory();
            let node_id = tree.add_node(node);

            // Link to parent
            if let Some(parent) = tree.get_node_mut(parent_id) {
                parent.add_child(node_id.clone());
            }

            // Recurse into directories
            if is_dir {
                self.scan_directory_recursive(&entry_path, &node_id, depth + 1, tree)?;
            }
        }

        Ok(())
    }

    /// Create a FileTreeNode from a path
    fn create_node(
        &self,
        path: &Path,
        parent_id: Option<String>,
        depth: usize,
    ) -> io::Result<FileTreeNode> {
        let metadata = fs::symlink_metadata(path)?;

        let node_type = if metadata.is_dir() {
            FileNodeType::Directory
        } else if metadata.is_symlink() {
            FileNodeType::Symlink
        } else {
            FileNodeType::File
        };

        let mut node = FileTreeNode::new(path.to_path_buf(), node_type, parent_id, depth);

        // Collect metadata if enabled
        if self.config.collect_metadata {
            node.metadata = self.collect_metadata(path, &metadata)?;
        }

        Ok(node)
    }

    /// Collect file metadata
    fn collect_metadata(
        &self,
        path: &Path,
        fs_metadata: &fs::Metadata,
    ) -> io::Result<FileMetadata> {
        let mut metadata = FileMetadata::default();

        // Basic metadata
        metadata.size = Some(fs_metadata.len());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions = Some(fs_metadata.permissions().mode());
        }

        // Timestamps
        if let Ok(modified) = fs_metadata.modified() {
            if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                metadata.modified = Some(duration.as_secs() as i64);
            }
        }

        if let Ok(created) = fs_metadata.created() {
            if let Ok(duration) = created.duration_since(UNIX_EPOCH) {
                metadata.created = Some(duration.as_secs() as i64);
            }
        }

        // Detect language from extension
        if self.config.detect_languages && fs_metadata.is_file() {
            metadata.language = detect_language(path);
        }

        // Detect if binary
        if fs_metadata.is_file() {
            metadata.is_binary = is_binary_file(path);
        }

        // Count lines for text files
        if self.config.count_lines && fs_metadata.is_file() && !metadata.is_binary {
            if let Some(max_size) = self.config.max_file_size {
                if fs_metadata.len() <= max_size {
                    metadata.line_count = count_lines(path).ok();
                }
            }
        }

        // Git status
        if self.config.track_git_status {
            if let Some(git_cache) = &self.git_status_cache {
                metadata.git_status = git_cache.get(path).cloned();
            }
        }

        Ok(metadata)
    }

    /// Ensure all parent directory nodes exist in the tree
    fn ensure_parent_nodes(
        &self,
        path: &Path,
        tree: &mut FileTree,
        path_to_node_id: &mut HashMap<PathBuf, String>,
    ) -> io::Result<()> {
        let mut ancestors: Vec<PathBuf> = Vec::new();
        let mut current = path.parent();

        // Collect all ancestors that don't exist yet
        while let Some(parent) = current {
            if !path_to_node_id.contains_key(parent) {
                ancestors.push(parent.to_path_buf());
            }
            current = parent.parent();
        }

        // Create nodes from root to leaf
        ancestors.reverse();
        for ancestor in ancestors {
            let parent_path = ancestor.parent();
            let parent_id = parent_path.and_then(|p| path_to_node_id.get(p).cloned());

            let depth = ancestor.components().count();
            let node = FileTreeNode::new(
                ancestor.clone(),
                FileNodeType::Directory,
                parent_id.clone(),
                depth,
            );
            let node_id = tree.add_node(node);
            path_to_node_id.insert(ancestor, node_id.clone());

            // Link to parent
            if let Some(parent_id) = parent_id {
                if let Some(parent) = tree.get_node_mut(&parent_id) {
                    parent.add_child(node_id);
                }
            }
        }

        Ok(())
    }

    /// Check if a path should be ignored
    fn should_ignore(&self, path: &Path) -> bool {
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        for pattern in &self.config.ignore_patterns {
            if pattern.starts_with("*.") {
                // Extension pattern
                let ext = &pattern[2..];
                if path.extension().and_then(|e| e.to_str()) == Some(ext) {
                    return true;
                }
            } else if file_name == pattern {
                return true;
            }
        }

        false
    }

    /// Load git status for all files in a repository
    fn load_git_status(&self, path: &Path) -> HashMap<PathBuf, String> {
        let mut status_map = HashMap::new();

        // Try to find git repository root
        let git_root = find_git_root(path);
        if git_root.is_none() {
            return status_map;
        }

        // Run git status --porcelain
        let output = std::process::Command::new("git")
            .arg("status")
            .arg("--porcelain")
            .current_dir(git_root.unwrap())
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                let status_text = String::from_utf8_lossy(&output.stdout);
                for line in status_text.lines() {
                    if line.len() < 4 {
                        continue;
                    }

                    let status = line[..2].trim().to_string();
                    let file_path = line[3..].trim();
                    let full_path = path.join(file_path);

                    status_map.insert(full_path, status);
                }
            }
        }

        status_map
    }
}

impl Default for FileTreeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Detect programming language from file extension
pub fn detect_language(path: &Path) -> Option<Language> {
    let extension = path.extension()?.to_str()?;

    match extension.to_lowercase().as_str() {
        "rs" => Some(Language::Rust),
        "py" | "pyw" | "pyi" => Some(Language::Python),
        "js" | "jsx" | "mjs" | "cjs" => Some(Language::JavaScript),
        "ts" | "tsx" | "mts" | "cts" => Some(Language::TypeScript),
        "go" => Some(Language::Go),
        "java" => Some(Language::Java),
        "c" | "h" => Some(Language::C),
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => Some(Language::Cpp),
        "rb" => Some(Language::Ruby),
        "php" => Some(Language::Php),
        "swift" => Some(Language::Swift),
        "kt" | "kts" => Some(Language::Kotlin),
        "scala" | "sc" => Some(Language::Scala),
        "sh" | "bash" => Some(Language::Bash),
        "sql" => Some(Language::Sql),
        "html" | "htm" => Some(Language::Html),
        "css" | "scss" | "sass" | "less" => Some(Language::Css),
        "json" => Some(Language::Json),
        "xml" => Some(Language::Xml),
        "yaml" | "yml" => Some(Language::Yaml),
        "toml" => Some(Language::Toml),
        "md" | "markdown" => Some(Language::Markdown),
        _ => None,
    }
}

/// Check if a file is binary
pub fn is_binary_file(path: &Path) -> bool {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return false,
    };

    let mut reader = BufReader::new(file);
    let mut buffer = vec![0u8; 8192];

    match reader.read(&mut buffer) {
        Ok(n) if n > 0 => {
            // Check for null bytes (common in binary files)
            buffer[..n].contains(&0)
        }
        _ => false,
    }
}

/// Count lines in a text file
pub fn count_lines(path: &Path) -> io::Result<usize> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().count())
}

/// Find the git repository root
pub fn find_git_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.to_path_buf();

    loop {
        let git_dir = current.join(".git");
        if git_dir.exists() {
            return Some(current);
        }

        if !current.pop() {
            break;
        }
    }

    None
}

/// Incremental file tree updater
pub struct FileTreeUpdater {
    builder: FileTreeBuilder,
}

impl FileTreeUpdater {
    /// Create a new updater
    pub fn new() -> Self {
        Self {
            builder: FileTreeBuilder::new(),
        }
    }

    /// Create an updater with custom config
    pub fn with_config(config: FileTreeBuilderConfig) -> Self {
        Self {
            builder: FileTreeBuilder::with_config(config),
        }
    }

    /// Add a file or directory to the tree
    pub fn add_path(&mut self, tree: &mut FileTree, path: &Path) -> io::Result<String> {
        // Check if path already exists
        if tree.get_node_by_path(&path.to_path_buf()).is_some() {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "Path already exists in tree",
            ));
        }

        // Find parent
        let parent_path = path.parent();
        let parent_id = parent_path.and_then(|p| {
            tree.get_node_by_path(&p.to_path_buf())
                .map(|n| n.node_id.clone())
        });

        // Calculate depth
        let depth = path.components().count();

        // Create node
        let node = self.builder.create_node(path, parent_id.clone(), depth)?;
        let node_id = tree.add_node(node);

        // Link to parent
        if let Some(parent_id) = parent_id {
            if let Some(parent) = tree.get_node_mut(&parent_id) {
                parent.add_child(node_id.clone());
            }
        }

        // If it's a directory, scan it
        if path.is_dir() {
            self.builder
                .scan_directory_recursive(path, &node_id, depth, tree)?;
        }

        Ok(node_id)
    }

    /// Remove a file or directory from the tree
    pub fn remove_path(&self, tree: &mut FileTree, path: &Path) -> io::Result<()> {
        let node_id = tree
            .get_node_by_path(&path.to_path_buf())
            .map(|n| n.node_id.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Path not found"))?;

        self.remove_node(tree, &node_id);
        Ok(())
    }

    /// Remove a node and all its descendants
    fn remove_node(&self, tree: &mut FileTree, node_id: &str) {
        // Get node and collect children
        let children = if let Some(node) = tree.get_node(node_id) {
            node.children.clone()
        } else {
            return;
        };

        // Recursively remove children
        for child_id in children {
            self.remove_node(tree, &child_id);
        }

        // Remove from parent's children list
        if let Some(node) = tree.get_node(node_id) {
            if let Some(parent_id) = &node.parent_id {
                if let Some(parent) = tree.get_node_mut(parent_id) {
                    parent.children.retain(|id| id != node_id);
                }
            }

            // Update counts
            match node.node_type {
                FileNodeType::File => tree.file_count = tree.file_count.saturating_sub(1),
                FileNodeType::Directory => {
                    tree.directory_count = tree.directory_count.saturating_sub(1)
                }
                _ => {}
            }

            // Remove from path index
            tree.path_index.remove(&node.path);
        }

        // Remove from nodes
        tree.nodes.remove(node_id);
    }

    /// Update metadata for a node
    pub fn update_metadata(&mut self, tree: &mut FileTree, path: &Path) -> io::Result<()> {
        let node_id = tree
            .get_node_by_path(&path.to_path_buf())
            .map(|n| n.node_id.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Path not found"))?;

        let fs_metadata = fs::symlink_metadata(path)?;
        let new_metadata = self.builder.collect_metadata(path, &fs_metadata)?;

        if let Some(node) = tree.get_node_mut(&node_id) {
            node.metadata = new_metadata;
        }

        Ok(())
    }

    /// Handle file move/rename
    pub fn move_path(
        &mut self,
        tree: &mut FileTree,
        old_path: &Path,
        new_path: &Path,
    ) -> io::Result<()> {
        // Get the node
        let node_id = tree
            .get_node_by_path(&old_path.to_path_buf())
            .map(|n| n.node_id.clone())
            .ok_or_else(|| io::Error::new(io::ErrorKind::NotFound, "Path not found"))?;

        // Update the node's path and name
        if let Some(node) = tree.get_node_mut(&node_id) {
            // Remove old path from index
            tree.path_index.remove(&node.path);

            // Update node
            node.path = new_path.to_path_buf();
            node.name = new_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_string();

            // Add new path to index
            tree.path_index
                .insert(new_path.to_path_buf(), node_id.clone());

            // Update metadata
            if let Ok(fs_metadata) = fs::symlink_metadata(new_path) {
                node.metadata = self.builder.collect_metadata(new_path, &fs_metadata)?;
            }
        }

        // Update parent if changed
        let old_parent = old_path.parent();
        let new_parent = new_path.parent();

        if old_parent != new_parent {
            // Remove from old parent
            if let Some(old_parent_path) = old_parent {
                if let Some(old_parent_node) = tree.get_node_by_path(&old_parent_path.to_path_buf())
                {
                    let old_parent_id = old_parent_node.node_id.clone();
                    if let Some(parent) = tree.get_node_mut(&old_parent_id) {
                        parent.children.retain(|id| id != &node_id);
                    }
                }
            }

            // Add to new parent
            if let Some(new_parent_path) = new_parent {
                if let Some(new_parent_node) = tree.get_node_by_path(&new_parent_path.to_path_buf())
                {
                    let new_parent_id = new_parent_node.node_id.clone();
                    if let Some(parent) = tree.get_node_mut(&new_parent_id) {
                        parent.add_child(node_id.clone());
                    }

                    // Update parent_id in the moved node
                    if let Some(node) = tree.get_node_mut(&node_id) {
                        node.parent_id = Some(new_parent_id);
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for FileTreeUpdater {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_structure(dir: &Path) -> io::Result<()> {
        fs::create_dir_all(dir.join("src"))?;
        fs::create_dir_all(dir.join("tests"))?;
        fs::write(dir.join("src/main.rs"), "fn main() {}")?;
        fs::write(dir.join("src/lib.rs"), "pub fn test() {}")?;
        fs::write(dir.join("tests/test.rs"), "#[test] fn test() {}")?;
        fs::write(dir.join("README.md"), "# Test Project")?;
        Ok(())
    }

    #[test]
    fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).unwrap();

        let mut builder = FileTreeBuilder::new();
        let tree = builder.scan_directory(temp_dir.path()).unwrap();

        assert!(tree.file_count > 0);
        assert!(tree.directory_count > 0);
    }

    #[test]
    fn test_detect_language() {
        assert_eq!(detect_language(Path::new("test.rs")), Some(Language::Rust));
        assert_eq!(
            detect_language(Path::new("test.py")),
            Some(Language::Python)
        );
        assert_eq!(
            detect_language(Path::new("test.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            detect_language(Path::new("test.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(detect_language(Path::new("test.txt")), None);
    }

    #[test]
    fn test_build_from_paths() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).unwrap();

        let paths = vec![
            temp_dir.path().join("src/main.rs"),
            temp_dir.path().join("src/lib.rs"),
            temp_dir.path().join("README.md"),
        ];

        let mut builder = FileTreeBuilder::new();
        let tree = builder.build_from_paths(&paths, temp_dir.path()).unwrap();

        assert!(tree.file_count >= 3);
    }

    #[test]
    fn test_incremental_add() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).unwrap();

        let mut builder = FileTreeBuilder::new();
        let mut tree = builder.scan_directory(temp_dir.path()).unwrap();
        let initial_count = tree.file_count;

        // Add a new file
        let new_file = temp_dir.path().join("new_file.rs");
        fs::write(&new_file, "fn new() {}").unwrap();

        let mut updater = FileTreeUpdater::new();
        updater.add_path(&mut tree, &new_file).unwrap();

        assert_eq!(tree.file_count, initial_count + 1);
    }

    #[test]
    fn test_incremental_remove() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).unwrap();

        let mut builder = FileTreeBuilder::new();
        let mut tree = builder.scan_directory(temp_dir.path()).unwrap();
        let initial_count = tree.file_count;

        // Remove a file
        let file_to_remove = temp_dir.path().join("README.md");
        let updater = FileTreeUpdater::new();
        updater.remove_path(&mut tree, &file_to_remove).unwrap();

        assert_eq!(tree.file_count, initial_count - 1);
    }

    #[test]
    fn test_move_path() {
        let temp_dir = TempDir::new().unwrap();
        create_test_structure(temp_dir.path()).unwrap();

        let mut builder = FileTreeBuilder::new();
        let mut tree = builder.scan_directory(temp_dir.path()).unwrap();

        let old_path = temp_dir.path().join("README.md");
        let new_path = temp_dir.path().join("README_NEW.md");

        let mut updater = FileTreeUpdater::new();
        updater.move_path(&mut tree, &old_path, &new_path).unwrap();

        assert!(tree.get_node_by_path(&new_path.to_path_buf()).is_some());
        assert!(tree.get_node_by_path(&old_path.to_path_buf()).is_none());
    }
}
