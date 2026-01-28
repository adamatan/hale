mod analysis;
mod config;
mod monitor;
mod ui;
mod utils;

use crate::analysis::aggregate_stats;
use crate::config::HISTORY_WINDOW_SIZE;
use crate::monitor::net_info::{self, NetworkInfo};
use crate::monitor::prober::run_probe_loop;
use crate::ui::parse_args;
use crate::utils::logger::SessionLogger;
use crossterm::event::{Event, EventStream, KeyCode, KeyModifiers};
use futures::StreamExt;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::mpsc;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let cli = parse_args();

    if cli.check {
        // CLI mode - single check and exit
        run_cli_mode(cli.format, cli.verbose).await?;
    } else {
        // TUI mode - continuous monitoring (default)
        run_tui_mode().await?;
    }

    Ok(())
}

async fn run_tui_mode() -> Result<(), Box<dyn std::error::Error>> {
    // Setup panic hook to restore terminal
    ui::tui::setup_panic_hook();

    // Initialize terminal
    let mut terminal = ui::tui::init_terminal()?;

    // Create logger
    let mut logger = SessionLogger::new()?;
    let log_path = logger.log_path().clone();

    // Create channel for probe results
    let (probe_tx, mut probe_rx) = mpsc::channel(100);

    // Create channel for aggregated stats (stats, latest_round)
    let (stats_tx, mut stats_rx) =
        mpsc::channel::<(crate::monitor::NetworkStats, crate::monitor::ProbeRound)>(100);

    // Create channel for network info
    let (net_info_tx, mut net_info_rx) = mpsc::channel::<NetworkInfo>(10);

    // Start probe loop
    let _probe_handle = tokio::spawn(async move {
        run_probe_loop(probe_tx).await;
    });

    // Start network info refresh loop
    let _net_info_handle = tokio::spawn(async move {
        // Initial fetch
        let info = net_info::refresh_network_info().await;
        let _ = net_info_tx.send(info).await;

        // Periodic fetch
        let mut interval = tokio::time::interval(Duration::from_secs(5));
        interval.tick().await; // First tick completes immediately, we already did initial fetch

        loop {
            interval.tick().await;
            let info = net_info::refresh_network_info().await;
            if net_info_tx.send(info).await.is_err() {
                break;
            }
        }
    });

    // Start aggregation task
    let _agg_handle = tokio::spawn(async move {
        let mut history = VecDeque::with_capacity(HISTORY_WINDOW_SIZE);

        while let Some(round) = probe_rx.recv().await {
            history.push_back(round.clone());

            // Keep history within size limit
            if history.len() > HISTORY_WINDOW_SIZE {
                history.pop_front();
            }

            // Aggregate statistics
            let rounds: Vec<_> = history.iter().cloned().collect();
            let stats = aggregate_stats(&rounds);

            // Send to TUI (stats and the round that triggered it)
            let _ = stats_tx.send((stats, round)).await;
        }
    });

    // Create TUI state
    let mut tui_state = ui::tui::TuiState::new();

    // Create shared atomic flag for quit signal
    let should_quit = Arc::new(AtomicBool::new(false));
    let should_quit_clone = should_quit.clone();

    // Spawn dedicated task for keyboard handling
    // This ensures keyboard events are processed immediately even if the main loop is busy
    tokio::spawn(async move {
        let mut event_stream = EventStream::new();

        while let Some(maybe_event) = event_stream.next().await {
            if let Ok(Event::Key(key)) = maybe_event {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        should_quit_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                    KeyCode::Char('c') | KeyCode::Char('C')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        should_quit_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                    KeyCode::Char('d') | KeyCode::Char('D')
                        if key.modifiers.contains(KeyModifiers::CONTROL) =>
                    {
                        should_quit_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                    KeyCode::Esc => {
                        should_quit_clone.store(true, Ordering::SeqCst);
                        break;
                    }
                    _ => {}
                }
            }
        }
    });

    // Run TUI loop with logging
    let result = async {
        loop {
            // Check quit flag
            if should_quit.load(Ordering::SeqCst) {
                tui_state.should_quit = true;
            }

            if tui_state.should_quit {
                break;
            }

            // Update terminal and handle events
            terminal.draw(|f| ui::tui::ui(f, &tui_state))?;

            tokio::select! {
                // Receive probe results and log status changes
                Some((stats, latest_round)) = stats_rx.recv() => {
                    logger.log_stats(&stats)?;
                    tui_state.update_stats(stats, latest_round);
                }

                // Receive network info updates
                Some(info) = net_info_rx.recv() => {
                    tui_state.network_info = Some(info);
                }

                // Handle Ctrl+C signal
                _ = tokio::signal::ctrl_c() => {
                    tui_state.should_quit = true;
                }
            }

            if tui_state.should_quit {
                break;
            }
        }

        Ok::<(), Box<dyn std::error::Error>>(())
    }
    .await;

    // Restore terminal
    ui::tui::restore_terminal(&mut terminal)?;

    // Display log path
    println!("\nSession log saved to: {}", log_path.display());

    result
}

async fn run_cli_mode(
    format: ui::OutputFormat,
    verbose: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Start network info fetch in background
    let net_info_task = tokio::spawn(async { net_info::refresh_network_info().await });

    // Create channel for probe results
    let (tx, mut rx) = mpsc::channel(100);

    // Start probe loop
    let _probe_handle = tokio::spawn(async move {
        run_probe_loop(tx).await;
    });

    // Create history buffer
    let mut history = VecDeque::with_capacity(HISTORY_WINDOW_SIZE);

    // Wait until we have enough history for a meaningful assessment
    // Collect at least 3 rounds before displaying
    let min_rounds = 3;
    let mut rounds_collected = 0;

    while let Some(round) = rx.recv().await {
        history.push_back(round);
        rounds_collected += 1;

        // Keep history within size limit
        if history.len() > HISTORY_WINDOW_SIZE {
            history.pop_front();
        }

        // Once we have enough data, aggregate and display once, then exit
        if rounds_collected >= min_rounds {
            let rounds: Vec<_> = history.iter().cloned().collect();
            let stats = aggregate_stats(&rounds);

            // Wait for network info (should be ready by now)
            let network_info = net_info_task.await.ok();

            // Display in CLI format
            ui::cli::display_cli_stats(&stats, &format, verbose, network_info.as_ref());

            // Exit with appropriate code
            let exit_code = ui::cli::get_exit_code(stats.status);
            std::process::exit(exit_code);
        }
    }

    Ok(())
}
