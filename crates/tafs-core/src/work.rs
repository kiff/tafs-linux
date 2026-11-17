//! Where a tafs cloud taf's work stands (`GET /v1/tafs/{id}/work`, `TafsCloud.TafWork`,
//! docs/reliability.md), read as leniently as the iPad's decoder, and what `WorkPanel` draws of
//! it. Every line and time is tafs cloud's, in the account's zone: the app formats none.

use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::home::Health;

/// The path, with the taf's id escaped as one segment.
pub fn path(taf: &str) -> String {
    format!("/v1/tafs/{}/work", segment(taf))
}

/// One path segment, escaped: anything but an unreserved character as %XX.
pub fn segment(taf: &str) -> String {
    let id: String = taf
        .bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect();
    id
}

fn time(v: Option<&Value>) -> Option<DateTime<Utc>> {
    v.and_then(Value::as_str)
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
}

fn string(o: &serde_json::Map<String, Value>, key: &str) -> String {
    o.get(key).and_then(Value::as_str).unwrap_or("").to_string()
}

/// Where a request's result went (`Delivery`), and whether it can be sent again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// `door_reply`, or a place of the taf's own, named by `key`.
    pub place: String,
    pub key: Option<String>,
    pub line: String,
    pub send_again: bool,
}

impl Delivery {
    /// What Send again names in its route (`sendPlace`): the door's reply, or the place's key.
    pub fn send_place(&self) -> String {
        if self.place == "door_reply" {
            "door_reply".into()
        } else {
            self.key.clone().unwrap_or_else(|| "door_reply".into())
        }
    }

    /// One delivery, as the iPad's lenient `Delivery`: none only when it isn't an object.
    pub fn read(d: &Value) -> Option<Delivery> {
        let d = d.as_object()?;
        Some(Delivery {
            place: d
                .get("place")
                .and_then(Value::as_str)
                .unwrap_or("door_reply")
                .to_string(),
            key: d.get("key").and_then(Value::as_str).map(str::to_string),
            line: string(d, "line"),
            send_again: d
                .get("send_again")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
    }
}

/// One request (`Request`): done, working, retrying, needs_you, waiting, stopped or failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub id: String,
    pub at: DateTime<Utc>,
    pub state: String,
    pub line: String,
    pub deliveries: Vec<Delivery>,
    /// What the run used, in tafs cloud's words (`creditsLine`): none until KIFF counted it.
    pub credits: Option<String>,
    /// A step the run handed its owner (`yourStep`), read with `try?`: one it can't read is none.
    pub your_step: Option<Box<crate::waits::YourStep>>,
}

impl Request {
    /// Its id and time are required, as the iPad's; the rest reads as nothing when it isn't one.
    pub fn read(v: &Value) -> Option<Request> {
        let o = v.as_object()?;
        let deliveries = match o.get("deliveries").and_then(Value::as_array) {
            Some(a) => a
                .iter()
                .map(Delivery::read)
                .collect::<Option<Vec<_>>>()
                .unwrap_or_default(),
            None => vec![],
        };
        Some(Request {
            id: o.get("id")?.as_str()?.to_string(),
            at: time(o.get("at"))?,
            state: string(o, "state"),
            line: string(o, "line"),
            deliveries,
            credits: o
                .get("credits")
                .and_then(|c| c.get("words"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|w| !w.is_empty())
                .map(str::to_string),
            your_step: o
                .get("your_step")
                .and_then(|v| serde_json::from_value(v.clone()).ok()),
        })
    }
}

/// A scheduled run that was skipped (`Skipped`): not a failure, nothing ran.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Skipped {
    pub due: DateTime<Utc>,
    pub words: String,
}

impl Skipped {
    pub fn read(v: &Value) -> Option<Skipped> {
        let o = v.as_object()?;
        Some(Skipped {
            due: time(o.get("due"))?,
            words: string(o, "words"),
        })
    }
}

/// A row of the work, newest first (`rows`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Row {
    Request(Request),
    Skipped(Skipped),
}

impl Row {
    pub fn at(&self) -> DateTime<Utc> {
        match self {
            Row::Request(r) => r.at,
            Row::Skipped(s) => s.due,
        }
    }
}

/// Its oldest waiting case (`OldestWaiting`): tafs cloud's words, amber once past the promise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OldestWaiting {
    pub words: String,
    pub late: bool,
}

/// The work as tafs cloud sends it (`TafWork`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Work {
    /// The summary's line (`summary.line`).
    pub line: String,
    pub requests: Vec<Request>,
    pub skipped: Vec<Skipped>,
    pub health: Option<Health>,
    pub oldest_waiting: Option<OldestWaiting>,
    /// The owner's promise in whole hours (`promiseHours`), none when unset.
    pub promise_hours: Option<i64>,
}

/// A list the iPad decodes whole: one item it can't read and the list reads as none.
fn all<T>(v: Option<&Value>, read: fn(&Value) -> Option<T>) -> Vec<T> {
    v.and_then(Value::as_array)
        .and_then(|a| a.iter().map(read).collect::<Option<Vec<T>>>())
        .unwrap_or_default()
}

/// tafs cloud's answer; none when it isn't an object, as a failed decode.
pub fn read(body: &[u8]) -> Option<Work> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let o = v.as_object()?;
    let line = o
        .get("summary")
        .and_then(Value::as_object)
        .map(|s| string(s, "line"))
        .unwrap_or_default();
    let oldest_waiting = o
        .get("oldest_waiting")
        .and_then(Value::as_object)
        .map(|w| OldestWaiting {
            words: string(w, "words"),
            late: w.get("late").and_then(Value::as_bool).unwrap_or(false),
        })
        .filter(|w| !w.words.trim().is_empty());
    Some(Work {
        line,
        requests: all(o.get("requests"), Request::read),
        skipped: all(o.get("skipped"), Skipped::read),
        health: o.get("health").and_then(Health::read),
        oldest_waiting,
        promise_hours: o
            .get("promise")
            .and_then(|p| p.get("hours"))
            .and_then(Value::as_i64)
            .filter(|h| *h > 0),
    })
}

impl Work {
    /// The requests and skipped firings, each in its place by time, newest first.
    pub fn rows(&self) -> Vec<Row> {
        let mut rows: Vec<Row> = self
            .requests
            .iter()
            .cloned()
            .map(Row::Request)
            .chain(self.skipped.iter().cloned().map(Row::Skipped))
            .collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.at()));
        rows
    }
}

/// How a request's line is coloured: needing the owner amber, failed red, else soft.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Amber,
    No,
    Soft,
}

pub fn tone(state: &str) -> Tone {
    match state {
        "needs_you" => Tone::Amber,
        "failed" => Tone::No,
        _ => Tone::Soft,
    }
}

/// What the panel draws (`WorkPanel`): the work's line unless the health's first line says it,
/// the health (the work's, else Home's), the oldest case, and the rows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    pub line: Option<String>,
    pub health: Option<Health>,
    pub oldest_waiting: Option<OldestWaiting>,
    pub rows: Vec<Row>,
}

pub fn panel(work: &Work, home_health: Option<&Health>) -> Panel {
    let health = work.health.clone().or_else(|| home_health.cloned());
    let line = Some(work.line.clone())
        .filter(|l| !l.is_empty())
        .filter(|l| !health.as_ref().is_some_and(|h| h.repeats(l)));
    Panel {
        line,
        health,
        oldest_waiting: work.oldest_waiting.clone(),
        rows: work.rows(),
    }
}

/// `TafWork.promiseChoices`: Answer within's whole hours.
pub const PROMISE_CHOICES: [i64; 7] = [1, 2, 4, 8, 24, 48, 168];

/// `PromiseRow.label`: whole days as days (24 h is "1 day"), anything else in hours.
pub fn promise_label(
    w: &crate::words::WordBook,
    hours: i64,
) -> Result<String, crate::words::WordsError> {
    if hours >= 24 && hours % 24 == 0 {
        w.count("work.answer_within_days", hours / 24, &[])
    } else {
        w.count("work.answer_within_hours", hours, &[])
    }
}

/// `PromiseRow.value`: the promise, or Not set.
pub fn promise_value(
    w: &crate::words::WordBook,
    hours: Option<i64>,
) -> Result<String, crate::words::WordsError> {
    match hours {
        Some(h) => promise_label(w, h),
        None => w.line("work.answer_within_none"),
    }
}

/// `setPromise`'s path.
pub fn promise_path(taf: &str) -> String {
    format!("/v1/tafs/{}/promise", segment(taf))
}

/// `setPromise`'s body: the hours, or null to clear it.
pub fn promise_body(hours: Option<i64>) -> Vec<u8> {
    serde_json::json!({ "hours": hours })
        .to_string()
        .into_bytes()
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORK: &str = r#"{
      "summary": {"status": "idle", "line": "Nothing ran today", "needs_you": 1},
      "health": {"level": "failing", "lines": ["Nothing ran today, so nothing ran.", " "],
                 "fixes": [{"line": 0, "kind": "credits", "words": "Add credits"},
                           {"line": 0, "kind": "coach", "words": "Ask Coach"},
                           {"line": 0, "kind": "teleport", "words": "Go"}]},
      "oldest_waiting": {"since": "2026-11-16T03:00:00Z", "words": "Oldest case waiting 5 h", "late": true},
      "promise": {"hours": 4},
      "requests": [
        {"id": "r1", "at": "2026-11-16T05:00:00Z", "state": "needs_you", "line": "Waiting for you: refund 1042",
         "credits": {"credits": 34, "counted": true, "words": " 34 credits "},
         "deliveries": [{"place": "door_reply", "state": "given_up", "line": "The reply wasn't sent", "send_again": true}]},
        {"id": "r2", "at": "2026-11-16T03:30:00.250Z", "state": "failed", "line": "Stopped: the app didn't answer"}
      ],
      "skipped": [{"due": "2026-11-16T04:00:00Z", "why": "busy", "words": "Skipped at 05:00: still busy"}]
    }"#;

    #[test]
    fn the_work_reads_as_the_ipad_decodes_it() {
        let w = read(WORK.as_bytes()).unwrap();
        assert_eq!(w.line, "Nothing ran today");
        assert_eq!(w.promise_hours, Some(4));
        assert_eq!(
            w.oldest_waiting.as_ref().unwrap().words,
            "Oldest case waiting 5 h"
        );
        assert_eq!(w.requests[0].credits.as_deref(), Some("34 credits"));
        assert!(w.requests[0].deliveries[0].send_again);
        let h = w.health.as_ref().unwrap();
        assert_eq!(h.lines.len(), 1, "a blank line is none");
        // Coach with nothing to say, and a kind the app doesn't know, are no button.
        let fixes: Vec<_> = h.fixes_for(0).iter().map(|f| f.kind.as_str()).collect();
        assert_eq!(fixes, ["credits"]);
        // Newest first, the skipped firing in its place.
        let order: Vec<_> = w
            .rows()
            .iter()
            .map(|r| match r {
                Row::Request(r) => r.id.clone(),
                Row::Skipped(_) => "skipped".into(),
            })
            .collect();
        assert_eq!(order, ["r1", "skipped", "r2"]);
    }

    #[test]
    fn the_panel_says_the_line_once() {
        let w = read(WORK.as_bytes()).unwrap();
        // The health's first line starts with the work's: said once, with its colour.
        assert_eq!(panel(&w, None).line, None);
        let mut healthy = w.clone();
        healthy.health = None;
        assert_eq!(
            panel(&healthy, None).line.as_deref(),
            Some("Nothing ran today")
        );
        // No health with the work: Home's.
        let home = Health {
            level: "attention".into(),
            lines: vec!["Its app is signed out".into()],
            fixes: vec![],
        };
        assert_eq!(panel(&healthy, Some(&home)).health, Some(home));
    }

    #[test]
    fn a_request_carries_the_step_it_handed_its_owner() {
        let step = r#"{"handoff": "h-1", "run": "run-1", "what": "Upload the receipt", "why": "", "words": {"title": "Your step", "done": "Done", "skip": "Skip it"}}"#;
        let w = read(format!(r#"{{"requests": [{{"id": "r", "at": "2026-11-16T05:00:00Z", "state": "needs_you", "line": "Waiting for you", "your_step": {step}}}]}}"#).as_bytes()).unwrap();
        let s = w.requests[0].your_step.as_ref().unwrap();
        assert_eq!(
            (s.handoff.as_str(), s.what.as_str()),
            ("h-1", "Upload the receipt")
        );
        // One it can't read is none (`try?`), and the request stays.
        let w = read(br#"{"requests": [{"id": "r", "at": "2026-11-16T05:00:00Z", "your_step": {"what": "no hand-off"}}]}"#).unwrap();
        assert_eq!(w.requests.len(), 1);
        assert_eq!(w.requests[0].your_step, None);
    }

    #[test]
    fn what_doesnt_read_is_none() {
        // One request without its id: no requests, as the iPad's whole-list decode.
        let w = read(br#"{"requests": [{"at": "2026-11-16T05:00:00Z"}, {"id": "r", "at": "2026-11-16T05:00:00Z"}], "promise": {"hours": 0}, "oldest_waiting": {"words": "  "}}"#).unwrap();
        assert!(w.requests.is_empty());
        assert_eq!(w.promise_hours, None);
        assert_eq!(w.oldest_waiting, None);
        assert!(read(b"[]").is_none());
        assert_eq!(tone("needs_you"), Tone::Amber);
        assert_eq!(tone("failed"), Tone::No);
        assert_eq!(tone("done"), Tone::Soft);
        assert_eq!(path("taf/1"), "/v1/tafs/taf%2F1/work");
    }

    #[test]
    fn answer_within() {
        let w =
            crate::words::WordBook::load(&crate::words::WordBook::default_root(), "en").unwrap();
        assert_eq!(promise_value(&w, None).unwrap(), "Not set");
        assert_eq!(promise_label(&w, 1).unwrap(), "1 hour");
        assert_eq!(promise_label(&w, 8).unwrap(), "8 hours");
        assert_eq!(promise_label(&w, 24).unwrap(), "1 day");
        assert_eq!(promise_label(&w, 168).unwrap(), "7 days");
        assert_eq!(promise_label(&w, 36).unwrap(), "36 hours");
        assert_eq!(promise_path("t 1"), "/v1/tafs/t%201/promise");
        assert_eq!(promise_body(Some(24)), br#"{"hours":24}"#);
        assert_eq!(promise_body(None), br#"{"hours":null}"#);
    }
}
