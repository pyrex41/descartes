use anyhow::Result;
use colored::Colorize;
use descartes_core::{
    ActorType, DescaratesConfig, Event, Message, MessageRole, ModelBackend, ModelRequest,
    ProviderFactory,
};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::io::{self, BufRead};
use std::time::SystemTime;
use tracing::{info, warn};
use uuid::Uuid;

pub async fn execute(
    config: &DescaratesConfig,
    task: &str,
    provider: Option<&str>,
    model: Option<&str>,
    system: Option<&str>,
    stream: bool,
) -> Result<()> {
    println!("{}", "Spawning agent...".green().bold());
    println!("  Task: {}", task.cyan());

    // Determine provider and model
    let provider_name = provider.unwrap_or(&config.providers.primary);
    let model_name = get_model_for_provider(config, provider_name, model)?;

    println!("  Provider: {}", provider_name.yellow());
    println!("  Model: {}", model_name.yellow());

    if let Some(sys) = system {
        println!("  System: {}", sys);
    }

    // Check for piped input
    let mut full_task = task.to_string();
    if !atty::is(atty::Stream::Stdin) {
        println!("\n{}", "Reading from stdin...".dim());
        let stdin = io::stdin();
        let mut piped_content = String::new();
        for line in stdin.lock().lines() {
            piped_content.push_str(&line?);
            piped_content.push('\n');
        }
        if !piped_content.is_empty() {
            full_task = format!("{}\n\nInput:\n{}", task, piped_content);
        }
    }

    // Create provider backend
    let backend = create_backend(config, provider_name, &model_name).await?;

    // Create model request
    let mut messages = vec![Message {
        role: MessageRole::User,
        content: full_task.clone(),
    }];

    let request = ModelRequest {
        messages,
        model: model_name.clone(),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        system_prompt: system.map(|s| s.to_string()),
        tools: None,
    };

    // Execute with streaming or non-streaming
    if stream {
        execute_streaming(&backend, request).await?;
    } else {
        execute_non_streaming(&backend, request).await?;
    }

    println!("\n{}", "Agent execution completed.".green().bold());

    Ok(())
}

async fn execute_streaming(backend: &Box<dyn ModelBackend>, request: ModelRequest) -> Result<()> {
    println!("\n{}", "Streaming response:".green());
    println!("{}", "─".repeat(80).dim());

    let mut stream = backend.stream(request).await?;

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                print!("{}", response.content);
                use std::io::Write;
                io::stdout().flush()?;
            }
            Err(e) => {
                warn!("Stream error: {}", e);
                break;
            }
        }
    }

    println!("\n{}", "─".repeat(80).dim());
    Ok(())
}

async fn execute_non_streaming(
    backend: &Box<dyn ModelBackend>,
    request: ModelRequest,
) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );
    spinner.set_message("Waiting for response...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let response = backend.complete(request).await?;

    spinner.finish_and_clear();

    println!("\n{}", "Response:".green());
    println!("{}", "─".repeat(80).dim());
    println!("{}", response.content);
    println!("{}", "─".repeat(80).dim());

    if let Some(tokens) = response.tokens_used {
        println!("\nTokens used: {}", tokens.to_string().cyan());
    }

    Ok(())
}

async fn create_backend(
    config: &DescaratesConfig,
    provider: &str,
    model: &str,
) -> Result<Box<dyn ModelBackend>> {
    info!("Creating backend for provider: {}", provider);

    let factory = ProviderFactory::new(config.clone());
    let backend = factory.create_backend(provider, model).await?;

    // Health check
    match backend.health_check().await {
        Ok(true) => {
            info!("Backend health check passed");
        }
        Ok(false) => {
            warn!("Backend health check failed, but continuing...");
        }
        Err(e) => {
            warn!("Backend health check error: {}, continuing anyway", e);
        }
    }

    Ok(backend)
}

fn get_model_for_provider(
    config: &DescaratesConfig,
    provider: &str,
    model: Option<&str>,
) -> Result<String> {
    if let Some(m) = model {
        return Ok(m.to_string());
    }

    // Get default model for provider
    match provider {
        "anthropic" => Ok(config.providers.anthropic.model.clone()),
        "openai" => Ok(config.providers.openai.model.clone()),
        "ollama" => Ok(config.providers.ollama.model.clone()),
        "deepseek" => Ok(config.providers.deepseek.model.clone()),
        "groq" => Ok(config.providers.groq.model.clone()),
        _ => anyhow::bail!("Unknown provider: {}", provider),
    }
}
