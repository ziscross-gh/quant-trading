//! Health monitoring and recovery system
//!
//! Monitors system health and automatically recovers from errors.

use crate::{Error, Result};
use chrono::{DateTime, Duration, Utc};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Health monitor
pub struct HealthMonitor {
    last_heartbeat: Arc<AtomicU64>,
    is_healthy: Arc<AtomicBool>,
    error_count: Arc<AtomicU64>,
    max_consecutive_errors: u64,
}

impl HealthMonitor {
    /// Create new health monitor
    pub fn new(max_consecutive_errors: u64) -> Self {
        Self {
            last_heartbeat: Arc::new(AtomicU64::new(Utc::now().timestamp() as u64)),
            is_healthy: Arc::new(AtomicBool::new(true)),
            error_count: Arc::new(AtomicU64::new(0)),
            max_consecutive_errors,
        }
    }

    /// Record heartbeat
    pub fn heartbeat(&self) {
        self.last_heartbeat
            .store(Utc::now().timestamp() as u64, Ordering::SeqCst);
        self.is_healthy.store(true, Ordering::SeqCst);
    }

    /// Record error
    pub fn record_error(&self) {
        let count = self.error_count.fetch_add(1, Ordering::SeqCst) + 1;

        if count >= self.max_consecutive_errors {
            error!("Max consecutive errors reached: {}", count);
            self.is_healthy.store(false, Ordering::SeqCst);
        } else {
            warn!("Error recorded ({}/{})", count, self.max_consecutive_errors);
        }
    }

    /// Reset error count after successful operation
    pub fn reset_errors(&self) {
        let prev_count = self.error_count.swap(0, Ordering::SeqCst);
        if prev_count > 0 {
            info!("Error count reset after successful operation");
        }
    }

    /// Check if system is healthy
    pub fn is_healthy(&self) -> bool {
        let healthy = self.is_healthy.load(Ordering::SeqCst);
        let last_beat = self.last_heartbeat.load(Ordering::SeqCst) as i64;
        let now = Utc::now().timestamp();

        // Consider unhealthy if no heartbeat in 5 minutes
        let has_recent_heartbeat = (now - last_beat) < 300;

        healthy && has_recent_heartbeat
    }

    /// Get last heartbeat time
    pub fn last_heartbeat(&self) -> DateTime<Utc> {
        let timestamp = self.last_heartbeat.load(Ordering::SeqCst) as i64;
        DateTime::from_timestamp(timestamp, 0).unwrap_or(Utc::now())
    }

    /// Get error count
    pub fn error_count(&self) -> u64 {
        self.error_count.load(Ordering::SeqCst)
    }

    /// Get health status
    pub fn status(&self) -> HealthStatus {
        let healthy = self.is_healthy();
        let errors = self.error_count();
        let last_beat = self.last_heartbeat();
        let since_heartbeat = Utc::now().signed_duration_since(last_beat);

        HealthStatus {
            healthy,
            error_count: errors,
            last_heartbeat: last_beat,
            seconds_since_heartbeat: since_heartbeat.num_seconds(),
        }
    }
}

/// Health status snapshot
#[derive(Debug, Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub error_count: u64,
    pub last_heartbeat: DateTime<Utc>,
    pub seconds_since_heartbeat: i64,
}

impl HealthStatus {
    /// Get status message
    pub fn message(&self) -> String {
        if self.healthy {
            format!(
                "✅ HEALTHY - {} errors, last heartbeat {}s ago",
                self.error_count, self.seconds_since_heartbeat
            )
        } else {
            format!(
                "🔴 UNHEALTHY - {} errors, last heartbeat {}s ago",
                self.error_count, self.seconds_since_heartbeat
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_monitor() {
        let monitor = HealthMonitor::new(3);

        // Initially healthy
        assert!(monitor.is_healthy());
        assert_eq!(monitor.error_count(), 0);

        // Record some errors
        monitor.record_error();
        monitor.record_error();
        assert!(monitor.is_healthy());
        assert_eq!(monitor.error_count(), 2);

        // Reach max errors
        monitor.record_error();
        assert!(!monitor.is_healthy());
        assert_eq!(monitor.error_count(), 3);

        // Reset
        monitor.reset_errors();
        assert_eq!(monitor.error_count(), 0);
    }

    #[test]
    fn test_heartbeat() {
        let monitor = HealthMonitor::new(5);
        monitor.heartbeat();
        assert!(monitor.is_healthy());

        let status = monitor.status();
        assert!(status.healthy);
        assert!(status.seconds_since_heartbeat < 2);
    }
}
