use crate::config::{LATENCY_OK_THRESHOLD_MS, LATENCY_SLOW_THRESHOLD_MS};
use crate::monitor::ConnectionStatus;

/// Determine connection status based on latency only
///
/// Status thresholds:
/// - OK: latency <100ms
/// - Slow: latency 100-300ms
/// - Disconnected: latency >300ms or no response
///
/// Note: This function is preserved for testing purposes. The actual status
/// determination happens in stats.rs via determine_status_from_latency().
#[allow(dead_code)]
pub fn determine_status(avg_latency_ms: f64) -> ConnectionStatus {
    if avg_latency_ms > LATENCY_SLOW_THRESHOLD_MS {
        ConnectionStatus::Disconnected
    } else if avg_latency_ms >= LATENCY_OK_THRESHOLD_MS {
        ConnectionStatus::Slow
    } else {
        ConnectionStatus::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_ok_all_good() {
        let status = determine_status(50.0);
        assert_eq!(status, ConnectionStatus::Ok);
    }

    #[test]
    fn test_status_ok_at_boundaries() {
        let status = determine_status(99.0);
        assert_eq!(status, ConnectionStatus::Ok);
    }

    #[test]
    fn test_status_slow_latency() {
        let status = determine_status(150.0);
        assert_eq!(status, ConnectionStatus::Slow);
    }

    #[test]
    fn test_status_slow_at_ok_threshold() {
        // Latency exactly at OK threshold should be slow
        let status = determine_status(100.0);
        assert_eq!(status, ConnectionStatus::Slow);
    }

    #[test]
    fn test_status_disconnected_latency() {
        let status = determine_status(400.0);
        assert_eq!(status, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_status_slow_at_slow_threshold() {
        // Latency exactly at slow threshold (300ms) should be slow, not disconnected
        let status = determine_status(300.0);
        assert_eq!(status, ConnectionStatus::Slow);
    }

    #[test]
    fn test_status_disconnected_all_bad() {
        let status = determine_status(500.0);
        assert_eq!(status, ConnectionStatus::Disconnected);
    }
}
