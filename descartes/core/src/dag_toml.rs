/// DAG TOML Serialization Support for Swarm.toml Format
///
/// This module provides serialization and deserialization of DAG structures
/// to/from TOML format compatible with Swarm.toml workflow definitions.
///
/// # TOML Format
///
/// The DAG can be serialized to TOML in two formats:
///
/// 1. Inline format (nodes and edges defined inline):
/// ```toml
/// [dag]
/// name = "My Workflow DAG"
/// description = "Task dependency graph"
///
/// [[dag.nodes]]
/// node_id = "uuid-here"
/// label = "Task 1"
/// position = { x = 100.0, y = 100.0 }
///
/// [[dag.edges]]
/// from = "uuid1"
/// to = "uuid2"
/// edge_type = "dependency"
/// ```
///
/// 2. Reference format (references to tasks defined elsewhere):
/// ```toml
/// [dag]
/// name = "My Workflow DAG"
///
/// [[dag.dependencies]]
/// task = "task1_id"
/// depends_on = ["task2_id", "task3_id"]
/// ```

use crate::dag::{DAG, DAGEdge, DAGError, DAGNode, DAGResult, EdgeType, Position};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// TOML-friendly representation of a DAG node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlDAGNode {
    pub node_id: String,
    pub task_id: Option<String>,
    pub label: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub position: Option<TomlPosition>,
    #[serde(default)]
    pub metadata: HashMap<String, toml::Value>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// TOML-friendly position
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlPosition {
    pub x: f64,
    pub y: f64,
}

impl From<Position> for TomlPosition {
    fn from(pos: Position) -> Self {
        TomlPosition { x: pos.x, y: pos.y }
    }
}

impl From<TomlPosition> for Position {
    fn from(pos: TomlPosition) -> Self {
        Position::new(pos.x, pos.y)
    }
}

/// TOML-friendly representation of a DAG edge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlDAGEdge {
    pub edge_id: Option<String>,
    pub from: String,
    pub to: String,
    #[serde(default)]
    pub edge_type: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub metadata: HashMap<String, toml::Value>,
}

/// TOML-friendly representation of task dependencies (simplified format)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlTaskDependency {
    pub task: String,
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub dependency_type: Option<String>,
}

/// TOML-friendly DAG structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TomlDAG {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub nodes: Vec<TomlDAGNode>,
    #[serde(default)]
    pub edges: Vec<TomlDAGEdge>,
    #[serde(default)]
    pub dependencies: Vec<TomlTaskDependency>,
    #[serde(default)]
    pub metadata: HashMap<String, toml::Value>,
}

impl TomlDAG {
    /// Convert to internal DAG representation
    pub fn to_dag(&self) -> DAGResult<DAG> {
        let mut dag = DAG::new(&self.name);
        dag.description = self.description.clone();

        // Convert nodes
        for toml_node in &self.nodes {
            let node_id = Uuid::parse_str(&toml_node.node_id)
                .map_err(|e| DAGError::SerializationError(format!("Invalid node UUID: {}", e)))?;

            let task_id = if let Some(ref task_id_str) = toml_node.task_id {
                Some(Uuid::parse_str(task_id_str).map_err(|e| {
                    DAGError::SerializationError(format!("Invalid task UUID: {}", e))
                })?)
            } else {
                None
            };

            let mut node = DAGNode::new(node_id, &toml_node.label);
            if let Some(task_id) = task_id {
                node.task_id = Some(task_id);
            }
            node.description = toml_node.description.clone();

            if let Some(ref pos) = toml_node.position {
                node.position = Position::new(pos.x, pos.y);
            }

            node.tags = toml_node.tags.clone();

            // Convert metadata
            for (key, value) in &toml_node.metadata {
                let json_value = toml_value_to_json(value)?;
                node.metadata.insert(key.clone(), json_value);
            }

            dag.add_node(node)?;
        }

        // Convert edges
        for toml_edge in &self.edges {
            let from_id = Uuid::parse_str(&toml_edge.from)
                .map_err(|e| DAGError::SerializationError(format!("Invalid from UUID: {}", e)))?;

            let to_id = Uuid::parse_str(&toml_edge.to)
                .map_err(|e| DAGError::SerializationError(format!("Invalid to UUID: {}", e)))?;

            let edge_type = parse_edge_type(&toml_edge.edge_type)?;

            let mut edge = DAGEdge::new(from_id, to_id, edge_type);

            if let Some(ref edge_id_str) = toml_edge.edge_id {
                edge.edge_id = Uuid::parse_str(edge_id_str).map_err(|e| {
                    DAGError::SerializationError(format!("Invalid edge UUID: {}", e))
                })?;
            }

            edge.label = toml_edge.label.clone();

            // Convert metadata
            for (key, value) in &toml_edge.metadata {
                let json_value = toml_value_to_json(value)?;
                edge.metadata.insert(key.clone(), json_value);
            }

            dag.add_edge(edge)?;
        }

        // Convert simplified dependencies format
        for dep in &self.dependencies {
            let task_id = Uuid::parse_str(&dep.task)
                .map_err(|e| DAGError::SerializationError(format!("Invalid task UUID: {}", e)))?;

            for depends_on_str in &dep.depends_on {
                let depends_on_id = Uuid::parse_str(depends_on_str).map_err(|e| {
                    DAGError::SerializationError(format!("Invalid dependency UUID: {}", e))
                })?;

                let edge_type = if let Some(ref type_str) = dep.dependency_type {
                    parse_edge_type(type_str)?
                } else {
                    EdgeType::Dependency
                };

                // Create edge from dependency to task (task depends on dependency)
                let edge = DAGEdge::new(depends_on_id, task_id, edge_type);
                dag.add_edge(edge)?;
            }
        }

        // Rebuild adjacency lists
        dag.rebuild_adjacency();

        Ok(dag)
    }

    /// Create from internal DAG representation
    pub fn from_dag(dag: &DAG) -> Self {
        let mut toml_nodes = Vec::new();
        let mut toml_edges = Vec::new();

        // Convert nodes
        for (_, node) in &dag.nodes {
            let toml_node = TomlDAGNode {
                node_id: node.node_id.to_string(),
                task_id: node.task_id.map(|id| id.to_string()),
                label: node.label.clone(),
                description: node.description.clone(),
                position: Some(TomlPosition::from(node.position)),
                metadata: node
                    .metadata
                    .iter()
                    .filter_map(|(k, v)| {
                        json_value_to_toml(v).ok().map(|toml_val| (k.clone(), toml_val))
                    })
                    .collect(),
                tags: node.tags.clone(),
            };
            toml_nodes.push(toml_node);
        }

        // Convert edges
        for (_, edge) in &dag.edges {
            let toml_edge = TomlDAGEdge {
                edge_id: Some(edge.edge_id.to_string()),
                from: edge.from_node_id.to_string(),
                to: edge.to_node_id.to_string(),
                edge_type: edge_type_to_string(&edge.edge_type),
                label: edge.label.clone(),
                metadata: edge
                    .metadata
                    .iter()
                    .filter_map(|(k, v)| {
                        json_value_to_toml(v).ok().map(|toml_val| (k.clone(), toml_val))
                    })
                    .collect(),
            };
            toml_edges.push(toml_edge);
        }

        TomlDAG {
            name: dag.name.clone(),
            description: dag.description.clone(),
            nodes: toml_nodes,
            edges: toml_edges,
            dependencies: Vec::new(), // Not used in this direction
            metadata: dag
                .metadata
                .iter()
                .filter_map(|(k, v)| {
                    json_value_to_toml(v).ok().map(|toml_val| (k.clone(), toml_val))
                })
                .collect(),
        }
    }

    /// Parse from TOML string
    pub fn from_toml_str(s: &str) -> DAGResult<Self> {
        toml::from_str(s)
            .map_err(|e| DAGError::DeserializationError(format!("TOML parse error: {}", e)))
    }

    /// Serialize to TOML string
    pub fn to_toml_string(&self) -> DAGResult<String> {
        toml::to_string_pretty(self)
            .map_err(|e| DAGError::SerializationError(format!("TOML serialization error: {}", e)))
    }
}

/// Parse edge type from string
fn parse_edge_type(s: &str) -> DAGResult<EdgeType> {
    match s.to_lowercase().as_str() {
        "dependency" | "" => Ok(EdgeType::Dependency),
        "soft_dependency" => Ok(EdgeType::SoftDependency),
        "optional_dependency" => Ok(EdgeType::OptionalDependency),
        "data_flow" => Ok(EdgeType::DataFlow),
        "trigger" => Ok(EdgeType::Trigger),
        other => Ok(EdgeType::Custom(other.to_string())),
    }
}

/// Convert edge type to string
fn edge_type_to_string(edge_type: &EdgeType) -> String {
    match edge_type {
        EdgeType::Dependency => "dependency".to_string(),
        EdgeType::SoftDependency => "soft_dependency".to_string(),
        EdgeType::OptionalDependency => "optional_dependency".to_string(),
        EdgeType::DataFlow => "data_flow".to_string(),
        EdgeType::Trigger => "trigger".to_string(),
        EdgeType::Custom(s) => s.clone(),
    }
}

/// Convert TOML value to JSON value
fn toml_value_to_json(value: &toml::Value) -> DAGResult<serde_json::Value> {
    match value {
        toml::Value::String(s) => Ok(serde_json::Value::String(s.clone())),
        toml::Value::Integer(i) => Ok(serde_json::Value::Number((*i).into())),
        toml::Value::Float(f) => {
            let num = serde_json::Number::from_f64(*f)
                .ok_or_else(|| DAGError::SerializationError("Invalid float value".to_string()))?;
            Ok(serde_json::Value::Number(num))
        }
        toml::Value::Boolean(b) => Ok(serde_json::Value::Bool(*b)),
        toml::Value::Array(arr) => {
            let json_arr: Result<Vec<_>, _> =
                arr.iter().map(toml_value_to_json).collect();
            Ok(serde_json::Value::Array(json_arr?))
        }
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                map.insert(k.clone(), toml_value_to_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        toml::Value::Datetime(dt) => Ok(serde_json::Value::String(dt.to_string())),
    }
}

/// Convert JSON value to TOML value
fn json_value_to_toml(value: &serde_json::Value) -> DAGResult<toml::Value> {
    match value {
        serde_json::Value::String(s) => Ok(toml::Value::String(s.clone())),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(toml::Value::Integer(i))
            } else if let Some(f) = n.as_f64() {
                Ok(toml::Value::Float(f))
            } else {
                Err(DAGError::SerializationError(
                    "Invalid number value".to_string(),
                ))
            }
        }
        serde_json::Value::Bool(b) => Ok(toml::Value::Boolean(*b)),
        serde_json::Value::Array(arr) => {
            let toml_arr: Result<Vec<_>, _> =
                arr.iter().map(json_value_to_toml).collect();
            Ok(toml::Value::Array(toml_arr?))
        }
        serde_json::Value::Object(obj) => {
            let mut table = toml::value::Table::new();
            for (k, v) in obj {
                table.insert(k.clone(), json_value_to_toml(v)?);
            }
            Ok(toml::Value::Table(table))
        }
        serde_json::Value::Null => Ok(toml::Value::String("null".to_string())),
    }
}

/// Helper to load DAG from TOML file
pub fn load_dag_from_toml(path: &std::path::Path) -> DAGResult<DAG> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| DAGError::DeserializationError(format!("Failed to read file: {}", e)))?;

    let toml_dag = TomlDAG::from_toml_str(&content)?;
    toml_dag.to_dag()
}

/// Helper to save DAG to TOML file
pub fn save_dag_to_toml(dag: &DAG, path: &std::path::Path) -> DAGResult<()> {
    let toml_dag = TomlDAG::from_dag(dag);
    let content = toml_dag.to_toml_string()?;

    std::fs::write(path, content)
        .map_err(|e| DAGError::SerializationError(format!("Failed to write file: {}", e)))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toml_serialization_roundtrip() {
        let mut dag = DAG::new("Test Workflow");
        dag.description = Some("A test workflow".to_string());

        let node1 = DAGNode::new_auto("Task 1")
            .with_position(100.0, 200.0)
            .with_metadata("priority", "high");

        let node2 = DAGNode::new_auto("Task 2")
            .with_position(300.0, 200.0)
            .with_tag("important");

        let id1 = node1.node_id;
        let id2 = node2.node_id;

        dag.add_node(node1).unwrap();
        dag.add_node(node2).unwrap();
        dag.add_edge(DAGEdge::dependency(id1, id2)).unwrap();

        // Convert to TOML
        let toml_dag = TomlDAG::from_dag(&dag);
        let toml_str = toml_dag.to_toml_string().unwrap();

        assert!(toml_str.contains("Test Workflow"));
        assert!(toml_str.contains("Task 1"));
        assert!(toml_str.contains("Task 2"));

        // Parse back
        let parsed_toml = TomlDAG::from_toml_str(&toml_str).unwrap();
        let parsed_dag = parsed_toml.to_dag().unwrap();

        assert_eq!(parsed_dag.name, dag.name);
        assert_eq!(parsed_dag.nodes.len(), dag.nodes.len());
        assert_eq!(parsed_dag.edges.len(), dag.edges.len());
    }

    #[test]
    fn test_simplified_dependencies_format() {
        let toml_str = r#"
name = "Simple Workflow"
description = "Test workflow"

[[dependencies]]
task = "550e8400-e29b-41d4-a716-446655440001"
depends_on = [
    "550e8400-e29b-41d4-a716-446655440002",
    "550e8400-e29b-41d4-a716-446655440003"
]
dependency_type = "dependency"
"#;

        let toml_dag = TomlDAG::from_toml_str(toml_str).unwrap();
        assert_eq!(toml_dag.dependencies.len(), 1);
        assert_eq!(toml_dag.dependencies[0].depends_on.len(), 2);
    }

    #[test]
    fn test_edge_type_conversion() {
        assert_eq!(edge_type_to_string(&EdgeType::Dependency), "dependency");
        assert_eq!(
            edge_type_to_string(&EdgeType::SoftDependency),
            "soft_dependency"
        );
        assert_eq!(
            edge_type_to_string(&EdgeType::Custom("custom".to_string())),
            "custom"
        );

        assert!(matches!(
            parse_edge_type("dependency").unwrap(),
            EdgeType::Dependency
        ));
        assert!(matches!(
            parse_edge_type("soft_dependency").unwrap(),
            EdgeType::SoftDependency
        ));
    }

    #[test]
    fn test_position_conversion() {
        let pos = Position::new(100.0, 200.0);
        let toml_pos = TomlPosition::from(pos);
        assert_eq!(toml_pos.x, 100.0);
        assert_eq!(toml_pos.y, 200.0);

        let back = Position::from(toml_pos);
        assert_eq!(back.x, 100.0);
        assert_eq!(back.y, 200.0);
    }
}
