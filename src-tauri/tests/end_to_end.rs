//! End-to-end transfers between two real nodes.
//!
//! These stand up an actual TLS listener and dial it over loopback, so they
//! exercise the whole stack: certificate generation, mutual authentication,
//! the trust gate, framing, streaming, and hash verification. The unit tests
//! cover the pieces; these confirm the pieces are wired to each other.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use fluqsr_lib::device::Identity;
use fluqsr_lib::settings::SettingsStore;
use fluqsr_lib::tls;
use fluqsr_lib::transfer::gate::Node;
use fluqsr_lib::transfer::{
    collect_files, recv, send, OfferDecision, TransferEvent, TransferManager,
};
use fluqsr_lib::trust::TrustStore;

struct Harness {
    dir: PathBuf,
    node: Node,
    settings: Arc<SettingsStore>,
}

impl Harness {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("fluqsr-e2e-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();

        let identity = Arc::new(Identity::load_or_create(&dir).unwrap());
        let settings = Arc::new(SettingsStore::load(&dir, format!("Node {tag}")).unwrap());
        settings.set_receive_dir(dir.join("inbox")).unwrap();

        Harness {
            node: Node {
                identity,
                trust: Arc::new(TrustStore::load(&dir).unwrap()),
                manager: Arc::new(TransferManager::new()),
            },
            settings,
            dir,
        }
    }

    fn inbox(&self) -> PathBuf {
        self.settings.receive_dir()
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.dir).ok();
    }
}

/// Pin each node's key on the other, as a completed pairing would.
fn pair(a: &Harness, b: &Harness) {
    a.node
        .trust
        .pair(
            &b.node.identity.device_id,
            &b.node.identity.device_name,
            b.node.identity.fingerprint(),
        )
        .unwrap();
    b.node
        .trust
        .pair(
            &a.node.identity.device_id,
            &a.node.identity.device_name,
            a.node.identity.fingerprint(),
        )
        .unwrap();
}

/// Bind an ephemeral port, then release it — the listener re-binds immediately.
/// Avoids collisions when tests run in parallel.
fn free_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port()
}

async fn start_listener(receiver: &Harness) -> u16 {
    let port = free_port();
    let node = receiver.node.clone();
    let settings = receiver.settings.clone();

    tokio::spawn(async move {
        let _ = recv::run_listener(node, settings, port).await;
    });

    // Give the listener a moment to bind before anyone dials it.
    tokio::time::sleep(Duration::from_millis(150)).await;
    port
}

/// Answer every offer that arrives, so tests that are not about the prompt do
/// not have to deal with it.
fn auto_accept_offers(manager: Arc<TransferManager>) {
    let mut events = manager.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            if let TransferEvent::Offer(offer) = event {
                let _ = manager.resolve_offer(&offer.transfer_id, OfferDecision::Accept);
            }
        }
    });
}

async fn send_paths(sender: &Harness, port: u16, paths: &[PathBuf]) -> fluqsr_lib::error::Result<()> {
    let files = collect_files(paths).unwrap();
    let total: u64 = files.iter().map(|f| f.size).sum();
    let transfer_id = uuid::Uuid::new_v4().to_string();

    sender.node.manager.register(
        &transfer_id,
        fluqsr_lib::transfer::Direction::Send,
        "peer",
        "Receiver",
        files.len(),
        total,
    );

    send::send(
        sender.node.clone(),
        send::SendRequest {
            transfer_id,
            target: format!("127.0.0.1:{port}").parse().unwrap(),
            peer_name: "Receiver".into(),
            files,
        },
    )
    .await
}

fn write(path: &Path, contents: &[u8]) -> PathBuf {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, contents).unwrap();
    path.to_path_buf()
}

#[tokio::test(flavor = "multi_thread")]
async fn a_single_file_arrives_intact() {
    tls::init_crypto_provider();

    let sender = Harness::new("send-one");
    let receiver = Harness::new("recv-one");
    pair(&sender, &receiver);
    auto_accept_offers(receiver.node.manager.clone());

    let source = write(&sender.dir.join("outbox/hello.txt"), b"hello over the wire");
    let port = start_listener(&receiver).await;

    send_paths(&sender, port, &[source]).await.unwrap();

    let landed = receiver.inbox().join("hello.txt");
    assert!(landed.exists(), "the file should have been written");
    assert_eq!(std::fs::read(&landed).unwrap(), b"hello over the wire");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_large_file_survives_chunking() {
    // Several chunks' worth, so the streaming loop and the hash both have to
    // handle more than one pass.
    tls::init_crypto_provider();

    let sender = Harness::new("send-big");
    let receiver = Harness::new("recv-big");
    pair(&sender, &receiver);
    auto_accept_offers(receiver.node.manager.clone());

    let payload: Vec<u8> = (0..3_000_000u32).map(|n| (n % 251) as u8).collect();
    let source = write(&sender.dir.join("outbox/big.bin"), &payload);
    let port = start_listener(&receiver).await;

    send_paths(&sender, port, &[source]).await.unwrap();

    let landed = receiver.inbox().join("big.bin");
    assert_eq!(
        std::fs::read(&landed).unwrap(),
        payload,
        "a multi-chunk file must arrive byte-identical"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_folder_keeps_its_structure() {
    tls::init_crypto_provider();

    let sender = Harness::new("send-tree");
    let receiver = Harness::new("recv-tree");
    pair(&sender, &receiver);
    auto_accept_offers(receiver.node.manager.clone());

    let root = sender.dir.join("outbox/project");
    write(&root.join("README.md"), b"readme");
    write(&root.join("src/main.rs"), b"fn main() {}");
    write(&root.join("src/deep/nested.txt"), b"nested");

    let port = start_listener(&receiver).await;
    send_paths(&sender, port, &[root]).await.unwrap();

    let inbox = receiver.inbox();
    assert_eq!(std::fs::read(inbox.join("project/README.md")).unwrap(), b"readme");
    assert_eq!(
        std::fs::read(inbox.join("project/src/main.rs")).unwrap(),
        b"fn main() {}"
    );
    assert_eq!(
        std::fs::read(inbox.join("project/src/deep/nested.txt")).unwrap(),
        b"nested"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_empty_file_round_trips() {
    tls::init_crypto_provider();

    let sender = Harness::new("send-empty");
    let receiver = Harness::new("recv-empty");
    pair(&sender, &receiver);
    auto_accept_offers(receiver.node.manager.clone());

    let source = write(&sender.dir.join("outbox/empty.bin"), b"");
    let port = start_listener(&receiver).await;

    send_paths(&sender, port, &[source]).await.unwrap();

    let landed = receiver.inbox().join("empty.bin");
    assert!(landed.exists(), "a zero-byte file must still be created");
    assert_eq!(std::fs::metadata(&landed).unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread")]
async fn declining_an_offer_writes_nothing() {
    tls::init_crypto_provider();

    let sender = Harness::new("send-declined");
    let receiver = Harness::new("recv-declined");
    pair(&sender, &receiver);

    // Decline everything.
    let manager = receiver.node.manager.clone();
    let mut events = manager.subscribe();
    tokio::spawn(async move {
        while let Ok(event) = events.recv().await {
            if let TransferEvent::Offer(offer) = event {
                let _ = manager.resolve_offer(
                    &offer.transfer_id,
                    OfferDecision::Decline(Some("no thanks".into())),
                );
            }
        }
    });

    let source = write(&sender.dir.join("outbox/unwanted.txt"), b"junk");
    let port = start_listener(&receiver).await;

    let result = send_paths(&sender, port, &[source]).await;

    assert!(result.is_err(), "the sender should learn it was declined");
    assert!(
        !receiver.inbox().join("unwanted.txt").exists(),
        "declining must not leave a file behind"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_unpaired_sender_is_refused_when_pairing_is_declined() {
    tls::init_crypto_provider();

    let sender = Harness::new("send-unpaired");
    let receiver = Harness::new("recv-unpaired");
    // Deliberately not paired.

    // Refuse the pairing on both sides.
    for manager in [sender.node.manager.clone(), receiver.node.manager.clone()] {
        let mut events = manager.subscribe();
        let manager = manager.clone();
        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                if let TransferEvent::Pairing(request) = event {
                    let _ = manager.resolve_pairing(&request.request_id, false);
                }
            }
        });
    }

    let source = write(&sender.dir.join("outbox/secret.txt"), b"should not arrive");
    let port = start_listener(&receiver).await;

    let result = send_paths(&sender, port, &[source]).await;

    assert!(result.is_err(), "an unconfirmed pairing must not proceed");
    assert!(
        !receiver.inbox().join("secret.txt").exists(),
        "no file may be written before pairing is confirmed"
    );
    assert!(
        receiver.node.trust.list().is_empty(),
        "a declined pairing must not pin the peer"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_impersonator_cannot_reuse_a_trusted_device_id() {
    // The core anti-MITM property: knowing a trusted device's ID is not enough,
    // because the pinned key will not match.
    tls::init_crypto_provider();

    let genuine = Harness::new("genuine");
    let impostor = Harness::new("impostor");
    let receiver = Harness::new("recv-impostor");

    // The receiver trusts the genuine device.
    receiver
        .node
        .trust
        .pair(
            &genuine.node.identity.device_id,
            "Trusted Laptop",
            genuine.node.identity.fingerprint(),
        )
        .unwrap();

    // The impostor claims the same device ID but holds a different key. Rather
    // than fake a Hello, put the impostor's own ID into the receiver's store
    // against the genuine key — the same mismatch the gate must catch.
    receiver
        .node
        .trust
        .pair(
            &impostor.node.identity.device_id,
            "Trusted Laptop",
            genuine.node.identity.fingerprint(),
        )
        .unwrap();

    auto_accept_offers(receiver.node.manager.clone());

    // The impostor confirms whatever it is asked, as an attacker would. The
    // refusal has to come from the receiver's pin check, not from the
    // attacker's own cooperation.
    {
        let manager = impostor.node.manager.clone();
        let mut events = manager.subscribe();
        let manager = manager.clone();
        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                if let TransferEvent::Pairing(request) = event {
                    let _ = manager.resolve_pairing(&request.request_id, true);
                }
            }
        });
    }

    let source = write(&impostor.dir.join("outbox/payload.txt"), b"malicious");
    let port = start_listener(&receiver).await;

    let result = send_paths(&impostor, port, &[source]).await;

    assert!(
        result.is_err(),
        "a key that does not match the pin must be refused"
    );
    assert!(
        !receiver.inbox().join("payload.txt").exists(),
        "nothing may be written for a peer that failed identity verification"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn a_traversal_path_never_escapes_the_inbox() {
    // The sender is trusted and the user accepted — the path check is the last
    // line of defence and has to hold on its own.
    tls::init_crypto_provider();

    let sender = Harness::new("send-traversal");
    let receiver = Harness::new("recv-traversal");
    pair(&sender, &receiver);
    auto_accept_offers(receiver.node.manager.clone());

    let source = write(&sender.dir.join("outbox/normal.txt"), b"data");
    let mut files = collect_files(&[source]).unwrap();
    // Rewrite the wire path to something hostile after collection.
    files[0].relative = "../../escaped.txt".to_string();

    let port = start_listener(&receiver).await;
    let transfer_id = uuid::Uuid::new_v4().to_string();
    sender.node.manager.register(
        &transfer_id,
        fluqsr_lib::transfer::Direction::Send,
        "peer",
        "Receiver",
        1,
        4,
    );

    let result = send::send(
        sender.node.clone(),
        send::SendRequest {
            transfer_id,
            target: format!("127.0.0.1:{port}").parse().unwrap(),
            peer_name: "Receiver".into(),
            files,
        },
    )
    .await;

    assert!(result.is_err(), "the receiver must reject the traversal");

    let escaped = receiver.inbox().parent().unwrap().join("escaped.txt");
    assert!(
        !escaped.exists(),
        "a file escaped the receive folder to {}",
        escaped.display()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn an_existing_file_is_not_overwritten() {
    tls::init_crypto_provider();

    let sender = Harness::new("send-collide");
    let receiver = Harness::new("recv-collide");
    pair(&sender, &receiver);
    auto_accept_offers(receiver.node.manager.clone());

    std::fs::create_dir_all(receiver.inbox()).unwrap();
    std::fs::write(receiver.inbox().join("notes.txt"), b"mine").unwrap();

    let source = write(&sender.dir.join("outbox/notes.txt"), b"theirs");
    let port = start_listener(&receiver).await;

    send_paths(&sender, port, &[source]).await.unwrap();

    assert_eq!(
        std::fs::read(receiver.inbox().join("notes.txt")).unwrap(),
        b"mine",
        "a sender must not be able to replace an existing file"
    );
    assert_eq!(
        std::fs::read(receiver.inbox().join("notes (1).txt")).unwrap(),
        b"theirs"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn many_small_files_all_arrive() {
    // Exercises the pipelining path, where per-file overhead dominates.
    tls::init_crypto_provider();

    let sender = Harness::new("send-many");
    let receiver = Harness::new("recv-many");
    pair(&sender, &receiver);
    auto_accept_offers(receiver.node.manager.clone());

    let root = sender.dir.join("outbox/many");
    for n in 0..200 {
        write(&root.join(format!("file-{n:03}.txt")), format!("body {n}").as_bytes());
    }

    let port = start_listener(&receiver).await;
    send_paths(&sender, port, &[root]).await.unwrap();

    let landed = std::fs::read_dir(receiver.inbox().join("many")).unwrap().count();
    assert_eq!(landed, 200, "every file in the batch should arrive");
    assert_eq!(
        std::fs::read(receiver.inbox().join("many/file-137.txt")).unwrap(),
        b"body 137"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn pairing_by_confirmation_succeeds_and_pins_both_sides() {
    tls::init_crypto_provider();

    let sender = Harness::new("send-pairing");
    let receiver = Harness::new("recv-pairing");

    // Confirm the pairing on both ends, as a user comparing matching codes
    // would, and capture what each side was shown.
    let codes = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));
    for manager in [sender.node.manager.clone(), receiver.node.manager.clone()] {
        let mut events = manager.subscribe();
        let manager = manager.clone();
        let codes = codes.clone();
        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                match event {
                    TransferEvent::Pairing(request) => {
                        codes.lock().unwrap().push(request.pairing_code.clone());
                        let _ = manager.resolve_pairing(&request.request_id, true);
                    }
                    TransferEvent::Offer(offer) => {
                        let _ = manager.resolve_offer(&offer.transfer_id, OfferDecision::Accept);
                    }
                    _ => {}
                }
            }
        });
    }

    let source = write(&sender.dir.join("outbox/first.txt"), b"first contact");
    let port = start_listener(&receiver).await;

    send_paths(&sender, port, &[source]).await.unwrap();

    assert_eq!(
        std::fs::read(receiver.inbox().join("first.txt")).unwrap(),
        b"first contact"
    );

    assert!(
        sender.node.trust.is_trusted(
            &receiver.node.identity.device_id,
            &receiver.node.identity.fingerprint()
        ),
        "the sender should have pinned the receiver"
    );
    assert!(
        receiver.node.trust.is_trusted(
            &sender.node.identity.device_id,
            &sender.node.identity.fingerprint()
        ),
        "the receiver should have pinned the sender"
    );

    let shown = codes.lock().unwrap().clone();
    assert_eq!(shown.len(), 2, "both devices should have prompted");
    assert_eq!(
        shown[0], shown[1],
        "both screens must display the same code, or the SAS check is meaningless"
    );
}
