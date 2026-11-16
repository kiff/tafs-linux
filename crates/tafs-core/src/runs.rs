//! A tafs cloud taf's runs (`RunsPanel`, `RetainedRuns`): when it runs next and Run now
//! (`POST /v1/tafs/{id}/run`), and the runs tafs cloud kept, a page at a time
//! (`GET /v1/tafs/{id}/work/runs`, `TafsCloud.RunsPage`), all in tafs cloud's words.

use serde_json::Value;

use crate::home::HomeTaf;
use crate::words::{WordBook, WordsError};
use crate::work::{segment, Request, Row, Skipped};

/// The panel's page holds 3 runs; the sheet with them all, 20 a page.
pub const PANEL: u32 = 3;
pub const ALL: u32 = 20;

/// A page of the runs, after `cursor`.
pub fn path(taf: &str, cursor: Option<&str>, limit: u32) -> String {
    let mut p = format!("/v1/tafs/{}/work/runs?limit={limit}", segment(taf));
    if let Some(c) = cursor {
        p.push_str("&cursor=");
        p.push_str(&segment(c));
    }
    p
}

pub fn run_path(taf: &str) -> String {
    format!("/v1/tafs/{}/run", segment(taf))
}

/// A page of the runs tafs cloud kept (`RunsPage`), as strictly as the iPad's decoder: a page
/// missing its title, or with a run it can't read, isn't one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunsPage {
    pub requests: Vec<Request>,
    pub next_cursor: Option<String>,
    pub title: String,
    pub more_title: String,
    pub end_words: Option<String>,
    /// On the first page: recent firings that started no run.
    pub skipped: Vec<Skipped>,
}

pub fn read_page(body: &[u8]) -> Option<RunsPage> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let o = v.as_object()?;
    let text = |k: &str| o.get(k).and_then(Value::as_str).map(str::to_string);
    // An optional field that's there must be what it says, as a synthesized decoder.
    let optional = |k: &str| -> Option<Option<String>> {
        match o.get(k) {
            None | Some(Value::Null) => Some(None),
            Some(Value::String(s)) => Some(Some(s.clone())),
            Some(_) => None,
        }
    };
    let skipped = match o.get("skipped") {
        None | Some(Value::Null) => vec![],
        Some(Value::Array(a)) => a.iter().map(Skipped::read).collect::<Option<_>>()?,
        Some(_) => return None,
    };
    Some(RunsPage {
        requests: o
            .get("requests")?
            .as_array()?
            .iter()
            .map(Request::read)
            .collect::<Option<_>>()?,
        next_cursor: optional("next_cursor")?,
        title: text("title")?,
        more_title: text("more_title")?,
        end_words: optional("end_words")?,
        skipped,
    })
}

impl RunsPage {
    /// The page's rows, newest first: its runs and, on the first page, the firings that started
    /// no run, each in its place by time (a tie keeps the runs first).
    pub fn rows(&self, first: bool) -> Vec<Row> {
        let mut rows: Vec<Row> = self.requests.iter().cloned().map(Row::Request).collect();
        if first {
            rows.extend(self.skipped.iter().cloned().map(Row::Skipped));
            // A stable sort keeps the original order on a tie, as the iPad's offset tiebreak.
            rows.sort_by_key(|r| std::cmp::Reverse(r.at()));
        }
        rows
    }
}

/// What Run now did (`RunNow`): the run's session, or none, with tafs cloud's words for why when
/// its check started no run. An older tafs cloud's bare `null` carries none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunNow {
    pub session: Option<String>,
    pub words: Option<String>,
}

pub fn read_run_now(body: &[u8]) -> Option<RunNow> {
    let v: Value = serde_json::from_slice(body).ok()?;
    if v.is_null() {
        return Some(RunNow {
            session: None,
            words: None,
        });
    }
    let o = v.as_object()?;
    let session = o.get("session").and_then(Value::as_str).map(str::to_string);
    let words = if session.is_none() {
        o.get("words")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|w| !w.is_empty())
            .map(str::to_string)
    } else {
        None
    };
    Some(RunNow { session, words })
}

/// The panel's line (`RunsPanel`): it works from its chat, it's paused, or tafs cloud's words
/// for its next run, else the app's own; `None` when it works from its chat, which has no Run now.
pub fn next_line(w: &WordBook, h: &HomeTaf) -> Result<(String, bool), WordsError> {
    if h.unscheduled == Some(true) {
        return Ok((w.line("taf.works_from_chat")?, false));
    }
    let line = if h.state == "paused" {
        w.line("taf.state.paused")?
    } else if let Some(words) = &h.next_run_words {
        words.clone()
    } else if let Some(t) = &h.next_run {
        w.get(
            "taf.next_run_at",
            &[("time", &crate::account_time::short_time(t))],
        )?
    } else {
        w.line("taf.on_its_schedule")?
    };
    Ok((line, true))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE: &str = r#"{"title": "Its runs", "more_title": "See all 12 runs", "next_cursor": "c/2",
      "requests": [
        {"id": "r2", "at": "2026-11-16T05:00:00Z", "state": "done", "line": "Done: 3 refunds"},
        {"id": "r1", "at": "2026-11-16T03:00:00Z", "state": "failed", "line": "Stopped"}],
      "skipped": [{"due": "2026-11-16T04:00:00Z", "why": "busy", "words": "Skipped: still busy"},
                  {"due": "2026-11-16T05:00:00Z", "why": "missed", "words": "Skipped at the same time"}]}"#;

    #[test]
    fn a_page_reads_as_strictly_as_the_ipad_decodes_it() {
        let p = read_page(PAGE.as_bytes()).unwrap();
        assert_eq!(p.title, "Its runs");
        assert_eq!(p.next_cursor.as_deref(), Some("c/2"));
        let ids: Vec<String> = p
            .rows(true)
            .iter()
            .map(|r| match r {
                Row::Request(r) => r.id.clone(),
                Row::Skipped(s) => s.words.clone(),
            })
            .collect();
        // Newest first; on a tie, the run first.
        assert_eq!(
            ids,
            [
                "r2",
                "Skipped at the same time",
                "Skipped: still busy",
                "r1"
            ]
        );
        // A later page draws its runs alone.
        assert_eq!(p.rows(false).len(), 2);
        // No title, a run without its id, or a cursor that isn't a string: no page.
        assert!(read_page(br#"{"requests": [], "more_title": "m"}"#).is_none());
        assert!(read_page(
            br#"{"title": "t", "more_title": "m", "requests": [{"at": "2026-11-16T05:00:00Z"}]}"#
        )
        .is_none());
        assert!(read_page(
            br#"{"title": "t", "more_title": "m", "requests": [], "next_cursor": 3}"#
        )
        .is_none());
        assert!(read_page(
            br#"{"title": "t", "more_title": "m", "requests": [], "next_cursor": null}"#
        )
        .is_some());
        assert_eq!(
            path("taf 1", Some("c/2"), 3),
            "/v1/tafs/taf%201/work/runs?limit=3&cursor=c%2F2"
        );
        assert_eq!(run_path("t"), "/v1/tafs/t/run");
    }

    #[test]
    fn run_now_says_why_only_when_nothing_started() {
        let r = read_run_now(br#"{"session": "s1", "words": "ignored"}"#).unwrap();
        assert_eq!((r.session.as_deref(), r.words), (Some("s1"), None));
        let r = read_run_now(br#"{"session": null, "words": " Nothing new since the last run. "}"#)
            .unwrap();
        assert_eq!(r.words.as_deref(), Some("Nothing new since the last run."));
        assert_eq!(read_run_now(b"null").unwrap().words, None);
        assert!(read_run_now(b"[]").is_none());
    }
}
