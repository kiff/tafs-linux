//! The device key: P-256, signing with no prompt (KIFF's desktop client contract §2, §4).
//!
//! On Linux there is no platform key store every desktop has, so the key is made in software
//! and its private half is kept sealed in the person's keyring (the app's `store`). The start
//! says so (`"key_store": "software"`), and KIFF makes the session view-only: it never approves
//! with this key. Every approval goes to the phone until a passkey approves for it.

use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use p256::ecdsa::signature::Signer;
use p256::ecdsa::{DerSignature, SigningKey};
use p256::pkcs8::{DecodePrivateKey, EncodePrivateKey, EncodePublicKey};
use sha2::{Digest, Sha256};

pub struct DeviceKey {
    key: SigningKey,
}

impl std::fmt::Debug for DeviceKey {
    // The private half is never printed.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DeviceKey({})", self.fingerprint())
    }
}

impl DeviceKey {
    /// A new key, from the system's randomness.
    pub fn generate() -> DeviceKey {
        DeviceKey {
            key: SigningKey::random(&mut rand_core::OsRng),
        }
    }

    /// The key as the keyring keeps it: PKCS#8 DER.
    pub fn to_pkcs8(&self) -> Vec<u8> {
        self.key
            .to_pkcs8_der()
            .expect("a P-256 key encodes")
            .as_bytes()
            .to_vec()
    }

    pub fn from_pkcs8(der: &[u8]) -> Option<DeviceKey> {
        SigningKey::from_pkcs8_der(der)
            .ok()
            .map(|key| DeviceKey { key })
    }

    /// The public half, SPKI DER.
    pub fn public_spki_der(&self) -> Vec<u8> {
        self.key
            .verifying_key()
            .to_public_key_der()
            .expect("a P-256 key encodes")
            .as_bytes()
            .to_vec()
    }

    /// The public half as KIFF takes it: standard base64, with padding, of SPKI DER.
    pub fn public_spki_b64(&self) -> String {
        STANDARD.encode(self.public_spki_der())
    }

    /// The key's fingerprint as KIFF shows it (`KIFFClient.fingerprint`): the first 8 bytes of
    /// SHA-256 of its SPKI DER, in hex, in groups of four.
    pub fn fingerprint(&self) -> String {
        fingerprint(&self.public_spki_der())
    }

    /// Signs a message: ECDSA P-256 over SHA-256 of its bytes, ASN.1 DER, standard base64 (§2.1).
    pub fn sign_b64(&self, message: &[u8]) -> String {
        let sig: DerSignature = self.key.sign(message);
        STANDARD.encode(sig.as_bytes())
    }
}

/// `KIFFClient.fingerprint`.
pub fn fingerprint(spki_der: &[u8]) -> String {
    let hex: String = Sha256::digest(spki_der)
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect();
    hex.as_bytes()
        .chunks(4)
        .map(|c| std::str::from_utf8(c).unwrap())
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::VerifyingKey;
    use p256::pkcs8::DecodePublicKey;

    #[test]
    fn spki_is_p256_and_signatures_are_der_kiff_verifies() {
        let key = DeviceKey::generate();
        let der = key.public_spki_der();
        assert!(der.len() <= 512);
        let public = VerifyingKey::from_public_key_der(&der).unwrap();
        let sig = STANDARD
            .decode(key.sign_b64(b"kiff.request.v1\nt\nPOST\n/x\ne3b0\n2026"))
            .unwrap();
        assert_eq!(sig[0], 0x30, "DER, never raw r||s");
        let sig = p256::ecdsa::DerSignature::try_from(sig.as_slice()).unwrap();
        public
            .verify(b"kiff.request.v1\nt\nPOST\n/x\ne3b0\n2026", &sig)
            .unwrap();
    }

    #[test]
    fn it_comes_back_from_the_keyring_as_the_same_key() {
        let key = DeviceKey::generate();
        let back = DeviceKey::from_pkcs8(&key.to_pkcs8()).unwrap();
        assert_eq!(back.public_spki_der(), key.public_spki_der());
        assert!(DeviceKey::from_pkcs8(b"not a key").is_none());
    }

    #[test]
    fn fingerprints_are_four_groups_of_four() {
        let f = DeviceKey::generate().fingerprint();
        assert_eq!(f.len(), 19);
        assert_eq!(f.split(' ').count(), 4);
        assert!(!format!("{:?}", DeviceKey::generate()).contains("SigningKey"));
    }
}
