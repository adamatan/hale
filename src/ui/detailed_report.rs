use crate::config::{PROBE_INTERVAL_MS, TARGETS, TARGET_LABELS};
use crate::monitor::net_info::NetworkInfo;
use crate::monitor::{ConnectionStatus, ProbeRound};
use crate::ui::tui::TuiState;
use chrono::{DateTime, Local, TimeZone};
use std::collections::VecDeque;
use std::fs;
use std::path::PathBuf;

/// Represents a single incident (status change) during the session
#[derive(Debug, Clone)]
pub struct SessionIncident {
    pub timestamp: DateTime<Local>,
    pub status: ConnectionStatus,
    pub duration_secs: f64,
    pub avg_latency_ms: Option<f64>,
}

/// Session quality score
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionScore {
    Perfect,      // 100% OK, no issues
    OK,           // >99% uptime, brief slowness
    Poor,         // 95-99% uptime or frequent slowness
    Unacceptable, // <95% uptime or extended outages
    NoConnection, // 0% connectivity
}

impl SessionScore {
    /// Get a one-line description of the score
    fn description(&self) -> &str {
        match self {
            SessionScore::Perfect => "100% uptime with no degradation",
            SessionScore::OK => ">99% uptime with minimal issues",
            SessionScore::Poor => "95-99% uptime or frequent degradation",
            SessionScore::Unacceptable => "<95% uptime or extended outages",
            SessionScore::NoConnection => "No successful connections",
        }
    }
}

/// Uptime statistics breakdown
#[derive(Debug, Clone)]
pub struct UptimeStats {
    pub ok_pct: f64,
    pub ok_secs: f64,
    pub slow_pct: f64,
    pub slow_secs: f64,
    pub disconnected_pct: f64,
    pub disconnected_secs: f64,
}

/// Complete detailed session report
#[derive(Debug, Clone)]
pub struct DetailedSessionReport {
    pub score: SessionScore,
    pub session_start: DateTime<Local>,
    pub session_end: DateTime<Local>,
    pub session_duration_secs: i64,
    pub targets_list: String,
    pub network_info: Option<NetworkInfo>,
    pub ip_change_count: usize,
    pub isp_location_change_count: usize,
    pub uptime_stats: UptimeStats,
    pub disconnection_count: usize,
    pub slow_count: usize,
    pub incidents: Vec<SessionIncident>,
}

/// Analyze probe history to identify discrete incidents
fn analyze_history_for_incidents(history: &VecDeque<ProbeRound>) -> Vec<SessionIncident> {
    let mut incidents = Vec::new();

    if history.is_empty() {
        return incidents;
    }

    let mut current_status: Option<ConnectionStatus> = None;
    let mut incident_start: Option<DateTime<Local>> = None;
    let mut incident_rounds = 0;
    let mut incident_latency_sum = 0.0;
    let mut incident_latency_count = 0;

    for round in history.iter() {
        // Determine round status from results
        let round_status = determine_round_status(round);

        // Convert UTC timestamp to Local
        let round_time = Local.from_utc_datetime(&round.timestamp.naive_utc());

        match current_status {
            None => {
                // First round - start tracking
                current_status = Some(round_status);
                incident_start = Some(round_time);
                incident_rounds = 1;

                // Track latency for non-disconnected rounds
                if round_status != ConnectionStatus::Disconnected {
                    let avg_latency = calculate_round_avg_latency(round);
                    if let Some(latency) = avg_latency {
                        incident_latency_sum += latency;
                        incident_latency_count += 1;
                    }
                }
            }
            Some(status) if status == round_status => {
                // Continue current incident
                incident_rounds += 1;

                if round_status != ConnectionStatus::Disconnected {
                    let avg_latency = calculate_round_avg_latency(round);
                    if let Some(latency) = avg_latency {
                        incident_latency_sum += latency;
                        incident_latency_count += 1;
                    }
                }
            }
            Some(status) => {
                // Status changed - record incident and start new one
                if let Some(start) = incident_start {
                    let duration_secs =
                        incident_rounds as f64 * (PROBE_INTERVAL_MS as f64 / 1000.0);
                    let avg_latency = if incident_latency_count > 0 {
                        Some(incident_latency_sum / incident_latency_count as f64)
                    } else {
                        None
                    };

                    // Only record non-OK incidents
                    if status != ConnectionStatus::Ok {
                        incidents.push(SessionIncident {
                            timestamp: start,
                            status,
                            duration_secs,
                            avg_latency_ms: avg_latency,
                        });
                    }
                }

                // Start new incident
                current_status = Some(round_status);
                incident_start = Some(round_time);
                incident_rounds = 1;
                incident_latency_sum = 0.0;
                incident_latency_count = 0;

                if round_status != ConnectionStatus::Disconnected {
                    let avg_latency = calculate_round_avg_latency(round);
                    if let Some(latency) = avg_latency {
                        incident_latency_sum += latency;
                        incident_latency_count += 1;
                    }
                }
            }
        }
    }

    // Record final incident if it's not OK
    if let (Some(status), Some(start)) = (current_status, incident_start) {
        if status != ConnectionStatus::Ok && incident_rounds > 0 {
            let duration_secs = incident_rounds as f64 * (PROBE_INTERVAL_MS as f64 / 1000.0);
            let avg_latency = if incident_latency_count > 0 {
                Some(incident_latency_sum / incident_latency_count as f64)
            } else {
                None
            };

            incidents.push(SessionIncident {
                timestamp: start,
                status,
                duration_secs,
                avg_latency_ms: avg_latency,
            });
        }
    }

    incidents
}

/// Determine the status of a single probe round
fn determine_round_status(round: &ProbeRound) -> ConnectionStatus {
    // Count successful probes
    let successful = round.results.iter().filter(|r| r.success).count();

    if successful == 0 {
        return ConnectionStatus::Disconnected;
    }

    // Calculate average latency of successful probes
    let latencies: Vec<f64> = round.results.iter().filter_map(|r| r.latency_ms).collect();

    if latencies.is_empty() {
        return ConnectionStatus::Disconnected;
    }

    let avg_latency = latencies.iter().sum::<f64>() / latencies.len() as f64;

    // Use same thresholds as main monitoring logic
    if avg_latency < crate::config::LATENCY_OK_THRESHOLD_MS {
        ConnectionStatus::Ok
    } else if avg_latency < crate::config::LATENCY_SLOW_THRESHOLD_MS {
        ConnectionStatus::Slow
    } else {
        ConnectionStatus::Disconnected
    }
}

/// Calculate average latency for a probe round
fn calculate_round_avg_latency(round: &ProbeRound) -> Option<f64> {
    let latencies: Vec<f64> = round.results.iter().filter_map(|r| r.latency_ms).collect();

    if latencies.is_empty() {
        None
    } else {
        Some(latencies.iter().sum::<f64>() / latencies.len() as f64)
    }
}

/// Calculate uptime statistics from probe history
fn calculate_uptime_stats(
    history: &VecDeque<ProbeRound>,
    session_duration_secs: i64,
) -> UptimeStats {
    if history.is_empty() || session_duration_secs == 0 {
        return UptimeStats {
            ok_pct: 0.0,
            ok_secs: 0.0,
            slow_pct: 0.0,
            slow_secs: 0.0,
            disconnected_pct: 0.0,
            disconnected_secs: 0.0,
        };
    }

    let mut ok_rounds = 0;
    let mut slow_rounds = 0;
    let mut disconnected_rounds = 0;

    for round in history.iter() {
        match determine_round_status(round) {
            ConnectionStatus::Ok => ok_rounds += 1,
            ConnectionStatus::Slow => slow_rounds += 1,
            ConnectionStatus::Disconnected => disconnected_rounds += 1,
        }
    }

    let seconds_per_round = PROBE_INTERVAL_MS as f64 / 1000.0;

    let ok_secs = ok_rounds as f64 * seconds_per_round;
    let slow_secs = slow_rounds as f64 * seconds_per_round;
    let disconnected_secs = disconnected_rounds as f64 * seconds_per_round;

    // Calculate total time based on actual probe rounds, not session duration
    let total_secs = ok_secs + slow_secs + disconnected_secs;

    // Avoid division by zero
    let (ok_pct, slow_pct, disconnected_pct) = if total_secs > 0.0 {
        (
            (ok_secs / total_secs) * 100.0,
            (slow_secs / total_secs) * 100.0,
            (disconnected_secs / total_secs) * 100.0,
        )
    } else {
        (0.0, 0.0, 0.0)
    };

    UptimeStats {
        ok_pct,
        ok_secs,
        slow_pct,
        slow_secs,
        disconnected_pct,
        disconnected_secs,
    }
}

/// Determine session score based on uptime and incidents
fn determine_score(uptime_stats: &UptimeStats, incidents: &[SessionIncident]) -> SessionScore {
    // NoConnection: 0% OK time
    if uptime_stats.ok_pct == 0.0 && uptime_stats.slow_pct == 0.0 {
        return SessionScore::NoConnection;
    }

    // Perfect: 100% OK
    if uptime_stats.ok_pct >= 99.99 && incidents.is_empty() {
        return SessionScore::Perfect;
    }

    // Count significant incidents (disconnections or slow periods >5s)
    let disconnection_count = incidents
        .iter()
        .filter(|i| i.status == ConnectionStatus::Disconnected)
        .count();
    let significant_slow_count = incidents
        .iter()
        .filter(|i| i.status == ConnectionStatus::Slow && i.duration_secs > 5.0)
        .count();

    // Check for extended outages (>5 minutes)
    let has_extended_outage = incidents.iter().any(|i| i.duration_secs > 300.0);

    let total_uptime_pct = uptime_stats.ok_pct + uptime_stats.slow_pct;

    // Unacceptable: <95% uptime OR extended outage OR multiple disconnections
    if total_uptime_pct < 95.0 || has_extended_outage || disconnection_count > 3 {
        return SessionScore::Unacceptable;
    }

    // Poor: 95-99% uptime OR frequent significant issues
    if total_uptime_pct < 99.0 || significant_slow_count > 5 || disconnection_count > 0 {
        return SessionScore::Poor;
    }

    // OK: Everything else (>99% uptime, no disconnections, minimal slow periods)
    SessionScore::OK
}

/// Calculate complete report from TUI state
fn calculate_report(state: &TuiState) -> DetailedSessionReport {
    let session_end = chrono::Utc::now();
    let session_duration_secs = session_end
        .signed_duration_since(state.session_start)
        .num_seconds();

    // Build targets list
    let targets_list = TARGETS
        .iter()
        .zip(TARGET_LABELS.iter())
        .map(|(target, label)| format!("{} ({})", label, target))
        .collect::<Vec<_>>()
        .join(", ");

    let uptime_stats = calculate_uptime_stats(&state.history, session_duration_secs);
    let incidents = analyze_history_for_incidents(&state.history);

    // Count disconnections vs slow periods
    let disconnection_count = incidents
        .iter()
        .filter(|i| i.status == ConnectionStatus::Disconnected)
        .count();
    let slow_count = incidents
        .iter()
        .filter(|i| i.status == ConnectionStatus::Slow)
        .count();

    let score = determine_score(&uptime_stats, &incidents);

    DetailedSessionReport {
        score,
        session_start: Local.from_utc_datetime(&state.session_start.naive_utc()),
        session_end: Local.from_utc_datetime(&session_end.naive_utc()),
        session_duration_secs,
        targets_list,
        network_info: state.network_info.clone(),
        ip_change_count: state.ip_change_count,
        isp_location_change_count: state.isp_location_change_count,
        uptime_stats,
        disconnection_count,
        slow_count,
        incidents,
    }
}

/// Format duration in seconds as human-readable string
fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{}s", secs.round() as i64)
    } else {
        let minutes = (secs / 60.0).floor() as i64;
        let remaining_secs = (secs % 60.0).round() as i64;
        if remaining_secs == 0 {
            format!("{}m", minutes)
        } else {
            format!("{}m {}s", minutes, remaining_secs)
        }
    }
}

/// Get local timezone abbreviation
fn get_timezone_abbrev() -> String {
    // Try to get timezone from current time
    let now = Local::now();
    let tz_str = format!("{}", now.format("%Z"));

    if tz_str.is_empty() {
        "Local".to_string()
    } else {
        tz_str
    }
}

/// Format network information section
fn format_network_info(
    network_info: &Option<NetworkInfo>,
    ip_change_count: usize,
    isp_location_change_count: usize,
) -> Vec<String> {
    let mut lines = Vec::new();

    if let Some(info) = network_info {
        // If there were changes, add those lines first
        if ip_change_count > 0 {
            lines.push(format!(
                "  IP changed {} time{} during session",
                ip_change_count,
                if ip_change_count == 1 { "" } else { "s" }
            ));
        }
        if isp_location_change_count > 0 {
            lines.push(format!(
                "  ISP/Location changed {} time{} during session",
                isp_location_change_count,
                if isp_location_change_count == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }

        // Show current info (with "Current" prefix if there were changes)
        let prefix = if ip_change_count > 0 || isp_location_change_count > 0 {
            "Current "
        } else {
            ""
        };

        // IP (prefer IPv4, fallback to IPv6)
        if let Some(ip) = info.public_ipv4.as_ref().or(info.public_ipv6.as_ref()) {
            lines.push(format!("  {}IP:       {}", prefix, ip));
        }

        // ISP with ASN
        if let Some(isp) = &info.isp {
            let isp_line = if let Some(asn) = &info.asn {
                format!("  {}ISP:      {} ({})", prefix, isp, asn)
            } else {
                format!("  {}ISP:      {}", prefix, isp)
            };
            lines.push(isp_line);
        }

        // Location (City, Country)
        match (&info.city, &info.country) {
            (Some(city), Some(country)) => {
                lines.push(format!("  {}Location: {}, {}", prefix, city, country));
            }
            (None, Some(country)) => {
                lines.push(format!("  {}Location: {}", prefix, country));
            }
            (Some(city), None) => {
                lines.push(format!("  {}Location: {}", prefix, city));
            }
            (None, None) => {}
        }
    } else {
        lines.push("  Network information unavailable".to_string());
    }

    lines
}

/// Get emoji indicator for session score
fn get_score_emoji(score: &SessionScore) -> &str {
    match score {
        SessionScore::Perfect => "✅",
        SessionScore::OK => "✅",
        SessionScore::Poor => "⚠️",
        SessionScore::Unacceptable => "❌",
        SessionScore::NoConnection => "❌",
    }
}

/// Format the detailed session report for console output (clean, readable)
fn format_detailed_report_console(report: &DetailedSessionReport) -> String {
    let mut output = String::new();
    let tz = get_timezone_abbrev();

    // Header
    output.push_str("Hale Session Summary\n");

    // Session Score
    let emoji = get_score_emoji(&report.score);
    output.push_str(&format!("{} Session Score: {:?}\n", emoji, report.score));
    output.push_str(&format!("  {}\n", report.score.description()));

    // Session Metadata
    output.push('\n');
    output.push_str("Session Metadata\n");
    output.push_str(&format!(
        "  Start:    {} ({})\n",
        report.session_start.format("%Y-%m-%d %H:%M:%S"),
        tz
    ));
    output.push_str(&format!(
        "  End:      {} ({})\n",
        report.session_end.format("%Y-%m-%d %H:%M:%S"),
        tz
    ));
    output.push_str(&format!(
        "  Duration: {}\n",
        format_duration(report.session_duration_secs as f64)
    ));
    output.push_str(&format!("  Targets:  {}\n", report.targets_list));

    // Network Information
    output.push('\n');
    output.push_str("Network Information\n");
    let network_lines = format_network_info(
        &report.network_info,
        report.ip_change_count,
        report.isp_location_change_count,
    );
    for line in network_lines {
        output.push_str(&line);
        output.push('\n');
    }

    // Uptime Statistics
    output.push('\n');
    output.push_str("Uptime Statistics\n");
    output.push_str(&format!(
        "  OK:           {:6.2}%  ({})\n",
        report.uptime_stats.ok_pct,
        format_duration(report.uptime_stats.ok_secs)
    ));
    output.push_str(&format!(
        "  Slow:         {:6.2}%  ({})\n",
        report.uptime_stats.slow_pct,
        format_duration(report.uptime_stats.slow_secs)
    ));
    output.push_str(&format!(
        "  Disconnected: {:6.2}%  ({})\n",
        report.uptime_stats.disconnected_pct,
        format_duration(report.uptime_stats.disconnected_secs)
    ));

    // Incident Counts
    output.push('\n');
    output.push_str("Incident Summary\n");
    output.push_str(&format!(
        "  Disconnections: {}\n",
        report.disconnection_count
    ));
    output.push_str(&format!("  Slow Periods:   {}\n", report.slow_count));

    // Detailed Issues Table
    if !report.incidents.is_empty() {
        output.push('\n');
        output.push_str("Detailed Issues\n");
        output.push_str(&format!(
            "  {:20} {:13} {:12} {:12}\n",
            "Timestamp", "Status", "Duration", "Avg Latency"
        ));

        for incident in &report.incidents {
            let timestamp_str = format!("{}", incident.timestamp.format("%Y-%m-%d %H:%M:%S"));
            let status_str = format!("{:?}", incident.status);
            let duration_str = format_duration(incident.duration_secs);
            let latency_str = if let Some(lat) = incident.avg_latency_ms {
                format!("{:.1} ms", lat)
            } else {
                "N/A".to_string()
            };

            output.push_str(&format!(
                "  {:20} {:13} {:12} {:12}\n",
                timestamp_str, status_str, duration_str, latency_str
            ));
        }
    }

    // Score Legend
    output.push('\n');
    output.push_str("Score Legend\n");
    output.push_str("  Perfect:      100% uptime with no degradation\n");
    output.push_str("  OK:           >99% uptime with minimal issues\n");
    output.push_str("  Poor:         95-99% uptime or frequent degradation\n");
    output.push_str("  Unacceptable: <95% uptime or extended outages\n");
    output.push_str("  NoConnection: No successful connections\n");

    output
}

/// Format the detailed session report in Markdown for file output
fn format_detailed_report_markdown(report: &DetailedSessionReport) -> String {
    let mut output = String::new();
    let tz = get_timezone_abbrev();

    // Header
    output.push_str("# HALE SESSION SUMMARY REPORT\n\n");

    // Session Score
    output.push_str(&format!("## Session Score: **{:?}**\n\n", report.score));
    output.push_str(&format!("_{}_\n\n", report.score.description()));

    // Session Metadata
    output.push_str("## Session Metadata\n\n");
    output.push_str(&format!(
        "- **Start:** {} ({})\n",
        report.session_start.format("%Y-%m-%d %H:%M:%S"),
        tz
    ));
    output.push_str(&format!(
        "- **End:** {} ({})\n",
        report.session_end.format("%Y-%m-%d %H:%M:%S"),
        tz
    ));
    output.push_str(&format!(
        "- **Duration:** {}\n",
        format_duration(report.session_duration_secs as f64)
    ));
    output.push_str(&format!("- **Targets:** {}\n\n", report.targets_list));

    // Network Information
    output.push_str("## Network Information\n\n");
    if let Some(info) = &report.network_info {
        // If there were changes, add those lines first
        if report.ip_change_count > 0 {
            output.push_str(&format!(
                "- **IP changed {} time{} during session**\n",
                report.ip_change_count,
                if report.ip_change_count == 1 { "" } else { "s" }
            ));
        }
        if report.isp_location_change_count > 0 {
            output.push_str(&format!(
                "- **ISP/Location changed {} time{} during session**\n",
                report.isp_location_change_count,
                if report.isp_location_change_count == 1 {
                    ""
                } else {
                    "s"
                }
            ));
        }

        // Show current info (with "Current" prefix if there were changes)
        let prefix = if report.ip_change_count > 0 || report.isp_location_change_count > 0 {
            "Current "
        } else {
            ""
        };

        // IP (prefer IPv4, fallback to IPv6)
        if let Some(ip) = info.public_ipv4.as_ref().or(info.public_ipv6.as_ref()) {
            output.push_str(&format!("- **{}IP:** {}\n", prefix, ip));
        }

        // ISP with ASN
        if let Some(isp) = &info.isp {
            let isp_line = if let Some(asn) = &info.asn {
                format!("- **{}ISP:** {} ({})\n", prefix, isp, asn)
            } else {
                format!("- **{}ISP:** {}\n", prefix, isp)
            };
            output.push_str(&isp_line);
        }

        // Location (City, Country)
        match (&info.city, &info.country) {
            (Some(city), Some(country)) => {
                output.push_str(&format!(
                    "- **{}Location:** {}, {}\n",
                    prefix, city, country
                ));
            }
            (None, Some(country)) => {
                output.push_str(&format!("- **{}Location:** {}\n", prefix, country));
            }
            (Some(city), None) => {
                output.push_str(&format!("- **{}Location:** {}\n", prefix, city));
            }
            (None, None) => {}
        }
    } else {
        output.push_str("_Network information unavailable_\n");
    }
    output.push('\n');

    // Uptime Statistics
    output.push_str("## Uptime Statistics\n\n");
    output.push_str(&format!(
        "- **OK:** {:.2}% ({})\n",
        report.uptime_stats.ok_pct,
        format_duration(report.uptime_stats.ok_secs)
    ));
    output.push_str(&format!(
        "- **Slow:** {:.2}% ({})\n",
        report.uptime_stats.slow_pct,
        format_duration(report.uptime_stats.slow_secs)
    ));
    output.push_str(&format!(
        "- **Disconnected:** {:.2}% ({})\n\n",
        report.uptime_stats.disconnected_pct,
        format_duration(report.uptime_stats.disconnected_secs)
    ));

    // Incident Counts
    output.push_str("## Incident Summary\n\n");
    output.push_str(&format!(
        "- **Disconnections:** {}\n",
        report.disconnection_count
    ));
    output.push_str(&format!("- **Slow Periods:** {}\n\n", report.slow_count));

    // Detailed Issues Table
    output.push_str("## Detailed Issues\n\n");
    if report.incidents.is_empty() {
        output.push_str("_No issues detected during this session._\n\n");
    } else {
        output.push_str("| Timestamp | Status | Duration | Avg Latency |\n");
        output.push_str("|-----------|--------|----------|-------------|\n");

        for incident in &report.incidents {
            let timestamp_str = incident.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
            let status_str = format!("{:?}", incident.status).to_uppercase();
            let duration_str = format_duration(incident.duration_secs);
            let latency_str = if let Some(lat) = incident.avg_latency_ms {
                format!("{:.1} ms", lat)
            } else {
                "N/A".to_string()
            };

            output.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                timestamp_str, status_str, duration_str, latency_str
            ));
        }
        output.push('\n');
    }

    // Score Legend
    output.push_str("## Score Legend\n\n");
    output.push_str("- **Perfect:** 100% uptime with no degradation\n");
    output.push_str("- **OK:** >99% uptime with minimal issues\n");
    output.push_str("- **Poor:** 95-99% uptime or frequent degradation\n");
    output.push_str("- **Unacceptable:** <95% uptime or extended outages\n");
    output.push_str("- **NoConnection:** No successful connections\n\n");

    // Footer
    output.push_str("---\n\n");
    output.push_str("_Created by **Hale** - your internet connection checker_\n\n");
    output.push_str(
        "**DISCLAIMER:** This software is provided \"AS IS\" without warranty of any kind. ",
    );
    output.push_str(
        "Network quality metrics are estimates and may not reflect actual conditions.\n\n",
    );
    output.push_str("**License:** MIT License - Copyright (c) 2026 Adam Matan  \n");
    output.push_str("**Repository:** https://github.com/adamatan/hale\n");

    output
}

/// Generate detailed report string for console output
pub fn generate_detailed_report(state: &TuiState) -> Result<String, Box<dyn std::error::Error>> {
    let report = calculate_report(state);
    Ok(format_detailed_report_console(&report))
}

/// Write detailed report to file in Markdown format and return the path
pub fn write_detailed_report(
    state: &TuiState,
    session_id: &str,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let report = calculate_report(state);
    let formatted = format_detailed_report_markdown(&report);

    // Generate filename with session_id
    let filename = format!("/tmp/hale-{}-summary.md", session_id);
    let path = PathBuf::from(&filename);

    // Write to file
    fs::write(&path, formatted)?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::tui::TuiState;

    #[test]
    fn test_write_detailed_report() {
        let state = TuiState::new();
        let session_id = "TEST-SESSION-1234";
        let result = write_detailed_report(&state, session_id);
        assert!(result.is_ok());
        let path = result.unwrap();
        assert!(path.exists());
        assert!(path
            .to_str()
            .unwrap()
            .contains("hale-TEST-SESSION-1234-summary.md"));
        let _ = std::fs::remove_file(path);
    }
}
