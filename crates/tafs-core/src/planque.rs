//! planque, which keeps a taf's vendor sign-ins (`PlanqueClient`): the one thing this computer
//! signs for it, its read of the account's grants (`grantsReadProof`), which tafs cloud forwards
//! as the device's proof (`Planque-Device-Signature`). With it, tafs cloud marks Coach's cards
//! with what a sign-in can't be narrowed to (`wider`); this computer signs no other request.

use std::sync::Mutex;

use chrono::{DateTime, TimeZone, Utc};
use sha2::{Digest, Sha256};

use crate::keys::DeviceKey;
use crate::work::segment;

/// planque's base (`PlanqueClient.baseURL`).
pub const PLANQUE: &str = "https://planque.kiff.dev";

/// The header the proof travels in.
pub const HEADER: &str = "Planque-Device-Signature";

/// What the device signs (`PlanqueClient.message`, planque's `device.Message`): seven lines, no
/// trailing newline. The base without its trailing slashes, the tenant, the method, the path and
/// query as sent, the body's SHA-256 in lowercase hex, and the signing time.
fn message(
    base: &str,
    tenant: &str,
    method: &str,
    uri: &str,
    body: &[u8],
    signed_at: &str,
) -> Vec<u8> {
    let base = base.trim_end_matches('/');
    let hash: String = Sha256::digest(body)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    [
        "planque.request.v1",
        base,
        tenant,
        method,
        uri,
        &hash,
        signed_at,
    ]
    .join("\n")
    .into_bytes()
}

/// The last time this app signed with, in milliseconds since 1970 (`SigningTime.last`).
static LAST: Mutex<Option<i64>> = Mutex::new(None);

/// `SigningTime.next`: RFC 3339 in UTC with milliseconds, never the same twice in this app, each
/// later than the last. planque and KIFF refuse a signed request they have already seen, and the
/// time is part of what's signed: two reads in the same millisecond would look like a replay.
pub fn signing_time() -> String {
    signing_time_at(Utc::now())
}

/// `SigningTime.next(now:)`.
pub fn signing_time_at(now: DateTime<Utc>) -> String {
    let mut last = LAST.lock().unwrap_or_else(|e| e.into_inner());
    let ms = now.timestamp_millis();
    let at = match *last {
        Some(l) => ms.max(l + 1),
        None => ms,
    };
    *last = Some(at);
    Utc.timestamp_millis_opt(at)
        .single()
        .unwrap_or(now)
        .format("%Y-%m-%dT%H:%M:%S%.3fZ")
        .to_string()
}

/// The account's grants at planque, its tenant escaped as one path segment.
pub fn grants_path(tenant: &str) -> String {
    format!("/v1/accounts/{}/grants", segment(tenant))
}

/// `grantsReadProof`: this device's signed read of the account's grants, `t={at},v1={sig},d={device}`.
/// The device key signs it with no prompt (`signConfirmed`): it grants nothing on its own. `device`
/// is this device's id in KIFF, which planque asks KIFF about.
pub fn grants_read_proof(key: &DeviceKey, device: &str, tenant: &str, base: &str) -> String {
    let at = signing_time();
    let bytes = message(base, tenant, "GET", &grants_path(tenant), b"", &at);
    format!("t={at},v1={},d={device}", key.sign_b64(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// This computer is view-only: planque's one signature from it is the read of the account's
    /// grants, a safe route. Nothing here can sign an approval, a confirm or any other call, and
    /// no other file builds planque's message.
    #[test]
    fn this_computer_signs_nothing_at_planque_but_the_grants_read() {
        let root = env!("CARGO_MANIFEST_DIR");
        let source = std::fs::read_to_string(format!("{root}/src/planque.rs")).unwrap();
        let code = &source[..source.find("#[cfg(test)]").unwrap()];
        // One signature, in `grants_read_proof`, over a GET of the grants and nothing else.
        assert_eq!(code.matches("sign_b64(").count(), 1);
        assert_eq!(code.matches("message(base, tenant, ").count(), 1);
        assert!(code.contains(r#"message(base, tenant, "GET", &grants_path(tenant), b"", &at)"#));
        for route in ["approve", "confirm", "POST", "PUT", "DELETE", "w="] {
            assert!(!code.contains(&format!("\"{route}")), "{route}");
        }
        // No other file in either crate builds planque's request message.
        let mut files = vec![];
        for dir in [format!("{root}/src"), format!("{root}/../tafs-app/src")] {
            let mut stack = vec![std::path::PathBuf::from(dir)];
            while let Some(d) = stack.pop() {
                for e in std::fs::read_dir(d).unwrap().flatten() {
                    let path = e.path();
                    if path.is_dir() {
                        stack.push(path);
                    } else if path.extension().is_some_and(|x| x == "rs") {
                        files.push(path);
                    }
                }
            }
        }
        let builders: Vec<_> = files
            .iter()
            .filter(|f| {
                std::fs::read_to_string(f)
                    .unwrap()
                    .contains("planque.request")
            })
            .collect();
        assert_eq!(builders.len(), 1, "{builders:?}");
        assert!(builders[0].ends_with("planque.rs"));
        // And the one proof is over that read.
        let key = DeviceKey::generate();
        let proof = grants_read_proof(&key, "dev_1", "t_1", PLANQUE);
        assert!(proof.starts_with("t=") && proof.contains(",v1=") && proof.ends_with(",d=dev_1"));
        assert!(!proof.contains(",w="));
    }
    use base64::engine::general_purpose::STANDARD;
    use base64::Engine;
    use p256::ecdsa::signature::Verifier;
    use p256::ecdsa::{DerSignature, VerifyingKey};
    use p256::pkcs8::DecodePublicKey;

    #[test]
    fn the_message_is_plannques_seven_lines() {
        let m = message(
            "https://planque.kiff.dev//",
            "t_1",
            "GET",
            "/v1/accounts/t_1/grants",
            b"",
            "2026-11-16T10:00:00.123Z",
        );
        assert_eq!(
            String::from_utf8(m).unwrap(),
            "planque.request.v1\nhttps://planque.kiff.dev\nt_1\nGET\n/v1/accounts/t_1/grants\n\
             e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\n2026-11-16T10:00:00.123Z"
        );
        // A body is hashed, never sent in the message.
        let m = message(PLANQUE, "t", "POST", "/x", b"{}", "at");
        assert!(String::from_utf8(m)
            .unwrap()
            .contains("\n44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a\n"));
    }

    #[test]
    fn the_proof_is_the_header_planque_reads_and_it_verifies() {
        let key = DeviceKey::generate();
        let proof = grants_read_proof(&key, "dev_9", "t 1", PLANQUE);
        let parts: Vec<&str> = proof.split(',').collect();
        assert_eq!(parts.len(), 3);
        let at = parts[0].strip_prefix("t=").unwrap();
        let sig = parts[1].strip_prefix("v1=").unwrap();
        assert_eq!(parts[2], "d=dev_9");
        assert!(chrono::DateTime::parse_from_rfc3339(at).is_ok());
        assert_eq!(at.len(), "2026-11-16T10:00:00.123Z".len());
        let public = VerifyingKey::from_public_key_der(&key.public_spki_der()).unwrap();
        let der = STANDARD.decode(sig).unwrap();
        let sig = DerSignature::try_from(der.as_slice()).unwrap();
        // Over the grants' path, the tenant escaped as one segment.
        let signed = message(PLANQUE, "t 1", "GET", "/v1/accounts/t%201/grants", b"", at);
        public.verify(&signed, &sig).unwrap();
        assert!(public
            .verify(&message(PLANQUE, "t 1", "GET", "/v1/other", b"", at), &sig)
            .is_err());
    }

    #[test]
    fn signing_times_only_go_forward() {
        let now = Utc.with_ymd_and_hms(2026, 11, 16, 10, 0, 0).unwrap();
        let a = signing_time_at(now);
        let b = signing_time_at(now);
        let c = signing_time_at(now - chrono::Duration::seconds(5));
        assert!(a < b && b < c, "{a} {b} {c}");
        let d = signing_time();
        let e = signing_time();
        assert!(c < d && d < e, "{c} {d} {e}");
        assert!(e.ends_with('Z'));
    }
}
