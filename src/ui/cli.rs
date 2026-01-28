use crate::monitor::{net_info::NetworkInfo, ConnectionStatus, NetworkStats};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "hale")]
#[command(about = "Instant network connection quality monitor")]
#[command(version)]
pub struct Cli {
    /// Run a single check and exit (CLI mode)
    #[arg(short = 'c', long = "check")]
    pub check: bool,

    /// Output format (only applies to CLI mode)
    #[arg(short, long, default_value = "text")]
    pub format: OutputFormat,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Debug, Clone, Default, clap::ValueEnum)]
pub enum OutputFormat {
    /// Plain text output
    #[default]
    Text,
    /// JSON output
    Json,
}

pub fn parse_args() -> Cli {
    Cli::parse()
}

/// Format and display network statistics in CLI mode
pub fn display_cli_stats(
    stats: &NetworkStats,
    format: &OutputFormat,
    verbose: bool,
    network_info: Option<&NetworkInfo>,
) {
    match format {
        OutputFormat::Text => {
            display_text_format(stats, verbose, network_info);
        }
        OutputFormat::Json => {
            display_json_format(stats, network_info);
        }
    }
}

/// Display stats in human-readable text format with ANSI colors
fn display_text_format(stats: &NetworkStats, verbose: bool, network_info: Option<&NetworkInfo>) {
    let (symbol, color_code, status_text) = match stats.status {
        ConnectionStatus::Ok => ("✓", "\x1b[32m", "OK"), // Green
        ConnectionStatus::Slow => ("⚠", "\x1b[33m", "Slow"), // Yellow
        ConnectionStatus::Disconnected => ("✗", "\x1b[31m", "DISCONNECTED"), // Red
    };

    // Main status line with color - only show latency
    let latency_display = if stats.avg_latency_ms > 0.0 {
        format!("{:.0}ms", stats.avg_latency_ms)
    } else {
        "timeout".to_string()
    };

    println!(
        "{}{} {}  {}\x1b[0m",
        color_code, symbol, status_text, latency_display
    );

    // Show reason if not OK
    if stats.status != ConnectionStatus::Ok {
        let reason = match stats.status {
            ConnectionStatus::Slow => format!("  High latency ({:.0}ms)", stats.avg_latency_ms),
            ConnectionStatus::Disconnected => {
                if stats.avg_latency_ms > 0.0 {
                    format!("  Very high latency ({:.0}ms)", stats.avg_latency_ms)
                } else {
                    "  No response from network".to_string()
                }
            }
            _ => String::new(),
        };
        if !reason.is_empty() {
            println!("{}", reason);
        }
    }

    // Verbose output
    if verbose {
        println!(
            "  Timestamp: {}",
            stats.timestamp.format("%Y-%m-%d %H:%M:%S")
        );

        if let Some(info) = network_info {
            if let (Some(city), Some(country)) = (&info.city, &info.country) {
                println!("  Location: {}, {}", city, country);
            }

            let isp_display = match (&info.isp, &info.asn) {
                (Some(isp), Some(asn)) => Some(format!("{} ({})", isp, asn)),
                (Some(isp), None) => Some(isp.clone()),
                (None, Some(asn)) => Some(asn.clone()),
                (None, None) => None,
            };

            if let Some(isp_str) = isp_display {
                println!("  Provider: {}", isp_str);
            }
        }
    }
}

/// Display stats in JSON format
fn display_json_format(stats: &NetworkStats, network_info: Option<&NetworkInfo>) {
    let mut json = format!(
        r#"{{"timestamp":"{}","status":"{}","latency_ms":{:.1}"#,
        stats.timestamp.to_rfc3339(),
        stats.status,
        stats.avg_latency_ms
    );

    if let Some(info) = network_info {
        if let Some(country) = &info.country {
            json.push_str(&format!(r#","country":"{}""#, country));
        }
        if let Some(city) = &info.city {
            json.push_str(&format!(r#","city":"{}""#, city));
        }
        if let Some(isp) = &info.isp {
            json.push_str(&format!(r#","isp":"{}""#, isp));
        }
        if let Some(org) = &info.org {
            json.push_str(&format!(r#","org":"{}""#, org));
        }
        if let Some(asn) = &info.asn {
            json.push_str(&format!(r#","asn":"{}""#, asn));
        }
    }

    json.push('}');
    println!("{}", json);
}

/// Get exit code based on connection status
pub fn get_exit_code(status: ConnectionStatus) -> i32 {
    match status {
        ConnectionStatus::Ok => 0,
        ConnectionStatus::Slow | ConnectionStatus::Disconnected => 1,
    }
}
