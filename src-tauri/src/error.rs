use serde::{Serialize, Serializer};

pub type Result<T> = std::result::Result<T, Error>;

/// Every fallible path in the app funnels through this. It serializes to a
/// plain string so Tauri commands can return it straight to the frontend
/// without the UI needing to know about Rust error shapes.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("malformed data: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("tls error: {0}")]
    Tls(#[from] rustls::Error),

    #[error("certificate error: {0}")]
    Certificate(String),

    #[error("discovery error: {0}")]
    Discovery(String),

    /// The peer spoke something we could not parse, or violated the protocol.
    #[error("protocol error: {0}")]
    Protocol(String),

    /// The peer's key does not match the fingerprint we have pinned for it.
    /// This is the signature of a man-in-the-middle, not a routine failure.
    #[error("identity mismatch for {device_id}: expected {expected}, got {actual}")]
    IdentityMismatch {
        device_id: String,
        expected: String,
        actual: String,
    },

    /// An incoming path failed validation. Refusing is always correct here.
    #[error("rejected unsafe path: {0}")]
    UnsafePath(String),

    #[error("peer not found: {0}")]
    UnknownPeer(String),

    #[error("transfer not found: {0}")]
    UnknownTransfer(String),

    #[error("transfer declined by peer{}", .0.as_ref().map(|r| format!(": {r}")).unwrap_or_default())]
    Declined(Option<String>),

    #[error("transfer cancelled")]
    Cancelled,

    #[error("{0}")]
    Other(String),
}

impl Serialize for Error {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl From<rcgen::Error> for Error {
    fn from(err: rcgen::Error) -> Self {
        Error::Certificate(err.to_string())
    }
}

impl From<String> for Error {
    fn from(err: String) -> Self {
        Error::Other(err)
    }
}

impl From<&str> for Error {
    fn from(err: &str) -> Self {
        Error::Other(err.to_string())
    }
}
