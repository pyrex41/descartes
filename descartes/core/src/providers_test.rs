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

    #[test]
    fn test_provider_factory_claude_code_cli() {
        let config = HashMap::new();
        let result = ProviderFactory::create("claude-code-cli", config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_factory_headless_cli() {
        let mut config = HashMap::new();
        config.insert("command".to_string(), "test-command".to_string());
        config.insert("args".to_string(), "arg1,arg2".to_string());

        let result = ProviderFactory::create("headless-cli", config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_provider_factory_headless_cli_missing_command() {
        let config = HashMap::new();
        let result = ProviderFactory::create("headless-cli", config);
        assert!(result.is_err());
    }

    #[test]
    fn test_provider_endpoint_extraction_openai() {
        let mut config = HashMap::new();
        config.insert("api_key".to_string(), "test-key".to_string());
        config.insert("endpoint".to_string(), "https://custom.openai.endpoint".to_string());

        let result = ProviderFactory::create("openai", config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        if let crate::traits::ModelProviderMode::Api { endpoint, .. } = provider.mode() {
            assert_eq!(endpoint, "https://custom.openai.endpoint");
        } else {
            panic!("Expected API mode");
        }
    }

    #[test]
    fn test_provider_endpoint_extraction_anthropic() {
        let mut config = HashMap::new();
        config.insert("api_key".to_string(), "test-key".to_string());
        config.insert("endpoint".to_string(), "https://custom.anthropic.endpoint".to_string());

        let result = ProviderFactory::create("anthropic", config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        if let crate::traits::ModelProviderMode::Api { endpoint, .. } = provider.mode() {
            assert_eq!(endpoint, "https://custom.anthropic.endpoint");
        } else {
            panic!("Expected API mode");
        }
    }

    #[test]
    fn test_provider_endpoint_extraction_ollama() {
        let mut config = HashMap::new();
        config.insert("endpoint".to_string(), "http://localhost:9999".to_string());

        let result = ProviderFactory::create("ollama", config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        if let crate::traits::ModelProviderMode::Local { endpoint, .. } = provider.mode() {
            assert_eq!(endpoint, "http://localhost:9999");
        } else {
            panic!("Expected Local mode");
        }
    }

    #[test]
    fn test_provider_config_extraction_timeout() {
        let mut config = HashMap::new();
        config.insert("endpoint".to_string(), "http://localhost:11434".to_string());
        config.insert("timeout_secs".to_string(), "600".to_string());

        let result = ProviderFactory::create("ollama", config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        if let crate::traits::ModelProviderMode::Local { timeout_secs, .. } = provider.mode() {
            assert_eq!(*timeout_secs, 600);
        } else {
            panic!("Expected Local mode with timeout");
        }
    }

    #[test]
    fn test_provider_config_extraction_headless_args() {
        let mut config = HashMap::new();
        config.insert("command".to_string(), "test-cmd".to_string());
        config.insert("args".to_string(), "arg1,arg2,arg3".to_string());

        let result = ProviderFactory::create("headless-cli", config);
        assert!(result.is_ok());

        let provider = result.unwrap();
        if let crate::traits::ModelProviderMode::Headless { command, args } = provider.mode() {
            assert_eq!(command, "test-cmd");
            assert_eq!(args.len(), 3);
            assert_eq!(args[0], "arg1");
            assert_eq!(args[1], "arg2");
            assert_eq!(args[2], "arg3");
        } else {
            panic!("Expected Headless mode");
        }
    }

    #[test]
    fn test_provider_mode_verification_openai() {
        let provider = OpenAiProvider::new("test-key".to_string(), None);
        matches!(provider.mode(), crate::traits::ModelProviderMode::Api { .. });
    }

    #[test]
    fn test_provider_mode_verification_anthropic() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        matches!(provider.mode(), crate::traits::ModelProviderMode::Api { .. });
    }

    #[test]
    fn test_provider_mode_verification_ollama() {
        let provider = OllamaProvider::new(None, None);
        matches!(provider.mode(), crate::traits::ModelProviderMode::Local { .. });
    }

    #[test]
    fn test_provider_mode_verification_claude_code() {
        let adapter = ClaudeCodeAdapter::new(None, None);
        matches!(adapter.mode(), crate::traits::ModelProviderMode::Headless { .. });
    }

    #[test]
    fn test_provider_mode_verification_headless_cli() {
        let adapter = HeadlessCliAdapter::new("test-cmd".to_string(), vec![]);
        matches!(adapter.mode(), crate::traits::ModelProviderMode::Headless { .. });
    }

    #[test]
    fn test_openai_provider_with_custom_endpoint() {
        let provider = OpenAiProvider::new(
            "test-key".to_string(),
            Some("https://custom.endpoint.com/v1".to_string())
        );
        assert_eq!(provider.name(), "openai");

        if let crate::traits::ModelProviderMode::Api { endpoint, .. } = provider.mode() {
            assert_eq!(endpoint, "https://custom.endpoint.com/v1");
        } else {
            panic!("Expected API mode");
        }
    }

    #[test]
    fn test_anthropic_provider_with_custom_endpoint() {
        let provider = AnthropicProvider::new(
            "test-key".to_string(),
            Some("https://custom.anthropic.com/v1".to_string())
        );
        assert_eq!(provider.name(), "anthropic");

        if let crate::traits::ModelProviderMode::Api { endpoint, .. } = provider.mode() {
            assert_eq!(endpoint, "https://custom.anthropic.com/v1");
        } else {
            panic!("Expected API mode");
        }
    }

    #[test]
    fn test_ollama_provider_with_custom_endpoint() {
        let provider = OllamaProvider::new(
            Some("http://192.168.1.100:11434".to_string()),
            Some(600)
        );
        assert_eq!(provider.name(), "ollama");

        if let crate::traits::ModelProviderMode::Local { endpoint, timeout_secs } = provider.mode() {
            assert_eq!(endpoint, "http://192.168.1.100:11434");
            assert_eq!(*timeout_secs, 600);
        } else {
            panic!("Expected Local mode");
        }
    }

    #[test]
    fn test_claude_code_adapter_with_custom_command() {
        let adapter = ClaudeCodeAdapter::new(
            Some("/usr/local/bin/claude".to_string()),
            Some(vec!["--model".to_string(), "claude-3".to_string()])
        );
        assert_eq!(adapter.name(), "claude-code-cli");

        if let crate::traits::ModelProviderMode::Headless { command, args } = adapter.mode() {
            assert_eq!(command, "/usr/local/bin/claude");
            assert_eq!(args.len(), 2);
        } else {
            panic!("Expected Headless mode");
        }
    }

    #[test]
    fn test_provider_factory_case_insensitive() {
        let mut config = HashMap::new();
        config.insert("api_key".to_string(), "test-key".to_string());

        // Test uppercase
        let result = ProviderFactory::create("OPENAI", config.clone());
        assert!(result.is_ok());

        // Test mixed case
        let result = ProviderFactory::create("OpenAI", config.clone());
        assert!(result.is_ok());

        // Test lowercase
        let result = ProviderFactory::create("openai", config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_openai_list_models() {
        let provider = OpenAiProvider::new("test-key".to_string(), None);
        let models = provider.list_models().await;
        assert!(models.is_ok());

        let model_list = models.unwrap();
        assert!(model_list.contains(&"gpt-4".to_string()));
        assert!(model_list.contains(&"gpt-4-turbo".to_string()));
        assert!(model_list.contains(&"gpt-3.5-turbo".to_string()));
    }

    #[tokio::test]
    async fn test_anthropic_list_models() {
        let provider = AnthropicProvider::new("test-key".to_string(), None);
        let models = provider.list_models().await;
        assert!(models.is_ok());

        let model_list = models.unwrap();
        assert!(model_list.contains(&"claude-3-opus-20240229".to_string()));
        assert!(model_list.contains(&"claude-3-sonnet-20240229".to_string()));
        assert!(model_list.contains(&"claude-3-haiku-20240307".to_string()));
    }

    #[tokio::test]
    async fn test_claude_code_list_models() {
        let adapter = ClaudeCodeAdapter::new(None, None);
        let models = adapter.list_models().await;
        assert!(models.is_ok());

        let model_list = models.unwrap();
        assert_eq!(model_list.len(), 1);
        assert_eq!(model_list[0], "claude");
    }

    #[tokio::test]
    async fn test_headless_cli_list_models() {
        let adapter = HeadlessCliAdapter::new("test-cmd".to_string(), vec![]);
        let models = adapter.list_models().await;
        assert!(models.is_ok());

        let model_list = models.unwrap();
        assert_eq!(model_list.len(), 1);
        assert_eq!(model_list[0], "default");
    }
}
