use crate::config::MAJORITY_THRESHOLD;
use crate::config::{LATENCY_OK_THRESHOLD_MS, LATENCY_SLOW_THRESHOLD_MS};
use crate::monitor::{ConnectionStatus, NetworkStats, ProbeRound};
use chrono::Utc;

/// Determine if a probe round represents packet loss using majority voting
/// Packet loss is counted only if >= MAJORITY_THRESHOLD targets fail
pub fn is_packet_lost(round: &ProbeRound) -> bool {
    let failed_count = round.results.iter().filter(|r| !r.success).count();
    failed_count >= MAJORITY_THRESHOLD
}

/// Get the average latency from a single probe round (successful probes only)
fn get_round_latency(round: &ProbeRound) -> Option<f64> {
    let successful_latencies: Vec<f64> =
        round.results.iter().filter_map(|r| r.latency_ms).collect();

    if successful_latencies.is_empty() {
        None
    } else {
        Some(successful_latencies.iter().sum::<f64>() / successful_latencies.len() as f64)
    }
}

/// Determine status based on latency only
fn determine_status_from_latency(latency_ms: Option<f64>) -> ConnectionStatus {
    match latency_ms {
        None => ConnectionStatus::Disconnected,
        Some(lat) if lat > LATENCY_SLOW_THRESHOLD_MS => ConnectionStatus::Disconnected,
        Some(lat) if lat > LATENCY_OK_THRESHOLD_MS => ConnectionStatus::Slow,
        Some(_) => ConnectionStatus::Ok,
    }
}

/// Aggregate statistics from the LATEST probe round only
/// This provides instant, real-time status based on current network state
pub fn aggregate_stats(rounds: &[ProbeRound]) -> NetworkStats {
    if rounds.is_empty() {
        return NetworkStats {
            avg_latency_ms: 0.0,
            loss_pct: 0.0,
            jitter_ms: 0.0,
            status: ConnectionStatus::Disconnected,
            timestamp: Utc::now(),
        };
    }

    // Use only the LATEST round for instant status
    let latest_round = rounds.last().unwrap();

    // Check if this round is a packet loss (majority of targets failed)
    let is_lost = is_packet_lost(latest_round);

    // Get average latency from the latest round
    let avg_latency_ms = get_round_latency(latest_round).unwrap_or(f64::MAX);

    // Determine status based on current state
    let status = if is_lost {
        ConnectionStatus::Disconnected
    } else {
        determine_status_from_latency(Some(avg_latency_ms))
    };

    NetworkStats {
        avg_latency_ms: if avg_latency_ms == f64::MAX {
            0.0
        } else {
            avg_latency_ms
        },
        loss_pct: 0.0,  // Not tracking over time anymore
        jitter_ms: 0.0, // Not tracking over time anymore
        status,
        timestamp: Utc::now(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::PingResult;
    use chrono::Utc;

    fn create_ping_result(target: &str, success: bool, latency_ms: Option<f64>) -> PingResult {
        PingResult {
            target: target.to_string(),
            success,
            latency_ms,
            timestamp: Utc::now(),
        }
    }

    fn create_probe_round(results: Vec<(bool, Option<f64>)>) -> ProbeRound {
        let targets = [
            "8.8.8.8",
            "s3.amazonaws.com",
            "portal.azure.com",
            "1.1.1.1",
            "9.9.9.9",
            "208.67.222.222",
        ];
        let ping_results: Vec<PingResult> = results
            .into_iter()
            .enumerate()
            .map(|(i, (success, latency))| {
                create_ping_result(targets[i % targets.len()], success, latency)
            })
            .collect();

        ProbeRound {
            results: ping_results,
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_is_packet_lost_all_success() {
        let round = create_probe_round(vec![
            (true, Some(50.0)),
            (true, Some(60.0)),
            (true, Some(55.0)),
            (true, Some(58.0)),
            (true, Some(50.0)),
            (true, Some(52.0)),
        ]);
        assert!(!is_packet_lost(&round));
    }

    #[test]
    fn test_is_packet_lost_one_failure() {
        let round = create_probe_round(vec![
            (false, None),
            (true, Some(60.0)),
            (true, Some(55.0)),
            (true, Some(58.0)),
            (true, Some(50.0)),
            (true, Some(52.0)),
        ]);
        // Only 1 failure, below threshold of 4
        assert!(!is_packet_lost(&round));
    }

    #[test]
    fn test_is_packet_lost_three_failures() {
        let round = create_probe_round(vec![
            (false, None),
            (false, None),
            (false, None),
            (true, Some(58.0)),
            (true, Some(50.0)),
            (true, Some(52.0)),
        ]);
        // 3 failures, still below threshold of 4 for 6 targets
        assert!(!is_packet_lost(&round));
    }

    #[test]
    fn test_is_packet_lost_majority_failure() {
        let round = create_probe_round(vec![
            (false, None),
            (false, None),
            (false, None),
            (false, None),
            (true, Some(50.0)),
            (true, Some(52.0)),
        ]);
        // 4 failures, meets threshold
        assert!(is_packet_lost(&round));
    }

    #[test]
    fn test_is_packet_lost_all_failure() {
        let round = create_probe_round(vec![
            (false, None),
            (false, None),
            (false, None),
            (false, None),
            (false, None),
            (false, None),
        ]);
        assert!(is_packet_lost(&round));
    }

    #[test]
    fn test_aggregate_stats_empty() {
        let stats = aggregate_stats(&[]);
        assert_eq!(stats.status, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_aggregate_stats_ok_latency() {
        let rounds = vec![create_probe_round(vec![
            (true, Some(50.0)),
            (true, Some(60.0)),
            (true, Some(55.0)),
            (true, Some(55.0)),
            (true, Some(55.0)),
            (true, Some(55.0)),
        ])];

        let stats = aggregate_stats(&rounds);
        assert_eq!(stats.status, ConnectionStatus::Ok);
        assert!((stats.avg_latency_ms - 55.0).abs() < 5.0);
    }

    #[test]
    fn test_aggregate_stats_slow_latency() {
        let rounds = vec![create_probe_round(vec![
            (true, Some(150.0)),
            (true, Some(160.0)),
            (true, Some(155.0)),
            (true, Some(155.0)),
            (true, Some(155.0)),
            (true, Some(155.0)),
        ])];

        let stats = aggregate_stats(&rounds);
        assert_eq!(stats.status, ConnectionStatus::Slow);
    }

    #[test]
    fn test_aggregate_stats_disconnected_by_latency() {
        let rounds = vec![create_probe_round(vec![
            (true, Some(350.0)),
            (true, Some(360.0)),
            (true, Some(355.0)),
            (true, Some(355.0)),
            (true, Some(355.0)),
            (true, Some(355.0)),
        ])];

        let stats = aggregate_stats(&rounds);
        assert_eq!(stats.status, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_aggregate_stats_disconnected_by_loss() {
        let rounds = vec![create_probe_round(vec![
            (false, None),
            (false, None),
            (false, None),
            (false, None),
            (true, Some(50.0)),
            (true, Some(52.0)),
        ])];

        let stats = aggregate_stats(&rounds);
        // Majority failed = disconnected
        assert_eq!(stats.status, ConnectionStatus::Disconnected);
    }

    #[test]
    fn test_instant_recovery() {
        // First round: disconnected
        let round1 = create_probe_round(vec![
            (false, None),
            (false, None),
            (false, None),
            (false, None),
            (false, None),
            (false, None),
        ]);
        let stats1 = aggregate_stats(&[round1.clone()]);
        assert_eq!(stats1.status, ConnectionStatus::Disconnected);

        // Second round: OK - should IMMEDIATELY show OK
        let round2 = create_probe_round(vec![
            (true, Some(50.0)),
            (true, Some(60.0)),
            (true, Some(55.0)),
            (true, Some(55.0)),
            (true, Some(55.0)),
            (true, Some(55.0)),
        ]);
        let stats2 = aggregate_stats(&[round1, round2]);
        assert_eq!(stats2.status, ConnectionStatus::Ok);
    }
}
