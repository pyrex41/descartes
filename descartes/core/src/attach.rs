//! Attachment token management for paused agent sessions.
//!
//! This module provides token generation, validation, and lifecycle management
//! for attaching external TUIs (like Claude Code or OpenCode) to paused agents.

use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Default TTL for attach tokens (5 minutes)
pub const DEFAULT_TOKEN_TTL_SECS: i64 = 300;

/// Attachment token with expiration and revocation tracking.
#[derive(Debug, Clone)]
pub struct AttachToken {
    /// The token string (UUID format)
    pub token: String,
    /// The agent this token is for
    pub agent_id: Uuid,
    /// When the token was created
    pub created_at: DateTime<Utc>,
    /// When the token expires
    pub expires_at: DateTime<Utc>,
    /// Whether the token has been explicitly revoked
    pub revoked: bool,
}

impl AttachToken {
    /// Create a new attach token for an agent.
    pub fn new(agent_id: Uuid, ttl_secs: i64) -> Self {
        let now = Utc::now();
        Self {
            token: Uuid::new_v4().to_string(),
            agent_id,
            created_at: now,
            expires_at: now + Duration::seconds(ttl_secs),
            revoked: false,
        }
    }

    /// Check if the token is valid (not expired and not revoked).
    pub fn is_valid(&self) -> bool {
        !self.revoked && Utc::now() < self.expires_at
    }

    /// Get the remaining TTL in seconds.
    pub fn remaining_secs(&self) -> i64 {
        let remaining = self.expires_at - Utc::now();
        remaining.num_seconds().max(0)
    }

    /// Get expiration as Unix timestamp.
    pub fn expires_at_unix(&self) -> i64 {
        self.expires_at.timestamp()
    }
}

/// Thread-safe store for managing attach tokens.
///
/// Provides token generation, validation, revocation, and automatic cleanup
/// of expired tokens.
pub struct AttachTokenStore {
    /// Map from token string to AttachToken
    tokens: Arc<RwLock<HashMap<String, AttachToken>>>,
    /// Map from agent_id to token strings (for quick lookup)
    agent_tokens: Arc<RwLock<HashMap<Uuid, Vec<String>>>>,
    /// Default TTL for new tokens
    ttl_secs: i64,
}

impl AttachTokenStore {
    /// Create a new token store with default TTL.
    pub fn new() -> Self {
        Self::with_ttl(DEFAULT_TOKEN_TTL_SECS)
    }

    /// Create a new token store with custom TTL.
    pub fn with_ttl(ttl_secs: i64) -> Self {
        Self {
            tokens: Arc::new(RwLock::new(HashMap::new())),
            agent_tokens: Arc::new(RwLock::new(HashMap::new())),
            ttl_secs,
        }
    }

    /// Generate a new attach token for an agent.
    ///
    /// Returns the generated token with connection info.
    pub async fn generate(&self, agent_id: Uuid) -> AttachToken {
        let token = AttachToken::new(agent_id, self.ttl_secs);
        let token_str = token.token.clone();

        // Store the token
        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(token_str.clone(), token.clone());
        }

        // Track agent -> token mapping
        {
            let mut agent_tokens = self.agent_tokens.write().await;
            agent_tokens
                .entry(agent_id)
                .or_insert_with(Vec::new)
                .push(token_str);
        }

        tracing::info!(
            agent_id = %agent_id,
            token = %token.token,
            expires_in_secs = token.remaining_secs(),
            "Generated attach token"
        );

        token
    }

    /// Generate a token with custom TTL.
    pub async fn generate_with_ttl(&self, agent_id: Uuid, ttl_secs: i64) -> AttachToken {
        let token = AttachToken::new(agent_id, ttl_secs);
        let token_str = token.token.clone();

        {
            let mut tokens = self.tokens.write().await;
            tokens.insert(token_str.clone(), token.clone());
        }

        {
            let mut agent_tokens = self.agent_tokens.write().await;
            agent_tokens
                .entry(agent_id)
                .or_insert_with(Vec::new)
                .push(token_str);
        }

        token
    }

    /// Validate a token and return the agent_id if valid.
    ///
    /// Returns None if the token is invalid, expired, or revoked.
    pub async fn validate(&self, token: &str) -> Option<Uuid> {
        let tokens = self.tokens.read().await;
        if let Some(attach_token) = tokens.get(token) {
            if attach_token.is_valid() {
                return Some(attach_token.agent_id);
            }
        }
        None
    }

    /// Get full token info if valid.
    pub async fn get_token(&self, token: &str) -> Option<AttachToken> {
        let tokens = self.tokens.read().await;
        tokens.get(token).filter(|t| t.is_valid()).cloned()
    }

    /// Revoke a specific token.
    ///
    /// Returns true if the token was found and revoked.
    pub async fn revoke(&self, token: &str) -> bool {
        let mut tokens = self.tokens.write().await;
        if let Some(attach_token) = tokens.get_mut(token) {
            attach_token.revoked = true;
            tracing::info!(token = %token, "Revoked attach token");
            true
        } else {
            false
        }
    }

    /// Revoke all tokens for a specific agent.
    ///
    /// Returns the number of tokens revoked.
    pub async fn revoke_for_agent(&self, agent_id: &Uuid) -> usize {
        let token_strs: Vec<String> = {
            let agent_tokens = self.agent_tokens.read().await;
            agent_tokens.get(agent_id).cloned().unwrap_or_default()
        };

        let mut revoked_count = 0;
        let mut tokens = self.tokens.write().await;
        for token_str in &token_strs {
            if let Some(token) = tokens.get_mut(token_str) {
                if !token.revoked {
                    token.revoked = true;
                    revoked_count += 1;
                }
            }
        }

        if revoked_count > 0 {
            tracing::info!(
                agent_id = %agent_id,
                count = revoked_count,
                "Revoked all tokens for agent"
            );
        }

        revoked_count
    }

    /// Cleanup expired and revoked tokens.
    ///
    /// Returns the number of tokens removed.
    pub async fn cleanup_expired(&self) -> usize {
        let now = Utc::now();
        let mut removed_count = 0;

        // Get tokens to remove
        let tokens_to_remove: Vec<(String, Uuid)> = {
            let tokens = self.tokens.read().await;
            tokens
                .iter()
                .filter(|(_, t)| t.revoked || now >= t.expires_at)
                .map(|(k, t)| (k.clone(), t.agent_id))
                .collect()
        };

        if !tokens_to_remove.is_empty() {
            // Remove from tokens map
            {
                let mut tokens = self.tokens.write().await;
                for (token_str, _) in &tokens_to_remove {
                    tokens.remove(token_str);
                    removed_count += 1;
                }
            }

            // Remove from agent_tokens map
            {
                let mut agent_tokens = self.agent_tokens.write().await;
                for (token_str, agent_id) in &tokens_to_remove {
                    if let Some(token_list) = agent_tokens.get_mut(agent_id) {
                        token_list.retain(|t| t != token_str);
                        if token_list.is_empty() {
                            agent_tokens.remove(agent_id);
                        }
                    }
                }
            }

            tracing::debug!(count = removed_count, "Cleaned up expired tokens");
        }

        removed_count
    }

    /// Get all valid tokens for an agent.
    pub async fn get_tokens_for_agent(&self, agent_id: &Uuid) -> Vec<AttachToken> {
        let token_strs: Vec<String> = {
            let agent_tokens = self.agent_tokens.read().await;
            agent_tokens.get(agent_id).cloned().unwrap_or_default()
        };

        let tokens = self.tokens.read().await;
        token_strs
            .iter()
            .filter_map(|ts| tokens.get(ts))
            .filter(|t| t.is_valid())
            .cloned()
            .collect()
    }

    /// Get total token count (including expired/revoked).
    pub async fn len(&self) -> usize {
        self.tokens.read().await.len()
    }

    /// Check if store is empty.
    pub async fn is_empty(&self) -> bool {
        self.tokens.read().await.is_empty()
    }

    /// Get count of valid (non-expired, non-revoked) tokens.
    pub async fn valid_count(&self) -> usize {
        self.tokens
            .read()
            .await
            .values()
            .filter(|t| t.is_valid())
            .count()
    }

    /// Start a background task to periodically cleanup expired tokens.
    ///
    /// Runs cleanup every `interval_secs` seconds.
    pub fn start_cleanup_task(self: &Arc<Self>, interval_secs: u64) {
        let store = Arc::clone(self);
        tokio::spawn(async move {
            let mut interval =
                tokio::time::interval(tokio::time::Duration::from_secs(interval_secs));
            loop {
                interval.tick().await;
                let removed = store.cleanup_expired().await;
                if removed > 0 {
                    tracing::debug!(
                        removed = removed,
                        "Periodic token cleanup completed"
                    );
                }
            }
        });
    }
}

impl Default for AttachTokenStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_token_generation() {
        let store = AttachTokenStore::new();
        let agent_id = Uuid::new_v4();

        let token = store.generate(agent_id).await;

        assert_eq!(token.agent_id, agent_id);
        assert!(token.is_valid());
        assert!(!token.revoked);
        assert!(token.remaining_secs() > 0);
    }

    #[tokio::test]
    async fn test_token_validation() {
        let store = AttachTokenStore::new();
        let agent_id = Uuid::new_v4();

        let token = store.generate(agent_id).await;
        let validated_agent = store.validate(&token.token).await;

        assert_eq!(validated_agent, Some(agent_id));
    }

    #[tokio::test]
    async fn test_invalid_token_validation() {
        let store = AttachTokenStore::new();

        let result = store.validate("invalid-token").await;

        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_token_revocation() {
        let store = AttachTokenStore::new();
        let agent_id = Uuid::new_v4();

        let token = store.generate(agent_id).await;

        // Token should be valid before revocation
        assert!(store.validate(&token.token).await.is_some());

        // Revoke the token
        let revoked = store.revoke(&token.token).await;
        assert!(revoked);

        // Token should be invalid after revocation
        assert!(store.validate(&token.token).await.is_none());
    }

    #[tokio::test]
    async fn test_revoke_for_agent() {
        let store = AttachTokenStore::new();
        let agent_id = Uuid::new_v4();

        // Generate multiple tokens
        let token1 = store.generate(agent_id).await;
        let token2 = store.generate(agent_id).await;

        // Both should be valid
        assert!(store.validate(&token1.token).await.is_some());
        assert!(store.validate(&token2.token).await.is_some());

        // Revoke all for agent
        let count = store.revoke_for_agent(&agent_id).await;
        assert_eq!(count, 2);

        // Both should be invalid
        assert!(store.validate(&token1.token).await.is_none());
        assert!(store.validate(&token2.token).await.is_none());
    }

    #[tokio::test]
    async fn test_token_expiration() {
        // Create store with 1 second TTL
        let store = AttachTokenStore::with_ttl(1);
        let agent_id = Uuid::new_v4();

        let token = store.generate(agent_id).await;
        assert!(store.validate(&token.token).await.is_some());

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Token should be expired
        assert!(store.validate(&token.token).await.is_none());
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let store = AttachTokenStore::with_ttl(1);
        let agent_id = Uuid::new_v4();

        store.generate(agent_id).await;
        assert_eq!(store.len().await, 1);

        // Wait for expiration
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

        // Cleanup
        let removed = store.cleanup_expired().await;
        assert_eq!(removed, 1);
        assert_eq!(store.len().await, 0);
    }

    #[tokio::test]
    async fn test_get_tokens_for_agent() {
        let store = AttachTokenStore::new();
        let agent_id = Uuid::new_v4();
        let other_agent_id = Uuid::new_v4();

        store.generate(agent_id).await;
        store.generate(agent_id).await;
        store.generate(other_agent_id).await;

        let tokens = store.get_tokens_for_agent(&agent_id).await;
        assert_eq!(tokens.len(), 2);

        let other_tokens = store.get_tokens_for_agent(&other_agent_id).await;
        assert_eq!(other_tokens.len(), 1);
    }

    #[test]
    fn test_attach_token_remaining_secs() {
        let token = AttachToken::new(Uuid::new_v4(), 300);
        let remaining = token.remaining_secs();
        // Should be close to 300 (allow for some time passing)
        assert!(remaining >= 299 && remaining <= 300);
    }

    #[test]
    fn test_attach_token_expires_at_unix() {
        let token = AttachToken::new(Uuid::new_v4(), 300);
        let expected = (Utc::now() + Duration::seconds(300)).timestamp();
        let actual = token.expires_at_unix();
        // Allow 1 second tolerance
        assert!((actual - expected).abs() <= 1);
    }
}
