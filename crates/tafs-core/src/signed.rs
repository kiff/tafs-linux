//! An act the iPad signs, on a computer that only views. This computer never signs: for each act
//! the iPad takes behind Face ID, it draws the iPad's button, and its sheet where the iPad opens
//! one first, up to the signature. The tap that would sign says that this computer can't approve
//! (`ApprovalGate`'s words) and sends nothing.
//!
//! Which calls KIFF takes only signed is `KIFFClient.signsSetup`, `grantsAuthority` and the
//! presigned ones (a device's removal, a pairing, a passkey). A call this computer makes as the
//! session is an [`Unsigned`], which can't be made for any of them.

use crate::approval::{self, NotReady, THIS_COMPUTER};
use crate::sidebar::Mode;
use crate::words::{WordBook, WordsError};

/// Opening an approval window, and closing it (`KIFFClient.windowPath`).
pub const WINDOW_PATH: &str = "/v1/me/window";

fn parts(path: &str) -> Vec<&str> {
    path.split('/').collect()
}

/// `answersHappened`: the owner's answer to "Did this happen?".
fn answers_happened(path: &str) -> bool {
    let p = parts(path);
    p.len() == 5 && path.starts_with("/v1/me/did-this-happen/") && !p[4].is_empty()
}

/// `KIFFClient.signsSetup`: whether KIFF takes `path` from a phone only with its signature.
pub fn signs_setup(path: &str) -> bool {
    const SETUP: [&str; 6] = [
        "/v1/me/agent-drafts",
        "/v1/me/alert-targets",
        "/v1/keys",
        "/v1/me/time-zone",
        "/v1/me/delete",
        "/v1/me/usage/limit",
    ];
    if SETUP.contains(&path) {
        return true;
    }
    if path == WINDOW_PATH || (path.starts_with("/v1/me/window/") && path.ends_with("/close")) {
        return true;
    }
    let p = parts(path);
    // Groups and sharing: each change is the person's act behind Face ID.
    if path == "/v1/groups" {
        return true;
    }
    if path.starts_with("/v1/groups/")
        && (p.len() == 4 || (p.len() == 6 && p[4] == "members"))
        && !p[3].is_empty()
    {
        return true;
    }
    let bare = path.split('?').next().unwrap_or(path);
    let b = parts(bare);
    if b.len() == 5 && bare.starts_with("/v1/agents/") && b[4] == "grants" && !b[3].is_empty() {
        return true;
    }
    if p.len() == 6
        && path.starts_with("/v1/me/pack-requests/")
        && p[5] == "decline"
        && !p[4].is_empty()
    {
        return true;
    }
    if answers_happened(path) {
        return true;
    }
    if path == "/v1/resume"
        || (p.len() == 5 && path.starts_with("/v1/agents/") && p[4] == "resume" && !p[3].is_empty())
    {
        return true;
    }
    p.len() == 6
        && path.starts_with("/v1/me/agent-drafts/")
        && (p[5] == "confirm" || p[5] == "trial")
        && !p[4].is_empty()
}

/// `KIFFClient.grantsAuthority`: whether the call grants authority, signed by the approval key.
pub fn grants_authority(method: &str, path: &str) -> bool {
    if method != "POST" {
        return false;
    }
    if path == WINDOW_PATH || path == "/v1/me/agent-drafts" || answers_happened(path) {
        return true;
    }
    let p = parts(path);
    if p.len() == 5 && path.starts_with("/v1/agents/") && p[4] == "grants" && !p[3].is_empty() {
        return true;
    }
    if path == "/v1/resume"
        || (p.len() == 5 && path.starts_with("/v1/agents/") && p[4] == "resume" && !p[3].is_empty())
    {
        return true;
    }
    p.len() == 6
        && path.starts_with("/v1/me/agent-drafts/")
        && p[5] == "confirm"
        && !p[4].is_empty()
}

/// `answerHandoff`: a step a taf handed its owner, answered with the device key's signature in its
/// body (`handoffMessage`).
fn answers_handoff(method: &str, path: &str) -> bool {
    let p = parts(path);
    method == "POST"
        && p.len() == 6
        && path.starts_with("/v1/me/handoffs/")
        && p[5] == "answer"
        && !p[4].is_empty()
}

/// `approveOnce`, `reject`, `changeCard`: a held call answered, with the device key's signature
/// in its body (`AnswerSignature`).
fn answers_hold(method: &str, path: &str) -> bool {
    let p = parts(path);
    method == "POST"
        && p.len() == 6
        && path.starts_with("/v1/me/card-exceptions/")
        && matches!(p[5], "approve-once" | "reject" | "change-card")
        && !p[4].is_empty()
}

/// Whether KIFF takes `method` on `path` only signed: a signed setup call, one that grants
/// authority, the session's approval key and its relink, or one the iPad builds and signs
/// beforehand (a device signed out, a computer's pairing, a passkey added or removed, a step's
/// or a held call's answer).
pub fn signed(method: &str, path: &str) -> bool {
    // A path that isn't plain (`//`, `/./`, `/../`) is taken as signed: nothing here builds one,
    // and none may slip past the lists below by its spelling.
    if path.contains("//") || path.contains("/./") || path.contains("/../") || path.ends_with("/..")
    {
        return true;
    }
    // The route itself, as KIFF matches it: no query, no fragment, no trailing `/`.
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let path = path.trim_end_matches('/');
    signs_setup(path)
        || grants_authority(method, path)
        || path == "/v1/me/device/approval-key"
        || path == "/v1/me/device/relink"
        || (method == "DELETE" && path.starts_with("/v1/me/devices/"))
        || (method == "POST" && path == "/v1/me/pairings")
        || (method != "GET" && path.starts_with("/v1/me/passkeys"))
        || answers_handoff(method, path)
        || answers_hold(method, path)
}

/// A path part, escaped (`KIFFClient.escaped`).
fn escaped(v: &str) -> String {
    crate::access::escaped(v)
}

/// `KIFFClient.revoke(cardID:)`'s path: a Card revoked as the session, unsigned (it only takes
/// authority away).
pub fn revoke_path(card: &str) -> String {
    format!("/v1/me/mandates/{}/revoke", escaped(card))
}

/// An act the iPad signs, which this computer draws up to its signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Act {
    /// `createGroup`: + Group's sheet, Create.
    CreateGroup { name: String },
    /// `renameGroup`: Rename's sheet, Save.
    RenameGroup { group: String, name: String },
    /// `deleteGroup`: Delete, after its question.
    DeleteGroup { group: String },
    /// `setGroupMember`: someone added, or their role changed.
    SetGroupMember {
        group: String,
        subject: String,
        role: String,
    },
    /// `removeGroupMember`.
    RemoveGroupMember { group: String, subject: String },
    /// `removeDevice`: another device signed out, after its question.
    SignOutDevice { device: String },
    /// `setAccountTimeZone`: a zone chosen, or this computer's.
    SetTimeZone { zone: String },
    /// `raiseLimit`: the spend limit raised one step.
    RaiseLimit,
    /// `declineRequest`: a member's request declined, with a note.
    DeclineRequest { request: String },
    /// `disconnect(grantID:)`: a connection's access removed, or its waiting request or sign-in
    /// cancelled, after its question (Connect apps). planque's call, behind Face ID.
    Disconnect { grant: String },
    /// `clearRemovedConnections`: a removed taf's connections cleared, one Face ID for all,
    /// then the device key signs each disconnect.
    ClearRemoved { grants: Vec<String> },
    /// `reconnect`: a connection signed in again, approved behind Face ID first.
    SignInAgain { grant: String },
    /// Connect apps' Connect, once "Which taf?" is answered: the iPad starts the connection
    /// (`startConnect`) only to approve it behind Face ID on its sheet. Started here, it would
    /// leave a request at planque this computer can never approve, so nothing is sent at all.
    Connect { entry: String, taf: String },
    /// `removeSlackDoor`: a Slack door's subscription stopped at planque, signed by the device,
    /// before the door goes.
    StopSlackSubscription { grant: String, trigger: String },
    /// `connectAgain`, `reconnect`: a connection signed in again, its approval at planque signed
    /// by the device before the vendor's page.
    ConnectAgain { grant: String },
    /// `useVersion`: a version made live, behind the device's own check before tafs cloud's
    /// switch begins (and the Card's confirmation when it brings one). The switch stops the taf
    /// until it's finished, so this computer doesn't begin it.
    MakeLive { taf: String, version: i64 },
    /// `removeTaf`: Remove for good, the `DELETE` to tafs cloud signed by the approval key.
    RemoveTaf { taf: String },
    /// `answerYourStep`: Done or Skip it on a step a taf handed its owner.
    AnswerStep { handoff: String },
    /// `approveOnce`: a held call approved once, from the dial's face.
    ApproveOnce { call: String },
    /// `approveKnobEvent`: an app event's subscription at planque, signed by the device.
    ApproveEvent { grant: String, event: String },
}

impl Act {
    /// The call the iPad makes once it has signed (`KIFFClient`): this computer never makes it.
    pub fn route(&self) -> (&'static str, String) {
        match self {
            Act::CreateGroup { .. } => ("POST", "/v1/groups".into()),
            Act::RenameGroup { group, .. } => ("PATCH", format!("/v1/groups/{}", escaped(group))),
            Act::DeleteGroup { group } => ("DELETE", format!("/v1/groups/{}", escaped(group))),
            Act::SetGroupMember { group, subject, .. } => (
                "PUT",
                format!("/v1/groups/{}/members/{}", escaped(group), escaped(subject)),
            ),
            Act::RemoveGroupMember { group, subject } => (
                "DELETE",
                format!("/v1/groups/{}/members/{}", escaped(group), escaped(subject)),
            ),
            Act::SignOutDevice { device } => {
                ("DELETE", format!("/v1/me/devices/{}", escaped(device)))
            }
            Act::SetTimeZone { .. } => ("PUT", "/v1/me/time-zone".into()),
            Act::RaiseLimit => ("PUT", "/v1/me/usage/limit".into()),
            Act::DeclineRequest { request } => (
                "POST",
                format!("/v1/me/pack-requests/{}/decline", escaped(request)),
            ),
            // planque's, under the account's path (`PlanqueClient.raw`): each removed one is
            // its own DELETE, the first named here.
            Act::Disconnect { grant } => ("DELETE", format!("grants/{}", escaped(grant))),
            Act::ClearRemoved { grants } => (
                "DELETE",
                format!(
                    "grants/{}",
                    escaped(grants.first().map(String::as_str).unwrap_or_default())
                ),
            ),
            Act::SignInAgain { grant } => ("POST", format!("grants/{}/approve", escaped(grant))),
            // tafs cloud's start, then planque's approval: this computer makes neither.
            Act::Connect { entry, .. } => ("POST", crate::connect::start_path(entry)),
            Act::StopSlackSubscription { grant, trigger } => (
                "DELETE",
                format!(
                    "grants/{}/triggers/{}",
                    escaped(grant),
                    escaped(trigger)
                ),
            ),
            Act::ConnectAgain { grant } => {
                ("POST", format!("grants/{}/approve", escaped(grant)))
            }
            Act::RemoveTaf { taf } => ("DELETE", format!("/v1/tafs/{}", escaped(taf))),
            Act::ApproveOnce { call } => (
                "POST",
                format!("/v1/me/card-exceptions/{}/approve-once", escaped(call)),
            ),
            Act::ApproveEvent { grant, .. } => {
                ("POST", format!("grants/{}/triggers", escaped(grant)))
            }
            Act::AnswerStep { handoff } => (
                "POST",
                format!("/v1/me/handoffs/{}/answer", escaped(handoff)),
            ),
            Act::MakeLive { taf, version } => (
                "POST",
                format!("/v1/tafs/{}/versions/{version}/switch", escaped(taf)),
            ),
        }
    }

    /// An act this computer never sends anything for, not even the unsigned call before its
    /// signature: planque's, and a connection's start.
    pub fn sends_nothing(&self) -> bool {
        self.at_planque() || self.at_device() || matches!(self, Act::Connect { .. })
    }

    /// Whether the device checks the person, or signs with its approval key, before a tafs
    /// cloud call (`useVersion`, `removeTaf`): this computer never sends it.
    pub fn at_device(&self) -> bool {
        matches!(self, Act::MakeLive { .. } | Act::RemoveTaf { .. })
    }

    /// The call goes to planque, which this computer never reaches (`session::may_send_to`).
    pub fn at_planque(&self) -> bool {
        matches!(
            self,
            Act::Disconnect { .. }
                | Act::ClearRemoved { .. }
                | Act::SignInAgain { .. }
                | Act::StopSlackSubscription { .. }
                | Act::ConnectAgain { .. }
                | Act::ApproveEvent { .. }
        )
    }
}

/// The tap that would sign `act`: what this computer says instead, and it sends nothing. Signed in
/// with KIFF, that it can't approve here (`ApprovalNotReady`'s words); in the sample, which the
/// iPad never signs for (`guard mode == .kiff`), nothing.
pub fn tap(w: &WordBook, mode: Mode, act: &Act) -> Result<Option<NotReady>, WordsError> {
    let _ = act;
    approval::notice(w, mode, THIS_COMPUTER)
}

/// A call this computer may make as the session: never one KIFF takes only signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsigned {
    method: &'static str,
    path: String,
    body: Vec<u8>,
}

impl Unsigned {
    /// None for a call KIFF takes only signed (`signed`).
    pub fn new(method: &'static str, path: String, body: Vec<u8>) -> Option<Unsigned> {
        (!signed(method, &path)).then_some(Unsigned { method, path, body })
    }

    pub fn method(&self) -> &'static str {
        self.method
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn body(&self) -> &[u8] {
        &self.body
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(lang: &str) -> WordBook {
        WordBook::load(&WordBook::default_root(), lang).unwrap()
    }

    fn acts() -> Vec<Act> {
        vec![
            Act::CreateGroup {
                name: "Finance".into(),
            },
            Act::RenameGroup {
                group: "g1".into(),
                name: "Money".into(),
            },
            Act::DeleteGroup { group: "g1".into() },
            Act::SetGroupMember {
                group: "g1".into(),
                subject: "auth0|rae".into(),
                role: "manager".into(),
            },
            Act::RemoveGroupMember {
                group: "g1".into(),
                subject: "auth0|rae".into(),
            },
            Act::SignOutDevice {
                device: "dev_2".into(),
            },
            Act::SetTimeZone {
                zone: "Europe/Paris".into(),
            },
            Act::RaiseLimit,
            Act::DeclineRequest {
                request: "req_1".into(),
            },
            Act::Disconnect {
                grant: "g_1".into(),
            },
            Act::ClearRemoved {
                grants: vec!["g_1".into(), "g_2".into()],
            },
            Act::SignInAgain {
                grant: "g_1".into(),
            },
            Act::Connect {
                entry: "qonto@2".into(),
                taf: "taf-1".into(),
            Act::StopSlackSubscription {
                grant: "g1".into(),
                trigger: "tr_1".into(),
            },
            Act::ConnectAgain { grant: "g1".into() },
            Act::MakeLive {
                taf: "t1".into(),
                version: 8,
            },
            Act::RemoveTaf { taf: "t1".into() },
            Act::AnswerStep {
                handoff: "h1".into(),
            },
            Act::ApproveOnce { call: "ce1".into() },
            Act::ApproveEvent {
                grant: "g1".into(),
                event: "NEW_TICKET".into(),
            },
        ]
    }

    #[test]
    fn no_signed_act_makes_a_request() {
        for act in acts() {
            let (method, path) = act.route();
            if act.at_planque() {
                // planque's: this computer sends nothing there, signed or not.
                let url = format!("{}/v1/accounts/t1/{path}", crate::planque::PLANQUE);
                assert!(!crate::session::may_send_to(&url), "{act:?}");
                continue;
            }
            if act.at_device() {
                // The device signs or checks first; the app never sends it.
                assert!(path.starts_with("/v1/tafs/t1"), "{act:?}");
                assert!(
                    matches!(method, "POST" | "DELETE"),
                    "{act:?} changes something"
                );
                continue;
            }
            if act.sends_nothing() {
                // A connection's start: never sent from here, though tafs cloud would take it
                // unsigned (the app has no call that makes it: `connections.rs`' test).
                assert_eq!(
                    (method, path.as_str()),
                    ("POST", "/v1/connect/apps/qonto@2")
                );
                continue;
            }
            assert!(
                signed(method, &path),
                "{act:?} is signed at {method} {path}"
            );
            assert!(
                Unsigned::new(method, path.clone(), b"{}".to_vec()).is_none(),
                "{act:?} can't be sent unsigned"
            );
        }
    }

    #[test]
    fn the_tap_that_would_sign_says_this_computer_cant_approve() {
        for lang in crate::words::SHIPPED {
            let w = words(lang);
            let line = approval::not_ready(&w, THIS_COMPUTER, None).unwrap();
            for act in acts() {
                assert_eq!(tap(&w, Mode::Kiff, &act).unwrap(), Some(line.clone()));
                // The sample signs nothing, so its tap does nothing.
                assert_eq!(tap(&w, Mode::Sample, &act).unwrap(), None);
                assert_eq!(tap(&w, Mode::SignedOut, &act).unwrap(), None);
            }
        }
        assert_eq!(
            tap(&words("en"), Mode::Kiff, &Act::RaiseLimit)
                .unwrap()
                .unwrap()
                .line,
            "This device can't approve. Approve on another device signed in to your account."
        );
    }

    #[test]
    fn revoking_a_card_is_the_sessions() {
        // `KIFFClient.revoke(cardID:)`: no signature, as the iPad sends it.
        let path = revoke_path("card 1");
        assert_eq!(path, "/v1/me/mandates/card%201/revoke");
        assert!(Unsigned::new("POST", path, b"{}".to_vec()).is_some());
    }

    #[test]
    fn signed_calls_as_the_ipad_lists_them() {
        for path in [
            "/v1/me/agent-drafts",
            "/v1/me/alert-targets",
            "/v1/keys",
            "/v1/me/time-zone",
            "/v1/me/delete",
            "/v1/me/usage/limit",
            "/v1/me/window",
            "/v1/me/window/w1/close",
            "/v1/groups",
            "/v1/groups/g1",
            "/v1/groups/g1/members/s1",
            "/v1/agents/a1/grants",
            "/v1/agents/a1/grants?group=g1",
            "/v1/me/pack-requests/r1/decline",
            "/v1/me/did-this-happen/p1",
            "/v1/resume",
            "/v1/agents/a1/resume",
            "/v1/me/agent-drafts/d1/confirm",
            "/v1/me/agent-drafts/d1/trial",
        ] {
            assert!(signs_setup(path), "{path}");
        }
        for path in [
            "/v1/groups/",
            "/v1/groups/g1/members",
            "/v1/me/pack-requests",
            "/v1/me/pack-requests/r1/withdraw",
            "/v1/me/usage",
            "/v1/me/devices",
            "/v1/me/agent-drafts/d1",
            "/v1/agents//grants",
        ] {
            assert!(!signs_setup(path), "{path}");
        }
        assert!(grants_authority("POST", "/v1/agents/a1/grants"));
        assert!(!grants_authority("DELETE", "/v1/agents/a1/grants"));
        assert!(!grants_authority("POST", "/v1/me/agent-drafts/d1/trial"));
        assert!(grants_authority("POST", "/v1/me/agent-drafts/d1/confirm"));
        // A device's removal, a pairing and a passkey are signed beforehand; reading them isn't.
        assert!(signed("DELETE", "/v1/me/devices/dev_2"));
        assert!(!signed("GET", "/v1/me/devices"));
        assert!(signed("POST", "/v1/me/pairings"));
        assert!(signed("POST", "/v1/me/passkeys"));
        assert!(signed("DELETE", "/v1/me/passkeys/pk1"));
        assert!(!signed("GET", "/v1/me/passkeys"));
        assert!(signed("POST", "/v1/me/device/approval-key"));
        // A step's answer carries the device's signature; reading the step doesn't.
        assert!(signed("POST", "/v1/me/handoffs/h1/answer"));
        assert!(!signed("GET", "/v1/me/handoffs/h1"));
        // A held call's answers carry it too; reading the call doesn't.
        for answer in ["approve-once", "reject", "change-card"] {
            assert!(signed(
                "POST",
                &format!("/v1/me/card-exceptions/ce1/{answer}")
            ));
        }
        assert!(!signed("GET", "/v1/me/card-exceptions/ce1"));
    }

    #[test]
    fn an_unsigned_call_is_one_kiff_takes_as_the_session() {
        let ask = Unsigned::new("POST", "/v1/me/pack-requests".into(), b"{}".to_vec()).unwrap();
        assert_eq!(
            (ask.method(), ask.path(), ask.body()),
            ("POST", "/v1/me/pack-requests", &b"{}"[..])
        );
        assert!(Unsigned::new("POST", "/v1/me/pack-requests/r1/withdraw".into(), vec![]).is_some());
        assert!(Unsigned::new("PUT", "/v1/me/time-zone".into(), vec![]).is_none());
    }

    #[test]
    fn a_signed_route_is_signed_however_it_is_spelled() {
        for (method, path) in [
            ("PUT", "/v1/me/time-zone?x=1"),
            ("PUT", "/v1/me/usage/limit/"),
            ("POST", "/v1/me/pack-requests/r1/decline?a"),
            ("POST", "/v1/me/agent-drafts/d1/confirm?x"),
            ("POST", "/v1/resume?"),
            ("PUT", "/v1/me/time-zone#x"),
            ("PUT", "/v1/me/time-zone//"),
            ("POST", "/v1/me/pack-requests/./r1/decline"),
            ("POST", "/v1/me/pack-requests/x/../r1/decline"),
            ("GET", "/v1/me//devices"),
        ] {
            assert!(signed(method, path), "{method} {path}");
            assert!(
                Unsigned::new(method, path.to_string(), vec![]).is_none(),
                "{method} {path}"
            );
        }
        // An unsigned route stays unsigned with a query or a trailing `/`.
        for path in ["/v1/me/pack-requests?x=1", "/v1/me/pack-requests/"] {
            assert!(!signed("POST", path), "{path}");
            assert!(
                Unsigned::new("POST", path.to_string(), vec![]).is_some(),
                "{path}"
            );
        }
    }

    #[test]
    fn a_part_is_escaped_in_the_route() {
        let (_, path) = Act::RemoveGroupMember {
            group: "g/1".into(),
            subject: "auth0|rae".into(),
        }
        .route();
        assert_eq!(path, "/v1/groups/g%2F1/members/auth0%7Crae");
        assert!(signed("DELETE", &path));
    }
}
