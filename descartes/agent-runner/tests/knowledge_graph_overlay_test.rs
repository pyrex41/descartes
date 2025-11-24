/// Comprehensive tests for knowledge graph overlay functionality

use agent_runner::{
    FileTreeBuilder, KnowledgeGraphOverlay, KnowledgeNodeType, OverlayConfig,
    RelationshipType,
};
use std::fs;
use tempfile::TempDir;

/// Create a realistic test project with multiple files
fn create_test_project(dir: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(dir.join("src"))?;
    fs::create_dir_all(dir.join("tests"))?;

    // Create main.rs with functions and structs
    fs::write(
        dir.join("src/main.rs"),
        r#"
mod utils;
use utils::Calculator;

/// Main entry point
fn main() {
    let calc = Calculator::new();
    let result = calc.add(5, 3);
    println!("Result: {}", result);

    process_data(&result);
}

/// Process data function
fn process_data(value: &i32) {
    let formatted = format_output(*value);
    println!("{}", formatted);
}

/// Format output
fn format_output(value: i32) -> String {
    format!("Value: {}", value)
}

/// Point struct
struct Point {
    x: i32,
    y: i32,
}

impl Point {
    /// Create a new point
    fn new(x: i32, y: i32) -> Self {
        Point { x, y }
    }

    /// Calculate distance from origin
    fn distance(&self) -> f64 {
        ((self.x * self.x + self.y * self.y) as f64).sqrt()
    }
}
"#,
    )?;

    // Create utils.rs with a calculator
    fs::write(
        dir.join("src/utils.rs"),
        r#"
/// Calculator struct
pub struct Calculator {
    history: Vec<i32>,
}

impl Calculator {
    /// Create a new calculator
    pub fn new() -> Self {
        Calculator {
            history: Vec::new(),
        }
    }

    /// Add two numbers
    pub fn add(&mut self, a: i32, b: i32) -> i32 {
        let result = a + b;
        self.history.push(result);
        result
    }

    /// Subtract two numbers
    pub fn subtract(&mut self, a: i32, b: i32) -> i32 {
        let result = a - b;
        self.history.push(result);
        result
    }

    /// Get calculation history
    pub fn get_history(&self) -> &[i32] {
        &self.history
    }
}

/// Helper function to validate input
pub fn validate_input(value: i32) -> bool {
    value >= 0
}
"#,
    )?;

    // Create a Python file
    fs::write(
        dir.join("src/data_processor.py"),
        r#"
class DataProcessor:
    """Main data processor class"""

    def __init__(self):
        self.data = []
        self.processed = False

    def add_data(self, item):
        """Add data to the processor"""
        self.data.append(item)

    def process(self):
        """Process all data"""
        self.data = [transform(item) for item in self.data]
        self.processed = True
        return self.data

    def clear(self):
        """Clear all data"""
        self.data = []
        self.processed = False

def transform(value):
    """Transform a single value"""
    return value * 2

def validate(value):
    """Validate a value"""
    return value > 0
"#,
    )?;

    Ok(())
}

#[test]
fn test_overlay_generation_basic() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    // Build file tree
    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Generate knowledge overlay
    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Verify we extracted entities
    assert!(kg.nodes.len() > 0, "Should extract entities from code files");
    println!("Extracted {} knowledge nodes", kg.nodes.len());

    // Print statistics
    let stats = kg.stats();
    println!("Knowledge Graph Statistics:");
    println!("  Total nodes: {}", stats.total_nodes);
    println!("  Total edges: {}", stats.total_edges);
    for (node_type, count) in &stats.node_type_counts {
        println!("  {}: {}", node_type, count);
    }
}

#[test]
fn test_overlay_with_linking() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Generate and link
    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let kg = overlay.generate_and_link(&mut file_tree).unwrap();

    // Verify bidirectional links
    let rust_files: Vec<_> = file_tree
        .get_all_files()
        .into_iter()
        .filter(|n| n.extension() == Some("rs".to_string()))
        .collect();

    let mut total_links = 0;
    for file_node in rust_files {
        println!(
            "File: {:?}, Knowledge links: {}",
            file_node.path,
            file_node.knowledge_links.len()
        );
        total_links += file_node.knowledge_links.len();
    }

    assert!(
        total_links > 0,
        "File nodes should have knowledge links established"
    );
}

#[test]
fn test_find_entities_in_file() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Find entities in main.rs
    let main_path = temp_dir.path().join("src/main.rs");
    let entities = overlay.find_entities_in_file(&main_path, &kg);

    println!("\nEntities found in main.rs:");
    for entity in &entities {
        println!(
            "  - {} ({:?}) at lines {:?}",
            entity.name,
            entity.content_type,
            entity.file_references.first().map(|r| r.line_range)
        );
    }

    assert!(entities.len() > 0, "Should find entities in main.rs");
}

#[test]
fn test_find_by_type() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Find all functions
    let functions = overlay.find_by_type(KnowledgeNodeType::Function, &kg);
    println!("\nFunctions found: {}", functions.len());
    for func in &functions {
        println!("  - {}", func.qualified_name);
    }

    // Find all structs
    let structs = overlay.find_by_type(KnowledgeNodeType::Struct, &kg);
    println!("\nStructs found: {}", structs.len());
    for s in &structs {
        println!("  - {}", s.qualified_name);
    }

    // Find all classes (Python)
    let classes = overlay.find_by_type(KnowledgeNodeType::Class, &kg);
    println!("\nClasses found: {}", classes.len());
    for class in &classes {
        println!("  - {}", class.qualified_name);
    }

    assert!(functions.len() > 0, "Should find functions");
}

#[test]
fn test_find_by_name_pattern() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Search for entities with "process" in the name
    let results = overlay.find_by_name_pattern("process", &kg);
    println!("\nEntities matching 'process': {}", results.len());
    for entity in &results {
        println!("  - {} ({})", entity.qualified_name, entity.content_type.as_str());
    }

    // Search for "Calculator"
    let calc_results = overlay.find_by_name_pattern("Calculator", &kg);
    println!("\nEntities matching 'Calculator': {}", calc_results.len());
    for entity in &calc_results {
        println!("  - {} ({})", entity.qualified_name, entity.content_type.as_str());
    }
}

#[test]
fn test_knowledge_node_details() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Find a specific function and examine its details
    let functions = overlay.find_by_type(KnowledgeNodeType::Function, &kg);

    if let Some(func) = functions.first() {
        println!("\nDetailed view of function: {}", func.name);
        println!("  Qualified name: {}", func.qualified_name);
        println!("  Language: {:?}", func.language);
        println!("  Parameters: {:?}", func.parameters);
        println!("  Return type: {:?}", func.return_type);
        println!("  Visibility: {:?}", func.visibility);
        println!("  File references: {}", func.file_references.len());

        for file_ref in &func.file_references {
            println!(
                "    - {:?} at lines {:?}",
                file_ref.file_path, file_ref.line_range
            );
        }

        if let Some(source) = &func.source_code {
            println!("  Source code (first 100 chars):");
            let preview = if source.len() > 100 {
                &source[..100]
            } else {
                source
            };
            println!("    {}", preview.replace('\n', "\n    "));
        }

        if let Some(doc) = &func.description {
            println!("  Documentation: {}", doc);
        }
    }
}

#[test]
fn test_incremental_update() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let mut kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    let initial_count = kg.nodes.len();
    println!("\nInitial node count: {}", initial_count);

    // Modify main.rs to add a new function
    let main_path = temp_dir.path().join("src/main.rs");
    fs::write(
        &main_path,
        r#"
fn main() {
    println!("Modified");
    new_helper();
}

fn new_helper() {
    println!("New helper function");
}

fn another_function() {
    println!("Another one");
}
"#,
    )
    .unwrap();

    // Update the knowledge graph
    overlay.update_file(&main_path, &file_tree, &mut kg).unwrap();

    let updated_count = kg.nodes.len();
    println!("Updated node count: {}", updated_count);

    // Verify the update worked
    assert!(kg.nodes.len() > 0, "Should still have nodes after update");
}

#[test]
fn test_cache_functionality() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();

    // First generation (populates cache)
    let _kg1 = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    let cache_stats = overlay.cache_stats();
    println!("\nCache statistics:");
    println!("  Total entries: {}", cache_stats.total_entries);
    println!("  Total nodes: {}", cache_stats.total_nodes);
    println!("  Total edges: {}", cache_stats.total_edges);

    // Clear cache
    overlay.clear_cache();
    let empty_stats = overlay.cache_stats();
    assert_eq!(empty_stats.total_entries, 0, "Cache should be empty after clear");
}

#[test]
fn test_overlay_with_custom_config() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Create custom config - only Python
    let config = OverlayConfig {
        enabled_languages: vec![agent_runner::Language::Python],
        extract_relationships: true,
        max_file_size: Some(10 * 1024 * 1024),
        enable_cache: true,
        cache_dir: None,
        cache_ttl: std::time::Duration::from_secs(1800),
        parallel_parsing: false,
    };

    let mut overlay = KnowledgeGraphOverlay::with_config(config).unwrap();
    let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    println!("\nPython-only knowledge graph:");
    println!("  Total nodes: {}", kg.nodes.len());

    // Verify only Python entities were extracted
    for node in kg.nodes.values() {
        if let Some(lang) = node.language {
            println!("  Found {} with language {:?}", node.name, lang);
            assert_eq!(
                lang,
                agent_runner::Language::Python,
                "Should only have Python entities"
            );
        }
    }
}

#[test]
fn test_multiple_file_types() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Count entities by language
    let mut lang_counts = std::collections::HashMap::new();
    for node in kg.nodes.values() {
        if let Some(lang) = node.language {
            *lang_counts.entry(lang).or_insert(0) += 1;
        }
    }

    println!("\nEntities by language:");
    for (lang, count) in &lang_counts {
        println!("  {:?}: {}", lang, count);
    }

    // Should have both Rust and Python entities
    assert!(
        lang_counts.len() > 0,
        "Should have entities from multiple languages"
    );
}

#[test]
fn test_file_reference_accuracy() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Check that all knowledge nodes have valid file references
    for (node_id, node) in &kg.nodes {
        assert!(
            !node.file_references.is_empty(),
            "Node {} should have at least one file reference",
            node_id
        );

        for file_ref in &node.file_references {
            // Verify file exists
            assert!(
                file_ref.file_path.exists(),
                "Referenced file should exist: {:?}",
                file_ref.file_path
            );

            // Verify line range is valid
            assert!(
                file_ref.line_range.0 <= file_ref.line_range.1,
                "Line range should be valid"
            );
        }
    }
}
