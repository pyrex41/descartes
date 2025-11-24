/// SQLite-backed implementation of the LeaseManager trait
/// Provides persistent, distributed file locking with TTL semantics

use crate::errors::{AgentError, AgentResult};
use crate::lease::{
    Lease, LeaseAcquisitionRequest, LeaseAcquisitionResponse, LeaseManager, LeaseReleaseRequest,
    LeaseReleaseResponse, LeaseRenewalRequest, LeaseRenewalResponse, LeaseStatus,
};
use async_trait::async_trait;
use chrono::{Duration, Utc};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Instant;
use uuid::Uuid;

/// SQLite-backed lease manager implementation
pub struct SqliteLeaseManager {
    /// Connection pool to SQLite database
    pool: SqlitePool,

    /// Path to the SQLite database file
    db_path: PathBuf,
}

impl SqliteLeaseManager {
    /// Create a new SQLite lease manager
    ///
    /// # Arguments
    /// * `db_path` - Path to the SQLite database file
    pub async fn new(db_path: PathBuf) -> AgentResult<Self> {
        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }

        // Create connection options with foreign keys enabled
        let connect_options = SqliteConnectOptions::from_str(db_path.to_string_lossy().as_ref())
            .map_err(|e| {
                AgentError::ExecutionError(format!("Failed to parse database path: {}", e))
            })?
            .create_if_missing(true)
            .foreign_keys(true);

        // Create connection pool
        let pool = SqlitePoolOptions::new()
            .connect_with(connect_options)
            .await
            .map_err(|e| {
                AgentError::ExecutionError(format!("Failed to create database pool: {}", e))
            })?;

        Ok(SqliteLeaseManager { pool, db_path })
    }

    /// Initialize the database schema
    pub async fn initialize(&self) -> AgentResult<()> {
        // Create leases table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS leases (
                id TEXT PRIMARY KEY NOT NULL,
                file_path TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                created_at INTEGER NOT NULL,
                expires_at INTEGER NOT NULL,
                ttl_seconds INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                renewal_count INTEGER NOT NULL DEFAULT 0,
                max_renewals INTEGER NOT NULL DEFAULT -1,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_leases_file_path ON leases(file_path);
            CREATE INDEX IF NOT EXISTS idx_leases_agent_id ON leases(agent_id);
            CREATE INDEX IF NOT EXISTS idx_leases_expires_at ON leases(expires_at) WHERE status = 'active';
            CREATE INDEX IF NOT EXISTS idx_leases_file_status ON leases(file_path, status);
            CREATE INDEX IF NOT EXISTS idx_leases_agent_status ON leases(agent_id, status);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| AgentError::ExecutionError(format!("Failed to create leases table: {}", e)))?;

        // Create lease history table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS lease_history (
                id TEXT PRIMARY KEY NOT NULL,
                lease_id TEXT NOT NULL,
                agent_id TEXT NOT NULL,
                file_path TEXT NOT NULL,
                event_type TEXT NOT NULL,
                status_before TEXT,
                status_after TEXT,
                renewal_count INTEGER,
                details TEXT,
                event_at INTEGER NOT NULL,
                reason TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_lease_history_lease_id ON lease_history(lease_id);
            CREATE INDEX IF NOT EXISTS idx_lease_history_agent_id ON lease_history(agent_id);
            CREATE INDEX IF NOT EXISTS idx_lease_history_event_type ON lease_history(event_type);
            CREATE INDEX IF NOT EXISTS idx_lease_history_file_path ON lease_history(file_path);
            CREATE INDEX IF NOT EXISTS idx_lease_history_event_at ON lease_history(event_at);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to create lease_history table: {}", e))
        })?;

        // Create lease configs table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS lease_configs (
                id TEXT PRIMARY KEY NOT NULL,
                config_key TEXT NOT NULL UNIQUE,
                config_value TEXT NOT NULL,
                value_type TEXT NOT NULL DEFAULT 'string',
                description TEXT,
                scope TEXT NOT NULL DEFAULT 'global',
                resource_id TEXT,
                requires_restart INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_lease_configs_key ON lease_configs(config_key);
            CREATE INDEX IF NOT EXISTS idx_lease_configs_scope ON lease_configs(scope, resource_id);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to create lease_configs table: {}", e))
        })?;

        // Create migrations table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS migrations (
                version INTEGER PRIMARY KEY NOT NULL,
                name TEXT NOT NULL UNIQUE,
                description TEXT,
                applied_at INTEGER NOT NULL,
                status TEXT NOT NULL DEFAULT 'success',
                error_message TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_migrations_name ON migrations(name);
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to create migrations table: {}", e))
        })?;

        Ok(())
    }

    /// Convert a database row to a Lease struct
    fn row_to_lease(row: &sqlx::sqlite::SqliteRow) -> AgentResult<Lease> {
        let id: String = row.get("id");
        let file_path: String = row.get("file_path");
        let agent_id: String = row.get("agent_id");
        let created_at: i64 = row.get("created_at");
        let expires_at: i64 = row.get("expires_at");
        let ttl_seconds: i64 = row.get("ttl_seconds");
        let status_str: String = row.get("status");
        let renewal_count: i32 = row.get("renewal_count");
        let max_renewals: i32 = row.get("max_renewals");

        // Parse UUIDs
        let lease_id = Uuid::parse_str(&id).map_err(|e| {
            AgentError::ExecutionError(format!("Invalid lease ID: {}", e))
        })?;

        let agent_uuid = Uuid::parse_str(&agent_id).map_err(|e| {
            AgentError::ExecutionError(format!("Invalid agent ID: {}", e))
        })?;

        // Parse status
        let status = match status_str.as_str() {
            "active" => LeaseStatus::Active,
            "expired" => LeaseStatus::Expired,
            "released" => LeaseStatus::Released,
            "pending" => LeaseStatus::Pending,
            "failed" => LeaseStatus::Failed,
            _ => LeaseStatus::Failed,
        };

        // Convert timestamps to chrono DateTime
        let created_at_dt = chrono::DateTime::<Utc>::from_timestamp(created_at, 0)
            .ok_or_else(|| AgentError::ExecutionError("Invalid created_at timestamp".to_string()))?;

        let expires_at_dt = chrono::DateTime::<Utc>::from_timestamp(expires_at, 0)
            .ok_or_else(|| AgentError::ExecutionError("Invalid expires_at timestamp".to_string()))?;

        Ok(Lease {
            id: lease_id,
            file_path: PathBuf::from(file_path),
            agent_id: agent_uuid,
            created_at: created_at_dt,
            expires_at: expires_at_dt,
            ttl: Duration::seconds(ttl_seconds),
            status,
            renewal_count: renewal_count as u32,
            max_renewals,
        })
    }

    /// Record a lease history event
    async fn record_history(
        &self,
        lease_id: &Uuid,
        agent_id: &Uuid,
        file_path: &Path,
        event_type: &str,
        status_before: Option<&str>,
        status_after: Option<&str>,
        renewal_count: Option<i32>,
        reason: Option<&str>,
    ) -> AgentResult<()> {
        let history_id = Uuid::new_v4().to_string();
        let lease_id_str = lease_id.to_string();
        let agent_id_str = agent_id.to_string();
        let file_path_str = file_path.to_string_lossy().to_string();
        let now = Utc::now().timestamp();

        sqlx::query(
            r#"
            INSERT INTO lease_history (id, lease_id, agent_id, file_path, event_type,
                                       status_before, status_after, renewal_count, event_at, reason)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(history_id)
        .bind(lease_id_str)
        .bind(agent_id_str)
        .bind(file_path_str)
        .bind(event_type)
        .bind(status_before)
        .bind(status_after)
        .bind(renewal_count)
        .bind(now)
        .bind(reason)
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to record lease history: {}", e))
        })?;

        Ok(())
    }
}

#[async_trait]
impl LeaseManager for SqliteLeaseManager {
    async fn acquire_lease(
        &self,
        request: LeaseAcquisitionRequest,
    ) -> AgentResult<LeaseAcquisitionResponse> {
        let start_time = Instant::now();
        let timeout_duration = std::time::Duration::from_millis(request.timeout_ms.unwrap_or(30000));

        loop {
            // Check if file is already locked by another agent
            let existing_lease: Option<(String,)> = sqlx::query_as(
                "SELECT id FROM leases WHERE file_path = ? AND status IN ('active', 'pending') AND agent_id != ? LIMIT 1"
            )
            .bind(request.file_path.to_string_lossy().as_ref())
            .bind(request.agent_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| {
                AgentError::ExecutionError(format!("Failed to check existing leases: {}", e))
            })?;

            if existing_lease.is_none() {
                // No conflicting lease, proceed with acquisition
                let lease_id = Uuid::new_v4();
                let ttl = Duration::seconds(request.ttl_seconds as i64);
                let now = Utc::now();
                let expires_at = now + ttl;

                let mut lease = Lease {
                    id: lease_id,
                    file_path: request.file_path.clone(),
                    agent_id: request.agent_id,
                    created_at: now,
                    expires_at,
                    ttl,
                    status: LeaseStatus::Pending,
                    renewal_count: 0,
                    max_renewals: request.max_renewals,
                };

                // Insert into database
                sqlx::query(
                    r#"
                    INSERT INTO leases (id, file_path, agent_id, created_at, expires_at,
                                       ttl_seconds, status, renewal_count, max_renewals, updated_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
                    "#,
                )
                .bind(lease_id.to_string())
                .bind(request.file_path.to_string_lossy().as_ref())
                .bind(request.agent_id.to_string())
                .bind(now.timestamp())
                .bind(expires_at.timestamp())
                .bind(request.ttl_seconds as i64)
                .bind("pending")
                .bind(0)
                .bind(request.max_renewals)
                .bind(now.timestamp())
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to insert lease: {}", e))
                })?;

                // Activate the lease
                lease.activate();

                sqlx::query("UPDATE leases SET status = ?, updated_at = ? WHERE id = ?")
                    .bind("active")
                    .bind(now.timestamp())
                    .bind(lease_id.to_string())
                    .execute(&self.pool)
                    .await
                    .map_err(|e| {
                        AgentError::ExecutionError(format!("Failed to activate lease: {}", e))
                    })?;

                // Record history
                let _ = self
                    .record_history(
                        &lease_id,
                        &request.agent_id,
                        &request.file_path,
                        "acquired",
                        Some("pending"),
                        Some("active"),
                        Some(0),
                        None,
                    )
                    .await;

                let wait_time_ms = start_time.elapsed().as_millis() as u64;

                return Ok(LeaseAcquisitionResponse {
                    success: true,
                    lease: Some(lease),
                    error: None,
                    wait_time_ms,
                });
            }

            // If blocking is disabled, return immediately
            if !request.blocking {
                let wait_time_ms = start_time.elapsed().as_millis() as u64;
                return Ok(LeaseAcquisitionResponse {
                    success: false,
                    lease: None,
                    error: Some("File is already locked by another agent".to_string()),
                    wait_time_ms,
                });
            }

            // Check if timeout exceeded
            if start_time.elapsed() >= timeout_duration {
                let wait_time_ms = start_time.elapsed().as_millis() as u64;
                return Ok(LeaseAcquisitionResponse {
                    success: false,
                    lease: None,
                    error: Some("Lease acquisition timeout".to_string()),
                    wait_time_ms,
                });
            }

            // Wait a bit before retrying
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }
    }

    async fn renew_lease(&self, request: LeaseRenewalRequest) -> AgentResult<LeaseRenewalResponse> {
        // Fetch the existing lease
        let lease = self.get_lease(&request.lease_id).await?;

        let lease = match lease {
            Some(mut l) => {
                // Verify the agent owns the lease
                if l.agent_id != request.agent_id {
                    return Ok(LeaseRenewalResponse {
                        success: false,
                        lease: None,
                        error: Some("Agent does not own this lease".to_string()),
                    });
                }

                // Attempt renewal
                if !l.renew() {
                    return Ok(LeaseRenewalResponse {
                        success: false,
                        lease: None,
                        error: Some("Maximum renewals exceeded".to_string()),
                    });
                }

                // Update TTL if provided
                if let Some(new_ttl_seconds) = request.new_ttl_seconds {
                    let new_ttl = Duration::seconds(new_ttl_seconds as i64);
                    l.ttl = new_ttl;
                    l.expires_at = Utc::now() + new_ttl;
                }

                // Update database
                let now = Utc::now();
                sqlx::query(
                    r#"
                    UPDATE leases
                    SET expires_at = ?, ttl_seconds = ?, renewal_count = ?,
                        status = ?, updated_at = ?
                    WHERE id = ?
                    "#,
                )
                .bind(l.expires_at.timestamp())
                .bind(l.ttl.num_seconds())
                .bind(l.renewal_count as i32)
                .bind("active")
                .bind(now.timestamp())
                .bind(request.lease_id.to_string())
                .execute(&self.pool)
                .await
                .map_err(|e| {
                    AgentError::ExecutionError(format!("Failed to update lease: {}", e))
                })?;

                // Record history
                let _ = self
                    .record_history(
                        &request.lease_id,
                        &request.agent_id,
                        &l.file_path,
                        "renewed",
                        Some("active"),
                        Some("active"),
                        Some(l.renewal_count as i32),
                        None,
                    )
                    .await;

                l
            }
            None => {
                return Ok(LeaseRenewalResponse {
                    success: false,
                    lease: None,
                    error: Some("Lease not found".to_string()),
                });
            }
        };

        Ok(LeaseRenewalResponse {
            success: true,
            lease: Some(lease),
            error: None,
        })
    }

    async fn release_lease(&self, request: LeaseReleaseRequest) -> AgentResult<LeaseReleaseResponse> {
        // Fetch the lease to verify ownership
        let lease = self.get_lease(&request.lease_id).await?;

        let lease = match lease {
            Some(l) => {
                // Verify the agent owns the lease
                if l.agent_id != request.agent_id {
                    return Ok(LeaseReleaseResponse {
                        success: false,
                        error: Some("Agent does not own this lease".to_string()),
                    });
                }
                l
            }
            None => {
                return Ok(LeaseReleaseResponse {
                    success: false,
                    error: Some("Lease not found".to_string()),
                });
            }
        };

        // Update lease status to released
        let now = Utc::now();
        let status_before = lease.status.to_string();

        sqlx::query("UPDATE leases SET status = ?, updated_at = ? WHERE id = ?")
            .bind("released")
            .bind(now.timestamp())
            .bind(request.lease_id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| AgentError::ExecutionError(format!("Failed to release lease: {}", e)))?;

        // Record history
        let _ = self
            .record_history(
                &request.lease_id,
                &request.agent_id,
                &lease.file_path,
                "released",
                Some(&status_before),
                Some("released"),
                None,
                None,
            )
            .await;

        Ok(LeaseReleaseResponse {
            success: true,
            error: None,
        })
    }

    async fn get_lease(&self, lease_id: &Uuid) -> AgentResult<Option<Lease>> {
        let row = sqlx::query("SELECT * FROM leases WHERE id = ?")
            .bind(lease_id.to_string())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| AgentError::ExecutionError(format!("Failed to fetch lease: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_lease(&r)?)),
            None => Ok(None),
        }
    }

    async fn get_file_leases(&self, file_path: &Path) -> AgentResult<Vec<Lease>> {
        let rows = sqlx::query("SELECT * FROM leases WHERE file_path = ? ORDER BY created_at DESC")
            .bind(file_path.to_string_lossy().as_ref())
            .fetch_all(&self.pool)
            .await
            .map_err(|e| {
                AgentError::ExecutionError(format!("Failed to fetch file leases: {}", e))
            })?;

        rows.iter().map(Self::row_to_lease).collect()
    }

    async fn get_agent_leases(&self, agent_id: &Uuid) -> AgentResult<Vec<Lease>> {
        let rows = sqlx::query(
            "SELECT * FROM leases WHERE agent_id = ? AND status = 'active' ORDER BY created_at DESC",
        )
        .bind(agent_id.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to fetch agent leases: {}", e))
        })?;

        rows.iter().map(Self::row_to_lease).collect()
    }

    async fn get_all_leases(&self) -> AgentResult<Vec<Lease>> {
        let rows = sqlx::query("SELECT * FROM leases ORDER BY created_at DESC")
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AgentError::ExecutionError(format!("Failed to fetch all leases: {}", e)))?;

        rows.iter().map(Self::row_to_lease).collect()
    }

    async fn is_file_locked(&self, file_path: &Path) -> AgentResult<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM leases WHERE file_path = ? AND status IN ('active', 'pending')",
        )
        .bind(file_path.to_string_lossy().as_ref())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to check file lock status: {}", e))
        })?;

        Ok(count.0 > 0)
    }

    async fn has_agent_lease(
        &self,
        agent_id: &Uuid,
        file_path: &Path,
    ) -> AgentResult<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM leases WHERE agent_id = ? AND file_path = ? AND status = 'active'",
        )
        .bind(agent_id.to_string())
        .bind(file_path.to_string_lossy().as_ref())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to check agent lease: {}", e))
        })?;

        Ok(count.0 > 0)
    }

    async fn cleanup_expired_leases(&self) -> AgentResult<usize> {
        let now = Utc::now().timestamp();

        // Get all expired leases before deleting
        let expired_leases = sqlx::query_as::<_, (String, String, String, String)>(
            "SELECT id, agent_id, file_path, status FROM leases WHERE expires_at <= ? AND status = 'active'",
        )
        .bind(now)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to fetch expired leases: {}", e))
        })?;

        // Update expired leases status
        let count = sqlx::query("UPDATE leases SET status = 'expired', updated_at = ? WHERE expires_at <= ? AND status = 'active'")
            .bind(now)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(|e| {
                AgentError::ExecutionError(format!("Failed to mark leases as expired: {}", e))
            })?
            .rows_affected();

        // Record history for expired leases
        for (lease_id, agent_id, file_path, _status) in expired_leases {
            let _ = self
                .record_history(
                    &Uuid::parse_str(&lease_id).unwrap_or_else(|_| Uuid::new_v4()),
                    &Uuid::parse_str(&agent_id).unwrap_or_else(|_| Uuid::new_v4()),
                    Path::new(&file_path),
                    "expired",
                    Some("active"),
                    Some("expired"),
                    None,
                    Some("TTL exceeded"),
                )
                .await;
        }

        Ok(count as usize)
    }

    async fn force_release_agent_leases(&self, agent_id: &Uuid) -> AgentResult<usize> {
        let now = Utc::now().timestamp();

        let count = sqlx::query(
            "UPDATE leases SET status = 'released', updated_at = ? WHERE agent_id = ? AND status = 'active'",
        )
        .bind(now)
        .bind(agent_id.to_string())
        .execute(&self.pool)
        .await
        .map_err(|e| {
            AgentError::ExecutionError(format!("Failed to force release agent leases: {}", e))
        })?
        .rows_affected();

        Ok(count as usize)
    }
}
