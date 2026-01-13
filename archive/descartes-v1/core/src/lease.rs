/// TTL-Based File Leasing System
/// Implements distributed file locking with time-to-live semantics for multi-agent workflows.
///
/// This module provides:
/// - Lease acquisition and release mechanisms
/// - Automatic expiration based on TTL
/// - Lease renewal for long-running operations
/// - Prevention of deadlocks with timeout mechanisms
/// - Tracking of which agent holds which files
use crate::errors::AgentResult;
use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

/// A lease representing exclusive access to a file.
///
/// Leases have a time-to-live (TTL) that determines how long the lease remains valid.
/// When a lease expires, it can be automatically released, allowing other agents to acquire it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Lease {
    /// Unique identifier for this lease
    pub id: Uuid,

    /// Path to the file being locked
    pub file_path: PathBuf,

    /// ID of the agent holding this lease
    pub agent_id: Uuid,

    /// Timestamp when the lease was created
    pub created_at: DateTime<Utc>,

    /// Timestamp when the lease will expire (created_at + ttl)
    pub expires_at: DateTime<Utc>,

    /// Time-to-live duration for this lease
    pub ttl: Duration,

    /// Current status of the lease
    pub status: LeaseStatus,

    /// Number of times this lease has been renewed
    pub renewal_count: u32,

    /// Maximum number of renewals allowed (-1 for unlimited)
    pub max_renewals: i32,
}

/// Status of a lease
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum LeaseStatus {
    /// Lease is active and valid
    Active,

    /// Lease has expired and is no longer valid
    Expired,

    /// Lease has been explicitly released by the agent
    Released,

    /// Lease is in the process of being acquired
    Pending,

    /// Lease acquisition or renewal failed
    Failed,
}

impl std::fmt::Display for LeaseStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LeaseStatus::Active => write!(f, "active"),
            LeaseStatus::Expired => write!(f, "expired"),
            LeaseStatus::Released => write!(f, "released"),
            LeaseStatus::Pending => write!(f, "pending"),
            LeaseStatus::Failed => write!(f, "failed"),
        }
    }
}

impl Lease {
    /// Create a new lease with the given parameters
    pub fn new(file_path: PathBuf, agent_id: Uuid, ttl: Duration, max_renewals: i32) -> Self {
        let id = Uuid::new_v4();
        let created_at = Utc::now();
        let expires_at = created_at + ttl;

        Lease {
            id,
            file_path,
            agent_id,
            created_at,
            expires_at,
            ttl,
            status: LeaseStatus::Pending,
            renewal_count: 0,
            max_renewals,
        }
    }

    /// Check if this lease is currently valid (active and not expired)
    pub fn is_valid(&self) -> bool {
        self.status == LeaseStatus::Active && Utc::now() < self.expires_at
    }

    /// Check if this lease has expired
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Get the remaining time until this lease expires
    pub fn time_remaining(&self) -> Option<Duration> {
        let now = Utc::now();
        if now < self.expires_at {
            (self.expires_at - now)
                .to_std()
                .ok()
                .and_then(|d| Duration::from_std(d).ok())
        } else {
            None
        }
    }

    /// Mark this lease as active (successfully acquired)
    pub fn activate(&mut self) {
        self.status = LeaseStatus::Active;
    }

    /// Mark this lease as expired
    pub fn mark_expired(&mut self) {
        self.status = LeaseStatus::Expired;
    }

    /// Mark this lease as released
    pub fn mark_released(&mut self) {
        self.status = LeaseStatus::Released;
    }

    /// Attempt to renew this lease
    ///
    /// Returns true if renewal was successful, false if max renewals exceeded
    pub fn renew(&mut self) -> bool {
        // Check if we can renew
        if self.max_renewals >= 0 && self.renewal_count >= self.max_renewals as u32 {
            return false;
        }

        // Reset expiration time
        self.expires_at = Utc::now() + self.ttl;
        self.renewal_count += 1;
        self.status = LeaseStatus::Active;

        true
    }
}

/// Parameters for acquiring a lease
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseAcquisitionRequest {
    /// Path to the file to lock
    pub file_path: PathBuf,

    /// ID of the agent requesting the lease
    pub agent_id: Uuid,

    /// Time-to-live for the lease in seconds
    pub ttl_seconds: u64,

    /// Maximum number of renewals (-1 for unlimited)
    pub max_renewals: i32,

    /// Timeout for acquiring the lease in milliseconds
    pub timeout_ms: Option<u64>,

    /// If true, block until lease is acquired or timeout expires
    pub blocking: bool,
}

/// Response from a lease acquisition attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseAcquisitionResponse {
    /// Whether the lease was successfully acquired
    pub success: bool,

    /// The acquired lease (if successful)
    pub lease: Option<Lease>,

    /// Error message (if acquisition failed)
    pub error: Option<String>,

    /// Time waited for the lease in milliseconds
    pub wait_time_ms: u64,
}

/// Parameters for renewing a lease
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRenewalRequest {
    /// ID of the lease to renew
    pub lease_id: Uuid,

    /// ID of the agent holding the lease
    pub agent_id: Uuid,

    /// New TTL in seconds (if different from current)
    pub new_ttl_seconds: Option<u64>,
}

/// Response from a lease renewal attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseRenewalResponse {
    /// Whether the renewal was successful
    pub success: bool,

    /// The renewed lease (if successful)
    pub lease: Option<Lease>,

    /// Error message (if renewal failed)
    pub error: Option<String>,
}

/// Parameters for releasing a lease
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseReleaseRequest {
    /// ID of the lease to release
    pub lease_id: Uuid,

    /// ID of the agent releasing the lease
    pub agent_id: Uuid,
}

/// Response from a lease release attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeaseReleaseResponse {
    /// Whether the release was successful
    pub success: bool,

    /// Error message (if release failed)
    pub error: Option<String>,
}

/// Lease manager trait - handles all lease operations
#[async_trait]
pub trait LeaseManager: Send + Sync {
    /// Acquire a lease for a file
    ///
    /// This operation will either:
    /// - Return immediately with the acquired lease
    /// - Block until the lease is available or timeout expires
    /// - Return an error if the lease cannot be acquired
    async fn acquire_lease(
        &self,
        request: LeaseAcquisitionRequest,
    ) -> AgentResult<LeaseAcquisitionResponse>;

    /// Renew an existing lease
    ///
    /// Extends the expiration time of an active lease.
    /// The agent must own the lease to renew it.
    async fn renew_lease(&self, request: LeaseRenewalRequest) -> AgentResult<LeaseRenewalResponse>;

    /// Release a lease
    ///
    /// Marks the lease as released, allowing other agents to acquire it.
    async fn release_lease(
        &self,
        request: LeaseReleaseRequest,
    ) -> AgentResult<LeaseReleaseResponse>;

    /// Get a lease by ID
    async fn get_lease(&self, lease_id: &Uuid) -> AgentResult<Option<Lease>>;

    /// Get all active leases for a specific file
    async fn get_file_leases(&self, file_path: &std::path::Path) -> AgentResult<Vec<Lease>>;

    /// Get all active leases held by an agent
    async fn get_agent_leases(&self, agent_id: &Uuid) -> AgentResult<Vec<Lease>>;

    /// Get all active leases in the system
    async fn get_all_leases(&self) -> AgentResult<Vec<Lease>>;

    /// Check if a file is currently locked (has an active lease)
    async fn is_file_locked(&self, file_path: &std::path::Path) -> AgentResult<bool>;

    /// Check if an agent has an active lease on a file
    async fn has_agent_lease(
        &self,
        agent_id: &Uuid,
        file_path: &std::path::Path,
    ) -> AgentResult<bool>;

    /// Clean up expired leases
    ///
    /// This operation should be called periodically to remove expired leases
    /// from the database. Returns the number of leases cleaned up.
    async fn cleanup_expired_leases(&self) -> AgentResult<usize>;

    /// Force release all leases held by an agent
    ///
    /// This is a dangerous operation that should only be used for cleanup
    /// or emergency situations when an agent has crashed.
    async fn force_release_agent_leases(&self, agent_id: &Uuid) -> AgentResult<usize>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lease_creation() {
        let agent_id = Uuid::new_v4();
        let file_path = PathBuf::from("/tmp/test.txt");
        let ttl = Duration::seconds(60);

        let lease = Lease::new(file_path.clone(), agent_id, ttl, 5);

        assert_eq!(lease.file_path, file_path);
        assert_eq!(lease.agent_id, agent_id);
        assert_eq!(lease.status, LeaseStatus::Pending);
        assert_eq!(lease.renewal_count, 0);
        assert_eq!(lease.max_renewals, 5);
    }

    #[test]
    fn test_lease_is_valid() {
        let agent_id = Uuid::new_v4();
        let file_path = PathBuf::from("/tmp/test.txt");
        let ttl = Duration::seconds(60);

        let mut lease = Lease::new(file_path, agent_id, ttl, 5);

        // Not valid yet (pending)
        assert!(!lease.is_valid());

        // Activate the lease
        lease.activate();

        // Now should be valid
        assert!(lease.is_valid());
        assert!(!lease.is_expired());
    }

    #[test]
    fn test_lease_renewal() {
        let agent_id = Uuid::new_v4();
        let file_path = PathBuf::from("/tmp/test.txt");
        let ttl = Duration::seconds(60);

        let mut lease = Lease::new(file_path, agent_id, ttl, 2);
        lease.activate();

        let initial_expiry = lease.expires_at;

        // Wait a bit and renew
        std::thread::sleep(std::time::Duration::from_millis(10));
        let renewed = lease.renew();

        assert!(renewed);
        assert_eq!(lease.renewal_count, 1);
        assert!(lease.expires_at > initial_expiry);

        // Renew again
        let renewed = lease.renew();
        assert!(renewed);
        assert_eq!(lease.renewal_count, 2);

        // Should not be able to renew anymore (max_renewals = 2)
        let renewed = lease.renew();
        assert!(!renewed);
        assert_eq!(lease.renewal_count, 2);
    }

    #[test]
    fn test_time_remaining() {
        let agent_id = Uuid::new_v4();
        let file_path = PathBuf::from("/tmp/test.txt");
        let ttl = Duration::seconds(60);

        let lease = Lease::new(file_path, agent_id, ttl, 5);

        let remaining = lease.time_remaining();
        assert!(remaining.is_some());
        let remaining = remaining.unwrap();

        // Should be approximately 60 seconds
        assert!(remaining.num_seconds() > 55);
        assert!(remaining.num_seconds() <= 60);
    }

    #[test]
    fn test_lease_status_transitions() {
        let agent_id = Uuid::new_v4();
        let file_path = PathBuf::from("/tmp/test.txt");
        let ttl = Duration::seconds(60);

        let mut lease = Lease::new(file_path, agent_id, ttl, 5);

        // Initial status
        assert_eq!(lease.status, LeaseStatus::Pending);

        // Activate
        lease.activate();
        assert_eq!(lease.status, LeaseStatus::Active);

        // Release
        lease.mark_released();
        assert_eq!(lease.status, LeaseStatus::Released);
    }
}
