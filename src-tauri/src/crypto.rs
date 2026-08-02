//! Device keys, certificate generation, and the fingerprints everything else
//! authenticates against.
//!
//! The model is deliberately CA-free. Each device holds one long-lived Ed25519
//! keypair. Its identity *is* the SHA-256 of that key's SubjectPublicKeyInfo
//! (an "SPKI pin"). Certificates are self-signed wrappers around that key and
//! carry no authority of their own — they exist only because TLS requires a
//! certificate. Trust comes from the pin, established once during pairing.

use std::fmt;

use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair, PKCS_ED25519};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::error::{Error, Result};

/// SHA-256 over a SubjectPublicKeyInfo DER. Comparing two of these is the only
/// thing that decides whether a peer is who it claims to be.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint([u8; 32]);

impl Fingerprint {
    pub fn from_spki_der(spki_der: &[u8]) -> Self {
        let digest = Sha256::digest(spki_der);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&digest);
        Fingerprint(bytes)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Full 64-character lowercase hex, for storage and exact comparison.
    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{b:02x}")).collect()
    }

    /// The first 8 bytes as `abcd-ef01-2345-6789`. Short enough to show in a
    /// peer list, long enough (64 bits) that collisions are not a practical
    /// concern. Never use this for an equality check — use the full hex.
    pub fn to_short(&self) -> String {
        self.0[..8]
            .chunks(2)
            .map(|pair| format!("{:02x}{:02x}", pair[0], pair[1]))
            .collect::<Vec<_>>()
            .join("-")
    }

    pub fn parse_hex(hex: &str) -> Result<Self> {
        let hex = hex.trim();
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(Error::Certificate(format!("invalid fingerprint: {hex}")));
        }
        let mut bytes = [0u8; 32];
        for (index, slot) in bytes.iter_mut().enumerate() {
            let start = index * 2;
            *slot = u8::from_str_radix(&hex[start..start + 2], 16)
                .map_err(|_| Error::Certificate("invalid fingerprint hex".into()))?;
        }
        Ok(Fingerprint(bytes))
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fingerprint({})", self.to_short())
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_hex())
    }
}

impl Serialize for Fingerprint {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_hex())
    }
}

impl<'de> Deserialize<'de> for Fingerprint {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let hex = String::deserialize(d)?;
        Fingerprint::parse_hex(&hex).map_err(serde::de::Error::custom)
    }
}

/// Derive the six-digit Short Authentication String both sides display during
/// pairing.
///
/// This is the whole anti-MITM story, so the details matter. The code is a
/// function of *both* fingerprints, sorted so the two peers agree on the order
/// regardless of who dialed. An attacker sitting in the middle necessarily
/// presents a different key to each side, so the two devices compute different
/// codes and the mismatch is visible to the user. A code derived from only one
/// side, or from a shared secret the attacker helped choose, would not have
/// this property.
pub fn pairing_code(a: &Fingerprint, b: &Fingerprint) -> String {
    let (first, second) = if a.as_bytes() <= b.as_bytes() {
        (a, b)
    } else {
        (b, a)
    };

    let mut hasher = Sha256::new();
    hasher.update(b"fluqsr-pairing-v1");
    hasher.update(first.as_bytes());
    hasher.update(second.as_bytes());
    let digest = hasher.finalize();

    // 20 bits of entropy, matching the strength conventional SAS schemes use.
    let value = u32::from_be_bytes([0, digest[0], digest[1], digest[2]]) % 1_000_000;
    format!("{value:06}")
}

/// A device's long-lived keypair, held as PKCS#8 PEM so it round-trips to disk.
pub struct DeviceKey {
    key_pair: KeyPair,
    fingerprint: Fingerprint,
}

impl DeviceKey {
    pub fn generate() -> Result<Self> {
        Self::from_key_pair(KeyPair::generate_for(&PKCS_ED25519)?)
    }

    pub fn from_pem(pem: &str) -> Result<Self> {
        Self::from_key_pair(KeyPair::from_pem(pem)?)
    }

    fn from_key_pair(key_pair: KeyPair) -> Result<Self> {
        let fingerprint = Fingerprint::from_spki_der(key_pair.public_key_der().as_slice());
        Ok(DeviceKey {
            key_pair,
            fingerprint,
        })
    }

    pub fn to_pem(&self) -> String {
        self.key_pair.serialize_pem()
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.fingerprint
    }

    /// Build the self-signed certificate presented on both sides of the
    /// connection. The subject is cosmetic — it appears in logs and nothing
    /// authenticates against it. Validity dates are wide because an expiry
    /// check would only ever break transfers between two devices that already
    /// trust each other's keys.
    pub fn certificate(&self, device_id: &str) -> Result<CertificateBundle> {
        let mut params = CertificateParams::new(vec!["fluqsr.local".to_string()])?;
        let mut name = DistinguishedName::new();
        name.push(DnType::CommonName, format!("fluqsr:{device_id}"));
        params.distinguished_name = name;

        let cert = params.self_signed(&self.key_pair)?;
        Ok(CertificateBundle {
            cert_der: cert.der().to_vec(),
            key_der: self.key_pair.serialize_der(),
        })
    }
}

pub struct CertificateBundle {
    pub cert_der: Vec<u8>,
    pub key_der: Vec<u8>,
}

/// Pull the SPKI fingerprint out of a peer's certificate.
///
/// We fingerprint the SubjectPublicKeyInfo rather than the whole certificate
/// DER on purpose: the key is the stable identity, while the surrounding
/// certificate can be re-encoded (different serial, validity, or even a
/// different rcgen version) without the device having changed at all.
pub fn fingerprint_from_cert_der(cert_der: &[u8]) -> Result<Fingerprint> {
    let (_, cert) = x509_parser::parse_x509_certificate(cert_der)
        .map_err(|err| Error::Certificate(format!("could not parse peer certificate: {err}")))?;
    Ok(Fingerprint::from_spki_der(cert.public_key().raw))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_survives_a_hex_round_trip() {
        let key = DeviceKey::generate().unwrap();
        let parsed = Fingerprint::parse_hex(&key.fingerprint().to_hex()).unwrap();
        assert_eq!(parsed, key.fingerprint());
    }

    #[test]
    fn rejects_malformed_fingerprints() {
        assert!(Fingerprint::parse_hex("abc").is_err());
        assert!(Fingerprint::parse_hex(&"z".repeat(64)).is_err());
    }

    #[test]
    fn key_survives_a_pem_round_trip() {
        let key = DeviceKey::generate().unwrap();
        let reloaded = DeviceKey::from_pem(&key.to_pem()).unwrap();
        assert_eq!(reloaded.fingerprint(), key.fingerprint());
    }

    #[test]
    fn certificate_carries_the_identity_key() {
        let key = DeviceKey::generate().unwrap();
        let bundle = key.certificate("device-a").unwrap();
        let from_cert = fingerprint_from_cert_der(&bundle.cert_der).unwrap();
        assert_eq!(
            from_cert,
            key.fingerprint(),
            "pinning the SPKI must match the key that signed the certificate"
        );
    }

    #[test]
    fn pairing_code_does_not_depend_on_who_dialled() {
        let a = DeviceKey::generate().unwrap().fingerprint();
        let b = DeviceKey::generate().unwrap().fingerprint();
        assert_eq!(pairing_code(&a, &b), pairing_code(&b, &a));
    }

    #[test]
    fn pairing_code_changes_when_a_key_is_substituted() {
        // The MITM case: the attacker terminates TLS and presents its own key
        // to each side, so at least one side must see a different code.
        let honest_a = DeviceKey::generate().unwrap().fingerprint();
        let honest_b = DeviceKey::generate().unwrap().fingerprint();
        let attacker = DeviceKey::generate().unwrap().fingerprint();

        let genuine = pairing_code(&honest_a, &honest_b);
        assert_ne!(genuine, pairing_code(&honest_a, &attacker));
        assert_ne!(genuine, pairing_code(&attacker, &honest_b));
    }

    #[test]
    fn pairing_code_is_always_six_digits() {
        for _ in 0..64 {
            let a = DeviceKey::generate().unwrap().fingerprint();
            let b = DeviceKey::generate().unwrap().fingerprint();
            let code = pairing_code(&a, &b);
            assert_eq!(code.len(), 6);
            assert!(code.chars().all(|c| c.is_ascii_digit()));
        }
    }
}
