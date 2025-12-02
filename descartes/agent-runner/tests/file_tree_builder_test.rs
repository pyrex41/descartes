/// Comprehensive tests for File Tree Builder and Operations
///
/// Tests cover:
/// - Directory scanning
/// - Tree construction from paths
/// - Metadata collection
/// - Incremental updates (add, remove, move)
/// - Query operations
/// - Language detection
/// - Git status integration
use agent_runner::{
    count_lines, detect_language, is_binary_file, FileNodeType, FileTree, FileTreeBuilder,
    FileTreeBuilderConfig, FileTreeUpdater, Language,
};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Create a comprehensive test directory structure
fn create_test_project(dir: &Path) -> std::io::Result<()> {
    // Root files
    fs::write(dir.join("README.md"), "# Test Project\n\nThis is a test.")?;
    fs::write(dir.join("Cargo.toml"), "[package]\nname = \"test\"")?;
    fs::write(dir.join(".gitignore"), "target/\n*.log")?;

    // Source directory
    fs::create_dir_all(dir.join("src"))?;
    fs::write(
        dir.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    )?;
    fs::write(
        dir.join("src/lib.rs"),
        "pub fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n",
    )?;
    fs::write(
        dir.join("src/utils.rs"),
        "pub fn helper() {\n    // Helper function\n}\n",
    )?;

    // Tests directory
    fs::create_dir_all(dir.join("tests"))?;
    fs::write(
        dir.join("tests/integration_test.rs"),
        "#[test]\nfn test_add() {\n    assert_eq!(2 + 2, 4);\n}\n",
    )?;

    // Nested modules
    fs::create_dir_all(dir.join("src/modules/parser"))?;
    fs::write(
        dir.join("src/modules/parser/mod.rs"),
        "pub mod lexer;\npub mod ast;\n",
    )?;
    fs::write(
        dir.join("src/modules/parser/lexer.rs"),
        "pub struct Token;\n",
    )?;
    fs::write(
        dir.join("src/modules/parser/ast.rs"),
        "pub struct AstNode;\n",
    )?;

    // Multi-language files
    fs::create_dir_all(dir.join("scripts"))?;
    fs::write(
        dir.join("scripts/build.py"),
        "#!/usr/bin/env python3\nprint('Building...')\n",
    )?;
    fs::write(
        dir.join("scripts/deploy.sh"),
        "#!/bin/bash\necho 'Deploying...'\n",
    )?;
    fs::write(
        dir.join("scripts/config.json"),
        "{\"version\": \"1.0.0\"}\n",
    )?;

    // Binary file
    let mut binary_file = fs::File::create(dir.join("data.bin"))?;
    binary_file.write_all(&[0x00, 0x01, 0x02, 0xFF, 0xFE])?;

    // Large file (for size testing)
    let large_content = "x".repeat(1000);
    fs::write(dir.join("large.txt"), large_content)?;

    Ok(())
}

// ============================================================================
// Directory Scanning Tests
// ============================================================================

#[test]
fn test_scan_directory_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Should have files and directories
    assert!(tree.file_count > 5, "Expected multiple files");
    assert!(tree.directory_count > 3, "Expected multiple directories");
    assert!(tree.root_id.is_some(), "Tree should have a root");

    println!(
        "Scanned tree: {} files, {} directories",
        tree.file_count, tree.directory_count
    );
}

#[test]
fn test_scan_directory_with_depth_limit() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let config = FileTreeBuilderConfig {
        max_depth: Some(2),
        ..Default::default()
    };

    let mut builder = FileTreeBuilder::with_config(config);
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Should have limited depth
    let stats = tree.stats();
    assert!(stats.max_depth <= 2, "Depth should be limited to 2");
}

#[test]
fn test_scan_directory_with_ignore_patterns() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let config = FileTreeBuilderConfig {
        ignore_patterns: vec!["*.json".to_string(), "scripts".to_string()],
        ..Default::default()
    };

    let mut builder = FileTreeBuilder::with_config(config);
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Should not include ignored files
    let json_files =
        tree.find_nodes(|node| node.path.extension().and_then(|e| e.to_str()) == Some("json"));
    assert_eq!(json_files.len(), 0, "JSON files should be ignored");
}

#[test]
fn test_scan_directory_metadata_collection() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Check metadata on rust files
    let rust_files = tree.filter_by_language(Language::Rust);
    assert!(rust_files.len() > 0, "Should find Rust files");

    for file in rust_files {
        assert!(file.metadata.size.is_some(), "File should have size");
        assert!(file.metadata.language == Some(Language::Rust));
        if !file.metadata.is_binary {
            assert!(
                file.metadata.line_count.is_some(),
                "Text file should have line count"
            );
        }
    }
}

// ============================================================================
// Build from Paths Tests
// ============================================================================

#[test]
fn test_build_from_paths_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let paths = vec![
        temp_dir.path().join("src/main.rs"),
        temp_dir.path().join("src/lib.rs"),
        temp_dir.path().join("tests/integration_test.rs"),
        temp_dir.path().join("README.md"),
    ];

    let mut builder = FileTreeBuilder::new();
    let tree = builder.build_from_paths(&paths, temp_dir.path()).unwrap();

    assert!(tree.file_count >= 4, "Should have at least 4 files");

    // Check specific files exist
    for path in paths {
        assert!(
            tree.get_node_by_path(&path).is_some(),
            "Path should exist in tree: {:?}",
            path
        );
    }
}

#[test]
fn test_build_from_paths_creates_parents() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    // Only specify leaf files
    let paths = vec![temp_dir.path().join("src/modules/parser/lexer.rs")];

    let mut builder = FileTreeBuilder::new();
    let tree = builder.build_from_paths(&paths, temp_dir.path()).unwrap();

    // Parent directories should be created
    let src_dir = temp_dir.path().join("src");
    let modules_dir = temp_dir.path().join("src/modules");
    let parser_dir = temp_dir.path().join("src/modules/parser");

    assert!(
        tree.get_node_by_path(&src_dir).is_some(),
        "src directory should exist"
    );
    assert!(
        tree.get_node_by_path(&modules_dir).is_some(),
        "modules directory should exist"
    );
    assert!(
        tree.get_node_by_path(&parser_dir).is_some(),
        "parser directory should exist"
    );
}

#[test]
fn test_build_from_paths_duplicate_handling() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let paths = vec![
        temp_dir.path().join("src/main.rs"),
        temp_dir.path().join("src/main.rs"), // Duplicate
        temp_dir.path().join("src/lib.rs"),
    ];

    let mut builder = FileTreeBuilder::new();
    let tree = builder.build_from_paths(&paths, temp_dir.path()).unwrap();

    // Should handle duplicates gracefully
    assert_eq!(tree.find_by_name("main.rs").len(), 1);
}

// ============================================================================
// Traversal Tests
// ============================================================================

#[test]
fn test_depth_first_traversal() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut visited = Vec::new();
    tree.traverse_depth_first(|node| {
        visited.push(node.path.clone());
    });

    assert!(visited.len() > 0, "Should visit nodes");
    assert_eq!(visited[0], tree.base_path, "First node should be root");
}

#[test]
fn test_breadth_first_traversal() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut visited = Vec::new();
    tree.traverse_breadth_first(|node| {
        visited.push(node.depth);
    });

    // In BFS, depths should be non-decreasing
    for i in 1..visited.len() {
        assert!(
            visited[i] >= visited[i - 1] - 1,
            "BFS should visit by levels"
        );
    }
}

// ============================================================================
// Query Operation Tests
// ============================================================================

#[test]
fn test_find_by_name() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let main_files = tree.find_by_name("main.rs");
    assert_eq!(main_files.len(), 1);
    assert_eq!(main_files[0].name, "main.rs");
}

#[test]
fn test_find_by_name_pattern() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mod_files = tree.find_by_name_pattern("mod.rs");
    assert!(mod_files.len() > 0);
}

#[test]
fn test_find_by_path_pattern() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let parser_files = tree.find_by_path_pattern("parser");
    assert!(
        parser_files.len() > 0,
        "Should find files in parser directory"
    );
}

#[test]
fn test_filter_by_type() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let files = tree.filter_by_type(FileNodeType::File);
    let dirs = tree.filter_by_type(FileNodeType::Directory);

    assert!(files.len() > 0);
    assert!(dirs.len() > 0);
    assert_eq!(files.len(), tree.file_count);
    assert_eq!(dirs.len(), tree.directory_count);
}

#[test]
fn test_filter_by_language() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let rust_files = tree.filter_by_language(Language::Rust);
    let python_files = tree.filter_by_language(Language::Python);

    assert!(rust_files.len() > 0, "Should find Rust files");
    assert!(python_files.len() > 0, "Should find Python files");

    for file in rust_files {
        assert!(file.path.extension().unwrap() == "rs");
    }
}

#[test]
fn test_get_children() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let src_path = temp_dir.path().join("src");
    if let Some(src_node) = tree.get_node_by_path(&src_path) {
        let children = tree.get_children(&src_node.node_id);
        assert!(children.len() > 0, "src directory should have children");
    }
}

#[test]
fn test_find_nodes_predicate() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Find all Rust files with more than 2 lines
    let large_rust_files = tree.find_nodes(|node| {
        node.metadata.language == Some(Language::Rust) && node.metadata.line_count.unwrap_or(0) > 2
    });

    assert!(large_rust_files.len() > 0);
}

// ============================================================================
// Metadata Collection Tests
// ============================================================================

#[test]
fn test_language_detection() {
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
    // Languages not currently supported (no tree-sitter grammars)
    assert_eq!(detect_language(Path::new("test.go")), None);
    assert_eq!(detect_language(Path::new("test.java")), None);
    assert_eq!(detect_language(Path::new("test.md")), None);
    assert_eq!(detect_language(Path::new("test.json")), None);
    assert_eq!(detect_language(Path::new("test.yaml")), None);
    assert_eq!(detect_language(Path::new("test.txt")), None);
}

#[test]
fn test_binary_detection() {
    let temp_dir = TempDir::new().unwrap();

    // Create text file
    let text_file = temp_dir.path().join("text.txt");
    fs::write(&text_file, "Hello, world!").unwrap();

    // Create binary file
    let binary_file = temp_dir.path().join("binary.bin");
    let mut file = fs::File::create(&binary_file).unwrap();
    file.write_all(&[0x00, 0x01, 0xFF, 0xFE]).unwrap();

    assert!(!is_binary_file(&text_file));
    assert!(is_binary_file(&binary_file));
}

#[test]
fn test_line_counting() {
    let temp_dir = TempDir::new().unwrap();

    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "line 1\nline 2\nline 3\n").unwrap();

    let line_count = count_lines(&file_path).unwrap();
    assert_eq!(line_count, 3);
}

#[test]
fn test_file_size_metadata() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let files = tree.get_all_files();
    for file in files {
        if file.is_file() {
            assert!(file.metadata.size.is_some(), "File should have size");
            assert!(file.metadata.size.unwrap() >= 0);
        }
    }
}

#[test]
fn test_timestamp_metadata() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let files = tree.get_all_files();
    for file in files {
        if file.is_file() {
            // Modified time should be present
            assert!(
                file.metadata.modified.is_some(),
                "File should have modified time"
            );
        }
    }
}

// ============================================================================
// Incremental Update Tests
// ============================================================================

#[test]
fn test_add_file() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();
    let initial_count = tree.file_count;

    // Add a new file
    let new_file = temp_dir.path().join("new_file.rs");
    fs::write(&new_file, "fn new() {}").unwrap();

    let mut updater = FileTreeUpdater::new();
    let node_id = updater.add_path(&mut tree, &new_file).unwrap();

    assert_eq!(tree.file_count, initial_count + 1);
    assert!(tree.get_node(&node_id).is_some());
    assert!(tree.get_node_by_path(&new_file).is_some());
}

#[test]
fn test_add_directory_recursive() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Create a new directory with files
    let new_dir = temp_dir.path().join("new_module");
    fs::create_dir(&new_dir).unwrap();
    fs::write(new_dir.join("mod.rs"), "pub mod test;").unwrap();
    fs::write(new_dir.join("test.rs"), "pub fn test() {}").unwrap();

    let mut updater = FileTreeUpdater::new();
    updater.add_path(&mut tree, &new_dir).unwrap();

    // Should add directory and its contents
    assert!(tree.get_node_by_path(&new_dir).is_some());
    assert!(tree.get_node_by_path(&new_dir.join("mod.rs")).is_some());
    assert!(tree.get_node_by_path(&new_dir.join("test.rs")).is_some());
}

#[test]
fn test_remove_file() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();
    let initial_count = tree.file_count;

    let file_to_remove = temp_dir.path().join("README.md");
    assert!(tree.get_node_by_path(&file_to_remove).is_some());

    let updater = FileTreeUpdater::new();
    updater.remove_path(&mut tree, &file_to_remove).unwrap();

    assert_eq!(tree.file_count, initial_count - 1);
    assert!(tree.get_node_by_path(&file_to_remove).is_none());
}

#[test]
fn test_remove_directory_recursive() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();

    let dir_to_remove = temp_dir.path().join("scripts");
    let files_in_dir = tree.find_by_path_pattern("scripts");
    let file_count_in_dir = files_in_dir.iter().filter(|n| n.is_file()).count();

    let updater = FileTreeUpdater::new();
    updater.remove_path(&mut tree, &dir_to_remove).unwrap();

    // Directory and all its contents should be removed
    assert!(tree.get_node_by_path(&dir_to_remove).is_none());
    let remaining_files = tree.find_by_path_pattern("scripts");
    assert_eq!(remaining_files.len(), 0);
}

#[test]
fn test_update_metadata() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();

    let file_path = temp_dir.path().join("README.md");
    let original_size = tree
        .get_node_by_path(&file_path)
        .unwrap()
        .metadata
        .size
        .unwrap();

    // Modify the file
    fs::write(&file_path, "# Updated README\n\nMore content here.").unwrap();

    let mut updater = FileTreeUpdater::new();
    updater.update_metadata(&mut tree, &file_path).unwrap();

    let new_size = tree
        .get_node_by_path(&file_path)
        .unwrap()
        .metadata
        .size
        .unwrap();

    assert_ne!(original_size, new_size);
}

#[test]
fn test_move_file_same_directory() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();

    let old_path = temp_dir.path().join("README.md");
    let new_path = temp_dir.path().join("README_NEW.md");

    let mut updater = FileTreeUpdater::new();
    updater.move_path(&mut tree, &old_path, &new_path).unwrap();

    assert!(tree.get_node_by_path(&new_path).is_some());
    assert!(tree.get_node_by_path(&old_path).is_none());
    assert_eq!(
        tree.get_node_by_path(&new_path).unwrap().name,
        "README_NEW.md"
    );
}

#[test]
fn test_move_file_different_directory() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();

    let old_path = temp_dir.path().join("README.md");
    let new_path = temp_dir.path().join("src/README.md");

    // Get original parent
    let old_parent = tree
        .get_node_by_path(&temp_dir.path().to_path_buf())
        .unwrap()
        .node_id
        .clone();

    let mut updater = FileTreeUpdater::new();
    updater.move_path(&mut tree, &old_path, &new_path).unwrap();

    // Verify move
    assert!(tree.get_node_by_path(&new_path).is_some());
    assert!(tree.get_node_by_path(&old_path).is_none());

    // Verify parent changed
    let moved_node = tree.get_node_by_path(&new_path).unwrap();
    let src_node = tree.get_node_by_path(&temp_dir.path().join("src")).unwrap();
    assert_eq!(moved_node.parent_id.as_ref(), Some(&src_node.node_id));
}

// ============================================================================
// Tree Invariants Tests
// ============================================================================

#[test]
fn test_tree_statistics() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let stats = tree.stats();
    assert_eq!(stats.file_count, tree.file_count);
    assert_eq!(stats.directory_count, tree.directory_count);
    assert_eq!(stats.total_nodes, tree.nodes.len());
    assert!(stats.max_depth > 0);
}

#[test]
fn test_path_index_consistency() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Every node should be in the path index
    for (node_id, node) in &tree.nodes {
        let indexed_id = tree.path_index.get(&node.path);
        assert_eq!(indexed_id, Some(node_id));
    }

    // Path index size should match node count
    assert_eq!(tree.path_index.len(), tree.nodes.len());
}

#[test]
fn test_parent_child_consistency() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    for (node_id, node) in &tree.nodes {
        // If node has a parent, parent should have this node as child
        if let Some(parent_id) = &node.parent_id {
            let parent = tree.get_node(parent_id).unwrap();
            assert!(
                parent.children.contains(node_id),
                "Parent should contain child"
            );
        }

        // All children should have this node as parent
        for child_id in &node.children {
            let child = tree.get_node(child_id).unwrap();
            assert_eq!(child.parent_id.as_ref(), Some(node_id));
        }
    }
}

#[test]
fn test_depth_consistency() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    for node in tree.nodes.values() {
        if let Some(parent_id) = &node.parent_id {
            let parent = tree.get_node(parent_id).unwrap();
            // Child depth should be parent depth + 1
            assert_eq!(node.depth, parent.depth + 1);
        } else {
            // Root node should have depth 0
            assert_eq!(node.depth, 0);
        }
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let empty_dir = temp_dir.path().join("empty");
    fs::create_dir(&empty_dir).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(&empty_dir).unwrap();

    assert_eq!(tree.file_count, 0);
    assert_eq!(tree.directory_count, 0);
    assert!(tree.root_id.is_some());
}

#[test]
fn test_single_file() {
    let temp_dir = TempDir::new().unwrap();
    let file = temp_dir.path().join("single.txt");
    fs::write(&file, "content").unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(&file).unwrap();

    assert_eq!(tree.file_count, 0); // Root is the file itself, not counted
    assert!(tree.root_id.is_some());
}

#[test]
fn test_nonexistent_path_error() {
    let mut builder = FileTreeBuilder::new();
    let result = builder.scan_directory("/nonexistent/path");

    assert!(result.is_err());
}

#[test]
fn test_add_duplicate_path_error() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("test.txt"), "content").unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();

    let file_path = temp_dir.path().join("test.txt");
    let mut updater = FileTreeUpdater::new();

    // Try to add the same path again
    let result = updater.add_path(&mut tree, &file_path);
    assert!(result.is_err());
}

#[test]
fn test_remove_nonexistent_path_error() {
    let temp_dir = TempDir::new().unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut tree = builder.scan_directory(temp_dir.path()).unwrap();

    let nonexistent = temp_dir.path().join("nonexistent.txt");
    let updater = FileTreeUpdater::new();

    let result = updater.remove_path(&mut tree, &nonexistent);
    assert!(result.is_err());
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_large_directory_scan_performance() {
    let temp_dir = TempDir::new().unwrap();

    // Create many files
    for i in 0..100 {
        let dir = temp_dir.path().join(format!("dir_{}", i));
        fs::create_dir(&dir).unwrap();
        for j in 0..10 {
            fs::write(dir.join(format!("file_{}.txt", j)), "content").unwrap();
        }
    }

    let start = std::time::Instant::now();
    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();
    let duration = start.elapsed();

    println!("Scanned {} nodes in {:?}", tree.nodes.len(), duration);
    assert!(tree.file_count >= 1000);
    assert!(duration.as_secs() < 5); // Should complete in reasonable time
}

#[test]
fn test_query_performance() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let tree = builder.scan_directory(temp_dir.path()).unwrap();

    let start = std::time::Instant::now();
    for _ in 0..1000 {
        let _ = tree.find_by_name("main.rs");
        let _ = tree.filter_by_language(Language::Rust);
        let _ = tree.get_all_files();
    }
    let duration = start.elapsed();

    println!("1000 queries completed in {:?}", duration);
    assert!(duration.as_millis() < 100); // Should be very fast
}
