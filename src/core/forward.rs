use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq)]
pub enum ForwardMethod {
    None,
    TestServer,
    Socket,
}

impl ForwardMethod {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" => Some(ForwardMethod::None),
            "test" => Some(ForwardMethod::TestServer),
            "socket" => Some(ForwardMethod::Socket),
            _ => None,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            ForwardMethod::None => "none",
            ForwardMethod::TestServer => "test",
            ForwardMethod::Socket => "socket",
        }
    }

    pub fn all() -> Vec<ForwardMethod> {
        vec![
            ForwardMethod::None,
            ForwardMethod::TestServer,
            ForwardMethod::Socket,
        ]
    }
}

pub struct Forwarder {
    method: ForwardMethod,
    running: Arc<AtomicBool>,
}

impl Forwarder {
    pub fn new(method: ForwardMethod) -> Self {
        Forwarder {
            method,
            running: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn start_forward(
        &mut self,
        ip: &str,
        port: u16,
        to_ip: &str,
        to_port: u16,
        udp: bool,
    ) -> Result<(), String> {
        match self.method {
            ForwardMethod::None => Ok(()),
            ForwardMethod::TestServer => {
                self.running.store(true, Ordering::SeqCst);
                self.start_test_server(ip, port, udp)
            }
            ForwardMethod::Socket => {
                self.running.store(true, Ordering::SeqCst);
                self.start_socket_forward(ip, port, to_ip, to_port, udp)
            }
        }
    }

    pub fn stop_forward(&mut self) {
        self.running.store(false, Ordering::SeqCst);
    }

    fn start_test_server(&self, _ip: &str, port: u16, udp: bool) -> Result<(), String> {
        let running = self.running.clone();
        if udp {
            let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))
                .map_err(|e| format!("Bind test server failed: {}", e))?;
            socket.set_read_timeout(Some(Duration::from_secs(1))).ok();
            thread::spawn(move || {
                let mut buf = [0u8; 8192];
                while running.load(Ordering::SeqCst) {
                    match socket.recv_from(&mut buf) {
                        Ok((_, addr)) => {
                            let _ = socket.send_to(b"It works! - HolePatch\r\n", addr);
                        }
                        Err(_) => continue,
                    }
                }
            });
        } else {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
                .map_err(|e| format!("Bind test server failed: {}", e))?;
            listener.set_nonblocking(true).ok();
            thread::spawn(move || {
                while running.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((mut conn, _)) => {
                            conn.set_read_timeout(Some(Duration::from_secs(3))).ok();
                            let mut buf = [0u8; 8192];
                            let _ = conn.read(&mut buf);
                            let content = "<html><body><h1>It works!</h1><hr/>HolePatch</body></html>";
                            let response = format!(
                                "HTTP/1.1 200 OK\r\n\
                                 Content-Type: text/html\r\n\
                                 Content-Length: {}\r\n\
                                 Connection: close\r\n\
                                 Server: HolePatch\r\n\
                                 \r\n\
                                 {}",
                                content.len(),
                                content
                            );
                            let _ = conn.write_all(response.as_bytes());
                            let _ = conn.shutdown(Shutdown::Both);
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            });
        }
        Ok(())
    }

    fn start_socket_forward(
        &self,
        _ip: &str,
        port: u16,
        to_ip: &str,
        to_port: u16,
        udp: bool,
    ) -> Result<(), String> {
        let running = self.running.clone();
        let to_addr: SocketAddr = format!("{}:{}", to_ip, to_port)
            .parse()
            .map_err(|e| format!("Invalid target address: {}", e))?;

        if udp {
            let socket = UdpSocket::bind(format!("0.0.0.0:{}", port))
                .map_err(|e| format!("Bind UDP forwarder failed: {}", e))?;
            socket.set_read_timeout(Some(Duration::from_secs(1))).ok();
            thread::spawn(move || {
                let mut buf = [0u8; 8192];
                while running.load(Ordering::SeqCst) {
                    match socket.recv_from(&mut buf) {
                        Ok((n, src_addr)) => {
                            if let Ok(out_sock) = UdpSocket::bind("0.0.0.0:0") {
                                let _ = out_sock.send_to(&buf[..n], to_addr);
                                out_sock.set_read_timeout(Some(Duration::from_secs(3))).ok();
                                let mut resp = [0u8; 8192];
                                if let Ok((rn, _)) = out_sock.recv_from(&mut resp) {
                                    let _ = socket.send_to(&resp[..rn], src_addr);
                                }
                            }
                        }
                        Err(_) => continue,
                    }
                }
            });
        } else {
            let listener = TcpListener::bind(format!("0.0.0.0:{}", port))
                .map_err(|e| format!("Bind TCP forwarder failed: {}", e))?;
            listener.set_nonblocking(true).ok();
            thread::spawn(move || {
                while running.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((inbound, _)) => {
                            let running_c = running.clone();
                            if let Ok(outbound) = TcpStream::connect_timeout(
                                &to_addr,
                                Duration::from_secs(3),
                            ) {
                                let inbound_clone = inbound.try_clone().unwrap();
                                let outbound_clone = outbound.try_clone().unwrap();

                                let r1 = running_c.clone();
                                thread::spawn(move || {
                                    forward_tcp_stream(inbound, outbound_clone, r1);
                                });
                                let r2 = running_c;
                                thread::spawn(move || {
                                    forward_tcp_stream(outbound, inbound_clone, r2);
                                });
                            }
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(100));
                        }
                        Err(_) => {
                            thread::sleep(Duration::from_millis(100));
                        }
                    }
                }
            });
        }
        Ok(())
    }
}

fn forward_tcp_stream(mut from: TcpStream, mut to: TcpStream, running: Arc<AtomicBool>) {
    let mut buf = [0u8; 8192];
    from.set_read_timeout(Some(Duration::from_secs(30))).ok();
    while running.load(Ordering::SeqCst) {
        match from.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                if to.write_all(&buf[..n]).is_err() {
                    break;
                }
            }
            Err(ref e)
                if e.kind() == std::io::ErrorKind::WouldBlock
                    || e.kind() == std::io::ErrorKind::TimedOut =>
            {
                continue;
            }
            Err(_) => break,
        }
    }
    let _ = from.shutdown(Shutdown::Both);
    let _ = to.shutdown(Shutdown::Both);
}
