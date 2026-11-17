//! What else waits in Needs you, from Home: a step a taf handed its owner (`TafsCloud.YourStep`,
//! `your_steps`) and Coach's chats waiting on their person (`CoachWaiting`, `coach_waiting`,
//! with "Set all aside", `coach_waiting_clear`). Read as the iPad's decoders read them: a step
//! without its `handoff` fails Home, as Swift's synthesized decoder does; a Coach wait never does.

use serde::de::{Deserialize, Deserializer, Error};
use serde_json::Value;

fn text(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn trimmed(v: &Value, key: &str) -> String {
    text(v, key).trim().to_string()
}

/// A step a taf handed its owner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YourStep {
    pub handoff: String,
    pub run: String,
    /// The taf it's on, by tafs cloud's id and name.
    pub taf: Option<(String, String)>,
    pub what: String,
    pub why: String,
    pub link: Option<String>,
    pub link_host: Option<String>,
    pub title: String,
    pub done: String,
    pub skip: String,
    pub open: Option<String>,
}

impl<'de> Deserialize<'de> for YourStep {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let handoff = v
            .get("handoff")
            .and_then(Value::as_str)
            .ok_or_else(|| D::Error::custom("a step without its handoff"))?
            .to_string();
        let optional = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        let words = v.get("words").cloned().unwrap_or(Value::Null);
        Ok(YourStep {
            handoff,
            run: text(&v, "run"),
            taf: v
                .get("taf")
                .and_then(|t| Some((t.get("id")?.as_str()?.to_string(), text(t, "name")))),
            what: text(&v, "what"),
            why: text(&v, "why"),
            link: optional("link"),
            link_host: optional("link_host"),
            title: text(&words, "title"),
            done: text(&words, "done"),
            skip: text(&words, "skip"),
            open: words
                .get("open")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }
}

impl YourStep {
    /// `openLink`: the vendor's page only as tafs cloud vouched for it: an https link whose own
    /// host is tafs cloud's `link_host`, with the button's words. Anything else is no button. The
    /// link it gives is the parsed URL's own serialization, the one checked, as the iPad keeps and
    /// opens its parsed `URL`.
    pub fn open_link(&self) -> Option<(String, String, String)> {
        let (link, host) = (self.link.as_ref()?, self.link_host.as_ref()?);
        let words = self.open.as_ref().filter(|w| !w.is_empty())?;
        if host.is_empty() {
            return None;
        }
        let url = url::Url::parse(link).ok()?;
        // What opens is the URL as checked, never the string as sent: another parser (GLib's,
        // under the launcher) may read the raw string's host differently, as it does `\@`.
        (url.scheme().eq_ignore_ascii_case("https") && url.host_str()?.eq_ignore_ascii_case(host))
            .then(|| (url.as_str().to_string(), host.clone(), words.clone()))
    }
}

/// A Coach chat waiting on its person: tafs cloud's line, its way in, and its "Set aside".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoachWaiting {
    pub taf: String,
    pub session: String,
    pub line: String,
    pub open: String,
    pub set_aside: String,
}

impl<'de> Deserialize<'de> for CoachWaiting {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let words = v.get("words").cloned().unwrap_or(Value::Null);
        Ok(CoachWaiting {
            taf: text(&v, "taf"),
            session: text(&v, "session"),
            line: trimmed(&words, "line"),
            open: trimmed(&words, "open"),
            set_aside: trimmed(&words, "set_aside"),
        })
    }
}

impl CoachWaiting {
    /// Listed only with its chat and both of tafs cloud's lines.
    pub fn usable(&self) -> bool {
        !self.session.is_empty() && !self.line.is_empty() && !self.open.is_empty()
    }
}

/// "Set all aside": its button, the ask's title, its words, and its yes and no.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CoachWaitingClear {
    pub button: String,
    pub title: String,
    pub text: String,
    pub yes: String,
    pub no: String,
}

impl<'de> Deserialize<'de> for CoachWaitingClear {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = Value::deserialize(d)?;
        let words = v.get("words").cloned().unwrap_or(Value::Null);
        Ok(CoachWaitingClear {
            button: trimmed(&words, "button"),
            title: trimmed(&words, "title"),
            text: trimmed(&words, "text"),
            yes: trimmed(&words, "yes"),
            no: trimmed(&words, "no"),
        })
    }
}

impl CoachWaitingClear {
    pub fn usable(&self) -> bool {
        !self.button.is_empty() && !self.title.is_empty() && !self.yes.is_empty()
    }
}

/// Where one waiting chat is set aside, or every one with none (`setAsidePath`): tafs cloud's
/// "not now", never an approval. Nothing is signed.
pub fn set_aside_path(session: Option<&str>) -> String {
    match session {
        Some(s) => format!(
            "/v1/home/coach-waiting/{}/set-aside",
            crate::access::escaped(s)
        ),
        None => "/v1/home/coach-waiting/set-aside".into(),
    }
}

/// What setting chats aside did (`SetAside`): the chats it ended. None when the answer can't be
/// read as an object; a field it can't read is empty, as the iPad's.
pub fn read_set_aside(body: &[u8]) -> Option<Vec<String>> {
    let v: Value = serde_json::from_slice(body).ok()?;
    v.as_object()?;
    Some(
        v.get("set_aside")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_aside_is_tafs_cloud_s_and_signs_nothing() {
        assert_eq!(
            set_aside_path(Some("s 1/x")),
            "/v1/home/coach-waiting/s%201%2Fx/set-aside"
        );
        assert_eq!(set_aside_path(None), "/v1/home/coach-waiting/set-aside");
        for p in [set_aside_path(Some("s1")), set_aside_path(None)] {
            assert!(!crate::signed::signed("POST", &p), "{p}");
        }
        assert_eq!(
            read_set_aside(br#"{"set_aside":["s1","s2"],"withdrawn":1}"#).unwrap(),
            ["s1", "s2"]
        );
        assert_eq!(
            read_set_aside(br#"{"already":true}"#).unwrap(),
            Vec::<String>::new()
        );
        assert_eq!(read_set_aside(b"[]"), None);
        let clear: CoachWaitingClear = serde_json::from_str(
            r#"{"words":{"button":"Set all aside","title":"End 2?","text":" Their asks go. ","yes":"End them","no":"Keep"}}"#,
        )
        .unwrap();
        assert_eq!(
            (clear.text.as_str(), clear.no.as_str()),
            ("Their asks go.", "Keep")
        );
    }

    #[test]
    fn steps_need_their_handoff_and_coach_waits_never_fail() {
        let s: YourStep = serde_json::from_str(
            r#"{"handoff":"h1","taf":{"id":"t","name":"Refunds"},"what":"Upload the receipt","link":"https://dashboard.stripe.com/x","link_host":"dashboard.stripe.com","words":{"title":"YOUR STEP","done":"Done","skip":"Skip it","open":"Open Stripe"}}"#,
        )
        .unwrap();
        assert_eq!(s.open_link().unwrap().1, "dashboard.stripe.com");
        assert!(serde_json::from_str::<YourStep>(r#"{"what":"x"}"#).is_err());
        // A link that isn't https, or whose host isn't tafs cloud's, is no button.
        let bad = YourStep {
            link: Some("http://dashboard.stripe.com".into()),
            ..s.clone()
        };
        assert_eq!(bad.open_link(), None);
        let other = YourStep {
            link: Some("https://evil.test/x".into()),
            ..s.clone()
        };
        assert_eq!(other.open_link(), None);
        // A backslash reads as a slash to the URL parser, and as part of the userinfo to GLib's:
        // the link that opens is the one checked, so its host is the vouched one either way.
        let tricky = YourStep {
            link: Some("https://good.example\\@evil.example/x".into()),
            link_host: Some("good.example".into()),
            ..s.clone()
        };
        assert_eq!(
            tricky.open_link().map(|(url, _, _)| url).as_deref(),
            Some("https://good.example/@evil.example/x")
        );
        let c: CoachWaiting = serde_json::from_str(
            r#"{"session":"s1","words":{"line":" Coach waits ","open":"Open Coach"}}"#,
        )
        .unwrap();
        assert!(c.usable());
        assert_eq!(c.line, "Coach waits");
        let odd: CoachWaiting = serde_json::from_str(r#"[1,2]"#).unwrap();
        assert!(!odd.usable());
        let clear: CoachWaitingClear = serde_json::from_str(
            r#"{"words":{"button":"Set all aside","title":"Set 2 aside?","yes":"Set aside"}}"#,
        )
        .unwrap();
        assert!(clear.usable());
    }
}

/// A Card unit request (`TafsCloud.CardUnitException`, `card_unit_exceptions` on Home): a taf asked
/// for more of a Card's unit than it allows, and tafs cloud's words for it. Every field is the
/// iPad's `let`, so one missing fails Home, as the iPad's synthesized decoder does.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct CardUnitException {
    pub id: String,
    pub kind: String,
    /// The tafs cloud taf it's for.
    pub taf: String,
    pub name: String,
    pub title: String,
    pub words: String,
    pub history: String,
    /// "Discuss with Coach": for someone who may use Coach and manages the taf.
    pub action_title: String,
    pub coach_text: String,
    /// "Dismiss": signed like a refusal.
    pub dismiss_title: String,
    pub can_approve_once: bool,
}
