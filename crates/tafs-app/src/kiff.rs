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

impl Kiff {
    pub fn new(languages: Vec<&'static str>) -> Kiff {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(15))
            .timeout(Duration::from_secs(30))
            // A redirect never carries a request on: KIFF's answers are read where they're sent.
            .redirects(0)
            .user_agent("tafs-linux")
            .build();
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
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .redirects(0)
        .user_agent("tafs-linux")
        .build();
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
