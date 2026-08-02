//! The sending half of a transfer.

use std::net::SocketAddr;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt, BufWriter};
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;

use crate::error::{Error, Result};
use crate::protocol::{
    self, Accept, Control, Hello, Offer, OfferedFile, TransferComplete, CHUNK_SIZE,
    PROTOCOL_VERSION,
};
use crate::tls;

use super::gate::{self, Node};
use super::{Direction, FileEntry, Status};

/// Giving up on a peer that stops responding mid-connection. Generous, because
/// the far side may legitimately be waiting on a human to answer a prompt.
const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct SendRequest {
    pub transfer_id: String,
    pub target: SocketAddr,
    pub peer_name: String,
    pub files: Vec<FileEntry>,
}

/// Connect to a peer, get consent, and stream the files.
///
/// Progress and failures are reported through the [`TransferManager`] rather
/// than the return value, since the UI is driven by events; the `Result` is
/// for the caller's own logging.
///
/// [`TransferManager`]: super::TransferManager
pub async fn send(node: Node, request: SendRequest) -> Result<()> {
    let transfer_id = request.transfer_id.clone();

    match send_inner(&node, request).await {
        Ok(()) => {
            node.manager.complete(&transfer_id);
            Ok(())
        }
        Err(Error::Cancelled) => {
            node.manager.set_status(&transfer_id, Status::Cancelled);
            Err(Error::Cancelled)
        }
        Err(Error::Declined(reason)) => {
            node.manager.set_status(&transfer_id, Status::Declined);
            Err(Error::Declined(reason))
        }
        Err(err) => {
            node.manager.fail(&transfer_id, &err.to_string());
            Err(err)
        }
    }
}

async fn send_inner(node: &Node, request: SendRequest) -> Result<()> {
    let SendRequest {
        transfer_id,
        target,
        files,
        ..
    } = request;

    let tcp = tokio::time::timeout(CONNECT_TIMEOUT, TcpStream::connect(target))
        .await
        .map_err(|_| Error::Other(format!("timed out connecting to {target}")))??;

    // Chunks are large and written back to back; Nagle's algorithm only adds
    // latency to the control messages between them.
    tcp.set_nodelay(true)?;

    let connector = TlsConnector::from(tls::client_config(&node.identity)?);
    let mut stream = connector.connect(tls::server_name()?, tcp).await?;

    // The handshake proved the peer holds this key. It has not yet told us
    // whether the key belongs to anyone we trust.
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

    node.manager
        .set_status(&transfer_id, Status::AwaitingApproval);

    // Nothing has been sent about the files yet, and nothing will be until
    // this returns Ok.
    let peer = gate::authenticate(node, presented, &theirs, Direction::Send).await?;

    if node.manager.is_cancelled(&transfer_id) {
        return Err(Error::Cancelled);
    }

    let offer = Offer {
        transfer_id: transfer_id.clone(),
        total_bytes: files.iter().map(|f| f.size).sum(),
        files: files
            .iter()
            .map(|file| OfferedFile {
                index: file.index,
                path: file.relative.clone(),
                size: file.size,
            })
            .collect(),
    };

    protocol::write_control(&mut stream, &Control::Offer(offer)).await?;
    stream.flush().await?;

    // Blocks until the far side's user answers, which is why there is no
    // timeout here.
    let accept = match protocol::read_control(&mut stream).await? {
        Control::Accept(accept) => accept,
        Control::Reject(reject) => return Err(Error::Declined(reject.reason)),
        other => {
            return Err(Error::Protocol(format!(
                "expected Accept or Reject from {}, got {}",
                peer.device_name,
                protocol::control_name(&other)
            )))
        }
    };

    stream_files(node, &transfer_id, &files, &accept, &mut stream).await?;

    protocol::write_control(
        &mut stream,
        &Control::TransferComplete(TransferComplete {
            transfer_id: transfer_id.clone(),
        }),
    )
    .await?;
    stream.flush().await?;

    // Wait for the receiver to echo the completion back.
    //
    // Without this the send would report success as soon as the last byte was
    // written to the socket, which says nothing about whether the far side
    // verified the hashes and committed the files. A transfer is only
    // finished once the receiver says it is.
    match protocol::read_control(&mut stream).await? {
        Control::TransferComplete(_) => {}
        Control::Cancel(cancel) => return Err(Error::Declined(cancel.reason)),
        other => {
            return Err(Error::Protocol(format!(
                "expected the receiver to confirm completion, got {}",
                protocol::control_name(&other)
            )))
        }
    }

    let _ = stream.shutdown().await;

    Ok(())
}

async fn stream_files<S>(
    node: &Node,
    transfer_id: &str,
    files: &[FileEntry],
    accept: &Accept,
    stream: &mut S,
) -> Result<()>
where
    S: tokio::io::AsyncWrite + Unpin,
{
    // Buffer the socket so a partially-filled final chunk does not cost an
    // extra syscall, and control frames coalesce with the data around them.
    let mut writer = BufWriter::with_capacity(CHUNK_SIZE, stream);
    let mut buffer = vec![0u8; CHUNK_SIZE];

    for accepted in &accept.files {
        // The receiver chooses which files it wants, so an index we do not
        // recognise is a protocol error rather than something to guess at.
        let file = files
            .iter()
            .find(|f| f.index == accepted.index)
            .ok_or_else(|| {
                Error::Protocol(format!(
                    "peer accepted file index {} which was never offered",
                    accepted.index
                ))
            })?;

        node.manager.begin_file(transfer_id, &file.relative);

        let mut handle = tokio::fs::File::open(&file.source).await.map_err(|err| {
            Error::Other(format!("could not read {}: {err}", file.source.display()))
        })?;

        let mut hasher = blake3::Hasher::new();

        // Resume: the receiver already holds these bytes, but they are still
        // part of the file's hash, so they have to go through the hasher even
        // though they are not sent again.
        if accepted.resume_offset > 0 {
            hash_prefix(&mut handle, accepted.resume_offset, &mut hasher, &mut buffer).await?;
            node.manager.advance(transfer_id, accepted.resume_offset);
        }

        loop {
            if node.manager.is_cancelled(transfer_id) {
                return Err(Error::Cancelled);
            }

            let read = handle.read(&mut buffer).await?;
            if read == 0 {
                break;
            }

            hasher.update(&buffer[..read]);
            protocol::write_data(&mut writer, file.index, &buffer[..read]).await?;
            node.manager.advance(transfer_id, read as u64);
        }

        protocol::write_control(
            &mut writer,
            &Control::FileComplete(protocol::FileComplete {
                index: file.index,
                hash: hasher.finalize().to_hex().to_string(),
            }),
        )
        .await?;

        node.manager.finish_file(transfer_id);
    }

    writer.flush().await?;
    Ok(())
}

/// Read and hash the first `length` bytes without sending them, then leave the
/// file positioned to continue.
async fn hash_prefix(
    handle: &mut tokio::fs::File,
    length: u64,
    hasher: &mut blake3::Hasher,
    buffer: &mut [u8],
) -> Result<()> {
    let mut remaining = length;

    while remaining > 0 {
        let want = remaining.min(buffer.len() as u64) as usize;
        let read = handle.read(&mut buffer[..want]).await?;
        if read == 0 {
            // The file shrank since the receiver last saw it, so resuming into
            // it would produce a corrupt result.
            return Err(Error::Other(
                "the file changed since the interrupted transfer; send it again".into(),
            ));
        }
        hasher.update(&buffer[..read]);
        remaining -= read as u64;
    }

    Ok(())
}
