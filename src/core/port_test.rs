use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

/// Test if a port is open from LAN
pub fn test_lan(addr: SocketAddr) -> i32 {
    match TcpStream::connect_timeout(&addr, Duration::from_secs(1)) {
        Ok(_) => {
            log::info!("LAN > {} [ OPEN ]", addr);
            1
        }
        Err(_) => {
            log::info!("LAN > {} [ CLOSED ]", addr);
            -1
        }
    }
}

/// Test if a port is open from WAN via ifconfig.co
pub fn test_wan(port: u16) -> i32 {
    let ret = test_ifconfig_co(port);
    if ret == 1 {
        log::info!("WAN > port {} [ OPEN ]", port);
        return 1;
    }
    if ret == -1 {
        log::info!("WAN > port {} [ CLOSED ]", port);
        return -1;
    }
    log::info!("WAN > port {} [ UNKNOWN ]", port);
    0
}

fn test_ifconfig_co(port: u16) -> i32 {
    let result = (|| -> Result<bool, Box<dyn std::error::Error>> {
        let mut stream = TcpStream::connect_timeout(
            &"ifconfig.co:80".parse::<SocketAddr>().unwrap_or_else(|_| {
                use std::net::ToSocketAddrs;
                "ifconfig.co:80"
                    .to_socket_addrs()
                    .ok()
                    .and_then(|mut a| a.next())
                    .unwrap_or_else(|| "0.0.0.0:80".parse().unwrap())
            }),
            Duration::from_secs(8),
        )?;
        stream.set_read_timeout(Some(Duration::from_secs(8)))?;

        let request = format!(
            "GET /port/{} HTTP/1.0\r\n\
             Host: ifconfig.co\r\n\
             User-Agent: curl/8.0.0 (HolePatch)\r\n\
             Accept: */*\r\n\
             Connection: close\r\n\
             \r\n",
            port
        );
        stream.write_all(request.as_bytes())?;

        let mut response = Vec::new();
        stream.read_to_end(&mut response)?;

        let response_str = String::from_utf8_lossy(&response);
        if let Some(body) = response_str.split("\r\n\r\n").nth(1) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
                if let Some(reachable) = json.get("reachable").and_then(|v| v.as_bool()) {
                    return Ok(reachable);
                }
            }
        }
        Err("Failed to parse response".into())
    })();

    match result {
        Ok(true) => 1,
        Ok(false) => -1,
        Err(e) => {
            log::debug!("port-test ifconfig.co error: {}", e);
            0
        }
    }
}
