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

/// Whether tafs cloud takes `method` on `path` only after the device has checked the person or
/// signed with its approval key (`useVersion`, `removeTaf`, a trial opened on a KIFF draft the
/// phone signed, a trial installed): a version switched to or made live, a trial opened or
/// installed, a taf removed for good. This computer never sends one, by path as well as by `Act`.
pub fn signed_at_cloud(method: &str, path: &str) -> bool {
    if path.contains("//") || path.contains("/./") || path.contains("/../") || path.ends_with("/..")
    {
        return true;
    }
    let path = path.split(['?', '#']).next().unwrap_or(path);
    let path = path.trim_end_matches('/');
    let p = parts(path);
    // `discardSettingUp`: signed by the approval key, as a removal is.
    if method == "POST"
        && path.starts_with("/v1/home/setting-up/")
        && p.len() == 6
        && p[5] == "discard"
    {
        return true;
    }
    if !path.starts_with("/v1/tafs/") || p.len() < 4 || p[3].is_empty() {
        return false;
    }
    matches!(
        (method, &p[4..]),
        ("DELETE", []) | ("POST", ["versions", _, "switch" | "live"])
    )
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
    /// `removePasskey`: one of the account's own passkeys removed, after its question.
    RemovePasskey { passkey: String },
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
    /// `resumeHalt`: a halted taf resumed, or every taf with no agent, confirmed by the device
    /// (`tafs.resume.v1`) and signed.
    Resume { agent: Option<String> },
    /// `confirmDraftAsTrial` and `confirmDraft` for a trial (`startTrying`, `installTrial`): a
    /// version's trial Card confirmed in KIFF as a trial (`trial`), or the same draft confirmed
    /// live to install it, exactly the draft version the owner read. KIFF's
    /// `POST /v1/me/agent-drafts/{id}/confirm`, one of its approval routes; on Linux the security
    /// key approves it. tafs cloud's own calls around it (`try`, `PUT …/trial`, `install`) go as
    /// the session: tafs cloud checks KIFF's draft before it opens or installs anything.
    ConfirmDraft {
        draft: String,
        version: String,
        trial: bool,
    },
    /// `goBackFromTrial`: the trial's KIFF draft ended with the setup signer, then the trial.
    EndTrial { draft: String },
    /// `discardSettingUp`: a taf being set up discarded, its Cards revoked and tafs cloud's
    /// `POST` signed by the approval key, as a removal is.
    DiscardSettingUp { agent: String },
    /// `answerYourStep`: Done or Skip it on a step a taf handed its owner.
    AnswerStep { handoff: String },
    /// `approveOnce`: a held call approved once, from the dial's face.
    ApproveOnce {
        call: String,
        /// The hold's `accounting_version`, which the signed answer names (`kiff.answer.v1`).
        accounting_version: Option<i64>,
    },
    /// `changeCard`: Extra room on a call over a total the Card keeps across calls, for this call
    /// until that total resets, signed as an answer (`change-card`).
    ExtraRoom {
        call: String,
        accounting_version: Option<i64>,
    },
    /// `dismissCardUnit`: a Card unit request dismissed, signed like a refusal (`reject`).
    DismissCardUnit { exception: String },
    /// `approveKnobEvent`: an app event's subscription at planque, signed by the device.
    ApproveEvent { grant: String, event: String },
    /// `share(agent:with:)`: a taf shared with a group or a person, from Share's second step.
    Share {
        agent: String,
        group: Option<String>,
        subject: Option<String>,
    },
    /// `unshare(agent:from:)`: a share taken away, from the same step.
    Unshare {
        agent: String,
        group: Option<String>,
        subject: Option<String>,
    },
    /// `DoorsModel.reopen`: a paused `api` door opened again. Pausing it is never signed.
    ReopenDoor { taf: String, door: String },
    /// `DoorsModel.newKey`: a new key for an `api` door, shown once.
    NewDoorKey { taf: String, door: String },
    /// `DoorsModel.revoke`: an `api` door's key revoked, after its question.
    RevokeDoorKey {
        taf: String,
        door: String,
        key: String,
    },
    /// `DoorsModel.removeAPI`: an `api` door removed, its keys with it.
    RemoveApiDoor { taf: String, door: String },
}

/// A query's value, escaped (`KIFFClient.escapedQuery`): letters, digits and `-._~` as they are.
fn escaped_query(v: &str) -> String {
    v.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

impl Act {
    /// The act as the iPad names it to Face ID, where the act alone says it: a new group's name,
    /// the zone, resuming every taf, the spend limit. Others are named where their names are
    /// known (`approval::refuse_as`).
    pub fn own_prompt(&self, w: &crate::words::WordBook) -> Option<String> {
        match self {
            Act::CreateGroup { name } => w.get("people.create_group", &[("name", name)]).ok(),
            Act::SetTimeZone { zone } => w.get("settings.time_zone.prompt", &[("zone", zone)]).ok(),
            Act::Resume { agent: None } => w.line("taf.halt.resume_all_prompt").ok(),
            Act::RaiseLimit => w.line("usage.raise_limit.prompt").ok(),
            _ => None,
        }
    }

    /// The whole request a security key would approve (method, path with its query, body), for
    /// the acts KIFF or tafs cloud take signed and whose body their fields hold, as the iPad's
    /// `KIFFClient` writes it. None for the rest: planque's (their own signing), a held call's or
    /// a step's answer (its signature is in the body), Make live (the device checks it first),
    /// and those whose body needs what the act doesn't carry yet (Raise the limit, Decline).
    pub fn request(&self) -> Option<(&'static str, String, Vec<u8>)> {
        let empty = || b"{}".to_vec();
        let body = match self {
            Act::CreateGroup { name } | Act::RenameGroup { name, .. } => {
                serde_json::to_vec(&serde_json::json!({ "name": name })).ok()?
            }
            Act::SetGroupMember { role, .. } => {
                serde_json::to_vec(&serde_json::json!({ "role": role })).ok()?
            }
            Act::SetTimeZone { zone } => {
                serde_json::to_vec(&serde_json::json!({ "time_zone": zone })).ok()?
            }
            // `Share`'s encoding: a nil group or subject is left out.
            Act::Share { group, subject, .. } => {
                let mut m = serde_json::Map::new();
                if let Some(g) = group {
                    m.insert("group".into(), g.clone().into());
                }
                if let Some(s) = subject {
                    m.insert("subject".into(), s.clone().into());
                }
                serde_json::to_vec(&m).ok()?
            }
            // An `api` door's, the iPad's exact bytes (`TafsCloud.reopenBody`; a DELETE's empty
            // body). New key stays on the phone: its answer is the key, shown once, which this
            // computer can't keep from a screen recording or a clipboard's history.
            Act::ReopenDoor { .. } => br#"{"open":true}"#.to_vec(),
            Act::RevokeDoorKey { .. } | Act::RemoveApiDoor { .. } => Vec::new(),
            // `preparePasskeyRemoval`: a DELETE with no body at all (`Data()`), not `{}`.
            Act::RemovePasskey { .. } => Vec::new(),
            Act::DeleteGroup { .. }
            | Act::RemoveGroupMember { .. }
            | Act::SignOutDevice { .. }
            | Act::Unshare { .. }
            | Act::Resume { .. }
            | Act::DiscardSettingUp { .. }
            | Act::RemoveTaf { .. }
            | Act::EndTrial { .. } => empty(),
            // `confirmDraftAsTrial`'s and `confirmDraft`'s bodies: the draft version read, and
            // `as: trial` to start trying it.
            Act::ConfirmDraft { version, trial, .. } => serde_json::to_vec(&if *trial {
                serde_json::json!({ "as": "trial", "version": version })
            } else {
                serde_json::json!({ "version": version })
            })
            .ok()?,
            _ => return None,
        };
        let (method, path) = self.route();
        Some((method, path, body))
    }

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
            Act::RemovePasskey { passkey } => (
                "DELETE",
                format!("{}/{}", crate::devices::PASSKEYS_PATH, escaped(passkey)),
            ),
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
                format!("grants/{}/triggers/{}", escaped(grant), escaped(trigger)),
            ),
            Act::ConnectAgain { grant } => ("POST", format!("grants/{}/approve", escaped(grant))),
            Act::RemoveTaf { taf } => ("DELETE", format!("/v1/tafs/{}", escaped(taf))),
            Act::Resume { agent } => (
                "POST",
                match agent {
                    Some(a) => format!("/v1/agents/{}/resume", escaped(a)),
                    None => "/v1/resume".into(),
                },
            ),
            Act::ConfirmDraft { draft, .. } => (
                "POST",
                format!("/v1/me/agent-drafts/{}/confirm", escaped(draft)),
            ),
            Act::EndTrial { draft } => (
                "DELETE",
                format!("/v1/me/agent-drafts/{}/trial", escaped(draft)),
            ),
            Act::DiscardSettingUp { agent } => (
                "POST",
                format!("/v1/home/setting-up/{}/discard", escaped(agent)),
            ),
            Act::ExtraRoom { call, .. } => (
                "POST",
                format!("/v1/me/card-exceptions/{}/change-card", escaped(call)),
            ),
            Act::ApproveOnce { call, .. } => (
                "POST",
                format!("/v1/me/card-exceptions/{}/approve-once", escaped(call)),
            ),
            Act::DismissCardUnit { exception } => (
                "POST",
                format!("/v1/me/card-exceptions/{}/reject", escaped(exception)),
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
            Act::Share { agent, .. } => ("POST", format!("/v1/agents/{}/grants", escaped(agent))),
            Act::Unshare {
                agent,
                group,
                subject,
            } => {
                let query = match (group, subject) {
                    (Some(g), _) => format!("group={}", escaped_query(g)),
                    (None, Some(s)) => format!("subject={}", escaped_query(s)),
                    (None, None) => String::new(),
                };
                (
                    "DELETE",
                    format!("/v1/agents/{}/grants?{query}", escaped(agent)),
                )
            }
            Act::ReopenDoor { taf, door } => ("PATCH", crate::doors::door_path(taf, door)),
            Act::NewDoorKey { taf, door } => ("POST", crate::doors::keys_path(taf, door)),
            Act::RevokeDoorKey { taf, door, key } => {
                ("DELETE", crate::doors::key_path(taf, door, key))
            }
            Act::RemoveApiDoor { taf, door } => ("DELETE", crate::doors::door_path(taf, door)),
        }
    }

    /// An act this computer never sends anything for, not even the unsigned call before its
    /// signature: planque's, and a connection's start.
    pub fn sends_nothing(&self) -> bool {
        self.at_planque()
            || self.at_device()
            || self.at_api_door()
            || matches!(self, Act::Connect { .. })
    }

    /// An `api` door's signed change (`DoorsModel`'s reopen, newKey, revoke, removeAPI). tafs
    /// cloud takes another door's open, keys and removal unsigned at the same paths, so the path
    /// can't tell: the doors screen refuses these before any call (`doors::api::only_signed`).
    pub fn at_api_door(&self) -> bool {
        self.api_door_matches()
    }

    /// A held call's answer that a security key signs in its body (`kiff.answer.v1`, KIFF's
    /// `checkPasskeyAnswer`): the hold, the answer and the accounting version it names. Approve
    /// once and Extra room (`change-card`) only, and only when the hold's accounting version was
    /// read. Never for a call read as a fill in the person's Chrome (`held::release_needed`): only
    /// the phone answers that one, with its release.
    pub fn answer(&self) -> Option<(&str, &'static str, i64)> {
        if let Act::ApproveOnce { call, .. } | Act::ExtraRoom { call, .. } = self {
            if crate::held::release_needed(call) {
                return None;
            }
        }
        match self {
            Act::ApproveOnce {
                call,
                accounting_version: Some(v),
            } => Some((call.as_str(), "approve-once", *v)),
            Act::ExtraRoom {
                call,
                accounting_version: Some(v),
            } => Some((call.as_str(), "change-card", *v)),
            _ => None,
        }
    }

    /// Whether the call goes to tafs cloud, not KIFF: what the device signs there (`at_device`),
    /// and an `api` door's signed changes.
    pub fn at_cloud(&self) -> bool {
        self.at_device() || self.at_api_door()
    }

    fn api_door_matches(&self) -> bool {
        matches!(
            self,
            Act::ReopenDoor { .. }
                | Act::NewDoorKey { .. }
                | Act::RevokeDoorKey { .. }
                | Act::RemoveApiDoor { .. }
        )
    }

    /// Whether the device checks the person, or signs with its approval key, before a tafs
    /// cloud call (`useVersion`, `removeTaf`): this computer never sends it.
    pub fn at_device(&self) -> bool {
        matches!(
            self,
            Act::MakeLive { .. } | Act::RemoveTaf { .. } | Act::DiscardSettingUp { .. }
        )
    }

    /// The connections an act ends at planque (Disconnect, Clear): what this computer may end with
    /// its device key once its security key checked the person, never on the device key alone. None
    /// for any other act, above all what grants access (connecting, Sign in again, keys, approvals).
    pub fn ends_connections(&self) -> Option<Vec<String>> {
        match self {
            Act::Disconnect { grant } if !grant.is_empty() => Some(vec![grant.clone()]),
            Act::ClearRemoved { grants }
                if !grants.is_empty() && grants.iter().all(|g| !g.is_empty()) =>
            {
                Some(grants.clone())
            }
            _ => None,
        }
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
            Act::RemovePasskey {
                passkey: "pk_1".into(),
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
            },
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
            Act::DiscardSettingUp {
                agent: "agt_1".into(),
            },
            Act::Resume {
                agent: Some("agt_1".into()),
            },
            Act::Resume { agent: None },
            Act::ConfirmDraft {
                draft: "drf_t".into(),
                version: "v3".into(),
                trial: true,
            },
            Act::ConfirmDraft {
                draft: "drf_t".into(),
                version: "v3".into(),
                trial: false,
            },
            Act::EndTrial {
                draft: "drf_t".into(),
            },
            Act::AnswerStep {
                handoff: "h1".into(),
            },
            Act::ApproveOnce {
                call: "ce1".into(),
                accounting_version: Some(3),
            },
            Act::ExtraRoom {
                call: "ce3".into(),
                accounting_version: Some(2),
            },
            Act::DismissCardUnit {
                exception: "ce2".into(),
            },
            Act::ApproveEvent {
                grant: "g1".into(),
                event: "NEW_TICKET".into(),
            },
            Act::Share {
                agent: "a1".into(),
                group: Some("g1".into()),
                subject: None,
            },
            Act::Unshare {
                agent: "a1".into(),
                group: None,
                subject: Some("auth0|rae".into()),
            },
            Act::ReopenDoor {
                taf: "t1".into(),
                door: "d1".into(),
            },
            Act::NewDoorKey {
                taf: "t1".into(),
                door: "d1".into(),
            },
            Act::RevokeDoorKey {
                taf: "t1".into(),
                door: "d1".into(),
                key: "k1".into(),
            },
            Act::RemoveApiDoor {
                taf: "t1".into(),
                door: "d1".into(),
            },
        ]
    }

    #[test]
    fn the_key_confirms_only_a_drafts_own_confirm() {
        // A trial's start and its install: KIFF's confirm, with exactly the version read.
        let start = Act::ConfirmDraft {
            draft: "drf 1".into(),
            version: "v3".into(),
            trial: true,
        };
        assert_eq!(
            start.request(),
            Some((
                "POST",
                "/v1/me/agent-drafts/drf%201/confirm".to_string(),
                br#"{"as":"trial","version":"v3"}"#.to_vec()
            ))
        );
        assert!(!start.at_cloud() && !start.at_planque() && start.answer().is_none());
        assert!(signed("POST", "/v1/me/agent-drafts/drf%201/confirm"));
        let install = Act::ConfirmDraft {
            draft: "drf_1".into(),
            version: "v3".into(),
            trial: false,
        };
        assert_eq!(
            install.request().map(|r| r.2),
            Some(br#"{"version":"v3"}"#.to_vec())
        );
        // No other act's request is a confirm or an approve, at KIFF or anywhere: planque's
        // approve and confirm have no request at all.
        for act in acts() {
            let Some((_, path, _)) = act.request() else {
                continue;
            };
            let last = path.split('?').next().unwrap_or(&path).rsplit('/').next();
            if matches!(last, Some("confirm" | "approve")) {
                assert!(
                    matches!(act, Act::ConfirmDraft { .. })
                        && path.starts_with("/v1/me/agent-drafts/")
                        && path.ends_with("/confirm"),
                    "{act:?}"
                );
            }
        }
        for act in acts().iter().filter(|a| a.at_planque()) {
            assert!(act.request().is_none(), "{act:?}");
        }
    }

    #[test]
    fn what_the_device_signs_at_tafs_cloud_is_never_sent() {
        for (m, p) in [
            ("DELETE", "/v1/tafs/t1"),
            ("DELETE", "/v1/tafs/t1/"),
            ("POST", "/v1/tafs/t1/versions/3/switch"),
            ("POST", "/v1/tafs/t1/versions/3/live?x=1"),
            ("POST", "/v1/tafs/t1//versions/3/live"),
            ("POST", "/v1/home/setting-up/agt_1/discard"),
        ] {
            assert!(signed_at_cloud(m, p), "{m} {p}");
        }
        // What the iPad sends unsigned: a door, a memory note, the trial's chat, going back, and
        // opening or installing a trial, which tafs cloud does only once KIFF's draft says so.
        for (m, p) in [
            ("POST", "/v1/tafs/t1/versions/3/try"),
            ("PUT", "/v1/tafs/t1/trial"),
            ("POST", "/v1/tafs/t1/trial/install"),
            ("DELETE", "/v1/tafs/t1/doors/d1"),
            ("DELETE", "/v1/tafs/t1/memory/n1"),
            ("POST", "/v1/tafs/t1/trial/messages"),
            ("DELETE", "/v1/tafs/t1/trial"),
            ("DELETE", "/v1/tafs/t1/switching"),
            ("POST", "/v1/home/coach-waiting/set-aside"),
        ] {
            assert!(!signed_at_cloud(m, p), "{m} {p}");
        }
        // Every act the device signs at tafs cloud is one of them.
        for act in acts().iter().filter(|a| a.at_device()) {
            let (m, p) = act.route();
            assert!(signed_at_cloud(m, &p), "{act:?}");
        }
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
                // The device signs or checks first; the app never sends it, by path too.
                assert!(signed_at_cloud(method, &path), "{act:?}");
                assert!(
                    matches!(method, "POST" | "PUT" | "DELETE"),
                    "{act:?} changes something"
                );
                continue;
            }
            if act.at_api_door() {
                // An `api` door's: never sent unsigned, whatever the path (`doors::api`); with the
                // security key, the iPad's exact request, at tafs cloud. New key only on the phone.
                assert!(path.starts_with("/v1/tafs/t1/doors/d1"), "{act:?}");
                assert!(act.at_cloud(), "{act:?}");
                match act {
                    Act::NewDoorKey { .. } => assert!(act.request().is_none()),
                    Act::ReopenDoor { .. } => assert_eq!(
                        act.request(),
                        Some(("PATCH", path.clone(), br#"{"open":true}"#.to_vec()))
                    ),
                    _ => assert_eq!(act.request(), Some((method, path.clone(), vec![]))),
                }
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

    #[test]
    fn a_keys_request_is_the_ipads_call_whole() {
        let (m, p, b) = Act::CreateGroup { name: "Ops".into() }.request().unwrap();
        assert_eq!(
            (m, p.as_str(), b.as_slice()),
            ("POST", "/v1/groups", &br#"{"name":"Ops"}"#[..])
        );
        let (_, p, b) = Act::SetTimeZone {
            zone: "Europe/Paris".into(),
        }
        .request()
        .unwrap();
        assert_eq!(
            (p.as_str(), b.as_slice()),
            ("/v1/me/time-zone", &br#"{"time_zone":"Europe/Paris"}"#[..])
        );
        let (m, p, b) = Act::Share {
            agent: "a1".into(),
            group: None,
            subject: Some("s1".into()),
        }
        .request()
        .unwrap();
        assert_eq!(
            (m, p.as_str(), b.as_slice()),
            ("POST", "/v1/agents/a1/grants", &br#"{"subject":"s1"}"#[..])
        );
        let (m, p, b) = Act::RemoveTaf { taf: "t 1".into() }.request().unwrap();
        assert_eq!(
            (m, p.as_str(), b.as_slice()),
            ("DELETE", "/v1/tafs/t%201", &b"{}"[..])
        );
        // Not this way: planque's, an answer signed in its body, Make live, Raise the limit.
        assert!(Act::Disconnect { grant: "g".into() }.request().is_none());
        assert!(Act::ApproveOnce {
            call: "c".into(),
            accounting_version: Some(1),
        }
        .request()
        .is_none());
        assert!(Act::MakeLive {
            taf: "t".into(),
            version: 2
        }
        .request()
        .is_none());
        assert!(Act::RaiseLimit.request().is_none());
        // Every request this way is one KIFF takes only signed, or tafs cloud checks the signature of.
        for act in [
            Act::CreateGroup { name: "x".into() },
            Act::SignOutDevice { device: "d".into() },
            Act::RemovePasskey {
                passkey: "p".into(),
            },
            Act::SetTimeZone { zone: "UTC".into() },
            Act::Resume { agent: None },
            Act::RemoveTaf { taf: "t".into() },
            Act::DiscardSettingUp { agent: "a".into() },
        ] {
            let (m, p, _) = act.request().unwrap();
            assert!(signed(m, &p) || signed_at_cloud(m, &p), "{m} {p}");
        }
    }

    #[test]
    fn only_disconnect_and_clear_end_connections() {
        assert_eq!(
            Act::Disconnect { grant: "g1".into() }.ends_connections(),
            Some(vec!["g1".to_string()])
        );
        assert_eq!(
            Act::ClearRemoved {
                grants: vec!["g1".into(), "g2".into()]
            }
            .ends_connections(),
            Some(vec!["g1".to_string(), "g2".to_string()])
        );
        // Nothing to end, or a blank id: none.
        assert_eq!(
            Act::Disconnect { grant: "".into() }.ends_connections(),
            None
        );
        assert_eq!(
            Act::ClearRemoved { grants: vec![] }.ends_connections(),
            None
        );
        assert_eq!(
            Act::ClearRemoved {
                grants: vec!["g1".into(), "".into()]
            }
            .ends_connections(),
            None
        );
        // Never what grants access: connecting, signing in again, keys, approvals.
        for act in acts() {
            if !matches!(act, Act::Disconnect { .. } | Act::ClearRemoved { .. }) {
                assert_eq!(act.ends_connections(), None, "{act:?}");
            }
        }
    }

    #[test]
    fn only_approve_once_with_its_accounting_version_is_an_answer() {
        let once = Act::ApproveOnce {
            call: "ce_1".into(),
            accounting_version: Some(4),
        };
        assert_eq!(once.answer(), Some(("ce_1", "approve-once", 4)));
        assert_eq!(
            once.route(),
            ("POST", "/v1/me/card-exceptions/ce_1/approve-once".into())
        );
        // Without the hold's version there's nothing to sign: the phone.
        let unread = Act::ApproveOnce {
            call: "ce_1".into(),
            accounting_version: None,
        };
        assert_eq!(unread.answer(), None);
        let room = Act::ExtraRoom {
            call: "ce_2".into(),
            accounting_version: Some(5),
        };
        assert_eq!(room.answer(), Some(("ce_2", "change-card", 5)));
        assert_eq!(
            room.route(),
            ("POST", "/v1/me/card-exceptions/ce_2/change-card".into())
        );
        for act in acts() {
            if !matches!(act, Act::ApproveOnce { .. } | Act::ExtraRoom { .. }) {
                assert_eq!(act.answer(), None, "{act:?}");
            }
        }
    }
}
