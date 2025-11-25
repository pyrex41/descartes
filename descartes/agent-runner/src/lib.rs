// Descartes Agent Runner: Semantic Code Parsing and Orchestration
// Part of the Descartes Multi-Agent AI Orchestration System
//
// This module provides high-performance semantic code parsing using Tree-Sitter,
// with support for multiple languages (Rust, Python, JavaScript, TypeScript).
//
// Features:
// - Multi-language AST parsing and traversal
// - Semantic node extraction (functions, classes, imports, etc.)
// - SQLite database storage for AST data
// - Parallel parsing of multiple files
// - Query interface for accessing semantic information
// - Integration with RAG (Retrieval-Augmented Generation) systems

pub mod db_schema;
pub mod errors;
pub mod file_tree_builder;
pub mod grammar;
pub mod knowledge_graph;
pub mod knowledge_graph_overlay;
pub mod parser;
pub mod rag;
pub mod semantic;
pub mod traversal;
pub mod types;

// Re-exports for convenient access
pub use db_schema::DbPool;
pub use errors::{ParserError, ParserResult};
pub use grammar::{create_parser, initialize_grammars, load_grammar};
pub use parser::SemanticParser;
pub use rag::{
    AnthropicEmbeddings, CodeChunk, EmbeddingCache, EmbeddingProvider, FullTextSearch,
    OpenAiEmbeddings, RagConfig, RagStats, RagSystem, SearchResult, SemanticChunker, VectorStore,
};
pub use semantic::{SemanticAnalysis, SemanticExtractor};
pub use traversal::{AstTraversal, QueryHelper, TraversalStrategy};
pub use types::{
    Language, ParseResult, ParseStatistics, ParserConfig, SemanticNode, SemanticNodeType,
};

pub use knowledge_graph::{
    CodeRepository, FileMetadata, FileNodeType, FileReference, FileTree, FileTreeNode,
    FileTreeStats, KnowledgeEdge, KnowledgeGraph, KnowledgeGraphStats, KnowledgeNode,
    KnowledgeNodeType, RelationshipType, RepositoryStats,
};

pub use file_tree_builder::{
    count_lines, detect_language, find_git_root, is_binary_file, FileTreeBuilder,
    FileTreeBuilderConfig, FileTreeUpdater,
};

pub use knowledge_graph_overlay::{CacheStats, KnowledgeGraphOverlay, OverlayConfig};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_parse_simple_rust() {
        let code = "fn main() { println!(\"hello\"); }";
        let mut parser = parser::SemanticParser::new().unwrap();
        let result = parser
            .parse_source(code, Language::Rust, "test.rs")
            .unwrap();

        assert_eq!(result.language, Language::Rust);
        assert!(result.error.is_none());
    }
}
