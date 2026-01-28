pub mod net_info;
pub mod prober;

use chrono::{DateTime, Utc};

/// Overall connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Connection is healthy
    Ok,
    /// Connection is degraded
    Slow,
    /// Connection is severely impaired or down
    Disconnected,
}

impl std::fmt::Display for ConnectionStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConnectionStatus::Ok => write!(f, "OK"),
            ConnectionStatus::Slow => write!(f, "SLOW"),
            ConnectionStatus::Disconnected => write!(f, "DISCONNECTED"),
        }
    }
}

/// Result of a single probe to one target
#[derive(Debug, Clone)]
pub struct PingResult {
    /// Target IP address (used for logging/debugging)
    #[allow(dead_code)]
    pub target: String,
    /// Whether the probe succeeded
    pub success: bool,
    /// Latency in milliseconds (None if failed)
    pub latency_ms: Option<f64>,
    /// Timestamp of the probe (used for logging/debugging)
    #[allow(dead_code)]
    pub timestamp: DateTime<Utc>,
}

/// Aggregated statistics for a time window
#[derive(Debug, Clone)]
pub struct NetworkStats {
    /// Average latency in milliseconds
    pub avg_latency_ms: f64,
    /// Packet loss percentage (0-100)
    pub loss_pct: f64,
    /// Jitter (latency variation) in milliseconds
    pub jitter_ms: f64,
    /// Overall connection status
    pub status: ConnectionStatus,
    /// Timestamp of the statistics
    pub timestamp: DateTime<Utc>,
}

/// Results from a single probe round to all targets
#[derive(Debug, Clone)]
pub struct ProbeRound {
    /// Results for each target
    pub results: Vec<PingResult>,
    /// Timestamp of the round (used for logging/debugging)
    #[allow(dead_code)]
    pub timestamp: DateTime<Utc>,
}
