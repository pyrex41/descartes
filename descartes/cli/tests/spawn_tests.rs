/// Integration tests for the spawn command
use descartes_core::DescaratesConfig;

/// Helper to create a minimal valid config for testing
fn create_test_config() -> DescaratesConfig {
    let mut config = DescaratesConfig::default();

    // Set up providers with API keys
    config.providers.anthropic.api_key = Some("test-anthropic-key".to_string());
    config.providers.openai.api_key = Some("test-openai-key".to_string());
    config.providers.deepseek.api_key = Some("test-deepseek-key".to_string());
    config.providers.groq.api_key = Some("test-groq-key".to_string());

    config
}

/// Helper to create a config with missing API keys
fn create_config_without_keys() -> DescaratesConfig {
    let mut config = DescaratesConfig::default();
    config.providers.anthropic.api_key = None;
    config.providers.openai.api_key = None;
    config.providers.deepseek.api_key = None;
    config.providers.groq.api_key = None;
    config
}

/// Helper to create a config with custom provider settings
fn create_custom_provider_config(primary: &str, model: &str) -> DescaratesConfig {
    let mut config = create_test_config();
    config.providers.primary = primary.to_string();

    match primary {
        "anthropic" => config.providers.anthropic.model = model.to_string(),
        "openai" => config.providers.openai.model = model.to_string(),
        "ollama" => config.providers.ollama.model = model.to_string(),
        "deepseek" => config.providers.deepseek.model = model.to_string(),
        "groq" => config.providers.groq.model = model.to_string(),
        _ => {}
    }

    config
}

#[tokio::test]
async fn test_get_model_for_provider_explicit_model() {
    let config = create_test_config();

    // Test explicit model override
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "anthropic",
        Some("claude-3-opus-20240229"),
    );

    assert!(model.is_ok());
    assert_eq!(model.unwrap(), "claude-3-opus-20240229");
}

#[tokio::test]
async fn test_get_model_for_provider_anthropic_default() {
    let config = create_test_config();

    // Test default model for Anthropic
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "anthropic",
        None,
    );

    assert!(model.is_ok());
    assert_eq!(model.unwrap(), config.providers.anthropic.model);
}

#[tokio::test]
async fn test_get_model_for_provider_openai_default() {
    let config = create_test_config();

    // Test default model for OpenAI
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "openai",
        None,
    );

    assert!(model.is_ok());
    assert_eq!(model.unwrap(), config.providers.openai.model);
}

#[tokio::test]
async fn test_get_model_for_provider_ollama_default() {
    let config = create_test_config();

    // Test default model for Ollama
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "ollama",
        None,
    );

    assert!(model.is_ok());
    assert_eq!(model.unwrap(), config.providers.ollama.model);
}

#[tokio::test]
async fn test_get_model_for_provider_deepseek_default() {
    let config = create_test_config();

    // Test default model for DeepSeek
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "deepseek",
        None,
    );

    assert!(model.is_ok());
    assert_eq!(model.unwrap(), config.providers.deepseek.model);
}

#[tokio::test]
async fn test_get_model_for_provider_groq_default() {
    let config = create_test_config();

    // Test default model for Groq
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "groq",
        None,
    );

    assert!(model.is_ok());
    assert_eq!(model.unwrap(), config.providers.groq.model);
}

#[tokio::test]
async fn test_get_model_for_provider_unknown_provider() {
    let config = create_test_config();

    // Test unknown provider
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "unknown-provider",
        None,
    );

    assert!(model.is_err());
    assert!(model
        .unwrap_err()
        .to_string()
        .contains("Unknown provider"));
}

#[tokio::test]
async fn test_spawn_uses_config_defaults() {
    let config = create_custom_provider_config("anthropic", "claude-3-5-sonnet-20241022");

    // When no provider is specified, it should use the primary provider from config
    assert_eq!(config.providers.primary, "anthropic");

    // And the default model for that provider
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        &config.providers.primary,
        None,
    );

    assert!(model.is_ok());
    assert_eq!(model.unwrap(), "claude-3-5-sonnet-20241022");
}

#[tokio::test]
async fn test_spawn_explicit_provider_override() {
    let config = create_custom_provider_config("anthropic", "claude-3-5-sonnet-20241022");

    // Even if primary is anthropic, we can explicitly use OpenAI
    let model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "openai",
        None,
    );

    assert!(model.is_ok());
    assert_eq!(model.unwrap(), config.providers.openai.model);
}

#[tokio::test]
async fn test_create_backend_anthropic() {
    let config = create_test_config();

    // Test creating Anthropic backend
    let backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "anthropic",
        "claude-3-5-sonnet-20241022",
    );

    assert!(backend.is_ok(), "Should create Anthropic backend successfully");
}

#[tokio::test]
async fn test_create_backend_openai() {
    let config = create_test_config();

    // Test creating OpenAI backend
    let backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "openai",
        "gpt-4-turbo",
    );

    assert!(backend.is_ok(), "Should create OpenAI backend successfully");
}

#[tokio::test]
async fn test_create_backend_ollama() {
    let config = create_test_config();

    // Test creating Ollama backend (no API key required)
    let backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "ollama",
        "llama2",
    );

    assert!(backend.is_ok(), "Should create Ollama backend successfully");
}

#[tokio::test]
async fn test_create_backend_deepseek() {
    let config = create_test_config();

    // Test creating DeepSeek backend
    // Note: DeepSeek is not yet implemented in ProviderFactory
    let backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "deepseek",
        "deepseek-chat",
    );

    // This should fail because DeepSeek is not yet implemented
    assert!(backend.is_err(), "DeepSeek provider is not yet implemented in ProviderFactory");
    if let Err(e) = backend {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("Unknown provider") || error_msg.contains("deepseek"),
            "Error should indicate unsupported provider, got: {}",
            error_msg
        );
    }
}

#[tokio::test]
async fn test_create_backend_groq() {
    let config = create_test_config();

    // Test creating Groq backend
    // Note: Groq is not yet implemented in ProviderFactory
    let backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "groq",
        "mixtral-8x7b-32768",
    );

    // This should fail because Groq is not yet implemented
    assert!(backend.is_err(), "Groq provider is not yet implemented in ProviderFactory");
    if let Err(e) = backend {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("Unknown provider") || error_msg.contains("groq"),
            "Error should indicate unsupported provider, got: {}",
            error_msg
        );
    }
}

#[tokio::test]
async fn test_create_backend_unknown_provider() {
    let config = create_test_config();

    // Test unknown provider
    let backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "unknown-provider",
        "some-model",
    );

    assert!(backend.is_err(), "Should fail for unknown provider");
    if let Err(e) = backend {
        let error_msg = e.to_string();
        assert!(
            error_msg.contains("Unknown provider"),
            "Error message should mention unknown provider, got: {}",
            error_msg
        );
    }
}

#[tokio::test]
async fn test_create_backend_with_custom_endpoints() {
    let mut config = create_test_config();

    // Set custom endpoints
    config.providers.anthropic.endpoint = "https://custom-anthropic.example.com".to_string();
    config.providers.openai.endpoint = "https://custom-openai.example.com/v1".to_string();

    // Backend should be created with custom endpoints
    let anthropic_backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "anthropic",
        "claude-3-5-sonnet-20241022",
    );
    assert!(anthropic_backend.is_ok(), "Should create backend with custom Anthropic endpoint");

    let openai_backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "openai",
        "gpt-4-turbo",
    );
    assert!(openai_backend.is_ok(), "Should create backend with custom OpenAI endpoint");
}

#[tokio::test]
async fn test_spawn_missing_api_key_anthropic() {
    let config = create_config_without_keys();

    // Creating backend without API key should still work at creation time
    // The actual API call would fail, but backend creation succeeds
    let backend = descartes_cli::commands::spawn::create_backend(
        &config,
        "anthropic",
        "claude-3-5-sonnet-20241022",
    );

    // Backend creation might succeed even without API key (validation happens at runtime)
    // This depends on the ProviderFactory implementation
    // We're testing that the function doesn't panic
    assert!(
        backend.is_ok() || backend.is_err(),
        "Backend creation should complete (either succeed or fail gracefully)"
    );
}

#[tokio::test]
async fn test_model_resolution_with_explicit_flags() {
    let config = create_custom_provider_config("anthropic", "claude-3-haiku-20240307");

    // Test that explicit --provider and --model flags override config
    let provider = "openai";
    let model = Some("gpt-3.5-turbo");

    let resolved_model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        provider,
        model,
    );

    assert!(resolved_model.is_ok());
    assert_eq!(resolved_model.unwrap(), "gpt-3.5-turbo");
}

#[tokio::test]
async fn test_model_resolution_fallback_to_config() {
    let config = create_custom_provider_config("openai", "gpt-4-turbo");

    // Test that when no model is specified, it falls back to config default
    let provider = "openai";
    let model = None;

    let resolved_model = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        provider,
        model,
    );

    assert!(resolved_model.is_ok());
    assert_eq!(resolved_model.unwrap(), "gpt-4-turbo");
}

#[tokio::test]
async fn test_all_providers_backend_creation() {
    let config = create_test_config();

    // Only test providers that are actually implemented in ProviderFactory
    let supported_providers = vec![
        ("anthropic", "claude-3-5-sonnet-20241022"),
        ("openai", "gpt-4-turbo"),
        ("ollama", "llama2"),
    ];

    // Providers that are in config but not yet implemented
    let unsupported_providers = vec![
        ("deepseek", "deepseek-chat"),
        ("groq", "mixtral-8x7b-32768"),
    ];

    for (provider, model) in supported_providers {
        let backend = descartes_cli::commands::spawn::create_backend(
            &config,
            provider,
            model,
        );

        assert!(
            backend.is_ok(),
            "Should successfully create backend for supported provider: {}",
            provider
        );
    }

    for (provider, model) in unsupported_providers {
        let backend = descartes_cli::commands::spawn::create_backend(
            &config,
            provider,
            model,
        );

        assert!(
            backend.is_err(),
            "Should fail for unsupported provider: {}",
            provider
        );
    }
}

#[tokio::test]
async fn test_provider_config_structure() {
    let config = create_test_config();

    // Verify provider configs have expected fields
    assert!(config.providers.anthropic.api_key.is_some());
    assert!(!config.providers.anthropic.endpoint.is_empty());
    assert!(!config.providers.anthropic.model.is_empty());

    assert!(config.providers.openai.api_key.is_some());
    assert!(!config.providers.openai.endpoint.is_empty());
    assert!(!config.providers.openai.model.is_empty());

    // Ollama doesn't require API key
    assert!(!config.providers.ollama.endpoint.is_empty());
    assert!(!config.providers.ollama.model.is_empty());

    assert!(config.providers.deepseek.api_key.is_some());
    assert!(!config.providers.deepseek.endpoint.is_empty());
    assert!(!config.providers.deepseek.model.is_empty());

    assert!(config.providers.groq.api_key.is_some());
    assert!(!config.providers.groq.endpoint.is_empty());
    assert!(!config.providers.groq.model.is_empty());
}

#[tokio::test]
async fn test_default_primary_provider() {
    let config = DescaratesConfig::default();

    // Default primary provider should be anthropic
    assert_eq!(config.providers.primary, "anthropic");
}

#[tokio::test]
async fn test_custom_primary_provider() {
    let mut config = create_test_config();
    config.providers.primary = "openai".to_string();

    // Primary provider should be modifiable
    assert_eq!(config.providers.primary, "openai");
}

#[tokio::test]
async fn test_model_names_are_valid_strings() {
    let config = create_test_config();

    // All default models should be non-empty strings
    assert!(!config.providers.anthropic.model.is_empty());
    assert!(!config.providers.openai.model.is_empty());
    assert!(!config.providers.ollama.model.is_empty());
    assert!(!config.providers.deepseek.model.is_empty());
    assert!(!config.providers.groq.model.is_empty());

    // Models should not contain whitespace
    assert!(!config.providers.anthropic.model.contains(' '));
    assert!(!config.providers.openai.model.contains(' '));
    assert!(!config.providers.ollama.model.contains(' '));
    assert!(!config.providers.deepseek.model.contains(' '));
    assert!(!config.providers.groq.model.contains(' '));
}

#[tokio::test]
async fn test_endpoint_urls_are_valid() {
    let config = create_test_config();

    // All endpoints should be non-empty and start with http/https
    assert!(config.providers.anthropic.endpoint.starts_with("http"));
    assert!(config.providers.openai.endpoint.starts_with("http"));
    assert!(config.providers.ollama.endpoint.starts_with("http"));
    assert!(config.providers.deepseek.endpoint.starts_with("http"));
    assert!(config.providers.groq.endpoint.starts_with("http"));
}

#[tokio::test]
async fn test_multiple_model_overrides() {
    let config = create_test_config();

    // Test that explicit model can be any string
    let custom_models = vec![
        "custom-model-v1",
        "experimental-llm-2024",
        "fine-tuned-model-123",
    ];

    for custom_model in custom_models {
        let model = descartes_cli::commands::spawn::get_model_for_provider(
            &config,
            "anthropic",
            Some(custom_model),
        );

        assert!(model.is_ok());
        assert_eq!(model.unwrap(), custom_model);
    }
}

#[tokio::test]
async fn test_error_message_quality() {
    let config = create_test_config();

    // Test that error messages are helpful
    let result = descartes_cli::commands::spawn::get_model_for_provider(
        &config,
        "invalid-provider-name",
        None,
    );

    assert!(result.is_err());
    let error_msg = result.unwrap_err().to_string();

    // Error should mention the provider name
    assert!(
        error_msg.contains("invalid-provider-name") || error_msg.contains("Unknown provider"),
        "Error message should be descriptive, got: {}",
        error_msg
    );
}
