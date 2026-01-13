//! Registry for active Swank sessions.

use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

use super::SwankClient;

/// Thread-safe registry mapping session IDs to SwankClient instances.
pub struct SwankSessionRegistry {
    sessions: DashMap<Uuid, Arc<SwankClient>>,
}

impl SwankSessionRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            sessions: DashMap::new(),
        }
    }

    /// Register a SwankClient for a session.
    pub fn insert(&self, session_id: Uuid, client: Arc<SwankClient>) {
        self.sessions.insert(session_id, client);
    }

    /// Get a SwankClient by session ID.
    pub fn get(&self, session_id: &Uuid) -> Option<Arc<SwankClient>> {
        self.sessions.get(session_id).map(|r| Arc::clone(&r))
    }

    /// Remove a session from the registry.
    pub fn remove(&self, session_id: &Uuid) -> Option<Arc<SwankClient>> {
        self.sessions.remove(session_id).map(|(_, v)| v)
    }

    /// Check if a session exists.
    pub fn contains(&self, session_id: &Uuid) -> bool {
        self.sessions.contains_key(session_id)
    }

    /// Get the number of active sessions.
    pub fn len(&self) -> usize {
        self.sessions.len()
    }

    /// Check if the registry is empty.
    pub fn is_empty(&self) -> bool {
        self.sessions.is_empty()
    }

    /// List all session IDs.
    pub fn list_sessions(&self) -> Vec<Uuid> {
        self.sessions.iter().map(|r| *r.key()).collect()
    }
}

impl Default for SwankSessionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_operations() {
        let registry = SwankSessionRegistry::new();
        let id = Uuid::new_v4();

        assert!(registry.is_empty());
        assert!(!registry.contains(&id));

        // We can't easily test insert/get without a real SwankClient,
        // but we can verify the registry structure works
        assert_eq!(registry.len(), 0);
    }

    #[test]
    fn test_registry_default() {
        let registry = SwankSessionRegistry::default();
        assert!(registry.is_empty());
    }

    #[test]
    fn test_list_sessions_empty() {
        let registry = SwankSessionRegistry::new();
        assert!(registry.list_sessions().is_empty());
    }
}
