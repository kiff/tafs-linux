//! Approving one act with this computer's security key (kiff/tafs `docs/computer-passkeys.md`,
//! "What a passkey approval carries"; the PM's decision (i), kiff/work#17, 22 Nov): the act's
//! whole request (`Act::request`) is the message a device key would sign (`kiff.request.v1`), its
//! hash the challenge; the PIN sheet asks the key registered with this computer when it was
//! paired, once; the answer is checked as KIFF checks it; and the request goes with its
//! `KIFF-Passkey-Assertion` header, those bytes and no others. One request, one approval: nothing
//! is kept to approve another.
//!
//! Only a session KIFF made `passkey`, whose pairing registered the key, approves this way; any
//! other still shows that this computer can't approve.

use std::sync::{Arc, Mutex};

use std::rc::Rc;
use tafs_core::passkey::{self, Assertion, SigningTime};
use tafs_core::security_key::{self, AssertionAsk};
use tafs_core::session::{ApprovalMode, KeptSession};
use tafs_core::signed::Act;
use tafs_core::words::WordBook;

use crate::fido_key::{KeyError, OpenKey};

/// Hears once: `Ok` once KIFF took the act, `Err` with the line to show, nothing when cancelled.
pub type Approved = Rc<dyn Fn(Option<Result<(), String>>)>;

/// `signed_at` never repeats, across every approval this app makes.
static SIGNING_TIME: Mutex<SigningTime> = Mutex::new(SigningTime::new());

/// Whether `act` can be approved here: the key is on, the session approves with a passkey and
/// holds this computer's key, and the act is one whose whole request this app writes.
pub fn can_approve(kept: Option<&KeptSession>, act: &Act) -> bool {
    passkey::ENABLED
        && kept
            .is_some_and(|k| k.approval_mode == ApprovalMode::Passkey && k.security_key.is_some())
        && act.request().is_some()
}

/// Asks the key to approve `act` over `parent`, and sends it. `done` hears once: `Ok` once KIFF
/// took it, or the line to show; nothing when the person cancelled.
///
/// `revoke_first`: Cards revoked at KIFF once the key approved and before the act goes (a taf
/// removed for good, `AppModel.removeTaf`): KIFF's, so revoked with the session, one plain call
/// each; how many KIFF didn't revoke is said with tafs cloud's answer.
#[allow(clippy::too_many_arguments)]
pub fn approve(
    w: Rc<WordBook>,
    parent: &impl gtk::prelude::IsA<gtk::Widget>,
    kept: KeptSession,
    languages: Vec<&'static str>,
    act: &Act,
    what: Option<&str>,
    revoke_first: Vec<String>,
    done: Approved,
) {
    let (Some((method, path, body)), Some(credential)) = (
        act.request(),
        kept.security_key
            .as_deref()
            .and_then(passkey::credential_id),
    ) else {
        return;
    };
    let tenant = kept.tenant.clone().unwrap_or_default();
    let token = kept.token.clone();
    let open = Arc::new(OpenKey::default());
    // KIFF's answer when it refused what the key approved: said once the sheet closes.
    let refused: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let removing = matches!(act, Act::RemoveTaf { .. });
    let act_sent = act.clone();
    let attempt: crate::pin_sheet::Attempt = {
        let (open, refused, w2) = (open.clone(), refused.clone(), (*w).clone());
        Arc::new(move |pin| {
            let signed_at = SIGNING_TIME
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .next(chrono::Utc::now());
            let message = passkey::request_v1(&tenant, method, &path, &body, &signed_at);
            let client_data = passkey::client_data_json(&message);
            let ask = AssertionAsk::new(&passkey::client_data_hash(&client_data), &credential)?;
            let answer = open.get_assertion(&ask, Some(pin))?;
            let assertion = Assertion {
                credential_id: answer.credential_id.unwrap_or_else(|| credential.clone()),
                authenticator_data: answer.authenticator_data,
                signature: answer.signature,
            };
            // What KIFF would refuse is never sent.
            let header = passkey::header(&signed_at, &credential, &client_data, &assertion, None)
                .map_err(|_| KeyError::Unreadable)?;
            // Its Cards first, as the iPad's removal: approved, then revoked, then sent.
            let cards_left = revoke_first
                .iter()
                .filter(|card| !revoke(&token, &languages, card))
                .count();
            // From here the act may have reached KIFF: whatever comes back, the sheet closes with
            // KIFF's line (or the network's), and never offers a try that would send it again.
            // `Ok`: it went, with a removal's line when a part didn't; `Err`: why it didn't.
            let sent: Result<Option<String>, String> = match crate::kiff::send_with_key(
                &token, &languages, &act_sent, method, &path, &body, &header,
            ) {
                Ok(a) if (200..300).contains(&a.status) => Ok(removing
                    .then(|| tafs_core::removal::read(&a.body))
                    .flatten()
                    .and_then(|r| tafs_core::removal::after(&w2, cards_left, &r).ok())
                    .flatten()),
                Ok(a) => Err(tafs_core::signin::refusal(&a, Some(&w2))
                    .words(&w2)
                    .unwrap_or_default()),
                Err(e) => Err(e.words(&w2).unwrap_or_default()),
            };
            let line = match sent {
                Ok(None) => return Ok(()),
                Ok(Some(not_finished)) => Some(not_finished),
                // A removal that didn't answer says what became of its Cards first.
                Err(why) if removing => {
                    tafs_core::removal::cloud_did_not_answer(&w2, cards_left, &why).ok()
                }
                Err(why) => Some(why),
            };
            *refused.lock().unwrap_or_else(|e| e.into_inner()) = line;
            Err(KeyError::Sent)
        })
    };
    // Its own handle: each `OpenKey` call opens the key, asks once and closes it, and libfido2
    // gives each open its own channel. If the tries are still being read when the PIN is sent,
    // the key answers one and refuses the other as busy: no tries line, or "failed" and the sheet
    // stays open to try again, the act not sent. One shared `OpenKey` would track only one.
    let read_tries: crate::pin_sheet::ReadTries = {
        let open = Arc::new(OpenKey::default());
        Arc::new(move || open.tries_left())
    };
    let stop: crate::pin_sheet::StopKey = {
        let open = open.clone();
        Arc::new(move || open.cancel())
    };
    let sheet_done: crate::pin_sheet::Done = {
        let w = w.clone();
        Rc::new(move |e| {
            let kiff = refused.lock().unwrap_or_else(|e| e.into_inner()).take();
            done(closed_with(&w, e, kiff))
        })
    };
    crate::pin_sheet::present(w, parent, what, attempt, read_tries, stop, sheet_done);
}

/// Revokes one Card at KIFF as the session (`kiff.revoke(cardID:)`): true once KIFF took it.
fn revoke(token: &str, languages: &[&str], card: &str) -> bool {
    tafs_core::signed::Unsigned::new("POST", tafs_core::signed::revoke_path(card), b"{}".to_vec())
        .and_then(|call| crate::kiff::kiff_send(token, languages, &call).ok())
        .is_some_and(|a| (200..300).contains(&a.status))
}

/// What the sheet closing comes to: approved; cancelled (nothing); or the line to show, KIFF's
/// own when it answered, never a debug string.
fn closed_with(
    w: &WordBook,
    e: Option<KeyError>,
    kiff: Option<String>,
) -> Option<Result<(), String>> {
    match e {
        None => Some(Ok(())),
        Some(KeyError::Cancelled) => None,
        Some(_) => Some(Err(kiff
            .filter(|l| !l.is_empty())
            .unwrap_or_else(|| security_key::words::failed(w).unwrap_or_default()))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_closed_sheet_says_kiffs_line_or_the_shared_one() {
        let w = WordBook::load(&WordBook::default_root(), "en").expect("the words at the pin");
        let failed = security_key::words::failed(&w).unwrap();
        assert_eq!(closed_with(&w, None, None), Some(Ok(())));
        assert_eq!(closed_with(&w, Some(KeyError::Cancelled), None), None);
        assert_eq!(
            closed_with(&w, Some(KeyError::Sent), Some("KIFF said no.".into())),
            Some(Err("KIFF said no.".into()))
        );
        // No answer to show: the shared line, never `Sent` or `Unreadable` as written in code.
        for e in [KeyError::Sent, KeyError::Unreadable] {
            assert_eq!(closed_with(&w, Some(e), None), Some(Err(failed.clone())));
        }
        assert_eq!(
            closed_with(&w, Some(KeyError::Sent), Some(String::new())),
            Some(Err(failed))
        );
    }

    #[test]
    fn only_a_session_with_the_key_approves() {
        use tafs_core::keys::DeviceKey;
        let act = Act::CreateGroup { name: "x".into() };
        assert!(!can_approve(None, &act));
        let token: tafs_core::signin::DeviceToken = serde_json::from_value(serde_json::json!({
            "access_token": "t", "token_type": "Bearer", "expires_in": 0,
            "tenant": "x", "subject": "s", "approval_mode": "passkey"
        }))
        .unwrap();
        let key = DeviceKey::generate();
        let cred = tafs_core::pairing_key::Credential {
            id: b"c".to_vec(),
            public_key: vec![],
        };
        let keyed = tafs_core::pairing::kept_with_key(&token, &key, &cred, 0);
        assert!(can_approve(Some(&keyed), &act));
        // Not one whose whole request this app doesn't write.
        assert!(!can_approve(
            Some(&keyed),
            &Act::ApproveOnce { call: "c".into() }
        ));
        // A view-only session, or a passkey one with no key registered here, can't.
        let view = tafs_core::pairing::kept(&token, &key, 0);
        assert!(!can_approve(Some(&view), &act));
        let unkeyed = KeptSession {
            security_key: None,
            ..keyed
        };
        assert!(!can_approve(Some(&unkeyed), &act));
    }
}
