use super::forward::{ForwardMethod, Forwarder};
use super::keepalive::KeepAlive;
use super::port_test;
use super::stun::StunClient;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Configuration for a Natter session
#[derive(Debug, Clone)]
pub struct NatterConfig {
    pub udp_mode: bool,
    pub bind_ip: String,
    pub bind_port: u16,
    pub stun_servers: Vec<(String, u16)>,
    pub keepalive_host: String,
    pub keepalive_port: u16,
    pub forward_method: ForwardMethod,
    pub target_ip: String,
    pub target_port: u16,
    pub keepalive_interval: u64,
}

impl Default for NatterConfig {
    fn default() -> Self {
        NatterConfig {
            udp_mode: false,
            bind_ip: "0.0.0.0".into(),
            bind_port: 0,
            stun_servers: vec![],
            keepalive_host: "www.baidu.com".into(),
            keepalive_port: 80,
            forward_method: ForwardMethod::TestServer,
            target_ip: "0.0.0.0".into(),
            target_port: 0,
            keepalive_interval: 15,
        }
    }
}

/// The current status of a Natter session
#[derive(Debug, Clone)]
pub enum NatterStatus {
    Idle,
    Connecting,
    Connected {
        inner_addr: SocketAddr,
        outer_addr: SocketAddr,
        route_info: String,
    },
    Error(String),
}

/// Log entry for the GUI
#[derive(Debug, Clone)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub message: String,
}

/// A running Natter session
pub struct NatterSession {
    running: Arc<AtomicBool>,
    pub status: Arc<Mutex<NatterStatus>>,
    pub logs: Arc<Mutex<Vec<LogEntry>>>,
}

impl NatterSession {
    pub fn new() -> Self {
        NatterSession {
            running: Arc::new(AtomicBool::new(false)),
            status: Arc::new(Mutex::new(NatterStatus::Idle)),
            logs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add_log(logs: &Arc<Mutex<Vec<LogEntry>>>, level: &str, message: &str) {
        let entry = LogEntry {
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            level: level.to_string(),
            message: message.to_string(),
        };
        if let Ok(mut l) = logs.lock() {
            l.push(entry);
            // Keep only last 200 log entries
            if l.len() > 200 {
                let excess = l.len() - 200;
                l.drain(0..excess);
            }
        }
    }

    pub fn start(&mut self, config: NatterConfig) {
        self.running.store(true, Ordering::SeqCst);
        *self.status.lock().unwrap() = NatterStatus::Connecting;
        self.logs.lock().unwrap().clear();

        let running = self.running.clone();
        let status = self.status.clone();
        let logs = self.logs.clone();

        thread::spawn(move || {
            Self::add_log(&logs, "INFO", &format!("HolePatch starting..."));
            Self::add_log(
                &logs,
                "INFO",
                &format!("Mode: {}", if config.udp_mode { "UDP" } else { "TCP" }),
            );

            // Determine actual config
            let mut stun_servers = config.stun_servers.clone();
            if stun_servers.is_empty() {
                stun_servers = if config.udp_mode {
                    super::stun::default_stun_servers_udp()
                } else {
                    super::stun::default_stun_servers_tcp()
                };
            }

            // Create STUN client
            let mut stun = StunClient::new(
                stun_servers,
                &config.bind_ip,
                config.bind_port,
                config.udp_mode,
            );

            Self::add_log(&logs, "INFO", "Getting NAT mapping via STUN...");
            let stun_result = match stun.get_mapping() {
                Ok(r) => r,
                Err(e) => {
                    let msg = format!("STUN failed: {}", e);
                    Self::add_log(&logs, "ERROR", &msg);
                    *status.lock().unwrap() = NatterStatus::Error(msg);
                    return;
                }
            };

            let inner_addr = stun_result.inner_addr;
            let outer_addr = stun_result.outer_addr;
            Self::add_log(
                &logs,
                "INFO",
                &format!("NAT mapping: {} -> {}", inner_addr, outer_addr),
            );

            // Update source for keep-alive
            let bind_ip = inner_addr.ip().to_string();
            let bind_port = inner_addr.port();

            // Create keep-alive
            let keepalive_host = if config.keepalive_host.is_empty() {
                if config.udp_mode {
                    "119.29.29.29"
                } else {
                    "www.baidu.com"
                }
                .to_string()
            } else {
                config.keepalive_host.clone()
            };
            let keepalive_port = if config.keepalive_port == 0 {
                if config.udp_mode { 53 } else { 80 }
            } else {
                config.keepalive_port
            };

            let mut keep_alive = KeepAlive::new(
                &keepalive_host,
                keepalive_port,
                &bind_ip,
                bind_port,
                config.udp_mode,
            );

            Self::add_log(&logs, "INFO", "Establishing keep-alive connection...");
            if let Err(e) = keep_alive.keep_alive() {
                Self::add_log(&logs, "WARN", &format!("Keep-alive init failed: {}", e));
            }

            // Determine target address
            let mut to_ip = config.target_ip.clone();
            let mut to_port = config.target_port;

            if to_ip == "0.0.0.0" || to_ip.is_empty() {
                to_ip = bind_ip.clone();
            }
            if to_port == 0 {
                to_port = outer_addr.port();
            }

            // For test server and none methods, target = natter address
            if config.forward_method == ForwardMethod::TestServer
                || config.forward_method == ForwardMethod::None
            {
                to_ip = inner_addr.ip().to_string();
                to_port = inner_addr.port();
            }

            // Start forwarder
            let mut forwarder = Forwarder::new(config.forward_method.clone());
            let fwd_method_name = config.forward_method.display_name();

            if config.forward_method != ForwardMethod::None {
                Self::add_log(
                    &logs,
                    "INFO",
                    &format!(
                        "Starting {} forward: {} -> {}:{}",
                        fwd_method_name, inner_addr, to_ip, to_port
                    ),
                );
                if let Err(e) = forwarder.start_forward(
                    &inner_addr.ip().to_string(),
                    inner_addr.port(),
                    &to_ip,
                    to_port,
                    config.udp_mode,
                ) {
                    let msg = format!("Forward failed: {}", e);
                    Self::add_log(&logs, "ERROR", &msg);
                    *status.lock().unwrap() = NatterStatus::Error(msg);
                    return;
                }
            }

            // Build route info string
            let route_info = if config.forward_method != ForwardMethod::None
                && config.forward_method != ForwardMethod::TestServer
            {
                format!(
                    "{}:{} <--{}--> {} <--Natter--> {}",
                    to_ip,
                    to_port,
                    fwd_method_name,
                    inner_addr,
                    outer_addr
                )
            } else {
                format!("{} <--Natter--> {}", inner_addr, outer_addr)
            };

            Self::add_log(&logs, "INFO", &route_info);

            // Port test (TCP only)
            if !config.udp_mode {
                Self::add_log(&logs, "INFO", "Testing port accessibility...");
                let target_addr: SocketAddr = format!("{}:{}", to_ip, to_port).parse().unwrap();
                let lan_result = port_test::test_lan(target_addr);
                match lan_result {
                    1 => Self::add_log(&logs, "INFO", &format!("LAN > {} [ OPEN ]", target_addr)),
                    -1 => Self::add_log(
                        &logs,
                        "WARN",
                        &format!("LAN > {} [ CLOSED ]", target_addr),
                    ),
                    _ => Self::add_log(
                        &logs,
                        "INFO",
                        &format!("LAN > {} [ UNKNOWN ]", target_addr),
                    ),
                }
            }

            if config.forward_method == ForwardMethod::TestServer {
                Self::add_log(&logs, "INFO", "Test mode is on.");
                Self::add_log(
                    &logs,
                    "INFO",
                    &format!(
                        "Please check {}://{}",
                        if config.udp_mode { "udp" } else { "http" },
                        outer_addr
                    ),
                );
            }

            *status.lock().unwrap() = NatterStatus::Connected {
                inner_addr,
                outer_addr,
                route_info: route_info.clone(),
            };

            // Main keep-alive loop
            let interval = Duration::from_secs(config.keepalive_interval);
            let mut cnt = 0u32;
            while running.load(Ordering::SeqCst) {
                cnt = (cnt + 1) % 20;
                let need_recheck = cnt == 0;

                if need_recheck {
                    Self::add_log(&logs, "DEBUG", "Rechecking mapping...");
                    match stun.get_mapping() {
                        Ok(new_result) => {
                            if new_result.outer_addr != outer_addr {
                                let msg = format!(
                                    "Mapped address changed: {} -> {}",
                                    outer_addr, new_result.outer_addr
                                );
                                Self::add_log(&logs, "WARN", &msg);
                                // Continue running but warn
                            }
                        }
                        Err(e) => {
                            Self::add_log(
                                &logs,
                                "WARN",
                                &format!("STUN recheck failed: {}", e),
                            );
                        }
                    }
                }

                if let Err(e) = keep_alive.keep_alive() {
                    Self::add_log(
                        &logs,
                        "WARN",
                        &format!("Keep-alive failed: {}", e),
                    );
                    keep_alive.reset();
                }

                // Sleep in small increments so we can check running flag
                for _ in 0..(interval.as_millis() / 500) {
                    if !running.load(Ordering::SeqCst) {
                        break;
                    }
                    thread::sleep(Duration::from_millis(500));
                }
            }

            // Cleanup
            forwarder.stop_forward();
            Self::add_log(&logs, "INFO", "Session stopped.");
            *status.lock().unwrap() = NatterStatus::Idle;
        });
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}
