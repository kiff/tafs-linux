//! Adding this computer with a phone (KIFF's desktop-client contract §5, as tafs-macos does it in
//! `Messages.swift`, `Signing.swift` and `Client.swift`): this computer shows a QR code with its
//! device key and name, and a 6-character check code; a phone that already approves scans it,
//! shows the same check code, and signs this computer's key; then this computer claims its
//! session with a proof that it holds that key.
//!
//! Linux's device key is in software, so the code says `"key_store": "software"` and carries
//! no approval key (`kind: passkey`): KIFF never lets this key approve. Approvals go to the
//! phone, or to a passkey.

use std::collections::BTreeMap;
use std::time::Duration;

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, SecondsFormat, Utc};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::keys::DeviceKey;
use crate::session::{ApprovalMode, KeptSession};
use crate::signin::{refusal, Cancelled, Clock, DeviceToken, KiffError, Stopped, Transport};
use crate::words::WordBook;

/// The payload's version (`PairingPayload.v`).
pub const VERSION: &str = "kiff.pair.v1";
/// The platform the code names.
pub const PLATFORM: &str = "linux";
/// How long a code is shown before a new one takes its place (`PairingPayload.lifetime`).
pub const LIFETIME: Duration = Duration::from_secs(5 * 60);
/// The wait between two claims (`Waiting.until(interval: 3, …)`).
pub const INTERVAL: Duration = Duration::from_secs(3);
/// What `slow_down` adds to the wait, for the rest of it.
pub const SLOW_DOWN: Duration = Duration::from_secs(5);
/// The longest a pairing is waited for, whatever the code says (`Waiting.longest`).
pub const LONGEST: Duration = Duration::from_secs(10 * 60);

/// Base64url without padding, as the QR code and the claim write a key (`Encoding.base64URL`).
pub fn base64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Lowercase hex (`Encoding.hex`).
pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Base32 with RFC 4648's alphabet and no padding (`Encoding.base32`).
pub fn base32(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
    let mut out = String::new();
    let mut buffer: u32 = 0;
    let mut bits = 0;
    for &byte in bytes {
        buffer = (buffer << 8) | u32::from(byte);
        bits += 8;
        while bits >= 5 {
            out.push(ALPHABET[((buffer >> (bits - 5)) & 31) as usize] as char);
            bits -= 5;
        }
        buffer &= (1 << bits) - 1;
    }
    if bits > 0 {
        out.push(ALPHABET[((buffer << (5 - bits)) & 31) as usize] as char);
    }
    out
}

/// A time as KIFF reads it: RFC 3339, in UTC, to the second (`KIFFMessage.time`).
pub fn time(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::Secs, true)
}

/// A pairing's nonce: base64url of 32 random bytes from the system's generator (`Signing.nonce`).
pub fn nonce() -> String {
    let mut bytes = [0u8; 32];
    rand_core::RngCore::fill_bytes(&mut rand_core::OsRng, &mut bytes);
    base64url(&bytes)
}

/// The 6 characters this computer and the phone both show before the phone approves
/// (`Signing.checkCode`): base32 of SHA-256 of the device key's SPKI DER, with no approval key
/// after it since Linux carries none.
pub fn check_code(device_key_spki_der: &[u8]) -> String {
    base32(&Sha256::digest(device_key_spki_der))
        .chars()
        .take(6)
        .collect()
}

/// What the device key signs to claim the pairing (`KIFFMessage.pairClaim`).
pub fn claim_message(nonce: &str, device_key_spki_der: &[u8]) -> String {
    [
        "kiff.pair.claim.v1",
        nonce,
        &hex(&Sha256::digest(device_key_spki_der)),
    ]
    .join("\n")
}

/// The claim's proof (`Signing.claimProof`): the device key's signature over `claim_message`,
/// DER in standard base64.
pub fn claim_proof(key: &DeviceKey, nonce: &str) -> String {
    key.sign_b64(claim_message(nonce, &key.public_spki_der()).as_bytes())
}

/// What the QR code holds (`PairingPayload`, `kiff.pair.v1`): this computer's public key and
/// name. No secret is in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payload {
    /// The device key: base64url of its SPKI DER.
    pub pub_key: String,
    pub name: String,
    /// Base64url of 32 random bytes. This computer keeps it to claim the pairing.
    pub nonce: String,
    /// When the code stops working, `LIFETIME` ahead.
    pub exp: String,
}

impl Payload {
    /// The payload for this computer's software key (`PairingPayload(deviceKey:keyStore:…)` with
    /// `keyStore: "software"`): no approval key, `kind: passkey`.
    pub fn new(key: &DeviceKey, name: &str, nonce: &str, now: DateTime<Utc>) -> Payload {
        let lifetime = chrono::Duration::from_std(LIFETIME).expect("5 minutes");
        Payload {
            pub_key: base64url(&key.public_spki_der()),
            name: name.to_string(),
            nonce: nonce.to_string(),
            exp: time(now + lifetime),
        }
    }

    /// The QR code's text (`PairingPayload.text`): JSON, keys sorted, slashes as they are.
    pub fn text(&self) -> String {
        let fields: BTreeMap<&str, &str> = BTreeMap::from([
            ("exp", self.exp.as_str()),
            ("key_store", "software"),
            ("kind", "passkey"),
            ("name", self.name.as_str()),
            ("nonce", self.nonce.as_str()),
            ("platform", PLATFORM),
            ("pub", self.pub_key.as_str()),
            ("v", VERSION),
        ]);
        // serde_json never escapes a slash, and a BTreeMap writes its keys sorted.
        serde_json::to_string(&fields).expect("strings encode")
    }
}

/// The QR code's modules, row by row from the top left, `true` dark; no quiet zone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QrModules {
    pub width: usize,
    pub dark: Vec<bool>,
}

impl QrModules {
    pub fn is_dark(&self, x: usize, y: usize) -> bool {
        self.dark[y * self.width + x]
    }
}

/// The QR code (`QRCode.image`): the pairing text as bytes (byte mode, whatever the characters),
/// error correction M, in the smallest version that holds it.
pub fn qr(text: &str) -> Option<QrModules> {
    use qrcode::bits::Bits;
    use qrcode::{EcLevel, QrCode, Version};
    (1..=40).find_map(|v| {
        let mut bits = Bits::new(Version::Normal(v));
        bits.push_byte_data(text.as_bytes()).ok()?;
        bits.push_terminator(EcLevel::M).ok()?;
        let code = QrCode::with_bits(bits, EcLevel::M).ok()?;
        Some(QrModules {
            width: code.width(),
            dark: code
                .to_colors()
                .into_iter()
                .map(|c| c == qrcode::Color::Dark)
                .collect(),
        })
    })
}

/// The claim's body (`KIFFClient.claimPairing`): the nonce, the key as the code wrote it, and
/// the proof.
pub fn claim_body(nonce: &str, pub_key: &str, proof: &str) -> Value {
    json!({ "nonce": nonce, "pub": pub_key, "proof": proof })
}

/// The claim's body from the welcome, after its tick: it also carries the terms version the tick
/// agreed to, so KIFF records the consent for this computer with the session (kiff/cloud#24). KIFF
/// checks it before anything is claimed: 409 `terms_outdated` (`is_terms_outdated`).
pub fn claim_body_with_terms(nonce: &str, pub_key: &str, proof: &str, terms: &str) -> Value {
    json!({ "nonce": nonce, "pub": pub_key, "proof": proof, "terms_version": terms })
}

/// Whether KIFF refused a claim because the terms changed since the tick.
pub fn is_terms_outdated(e: &KiffError) -> bool {
    matches!(e, KiffError::Server { status: 409, message } if message == "terms_outdated")
}

/// One claim (`POST /v1/pair/claim`): the session, or why not yet. KIFF's answers map as
/// `KIFFError(status:body:)`: 400 `authorization_pending`, `slow_down`, `expired_token`, and
/// 403 `invalid_proof`; anything else is KIFF's refusal as it said it.
pub fn claim(
    t: &dyn Transport,
    body: &Value,
    w: Option<&WordBook>,
) -> Result<DeviceToken, KiffError> {
    let bytes = serde_json::to_vec(body).map_err(|_| KiffError::Unreadable)?;
    let answer = t.post("/v1/pair/claim", &bytes)?;
    if (200..300).contains(&answer.status) {
        return serde_json::from_slice(&answer.body).map_err(|_| KiffError::Unreadable);
    }
    match refusal(&answer, w) {
        KiffError::Server {
            status: 400,
            message,
        } => match message.as_str() {
            "authorization_pending" => Err(KiffError::AuthorizationPending),
            "slow_down" => Err(KiffError::SlowDown),
            "expired_token" => Err(KiffError::Expired),
            _ => Err(KiffError::Server {
                status: 400,
                message,
            }),
        },
        other => Err(other),
    }
}

/// The session a pairing keeps, bound to the key the phone approved. It is kept **view-only**
/// whatever the claim's answer says: this computer's key is in software (the code said
/// `key_store: software`), and KIFF lets a software key approve nothing. KIFF answers `view` for
/// a software key; this keeps it so whatever the answer says. The view-only notice is then drawn from KIFF's `/v1/me`, as
/// for any session.
pub fn kept(token: &DeviceToken, key: &DeviceKey, now: i64) -> KeptSession {
    KeptSession {
        approval_mode: ApprovalMode::View,
        paired: true,
        ..KeptSession::from_token(token, key, now)
    }
}

/// `Waiting.until` for a pairing: one interval before the first claim and between claims; "not
/// yet" claims again; "slow down" adds 5 seconds to the interval for good; past the code's
/// lifetime it ends as expired without claiming again; any other answer ends the wait.
pub fn wait(
    t: &dyn Transport,
    clock: &dyn Clock,
    body: &Value,
    w: Option<&WordBook>,
) -> Result<DeviceToken, Stopped> {
    let mut interval = INTERVAL;
    let mut waited = Duration::ZERO;
    let limit = LIFETIME.min(LONGEST);
    loop {
        if waited + interval > limit {
            return Err(Stopped::Failed(KiffError::Expired));
        }
        clock
            .wait(interval)
            .map_err(|Cancelled| Stopped::Cancelled)?;
        waited += interval;
        match claim(t, body, w) {
            Ok(token) => return Ok(token),
            Err(KiffError::AuthorizationPending) => continue,
            Err(KiffError::SlowDown) => interval += SLOW_DOWN,
            Err(e) => return Err(Stopped::Failed(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signin::Answer;
    use chrono::TimeZone;
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::{DerSignature, VerifyingKey};
    use p256::pkcs8::DecodePublicKey;
    use std::cell::{Cell, RefCell};

    struct Script {
        answers: RefCell<Vec<Result<Answer, KiffError>>>,
        sent: RefCell<Vec<(String, Value)>>,
    }

    impl Script {
        fn new(answers: Vec<Result<Answer, KiffError>>) -> Script {
            Script {
                answers: RefCell::new(answers),
                sent: RefCell::new(vec![]),
            }
        }
    }

    impl Transport for Script {
        fn post(&self, path: &str, body: &[u8]) -> Result<Answer, KiffError> {
            self.sent
                .borrow_mut()
                .push((path.into(), serde_json::from_slice(body).unwrap()));
            self.answers.borrow_mut().remove(0)
        }
    }

    struct Ticks {
        waits: RefCell<Vec<Duration>>,
        cancel_after: Cell<Option<usize>>,
    }

    impl Ticks {
        fn new() -> Ticks {
            Ticks {
                waits: RefCell::new(vec![]),
                cancel_after: Cell::new(None),
            }
        }
    }

    impl Clock for Ticks {
        fn elapsed(&self) -> Duration {
            self.waits.borrow().iter().sum()
        }
        fn wait(&self, d: Duration) -> Result<(), Cancelled> {
            if self.cancel_after.get() == Some(self.waits.borrow().len()) {
                return Err(Cancelled);
            }
            self.waits.borrow_mut().push(d);
            Ok(())
        }
    }

    fn refused(status: u16, error: &str) -> Result<Answer, KiffError> {
        Ok(Answer {
            status,
            body: format!(r#"{{"error":"{error}"}}"#).into_bytes(),
        })
    }

    fn session() -> Result<Answer, KiffError> {
        Ok(Answer {
            status: 200,
            body: br#"{"access_token":"tok","expires_in":7776000,"tenant":"t_1","subject":"u_1","device_id":"dev_9"}"#.to_vec(),
        })
    }

    #[test]
    fn base32_is_rfc_4648_with_no_padding() {
        // RFC 4648 §10's vectors, padding dropped.
        assert_eq!(base32(b""), "");
        assert_eq!(base32(b"f"), "MY");
        assert_eq!(base32(b"fo"), "MZXQ");
        assert_eq!(base32(b"foo"), "MZXW6");
        assert_eq!(base32(b"foob"), "MZXW6YQ");
        assert_eq!(base32(b"fooba"), "MZXW6YTB");
        assert_eq!(base32(b"foobar"), "MZXW6YTBOI");
    }

    #[test]
    fn the_check_code_is_six_characters_of_the_keys_hash() {
        let key = DeviceKey::generate();
        let der = key.public_spki_der();
        let code = check_code(&der);
        assert_eq!(code.len(), 6);
        assert_eq!(code, base32(&Sha256::digest(&der))[..6]);
        assert!(code
            .chars()
            .all(|c| c.is_ascii_uppercase() || ('2'..='7').contains(&c)));
        // Another key, another code.
        assert_ne!(check_code(&DeviceKey::generate().public_spki_der()), code);
    }

    #[test]
    fn the_code_carries_the_software_key_and_no_approval_key() {
        let key = DeviceKey::generate();
        let at = Utc.with_ymd_and_hms(2026, 11, 15, 17, 40, 5).unwrap();
        let payload = Payload::new(&key, "tafs on this Linux computer", "bm9uY2U", at);
        let text = payload.text();
        let pub_key = base64url(&key.public_spki_der());
        assert_eq!(
            text,
            format!(
                r#"{{"exp":"2026-11-15T17:45:05Z","key_store":"software","kind":"passkey","name":"tafs on this Linux computer","nonce":"bm9uY2U","platform":"linux","pub":"{pub_key}","v":"kiff.pair.v1"}}"#
            )
        );
        assert!(!text.contains("approval_pub"));
        assert!(!pub_key.contains(['=', '+', '/']));
        assert_eq!(
            URL_SAFE_NO_PAD.decode(&payload.pub_key).unwrap(),
            key.public_spki_der()
        );
        // Slashes stay as they are.
        let slashed = Payload {
            name: "a/b".into(),
            ..payload
        };
        assert!(slashed.text().contains(r#""name":"a/b""#));
    }

    #[test]
    fn the_qr_code_reads_back_as_a_phones_camera_would() {
        let key = DeviceKey::generate();
        let text = Payload::new(&key, "tafs on this Linux computer", &nonce(), Utc::now()).text();
        let modules = qr(&text).unwrap();
        // Drawn as the screen draws it: 4 pixels a module, a quiet zone of 4 modules.
        let (scale, quiet) = (4, 4);
        let side = (modules.width + 2 * quiet) * scale;
        let mut image = rqrr::PreparedImage::prepare_from_greyscale(side, side, |x, y| {
            let (mx, my) = (x / scale, y / scale);
            let inside = (quiet..quiet + modules.width).contains(&mx)
                && (quiet..quiet + modules.width).contains(&my);
            if inside && modules.is_dark(mx - quiet, my - quiet) {
                0
            } else {
                255
            }
        });
        let grids = image.detect_grids();
        assert_eq!(grids.len(), 1);
        let (meta, read) = grids[0].decode().unwrap();
        assert_eq!(read, text);
        // Error correction M (rqrr numbers M as 0, as the format bits write it).
        assert_eq!(meta.ecc_level, 0);
    }

    #[test]
    fn a_nonce_is_32_random_bytes() {
        let a = nonce();
        assert_eq!(URL_SAFE_NO_PAD.decode(&a).unwrap().len(), 32);
        assert_eq!(a.len(), 43);
        assert_ne!(a, nonce());
    }

    #[test]
    fn the_proof_is_the_keys_signature_over_the_nonce_and_its_hash() {
        let key = DeviceKey::generate();
        let der = key.public_spki_der();
        let message = claim_message("n0nce", &der);
        let hash = hex(&Sha256::digest(&der));
        assert_eq!(message, format!("kiff.pair.claim.v1\nn0nce\n{hash}"));
        assert_eq!(hash.len(), 64);
        let proof = claim_proof(&key, "n0nce");
        let sig = DerSignature::try_from(
            base64::engine::general_purpose::STANDARD
                .decode(&proof)
                .unwrap()
                .as_slice(),
        )
        .unwrap();
        let public = VerifyingKey::from_public_key_der(&der).unwrap();
        assert!(public.verify(message.as_bytes(), &sig).is_ok());
        assert!(public.verify(b"another message", &sig).is_err());
    }

    #[test]
    fn a_claim_sends_the_nonce_the_key_and_the_proof() {
        let t = Script::new(vec![session()]);
        let token = claim(&t, &claim_body("n", "PUB", "PROOF"), None).unwrap();
        assert_eq!(token.access_token, "tok");
        assert_eq!(token.tenant.as_deref(), Some("t_1"));
        assert_eq!(token.subject.as_deref(), Some("u_1"));
        assert_eq!(token.expires_in, 7776000);
        // KIFF's word, as it said it: none here.
        assert_eq!(token.approval_mode, None);
        let sent = t.sent.borrow();
        assert_eq!(sent[0].0, "/v1/pair/claim");
        assert_eq!(
            sent[0].1,
            json!({ "nonce": "n", "pub": "PUB", "proof": "PROOF" })
        );
    }

    #[test]
    fn a_paired_software_key_keeps_the_view_only_notice() {
        let key = DeviceKey::generate();
        for mode in [None, Some("view"), Some("passkey"), Some("key")] {
            let body = match mode {
                Some(m) => {
                    format!(r#"{{"access_token":"tok","expires_in":60,"approval_mode":"{m}"}}"#)
                }
                None => r#"{"access_token":"tok","expires_in":60}"#.to_string(),
            };
            let t = Script::new(vec![Ok(Answer {
                status: 200,
                body: body.into_bytes(),
            })]);
            let token = claim(&t, &claim_body("n", "p", "q"), None).unwrap();
            assert_eq!(token.approval_mode.as_deref(), mode);
            let session = kept(&token, &key, 1_000);
            assert_eq!(session.approval_mode, ApprovalMode::View, "{mode:?}");
            assert!(session.approval_mode.is_view_only());
            // Paired: the notice says approvals go to the phone, with nothing to add.
            assert!(session.paired);
            assert_eq!(session.token, "tok");
            assert_eq!(session.expires_at, 1_060);
            assert_eq!(
                session.key().unwrap().public_spki_der(),
                key.public_spki_der()
            );
        }
    }

    #[test]
    fn every_answer_the_contract_names() {
        let body = claim_body("n", "p", "q");
        let one = |answer| claim(&Script::new(vec![answer]), &body, None);
        assert_eq!(
            one(refused(400, "authorization_pending")),
            Err(KiffError::AuthorizationPending)
        );
        assert_eq!(one(refused(400, "slow_down")), Err(KiffError::SlowDown));
        assert_eq!(one(refused(400, "expired_token")), Err(KiffError::Expired));
        assert_eq!(
            one(refused(403, "invalid_proof")),
            Err(KiffError::Server {
                status: 403,
                message: "invalid_proof".into()
            })
        );
        assert_eq!(
            one(refused(400, "invalid_request")),
            Err(KiffError::Server {
                status: 400,
                message: "invalid_request".into()
            })
        );
        assert_eq!(
            one(Ok(Answer {
                status: 200,
                body: b"{}".to_vec()
            })),
            Err(KiffError::Unreadable)
        );
        assert_eq!(one(Err(KiffError::Offline)), Err(KiffError::Offline));
    }

    #[test]
    fn the_wait_paces_every_claim() {
        let t = Script::new(vec![
            refused(400, "authorization_pending"),
            refused(400, "slow_down"),
            refused(400, "authorization_pending"),
            session(),
        ]);
        let clock = Ticks::new();
        let token = wait(&t, &clock, &claim_body("n", "p", "q"), None).unwrap();
        assert_eq!(token.access_token, "tok");
        let s = Duration::from_secs;
        // One interval before the first claim; slow down adds 5 s for the rest.
        assert_eq!(*clock.waits.borrow(), vec![s(3), s(3), s(8), s(8)]);
        assert_eq!(t.sent.borrow().len(), 4);
    }

    #[test]
    fn the_wait_ends_with_the_code() {
        let pending: Vec<_> = (0..200)
            .map(|_| refused(400, "authorization_pending"))
            .collect();
        let t = Script::new(pending);
        let clock = Ticks::new();
        let got = wait(&t, &clock, &claim_body("n", "p", "q"), None);
        assert_eq!(got, Err(Stopped::Failed(KiffError::Expired)));
        // 100 claims of 3 s is the 5 minutes; none after.
        assert_eq!(t.sent.borrow().len(), 100);
        assert_eq!(clock.elapsed(), LIFETIME);
    }

    #[test]
    fn any_other_answer_or_a_cancel_ends_the_wait() {
        let t = Script::new(vec![
            refused(400, "authorization_pending"),
            refused(403, "invalid_proof"),
        ]);
        let got = wait(&t, &Ticks::new(), &claim_body("n", "p", "q"), None);
        assert_eq!(
            got,
            Err(Stopped::Failed(KiffError::Server {
                status: 403,
                message: "invalid_proof".into()
            }))
        );
        let t = Script::new(vec![Err(KiffError::Offline)]);
        assert_eq!(
            wait(&t, &Ticks::new(), &claim_body("n", "p", "q"), None),
            Err(Stopped::Failed(KiffError::Offline))
        );
        let t = Script::new(vec![refused(400, "authorization_pending")]);
        let clock = Ticks::new();
        clock.cancel_after.set(Some(1));
        assert_eq!(
            wait(&t, &clock, &claim_body("n", "p", "q"), None),
            Err(Stopped::Cancelled)
        );
        assert_eq!(t.sent.borrow().len(), 1);
    }

    #[test]
    fn a_claim_after_the_tick_carries_the_terms() {
        let b = claim_body_with_terms("n", "p", "pr", "2026-11-14");
        assert_eq!(b["terms_version"], "2026-11-14");
        assert_eq!(b["nonce"], "n");
        assert!(claim_body("n", "p", "pr").get("terms_version").is_none());
        assert!(is_terms_outdated(&KiffError::Server {
            status: 409,
            message: "terms_outdated".into()
        }));
        assert!(!is_terms_outdated(&KiffError::Expired));
    }
}
