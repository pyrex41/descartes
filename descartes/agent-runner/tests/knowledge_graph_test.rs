/// Integration tests for File Tree and Knowledge Graph models
use agent_runner::{
    CodeRepository, FileMetadata, FileNodeType, FileReference, FileTree, FileTreeNode,
    KnowledgeEdge, KnowledgeGraph, KnowledgeNode, KnowledgeNodeType, Language, RelationshipType,
};
use std::path::PathBuf;

#[test]
fn test_file_tree_creation() {
    let mut tree = FileTree::new(PathBuf::from("/test"));

    let root = FileTreeNode::new(PathBuf::from("/test"), FileNodeType::Directory, None, 0);
    let root_id = tree.add_node(root);

    assert_eq!(tree.directory_count, 1);
    assert_eq!(tree.file_count, 0);
    assert_eq!(tree.root_id, Some(root_id.clone()));

    let node = tree.get_node(&root_id).unwrap();
    assert_eq!(node.name, "test");
    assert!(node.is_directory());
}

#[test]
fn test_file_tree_hierarchy() {
    let mut tree = FileTree::new(PathBuf::from("/test"));

    // Create root
    let root = FileTreeNode::new(PathBuf::from("/test"), FileNodeType::Directory, None, 0);
    let root_id = tree.add_node(root);

    // Create child directory
    let child_dir = FileTreeNode::new(
        PathBuf::from("/test/src"),
        FileNodeType::Directory,
        Some(root_id.clone()),
        1,
    );
    let child_dir_id = tree.add_node(child_dir);

    // Link parent to child
    if let Some(root_node) = tree.get_node_mut(&root_id) {
        root_node.add_child(child_dir_id.clone());
    }

    // Create file
    let file = FileTreeNode::new(
        PathBuf::from("/test/src/main.rs"),
        FileNodeType::File,
        Some(child_dir_id.clone()),
        2,
    );
    let file_id = tree.add_node(file);

    // Link directory to file
    if let Some(dir_node) = tree.get_node_mut(&child_dir_id) {
        dir_node.add_child(file_id.clone());
    }

    assert_eq!(tree.file_count, 1);
    assert_eq!(tree.directory_count, 2);

    let children = tree.get_children(&root_id);
    assert_eq!(children.len(), 1);
    assert_eq!(children[0].name, "src");
}

#[test]
fn test_file_tree_traversal() {
    let mut tree = FileTree::new(PathBuf::from("/test"));

    let root = FileTreeNode::new(PathBuf::from("/test"), FileNodeType::Directory, None, 0);
    let root_id = tree.add_node(root);

    let file1 = FileTreeNode::new(
        PathBuf::from("/test/a.txt"),
        FileNodeType::File,
        Some(root_id.clone()),
        1,
    );
    let file1_id = tree.add_node(file1);

    let file2 = FileTreeNode::new(
        PathBuf::from("/test/b.txt"),
        FileNodeType::File,
        Some(root_id.clone()),
        1,
    );
    let file2_id = tree.add_node(file2);

    if let Some(root_node) = tree.get_node_mut(&root_id) {
        root_node.add_child(file1_id.clone());
        root_node.add_child(file2_id.clone());
    }

    let mut visited = Vec::new();
    tree.traverse_depth_first(|node| {
        visited.push(node.name.clone());
    });

    assert_eq!(visited.len(), 3);
    assert_eq!(visited[0], "test");
}

#[test]
fn test_file_tree_query() {
    let mut tree = FileTree::new(PathBuf::from("/test"));

    let mut file1 = FileTreeNode::new(PathBuf::from("/test/main.rs"), FileNodeType::File, None, 0);
    file1.metadata.language = Some(Language::Rust);
    tree.add_node(file1);

    let mut file2 = FileTreeNode::new(
        PathBuf::from("/test/script.py"),
        FileNodeType::File,
        None,
        0,
    );
    file2.metadata.language = Some(Language::Python);
    tree.add_node(file2);

    let rust_files = tree.find_nodes(|node| node.metadata.language == Some(Language::Rust));
    assert_eq!(rust_files.len(), 1);
    assert_eq!(rust_files[0].name, "main.rs");
}

#[test]
fn test_knowledge_node_creation() {
    let mut node = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "test_func".to_string(),
        "module::test_func".to_string(),
    );

    assert_eq!(node.name, "test_func");
    assert_eq!(node.qualified_name, "module::test_func");
    assert_eq!(node.content_type, KnowledgeNodeType::Function);

    node.add_tag("test".to_string());
    assert!(node.tags.contains("test"));
}

#[test]
fn test_knowledge_graph_creation() {
    let mut graph = KnowledgeGraph::new();

    let node1 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "func_a".to_string(),
        "module::func_a".to_string(),
    );

    let node2 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "func_b".to_string(),
        "module::func_b".to_string(),
    );

    let id1 = graph.add_node(node1);
    let id2 = graph.add_node(node2);

    assert_eq!(graph.nodes.len(), 2);
    assert!(graph.get_node(&id1).is_some());
    assert!(graph.get_node(&id2).is_some());
}

#[test]
fn test_knowledge_graph_edges() {
    let mut graph = KnowledgeGraph::new();

    let node1 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "caller".to_string(),
        "caller".to_string(),
    );

    let node2 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "callee".to_string(),
        "callee".to_string(),
    );

    let id1 = graph.add_node(node1);
    let id2 = graph.add_node(node2);

    let edge = KnowledgeEdge::new(id1.clone(), id2.clone(), RelationshipType::Calls);
    let edge_id = graph.add_edge(edge);

    assert_eq!(graph.edges.len(), 1);

    let outgoing = graph.get_outgoing_edges(&id1);
    assert_eq!(outgoing.len(), 1);
    assert_eq!(outgoing[0].relationship_type, RelationshipType::Calls);

    let incoming = graph.get_incoming_edges(&id2);
    assert_eq!(incoming.len(), 1);
}

#[test]
fn test_knowledge_graph_neighbors() {
    let mut graph = KnowledgeGraph::new();

    let node1 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "a".to_string(),
        "a".to_string(),
    );
    let node2 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "b".to_string(),
        "b".to_string(),
    );
    let node3 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "c".to_string(),
        "c".to_string(),
    );

    let id1 = graph.add_node(node1);
    let id2 = graph.add_node(node2);
    let id3 = graph.add_node(node3);

    graph.add_edge(KnowledgeEdge::new(
        id1.clone(),
        id2.clone(),
        RelationshipType::Calls,
    ));
    graph.add_edge(KnowledgeEdge::new(
        id1.clone(),
        id3.clone(),
        RelationshipType::Uses,
    ));

    let neighbors = graph.get_neighbors(&id1);
    assert_eq!(neighbors.len(), 2);

    let called = graph.get_neighbors_by_relationship(&id1, RelationshipType::Calls);
    assert_eq!(called.len(), 1);
    assert_eq!(called[0].name, "b");
}

#[test]
fn test_knowledge_graph_path_finding() {
    let mut graph = KnowledgeGraph::new();

    let node1 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "a".to_string(),
        "a".to_string(),
    );
    let node2 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "b".to_string(),
        "b".to_string(),
    );
    let node3 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "c".to_string(),
        "c".to_string(),
    );

    let id1 = graph.add_node(node1);
    let id2 = graph.add_node(node2);
    let id3 = graph.add_node(node3);

    graph.add_edge(KnowledgeEdge::new(
        id1.clone(),
        id2.clone(),
        RelationshipType::Calls,
    ));
    graph.add_edge(KnowledgeEdge::new(
        id2.clone(),
        id3.clone(),
        RelationshipType::Calls,
    ));

    let path = graph.find_path(&id1, &id3);
    assert!(path.is_some());

    let path = path.unwrap();
    assert_eq!(path.len(), 3);
    assert_eq!(path[0], id1);
    assert_eq!(path[1], id2);
    assert_eq!(path[2], id3);
}

#[test]
fn test_knowledge_graph_type_index() {
    let mut graph = KnowledgeGraph::new();

    let func1 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "func1".to_string(),
        "func1".to_string(),
    );
    let func2 = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "func2".to_string(),
        "func2".to_string(),
    );
    let class1 = KnowledgeNode::new(
        KnowledgeNodeType::Class,
        "class1".to_string(),
        "class1".to_string(),
    );

    graph.add_node(func1);
    graph.add_node(func2);
    graph.add_node(class1);

    let functions = graph.get_nodes_by_type(KnowledgeNodeType::Function);
    assert_eq!(functions.len(), 2);

    let classes = graph.get_nodes_by_type(KnowledgeNodeType::Class);
    assert_eq!(classes.len(), 1);
}

#[test]
fn test_code_repository() {
    let mut repo = CodeRepository::new(PathBuf::from("/test"));

    // Add file tree node
    let file = FileTreeNode::new(PathBuf::from("/test/main.rs"), FileNodeType::File, None, 0);
    let file_id = repo.file_tree.add_node(file);

    // Add knowledge node
    let func = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "main".to_string(),
        "main".to_string(),
    );
    let func_id = repo.knowledge_graph.add_node(func);

    // Link them
    if let Some(file_node) = repo.file_tree.get_node_mut(&file_id) {
        file_node.add_knowledge_link(func_id.clone());
    }

    if let Some(func_node) = repo.knowledge_graph.get_node_mut(&func_id) {
        func_node.add_file_reference(FileReference::new(
            file_id.clone(),
            PathBuf::from("/test/main.rs"),
            (1, 10),
        ));
    }

    // Verify links
    let file_node = repo.file_tree.get_node(&file_id).unwrap();
    assert_eq!(file_node.knowledge_links.len(), 1);
    assert_eq!(file_node.knowledge_links[0], func_id);

    let func_node = repo.knowledge_graph.get_node(&func_id).unwrap();
    assert_eq!(func_node.file_references.len(), 1);
    assert_eq!(func_node.file_references[0].file_node_id, file_id);
}

#[test]
fn test_statistics() {
    let mut repo = CodeRepository::new(PathBuf::from("/test"));

    // Add some nodes
    let root = FileTreeNode::new(PathBuf::from("/test"), FileNodeType::Directory, None, 0);
    repo.file_tree.add_node(root);

    let file = FileTreeNode::new(PathBuf::from("/test/main.rs"), FileNodeType::File, None, 1);
    repo.file_tree.add_node(file);

    let func = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "main".to_string(),
        "main".to_string(),
    );
    repo.knowledge_graph.add_node(func);

    // Get stats
    let stats = repo.stats();

    assert_eq!(stats.file_tree_stats.total_nodes, 2);
    assert_eq!(stats.file_tree_stats.file_count, 1);
    assert_eq!(stats.file_tree_stats.directory_count, 1);

    assert_eq!(stats.knowledge_graph_stats.total_nodes, 1);
    assert_eq!(stats.knowledge_graph_stats.total_edges, 0);
}

#[test]
fn test_edge_weight() {
    let edge = KnowledgeEdge::new(
        "id1".to_string(),
        "id2".to_string(),
        RelationshipType::Calls,
    )
    .with_weight(0.5);

    assert_eq!(edge.weight, 0.5);

    // Test clamping
    let edge2 = KnowledgeEdge::new(
        "id1".to_string(),
        "id2".to_string(),
        RelationshipType::Calls,
    )
    .with_weight(1.5);

    assert_eq!(edge2.weight, 1.0);
}

#[test]
fn test_relationship_type_strings() {
    assert_eq!(RelationshipType::Calls.as_str(), "calls");
    assert_eq!(RelationshipType::Imports.as_str(), "imports");
    assert_eq!(RelationshipType::Inherits.as_str(), "inherits");
    assert_eq!(RelationshipType::Implements.as_str(), "implements");
}

#[test]
fn test_knowledge_node_type_strings() {
    assert_eq!(KnowledgeNodeType::Function.as_str(), "function");
    assert_eq!(KnowledgeNodeType::Class.as_str(), "class");
    assert_eq!(KnowledgeNodeType::Struct.as_str(), "struct");
}

#[test]
fn test_file_node_type_strings() {
    assert_eq!(FileNodeType::File.as_str(), "file");
    assert_eq!(FileNodeType::Directory.as_str(), "directory");
    assert_eq!(FileNodeType::Symlink.as_str(), "symlink");
}

#[test]
fn test_file_metadata_default() {
    let metadata = FileMetadata::default();
    assert!(!metadata.is_binary);
    assert!(metadata.size.is_none());
    assert!(metadata.language.is_none());
}

#[test]
fn test_serialization() {
    let node = KnowledgeNode::new(
        KnowledgeNodeType::Function,
        "test".to_string(),
        "module::test".to_string(),
    );

    // Test JSON serialization
    let json = serde_json::to_string(&node).unwrap();
    assert!(json.contains("\"name\":\"test\""));

    let deserialized: KnowledgeNode = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "test");
}
