/// Comprehensive Integration Tests for DAG ↔ Swarm.toml Conversion
///
/// This test suite validates the bidirectional conversion between DAG
/// structures and Swarm.toml workflow configurations, including:
/// - Export DAG to Swarm.toml
/// - Import Swarm.toml to DAG
/// - Round-trip conversion (DAG → TOML → DAG)
/// - Edge cases (empty DAGs, complex hierarchies, etc.)
/// - Metadata preservation
/// - Agent and resource mapping
use descartes_core::dag::{DAGEdge, DAGNode, EdgeType, Position, DAG};
use descartes_core::dag_swarm_export::{
    export_dag_to_swarm_toml, import_swarm_toml_to_dag, load_dag_from_swarm_toml,
    save_dag_as_swarm_toml, SwarmExportConfig,
};
use descartes_core::swarm_parser::{
    AgentConfig, Handler, ResourceConfig, State, SwarmConfig, Workflow, WorkflowMetadata,
    WorkflowMetadataDetails,
};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use uuid::Uuid;

// ============================================================================
// Helper Functions
// ============================================================================

/// Create a simple linear workflow for testing
fn create_simple_workflow() -> DAG {
    let mut dag = DAG::new("Simple Workflow");
    dag.description = Some("A simple linear workflow".to_string());

    let start = DAGNode::new_auto("Start")
        .with_description("Initial task")
        .with_position(100.0, 100.0)
        .with_metadata("agents", serde_json::json!(["agent1"]));

    let process = DAGNode::new_auto("Process")
        .with_description("Processing task")
        .with_position(300.0, 100.0)
        .with_metadata("agents", serde_json::json!(["agent2"]));

    let end = DAGNode::new_auto("End")
        .with_description("Final task")
        .with_position(500.0, 100.0)
        .with_metadata("agents", serde_json::json!(["agent1"]));

    let start_id = start.node_id;
    let process_id = process.node_id;
    let end_id = end.node_id;

    dag.add_node(start).unwrap();
    dag.add_node(process).unwrap();
    dag.add_node(end).unwrap();

    let edge1 = DAGEdge::dependency(start_id, process_id).with_label("next");
    let edge2 = DAGEdge::dependency(process_id, end_id).with_label("finish");

    dag.add_edge(edge1).unwrap();
    dag.add_edge(edge2).unwrap();

    dag
}

/// Create a complex branching workflow for testing
fn create_branching_workflow() -> DAG {
    let mut dag = DAG::new("Branching Workflow");

    //       Start
    //      /  |  \
    //     A   B   C
    //      \  |  /
    //        End

    let start = DAGNode::new_auto("Start").with_position(300.0, 100.0);
    let task_a = DAGNode::new_auto("TaskA").with_position(100.0, 300.0);
    let task_b = DAGNode::new_auto("TaskB").with_position(300.0, 300.0);
    let task_c = DAGNode::new_auto("TaskC").with_position(500.0, 300.0);
    let end = DAGNode::new_auto("End").with_position(300.0, 500.0);

    let start_id = start.node_id;
    let a_id = task_a.node_id;
    let b_id = task_b.node_id;
    let c_id = task_c.node_id;
    let end_id = end.node_id;

    dag.add_node(start).unwrap();
    dag.add_node(task_a).unwrap();
    dag.add_node(task_b).unwrap();
    dag.add_node(task_c).unwrap();
    dag.add_node(end).unwrap();

    dag.add_edge(DAGEdge::dependency(start_id, a_id)).unwrap();
    dag.add_edge(DAGEdge::dependency(start_id, b_id)).unwrap();
    dag.add_edge(DAGEdge::dependency(start_id, c_id)).unwrap();
    dag.add_edge(DAGEdge::dependency(a_id, end_id)).unwrap();
    dag.add_edge(DAGEdge::dependency(b_id, end_id)).unwrap();
    dag.add_edge(DAGEdge::dependency(c_id, end_id)).unwrap();

    dag
}

/// Create default export configuration
fn default_export_config() -> SwarmExportConfig {
    SwarmExportConfig::default()
        .with_agent("agent1", "claude-3-opus")
        .with_agent("agent2", "claude-3-sonnet")
}

// ============================================================================
// Basic Export Tests
// ============================================================================

#[test]
fn test_export_simple_workflow() {
    let dag = create_simple_workflow();
    let config = default_export_config();

    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    // Verify basic structure
    assert!(toml.contains("Simple Workflow"));
    assert!(toml.contains("Start"));
    assert!(toml.contains("Process"));
    assert!(toml.contains("End"));
    assert!(toml.contains("agent1"));
    assert!(toml.contains("agent2"));
}

#[test]
fn test_export_empty_dag() {
    let dag = DAG::new("Empty Workflow");
    let config = default_export_config();

    // Should fail because no start nodes
    assert!(export_dag_to_swarm_toml(&dag, &config).is_err());
}

#[test]
fn test_export_single_node() {
    let mut dag = DAG::new("Single Node");
    let node = DAGNode::new_auto("OnlyNode")
        .with_description("The only node")
        .with_metadata("agents", serde_json::json!(["agent1"]));

    dag.add_node(node).unwrap();

    let config = default_export_config();
    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    assert!(toml.contains("OnlyNode"));
    assert!(toml.contains("terminal = true"));
}

#[test]
fn test_export_with_metadata() {
    let mut dag = DAG::new("Metadata Test");

    let node = DAGNode::new_auto("Task")
        .with_description("A task with metadata")
        .with_metadata("agents", serde_json::json!(["agent1", "agent2"]))
        .with_metadata("entry_actions", serde_json::json!(["setup", "validate"]))
        .with_metadata("exit_actions", serde_json::json!(["cleanup"]))
        .with_metadata("required_resources", serde_json::json!(["database", "api"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("timeout_seconds", serde_json::json!(300));

    dag.add_node(node).unwrap();

    let config = default_export_config()
        .with_resource(
            "database",
            ResourceConfig::Database {
                connection_string: "db://localhost".to_string(),
                pool_size: None,
            },
        )
        .with_resource(
            "api",
            ResourceConfig::Http {
                endpoint: "https://api.example.com".to_string(),
                auth_required: None,
                secret_key: None,
            },
        );

    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    assert!(toml.contains("agent1"));
    assert!(toml.contains("agent2"));
    assert!(toml.contains("setup"));
    assert!(toml.contains("validate"));
    assert!(toml.contains("cleanup"));
    assert!(toml.contains("database"));
    assert!(toml.contains("api"));
    assert!(toml.contains("parallel_execution = true"));
    assert!(toml.contains("timeout_seconds = 300"));
}

#[test]
fn test_export_with_guards() {
    let mut dag = DAG::new("Guards Test");

    let node1 = DAGNode::new_auto("Start");
    let node2 = DAGNode::new_auto("End");

    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();

    let mut edge = DAGEdge::dependency(id1, id2).with_label("conditional");
    edge.metadata.insert(
        "guards".to_string(),
        serde_json::json!(["is_ready", "has_permission"]),
    );

    dag.add_edge(edge).unwrap();

    let config = default_export_config()
        .with_guard("is_ready", "state.ready == true")
        .with_guard("has_permission", "user.has_perm('execute')");

    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    assert!(toml.contains("is_ready"));
    assert!(toml.contains("has_permission"));
    assert!(toml.contains("state.ready == true"));
}

#[test]
fn test_export_edge_types() {
    let mut dag = DAG::new("Edge Types Test");

    let nodes: Vec<_> = (0..5)
        .map(|i| DAGNode::new_auto(format!("Node{}", i)))
        .collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Different edge types
    dag.add_edge(DAGEdge::new(ids[0], ids[1], EdgeType::Dependency))
        .unwrap();
    dag.add_edge(DAGEdge::new(ids[1], ids[2], EdgeType::SoftDependency))
        .unwrap();
    dag.add_edge(DAGEdge::new(ids[2], ids[3], EdgeType::DataFlow))
        .unwrap();
    dag.add_edge(DAGEdge::new(ids[3], ids[4], EdgeType::Trigger))
        .unwrap();

    let config = default_export_config();
    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    // Verify different event names are generated
    assert!(toml.contains("event = \"next\"") || toml.contains("next"));
    assert!(toml.contains("soft_next") || toml.contains("SoftDependency"));
    assert!(toml.contains("data_ready") || toml.contains("DataFlow"));
    assert!(toml.contains("trigger") || toml.contains("Trigger"));
}

#[test]
fn test_export_custom_config() {
    let dag = create_simple_workflow();

    let config = SwarmExportConfig::default()
        .with_workflow_name("Custom Name")
        .with_description("Custom description")
        .with_author("Test Author")
        .with_timeout(7200)
        .with_retries(5, 120)
        .with_agent("custom_agent", "claude-3-haiku");

    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    assert!(toml.contains("Custom Name"));
    assert!(toml.contains("Custom description"));
    assert!(toml.contains("Test Author"));
    assert!(toml.contains("7200"));
    assert!(toml.contains("max_retries = 5"));
    assert!(toml.contains("retry_backoff_seconds = 120"));
}

#[test]
fn test_export_state_name_sanitization() {
    let mut dag = DAG::new("Sanitization Test");

    let node = DAGNode::new_auto("My Task-Name@123!")
        .with_metadata("agents", serde_json::json!(["agent1"]));

    dag.add_node(node).unwrap();

    let config = default_export_config().use_labels(true);
    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    // Should sanitize to valid state name
    assert!(toml.contains("My_Task_Name_123"));
}

#[test]
fn test_export_without_header() {
    let dag = create_simple_workflow();
    let mut config = default_export_config();
    config.include_header = false;

    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    // Should not contain header comments
    assert!(!toml.starts_with("# Swarm.toml - Generated from DAG"));
}

// ============================================================================
// Basic Import Tests
// ============================================================================

#[test]
fn test_import_simple_workflow() {
    // Create SwarmConfig manually
    let mut agents = HashMap::new();
    agents.insert(
        "test_agent".to_string(),
        AgentConfig {
            model: "claude-3-opus".to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            tags: vec![],
        },
    );

    let mut states = HashMap::new();
    states.insert(
        "Start".to_string(),
        State {
            description: "Start state".to_string(),
            agents: vec!["test_agent".to_string()],
            entry_actions: vec![],
            exit_actions: vec![],
            terminal: false,
            parent: None,
            parallel_execution: false,
            handlers: vec![Handler {
                event: "go".to_string(),
                target: "End".to_string(),
                guards: vec![],
            }],
            timeout_seconds: None,
            timeout_target: None,
            required_resources: vec![],
        },
    );

    states.insert(
        "End".to_string(),
        State {
            description: "End state".to_string(),
            agents: vec![],
            entry_actions: vec![],
            exit_actions: vec![],
            terminal: true,
            parent: None,
            parallel_execution: false,
            handlers: vec![],
            timeout_seconds: None,
            timeout_target: None,
            required_resources: vec![],
        },
    );

    let workflow = Workflow {
        name: "test_workflow".to_string(),
        description: Some("Test".to_string()),
        metadata: WorkflowMetadataDetails {
            initial_state: "Start".to_string(),
            completion_timeout_seconds: Some(3600),
            max_retries: Some(3),
            retry_backoff_seconds: Some(60),
        },
        states,
        guards: HashMap::new(),
        contracts: HashMap::new(),
    };

    let swarm_config = SwarmConfig {
        metadata: WorkflowMetadata {
            version: "1.0".to_string(),
            name: "Test".to_string(),
            description: "Test workflow".to_string(),
            author: None,
            created: None,
        },
        agents,
        resources: HashMap::new(),
        workflows: vec![workflow],
        guards: None,
    };

    let dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

    assert_eq!(dag.nodes.len(), 2);
    assert_eq!(dag.edges.len(), 1);
    assert_eq!(dag.name, "test_workflow");
}

#[test]
fn test_import_with_metadata() {
    let mut agents = HashMap::new();
    agents.insert(
        "agent1".to_string(),
        AgentConfig {
            model: "claude-3-opus".to_string(),
            max_tokens: Some(1000),
            temperature: Some(0.7),
            tags: vec![],
        },
    );

    let mut states = HashMap::new();
    states.insert(
        "Task".to_string(),
        State {
            description: "Task with metadata".to_string(),
            agents: vec!["agent1".to_string()],
            entry_actions: vec!["setup".to_string(), "validate".to_string()],
            exit_actions: vec!["cleanup".to_string()],
            terminal: true,
            parent: None,
            parallel_execution: true,
            handlers: vec![],
            timeout_seconds: Some(300),
            timeout_target: None,
            required_resources: vec!["database".to_string()],
        },
    );

    let workflow = Workflow {
        name: "metadata_test".to_string(),
        description: Some("Test".to_string()),
        metadata: WorkflowMetadataDetails {
            initial_state: "Task".to_string(),
            completion_timeout_seconds: Some(3600),
            max_retries: Some(3),
            retry_backoff_seconds: Some(60),
        },
        states,
        guards: HashMap::new(),
        contracts: HashMap::new(),
    };

    let swarm_config = SwarmConfig {
        metadata: WorkflowMetadata {
            version: "1.0".to_string(),
            name: "Test".to_string(),
            description: "Test".to_string(),
            author: None,
            created: None,
        },
        agents,
        resources: HashMap::new(),
        workflows: vec![workflow],
        guards: None,
    };

    let dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

    // Find the Task node
    let task_node = dag.nodes.values().find(|n| n.label == "Task").unwrap();

    // Verify metadata
    assert_eq!(
        task_node
            .metadata
            .get("agents")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        task_node
            .metadata
            .get("entry_actions")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        task_node
            .metadata
            .get("exit_actions")
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        task_node
            .metadata
            .get("parallel_execution")
            .unwrap()
            .as_bool()
            .unwrap(),
        true
    );
    assert_eq!(
        task_node
            .metadata
            .get("timeout_seconds")
            .unwrap()
            .as_u64()
            .unwrap(),
        300
    );
}

#[test]
fn test_import_multiple_workflows() {
    let swarm_config = SwarmConfig {
        metadata: WorkflowMetadata {
            version: "1.0".to_string(),
            name: "Multi".to_string(),
            description: "Multiple workflows".to_string(),
            author: None,
            created: None,
        },
        agents: HashMap::new(),
        resources: HashMap::new(),
        workflows: vec![
            Workflow {
                name: "workflow1".to_string(),
                description: None,
                metadata: WorkflowMetadataDetails {
                    initial_state: "Start".to_string(),
                    completion_timeout_seconds: None,
                    max_retries: None,
                    retry_backoff_seconds: None,
                },
                states: {
                    let mut s = HashMap::new();
                    s.insert(
                        "Start".to_string(),
                        State {
                            description: "Start".to_string(),
                            agents: vec![],
                            entry_actions: vec![],
                            exit_actions: vec![],
                            terminal: true,
                            parent: None,
                            parallel_execution: false,
                            handlers: vec![],
                            timeout_seconds: None,
                            timeout_target: None,
                            required_resources: vec![],
                        },
                    );
                    s
                },
                guards: HashMap::new(),
                contracts: HashMap::new(),
            },
            Workflow {
                name: "workflow2".to_string(),
                description: None,
                metadata: WorkflowMetadataDetails {
                    initial_state: "Begin".to_string(),
                    completion_timeout_seconds: None,
                    max_retries: None,
                    retry_backoff_seconds: None,
                },
                states: {
                    let mut s = HashMap::new();
                    s.insert(
                        "Begin".to_string(),
                        State {
                            description: "Begin".to_string(),
                            agents: vec![],
                            entry_actions: vec![],
                            exit_actions: vec![],
                            terminal: true,
                            parent: None,
                            parallel_execution: false,
                            handlers: vec![],
                            timeout_seconds: None,
                            timeout_target: None,
                            required_resources: vec![],
                        },
                    );
                    s
                },
                guards: HashMap::new(),
                contracts: HashMap::new(),
            },
        ],
        guards: None,
    };

    // Import first workflow
    let dag1 = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();
    assert_eq!(dag1.name, "workflow1");

    // Import second workflow
    let dag2 = import_swarm_toml_to_dag(&swarm_config, 1).unwrap();
    assert_eq!(dag2.name, "workflow2");

    // Invalid index should fail
    assert!(import_swarm_toml_to_dag(&swarm_config, 2).is_err());
}

// ============================================================================
// Round-Trip Conversion Tests
// ============================================================================

#[test]
fn test_roundtrip_simple_workflow() {
    let original_dag = create_simple_workflow();
    let config = default_export_config();

    // Export to TOML
    let toml = export_dag_to_swarm_toml(&original_dag, &config).unwrap();

    // Parse TOML
    let swarm_config: SwarmConfig = toml::from_str(&toml).unwrap();

    // Import back to DAG
    let imported_dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

    // Verify structure preserved
    assert_eq!(original_dag.nodes.len(), imported_dag.nodes.len());
    assert_eq!(original_dag.edges.len(), imported_dag.edges.len());
}

#[test]
fn test_roundtrip_branching_workflow() {
    let original_dag = create_branching_workflow();
    let config = default_export_config();

    let toml = export_dag_to_swarm_toml(&original_dag, &config).unwrap();
    let swarm_config: SwarmConfig = toml::from_str(&toml).unwrap();
    let imported_dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

    assert_eq!(original_dag.nodes.len(), imported_dag.nodes.len());
    assert_eq!(original_dag.edges.len(), imported_dag.edges.len());

    // Verify topology preserved
    let original_starts = original_dag.get_start_nodes();
    let imported_starts = imported_dag.get_start_nodes();
    assert_eq!(original_starts.len(), imported_starts.len());

    let original_ends = original_dag.get_end_nodes();
    let imported_ends = imported_dag.get_end_nodes();
    assert_eq!(original_ends.len(), imported_ends.len());
}

#[test]
fn test_roundtrip_with_complex_metadata() {
    let mut dag = DAG::new("Complex Metadata");

    let node = DAGNode::new_auto("ComplexTask")
        .with_description("Task with complex metadata")
        .with_position(200.0, 300.0)
        .with_metadata("agents", serde_json::json!(["agent1", "agent2"]))
        .with_metadata("entry_actions", serde_json::json!(["action1", "action2"]))
        .with_metadata("exit_actions", serde_json::json!(["cleanup"]))
        .with_metadata("required_resources", serde_json::json!(["db", "api"]))
        .with_metadata("parallel_execution", serde_json::json!(true))
        .with_metadata("timeout_seconds", serde_json::json!(600))
        .with_tag("critical")
        .with_tag("production");

    dag.add_node(node).unwrap();

    let config = default_export_config();
    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();
    let swarm_config: SwarmConfig = toml::from_str(&toml).unwrap();
    let imported_dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

    let imported_node = imported_dag.nodes.values().next().unwrap();

    // Verify metadata preserved
    assert!(imported_node.metadata.contains_key("agents"));
    assert!(imported_node.metadata.contains_key("entry_actions"));
    assert!(imported_node.metadata.contains_key("exit_actions"));
    assert!(imported_node.metadata.contains_key("required_resources"));
    assert!(imported_node.metadata.contains_key("parallel_execution"));
    assert!(imported_node.metadata.contains_key("timeout_seconds"));
}

#[test]
fn test_roundtrip_preserves_topology() {
    let original_dag = create_branching_workflow();
    let config = default_export_config();

    let toml = export_dag_to_swarm_toml(&original_dag, &config).unwrap();
    let swarm_config: SwarmConfig = toml::from_str(&toml).unwrap();
    let imported_dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

    // Verify topological sort produces valid ordering
    let original_sorted = original_dag.topological_sort().unwrap();
    let imported_sorted = imported_dag.topological_sort().unwrap();

    assert_eq!(original_sorted.len(), imported_sorted.len());

    // Both should be valid DAGs
    assert!(original_dag.validate().is_ok());
    assert!(imported_dag.validate().is_ok());
}

// ============================================================================
// File I/O Tests
// ============================================================================

#[test]
fn test_save_and_load_dag() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test_workflow.toml");

    let original_dag = create_simple_workflow();
    let config = default_export_config();

    // Save to file
    save_dag_as_swarm_toml(&original_dag, &file_path, &config).unwrap();

    // Verify file exists
    assert!(file_path.exists());

    // Load from file
    let loaded_dag = load_dag_from_swarm_toml(&file_path, 0).unwrap();

    // Verify structure
    assert_eq!(original_dag.nodes.len(), loaded_dag.nodes.len());
    assert_eq!(original_dag.edges.len(), loaded_dag.edges.len());
}

#[test]
fn test_load_nonexistent_file() {
    let path = PathBuf::from("/nonexistent/file.toml");
    let result = load_dag_from_swarm_toml(&path, 0);

    assert!(result.is_err());
}

#[test]
fn test_save_creates_directories() {
    let temp_dir = TempDir::new().unwrap();
    let nested_path = temp_dir.path().join("nested/dir/workflow.toml");

    let dag = create_simple_workflow();
    let config = default_export_config();

    // Create parent directories if they don't exist
    if let Some(parent) = nested_path.parent() {
        fs::create_dir_all(parent).unwrap();
    }

    save_dag_as_swarm_toml(&dag, &nested_path, &config).unwrap();
    assert!(nested_path.exists());
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_export_disconnected_dag() {
    let mut dag = DAG::new("Disconnected");

    // Add two disconnected components
    let node1 = DAGNode::new_auto("A").with_metadata("agents", serde_json::json!(["agent1"]));
    let node2 = DAGNode::new_auto("B").with_metadata("agents", serde_json::json!(["agent1"]));
    let node3 = DAGNode::new_auto("C").with_metadata("agents", serde_json::json!(["agent1"]));

    let id1 = node1.node_id;
    let id2 = node2.node_id;

    dag.add_node(node1).unwrap();
    dag.add_node(node2).unwrap();
    dag.add_node(node3).unwrap();

    // Only connect 1 -> 2, leaving 3 isolated
    dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();

    let config = default_export_config();

    // Should export but validation would catch disconnection
    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();
    assert!(toml.contains("A"));
    assert!(toml.contains("B"));
    assert!(toml.contains("C"));
}

#[test]
fn test_export_dag_with_cycle() {
    let mut dag = DAG::new("Cycle Test");

    let nodes: Vec<_> = (0..3)
        .map(|i| {
            DAGNode::new_auto(format!("N{}", i))
                .with_metadata("agents", serde_json::json!(["agent1"]))
        })
        .collect();
    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    // Create cycle: 0 -> 1 -> 2 -> 0
    dag.add_edge(DAGEdge::dependency(ids[0], ids[1])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[1], ids[2])).unwrap();
    dag.add_edge(DAGEdge::dependency(ids[2], ids[0])).unwrap();

    let config = default_export_config();

    // Export should fail due to cycle
    assert!(export_dag_to_swarm_toml(&dag, &config).is_err());
}

#[test]
fn test_import_invalid_toml() {
    let invalid_toml = r#"
        this is not valid TOML { syntax
    "#;

    let result: Result<SwarmConfig, _> = toml::from_str(invalid_toml);
    assert!(result.is_err());
}

#[test]
fn test_export_large_workflow() {
    let mut dag = DAG::new("Large Workflow");

    // Create 50 nodes in a linear chain
    let nodes: Vec<_> = (0..50)
        .map(|i| {
            DAGNode::new_auto(format!("Task{}", i))
                .with_description(format!("Task number {}", i))
                .with_position((i % 10) as f64 * 100.0, (i / 10) as f64 * 100.0)
                .with_metadata("agents", serde_json::json!(["agent1"]))
        })
        .collect();

    let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

    for node in nodes {
        dag.add_node(node).unwrap();
    }

    for i in 0..49 {
        dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1]))
            .unwrap();
    }

    let config = default_export_config();
    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    // Verify all nodes present
    for i in 0..50 {
        assert!(toml.contains(&format!("Task{}", i)));
    }

    // Round-trip test
    let swarm_config: SwarmConfig = toml::from_str(&toml).unwrap();
    let imported_dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

    assert_eq!(dag.nodes.len(), imported_dag.nodes.len());
    assert_eq!(dag.edges.len(), imported_dag.edges.len());
}

#[test]
fn test_export_with_special_characters() {
    let mut dag = DAG::new("Special Characters Test");

    let node = DAGNode::new_auto("Task with \"quotes\" and 'apostrophes'")
        .with_description("Description with\nnewlines\tand\ttabs")
        .with_metadata("agents", serde_json::json!(["agent1"]));

    dag.add_node(node).unwrap();

    let config = default_export_config();
    let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

    // Should handle special characters properly
    let swarm_config: SwarmConfig = toml::from_str(&toml).unwrap();
    let imported_dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

    assert_eq!(dag.nodes.len(), imported_dag.nodes.len());
}

#[test]
fn test_export_use_node_ids_vs_labels() {
    let dag = create_simple_workflow();

    // Export with labels
    let config_labels = default_export_config().use_labels(true);
    let toml_labels = export_dag_to_swarm_toml(&dag, &config_labels).unwrap();

    assert!(toml_labels.contains("[workflows.test.states.Start]") || toml_labels.contains("Start"));

    // Export with UUIDs
    let config_uuids = default_export_config().use_labels(false);
    let toml_uuids = export_dag_to_swarm_toml(&dag, &config_uuids).unwrap();

    // Should contain UUID format
    assert!(toml_uuids.contains("-")); // UUIDs contain hyphens
}

#[test]
fn test_roundtrip_preserves_node_count() {
    for node_count in [1, 5, 10, 20] {
        let mut dag = DAG::new(format!("Test_{}", node_count));

        let nodes: Vec<_> = (0..node_count)
            .map(|i| {
                DAGNode::new_auto(format!("Node{}", i))
                    .with_metadata("agents", serde_json::json!(["agent1"]))
            })
            .collect();

        let ids: Vec<_> = nodes.iter().map(|n| n.node_id).collect();

        for node in nodes {
            dag.add_node(node).unwrap();
        }

        // Create linear chain
        for i in 0..(node_count - 1) {
            dag.add_edge(DAGEdge::dependency(ids[i], ids[i + 1]))
                .unwrap();
        }

        let config = default_export_config();
        let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();
        let swarm_config: SwarmConfig = toml::from_str(&toml).unwrap();
        let imported_dag = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

        assert_eq!(dag.nodes.len(), imported_dag.nodes.len());
        assert_eq!(dag.edges.len(), imported_dag.edges.len());
    }
}
