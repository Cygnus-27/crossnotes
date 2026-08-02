//! The trust gate every connection passes through, in both directions.
//!
//! This is the single place where "a TLS handshake completed" is converted
//! into "we know who this is and the user agreed to talk to them". Both
//! [`super::send`] and [`super::recv`] call it before acting on a connection,
//! and neither does its own trust reasoning.

use std::sync::Arc;

use crate::crypto::{pairing_code, Fingerprint};
use crate::device::Identity;
use crate::error::{Error, Result};
use crate::protocol::Hello;
use crate::trust::{TrustStore, TrustVerdict, TrustedPeer};

use super::{Direction, IdentityWarning, PairingRequest, TransferManager};

/// Everything the gate needs. Shared with the send and receive paths so they
/// do not each thread the same four handles through their signatures.
#[derive(Clone)]
pub struct Node {
    pub identity: Arc<Identity>,
    pub trust: Arc<TrustStore>,
    pub manager: Arc<TransferManager>,
}

/// Decide whether we may proceed with this peer.
///
/// Three outcomes, and the difference between them matters:
///
/// * **pinned key matches** — proceed silently. This is the common path.
/// * **pinned key differs** — refuse, and raise a distinct warning. Either the
///   peer reinstalled or someone is impersonating it, and we cannot tell which
///   from here, so a human must. Notably this is *not* downgraded to a fresh
///   pairing prompt: an attacker who could trigger one would only need the
///   user to click through a dialog they see routinely.
/// * **unknown device** — ask the user to compare a six-digit code with the
///   other screen, then pin on confirmation.
pub async fn authenticate(
    node: &Node,
    presented: Fingerprint,
    hello: &Hello,
    direction: Direction,
) -> Result<TrustedPeer> {
    match node.trust.verify(&hello.device_id, &presented) {
        TrustVerdict::Trusted(peer) => {
            node.trust.refresh_name(&hello.device_id, &hello.device_name);
            Ok(*peer)
        }

        TrustVerdict::Mismatch { expected } => {
            node.manager.warn_identity_mismatch(IdentityWarning {
                peer_device_id: hello.device_id.clone(),
                peer_name: hello.device_name.clone(),
                expected_fingerprint: expected.to_hex(),
                presented_fingerprint: presented.to_hex(),
            });

            Err(Error::IdentityMismatch {
                device_id: hello.device_id.clone(),
                expected: expected.to_short(),
                actual: presented.to_short(),
            })
        }

        TrustVerdict::Unknown => {
            // Derived from both fingerprints, so a man in the middle — who
            // necessarily holds a different key toward each side — cannot make
            // the two screens agree.
            let code = pairing_code(&node.identity.fingerprint(), &presented);

            let confirmed = node
                .manager
                .await_pairing_decision(PairingRequest {
                    request_id: uuid::Uuid::new_v4().to_string(),
                    peer_device_id: hello.device_id.clone(),
                    peer_name: hello.device_name.clone(),
                    peer_fingerprint: presented.to_hex(),
                    pairing_code: code,
                    direction,
                })
                .await;

            if !confirmed {
                return Err(Error::Declined(Some("pairing was not confirmed".into())));
            }

            node.trust
                .pair(&hello.device_id, &hello.device_name, presented)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::DeviceKey;
    use crate::protocol::PROTOCOL_VERSION;
    use std::path::{Path, PathBuf};

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fluqsr-gate-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn node(dir: &Path) -> Node {
        Node {
            identity: Arc::new(Identity::load_or_create(dir).unwrap()),
            trust: Arc::new(TrustStore::load(dir).unwrap()),
            manager: Arc::new(TransferManager::new()),
        }
    }

    fn hello(device_id: &str) -> Hello {
        Hello {
            protocol_version: PROTOCOL_VERSION,
            device_id: device_id.into(),
            device_name: "Peer Laptop".into(),
            platform: "linux".into(),
        }
    }

    #[tokio::test]
    async fn a_pinned_peer_passes_without_prompting() {
        let dir = temp_dir("pinned");
        let node = node(&dir);
        let peer_fp = DeviceKey::generate().unwrap().fingerprint();

        node.trust.pair("peer", "Peer Laptop", peer_fp).unwrap();

        let result = authenticate(&node, peer_fp, &hello("peer"), Direction::Receive).await;
        assert!(result.is_ok(), "a pinned peer must not require re-approval");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_substituted_key_is_refused_outright() {
        // The impersonation case. This must fail, and must not fall back to a
        // pairing prompt the user might click through.
        let dir = temp_dir("substituted");
        let node = node(&dir);

        let genuine = DeviceKey::generate().unwrap().fingerprint();
        let attacker = DeviceKey::generate().unwrap().fingerprint();
        node.trust.pair("peer", "Peer Laptop", genuine).unwrap();

        let result = authenticate(&node, attacker, &hello("peer"), Direction::Receive).await;

        assert!(matches!(result, Err(Error::IdentityMismatch { .. })));
        assert_eq!(
            node.trust.get("peer").unwrap().fingerprint,
            genuine,
            "a refused connection must not overwrite the pin"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_unknown_peer_is_pinned_once_confirmed() {
        let dir = temp_dir("confirm");
        let node = node(&dir);
        let peer_fp = DeviceKey::generate().unwrap().fingerprint();

        let waiter = {
            let node = node.clone();
            tokio::spawn(async move {
                authenticate(&node, peer_fp, &hello("peer"), Direction::Receive).await
            })
        };

        // Approve whatever pairing request appears.
        let manager = node.manager.clone();
        let mut events = manager.subscribe();
        let request_id = loop {
            if let Ok(super::super::TransferEvent::Pairing(request)) = events.recv().await {
                break request.request_id;
            }
        };
        manager.resolve_pairing(&request_id, true).unwrap();

        assert!(waiter.await.unwrap().is_ok());
        assert!(node.trust.is_trusted("peer", &peer_fp));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_declined_pairing_pins_nothing() {
        let dir = temp_dir("decline");
        let node = node(&dir);
        let peer_fp = DeviceKey::generate().unwrap().fingerprint();

        let waiter = {
            let node = node.clone();
            tokio::spawn(async move {
                authenticate(&node, peer_fp, &hello("peer"), Direction::Receive).await
            })
        };

        let manager = node.manager.clone();
        let mut events = manager.subscribe();
        let request_id = loop {
            if let Ok(super::super::TransferEvent::Pairing(request)) = events.recv().await {
                break request.request_id;
            }
        };
        manager.resolve_pairing(&request_id, false).unwrap();

        assert!(matches!(waiter.await.unwrap(), Err(Error::Declined(_))));
        assert!(
            node.trust.get("peer").is_none(),
            "declining must leave the trust store untouched"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn both_sides_of_a_pairing_see_the_same_code() {
        // The property the whole SAS check rests on.
        let dir_a = temp_dir("code-a");
        let dir_b = temp_dir("code-b");
        let a = Identity::load_or_create(&dir_a).unwrap();
        let b = Identity::load_or_create(&dir_b).unwrap();

        assert_eq!(
            pairing_code(&a.fingerprint(), &b.fingerprint()),
            pairing_code(&b.fingerprint(), &a.fingerprint())
        );

        std::fs::remove_dir_all(&dir_a).ok();
        std::fs::remove_dir_all(&dir_b).ok();
    }
}
