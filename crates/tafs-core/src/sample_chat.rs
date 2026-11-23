//! The sample's chat with a taf (`AppModel.sampleReply`, `sampleHello`, `loadSample`'s Support
//! replies, `deliverChatOnce`'s sample branch): nothing is sent. A taf says hello; told to stop it
//! pauses, told to go it carries on, asked for more room it points at its Card, and anything else
//! is answered with what it did lately and what waits on the owner.

use chrono::{DateTime, Duration, Utc};

use crate::coach::{ChatMessage, From, MessageKind, Work, WorkStep};
use crate::stream::Event;
use crate::words::{WordBook, WordsError};

/// The session the sample's messages are in.
pub const SESSION: &str = "sample";

/// How long the iPad's sample takes to answer (`sampleReplyDelay`): half of it is a beat.
pub const REPLY_MS: u64 = 700;

/// What an answer does to the taf beside what it says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Effect {
    /// Stop, pause, hold off: paused, if it wasn't.
    Pause,
    /// Go, carry on, resume, continue: back to work, if it was paused.
    Resume,
    None,
}

/// Which of `sampleReply`'s answers a message gets, the first that matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Paused,
    BackToWork,
    MoreRoom,
    Lately,
}

/// `sampleReply`'s rules, on the message as it's sent (trimmed): "stop", "pause" or "hold off"
/// anywhere; "go", "carry on", "resume" or "continue" as the whole message or its first words;
/// "more", "raise", "limit", "bigger" or "increase" anywhere; else what it did lately.
pub fn kind(text: &str) -> Kind {
    let lower = text.trim().to_lowercase();
    if ["stop", "pause", "hold off"]
        .iter()
        .any(|w| lower.contains(w))
    {
        return Kind::Paused;
    }
    if ["go", "carry on", "resume", "continue"]
        .iter()
        .any(|w| lower == *w || lower.starts_with(&format!("{w} ")))
    {
        return Kind::BackToWork;
    }
    if ["more", "raise", "limit", "bigger", "increase"]
        .iter()
        .any(|w| lower.contains(w))
    {
        return Kind::MoreRoom;
    }
    Kind::Lately
}

/// The answer to `text` and what it does: `done`, the titles of the taf's Record lines, newest
/// first; `waiting`, how many of its calls wait on the owner. None for a taf that's gone.
pub fn reply(
    w: &WordBook,
    text: &str,
    done: &[String],
    waiting: usize,
) -> Result<(String, Effect), WordsError> {
    Ok(match kind(text) {
        Kind::Paused => (w.line("sample.reply.paused")?, Effect::Pause),
        Kind::BackToWork => (w.line("sample.reply.back_to_work")?, Effect::Resume),
        Kind::MoreRoom => (w.line("sample.reply.more_room")?, Effect::None),
        Kind::Lately => {
            let mut lines = Vec::new();
            let first: Vec<String> = done.iter().take(3).map(|t| format!("• {t}")).collect();
            lines.push(if first.is_empty() {
                w.line("sample.reply.nothing_done")?
            } else {
                w.get("sample.reply.lately", &[("done", &first.join("\n"))])?
            });
            if waiting > 0 {
                lines.push(w.count("sample.reply.waiting", waiting as i64, &[])?);
            }
            lines.push(w.line("sample.reply.note")?);
            (lines.join("\n\n"), Effect::None)
        }
    })
}

/// `sampleReply` for a taf the sample no longer has.
pub fn gone(w: &WordBook) -> Result<String, WordsError> {
    w.line("sample.reply.gone")
}

/// One of the sample's messages, at `at`.
pub fn message(from: From, text: &str, at: DateTime<Utc>) -> ChatMessage {
    ChatMessage {
        at,
        from,
        kind: MessageKind::Message,
        text: text.to_string(),
        session: SESSION.to_string(),
        door: None,
        work: None,
        attachments: vec![],
        server_id: None,
        answers: None,
        delivered: None,
        counted: None,
        refused: None,
    }
}

/// What a sample taf's chat starts with (`loadChat`'s hello, `loadSample`): its hello, with
/// what it's doing now; and for Support replies, the owner's question and its answer too.
pub fn opening(
    w: &WordBook,
    key: &str,
    name: &str,
    now_line: &str,
    done: &[String],
    waiting: usize,
    at: DateTime<Utc>,
) -> Result<Vec<ChatMessage>, WordsError> {
    let mut out = vec![message(
        From::Taf,
        &w.get("sample.hello", &[("name", name), ("now", now_line)])?,
        at,
    )];
    if key == "support" {
        let question = w.line("sample.chat.question")?;
        let (answer, _) = reply(w, &question, done, waiting)?;
        // A millisecond apart, in the order they're read: each its own row (a message is known by
        // its time, as tafs cloud's are).
        out.push(message(
            From::Owner,
            &question,
            at + Duration::milliseconds(1),
        ));
        out.push(message(From::Taf, &answer, at + Duration::milliseconds(2)));
    }
    Ok(out)
}

/// The sample's answer as it streams (`deliverChatOnce`, kiff/tafs#285), each event after the
/// milliseconds before it: what it's thinking, a beat each, its notes read (`memory`), then its
/// words a word at a time, a twelfth of a beat each. None when `beat_ms` is 0: it answers at once.
pub fn script(w: &WordBook, reply: &str, beat_ms: u64) -> Result<Vec<(u64, Event)>, WordsError> {
    if beat_ms == 0 {
        return Ok(vec![]);
    }
    let mut out = vec![
        (
            beat_ms,
            Event::Thinking(w.line("sample.reply.looking_lately")?),
        ),
        (
            beat_ms,
            Event::Thinking(w.line("needs_you.answer_names_each")?),
        ),
        (
            beat_ms,
            Event::Activity {
                tool: "memory".into(),
                step: None,
                call: Some("sample-1".into()),
            },
        ),
        (
            beat_ms,
            Event::Step {
                call: "sample-1".into(),
                status: "done".into(),
            },
        ),
    ];
    let word = (beat_ms / 12).max(1);
    for (i, piece) in reply.split(' ').enumerate() {
        let text = if i == 0 {
            piece.to_string()
        } else {
            format!(" {piece}")
        };
        out.push((word, Event::Text(text)));
    }
    Ok(out)
}

/// What the answer worked on, folded under it (`WorkSummary`): its notes read, for how long.
pub fn work(ms: i64) -> Work {
    Work {
        ms,
        steps: vec![WorkStep {
            tool: "memory".into(),
            error: false,
            not_made: false,
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    fn at() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-11-16T09:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn a_sample_taf_says_hello_and_answers() {
        let w = en();
        let o = opening(&w, "invoices", "Invoices", "Next run 9:00", &[], 0, at()).unwrap();
        assert_eq!(o.len(), 1);
        assert_eq!(o[0].from, From::Taf);
        assert!(o[0]
            .text
            .starts_with("Hi, I'm Invoices. I'm on it: Next run 9:00."));
        let (r, e) = reply(&w, "What did you do?", &[], 0).unwrap();
        assert_eq!(e, Effect::None);
        assert_eq!(
            r,
            "Nothing done yet today.\n\n(Sample reply. Signed in with KIFF, your taf answers for real.)"
        );
    }

    #[test]
    fn telling_a_sample_taf_to_stop_pauses_it_and_go_resumes_it() {
        let w = en();
        assert_eq!(
            reply(&w, "Please stop for today", &[], 0).unwrap(),
            (w.line("sample.reply.paused").unwrap(), Effect::Pause)
        );
        assert_eq!(
            reply(&w, "go", &[], 0).unwrap(),
            (w.line("sample.reply.back_to_work").unwrap(), Effect::Resume)
        );
        // As the iPad's rules read: anywhere for stop, whole or first words for go.
        assert_eq!(kind("That's unstoppable"), Kind::Paused);
        assert_eq!(kind("Carry on, don't stop"), Kind::Paused);
        assert_eq!(kind("Go ahead"), Kind::BackToWork);
        assert_eq!(kind("let's go"), Kind::Lately);
        assert_eq!(kind("  CONTINUE  "), Kind::BackToWork);
    }

    #[test]
    fn asking_for_more_room_is_answered_with_the_card() {
        let w = en();
        let (r, e) = reply(&w, "Raise your limit to €500", &[], 0).unwrap();
        assert!(r.contains("only you can"), "{r}");
        assert_eq!(e, Effect::None);
        assert_eq!(kind("anymore"), Kind::MoreRoom);
        assert_eq!(kind("How did this morning go?"), Kind::Lately);
    }

    #[test]
    fn support_replies_starts_with_its_question_answered() {
        let w = en();
        let done = vec![
            "Held: Promise a replacement on ticket #7802".to_string(),
            "Reply to ticket #7781".to_string(),
            "Support replies started".to_string(),
            "An older one".to_string(),
        ];
        let o = opening(&w, "support", "Support replies", "Working", &done, 1, at()).unwrap();
        assert_eq!(o.len(), 3);
        assert_eq!(
            (o[1].from, o[1].text.as_str()),
            (From::Owner, "How did this morning go?")
        );
        assert_eq!(
            o[2].text,
            "Lately:\n• Held: Promise a replacement on ticket #7802\n• Reply to ticket #7781\n• Support replies started\n\n1 thing waits on you in Needs you.\n\n(Sample reply. Signed in with KIFF, your taf answers for real.)"
        );
        assert!(o.iter().all(|m| m.session == SESSION));
        // Each its own row, in order.
        assert!(o[0].at < o[1].at && o[1].at < o[2].at);
        assert!(o[0].id() != o[2].id());
        let (r, _) = reply(&w, "?", &done[..1], 2).unwrap();
        assert!(r.contains("2 things wait on you in Needs you."), "{r}");
    }

    #[test]
    fn the_answer_streams_at_the_ipads_beats() {
        let w = en();
        let s = script(&w, "Back to  work.", 350).unwrap();
        assert_eq!(s.len(), 4 + 4);
        assert!(s[..4].iter().all(|(ms, _)| *ms == 350));
        assert!(matches!(&s[2].1, Event::Activity { tool, .. } if tool == "memory"));
        let words: String = s[4..]
            .iter()
            .map(|(ms, e)| {
                assert_eq!(*ms, 29);
                match e {
                    Event::Text(t) => t.clone(),
                    _ => panic!("{e:?}"),
                }
            })
            .collect();
        // Every word, the spaces as written.
        assert_eq!(words, "Back to  work.");
        assert!(script(&w, "x", 0).unwrap().is_empty());
        assert_eq!(work(1400).steps[0].tool, "memory");
    }
}
