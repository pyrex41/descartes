/// Language grammar initialization and management
use crate::errors::{ParserError, ParserResult};
use crate::types::Language;
use once_cell::sync::Lazy;
use std::sync::RwLock;
use tree_sitter::{Language as TreeSitterLanguage, Parser};

/// Grammar cache for loaded languages
pub struct GrammarCache {
    pub rust: Option<TreeSitterLanguage>,
    pub python: Option<TreeSitterLanguage>,
    pub javascript: Option<TreeSitterLanguage>,
    pub typescript: Option<TreeSitterLanguage>,
}

impl Default for GrammarCache {
    fn default() -> Self {
        GrammarCache {
            rust: None,
            python: None,
            javascript: None,
            typescript: None,
        }
    }
}

/// Global grammar cache
static GRAMMAR_CACHE: Lazy<RwLock<GrammarCache>> = Lazy::new(|| RwLock::new(GrammarCache::default()));

/// Load the grammar for a specific language
pub fn load_grammar(language: Language) -> ParserResult<TreeSitterLanguage> {
    let mut cache = GRAMMAR_CACHE.write().map_err(|e| {
        ParserError::LanguageInitError(format!("Failed to acquire grammar cache lock: {}", e))
    })?;

    let lang = match language {
        Language::Rust => {
            if cache.rust.is_none() {
                cache.rust = Some(unsafe { tree_sitter_rust::language() });
            }
            cache.rust
        }
        Language::Python => {
            if cache.python.is_none() {
                cache.python = Some(unsafe { tree_sitter_python::language() });
            }
            cache.python
        }
        Language::JavaScript => {
            if cache.javascript.is_none() {
                cache.javascript = Some(unsafe { tree_sitter_javascript::language() });
            }
            cache.javascript
        }
        Language::TypeScript => {
            if cache.typescript.is_none() {
                cache.typescript = Some(unsafe { tree_sitter_typescript::language() });
            }
            cache.typescript
        }
    };

    lang.ok_or_else(|| {
        ParserError::GrammarLoadError(format!("Failed to load {} grammar", language.as_str()))
    })
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

/// Get a cached grammar for a language
pub fn get_cached_grammar(language: Language) -> Option<TreeSitterLanguage> {
    let cache = GRAMMAR_CACHE.read().ok()?;

    match language {
        Language::Rust => cache.rust,
        Language::Python => cache.python,
        Language::JavaScript => cache.javascript,
        Language::TypeScript => cache.typescript,
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

    cache.rust = None;
    cache.python = None;
    cache.javascript = None;
    cache.typescript = None;

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
}
