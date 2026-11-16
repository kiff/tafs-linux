//! Checks a KIFF receipt on this computer (`KIFF/ReceiptCheck.swift`, kiff/tafs#167), as KIFF's
//! own verification does: the signed digest is keccak256(ascii(sha256hex(causal_chain))), and the
//! signer recovered from the 65-byte signature must be KIFF's.
//!
//! The chain's bytes are taken from the answer as KIFF wrote them, never re-encoded
//! (`raw_value`): `GET /v1/receipts/{trace_id}` writes the stored receipt with Go's encoder, so
//! its `causal_chain` is byte for byte what KIFF hashed. Only the chain is signed.

use k256::ecdsa::{RecoveryId, Signature, VerifyingKey};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sha3::Keccak256;

use crate::words::{WordBook, WordsError};

/// KIFF's receipt signer in production (kiff/tafs#167): one key for every account, the deployer
/// of KIFF's anchor contract and the sender of its anchor transactions. A rotation ships in a new
/// build first.
pub const KIFF_SIGNER: &str = "0x2f297affd7b62ef712a1e3158421ccb4bf2efcbe";

pub const ALGORITHM: &str = "secp256k1+keccak256";

/// Why a receipt doesn't check out, each in the shared words (`receipt.check.*`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    Unreadable,
    UnknownSignature,
    NoSteps,
    MalformedSignature,
    SignatureMismatch,
    NotKiff,
}

impl Failure {
    pub fn words(self, w: &WordBook) -> Result<String, WordsError> {
        w.line(match self {
            Failure::Unreadable => "receipt.check.unreadable",
            Failure::UnknownSignature => "receipt.check.unknown_signature",
            Failure::NoSteps => "receipt.check.no_steps",
            Failure::MalformedSignature => "receipt.check.malformed_signature",
            Failure::SignatureMismatch => "receipt.check.signature_mismatch",
            Failure::NotKiff => "receipt.check.not_kiff",
        })
    }
}

/// What checking a receipt came to (`ReceiptCheck`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Check {
    /// The signature recovers to KIFF's signer over this chain.
    Verified { signer: String },
    /// KIFF signs in its next sweep, about 30 s after the decision.
    NotSignedYet,
    /// It doesn't check out, and why.
    Failed(Failure),
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Hex with or without `0x`; none when it isn't hex.
pub fn unhex(s: &str) -> Option<Vec<u8>> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Keccak-256 (Ethereum's, not SHA3-256).
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    Keccak256::digest(data).into()
}

/// The receipt in `answer`, checked against `expected` (KIFF's signer in production).
pub fn check(answer: &[u8], expected: &str) -> Check {
    let Ok(Value::Object(receipt)) = serde_json::from_slice::<Value>(answer) else {
        return Check::Failed(Failure::Unreadable);
    };
    // A key written twice: the decoder above keeps the last copy, the chain's scan the first.
    // Refused, so the steps shown are always the steps checked.
    if repeats_a_top_level_key(answer) {
        return Check::Failed(Failure::Unreadable);
    }
    let Some(Value::Object(attestation)) = receipt.get("attestation") else {
        return Check::NotSignedYet;
    };
    if attestation.get("algorithm").and_then(Value::as_str) != Some(ALGORITHM) {
        return Check::Failed(Failure::UnknownSignature);
    }
    let Some(chain) = raw_value("causal_chain", answer) else {
        return Check::Failed(Failure::NoSteps);
    };
    let Some(signature) = attestation
        .get("signature")
        .and_then(Value::as_str)
        .and_then(unhex)
        .filter(|s| s.len() == 65)
    else {
        return Check::Failed(Failure::MalformedSignature);
    };
    let declared = attestation
        .get("wallet_address")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_lowercase();
    let chain_hash = hex(&Sha256::digest(chain));
    let digest = keccak256(chain_hash.as_bytes());
    let Some(recovered) = recover_address(&digest, &signature) else {
        return Check::Failed(Failure::SignatureMismatch);
    };
    if recovered != declared {
        return Check::Failed(Failure::SignatureMismatch);
    }
    if recovered != expected.to_lowercase() {
        return Check::Failed(Failure::NotKiff);
    }
    Check::Verified { signer: recovered }
}

/// The Ethereum-style address that signed the digest: keccak256 of the uncompressed key, its last
/// 20 bytes. None when the signature can't be parsed or recovers no key.
pub fn recover_address(digest: &[u8], signature: &[u8]) -> Option<String> {
    if digest.len() != 32 || signature.len() != 65 {
        return None;
    }
    let mut v = signature[64];
    if v >= 27 {
        v -= 27;
    }
    let mut id = RecoveryId::from_byte(v)?;
    let mut sig = Signature::from_slice(&signature[..64]).ok()?;
    // libsecp256k1 recovers from a high S as well; k256 takes only a low one. The same key
    // signs with S or with n − S, the point's y parity flipped.
    if let Some(low) = sig.normalize_s() {
        sig = low;
        id = RecoveryId::new(!id.is_y_odd(), id.is_x_reduced());
    }
    let key = VerifyingKey::recover_from_prehash(digest, &sig, id).ok()?;
    let point = key.to_encoded_point(false);
    let hash = keccak256(&point.as_bytes()[1..]);
    Some(format!("0x{}", hex(&hash[12..])))
}

fn skip_space(b: &[u8], mut i: usize) -> usize {
    while i < b.len() && matches!(b[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    i
}

/// The index after a string's closing quote; `start` is its opening quote.
fn end_of_string(b: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    while i < b.len() {
        match b[i] {
            b'\\' => i += 2,
            b'"' => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// The index after a value: a string, an object or array (to its matching close), or a literal
/// or number (to the next delimiter).
fn end_of_value(b: &[u8], start: usize) -> Option<usize> {
    match *b.get(start)? {
        b'"' => end_of_string(b, start),
        b'{' | b'[' => {
            let mut depth = 0usize;
            let mut i = start;
            while i < b.len() {
                match b[i] {
                    b'"' => {
                        i = end_of_string(b, i)?;
                        continue;
                    }
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => {
                        depth -= 1;
                        if depth == 0 {
                            return Some(i + 1);
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            None
        }
        _ => {
            let mut i = start;
            while i < b.len() && !matches!(b[i], b',' | b'}' | b']' | b' ' | b'\t' | b'\n' | b'\r')
            {
                i += 1;
            }
            (i > start).then_some(i)
        }
    }
}

/// A JSON object's top-level members, each key decoded and its value's own bytes as written; none
/// when `data` isn't one well-formed object (whatever follows its closing brace aside).
pub fn raw_members(data: &[u8]) -> Option<Vec<(String, &[u8])>> {
    let b = data;
    let mut i = skip_space(b, 0);
    if b.get(i) != Some(&b'{') {
        return None;
    }
    i = skip_space(b, i + 1);
    let mut out = vec![];
    if b.get(i) == Some(&b'}') {
        return Some(out);
    }
    loop {
        if b.get(i) != Some(&b'"') {
            return None;
        }
        let key_end = end_of_string(b, i)?;
        let name: String = serde_json::from_slice(&b[i..key_end]).ok()?;
        i = skip_space(b, key_end);
        if b.get(i) != Some(&b':') {
            return None;
        }
        i = skip_space(b, i + 1);
        let value_end = end_of_value(b, i)?;
        out.push((name, &b[i..value_end]));
        i = skip_space(b, value_end);
        match b.get(i)? {
            b',' => i = skip_space(b, i + 1),
            b'}' => return Some(out),
            _ => return None,
        }
    }
}

/// A top-level value's own bytes in a JSON object, as written (`RawJSON.value`): the receipt's
/// signed chain is hashed as KIFF wrote it, not as a decoder would write it again. None when the
/// key isn't there, and when the top level names any key twice: a decoder keeps the last copy
/// and this scan the first, so a repeated key could show one chain while another is checked.
pub fn raw_value<'a>(key: &str, data: &'a [u8]) -> Option<&'a [u8]> {
    let members = raw_members(data)?;
    if repeats_a_key(&members) {
        return None;
    }
    members.into_iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn repeats_a_key(members: &[(String, &[u8])]) -> bool {
    let mut seen = std::collections::BTreeSet::new();
    members.iter().any(|(k, _)| !seen.insert(k.as_str()))
}

/// Whether a JSON object's top level names a key twice (or isn't an object it can read): such a
/// receipt is refused as unreadable, so what's checked and what's shown are the same bytes.
pub fn repeats_a_top_level_key(data: &[u8]) -> bool {
    raw_members(data).is_none_or(|m| repeats_a_key(&m))
}

/// A JSON array's elements, each its own bytes as written (`RawJSON.elements`); none when `data`
/// isn't an array.
pub fn raw_elements(data: &[u8]) -> Option<Vec<&[u8]>> {
    let b = data;
    let mut i = skip_space(b, 0);
    if b.get(i) != Some(&b'[') {
        return None;
    }
    i = skip_space(b, i + 1);
    let mut out = vec![];
    if b.get(i) == Some(&b']') {
        return Some(out);
    }
    loop {
        let end = end_of_value(b, i)?;
        out.push(&b[i..end]);
        i = skip_space(b, end);
        match b.get(i)? {
            b']' => return Some(out),
            b',' => i = skip_space(b, i + 1),
            _ => return None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The iPad's `ReceiptCheckTests.signed`: a receipt the test key signed, as Go writes it.
    const SIGNED: &str = r#"{"schema_version":"0.1","trace_id":"01KTRACE","tenant_id":"t_1","entity_id":"pi_3Q_test","entity_type":"Payment","action_name":"stripe_create_refund","outcome":"allowed","issued_at":"2026-10-21T00:41:03.12Z","causal_chain":[{"sequence":1,"kind":"decision","actor_id":"agt_refunds","occurred_at":"2026-10-21T00:41:03.12Z","message":"held: over \u003c€100\u003e \u0026 more","data_hash":"aa"},{"sequence":2,"kind":"settled","occurred_at":"2026-10-21T00:41:04.12Z","data_hash":"bb","data":{"amount":15000,"answer":"approved_once","big":1e+21,"tool":{"status":"done"}}}],"attestation":{"algorithm":"secp256k1+keccak256","signature":"79917d81e09b24ef196e5b71c72ea1c5cc8951e628b63922072ef59832d0a43e4a0b198bead4ddad1312fe7f1e9b6d41b5f617fe08c5aed59abcd9e5b225a2ce01","wallet_address":"0x2c7536e3605d9c16a7a3d7b1898e529396a65c23","anchor_tx_hash":"","anchored_at":"2026-10-21T00:41:03.12Z"},"risk_score":null,"compliance_map":null,"insurer_envelope":null}"#;
    const TEST_SIGNER: &str = "0x2c7536e3605d9c16a7a3d7b1898e529396a65c23";

    #[test]
    fn keccak_matches_the_standard_vectors() {
        assert_eq!(
            hex(&keccak256(b"")),
            "c5d2460186f7233c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470"
        );
        assert_eq!(
            hex(&keccak256(b"abc")),
            "4e03657aea45a94fc7d47ba826c8d667c0d1e6e33a64a036ec44f58fa12d6c45"
        );
        // Either side of a block (136 bytes), against Go's sha3.NewLegacyKeccak256.
        assert_eq!(
            hex(&keccak256(&[0x61; 135])),
            "34367dc248bbd832f4e3e69dfaac2f92638bd0bbd18f2912ba4ef454919cf446"
        );
        assert_eq!(
            hex(&keccak256(&[0x61; 136])),
            "a6c4d403279fe3e0af03729caada8374b5ca54d8065329a3ebcaeb4b60aa386e"
        );
        assert_eq!(
            hex(&keccak256(&[0x61; 300])),
            "5b7e0e47a96f32a88b4f14ca177982790807c40e1a105742ba0fc1babe1ef826"
        );
    }

    #[test]
    fn a_receipt_kiff_signed_checks_out() {
        assert_eq!(
            check(SIGNED.as_bytes(), TEST_SIGNER),
            Check::Verified {
                signer: TEST_SIGNER.into()
            }
        );
    }

    #[test]
    fn the_chain_is_hashed_as_written_not_reencoded() {
        let chain =
            std::str::from_utf8(raw_value("causal_chain", SIGNED.as_bytes()).unwrap()).unwrap();
        assert!(chain.starts_with(r#"[{"sequence":1,"kind":"decision""#));
        // As Go wrote it, escaped: a decoder and re-encoder would write "<€100> & more".
        assert!(chain.contains(r#"\u003c€100\u003e \u0026 more"#));
        assert!(chain.ends_with(r#""big":1e+21,"tool":{"status":"done"}}}]"#));
        assert_eq!(raw_value("missing", SIGNED.as_bytes()), None);
        assert_eq!(raw_elements(chain.as_bytes()).unwrap().len(), 2);
        assert_eq!(raw_elements(b"[]").unwrap().len(), 0);
        assert_eq!(raw_elements(b"{}"), None);
    }

    #[test]
    fn a_changed_byte_in_the_chain_fails() {
        let changed = SIGNED.replace(r#""data_hash":"bb""#, r#""data_hash":"bc""#);
        assert_ne!(changed, SIGNED);
        assert_eq!(
            check(changed.as_bytes(), TEST_SIGNER),
            Check::Failed(Failure::SignatureMismatch)
        );
        let amount = SIGNED.replace(r#""amount":15000"#, r#""amount":95000"#);
        assert_eq!(
            check(amount.as_bytes(), TEST_SIGNER),
            Check::Failed(Failure::SignatureMismatch)
        );
    }

    #[test]
    fn a_receipt_signed_by_another_key_is_not_kiffs() {
        // The test key's receipt is sound, but production pins KIFF's signer.
        assert_eq!(
            check(SIGNED.as_bytes(), KIFF_SIGNER),
            Check::Failed(Failure::NotKiff)
        );
    }

    #[test]
    fn not_signed_yet_is_not_a_failure() {
        let i = SIGNED.find(r#""attestation":{"#).unwrap();
        let j = i + SIGNED[i..].find('}').unwrap() + 1;
        let pending = format!("{}\"attestation\":null{}", &SIGNED[..i], &SIGNED[j..]);
        assert_eq!(check(pending.as_bytes(), KIFF_SIGNER), Check::NotSignedYet);
        assert_eq!(
            check(b"not json", KIFF_SIGNER),
            Check::Failed(Failure::Unreadable)
        );
        let other = SIGNED.replace("secp256k1+keccak256", "ed25519");
        assert_eq!(
            check(other.as_bytes(), TEST_SIGNER),
            Check::Failed(Failure::UnknownSignature)
        );
    }

    #[test]
    fn a_malformed_signature_fails_without_stopping_the_app() {
        assert_eq!(recover_address(&[1; 32], &[0; 65]), None);
        assert_eq!(recover_address(&[1; 32], &[0xFF; 65]), None);
        let i = SIGNED.find(r#""signature":""#).unwrap() + r#""signature":""#.len();
        let j = i + SIGNED[i..].find('"').unwrap();
        let short = format!("{}00{}", &SIGNED[..i], &SIGNED[j..]);
        assert_eq!(
            check(short.as_bytes(), TEST_SIGNER),
            Check::Failed(Failure::MalformedSignature)
        );
    }

    /// A receipt that names a top-level key twice is refused: a decoder keeps the last copy, the
    /// chain's scan the first, so the steps shown could differ from the steps checked.
    #[test]
    fn a_key_written_twice_is_refused() {
        let chain =
            std::str::from_utf8(raw_value("causal_chain", SIGNED.as_bytes()).unwrap()).unwrap();
        let twice = SIGNED.replacen(
            r#""attestation":"#,
            r#""causal_chain":[{"sequence":1,"kind":"decision","message":"other"}],"attestation":"#,
            1,
        );
        assert!(twice.contains(chain));
        assert_eq!(
            check(twice.as_bytes(), TEST_SIGNER),
            Check::Failed(Failure::Unreadable)
        );
        assert_eq!(raw_value("causal_chain", twice.as_bytes()), None);
        assert!(repeats_a_top_level_key(twice.as_bytes()));
        let i = SIGNED.find(r#""attestation":{"#).unwrap();
        let j = i + SIGNED[i..].find('}').unwrap() + 1;
        let two = format!("{},\"attestation\":null{}", &SIGNED[..j], &SIGNED[j..]);
        assert_eq!(
            check(two.as_bytes(), TEST_SIGNER),
            Check::Failed(Failure::Unreadable)
        );
        // A key repeated deeper down is the chain's own business, and still checks.
        assert!(!repeats_a_top_level_key(SIGNED.as_bytes()));
        assert!(!repeats_a_top_level_key(br#"{"a":{"b":1,"b":2}}"#));
        assert!(repeats_a_top_level_key(b"[1]"));
    }

    /// libsecp256k1 recovers from a high S too: the same signature with S as n − S and the
    /// recovery id's parity flipped recovers the same signer.
    #[test]
    fn a_high_s_recovers_the_same_signer() {
        let chain = raw_value("causal_chain", SIGNED.as_bytes()).unwrap();
        let digest = keccak256(hex(&Sha256::digest(chain)).as_bytes());
        let low = unhex("79917d81e09b24ef196e5b71c72ea1c5cc8951e628b63922072ef59832d0a43e4a0b198bead4ddad1312fe7f1e9b6d41b5f617fe08c5aed59abcd9e5b225a2ce01").unwrap();
        let sig = Signature::from_slice(&low[..64]).unwrap();
        let (r, s) = sig.split_scalars();
        let high = Signature::from_scalars(r, -*s).unwrap();
        assert!(high.normalize_s().is_some(), "n − S is the high S");
        let mut bytes = high.to_bytes().to_vec();
        bytes.push(1 - (low[64] % 27));
        assert_eq!(
            recover_address(&digest, &bytes).as_deref(),
            Some(TEST_SIGNER)
        );
        assert_eq!(recover_address(&digest, &low).as_deref(), Some(TEST_SIGNER));
    }

    #[test]
    fn each_failure_has_its_words() {
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            for f in [
                Failure::Unreadable,
                Failure::UnknownSignature,
                Failure::NoSteps,
                Failure::MalformedSignature,
                Failure::SignatureMismatch,
                Failure::NotKiff,
            ] {
                f.words(&w).unwrap();
            }
        }
        let en = WordBook::load(&WordBook::default_root(), "en").unwrap();
        assert_eq!(
            Failure::SignatureMismatch.words(&en).unwrap(),
            "The signature doesn't match these steps."
        );
        assert_eq!(
            Failure::NotKiff.words(&en).unwrap(),
            "It isn't signed by KIFF."
        );
    }
}
