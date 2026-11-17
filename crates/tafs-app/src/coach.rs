//! Coach in Coach mode's panel (`SetupChatView` at a regular width, `coach_mode.rs`): Coach's conversation as tafs
//! cloud reads it with this computer's proof, drawn as the iPad draws it, with its message box.
//! The owner writes to Coach, unsigned, as the session (`TafsCloud.sendMessage`, `steer`, `stop`);
//! the sent message shows at once until tafs cloud's copy takes its place, and the thread is read
//! again every few seconds while the page is open (`followSetup`, no stream on this computer).
//! A confirm's choices and a questions card are answered here as data (`answerCard`); where an
//! approval's or a sign-in's buttons would be, it says this computer can't approve
//! (`ApprovalGate`), under tafs cloud's `wider` line. The lines are `tafs-core/src/coach_lines.rs`,
//! what's sent and how the conversation holds it `tafs-core/src/coach_send.rs`.

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;
use tafs_core::coach::{self, Kind, SetupThread};
use tafs_core::coach_lines::{
    ApprovalLines, Card, CoachLines, ComposerLines, DraftShown, Line, Listed, Mark, Outcome,
    PlanLines, PlanStep, ResultLines, SignInLines, Tone, UnsentLines,
};
use tafs_core::coach_send::{
    self, CardSend, Conversation, Delivery, Polls, QuestionSelection, Refusal, Steered, Submit,
};
use tafs_core::coach_tools::CoachTools;
use tafs_core::session::KeptSession;
use tafs_core::sidebar::{Mode, Selection};
use tafs_core::signin::Answer;
use tafs_core::words::WordBook;

use crate::chat::{owner, Composer, Sent};
use crate::widgets::{eyebrow, one_line, outline, panel, primary, symbol, text};

/// Where Coach's conversation is read from, and where what's written goes.
#[derive(Clone)]
pub enum CoachSource {
    /// A conversation as tafs cloud answers it, from a file: nothing is read again or sent. With
    /// `chats`, Coach's chats as tafs cloud lists them, and Previous conversations open over it.
    File {
        thread: std::path::PathBuf,
        chats: Option<std::path::PathBuf>,
        /// KIFF's answers to the drafts the approvals name, by id (`{"status", "answer"}`).
        drafts: Option<std::path::PathBuf>,
        /// tafs cloud's `GET /v1/setup` answer, for Coach's tools.
        setup: Option<std::path::PathBuf>,
    },
    /// tafs cloud, as the session, with this device's proof when it has a key and KIFF names
    /// this computer. `signed_out` is called when tafs cloud says the session is over.
    Cloud {
        kept: Box<KeptSession>,
        languages: Vec<&'static str>,
        signed_out: Rc<dyn Fn()>,
    },
    /// The sample has no Coach: its title alone, as before.
    None,
}

thread_local! {
    /// A screenshot opens Coach's tools step over the conversation.
    pub static OPEN_TOOLS: Cell<bool> = const { Cell::new(false) };
    /// What the owner is typing to Coach, not sent yet (`setupDraft`): kept while the app runs, so
    /// leaving Coach and coming back keeps it. Signing out forgets it.
    static DRAFT: RefCell<String> = const { RefCell::new(String::new()) };
}

/// Words put in Coach's box before it opens (`setupDraft = …; coachOpen = true`): the brief's
/// next move, or a mission's name from its log.
pub fn set_draft(words: &str) {
    DRAFT.with(|d| *d.borrow_mut() = words.to_string());
}

/// Signed out: nothing written for the last account stays.
pub fn forget() {
    DRAFT.with(|d| d.borrow_mut().clear());
}

/// A read or a send that didn't go: tafs cloud ended the session, or why, in words.
enum Fail {
    SignedOut,
    Why(String),
}

/// The page and what it holds while it's open.
struct Page {
    w: Rc<WordBook>,
    source: CoachSource,
    listed: Rc<Vec<Listed>>,
    open: Rc<dyn Fn(Selection)>,
    conv: RefCell<Conversation>,
    /// This computer's id in KIFF, once read, for the proof.
    device: RefCell<Option<String>>,
    /// What's picked on each questions card, by its ref (`QuestionSelection`, the card's state).
    selections: RefCell<HashMap<String, QuestionSelection>>,
    /// Questions cards put away to talk them through (Chat about this), by ref.
    chatting: RefCell<HashSet<String>>,
    /// The own-answer field that had the keyboard, by card ref and question, so a redraw keeps it.
    typing: RefCell<Option<(String, String)>>,
    /// What's drawn now, so a read that changed nothing draws nothing.
    drawn: RefCell<Option<Drawn>>,
    reading: Cell<bool>,
    /// Read, and the account has no Coach yet: starting it is signed, on the phone.
    no_coach: Cell<bool>,
    polls: RefCell<Polls>,
    /// The thread goes to its latest at the next draw.
    follow: Cell<bool>,
    root: gtk::Box,
    tools: gtk::Box,
    column: gtk::Box,
    plan_slot: gtk::Box,
    scroller: gtk::ScrolledWindow,
    /// ↓ and the Needs you bar over the conversation.
    jump: Rc<crate::chat::Jump>,
    error: gtk::Label,
    composer: Composer,
    /// The window is wide enough for the panel beside the conversation.
    wide: Cell<bool>,
    /// Coach's tools as tafs cloud last read them (`coachTools`), and their row above the
    /// conversation (`CoachToolsRow`).
    coach_tools: RefCell<Option<CoachTools>>,
    tools_row: gtk::Button,
    /// The files picked for the next message (`setupFiles`).
    attach: Rc<crate::attach::Attach>,
    /// "Taking shape" beside the conversation, and what it holds.
    side: gtk::Box,
    shape: gtk::Box,
    shape_drawn: RefCell<Option<tafs_core::setup_shape::ShapeLines>>,
}

/// What a draw shows, to compare with the last.
#[derive(PartialEq)]
struct Drawn {
    lines: Result<CoachLines, String>,
    unsent: Vec<UnsentLines>,
    ended: Option<String>,
    answered: Vec<String>,
    notes: Vec<(String, String)>,
    chatting: Vec<String>,
    selections: Vec<(String, QuestionSelection)>,
}

/// The page, and what it reads.
pub fn page(
    w: Rc<WordBook>,
    source: CoachSource,
    listed: Rc<Vec<Listed>>,
    open: Rc<dyn Fn(Selection)>,
    chrome: Option<crate::coach_mode::Chrome>,
) -> gtk::Widget {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("tafs-coach-wash");
    root.set_vexpand(true);
    // The inline title, as the iPad's navigation bar has it.
    let title = text(
        &w.line("coach.title").unwrap_or_default(),
        17,
        600,
        &["tafs-ink"],
    );
    title.set_xalign(0.5);
    title.set_margin_top(14);
    title.set_margin_bottom(12);
    let bar = gtk::CenterBox::new();
    bar.add_css_class("tafs-coach-title");
    bar.set_center_widget(Some(&title));
    // New conversation, and the menu with Previous conversations, at its trailing edge.
    let tools = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    tools.set_margin_end(10);
    tools.set_valign(gtk::Align::Center);
    let new_chat = gtk::Button::new();
    new_chat.add_css_class("tafs-bar-icon");
    new_chat.set_child(Some(&symbol("tafs-compose-symbolic", 18)));
    new_chat.set_tooltip_text(w.line("coach.new_conversation").ok().as_deref());
    new_chat.update_property(&[gtk::accessible::Property::Label(
        &w.line("coach.new_conversation").unwrap_or_default(),
    )]);
    new_chat.set_widget_name("coach.new");
    tools.append(&new_chat);
    let menu = gtk::MenuButton::new();
    menu.add_css_class("tafs-bar-icon");
    menu.set_icon_name("view-more-symbolic");
    menu.update_property(&[gtk::accessible::Property::Label(
        &w.line("coach.conversations").unwrap_or_default(),
    )]);
    let previous = gtk::Button::with_label(&w.line("coach.chats.previous").unwrap_or_default());
    previous.add_css_class("flat");
    let popover = gtk::Popover::new();
    popover.set_child(Some(&previous));
    menu.set_popover(Some(&popover));
    tools.append(&menu);
    tools.set_visible(false);
    bar.set_end_widget(Some(&tools));
    // In Coach mode's panel: Back, and ⤢ at the leading edge (`CoachView`).
    if let Some(chrome) = &chrome {
        bar.set_start_widget(Some(&crate::coach_mode::chrome_buttons(&w, chrome)));
    }
    root.append(&bar);
    // A wide window (`shaping`): the conversation, and beside it what it has made so far.
    let chat = gtk::Box::new(gtk::Orientation::Vertical, 0);
    chat.set_hexpand(true);
    // Coach's tools still to give: one line, which opens the step.
    let tools_row = gtk::Button::new();
    tools_row.add_css_class("tafs-tools-row");
    tools_row.set_widget_name("coach-tools-row");
    tools_row.set_visible(false);
    chat.append(&tools_row);
    let plan_slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    chat.append(&plan_slot);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    column.set_valign(gtk::Align::Start);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    scroller.add_css_class("tafs-coach-wash");
    let jump = crate::chat::Jump::new(&w, &scroller);
    chat.append(&jump.overlay);
    let error = text("", 13, 400, &["tafs-no"]);
    error.set_margin_start(16);
    error.set_margin_end(16);
    error.set_margin_bottom(4);
    error.set_visible(false);
    chat.append(&error);
    let composer = crate::chat::composer(crate::chat::Whose::Coach);
    composer.bar.set_visible(false);
    chat.append(&composer.bar);
    // Coach's files go to Coach's taf at tafs cloud, once it's read (`filesTarget`).
    let attach = crate::attach::Attach::new(w.clone(), None);
    attach.place(&composer);
    let side = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    side.append(&gtk::Separator::new(gtk::Orientation::Vertical));
    let shape = gtk::Box::new(gtk::Orientation::Vertical, 22);
    shape.set_margin_start(20);
    shape.set_margin_end(20);
    shape.set_margin_top(20);
    shape.set_margin_bottom(20);
    shape.set_valign(gtk::Align::Start);
    let shape_scroller = gtk::ScrolledWindow::new();
    shape_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    shape_scroller.set_size_request(tafs_core::setup_shape::PANEL_WIDTH, -1);
    shape_scroller.set_child(Some(&shape));
    shape_scroller.add_css_class("tafs-coach-panel");
    shape_scroller.set_widget_name("coach.shape");
    shape_scroller.set_hexpand(false);
    side.append(&shape_scroller);
    side.set_hexpand(false);
    side.set_visible(false);
    let body = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    body.set_vexpand(true);
    body.append(&chat);
    body.append(&side);
    root.append(&body);

    let page = Rc::new(Page {
        w,
        source,
        listed,
        open,
        conv: RefCell::new(Conversation::default()),
        device: RefCell::new(None),
        selections: RefCell::new(HashMap::new()),
        chatting: RefCell::new(HashSet::new()),
        typing: RefCell::new(None),
        drawn: RefCell::new(None),
        reading: Cell::new(false),
        no_coach: Cell::new(false),
        polls: RefCell::new(Polls::default()),
        follow: Cell::new(true),
        root: root.clone(),
        tools,
        column,
        plan_slot,
        scroller,
        jump,
        error,
        composer,
        wide: Cell::new(false),
        coach_tools: RefCell::new(None),
        tools_row,
        attach,
        side,
        shape,
        shape_drawn: RefCell::new(None),
    });
    // Opened for a new taf, a job handed over or a setup to finish: that conversation.
    crate::coach_start::apply(&mut page.conv.borrow_mut());
    wire_composer(&page);
    {
        let p = Rc::downgrade(&page);
        page.attach.on_change(Rc::new(move || {
            if let Some(p) = p.upgrade() {
                p.composer_state();
            }
        }));
    }
    {
        let p = Rc::downgrade(&page);
        page.tools_row.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.open_tools();
            }
        });
    }
    // The page's width (`GeometryReader`): side by side from 780, else the conversation alone.
    // Only a change of side draws again. The callback holds the page while it's on screen, and
    // goes with it.
    {
        let p = page.clone();
        let id = page.root.add_tick_callback(move |widget, _| {
            let width = widget.width();
            if width > 0 {
                let wide = width >= tafs_core::setup_shape::SIDE_BY_SIDE;
                if p.wide.replace(wide) != wide {
                    *p.drawn.borrow_mut() = None;
                    p.draw();
                }
            }
            glib::ControlFlow::Continue
        });
        let id = RefCell::new(Some(id));
        page.root.connect_unmap(move |_| {
            if let Some(id) = id.borrow_mut().take() {
                id.remove();
            }
        });
    }
    {
        let p = Rc::downgrade(&page);
        new_chat.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.new_conversation();
            }
        });
        let p = Rc::downgrade(&page);
        previous.connect_clicked(move |_| {
            popover.popdown();
            if let Some(p) = p.upgrade() {
                p.previous();
            }
        });
    }

    match page.source.clone() {
        CoachSource::None => {}
        CoachSource::File {
            thread: path,
            chats,
            drafts,
            setup,
        } => {
            if let Some(setup) = setup {
                *page.coach_tools.borrow_mut() = std::fs::read(&setup)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                    .and_then(|v| serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok())
                    .and_then(|b| CoachTools::read(&b));
            }
            let read = std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok())
                .and_then(|b| SetupThread::read(&b));
            match read {
                Some(t) => {
                    {
                        let mut c = page.conv.borrow_mut();
                        c.setup = Some(String::new());
                        c.chat = t.session();
                        c.took(t, chrono::Utc::now());
                    }
                    if let Some(drafts) = drafts {
                        page.drafts_from(&drafts);
                    }
                    page.draw();
                    if OPEN_TOOLS.with(|t| t.replace(false)) {
                        let p = page.clone();
                        glib::idle_add_local_once(move || p.open_tools());
                    }
                    if let Some(chats) = chats {
                        page.conv.borrow_mut().chats = std::fs::read(&chats)
                            .ok()
                            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                            .and_then(|v| serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok())
                            .and_then(|b| coach::chats(&b))
                            .unwrap_or_default();
                        let p = page.clone();
                        glib::idle_add_local_once(move || p.previous());
                    }
                }
                // `GET /v1/setup` with no Coach yet: its start.
                None if std::fs::read(&path)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                    .and_then(|v| serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok())
                    .is_some_and(|b| coach::setup_id(&b) == Ok(None)) =>
                {
                    page.no_coach.set(true);
                    page.draw();
                }
                None => {
                    eprintln!(
                        "{} isn't Coach's conversation as tafs cloud answers it",
                        path.display()
                    );
                    std::process::exit(2);
                }
            }
        }
        CoachSource::Cloud { .. } => {
            let p = page.clone();
            glib::spawn_future_local(async move {
                p.load_setup().await;
                p.follow_setup().await;
            });
        }
    }
    root.upcast()
}

fn wire_composer(page: &Rc<Page>) {
    let c = &page.composer;
    let buffer = c.view.buffer();
    DRAFT.with(|d| buffer.set_text(&d.borrow()));
    {
        let p = Rc::downgrade(page);
        buffer.connect_changed(move |b| {
            let words = b.text(&b.start_iter(), &b.end_iter(), false).to_string();
            DRAFT.with(|d| *d.borrow_mut() = words);
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
    // ⌘↩ sends and ⌘. stops, as on the iPad's keyboard: Ctrl here.
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
        if let CoachSource::Cloud { signed_out, .. } = &self.source {
            signed_out();
        }
    }

    /// Still on screen: the reads go on only while the page is in the window.
    fn open(&self) -> bool {
        self.root.root().is_some()
    }

    fn draft(&self) -> String {
        self.composer.draft()
    }

    fn set_draft(&self, words: &str) {
        self.composer.set_draft(words);
    }

    /// `loadSetup`: Coach's taf, then its thread.
    async fn load_setup(self: &Rc<Self>) {
        let CoachSource::Cloud {
            kept, languages, ..
        } = self.source.clone()
        else {
            return;
        };
        let words = (*self.w).clone();
        let read = gtk::gio::spawn_blocking(move || read_setup(&words, &kept, &languages))
            .await
            .unwrap_or_else(|_| Err(Fail::Why(String::new())));
        match read {
            Ok((Some(id), tools)) => {
                if let CoachSource::Cloud {
                    kept, languages, ..
                } = &self.source
                {
                    self.attach.set_target(Some(crate::attach::Target {
                        token: kept.token.clone(),
                        languages: languages.clone(),
                        taf: id.clone(),
                    }));
                }
                self.conv.borrow_mut().setup = Some(id);
                *self.coach_tools.borrow_mut() = tools;
                self.load_thread().await;
            }
            // No Coach yet: its start, which is signed, on the phone.
            Ok((None, _)) => {
                self.no_coach.set(true);
                *self.drawn.borrow_mut() = None;
                self.draw();
            }
            Err(Fail::SignedOut) => self.signed_out(),
            Err(Fail::Why(why)) => {
                self.conv.borrow_mut().error = Some(self.not_loaded(&why));
                self.draw();
            }
        }
    }

    fn not_loaded(&self, why: &str) -> String {
        self.w
            .get("coach.conversation_not_loaded", &[("why", why)])
            .unwrap_or_else(|_| why.to_string())
    }

    /// `loadSetupThread`: Coach's newest chat when none is chosen, this device's proof, then the
    /// conversation with it. A new conversation has nothing to read until its first message.
    async fn load_thread(self: &Rc<Self>) {
        let CoachSource::Cloud {
            kept, languages, ..
        } = self.source.clone()
        else {
            return;
        };
        let (setup, chat, generation) = {
            let c = self.conv.borrow();
            if c.fresh {
                return;
            }
            let Some(setup) = c.setup.clone() else { return };
            (setup, c.chat.clone(), c.generation)
        };
        if self.reading.replace(true) {
            return;
        }
        let words = (*self.w).clone();
        let device = self.device.borrow().clone();
        let read = gtk::gio::spawn_blocking(move || {
            read_thread(&words, &kept, &languages, &setup, chat, device)
        })
        .await
        .unwrap_or_else(|_| Err(Fail::Why(String::new())));
        self.reading.set(false);
        if !self.open() {
            return;
        }
        match read {
            Err(Fail::SignedOut) => return self.signed_out(),
            Ok(r) => {
                if r.device.is_some() {
                    *self.device.borrow_mut() = r.device;
                }
                let mut c = self.conv.borrow_mut();
                if c.generation != generation || c.fresh {
                    return;
                }
                if let Some(chats) = r.chats {
                    c.chats = chats;
                }
                if c.chat.is_none() {
                    c.chat = r.chat;
                }
                c.took(r.thread, chrono::Utc::now());
            }
            Err(Fail::Why(why)) => {
                let mut c = self.conv.borrow_mut();
                if c.generation != generation {
                    return;
                }
                c.error = Some(self.not_loaded(&why));
            }
        }
        self.draw();
        // What an approval needs to be shown: KIFF's draft, read again until it's confirmed.
        let wanted = self.conv.borrow().drafts_to_read();
        if !wanted.is_empty() {
            self.read_drafts(wanted).await;
            self.draw();
        }
    }

    /// A screenshot's drafts, from its file: each KIFF's answer, as a read would take it.
    fn drafts_from(&self, path: &std::path::Path) {
        let Some(map) = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| v.as_object().cloned())
        else {
            eprintln!("{} isn't KIFF's drafts by id", path.display());
            std::process::exit(2);
        };
        for (id, a) in map {
            let answer = Answer {
                status: a.get("status").and_then(|s| s.as_u64()).unwrap_or(200) as u16,
                body: serde_json::to_vec(a.get("answer").unwrap_or(&a)).unwrap_or_default(),
            };
            let read = if (200..300).contains(&answer.status) {
                tafs_core::drafts::AgentDraft::read(&answer.body)
                    .ok_or_else(|| tafs_core::signin::plain_words(&self.w, 200).unwrap_or_default())
            } else {
                Err(refusal(&self.w, &answer))
            };
            self.conv.borrow_mut().took_draft(&id, read);
        }
    }

    /// `readCardDraft`, for each: KIFF's draft as the session, or KIFF's reason kept for the card.
    /// A screenshot's drafts come from its file, and nothing is read.
    async fn read_drafts(self: &Rc<Self>, ids: Vec<String>) {
        let CoachSource::Cloud {
            kept, languages, ..
        } = self.source.clone()
        else {
            return;
        };
        for id in ids {
            let words = (*self.w).clone();
            let (token, languages) = (kept.token.clone(), languages.clone());
            let read_id = id.clone();
            let read =
                gtk::gio::spawn_blocking(move || read_draft(&words, &token, &languages, &read_id))
                    .await
                    .unwrap_or_else(|_| Err(Fail::Why(String::new())));
            if !self.open() {
                return;
            }
            match read {
                Ok(d) => self.conv.borrow_mut().took_draft(&id, Ok(d)),
                Err(Fail::SignedOut) => return self.signed_out(),
                Err(Fail::Why(why)) => self.conv.borrow_mut().took_draft(&id, Err(why)),
            }
        }
    }

    /// `followSetup`: the thread read again while the page is open, every second while Coach
    /// works, else every three, for five minutes after the owner last wrote. A read never
    /// overlaps another; leaving the page ends it.
    async fn follow_setup(self: &Rc<Self>) {
        loop {
            let wait = Polls::interval(&self.conv.borrow());
            glib::timeout_future(wait).await;
            if !self.open() {
                return;
            }
            if self.reading.get() {
                continue;
            }
            let read = self.polls.borrow_mut().read_now(&self.conv.borrow());
            if read {
                self.load_thread().await;
            }
        }
    }

    /// A send to tafs cloud as the session, off the main thread. A screenshot's sends nothing.
    async fn post(&self, path: String, body: Vec<u8>) -> Option<Sent> {
        let CoachSource::Cloud {
            kept, languages, ..
        } = &self.source
        else {
            return None;
        };
        Some(crate::chat::post(&self.w, &kept.token, languages, path, body).await)
    }

    /// Coach's answer's menu (`reportable(_:in: nil)`): Report this answer, by Coach's taf.
    fn reportable(self: &Rc<Self>, on: &gtk::Box, r: &tafs_core::report::Reported) {
        let to = match &self.source {
            CoachSource::Cloud {
                kept,
                languages,
                signed_out,
            } => crate::report_sheet::ReportTo::Cloud {
                token: kept.token.clone(),
                languages: languages.clone(),
                signed_out: signed_out.clone(),
            },
            _ => crate::report_sheet::ReportTo::Nowhere,
        };
        let Some(taf) = self.conv.borrow().setup.clone() else {
            return;
        };
        let (w, r, at) = (self.w.clone(), r.clone(), on.clone());
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

    /// The AI-data consent's screen, for a turn tafs cloud refused for want of it.
    fn consent(&self, refusal: &[u8]) {
        let (
            CoachSource::Cloud {
                kept, languages, ..
            },
            Some(window),
        ) = (&self.source, self.root.root())
        else {
            return;
        };
        crate::ai_consent::required(
            self.w.clone(),
            crate::ai_consent::Source::Cloud {
                token: kept.token.clone(),
                languages: languages.clone(),
            },
            refusal,
            window.upcast(),
        );
    }

    /// Send (`send`, `submitSetup`): the words leave the box; a message goes now, or, while Coach
    /// works, to its run. Words that weren't taken go back in the box.
    fn send(self: &Rc<Self>) {
        if matches!(self.source, CoachSource::File { .. }) {
            return;
        }
        let words = self.draft();
        // A message with files goes as a message of its own (`deliverSetup(text, files:)`).
        let taken = self.attach.take();
        if !taken.is_empty() {
            let refs: Vec<_> = taken
                .iter()
                .filter_map(|f| f.attachment().cloned())
                .collect();
            let delivery = self
                .conv
                .borrow_mut()
                .deliver_with(&words, refs, chrono::Utc::now());
            let Some(d) = delivery else {
                self.attach.put_back(taken);
                return;
            };
            self.set_draft("");
            self.follow.set(true);
            self.draw();
            let p = self.clone();
            glib::spawn_future_local(async move {
                if !p.deliver(d).await {
                    if p.draft().is_empty() {
                        p.set_draft(&words);
                    }
                    p.attach.put_back(taken);
                }
            });
            return;
        }
        let submit = self.conv.borrow_mut().submit(&words, chrono::Utc::now());
        match submit {
            Submit::Stay => {}
            Submit::Send(d) => {
                self.set_draft("");
                self.follow.set(true);
                self.draw();
                let p = self.clone();
                glib::spawn_future_local(async move {
                    if !p.deliver(d).await && p.draft().is_empty() {
                        p.set_draft(&words);
                    }
                });
            }
            Submit::Steer { run, text } => {
                self.set_draft("");
                let p = self.clone();
                glib::spawn_future_local(async move {
                    if !p.steer(&run, &text).await && p.draft().is_empty() {
                        p.set_draft(&words);
                    }
                });
            }
        }
    }

    /// `deliverSetupOnce`'s send: true once the message is taken (sent, or kept as not sent).
    async fn deliver(self: &Rc<Self>, d: Delivery) -> bool {
        let Some(setup) = self.conv.borrow().setup.clone() else {
            return false;
        };
        let body = coach_send::message_body(&d.text, &d.target, &d.files);
        let Some(answer) = self.post(coach_send::messages_path(&setup), body).await else {
            return false;
        };
        if self.conv.borrow().generation != d.generation {
            return false;
        }
        let now = chrono::Utc::now();
        let taken = match answer {
            Ok(a) if a.status == 401 => {
                self.signed_out();
                return false;
            }
            Ok(a) if (200..300).contains(&a.status) => match coach_send::sent(&a.body) {
                Some(sent) => {
                    self.conv.borrow_mut().delivered(&d, &sent, now);
                    self.draw();
                    self.load_thread().await;
                    return true;
                }
                None => {
                    let why = tafs_core::signin::plain_words(&self.w, 200).unwrap_or_default();
                    self.conv.borrow_mut().not_sent(&d, &why, None, now);
                    true
                }
            },
            Ok(a) => {
                let r = Refusal::read(a.status, &a.body);
                if r.wants_consent() {
                    // Its screen says why; the words go back in the box, with no Not sent row.
                    self.conv.borrow_mut().not_tried(&d, None);
                    self.consent(&a.body);
                    false
                } else if let Some(words) = r.waits_for_credits(&self.w).ok().flatten() {
                    self.conv.borrow_mut().not_tried(&d, Some(words));
                    false
                } else {
                    let why = r.words(&self.w).unwrap_or_default();
                    self.conv
                        .borrow_mut()
                        .not_sent(&d, &why, r.halted.clone(), now);
                    true
                }
            }
            Err(why) => {
                self.conv.borrow_mut().not_sent(&d, &why, None, now);
                true
            }
        };
        self.draw();
        taken
    }

    /// Written to Coach's run while it works (`steer`): true once taken, held or kept as not sent.
    async fn steer(self: &Rc<Self>, run: &str, words: &str) -> bool {
        let (Some(setup), generation) = ({
            let c = self.conv.borrow();
            (c.setup.clone(), c.generation)
        }) else {
            return false;
        };
        let Some(answer) = self
            .post(
                coach_send::steer_path(&setup),
                coach_send::steer_body(run, words),
            )
            .await
        else {
            return false;
        };
        if self.conv.borrow().generation != generation {
            return true;
        }
        let now = chrono::Utc::now();
        let taken = match answer {
            Ok(a) if a.status == 401 => {
                self.signed_out();
                return false;
            }
            Ok(a) if (200..300).contains(&a.status) => match coach_send::steered(&a.body) {
                Some(s) => {
                    self.conv.borrow_mut().steered(run, words, &s, now);
                    self.draw();
                    if matches!(s, Steered::Kept { sent: Some(_), .. }) {
                        self.load_thread().await;
                    }
                    return true;
                }
                None => {
                    let why = tafs_core::signin::plain_words(&self.w, 200).unwrap_or_default();
                    self.conv
                        .borrow_mut()
                        .steer_not_sent(words, &why, None, now);
                    true
                }
            },
            Ok(a) => {
                let r = Refusal::read(a.status, &a.body);
                if r.code.as_deref() == Some("held_full") {
                    // More than tafs cloud holds: nothing is cut, the words stay, with its why.
                    self.conv.borrow_mut().error = Some(
                        r.error
                            .clone()
                            .unwrap_or_else(|| r.words(&self.w).unwrap_or_default()),
                    );
                    false
                } else if let Some(words) = r.waits_for_credits(&self.w).ok().flatten() {
                    self.conv.borrow_mut().error = Some(words);
                    false
                } else if r.wants_consent() {
                    self.consent(&a.body);
                    false
                } else {
                    let why = r.words(&self.w).unwrap_or_default();
                    self.conv
                        .borrow_mut()
                        .steer_not_sent(words, &why, r.halted.clone(), now);
                    true
                }
            }
            Err(why) => {
                self.conv
                    .borrow_mut()
                    .steer_not_sent(words, &why, None, now);
                true
            }
        };
        self.draw();
        taken
    }

    /// Stop (`stopSetup`): Coach's current run is cancelled, then the thread read again.
    fn stop(self: &Rc<Self>) {
        if matches!(self.source, CoachSource::File { .. }) {
            return;
        }
        let (Some(session), Some(setup)) = ({
            let mut c = self.conv.borrow_mut();
            (c.stop(), c.setup.clone())
        }) else {
            return;
        };
        self.draw();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let Some(answer) = p
                .post(
                    coach_send::stop_path(&setup),
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
                let mut c = p.conv.borrow_mut();
                match failed {
                    None => c.ended = Some(coach_send::Ended::Stopped),
                    Some(why) => {
                        c.error = Some(
                            p.w.get("work.did_not_stop", &[("why", &why)])
                                .unwrap_or(why),
                        )
                    }
                }
            }
            p.draw();
            p.load_thread().await;
        });
    }

    /// A card answered as data (`answer`): marked here at once, then the run it let go on is
    /// followed; a card that doesn't wait any more (409) is read again, and any other failure
    /// says so under the card.
    fn answer(self: &Rc<Self>, id: &str, kind: Kind, words: &str) {
        if matches!(self.source, CoachSource::File { .. }) {
            return;
        }
        let (Some(send), generation) = ({
            let mut c = self.conv.borrow_mut();
            (c.answer(id, kind, words), c.generation)
        }) else {
            return;
        };
        self.draw();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let CardSend {
                setup, id, body, ..
            } = send.clone();
            let Some(answer) = p.post(coach_send::answer_path(&setup, &id), body).await else {
                return;
            };
            if p.conv.borrow().generation != generation {
                return;
            }
            let note = p.w.line("coach.answer_did_not_reach_setup").ok();
            match answer {
                Ok(a) if a.status == 401 => return p.signed_out(),
                Ok(a) if (200..300).contains(&a.status) => {
                    match coach_send::card_answered(&a.body) {
                        Some(r) => {
                            p.conv.borrow_mut().answer_taken(&r, chrono::Utc::now());
                        }
                        None => p.conv.borrow_mut().answer_not_taken(&send, note),
                    }
                }
                Ok(a) if a.status == 409 => p.conv.borrow_mut().answer_not_taken(&send, None),
                Ok(a) => {
                    let r = Refusal::read(a.status, &a.body);
                    if r.wants_consent() {
                        // Its screen says why; the card stays as it was, its answers as chosen.
                        p.conv.borrow_mut().answer_not_taken(&send, None);
                        p.consent(&a.body);
                    } else {
                        p.conv.borrow_mut().answer_not_taken(&send, note);
                    }
                }
                Err(_) => p.conv.borrow_mut().answer_not_taken(&send, note),
            }
            p.draw();
            p.load_thread().await;
        });
    }

    /// Retry on a message that didn't go (`retrySetup`): sent once more; not taken, it goes back.
    fn retry(self: &Rc<Self>, id: u64) {
        if matches!(self.source, CoachSource::File { .. }) {
            return;
        }
        let Some((at, message)) = self.conv.borrow_mut().retry(id) else {
            self.draw();
            return;
        };
        let submit = self
            .conv
            .borrow_mut()
            .submit(&message.text, chrono::Utc::now());
        self.draw();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let taken = match submit {
                Submit::Stay => false,
                Submit::Send(d) => p.deliver(d).await,
                Submit::Steer { run, text } => p.steer(&run, &text).await,
            };
            if !taken {
                p.conv.borrow_mut().retry_back(at, message);
                p.draw();
            }
        });
    }

    /// Delete (`discardUnsent`): out of the thread, not sent.
    fn discard(self: &Rc<Self>, id: u64) {
        self.conv.borrow_mut().discard(id);
        self.draw();
    }

    /// Edit (`editUnsent`): out of the thread, its words back in the box.
    fn edit(self: &Rc<Self>, id: u64) {
        let words = self.conv.borrow_mut().edit(id, &self.w);
        if let Some(words) = words {
            self.set_draft(&words);
            self.composer.view.grab_focus();
        }
        self.draw();
    }

    /// The box's placeholder and button: Stop while Coach works and nothing is written.
    fn composer_state(&self) {
        let c = self.conv.borrow();
        let thinking = c.thinking();
        let Ok(ComposerLines {
            placeholder,
            send,
            stop,
        }) = tafs_core::coach_lines::composer(&self.w, thinking)
        else {
            return;
        };
        self.composer.bar.set_visible(c.setup.is_some());
        self.tools.set_visible(c.setup.is_some());
        let (ready, held) = self.attach.send_state(&send, thinking);
        self.composer.show_with(
            &placeholder,
            &send,
            &stop,
            thinking,
            Some((ready, held.as_deref())),
        );
    }

    /// `CoachToolsRow`: tafs cloud's line, and a chevron, above the conversation while Coach's
    /// tools are still to give; nothing once they're given, or before tafs cloud has said.
    fn draw_tools_row(&self) {
        let shown = self.conv.borrow().setup.is_some();
        let words = tafs_core::coach_tools::row(&self.w, self.coach_tools.borrow().as_ref())
            .ok()
            .flatten()
            .filter(|_| shown);
        let Some(words) = words else {
            self.tools_row.set_visible(false);
            return;
        };
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let l = one_line(&words, 14, 600, &["tafs-amber-text"]);
        l.set_hexpand(true);
        h.append(&l);
        let chevron = symbol("go-next-symbolic", 13);
        chevron.add_css_class("tafs-dim");
        chevron.set_accessible_role(gtk::AccessibleRole::Presentation);
        h.append(&chevron);
        self.tools_row.set_child(Some(&h));
        self.tools_row.set_visible(true);
    }

    /// `CoachToolsSheet`: the step in tafs cloud's words, each tool under a few groups and folded,
    /// Coach's Cards in KIFF's words, then the one confirmation that gives them: signed on the
    /// iPad, behind `ApprovalGate`, so here the gate's line stands in its place and nothing is
    /// sent. Not now closes it.
    fn open_tools(self: &Rc<Self>) {
        let Some(tools) = self.coach_tools.borrow().clone() else {
            return;
        };
        let Ok(l) = tafs_core::coach_tools::sheet(&self.w, &tools) else {
            return;
        };
        let said = |s: &tafs_core::coach_tools::Said| {
            text(
                &s.words,
                s.size,
                s.weight,
                &[if s.soft { "tafs-soft" } else { "tafs-ink" }],
            )
        };
        let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
        column.set_margin_start(20);
        column.set_margin_end(20);
        column.set_margin_top(20);
        column.set_margin_bottom(20);
        for s in &l.lines {
            column.append(&said(s));
        }
        if let Some(details) = &l.details {
            let fold = gtk::Box::new(gtk::Orientation::Vertical, 2);
            for s in &l.folded {
                fold.append(&said(s));
            }
            fold.set_visible(false);
            let label = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            label.append(&text(details, 14, 600, &["tafs-amber-text"]));
            let chevron = symbol("pan-down-symbolic", 12);
            chevron.add_css_class("tafs-amber-text");
            label.append(&chevron);
            let b = gtk::Button::new();
            b.add_css_class("tafs-plain");
            b.set_child(Some(&label));
            b.set_halign(gtk::Align::Start);
            b.set_widget_name("coach-tools-details");
            {
                let fold = fold.clone();
                b.connect_clicked(move |_| {
                    let open = !fold.is_visible();
                    fold.set_visible(open);
                    chevron.set_icon_name(Some(if open {
                        "pan-up-symbolic"
                    } else {
                        "pan-down-symbolic"
                    }));
                });
            }
            column.append(&b);
            column.append(&fold);
        }
        if let Some(ask) = &l.ask {
            let a = said(ask);
            a.set_margin_top(6);
            column.append(&a);
        }
        if l.gives {
            // The one confirmation, the person's: why this computer can't, in its place.
            let give = primary(
                &tools
                    .step
                    .as_ref()
                    .map(|s| s.give.clone())
                    .filter(|g| !g.trim().is_empty())
                    .unwrap_or_else(|| self.w.line("coach.tools.give").unwrap_or_default()),
            );
            give.set_widget_name("coach-tools-give");
            column.append(&crate::approval::gate(&self.w, Mode::Kiff, &give));
        }
        let not_now = outline(&l.not_now);
        not_now.set_widget_name("coach-tools-not-now");
        column.append(&not_now);
        let scroller = gtk::ScrolledWindow::new();
        scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        scroller.set_vexpand(true);
        scroller.set_child(Some(&column));
        let header = adw::HeaderBar::new();
        let toolbar = adw::ToolbarView::new();
        toolbar.add_top_bar(&header);
        toolbar.set_content(Some(&scroller));
        crate::widgets::ground(&toolbar);
        let dialog = adw::Dialog::new();
        dialog.set_title(&l.title);
        dialog.set_content_width(520);
        dialog.set_content_height(640);
        dialog.set_child(Some(&toolbar));
        {
            let d = dialog.downgrade();
            not_now.connect_clicked(move |_| {
                if let Some(d) = d.upgrade() {
                    d.close();
                }
            });
        }
        dialog.present(Some(&self.root));
    }

    /// `shaping`: a wide window with Coach's conversation shows what it has made so far beside
    /// it (`SetupShapePanel`), drawn again only when it changed. Whether it shows.
    fn draw_shape(&self) -> bool {
        let shaping = self.wide.get() && self.conv.borrow().setup.is_some();
        self.side.set_visible(shaping);
        if !shaping {
            return false;
        }
        let Ok(lines) = tafs_core::setup_shape::SetupShape::new(&self.w, &self.conv.borrow())
            .and_then(|s| tafs_core::setup_shape::lines(&self.w, &s))
        else {
            return true;
        };
        if self.shape_drawn.borrow().as_ref() == Some(&lines) {
            return true;
        }
        while let Some(c) = self.shape.first_child() {
            self.shape.remove(&c);
        }
        shape_panel(&self.shape, &lines);
        *self.shape_drawn.borrow_mut() = Some(lines);
        true
    }

    /// Draws what the conversation holds now, unless it's what's drawn already.
    fn draw(self: &Rc<Self>) {
        self.composer_state();
        let c = self.conv.borrow();
        match &c.error {
            Some(why) => {
                self.error.set_label(why);
                self.error.set_visible(true);
            }
            None => self.error.set_visible(false),
        }
        let w = &self.w;
        let lines = tafs_core::coach_lines::conversation_lines(
            w,
            &c,
            &self.listed,
            Mode::Kiff,
            chrono::Utc::now(),
        )
        .map_err(|e| e.to_string());
        let unsent = tafs_core::coach_lines::unsent(w, &c.thread, &c.unsent).unwrap_or_default();
        let ended = if c.thinking() {
            None
        } else {
            tafs_core::coach_lines::ended(w, c.ended).ok().flatten()
        };
        let mut answered: Vec<String> = c.answered.iter().cloned().collect();
        answered.sort();
        let mut notes: Vec<(String, String)> = c
            .notes
            .iter()
            .map(|(a, b)| (a.clone(), b.clone()))
            .collect();
        notes.sort();
        let mut chatting: Vec<String> = self.chatting.borrow().iter().cloned().collect();
        chatting.sort();
        let mut selections: Vec<(String, QuestionSelection)> = self
            .selections
            .borrow()
            .iter()
            .map(|(r, s)| (r.clone(), s.clone()))
            .collect();
        selections.sort_by(|a, b| a.0.cmp(&b.0));
        let now = Drawn {
            lines,
            unsent,
            ended,
            answered,
            notes,
            chatting,
            selections,
        };
        // The card waiting for the owner (`setupWaiting`), for its bar.
        let waiting = tafs_core::setup_shape::waiting(&c).and_then(|s| {
            tafs_core::coach_lines::needs_you_bar(w, &s.component)
                .ok()
                .map(|words| (s.card_ref(), words))
        });
        let ready = !c.thread.items().is_empty();
        drop(c);
        self.draw_tools_row();
        let shaping = self.draw_shape();
        if self.drawn.borrow().as_ref() == Some(&now) {
            return;
        }
        let keep = self.scroller.vadjustment().value();
        let before = self
            .drawn
            .borrow()
            .as_ref()
            .and_then(|d| d.lines.as_ref().ok().map(|l| l.order.len()));
        self.jump.hold();
        while let Some(child) = self.column.first_child() {
            self.column.remove(&child);
        }
        while let Some(child) = self.plan_slot.first_child() {
            self.plan_slot.remove(&child);
        }
        match &now.lines {
            Ok(lines) => {
                // Beside the conversation, the plan is in the panel.
                if let Some(plan) = lines.plan.as_ref().filter(|_| !shaping) {
                    let p = plan_panel(plan);
                    p.set_margin_start(12);
                    p.set_margin_end(12);
                    p.set_margin_top(8);
                    self.plan_slot.append(&p);
                }
                draw_thread(self, lines, &now);
            }
            Err(e) => {
                self.error.set_label(e);
                self.error.set_visible(true);
            }
        }
        // What's drawn keeps its place from now on (`rememberDrawn`).
        if let Ok(lines) = &now.lines {
            self.conv.borrow_mut().drawn = lines.order.clone();
        }
        let grew = matches!(
            (before, now.lines.as_ref().ok().map(|l| l.order.len())),
            (Some(b), Some(n)) if n > b
        );
        *self.drawn.borrow_mut() = Some(now);
        // The bar for the card that waits, while it's out of view (`NeedsYouBar`).
        let card = waiting.as_ref().and_then(|(r, words)| {
            named(self.column.upcast_ref(), &format!("card.{r}")).map(|c| (words.as_str(), c))
        });
        self.jump.waiting(card);
        // Something new below while the owner reads back: ↓'s dot.
        if grew {
            self.jump.arrived();
        }
        // The thread follows what arrives while its latest is in view (a message sent brings it
        // there), and lands first on the card waiting, or at the latest.
        if self.follow.replace(false) {
            self.jump.following.set(true);
        }
        let jump = self.jump.clone();
        glib::idle_add_local_once(move || jump.put(keep, ready, false));
    }
}

/// The widget named `name` under `root`, if it's drawn.
pub(crate) fn named(root: &gtk::Widget, name: &str) -> Option<gtk::Widget> {
    let mut stack = vec![root.clone()];
    while let Some(w) = stack.pop() {
        if w.widget_name() == name {
            return Some(w);
        }
        stack.extend(std::iter::successors(w.first_child(), |c| c.next_sibling()));
    }
    None
}

/// `setupRead`'s Coach: its id at tafs cloud, or none before it's started.
fn read_setup(
    w: &WordBook,
    kept: &KeptSession,
    languages: &[&'static str],
) -> Result<(Option<String>, Option<CoachTools>), Fail> {
    let a = crate::kiff::cloud(&kept.token, languages, coach::SETUP_PATH)
        .map_err(|e| Fail::Why(e.words(w).unwrap_or_default()))?;
    if a.status == 401 {
        return Err(Fail::SignedOut);
    }
    if !(200..300).contains(&a.status) {
        return Err(Fail::Why(refusal(w, &a)));
    }
    let id = coach::setup_id(&a.body)
        .map_err(|_| Fail::Why(tafs_core::signin::plain_words(w, 200).unwrap_or_default()))?;
    // Coach's tools as KIFF has them, from the same read (`setupRead().tools`).
    Ok((id, CoachTools::read(&a.body)))
}

/// What a thread read brought: the chats when they were read, the one chosen, and the thread.
struct ThreadRead {
    chats: Option<Vec<coach::Chat>>,
    chat: Option<String>,
    thread: SetupThread,
    device: Option<String>,
}

/// `loadSetupThread`'s reads: with no chat chosen yet, Coach's chats and the newest; this
/// device's proof; then the conversation with it. Without a key or this computer's id in KIFF,
/// it reads without the proof, as the iPad does; tafs cloud then marks no card.
fn read_thread(
    w: &WordBook,
    kept: &KeptSession,
    languages: &[&'static str],
    setup: &str,
    chat: Option<String>,
    device: Option<String>,
) -> Result<ThreadRead, Fail> {
    let token = &kept.token;
    let mut chats = None;
    let mut chosen = chat.clone();
    if chat.is_none() {
        // Coach opens on its newest conversation alone; without the list, the thread reads as before.
        if let Some(read) = crate::kiff::cloud(token, languages, &coach::chats_path(setup))
            .ok()
            .filter(|a| (200..300).contains(&a.status))
            .and_then(|a| coach::chats(&a.body))
        {
            chosen = coach::newest(&read).map(|c| c.session.clone());
            chats = Some(read);
        }
    }
    let device = device.or_else(|| this_device(kept, languages));
    let proof = device.as_deref().and_then(|d| proof(kept, d));
    // This computer signs no step beyond reading: it never asks for `can`.
    let path = coach::conversation_path(setup, chosen.as_deref(), proof.is_some(), &[]);
    let headers: Vec<(&str, &str)> = proof
        .as_deref()
        .map(|p| vec![(tafs_core::planque::HEADER, p)])
        .unwrap_or_default();
    let a = crate::kiff::cloud_with(token, languages, &path, &headers)
        .map_err(|e| Fail::Why(e.words(w).unwrap_or_default()))?;
    if a.status == 401 {
        return Err(Fail::SignedOut);
    }
    if !(200..300).contains(&a.status) {
        return Err(Fail::Why(refusal(w, &a)));
    }
    let thread = SetupThread::read(&a.body)
        .ok_or_else(|| Fail::Why(tafs_core::signin::plain_words(w, 200).unwrap_or_default()))?;
    Ok(ThreadRead {
        chats,
        chat: chosen,
        thread,
        device,
    })
}

/// `kiff.draft(id:)`: KIFF's draft as the session, unsigned. Why it wasn't read, in KIFF's words
/// or the app's.
fn read_draft(
    w: &WordBook,
    token: &str,
    languages: &[&'static str],
    id: &str,
) -> Result<tafs_core::drafts::AgentDraft, Fail> {
    let call = tafs_core::drafts::draft_read(id).ok_or(Fail::Why(String::new()))?;
    let a = crate::kiff::kiff_get(token, languages, call.path())
        .map_err(|e| Fail::Why(e.words(w).unwrap_or_default()))?;
    if a.status == 401 {
        return Err(Fail::SignedOut);
    }
    if !(200..300).contains(&a.status) {
        return Err(Fail::Why(refusal(w, &a)));
    }
    tafs_core::drafts::AgentDraft::read(&a.body)
        .ok_or_else(|| Fail::Why(tafs_core::signin::plain_words(w, 200).unwrap_or_default()))
}

/// tafs cloud's words for a refusal, or the app's plain words for what went wrong.
fn refusal(w: &WordBook, a: &Answer) -> String {
    tafs_core::signin::refusal(a, Some(w))
        .words(w)
        .unwrap_or_default()
}

/// This computer's id in KIFF (`thisDeviceID`), for a session with a key and a tenant.
pub(crate) fn this_device(kept: &KeptSession, languages: &[&'static str]) -> Option<String> {
    kept.tenant.as_deref().filter(|t| !t.is_empty())?;
    kept.key()?;
    let a = crate::kiff::devices(&kept.token, languages).ok()?;
    if !(200..300).contains(&a.status) {
        return None;
    }
    tafs_core::devices::this_device_id(&a.body)
}

/// This device's signed read of its grants (`grantsReadProof`): only for a session with a key and
/// a tenant, once KIFF names this computer. None otherwise.
pub(crate) fn proof(kept: &KeptSession, device: &str) -> Option<String> {
    let tenant = kept.tenant.as_deref().filter(|t| !t.is_empty())?;
    let key = kept.key()?;
    Some(tafs_core::planque::grants_read_proof(
        &key,
        device,
        tenant,
        tafs_core::planque::PLANQUE,
    ))
}

fn tone_class(t: Tone) -> &'static str {
    match t {
        Tone::Ink => "tafs-ink",
        Tone::Soft => "tafs-soft",
        Tone::Dim | Tone::Line => "tafs-dim",
        Tone::Ok => "tafs-ok",
        Tone::No => "tafs-no",
        Tone::Amber => "tafs-amber-text",
    }
}

/// A panel with its edge in a state's colour (`Panel(stroke:)`).
fn card_panel(stroke: Tone) -> gtk::Box {
    let p = panel(false);
    match stroke {
        Tone::Amber => p.add_css_class("edge-amber"),
        Tone::Ok => p.add_css_class("edge-ok"),
        Tone::No => p.add_css_class("edge-no"),
        Tone::Ink => p.add_css_class("edge-ink"),
        _ => {}
    }
    p
}

/// `Eyebrow(text:color:)`.
fn eyebrow_in(s: &str, tone: Tone) -> gtk::Label {
    let l = eyebrow(s, tone == Tone::Amber);
    match tone {
        Tone::Ok => l.add_css_class("ok"),
        Tone::No => l.add_css_class("tafs-no"),
        _ => {}
    }
    l
}

fn draw_thread(page: &Rc<Page>, lines: &CoachLines, drawn: &Drawn) {
    let column = &page.column;
    if page.no_coach.get() {
        column.append(&intro(&page.w));
        return;
    }
    if let Some((title, line)) = &lines.starts {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 12);
        v.set_margin_top(16);
        // Board 7, Coach with no taf or one yet: room for more.
        if page.listed.len() <= 1 {
            v.append(&crate::illustrations::coach_first().leading());
        }
        v.append(&text(title, 26, 700, &["tafs-ink"]));
        v.append(&text(line, 15, 400, &["tafs-soft"]));
        column.append(&v);
    }
    for line in &lines.lines {
        column.append(&match line {
            Line::Gap(at) => crate::chat::gap(at),
            Line::Owner {
                text: words,
                answered,
                files,
            } => {
                let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
                // Files sent alone have no words: their chips stand for the message.
                if !words.is_empty() || files.is_empty() {
                    v.append(&owner(words, answered.as_deref(), false));
                }
                if !files.is_empty() {
                    let attach = page.attach.clone();
                    v.append(&crate::chat::message_files(
                        &page.w,
                        files,
                        Rc::new(move |f, chip| attach.open_sent(f, chip)),
                    ));
                }
                v.upcast()
            }
            Line::Coach {
                work,
                text: words,
                report,
            } => {
                let v = crate::chat::answer(work.as_ref(), words);
                // Its menu: Report this answer, where it can be reported.
                if let Some(r) = report {
                    page.reportable(&v, r);
                }
                v.upcast()
            }
            Line::Card { card_ref, card } => {
                let c = card_view(page, card_ref, card);
                c.set_widget_name(&format!("card.{card_ref}"));
                c
            }
            Line::Drive {
                eyebrow: e,
                task,
                steps,
                none,
                ended,
            } => {
                let p = panel(false);
                p.append(&eyebrow_in(e, Tone::Amber));
                p.append(&text(task, 17, 600, &["tafs-ink"]));
                if let Some(n) = none {
                    p.append(&text(n, 14, 400, &["tafs-soft"]));
                }
                for step in steps {
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                    let dot = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    dot.add_css_class("tafs-step-dot");
                    dot.set_size_request(5, 5);
                    dot.set_valign(gtk::Align::Start);
                    dot.set_margin_top(8);
                    row.append(&dot);
                    let l = text(step, 14, 400, &["tafs-ink"]);
                    l.set_hexpand(true);
                    row.append(&l);
                    p.append(&row);
                }
                p.append(&text(ended, 13, 400, &["tafs-soft"]));
                p.upcast()
            }
        });
    }
    if let Some((title, said)) = &lines.away {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
        b.add_css_class("tafs-away");
        b.append(&text(title, 14, 600, &["tafs-ink"]));
        for s in said {
            b.append(&text(s, 13, 400, &["tafs-soft"]));
        }
        column.append(&b);
    }
    for said in &lines.coach_lines {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let arrow = text("↳", 11, 400, &["tafs-dim"]);
        arrow.set_valign(gtk::Align::Start);
        arrow.set_accessible_role(gtk::AccessibleRole::Presentation);
        h.append(&arrow);
        let l = text(said, 13, 400, &["tafs-soft"]);
        l.set_hexpand(true);
        h.append(&l);
        column.append(&h);
    }
    let busy = page.conv.borrow().sending;
    for u in &drawn.unsent {
        column.append(&unsent(page, u, busy));
    }
    // Coach at work: the dots where its words will be.
    if let Some(answering) = &lines.thinking {
        column.append(&crate::chat::dots(answering));
    }
    // How the last run ended, when the owner stopped it.
    if let Some(stopped) = &drawn.ended {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let m = symbol("media-playback-stop-symbolic", 13);
        m.add_css_class("tafs-dim");
        m.set_accessible_role(gtk::AccessibleRole::Presentation);
        h.append(&m);
        h.append(&text(stopped, 13, 400, &["tafs-dim"]));
        column.append(&h);
    }
    for (words, note) in &lines.steering {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
        v.append(&owner(words, None, true));
        let n = text(note, 12, 400, &["tafs-dim"]);
        n.set_xalign(1.0);
        v.append(&n);
        column.append(&v);
    }
    // tafs cloud holds it, and sends it once Coach is done (`QueuedBubble`).
    if let Some((words, waits)) = &lines.queued {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let b = owner(words, None, false);
        b.add_css_class("tafs-unsent");
        v.append(&b);
        let n = text(waits, 13, 400, &["tafs-dim"]);
        n.set_xalign(1.0);
        v.append(&n);
        column.append(&v);
    }
}

/// Coach before it's started (`introBefore`): board 9, "Coach mode", and, where Start would be,
/// why this computer can't: starting Coach is signed, so it's done on the phone.
fn intro(w: &WordBook) -> gtk::Widget {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 14);
    v.set_margin_top(24);
    v.append(&crate::illustrations::coach_starting());
    v.append(&text(
        &w.line("coach.coach_mode").unwrap_or_default(),
        30,
        700,
        &["tafs-coach-text"],
    ));
    if let Ok(tafs_core::approval::Gate::NotReady(n)) =
        tafs_core::approval::gate(w, Mode::Kiff, tafs_core::approval::THIS_COMPUTER)
    {
        v.append(&crate::approval::not_ready(&n));
    }
    v.upcast()
}

/// An unsent message's menu (`contextMenu`), on a right click or a long press: Copy, the message
/// as it was written, and Delete, out of the thread.
pub(crate) fn unsent_menu(
    at: &gtk::Box,
    copy: &str,
    text: &str,
    delete: &str,
    discard: impl Fn() + 'static,
) {
    let (widget, words) = (at.clone(), text.to_owned());
    crate::chat::menu_on(
        at,
        vec![
            (
                copy.to_owned(),
                Rc::new(move || widget.clipboard().set_text(&words)) as Rc<dyn Fn()>,
            ),
            (delete.to_owned(), Rc::new(discard)),
        ],
    );
}

/// `UnsentBubble`: the words dimmed, "Not sent · why", then Edit and Retry; for a send refused
/// because the account is halted, Resume (its Halt screen) and Retry.
fn unsent(page: &Rc<Page>, u: &UnsentLines, busy: bool) -> gtk::Widget {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let b = owner(&u.words, None, false);
    b.add_css_class("tafs-unsent");
    v.append(&b);
    unsent_menu(&v, &u.copy, &u.text, &u.delete, {
        let (p, id) = (Rc::downgrade(page), u.id);
        move || {
            if let Some(p) = p.upgrade() {
                p.discard(id);
            }
        }
    });
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    h.set_halign(gtk::Align::End);
    let m = symbol(
        if u.resume.is_some() {
            "process-stop-symbolic"
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
    if let Some(resume) = &u.resume {
        let b = link(resume, true);
        let open = page.open.clone();
        b.connect_clicked(move |_| open(Selection::HaltAll));
        row.append(&b);
    }
    if let Some(edit) = &u.edit {
        let b = link(edit, false);
        let p = Rc::downgrade(page);
        b.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.edit(id);
            }
        });
        row.append(&b);
    }
    let retry = link(&u.retry, u.resume.is_none());
    retry.set_sensitive(!busy);
    let p = Rc::downgrade(page);
    retry.connect_clicked(move |_| {
        if let Some(p) = p.upgrade() {
            p.retry(id);
        }
    });
    row.append(&retry);
    v.append(&row);
    v.upcast()
}

/// `StepMark`: done, failed, at work or to do.
fn plan_mark(step: &PlanStep) -> gtk::Widget {
    let mark = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    mark.add_css_class("tafs-plan-mark");
    mark.set_halign(gtk::Align::Center);
    mark.set_valign(gtk::Align::Center);
    match step.status.as_str() {
        "done" | "failed" => {
            mark.add_css_class(&step.status);
            let i = symbol(
                if step.status == "done" {
                    "tafs-checkmark-symbolic"
                } else {
                    "window-close-symbolic"
                },
                9,
            );
            i.set_hexpand(true);
            mark.append(&i);
        }
        "active" => mark.add_css_class("active"),
        _ => mark.add_css_class("pending"),
    }
    mark.set_accessible_role(gtk::AccessibleRole::Img);
    mark.update_property(&[gtk::accessible::Property::Label(&step.mark_words)]);
    mark.upcast()
}

/// `SetupPlan`: pinned above the thread, folded to the step in progress until opened.
fn plan_panel(plan: &PlanLines) -> gtk::Box {
    let p = panel(false);
    let b = gtk::Button::new();
    b.add_css_class("tafs-plain");
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let now_mark = if plan.now.status.is_empty() {
        None
    } else {
        Some(plan_mark(&plan.now))
    };
    if let Some(m) = &now_mark {
        h.append(m);
    }
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    v.append(&eyebrow(&plan.progress, false));
    let now = crate::widgets::one_line(&plan.now.label, 14, 600, &["tafs-ink"]);
    v.append(&now);
    h.append(&v);
    let chevron = symbol("pan-down-symbolic", 12);
    chevron.add_css_class("tafs-dim");
    h.append(&chevron);
    b.set_child(Some(&h));
    b.update_property(&[gtk::accessible::Property::Description(&plan.shows)]);
    p.append(&b);
    let steps = gtk::Box::new(gtk::Orientation::Vertical, 8);
    for s in &plan.steps {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(&plan_mark(s));
        let l = text(
            &s.label,
            14,
            if s.status == "active" { 600 } else { 400 },
            &[if s.status == "pending" || s.status == "skipped" {
                "tafs-dim"
            } else {
                "tafs-ink"
            }],
        );
        if s.status == "skipped" {
            let attrs = l.attributes().unwrap_or_default();
            attrs.insert(gtk::pango::AttrInt::new_strikethrough(true));
            l.set_attributes(Some(&attrs));
        }
        row.append(&l);
        steps.append(&row);
    }
    steps.set_visible(false);
    p.append(&steps);
    let (shows, folds) = (plan.shows.clone(), plan.folds.clone());
    b.connect_clicked(move |b| {
        let open = !steps.is_visible();
        steps.set_visible(open);
        now.set_visible(!open);
        if let Some(m) = &now_mark {
            m.set_visible(!open);
        }
        chevron.set_icon_name(Some(if open {
            "pan-up-symbolic"
        } else {
            "pan-down-symbolic"
        }));
        b.update_property(&[gtk::accessible::Property::Description(if open {
            &folds
        } else {
            &shows
        })]);
    });
    p
}

/// `OutcomeLine`: an answered card in one line, its mark in its colour.
fn outcome(o: &Outcome) -> gtk::Box {
    outcome_with(o, &[])
}

/// `OutlineLine` with its `detail`: what was approved, the signed Card's lines under the line.
fn outcome_with(o: &Outcome, detail: &[String]) -> gtk::Box {
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    h.add_css_class("tafs-outcome");
    let (icon, class) = match o.mark {
        Mark::Check => ("emblem-ok-symbolic", tone_class(o.tone)),
        Mark::Minus => ("list-remove-symbolic", tone_class(o.tone)),
        Mark::Below => ("go-down-symbolic", tone_class(o.tone)),
    };
    let m = symbol(icon, 14);
    m.add_css_class(class);
    m.set_valign(gtk::Align::Start);
    m.set_margin_top(1);
    h.append(&m);
    let l = text(&o.text, 14, 400, &["tafs-soft"]);
    if detail.is_empty() {
        l.set_hexpand(true);
        h.append(&l);
    } else {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
        v.set_hexpand(true);
        v.append(&l);
        for line in detail {
            let d = text(line, 12, 400, &["tafs-dim", "tafs-mono"]);
            v.append(&d);
        }
        h.append(&v);
    }
    if o.more {
        let c = symbol("pan-down-symbolic", 11);
        c.add_css_class("tafs-dim");
        h.append(&c);
    }
    h
}

/// A row of buttons that wraps (`WrapRow`).
fn wrap_row(spacing: u32) -> gtk::FlowBox {
    let f = gtk::FlowBox::new();
    f.set_selection_mode(gtk::SelectionMode::None);
    f.set_column_spacing(spacing);
    f.set_row_spacing(spacing);
    f.set_homogeneous(false);
    f.set_max_children_per_line(12);
    // Filling the row: a row that shrinks to its children misreads its height once they wrap.
    f.set_halign(gtk::Align::Fill);
    f.set_valign(gtk::Align::Start);
    f.set_can_focus(false);
    f
}

fn card_view(page: &Rc<Page>, card_ref: &str, card: &Card) -> gtk::Widget {
    let open = &page.open;
    match card {
        Card::Notice {
            title,
            text: words,
            tone,
        } => {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
            v.add_css_class("tafs-notice");
            match tone {
                Tone::No => v.add_css_class("no"),
                Tone::Amber => v.add_css_class("amber"),
                Tone::Ok => v.add_css_class("ok"),
                _ => {}
            }
            if let Some(t) = title {
                v.append(&text(t, 14, 600, &[tone_class(*tone)]));
            }
            v.append(&text(words, 14, 400, &["tafs-ink"]));
            v.upcast()
        }
        Card::Outcome(o) => outcome(o).upcast(),
        Card::Confirm {
            id,
            text: words,
            options,
        } => {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 8);
            v.append(&text(words, 15, 400, &["tafs-ink"]));
            let row = wrap_row(8);
            // Coach at work doesn't stop a tap: the answer waits for it. Answered here, it's quiet.
            let answered = page.conv.borrow().is_answered(id);
            for (label, value) in options {
                let b = gtk::Button::with_label(label);
                b.add_css_class("tafs-capsule");
                b.set_sensitive(!answered);
                b.set_halign(gtk::Align::Start);
                let (p, id, value) = (Rc::downgrade(page), id.clone(), value.clone());
                b.connect_clicked(move |_| {
                    if let Some(p) = p.upgrade() {
                        p.answer(&id, Kind::Confirm, &value);
                    }
                });
                row.insert(&b, -1);
            }
            v.append(&row);
            if let Some(n) = card_note(page, id) {
                v.append(&n);
            }
            v.upcast()
        }
        Card::Questions { .. } => questions_card(page, card_ref, card),
        Card::Approval(a) => approval(page, a),
        Card::SignIn(s) => sign_in(s),
        Card::Result(r) => result(r, open),
        Card::Drive {
            eyebrow: e,
            task,
            note,
            gate,
        } => {
            let p = card_panel(Tone::Amber);
            p.append(&eyebrow_in(e, Tone::Amber));
            p.append(&text(task, 20, 700, &["tafs-ink"]));
            p.append(&text(note, 14, 400, &["tafs-soft"]));
            if let Some(g) = gate {
                p.append(&crate::approval::not_ready(g));
            }
            p.upcast()
        }
        Card::Title(t) => {
            let p = panel(false);
            p.append(&text(t, 15, 600, &["tafs-ink"]));
            p.upcast()
        }
    }
}

/// tafs cloud's `wider` line, above where the buttons go (`WiderLine`): 14 medium, in the amber.
fn wider_line(words: &str) -> gtk::Label {
    let l = text(words, 14, 500, &["tafs-amber-text"]);
    l.set_widget_name("card.wider");
    l
}

/// The taf's Card on an approval: KIFF's draft, its title (or tafs cloud's words for the one
/// approved next) over its lines in monospace on the raised colour; or why it isn't drawn yet,
/// with Try again when KIFF wouldn't give it.
fn draft_shown(page: &Rc<Page>, d: &DraftShown) -> gtk::Widget {
    match d {
        DraftShown::Card {
            title,
            next_words,
            lines,
        } => {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            v.add_css_class("tafs-card-words");
            v.set_widget_name("approval.card");
            if let Some(words) = next_words {
                let l = text(words, 13, 600, &["tafs-soft"]);
                l.set_widget_name("approval.card-next");
                v.append(&l);
            } else if let Some(t) = title {
                v.append(&eyebrow(t, false));
            }
            let tone = if next_words.is_some() {
                "tafs-dim"
            } else {
                "tafs-ink"
            };
            for line in lines {
                v.append(&text(line, 12, 400, &[tone, "tafs-mono"]));
            }
            v.upcast()
        }
        DraftShown::NotRead { id, why, try_again } => {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            v.append(&text(why, 13, 400, &["tafs-no"]));
            let b = gtk::Button::with_label(try_again);
            b.add_css_class("tafs-coach-link");
            b.add_css_class("small");
            b.set_halign(gtk::Align::Start);
            b.set_widget_name("approval.draft-try-again");
            let (p, id) = (Rc::downgrade(page), id.clone());
            b.connect_clicked(move |_| {
                if let Some(p) = p.upgrade() {
                    let p2 = p.clone();
                    let id = id.clone();
                    glib::spawn_future_local(async move {
                        p2.read_drafts(vec![id]).await;
                        p2.draw();
                    });
                }
            });
            v.append(&b);
            v.upcast()
        }
        DraftShown::Reading(words) => text(words, 13, 400, &["tafs-dim"]).upcast(),
    }
}

/// `SetupShapePanel`: "Taking shape", the taf's name or the plan's title, then the plan with
/// each step's mark, and the Card KIFF drafted, dashed until it's confirmed.
fn shape_panel(into: &gtk::Box, l: &tafs_core::setup_shape::ShapeLines) {
    let head = gtk::Box::new(gtk::Orientation::Vertical, 6);
    head.append(&eyebrow(&l.eyebrow, false));
    if let Some(h) = &l.heading {
        head.append(&text(h, 26, 700, &["tafs-ink"]));
    }
    if let Some(e) = &l.empty {
        head.append(&text(e, 14, 400, &["tafs-soft"]));
    }
    into.append(&head);
    if let Some((progress, steps)) = &l.plan {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 10);
        v.append(&eyebrow(progress, false));
        for step in steps {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let m = plan_mark(&step.mark);
            m.set_valign(gtk::Align::Start);
            m.set_halign(gtk::Align::Start);
            m.set_hexpand(false);
            m.set_margin_top(1);
            row.append(&m);
            let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
            words.set_hexpand(true);
            let label = text(
                &step.mark.label,
                14,
                if step.strong { 600 } else { 400 },
                &[if step.dim { "tafs-dim" } else { "tafs-ink" }],
            );
            if step.struck {
                if let Some(attrs) = label.attributes() {
                    attrs.insert(gtk::pango::AttrInt::new_strikethrough(true));
                    label.set_attributes(Some(&attrs));
                }
            }
            words.append(&label);
            if let Some(n) = &step.needs_you {
                words.append(&text(n, 12, 600, &["tafs-amber-text"]));
            }
            row.append(&words);
            v.append(&row);
        }
        into.append(&v);
    }
    if let Some(card) = &l.card {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 10);
        v.append(&eyebrow(&card.eyebrow, false));
        let face = gtk::Box::new(gtk::Orientation::Vertical, 6);
        face.add_css_class("tafs-shape-card");
        if card.confirmed {
            face.add_css_class("confirmed");
        }
        face.append(&text(&card.title, 13, 600, &["tafs-ink", "tafs-mono"]));
        for line in &card.lines {
            face.append(&text(line, 12, 400, &["tafs-ink", "tafs-mono"]));
        }
        v.append(&face);
        if let Some(u) = &card.until {
            v.append(&text(u, 12, 400, &["tafs-dim"]));
        }
        into.append(&v);
    }
}

/// `SetupApproval`, read only.
fn approval(page: &Rc<Page>, a: &ApprovalLines) -> gtk::Widget {
    let p = card_panel(a.stroke);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let e = eyebrow_in(&a.eyebrow, a.eyebrow_tone);
    e.set_hexpand(true);
    head.append(&e);
    let fold = a.fold.as_ref().map(|f| {
        let b = gtk::Button::with_label(f);
        b.add_css_class("tafs-plain");
        b.add_css_class("tafs-dim");
        head.append(&b);
        b
    });
    p.append(&head);
    if let Some(done) = &a.done {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let m = symbol("emblem-ok-symbolic", 14);
        m.add_css_class("tafs-ok");
        h.append(&m);
        h.append(&text(done, 14, 600, &["tafs-ok"]));
        p.append(&h);
    }
    if let Some(t) = &a.title {
        p.append(&text(t, 17, 600, &["tafs-ink"]));
    }
    if let Some(t) = &a.text {
        p.append(&text(t, 15, 400, &["tafs-ink"]));
    }
    if let Some(d) = &a.draft {
        p.append(&draft_shown(page, d));
    }
    if let Some(next) = &a.card_next {
        p.append(&text(next, 13, 600, &["tafs-soft"]));
    }
    if let Some(w) = &a.wider {
        p.append(&wider_line(w));
    }
    if let Some(r) = &a.replaced {
        p.append(&text(r, 13, 400, &["tafs-soft"]));
    }
    if let Some(f) = &a.failed {
        p.append(&text(f, 14, 400, &["tafs-no"]));
    }
    if let Some(g) = &a.gate {
        p.append(&crate::approval::not_ready(g));
    }
    let Some(folded) = &a.folded else {
        return p.upcast();
    };
    // Answered, or asked again below: its line, which opens the card; Fold puts it back.
    let stack = gtk::Stack::new();
    stack.set_vhomogeneous(false);
    stack.set_hhomogeneous(false);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(200);
    let line = gtk::Button::new();
    line.add_css_class("tafs-outcome-button");
    line.set_child(Some(&outcome_with(folded, &a.signed_lines)));
    if let Some(hint) = &folded.hint {
        line.update_property(&[gtk::accessible::Property::Description(hint)]);
    }
    stack.add_named(&line, Some("line"));
    stack.add_named(&p, Some("card"));
    stack.set_visible_child_name("line");
    {
        let stack = stack.clone();
        line.connect_clicked(move |_| stack.set_visible_child_name("card"));
    }
    if let Some(fold) = fold {
        let stack = stack.clone();
        fold.connect_clicked(move |_| stack.set_visible_child_name("line"));
    }
    stack.upcast()
}

/// A vendor's sign-in, read only: its words, the wider line, and why this computer can't do it
/// in its button's place.
fn sign_in(s: &SignInLines) -> gtk::Widget {
    let p = card_panel(s.stroke);
    p.append(&eyebrow_in(&s.eyebrow, s.eyebrow_tone));
    p.append(&text(&s.title, 20, 700, &["tafs-ink"]));
    if let Some(f) = &s.failed {
        p.append(&text(f, 14, 400, &["tafs-no"]));
    }
    p.append(&text(&s.note, 14, 400, &["tafs-soft"]));
    if let Some(w) = &s.wider {
        p.append(&wider_line(w));
    }
    if let Some(g) = &s.gate {
        p.append(&crate::approval::not_ready(g));
    }
    p.upcast()
}

/// A result: the taf set up, or its new version.
fn result(r: &ResultLines, open: &Rc<dyn Fn(Selection)>) -> gtk::Widget {
    let p = card_panel(Tone::Ok);
    p.append(&eyebrow_in(&r.eyebrow, Tone::Ok));
    p.append(&text(&r.text, 15, 400, &["tafs-ink"]));
    if let Some(d) = &r.detail {
        p.append(&text(d, 13, 400, &["tafs-soft"]));
    }
    if let Some((words, choose)) = &r.plan_needed {
        p.append(&text(words, 14, 600, &["tafs-amber-text"]));
        let b = outline(choose);
        let open = open.clone();
        b.connect_clicked(move |_| open(Selection::Usage));
        p.append(&b);
    }
    if let Some((words, key)) = &r.open {
        let b = primary(words);
        let (open, key) = (open.clone(), key.clone());
        b.connect_clicked(move |_| open(Selection::Taf(key.clone())));
        p.append(&b);
    }
    for (words, tone) in &r.lines {
        p.append(&text(words, 13, 400, &[tone_class(*tone)]));
    }
    if let Some(preparing) = &r.preparing {
        // Its versions aren't on this computer: the button is there, dimmed.
        let b = primary(preparing);
        b.set_sensitive(false);
        p.append(&b);
    }
    p.upcast()
}

/// `CardNote`: why an answer didn't go, under its card.
fn card_note(page: &Page, id: &str) -> Option<gtk::Widget> {
    let note = page.conv.borrow().notes.get(id).cloned()?;
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let m = symbol("dialog-information-symbolic", 13);
    m.add_css_class("tafs-soft");
    m.set_valign(gtk::Align::Start);
    m.set_accessible_role(gtk::AccessibleRole::Presentation);
    h.append(&m);
    let l = text(&note, 13, 400, &["tafs-soft"]);
    l.set_hexpand(true);
    h.append(&l);
    Some(h.upcast())
}

/// `QuestionsCard` as it waits: each question's choices and Something else…, its own-answer
/// field, Send answers once each question has one, and Chat about this, which puts the card away
/// so the owner talks it through instead.
fn questions_card(page: &Rc<Page>, card_ref: &str, card: &Card) -> gtk::Widget {
    let Card::Questions {
        id,
        text: words,
        eyebrow: e,
        questions,
        asked,
        something_else,
        own_answer,
        send,
        chat_about,
        talking,
        show_again,
        ..
    } = card
    else {
        return gtk::Box::new(gtk::Orientation::Vertical, 0).upcast();
    };
    let card_ref = card_ref.to_string();
    if page.chatting.borrow().contains(&card_ref) {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        h.append(&text(talking, 13, 400, &["tafs-soft"]));
        let again = gtk::Button::with_label(show_again);
        again.add_css_class("tafs-coach-link");
        again.add_css_class("small");
        let (p, r) = (Rc::downgrade(page), card_ref.clone());
        again.connect_clicked(move |_| {
            if let Some(p) = p.upgrade() {
                p.chatting.borrow_mut().remove(&r);
                p.draw();
            }
        });
        h.append(&again);
        return h.upcast();
    }
    let busy = page.conv.borrow().is_answered(id);
    let selection = page
        .selections
        .borrow()
        .get(&card_ref)
        .cloned()
        .unwrap_or_default();
    let p = panel(false);
    if let Some(t) = words {
        p.append(&text(t, 15, 400, &["tafs-ink"]));
    }
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let eb = eyebrow(e, false);
    eb.set_hexpand(true);
    h.append(&eb);
    let count_of = {
        let w = page.w.clone();
        let asked = asked.clone();
        move |s: &QuestionSelection| {
            tafs_core::coach_lines::answered_of(&w, s.answered(&asked), asked.len())
                .unwrap_or_default()
        }
    };
    let count = text(&count_of(&selection), 12, 400, &["tafs-mono", "tafs-dim"]);
    h.append(&count);
    p.append(&h);
    let send_button = primary(send);
    send_button.set_hexpand(false);
    send_button.set_sensitive(!busy && selection.send(asked).is_some());
    // Typing in an own answer changes its count and Send at once, with no redraw.
    let typed: Rc<dyn Fn()> = {
        let (page, r, count, send_button, asked) = (
            Rc::downgrade(page),
            card_ref.clone(),
            count.clone(),
            send_button.clone(),
            asked.clone(),
        );
        Rc::new(move || {
            let Some(page) = page.upgrade() else { return };
            let s = page
                .selections
                .borrow()
                .get(&r)
                .cloned()
                .unwrap_or_default();
            count.set_label(&count_of(&s));
            send_button.set_sensitive(!busy && s.send(&asked).is_some());
        })
    };
    for (q, lines) in asked.iter().zip(questions) {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 10);
        v.append(&text(&lines.title, 16, 600, &["tafs-ink"]));
        let row = wrap_row(8);
        let own_open = selection.own_open.contains(&q.id);
        let choices = q
            .options
            .iter()
            .zip(&lines.options)
            .map(|(o, (label, detail))| {
                (
                    label.clone(),
                    detail.clone(),
                    Some(o.value.clone()),
                    selection.is_picked(q, &o.value),
                )
            })
            .chain(std::iter::once((
                something_else.clone(),
                None,
                None,
                own_open,
            )));
        for (label, detail, value, on) in choices {
            let b = gtk::Button::new();
            b.add_css_class("tafs-choice");
            if on {
                b.add_css_class("on");
            }
            let inner = gtk::Box::new(gtk::Orientation::Vertical, 2);
            // One line each in the row: a wrapping label can't tell the row its height.
            let l = text(&label, 15, if on { 600 } else { 400 }, &[]);
            l.set_wrap(false);
            inner.append(&l);
            if let Some(d) = &detail {
                let l = text(d, 12, 400, &["detail"]);
                l.set_wrap(false);
                inner.append(&l);
            }
            b.set_child(Some(&inner));
            b.set_sensitive(!busy);
            b.set_halign(gtk::Align::Start);
            b.update_state(&[gtk::accessible::State::Pressed(if on {
                gtk::AccessibleTristate::True
            } else {
                gtk::AccessibleTristate::False
            })]);
            let (pg, r, q) = (Rc::downgrade(page), card_ref.clone(), q.clone());
            b.connect_clicked(move |_| {
                let Some(pg) = pg.upgrade() else { return };
                {
                    let mut all = pg.selections.borrow_mut();
                    let s = all.entry(r.clone()).or_default();
                    match &value {
                        Some(v) => s.pick(&q, v),
                        None => {
                            s.toggle_own(&q);
                            if s.own_open.contains(&q.id) {
                                *pg.typing.borrow_mut() = Some((r.clone(), q.id.clone()));
                            }
                        }
                    }
                }
                pg.draw();
            });
            row.insert(&b, -1);
        }
        v.append(&row);
        if own_open {
            let entry = gtk::Entry::new();
            entry.add_css_class("tafs-own-answer");
            entry.set_placeholder_text(Some(own_answer));
            entry.update_property(&[gtk::accessible::Property::Label(&lines.own_answer_to)]);
            entry.set_text(selection.own.get(&q.id).map(String::as_str).unwrap_or(""));
            entry.set_sensitive(!busy);
            {
                let (pg, r, qid, typed) = (
                    Rc::downgrade(page),
                    card_ref.clone(),
                    q.id.clone(),
                    typed.clone(),
                );
                entry.connect_changed(move |e| {
                    let Some(pg) = pg.upgrade() else { return };
                    pg.selections
                        .borrow_mut()
                        .entry(r.clone())
                        .or_default()
                        .own
                        .insert(qid.clone(), e.text().to_string());
                    typed();
                });
            }
            let focus = gtk::EventControllerFocus::new();
            {
                let (pg, r, qid) = (Rc::downgrade(page), card_ref.clone(), q.id.clone());
                focus.connect_enter(move |_| {
                    if let Some(pg) = pg.upgrade() {
                        *pg.typing.borrow_mut() = Some((r.clone(), qid.clone()));
                    }
                });
            }
            {
                let (pg, r, qid) = (Rc::downgrade(page), card_ref.clone(), q.id.clone());
                focus.connect_leave(move |_| {
                    if let Some(pg) = pg.upgrade() {
                        let mut t = pg.typing.borrow_mut();
                        if t.as_ref() == Some(&(r.clone(), qid.clone())) {
                            *t = None;
                        }
                    }
                });
            }
            entry.add_controller(focus);
            // The field that had the keyboard before a redraw has it again.
            if page.typing.borrow().as_ref() == Some(&(card_ref.clone(), q.id.clone())) {
                let entry = entry.clone();
                glib::idle_add_local_once(move || {
                    entry.grab_focus();
                    entry.set_position(-1);
                });
            }
            v.append(&entry);
        }
        if let Some(n) = &lines.note {
            v.append(&text(n, 13, 400, &["tafs-soft"]));
        }
        p.append(&v);
    }
    let line = gtk::Separator::new(gtk::Orientation::Horizontal);
    line.add_css_class("tafs-line");
    p.append(&line);
    let row = wrap_row(12);
    {
        let (pg, r, id, asked) = (
            Rc::downgrade(page),
            card_ref.clone(),
            id.clone(),
            asked.clone(),
        );
        send_button.connect_clicked(move |_| {
            let Some(pg) = pg.upgrade() else { return };
            let words = pg.selections.borrow().get(&r).and_then(|s| s.send(&asked));
            if let Some(words) = words {
                pg.answer(&id, Kind::Questions, &words);
            }
        });
    }
    row.insert(&send_button, -1);
    let chat = gtk::Button::new();
    chat.add_css_class("tafs-coach-link");
    let ch = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let bubble = symbol("tafs-bubble-symbolic", 15);
    bubble.set_accessible_role(gtk::AccessibleRole::Presentation);
    ch.append(&bubble);
    ch.append(&gtk::Label::new(Some(chat_about)));
    chat.set_child(Some(&ch));
    chat.update_property(&[gtk::accessible::Property::Label(chat_about)]);
    {
        let (pg, r) = (Rc::downgrade(page), card_ref.clone());
        chat.connect_clicked(move |_| {
            if let Some(pg) = pg.upgrade() {
                pg.chatting.borrow_mut().insert(r.clone());
                pg.draw();
            }
        });
    }
    row.insert(&chat, -1);
    p.append(&row);
    if let Some(n) = card_note(page, id) {
        p.append(&n);
    }
    p.upcast()
}

/// Previous conversations' list, and Archived's, filled again on each change.
type Fill = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

fn refill(fill: &Fill) {
    let f = fill.borrow().clone();
    if let Some(f) = f {
        f();
    }
}

impl Page {
    /// A read at tafs cloud as the session, off the main thread: the body of a 2xx. None for a
    /// screenshot, or a read that didn't go; a 401 signs out.
    async fn get(&self, path: String) -> Option<Vec<u8>> {
        let CoachSource::Cloud {
            kept, languages, ..
        } = self.source.clone()
        else {
            return None;
        };
        let a = gtk::gio::spawn_blocking(move || {
            crate::kiff::cloud(&kept.token, &languages, &path).ok()
        })
        .await
        .ok()
        .flatten()?;
        if a.status == 401 {
            self.signed_out();
            return None;
        }
        (200..300).contains(&a.status).then_some(a.body)
    }

    /// Forgets what was drawn and picked: another conversation is drawn from nothing.
    fn redraw_from_nothing(self: &Rc<Self>) {
        self.jump.reland();
        self.selections.borrow_mut().clear();
        self.chatting.borrow_mut().clear();
        *self.typing.borrow_mut() = None;
        *self.drawn.borrow_mut() = None;
        *self.polls.borrow_mut() = Polls::default();
        self.draw();
    }

    /// New conversation (`newConversation`): an empty chat, and the next message starts it. One
    /// that's empty already stays, and shows its top. The box gets the keyboard.
    fn new_conversation(self: &Rc<Self>) {
        if self.conv.borrow().is_empty() {
            self.scroller.vadjustment().set_value(0.0);
        } else {
            self.conv.borrow_mut().new_chat();
            self.redraw_from_nothing();
        }
        self.composer.view.grab_focus();
    }

    /// One of Coach's chats reopened (`openSetupChat`): shown alone, and messages go on in it.
    fn open_chat(self: &Rc<Self>, session: &str) {
        if matches!(self.source, CoachSource::File { .. }) {
            return;
        }
        self.conv.borrow_mut().open_chat(session);
        self.redraw_from_nothing();
        let p = self.clone();
        glib::spawn_future_local(async move { p.load_thread().await });
    }

    /// `SetupChatsList`: Coach's chats, newest first, each by what the owner asked; one opens it,
    /// Archive takes it out of the list once confirmed, and Archived lists the ones put away.
    fn previous(self: &Rc<Self>) {
        let Some(window) = self.root.root() else {
            return;
        };
        let w = self.w.clone();
        let l = |k: &str| w.line(k).unwrap_or_default();
        let dialog = adw::Dialog::new();
        dialog.set_content_width(520);
        dialog.set_content_height(560);
        let nav = adw::NavigationView::new();
        dialog.set_child(Some(&nav));
        let note: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let list = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let fill: Fill = Rc::new(RefCell::new(None));
        let archived = self.archived_page(&dialog);
        let fill_now: Rc<dyn Fn()> = {
            let (p, list, note, dialog, nav, w) = (
                Rc::downgrade(self),
                list.clone(),
                note.clone(),
                dialog.downgrade(),
                nav.downgrade(),
                w.clone(),
            );
            let fill = Rc::downgrade(&fill);
            Rc::new(move || {
                let (Some(p), Some(dialog), Some(nav)) =
                    (p.upgrade(), dialog.upgrade(), nav.upgrade())
                else {
                    return;
                };
                while let Some(c) = list.first_child() {
                    list.remove(&c);
                }
                if let Some(n) = note.borrow().as_ref() {
                    list.append(&text(n, 13, 400, &["tafs-no"]));
                }
                let rows = {
                    let c = p.conv.borrow();
                    let shown = if c.fresh { None } else { c.chat.clone() };
                    tafs_core::coach_lines::chat_rows(&w, &c.chats, shown.as_deref())
                        .unwrap_or_default()
                };
                if rows.is_empty() {
                    let none = text(
                        &w.line("coach.chats.none").unwrap_or_default(),
                        14,
                        400,
                        &["tafs-dim"],
                    );
                    none.set_xalign(0.5);
                    none.set_margin_top(24);
                    none.set_margin_bottom(24);
                    list.append(&none);
                }
                for row in rows {
                    let h = chat_row(&row);
                    {
                        let (p, dialog, session) =
                            (Rc::downgrade(&p), dialog.downgrade(), row.session.clone());
                        h.0.connect_clicked(move |_| {
                            if let Some(d) = dialog.upgrade() {
                                d.close();
                            }
                            if let Some(p) = p.upgrade() {
                                p.open_chat(&session);
                            }
                        });
                    }
                    let archive =
                        gtk::Button::with_label(&w.line("coach.chats.archive").unwrap_or_default());
                    archive.add_css_class("tafs-unsent-act");
                    archive.set_valign(gtk::Align::Center);
                    {
                        let (p, dialog, session, note, fill) = (
                            Rc::downgrade(&p),
                            dialog.downgrade(),
                            row.session.clone(),
                            note.clone(),
                            fill.clone(),
                        );
                        archive.connect_clicked(move |_| {
                            let (Some(p), Some(dialog)) = (p.upgrade(), dialog.upgrade()) else {
                                return;
                            };
                            let fill = fill.upgrade();
                            p.ask_archive(&dialog, &session, note.clone(), fill);
                        });
                    }
                    h.1.append(&archive);
                    list.append(&h.1);
                }
                let to_archived = gtk::Button::new();
                to_archived.add_css_class("tafs-chat-row");
                let a = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let icon = symbol("tafs-archive-symbolic", 16);
                icon.add_css_class("tafs-ink");
                icon.set_accessible_role(gtk::AccessibleRole::Presentation);
                a.append(&icon);
                a.append(&text(
                    &w.line("coach.chats.archived").unwrap_or_default(),
                    15,
                    400,
                    &["tafs-ink"],
                ));
                to_archived.set_child(Some(&a));
                let archived = archived.clone();
                let nav = nav.downgrade();
                to_archived.connect_clicked(move |_| {
                    if let Some(nav) = nav.upgrade() {
                        nav.push(&archived);
                    }
                });
                let line = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                line.add_css_class("tafs-chat-line");
                to_archived.set_hexpand(true);
                line.append(&to_archived);
                list.append(&line);
            })
        };
        *fill.borrow_mut() = Some(fill_now.clone());
        fill_now();
        nav.add(&sheet_page(
            &l("coach.chats.previous"),
            &list,
            &dialog,
            &l("record.filter.done"),
        ));
        // The list's filler lives as long as the sheet.
        let keep = fill.clone();
        dialog.connect_closed(move |_| {
            keep.borrow_mut().take();
        });
        dialog.present(Some(&window));
        // `.task { loadSetupChats() }`: read again as it opens.
        let p = self.clone();
        glib::spawn_future_local(async move {
            let Some(setup) = p.conv.borrow().setup.clone() else {
                return;
            };
            let Some(chats) = p
                .get(coach::chats_path(&setup))
                .await
                .and_then(|b| coach::chats(&b))
            else {
                return;
            };
            p.conv.borrow_mut().chats = chats;
            refill(&fill);
        });
    }

    /// Archive's question, then the archive (`archiveSetupChat`): the one shown gives way to a new
    /// conversation; refused, why is said in the list.
    fn ask_archive(
        self: &Rc<Self>,
        dialog: &adw::Dialog,
        session: &str,
        note: Rc<RefCell<Option<String>>>,
        fill: Option<Fill>,
    ) {
        let l = |k: &str| self.w.line(k).unwrap_or_default();
        let ask = adw::AlertDialog::new(
            Some(&l("coach.chats.archive_title")),
            Some(&l("coach.chats.archive_body")),
        );
        ask.add_response("cancel", &l("common.cancel"));
        ask.add_response("archive", &l("coach.chats.archive"));
        ask.set_default_response(Some("cancel"));
        ask.set_close_response("cancel");
        let (p, session) = (self.clone(), session.to_string());
        ask.connect_response(None, move |_, response| {
            if response != "archive" {
                return;
            }
            let (p, session, note, fill) = (p.clone(), session.clone(), note.clone(), fill.clone());
            glib::spawn_future_local(async move {
                *note.borrow_mut() = None;
                let Some(setup) = p.conv.borrow().setup.clone() else {
                    return;
                };
                let why = match p
                    .post(coach_send::archive_path(&setup, &session), b"{}".to_vec())
                    .await
                {
                    None => return,
                    Some(Ok(a)) if a.status == 401 => return p.signed_out(),
                    Some(Ok(a)) if (200..300).contains(&a.status) => {
                        match coach_send::archived(&a.body) {
                            Some(_) => None,
                            None => {
                                Some(tafs_core::signin::plain_words(&p.w, 200).unwrap_or_default())
                            }
                        }
                    }
                    Some(Ok(a)) => Some(
                        Refusal::read(a.status, &a.body)
                            .words(&p.w)
                            .unwrap_or_default(),
                    ),
                    Some(Err(why)) => Some(why),
                };
                match why {
                    None => {
                        if p.conv.borrow_mut().archived(&session) {
                            p.redraw_from_nothing();
                        }
                    }
                    Some(why) => {
                        *note.borrow_mut() = Some(
                            p.w.get("taf.not_archived", &[("why", &why)])
                                .or_else(|_| p.w.line("coach.not_archived_try_again"))
                                .unwrap_or(why),
                        );
                    }
                }
                if let Some(f) = fill {
                    refill(&f);
                }
            });
        });
        ask.present(Some(dialog));
    }

    /// `ArchivedSetupChats`: the chats put away, read as it opens; one opens it, Put back returns it
    /// to the list.
    fn archived_page(self: &Rc<Self>, dialog: &adw::Dialog) -> adw::NavigationPage {
        let w = self.w.clone();
        let list = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let chats: Rc<RefCell<Vec<coach::Chat>>> = Rc::new(RefCell::new(Vec::new()));
        let note: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let fill: Fill = Rc::new(RefCell::new(None));
        let fill_now: Rc<dyn Fn()> = {
            let (p, list, chats, note, dialog, w) = (
                Rc::downgrade(self),
                list.clone(),
                chats.clone(),
                note.clone(),
                dialog.downgrade(),
                w.clone(),
            );
            let fill = Rc::downgrade(&fill);
            Rc::new(move || {
                let Some(p) = p.upgrade() else { return };
                while let Some(c) = list.first_child() {
                    list.remove(&c);
                }
                if let Some(n) = note.borrow().as_ref() {
                    list.append(&text(n, 13, 400, &["tafs-no"]));
                }
                let rows = tafs_core::coach_lines::chat_rows(&w, &chats.borrow(), None)
                    .unwrap_or_default();
                if rows.is_empty() {
                    let none = text(
                        &w.line("coach.chats.nothing_archived").unwrap_or_default(),
                        14,
                        400,
                        &["tafs-dim"],
                    );
                    none.set_xalign(0.5);
                    none.set_margin_top(24);
                    list.append(&none);
                }
                for row in rows {
                    let h = chat_row(&row);
                    {
                        let (p, dialog, session) =
                            (Rc::downgrade(&p), dialog.clone(), row.session.clone());
                        h.0.connect_clicked(move |_| {
                            if let Some(d) = dialog.upgrade() {
                                d.close();
                            }
                            if let Some(p) = p.upgrade() {
                                p.open_chat(&session);
                            }
                        });
                    }
                    let back = gtk::Button::with_label(
                        &w.line("coach.chats.put_back").unwrap_or_default(),
                    );
                    back.add_css_class("tafs-unsent-act");
                    back.set_valign(gtk::Align::Center);
                    {
                        let (p, session, chats, note, fill) = (
                            Rc::downgrade(&p),
                            row.session.clone(),
                            chats.clone(),
                            note.clone(),
                            fill.clone(),
                        );
                        back.connect_clicked(move |_| {
                            let Some(p) = p.upgrade() else { return };
                            let (session, chats, note, fill) =
                                (session.clone(), chats.clone(), note.clone(), fill.clone());
                            glib::spawn_future_local(async move {
                                *note.borrow_mut() = None;
                                let Some(setup) = p.conv.borrow().setup.clone() else {
                                    return;
                                };
                                let why = match p
                                    .post(
                                        coach_send::unarchive_path(&setup, &session),
                                        b"{}".to_vec(),
                                    )
                                    .await
                                {
                                    None => return,
                                    Some(Ok(a)) if a.status == 401 => return p.signed_out(),
                                    Some(Ok(a)) if (200..300).contains(&a.status) => {
                                        match coach_send::archived(&a.body) {
                                            Some(_) => None,
                                            None => Some(
                                                tafs_core::signin::plain_words(&p.w, 200)
                                                    .unwrap_or_default(),
                                            ),
                                        }
                                    }
                                    Some(Ok(a)) => Some(
                                        Refusal::read(a.status, &a.body)
                                            .words(&p.w)
                                            .unwrap_or_default(),
                                    ),
                                    Some(Err(why)) => Some(why),
                                };
                                match why {
                                    None => {
                                        chats.borrow_mut().retain(|c| c.session != session);
                                        // Back in the list: Previous conversations reads it again.
                                        if let Some(read) = p
                                            .get(coach::chats_path(&setup))
                                            .await
                                            .and_then(|b| coach::chats(&b))
                                        {
                                            p.conv.borrow_mut().chats = read;
                                        }
                                    }
                                    Some(why) => {
                                        *note.borrow_mut() = Some(
                                            p.w.get("taf.not_put_back", &[("why", &why)])
                                                .or_else(|_| {
                                                    p.w.line("coach.not_put_back_try_again")
                                                })
                                                .unwrap_or(why),
                                        );
                                    }
                                }
                                if let Some(f) = fill.upgrade() {
                                    refill(&f);
                                }
                            });
                        });
                    }
                    h.1.append(&back);
                    list.append(&h.1);
                }
            })
        };
        *fill.borrow_mut() = Some(fill_now.clone());
        fill_now();
        let title = w.line("coach.chats.archived").unwrap_or_default();
        let page = sheet_page(
            &title,
            &list,
            dialog,
            &w.line("record.filter.done").unwrap_or_default(),
        );
        // Read each time it's shown (`.task { loadArchivedSetupChats() }`).
        let (p, chats, fill2) = (Rc::downgrade(self), chats.clone(), fill.clone());
        page.connect_showing(move |_| {
            let Some(p) = p.upgrade() else { return };
            let (chats, fill) = (chats.clone(), fill2.clone());
            glib::spawn_future_local(async move {
                let Some(setup) = p.conv.borrow().setup.clone() else {
                    return;
                };
                if let Some(read) = p
                    .get(coach_send::archived_path(&setup))
                    .await
                    .and_then(|b| coach::chats(&b))
                {
                    *chats.borrow_mut() = read;
                    refill(&fill);
                }
            });
        });
        // The page holds its own list's filler until it goes.
        let keep = fill.clone();
        page.connect_destroy(move |_| {
            keep.borrow_mut().take();
        });
        page
    }
}

/// A chat in a list: the button that opens it (its title in two lines at most and when it last
/// changed), in a row that takes its act at the end.
fn chat_row(row: &tafs_core::coach_lines::ChatRow) -> (gtk::Button, gtk::Box) {
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    h.add_css_class("tafs-chat-line");
    if row.shown {
        h.add_css_class("shown");
    }
    let b = gtk::Button::new();
    b.add_css_class("tafs-chat-row");
    b.set_hexpand(true);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let t = text(&row.title, 15, 400, &["tafs-ink"]);
    t.set_lines(2);
    t.set_ellipsize(gtk::pango::EllipsizeMode::End);
    v.append(&t);
    v.append(&text(&row.when, 12, 400, &["tafs-dim"]));
    b.set_child(Some(&v));
    h.append(&b);
    (b, h)
}

/// A sheet's page: its title in the bar with Done, and its list.
fn sheet_page(
    title: &str,
    list: &gtk::Box,
    dialog: &adw::Dialog,
    done: &str,
) -> adw::NavigationPage {
    list.set_margin_start(16);
    list.set_margin_end(16);
    list.set_margin_top(8);
    list.set_margin_bottom(16);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(list));
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    let d = gtk::Button::with_label(done);
    d.add_css_class("tafs-unsent-act");
    d.add_css_class("strong");
    let dialog = dialog.downgrade();
    d.connect_clicked(move |_| {
        if let Some(dialog) = dialog.upgrade() {
            dialog.close();
        }
    });
    header.pack_end(&d);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    adw::NavigationPage::new(&toolbar, title)
}
