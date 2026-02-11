use rand::Rng;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::time::Duration;

/// Default STUN server list for TCP mode
pub fn default_stun_servers_tcp() -> Vec<(String, u16)> {
    vec![
        ("fwa.lifesizecloud.com".into(), 3478),
        ("global.turn.twilio.com".into(), 3478),
        ("turn.cloudflare.com".into(), 3478),
        ("stun.isp.net.au".into(), 3478),
        ("stun.nextcloud.com".into(), 3478),
        ("stun.freeswitch.org".into(), 3478),
        ("stun.voip.blackberry.com".into(), 3478),
        ("stunserver.stunprotocol.org".into(), 3478),
        ("stun.sipnet.com".into(), 3478),
        ("stun.radiojar.com".into(), 3478),
        ("stun.sonetel.com".into(), 3478),
        ("stun.telnyx.com".into(), 3478),
        ("turn.cloud-rtc.com".into(), 80),
    ]
}

/// Default STUN server list for UDP mode
pub fn default_stun_servers_udp() -> Vec<(String, u16)> {
    vec![
        ("stun.miwifi.com".into(), 3478),
        ("stun.chat.bilibili.com".into(), 3478),
        ("stun.hitv.com".into(), 3478),
        ("stun.cdnbye.com".into(), 3478),
        ("stun.douyucdn.cn".into(), 18000),
        ("fwa.lifesizecloud.com".into(), 3478),
        ("global.turn.twilio.com".into(), 3478),
        ("turn.cloudflare.com".into(), 3478),
        ("stun.isp.net.au".into(), 3478),
        ("stun.nextcloud.com".into(), 3478),
        ("stun.freeswitch.org".into(), 3478),
        ("stun.voip.blackberry.com".into(), 3478),
        ("stunserver.stunprotocol.org".into(), 3478),
        ("stun.sipnet.com".into(), 3478),
        ("stun.radiojar.com".into(), 3478),
        ("stun.sonetel.com".into(), 3478),
        ("stun.telnyx.com".into(), 3478),
    ]
}

#[derive(Debug, Clone)]
pub struct StunResult {
    pub inner_addr: SocketAddr,
    pub outer_addr: SocketAddr,
}

pub struct StunClient {
    pub stun_server_list: Vec<(String, u16)>,
    pub source_host: String,
    pub source_port: u16,
    pub udp: bool,
}

impl StunClient {
    pub fn new(
        stun_server_list: Vec<(String, u16)>,
        source_host: &str,
        source_port: u16,
        udp: bool,
    ) -> Self {
        StunClient {
            stun_server_list,
            source_host: source_host.to_string(),
            source_port,
            udp,
        }
    }

    /// Build the 20-byte STUN Binding Request
    fn build_stun_request() -> Vec<u8> {
        let mut rng = rand::thread_rng();
        let mut buf = Vec::with_capacity(20);
        // Message Type: Binding Request (0x0001), Length: 0
        buf.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        // Magic Cookie: 0x2112A442
        buf.extend_from_slice(&0x2112A442u32.to_be_bytes());
        // Transaction ID (12 bytes: we use 4 fixed + 4 random + 4 random)
        buf.extend_from_slice(&0x4E415452u32.to_be_bytes());
        buf.extend_from_slice(&rng.gen::<u32>().to_be_bytes());
        buf.extend_from_slice(&rng.gen::<u32>().to_be_bytes());
        buf
    }

    /// Parse the STUN response to extract the mapped address
    fn parse_stun_response(data: &[u8]) -> Result<SocketAddr, String> {
        if data.len() < 20 {
            return Err("STUN response too short".into());
        }
        let mut payload = &data[20..];
        while payload.len() >= 4 {
            let attr_type = u16::from_be_bytes([payload[0], payload[1]]);
            let attr_len = u16::from_be_bytes([payload[2], payload[3]]) as usize;
            if attr_len + 4 > payload.len() {
                break;
            }
            // MAPPED-ADDRESS (0x0001) or XOR-MAPPED-ADDRESS (0x0020)
            if (attr_type == 0x0001 || attr_type == 0x0020) && attr_len >= 8 {
                let port = u16::from_be_bytes([payload[6], payload[7]]);
                let ip = u32::from_be_bytes([payload[8], payload[9], payload[10], payload[11]]);
                let (port, ip) = if attr_type == 0x0020 {
                    (port ^ 0x2112, ip ^ 0x2112A442)
                } else {
                    (port, ip)
                };
                let ip_addr = std::net::Ipv4Addr::from(ip);
                return Ok(SocketAddr::new(ip_addr.into(), port));
            }
            payload = &payload[4 + attr_len..];
        }
        Err("No mapped address in STUN response".into())
    }

    /// Get mapping via TCP STUN
    fn get_mapping_tcp(&mut self, server: &(String, u16)) -> Result<StunResult, String> {
        let addr_str = format!("{}:{}", server.0, server.1);
        let addrs: Vec<SocketAddr> = addr_str
            .to_socket_addrs()
            .map_err(|e| format!("DNS resolve failed for {}: {}", addr_str, e))?
            .collect();
        if addrs.is_empty() {
            return Err(format!("No address found for {}", addr_str));
        }

        let bind_addr: SocketAddr = format!("{}:{}", self.source_host, self.source_port)
            .parse()
            .map_err(|e| format!("Invalid bind address: {}", e))?;

        let socket = socket2::Socket::new(
            socket2::Domain::IPV4,
            socket2::Type::STREAM,
            Some(socket2::Protocol::TCP),
        )
        .map_err(|e| format!("Socket creation failed: {}", e))?;

        socket.set_reuse_address(true).ok();
        socket
            .bind(&bind_addr.into())
            .map_err(|e| format!("Bind to {} failed: {}", bind_addr, e))?;
        socket
            .connect_timeout(&addrs[0].into(), Duration::from_secs(3))
            .map_err(|e| format!("Connect to {} failed: {}", addrs[0], e))?;

        let mut stream: TcpStream = socket.into();
        stream.set_read_timeout(Some(Duration::from_secs(3))).ok();

        let inner_addr = stream
            .local_addr()
            .map_err(|e| format!("Get local addr failed: {}", e))?;
        self.source_host = inner_addr.ip().to_string();
        self.source_port = inner_addr.port();

        let req = Self::build_stun_request();
        stream
            .write_all(&req)
            .map_err(|e| format!("Send STUN request failed: {}", e))?;

        let mut buf = vec![0u8; 1500];
        let n = stream
            .read(&mut buf)
            .map_err(|e| format!("Read STUN response failed: {}", e))?;

        let outer_addr = Self::parse_stun_response(&buf[..n])?;

        Ok(StunResult {
            inner_addr,
            outer_addr,
        })
    }

    /// Get mapping via UDP STUN
    fn get_mapping_udp(&mut self, server: &(String, u16)) -> Result<StunResult, String> {
        let addr_str = format!("{}:{}", server.0, server.1);
        let addrs: Vec<SocketAddr> = addr_str
            .to_socket_addrs()
            .map_err(|e| format!("DNS resolve failed for {}: {}", addr_str, e))?
            .collect();
        if addrs.is_empty() {
            return Err(format!("No address found for {}", addr_str));
        }

        let bind_addr = format!("{}:{}", self.source_host, self.source_port);
        let socket = UdpSocket::bind(&bind_addr)
            .map_err(|e| format!("Bind to {} failed: {}", bind_addr, e))?;
        socket.set_read_timeout(Some(Duration::from_secs(3))).ok();
        socket
            .connect(addrs[0])
            .map_err(|e| format!("Connect to {} failed: {}", addrs[0], e))?;

        let inner_addr = socket
            .local_addr()
            .map_err(|e| format!("Get local addr failed: {}", e))?;
        self.source_host = inner_addr.ip().to_string();
        self.source_port = inner_addr.port();

        let req = Self::build_stun_request();
        socket
            .send(&req)
            .map_err(|e| format!("Send STUN request failed: {}", e))?;

        let mut buf = vec![0u8; 1500];
        let n = socket
            .recv(&mut buf)
            .map_err(|e| format!("Read STUN response failed: {}", e))?;

        let outer_addr = Self::parse_stun_response(&buf[..n])?;

        Ok(StunResult {
            inner_addr,
            outer_addr,
        })
    }

    /// Try each STUN server until one succeeds
    pub fn get_mapping(&mut self) -> Result<StunResult, String> {
        let servers = self.stun_server_list.clone();
        let mut last_err = String::new();
        for server in &servers {
            let result = if self.udp {
                self.get_mapping_udp(server)
            } else {
                self.get_mapping_tcp(server)
            };
            match result {
                Ok(r) => {
                    log::info!(
                        "STUN: Got mapping {} -> {} from {}:{}",
                        r.inner_addr,
                        r.outer_addr,
                        server.0,
                        server.1
                    );
                    return Ok(r);
                }
                Err(e) => {
                    log::warn!("STUN server {}:{} unavailable: {}", server.0, server.1, e);
                    last_err = e;
                }
            }
        }
        Err(format!(
            "All STUN servers unavailable. Last error: {}",
            last_err
        ))
    }
}
