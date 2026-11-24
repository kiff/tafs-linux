//! Raise the limit and Decline on this computer: what KIFF takes signed by the device key
//! (`KIFF-Device-Signature`, `PhoneSignedRules`) but never as a passkey's approval. A device key on a
//! computer's disk is no proof that a person is there, so here it signs only after this computer's
//! security key checked the person, by touch and PIN, for that very request:
//!
//! 1. `Request::new`: the act's own request (`Act::request`, only a `device_signed` act's), and its
//!    `kiff.request.v1` message, which KIFF checks the device key's signature against.
//! 2. The security key answers over `Request::client_data`, a `tafs.presence.request.v1` message
//!    naming that request's hash. It's never sent: its first line keeps it from ever being a
//!    message KIFF would take as an approval.
//! 3. `Request::presence`: the answer checked as KIFF checks an assertion (kiff.dev's, present and
//!    verified, the credential asked, not a synced passkey) and by the registered key's own
//!    signature. Only this makes a `Presence`; nothing else can.
//! 4. `Request::sign`: the device key signs, taking the request and its `Presence` both, and only
//!    when the presence names this request. Neither can be copied or kept for another.
//!
//! So no device-signed request is ever built without a fresh check of the person for it.

use sha2::{Digest, Sha256};

use crate::keys::DeviceKey;
use crate::passkey::{self, Assertion, Refusal};
use crate::signed::Act;

/// The header KIFF reads the device key's signature from: `t=<signed_at>,v1=<base64 DER ECDSA>`.
pub const HEADER: &str = "KIFF-Device-Signature";

/// One device-signed act's request, waiting for the person. Not `Clone`: one check, one request.
#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    method: &'static str,
    path: String,
    body: Vec<u8>,
    signed_at: String,
    /// `kiff.request.v1` over the above.
    message: Vec<u8>,
    /// `tafs.presence.request.v1` over the message's hash: what the security key answers.
    presence: Vec<u8>,
}

/// The security key's answer, checked, for one request's message. Only `Request::presence` makes
/// one; `Request::sign` takes it. Not `Clone`.
///
/// ```compile_fail
/// let p = tafs_core::device_signed::Presence { message: vec![] };
/// ```
///
/// ```compile_fail
/// fn again(p: &tafs_core::device_signed::Presence) -> tafs_core::device_signed::Presence {
///     p.clone()
/// }
/// ```
#[derive(Debug, PartialEq, Eq)]
pub struct Presence {
    message: Vec<u8>,
}

/// What goes to KIFF: the act's own request and the device key's header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signed {
    pub method: &'static str,
    pub path: String,
    pub body: Vec<u8>,
    pub header: String,
}

/// Why the security key's answer doesn't count for the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotPresent {
    /// What KIFF would refuse in an assertion (`passkey::refusal`).
    Refused(Refusal),
    /// Not the registered key's signature over this request's presence.
    NotThisKey,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// `tafs.presence.request.v1`: the tenant, `signed_at`, and the request message's SHA-256 in
/// lowercase hex, one per line.
pub fn presence_message(tenant: &str, signed_at: &str, request: &[u8]) -> Vec<u8> {
    [
        "tafs.presence.request.v1",
        tenant,
        signed_at,
        &hex(&Sha256::digest(request)),
    ]
    .join("\n")
    .into_bytes()
}

impl Request {
    /// The act's request, signed at `signed_at` for `tenant`; none for an act that isn't
    /// device-signed, or with no tenant.
    pub fn new(tenant: &str, act: &Act, signed_at: &str) -> Option<Request> {
        if !act.device_signed() || tenant.is_empty() {
            return None;
        }
        let (method, path, body) = act.request()?;
        let message = passkey::request_v1(tenant, method, &path, &body, signed_at);
        let presence = presence_message(tenant, signed_at, &message);
        Some(Request {
            method,
            path,
            body,
            signed_at: signed_at.to_string(),
            message,
            presence,
        })
    }

    /// The client data the security key signs over (its challenge, the presence message's hash).
    pub fn client_data(&self) -> Vec<u8> {
        passkey::client_data_json(&self.presence)
    }

    /// The key's answer, checked: `asked` is the credential asked for, `public_key` its SPKI DER,
    /// registered with this computer when it was paired.
    pub fn presence(
        &self,
        asked: &[u8],
        public_key: &[u8],
        answer: &Assertion,
    ) -> Result<Presence, NotPresent> {
        if let Some(r) = passkey::refusal(&answer.authenticator_data, asked, &answer.credential_id)
        {
            return Err(NotPresent::Refused(r));
        }
        if !passkey::signature_holds(
            public_key,
            &answer.authenticator_data,
            &self.client_data(),
            &answer.signature,
        ) {
            return Err(NotPresent::NotThisKey);
        }
        Ok(Presence {
            message: self.message.clone(),
        })
    }

    /// The device key's signature, once: none when `presence` was another request's.
    pub fn sign(self, presence: Presence, key: &DeviceKey) -> Option<Signed> {
        if presence.message != self.message {
            return None;
        }
        Some(Signed {
            header: format!("t={},v1={}", self.signed_at, key.sign_b64(&self.message)),
            method: self.method,
            path: self.path,
            body: self.body,
        })
    }
}

/// The line when KIFF didn't take a device-signed act (`raiseLimit`'s and `declineRequest`'s
/// catches): for a raise, KIFF's 409s in their own words (the limit changed meanwhile, or is as
/// high as the app may raise it), else that the limit wasn't raised, with why; for a decline, that
/// it wasn't declined, with why.
pub fn not_taken(
    w: &crate::words::WordBook,
    act: &Act,
    answer: &crate::signin::Answer,
) -> Result<String, crate::words::WordsError> {
    let why = crate::signin::refusal(answer, Some(w)).words(w)?;
    match act {
        Act::RaiseLimit { .. } => {
            let code = serde_json::from_slice::<serde_json::Value>(&answer.body)
                .ok()
                .and_then(|v| v.get("error")?.as_str().map(str::to_string));
            match (answer.status, code.as_deref()) {
                (409, Some("limit_changed")) => w.line("usage.limit_changed"),
                (409, Some("limit_ceiling")) => w.line("usage.limit_ceiling"),
                _ => w.get("usage.raise_limit.not_raised", &[("why", &why)]),
            }
        }
        _ => w.get("billing.request.not_declined", &[("why", &why)]),
    }
}

/// `not_taken` when KIFF wasn't reached at all.
pub fn not_sent(
    w: &crate::words::WordBook,
    act: &Act,
    e: &crate::signin::KiffError,
) -> Result<String, crate::words::WordsError> {
    let why = e.words(w)?;
    match act {
        Act::RaiseLimit { .. } => w.get("usage.raise_limit.not_raised", &[("why", &why)]),
        _ => w.get("billing.request.not_declined", &[("why", &why)]),
    }
}

#[cfg(test)]
mod tests;
