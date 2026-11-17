//! A taf's chat on its page's Chat tab (`TafChatView`), for a taf tafs cloud runs, signed in with
//! KIFF: its thread as tafs cloud reads it, drawn as the iPad draws it, with the message box. The
//! owner writes to the taf as the session, unsigned: a message into its live chat, or, while it
//! works, words to its run; Stop cancels the run. Linux has no stream: the thread is read again
//! every 4 s while the taf owes an answer or works (`followChat`'s plain reads). What the chat
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
    },
    /// No chat here: the sample's, and a taf tafs cloud doesn't run.
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

thread_local! {
    /// Each taf's chat as the app holds it (`chats`, `chatStates`), by its id at tafs cloud: kept
    /// while the app runs, so its page drawn again keeps what was sent and not sent.
    static CHATS: RefCell<HashMap<String, Rc<RefCell<TafChat>>>> = RefCell::new(HashMap::new());
    /// What the owner is typing to each taf, not sent yet.
    static DRAFTS: RefCell<HashMap<String, String>> = RefCell::new(HashMap::new());
}

/// Signed out: nothing written for the last account stays.
pub fn forget() {
    CHATS.with(|c| c.borrow_mut().clear());
    DRAFTS.with(|d| d.borrow_mut().clear());
}

/// The pane and what it holds while it's open.
struct Page {
    w: Rc<WordBook>,
    taf: ChatTaf,
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
    attach.button.set_visible(!source.is_none() && !taf.trial);

    let chat = CHATS.with(|c| {
        c.borrow_mut()
            .entry(taf.key())
            .or_insert_with(|| Rc::new(RefCell::new(TafChat::new(&taf.id))))
            .clone()
    });
    let page = Rc::new(Page {
        w,
        taf,
        source,
        chat,
        drawn: RefCell::new(None),
        reading: Cell::new(false),
        polls: RefCell::new(Polls::default()),
        follow: Cell::new(true),
        root: root.clone(),
        column,
        scroller,
        jump,
        error,
        composer,
        attach,
    });
    wire_composer(&page);
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
        } => {
            let read = std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok())
                .and_then(|b| TafThread::read(&b));
            match read {
                Some(t) => {
                    // A screenshot's chat starts from its file alone.
                    let fresh = TafChat::new(&page.taf.id);
                    *page.chat.borrow_mut() = fresh;
                    page.chat.borrow_mut().took(&page.w, t, chrono::Utc::now());
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
                p.follow_chat().await;
            });
        }
        ChatSource::None => page.draw(),
    }
    root.upcast()
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
        matches!(self.source, ChatSource::Cloud { .. })
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
    }

    /// `followChat`: the thread read again every 4 s while the taf owes an answer or works, at
    /// most 30 times for each message of the owner's. A read never overlaps another; leaving the
    /// chat ends it.
    async fn follow_chat(self: &Rc<Self>) {
        loop {
            glib::timeout_future(taf_chat::POLL).await;
            if !self.open() {
                return;
            }
            if self.reading.get() {
                continue;
            }
            let read = self.polls.borrow_mut().read_now(&self.chat.borrow());
            if read {
                self.load().await;
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
                self.composer.set_draft("");
                self.follow.set(true);
                self.draw();
                let p = self.clone();
                glib::spawn_future_local(async move {
                    if !p.deliver(d).await {
                        back(&p);
                    }
                });
            }
            Submit::Steer { run, text } => {
                self.composer.set_draft("");
                let p = self.clone();
                glib::spawn_future_local(async move {
                    if !p.steer(&run, &text).await {
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
                coach_send::message_body(&d.text, &d.target, &d.files),
            )
        };
        let Some(answer) = self.post(path, body).await else {
            return false;
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
                    self.chat.borrow_mut().not_sent(&d, &why, None, false, now);
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
                        self.chat.borrow_mut().not_sent(&d, &why, halted, busy, now);
                        true
                    }
                    Err(_) => {
                        self.chat.borrow_mut().not_tried(&d, None);
                        false
                    }
                }
            }
            Err(why) => {
                self.chat.borrow_mut().not_sent(&d, &why, None, false, now);
                true
            }
        };
        self.draw();
        taken
    }

    /// Written to the run while the taf works (`steer`, once, with `hold`): true once taken, held
    /// or kept as not sent. More than tafs cloud holds leaves the words in the box.
    async fn steer(self: &Rc<Self>, run: &str, words: &str) -> bool {
        let Some(answer) = self
            .post(
                coach_send::steer_path(&self.taf.id),
                coach_send::steer_body(run, words),
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
                    self.chat
                        .borrow_mut()
                        .steer_not_sent(words, &why, None, false, now);
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
                        self.chat
                            .borrow_mut()
                            .steer_not_sent(words, &why, halted, busy, now);
                        true
                    }
                    Err(_) => false,
                }
            }
            Err(why) => {
                self.chat
                    .borrow_mut()
                    .steer_not_sent(words, &why, None, false, now);
                true
            }
        };
        self.draw();
        taken
    }

    /// Stop (`stopChat`): the taf's current run is cancelled, over here at once, then the thread
    /// read again.
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
        self.draw();
        let p = self.clone();
        glib::spawn_future_local(async move {
            let taken = match submit {
                Submit::Stay => false,
                Submit::Send(d) => p.deliver(d).await,
                Submit::Steer { run, text } => p.steer(&run, &text).await,
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
        let thinking = self.chat.borrow().thinking();
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
            .taf
            .held
            .iter()
            .map(|h| (h.id.clone(), h.held_at.with_timezone(&chrono::Utc)))
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
                    .position(|l| matches!(l, Line::Held(k) if k == &h.id))?;
                Some((h, at))
            })
            .max_by_key(|(h, _)| h.held_at);
        let near_end = waiting.is_none_or(|(_, at)| lines.lines.len() - at <= 3);
        let pill = waiting.and_then(|(h, _)| {
            let card = crate::coach::named(self.column.upcast_ref(), &format!("held.{}", h.id))?;
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
        glib::idle_add_local_once(move || jump.put(keep, ready, near_end));
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
        for line in &lines.lines {
            column.append(&match line {
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
                    let answer = crate::chat::answer_words(words);
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
                Line::Held(key) => match self.taf.held.iter().find(|h| &h.id == key) {
                    Some(card) => {
                        let c = crate::needs_you::held_card(&self.w, card);
                        c.set_widget_name(&format!("held.{key}"));
                        c.upcast()
                    }
                    None => continue,
                },
                Line::AskCoach { text: words, ask } => {
                    let p = panel(false);
                    let l = text(words, 15, 400, &["tafs-ink"]);
                    p.append(&l);
                    if let Some(ask) = ask {
                        // Coach on this taf (`improveTaf`) isn't on this computer yet: there, dimmed.
                        let b = outline(ask);
                        b.set_sensitive(false);
                        p.append(&b);
                    }
                    p.upcast()
                }
            });
        }
        let busy = self.chat.borrow().sending;
        for u in &lines.unsent {
            column.append(&self.unsent(u, busy));
        }
        // The taf at work: the dots where its words will be.
        if let Some(answering) = &lines.thinking {
            column.append(&crate::chat::dots(answering));
        }
        // Written to the run while it goes, not read yet.
        for (words, note) in &lines.steering {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
            v.append(&owner(words, None, true));
            let n = text(note, 12, 400, &["tafs-dim"]);
            n.set_xalign(1.0);
            v.append(&n);
            column.append(&v);
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
