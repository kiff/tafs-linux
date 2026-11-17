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

/// A taf's halt, or with none the account's, at tafs cloud as the session: `GET` reads where it
/// stands (`halted`); `POST` halts it (`halt`), one tap and no signature, since it only takes
/// authority away. The taf is tafs cloud's id, escaped as one path segment.
pub fn halt(
    token: &str,
    languages: &[&str],
    taf: Option<&str>,
    halt_it: bool,
) -> Result<Answer, KiffError> {
    let url = format!(
        "{}{}",
        tafs_core::home::TAFS_CLOUD,
        tafs_core::halt::path(taf)
    );
    if !may_send_to(&url) {
        return Err(KiffError::Unreadable);
    }
    let agent = agent();
    let request = if halt_it {
        agent.post(&url)
    } else {
        agent.get(&url)
    }
    .set("Accept", "application/json")
    .set("Accept-Language", &accept_language(languages))
    .set("Authorization", &format!("Bearer {token}"));
    Kiff::read(if halt_it {
        request
            .set("Content-Type", "application/json")
            .send_bytes(b"{}")
    } else {
        request.call()
    })
}

/// Run now (`TafsCloud.runNow`): `POST /v1/tafs/{id}/run` at tafs cloud as the session, with an
/// empty body. It fires the taf's schedule; no signature, as the iPad's.
pub fn run_now(token: &str, languages: &[&str], taf: &str) -> Result<Answer, KiffError> {
    let url = format!(
        "{}{}",
        tafs_core::home::TAFS_CLOUD,
        tafs_core::runs::run_path(taf)
    );
    if !may_send_to(&url) {
        return Err(KiffError::Unreadable);
    }
    Kiff::read(
        agent()
            .post(&url)
            .set("Accept", "application/json")
            .set("Accept-Language", &accept_language(languages))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_bytes(b"{}"),
    )
}

/// Send again (`TafsCloud.sendAgain`): a delivery given up on, sent once more, at tafs cloud as
/// the session. It does no work and signs nothing.
pub fn send_again(token: &str, languages: &[&str], path: &str) -> Result<Answer, KiffError> {
    let url = format!("{}{path}", tafs_core::home::TAFS_CLOUD);
    if !may_send_to(&url) || tafs_core::signed::signed_at_cloud("POST", path) {
        return Err(KiffError::Unreadable);
    }
    Kiff::read(
        agent()
            .post(&url)
            .set("Accept", "application/json")
            .set("Accept-Language", &accept_language(languages))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_bytes(b"{}"),
    )
}

/// A change at tafs cloud as the session, by its method and path, with a JSON body or none: the
/// AI-data consent (`POST`, `DELETE`) and Coach's suggestions (`PUT`). It signs nothing.
pub fn cloud_send(
    token: &str,
    languages: &[&str],
    method: &str,
    path: &str,
    body: Option<&[u8]>,
) -> Result<Answer, KiffError> {
    let url = format!("{}{path}", tafs_core::home::TAFS_CLOUD);
    // What the device signs at tafs cloud is never sent from here, whatever builds its path.
    if !may_send_to(&url)
        || !matches!(method, "POST" | "PUT" | "PATCH" | "DELETE")
        || tafs_core::signed::signed_at_cloud(method, path)
    {
        return Err(KiffError::Unreadable);
    }
    let request = agent()
        .request(method, &url)
        .set("Accept", "application/json")
        .set("Accept-Language", &accept_language(languages))
        .set("Authorization", &format!("Bearer {token}"));
    Kiff::read(match body {
        Some(b) => request
            .set("Content-Type", "application/json")
            .send_bytes(b),
        None => request.call(),
    })
}

/// Coach's waiting chats set aside (`TafsCloud.setAsideCoach`): one, or every one, at tafs cloud
/// as the session. It signs nothing; this device's signed read of its connections (`proof`), when
/// it has one, goes as `Planque-Device-Signature` so tafs cloud can withdraw a connection the chat
/// asked for. Neither the token nor the proof is ever logged.
pub fn set_aside(
    token: &str,
    languages: &[&str],
    path: &str,
    proof: Option<&str>,
) -> Result<Answer, KiffError> {
    let url = format!("{}{path}", tafs_core::home::TAFS_CLOUD);
    if !may_send_to(&url)
        || tafs_core::signed::signed("POST", path)
        || tafs_core::signed::signed_at_cloud("POST", path)
    {
        return Err(KiffError::Unreadable);
    }
    let mut request = agent()
        .post(&url)
        .set("Accept", "application/json")
        .set("Accept-Language", &accept_language(languages))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json");
    if let Some(proof) = proof {
        request = request.set(tafs_core::planque::HEADER, proof);
    }
    Kiff::read(request.send_bytes(b"{}"))
}

/// A knock answered (`TafsCloud.answerKnock`): "do", "pair" or "ignore", at tafs cloud as the
/// session. It signs nothing.
pub fn answer_knock(
    token: &str,
    languages: &[&str],
    path: &str,
    body: &[u8],
) -> Result<Answer, KiffError> {
    let url = format!("{}{path}", tafs_core::home::TAFS_CLOUD);
    if !may_send_to(&url) || tafs_core::signed::signed_at_cloud("POST", path) {
        return Err(KiffError::Unreadable);
    }
    Kiff::read(
        agent()
            .post(&url)
            .set("Accept", "application/json")
            .set("Accept-Language", &accept_language(languages))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_bytes(body),
    )
}

/// `GET /v1/recipes/gallery` at tafs cloud as the session (`TafsCloud.gallery`): every
/// ready-made job, at any time.
pub fn gallery(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    let url = format!("{}/v1/recipes/gallery", tafs_core::home::TAFS_CLOUD);
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

/// `GET /v1/recipes` at tafs cloud as the session (`TafsCloud.recipes`): the jobs Home lists with
/// no taf yet.
pub fn recipes(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    let url = format!("{}/v1/recipes", tafs_core::home::TAFS_CLOUD);
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

/// An app's logo from tafs cloud (`AsyncImage(url: logoURL(…))`): no token, as the iPad's image
/// load sends none; only a URL `recipes::logo_url` made; at most 1 MB. None for anything else.
pub fn logo(url: &str) -> Option<Vec<u8>> {
    use std::io::Read;
    if !url.starts_with(&format!("{}/v1/logos/", tafs_core::home::TAFS_CLOUD)) {
        return None;
    }
    let answer = agent().get(url).call().ok()?;
    if answer.status() != 200 {
        return None;
    }
    let mut bytes = Vec::new();
    answer
        .into_reader()
        .take(1 << 20)
        .read_to_end(&mut bytes)
        .ok()?;
    Some(bytes)
}

/// A read at KIFF as the session, by its path (People & groups).
pub fn kiff_get(token: &str, languages: &[&str], path: &str) -> Result<Answer, KiffError> {
    get(&format!("{KIFF_API}{path}"), token, languages)
}

/// A change at KIFF as the session, unsigned: asking an admin, or withdrawing the request. Only an
/// `Unsigned` is sent, which can't be one of the calls KIFF takes only signed.
pub fn kiff_send(
    token: &str,
    languages: &[&str],
    call: &tafs_core::signed::Unsigned,
) -> Result<Answer, KiffError> {
    let url = format!("{KIFF_API}{}", call.path());
    if !may_send_to(&url) {
        return Err(KiffError::Unreadable);
    }
    Kiff::read(
        agent()
            .request(call.method(), &url)
            .set("Accept", "application/json")
            .set("Accept-Language", &accept_language(languages))
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .send_bytes(call.body()),
    )
}

/// A file's bytes, straight to storage where tafs cloud said (`TafsCloud.upload`): no session
/// goes with them, only the headers tafs cloud gave, and only to an HTTPS address. Two minutes.
pub fn upload(to: &tafs_core::attach::Upload, data: &[u8]) -> Result<u16, KiffError> {
    if !to.sendable() {
        return Err(KiffError::Unreadable);
    }
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(120))
        .redirects(0)
        .user_agent("tafs-linux")
        .try_proxy_from_env(true)
        .build();
    let mut request = agent.request(&to.method, &to.url);
    for (name, value) in &to.headers {
        request = request.set(name, value);
    }
    match request.send_bytes(data) {
        Ok(r) => Ok(r.status()),
        Err(ureq::Error::Status(status, _)) => Ok(status),
        Err(ureq::Error::Transport(_)) => Err(KiffError::Offline),
    }
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
/// A GET at tafs cloud as the session whose answer is saved as files (`exportReceipts`,
/// `exportChats`): up to 32 MB is read, and an answer longer than that is refused whole rather
/// than saved cut short.
pub fn cloud_file(token: &str, languages: &[&str], path: &str) -> Result<Answer, KiffError> {
    const MOST: u64 = 32 << 20;
    let url = format!("{}{path}", tafs_core::home::TAFS_CLOUD);
    if !may_send_to(&url) {
        return Err(KiffError::Unreadable);
    }
    let response = match agent()
        .get(&url)
        .set("Accept", "application/json")
        .set("Accept-Language", &accept_language(languages))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
    {
        Ok(r) => r,
        Err(ureq::Error::Status(_, r)) => r,
        Err(ureq::Error::Transport(_)) => return Err(KiffError::Offline),
    };
    let status = response.status();
    let mut body = Vec::new();
    response
        .into_reader()
        .take(MOST + 1)
        .read_to_end(&mut body)
        .map_err(|_| KiffError::Offline)?;
    if body.len() as u64 > MOST {
        return Err(KiffError::Unreadable);
    }
    Ok(Answer { status, body })
}

pub fn cloud(token: &str, languages: &[&str], path: &str) -> Result<Answer, KiffError> {
    cloud_with(token, languages, path, &[])
}

/// A read on tafs cloud as the session with headers of its own: Coach's conversation with this
/// device's proof (`Planque-Device-Signature`), which tafs cloud only forwards. Neither the token
/// nor a header's value is ever logged.
pub fn cloud_with(
    token: &str,
    languages: &[&str],
    path: &str,
    headers: &[(&str, &str)],
) -> Result<Answer, KiffError> {
    get_with(
        &format!("{}{path}", tafs_core::home::TAFS_CLOUD),
        token,
        languages,
        headers,
    )
}

/// A GET as the session, to KIFF's API or tafs cloud only, with no redirect followed.
fn get(url: &str, token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    get_with(url, token, languages, &[])
}

fn get_with(
    url: &str,
    token: &str,
    languages: &[&str],
    headers: &[(&str, &str)],
) -> Result<Answer, KiffError> {
    if !may_send_to(url) {
        return Err(KiffError::Unreadable);
    }
    let mut request = agent()
        .get(url)
        .set("Accept", "application/json")
        .set("Accept-Language", &accept_language(languages))
        .set("Authorization", &format!("Bearer {token}"));
    for (name, value) in headers {
        request = request.set(name, value);
    }
    Kiff::read(request.call())
}

/// `GET /v1/me/did-this-happen` at KIFF as the session (`didThisHappen`): the money calls KIFF
/// can't prove, newest first.
pub fn did_this_happen(token: &str, languages: &[&str]) -> Result<Answer, KiffError> {
    let url = format!("{KIFF_API}/v1/me/did-this-happen");
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
