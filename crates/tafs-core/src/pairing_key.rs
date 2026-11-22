//! A security key registered with this computer when it's paired (kiff/tafs
//! `docs/computer-passkeys.md`, "A security key goes with its computer"; the PM's decision (i),
//! kiff/work#17, 22 Nov): the computer makes a credential on the key before it shows the QR, the
//! QR names it (`credential: {id, public_key}`), the phone approves the pairing as any pairing,
//! and the claim proves the key made that credential for this pairing and no other
//! (`credential_proof`, KIFF's `VerifyPairingCredential`). The device key stays in software and
//! grants nothing; only a fresh assertion from the key approves.
//!
//! Switched off with the rest of `passkey` until KIFF takes a software-key computer's credential.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::passkey::{challenge, AuthenticatorData, ORIGIN, RP_ID};

/// What the creation signs: `kiff.passkey.pair.v1`, the QR's nonce and its `pub`, one per line
/// (KIFF's `PairingCredentialMessage`), so the credential is made for this pairing and no other.
pub fn credential_message(nonce: &str, pub_key: &str) -> Vec<u8> {
    format!("kiff.passkey.pair.v1\n{nonce}\n{pub_key}").into_bytes()
}

/// The creation's client data, these exact bytes, in this order (`webauthn.create`).
pub fn create_client_data_json(message: &[u8]) -> Vec<u8> {
    format!(
        r#"{{"type":"webauthn.create","challenge":"{}","origin":"{ORIGIN}","crossOrigin":false}}"#,
        challenge(message)
    )
    .into_bytes()
}

/// The credential a key made: its id and its public key as SPKI DER.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credential {
    pub id: Vec<u8>,
    pub public_key: Vec<u8>,
}

impl Credential {
    /// The QR's `credential`: both base64url without padding.
    pub fn qr_field(&self) -> Value {
        json!({
            "id": URL_SAFE_NO_PAD.encode(&self.id),
            "public_key": URL_SAFE_NO_PAD.encode(&self.public_key),
        })
    }
}

/// Why a creation is never sent: KIFF would refuse it (`verifyCreation`'s checks, in order).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refused {
    /// The authenticator data or its credential can't be read.
    Unreadable,
    /// A relying-party hash that isn't `sha256("kiff.dev")`.
    NotKiff,
    /// No user presence, or no user verification (the PIN).
    NotVerified,
    /// No attested credential in it.
    NoCredential,
    /// The key isn't an ES256 P-256 key on the curve.
    NotEs256,
    /// Backup eligible or backed up: a synced passkey isn't this computer's own.
    CanLeave,
}

/// The credential in a creation's authenticator data, checked as KIFF checks it.
pub fn read_created(authenticator_data: &[u8]) -> Result<Credential, Refused> {
    let head = AuthenticatorData::parse(authenticator_data).ok_or(Refused::Unreadable)?;
    if head.rp_id_hash[..] != Sha256::digest(RP_ID.as_bytes())[..] {
        return Err(Refused::NotKiff);
    }
    if !head.user_present() || !head.user_verified() {
        return Err(Refused::NotVerified);
    }
    if head.flags & 0x40 == 0 {
        return Err(Refused::NoCredential);
    }
    if head.backup_eligible() || head.backed_up() {
        return Err(Refused::CanLeave);
    }
    // The head (37), then the AAGUID (16), the id's length (2), the id, its COSE key.
    let rest = authenticator_data.get(37..).ok_or(Refused::Unreadable)?;
    let len_bytes = rest.get(16..18).ok_or(Refused::Unreadable)?;
    let id_len = u16::from_be_bytes([len_bytes[0], len_bytes[1]]) as usize;
    if id_len == 0 || id_len > 1023 {
        return Err(Refused::Unreadable);
    }
    let id = rest
        .get(18..18 + id_len)
        .ok_or(Refused::Unreadable)?
        .to_vec();
    let cose = rest.get(18 + id_len..).ok_or(Refused::Unreadable)?;
    let (key, used) = cose_es256(cose)?;
    // Anything after the key is extensions, only when the flag says so.
    if head.flags & 0x80 == 0 && used != cose.len() {
        return Err(Refused::Unreadable);
    }
    Ok(Credential {
        id,
        public_key: key,
    })
}

/// A COSE EC2 key that is exactly ES256 on P-256 (kty 2, alg -7, crv 1, x, y), as SPKI DER; with
/// the bytes it took.
fn cose_es256(data: &[u8]) -> Result<(Vec<u8>, usize), Refused> {
    let mut r = Cbor { data, at: 0 };
    let n = r.map_len().ok_or(Refused::NotEs256)?;
    if n != 5 {
        return Err(Refused::NotEs256);
    }
    let (mut kty, mut alg, mut crv) = (None, None, None);
    let (mut x, mut y): (Option<Vec<u8>>, Option<Vec<u8>>) = (None, None);
    for _ in 0..n {
        let k = r.int().ok_or(Refused::NotEs256)?;
        match k {
            1 => kty = r.int(),
            3 => alg = r.int(),
            -1 => crv = r.int(),
            -2 => x = r.bytes(),
            -3 => y = r.bytes(),
            _ => return Err(Refused::NotEs256),
        }
    }
    let (Some(2), Some(-7), Some(1), Some(x), Some(y)) = (kty, alg, crv, x, y) else {
        return Err(Refused::NotEs256);
    };
    if x.len() != 32 || y.len() != 32 {
        return Err(Refused::NotEs256);
    }
    let mut point = vec![4u8];
    point.extend_from_slice(&x);
    point.extend_from_slice(&y);
    use p256::pkcs8::EncodePublicKey;
    let key = p256::PublicKey::from_sec1_bytes(&point).map_err(|_| Refused::NotEs256)?;
    let der = key.to_public_key_der().map_err(|_| Refused::NotEs256)?;
    Ok((der.as_bytes().to_vec(), r.at))
}

/// Just enough CBOR for a COSE key: a map's length, integers and byte strings.
struct Cbor<'a> {
    data: &'a [u8],
    at: usize,
}

impl Cbor<'_> {
    fn head(&mut self) -> Option<(u8, u64)> {
        let b = *self.data.get(self.at)?;
        self.at += 1;
        let (major, info) = (b >> 5, b & 0x1f);
        let n = match info {
            0..=23 => info as u64,
            24 => self.take(1)?[0] as u64,
            25 => u16::from_be_bytes(self.take(2)?.try_into().ok()?) as u64,
            26 => u32::from_be_bytes(self.take(4)?.try_into().ok()?) as u64,
            _ => return None,
        };
        Some((major, n))
    }
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.data.get(self.at..self.at + n)?;
        self.at += n;
        Some(s)
    }
    fn map_len(&mut self) -> Option<u64> {
        match self.head()? {
            (5, n) => Some(n),
            _ => None,
        }
    }
    fn int(&mut self) -> Option<i64> {
        match self.head()? {
            (0, n) => i64::try_from(n).ok(),
            (1, n) => Some(-1 - i64::try_from(n).ok()?),
            _ => None,
        }
    }
    fn bytes(&mut self) -> Option<Vec<u8>> {
        match self.head()? {
            (2, n) => Some(self.take(n as usize)?.to_vec()),
            _ => None,
        }
    }
}

/// The claim's `credential_proof`: the creation's client data and an attestation object of
/// format `none` over its authenticator data, both base64url without padding. KIFF takes `none`
/// only: the key's own attestation is never sent.
pub fn credential_proof(client_data_json: &[u8], authenticator_data: &[u8]) -> Value {
    let object = crate::security_key::attestation_object("none", &[0xa0], authenticator_data);
    json!({
        "client_data_json": URL_SAFE_NO_PAD.encode(client_data_json),
        "attestation_object": URL_SAFE_NO_PAD.encode(object),
    })
}

/// The check code with the key's credential: KIFF's `PairingCode(pub, credential key)`, the first
/// six base32 characters of SHA-256 over both, so the code the phone shows vouches for the key.
pub fn check_code(device_key_spki_der: &[u8], credential_key_spki_der: &[u8]) -> String {
    let mut both = device_key_spki_der.to_vec();
    both.extend_from_slice(credential_key_spki_der);
    crate::pairing::check_code(&both)
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::SigningKey;
    use p256::pkcs8::EncodePublicKey;

    /// A creation's authenticator data as a key would write it.
    fn created(flags: u8, id: &[u8], key: &SigningKey, rp: &str) -> Vec<u8> {
        let point = key.verifying_key().to_encoded_point(false);
        let (x, y) = (point.x().unwrap(), point.y().unwrap());
        let mut d = Sha256::digest(rp.as_bytes()).to_vec();
        d.push(flags);
        d.extend_from_slice(&7u32.to_be_bytes());
        d.extend_from_slice(&[0u8; 16]);
        d.extend_from_slice(&(id.len() as u16).to_be_bytes());
        d.extend_from_slice(id);
        // {1: 2, 3: -7, -1: 1, -2: x, -3: y}
        d.extend_from_slice(&[0xa5, 0x01, 0x02, 0x03, 0x26, 0x20, 0x01, 0x21, 0x58, 0x20]);
        d.extend_from_slice(x);
        d.extend_from_slice(&[0x22, 0x58, 0x20]);
        d.extend_from_slice(y);
        d
    }

    fn key() -> SigningKey {
        SigningKey::from_slice(&[7u8; 32]).unwrap()
    }

    #[test]
    fn the_credential_is_read_as_kiff_reads_it() {
        let k = key();
        let c = read_created(&created(0x45, b"cred-1", &k, "kiff.dev")).unwrap();
        assert_eq!(c.id, b"cred-1");
        assert_eq!(
            c.public_key,
            k.verifying_key().to_public_key_der().unwrap().as_bytes()
        );
        assert_eq!(c.qr_field()["id"], "Y3JlZC0x");
    }

    #[test]
    fn what_kiff_would_refuse_is_never_sent() {
        let k = key();
        let read = |flags, rp: &str| read_created(&created(flags, b"c", &k, rp));
        assert_eq!(read(0x45, "evil.dev"), Err(Refused::NotKiff));
        assert_eq!(read(0x41, "kiff.dev"), Err(Refused::NotVerified));
        assert_eq!(read(0x44, "kiff.dev"), Err(Refused::NotVerified));
        assert_eq!(read(0x05, "kiff.dev"), Err(Refused::NoCredential));
        assert_eq!(read(0x4d, "kiff.dev"), Err(Refused::CanLeave));
        assert_eq!(read_created(&[0u8; 20]), Err(Refused::Unreadable));
        // Trailing bytes with no extensions flag.
        let mut extra = created(0x45, b"c", &k, "kiff.dev");
        extra.push(0);
        assert_eq!(read_created(&extra), Err(Refused::Unreadable));
    }

    #[test]
    fn the_creation_is_for_this_pairing() {
        let m = credential_message("NONCE", "PUB");
        assert_eq!(m, b"kiff.passkey.pair.v1\nNONCE\nPUB");
        let cd = String::from_utf8(create_client_data_json(&m)).unwrap();
        assert_eq!(
            cd,
            format!(
                r#"{{"type":"webauthn.create","challenge":"{}","origin":"https://kiff.dev","crossOrigin":false}}"#,
                challenge(&m)
            )
        );
        let proof = credential_proof(b"cd", b"ad");
        let object = URL_SAFE_NO_PAD
            .decode(proof["attestation_object"].as_str().unwrap())
            .unwrap();
        // {"fmt": "none", "attStmt": {}, "authData": h'6164'}
        assert_eq!(&object[..1], &[0xa3]);
        assert!(object.windows(4).any(|w| w == b"none"));
        assert_eq!(proof["client_data_json"], "Y2Q");
    }

    #[test]
    fn the_check_code_vouches_for_both_keys() {
        let a = check_code(b"pub", b"cred");
        assert_eq!(a.len(), 6);
        assert_ne!(a, crate::pairing::check_code(b"pub"));
        assert_eq!(a, crate::pairing::check_code(b"pubcred"));
    }
}
