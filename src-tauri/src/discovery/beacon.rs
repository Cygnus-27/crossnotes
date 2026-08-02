//! UDP multicast heartbeat.
//!
//! Every device joins one multicast group and announces itself on a timer.
//! When a device hears an announcement from a peer it has not seen, it answers
//! directly over unicast so the newcomer learns about it right away instead of
//! waiting up to a full interval.
//!
//! Replies are never replied to. Without that rule two devices would answer
//! each other's answers indefinitely.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;

use socket2::{Domain, Protocol, Socket, Type};
use tokio::net::UdpSocket;

use super::{
    Announcement, AnnouncementKind, DiscoveredPeer, DiscoverySource, PeerRegistry,
    ANNOUNCE_INTERVAL, DISCOVERY_PORT, MULTICAST_GROUP,
};
use crate::error::{Error, Result};

/// Announcements are small; anything larger is not ours.
const MAX_DATAGRAM: usize = 2048;

pub struct Beacon {
    socket: Arc<UdpSocket>,
    registry: Arc<PeerRegistry>,
    self_announcement: Announcement,
}

impl Beacon {
    pub fn bind(registry: Arc<PeerRegistry>, self_announcement: Announcement) -> Result<Self> {
        let socket = bind_multicast_socket().map_err(|err| {
            Error::Discovery(format!(
                "could not open the discovery socket on port {DISCOVERY_PORT}: {err}. \
                 Another copy of fluqsr may already be running, or a firewall may be blocking it."
            ))
        })?;

        Ok(Beacon {
            socket: Arc::new(socket),
            registry,
            self_announcement,
        })
    }

    /// Announce on a timer until cancelled.
    pub async fn run_announcer(&self) {
        let target = SocketAddr::from(SocketAddrV4::new(MULTICAST_GROUP, DISCOVERY_PORT));
        let mut ticker = tokio::time::interval(ANNOUNCE_INTERVAL);

        loop {
            ticker.tick().await;

            let mut announcement = self.self_announcement.clone();
            announcement.kind = AnnouncementKind::Announce;

            if let Ok(payload) = serde_json::to_vec(&announcement) {
                // A send failure here is routine, not exceptional: the network
                // may be down, or no interface may have joined the group yet.
                // The next tick tries again.
                let _ = self.socket.send_to(&payload, target).await;
            }
        }
    }

    /// Listen for announcements until cancelled.
    pub async fn run_listener(&self) {
        let mut buffer = vec![0u8; MAX_DATAGRAM];

        loop {
            let (len, source) = match self.socket.recv_from(&mut buffer).await {
                Ok(result) => result,
                // One malformed or oversized datagram must not kill discovery.
                Err(_) => continue,
            };

            let Ok(announcement) = serde_json::from_slice::<Announcement>(&buffer[..len]) else {
                continue;
            };

            self.handle(announcement, source).await;
        }
    }

    async fn handle(&self, announcement: Announcement, source: SocketAddr) {
        if announcement.device_id == self.self_announcement.device_id {
            return;
        }

        if announcement.kind == AnnouncementKind::Goodbye {
            self.registry.forget(&announcement.device_id);
            return;
        }

        let peer = DiscoveredPeer {
            device_id: announcement.device_id.clone(),
            // Trim and bound the name: it is attacker-controlled and goes
            // straight into the UI.
            device_name: clean_label(&announcement.device_name, "Unknown device"),
            platform: clean_label(&announcement.platform, "unknown"),
            // The source address is what we can actually reach, unlike any
            // address the peer might have put in the payload.
            address: source.ip(),
            port: announcement.port,
            source: DiscoverySource::Beacon,
            last_seen_secs: 0,
            fingerprint_hint: announcement.fingerprint_hint,
        };

        let is_new = self.registry.observe(peer);

        // Answer a newcomer directly so it does not have to wait for our next
        // scheduled announcement. Only answer announcements, never replies.
        if is_new && announcement.kind == AnnouncementKind::Announce {
            let mut reply = self.self_announcement.clone();
            reply.kind = AnnouncementKind::Reply;
            if let Ok(payload) = serde_json::to_vec(&reply) {
                let _ = self.socket.send_to(&payload, source).await;
            }
        }
    }

    /// Best-effort "I'm leaving" so peers remove us promptly.
    pub async fn send_goodbye(&self) {
        let mut goodbye = self.self_announcement.clone();
        goodbye.kind = AnnouncementKind::Goodbye;

        if let Ok(payload) = serde_json::to_vec(&goodbye) {
            let target = SocketAddr::from(SocketAddrV4::new(MULTICAST_GROUP, DISCOVERY_PORT));
            let _ = self.socket.send_to(&payload, target).await;
        }
    }
}

/// Bind a UDP socket suitable for multicast discovery.
///
/// `SO_REUSEADDR` (and `SO_REUSEPORT` where it exists) must be set *before*
/// bind, which is why this goes through socket2 rather than tokio directly.
/// Without it, a second instance on the same machine — the normal case while
/// developing — fails to start.
fn bind_multicast_socket() -> std::io::Result<UdpSocket> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, Some(Protocol::UDP))?;

    socket.set_reuse_address(true)?;
    #[cfg(unix)]
    socket.set_reuse_port(true)?;

    let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, DISCOVERY_PORT);
    socket.bind(&bind_addr.into())?;

    // Deliberately on: hearing our own packets makes two instances on one
    // machine discover each other, and the registry filters us out by device
    // ID anyway.
    socket.set_multicast_loop_v4(true)?;

    // TTL 1 keeps announcements on the local segment.
    socket.set_multicast_ttl_v4(1)?;

    // Joining on the unspecified interface lets the OS pick. On a machine with
    // several interfaces this may only cover the default one — a known
    // limitation, and the reason manual peer entry exists.
    socket.join_multicast_v4(&MULTICAST_GROUP, &Ipv4Addr::UNSPECIFIED)?;

    socket.set_nonblocking(true)?;

    UdpSocket::from_std(socket.into())
}

/// Bound and sanitize a label that arrived over the network before it reaches
/// the UI. Control characters and unbounded length are the concerns here.
fn clean_label(value: &str, fallback: &str) -> String {
    let cleaned: String = value
        .chars()
        .filter(|ch| !ch.is_control())
        .take(64)
        .collect();

    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn labels_are_bounded_and_stripped() {
        assert_eq!(clean_label("Laptop", "fallback"), "Laptop");
        assert_eq!(clean_label("  spaced  ", "fallback"), "spaced");
        assert_eq!(clean_label("", "fallback"), "fallback");
        assert_eq!(clean_label("   ", "fallback"), "fallback");
    }

    #[test]
    fn labels_drop_control_characters() {
        // The escape byte and the bell are removed; the printable remainder of
        // an ANSI sequence is left alone, which is fine — the peer list is
        // rendered as HTML text, not written to a terminal.
        let cleaned = clean_label("Lap\u{1b}top\u{7}", "fallback");
        assert_eq!(cleaned, "Laptop");
        assert!(!cleaned.chars().any(|c| c.is_control()));
    }

    #[test]
    fn labels_cannot_smuggle_newlines_or_nulls() {
        let cleaned = clean_label("Laptop\n\r\0evil", "fallback");
        assert_eq!(cleaned, "Laptopevil");
    }

    #[test]
    fn absurdly_long_labels_are_truncated() {
        let cleaned = clean_label(&"a".repeat(10_000), "fallback");
        assert_eq!(cleaned.chars().count(), 64);
    }

    // These need a Tokio reactor: `UdpSocket::from_std` registers the socket
    // with the running runtime.

    #[tokio::test]
    async fn the_socket_binds() {
        // Guards the socket option ordering, which is easy to get wrong and
        // fails only at runtime.
        if let Err(err) = bind_multicast_socket() {
            // Sandboxes often have no multicast-capable interface.
            eprintln!("skipping: multicast unavailable in this environment: {err}");
        }
    }

    #[tokio::test]
    async fn two_sockets_can_share_the_port() {
        // Two app instances on one machine is the normal development setup, so
        // SO_REUSEADDR must actually be taking effect.
        let Ok(_first) = bind_multicast_socket() else {
            eprintln!("skipping: multicast unavailable in this environment");
            return;
        };
        assert!(
            bind_multicast_socket().is_ok(),
            "a second instance must be able to bind the discovery port"
        );
    }
}
