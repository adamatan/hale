# Improved Top Bar PRD

## 1. Overview
The goal is to redesign the top section of the `hale` TUI to provide a richer, multi-line dashboard that displays real-time connection status, historical latency trends, and reliability metrics at a glance.

## 2. Layout & UI
The top bar will be expanded into a dashboard header with 3 distinct columns.

### Height
- **Total Height**: 5-6 lines (Title + 4 data rows).

### Columns

#### Column 1: Status (Left)
Focuses on the immediate health of the connection.
- **Line 1**: Overall Status (e.g., "✓ ONLINE", "⚠ SLOW", "✗ OFFLINE") with color coding.
- **Line 2**: Target Health (e.g., "Targets: 6/6 Reachable").
- **Line 3**: Current Packet Loss % (e.g., "Loss: 0%").
- **Line 4**: (Optional) Current Jitter or other immediate metric.

#### Column 2: Latency Stats (Middle)
Focuses on latency trends over time to identify degradation.
- **Line 1**: **Current**: XX ms
- **Line 2**: **1m Avg**: XX ms
- **Line 3**: **5m Avg**: XX ms
- **Line 4**: **15m Avg**: XX ms

#### Column 3: Time & Reliability (Right)
Focuses on session duration and stability over time.
- **Line 1**: **Session**: HH:MM:SS (Total running time)
- **Line 2**: **Since Incident**: HH:MM:SS (Time since last disconnection/slowdown)
- **Line 3**: **Uptime (1m)**: 99.9%
- **Line 4**: **Uptime (5m)**: 99.9%
- **Line 5**: **Uptime (15m)**: 99.9%

## 3. Functional Requirements

### Data Processing
The application must calculate rolling statistics based on the `history` buffer (`VecDeque<ProbeRound>`).
Given `PROBE_INTERVAL_MS = 500`:
- **1 minute** = 120 samples
- **5 minutes** = 600 samples
- **15 minutes** = 1800 samples

### Metrics
1.  **Latency Average**: Sum of average latency of successful probes in the window / count of successful rounds.
2.  **Uptime %**: (Number of "OK" rounds in window / Total rounds in window) * 100.
    - "OK" round = Sufficient successful probes and latency within acceptable limits.

## 4. Implementation Plan
1.  **Backend (`src/ui/tui.rs` or `src/analysis/`)**: Implement helper functions to calculate stats for a given time window (1m, 5m, 15m) from `state.history`.
2.  **Frontend (`src/ui/tui.rs`)**:
    - Modify `render_status_banner` to split into 3 vertical chunks.
    - Create detailed text generation for each column.
    - Adjust layout constraints to accommodate the taller header.

## 5. Acceptance Criteria
- [ ] Top bar displays 3 columns as specified.
- [ ] Latency values for 1m, 5m, 15m are accurate and update in real-time.
- [ ] Uptime percentages for 1m, 5m, 15m are accurate.
- [ ] Layout handles window resizing gracefully (truncating or hiding details if too small).
