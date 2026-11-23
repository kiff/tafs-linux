//! WhatsApp's signup for a door (`DoorsModel.add`'s WhatsApp branch, `whatsappSignupURL`), on a
//! computer: tafs cloud's page runs it in the browser and comes back to `tafs://doors/whatsapp`,
//! a link any page could open. So each signup carries this app's own one-use `state`, and only a
//! return with that `state`, for that taf, within 15 minutes, is taken (docs/doors.md, WhatsApp).

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand_core::RngCore;
use std::time::{Duration, Instant};

/// How long a signup's return is taken (tafs cloud's docs/doors.md).
pub const LIFETIME: Duration = Duration::from_secs(15 * 60);

/// The scheme the return comes back on.
pub const SCHEME: &str = "tafs";

/// A signup this app started: the taf it's for, its `state` and when it began.
pub struct Signup {
    taf: String,
    state: String,
    started: Instant,
}

impl std::fmt::Debug for Signup {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signup({})", self.taf)
    }
}

/// What a return carries, once it's this signup's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Back {
    /// WhatsApp gave a code: the door is added with it.
    Done {
        code: String,
        phone_number_id: String,
        waba_id: String,
    },
    /// This signup's return, with no code: it didn't finish.
    NotFinished,
}

impl Signup {
    /// A fresh signup for `taf`, with a fresh `state`: 32 random bytes, base64url, 43 characters
    /// (tafs cloud takes 16 to 64 letters, digits, `-` or `_`).
    pub fn new(taf: &str) -> Signup {
        let mut b = [0u8; 32];
        rand_core::OsRng.fill_bytes(&mut b);
        Signup::with(taf, &URL_SAFE_NO_PAD.encode(b), Instant::now())
    }

    /// For tests: a known state and start.
    pub fn with(taf: &str, state: &str, started: Instant) -> Signup {
        Signup {
            taf: taf.into(),
            state: state.into(),
            started,
        }
    }

    pub fn taf(&self) -> &str {
        &self.taf
    }

    /// Past 15 minutes, a return is no longer taken.
    pub fn expired(&self, now: Instant) -> bool {
        now.saturating_duration_since(self.started) > LIFETIME
    }

    /// tafs cloud's page for this signup: `<cloud>/doors/whatsapp/connect?taf=…&state=…`.
    pub fn page(&self, cloud: &str) -> String {
        let mut u = url::Url::parse(cloud).expect("tafs cloud's address");
        u.path_segments_mut()
            .expect("a base")
            .pop_if_empty()
            .extend(["doors", "whatsapp", "connect"]);
        u.query_pairs_mut()
            .append_pair("taf", &self.taf)
            .append_pair("state", &self.state);
        u.to_string()
    }

    /// The return, if it's this signup's: `tafs://doors/whatsapp`, each field once, this `state`,
    /// this taf, within 15 minutes. Anything else is None, not even read as a refusal.
    pub fn answer(&self, link: &str, now: Instant) -> Option<Back> {
        if self.expired(now) {
            return None;
        }
        let url = url::Url::parse(link).ok()?;
        if url.scheme() != SCHEME
            || url.host_str() != Some("doors")
            || url.path() != "/whatsapp"
            || url.port().is_some()
            || !url.username().is_empty()
            || url.fragment().is_some()
        {
            return None;
        }
        let (mut state, mut taf, mut code, mut phone, mut waba) = (None, None, None, None, None);
        for (k, v) in url.query_pairs() {
            let slot = match k.as_ref() {
                "state" => &mut state,
                "taf" => &mut taf,
                "code" => &mut code,
                "phone_number_id" => &mut phone,
                "waba_id" => &mut waba,
                _ => continue,
            };
            // A field given twice is never ours.
            if slot.replace(v.into_owned()).is_some() {
                return None;
            }
        }
        if state.as_deref() != Some(self.state.as_str()) || taf.as_deref() != Some(&self.taf) {
            return None;
        }
        Some(match code {
            Some(code) if !code.is_empty() => Back::Done {
                code,
                phone_number_id: phone.unwrap_or_default(),
                waba_id: waba.unwrap_or_default(),
            },
            _ => Back::NotFinished,
        })
    }
}

/// Whether a link the desktop handed the app is a WhatsApp signup's return, to route it.
pub fn is_return(link: &str) -> bool {
    link.starts_with("tafs://doors/whatsapp")
}

/// `addDoor`'s body for WhatsApp: the app and what the signup gave.
pub fn add_body(app: &str, back: &Back) -> Option<Vec<u8>> {
    match back {
        Back::Done {
            code,
            phone_number_id,
            waba_id,
        } => Some(crate::doors::add_body(
            app,
            &[
                ("code", code),
                ("phone_number_id", phone_number_id),
                ("waba_id", waba_id),
            ],
        )),
        Back::NotFinished => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const STATE: &str = "s7aTe_0123456789abcdef";

    fn signup(at: Instant) -> Signup {
        Signup::with("taf-1", STATE, at)
    }

    fn back(q: &str) -> String {
        format!("tafs://doors/whatsapp?{q}")
    }

    #[test]
    fn the_page_carries_the_taf_and_a_fresh_state() {
        let s = signup(Instant::now());
        assert_eq!(
            s.page("https://cloud.tafs.kiff.dev"),
            format!("https://cloud.tafs.kiff.dev/doors/whatsapp/connect?taf=taf-1&state={STATE}")
        );
        // A taf id is a query value, escaped as one.
        let odd = Signup::with("a b&c", STATE, Instant::now());
        assert!(odd
            .page("https://cloud.tafs.kiff.dev/")
            .contains("taf=a+b%26c&"));
        // Each signup its own state, in tafs cloud's alphabet and length.
        let (a, b) = (Signup::new("t"), Signup::new("t"));
        assert_ne!(a.state, b.state);
        assert_eq!(a.state.len(), 43);
        assert!(a
            .state
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'));
    }

    #[test]
    fn its_own_return_is_taken() {
        let now = Instant::now();
        let s = signup(now);
        let got = s.answer(
            &back(&format!(
                "taf=taf-1&code=c0de&phone_number_id=109&waba_id=209&state={STATE}"
            )),
            now,
        );
        assert_eq!(
            got,
            Some(Back::Done {
                code: "c0de".into(),
                phone_number_id: "109".into(),
                waba_id: "209".into()
            })
        );
        let body: serde_json::Value =
            serde_json::from_slice(&add_body("whatsapp", got.as_ref().unwrap()).unwrap()).unwrap();
        assert_eq!(
            body,
            serde_json::json!({"app":"whatsapp","code":"c0de","phone_number_id":"109","waba_id":"209"})
        );
        // Its own return with no code: it didn't finish, and nothing is sent.
        let none = s.answer(&back(&format!("taf=taf-1&code=&state={STATE}")), now);
        assert_eq!(none, Some(Back::NotFinished));
        assert_eq!(add_body("whatsapp", &Back::NotFinished), None);
    }

    #[test]
    fn any_other_return_is_dropped() {
        let now = Instant::now();
        let s = signup(now);
        let ok = format!("taf=taf-1&code=c&state={STATE}");
        for link in [
            // No state, another state, another taf: a page any site could open.
            back("taf=taf-1&code=c"),
            back("taf=taf-1&code=c&state=someone_elses_state_00"),
            back(&format!("taf=taf-2&code=c&state={STATE}")),
            back(&format!("code=c&state={STATE}")),
            // A field twice.
            back(&format!("{ok}&code=d")),
            back(&format!("{ok}&state={STATE}")),
            // Anywhere else.
            format!("tafs://doors/slack?{ok}"),
            format!("tafs://evil/whatsapp?{ok}"),
            format!("tafs://doors:1/whatsapp?{ok}"),
            format!("tafs://x@doors/whatsapp?{ok}"),
            format!("https://doors/whatsapp?{ok}"),
            format!("dev.kiff.tafs:/doors/whatsapp?{ok}"),
            format!("{}#frag", back(&ok)),
            "not a link".into(),
        ] {
            assert_eq!(s.answer(&link, now), None, "{link}");
        }
        assert!(s.answer(&back(&ok), now).is_some());
    }

    #[test]
    fn a_return_after_15_minutes_is_dropped() {
        let start = Instant::now();
        let s = signup(start);
        let link = back(&format!("taf=taf-1&code=c&state={STATE}"));
        assert!(s.answer(&link, start + LIFETIME).is_some());
        assert!(!s.expired(start + LIFETIME));
        assert!(s.expired(start + LIFETIME + Duration::from_secs(1)));
        assert_eq!(
            s.answer(&link, start + LIFETIME + Duration::from_secs(1)),
            None
        );
    }

    #[test]
    fn a_return_is_known_to_route_it() {
        assert!(is_return("tafs://doors/whatsapp?state=x"));
        assert!(!is_return("dev.kiff.tafs:/oauth?code=x"));
    }
}
