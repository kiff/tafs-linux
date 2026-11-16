//! Reporting an answer Coach or a taf wrote (`ReportAnswerSheet`, `TafsCloud.ReportReason`,
//! `reportAnswer`, `AppModel.canReport` and `reportAnswer`): `POST
//! /v1/tafs/{id}/messages/{message}/report` at tafs cloud as the session, unsigned, with a reason,
//! an optional note and the answer's chat. tafs cloud keeps the answer as it was written, and a
//! person at KIFF reads it. Nothing here sends.

use serde_json::{json, Map, Value};

use crate::coach::{bool_if, field, object, text, ChatMessage, From, MessageKind};
use crate::words::{WordBook, WordsError};
use crate::work::segment;

/// The longest note tafs cloud takes (`noteLimit`), in characters.
pub const NOTE_LIMIT: usize = 1000;

/// Why a person reports an answer (`ReportReason`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reason {
    Offensive,
    Harmful,
    Wrong,
    Other,
}

impl Reason {
    /// In the sheet's order (`allCases`).
    pub const ALL: [Reason; 4] = [
        Reason::Offensive,
        Reason::Harmful,
        Reason::Wrong,
        Reason::Other,
    ];

    /// As tafs cloud takes it.
    pub fn raw(self) -> &'static str {
        match self {
            Reason::Offensive => "offensive",
            Reason::Harmful => "harmful",
            Reason::Wrong => "wrong",
            Reason::Other => "other",
        }
    }

    pub fn title(self, w: &WordBook) -> Result<String, WordsError> {
        w.line(&format!("report.reason.{}", self.raw()))
    }
}

/// An answer to report: tafs cloud's id for it, and its chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reported {
    pub message: String,
    pub session: String,
}

/// `canReport`: an answer of Coach's or a taf's, never the owner's own words or a run's kickoff,
/// and one tafs cloud has an id for (not one still on its way).
pub fn reportable(m: &ChatMessage) -> Option<Reported> {
    if m.from != From::Taf || m.kind != MessageKind::Message || m.text.is_empty() {
        return None;
    }
    Some(Reported {
        message: m.server_id.clone()?,
        session: m.session.clone(),
    })
}

/// `POST /v1/tafs/{id}/messages/{message}/report`: Coach's answers by Coach's taf.
pub fn path(taf: &str, message: &str) -> String {
    format!(
        "/v1/tafs/{}/messages/{}/report",
        segment(taf),
        segment(message)
    )
}

/// `reportAnswer`'s body as `JSONEncoder` writes it: the reason; the note trimmed, left out when
/// blank; the chat.
pub fn body(reason: Reason, note: &str, session: Option<&str>) -> Vec<u8> {
    let mut b = Map::new();
    b.insert("reason".into(), json!(reason.raw()));
    let note = note.trim();
    if !note.is_empty() {
        b.insert("note".into(), json!(note));
    }
    if let Some(s) = session {
        b.insert("session".into(), json!(s));
    }
    Value::Object(b).to_string().into_bytes()
}

/// tafs cloud's answer, as strictly as `AnswerReport`'s synthesized decoder: whether the report
/// is new (false: this person had reported the answer already).
pub fn answered(body: &[u8]) -> Option<bool> {
    let v: Value = serde_json::from_slice(body).ok()?;
    (|| -> crate::coach::Read<bool> {
        let r = object(field(object(&v)?, "report").ok_or(())?)?;
        text(r, "id")?;
        text(r, "at")?;
        bool_if(r, "new")?.ok_or(())
    })()
    .ok()
}

/// How a report went (`ReportOutcome`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Sent,
    Already,
    /// Not sent, with why, in tafs cloud's words when it gives them, else the app's.
    Failed(String),
}

/// `reportAnswer`'s outcome from tafs cloud's answer: kept or kept before; refused with tafs
/// cloud's words; an answer gone (404); anything else in the app's words, never tafs cloud's own
/// wording, which is for developers.
pub fn outcome(w: &WordBook, status: u16, body: &[u8]) -> Result<Outcome, WordsError> {
    if (200..300).contains(&status) {
        return Ok(match answered(body) {
            Some(true) => Outcome::Sent,
            Some(false) => Outcome::Already,
            None => Outcome::Failed(w.line("kiff.did_not_reach_kiff_try")?),
        });
    }
    let r = crate::coach_send::Refusal::read(status, body);
    Ok(Outcome::Failed(match r.words {
        Some(words) => words,
        None if status == 404 => w.line("needs_you.answer_gone")?,
        None => w.line("kiff.did_not_reach_kiff_try")?,
    }))
}

/// The report didn't reach tafs cloud.
pub fn unreached(w: &WordBook) -> Result<Outcome, WordsError> {
    Ok(Outcome::Failed(w.line("kiff.did_not_reach_kiff_try")?))
}

/// The sheet's words (`ReportAnswerSheet`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetLines {
    pub title: String,
    pub what_wrong: String,
    pub reasons: Vec<(Reason, String)>,
    pub note_hint: String,
    /// Under a note over the limit.
    pub note_too_long: String,
    pub footer: String,
    pub cancel: String,
    pub send: String,
    pub done: String,
}

pub fn sheet(w: &WordBook) -> Result<SheetLines, WordsError> {
    Ok(SheetLines {
        title: w.line("report.menu")?,
        what_wrong: w.line("report.what_wrong")?,
        reasons: Reason::ALL
            .iter()
            .map(|r| Ok((*r, r.title(w)?)))
            .collect::<Result<_, WordsError>>()?,
        note_hint: w.line("report.note_hint")?,
        note_too_long: w.line("report.note_most_characters")?,
        footer: w.line("report.person_kiff_reads_your_report")?,
        cancel: w.line("common.cancel")?,
        send: w.line("common.send")?,
        done: w.line("common.done")?,
    })
}

/// Send is there once a reason is chosen, nothing is on its way, and the note fits.
pub fn can_send(reason: Option<Reason>, sending: bool, note: &str) -> bool {
    reason.is_some() && !sending && note.chars().count() <= NOTE_LIMIT
}

/// What the sheet says once it's done, or under it when it wasn't sent.
pub fn said(w: &WordBook, o: &Outcome) -> Result<String, WordsError> {
    match o {
        Outcome::Sent => w.line("report.thanks"),
        Outcome::Already => w.line("report.already"),
        Outcome::Failed(why) => w.get("report.not_sent_why", &[("failed", why)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    fn v(b: &[u8]) -> Value {
        serde_json::from_slice(b).unwrap()
    }

    fn message(from: From, kind: MessageKind, text: &str, id: Option<&str>) -> ChatMessage {
        ChatMessage {
            at: chrono::Utc::now(),
            from,
            kind,
            text: text.into(),
            session: "sesn_9".into(),
            door: None,
            work: None,
            attachments: vec![],
            server_id: id.map(str::to_string),
            answers: None,
            delivered: None,
            counted: None,
            refused: None,
        }
    }

    #[test]
    fn an_answer_is_reported_by_its_id_with_its_chat_and_note() {
        assert_eq!(
            path("depl_1", "run_1.4"),
            "/v1/tafs/depl_1/messages/run_1.4/report"
        );
        assert_eq!(path("a/b", "c d"), "/v1/tafs/a%2Fb/messages/c%20d/report");
        assert_eq!(
            v(&body(
                Reason::Wrong,
                "  It refunded the wrong order. ",
                Some("sesn_9")
            )),
            json!({"reason": "wrong", "note": "It refunded the wrong order.", "session": "sesn_9"})
        );
        // A blank note is left out, and so is a chat it doesn't know.
        assert_eq!(
            v(&body(Reason::Offensive, "   ", Some("sesn_9"))),
            json!({"reason": "offensive", "session": "sesn_9"})
        );
        assert_eq!(
            v(&body(Reason::Other, "", None)),
            json!({"reason": "other"})
        );
        let raws: Vec<&str> = Reason::ALL.iter().map(|r| r.raw()).collect();
        assert_eq!(raws, ["offensive", "harmful", "wrong", "other"]);
    }

    #[test]
    fn only_an_answer_with_an_id_is_reportable() {
        let answer = message(
            From::Taf,
            MessageKind::Message,
            "Refunded 4410.",
            Some("run_1.4"),
        );
        assert_eq!(
            reportable(&answer),
            Some(Reported {
                message: "run_1.4".into(),
                session: "sesn_9".into()
            })
        );
        assert_eq!(
            reportable(&message(
                From::Owner,
                MessageKind::Message,
                "Skip 4410",
                Some("run_1.in")
            )),
            None
        );
        assert_eq!(
            reportable(&message(
                From::Taf,
                MessageKind::Kickoff,
                "Work the queue",
                Some("run_1.0")
            )),
            None
        );
        assert_eq!(
            reportable(&message(
                From::Taf,
                MessageKind::Message,
                "Refunding…",
                None
            )),
            None
        );
        assert_eq!(
            reportable(&message(From::Taf, MessageKind::Message, "", Some("x"))),
            None
        );
    }

    #[test]
    fn the_answer_is_read_as_strictly_as_the_ipad() {
        assert_eq!(
            answered(br#"{"report":{"id":"r1","at":"2026-11-01T15:00:00Z","new":true}}"#),
            Some(true)
        );
        assert_eq!(
            answered(br#"{"report":{"id":"r1","at":"2026-11-01T15:00:00Z","new":false}}"#),
            Some(false)
        );
        assert_eq!(answered(br#"{"report":{"id":"r1","new":true}}"#), None);
        assert_eq!(
            answered(br#"{"report":{"id":"r1","at":"x","new":"yes"}}"#),
            None
        );
        assert_eq!(answered(br#"{"id":"r1","at":"x","new":true}"#), None);
    }

    #[test]
    fn how_it_went_in_the_app_s_words_or_tafs_cloud_s() {
        let w = en();
        let ok = br#"{"report":{"id":"r1","at":"2026-11-01T15:00:00Z","new":true}}"#;
        assert_eq!(outcome(&w, 200, ok).unwrap(), Outcome::Sent);
        let again = br#"{"report":{"id":"r1","at":"2026-11-01T15:00:00Z","new":false}}"#;
        assert_eq!(outcome(&w, 200, again).unwrap(), Outcome::Already);
        assert_eq!(
            outcome(&w, 404, br#"{"error":"no answer of the taf's by that id"}"#).unwrap(),
            Outcome::Failed("This answer isn't on tafs cloud any more.".into())
        );
        assert_eq!(
            outcome(&w, 429, br#"{"words":"Too many reports today."}"#).unwrap(),
            Outcome::Failed("Too many reports today.".into())
        );
        assert_eq!(
            outcome(&w, 500, b"{}").unwrap(),
            Outcome::Failed("It didn't reach KIFF. Try again.".into())
        );
        assert_eq!(
            outcome(&w, 200, b"{}").unwrap(),
            Outcome::Failed("It didn't reach KIFF. Try again.".into())
        );
        assert_eq!(
            said(&w, &Outcome::Sent).unwrap(),
            "Thanks. Someone at KIFF will read your report."
        );
        assert_eq!(
            said(&w, &Outcome::Already).unwrap(),
            "You've already reported this answer."
        );
        assert_eq!(
            said(&w, &Outcome::Failed("x".into())).unwrap(),
            "Not sent: x"
        );
    }

    #[test]
    fn send_waits_for_a_reason_and_a_note_that_fits() {
        let w = en();
        let s = sheet(&w).unwrap();
        assert_eq!(s.title, "Report this answer");
        assert_eq!(s.reasons[2], (Reason::Wrong, "Wrong".into()));
        assert!(!can_send(None, false, ""));
        assert!(can_send(Some(Reason::Wrong), false, ""));
        assert!(!can_send(Some(Reason::Wrong), true, ""));
        assert!(can_send(
            Some(Reason::Wrong),
            false,
            &"é".repeat(NOTE_LIMIT)
        ));
        assert!(!can_send(
            Some(Reason::Wrong),
            false,
            &"é".repeat(NOTE_LIMIT + 1)
        ));
    }
}
