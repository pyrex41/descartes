/// AST traversal utilities for Tree-Sitter trees
use crate::errors::{ParserError, ParserResult};
use crate::types::{Language, SemanticNodeType};
use std::collections::VecDeque;
use tree_sitter::{Node, Tree};

/// A traversal strategy for the AST
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraversalStrategy {
    /// Breadth-first traversal
    BreadthFirst,
    /// Depth-first pre-order traversal
    DepthFirstPreOrder,
    /// Depth-first post-order traversal
    DepthFirstPostOrder,
}

/// Metadata about an AST node during traversal
#[derive(Debug, Clone)]
pub struct NodeMetadata {
    /// Depth in the tree (0 = root)
    pub depth: usize,
    /// Index among siblings
    pub sibling_index: usize,
    /// Total number of siblings
    pub sibling_count: usize,
    /// Child count
    pub child_count: usize,
    /// Whether the node has children
    pub has_children: bool,
}

/// A node visitor callback
pub type NodeVisitor<'a> = Box<dyn FnMut(Node<'a>, &NodeMetadata) -> ParserResult<()> + 'a>;

/// AST traversal engine
pub struct AstTraversal<'a> {
    tree: &'a Tree,
    strategy: TraversalStrategy,
    max_depth: Option<usize>,
    language: Language,
}

impl<'a> AstTraversal<'a> {
    /// Create a new AST traversal
    pub fn new(tree: &'a Tree, language: Language) -> Self {
        AstTraversal {
            tree,
            strategy: TraversalStrategy::DepthFirstPreOrder,
            max_depth: None,
            language,
        }
    }

    /// Set the traversal strategy
    pub fn with_strategy(mut self, strategy: TraversalStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Set the maximum depth to traverse
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = Some(depth);
        self
    }

    /// Get the root node
    pub fn root(&self) -> Node<'a> {
        self.tree.root_node()
    }

    /// Visit all nodes matching a predicate
    pub fn visit_matching<F>(&self, mut visitor: F) -> ParserResult<usize>
    where
        F: FnMut(Node<'a>, &NodeMetadata) -> ParserResult<bool>,
    {
        let mut count = 0;
        let root = self.root();
        let mut queue = VecDeque::new();
        queue.push_back((root, 0, 0));

        while let Some((node, depth, sibling_idx)) = queue.pop_front() {
            // Check depth limit
            if let Some(max_d) = self.max_depth {
                if depth > max_d {
                    continue;
                }
            }

            let sibling_count = if node.parent().is_some() {
                node.parent().map(|p| p.child_count()).unwrap_or(1)
            } else {
                1
            };

            let metadata = NodeMetadata {
                depth,
                sibling_index: sibling_idx,
                sibling_count,
                child_count: node.child_count(),
                has_children: node.child_count() > 0,
            };

            if visitor(node, &metadata)? {
                count += 1;
            }

            // Add children to queue based on strategy
            let children: Vec<_> = node.children(&mut node.walk()).collect();

            match self.strategy {
                TraversalStrategy::BreadthFirst => {
                    for (idx, child) in children.iter().enumerate() {
                        queue.push_back((*child, depth + 1, idx));
                    }
                }
                TraversalStrategy::DepthFirstPreOrder => {
                    for (idx, _child) in children.iter().enumerate().rev() {
                        queue.push_back((children[idx], depth + 1, idx));
                    }
                }
                TraversalStrategy::DepthFirstPostOrder => {
                    // For post-order, we process after children
                    for (idx, child) in children.iter().enumerate() {
                        queue.push_back((*child, depth + 1, idx));
                    }
                }
            }
        }

        Ok(count)
    }

    /// Find all nodes of a specific kind
    pub fn find_nodes_by_kind(&self, kind: &str) -> ParserResult<Vec<Node<'a>>> {
        let mut nodes = Vec::new();
        self.visit_matching(|node, _metadata| {
            if node.kind() == kind {
                nodes.push(node);
                Ok(true)
            } else {
                Ok(false)
            }
        })?;
        Ok(nodes)
    }

    /// Find the first node matching a predicate
    pub fn find_first<F>(&self, mut predicate: F) -> ParserResult<Option<Node<'a>>>
    where
        F: FnMut(Node<'a>) -> bool,
    {
        let root = self.root();
        let mut queue = VecDeque::new();
        queue.push_back(root);

        while let Some(node) = queue.pop_front() {
            if predicate(node) {
                return Ok(Some(node));
            }

            for child in node.children(&mut node.walk()) {
                queue.push_back(child);
            }
        }

        Ok(None)
    }

    /// Extract the source code for a node
    pub fn get_node_source<'b>(&self, node: Node<'a>, source: &'b [u8]) -> ParserResult<&'b str> {
        let start = node.start_byte();
        let end = node.end_byte();

        if start >= source.len() || end > source.len() {
            return Err(ParserError::NodeExtractionError(
                "Node range out of bounds".to_string(),
            ));
        }

        let source_slice = &source[start..end];
        String::from_utf8(source_slice.to_vec())
            .map(|s| Box::leak(s.into_boxed_str()))
            .map_err(|e| ParserError::Utf8Error(e))
    }

    /// Get all named children of a node
    pub fn get_named_children(&self, node: Node<'a>) -> Vec<Node<'a>> {
        node.children(&mut node.walk())
            .filter(|child| !child.is_missing() && !child.is_extra())
            .collect()
    }

    /// Get the parent chain for a node
    pub fn get_ancestor_chain(&self, mut node: Node<'a>) -> Vec<Node<'a>> {
        let mut chain = vec![node];
        while let Some(parent) = node.parent() {
            chain.push(parent);
            node = parent;
        }
        chain.reverse();
        chain
    }

    /// Count nodes of each kind
    pub fn count_nodes_by_kind(&self) -> ParserResult<std::collections::HashMap<String, usize>> {
        let mut counts = std::collections::HashMap::new();
        self.visit_matching(|node, _metadata| {
            *counts.entry(node.kind().to_string()).or_insert(0) += 1;
            Ok(true)
        })?;
        Ok(counts)
    }

    /// Get basic statistics about the tree
    pub fn get_statistics(&self) -> ParserResult<TreeStatistics> {
        let root = self.root();
        let mut total_nodes = 0;
        let mut max_depth = 0;
        let mut node_kinds = std::collections::HashMap::new();

        self.visit_matching(|node, metadata| {
            total_nodes += 1;
            if metadata.depth > max_depth {
                max_depth = metadata.depth;
            }
            *node_kinds.entry(node.kind().to_string()).or_insert(0) += 1;
            Ok(true)
        })?;

        Ok(TreeStatistics {
            total_nodes,
            max_depth,
            node_kinds,
            root_kind: root.kind().to_string(),
        })
    }
}

/// Statistics about an AST tree
#[derive(Debug, Clone)]
pub struct TreeStatistics {
    pub total_nodes: usize,
    pub max_depth: usize,
    pub node_kinds: std::collections::HashMap<String, usize>,
    pub root_kind: String,
}

/// Query helper for selecting nodes using Tree-Sitter query language
pub struct QueryHelper {
    query: tree_sitter::Query,
    language: Language,
}

impl QueryHelper {
    /// Create a new query helper
    pub fn new(language: Language, query_string: &str) -> ParserResult<Self> {
        let lang_ts = crate::grammar::load_grammar(language)?;

        let query = tree_sitter::Query::new(lang_ts, query_string).map_err(|e| {
            ParserError::QueryCompileError(format!("Failed to compile query: {}", e))
        })?;

        Ok(QueryHelper { query, language })
    }

    /// Execute the query on a tree
    pub fn execute<'a>(&self, tree: &'a Tree, source: &[u8]) -> ParserResult<Vec<QueryMatch<'a>>> {
        let mut cursor = tree_sitter::QueryCursor::new();
        let root = tree.root_node();

        let matches = cursor.matches(&self.query, root, source);

        let results = matches
            .map(|m| QueryMatch {
                capture_names: self.query.capture_names().to_vec(),
                captures: m
                    .captures
                    .iter()
                    .map(|c| (c.index as usize, c.node))
                    .collect(),
            })
            .collect();

        Ok(results)
    }
}

/// A query match result
#[derive(Debug)]
pub struct QueryMatch<'a> {
    pub capture_names: Vec<String>,
    pub captures: Vec<(usize, Node<'a>)>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::create_parser;

    #[test]
    fn test_traversal_strategies() {
        let code = r#"fn hello() { println!("world"); }"#;
        let mut parser = create_parser(Language::Rust).unwrap();
        let tree = parser.parse(code, None).unwrap();

        let traversal = AstTraversal::new(&tree, Language::Rust);
        let stats = traversal.get_statistics().unwrap();
        assert!(stats.total_nodes > 0);
    }

    #[test]
    fn test_find_nodes_by_kind() {
        let code = r#"fn hello() { fn inner() {} }"#;
        let mut parser = create_parser(Language::Rust).unwrap();
        let tree = parser.parse(code, None).unwrap();

        let traversal = AstTraversal::new(&tree, Language::Rust);
        let functions = traversal.find_nodes_by_kind("function_item").unwrap();
        assert!(!functions.is_empty());
    }
}
