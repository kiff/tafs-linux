//! An approval card's Approve for the one step this computer can sign: **the Card's** (`cards`),
//! KIFF's draft confirm, which KIFF takes from this computer's security key (kiff/cloud
//! `confirmDraft`, the policy's passkey path), as the iPad's `approve(_:shown:shownDraft:)` does
//! for its `cards` step and as tafs-macos's `SetupApproval`. The connection's own step (`connect`)
//! is planque's routine route, which takes the phone's approval key only, so it stays the phone's.
//!
//! tafs cloud decides each step. This computer asks (`approval`, with its planque read proof), and
//! tafs cloud answers:
//! - `answered`: done, with its note;
//! - `read_again`: what the card showed is out of date;
//! - `sign`: one ask.
//!
//! An ask is signed only when it's the Card's, for the draft and the version the card drew, and
//! its message is exactly the one this computer builds itself (`cards_message`). Then the draft is
//! confirmed on the security key, and what came of it is reported (`report`), signed by the device
//! key over the exact bytes sent. Any other ask signs nothing.

use serde_json::{Map, Value};

use crate::access::escaped;
use crate::keys::DeviceKey;

/// KIFF's draft an ask confirms, at exactly this version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskDraft {
    pub draft: String,
    pub version: String,
}

/// One ask from tafs cloud: what to sign, and the words for its prompt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ask {
    pub id: String,
    /// `connect` or `cards`.
    pub step: String,
    /// The exact message the step stands for, which this computer rebuilds and compares.
    pub message: String,
    /// What the prompt says.
    pub reason: String,
    pub kiff: Option<AskDraft>,
}

/// tafs cloud's answer to Approve, or to what came of a signed ask.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Done: Coach heard it, with tafs cloud's note when it gave one.
    Answered { note: Option<String> },
    /// What the card showed is out of date: read the draft again, and sign nothing.
    ReadAgain { note: Option<String> },
    /// One ask to sign, in the chat `session` names.
    Sign { ask: Ask, session: Option<String> },
    /// An outcome this app doesn't know, or an ask it can't read: nothing is signed.
    Unknown,
}

fn text(o: &Map<String, Value>, key: &str) -> Option<String> {
    o.get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|s| !s.trim().is_empty())
}

impl Decision {
    /// tafs cloud's answer, read leniently: anything unreadable is `Unknown`, never a signature.
    pub fn read(body: &[u8]) -> Decision {
        let Ok(Value::Object(o)) = serde_json::from_slice::<Value>(body) else {
            return Decision::Unknown;
        };
        let note = text(&o, "note");
        match o.get("outcome").and_then(Value::as_str) {
            Some("answered") => Decision::Answered { note },
            Some("read_again") => Decision::ReadAgain { note },
            Some("sign") => {
                let Some(Value::Object(a)) = o.get("ask") else {
                    return Decision::Unknown;
                };
                let kiff = match a.get("kiff") {
                    Some(Value::Object(k)) => match (text(k, "draft"), text(k, "version")) {
                        (Some(draft), Some(version)) => Some(AskDraft { draft, version }),
                        _ => return Decision::Unknown,
                    },
                    None | Some(Value::Null) => None,
                    Some(_) => return Decision::Unknown,
                };
                let (Some(id), Some(step), Some(message)) =
                    (text(a, "id"), text(a, "step"), text(a, "message"))
                else {
                    return Decision::Unknown;
                };
                Decision::Sign {
                    ask: Ask {
                        id,
                        step,
                        message,
                        reason: text(a, "reason").unwrap_or_default(),
                        kiff,
                    },
                    session: text(&o, "session"),
                }
            }
            _ => Decision::Unknown,
        }
    }
}

/// The Card step's message, as the iPad's `expectedApprovalMessage` builds it for `cards`.
pub fn cards_message(tenant: &str, draft: &str, version: &str) -> String {
    format!("tafs.cards.v1|{tenant}|{draft}|{version}")
}

/// Whether this computer signs `ask`: the Card's step only, for the draft the card names at the
/// version it drew, with exactly the message this computer builds. Anything else, the
/// connection's step above all, is never signed here.
pub fn signs_here(ask: &Ask, draft: &str, shown: &str, tenant: &str) -> bool {
    let Some(k) = &ask.kiff else {
        return false;
    };
    ask.step == "cards"
        && !tenant.is_empty()
        && k.draft == draft
        && k.version == shown
        && ask.message == cards_message(tenant, draft, shown)
}

/// Whether a card offers Approve here: an approval whose connection is already made, whose Card
/// comes next and is drawn from KIFF's draft at a version. Its connection's own approval stays the
/// phone's.
pub fn offered(connected: bool, card_next: bool, draft: Option<&str>, shown: Option<&str>) -> bool {
    connected
        && card_next
        && draft.is_some_and(|d| !d.is_empty())
        && shown.is_some_and(|v| !v.is_empty())
}

/// `POST /v1/tafs/{taf}/components/{component}/approval`.
pub fn ask_path(taf: &str, component: &str) -> String {
    format!(
        "/v1/tafs/{}/components/{}/approval",
        escaped(taf),
        escaped(component)
    )
}

/// Approve clicked: what the card showed, so tafs cloud compares it with what KIFF holds now.
pub fn ask_body(grant: &str, draft: &str, shown: &str, session: Option<&str>) -> Vec<u8> {
    let mut b = serde_json::json!({ "grant": grant, "draft": draft, "shown_draft": shown });
    if let Some(s) = session.filter(|s| !s.is_empty()) {
        b["session"] = Value::String(s.to_string());
    }
    serde_json::to_vec(&b).unwrap_or_default()
}

/// `POST /v1/tafs/{taf}/components/{component}/approval/{ask}`.
pub fn report_path(taf: &str, component: &str, ask: &str) -> String {
    format!("{}/{}", ask_path(taf, component), escaped(ask))
}

/// What came of the confirm: `confirmed`, or `failed` with why. Never an answer, only what
/// happened. Keys sorted, as the Mac's `JSONEncoder` sends them.
pub fn report_body(came: &Result<(), String>, session: Option<&str>) -> Vec<u8> {
    let mut b = Map::new();
    b.insert(
        "result".into(),
        Value::String(if came.is_ok() { "confirmed" } else { "failed" }.into()),
    );
    if let Some(s) = session.filter(|s| !s.is_empty()) {
        b.insert("session".into(), Value::String(s.to_string()));
    }
    if let Err(why) = came {
        let why: String = why.trim().chars().take(500).collect();
        if !why.is_empty() {
            b.insert("why".into(), Value::String(why));
        }
    }
    serde_json::to_vec(&Value::Object(b)).unwrap_or_default()
}

/// The report's `KIFF-Device-Signature`: the device key over `kiff.request.v1` for exactly this
/// request to tafs cloud (`t=<signed_at>,v1=<base64 DER ECDSA>`), as a setup call tafs cloud hands
/// KIFF to check. It reports; it grants nothing. Made only once the security key answered the
/// Card's confirm (`key_approval`).
pub fn report_signature(
    key: &DeviceKey,
    tenant: &str,
    path: &str,
    body: &[u8],
    signed_at: &str,
) -> String {
    let message = crate::passkey::request_v1(tenant, "POST", path, body, signed_at);
    format!("t={signed_at},v1={}", key.sign_b64(&message))
}

#[cfg(test)]
mod tests;
