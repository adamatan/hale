use crate::config::{PROBE_INTERVAL_MS, PROBE_TIMEOUT_MS, TARGETS, TARGET_PORT};
use crate::monitor::{PingResult, ProbeRound};
use chrono::Utc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::time::{timeout, Instant};

/// Probe a single target with TCP connect
async fn probe_target(target: &str, port: u16) -> PingResult {
    let start = Instant::now();
    let addr = format!("{}:{}", target, port);

    let result = timeout(
        Duration::from_millis(PROBE_TIMEOUT_MS),
        TcpStream::connect(&addr),
    )
    .await;

    let elapsed = start.elapsed();
    let latency_ms = elapsed.as_secs_f64() * 1000.0;

    let success = result.is_ok() && result.unwrap().is_ok();

    PingResult {
        target: target.to_string(),
        success,
        latency_ms: if success { Some(latency_ms) } else { None },
        timestamp: Utc::now(),
    }
}

/// Probe all targets concurrently
async fn probe_all_targets() -> ProbeRound {
    let mut tasks = Vec::new();

    for &target in TARGETS.iter() {
        let task = tokio::spawn(probe_target(target, TARGET_PORT));
        tasks.push(task);
    }

    let mut results = Vec::new();
    for task in tasks {
        if let Ok(result) = task.await {
            results.push(result);
        }
    }

    ProbeRound {
        results,
        timestamp: Utc::now(),
    }
}

/// Run the probe loop, sending results through a channel
pub async fn run_probe_loop(tx: mpsc::Sender<ProbeRound>) {
    let mut interval = tokio::time::interval(Duration::from_millis(PROBE_INTERVAL_MS));

    loop {
        interval.tick().await;

        let round = probe_all_targets().await;

        if tx.send(round).await.is_err() {
            // Channel closed, exit loop
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_probe_target_success() {
        // Google DNS should be reachable
        let result = probe_target("8.8.8.8", 443).await;
        assert_eq!(result.target, "8.8.8.8");
        // We can't guarantee success in all environments, but we can check structure
        if result.success {
            assert!(result.latency_ms.is_some());
            assert!(result.latency_ms.unwrap() >= 0.0);
        }
    }

    #[tokio::test]
    async fn test_probe_target_failure() {
        // Invalid IP should fail
        let result = probe_target("192.0.2.1", 443).await;
        assert_eq!(result.target, "192.0.2.1");
        // This should timeout and fail
        assert!(!result.success);
        assert!(result.latency_ms.is_none());
    }

    #[tokio::test]
    async fn test_probe_all_targets() {
        let round = probe_all_targets().await;
        assert_eq!(round.results.len(), TARGETS.len());

        for result in round.results {
            assert!(TARGETS.contains(&result.target.as_str()));
        }
    }
}
