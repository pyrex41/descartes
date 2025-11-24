/// Integration tests for model providers.
#[cfg(test)]
mod tests {
    use crate::providers::*;
    use crate::traits::*;
    use std::collections::HashMap;

    #[test]
    fn test_openai_provider_creation() {
        let provider = OpenAiProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "openai");
    }

    #[test]
    fn test_anthropic_provider_creation() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        assert_eq!(provider.name(), "anthropic");
    }

    #[test]
    fn test_claude_code_adapter_creation() {
        let adapter = ClaudeCodeAdapter::new(None, None);
        assert_eq!(adapter.name(), "claude-code-cli");
    }

    #[test]
    fn test_ollama_provider_creation() {
        let provider = OllamaProvider::new(None, None);
        assert_eq!(provider.name(), "ollama");
    }

    #[test]
    fn test_headless_cli_adapter_creation() {
        let adapter = HeadlessCliAdapter::new("test-command".to_string(), vec![]);
        assert_eq!(adapter.name(), "headless-cli");
    }

    #[test]
    fn test_provider_factory_openai() {
        let mut config = HashMap::new();
        config.insert("api_key".to_string(), "test-key".to_string());

        let result = ProviderFactory::create("openai", config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_factory_anthropic() {
        let mut config = HashMap::new();
        config.insert("api_key".to_string(), "test-key".to_string());

        let result = ProviderFactory::create("anthropic", config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_factory_ollama() {
        let config = HashMap::new();
        let result = ProviderFactory::create("ollama", config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_factory_missing_api_key() {
        let config = HashMap::new();
        let result = ProviderFactory::create("openai", config);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_factory_unknown_provider() {
        let config = HashMap::new();
        let result = ProviderFactory::create("unknown-provider", config);
        assert!(result.is_err());
    }
}
