//! Coach's conversation (`TafsCloud.setupThread`, `SetupComponent`, `SetupThread`): which taf is
//! Coach's (`GET /v1/setup`), its chats (`GET /v1/tafs/{id}/chats`), and one conversation
//! (`GET /v1/tafs/{id}/conversation[/{session}]`), each decoded exactly as strictly or as
//! leniently as the iPad's decoders: a synthesized field fails the whole read when it is of
//! another shape, a field the iPad reads with `try?` is none instead, and a card or a line the
//! app can't read is left out, never the thread with it.

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use crate::work::segment;

type Read<T> = Result<T, ()>;

// MARK: Reading JSON as Swift's `Decodable` does.

fn field<'a>(v: &'a Map<String, Value>, key: &str) -> Option<&'a Value> {
    match v.get(key) {
        None | Some(Value::Null) => None,
        Some(x) => Some(x),
    }
}

/// `decode(String.self)`: there, and a string.
fn text(v: &Map<String, Value>, key: &str) -> Read<String> {
    field(v, key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or(())
}

/// `decodeIfPresent(String.self)`: absent or null is none, another shape fails.
fn text_if(v: &Map<String, Value>, key: &str) -> Read<Option<String>> {
    match field(v, key) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(()),
    }
}

/// `try? decodeIfPresent(String.self)`: anything but a string is none.
fn text_lenient(v: &Map<String, Value>, key: &str) -> Option<String> {
    text_if(v, key).ok().flatten()
}

fn bool_if(v: &Map<String, Value>, key: &str) -> Read<Option<bool>> {
    match field(v, key) {
        None => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        Some(_) => Err(()),
    }
}

fn int(v: &Value) -> Read<i64> {
    v.as_i64().ok_or(())
}

fn int_if(v: &Map<String, Value>, key: &str) -> Read<Option<i64>> {
    field(v, key).map(int).transpose()
}

/// `KIFFClient.decoder`'s dates: RFC 3339, with or without fractional seconds.
fn date_of(v: &Value) -> Read<DateTime<Utc>> {
    let s = v.as_str().ok_or(())?;
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|_| ())
}

fn date(v: &Map<String, Value>, key: &str) -> Read<DateTime<Utc>> {
    date_of(field(v, key).ok_or(())?)
}

fn object(v: &Value) -> Read<&Map<String, Value>> {
    v.as_object().ok_or(())
}

/// `decodeIfPresent([T].self)`: absent or null is none; not an array, or one element that fails,
/// fails.
fn array_if<T>(
    v: &Map<String, Value>,
    key: &str,
    each: impl Fn(&Value) -> Read<T>,
) -> Read<Option<Vec<T>>> {
    match field(v, key) {
        None => Ok(None),
        Some(Value::Array(a)) => a.iter().map(each).collect::<Read<Vec<T>>>().map(Some),
        Some(_) => Err(()),
    }
}

fn array<T>(v: &Map<String, Value>, key: &str, each: impl Fn(&Value) -> Read<T>) -> Read<Vec<T>> {
    array_if(v, key, each)?.ok_or(())
}

fn strings(v: &Value) -> Read<String> {
    v.as_str().map(str::to_string).ok_or(())
}

/// `ChatMessage.line`: as written, without the space around it; none for a blank one.
pub fn line(words: Option<&str>) -> Option<String> {
    let t = words?.trim();
    (!t.is_empty()).then(|| t.to_string())
}

// MARK: The setup read.

/// `GET /v1/setup`: the account's Coach, read as `setupRead`'s `CloudTaf?` (every field it
/// declares, as strictly), or none before it is started.
pub const SETUP_PATH: &str = "/v1/setup";

/// An answer of another shape than the iPad reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unreadable;

/// Coach's id at tafs cloud, from `GET /v1/setup`; `Ok(None)` when the account has no Coach yet.
/// An answer of another shape is an error, as the iPad's read throws.
pub fn setup_id(body: &[u8]) -> Result<Option<String>, Unreadable> {
    setup_id_of(body).map_err(|_| Unreadable)
}

fn setup_id_of(body: &[u8]) -> Read<Option<String>> {
    let v: Value = serde_json::from_slice(body).map_err(|_| ())?;
    let top = object(&v)?;
    text_if(top, "time_zone")?;
    let Some(s) = field(top, "setup") else {
        return Ok(None);
    };
    let s = object(s)?;
    let id = text(s, "id")?;
    text(s, "name")?;
    text(s, "job")?;
    text_if(s, "kiff_agent")?;
    text(s, "status")?;
    if let Some(d) = field(s, "next_run") {
        date_of(d)?;
    }
    if let Some(d) = field(s, "last_run") {
        date_of(d)?;
    }
    bool_if(s, "unscheduled")?;
    array_if(s, "results", |x| Ok(x.clone()))?;
    for k in [
        "time_zone_words",
        "schedule_time_zone",
        "schedule_time_zone_words",
        "next_run_words",
    ] {
        text_if(s, k)?;
    }
    Ok(Some(id))
}

// MARK: Chats.

/// One of Coach's chats (`TafsCloud.Chat`), named by its first words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chat {
    pub session: String,
    pub started: DateTime<Utc>,
    pub updated: DateTime<Utc>,
    pub title: String,
}

/// `GET /v1/tafs/{id}/chats`.
pub fn chats_path(taf: &str) -> String {
    format!("/v1/tafs/{}/chats", segment(taf))
}

/// The chats, newest first, as strictly as `chats(_:)`: one it can't read fails the list.
pub fn chats(body: &[u8]) -> Option<Vec<Chat>> {
    let v: Value = serde_json::from_slice(body).ok()?;
    array(object(&v).ok()?, "chats", |c| {
        let c = object(c)?;
        Ok(Chat {
            session: text(c, "session")?,
            started: date(c, "started")?,
            updated: date(c, "updated")?,
            title: text(c, "title")?,
        })
    })
    .ok()
}

/// The conversation Coach opens on at launch (`loadSetupThread`): the newest by its last change,
/// the earlier ones under Previous conversations, never several together.
pub fn newest(chats: &[Chat]) -> Option<&Chat> {
    // `max(by:)` keeps the last of equals.
    chats
        .iter()
        .fold(None, |best: Option<&Chat>, c| match best {
            Some(b) if c.updated < b.updated => Some(b),
            _ => Some(c),
        })
}

// MARK: The conversation.

/// `setupThread(_:session:proof:can:)`'s path: the conversation, or one chat of it, and `?can=`
/// only with a proof and steps signed here beyond connect and cards. This computer signs none,
/// so it never asks for them.
pub fn conversation_path(taf: &str, session: Option<&str>, proof: bool, can: &[&str]) -> String {
    let mut path = format!("/v1/tafs/{}/conversation", segment(taf));
    if let Some(s) = session {
        path.push('/');
        path.push_str(&segment(s));
    }
    if proof && !can.is_empty() {
        path.push_str("?can=");
        path.push_str(&can.join(","));
    }
    path
}

/// Who wrote a message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum From {
    Owner,
    Taf,
}

/// A message, or a run's kickoff.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Kickoff,
    Message,
}

/// A door's message or its answer (`DoorMark`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoorMark {
    pub app: String,
    pub from: Option<String>,
    pub subject: Option<String>,
    pub reply: Option<String>,
    pub why: Option<String>,
}

/// One step of a message's work (`Work.Step`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkStep {
    pub tool: String,
    pub error: bool,
    pub not_made: bool,
}

/// How long it worked, and the tools it called, in order (`Work`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Work {
    pub ms: i64,
    pub steps: Vec<WorkStep>,
}

/// A file attached to a message (`AttachmentRef`), never its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub id: String,
    pub kind: String,
    pub media_type: String,
    pub name: String,
    pub size: i64,
}

/// A refused connection's lines (`ChatMessage.Refused`), each tafs cloud's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    pub words: String,
    pub why: Option<String>,
    pub next: String,
}

impl Refused {
    /// Its lines, in order, the blank ones left out.
    pub fn lines(&self) -> Vec<String> {
        [
            Some(self.words.as_str()),
            self.why.as_deref(),
            Some(&self.next),
        ]
        .into_iter()
        .filter_map(line)
        .collect()
    }
}

/// One line of the thread (`ChatMessage`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub at: DateTime<Utc>,
    pub from: From,
    pub kind: MessageKind,
    pub text: String,
    pub session: String,
    pub door: Option<DoorMark>,
    pub work: Option<Work>,
    pub attachments: Vec<Attachment>,
    pub server_id: Option<String>,
    /// The card this owner's message answered, by id.
    pub answers: Option<String>,
    pub delivered: Option<String>,
    pub counted: Option<String>,
    pub refused: Option<Refused>,
}

impl ChatMessage {
    /// `ChatMessage.init(from:)`: its time, who, words and session strictly; any mark the app
    /// can't read leaves it a plain message.
    fn read(v: &Value) -> Read<ChatMessage> {
        let c = object(v)?;
        let at = date(c, "at")?;
        let from = match text(c, "from")?.as_str() {
            "owner" => From::Owner,
            "taf" => From::Taf,
            _ => return Err(()),
        };
        let kind = match field(c, "kind").and_then(Value::as_str) {
            Some("kickoff") => MessageKind::Kickoff,
            _ => MessageKind::Message,
        };
        Ok(ChatMessage {
            at,
            from,
            kind,
            text: text(c, "text")?,
            session: text(c, "session")?,
            door: field(c, "door").and_then(|d| {
                let d = object(d).ok()?;
                Some(DoorMark {
                    app: text(d, "app").ok()?,
                    from: text_if(d, "from").ok()?,
                    subject: text_if(d, "subject").ok()?,
                    reply: text_if(d, "reply").ok()?,
                    why: text_if(d, "why").ok()?,
                })
            }),
            work: field(c, "work").and_then(|w| {
                let w = object(w).ok()?;
                Some(Work {
                    ms: int(field(w, "ms")?).ok()?,
                    steps: array(w, "steps", |s| {
                        let s = object(s)?;
                        Ok(WorkStep {
                            tool: text(s, "tool")?,
                            error: bool_if(s, "error").ok().flatten().unwrap_or(false),
                            not_made: bool_if(s, "not_made").ok().flatten().unwrap_or(false),
                        })
                    })
                    .ok()?,
                })
            }),
            attachments: array_if(c, "attachments", |a| {
                let a = object(a)?;
                Ok(Attachment {
                    id: text(a, "id")?,
                    kind: text(a, "kind")?,
                    media_type: text(a, "media_type")?,
                    name: text(a, "name")?,
                    size: int(field(a, "size").ok_or(())?)?,
                })
            })
            .ok()
            .flatten()
            .unwrap_or_default(),
            server_id: text_lenient(c, "id"),
            answers: text_lenient(c, "answers"),
            delivered: line(text_lenient(c, "delivered").as_deref()),
            counted: line(text_lenient(c, "counted").as_deref()),
            refused: field(c, "refused")
                .and_then(|r| {
                    let r = object(r).ok()?;
                    Some(Refused {
                        words: text_lenient(r, "words").unwrap_or_default(),
                        why: line(text_lenient(r, "why").as_deref()),
                        next: text_lenient(r, "next").unwrap_or_default(),
                    })
                })
                .filter(|r| !r.lines().is_empty()),
        })
    }

    /// The refused connection drawn under a taf's words: none on an owner's message or a kickoff.
    pub fn refused_lines(&self) -> Option<&Refused> {
        if self.from == From::Taf && self.kind != MessageKind::Kickoff {
            self.refused.as_ref()
        } else {
            None
        }
    }

    /// `ChatMessage.id`: a taf's message by its time and kind, an owner's by tafs cloud's id, else
    /// its words.
    pub fn id(&self) -> String {
        let at = self.at.timestamp_millis();
        match (self.from, &self.server_id) {
            (From::Taf, _) => format!(
                "{}|{at}|taf|{}",
                self.session,
                if self.kind == MessageKind::Kickoff {
                    "kickoff"
                } else {
                    "message"
                }
            ),
            (From::Owner, Some(id)) => format!("{}|owner|{id}", self.session),
            (From::Owner, None) => format!("{}|{at}|owner|{}", self.session, self.text),
        }
    }
}

/// What a component is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    Steps,
    Notice,
    Confirm,
    Questions,
    Approval,
    SignIn,
    Result,
    Knob,
    Drive,
}

impl Kind {
    pub fn from_raw(s: &str) -> Option<Kind> {
        Some(match s {
            "steps" => Kind::Steps,
            "notice" => Kind::Notice,
            "confirm" => Kind::Confirm,
            "questions" => Kind::Questions,
            "approval" => Kind::Approval,
            "sign_in" => Kind::SignIn,
            "result" => Kind::Result,
            "knob" => Kind::Knob,
            "drive" => Kind::Drive,
            _ => return None,
        })
    }

    pub fn raw(self) -> &'static str {
        match self {
            Kind::Steps => "steps",
            Kind::Notice => "notice",
            Kind::Confirm => "confirm",
            Kind::Questions => "questions",
            Kind::Approval => "approval",
            Kind::SignIn => "sign_in",
            Kind::Result => "result",
            Kind::Knob => "knob",
            Kind::Drive => "drive",
        }
    }
}

/// A plan step: pending, active, done, failed or skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub label: String,
    pub status: String,
}

/// A confirm card's option.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub value: String,
    pub label: String,
}

/// One of a questions card's choices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionChoice {
    pub value: String,
    pub label: String,
    pub description: Option<String>,
}

/// One of several questions asked at once: every word the agent's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Question {
    pub id: String,
    pub text: String,
    pub options: Vec<QuestionChoice>,
    pub multi: Option<bool>,
    pub note: Option<String>,
}

/// The taf a result hands over (`SetupComponent.Taf`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taf {
    pub agent: String,
    pub name: String,
    pub job: Option<String>,
    pub draft: Option<String>,
    pub improves: Option<String>,
    pub note: Option<String>,
    pub try_with: Option<String>,
    pub notes: Option<String>,
    pub unscheduled: Option<bool>,
}

/// On an approval whose connection is already connected, tafs cloud's mark (`Connection`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connection {
    pub done: Option<bool>,
    pub words: Option<String>,
}

impl Connection {
    /// tafs cloud's line, when it says the connection is done and gives words for it.
    pub fn done_words(&self) -> Option<&str> {
        if self.done != Some(true) {
            return None;
        }
        self.words.as_deref().filter(|w| !w.trim().is_empty())
    }
}

/// What an approval's fingerprint covers, as tafs cloud marks it (`Signs`), read leniently: a
/// field of another shape is none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signs {
    pub card: Option<bool>,
    pub words: Option<String>,
    pub takes: Option<String>,
}

impl Signs {
    /// The Card isn't signed by this approval: it comes next.
    pub fn card_next(&self) -> bool {
        self.card == Some(false)
    }

    /// Approve is a tap inside an open approval window.
    pub fn tap(&self) -> bool {
        self.takes.as_deref() == Some("tap")
    }
}

/// tafs cloud's mark that the app's sign-in can't be narrowed to the job (`Wider`), read
/// leniently: a mark of another shape is no mark, never the card unread.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Wider {
    pub words: Option<String>,
}

/// One thing Coach shows in its chat (`SetupComponent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Component {
    pub kind: Kind,
    pub id: String,
    pub title: Option<String>,
    pub text: Option<String>,
    /// info, success, warning or error.
    pub tone: Option<String>,
    pub steps: Option<Vec<Step>>,
    pub options: Option<Vec<Choice>>,
    pub questions: Option<Vec<Question>>,
    pub grant: Option<String>,
    pub draft: Option<String>,
    pub service: Option<String>,
    pub taf: Option<Taf>,
    pub tools: Option<Vec<String>>,
    pub yes: Option<String>,
    pub no: Option<String>,
    pub note: Option<String>,
    pub version: Option<i64>,
    pub taf_id: Option<String>,
    pub waits: Option<String>,
    pub connection: Option<Connection>,
    pub signs: Option<Signs>,
    pub wider: Option<Wider>,
}

/// A blank line of words is none (`Signs`, `Wider`).
fn words_of(s: Option<String>) -> Option<String> {
    s.filter(|w| !w.trim().is_empty())
}

impl Component {
    /// The synthesized decoder: every declared field as its type, or the card fails; `signs` and
    /// `wider` as their own lenient decoders.
    fn read(v: &Value) -> Read<Component> {
        let c = object(v)?;
        let kind = Kind::from_raw(&text(c, "type")?).ok_or(())?;
        let id = text(c, "id")?;
        let steps = array_if(c, "steps", |s| {
            let s = object(s)?;
            Ok(Step {
                label: text(s, "label")?,
                status: text(s, "status")?,
            })
        })?;
        let options = array_if(c, "options", |o| {
            let o = object(o)?;
            Ok(Choice {
                value: text(o, "value")?,
                label: text(o, "label")?,
            })
        })?;
        let questions = array_if(c, "questions", |q| {
            let q = object(q)?;
            Ok(Question {
                id: text(q, "id")?,
                text: text(q, "text")?,
                options: array(q, "options", |o| {
                    let o = object(o)?;
                    Ok(QuestionChoice {
                        value: text(o, "value")?,
                        label: text(o, "label")?,
                        description: text_if(o, "description")?,
                    })
                })?,
                multi: bool_if(q, "multi")?,
                note: text_if(q, "note")?,
            })
        })?;
        let taf = match field(c, "taf") {
            None => None,
            Some(t) => {
                let t = object(t)?;
                Some(Taf {
                    agent: text(t, "agent")?,
                    name: text(t, "name")?,
                    job: text_if(t, "job")?,
                    draft: text_if(t, "draft")?,
                    improves: text_if(t, "improves")?,
                    note: text_if(t, "note")?,
                    try_with: text_if(t, "try")?,
                    notes: text_if(t, "notes")?,
                    unscheduled: bool_if(t, "unscheduled")?,
                })
            }
        };
        let connection = match field(c, "connection") {
            None => None,
            Some(x) => {
                let x = object(x)?;
                Some(Connection {
                    done: bool_if(x, "done")?,
                    words: text_if(x, "words")?,
                })
            }
        };
        // `Signs.init(from:)`: a keyed container or the card fails; each field leniently.
        let signs = match field(c, "signs") {
            None => None,
            Some(x) => {
                let x = object(x)?;
                Some(Signs {
                    card: bool_if(x, "card").ok().flatten(),
                    words: words_of(text_lenient(x, "words")),
                    takes: text_lenient(x, "takes"),
                })
            }
        };
        // `Wider.init(from:)`: anything but an object is a mark with no words.
        let wider = field(c, "wider").map(|x| Wider {
            words: x
                .as_object()
                .and_then(|x| words_of(text_lenient(x, "words"))),
        });
        Ok(Component {
            kind,
            id,
            title: text_if(c, "title")?,
            text: text_if(c, "text")?,
            tone: text_if(c, "tone")?,
            steps,
            options,
            questions,
            grant: text_if(c, "grant")?,
            draft: text_if(c, "draft")?,
            service: text_if(c, "service")?,
            taf,
            tools: array_if(c, "tools", strings)?,
            yes: text_if(c, "yes")?,
            no: text_if(c, "no")?,
            note: text_if(c, "note")?,
            version: int_if(c, "version")?,
            taf_id: text_if(c, "taf_id")?,
            waits: text_if(c, "waits")?,
            connection,
            signs,
            wider,
        })
    }

    /// The wider line's words (`widerWords`), when tafs cloud sent some.
    pub fn wider_words(&self) -> Option<&str> {
        self.wider.as_ref()?.words.as_deref()
    }

    /// Whether the run waits on it, as its run marks it (`SetupThread.waits`).
    pub fn waits(&self) -> bool {
        self.waits.as_deref().is_some_and(|w| !w.is_empty())
    }
}

/// What tafs cloud did with a result it answered itself (`CardAnswer.Outcome`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Outcome {
    pub taf_id: Option<String>,
    pub version: Option<i64>,
    pub failed: Option<String>,
    /// tafs cloud's words when the taf was made with no plan, trimmed; none when it has one.
    pub plan_needed: Option<String>,
}

/// A card's answer as Coach's run recorded it (`CardAnswer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardAnswer {
    pub state: String,
    pub words: String,
    pub by: Option<String>,
    pub outcome: Option<Outcome>,
    pub at: Option<DateTime<Utc>>,
}

impl CardAnswer {
    fn read(v: &Value) -> Read<CardAnswer> {
        let c = object(v)?;
        Ok(CardAnswer {
            state: text_if(c, "state")?.unwrap_or_else(|| "answered".into()),
            words: text_if(c, "words")?.unwrap_or_default(),
            by: text_if(c, "by")?,
            outcome: field(c, "outcome").and_then(|o| {
                let o = object(o).ok()?;
                Some(Outcome {
                    taf_id: text_lenient(o, "taf_id"),
                    version: int_if(o, "version").ok().flatten(),
                    failed: text_lenient(o, "failed"),
                    plan_needed: field(o, "plan_needed")
                        .and_then(|p| {
                            let p = object(p).ok()?;
                            text_if(p, "words").ok()?
                        })
                        .map(|w| w.trim().to_string())
                        .filter(|w| !w.is_empty()),
                })
            }),
            at: text_lenient(c, "at").and_then(|s| date_of(&Value::String(s)).ok()),
        })
    }
}

/// A component as the conversation lists it (`TafsCloud.Shown`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shown {
    pub at: DateTime<Utc>,
    pub session: String,
    pub run: String,
    pub component: Component,
    pub answer: Option<CardAnswer>,
    pub key: Option<String>,
}

impl Shown {
    fn read(v: &Value) -> Read<Shown> {
        let c = object(v)?;
        Ok(Shown {
            at: date(c, "at")?,
            session: text(c, "session")?,
            run: text(c, "run")?,
            component: Component::read(field(c, "component").ok_or(())?)?,
            // An answer the app can't read leaves the card unanswered, never the card out.
            answer: field(c, "answer").and_then(|a| CardAnswer::read(a).ok()),
            key: text_lenient(c, "key"),
        })
    }
}

/// A component as the thread holds it (`ShownComponent`): where it first appeared, its latest
/// version, and its answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShownComponent {
    pub first_at: DateTime<Utc>,
    pub session: String,
    pub run: String,
    pub component: Component,
    pub answer: Option<CardAnswer>,
    pub key: Option<String>,
}

impl ShownComponent {
    /// Which card it is: tafs cloud's key, else its conversation and id.
    pub fn card_ref(&self) -> String {
        self.key
            .clone()
            .unwrap_or_else(|| format!("{}#{}", self.session, self.component.id))
    }
}

/// How a task Coach handed a taf went (`CoachLine`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachLine {
    pub at: DateTime<Utc>,
    pub taf: String,
    pub said: String,
    pub away: Option<bool>,
}

/// A drive's end in the thread (`DriveRecap`): what Coach did in the app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DriveRecap {
    pub id: String,
    pub session: Option<String>,
    pub task: String,
    pub why: Option<String>,
    pub words: Option<String>,
    pub at: DateTime<Utc>,
}

/// A message written to Coach's run (`Steering.Message`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringMessage {
    pub text: String,
    pub at: DateTime<Utc>,
}

/// What was written to Coach's latest run that it hasn't read (`Steering`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Steering {
    pub run: String,
    pub status: String,
    pub untaken: Vec<SteeringMessage>,
    pub held: Option<SteeringMessage>,
    pub not_sent: Vec<(SteeringMessage, String)>,
}

impl Steering {
    fn read(v: &Value) -> Read<Steering> {
        let c = object(v)?;
        let message = |m: &Value| -> Read<SteeringMessage> {
            let m = object(m)?;
            Ok(SteeringMessage {
                text: text(m, "text")?,
                at: date(m, "at")?,
            })
        };
        let untaken = match field(c, "untaken") {
            Some(Value::Array(a)) => a.iter().filter_map(|m| message(m).ok()).collect(),
            _ => return Err(()),
        };
        Ok(Steering {
            run: text(c, "run")?,
            status: text(c, "status")?,
            untaken,
            held: field(c, "held").and_then(|m| message(m).ok()),
            not_sent: match field(c, "not_sent") {
                Some(Value::Array(a)) => a
                    .iter()
                    .filter_map(|m| {
                        let why = text(object(m).ok()?, "why").ok()?;
                        Some((message(m).ok()?, why))
                    })
                    .collect(),
                _ => vec![],
            },
        })
    }

    /// The run is over: what it didn't read, it never will.
    pub fn over(&self) -> bool {
        matches!(self.status.as_str(), "COMPLETED" | "ERROR" | "CANCELLED")
    }
}

/// Coach's thread (`SetupThread`): the messages, and the components where they first appeared.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetupThread {
    pub messages: Vec<ChatMessage>,
    pub components: Vec<ShownComponent>,
    /// The session Coach is at work in now; none when idle.
    pub working: Option<String>,
    pub coach_lines: Vec<CoachLine>,
    pub drives: Vec<DriveRecap>,
    pub working_run: Option<String>,
    pub steering: Option<Steering>,
}

/// The thread in order (`SetupThread.Item`), the plan left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item<'a> {
    Message(&'a ChatMessage),
    Component(&'a ShownComponent),
    Drive(&'a DriveRecap),
}

impl Item<'_> {
    pub fn id(&self) -> String {
        match self {
            Item::Message(m) => format!("m|{}", m.id()),
            Item::Component(c) => format!("c|{}", c.card_ref()),
            Item::Drive(d) => format!("d|{}", d.id),
        }
    }

    pub fn at(&self) -> DateTime<Utc> {
        match self {
            Item::Message(m) => m.at,
            Item::Component(c) => c.first_at,
            Item::Drive(d) => d.at,
        }
    }
}

/// The owner's answer to a component (`SetupThread.Answer`): contract 1's
/// "[<type> <id>] <answer>", or what Coach's run recorded on the card.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub kind: String,
    pub id: String,
    pub text: String,
    /// The recorded words for it, when it came as data.
    pub words: Option<String>,
}

impl Answer {
    /// "[<type> <id>] <answer>", of a type the app knows.
    pub fn parse(message: &str) -> Option<Answer> {
        let rest = message.strip_prefix('[')?;
        let close = rest.find(']')?;
        let head = &rest[..close];
        let mut parts = head.splitn(2, ' ');
        let (kind, id) = (parts.next()?, parts.next()?);
        if kind.is_empty() || id.is_empty() {
            return None;
        }
        Kind::from_raw(kind)?;
        Some(Answer {
            kind: kind.into(),
            id: id.into(),
            text: rest[close + 1..]
                .trim_matches(|c: char| c.is_whitespace() && c != '\n' && c != '\r')
                .into(),
            words: None,
        })
    }

    pub fn failed(&self) -> bool {
        self.text.starts_with("failed")
    }

    /// Why it failed, after "failed: ".
    pub fn why(&self) -> Option<String> {
        if !self.failed() {
            return None;
        }
        let rest = self.text["failed".len()..].trim_start_matches([':', ' ']);
        (!rest.is_empty()).then(|| rest.to_string())
    }
}

/// Where a new taf's hand-over stands (`SetupThread.HandOver`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HandOver {
    SetUp,
    Failed(String),
    SettingUp,
}

/// What became of an improvement's result (`SetupThread.Improvement`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Improvement {
    Made(i64),
    Failed(String),
    Preparing,
}

impl SetupThread {
    /// `setupThread(decoding:)`: the messages strictly, everything else as leniently as the iPad.
    pub fn read(body: &[u8]) -> Option<SetupThread> {
        let v: Value = serde_json::from_slice(body).ok()?;
        Self::read_value(&v).ok()
    }

    fn read_value(v: &Value) -> Read<SetupThread> {
        let r = object(v)?;
        let messages = array(r, "messages", ChatMessage::read)?;
        // Each card the app can't read is left out, never the thread.
        let components = array_if(r, "components", |c| Ok(Shown::read(c).ok()))?
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let working = match field(r, "working") {
            None => None,
            Some(w) => {
                let w = object(w)?;
                Some((text(w, "session")?, text_if(w, "run")?))
            }
        };
        let coach_lines = array_if(r, "coach_lines", |l| {
            Ok((|| -> Read<CoachLine> {
                let l = object(l)?;
                Ok(CoachLine {
                    at: date(l, "at")?,
                    taf: text(l, "taf")?,
                    said: text(l, "said")?,
                    away: bool_if(l, "away")?,
                })
            })()
            .ok())
        })?
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect();
        // Of the thread's items, only a drive's end: the rest is read from messages and cards.
        let drives = array_if(r, "items", |i| {
            Ok((|| -> Option<DriveRecap> {
                let i = i.as_object()?;
                if field(i, "type")?.as_str()? != "drive" {
                    return None;
                }
                let d = object(field(i, "drive")?).ok()?;
                Some(DriveRecap {
                    id: text(d, "id").ok()?,
                    session: text_if(d, "session").ok()?,
                    task: text(d, "task").ok()?,
                    why: text_if(d, "why").ok()?,
                    words: text_if(d, "words").ok()?,
                    at: date(d, "at").ok()?,
                })
            })())
        })?
        .unwrap_or_default()
        .into_iter()
        .flatten()
        .collect();
        let steering = field(r, "steering").and_then(|s| Steering::read(s).ok());
        Ok(SetupThread {
            messages,
            components: Self::fold(components),
            working: working.as_ref().map(|w| w.0.clone()),
            working_run: working.and_then(|w| w.1),
            coach_lines,
            drives,
            steering,
        })
    }

    /// `fold`: the latest version of each card, by its ref, at the time it first appeared, with
    /// the answer tafs cloud put on its latest.
    pub fn fold(shown: Vec<Shown>) -> Vec<ShownComponent> {
        let mut out: Vec<ShownComponent> = Vec::new();
        for s in shown {
            let card = s
                .key
                .clone()
                .unwrap_or_else(|| format!("{}#{}", s.session, s.component.id));
            if let Some(o) = out.iter_mut().find(|o| o.card_ref() == card) {
                o.component = s.component;
                o.answer = s.answer;
            } else {
                out.push(ShownComponent {
                    first_at: s.at,
                    session: s.session,
                    run: s.run,
                    component: s.component,
                    answer: s.answer,
                    key: s.key,
                });
            }
        }
        out
    }

    /// The plan, pinned above the thread.
    pub fn plan(&self) -> Option<&ShownComponent> {
        self.components
            .iter()
            .rev()
            .find(|c| c.component.kind == Kind::Steps)
    }

    /// Where a card goes in the thread (`placedAt`): one that waits on the owner after what
    /// Coach wrote with it, up to the owner's next message or its answer.
    fn placed_at(&self, c: &ShownComponent) -> DateTime<Utc> {
        if !c.component.waits() {
            return c.first_at;
        }
        if c.answer.as_ref().is_some_and(|a| a.at.is_none()) {
            return c.first_at;
        }
        let next_owner = self
            .messages
            .iter()
            .find(|m| m.from == From::Owner && m.at > c.first_at)
            .map(|m| m.at);
        let answered = c.answer.as_ref().and_then(|a| a.at);
        let end = match (next_owner, answered) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        self.messages
            .iter()
            .rev()
            .find(|m| {
                m.from == From::Taf
                    && m.session == c.session
                    && m.at >= c.first_at
                    && end.is_none_or(|e| m.at < e)
            })
            .map_or(c.first_at, |m| m.at)
    }

    /// The thread in order (`items`), the plan left out, each once by its id.
    pub fn items(&self) -> Vec<Item<'_>> {
        let mut placed: Vec<(Item, DateTime<Utc>)> = self
            .messages
            .iter()
            .map(|m| (Item::Message(m), m.at))
            .collect();
        placed.extend(
            self.components
                .iter()
                .filter(|c| c.component.kind != Kind::Steps)
                .map(|c| (Item::Component(c), self.placed_at(c))),
        );
        placed.extend(self.drives.iter().map(|d| (Item::Drive(d), d.at)));
        // A stable sort: a message before a component placed at its time.
        placed.sort_by_key(|p| p.1);
        let mut seen = std::collections::HashSet::new();
        placed
            .into_iter()
            .map(|p| p.0)
            .filter(|i| seen.insert(i.id()))
            .collect()
    }

    /// A card's recorded answer in the form the cards read (`answer(_:to:)`): none for one that
    /// stopped waiting with no answer, or that the owner wrote past.
    pub fn recorded(recorded: &CardAnswer, c: &Component) -> Option<Answer> {
        if recorded.state != "answered" {
            return None;
        }
        let words = &recorded.words;
        let failed = words.find(" · it didn't go through: ").map(|i| {
            format!(
                "failed: {}",
                &words[i + " · it didn't go through: ".len()..]
            )
        });
        let text = match c.kind {
            Kind::Confirm => c
                .options
                .as_ref()
                .and_then(|o| o.iter().find(|o| &o.label == words))
                .map_or_else(|| words.clone(), |o| o.value.clone()),
            Kind::Approval => failed.unwrap_or_else(|| {
                if words == "Declined" {
                    "declined".into()
                } else {
                    "approved".into()
                }
            }),
            Kind::SignIn => failed.unwrap_or_else(|| {
                if words == "Signed in" {
                    "signed in".into()
                } else {
                    "not now".into()
                }
            }),
            Kind::Drive => {
                if words.to_lowercase().starts_with("approved") {
                    "approved".into()
                } else {
                    "declined".into()
                }
            }
            _ => words.clone(),
        };
        Some(Answer {
            kind: c.kind.raw().into(),
            id: c.id.clone(),
            text,
            words: Some(words.clone()),
        })
    }

    /// The owner's latest answer to a component (`answer(to:)`): the one recorded on the card,
    /// else one sent the old way, as a message, since the card appeared.
    pub fn answer_to(&self, id: &str) -> Option<Answer> {
        let shown = self.components.iter().rev().find(|c| c.component.id == id);
        if let Some(s) = shown {
            if let Some(a) = s
                .answer
                .as_ref()
                .and_then(|r| Self::recorded(r, &s.component))
            {
                return Some(a);
            }
        }
        let since = shown.map(|s| s.first_at);
        self.messages
            .iter()
            .rev()
            .filter(|m| m.from == From::Owner && since.is_none_or(|s| m.at >= s))
            .filter_map(|m| Answer::parse(&m.text))
            .find(|a| a.id == id)
    }

    /// Whether a newer approval asks about the same connection (`replaced`).
    pub fn replaced(&self, id: &str) -> bool {
        let Some(shown) = self.components.iter().find(|c| c.component.id == id) else {
            return false;
        };
        let Some(grant) = &shown.component.grant else {
            return false;
        };
        self.components.iter().any(|o| {
            o.component.id != id
                && o.component.kind == Kind::Approval
                && o.component.grant.as_ref() == Some(grant)
                && o.first_at > shown.first_at
        })
    }

    fn outcome(&self, card: &str) -> Option<&Outcome> {
        self.components
            .iter()
            .find(|c| c.card_ref() == card)?
            .answer
            .as_ref()?
            .outcome
            .as_ref()
    }

    /// Where a new taf's hand-over stands (`handOver`): `listed`, it's already on the list.
    pub fn hand_over(&self, card: &str, listed: bool) -> HandOver {
        let outcome = self.outcome(card);
        if let Some(why) = outcome.and_then(|o| o.failed.clone()) {
            return HandOver::Failed(why);
        }
        if outcome.is_some_and(|o| o.taf_id.is_some()) || listed {
            return HandOver::SetUp;
        }
        HandOver::SettingUp
    }

    /// tafs cloud's words when the taf was made with no plan (`planNeeded`).
    pub fn plan_needed(&self, card: &str) -> Option<String> {
        self.outcome(card)?.plan_needed.clone()
    }

    /// What became of an improvement (`improvement`).
    pub fn improvement(&self, card: &str) -> Improvement {
        let outcome = self.outcome(card);
        if let Some(why) = outcome.and_then(|o| o.failed.clone()) {
            return Improvement::Failed(why);
        }
        if let Some(n) = outcome.and_then(|o| o.version) {
            return Improvement::Made(n);
        }
        Improvement::Preparing
    }
}

/// `KIFFMapping.vendorWords`: "stripe-mcp" or "mcp.stripe.com" → "Stripe".
pub fn vendor_words(vendor: &str) -> String {
    let mut v = vendor.to_lowercase();
    if let Some(rest) = v.strip_prefix("mcp.") {
        v = rest.to_string();
    }
    for suffix in ["-mcp", "_mcp", ".mcp"] {
        if let Some(rest) = v.strip_suffix(suffix) {
            v = rest.to_string();
        }
    }
    if let Some(dot) = v.find('.') {
        v.truncate(dot);
    }
    let mut chars = v.chars();
    match chars.next() {
        None => vendor.to_string(),
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().replace('-', " "),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn contract(name: &str) -> Value {
        let path = format!(
            "{}/../../shared/contracts/{name}",
            env!("CARGO_MANIFEST_DIR")
        );
        let file: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        file["answer"].clone()
    }

    fn thread(v: &Value) -> Option<SetupThread> {
        SetupThread::read(&serde_json::to_vec(v).unwrap())
    }

    #[test]
    fn the_contracts_conversation_reads_as_the_ipad_reads_it() {
        let t = thread(&contract("coach-conversation.json")).unwrap();
        assert_eq!(t.messages.len(), 4);
        let reply = &t.messages[1];
        assert_eq!(reply.from, From::Taf);
        assert_eq!(reply.server_id.as_deref(), Some("run2.3"));
        let work = reply.work.as_ref().unwrap();
        assert_eq!(work.ms, 61000);
        assert_eq!(work.steps[0].tool, "planque");
        assert_eq!(
            reply.refused_lines().unwrap().lines(),
            [
                "Coach couldn't connect Shippo yet",
                "Why: managed sign-in isn't set up for this app",
                "Next: ask Coach to connect Shippo with its key instead, or to use another app."
            ]
        );
        assert_eq!(t.messages[2].answers.as_deref(), Some("limits"));
        assert_eq!(t.components.len(), 3);
        let q = &t.components[0];
        assert_eq!(q.card_ref(), "sess1#limits");
        assert_eq!(q.component.kind, Kind::Questions);
        assert_eq!(
            q.component.questions.as_ref().unwrap()[0].options[0].label,
            "€50"
        );
        assert_eq!(q.answer.as_ref().unwrap().state, "wrote");
        // Written past, not answered: the card reads as not answered.
        assert_eq!(t.answer_to("limits"), None);
        let a = &t.components[1].component;
        assert_eq!(a.kind, Kind::Approval);
        assert_eq!(a.grant.as_deref(), Some("grt_1"));
        assert_eq!(a.wider_words(), None);
        assert_eq!(t.answer_to("connect-stripe").unwrap().text, "approved");
        let r = &t.components[2];
        assert_eq!(r.component.taf.as_ref().unwrap().name, "Refunds");
        assert_eq!(
            r.answer
                .as_ref()
                .unwrap()
                .outcome
                .as_ref()
                .unwrap()
                .taf_id
                .as_deref(),
            Some("taf-aaaa0000000000000000")
        );
        assert_eq!(t.hand_over("sess1#done", false), HandOver::SetUp);
        assert_eq!(t.working.as_deref(), Some("sess1"));
        assert_eq!(t.working_run.as_deref(), Some("run2"));
        assert_eq!(t.steering, None);
        assert_eq!(t.coach_lines.len(), 1);
        assert_eq!(t.coach_lines[0].said, "Refunded $20 to order 42.");
        assert!(t.drives.is_empty());
        // In order: each waiting card after what Coach wrote with it, up to its answer.
        let order: Vec<String> = t.items().iter().map(Item::id).collect();
        assert_eq!(order.len(), 7);
        assert!(order[0].starts_with("m|sess1|owner|run2.in"));
        assert!(order[1].starts_with("m|sess1|") && order[1].contains("|taf|"));
        assert_eq!(order[2], "c|sess1#limits");
        assert_eq!(order[3], "m|sess1|owner|run2.5");
        assert_eq!(order[4], "c|sess1#connect-stripe");
        assert_eq!(order[5], "c|sess1#done");
        assert!(order[6].contains("|taf|"));
    }

    #[test]
    fn the_contracts_chats_read_and_the_newest_opens() {
        let body = serde_json::to_vec(&contract("coach-chats.json")).unwrap();
        let chats = chats(&body).unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].title, "Set up refunds for my shop");
        assert_eq!(newest(&chats).unwrap().session, "sess1");
        // One chat it can't read fails the list.
        assert!(chats_of(json!({"chats": [{"session": "s"}]})).is_none());
        let two = chats_of(json!({"chats": [
            {"session": "a", "started": "2026-10-16T12:00:00Z", "updated": "2026-10-17T12:00:00Z", "title": "A"},
            {"session": "b", "started": "2026-10-16T12:00:00Z", "updated": "2026-10-16T12:00:00Z", "title": "B"}
        ]}))
        .unwrap();
        assert_eq!(newest(&two).unwrap().session, "a");
    }

    fn chats_of(v: Value) -> Option<Vec<Chat>> {
        chats(&serde_json::to_vec(&v).unwrap())
    }

    #[test]
    fn coach_is_the_setup_reads_taf() {
        let body = serde_json::to_vec(&contract("setup-coach.json")).unwrap();
        assert_eq!(
            setup_id(&body).unwrap().as_deref(),
            Some("taf-c0ac4000000000000000")
        );
        assert_eq!(setup_id(br#"{"setup": null}"#), Ok(None));
        assert!(setup_id(br#"{"setup": {"id": "x"}}"#).is_err());
    }

    #[test]
    fn the_path_asks_for_steps_only_with_a_proof() {
        assert_eq!(
            conversation_path("taf-1", None, false, &[]),
            "/v1/tafs/taf-1/conversation"
        );
        assert_eq!(
            conversation_path("taf-1", Some("sess 1"), true, &[]),
            "/v1/tafs/taf-1/conversation/sess%201"
        );
        assert_eq!(
            conversation_path("taf-1", Some("s"), false, &["share"]),
            "/v1/tafs/taf-1/conversation/s"
        );
        assert_eq!(
            conversation_path("taf-1", Some("s"), true, &["share", "trial"]),
            "/v1/tafs/taf-1/conversation/s?can=share,trial"
        );
    }

    /// The contract's conversation with its approval carrying `wider` as given.
    fn with_wider(wider: Value) -> Value {
        let mut v = contract("coach-conversation.json");
        v["components"][1]["component"]["wider"] = wider;
        v
    }

    #[test]
    fn a_wider_mark_reaches_its_words_and_any_other_shape_is_no_mark() {
        let words = "The sign-in for Google Sheets can't be narrowed to this job: it opens all your sheets.";
        let t = thread(&with_wider(json!({"words": words}))).unwrap();
        assert_eq!(t.components[1].component.wider_words(), Some(words));
        for other in [
            json!("wider"),
            json!(3),
            json!([words]),
            json!({"words": 7}),
            json!({"words": "  \n "}),
            json!({}),
            Value::Null,
        ] {
            let t = thread(&with_wider(other.clone())).unwrap();
            assert_eq!(t.components.len(), 3, "{other}: the card stays");
            assert_eq!(t.components[1].component.wider_words(), None, "{other}");
        }
        // The app's own fixture carries it too.
        let path = format!(
            "{}/../tafs-app/fixtures/coach-wider.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let file: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        let t = thread(&file["answer"]).unwrap();
        assert!(t.components.iter().any(|c| c
            .component
            .wider_words()
            .is_some_and(|w| w.starts_with("The sign-in for Google Sheets"))));
    }

    #[test]
    fn signs_are_read_leniently_but_must_be_a_mark() {
        let mut v = contract("coach-conversation.json");
        v["components"][1]["component"]["signs"] =
            json!({"card": "no", "words": "  ", "takes": "tap"});
        let t = thread(&v).unwrap();
        let s = t.components[1].component.signs.as_ref().unwrap();
        assert_eq!((s.card, s.words.as_deref(), s.tap()), (None, None, true));
        // Not an object: the card can't be read, and is left out.
        v["components"][1]["component"]["signs"] = json!("tap");
        assert_eq!(thread(&v).unwrap().components.len(), 2);
    }

    #[test]
    fn a_card_the_app_cant_read_is_left_out_never_the_thread() {
        let mut v = contract("coach-conversation.json");
        v["components"][0]["component"]["type"] = json!("hologram");
        v["components"][1]["component"]["connection"] = json!({"done": "yes"});
        v["components"][2]["answer"] = json!({"state": 5});
        v["coach_lines"] = json!([{"at": "never", "taf": "Rae", "said": "x"}, {"at": "2026-10-16T12:01:10Z", "taf": "Rae", "said": "y"}]);
        v["items"] = json!([{"type": "drive", "drive": {"id": "d1", "task": "Look", "at": "2026-10-16T12:02:00Z"}}, {"type": "drive", "drive": {}}, 4]);
        v["steering"] = json!({"run": "r"});
        let t = thread(&v).unwrap();
        assert_eq!(t.components.len(), 1);
        assert_eq!(t.components[0].component.id, "done");
        // An answer it can't read leaves the card unanswered.
        assert_eq!(t.components[0].answer, None);
        assert_eq!(t.coach_lines.len(), 1);
        assert_eq!(t.drives.len(), 1);
        assert_eq!(t.drives[0].task, "Look");
        assert_eq!(t.steering, None);
        // A message it can't read fails the thread, as do a thread with no messages and
        // components or working of another shape.
        let mut bad = contract("coach-conversation.json");
        bad["messages"][0]["from"] = json!("someone");
        assert!(thread(&bad).is_none());
        let mut bad = contract("coach-conversation.json");
        bad.as_object_mut().unwrap().remove("messages");
        assert!(thread(&bad).is_none());
        let mut bad = contract("coach-conversation.json");
        bad["components"] = json!({});
        assert!(thread(&bad).is_none());
        let mut bad = contract("coach-conversation.json");
        bad["working"] = json!({"run": "r"});
        assert!(thread(&bad).is_none());
        // A mark on a message it can't read leaves the message plain.
        let mut ok = contract("coach-conversation.json");
        ok["messages"][1]["work"] = json!({"ms": "long"});
        ok["messages"][1]["kind"] = json!("aside");
        ok["messages"][1]["refused"] = json!({"words": " ", "next": ""});
        let t = thread(&ok).unwrap();
        assert_eq!(t.messages[1].work, None);
        assert_eq!(t.messages[1].kind, MessageKind::Message);
        assert_eq!(t.messages[1].refused, None);
    }

    #[test]
    fn a_card_shown_again_folds_into_its_first_place() {
        let mut v = contract("coach-conversation.json");
        let mut again = v["components"][1].clone();
        again["at"] = json!("2026-10-16T12:00:45.000Z");
        again["component"]["title"] = json!("Connect Stripe again");
        again.as_object_mut().unwrap().remove("answer");
        v["components"].as_array_mut().unwrap().push(again);
        let t = thread(&v).unwrap();
        assert_eq!(t.components.len(), 3);
        let c = &t.components[1];
        assert_eq!(c.component.title.as_deref(), Some("Connect Stripe again"));
        assert_eq!(c.first_at.to_rfc3339(), "2026-10-16T12:00:25+00:00");
        // tafs cloud puts an answer on the latest card: one shown again waits anew.
        assert_eq!(c.answer, None);
    }

    #[test]
    fn answers_read_as_the_cards_read_them() {
        let a = Answer::parse("[approval connect-stripe] failed: the vendor said no").unwrap();
        assert_eq!(
            (a.kind.as_str(), a.id.as_str()),
            ("approval", "connect-stripe")
        );
        assert!(a.failed());
        assert_eq!(a.why().as_deref(), Some("the vendor said no"));
        assert_eq!(Answer::parse("[hologram x] y"), None);
        assert_eq!(Answer::parse("[approval] y"), None);
        assert_eq!(Answer::parse("plain words"), None);
        let c = Component::read(&json!({"type": "approval", "id": "a"})).unwrap();
        let recorded = |words: &str| CardAnswer {
            state: "answered".into(),
            words: words.into(),
            by: None,
            outcome: None,
            at: None,
        };
        assert_eq!(
            SetupThread::recorded(&recorded("Approved · it didn't go through: no"), &c)
                .unwrap()
                .text,
            "failed: no"
        );
        assert_eq!(
            SetupThread::recorded(&recorded("Declined"), &c)
                .unwrap()
                .text,
            "declined"
        );
    }

    #[test]
    fn vendors_as_people_say_them() {
        assert_eq!(vendor_words("mcp.stripe.com"), "Stripe");
        assert_eq!(vendor_words("google-sheets-mcp"), "Google sheets");
        assert_eq!(vendor_words(""), "");
    }
}
