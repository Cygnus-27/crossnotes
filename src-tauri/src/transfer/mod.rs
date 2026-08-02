//! Transfer state, progress reporting, and the approval gates.

pub mod gate;
pub mod recv;
pub mod send;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, oneshot};

use crate::error::{Error, Result};

/// Progress updates are coalesced to this interval. At 100 MB/s a naive
/// per-chunk emit would fire ~200 times a second and swamp the webview for no
/// visible benefit.
pub const PROGRESS_INTERVAL_MS: u128 = 100;

/// How long a prompt may sit unanswered before the transfer gives up.
///
/// A prompt holds an open connection and a live task while it waits, so
/// without a bound anyone on the network could pin resources open simply by
/// connecting and never being answered. Long enough that a user who wandered
/// off mid-transfer can still come back to it.
pub const APPROVAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(180);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Direction {
    Send,
    Receive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Status {
    /// Connecting and handshaking.
    Connecting,
    /// Blocked on a human: either a pairing confirmation or an accept prompt.
    AwaitingApproval,
    Active,
    Completed,
    Declined,
    Cancelled,
    Failed,
}

impl Status {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Status::Completed | Status::Declined | Status::Cancelled | Status::Failed
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferProgress {
    pub transfer_id: String,
    pub direction: Direction,
    pub peer_device_id: String,
    pub peer_name: String,
    pub status: Status,
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub file_count: usize,
    pub files_completed: usize,
    pub current_file: Option<String>,
    pub bytes_per_second: u64,
    pub error: Option<String>,
}

/// An offer waiting on the user's decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IncomingOffer {
    pub transfer_id: String,
    pub peer_device_id: String,
    pub peer_name: String,
    pub peer_fingerprint: String,
    pub file_count: usize,
    pub total_bytes: u64,
    /// First few names, for the prompt. Not the whole list — an offer can
    /// contain tens of thousands of files.
    pub preview: Vec<String>,
}

/// A first-contact pairing waiting on the user comparing codes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PairingRequest {
    pub request_id: String,
    pub peer_device_id: String,
    pub peer_name: String,
    pub peer_fingerprint: String,
    /// The six digits that must match on both screens.
    pub pairing_code: String,
    pub direction: Direction,
}

/// Raised when a known device presents an unexpected key. Surfaced loudly and
/// separately from ordinary failures — this is the one error that may mean
/// someone is actively interfering.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityWarning {
    pub peer_device_id: String,
    pub peer_name: String,
    pub expected_fingerprint: String,
    pub presented_fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum TransferEvent {
    Progress(TransferProgress),
    Offer(IncomingOffer),
    Pairing(PairingRequest),
    Identity(IdentityWarning),
}

impl TransferEvent {
    /// Tauri event name this maps to on the frontend.
    pub fn channel(&self) -> &'static str {
        match self {
            TransferEvent::Progress(_) => "transfer://progress",
            TransferEvent::Offer(_) => "transfer://offer",
            TransferEvent::Pairing(_) => "transfer://pairing",
            TransferEvent::Identity(_) => "security://identity-mismatch",
        }
    }
}

/// One file inside a transfer.
#[derive(Debug, Clone)]
pub struct FileEntry {
    pub index: u32,
    /// Absolute path on the sender's disk. Empty on the receiving side.
    pub source: PathBuf,
    /// Path relative to the transfer root, as it goes on the wire.
    pub relative: String,
    pub size: u64,
}

struct TransferState {
    progress: TransferProgress,
    cancel: Arc<AtomicBool>,
    started: Instant,
    last_emit: Instant,
}

/// Owns every in-flight transfer plus the channels the UI answers prompts on.
pub struct TransferManager {
    transfers: Mutex<HashMap<String, TransferState>>,
    /// Offers blocked on an accept/decline decision.
    pending_offers: Mutex<HashMap<String, oneshot::Sender<OfferDecision>>>,
    /// Pairings blocked on the user confirming the code.
    pending_pairings: Mutex<HashMap<String, oneshot::Sender<bool>>>,
    events: broadcast::Sender<TransferEvent>,
}

#[derive(Debug, Clone)]
pub enum OfferDecision {
    Accept,
    Decline(Option<String>),
}

impl Default for TransferManager {
    fn default() -> Self {
        Self::new()
    }
}

impl TransferManager {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(256);
        TransferManager {
            transfers: Mutex::new(HashMap::new()),
            pending_offers: Mutex::new(HashMap::new()),
            pending_pairings: Mutex::new(HashMap::new()),
            events,
        }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TransferEvent> {
        self.events.subscribe()
    }

    fn emit(&self, event: TransferEvent) {
        // No subscribers is normal at startup and during shutdown.
        let _ = self.events.send(event);
    }

    pub fn register(
        &self,
        transfer_id: &str,
        direction: Direction,
        peer_device_id: &str,
        peer_name: &str,
        file_count: usize,
        total_bytes: u64,
    ) -> Arc<AtomicBool> {
        let cancel = Arc::new(AtomicBool::new(false));
        let progress = TransferProgress {
            transfer_id: transfer_id.to_string(),
            direction,
            peer_device_id: peer_device_id.to_string(),
            peer_name: peer_name.to_string(),
            status: Status::Connecting,
            total_bytes,
            transferred_bytes: 0,
            file_count,
            files_completed: 0,
            current_file: None,
            bytes_per_second: 0,
            error: None,
        };

        let now = Instant::now();
        self.transfers.lock().unwrap().insert(
            transfer_id.to_string(),
            TransferState {
                progress: progress.clone(),
                cancel: cancel.clone(),
                started: now,
                last_emit: now,
            },
        );

        self.emit(TransferEvent::Progress(progress));
        cancel
    }

    pub fn set_status(&self, transfer_id: &str, status: Status) {
        let snapshot = {
            let mut transfers = self.transfers.lock().unwrap();
            let Some(state) = transfers.get_mut(transfer_id) else {
                return;
            };
            state.progress.status = status;
            state.progress.clone()
        };
        // Status changes are rare and meaningful, so they bypass throttling.
        self.emit(TransferEvent::Progress(snapshot));
    }

    pub fn fail(&self, transfer_id: &str, error: &str) {
        let snapshot = {
            let mut transfers = self.transfers.lock().unwrap();
            let Some(state) = transfers.get_mut(transfer_id) else {
                return;
            };
            state.progress.status = Status::Failed;
            state.progress.error = Some(error.to_string());
            state.progress.current_file = None;
            state.progress.clone()
        };
        self.emit(TransferEvent::Progress(snapshot));
    }

    pub fn begin_file(&self, transfer_id: &str, relative: &str) {
        let snapshot = {
            let mut transfers = self.transfers.lock().unwrap();
            let Some(state) = transfers.get_mut(transfer_id) else {
                return;
            };
            state.progress.status = Status::Active;
            state.progress.current_file = Some(relative.to_string());
            state.progress.clone()
        };
        self.emit(TransferEvent::Progress(snapshot));
    }

    pub fn finish_file(&self, transfer_id: &str) {
        let snapshot = {
            let mut transfers = self.transfers.lock().unwrap();
            let Some(state) = transfers.get_mut(transfer_id) else {
                return;
            };
            state.progress.files_completed += 1;
            state.progress.clone()
        };
        self.emit(TransferEvent::Progress(snapshot));
    }

    /// Add to the byte counter, emitting at most once per
    /// [`PROGRESS_INTERVAL_MS`].
    pub fn advance(&self, transfer_id: &str, bytes: u64) {
        let snapshot = {
            let mut transfers = self.transfers.lock().unwrap();
            let Some(state) = transfers.get_mut(transfer_id) else {
                return;
            };

            state.progress.transferred_bytes += bytes;

            let elapsed = state.started.elapsed().as_secs_f64();
            state.progress.bytes_per_second = if elapsed > 0.05 {
                (state.progress.transferred_bytes as f64 / elapsed) as u64
            } else {
                0
            };

            if state.last_emit.elapsed().as_millis() < PROGRESS_INTERVAL_MS {
                return;
            }
            state.last_emit = Instant::now();
            state.progress.clone()
        };
        self.emit(TransferEvent::Progress(snapshot));
    }

    pub fn complete(&self, transfer_id: &str) {
        let snapshot = {
            let mut transfers = self.transfers.lock().unwrap();
            let Some(state) = transfers.get_mut(transfer_id) else {
                return;
            };
            state.progress.status = Status::Completed;
            state.progress.current_file = None;
            // Snap to 100%: rounding across chunks can leave the counter a few
            // bytes short of the total, which looks like a stalled transfer.
            state.progress.transferred_bytes = state.progress.total_bytes;
            state.progress.clone()
        };
        self.emit(TransferEvent::Progress(snapshot));
    }

    pub fn cancel(&self, transfer_id: &str) -> Result<()> {
        let cancel = {
            let transfers = self.transfers.lock().unwrap();
            transfers
                .get(transfer_id)
                .map(|state| state.cancel.clone())
                .ok_or_else(|| Error::UnknownTransfer(transfer_id.to_string()))?
        };

        cancel.store(true, Ordering::SeqCst);

        // A transfer still waiting on a prompt has no loop to notice the flag,
        // so resolve the prompt too.
        if let Some(sender) = self.pending_offers.lock().unwrap().remove(transfer_id) {
            let _ = sender.send(OfferDecision::Decline(Some("cancelled".into())));
        }

        self.set_status(transfer_id, Status::Cancelled);
        Ok(())
    }

    pub fn is_cancelled(&self, transfer_id: &str) -> bool {
        self.transfers
            .lock()
            .unwrap()
            .get(transfer_id)
            .map(|state| state.cancel.load(Ordering::SeqCst))
            .unwrap_or(false)
    }

    pub fn list(&self) -> Vec<TransferProgress> {
        self.transfers
            .lock()
            .unwrap()
            .values()
            .map(|state| state.progress.clone())
            .collect()
    }

    /// Drop finished transfers from the list.
    pub fn clear_finished(&self) {
        self.transfers
            .lock()
            .unwrap()
            .retain(|_, state| !state.progress.status.is_terminal());
    }

    // ---- Approval gates -------------------------------------------------

    /// Publish an offer and block until the user answers.
    ///
    /// The receive path holds an open connection while this waits, which is
    /// intentional: the sender should see the connection stay up while someone
    /// decides, rather than being told the transfer failed.
    pub async fn await_offer_decision(&self, offer: IncomingOffer) -> OfferDecision {
        let (tx, rx) = oneshot::channel();
        self.pending_offers
            .lock()
            .unwrap()
            .insert(offer.transfer_id.clone(), tx);

        let transfer_id = offer.transfer_id.clone();
        self.emit(TransferEvent::Offer(offer));

        // Declining is the default on every abnormal outcome: a dropped sender
        // (the app is shutting down) and a timeout both end up here, and
        // neither is a reason to start writing someone's files to disk.
        match tokio::time::timeout(APPROVAL_TIMEOUT, rx).await {
            Ok(Ok(decision)) => decision,
            Ok(Err(_)) => OfferDecision::Decline(Some("no response".into())),
            Err(_) => {
                self.pending_offers.lock().unwrap().remove(&transfer_id);
                OfferDecision::Decline(Some("timed out waiting for a response".into()))
            }
        }
    }

    pub fn resolve_offer(&self, transfer_id: &str, decision: OfferDecision) -> Result<()> {
        let sender = self
            .pending_offers
            .lock()
            .unwrap()
            .remove(transfer_id)
            .ok_or_else(|| Error::UnknownTransfer(transfer_id.to_string()))?;

        sender
            .send(decision)
            .map_err(|_| Error::Other("the transfer is no longer waiting".into()))
    }

    /// Publish a pairing request and block until the user confirms the code.
    ///
    /// Defaults to *not* paired on every abnormal outcome. A pairing that
    /// succeeds by accident is a pinned key for a device nobody vouched for.
    pub async fn await_pairing_decision(&self, request: PairingRequest) -> bool {
        let (tx, rx) = oneshot::channel();
        self.pending_pairings
            .lock()
            .unwrap()
            .insert(request.request_id.clone(), tx);

        let request_id = request.request_id.clone();
        self.emit(TransferEvent::Pairing(request));

        match tokio::time::timeout(APPROVAL_TIMEOUT, rx).await {
            Ok(Ok(confirmed)) => confirmed,
            Ok(Err(_)) => false,
            Err(_) => {
                self.pending_pairings.lock().unwrap().remove(&request_id);
                false
            }
        }
    }

    pub fn resolve_pairing(&self, request_id: &str, confirmed: bool) -> Result<()> {
        let sender = self
            .pending_pairings
            .lock()
            .unwrap()
            .remove(request_id)
            .ok_or_else(|| Error::Other(format!("no pairing in progress for {request_id}")))?;

        sender
            .send(confirmed)
            .map_err(|_| Error::Other("the pairing is no longer waiting".into()))
    }

    pub fn warn_identity_mismatch(&self, warning: IdentityWarning) {
        self.emit(TransferEvent::Identity(warning));
    }
}

/// Expand the user's selection into a flat file list with wire-relative paths.
///
/// Folders are walked; a selected folder keeps its own name as the top-level
/// prefix so the structure is reproduced on the other side.
pub fn collect_files(selection: &[PathBuf]) -> Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    let mut index = 0u32;

    for root in selection {
        let metadata = std::fs::symlink_metadata(root)?;

        if metadata.is_file() {
            let name = root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "file".to_string());
            entries.push(FileEntry {
                index,
                source: root.clone(),
                relative: name,
                size: metadata.len(),
            });
            index += 1;
        } else if metadata.is_dir() {
            let prefix = root
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "folder".to_string());
            walk_dir(root, &prefix, &mut entries, &mut index, 0)?;
        }
        // Symlinks at the top level are skipped rather than followed: the user
        // picked a link, and resolving it could pull in a target they did not
        // intend to share.
    }

    Ok(entries)
}

/// Depth limit, guarding against pathological trees and any symlink loop that
/// slips past the per-entry check.
const MAX_WALK_DEPTH: usize = 64;

fn walk_dir(
    dir: &Path,
    prefix: &str,
    entries: &mut Vec<FileEntry>,
    index: &mut u32,
    depth: usize,
) -> Result<()> {
    if depth >= MAX_WALK_DEPTH {
        return Ok(());
    }

    let mut children: Vec<_> = std::fs::read_dir(dir)?.filter_map(|e| e.ok()).collect();
    // Stable ordering makes transfers reproducible and progress monotonic.
    children.sort_by_key(|entry| entry.file_name());

    for child in children {
        let path = child.path();
        // symlink_metadata, not metadata: this must not follow links.
        let Ok(metadata) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if metadata.file_type().is_symlink() {
            continue;
        }

        let name = child.file_name().to_string_lossy().to_string();
        let relative = format!("{prefix}/{name}");

        if metadata.is_dir() {
            walk_dir(&path, &relative, entries, index, depth + 1)?;
        } else if metadata.is_file() {
            entries.push(FileEntry {
                index: *index,
                source: path,
                relative,
                size: metadata.len(),
            });
            *index += 1;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fluqsr-xfer-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn registering_a_transfer_makes_it_listable() {
        let manager = TransferManager::new();
        manager.register("t1", Direction::Send, "peer", "Laptop", 2, 100);

        let list = manager.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].status, Status::Connecting);
        assert_eq!(list[0].total_bytes, 100);
    }

    #[test]
    fn progress_accumulates() {
        let manager = TransferManager::new();
        manager.register("t1", Direction::Send, "peer", "Laptop", 1, 100);

        manager.advance("t1", 40);
        manager.advance("t1", 35);

        assert_eq!(manager.list()[0].transferred_bytes, 75);
    }

    #[test]
    fn completion_snaps_the_counter_to_the_total() {
        let manager = TransferManager::new();
        manager.register("t1", Direction::Send, "peer", "Laptop", 1, 100);
        manager.advance("t1", 99);
        manager.complete("t1");

        let progress = &manager.list()[0];
        assert_eq!(progress.status, Status::Completed);
        assert_eq!(
            progress.transferred_bytes, progress.total_bytes,
            "a finished transfer must not display as stuck at 99%"
        );
    }

    #[test]
    fn cancelling_sets_the_flag_and_the_status() {
        let manager = TransferManager::new();
        manager.register("t1", Direction::Send, "peer", "Laptop", 1, 100);

        manager.cancel("t1").unwrap();

        assert!(manager.is_cancelled("t1"));
        assert_eq!(manager.list()[0].status, Status::Cancelled);
    }

    #[test]
    fn cancelling_an_unknown_transfer_errors() {
        assert!(TransferManager::new().cancel("nope").is_err());
    }

    #[test]
    fn failures_carry_their_reason() {
        let manager = TransferManager::new();
        manager.register("t1", Direction::Receive, "peer", "Laptop", 1, 100);
        manager.fail("t1", "connection reset");

        let progress = &manager.list()[0];
        assert_eq!(progress.status, Status::Failed);
        assert_eq!(progress.error.as_deref(), Some("connection reset"));
    }

    #[test]
    fn clearing_keeps_only_live_transfers() {
        let manager = TransferManager::new();
        manager.register("done", Direction::Send, "p", "L", 1, 10);
        manager.register("live", Direction::Send, "p", "L", 1, 10);
        manager.complete("done");

        manager.clear_finished();

        let list = manager.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].transfer_id, "live");
    }

    #[tokio::test]
    async fn an_offer_resolves_with_the_users_answer() {
        let manager = Arc::new(TransferManager::new());
        let offer = IncomingOffer {
            transfer_id: "t1".into(),
            peer_device_id: "peer".into(),
            peer_name: "Laptop".into(),
            peer_fingerprint: "ab".repeat(32),
            file_count: 1,
            total_bytes: 10,
            preview: vec!["a.txt".into()],
        };

        let waiter = {
            let manager = manager.clone();
            tokio::spawn(async move { manager.await_offer_decision(offer).await })
        };

        // Let the waiter register before answering.
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        manager.resolve_offer("t1", OfferDecision::Accept).unwrap();

        assert!(matches!(waiter.await.unwrap(), OfferDecision::Accept));
    }

    #[tokio::test]
    async fn cancelling_while_awaiting_approval_declines_the_offer() {
        let manager = Arc::new(TransferManager::new());
        manager.register("t1", Direction::Receive, "peer", "Laptop", 1, 10);

        let offer = IncomingOffer {
            transfer_id: "t1".into(),
            peer_device_id: "peer".into(),
            peer_name: "Laptop".into(),
            peer_fingerprint: "ab".repeat(32),
            file_count: 1,
            total_bytes: 10,
            preview: vec![],
        };

        let waiter = {
            let manager = manager.clone();
            tokio::spawn(async move { manager.await_offer_decision(offer).await })
        };

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        manager.cancel("t1").unwrap();

        assert!(
            matches!(waiter.await.unwrap(), OfferDecision::Decline(_)),
            "a cancel must unblock the prompt rather than leave it hanging"
        );
    }

    #[tokio::test]
    async fn pairing_resolves_with_the_users_answer() {
        let manager = Arc::new(TransferManager::new());
        let request = PairingRequest {
            request_id: "p1".into(),
            peer_device_id: "peer".into(),
            peer_name: "Laptop".into(),
            peer_fingerprint: "ab".repeat(32),
            pairing_code: "123456".into(),
            direction: Direction::Receive,
        };

        let waiter = {
            let manager = manager.clone();
            tokio::spawn(async move { manager.await_pairing_decision(request).await })
        };

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        manager.resolve_pairing("p1", true).unwrap();

        assert!(waiter.await.unwrap());
    }

    #[test]
    fn collects_a_single_file() {
        let dir = temp_dir("collect-file");
        let file = dir.join("notes.txt");
        std::fs::write(&file, b"hello").unwrap();

        let entries = collect_files(&[file]).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].relative, "notes.txt");
        assert_eq!(entries[0].size, 5);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn collects_a_folder_and_keeps_its_structure() {
        let dir = temp_dir("collect-folder");
        let project = dir.join("project");
        std::fs::create_dir_all(project.join("src")).unwrap();
        std::fs::write(project.join("README.md"), b"readme").unwrap();
        std::fs::write(project.join("src").join("main.rs"), b"fn main() {}").unwrap();

        let entries = collect_files(&[project]).unwrap();
        let mut paths: Vec<_> = entries.iter().map(|e| e.relative.clone()).collect();
        paths.sort();

        assert_eq!(paths, vec!["project/README.md", "project/src/main.rs"]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_indices_are_unique_and_sequential() {
        let dir = temp_dir("collect-indices");
        std::fs::create_dir_all(dir.join("many")).unwrap();
        for n in 0..5 {
            std::fs::write(dir.join("many").join(format!("{n}.txt")), b"x").unwrap();
        }

        let entries = collect_files(&[dir.join("many")]).unwrap();
        let indices: Vec<_> = entries.iter().map(|e| e.index).collect();
        assert_eq!(indices, vec![0, 1, 2, 3, 4]);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn empty_files_are_included() {
        let dir = temp_dir("collect-empty");
        let file = dir.join("empty.bin");
        std::fs::write(&file, b"").unwrap();

        let entries = collect_files(&[file]).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].size, 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[cfg(unix)]
    #[test]
    fn symlinks_are_not_followed() {
        // Following a link inside a shared folder could pull in a file well
        // outside what the user chose to send.
        let dir = temp_dir("collect-symlink");
        let secret = dir.join("secret.txt");
        std::fs::write(&secret, b"private").unwrap();

        let shared = dir.join("shared");
        std::fs::create_dir_all(&shared).unwrap();
        std::fs::write(shared.join("ok.txt"), b"fine").unwrap();
        std::os::unix::fs::symlink(&secret, shared.join("link.txt")).unwrap();

        let entries = collect_files(&[shared]).unwrap();
        let paths: Vec<_> = entries.iter().map(|e| e.relative.clone()).collect();

        assert_eq!(paths, vec!["shared/ok.txt"]);

        std::fs::remove_dir_all(&dir).ok();
    }
}
