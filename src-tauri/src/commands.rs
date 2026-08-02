//! The command surface the frontend calls into.
//!
//! These are thin on purpose: parse and validate arguments, hand off to the
//! module that owns the behaviour, and translate errors. No transfer logic or
//! trust decisions live here.

use std::net::SocketAddr;
use std::path::PathBuf;

use serde::Serialize;
use tauri::State;

use crate::device::DeviceInfo;
use crate::discovery::{DiscoveredPeer, DEFAULT_TRANSFER_PORT};
use crate::error::{Error, Result};
use crate::settings::Settings;
use crate::transfer::{
    collect_files, send, Direction, OfferDecision, TransferProgress,
};
use crate::trust::TrustedPeer;
use crate::AppState;

/// Everything the UI needs to render the "this device" panel.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelfView {
    pub device: DeviceInfo,
    /// Short form of our own fingerprint, for the user to read aloud when
    /// verifying a pairing over a phone call.
    pub short_fingerprint: String,
    pub settings: Settings,
}

#[tauri::command]
pub fn get_self(state: State<'_, AppState>) -> SelfView {
    let mut device = state.node.identity.info();
    // The display name is user-editable and lives in settings; the identity
    // file keeps whatever it was created with.
    device.device_name = state.settings.device_name();

    SelfView {
        short_fingerprint: state.node.identity.fingerprint().to_short(),
        device,
        settings: state.settings.get(),
    }
}

#[tauri::command]
pub fn list_peers(state: State<'_, AppState>) -> Vec<DiscoveredPeer> {
    state.peers.snapshot()
}

#[tauri::command]
pub fn list_trusted(state: State<'_, AppState>) -> Vec<TrustedPeer> {
    state.node.trust.list()
}

#[tauri::command]
pub fn list_transfers(state: State<'_, AppState>) -> Vec<TransferProgress> {
    state.node.transfers_snapshot()
}

#[tauri::command]
pub fn clear_finished_transfers(state: State<'_, AppState>) {
    state.node.manager.clear_finished();
}

/// Send to a peer we discovered on the network.
#[tauri::command]
pub async fn send_to_peer(
    state: State<'_, AppState>,
    device_id: String,
    paths: Vec<String>,
) -> Result<String> {
    let peer = state
        .peers
        .get(&device_id)
        .ok_or_else(|| Error::UnknownPeer(device_id.clone()))?;

    let target = SocketAddr::new(peer.address, peer.port);
    spawn_send(&state, target, peer.device_name, paths)
}

/// Send to a manually entered address, for networks where discovery is
/// blocked. Accepts `host` or `host:port`.
#[tauri::command]
pub async fn send_to_address(
    state: State<'_, AppState>,
    address: String,
    paths: Vec<String>,
) -> Result<String> {
    let target = parse_target(&address)?;
    spawn_send(&state, target, address, paths)
}

fn spawn_send(
    state: &AppState,
    target: SocketAddr,
    peer_label: String,
    paths: Vec<String>,
) -> Result<String> {
    if paths.is_empty() {
        return Err(Error::Other("no files were selected".into()));
    }

    let selection: Vec<PathBuf> = paths.into_iter().map(PathBuf::from).collect();
    let files = collect_files(&selection)?;

    if files.is_empty() {
        return Err(Error::Other(
            "the selection contained no files to send".into(),
        ));
    }

    let transfer_id = uuid::Uuid::new_v4().to_string();
    let total_bytes = files.iter().map(|file| file.size).sum();

    state.node.manager.register(
        &transfer_id,
        Direction::Send,
        "",
        &peer_label,
        files.len(),
        total_bytes,
    );

    let node = state.node.clone();
    let request = send::SendRequest {
        transfer_id: transfer_id.clone(),
        target,
        peer_name: peer_label,
        files,
    };

    // Detached: the command returns as soon as the transfer is registered, and
    // the UI follows it through progress events instead of a blocked call.
    tauri::async_runtime::spawn(async move {
        let _ = send::send(node, request).await;
    });

    Ok(transfer_id)
}

#[tauri::command]
pub fn respond_to_offer(
    state: State<'_, AppState>,
    transfer_id: String,
    accept: bool,
) -> Result<()> {
    let decision = if accept {
        OfferDecision::Accept
    } else {
        OfferDecision::Decline(Some("declined on the other device".into()))
    };
    state.node.manager.resolve_offer(&transfer_id, decision)
}

#[tauri::command]
pub fn respond_to_pairing(
    state: State<'_, AppState>,
    request_id: String,
    confirmed: bool,
) -> Result<()> {
    state.node.manager.resolve_pairing(&request_id, confirmed)
}

#[tauri::command]
pub fn cancel_transfer(state: State<'_, AppState>, transfer_id: String) -> Result<()> {
    state.node.manager.cancel(&transfer_id)
}

#[tauri::command]
pub fn forget_peer(state: State<'_, AppState>, device_id: String) -> Result<()> {
    state.node.trust.forget(&device_id)
}

#[tauri::command]
pub fn set_auto_accept(
    state: State<'_, AppState>,
    device_id: String,
    enabled: bool,
) -> Result<()> {
    state.node.trust.set_auto_accept(&device_id, enabled)
}

#[tauri::command]
pub fn set_receive_dir(state: State<'_, AppState>, path: String) -> Result<Settings> {
    state.settings.set_receive_dir(PathBuf::from(path))?;
    Ok(state.settings.get())
}

#[tauri::command]
pub fn set_device_name(state: State<'_, AppState>, name: String) -> Result<Settings> {
    state.settings.set_device_name(name)?;
    Ok(state.settings.get())
}

#[tauri::command]
pub fn get_receive_dir(state: State<'_, AppState>) -> String {
    state.settings.receive_dir().to_string_lossy().to_string()
}

/// Parse `host` or `host:port` into a socket address.
fn parse_target(input: &str) -> Result<SocketAddr> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(Error::Other("enter an address".into()));
    }

    if let Ok(addr) = trimmed.parse::<SocketAddr>() {
        return Ok(addr);
    }

    if let Ok(ip) = trimmed.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(ip, DEFAULT_TRANSFER_PORT));
    }

    Err(Error::Other(format!(
        "{trimmed} is not a valid address. Use an IP like 192.168.1.42, \
         optionally with a port: 192.168.1.42:{DEFAULT_TRANSFER_PORT}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn a_bare_ip_gets_the_default_port() {
        let target = parse_target("192.168.1.42").unwrap();
        assert_eq!(target.ip(), IpAddr::V4(Ipv4Addr::new(192, 168, 1, 42)));
        assert_eq!(target.port(), DEFAULT_TRANSFER_PORT);
    }

    #[test]
    fn an_explicit_port_is_honoured() {
        assert_eq!(parse_target("192.168.1.42:9000").unwrap().port(), 9000);
    }

    #[test]
    fn surrounding_whitespace_is_tolerated() {
        assert!(parse_target("  192.168.1.42  ").is_ok());
    }

    #[test]
    fn ipv6_is_accepted() {
        assert!(parse_target("[::1]:47654").is_ok());
        assert!(parse_target("::1").is_ok());
    }

    #[test]
    fn nonsense_is_rejected_with_a_usable_message() {
        for bad in ["", "   ", "not an address", "999.999.999.999"] {
            let err = parse_target(bad).unwrap_err().to_string();
            assert!(!err.is_empty(), "{bad} should produce an error");
        }
    }
}
