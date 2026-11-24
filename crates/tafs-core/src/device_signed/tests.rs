use super::*;
use p256::ecdsa::signature::{Signer, Verifier};
use p256::ecdsa::{DerSignature, SigningKey, VerifyingKey};
use p256::pkcs8::{DecodePublicKey, EncodePublicKey};

const AT: &str = "2026-11-24T01:30:00Z";
const CREDENTIAL: &[u8] = b"credential-1";

/// This computer's security key, registered when it was paired.
fn security_key() -> (SigningKey, Vec<u8>) {
    let key = SigningKey::from_slice(&[11u8; 32]).unwrap();
    let public = key
        .verifying_key()
        .to_public_key_der()
        .unwrap()
        .as_bytes()
        .to_vec();
    (key, public)
}

/// Stands in for the security key: signs authData ‖ sha256(clientData), as an authenticator.
fn answer(key: &SigningKey, flags: u8, credential: &[u8], client: &[u8]) -> Assertion {
    let mut auth = Sha256::digest(passkey::RP_ID.as_bytes()).to_vec();
    auth.push(flags);
    auth.extend_from_slice(&3u32.to_be_bytes());
    let mut signed = auth.clone();
    signed.extend_from_slice(&Sha256::digest(client));
    let sig: DerSignature = key.sign(&signed);
    Assertion {
        credential_id: credential.to_vec(),
        authenticator_data: auth,
        signature: sig.as_bytes().to_vec(),
    }
}

const PRESENT_VERIFIED: u8 = 0x05;

fn raise() -> Act {
    Act::RaiseLimit {
        from_cents: 100_000,
    }
}

fn decline() -> Act {
    Act::DeclineRequest {
        request: "req_1".into(),
        note: "Not this month".into(),
    }
}

#[test]
fn only_raise_and_decline_are_signed_this_way() {
    assert!(Request::new("t_acme", &raise(), AT).is_some());
    assert!(Request::new("t_acme", &decline(), AT).is_some());
    assert!(Request::new("", &raise(), AT).is_none(), "no tenant");
    for act in [
        Act::Disconnect { grant: "g1".into() },
        Act::SetTimeZone {
            zone: "Europe/Paris".into(),
        },
        Act::SignOutDevice {
            device: "d1".into(),
        },
    ] {
        assert!(!act.device_signed());
        assert!(Request::new("t_acme", &act, AT).is_none(), "{act:?}");
    }
}

#[test]
fn the_device_key_signs_kiffs_own_message_after_the_person() {
    let (sk, public) = security_key();
    let device = DeviceKey::generate();
    let r = Request::new("t_acme", &raise(), AT).unwrap();
    let a = answer(&sk, PRESENT_VERIFIED, CREDENTIAL, &r.client_data());
    let p = r.presence(CREDENTIAL, &public, &a).unwrap();
    let s = r.sign(p, &device).unwrap();
    assert_eq!(
        (s.method, s.path.as_str(), s.body.as_slice()),
        (
            "PUT",
            "/v1/me/usage/limit",
            br#"{"from_cents":100000,"raise":true}"#.as_slice()
        )
    );
    // KIFF's check (`VerifyRequest`): `t=…,v1=…`, the device key's DER ECDSA over the SHA-256 of
    // `kiff.request.v1` for exactly this request.
    let (t, v1) = s.header.split_once(",v1=").unwrap();
    assert_eq!(t, format!("t={AT}"));
    let message = passkey::request_v1("t_acme", "PUT", "/v1/me/usage/limit", &s.body, AT);
    let vk = VerifyingKey::from_public_key_der(&device.public_spki_der()).unwrap();
    use base64::Engine;
    let sig = base64::engine::general_purpose::STANDARD
        .decode(v1)
        .unwrap();
    let sig = p256::ecdsa::Signature::from_der(&sig).unwrap();
    assert!(vk.verify(&message, &sig).is_ok());
}

#[test]
fn what_the_security_key_signs_is_never_a_message_kiff_takes() {
    let r = Request::new("t_acme", &decline(), AT).unwrap();
    let client = String::from_utf8(r.client_data()).unwrap();
    // The challenge is the presence message's, never the request's own: a passkey assertion KIFF
    // would take for this request is never made.
    assert!(client.contains(&passkey::challenge(&r.presence)));
    assert!(!client.contains(&passkey::challenge(&r.message)));
    assert!(r
        .presence
        .starts_with(b"tafs.presence.request.v1\nt_acme\n"));
    assert!(!r.presence.starts_with(b"kiff."));
}

#[test]
fn no_request_is_signed_without_a_checked_presence_for_it() {
    let (sk, public) = security_key();
    let (other_key, _) = {
        let k = SigningKey::from_slice(&[12u8; 32]).unwrap();
        (k, ())
    };
    let r = Request::new("t_acme", &raise(), AT).unwrap();
    let client = r.client_data();
    // The person not verified (no PIN), not present, a synced passkey, another credential,
    // another relying party: what KIFF would refuse.
    for (flags, cred, why) in [
        (0x01, CREDENTIAL, Refusal::NotVerified),
        (0x04, CREDENTIAL, Refusal::NotVerified),
        (0x05 | 0x08, CREDENTIAL, Refusal::CanLeave),
        (0x05, b"credential-2".as_slice(), Refusal::AnotherCredential),
    ] {
        let a = answer(&sk, flags, cred, &client);
        assert_eq!(
            r.presence(CREDENTIAL, &public, &a),
            Err(NotPresent::Refused(why))
        );
    }
    let mut a = answer(&sk, PRESENT_VERIFIED, CREDENTIAL, &client);
    a.authenticator_data[0] ^= 1;
    assert!(matches!(
        r.presence(CREDENTIAL, &public, &a),
        Err(NotPresent::Refused(Refusal::NotKiff))
    ));
    // Another key answering with the registered credential's id.
    let a = answer(&other_key, PRESENT_VERIFIED, CREDENTIAL, &client);
    assert_eq!(
        r.presence(CREDENTIAL, &public, &a),
        Err(NotPresent::NotThisKey)
    );
    // The registered key, over another request (a Decline, or the same Raise signed at another
    // time): not this one's.
    for other in [
        Request::new("t_acme", &decline(), AT).unwrap(),
        Request::new("t_acme", &raise(), "2026-11-24T01:30:01Z").unwrap(),
        Request::new("t_other", &raise(), AT).unwrap(),
        Request::new("t_acme", &Act::RaiseLimit { from_cents: 1 }, AT).unwrap(),
    ] {
        let a = answer(&sk, PRESENT_VERIFIED, CREDENTIAL, &other.client_data());
        assert_eq!(
            r.presence(CREDENTIAL, &public, &a),
            Err(NotPresent::NotThisKey)
        );
    }
}

#[test]
fn a_presence_signs_its_own_request_once_and_no_other() {
    let (sk, public) = security_key();
    let device = DeviceKey::generate();
    let raise_r = Request::new("t_acme", &raise(), AT).unwrap();
    let decline_r = Request::new("t_acme", &decline(), AT).unwrap();
    let a = answer(&sk, PRESENT_VERIFIED, CREDENTIAL, &raise_r.client_data());
    let p = raise_r.presence(CREDENTIAL, &public, &a).unwrap();
    // Raise's presence doesn't sign the Decline, and is spent trying.
    assert!(decline_r.sign(p, &device).is_none());
    // Raise needs the person again: a new check makes a new presence (`sign` took the request and
    // the presence both, so neither can sign twice; the doc tests on `Presence` pin that it can't
    // be made or copied outside this module).
    let p = raise_r.presence(CREDENTIAL, &public, &a).unwrap();
    assert!(raise_r.sign(p, &device).is_some());
}

#[test]
fn kiffs_refusals_are_said_in_the_apps_words() {
    let w = crate::words::WordBook::load(&crate::words::WordBook::default_root(), "en").unwrap();
    let said = |act: &Act, status: u16, body: &str| {
        not_taken(
            &w,
            act,
            &crate::signin::Answer {
                status,
                body: body.as_bytes().to_vec(),
            },
        )
        .unwrap()
    };
    assert_eq!(
        said(
            &raise(),
            409,
            r#"{"error":"limit_changed","limit_cents":13000}"#
        ),
        w.line("usage.limit_changed").unwrap()
    );
    assert_eq!(
        said(
            &raise(),
            409,
            r#"{"error":"limit_ceiling","limit_cents":130000}"#
        ),
        w.line("usage.limit_ceiling").unwrap()
    );
    let other = said(&raise(), 403, r#"{"error":"forbidden"}"#);
    assert!(other.starts_with("The limit wasn't raised."), "{other}");
    // A decline's 409 is no raise's.
    let declined = said(&decline(), 409, r#"{"error":"limit_changed"}"#);
    assert!(
        declined.starts_with("The request wasn't declined."),
        "{declined}"
    );
    let offline = not_sent(&w, &raise(), &crate::signin::KiffError::Offline).unwrap();
    assert!(offline.starts_with("The limit wasn't raised."), "{offline}");
}
