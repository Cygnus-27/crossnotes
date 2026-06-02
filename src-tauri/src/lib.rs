// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeSet, HashSet},
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Mutex,
    },
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Emitter, Manager, State};

const SYNC_SERVICE_TYPE: &str = "_crossnotes._tcp.local.";
const DEFAULT_SYNC_PORT: u16 = 37642;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AttachedFile {
    file_name: String,
    relative_path: String,
    is_image: bool,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct SyncManifest {
    selected_notes: Vec<String>,
    last_triggered_at: Option<u64>,
    last_export_path: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncTriggerResult {
    selected_count: usize,
    exported_count: usize,
    export_path: String,
    manifest_path: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct DeviceIdentity {
    device_id: String,
    device_name: String,
    created_at: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPackageManifest {
    package_version: u8,
    source_device: DeviceIdentity,
    created_at: u64,
    files: Vec<SyncPackageFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncPackageFile {
    relative_path: String,
    size_bytes: u64,
    modified_at: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncImportResult {
    source_device_id: String,
    imported_count: usize,
    skipped_count: usize,
    conflict_count: usize,
    conflicts: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LanSendResult {
    sent_count: usize,
    peer_addr: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct SyncPeer {
    device_id: String,
    device_name: String,
    host: String,
    port: u16,
    paired: bool,
}

#[derive(Serialize, Deserialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct TrustedDevice {
    device_id: String,
    device_name: String,
    paired_at: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SyncStartResult {
    port: u16,
    device_id: String,
    device_name: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PairRequest {
    code: String,
    device: DeviceIdentity,
}

/// Process-wide sync runtime shared between Tauri commands and the
/// background TCP / mDNS threads.
#[derive(Default)]
struct SyncRuntime {
    active_vault: Mutex<Option<PathBuf>>,
    peers: Mutex<Vec<SyncPeer>>,
    pending_pair_code: Mutex<Option<String>>,
    receiver_started: AtomicBool,
    discovery_started: AtomicBool,
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn open_in_file_manager(path: String) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    let status = Command::new("explorer")
        .arg(path)
        .status()
        .map_err(|err| format!("failed to launch explorer: {err}"))?;

    #[cfg(target_os = "macos")]
    let status = Command::new("open")
        .arg(path)
        .status()
        .map_err(|err| format!("failed to launch Finder: {err}"))?;

    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("xdg-open")
        .arg(path)
        .status()
        .map_err(|err| format!("failed to launch file manager: {err}"))?;

    if status.success() {
        Ok(())
    } else {
        Err(format!("file manager exited with status: {status}"))
    }
}

#[tauri::command]
fn get_default_vault(app: tauri::AppHandle) -> Result<String, String> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;
    let vault_path = app_data_dir.join("Default Vault");
    fs::create_dir_all(&vault_path)
        .map_err(|err| format!("failed to create default vault: {err}"))?;
    ensure_crossnotes_dir(&vault_path)?;
    write_sync_manifest(
        &vault_path,
        &read_sync_manifest(&vault_path).unwrap_or_default(),
    )?;
    read_device_identity(&vault_path)?;
    Ok(path_to_string(&vault_path))
}

#[tauri::command]
fn attach_file_to_vault(source_path: String, vault_path: String) -> Result<AttachedFile, String> {
    let source = Path::new(&source_path);
    if !source.is_file() {
        return Err(format!("source is not a file: {source_path}"));
    }

    let vault = Path::new(&vault_path);
    if !vault.is_dir() {
        return Err(format!("vault is not a directory: {vault_path}"));
    }

    let attachments_dir = vault.join("Attachments");
    fs::create_dir_all(&attachments_dir)
        .map_err(|err| format!("failed to create Attachments folder: {err}"))?;

    let original_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "source file has no valid UTF-8 filename".to_string())?;
    let safe_name = sanitize_file_name(original_name);
    let destination_name = unique_file_name(&attachments_dir, &safe_name);
    let destination = attachments_dir.join(&destination_name);

    fs::copy(source, &destination).map_err(|err| format!("failed to copy attachment: {err}"))?;

    Ok(AttachedFile {
        file_name: destination_name.clone(),
        relative_path: format!("Attachments/{destination_name}"),
        is_image: is_image_file(&destination_name),
    })
}

#[tauri::command]
fn create_vault(parent_path: String, vault_name: String) -> Result<String, String> {
    let parent = Path::new(&parent_path);
    if !parent.is_dir() {
        return Err(format!("parent path is not a directory: {parent_path}"));
    }

    let safe_name = sanitize_file_name(&vault_name);
    if safe_name.is_empty() || safe_name == "attachment" {
        return Err("vault name must contain at least one valid character".to_string());
    }

    let vault_path = parent.join(safe_name);
    if vault_path.exists() {
        return Err(format!(
            "a file or folder already exists at {}",
            vault_path.display()
        ));
    }

    fs::create_dir_all(&vault_path).map_err(|err| format!("failed to create vault: {err}"))?;
    ensure_crossnotes_dir(&vault_path)?;
    write_sync_manifest(&vault_path, &SyncManifest::default())?;
    read_device_identity(&vault_path)?;

    Ok(path_to_string(&vault_path))
}

#[tauri::command]
fn get_sync_manifest(vault_path: String) -> Result<SyncManifest, String> {
    let vault = validate_vault(&vault_path)?;
    read_sync_manifest(&vault)
}

#[tauri::command]
fn get_device_identity(vault_path: String) -> Result<DeviceIdentity, String> {
    let vault = validate_vault(&vault_path)?;
    read_device_identity(&vault)
}

#[tauri::command]
fn set_note_sync_enabled(
    vault_path: String,
    note_path: String,
    enabled: bool,
) -> Result<SyncManifest, String> {
    let vault = validate_vault(&vault_path)?;
    let relative_note_path = relative_note_path(&vault, Path::new(&note_path))?;
    let mut manifest = read_sync_manifest(&vault)?;
    let mut selected_notes = manifest
        .selected_notes
        .into_iter()
        .collect::<BTreeSet<String>>();

    if enabled {
        selected_notes.insert(relative_note_path);
    } else {
        selected_notes.remove(&relative_note_path);
    }

    manifest.selected_notes = selected_notes.into_iter().collect();
    write_sync_manifest(&vault, &manifest)?;
    Ok(manifest)
}

#[tauri::command]
fn trigger_sync(vault_path: String) -> Result<SyncTriggerResult, String> {
    let vault = validate_vault(&vault_path)?;
    let mut manifest = read_sync_manifest(&vault)?;
    let device = read_device_identity(&vault)?;
    let created_at = unix_timestamp()?;
    let sync_root = ensure_crossnotes_dir(&vault)?.join("sync-out");
    let sync_dir = sync_root.join(format!("{}-{created_at}", device.device_id));

    fs::create_dir_all(&sync_dir).map_err(|err| format!("failed to create sync package: {err}"))?;

    let mut exported_count = 0;
    let mut package_files = Vec::new();
    let mut staged: HashSet<String> = HashSet::new();
    for relative_path in &manifest.selected_notes {
        validate_relative_sync_path(relative_path)?;
        let source = vault.join(relative_path);
        if !source.is_file() {
            continue;
        }

        if stage_sync_file(&vault, &sync_dir, relative_path, &mut package_files, &mut staged)? {
            exported_count += 1;
        }

        // Pull in any attachments the note links to so the references survive.
        if let Ok(content) = fs::read_to_string(&source) {
            for attachment in collect_attachment_paths(&content) {
                if validate_relative_sync_path(&attachment).is_err() {
                    continue;
                }
                if vault.join(&attachment).is_file() {
                    stage_sync_file(
                        &vault,
                        &sync_dir,
                        &attachment,
                        &mut package_files,
                        &mut staged,
                    )?;
                }
            }
        }
    }

    let package_manifest = SyncPackageManifest {
        package_version: 1,
        source_device: device,
        created_at,
        files: package_files,
    };
    write_json_file(
        &sync_dir.join("crossnotes-sync-package.json"),
        &package_manifest,
    )?;

    manifest.last_triggered_at = Some(created_at);
    manifest.last_export_path = Some(path_to_string(&sync_dir));
    write_sync_manifest(&vault, &manifest)?;

    Ok(SyncTriggerResult {
        selected_count: manifest.selected_notes.len(),
        exported_count,
        export_path: path_to_string(&sync_dir),
        manifest_path: path_to_string(&sync_manifest_path(&vault)),
    })
}

#[tauri::command]
fn import_sync_package(
    vault_path: String,
    package_path: String,
) -> Result<SyncImportResult, String> {
    let vault = validate_vault(&vault_path)?;
    let package = PathBuf::from(&package_path);
    if !package.is_dir() {
        return Err(format!("sync package is not a folder: {package_path}"));
    }

    let manifest_path = package.join("crossnotes-sync-package.json");
    if !manifest_path.is_file() {
        return Err("selected folder is missing crossnotes-sync-package.json".to_string());
    }

    let package_manifest: SyncPackageManifest = read_json_file(&manifest_path)?;
    if package_manifest.package_version != 1 {
        return Err(format!(
            "unsupported sync package version: {}",
            package_manifest.package_version
        ));
    }

    let local_device = read_device_identity(&vault)?;
    if package_manifest.source_device.device_id == local_device.device_id {
        return Err("this package was created by this vault/device".to_string());
    }

    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut conflicts = Vec::new();

    for file in package_manifest.files {
        validate_relative_sync_path(&file.relative_path)?;
        let source = package.join(&file.relative_path);
        if !source.is_file() {
            skipped_count += 1;
            continue;
        }

        let incoming = fs::read(&source)
            .map_err(|err| format!("failed to read incoming file {}: {err}", source.display()))?;
        let destination = vault.join(&file.relative_path);
        match apply_incoming_file(
            &destination,
            &incoming,
            &package_manifest.source_device.device_id,
            package_manifest.created_at,
        )? {
            ImportOutcome::Imported => imported_count += 1,
            ImportOutcome::Skipped => skipped_count += 1,
            ImportOutcome::Conflict(path) => conflicts.push(path),
        }
    }

    Ok(SyncImportResult {
        source_device_id: package_manifest.source_device.device_id,
        imported_count,
        skipped_count,
        conflict_count: conflicts.len(),
        conflicts,
    })
}

enum ImportOutcome {
    Imported,
    Skipped,
    Conflict(String),
}

/// Decide what to do with one incoming file: skip if identical, write a
/// conflict copy if it differs from an existing note, otherwise import it.
/// All writes are atomic.
fn apply_incoming_file(
    destination: &Path,
    incoming: &[u8],
    source_device_id: &str,
    created_at: u64,
) -> Result<ImportOutcome, String> {
    if destination.exists() {
        let existing = fs::read(destination).map_err(|err| {
            format!(
                "failed to read existing file {}: {err}",
                destination.display()
            )
        })?;

        if existing == incoming {
            return Ok(ImportOutcome::Skipped);
        }

        let conflict_path = conflict_file_path(destination, source_device_id, created_at);
        write_atomic(&conflict_path, incoming)?;
        return Ok(ImportOutcome::Conflict(path_to_string(&conflict_path)));
    }

    write_atomic(destination, incoming)?;
    Ok(ImportOutcome::Imported)
}

#[tauri::command]
fn set_active_vault(state: State<'_, SyncRuntime>, vault_path: String) -> Result<(), String> {
    let vault = validate_vault(&vault_path)?;
    *state.active_vault.lock().unwrap() = Some(vault);
    Ok(())
}

/// Start the LAN sync stack for the active vault: a TCP receiver that handles
/// both file transfers and pairing, plus mDNS advertise + discovery. Safe to
/// call repeatedly — the receiver and discovery threads only start once.
#[tauri::command]
fn start_sync(
    app: AppHandle,
    state: State<'_, SyncRuntime>,
    vault_path: String,
    port: Option<u16>,
) -> Result<SyncStartResult, String> {
    let vault = validate_vault(&vault_path)?;
    let device = read_device_identity(&vault)?;
    *state.active_vault.lock().unwrap() = Some(vault);
    let port = port.unwrap_or(DEFAULT_SYNC_PORT);

    if state
        .receiver_started
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        let listener = TcpListener::bind(("0.0.0.0", port)).map_err(|err| {
            state.receiver_started.store(false, Ordering::SeqCst);
            format!("failed to start LAN sync receiver on port {port}: {err}")
        })?;
        let app_for_thread = app.clone();
        thread::spawn(move || {
            for incoming in listener.incoming() {
                match incoming {
                    Ok(mut stream) => {
                        if let Err(err) = handle_sync_connection(&app_for_thread, &mut stream) {
                            let _ = app_for_thread.emit("sync://error", err);
                        }
                    }
                    Err(err) => eprintln!("LAN sync connection failed: {err}"),
                }
            }
        });
    }

    if state
        .discovery_started
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_ok()
    {
        if let Err(err) = start_discovery(app, device.clone(), port) {
            state.discovery_started.store(false, Ordering::SeqCst);
            return Err(err);
        }
    }

    Ok(SyncStartResult {
        port,
        device_id: device.device_id,
        device_name: device.device_name,
    })
}

#[tauri::command]
fn begin_pairing(state: State<'_, SyncRuntime>) -> String {
    let code = generate_pair_code();
    *state.pending_pair_code.lock().unwrap() = Some(code.clone());
    code
}

#[tauri::command]
fn cancel_pairing(state: State<'_, SyncRuntime>) {
    *state.pending_pair_code.lock().unwrap() = None;
}

#[tauri::command]
fn get_trusted_devices(app: AppHandle) -> Vec<TrustedDevice> {
    read_trusted_devices(&app)
}

/// Try the entered code against every discovered peer. The peer whose pending
/// code matches accepts, and both sides record each other as trusted.
#[tauri::command]
fn pair_with_code(
    app: AppHandle,
    state: State<'_, SyncRuntime>,
    code: String,
) -> Result<DeviceIdentity, String> {
    let vault = state
        .active_vault
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "start sync before pairing".to_string())?;
    let local_device = read_device_identity(&vault)?;
    let peers = state.peers.lock().unwrap().clone();
    if peers.is_empty() {
        return Err("No devices found yet — open CrossNotes on the other device first.".to_string());
    }

    let trimmed = code.trim().to_string();
    let mut last_err = "No device accepted that code.".to_string();
    for peer in peers {
        match request_pair_with_peer(&peer, &trimmed, &local_device) {
            Ok(remote) => {
                add_trusted_device(
                    &app,
                    &TrustedDevice {
                        device_id: remote.device_id.clone(),
                        device_name: remote.device_name.clone(),
                        paired_at: unix_timestamp().unwrap_or(0),
                    },
                )?;
                refresh_peer_pairing(&app);
                return Ok(remote);
            }
            Err(err) => last_err = err,
        }
    }
    Err(last_err)
}

/// Read the 5-byte magic and dispatch to the file-transfer or pairing handler.
fn handle_sync_connection(app: &AppHandle, stream: &mut TcpStream) -> Result<(), String> {
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| format!("failed to set LAN read timeout: {err}"))?;

    let mut magic = [0_u8; 5];
    stream
        .read_exact(&mut magic)
        .map_err(|err| format!("failed to read LAN header: {err}"))?;

    let vault = app
        .state::<SyncRuntime>()
        .active_vault
        .lock()
        .unwrap()
        .clone()
        .ok_or_else(|| "no active vault for incoming sync".to_string())?;

    match &magic {
        b"CNPK1" => {
            let result = receive_lan_sync_package(&vault, stream)?;
            app.emit("sync://received", &result)
                .map_err(|err| format!("failed to emit sync event: {err}"))?;
            Ok(())
        }
        b"CNPR1" => {
            let trusted = handle_pair_request(app, &vault, stream)?;
            app.emit("sync://paired", &trusted)
                .map_err(|err| format!("failed to emit pair event: {err}"))?;
            refresh_peer_pairing(app);
            Ok(())
        }
        _ => Err("incoming connection has an invalid header".to_string()),
    }
}

fn handle_pair_request(
    app: &AppHandle,
    vault: &Path,
    stream: &mut TcpStream,
) -> Result<TrustedDevice, String> {
    let mut len_bytes = [0_u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .map_err(|err| format!("failed to read pair request length: {err}"))?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len == 0 || len > 100_000 {
        return Err("pair request has an invalid size".to_string());
    }
    let mut buf = vec![0_u8; len];
    stream
        .read_exact(&mut buf)
        .map_err(|err| format!("failed to read pair request: {err}"))?;
    let request: PairRequest = serde_json::from_slice(&buf)
        .map_err(|err| format!("failed to parse pair request: {err}"))?;

    let runtime = app.state::<SyncRuntime>();
    let expected = runtime.pending_pair_code.lock().unwrap().clone();
    let accepted = matches!(expected, Some(code) if code.eq_ignore_ascii_case(request.code.trim()));

    let local_device = read_device_identity(vault)?;
    let identity_bytes = serde_json::to_vec(&local_device)
        .map_err(|err| format!("failed to encode identity: {err}"))?;
    stream
        .write_all(&[u8::from(accepted)])
        .and_then(|_| stream.write_all(&(identity_bytes.len() as u32).to_be_bytes()))
        .and_then(|_| stream.write_all(&identity_bytes))
        .map_err(|err| format!("failed to send pair reply: {err}"))?;

    if !accepted {
        return Err("a device tried to pair with the wrong code".to_string());
    }

    *runtime.pending_pair_code.lock().unwrap() = None;
    let trusted = TrustedDevice {
        device_id: request.device.device_id,
        device_name: request.device.device_name,
        paired_at: unix_timestamp().unwrap_or(0),
    };
    add_trusted_device(app, &trusted)?;
    Ok(trusted)
}

fn request_pair_with_peer(
    peer: &SyncPeer,
    code: &str,
    local: &DeviceIdentity,
) -> Result<DeviceIdentity, String> {
    let addr = format!("{}:{}", peer.host, peer.port);
    let mut stream = TcpStream::connect(&addr)
        .map_err(|err| format!("failed to reach {}: {err}", peer.device_name))?;
    stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(10))).ok();

    let request = PairRequest {
        code: code.to_string(),
        device: local.clone(),
    };
    let body = serde_json::to_vec(&request)
        .map_err(|err| format!("failed to encode pair request: {err}"))?;
    stream
        .write_all(b"CNPR1")
        .and_then(|_| stream.write_all(&(body.len() as u32).to_be_bytes()))
        .and_then(|_| stream.write_all(&body))
        .map_err(|err| format!("failed to send pair request: {err}"))?;

    let mut accept = [0_u8; 1];
    stream
        .read_exact(&mut accept)
        .map_err(|err| format!("no pair reply from {}: {err}", peer.device_name))?;
    let mut len_bytes = [0_u8; 4];
    stream
        .read_exact(&mut len_bytes)
        .map_err(|err| format!("failed to read pair reply length: {err}"))?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    if len == 0 || len > 100_000 {
        return Err("pair reply has an invalid size".to_string());
    }
    let mut buf = vec![0_u8; len];
    stream
        .read_exact(&mut buf)
        .map_err(|err| format!("failed to read pair reply: {err}"))?;
    let remote: DeviceIdentity = serde_json::from_slice(&buf)
        .map_err(|err| format!("failed to parse pair reply: {err}"))?;

    if accept[0] != 1 {
        return Err(format!("{} declined the code", peer.device_name));
    }
    Ok(remote)
}

fn start_discovery(app: AppHandle, device: DeviceIdentity, port: u16) -> Result<(), String> {
    let mdns = ServiceDaemon::new().map_err(|err| format!("failed to start mDNS: {err}"))?;

    let host_label = format!("{}.local.", sanitize_host_label(&device.device_id));
    let properties = [
        ("device_id", device.device_id.as_str()),
        ("device_name", device.device_name.as_str()),
    ];
    let service = ServiceInfo::new(
        SYNC_SERVICE_TYPE,
        &device.device_id,
        &host_label,
        "",
        port,
        &properties[..],
    )
    .map_err(|err| format!("failed to build mDNS service: {err}"))?
    .enable_addr_auto();

    mdns.register(service)
        .map_err(|err| format!("failed to register mDNS service: {err}"))?;
    let receiver = mdns
        .browse(SYNC_SERVICE_TYPE)
        .map_err(|err| format!("failed to browse mDNS: {err}"))?;

    let local_id = device.device_id;
    thread::spawn(move || {
        let _daemon = mdns; // keep the daemon alive for the thread's lifetime
        while let Ok(event) = receiver.recv() {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let device_id = match info.get_property_val_str("device_id") {
                        Some(id) if id != local_id => id.to_string(),
                        _ => continue,
                    };
                    let host = match info.get_addresses_v4().into_iter().next() {
                        Some(ip) => ip.to_string(),
                        None => continue,
                    };
                    let peer = SyncPeer {
                        device_id: device_id.clone(),
                        device_name: info
                            .get_property_val_str("device_name")
                            .unwrap_or("Unknown device")
                            .to_string(),
                        host,
                        port: info.get_port(),
                        paired: is_trusted(&app, &device_id),
                    };
                    upsert_peer(&app, peer);
                }
                ServiceEvent::ServiceRemoved(_ty, fullname) => {
                    let suffix = format!(".{SYNC_SERVICE_TYPE}");
                    let instance = fullname.strip_suffix(&suffix).unwrap_or(&fullname);
                    remove_peer(&app, instance);
                }
                _ => {}
            }
        }
    });

    Ok(())
}

fn upsert_peer(app: &AppHandle, peer: SyncPeer) {
    {
        let runtime = app.state::<SyncRuntime>();
        let mut peers = runtime.peers.lock().unwrap();
        if let Some(existing) = peers.iter_mut().find(|p| p.device_id == peer.device_id) {
            *existing = peer;
        } else {
            peers.push(peer);
        }
    }
    emit_peers(app);
}

fn remove_peer(app: &AppHandle, device_id: &str) {
    {
        let runtime = app.state::<SyncRuntime>();
        runtime
            .peers
            .lock()
            .unwrap()
            .retain(|p| p.device_id != device_id);
    }
    emit_peers(app);
}

fn refresh_peer_pairing(app: &AppHandle) {
    {
        let runtime = app.state::<SyncRuntime>();
        let mut peers = runtime.peers.lock().unwrap();
        for peer in peers.iter_mut() {
            peer.paired = is_trusted(app, &peer.device_id);
        }
    }
    emit_peers(app);
}

fn emit_peers(app: &AppHandle) {
    let peers = app.state::<SyncRuntime>().peers.lock().unwrap().clone();
    let _ = app.emit("sync://peers", peers);
}

fn sanitize_host_label(input: &str) -> String {
    let label: String = input
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() || ch == '-' { ch } else { '-' })
        .collect();
    if label.is_empty() {
        "crossnotes".to_string()
    } else {
        label
    }
}

fn generate_pair_code() -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    let mut x = unix_timestamp()
        .unwrap_or(1)
        .wrapping_mul(2_654_435_761)
        .wrapping_add(std::process::id() as u64)
        | 1;
    let mut code = String::with_capacity(5);
    for _ in 0..5 {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        code.push(ALPHABET[(x as usize) % ALPHABET.len()] as char);
    }
    code
}

fn trusted_devices_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|err| format!("failed to resolve app data directory: {err}"))?;
    fs::create_dir_all(&dir)
        .map_err(|err| format!("failed to create app data directory: {err}"))?;
    Ok(dir.join("trusted-devices.json"))
}

fn read_trusted_devices(app: &AppHandle) -> Vec<TrustedDevice> {
    let path = match trusted_devices_path(app) {
        Ok(path) => path,
        Err(_) => return Vec::new(),
    };
    if !path.exists() {
        return Vec::new();
    }
    fs::read_to_string(&path)
        .ok()
        .and_then(|contents| serde_json::from_str(&contents).ok())
        .unwrap_or_default()
}

fn add_trusted_device(app: &AppHandle, device: &TrustedDevice) -> Result<(), String> {
    let mut list = read_trusted_devices(app);
    if !list.iter().any(|d| d.device_id == device.device_id) {
        list.push(device.clone());
        write_json_file(&trusted_devices_path(app)?, &list)?;
    }
    Ok(())
}

fn is_trusted(app: &AppHandle, device_id: &str) -> bool {
    read_trusted_devices(app)
        .iter()
        .any(|d| d.device_id == device_id)
}

#[tauri::command]
fn send_lan_sync_package(
    peer_host: String,
    port: Option<u16>,
    package_path: String,
) -> Result<LanSendResult, String> {
    let package = PathBuf::from(&package_path);
    if !package.is_dir() {
        return Err(format!("sync package is not a folder: {package_path}"));
    }

    let manifest_path = package.join("crossnotes-sync-package.json");
    let package_manifest: SyncPackageManifest = read_json_file(&manifest_path)?;
    let peer_addr = format!("{}:{}", peer_host.trim(), port.unwrap_or(37642));
    let mut stream = TcpStream::connect(&peer_addr)
        .map_err(|err| format!("failed to connect to {peer_addr}: {err}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|err| format!("failed to set LAN sync timeout: {err}"))?;

    let manifest_bytes = serde_json::to_vec(&package_manifest)
        .map_err(|err| format!("failed to encode sync package manifest: {err}"))?;
    stream
        .write_all(b"CNPK1")
        .and_then(|_| stream.write_all(&(manifest_bytes.len() as u32).to_be_bytes()))
        .and_then(|_| stream.write_all(&manifest_bytes))
        .map_err(|err| format!("failed to send sync package header: {err}"))?;

    for file in &package_manifest.files {
        validate_relative_sync_path(&file.relative_path)?;
        let contents = fs::read(package.join(&file.relative_path)).map_err(|err| {
            format!(
                "failed to read sync package file {}: {err}",
                file.relative_path
            )
        })?;
        stream
            .write_all(&(contents.len() as u64).to_be_bytes())
            .and_then(|_| stream.write_all(&contents))
            .map_err(|err| {
                format!(
                    "failed to send sync package file {}: {err}",
                    file.relative_path
                )
            })?;
    }

    Ok(LanSendResult {
        sent_count: package_manifest.files.len(),
        peer_addr,
    })
}

/// Reads a CNPK1 package body from `stream`. The 5-byte magic must already be
/// consumed by the connection dispatcher.
fn receive_lan_sync_package(
    vault: &Path,
    stream: &mut TcpStream,
) -> Result<SyncImportResult, String> {
    let mut manifest_len_bytes = [0_u8; 4];
    stream
        .read_exact(&mut manifest_len_bytes)
        .map_err(|err| format!("failed to read LAN sync manifest length: {err}"))?;
    let manifest_len = u32::from_be_bytes(manifest_len_bytes) as usize;
    if manifest_len == 0 || manifest_len > 2_000_000 {
        return Err("incoming LAN sync manifest has an invalid size".to_string());
    }

    let mut manifest_bytes = vec![0_u8; manifest_len];
    stream
        .read_exact(&mut manifest_bytes)
        .map_err(|err| format!("failed to read LAN sync manifest: {err}"))?;
    let package_manifest: SyncPackageManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|err| format!("failed to parse LAN sync manifest: {err}"))?;

    let local_device = read_device_identity(vault)?;
    if package_manifest.source_device.device_id == local_device.device_id {
        return Err("incoming LAN sync package came from this vault/device".to_string());
    }

    let mut imported_count = 0;
    let mut skipped_count = 0;
    let mut conflicts = Vec::new();

    for file in &package_manifest.files {
        validate_relative_sync_path(&file.relative_path)?;
        let mut file_len_bytes = [0_u8; 8];
        stream
            .read_exact(&mut file_len_bytes)
            .map_err(|err| format!("failed to read LAN sync file length: {err}"))?;
        let file_len = u64::from_be_bytes(file_len_bytes);
        if file_len > 50_000_000 {
            return Err(format!(
                "incoming file is too large: {}",
                file.relative_path
            ));
        }

        let mut incoming = vec![0_u8; file_len as usize];
        stream
            .read_exact(&mut incoming)
            .map_err(|err| format!("failed to read LAN sync file {}: {err}", file.relative_path))?;

        let destination = vault.join(&file.relative_path);
        match apply_incoming_file(
            &destination,
            &incoming,
            &package_manifest.source_device.device_id,
            package_manifest.created_at,
        )? {
            ImportOutcome::Imported => imported_count += 1,
            ImportOutcome::Skipped => skipped_count += 1,
            ImportOutcome::Conflict(path) => conflicts.push(path),
        }
    }

    Ok(SyncImportResult {
        source_device_id: package_manifest.source_device.device_id,
        imported_count,
        skipped_count,
        conflict_count: conflicts.len(),
        conflicts,
    })
}

fn sanitize_file_name(file_name: &str) -> String {
    let sanitized: String = file_name
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            ch if ch.is_control() => '-',
            ch => ch,
        })
        .collect();

    let trimmed = sanitized.trim_matches([' ', '.']);
    if trimmed.is_empty() {
        "attachment".to_string()
    } else {
        trimmed.to_string()
    }
}

fn unique_file_name(directory: &Path, file_name: &str) -> String {
    let path = Path::new(file_name);
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("attachment");
    let extension = path.extension().and_then(|extension| extension.to_str());

    let mut candidate = file_name.to_string();
    let mut counter = 1;

    while directory.join(&candidate).exists() {
        candidate = match extension {
            Some(extension) => format!("{stem}-{counter}.{extension}"),
            None => format!("{stem}-{counter}"),
        };
        counter += 1;
    }

    candidate
}

fn is_image_file(file_name: &str) -> bool {
    let extension = Path::new(file_name)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase());

    matches!(
        extension.as_deref(),
        Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "svg" | "avif")
    )
}

fn validate_vault(vault_path: &str) -> Result<PathBuf, String> {
    let vault = PathBuf::from(vault_path);
    if !vault.is_dir() {
        return Err(format!("vault is not a directory: {vault_path}"));
    }
    Ok(vault)
}

fn ensure_crossnotes_dir(vault: &Path) -> Result<PathBuf, String> {
    let crossnotes_dir = vault.join(".crossnotes");
    fs::create_dir_all(&crossnotes_dir)
        .map_err(|err| format!("failed to create .crossnotes folder: {err}"))?;
    Ok(crossnotes_dir)
}

fn sync_manifest_path(vault: &Path) -> PathBuf {
    vault.join(".crossnotes").join("sync-manifest.json")
}

fn device_identity_path(vault: &Path) -> PathBuf {
    vault.join(".crossnotes").join("device.json")
}

fn read_sync_manifest(vault: &Path) -> Result<SyncManifest, String> {
    ensure_crossnotes_dir(vault)?;
    let manifest_path = sync_manifest_path(vault);
    if !manifest_path.exists() {
        let manifest = SyncManifest::default();
        write_sync_manifest(vault, &manifest)?;
        return Ok(manifest);
    }

    let contents = fs::read_to_string(&manifest_path)
        .map_err(|err| format!("failed to read sync manifest: {err}"))?;
    serde_json::from_str(&contents).map_err(|err| format!("failed to parse sync manifest: {err}"))
}

fn write_sync_manifest(vault: &Path, manifest: &SyncManifest) -> Result<(), String> {
    ensure_crossnotes_dir(vault)?;
    write_json_file(&sync_manifest_path(vault), manifest)
}

fn read_device_identity(vault: &Path) -> Result<DeviceIdentity, String> {
    ensure_crossnotes_dir(vault)?;
    let identity_path = device_identity_path(vault);
    if identity_path.exists() {
        return read_json_file(&identity_path);
    }

    let created_at = unix_timestamp()?;
    let identity = DeviceIdentity {
        device_id: format!("crossnotes-{created_at}-{}", std::process::id()),
        device_name: default_device_name(),
        created_at,
    };
    write_json_file(&identity_path, &identity)?;
    Ok(identity)
}

fn read_json_file<T>(path: &Path) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let contents = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&contents)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn write_json_file<T>(path: &Path, value: &T) -> Result<(), String>
where
    T: Serialize,
{
    let contents = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize {}: {err}", path.display()))?;
    fs::write(path, contents).map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn relative_note_path(vault: &Path, note_path: &Path) -> Result<String, String> {
    let relative = note_path
        .strip_prefix(vault)
        .map_err(|_| "note must be inside the active vault".to_string())?;

    let relative = relative
        .to_str()
        .ok_or_else(|| "note path contains invalid UTF-8".to_string())?
        .replace('\\', "/");

    if relative.starts_with(".crossnotes/") || relative == ".crossnotes" {
        return Err("internal CrossNotes files cannot be selected for sync".to_string());
    }

    if !relative.ends_with(".md") {
        return Err("only markdown notes can be selected for sync".to_string());
    }

    Ok(relative)
}

fn validate_relative_sync_path(relative_path: &str) -> Result<(), String> {
    let path = Path::new(relative_path);
    if relative_path.trim().is_empty() || path.is_absolute() {
        return Err("sync package contains an invalid note path".to_string());
    }

    let is_note = relative_path.ends_with(".md");
    let is_attachment = relative_path.starts_with("Attachments/");
    if !is_note && !is_attachment {
        return Err(format!(
            "sync package contains an unsupported file: {relative_path}"
        ));
    }

    if relative_path.starts_with(".crossnotes/") || relative_path == ".crossnotes" {
        return Err("sync package cannot include CrossNotes internal files".to_string());
    }

    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("sync package contains a path traversal entry".to_string());
    }

    Ok(())
}

/// Copy one vault-relative file into the staging package, recording its
/// metadata. Returns false if the file was already staged.
fn stage_sync_file(
    vault: &Path,
    sync_dir: &Path,
    relative_path: &str,
    package_files: &mut Vec<SyncPackageFile>,
    staged: &mut HashSet<String>,
) -> Result<bool, String> {
    if !staged.insert(relative_path.to_string()) {
        return Ok(false);
    }

    let source = vault.join(relative_path);
    let destination = sync_dir.join(relative_path);
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("failed to create sync package folder: {err}"))?;
    }
    fs::copy(&source, &destination)
        .map_err(|err| format!("failed to stage {} for sync: {err}", source.display()))?;

    let metadata = fs::metadata(&source)
        .map_err(|err| format!("failed to read metadata for {}: {err}", source.display()))?;
    package_files.push(SyncPackageFile {
        relative_path: relative_path.to_string(),
        size_bytes: metadata.len(),
        modified_at: metadata.modified().ok().and_then(system_time_to_unix),
    });
    Ok(true)
}

/// Scan markdown content for `Attachments/…` references (with or without a
/// leading `./`) and return their vault-relative paths.
fn collect_attachment_paths(content: &str) -> Vec<String> {
    const NEEDLE: &str = "Attachments/";
    let stop = |ch: char| {
        matches!(
            ch,
            ')' | ']' | '(' | '"' | '\'' | '`' | '<' | '>' | '|' | ' ' | '\t' | '\n' | '\r'
        )
    };

    let mut out = Vec::new();
    let mut search_from = 0;
    while let Some(found) = content[search_from..].find(NEEDLE) {
        let start = search_from + found;
        let rest = &content[start..];
        let end = rest.find(stop).unwrap_or(rest.len());
        let raw = &rest[..end];
        search_from = start + NEEDLE.len();

        let decoded = percent_decode(raw);
        if !decoded.contains("..") && decoded.len() > NEEDLE.len() {
            out.push(decoded);
        }
    }

    out.sort();
    out.dedup();
    out
}

fn percent_decode(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(byte) = u8::from_str_radix(&input[i + 1..i + 3], 16) {
                out.push(byte);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Write `contents` to `path` by writing a sibling temp file and renaming it
/// into place, so a crash mid-write can never leave a truncated note.
fn write_atomic(path: &Path, contents: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("invalid destination path: {}", path.display()))?;
    fs::create_dir_all(parent)
        .map_err(|err| format!("failed to create folder {}: {err}", parent.display()))?;

    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("note");
    let tmp = parent.join(format!(
        ".{file_name}.tmp-{}-{}",
        std::process::id(),
        unix_timestamp().unwrap_or(0)
    ));

    fs::write(&tmp, contents)
        .map_err(|err| format!("failed to write {}: {err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        let _ = fs::remove_file(&tmp);
        format!("failed to finalize {}: {err}", path.display())
    })
}

fn conflict_file_path(destination: &Path, source_device_id: &str, created_at: u64) -> PathBuf {
    let parent = destination.parent().unwrap_or_else(|| Path::new(""));
    let stem = destination
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("note");
    let extension = destination
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or("md");
    let safe_device_id = sanitize_file_name(source_device_id);

    parent.join(format!(
        "{stem}.conflict-{safe_device_id}-{created_at}.{extension}"
    ))
}

fn default_device_name() -> String {
    std::env::var("CROSSNOTES_DEVICE_NAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "CrossNotes Device".to_string())
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn system_time_to_unix(time: SystemTime) -> Option<u64> {
    time.duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_secs())
}

fn unix_timestamp() -> Result<u64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .map_err(|err| format!("system clock is before UNIX epoch: {err}"))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .manage(SyncRuntime::default())
        .invoke_handler(tauri::generate_handler![
            greet,
            open_in_file_manager,
            get_default_vault,
            attach_file_to_vault,
            create_vault,
            get_sync_manifest,
            get_device_identity,
            set_note_sync_enabled,
            trigger_sync,
            import_sync_package,
            set_active_vault,
            start_sync,
            begin_pairing,
            cancel_pairing,
            pair_with_code,
            get_trusted_devices,
            send_lan_sync_package
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
