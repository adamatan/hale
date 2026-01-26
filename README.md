# hale

[![CI](https://github.com/adamatan/hale/actions/workflows/ci.yml/badge.svg)](https://github.com/adamatan/hale/actions/workflows/ci.yml)

**hale** is a lightweight, high-precision network connection quality monitor. It answers the critical question: *"Is my internet connection OK right now?"*

Unlike standard ping tools, **hale** uses concurrent TCP probing across six major global networks to provide an at-a-glance health assessment without requiring root/sudo privileges.

## Vision & Purpose

It's often difficult to tell if internet slowness is caused by your own connection, a specific website, or a wider outage. **hale** provides immediate clarity through:
*   **Immediate clarity**: Big, obvious status indicators (OK/Slow/Disconnected).
*   **Context-aware feedback**: Descriptive explanations when issues are detected.
*   **High-precision monitoring**: Optimized for detecting micro-dropouts and stability issues that standard tools miss.

## Key Features

*   **Dual Mode Operation**: 
    *   **TUI (Default)**: A rich, real-time dashboard with historical status bars and per-provider metrics.
    *   **CLI (`--check`)**: A one-shot diagnostic tool with proper exit codes for scripts and automation.
*   **Intelligent Analysis**:
    *   **Multi-Provider Probing**: Simultaneously monitors Google, AWS, Azure, Cloudflare, Quad9, and OpenDNS.
    *   **Majority Voting**: Uses a 4/6 threshold to filter out localized network noise or single-provider outages.
    *   **High Frequency**: 500ms update interval for immediate detection of micro-dropouts.
*   **Historical Visibility**:
    *   Per-provider status history bars.
    *   5-minute and 1-hour aggregate connection overviews.
    *   Automatic downtime tracking and duration calculation.
*   **Lean & Safe**:
    *   **Resource Efficient**: Memory footprint < 5MB, CPU usage < 1%.
    *   **Zero Configuration**: Works out of the box with sensible defaults.
    *   **Terminal Safety**: Implements panic hooks to ensure your terminal is always restored to its original state.

## Installation

Ensure you have the Rust toolchain installed, then:

```bash
# Build the release binary
cargo build --release

# The binary is now available at
./target/release/hale
```

## Usage

### Terminal User Interface (Interactive)
The default mode provides a full-screen dashboard for continuous monitoring.

```bash
hale
```
*   **Top Banner**: Real-time status, average latency, and uptime timer.
*   **Targets**: Live latency and status history for each of the 6 providers.
*   **Overview**: 5m and 1h sliding windows of your overall connection health.
*   **Exit**: Press `q`, `ESC`, or `Ctrl+C`.

### Command Line Interface (One-shot)
Ideal for quick checks or integration into other tools and scripts.

```bash
# Basic check
hale --check

# Verbose output with timestamps
hale --check --verbose

# Machine-readable JSON output
hale --check --format json
```

### Exit Codes
The CLI mode returns standard Unix exit codes:
*   `0`: Connection is **OK**.
*   `1`: Connection is **Slow** or **Disconnected**.

## Architecture & Tech Stack

### Technology Stack
*   **Language**: Rust (2021 Edition)
*   **Async Runtime**: `tokio` (v1) for concurrent non-blocking probing.
*   **TUI Framework**: `ratatui` with `crossterm` backend.
*   **CLI Parsing**: `clap` (v4) with derive macros.

### Module Structure
*   **Monitor**: Handles concurrent TCP (port 443) handshake probing.
*   **Analysis**: Aggregates raw data and applies majority voting for stability.
*   **UI**: Renders the interactive TUI and formatted CLI outputs.
*   **Utils**: Provides session logging with automatic downtime calculation.

### Monitoring Targets
Hale monitors the entry points of the world's most critical internet infrastructure:
1.  **Google**: `8.8.8.8`
2.  **AWS**: `s3.amazonaws.com`
3.  **Azure**: `portal.azure.com`
4.  **Cloudflare**: `1.1.1.1`
5.  **Quad9**: `9.9.9.9`
6.  **OpenDNS**: `208.67.222.222`

### Status Thresholds
*   **OK**: Latency < 100ms.
*   **Slow**: Latency 100-300ms.
*   **Disconnected**: Latency > 300ms or majority (4/6) timeout.

## Logging
Hale automatically creates a detailed session log at:
`/tmp/hale-{timestamp}.log`

This log records every status change and precisely calculates how long each network incident lasted.

## Testing
Hale includes a comprehensive suite of unit tests covering its analysis engine and probing logic.

```bash
cargo test
```

## License
This project is licensed under the MIT License - see the LICENSE file for details.
