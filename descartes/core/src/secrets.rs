/// Secure secret management module for encrypted storage.
/// Implements AES-256-GCM encryption with key derivation from master password.
/// Supports multiple secret types, versioning, and audit logging.

use crate::errors::{StateStoreError, StateStoreResult};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;
use zeroize::Zeroize;

/// Secret types supported by the system
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretType {
    /// API key for external services
    ApiKey,
    /// OAuth2 token
    OAuthToken,
    /// Database password
    DatabasePassword,
    /// Private key (crypto/SSH)
    PrivateKey,
    /// Custom/generic secret
    Custom,
}

impl SecretType {
    /// Get string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            SecretType::ApiKey => "api_key",
            SecretType::OAuthToken => "oauth_token",
            SecretType::DatabasePassword => "database_password",
            SecretType::PrivateKey => "private_key",
            SecretType::Custom => "custom",
        }
    }
}

/// Metadata about a secret (never encrypted)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretMetadata {
    /// Unique identifier for this secret
    pub id: Uuid,
    /// Human-readable name/identifier
    pub name: String,
    /// Type of secret
    pub secret_type: SecretType,
    /// Description/purpose
    pub description: Option<String>,
    /// Service/application this secret belongs to
    pub service: Option<String>,
    /// Current version number
    pub current_version: u32,
    /// When the secret was created
    pub created_at: DateTime<Utc>,
    /// When the secret was last updated
    pub updated_at: DateTime<Utc>,
    /// When the secret was last accessed
    pub last_accessed_at: Option<DateTime<Utc>>,
    /// When the secret expires (optional)
    pub expires_at: Option<DateTime<Utc>>,
    /// Tags for organizing secrets
    pub tags: Vec<String>,
    /// Is this secret currently active?
    pub is_active: bool,
}

/// Encrypted secret value and nonce (IV)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedSecretData {
    /// The encrypted secret value
    pub ciphertext: Vec<u8>,
    /// Nonce/IV used for encryption (must be unique per encryption)
    pub nonce: Vec<u8>,
    /// Authentication tag from AES-GCM
    pub tag: Vec<u8>,
    /// Version of the encryption scheme
    pub version: u8,
}

/// A secret entry with metadata and encrypted value
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    /// Metadata about the secret
    pub metadata: SecretMetadata,
    /// Encrypted secret data
    pub encrypted_data: EncryptedSecretData,
}

/// Secret version for rotation tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretVersion {
    /// Reference to parent secret
    pub secret_id: Uuid,
    /// Version number
    pub version: u32,
    /// Encrypted value for this version
    pub encrypted_data: EncryptedSecretData,
    /// When this version was created
    pub created_at: DateTime<Utc>,
    /// When this version was rotated/replaced
    pub rotated_at: Option<DateTime<Utc>>,
    /// Is this version still valid?
    pub is_valid: bool,
}

/// Access permission level for secrets
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    /// No access
    None,
    /// Can view secret metadata only
    ViewMetadata,
    /// Can read secret value
    Read,
    /// Can update secret value
    Update,
    /// Can delete secret
    Delete,
    /// Full administrative access
    Admin,
}

/// Access control entry for a secret
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessControlEntry {
    /// ID of the user/principal
    pub principal_id: String,
    /// Principal type (user, role, service)
    pub principal_type: PrincipalType,
    /// Permission level
    pub permission: PermissionLevel,
    /// When access was granted
    pub granted_at: DateTime<Utc>,
    /// When access expires (optional)
    pub expires_at: Option<DateTime<Utc>>,
    /// Reason for grant
    pub reason: Option<String>,
}

/// Type of principal for access control
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PrincipalType {
    /// Individual user
    User,
    /// Role-based access
    Role,
    /// Service account
    Service,
}

/// Audit log entry for secret access
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    /// Unique audit log ID
    pub id: Uuid,
    /// Secret ID being accessed
    pub secret_id: Uuid,
    /// User/principal performing the action
    pub principal_id: String,
    /// Type of action performed
    pub action: SecretAction,
    /// Whether action was successful
    pub success: bool,
    /// Error message if action failed
    pub error: Option<String>,
    /// Timestamp of the action
    pub timestamp: DateTime<Utc>,
    /// IP address or source identifier
    pub source: Option<String>,
    /// Additional context
    pub context: Option<String>,
}

/// Types of actions that can be performed on secrets
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretAction {
    /// Secret created
    Create,
    /// Secret value read
    Read,
    /// Secret value updated
    Update,
    /// Secret deleted
    Delete,
    /// Secret version rotated
    Rotate,
    /// Metadata modified
    ModifyMetadata,
    /// Access control changed
    ChangePermissions,
    /// Secret exported
    Export,
    /// Access attempt denied
    AccessDenied,
}

/// Key derivation parameters (PBKDF2/Argon2)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyDerivationParams {
    /// Derivation algorithm version
    pub version: u8,
    /// Algorithm name (pbkdf2, argon2id)
    pub algorithm: String,
    /// Salt for key derivation (random bytes)
    pub salt: Vec<u8>,
    /// For PBKDF2: iteration count
    pub iterations: Option<u32>,
    /// For Argon2: memory in KiB
    pub memory_cost: Option<u32>,
    /// For Argon2: parallelism factor
    pub parallelism: Option<u32>,
    /// Output key length in bytes
    pub key_length: u32,
}

impl Default for KeyDerivationParams {
    fn default() -> Self {
        Self {
            version: 1,
            algorithm: "argon2id".to_string(),
            salt: vec![],
            iterations: None,
            memory_cost: Some(19456), // 19 MiB
            parallelism: Some(1),
            key_length: 32, // 256 bits
        }
    }
}

/// Master key information (metadata only, never expose actual key)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasterKeyInfo {
    /// Unique identifier for the master key
    pub id: Uuid,
    /// Key derivation parameters
    pub key_derivation: KeyDerivationParams,
    /// Hash of master password (for verification without storing password)
    pub password_hash: Vec<u8>,
    /// When the master key was created
    pub created_at: DateTime<Utc>,
    /// When the master key was last rotated
    pub last_rotated_at: Option<DateTime<Utc>>,
    /// Key rotation interval in days (0 = no auto-rotation)
    pub rotation_interval_days: u32,
    /// Is this master key active?
    pub is_active: bool,
}

/// Request to create a new secret
#[derive(Debug, Clone)]
pub struct CreateSecretRequest {
    /// Human-readable name
    pub name: String,
    /// The secret value (will be encrypted)
    pub value: Vec<u8>,
    /// Type of secret
    pub secret_type: SecretType,
    /// Optional description
    pub description: Option<String>,
    /// Optional service name
    pub service: Option<String>,
    /// Optional tags
    pub tags: Vec<String>,
    /// Optional expiration date
    pub expires_at: Option<DateTime<Utc>>,
}

/// Request to update a secret
#[derive(Debug, Clone)]
pub struct UpdateSecretRequest {
    /// New secret value (will be encrypted)
    pub value: Vec<u8>,
    /// Whether to create a new version
    pub rotate: bool,
    /// Optional description update
    pub description: Option<String>,
    /// Optional tag update
    pub tags: Option<Vec<String>>,
}

/// Secret Store trait - manages encrypted secrets
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Initialize the secret store with master key setup
    async fn initialize(&mut self, master_password: &str) -> StateStoreResult<()>;

    /// Verify master password
    async fn verify_master_password(&self, password: &str) -> StateStoreResult<bool>;

    /// Create a new secret
    async fn create_secret(&self, request: CreateSecretRequest) -> StateStoreResult<Secret>;

    /// Get a secret by ID (requires decryption)
    async fn get_secret(&self, secret_id: &Uuid) -> StateStoreResult<Option<Secret>>;

    /// Get secret metadata without decrypting value
    async fn get_secret_metadata(&self, secret_id: &Uuid) -> StateStoreResult<Option<SecretMetadata>>;

    /// List all secrets (metadata only)
    async fn list_secrets(&self) -> StateStoreResult<Vec<SecretMetadata>>;

    /// Search secrets by name
    async fn search_secrets(&self, query: &str) -> StateStoreResult<Vec<SecretMetadata>>;

    /// Search secrets by tag
    async fn search_by_tag(&self, tag: &str) -> StateStoreResult<Vec<SecretMetadata>>;

    /// Update secret value
    async fn update_secret(
        &self,
        secret_id: &Uuid,
        request: UpdateSecretRequest,
    ) -> StateStoreResult<Secret>;

    /// Delete a secret
    async fn delete_secret(&self, secret_id: &Uuid) -> StateStoreResult<()>;

    /// Get secret version history
    async fn get_secret_versions(&self, secret_id: &Uuid) -> StateStoreResult<Vec<SecretVersion>>;

    /// Rotate a secret to a new version
    async fn rotate_secret(
        &self,
        secret_id: &Uuid,
        new_value: Vec<u8>,
    ) -> StateStoreResult<SecretVersion>;

    /// Get specific secret version
    async fn get_secret_version(
        &self,
        secret_id: &Uuid,
        version: u32,
    ) -> StateStoreResult<Option<SecretVersion>>;

    /// Set access permissions for a secret
    async fn set_permission(
        &self,
        secret_id: &Uuid,
        access: AccessControlEntry,
    ) -> StateStoreResult<()>;

    /// Get access control for a secret
    async fn get_permissions(&self, secret_id: &Uuid) -> StateStoreResult<Vec<AccessControlEntry>>;

    /// Check if principal has permission for action
    async fn check_permission(
        &self,
        secret_id: &Uuid,
        principal_id: &str,
        permission: PermissionLevel,
    ) -> StateStoreResult<bool>;

    /// Get audit log for a secret
    async fn get_audit_log(&self, secret_id: &Uuid) -> StateStoreResult<Vec<AuditLogEntry>>;

    /// Log an access event
    async fn log_access(
        &self,
        secret_id: &Uuid,
        principal_id: &str,
        action: SecretAction,
        success: bool,
        error: Option<&str>,
    ) -> StateStoreResult<()>;

    /// Rotate master key
    async fn rotate_master_key(&mut self, new_password: &str) -> StateStoreResult<()>;

    /// Get master key info
    async fn get_master_key_info(&self) -> StateStoreResult<Option<MasterKeyInfo>>;

    /// Export secret (with audit log)
    async fn export_secret(&self, secret_id: &Uuid) -> StateStoreResult<Secret>;

    /// Clear sensitive data from memory (zeroize operations)
    fn clear_sensitive_data(&mut self) -> StateStoreResult<()>;
}

/// Encryption context for operations
#[derive(Debug)]
pub struct EncryptionContext {
    /// Derived encryption key (must be zeroed after use)
    pub key: Vec<u8>,
    /// Key derivation parameters
    pub kdf_params: KeyDerivationParams,
}

impl Drop for EncryptionContext {
    fn drop(&mut self) {
        // Zeroize sensitive data on drop
        self.key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_type_as_str() {
        assert_eq!(SecretType::ApiKey.as_str(), "api_key");
        assert_eq!(SecretType::OAuthToken.as_str(), "oauth_token");
        assert_eq!(SecretType::PrivateKey.as_str(), "private_key");
    }

    #[test]
    fn test_permission_level_ordering() {
        assert!(PermissionLevel::None < PermissionLevel::ViewMetadata);
        assert!(PermissionLevel::ViewMetadata < PermissionLevel::Read);
        assert!(PermissionLevel::Read < PermissionLevel::Update);
        assert!(PermissionLevel::Update < PermissionLevel::Delete);
        assert!(PermissionLevel::Delete < PermissionLevel::Admin);
    }

    #[test]
    fn test_key_derivation_params_default() {
        let params = KeyDerivationParams::default();
        assert_eq!(params.algorithm, "argon2id");
        assert_eq!(params.key_length, 32);
        assert_eq!(params.memory_cost, Some(19456));
    }
}
