# Descartes Provider System - Usage Examples

## Quick Start

### 1. Using the Anthropic API Provider

```rust
use descartes_core::{
    ProviderFactory, ModelRequest, Message, MessageRole,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create provider
    let mut config = HashMap::new();
    config.insert("api_key".to_string(),
        std::env::var("ANTHROPIC_API_KEY").unwrap());

    let mut provider = ProviderFactory::create("anthropic", config)?;
    provider.initialize().await?;

    // Create request
    let request = ModelRequest {
        messages: vec![
            Message {
                role: MessageRole::User,
                content: "Explain quantum computing in one sentence.".to_string(),
            }
        ],
        model: "claude-3-haiku-20240307".to_string(),
        max_tokens: Some(256),
        temperature: Some(0.7),
        system_prompt: None,
        tools: None,
    };

    // Get response
    let response = provider.complete(request).await?;
    println!("Response: {}", response.content);

    // Cleanup
    provider.shutdown().await?;

    Ok(())
}
```

### 2. Using the Claude Code CLI Adapter

```rust
use descartes_core::{
    ProviderFactory, ModelRequest, Message, MessageRole,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // No configuration needed for local Claude CLI
    let config = HashMap::new();

    let mut provider = ProviderFactory::create("claude-code-cli", config)?;
    provider.initialize().await?;

    // Check health before use
    if !provider.health_check().await? {
        eprintln!("Claude CLI not available");
        return Ok(());
    }

    let request = ModelRequest {
        messages: vec![
            Message {
                role: MessageRole::System,
                content: "You are a code review expert.".to_string(),
            },
            Message {
                role: MessageRole::User,
                content: "Review this Rust function: fn add(a: i32, b: i32) -> i32 { a + b }".to_string(),
            }
        ],
        model: "claude".to_string(),
        max_tokens: Some(512),
        temperature: Some(0.5),
        system_prompt: None,
        tools: None,
    };

    let response = provider.complete(request).await?;
    println!("{}", response.content);

    provider.shutdown().await?;
    Ok(())
}
```

### 3. Using the Ollama Local Provider

```rust
use descartes_core::{
    ProviderFactory, ModelRequest, Message, MessageRole,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = HashMap::new();
    // Optional: customize endpoint and timeout
    config.insert("endpoint".to_string(), "http://localhost:11434".to_string());
    config.insert("timeout_secs".to_string(), "300".to_string());

    let mut provider = ProviderFactory::create("ollama", config)?;
    provider.initialize().await?;

    // List available models
    let models = provider.list_models().await?;
    println!("Available models: {:?}", models);

    // Use a model
    if models.iter().any(|m| m.contains("mistral")) {
        let request = ModelRequest {
            messages: vec![
                Message {
                    role: MessageRole::User,
                    content: "Write a Python hello world.".to_string(),
                }
            ],
            model: "mistral".to_string(),
            max_tokens: Some(256),
            temperature: None,
            system_prompt: None,
            tools: None,
        };

        let response = provider.complete(request).await?;
        println!("Response:\n{}", response.content);
    }

    provider.shutdown().await?;
    Ok(())
}
```

### 4. Using OpenAI API Provider

```rust
use descartes_core::{
    ProviderFactory, ModelRequest, Message, MessageRole,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = HashMap::new();
    config.insert("api_key".to_string(),
        std::env::var("OPENAI_API_KEY").unwrap());
    config.insert("endpoint".to_string(),
        "https://api.openai.com/v1".to_string());

    let mut provider = ProviderFactory::create("openai", config)?;
    provider.initialize().await?;

    let request = ModelRequest {
        messages: vec![
            Message {
                role: MessageRole::User,
                content: "What is 2+2?".to_string(),
            }
        ],
        model: "gpt-3.5-turbo".to_string(),
        max_tokens: Some(100),
        temperature: Some(0.0),
        system_prompt: None,
        tools: None,
    };

    let response = provider.complete(request).await?;
    println!("GPT says: {}", response.content);

    provider.shutdown().await?;
    Ok(())
}
```

### 5. Generic Headless CLI Adapter

```rust
use descartes_core::{
    ProviderFactory, ModelRequest, Message, MessageRole,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = HashMap::new();
    config.insert("command".to_string(), "opencode".to_string());
    config.insert("args".to_string(), "serve,--port=8000".to_string());

    let mut provider = ProviderFactory::create("headless-cli", config)?;
    provider.initialize().await?;

    let request = ModelRequest {
        messages: vec![
            Message {
                role: MessageRole::User,
                content: "Generate a test file".to_string(),
            }
        ],
        model: "default".to_string(),
        max_tokens: Some(1024),
        temperature: None,
        system_prompt: None,
        tools: None,
    };

    let response = provider.complete(request).await?;
    println!("{}", response.content);

    provider.shutdown().await?;
    Ok(())
}
```

## Advanced Usage

### Provider Selection Based on Task Complexity

```rust
use descartes_core::ProviderFactory;
use std::collections::HashMap;

async fn select_provider_for_task(
    task_complexity: &str,
) -> Result<Box<dyn descartes_core::ModelBackend>, Box<dyn std::error::Error>> {
    match task_complexity {
        "simple" => {
            // Use fast, cheap model
            let config = HashMap::new();
            Ok(ProviderFactory::create("ollama", config)?)
        },
        "medium" => {
            // Use balanced model
            let mut config = HashMap::new();
            config.insert("api_key".to_string(),
                std::env::var("ANTHROPIC_API_KEY")?);
            Ok(ProviderFactory::create("anthropic", config)?)
        },
        "complex" => {
            // Use best model
            let mut config = HashMap::new();
            config.insert("api_key".to_string(),
                std::env::var("OPENAI_API_KEY")?);
            Ok(ProviderFactory::create("openai", config)?)
        },
        _ => Err("Unknown complexity level".into()),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut provider = select_provider_for_task("medium").await?;
    provider.initialize().await?;

    // Use provider...

    Ok(())
}
```

### Provider Health Checks and Fallback

```rust
use descartes_core::ProviderFactory;
use std::collections::HashMap;

async fn get_healthy_provider(
) -> Result<Box<dyn descartes_core::ModelBackend>, Box<dyn std::error::Error>> {
    let providers = vec!["claude-code-cli", "anthropic", "ollama"];

    for provider_name in providers {
        let config = match provider_name {
            "anthropic" => {
                let mut c = HashMap::new();
                c.insert("api_key".to_string(),
                    std::env::var("ANTHROPIC_API_KEY").ok().unwrap_or_default());
                c
            },
            _ => HashMap::new(),
        };

        match ProviderFactory::create(provider_name, config) {
            Ok(mut provider) => {
                if provider.initialize().await.is_ok()
                    && provider.health_check().await.unwrap_or(false) {
                    println!("Using provider: {}", provider.name());
                    return Ok(provider);
                }
            },
            Err(_) => continue,
        }
    }

    Err("No healthy providers available".into())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut provider = get_healthy_provider().await?;
    // Use provider with confidence it's healthy
    Ok(())
}
```

### Token Estimation for Cost Planning

```rust
use descartes_core::ProviderFactory;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = HashMap::new();
    config.insert("api_key".to_string(),
        std::env::var("ANTHROPIC_API_KEY")?);

    let provider = ProviderFactory::create("anthropic", config)?;

    let prompt = "Write a comprehensive guide on machine learning";
    let tokens = provider.estimate_tokens(prompt).await?;

    println!("Estimated tokens: {}", tokens);
    println!("Estimated cost: ${:.4}", (tokens as f32) * 0.003 / 1000.0);

    Ok(())
}
```

### Model Enumeration

```rust
use descartes_core::ProviderFactory;
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Local models from Ollama
    let mut provider = ProviderFactory::create("ollama", HashMap::new())?;
    let models = provider.list_models().await?;
    println!("Ollama models: {:?}", models);

    Ok(())
}
```

## Error Handling

### Comprehensive Error Handling Example

```rust
use descartes_core::{
    ProviderFactory, ModelRequest, Message, MessageRole, ProviderError,
};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = HashMap::new();
    config.insert("api_key".to_string(),
        std::env::var("ANTHROPIC_API_KEY").unwrap_or_default());

    let mut provider = match ProviderFactory::create("anthropic", config) {
        Ok(p) => p,
        Err(ProviderError::ConfigError(msg)) => {
            eprintln!("Configuration error: {}", msg);
            return Err(format!("Failed to configure provider: {}", msg).into());
        },
        Err(e) => {
            eprintln!("Provider creation error: {}", e);
            return Err(e.into());
        }
    };

    if let Err(e) = provider.initialize().await {
        eprintln!("Initialization failed: {}", e);
        match e {
            descartes_core::AgentError::ProviderError(
                ProviderError::AuthenticationError(_)
            ) => {
                eprintln!("Check your API key");
            },
            _ => {},
        }
        return Err(e.into());
    }

    let request = ModelRequest {
        messages: vec![
            Message {
                role: MessageRole::User,
                content: "Hello".to_string(),
            }
        ],
        model: "claude-3-opus-20240229".to_string(),
        max_tokens: Some(256),
        temperature: None,
        system_prompt: None,
        tools: None,
    };

    match provider.complete(request).await {
        Ok(response) => println!("Success: {}", response.content),
        Err(descartes_core::AgentError::ProviderError(
            ProviderError::RateLimited
        )) => {
            eprintln!("Rate limited - please retry in a moment");
        },
        Err(descartes_core::AgentError::ProviderError(
            ProviderError::Timeout
        )) => {
            eprintln!("Request timeout");
        },
        Err(e) => {
            eprintln!("Request failed: {}", e);
        }
    }

    provider.shutdown().await?;
    Ok(())
}
```

## Environment Setup

### macOS/Linux

```bash
# Set API keys
export OPENAI_API_KEY="sk-xxxx"
export ANTHROPIC_API_KEY="sk-ant-xxxx"

# Install Claude Code CLI
brew install descartes/descartes/claude

# Install Ollama
brew install ollama
ollama pull mistral  # Download a model
```

### Starting Services

```bash
# Ollama (if using local mode)
ollama serve

# In another terminal, your Rust app
cargo run --release
```

## Testing Providers

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use descartes_core::ProviderFactory;
    use std::collections::HashMap;

    #[test]
    fn test_anthropic_factory() {
        let mut config = HashMap::new();
        config.insert("api_key".to_string(), "test-key".to_string());

        let result = ProviderFactory::create("anthropic", config);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_ollama_health_check() {
        let provider = ProviderFactory::create("ollama", HashMap::new())
            .expect("Failed to create provider");

        let is_healthy = provider.health_check().await
            .unwrap_or(false);

        if !is_healthy {
            eprintln!("Ollama not available - skipping test");
        }
    }
}
```

## Performance Tips

1. **Reuse Providers**: Initialize once, reuse across multiple requests
2. **Batch Requests**: Use provider-specific batch APIs when available
3. **Model Selection**: Choose appropriate models for task complexity
4. **Token Management**: Estimate tokens to optimize costs
5. **Health Checks**: Implement periodic health checks for reliability

## Next Steps

See `PROVIDER_DESIGN.md` for comprehensive architectural documentation.
