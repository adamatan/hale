# Implementation Plan - Improved Top Bar

## 1. Implementation Overview

**Goal**: Replace the current single-line status bar with a comprehensive 3-column dashboard displaying status, latency trends, and reliability metrics.

**Key Components**:
1.  **Core Logic**: New statistical helper methods in `TuiState`.
2.  **UI Layout**: Expanded top banner area (Height: 6 lines).
3.  **Visualization**: Detailed rendering logic for Status, Latency, and Reliability columns.

## 2. Implementation Phases

### Phase 1: Core Logic Implementation
Implement the statistical analysis methods required to feed the UI.

#### Task 1.1: Implement `get_window_stats`
**File**: `src/ui/tui.rs`
**Objective**: Add a method to `TuiState` that calculates average latency and uptime percentage for a specific time window.
**Logic**:
- Iterate backwards through `self.history`.
- Filter rounds within `now - duration`.
- **Latency**: Average of all successful probe latencies in the window.
- **Uptime**: (Count of rounds != Disconnected) / Total Rounds * 100.0.
- **Signature**: `pub fn get_window_stats(&self, duration: chrono::Duration) -> (Option<f64>, f64)`

#### Task 1.2: Add Unit Tests
**File**: `src/ui/tui.rs` (mod tests)
**Objective**: Verify `get_window_stats` correctness.
**Tests**:
- Test with empty history.
- Test with mixed success/fail rounds.
- Test with 100% packet loss.
- Test 1m, 5m window calculations.

### Phase 2: UI Layout Updates
Adjust the main TUI grid to accommodate the new dashboard.

#### Task 2.1: Resize Top Banner
**File**: `src/ui/tui.rs` (`ui` function)
**Action**: Change the first constraint of the main layout from `Constraint::Length(3)` to `Constraint::Length(6)`.

#### Task 2.2: Scaffolding for 3 Columns
**File**: `src/ui/tui.rs` (`render_status_banner` function)
**Action**:
- Clear existing content of `render_status_banner`.
- Create a horizontal layout splitting the area into 3 chunks:
    - Column 1: 33% (Status)
    - Column 2: 34% (Latency)
    - Column 3: 33% (Reliability)
- Render temporary borders for each chunk to verify alignment.

### Phase 3: Column Rendering
Implement the content for each of the three dashboard columns.

#### Task 3.1: Render Status Column (Left)
**File**: `src/ui/tui.rs`
**Content**:
- **Line 1**: "ONLINE", "SLOW", or "OFFLINE" (Bold, colored background/text).
- **Line 2**: "Targets: X/Y Reachable" (calculated from latest round).
- **Line 3**: "Loss: X%" (from `state.stats`).
- **Details**: Use `Paragraph` widgets. Center align the status.

#### Task 3.2: Render Latency Column (Middle)
**File**: `src/ui/tui.rs`
**Content**:
- **Header**: "Latency Stats"
- **Line 1**: "Current: XX ms"
- **Line 2**: "1m Avg:  XX ms" (via `get_window_stats`)
- **Line 3**: "5m Avg:  XX ms"
- **Line 4**: "15m Avg: XX ms"
- **Details**: Ensure alignment matches vertically. Use "N/A" if history is insufficient.

#### Task 3.3: Render Reliability Column (Right)
**File**: `src/ui/tui.rs`
**Content**:
- **Header**: "Reliability"
- **Line 1**: "Session: HH:MM:SS" (existing `format_running_time`)
- **Line 2**: "Since Incident: HH:MM:SS" (existing `time_since_last_incident`)
- **Line 3**: "Uptime (1m): XX.X%"
- **Line 4**: "Uptime (5m): XX.X%"
- **Line 5**: "Uptime (15m): XX.X%"

### Phase 4: Final Polish
Refine the visual presentation and clean up code.

#### Task 4.1: Styling and Colors
- Ensure consistent color coding (Green = Good, Yellow = Slow, Red = Bad).
- Add Borders::ALL to the 3 main blocks.
- Check for text truncation on small screens.

#### Task 4.2: Code Cleanup
- Remove any unused helper variables from the old banner implementation.
- Run `cargo fmt` and `cargo clippy`.

## 3. Testing Strategy

### Unit Testing
- Run `cargo test` to verify `get_window_stats` logic.

### Visual Verification
- **Run the app**: `cargo run`
- **Check Layout**: Verify the top bar is taller and split into 3 columns.
- **Check Data**:
    - Disconnect network to see "Status" change to OFFLINE.
    - Wait 1 minute to see "1m Avg" and "Uptime (1m)" populate.
    - Verify "Targets" count matches your config.

## 4. Dependencies
- No new crates required.
- Depends on `chrono` (already in use).
