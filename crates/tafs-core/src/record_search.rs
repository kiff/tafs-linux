//! The Record searched across the person's tafs (`TafsCloud+Record.swift`, `RecordSearchBox`,
//! `RecordResults`, `RecordHitRow`, `RecordCaseSheet`): `GET /v1/record?q=` and one case,
//! `GET /v1/record/cases/{about}`. tafs cloud finds, words and decides; the app sends what's typed
//! or tapped, and draws what comes back.

use serde_json::{Map, Value};

use crate::activity::{ActivityEntry, NextStep};

/// A call found (`RecordHit`): a taf's Activity entry, which taf made it, and what it's about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordHit {
    pub entry: ActivityEntry,
    /// tafs cloud's id for the taf, and its name ("" when it sent none).
    pub taf_id: String,
    pub taf_name: String,
    /// What it's about, which opens its case; none when blank.
    pub about: Option<String>,
}

fn lenient_str(o: &Map<String, Value>, k: &str) -> Option<String> {
    o.get(k).and_then(Value::as_str).map(str::to_string)
}

fn hit(v: &Value) -> Option<RecordHit> {
    // The entry is decoded from the same object, as `ActivityEntry(from: decoder)`.
    let entry: ActivityEntry = serde_json::from_value(v.clone()).ok()?;
    let o = v.as_object()?;
    let taf = o.get("taf")?.as_object()?;
    let about = lenient_str(o, "about").filter(|a| !a.trim().is_empty());
    Some(RecordHit {
        entry,
        taf_id: taf.get("id")?.as_str()?.to_string(),
        taf_name: lenient_str(taf, "name").unwrap_or_default(),
        about,
    })
}

// `try c.decodeIfPresent([T].self) ?? []`: absent or null is none; anything else must all read.
fn strict_list<T>(
    o: &Map<String, Value>,
    k: &str,
    read: fn(&Value) -> Option<T>,
) -> Option<Vec<T>> {
    match o.get(k) {
        None | Some(Value::Null) => Some(vec![]),
        Some(Value::Array(a)) => a.iter().map(read).collect(),
        Some(_) => None,
    }
}

/// tafs cloud's group of hits (`RecordHitGroup`): its id required, its title and hits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HitGroup {
    pub id: String,
    pub title: String,
    pub entries: Vec<RecordHit>,
}

fn group(v: &Value) -> Option<HitGroup> {
    let o = v.as_object()?;
    Some(HitGroup {
        id: o.get("id")?.as_str()?.to_string(),
        title: lenient_str(o, "title").unwrap_or_default(),
        entries: strict_list(o, "entries", hit)?,
    })
}

/// One page of a search, newest first (`RecordSearchPage`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchPage {
    pub q: String,
    /// The search box's words.
    pub hint: String,
    pub entries: Vec<RecordHit>,
    pub groups: Vec<HitGroup>,
    pub next_cursor: Option<String>,
    pub end_words: Option<String>,
    /// "Nothing matched", in tafs cloud's words.
    pub none_words: Option<String>,
}

impl SearchPage {
    /// Read as the iPad's decoder: the words forgiven, the hits and groups required to read.
    pub fn read(body: &[u8]) -> Option<SearchPage> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let o = v.as_object()?;
        Some(SearchPage {
            q: lenient_str(o, "q").unwrap_or_default(),
            hint: lenient_str(o, "hint").unwrap_or_default(),
            entries: strict_list(o, "entries", hit)?,
            groups: strict_list(o, "groups", group)?,
            next_cursor: lenient_str(o, "next_cursor"),
            end_words: lenient_str(o, "end_words"),
            none_words: lenient_str(o, "none_words"),
        })
    }

    /// `shownGroups`: tafs cloud's groups, or every entry as one when it sent none.
    pub fn shown_groups(&self) -> Vec<HitGroup> {
        if self.groups.is_empty() && !self.entries.is_empty() {
            vec![HitGroup {
                id: "all".into(),
                title: String::new(),
                entries: self.entries.clone(),
            }]
        } else {
            self.groups.clone()
        }
    }

    /// `isEmpty`.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty() && self.groups.iter().all(|g| g.entries.is_empty())
    }
}

/// A search as it stands (`RecordSearch`): what was asked, the pages read, and whether more come.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Search {
    pub q: String,
    pub pages: Vec<SearchPage>,
    pub loading: bool,
    pub error: Option<String>,
}

impl Search {
    pub fn next_cursor(&self) -> Option<&str> {
        self.pages.last().and_then(|p| p.next_cursor.as_deref())
    }

    /// "Nothing matched": every page read is empty, in the last page's words.
    pub fn none_words(&self) -> Option<&str> {
        if !self.pages.is_empty() && self.pages.iter().all(SearchPage::is_empty) {
            self.pages.last().and_then(|p| p.none_words.as_deref())
        } else {
            None
        }
    }

    /// The end's words, once no page follows and none is loading.
    pub fn end_words(&self) -> Option<&str> {
        if self.loading || self.next_cursor().is_some() {
            return None;
        }
        self.pages.last().and_then(|p| p.end_words.as_deref())
    }
}

/// One case (`RecordCase`): every call about the same thing, oldest first, and the latest step
/// that didn't go through, in tafs cloud's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordCase {
    pub about: String,
    pub steps: Vec<RecordHit>,
    pub failed: Option<Failed>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failed {
    pub id: String,
    pub words: Option<String>,
    pub next: Option<NextStep>,
}

impl RecordCase {
    /// Read as the iPad's decoder: `about` required, the steps required to read, `failed` forgiven.
    pub fn read(body: &[u8]) -> Option<RecordCase> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let o = v.as_object()?;
        let failed = o.get("failed").and_then(Value::as_object).and_then(|f| {
            Some(Failed {
                id: f.get("id")?.as_str()?.to_string(),
                words: lenient_str(f, "words"),
                next: f
                    .get("next")
                    .and_then(|n| serde_json::from_value(n.clone()).ok()),
            })
        });
        Some(RecordCase {
            about: o.get("about")?.as_str()?.to_string(),
            steps: strict_list(o, "steps", hit)?,
            failed,
        })
    }

    /// The step that didn't go through, named first in red; none when tafs cloud sent no words.
    pub fn failed_words(&self) -> Option<&str> {
        self.failed
            .as_ref()
            .and_then(|f| f.words.as_deref())
            .filter(|w| !w.trim().is_empty())
    }
}

// What a query keeps as it is: letters and digits; everything else is percent-encoded.
fn encode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// `searchRecord`'s path: what was typed, the cursor that continues the same search, a limit.
pub fn search_path(q: &str, cursor: Option<&str>, limit: Option<u32>) -> String {
    let mut query = vec![format!("q={}", encode(q))];
    if let Some(c) = cursor {
        query.push(format!("cursor={}", encode(c)));
    }
    if let Some(l) = limit {
        query.push(format!("limit={l}"));
    }
    format!("/v1/record?{}", query.join("&"))
}

/// One case's path, its `about` escaped as one segment.
pub fn case_path(about: &str) -> String {
    format!("/v1/record/cases/{}", crate::access::escaped(about))
}

/// What's asked: typed words trimmed; blank is no search, and the Record shows again.
pub fn asked(typed: &str) -> Option<String> {
    let t = typed.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// How long the box waits after the last key before it searches (`recordTypingPause`).
pub const TYPING_PAUSE: std::time::Duration = std::time::Duration::from_millis(400);
