//! A taf's chat (`TafChatView`, `AppModel`'s Chat and "A taf's chat, live", `TafsCloud.Thread`):
//! the thread as tafs cloud reads it (`GET /v1/tafs/{id}/conversation`, decoded exactly as strictly
//! or as leniently as the iPad's `thread(_:)`), what the chat holds while a message goes
//! (`ChatState`, `deliverChat`, `submitChat`, `stopChat`, `retryChat`, `settleSteering`), the
//! thread in one order (`chatItems`, `heldAfterItsWords`, `withheldUntilWords`), and what the chat
//! draws, every word tafs cloud's, the card's or the app's at the pin. The sends are Coach's own
//! (`coach_send`): the same routes, bodies and answers, into the taf's live chat. Nothing here
//! sends, and nothing is signed: Resume, after a send refused for a halt, is the phone's.

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use crate::approval::{self, Gate, NotReady};
use crate::coach::{
    array_if, date, field, object, text, text_if, ChatMessage, From, MessageKind, Read, Refused,
    Steering, SteeringMessage, PENDING,
};
use crate::coach_lines::{gap, WorkLines};
use crate::coach_send::{
    already_recorded, merge, stamp, steered_into, ChatTarget, Delivery, Ended, Loaded, Refusal,
    Sent, Steered, Submit, Then, Unsent, CHAT_LIMIT,
};
use crate::sidebar::Mode;
use crate::stream::{DoneCall, Event, LiveAnswer, LiveState, ReplyPart, StepStatus};
use crate::words::{WordBook, WordsError};

/// `GET /v1/tafs/{id}/conversation` (`conversation`): the taf's last runs and the person's chats
/// with it, read without a session: the live chat.
pub fn conversation_path(taf: &str) -> String {
    crate::coach::conversation_path(taf, None, false, &[])
}

/// Where the latest run stopped without finishing (`Capped`): `cost_cap`, `turn_cap`, `max_token`
/// or `refused`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capped {
    pub run: String,
    pub stop: String,
}

/// A taf's card handing a setting to Coach (`AskCoachCard`): the owner's request in the taf's
/// words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskCoachCard {
    pub id: String,
    pub at: DateTime<Utc>,
    pub session: String,
    pub text: String,
}

impl AskCoachCard {
    /// `AskCoachCard.Lenient`: an `ask_coach` card with words, else none; one the app can't read
    /// is none too, never the thread with it.
    fn read(v: &Value) -> Option<AskCoachCard> {
        let shown = object(v).ok()?;
        let at = date(shown, "at").ok()?;
        let session = text_if(shown, "session").ok()?;
        let c = object(field(shown, "component")?).ok()?;
        let kind = text(c, "type").ok()?;
        let id = text(c, "id").ok()?;
        let words = text_if(c, "text").ok()?;
        if kind != "ask_coach" {
            return None;
        }
        let words = words.filter(|t| !t.trim().is_empty())?;
        Some(AskCoachCard {
            id,
            at,
            session: session.unwrap_or_default(),
            text: words,
        })
    }
}

/// A taf's thread, oldest first (`TafsCloud.Thread`), and the session it's at work in now.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TafThread {
    pub messages: Vec<ChatMessage>,
    pub working: Option<String>,
    /// The run at work in `working`.
    pub working_run: Option<String>,
    /// What the owner wrote to the latest run that it hasn't read.
    pub steering: Option<Steering>,
    /// The latest run stopped without finishing, with no words for it.
    pub capped: Option<Capped>,
    /// The taf's `ask_coach` cards: the only cards of a taf's own thread the iPad draws.
    pub ask_coach: Vec<AskCoachCard>,
}

impl TafThread {
    /// `thread(_:)`: the messages strictly, `working` strictly when there, the steering and the
    /// stop leniently (one the app can't read is none), each card leniently, but `components`
    /// itself an array.
    pub fn read(body: &[u8]) -> Option<TafThread> {
        let v: Value = serde_json::from_slice(body).ok()?;
        Self::read_value(&v).ok()
    }

    fn read_value(v: &Value) -> Read<TafThread> {
        let r = object(v)?;
        let messages = crate::coach::array(r, "messages", ChatMessage::read)?;
        let working = match field(r, "working") {
            None => None,
            Some(w) => {
                let w = object(w)?;
                Some((text(w, "session")?, text_if(w, "run")?))
            }
        };
        let cap = |key: &str| -> Option<Capped> {
            let c = object(field(r, key)?).ok()?;
            Some(Capped {
                run: text(c, "run").ok()?,
                stop: text(c, "stop").ok()?,
            })
        };
        let ask_coach = array_if(r, "components", |c| Ok(AskCoachCard::read(c)))?
            .unwrap_or_default()
            .into_iter()
            .flatten()
            .collect();
        Ok(TafThread {
            messages,
            working_run: working.as_ref().and_then(|w| w.1.clone()),
            working: working.map(|w| w.0),
            steering: field(r, "steering").and_then(|s| Steering::read(s).ok()),
            // Any stop tafs cloud names; `capped` carries the two caps alone, for older apps.
            capped: cap("stopped").or_else(|| cap("capped")),
            ask_coach,
        })
    }
}

/// The stops of the run's own that end it with no words (`capStops`).
pub const CAP_STOPS: [&str; 4] = ["cost_cap", "turn_cap", "max_token", "refused"];

/// How often the thread is read again while the taf owes an answer (`chatPollInterval`), and how
/// many times for one unanswered message (`chatPollBudget`: two minutes).
pub const POLL: std::time::Duration = std::time::Duration::from_secs(4);
pub const POLL_BUDGET: u32 = 30;

/// How long a held call's card waits in the chat for the taf's words that explain it
/// (`heldWaitsForWords`).
pub const HELD_WAITS_FOR_WORDS: i64 = 45;

/// A taf's chat as the app holds it (`chats[tafID]`, `ChatState`): what tafs cloud read, what was
/// sent from here that it doesn't have yet, and how the last run ended.
#[derive(Debug, Clone, Default)]
pub struct TafChat {
    /// The taf at tafs cloud.
    pub taf: Option<String>,
    pub thread: TafThread,
    /// Read at least once.
    pub loaded: bool,
    sent: Vec<ChatMessage>,
    /// When the owner last wrote, until the taf answers: tafs cloud may not see the run yet.
    pub awaiting: Option<DateTime<Utc>>,
    /// A message is on its way (`chatSending`): one at a time.
    pub sending: bool,
    pub unsent: Vec<Unsent>,
    next_unsent: u64,
    /// A Retry's id, for the send it starts.
    reuse_id: Option<String>,
    /// The unread messages of runs dealt with here, by hash (in memory: this computer keeps no
    /// list across launches).
    settled: HashSet<String>,
    pub ended: Option<Ended>,
    /// The session the owner stopped: over here at once, whatever tafs cloud still says.
    pub stopped: Option<String>,
    /// The latest run stopped at a cap of its own (`capped`).
    pub capped: Option<String>,
    /// What's said above the box (`chatErrors`).
    pub error: Option<String>,
    /// The taf's answer as it streams (`ChatState.live`).
    pub live: Option<LiveAnswer>,
    /// What went through before a run that failed part-way stopped (`wentThrough`).
    pub went_through: Vec<DoneCall>,
    /// tafs cloud's words for why that run stopped (`failedWhy`).
    pub failed_why: Option<String>,
    /// The runs picked back up here, each once (`resumedRuns`).
    resumed: HashSet<String>,
    /// Each held call's card key by KIFF's decision id (`heldKeys`), once KIFF was read: a call
    /// held in a streamed answer stays placed after it's answered and no longer held.
    pub held_keys: HashMap<String, String>,
    /// Where each held call was made (`anchors`, kiff/tafs#206): the item it came after, by its
    /// card's key. Its card stays there.
    pub anchors: HashMap<String, String>,
    /// This taf's Record lines as KIFF's activity was last read (`entries(for:)`): what became
    /// of its calls, and what each moved.
    pub record: Vec<crate::record::Entry>,
    /// Its doors' open knocks as last read (`chatKnocks`): someone wrote through a door, and the
    /// owner answers it here. None where tafs cloud offers no doors.
    pub knocks: Vec<crate::doors::Knock>,
}

/// A taf's stream followed (`followChatStream`'s own): how the run ended, once it says, what
/// went through before, and tafs cloud's words for why.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Following {
    pub ended: Option<String>,
    pub done: Vec<DoneCall>,
    pub why: Option<String>,
}

/// A steered message's key: its run, time and words, hashed, never kept as words.
fn steering_key(s: &Steering, m: &SteeringMessage) -> String {
    use sha2::{Digest, Sha256};
    let at = m.at.timestamp_millis() as f64 / 1000.0;
    let digest = Sha256::digest(format!("{}|{at}|{}", s.run, m.text).as_bytes());
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

impl TafChat {
    pub fn new(taf: &str) -> TafChat {
        TafChat {
            taf: Some(taf.into()),
            ..TafChat::default()
        }
    }

    /// The taf is at work on its chat (`chatThinking`): tafs cloud says so, unless the owner
    /// stopped that session, or the owner just wrote.
    pub fn thinking(&self) -> bool {
        let working = self
            .thread
            .working
            .as_ref()
            .filter(|w| self.stopped.as_ref() != Some(*w));
        working.is_some() || self.awaiting.is_some()
    }

    /// The taf hasn't answered the owner's last message yet (`awaitingReply`).
    pub fn awaiting_reply(&self) -> bool {
        self.thread
            .messages
            .last()
            .is_some_and(|m| m.from == From::Owner)
    }

    /// tafs cloud's thread read (`loadChat`): with what was sent from here that it doesn't have
    /// yet, the stop and the cap as it says them, the wait settled, and what a run never read
    /// shown as not sent.
    pub fn took(&mut self, w: &WordBook, thread: TafThread, now: DateTime<Utc>) {
        let (messages, waiting) = merge(thread.messages.clone(), &self.sent, now);
        self.sent = waiting;
        // A stopped session that tafs cloud no longer has at work is done with.
        if self.stopped.is_some() && thread.working != self.stopped {
            self.stopped = None;
        }
        // A run at work, or one that answered since, is past its cap.
        self.capped = if thread.working.is_none() {
            thread.capped.as_ref().map(|c| c.stop.clone())
        } else {
            None
        };
        self.thread = TafThread { messages, ..thread };
        self.loaded = true;
        self.error = None;
        self.settle_awaiting(now);
        self.settle_steering(w, now);
    }

    /// `settleChatAwaiting`: the wait ends once tafs cloud says the taf is at work, once it
    /// answered after the owner wrote, or after two minutes with no sign.
    pub fn settle_awaiting(&mut self, now: DateTime<Utc>) {
        let Some(since) = self.awaiting else { return };
        let answered = self
            .thread
            .messages
            .iter()
            .any(|m| m.from == From::Taf && m.at >= since);
        if self.thread.working.is_some() || answered || now - since > Duration::seconds(120) {
            self.awaiting = None;
        }
    }

    /// `settleSteering`: tafs cloud's `not_sent`, in its words, and what a run that's over never
    /// read, each shown once as not sent. tafs cloud sends the rest itself.
    fn settle_steering(&mut self, w: &WordBook, now: DateTime<Utc>) {
        let Some(s) = self.thread.steering.clone() else {
            return;
        };
        let mut fresh: Vec<(SteeringMessage, String)> = s.not_sent.clone();
        if s.over() {
            let why = w
                .line(if s.status == "CANCELLED" {
                    "chat.steer.stopped_before_read"
                } else {
                    "chat.steer.not_read_before_end"
                })
                .unwrap_or_default();
            fresh.extend(s.untaken.iter().map(|m| (m.clone(), why.clone())));
        }
        for (m, why) in fresh {
            let key = steering_key(&s, &m);
            if self.settled.insert(key.clone()) {
                self.keep_unsent(&m.text, &why, None, Some(key), false, now);
            }
        }
    }

    fn keep_unsent(
        &mut self,
        text: &str,
        why: &str,
        halted: Option<String>,
        settle: Option<String>,
        busy: bool,
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
            busy,
            client_id: None,
        });
    }

    /// A message went through: one kept only because the owner wrote too fast is over too
    /// (`clearBusyUnsent`).
    fn clear_busy(&mut self) {
        self.unsent.retain(|u| !u.busy);
    }

    /// The composer's Send (`submitChat`): a message now, or, while the taf works, to its run.
    /// A message too long stays in the box, with why above it.
    pub fn submit(
        &mut self,
        w: &WordBook,
        text: &str,
        now: DateTime<Utc>,
    ) -> Result<Submit, WordsError> {
        let text = text.trim();
        if text.is_empty() || self.taf.is_none() {
            return Ok(Submit::Stay);
        }
        if self.thinking() || self.sending {
            // The message that starts it is still on its way: no run yet, so it stays in the box.
            return Ok(match self.thread.working_run.clone() {
                Some(run) => Submit::Steer {
                    run,
                    text: text.into(),
                    client_id: self.message_id(),
                },
                None => Submit::Stay,
            });
        }
        if text.chars().count() > CHAT_LIMIT {
            self.error = Some(w.count("chat.too_long", CHAT_LIMIT as i64, &[])?);
            return Ok(Submit::Stay);
        }
        Ok(match self.deliver(text, now) {
            Some(d) => Submit::Send(d),
            None => Submit::Stay,
        })
    }

    /// `deliverChatOnce`'s start: the message shown at once, faded, until tafs cloud's copy takes
    /// its place; it goes into the taf's live chat. None when it isn't tried.
    pub fn deliver(&mut self, text: &str, now: DateTime<Utc>) -> Option<Delivery> {
        self.deliver_with(text, vec![], now)
    }

    /// `submitChat` with files: a message of their own, the words with them or none.
    pub fn deliver_with(
        &mut self,
        text: &str,
        files: Vec<crate::coach::Attachment>,
        now: DateTime<Utc>,
    ) -> Option<Delivery> {
        let text = text.trim();
        if (text.is_empty() && files.is_empty())
            || text.chars().count() > CHAT_LIMIT
            || self.taf.is_none()
        {
            return None;
        }
        if self.sending {
            return None;
        }
        self.sending = true;
        self.error = None;
        self.ended = None;
        self.capped = None;
        self.went_through.clear();
        self.failed_why = None;
        let latest = self.thread.messages.iter().map(|m| m.at).max();
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
            target: ChatTarget::default(),
            generation: 0,
            files,
            client_id: self.message_id(),
        })
    }

    fn drop_local(&mut self, local: &str) {
        self.sent.retain(|m| m.id() != local);
        self.thread.messages.retain(|m| m.id() != local);
    }

    /// Sent: the copy takes tafs cloud's id (`markSent`), and the taf is awaited until it answers.
    pub fn delivered(&mut self, d: &Delivery, sent: &Sent, now: DateTime<Utc>) {
        self.sending = false;
        if let Some(id) = &sent.message {
            for m in self.sent.iter_mut().chain(self.thread.messages.iter_mut()) {
                if m.id() == d.local && m.server_id.is_none() {
                    m.server_id = Some(id.clone());
                }
            }
        }
        self.awaiting = Some(now);
        self.clear_busy();
    }

    /// The stream opened for the message (`deliverChatOnce`, streamed): the taf is at work, and
    /// its answer shows as it's written.
    pub fn streaming(&mut self, now: DateTime<Utc>) {
        self.sending = false;
        self.awaiting = Some(now);
        self.live = Some(LiveAnswer::new(now.timestamp()));
        self.clear_busy();
    }

    /// One of the stream's events (`followChatStream`), for the message on its way `sent`: what
    /// the app does next. The taf's words up to each tool call stay as they were written, in
    /// the order they came; the thread is read only when it holds something new (a message the
    /// owner wrote meanwhile, taken; a run picked back up, saying something).
    pub fn stream_event(
        &mut self,
        w: &WordBook,
        e: &Event,
        sent: Option<&str>,
        f: &mut Following,
    ) -> Result<Then, WordsError> {
        let Some(live) = self.live.as_mut() else {
            return Ok(Then::Draw);
        };
        Ok(match e {
            Event::Session {
                session,
                run,
                message,
                ..
            } => {
                live.session = Some(session.clone());
                live.run = run.clone();
                if let (Some(sent), Some(id)) = (sent, message) {
                    for m in self.sent.iter_mut().chain(self.thread.messages.iter_mut()) {
                        if m.id() == sent && m.server_id.is_none() {
                            m.server_id = Some(id.clone());
                        }
                    }
                }
                Then::Draw
            }
            Event::Taken | Event::Said => Then::Load(Loaded::Keep),
            Event::Text(t) => {
                live.text.push_str(t);
                // Writing again: the tool call it waited on is over.
                live.activity = None;
                live.thinking.clear();
                Then::Draw
            }
            Event::Thinking(t) => {
                live.thinking.push_str(t);
                Then::Draw
            }
            Event::State { state, words } => {
                live.state = Some(LiveState {
                    state: state.clone(),
                    words: words.clone(),
                });
                Then::Draw
            }
            Event::Activity { tool, step, call } => {
                // The words so far are done; the answer goes on below them.
                if !live.text.is_empty() {
                    let t = std::mem::take(&mut live.text);
                    live.parts.push(ReplyPart::Text(t));
                }
                let words = match step {
                    Some(s) => s.clone(),
                    None => crate::coach_lines::taf_activity_words(w, tool)?,
                };
                live.activity = Some(words.clone());
                live.began(tool, Some(&words), call.as_deref());
                Then::Draw
            }
            Event::Step { call, status } => {
                live.ended(Some(call), StepStatus::from_stream(status));
                Then::Draw
            }
            // It went on after the owner answered a held call: what it did before shows with
            // the answer it wrote then.
            Event::Part => {
                live.new_part();
                Then::Draw
            }
            Event::Held {
                exception,
                invocation,
            } => {
                live.ended(None, StepStatus::Held);
                live.parts.push(ReplyPart::Held {
                    exception: exception.clone(),
                    invocation: invocation.clone(),
                });
                Then::Draw
            }
            Event::End {
                status,
                stop,
                done,
                words,
                ..
            } => {
                f.ended = Some(status.clone());
                f.done = done.clone();
                f.why = words.clone();
                live.activity = None;
                if CAP_STOPS.contains(&stop.as_str()) {
                    self.capped = Some(stop.clone());
                }
                Then::Draw
            }
            Event::Component | Event::Broken => Then::Draw,
        })
    }

    /// `resumeChat`'s check: the run tafs cloud says is at work while nothing here follows it (the
    /// owner left the chat, or the app, and came back), with its chat; once per run. None while a
    /// message is on its way, an answer streams, or the owner stopped that chat.
    pub fn to_resume(&mut self) -> Option<(String, String)> {
        if self.sending || self.live.is_some() || self.taf.is_none() {
            return None;
        }
        let run = self.thread.working_run.clone()?;
        let session = self.thread.working.clone()?;
        if self.stopped.as_ref() == Some(&session) || !self.resumed.insert(run.clone()) {
            return None;
        }
        Some((run, session))
    }

    /// A run picked back up: its answer streams from what it did so far, under the owner's last
    /// message, which is what Retry sends again should it fail.
    pub fn resuming(&mut self, run: &str, session: &str, now: DateTime<Utc>) -> String {
        let asked = self
            .thread
            .messages
            .iter()
            .rev()
            .find(|m| m.from == From::Owner);
        let mut live = LiveAnswer::new(asked.map_or(now, |m| m.at).timestamp());
        live.run = Some(run.into());
        live.session = Some(session.into());
        self.live = Some(live);
        asked.map(|m| m.text.clone()).unwrap_or_default()
    }

    /// The stream is over, ended or dropped: nothing is at work once the run ended, and how it
    /// ended shows when the owner should know. `text` is what started it, sent again by Retry.
    /// The live answer goes once the thread is read (`live_gone`).
    pub fn stream_over(&mut self, f: &Following, text: &str) {
        if let Some(live) = self.live.as_mut() {
            live.activity = None;
        }
        if f.ended.is_some() {
            self.awaiting = None;
        }
        match f.ended.as_deref() {
            Some("ERROR") => {
                self.ended = Some(Ended::Failed {
                    retry: text.to_string(),
                });
                self.went_through = f.done.clone();
                self.failed_why = f.why.clone();
            }
            Some("CANCELLED") => self.ended = Some(Ended::Stopped),
            _ => {}
        }
    }

    /// KIFF's held calls, read: each one's card key by its decision id (`heldKeys`).
    pub fn know_held(&mut self, calls: impl IntoIterator<Item = (String, String)>) {
        self.held_keys.extend(calls);
    }

    /// `anchorHeld`: once a streamed answer is recorded, where each of its held calls was made,
    /// after the recorded words that came before it, or after the owner's message when it came
    /// first. The answer's words are the session's taf messages after the owner's last, in order.
    pub fn anchor_held(&mut self) {
        let Some(live) = &self.live else {
            return;
        };
        if !live
            .parts
            .iter()
            .any(|p| matches!(p, ReplyPart::Held { .. }))
        {
            return;
        }
        let messages: Vec<&ChatMessage> = self
            .thread
            .messages
            .iter()
            .filter(|m| live.session.as_deref() == Some(m.session.as_str()))
            .collect();
        let last_owner = messages.iter().rposition(|m| m.from == From::Owner);
        let answer: Vec<&&ChatMessage> = messages[last_owner.map_or(0, |i| i + 1)..]
            .iter()
            .filter(|m| m.from == From::Taf)
            .collect();
        let mut before = last_owner.map(|i| format!("m|{}", messages[i].id()));
        let mut said = 0;
        for part in &live.parts {
            match part {
                ReplyPart::Text(_) => {
                    if let Some(m) = answer.get(said) {
                        before = Some(format!("m|{}", m.id()));
                    }
                    said += 1;
                }
                ReplyPart::Held { exception, .. } => {
                    if let (Some(key), Some(before)) = (self.held_keys.get(exception), &before) {
                        self.anchors.insert(key.clone(), before.clone());
                    }
                }
            }
        }
    }

    /// The stream's answer is the thread's now: it goes.
    pub fn live_gone(&mut self) {
        self.live = None;
    }

    /// `retryChatRun`: the message the run that failed part-way started with, to send again.
    pub fn retry_run(&mut self) -> Option<String> {
        match self.ended.take() {
            Some(Ended::Failed { retry }) => Some(retry),
            other => {
                self.ended = other;
                None
            }
        }
    }

    /// Refused for want of the consent, or because the tafs wait for credits (`why`): the message
    /// leaves the thread, with no Not sent row, and goes back in the box.
    pub fn not_tried(&mut self, d: &Delivery, why: Option<String>) {
        self.sending = false;
        self.drop_local(&d.local);
        if why.is_some() {
            self.error = why;
        }
    }

    /// It didn't go: kept in the thread as not sent, with why; `busy` when the owner wrote too
    /// many too fast.
    pub fn not_sent(
        &mut self,
        d: &Delivery,
        why: &str,
        halted: Option<String>,
        busy: bool,
        answered: Option<u16>,
        now: DateTime<Utc>,
    ) {
        self.sending = false;
        self.drop_local(&d.local);
        self.keep_unsent(&d.text, why, halted, None, busy, now);
        // Retry keeps the id only where tafs cloud may have taken it (`retry_keeps_id`).
        if crate::coach_send::retry_keeps_id(answered) {
            self.unsent_id(&d.client_id);
        }
    }

    /// The id the next message goes under: a Retry's, else a new one.
    fn message_id(&mut self) -> String {
        self.reuse_id
            .take()
            .unwrap_or_else(crate::coach_send::new_message_id)
    }

    /// The row just kept as not sent keeps its message's id, for Retry.
    fn unsent_id(&mut self, id: &str) {
        if let Some(u) = self.unsent.last_mut() {
            u.client_id = Some(id.into());
        }
    }

    /// Written to the run while the taf works (`steer`, with `hold`): shown at once, as the run
    /// now holds it. Sent at once as the next message: the taf is awaited.
    pub fn steered(&mut self, run: &str, text: &str, steered: &Steered, now: DateTime<Utc>) {
        let current = self.thread.steering.take();
        self.thread.steering = Some(steered_into(current, run, text, steered, now));
        if matches!(steered, Steered::Kept { sent: Some(_), .. }) {
            self.awaiting = Some(now);
        }
        self.clear_busy();
    }

    /// A steer that didn't reach tafs cloud stays in the thread as not sent. The app never
    /// resends it on its own. `sent`: the steer's id, and tafs cloud's status, none when nothing
    /// answered (`retry_keeps_id`).
    pub fn steer_not_sent(
        &mut self,
        text: &str,
        sent: (&str, Option<u16>),
        why: &str,
        halted: Option<String>,
        busy: bool,
        now: DateTime<Utc>,
    ) {
        self.keep_unsent(text, why, halted, None, busy, now);
        let (id, answered) = sent;
        if crate::coach_send::retry_keeps_id(answered) {
            self.unsent_id(id);
        }
    }

    /// The chat Stop stops (`stopChat`): the one at work, else the thread's latest. The wait ends.
    pub fn stop(&mut self) -> Option<String> {
        self.taf.as_ref()?;
        // A streamed message's own chat first.
        let live = self.live.as_ref().and_then(|l| l.session.clone());
        let session = live.or_else(|| self.thread.working.clone()).or_else(|| {
            self.thread
                .messages
                .iter()
                .rev()
                .find(|m| m.session != PENDING)
                .map(|m| m.session.clone())
        })?;
        self.awaiting = None;
        Some(session)
    }

    /// Stopped at once, as the owner sees it: the run ends at its next step.
    pub fn stopped(&mut self, session: &str) {
        self.ended = Some(Ended::Stopped);
        self.stopped = Some(session.into());
    }

    /// Retry on a message that didn't go (`retryChat`): its words to submit again, taken out of
    /// its place until it's sent or kept again.
    pub fn retry(&mut self, id: u64) -> Option<(usize, Unsent)> {
        if self.sending {
            return None;
        }
        let at = self.unsent.iter().position(|u| u.id == id)?;
        let message = self.unsent.remove(at);
        // Sent again under its own id: tafs cloud acts on it once.
        self.reuse_id = message.client_id.clone();
        Some((at, message))
    }

    /// A retry that wasn't taken at all goes back in its place.
    pub fn retry_back(&mut self, at: usize, message: Unsent) {
        self.reuse_id = None;
        let at = at.min(self.unsent.len());
        self.unsent.insert(at, message);
    }

    /// A retry tried and kept as not sent again: still owed, so the new row carries its hash on.
    pub fn retry_kept(&mut self, left: usize, message: &Unsent) {
        if self.unsent.len() > left && message.settle.is_some() {
            if let Some(last) = self.unsent.last_mut() {
                last.settle = message.settle.clone();
            }
        }
    }

    /// Delete (`discardUnsentChat`): out of the thread, not sent.
    pub fn discard(&mut self, id: u64) {
        self.unsent.retain(|u| u.id != id);
    }

    /// Edit (`editUnsentChat`): out of the thread, its words back in the box.
    pub fn edit(&mut self, id: u64) -> Option<String> {
        let message = self.unsent.iter().find(|u| u.id == id)?.clone();
        self.unsent.retain(|u| u.id != id);
        Some(message.text)
    }

    /// What the thread is read again for (`followChat`): while the taf owes an answer or works,
    /// with a message of the owner's to follow.
    pub fn follows(&self) -> bool {
        self.awaiting_reply() || self.thinking()
    }

    /// The owner's last message, by id: each one is followed for at most `POLL_BUDGET` reads.
    pub fn last_owner(&self) -> Option<String> {
        self.thread
            .messages
            .iter()
            .rev()
            .find(|m| m.from == From::Owner)
            .map(ChatMessage::id)
    }
}

/// `followChat`'s reads: every 4 s while the taf owes an answer or works, at most 30 for each
/// message of the owner's.
#[derive(Debug, Clone, Default)]
pub struct Polls {
    waiting_for: Option<String>,
    polls: u32,
}

impl Polls {
    /// Whether to read the thread again now.
    pub fn read_now(&mut self, c: &TafChat) -> bool {
        if !c.follows() {
            return false;
        }
        let Some(last) = c.last_owner() else {
            return false;
        };
        if self.waiting_for.as_ref() != Some(&last) {
            self.waiting_for = Some(last);
            self.polls = 0;
        }
        if self.polls >= POLL_BUDGET {
            return false;
        }
        self.polls += 1;
        true
    }
}

// MARK: The thread in one order.

/// One thing in a taf's chat, in the order it happened (`ChatItem`): a message, a call KIFF holds
/// for the owner (by its card's key, held at its time), or a setting the taf hands to Coach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item<'a> {
    Message(&'a ChatMessage),
    Held {
        key: &'a str,
        at: DateTime<Utc>,
    },
    AskCoach(&'a AskCoachCard),
    /// What became of a held call, or a call the owner approved, from the Record (`outcome`): the
    /// same row as its card, by its key.
    Outcome(&'a crate::record::Entry),
    /// Someone wrote through a door (`knock`), at its time among the messages: the owner answers
    /// it there.
    Knock(&'a crate::doors::Knock),
    /// The taf's words in the answer streaming now, up to a tool call or being written.
    Streaming {
        id: String,
        text: String,
        session: String,
        at: DateTime<Utc>,
    },
}

impl Item<'_> {
    pub fn id(&self) -> String {
        match self {
            Item::Message(m) => format!("m|{}", m.id()),
            Item::Held { key, .. } => format!("k|{key}"),
            Item::AskCoach(c) => format!("a|{}", c.id),
            Item::Outcome(e) => format!("k|{}", e.id),
            Item::Knock(k) => format!("d|{}", k.id),
            Item::Streaming { id, .. } => id.clone(),
        }
    }

    pub fn at(&self) -> DateTime<Utc> {
        match self {
            Item::Message(m) => m.at,
            Item::Held { at, .. } => *at,
            Item::AskCoach(c) => c.at,
            // When the owner answered it: after the taf's "waiting for your approval" and before
            // what it did next. Without the answer's time, where its card was: when it was made.
            Item::Outcome(e) => e
                .answer
                .as_ref()
                .and_then(|a| a.at)
                .or(e.made_at)
                .unwrap_or(e.at),
            Item::Knock(k) => k.at,
            Item::Streaming { at, .. } => *at,
        }
    }

    /// A held call's card, or its outcome while it has no answer's time (`isCard`): it goes after
    /// the taf's words that ask for it, and where it was made.
    fn card_key(&self) -> Option<&str> {
        match self {
            Item::Held { key, .. } => Some(key),
            Item::Outcome(e) if e.answer.as_ref().and_then(|a| a.at).is_none() => Some(&e.id),
            _ => None,
        }
    }

    /// The session of the taf's words: none for anything else (`tafWords`).
    fn taf_words(&self) -> Option<&str> {
        match self {
            Item::Message(m) if m.from == From::Taf && m.kind != MessageKind::Kickoff => {
                Some(&m.session)
            }
            Item::Streaming { session, .. } => Some(session),
            _ => None,
        }
    }
}

/// The Record lines a taf's chat shows (`chatItems`' outcomes): since its first message, what
/// became of every call approved or refused, and of a call the owner approved that wasn't done,
/// is on its way, or whose outcome is unknown. None before the chat's first message.
pub fn outcomes(
    record: &[crate::record::Entry],
    first: Option<DateTime<Utc>>,
) -> impl Iterator<Item = &crate::record::Entry> {
    use crate::record::Kind;
    record.iter().filter(move |e| {
        first.is_some_and(|first| e.at >= first)
            && (matches!(e.kind, Kind::Approved | Kind::Refused)
                || (e.owner_approved
                    && matches!(e.kind, Kind::NotDone | Kind::Sending | Kind::Unknown)))
    })
}

/// `chatItems`: the messages, the calls KIFF holds for the owner where they were held, and the
/// settings the taf handed to Coach, by time, each once. A held call's card comes after the taf's
/// words that ask for it, and waits for them a while.
pub fn items<'a>(
    thread: &'a TafThread,
    held: &'a [(String, DateTime<Utc>)],
    now: DateTime<Utc>,
) -> Vec<Item<'a>> {
    items_with(thread, held, None, now)
}

/// `chatItems` with the answer streaming now after the rest, in the order it came.
pub fn items_with<'a>(
    thread: &'a TafThread,
    held: &'a [(String, DateTime<Utc>)],
    live: Option<&LiveAnswer>,
    now: DateTime<Utc>,
) -> Vec<Item<'a>> {
    let mut all: Vec<Item> = thread.messages.iter().map(Item::Message).collect();
    all.extend(held.iter().map(|(key, at)| Item::Held { key, at: *at }));
    all.extend(thread.ask_coach.iter().map(Item::AskCoach));
    // A stable sort: by time, then the order they came in.
    all.sort_by_key(|i| i.at());
    if let Some(live) = live {
        all.extend(streaming_items(live, &thread.messages));
    }
    let mut seen = HashSet::new();
    all.retain(|i| seen.insert(i.id()));
    withheld_until_words(held_after_its_words(all), now)
}

/// `chatItems` as a chat holds them: as `items_with`, and the calls held in the answer streaming
/// now placed in it, where they were made, not by time; once it's recorded, each held call after
/// the item it came after (`placed`).
pub fn items_of<'a>(
    c: &'a TafChat,
    held: &'a [(String, DateTime<Utc>)],
    now: DateTime<Utc>,
) -> Vec<Item<'a>> {
    let key_of = |exception: &str| c.held_keys.get(exception).map(String::as_str);
    let streaming: HashSet<&str> = c
        .live
        .iter()
        .flat_map(|l| l.parts.iter())
        .filter_map(|p| match p {
            ReplyPart::Held { exception, .. } => key_of(exception),
            ReplyPart::Text(_) => None,
        })
        .collect();
    let mut all: Vec<Item> = c.thread.messages.iter().map(Item::Message).collect();
    all.extend(
        held.iter()
            .filter(|(key, _)| !streaming.contains(key.as_str()))
            .map(|(key, at)| Item::Held { key, at: *at }),
    );
    // A door's knocks sit at their own times, so a reply newer than a knock comes after it.
    all.extend(c.knocks.iter().map(Item::Knock));
    all.extend(c.thread.ask_coach.iter().map(Item::AskCoach));
    // What became of its calls, done or not (kiff/tafs#115); those of the answer streaming now
    // are placed in it.
    let first = c.thread.messages.first().map(|m| m.at);
    all.extend(
        outcomes(&c.record, first)
            .filter(|e| !streaming.contains(e.id.as_str()))
            .map(Item::Outcome),
    );
    all.sort_by_key(|i| i.at());
    let mut all = placed(all, &c.anchors);
    if let Some(live) = &c.live {
        let run = live.run.as_deref().unwrap_or("live");
        let session = live.session.clone().unwrap_or_else(|| "live".into());
        let at = DateTime::from_timestamp(live.started, 0).unwrap_or_default();
        let mut n = 0;
        let mut words = |t: &str, all: &mut Vec<Item<'a>>| {
            if !already_recorded(t, live.session.as_deref(), live.started, &c.thread.messages) {
                all.push(Item::Streaming {
                    id: format!("s|{run}|{n}"),
                    text: t.to_string(),
                    session: session.clone(),
                    at,
                });
            }
            n += 1;
        };
        for part in &live.parts {
            match part {
                ReplyPart::Text(t) => words(t, &mut all),
                // Where it was made in the answer: its card while KIFF holds it, then what became
                // of it, in the same place.
                ReplyPart::Held { exception, .. } => {
                    let Some(k) = key_of(exception) else {
                        continue;
                    };
                    let answered = c
                        .record
                        .iter()
                        .find(|e| e.id == k && e.kind != crate::record::Kind::Held);
                    if let Some(e) = answered {
                        all.push(Item::Outcome(e));
                    } else if let Some((key, at)) = held.iter().find(|(key, _)| key == k) {
                        all.push(Item::Held { key, at: *at });
                    }
                }
            }
        }
        if !live.text.is_empty() {
            words(&live.text, &mut all);
        }
    }
    let mut seen = HashSet::new();
    all.retain(|i| seen.insert(i.id()));
    withheld_until_words(held_after_its_words(all), now)
}

/// `placed`: each held call moved to just after the item it was made after, when that item is in
/// the chat; the rest keep their place by time.
pub fn placed<'a>(items: Vec<Item<'a>>, anchors: &HashMap<String, String>) -> Vec<Item<'a>> {
    // An answered call goes where it was answered, by time, not where it was held.
    let anchor = |i: &Item| i.card_key().and_then(|k| anchors.get(k).cloned());
    let present: HashSet<String> = items.iter().map(Item::id).collect();
    let (moving, staying): (Vec<_>, Vec<_>) = items
        .into_iter()
        .partition(|i| anchor(i).is_some_and(|a| present.contains(&a)));
    if moving.is_empty() {
        return staying;
    }
    let mut out = Vec::new();
    for item in staying {
        let mut after = item.id();
        out.push(item);
        // Each held call made after this item, in the order they were made.
        for m in &moving {
            if anchor(m).as_deref() == Some(after.as_str()) {
                after = m.id();
                out.push(m.clone());
            }
        }
    }
    out
}

/// `streamingItems`: the answer streaming now, its words up to each tool call and those being
/// written, each once: words the thread already holds show there. A call KIFF held in it shows
/// where Needs you's list places it.
fn streaming_items(live: &LiveAnswer, recorded: &[ChatMessage]) -> Vec<Item<'static>> {
    let run = live.run.as_deref().unwrap_or("live");
    let session = live.session.clone().unwrap_or_else(|| "live".into());
    let at = DateTime::from_timestamp(live.started, 0).unwrap_or_default();
    let texts = live
        .parts
        .iter()
        .filter_map(|p| match p {
            ReplyPart::Text(t) => Some(t.as_str()),
            ReplyPart::Held { .. } => None,
        })
        .chain(Some(live.text.as_str()).filter(|t| !t.is_empty()));
    texts
        .enumerate()
        .filter(|(_, t)| !already_recorded(t, live.session.as_deref(), live.started, recorded))
        .map(|(n, t)| Item::Streaming {
            id: format!("s|{run}|{n}"),
            text: t.to_string(),
            session: session.clone(),
            at,
        })
        .collect()
}

/// `heldAfterItsWords`: a held call's card moves past the words of the turn right after it, those
/// in that turn's session: the call is held before the taf writes "waiting for your approval".
pub fn held_after_its_words(mut out: Vec<Item<'_>>) -> Vec<Item<'_>> {
    let mut i = 0;
    while i < out.len() {
        if out[i].card_key().is_none() {
            i += 1;
            continue;
        }
        let session = out
            .get(i + 1)
            .and_then(|n| n.taf_words().map(str::to_string));
        let mut j = i;
        while j + 1 < out.len() {
            let next = out[j + 1].taf_words().map(str::to_string);
            if next.is_none() || next != session {
                break;
            }
            out.swap(j, j + 1);
            j += 1;
        }
        i = j + 1;
    }
    out
}

/// `withheldUntilWords`: a call held after the owner's last message waits for the taf's words,
/// at most `HELD_WAITS_FOR_WORDS` seconds after it was held; then it shows, words or not.
pub fn withheld_until_words(items: Vec<Item<'_>>, now: DateTime<Utc>) -> Vec<Item<'_>> {
    let Some(owner) = items
        .iter()
        .rposition(|i| matches!(i, Item::Message(m) if m.from == From::Owner))
    else {
        return items;
    };
    let said = items[owner + 1..].iter().any(|i| match i {
        Item::Message(m) => m.from == From::Taf && m.kind != MessageKind::Kickoff,
        Item::Streaming { .. } => true,
        _ => false,
    });
    if said {
        return items;
    }
    items
        .into_iter()
        .enumerate()
        .filter(|(i, item)| match item {
            Item::Held { at, .. } if *i > owner => {
                (now - *at).num_seconds() >= HELD_WAITS_FOR_WORDS
            }
            _ => true,
        })
        .map(|(_, item)| item)
        .collect()
}

// MARK: What the chat draws.

/// One line of a taf's chat.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A time between parts of the thread that are far apart.
    Gap(String),
    /// A run's first message, its job's instruction, as a small note.
    Kickoff(String),
    /// The owner's words on the right. `menu`: its menu sends it again, puts it in the box, or
    /// copies it. `stamp`: the door it came through. `pending`: on its way, faded.
    Owner {
        text: String,
        answered: Option<String>,
        stamp: Option<String>,
        menu: bool,
        pending: bool,
        /// The files sent with it (`MessageFiles`), each opened full size.
        files: Vec<crate::coach::Attachment>,
    },
    /// The taf's words full width, with tafs cloud's lines around them: where the run's result
    /// went above, what it counted under, and a connection refused, never folded.
    Taf {
        work: Option<WorkLines>,
        delivered: Option<String>,
        text: String,
        counted: Option<String>,
        refused: Option<Refused>,
        /// How it went back through its door, and whether that failed.
        stamp: Option<(String, bool)>,
        /// tafs cloud's id for it and its chat, when it can be reported (`reportable`).
        report: Option<crate::report::Reported>,
        /// Still being written: shown at an even pace, what it's in the middle of closed
        /// (`SteadyMarkdown`).
        streaming: bool,
    },
    /// A call KIFF holds for the owner: its card, by its key.
    Held(String),
    /// What became of a call, from the Record (`OutcomeLine`), where its card was or when it was
    /// answered.
    Outcome(crate::coach_lines::Outcome),
    /// Someone wrote through a door (`KnockCard`): its card, answered here.
    Knock(crate::doors::Knock),
    /// A setting the taf hands to Coach, with Ask Coach where it's offered.
    AskCoach { text: String, ask: Option<String> },
}

/// A message that didn't go through (`UnsentBubble`): the owner's words, dimmed, then why. A send
/// refused for a halt says which, in tafs cloud's words, and why this computer can't resume it in
/// Resume's place: resuming is signed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsentLines {
    pub id: u64,
    pub words: String,
    /// "Not sent · why", or "{taf} is halted" for an older tafs cloud's halt.
    pub note: String,
    /// Refused for a halt: the hand's mark.
    pub halted: bool,
    /// Under an older tafs cloud's halt: nothing reaches it until it's resumed.
    pub resume_note: Option<String>,
    /// Why this computer can't resume, where Resume would be.
    pub cant: Option<NotReady>,
    pub edit: Option<String>,
    pub retry: Option<String>,
    /// Its menu (`contextMenu`): Copy and Delete.
    pub copy: String,
    pub delete: String,
}

/// The message box's words (`composer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerLines {
    pub placeholder: String,
    pub send: String,
    pub stop: String,
}

/// A taf's chat.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatLines {
    /// The empty chat's words.
    pub empty: Option<String>,
    pub lines: Vec<Line>,
    pub unsent: Vec<UnsentLines>,
    /// The taf is at work, and nothing streams: the dots, and what a screen reader says.
    pub thinking: Option<String>,
    /// What the taf is doing as its answer streams (`LiveWork`), and the line it goes before:
    /// the answer's first words, else under the rest.
    pub live: Option<(crate::coach_lines::LiveWorkLines, Option<usize>)>,
    /// The last run failed part-way: what went through before, and Retry.
    pub failed: Option<(String, String)>,
    /// Written to its run, not read yet, and when it will be.
    pub steering: Vec<(String, String)>,
    /// The latest run stopped at a cap: why.
    pub capped: Option<String>,
    /// What tafs cloud holds for this chat, to go once the taf is done, and that it waits.
    pub queued: Option<(String, String)>,
    /// An owner's message's menu: Send again, Edit and send, Copy.
    pub menu: (String, String, String),
    /// The owner's messages among `lines`, by their line: what the pin can hold.
    pub pins: Vec<(usize, crate::pin::Candidate)>,
}

/// What a taf's chat is with: its name, and what this person may do there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Whose<'a> {
    pub name: Option<&'a str>,
    /// May use Coach and manages the taf: its Ask Coach cards offer Ask Coach.
    pub ask_coach: bool,
}

/// The composer's words: while the taf works, what's written goes to its run.
pub fn composer(
    w: &WordBook,
    name: Option<&str>,
    thinking: bool,
) -> Result<ComposerLines, WordsError> {
    let name = match name {
        Some(n) => n.to_string(),
        None => w.line("chat.your_taf")?,
    };
    Ok(ComposerLines {
        placeholder: if thinking {
            w.line("chat.write_next")?
        } else {
            w.get("chat.message_taf", &[("taf", &name)])?
        },
        send: if thinking {
            w.get("chat.send_while_works", &[("taf", &name)])?
        } else {
            w.line("common.send")?
        },
        stop: w.line("chat.stop")?,
    })
}

/// `OutcomeLine` in a taf's chat (`outcomeText`, `outcomeTone`, `outcomeMark`): what became of a
/// call, as the Record says it. Approved "by you" only when KIFF says the reader answered it
/// (`answer.you`); this computer answers none, so there is no line of its own.
pub fn outcome_line(
    w: &WordBook,
    e: &crate::record::Entry,
) -> Result<crate::coach_lines::Outcome, WordsError> {
    use crate::coach_lines::{Mark, Outcome, Tone};
    use crate::record::Kind;
    let title = [("title", e.title.as_str())];
    let text = match e.kind {
        // Why, as the Record says it: a vendor's refusal still counts (kiff/tafs#206 6.13).
        Kind::NotDone => w.get(
            "chat.outcome.not_done",
            &[("title", e.title.as_str()), ("detail", e.detail.as_str())],
        )?,
        Kind::Sending => w.get("chat.outcome.sending", &title)?,
        Kind::Unknown => w.get("chat.outcome.unknown", &title)?,
        Kind::Refused => w.get("chat.outcome.refused", &title)?,
        Kind::Approved if e.answer.as_ref().and_then(|a| a.you) == Some(true) => {
            w.get("chat.outcome.approved", &title)?
        }
        Kind::Approved => w.get("chat.outcome.approved_done", &title)?,
        // The Record's own title for anything else, as the iPad's default.
        _ => e.title.clone(),
    };
    Ok(Outcome {
        text,
        tone: match e.kind {
            Kind::Approved => Tone::Ok,
            Kind::NotDone => Tone::No,
            Kind::Sending | Kind::Unknown => Tone::Amber,
            _ => Tone::Dim,
        },
        mark: match e.kind {
            Kind::Approved => Mark::Check,
            Kind::Sending => Mark::Clock,
            Kind::Unknown => Mark::Question,
            _ => Mark::Cross,
        },
        more: false,
        hint: None,
    })
}

/// What each of a taf's calls moved, by KIFF's key for it (`recordEntry(forCall:)?.amountText`):
/// the first Record line for the key, as the iPad finds it.
pub fn call_amounts(
    w: &WordBook,
    record: &[crate::record::Entry],
) -> Result<HashMap<String, String>, WordsError> {
    let mut out = HashMap::new();
    for e in record {
        let Some(key) = &e.entity_id else { continue };
        if out.contains_key(key) || crate::record::entry_for_call(record, key) != Some(e) {
            continue;
        }
        if let Some(a) = e.amount_text(w)? {
            out.insert(key.clone(), a);
        }
    }
    Ok(out)
}

/// `failedWords`: a run that failed part-way, in tafs cloud's words when it gave some, then what
/// went through before it stopped: each call by its tool's name in words, what it acted on, and
/// what it moved, from the Record (`amounts`, by KIFF's key for the call).
pub fn failed_words(
    w: &WordBook,
    taf: &str,
    done: &[DoneCall],
    amounts: &HashMap<String, String>,
    why: Option<&str>,
) -> Result<String, WordsError> {
    let calls: Vec<String> = done
        .iter()
        .map(|c| {
            let words = crate::held::tool_words(&c.tool);
            let on = match &c.resource {
                Some(r) => format!("{words} on {r}"),
                None => words,
            };
            match c.invocation.as_ref().and_then(|k| amounts.get(k)) {
                Some(a) => format!("{on}, {a}"),
                None => on,
            }
        })
        .collect();
    let list = calls.join("; ");
    match why {
        Some(why) if calls.is_empty() => Ok(why.to_string()),
        Some(why) => w.get(
            "chat.failed.with_words",
            &[("words", why.trim()), ("calls", &list)],
        ),
        None if calls.is_empty() => w.get("work.stopped_part_way_before_finished", &[("taf", taf)]),
        None if calls.len() == 1 => w.get("chat.failed.after", &[("taf", taf), ("calls", &list)]),
        None => w.get(
            "chat.failed.after_several",
            &[("taf", taf), ("calls", &list)],
        ),
    }
}

/// `capWords`: what the chat says when its latest run stopped at a cap, never "…".
pub fn cap_words(w: &WordBook, stop: &str, taf: &str) -> Result<Option<String>, WordsError> {
    let key = match stop {
        "cost_cap" => "chat.cap.cost",
        "turn_cap" => "chat.cap.turns",
        "max_token" => "chat.stopped.out_of_room",
        "refused" => "chat.stopped.declined",
        _ => return Ok(None),
    };
    w.get(key, &[("taf", taf)]).map(Some)
}

/// `DoorStamp`: a door's message by its door and sender; its answer, how it went back.
pub fn door_stamp(
    w: &WordBook,
    m: &crate::coach::DoorMark,
    from_owner: bool,
) -> Result<(String, bool), WordsError> {
    let app = crate::doors::label(&m.app).to_uppercase();
    if from_owner {
        let by = w.get("door.stamp.by", &[("app", &app)])?;
        return Ok((
            match &m.from {
                Some(f) => format!("{by} · {f}"),
                None => by,
            },
            false,
        ));
    }
    Ok(match m.reply.as_deref() {
        Some("sent") => (w.get("door.stamp.sent_back", &[("app", &app)])?, false),
        Some("not_sent") => {
            let not = w.line("door.stamp.not_sent_back")?;
            (
                match &m.why {
                    Some(why) => format!("{not} · {why}"),
                    None => not,
                },
                true,
            )
        }
        _ => (w.get("door.stamp.goes_back", &[("app", &app)])?, false),
    })
}

/// `UnsentBubble.halted`: an older tafs cloud's refusal for a halt, by its words.
pub fn halted_words(why: &str) -> bool {
    let w = why.to_lowercase();
    w.starts_with("halted") || w.contains("is halted")
}

/// What a taf's chat draws, as it holds it now. `held`: its held calls' card keys, each with when
/// it was held.
pub fn lines(
    w: &WordBook,
    c: &TafChat,
    whose: &Whose,
    held: &[(String, DateTime<Utc>)],
    now: DateTime<Utc>,
) -> Result<ChatLines, WordsError> {
    let items = items_of(c, held, now);
    let the_taf = match whose.name {
        Some(n) => n.to_string(),
        None => w.line("chat.the_taf")?,
    };
    let mut out = Vec::new();
    let mut live_at = None;
    let mut pins = Vec::new();
    for (i, item) in items.iter().enumerate() {
        // The answer streaming now starts at its first words: its work block goes just before.
        if live_at.is_none() && matches!(item, Item::Streaming { .. }) {
            live_at = Some(out.len());
        }
        let at = item.at();
        if i == 0 || (at - items[i - 1].at()).num_seconds() >= 3600 {
            out.push(Line::Gap(gap(w.language(), at, now)));
        }
        if let Item::Message(m) = item {
            if m.from == From::Owner && m.kind != MessageKind::Kickoff {
                pins.push((
                    out.len(),
                    crate::pin::Candidate {
                        id: item.id(),
                        words: m.text.clone(),
                        message: true,
                        unsent: false,
                    },
                ));
            }
        }
        out.push(match item {
            Item::Streaming { text, .. } => Line::Taf {
                work: None,
                delivered: None,
                text: text.clone(),
                counted: None,
                refused: None,
                stamp: None,
                report: None,
                streaming: true,
            },
            Item::Message(m) => message(w, m)?,
            Item::Held { key, .. } => Line::Held(key.to_string()),
            Item::Outcome(e) => Line::Outcome(outcome_line(w, e)?),
            Item::Knock(k) => Line::Knock((*k).clone()),
            Item::AskCoach(card) => Line::AskCoach {
                text: card.text.clone(),
                ask: if whose.ask_coach {
                    Some(w.line("coach.ask")?)
                } else {
                    None
                },
            },
        });
    }
    let unsent = c
        .unsent
        .iter()
        .map(|u| unsent(w, u, whose.name))
        .collect::<Result<Vec<_>, _>>()?;
    let thinking = c.thinking();
    let steering = match &c.thread.steering {
        Some(s) if !s.over() => {
            let paused = s.status == "PAUSED";
            let note = match whose.name {
                Some(n) => w.get(
                    if paused {
                        "chat.steer.taf_reads_paused"
                    } else {
                        "chat.steer.taf_reads_next"
                    },
                    &[("taf", n)],
                )?,
                None => w.line(if paused {
                    "chat.steer.it_reads_paused"
                } else {
                    "chat.steer.it_reads_next"
                })?,
            };
            s.untaken
                .iter()
                .map(|m| (m.text.clone(), note.clone()))
                .collect()
        }
        _ => vec![],
    };
    Ok(ChatLines {
        empty: if items.is_empty() && c.unsent.is_empty() {
            let name = match whose.name {
                Some(n) => n.to_string(),
                None => w.line("chat.it_lower")?,
            };
            Some(w.get("chat.empty_named", &[("taf", &name)])?)
        } else {
            None
        },
        lines: out,
        unsent,
        thinking: if thinking && c.live.is_none() {
            Some(w.line("chat.answering")?)
        } else {
            None
        },
        live: match &c.live {
            Some(l) => crate::coach_lines::live_work_lines(w, l, |tool| {
                crate::coach_lines::taf_activity_words(w, tool).map(Some)
            })?
            .map(|work| (work, live_at)),
            None => None,
        },
        failed: match (&c.ended, thinking) {
            (Some(Ended::Failed { .. }), false) => Some((
                failed_words(
                    w,
                    &the_taf,
                    &c.went_through,
                    &call_amounts(w, &c.record)?,
                    c.failed_why.as_deref(),
                )?,
                w.line("chat.retry")?,
            )),
            _ => None,
        },
        steering,
        // Stopped needs no line: the chat is simply ready again. Stopped at its own cap, it says why.
        capped: match (&c.capped, thinking, &c.ended) {
            (_, _, Some(Ended::Failed { .. })) => None,
            (Some(stop), false, _) => cap_words(w, stop, &the_taf)?,
            _ => None,
        },
        queued: match c.thread.steering.as_ref().and_then(|s| s.held.as_ref()) {
            Some(m) => Some((m.text.clone(), w.line("chat.held")?)),
            None => None,
        },
        menu: (
            w.line("chat.send_again")?,
            w.line("chat.edit_and_send")?,
            w.line("common.copy")?,
        ),
        pins,
    })
}

fn message(w: &WordBook, m: &ChatMessage) -> Result<Line, WordsError> {
    if m.kind == MessageKind::Kickoff {
        return Ok(Line::Kickoff(w.get(
            "chat.run_started_with",
            &[
                ("time", &crate::account_time::short_time(&m.at)),
                ("text", &m.text),
            ],
        )?));
    }
    let door = m.door.as_ref();
    Ok(match m.from {
        From::Owner => Line::Owner {
            text: m.text.clone(),
            answered: if door.is_none() && m.answers.is_some() {
                Some(w.line("chat.answered_card_above")?)
            } else {
                None
            },
            stamp: door
                .map(|d| door_stamp(w, d, true).map(|s| s.0))
                .transpose()?,
            menu: door.is_none(),
            pending: m.session == PENDING,
            files: m.attachments.clone(),
        },
        From::Taf => Line::Taf {
            // What it did before writing it, folded.
            work: m
                .work
                .as_ref()
                .map(|work| crate::coach_lines::taf_work_lines(w, work))
                .transpose()?,
            delivered: m.delivered.clone(),
            text: m.text.clone(),
            counted: m.counted.clone(),
            refused: m.refused_lines().cloned(),
            stamp: door.map(|d| door_stamp(w, d, false)).transpose()?,
            report: crate::report::reportable(m),
            streaming: false,
        },
    })
}

fn unsent(w: &WordBook, u: &Unsent, name: Option<&str>) -> Result<UnsentLines, WordsError> {
    let not_sent = format!("{} · {}", w.line("setup.not_sent")?, u.why);
    let cant = || -> Result<Option<NotReady>, WordsError> {
        Ok(
            match approval::gate(w, Mode::Kiff, approval::THIS_COMPUTER)? {
                Gate::NotReady(n) => Some(n),
                Gate::Button => None,
            },
        )
    };
    // Refused for a halt, as tafs cloud says, in its words: Retry sends it once it's resumed,
    // on the phone.
    if matches!(u.halted.as_deref(), Some("account") | Some("taf")) {
        return Ok(UnsentLines {
            id: u.id,
            words: u.text.clone(),
            note: not_sent,
            halted: true,
            resume_note: None,
            cant: cant()?,
            edit: None,
            retry: Some(w.line("chat.retry")?),
            copy: w.line("common.copy")?,
            delete: w.line("chat.delete")?,
        });
    }
    // An older tafs cloud's halt, by its words: that it's halted, and nothing reaches it.
    if let (Some(n), true) = (name, halted_words(&u.why)) {
        return Ok(UnsentLines {
            id: u.id,
            words: u.text.clone(),
            note: w.get("coach.taf_halted", &[("taf", n)])?,
            halted: true,
            resume_note: Some(w.line("coach.nothing_reaches_until_you_resume")?),
            cant: cant()?,
            edit: None,
            retry: None,
            copy: w.line("common.copy")?,
            delete: w.line("chat.delete")?,
        });
    }
    Ok(UnsentLines {
        id: u.id,
        words: u.text.clone(),
        note: not_sent,
        halted: false,
        resume_note: None,
        cant: None,
        edit: Some(w.line("setup.edit")?),
        retry: Some(w.line("chat.retry")?),
        copy: w.line("common.copy")?,
        delete: w.line("chat.delete")?,
    })
}

/// What a refused send does (`deliverChatOnce`'s and `submitChat`'s catches).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendRefused {
    /// For want of the AI-data consent: its screen, and the words back in the box.
    Consent,
    /// The tafs wait for credits, or more than tafs cloud holds: its words above the box, the
    /// owner's back in it.
    Box(String),
    /// Kept in the thread as not sent, with why; `busy` for too many too fast.
    NotSent {
        why: String,
        halted: Option<String>,
        busy: bool,
    },
}

/// How a refusal of a message, or of words written to the run, is shown. `steer`: a `held_full`
/// refusal says tafs cloud's error as it is and keeps the words in the box.
pub fn refused(
    w: &WordBook,
    r: &Refusal,
    taf: &str,
    steer: bool,
) -> Result<SendRefused, WordsError> {
    if steer && r.code.as_deref() == Some("held_full") {
        return Ok(SendRefused::Box(match &r.error {
            Some(e) => e.clone(),
            None => r.words(w)?,
        }));
    }
    if r.status == 429 && r.words.is_none() {
        return Ok(SendRefused::NotSent {
            why: w.get("chat.too_many", &[("taf", taf)])?,
            halted: None,
            busy: true,
        });
    }
    if r.wants_consent() {
        return Ok(SendRefused::Consent);
    }
    if let Some(words) = r.waits_for_credits(w)? {
        return Ok(SendRefused::Box(words));
    }
    Ok(SendRefused::NotSent {
        why: r.words(w)?,
        halted: r.halted.clone(),
        busy: false,
    })
}

#[cfg(test)]
mod tests;
