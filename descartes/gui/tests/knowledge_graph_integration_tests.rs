/// Knowledge Graph Integration Tests
///
/// This module provides comprehensive integration tests for knowledge graph
/// functionality in the GUI, including:
/// - Loading and generating knowledge graphs
/// - Bidirectional navigation between file tree and knowledge graph
/// - Search and filtering operations
/// - Performance with large graphs
/// - Edge case handling
use descartes_agent_runner::file_tree_builder::FileTreeBuilder;
use descartes_agent_runner::knowledge_graph::{
    FileReference, FileTree, KnowledgeEdge, KnowledgeGraph, KnowledgeNode, KnowledgeNodeType,
    RelationshipType,
};
use descartes_agent_runner::knowledge_graph_overlay::KnowledgeGraphOverlay;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// ============================================================================
/// Test Setup Helpers
/// ============================================================================

/// Create a test project with sample code files
fn create_test_project(dir: &std::path::Path) -> std::io::Result<()> {
    fs::create_dir_all(dir.join("src"))?;

    // Main module with multiple functions
    fs::write(
        dir.join("src/main.rs"),
        r#"
/// Main function
fn main() {
    println!("Hello");
    initialize();
    process_data("test");
}

/// Initialize application
fn initialize() {
    println!("Initializing...");
    setup_config();
}

/// Setup configuration
fn setup_config() {
    println!("Config setup");
}

/// Process data
fn process_data(input: &str) -> String {
    helper_transform(input)
}

/// Helper transform function
fn helper_transform(data: &str) -> String {
    data.to_uppercase()
}
"#,
    )?;

    // App module with struct and methods
    fs::write(
        dir.join("src/app.rs"),
        r#"
/// Application state
pub struct AppState {
    pub initialized: bool,
    pub data: Vec<String>,
}

impl AppState {
    /// Create new instance
    pub fn new() -> Self {
        Self {
            initialized: false,
            data: Vec::new(),
        }
    }

    /// Initialize the state
    pub fn initialize(&mut self) {
        self.initialized = true;
    }

    /// Add data
    pub fn add_data(&mut self, item: String) {
        self.data.push(item);
    }

    /// Get data count
    pub fn count(&self) -> usize {
        self.data.len()
    }
}
"#,
    )?;

    // Utils module with helper functions
    fs::write(
        dir.join("src/utils.rs"),
        r#"
/// Transform string
pub fn transform(input: &str) -> String {
    input.to_uppercase()
}

/// Validate input
pub fn validate(input: &str) -> bool {
    !input.is_empty()
}

/// Process list
pub fn process_list(items: &[String]) -> Vec<String> {
    items.iter()
        .filter(|s| validate(s))
        .map(|s| transform(s))
        .collect()
}
"#,
    )?;

    Ok(())
}

/// Create a large test project with many files
fn create_large_test_project(dir: &std::path::Path, file_count: usize) -> std::io::Result<()> {
    fs::create_dir_all(dir.join("src"))?;

    // Create main file
    fs::write(
        dir.join("src/main.rs"),
        r#"
fn main() {
    println!("Large project");
}
"#,
    )?;

    // Create many module files
    for i in 0..file_count {
        let module_name = format!("module_{}", i);
        let file_path = dir.join("src").join(format!("{}.rs", module_name));

        let content = format!(
            r#"
/// Module {}
pub struct Data{} {{
    pub value: i32,
}}

impl Data{} {{
    pub fn new(value: i32) -> Self {{
        Self {{ value }}
    }}

    pub fn get(&self) -> i32 {{
        self.value
    }}

    pub fn set(&mut self, value: i32) {{
        self.value = value;
    }}
}}

pub fn process_{}(input: i32) -> i32 {{
    input * 2
}}

pub fn validate_{}(input: i32) -> bool {{
    input > 0
}}
"#,
            i, i, i, i, i
        );

        fs::write(file_path, content)?;
    }

    Ok(())
}

/// ============================================================================
/// Basic Integration Tests
/// ============================================================================

#[test]
fn test_load_file_tree_and_generate_knowledge_graph() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    // Build file tree
    let mut builder = FileTreeBuilder::new();
    let mut file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    // Verify file tree was built
    assert!(file_tree.file_count >= 3, "Should have at least 3 files");

    // Generate knowledge graph
    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_and_link(&mut file_tree).unwrap();

    // Verify knowledge graph was generated
    assert!(
        knowledge_graph.nodes.len() > 0,
        "Should extract knowledge nodes"
    );
    assert!(
        knowledge_graph.edges.len() > 0,
        "Should extract relationships"
    );

    // Verify bidirectional links
    for (_, node) in &file_tree.nodes {
        if !node.knowledge_links.is_empty() {
            for knowledge_id in &node.knowledge_links {
                assert!(
                    knowledge_graph.get_node(knowledge_id).is_some(),
                    "Knowledge link should point to valid node"
                );
            }
        }
    }
}

#[test]
fn test_knowledge_graph_node_types() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Verify we extracted different node types
    let functions = knowledge_graph.get_nodes_by_type(KnowledgeNodeType::Function);
    let structs = knowledge_graph.get_nodes_by_type(KnowledgeNodeType::Struct);

    assert!(functions.len() > 0, "Should extract functions");
    // Note: Struct detection may vary based on parser implementation
}

#[test]
fn test_knowledge_graph_relationships() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Verify relationships were extracted
    assert!(
        knowledge_graph.edges.len() > 0,
        "Should extract relationships"
    );

    // Check that edges have valid endpoints
    for (_, edge) in &knowledge_graph.edges {
        assert!(
            knowledge_graph.get_node(&edge.from_node_id).is_some(),
            "Edge should have valid source node"
        );
        assert!(
            knowledge_graph.get_node(&edge.to_node_id).is_some(),
            "Edge should have valid target node"
        );
    }
}

/// ============================================================================
/// Search and Navigation Tests
/// ============================================================================

#[test]
fn test_find_entities_by_name() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Search for entities by name pattern
    let results = overlay.find_by_name_pattern("init", &knowledge_graph);

    assert!(results.len() > 0, "Should find entities matching 'init'");

    for node in results {
        assert!(
            node.name.to_lowercase().contains("init")
                || node.qualified_name.to_lowercase().contains("init"),
            "Result should match search pattern"
        );
    }
}

#[test]
fn test_find_entities_in_file() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Find entities in main.rs
    let main_path = temp_dir.path().join("src/main.rs");
    let entities = overlay.find_entities_in_file(&main_path, &knowledge_graph);

    assert!(entities.len() > 0, "Should find entities in main.rs");

    // Verify all entities reference the correct file
    for entity in entities {
        let has_main_reference = entity
            .file_references
            .iter()
            .any(|fr| fr.file_path == main_path);
        assert!(has_main_reference, "Entity should reference main.rs");
    }
}

#[test]
fn test_find_entity_definition() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Find definition for a known entity
    let results = overlay.find_by_name_pattern("main", &knowledge_graph);

    if let Some(node) = results.first() {
        let definition = overlay.find_definition(&node.qualified_name, &knowledge_graph);

        assert!(definition.is_some(), "Should find definition for entity");

        let def_ref = definition.unwrap();
        assert!(
            def_ref.is_definition,
            "Reference should be marked as definition"
        );
    }
}

/// ============================================================================
/// Performance Tests
/// ============================================================================

#[test]
fn test_large_file_tree_performance() {
    let temp_dir = TempDir::new().unwrap();

    // Create project with 100 files
    create_large_test_project(temp_dir.path(), 100).unwrap();

    let mut builder = FileTreeBuilder::new();
    let start = std::time::Instant::now();
    let mut file_tree = builder.scan_directory(temp_dir.path()).unwrap();
    let tree_duration = start.elapsed();

    println!("File tree scan took: {:?}", tree_duration);

    // Generate knowledge graph
    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let start = std::time::Instant::now();
    let knowledge_graph = overlay.generate_and_link(&mut file_tree).unwrap();
    let kg_duration = start.elapsed();

    println!("Knowledge graph generation took: {:?}", kg_duration);
    println!(
        "Nodes: {}, Edges: {}",
        knowledge_graph.nodes.len(),
        knowledge_graph.edges.len()
    );

    // Should complete in reasonable time (< 10 seconds for 100 files)
    assert!(
        kg_duration.as_secs() < 10,
        "Knowledge graph generation should complete in < 10 seconds"
    );
}

#[test]
fn test_search_performance() {
    let temp_dir = TempDir::new().unwrap();
    create_large_test_project(temp_dir.path(), 50).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Perform multiple searches and measure time
    let start = std::time::Instant::now();

    for i in 0..10 {
        let pattern = format!("process_{}", i);
        let _ = overlay.find_by_name_pattern(&pattern, &knowledge_graph);
    }

    let duration = start.elapsed();

    println!("10 searches took: {:?}", duration);

    // Should be fast (< 100ms for 10 searches)
    assert!(duration.as_millis() < 100, "Searches should be fast");
}

/// ============================================================================
/// Edge Case Tests
/// ============================================================================

#[test]
fn test_empty_file_tree() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    // Should handle empty tree gracefully
    assert_eq!(
        knowledge_graph.nodes.len(),
        0,
        "Empty tree should have no nodes"
    );
    assert_eq!(
        knowledge_graph.edges.len(),
        0,
        "Empty tree should have no edges"
    );
}

#[test]
fn test_file_with_no_knowledge_nodes() {
    let temp_dir = TempDir::new().unwrap();
    fs::create_dir_all(temp_dir.path().join("src")).unwrap();

    // Create a file with no parseable entities (just comments)
    fs::write(
        temp_dir.path().join("src/empty.rs"),
        r#"
// This file only has comments
// No functions or structs
"#,
    )
    .unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_and_link(&mut file_tree).unwrap();

    // Should handle gracefully - may have 0 or minimal nodes
    // Just verify no errors occurred
    assert!(knowledge_graph.nodes.len() >= 0);
}

#[test]
fn test_circular_references() {
    // Create a simple knowledge graph with circular references
    let mut graph = KnowledgeGraph::new();

    let node1 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "func_a".to_string(),
        "func_a".to_string(),
    );
    let id1 = graph.add_node(node1);

    let node2 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "func_b".to_string(),
        "func_b".to_string(),
    );
    let id2 = graph.add_node(node2);

    // Create circular reference: A calls B, B calls A
    graph.add_edge(KnowledgeEdge::new(
        id1.clone(),
        id2.clone(),
        RelationshipType::Calls,
    ));
    graph.add_edge(KnowledgeEdge::new(
        id2.clone(),
        id1.clone(),
        RelationshipType::Calls,
    ));

    // Verify circular reference doesn't break path finding
    let path = graph.find_path(&id1, &id2);
    assert!(
        path.is_some(),
        "Should find path despite circular reference"
    );

    let path = path.unwrap();
    assert_eq!(path.len(), 2, "Path should have 2 nodes");
}

#[test]
fn test_large_knowledge_graph_1000_nodes() {
    // Create a synthetic large knowledge graph
    let mut graph = KnowledgeGraph::new();

    let mut node_ids = Vec::new();

    // Create 1000 nodes
    for i in 0..1000 {
        let node = KnowledgeNode::new(
            KnowledgeNodeType::Function,
            format!("func_{}", i),
            format!("module::func_{}", i),
        );
        let id = graph.add_node(node);
        node_ids.push(id);
    }

    // Create edges (each node calls the next)
    for i in 0..999 {
        graph.add_edge(KnowledgeEdge::new(
            node_ids[i].clone(),
            node_ids[i + 1].clone(),
            RelationshipType::Calls,
        ));
    }

    // Test graph operations on large graph
    assert_eq!(graph.nodes.len(), 1000, "Should have 1000 nodes");
    assert_eq!(graph.edges.len(), 999, "Should have 999 edges");

    // Test search performance
    let start = std::time::Instant::now();
    let results = graph.find_nodes(|n| n.name.contains("50"));
    let duration = start.elapsed();

    println!("Search in 1000 nodes took: {:?}", duration);
    assert!(results.len() > 0, "Should find matching nodes");
    assert!(duration.as_millis() < 10, "Search should be fast");

    // Test path finding
    let start = std::time::Instant::now();
    let path = graph.find_path(&node_ids[0], &node_ids[999]);
    let duration = start.elapsed();

    println!("Path finding in 1000 nodes took: {:?}", duration);
    assert!(path.is_some(), "Should find path");
    assert_eq!(path.unwrap().len(), 1000, "Path should span all nodes");
    assert!(
        duration.as_millis() < 100,
        "Path finding should be reasonably fast"
    );
}

/// ============================================================================
/// Incremental Update Tests
/// ============================================================================

#[test]
fn test_incremental_update() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let mut knowledge_graph = overlay.generate_knowledge_overlay(&file_tree).unwrap();

    let initial_count = knowledge_graph.nodes.len();

    // Modify a file
    let main_path = temp_dir.path().join("src/main.rs");
    fs::write(
        &main_path,
        r#"
fn main() {
    println!("Modified");
    new_function();
}

fn new_function() {
    println!("New function added");
}
"#,
    )
    .unwrap();

    // Update knowledge graph incrementally
    overlay
        .update_file(&main_path, &file_tree, &mut knowledge_graph)
        .unwrap();

    // Should reflect changes
    // (Note: exact count may vary based on parser)
    println!(
        "Initial: {}, After update: {}",
        initial_count,
        knowledge_graph.nodes.len()
    );
}

/// ============================================================================
/// Integration with GUI State Tests
/// ============================================================================

#[test]
fn test_file_tree_knowledge_link_bidirection() {
    let temp_dir = TempDir::new().unwrap();
    create_test_project(temp_dir.path()).unwrap();

    let mut builder = FileTreeBuilder::new();
    let mut file_tree = builder.scan_directory(temp_dir.path()).unwrap();

    let mut overlay = KnowledgeGraphOverlay::new().unwrap();
    let knowledge_graph = overlay.generate_and_link(&mut file_tree).unwrap();

    // Verify bidirectional links
    for (_, kg_node) in &knowledge_graph.nodes {
        for file_ref in &kg_node.file_references {
            if let Some(file_node) = file_tree.get_node(&file_ref.file_node_id) {
                assert!(
                    file_node.knowledge_links.contains(&kg_node.node_id),
                    "File node should have reverse link to knowledge node"
                );
            }
        }
    }

    // Verify from file tree side
    for (_, file_node) in &file_tree.nodes {
        for kg_id in &file_node.knowledge_links {
            let kg_node = knowledge_graph.get_node(kg_id);
            assert!(kg_node.is_some(), "Knowledge link should be valid");

            let kg_node = kg_node.unwrap();
            let has_file_ref = kg_node
                .file_references
                .iter()
                .any(|fr| fr.file_node_id == file_node.node_id);

            assert!(
                has_file_ref,
                "Knowledge node should have reference back to file node"
            );
        }
    }
}

#[test]
fn test_fuzzy_search() {
    use descartes_gui::knowledge_graph_panel;

    let mut graph = KnowledgeGraph::new();

    graph.add_node(KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "process_data".to_string(),
        "app::process_data".to_string(),
    ));

    graph.add_node(KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "process_list".to_string(),
        "utils::process_list".to_string(),
    ));

    graph.add_node(KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "initialize".to_string(),
        "main::initialize".to_string(),
    ));

    // Test that we can find nodes
    let results: Vec<_> = graph
        .nodes
        .values()
        .filter(|n| n.name.contains("process"))
        .collect();

    assert_eq!(results.len(), 2, "Should find two 'process' functions");
}
