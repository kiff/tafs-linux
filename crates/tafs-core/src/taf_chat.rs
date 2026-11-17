//! A taf's chat (`TafChatView`, `AppModel`'s Chat and "A taf's chat, live", `TafsCloud.Thread`):
//! the thread as tafs cloud reads it (`GET /v1/tafs/{id}/conversation`, decoded exactly as strictly
//! or as leniently as the iPad's `thread(_:)`), what the chat holds while a message goes
//! (`ChatState`, `deliverChat`, `submitChat`, `stopChat`, `retryChat`, `settleSteering`), the
//! thread in one order (`chatItems`, `heldAfterItsWords`, `withheldUntilWords`), and what the chat
//! draws, every word tafs cloud's, the card's or the app's at the pin. The sends are Coach's own
//! (`coach_send`): the same routes, bodies and answers, into the taf's live chat. Nothing here
//! sends, and nothing is signed: Resume, after a send refused for a halt, is the phone's.

use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;

use crate::approval::{self, Gate, NotReady};
use crate::coach::{
    array_if, date, field, object, text, text_if, ChatMessage, From, MessageKind, Read, Refused,
    Steering, SteeringMessage, PENDING,
};
use crate::coach_lines::{gap, WorkLines};
use crate::coach_send::{
    merge, stamp, steered_into, ChatTarget, Delivery, Ended, Refusal, Sent, Steered, Submit,
    Unsent, CHAT_LIMIT,
};
use crate::sidebar::Mode;
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
        now: DateTime<Utc>,
    ) {
        self.sending = false;
        self.drop_local(&d.local);
        self.keep_unsent(&d.text, why, halted, None, busy, now);
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
    /// resends it on its own.
    pub fn steer_not_sent(
        &mut self,
        text: &str,
        why: &str,
        halted: Option<String>,
        busy: bool,
        now: DateTime<Utc>,
    ) {
        self.keep_unsent(text, why, halted, None, busy, now);
    }

    /// The chat Stop stops (`stopChat`): the one at work, else the thread's latest. The wait ends.
    pub fn stop(&mut self) -> Option<String> {
        self.taf.as_ref()?;
        let session = self.thread.working.clone().or_else(|| {
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
        Some((at, self.unsent.remove(at)))
    }

    /// A retry that wasn't taken at all goes back in its place.
    pub fn retry_back(&mut self, at: usize, message: Unsent) {
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
    Held { key: &'a str, at: DateTime<Utc> },
    AskCoach(&'a AskCoachCard),
}

impl Item<'_> {
    pub fn id(&self) -> String {
        match self {
            Item::Message(m) => format!("m|{}", m.id()),
            Item::Held { key, .. } => format!("k|{key}"),
            Item::AskCoach(c) => format!("a|{}", c.id),
        }
    }

    pub fn at(&self) -> DateTime<Utc> {
        match self {
            Item::Message(m) => m.at,
            Item::Held { at, .. } => *at,
            Item::AskCoach(c) => c.at,
        }
    }

    /// The session of the taf's words: none for anything else (`tafWords`).
    fn taf_words(&self) -> Option<&str> {
        match self {
            Item::Message(m) if m.from == From::Taf && m.kind != MessageKind::Kickoff => {
                Some(&m.session)
            }
            _ => None,
        }
    }
}

/// `chatItems`: the messages, the calls KIFF holds for the owner where they were held, and the
/// settings the taf handed to Coach, by time, each once. A held call's card comes after the taf's
/// words that ask for it, and waits for them a while.
pub fn items<'a>(
    thread: &'a TafThread,
    held: &'a [(String, DateTime<Utc>)],
    now: DateTime<Utc>,
) -> Vec<Item<'a>> {
    let mut all: Vec<Item> = thread.messages.iter().map(Item::Message).collect();
    all.extend(held.iter().map(|(key, at)| Item::Held { key, at: *at }));
    all.extend(thread.ask_coach.iter().map(Item::AskCoach));
    // A stable sort: by time, then the order they came in.
    all.sort_by_key(|i| i.at());
    let mut seen = HashSet::new();
    all.retain(|i| seen.insert(i.id()));
    withheld_until_words(held_after_its_words(all), now)
}

/// `heldAfterItsWords`: a held call's card moves past the words of the turn right after it, those
/// in that turn's session: the call is held before the taf writes "waiting for your approval".
pub fn held_after_its_words(mut out: Vec<Item<'_>>) -> Vec<Item<'_>> {
    let mut i = 0;
    while i < out.len() {
        if !matches!(out[i], Item::Held { .. }) {
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
    let said = items[owner + 1..].iter().any(
        |i| matches!(i, Item::Message(m) if m.from == From::Taf && m.kind != MessageKind::Kickoff),
    );
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
    },
    /// A call KIFF holds for the owner: its card, by its key.
    Held(String),
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
    /// The taf is at work: the dots, and what a screen reader says.
    pub thinking: Option<String>,
    /// Written to its run, not read yet, and when it will be.
    pub steering: Vec<(String, String)>,
    /// The latest run stopped at a cap: why.
    pub capped: Option<String>,
    /// What tafs cloud holds for this chat, to go once the taf is done, and that it waits.
    pub queued: Option<(String, String)>,
    /// An owner's message's menu: Send again, Edit and send, Copy.
    pub menu: (String, String, String),
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
    let items = items(&c.thread, held, now);
    let the_taf = match whose.name {
        Some(n) => n.to_string(),
        None => w.line("chat.the_taf")?,
    };
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let at = item.at();
        if i == 0 || (at - items[i - 1].at()).num_seconds() >= 3600 {
            out.push(Line::Gap(gap(w.language(), at, now)));
        }
        out.push(match item {
            Item::Message(m) => message(w, m)?,
            Item::Held { key, .. } => Line::Held(key.to_string()),
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
        thinking: if thinking {
            Some(w.line("chat.answering")?)
        } else {
            None
        },
        steering,
        // Stopped needs no line: the chat is simply ready again. Stopped at its own cap, it says why.
        capped: match (&c.capped, thinking) {
            (Some(stop), false) => cap_words(w, stop, &the_taf)?,
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
