/// DAG to Swarm.toml Export/Import Module
///
/// This module provides comprehensive conversion between DAG structures and Swarm.toml
/// workflow configurations, enabling visual DAG editors to generate executable workflows.
///
/// # Features
///
/// - Export DAG to Swarm.toml format with full workflow configuration
/// - Import Swarm.toml workflows back to DAG representation
/// - Automatic mapping of nodes to states and edges to event handlers
/// - Preservation of metadata, agents, and resources
/// - Validation of workflow constraints
/// - File I/O helpers for seamless integration
///
/// # Examples
///
/// ```rust,no_run
/// use descartes_core::dag::DAG;
/// use descartes_core::dag_swarm_export::{export_dag_to_swarm_toml, SwarmExportConfig};
///
/// let dag = DAG::new("My Workflow");
/// // ... add nodes and edges ...
///
/// let config = SwarmExportConfig::default()
///     .with_workflow_name("my_workflow")
///     .with_agent("default_agent", "claude-3-sonnet");
///
/// let swarm_toml = export_dag_to_swarm_toml(&dag, &config).unwrap();
/// println!("{}", swarm_toml);
/// ```
use crate::dag::{DAGEdge, DAGError, DAGNode, DAGResult, EdgeType, DAG};
use crate::swarm_parser::{
    AgentConfig, Handler, ResourceConfig, State, SwarmConfig, Workflow, WorkflowMetadata,
    WorkflowMetadataDetails,
};
use std::collections::HashMap;
use std::path::Path;
use uuid::Uuid;

/// Configuration for DAG to Swarm.toml export
#[derive(Debug, Clone)]
pub struct SwarmExportConfig {
    /// Workflow name (defaults to DAG name)
    pub workflow_name: Option<String>,

    /// Workflow description
    pub workflow_description: Option<String>,

    /// Initial state name (defaults to first root node)
    pub initial_state: Option<String>,

    /// Agents configuration (name -> AgentConfig)
    pub agents: HashMap<String, AgentConfig>,

    /// Resources configuration (name -> ResourceConfig)
    pub resources: HashMap<String, ResourceConfig>,

    /// Global guards (name -> expression)
    pub guards: HashMap<String, String>,

    /// Metadata author
    pub author: Option<String>,

    /// Completion timeout in seconds
    pub completion_timeout_seconds: Option<u64>,

    /// Maximum retries
    pub max_retries: Option<u32>,

    /// Retry backoff in seconds
    pub retry_backoff_seconds: Option<u64>,

    /// Whether to use node labels as state names (vs node_id)
    pub use_labels_as_state_names: bool,

    /// Default event name for dependencies
    pub default_event_name: String,

    /// Include header comment in output
    pub include_header: bool,
}

impl Default for SwarmExportConfig {
    fn default() -> Self {
        SwarmExportConfig {
            workflow_name: None,
            workflow_description: None,
            initial_state: None,
            agents: HashMap::new(),
            resources: HashMap::new(),
            guards: HashMap::new(),
            author: Some("Descartes DAG Export".to_string()),
            completion_timeout_seconds: Some(3600),
            max_retries: Some(3),
            retry_backoff_seconds: Some(60),
            use_labels_as_state_names: true,
            default_event_name: "next".to_string(),
            include_header: true,
        }
    }
}

impl SwarmExportConfig {
    /// Create a new export configuration
    pub fn new() -> Self {
        Self::default()
    }

    /// Set workflow name
    pub fn with_workflow_name(mut self, name: impl Into<String>) -> Self {
        self.workflow_name = Some(name.into());
        self
    }

    /// Set workflow description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.workflow_description = Some(description.into());
        self
    }

    /// Add an agent configuration
    pub fn with_agent(mut self, name: impl Into<String>, model: impl Into<String>) -> Self {
        let agent = AgentConfig {
            model: model.into(),
            max_tokens: Some(4000),
            temperature: Some(0.7),
            tags: vec![],
        };
        self.agents.insert(name.into(), agent);
        self
    }

    /// Add a detailed agent configuration
    pub fn with_agent_config(mut self, name: impl Into<String>, config: AgentConfig) -> Self {
        self.agents.insert(name.into(), config);
        self
    }

    /// Add a resource configuration
    pub fn with_resource(mut self, name: impl Into<String>, resource: ResourceConfig) -> Self {
        self.resources.insert(name.into(), resource);
        self
    }

    /// Add a guard expression
    pub fn with_guard(mut self, name: impl Into<String>, expression: impl Into<String>) -> Self {
        self.guards.insert(name.into(), expression.into());
        self
    }

    /// Set initial state
    pub fn with_initial_state(mut self, state: impl Into<String>) -> Self {
        self.initial_state = Some(state.into());
        self
    }

    /// Set author
    pub fn with_author(mut self, author: impl Into<String>) -> Self {
        self.author = Some(author.into());
        self
    }

    /// Set timeout configuration
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.completion_timeout_seconds = Some(timeout_seconds);
        self
    }

    /// Set retry configuration
    pub fn with_retries(mut self, max_retries: u32, backoff_seconds: u64) -> Self {
        self.max_retries = Some(max_retries);
        self.retry_backoff_seconds = Some(backoff_seconds);
        self
    }

    /// Use node labels as state names instead of IDs
    pub fn use_labels(mut self, use_labels: bool) -> Self {
        self.use_labels_as_state_names = use_labels;
        self
    }
}

/// Export a DAG to Swarm.toml format
pub fn export_dag_to_swarm_toml(dag: &DAG, config: &SwarmExportConfig) -> DAGResult<String> {
    // Validate DAG
    dag.validate()?;

    // Build the workflow
    let workflow = build_workflow_from_dag(dag, config)?;

    // Build the SwarmConfig
    let swarm_config = SwarmConfig {
        metadata: WorkflowMetadata {
            version: "1.0".to_string(),
            name: config
                .workflow_name
                .clone()
                .or_else(|| Some(dag.name.clone()))
                .unwrap_or_else(|| "Exported Workflow".to_string()),
            description: config
                .workflow_description
                .clone()
                .or_else(|| dag.description.clone())
                .unwrap_or_else(|| "Generated from DAG".to_string()),
            author: config.author.clone(),
            created: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
        },
        agents: config.agents.clone(),
        resources: config.resources.clone(),
        workflows: vec![workflow],
        guards: if config.guards.is_empty() {
            None
        } else {
            Some(config.guards.clone())
        },
    };

    // Serialize to TOML
    let toml_string = toml::to_string_pretty(&swarm_config)
        .map_err(|e| DAGError::SerializationError(format!("TOML serialization failed: {}", e)))?;

    // Add header if requested
    if config.include_header {
        let header = format!(
            "# Swarm.toml - Generated from DAG '{}'\n\
             # Generated: {}\n\
             # This file was automatically generated from a visual DAG representation.\n\
             # Edit with caution as manual changes may be lost on regeneration.\n\n",
            dag.name,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        Ok(header + &toml_string)
    } else {
        Ok(toml_string)
    }
}

/// Build a workflow from a DAG
fn build_workflow_from_dag(dag: &DAG, config: &SwarmExportConfig) -> DAGResult<Workflow> {
    let mut states = HashMap::new();

    // Determine initial state
    let roots = dag.get_start_nodes();
    if roots.is_empty() {
        return Err(DAGError::ValidationError(
            "DAG has no start nodes (roots)".to_string(),
        ));
    }

    let initial_state = if let Some(ref init) = config.initial_state {
        init.clone()
    } else {
        // Use first root node
        let root_id = roots[0];
        let root_node = dag
            .get_node(root_id)
            .ok_or(DAGError::NodeNotFound(root_id))?;
        get_state_name(root_node, config)
    };

    // Convert each node to a state
    for node in dag.nodes.values() {
        let state = build_state_from_node(node, dag, config)?;
        let state_name = get_state_name(node, config);
        states.insert(state_name, state);
    }

    // Build workflow
    Ok(Workflow {
        name: config
            .workflow_name
            .clone()
            .unwrap_or_else(|| dag.name.clone()),
        description: config
            .workflow_description
            .clone()
            .or_else(|| dag.description.clone()),
        metadata: WorkflowMetadataDetails {
            initial_state,
            completion_timeout_seconds: config.completion_timeout_seconds,
            max_retries: config.max_retries,
            retry_backoff_seconds: config.retry_backoff_seconds,
        },
        states,
        guards: config.guards.clone(),
        contracts: HashMap::new(),
    })
}

/// Build a state from a DAG node
fn build_state_from_node(
    node: &DAGNode,
    dag: &DAG,
    config: &SwarmExportConfig,
) -> DAGResult<State> {
    // Determine if terminal (no outgoing edges)
    let is_terminal = dag.get_outgoing_edges(node.node_id).is_empty();

    // Build handlers from outgoing edges
    let mut handlers = Vec::new();
    if !is_terminal {
        for edge in dag.get_outgoing_edges(node.node_id) {
            let target_node = dag
                .get_node(edge.to_node_id)
                .ok_or(DAGError::NodeNotFound(edge.to_node_id))?;

            let event_name = get_event_name(edge, config);
            let target_state = get_state_name(target_node, config);
            let guards = extract_guards_from_edge(edge);

            handlers.push(Handler {
                event: event_name,
                target: target_state,
                guards,
            });
        }
    }

    // Extract agents from metadata
    let agents = extract_agents_from_node(node, config);

    // Extract actions from metadata
    let entry_actions = extract_entry_actions(node);
    let exit_actions = extract_exit_actions(node);

    // Extract resources
    let required_resources = extract_resources(node);

    // Extract timeout configuration
    let (timeout_seconds, timeout_target) = extract_timeout_config(node, dag, config);

    // Extract parent state
    let parent = extract_parent_state(node);

    // Extract parallel execution flag
    let parallel_execution = extract_parallel_execution(node);

    Ok(State {
        description: node
            .description
            .clone()
            .unwrap_or_else(|| format!("State: {}", node.label)),
        agents,
        entry_actions,
        exit_actions,
        terminal: is_terminal,
        parent,
        parallel_execution,
        handlers,
        timeout_seconds,
        timeout_target,
        required_resources,
    })
}

/// Get state name from node
fn get_state_name(node: &DAGNode, config: &SwarmExportConfig) -> String {
    if config.use_labels_as_state_names {
        sanitize_state_name(&node.label)
    } else {
        node.node_id.to_string()
    }
}

/// Sanitize state name (replace spaces, special chars)
fn sanitize_state_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

/// Get event name from edge
fn get_event_name(edge: &DAGEdge, config: &SwarmExportConfig) -> String {
    // Check if edge has a label
    if let Some(ref label) = edge.label {
        sanitize_state_name(label)
    } else if let Some(event) = edge.metadata.get("event") {
        // Check metadata for event name
        event
            .as_str()
            .unwrap_or(&config.default_event_name)
            .to_string()
    } else {
        // Use edge type as event name
        match edge.edge_type {
            EdgeType::Dependency => config.default_event_name.clone(),
            EdgeType::SoftDependency => "soft_next".to_string(),
            EdgeType::OptionalDependency => "optional".to_string(),
            EdgeType::DataFlow => "data_ready".to_string(),
            EdgeType::Trigger => "trigger".to_string(),
            EdgeType::Custom(ref name) => sanitize_state_name(name),
        }
    }
}

/// Extract guards from edge metadata
fn extract_guards_from_edge(edge: &DAGEdge) -> Vec<String> {
    if let Some(guards_value) = edge.metadata.get("guards") {
        if let Some(guards_array) = guards_value.as_array() {
            return guards_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        } else if let Some(guard_str) = guards_value.as_str() {
            return vec![guard_str.to_string()];
        }
    }
    Vec::new()
}

/// Extract agents from node metadata
fn extract_agents_from_node(node: &DAGNode, config: &SwarmExportConfig) -> Vec<String> {
    if let Some(agents_value) = node.metadata.get("agents") {
        if let Some(agents_array) = agents_value.as_array() {
            return agents_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        } else if let Some(agent_str) = agents_value.as_str() {
            return vec![agent_str.to_string()];
        }
    }

    // If no agents specified, check if there's a default agent in config
    if !config.agents.is_empty() {
        // Use first agent as default
        if let Some(first_agent) = config.agents.keys().next() {
            return vec![first_agent.clone()];
        }
    }

    Vec::new()
}

/// Extract entry actions from node metadata
fn extract_entry_actions(node: &DAGNode) -> Vec<String> {
    if let Some(actions) = node.metadata.get("entry_actions") {
        if let Some(actions_array) = actions.as_array() {
            return actions_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        } else if let Some(action_str) = actions.as_str() {
            return vec![action_str.to_string()];
        }
    }
    Vec::new()
}

/// Extract exit actions from node metadata
fn extract_exit_actions(node: &DAGNode) -> Vec<String> {
    if let Some(actions) = node.metadata.get("exit_actions") {
        if let Some(actions_array) = actions.as_array() {
            return actions_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        } else if let Some(action_str) = actions.as_str() {
            return vec![action_str.to_string()];
        }
    }
    Vec::new()
}

/// Extract required resources from node metadata
fn extract_resources(node: &DAGNode) -> Vec<String> {
    if let Some(resources) = node.metadata.get("required_resources") {
        if let Some(resources_array) = resources.as_array() {
            return resources_array
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
        } else if let Some(resource_str) = resources.as_str() {
            return vec![resource_str.to_string()];
        }
    }
    Vec::new()
}

/// Extract timeout configuration from node metadata
fn extract_timeout_config(
    node: &DAGNode,
    dag: &DAG,
    config: &SwarmExportConfig,
) -> (Option<u64>, Option<String>) {
    let timeout_seconds = node
        .metadata
        .get("timeout_seconds")
        .and_then(|v| v.as_u64());

    let timeout_target = node
        .metadata
        .get("timeout_target")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .or_else(|| {
            // Try to find timeout target by UUID
            node.metadata
                .get("timeout_target_id")
                .and_then(|v| v.as_str())
                .and_then(|id_str| Uuid::parse_str(id_str).ok())
                .and_then(|uuid| dag.get_node(uuid))
                .map(|target_node| get_state_name(target_node, config))
        });

    (timeout_seconds, timeout_target)
}

/// Extract parent state from node metadata
fn extract_parent_state(node: &DAGNode) -> Option<String> {
    node.metadata
        .get("parent")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Extract parallel execution flag from node metadata
fn extract_parallel_execution(node: &DAGNode) -> bool {
    node.metadata
        .get("parallel_execution")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Import a Swarm.toml workflow to DAG representation
pub fn import_swarm_toml_to_dag(
    swarm_config: &SwarmConfig,
    workflow_index: usize,
) -> DAGResult<DAG> {
    if workflow_index >= swarm_config.workflows.len() {
        return Err(DAGError::DeserializationError(format!(
            "Workflow index {} out of bounds (total: {})",
            workflow_index,
            swarm_config.workflows.len()
        )));
    }

    let workflow = &swarm_config.workflows[workflow_index];
    let mut dag = DAG::new(&workflow.name);
    dag.description = workflow.description.clone();

    // Map state names to node IDs
    let mut state_to_node: HashMap<String, Uuid> = HashMap::new();

    // Create nodes from states
    for (state_name, state) in &workflow.states {
        let node_id = Uuid::new_v4();
        let mut node = DAGNode::new(node_id, state_name);
        node.description = Some(state.description.clone());

        // Store metadata
        if !state.agents.is_empty() {
            node.metadata.insert(
                "agents".to_string(),
                serde_json::Value::Array(
                    state
                        .agents
                        .iter()
                        .map(|a| serde_json::Value::String(a.clone()))
                        .collect(),
                ),
            );
        }

        if !state.entry_actions.is_empty() {
            node.metadata.insert(
                "entry_actions".to_string(),
                serde_json::Value::Array(
                    state
                        .entry_actions
                        .iter()
                        .map(|a| serde_json::Value::String(a.clone()))
                        .collect(),
                ),
            );
        }

        if !state.exit_actions.is_empty() {
            node.metadata.insert(
                "exit_actions".to_string(),
                serde_json::Value::Array(
                    state
                        .exit_actions
                        .iter()
                        .map(|a| serde_json::Value::String(a.clone()))
                        .collect(),
                ),
            );
        }

        if !state.required_resources.is_empty() {
            node.metadata.insert(
                "required_resources".to_string(),
                serde_json::Value::Array(
                    state
                        .required_resources
                        .iter()
                        .map(|r| serde_json::Value::String(r.clone()))
                        .collect(),
                ),
            );
        }

        if let Some(parent) = &state.parent {
            node.metadata.insert(
                "parent".to_string(),
                serde_json::Value::String(parent.clone()),
            );
        }

        if state.parallel_execution {
            node.metadata.insert(
                "parallel_execution".to_string(),
                serde_json::Value::Bool(true),
            );
        }

        if let Some(timeout) = state.timeout_seconds {
            node.metadata.insert(
                "timeout_seconds".to_string(),
                serde_json::Value::Number(timeout.into()),
            );
        }

        if let Some(timeout_target) = &state.timeout_target {
            node.metadata.insert(
                "timeout_target".to_string(),
                serde_json::Value::String(timeout_target.clone()),
            );
        }

        node.metadata.insert(
            "terminal".to_string(),
            serde_json::Value::Bool(state.terminal),
        );

        state_to_node.insert(state_name.clone(), node_id);
        dag.add_node(node)?;
    }

    // Create edges from handlers
    for (state_name, state) in &workflow.states {
        let from_node_id = *state_to_node.get(state_name).ok_or_else(|| {
            DAGError::DeserializationError(format!("State '{}' not found in mapping", state_name))
        })?;

        for handler in &state.handlers {
            let to_node_id = *state_to_node.get(&handler.target).ok_or_else(|| {
                DAGError::DeserializationError(format!(
                    "Target state '{}' not found in mapping",
                    handler.target
                ))
            })?;

            let mut edge = DAGEdge::dependency(from_node_id, to_node_id);
            edge.label = Some(handler.event.clone());

            // Store guards in metadata
            if !handler.guards.is_empty() {
                edge.metadata.insert(
                    "guards".to_string(),
                    serde_json::Value::Array(
                        handler
                            .guards
                            .iter()
                            .map(|g| serde_json::Value::String(g.clone()))
                            .collect(),
                    ),
                );
            }

            // Store event name in metadata
            edge.metadata.insert(
                "event".to_string(),
                serde_json::Value::String(handler.event.clone()),
            );

            dag.add_edge(edge)?;
        }
    }

    dag.rebuild_adjacency();
    Ok(dag)
}

/// Save a DAG as Swarm.toml file
pub fn save_dag_as_swarm_toml(dag: &DAG, path: &Path, config: &SwarmExportConfig) -> DAGResult<()> {
    let toml_content = export_dag_to_swarm_toml(dag, config)?;
    std::fs::write(path, toml_content)
        .map_err(|e| DAGError::SerializationError(format!("Failed to write file: {}", e)))?;
    Ok(())
}

/// Load a Swarm.toml file and convert to DAG
pub fn load_dag_from_swarm_toml(path: &Path, workflow_index: usize) -> DAGResult<DAG> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| DAGError::DeserializationError(format!("Failed to read file: {}", e)))?;

    let swarm_config: SwarmConfig = toml::from_str(&content)
        .map_err(|e| DAGError::DeserializationError(format!("TOML parse error: {}", e)))?;

    import_swarm_toml_to_dag(&swarm_config, workflow_index)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dag::EdgeType;

    #[test]
    fn test_basic_dag_to_swarm_export() {
        let mut dag = DAG::new("Test Workflow");
        dag.description = Some("A test workflow for export".to_string());

        // Create simple workflow: Start -> Processing -> Complete
        let start = DAGNode::new_auto("Start")
            .with_description("Initial state")
            .with_metadata("agents", serde_json::json!(["agent1"]));

        let processing = DAGNode::new_auto("Processing")
            .with_description("Processing state")
            .with_metadata("agents", serde_json::json!(["agent2"]))
            .with_metadata("entry_actions", serde_json::json!(["process_data"]));

        let complete = DAGNode::new_auto("Complete").with_description("Terminal state");

        let start_id = start.node_id;
        let processing_id = processing.node_id;
        let complete_id = complete.node_id;

        dag.add_node(start).unwrap();
        dag.add_node(processing).unwrap();
        dag.add_node(complete).unwrap();

        let mut edge1 = DAGEdge::dependency(start_id, processing_id);
        edge1.label = Some("start_processing".to_string());
        dag.add_edge(edge1).unwrap();

        let mut edge2 = DAGEdge::dependency(processing_id, complete_id);
        edge2.label = Some("finish".to_string());
        dag.add_edge(edge2).unwrap();

        let config = SwarmExportConfig::default()
            .with_agent("agent1", "claude-3-opus")
            .with_agent("agent2", "claude-3-sonnet");

        let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

        assert!(toml.contains("Test Workflow"));
        assert!(toml.contains("Start"));
        assert!(toml.contains("Processing"));
        assert!(toml.contains("Complete"));
        assert!(toml.contains("start_processing"));
        assert!(toml.contains("agent1"));
        assert!(toml.contains("agent2"));
    }

    #[test]
    fn test_swarm_to_dag_import() {
        // Create a Swarm config
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
    }

    #[test]
    fn test_roundtrip_conversion() {
        // Create DAG
        let mut dag = DAG::new("Roundtrip Test");

        let node1 = DAGNode::new_auto("NodeA").with_description("First node");
        let node2 = DAGNode::new_auto("NodeB").with_description("Second node");

        let id1 = node1.node_id;
        let id2 = node2.node_id;

        dag.add_node(node1).unwrap();
        dag.add_node(node2).unwrap();
        dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();

        // Export to Swarm
        let config = SwarmExportConfig::default().with_agent("default", "claude-3-opus");

        let toml = export_dag_to_swarm_toml(&dag, &config).unwrap();

        // Parse back
        let swarm_config: SwarmConfig = toml::from_str(&toml).unwrap();

        // Import back to DAG
        let dag2 = import_swarm_toml_to_dag(&swarm_config, 0).unwrap();

        assert_eq!(dag.nodes.len(), dag2.nodes.len());
        assert_eq!(dag.edges.len(), dag2.edges.len());
    }

    #[test]
    fn test_state_name_sanitization() {
        assert_eq!(sanitize_state_name("Hello World"), "Hello_World");
        assert_eq!(sanitize_state_name("Test-State"), "Test_State");
        assert_eq!(sanitize_state_name("State@123"), "State_123");
        assert_eq!(sanitize_state_name("Valid_State"), "Valid_State");
    }

    #[test]
    fn test_complex_metadata_extraction() {
        let mut node = DAGNode::new_auto("Test");

        node.metadata.insert(
            "agents".to_string(),
            serde_json::json!(["agent1", "agent2"]),
        );
        node.metadata.insert(
            "entry_actions".to_string(),
            serde_json::json!(["action1", "action2"]),
        );
        node.metadata
            .insert("parallel_execution".to_string(), serde_json::json!(true));

        let config = SwarmExportConfig::default();

        let agents = extract_agents_from_node(&node, &config);
        assert_eq!(agents, vec!["agent1", "agent2"]);

        let actions = extract_entry_actions(&node);
        assert_eq!(actions, vec!["action1", "action2"]);

        let parallel = extract_parallel_execution(&node);
        assert!(parallel);
    }
}
