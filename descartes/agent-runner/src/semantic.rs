/// Semantic extraction from AST
use crate::errors::{ParserError, ParserResult};
use crate::traversal::AstTraversal;
use crate::types::{Language, Parameter, SemanticNode, SemanticNodeType};
use serde_json::Value;
use std::collections::HashMap;
use tree_sitter::{Node, Tree};

/// Semantic extractor for analyzing code
pub struct SemanticExtractor {
    language: Language,
}

impl SemanticExtractor {
    /// Create a new semantic extractor for a language
    pub fn new(language: Language) -> Self {
        SemanticExtractor { language }
    }

    /// Extract semantic nodes from a tree
    pub fn extract_nodes(
        &self,
        tree: &Tree,
        source_code: &str,
        file_path: &str,
    ) -> ParserResult<Vec<SemanticNode>> {
        let mut nodes = Vec::new();
        let traversal = AstTraversal::new(tree, self.language);

        let mut id_counter = 0;

        // Visit all named nodes
        traversal.visit_matching(|node, _metadata| {
            if node.is_named() {
                if let Ok(semantic_node) =
                    self.extract_semantic_node(&node, source_code, file_path, &mut id_counter)
                {
                    nodes.push(semantic_node);
                }
            }
            Ok(true)
        })?;

        Ok(nodes)
    }

    /// Extract a single semantic node from a Tree-Sitter node
    fn extract_semantic_node(
        &self,
        node: &Node,
        source_code: &str,
        file_path: &str,
        id_counter: &mut usize,
    ) -> ParserResult<SemanticNode> {
        let node_id = format!(
            "{}_{}_{}_{}",
            file_path,
            node.start_byte(),
            node.end_byte(),
            id_counter
        );
        *id_counter += 1;

        let (node_type, name) = self.classify_node(node, source_code)?;

        let source = self.extract_source(node, source_code)?;
        let line_range = (node.start_byte() / 80, node.end_byte() / 80); // Approximate line numbers
        let column_range = Some((0, 80));

        let (signature, return_type, parameters) = self.extract_signature(node, source_code)?;

        let dependencies = self.extract_dependencies(node, source_code)?;

        let semantic_node = SemanticNode {
            id: node_id,
            node_type,
            name,
            source_code: source,
            documentation: self.extract_documentation(node, source_code),
            qualified_name: self.extract_qualified_name(node, source_code),
            line_range,
            column_range,
            language: self.language,
            file_path: file_path.to_string(),
            parent_id: None,
            child_ids: Vec::new(),
            signature,
            return_type,
            parameters,
            dependencies,
            type_parameters: self.extract_type_parameters(node, source_code)?,
            visibility: self.extract_visibility(node),
            metadata: HashMap::new(),
        };

        Ok(semantic_node)
    }

    /// Extract source code from a node
    fn extract_source(&self, node: &Node, source_code: &str) -> ParserResult<String> {
        let start = node.start_byte();
        let end = node.end_byte();

        if start >= source_code.len() || end > source_code.len() {
            return Ok(String::new());
        }

        Ok(source_code[start..end].to_string())
    }

    /// Classify the node type and extract its name
    fn classify_node(
        &self,
        node: &Node,
        source_code: &str,
    ) -> ParserResult<(SemanticNodeType, String)> {
        let kind = node.kind();
        let name = self.extract_name(node, source_code).unwrap_or_default();

        let node_type = match self.language {
            Language::Rust => self.classify_rust_node(kind, node),
            Language::Python => self.classify_python_node(kind, node),
            Language::JavaScript | Language::TypeScript => self.classify_js_node(kind, node),
        };

        Ok((node_type, name))
    }

    /// Extract the name of a node
    fn extract_name(&self, node: &Node, source_code: &str) -> Option<String> {
        match self.language {
            Language::Rust => {
                // For Rust, look for identifier child nodes
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "identifier" {
                        return Some(self.extract_source(child, source_code).ok()?);
                    }
                }
                None
            }
            Language::Python => {
                // For Python, look for NAME nodes
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "identifier" {
                        return Some(self.extract_source(child, source_code).ok()?);
                    }
                }
                None
            }
            Language::JavaScript | Language::TypeScript => {
                // For JS/TS, look for identifier nodes
                for child in node.children(&mut node.walk()) {
                    if child.kind() == "identifier" {
                        return Some(self.extract_source(child, source_code).ok()?);
                    }
                }
                None
            }
        }
    }

    /// Classify a Rust node
    fn classify_rust_node(&self, kind: &str, _node: &Node) -> SemanticNodeType {
        match kind {
            "function_item" | "fn_item" => SemanticNodeType::Function,
            "struct_item" => SemanticNodeType::Struct,
            "enum_item" => SemanticNodeType::Enum,
            "trait_item" => SemanticNodeType::Interface,
            "impl_item" => SemanticNodeType::Class,
            "use_declaration" | "use_as_clause" => SemanticNodeType::Import,
            "type_alias" => SemanticNodeType::TypeAlias,
            "const_item" => SemanticNodeType::Constant,
            "let_declaration" => SemanticNodeType::Variable,
            "line_comment" | "block_comment" => SemanticNodeType::Comment,
            "macro_definition" => SemanticNodeType::Macro,
            "field_declaration" => SemanticNodeType::Property,
            "source_file" => SemanticNodeType::Module,
            "module" => SemanticNodeType::Module,
            _ => SemanticNodeType::Other,
        }
    }

    /// Classify a Python node
    fn classify_python_node(&self, kind: &str, _node: &Node) -> SemanticNodeType {
        match kind {
            "function_definition" => SemanticNodeType::Function,
            "class_definition" => SemanticNodeType::Class,
            "import_statement" | "import_from_statement" => SemanticNodeType::Import,
            "assignment" => SemanticNodeType::Variable,
            "comment" => SemanticNodeType::Comment,
            "module" => SemanticNodeType::Module,
            _ => SemanticNodeType::Other,
        }
    }

    /// Classify a JavaScript/TypeScript node
    fn classify_js_node(&self, kind: &str, _node: &Node) -> SemanticNodeType {
        match kind {
            "function_declaration" | "function_expression" | "arrow_function" => {
                SemanticNodeType::Function
            }
            "class_declaration" => SemanticNodeType::Class,
            "import_statement" => SemanticNodeType::Import,
            "export_statement" => SemanticNodeType::Export,
            "variable_declarator" => SemanticNodeType::Variable,
            "comment" => SemanticNodeType::Comment,
            "interface_declaration" | "type_alias_declaration" => SemanticNodeType::Interface,
            "program" | "module" => SemanticNodeType::Module,
            "method_definition" => SemanticNodeType::Method,
            "property_identifier" => SemanticNodeType::Property,
            _ => SemanticNodeType::Other,
        }
    }

    /// Extract function/method signature
    fn extract_signature(
        &self,
        _node: &Node,
        _source_code: &str,
    ) -> ParserResult<(Option<String>, Option<String>, Vec<Parameter>)> {
        // This would require language-specific parsing of parameter lists
        // For now, return empty signature
        Ok((None, None, Vec::new()))
    }

    /// Extract type parameters (generics)
    fn extract_type_parameters(
        &self,
        _node: &Node,
        _source_code: &str,
    ) -> ParserResult<Vec<String>> {
        // This would require language-specific parsing
        Ok(Vec::new())
    }

    /// Extract visibility modifier
    fn extract_visibility(&self, _node: &Node) -> Option<String> {
        // This would require looking at preceding keywords
        None
    }

    /// Extract dependencies/imports this node uses
    fn extract_dependencies(&self, _node: &Node, _source_code: &str) -> ParserResult<Vec<String>> {
        // This would require analyzing all referenced types/modules
        Ok(Vec::new())
    }

    /// Extract documentation comments
    fn extract_documentation(&self, node: &Node, source_code: &str) -> Option<String> {
        // Look for preceding comment nodes
        if let Some(parent) = node.parent() {
            for child in parent.children(&mut parent.walk()) {
                // Check if child is a comment immediately before this node
                if child.end_byte() < node.start_byte() && child.kind().contains("comment") {
                    if let Ok(doc) = self.extract_source(&child, source_code) {
                        return Some(doc);
                    }
                }
            }
        }
        None
    }

    /// Extract qualified name (full path)
    fn extract_qualified_name(&self, node: &Node, source_code: &str) -> String {
        match self.extract_name(node, source_code) {
            Some(name) => name,
            None => node.kind().to_string(),
        }
    }
}

/// Semantic analysis results
#[derive(Debug, Clone)]
pub struct SemanticAnalysis {
    pub nodes: Vec<SemanticNode>,
    pub relationships: Vec<(String, String)>, // (source_id, target_id)
    pub statistics: SemanticStatistics,
}

/// Statistics about semantic extraction
#[derive(Debug, Clone, Default)]
pub struct SemanticStatistics {
    pub total_nodes: usize,
    pub nodes_by_type: HashMap<String, usize>,
    pub extraction_time_ms: u128,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::grammar::create_parser;

    #[test]
    fn test_extract_rust_functions() {
        let code = r#"
        /// Documentation
        fn hello(name: &str) -> String {
            format!("Hello, {}", name)
        }
        "#;

        let mut parser = create_parser(Language::Rust).unwrap();
        let tree = parser.parse(code, None).unwrap();

        let extractor = SemanticExtractor::new(Language::Rust);
        let nodes = extractor.extract_nodes(&tree, code, "test.rs").unwrap();

        assert!(!nodes.is_empty());
    }

    #[test]
    fn test_classify_nodes() {
        let code = r#"
        fn hello() {}
        struct Point { x: i32, y: i32 }
        use std::collections::HashMap;
        "#;

        let mut parser = create_parser(Language::Rust).unwrap();
        let tree = parser.parse(code, None).unwrap();

        let extractor = SemanticExtractor::new(Language::Rust);
        let nodes = extractor.extract_nodes(&tree, code, "test.rs").unwrap();

        let has_function = nodes
            .iter()
            .any(|n| n.node_type == SemanticNodeType::Function);
        let has_struct = nodes
            .iter()
            .any(|n| n.node_type == SemanticNodeType::Struct);
        let has_import = nodes
            .iter()
            .any(|n| n.node_type == SemanticNodeType::Import);

        assert!(has_function || has_struct || has_import);
    }
}
