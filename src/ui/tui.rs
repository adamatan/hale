use crate::config::{HISTORY_WINDOW_SIZE, TARGETS, TARGET_LABELS};
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
        }
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
            format!("Uptime: {}h {}m {}s", hours, minutes, secs)
        } else if minutes > 0 {
            format!("Uptime: {}m {}s", minutes, secs)
        } else {
            format!("Uptime: {}s", secs)
        }
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
            Constraint::Length(3), // Status banner
            Constraint::Min(16),   // Main content (3 sites + spacer + 2 timeframes = 16 lines)
            Constraint::Length(6), // Long-term status (reduced from 8)
        ])
        .split(f.size());

    render_status_banner(f, main_chunks[0], state);
    render_main_content(f, main_chunks[1], state);
    render_long_term_status(f, main_chunks[2], state);
}

fn render_status_banner(f: &mut Frame, area: Rect, state: &TuiState) {
    let (symbol, text, bg_color, fg_color) = if let Some(stats) = &state.stats {
        match stats.status {
            ConnectionStatus::Ok => ("✓", "OK", Color::Green, Color::Black),
            ConnectionStatus::Slow => ("⚠", "SLOW", Color::Yellow, Color::Black),
            ConnectionStatus::Disconnected => ("✗", "DISCONNECTED", Color::Red, Color::White),
        }
    } else {
        ("⋯", "INITIALIZING", Color::DarkGray, Color::White)
    };

    let content_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    // Left Box: Latency
    let latency_text = if let Some(stats) = &state.stats {
        format!("{:.0}ms", stats.avg_latency_ms)
    } else {
        "...".to_string()
    };
    let latency_block = Block::default().borders(Borders::ALL).title("Avg Latency");
    let latency_para = Paragraph::new(latency_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(latency_block);
    f.render_widget(latency_para, content_chunks[0]);

    // Center Box: Status (Only this one gets background color INSIDE)
    let status_text = format!("{} {}", symbol, text);
    let status_block = Block::default().borders(Borders::ALL).title("Status");

    // We render the block first, then render the paragraph inside it with the background style
    f.render_widget(status_block.clone(), content_chunks[1]);

    let status_para = Paragraph::new(status_text)
        .style(
            Style::default()
                .bg(bg_color)
                .fg(fg_color)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center);

    // Render the paragraph in the inner area of the block
    f.render_widget(status_para, status_block.inner(content_chunks[1]));

    // Right Box: Uptime
    let uptime_text = state.time_since_last_incident();
    let uptime_block = Block::default().borders(Borders::ALL).title("Timer");
    let uptime_para = Paragraph::new(uptime_text)
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .alignment(Alignment::Center)
        .block(uptime_block);
    f.render_widget(uptime_para, content_chunks[2]);
}

fn render_main_content(f: &mut Frame, area: Rect, state: &TuiState) {
    let provider_count = TARGET_LABELS.len();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length((provider_count + 2) as u16), // Dynamic Provider box height
            Constraint::Length(1),                           // Spacer
            Constraint::Length(3),                           // 5m
            Constraint::Length(3),                           // 1h
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
        .constraints(vec![Constraint::Length(1); provider_count])
        .split(inner_area);

    for i in 0..provider_count {
        render_site_row(f, provider_rows[i], state, i, false);
    }

    render_timeframe_row(f, rows[2], state, "5 minutes", 300);
    render_timeframe_row(f, rows[3], state, "1 hour", 3600);
}

fn render_site_row(f: &mut Frame, area: Rect, state: &TuiState, idx: usize, use_borders: bool) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(38), // Increased for Name, IP, and Latency alignment
            Constraint::Min(10),    // Status bar
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

    // Sub-layout for Name (left), IP (left), and latency (right)
    let info_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(11), // Name
            Constraint::Length(18), // IP
            Constraint::Min(5),     // Latency
        ])
        .split(chunks[0]);

    let name_para = Paragraph::new(Span::styled(
        name,
        Style::default().add_modifier(Modifier::BOLD),
    ));
    let ip_para = Paragraph::new(Span::styled(ip, Style::default().fg(Color::DarkGray)));
    let latency_para = Paragraph::new(Span::styled(latency, Style::default().fg(Color::Cyan)))
        .alignment(Alignment::Right);

    f.render_widget(name_para, info_chunks[0]);
    f.render_widget(ip_para, info_chunks[1]);
    f.render_widget(latency_para, info_chunks[2]);

    // Render bar
    let bar_width = chunks[1].width as usize;
    let (effective_width, bar_block) = if use_borders {
        if bar_width <= 2 {
            return;
        }
        (bar_width - 2, Block::default().borders(Borders::ALL))
    } else {
        (bar_width, Block::default())
    };

    let mut spans = Vec::new();

    // Each character represents one probe round (simplification for site rows)
    let history_len = state.history.len();
    let start_idx = history_len.saturating_sub(effective_width);

    for i in start_idx..history_len {
        let round = &state.history[i];
        let status = if let Some(res) = round.results.get(idx) {
            if res.success {
                if let Some(l) = res.latency_ms {
                    if l > 300.0 {
                        ConnectionStatus::Disconnected
                    } else if l > 100.0 {
                        ConnectionStatus::Slow
                    } else {
                        ConnectionStatus::Ok
                    }
                } else {
                    ConnectionStatus::Disconnected
                }
            } else {
                ConnectionStatus::Disconnected
            }
        } else {
            ConnectionStatus::Disconnected
        };

        let (ch, color) = match status {
            ConnectionStatus::Ok => ('·', Color::Green),
            ConnectionStatus::Slow => ('!', Color::Yellow),
            ConnectionStatus::Disconnected => ('█', Color::Red),
        };
        spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
    }

    f.render_widget(
        Paragraph::new(Line::from(spans)).block(bar_block),
        chunks[1],
    );
}

fn render_timeframe_row(f: &mut Frame, area: Rect, state: &TuiState, label: &str, seconds: i64) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(38), Constraint::Min(10)])
        .split(area);

    f.render_widget(
        Paragraph::new(label)
            .block(Block::default().borders(Borders::LEFT | Borders::TOP | Borders::BOTTOM)),
        chunks[0],
    );

    let bar_width = chunks[1].width as usize;
    if bar_width <= 2 {
        return;
    } // Account for borders
    let effective_width = bar_width - 2;

    let mut spans = Vec::new();
    let now = Utc::now();

    // Determine the window for this timeframe
    let elapsed_secs = now.signed_duration_since(state.session_start).num_seconds();
    let (window_start, _window_end) = if elapsed_secs < seconds {
        // Session is younger than the timeframe: show from session start
        (
            state.session_start,
            state.session_start + chrono::Duration::seconds(seconds),
        )
    } else {
        // Session is older: show last X seconds (sliding window)
        (now - chrono::Duration::seconds(seconds), now)
    };

    // Duration per character in seconds
    let seconds_per_char = seconds as f64 / effective_width as f64;

    for i in 0..effective_width {
        let bucket_start = window_start
            + chrono::Duration::milliseconds((i as f64 * seconds_per_char * 1000.0) as i64);
        let bucket_end = window_start
            + chrono::Duration::milliseconds(((i + 1) as f64 * seconds_per_char * 1000.0) as i64);

        // Find if any disconnection or slow status in this bucket
        let mut has_disconnection = false;
        let mut has_data = false;
        let mut has_slow = false;

        for round in state.history.iter() {
            if round.timestamp >= bucket_start && round.timestamp < bucket_end {
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

    f.render_widget(
        Paragraph::new(Line::from(spans)).block(Block::default().borders(Borders::ALL)),
        chunks[1],
    );
}

fn render_long_term_status(f: &mut Frame, area: Rect, state: &TuiState) {
    let block = Block::default()
        .title("Session Info")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::White));

    let running_time = state.format_running_time();
    let mut lines = vec![Line::from(Span::styled(
        format!("Running time: {}", running_time),
        Style::default().fg(Color::Cyan),
    ))];

    let recent_disconnections: Vec<_> = state.disconnections.iter().rev().take(3).collect();
    if recent_disconnections.is_empty() {
        lines.push(Line::from(Span::styled(
            "No disconnections",
            Style::default().fg(Color::Green),
        )));
    } else {
        for event in recent_disconnections {
            let duration = event.duration_seconds();
            let status_text = if event.end_time.is_some() {
                format!("Disconnection: {}s (recovered)", duration)
            } else {
                format!("Disconnection: {}s (ongoing)", duration)
            };
            lines.push(Line::from(Span::styled(
                status_text,
                Style::default().fg(Color::Red),
            )));
        }
    }

    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "Press 'q' to quit",
        Style::default().fg(Color::DarkGray),
    )));

    f.render_widget(Paragraph::new(lines).block(block), area);
}
