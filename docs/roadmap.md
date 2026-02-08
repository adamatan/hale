# hale Roadmap

## High Priority

- [ ] **Agents Configuration (AGENTS.md)** - Add configuration file to instruct AI agents on desired workflows: testing requirements, architecture guidelines, git conventions (worktrees, branches), and working with PRs and GitHub Actions
- [x] **Update Agent Workflow** - Update AGENTS.md to include roadmap updates in PR and improve cleanup phase.
- [x] **Immediate Keyboard Handling** - Implement responsive keyboard input (especially for immediate exit) using raw mode and async polling as seen in gping. See `docs/immediate-keyboard-detection.md`.
- [ ] **Measurement Validation** - Add robust validations to ensure measurements are accurate and trustworthy (e.g., cross-check with system tools, sanity checks, outlier detection)
- [x] **Improve README** - Enhance with clearer value proposition (why use hale over ping/mtr) and more prominent quick start section
- [x] **Simplify Session Info** - Remove the session info box and replace with a single line showing how to quit
- [ ] **UX Restructure** - Reorganize TUI layout, improve color scheme, add compact mode, keyboard shortcuts, group related info, merge duplicate data in blocks, add keyboard shortcuts help section

## Medium Priority

- [x] **Exit Codes** - Implement meaningful exit codes: 0 for OK, 1 for no internet connection at all, 2 for disconnections detected
- [ ] **Installation Packages** - Homebrew, GitHub binary releases, improved cargo install documentation, apt/deb packages
- [ ] **Terminal Size Awareness** - Graceful degradation as terminal shrinks: status (always visible) → network stats → history → specific targets → aggregate. Minimum size = whatever can display status
- [ ] **Custom Probe Targets** - CLI flags `--add-targets/-t` to extend default providers, `--replace-targets/-T` to use only custom targets
- [ ] **Improved Logs and Reports** - Configurable log path, summary reports on exit, CSV/JSON export formats
- [ ] **Grievance Mode** - Generate copy-pasteable plain text paragraph with targets, test duration, average latency, downtime count, and average duration for ISP/network admin complaints
- [ ] **CLI Commands in Reports** - Include CLI commands in reports so users can replicate tests (e.g., how to run the latency test from the command line)
- [ ] **Help Screen (`?` key)** - htop-style help overlay showing all keyboard shortcuts and CLI commands users can run to validate the displayed stats
- [x] **Screenshot in README** - Add a screenshot of the TUI to the README for better first impressions
- [ ] **Local Network Health** - Show gateway reachability and latency separately from internet targets, distinguish LAN vs WAN issues

## Low Priority

- [ ] **Configurable Thresholds** - CLI flags for latency boundaries (OK/Slow), timeout duration, majority voting threshold, probe interval
- [ ] **License and Credits in CLI Help** - Add license information and credits/acknowledgements to the CLI help output
