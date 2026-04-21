use default_net;
use std::process::Command;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

#[derive(Debug, Clone)]
pub struct NetworkInfo {
    pub public_ipv4: Option<String>,
    pub public_ipv6: Option<String>,
    pub interface_name: Option<String>,
    pub interface_type: Option<String>,
    pub local_ip: Option<String>,
    pub wifi_ssid: Option<String>,
    pub signal_strength: Option<i32>, // RSSI in dBm
    pub signal_noise: Option<i32>,    // Noise in dBm
    pub country: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub org: Option<String>,
    pub asn: Option<String>,
}

async fn fetch_public_ip(service_host: &str) -> Option<String> {
    let fetch_task = async {
        let addr = format!("{}:80", service_host);
        let mut stream = TcpStream::connect(&addr).await.ok()?;

        let request = format!(
            "GET / HTTP/1.0\r\nHost: {}\r\nUser-Agent: hale\r\n\r\n",
            service_host
        );
        stream.write_all(request.as_bytes()).await.ok()?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await.ok()?;

        let parts: Vec<&str> = response.split("\r\n\r\n").collect();
        if parts.len() > 1 {
            let body = parts[1].trim();
            // Validate that the body is a valid IP address
            if body.parse::<std::net::IpAddr>().is_ok() {
                Some(body.to_string())
            } else {
                None
            }
        } else {
            None
        }
    };

    // Timeout after 2 seconds
    timeout(Duration::from_secs(2), fetch_task)
        .await
        .unwrap_or(None)
}

#[cfg(target_os = "macos")]
fn get_wifi_info(_interface_name: &str) -> (Option<String>, Option<i32>, Option<i32>) {
    // 1. Get SSID using networksetup (faster/simpler for just name)
    let ssid = {
        let output = Command::new("networksetup")
            .args(["-getairportnetwork", _interface_name])
            .output()
            .ok();

        output.and_then(|o| {
            let stdout = String::from_utf8_lossy(&o.stdout);
            if stdout.contains("Current Wi-Fi Network:") {
                stdout.split(": ").nth(1).map(|s| s.trim().to_string())
            } else {
                None
            }
        })
    };

    // 2. Get Signal/Noise using system_profiler
    // We always check this because networksetup can sometimes fail to report the SSID
    // even when connected (e.g. "You are not associated with an AirPort network").
    let (signal, noise) = {
        let output = Command::new("system_profiler")
            .arg("SPAirPortDataType")
            .output()
            .ok();

        if let Some(o) = output {
            let stdout = String::from_utf8_lossy(&o.stdout);
            // Look for "Signal / Noise: -XX dBm / -XX dBm"
            if let Some(line) = stdout.lines().find(|l| l.contains("Signal / Noise:")) {
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() > 1 {
                    let values: Vec<&str> = parts[1].split('/').collect();
                    if values.len() == 2 {
                        let parse_dbm = |s: &str| -> Option<i32> {
                            s.split_whitespace().next()?.parse::<i32>().ok()
                        };
                        (parse_dbm(values[0]), parse_dbm(values[1]))
                    } else {
                        (None, None)
                    }
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    };

    (ssid, signal, noise)
}

#[cfg(target_os = "linux")]
fn get_wifi_info(_interface_name: &str) -> (Option<String>, Option<i32>, Option<i32>) {
    // 1. Get SSID
    let ssid = {
        let output = Command::new("iwgetid").arg("-r").output().ok();
        output.and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if !s.is_empty() {
                Some(s)
            } else {
                None
            }
        })
    };

    // 2. Get Signal from /proc/net/wireless
    // Format: inter-| sta-|   quality        |   discarded packets               | missed | WE
    //  face | tus | link level noise |  nwid  crypt   frag  retry   misc | beacon | 22
    //   wlan0: 0000   50.  -60.  -256        0      0      0      0      0        0
    let (signal, noise) = if ssid.is_some() {
        if let Ok(content) = std::fs::read_to_string("/proc/net/wireless") {
            // Find line with interface name
            if let Some(line) = content.lines().find(|l| l.contains(_interface_name)) {
                let fields: Vec<&str> = line.split_whitespace().collect();
                // fields[0] is "wlan0:", fields[1] is status
                // fields[2] is link quality, fields[3] is level (signal), fields[4] is noise
                // Note: format can vary slightly by driver, but level is usually 3rd or 4th col
                // In the example above: col 0=face, 1=status, 2=link, 3=level, 4=noise
                let parse_val = |s: &str| s.trim_matches('.').parse::<i32>().ok();

                // Try standard positions
                let level = fields.get(3).and_then(|s| parse_val(s));
                let noise_val = fields.get(4).and_then(|s| parse_val(s));

                (level, noise_val)
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    } else {
        (None, None)
    };

    (ssid, signal, noise)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn get_wifi_info(_interface_name: &str) -> (Option<String>, Option<i32>, Option<i32>) {
    (None, None, None)
}

async fn get_local_interface_info() -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<i32>,
    Option<i32>,
) {
    // Spawn blocking task for default-net
    let result = tokio::task::spawn_blocking(|| {
        if let Ok(interface) = default_net::get_default_interface() {
            let name = Some(interface.name.clone());
            let type_str = Some(format!("{:?}", interface.if_type));

            // Try IPv4 first, then IPv6
            let local_ip = if !interface.ipv4.is_empty() {
                Some(interface.ipv4[0].addr.to_string())
            } else if !interface.ipv6.is_empty() {
                Some(interface.ipv6[0].addr.to_string())
            } else {
                None
            };

            // Get SSID and Signal if wireless (or always on macOS to handle VPN/utun)
            #[cfg(target_os = "macos")]
            let (ssid, signal, noise) = get_wifi_info(&interface.name);

            #[cfg(not(target_os = "macos"))]
            let (ssid, signal, noise) = if format!("{:?}", interface.if_type).contains("Wireless") {
                get_wifi_info(&interface.name)
            } else {
                (None, None, None)
            };

            (name, type_str, local_ip, ssid, signal, noise)
        } else {
            (None, None, None, None, None, None)
        }
    })
    .await;

    result.unwrap_or((None, None, None, None, None, None))
}

async fn fetch_geo_info() -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let fetch_task = async {
        let host = "ip-api.com";
        let addr = format!("{}:80", host);
        let mut stream = TcpStream::connect(&addr).await.ok()?;

        let request = format!(
            "GET /line/?fields=country,city,isp,org,as HTTP/1.0\r\nHost: {}\r\nUser-Agent: hale\r\n\r\n",
            host
        );
        stream.write_all(request.as_bytes()).await.ok()?;

        let mut response = String::new();
        stream.read_to_string(&mut response).await.ok()?;

        // Split by double CRLF to get body
        let parts: Vec<&str> = response.split("\r\n\r\n").collect();
        if parts.len() > 1 {
            let body = parts[1];
            let lines: Vec<&str> = body.trim().split('\n').collect();

            // Map lines to fields: country, city, isp, org, asn
            let country = lines.first().map(|s| s.trim().to_string());
            let city = lines.get(1).map(|s| s.trim().to_string());
            let isp = lines.get(2).map(|s| s.trim().to_string());
            let org = lines.get(3).map(|s| s.trim().to_string());
            let asn = lines.get(4).map(|s| s.trim().to_string());

            Some((country, city, isp, org, asn))
        } else {
            None
        }
    };

    // Timeout after 2 seconds
    timeout(Duration::from_secs(2), fetch_task)
        .await
        .unwrap_or(None)
        .unwrap_or((None, None, None, None, None))
}

pub async fn refresh_network_info() -> NetworkInfo {
    let (
        ipv4,
        ipv6,
        (if_name, if_type, local_ip, wifi_ssid, signal_strength, signal_noise),
        geo_info,
    ) = tokio::join!(
        fetch_public_ip("api.ipify.org"),
        fetch_public_ip("api6.ipify.org"),
        get_local_interface_info(),
        fetch_geo_info()
    );

    let (country, city, isp, org, asn) = geo_info;

    NetworkInfo {
        public_ipv4: ipv4,
        public_ipv6: ipv6,
        interface_name: if_name,
        interface_type: if_type,
        local_ip,
        wifi_ssid,
        signal_strength,
        signal_noise,
        country,
        city,
        isp,
        org,
        asn,
    }
}
