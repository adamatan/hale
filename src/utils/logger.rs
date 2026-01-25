use crate::monitor::{ConnectionStatus, NetworkStats};
use chrono::{DateTime, Utc};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

/// Session logger that tracks status changes
pub struct SessionLogger {
    log_file: File,
    log_path: PathBuf,
    last_status: Option<ConnectionStatus>,
    last_status_change: Option<DateTime<Utc>>,
}

impl SessionLogger {
    /// Create a new session logger
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let timestamp = Utc::now().format("%Y%m%d-%H%M%S");
        let log_path = PathBuf::from(format!("/tmp/hale-{}.log", timestamp));

        let log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        let mut logger = Self {
            log_file,
            log_path,
            last_status: None,
            last_status_change: None,
        };

        // Write header
        writeln!(logger.log_file, "hale Session Log")?;
        writeln!(
            logger.log_file,
            "Started: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        )?;
        writeln!(logger.log_file, "{}", "=".repeat(80))?;
        logger.log_file.flush()?;

        Ok(logger)
    }

    /// Log network statistics (only on status change)
    pub fn log_stats(&mut self, stats: &NetworkStats) -> Result<(), Box<dyn std::error::Error>> {
        let current_status = stats.status;

        // Check if status changed
        if let Some(last_status) = self.last_status {
            if last_status != current_status {
                // Status changed - log it
                let downtime_msg = if let Some(last_change) = self.last_status_change {
                    // Calculate downtime if we were in a non-OK state
                    if last_status != ConnectionStatus::Ok {
                        let duration = stats.timestamp.signed_duration_since(last_change);
                        let minutes = duration.num_minutes();
                        let seconds = duration.num_seconds() % 60;
                        format!(" | Downtime: {}m {}s", minutes, seconds)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                writeln!(
                    self.log_file,
                    "{} | STATUS_CHANGE | {} -> {} | latency={:.0}ms, loss={:.1}%, jitter={:.0}ms{}",
                    stats.timestamp.format("%Y-%m-%d %H:%M:%S"),
                    last_status,
                    current_status,
                    stats.avg_latency_ms,
                    stats.loss_pct,
                    stats.jitter_ms,
                    downtime_msg
                )?;
                self.log_file.flush()?;

                self.last_status_change = Some(stats.timestamp);
            }
        } else {
            // First status
            writeln!(
                self.log_file,
                "{} | INITIAL_STATUS | {} | latency={:.0}ms, loss={:.1}%, jitter={:.0}ms",
                stats.timestamp.format("%Y-%m-%d %H:%M:%S"),
                current_status,
                stats.avg_latency_ms,
                stats.loss_pct,
                stats.jitter_ms
            )?;
            self.log_file.flush()?;

            self.last_status_change = Some(stats.timestamp);
        }

        self.last_status = Some(current_status);
        Ok(())
    }

    /// Get the log file path
    pub fn log_path(&self) -> &PathBuf {
        &self.log_path
    }

    /// Write session end marker
    pub fn close(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        writeln!(self.log_file, "{}", "=".repeat(80))?;
        writeln!(
            self.log_file,
            "Session ended: {}",
            Utc::now().format("%Y-%m-%d %H:%M:%S")
        )?;
        self.log_file.flush()?;
        Ok(())
    }
}

impl Drop for SessionLogger {
    fn drop(&mut self) {
        let _ = self.close();
    }
}
