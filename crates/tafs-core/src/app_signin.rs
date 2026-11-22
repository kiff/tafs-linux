//! Signing in inside the app (K5, kiff/cloud `api/internal/appsignin/README.md`; the iPad's and
//! Android's `AppSignIn`): KIFF's `/oauth/authorize` with PKCE in the person's browser, the way
//! back at `dev.kiff.tafs:/oauth`, then `POST /v1/auth/app/token` with the terms version the
//! welcome's tick agreed to, for today's `DeviceToken`.
//!
//! On Linux the browser is the person's own, with its cookies: there's no private sheet. KIFF
//! never issues a code without the person acting on its page for this request, which is the guard
//! there (kiff/cloud#23), as on Android with a browser that has no private tabs.

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand_core::RngCore;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::signin::{refusal, Answer, DeviceToken, KiffError, Transport};
use crate::words::WordBook;

/// The one client every tafs app is (first party: no consent page).
pub const CLIENT_ID: &str = "tafs-app";
/// The one way back KIFF accepts, exactly: a private-use scheme (RFC 8252 §7.1).
pub const REDIRECT_URI: &str = "dev.kiff.tafs:/oauth";
/// KIFF's authorize page.
pub const AUTHORIZE: &str = "https://api.kiff.dev/oauth/authorize";
/// The languages KIFF's sign-in page speaks (`ui_locales`); any other is left out.
pub const PAGE_LANGUAGES: [&str; 8] = ["en", "de", "fr", "es", "it", "pt", "nl", "pl"];

/// One attempt: its verifier and `state`, made fresh for each Continue. Neither is ever printed.
#[derive(Clone, PartialEq, Eq)]
pub struct Attempt {
    verifier: String,
    state: String,
}

impl std::fmt::Debug for Attempt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Attempt(…)")
    }
}

/// 32 random bytes, base64url without padding: 43 characters.
fn random_43() -> String {
    let mut b = [0u8; 32];
    rand_core::OsRng.fill_bytes(&mut b);
    URL_SAFE_NO_PAD.encode(b)
}

impl Attempt {
    /// A fresh verifier (43 characters, RFC 7636's least) and a fresh `state` (256 bits).
    pub fn new() -> Attempt {
        Attempt {
            verifier: random_43(),
            state: random_43(),
        }
    }

    /// For tests: a known verifier and state.
    pub fn with(verifier: &str, state: &str) -> Attempt {
        Attempt {
            verifier: verifier.into(),
            state: state.into(),
        }
    }

    /// S256: base64url, no padding, of the verifier's SHA-256.
    pub fn challenge(&self) -> String {
        URL_SAFE_NO_PAD.encode(Sha256::digest(self.verifier.as_bytes()))
    }

    /// KIFF's authorize link for this attempt, in the app's language where KIFF's page speaks it.
    pub fn authorize_link(&self, language: &str, login_hint: Option<&str>) -> String {
        let mut url = url::Url::parse(AUTHORIZE).expect("KIFF's authorize link");
        {
            let mut q = url.query_pairs_mut();
            q.append_pair("response_type", "code")
                .append_pair("client_id", CLIENT_ID)
                .append_pair("redirect_uri", REDIRECT_URI)
                .append_pair("code_challenge", &self.challenge())
                .append_pair("code_challenge_method", "S256")
                .append_pair("state", &self.state);
            let code = crate::language::kiff_code(language);
            if PAGE_LANGUAGES.contains(&code) {
                q.append_pair("ui_locales", code);
            }
            if let Some(hint) = login_hint.filter(|h| !h.is_empty()) {
                q.append_pair("login_hint", hint);
            }
        }
        url.into()
    }

    /// What a link back says, for this attempt only (`AppSignIn.answer`). It's ours only when it
    /// is `dev.kiff.tafs:/oauth` exactly and carries this attempt's `state`; anything else is
    /// `None` and never used, not even as a refusal.
    pub fn answer(&self, link: &str) -> Option<Back> {
        let url = url::Url::parse(link).ok()?;
        if url.scheme() != "dev.kiff.tafs"
            || url.path() != "/oauth"
            || url.host().is_some()
            || url.fragment().is_some()
        {
            return None;
        }
        let mut state = None;
        let mut code = None;
        let mut error = None;
        for (k, v) in url.query_pairs() {
            let slot = match k.as_ref() {
                "state" => &mut state,
                "code" => &mut code,
                "error" => &mut error,
                _ => continue,
            };
            // A field given twice is never ours.
            if slot.replace(v.into_owned()).is_some() {
                return None;
            }
        }
        if state.as_deref() != Some(self.state.as_str()) {
            return None;
        }
        match (code, error) {
            (Some(code), None) if !code.is_empty() => Some(Back::Code(code)),
            (None, Some(error)) => Some(Back::Refused(error)),
            _ => None,
        }
    }

    /// `POST /v1/auth/app/token`'s body: the code, this attempt's verifier, the terms version
    /// the tick agreed to, and the device exactly as `/v1/auth/device/start` sends it.
    pub fn token_body(&self, code: &str, terms_version: &str, device: &Value) -> Value {
        let mut body = json!({
            "client_id": CLIENT_ID,
            "code": code,
            "code_verifier": self.verifier,
            "redirect_uri": REDIRECT_URI,
            "terms_version": terms_version,
        });
        if let (Some(b), Some(d)) = (body.as_object_mut(), device.as_object()) {
            for (k, v) in d {
                b.insert(k.clone(), v.clone());
            }
        }
        body
    }
}

impl Default for Attempt {
    fn default() -> Self {
        Attempt::new()
    }
}

/// What came back from KIFF's page for this attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Back {
    /// A code, good for 60 seconds, once.
    Code(String),
    /// KIFF's `error`: `access_denied`, `unsupported_client`, `temporarily_unavailable`…
    Refused(String),
}

/// How Continue ended when it didn't sign in (`continueOutcome`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The person declined on KIFF's page: nothing is said, the welcome is as it was.
    Declined,
    /// Signing in inside the app is off at KIFF: its words, or "isn't on yet".
    NotOn,
    /// The terms changed since the tick: the box is unticked, and the welcome says why.
    TermsChanged,
    /// Anything else: signing in didn't work, the tick kept, Continue free again.
    Failed(KiffError),
}

/// What KIFF's refusal on the way back means.
pub fn back_refused(error: &str) -> Outcome {
    match error {
        "access_denied" => Outcome::Declined,
        "unsupported_client" => Outcome::NotOn,
        other => Outcome::Failed(KiffError::Server {
            status: 400,
            message: other.to_string(),
        }),
    }
}

/// `POST /v1/auth/app/token`: the session, or how Continue ends.
pub fn token(
    t: &dyn Transport,
    body: &Value,
    w: Option<&WordBook>,
) -> Result<DeviceToken, Outcome> {
    let bytes = serde_json::to_vec(body).map_err(|_| Outcome::Failed(KiffError::Unreadable))?;
    let answer = t
        .post("/v1/auth/app/token", &bytes)
        .map_err(Outcome::Failed)?;
    if (200..300).contains(&answer.status) {
        return serde_json::from_slice(&answer.body)
            .map_err(|_| Outcome::Failed(KiffError::Unreadable));
    }
    Err(match refusal(&answer, w) {
        KiffError::Server { message, .. } if message == "terms_outdated" => Outcome::TermsChanged,
        KiffError::Server { message, .. } if message == "unsupported_client" => Outcome::NotOn,
        other => Outcome::Failed(other),
    })
}

/// `GET /v1/auth/app/terms`, read: the terms version in force.
pub fn terms_version(answer: &Answer) -> Option<String> {
    if !(200..300).contains(&answer.status) {
        return None;
    }
    let v: Value = serde_json::from_slice(&answer.body).ok()?;
    v.get("terms_version")?
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    #[test]
    fn the_challenge_is_rfc_7636s() {
        // RFC 7636 Appendix B.
        let a = Attempt::with("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk", "s");
        assert_eq!(a.challenge(), "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM");
        let fresh = Attempt::new();
        assert_eq!(fresh.verifier.len(), 43);
        assert_ne!(fresh, Attempt::new());
        assert_eq!(format!("{fresh:?}"), "Attempt(…)");
    }

    #[test]
    fn the_authorize_link_is_kiffs() {
        let a = Attempt::with("v", "st8");
        let link = url::Url::parse(&a.authorize_link("pt-BR", Some("r@acme.test"))).unwrap();
        assert_eq!(link.host_str(), Some("api.kiff.dev"));
        assert_eq!(link.path(), "/oauth/authorize");
        let q: Vec<(String, String)> = link.query_pairs().into_owned().collect();
        let get = |k: &str| q.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str());
        assert_eq!(get("response_type"), Some("code"));
        assert_eq!(get("client_id"), Some("tafs-app"));
        assert_eq!(get("redirect_uri"), Some("dev.kiff.tafs:/oauth"));
        assert_eq!(get("code_challenge_method"), Some("S256"));
        assert_eq!(get("code_challenge"), Some(a.challenge().as_str()));
        assert_eq!(get("state"), Some("st8"));
        assert_eq!(get("ui_locales"), Some("pt"));
        assert_eq!(get("login_hint"), Some("r@acme.test"));
        assert_eq!(get("scope"), None);
        // A language KIFF's page doesn't speak is left out: the page falls back to English.
        assert!(!a.authorize_link("ja", None).contains("ui_locales"));
    }

    #[test]
    fn only_this_attempts_link_back_is_taken() {
        let a = Attempt::with("v", "st8");
        assert_eq!(
            a.answer("dev.kiff.tafs:/oauth?code=c1&state=st8"),
            Some(Back::Code("c1".into()))
        );
        assert_eq!(
            a.answer("dev.kiff.tafs:/oauth?error=access_denied&state=st8"),
            Some(Back::Refused("access_denied".into()))
        );
        for other in [
            "dev.kiff.tafs:/oauth?code=c1&state=other",
            "dev.kiff.tafs:/oauth?code=c1",
            "dev.kiff.tafs:/oauth?code=c1&state=st8&state=st8",
            "dev.kiff.tafs:/oauth?code=c1&code=c2&state=st8",
            "dev.kiff.tafs:/other?code=c1&state=st8",
            "dev.kiff.tafs://oauth?code=c1&state=st8",
            "evil.tafs:/oauth?code=c1&state=st8",
            "https://kiff.dev/oauth?code=c1&state=st8",
            "dev.kiff.tafs:/oauth?code=&state=st8",
            "dev.kiff.tafs:/oauth?code=c1&error=x&state=st8",
            "dev.kiff.tafs:/oauth?code=c1&state=st8#x",
            "not a link",
        ] {
            assert_eq!(a.answer(other), None, "{other}");
        }
    }

    #[test]
    fn the_token_body_carries_the_terms_and_the_device() {
        let a = Attempt::with("ver", "s");
        let device = crate::signin::start_body("SPKI", "tafs on Linux");
        let b = a.token_body("c1", "2026-11-14", &device);
        assert_eq!(b["client_id"], "tafs-app");
        assert_eq!(b["code"], "c1");
        assert_eq!(b["code_verifier"], "ver");
        assert_eq!(b["redirect_uri"], "dev.kiff.tafs:/oauth");
        assert_eq!(b["terms_version"], "2026-11-14");
        assert_eq!(b["public_key"], "SPKI");
        assert_eq!(b["key_store"], "software");
        assert_eq!(b["device_name"], "tafs on Linux");
    }

    struct One(RefCell<Option<Answer>>);
    impl Transport for One {
        fn post(&self, path: &str, _: &[u8]) -> Result<Answer, KiffError> {
            assert_eq!(path, "/v1/auth/app/token");
            self.0.borrow_mut().take().ok_or(KiffError::Offline)
        }
    }
    fn answer(status: u16, body: Value) -> One {
        One(RefCell::new(Some(Answer {
            status,
            body: serde_json::to_vec(&body).unwrap(),
        })))
    }

    #[test]
    fn kiffs_answers_end_continue_as_the_ipads() {
        let body = json!({});
        let ok = answer(
            200,
            json!({"access_token":"tok","token_type":"Bearer","expires_in":0,"tenant":"t","subject":"s","approval_mode":"view"}),
        );
        assert_eq!(token(&ok, &body, None).unwrap().access_token, "tok");
        assert_eq!(
            token(&answer(409, json!({"error":"terms_outdated"})), &body, None),
            Err(Outcome::TermsChanged)
        );
        assert_eq!(
            token(
                &answer(400, json!({"error":"unsupported_client"})),
                &body,
                None
            ),
            Err(Outcome::NotOn)
        );
        assert!(matches!(
            token(&answer(400, json!({"error":"invalid_grant"})), &body, None),
            Err(Outcome::Failed(_))
        ));
        assert_eq!(back_refused("access_denied"), Outcome::Declined);
        assert_eq!(back_refused("unsupported_client"), Outcome::NotOn);
    }

    #[test]
    fn the_terms_version_in_force() {
        let a = |status, body: &str| Answer {
            status,
            body: body.as_bytes().to_vec(),
        };
        assert_eq!(
            terms_version(&a(200, r#"{"terms_version":"2026-11-14"}"#)).as_deref(),
            Some("2026-11-14")
        );
        assert_eq!(terms_version(&a(200, r#"{"terms_version":""}"#)), None);
        assert_eq!(terms_version(&a(500, r#"{"terms_version":"x"}"#)), None);
    }
}
