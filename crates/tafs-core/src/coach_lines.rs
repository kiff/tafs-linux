//! What Coach's screen draws (`SetupChatView` at a regular width, read only): the plan pinned
//! above the thread, the owner's and Coach's messages, each card by its kind
//! (`SetupComponentView`, `SetupApproval`, `QuestionsCard`, `DriveCard`), the drives that ended,
//! and how the tasks Coach handed out went. Every word is the card's, tafs cloud's, or the app's
//! at the pin. This computer answers nothing: where a card's buttons would be, an approval or a
//! sign-in says that it can't approve here (`ApprovalGate`), and the other cards' answers are
//! drawn dimmed.

use std::collections::HashMap;

use chrono::{DateTime, Utc};

use crate::approval::{self, Gate, NotReady};
use crate::coach::{
    vendor_words, Answer, ChatMessage, Component, From, HandOver, Improvement, Item, Kind,
    MessageKind, SetupThread, ShownComponent, Work,
};
use crate::drafts::AgentDraft;
use crate::sidebar::Mode;
use crate::words::{WordBook, WordsError};

/// A colour of the theme a line is drawn in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Ink,
    Soft,
    Dim,
    Ok,
    No,
    Amber,
    /// The panel's own edge, the line colour.
    Line,
}

/// The mark before an answered card's line (`OutcomeLine.mark`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    Check,
    Minus,
    Below,
}

/// An answered card, folded to one line (`OutcomeLine`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub text: String,
    pub tone: Tone,
    pub mark: Mark,
    /// A tap opens the card again.
    pub more: bool,
    /// What a screen reader says opening it does.
    pub hint: Option<String>,
}

/// A step's outcome in a message's work (`AppModel.LiveStep.Status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Done,
    Error,
    NotMade,
}

/// One row of a message's work (`WorkLog.Row`), in its past words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepRow {
    pub words: String,
    pub status: StepStatus,
    /// "not made · read first", or how many tries.
    pub note: Option<String>,
}

/// A finished answer's work, folded (`WorkSummary`): "Worked for 12 s ›", which opens its steps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkLines {
    pub summary: String,
    pub accessible: String,
    pub shows: String,
    pub hides: String,
    pub steps: Vec<StepRow>,
}

/// A questions card's question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestionLines {
    pub title: String,
    /// Its own-answer field's label for a screen reader (`ownAnswerTo`).
    pub own_answer_to: String,
    /// Each choice's label and its line under it.
    pub options: Vec<(String, Option<String>)>,
    pub note: Option<String>,
}

/// A plan step's state, for its mark (`StepMark`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStep {
    pub status: String,
    pub label: String,
    /// What a screen reader says for its mark.
    pub mark_words: String,
}

/// The plan, pinned above the thread (`SetupPlan`): folded to the step in progress until opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanLines {
    pub progress: String,
    /// The step it's at, or the plan's title, folded.
    pub now: PlanStep,
    pub steps: Vec<PlanStep>,
    pub shows: String,
    pub folds: String,
}

/// An approval (`SetupApproval`), read only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalLines {
    /// Answered, or replaced by a newer request: folded to this line until opened.
    pub folded: Option<Outcome>,
    /// Fold, once opened from its line.
    pub fold: Option<String>,
    pub eyebrow: String,
    pub eyebrow_tone: Tone,
    pub stroke: Tone,
    /// tafs cloud's line that the connection is already done.
    pub done: Option<String>,
    /// The card's own words: what this computer has of the request, planque's not being read.
    pub title: Option<String>,
    pub text: Option<String>,
    /// tafs cloud's words for a Card approved next.
    pub card_next: Option<String>,
    /// tafs cloud's `wider.words`, above where the buttons go.
    pub wider: Option<String>,
    /// A newer request is below.
    pub replaced: Option<String>,
    /// It didn't go through, and why.
    pub failed: Option<String>,
    /// Why this computer can't approve, in the buttons' place.
    pub gate: Option<NotReady>,
    /// KIFF's draft of the taf's Card, as far as it's read.
    pub draft: Option<DraftShown>,
    /// What was approved, under the folded line: the signed Card's lines, as KIFF has them.
    pub signed_lines: Vec<String>,
}

/// The taf's Card on an approval (`SetupApproval`): KIFF's draft as it reads, or why it isn't
/// drawn yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftShown {
    /// The Card's lines under its title, or, as the one approved next, under tafs cloud's words
    /// for it and dimmed.
    Card {
        title: Option<String>,
        next_words: Option<String>,
        lines: Vec<String>,
    },
    /// KIFF wouldn't give it: its reason, and Try again, which reads draft `id` again.
    NotRead {
        id: String,
        why: String,
        try_again: String,
    },
    /// Not read yet.
    Reading(String),
}

/// KIFF's drafts as this computer read them, and why one wasn't (`setupDrafts`,
/// `setupDraftErrors`).
#[derive(Clone, Copy)]
pub struct Drafts<'a> {
    pub read: &'a HashMap<String, AgentDraft>,
    pub errors: &'a HashMap<String, String>,
}

impl<'a> Drafts<'a> {
    /// The draft a card names, as read.
    pub fn of(&self, c: &Component) -> Option<&'a AgentDraft> {
        c.draft.as_deref().and_then(|d| self.read.get(d))
    }
}

/// A vendor's sign-in (`SetupComponentView.signIn`), read only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignInLines {
    pub eyebrow: String,
    pub eyebrow_tone: Tone,
    pub stroke: Tone,
    pub title: String,
    pub failed: Option<String>,
    pub note: String,
    pub wider: Option<String>,
    pub gate: Option<NotReady>,
}

/// A result: a new taf handed over, or an improvement's new version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultLines {
    pub eyebrow: String,
    pub text: String,
    /// An improvement's note, or the tools a new taf has, in words.
    pub detail: Option<String>,
    /// tafs cloud's words when the taf was made with no plan, and where a plan is chosen.
    pub plan_needed: Option<(String, String)>,
    /// The taf's page, once listed: its button's words and the taf's key.
    pub open: Option<(String, String)>,
    /// What stands where the button would, each in its colour.
    pub lines: Vec<(String, Tone)>,
    /// The new version is being made: its button, dimmed.
    pub preparing: Option<String>,
    /// The new version, made, of a taf listed here: its button's words ("See version 3") and the
    /// taf's key, whose Versions it opens.
    pub see_version: Option<(String, String)>,
}

/// A card, by its kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Card {
    Notice {
        title: Option<String>,
        text: String,
        tone: Tone,
    },
    Outcome(Outcome),
    /// A confirm's question and its choices, each its label and the value sent.
    Confirm {
        id: String,
        text: String,
        options: Vec<(String, String)>,
    },
    /// A questions card as it waits (`QuestionsCard`): what the owner picks is the app's to
    /// keep, so the card carries its questions as Coach sent them.
    Questions {
        id: String,
        text: Option<String>,
        eyebrow: String,
        count: String,
        questions: Vec<QuestionLines>,
        asked: Vec<crate::coach::Question>,
        something_else: String,
        own_answer: String,
        send: String,
        chat_about: String,
        talking: String,
        show_again: String,
    },
    Approval(ApprovalLines),
    SignIn(SignInLines),
    Result(ResultLines),
    /// Coach asks to drive the app (`DriveCard`), never on this computer.
    Drive {
        eyebrow: String,
        task: String,
        note: String,
        gate: Option<NotReady>,
    },
    /// A dial card: its title alone, until it's ported.
    Title(String),
}

/// One line of the thread. Built once a read, so a card isn't boxed.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Line {
    /// A time between parts of the thread that are far apart.
    Gap(String),
    /// The owner's words, and that they answered the card above.
    Owner {
        text: String,
        answered: Option<String>,
        /// The files sent with it (`MessageFiles`), each opened full size.
        files: Vec<crate::coach::Attachment>,
    },
    /// Coach's words (Markdown), under its work; tafs cloud's id for them and their chat when
    /// they can be reported.
    Coach {
        work: Option<WorkLines>,
        text: String,
        report: Option<crate::report::Reported>,
    },
    Card {
        card_ref: String,
        card: Card,
    },
    /// A drive's end: what Coach did in the app, each step, and why it ended.
    Drive {
        eyebrow: String,
        task: String,
        steps: Vec<String>,
        /// No step was done.
        none: Option<String>,
        ended: String,
    },
}

/// The empty chat of a conversation on a taf (`starts` with `setupImproving`): what it's about, as
/// a choice (`startSwitch`: a new taf, or this taf, chosen), "Improve {taf}" and what Coach does
/// with it, and three first messages that send as they are (`improveExamples`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImproveLines {
    pub about: String,
    pub new_taf: String,
    pub improve: String,
    pub line: String,
    pub examples: Vec<String>,
}

/// Coach's screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachLines {
    pub title: String,
    pub plan: Option<PlanLines>,
    /// The empty chat: the question it answers, and what Coach does.
    pub starts: Option<(String, String)>,
    /// The empty chat of a conversation on a taf, in `starts`' place.
    pub improve: Option<ImproveLines>,
    pub lines: Vec<Line>,
    /// What ended while the owner was away, in one message.
    pub away: Option<(String, Vec<String>)>,
    /// How the tasks Coach handed tafs went, until Coach reads them.
    pub coach_lines: Vec<String>,
    /// Coach is at work.
    pub thinking: Option<String>,
    /// Written to Coach's run while it goes, not read yet, and when it will be.
    pub steering: Vec<(String, String)>,
    /// What tafs cloud holds for this chat, to go once Coach is done (`QueuedBubble`), and that it
    /// waits.
    pub queued: Option<(String, String)>,
    /// The thread's items in the order drawn, to keep it (`rememberDrawn`).
    pub order: Vec<crate::coach::Entry>,
}

/// The message box (`composer`): what it says while empty, and its button's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComposerLines {
    pub placeholder: String,
    /// Send, for a screen reader: while Coach works, that it goes when the answer is done.
    pub send: String,
    /// Stop, which takes Send's place while Coach works and nothing is written.
    pub stop: String,
}

/// `composer`'s words.
pub fn composer(w: &WordBook, thinking: bool) -> Result<ComposerLines, WordsError> {
    Ok(ComposerLines {
        placeholder: w.line(if thinking {
            "chat.write_next"
        } else {
            "coach.ask_anything"
        })?,
        send: w.line(if thinking {
            "coach.send_when_answer_done"
        } else {
            "common.send"
        })?,
        stop: w.line("setup.stop")?,
    })
}

/// `NeedsYouBar`'s words for the card waiting while it's out of view: an approval with its app,
/// a sign-in at its app, a drive, or a question. The app is the card's own service: planque's
/// grants, which name it first on the iPad, aren't read here.
pub fn needs_you_bar(w: &WordBook, c: &crate::coach::Component) -> Result<String, WordsError> {
    let vendor = c.service.as_deref().filter(|s| !s.is_empty());
    Ok(match c.kind {
        Kind::Approval => {
            let words = w.line("coach.needs_you_approve")?;
            match vendor {
                Some(v) => format!("{words} {v}"),
                None => words,
            }
        }
        Kind::SignIn => {
            let words = w.line("coach.sign_in.eyebrow")?;
            match vendor {
                Some(v) => format!("{words} at {v}"),
                None => words,
            }
        }
        Kind::Drive => w.line("drive.needs_you_let_coach_drive")?,
        _ => w.line("coach.needs_you_question")?,
    })
}

/// A message that didn't go through (`UnsentBubble`): the owner's words, dimmed, "Not sent · why",
/// then Edit and Retry, or Resume and Retry for a send refused because the account is halted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnsentLines {
    pub id: u64,
    pub words: String,
    pub note: String,
    pub edit: Option<String>,
    pub retry: String,
    /// Opens the account's halt, where it's resumed.
    pub resume: Option<String>,
    /// Its menu (`contextMenu`): Copy, which copies the message as it was written, and Delete.
    pub copy: String,
    pub text: String,
    pub delete: String,
}

/// Coach's messages that didn't go through, in order.
pub fn unsent(
    w: &WordBook,
    thread: &SetupThread,
    unsent: &[crate::coach_send::Unsent],
) -> Result<Vec<UnsentLines>, WordsError> {
    unsent
        .iter()
        .map(|u| {
            // Coach's chat is no taf's: only an account's halt offers Resume.
            let account = u.halted.as_deref() == Some("account");
            Ok(UnsentLines {
                id: u.id,
                words: owner_words(w, thread, &u.text)?,
                note: format!("{} · {}", w.line("setup.not_sent")?, u.why),
                edit: if account {
                    None
                } else {
                    Some(w.line("setup.edit")?)
                },
                retry: w.line("chat.retry")?,
                resume: if account {
                    Some(w.line("taf.resume")?)
                } else {
                    None
                },
                copy: w.line("common.copy")?,
                text: u.text.clone(),
                delete: w.line("chat.delete")?,
            })
        })
        .collect()
}

/// One of Coach's chats in Previous conversations (`SetupChatsList`): what the owner asked, when it
/// last changed, and whether it's the one shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatRow {
    pub session: String,
    pub title: String,
    pub when: String,
    pub shown: bool,
}

/// The chats, in tafs cloud's order (newest first).
pub fn chat_rows(
    w: &WordBook,
    chats: &[crate::coach::Chat],
    shown: Option<&str>,
) -> Result<Vec<ChatRow>, WordsError> {
    chats
        .iter()
        .map(|c| {
            Ok(ChatRow {
                session: c.session.clone(),
                title: if c.title.trim().is_empty() {
                    w.line("coach.chats.untitled")?
                } else {
                    c.title.clone()
                },
                when: format!(
                    "{}, {}",
                    crate::home::long_date(w.language(), crate::account_time::day(&c.updated)),
                    crate::account_time::short_time(&c.updated)
                ),
                shown: shown == Some(c.session.as_str()),
            })
        })
        .collect()
}

/// How the last run ended (`ended`): stopped by the owner.
pub fn ended(
    w: &WordBook,
    ended: Option<crate::coach_send::Ended>,
) -> Result<Option<String>, WordsError> {
    match ended {
        Some(crate::coach_send::Ended::Stopped) => w.line("taf.state.stopped").map(Some),
        None => Ok(None),
    }
}

/// A taf on the account's list, by its KIFF agent, with its key and name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listed {
    pub agent: String,
    pub key: String,
    pub name: String,
}

/// Where an approval or a sign-in stands (`AppModel.CardState`), from the thread alone: this
/// computer reads no grant from planque.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CardState {
    NeedsYou,
    Done,
    Declined,
    Failed(Option<String>),
}

/// `setupCardState`, without planque's grant.
pub fn card_state(thread: &SetupThread, c: &Component) -> CardState {
    let answer = thread.answer_to(&c.id);
    if c.kind == Kind::Approval
        && answer.as_ref().is_some_and(|a| {
            !a.failed()
                && (a.text.starts_with("approved") || a.text.starts_with("already approved"))
        })
    {
        return CardState::Done;
    }
    if let Some(a) = &answer {
        if a.text == "declined" || a.text == "not now" {
            return CardState::Declined;
        }
        if a.failed() {
            return CardState::Failed(a.why());
        }
    }
    if c.kind == Kind::SignIn && answer.is_some() {
        return CardState::Done;
    }
    CardState::NeedsYou
}

/// `QuestionAnswers.words`: the answers in the owner's words, in the card's order.
pub fn question_words(
    w: &WordBook,
    text: &str,
    questions: &[crate::coach::Question],
) -> Result<Option<String>, WordsError> {
    let Ok(serde_json::Value::Object(obj)) = serde_json::from_str::<serde_json::Value>(text) else {
        return Ok(None);
    };
    if obj.values().any(|v| !v.is_object()) {
        return Ok(None);
    }
    let mut order: Vec<String> = questions.iter().map(|q| q.id.clone()).collect();
    let mut rest: Vec<String> = obj
        .keys()
        .filter(|k| !questions.iter().any(|q| &q.id == *k))
        .cloned()
        .collect();
    rest.sort();
    order.extend(rest);
    let mut parts = Vec::new();
    for id in order {
        let Some(a) = obj.get(&id) else { continue };
        let labels = questions
            .iter()
            .find(|q| q.id == id)
            .map(|q| q.options.as_slice())
            .unwrap_or_default();
        let label = |v: &str| {
            labels
                .iter()
                .find(|o| o.value == v)
                .map_or_else(|| v.to_string(), |o| o.label.clone())
        };
        if let Some(v) = a.get("value").and_then(|v| v.as_str()) {
            parts.push(label(v));
        } else if let Some(vs) = a.get("values").and_then(|v| v.as_array()).and_then(|vs| {
            vs.iter()
                .map(|v| v.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>()
        }) {
            parts.push(vs.iter().map(|v| label(v)).collect::<Vec<_>>().join(", "));
        } else if let Some(own) = a.get("own").and_then(|v| v.as_str()) {
            parts.push(w.get("coach.answer.in_your_words", &[("text", own)])?);
        }
    }
    Ok((!parts.is_empty()).then(|| parts.join(" · ")))
}

/// "1 of 3 answered", on a questions card.
pub fn answered_of(w: &WordBook, answered: usize, total: usize) -> Result<String, WordsError> {
    w.get(
        "coach.questions.answered_of",
        &[
            ("answered", &answered.to_string()),
            ("total", &total.to_string()),
        ],
    )
}

/// `ownerWords`: an answer to a card reads as what the owner chose, never as contract 1's form.
pub fn owner_words(w: &WordBook, thread: &SetupThread, text: &str) -> Result<String, WordsError> {
    let Some(a) = Answer::parse(text) else {
        return Ok(text.to_string());
    };
    let component = |id: &str| {
        thread
            .components
            .iter()
            .find(|c| c.component.id == id)
            .map(|c| &c.component)
    };
    Ok(match Kind::from_raw(&a.kind) {
        Some(Kind::Confirm) => component(&a.id)
            .and_then(|c| c.options.as_ref())
            .and_then(|o| o.iter().find(|o| o.value == a.text))
            .map_or_else(|| a.text.clone(), |o| o.label.clone()),
        Some(Kind::Approval) if a.failed() => w.line("coach.answer.approve_failed")?,
        Some(Kind::Approval) if a.text == "declined" => w.line("coach.answer.decline")?,
        Some(Kind::Approval) => w.line("approval.approve")?,
        Some(Kind::SignIn) if a.failed() => w.line("coach.answer.sign_in_failed")?,
        Some(Kind::SignIn) if a.text == "not now" => w.line("common.not_now")?,
        Some(Kind::SignIn) => w.line("coach.answer.signed_in")?,
        Some(Kind::Drive) if a.text == "declined" => w.line("common.not_now")?,
        Some(Kind::Drive) => w.line("approval.approve")?,
        Some(Kind::Questions) => {
            let questions = component(&a.id)
                .and_then(|c| c.questions.clone())
                .unwrap_or_default();
            question_words(w, &a.text, &questions)?.unwrap_or(a.text.clone())
        }
        _ => a.text.clone(),
    })
}

/// `AppModel.activityWords`: Coach's step in Coach's words; none for what it only showed.
pub fn activity_words(w: &WordBook, tool: &str) -> Result<Option<String>, WordsError> {
    let name = tool.rsplit("__").next().unwrap_or(tool);
    let key = match name {
        "planque" => "coach.activity.connection",
        "kiff_draft" => "coach.activity.card",
        "kiff_card" | "kiff_pending" => "coach.activity.kiff",
        "tafs_state" => "coach.activity.tafs",
        "taf_run" => "coach.activity.run",
        "taf_task" => "coach.activity.task",
        "run_stop" => "coach.activity.stop",
        "taf_pause" => "coach.activity.pause",
        "version_try" => "coach.activity.try_version",
        "version_install" => "coach.activity.install_version",
        "memory" => "taf.activity.reading_notes",
        "show" => return Ok(None),
        _ => "chat.working",
    };
    w.line(key).map(Some)
}

/// `WorkLog.duration`: "12 s", "1 min 5 s".
pub fn duration(w: &WordBook, ms: i64) -> Result<String, WordsError> {
    let s = ((ms as f64 / 1000.0).round() as i64).max(1);
    if s < 60 {
        w.get("work.duration.seconds", &[("seconds", &s.to_string())])
    } else {
        w.get(
            "work.duration.minutes",
            &[
                ("minutes", &(s / 60).to_string()),
                ("seconds", &(s % 60).to_string()),
            ],
        )
    }
}

/// `WorkSummary(coach: true)`: Coach's steps in its words, without what it only showed.
pub fn work_lines(w: &WordBook, work: &Work) -> Result<WorkLines, WordsError> {
    work_lines_in(w, work, |tool| activity_words(w, tool))
}

/// `AppModel.tafActivityWords`: what a taf is doing, in the owner's words, from the tool it calls.
/// Every tool that reaches the company's systems goes through KIFF, by its name in words past any
/// prefix (`KIFFMapping.toolWords`): this computer reads no connection's title for it.
pub fn taf_activity_words(w: &WordBook, tool: &str) -> Result<String, WordsError> {
    match tool {
        "memory" => w.line("taf.activity.reading_notes"),
        "kiff_pending" => w.line("taf.activity.checking_waits"),
        _ => w.get(
            "work.asking_kiff_run",
            &[("tool", &crate::held::tool_words(tool))],
        ),
    }
}

/// `WorkSummary` in a taf's chat: every step in the taf's words.
pub fn taf_work_lines(w: &WordBook, work: &Work) -> Result<WorkLines, WordsError> {
    work_lines_in(w, work, |tool| taf_activity_words(w, tool).map(Some))
}

/// `WorkSummary` with `WorkLog.rows(running: false)`: each step in `words`' words (one with none
/// is left out), tries of one step after a failed or unmade one as one row, and the failed ones
/// left out once something worked.
fn work_lines_in(
    w: &WordBook,
    work: &Work,
    words: impl Fn(&str) -> Result<Option<String>, WordsError>,
) -> Result<WorkLines, WordsError> {
    let mut steps: Vec<(String, StepStatus)> = Vec::new();
    for s in &work.steps {
        let Some(words) = words(&s.tool)? else {
            continue;
        };
        let status = if s.not_made {
            StepStatus::NotMade
        } else if s.error {
            StepStatus::Error
        } else {
            StepStatus::Done
        };
        steps.push((words, status));
    }
    let mut groups: Vec<Vec<(String, StepStatus)>> = Vec::new();
    for s in steps {
        match groups.last_mut() {
            Some(g)
                if g[0].0 == s.0
                    && matches!(g[g.len() - 1].1, StepStatus::Error | StepStatus::NotMade) =>
            {
                g.push(s)
            }
            _ => groups.push(vec![s]),
        }
    }
    let rows: Vec<(String, StepStatus, usize)> = groups
        .iter()
        .map(|g| (g[0].0.clone(), g[g.len() - 1].1, g.len()))
        .collect();
    let any_done = rows.iter().any(|r| r.1 == StepStatus::Done);
    let mut out = Vec::new();
    for (words, status, tries) in rows {
        if any_done && status == StepStatus::Error {
            continue;
        }
        out.push(StepRow {
            words: words.strip_suffix('…').unwrap_or(&words).to_string(),
            status,
            note: if status == StepStatus::NotMade {
                Some(w.line("chat.work.not_made_read_first")?)
            } else if tries > 1 {
                Some(w.count("chat.work.tries", tries as i64, &[])?)
            } else {
                None
            },
        });
    }
    let took = duration(w, work.ms)?;
    Ok(WorkLines {
        summary: w.get("chat.work.worked_for", &[("ms", &took)])?,
        accessible: w.count(
            "chat.work.worked_for_steps",
            out.len() as i64,
            &[("took", &took)],
        )?,
        shows: w.line("chat.work.shows_steps")?,
        hides: w.line("chat.work.hides_steps")?,
        steps: out,
    })
}

pub fn plan_step(w: &WordBook, status: &str, label: &str) -> Result<PlanStep, WordsError> {
    Ok(PlanStep {
        status: status.to_string(),
        label: label.to_string(),
        mark_words: w.line(match status {
            "done" => "coach.plan.step_done",
            "failed" => "coach.plan.step_failed",
            "active" => "coach.plan.step_active",
            "skipped" => "coach.plan.step_skipped",
            _ => "coach.plan.step_pending",
        })?,
    })
}

/// `SetupPlan`.
fn plan_lines(w: &WordBook, shown: &ShownComponent) -> Result<PlanLines, WordsError> {
    let steps = shown.component.steps.clone().unwrap_or_default();
    let done = steps.iter().filter(|s| s.status == "done").count();
    // The step it's at: the active one, else the first not yet done, else the last.
    let now = steps
        .iter()
        .find(|s| s.status == "active")
        .or_else(|| {
            steps
                .iter()
                .find(|s| s.status == "pending" || s.status == "failed")
        })
        .or(steps.last());
    let now = match now {
        Some(s) => plan_step(w, &s.status, &s.label)?,
        None => PlanStep {
            status: String::new(),
            label: match &shown.component.title {
                Some(t) => t.clone(),
                None => w.line("coach.plan_title")?,
            },
            mark_words: String::new(),
        },
    };
    Ok(PlanLines {
        progress: w.get(
            "coach.plan.progress",
            &[
                ("done", &done.to_string()),
                ("total", &steps.len().to_string()),
            ],
        )?,
        now,
        steps: steps
            .iter()
            .map(|s| plan_step(w, &s.status, &s.label))
            .collect::<Result<_, _>>()?,
        shows: w.line("coach.shows_whole_plan")?,
        folds: w.line("coach.folds_plan")?,
    })
}

fn gate(w: &WordBook, mode: Mode) -> Result<Option<NotReady>, WordsError> {
    Ok(match approval::gate(w, mode, approval::THIS_COMPUTER)? {
        Gate::NotReady(n) => Some(n),
        Gate::Button => None,
    })
}

/// A string with the space around it gone, or none.
fn nonblank(s: Option<&str>) -> Option<String> {
    s.map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

/// `SetupApproval`, read only: the card's words, tafs cloud's marks, then why this computer
/// can't approve in its buttons' place.
fn approval_lines(
    w: &WordBook,
    thread: &SetupThread,
    c: &Component,
    mode: Mode,
    drafts: Drafts,
) -> Result<ApprovalLines, WordsError> {
    let state = card_state(thread, c);
    let draft = drafts.of(c);
    // The draft whose Card this card's approval signed: KIFF took the signature, and it holds
    // tools (`signedCard`).
    let signed_card = draft.filter(|d| d.signed() && d.holds_tools());
    let replaced = thread.replaced(&c.id);
    let answer = thread.answer_to(&c.id);
    let title = nonblank(c.title.as_deref());
    let vendor = match &c.service {
        Some(s) => vendor_words(s),
        None => w.line("coach.the_vendor")?,
    };
    let folded = if replaced {
        // What happened, then where to look.
        let line = match &answer {
            Some(a) if !a.failed() && a.text.starts_with("approved") => {
                "coach.answer.approved_more_below"
            }
            Some(a) if a.text == "declined" => "coach.answer.declined_again_below",
            _ => "coach.setup_changed_request_see_below",
        };
        Some(Outcome {
            text: w.line(line)?,
            tone: Tone::Dim,
            mark: Mark::Below,
            more: true,
            hint: Some(w.line("coach.shows_earlier_request")?),
        })
    } else if matches!(state, CardState::Done | CardState::Declined) {
        let declined = state == CardState::Declined;
        // A Card that was signed is named first, before Coach's title for the card.
        let signed = match signed_card {
            Some(d) if !declined => Some(d.card_title(w)?),
            _ => None,
        };
        // An approval with the taf's Card is the Card's, not the tool's again.
        let card = match draft.filter(|d| d.holds_tools()) {
            Some(d) => Some(d.card_title(w)?),
            None => None,
        };
        let text = if !declined
            && answer
                .as_ref()
                .is_some_and(|a| a.text.starts_with("already approved"))
        {
            format!(
                "{} · {}",
                w.line("coach.already_approved")?,
                signed.clone().or(title.clone()).unwrap_or(vendor.clone())
            )
        } else if let Some(t) = &signed {
            w.get("coach.answer.approved_title", &[("title", t)])?
        } else if let Some(t) = &title {
            w.get(
                if declined {
                    "coach.answer.declined_title"
                } else {
                    "coach.answer.approved_title"
                },
                &[("title", t)],
            )?
        } else if let Some(t) = &card {
            w.get(
                if declined {
                    "coach.answer.declined_title"
                } else {
                    "coach.answer.approved_title"
                },
                &[("title", t)],
            )?
        } else if declined {
            w.get("coach.answer.declined_title", &[("title", &vendor)])?
        } else {
            w.get("coach.approved_signed", &[("vendor", &vendor)])?
        };
        Some(Outcome {
            text,
            tone: if declined { Tone::Dim } else { Tone::Ok },
            mark: if declined { Mark::Minus } else { Mark::Check },
            more: true,
            hint: Some(w.line("coach.shows_approval")?),
        })
    } else {
        None
    };
    let (eyebrow, eyebrow_tone) = if replaced {
        (w.line("coach.earlier_request")?, Tone::Dim)
    } else {
        match &state {
            CardState::Done => (w.line("coach.answer.approved_short")?, Tone::Ok),
            CardState::Declined => (w.line("coach.answer.declined_short")?, Tone::Dim),
            CardState::Failed(_) => (w.line("coach.answer.failed_short")?, Tone::No),
            CardState::NeedsYou => (w.line("coach.needs_you_approve")?, Tone::Amber),
        }
    };
    let done = c
        .connection
        .as_ref()
        .and_then(|x| x.done_words())
        .map(str::to_string);
    // `cardNextWords`: only on a card with no draft, and not when its mark says the Card is
    // signed now or the connection is already done.
    let card_next = if c.draft.is_none()
        && c.signs.as_ref().and_then(|s| s.card) != Some(true)
        && done.is_none()
    {
        c.signs.as_ref().and_then(|s| s.words.clone())
    } else {
        None
    };
    let failed = match (&state, replaced) {
        (CardState::Failed(why), false) => Some(match why {
            Some(why) => w.get("coach.answer.failed_why", &[("value", why)])?,
            None => w.line("coach.did_not_go_through_nothing")?,
        }),
        _ => None,
    };
    let asks = !replaced && matches!(state, CardState::NeedsYou | CardState::Failed(_));
    // The taf's Card as the approval signs it, or, by tafs cloud's mark, as the one approved
    // next: under its words and dimmed. Marked next with no words, it isn't drawn. A draft not
    // read yet says so, never a blank: KIFF's reason with Try again when it wouldn't give it.
    let next = c.signs.as_ref().is_some_and(|s| s.card_next());
    let next_words = c.signs.as_ref().and_then(|s| s.words.clone());
    let draft_shown = match (draft, &c.draft) {
        (Some(d), _) if d.cards.is_some() && !(next && next_words.is_none()) => {
            Some(DraftShown::Card {
                title: if next { None } else { Some(d.card_title(w)?) },
                next_words: if next { next_words } else { None },
                lines: d.lines(w)?,
            })
        }
        (None, Some(id)) if !replaced => Some(match drafts.errors.get(id) {
            Some(why) => DraftShown::NotRead {
                id: id.clone(),
                why: w.get("coach.card_draft_not_read", &[("why", why)])?,
                try_again: w.line("common.try_again")?,
            },
            None => DraftShown::Reading(w.line("coach.reading_kiff_words")?),
        }),
        _ => None,
    };
    let signed_lines = match (&state, signed_card) {
        (CardState::Done, Some(d)) => d.lines(w)?,
        _ => vec![],
    };
    Ok(ApprovalLines {
        draft: draft_shown,
        signed_lines,
        fold: if folded.is_some() {
            Some(w.line("coach.plan.fold_short")?)
        } else {
            None
        },
        folded,
        eyebrow,
        eyebrow_tone,
        stroke: if replaced {
            Tone::Line
        } else {
            match state {
                CardState::Done => Tone::Ok,
                CardState::Failed(_) => Tone::No,
                _ => Tone::Amber,
            }
        },
        title: if done.is_some() { None } else { title },
        text: if done.is_some() {
            None
        } else {
            nonblank(c.text.as_deref())
        },
        done,
        card_next,
        wider: if replaced {
            None
        } else {
            c.wider_words().map(str::to_string)
        },
        replaced: if replaced {
            Some(w.line("coach.newer_request_connection_below")?)
        } else {
            None
        },
        failed,
        gate: if asks { gate(w, mode)? } else { None },
    })
}

/// `SetupComponentView.signIn`, read only.
fn sign_in(
    w: &WordBook,
    thread: &SetupThread,
    c: &Component,
    mode: Mode,
) -> Result<Card, WordsError> {
    // The app by name: the card's service; "the vendor" only when it names none.
    let app = nonblank(c.service.as_deref());
    let vendor = match &app {
        Some(a) => a.clone(),
        None => w.line("coach.the_vendor")?,
    };
    let state = card_state(thread, c);
    Ok(match state {
        CardState::Done => Card::Outcome(Outcome {
            text: w.get("coach.answer.signed_in_at", &[("vendor", &vendor)])?,
            tone: Tone::Ok,
            mark: Mark::Check,
            more: false,
            hint: None,
        }),
        CardState::Declined => Card::Outcome(Outcome {
            text: w.get("coach.not_no_sign", &[("vendor", &vendor)])?,
            tone: Tone::Dim,
            mark: Mark::Minus,
            more: false,
            hint: None,
        }),
        _ => {
            let failure = matches!(state, CardState::Failed(_));
            Card::SignIn(SignInLines {
                eyebrow: w.line(if failure {
                    "coach.did_not_go_through_sign"
                } else {
                    "coach.sign_in.eyebrow"
                })?,
                eyebrow_tone: if failure { Tone::No } else { Tone::Amber },
                stroke: if failure { Tone::Ink } else { Tone::Amber },
                title: match &app {
                    Some(a) => w.get("coach.sign_in.at", &[("vendor", a)])?,
                    None => w.line("coach.sign_in.at_vendor")?,
                },
                failed: match &state {
                    CardState::Failed(Some(why)) => {
                        Some(w.get("coach.answer.failed_why", &[("value", why)])?)
                    }
                    CardState::Failed(None) => Some(w.line("coach.answer.failed")?),
                    _ => None,
                },
                note: match &c.note {
                    Some(n) => n.clone(),
                    None => w.line("coach.sign_in_note")?,
                },
                wider: c.wider_words().map(str::to_string),
                gate: gate(w, mode)?,
            })
        }
    })
}

/// A result: an improvement's or a new taf's (`result`, `improvement`, `handOver`).
fn result(
    w: &WordBook,
    thread: &SetupThread,
    shown: &ShownComponent,
    listed: &[Listed],
) -> Result<Card, WordsError> {
    let c = &shown.component;
    let card = shown.card_ref();
    let text = c.text.clone().unwrap_or_default();
    if c.taf.as_ref().is_some_and(|t| t.improves.is_some()) {
        let name = c.taf.as_ref().map(|t| t.name.clone()).unwrap_or_default();
        let mut out = ResultLines {
            eyebrow: w.get("coach.result.new_version", &[("taf", &name)])?,
            text,
            detail: c
                .taf
                .as_ref()
                .and_then(|t| t.note.clone())
                .filter(|n| !n.is_empty()),
            plan_needed: None,
            open: None,
            lines: vec![],
            preparing: None,
            see_version: None,
        };
        // The taf it improves (`improvedTaf`): by its id at tafs cloud, else by its agent.
        let improved = c.taf.as_ref().and_then(|t| {
            let improves = t.improves.as_deref();
            listed
                .iter()
                .find(|l| Some(l.key.as_str()) == improves)
                .or_else(|| listed.iter().find(|l| Some(l.agent.as_str()) == improves))
                .or_else(|| listed.iter().find(|l| l.agent == t.agent))
        });
        match thread.improvement(&card) {
            // Its Versions, where it's tried and switched to; a taf not listed here, in words.
            Improvement::Made(n) => match improved {
                Some(l) => {
                    out.see_version = Some((
                        w.get("coach.result.see_version", &[("version", &n.to_string())])?,
                        l.key.clone(),
                    ))
                }
                None => out.lines.push((
                    w.get("coach.result.version_made", &[("version", &n.to_string())])?,
                    Tone::Dim,
                )),
            },
            Improvement::Failed(why) => {
                out.lines.push((
                    w.get(
                        "coach.not_improved",
                        &[("owner", &name), ("why", why.trim_matches(['.', ' ']))],
                    )?,
                    Tone::No,
                ));
                out.lines.push((w.line("coach.coach_knows")?, Tone::Dim));
            }
            Improvement::Preparing => out.preparing = Some(w.line("coach.result.preparing")?),
        }
        return Ok(Card::Result(out));
    }
    // The name the taf got once started: tafs cloud may have given it the first free one.
    let started = c
        .taf
        .as_ref()
        .and_then(|t| listed.iter().find(|l| l.agent == t.agent));
    let setup_name = c.taf.as_ref().map(|t| t.name.clone()).unwrap_or_default();
    let name = started.map_or(setup_name.clone(), |s| s.name.clone());
    let renamed = started.is_some_and(|s| s.name != setup_name);
    let open = if renamed {
        w.get("coach.result.open", &[("taf", &name)])?
    } else {
        match &c.yes {
            Some(y) => y.clone(),
            None => w.get("coach.result.see_it", &[("taf", &name)])?,
        }
    };
    let mut out = ResultLines {
        eyebrow: w.get("coach.result.set_up", &[("taf", &name)])?,
        text,
        // In words, as the approval named them.
        detail: c.tools.as_ref().filter(|t| !t.is_empty()).map(|t| {
            t.iter()
                .map(|x| crate::held::tool_words(x))
                .collect::<Vec<_>>()
                .join(" · ")
        }),
        plan_needed: None,
        open: None,
        lines: vec![],
        preparing: None,
        see_version: None,
    };
    match thread.hand_over(&card, started.is_some()) {
        HandOver::SetUp => {
            if let Some(words) = thread.plan_needed(&card) {
                out.plan_needed = Some((words, w.line("usage.choose_plan")?));
            }
            match started {
                Some(s) => out.open = Some((open, s.key.clone())),
                None => out
                    .lines
                    .push((w.get("coach.setting_up", &[("name", &name)])?, Tone::Dim)),
            }
        }
        HandOver::Failed(why) => {
            out.lines.push((
                w.get(
                    "coach.not_set_up",
                    &[("name", &name), ("why", why.trim_matches(['.', ' ']))],
                )?,
                Tone::No,
            ));
            out.lines.push((w.line("coach.coach_knows")?, Tone::Dim));
        }
        HandOver::SettingUp => out
            .lines
            .push((w.get("coach.setting_up", &[("name", &name)])?, Tone::Dim)),
    }
    Ok(Card::Result(out))
}

/// One card by its kind (`SetupComponentView`).
fn card(
    w: &WordBook,
    thread: &SetupThread,
    shown: &ShownComponent,
    listed: &[Listed],
    mode: Mode,
    drafts: Drafts,
) -> Result<Option<Card>, WordsError> {
    let c = &shown.component;
    Ok(Some(match c.kind {
        Kind::Steps => return Ok(None),
        Kind::Notice => Card::Notice {
            title: c.title.clone(),
            text: c.text.clone().unwrap_or_default(),
            tone: match c.tone.as_deref() {
                Some("error") => Tone::No,
                Some("warning") => Tone::Amber,
                Some("success") => Tone::Ok,
                _ => Tone::Soft,
            },
        },
        Kind::Confirm => match thread.answer_to(&c.id) {
            Some(a) => {
                let chosen = c
                    .options
                    .as_ref()
                    .and_then(|o| o.iter().find(|o| o.value == a.text))
                    .map_or(a.text.clone(), |o| o.label.clone());
                let asked = match &c.text {
                    Some(t) => t.clone(),
                    None => w.line("coach.questions.the_question")?,
                };
                Card::Outcome(Outcome {
                    text: format!("{asked} · {chosen}"),
                    tone: Tone::Ok,
                    mark: Mark::Check,
                    more: false,
                    hint: None,
                })
            }
            None => Card::Confirm {
                id: c.id.clone(),
                text: c.text.clone().unwrap_or_default(),
                options: c
                    .options
                    .iter()
                    .flatten()
                    .map(|o| (o.label.clone(), o.value.clone()))
                    .collect(),
            },
        },
        Kind::Questions => {
            let questions = c.questions.clone().unwrap_or_default();
            match thread.answer_to(&c.id) {
                Some(a) => {
                    let words = match a.words {
                        Some(words) => words,
                        None => question_words(w, &a.text, &questions)?.unwrap_or(a.text),
                    };
                    Card::Outcome(Outcome {
                        text: w.line("coach.questions.your_answers")? + &words,
                        tone: Tone::Ok,
                        mark: Mark::Check,
                        more: false,
                        hint: None,
                    })
                }
                None => Card::Questions {
                    id: c.id.clone(),
                    text: c.text.clone().filter(|t| !t.is_empty()),
                    eyebrow: match &c.title {
                        Some(t) => t.clone(),
                        None => w.line("coach.setup_needs")?,
                    },
                    count: answered_of(w, 0, questions.len())?,
                    questions: questions
                        .iter()
                        .enumerate()
                        .map(|(i, q)| {
                            Ok(QuestionLines {
                                title: format!("{}. {}", i + 1, q.text),
                                own_answer_to: w
                                    .get("coach.questions.own_answer_to", &[("text", &q.text)])?,
                                options: q
                                    .options
                                    .iter()
                                    .map(|o| {
                                        (
                                            o.label.clone(),
                                            o.description.clone().filter(|d| !d.is_empty()),
                                        )
                                    })
                                    .collect(),
                                note: q.note.clone().filter(|n| !n.is_empty()),
                            })
                        })
                        .collect::<Result<_, WordsError>>()?,
                    asked: questions.clone(),
                    something_else: w.line("coach.questions.something_else")?,
                    own_answer: w.line("coach.own_answer")?,
                    send: w.line("coach.questions.send")?,
                    chat_about: w.line("coach.questions.chat_about")?,
                    talking: w.line("coach.questions.talking")?,
                    show_again: w.line("coach.questions.show_again")?,
                },
            }
        }
        Kind::Approval => Card::Approval(approval_lines(w, thread, c, mode, drafts)?),
        Kind::SignIn => sign_in(w, thread, c, mode)?,
        Kind::Result => result(w, thread, shown, listed)?,
        Kind::Drive => {
            let task = match (&c.text, &c.title) {
                (Some(t), _) | (None, Some(t)) => t.clone(),
                _ => w.line("drive.drive_app")?,
            };
            match thread.answer_to(&c.id) {
                Some(a) => {
                    let declined = a.text == "declined";
                    Card::Outcome(Outcome {
                        text: if declined {
                            w.line("drive.declined_line")?
                        } else {
                            w.get("drive.coach_drove_app", &[("task", &task)])?
                        },
                        tone: if declined { Tone::Dim } else { Tone::Ok },
                        mark: if declined { Mark::Minus } else { Mark::Check },
                        more: false,
                        hint: None,
                    })
                }
                None => Card::Drive {
                    eyebrow: w.line("drive.needs_you_let_coach_drive")?,
                    task,
                    note: w.line("drive.card_note_scrolls_pauses")?,
                    gate: gate(w, mode)?,
                },
            }
        }
        Kind::Knob => match nonblank(c.title.as_deref()).or(nonblank(c.text.as_deref())) {
            Some(t) => Card::Title(t),
            None => return Ok(None),
        },
    }))
}

/// `DriveRecapCard`: "What I did", the task, each step with what it touched, then why it ended:
/// tafs cloud's words as a caption, else the app's for its reason.
fn drive_recap(w: &WordBook, d: &crate::coach::DriveRecap) -> Result<Line, WordsError> {
    let steps = d
        .steps
        .iter()
        .map(|s| match &s.touched {
            Some((words, taf)) => {
                let taf = taf.as_ref().map(|t| format!(" · {t}")).unwrap_or_default();
                format!("{} · {words}{taf}", s.line)
            }
            None => s.line.clone(),
        })
        .collect();
    let ended = match d.words.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
        // `DriveSession.sentence`: its first letter a capital.
        Some(said) => {
            let mut c = said.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        }
        None => w.line(match d.why.as_deref() {
            Some("owner_took_over") => "drive.ended.owner_took_over",
            Some("left_app") => "drive.ended.left_app",
            Some("stopped") => "drive.ended.stopped",
            Some("locked") => "drive.ended.locked",
            Some("signed_out") => "drive.ended.signed_out",
            Some("halted") => "drive.ended.halted",
            Some("done") => "drive.ended.done",
            Some("idle") => "drive.ended.idle",
            Some("replaced") => "drive.ended.replaced",
            _ => "drive.ended.other",
        })?,
    };
    Ok(Line::Drive {
        eyebrow: w.line("drive.what_i_did")?,
        task: d.task.clone(),
        none: if d.steps.is_empty() {
            Some(w.line("drive.no_step_done")?)
        } else {
            None
        },
        steps,
        ended,
    })
}

/// `gapTime`: a time between parts of the thread an hour or more apart, and at its top: the time
/// of day today, else the date and the time.
pub fn gap(language: &str, at: DateTime<Utc>, now: DateTime<Utc>) -> String {
    let time = crate::account_time::short_time(&at);
    let day = crate::account_time::day(&at);
    if day == crate::account_time::day(&now) {
        time
    } else {
        format!("{}, {time}", crate::home::long_date(language, day))
    }
}

/// Coach's screen from its thread. `listed`: the account's tafs, to open the one a result set up.
pub fn lines(
    w: &WordBook,
    thread: &SetupThread,
    listed: &[Listed],
    mode: Mode,
    now: DateTime<Utc>,
) -> Result<CoachLines, WordsError> {
    lines_while(
        w,
        thread,
        listed,
        mode,
        now,
        Here {
            thinking: thread.working.is_some(),
            unsent: false,
            drawn: &[],
            drafts: Drafts {
                read: &HashMap::new(),
                errors: &HashMap::new(),
            },
        },
    )
}

/// Coach's screen as the conversation holds it here: at work while tafs cloud says so or the
/// owner just wrote (`setupThinking`), and with no empty chat's words over a message not sent.
pub fn conversation_lines(
    w: &WordBook,
    c: &crate::coach_send::Conversation,
    listed: &[Listed],
    mode: Mode,
    now: DateTime<Utc>,
) -> Result<CoachLines, WordsError> {
    let mut lines = lines_while(
        w,
        &c.thread,
        listed,
        mode,
        now,
        Here {
            thinking: c.thinking(),
            unsent: !c.unsent.is_empty(),
            drawn: &c.drawn,
            drafts: Drafts {
                read: &c.drafts,
                errors: &c.draft_errors,
            },
        },
    )?;
    // A conversation on a taf, empty: what it's about, and ways to start on it.
    if let (Some(i), true) = (&c.improving, lines.starts.is_some()) {
        let improve = w.get("coach.improve_taf", &[("taf", &i.name)])?;
        lines.starts = None;
        lines.improve = Some(ImproveLines {
            about: w.line("coach.what_conversation_about")?,
            new_taf: w.line("home.new_taf")?,
            improve,
            line: w.get("coach.improve_line", &[("name", &i.name)])?,
            examples: [
                "coach.improve.how_is_it_going",
                "coach.improve.why_refused",
                "coach.improve.cant_find_work",
            ]
            .into_iter()
            .map(|k| w.line(k))
            .collect::<Result<_, _>>()?,
        });
    }
    Ok(lines)
}

/// What the conversation holds here beside the thread: Coach at work or awaited, a message not
/// sent, and the order drawn.
struct Here<'a> {
    thinking: bool,
    unsent: bool,
    drawn: &'a [crate::coach::Entry],
    drafts: Drafts<'a>,
}

fn lines_while(
    w: &WordBook,
    thread: &SetupThread,
    listed: &[Listed],
    mode: Mode,
    now: DateTime<Utc>,
    here: Here,
) -> Result<CoachLines, WordsError> {
    let Here {
        thinking,
        unsent,
        drawn,
        drafts,
    } = here;
    // An answer to a card the thread shows reads once, as the card's outcome: no bubble repeats
    // it. One that didn't go through keeps its bubble, which says so.
    let owner_text = |i: &Item| match i {
        Item::Message(m) if m.from == From::Owner => Some(m.text.clone()),
        _ => None,
    };
    let items: Vec<Item> = thread
        .items()
        .into_iter()
        .filter(|item| match item {
            Item::Message(m) if m.from == From::Owner => !Answer::parse(&m.text).is_some_and(|a| {
                !a.failed() && thread.components.iter().any(|c| c.component.id == a.id)
            }),
            _ => true,
        })
        .collect();
    // In the order already drawn: nothing moves when tafs cloud's copy lands (`SteadyOrder`).
    let items = crate::coach::steady_order(items, drawn, Item::id, Item::at, owner_text);
    let order = items
        .iter()
        .map(|i| crate::coach::Entry {
            id: i.id(),
            owner_text: owner_text(i),
        })
        .collect();
    let mut out = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let at = item.at();
        if i == 0 || (at - items[i - 1].at()).num_seconds() >= 3600 {
            out.push(Line::Gap(gap(w.language(), at, now)));
        }
        match item {
            Item::Message(m) => out.push(message(w, thread, m)?),
            Item::Component(c) => {
                if let Some(card) = card(w, thread, c, listed, mode, drafts)? {
                    out.push(Line::Card {
                        card_ref: c.card_ref(),
                        card,
                    });
                }
            }
            Item::Drive(d) => out.push(drive_recap(w, d)?),
        }
    }
    let say = |l: &crate::coach::CoachLine| format!("{}: {}", l.taf, l.said);
    let away: Vec<String> = thread
        .coach_lines
        .iter()
        .filter(|l| l.away == Some(true))
        .map(say)
        .collect();
    let steering = match &thread.steering {
        Some(s) if !s.over() => {
            let note = w.line(if s.status == "PAUSED" {
                "chat.steer.coach_reads_paused"
            } else {
                "chat.steer.coach_reads_next"
            })?;
            s.untaken
                .iter()
                .map(|m| (m.text.clone(), note.clone()))
                .collect()
        }
        _ => vec![],
    };
    Ok(CoachLines {
        title: w.line("coach.title")?,
        plan: thread.plan().map(|p| plan_lines(w, p)).transpose()?,
        starts: if items.is_empty() && !thinking && !unsent {
            Some((
                w.line("coach.ask_anything")?,
                w.line("coach.ask_anything_line")?,
            ))
        } else {
            None
        },
        improve: None,
        lines: out,
        away: if away.is_empty() {
            None
        } else {
            Some((w.line("coach.away")?, away))
        },
        coach_lines: thread
            .coach_lines
            .iter()
            .filter(|l| l.away != Some(true))
            .map(say)
            .collect(),
        thinking: if thinking {
            Some(w.line("chat.answering")?)
        } else {
            None
        },
        steering,
        queued: match thread.steering.as_ref().and_then(|s| s.held.as_ref()) {
            Some(m) => Some((m.text.clone(), w.line("chat.held")?)),
            None => None,
        },
        order,
    })
}

fn message(w: &WordBook, thread: &SetupThread, m: &ChatMessage) -> Result<Line, WordsError> {
    Ok(match m.from {
        From::Owner => Line::Owner {
            text: owner_words(w, thread, &m.text)?,
            answered: if m.answers.is_some() {
                Some(w.line("chat.answered_card_above")?)
            } else {
                None
            },
            files: m.attachments.clone(),
        },
        From::Taf if m.kind == MessageKind::Kickoff => Line::Gap(w.get(
            "chat.run_started_with",
            &[
                ("time", &crate::account_time::short_time(&m.at)),
                ("text", &m.text),
            ],
        )?),
        From::Taf => Line::Coach {
            work: m
                .work
                .as_ref()
                .map(|work| work_lines(w, work))
                .transpose()?,
            text: m.text.clone(),
            report: crate::report::reportable(m),
        },
    })
}

/// Coach's Markdown as Pango markup (`MarkdownText`), the parts Coach writes: headings, lists,
/// bold, italic, code and links (their words). Everything else is its text, escaped.
pub fn markup(text: &str) -> String {
    fn escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
    }
    /// One pair of a marker around words, each pair on the same line.
    fn pairs(s: &str, marker: &str, open: &str, close: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(start) = rest.find(marker) {
            let after = &rest[start + marker.len()..];
            match after.find(marker) {
                Some(end) if end > 0 && !after[..end].starts_with(' ') => {
                    out.push_str(&rest[..start]);
                    out.push_str(open);
                    out.push_str(&after[..end]);
                    out.push_str(close);
                    rest = &after[end + marker.len()..];
                }
                _ => {
                    out.push_str(&rest[..start + marker.len()]);
                    rest = after;
                }
            }
        }
        out.push_str(rest);
        out
    }
    fn links(s: &str) -> String {
        let mut out = String::new();
        let mut rest = s;
        while let Some(open) = rest.find('[') {
            let after = &rest[open + 1..];
            if let Some(close) = after.find("](") {
                if let Some(end) = after[close + 2..].find(')') {
                    out.push_str(&rest[..open]);
                    out.push_str(&after[..close]);
                    rest = &after[close + 2 + end + 1..];
                    continue;
                }
            }
            out.push_str(&rest[..open + 1]);
            rest = after;
        }
        out.push_str(rest);
        out
    }
    fn inline(s: &str) -> String {
        // Code first, so nothing inside it is read as formatting.
        let mut out = String::new();
        let parts: Vec<&str> = s.split('`').collect();
        for (i, part) in parts.iter().enumerate() {
            if i % 2 == 1 && i + 1 < parts.len() {
                out.push_str("<tt>");
                out.push_str(&escape(part));
                out.push_str("</tt>");
            } else {
                if i % 2 == 1 {
                    // A backtick never closed is itself.
                    out.push('`');
                }
                let t = escape(&links(part));
                let t = pairs(&t, "**", "<b>", "</b>");
                let t = pairs(&t, "__", "<b>", "</b>");
                let t = pairs(&t, "*", "<i>", "</i>");
                out.push_str(&t);
            }
        }
        out
    }
    text.lines()
        .map(|l| {
            let t = l.trim_start();
            let indent = &l[..l.len() - t.len()];
            if let Some(h) = t
                .strip_prefix("### ")
                .or(t.strip_prefix("## "))
                .or(t.strip_prefix("# "))
            {
                format!("<b>{}</b>", inline(h))
            } else if let Some(item) = t.strip_prefix("- ").or(t.strip_prefix("* ")) {
                format!("{indent}• {}", inline(item))
            } else {
                format!("{indent}{}", inline(t))
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    fn book() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    #[test]
    fn an_empty_conversation_on_a_taf_says_so_and_offers_first_messages() {
        let w = book();
        let mut c = crate::coach_send::Conversation::default();
        c.improve("taf_1", "Rae");
        let l = conversation_lines(&w, &c, &[], Mode::Kiff, now()).unwrap();
        assert_eq!(l.starts, None);
        let i = l.improve.unwrap();
        assert_eq!(
            (i.new_taf.as_str(), i.improve.as_str()),
            ("New taf", "Improve Rae")
        );
        assert!(i.line.contains("what Rae did"));
        assert_eq!(
            i.examples,
            [
                "How is it going?",
                "Why was something refused or held?",
                "It can't find the work it should do"
            ]
        );
        // On no taf: Coach's own empty chat.
        c.new_chat();
        let l = conversation_lines(&w, &c, &[], Mode::Kiff, now()).unwrap();
        assert!(l.improve.is_none() && l.starts.is_some());
    }

    #[test]
    fn the_needs_you_bar_says_what_waits() {
        let w = book();
        let bar = |v: Value| {
            let c = crate::coach::Component::read(&v).unwrap();
            needs_you_bar(&w, &c).unwrap()
        };
        assert_eq!(
            bar(json!({"type": "approval", "id": "a", "service": "Stripe"})),
            "Needs you · approve Stripe"
        );
        assert_eq!(
            bar(json!({"type": "approval", "id": "a"})),
            "Needs you · approve"
        );
        assert_eq!(
            bar(json!({"type": "sign_in", "id": "s", "service": "Gmail"})),
            "Needs you · sign in at Gmail"
        );
        assert_eq!(
            bar(json!({"type": "drive", "id": "d"})),
            "Needs you · let Coach drive"
        );
        assert_eq!(
            bar(json!({"type": "questions", "id": "q"})),
            "Needs you · a question"
        );
    }

    fn contract() -> Value {
        let path = format!(
            "{}/../../shared/contracts/coach-conversation.json",
            env!("CARGO_MANIFEST_DIR")
        );
        serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()["answer"].clone()
    }

    fn read(v: &Value) -> SetupThread {
        SetupThread::read(&serde_json::to_vec(v).unwrap()).unwrap()
    }

    fn now() -> DateTime<Utc> {
        DateTime::parse_from_rfc3339("2026-10-16T13:00:00Z")
            .unwrap()
            .with_timezone(&Utc)
    }

    /// A thread with one approval naming KIFF's draft `drf_s1`, answered with `answer` or waiting.
    fn approval_thread(answer: Option<&str>) -> crate::coach_send::Conversation {
        let mut card = json!({
            "at": "2026-10-16T12:00:25.000Z", "session": "sess1", "run": "run2", "seq": 6,
            "component": {"type": "approval", "id": "card", "title": "Refunds' Card",
                          "text": "Approve the Card Refunds works under.", "draft": "drf_s1",
                          "waits": "owner"},
            "key": "sess1#card"
        });
        if let Some(a) = answer {
            card["answer"] =
                json!({"state": "answered", "words": a, "at": "2026-10-16T12:00:40.000Z"});
        }
        let mut c = crate::coach_send::Conversation::default();
        c.took(read(&json!({"messages": [], "components": [card]})), now());
        c
    }

    fn approval_of(c: &crate::coach_send::Conversation) -> ApprovalLines {
        let l = conversation_lines(&book(), c, &[], Mode::Kiff, now()).unwrap();
        l.lines
            .into_iter()
            .find_map(|l| match l {
                Line::Card {
                    card: Card::Approval(a),
                    ..
                } => Some(a),
                _ => None,
            })
            .unwrap()
    }

    fn draft(status: &str) -> AgentDraft {
        AgentDraft::read(
            json!({"id": "drf_s1", "agent_id": "refunds-x", "status": status, "version": "v1",
                   "label": "Refunds", "cards": [{"lines": ["Refund a payment: up to €5 each"]}]})
            .to_string()
            .as_bytes(),
        )
        .unwrap()
    }

    #[test]
    fn an_approvals_draft_is_read_said_or_drawn() {
        let mut c = approval_thread(None);
        assert_eq!(c.drafts_to_read(), ["drf_s1"]);
        // Not read yet: said, never a blank.
        assert_eq!(
            approval_of(&c).draft,
            Some(DraftShown::Reading(
                "Reading KIFF's words for this connection…".into()
            ))
        );
        // KIFF wouldn't give it: its reason, with Try again.
        c.took_draft("drf_s1", Err("Not found.".into()));
        assert_eq!(
            approval_of(&c).draft,
            Some(DraftShown::NotRead {
                id: "drf_s1".into(),
                why: "KIFF's draft of this Card couldn't be read. Not found.".into(),
                try_again: "Try again".into(),
            })
        );
        // Read: its title and KIFF's lines, and the error gone.
        c.took_draft("drf_s1", Ok(draft("draft")));
        assert!(c.draft_errors.is_empty());
        let a = approval_of(&c);
        assert_eq!(
            a.draft,
            Some(DraftShown::Card {
                title: Some("Refunds' Card".into()),
                next_words: None,
                lines: vec!["Refund a payment: up to €5 each".into()],
            })
        );
        assert!(a.gate.is_some(), "Approve stays the phone's");
        // A draft read before stays drawn when a later read fails.
        c.took_draft("drf_s1", Err("Offline.".into()));
        assert!(matches!(
            approval_of(&c).draft,
            Some(DraftShown::Card { .. })
        ));
        // Confirmed, it isn't read again.
        c.took_draft("drf_s1", Ok(draft("confirmed")));
        assert!(c.drafts_to_read().is_empty());
    }

    #[test]
    fn an_approved_card_names_the_card_it_signed() {
        let mut c = approval_thread(Some("approved"));
        // Coach's title for the card, while the draft isn't read.
        let a = approval_of(&c);
        assert_eq!(a.folded.unwrap().text, "Approved · Refunds' Card");
        assert!(a.signed_lines.is_empty());
        c.took_draft("drf_s1", Ok(draft("claimed")));
        let a = approval_of(&c);
        assert_eq!(a.folded.unwrap().text, "Approved · Refunds' Card");
        assert_eq!(a.signed_lines, ["Refund a payment: up to €5 each"]);
        // Not signed yet at KIFF: no lines under the line.
        c.took_draft("drf_s1", Ok(draft("draft")));
        assert!(approval_of(&c).signed_lines.is_empty());
    }

    #[test]
    fn a_card_marked_next_draws_under_tafs_clouds_words() {
        let mut c = approval_thread(None);
        let mut t = c.thread.clone();
        t.components[0].component.signs = Some(crate::coach::Signs {
            card: Some(false),
            words: Some("Next, you approve Refunds' Card.".into()),
            takes: None,
        });
        c.took(t.clone(), now());
        c.took_draft("drf_s1", Ok(draft("draft")));
        assert_eq!(
            approval_of(&c).draft,
            Some(DraftShown::Card {
                title: None,
                next_words: Some("Next, you approve Refunds' Card.".into()),
                lines: vec!["Refund a payment: up to €5 each".into()],
            })
        );
        // Marked next with no words, it isn't drawn.
        t.components[0].component.signs.as_mut().unwrap().words = None;
        c.took(t, now());
        assert_eq!(approval_of(&c).draft, None);
    }

    #[test]
    fn a_drives_end_says_what_it_did_and_why_it_ended() {
        let w = book();
        let recap = |drive: serde_json::Value| {
            let t = read(&json!({"messages": [], "items": [{"type": "drive", "drive": drive}]}));
            let l = lines(&w, &t, &[], Mode::Kiff, now()).unwrap();
            l.lines
                .into_iter()
                .find(|l| matches!(l, Line::Drive { .. }))
                .unwrap()
        };
        assert_eq!(
            recap(
                json!({"id": "d1", "task": "Open Connect apps", "why": "stopped",
                "at": "2026-10-16T12:02:00Z", "steps": [
                    {"at": "2026-10-16T12:01:00Z", "line": "Tapped Connect apps",
                     "touched": {"words": "Connect apps", "taf": "Refunds"}},
                    {"at": "2026-10-16T12:01:10Z", "line": "Scrolled down"}]})
            ),
            Line::Drive {
                eyebrow: "What I did".into(),
                task: "Open Connect apps".into(),
                steps: vec![
                    "Tapped Connect apps · Connect apps · Refunds".into(),
                    "Scrolled down".into()
                ],
                none: None,
                ended: "You stopped Coach".into(),
            }
        );
        // tafs cloud's words as a caption; a step it can't read leaves the steps out.
        let Line::Drive {
            steps, none, ended, ..
        } = recap(
            json!({"id": "d2", "task": "Look", "why": "halted", "words": " the owner stopped it",
            "at": "2026-10-16T12:02:00Z", "steps": [{"line": "no time"}]}),
        )
        else {
            unreachable!()
        };
        assert!(steps.is_empty());
        assert_eq!(none.as_deref(), Some("No step was done."));
        assert_eq!(ended, "The owner stopped it");
        // A reason it doesn't know.
        let Line::Drive { ended, .. } = recap(
            json!({"id": "d3", "task": "Look", "why": "meteor", "at": "2026-10-16T12:02:00Z"}),
        ) else {
            unreachable!()
        };
        assert_eq!(ended, "Coach stopped");
    }

    #[test]
    fn the_contracts_thread_draws_as_the_ipad_draws_it() {
        let w = book();
        let t = read(&contract());
        let l = lines(&w, &t, &[], Mode::Kiff, now()).unwrap();
        assert_eq!(l.title, "Coach");
        assert_eq!(l.plan, None);
        assert_eq!(l.starts, None);
        assert_eq!(l.thinking.as_deref(), Some("Answering"));
        assert_eq!(l.lines[0], Line::Gap("12:00".into()));
        assert_eq!(
            l.lines[1],
            Line::Owner {
                text: "Set up refunds for my shop".into(),
                answered: None,
                files: vec![]
            }
        );
        let Line::Coach { work, report, .. } = &l.lines[2] else {
            panic!("{:?}", l.lines[2])
        };
        // Coach's answer can be reported, by tafs cloud's id for it and its chat.
        assert_eq!(
            report,
            &Some(crate::report::Reported {
                message: "run2.3".into(),
                session: "sess1".into()
            })
        );
        let work = work.as_ref().unwrap();
        assert_eq!(work.summary, "Worked for 1 min 1 s");
        assert_eq!(work.steps[0].words, "Working on the connection");
        let Line::Card {
            card:
                Card::Questions {
                    eyebrow,
                    count,
                    questions,
                    ..
                },
            ..
        } = &l.lines[3]
        else {
            panic!("{:?}", l.lines[3])
        };
        assert_eq!(eyebrow, "Refunds' limits");
        assert_eq!(count, "0 of 1 answered");
        assert_eq!(questions[0].title, "1. Most for one refund?");
        assert_eq!(
            l.lines[4],
            Line::Owner {
                text: "Up to €5 each, €50 a day".into(),
                answered: Some("Answered the card above".into()),
                files: vec![]
            }
        );
        let Line::Card {
            card: Card::Approval(a),
            ..
        } = &l.lines[5]
        else {
            panic!("{:?}", l.lines[5])
        };
        assert_eq!(a.folded.as_ref().unwrap().text, "Approved · Connect Stripe");
        assert_eq!(a.wider, None);
        assert_eq!(a.gate, None, "answered: no buttons to stand in for");
        let Line::Card {
            card: Card::Result(r),
            ..
        } = &l.lines[6]
        else {
            panic!("{:?}", l.lines[6])
        };
        assert_eq!(r.eyebrow, "Set up · Refunds");
        assert_eq!(r.detail.as_deref(), Some("list requests k3f9"));
        assert_eq!(r.lines, [("Setting up Refunds…".to_string(), Tone::Dim)]);
        assert!(matches!(&l.lines[7], Line::Coach { text, .. } if text == "Refunds is set up."));
        assert_eq!(l.coach_lines, ["Rae: Refunded $20 to order 42."]);
        // Listed, its button opens it.
        let listed = [Listed {
            agent: "refunds-x".into(),
            key: "taf-1".into(),
            name: "Refunds".into(),
        }];
        let l = lines(&w, &t, &listed, Mode::Kiff, now()).unwrap();
        let Line::Card {
            card: Card::Result(r),
            ..
        } = &l.lines[6]
        else {
            panic!()
        };
        assert_eq!(r.open, Some(("See it on Refunds".into(), "taf-1".into())));
    }

    #[test]
    fn an_improvement_made_opens_its_tafs_versions_where_it_is_listed() {
        let w = book();
        let mut v = contract();
        for c in v["components"].as_array_mut().unwrap() {
            if c["component"]["type"] == "result" {
                c["component"]["taf"]["improves"] = json!("taf-1");
                c["answer"]["outcome"] = json!({"version": 3});
            }
        }
        let t = read(&v);
        let result = |listed: &[Listed]| {
            let l = lines(&w, &t, listed, Mode::Kiff, now()).unwrap();
            l.lines
                .into_iter()
                .find_map(|line| match line {
                    Line::Card {
                        card: Card::Result(r),
                        ..
                    } => Some(r),
                    _ => None,
                })
                .unwrap()
        };
        // Listed by its id at tafs cloud: See version, opening that taf's Versions.
        let listed = [Listed {
            agent: "refunds-x".into(),
            key: "taf-1".into(),
            name: "Refunds".into(),
        }];
        let r = result(&listed);
        assert_eq!(
            r.see_version,
            Some(("See version 3".into(), "taf-1".into()))
        );
        assert!(r.lines.is_empty());
        // Not listed here: that it's made, in words.
        let r = result(&[]);
        assert_eq!(r.see_version, None);
        assert_eq!(r.lines.len(), 1);
    }

    #[test]
    fn a_waiting_approval_draws_its_wider_line_above_the_cant_approve_line() {
        let w = book();
        let mut v = contract();
        let words =
            "The sign-in for Google Sheets can't be narrowed to this job: it opens every sheet.";
        v["components"][1]["component"]["wider"] = json!({ "words": words });
        v["components"][1].as_object_mut().unwrap().remove("answer");
        let t = read(&v);
        let l = lines(&w, &t, &[], Mode::Kiff, now()).unwrap();
        let a = l
            .lines
            .iter()
            .find_map(|l| match l {
                Line::Card {
                    card: Card::Approval(a),
                    ..
                } => Some(a),
                _ => None,
            })
            .unwrap();
        assert_eq!(a.folded, None);
        assert_eq!(a.eyebrow, "Needs you · approve");
        assert_eq!(a.title.as_deref(), Some("Connect Stripe"));
        assert_eq!(
            a.text.as_deref(),
            Some("Approve to connect Stripe for Refunds.")
        );
        assert_eq!(a.wider.as_deref(), Some(words));
        assert!(a.gate.is_some());
        // A sign-in carries it too.
        let mut v = contract();
        v["components"][1]["component"] = json!({
            "type": "sign_in", "id": "sheets", "service": "Google Sheets", "waits": "owner",
            "wider": { "words": words }
        });
        v["components"][1].as_object_mut().unwrap().remove("answer");
        let t = read(&v);
        let l = lines(&w, &t, &[], Mode::Kiff, now()).unwrap();
        let s = l
            .lines
            .iter()
            .find_map(|l| match l {
                Line::Card {
                    card: Card::SignIn(s),
                    ..
                } => Some(s),
                _ => None,
            })
            .unwrap();
        assert_eq!(s.title, "Sign in at Google Sheets");
        assert_eq!(s.wider.as_deref(), Some(words));
        assert!(s.gate.is_some());
    }

    #[test]
    fn owner_answers_read_as_their_choice() {
        let w = book();
        let t = read(&contract());
        assert_eq!(
            owner_words(&w, &t, "[approval connect-stripe] approved").unwrap(),
            "Approve"
        );
        assert_eq!(
            owner_words(&w, &t, "[sign_in x] not now").unwrap(),
            "Not now"
        );
        assert_eq!(
            owner_words(
                &w,
                &t,
                r#"[questions limits] {"per_refund": {"value": "50"}}"#
            )
            .unwrap(),
            "€50"
        );
        assert_eq!(
            owner_words(
                &w,
                &t,
                r#"[questions limits] {"per_refund": {"own": "40"}}"#
            )
            .unwrap(),
            "In your words: “40”"
        );
        assert_eq!(owner_words(&w, &t, "hello").unwrap(), "hello");
    }

    #[test]
    fn coach_writes_markdown_and_it_draws_as_formatting() {
        assert_eq!(
            markup("## Next\n- **Stripe** & `list_requests`\n- see [the docs](https://x) <now>"),
            "<b>Next</b>\n• <b>Stripe</b> &amp; <tt>list_requests</tt>\n• see the docs &lt;now&gt;"
        );
        assert_eq!(markup("a * b * c"), "a * b * c");
        assert_eq!(markup("*one*"), "<i>one</i>");
    }
}
