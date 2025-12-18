/// Metrics collection and exposure
use crate::errors::{DaemonError, DaemonResult};
use crate::types::{MetricsAgents, MetricsResponse, MetricsSystem};
use chrono::Utc;
use prometheus::{Counter, Encoder, Histogram, HistogramOpts, IntGauge, Registry};
use std::sync::Arc;
use std::time::Instant;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

/// Metrics collector
pub struct MetricsCollector {
    registry: Arc<Registry>,

    // Request metrics
    pub request_total: Counter,
    pub request_duration: Histogram,
    pub request_errors: Counter,

    // Agent metrics
    pub agents_spawned: Counter,
    pub agents_killed: Counter,
    pub agents_active: IntGauge,

    // Connection metrics
    pub connections_total: Counter,
    pub connections_active: IntGauge,

    // Server metrics
    pub server_uptime_secs: Arc<std::sync::atomic::AtomicU64>,
    pub server_start: Instant,

    // System info for CPU/memory tracking
    sysinfo: std::sync::Mutex<System>,
    pid: Pid,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new() -> DaemonResult<Self> {
        let registry = Arc::new(Registry::new());

        let request_total = Counter::new("requests_total", "Total requests")
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(request_total.clone()))
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;

        let opts = HistogramOpts::new("request_duration_seconds", "Request duration");
        let request_duration =
            Histogram::with_opts(opts).map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(request_duration.clone()))
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;

        let request_errors = Counter::new("request_errors_total", "Total request errors")
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(request_errors.clone()))
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;

        let agents_spawned = Counter::new("agents_spawned_total", "Total agents spawned")
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(agents_spawned.clone()))
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;

        let agents_killed = Counter::new("agents_killed_total", "Total agents killed")
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(agents_killed.clone()))
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;

        let agents_active = IntGauge::new("agents_active", "Active agents")
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(agents_active.clone()))
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;

        let connections_total = Counter::new("connections_total", "Total connections")
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(connections_total.clone()))
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;

        let connections_active = IntGauge::new("connections_active", "Active connections")
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        registry
            .register(Box::new(connections_active.clone()))
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;

        // Initialize sysinfo for this process
        let pid = Pid::from_u32(std::process::id());
        let sysinfo = System::new_all();

        Ok(MetricsCollector {
            registry,
            request_total,
            request_duration,
            request_errors,
            agents_spawned,
            agents_killed,
            agents_active,
            connections_total,
            connections_active,
            server_uptime_secs: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            server_start: Instant::now(),
            sysinfo: std::sync::Mutex::new(sysinfo),
            pid,
        })
    }

    /// Record a request
    pub fn record_request(&self, duration_secs: f64) {
        self.request_total.inc();
        self.request_duration.observe(duration_secs);
    }

    /// Record a request error
    pub fn record_error(&self) {
        self.request_errors.inc();
    }

    /// Record agent spawn
    pub fn record_agent_spawn(&self) {
        self.agents_spawned.inc();
        self.agents_active.inc();
    }

    /// Record agent kill
    pub fn record_agent_kill(&self) {
        self.agents_killed.inc();
        self.agents_active.dec();
    }

    /// Record new connection
    pub fn record_connection(&self) {
        self.connections_total.inc();
        self.connections_active.inc();
    }

    /// Record connection closed
    pub fn record_connection_closed(&self) {
        self.connections_active.dec();
    }

    /// Get all metrics in Prometheus format
    pub fn gather_metrics(&self) -> DaemonResult<String> {
        let metrics = self.registry.gather();
        let mut buffer = Vec::new();
        let encoder = prometheus::TextEncoder::new();
        encoder
            .encode(&metrics, &mut buffer)
            .map_err(|e| DaemonError::MetricsError(e.to_string()))?;
        String::from_utf8(buffer).map_err(|e| DaemonError::MetricsError(e.to_string()))
    }

    /// Get metrics response
    pub fn get_metrics_response(&self) -> MetricsResponse {
        let agents = MetricsAgents {
            total: (self.agents_spawned.get() as usize),
            running: self.agents_active.get() as usize,
            paused: 0,
            stopped: 0,
            failed: 0,
        };

        let uptime_secs = self.server_start.elapsed().as_secs();
        self.server_uptime_secs
            .store(uptime_secs, std::sync::atomic::Ordering::Relaxed);

        // Get actual CPU and memory usage for this process
        let (memory_usage_mb, cpu_usage_percent) = {
            let mut sys = self.sysinfo.lock().unwrap();
            // Refresh process-specific data for our PID only
            sys.refresh_processes_specifics(
                ProcessesToUpdate::Some(&[self.pid]),
                false, // Don't remove dead processes
                ProcessRefreshKind::everything(),
            );

            if let Some(process) = sys.process(self.pid) {
                let memory_mb = process.memory() as f64 / (1024.0 * 1024.0);
                let cpu_percent = process.cpu_usage() as f64;
                (memory_mb, cpu_percent)
            } else {
                (0.0, 0.0)
            }
        };

        let system = MetricsSystem {
            uptime_secs,
            memory_usage_mb,
            cpu_usage_percent,
            active_connections: self.connections_active.get() as usize,
        };

        MetricsResponse {
            agents,
            system,
            timestamp: Utc::now(),
        }
    }

    /// Get the registry
    pub fn registry(&self) -> Arc<Registry> {
        self.registry.clone()
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new().unwrap_or_else(|e| {
            tracing::error!("Failed to create metrics collector: {:?}", e);
            panic!("Cannot create metrics collector");
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = MetricsCollector::new().unwrap();
        assert!(metrics.gather_metrics().is_ok());
    }

    #[test]
    fn test_request_recording() {
        let metrics = MetricsCollector::new().unwrap();
        metrics.record_request(0.5);
        assert_eq!(metrics.request_total.get(), 1.0);
    }

    #[test]
    fn test_agent_recording() {
        let metrics = MetricsCollector::new().unwrap();
        metrics.record_agent_spawn();
        assert_eq!(metrics.agents_active.get(), 1);

        metrics.record_agent_kill();
        assert_eq!(metrics.agents_active.get(), 0);
    }
}
