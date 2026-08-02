//! Finding other fluqsr devices on the local network.
//!
//! Two independent transports feed one registry, because either can be
//! unavailable on a given network:
//!
//!   * [`beacon`] — a UDP multicast heartbeat. Self-contained, needs no system
//!     daemon, and keeps working where mDNS is filtered.
//!   * [`mdns`] — standard DNS-SD. Better OS integration and interoperability,
//!     but blocked or unreliable on plenty of managed networks.
//!
//! Neither is authoritative. Discovery decides what the user *sees*; nothing
//! discovered here is trusted until a TLS handshake proves the peer's key.

pub mod beacon;
pub mod mdns;

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::watch;

/// UDP port for the multicast beacon.
pub const DISCOVERY_PORT: u16 = 47653;

/// Default TCP port transfers are accepted on.
pub const DEFAULT_TRANSFER_PORT: u16 = 47654;

/// Administratively-scoped multicast group — routers will not forward it off
/// the local network, which is exactly the reach we want.
pub const MULTICAST_GROUP: std::net::Ipv4Addr = std::net::Ipv4Addr::new(239, 255, 77, 7);

pub const MDNS_SERVICE_TYPE: &str = "_fluqsr._tcp.local.";

/// How often each device announces itself.
pub const ANNOUNCE_INTERVAL: Duration = Duration::from_secs(3);

/// A peer is dropped from the list after this long without a beacon. Several
/// announce intervals, so one dropped packet does not make a device flicker
/// out of the UI.
pub const PEER_TIMEOUT: Duration = Duration::from_secs(15);

/// Which transport told us about a peer. Purely informational — useful when
/// diagnosing a network where only one of the two works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DiscoverySource {
    Beacon,
    Mdns,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredPeer {
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    pub address: IpAddr,
    pub port: u16,
    pub source: DiscoverySource,
    /// Seconds since this peer was last heard from.
    pub last_seen_secs: u64,
    /// The fingerprint the peer *claims*, for display only.
    ///
    /// Anyone on the network can put anything here — it is unauthenticated
    /// broadcast data. It exists so the UI can show a likely-paired hint
    /// before connecting, and must never be used to decide trust. The
    /// authoritative fingerprint comes from the TLS certificate.
    pub fingerprint_hint: Option<String>,
}

struct PeerEntry {
    peer: DiscoveredPeer,
    last_seen: Instant,
}

/// The merged view of everything both transports have found.
pub struct PeerRegistry {
    peers: Mutex<HashMap<String, PeerEntry>>,
    /// Own device ID, so we filter ourselves out — multicast loopback means we
    /// receive our own announcements.
    self_device_id: String,
    changes: watch::Sender<Vec<DiscoveredPeer>>,
}

impl PeerRegistry {
    pub fn new(self_device_id: String) -> Self {
        let (changes, _) = watch::channel(Vec::new());
        PeerRegistry {
            peers: Mutex::new(HashMap::new()),
            self_device_id,
            changes,
        }
    }

    /// Subscribe to peer-list changes. The UI layer forwards these to the
    /// frontend; nothing else in the backend needs to poll.
    pub fn subscribe(&self) -> watch::Receiver<Vec<DiscoveredPeer>> {
        self.changes.subscribe()
    }

    /// Record a sighting. Returns true when this was a peer we had not seen
    /// before, which the beacon uses to decide whether to reply immediately
    /// instead of waiting for the next interval.
    pub fn observe(&self, peer: DiscoveredPeer) -> bool {
        if peer.device_id == self.self_device_id {
            return false;
        }

        let is_new = {
            let mut peers = self.peers.lock().unwrap();
            let is_new = !peers.contains_key(&peer.device_id);

            // A peer found by both transports keeps whichever entry arrived
            // first; the address and port are the same either way, and
            // churning the source field would make the UI flicker.
            peers.insert(
                peer.device_id.clone(),
                PeerEntry {
                    peer,
                    last_seen: Instant::now(),
                },
            );
            is_new
        };

        self.publish();
        is_new
    }

    pub fn forget(&self, device_id: &str) {
        self.peers.lock().unwrap().remove(device_id);
        self.publish();
    }

    /// Drop peers that have gone quiet. Called on a timer.
    pub fn evict_stale(&self) {
        let removed = {
            let mut peers = self.peers.lock().unwrap();
            let before = peers.len();
            peers.retain(|_, entry| entry.last_seen.elapsed() < PEER_TIMEOUT);
            before != peers.len()
        };

        if removed {
            self.publish();
        }
    }

    pub fn snapshot(&self) -> Vec<DiscoveredPeer> {
        let peers = self.peers.lock().unwrap();
        let mut list: Vec<_> = peers
            .values()
            .map(|entry| {
                let mut peer = entry.peer.clone();
                peer.last_seen_secs = entry.last_seen.elapsed().as_secs();
                peer
            })
            .collect();

        list.sort_by(|a, b| {
            a.device_name
                .to_lowercase()
                .cmp(&b.device_name.to_lowercase())
                .then_with(|| a.device_id.cmp(&b.device_id))
        });
        list
    }

    pub fn get(&self, device_id: &str) -> Option<DiscoveredPeer> {
        self.peers
            .lock()
            .unwrap()
            .get(device_id)
            .map(|entry| entry.peer.clone())
    }

    fn publish(&self) {
        // A send failure only means nothing is listening yet, which is fine.
        let _ = self.changes.send(self.snapshot());
    }
}

/// What a device broadcasts about itself.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Announcement {
    pub v: u32,
    pub kind: AnnouncementKind,
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
    /// TCP port this device accepts transfers on. The address is taken from
    /// the packet's source rather than being carried in the payload — a device
    /// behind multiple interfaces cannot reliably know which of its own
    /// addresses the receiver can reach.
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint_hint: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AnnouncementKind {
    /// A periodic multicast heartbeat.
    Announce,
    /// A unicast answer to a heartbeat from a device we had not seen. Replies
    /// are not themselves replied to, which stops two devices from ping-ponging
    /// forever.
    Reply,
    /// Sent on a clean shutdown so peers drop us immediately rather than
    /// waiting out the timeout.
    Goodbye,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    fn peer(id: &str, name: &str) -> DiscoveredPeer {
        DiscoveredPeer {
            device_id: id.into(),
            device_name: name.into(),
            platform: "linux".into(),
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 20)),
            port: DEFAULT_TRANSFER_PORT,
            source: DiscoverySource::Beacon,
            last_seen_secs: 0,
            fingerprint_hint: None,
        }
    }

    #[test]
    fn records_a_new_peer() {
        let registry = PeerRegistry::new("me".into());
        assert!(registry.observe(peer("them", "Laptop")));
        assert_eq!(registry.snapshot().len(), 1);
    }

    #[test]
    fn ignores_our_own_announcements() {
        // Multicast loopback means we always hear ourselves.
        let registry = PeerRegistry::new("me".into());
        assert!(!registry.observe(peer("me", "This Device")));
        assert!(registry.snapshot().is_empty());
    }

    #[test]
    fn repeat_sightings_do_not_duplicate() {
        let registry = PeerRegistry::new("me".into());
        assert!(registry.observe(peer("them", "Laptop")));
        assert!(!registry.observe(peer("them", "Laptop")));
        assert_eq!(registry.snapshot().len(), 1);
    }

    #[test]
    fn a_peer_found_by_both_transports_appears_once() {
        let registry = PeerRegistry::new("me".into());

        registry.observe(peer("them", "Laptop"));
        let mut via_mdns = peer("them", "Laptop");
        via_mdns.source = DiscoverySource::Mdns;
        registry.observe(via_mdns);

        assert_eq!(
            registry.snapshot().len(),
            1,
            "the same device seen twice must merge, not double up"
        );
    }

    #[test]
    fn a_renamed_peer_updates_in_place() {
        let registry = PeerRegistry::new("me".into());
        registry.observe(peer("them", "Laptop"));
        registry.observe(peer("them", "Athar's Laptop"));

        let list = registry.snapshot();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].device_name, "Athar's Laptop");
    }

    #[test]
    fn peers_sort_by_name() {
        let registry = PeerRegistry::new("me".into());
        registry.observe(peer("c", "Zebra"));
        registry.observe(peer("a", "apple"));
        registry.observe(peer("b", "Mango"));

        let names: Vec<_> = registry
            .snapshot()
            .into_iter()
            .map(|p| p.device_name)
            .collect();
        assert_eq!(names, vec!["apple", "Mango", "Zebra"]);
    }

    #[test]
    fn forgetting_removes_a_peer() {
        let registry = PeerRegistry::new("me".into());
        registry.observe(peer("them", "Laptop"));
        registry.forget("them");
        assert!(registry.snapshot().is_empty());
    }

    #[test]
    fn fresh_peers_survive_eviction() {
        let registry = PeerRegistry::new("me".into());
        registry.observe(peer("them", "Laptop"));
        registry.evict_stale();
        assert_eq!(registry.snapshot().len(), 1);
    }

    #[test]
    fn subscribers_see_the_updated_list() {
        let registry = PeerRegistry::new("me".into());
        let mut rx = registry.subscribe();

        registry.observe(peer("them", "Laptop"));

        let list = rx.borrow_and_update().clone();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].device_id, "them");
    }

    #[test]
    fn announcements_round_trip_as_json() {
        let announcement = Announcement {
            v: 1,
            kind: AnnouncementKind::Announce,
            device_id: "abc".into(),
            device_name: "Laptop".into(),
            platform: "windows".into(),
            port: DEFAULT_TRANSFER_PORT,
            fingerprint_hint: None,
        };

        let encoded = serde_json::to_vec(&announcement).unwrap();
        let decoded: Announcement = serde_json::from_slice(&encoded).unwrap();

        assert_eq!(decoded.device_id, "abc");
        assert_eq!(decoded.kind, AnnouncementKind::Announce);
        assert_eq!(decoded.port, DEFAULT_TRANSFER_PORT);
    }

    #[test]
    fn the_multicast_group_stays_link_local() {
        // 239.x is administratively scoped: routers will not forward it beyond
        // the local network. Widening this would leak device names off-site.
        assert_eq!(MULTICAST_GROUP.octets()[0], 239);
        assert!(MULTICAST_GROUP.is_multicast());
    }
}
