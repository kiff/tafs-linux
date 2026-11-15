//! KIFF over HTTPS (`KIFFClient.send`, the parts sign-in uses): JSON, the app's language as
//! `Accept-Language`, and a refusal read by its status and KIFF's code only.

use std::io::Read;
use std::time::Duration;

use tafs_core::language::accept_language;
use tafs_core::session::may_send_to;
use tafs_core::signin::{Answer, KiffError, Transport, KIFF_API};

pub struct Kiff {
    agent: ureq::Agent,
    base: String,
    languages: Vec<&'static str>,
}

/// The app's HTTP: 15 s to connect, 30 s for an answer, and a redirect never carries a request
/// on (KIFF's answers are read where they're sent). It goes through the proxy the desktop names
/// (`HTTPS_PROXY`, as GNOME's own apps do) and trusts this computer's certificate authorities, so
/// a network that requires a proxy reaches KIFF too.
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .redirects(0)
        .user_agent("tafs-linux")
        .try_proxy_from_env(true)
        .build()
}

impl Kiff {
    pub fn new(languages: Vec<&'static str>) -> Kiff {
        let agent = agent();
        Kiff {
            agent,
            base: KIFF_API.to_string(),
            languages,
        }
    }

    fn read(result: Result<ureq::Response, ureq::Error>) -> Result<Answer, KiffError> {
        let response = match result {
            Ok(r) => r,
            Err(ureq::Error::Status(_, r)) => r,
            Err(ureq::Error::Transport(_)) => return Err(KiffError::Offline),
        };
        let status = response.status();
        let mut body = Vec::new();
        // KIFF's answers are small: at most 1 MB is read.
        response
            .into_reader()
            .take(1 << 20)
            .read_to_end(&mut body)
            .map_err(|_| KiffError::Offline)?;
        Ok(Answer { status, body })
    }

    /// `POST /v1/auth/device/logout` with the session alone (`signOut`), as a replaced or
    /// signed-out session ends at KIFF.
    pub fn sign_out(&self, token: &str) -> Result<(), KiffError> {
        let url = format!("{}/v1/auth/device/logout", self.base);
        if !may_send_to(&url) {
            return Err(KiffError::Unreadable);
        }
        let answer = Self::read(
            self.agent
                .post(&url)
                .set("Accept", "application/json")
                .set("Content-Type", "application/json")
                .set("Authorization", &format!("Bearer {token}"))
                .send_bytes(b"{}"),
        )?;
        if (200..300).contains(&answer.status) {
            Ok(())
        } else {
            Err(tafs_core::signin::refusal(&answer, None))
        }
    }
}

/// `GET /v1/home` on tafs cloud as the session, in the app's language (`TafsCloud.home`). The
/// token goes only to tafs cloud's exact host.
pub fn home(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    let url = format!("{}/v1/home", tafs_core::home::TAFS_CLOUD);
    if !may_send_to(&url) {
        return Err(KiffError::Unreadable);
    }
    let agent = agent();
    Kiff::read(
        agent
            .get(&url)
            .set("Accept", "application/json")
            .set("Accept-Language", &accept_language(languages))
            .set("Authorization", &format!("Bearer {token}"))
            .call(),
    )
}

/// A read at KIFF as the session, by its path (People & groups).
pub fn kiff_get(token: &str, languages: &[&str], path: &str) -> Result<Answer, KiffError> {
    get(&format!("{KIFF_API}{path}"), token, languages)
}

/// `GET /v1/me/devices` at KIFF as the session (`devices()`).
pub fn devices(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    get(
        &format!("{KIFF_API}{}", tafs_core::devices::PATH),
        token,
        languages,
    )
}

/// `GET /v1/me/usage` at KIFF as the session (`usage()`): the plan, who bills it, the store and
/// members' requests.
pub fn usage(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    get(
        &format!("{KIFF_API}{}", tafs_core::usage::KIFF_USAGE_PATH),
        token,
        languages,
    )
}

/// `GET /v1/me` at KIFF as the session (`me`): the account, and how this session approves.
pub fn me(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    get(&format!("{KIFF_API}/v1/me"), token, languages)
}

/// `GET /v1/me/activity?limit=100` at KIFF as the session (`activity`): the Record.
pub fn activity(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    get(
        &format!("{KIFF_API}/v1/me/activity?limit=100"),
        token,
        languages,
    )
}

/// A taf's Activity page on tafs cloud as the session (`activityPage`), by its path.
pub fn taf_activity(token: &str, languages: &[&str], path: &str) -> Result<Answer, KiffError> {
    get(
        &format!("{}{path}", tafs_core::home::TAFS_CLOUD),
        token,
        languages,
    )
}

/// A read on tafs cloud as the session, by its path (Plan & usage's balance and credits).
pub fn cloud(token: &str, languages: &[&str], path: &str) -> Result<Answer, KiffError> {
    get(
        &format!("{}{path}", tafs_core::home::TAFS_CLOUD),
        token,
        languages,
    )
}

/// A GET as the session, to KIFF's API or tafs cloud only, with no redirect followed.
fn get(url: &str, token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    if !may_send_to(url) {
        return Err(KiffError::Unreadable);
    }
    let agent = agent();
    Kiff::read(
        agent
            .get(url)
            .set("Accept", "application/json")
            .set("Accept-Language", &accept_language(languages))
            .set("Authorization", &format!("Bearer {token}"))
            .call(),
    )
}

/// `GET /v1/me/card-exceptions?status=held` at KIFF as the session (`heldCalls`).
pub fn held(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    let url = format!("{KIFF_API}/v1/me/card-exceptions?status=held");
    if !may_send_to(&url) {
        return Err(KiffError::Unreadable);
    }
    let agent = agent();
    Kiff::read(
        agent
            .get(&url)
            .set("Accept", "application/json")
            .set("Accept-Language", &accept_language(languages))
            .set("Authorization", &format!("Bearer {token}"))
            .call(),
    )
}

impl Transport for Kiff {
    fn post(&self, path: &str, body: &[u8]) -> Result<Answer, KiffError> {
        let url = format!("{}{}", self.base, path);
        Self::read(
            self.agent
                .post(&url)
                .set("Accept", "application/json")
                .set("Content-Type", "application/json")
                .set("Accept-Language", &accept_language(&self.languages))
                .send_bytes(body),
        )
    }
}
