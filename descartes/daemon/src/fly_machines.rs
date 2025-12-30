/// Fly.io Machines API client for spawning ephemeral worker machines
///
/// This module provides an HTTP client for managing Fly.io Machines that run
/// Descartes worker processes. Workers are ephemeral compute instances that
/// execute agent sessions and report back to the orchestrator.
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;

const FLY_API_BASE: &str = "https://api.machines.dev/v1";

/// Client for interacting with Fly.io Machines API
#[derive(Clone)]
pub struct FlyMachinesClient {
    client: Client,
    api_token: String,
    app_name: String,
}

/// Request structure for creating a new machine
#[derive(Debug, Serialize)]
pub struct CreateMachineRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub config: MachineConfig,
}

/// Machine configuration including image, resources, and environment
#[derive(Debug, Serialize)]
pub struct MachineConfig {
    pub image: String,
    pub auto_destroy: bool,
    pub env: HashMap<String, String>,
    pub restart: RestartPolicy,
    pub guest: GuestConfig,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<InitConfig>,
}

/// Restart policy for the machine
#[derive(Debug, Serialize)]
pub struct RestartPolicy {
    pub policy: String,
}

/// Guest configuration specifying CPU and memory resources
#[derive(Debug, Serialize)]
pub struct GuestConfig {
    pub cpu_kind: String,
    pub cpus: u32,
    pub memory_mb: u32,
}

/// Init configuration with command to run
#[derive(Debug, Serialize)]
pub struct InitConfig {
    pub cmd: Vec<String>,
}

/// Machine representation from Fly.io API
#[derive(Debug, Deserialize)]
pub struct Machine {
    pub id: String,
    pub name: String,
    pub state: String,
    pub instance_id: String,
    pub private_ip: String,
    pub created_at: String,
}

/// Machine event from Fly.io API
#[derive(Debug, Deserialize)]
pub struct MachineEvent {
    pub id: String,
    #[serde(rename = "type")]
    pub event_type: String,
    pub status: String,
    pub timestamp: i64,
}

impl FlyMachinesClient {
    /// Create a new Fly.io Machines API client
    ///
    /// Reads configuration from environment variables:
    /// - FLY_API_TOKEN: Required authentication token
    /// - FLY_WORKER_APP: Optional app name (defaults to "descartes-workers")
    pub fn new() -> Result<Self, String> {
        let api_token = env::var("FLY_API_TOKEN")
            .map_err(|_| "FLY_API_TOKEN not set".to_string())?;
        let app_name = env::var("FLY_WORKER_APP")
            .unwrap_or_else(|_| "descartes-workers".to_string());

        Ok(Self {
            client: Client::new(),
            api_token,
            app_name,
        })
    }

    /// Spawn a new worker machine for executing a task
    ///
    /// Creates an ephemeral Fly.io machine configured to run a Descartes worker
    /// that will execute the specified task and report results to the callback URL.
    pub async fn spawn_worker(
        &self,
        task_id: &str,
        project_id: &str,
        callback_url: &str,
    ) -> Result<Machine, reqwest::Error> {
        let mut env = HashMap::new();
        env.insert("DESCARTES_TASK_ID".to_string(), task_id.to_string());
        env.insert("DESCARTES_PROJECT_ID".to_string(), project_id.to_string());
        env.insert("DESCARTES_CALLBACK_URL".to_string(), callback_url.to_string());

        let request = CreateMachineRequest {
            name: Some(format!("worker-{}", task_id)),
            config: MachineConfig {
                image: env::var("FLY_WORKER_IMAGE")
                    .unwrap_or_else(|_| "descartes-worker:latest".to_string()),
                auto_destroy: true,
                env,
                restart: RestartPolicy {
                    policy: "no".to_string(),
                },
                guest: GuestConfig {
                    cpu_kind: "shared".to_string(),
                    cpus: 1,
                    memory_mb: 512,
                },
                init: Some(InitConfig {
                    cmd: vec!["/app/worker".to_string()],
                }),
            },
        };

        let url = format!("{}/apps/{}/machines", FLY_API_BASE, self.app_name);

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json::<Machine>()
            .await
    }

    /// Get information about a specific machine
    pub async fn get_machine(&self, machine_id: &str) -> Result<Machine, reqwest::Error> {
        let url = format!(
            "{}/apps/{}/machines/{}",
            FLY_API_BASE, self.app_name, machine_id
        );

        self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?
            .error_for_status()?
            .json::<Machine>()
            .await
    }

    /// Stop a running machine
    pub async fn stop_machine(&self, machine_id: &str) -> Result<(), reqwest::Error> {
        let url = format!(
            "{}/apps/{}/machines/{}/stop",
            FLY_API_BASE, self.app_name, machine_id
        );

        self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// Destroy a machine permanently
    pub async fn destroy_machine(&self, machine_id: &str) -> Result<(), reqwest::Error> {
        let url = format!(
            "{}/apps/{}/machines/{}",
            FLY_API_BASE, self.app_name, machine_id
        );

        self.client
            .delete(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    /// List all machines in the app
    pub async fn list_machines(&self) -> Result<Vec<Machine>, reqwest::Error> {
        let url = format!("{}/apps/{}/machines", FLY_API_BASE, self.app_name);

        self.client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_token))
            .send()
            .await?
            .error_for_status()?
            .json::<Vec<Machine>>()
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_machine_request_serialization() {
        let mut env = HashMap::new();
        env.insert("TEST_VAR".to_string(), "test_value".to_string());

        let request = CreateMachineRequest {
            name: Some("test-machine".to_string()),
            config: MachineConfig {
                image: "test:latest".to_string(),
                auto_destroy: true,
                env,
                restart: RestartPolicy {
                    policy: "no".to_string(),
                },
                guest: GuestConfig {
                    cpu_kind: "shared".to_string(),
                    cpus: 1,
                    memory_mb: 512,
                },
                init: Some(InitConfig {
                    cmd: vec!["/app/worker".to_string()],
                }),
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("test-machine"));
        assert!(json.contains("test:latest"));
        assert!(json.contains("TEST_VAR"));
    }

    #[test]
    fn test_machine_deserialization() {
        let json = r#"{
            "id": "machine123",
            "name": "worker-task1",
            "state": "started",
            "instance_id": "inst123",
            "private_ip": "172.16.0.1",
            "created_at": "2025-12-30T12:00:00Z"
        }"#;

        let machine: Machine = serde_json::from_str(json).unwrap();
        assert_eq!(machine.id, "machine123");
        assert_eq!(machine.name, "worker-task1");
        assert_eq!(machine.state, "started");
    }

    #[test]
    fn test_client_creation_requires_token() {
        // Clear the env var if it exists
        env::remove_var("FLY_API_TOKEN");

        let result = FlyMachinesClient::new();
        assert!(result.is_err());
        if let Err(e) = result {
            assert_eq!(e, "FLY_API_TOKEN not set");
        }
    }

    #[test]
    fn test_client_creation_with_defaults() {
        env::set_var("FLY_API_TOKEN", "test_token");
        env::remove_var("FLY_WORKER_APP");

        let client = FlyMachinesClient::new().unwrap();
        assert_eq!(client.app_name, "descartes-workers");

        // Cleanup
        env::remove_var("FLY_API_TOKEN");
    }

    #[test]
    fn test_client_creation_with_custom_app() {
        env::set_var("FLY_API_TOKEN", "test_token");
        env::set_var("FLY_WORKER_APP", "my-workers");

        let client = FlyMachinesClient::new().unwrap();
        assert_eq!(client.app_name, "my-workers");

        // Cleanup
        env::remove_var("FLY_API_TOKEN");
        env::remove_var("FLY_WORKER_APP");
    }
}
