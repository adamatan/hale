use crate::config::{HISTORY_WINDOW_SIZE, PROBE_INTERVAL_MS, TARGETS, TARGET_LABELS};
use crate::monitor::net_info::NetworkInfo;
use crate::monitor::{ConnectionStatus, NetworkStats, ProbeRound};
use chrono::{DateTime, Utc};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};
use std::collections::VecDeque;
use std::io;

/// Initialize the terminal for TUI mode
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>, Box<dyn std::error::Error>>
{
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to its original state
pub fn restore_terminal(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
) -> Result<(), Box<dyn std::error::Error>> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Set up panic hook to ensure terminal is always restored
pub fn setup_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

/// Record of a disconnection event
#[derive(Debug, Clone)]
pub struct DisconnectionEvent {
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
}

impl DisconnectionEvent {
    pub fn duration_seconds(&self) -> i64 {
        if let Some(end) = self.end_time {
            end.signed_duration_since(self.start_time).num_seconds()
        } else {
            Utc::now()
                .signed_duration_since(self.start_time)
                .num_seconds()
        }
    }
}

/// State of the TUI application
pub struct TuiState {
    pub stats: Option<NetworkStats>,
    pub should_quit: bool,
    pub history: VecDeque<ProbeRound>,
    pub session_start: DateTime<Utc>,
    pub disconnections: Vec<DisconnectionEvent>,
    pub last_status: Option<ConnectionStatus>,
    pub network_info: Option<NetworkInfo>,
    pub prev_network_info: Option<NetworkInfo>,
    pub ip_change_count: usize,
    pub isp_location_change_count: usize,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            stats: None,
            should_quit: false,
            history: VecDeque::with_capacity(HISTORY_WINDOW_SIZE),
            session_start: Utc::now(),
            disconnections: Vec::new(),
            last_status: None,
            network_info: None,
            prev_network_info: None,
            ip_change_count: 0,
            isp_location_change_count: 0,
        }
    }

    /// Update network info and track changes
    pub fn update_network_info(&mut self, new_info: NetworkInfo) {
        if let Some(prev) = &self.network_info {
            // Check if IP changed (IPv4 or IPv6)
            let ip_changed = prev.public_ipv4 != new_info.public_ipv4
                || prev.public_ipv6 != new_info.public_ipv6;

            // Check if ISP or location changed
            let isp_location_changed = prev.isp != new_info.isp
                || prev.asn != new_info.asn
                || prev.city != new_info.city
                || prev.country != new_info.country;

            if ip_changed {
                self.ip_change_count += 1;
            }

            if isp_location_changed {
                self.isp_location_change_count += 1;
            }

            self.prev_network_info = self.network_info.clone();
        }

        self.network_info = Some(new_info);
    }

    pub fn time_since_last_incident(&self) -> String {
        let now = Utc::now();
        let last_incident_end = self
            .disconnections
            .iter()
            .filter_map(|d| d.end_time)
            .next_back();

        let start = last_incident_end.unwrap_or(self.session_start);
        let duration = now.signed_duration_since(start);
        let seconds = duration.num_seconds();

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

    pub fn last_outage_duration(&self) -> String {
        let last_completed = self.disconnections.iter().rfind(|d| d.end_time.is_some());

        if let Some(event) = last_completed {
            let seconds = event.duration_seconds();
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
        } else {
            "None".to_string()
        }
    }

    pub fn get_window_stats(&self, duration: chrono::Duration) -> (Option<f64>, f64) {
        let now = Utc::now();
        let cutoff = now - duration;

        let rounds_in_window: Vec<&ProbeRound> = self
            .history
            .iter()
            .rev()
            .take_while(|round| round.timestamp >= cutoff)
            .collect();

        if rounds_in_window.is_empty() {
            return (None, 100.0);
        }

        let total_rounds = rounds_in_window.len();
        let mut connected_count = 0;
        let mut latency_sum = 0.0;
        let mut latency_count = 0;

        for round in &rounds_in_window {
            let failed_count = round.results.iter().filter(|r| !r.success).count();
            let avg_latency: f64 = if round.results.iter().any(|r| r.success) {
                round
                    .results
                    .iter()
                    .filter_map(|r| r.latency_ms)
                    .sum::<f64>()
                    / round.results.iter().filter(|r| r.success).count() as f64
            } else {
                f64::MAX
            };

            let is_disconnected = failed_count >= 2 || avg_latency > 300.0;

            if !is_disconnected {
                connected_count += 1;
                latency_sum += avg_latency;
                latency_count += 1;
            }
        }

        let avg_latency = if latency_count > 0 {
            Some(latency_sum / latency_count as f64)
        } else {
            None
        };

        let uptime_pct = if total_rounds > 0 {
            (connected_count as f64 / total_rounds as f64) * 100.0
        } else {
            100.0
        };

        (avg_latency, uptime_pct)
    }

    /// Update state with new stats and track disconnections
    pub fn update_stats(&mut self, stats: NetworkStats, latest_round: ProbeRound) {
        let current_status = stats.status;

        // Track disconnection events
        match (self.last_status, current_status) {
            // Started a disconnection
            (
                Some(ConnectionStatus::Ok | ConnectionStatus::Slow),
                ConnectionStatus::Disconnected,
            )
            | (None, ConnectionStatus::Disconnected) => {
                self.disconnections.push(DisconnectionEvent {
                    start_time: Utc::now(),
                    end_time: None,
                });
            }
            // Ended a disconnection
            (
                Some(ConnectionStatus::Disconnected),
                ConnectionStatus::Ok | ConnectionStatus::Slow,
            ) => {
                if let Some(last_event) = self.disconnections.last_mut() {
                    if last_event.end_time.is_none() {
                        last_event.end_time = Some(Utc::now());
                    }
                }
            }
            _ => {}
        }

        self.history.push_back(latest_round);
        if self.history.len() > HISTORY_WINDOW_SIZE {
            self.history.pop_front();
        }

        self.last_status = Some(current_status);
        self.stats = Some(stats);
    }

    /// Get running time in seconds
    pub fn running_time_seconds(&self) -> i64 {
        Utc::now()
            .signed_duration_since(self.session_start)
            .num_seconds()
    }

    /// Format running time as human readable
    pub fn format_running_time(&self) -> String {
        let seconds = self.running_time_seconds();
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
}

/// Render the TUI interface
pub fn ui(f: &mut Frame, state: &TuiState) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // Improved Status banner
            Constraint::Length(6), // Network Info Block
            Constraint::Min(16),   // Main content (will expand by 5 lines)
            Constraint::Length(1), // Quit instruction footer
        ])
        .split(f.size());

    render_status_banner(f, main_chunks[0], state);
    render_network_info(f, main_chunks[1], state);
    render_main_content(f, main_chunks[2], state);
    render_long_term_status(f, main_chunks[3], state);
}

fn render_status_banner(f: &mut Frame, area: Rect, state: &TuiState) {
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(14), // Status block
            Constraint::Min(40),    // Stats Table
            Constraint::Length(30), // Times
        ])
        .split(area);

    // --- Column 1: Status Block ---
    let (status_str, bg_color, fg_color) = if let Some(stats) = &state.stats {
        match stats.status {
            ConnectionStatus::Ok => ("OK", Color::Green, Color::Black),
            ConnectionStatus::Slow => ("SLOW", Color::Yellow, Color::Black),
            ConnectionStatus::Disconnected => ("OFFLINE", Color::Red, Color::White),
        }
    } else {
        ("INIT", Color::DarkGray, Color::White)
    };

    let status_block = Block::default().borders(Borders::ALL);
    let status_inner = status_block.inner(columns[0]);
    f.render_widget(status_block, columns[0]);

    // Calculate vertical centering
    let status_height = status_inner.height;
    let vertical_padding = status_height.saturating_sub(1) / 2;

    // Create vertically centered content with newlines
    let mut status_lines = vec![];
    for _ in 0..vertical_padding {
        status_lines.push(Line::from(""));
    }
    status_lines.push(Line::from(Span::styled(
        status_str,
        Style::default().fg(fg_color).add_modifier(Modifier::BOLD),
    )));

    let status_para = Paragraph::new(status_lines)
        .style(Style::default().bg(bg_color).fg(fg_color)) // Ensure paragraph matches
        .alignment(Alignment::Center);
    f.render_widget(status_para, status_inner);

    // --- Column 2: Stats Table ---
    let stats_block = Block::default()
        .borders(Borders::ALL)
        .title("Network Stats");
    let stats_inner = stats_block.inner(columns[1]);
    f.render_widget(stats_block, columns[1]);

    // Get current latency
    let current_lat = if let Some(stats) = &state.stats {
        format!("{:.0}ms", stats.avg_latency_ms)
    } else {
        "--".to_string()
    };

    // Get window stats
    let (avg_1m, up_1m) = state.get_window_stats(chrono::Duration::minutes(1));
    let (avg_5m, up_5m) = state.get_window_stats(chrono::Duration::minutes(5));
    let (avg_15m, up_15m) = state.get_window_stats(chrono::Duration::minutes(15));

    let fmt_lat = |val: Option<f64>| {
        val.map(|v| format!("{:.0}ms", v))
            .unwrap_or_else(|| "N/A".to_string())
    };

    let fmt_up = |val: f64, width: usize| {
        let color = if val > 99.0 {
            Color::Green
        } else if val > 95.0 {
            Color::Yellow
        } else {
            Color::Red
        };
        let s = format!("{:.1}%", val);
        Span::styled(
            format!("{:>width$}", s, width = width),
            Style::default().fg(color),
        )
    };

    // Column widths for the table (right-aligned)
    let col_width = 9;
    let label_width = 10;

    // Header row (right-aligned)
    let header = Line::from(vec![
        Span::styled(
            format!("{:>label_width$}", "", label_width = label_width),
            Style::default(),
        ),
        Span::styled(
            format!("{:>col_width$}", "Now", col_width = col_width),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>col_width$}", "1m", col_width = col_width),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>col_width$}", "5m", col_width = col_width),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("{:>col_width$}", "15m", col_width = col_width),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    // Latency row (right-aligned)
    let latency_row = Line::from(vec![
        Span::styled(
            format!("{:>label_width$}", "Latency", label_width = label_width),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{:>col_width$}", current_lat, col_width = col_width),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{:>col_width$}", fmt_lat(avg_1m), col_width = col_width),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{:>col_width$}", fmt_lat(avg_5m), col_width = col_width),
            Style::default().fg(Color::Cyan),
        ),
        Span::styled(
            format!("{:>col_width$}", fmt_lat(avg_15m), col_width = col_width),
            Style::default().fg(Color::Cyan),
        ),
    ]);

    // Uptime row (right-aligned with colors)
    let up_now_color = if let Some(stats) = &state.stats {
        match stats.status {
            ConnectionStatus::Ok => Color::Green,
            ConnectionStatus::Slow => Color::Yellow,
            ConnectionStatus::Disconnected => Color::Red,
        }
    } else {
        Color::DarkGray
    };

    let up_now_str = if let Some(stats) = &state.stats {
        match stats.status {
            ConnectionStatus::Ok => "100%",
            ConnectionStatus::Slow => "~",
            ConnectionStatus::Disconnected => "0%",
        }
    } else {
        "--"
    };

    let uptime_row = Line::from(vec![
        Span::styled(
            format!("{:>label_width$}", "Uptime", label_width = label_width),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            format!("{:>col_width$}", up_now_str, col_width = col_width),
            Style::default().fg(up_now_color),
        ),
        fmt_up(up_1m, col_width),
        fmt_up(up_5m, col_width),
        fmt_up(up_15m, col_width),
    ]);

    // Latency row (right-aligned)

    let table_lines = vec![header, latency_row, uptime_row];

    f.render_widget(
        Paragraph::new(table_lines).alignment(Alignment::Left),
        stats_inner,
    );

    // --- Right Section: Times ---
    let times_block = Block::default().borders(Borders::ALL).title("Times");
    let times_inner = times_block.inner(columns[2]);
    f.render_widget(times_block, columns[2]);

    let time_label_width = 14;

    let times_lines = vec![
        Line::from(vec![
            Span::styled(
                format!("{:>w$} ", "Session:", w = time_label_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(state.format_running_time()),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:>w$} ", "Since Outage:", w = time_label_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(state.time_since_last_incident()),
        ]),
        Line::from(vec![
            Span::styled(
                format!("{:>w$} ", "Last Outage:", w = time_label_width),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(state.last_outage_duration()),
        ]),
    ];

    f.render_widget(
        Paragraph::new(times_lines).alignment(Alignment::Left),
        times_inner,
    );
}

fn render_network_info(f: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title("Network Identity");
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_area);

    // Left Column: Public IPs and Location
    let (ipv4, ipv6, location_line) = if let Some(info) = &state.network_info {
        let loc = if let (Some(city), Some(country)) = (&info.city, &info.country) {
            format!("{}, {}", city, country)
        } else {
            "N/A".to_string()
        };
        (
            info.public_ipv4.as_deref().unwrap_or("N/A"),
            info.public_ipv6.as_deref().unwrap_or("N/A"),
            loc,
        )
    } else {
        ("Loading...", "Loading...", "Loading...".to_string())
    };

    let ip_lines = vec![
        Line::from(vec![
            Span::styled("IPv4: ", Style::default().fg(Color::DarkGray)),
            Span::styled(ipv4, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("IPv6: ", Style::default().fg(Color::DarkGray)),
            Span::styled(ipv6, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("Loc:  ", Style::default().fg(Color::DarkGray)),
            Span::styled(location_line, Style::default().fg(Color::White)),
        ]),
    ];
    f.render_widget(Paragraph::new(ip_lines), columns[0]);

    // Right Column: Interface Info and ISP
    let (if_name, if_type, local_ip, wifi_ssid, signal_strength, _signal_noise, isp_line) =
        if let Some(info) = &state.network_info {
            let isp_display = match (&info.isp, &info.asn) {
                (Some(isp), Some(asn)) => format!("{} ({})", isp, asn),
                (Some(isp), None) => isp.clone(),
                (None, Some(asn)) => asn.clone(),
                (None, None) => "N/A".to_string(),
            };
            (
                info.interface_name.as_deref().unwrap_or("Unknown"),
                info.interface_type.as_deref().unwrap_or(""),
                info.local_ip.as_deref().unwrap_or("N/A"),
                info.wifi_ssid.as_deref(),
                info.signal_strength,
                info.signal_noise,
                isp_display,
            )
        } else {
            ("...", "", "...", None, None, None, "...".to_string())
        };

    let type_display = if let Some(ssid) = wifi_ssid {
        format!(" (Wi-Fi: {})", ssid)
    } else if !if_type.is_empty() {
        format!(" ({})", if_type)
    } else {
        String::new()
    };

    let mut interface_lines = vec![
        Line::from(vec![
            Span::styled("Int:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                format!("{}{}", if_name, type_display),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("Local: ", Style::default().fg(Color::DarkGray)),
            Span::styled(local_ip, Style::default().fg(Color::White)),
        ]),
    ];

    // Insert Signal line if available
    if let Some(signal) = signal_strength {
        // Color code the signal
        // Excellent/Good: > -60 dBm
        // Fair: -60 to -70 dBm
        // Weak: < -70 dBm
        let color = if signal > -60 {
            Color::Green
        } else if signal > -70 {
            Color::Yellow
        } else {
            Color::Red
        };

        interface_lines.push(Line::from(vec![
            Span::styled("Signal:", Style::default().fg(Color::DarkGray)),
            Span::styled(format!(" {} dBm", signal), Style::default().fg(color)),
        ]));
    }

    interface_lines.push(Line::from(vec![
        Span::styled("ISP:   ", Style::default().fg(Color::DarkGray)),
        Span::styled(isp_line, Style::default().fg(Color::White)),
    ]));

    f.render_widget(Paragraph::new(interface_lines), columns[1]);
}

fn render_main_content(f: &mut Frame, area: Rect, state: &TuiState) {
    let provider_count = TARGET_LABELS.len();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((provider_count + 1 + 1 + 2) as u16), // Dynamic Provider box height (+1 spacer, +1 Aggregate)
            Constraint::Length(1),                                   // Spacer
            Constraint::Length(4), // Unified History (Title + 2 rows + Borders)
        ])
        .split(area);

    let provider_block = Block::default()
        .title("Targets")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray));

    let inner_area = provider_block.inner(rows[0]);
    f.render_widget(provider_block, rows[0]);

    let provider_rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints({
            let mut c = vec![Constraint::Length(1); provider_count];
            c.push(Constraint::Length(1)); // Spacer
            c.push(Constraint::Length(1)); // For Aggregate row
            c
        })
        .split(inner_area);

    for i in 0..provider_count {
        render_site_row(f, provider_rows[i], state, i, false);
    }

    // Render Global/Aggregate Row (after spacer)
    render_global_row(f, provider_rows[provider_count + 1], state);

    render_unified_history_box(f, rows[2], state);
}

fn render_site_row(f: &mut Frame, area: Rect, state: &TuiState, idx: usize, use_borders: bool) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(11), // Name
            Constraint::Length(18), // IP
            Constraint::Length(8),  // Latency (Right aligned)
            Constraint::Min(10),    // Bar (Expands)
        ])
        .split(area);

    let name = TARGET_LABELS[idx];
    let ip = TARGETS[idx];
    let latency = if let Some(round) = state.history.back() {
        if let Some(res) = round.results.get(idx) {
            if let Some(l) = res.latency_ms {
                format!("{:.0}ms ", l)
            } else {
                "timeout ".to_string()
            }
        } else {
            "... ".to_string()
        }
    } else {
        "... ".to_string()
    };

    let name_para = Paragraph::new(Span::styled(
        name,
        Style::default().add_modifier(Modifier::BOLD),
    ));
    let ip_para = Paragraph::new(Span::styled(ip, Style::default().fg(Color::DarkGray)));
    let latency_para = Paragraph::new(Span::styled(latency, Style::default().fg(Color::Cyan)))
        .alignment(Alignment::Right);

    f.render_widget(name_para, chunks[0]);
    f.render_widget(ip_para, chunks[1]);
    f.render_widget(latency_para, chunks[2]);

    // Render bar
    let bar_width = chunks[3].width as usize;
    let (effective_width, bar_block) = if use_borders {
        if bar_width <= 2 {
            return;
        }
        (bar_width - 2, Block::default().borders(Borders::ALL))
    } else {
        (bar_width, Block::default())
    };

    let mut spans = Vec::new();

    // Show 1 minute of history, spanning the full bar width
    let window_seconds: i64 = 60;
    let now = Utc::now();
    let probe_interval = chrono::Duration::milliseconds(PROBE_INTERVAL_MS as i64);

    // Determine the window for this timeframe
    let elapsed_secs = now.signed_duration_since(state.session_start).num_seconds();

    // Duration per character in seconds
    let total_duration_ms = window_seconds * 1000;
    // Add tolerance for probe jitter (50% of interval)
    let tolerance = chrono::Duration::milliseconds((PROBE_INTERVAL_MS / 2) as i64);

    // Snap window start to character width to prevent aliasing/shimmering
    let ms_per_char = if effective_width > 0 {
        total_duration_ms / effective_width as i64
    } else {
        1000
    };

    let elapsed_ms = now
        .signed_duration_since(state.session_start)
        .num_milliseconds();
    let snapped_elapsed_ms = (elapsed_ms / ms_per_char) * ms_per_char;

    // Recalculate window start with snapped time
    let window_start = if elapsed_secs < window_seconds {
        state.session_start
    } else {
        state.session_start + chrono::Duration::milliseconds(snapped_elapsed_ms)
            - chrono::Duration::seconds(window_seconds)
    };

    // Optimization: Pre-filter history to only include relevant rounds
    // Find the first round that ends after our window starts
    let start_index = state
        .history
        .iter()
        .position(|round| {
            let probe_valid_until = round.timestamp + probe_interval + tolerance;
            probe_valid_until > window_start
        })
        .unwrap_or(state.history.len());

    let relevant_history: Vec<_> = state.history.iter().skip(start_index).collect();

    for i in 0..effective_width {
        let bucket_start_ms = (i as i64 * total_duration_ms) / effective_width as i64;
        let bucket_end_ms = ((i + 1) as i64 * total_duration_ms) / effective_width as i64;

        let bucket_start = window_start + chrono::Duration::milliseconds(bucket_start_ms);
        let bucket_end = window_start + chrono::Duration::milliseconds(bucket_end_ms);

        let mut has_data = false;
        let mut status = ConnectionStatus::Ok;

        for round in &relevant_history {
            let probe_valid_until = round.timestamp + probe_interval + tolerance;

            if round.timestamp < bucket_end && probe_valid_until > bucket_start {
                has_data = true;

                if let Some(res) = round.results.get(idx) {
                    if res.success {
                        if let Some(l) = res.latency_ms {
                            if l > 300.0 {
                                status = ConnectionStatus::Disconnected;
                                break;
                            } else if l > 100.0 && status != ConnectionStatus::Disconnected {
                                status = ConnectionStatus::Slow;
                            }
                        } else {
                            status = ConnectionStatus::Disconnected;
                            break;
                        }
                    } else {
                        status = ConnectionStatus::Disconnected;
                        break;
                    }
                }
            }
        }

        let (ch, color) = if !has_data {
            (' ', Color::Black)
        } else {
            match status {
                ConnectionStatus::Ok => ('·', Color::Green),
                ConnectionStatus::Slow => ('!', Color::Yellow),
                ConnectionStatus::Disconnected => ('█', Color::Red),
            }
        };
        spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).block(bar_block),
        chunks[3],
    );
}

fn render_global_row(f: &mut Frame, area: Rect, state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(11), // Name
            Constraint::Length(18), // IP
            Constraint::Length(8),  // Latency
            Constraint::Min(10),    // Bar
        ])
        .split(area);

    // Calculate average latency
    let latency_str = if let Some(round) = state.history.back() {
        let successful_results: Vec<f64> = round
            .results
            .iter()
            .filter(|r| r.success)
            .filter_map(|r| r.latency_ms)
            .collect();

        if !successful_results.is_empty() {
            let avg = successful_results.iter().sum::<f64>() / successful_results.len() as f64;
            format!("{:.0}ms ", avg)
        } else {
            "... ".to_string()
        }
    } else {
        "... ".to_string()
    };

    let name_para = Paragraph::new(Span::styled(
        "Aggregate",
        Style::default().add_modifier(Modifier::BOLD),
    ));
    let ip_para = Paragraph::new(Span::styled("", Style::default().fg(Color::DarkGray)));
    let latency_para = Paragraph::new(Span::styled(latency_str, Style::default().fg(Color::Cyan)))
        .alignment(Alignment::Right);

    f.render_widget(name_para, chunks[0]);
    f.render_widget(ip_para, chunks[1]);
    f.render_widget(latency_para, chunks[2]);

    // Render bar
    let bar_width = chunks[3].width as usize;
    if bar_width <= 2 {
        return;
    }

    // Use effective width directly (no border logic needed as this is inside the container)
    let effective_width = bar_width;

    let mut spans = Vec::new();

    // Show 1 minute of history, spanning the full bar width
    let window_seconds: i64 = 60;
    let now = Utc::now();
    let probe_interval = chrono::Duration::milliseconds(PROBE_INTERVAL_MS as i64);

    // Determine the window for this timeframe
    let elapsed_secs = now.signed_duration_since(state.session_start).num_seconds();

    // Duration per character in seconds
    let total_duration_ms = window_seconds * 1000;
    // Add tolerance for probe jitter (50% of interval)
    let tolerance = chrono::Duration::milliseconds((PROBE_INTERVAL_MS / 2) as i64);

    // Snap window start to character width to prevent aliasing/shimmering
    let ms_per_char = if effective_width > 0 {
        total_duration_ms / effective_width as i64
    } else {
        1000
    };

    let elapsed_ms = now
        .signed_duration_since(state.session_start)
        .num_milliseconds();
    let snapped_elapsed_ms = (elapsed_ms / ms_per_char) * ms_per_char;

    // Recalculate window start with snapped time
    let window_start = if elapsed_secs < window_seconds {
        state.session_start
    } else {
        state.session_start + chrono::Duration::milliseconds(snapped_elapsed_ms)
            - chrono::Duration::seconds(window_seconds)
    };

    // Optimization: Pre-filter history to only include relevant rounds
    // Find the first round that ends after our window starts
    let start_index = state
        .history
        .iter()
        .position(|round| {
            let probe_valid_until = round.timestamp + probe_interval + tolerance;
            probe_valid_until > window_start
        })
        .unwrap_or(state.history.len());

    let relevant_history: Vec<_> = state.history.iter().skip(start_index).collect();

    for i in 0..effective_width {
        let bucket_start_ms = (i as i64 * total_duration_ms) / effective_width as i64;
        let bucket_end_ms = ((i + 1) as i64 * total_duration_ms) / effective_width as i64;

        let bucket_start = window_start + chrono::Duration::milliseconds(bucket_start_ms);
        let bucket_end = window_start + chrono::Duration::milliseconds(bucket_end_ms);

        let mut has_data = false;
        let mut status = ConnectionStatus::Ok;

        for round in &relevant_history {
            let probe_valid_until = round.timestamp + probe_interval + tolerance;

            if round.timestamp < bucket_end && probe_valid_until > bucket_start {
                has_data = true;

                // Majority vote logic
                let failed_count = round.results.iter().filter(|r| !r.success).count();
                let avg_latency: f64 = if round.results.iter().any(|r| r.success) {
                    round
                        .results
                        .iter()
                        .filter_map(|r| r.latency_ms)
                        .sum::<f64>()
                        / round.results.iter().filter(|r| r.success).count() as f64
                } else {
                    f64::MAX
                };

                if failed_count >= 2 || avg_latency > 300.0 {
                    status = ConnectionStatus::Disconnected;
                    break;
                } else if avg_latency > 100.0 && status != ConnectionStatus::Disconnected {
                    status = ConnectionStatus::Slow;
                }
            }
        }

        let (ch, color) = if !has_data {
            (' ', Color::Black)
        } else {
            match status {
                ConnectionStatus::Ok => ('·', Color::Green),
                ConnectionStatus::Slow => ('!', Color::Yellow),
                ConnectionStatus::Disconnected => ('█', Color::Red),
            }
        };
        spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), chunks[3]);
}

fn render_unified_history_box(f: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default().title("History").borders(Borders::ALL);
    let inner_area = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner_area);

    render_summary_row(f, rows[0], state, "5m", 300);
    render_summary_row(f, rows[1], state, "15m", 900);
}

fn render_summary_row(f: &mut Frame, area: Rect, state: &TuiState, label: &str, seconds: i64) {
    // Match the layout of render_site_row: Name(11) + IP(18) + Latency(8) = 37
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(37), Constraint::Min(10)])
        .split(area);

    f.render_widget(
        Paragraph::new(label).style(Style::default().fg(Color::DarkGray)),
        chunks[0],
    );

    let bar_width = chunks[1].width as usize;
    if bar_width == 0 {
        return;
    }

    let effective_width = bar_width;

    let mut spans = Vec::new();
    let now = Utc::now();
    let probe_interval = chrono::Duration::milliseconds(PROBE_INTERVAL_MS as i64);

    // Determine the window for this timeframe
    let elapsed_secs = now.signed_duration_since(state.session_start).num_seconds();
    // Duration per character in seconds
    let total_duration_ms = seconds * 1000;
    // Add tolerance for probe jitter (50% of interval)
    let tolerance = chrono::Duration::milliseconds((PROBE_INTERVAL_MS / 2) as i64);

    // Snap window start to character width to prevent aliasing/shimmering
    let ms_per_char = if effective_width > 0 {
        total_duration_ms / effective_width as i64
    } else {
        1000
    };

    let elapsed_ms = now
        .signed_duration_since(state.session_start)
        .num_milliseconds();
    let snapped_elapsed_ms = (elapsed_ms / ms_per_char) * ms_per_char;

    // Recalculate window start with snapped time
    let window_start = if elapsed_secs < seconds {
        state.session_start
    } else {
        state.session_start + chrono::Duration::milliseconds(snapped_elapsed_ms)
            - chrono::Duration::seconds(seconds)
    };

    // Optimization: Pre-filter history to only include relevant rounds
    // Find the first round that ends after our window starts
    let start_index = state
        .history
        .iter()
        .position(|round| {
            let probe_valid_until = round.timestamp + probe_interval + tolerance;
            probe_valid_until > window_start
        })
        .unwrap_or(state.history.len());

    let relevant_history: Vec<_> = state.history.iter().skip(start_index).collect();

    for i in 0..effective_width {
        let bucket_start_ms = (i as i64 * total_duration_ms) / effective_width as i64;
        let bucket_end_ms = ((i + 1) as i64 * total_duration_ms) / effective_width as i64;

        let bucket_start = window_start + chrono::Duration::milliseconds(bucket_start_ms);
        let bucket_end = window_start + chrono::Duration::milliseconds(bucket_end_ms);

        // Find if any disconnection or slow status in this bucket
        let mut has_disconnection = false;
        let mut has_data = false;
        let mut has_slow = false;

        for round in &relevant_history {
            // Gap fix: Check if the probe's validity window overlaps with the bucket
            // Probe is valid for [timestamp, timestamp + interval)
            // Overlap condition: start1 < end2 && start2 < end1
            let probe_valid_until = round.timestamp + probe_interval + tolerance;

            if round.timestamp < bucket_end && probe_valid_until > bucket_start {
                has_data = true;

                // Disconnection: majority vote or high latency
                let failed_count = round.results.iter().filter(|r| !r.success).count();
                let avg_latency: f64 = if round.results.iter().any(|r| r.success) {
                    round
                        .results
                        .iter()
                        .filter_map(|r| r.latency_ms)
                        .sum::<f64>()
                        / round.results.iter().filter(|r| r.success).count() as f64
                } else {
                    f64::MAX
                };

                if failed_count >= 2 || avg_latency > 300.0 {
                    has_disconnection = true;
                    break;
                } else if avg_latency > 100.0 {
                    has_slow = true;
                }
            }
        }

        let (ch, color) = if !has_data {
            (' ', Color::Black)
        } else if has_disconnection {
            ('█', Color::Red)
        } else if has_slow {
            ('!', Color::Yellow)
        } else {
            ('·', Color::Green)
        };
        spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }

    f.render_widget(Paragraph::new(Line::from(spans)), chunks[1]);
}

fn render_long_term_status(f: &mut Frame, area: Rect, _state: &TuiState) {
    let quit_text = Paragraph::new(Line::from(Span::styled(
        "Press 'q' to quit",
        Style::default().fg(Color::DarkGray),
    )))
    .alignment(Alignment::Center);

    f.render_widget(quit_text, area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::{PingResult, ProbeRound};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    #[test]
    fn test_render_summary_row_performance() {
        let mut state = TuiState::new();
        let now = Utc::now();

        // Fill history with 7200 items (simulating 1 hour)
        for i in 0..7200 {
            let timestamp = now - chrono::Duration::milliseconds((7200 - i) * 500);
            let round = ProbeRound {
                results: vec![PingResult {
                    target: "8.8.8.8".to_string(),
                    success: true,
                    latency_ms: Some(20.0),
                    timestamp,
                }],
                timestamp,
            };
            state.history.push_back(round);
        }

        let backend = TestBackend::new(100, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let start = std::time::Instant::now();

        // Render 10 frames
        for _ in 0..10 {
            terminal
                .draw(|f| {
                    let area = f.size();
                    render_summary_row(f, area, &state, "Test", 300); // 5m window
                })
                .unwrap();
        }

        let duration = start.elapsed();
        println!("Rendered 10 frames with full history in {:?}", duration);

        // Assertion to ensure it's reasonably fast (e.g., < 100ms for 10 frames)
        // Without optimization, this would likely be much slower.
        // With optimization, O(width * relevant_history) vs O(width * full_history)
        // relevant_history for 5m (300s) is 600 items. Full is 7200.
        // So it should be ~10x faster.
        assert!(
            duration.as_millis() < 500,
            "Rendering took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_render_site_row_performance() {
        let mut state = TuiState::new();
        let now = Utc::now();

        // Fill history with 7200 items (simulating 1 hour)
        for i in 0..7200 {
            let timestamp = now - chrono::Duration::milliseconds((7200 - i) * 500);
            let round = ProbeRound {
                results: vec![PingResult {
                    target: "8.8.8.8".to_string(),
                    success: true,
                    latency_ms: Some(20.0),
                    timestamp,
                }],
                timestamp,
            };
            state.history.push_back(round);
        }

        let backend = TestBackend::new(100, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let start = std::time::Instant::now();

        // Render 10 frames
        for _ in 0..10 {
            terminal
                .draw(|f| {
                    let area = Rect::new(0, 0, 100, 1);
                    // idx 0 corresponds to the single target we added result for
                    render_site_row(f, area, &state, 0, false);
                })
                .unwrap();
        }

        let duration = start.elapsed();
        println!(
            "Rendered 10 frames of site_row with full history in {:?}",
            duration
        );

        assert!(
            duration.as_millis() < 50,
            "Rendering site_row took too long: {:?}",
            duration
        );
    }

    #[test]
    fn test_render_global_row_performance() {
        let mut state = TuiState::new();
        let now = Utc::now();

        // Fill history with 7200 items
        for i in 0..7200 {
            let timestamp = now - chrono::Duration::milliseconds((7200 - i) * 500);
            let round = ProbeRound {
                results: vec![PingResult {
                    target: "8.8.8.8".to_string(),
                    success: true,
                    latency_ms: Some(20.0),
                    timestamp,
                }],
                timestamp,
            };
            state.history.push_back(round);
        }

        let backend = TestBackend::new(100, 10);
        let mut terminal = Terminal::new(backend).unwrap();

        let start = std::time::Instant::now();

        // Render 10 frames
        for _ in 0..10 {
            terminal
                .draw(|f| {
                    let area = Rect::new(0, 0, 100, 1);
                    render_global_row(f, area, &state);
                })
                .unwrap();
        }

        let duration = start.elapsed();
        println!(
            "Rendered 10 frames of global_row with full history in {:?}",
            duration
        );

        assert!(
            duration.as_millis() < 50,
            "Rendering global_row took too long: {:?}",
            duration
        );
    }
}
