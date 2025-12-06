use anyhow::Result;
use colored::Colorize;
use descartes_core::{
    default_sessions_dir, get_system_prompt, get_tools, DescaratesConfig, Message, MessageRole,
    ModelBackend, ModelRequest, ProviderFactory, ToolLevel, TranscriptWriter,
};
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::path::PathBuf;
use tracing::{info, warn};

/// Parse tool level from string
fn parse_tool_level(level: &str, no_spawn: bool) -> ToolLevel {
    match level.to_lowercase().as_str() {
        "minimal" => ToolLevel::Minimal,
        "orchestrator" => {
            if no_spawn {
                // Sub-sessions cannot spawn their own sub-sessions
                ToolLevel::Minimal
            } else {
                ToolLevel::Orchestrator
            }
        }
        "readonly" => ToolLevel::ReadOnly,
        "researcher" => ToolLevel::Researcher,
        "planner" => ToolLevel::Planner,
        _ => {
            warn!("Unknown tool level '{}', defaulting to minimal", level);
            ToolLevel::Minimal
        }
    }
}

pub async fn execute(
    config: &DescaratesConfig,
    task: &str,
    provider: Option<&str>,
    model: Option<&str>,
    system: Option<&str>,
    stream: bool,
    tool_level: &str,
    no_spawn: bool,
    transcript_dir: Option<&str>,
) -> Result<()> {
    // Slick header
    println!();
    println!("{}", "┌─ descartes ─────────────────────────────────────┐".cyan());
    println!("{}", "│  4 tools. Full observability. Zero bloat.      │".cyan());
    println!("{}", "└─────────────────────────────────────────────────┘".cyan());
    println!();

    println!("{}", "Spawning agent...".green().bold());
    println!("  Task: {}", task.cyan());

    // Determine provider and model
    let provider_name = provider.unwrap_or(&config.providers.primary);
    let model_name = get_model_for_provider(config, provider_name, model)?;

    println!("  Provider: {}", provider_name.yellow());
    println!("  Model: {}", model_name.yellow());

    // Parse tool level (with recursive prevention)
    let level = parse_tool_level(tool_level, no_spawn);
    let level_str = match level {
        ToolLevel::Minimal => "minimal",
        ToolLevel::Orchestrator => "orchestrator",
        ToolLevel::ReadOnly => "readonly",
        ToolLevel::Researcher => "researcher",
        ToolLevel::Planner => "planner",
    };
    println!("  Tool level: {}", level_str.yellow());

    if no_spawn {
        println!("  {}", "(sub-session: spawn disabled)".dimmed());
    }

    // Get tools for this level
    let tools = get_tools(level);
    println!(
        "  Tools: {}",
        tools
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
            .dimmed()
    );

    // Determine system prompt
    let system_prompt = system
        .map(|s| s.to_string())
        .unwrap_or_else(|| get_system_prompt(level).to_string());

    // Initialize transcript writer
    let sessions_dir = transcript_dir
        .map(PathBuf::from)
        .unwrap_or_else(default_sessions_dir);

    let mut transcript = TranscriptWriter::new(
        &sessions_dir,
        provider_name,
        &model_name,
        task,
        None, // parent_session_id (set by orchestrator if this is a sub-session)
        Some(level_str),
    )?;

    println!(
        "  Transcript: {}",
        transcript.path().display().to_string().dimmed()
    );

    // Check for piped input
    let mut full_task = task.to_string();
    if !atty::is(atty::Stream::Stdin) {
        println!("\n{}", "Reading from stdin...".dimmed());
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

    // Create provider backend and initialize it
    let mut backend = create_backend(config, provider_name, &model_name)?;
    backend.initialize().await?;

    // Create model request with tools
    let messages = vec![Message {
        role: MessageRole::User,
        content: full_task.clone(),
    }];

    // Log user message to transcript
    transcript.add_user_message(&full_task);

    let request = ModelRequest {
        messages,
        model: model_name.clone(),
        max_tokens: Some(4096),
        temperature: Some(0.7),
        system_prompt: Some(system_prompt),
        tools: Some(tools),
    };

    // Execute with streaming or non-streaming
    // Fall back to non-streaming if provider doesn't support streaming
    let response_content = if stream {
        match execute_streaming(backend.as_ref(), request.clone()).await {
            Ok(content) => content,
            Err(e) if e.to_string().contains("Streaming not yet implemented")
                   || e.to_string().contains("Unsupported feature") => {
                println!("{}", "(streaming not supported, using non-streaming mode)".dimmed());
                execute_non_streaming(backend.as_ref(), request).await?
            }
            Err(e) => return Err(e),
        }
    } else {
        execute_non_streaming(backend.as_ref(), request).await?
    };

    // Log assistant response to transcript
    transcript.add_assistant_message(&response_content);

    // Save transcript
    let transcript_path = transcript.save()?;
    println!(
        "\n{} {}",
        "Transcript saved:".dimmed(),
        transcript_path.display()
    );

    println!("\n{}", "Agent execution completed.".green().bold());

    Ok(())
}

async fn execute_streaming(
    backend: &dyn ModelBackend,
    request: ModelRequest,
) -> Result<String> {
    println!("\n{}", "Streaming response:".green());
    println!("{}", "─".repeat(80).dimmed());

    let mut stream = backend.stream(request).await?;
    let mut full_response = String::new();

    while let Some(result) = stream.next().await {
        match result {
            Ok(response) => {
                print!("{}", response.content);
                full_response.push_str(&response.content);
                use std::io::Write;
                io::stdout().flush()?;
            }
            Err(e) => {
                warn!("Stream error: {}", e);
                break;
            }
        }
    }

    println!("\n{}", "─".repeat(80).dimmed());
    Ok(full_response)
}

async fn execute_non_streaming(
    backend: &dyn ModelBackend,
    request: ModelRequest,
) -> Result<String> {
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
    println!("{}", "─".repeat(80).dimmed());
    println!("{}", response.content);
    println!("{}", "─".repeat(80).dimmed());

    if let Some(tokens) = response.tokens_used {
        println!("\nTokens used: {}", tokens.to_string().cyan());
    }

    Ok(response.content)
}

pub fn create_backend(
    config: &DescaratesConfig,
    provider: &str,
    _model: &str,
) -> Result<Box<dyn ModelBackend>> {
    info!("Creating backend for provider: {}", provider);

    // Build config HashMap from DescaratesConfig
    let mut provider_config: HashMap<String, String> = HashMap::new();

    match provider {
        "anthropic" => {
            match &config.providers.anthropic.api_key {
                Some(api_key) if !api_key.is_empty() => {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
                _ => {
                    eprintln!();
                    eprintln!("{}", "✗ Anthropic API key not configured".red().bold());
                    eprintln!();
                    eprintln!("  To fix, set your API key:");
                    eprintln!("    {}", "export ANTHROPIC_API_KEY=sk-ant-...".cyan());
                    eprintln!();
                    eprintln!("  Or add to ~/.descartes/config.toml:");
                    eprintln!("    {}", "[providers.anthropic]".dimmed());
                    eprintln!("    {}", "api_key = \"sk-ant-...\"".dimmed());
                    eprintln!();
                    eprintln!("  Get your key at: {}", "https://console.anthropic.com".cyan());
                    eprintln!();
                    anyhow::bail!("Anthropic API key not configured");
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.anthropic.endpoint.clone(),
            );
        }
        "openai" => {
            match &config.providers.openai.api_key {
                Some(api_key) if !api_key.is_empty() => {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
                _ => {
                    eprintln!();
                    eprintln!("{}", "✗ OpenAI API key not configured".red().bold());
                    eprintln!();
                    eprintln!("  To fix, set your API key:");
                    eprintln!("    {}", "export OPENAI_API_KEY=sk-...".cyan());
                    eprintln!();
                    eprintln!("  Get your key at: {}", "https://platform.openai.com/api-keys".cyan());
                    eprintln!();
                    anyhow::bail!("OpenAI API key not configured");
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.openai.endpoint.clone(),
            );
        }
        "ollama" => {
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.ollama.endpoint.clone(),
            );
        }
        "deepseek" => {
            match &config.providers.deepseek.api_key {
                Some(api_key) if !api_key.is_empty() => {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
                _ => {
                    eprintln!();
                    eprintln!("{}", "✗ DeepSeek API key not configured".red().bold());
                    eprintln!();
                    eprintln!("  To fix, set your API key:");
                    eprintln!("    {}", "export DEEPSEEK_API_KEY=...".cyan());
                    eprintln!();
                    anyhow::bail!("DeepSeek API key not configured");
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.deepseek.endpoint.clone(),
            );
        }
        "groq" => {
            match &config.providers.groq.api_key {
                Some(api_key) if !api_key.is_empty() => {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
                _ => {
                    eprintln!();
                    eprintln!("{}", "✗ Groq API key not configured".red().bold());
                    eprintln!();
                    eprintln!("  To fix, set your API key:");
                    eprintln!("    {}", "export GROQ_API_KEY=...".cyan());
                    eprintln!();
                    eprintln!("  Get your key at: {}", "https://console.groq.com".cyan());
                    eprintln!();
                    anyhow::bail!("Groq API key not configured");
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.groq.endpoint.clone(),
            );
        }
        "grok" => {
            match &config.providers.grok.api_key {
                Some(api_key) if !api_key.is_empty() => {
                    provider_config.insert("api_key".to_string(), api_key.clone());
                }
                _ => {
                    eprintln!();
                    eprintln!("{}", "✗ Grok (xAI) API key not configured".red().bold());
                    eprintln!();
                    eprintln!("  To fix, set your API key:");
                    eprintln!("    {}", "export XAI_API_KEY=...".cyan());
                    eprintln!();
                    eprintln!("  Get your key at: {}", "https://console.x.ai".cyan());
                    eprintln!();
                    anyhow::bail!("Grok API key not configured");
                }
            }
            provider_config.insert(
                "endpoint".to_string(),
                config.providers.grok.endpoint.clone(),
            );
        }
        _ => {
            eprintln!();
            eprintln!("{}", format!("✗ Unknown provider: {}", provider).red().bold());
            eprintln!();
            eprintln!("  Available providers:");
            eprintln!("    {} - Grok models (default)", "grok".cyan());
            eprintln!("    {} - Claude models", "anthropic".cyan());
            eprintln!("    {} - GPT models", "openai".cyan());
            eprintln!("    {} - Local models", "ollama".cyan());
            eprintln!("    {} - DeepSeek models", "deepseek".cyan());
            eprintln!("    {} - Fast inference", "groq".cyan());
            eprintln!();
            anyhow::bail!("Unknown provider: {}", provider);
        }
    }

    let backend = ProviderFactory::create(provider, provider_config)?;

    Ok(backend)
}

pub fn get_model_for_provider(
    config: &DescaratesConfig,
    provider: &str,
    model: Option<&str>,
) -> Result<String> {
    if let Some(m) = model {
        return Ok(m.to_string());
    }

    // Get default model for provider
    match provider {
        "grok" => Ok(config.providers.grok.model.clone()),
        "anthropic" => Ok(config.providers.anthropic.model.clone()),
        "openai" => Ok(config.providers.openai.model.clone()),
        "ollama" => Ok(config.providers.ollama.model.clone()),
        "deepseek" => Ok(config.providers.deepseek.model.clone()),
        "groq" => Ok(config.providers.groq.model.clone()),
        _ => anyhow::bail!("Unknown provider: {}", provider),
    }
}
