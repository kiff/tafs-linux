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
        && (act.request().is_some() || act.answer().is_some())
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
    // A request signed in its header, or a held call's answer signed in its body.
    let signs = match (act.request(), act.answer()) {
        (Some((method, path, body)), _) => Signs::Request { method, path, body },
        (None, Some((hold, answer, version))) => Signs::Answer {
            hold: hold.to_string(),
            answer,
            version,
        },
        (None, None) => return,
    };
    let Some(credential) = kept
        .security_key
        .as_deref()
        .and_then(passkey::credential_id)
    else {
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
            let message = match &signs {
                Signs::Request { method, path, body } => {
                    passkey::request_v1(&tenant, method, path, body, &signed_at)
                }
                Signs::Answer {
                    hold,
                    answer,
                    version,
                } => passkey::answer_v1(&tenant, hold, answer, *version, &signed_at),
            };
            let client_data = passkey::client_data_json(&message);
            let ask = AssertionAsk::new(&passkey::client_data_hash(&client_data), &credential)?;
            let answer = open.get_assertion(&ask, Some(pin))?;
            let assertion = Assertion {
                credential_id: answer.credential_id.unwrap_or_else(|| credential.clone()),
                authenticator_data: answer.authenticator_data,
                signature: answer.signature,
            };
            // What KIFF would refuse is never sent: the assertion in the request's header, or in
            // the answer's body.
            let signed = match &signs {
                Signs::Request { .. } => {
                    passkey::header(&signed_at, &credential, &client_data, &assertion, None)
                }
                Signs::Answer { .. } => {
                    passkey::answer_body(&signed_at, &credential, &client_data, &assertion)
                        .map(|b| String::from_utf8(b).unwrap_or_default())
                }
            }
            .map_err(|_| KeyError::Unreadable)?;
            // Its Cards first, as the iPad's removal: approved, then revoked, then sent.
            let cards_left = revoke_first
                .iter()
                .filter(|card| !revoke(&token, &languages, card))
                .count();
            // From here the act may have reached KIFF: whatever comes back, the sheet closes with
            // KIFF's line (or the network's), and never offers a try that would send it again.
            // `Ok`: it went, with a removal's line when a part didn't; `Err`: why it didn't.
            let answered = match &signs {
                Signs::Request { method, path, body } => crate::kiff::send_with_key(
                    &token, &languages, &act_sent, method, path, body, &signed,
                ),
                Signs::Answer { .. } => {
                    crate::kiff::send_answer(&token, &languages, &act_sent, signed.as_bytes())
                }
            };
            let sent: Result<Option<String>, String> = match answered {
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

/// Whether `act` can end its connections here (the owner's 22 Nov rule): it's Disconnect or Clear,
/// and this computer has a security key, so a touch and a PIN check the person before the device
/// key signs at planque. Anything that grants access never can; a computer with no key, the phone.
pub fn can_end(kept: Option<&KeptSession>, act: &Act) -> bool {
    passkey::ENABLED
        && kept.is_some_and(|k| {
            k.approval_mode == ApprovalMode::Passkey
                && k.security_key.is_some()
                // The key's answer is checked against its own public key, so a session kept
                // without it asks the phone instead.
                && k.security_key_public.as_deref().and_then(passkey::credential_id).is_some()
        })
        && act.ends_connections().is_some()
}

/// Disconnect or Clear with this computer's security key: the PIN sheet asks the key registered
/// here to touch and take its PIN over `tafs.presence.v1` (never sent anywhere), checked as KIFF
/// checks an assertion (kiff.dev's, the person present and verified, the credential asked, not a
/// synced passkey); only then does the device key sign each connection's `DELETE` at planque
/// (`planque::Safe::Disconnect`), one after another. `done` hears once, as `approve`'s.
pub fn end_with_presence(
    w: Rc<WordBook>,
    parent: &impl gtk::prelude::IsA<gtk::Widget>,
    kept: KeptSession,
    languages: Vec<&'static str>,
    act: &Act,
    what: Option<&str>,
    done: Approved,
) {
    let Some(grants) = act.ends_connections() else {
        return;
    };
    let Some(credential) = kept
        .security_key
        .as_deref()
        .and_then(passkey::credential_id)
    else {
        return;
    };
    // The registered key's public half: its answer is checked against it, since nothing else
    // ever will. None: nothing goes (`can_end` already offered the phone).
    let Some(public_key) = kept
        .security_key_public
        .as_deref()
        .and_then(passkey::credential_id)
    else {
        return;
    };
    let tenant = kept.tenant.clone().unwrap_or_default();
    let open = Arc::new(OpenKey::default());
    let refused: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let attempt: crate::pin_sheet::Attempt = {
        let (open, refused, w2) = (open.clone(), refused.clone(), (*w).clone());
        Arc::new(move |pin| {
            let signed_at = SIGNING_TIME
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .next(chrono::Utc::now());
            let message = passkey::presence_v1(&tenant, &grants, &signed_at);
            let client_data = passkey::client_data_json(&message);
            let ask = AssertionAsk::new(&passkey::client_data_hash(&client_data), &credential)?;
            let answer = open.get_assertion(&ask, Some(pin))?;
            let answered = answer
                .credential_id
                .clone()
                .unwrap_or_else(|| credential.clone());
            // The person checked as KIFF would check them, or nothing is sent; and the answer is
            // the registered key's own, by its signature, since it's never sent to be checked.
            if passkey::refusal(&answer.authenticator_data, &credential, &answered).is_some()
                || !passkey::signature_holds(
                    &public_key,
                    &answer.authenticator_data,
                    &client_data,
                    &answer.signature,
                )
            {
                return Err(KeyError::Unreadable);
            }
            let line = end_at_planque(&w2, &kept, &languages, &grants);
            *refused.lock().unwrap_or_else(|e| e.into_inner()) = line.clone();
            match line {
                None => Ok(()),
                Some(_) => Err(KeyError::Sent),
            }
        })
    };
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

/// Each connection's `DELETE` at planque, signed by the device key once the security key checked
/// the person: none when every one went, else planque's line for the first that didn't (one
/// already gone counts as gone). Never tried again from here: the sheet closes with the line.
fn end_at_planque(
    w: &WordBook,
    kept: &KeptSession,
    languages: &[&'static str],
    grants: &[String],
) -> Option<String> {
    let Some(device) = crate::coach::this_device(kept, languages) else {
        return Some(
            tafs_core::signin::KiffError::Offline
                .words(w)
                .unwrap_or_default(),
        );
    };
    for grant in grants {
        match crate::kiff::planque_safe(
            kept,
            &device,
            languages,
            tafs_core::planque::Safe::Disconnect(grant),
        ) {
            Ok(a) if (200..300).contains(&a.status) || a.status == 404 => {}
            Ok(a) => {
                return Some(
                    tafs_core::signin::refusal(&a, Some(w))
                        .words(w)
                        .unwrap_or_default(),
                )
            }
            Err(e) => return Some(e.words(w).unwrap_or_default()),
        }
    }
    None
}

/// What the key signs: a request, its assertion in a header (`kiff.request.v1`); or a held call's
/// answer, its assertion in the body (`kiff.answer.v1`).
enum Signs {
    Request {
        method: &'static str,
        path: String,
        body: Vec<u8>,
    },
    Answer {
        hold: String,
        answer: &'static str,
        version: i64,
    },
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
    fn only_disconnect_and_clear_end_with_the_key_and_only_with_it() {
        use tafs_core::keys::DeviceKey;
        let token: tafs_core::signin::DeviceToken = serde_json::from_value(serde_json::json!({
            "access_token": "t", "token_type": "Bearer", "expires_in": 0,
            "tenant": "x", "subject": "s", "approval_mode": "passkey"
        }))
        .unwrap();
        let key = DeviceKey::generate();
        let cred = tafs_core::pairing_key::Credential {
            id: b"c".to_vec(),
            public_key: b"spki".to_vec(),
        };
        let keyed = tafs_core::pairing::kept_with_key(&token, &key, &cred, 0);
        let disconnect = Act::Disconnect { grant: "g1".into() };
        let clear = Act::ClearRemoved {
            grants: vec!["g1".into(), "g2".into()],
        };
        assert!(can_end(Some(&keyed), &disconnect));
        assert!(can_end(Some(&keyed), &clear));
        // Neither is a request the key approves itself.
        assert!(!can_approve(Some(&keyed), &disconnect));
        // What grants access never ends this way.
        for act in [
            Act::SignInAgain { grant: "g1".into() },
            Act::ConnectAgain { grant: "g1".into() },
            Act::CreateGroup { name: "x".into() },
        ] {
            assert!(!can_end(Some(&keyed), &act), "{act:?}");
        }
        // A computer with no key, or a view-only session: the phone.
        let view = tafs_core::pairing::kept(&token, &key, 0);
        assert!(!can_end(Some(&view), &disconnect));
        let unkeyed = KeptSession {
            security_key: None,
            security_key_public: None,
            ..keyed
        };
        assert!(!can_end(Some(&unkeyed), &disconnect));
        assert!(!can_end(None, &disconnect));
        // A session kept before the key's public half was: its answer couldn't be checked, so
        // the phone.
        let unchecked = KeptSession {
            security_key_public: None,
            ..unkeyed.clone()
        };
        let unchecked = KeptSession {
            security_key: keyed.security_key.clone(),
            ..unchecked
        };
        assert!(!can_end(Some(&unchecked), &disconnect));
    }

    #[test]
    fn a_connection_is_ended_at_planque_only_behind_the_key() {
        // `Safe::Disconnect` is sent from one place: after the key's touch and PIN, here.
        let root = env!("CARGO_MANIFEST_DIR");
        for entry in std::fs::read_dir(format!("{root}/src")).unwrap() {
            let path = entry.unwrap().path();
            let source = std::fs::read_to_string(&path).unwrap();
            let code = source.split("#[cfg(test)]").next().unwrap_or_default();
            // A call, not a mention in a comment.
            let uses = code.matches("Safe::Disconnect(").count();
            if path.ends_with("key_approval.rs") {
                assert_eq!(uses, 1, "one send, in end_at_planque");
                let send = code.find("Safe::Disconnect(").unwrap();
                let checked = code
                    .find("passkey::refusal(&answer.authenticator_data")
                    .unwrap();
                // The answer's own signature, against the registered key, before anything goes.
                let signed = code.find("passkey::signature_holds(").unwrap();
                assert!(checked < signed, "flags, then the signature");
                assert!(
                    signed < code.find("end_at_planque(&w2").unwrap(),
                    "the signature is checked before planque hears anything"
                );
                let ended = code.find("end_at_planque(&w2").unwrap();
                assert!(checked < ended && ended < send, "checked, then ended");
            } else {
                assert_eq!(uses, 0, "{}", path.display());
            }
        }
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
            &Act::ApproveOnce {
                call: "c".into(),
                accounting_version: None,
            }
        ));
        // A held call's answer with its accounting version is signed here: Approve once and
        // Extra room alike.
        assert!(can_approve(
            Some(&keyed),
            &Act::ApproveOnce {
                call: "c".into(),
                accounting_version: Some(7),
            }
        ));
        assert!(can_approve(
            Some(&keyed),
            &Act::ExtraRoom {
                call: "c".into(),
                accounting_version: Some(7),
            }
        ));
        // A view-only session, or a passkey one with no key registered here, can't.
        let view = tafs_core::pairing::kept(&token, &key, 0);
        assert!(!can_approve(Some(&view), &act));
        let unkeyed = KeptSession {
            security_key: None,
            security_key_public: None,
            ..keyed
        };
        assert!(!can_approve(Some(&unkeyed), &act));
    }
}
