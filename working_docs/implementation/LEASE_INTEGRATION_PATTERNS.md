# Lease System Integration Patterns

Advanced patterns for integrating the TTL-Based File Leasing System with the Descartes agent orchestration framework.

## Pattern 1: Agent-Based Lease Management

Integrate lease management directly into the AgentRunner lifecycle.

```rust
use descartes_core::{AgentConfig, SqliteLeaseManager};
use std::path::PathBuf;

pub struct LeaseAwareAgent {
    lease_manager: Arc<SqliteLeaseManager>,
    config: AgentConfig,
    active_leases: DashMap<Uuid, Lease>,
}

impl LeaseAwareAgent {
    pub async fn execute_with_file_lock(
        &self,
        file_path: PathBuf,
        ttl_seconds: u64,
        operation: impl Fn() -> BoxFuture<'static, AgentResult<()>>,
    ) -> AgentResult<()> {
        // Acquire lease
        let request = LeaseAcquisitionRequest {
            file_path: file_path.clone(),
            agent_id: self.config.id,
            ttl_seconds,
            max_renewals: 5,
            timeout_ms: Some(30000),
            blocking: true,
        };

        let response = self.lease_manager.acquire_lease(request).await?;

        if !response.success {
            return Err(AgentError::ExecutionError(
                response.error.unwrap_or_default()
            ));
        }

        let lease = response.lease.unwrap();
        self.active_leases.insert(lease.id, lease.clone());

        // Execute operation
        let result = operation().await;

        // Release lease
        self.lease_manager.release_lease(LeaseReleaseRequest {
            lease_id: lease.id,
            agent_id: self.config.id,
        }).await?;

        self.active_leases.remove(&lease.id);
        result
    }
}
```

## Pattern 2: Background Lease Renewal Task

Keep leases active for long-running operations with automatic renewal.

```rust
pub async fn lease_renewal_service(
    manager: Arc<SqliteLeaseManager>,
    agent_id: Uuid,
    interval_secs: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));

    loop {
        interval.tick().await;

        // Get all active leases for this agent
        match manager.get_agent_leases(&agent_id).await {
            Ok(leases) => {
                for mut lease in leases {
                    // Check if renewal needed
                    if let Some(remaining) = lease.time_remaining() {
                        let threshold = Duration::seconds(interval_secs as i64 * 2);

                        if remaining < threshold {
                            // Renew before expiration
                            let request = LeaseRenewalRequest {
                                lease_id: lease.id,
                                agent_id,
                                new_ttl_seconds: None,
                            };

                            if let Err(e) = manager.renew_lease(request).await {
                                tracing::warn!(
                                    "Failed to renew lease {}: {}",
                                    lease.id, e
                                );
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to fetch agent leases: {}", e);
            }
        }
    }
}
```

## Pattern 3: Automatic Cleanup Task

Periodically clean up expired leases and maintain database health.

```rust
pub async fn lease_cleanup_service(
    manager: Arc<SqliteLeaseManager>,
    cleanup_interval_secs: u64,
    history_retention_days: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(cleanup_interval_secs));

    loop {
        interval.tick().await;

        // Clean up expired leases
        match manager.cleanup_expired_leases().await {
            Ok(count) => {
                if count > 0 {
                    tracing::info!("Cleaned up {} expired leases", count);
                }
            }
            Err(e) => {
                tracing::error!("Cleanup failed: {}", e);
            }
        }

        // Clean up old history records
        let retention_timestamp = Utc::now() - Duration::days(history_retention_days as i64);
        if let Err(e) = sqlx::query(
            "DELETE FROM lease_history WHERE event_at < ?"
        )
        .bind(retention_timestamp.timestamp())
        .execute(&pool)
        .await {
            tracing::error!("Failed to clean history: {}", e);
        }
    }
}
```

## Pattern 4: Deadlock Detection and Prevention

Monitor for potential deadlock situations and take corrective action.

```rust
pub async fn deadlock_monitor_service(
    manager: Arc<SqliteLeaseManager>,
    check_interval_secs: u64,
    max_lock_age_secs: u64,
) {
    let mut interval = tokio::time::interval(Duration::from_secs(check_interval_secs));

    loop {
        interval.tick().await;

        match manager.get_all_leases().await {
            Ok(leases) => {
                for lease in leases {
                    if lease.status == LeaseStatus::Active {
                        // Check if lease is unusually old
                        if let Some(age) = chrono::Utc::now()
                            .signed_duration_since(lease.created_at)
                            .to_std()
                            .ok()
                            .and_then(|d| u64::try_from(d.as_secs()).ok())
                        {
                            if age > max_lock_age_secs {
                                tracing::warn!(
                                    "Lease {} held by agent {} for {} seconds (max: {})",
                                    lease.id, lease.agent_id, age, max_lock_age_secs
                                );

                                // Optionally force-release if agent is unresponsive
                                // manager.force_release_agent_leases(&lease.agent_id).await?;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to check for deadlocks: {}", e);
            }
        }
    }
}
```

## Pattern 5: File-Based Lock Coordination

Coordinate across multiple file operations with transactional semantics.

```rust
pub struct TransactionalFileLock {
    manager: Arc<SqliteLeaseManager>,
    agent_id: Uuid,
    files: Vec<PathBuf>,
    leases: Vec<Lease>,
}

impl TransactionalFileLock {
    pub async fn acquire_all(
        manager: Arc<SqliteLeaseManager>,
        agent_id: Uuid,
        files: Vec<PathBuf>,
        ttl_seconds: u64,
    ) -> AgentResult<Self> {
        let mut leases = Vec::new();

        // Sort files to prevent deadlock through consistent ordering
        let mut sorted_files = files.clone();
        sorted_files.sort();

        for file_path in &sorted_files {
            let request = LeaseAcquisitionRequest {
                file_path: file_path.clone(),
                agent_id,
                ttl_seconds,
                max_renewals: 3,
                timeout_ms: Some(30000),
                blocking: true,
            };

            let response = manager.acquire_lease(request).await?;

            if !response.success {
                // Rollback: release previously acquired leases
                for lease in &leases {
                    let _ = manager.release_lease(LeaseReleaseRequest {
                        lease_id: lease.id,
                        agent_id,
                    }).await;
                }

                return Err(AgentError::ExecutionError(
                    response.error.unwrap_or_default()
                ));
            }

            leases.push(response.lease.unwrap());
        }

        Ok(TransactionalFileLock {
            manager,
            agent_id,
            files: sorted_files,
            leases,
        })
    }

    pub async fn release_all(&self) -> AgentResult<()> {
        for lease in &self.leases {
            self.manager.release_lease(LeaseReleaseRequest {
                lease_id: lease.id,
                agent_id: self.agent_id,
            }).await?;
        }
        Ok(())
    }

    pub fn check_validity(&self) -> bool {
        self.leases.iter().all(|lease| lease.is_valid())
    }
}

// Usage:
let lock = TransactionalFileLock::acquire_all(
    manager,
    agent_id,
    vec![file1, file2, file3],
    3600,
).await?;

// Do work...

lock.release_all().await?;
```

## Pattern 6: Lease-Based Event Notifications

Integrate with the event system to track lease lifecycle events.

```rust
pub struct LeaseEventAdapter {
    manager: Arc<SqliteLeaseManager>,
    event_router: Arc<EventRouter>,
}

impl LeaseEventAdapter {
    pub async fn emit_acquisition_event(
        &self,
        lease: &Lease,
        success: bool,
        wait_time_ms: u64,
    ) -> AgentResult<()> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "lease_acquired".to_string(),
            timestamp: Utc::now().timestamp(),
            session_id: "system".to_string(),
            actor_type: ActorType::System,
            actor_id: lease.agent_id.to_string(),
            content: format!(
                "Agent {} acquired lease for {}",
                lease.agent_id,
                lease.file_path.display()
            ),
            metadata: Some(serde_json::json!({
                "lease_id": lease.id.to_string(),
                "file_path": lease.file_path.to_string_lossy(),
                "ttl_seconds": lease.ttl.num_seconds(),
                "success": success,
                "wait_time_ms": wait_time_ms,
            })),
            git_commit: None,
        };

        self.event_router.emit(event).await?;
        Ok(())
    }

    pub async fn emit_renewal_event(
        &self,
        lease: &Lease,
    ) -> AgentResult<()> {
        let event = Event {
            id: Uuid::new_v4(),
            event_type: "lease_renewed".to_string(),
            timestamp: Utc::now().timestamp(),
            session_id: "system".to_string(),
            actor_type: ActorType::System,
            actor_id: lease.agent_id.to_string(),
            content: format!(
                "Agent {} renewed lease for {}",
                lease.agent_id,
                lease.file_path.display()
            ),
            metadata: Some(serde_json::json!({
                "lease_id": lease.id.to_string(),
                "renewal_count": lease.renewal_count,
                "max_renewals": lease.max_renewals,
            })),
            git_commit: None,
        };

        self.event_router.emit(event).await?;
        Ok(())
    }
}
```

## Pattern 7: Metrics and Monitoring

Export lease metrics for observability.

```rust
pub struct LeaseMetrics {
    active_leases: prometheus::IntGaugeVec,
    acquisitions_total: prometheus::IntCounterVec,
    renewals_total: prometheus::IntCounterVec,
    conflicts_total: prometheus::IntCounter,
    cleanup_duration_secs: prometheus::HistogramVec,
}

impl LeaseMetrics {
    pub async fn collect_metrics(
        &self,
        manager: Arc<SqliteLeaseManager>,
    ) -> AgentResult<()> {
        // Get all leases and categorize
        let all_leases = manager.get_all_leases().await?;

        let mut active_count = 0;
        let mut expired_count = 0;
        let mut released_count = 0;

        for lease in all_leases {
            match lease.status {
                LeaseStatus::Active => active_count += 1,
                LeaseStatus::Expired => expired_count += 1,
                LeaseStatus::Released => released_count += 1,
                _ => {}
            }
        }

        self.active_leases
            .with_label_values(&["active"])
            .set(active_count);

        self.active_leases
            .with_label_values(&["expired"])
            .set(expired_count);

        self.active_leases
            .with_label_values(&["released"])
            .set(released_count);

        Ok(())
    }

    pub fn record_acquisition(&self, success: bool) {
        let label = if success { "success" } else { "failure" };
        self.acquisitions_total
            .with_label_values(&[label])
            .inc();

        if !success {
            self.conflicts_total.inc();
        }
    }

    pub fn record_renewal(&self, success: bool) {
        let label = if success { "success" } else { "failure" };
        self.renewals_total
            .with_label_values(&[label])
            .inc();
    }
}
```

## Pattern 8: Graceful Shutdown with Lease Cleanup

Ensure all leases are released during system shutdown.

```rust
pub struct GracefulShutdown {
    manager: Arc<SqliteLeaseManager>,
    active_agents: Arc<DashMap<Uuid, AgentHandle>>,
}

impl GracefulShutdown {
    pub async fn shutdown(&self) -> AgentResult<()> {
        tracing::info!("Starting graceful shutdown of lease system");

        // Get all active agents
        let agents: Vec<Uuid> = self.active_agents
            .iter()
            .map(|entry| *entry.key())
            .collect();

        // Release leases for each agent
        for agent_id in agents {
            match self.manager.force_release_agent_leases(&agent_id).await {
                Ok(count) => {
                    if count > 0 {
                        tracing::info!(
                            "Released {} leases for agent {}",
                            count, agent_id
                        );
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "Failed to release leases for agent {}: {}",
                        agent_id, e
                    );
                }
            }
        }

        // Run final cleanup
        match self.manager.cleanup_expired_leases().await {
            Ok(count) => {
                tracing::info!("Final cleanup removed {} expired leases", count);
            }
            Err(e) => {
                tracing::error!("Final cleanup failed: {}", e);
            }
        }

        tracing::info!("Lease system shutdown complete");
        Ok(())
    }
}
```

## Pattern 9: Dynamic TTL Adjustment

Adjust lease TTL based on agent behavior and system load.

```rust
pub struct AdaptiveLeaseManager {
    manager: Arc<SqliteLeaseManager>,
    min_ttl: u64,
    max_ttl: u64,
    system_load_threshold: f32,
}

impl AdaptiveLeaseManager {
    pub async fn acquire_with_adaptive_ttl(
        &self,
        request: LeaseAcquisitionRequest,
        system_load: f32,
    ) -> AgentResult<LeaseAcquisitionResponse> {
        // Adjust TTL based on system load
        let adjusted_ttl = if system_load > self.system_load_threshold {
            // Under high load, use shorter TTL to allow fairness
            self.min_ttl
        } else {
            // Under normal load, use standard TTL
            request.ttl_seconds
        };

        let mut adjusted_request = request;
        adjusted_request.ttl_seconds = adjusted_ttl;

        self.manager.acquire_lease(adjusted_request).await
    }
}
```

## Pattern 10: Distributed Lease Coordination

Prepare for multi-instance deployment (future enhancement).

```rust
pub trait DistributedLeaseBackend: Send + Sync {
    async fn acquire_lease(&self, request: LeaseAcquisitionRequest)
        -> AgentResult<LeaseAcquisitionResponse>;

    async fn renew_lease(&self, request: LeaseRenewalRequest)
        -> AgentResult<LeaseRenewalResponse>;

    async fn release_lease(&self, request: LeaseReleaseRequest)
        -> AgentResult<LeaseReleaseResponse>;
}

// Future implementations:
// pub struct RedisLeaseManager { ... }
// pub struct EtcdLeaseManager { ... }
// pub struct ConsulLeaseManager { ... }

pub fn create_lease_manager(backend: &str) -> AgentResult<Arc<dyn DistributedLeaseBackend>> {
    match backend {
        "sqlite" => Ok(Arc::new(SqliteLeaseManager::new(
            PathBuf::from("leases.db")
        ).await?)),
        "redis" => {
            // Future: Ok(Arc::new(RedisLeaseManager::new(...).await?))
            Err(AgentError::ExecutionError(
                "Redis backend not yet implemented".to_string()
            ))
        }
        _ => Err(AgentError::ExecutionError(
            format!("Unknown lease backend: {}", backend)
        )),
    }
}
```

## Summary

These patterns provide:

1. **Agent Integration**: Direct lease management in agent lifecycle
2. **Automatic Renewal**: Background tasks keep leases alive
3. **Cleanup**: Periodic maintenance and history retention
4. **Deadlock Detection**: Monitor for stuck locks
5. **Transactional Safety**: Multi-file coordination
6. **Event Integration**: Full system observability
7. **Metrics**: Prometheus-compatible monitoring
8. **Graceful Shutdown**: Clean resource release
9. **Adaptive Behavior**: Dynamic TTL adjustment
10. **Future Scaling**: Distributed backend abstraction

These patterns are production-ready and can be combined as needed for your specific use cases.
