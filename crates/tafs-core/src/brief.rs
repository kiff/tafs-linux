//! "Your brief" (Missions design v2.1 §7), read from tafs cloud's `GET /v1/brief?label=` as the
//! iPad reads it (`TafsCloud+Brief.swift` › `Brief`), and what the iPad's model decides about it
//! (`AppModel`: `loadBrief`, `briefDecisions`, `needsYouBeyondBrief`, `destination(of:answer:)`,
//! `laterBrief`). Every word, count, order and urgency is tafs cloud's; the app draws it and
//! decides nothing.
//!
//! As lenient as the iPad's decoders: the brief fails only when it isn't an object; a part the
//! app can't read is left out, never the brief with it. A heading tafs cloud didn't send (or sent
//! blank) is the shared catalog's (`brief.*`), so a brief never shows a blank heading.
//!
//! The labels (`Brief.Label`, docs/labels.md) are Home's too: `LabelList`, `LabelIDs` and
//! `LabelsWords` read them there, and `chips` lays out the row the brief and the Overview share
//! (`LabelChips`).

use serde_json::{Map, Value};

use crate::words::{WordBook, WordsError};

/// A line of text, trimmed; none when absent, blank or of another shape (`Brief.text`).
pub(crate) fn text(o: &Map<String, Value>, key: &str) -> Option<String> {
    let t = o.get(key)?.as_str()?.trim();
    (!t.is_empty()).then(|| t.to_string())
}

/// `try? decodeIfPresent(Bool.self)`: a JSON bool, anything else none.
fn flag(o: &Map<String, Value>, key: &str) -> Option<bool> {
    o.get(key)?.as_bool()
}

/// `try? decodeIfPresent(Int.self)`: a whole number, anything else none.
fn whole(o: &Map<String, Value>, key: &str) -> Option<i64> {
    let v = o.get(key)?;
    v.as_i64().or_else(|| {
        v.as_f64()
            .filter(|f| f.fract() == 0.0 && f.abs() < 9.0e15)
            .map(|f| f as i64)
    })
}

/// `{words}` as its own small struct (`LaterWords`, `CreditWords`): the words as sent, untrimmed;
/// none when blank, absent or of another shape.
fn words_of(o: &Map<String, Value>, key: &str) -> Option<String> {
    let w = o.get(key)?.as_object()?.get("words")?.as_str()?;
    (!w.is_empty()).then(|| w.to_string())
}

/// A list read item by item (`Brief.list`): one the app can't read is left out, never the list;
/// another shape is none.
fn list<T>(o: &Map<String, Value>, key: &str, read: impl Fn(&Value) -> T) -> Vec<T> {
    o.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().map(read).collect())
        .unwrap_or_default()
}

/// The brief's headings, in the person's language (`Brief.Words`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Words {
    pub tab: String,
    pub title: String,
    pub when: String,
    pub decisions: String,
    pub done: String,
    pub next_move: String,
    pub recommends: String,
    pub meanwhile: String,
    pub evidence: String,
    pub all: String,
    pub urgent: String,
}

impl Words {
    /// Every heading from the shared catalog (`brief.*`): for a brief tafs cloud sent no words for.
    pub fn shared(w: &WordBook) -> Result<Words, WordsError> {
        let title = w.line("brief.title")?;
        Ok(Words {
            tab: title.clone(),
            title,
            when: String::new(),
            decisions: w.line("brief.decisions")?,
            done: w.line("brief.done")?,
            next_move: w.line("brief.next_move")?,
            recommends: w.line("brief.recommends")?,
            meanwhile: w.line("brief.meanwhile")?,
            evidence: w.line("brief.evidence")?,
            all: String::new(),
            urgent: w.line("brief.urgent")?,
        })
    }

    /// tafs cloud's words first; a heading it didn't send is the shared catalog's. Another shape
    /// than an object is as none sent.
    fn read(v: Option<&Value>, shared: &Words) -> Words {
        let Some(c) = v.and_then(Value::as_object) else {
            return shared.clone();
        };
        let or = |k: &str, s: &str| text(c, k).unwrap_or_else(|| s.to_string());
        let title = or("title", &shared.title);
        Words {
            // tafs cloud's route sends the tab's name as `title`.
            tab: or("tab", &title),
            when: text(c, "when").unwrap_or_default(),
            decisions: or("decisions", &shared.decisions),
            done: or("done", &shared.done),
            next_move: or("next_move", &shared.next_move),
            recommends: or("recommends", &shared.recommends),
            meanwhile: or("meanwhile", &shared.meanwhile),
            evidence: or("evidence", &shared.evidence),
            all: text(c, "all").unwrap_or_default(),
            urgent: or("urgent", &shared.urgent),
            title,
        }
    }
}

/// What a decision is about (`Brief.Ref`): a held call, a step, a connection, a proposal, or a
/// taf whose check keeps failing.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ref {
    pub kind: String,
    pub id: String,
    /// tafs cloud's id for its taf.
    pub taf: String,
}

impl Ref {
    fn read(v: Option<&Value>) -> Ref {
        let Some(c) = v.and_then(Value::as_object) else {
            return Ref::default();
        };
        Ref {
            kind: text(c, "kind").unwrap_or_default(),
            id: text(c, "id").unwrap_or_default(),
            taf: text(c, "taf").unwrap_or_default(),
        }
    }
}

/// One answer to a decision (`Brief.Answer`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Answer {
    pub words: String,
    pub answer: String,
    /// At most one, Coach's pick; with no `why`, the answers show as equals.
    pub recommended: bool,
    /// A decline ("Don't send", "Not now"): drawn as one, never read as a no by the app.
    pub decline: bool,
}

impl Answer {
    fn read(v: &Value) -> Answer {
        let Some(c) = v.as_object() else {
            return Answer::default();
        };
        Answer {
            words: text(c, "words").unwrap_or_default(),
            answer: text(c, "answer").unwrap_or_default(),
            recommended: flag(c, "recommended").unwrap_or(false),
            decline: flag(c, "decline").unwrap_or(false),
        }
    }
}

/// One decision (`Brief.Decision`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Decision {
    pub id: String,
    pub question: String,
    pub answers: Vec<Answer>,
    /// Why Coach recommends its answer; none, and no answer is drawn as recommended.
    pub why: Option<String>,
    pub meanwhile: String,
    /// When it must be decided by, in words ("by Fri 12:00").
    pub deadline: Option<String>,
    pub urgent: bool,
    /// "Later" moves it to the next brief; left out when it can't be put off.
    pub later: Option<String>,
    /// Why it can't wait, when `later` is left out.
    pub cant_wait: Option<String>,
    /// A line under the answers.
    pub note: Option<String>,
    pub r#ref: Ref,
}

impl Decision {
    fn read(v: &Value) -> Decision {
        let Some(c) = v.as_object() else {
            return Decision::default();
        };
        Decision {
            id: text(c, "id").unwrap_or_default(),
            question: text(c, "question").unwrap_or_default(),
            answers: list(c, "answers", Answer::read)
                .into_iter()
                .filter(|a| !a.words.is_empty())
                .collect(),
            why: text(c, "why"),
            meanwhile: text(c, "meanwhile").unwrap_or_default(),
            deadline: text(c, "deadline"),
            urgent: flag(c, "urgent").unwrap_or(false),
            later: words_of(c, "later"),
            // tafs cloud's route says why it can't wait as `now_why`.
            cant_wait: text(c, "cant_wait").or_else(|| text(c, "now_why")),
            note: text(c, "note"),
            r#ref: Ref::read(c.get("ref")),
        }
    }

    /// Coach's pick, drawn first and filled; only with its why.
    pub fn recommended(&self) -> Option<&Answer> {
        self.why.as_ref()?;
        self.answers.iter().find(|a| a.recommended)
    }

    /// The answers beside the pick, in tafs cloud's order (`others`).
    pub fn others(&self) -> Vec<&Answer> {
        let pick = self.recommended();
        self.answers.iter().filter(|a| Some(*a) != pick).collect()
    }

    /// Later, as the card draws it: an urgent decision can't be put off, whatever was sent.
    pub fn later_shown(&self) -> Option<&str> {
        if self.urgent {
            None
        } else {
            self.later.as_deref()
        }
    }

    /// One the app can draw: a question and at least one answer.
    pub fn usable(&self) -> bool {
        !self.id.is_empty() && !self.question.is_empty() && !self.answers.is_empty()
    }
}

/// One item a job works on, in tafs cloud's line (`Brief.Case`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Case {
    pub name: String,
    pub words: String,
    pub state: String,
    pub next_check: Option<String>,
}

impl Case {
    fn read(v: &Value) -> Case {
        let Some(c) = v.as_object() else {
            return Case::default();
        };
        Case {
            name: text(c, "name").unwrap_or_default(),
            words: text(c, "words").unwrap_or_default(),
            state: text(c, "state").unwrap_or_default(),
            next_check: text(c, "next_check"),
        }
    }

    /// Closed only when the source says so.
    pub fn closed(&self) -> bool {
        self.state == "closed"
    }

    pub fn waiting(&self) -> bool {
        self.state == "waiting"
    }
}

/// What got done, a mission's cases (`Brief.Done`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Done {
    pub mission: Option<String>,
    pub cases: Vec<Case>,
    /// What it's worth, in words ("€3,400 due back").
    pub value: Option<String>,
    /// The mission whose log holds the evidence.
    pub log: Option<String>,
}

impl Done {
    fn read(v: &Value) -> Done {
        let Some(c) = v.as_object() else {
            return Done::default();
        };
        Done {
            mission: text(c, "mission"),
            cases: list(c, "cases", Case::read)
                .into_iter()
                .filter(|x| !x.words.is_empty() || !x.name.is_empty())
                .collect(),
            value: text(c, "value"),
            log: text(c, "log"),
        }
    }
}

/// One next move (`Brief.NextMove`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NextMove {
    pub words: String,
    pub start: String,
    pub dismiss: String,
    pub cases: Vec<String>,
}

impl NextMove {
    fn read(v: &Value) -> NextMove {
        let Some(c) = v.as_object() else {
            return NextMove::default();
        };
        NextMove {
            words: text(c, "words").unwrap_or_default(),
            start: text(c, "start").unwrap_or_default(),
            dismiss: text(c, "dismiss").unwrap_or_default(),
            // `[String]` as a whole: one of another shape is none at all.
            cases: c
                .get("cases")
                .and_then(Value::as_array)
                .and_then(|a| {
                    a.iter()
                        .map(|x| x.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()
                })
                .unwrap_or_default(),
        }
    }

    pub fn usable(&self) -> bool {
        !self.words.is_empty() && !self.start.is_empty() && !self.dismiss.is_empty()
    }
}

/// A count and its words: "more" decisions, the "rest" that need nothing now (`Brief.Counted`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Counted {
    pub count: i64,
    pub words: String,
}

impl Counted {
    fn read(v: &Value) -> Counted {
        let Some(c) = v.as_object() else {
            return Counted::default();
        };
        Counted {
            count: whole(c, "count").unwrap_or(0),
            words: text(c, "words").unwrap_or_default(),
        }
    }
}

/// A label chip, as Home and the brief send it (`Brief.Label`): its name in the person's
/// language, its colour as one of Explore's category tokens, how many listed tafs carry it, how
/// many of those wait for the person (the amber dot above 0), and Home's line filtered to it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Label {
    pub id: String,
    pub name: String,
    pub color: String,
    pub count: i64,
    pub waiting: i64,
    pub headline: Option<String>,
}

impl Label {
    pub fn read(v: &Value) -> Label {
        let Some(c) = v.as_object() else {
            return Label::default();
        };
        Label {
            id: text(c, "id").unwrap_or_default(),
            name: text(c, "name").unwrap_or_default(),
            color: text(c, "color").unwrap_or_default(),
            count: whole(c, "count").unwrap_or(0),
            // A count; a yes/no from an earlier draft reads as one waiting.
            waiting: whole(c, "waiting")
                .unwrap_or_else(|| i64::from(flag(c, "waiting") == Some(true))),
            headline: c
                .get("headline")
                .and_then(Value::as_object)
                .and_then(|h| h.get("words"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
        }
    }

    /// One the app can draw and filter by.
    pub fn usable(&self) -> bool {
        !self.id.is_empty() && !self.name.is_empty()
    }
}

/// "Your brief" (`TafsCloud.Brief`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Brief {
    pub at: Option<String>,
    pub next_at: Option<String>,
    pub decisions: Vec<Decision>,
    pub more: Option<Counted>,
    pub done: Vec<Done>,
    pub next_move: Option<NextMove>,
    pub rest: Option<Counted>,
    pub credits: Option<String>,
    pub labels: Vec<Label>,
    pub words: Words,
}

impl Brief {
    /// The route's path: the brief, filtered to a label (`brief(label:)`).
    pub fn path(label: Option<&str>) -> String {
        match label {
            Some(l) => format!("/v1/brief?label={}", encode(l, b"")),
            None => "/v1/brief".into(),
        }
    }

    /// tafs cloud's answer, read as the iPad reads it; none only when it isn't an object (or
    /// isn't JSON). `w` gives the shared catalog's headings.
    pub fn read(body: &[u8], w: &WordBook) -> Result<Option<Brief>, WordsError> {
        let shared = Words::shared(w)?;
        let Ok(Value::Object(c)) = serde_json::from_slice::<Value>(body) else {
            return Ok(None);
        };
        Ok(Some(Brief {
            at: text(&c, "at"),
            next_at: text(&c, "next_at"),
            decisions: list(&c, "decisions", Decision::read)
                .into_iter()
                .filter(Decision::usable)
                .collect(),
            more: c
                .get("more")
                .map(Counted::read)
                .filter(|m| m.count > 0 && !m.words.is_empty()),
            done: list(&c, "done", Done::read)
                .into_iter()
                .filter(|d| !d.cases.is_empty())
                .collect(),
            next_move: c
                .get("next_move")
                .map(NextMove::read)
                .filter(NextMove::usable),
            rest: c
                .get("rest")
                .map(Counted::read)
                .filter(|r| !r.words.is_empty()),
            credits: words_of(&c, "credits"),
            labels: list(&c, "labels", Label::read)
                .into_iter()
                .filter(Label::usable)
                .collect(),
            // No `words` sent at all: every heading is the shared catalog's, as for one left out.
            words: Words::read(c.get("words"), &shared),
        }))
    }

    /// `loadBrief`'s switch: a brief with nothing in it (no decision, nothing done, no next move)
    /// is no brief, and Needs you is as today.
    pub fn shown(self) -> Option<Brief> {
        (!(self.decisions.is_empty() && self.done.is_empty() && self.next_move.is_none()))
            .then_some(self)
    }

    /// `briefDecisions`: the decisions still to answer here, those set aside left out.
    pub fn open_decisions<'a>(
        &'a self,
        set_aside: &'a std::collections::BTreeSet<String>,
    ) -> impl Iterator<Item = &'a Decision> + 'a {
        self.decisions
            .iter()
            .filter(move |d| !set_aside.contains(&d.id))
    }
}

/// `needsYouBeyondBrief`: how much of Needs you the brief doesn't show, matched item by item,
/// never by number: a held call or a step the brief shows is matched by its `ref`
/// (`hold:<id>`, `step:<handoff>`), and anything the brief can't carry (`others`) counts.
pub fn needs_you_beyond_brief(items: &[String], others: i64, brief: &[&Decision]) -> i64 {
    let shown: std::collections::BTreeSet<String> = brief
        .iter()
        .map(|d| format!("{}:{}", d.r#ref.kind, d.r#ref.id))
        .collect();
    items.iter().filter(|i| !shown.contains(*i)).count() as i64 + others
}

/// `briefNeedsYouLine`: Needs you's count of what the brief doesn't show, under the brief;
/// none when the brief shows it all.
pub fn needs_you_line(
    w: &WordBook,
    items: &[String],
    others: i64,
    brief: &[&Decision],
) -> Result<Option<String>, WordsError> {
    let beyond = needs_you_beyond_brief(items, others, brief);
    if beyond > 0 {
        w.count("needs_you.count", beyond, &[]).map(Some)
    } else {
        Ok(None)
    }
}

/// Where a decision's answer opens on the iPad (`destination(of:answer:)`): the route it already
/// has, where its own answer is given. Nothing is answered from the brief.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Destination {
    /// A held call, on KIFF's own card with no answer chosen: signed there.
    Hold(String),
    /// A step a taf handed its owner, on Needs you: Done and Skip it are signed.
    Step(String),
    /// A taf whose check keeps failing: its page, by tafs cloud's id ("Open the taf").
    Taf(String),
    /// Needs you, for anything else.
    NeedsYou,
}

impl Destination {
    pub fn of(decision: &Decision) -> Destination {
        let r = &decision.r#ref;
        if r.kind == "check" {
            return if r.taf.is_empty() {
                Destination::NeedsYou
            } else {
                Destination::Taf(r.taf.clone())
            };
        }
        if r.id.is_empty() {
            return Destination::NeedsYou;
        }
        match r.kind.as_str() {
            "hold" => Destination::Hold(r.id.clone()),
            "step" => Destination::Step(r.id.clone()),
            _ => Destination::NeedsYou,
        }
    }

    /// Answered with a signature on the iPad (KIFF's card behind Face ID, a step's Done or Skip
    /// it): this computer can't, so `ApprovalGate`'s words say why.
    pub fn signed(&self) -> bool {
        matches!(self, Destination::Hold(_) | Destination::Step(_))
    }
}

/// "Later" (`briefLater`): the decision moves to the next brief. Its id ("hold:ce_1") is kept as
/// written: only what a path segment can't hold is escaped, "/" included. None for a decision
/// that can't be put off (`laterBrief`'s guard: no Later, or urgent).
pub fn later_path(decision: &Decision) -> Option<String> {
    decision.later_shown()?;
    Some(format!(
        "/v1/brief/decisions/{}/later",
        encode(&decision.id, b"-._~!$&'()*+,;=:@")
    ))
}

/// Percent-encodes all but ASCII letters and digits and the bytes `keep`.
pub(crate) fn encode(s: &str, keep: &[u8]) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || keep.contains(&b) {
            out.push(b as char);
        } else {
            out.push_str(&format!("%{b:02X}"));
        }
    }
    out
}

/// One chip of a row (`LabelChip`): its words, its count, the amber dot, chosen or not, and what
/// a tap does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chip {
    pub words: String,
    pub count: Option<i64>,
    pub waiting: bool,
    pub selected: bool,
    pub tap: ChipTap,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChipTap {
    /// Filter to this label, or none (All).
    Choose(Option<String>),
    /// "+N": show them all.
    Expand,
}

/// `LabelChips`: All, then two labels, then "+N" for the rest, which shows them all once tapped.
/// The chosen one is inverted; tapped again, it goes back to All.
pub fn chips(labels: &[Label], selected: Option<&str>, all: &str, expanded: bool) -> Vec<Chip> {
    let shown = if expanded {
        labels
    } else {
        &labels[..labels.len().min(2)]
    };
    let hidden = labels.len() - shown.len();
    let mut out = vec![Chip {
        words: all.to_string(),
        count: None,
        waiting: false,
        selected: selected.is_none(),
        tap: ChipTap::Choose(None),
    }];
    for l in shown {
        let chosen = selected == Some(l.id.as_str());
        out.push(Chip {
            words: l.name.clone(),
            count: Some(l.count),
            waiting: l.waiting > 0,
            selected: chosen,
            tap: ChipTap::Choose((!chosen).then(|| l.id.clone())),
        });
    }
    if hidden > 0 {
        out.push(Chip {
            words: format!("+{hidden}"),
            count: None,
            waiting: false,
            selected: false,
            tap: ChipTap::Expand,
        });
    }
    out
}

/// `TafLabels`: a taf's labels as small grey chips, two, then "+N"; none without labels.
pub fn taf_chips(names: &[String]) -> Vec<String> {
    let mut out: Vec<String> = names.iter().take(2).cloned().collect();
    if names.len() > 2 {
        out.push(format!("+{}", names.len() - 2));
    }
    out
}

/// The first chip's word (`labelsAllWords`): Home's, the brief's, or the app's own where neither
/// sent one.
pub fn all_words(
    w: &WordBook,
    home: Option<&str>,
    brief: Option<&Brief>,
) -> Result<String, WordsError> {
    if let Some(h) = home {
        return Ok(h.to_string());
    }
    if let Some(b) = brief.filter(|b| !b.words.all.is_empty()) {
        return Ok(b.words.all.clone());
    }
    w.line("home.labels_all")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").expect("the words at the pin")
    }

    fn brief(json: &str) -> Brief {
        Brief::read(json.as_bytes(), &en()).unwrap().unwrap()
    }

    const LIVE: &str = r#"{"decisions":[{"id":"d1","question":"Pay 3 supplier bills due Friday, €1,240?","why":"All three match their orders.","meanwhile":"these three wait.",
      "answers":[{"words":"Pay the 3","answer":"approve","recommended":true},{"words":"Not now","answer":"refuse","decline":true}],
      "later":{"words":"Later"},"ref":{"kind":"hold","id":"ce_1","taf":"depl_1"}}]}"#;

    /// The route's own shape: the tab's name in `words.title`, why a decision can't wait in
    /// `now_why`, and a held call's and a step's `ref`.
    #[test]
    fn the_routes_shape_as_tafs_cloud_serves_it() {
        let b = brief(
            r#"{"at":"2026-11-16T07:00:00Z","next_at":"2026-11-16T17:00:00Z",
         "decisions":[{"id":"hold:ce_1","question":"Refund order 4502, €480.00?","answers":[{"words":"Approve","answer":"approve","recommended":false},{"words":"Refuse","answer":"refuse","recommended":false}],
                       "meanwhile":"Nothing is sent.","urgent":true,"now_why":"It lapses at 08:30, before the next brief.","ref":{"kind":"hold","id":"ce_1","taf":"depl_1"}},
                      {"id":"step:h1","question":"Sign the delivery note","answers":[{"words":"Done","answer":"done","recommended":false}],
                       "meanwhile":"Rae waits.","urgent":false,"later":{"words":"Later"},"ref":{"kind":"step","id":"h1","taf":"depl_1"}}],
         "done":[],"rest":{"count":3,"words":"3 things wait that need nothing from you now"},"labels":[],
         "words":{"title":"Your brief","decisions":"Decisions · 2","done":"Done","rest":"The rest"}}"#,
        );
        assert_eq!(b.words.tab, "Your brief", "the tab's name, from `title`");
        let ids: Vec<_> = b.decisions.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(ids, ["hold:ce_1", "step:h1"]);
        assert_eq!(
            b.decisions[0].cant_wait.as_deref(),
            Some("It lapses at 08:30, before the next brief.")
        );
        assert_eq!(b.decisions[0].later, None, "urgent: no Later");
        assert_eq!(b.decisions[1].later.as_deref(), Some("Later"));
        assert_eq!(
            Destination::of(&b.decisions[0]),
            Destination::Hold("ce_1".into())
        );
        assert_eq!(
            Destination::of(&b.decisions[1]),
            Destination::Step("h1".into())
        );
        assert_eq!(b.rest.as_ref().map(|r| r.count), Some(3));
    }

    #[test]
    fn the_brief_reads_v21s_fields_as_sent() {
        let b = brief(
            r#"{"at":"2026-11-14T08:00:00+01:00","next_at":"2026-11-14T18:00:00+01:00",
         "decisions":[{"id":"d1","question":"Pay 3 supplier bills due Friday, €1,240?",
                       "answers":[{"words":"Pay the 3","answer":"approve","recommended":true},{"words":"Not now","answer":"reject","decline":true}],
                       "why":"All three match their orders.","meanwhile":"these three wait.","deadline":"by Fri 12:00","urgent":false,
                       "later":{"words":"Later"},"ref":{"kind":"hold","id":"h1","taf":"t1"}},
                      {"id":"d2","question":"Pay Mercier €2,100 before 12:00?","answers":[{"words":"Pay €2,100","answer":"approve","recommended":true}],
                       "why":"It matches order 0388.","meanwhile":"held.","urgent":true,"cant_wait":"This can't wait: it expires at 12:00.","ref":{"kind":"hold","id":"h2","taf":"t1"}},
                      {"id":"d3","question":"","answers":[]}, 7],
         "more":{"count":2,"words":"2 more decisions"},
         "done":[{"mission":"Money","cases":[{"name":"Brio Bikes, invoice 0139","words":"paid Thursday · closed","state":"closed"},{"words":"x","state":"waiting","next_check":"Friday"}],"value":"€3,400 due back","log":"m1"},{"cases":[]}],
         "next_move":{"words":"Prepare an escalation summary?","cases":["0142"],"start":"Start","dismiss":"Dismiss"},
         "rest":{"count":14,"words":"14 things wait that need nothing from you now"},
         "credits":{"words":"Today: 38 credits."},
         "labels":[{"id":"money","name":"Money","color":2,"count":5,"waiting":true},{"id":"","name":"x"}],
         "words":{"tab":"Your brief","title":"Your brief","decisions":"Decisions · 2","next_move":"Next move","recommends":"Coach recommends"}}"#,
        );
        let ids: Vec<_> = b.decisions.iter().map(|d| d.id.as_str()).collect();
        assert_eq!(
            ids,
            ["d1", "d2"],
            "an empty question, no answers, or another shape: left out"
        );
        let d1 = &b.decisions[0];
        assert_eq!(
            d1.recommended().map(|a| a.words.as_str()),
            Some("Pay the 3")
        );
        assert!(d1.answers.last().unwrap().decline);
        assert_eq!(d1.later.as_deref(), Some("Later"));
        assert_eq!(
            d1.r#ref,
            Ref {
                kind: "hold".into(),
                id: "h1".into(),
                taf: "t1".into()
            }
        );
        let others: Vec<_> = d1.others().iter().map(|a| a.words.as_str()).collect();
        assert_eq!(others, ["Not now"]);
        let d2 = &b.decisions[1];
        assert!(d2.urgent);
        assert_eq!(d2.later, None, "it can't be put off");
        assert_eq!(
            d2.cant_wait.as_deref(),
            Some("This can't wait: it expires at 12:00.")
        );
        assert_eq!(
            b.more.as_ref().map(|m| m.words.as_str()),
            Some("2 more decisions")
        );
        assert_eq!(b.done.len(), 1, "a group with no cases: left out");
        assert!(b.done[0].cases[0].closed());
        assert!(b.done[0].cases[1].waiting());
        assert_eq!(b.done[0].cases[1].next_check.as_deref(), Some("Friday"));
        assert_eq!(b.done[0].log.as_deref(), Some("m1"));
        let m = b.next_move.as_ref().unwrap();
        assert_eq!(m.start, "Start");
        assert_eq!(m.cases, ["0142"]);
        assert_eq!(b.rest.as_ref().map(|r| r.count), Some(14));
        assert_eq!(b.credits.as_deref(), Some("Today: 38 credits."));
        assert_eq!(b.labels.len(), 1);
        assert_eq!(b.labels[0].id, "money");
        assert_eq!(b.labels[0].color, "", "a colour of another shape is none");
        assert_eq!(b.labels[0].waiting, 1, "a yes reads as one waiting");
        assert_eq!(b.words.next_move, "Next move");
        assert_eq!(b.at.as_deref(), Some("2026-11-14T08:00:00+01:00"));
    }

    /// With no why, no answer is drawn as recommended: they show as equals.
    #[test]
    fn a_recommendation_needs_its_why() {
        let b = brief(
            r#"{"decisions":[{"id":"d","question":"q","answers":[{"words":"Yes","answer":"approve","recommended":true},{"words":"No","answer":"reject"}],"meanwhile":"m","ref":{}}]}"#,
        );
        assert_eq!(b.decisions[0].recommended(), None);
        assert_eq!(b.decisions[0].others().len(), 2, "both as equals");
    }

    /// A heading tafs cloud didn't send is the shared catalog's (`brief.*`), never blank; one it
    /// sent wins.
    #[test]
    fn a_heading_tafs_cloud_didnt_send_is_the_shared_catalogs() {
        let w = en();
        let shared = Words::shared(&w).unwrap();
        let bare = brief(r#"{"decisions":[]}"#);
        assert_eq!(bare.words.title, w.line("brief.title").unwrap());
        assert_eq!(bare.words.tab, bare.words.title, "the tab is the title");
        assert_eq!(bare.words.decisions, w.line("brief.decisions").unwrap());
        assert_eq!(bare.words.done, w.line("brief.done").unwrap());
        assert_eq!(bare.words.next_move, w.line("brief.next_move").unwrap());
        assert_eq!(bare.words.recommends, w.line("brief.recommends").unwrap());
        assert_eq!(bare.words.meanwhile, w.line("brief.meanwhile").unwrap());
        assert_eq!(bare.words.evidence, w.line("brief.evidence").unwrap());
        assert_eq!(bare.words.urgent, w.line("brief.urgent").unwrap());
        assert_eq!(
            bare.words.when, "",
            "no shared word for when: tafs cloud's or nothing"
        );
        assert_eq!(
            brief(r#"{"decisions":[],"words":"Your brief"}"#).words,
            shared,
            "words of another shape are as none"
        );
        let sent = brief(
            r#"{"decisions":[],"words":{"title":"Morning brief","decisions":"Decisions · 2","urgent":" "}}"#,
        );
        assert_eq!(sent.words.title, "Morning brief");
        assert_eq!(sent.words.tab, "Morning brief");
        assert_eq!(sent.words.decisions, "Decisions · 2");
        assert_eq!(
            sent.words.urgent,
            w.line("brief.urgent").unwrap(),
            "a blank word counts as not sent"
        );
        assert_eq!(sent.words.done, w.line("brief.done").unwrap());
    }

    #[test]
    fn only_an_object_is_a_brief_and_an_empty_one_is_none() {
        let w = en();
        assert_eq!(Brief::read(b"[]", &w).unwrap(), None);
        assert_eq!(Brief::read(b"not json", &w).unwrap(), None);
        assert_eq!(brief("{}").shown(), None, "nothing in it: Needs you");
        assert!(brief(LIVE).shown().is_some());
        let only_done = brief(r#"{"done":[{"cases":[{"words":"paid"}]}]}"#);
        assert!(only_done.shown().is_some());
        let only_move = brief(r#"{"next_move":{"words":"w","start":"s","dismiss":"d"}}"#);
        assert!(only_move.shown().is_some());
        let half_move = brief(r#"{"next_move":{"words":"w","start":"s"}}"#);
        assert_eq!(half_move.next_move, None, "a move needs all three words");
        let odd = brief(
            r#"{"decisions":{},"more":{"count":0,"words":"x"},"rest":{"count":2},"credits":"x","labels":"x","done":7,"next_move":[]}"#,
        );
        assert!(odd.decisions.is_empty() && odd.done.is_empty() && odd.labels.is_empty());
        assert_eq!(odd.more, None, "no count: no more");
        assert_eq!(odd.rest, None, "no words: no rest");
        assert_eq!(odd.credits, None);
    }

    /// Needs you stays reachable whatever the brief shows, matched by item: equal counts of
    /// different items still leave a link.
    #[test]
    fn needs_you_stays_reachable_matched_by_item_never_by_number() {
        let d = |kind: &str, id: &str| Decision {
            id: format!("{kind}:{id}"),
            r#ref: Ref {
                kind: kind.into(),
                id: id.into(),
                taf: String::new(),
            },
            ..Default::default()
        };
        let (a, b, c) = (d("hold", "ce_1"), d("step", "h1"), d("hold", "ce_2"));
        let shown = [&a, &b, &c];
        let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            needs_you_beyond_brief(&s(&["hold:ce_1", "step:h1", "hold:ce_9"]), 0, &shown),
            1,
            "three and three, but one of Needs you's isn't in the brief"
        );
        assert_eq!(
            needs_you_beyond_brief(&s(&["hold:ce_1", "step:h1"]), 0, &shown),
            0
        );
        assert_eq!(needs_you_beyond_brief(&s(&["hold:ce_1"]), 2, &shown), 2);
        assert_eq!(
            needs_you_beyond_brief(&s(&["step:ce_1"]), 0, &shown),
            1,
            "matched by kind too"
        );
        let w = en();
        assert_eq!(
            needs_you_line(&w, &s(&["hold:ce_9"]), 1, &shown).unwrap(),
            Some(w.count("needs_you.count", 2, &[]).unwrap())
        );
        assert_eq!(needs_you_line(&w, &[], 0, &shown).unwrap(), None);
    }

    /// An answer opens where its own answer is given, and only a held call's or a step's is
    /// signed; a check decision opens its taf ("Open the taf"), by tafs cloud's id.
    #[test]
    fn where_each_answer_opens() {
        let mut d = brief(LIVE).decisions[0].clone();
        assert_eq!(Destination::of(&d), Destination::Hold("ce_1".into()));
        assert!(Destination::of(&d).signed());
        d.r#ref = Ref {
            kind: "proposal".into(),
            id: "p1".into(),
            taf: String::new(),
        };
        assert_eq!(Destination::of(&d), Destination::NeedsYou);
        d.r#ref = Ref {
            kind: "hold".into(),
            id: String::new(),
            taf: String::new(),
        };
        assert_eq!(Destination::of(&d), Destination::NeedsYou);
        d.r#ref = Ref {
            kind: "check".into(),
            id: String::new(),
            taf: "depl_1".into(),
        };
        assert_eq!(Destination::of(&d), Destination::Taf("depl_1".into()));
        assert!(!Destination::of(&d).signed(), "opening a taf signs nothing");
        d.r#ref = Ref {
            kind: "check".into(),
            id: "c1".into(),
            taf: String::new(),
        };
        assert_eq!(Destination::of(&d), Destination::NeedsYou);
        d.r#ref.kind = "step".into();
        assert!(Destination::of(&d).signed());
    }

    /// Later's route keeps the decision's id as written (":" isn't escaped, "/" is); an urgent
    /// decision is never put off, even one sent with a Later.
    #[test]
    fn laters_route_and_an_urgent_decision() {
        let mut d = brief(LIVE).decisions[0].clone();
        d.id = "hold:ce_1".into();
        assert_eq!(
            later_path(&d).as_deref(),
            Some("/v1/brief/decisions/hold:ce_1/later")
        );
        d.id = "a/b c".into();
        assert_eq!(
            later_path(&d).as_deref(),
            Some("/v1/brief/decisions/a%2Fb%20c/later")
        );
        d.urgent = true;
        assert_eq!(later_path(&d), None);
        assert_eq!(d.later_shown(), None, "no Later on an urgent card");
        d.urgent = false;
        d.later = None;
        assert_eq!(later_path(&d), None);
    }

    #[test]
    fn the_briefs_path_by_label() {
        assert_eq!(Brief::path(None), "/v1/brief");
        assert_eq!(Brief::path(Some("money")), "/v1/brief?label=money");
        assert_eq!(Brief::path(Some("a b&c")), "/v1/brief?label=a%20b%26c");
    }

    #[test]
    fn set_aside_decisions_are_left_out() {
        let b = brief(LIVE);
        let mut aside = std::collections::BTreeSet::new();
        assert_eq!(b.open_decisions(&aside).count(), 1);
        aside.insert("d1".to_string());
        assert_eq!(b.open_decisions(&aside).count(), 0);
    }

    /// All, two labels, then "+N"; the chosen one inverted, tapped again back to All.
    #[test]
    fn the_chips_row() {
        let l = |id: &str, waiting: i64| Label {
            id: id.into(),
            name: id.to_uppercase(),
            count: 2,
            waiting,
            ..Default::default()
        };
        let labels = [l("a", 1), l("b", 0), l("c", 0)];
        let row = chips(&labels, None, "All", false);
        let words: Vec<_> = row.iter().map(|c| c.words.as_str()).collect();
        assert_eq!(words, ["All", "A", "B", "+1"]);
        assert!(row[0].selected && row[0].count.is_none());
        assert!(row[1].waiting && !row[2].waiting);
        assert_eq!(row[1].tap, ChipTap::Choose(Some("a".into())));
        assert_eq!(row[3].tap, ChipTap::Expand);
        let row = chips(&labels, Some("a"), "All", true);
        assert_eq!(row.len(), 4, "expanded: every label, no +N");
        assert!(!row[0].selected && row[1].selected);
        assert_eq!(row[1].tap, ChipTap::Choose(None), "tapped again: All");
        assert_eq!(chips(&labels[..2], None, "All", false).len(), 3);
        let names: Vec<String> = ["Money", "Ops", "Clients"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(taf_chips(&names), ["Money", "Ops", "+1"]);
        assert_eq!(taf_chips(&names[..2]), ["Money", "Ops"]);
        assert!(taf_chips(&[]).is_empty());
    }

    #[test]
    fn the_first_chips_word() {
        let w = en();
        assert_eq!(all_words(&w, Some("Alle"), None).unwrap(), "Alle");
        let mut b = brief(LIVE);
        assert_eq!(
            all_words(&w, None, Some(&b)).unwrap(),
            w.line("home.labels_all").unwrap()
        );
        b.words.all = "Everything".into();
        assert_eq!(all_words(&w, None, Some(&b)).unwrap(), "Everything");
    }
}
