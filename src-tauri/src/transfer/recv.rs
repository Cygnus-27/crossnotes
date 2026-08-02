//! The receiving half of a transfer.
//!
//! This side handles data chosen entirely by someone else, so the ordering
//! here is deliberate: authenticate, then get consent, then validate every
//! path, and only then touch the disk.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;

use crate::error::{Error, Result};
use crate::paths;
use crate::protocol::{
    self, Accept, AcceptedFile, Control, Hello, Offer, Reject, PROTOCOL_VERSION,
};
use crate::settings::SettingsStore;
use crate::tls;

use super::gate::{self, Node};
use super::{Direction, IncomingOffer, OfferDecision, Status};

/// How many filenames the accept prompt shows before summarising.
const PREVIEW_LIMIT: usize = 8;

/// Accept connections until the process exits.
pub async fn run_listener(
    node: Node,
    settings: Arc<SettingsStore>,
    port: u16,
) -> Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", port)).await.map_err(|err| {
        Error::Other(format!(
            "could not listen on port {port}: {err}. \
             Another copy of fluqsr may be running, or a firewall may be blocking it."
        ))
    })?;

    let acceptor = TlsAcceptor::from(tls::server_config(&node.identity)?);

    loop {
        let Ok((tcp, remote)) = listener.accept().await else {
            continue;
        };

        let node = node.clone();
        let settings = settings.clone();
        let acceptor = acceptor.clone();

        // One task per connection: a peer sitting on an unanswered prompt must
        // not stop anyone else from connecting.
        tokio::spawn(async move {
            if let Err(err) = handle_connection(node, settings, acceptor, tcp, remote).await {
                // Nothing to do beyond reporting — the connection is already
                // gone, and the manager has recorded the failure if a transfer
                // had been registered.
                eprintln!("fluqsr: connection from {remote} ended: {err}");
            }
        });
    }
}

async fn handle_connection(
    node: Node,
    settings: Arc<SettingsStore>,
    acceptor: TlsAcceptor,
    tcp: TcpStream,
    remote: SocketAddr,
) -> Result<()> {
    tcp.set_nodelay(true)?;
    let mut stream = acceptor.accept(tcp).await?;

    let presented = {
        let (_, connection) = stream.get_ref();
        tls::PeerIdentity::from_certs(connection.peer_certificates())?.fingerprint
    };

    let ours = Hello {
        protocol_version: PROTOCOL_VERSION,
        device_id: node.identity.device_id.clone(),
        device_name: node.identity.device_name.clone(),
        platform: crate::device::current_platform().to_string(),
    };
    let theirs = protocol::handshake(&mut stream, &ours).await?;

    // Gate one: is this a device we trust? Refusing here means we never even
    // learn what they wanted to send.
    let peer = gate::authenticate(&node, presented, &theirs, Direction::Receive).await?;

    let offer = match protocol::read_control(&mut stream).await? {
        Control::Offer(offer) => offer,
        other => {
            return Err(Error::Protocol(format!(
                "expected an Offer from {remote}, got {}",
                protocol::control_name(&other)
            )))
        }
    };

    let file_count = offer.files.len();
    node.manager.register(
        &offer.transfer_id,
        Direction::Receive,
        &theirs.device_id,
        &peer.device_name,
        file_count,
        offer.total_bytes,
    );

    // Gate two: does the user want these files? Skipped only for a peer that
    // is both pinned and explicitly marked auto-accept.
    let auto = node.trust.auto_accepts(&theirs.device_id, &presented);
    if !auto {
        node.manager
            .set_status(&offer.transfer_id, Status::AwaitingApproval);

        let decision = node
            .manager
            .await_offer_decision(IncomingOffer {
                transfer_id: offer.transfer_id.clone(),
                peer_device_id: theirs.device_id.clone(),
                peer_name: peer.device_name.clone(),
                peer_fingerprint: presented.to_hex(),
                file_count,
                total_bytes: offer.total_bytes,
                preview: offer
                    .files
                    .iter()
                    .take(PREVIEW_LIMIT)
                    .map(|file| file.path.clone())
                    .collect(),
            })
            .await;

        if let OfferDecision::Decline(reason) = decision {
            protocol::write_control(
                &mut stream,
                &Control::Reject(Reject {
                    transfer_id: offer.transfer_id.clone(),
                    reason: reason.clone(),
                }),
            )
            .await?;
            stream.flush().await?;
            node.manager
                .set_status(&offer.transfer_id, Status::Declined);
            return Ok(());
        }
    }

    // Gate three: every path is validated before a single byte is written.
    // A path that fails here fails the whole transfer rather than being
    // quietly skipped — a sender trying to write outside the receive folder
    // is not having an accident.
    let receive_dir = settings.receive_dir();
    tokio::fs::create_dir_all(&receive_dir).await?;

    let plan = build_plan(&offer, &receive_dir)?;

    let accept = Accept {
        transfer_id: offer.transfer_id.clone(),
        files: plan
            .values()
            .map(|file| AcceptedFile {
                index: file.index,
                resume_offset: file.resume_offset,
            })
            .collect(),
    };
    protocol::write_control(&mut stream, &Control::Accept(accept)).await?;
    stream.flush().await?;

    match receive_files(&node, &offer, plan, &mut stream).await {
        Ok(()) => {
            // Confirm back to the sender only now, with every file hashed,
            // verified, and renamed into place. This is what lets the sending
            // side report a transfer as genuinely done.
            protocol::write_control(
                &mut stream,
                &Control::TransferComplete(protocol::TransferComplete {
                    transfer_id: offer.transfer_id.clone(),
                }),
            )
            .await?;
            stream.flush().await?;

            node.manager.complete(&offer.transfer_id);
            Ok(())
        }
        Err(Error::Cancelled) => {
            node.manager
                .set_status(&offer.transfer_id, Status::Cancelled);
            Err(Error::Cancelled)
        }
        Err(err) => {
            node.manager.fail(&offer.transfer_id, &err.to_string());
            Err(err)
        }
    }
}

/// A validated destination for one incoming file.
struct PlannedFile {
    index: u32,
    /// Where the bytes accumulate while in flight.
    partial: PathBuf,
    /// Where the file lands once its hash checks out.
    final_path: PathBuf,
    resume_offset: u64,
    declared_size: u64,
}

/// Turn an offer into validated destinations, before anything is written.
fn build_plan(offer: &Offer, receive_dir: &std::path::Path) -> Result<HashMap<u32, PlannedFile>> {
    let mut plan = HashMap::new();

    for file in &offer.files {
        // Two independent checks: the string is structurally safe, and the
        // joined result really is inside the receive folder.
        let relative = paths::safe_relative_path(&file.path)?;
        let candidate = paths::resolve_within(receive_dir, &relative)?;

        let partial = paths::partial_path(&candidate);

        // Resume only from a partial that is shorter than what is being
        // offered. A longer one belongs to something else, and its bytes would
        // corrupt this file.
        let resume_offset = std::fs::metadata(&partial)
            .map(|meta| meta.len())
            .unwrap_or(0)
            .min(file.size);
        let resume_offset = if resume_offset >= file.size {
            0
        } else {
            resume_offset
        };

        if plan
            .insert(
                file.index,
                PlannedFile {
                    index: file.index,
                    partial,
                    final_path: candidate,
                    resume_offset,
                    declared_size: file.size,
                },
            )
            .is_some()
        {
            return Err(Error::Protocol(format!(
                "offer reuses file index {}",
                file.index
            )));
        }
    }

    Ok(plan)
}

async fn receive_files<S>(
    node: &Node,
    offer: &Offer,
    plan: HashMap<u32, PlannedFile>,
    stream: &mut S,
) -> Result<()>
where
    S: tokio::io::AsyncRead + Unpin,
{
    let mut open: HashMap<u32, OpenFile> = HashMap::new();

    loop {
        if node.manager.is_cancelled(&offer.transfer_id) {
            return Err(Error::Cancelled);
        }

        let Some(frame) = protocol::read_frame(stream).await? else {
            return Err(Error::Protocol(
                "the sender disconnected before the transfer finished".into(),
            ));
        };

        match frame {
            protocol::Frame::Data { index, bytes } => {
                let planned = plan.get(&index).ok_or_else(|| {
                    Error::Protocol(format!("received data for unoffered file index {index}"))
                })?;

                // Opened lazily so an offer of 50,000 files does not exhaust
                // the file-descriptor limit up front.
                let file = match open.get_mut(&index) {
                    Some(file) => file,
                    None => {
                        node.manager.begin_file(
                            &offer.transfer_id,
                            &planned.final_path.to_string_lossy(),
                        );
                        let opened = OpenFile::create(planned).await?;
                        if planned.resume_offset > 0 {
                            node.manager
                                .advance(&offer.transfer_id, planned.resume_offset);
                        }
                        open.entry(index).or_insert(opened)
                    }
                };

                file.write(&bytes).await?;
                node.manager.advance(&offer.transfer_id, bytes.len() as u64);
            }

            protocol::Frame::Control(Control::FileComplete(complete)) => {
                let planned = plan.get(&complete.index).ok_or_else(|| {
                    Error::Protocol(format!(
                        "completion for unoffered file index {}",
                        complete.index
                    ))
                })?;

                // A zero-byte file produces no data frames, so it may not be
                // open yet.
                let file = match open.remove(&complete.index) {
                    Some(file) => file,
                    None => OpenFile::create(planned).await?,
                };

                file.finish(planned, &complete.hash).await?;
                node.manager.finish_file(&offer.transfer_id);
            }

            protocol::Frame::Control(Control::TransferComplete(_)) => break,

            protocol::Frame::Control(Control::Cancel(cancel)) => {
                return Err(Error::Declined(cancel.reason));
            }

            protocol::Frame::Control(other) => {
                return Err(Error::Protocol(format!(
                    "unexpected {} during transfer",
                    protocol::control_name(&other)
                )))
            }
        }
    }

    // Anything still open never got its completion message, so its contents
    // cannot be verified. Leaving the .part behind lets a retry resume.
    if !open.is_empty() {
        return Err(Error::Protocol(format!(
            "{} file(s) ended without a completion message",
            open.len()
        )));
    }

    Ok(())
}

struct OpenFile {
    writer: BufWriter<tokio::fs::File>,
    hasher: blake3::Hasher,
    /// Bytes written so far, including any resumed prefix.
    written: u64,
    /// Hard ceiling from the offer the user approved.
    limit: u64,
}

/// Slack allowed above a file's declared size before the transfer is aborted.
///
/// A file can legitimately grow a little between being offered and being read
/// — a log still being appended to, say — and failing on a single extra byte
/// would be needlessly brittle. What this must not permit is a sender
/// streaming unbounded data into a transfer the user approved as small.
const SIZE_OVERRUN_ALLOWANCE: u64 = 16 * 1024 * 1024;

impl OpenFile {
    async fn create(planned: &PlannedFile) -> Result<Self> {
        if let Some(parent) = planned.partial.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let mut hasher = blake3::Hasher::new();

        let handle = if planned.resume_offset > 0 {
            // Fold the bytes already on disk into the hash so the final check
            // covers the whole file, not just this session's portion.
            hash_existing(&planned.partial, planned.resume_offset, &mut hasher).await?;
            tokio::fs::OpenOptions::new()
                .append(true)
                .open(&planned.partial)
                .await?
        } else {
            tokio::fs::File::create(&planned.partial).await?
        };

        Ok(OpenFile {
            writer: BufWriter::with_capacity(protocol::CHUNK_SIZE, handle),
            hasher,
            written: planned.resume_offset,
            limit: planned
                .declared_size
                .saturating_add(SIZE_OVERRUN_ALLOWANCE),
        })
    }

    async fn write(&mut self, bytes: &[u8]) -> Result<()> {
        // The user approved a specific number of bytes. Without this check a
        // peer could accept a 1 KB offer and then stream until the disk filled,
        // since nothing else bounds how much data a sender may push.
        self.written = self.written.saturating_add(bytes.len() as u64);
        if self.written > self.limit {
            return Err(Error::Protocol(
                "the sender exceeded the file size it offered; transfer aborted".into(),
            ));
        }

        self.hasher.update(bytes);
        self.writer.write_all(bytes).await?;
        Ok(())
    }

    /// Flush, verify, and promote the partial file to its final name.
    async fn finish(mut self, planned: &PlannedFile, expected_hash: &str) -> Result<()> {
        self.writer.flush().await?;

        let file = self.writer.into_inner();
        file.sync_all().await?;

        // Close the handle before renaming. Dropping a `tokio::fs::File` hands
        // the close to the blocking pool, so the handle can still be open when
        // the rename runs — and Windows refuses to rename an open file with
        // "Access is denied". Converting to a std File and dropping that closes
        // it synchronously, right here.
        drop(file.into_std().await);

        let actual = self.hasher.finalize().to_hex().to_string();
        if actual != expected_hash {
            // Corrupt, truncated, or tampered with. Delete rather than keep,
            // so a retry starts clean instead of resuming onto bad bytes.
            let _ = tokio::fs::remove_file(&planned.partial).await;
            return Err(Error::Protocol(format!(
                "{} failed its integrity check and was discarded",
                planned.final_path.display()
            )));
        }

        // Pick the free name only now, so a transfer that fails verification
        // never reserves a slot next to the user's existing files.
        let destination = paths::unique_path(&planned.final_path);
        tokio::fs::rename(&planned.partial, &destination).await?;

        strip_executable_bit(&destination).await;
        Ok(())
    }
}

async fn hash_existing(path: &std::path::Path, length: u64, hasher: &mut blake3::Hasher) -> Result<()> {
    let mut handle = tokio::fs::File::open(path).await?;
    let mut buffer = vec![0u8; protocol::CHUNK_SIZE];
    let mut remaining = length;

    while remaining > 0 {
        let want = remaining.min(buffer.len() as u64) as usize;
        let read = handle.read(&mut buffer[..want]).await?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
        remaining -= read as u64;
    }

    Ok(())
}

/// Received files are data, never programs. Clearing the executable bit means
/// a file that arrives cannot be run by a stray double-click, and reinforces
/// that fluqsr only ever reveals files in a folder — it does not open them.
async fn strip_executable_bit(path: &std::path::Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(metadata) = tokio::fs::metadata(path).await {
            let mut permissions = metadata.permissions();
            permissions.set_mode(permissions.mode() & !0o111);
            let _ = tokio::fs::set_permissions(path, permissions).await;
        }
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::OfferedFile;

    fn temp_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fluqsr-recv-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn offer(paths: &[(&str, u64)]) -> Offer {
        Offer {
            transfer_id: "t1".into(),
            total_bytes: paths.iter().map(|(_, size)| size).sum(),
            files: paths
                .iter()
                .enumerate()
                .map(|(index, (path, size))| OfferedFile {
                    index: index as u32,
                    path: (*path).to_string(),
                    size: *size,
                })
                .collect(),
        }
    }

    #[test]
    fn plans_ordinary_files() {
        let dir = temp_dir("plan");
        let plan = build_plan(&offer(&[("a.txt", 10), ("sub/b.txt", 20)]), &dir).unwrap();

        assert_eq!(plan.len(), 2);
        assert!(plan[&0].final_path.starts_with(&dir));
        assert!(plan[&1].final_path.starts_with(&dir));

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn refuses_a_traversal_in_an_offer() {
        // The headline attack: a sender naming a file that would land outside
        // the receive folder.
        let dir = temp_dir("plan-traversal");

        for evil in [
            "../escaped.txt",
            "../../../../etc/passwd",
            "/etc/passwd",
            "C:/Windows/System32/evil.dll",
        ] {
            let result = build_plan(&offer(&[(evil, 10)]), &dir);
            assert!(
                result.is_err(),
                "planning must reject {evil} rather than write it"
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn every_planned_path_stays_inside_the_receive_folder() {
        let dir = temp_dir("plan-contained");
        let plan = build_plan(
            &offer(&[("deep/nested/tree/file.bin", 1), ("top.txt", 1)]),
            &dir,
        )
        .unwrap();

        let canonical = dir.canonicalize().unwrap();
        for file in plan.values() {
            let resolved = file
                .final_path
                .parent()
                .and_then(|p| p.canonicalize().ok())
                .unwrap_or_else(|| canonical.clone());
            assert!(
                resolved.starts_with(&canonical) || file.final_path.starts_with(&dir),
                "{:?} escaped the receive folder",
                file.final_path
            );
        }

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn rejects_a_duplicate_file_index() {
        let dir = temp_dir("plan-dupe");
        let mut duplicated = offer(&[("a.txt", 1), ("b.txt", 1)]);
        duplicated.files[1].index = 0;

        assert!(build_plan(&duplicated, &dir).is_err());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_fresh_file_starts_at_offset_zero() {
        let dir = temp_dir("plan-fresh");
        let plan = build_plan(&offer(&[("a.txt", 100)]), &dir).unwrap();
        assert_eq!(plan[&0].resume_offset, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_interrupted_file_resumes_from_its_partial() {
        let dir = temp_dir("plan-resume");
        let partial = paths::partial_path(&dir.join("movie.mkv"));
        std::fs::write(&partial, vec![0u8; 512]).unwrap();

        let plan = build_plan(&offer(&[("movie.mkv", 4096)]), &dir).unwrap();
        assert_eq!(plan[&0].resume_offset, 512);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn an_oversized_partial_restarts_instead_of_resuming() {
        // A leftover .part longer than the incoming file belongs to something
        // else; resuming onto it would splice two files together.
        let dir = temp_dir("plan-oversized");
        let partial = paths::partial_path(&dir.join("a.txt"));
        std::fs::write(&partial, vec![0u8; 9999]).unwrap();

        let plan = build_plan(&offer(&[("a.txt", 100)]), &dir).unwrap();
        assert_eq!(plan[&0].resume_offset, 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn declared_size_is_recorded_for_progress() {
        let dir = temp_dir("plan-size");
        let plan = build_plan(&offer(&[("a.txt", 4242)]), &dir).unwrap();
        assert_eq!(plan[&0].declared_size, 4242);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_verified_file_is_promoted_and_the_partial_removed() {
        let dir = temp_dir("finish-ok");
        let plan = build_plan(&offer(&[("hello.txt", 5)]), &dir).unwrap();
        let planned = &plan[&0];

        let mut open = OpenFile::create(planned).await.unwrap();
        open.write(b"hello").await.unwrap();
        let hash = blake3::hash(b"hello").to_hex().to_string();
        open.finish(planned, &hash).await.unwrap();

        assert_eq!(std::fs::read(dir.join("hello.txt")).unwrap(), b"hello");
        assert!(!planned.partial.exists(), "the .part file must be cleaned up");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_corrupt_file_is_discarded_not_kept() {
        let dir = temp_dir("finish-corrupt");
        let plan = build_plan(&offer(&[("hello.txt", 5)]), &dir).unwrap();
        let planned = &plan[&0];

        let mut open = OpenFile::create(planned).await.unwrap();
        open.write(b"tampered").await.unwrap();

        let wrong_hash = blake3::hash(b"hello").to_hex().to_string();
        assert!(open.finish(planned, &wrong_hash).await.is_err());

        assert!(!dir.join("hello.txt").exists(), "no file should be promoted");
        assert!(!planned.partial.exists(), "bad bytes must not be left to resume onto");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn an_existing_file_is_never_overwritten() {
        let dir = temp_dir("finish-collide");
        std::fs::write(dir.join("hello.txt"), b"original").unwrap();

        let plan = build_plan(&offer(&[("hello.txt", 3)]), &dir).unwrap();
        let planned = &plan[&0];

        let mut open = OpenFile::create(planned).await.unwrap();
        open.write(b"new").await.unwrap();
        open.finish(planned, blake3::hash(b"new").to_hex().as_str())
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(dir.join("hello.txt")).unwrap(),
            b"original",
            "a sender must not be able to replace a file by naming it"
        );
        assert_eq!(std::fs::read(dir.join("hello (1).txt")).unwrap(), b"new");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_zero_byte_file_round_trips() {
        let dir = temp_dir("finish-empty");
        let plan = build_plan(&offer(&[("empty.bin", 0)]), &dir).unwrap();
        let planned = &plan[&0];

        let open = OpenFile::create(planned).await.unwrap();
        open.finish(planned, blake3::hash(b"").to_hex().as_str())
            .await
            .unwrap();

        assert!(dir.join("empty.bin").exists());
        assert_eq!(std::fs::metadata(dir.join("empty.bin")).unwrap().len(), 0);

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn nested_directories_are_created() {
        let dir = temp_dir("finish-nested");
        let plan = build_plan(&offer(&[("a/b/c/deep.txt", 4)]), &dir).unwrap();
        let planned = &plan[&0];

        let mut open = OpenFile::create(planned).await.unwrap();
        open.write(b"deep").await.unwrap();
        open.finish(planned, blake3::hash(b"deep").to_hex().as_str())
            .await
            .unwrap();

        assert_eq!(
            std::fs::read(dir.join("a").join("b").join("c").join("deep.txt")).unwrap(),
            b"deep"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_sender_cannot_exceed_the_size_it_offered() {
        // Disk-fill guard: the user approved a small transfer, so the sender
        // must not be able to stream indefinitely into it.
        let dir = temp_dir("overrun");
        let plan = build_plan(&offer(&[("small.bin", 10)]), &dir).unwrap();
        let planned = &plan[&0];

        let mut open = OpenFile::create(planned).await.unwrap();
        let chunk = vec![0u8; 1024 * 1024];

        let mut hit_limit = false;
        for _ in 0..64 {
            if open.write(&chunk).await.is_err() {
                hit_limit = true;
                break;
            }
        }

        assert!(
            hit_limit,
            "writing far beyond the declared size must be refused"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn a_file_that_grew_slightly_is_still_accepted() {
        // The flip side: a file appended to between offer and read should not
        // fail on a handful of extra bytes.
        let dir = temp_dir("overrun-slack");
        let plan = build_plan(&offer(&[("log.txt", 4)]), &dir).unwrap();
        let planned = &plan[&0];

        let mut open = OpenFile::create(planned).await.unwrap();
        assert!(open.write(b"more than four bytes").await.is_ok());

        std::fs::remove_dir_all(&dir).ok();
    }

    #[tokio::test]
    async fn resuming_hashes_the_bytes_already_on_disk() {
        // The whole file must hash correctly, not just the resumed portion.
        let dir = temp_dir("finish-resume");
        let partial = paths::partial_path(&dir.join("movie.bin"));
        std::fs::write(&partial, b"first-half-").unwrap();

        let plan = build_plan(&offer(&[("movie.bin", 22)]), &dir).unwrap();
        let planned = &plan[&0];
        assert_eq!(planned.resume_offset, 11);

        let mut open = OpenFile::create(planned).await.unwrap();
        open.write(b"second-half").await.unwrap();

        let whole = blake3::hash(b"first-half-second-half").to_hex().to_string();
        open.finish(planned, &whole).await.unwrap();

        assert_eq!(
            std::fs::read(dir.join("movie.bin")).unwrap(),
            b"first-half-second-half"
        );

        std::fs::remove_dir_all(&dir).ok();
    }
}
