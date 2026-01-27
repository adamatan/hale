# Technical Specification: Improved Top Bar

## 1. Overview
This document outlines the technical changes required to implement the "Improved Top Bar" feature in `hale`. The goal is to replace the current single-line status bar with a 3-column dashboard displaying real-time status, latency trends, and reliability metrics.

## 2. Architecture Changes

### 2.1 Data Structures
No changes to `src/monitor/mod.rs` structs are strictly necessary, but we will add helper methods to `TuiState` in `src/ui/tui.rs` to calculate rolling statistics from the existing `history` buffer.

### 2.2 New Methods in `TuiState`

We will implement a helper method to calculate stats for a given duration.

```rust
impl TuiState {
    /// Calculate average latency and uptime percentage for the given duration
    /// Returns (avg_latency_ms, uptime_pct)
    pub fn get_window_stats(&self, duration: chrono::Duration) -> (Option<f64>, f64) {
        // Implementation details:
        // 1. Iterate backwards through self.history
        // 2. Filter rounds within the duration window
        // 3. Sum valid latencies and count valid rounds
        // 4. Count "Uptime" rounds (status != Disconnected)
        // 5. Return aggregates
    }
}
```

### 2.3 UI Layout Updates

The `render_status_banner` function in `src/ui/tui.rs` will be completely rewritten.

**Current Layout:**
- Horizontal chunks: [Latency, Status, Timer]
- Height: 3 lines (Fixed)

**New Layout:**
- Height: 6 lines (Title + 4-5 data rows)
- Horizontal chunks:
    1.  **Status (Left)**: Immediate health (Status, Reachable targets, Packet Loss).
    2.  **Latency Stats (Middle)**: Rolling averages (Current, 1m, 5m, 15m).
    3.  **Reliability (Right)**: Session timers and Uptime % (1m, 5m, 15m).

## 3. Implementation Details

### 3.1 `src/ui/tui.rs`

#### `TuiState::get_window_stats` logic
- **Input**: `duration` (e.g., 1 minute).
- **Process**:
    - `now = Utc::now()`
    - `cutoff = now - duration`
    - Filter `self.history`: `round.timestamp >= cutoff`
    - **Latency**:
        - Collect all successful `PingResult` latencies in these rounds.
        - Average them.
    - **Uptime**:
        - Iterate rounds.
        - Determine status of each round (similar to `render_site_row` logic or `NetworkStats` logic).
        - `if status != Disconnected { up_count += 1 }`
        - `uptime = up_count / total_rounds * 100.0`

#### `render_status_banner` logic
1.  **Calculate Metrics**:
    - Call `get_window_stats` for 1m, 5m, 15m.
    - Get current stats from `state.stats`.
    - Get latest round info for "Targets Reachable".

2.  **Left Column (Status)**:
    - Status Line: Big, Bold, Colored (Green/Yellow/Red).
    - Targets: Count `success` in `state.history.back().results`.
    - Loss: `state.stats.loss_pct`.

3.  **Middle Column (Latency)**:
    - Current: `state.stats.avg_latency_ms`.
    - 1m/5m/15m: Display values returned from helper.
    - Use "N/A" if not enough data (optional, or just show partial data).

4.  **Right Column (Reliability)**:
    - Session Time: `state.format_running_time()`.
    - Since Incident: `state.time_since_last_incident()`.
    - Uptime 1m/5m/15m: Display values.

### 3.2 Constants
- Ensure `HISTORY_WINDOW_SIZE` is sufficient (it is 7200 samples = 1h).

## 4. Migration Plan
1.  **Update `TuiState`**: Add `get_window_stats` method.
2.  **Update `ui` function**: Change constraint for top banner from `Length(3)` to `Length(6)`.
3.  **Refactor `render_status_banner`**: Implement the new 3-column layout.

## 5. Testing Strategy
- **Unit Tests**: Test `get_window_stats` with mocked history data to ensure math is correct.
- **Visual Verification**: Run the TUI and verify:
    - Layout matches design.
    - Values update in real-time.
    - Resizing window doesn't crash (Ratatui handles this, but check for content clipping).
