//! The AI-data consent (`TafsCloud+AIConsent.swift`, `AIConsentView`, `AIConsentRow`) and Coach's
//! "Show suggestions" setting (`TafsCloud.Suggestions`), both read and changed at tafs cloud as the
//! session, unsigned. Every word of the consent is tafs cloud's.

use serde::{Deserialize, Deserializer};
use serde_json::Value;

/// `GET`, `POST` (allow) and `DELETE` (withdraw): the consent's route.
pub const CONSENT_PATH: &str = "/v1/me/ai-consent";
/// `GET` and `PUT`: the account's "Show suggestions".
pub const SUGGESTIONS_PATH: &str = "/v1/me/suggestions";

/// The consent screen's words (`AIConsent.Words`), each required, as the iPad's decoder.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConsentWords {
    pub title: String,
    pub lines: Vec<String>,
    pub allow: String,
    pub not_now: String,
    pub withdraw: String,
    /// Account's row once it's given ("AI use: allowed").
    pub given: String,
}

/// The person's consent as tafs cloud keeps it (`TafsCloud.AIConsent`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct AIConsent {
    pub given: bool,
    #[serde(default)]
    pub at: Option<String>,
    /// The words' version: a yes is sent with the version it was given to.
    pub version: String,
    pub words: ConsentWords,
}

impl AIConsent {
    /// `drawable`: a title, the lines, Allow and Not now; anything less is no screen.
    pub fn drawable(&self) -> bool {
        [&self.words.title, &self.words.allow, &self.words.not_now]
            .iter()
            .all(|w| !w.trim().is_empty())
            && !self.words.lines.is_empty()
    }

    /// Withdraw, in Account, when tafs cloud sent words for it.
    pub fn withdraw(&self) -> Option<&str> {
        Some(self.words.withdraw.as_str()).filter(|w| !w.trim().is_empty())
    }
}

/// `{"consent": …}`, from any of the route's answers, or the consent a refusal carries (a 409 for
/// an older screen's yes, a 403 `ai_consent_required`). None when it can't be read.
pub fn read_consent(body: &[u8]) -> Option<AIConsent> {
    #[derive(Deserialize)]
    struct Answer {
        consent: AIConsent,
    }
    serde_json::from_slice::<Answer>(body)
        .ok()
        .map(|a| a.consent)
}

/// Allow's body: the version of the words the person read.
pub fn allow_body(version: &str) -> Vec<u8> {
    serde_json::json!({ "version": version })
        .to_string()
        .into_bytes()
}

/// The account's "Show suggestions" (`Suggestions`): on and no words when absent; a field of the
/// wrong kind fails the read, as `decodeIfPresent` does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestions {
    pub show: bool,
    pub words: String,
}

impl<'de> Deserialize<'de> for Suggestions {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Suggestions, D::Error> {
        let v = serde_json::Map::<String, Value>::deserialize(d)?;
        let show = match v.get("show") {
            None | Some(Value::Null) => true,
            Some(Value::Bool(b)) => *b,
            Some(_) => return Err(serde::de::Error::custom("show isn't a bool")),
        };
        let words = match v.get("words") {
            None | Some(Value::Null) => String::new(),
            Some(Value::String(s)) => s.clone(),
            Some(_) => return Err(serde::de::Error::custom("words isn't text")),
        };
        Ok(Suggestions { show, words })
    }
}

/// `{"suggestions": …}`, from `GET` or `PUT`. None when it can't be read.
pub fn read_suggestions(body: &[u8]) -> Option<Suggestions> {
    #[derive(Deserialize)]
    struct Answer {
        suggestions: Suggestions,
    }
    serde_json::from_slice::<Answer>(body)
        .ok()
        .map(|a| a.suggestions)
}

/// `setSuggestions`' body.
pub fn suggestions_body(show: bool) -> Vec<u8> {
    serde_json::json!({ "show": show }).to_string().into_bytes()
}

/// Whether Account's Coach section is drawn: signed in with KIFF, for someone who may use Coach,
/// once the setting is read. Its switch works only for someone who manages Cards
/// (`canManageCards`, none listed counting as allowed).
pub fn switch_enabled(capabilities: Option<&[String]>) -> bool {
    capabilities.is_none_or(|c| c.iter().any(|c| c == "cards.manage"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions_read_as_the_ipad_reads_them() {
        assert_eq!(
            read_suggestions(br#"{"suggestions":{}}"#),
            Some(Suggestions {
                show: true,
                words: String::new()
            })
        );
        assert_eq!(
            read_suggestions(br#"{"suggestions":{"show":false,"words":"Off for good."}}"#),
            Some(Suggestions {
                show: false,
                words: "Off for good.".into()
            })
        );
        assert_eq!(read_suggestions(br#"{"suggestions":{"show":"no"}}"#), None);
        assert_eq!(read_suggestions(br#"{}"#), None);
        assert_eq!(suggestions_body(false), br#"{"show":false}"#);
    }

    #[test]
    fn a_consent_without_its_words_is_no_screen() {
        let mut c = AIConsent {
            given: false,
            at: None,
            version: "1".into(),
            words: ConsentWords {
                title: "T".into(),
                lines: vec!["L".into()],
                allow: "Allow".into(),
                not_now: "Not now".into(),
                withdraw: " ".into(),
                given: "AI use: allowed".into(),
            },
        };
        assert!(c.drawable());
        assert_eq!(c.withdraw(), None);
        c.words.lines.clear();
        assert!(!c.drawable());
        assert_eq!(allow_body("1"), br#"{"version":"1"}"#);
        assert!(switch_enabled(None));
        assert!(!switch_enabled(Some(&["usage.read".to_string()])));
        assert!(switch_enabled(Some(&["cards.manage".to_string()])));
        // KIFF listing nothing this person may do isn't the same as listing none at all: the
        // iPad's `managesCards` is `capabilities.map { contains } ?? true`.
        assert!(!switch_enabled(Some(&[])));
    }
}
