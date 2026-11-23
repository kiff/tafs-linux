//! KIFF's words for a connection's profile (`kiff.words.v1`, the iPad's `ProfileWords`): what the
//! owner reads before approving a connection. KIFF writes them, never Coach or the app. The app
//! shows each line's text verbatim, in order, and `kind` only groups and styles it.
//!
//! They're read from KIFF with the person's own session, for the `kiff.profile.v1` in the
//! grant's request (`POST /v1/profiles/words`, the profile's canonical form as its body), and
//! checked before they're shown: KIFF's digest of the profile must be the profile's, and the
//! words' own digest theirs.

use serde_json::Value;

use crate::jcs;
use crate::words::{WordBook, WordsError};

/// The words' version this app reads.
pub const VERSION: &str = "kiff.words.v1";

/// Where they're read: KIFF, as the session.
pub const PATH: &str = "/v1/profiles/words";

/// One line, verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// The operation it's about; none for a line about the whole connection ("unpinned").
    pub op: Option<String>,
    pub kind: String,
    pub text: String,
}

impl Line {
    /// What KIFF can't check for this connection, probation and an unpinned vendor: in amber.
    pub fn caution(&self) -> bool {
        matches!(self.kind.as_str(), "missing" | "probation" | "unpinned")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileWords {
    pub lines: Vec<Line>,
    pub profile_sha256: String,
    pub words_sha256: String,
}

/// Why KIFF's answer isn't shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    /// KIFF answered something that isn't words this app reads.
    NotWords,
    /// The words are for another profile than the one shown.
    OtherProfile,
    /// The words aren't the ones their digest names.
    WordsChanged,
}

impl Failure {
    /// The iPad's words for it.
    pub fn words(self, w: &WordBook) -> Result<String, WordsError> {
        w.line(match self {
            Failure::NotWords => "connect.kiff_words_connection_could_not",
            Failure::OtherProfile => "connect.words_another_connection_than_one",
            Failure::WordsChanged => "kiff.kiff_words_do_not_match",
        })
    }
}

/// The body KIFF is sent: the profile's canonical form. None for a profile with none.
pub fn body(profile: &Value) -> Option<Vec<u8>> {
    jcs::canonical(profile).ok().map(String::into_bytes)
}

/// KIFF's answer to `POST /v1/profiles/words`, checked against `profile`, the profile the owner
/// is shown: its digest, the words' version and their own digest (`ProfileWords.checked`).
pub fn checked(answer: &[u8], profile: &Value) -> Result<ProfileWords, Failure> {
    let a: Value = serde_json::from_slice(answer).map_err(|_| Failure::NotWords)?;
    let words = a.get("words").ok_or(Failure::NotWords)?;
    let str_of = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
    if words.get("v").and_then(Value::as_str) != Some(VERSION) {
        return Err(Failure::NotWords);
    }
    let (Some(profile_sha), Some(words_sha), Some(raw)) = (
        str_of(&a, "profile_sha256"),
        str_of(&a, "words_sha256"),
        words.get("lines").and_then(Value::as_array),
    ) else {
        return Err(Failure::NotWords);
    };
    if jcs::digest(profile).map_err(|_| Failure::OtherProfile)? != profile_sha {
        return Err(Failure::OtherProfile);
    }
    if jcs::digest(words).map_err(|_| Failure::WordsChanged)? != words_sha {
        return Err(Failure::WordsChanged);
    }
    let lines = raw
        .iter()
        .map(|l| {
            Some(Line {
                op: str_of(l, "op"),
                kind: str_of(l, "kind")?,
                text: str_of(l, "text")?,
            })
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(Failure::NotWords)?;
    Ok(ProfileWords {
        lines,
        profile_sha256: profile_sha,
        words_sha256: words_sha,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const VECTORS: [&str; 5] = [
        "gmail-drafts",
        "holded-invoices",
        "notion-pages",
        "slack-messages",
        "stripe-refunds",
    ];

    /// KIFF's `kiff.profile.v1` / `kiff.words.v1` vectors, as the iPad keeps them.
    fn vector(name: &str) -> (Value, Vec<u8>) {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/profile");
        let profile = std::fs::read(format!("{dir}/{name}.json")).unwrap();
        let expected = std::fs::read(format!("{dir}/{name}.expected.json")).unwrap();
        (serde_json::from_slice(&profile).unwrap(), expected)
    }

    /// Each vector: the profile's digest is KIFF's, its words check, in order, verbatim.
    #[test]
    fn the_golden_vectors() {
        for name in VECTORS {
            let (profile, expected) = vector(name);
            let answer: Value = serde_json::from_slice(&expected).unwrap();
            assert_eq!(
                Some(jcs::digest(&profile).unwrap().as_str()),
                answer["profile_sha256"].as_str(),
                "{name}"
            );
            let words = checked(&expected, &profile).unwrap();
            assert_eq!(
                Some(words.words_sha256.as_str()),
                answer["words_sha256"].as_str(),
                "{name}"
            );
            let lines = answer["words"]["lines"].as_array().unwrap();
            assert!(!words.lines.is_empty(), "{name}");
            assert_eq!(
                words
                    .lines
                    .iter()
                    .map(|l| l.text.as_str())
                    .collect::<Vec<_>>(),
                lines
                    .iter()
                    .map(|l| l["text"].as_str().unwrap())
                    .collect::<Vec<_>>(),
                "{name}"
            );
            assert_eq!(
                words.lines.iter().map(|l| l.op.clone()).collect::<Vec<_>>(),
                lines
                    .iter()
                    .map(|l| l["op"].as_str().map(str::to_string))
                    .collect::<Vec<_>>(),
                "{name}: a line about the whole connection has no op"
            );
            // The body sent is the profile's canonical form.
            assert_eq!(
                body(&profile).unwrap(),
                jcs::canonical(&profile).unwrap().into_bytes()
            );
        }
    }

    #[test]
    fn words_for_another_profile_or_changed_are_refused() {
        let (profile, expected) = vector("stripe-refunds");
        let (other, _) = vector("gmail-drafts");
        assert_eq!(checked(&expected, &other), Err(Failure::OtherProfile));
        let mut changed: Value = serde_json::from_slice(&expected).unwrap();
        changed["words"]["lines"][0]["text"] = Value::from("Refund everything.");
        assert_eq!(
            checked(&serde_json::to_vec(&changed).unwrap(), &profile),
            Err(Failure::WordsChanged)
        );
        let mut version: Value = serde_json::from_slice(&expected).unwrap();
        version["words"]["v"] = Value::from("kiff.words.v2");
        assert_eq!(
            checked(&serde_json::to_vec(&version).unwrap(), &profile),
            Err(Failure::NotWords)
        );
        assert_eq!(checked(b"nope", &profile), Err(Failure::NotWords));
    }

    #[test]
    fn what_kiff_cant_check_is_a_caution() {
        let line = |kind: &str| Line {
            op: None,
            kind: kind.into(),
            text: String::new(),
        };
        assert!(line("unpinned").caution() && line("probation").caution());
        assert!(line("missing").caution());
        assert!(!line("does").caution());
    }
}
