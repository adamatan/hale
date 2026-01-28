use default_net;
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
}

impl NetworkInfo {
    pub fn new() -> Self {
        Self {
            public_ipv4: None,
            public_ipv6: None,
            interface_name: None,
            interface_type: None,
            local_ip: None,
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
            Some(parts[1].trim().to_string())
        } else {
            None
        }
    };

    // Timeout after 2 seconds
    timeout(Duration::from_secs(2), fetch_task)
        .await
        .unwrap_or(None)
}

async fn get_local_interface_info() -> (Option<String>, Option<String>, Option<String>) {
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
            
            (name, type_str, local_ip)
        } else {
            (None, None, None)
        }
    })
    .await;

    result.unwrap_or((None, None, None))
}

pub async fn refresh_network_info() -> NetworkInfo {
    let (ipv4, ipv6, (if_name, if_type, local_ip)) = tokio::join!(
        fetch_public_ip("api.ipify.org"),
        fetch_public_ip("api6.ipify.org"),
        get_local_interface_info()
    );

    NetworkInfo {
        public_ipv4: ipv4,
        public_ipv6: ipv6,
        interface_name: if_name,
        interface_type: if_type,
        local_ip,
    }
}
