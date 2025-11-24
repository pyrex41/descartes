/// Type definitions for semantic extraction and AST representation
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Supported programming languages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Language {
    Rust,
    Python,
    JavaScript,
    TypeScript,
}

impl Language {
    /// Get the string representation of the language
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::JavaScript => "javascript",
            Language::TypeScript => "typescript",
        }
    }

    /// Parse language from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Some(Language::Rust),
            "python" | "py" => Some(Language::Python),
            "javascript" | "js" => Some(Language::JavaScript),
            "typescript" | "ts" => Some(Language::TypeScript),
            _ => None,
        }
    }

    /// Get file extensions for this language
    pub fn extensions(&self) -> &'static [&'static str] {
        match self {
            Language::Rust => &["rs"],
            Language::Python => &["py"],
            Language::JavaScript => &["js", "jsx", "mjs"],
            Language::TypeScript => &["ts", "tsx"],
        }
    }
}

/// Type of semantic node extracted from AST
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SemanticNodeType {
    /// Top-level module or file
    Module,
    /// Function definition
    Function,
    /// Class definition
    Class,
    /// Struct definition
    Struct,
    /// Enum definition
    Enum,
    /// Interface/Trait definition
    Interface,
    /// Import statement
    Import,
    /// Export statement
    Export,
    /// Type alias
    TypeAlias,
    /// Constant definition
    Constant,
    /// Variable declaration
    Variable,
    /// Comment block
    Comment,
    /// Type annotation
    Type,
    /// Macro definition
    Macro,
    /// Method definition (part of class/struct)
    Method,
    /// Property/Field
    Property,
    /// Other/Unknown
    Other,
}

impl SemanticNodeType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SemanticNodeType::Module => "module",
            SemanticNodeType::Function => "function",
            SemanticNodeType::Class => "class",
            SemanticNodeType::Struct => "struct",
            SemanticNodeType::Enum => "enum",
            SemanticNodeType::Interface => "interface",
            SemanticNodeType::Import => "import",
            SemanticNodeType::Export => "export",
            SemanticNodeType::TypeAlias => "type_alias",
            SemanticNodeType::Constant => "constant",
            SemanticNodeType::Variable => "variable",
            SemanticNodeType::Comment => "comment",
            SemanticNodeType::Type => "type",
            SemanticNodeType::Macro => "macro",
            SemanticNodeType::Method => "method",
            SemanticNodeType::Property => "property",
            SemanticNodeType::Other => "other",
        }
    }
}

/// A semantic node extracted from the AST
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticNode {
    /// Unique identifier (typically a hash or UUID)
    pub id: String,

    /// Type of this node
    pub node_type: SemanticNodeType,

    /// Name of the node (function name, class name, etc.)
    pub name: String,

    /// The source code text of this node
    pub source_code: String,

    /// The documentation/comment preceding this node
    pub documentation: Option<String>,

    /// Full qualified path (e.g., "module::submodule::function")
    pub qualified_name: String,

    /// Line range [start, end] in the source file
    pub line_range: (usize, usize),

    /// Column range [start, end] at the start line
    pub column_range: Option<(usize, usize)>,

    /// Language this node belongs to
    pub language: Language,

    /// File path this node came from
    pub file_path: String,

    /// Parent node ID (for hierarchy)
    pub parent_id: Option<String>,

    /// Child node IDs
    pub child_ids: Vec<String>,

    /// Extracted signatures (for functions, methods, etc.)
    pub signature: Option<String>,

    /// Return type (if applicable)
    pub return_type: Option<String>,

    /// Parameter information
    pub parameters: Vec<Parameter>,

    /// Dependencies/imports this node uses
    pub dependencies: Vec<String>,

    /// Generic/Template parameters
    pub type_parameters: Vec<String>,

    /// Visibility modifier (public, private, etc.)
    pub visibility: Option<String>,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Parameter information for functions/methods
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub param_type: String,
    pub has_default: bool,
    pub is_variadic: bool,
}

/// Configuration for the parser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserConfig {
    /// Languages to support
    pub languages: Vec<Language>,

    /// Maximum tree depth to traverse
    pub max_depth: Option<usize>,

    /// Whether to extract documentation
    pub extract_docs: bool,

    /// Whether to extract type information
    pub extract_types: bool,

    /// Whether to extract dependencies
    pub extract_dependencies: bool,

    /// Whether to perform parallel parsing
    pub parallel: bool,

    /// Number of threads for parallel processing
    pub num_threads: Option<usize>,

    /// Query expressions to use
    pub queries: HashMap<String, String>,
}

impl Default for ParserConfig {
    fn default() -> Self {
        ParserConfig {
            languages: vec![
                Language::Rust,
                Language::Python,
                Language::JavaScript,
                Language::TypeScript,
            ],
            max_depth: None,
            extract_docs: true,
            extract_types: true,
            extract_dependencies: true,
            parallel: true,
            num_threads: None,
            queries: HashMap::new(),
        }
    }
}

/// Statistics about parsed content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParseStatistics {
    pub total_nodes: usize,
    pub node_counts: HashMap<String, usize>,
    pub parse_duration_ms: u128,
    pub files_processed: usize,
    pub total_lines: usize,
    pub errors_encountered: Vec<String>,
}

/// Result of parsing a file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParseResult {
    pub file_path: String,
    pub language: Language,
    pub nodes: Vec<SemanticNode>,
    pub total_nodes: usize,
    pub parse_duration_ms: u128,
    pub error: Option<String>,
}
