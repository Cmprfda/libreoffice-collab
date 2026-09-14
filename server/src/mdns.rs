//! mDNS/Bonjour advertisement.
//!
//! This is the whole of "zero configuration": the host shouts its name, port
//! and Collabora URL onto the local link, and every client picks it up without
//! anyone typing an IP address.

use std::collections::HashMap;
use std::net::Ipv4Addr;

use mdns_sd::{ServiceDaemon, ServiceInfo};

use crate::config::Config;

pub const SERVICE_TYPE: &str = "_locollab._tcp.local.";

/// Registers the service and returns the daemon.
///
/// The daemon must stay alive for the advert to keep being answered, so the
/// caller holds on to it for the lifetime of the process.
pub fn advertise(config: &Config) -> anyhow::Result<ServiceDaemon> {
    let daemon = ServiceDaemon::new()?;

    let mut properties = HashMap::new();
    properties.insert("name".to_string(), config.name.clone());
    properties.insert("version".to_string(), env!("CARGO_PKG_VERSION").to_string());
    properties.insert("cool".to_string(), config.cool_url.clone());
    properties.insert("scheme".to_string(), "http".to_string());

    // The instance name is what appears in generic Bonjour browsers; the
    // friendly name the app shows comes from the `name` TXT record.
    let instance = sanitise_instance(&config.name);
    let host_name = format!("{}.local.", sanitise_instance(&config.name).to_lowercase());

    let service = ServiceInfo::new(
        SERVICE_TYPE,
        &instance,
        &host_name,
        config.public_ip,
        config.port,
        properties,
    )?;

    daemon.register(service)?;
    Ok(daemon)
}

/// mDNS instance labels must not contain dots; everything else is tolerated.
fn sanitise_instance(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| if c == '.' { '-' } else { c })
        .collect();
    if cleaned.trim().is_empty() {
        "LibreOffice-Collab".to_string()
    } else {
        cleaned.trim().to_string()
    }
}

/// Convenience for the startup banner.
pub fn describe(config: &Config) -> String {
    let ip: Ipv4Addr = config.public_ip;
    format!("{} @ {}:{}", config.name, ip, config.port)
}
