# File Tree Builder - Quick Start Guide

## Overview

The File Tree Builder provides comprehensive file system scanning, tree construction, and manipulation capabilities for the Descartes project.

## Basic Usage

### 1. Scan a Directory

```rust
use agent_runner::FileTreeBuilder;

let mut builder = FileTreeBuilder::new();
let tree = builder.scan_directory("/path/to/project")?;

println!("Found {} files in {} directories",
    tree.file_count, tree.directory_count);
```

### 2. Query Files

```rust
// Find by name
let main_files = tree.find_by_name("main.rs");

// Find by pattern
let test_files = tree.find_by_name_pattern("test");

// Filter by language
let rust_files = tree.filter_by_language(Language::Rust);

// Filter by type
let directories = tree.filter_by_type(FileNodeType::Directory);

// Custom predicate
let large_files = tree.find_nodes(|node| {
    node.metadata.size.unwrap_or(0) > 100000
});
```

### 3. Traverse the Tree

```rust
// Depth-first
tree.traverse_depth_first(|node| {
    println!("{}: {}", node.depth, node.path.display());
});

// Breadth-first
tree.traverse_breadth_first(|node| {
    if node.is_file() {
        process_file(node);
    }
});
```

### 4. Incremental Updates

```rust
use agent_runner::FileTreeUpdater;

let mut updater = FileTreeUpdater::new();

// Add a file
updater.add_path(&mut tree, &new_file_path)?;

// Remove a file
updater.remove_path(&mut tree, &old_file_path)?;

// Update metadata
updater.update_metadata(&mut tree, &modified_file_path)?;

// Move/rename
updater.move_path(&mut tree, &old_path, &new_path)?;
```

### 5. Custom Configuration

```rust
use agent_runner::FileTreeBuilderConfig;

let config = FileTreeBuilderConfig {
    max_depth: Some(5),
    ignore_patterns: vec![
        "*.log".to_string(),
        "target".to_string(),
    ],
    max_file_size: Some(10 * 1024 * 1024), // 10 MB
    ..Default::default()
};

let mut builder = FileTreeBuilder::with_config(config);
let tree = builder.scan_directory("/project")?;
```

### 6. Build from Paths

```rust
let paths = vec![
    PathBuf::from("src/main.rs"),
    PathBuf::from("src/lib.rs"),
    PathBuf::from("tests/test.rs"),
];

let tree = builder.build_from_paths(&paths, "/project")?;
```

## Key Features

### Metadata Collection

Each file node automatically collects:
- File size
- Timestamps (created, modified)
- Programming language
- Line count (for text files)
- Binary file detection
- Git status
- Unix permissions (Unix only)

### Language Detection

Supports 20+ languages:
- Rust, Python, JavaScript, TypeScript
- Go, Java, C, C++
- Ruby, PHP, Swift, Kotlin, Scala
- Bash, SQL, HTML, CSS
- JSON, YAML, TOML, XML, Markdown

### Git Integration

Automatically tracks git status:
- Modified (M)
- Added (A)
- Deleted (D)
- Untracked (??)
- And more...

## Performance

- **O(1)** path lookups (HashMap-based)
- **O(n)** directory scanning (optimal)
- **1000+ files** scanned in <5 seconds
- **1000 queries** in <100ms

## Configuration Options

```rust
pub struct FileTreeBuilderConfig {
    pub max_depth: Option<usize>,           // Depth limit
    pub follow_symlinks: bool,              // Follow symlinks
    pub collect_metadata: bool,             // Collect metadata
    pub detect_languages: bool,             // Detect languages
    pub count_lines: bool,                  // Count lines
    pub track_git_status: bool,             // Git status
    pub ignore_patterns: Vec<String>,       // Ignore patterns
    pub max_file_size: Option<u64>,         // Max file size
}
```

Default ignore patterns:
- `.git`, `node_modules`, `target`
- `__pycache__`, `*.pyc`, `.DS_Store`

## Common Patterns

### Find All Test Files

```rust
let test_files = tree.find_nodes(|node| {
    node.is_file() && (
        node.path.to_str().unwrap().contains("test") ||
        node.name.contains("test")
    )
});
```

### Calculate Total Lines of Code

```rust
let total_lines: usize = tree
    .get_all_files()
    .iter()
    .filter_map(|f| f.metadata.line_count)
    .sum();
```

### Find Modified Files (Git)

```rust
let modified_files = tree.find_nodes(|node| {
    node.metadata.git_status.as_deref() == Some("M")
});
```

### Get Language Statistics

```rust
for lang in [Language::Rust, Language::Python] {
    let files = tree.filter_by_language(lang);
    let lines: usize = files
        .iter()
        .filter_map(|f| f.metadata.line_count)
        .sum();

    println!("{:?}: {} files, {} lines", lang, files.len(), lines);
}
```

## Error Handling

All operations return `io::Result<T>`:

```rust
match builder.scan_directory("/path") {
    Ok(tree) => println!("Success: {} files", tree.file_count),
    Err(e) => eprintln!("Error: {}", e),
}
```

## Tree Invariants

The implementation maintains these invariants:
- Every node (except root) has exactly one parent
- Parent's children list includes the child
- Path index is consistent with nodes
- File/directory counts are accurate
- Depth values are correct

## Integration

Works seamlessly with phase3:9.1 models:
- `FileTree` - Container
- `FileTreeNode` - Individual nodes
- `FileNodeType` - Type enumeration
- `FileMetadata` - Metadata structure

## Examples

See:
- `examples/file_tree_usage.rs` - Comprehensive examples
- `tests/file_tree_builder_test.rs` - 52 test cases
- `PHASE3_9.2_IMPLEMENTATION_REPORT.md` - Detailed documentation

## API Reference

### FileTreeBuilder

```rust
impl FileTreeBuilder {
    pub fn new() -> Self
    pub fn with_config(config: FileTreeBuilderConfig) -> Self
    pub fn scan_directory<P: AsRef<Path>>(&mut self, path: P) -> io::Result<FileTree>
    pub fn build_from_paths<P: AsRef<Path>>(&mut self, paths: &[PathBuf], base: P) -> io::Result<FileTree>
}
```

### FileTreeUpdater

```rust
impl FileTreeUpdater {
    pub fn new() -> Self
    pub fn add_path(&mut self, tree: &mut FileTree, path: &Path) -> io::Result<String>
    pub fn remove_path(&self, tree: &mut FileTree, path: &Path) -> io::Result<()>
    pub fn update_metadata(&mut self, tree: &mut FileTree, path: &Path) -> io::Result<()>
    pub fn move_path(&mut self, tree: &mut FileTree, old: &Path, new: &Path) -> io::Result<()>
}
```

### FileTree (Enhanced)

```rust
impl FileTree {
    // From phase3:9.1
    pub fn get_node(&self, id: &str) -> Option<&FileTreeNode>
    pub fn get_node_by_path(&self, path: &PathBuf) -> Option<&FileTreeNode>
    pub fn traverse_depth_first<F>(&self, visitor: F)
    pub fn traverse_breadth_first<F>(&self, visitor: F)
    pub fn find_nodes<F>(&self, predicate: F) -> Vec<&FileTreeNode>

    // New in phase3:9.2
    pub fn find_by_name(&self, name: &str) -> Vec<&FileTreeNode>
    pub fn find_by_name_pattern(&self, pattern: &str) -> Vec<&FileTreeNode>
    pub fn filter_by_type(&self, node_type: FileNodeType) -> Vec<&FileTreeNode>
    pub fn filter_by_language(&self, language: Language) -> Vec<&FileTreeNode>
    pub fn find_by_path_pattern(&self, pattern: &str) -> Vec<&FileTreeNode>
}
```

### Utility Functions

```rust
pub fn detect_language(path: &Path) -> Option<Language>
pub fn is_binary_file(path: &Path) -> bool
pub fn count_lines(path: &Path) -> io::Result<usize>
pub fn find_git_root(path: &Path) -> Option<PathBuf>
```

## Next Steps

Ready for **phase3:9.3 - Knowledge Graph Builder**:
- Parse code files from the tree
- Extract semantic entities
- Build knowledge graph
- Link to file tree nodes

## Documentation

- **Implementation Report**: `PHASE3_9.2_IMPLEMENTATION_REPORT.md`
- **Test Suite**: `tests/file_tree_builder_test.rs`
- **Examples**: `examples/file_tree_usage.rs`
- **Source Code**: `src/file_tree_builder.rs`
