/// High-level parser interface for semantic code analysis
use crate::errors::ParserResult;
use crate::grammar::create_parser;
use crate::semantic::{SemanticAnalysis, SemanticExtractor, SemanticStatistics};
use crate::traversal::AstTraversal;
use crate::types::{Language, ParseResult, ParserConfig, ParseStatistics, SemanticNode};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;
use tree_sitter::Parser;

/// Main semantic parser for code
pub struct SemanticParser {
    config: ParserConfig,
    parsers: HashMap<Language, Parser>,
}

impl SemanticParser {
    /// Create a new semantic parser with default configuration
    pub fn new() -> ParserResult<Self> {
        Self::with_config(ParserConfig::default())
    }

    /// Create a new semantic parser with custom configuration
    pub fn with_config(config: ParserConfig) -> ParserResult<Self> {
        let mut parsers = HashMap::new();

        // Initialize parsers for all configured languages
        for lang in &config.languages {
            let parser = create_parser(*lang)?;
            parsers.insert(*lang, parser);
            tracing::info!("Parser initialized for {}", lang.as_str());
        }

        Ok(SemanticParser { config, parsers })
    }

    /// Parse a single file and extract semantic nodes
    pub fn parse_file(&mut self, file_path: &str) -> ParserResult<ParseResult> {
        let start = Instant::now();
        let path = Path::new(file_path);

        // Detect language from file extension
        let language = self.detect_language(file_path)?;

        // Read file
        let source_code = fs::read_to_string(file_path).map_err(|e| {
            crate::errors::ParserError::IoError(e)
        })?;

        // Parse
        let parser = self.parsers.get_mut(&language).ok_or_else(|| {
            crate::errors::ParserError::InvalidLanguage(language.as_str().to_string())
        })?;

        let tree = match parser.parse(&source_code, None) {
            Some(t) => t,
            None => {
                return Ok(ParseResult {
                    file_path: file_path.to_string(),
                    language,
                    nodes: Vec::new(),
                    total_nodes: 0,
                    parse_duration_ms: start.elapsed().as_millis(),
                    error: Some("Failed to parse file".to_string()),
                })
            }
        };

        // Extract semantic nodes
        let extractor = SemanticExtractor::new(language);
        let nodes = extractor.extract_nodes(&tree, &source_code, file_path)?;

        let result = ParseResult {
            file_path: file_path.to_string(),
            language,
            total_nodes: nodes.len(),
            nodes,
            parse_duration_ms: start.elapsed().as_millis(),
            error: None,
        };

        Ok(result)
    }

    /// Parse multiple files (optionally in parallel)
    pub fn parse_files(&mut self, file_paths: &[&str]) -> ParserResult<Vec<ParseResult>> {
        if self.config.parallel {
            self.parse_files_parallel(file_paths)
        } else {
            self.parse_files_sequential(file_paths)
        }
    }

    /// Parse files sequentially
    fn parse_files_sequential(&mut self, file_paths: &[&str]) -> ParserResult<Vec<ParseResult>> {
        let mut results = Vec::new();
        for file_path in file_paths {
            match self.parse_file(file_path) {
                Ok(result) => results.push(result),
                Err(e) => {
                    tracing::warn!("Failed to parse {}: {}", file_path, e);
                    results.push(ParseResult {
                        file_path: file_path.to_string(),
                        language: Language::Rust, // Default fallback
                        nodes: Vec::new(),
                        total_nodes: 0,
                        parse_duration_ms: 0,
                        error: Some(e.to_string()),
                    });
                }
            }
        }
        Ok(results)
    }

    /// Parse files in parallel using rayon
    fn parse_files_parallel(&mut self, file_paths: &[&str]) -> ParserResult<Vec<ParseResult>> {
        use rayon::prelude::*;

        let results = file_paths
            .par_iter()
            .map(|file_path| {
                let mut parser = match create_parser(self.detect_language(file_path).unwrap_or(Language::Rust)) {
                    Ok(p) => p,
                    Err(e) => {
                        return ParseResult {
                            file_path: file_path.to_string(),
                            language: Language::Rust,
                            nodes: Vec::new(),
                            total_nodes: 0,
                            parse_duration_ms: 0,
                            error: Some(e.to_string()),
                        }
                    }
                };

                let source_code = match fs::read_to_string(file_path) {
                    Ok(c) => c,
                    Err(e) => {
                        return ParseResult {
                            file_path: file_path.to_string(),
                            language: Language::Rust,
                            nodes: Vec::new(),
                            total_nodes: 0,
                            parse_duration_ms: 0,
                            error: Some(e.to_string()),
                        }
                    }
                };

                let language = self.detect_language(file_path).unwrap_or(Language::Rust);

                let tree = match parser.parse(&source_code, None) {
                    Some(t) => t,
                    None => {
                        return ParseResult {
                            file_path: file_path.to_string(),
                            language,
                            nodes: Vec::new(),
                            total_nodes: 0,
                            parse_duration_ms: 0,
                            error: Some("Failed to parse".to_string()),
                        }
                    }
                };

                let extractor = SemanticExtractor::new(language);
                match extractor.extract_nodes(&tree, &source_code, file_path) {
                    Ok(nodes) => ParseResult {
                        file_path: file_path.to_string(),
                        language,
                        total_nodes: nodes.len(),
                        nodes,
                        parse_duration_ms: 0,
                        error: None,
                    },
                    Err(e) => ParseResult {
                        file_path: file_path.to_string(),
                        language,
                        nodes: Vec::new(),
                        total_nodes: 0,
                        parse_duration_ms: 0,
                        error: Some(e.to_string()),
                    },
                }
            })
            .collect();

        Ok(results)
    }

    /// Detect language from file extension
    fn detect_language(&self, file_path: &str) -> ParserResult<Language> {
        let path = Path::new(file_path);
        let extension = path
            .extension()
            .and_then(|e| e.to_str())
            .ok_or_else(|| {
                crate::errors::ParserError::InvalidLanguage("Unknown file extension".to_string())
            })?;

        Language::from_str(extension).ok_or_else(|| {
            crate::errors::ParserError::InvalidLanguage(format!("Unsupported extension: {}", extension))
        })
    }

    /// Parse source code directly
    pub fn parse_source(
        &mut self,
        source_code: &str,
        language: Language,
        file_path: &str,
    ) -> ParserResult<ParseResult> {
        let start = Instant::now();

        let parser = self.parsers.get_mut(&language).ok_or_else(|| {
            crate::errors::ParserError::InvalidLanguage(language.as_str().to_string())
        })?;

        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => {
                return Ok(ParseResult {
                    file_path: file_path.to_string(),
                    language,
                    nodes: Vec::new(),
                    total_nodes: 0,
                    parse_duration_ms: start.elapsed().as_millis(),
                    error: Some("Parse failed".to_string()),
                })
            }
        };

        let extractor = SemanticExtractor::new(language);
        let nodes = extractor.extract_nodes(&tree, source_code, file_path)?;

        Ok(ParseResult {
            file_path: file_path.to_string(),
            language,
            total_nodes: nodes.len(),
            nodes,
            parse_duration_ms: start.elapsed().as_millis(),
            error: None,
        })
    }

    /// Query AST nodes using Tree-Sitter query syntax
    pub fn query_nodes(
        &mut self,
        source_code: &str,
        language: Language,
        query_string: &str,
    ) -> ParserResult<Vec<SemanticNode>> {
        let parser = self.parsers.get_mut(&language).ok_or_else(|| {
            crate::errors::ParserError::InvalidLanguage(language.as_str().to_string())
        })?;

        let tree = match parser.parse(source_code, None) {
            Some(t) => t,
            None => {
                return Err(crate::errors::ParserError::ParseError(
                    "Failed to parse source".to_string(),
                ))
            }
        };

        let traversal = AstTraversal::new(&tree, language);

        // Simple kind-based filtering (could be extended with full query support)
        let mut results = Vec::new();
        traversal.visit_matching(|node, _| {
            if node.kind() == query_string || node.kind().contains(query_string) {
                results.push(node.kind().to_string());
            }
            Ok(true)
        })?;

        Ok(Vec::new())
    }

    /// Get parser statistics
    pub fn get_statistics(&mut self, file_paths: &[&str]) -> ParserResult<ParseStatistics> {
        let start = Instant::now();
        let results = self.parse_files(file_paths)?;

        let total_nodes: usize = results.iter().map(|r| r.total_nodes).sum();
        let files_processed = results.len();
        let total_lines: usize = results
            .iter()
            .flat_map(|r| r.nodes.iter())
            .map(|n| n.line_range.1 - n.line_range.0)
            .sum();

        let mut node_counts: HashMap<String, usize> = HashMap::new();
        for result in &results {
            for node in &result.nodes {
                *node_counts.entry(node.node_type.as_str().to_string()).or_insert(0) += 1;
            }
        }

        let errors: Vec<String> = results
            .iter()
            .filter_map(|r| r.error.clone())
            .collect();

        Ok(ParseStatistics {
            total_nodes,
            node_counts,
            parse_duration_ms: start.elapsed().as_millis(),
            files_processed,
            total_lines,
            errors_encountered: errors,
        })
    }
}

impl Default for SemanticParser {
    fn default() -> Self {
        Self::new().expect("Failed to create default semantic parser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_rust_code() {
        let code = r#"
        fn main() {
            println!("Hello, world!");
        }
        "#;

        let mut parser = SemanticParser::new().unwrap();
        let result = parser.parse_source(code, Language::Rust, "main.rs").unwrap();

        assert_eq!(result.language, Language::Rust);
        assert!(!result.nodes.is_empty());
    }

    #[test]
    fn test_parse_python_code() {
        let code = r#"
        def hello():
            print("Hello, world!")
        "#;

        let mut parser = SemanticParser::new().unwrap();
        let result = parser.parse_source(code, Language::Python, "main.py").unwrap();

        assert_eq!(result.language, Language::Python);
    }

    #[test]
    fn test_detect_language() {
        let mut parser = SemanticParser::new().unwrap();

        assert_eq!(parser.detect_language("test.rs").unwrap(), Language::Rust);
        assert_eq!(parser.detect_language("test.py").unwrap(), Language::Python);
        assert_eq!(parser.detect_language("test.js").unwrap(), Language::JavaScript);
        assert_eq!(parser.detect_language("test.ts").unwrap(), Language::TypeScript);
    }
}
