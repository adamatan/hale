/// Network probe targets (public DNS servers and cloud endpoints)
pub const TARGETS: [&str; 6] = [
    "8.8.8.8",
    "s3.amazonaws.com",
    "portal.azure.com",
    "1.1.1.1",
    "9.9.9.9",
    "208.67.222.222",
];

/// Target labels for display
pub const TARGET_LABELS: [&str; 6] = ["Google", "AWS", "Azure", "Cloudflare", "Quad9", "OpenDNS"];

/// Target port for TCP connections (HTTPS)
pub const TARGET_PORT: u16 = 443;

/// Timeout for each probe attempt in milliseconds
pub const PROBE_TIMEOUT_MS: u64 = 1000;

/// Interval between probe rounds in milliseconds (0.5 seconds for visualization)
pub const PROBE_INTERVAL_MS: u64 = 500;

/// Latency threshold for "OK" status (milliseconds)
pub const LATENCY_OK_THRESHOLD_MS: f64 = 100.0;

/// Latency threshold for "Slow" status (milliseconds)
pub const LATENCY_SLOW_THRESHOLD_MS: f64 = 300.0;

/// Number of historical probe results to retain (1 hour at 500ms intervals)
pub const HISTORY_WINDOW_SIZE: usize = 7200;

/// Number of targets that must fail to count as packet loss
pub const MAJORITY_THRESHOLD: usize = 4;
