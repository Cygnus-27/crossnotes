//! Mutual TLS 1.3 between two devices that share no certificate authority.
//!
//! ## Why the verifiers accept every certificate
//!
//! There is no CA in this system and there cannot be one — the peers are two
//! laptops on a hotel network, not services with a chain of trust. So the
//! usual X.509 validation has nothing to validate against, and both verifiers
//! below deliberately return "valid" for any well-formed certificate.
//!
//! That is only safe because of what happens immediately afterwards. TLS still
//! does the part that matters: it proves the peer holds the private key for
//! the certificate it presented, and it encrypts the session against that key.
//! What TLS cannot tell us is whether that key belongs to the device we meant
//! to talk to. `PeerIdentity::from_connection` answers that by extracting the
//! key's fingerprint, and every caller must then check it against the trust
//! store before sending or writing anything.
//!
//! The invariant, stated once: **a TLS handshake completing means nothing on
//! its own. Nothing may act on a connection until the fingerprint has been
//! checked.**
//!
//! Both sides require a client certificate, so this identification works in
//! both directions — the receiver learns who is calling, and the sender learns
//! who answered.

use std::sync::Arc;

use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName, UnixTime};
use rustls::server::danger::{ClientCertVerified, ClientCertVerifier};
use rustls::{ClientConfig, DigitallySignedStruct, ServerConfig, SignatureScheme};

use crate::crypto::{fingerprint_from_cert_der, Fingerprint};
use crate::device::Identity;
use crate::error::{Error, Result};

/// The name the client asks for. Never validated against the certificate —
/// see the module comment — but TLS requires one to be supplied.
pub const SNI_NAME: &str = "fluqsr.local";

/// Install the process-wide crypto provider. Safe to call more than once.
pub fn init_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

fn provider() -> Arc<rustls::crypto::CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

/// Accepts any syntactically valid certificate, then defers the real decision
/// to the fingerprint check the caller performs after the handshake.
#[derive(Debug)]
struct DeferToFingerprint {
    provider: Arc<rustls::crypto::CryptoProvider>,
}

impl DeferToFingerprint {
    fn new() -> Arc<Self> {
        Arc::new(DeferToFingerprint {
            provider: provider(),
        })
    }

    fn verify_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
        tls13: bool,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        // This part is genuinely verified: it proves the peer holds the
        // private key matching the certificate whose fingerprint we are about
        // to pin. Without it, anyone could replay someone else's certificate.
        let algorithms = &self.provider.signature_verification_algorithms;
        if tls13 {
            rustls::crypto::verify_tls13_signature(message, cert, dss, algorithms)
        } else {
            rustls::crypto::verify_tls12_signature(message, cert, dss, algorithms)
        }
    }

    fn schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

impl ServerCertVerifier for DeferToFingerprint {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.verify_signature(message, cert, dss, false)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.verify_signature(message, cert, dss, true)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }
}

impl ClientCertVerifier for DeferToFingerprint {
    fn root_hint_subjects(&self) -> &[rustls::DistinguishedName] {
        // No CA, so no subjects to hint at. Peers always send their one
        // self-signed certificate regardless.
        &[]
    }

    fn verify_client_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _now: UnixTime,
    ) -> std::result::Result<ClientCertVerified, rustls::Error> {
        Ok(ClientCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.verify_signature(message, cert, dss, false)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        self.verify_signature(message, cert, dss, true)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.schemes()
    }

    fn offer_client_auth(&self) -> bool {
        true
    }

    /// Mandatory, not optional. A connection where we cannot identify the
    /// caller is useless to us — we would have nothing to check against the
    /// trust store.
    fn client_auth_mandatory(&self) -> bool {
        true
    }
}

fn own_credentials(identity: &Identity) -> Result<(Vec<CertificateDer<'static>>, PrivateKeyDer<'static>)> {
    let bundle = identity.key.certificate(&identity.device_id)?;
    let cert = CertificateDer::from(bundle.cert_der);
    let key = PrivateKeyDer::try_from(bundle.key_der)
        .map_err(|err| Error::Certificate(format!("could not encode private key: {err}")))?;
    Ok((vec![cert], key))
}

/// Server side of a transfer connection.
pub fn server_config(identity: &Identity) -> Result<Arc<ServerConfig>> {
    let (certs, key) = own_credentials(identity)?;

    let config = ServerConfig::builder_with_provider(provider())
        // TLS 1.3 only. There is no legacy peer to accommodate, so there is no
        // reason to carry TLS 1.2's weaker cipher suites and renegotiation.
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|err| Error::Certificate(err.to_string()))?
        .with_client_cert_verifier(DeferToFingerprint::new())
        .with_single_cert(certs, key)?;

    Ok(Arc::new(config))
}

/// Client side of a transfer connection.
pub fn client_config(identity: &Identity) -> Result<Arc<ClientConfig>> {
    let (certs, key) = own_credentials(identity)?;

    let config = ClientConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|err| Error::Certificate(err.to_string()))?
        .dangerous()
        .with_custom_certificate_verifier(DeferToFingerprint::new())
        .with_client_auth_cert(certs, key)?;

    Ok(Arc::new(config))
}

pub fn server_name() -> Result<ServerName<'static>> {
    ServerName::try_from(SNI_NAME)
        .map_err(|err| Error::Certificate(format!("invalid server name: {err}")))
}

/// The peer's cryptographic identity, taken from the certificate it proved
/// possession of during the handshake.
#[derive(Debug, Clone)]
pub struct PeerIdentity {
    pub fingerprint: Fingerprint,
}

impl PeerIdentity {
    /// Extract the peer's fingerprint from a completed handshake.
    ///
    /// Fails when the peer sent no certificate. That should be impossible
    /// given `client_auth_mandatory`, but treating it as an error rather than
    /// an `Option` means a caller cannot accidentally proceed without an
    /// identity to check.
    pub fn from_certs(certs: Option<&[CertificateDer<'_>]>) -> Result<Self> {
        let end_entity = certs
            .and_then(|certs| certs.first())
            .ok_or_else(|| Error::Certificate("peer presented no certificate".into()))?;

        Ok(PeerIdentity {
            fingerprint: fingerprint_from_cert_der(end_entity)?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn temp_identity(tag: &str) -> (PathBuf, Identity) {
        let dir = std::env::temp_dir().join(format!("fluqsr-tls-{tag}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let identity = Identity::load_or_create(&dir).unwrap();
        (dir, identity)
    }

    #[test]
    fn builds_a_server_config() {
        init_crypto_provider();
        let (dir, identity) = temp_identity("server");
        assert!(server_config(&identity).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn builds_a_client_config() {
        init_crypto_provider();
        let (dir, identity) = temp_identity("client");
        assert!(client_config(&identity).is_ok());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn peer_identity_matches_the_presenting_device() {
        init_crypto_provider();
        let (dir, identity) = temp_identity("identity");

        let bundle = identity.key.certificate(&identity.device_id).unwrap();
        let certs = vec![CertificateDer::from(bundle.cert_der)];
        let peer = PeerIdentity::from_certs(Some(&certs)).unwrap();

        assert_eq!(peer.fingerprint, identity.fingerprint());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn a_missing_certificate_is_an_error_not_an_absence() {
        init_crypto_provider();
        assert!(PeerIdentity::from_certs(None).is_err());
        assert!(PeerIdentity::from_certs(Some(&[])).is_err());
    }

    #[test]
    fn different_devices_produce_different_peer_identities() {
        init_crypto_provider();
        let (dir_a, a) = temp_identity("distinct-a");
        let (dir_b, b) = temp_identity("distinct-b");

        let cert_a = vec![CertificateDer::from(
            a.key.certificate(&a.device_id).unwrap().cert_der,
        )];
        let cert_b = vec![CertificateDer::from(
            b.key.certificate(&b.device_id).unwrap().cert_der,
        )];

        assert_ne!(
            PeerIdentity::from_certs(Some(&cert_a)).unwrap().fingerprint,
            PeerIdentity::from_certs(Some(&cert_b)).unwrap().fingerprint
        );

        std::fs::remove_dir_all(&dir_a).ok();
        std::fs::remove_dir_all(&dir_b).ok();
    }

    #[test]
    fn server_name_is_constructible() {
        assert!(server_name().is_ok());
    }
}
