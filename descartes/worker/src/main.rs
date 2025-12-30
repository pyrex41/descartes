use clap::Parser;
use reqwest::Client;
use std::env;
use tracing::info;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    task: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let task_id = args.task;
    let callback_url = env::var("CALLBACK_URL")?;
    let project_id = env::var("PROJECT_ID")?;

    info!("Worker starting for task: {}", task_id);

    // Notify orchestrator we're starting
    let client = Client::new();
    client.post(&format!("{}/started", callback_url))
        .json(&serde_json::json!({
            "task_id": task_id,
            "project_id": project_id,
            "status": "started"
        }))
        .send()
        .await?;

    // Get task details (simplified for MVP - would use SCUD in production)
    let task_description = format!("Execute task: {}", task_id);

    // For MVP, simulate task execution
    // In production, this would spawn a Claude agent
    info!("Executing task: {}", task_description);

    // Simulate work
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    let result = serde_json::json!({
        "task_id": task_id,
        "status": "completed",
        "message": "Task executed successfully"
    });

    // Report completion
    client.post(&format!("{}/completed", callback_url))
        .json(&result)
        .send()
        .await?;

    info!("Worker exiting");
    Ok(())
}
