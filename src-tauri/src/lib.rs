//! fluqsr — peer-to-peer file transfer over the local network.
//!
//! Module map:
//!   * [`crypto`], [`device`], [`trust`] — who devices are and which ones we
//!     have vouched for.
//!   * [`tls`] — the encrypted channel, and the fingerprint check that gives
//!     it meaning.
//!   * [`discovery`] — finding peers, over two independent transports.
//!   * [`protocol`], [`transfer`] — the wire format and the transfer engine.
//!   * [`paths`] — turning sender-controlled strings into safe destinations.

pub mod commands;
pub mod crypto;
pub mod device;
pub mod discovery;
pub mod error;
pub mod paths;
pub mod protocol;
pub mod settings;
pub mod store;
pub mod tls;
pub mod transfer;
pub mod trust;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use device::Identity;
use discovery::beacon::Beacon;
use discovery::mdns::MdnsService;
use discovery::{Announcement, AnnouncementKind, PeerRegistry};
use settings::SettingsStore;
use transfer::gate::Node;
use transfer::{TransferManager, TransferProgress};
use trust::TrustStore;

impl Node {
    pub fn transfers_snapshot(&self) -> Vec<TransferProgress> {
        self.manager.list()
    }
}

pub struct AppState {
    pub node: Node,
    pub peers: Arc<PeerRegistry>,
    pub settings: Arc<SettingsStore>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Must happen before any TLS config is built.
    tls::init_crypto_provider();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let data_dir = device::data_dir(app.path().app_data_dir()?)?;

            let identity = Arc::new(Identity::load_or_create(&data_dir)?);
            let trust = Arc::new(TrustStore::load(&data_dir)?);
            let settings = Arc::new(SettingsStore::load(&data_dir, identity.device_name.clone())?);
            let peers = Arc::new(PeerRegistry::new(identity.device_id.clone()));
            let manager = Arc::new(TransferManager::new());

            let node = Node {
                identity: identity.clone(),
                trust,
                manager: manager.clone(),
            };

            app.manage(AppState {
                node: node.clone(),
                peers: peers.clone(),
                settings: settings.clone(),
            });

            let port = settings.transfer_port();

            // --- Accept incoming transfers ---------------------------------
            {
                let node = node.clone();
                let settings = settings.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(err) = transfer::recv::run_listener(node, settings, port).await {
                        eprintln!("fluqsr: the transfer listener stopped: {err}");
                    }
                });
            }

            // --- Multicast discovery ---------------------------------------
            let announcement = Announcement {
                v: 1,
                kind: AnnouncementKind::Announce,
                device_id: identity.device_id.clone(),
                device_name: settings.device_name(),
                platform: device::current_platform().to_string(),
                port,
                fingerprint_hint: Some(identity.fingerprint().to_hex()),
            };

            match Beacon::bind(peers.clone(), announcement) {
                Ok(beacon) => {
                    let beacon = Arc::new(beacon);
                    let announcer = beacon.clone();
                    tauri::async_runtime::spawn(async move { announcer.run_announcer().await });
                    tauri::async_runtime::spawn(async move { beacon.run_listener().await });
                }
                Err(err) => {
                    // Discovery failing is survivable — manual addressing still
                    // works — so this is reported rather than fatal.
                    eprintln!("fluqsr: multicast discovery unavailable: {err}");
                }
            }

            // --- mDNS discovery --------------------------------------------
            match MdnsService::start() {
                Ok(mut mdns) => {
                    if let Err(err) = mdns.advertise(
                        &identity.device_id,
                        &settings.device_name(),
                        device::current_platform(),
                        port,
                    ) {
                        eprintln!("fluqsr: could not advertise over mDNS: {err}");
                    }
                    if let Err(err) = mdns.run_browser(peers.clone(), identity.device_id.clone()) {
                        eprintln!("fluqsr: could not browse over mDNS: {err}");
                    }
                    // Held for the process lifetime; dropping it would
                    // unregister the service.
                    std::mem::forget(mdns);
                }
                Err(err) => eprintln!("fluqsr: mDNS unavailable: {err}"),
            }

            // --- Expire peers that have gone quiet -------------------------
            {
                let peers = peers.clone();
                tauri::async_runtime::spawn(async move {
                    let mut ticker = tokio::time::interval(discovery::ANNOUNCE_INTERVAL);
                    loop {
                        ticker.tick().await;
                        peers.evict_stale();
                    }
                });
            }

            // --- Forward peer changes to the UI ----------------------------
            {
                let handle = app.handle().clone();
                let mut updates = peers.subscribe();
                tauri::async_runtime::spawn(async move {
                    while updates.changed().await.is_ok() {
                        let snapshot = updates.borrow_and_update().clone();
                        let _ = handle.emit("peers://updated", snapshot);
                    }
                });
            }

            // --- Forward transfer events to the UI -------------------------
            {
                let handle = app.handle().clone();
                let mut events = manager.subscribe();
                tauri::async_runtime::spawn(async move {
                    loop {
                        match events.recv().await {
                            Ok(event) => {
                                let _ = handle.emit(event.channel(), event);
                            }
                            // Lagged means the UI fell behind a burst of
                            // progress updates. The next event carries current
                            // totals anyway, so dropping the backlog is fine.
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(_) => break,
                        }
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_self,
            commands::list_peers,
            commands::list_trusted,
            commands::list_transfers,
            commands::clear_finished_transfers,
            commands::send_to_peer,
            commands::send_to_address,
            commands::respond_to_offer,
            commands::respond_to_pairing,
            commands::cancel_transfer,
            commands::forget_peer,
            commands::set_auto_accept,
            commands::set_receive_dir,
            commands::set_device_name,
            commands::get_receive_dir,
        ])
        .run(tauri::generate_context!())
        .expect("error while running fluqsr");
}
