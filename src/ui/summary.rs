use crate::monitor::net_info::NetworkInfo;
use crate::monitor::{ConnectionStatus, NetworkStats};
use crate::ui::tui::TuiState;

/// Format session summary for TUI mode exit
#[allow(dead_code)]
pub fn format_summary(state: &TuiState, short: bool) -> String {
    format_summary_with_report_path(state, short, None)
}

/// Format session summary for TUI mode exit with optional report path
pub fn format_summary_with_report_path(
    state: &TuiState,
    short: bool,
    report_path: Option<&std::path::Path>,
) -> String {
    if short {
        format_short_summary(state, report_path)
    } else {
        format_default_summary(state)
    }
}

/// Format session summary for CLI mode exit
pub fn format_cli_summary(
    stats: &NetworkStats,
    network_info: Option<&NetworkInfo>,
    duration_secs: i64,
    short: bool,
) -> String {
    if short {
        format_short_cli_summary(stats, network_info, duration_secs)
    } else {
        format_default_cli_summary(stats, network_info, duration_secs)
    }
}

/// Get status symbol based on ConnectionStatus
fn get_status_symbol(status: ConnectionStatus) -> &'static str {
    match status {
        ConnectionStatus::Ok => "✓",
        ConnectionStatus::Slow => "⚠",
        ConnectionStatus::Disconnected => "✗",
    }
}

/// Format duration in human readable form
fn format_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;

    if hours > 0 {
        format!("{}h {}m {}s", hours, minutes, secs)
    } else if minutes > 0 {
        format!("{}m {}s", minutes, secs)
    } else {
        format!("{}s", secs)
    }
}

/// Format network info line (IP • ISP • Location)
fn format_network_line(network_info: Option<&NetworkInfo>) -> String {
    if let Some(info) = network_info {
        let mut parts = Vec::new();

        // Prefer IPv4, fallback to IPv6
        if let Some(ip) = info.public_ipv4.as_ref().or(info.public_ipv6.as_ref()) {
            parts.push(ip.clone());
        }

        // ISP with ASN
        match (&info.isp, &info.asn) {
            (Some(isp), Some(asn)) => parts.push(format!("{} ({})", isp, asn)),
            (Some(isp), None) => parts.push(isp.clone()),
            (None, Some(asn)) => parts.push(asn.clone()),
            (None, None) => {}
        }

        // Location (City, Country)
        match (&info.city, &info.country) {
            (Some(city), Some(country)) => parts.push(format!("{}, {}", city, country)),
            (Some(city), None) => parts.push(city.clone()),
            (None, Some(country)) => parts.push(country.clone()),
            (None, None) => {}
        }

        // Signal Strength
        if let Some(signal) = info.signal_strength {
            parts.push(format!("Signal: {} dBm", signal));
        }

        if parts.is_empty() {
            "N/A".to_string()
        } else {
            parts.join(" • ")
        }
    } else {
        "N/A".to_string()
    }
}

/// Format default (multi-line) summary for TUI mode
fn format_default_summary(state: &TuiState) -> String {
    // Check if we have any data
    if state.stats.is_none() && state.history.is_empty() {
        return "Session Summary:\n  No data collected".to_string();
    }

    let stats = state.stats.as_ref();
    let status_symbol = if let Some(s) = stats {
        get_status_symbol(s.status)
    } else {
        "✗"
    };

    // Calculate session-wide uptime using get_window_stats with full session duration
    let session_duration = chrono::Duration::seconds(state.running_time_seconds());
    let (_avg_latency, uptime_pct) = state.get_window_stats(session_duration);

    // Count disconnections and calculate total downtime
    let disconnection_count = state.disconnections.len();
    let total_downtime_secs: i64 = state
        .disconnections
        .iter()
        .map(|d| d.duration_seconds())
        .sum();

    // Format availability line
    let running_time = state.format_running_time();
    let availability_line = if disconnection_count == 0 {
        format!(
            "  {} {:.1}% uptime (no outages) | {}",
            status_symbol, uptime_pct, running_time
        )
    } else {
        format!(
            "  {} {:.1}% uptime ({} {}, {} total downtime) | {}",
            status_symbol,
            uptime_pct,
            disconnection_count,
            if disconnection_count == 1 {
                "outage"
            } else {
                "outages"
            },
            format_duration(total_downtime_secs),
            running_time
        )
    };

    // Format performance line
    let latency_ms = stats.map(|s| s.avg_latency_ms).unwrap_or(0.0);
    let performance_line = format!("  {} {:.0}ms avg latency", status_symbol, latency_ms);

    // Format network line
    let network_line = format!(
        "  Network: {}",
        format_network_line(state.network_info.as_ref())
    );

    format!(
        "Session Summary:\n{}\n{}\n{}",
        availability_line, performance_line, network_line
    )
}

/// Format short (one-line) summary for TUI mode
fn format_short_summary(state: &TuiState, report_path: Option<&std::path::Path>) -> String {
    // Check if we have any data
    if state.stats.is_none() && state.history.is_empty() {
        return "No data collected".to_string();
    }

    let stats = state.stats.as_ref();
    let status_symbol = if let Some(s) = stats {
        get_status_symbol(s.status)
    } else {
        "✗"
    };

    // Calculate session-wide uptime
    let session_duration = chrono::Duration::seconds(state.running_time_seconds());
    let (_avg_latency, uptime_pct) = state.get_window_stats(session_duration);

    // Get latency
    let latency_ms = stats.map(|s| s.avg_latency_ms).unwrap_or(0.0);

    // Format network info (simplified: IP • ISP)
    let network_short = if let Some(info) = &state.network_info {
        let mut parts = Vec::new();

        // Prefer IPv4
        if let Some(ip) = info.public_ipv4.as_ref().or(info.public_ipv6.as_ref()) {
            parts.push(ip.clone());
        }

        // ISP only (no ASN in short format)
        if let Some(isp) = &info.isp {
            parts.push(isp.clone());
        }

        if parts.is_empty() {
            "N/A".to_string()
        } else {
            parts.join(" • ")
        }
    } else {
        "N/A".to_string()
    };

    let running_time = state.format_running_time();

    let mut output = format!(
        "{} {:.1}% uptime | {:.0}ms | {} | (test duration: {})",
        status_symbol, uptime_pct, latency_ms, network_short, running_time
    );

    // Append report path if provided
    if let Some(path) = report_path {
        output.push_str(&format!(" | Report: {}", path.display()));
    }

    output
}

/// Format default (multi-line) summary for CLI mode
fn format_default_cli_summary(
    stats: &NetworkStats,
    network_info: Option<&NetworkInfo>,
    duration_secs: i64,
) -> String {
    let status_symbol = get_status_symbol(stats.status);

    // CLI mode: simple calculation (no historical data, assume 100% uptime for successful check)
    let uptime_pct = if stats.status == ConnectionStatus::Disconnected {
        0.0
    } else {
        100.0
    };

    // Format availability line (CLI mode doesn't track outages)
    let availability_line = format!(
        "  {} {:.1}% uptime (no outages) | {}",
        status_symbol,
        uptime_pct,
        format_duration(duration_secs)
    );

    // Format performance line
    let performance_line = format!(
        "  {} {:.0}ms avg latency",
        status_symbol, stats.avg_latency_ms
    );

    // Format network line
    let network_line = format!("  Network: {}", format_network_line(network_info));

    format!(
        "Session Summary:\n{}\n{}\n{}",
        availability_line, performance_line, network_line
    )
}

/// Format short (one-line) summary for CLI mode
fn format_short_cli_summary(
    stats: &NetworkStats,
    network_info: Option<&NetworkInfo>,
    duration_secs: i64,
) -> String {
    let status_symbol = get_status_symbol(stats.status);

    // CLI mode: simple calculation
    let uptime_pct = if stats.status == ConnectionStatus::Disconnected {
        0.0
    } else {
        100.0
    };

    // Format network info (simplified: IP • ISP)
    let network_short = if let Some(info) = network_info {
        let mut parts = Vec::new();

        // Prefer IPv4
        if let Some(ip) = info.public_ipv4.as_ref().or(info.public_ipv6.as_ref()) {
            parts.push(ip.clone());
        }

        // ISP only (no ASN in short format)
        if let Some(isp) = &info.isp {
            parts.push(isp.clone());
        }

        if parts.is_empty() {
            "N/A".to_string()
        } else {
            parts.join(" • ")
        }
    } else {
        "N/A".to_string()
    };

    format!(
        "{} {:.1}% uptime | {:.0}ms | {} | (test duration: {})",
        status_symbol,
        uptime_pct,
        stats.avg_latency_ms,
        network_short,
        format_duration(duration_secs)
    )
}
