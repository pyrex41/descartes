/// Authentication and authorization
use crate::config::AuthConfig;
use crate::errors::{DaemonError, DaemonResult};
use crate::types::AuthToken;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: i64,
    pub iat: i64,
    pub scope: Vec<String>,
}

/// Authentication manager
pub struct AuthManager {
    config: AuthConfig,
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl AuthManager {
    /// Create a new authentication manager
    pub fn new(config: AuthConfig) -> DaemonResult<Self> {
        let encoding_key = EncodingKey::from_secret(config.jwt_secret.as_bytes());
        let decoding_key = DecodingKey::from_secret(config.jwt_secret.as_bytes());
        Ok(AuthManager {
            config,
            encoding_key,
            decoding_key,
        })
    }

    /// Generate a token
    pub fn generate_token(&self, sub: &str, scope: Vec<String>) -> DaemonResult<AuthToken> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.token_expiry_secs as i64);

        let claims = Claims {
            sub: sub.to_string(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            scope,
        };

        let token = encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| DaemonError::AuthError(format!("Token generation failed: {}", e)))?;

        Ok(AuthToken {
            token,
            expires_at: exp,
            scope: claims.scope,
        })
    }

    /// Verify a token
    pub fn verify_token(&self, token: &str) -> DaemonResult<Claims> {
        decode::<Claims>(token, &self.decoding_key, &Validation::default())
            .map(|data| data.claims)
            .map_err(|e| DaemonError::AuthError(format!("Token verification failed: {}", e)))
    }

    /// Verify API key
    pub fn verify_api_key(&self, key: &str) -> DaemonResult<()> {
        match &self.config.api_key {
            Some(stored_key) if stored_key == key => Ok(()),
            _ => Err(DaemonError::AuthError("Invalid API key".to_string())),
        }
    }

    /// Check if authentication is enabled
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }
}

/// Request authorization info
#[derive(Debug, Clone)]
pub struct AuthContext {
    pub user_id: String,
    pub scope: Vec<String>,
    pub authenticated: bool,
}

impl AuthContext {
    /// Create a new authenticated context
    pub fn new(user_id: String, scope: Vec<String>) -> Self {
        AuthContext {
            user_id,
            scope,
            authenticated: true,
        }
    }

    /// Create an unauthenticated context
    pub fn unauthenticated() -> Self {
        AuthContext {
            user_id: "anonymous".to_string(),
            scope: vec![],
            authenticated: false,
        }
    }

    /// Check if user has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scope.contains(&scope.to_string())
    }

    /// Check if user can perform an action
    pub fn can_perform(&self, action: &str) -> bool {
        if !self.authenticated {
            return false;
        }
        self.has_scope(action) || self.has_scope("*")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: "test-secret".to_string(),
            token_expiry_secs: 3600,
            api_key: None,
        };

        let manager = AuthManager::new(config).unwrap();
        let token = manager
            .generate_token("user1", vec!["read".to_string()])
            .unwrap();

        assert!(!token.token.is_empty());
        assert_eq!(token.scope, vec!["read"]);
    }

    #[test]
    fn test_token_verification() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: "test-secret".to_string(),
            token_expiry_secs: 3600,
            api_key: None,
        };

        let manager = AuthManager::new(config).unwrap();
        let token_info = manager
            .generate_token("user1", vec!["read".to_string()])
            .unwrap();
        let claims = manager.verify_token(&token_info.token).unwrap();

        assert_eq!(claims.sub, "user1");
        assert_eq!(claims.scope, vec!["read"]);
    }

    #[test]
    fn test_api_key_verification() {
        let config = AuthConfig {
            enabled: true,
            jwt_secret: "test-secret".to_string(),
            token_expiry_secs: 3600,
            api_key: Some("test-key".to_string()),
        };

        let manager = AuthManager::new(config).unwrap();

        assert!(manager.verify_api_key("test-key").is_ok());
        assert!(manager.verify_api_key("wrong-key").is_err());
    }

    #[test]
    fn test_auth_context() {
        let ctx = AuthContext::new(
            "user1".to_string(),
            vec!["agent:read".to_string(), "agent:write".to_string()],
        );

        assert!(ctx.authenticated);
        assert!(ctx.can_perform("agent:read"));
        assert!(ctx.can_perform("agent:write"));
        assert!(!ctx.can_perform("admin"));
    }

    #[test]
    fn test_auth_context_wildcard() {
        let ctx = AuthContext::new("admin".to_string(), vec!["*".to_string()]);

        assert!(ctx.can_perform("any:action"));
        assert!(ctx.can_perform("admin:action"));
    }
}
