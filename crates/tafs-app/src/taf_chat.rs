//! A taf's chat on its page's Chat tab (`TafChatView`), for a taf tafs cloud runs, signed in with
//! KIFF: its thread as tafs cloud reads it, drawn as the iPad draws it, with the message box. The
//! owner writes to the taf as the session, unsigned: a message into its live chat, or, while it
//! works, words to its run; Stop cancels the run. An answer is drawn as it streams; while nothing
//! streams and the taf owes an answer or works, the thread is read again, backing off
//! (`followChat`'s plain reads). What the chat
//! holds is kept per taf while the app runs, so the page drawn again keeps it. The lines are
//! `tafs-core/src/taf_chat.rs`.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;
use tafs_core::coach_send::{self, Delivery, Refusal, Steered, Submit};
use tafs_core::needs_you::CardLines;
use tafs_core::taf_chat::{
    self, ChatLines, Line, Polls, SendRefused, TafChat, TafThread, UnsentLines, Whose,
};
use tafs_core::words::WordBook;

use crate::chat::{owner, Composer};
use crate::widgets::{outline, panel, symbol, text};

/// Where a taf's chat is read from, and where what's written goes.
#[derive(Clone)]
pub enum ChatSource {
    /// tafs cloud, as the session. `signed_out` is called when tafs cloud says the session is over.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
        signed_out: Rc<dyn Fn()>,
    },
    /// A thread as tafs cloud answers it, from a file (`--chat`): nothing is read again or sent.
    /// `report`: Report this answer opens over its last answer that can be reported (`--report`).
    File {
        thread: std::path::PathBuf,
        report: bool,
        /// A stream's events (`--chat-stream`): an answer as it streams, after the thread.
        stream: Option<std::path::PathBuf>,
    },
    /// The sample's chat (`sampleReply`): nothing is read or sent. `now_line`: what the taf is
    /// doing, for its hello; `done`, its Record lines' titles, newest first; `waiting`, its calls
    /// waiting on the owner; `beat_ms`, half of how long it takes to answer (0: at once).
    Sample {
        now_line: String,
        done: Vec<String>,
        waiting: usize,
        beat_ms: u64,
    },
    /// No chat here: a taf tafs cloud doesn't run.
    None,
}

impl ChatSource {
    pub fn is_none(&self) -> bool {
        matches!(self, ChatSource::None)
    }
}

/// The taf the chat is with.
pub struct ChatTaf {
    /// Its id at tafs cloud.
    pub id: String,
    pub name: String,
    /// May use Coach and manages it: its Ask Coach cards offer Ask Coach.
    pub ask_coach: bool,
    /// Its held calls' cards, as Needs you draws them, placed where they were held.
    pub held: Vec<CardLines>,
    /// Its open trial's own chat (`TrialScreen`), not the taf's: read and sent at the trial's
    /// paths, kept apart, and with no files.
    pub trial: bool,
    /// How its held calls are read again, from KIFF, for a tafs cloud taf.
    pub reread: Option<HeldReread>,
    /// Its Record lines as the page had them (`entries(for:)`): what became of its calls, until
    /// KIFF's activity is read again here.
    pub record: Vec<tafs_core::record::Entry>,
    /// Where its doors are read, where tafs cloud offers doors (`doorsOffered`): their knocks sit
    /// among the messages. None for a trial's chat.
    pub doors: crate::doors::DoorsSource,
}

/// What reading a taf's held calls again takes (`refresh`'s, for this taf): its KIFF agent, and
/// what its cards show of it.
#[derive(Clone)]
pub struct HeldReread {
    pub agent: String,
    pub key: String,
    pub name: String,
    pub initial: String,
    pub color: i64,
    /// May answer held calls (`canAnswerHolds`).
    pub may_answer: bool,
    /// Manages Cards (`canManageCards`): Extra room on a call over a total.
    pub manages_cards: bool,
}

impl ChatTaf {
    /// What the app keeps its chat and draft under: the trial's apart from the taf's.
    fn key(&self) -> String {
        if self.trial {
            format!("trial:{}", self.id)
        } else {
            self.id.clone()
        }
    }
}

/// When a taf's chat, as kept while the app runs, last had a message: Home's order counts it as
/// the taf being used (`byRecent`'s `chats`). None for a chat not opened since the app started.
pub fn last_message_at(key: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    CHATS.with(|c| {
        c.borrow()
            .get(key)
            .and_then(|chat| chat.borrow().thread.messages.iter().map(|m| m.at).max())
    })
}

/// Whether a sample taf is paused now (`state == .paused`), for its Setup's Pause or Resume.
pub fn sample_paused_now(key: &str) -> bool {
    sample_paused(key) == Some(true)
}

/// What hears a sample taf's pause: true while it's still there to hear it.
type PauseSeen = Box<dyn Fn() -> bool>;

/// `seen` hears each pause or resume of a sample taf, until it answers false.
pub fn on_sample_pause_seen(seen: impl Fn() -> bool + 'static) {
    SAMPLE_PAUSE_SEEN.with(|s| s.borrow_mut().push(Box::new(seen)));
}

/// A sample taf paused or set back to work (its chat, its Setup): its page's state draws it again
/// at once; one that answers false is gone and is dropped.
fn sample_pause_seen() {
    let seen = SAMPLE_PAUSE_SEEN.with(|s| std::mem::take(&mut *s.borrow_mut()));
    let kept: Vec<PauseSeen> = seen.into_iter().filter(|f| f()).collect();
    SAMPLE_PAUSE_SEEN.with(|s| {
        let mut s = s.borrow_mut();
        let added = std::mem::take(&mut *s);
        *s = kept;
        s.extend(added);
    });
}

/// `togglePause` on a sample taf from its Setup: paused, or back to work. The sidebar's row says
/// so at once, as a pause from its chat does (`shell::reread_home`).
pub fn toggle_sample_pause(key: &str) -> bool {
    let paused = !sample_paused_now(key);
    SAMPLE_PAUSED.with(|s| s.borrow_mut().insert(key.to_string(), paused));
    crate::shell::reread_home();
    sample_pause_seen();
    paused
}

thread_local! {
    /// Each sample taf the owner paused (true) or told to carry on (false) in its chat, by its
    /// key: kept while the app runs, as the sample keeps no chats either.
    static SAMPLE_PAUSED: RefCell<HashMap<String, bool>> = RefCell::new(HashMap::new());
    /// What else shows a sample taf's pause (its page's state chip), each kept while it's there:
    /// one that answers false is gone and is dropped.
    static SAMPLE_PAUSE_SEEN: RefCell<Vec<PauseSeen>> = RefCell::default();
    /// Each taf's chat as the app holds it (`chats`, `chatStates`), by its id at tafs cloud: kept
    /// while the app runs, so its page drawn again keeps what was sent and not sent.
    static CHATS: RefCell<HashMap<String, Rc<RefCell<TafChat>>>> = RefCell::new(HashMap::new());
    /// What the owner is typing to each taf, not sent yet.
    static DRAFTS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
    /// Each taf's chat page while it's open, to draw again when something changes it from
    /// elsewhere (an approval that landed after its run ended).
    static OPEN: RefCell<HashMap<String, std::rc::Weak<Page>>> = RefCell::new(HashMap::new());
}

/// An approval given on a held card landed after its run had ended (`runEndedAfterApproval`):
/// tafs cloud's words in the taf's chat, with Retry on the owner's last message, drawn again if
/// it's open. A chat not opened yet reads its thread when it opens, as before.
pub fn run_ended_after_approval(taf: &str, words: &str) {
    let Some(chat) = CHATS.with(|c| c.borrow().get(taf).cloned()) else {
        return;
    };
    if !chat.borrow_mut().run_ended_after_approval(words) {
        return;
    }
    if let Some(page) = OPEN.with(|o| o.borrow().get(taf).and_then(|p| p.upgrade())) {
        page.draw();
    }
}

/// A sample taf as its chat left it: paused (true), told to carry on (false), or as it was.
pub fn sample_paused(key: &str) -> Option<bool> {
    SAMPLE_PAUSED.with(|s| s.borrow().get(key).copied())
}

/// Signed out: nothing written for the last account stays.
pub fn forget() {
    SAMPLE_PAUSED.with(|s| s.borrow_mut().clear());
    SAMPLE_PAUSE_SEEN.with(|s| s.borrow_mut().clear());
    CHATS.with(|c| c.borrow_mut().clear());
    DRAFTS.with(|d| d.borrow_mut().clear());
    OPEN.with(|o| o.borrow_mut().clear());
}

/// The pane and what it holds while it's open.
/// What hears whether a drawn chat holds anything (`on_drawn`).
type Drawn = Rc<dyn Fn(bool)>;

struct Page {
    /// Told, each time the chat is drawn, whether it holds anything yet (`chatItems` not empty):
    /// the trial's sheet shows its guide before the first message only.
    on_drawn: RefCell<Option<Drawn>>,
    /// How much of the answer streaming now shows (`SteadyMarkdown`).
    live_shown: crate::chat::Shown,
    w: Rc<WordBook>,
    taf: ChatTaf,
    /// Its held calls' cards: as the page opened, then as KIFF was read again here.
    held: RefCell<Vec<CardLines>>,
    source: ChatSource,
    chat: Rc<RefCell<TafChat>>,
    drawn: RefCell<Option<ChatLines>>,
    reading: Cell<bool>,
    polls: RefCell<Polls>,
    /// The thread goes to its latest at the next draw.
    follow: Cell<bool>,
    root: gtk::Box,
    column: gtk::Box,
    scroller: gtk::ScrolledWindow,
    /// ↓ and the Needs you pill over the thread.
    jump: Rc<crate::chat::Jump>,
    error: gtk::Label,
    composer: Composer,
    /// The files picked for the next message (`chatFiles[taf]`).
    attach: Rc<crate::attach::Attach>,
    /// Its doors (`doors(for:)`), shared with its Activity and its doors sheet, and how many
    /// messages the thread had when they were last read: they're read again when that changes.
    doors: Option<(crate::doors::DoorsModel, Cell<Option<usize>>)>,
}

/// The Chat tab's pane: the thread, the line above the box, and the box.
pub fn pane(w: Rc<WordBook>, taf: ChatTaf, source: ChatSource) -> gtk::Widget {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_vexpand(true);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    column.set_valign(gtk::Align::Start);
    column.set_widget_name("chat-thread");
    // A readable column on a wide window: the owner's words and the taf's sit together.
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(720);
    clamp.set_tightening_threshold(720);
    clamp.set_child(Some(&column));
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&clamp));
    let jump = crate::chat::Jump::new(&w, &scroller);
    root.append(&jump.overlay);
    let error = text("", 13, 400, &["tafs-no"]);
    error.set_margin_start(16);
    error.set_margin_end(16);
    error.set_margin_bottom(4);
    error.set_visible(false);
    root.append(&error);
    let composer = crate::chat::composer(crate::chat::Whose::Taf);
    let bar_clamp = adw::Clamp::new();
    bar_clamp.set_maximum_size(720);
    bar_clamp.set_tightening_threshold(720);
    let field = composer.bar.first_child().expect("the composer's field");
    composer.bar.remove(&field);
    bar_clamp.set_child(Some(&field));
    composer.bar.append(&bar_clamp);
    root.append(&composer.bar);
    // The taf's files go to it at tafs cloud (`filesTarget`); a screenshot's go nowhere.
    let attach = crate::attach::Attach::new(
        w.clone(),
        match &source {
            ChatSource::Cloud {
                token, languages, ..
            } => Some(crate::attach::Target {
                token: token.clone(),
                languages: languages.clone(),
                taf: taf.id.clone(),
            }),
            _ => None,
        },
    );
    attach.place(&composer);
    // No files in the sample's chat (`filesTarget` is the session's).
    attach.button.set_visible(
        !source.is_none() && !taf.trial && !matches!(source, ChatSource::Sample { .. }),
    );

    let chat = CHATS.with(|c| {
        c.borrow_mut()
            .entry(taf.key())
            .or_insert_with(|| Rc::new(RefCell::new(TafChat::new(&taf.id))))
            .clone()
    });
    // What became of its calls, as the page had them: from the same read as its held cards, so a
    // card answered since shows what became of it (a trial's chat has none).
    if !taf.trial {
        chat.borrow_mut().record = taf.record.clone();
    }
    let doors = match (&taf.doors, taf.trial) {
        (crate::doors::DoorsSource::None, _) | (_, true) => None,
        (source, false) => Some((
            crate::doors::shared(w.clone(), taf.id.clone(), source.clone()),
            Cell::new(None),
        )),
    };
    let page = Rc::new(Page {
        live_shown: crate::chat::Shown::default(),
        w,
        doors,
        held: RefCell::new(taf.held.clone()),
        taf,
        source,
        chat,
        drawn: RefCell::new(None),
        reading: Cell::new(false),
        polls: RefCell::new(taf_chat::polls()),
        follow: Cell::new(true),
        on_drawn: RefCell::new(None),
        root: root.clone(),
        column,
        scroller,
        jump,
        error,
        composer,
        attach,
    });
    wire_composer(&page);
    OPEN.with(|o| {
        let mut o = o.borrow_mut();
        o.retain(|_, p| p.strong_count() > 0);
        o.insert(page.taf.key(), Rc::downgrade(&page));
    });
    // No doors offered (or a trial's chat): no knocks kept from an earlier read.
    if page.doors.is_none() {
        page.chat.borrow_mut().knocks.clear();
    }
    // Its doors' open knocks, as they're read or answered: drawn again among the messages.
    if let Some((model, _)) = &page.doors {
        let p = Rc::downgrade(&page);
        let listening = model.listen(Rc::new(move || {
            if let Some(p) = p.upgrade() {
                let knocks = p.doors.as_ref().map(|(m, _)| m.open_knocks());
                p.chat.borrow_mut().knocks = knocks.unwrap_or_default();
                // Busy or not, the cards' buttons change with it.
                p.drawn.take();
                if p.open() {
                    p.draw();
                }
            }
        }));
        page.chat.borrow_mut().knocks = model.open_knocks();
        let model = model.clone();
        page.root
            .connect_destroy(move |_| model.unlisten(listening));
    }
    {
        let p = Rc::downgrade(&page);
        page.attach.on_change(Rc::new(move || {
            if let Some(p) = p.upgrade() {
                p.composer_state();
            }
        }));
    }
    match page.source.clone() {
        ChatSource::File {
            thread: path,
            report,
            stream,
        } => {
            let read = std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok())
                .and_then(|b| TafThread::read(&b));
            match read {
                Some(t) => {
                    // A screenshot's chat starts from its file alone, with the page's Record.
                    let mut fresh = TafChat::new(&page.taf.id);
                    fresh.record = page.taf.record.clone();
                    *page.chat.borrow_mut() = fresh;
                    page.chat.borrow_mut().took(&page.w, t, chrono::Utc::now());
                    // Kept while it's open, as a read's loop keeps a signed-in chat: what's read
                    // after it's drawn (its doors) still reaches it.
                    let p = page.clone();
                    glib::spawn_future_local(async move {
                        glib::timeout_future(std::time::Duration::from_secs(1)).await;
                        while p.open() {
                            glib::timeout_future(std::time::Duration::from_secs(1)).await;
                        }
                    });
                    match stream {
                        Some(stream) if crate::chat::PIN_CHECK.with(|c| c.get()) => {
                            page.pin_check(&stream)
                        }
                        Some(stream) => page.stream_from(&stream),
                        None => {}
                    }
                }
                None => {
                    eprintln!(
                        "{} isn't a taf's thread as tafs cloud answers it",
                        path.display()
                    );
                    std::process::exit(2);
                }
            }
            page.draw();
            if report {
                let last = page
                    .chat
                    .borrow()
                    .thread
                    .messages
                    .iter()
                    .rev()
                    .find_map(tafs_core::report::reportable);
                if let Some(r) = last {
                    let (w, taf, at) = (page.w.clone(), page.taf.id.clone(), page.root.clone());
                    glib::idle_add_local_once(move || {
                        crate::report_sheet::open(
                            w,
                            at.upcast_ref(),
                            taf,
                            r,
                            crate::report_sheet::ReportTo::Nowhere,
                        )
                    });
                }
            }
        }
        ChatSource::Cloud { .. } => {
            page.draw();
            let p = page.clone();
            glib::spawn_future_local(async move {
                p.load().await;
                p.resume().await;
                p.follow_chat().await;
            });
        }
        // The sample's: its hello, and Support replies' question answered, once.
        ChatSource::Sample {
            now_line,
            done,
            waiting,
            ..
        } => {
            if !page.chat.borrow().loaded {
                if let Ok(o) = tafs_core::sample_chat::opening(
                    &page.w,
                    &page.taf.id,
                    &page.taf.name,
                    &now_line,
                    &done,
                    waiting,
                    chrono::Utc::now(),
                ) {
                    page.chat.borrow_mut().sample_seed(o);
                }
            }
            page.draw();
            // Kept while it's open, as a read's loop keeps a signed-in chat: nothing else holds
            // it, and its box and answers need it.
            let p = page.clone();
            glib::spawn_future_local(async move {
                glib::timeout_future(std::time::Duration::from_secs(1)).await;
                while p.open() {
                    glib::timeout_future(std::time::Duration::from_secs(1)).await;
                }
            });
        }
        ChatSource::None => page.draw(),
    }
    // SAFETY: only ever read back as the type it's stored as, by `on_drawn`.
    unsafe { root.set_data("taf-chat", page.clone()) };
    root.upcast()
}

/// `told` hears, at once and each time the chat `pane` is drawn, whether it holds anything yet.
pub fn on_drawn(pane: &gtk::Widget, told: Drawn) {
    // SAFETY: stored by `pane` as an `Rc<Page>`, and read back only as that.
    let page = unsafe {
        pane.data::<Rc<Page>>("taf-chat")
            .map(|p| p.as_ref().clone())
    };
    if let Some(page) = page {
        *page.on_drawn.borrow_mut() = Some(told);
        page.draw();
    }
}

fn wire_composer(page: &Rc<Page>) {
    let c = &page.composer;
    let buffer = c.view.buffer();
    let id = page.taf.key();
    DRAFTS.with(|d| buffer.set_text(d.borrow().get(&id).map_or("", String::as_str)));
    {
        let p = Rc::downgrade(page);
        buffer.connect_changed(move |b| {
            let words = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
            DRAFTS.with(|d| d.borrow_mut().insert(id.clone(), words));
            if let Some(p) = p.upgrade() {
                p.composer_state();
            }
        });
    }
    {
        let p = Rc::downgrade(page);
        c.send.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.send();
            }
        });
    }
    {
        let p = Rc::downgrade(page);
        c.stop.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.stop();
            }
        });
    }
    let (p, q) = (Rc::downgrade(page), Rc::downgrade(page));
    c.keys(
        move || {
            if let Some(p) = p.upgrade() {
                p.send();
            }
        },
        move || {
            if let Some(q) = q.upgrade() {
                q.stop();
            }
        },
    );
}

impl Page {
    fn signed_out(&self) {
        if let ChatSource::Cloud { signed_out, .. } = &self.source {
            signed_out();
        }
    }

    /// Still on screen: the reads go on only while the pane is in the window.
    fn open(&self) -> bool {
        self.root.root().is_some()
    }

    fn sends(&self) -> bool {
        matches!(
            self.source,
            ChatSource::Cloud { .. } | ChatSource::Sample { .. }
        )
    }

    /// The sample's answer (`deliverChatOnce`'s sample branch, kiff/tafs#285): worked out as
    /// it's sent, so a pause takes effect at once; then written as a run would stream it, what
    /// it's thinking, its notes read, its words; then kept, with what it worked on. Left mid-way
    /// (the chat closed), it's given up and the message stays.
    async fn sample_reply(self: &Rc<Self>, d: Delivery) {
        let ChatSource::Sample {
            done,
            waiting,
            beat_ms,
            ..
        } = self.source.clone()
        else {
            return;
        };
        let Ok((answer, effect)) = tafs_core::sample_chat::reply(&self.w, &d.text, &done, waiting)
        else {
            self.chat.borrow_mut().sample_gone();
            return;
        };
        let paused = match effect {
            tafs_core::sample_chat::Effect::Pause => Some(true),
            tafs_core::sample_chat::Effect::Resume => {
                sample_paused(&self.taf.id).filter(|p| *p).map(|_| false)
            }
            tafs_core::sample_chat::Effect::None => None,
        };
        if let Some(p) = paused {
            SAMPLE_PAUSED.with(|s| s.borrow_mut().insert(self.taf.id.clone(), p));
            // The sidebar's row and the taf's page say so at once.
            crate::shell::reread_home();
            sample_pause_seen();
        }
        let started = std::time::Instant::now();
        self.chat.borrow_mut().sample_live(chrono::Utc::now());
        self.draw();
        let script = tafs_core::sample_chat::script(&self.w, &answer, beat_ms).unwrap_or_default();
        let mut f = taf_chat::Following::default();
        for (ms, e) in &script {
            glib::timeout_future(std::time::Duration::from_millis(*ms)).await;
            if !self.open() {
                self.chat.borrow_mut().sample_gone();
                return;
            }
            let _ = self
                .chat
                .borrow_mut()
                .stream_event(&self.w, e, None, &mut f);
            self.draw();
        }
        let work = (beat_ms > 0)
            .then(|| tafs_core::sample_chat::work(started.elapsed().as_millis() as i64));
        self.chat
            .borrow_mut()
            .sample_answered(&d, &answer, work, chrono::Utc::now());
        self.draw();
    }

    /// `loadChat`: the thread from tafs cloud, with what was sent from here that it doesn't have.
    async fn load(self: &Rc<Self>) {
        let ChatSource::Cloud {
            token, languages, ..
        } = self.source.clone()
        else {
            return;
        };
        if self.reading.replace(true) {
            return;
        }
        let path = if self.taf.trial {
            tafs_core::versions::trial_conversation_path(&self.taf.id)
        } else {
            taf_chat::conversation_path(&self.taf.id)
        };
        let words = (*self.w).clone();
        let read = gtk::gio::spawn_blocking(move || {
            let a = crate::kiff::cloud(&token, &languages, &path)
                .map_err(|e| Some(e.words(&words).unwrap_or_default()))?;
            if a.status == 401 {
                return Err(None);
            }
            if !(200..300).contains(&a.status) {
                return Err(Some(
                    tafs_core::signin::refusal(&a, Some(&words))
                        .words(&words)
                        .unwrap_or_default(),
                ));
            }
            TafThread::read(&a.body).ok_or_else(|| {
                Some(tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
            })
        })
        .await
        .unwrap_or_else(|_| Err(Some(String::new())));
        self.reading.set(false);
        // A run seen at work here, over now: what it did reaches the record (`loadChat`'s
        // `refresh`).
        let run_over = matches!(&read, Ok(t) if t.working.is_none())
            && self.chat.borrow().thread.working.is_some();
        match read {
            Err(None) => return self.signed_out(),
            Ok(t) => self.chat.borrow_mut().took(&self.w, t, chrono::Utc::now()),
            Err(Some(why)) => {
                self.chat.borrow_mut().error = Some(
                    self.w
                        .get("chat.thread_not_loaded", &[("why", &why)])
                        .unwrap_or(why),
                )
            }
        }
        if self.open() {
            self.draw();
        }
        if run_over {
            crate::shell::reread_home();
            self.reread_held().await;
            if self.open() {
                self.draw();
            }
        }
    }

    /// `followChat`: while the taf owes an answer or works and nothing streams, the thread read
    /// again as `follow` says: 2.5 s after the owner wrote, then further apart while nothing
    /// changes (5, 10, 30 s, each ±20%), for two minutes. Nothing is read while the window isn't
    /// seen, and it's read at once when it is again. The timer is local; a read never overlaps
    /// another; leaving the chat ends it.
    async fn follow_chat(self: &Rc<Self>) {
        let mut hidden = false;
        loop {
            glib::timeout_future(tafs_core::follow::TICK).await;
            if !self.open() {
                return;
            }
            if !crate::chat::seen(self.root.upcast_ref()) {
                hidden = true;
                continue;
            }
            if std::mem::take(&mut hidden) {
                self.polls.borrow_mut().seen_again();
            }
            // A stream shows what the taf does, and reads the thread itself.
            if self.reading.get() || self.chat.borrow().live.is_some() {
                continue;
            }
            self.resume().await;
            if self.chat.borrow().live.is_some() {
                continue;
            }
            let Some(owner) = self.chat.borrow().to_follow() else {
                continue;
            };
            let due = self.polls.borrow_mut().tick(
                tafs_core::follow::TICK,
                Some(owner),
                glib::random_double(),
            );
            if due {
                self.load().await;
                let thread = self.chat.borrow().thread.clone();
                self.polls.borrow_mut().read(&thread, glib::random_double());
            }
        }
    }

    async fn post(&self, path: String, body: Vec<u8>) -> Option<crate::chat::Sent> {
        let ChatSource::Cloud {
            token, languages, ..
        } = &self.source
        else {
            return None;
        };
        Some(crate::chat::post(&self.w, token, languages, path, body).await)
    }

    /// The AI-data consent's screen, for a turn tafs cloud refused for want of it.
    fn consent(&self, refusal: &[u8]) {
        let (
            ChatSource::Cloud {
                token, languages, ..
            },
            Some(window),
        ) = (&self.source, self.root.root())
        else {
            return;
        };
        crate::ai_consent::required(
            self.w.clone(),
            crate::ai_consent::Source::Cloud {
                token: token.clone(),
                languages: languages.clone(),
            },
            refusal,
            window.upcast(),
        );
    }

    /// Send: what's in the box.
    fn send(self: &Rc<Self>) {
        let words = self.composer.draft();
        // A message with files goes as a message of its own.
        let taken = self.attach.take();
        if !taken.is_empty() {
            if !self.sends() {
                self.attach.put_back(taken);
                return;
            }
            let refs: Vec<_> = taken
                .iter()
                .filter_map(|f| f.attachment().cloned())
                .collect();
            let delivery = self
                .chat
                .borrow_mut()
                .deliver_with(&words, refs, chrono::Utc::now());
            let Some(d) = delivery else {
                self.attach.put_back(taken);
                return;
            };
            self.jump.sent(&d.text);
            self.composer.set_draft("");
            self.follow.set(true);
            self.draw();
            let p = self.clone();
            glib::spawn_future_local(async move {
                if !p.deliver(d).await {
                    if p.composer.draft().is_empty() {
                        p.composer.set_draft(&words);
                    }
                    p.attach.put_back(taken);
                }
            });
            return;
        }
        self.send_words(&words, true);
    }

    /// The words go (`submitChat`): a message now, or, while the taf works, to its run. Words that
    /// weren't taken go back in the box. `from_box`: they were the box's.
    fn send_words(self: &Rc<Self>, words: &str, from_box: bool) {
        if !self.sends() {
            return;
        }
        let submit = self
            .chat
            .borrow_mut()
            .submit(&self.w, words, chrono::Utc::now())
            .unwrap_or(Submit::Stay);
        let words = words.to_string();
        let back = move |p: &Rc<Page>| {
            if p.composer.draft().is_empty() {
                p.composer.set_draft(&words);
            }
        };
        match submit {
            Submit::Stay => {
                if !from_box {
                    back(self);
                }
                self.draw();
            }
            Submit::Send(d) => {
                self.jump.sent(&d.text);
                self.composer.set_draft("");
                self.follow.set(true);
                self.draw();
                let p = self.clone();
                if matches!(self.source, ChatSource::Sample { .. }) {
                    glib::spawn_future_local(async move { p.sample_reply(d).await });
                    return;
                }
                glib::spawn_future_local(async move {
                    if !p.deliver(d).await {
                        back(&p);
                    }
                });
            }
            Submit::Steer {
                run,
                text,
                client_id,
            } => {
                self.jump.sent(&text);
                self.composer.set_draft("");
                let p = self.clone();
                glib::spawn_future_local(async move {
                    if !p.steer(&run, &text, &client_id).await {
                        back(&p);
                    }
                });
            }
        }
    }

    /// `deliverChatOnce`'s send: true once the message is taken (sent, or kept as not sent).
    async fn deliver(self: &Rc<Self>, d: Delivery) -> bool {
        // The trial's chat takes its words alone (`trialMessage`).
        let (path, body) = if self.taf.trial {
            (
                tafs_core::versions::trial_messages_path(&self.taf.id),
                serde_json::json!({ "text": d.text })
                    .to_string()
                    .into_bytes(),
            )
        } else {
            (
                coach_send::messages_path(&self.taf.id),
                coach_send::message_body(
                    &d.text,
                    &d.target,
                    &d.files,
                    (!self.taf.trial).then_some(d.client_id.as_str()),
                ),
            )
        };
        // Streamed, so the taf's words show as it writes them; sent plainly when the stream can't
        // open. A trial's turn runs in the background, and its chat is read as it goes.
        let streamed = match (&self.source, self.taf.trial) {
            (
                ChatSource::Cloud {
                    token, languages, ..
                },
                false,
            ) => crate::chat::open_stream(token, languages, &self.taf.id, body.clone()).await,
            _ => crate::chat::Streamed::Plain,
        };
        let answer = match streamed {
            crate::chat::Streamed::Opened(events) => {
                self.chat.borrow_mut().streaming(chrono::Utc::now());
                self.draw();
                let p = self.clone();
                let (text, local) = (d.text.clone(), d.local.clone());
                glib::spawn_future_local(async move {
                    p.follow_stream(events, text, Some(local)).await;
                });
                return true;
            }
            crate::chat::Streamed::Refused(a) => Ok(a),
            crate::chat::Streamed::Plain => match self.post(path, body).await {
                Some(answer) => answer,
                None => return false,
            },
        };
        let now = chrono::Utc::now();
        let taken = match answer {
            Ok(a) if a.status == 401 => {
                self.signed_out();
                return false;
            }
            Ok(a) if (200..300).contains(&a.status) => match if self.taf.trial {
                tafs_core::versions::trial_sent(&a.body)
            } else {
                coach_send::sent(&a.body)
            } {
                Some(sent) => {
                    self.chat.borrow_mut().delivered(&d, &sent, now);
                    self.draw();
                    self.load().await;
                    return true;
                }
                None => {
                    let why = tafs_core::signin::plain_words(&self.w, 200).unwrap_or_default();
                    self.chat
                        .borrow_mut()
                        .not_sent(&d, &why, None, false, Some(200), now);
                    true
                }
            },
            Ok(a) => {
                let r = Refusal::read(a.status, &a.body);
                match taf_chat::refused(&self.w, &r, &self.taf.name, false) {
                    Ok(SendRefused::Consent) => {
                        // Its screen says why; the words go back in the box, with no Not sent row.
                        self.chat.borrow_mut().not_tried(&d, None);
                        self.consent(&a.body);
                        false
                    }
                    Ok(SendRefused::Box(words)) => {
                        self.chat.borrow_mut().not_tried(&d, Some(words));
                        false
                    }
                    Ok(SendRefused::NotSent { why, halted, busy }) => {
                        self.chat.borrow_mut().not_sent(
                            &d,
                            &why,
                            halted,
                            busy,
                            Some(r.status),
                            now,
                        );
                        true
                    }
                    Err(_) => {
                        self.chat.borrow_mut().not_tried(&d, None);
                        false
                    }
                }
            }
            Err(why) => {
                self.chat
                    .borrow_mut()
                    .not_sent(&d, &why, None, false, None, now);
                true
            }
        };
        self.draw();
        taken
    }

    /// `followChatStream`: the taf's answer drawn as it streams, its words in the order they came.
    /// A stream that drops leaves the reads to show the rest, as they do without one; leaving the
    /// chat stops following it (the run goes on at tafs cloud).
    async fn follow_stream(
        self: &Rc<Self>,
        events: async_channel::Receiver<tafs_core::stream::Event>,
        text: String,
        sent: Option<String>,
    ) {
        let mut f = taf_chat::Following::default();
        while let Ok(e) = events.recv().await {
            if !self.open() {
                self.chat.borrow_mut().live_gone();
                return;
            }
            let then = self
                .chat
                .borrow_mut()
                .stream_event(&self.w, &e, sent.as_deref(), &mut f);
            // A call held: KIFF is read for its card, where the call was made.
            if matches!(e, tafs_core::stream::Event::Held { .. }) {
                let p = self.clone();
                crate::shell::reread_home();
                glib::spawn_future_local(async move {
                    p.reread_held().await;
                    p.draw();
                });
            }
            // Words come many a second: drawn once the ones already here are taken.
            if matches!(then, Ok(coach_send::Then::Draw)) && !events.is_empty() {
                continue;
            }
            self.draw();
            if let Ok(coach_send::Then::Load(_)) = then {
                self.load_now().await;
            }
        }
        if !self.open() {
            self.chat.borrow_mut().live_gone();
            return;
        }
        // The recorded answer first, then the live one goes: no gap between them. KIFF is read
        // again (`refresh`), and each held call keeps its place, after what the taf wrote before
        // it (`anchorHeld`).
        self.chat.borrow_mut().stream_over(&f, &text);
        self.load_now().await;
        crate::shell::reread_home();
        self.reread_held().await;
        self.draw();
        let mut c = self.chat.borrow_mut();
        c.anchor_held();
        c.live_gone();
        drop(c);
        self.draw();
    }

    /// `refresh`, for this taf: KIFF's held calls read again, and its cards as Needs you draws
    /// them; then KIFF's activity, for what became of its calls. A read that fails changes
    /// nothing.
    async fn reread_held(self: &Rc<Self>) {
        // Signed out by the first read: the second isn't sent.
        if self.reread_calls().await {
            return;
        }
        self.reread_record().await;
    }

    /// `refresh`'s activity, for this taf (`activityCalls`, `KIFFMapping.record`): its Record
    /// lines, for the outcomes in its chat and what each call moved. A read that fails changes
    /// nothing.
    async fn reread_record(self: &Rc<Self>) {
        let (
            Some(r),
            ChatSource::Cloud {
                token,
                languages,
                signed_out,
            },
        ) = (self.taf.reread.clone(), &self.source)
        else {
            return;
        };
        let (token, languages, signed_out) = (token.clone(), languages.clone(), signed_out.clone());
        let read = gtk::gio::spawn_blocking(move || crate::kiff::activity(&token, &languages).ok())
            .await
            .ok()
            .flatten();
        let Some(answer) = read else { return };
        if answer.status == 401 {
            signed_out();
            return;
        }
        let Some(calls) = (answer.status == 200)
            .then(|| tafs_core::record::read_activity(&answer.body))
            .flatten()
        else {
            return;
        };
        let taf_of = |actor: &str| (actor == r.agent).then(|| r.key.clone());
        // `toolVendors`: none, as this computer doesn't read planque's connections (the Record's).
        let vendors = tafs_core::connections::tool_vendors(&[]);
        if let Ok(entries) = tafs_core::record::record(&self.w, &calls, &taf_of, &vendors) {
            self.chat.borrow_mut().record = entries;
        }
    }

    /// KIFF's held calls read again, for this taf, and its cards as Needs you draws them. True
    /// when KIFF said the session is over, and this computer was signed out.
    async fn reread_calls(self: &Rc<Self>) -> bool {
        let (
            Some(r),
            ChatSource::Cloud {
                token,
                languages,
                signed_out,
            },
        ) = (self.taf.reread.clone(), &self.source)
        else {
            return false;
        };
        let (token, languages, signed_out) = (token.clone(), languages.clone(), signed_out.clone());
        let read = gtk::gio::spawn_blocking(move || crate::kiff::held(&token, &languages).ok())
            .await
            .ok()
            .flatten();
        let Some(answer) = read else { return false };
        if answer.status == 401 {
            signed_out();
            return true;
        }
        let Some(exceptions) = (answer.status == 200)
            .then(|| tafs_core::held::read_held(&answer.body))
            .flatten()
        else {
            return false;
        };
        let calls: Vec<_> = exceptions
            .iter()
            .filter(|e| e.subject == r.agent)
            .filter_map(|e| tafs_core::held::held(&self.w, e, &r.key).ok())
            .collect();
        let holders = [tafs_core::needs_you::Holder {
            key: &r.key,
            name: &r.name,
            initial: &r.initial,
            color: r.color,
        }];
        if let Ok(l) = tafs_core::needs_you::lines(
            &self.w,
            &calls,
            &holders,
            r.may_answer,
            r.manages_cards,
            chrono::Local::now(),
        ) {
            *self.held.borrow_mut() = l.cards;
        }
        false
    }

    /// A screenshot's stream, from its file: each line read as the stream's are, taken as
    /// `follow_stream` takes them, with no thread read between.
    fn stream_from(&self, path: &std::path::Path) {
        let Ok(lines) = std::fs::read_to_string(path) else {
            eprintln!("{} isn't a stream's events", path.display());
            std::process::exit(2);
        };
        let mut c = self.chat.borrow_mut();
        c.streaming(chrono::Utc::now());
        let (mut reader, mut f) = (
            tafs_core::stream::Reader::default(),
            taf_chat::Following::default(),
        );
        for line in lines.lines() {
            if let Some(e) = reader.line(line) {
                let _ = c.stream_event(&self.w, &e, None, &mut f);
            }
        }
    }

    /// `resumeChat`: a run of the person's that tafs cloud says is at work while nothing here
    /// follows it is picked back up (`GET …/runs/{run}/stream?after=0`): its steps so far, then
    /// the rest as it goes, and its end reads the thread, as a message's stream does. Once per
    /// run: one whose stream can't open is left to the reads. Never a trial's.
    async fn resume(self: &Rc<Self>) {
        let ChatSource::Cloud {
            token, languages, ..
        } = self.source.clone()
        else {
            return;
        };
        if self.taf.trial {
            return;
        }
        let Some((run, session)) = self.chat.borrow_mut().to_resume() else {
            return;
        };
        let path = tafs_core::stream::resume_path(&self.taf.id, &run, 0);
        let opened = gtk::gio::spawn_blocking(move || {
            crate::kiff::cloud_stream(&token, &languages, "GET", &path, None)
        })
        .await;
        let Ok(Ok(crate::kiff::Opened::Events(events))) = opened else {
            return;
        };
        if !self.open() || self.chat.borrow().live.is_some() {
            return;
        }
        let text = self
            .chat
            .borrow_mut()
            .resuming(&run, &session, chrono::Utc::now());
        self.draw();
        let p = self.clone();
        glib::spawn_future_local(async move {
            p.follow_stream(events, text, None).await;
        });
    }

    /// `--pin-check`: a message sent here, its stream's events played as they come, and the run's
    /// end as tafs cloud's reload brings it, with the pin checked as the iPad's `ChatPinUITests`.
    /// Nothing is sent.
    fn pin_check(self: &Rc<Self>, path: &std::path::Path) {
        const WORDS: &str = "Which refunds went out this morning?";
        let events = crate::chat::events_from(path);
        let sent: Rc<RefCell<Option<Delivery>>> = Rc::default();
        let f: Rc<RefCell<taf_chat::Following>> = Rc::default();
        let send = {
            let (p, sent) = (self.clone(), sent.clone());
            Box::new(move || {
                let now = chrono::Utc::now();
                let d = p.chat.borrow_mut().deliver(WORDS, now);
                p.jump.sent(WORDS);
                p.chat.borrow_mut().streaming(now);
                *sent.borrow_mut() = d;
                p.follow.set(true);
                p.draw();
            })
        };
        let event = {
            let (p, sent, f) = (self.clone(), sent.clone(), f.clone());
            Box::new(move |e: &tafs_core::stream::Event| {
                let local = sent.borrow().as_ref().map(|d| d.local.clone());
                let _ = p.chat.borrow_mut().stream_event(
                    &p.w,
                    e,
                    local.as_deref(),
                    &mut f.borrow_mut(),
                );
                p.draw();
            })
        };
        let finish = {
            let (p, sent, f) = (self.clone(), sent.clone(), f.clone());
            Box::new(move || {
                let now = chrono::Utc::now();
                let mut c = p.chat.borrow_mut();
                c.stream_over(&f.borrow(), WORDS);
                let live = c.live.clone().unwrap_or_default();
                let session = live.session.clone().unwrap_or_else(|| "live".into());
                // tafs cloud's copy of the message, in its chat (another id than the one shown
                // while it worked), and the answer recorded.
                let _ = sent.borrow();
                for m in c.thread.messages.iter_mut() {
                    if m.session == tafs_core::coach::PENDING {
                        m.session = session.clone();
                    }
                }
                let mut words: Vec<String> = live
                    .parts
                    .iter()
                    .filter_map(|p| match p {
                        tafs_core::stream::ReplyPart::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect();
                words.push(live.text.clone());
                let mut answer = c
                    .thread
                    .messages
                    .last()
                    .cloned()
                    .unwrap_or_else(|| unreachable!("the message just sent"));
                answer.from = tafs_core::coach::From::Taf;
                answer.text = words.concat();
                answer.session = session;
                answer.at = now;
                answer.server_id = None;
                answer.attachments = vec![];
                c.thread.messages.push(answer);
                c.awaiting = None;
                c.anchor_held();
                c.live_gone();
                drop(c);
                p.draw();
            })
        };
        crate::chat::pin_check(
            self.jump.clone(),
            events,
            crate::chat::PinSteps {
                send,
                event,
                finish,
                root: self.column.clone().upcast(),
            },
        );
    }

    /// The thread read now, after a read already going ends.
    async fn load_now(self: &Rc<Self>) {
        while self.reading.get() {
            glib::timeout_future(std::time::Duration::from_millis(50)).await;
        }
        self.load().await;
    }

    /// Retry on a run that failed part-way (`retryChatRun`): its message, sent again.
    fn retry_run(self: &Rc<Self>) {
        let retry = self.chat.borrow_mut().retry_run();
        if let Some(text) = retry {
            self.send_words(&text, false);
        }
    }

    /// Written to the run while the taf works (`steer`, once, with `hold`): true once taken, held
    /// or kept as not sent. More than tafs cloud holds leaves the words in the box.
    /// `id`: the message's `client_message_id`.
    async fn steer(self: &Rc<Self>, run: &str, words: &str, id: &str) -> bool {
        let Some(answer) = self
            .post(
                coach_send::steer_path(&self.taf.id),
                coach_send::steer_body(run, words, (!self.taf.trial).then_some(id)),
            )
            .await
        else {
            return false;
        };
        let now = chrono::Utc::now();
        let taken = match answer {
            Ok(a) if a.status == 401 => {
                self.signed_out();
                return false;
            }
            Ok(a) if (200..300).contains(&a.status) => match coach_send::steered(&a.body) {
                Some(s) => {
                    self.chat.borrow_mut().steered(run, words, &s, now);
                    self.draw();
                    if matches!(s, Steered::Kept { sent: Some(_), .. }) {
                        self.load().await;
                    }
                    return true;
                }
                None => {
                    let why = tafs_core::signin::plain_words(&self.w, 200).unwrap_or_default();
                    self.chat.borrow_mut().steer_not_sent(
                        words,
                        (id, Some(200)),
                        &why,
                        None,
                        false,
                        now,
                    );
                    true
                }
            },
            Ok(a) => {
                let r = Refusal::read(a.status, &a.body);
                match taf_chat::refused(&self.w, &r, &self.taf.name, true) {
                    Ok(SendRefused::Consent) => {
                        self.consent(&a.body);
                        false
                    }
                    Ok(SendRefused::Box(why)) => {
                        self.chat.borrow_mut().error = Some(why);
                        false
                    }
                    Ok(SendRefused::NotSent { why, halted, busy }) => {
                        self.chat.borrow_mut().steer_not_sent(
                            words,
                            (id, Some(a.status)),
                            &why,
                            halted,
                            busy,
                            now,
                        );
                        true
                    }
                    Err(_) => false,
                }
            }
            Err(why) => {
                self.chat
                    .borrow_mut()
                    .steer_not_sent(words, (id, None), &why, None, false, now);
                true
            }
        };
        self.draw();
        taken
    }

    /// Stop (`stopChat`): the taf's current run is cancelled, over here at once, then the thread
    /// read again.
    /// The loop card (`LoopCardView`): "Still reading" and its line, in tafs cloud's words; its
    /// answers (the first one primary) while the run waits on it, else how it was answered.
    fn loop_card(
        self: &Rc<Self>,
        id: &str,
        title: &str,
        words: &str,
        options: &[tafs_core::coach::Choice],
        answered: Option<&str>,
        taken: bool,
    ) -> gtk::Widget {
        let p = panel(false);
        p.set_widget_name(&format!("taf-chat.loop.{id}"));
        if !title.is_empty() {
            p.append(&text(title, 15, 600, &["tafs-ink"]));
        }
        if !words.is_empty() {
            p.append(&text(words, 14, 400, &["tafs-soft"]));
        }
        match answered {
            Some(how) => {
                if !how.is_empty() {
                    p.append(&text(how, 13, 400, &["tafs-dim"]));
                }
            }
            None => {
                for (i, o) in options.iter().enumerate() {
                    let b = if i == 0 {
                        crate::widgets::primary(&o.label)
                    } else {
                        outline(&o.label)
                    };
                    b.set_widget_name(&format!("taf-chat.loop.{}", o.value));
                    b.set_sensitive(!taken);
                    let (me, id, value) = (self.clone(), id.to_string(), o.value.clone());
                    b.connect_clicked(move |_| me.answer_loop(&id, &value));
                    p.append(&b);
                }
            }
        }
        p.upcast()
    }

    /// `answerLoopCard`: the answer goes to the taf's own route with the card's chat
    /// (`POST /v1/tafs/{id}/components/{component}/answer`), unsigned as the session. Taken, the
    /// thread is read again and the run that waited is followed to its end. A card that no
    /// longer waits (409) is read again; anything else says why above the box.
    fn answer_loop(self: &Rc<Self>, id: &str, value: &str) {
        if !matches!(self.source, ChatSource::Cloud { .. }) {
            return;
        }
        let Some((component, body, what)) = self.chat.borrow_mut().answer_loop(id, value) else {
            return;
        };
        self.draw();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let Some(answer) = p
                .post(coach_send::answer_path(&p.taf.id, &component), body)
                .await
            else {
                return;
            };
            let failed = match answer {
                Ok(a) if a.status == 401 => return p.signed_out(),
                Ok(a) if a.status == 409 => None,
                Ok(a) if (200..300).contains(&a.status) => {
                    match coach_send::card_answered(&a.body) {
                        Some(r) => {
                            p.chat
                                .borrow_mut()
                                .loop_answered(&r.run, chrono::Utc::now());
                            p.draw();
                            p.load().await;
                            p.resume().await;
                            return;
                        }
                        None => Some(tafs_core::signin::plain_words(&p.w, 200).unwrap_or_default()),
                    }
                }
                Ok(a) => Some(
                    Refusal::read(a.status, &a.body)
                        .words(&p.w)
                        .unwrap_or_default(),
                ),
                Err(why) => Some(why),
            };
            let why = failed.map(|why| {
                p.w.get("common.didnt_go_through", &[("what", &what), ("why", &why)])
                    .unwrap_or(why)
            });
            let reread = why.is_none();
            p.chat.borrow_mut().loop_not_taken(&component, why);
            p.draw();
            if reread {
                p.load().await;
            }
        });
    }

    fn stop(self: &Rc<Self>) {
        if !self.sends() {
            return;
        }
        let Some(session) = self.chat.borrow_mut().stop() else {
            return;
        };
        self.draw();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let Some(answer) = p
                .post(
                    coach_send::stop_path(&p.taf.id),
                    coach_send::stop_body(&session),
                )
                .await
            else {
                return;
            };
            let failed = match answer {
                Ok(a) if a.status == 401 => return p.signed_out(),
                Ok(a) if (200..300).contains(&a.status) && coach_send::stopped(&a.body) => None,
                Ok(a) if (200..300).contains(&a.status) => {
                    Some(tafs_core::signin::plain_words(&p.w, 200).unwrap_or_default())
                }
                Ok(a) => Some(
                    Refusal::read(a.status, &a.body)
                        .words(&p.w)
                        .unwrap_or_default(),
                ),
                Err(why) => Some(why),
            };
            {
                let mut c = p.chat.borrow_mut();
                match failed {
                    None => c.stopped(&session),
                    Some(why) => {
                        c.error = Some(
                            p.w.get("work.did_not_stop", &[("why", &why)])
                                .unwrap_or(why),
                        )
                    }
                }
            }
            p.draw();
            p.load().await;
        });
    }

    /// Retry on a message that didn't go (`retryChat`): sent once more; not taken, it goes back.
    fn retry(self: &Rc<Self>, id: u64) {
        if !self.sends() {
            return;
        }
        let Some((at, message)) = self.chat.borrow_mut().retry(id) else {
            return;
        };
        let left = self.chat.borrow().unsent.len();
        let submit = self
            .chat
            .borrow_mut()
            .submit(&self.w, &message.text, chrono::Utc::now())
            .unwrap_or(Submit::Stay);
        if !matches!(submit, Submit::Stay) {
            self.jump.sent(&message.text);
        }
        self.draw();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let taken = match submit {
                Submit::Stay => false,
                Submit::Send(d) => p.deliver(d).await,
                Submit::Steer {
                    run,
                    text,
                    client_id,
                } => p.steer(&run, &text, &client_id).await,
            };
            if taken {
                p.chat.borrow_mut().retry_kept(left, &message);
            } else {
                p.chat.borrow_mut().retry_back(at, message);
            }
            p.draw();
        });
    }

    /// Delete (`discardUnsentChat`): out of the thread, not sent.
    fn discard(self: &Rc<Self>, id: u64) {
        self.chat.borrow_mut().discard(id);
        self.draw();
    }

    /// Edit: out of the thread, its words back in the box.
    fn edit(self: &Rc<Self>, id: u64) {
        let words = self.chat.borrow_mut().edit(id);
        if let Some(words) = words {
            self.composer.set_draft(&words);
            self.composer.view.grab_focus();
        }
        self.draw();
    }

    /// The box's placeholder and button: Stop while the taf works and nothing is written.
    fn composer_state(&self) {
        // The sample's taf is never at work on its chat: no Stop, no steering.
        let thinking =
            self.chat.borrow().thinking() && !matches!(self.source, ChatSource::Sample { .. });
        if let Ok(k) = taf_chat::composer(&self.w, Some(&self.taf.name), thinking) {
            let (ready, held) = self.attach.send_state(&k.send, thinking);
            self.composer.show_with(
                &k.placeholder,
                &k.send,
                &k.stop,
                thinking,
                Some((ready, held.as_deref())),
            );
        }
    }

    /// Draws what the chat holds now, unless it's what's drawn already.
    fn draw(self: &Rc<Self>) {
        self.composer_state();
        let told = self.on_drawn.borrow().clone();
        if let Some(told) = told {
            let c = self.chat.borrow();
            let any = !c.thread.messages.is_empty() || !c.unsent.is_empty() || c.live.is_some();
            drop(c);
            told(any);
        }
        // The doors read again each time the thread's messages change, and once it's read: their
        // knocks sit among them (`.task(id: chats[tafID]?.count)`).
        if let Some((model, seen)) = &self.doors {
            let c = self.chat.borrow();
            let count = c.loaded.then_some(c.thread.messages.len());
            drop(c);
            if count.is_some() && seen.get() != count {
                seen.set(count);
                model.load();
            }
        }
        // Which call each held card is, by KIFF's id: a streamed answer names its held calls so.
        let known: Vec<(String, String)> = self
            .held
            .borrow()
            .iter()
            .filter_map(|h| Some((h.kiff_id.clone()?, h.record.clone())))
            .collect();
        self.chat.borrow_mut().know_held(known);
        let c = self.chat.borrow();
        match &c.error {
            Some(why) => {
                self.error.set_label(why);
                self.error.set_visible(true);
            }
            None => self.error.set_visible(false),
        }
        if !c.loaded && c.error.is_none() && c.unsent.is_empty() {
            return;
        }
        let held: Vec<(String, chrono::DateTime<chrono::Utc>)> = self
            .held
            .borrow()
            .iter()
            .map(|h| (h.record.clone(), h.held_at.with_timezone(&chrono::Utc)))
            .collect();
        let whose = Whose {
            name: Some(&self.taf.name),
            ask_coach: self.taf.ask_coach,
        };
        let lines = match taf_chat::lines(&self.w, &c, &whose, &held, chrono::Utc::now()) {
            Ok(l) => l,
            Err(e) => {
                self.error.set_label(&e.to_string());
                self.error.set_visible(true);
                return;
            }
        };
        drop(c);
        if self.drawn.borrow().as_ref() == Some(&lines) {
            return;
        }
        let keep = self.scroller.vadjustment().value();
        let before = self.drawn.borrow().as_ref().map(|d| d.lines.len());
        self.jump.hold();
        self.jump.begin();
        while let Some(child) = self.column.first_child() {
            self.column.remove(&child);
        }
        self.draw_lines(&lines);
        // The held call waiting for the owner (`chatWaiting`): the taf's latest, where the
        // thread shows it, and whether it's among its last few items.
        let waiting = self
            .taf
            .held
            .iter()
            .filter_map(|h| {
                let at = lines
                    .lines
                    .iter()
                    .position(|l| matches!(l, Line::Held(k) if k == &h.record))?;
                Some((h, at))
            })
            .max_by_key(|(h, _)| h.held_at);
        let near_end = waiting.is_none_or(|(_, at)| lines.lines.len() - at <= 3);
        let pill = waiting.and_then(|(h, _)| {
            let card =
                crate::coach::named(self.column.upcast_ref(), &format!("held.{}", h.record))?;
            let words = self.w.get("chat.needs_you", &[]).ok()? + &h.summary;
            Some((words, card))
        });
        self.jump.waiting(
            pill.as_ref()
                .map(|(words, card)| (words.as_str(), card.clone())),
        );
        let (ready, grew) = (
            !lines.lines.is_empty(),
            before.is_some_and(|b| lines.lines.len() > b),
        );
        // What the pin can hold, in the thread's order (`pinNext`, `pinned`).
        let candidates = crate::chat::pin_candidates(
            &lines.pins,
            lines.unsent.iter().map(|u| (u.id, u.words.clone())),
            &lines.steering,
            lines.queued.as_ref(),
        );
        *self.drawn.borrow_mut() = Some(lines);
        // Something new below while the owner reads back: ↓'s dot.
        if grew {
            self.jump.arrived();
        }
        // The thread follows what arrives while its latest is in view (a message sent brings it
        // there), and lands first on the held call waiting, or at the newest.
        if self.follow.replace(false) {
            self.jump.following.set(true);
        }
        let jump = self.jump.clone();
        let column = self.column.clone();
        glib::idle_add_local_once(move || jump.drawn(&column, &candidates, keep, ready, near_end));
    }

    fn draw_lines(self: &Rc<Self>, lines: &ChatLines) {
        let column = &self.column;
        if let Some(empty) = &lines.empty {
            let l = text(empty, 14, 400, &["tafs-dim"]);
            l.set_xalign(0.5);
            l.set_justify(gtk::Justification::Center);
            l.set_margin_top(30);
            l.set_margin_bottom(30);
            column.append(&l);
        }
        // The answer streaming now: its work block, before its first words.
        let live_at = lines.live.as_ref().and_then(|(_, at)| *at);
        // Only the words being written now are paced; those before a tool call are whole.
        let pacing = lines.lines.iter().rposition(|l| {
            matches!(
                l,
                Line::Taf {
                    streaming: true,
                    ..
                }
            )
        });
        if pacing.is_none() {
            self.live_shown.set(None);
        }
        for (i, line) in lines.lines.iter().enumerate() {
            if let (Some((work, _)), true) = (&lines.live, live_at == Some(i)) {
                column.append(&crate::chat::live_work(work));
            }
            let row: gtk::Widget = match line {
                Line::Gap(at) => crate::chat::gap(at),
                Line::Kickoff(note) => {
                    let l = text(note, 12, 400, &["tafs-dim"]);
                    l.set_xalign(0.5);
                    l.set_justify(gtk::Justification::Center);
                    l.set_lines(2);
                    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    l.set_margin_top(4);
                    l.set_margin_bottom(4);
                    l.upcast()
                }
                Line::Owner {
                    text: words,
                    answered,
                    stamp,
                    menu,
                    pending,
                    files,
                } => self.owner_line(
                    lines,
                    words,
                    answered.as_deref(),
                    stamp.as_deref(),
                    (*menu, *pending),
                    files,
                ),
                Line::Taf {
                    work,
                    delivered,
                    text: words,
                    counted,
                    refused,
                    stamp,
                    report,
                    streaming,
                } => {
                    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
                    // Its menu: Report this answer, where it can be reported.
                    if let Some(r) = report {
                        self.reportable(&v, r);
                    }
                    if let Some(work) = work {
                        v.append(&crate::chat::work_summary(work));
                    }
                    // Where the run's result went: tafs cloud's line, above the taf's words.
                    if let Some(d) = delivered {
                        v.append(&run_line(d));
                    }
                    // Only the part being written now is paced and healed; a part finished before
                    // a tool call is drawn as written, so its own "[1]" stays (parity-steward's note).
                    let answer = if *streaming && pacing == Some(i) {
                        crate::chat::steady_answer_words(words, &self.live_shown)
                    } else {
                        crate::chat::answer_words(words)
                    };
                    answer.set_widget_name("taf-message");
                    v.append(&answer);
                    if let Some((s, failed)) = stamp {
                        v.append(&door_stamp(s, false, *failed));
                    }
                    // What the run did, counted from the record: tafs cloud's line, under them.
                    if let Some(c) = counted {
                        v.append(&run_line(c));
                    }
                    // A connection its work asked for that was refused: never folded.
                    if let Some(r) = refused {
                        v.append(&refused_lines(r));
                    }
                    v.upcast()
                }
                Line::Held(key) => match self.held.borrow().iter().find(|h| &h.record == key) {
                    Some(card) => {
                        let c = crate::needs_you::held_card(&self.w, card);
                        c.set_widget_name(&format!("held.{key}"));
                        c.upcast()
                    }
                    None => continue,
                },
                // What became of a call, from the Record, where its card was or when answered.
                Line::Outcome(o) => crate::coach::outcome(o).upcast(),
                // Someone wrote through a door: its card, answered here, eight lines of it.
                Line::Knock(k) => match &self.doors {
                    Some((model, _)) => {
                        let busy = model.state.borrow().busy;
                        let card = crate::knocks::knock_card(
                            model,
                            Some(&self.taf.name),
                            k,
                            busy,
                            false,
                            &self.column,
                        );
                        card.set_widget_name(&format!("knock.{}", k.id));
                        card.upcast()
                    }
                    None => continue,
                },
                Line::Loop {
                    id,
                    title,
                    text: words,
                    options,
                    answered,
                    taken,
                } => self.loop_card(id, title, words, options, answered.as_deref(), *taken),
                Line::AskCoach { text: words, ask } => {
                    let p = panel(false);
                    let l = text(words, 15, 400, &["tafs-ink"]);
                    p.append(&l);
                    if let Some(ask) = ask {
                        // Coach on this taf, the card's words in its box (`askCoachAbout`).
                        let b = outline(ask);
                        b.set_widget_name("taf-chat.ask-coach");
                        let (taf, name, say) =
                            (self.taf.id.clone(), self.taf.name.clone(), words.clone());
                        b.connect_clicked(move |_| {
                            crate::coach_start::improve(&taf, &name, Some(&say))
                        });
                        p.append(&b);
                    }
                    p.upcast()
                }
            };
            // What the pin can hold: the owner's message, by its id.
            if let Some((_, c)) = lines.pins.iter().find(|(at, _)| *at == i) {
                self.jump.row(&c.id, &row);
            }
            column.append(&row);
        }
        let busy = self.chat.borrow().sending;
        for u in &lines.unsent {
            let row = self.unsent(u, busy);
            self.jump.row(&format!("u|{}", u.id), &row);
            column.append(&row);
        }
        // What the taf is doing and the steps it took, under its words so far; the dots while
        // nothing streams.
        match &lines.live {
            Some((work, None)) => column.append(&crate::chat::live_work(work)),
            Some(_) => {}
            None => {
                if let Some(answering) = &lines.thinking {
                    column.append(&crate::chat::dots(answering));
                }
            }
        }
        // Written to the run while it goes, not read yet.
        for (i, (words, note)) in lines.steering.iter().enumerate() {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
            v.append(&owner(words, None, true));
            let n = text(note, 12, 400, &["tafs-dim"]);
            n.set_xalign(1.0);
            v.append(&n);
            self.jump.row(&format!("steer|{i}"), &v);
            column.append(&v);
        }
        // It failed part-way: what went through before, and Retry, which sends its message again.
        if let Some((words, retry)) = &lines.failed {
            let h = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            h.add_css_class("tafs-notice");
            h.add_css_class("tafs-failed-run");
            let l = text(words, 14, 400, &["tafs-ink"]);
            l.set_hexpand(true);
            h.append(&l);
            let b = gtk::Button::with_label(retry);
            b.add_css_class("flat");
            b.set_sensitive(!busy);
            let p = self.clone();
            b.connect_clicked(move |_| p.retry_run());
            h.append(&b);
            column.append(&h);
        }
        // Stopped at its own cap, with no words: why.
        if let Some(capped) = &lines.capped {
            let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
            b.add_css_class("tafs-notice");
            b.add_css_class("amber");
            b.append(&text(capped, 14, 400, &["tafs-ink"]));
            column.append(&b);
        }
        // tafs cloud holds it, and sends it once the taf is done (`QueuedBubble`).
        if let Some((words, waits)) = &lines.queued {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let b = owner(words, None, false);
            b.add_css_class("tafs-unsent");
            v.append(&b);
            let n = text(waits, 13, 400, &["tafs-dim"]);
            n.set_xalign(1.0);
            v.append(&n);
            self.jump.row("queued", &v);
            column.append(&v);
        }
    }

    /// The owner's words, with the door they came through above them; a message sent from here
    /// has its menu: Send again, Edit and send, Copy.
    fn owner_line(
        self: &Rc<Self>,
        lines: &ChatLines,
        words: &str,
        answered: Option<&str>,
        stamp: Option<&str>,
        (menu, pending): (bool, bool),
        files: &[tafs_core::coach::Attachment],
    ) -> gtk::Widget {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
        if let Some(s) = stamp {
            v.append(&door_stamp(s, true, false));
        }
        let b = owner(words, answered, false);
        if pending {
            b.set_opacity(0.7);
        }
        if menu {
            let (again, edit, copy) = lines.menu.clone();
            let send_words = words.to_string();
            let edit_words = words.to_string();
            let copy_words = words.to_string();
            let (p, q) = (Rc::downgrade(self), Rc::downgrade(self));
            let widget = b.clone();
            crate::chat::menu_on(
                &b,
                vec![
                    (
                        again,
                        Rc::new(move || {
                            if let Some(p) = p.upgrade() {
                                p.send_words(&send_words, false);
                            }
                        }),
                    ),
                    (
                        edit,
                        Rc::new(move || {
                            if let Some(q) = q.upgrade() {
                                q.composer.set_draft(&edit_words);
                                q.composer.view.grab_focus();
                            }
                        }),
                    ),
                    (
                        copy,
                        Rc::new(move || widget.clipboard().set_text(&copy_words)),
                    ),
                ],
            );
        }
        // Files sent alone have no words: their chips stand for the message.
        if !words.is_empty() || files.is_empty() {
            v.append(&b);
        }
        if !files.is_empty() {
            let attach = self.attach.clone();
            v.append(&crate::chat::message_files(
                &self.w,
                files,
                Rc::new(move |f, chip| attach.open_sent(f, chip)),
            ));
        }
        v.upcast()
    }

    /// The answer's menu (`reportable`): Report this answer, which opens its sheet.
    fn reportable(self: &Rc<Self>, on: &gtk::Box, r: &tafs_core::report::Reported) {
        let to = match &self.source {
            ChatSource::Cloud {
                token,
                languages,
                signed_out,
            } => crate::report_sheet::ReportTo::Cloud {
                token: token.clone(),
                languages: languages.clone(),
                signed_out: signed_out.clone(),
            },
            _ => crate::report_sheet::ReportTo::Nowhere,
        };
        let (w, taf, r, at) = (self.w.clone(), self.taf.id.clone(), r.clone(), on.clone());
        crate::chat::menu_on(
            on,
            vec![(
                self.w.line("report.menu").unwrap_or_default(),
                Rc::new(move || {
                    crate::report_sheet::open(
                        w.clone(),
                        at.upcast_ref(),
                        taf.clone(),
                        r.clone(),
                        to.clone(),
                    )
                }),
            )],
        );
    }

    /// `UnsentBubble`: the words dimmed, then why. A send refused for a halt says so, and where
    /// Resume would be, why this computer can't: resuming is signed, on the phone.
    fn unsent(self: &Rc<Self>, u: &UnsentLines, busy: bool) -> gtk::Widget {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let b = owner(&u.words, None, false);
        b.add_css_class("tafs-unsent");
        v.append(&b);
        crate::coach::unsent_menu(&v, &u.copy, &u.words, &u.delete, {
            let (p, id) = (Rc::downgrade(self), u.id);
            move || {
                if let Some(p) = p.upgrade() {
                    p.discard(id);
                }
            }
        });
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        h.set_halign(gtk::Align::End);
        let m = symbol(
            if u.halted {
                crate::widgets::icon_for("hand.raised.fill")
            } else {
                "dialog-error-symbolic"
            },
            13,
        );
        m.add_css_class("tafs-no");
        m.set_valign(gtk::Align::Start);
        m.set_accessible_role(gtk::AccessibleRole::Presentation);
        h.append(&m);
        let note = text(&u.note, 13, 400, &["tafs-no"]);
        note.set_xalign(1.0);
        note.set_justify(gtk::Justification::Right);
        h.append(&note);
        v.append(&h);
        if let Some(n) = &u.resume_note {
            let l = text(n, 12, 400, &["tafs-dim"]);
            l.set_xalign(1.0);
            l.set_justify(gtk::Justification::Right);
            v.append(&l);
        }
        if let Some(cant) = &u.cant {
            let c = crate::approval::not_ready(cant);
            c.set_halign(gtk::Align::End);
            c.set_hexpand(false);
            for child in std::iter::successors(c.first_child(), |w| w.next_sibling()) {
                if let Ok(l) = child.clone().downcast::<gtk::Label>() {
                    l.set_xalign(1.0);
                    l.set_justify(gtk::Justification::Right);
                }
            }
            v.append(&c);
        }
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        row.set_halign(gtk::Align::End);
        let link = |words: &str, strong: bool| {
            let b = gtk::Button::with_label(words);
            b.add_css_class("tafs-unsent-act");
            if strong {
                b.add_css_class("strong");
            }
            b
        };
        let id = u.id;
        if let Some(edit) = &u.edit {
            let b = link(edit, false);
            let p = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() {
                    p.edit(id);
                }
            });
            row.append(&b);
        }
        if let Some(retry) = &u.retry {
            let b = link(retry, !u.halted);
            b.set_sensitive(!busy);
            let p = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() {
                    p.retry(id);
                }
            });
            row.append(&b);
        }
        if row.first_child().is_some() {
            v.append(&row);
        }
        v.upcast()
    }
}

/// One of tafs cloud's lines about a run, beside the taf's words (`RunLine`): 13, soft, as written.
fn run_line(words: &str) -> gtk::Label {
    let l = text(words, 13, 400, &["tafs-soft"]);
    l.set_selectable(true);
    l
}

/// A refused connection's lines (`RefusedLines`), each as tafs cloud wrote it: the step's in the
/// failure colour, then why and what to do next.
fn refused_lines(r: &tafs_core::coach::Refused) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_widget_name("taf-refused");
    for (i, line) in r.lines().iter().enumerate() {
        let l = if i == 0 {
            text(line, 13, 500, &["tafs-no"])
        } else {
            text(line, 13, 400, &["tafs-soft"])
        };
        l.set_selectable(true);
        v.append(&l);
    }
    v
}

/// `DoorStamp`: monospaced 10 medium, in the blue, or the failure colour when it wasn't sent back.
fn door_stamp(words: &str, from_owner: bool, failed: bool) -> gtk::Label {
    let l = text(
        words,
        10,
        500,
        &[if failed { "tafs-no" } else { "tafs-blue" }],
    );
    l.add_css_class("tafs-mono");
    l.set_lines(2);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    l.set_xalign(if from_owner { 1.0 } else { 0.0 });
    l.update_property(&[gtk::accessible::Property::Label(&words.to_lowercase())]);
    l
}
