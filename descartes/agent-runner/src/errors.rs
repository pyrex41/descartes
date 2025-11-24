/// Error types for the parser module
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ParserError {
    #[error("Language initialization failed: {0}")]
    LanguageInitError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Tree-sitter error: {0}")]
    TreeSitterError(String),

    #[error("Grammar loading failed: {0}")]
    GrammarLoadError(String),

    #[error("Invalid language: {0}")]
    InvalidLanguage(String),

    #[error("Query compilation failed: {0}")]
    QueryCompileError(String),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("UTF-8 error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),

    #[error("Node extraction failed: {0}")]
    NodeExtractionError(String),

    #[error("Traversal error: {0}")]
    TraversalError(String),

    #[error("Semantic extraction failed: {0}")]
    SemanticExtractionError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

pub type ParserResult<T> = Result<T, ParserError>;
