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

pub mod errors;
pub mod types;
pub mod grammar;
pub mod traversal;
pub mod semantic;
pub mod parser;
pub mod db_schema;
pub mod rag;

// Re-exports for convenient access
pub use errors::{ParserError, ParserResult};
pub use types::{Language, SemanticNode, SemanticNodeType, ParserConfig, ParseResult, ParseStatistics};
pub use grammar::{load_grammar, create_parser, initialize_grammars};
pub use traversal::{AstTraversal, TraversalStrategy, QueryHelper};
pub use semantic::{SemanticExtractor, SemanticAnalysis};
pub use parser::SemanticParser;
pub use db_schema::DbPool;
pub use rag::{
    RagSystem, RagConfig, RagStats, CodeChunk, SearchResult,
    EmbeddingProvider, OpenAiEmbeddings, AnthropicEmbeddings,
    EmbeddingCache, SemanticChunker, VectorStore, FullTextSearch,
};

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
        let result = parser.parse_source(code, Language::Rust, "test.rs").unwrap();

        assert_eq!(result.language, Language::Rust);
        assert!(result.error.is_none());
    }
}
