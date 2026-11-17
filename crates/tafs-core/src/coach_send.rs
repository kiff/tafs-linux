//! What the owner sends Coach, unsigned, at tafs cloud as the session (`TafsCloud.sendMessage`,
//! `steer`, `stop`, `answerCard`) and what the conversation does with it while it goes
//! (`AppModel`'s Coach: `deliverSetup`, `submitSetup`, `withSent`, `answer`, `stopSetup`,
//! `followSetup`, `SentMessages`, `QuestionAnswers`, `QuestionSelection`). Each request's path and
//! body is built here, and each answer decoded as strictly as the iPad's `Decodable`. The app
//! makes the calls; nothing here sends.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde_json::{json, Map, Value};

use crate::coach::{
    array_if, bool_if, field, object, text, text_if, Answer, Attachment, Chat, ChatMessage, From,
    Kind, MessageKind, Question, Read, SetupThread, Steering, SteeringMessage, PENDING,
};
use crate::words::{WordBook, WordsError};
use crate::work::segment;

// MARK: Routes and bodies.

/// The longest message a chat takes (`AppModel.chatLimit`), in characters.
pub const CHAT_LIMIT: usize = 2000;

/// `POST /v1/tafs/{id}/messages` (`sendMessage`).
pub fn messages_path(taf: &str) -> String {
    format!("/v1/tafs/{}/messages", segment(taf))
}

/// `POST /v1/tafs/{id}/improve` (`improve`): an improvement's first message, on the taf it
/// improves; tafs cloud adds the taf's context to it.
pub fn improve_path(taf: &str) -> String {
    format!("/v1/tafs/{}/improve", segment(taf))
}

/// An improvement's first message: its words alone.
pub fn improve_body(text: &str) -> Vec<u8> {
    json!({ "text": text }).to_string().into_bytes()
}

/// `POST /v1/tafs/{id}/steer` (`steer`).
pub fn steer_path(taf: &str) -> String {
    format!("/v1/tafs/{}/steer", segment(taf))
}

/// `POST /v1/tafs/{id}/stop` (`stop`).
pub fn stop_path(taf: &str) -> String {
    format!("/v1/tafs/{}/stop", segment(taf))
}

/// `POST /v1/tafs/{id}/components/{component}/answer` (`answerCard`).
pub fn answer_path(taf: &str, component: &str) -> String {
    format!(
        "/v1/tafs/{}/components/{}/answer",
        segment(taf),
        segment(component)
    )
}

/// `POST /v1/tafs/{id}/chats/{session}/archive` (`archiveChat`): out of the list on every device.
pub fn archive_path(taf: &str, session: &str) -> String {
    format!(
        "/v1/tafs/{}/chats/{}/archive",
        segment(taf),
        segment(session)
    )
}

/// `POST /v1/tafs/{id}/chats/{session}/unarchive` (`unarchiveChat`): back in the list.
pub fn unarchive_path(taf: &str, session: &str) -> String {
    format!(
        "/v1/tafs/{}/chats/{}/unarchive",
        segment(taf),
        segment(session)
    )
}

/// `GET /v1/tafs/{id}/chats/archived` (`archivedChats`), read as `chats`.
pub fn archived_path(taf: &str) -> String {
    format!("/v1/tafs/{}/chats/archived", segment(taf))
}

/// Archive's and Put back's answer: `{"archived": Bool}`, or it can't be read.
pub fn archived(body: &[u8]) -> Option<bool> {
    serde_json::from_slice::<Value>(body)
        .ok()?
        .get("archived")?
        .as_bool()
}

/// Which chat a message goes into (`ChatTarget`): the live one, one the owner reopened, or a new
/// one; and whether the app follows a new chat tafs cloud starts after a hand-over.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ChatTarget {
    pub session: Option<String>,
    pub new_chat: bool,
    pub new_chat_after_handover: bool,
}

impl ChatTarget {
    /// `setupTarget`: a new chat, the one shown, or the live one; every Coach send says the app
    /// follows a new chat.
    pub fn coach(fresh: bool, chat: Option<&str>) -> ChatTarget {
        ChatTarget {
            session: if fresh {
                None
            } else {
                chat.map(str::to_string)
            },
            new_chat: fresh,
            new_chat_after_handover: true,
        }
    }
}

/// `MessageBody` as `JSONEncoder` writes it: an absent option is left out, `new_chat` and
/// `new_chat_after_handover` only when true, and the files' ids only when there are some.
pub fn message_body(text: &str, target: &ChatTarget, files: &[Attachment]) -> Vec<u8> {
    let mut body = Map::new();
    body.insert("text".into(), json!(text));
    if let Some(s) = &target.session {
        body.insert("session".into(), json!(s));
    }
    if !files.is_empty() {
        let ids: Vec<&str> = files.iter().map(|f| f.id.as_str()).collect();
        body.insert("attachments".into(), json!(ids));
    }
    if target.new_chat {
        body.insert("new_chat".into(), json!(true));
    }
    if target.new_chat_after_handover {
        body.insert("new_chat_after_handover".into(), json!(true));
    }
    Value::Object(body).to_string().into_bytes()
}

/// `steer`'s body: the run, the words, and `hold`, so what the run doesn't take tafs cloud keeps.
pub fn steer_body(run: &str, text: &str) -> Vec<u8> {
    json!({"run": run, "text": text, "hold": true})
        .to_string()
        .into_bytes()
}

/// `stop`'s body: the chat whose latest run is cancelled.
pub fn stop_body(session: &str) -> Vec<u8> {
    json!({ "session": session }).to_string().into_bytes()
}

/// tafs cloud's answer to a message (`Sent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sent {
    pub session: String,
    pub new_session: bool,
    /// tafs cloud's id for the message, as the thread will name it; none from an older one.
    pub message: Option<String>,
}

/// `Sent`, as strictly as its synthesized decoder.
pub fn sent(body: &[u8]) -> Option<Sent> {
    let v: Value = serde_json::from_slice(body).ok()?;
    (|| -> Read<Sent> {
        let o = object(&v)?;
        Ok(Sent {
            session: text(o, "session")?,
            new_session: bool_if(o, "new_session")?.ok_or(())?,
            message: match field(o, "message") {
                None => None,
                Some(m) => Some(text(object(m)?, "id")?),
            },
        })
    })()
    .ok()
}

/// What tafs cloud did with words written to the run at work (`Steered`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Steered {
    /// The run took it, and reads it at its next step.
    Taken { status: String },
    /// The run didn't: `sent`, it went at once as the next message; `held`, what waits now.
    Kept {
        sent: Option<(String, String)>,
        held: Option<SteeringMessage>,
    },
}

/// `steer`'s answer, as strictly as its decoder.
pub fn steered(body: &[u8]) -> Option<Steered> {
    let v: Value = serde_json::from_slice(body).ok()?;
    (|| -> Read<Steered> {
        let o = object(&v)?;
        let status = text_if(o, "status")?;
        let sent = match field(o, "sent") {
            None => None,
            Some(s) => {
                let s = object(s)?;
                Some((text(s, "run")?, text(s, "session")?))
            }
        };
        let held = match field(o, "held") {
            None => None,
            Some(m) => {
                let m = object(m)?;
                Some(SteeringMessage {
                    text: text(m, "text")?,
                    at: crate::coach::date(m, "at")?,
                })
            }
        };
        Ok(match status {
            Some(status) => Steered::Taken { status },
            None => Steered::Kept { sent, held },
        })
    })()
    .ok()
}

/// What the run at work holds once words were written to it (`submitChat`'s and `submitSetup`'s
/// steer): taken, they join what it hasn't read; kept, tafs cloud's `held` is what waits now. Shown
/// at once, as the next read of the thread says the same.
pub fn steered_into(
    current: Option<Steering>,
    run: &str,
    text: &str,
    steered: &Steered,
    now: DateTime<Utc>,
) -> Steering {
    match steered {
        Steered::Taken { status } => {
            let current = current.filter(|s| s.run == run);
            let mut untaken = current
                .as_ref()
                .map(|c| c.untaken.clone())
                .unwrap_or_default();
            untaken.push(SteeringMessage {
                text: text.into(),
                at: now,
            });
            Steering {
                run: run.into(),
                status: status.clone(),
                untaken,
                held: current.as_ref().and_then(|c| c.held.clone()),
                not_sent: current.map(|c| c.not_sent).unwrap_or_default(),
            }
        }
        Steered::Kept { held, .. } => Steering {
            run: current
                .as_ref()
                .map_or_else(|| run.to_string(), |c| c.run.clone()),
            status: current
                .as_ref()
                .map_or_else(|| "RUNNING".to_string(), |c| c.status.clone()),
            untaken: current
                .as_ref()
                .map(|c| c.untaken.clone())
                .unwrap_or_default(),
            held: held.clone(),
            not_sent: current.map(|c| c.not_sent).unwrap_or_default(),
        },
    }
}

/// `stop`'s answer: `{"stopped": …}`, a Bool, or it can't be read.
pub fn stopped(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("stopped").map(Value::is_boolean))
        == Some(true)
}

/// One question's answer as tafs cloud takes it (`CardAnswerValue.QuestionAnswer`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuestionAnswer {
    pub value: Option<String>,
    pub values: Option<Vec<String>>,
    pub own: Option<String>,
}

/// A card's answer as data (`CardAnswerValue`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerValue {
    Text(String),
    Questions(BTreeMap<String, QuestionAnswer>),
}

impl AnswerValue {
    fn json(&self) -> Value {
        match self {
            AnswerValue::Text(s) => json!(s),
            AnswerValue::Questions(q) => Value::Object(
                q.iter()
                    .map(|(id, a)| {
                        let mut o = Map::new();
                        if let Some(v) = &a.value {
                            o.insert("value".into(), json!(v));
                        }
                        if let Some(v) = &a.values {
                            o.insert("values".into(), json!(v));
                        }
                        if let Some(v) = &a.own {
                            o.insert("own".into(), json!(v));
                        }
                        (id.clone(), Value::Object(o))
                    })
                    .collect(),
            ),
        }
    }
}

/// `answerValue`: a card's answer as tafs cloud takes it, from the card's own form of it; none
/// for one the card doesn't take. Only a confirm and a questions card are answered here: this
/// computer is view-only, so an approval, a sign-in or a drive never gets an answer from it, not
/// even a decline. They're answered on the phone, and nothing here can build "approved" or
/// "signed in" by mistake.
pub fn answer_value(kind: Kind, text: &str) -> Option<AnswerValue> {
    match kind {
        Kind::Confirm => (!text.is_empty()).then(|| AnswerValue::Text(text.into())),
        Kind::Questions => {
            // `[String: QuestionAnswer]`, each field as its synthesized decoder reads it.
            let v: Value = serde_json::from_str(text).ok()?;
            let o = v.as_object()?;
            let mut out = BTreeMap::new();
            for (id, a) in o {
                let a = a.as_object()?;
                out.insert(
                    id.clone(),
                    QuestionAnswer {
                        value: text_if(a, "value").ok()?,
                        values: array_if(a, "values", crate::coach::strings).ok()?,
                        own: text_if(a, "own").ok()?,
                    },
                );
            }
            Some(AnswerValue::Questions(out))
        }
        // The phone's: approving, declining, signing in, Not now, a drive.
        Kind::Approval | Kind::SignIn | Kind::Drive => None,
        _ => None,
    }
}

/// `answerCard`'s body: the answer, and the chat the card is in when known.
pub fn answer_body(value: &AnswerValue, session: Option<&str>) -> Vec<u8> {
    let mut body = Map::new();
    body.insert("answer".into(), value.json());
    if let Some(s) = session {
        body.insert("session".into(), json!(s));
    }
    Value::Object(body).to_string().into_bytes()
}

/// tafs cloud took a card's answer (`CardAnswered`): the run that waited goes on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardAnswered {
    pub session: String,
    pub run: String,
    pub status: String,
}

pub fn card_answered(body: &[u8]) -> Option<CardAnswered> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let o = object(&v).ok()?;
    Some(CardAnswered {
        session: text(o, "session").ok()?,
        run: text(o, "run").ok()?,
        status: text(o, "status").ok()?,
    })
}

// MARK: A refusal.

/// tafs cloud's code for a turn refused for want of the AI-data consent.
pub const AI_CONSENT_REQUIRED: &str = "ai_consent_required";
/// The codes of a refusal because the tafs wait for credits (`creditsCodes`).
pub const CREDITS_CODES: [&str; 3] = ["credits_used_up", "limit_reached", "no_plan"];

/// tafs cloud's refusal (`CloudError.refusal`): its body read as one `Decodable`, so a field of
/// another shape leaves none of it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Refusal {
    pub status: u16,
    pub error: Option<String>,
    pub words: Option<String>,
    pub code: Option<String>,
    /// `account` or `taf`, else none.
    pub halted: Option<String>,
}

impl Refusal {
    pub fn read(status: u16, body: &[u8]) -> Refusal {
        let read = || -> Read<Refusal> {
            let v: Value = serde_json::from_slice(body).map_err(|_| ())?;
            let o = object(&v)?;
            array_if(o, "blocking", |b| {
                let b = object(b)?;
                text(b, "run")?;
                text(b, "session")?;
                text(b, "status")?;
                text_if(b, "trigger")?;
                Ok(())
            })?;
            array_if(o, "cards", crate::coach::strings)?;
            Ok(Refusal {
                status,
                error: text_if(o, "error")?,
                words: text_if(o, "words")?,
                code: text_if(o, "code")?,
                halted: text_if(o, "halted")?.filter(|h| h == "account" || h == "taf"),
            })
        };
        read().unwrap_or(Refusal {
            status,
            ..Refusal::default()
        })
    }

    /// `errorDescription`: tafs cloud's words, else its error, else "tafs cloud answered 500".
    pub fn words(&self, w: &WordBook) -> Result<String, WordsError> {
        match self.words.as_ref().or(self.error.as_ref()) {
            Some(s) => Ok(s.clone()),
            None => w.get(
                "error.cloud_answered",
                &[("status", &self.status.to_string())],
            ),
        }
    }

    /// Refused for want of the AI-data consent.
    pub fn wants_consent(&self) -> bool {
        self.code.as_deref() == Some(AI_CONSENT_REQUIRED)
    }

    /// `waitsForCredits`: tafs cloud's words when the tafs wait for credits; none otherwise.
    pub fn waits_for_credits(&self, w: &WordBook) -> Result<Option<String>, WordsError> {
        if self.status != 409
            || !self
                .code
                .as_deref()
                .is_some_and(|c| CREDITS_CODES.contains(&c))
        {
            return Ok(None);
        }
        self.words(w).map(Some)
    }
}

// MARK: Sent messages.

/// How long a sent message waits for the thread's copy (`SentMessages.kept`).
pub fn kept() -> Duration {
    Duration::seconds(120)
}

/// `SentMessages.merge`: the thread's messages with the sent ones it doesn't have yet, and those
/// still waiting. The copy is found by tafs cloud's id; only a send with none, by its words.
pub fn merge(
    messages: Vec<ChatMessage>,
    sent: &[ChatMessage],
    now: DateTime<Utc>,
) -> (Vec<ChatMessage>, Vec<ChatMessage>) {
    let mut taken = HashSet::new();
    let waiting: Vec<ChatMessage> = sent
        .iter()
        .filter(|s| {
            if now - s.at > kept() {
                return false;
            }
            let copy = messages.iter().enumerate().position(|(i, m)| {
                if taken.contains(&i) || m.from != From::Owner {
                    return false;
                }
                match &s.server_id {
                    Some(id) => m.server_id.as_ref() == Some(id),
                    None => m.text == s.text && m.at >= s.at - kept(),
                }
            });
            match copy {
                Some(i) => {
                    taken.insert(i);
                    false
                }
                None => true,
            }
        })
        .cloned()
        .collect();
    let mut out = messages;
    out.extend(waiting.iter().cloned());
    (out, waiting)
}

/// `SentMessages.stamp`: now, never before the thread's latest.
pub fn stamp(latest: Option<DateTime<Utc>>, now: DateTime<Utc>) -> DateTime<Utc> {
    match latest {
        Some(l) => now.max(l + Duration::milliseconds(1)),
        None => now,
    }
}

// MARK: A questions card's answers.

/// One question's answer as the owner gave it (`QuestionAnswers.Answer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuestionPick {
    Choice(String),
    Choices(Vec<String>),
    Own(String),
}

/// `QuestionAnswers.encode`: the message's answer text, JSON with its keys sorted.
pub fn encode_answers(answers: &BTreeMap<String, QuestionPick>) -> String {
    let o: Map<String, Value> = answers
        .iter()
        .map(|(id, a)| {
            (
                id.clone(),
                match a {
                    QuestionPick::Choice(v) => json!({ "value": v }),
                    QuestionPick::Choices(vs) => json!({ "values": vs }),
                    QuestionPick::Own(t) => json!({ "own": t }),
                },
            )
        })
        .collect();
    // serde_json's map is sorted by key, as `.sortedKeys`.
    Value::Object(o).to_string()
}

/// What the owner has picked on a questions card so far (`QuestionSelection`): a choice (several
/// on a multi question) or their own words, never both.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct QuestionSelection {
    pub picked: HashMap<String, HashSet<String>>,
    pub own: HashMap<String, String>,
    pub own_open: HashSet<String>,
}

impl QuestionSelection {
    /// Something else… opens the own field and clears the picks, or closes it.
    pub fn toggle_own(&mut self, q: &Question) {
        if !self.own_open.remove(&q.id) {
            self.own_open.insert(q.id.clone());
            self.picked.insert(q.id.clone(), HashSet::new());
        }
    }

    /// A choice: toggled on a multi question, the one choice otherwise; the own field closes.
    pub fn pick(&mut self, q: &Question, value: &str) {
        if q.multi == Some(true) {
            let set = self.picked.entry(q.id.clone()).or_default();
            if !set.remove(value) {
                set.insert(value.to_string());
            }
        } else {
            self.picked
                .insert(q.id.clone(), HashSet::from([value.to_string()]));
        }
        self.own_open.remove(&q.id);
    }

    pub fn is_picked(&self, q: &Question, value: &str) -> bool {
        self.picked.get(&q.id).is_some_and(|s| s.contains(value))
    }

    /// The question's answer, if it has one.
    pub fn answer(&self, q: &Question) -> Option<QuestionPick> {
        if self.own_open.contains(&q.id) {
            let words = self.own.get(&q.id).map(|s| s.trim()).unwrap_or_default();
            return (!words.is_empty()).then(|| QuestionPick::Own(words.to_string()));
        }
        let chosen: Vec<String> = q
            .options
            .iter()
            .map(|o| o.value.clone())
            .filter(|v| self.is_picked(q, v))
            .collect();
        if chosen.is_empty() {
            return None;
        }
        Some(if q.multi == Some(true) {
            QuestionPick::Choices(chosen)
        } else {
            QuestionPick::Choice(chosen[0].clone())
        })
    }

    /// How many questions have an answer (`answeredCount`).
    pub fn answered(&self, questions: &[Question]) -> usize {
        questions
            .iter()
            .filter(|q| self.answer(q).is_some())
            .count()
    }

    /// Send answers: every question answered, as the message's text; none before.
    pub fn send(&self, questions: &[Question]) -> Option<String> {
        let answers: BTreeMap<String, QuestionPick> = questions
            .iter()
            .filter_map(|q| Some((q.id.clone(), self.answer(q)?)))
            .collect();
        (answers.len() == questions.len()).then(|| encode_answers(&answers))
    }
}

// MARK: The conversation, as the app holds it.

/// How often the thread is read again while Coach is open (`setupPollInterval`), every second
/// while it works (`setupWorkingPollInterval`), and how many reads after the owner last wrote
/// (`setupPollBudget`: five minutes).
pub const POLL: std::time::Duration = std::time::Duration::from_secs(3);
pub const WORKING_POLL: std::time::Duration = std::time::Duration::from_secs(1);
pub const POLL_BUDGET: u32 = 100;

/// How the last run ended, when the owner should know (`RunEnd`): they stopped it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ended {
    Stopped,
}

/// A message that didn't go through (`UnsentMessage`), kept where it was written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unsent {
    pub id: u64,
    pub text: String,
    pub why: String,
    pub at: DateTime<Utc>,
    /// Refused for a halt: `account` or `taf`.
    pub halted: Option<String>,
    /// From the thread's steering, by its hash: shown once.
    pub settle: Option<String>,
    /// Not sent only because the owner wrote too many too fast (`busy`): a message that goes
    /// through takes it away.
    pub busy: bool,
}

/// A message on its way: the local copy's id, its words and where it goes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    pub local: String,
    pub text: String,
    pub target: ChatTarget,
    pub generation: u64,
    /// The files it carries.
    pub files: Vec<Attachment>,
}

/// What Submit does with the composer's words (`submitSetup`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Submit {
    /// Nothing to send, or not now: the words stay in the box.
    Stay,
    /// A message: send it.
    Send(Delivery),
    /// Coach is at work: write it to its run.
    Steer { run: String, text: String },
}

/// Coach's conversation as the app holds it (`AppModel`'s `setup*`): what tafs cloud read, what
/// was sent from here that it doesn't have yet, and what this computer did with each card.
#[derive(Debug, Clone, Default)]
pub struct Conversation {
    /// Coach's taf at tafs cloud (`setupTaf`).
    pub setup: Option<String>,
    /// The chat shown (`setupChat`); none until chosen.
    pub chat: Option<String>,
    /// A new conversation, until its first message is sent (`setupFresh`).
    pub fresh: bool,
    pub thread: SetupThread,
    sent: Vec<ChatMessage>,
    pub awaiting: Option<DateTime<Utc>>,
    pub sending: bool,
    pub ended: Option<Ended>,
    pub unsent: Vec<Unsent>,
    next_unsent: u64,
    settled: HashSet<String>,
    /// The cards answered here, by ref (`answered`).
    pub answered: HashSet<String>,
    /// Why an answer didn't go, by card id (`cardNotes`).
    pub notes: HashMap<String, String>,
    pub error: Option<String>,
    /// Bumped by each new or reopened conversation (`setupConversationGeneration`).
    pub generation: u64,
    /// Coach's chats, newest first, for Previous conversations.
    pub chats: Vec<Chat>,
    /// What the thread has drawn, in its order (`SteadyOrder`); another conversation starts from
    /// nothing.
    pub drawn: Vec<crate::coach::Entry>,
    /// KIFF's drafts the approval cards name, by id, as last read (`setupDrafts`).
    pub drafts: HashMap<String, crate::drafts::AgentDraft>,
    /// Why KIFF's draft wasn't read, by id, until a read gives it (`setupDraftErrors`).
    pub draft_errors: HashMap<String, String>,
    /// The taf this new conversation improves, until its first message has gone
    /// (`setupImproving`).
    pub improving: Option<Improving>,
}

/// A taf Coach is on (`setupImproving`): its id at tafs cloud, and its name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Improving {
    pub id: String,
    pub name: String,
}

impl Conversation {
    /// Where the next message goes (`setupTarget`).
    pub fn target(&self) -> ChatTarget {
        ChatTarget::coach(self.fresh, self.chat.as_deref())
    }

    /// Coach is at work (`setupThinking`): tafs cloud says so, or the owner just wrote.
    pub fn thinking(&self) -> bool {
        self.thread.working.is_some() || self.awaiting.is_some()
    }

    /// `cardRef`: the card by its ref, the unanswered one first.
    pub fn card_ref(&self, id: &str) -> String {
        let c = &self.thread.components;
        c.iter()
            .rev()
            .find(|s| s.component.id == id && s.answer.is_none())
            .or_else(|| c.iter().rev().find(|s| s.component.id == id))
            .map(|s| s.card_ref())
            .unwrap_or_else(|| format!("{}#{id}", self.chat.as_deref().unwrap_or_default()))
    }

    /// Answered here (`isAnswered`).
    pub fn is_answered(&self, id: &str) -> bool {
        self.answered.contains(&self.card_ref(id))
    }

    /// The drafts to read after a thread read (`loadSetupThread`'s last step): each one an
    /// approval or a sign-in names that isn't read as confirmed, once each, in the thread's order.
    pub fn drafts_to_read(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for c in &self.thread.components {
            let c = &c.component;
            if !matches!(c.kind, Kind::Approval | Kind::SignIn) {
                continue;
            }
            let Some(d) = &c.draft else { continue };
            if self.drafts.get(d).is_some_and(|x| x.confirmed()) || out.contains(d) {
                continue;
            }
            out.push(d.clone());
        }
        out
    }

    /// `readCardDraft`'s answer: the draft, kept and its error gone; or KIFF's reason, kept for
    /// the card while a draft read before stays drawn.
    pub fn took_draft(&mut self, id: &str, read: Result<crate::drafts::AgentDraft, String>) {
        match read {
            Ok(d) => {
                self.drafts.insert(id.to_string(), d);
                self.draft_errors.remove(id);
            }
            Err(why) => {
                self.draft_errors.insert(id.to_string(), why);
            }
        }
    }

    /// tafs cloud's thread read (`loadSetupThread`): with what was sent from here that it doesn't
    /// have yet, the wait settled, and what Coach's run never read shown as not sent.
    pub fn took(&mut self, thread: SetupThread, now: DateTime<Utc>) {
        let (messages, waiting) = merge(thread.messages.clone(), &self.sent, now);
        self.sent = waiting;
        self.thread = SetupThread { messages, ..thread };
        self.error = None;
        self.settle_awaiting(now);
        self.settle_steering(now);
    }

    /// `settleSetupAwaiting`: the wait ends once tafs cloud says Coach is at work, once it
    /// answered after the owner wrote, or after two minutes.
    pub fn settle_awaiting(&mut self, now: DateTime<Utc>) {
        let Some(since) = self.awaiting else { return };
        let answered = self
            .thread
            .messages
            .iter()
            .any(|m| m.from == From::Taf && m.at >= since)
            || self.thread.components.iter().any(|c| c.first_at >= since);
        if self.thread.working.is_some() || answered || now - since > Duration::seconds(120) {
            self.awaiting = None;
        }
    }

    /// `settleSetupSteering`: tafs cloud's `not_sent`, each shown once as not sent.
    fn settle_steering(&mut self, now: DateTime<Utc>) {
        let Some(s) = self.thread.steering.clone() else {
            return;
        };
        for (m, why) in &s.not_sent {
            let key = steering_key(&s, m);
            if self.settled.insert(key.clone()) {
                self.keep_unsent(&m.text, why, None, Some(key), now);
            }
        }
    }

    fn keep_unsent(
        &mut self,
        text: &str,
        why: &str,
        halted: Option<String>,
        settle: Option<String>,
        now: DateTime<Utc>,
    ) {
        self.next_unsent += 1;
        self.unsent.push(Unsent {
            id: self.next_unsent,
            text: text.into(),
            why: why.into(),
            at: now,
            halted,
            settle,
            busy: false,
        });
    }

    /// The composer's Send (`submitSetup`): a message now, or, while Coach works, to its run.
    pub fn submit(&mut self, text: &str, now: DateTime<Utc>) -> Submit {
        let text = text.trim();
        if text.is_empty() || self.setup.is_none() {
            return Submit::Stay;
        }
        if self.thinking() || self.sending {
            // No run yet: the message that starts it is still on its way.
            return match self.thread.working_run.clone() {
                Some(run) => Submit::Steer {
                    run,
                    text: text.into(),
                },
                None => Submit::Stay,
            };
        }
        match self.deliver(text, now) {
            Some(d) => Submit::Send(d),
            None => Submit::Stay,
        }
    }

    /// `deliverSetupOnce`'s start: the message shown at once, the way it was sent, until tafs
    /// cloud's copy takes its place. None when it isn't tried.
    pub fn deliver(&mut self, text: &str, now: DateTime<Utc>) -> Option<Delivery> {
        self.deliver_with(text, vec![], now)
    }

    /// `submitSetup` with files: a message of their own, the words with them or none
    /// (`deliverSetup(text, files:)`). None when it isn't tried.
    pub fn deliver_with(
        &mut self,
        text: &str,
        files: Vec<Attachment>,
        now: DateTime<Utc>,
    ) -> Option<Delivery> {
        let text = text.trim();
        if (text.is_empty() && files.is_empty())
            || text.chars().count() > CHAT_LIMIT
            || self.setup.is_none()
        {
            return None;
        }
        if self.sending {
            return None;
        }
        self.sending = true;
        self.ended = None;
        let latest = self.thread.items().iter().map(|i| i.at()).max();
        let mine = ChatMessage {
            at: stamp(latest, now),
            from: From::Owner,
            kind: MessageKind::Message,
            text: text.into(),
            session: PENDING.into(),
            door: None,
            work: None,
            attachments: files.clone(),
            server_id: None,
            answers: None,
            delivered: None,
            counted: None,
            refused: None,
        };
        let local = mine.id();
        self.sent.push(mine.clone());
        self.thread.messages.push(mine);
        Some(Delivery {
            local,
            text: text.into(),
            target: self.target(),
            generation: self.generation,
            files,
        })
    }

    fn drop_local(&mut self, local: &str) {
        self.sent.retain(|m| m.id() != local);
        self.thread.messages.retain(|m| m.id() != local);
    }

    /// An improvement's first message taken (`improve`): the conversation goes on in the chat
    /// tafs cloud answered with, as any other, and Coach is at work.
    pub fn improved(&mut self, d: &Delivery, sent: &Sent, now: DateTime<Utc>) {
        self.sending = false;
        self.improving = None;
        if let Some(id) = &sent.message {
            self.mark_sent(&d.local, id);
        }
        self.settle_chat(&sent.session);
        self.awaiting = Some(now);
    }

    /// Sent: the chat it went into is the one shown, a new chat after a hand-over is followed,
    /// and the copy takes tafs cloud's id. Coach is awaited until it answers.
    pub fn delivered(&mut self, d: &Delivery, sent: &Sent, now: DateTime<Utc>) {
        self.sending = false;
        self.settle_chat(&sent.session);
        self.follow_new_chat(&sent.session, sent.new_session, &d.local);
        if let Some(id) = &sent.message {
            self.mark_sent(&d.local, id);
        }
        self.awaiting = Some(now);
    }

    /// Refused for want of the consent, or because the tafs wait for credits (`why`): the
    /// message leaves the thread, with no Not sent row, and goes back in the box.
    pub fn not_tried(&mut self, d: &Delivery, why: Option<String>) {
        self.sending = false;
        self.drop_local(&d.local);
        if why.is_some() {
            self.error = why;
        }
    }

    /// It didn't go: kept in the thread as not sent, with why.
    pub fn not_sent(
        &mut self,
        d: &Delivery,
        why: &str,
        halted: Option<String>,
        now: DateTime<Utc>,
    ) {
        self.sending = false;
        self.drop_local(&d.local);
        self.keep_unsent(&d.text, why, halted, None, now);
    }

    /// Written to the run while Coach works (`steer`): shown at once, as the run now holds it.
    pub fn steered(&mut self, run: &str, text: &str, steered: &Steered, now: DateTime<Utc>) {
        let current = self.thread.steering.take();
        self.thread.steering = Some(steered_into(current, run, text, steered, now));
        if matches!(steered, Steered::Kept { sent: Some(_), .. }) {
            self.awaiting = Some(now);
        }
    }

    /// A steer that didn't reach tafs cloud stays in the thread as not sent.
    pub fn steer_not_sent(
        &mut self,
        text: &str,
        why: &str,
        halted: Option<String>,
        now: DateTime<Utc>,
    ) {
        self.keep_unsent(text, why, halted, None, now);
    }

    /// `settleSetupChat`: once a new conversation's first message went, its chat is the one shown.
    pub fn settle_chat(&mut self, session: &str) {
        if self.fresh {
            self.drawn.clear();
            self.chat = Some(session.into());
            self.fresh = false;
        }
    }

    /// `followNewSetupChat`: tafs cloud started a new chat for the message just sent to a chat
    /// that had handed a taf over; it's the one shown, with that message.
    pub fn follow_new_chat(&mut self, session: &str, new_session: bool, sent: &str) -> bool {
        let Some(open) = &self.chat else { return false };
        if !new_session || session.is_empty() || open == session {
            return false;
        }
        self.generation += 1;
        self.chat = Some(session.into());
        self.fresh = false;
        self.drawn.clear();
        self.sent.retain(|m| m.id() == sent);
        self.thread = SetupThread {
            messages: self.sent.clone(),
            ..SetupThread::default()
        };
        true
    }

    /// `markSetupSent`: the message on its way takes tafs cloud's id.
    fn mark_sent(&mut self, local: &str, server: &str) {
        for m in self.sent.iter_mut().chain(self.thread.messages.iter_mut()) {
            if m.id() == local && m.server_id.is_none() {
                m.server_id = Some(server.into());
            }
        }
    }

    /// The chat Stop stops (`stopSetup`): the one at work, else the thread's. The wait ends.
    pub fn stop(&mut self) -> Option<String> {
        let session = self
            .thread
            .working
            .clone()
            .or_else(|| self.thread.session())?;
        self.setup.as_ref()?;
        self.awaiting = None;
        Some(session)
    }

    /// Retry on a message that didn't go (`retrySetup`): its words to submit again, or none when
    /// its card was answered since (it's dropped then).
    pub fn retry(&mut self, id: u64) -> Option<(usize, Unsent)> {
        if self.sending {
            return None;
        }
        let at = self.unsent.iter().position(|u| u.id == id)?;
        let message = self.unsent.remove(at);
        if let Some(a) = Answer::parse(&message.text) {
            if self.thread.answer_to(&a.id).is_some() {
                return None;
            }
            let r = self.card_ref(&a.id);
            self.answered.insert(r);
        }
        Some((at, message))
    }

    /// A retry that wasn't taken goes back in its place, its card answerable again.
    pub fn retry_back(&mut self, at: usize, message: Unsent) {
        if let Some(a) = Answer::parse(&message.text) {
            let r = self.card_ref(&a.id);
            self.answered.remove(&r);
        }
        let at = at.min(self.unsent.len());
        self.unsent.insert(at, message);
    }

    /// Delete (`discardUnsent`): out of the thread, not sent. An answer's card can be answered
    /// again, on the card.
    pub fn discard(&mut self, id: u64) {
        let Some(message) = self.unsent.iter().find(|u| u.id == id).cloned() else {
            return;
        };
        self.unsent.retain(|u| u.id != id);
        if let Some(a) = Answer::parse(&message.text) {
            let r = self.card_ref(&a.id);
            self.answered.remove(&r);
        }
    }

    /// Edit (`editUnsent`): out of the thread, its words as the owner reads them.
    pub fn edit(&mut self, id: u64, w: &WordBook) -> Option<String> {
        let message = self.unsent.iter().find(|u| u.id == id)?.clone();
        self.unsent.retain(|u| u.id != id);
        if let Some(a) = Answer::parse(&message.text) {
            let r = self.card_ref(&a.id);
            self.answered.remove(&r);
        }
        crate::coach_lines::owner_words(w, &self.thread, &message.text).ok()
    }

    /// A card answered (`answer`): its ref, the answer as data and the chat it's in, or none
    /// when the card doesn't take that answer.
    pub fn answer(&mut self, id: &str, kind: Kind, text: &str) -> Option<CardSend> {
        self.unsent
            .retain(|u| Answer::parse(&u.text).is_none_or(|a| a.id != id));
        let setup = self.setup.clone()?;
        let value = answer_value(kind, text)?;
        let card_ref = self.card_ref(id);
        self.answered.insert(card_ref.clone());
        self.notes.remove(id);
        let session = self
            .thread
            .components
            .iter()
            .rev()
            .find(|c| c.card_ref() == card_ref)
            .map(|c| c.session.clone());
        Some(CardSend {
            setup,
            id: id.into(),
            card_ref,
            body: answer_body(&value, session.as_deref()),
        })
    }

    /// tafs cloud took it: the run goes on, and is followed as a message's.
    pub fn answer_taken(&mut self, a: &CardAnswered, now: DateTime<Utc>) {
        self.settle_chat(&a.session);
        self.awaiting = Some(now);
    }

    /// Not taken: the card can be answered again, with `note` under it when it says why.
    pub fn answer_not_taken(&mut self, card: &CardSend, note: Option<String>) {
        self.answered.remove(&card.card_ref);
        if let Some(n) = note {
            self.notes.insert(card.id.clone(), n);
        }
    }

    /// Nothing in it yet (`coachChatIsEmpty`): no message, card or answer, sent or on its way.
    /// New conversation keeps it.
    pub fn is_empty(&self) -> bool {
        self.improving.is_none()
            && self.thread.items().is_empty()
            && self.sent.is_empty()
            && self.unsent.is_empty()
            && self
                .thread
                .steering
                .as_ref()
                .is_none_or(|s| s.held.is_none())
            && !self.thinking()
    }

    /// A chat archived (`archiveSetupChat`): out of the list; the one shown gives way to a new
    /// conversation. True when it did.
    pub fn archived(&mut self, session: &str) -> bool {
        self.chats.retain(|c| c.session != session);
        let shown = match &self.chat {
            Some(c) => c == session,
            None => self.thread.session().as_deref() == Some(session),
        };
        if shown {
            self.new_chat();
        }
        shown
    }

    /// A new conversation (`newSetupChat`): an empty chat, and the next message starts it.
    pub fn new_chat(&mut self) {
        self.generation += 1;
        self.drawn.clear();
        self.chat = None;
        self.fresh = true;
        self.thread = SetupThread::default();
        self.sent.clear();
        self.awaiting = None;
        self.unsent.clear();
        self.ended = None;
        self.error = None;
        self.sending = false;
        self.improving = None;
    }

    /// `improveTaf`: a new conversation on the taf `id` (at tafs cloud) called `name`, whose first
    /// message goes through tafs cloud with the taf's context.
    pub fn improve(&mut self, id: &str, name: &str) {
        self.new_chat();
        self.improving = Some(Improving {
            id: id.into(),
            name: name.into(),
        });
    }

    /// One of Coach's chats reopened (`openSetupChat`): shown alone, and messages go on in it.
    pub fn open_chat(&mut self, session: &str) {
        self.generation += 1;
        self.drawn.clear();
        self.sending = false;
        self.chat = Some(session.into());
        self.fresh = false;
        self.unsent.clear();
        self.sent.clear();
        self.ended = None;
        self.improving = None;
    }
}

/// A card's answer on its way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardSend {
    pub setup: String,
    pub id: String,
    pub card_ref: String,
    pub body: Vec<u8>,
}

/// A steered message's key: its run, time and words, hashed.
fn steering_key(s: &Steering, m: &SteeringMessage) -> String {
    use sha2::{Digest, Sha256};
    let at = m.at.timestamp_millis() as f64 / 1000.0;
    let digest = Sha256::digest(format!("coach|{}|{at}|{}", s.run, m.text).as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

/// `followSetup`'s reads: every second while Coach works, else every three, up to the budget
/// after the owner last wrote.
#[derive(Debug, Clone, Default)]
pub struct Polls {
    last_owner: Option<String>,
    polls: u32,
}

impl Polls {
    /// How long to wait before the next look.
    pub fn interval(c: &Conversation) -> std::time::Duration {
        if c.thinking() {
            WORKING_POLL
        } else {
            POLL
        }
    }

    /// Whether to read the thread again now.
    pub fn read_now(&mut self, c: &Conversation) -> bool {
        let mine = c
            .thread
            .messages
            .iter()
            .rev()
            .find(|m| m.from == From::Owner)
            .map(ChatMessage::id);
        if mine != self.last_owner {
            self.last_owner = mine;
            self.polls = 0;
        }
        if self.polls >= POLL_BUDGET {
            return false;
        }
        self.polls += 1;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coach::{Question, QuestionChoice};

    fn v(b: &[u8]) -> Value {
        serde_json::from_slice(b).unwrap()
    }

    fn at(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
    }

    fn contract(name: &str) -> Value {
        let path = format!(
            "{}/../../shared/contracts/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let file: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        file["answer"].clone()
    }

    #[test]
    fn the_paths_escape_each_segment() {
        assert_eq!(messages_path("taf 1"), "/v1/tafs/taf%201/messages");
        assert_eq!(steer_path("taf-1"), "/v1/tafs/taf-1/steer");
        assert_eq!(stop_path("taf-1"), "/v1/tafs/taf-1/stop");
        assert_eq!(
            answer_path("taf-1", "a/b"),
            "/v1/tafs/taf-1/components/a%2Fb/answer"
        );
    }

    #[test]
    fn a_message_goes_where_coach_is_shown_and_follows_a_new_chat() {
        let fresh = ChatTarget::coach(true, Some("sess1"));
        assert_eq!(
            v(&message_body("Hi", &fresh, &[])),
            json!({"text": "Hi", "new_chat": true, "new_chat_after_handover": true})
        );
        let shown = ChatTarget::coach(false, Some("sess1"));
        assert_eq!(
            v(&message_body("Hi", &shown, &[])),
            json!({"text": "Hi", "session": "sess1", "new_chat_after_handover": true})
        );
        let live = ChatTarget::coach(false, None);
        assert_eq!(
            v(&message_body("Hi", &live, &[])),
            json!({"text": "Hi", "new_chat_after_handover": true})
        );
        assert_eq!(
            v(&steer_body("run2", "and refunds")),
            json!({"run": "run2", "text": "and refunds", "hold": true})
        );
        assert_eq!(v(&stop_body("sess1")), json!({"session": "sess1"}));
    }

    #[test]
    fn answers_decode_as_strictly_as_the_ipad() {
        assert_eq!(
            sent(br#"{"session":"s","new_session":false,"message":{"id":"run3.1"}}"#),
            Some(Sent {
                session: "s".into(),
                new_session: false,
                message: Some("run3.1".into())
            })
        );
        assert_eq!(
            sent(br#"{"session":"s","new_session":true}"#)
                .unwrap()
                .message,
            None
        );
        assert_eq!(sent(br#"{"session":"s"}"#), None);
        assert_eq!(
            sent(br#"{"session":"s","new_session":false,"message":{}}"#),
            None
        );
        assert_eq!(
            steered(br#"{"status":"RUNNING"}"#),
            Some(Steered::Taken {
                status: "RUNNING".into()
            })
        );
        assert_eq!(
            steered(br#"{"held":{"text":"x","at":"2026-10-16T12:00:00Z"}}"#),
            Some(Steered::Kept {
                sent: None,
                held: Some(SteeringMessage {
                    text: "x".into(),
                    at: at("2026-10-16T12:00:00Z")
                })
            })
        );
        assert_eq!(steered(br#"{"sent":{"run":"r"}}"#), None);
        assert!(stopped(br#"{"stopped":true}"#));
        assert!(stopped(br#"{"stopped":false}"#));
        assert!(!stopped(br#"{"stopped":"yes"}"#));
        assert_eq!(
            card_answered(br#"{"session":"s","run":"r","status":"RUNNING"}"#),
            Some(CardAnswered {
                session: "s".into(),
                run: "r".into(),
                status: "RUNNING".into()
            })
        );
        assert_eq!(card_answered(br#"{"session":"s","run":"r"}"#), None);
    }

    #[test]
    fn a_cards_answer_goes_as_data() {
        assert_eq!(
            v(&answer_body(
                &answer_value(Kind::Confirm, "live").unwrap(),
                Some("sess1")
            )),
            json!({"answer": "live", "session": "sess1"})
        );
        assert_eq!(answer_value(Kind::Confirm, ""), None);
        let q = answer_value(
            Kind::Questions,
            r#"{"limit":{"value":"50"},"days":{"values":["mon","tue"]},"why":{"own":"Busy"}}"#,
        )
        .unwrap();
        assert_eq!(
            v(&answer_body(&q, None)),
            json!({"answer": {"limit": {"value": "50"}, "days": {"values": ["mon", "tue"]}, "why": {"own": "Busy"}}})
        );
        // A field of another shape: not answers.
        assert_eq!(
            answer_value(Kind::Questions, r#"{"limit":{"value":5}}"#),
            None
        );
        assert_eq!(answer_value(Kind::Questions, "[]"), None);
        assert_eq!(answer_value(Kind::Notice, "x"), None);
    }

    #[test]
    fn this_computer_never_builds_an_approval_a_sign_in_or_a_drive_answer() {
        // View-only: whatever the text, the phone's cards get no answer from here.
        for kind in [Kind::Approval, Kind::SignIn, Kind::Drive] {
            for text in [
                "approved",
                "approved, signed in",
                "declined",
                "already approved: g_1",
                "signed in",
                "not now",
                "failed: no key",
            ] {
                assert_eq!(answer_value(kind, text), None, "{kind:?} {text}");
            }
        }
        // And so no answer to send for one, through the same path the app takes.
        let mut c = Conversation {
            setup: Some("coach".into()),
            ..Default::default()
        };
        assert!(c.answer("q1", Kind::Confirm, "live").is_some());
        assert!(c.answer("a1", Kind::Approval, "approved").is_none());
        assert!(c.answer("s1", Kind::SignIn, "signed in").is_none());
    }

    #[test]
    fn a_refusal_is_read_as_one_body() {
        let r = Refusal::read(
            403,
            br#"{"error":"forbidden","code":"ai_consent_required","consent":{}}"#,
        );
        assert!(r.wants_consent());
        let r = Refusal::read(
            409,
            br#"{"code":"no_plan","words":"Choose a plan","halted":"me"}"#,
        );
        assert_eq!(r.halted, None);
        assert!(!r.wants_consent());
        // A field of another shape: none of it.
        let r = Refusal::read(403, br#"{"code":"ai_consent_required","blocking":[{}]}"#);
        assert_eq!(r.code, None);
        let r = Refusal::read(409, br#"{"code":"held_full","halted":"taf"}"#);
        assert_eq!(r.halted.as_deref(), Some("taf"));
    }

    fn mine(text: &str, at_: &str, server: Option<&str>) -> ChatMessage {
        ChatMessage {
            at: at(at_),
            from: From::Owner,
            kind: MessageKind::Message,
            text: text.into(),
            session: PENDING.into(),
            door: None,
            work: None,
            attachments: vec![],
            server_id: server.map(str::to_string),
            answers: None,
            delivered: None,
            counted: None,
            refused: None,
        }
    }

    #[test]
    fn a_sent_message_stays_until_its_copy_lands_by_id() {
        let now = at("2026-10-16T12:01:00Z");
        let thread = vec![mine("Sí", "2026-10-16T12:00:00Z", Some("old"))];
        let sent = vec![mine("Sí", "2026-10-16T12:00:30Z", Some("new"))];
        // The same words with another id are never its copy.
        let (all, waiting) = merge(thread.clone(), &sent, now);
        assert_eq!((all.len(), waiting.len()), (2, 1));
        let mut copy = mine("Sí", "2026-10-16T12:00:31Z", Some("new"));
        copy.session = "sess1".into();
        let (all, waiting) = merge(vec![thread[0].clone(), copy], &sent, now);
        assert_eq!((all.len(), waiting.len()), (2, 0));
        // With no id, by its words; after two minutes, gone either way.
        let unmarked = vec![mine("Hi", "2026-10-16T12:00:30Z", None)];
        let (_, waiting) = merge(
            vec![mine("Hi", "2026-10-16T12:00:30Z", Some("x"))],
            &unmarked,
            now,
        );
        assert!(waiting.is_empty());
        let (_, waiting) = merge(vec![], &unmarked, at("2026-10-16T12:03:00Z"));
        assert!(waiting.is_empty());
        assert_eq!(
            stamp(Some(at("2026-10-16T12:05:00Z")), now),
            at("2026-10-16T12:05:00.001Z")
        );
        assert_eq!(stamp(None, now), now);
    }

    fn question(id: &str, multi: Option<bool>) -> Question {
        Question {
            id: id.into(),
            text: "?".into(),
            options: ["a", "b"]
                .iter()
                .map(|v| QuestionChoice {
                    value: v.to_string(),
                    label: v.to_uppercase(),
                    description: None,
                })
                .collect(),
            multi,
            note: None,
        }
    }

    #[test]
    fn a_questions_card_sends_once_each_question_is_answered() {
        let (one, many) = (question("one", None), question("many", Some(true)));
        let qs = vec![one.clone(), many.clone()];
        let mut s = QuestionSelection::default();
        s.pick(&one, "a");
        s.pick(&one, "b");
        assert_eq!(s.answer(&one), Some(QuestionPick::Choice("b".into())));
        assert_eq!(s.answered(&qs), 1);
        assert_eq!(s.send(&qs), None);
        s.pick(&many, "b");
        s.pick(&many, "a");
        assert_eq!(
            s.answer(&many),
            Some(QuestionPick::Choices(vec!["a".into(), "b".into()]))
        );
        assert_eq!(
            s.send(&qs).unwrap(),
            r#"{"many":{"values":["a","b"]},"one":{"value":"b"}}"#
        );
        // Something else… clears the picks; blank words are no answer.
        s.toggle_own(&one);
        assert_eq!(s.answer(&one), None);
        s.own.insert("one".into(), "  Mine ".into());
        assert_eq!(s.answer(&one), Some(QuestionPick::Own("Mine".into())));
        s.pick(&one, "a");
        assert!(!s.own_open.contains("one"));
    }

    fn conversation() -> Conversation {
        let t =
            SetupThread::read(&serde_json::to_vec(&contract("coach-conversation.json")).unwrap())
                .unwrap();
        let mut c = Conversation {
            setup: Some("taf-c".into()),
            chat: Some("sess1".into()),
            ..Conversation::default()
        };
        c.took(
            SetupThread { working: None, ..t },
            at("2026-10-16T13:00:00Z"),
        );
        c
    }

    #[test]
    fn sending_shows_the_message_then_takes_tafs_clouds_copy() {
        let mut c = conversation();
        let now = at("2026-10-16T13:00:00Z");
        let n = c.thread.messages.len();
        let Submit::Send(d) = c.submit("  More refunds ", now) else {
            panic!("not sent")
        };
        assert_eq!(d.text, "More refunds");
        assert_eq!(d.target, ChatTarget::coach(false, Some("sess1")));
        assert!(c.sending);
        assert_eq!(c.thread.messages.len(), n + 1);
        // Busy: nothing else is tried.
        assert_eq!(c.deliver("again", now), None);
        c.delivered(
            &d,
            &Sent {
                session: "sess1".into(),
                new_session: false,
                message: Some("run3.in".into()),
            },
            now,
        );
        assert!(c.thinking() && !c.sending);
        assert_eq!(
            c.thread.messages.last().unwrap().server_id.as_deref(),
            Some("run3.in")
        );
        // A read without it keeps it; one with it takes its place.
        let mut read = c.thread.clone();
        read.messages.pop();
        c.took(read.clone(), now);
        assert_eq!(c.thread.messages.len(), n + 1);
        let mut copy = mine("More refunds", "2026-10-16T13:00:01Z", Some("run3.in"));
        copy.session = "sess1".into();
        read.messages.push(copy);
        read.working = Some("sess1".into());
        read.working_run = Some("run3".into());
        c.took(read, now);
        assert_eq!(c.thread.messages.len(), n + 1);
        assert_eq!(c.thread.messages.last().unwrap().session, "sess1");
        assert_eq!(c.awaiting, None);
        // At work: Send writes to its run.
        assert_eq!(
            c.submit("and Stripe", now),
            Submit::Steer {
                run: "run3".into(),
                text: "and Stripe".into()
            }
        );
        c.steered(
            "run3",
            "and Stripe",
            &Steered::Taken {
                status: "RUNNING".into(),
            },
            now,
        );
        assert_eq!(
            c.thread.steering.as_ref().unwrap().untaken[0].text,
            "and Stripe"
        );
        assert_eq!(c.stop().as_deref(), Some("sess1"));
    }

    #[test]
    fn a_refused_message_goes_back_or_stays_as_not_sent() {
        let mut c = conversation();
        let now = at("2026-10-16T13:00:00Z");
        let n = c.thread.messages.len();
        let Submit::Send(d) = c.submit("Hi", now) else {
            panic!()
        };
        c.not_tried(&d, None);
        assert_eq!(c.thread.messages.len(), n);
        assert!(c.unsent.is_empty() && !c.sending);
        let Submit::Send(d) = c.submit("Hi", now) else {
            panic!()
        };
        c.not_sent(&d, "offline", None, now);
        assert_eq!(c.thread.messages.len(), n);
        assert_eq!(c.unsent[0].text, "Hi");
        let id = c.unsent[0].id;
        let (place, back) = c.retry(id).unwrap();
        assert!(c.unsent.is_empty());
        c.retry_back(place, back);
        assert_eq!(c.unsent.len(), 1);
        // Its menu: Copy copies it as written, Delete takes it out, unsent.
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let l = crate::coach_lines::unsent(&w, &c.thread, &c.unsent).unwrap();
        assert_eq!(
            (l[0].copy.as_str(), l[0].text.as_str(), l[0].delete.as_str()),
            ("Copy", "Hi", "Delete")
        );
        c.discard(id);
        assert!(c.unsent.is_empty());
        c.discard(id);
        assert!(c.submit(&"x".repeat(CHAT_LIMIT + 1), now) == Submit::Stay);
    }

    #[test]
    fn an_improvement_first_message_goes_with_the_tafs_context() {
        assert_eq!(improve_path("taf 1"), "/v1/tafs/taf%201/improve");
        assert_eq!(
            v(&improve_body("Make it faster")),
            json!({"text": "Make it faster"})
        );
        // The session's, as any message: not one of the device-signed routes.
        assert!(!crate::signed::signed_at_cloud(
            "POST",
            &improve_path("taf_1")
        ));
        let mut c = conversation();
        c.improve("taf_1", "Refunds");
        assert!(c.fresh && c.improving.is_some() && !c.is_empty());
        let now = at("2026-10-16T13:00:00Z");
        let Submit::Send(d) = c.submit("Make it faster", now) else {
            panic!()
        };
        c.improved(
            &d,
            &Sent {
                session: "sess7".into(),
                new_session: true,
                message: None,
            },
            now,
        );
        assert_eq!(c.improving, None);
        assert_eq!(c.chat.as_deref(), Some("sess7"));
        assert!(c.thinking() && !c.sending);
        // Another chat reopened is on no taf.
        c.improve("taf_1", "Refunds");
        c.open_chat("sess2");
        assert_eq!(c.improving, None);
    }

    #[test]
    fn a_new_conversation_starts_empty_and_settles_on_its_chat() {
        let mut c = conversation();
        c.new_chat();
        assert!(c.fresh && c.thread.messages.is_empty());
        let now = at("2026-10-16T13:00:00Z");
        let Submit::Send(d) = c.submit("New job", now) else {
            panic!()
        };
        assert!(d.target.new_chat && d.target.session.is_none());
        c.delivered(
            &d,
            &Sent {
                session: "sess9".into(),
                new_session: true,
                message: None,
            },
            now,
        );
        assert_eq!((c.chat.as_deref(), c.fresh), (Some("sess9"), false));
        // A chat that had handed a taf over: tafs cloud's new one is followed.
        let g = c.generation;
        assert!(c.follow_new_chat("sess10", true, &d.local));
        assert_eq!(c.chat.as_deref(), Some("sess10"));
        assert_eq!(c.generation, g + 1);
        assert_eq!(c.thread.messages.len(), 1);
    }

    #[test]
    fn a_card_answer_is_marked_until_tafs_cloud_takes_it() {
        let mut c = conversation();
        // The contract's questions card was written past: answering it goes as data.
        let send = c
            .answer("limits", Kind::Questions, r#"{"limit":{"value":"50"}}"#)
            .unwrap();
        assert_eq!(send.card_ref, "sess1#limits");
        assert_eq!(
            v(&send.body),
            json!({"answer": {"limit": {"value": "50"}}, "session": "sess1"})
        );
        assert!(c.is_answered("limits"));
        c.answer_not_taken(&send, Some("didn't reach".into()));
        assert!(!c.is_answered("limits"));
        assert_eq!(
            c.notes.get("limits").map(String::as_str),
            Some("didn't reach")
        );
        assert_eq!(c.answer("limits", Kind::Notice, "x"), None);
    }

    #[test]
    fn reads_go_on_for_the_budget_after_the_owner_last_wrote() {
        let mut c = conversation();
        let mut p = Polls::default();
        assert_eq!(Polls::interval(&c), POLL);
        for _ in 0..POLL_BUDGET {
            assert!(p.read_now(&c));
        }
        assert!(!p.read_now(&c));
        let now = at("2026-10-16T13:00:00Z");
        let Submit::Send(_) = c.submit("Again", now) else {
            panic!()
        };
        assert!(p.read_now(&c));
        c.awaiting = Some(now);
        assert_eq!(Polls::interval(&c), WORKING_POLL);
    }

    #[test]
    fn archiving_the_chat_shown_starts_a_new_one() {
        assert_eq!(
            archive_path("taf-1", "sess 1"),
            "/v1/tafs/taf-1/chats/sess%201/archive"
        );
        assert_eq!(
            unarchive_path("taf-1", "s"),
            "/v1/tafs/taf-1/chats/s/unarchive"
        );
        assert_eq!(archived_path("taf-1"), "/v1/tafs/taf-1/chats/archived");
        assert_eq!(archived(br#"{"archived":true}"#), Some(true));
        assert_eq!(archived(br#"{"archived":"yes"}"#), None);
        let mut c = conversation();
        assert!(!c.is_empty());
        assert!(!c.archived("other"));
        assert!(!c.fresh);
        assert!(c.archived("sess1"));
        assert!(c.fresh && c.is_empty());
        let mut c = conversation();
        c.open_chat("sess2");
        assert_eq!(c.chat.as_deref(), Some("sess2"));
        assert!(!c.fresh);
    }

    #[test]
    fn the_owners_message_keeps_its_place_when_tafs_clouds_copy_lands() {
        use crate::coach_lines::{conversation_lines, Line};
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let mut c = conversation();
        let now = at("2026-10-16T13:00:00Z");
        let Submit::Send(d) = c.submit("And chargebacks", now) else {
            panic!()
        };
        c.delivered(
            &d,
            &Sent {
                session: "sess1".into(),
                new_session: false,
                message: Some("run3.in".into()),
            },
            now,
        );
        let mode = crate::sidebar::Mode::Kiff;
        let drawn = conversation_lines(&w, &c, &[], mode, now).unwrap();
        c.drawn = drawn.order.clone();
        let last = |l: &crate::coach_lines::CoachLines| {
            l.lines
                .iter()
                .rev()
                .find_map(|x| match x {
                    Line::Owner { text, .. } => Some(text.clone()),
                    Line::Coach { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .unwrap()
        };
        assert_eq!(last(&drawn), "And chargebacks");
        // tafs cloud's copy carries its own time, before Coach's last words: it stays last.
        let mut read = c.thread.clone();
        read.messages.retain(|m| m.session != PENDING);
        let mut copy = mine("And chargebacks", "2026-10-16T12:00:30Z", Some("run3.in"));
        copy.session = "sess1".into();
        read.messages.push(copy);
        c.took(read.clone(), now);
        let steady = conversation_lines(&w, &c, &[], mode, now).unwrap();
        assert_eq!(last(&steady), "And chargebacks");
        // Drawn from nothing, it's in the thread's own order.
        c.drawn.clear();
        let own = conversation_lines(&w, &c, &[], mode, now).unwrap();
        assert_eq!(last(&own), "Refunds is set up.");
    }

    #[test]
    fn the_thread_is_in_its_latest_chat() {
        let c = conversation();
        assert_eq!(c.thread.session().as_deref(), Some("sess1"));
        assert_eq!(SetupThread::default().session(), None);
    }
}
