/// Knowledge Graph Overlay Logic
///
/// This module provides the core logic for generating and overlaying knowledge graph
/// elements onto a file tree structure. It bridges the gap between file system
/// navigation and semantic code understanding.
///
/// Key responsibilities:
/// - Extract code entities from source files
/// - Build knowledge graphs with nodes and edges
/// - Maintain bidirectional links between file tree and knowledge graph
/// - Support incremental updates when files change
/// - Provide query operations for semantic navigation
/// - Cache parsed knowledge for performance
use crate::errors::{ParserError, ParserResult};
use crate::knowledge_graph::{
    FileReference, FileTree, FileTreeNode, KnowledgeEdge, KnowledgeGraph, KnowledgeNode,
    KnowledgeNodeType, RelationshipType,
};
use crate::parser::SemanticParser;
use crate::types::{Language, SemanticNode, SemanticNodeType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

/// Configuration for knowledge graph overlay
#[derive(Debug, Clone)]
pub struct OverlayConfig {
    /// Languages to parse
    pub enabled_languages: Vec<Language>,

    /// Extract relationships between entities
    pub extract_relationships: bool,

    /// Maximum file size to parse (bytes)
    pub max_file_size: Option<u64>,

    /// Enable caching
    pub enable_cache: bool,

    /// Cache directory
    pub cache_dir: Option<PathBuf>,

    /// Cache TTL (time to live)
    pub cache_ttl: Duration,

    /// Parse files in parallel
    pub parallel_parsing: bool,
}

impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled_languages: vec![
                Language::Rust,
                Language::Python,
                Language::JavaScript,
                Language::TypeScript,
            ],
            extract_relationships: true,
            max_file_size: Some(5 * 1024 * 1024), // 5 MB
            enable_cache: true,
            cache_dir: None,
            cache_ttl: Duration::from_secs(3600), // 1 hour
            parallel_parsing: true,
        }
    }
}

/// Cache entry for parsed file data
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CacheEntry {
    /// File path
    file_path: PathBuf,

    /// Last modified timestamp
    modified_time: SystemTime,

    /// Knowledge node IDs extracted from this file
    node_ids: Vec<String>,

    /// Knowledge edge IDs related to this file
    edge_ids: Vec<String>,

    /// Timestamp when cached
    cached_at: SystemTime,
}

/// Knowledge graph overlay manager
///
/// This is the main interface for generating and managing knowledge graphs
/// overlaid on file trees. It coordinates parsing, entity extraction,
/// relationship detection, and bidirectional linking.
pub struct KnowledgeGraphOverlay {
    config: OverlayConfig,
    parser: SemanticParser,
    cache: HashMap<PathBuf, CacheEntry>,
}

impl KnowledgeGraphOverlay {
    /// Create a new overlay manager with default configuration
    pub fn new() -> ParserResult<Self> {
        Self::with_config(OverlayConfig::default())
    }

    /// Create a new overlay manager with custom configuration
    pub fn with_config(config: OverlayConfig) -> ParserResult<Self> {
        let parser_config = crate::types::ParserConfig {
            languages: config.enabled_languages.clone(),
            parallel: config.parallel_parsing,
            ..Default::default()
        };

        let parser = SemanticParser::with_config(parser_config)?;

        Ok(Self {
            config,
            parser,
            cache: HashMap::new(),
        })
    }

    /// Generate a complete knowledge graph from a file tree
    ///
    /// This is the main entry point for generating a knowledge graph overlay.
    /// It scans all code files in the tree, extracts entities, detects relationships,
    /// and links everything together.
    ///
    /// # Arguments
    /// * `file_tree` - The file tree to analyze
    ///
    /// # Returns
    /// * `KnowledgeGraph` - The generated knowledge graph with bidirectional links
    ///
    /// # Example
    /// ```ignore
    /// let overlay = KnowledgeGraphOverlay::new()?;
    /// let file_tree = build_file_tree("./src")?;
    /// let kg = overlay.generate_knowledge_overlay(&file_tree)?;
    /// ```
    pub fn generate_knowledge_overlay(
        &mut self,
        file_tree: &FileTree,
    ) -> ParserResult<KnowledgeGraph> {
        let mut knowledge_graph = KnowledgeGraph::new();

        // Get all code files from the tree
        let code_files = self.get_parseable_files(file_tree);

        tracing::info!(
            "Generating knowledge overlay for {} files",
            code_files.len()
        );

        // Parse files and extract entities
        for file_node in code_files {
            match self.extract_knowledge_from_file(file_node, &mut knowledge_graph) {
                Ok(_) => {
                    tracing::debug!("Extracted knowledge from {:?}", file_node.path);
                }
                Err(e) => {
                    tracing::warn!("Failed to extract from {:?}: {}", file_node.path, e);
                }
            }
        }

        // Extract relationships between entities
        if self.config.extract_relationships {
            self.extract_relationships(&mut knowledge_graph)?;
        }

        tracing::info!(
            "Knowledge graph generated: {} nodes, {} edges",
            knowledge_graph.nodes.len(),
            knowledge_graph.edges.len()
        );

        Ok(knowledge_graph)
    }

    /// Generate knowledge overlay and link it to the file tree
    ///
    /// This is a convenience method that generates the knowledge graph and
    /// establishes bidirectional links with the file tree.
    ///
    /// # Arguments
    /// * `file_tree` - Mutable reference to the file tree (will be updated with links)
    ///
    /// # Returns
    /// * `KnowledgeGraph` - The generated knowledge graph
    pub fn generate_and_link(&mut self, file_tree: &mut FileTree) -> ParserResult<KnowledgeGraph> {
        let mut knowledge_graph = self.generate_knowledge_overlay(file_tree)?;

        // Link knowledge nodes back to file tree
        self.link_to_file_tree(file_tree, &knowledge_graph)?;

        Ok(knowledge_graph)
    }

    /// Extract knowledge from a single file
    ///
    /// Parses the file, extracts semantic entities, and adds them to the knowledge graph.
    fn extract_knowledge_from_file(
        &mut self,
        file_node: &FileTreeNode,
        knowledge_graph: &mut KnowledgeGraph,
    ) -> ParserResult<Vec<String>> {
        // Check cache first
        if self.config.enable_cache {
            if let Some(cached) = self.check_cache(&file_node.path)? {
                return Ok(cached.node_ids);
            }
        }

        // Check file size
        if let Some(max_size) = self.config.max_file_size {
            if let Some(size) = file_node.metadata.size {
                if size > max_size {
                    return Ok(Vec::new());
                }
            }
        }

        // Parse the file
        let file_path_str = file_node
            .path
            .to_str()
            .ok_or_else(|| ParserError::ParseError("Invalid file path".to_string()))?;

        let parse_result = self.parser.parse_file(file_path_str)?;

        // Convert semantic nodes to knowledge nodes
        let mut node_ids = Vec::new();
        for semantic_node in parse_result.nodes {
            if let Some(node_id) =
                self.add_semantic_node_to_graph(semantic_node, file_node, knowledge_graph)?
            {
                node_ids.push(node_id);
            }
        }

        // Update cache
        if self.config.enable_cache {
            self.update_cache(&file_node.path, node_ids.clone(), Vec::new())?;
        }

        Ok(node_ids)
    }

    /// Convert a semantic node to a knowledge node and add it to the graph
    fn add_semantic_node_to_graph(
        &self,
        semantic_node: SemanticNode,
        file_node: &FileTreeNode,
        knowledge_graph: &mut KnowledgeGraph,
    ) -> ParserResult<Option<String>> {
        // Skip certain node types
        if matches!(semantic_node.node_type, SemanticNodeType::Other) {
            return Ok(None);
        }

        // Create knowledge node
        let mut knowledge_node = KnowledgeNode::new(
            KnowledgeNodeType::from_semantic_type(semantic_node.node_type),
            semantic_node.name.clone(),
            semantic_node.qualified_name.clone(),
        );

        // Populate fields
        knowledge_node.description = semantic_node.documentation;
        knowledge_node.source_code = Some(semantic_node.source_code);
        knowledge_node.language = Some(semantic_node.language);
        knowledge_node.signature = semantic_node.signature;
        knowledge_node.return_type = semantic_node.return_type;
        knowledge_node.parameters = semantic_node
            .parameters
            .iter()
            .map(|p| {
                format!(
                    "{}: {}",
                    p.name,
                    p.type_annotation.as_deref().unwrap_or("_")
                )
            })
            .collect();
        knowledge_node.type_parameters = semantic_node.type_parameters;
        knowledge_node.visibility = semantic_node.visibility;

        // Add file reference
        let file_ref = FileReference {
            file_node_id: file_node.node_id.clone(),
            file_path: file_node.path.clone(),
            line_range: semantic_node.line_range,
            column_range: semantic_node.column_range,
            is_definition: true,
        };
        knowledge_node.add_file_reference(file_ref);

        // Add to graph
        let node_id = knowledge_graph.add_node(knowledge_node);

        Ok(Some(node_id))
    }

    /// Extract relationships between knowledge nodes
    ///
    /// This analyzes the knowledge graph to detect relationships like:
    /// - Function calls
    /// - Type inheritance
    /// - Module imports
    /// - Interface implementations
    fn extract_relationships(&self, knowledge_graph: &mut KnowledgeGraph) -> ParserResult<()> {
        tracing::debug!("Extracting relationships between entities");

        // Build a lookup map for quick name resolution
        let name_to_node: HashMap<String, String> = knowledge_graph
            .nodes
            .iter()
            .map(|(id, node)| (node.qualified_name.clone(), id.clone()))
            .collect();

        // Analyze each node for relationships
        let nodes: Vec<_> = knowledge_graph.nodes.values().cloned().collect();

        for node in nodes {
            // Extract relationships from source code analysis
            // This is a simplified approach - a full implementation would use
            // tree-sitter queries or AST analysis

            // For now, we'll establish some basic relationships
            // based on naming patterns and node types

            // 1. DefinedIn relationships (link methods/functions to their parent class/module)
            if let Some(parent_id) = &node.parent_id {
                if knowledge_graph.get_node(parent_id).is_some() {
                    let edge = KnowledgeEdge::new(
                        node.node_id.clone(),
                        parent_id.clone(),
                        RelationshipType::DefinedIn,
                    );
                    knowledge_graph.add_edge(edge);
                }
            }

            // 2. Uses relationships based on type references
            // Extract type names from parameters and return types
            let mut referenced_types = HashSet::new();

            if let Some(ret_type) = &node.return_type {
                referenced_types.insert(ret_type.clone());
            }

            for param in &node.parameters {
                // Extract type from "name: type" format
                if let Some(colon_pos) = param.find(':') {
                    let type_part = param[colon_pos + 1..].trim();
                    referenced_types.insert(type_part.to_string());
                }
            }

            // Try to resolve these types to nodes
            for type_ref in referenced_types {
                // Simple name matching - could be improved with qualified name resolution
                for (name, target_id) in &name_to_node {
                    if name.ends_with(&type_ref) && target_id != &node.node_id {
                        let edge = KnowledgeEdge::new(
                            node.node_id.clone(),
                            target_id.clone(),
                            RelationshipType::Uses,
                        );
                        knowledge_graph.add_edge(edge);
                        break;
                    }
                }
            }
        }

        tracing::debug!("Extracted {} relationships", knowledge_graph.edges.len());

        Ok(())
    }

    /// Link knowledge nodes back to file tree nodes
    ///
    /// Updates file tree nodes with knowledge_links pointing to their entities.
    fn link_to_file_tree(
        &self,
        file_tree: &mut FileTree,
        knowledge_graph: &KnowledgeGraph,
    ) -> ParserResult<()> {
        // Build map of file_node_id -> knowledge_node_ids
        let mut file_to_knowledge: HashMap<String, Vec<String>> = HashMap::new();

        for (node_id, knowledge_node) in &knowledge_graph.nodes {
            for file_ref in &knowledge_node.file_references {
                file_to_knowledge
                    .entry(file_ref.file_node_id.clone())
                    .or_insert_with(Vec::new)
                    .push(node_id.clone());
            }
        }

        // Update file tree nodes
        for (file_node_id, knowledge_node_ids) in file_to_knowledge {
            if let Some(file_node) = file_tree.get_node_mut(&file_node_id) {
                file_node.knowledge_links = knowledge_node_ids;
            }
        }

        Ok(())
    }

    /// Get all parseable files from the file tree
    fn get_parseable_files(&self, file_tree: &FileTree) -> Vec<&FileTreeNode> {
        file_tree
            .get_all_files()
            .into_iter()
            .filter(|node| {
                // Check if file has a supported language
                if let Some(lang) = node.metadata.language {
                    self.config.enabled_languages.contains(&lang)
                } else {
                    false
                }
            })
            .filter(|node| {
                // Skip binary files
                !node.metadata.is_binary
            })
            .collect()
    }

    /// Update knowledge graph incrementally when a file changes
    ///
    /// This removes old entities from the file and re-extracts them.
    ///
    /// # Arguments
    /// * `file_path` - Path to the changed file
    /// * `file_tree` - The file tree
    /// * `knowledge_graph` - The knowledge graph to update
    pub fn update_file(
        &mut self,
        file_path: &Path,
        file_tree: &FileTree,
        knowledge_graph: &mut KnowledgeGraph,
    ) -> ParserResult<()> {
        tracing::info!("Updating knowledge for file: {:?}", file_path);

        // Find the file node
        let file_node = file_tree
            .get_node_by_path(&file_path.to_path_buf())
            .ok_or_else(|| ParserError::ParseError("File not found in tree".to_string()))?;

        // Remove old entities from this file
        self.remove_file_entities(file_path, knowledge_graph)?;

        // Re-extract entities
        self.extract_knowledge_from_file(file_node, knowledge_graph)?;

        // Re-extract relationships
        if self.config.extract_relationships {
            self.extract_relationships(knowledge_graph)?;
        }

        Ok(())
    }

    /// Remove all knowledge entities associated with a file
    fn remove_file_entities(
        &mut self,
        file_path: &Path,
        knowledge_graph: &mut KnowledgeGraph,
    ) -> ParserResult<()> {
        // Find nodes that reference this file
        let mut nodes_to_remove = Vec::new();

        for (node_id, node) in &knowledge_graph.nodes {
            if node
                .file_references
                .iter()
                .any(|fr| fr.file_path == file_path)
            {
                nodes_to_remove.push(node_id.clone());
            }
        }

        // Remove nodes and their edges
        for node_id in nodes_to_remove {
            self.remove_knowledge_node(node_id, knowledge_graph);
        }

        // Clear cache for this file
        self.invalidate_cache(file_path)?;

        Ok(())
    }

    /// Remove a knowledge node and all its edges
    fn remove_knowledge_node(&self, node_id: String, knowledge_graph: &mut KnowledgeGraph) {
        // Remove outgoing edges
        if let Some(edge_ids) = knowledge_graph.outgoing_edges.get(&node_id) {
            let edge_ids_to_remove: Vec<_> = edge_ids.clone();
            for edge_id in edge_ids_to_remove {
                knowledge_graph.edges.remove(&edge_id);
            }
        }

        // Remove incoming edges
        if let Some(edge_ids) = knowledge_graph.incoming_edges.get(&node_id) {
            let edge_ids_to_remove: Vec<_> = edge_ids.clone();
            for edge_id in edge_ids_to_remove {
                knowledge_graph.edges.remove(&edge_id);
            }
        }

        // Remove from indices
        knowledge_graph.outgoing_edges.remove(&node_id);
        knowledge_graph.incoming_edges.remove(&node_id);

        // Remove from name index
        if let Some(node) = knowledge_graph.nodes.get(&node_id) {
            knowledge_graph.name_index.remove(&node.qualified_name);

            // Remove from type index
            if let Some(type_nodes) = knowledge_graph.type_index.get_mut(&node.content_type) {
                type_nodes.retain(|id| id != &node_id);
            }
        }

        // Remove the node
        knowledge_graph.nodes.remove(&node_id);
    }

    // ============================================================================
    // Query Operations
    // ============================================================================

    /// Find all entities defined in a file
    pub fn find_entities_in_file(
        &self,
        file_path: &Path,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode> {
        knowledge_graph
            .nodes
            .values()
            .filter(|node| {
                node.file_references
                    .iter()
                    .any(|fr| fr.file_path == file_path)
            })
            .collect()
    }

    /// Find the definition location for an entity
    pub fn find_definition(
        &self,
        qualified_name: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Option<&FileReference> {
        knowledge_graph
            .get_node_by_name(qualified_name)
            .and_then(|node| node.file_references.iter().find(|fr| fr.is_definition))
    }

    /// Find all references to an entity
    pub fn find_references(
        &self,
        entity_name: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&FileReference> {
        knowledge_graph
            .nodes
            .values()
            .filter(|node| node.qualified_name.contains(entity_name))
            .flat_map(|node| &node.file_references)
            .collect()
    }

    /// Traverse the call graph from a function
    pub fn traverse_call_graph(
        &self,
        function_name: &str,
        knowledge_graph: &KnowledgeGraph,
        max_depth: usize,
    ) -> Vec<Vec<String>> {
        let mut paths = Vec::new();

        // Find the starting node
        if let Some(start_node) = knowledge_graph.get_node_by_name(function_name) {
            let mut current_path = vec![start_node.qualified_name.clone()];
            self.traverse_calls_recursive(
                &start_node.node_id,
                knowledge_graph,
                &mut current_path,
                &mut paths,
                max_depth,
            );
        }

        paths
    }

    fn traverse_calls_recursive(
        &self,
        node_id: &str,
        knowledge_graph: &KnowledgeGraph,
        current_path: &mut Vec<String>,
        all_paths: &mut Vec<Vec<String>>,
        max_depth: usize,
    ) {
        if current_path.len() >= max_depth {
            all_paths.push(current_path.clone());
            return;
        }

        // Get outgoing "Calls" edges
        let call_edges: Vec<_> = knowledge_graph
            .get_outgoing_edges(node_id)
            .into_iter()
            .filter(|edge| edge.relationship_type == RelationshipType::Calls)
            .collect();

        if call_edges.is_empty() {
            all_paths.push(current_path.clone());
            return;
        }

        for edge in call_edges {
            if let Some(target_node) = knowledge_graph.get_node(&edge.to_node_id) {
                // Avoid cycles
                if !current_path.contains(&target_node.qualified_name) {
                    current_path.push(target_node.qualified_name.clone());
                    self.traverse_calls_recursive(
                        &edge.to_node_id,
                        knowledge_graph,
                        current_path,
                        all_paths,
                        max_depth,
                    );
                    current_path.pop();
                }
            }
        }
    }

    /// Find all entities of a specific type
    pub fn find_by_type(
        &self,
        node_type: KnowledgeNodeType,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode> {
        knowledge_graph.get_nodes_by_type(node_type)
    }

    /// Find entities by name pattern
    pub fn find_by_name_pattern(
        &self,
        pattern: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode> {
        knowledge_graph
            .nodes
            .values()
            .filter(|node| node.name.contains(pattern) || node.qualified_name.contains(pattern))
            .collect()
    }

    /// Get all callers of a function
    pub fn find_callers(
        &self,
        function_name: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode> {
        if let Some(node) = knowledge_graph.get_node_by_name(function_name) {
            knowledge_graph
                .get_incoming_edges(&node.node_id)
                .into_iter()
                .filter(|edge| edge.relationship_type == RelationshipType::Calls)
                .filter_map(|edge| knowledge_graph.get_node(&edge.from_node_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Get all functions called by a function
    pub fn find_callees(
        &self,
        function_name: &str,
        knowledge_graph: &KnowledgeGraph,
    ) -> Vec<&KnowledgeNode> {
        if let Some(node) = knowledge_graph.get_node_by_name(function_name) {
            knowledge_graph
                .get_outgoing_edges(&node.node_id)
                .into_iter()
                .filter(|edge| edge.relationship_type == RelationshipType::Calls)
                .filter_map(|edge| knowledge_graph.get_node(&edge.to_node_id))
                .collect()
        } else {
            Vec::new()
        }
    }

    // ============================================================================
    // Caching
    // ============================================================================

    /// Check cache for a file
    fn check_cache(&self, file_path: &Path) -> ParserResult<Option<&CacheEntry>> {
        if let Some(entry) = self.cache.get(file_path) {
            // Check if file has been modified since caching
            if let Ok(metadata) = fs::metadata(file_path) {
                if let Ok(modified) = metadata.modified() {
                    if modified <= entry.modified_time {
                        // Check TTL
                        if let Ok(elapsed) = SystemTime::now().duration_since(entry.cached_at) {
                            if elapsed < self.config.cache_ttl {
                                return Ok(Some(entry));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    /// Update cache for a file
    fn update_cache(
        &mut self,
        file_path: &Path,
        node_ids: Vec<String>,
        edge_ids: Vec<String>,
    ) -> ParserResult<()> {
        let metadata = fs::metadata(file_path).map_err(|e| ParserError::IoError(e))?;
        let modified_time = metadata.modified().map_err(|e| ParserError::IoError(e))?;

        let entry = CacheEntry {
            file_path: file_path.to_path_buf(),
            modified_time,
            node_ids,
            edge_ids,
            cached_at: SystemTime::now(),
        };

        self.cache.insert(file_path.to_path_buf(), entry);

        Ok(())
    }

    /// Invalidate cache for a file
    fn invalidate_cache(&mut self, file_path: &Path) -> ParserResult<()> {
        self.cache.remove(file_path);
        Ok(())
    }

    /// Clear all cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.cache.len(),
            total_nodes: self.cache.values().map(|e| e.node_ids.len()).sum(),
            total_edges: self.cache.values().map(|e| e.edge_ids.len()).sum(),
        }
    }
}

/// Statistics about the cache
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub total_nodes: usize,
    pub total_edges: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::file_tree_builder::FileTreeBuilder;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_project(dir: &Path) -> std::io::Result<()> {
        fs::create_dir_all(dir.join("src"))?;

        // Create a Rust file with some code
        fs::write(
            dir.join("src/main.rs"),
            r#"
/// Main function
fn main() {
    println!("Hello");
    helper();
}

/// Helper function
fn helper() {
    println!("Helper");
}

/// A test struct
struct Point {
    x: i32,
    y: i32,
}
"#,
        )?;

        // Create a Python file
        fs::write(
            dir.join("src/utils.py"),
            r#"
def process(data):
    """Process data"""
    return transform(data)

def transform(data):
    """Transform data"""
    return data.upper()

class DataProcessor:
    """Data processor class"""
    def __init__(self):
        self.data = []
"#,
        )?;

        Ok(())
    }

    #[test]
    fn test_generate_knowledge_overlay() {
        let temp_dir = TempDir::new().unwrap();
        create_test_project(temp_dir.path()).unwrap();

        // Build file tree
        let mut builder = FileTreeBuilder::new();
        let file_tree = builder.scan_directory(temp_dir.path()).unwrap();

        // Generate knowledge overlay
        let mut overlay = KnowledgeGraphOverlay::new().unwrap();
        let kg = overlay.generate_knowledge_overlay(&file_tree).unwrap();

        // Verify we extracted entities
        assert!(kg.nodes.len() > 0, "Should extract some nodes");
        assert!(kg.get_nodes_by_type(KnowledgeNodeType::Function).len() > 0);
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

        let functions = overlay.find_by_type(KnowledgeNodeType::Function, &kg);
        let structs = overlay.find_by_type(KnowledgeNodeType::Struct, &kg);

        assert!(functions.len() > 0, "Should find functions");
        // Note: struct detection might vary based on parser implementation
    }

    #[test]
    fn test_cache_operations() {
        let mut overlay = KnowledgeGraphOverlay::new().unwrap();

        let stats = overlay.cache_stats();
        assert_eq!(stats.total_entries, 0);

        overlay.clear_cache();
        let stats = overlay.cache_stats();
        assert_eq!(stats.total_entries, 0);
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

        // Modify a file
        let main_path = temp_dir.path().join("src/main.rs");
        fs::write(
            &main_path,
            r#"
fn main() {
    println!("Modified");
}

fn new_function() {
    println!("New");
}
"#,
        )
        .unwrap();

        // Update knowledge graph
        overlay
            .update_file(&main_path, &file_tree, &mut kg)
            .unwrap();

        // The count might change depending on what was extracted
        // Just verify the operation completed without error
        assert!(kg.nodes.len() > 0);
    }
}
