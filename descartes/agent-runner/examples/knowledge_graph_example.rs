/// Example demonstrating the usage of FileTree and KnowledgeGraph models
///
/// This example shows:
/// - Creating a file tree structure
/// - Building a knowledge graph from code
/// - Linking file tree nodes to knowledge graph nodes
/// - Querying and traversing the structures
use agent_runner::{
    CodeRepository, FileMetadata, FileNodeType, FileReference, FileTree, FileTreeNode,
    KnowledgeEdge, KnowledgeGraph, KnowledgeNode, KnowledgeNodeType, Language, RelationshipType,
};
use std::path::PathBuf;

fn main() {
    println!("=== File Tree and Knowledge Graph Example ===\n");

    // Create a code repository
    let mut repo = CodeRepository::new(PathBuf::from("/home/user/project"));

    // ========================================================================
    // Part 1: Build File Tree
    // ========================================================================
    println!("1. Building File Tree...");

    // Create root directory
    let mut root = FileTreeNode::new(
        PathBuf::from("/home/user/project"),
        FileNodeType::Directory,
        None,
        0,
    );
    let root_id = repo.file_tree.add_node(root.clone());

    // Create src directory
    let mut src_dir = FileTreeNode::new(
        PathBuf::from("/home/user/project/src"),
        FileNodeType::Directory,
        Some(root_id.clone()),
        1,
    );
    let src_dir_id = repo.file_tree.add_node(src_dir.clone());

    // Add src as child of root
    if let Some(root_node) = repo.file_tree.get_node_mut(&root_id) {
        root_node.add_child(src_dir_id.clone());
    }

    // Create lib.rs file
    let mut lib_file = FileTreeNode::new(
        PathBuf::from("/home/user/project/src/lib.rs"),
        FileNodeType::File,
        Some(src_dir_id.clone()),
        2,
    );
    lib_file.metadata = FileMetadata {
        size: Some(5000),
        line_count: Some(150),
        language: Some(Language::Rust),
        is_binary: false,
        ..Default::default()
    };
    let lib_file_id = repo.file_tree.add_node(lib_file.clone());

    // Add lib.rs as child of src
    if let Some(src_node) = repo.file_tree.get_node_mut(&src_dir_id) {
        src_node.add_child(lib_file_id.clone());
    }

    // Create main.rs file
    let mut main_file = FileTreeNode::new(
        PathBuf::from("/home/user/project/src/main.rs"),
        FileNodeType::File,
        Some(src_dir_id.clone()),
        2,
    );
    main_file.metadata = FileMetadata {
        size: Some(3000),
        line_count: Some(100),
        language: Some(Language::Rust),
        is_binary: false,
        ..Default::default()
    };
    let main_file_id = repo.file_tree.add_node(main_file.clone());

    // Add main.rs as child of src
    if let Some(src_node) = repo.file_tree.get_node_mut(&src_dir_id) {
        src_node.add_child(main_file_id.clone());
    }

    println!(
        "  Created file tree with {} nodes",
        repo.file_tree.nodes.len()
    );
    println!("  Files: {}", repo.file_tree.file_count);
    println!("  Directories: {}\n", repo.file_tree.directory_count);

    // ========================================================================
    // Part 2: Build Knowledge Graph
    // ========================================================================
    println!("2. Building Knowledge Graph...");

    // Create a module node
    let mut module_node = KnowledgeNode::new(
        KnowledgeNodeType::Module,
        "mylib".to_string(),
        "mylib".to_string(),
    );
    module_node.description = Some("Main library module".to_string());
    module_node.language = Some(Language::Rust);
    module_node.add_file_reference(FileReference::new(
        lib_file_id.clone(),
        PathBuf::from("/home/user/project/src/lib.rs"),
        (1, 150),
    ));
    let module_id = repo.knowledge_graph.add_node(module_node);

    // Create a struct node
    let mut struct_node = KnowledgeNode::new(
        KnowledgeNodeType::Struct,
        "Config".to_string(),
        "mylib::Config".to_string(),
    );
    struct_node.description = Some("Configuration structure".to_string());
    struct_node.language = Some(Language::Rust);
    struct_node.source_code = Some("pub struct Config { pub name: String }".to_string());
    struct_node.parent_id = Some(module_id.clone());
    struct_node.visibility = Some("public".to_string());
    struct_node.add_file_reference(FileReference::new(
        lib_file_id.clone(),
        PathBuf::from("/home/user/project/src/lib.rs"),
        (10, 15),
    ));
    let struct_id = repo.knowledge_graph.add_node(struct_node);

    // Create a function node
    let mut func_node = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "load_config".to_string(),
        "mylib::load_config".to_string(),
    );
    func_node.description = Some("Loads configuration from file".to_string());
    func_node.language = Some(Language::Rust);
    func_node.signature = Some("pub fn load_config(path: &str) -> Result<Config>".to_string());
    func_node.return_type = Some("Result<Config>".to_string());
    func_node.parameters = vec!["path: &str".to_string()];
    func_node.parent_id = Some(module_id.clone());
    func_node.visibility = Some("public".to_string());
    func_node.add_file_reference(FileReference::new(
        lib_file_id.clone(),
        PathBuf::from("/home/user/project/src/lib.rs"),
        (20, 30),
    ));
    let func_id = repo.knowledge_graph.add_node(func_node);

    // Create main function node
    let mut main_func = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "main".to_string(),
        "main".to_string(),
    );
    main_func.description = Some("Application entry point".to_string());
    main_func.language = Some(Language::Rust);
    main_func.signature = Some("fn main()".to_string());
    main_func.add_file_reference(FileReference::new(
        main_file_id.clone(),
        PathBuf::from("/home/user/project/src/main.rs"),
        (1, 20),
    ));
    let main_id = repo.knowledge_graph.add_node(main_func);

    println!(
        "  Created knowledge graph with {} nodes",
        repo.knowledge_graph.nodes.len()
    );

    // ========================================================================
    // Part 3: Create Relationships (Edges)
    // ========================================================================
    println!("\n3. Creating Relationships...");

    // Module defines struct
    let edge1 = KnowledgeEdge::new(
        module_id.clone(),
        struct_id.clone(),
        RelationshipType::Defines,
    );
    repo.knowledge_graph.add_edge(edge1);

    // Module defines function
    let edge2 = KnowledgeEdge::new(
        module_id.clone(),
        func_id.clone(),
        RelationshipType::Defines,
    );
    repo.knowledge_graph.add_edge(edge2);

    // Function uses struct
    let edge3 = KnowledgeEdge::new(func_id.clone(), struct_id.clone(), RelationshipType::Uses);
    repo.knowledge_graph.add_edge(edge3);

    // Main calls load_config
    let edge4 = KnowledgeEdge::new(main_id.clone(), func_id.clone(), RelationshipType::Calls);
    repo.knowledge_graph.add_edge(edge4);

    println!("  Created {} edges", repo.knowledge_graph.edges.len());

    // ========================================================================
    // Part 4: Link File Tree to Knowledge Graph
    // ========================================================================
    println!("\n4. Linking File Tree to Knowledge Graph...");

    // Link lib.rs to its knowledge nodes
    if let Some(lib_node) = repo.file_tree.get_node_mut(&lib_file_id) {
        lib_node.add_knowledge_link(module_id.clone());
        lib_node.add_knowledge_link(struct_id.clone());
        lib_node.add_knowledge_link(func_id.clone());
        lib_node.indexed = true;
    }

    // Link main.rs to its knowledge nodes
    if let Some(main_node) = repo.file_tree.get_node_mut(&main_file_id) {
        main_node.add_knowledge_link(main_id.clone());
        main_node.indexed = true;
    }

    println!("  Linked file tree nodes to knowledge graph");

    // ========================================================================
    // Part 5: Query and Traverse
    // ========================================================================
    println!("\n5. Querying and Traversing...\n");

    // Traverse file tree depth-first
    println!("  File Tree (depth-first):");
    repo.file_tree.traverse_depth_first(|node| {
        let indent = "  ".repeat(node.depth);
        let node_type = if node.is_directory() { "DIR" } else { "FILE" };
        println!("    {}{} [{}]", indent, node.name, node_type);
    });

    // Find all Rust files
    println!("\n  Rust files:");
    let rust_files = repo
        .file_tree
        .find_nodes(|node| node.metadata.language == Some(Language::Rust));
    for file in rust_files {
        println!(
            "    - {} ({} lines)",
            file.path.display(),
            file.metadata.line_count.unwrap_or(0)
        );
    }

    // Get all functions in the knowledge graph
    println!("\n  Functions in knowledge graph:");
    let functions = repo
        .knowledge_graph
        .get_nodes_by_type(KnowledgeNodeType::Function);
    for func in functions {
        println!("    - {}", func.qualified_name);
        if let Some(sig) = &func.signature {
            println!("      {}", sig);
        }
    }

    // Find what the main function calls
    println!("\n  Dependencies of main():");
    let called_by_main = repo
        .knowledge_graph
        .get_neighbors_by_relationship(&main_id, RelationshipType::Calls);
    for dep in called_by_main {
        println!("    - calls: {}", dep.name);
    }

    // Find what uses the Config struct
    println!("\n  What uses Config struct:");
    let users_of_config = repo.knowledge_graph.get_incoming_edges(&struct_id);
    for edge in users_of_config {
        if let Some(from_node) = repo.knowledge_graph.get_node(&edge.from_node_id) {
            println!(
                "    - {} {} it",
                from_node.name,
                edge.relationship_type.as_str()
            );
        }
    }

    // Find path between nodes
    println!("\n  Path from main to Config struct:");
    if let Some(path) = repo.knowledge_graph.find_path(&main_id, &struct_id) {
        print!("    ");
        for (i, node_id) in path.iter().enumerate() {
            if let Some(node) = repo.knowledge_graph.get_node(node_id) {
                print!("{}", node.name);
                if i < path.len() - 1 {
                    print!(" -> ");
                }
            }
        }
        println!();
    }

    // ========================================================================
    // Part 6: Statistics
    // ========================================================================
    println!("\n6. Statistics:\n");

    let stats = repo.stats();

    println!("  File Tree:");
    println!("    Total nodes: {}", stats.file_tree_stats.total_nodes);
    println!("    Files: {}", stats.file_tree_stats.file_count);
    println!("    Directories: {}", stats.file_tree_stats.directory_count);
    println!("    Indexed: {}", stats.file_tree_stats.indexed_count);
    println!("    Max depth: {}", stats.file_tree_stats.max_depth);

    println!("\n  Knowledge Graph:");
    println!(
        "    Total nodes: {}",
        stats.knowledge_graph_stats.total_nodes
    );
    println!(
        "    Total edges: {}",
        stats.knowledge_graph_stats.total_edges
    );
    println!(
        "    Average degree: {:.2}",
        stats.knowledge_graph_stats.avg_degree
    );
    println!("    Node types:");
    for (node_type, count) in &stats.knowledge_graph_stats.node_type_counts {
        println!("      - {}: {}", node_type, count);
    }

    println!("\n=== Example Complete ===");
}
