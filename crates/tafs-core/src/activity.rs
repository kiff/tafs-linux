//! A tafs cloud taf's Activity (`TafActivityList`, `SourceActivityRow`, `NextStepRow`): tafs
//! cloud's own pages (`GET /v1/tafs/{id}/activity`, `TafsCloud.ActivityPage`), its filters, its
//! day groups and each call's line, drawn as sent.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

use crate::record::ActivityCall;

fn time<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
    let s = String::deserialize(d)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Choice {
    pub id: String,
    pub title: String,
}

/// What the owner can do about a call that wasn't done, in tafs cloud's words (`NextStep`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NextStep {
    pub kind: String,
    pub words: String,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub taf: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActivityEntry {
    pub id: String,
    #[serde(deserialize_with = "time")]
    pub at: DateTime<Utc>,
    pub kind: String,
    pub title: String,
    pub detail: String,
    pub time: String,
    pub amount_text: Option<String>,
    pub call: ActivityCall,
    #[serde(default)]
    pub next: Option<NextStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActivityGroup {
    pub id: String,
    pub title: String,
    pub entries: Vec<ActivityEntry>,
}

/// One page (`ActivityPage`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActivityPage {
    pub entries: Vec<ActivityEntry>,
    pub groups: Vec<ActivityGroup>,
    pub filters: Vec<Choice>,
    pub filter: String,
    pub revision: String,
    pub next_cursor: Option<String>,
    #[serde(default)]
    pub end_words: Option<String>,
    /// The taf's Card unit requests, newest as tafs cloud sends them; absent, Home's are drawn.
    #[serde(default)]
    pub card_unit_exceptions: Option<Vec<crate::waits::CardUnitException>>,
    #[serde(default)]
    pub card_unit_exceptions_notice: Option<String>,
}

impl ActivityPage {
    pub fn read(body: &[u8]) -> Option<ActivityPage> {
        serde_json::from_slice(body).ok()
    }
}

/// The page's path (`activityPage`): the taf by its id, the filter, and the cursor, each escaped.
pub fn path(taf: &str, filter: &str, cursor: Option<&str>) -> String {
    let escaped = |v: &str| {
        url::form_urlencoded::byte_serialize(v.as_bytes())
            .collect::<String>()
            .replace('+', "%20")
    };
    let mut p = format!(
        "/v1/tafs/{}/activity?filter={}",
        escaped(taf),
        escaped(filter)
    );
    if let Some(c) = cursor {
        p.push_str(&format!("&cursor={}", escaped(c)));
    }
    p
}

/// What shows under the pages (`TafActivityList`'s foot): an error and Try again, more to load,
/// tafs cloud's end words, or "Nothing here." when no page has an entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Foot {
    Error(String),
    More(String),
    End(String),
    NothingHere,
    Nothing,
}

pub fn foot(pages: &[ActivityPage], error: Option<&str>) -> Foot {
    if let Some(e) = error {
        return Foot::Error(e.to_string());
    }
    if let Some(cursor) = pages.last().and_then(|p| p.next_cursor.clone()) {
        return Foot::More(cursor);
    }
    if let Some(words) = pages.last().and_then(|p| p.end_words.clone()) {
        return Foot::End(words);
    }
    if pages.iter().all(|p| p.entries.is_empty()) {
        return Foot::NothingHere;
    }
    Foot::Nothing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_page_carries_its_card_unit_requests_or_none() {
        let page = |extra: &str| {
            ActivityPage::read(
                format!(
                    r#"{{"entries":[],"groups":[],"filters":[],"filter":"all","revision":"r","next_cursor":null{extra}}}"#
                )
                .as_bytes(),
            )
        };
        assert_eq!(page("").unwrap().card_unit_exceptions, None);
        assert_eq!(
            page(r#","card_unit_exceptions":[]"#)
                .unwrap()
                .card_unit_exceptions,
            Some(vec![])
        );
        let ask = r#"{"id":"e","kind":"card_unit","taf":"t","name":"Rae","title":"T","words":"W","history":"H","action_title":"A","coach_text":"C","dismiss_title":"D","can_approve_once":false}"#;
        let read = page(&format!(r#","card_unit_exceptions":[{ask}]"#)).unwrap();
        assert_eq!(read.card_unit_exceptions.unwrap()[0].title, "T");
        // One without what the iPad needs: the page isn't read, as its decoder does.
        assert!(page(r#","card_unit_exceptions":[{"id":"e"}]"#).is_none());
    }

    #[test]
    fn paths_are_escaped() {
        assert_eq!(
            path("taf-1", "all", None),
            "/v1/tafs/taf-1/activity?filter=all"
        );
        assert_eq!(
            path("a/b", "held", Some("c 1&x")),
            "/v1/tafs/a%2Fb/activity?filter=held&cursor=c%201%26x"
        );
    }
}
