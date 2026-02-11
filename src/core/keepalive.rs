use rand::Rng;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, UdpSocket};
use std::time::Duration;

pub struct KeepAlive {
    host: String,
    port: u16,
    source_host: String,
    source_port: u16,
    udp: bool,
    tcp_stream: Option<TcpStream>,
    udp_socket: Option<UdpSocket>,
    reconn: bool,
}

impl KeepAlive {
    pub fn new(
        host: &str,
        port: u16,
        source_host: &str,
        source_port: u16,
        udp: bool,
    ) -> Self {
        KeepAlive {
            host: host.to_string(),
            port,
            source_host: source_host.to_string(),
            source_port,
            udp,
            tcp_stream: None,
            udp_socket: None,
            reconn: false,
        }
    }

    fn connect_tcp(&mut self) -> Result<(), String> {
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
            .map_err(|e| format!("Bind failed: {}", e))?;

        let target: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .or_else(|_| {
                use std::net::ToSocketAddrs;
                format!("{}:{}", self.host, self.port)
                    .to_socket_addrs()
                    .map_err(|e| format!("DNS resolve failed: {}", e))
                    .and_then(|mut addrs| {
                        addrs.next().ok_or_else(|| "No address found".to_string())
                    })
            })?;

        socket
            .connect_timeout(&target.into(), Duration::from_secs(3))
            .map_err(|e| format!("Connect to {}:{} failed: {}", self.host, self.port, e))?;

        let stream: TcpStream = socket.into();
        stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
        stream.set_nonblocking(false).ok();
        self.tcp_stream = Some(stream);

        if self.reconn {
            log::info!("keep-alive: connection restored");
        }
        self.reconn = false;
        Ok(())
    }

    fn connect_udp(&mut self) -> Result<(), String> {
        let bind_addr = format!("{}:{}", self.source_host, self.source_port);
        let socket =
            UdpSocket::bind(&bind_addr).map_err(|e| format!("Bind failed: {}", e))?;
        socket
            .set_read_timeout(Some(Duration::from_secs(3)))
            .ok();

        let target: SocketAddr = format!("{}:{}", self.host, self.port)
            .parse()
            .or_else(|_| {
                use std::net::ToSocketAddrs;
                format!("{}:{}", self.host, self.port)
                    .to_socket_addrs()
                    .map_err(|e| format!("DNS resolve failed: {}", e))
                    .and_then(|mut addrs| {
                        addrs.next().ok_or_else(|| "No address found".to_string())
                    })
            })?;

        socket
            .connect(target)
            .map_err(|e| format!("Connect failed: {}", e))?;
        self.udp_socket = Some(socket);
        self.reconn = false;
        Ok(())
    }

    pub fn keep_alive(&mut self) -> Result<(), String> {
        if self.udp {
            self.keep_alive_udp()
        } else {
            self.keep_alive_tcp()
        }
    }

    fn keep_alive_tcp(&mut self) -> Result<(), String> {
        if self.tcp_stream.is_none() {
            self.connect_tcp()?;
        }
        let stream = self.tcp_stream.as_mut().unwrap();

        let request = format!(
            "HEAD /natter-keep-alive HTTP/1.1\r\n\
             Host: {}\r\n\
             User-Agent: curl/8.0.0 (HolePatch)\r\n\
             Accept: */*\r\n\
             Connection: keep-alive\r\n\
             \r\n",
            self.host
        );
        stream
            .write_all(request.as_bytes())
            .map_err(|e| format!("Send keep-alive failed: {}", e))?;

        // Read response (drain buffer)
        let mut buf = vec![0u8; 4096];
        loop {
            match stream.read(&mut buf) {
                Ok(0) => return Err("Keep-alive server closed connection".into()),
                Ok(_) => {}
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                    break;
                }
                Err(e) => return Err(format!("Read keep-alive failed: {}", e)),
            }
        }
        log::debug!("keep-alive: OK");
        Ok(())
    }

    fn keep_alive_udp(&mut self) -> Result<(), String> {
        if self.udp_socket.is_none() {
            self.connect_udp()?;
        }
        let socket = self.udp_socket.as_ref().unwrap();

        // Send a DNS request as keep-alive
        let mut rng = rand::thread_rng();
        let mut req = Vec::new();
        req.extend_from_slice(&rng.gen::<u16>().to_be_bytes()); // Transaction ID
        req.extend_from_slice(&0x0100u16.to_be_bytes()); // Flags: standard query
        req.extend_from_slice(&0x0001u16.to_be_bytes()); // Questions: 1
        req.extend_from_slice(&[0u8; 6]); // Answer, Authority, Additional: 0
        // Query: keepalive.natter
        req.extend_from_slice(b"\x09keepalive\x06natter\x00");
        req.extend_from_slice(&0x0001u16.to_be_bytes()); // Type A
        req.extend_from_slice(&0x0001u16.to_be_bytes()); // Class IN

        socket
            .send(&req)
            .map_err(|e| format!("Send keep-alive failed: {}", e))?;

        let mut buf = vec![0u8; 1500];
        match socket.recv(&mut buf) {
            Ok(_) => {}
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                // timeout is ok for UDP
            }
            Err(e) => return Err(format!("UDP keep-alive recv failed: {}", e)),
        }
        log::debug!("keep-alive: OK");
        Ok(())
    }

    pub fn reset(&mut self) {
        self.tcp_stream = None;
        self.udp_socket = None;
        self.reconn = true;
    }
}
