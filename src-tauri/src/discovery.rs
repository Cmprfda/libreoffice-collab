//! Zero-configuration server discovery over mDNS (Bonjour).
//!
//! The host advertises `_locollab._tcp.local.` with a few TXT records; every
//! client browses for it continuously. Nothing has to be typed, no IP address
//! is ever shown to the user unless they ask for it.
//!
//! TXT records published by `collab-server`:
//!   name    friendly host name, e.g. "Escritorio - Piso 2"
//!   version server version
//!   cool    Collabora Online base URL, e.g. "http://192.168.1.50:9980"
//!   scheme  "http" or "https" for the REST/WOPI endpoint

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};

pub const SERVICE_TYPE: &str = "_locollab._tcp.local.";
pub const EVENT_SERVERS_CHANGED: &str = "servers://changed";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredServer {
    /// mDNS instance fullname, or "manual" for a hand-typed address.
    pub id: String,
    pub name: String,
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub cool_url: String,
    pub version: String,
    pub manual: bool,
}

/// Everything the browser currently sees, keyed by mDNS fullname.
#[derive(Default)]
pub struct Discovery {
    daemon: Mutex<Option<ServiceDaemon>>,
    servers: Arc<Mutex<HashMap<String, DiscoveredServer>>>,
}

impl Discovery {
    pub fn new() -> Self {
        Self::default()
    }

    /// Current snapshot, sorted so the list does not jump around between renders.
    pub fn snapshot(&self) -> Vec<DiscoveredServer> {
        let mut list: Vec<_> = self
            .servers
            .lock()
            .map(|guard| guard.values().cloned().collect())
            .unwrap_or_default();
        list.sort_by_key(|server| server.name.to_lowercase());
        list
    }

    /// Starts (or restarts) the mDNS browser. Safe to call repeatedly — the
    /// Refresh button in the UI does exactly that.
    pub fn start(&self, app: AppHandle) -> anyhow::Result<()> {
        self.stop();

        let daemon = ServiceDaemon::new()?;
        let receiver = daemon.browse(SERVICE_TYPE)?;

        if let Ok(mut slot) = self.daemon.lock() {
            *slot = Some(daemon);
        }

        let servers = Arc::clone(&self.servers);

        // mdns-sd hands us a blocking channel, so this lives on its own thread
        // rather than inside the async runtime.
        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                let changed = match event {
                    ServiceEvent::ServiceResolved(info) => {
                        match to_server(&info) {
                            Some(server) => {
                                if let Ok(mut map) = servers.lock() {
                                    map.insert(server.id.clone(), server);
                                }
                                true
                            }
                            // A host that advertises the service type but has no
                            // usable IPv4 address is ignored rather than shown
                            // as a broken entry.
                            None => false,
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        if let Ok(mut map) = servers.lock() {
                            map.remove(&fullname).is_some()
                        } else {
                            false
                        }
                    }
                    _ => false,
                };

                if changed {
                    let list: Vec<DiscoveredServer> = servers
                        .lock()
                        .map(|guard| guard.values().cloned().collect())
                        .unwrap_or_default();
                    let _ = app.emit(EVENT_SERVERS_CHANGED, list);
                }
            }
            log::debug!("mDNS browse channel closed");
        });

        Ok(())
    }

    /// Shuts the browser down and clears the cache so stale hosts disappear.
    pub fn stop(&self) {
        if let Ok(mut slot) = self.daemon.lock() {
            if let Some(daemon) = slot.take() {
                let _ = daemon.shutdown();
            }
        }
        if let Ok(mut map) = self.servers.lock() {
            map.clear();
        }
    }
}

/// Turns a resolved mDNS record into something the UI can display.
fn to_server(info: &ServiceInfo) -> Option<DiscoveredServer> {
    // Prefer IPv4: Collabora and the WOPI host are reached by literal address,
    // and IPv6 link-local addresses need a zone index that URLs cannot carry.
    let ip: IpAddr = info
        .get_addresses()
        .iter()
        .find(|addr| addr.is_ipv4())
        .copied()
        .or_else(|| info.get_addresses().iter().next().copied())?;

    let port = info.get_port();
    let scheme = info.get_property_val_str("scheme").unwrap_or("http");
    let base_url = format!("{scheme}://{ip}:{port}");

    // Falls back to the base URL's host on the default Collabora port so an
    // older host that does not publish `cool` still works.
    let cool_url = info
        .get_property_val_str("cool")
        .map(str::to_owned)
        .unwrap_or_else(|| format!("http://{ip}:9980"));

    let name = info
        .get_property_val_str("name")
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        // Instance name looks like "Escritorio._locollab._tcp.local." — trim it.
        .unwrap_or_else(|| {
            info.get_fullname()
                .split('.')
                .next()
                .unwrap_or("LibreOffice Collab")
                .replace('\\', "")
        });

    Some(DiscoveredServer {
        id: info.get_fullname().to_string(),
        name,
        host: ip.to_string(),
        port,
        base_url,
        cool_url,
        version: info
            .get_property_val_str("version")
            .unwrap_or("")
            .to_string(),
        manual: false,
    })
}
