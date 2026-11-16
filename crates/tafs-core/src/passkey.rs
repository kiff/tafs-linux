//! Approving with a passkey (kiff/tafs `docs/computer-passkeys.md`, "What a passkey approval
//! carries"): the message a device key would sign, the WebAuthn challenge and client data built
//! here, the checks that refuse before sending what KIFF would refuse, and the
//! `KIFF-Passkey-Assertion` header. Linux approves with a USB security key (CTAP2), which comes
//! next; until it does, this is switched off (`ENABLED`) and nothing asks a key for anything.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use sha2::{Digest, Sha256};

/// Off until the security key and its pairing are built: nothing signs with a passkey.
pub const ENABLED: bool = false;

pub const RP_ID: &str = "kiff.dev";
pub const ORIGIN: &str = "https://kiff.dev";
pub const HEADER: &str = "KIFF-Passkey-Assertion";

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `kiff.request.v1` (`KIFFClient.requestMessage`): the tenant, the method, the path with its
/// query, the body's SHA-256 in lowercase hex, and `signed_at`, one per line.
pub fn request_v1(tenant: &str, method: &str, uri: &str, body: &[u8], signed_at: &str) -> Vec<u8> {
    let body_hash = hex(&Sha256::digest(body));
    [
        "kiff.request.v1",
        tenant,
        method,
        uri,
        &body_hash,
        signed_at,
    ]
    .join("\n")
    .into_bytes()
}

/// `kiff.requests.v1`: two requests signed together, by their messages' SHA-256 in hex.
pub fn requests_v1(tenant: &str, signed_at: &str, one: &[u8], other: &[u8]) -> Vec<u8> {
    let (a, b) = (hex(&Sha256::digest(one)), hex(&Sha256::digest(other)));
    ["kiff.requests.v1", tenant, signed_at, &a, &b]
        .join("\n")
        .into_bytes()
}

/// The challenge: `base64url(sha256(message))`, so the assertion approves that message alone.
pub fn challenge(message: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(message))
}

/// The client data, these exact bytes, in this order; the authenticator signs
/// `authenticatorData ‖ sha256(clientData)`.
pub fn client_data_json(message: &[u8]) -> Vec<u8> {
    format!(
        r#"{{"type":"webauthn.get","challenge":"{}","origin":"{ORIGIN}","crossOrigin":false}}"#,
        challenge(message)
    )
    .into_bytes()
}

/// authenticatorData's fixed head: the relying party's hash, the flags, the sign count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatorData {
    pub rp_id_hash: [u8; 32],
    pub flags: u8,
    pub sign_count: u32,
}

impl AuthenticatorData {
    pub fn parse(data: &[u8]) -> Option<AuthenticatorData> {
        if data.len() < 37 {
            return None;
        }
        Some(AuthenticatorData {
            rp_id_hash: data[..32].try_into().ok()?,
            flags: data[32],
            sign_count: u32::from_be_bytes(data[33..37].try_into().ok()?),
        })
    }
    pub fn user_present(&self) -> bool {
        self.flags & 0x01 != 0
    }
    pub fn user_verified(&self) -> bool {
        self.flags & 0x04 != 0
    }
    pub fn backup_eligible(&self) -> bool {
        self.flags & 0x08 != 0
    }
    pub fn backed_up(&self) -> bool {
        self.flags & 0x10 != 0
    }
}

/// Why KIFF would refuse an assertion, so it's never sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// authenticatorData too short to read.
    Unreadable,
    /// A relying-party hash that isn't `sha256("kiff.dev")`.
    NotKiff,
    /// No user presence, or no user verification (the PIN).
    NotVerified,
    /// A credential other than the one asked for.
    AnotherCredential,
    /// Backup eligible or backed up: a synced passkey isn't this computer's own.
    CanLeave,
}

/// The checks before sending (`computer-passkeys.md` step 4), in its order.
pub fn refusal(authenticator_data: &[u8], asked: &[u8], answered: &[u8]) -> Option<Refusal> {
    let Some(a) = AuthenticatorData::parse(authenticator_data) else {
        return Some(Refusal::Unreadable);
    };
    if a.rp_id_hash[..] != Sha256::digest(RP_ID.as_bytes())[..] {
        return Some(Refusal::NotKiff);
    }
    if !a.user_present() || !a.user_verified() {
        return Some(Refusal::NotVerified);
    }
    if asked != answered {
        return Some(Refusal::AnotherCredential);
    }
    if a.backup_eligible() || a.backed_up() {
        return Some(Refusal::CanLeave);
    }
    None
}

/// What an authenticator answered for one assertion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assertion {
    pub credential_id: Vec<u8>,
    pub authenticator_data: Vec<u8>,
    /// DER ECDSA.
    pub signature: Vec<u8>,
}

/// The header's value: `t=<signed_at>,a=<base64url of {credential_id, client_data_json,
/// authenticator_data, signature}>`, each field base64url with no padding; with another request
/// signed together, `,with=sha256:<its message's hash>`. Refused, never built, when KIFF would
/// refuse it.
pub fn header(
    signed_at: &str,
    asked: &[u8],
    client_data: &[u8],
    a: &Assertion,
    with: Option<&[u8]>,
) -> Result<String, Refusal> {
    if let Some(r) = refusal(&a.authenticator_data, asked, &a.credential_id) {
        return Err(r);
    }
    let b = |v: &[u8]| URL_SAFE_NO_PAD.encode(v);
    let json = format!(
        r#"{{"credential_id":"{}","client_data_json":"{}","authenticator_data":"{}","signature":"{}"}}"#,
        b(&a.credential_id),
        b(client_data),
        b(&a.authenticator_data),
        b(&a.signature)
    );
    let mut h = format!("t={signed_at},a={}", b(json.as_bytes()));
    if let Some(other) = with {
        h.push_str(&format!(",with=sha256:{}", hex(&Sha256::digest(other))));
    }
    Ok(h)
}

/// `signed_at`: RFC 3339 with milliseconds, in UTC, and never the same twice: a time at or
/// before the last one given is the last one plus a millisecond.
#[derive(Debug, Default)]
pub struct SigningTime {
    last: Option<chrono::DateTime<chrono::Utc>>,
}

impl SigningTime {
    pub fn next(&mut self, now: chrono::DateTime<chrono::Utc>) -> String {
        use chrono::{SubsecRound, TimeDelta};
        let mut t = now.trunc_subsecs(3);
        if let Some(last) = self.last {
            if t <= last {
                t = last + TimeDelta::milliseconds(1);
            }
        }
        self.last = Some(t);
        t.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::ecdsa::signature::{Signer, Verifier};
    use p256::ecdsa::{DerSignature, SigningKey, VerifyingKey};

    const UP: u8 = 0x01;
    const UV: u8 = 0x04;
    const BE: u8 = 0x08;
    const BS: u8 = 0x10;

    /// Stands in for a security key: signs authData ‖ sha256(clientData), as an authenticator.
    fn authenticate(
        key: &SigningKey,
        rp: &str,
        flags: u8,
        credential: &[u8],
        client: &[u8],
    ) -> Assertion {
        let mut auth = Sha256::digest(rp.as_bytes()).to_vec();
        auth.push(flags);
        auth.extend_from_slice(&7u32.to_be_bytes());
        let mut signed = auth.clone();
        signed.extend_from_slice(&Sha256::digest(client));
        let sig: DerSignature = key.sign(&signed);
        Assertion {
            credential_id: credential.to_vec(),
            authenticator_data: auth,
            signature: sig.as_bytes().to_vec(),
        }
    }

    #[test]
    fn the_message_is_the_docs_test_vector() {
        let body = br#"{"session":"s_1","task":"Run Chargebacks","component":"d1"}"#;
        let m = request_v1(
            "t_acme",
            "POST",
            "/v1/coach/drive",
            body,
            "2026-11-06T00:30:00Z",
        );
        assert_eq!(
            String::from_utf8(m).unwrap(),
            "kiff.request.v1\nt_acme\nPOST\n/v1/coach/drive\nfea4f2ebc2091cd2cbf48e1cc3b6285603bcc194cddea23f5737dfdc2a78414f\n2026-11-06T00:30:00Z"
        );
    }

    #[test]
    fn the_header_verifies_as_kiff_verifies_it() {
        let key = SigningKey::from_slice(&[7u8; 32]).unwrap();
        let credential = b"this-computer-security-key";
        let message = request_v1(
            "t1",
            "POST",
            "/v1/cards",
            br#"{"card":"c1"}"#,
            "2026-11-14T20:00:00.000Z",
        );
        let client = client_data_json(&message);
        let a = authenticate(&key, RP_ID, UP | UV, credential, &client);
        let h = header("2026-11-14T20:00:00.000Z", credential, &client, &a, None).unwrap();
        let encoded = h.strip_prefix("t=2026-11-14T20:00:00.000Z,a=").unwrap();
        let v: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(encoded).unwrap()).unwrap();
        let field = |k: &str| URL_SAFE_NO_PAD.decode(v[k].as_str().unwrap()).unwrap();
        assert_eq!(field("credential_id"), credential);
        let raw = field("client_data_json");
        // The exact bytes, in order.
        assert_eq!(
            String::from_utf8(raw.clone()).unwrap(),
            format!(
                r#"{{"type":"webauthn.get","challenge":"{}","origin":"https://kiff.dev","crossOrigin":false}}"#,
                URL_SAFE_NO_PAD.encode(Sha256::digest(&message))
            )
        );
        let auth = field("authenticator_data");
        assert_eq!(auth[..32], Sha256::digest(b"kiff.dev")[..]);
        let mut signed = auth.clone();
        signed.extend_from_slice(&Sha256::digest(&raw));
        let sig = DerSignature::try_from(field("signature").as_slice()).unwrap();
        VerifyingKey::from(&key).verify(&signed, &sig).unwrap();
    }

    #[test]
    fn two_requests_signed_together_name_the_other() {
        let one = request_v1("t1", "POST", "/a", b"{}", "2026-11-14T20:00:00.000Z");
        let other = request_v1("t1", "POST", "/b", b"{}", "2026-11-14T20:00:00.000Z");
        let both = requests_v1("t1", "2026-11-14T20:00:00.000Z", &one, &other);
        let lines: Vec<&str> = std::str::from_utf8(&both).unwrap().lines().collect();
        assert_eq!(
            lines[..3],
            ["kiff.requests.v1", "t1", "2026-11-14T20:00:00.000Z"]
        );
        assert_eq!(lines[3], hex(&Sha256::digest(&one)));
        let key = SigningKey::from_slice(&[9u8; 32]).unwrap();
        let client = client_data_json(&both);
        let a = authenticate(&key, RP_ID, UP | UV, b"k", &client);
        let h = header("2026-11-14T20:00:00.000Z", b"k", &client, &a, Some(&other)).unwrap();
        let with = h.rsplit_once(",with=sha256:").unwrap().1;
        assert_eq!(with, hex(&Sha256::digest(&other)));
        assert_eq!(with.len(), 64);
    }

    #[test]
    fn what_kiff_would_refuse_is_never_sent() {
        let key = SigningKey::from_slice(&[7u8; 32]).unwrap();
        let client = client_data_json(b"m");
        let send = |rp: &str, flags: u8, answered: &[u8]| {
            let a = authenticate(&key, rp, flags, answered, &client);
            header("t", b"asked", &client, &a, None)
        };
        assert!(send(RP_ID, UP | UV, b"asked").is_ok());
        assert_eq!(
            send("evil.example", UP | UV, b"asked"),
            Err(Refusal::NotKiff)
        );
        assert_eq!(send(RP_ID, UP, b"asked"), Err(Refusal::NotVerified));
        assert_eq!(send(RP_ID, UV, b"asked"), Err(Refusal::NotVerified));
        assert_eq!(
            send(RP_ID, UP | UV, b"other"),
            Err(Refusal::AnotherCredential)
        );
        assert_eq!(send(RP_ID, UP | UV | BE, b"asked"), Err(Refusal::CanLeave));
        assert_eq!(send(RP_ID, UP | UV | BS, b"asked"), Err(Refusal::CanLeave));
        assert_eq!(refusal(&[0u8; 36], b"a", b"a"), Some(Refusal::Unreadable));
    }

    #[test]
    fn the_challenge_changes_with_the_request() {
        let a = client_data_json(&request_v1(
            "t",
            "POST",
            "/x",
            b"{}",
            "2026-11-14T20:00:00.000Z",
        ));
        let b = client_data_json(&request_v1(
            "t",
            "POST",
            "/x",
            b"{}",
            "2026-11-14T20:00:00.001Z",
        ));
        let c = client_data_json(&request_v1(
            "t",
            "POST",
            "/x",
            b"{ }",
            "2026-11-14T20:00:00.000Z",
        ));
        assert_ne!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn signed_at_has_milliseconds_and_never_repeats() {
        use chrono::TimeZone;
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 11, 14, 20, 0, 0)
            .unwrap()
            + chrono::TimeDelta::microseconds(123_456);
        let mut t = SigningTime::default();
        assert_eq!(t.next(now), "2026-11-14T20:00:00.123Z");
        assert_eq!(t.next(now), "2026-11-14T20:00:00.124Z");
        assert_eq!(
            t.next(now - chrono::TimeDelta::seconds(5)),
            "2026-11-14T20:00:00.125Z"
        );
        assert_eq!(
            t.next(now + chrono::TimeDelta::seconds(1)),
            "2026-11-14T20:00:01.123Z"
        );
    }

    #[test]
    fn it_is_switched_off() {
        const { assert!(!ENABLED) };
    }
}
