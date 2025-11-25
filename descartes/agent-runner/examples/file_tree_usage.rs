/// File Tree Builder Usage Examples
///
/// This example demonstrates how to use the file tree builder
/// to scan directories, build trees, and perform operations.
use agent_runner::{
    FileNodeType, FileTree, FileTreeBuilder, FileTreeBuilderConfig, FileTreeUpdater, Language,
};
use std::path::PathBuf;

fn main() -> std::io::Result<()> {
    println!("=== File Tree Builder Examples ===\n");

    // Example 1: Basic Directory Scanning
    example_1_basic_scan()?;

    // Example 2: Custom Configuration
    example_2_custom_config()?;

    // Example 3: Query Operations
    example_3_queries()?;

    // Example 4: Incremental Updates
    example_4_incremental_updates()?;

    // Example 5: Build from Paths
    example_5_build_from_paths()?;

    Ok(())
}

/// Example 1: Basic directory scanning
fn example_1_basic_scan() -> std::io::Result<()> {
    println!("--- Example 1: Basic Directory Scanning ---");

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(".")?;

    println!("Scanned directory tree:");
    println!("  Files: {}", tree.file_count);
    println!("  Directories: {}", tree.directory_count);
    println!("  Total nodes: {}", tree.nodes.len());

    let stats = tree.stats();
    println!("  Max depth: {}", stats.max_depth);
    println!();

    Ok(())
}

/// Example 2: Custom configuration
fn example_2_custom_config() -> std::io::Result<()> {
    println!("--- Example 2: Custom Configuration ---");

    let config = FileTreeBuilderConfig {
        max_depth: Some(3),
        follow_symlinks: false,
        collect_metadata: true,
        detect_languages: true,
        count_lines: true,
        track_git_status: true,
        ignore_patterns: vec![
            "*.log".to_string(),
            "*.tmp".to_string(),
            "target".to_string(),
        ],
        max_file_size: Some(1024 * 1024), // 1 MB
    };

    let mut builder = FileTreeBuilder::with_config(config);
    let tree = builder.scan_directory(".")?;

    println!("Scanned with custom config:");
    println!("  Files found: {}", tree.file_count);
    println!();

    Ok(())
}

/// Example 3: Query operations
fn example_3_queries() -> std::io::Result<()> {
    println!("--- Example 3: Query Operations ---");

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(".")?;

    // Find by name
    let cargo_files = tree.find_by_name("Cargo.toml");
    println!("Found {} Cargo.toml files", cargo_files.len());

    // Find by pattern
    let test_files = tree.find_by_name_pattern("test");
    println!("Found {} files matching 'test'", test_files.len());

    // Filter by type
    let directories = tree.filter_by_type(FileNodeType::Directory);
    println!("Found {} directories", directories.len());

    // Filter by language
    let rust_files = tree.filter_by_language(Language::Rust);
    println!("Found {} Rust files", rust_files.len());

    if !rust_files.is_empty() {
        let total_lines: usize = rust_files
            .iter()
            .filter_map(|f| f.metadata.line_count)
            .sum();
        println!("  Total Rust LOC: {}", total_lines);
    }

    // Custom predicate
    let large_files =
        tree.find_nodes(|node| node.is_file() && node.metadata.size.unwrap_or(0) > 10000);
    println!("Found {} files larger than 10KB", large_files.len());
    println!();

    Ok(())
}

/// Example 4: Incremental updates
fn example_4_incremental_updates() -> std::io::Result<()> {
    println!("--- Example 4: Incremental Updates ---");

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(".")?;

    let initial_count = tree.file_count;
    println!("Initial file count: {}", initial_count);

    // Note: These are demonstrations - would need actual files to work
    let mut updater = FileTreeUpdater::new();

    // Add a file (if it exists)
    // updater.add_path(&mut tree, &PathBuf::from("new_file.rs"))?;
    // println!("After add: {} files", tree.file_count);

    // Update metadata
    if let Some(node) = tree.nodes.values().next() {
        if node.is_file() {
            println!("Can update metadata for: {}", node.path.display());
        }
    }

    // Remove a file
    // updater.remove_path(&mut tree, &PathBuf::from("old_file.rs"))?;
    // println!("After remove: {} files", tree.file_count);

    // Move/rename a file
    // updater.move_path(&mut tree, &old_path, &new_path)?;
    // println!("File moved successfully");

    println!("Incremental updates demonstrated");
    println!();

    Ok(())
}

/// Example 5: Build from explicit paths
fn example_5_build_from_paths() -> std::io::Result<()> {
    println!("--- Example 5: Build from Paths ---");

    // Collect specific paths (e.g., from git diff)
    let paths = vec![
        PathBuf::from("src/lib.rs"),
        PathBuf::from("src/file_tree_builder.rs"),
        PathBuf::from("Cargo.toml"),
    ];

    let mut builder = FileTreeBuilder::new();

    // This would work if we're in the right directory
    // let tree = builder.build_from_paths(&paths, ".")?;
    // println!("Built tree from {} paths", paths.len());
    // println!("  Total nodes: {}", tree.nodes.len());

    println!("Build from paths demonstrated");
    println!("  Paths specified: {}", paths.len());
    println!();

    Ok(())
}

// Additional helper examples

/// Traverse the tree and print structure
#[allow(dead_code)]
fn print_tree_structure(tree: &FileTree) {
    println!("Tree structure:");
    tree.traverse_depth_first(|node| {
        let indent = "  ".repeat(node.depth);
        let type_char = if node.is_directory() { "📁" } else { "📄" };
        println!("{}{} {}", indent, type_char, node.name);
    });
}

/// Find all files with a specific extension
#[allow(dead_code)]
fn find_by_extension(tree: &FileTree, ext: &str) -> Vec<PathBuf> {
    tree.find_nodes(|node| node.is_file() && node.extension().as_deref() == Some(ext))
        .iter()
        .map(|n| n.path.clone())
        .collect()
}

/// Get statistics by language
#[allow(dead_code)]
fn language_statistics(tree: &FileTree) {
    let languages = vec![
        Language::Rust,
        Language::Python,
        Language::JavaScript,
        Language::TypeScript,
    ];

    println!("Language statistics:");
    for lang in languages {
        let files = tree.filter_by_language(lang);
        if !files.is_empty() {
            let lines: usize = files.iter().filter_map(|f| f.metadata.line_count).sum();
            println!("  {:?}: {} files, {} lines", lang, files.len(), lines);
        }
    }
}
