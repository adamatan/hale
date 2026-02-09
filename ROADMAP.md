# Hale Project Roadmap

This document outlines completed features and future development plans for Hale, the network monitoring tool.

## Completed Features

### Core Monitoring
- ✅ Real-time network quality monitoring with visual TUI
- ✅ Multi-target probing (Google, AWS, Azure, Cloudflare, Quad9, OpenDNS)
- ✅ Connection status detection (OK/Slow/Disconnected)
- ✅ Latency tracking and jitter calculation
- ✅ Packet loss detection
- ✅ Network interface information display

### Reporting & Output
- ✅ Detailed session summary reports with scoring
  - Session score calculation (Perfect/OK/Poor/Unacceptable/NoConnection)
  - Uptime statistics with absolute time breakdown
  - Incident timeline with timestamps and durations
  - Automatic report generation to `/tmp/hale-summary-{timestamp}.md` (Markdown format)
  - Console display (detailed by default, short with `--short` flag)
- ✅ Session logging to `/tmp/hale-{timestamp}.log`
- ✅ Improve log and summary filename readability and uniqueness (`hale-YYYY-MM-DD-HH-MM-SS-RAND.log`)
- ✅ Short summary output format
- ✅ Meaningful exit codes based on session quality

### User Experience
- ✅ Interactive TUI with real-time updates
- ✅ Visual sparkline graphs for latency history
- ✅ Color-coded status indicators
- ✅ Session duration tracking
- ✅ Time since last incident display
- ✅ Graceful keyboard interrupt handling (Ctrl+C)

## Planned Features

### Enhanced Reporting
- [ ] JSON export format for session data
  - Machine-readable output for integration with other tools
  - Structured incident data export
  - API-friendly format
- [ ] Multi-session analytics
  - Aggregate statistics across multiple monitoring sessions
  - Trend analysis over time
  - Historical comparison reports
- [ ] Custom report templates
  - User-definable report formats
  - Configurable incident thresholds
  - Customizable scoring criteria

### Configuration & Customization
- [ ] Configuration file support
  - User-defined probe targets
  - Adjustable latency thresholds
  - Custom probe intervals
- [ ] Command-line options for thresholds
  - Override default OK/Slow latency limits
  - Configure packet loss sensitivity
  - Adjust history window size

### Advanced Monitoring
- [ ] Bandwidth testing
  - Upload/download speed measurements
  - Throughput monitoring
- [ ] DNS resolution monitoring
  - DNS query latency tracking
  - DNS failure detection
- [ ] Geographic latency analysis
  - Region-specific performance metrics
  - Multi-region target groups

### Integration & Export
- [ ] Webhook notifications
  - Alert on disconnection events
  - Configurable notification triggers
- [ ] Metrics export
  - Prometheus metrics endpoint
  - StatsD integration
  - InfluxDB support
- [ ] Cloud integration
  - AWS CloudWatch metrics
  - Azure Monitor integration

### Platform Support
- [ ] Windows native support
  - Windows-specific network APIs
  - PowerShell integration
- [ ] Docker container image
  - Pre-built container for easy deployment
  - Multi-architecture support (amd64, arm64)

## Contributing

We welcome contributions! If you're interested in working on any of these features or have ideas for new ones, please:

1. Check existing issues and pull requests
2. Open an issue to discuss major changes
3. Submit a pull request with your implementation

## Version Planning

- **v0.2.x**: Enhanced reporting and JSON export
- **v0.3.x**: Configuration file support and customization
- **v0.4.x**: Advanced monitoring features
- **v0.5.x**: Integration and export capabilities
- **v1.0.0**: Stable release with comprehensive feature set

---

For more information, see the [README](README.md) or visit [github.com/adam-matan/hale](https://github.com/adam-matan/hale).
