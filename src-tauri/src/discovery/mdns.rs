//! DNS-SD discovery, as a second opinion alongside the multicast beacon.
//!
//! This exists because mDNS is what the rest of the world speaks: it plays
//! nicely with OS-level service browsers and works on some networks that drop
//! our own multicast group. It is not a replacement for [`super::beacon`] —
//! plenty of managed networks filter mDNS specifically, which is why both run.

use std::net::IpAddr;
use std::sync::Arc;

use mdns_sd::{ResolvedService, ServiceDaemon, ServiceEvent, ServiceInfo};

use super::{DiscoveredPeer, DiscoverySource, PeerRegistry, MDNS_SERVICE_TYPE};
use crate::error::{Error, Result};

/// TXT keys carrying the metadata the beacon puts in its JSON payload.
const TXT_DEVICE_ID: &str = "id";
const TXT_DEVICE_NAME: &str = "name";
const TXT_PLATFORM: &str = "os";

pub struct MdnsService {
    daemon: ServiceDaemon,
    instance_fullname: Option<String>,
}

impl MdnsService {
    pub fn start() -> Result<Self> {
        let daemon = ServiceDaemon::new()
            .map_err(|err| Error::Discovery(format!("could not start mDNS: {err}")))?;

        Ok(MdnsService {
            daemon,
            instance_fullname: None,
        })
    }

    /// Publish this device as an mDNS service.
    pub fn advertise(
        &mut self,
        device_id: &str,
        device_name: &str,
        platform: &str,
        port: u16,
    ) -> Result<()> {
        // The instance name must be unique on the network and DNS-safe, so it
        // is derived from the device ID rather than the user-visible name,
        // which may contain anything at all.
        let instance = sanitize_instance_name(device_id);

        let properties = [
            (TXT_DEVICE_ID, device_id),
            (TXT_DEVICE_NAME, device_name),
            (TXT_PLATFORM, platform),
        ];

        let service = ServiceInfo::new(
            MDNS_SERVICE_TYPE,
            &instance,
            &format!("{instance}.local."),
            "",
            port,
            &properties[..],
        )
        .map_err(|err| Error::Discovery(format!("could not build the mDNS record: {err}")))?
        // Let mdns-sd fill in this host's addresses; enumerating interfaces
        // ourselves would be wrong on multi-homed machines.
        .enable_addr_auto();

        self.instance_fullname = Some(service.get_fullname().to_string());

        self.daemon
            .register(service)
            .map_err(|err| Error::Discovery(format!("could not publish over mDNS: {err}")))?;

        Ok(())
    }

    /// Browse for peers, feeding anything found into the shared registry.
    /// Runs until the daemon shuts down.
    pub fn run_browser(&self, registry: Arc<PeerRegistry>, self_device_id: String) -> Result<()> {
        let receiver = self
            .daemon
            .browse(MDNS_SERVICE_TYPE)
            .map_err(|err| Error::Discovery(format!("could not browse over mDNS: {err}")))?;

        std::thread::spawn(move || {
            while let Ok(event) = receiver.recv() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        if let Some(peer) = to_peer(&info, &self_device_id) {
                            registry.observe(peer);
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        // The fullname embeds the sanitized device ID, so the
                        // registry entry can be found without another lookup.
                        if let Some(device_id) = device_id_from_fullname(&fullname) {
                            registry.forget(&device_id);
                        }
                    }
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub fn shutdown(&self) {
        if let Some(fullname) = &self.instance_fullname {
            let _ = self.daemon.unregister(fullname);
        }
        let _ = self.daemon.shutdown();
    }
}

fn to_peer(service: &ResolvedService, self_device_id: &str) -> Option<DiscoveredPeer> {
    let device_id = service
        .txt_properties
        .get_property_val_str(TXT_DEVICE_ID)
        .map(|s| s.to_string())
        .unwrap_or_else(|| device_id_from_fullname(&service.fullname).unwrap_or_default());

    if device_id.is_empty() || device_id == self_device_id {
        return None;
    }

    // Prefer IPv4: the transfer listener binds v4, and a link-local IPv6
    // address would need its scope ID to be dialable at all.
    let address: IpAddr = service
        .addresses
        .iter()
        .find(|addr| addr.is_ipv4())
        .or_else(|| service.addresses.iter().next())
        .map(|addr| addr.to_ip_addr())?;

    Some(DiscoveredPeer {
        device_name: service
            .txt_properties
            .get_property_val_str(TXT_DEVICE_NAME)
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("Unknown device")
            .chars()
            .filter(|ch| !ch.is_control())
            .take(64)
            .collect(),
        platform: service
            .txt_properties
            .get_property_val_str(TXT_PLATFORM)
            .unwrap_or("unknown")
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .take(16)
            .collect(),
        device_id,
        address,
        port: service.port,
        source: DiscoverySource::Mdns,
        last_seen_secs: 0,
        fingerprint_hint: None,
    })
}

/// Reduce a device ID to something valid in a DNS label.
fn sanitize_instance_name(device_id: &str) -> String {
    let cleaned: String = device_id
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-')
        .take(48)
        .collect();

    if cleaned.is_empty() {
        format!("fluqsr-{}", uuid::Uuid::new_v4().simple())
    } else {
        format!("fluqsr-{cleaned}")
    }
}

/// Recover the sanitized device ID from `fluqsr-<id>._fluqsr._tcp.local.`.
fn device_id_from_fullname(fullname: &str) -> Option<String> {
    fullname
        .split('.')
        .next()
        .and_then(|instance| instance.strip_prefix("fluqsr-"))
        .map(|id| id.to_string())
        .filter(|id| !id.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instance_names_are_dns_safe() {
        let name = sanitize_instance_name("3f2a-9c11-BEEF");
        assert!(name.starts_with("fluqsr-"));
        assert!(name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-'));
    }

    #[test]
    fn instance_names_strip_hostile_characters() {
        let name = sanitize_instance_name("../../evil name.local");
        assert!(!name.contains('.'));
        assert!(!name.contains('/'));
        assert!(!name.contains(' '));
    }

    #[test]
    fn an_empty_device_id_still_yields_a_usable_name() {
        let name = sanitize_instance_name("");
        assert!(name.starts_with("fluqsr-"));
        assert!(name.len() > "fluqsr-".len());
    }

    #[test]
    fn instance_names_are_bounded() {
        assert!(sanitize_instance_name(&"a".repeat(500)).len() <= 48 + "fluqsr-".len());
    }

    #[test]
    fn device_id_round_trips_through_the_fullname() {
        let device_id = "3f2a9c11";
        let instance = sanitize_instance_name(device_id);
        let fullname = format!("{instance}.{MDNS_SERVICE_TYPE}");

        assert_eq!(device_id_from_fullname(&fullname).as_deref(), Some(device_id));
    }

    #[test]
    fn a_foreign_fullname_yields_nothing() {
        assert!(device_id_from_fullname("someoneelse._http._tcp.local.").is_none());
        assert!(device_id_from_fullname("fluqsr-._fluqsr._tcp.local.").is_none());
    }
}
