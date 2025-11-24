/// Language grammar initialization and management
use crate::errors::{ParserError, ParserResult};
use crate::types::Language;
use once_cell::sync::Lazy;
use std::sync::RwLock;
use tree_sitter::{Language as TreeSitterLanguage, Parser};

/// Grammar cache state
pub struct GrammarCache {
    pub rust_loaded: bool,
    pub python_loaded: bool,
    pub javascript_loaded: bool,
    pub typescript_loaded: bool,
}

impl Default for GrammarCache {
    fn default() -> Self {
        GrammarCache {
            rust_loaded: false,
            python_loaded: false,
            javascript_loaded: false,
            typescript_loaded: false,
        }
    }
}

/// Global grammar cache state
static GRAMMAR_CACHE: Lazy<RwLock<GrammarCache>> = Lazy::new(|| RwLock::new(GrammarCache::default()));

/// Load the grammar for a specific language
pub fn load_grammar(language: Language) -> ParserResult<TreeSitterLanguage> {
    let result = match language {
        Language::Rust => tree_sitter_rust::language(),
        Language::Python => tree_sitter_python::language(),
        Language::JavaScript => tree_sitter_javascript::language(),
        Language::TypeScript => tree_sitter_typescript::language_typescript(),
    };

    // Mark as loaded in cache
    let mut cache = GRAMMAR_CACHE.write().map_err(|e| {
        ParserError::LanguageInitError(format!("Failed to acquire grammar cache lock: {}", e))
    })?;

    match language {
        Language::Rust => cache.rust_loaded = true,
        Language::Python => cache.python_loaded = true,
        Language::JavaScript => cache.javascript_loaded = true,
        Language::TypeScript => cache.typescript_loaded = true,
    }

    Ok(result)
}

/// Create a new Tree-Sitter parser with the specified language
pub fn create_parser(language: Language) -> ParserResult<Parser> {
    let grammar = load_grammar(language)?;
    let mut parser = Parser::new();

    parser.set_language(grammar).map_err(|e| {
        ParserError::LanguageInitError(format!("Failed to set language for parser: {}", e))
    })?;

    Ok(parser)
}

/// Check if a grammar has been loaded
pub fn is_grammar_loaded(language: Language) -> bool {
    if let Ok(cache) = GRAMMAR_CACHE.read() {
        match language {
            Language::Rust => cache.rust_loaded,
            Language::Python => cache.python_loaded,
            Language::JavaScript => cache.javascript_loaded,
            Language::TypeScript => cache.typescript_loaded,
        }
    } else {
        false
    }
}

/// Initialize all supported language grammars
pub fn initialize_grammars(languages: &[Language]) -> ParserResult<()> {
    for lang in languages {
        load_grammar(*lang)?;
        tracing::info!("Grammar loaded for {}", lang.as_str());
    }
    Ok(())
}

/// Clear the grammar cache
pub fn clear_grammar_cache() -> ParserResult<()> {
    let mut cache = GRAMMAR_CACHE.write().map_err(|e| {
        ParserError::LanguageInitError(format!("Failed to acquire grammar cache lock: {}", e))
    })?;

    cache.rust_loaded = false;
    cache.python_loaded = false;
    cache.javascript_loaded = false;
    cache.typescript_loaded = false;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_rust_grammar() {
        let result = load_grammar(Language::Rust);
        assert!(result.is_ok());
    }

    #[test]
    fn test_load_python_grammar() {
        let result = load_grammar(Language::Python);
        assert!(result.is_ok());
    }

    #[test]
    fn test_create_parser() {
        let result = create_parser(Language::Rust);
        assert!(result.is_ok());
    }

    #[test]
    fn test_initialize_all_grammars() {
        let languages = vec![
            Language::Rust,
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
        ];
        let result = initialize_grammars(&languages);
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_grammar_loaded() {
        // Clear cache first to ensure clean state
        let _ = clear_grammar_cache();

        // Initially, grammars should not be loaded
        assert!(!is_grammar_loaded(Language::Rust));
        assert!(!is_grammar_loaded(Language::Python));

        // Load a grammar
        let _ = load_grammar(Language::Rust);
        assert!(is_grammar_loaded(Language::Rust));
        assert!(!is_grammar_loaded(Language::Python));

        // Load another grammar
        let _ = load_grammar(Language::Python);
        assert!(is_grammar_loaded(Language::Rust));
        assert!(is_grammar_loaded(Language::Python));
    }

    #[test]
    fn test_clear_grammar_cache() {
        // Load all grammars
        let languages = vec![
            Language::Rust,
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
        ];
        let _ = initialize_grammars(&languages);

        // Verify all are loaded
        assert!(is_grammar_loaded(Language::Rust));
        assert!(is_grammar_loaded(Language::Python));
        assert!(is_grammar_loaded(Language::JavaScript));
        assert!(is_grammar_loaded(Language::TypeScript));

        // Clear the cache
        let result = clear_grammar_cache();
        assert!(result.is_ok());

        // Verify all are cleared
        assert!(!is_grammar_loaded(Language::Rust));
        assert!(!is_grammar_loaded(Language::Python));
        assert!(!is_grammar_loaded(Language::JavaScript));
        assert!(!is_grammar_loaded(Language::TypeScript));
    }

    #[test]
    fn test_load_all_language_grammars() {
        let languages = vec![
            Language::Rust,
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
        ];

        for lang in languages {
            let result = load_grammar(lang);
            assert!(result.is_ok(), "Failed to load grammar for {}", lang.as_str());
            assert!(
                is_grammar_loaded(lang),
                "Grammar not marked as loaded for {}",
                lang.as_str()
            );
        }
    }

    #[test]
    fn test_create_parser_all_languages() {
        let languages = vec![
            Language::Rust,
            Language::Python,
            Language::JavaScript,
            Language::TypeScript,
        ];

        for lang in languages {
            let result = create_parser(lang);
            assert!(result.is_ok(), "Failed to create parser for {}", lang.as_str());
        }
    }
}
