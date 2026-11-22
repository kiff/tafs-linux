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
    /// "Report this payment": only on a payment this reader may report and that isn't reported
    /// yet. tafs cloud decides who sees the button; with no `report` there is none.
    #[serde(default)]
    pub report: Option<crate::payment_report::Offer>,
    /// When it was reported, as KIFF keeps it on the receipt.
    #[serde(default)]
    pub reported_at: Option<String>,
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

/// The page's path (`activityPage`): the taf by its id, the filter, and the cursor, each escaped,
/// and an app's calls (`selection`, a connected app's `activity_query` as tafs cloud wrote it)
/// after the filter. A selection that isn't a plain query is left out.
pub fn path(taf: &str, filter: &str, cursor: Option<&str>, selection: &str) -> String {
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
    let plain = |s: &str| {
        s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.~=&%+".contains(c))
    };
    if !selection.is_empty() && plain(selection) {
        p.push('&');
        p.push_str(selection);
    }
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

/// What a not-done row's next step does (`NextStepWords.Action`), as this computer can: tafs
/// cloud says which step and its words, the app only finds what the button opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NextAction {
    /// Connect apps, where the owner connects that app again. Signing in again where its vendor
    /// signed it out needs planque's grants, which this computer doesn't read: Connect apps.
    ConnectApps,
    /// Coach on the taf, with these words in the box for the owner to send.
    AskCoach(String),
    /// A line: try later, or a kind this app doesn't know.
    Line,
}

/// `NextStepWords.action`; `fallback` is Coach's first message when tafs cloud sent none.
pub fn next_action(next: &NextStep, fallback: &str) -> NextAction {
    match next.kind.as_str() {
        "reconnect" => NextAction::ConnectApps,
        "ask_coach" => {
            let message = next.message.as_deref().map(str::trim).unwrap_or("");
            NextAction::AskCoach(if message.is_empty() {
                fallback.to_string()
            } else {
                next.message.clone().unwrap_or_default()
            })
        }
        _ => NextAction::Line,
    }
}

/// `NextStepRow`'s `may`: a button only for whoever manages the taf, signed in; Coach's only for
/// whoever may use Coach.
pub fn next_has_button(action: &NextAction, manages: bool, may_use_coach: bool) -> bool {
    manages
        && match action {
            NextAction::AskCoach(_) => may_use_coach,
            NextAction::ConnectApps => true,
            NextAction::Line => false,
        }
}

/// `SourceActivityRow.evidence`: the row as the Record's line its receipt opens on, its kind,
/// title and detail tafs cloud's (`RecordEntry.Kind(rawValue:)`, else unknown); none for a call
/// with no actor.
pub fn evidence(
    w: &crate::words::WordBook,
    e: &ActivityEntry,
    taf: &str,
) -> Option<crate::record::Entry> {
    use crate::record::Kind;
    e.call.actor_id.as_ref()?;
    let mut entry = crate::record::record(
        w,
        std::slice::from_ref(&e.call),
        &|_| Some(taf.to_string()),
        // No connections are read here: which vendor refused stays unnamed.
        &Default::default(),
    )
    .ok()?
    .into_iter()
    .next()?;
    entry.kind = match e.kind.as_str() {
        "done" => Kind::Done,
        "approved" => Kind::Approved,
        "refused" => Kind::Refused,
        "held" => Kind::Held,
        "revoked" => Kind::Revoked,
        "started" => Kind::Started,
        "not_done" => Kind::NotDone,
        "sending" => Kind::Sending,
        "in_tafs" => Kind::InTafs,
        _ => Kind::Unknown,
    };
    entry.title = e.title.clone();
    entry.detail = e.detail.clone();
    entry.report =
        crate::payment_report::Reportable::from_cloud(e.report.as_ref(), e.reported_at.as_deref());
    Some(entry)
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
            path("taf-1", "all", None, ""),
            "/v1/tafs/taf-1/activity?filter=all"
        );
        assert_eq!(
            path("a/b", "held", Some("c 1&x"), ""),
            "/v1/tafs/a%2Fb/activity?filter=held&cursor=c%201%26x"
        );
        // An app's calls, as tafs cloud wrote its query, after the filter.
        assert_eq!(
            path("taf-1", "all", Some("c2"), "app=stripe&grant=g_1"),
            "/v1/tafs/taf-1/activity?filter=all&app=stripe&grant=g_1&cursor=c2"
        );
        // One that isn't a plain query is left out.
        assert_eq!(
            path("taf-1", "all", None, "app=x#/../me"),
            "/v1/tafs/taf-1/activity?filter=all"
        );
    }

    #[test]
    fn a_next_step_and_what_it_opens() {
        let step = |kind: &str, message: Option<&str>| NextStep {
            kind: kind.into(),
            words: "Connect it again".into(),
            tool: Some("stripe.refund".into()),
            taf: None,
            message: message.map(String::from),
        };
        assert_eq!(
            next_action(&step("reconnect", None), "f"),
            NextAction::ConnectApps
        );
        assert_eq!(
            next_action(&step("ask_coach", Some("Why did it stop?")), "f"),
            NextAction::AskCoach("Why did it stop?".into())
        );
        assert_eq!(
            next_action(&step("ask_coach", Some("  ")), "Refund: not sent"),
            NextAction::AskCoach("Refund: not sent".into())
        );
        assert_eq!(next_action(&step("try_later", None), "f"), NextAction::Line);
        let coach = NextAction::AskCoach("x".into());
        assert!(next_has_button(&coach, true, true));
        assert!(!next_has_button(&coach, true, false));
        assert!(!next_has_button(&NextAction::ConnectApps, false, true));
        assert!(!next_has_button(&NextAction::Line, true, true));
    }

    #[test]
    fn a_row_opens_its_receipt_as_the_records_line() {
        let w =
            crate::words::WordBook::load(&crate::words::WordBook::default_root(), "en").unwrap();
        let body = include_str!("../../../shared/contracts/taf-activity.json");
        let v: serde_json::Value = serde_json::from_str(body).unwrap();
        let page = ActivityPage::read(v["answer"].to_string().as_bytes()).unwrap();
        let mut e = page.entries[0].clone();
        let entry = evidence(&w, &e, "rae").unwrap();
        assert_eq!(entry.taf, "rae");
        assert_eq!(entry.kind, crate::record::Kind::Done);
        assert_eq!(entry.title, e.title);
        e.kind = "not_done".into();
        assert_eq!(
            evidence(&w, &e, "rae").unwrap().kind,
            crate::record::Kind::NotDone
        );
        e.kind = "whatever".into();
        assert_eq!(
            evidence(&w, &e, "rae").unwrap().kind,
            crate::record::Kind::Unknown
        );
        e.call.actor_id = None;
        assert!(evidence(&w, &e, "rae").is_none());
    }
}
