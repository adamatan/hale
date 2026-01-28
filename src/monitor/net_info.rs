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
    pub country: Option<String>,
    pub city: Option<String>,
    pub isp: Option<String>,
    pub org: Option<String>,
    pub asn: Option<String>,
}

impl NetworkInfo {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            public_ipv4: None,
            public_ipv6: None,
            interface_name: None,
            interface_type: None,
            local_ip: None,
            wifi_ssid: None,
            country: None,
            city: None,
            isp: None,
            org: None,
            asn: None,
        }
    }
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

fn get_wifi_ssid(_interface_name: &str) -> Option<String> {
    // macOS implementation using networksetup
    #[cfg(target_os = "macos")]
    {
        let output = Command::new("networksetup")
            .args(&["-getairportnetwork", _interface_name])
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        // Output format: "Current Wi-Fi Network: MyWifiName\n"
        if stdout.contains("Current Wi-Fi Network:") {
            return stdout.split(": ").nth(1).map(|s| s.trim().to_string());
        }
    }

    // Linux implementation using iwgetid
    #[cfg(target_os = "linux")]
    {
        let output = Command::new("iwgetid").arg("-r").output().ok()?;
        let ssid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ssid.is_empty() {
            return Some(ssid);
        }
    }

    None
}

async fn get_local_interface_info() -> (
    Option<String>,
    Option<String>,
    Option<String>,
    Option<String>,
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

            // Get SSID if wireless
            let ssid = if format!("{:?}", interface.if_type).contains("Wireless") {
                get_wifi_ssid(&interface.name)
            } else {
                None
            };

            (name, type_str, local_ip, ssid)
        } else {
            (None, None, None, None)
        }
    })
    .await;

    result.unwrap_or((None, None, None, None))
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
            let country = lines.get(0).map(|s| s.trim().to_string());
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
    let (ipv4, ipv6, (if_name, if_type, local_ip, wifi_ssid), geo_info) = tokio::join!(
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
        country,
        city,
        isp,
        org,
        asn,
    }
}
