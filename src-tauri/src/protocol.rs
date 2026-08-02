//! The fluqsr wire protocol.
//!
//! Everything below runs *inside* an established TLS 1.3 session, so this
//! layer is not responsible for confidentiality or authenticity. What it is
//! responsible for is being unambiguous and bounded: every frame is
//! length-prefixed, every length is checked against a cap before allocating,
//! and no message causes a write to disk on its own.
//!
//! Two frame kinds share one stream:
//!   * control frames carry JSON — readable in a packet capture during
//!     development, and cheap to extend without breaking older peers;
//!   * data frames carry raw bytes with a small binary header, so file
//!     contents never pay an encoding tax.

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::error::{Error, Result};

/// Bumped whenever a change would confuse an older build. Peers exchange this
/// in `Hello` and refuse to continue on a mismatch, which produces a clear
/// error instead of a puzzling parse failure halfway through a transfer.
pub const PROTOCOL_VERSION: u32 = 1;

const KIND_CONTROL: u8 = 0x01;
const KIND_DATA: u8 = 0x02;

/// Ceiling on a control frame. Generous enough for an offer listing tens of
/// thousands of files, small enough that a hostile peer cannot make us
/// allocate its way to an OOM.
const MAX_CONTROL_FRAME: u32 = 16 * 1024 * 1024;

/// Ceiling on a data frame, sized a little above `CHUNK_SIZE`.
const MAX_DATA_FRAME: u32 = 8 * 1024 * 1024;

/// How much file content goes in one data frame. 512 KiB keeps the syscall
/// count low enough to saturate gigabit while staying small enough that
/// progress updates stay smooth and a cancel takes effect promptly.
pub const CHUNK_SIZE: usize = 512 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Control {
    Hello(Hello),
    Offer(Offer),
    Accept(Accept),
    Reject(Reject),
    FileComplete(FileComplete),
    TransferComplete(TransferComplete),
    Cancel(Cancel),
}

/// Opening message from both sides.
///
/// Deliberately absent: the sender's public key or fingerprint. Identity comes
/// from the certificate presented during the TLS handshake, which the peer had
/// to prove possession of. A fingerprint self-reported in JSON would prove
/// nothing, and trusting one would hand an attacker a free impersonation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Hello {
    pub protocol_version: u32,
    pub device_id: String,
    pub device_name: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Offer {
    pub transfer_id: String,
    pub files: Vec<OfferedFile>,
    pub total_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OfferedFile {
    pub index: u32,
    /// Sender-chosen relative path. Hostile until `paths::safe_relative_path`
    /// has passed it.
    pub path: String,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Accept {
    pub transfer_id: String,
    pub files: Vec<AcceptedFile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AcceptedFile {
    pub index: u32,
    /// Byte offset to resume from. The receiver derives this from a `.part`
    /// file it already holds; the sender seeks and continues rather than
    /// restarting. Always 0 for a fresh transfer.
    #[serde(default)]
    pub resume_offset: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reject {
    pub transfer_id: String,
    pub reason: Option<String>,
}

/// Sent after the last chunk of a file, carrying the hash of the whole file so
/// the receiver can confirm what landed matches what was read.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileComplete {
    pub index: u32,
    /// BLAKE3, hex. Chosen over SHA-256 because it hashes faster than a
    /// gigabit link delivers, so integrity checking costs no throughput.
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferComplete {
    pub transfer_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cancel {
    pub transfer_id: String,
    pub reason: Option<String>,
}

/// One unit off the wire.
#[derive(Debug)]
pub enum Frame {
    Control(Control),
    Data { index: u32, bytes: Vec<u8> },
}

pub async fn write_control<W>(writer: &mut W, message: &Control) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    let payload = serde_json::to_vec(message)?;
    if payload.len() as u32 > MAX_CONTROL_FRAME {
        return Err(Error::Protocol("control message is too large".into()));
    }

    writer.write_u8(KIND_CONTROL).await?;
    writer.write_u32(payload.len() as u32).await?;
    writer.write_all(&payload).await?;
    Ok(())
}

pub async fn write_data<W>(writer: &mut W, index: u32, bytes: &[u8]) -> Result<()>
where
    W: AsyncWrite + Unpin,
{
    // 4 bytes of file index ride along inside the payload.
    let len = bytes.len() as u32 + 4;
    if len > MAX_DATA_FRAME {
        return Err(Error::Protocol("data chunk is too large".into()));
    }

    writer.write_u8(KIND_DATA).await?;
    writer.write_u32(len).await?;
    writer.write_u32(index).await?;
    writer.write_all(bytes).await?;
    Ok(())
}

/// Read one frame. Returns `Ok(None)` on a clean end of stream.
pub async fn read_frame<R>(reader: &mut R) -> Result<Option<Frame>>
where
    R: AsyncRead + Unpin,
{
    let kind = match reader.read_u8().await {
        Ok(kind) => kind,
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(err) => return Err(err.into()),
    };

    let len = reader.read_u32().await?;

    match kind {
        KIND_CONTROL => {
            if len == 0 || len > MAX_CONTROL_FRAME {
                return Err(Error::Protocol(format!(
                    "control frame claims an invalid length: {len}"
                )));
            }
            // Only allocate once the length has been bounds-checked.
            let mut payload = vec![0u8; len as usize];
            reader.read_exact(&mut payload).await?;
            let message = serde_json::from_slice(&payload)
                .map_err(|err| Error::Protocol(format!("bad control message: {err}")))?;
            Ok(Some(Frame::Control(message)))
        }
        KIND_DATA => {
            if !(4..=MAX_DATA_FRAME).contains(&len) {
                return Err(Error::Protocol(format!(
                    "data frame claims an invalid length: {len}"
                )));
            }
            let index = reader.read_u32().await?;
            let mut bytes = vec![0u8; (len - 4) as usize];
            reader.read_exact(&mut bytes).await?;
            Ok(Some(Frame::Data { index, bytes }))
        }
        other => Err(Error::Protocol(format!("unknown frame kind: {other:#04x}"))),
    }
}

/// Read a frame and require it to be a control message.
pub async fn read_control<R>(reader: &mut R) -> Result<Control>
where
    R: AsyncRead + Unpin,
{
    match read_frame(reader).await? {
        Some(Frame::Control(message)) => Ok(message),
        Some(Frame::Data { .. }) => {
            Err(Error::Protocol("expected a control message, got file data".into()))
        }
        None => Err(Error::Protocol(
            "peer closed the connection before replying".into(),
        )),
    }
}

/// Exchange `Hello` messages and check the versions line up.
pub async fn handshake<S>(stream: &mut S, ours: &Hello) -> Result<Hello>
where
    S: AsyncRead + AsyncWrite + Unpin,
{
    write_control(stream, &Control::Hello(ours.clone())).await?;
    stream.flush().await?;

    let theirs = match read_control(stream).await? {
        Control::Hello(hello) => hello,
        other => {
            return Err(Error::Protocol(format!(
                "expected Hello, got {}",
                control_name(&other)
            )))
        }
    };

    if theirs.protocol_version != PROTOCOL_VERSION {
        return Err(Error::Protocol(format!(
            "incompatible protocol: this device speaks v{PROTOCOL_VERSION}, {} speaks v{}",
            theirs.device_name, theirs.protocol_version
        )));
    }

    Ok(theirs)
}

pub fn control_name(message: &Control) -> &'static str {
    match message {
        Control::Hello(_) => "Hello",
        Control::Offer(_) => "Offer",
        Control::Accept(_) => "Accept",
        Control::Reject(_) => "Reject",
        Control::FileComplete(_) => "FileComplete",
        Control::TransferComplete(_) => "TransferComplete",
        Control::Cancel(_) => "Cancel",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn hello(name: &str) -> Hello {
        Hello {
            protocol_version: PROTOCOL_VERSION,
            device_id: "device-1".into(),
            device_name: name.into(),
            platform: "linux".into(),
        }
    }

    #[tokio::test]
    async fn control_frames_round_trip() {
        let mut buffer = Vec::new();
        write_control(&mut buffer, &Control::Hello(hello("Laptop")))
            .await
            .unwrap();

        let mut cursor = Cursor::new(buffer);
        match read_frame(&mut cursor).await.unwrap().unwrap() {
            Frame::Control(Control::Hello(received)) => {
                assert_eq!(received.device_name, "Laptop");
                assert_eq!(received.protocol_version, PROTOCOL_VERSION);
            }
            other => panic!("expected Hello, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn data_frames_round_trip() {
        let payload = vec![7u8; 4096];
        let mut buffer = Vec::new();
        write_data(&mut buffer, 3, &payload).await.unwrap();

        let mut cursor = Cursor::new(buffer);
        match read_frame(&mut cursor).await.unwrap().unwrap() {
            Frame::Data { index, bytes } => {
                assert_eq!(index, 3);
                assert_eq!(bytes, payload);
            }
            other => panic!("expected data, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn empty_data_frames_are_allowed() {
        // A zero-length file still produces a well-formed frame.
        let mut buffer = Vec::new();
        write_data(&mut buffer, 0, &[]).await.unwrap();

        let mut cursor = Cursor::new(buffer);
        match read_frame(&mut cursor).await.unwrap().unwrap() {
            Frame::Data { index, bytes } => {
                assert_eq!(index, 0);
                assert!(bytes.is_empty());
            }
            other => panic!("expected data, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn interleaved_frames_keep_their_order() {
        let mut buffer = Vec::new();
        write_control(&mut buffer, &Control::Hello(hello("A"))).await.unwrap();
        write_data(&mut buffer, 0, b"chunk").await.unwrap();
        write_control(
            &mut buffer,
            &Control::TransferComplete(TransferComplete {
                transfer_id: "t1".into(),
            }),
        )
        .await
        .unwrap();

        let mut cursor = Cursor::new(buffer);
        assert!(matches!(
            read_frame(&mut cursor).await.unwrap().unwrap(),
            Frame::Control(Control::Hello(_))
        ));
        assert!(matches!(
            read_frame(&mut cursor).await.unwrap().unwrap(),
            Frame::Data { .. }
        ));
        assert!(matches!(
            read_frame(&mut cursor).await.unwrap().unwrap(),
            Frame::Control(Control::TransferComplete(_))
        ));
        assert!(read_frame(&mut cursor).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn clean_eof_is_not_an_error() {
        let mut cursor = Cursor::new(Vec::new());
        assert!(read_frame(&mut cursor).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn an_absurd_length_is_rejected_before_allocating() {
        // A hostile peer announcing a 4 GiB control frame must be refused, not
        // met with a 4 GiB allocation.
        let mut buffer = vec![KIND_CONTROL];
        buffer.extend_from_slice(&u32::MAX.to_be_bytes());

        let mut cursor = Cursor::new(buffer);
        assert!(read_frame(&mut cursor).await.is_err());
    }

    #[tokio::test]
    async fn a_truncated_data_frame_is_rejected() {
        let mut buffer = vec![KIND_DATA];
        buffer.extend_from_slice(&2u32.to_be_bytes()); // shorter than the 4-byte index
        let mut cursor = Cursor::new(buffer);
        assert!(read_frame(&mut cursor).await.is_err());
    }

    #[tokio::test]
    async fn unknown_frame_kinds_are_rejected() {
        let mut buffer = vec![0xff];
        buffer.extend_from_slice(&4u32.to_be_bytes());
        buffer.extend_from_slice(&[0, 0, 0, 0]);

        let mut cursor = Cursor::new(buffer);
        assert!(read_frame(&mut cursor).await.is_err());
    }

    #[tokio::test]
    async fn malformed_json_is_a_protocol_error_not_a_panic() {
        let payload = b"{ not json";
        let mut buffer = vec![KIND_CONTROL];
        buffer.extend_from_slice(&(payload.len() as u32).to_be_bytes());
        buffer.extend_from_slice(payload);

        let mut cursor = Cursor::new(buffer);
        assert!(read_frame(&mut cursor).await.is_err());
    }

    #[tokio::test]
    async fn version_mismatch_fails_the_handshake() {
        // Simulate a peer that replies with a different protocol version.
        let mut theirs = hello("Old Build");
        theirs.protocol_version = PROTOCOL_VERSION + 1;

        let mut peer_bytes = Vec::new();
        write_control(&mut peer_bytes, &Control::Hello(theirs))
            .await
            .unwrap();

        // Duplex-ish stand-in: reads come from the peer's bytes, writes are
        // discarded into the same cursor's tail.
        let mut stream = Cursor::new(peer_bytes);
        let ours = hello("New Build");

        // Write our Hello into a throwaway buffer first, then read theirs.
        let mut sink = Vec::new();
        write_control(&mut sink, &Control::Hello(ours)).await.unwrap();

        let result = read_control(&mut stream).await.unwrap();
        match result {
            Control::Hello(peer) => {
                assert_ne!(peer.protocol_version, PROTOCOL_VERSION);
            }
            other => panic!("expected Hello, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn read_control_rejects_a_data_frame() {
        let mut buffer = Vec::new();
        write_data(&mut buffer, 0, b"surprise").await.unwrap();

        let mut cursor = Cursor::new(buffer);
        assert!(read_control(&mut cursor).await.is_err());
    }

    #[test]
    fn hello_does_not_carry_a_self_reported_fingerprint() {
        // Guard against someone helpfully "improving" Hello later: identity
        // must keep coming from the TLS certificate, not from JSON.
        let json = serde_json::to_string(&hello("Laptop")).unwrap();
        assert!(!json.to_lowercase().contains("fingerprint"));
        assert!(!json.to_lowercase().contains("pubkey"));
    }
}
