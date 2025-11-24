/// Connection pool management

use crate::config::PoolConfig;
use crate::errors::{DaemonError, DaemonResult};
use crate::types::ConnectionInfo;
use chrono::Utc;
use dashmap::DashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use uuid::Uuid;

/// Connection pool
pub struct ConnectionPool {
    config: PoolConfig,
    connections: DashMap<String, ConnectionInfo>,
    active_count: Arc<AtomicU32>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: PoolConfig) -> Self {
        ConnectionPool {
            config,
            connections: DashMap::new(),
            active_count: Arc::new(AtomicU32::new(0)),
        }
    }

    /// Register a new connection
    pub fn register(&self, client_addr: String) -> DaemonResult<String> {
        let active = self.active_count.load(Ordering::SeqCst);
        if active >= self.config.max_size {
            return Err(DaemonError::PoolError("Connection pool is full".to_string()));
        }

        let conn_id = Uuid::new_v4().to_string();
        let now = Utc::now();

        let conn_info = ConnectionInfo {
            id: conn_id.clone(),
            client_addr,
            connected_at: now,
            last_activity: now,
        };

        self.connections.insert(conn_id.clone(), conn_info);
        self.active_count.fetch_add(1, Ordering::SeqCst);

        Ok(conn_id)
    }

    /// Unregister a connection
    pub fn unregister(&self, conn_id: &str) -> DaemonResult<()> {
        self.connections.remove(conn_id);
        self.active_count.fetch_sub(1, Ordering::SeqCst);
        Ok(())
    }

    /// Update last activity for a connection
    pub fn touch(&self, conn_id: &str) -> DaemonResult<()> {
        if let Some(mut conn) = self.connections.get_mut(conn_id) {
            conn.last_activity = Utc::now();
            Ok(())
        } else {
            Err(DaemonError::PoolError(format!(
                "Connection not found: {}",
                conn_id
            )))
        }
    }

    /// Get connection info
    pub fn get(&self, conn_id: &str) -> DaemonResult<ConnectionInfo> {
        self.connections
            .get(conn_id)
            .map(|r| r.clone())
            .ok_or_else(|| {
                DaemonError::PoolError(format!("Connection not found: {}", conn_id))
            })
    }

    /// Get all active connections
    pub fn get_all(&self) -> Vec<ConnectionInfo> {
        self.connections.iter().map(|r| r.clone()).collect()
    }

    /// Get active connection count
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::SeqCst) as usize
    }

    /// Check and remove idle connections
    pub fn cleanup_idle(&self) -> usize {
        let now = Utc::now();
        let timeout_secs = self.config.idle_timeout_secs as i64;
        let mut removed = 0;

        self.connections.retain(|_, conn| {
            let elapsed = now
                .signed_duration_since(conn.last_activity)
                .num_seconds();
            if elapsed > timeout_secs {
                removed += 1;
                self.active_count.fetch_sub(1, Ordering::SeqCst);
                false
            } else {
                true
            }
        });

        removed
    }

    /// Get pool statistics
    pub fn stats(&self) -> PoolStats {
        PoolStats {
            total_connections: self.connections.len(),
            active_connections: self.active_count.load(Ordering::SeqCst) as usize,
            max_connections: self.config.max_size as usize,
            utilization: (self.active_count.load(Ordering::SeqCst) as f64
                / self.config.max_size as f64)
                * 100.0,
        }
    }
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub active_connections: usize,
    pub max_connections: usize,
    pub utilization: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_registration() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);

        let conn_id = pool.register("127.0.0.1:12345".to_string()).unwrap();
        assert!(!conn_id.is_empty());
        assert_eq!(pool.active_count(), 1);
    }

    #[test]
    fn test_connection_unregistration() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);

        let conn_id = pool.register("127.0.0.1:12345".to_string()).unwrap();
        pool.unregister(&conn_id).unwrap();
        assert_eq!(pool.active_count(), 0);
    }

    #[test]
    fn test_pool_overflow() {
        let mut config = PoolConfig::default();
        config.max_size = 1;
        let pool = ConnectionPool::new(config);

        let _conn1 = pool.register("127.0.0.1:12345".to_string()).unwrap();
        let result = pool.register("127.0.0.1:12346".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_touch() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);

        let conn_id = pool.register("127.0.0.1:12345".to_string()).unwrap();
        let initial = pool.get(&conn_id).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(10));
        pool.touch(&conn_id).unwrap();

        let updated = pool.get(&conn_id).unwrap();
        assert!(updated.last_activity > initial.last_activity);
    }
}
