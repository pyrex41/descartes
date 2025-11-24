/// Example RPC client for Descartes daemon

use serde_json::{json, Value};

/// Simple RPC client
pub struct RpcClient {
    url: String,
}

impl RpcClient {
    /// Create a new RPC client
    pub fn new(url: &str) -> Self {
        RpcClient {
            url: url.to_string(),
        }
    }

    /// Call a JSON-RPC 2.0 method
    pub async fn call(&self, method: &str, params: Option<Value>) -> Result<Value, Box<dyn std::error::Error>> {
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params.unwrap_or(Value::Null),
            "id": uuid::Uuid::new_v4().to_string()
        });

        let client = reqwest::Client::new();
        let response = client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        let result: Value = response.json().await?;

        if let Some(error) = result.get("error") {
            return Err(format!("RPC error: {}", error).into());
        }

        Ok(result.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Spawn an agent
    pub async fn spawn_agent(
        &self,
        name: &str,
        agent_type: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let params = json!({
            "name": name,
            "agent_type": agent_type,
            "config": {}
        });

        let result = self.call("agent.spawn", Some(params)).await?;
        Ok(result
            .get("agent_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// List all agents
    pub async fn list_agents(&self) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let result = self.call("agent.list", None).await?;
        let agents = result
            .get("agents")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(agents)
    }

    /// Kill an agent
    pub async fn kill_agent(&self, agent_id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let params = json!({
            "agent_id": agent_id,
            "force": false
        });

        self.call("agent.kill", Some(params)).await?;
        Ok(())
    }

    /// Get agent logs
    pub async fn get_logs(
        &self,
        agent_id: &str,
        limit: Option<usize>,
    ) -> Result<Vec<Value>, Box<dyn std::error::Error>> {
        let params = json!({
            "agent_id": agent_id,
            "limit": limit.unwrap_or(100),
            "offset": 0
        });

        let result = self.call("agent.logs", Some(params)).await?;
        let logs = result
            .get("logs")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        Ok(logs)
    }

    /// Execute workflow
    pub async fn execute_workflow(
        &self,
        workflow_id: &str,
        agents: Vec<&str>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let params = json!({
            "workflow_id": workflow_id,
            "agents": agents,
            "config": {}
        });

        let result = self.call("workflow.execute", Some(params)).await?;
        Ok(result
            .get("execution_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string())
    }

    /// Query state
    pub async fn query_state(
        &self,
        agent_id: Option<&str>,
    ) -> Result<Value, Box<dyn std::error::Error>> {
        let params = if let Some(id) = agent_id {
            json!({ "agent_id": id })
        } else {
            json!({})
        };

        self.call("state.query", Some(params)).await
    }

    /// Get system health
    pub async fn health(&self) -> Result<Value, Box<dyn std::error::Error>> {
        self.call("system.health", None).await
    }

    /// Get metrics
    pub async fn metrics(&self) -> Result<Value, Box<dyn std::error::Error>> {
        self.call("system.metrics", None).await
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client
    let client = RpcClient::new("http://127.0.0.1:8080");

    // Check health
    println!("Checking server health...");
    let health = client.health().await?;
    println!("Health: {}", serde_json::to_string_pretty(&health)?);

    // List agents
    println!("\nListing agents...");
    let agents = client.list_agents().await?;
    println!("Found {} agents", agents.len());
    for agent in agents {
        println!("  - {}", agent);
    }

    // Spawn agent
    println!("\nSpawning agent...");
    let agent_id = client.spawn_agent("test-agent", "basic").await?;
    println!("Spawned agent: {}", agent_id);

    // List agents again
    println!("\nListing agents again...");
    let agents = client.list_agents().await?;
    println!("Found {} agents", agents.len());

    // Get metrics
    println!("\nGetting metrics...");
    let metrics = client.metrics().await?;
    println!("Metrics: {}", serde_json::to_string_pretty(&metrics)?);

    // Kill agent
    println!("\nKilling agent...");
    client.kill_agent(&agent_id).await?;
    println!("Agent killed");

    Ok(())
}
