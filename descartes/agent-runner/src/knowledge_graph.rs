/// Knowledge Graph and File Tree Data Models
///
/// This module provides data structures for representing:
/// - File system hierarchies (FileTreeNode, FileTree)
/// - Code knowledge graphs (KnowledgeNode, KnowledgeEdge, KnowledgeGraph)
/// - Bidirectional linking between file system and knowledge structures
///
/// These models enable:
/// - Efficient navigation of project structure
/// - Semantic code understanding and querying
/// - Relationship tracking between code entities
/// - Integration with RAG and semantic parsing systems

use crate::types::{Language, SemanticNodeType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use uuid::Uuid;

/// ============================================================================
/// File Tree Models
/// ============================================================================

/// Type of file system node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FileNodeType {
    /// Regular file
    File,
    /// Directory
    Directory,
    /// Symbolic link
    Symlink,
}

impl FileNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FileNodeType::File => "file",
            FileNodeType::Directory => "directory",
            FileNodeType::Symlink => "symlink",
        }
    }
}

/// Metadata associated with a file tree node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    /// File size in bytes
    pub size: Option<u64>,

    /// Last modified timestamp (Unix epoch)
    pub modified: Option<i64>,

    /// Created timestamp (Unix epoch)
    pub created: Option<i64>,

    /// File permissions (Unix mode)
    pub permissions: Option<u32>,

    /// MIME type or detected file type
    pub mime_type: Option<String>,

    /// Programming language (if code file)
    pub language: Option<Language>,

    /// Whether the file is binary
    pub is_binary: bool,

    /// Number of lines (for text files)
    pub line_count: Option<usize>,

    /// Git status (modified, added, deleted, etc.)
    pub git_status: Option<String>,

    /// Additional custom metadata
    pub custom: HashMap<String, String>,
}

impl Default for FileMetadata {
    fn default() -> Self {
        Self {
            size: None,
            modified: None,
            created: None,
            permissions: None,
            mime_type: None,
            language: None,
            is_binary: false,
            line_count: None,
            git_status: None,
            custom: HashMap::new(),
        }
    }
}

/// A node in the file tree representing a file or directory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeNode {
    /// Unique identifier for this node
    pub node_id: String,

    /// Full path to the file/directory
    pub path: PathBuf,

    /// Name of the file/directory (without path)
    pub name: String,

    /// Type of node (file, directory, symlink)
    pub node_type: FileNodeType,

    /// Parent node ID (None for root)
    pub parent_id: Option<String>,

    /// Child node IDs (for directories)
    pub children: Vec<String>,

    /// File metadata
    pub metadata: FileMetadata,

    /// References to knowledge graph nodes found in this file
    pub knowledge_links: Vec<String>,

    /// Whether this node has been indexed for search
    pub indexed: bool,

    /// Depth in the tree (0 for root)
    pub depth: usize,
}

impl FileTreeNode {
    /// Create a new file tree node
    pub fn new(path: PathBuf, node_type: FileNodeType, parent_id: Option<String>, depth: usize) -> Self {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        Self {
            node_id: Uuid::new_v4().to_string(),
            path,
            name,
            node_type,
            parent_id,
            children: Vec::new(),
            metadata: FileMetadata::default(),
            knowledge_links: Vec::new(),
            indexed: false,
            depth,
        }
    }

    /// Check if this is a directory
    pub fn is_directory(&self) -> bool {
        self.node_type == FileNodeType::Directory
    }

    /// Check if this is a file
    pub fn is_file(&self) -> bool {
        self.node_type == FileNodeType::File
    }

    /// Add a child node ID
    pub fn add_child(&mut self, child_id: String) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }

    /// Add a knowledge graph link
    pub fn add_knowledge_link(&mut self, knowledge_node_id: String) {
        if !self.knowledge_links.contains(&knowledge_node_id) {
            self.knowledge_links.push(knowledge_node_id);
        }
    }

    /// Get file extension
    pub fn extension(&self) -> Option<String> {
        self.path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string())
    }
}

/// Container for the complete file tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTree {
    /// Root node ID
    pub root_id: Option<String>,

    /// All nodes indexed by ID
    pub nodes: HashMap<String, FileTreeNode>,

    /// Index: path -> node_id for fast lookup
    pub path_index: HashMap<PathBuf, String>,

    /// Total number of files
    pub file_count: usize,

    /// Total number of directories
    pub directory_count: usize,

    /// Base path of the tree
    pub base_path: PathBuf,
}

impl FileTree {
    /// Create a new empty file tree
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            root_id: None,
            nodes: HashMap::new(),
            path_index: HashMap::new(),
            file_count: 0,
            directory_count: 0,
            base_path,
        }
    }

    /// Add a node to the tree
    pub fn add_node(&mut self, node: FileTreeNode) -> String {
        let node_id = node.node_id.clone();
        let path = node.path.clone();

        // Update counts
        match node.node_type {
            FileNodeType::File => self.file_count += 1,
            FileNodeType::Directory => self.directory_count += 1,
            _ => {}
        }

        // Set as root if first node
        if self.root_id.is_none() {
            self.root_id = Some(node_id.clone());
        }

        // Add to collections
        self.path_index.insert(path, node_id.clone());
        self.nodes.insert(node_id.clone(), node);

        node_id
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&FileTreeNode> {
        self.nodes.get(node_id)
    }

    /// Get a mutable node by ID
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut FileTreeNode> {
        self.nodes.get_mut(node_id)
    }

    /// Get a node by path
    pub fn get_node_by_path(&self, path: &PathBuf) -> Option<&FileTreeNode> {
        self.path_index.get(path).and_then(|id| self.nodes.get(id))
    }

    /// Find nodes by name (exact match)
    pub fn find_by_name(&self, name: &str) -> Vec<&FileTreeNode> {
        self.nodes
            .values()
            .filter(|node| node.name == name)
            .collect()
    }

    /// Find nodes by name pattern (contains)
    pub fn find_by_name_pattern(&self, pattern: &str) -> Vec<&FileTreeNode> {
        self.nodes
            .values()
            .filter(|node| node.name.contains(pattern))
            .collect()
    }

    /// Filter nodes by type
    pub fn filter_by_type(&self, node_type: FileNodeType) -> Vec<&FileTreeNode> {
        self.nodes
            .values()
            .filter(|node| node.node_type == node_type)
            .collect()
    }

    /// Filter nodes by language
    pub fn filter_by_language(&self, language: Language) -> Vec<&FileTreeNode> {
        self.nodes
            .values()
            .filter(|node| node.metadata.language == Some(language))
            .collect()
    }

    /// Find nodes by path pattern (glob-like)
    pub fn find_by_path_pattern(&self, pattern: &str) -> Vec<&FileTreeNode> {
        self.nodes
            .values()
            .filter(|node| {
                if let Some(path_str) = node.path.to_str() {
                    path_str.contains(pattern)
                } else {
                    false
                }
            })
            .collect()
    }

    /// Get all children of a node
    pub fn get_children(&self, node_id: &str) -> Vec<&FileTreeNode> {
        self.nodes
            .get(node_id)
            .map(|node| {
                node.children
                    .iter()
                    .filter_map(|child_id| self.nodes.get(child_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all files (non-directories) in the tree
    pub fn get_all_files(&self) -> Vec<&FileTreeNode> {
        self.nodes
            .values()
            .filter(|node| node.is_file())
            .collect()
    }

    /// Get all directories in the tree
    pub fn get_all_directories(&self) -> Vec<&FileTreeNode> {
        self.nodes
            .values()
            .filter(|node| node.is_directory())
            .collect()
    }

    /// Traverse the tree depth-first
    pub fn traverse_depth_first<F>(&self, mut visitor: F)
    where
        F: FnMut(&FileTreeNode),
    {
        if let Some(root_id) = &self.root_id {
            self.traverse_depth_first_recursive(root_id, &mut visitor);
        }
    }

    fn traverse_depth_first_recursive<F>(&self, node_id: &str, visitor: &mut F)
    where
        F: FnMut(&FileTreeNode),
    {
        if let Some(node) = self.nodes.get(node_id) {
            visitor(node);
            for child_id in &node.children {
                self.traverse_depth_first_recursive(child_id, visitor);
            }
        }
    }

    /// Traverse the tree breadth-first
    pub fn traverse_breadth_first<F>(&self, mut visitor: F)
    where
        F: FnMut(&FileTreeNode),
    {
        if let Some(root_id) = &self.root_id {
            let mut queue = vec![root_id.clone()];

            while let Some(node_id) = queue.pop() {
                if let Some(node) = self.nodes.get(&node_id) {
                    visitor(node);
                    queue.extend(node.children.clone());
                }
            }
        }
    }

    /// Find nodes matching a predicate
    pub fn find_nodes<F>(&self, predicate: F) -> Vec<&FileTreeNode>
    where
        F: Fn(&FileTreeNode) -> bool,
    {
        self.nodes.values().filter(|node| predicate(node)).collect()
    }

    /// Get tree statistics
    pub fn stats(&self) -> FileTreeStats {
        FileTreeStats {
            total_nodes: self.nodes.len(),
            file_count: self.file_count,
            directory_count: self.directory_count,
            indexed_count: self.nodes.values().filter(|n| n.indexed).count(),
            max_depth: self.nodes.values().map(|n| n.depth).max().unwrap_or(0),
        }
    }
}

/// Statistics about the file tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeStats {
    pub total_nodes: usize,
    pub file_count: usize,
    pub directory_count: usize,
    pub indexed_count: usize,
    pub max_depth: usize,
}

/// ============================================================================
/// Knowledge Graph Models
/// ============================================================================

/// Type of knowledge node representing code entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KnowledgeNodeType {
    /// Function definition
    Function,
    /// Method definition (part of a class/struct)
    Method,
    /// Class definition
    Class,
    /// Struct definition
    Struct,
    /// Enum definition
    Enum,
    /// Interface/Trait definition
    Interface,
    /// Module/Package
    Module,
    /// Type alias
    TypeAlias,
    /// Constant definition
    Constant,
    /// Variable declaration
    Variable,
    /// Macro definition
    Macro,
    /// Concept (high-level abstraction)
    Concept,
    /// Comment/Documentation
    Documentation,
}

impl KnowledgeNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            KnowledgeNodeType::Function => "function",
            KnowledgeNodeType::Method => "method",
            KnowledgeNodeType::Class => "class",
            KnowledgeNodeType::Struct => "struct",
            KnowledgeNodeType::Enum => "enum",
            KnowledgeNodeType::Interface => "interface",
            KnowledgeNodeType::Module => "module",
            KnowledgeNodeType::TypeAlias => "type_alias",
            KnowledgeNodeType::Constant => "constant",
            KnowledgeNodeType::Variable => "variable",
            KnowledgeNodeType::Macro => "macro",
            KnowledgeNodeType::Concept => "concept",
            KnowledgeNodeType::Documentation => "documentation",
        }
    }

    /// Convert from SemanticNodeType
    pub fn from_semantic_type(semantic_type: SemanticNodeType) -> Self {
        match semantic_type {
            SemanticNodeType::Function => KnowledgeNodeType::Function,
            SemanticNodeType::Method => KnowledgeNodeType::Method,
            SemanticNodeType::Class => KnowledgeNodeType::Class,
            SemanticNodeType::Struct => KnowledgeNodeType::Struct,
            SemanticNodeType::Enum => KnowledgeNodeType::Enum,
            SemanticNodeType::Interface => KnowledgeNodeType::Interface,
            SemanticNodeType::Module => KnowledgeNodeType::Module,
            SemanticNodeType::TypeAlias => KnowledgeNodeType::TypeAlias,
            SemanticNodeType::Constant => KnowledgeNodeType::Constant,
            SemanticNodeType::Variable => KnowledgeNodeType::Variable,
            SemanticNodeType::Macro => KnowledgeNodeType::Macro,
            SemanticNodeType::Comment => KnowledgeNodeType::Documentation,
            _ => KnowledgeNodeType::Concept,
        }
    }
}

/// A node in the knowledge graph representing a code entity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNode {
    /// Unique identifier for this node
    pub node_id: String,

    /// Type of code entity
    pub content_type: KnowledgeNodeType,

    /// Name of the entity
    pub name: String,

    /// Fully qualified name (e.g., "module::class::method")
    pub qualified_name: String,

    /// Description/documentation
    pub description: Option<String>,

    /// Source code content
    pub source_code: Option<String>,

    /// Programming language
    pub language: Option<Language>,

    /// Function/method signature
    pub signature: Option<String>,

    /// Return type (if applicable)
    pub return_type: Option<String>,

    /// Parameters (for functions/methods)
    pub parameters: Vec<String>,

    /// Type parameters/generics
    pub type_parameters: Vec<String>,

    /// References to file tree nodes where this entity is defined
    pub file_references: Vec<FileReference>,

    /// Parent node ID (for nested entities)
    pub parent_id: Option<String>,

    /// Child node IDs
    pub children: Vec<String>,

    /// Visibility (public, private, etc.)
    pub visibility: Option<String>,

    /// Tags for categorization
    pub tags: HashSet<String>,

    /// Metadata
    pub metadata: HashMap<String, String>,
}

impl KnowledgeNode {
    /// Create a new knowledge node
    pub fn new(
        content_type: KnowledgeNodeType,
        name: String,
        qualified_name: String,
    ) -> Self {
        Self {
            node_id: Uuid::new_v4().to_string(),
            content_type,
            name,
            qualified_name,
            description: None,
            source_code: None,
            language: None,
            signature: None,
            return_type: None,
            parameters: Vec::new(),
            type_parameters: Vec::new(),
            file_references: Vec::new(),
            parent_id: None,
            children: Vec::new(),
            visibility: None,
            tags: HashSet::new(),
            metadata: HashMap::new(),
        }
    }

    /// Add a file reference
    pub fn add_file_reference(&mut self, file_ref: FileReference) {
        self.file_references.push(file_ref);
    }

    /// Add a child node
    pub fn add_child(&mut self, child_id: String) {
        if !self.children.contains(&child_id) {
            self.children.push(child_id);
        }
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: String) {
        self.tags.insert(tag);
    }
}

/// Reference to a location in a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileReference {
    /// File tree node ID
    pub file_node_id: String,

    /// File path
    pub file_path: PathBuf,

    /// Line range in the file [start, end]
    pub line_range: (usize, usize),

    /// Column range at start line
    pub column_range: Option<(usize, usize)>,

    /// Whether this is the primary definition
    pub is_definition: bool,
}

impl FileReference {
    pub fn new(
        file_node_id: String,
        file_path: PathBuf,
        line_range: (usize, usize),
    ) -> Self {
        Self {
            file_node_id,
            file_path,
            line_range,
            column_range: None,
            is_definition: true,
        }
    }
}

/// Type of relationship between knowledge nodes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum RelationshipType {
    /// Function A calls function B
    Calls,
    /// Module A imports module B
    Imports,
    /// Class A inherits from class B
    Inherits,
    /// Class A implements interface B
    Implements,
    /// Entity A uses/references entity B
    Uses,
    /// Entity A defines entity B
    Defines,
    /// Entity A is defined in entity B
    DefinedIn,
    /// Entity A overrides entity B
    Overrides,
    /// Entity A extends entity B
    Extends,
    /// Entity A depends on entity B
    DependsOn,
    /// Entity A is similar to entity B
    SimilarTo,
    /// Generic relationship
    Related,
}

impl RelationshipType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationshipType::Calls => "calls",
            RelationshipType::Imports => "imports",
            RelationshipType::Inherits => "inherits",
            RelationshipType::Implements => "implements",
            RelationshipType::Uses => "uses",
            RelationshipType::Defines => "defines",
            RelationshipType::DefinedIn => "defined_in",
            RelationshipType::Overrides => "overrides",
            RelationshipType::Extends => "extends",
            RelationshipType::DependsOn => "depends_on",
            RelationshipType::SimilarTo => "similar_to",
            RelationshipType::Related => "related",
        }
    }
}

/// An edge in the knowledge graph representing a relationship
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeEdge {
    /// Unique identifier for this edge
    pub edge_id: String,

    /// Source node ID
    pub from_node_id: String,

    /// Target node ID
    pub to_node_id: String,

    /// Type of relationship
    pub relationship_type: RelationshipType,

    /// Weight/strength of the relationship (0.0-1.0)
    pub weight: f32,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl KnowledgeEdge {
    /// Create a new knowledge edge
    pub fn new(
        from_node_id: String,
        to_node_id: String,
        relationship_type: RelationshipType,
    ) -> Self {
        Self {
            edge_id: Uuid::new_v4().to_string(),
            from_node_id,
            to_node_id,
            relationship_type,
            weight: 1.0,
            metadata: HashMap::new(),
        }
    }

    /// Set the weight
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }
}

/// Container for the complete knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    /// All nodes indexed by ID
    pub nodes: HashMap<String, KnowledgeNode>,

    /// All edges indexed by ID
    pub edges: HashMap<String, KnowledgeEdge>,

    /// Index: qualified_name -> node_id for fast lookup
    pub name_index: HashMap<String, String>,

    /// Index: from_node_id -> list of edge_ids (outgoing edges)
    pub outgoing_edges: HashMap<String, Vec<String>>,

    /// Index: to_node_id -> list of edge_ids (incoming edges)
    pub incoming_edges: HashMap<String, Vec<String>>,

    /// Index: node_type -> list of node_ids
    pub type_index: HashMap<KnowledgeNodeType, Vec<String>>,
}

impl KnowledgeGraph {
    /// Create a new empty knowledge graph
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            edges: HashMap::new(),
            name_index: HashMap::new(),
            outgoing_edges: HashMap::new(),
            incoming_edges: HashMap::new(),
            type_index: HashMap::new(),
        }
    }

    /// Add a node to the graph
    pub fn add_node(&mut self, node: KnowledgeNode) -> String {
        let node_id = node.node_id.clone();
        let qualified_name = node.qualified_name.clone();
        let content_type = node.content_type;

        // Update indices
        self.name_index.insert(qualified_name, node_id.clone());
        self.type_index
            .entry(content_type)
            .or_insert_with(Vec::new)
            .push(node_id.clone());

        // Add node
        self.nodes.insert(node_id.clone(), node);

        node_id
    }

    /// Add an edge to the graph
    pub fn add_edge(&mut self, edge: KnowledgeEdge) -> String {
        let edge_id = edge.edge_id.clone();
        let from_id = edge.from_node_id.clone();
        let to_id = edge.to_node_id.clone();

        // Update indices
        self.outgoing_edges
            .entry(from_id)
            .or_insert_with(Vec::new)
            .push(edge_id.clone());

        self.incoming_edges
            .entry(to_id)
            .or_insert_with(Vec::new)
            .push(edge_id.clone());

        // Add edge
        self.edges.insert(edge_id.clone(), edge);

        edge_id
    }

    /// Get a node by ID
    pub fn get_node(&self, node_id: &str) -> Option<&KnowledgeNode> {
        self.nodes.get(node_id)
    }

    /// Get a mutable node by ID
    pub fn get_node_mut(&mut self, node_id: &str) -> Option<&mut KnowledgeNode> {
        self.nodes.get_mut(node_id)
    }

    /// Get a node by qualified name
    pub fn get_node_by_name(&self, qualified_name: &str) -> Option<&KnowledgeNode> {
        self.name_index
            .get(qualified_name)
            .and_then(|id| self.nodes.get(id))
    }

    /// Get all nodes of a specific type
    pub fn get_nodes_by_type(&self, node_type: KnowledgeNodeType) -> Vec<&KnowledgeNode> {
        self.type_index
            .get(&node_type)
            .map(|ids| ids.iter().filter_map(|id| self.nodes.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get outgoing edges from a node
    pub fn get_outgoing_edges(&self, node_id: &str) -> Vec<&KnowledgeEdge> {
        self.outgoing_edges
            .get(node_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|edge_id| self.edges.get(edge_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get incoming edges to a node
    pub fn get_incoming_edges(&self, node_id: &str) -> Vec<&KnowledgeEdge> {
        self.incoming_edges
            .get(node_id)
            .map(|edge_ids| {
                edge_ids
                    .iter()
                    .filter_map(|edge_id| self.edges.get(edge_id))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get all neighbors of a node (connected by any edge)
    pub fn get_neighbors(&self, node_id: &str) -> Vec<&KnowledgeNode> {
        let mut neighbor_ids = HashSet::new();

        // Outgoing edges
        if let Some(edge_ids) = self.outgoing_edges.get(node_id) {
            for edge_id in edge_ids {
                if let Some(edge) = self.edges.get(edge_id) {
                    neighbor_ids.insert(&edge.to_node_id);
                }
            }
        }

        // Incoming edges
        if let Some(edge_ids) = self.incoming_edges.get(node_id) {
            for edge_id in edge_ids {
                if let Some(edge) = self.edges.get(edge_id) {
                    neighbor_ids.insert(&edge.from_node_id);
                }
            }
        }

        neighbor_ids
            .iter()
            .filter_map(|id| self.nodes.get(*id))
            .collect()
    }

    /// Get neighbors connected by a specific relationship type
    pub fn get_neighbors_by_relationship(
        &self,
        node_id: &str,
        relationship: RelationshipType,
    ) -> Vec<&KnowledgeNode> {
        let mut neighbor_ids = HashSet::new();

        // Outgoing edges
        for edge in self.get_outgoing_edges(node_id) {
            if edge.relationship_type == relationship {
                neighbor_ids.insert(&edge.to_node_id);
            }
        }

        // Incoming edges
        for edge in self.get_incoming_edges(node_id) {
            if edge.relationship_type == relationship {
                neighbor_ids.insert(&edge.from_node_id);
            }
        }

        neighbor_ids
            .iter()
            .filter_map(|id| self.nodes.get(*id))
            .collect()
    }

    /// Find shortest path between two nodes
    pub fn find_path(&self, from_id: &str, to_id: &str) -> Option<Vec<String>> {
        use std::collections::VecDeque;

        if from_id == to_id {
            return Some(vec![from_id.to_string()]);
        }

        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        let mut parent = HashMap::new();

        queue.push_back(from_id.to_string());
        visited.insert(from_id.to_string());

        while let Some(current) = queue.pop_front() {
            if current == to_id {
                // Reconstruct path
                let mut path = vec![current.clone()];
                let mut node = current;

                while let Some(prev) = parent.get(&node) {
                    path.push(prev.clone());
                    node = prev.clone();
                }

                path.reverse();
                return Some(path);
            }

            // Visit neighbors
            for neighbor in self.get_neighbors(&current) {
                let neighbor_id = neighbor.node_id.clone();
                if !visited.contains(&neighbor_id) {
                    visited.insert(neighbor_id.clone());
                    parent.insert(neighbor_id.clone(), current.clone());
                    queue.push_back(neighbor_id);
                }
            }
        }

        None
    }

    /// Find nodes matching a predicate
    pub fn find_nodes<F>(&self, predicate: F) -> Vec<&KnowledgeNode>
    where
        F: Fn(&KnowledgeNode) -> bool,
    {
        self.nodes.values().filter(|node| predicate(node)).collect()
    }

    /// Get graph statistics
    pub fn stats(&self) -> KnowledgeGraphStats {
        KnowledgeGraphStats {
            total_nodes: self.nodes.len(),
            total_edges: self.edges.len(),
            node_type_counts: self
                .type_index
                .iter()
                .map(|(k, v)| (k.as_str().to_string(), v.len()))
                .collect(),
            avg_degree: if self.nodes.is_empty() {
                0.0
            } else {
                (self.edges.len() * 2) as f32 / self.nodes.len() as f32
            },
        }
    }
}

impl Default for KnowledgeGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics about the knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub node_type_counts: HashMap<String, usize>,
    pub avg_degree: f32,
}

/// ============================================================================
/// Combined File Tree and Knowledge Graph Container
/// ============================================================================

/// Combined structure linking file tree and knowledge graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeRepository {
    /// The file tree structure
    pub file_tree: FileTree,

    /// The knowledge graph
    pub knowledge_graph: KnowledgeGraph,

    /// Repository metadata
    pub metadata: HashMap<String, String>,
}

impl CodeRepository {
    /// Create a new code repository
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            file_tree: FileTree::new(base_path),
            knowledge_graph: KnowledgeGraph::new(),
            metadata: HashMap::new(),
        }
    }

    /// Get combined statistics
    pub fn stats(&self) -> RepositoryStats {
        RepositoryStats {
            file_tree_stats: self.file_tree.stats(),
            knowledge_graph_stats: self.knowledge_graph.stats(),
        }
    }
}

/// Combined statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepositoryStats {
    pub file_tree_stats: FileTreeStats,
    pub knowledge_graph_stats: KnowledgeGraphStats,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_tree_node_creation() {
        let path = PathBuf::from("/test/file.rs");
        let node = FileTreeNode::new(path.clone(), FileNodeType::File, None, 0);

        assert_eq!(node.name, "file.rs");
        assert_eq!(node.path, path);
        assert!(node.is_file());
        assert!(!node.is_directory());
    }

    #[test]
    fn test_file_tree_operations() {
        let mut tree = FileTree::new(PathBuf::from("/test"));

        let root = FileTreeNode::new(
            PathBuf::from("/test"),
            FileNodeType::Directory,
            None,
            0,
        );
        let root_id = tree.add_node(root);

        let file = FileTreeNode::new(
            PathBuf::from("/test/file.rs"),
            FileNodeType::File,
            Some(root_id.clone()),
            1,
        );
        let file_id = tree.add_node(file);

        // Add child relationship
        if let Some(root_node) = tree.get_node_mut(&root_id) {
            root_node.add_child(file_id.clone());
        }

        assert_eq!(tree.file_count, 1);
        assert_eq!(tree.directory_count, 1);

        let children = tree.get_children(&root_id);
        assert_eq!(children.len(), 1);
    }

    #[test]
    fn test_knowledge_node_creation() {
        let node = KnowledgeNode::new(
            KnowledgeNodeType::Function,
            "test_func".to_string(),
            "module::test_func".to_string(),
        );

        assert_eq!(node.name, "test_func");
        assert_eq!(node.content_type, KnowledgeNodeType::Function);
    }

    #[test]
    fn test_knowledge_graph_operations() {
        let mut graph = KnowledgeGraph::new();

        let mut node1 = KnowledgeNode::new(
            KnowledgeNodeType::Function,
            "func_a".to_string(),
            "module::func_a".to_string(),
        );
        node1.language = Some(Language::Rust);

        let mut node2 = KnowledgeNode::new(
            KnowledgeNodeType::Function,
            "func_b".to_string(),
            "module::func_b".to_string(),
        );
        node2.language = Some(Language::Rust);

        let node1_id = graph.add_node(node1);
        let node2_id = graph.add_node(node2);

        let edge = KnowledgeEdge::new(
            node1_id.clone(),
            node2_id.clone(),
            RelationshipType::Calls,
        );
        graph.add_edge(edge);

        assert_eq!(graph.nodes.len(), 2);
        assert_eq!(graph.edges.len(), 1);

        let outgoing = graph.get_outgoing_edges(&node1_id);
        assert_eq!(outgoing.len(), 1);
        assert_eq!(outgoing[0].relationship_type, RelationshipType::Calls);
    }

    #[test]
    fn test_path_finding() {
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

        graph.add_edge(KnowledgeEdge::new(id1.clone(), id2.clone(), RelationshipType::Calls));
        graph.add_edge(KnowledgeEdge::new(id2.clone(), id3.clone(), RelationshipType::Calls));

        let path = graph.find_path(&id1, &id3);
        assert!(path.is_some());
        assert_eq!(path.unwrap().len(), 3);
    }

    #[test]
    fn test_relationship_type_conversion() {
        assert_eq!(RelationshipType::Calls.as_str(), "calls");
        assert_eq!(RelationshipType::Inherits.as_str(), "inherits");
    }
}
