//! Runtime configuration.
//!
//! Everything has a working default so `collab-server.exe` can be double-clicked
//! with no arguments. The setup script still passes explicit values so the
//! shared folder ends up somewhere predictable.

use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Config {
    /// Folder whose files are offered for collaborative editing.
    pub docs_dir: PathBuf,
    /// Port for the REST + WOPI endpoints.
    pub port: u16,
    /// Base URL of the Collabora Online container, e.g. "http://192.168.1.50:9980".
    pub cool_url: String,
    /// Friendly name shown in the client's server list.
    pub name: String,
    /// LAN address other machines use to reach this host.
    pub public_ip: Ipv4Addr,
}

impl Config {
    /// Reads CLI flags, then environment variables, then falls back to defaults.
    ///
    /// Flags: --dir <path> --port <n> --cool <url> --name <text>
    /// Env:   COLLAB_DIR, COLLAB_PORT, COLLAB_COOL_URL, COLLAB_NAME
    pub fn load() -> Self {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let flag = |name: &str| -> Option<String> {
            args.iter()
                .position(|arg| arg == name)
                .and_then(|index| args.get(index + 1))
                .cloned()
        };

        let public_ip = detect_lan_ipv4();

        let docs_dir = flag("--dir")
            .or_else(|| std::env::var("COLLAB_DIR").ok())
            .map(PathBuf::from)
            .unwrap_or_else(default_docs_dir);

        let port = flag("--port")
            .or_else(|| std::env::var("COLLAB_PORT").ok())
            .and_then(|value| value.parse().ok())
            .unwrap_or(7373);

        let cool_url = flag("--cool")
            .or_else(|| std::env::var("COLLAB_COOL_URL").ok())
            .unwrap_or_else(|| format!("http://{public_ip}:9980"))
            .trim_end_matches('/')
            .to_string();

        let name = flag("--name")
            .or_else(|| std::env::var("COLLAB_NAME").ok())
            .unwrap_or_else(machine_name);

        Self {
            docs_dir,
            port,
            cool_url,
            name,
            public_ip,
        }
    }

    /// Base URL that clients and Collabora use to reach this server.
    pub fn base_url(&self) -> String {
        format!("http://{}:{}", self.public_ip, self.port)
    }
}

/// `%PUBLIC%\Documentos Partilhados` on Windows, `./documentos` elsewhere.
fn default_docs_dir() -> PathBuf {
    if let Ok(public) = std::env::var("PUBLIC") {
        return PathBuf::from(public).join("Documentos Partilhados");
    }
    PathBuf::from("documentos")
}

fn machine_name() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "LibreOffice Collab".to_string())
}

/// Finds the address this machine uses to reach the rest of the LAN.
///
/// Connecting a UDP socket sends no packets; it only makes the OS pick a route
/// and bind a source address, which is exactly the address we want to publish.
/// Falls back to loopback when the machine is offline.
pub fn detect_lan_ipv4() -> Ipv4Addr {
    let socket = match UdpSocket::bind("0.0.0.0:0") {
        Ok(socket) => socket,
        Err(_) => return Ipv4Addr::LOCALHOST,
    };
    // Any routable address works here; nothing is actually contacted.
    if socket.connect("8.8.8.8:80").is_err() {
        return Ipv4Addr::LOCALHOST;
    }
    match socket.local_addr() {
        Ok(addr) => match addr.ip() {
            IpAddr::V4(ip) => ip,
            IpAddr::V6(_) => Ipv4Addr::LOCALHOST,
        },
        Err(_) => Ipv4Addr::LOCALHOST,
    }
}
