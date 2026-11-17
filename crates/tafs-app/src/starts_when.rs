//! Setup's own screens (`TafSetupViews.swift`): Starts when (`StartsWhenView`: its schedule, the
//! app events it listens to, its doors, the follow-ups it set itself, Pause every start) and
//! Results go to (`ResultsGoToView`), over the dial as tafs cloud sends it (`GET …/knob`). A change
//! to the dial is the session's (`PUT …/knob`): it waits as the taf's next version
//! (`KnobPendingPanel`), made live with the device's check, which this computer can't make. tafs'
//! own places change at once (`PUT …/results`); a follow-up is removed, every start paused or
//! resumed, all as the session. Approving an app event is signed at planque, and the iPad itself
//! disables it where the device can't approve. Both open as a sheet with its own navigation, so a
//! door's Edit opens its screen on top. The reads are `tafs-core/src/knob.rs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use serde_json::Value;
use tafs_core::knob::{self, Item, Knob, Setting};
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, outline, panel, primary, text};

/// Where the dial is read and changed.
#[derive(Clone)]
pub enum KnobSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's dial (`--knob`): read, and nothing a change would send is sent.
    File(std::path::PathBuf),
    None,
}

/// The taf Setup's screens are for.
pub struct KnobTaf {
    pub id: String,
    pub name: String,
    pub initial: String,
    pub color: i64,
    pub manages: bool,
    pub may_use_coach: bool,
    /// Its tools can't find its work by themselves: no schedule (`worksFromChat`).
    pub works_from_chat: bool,
    /// The line under its schedule (`knob::next_run_line`).
    pub next_run: Option<String>,
    /// Whether every start is paused, as Home read it.
    pub paused: bool,
    /// Where tafs cloud delivers its results itself, live, as Home read them.
    pub results: Option<Vec<Value>>,
    /// This person's sign-in email: Email me's line, and their own key at a door.
    pub email: Option<String>,
    pub doors: crate::doors::DoorsSource,
    pub work: crate::work_panel::WorkSource,
    /// Where a run's own page is read (the dial's requests).
    pub runs: crate::runs_panel::RunsSource,
    /// Opens Coach (`improveTaf`, then Coach).
    pub open_coach: Rc<dyn Fn()>,
}

type Listeners = Rc<RefCell<Vec<(u64, Rc<dyn Fn()>)>>>;

/// The dial as the screens share it (`knobs`, `knobChanges`), and what the screens draw from.
#[derive(Clone)]
pub struct Knobs {
    w: Rc<WordBook>,
    taf: Rc<KnobTaf>,
    source: KnobSource,
    knob: Rc<RefCell<Option<Knob>>>,
    /// Why the dial couldn't be read or changed, in tafs cloud's words.
    error: Rc<RefCell<Option<String>>>,
    busy: Rc<Cell<bool>>,
    /// The version the last change made (`knobChanges`), until it's live.
    changed: Rc<Cell<Option<i64>>>,
    paused: Rc<Cell<bool>>,
    results: Rc<RefCell<Option<Vec<Value>>>>,
    /// Its work, for Results go to's replies not sent.
    work: Rc<RefCell<Option<tafs_core::work::Work>>>,
    sending: Rc<RefCell<Vec<String>>>,
    listeners: Listeners,
    next: Rc<Cell<u64>>,
}

/// What a request came to.
pub(crate) enum Sent {
    Answer(tafs_core::signin::Answer),
    /// A screenshot's: nothing sent.
    Nothing,
    Failed(String),
}

impl Knobs {
    pub fn new(w: Rc<WordBook>, taf: KnobTaf, source: KnobSource) -> Knobs {
        let paused = taf.paused;
        let results = taf.results.clone();
        Knobs {
            w,
            taf: Rc::new(taf),
            source,
            knob: Rc::new(RefCell::new(None)),
            error: Rc::new(RefCell::new(None)),
            busy: Rc::new(Cell::new(false)),
            changed: Rc::new(Cell::new(None)),
            paused: Rc::new(Cell::new(paused)),
            results: Rc::new(RefCell::new(results)),
            work: Rc::new(RefCell::new(None)),
            sending: Rc::new(RefCell::new(Vec::new())),
            listeners: Rc::new(RefCell::new(Vec::new())),
            next: Rc::new(Cell::new(0)),
        }
    }

    pub(crate) fn listen(&self, f: Rc<dyn Fn()>) -> u64 {
        let id = self.next.get() + 1;
        self.next.set(id);
        self.listeners.borrow_mut().push((id, f));
        id
    }

    pub(crate) fn unlisten(&self, id: u64) {
        self.listeners.borrow_mut().retain(|(i, _)| *i != id);
    }

    pub(crate) fn changed_now(&self) {
        let all: Vec<Rc<dyn Fn()>> = self
            .listeners
            .borrow()
            .iter()
            .map(|(_, f)| f.clone())
            .collect();
        for f in all {
            f();
        }
    }

    pub(crate) fn l(&self, k: &str) -> String {
        self.w.line(k).unwrap_or_default()
    }

    /// One request to tafs cloud as the session; a screenshot's sends nothing.
    pub(crate) async fn send(
        &self,
        method: &'static str,
        path: String,
        body: Option<Vec<u8>>,
    ) -> Sent {
        let (words, source) = ((*self.w).clone(), self.source.clone());
        gio::spawn_blocking(move || match &source {
            KnobSource::Cloud { token, languages } => {
                let a = if method == "GET" {
                    crate::kiff::cloud(token, languages, &path)
                } else {
                    crate::kiff::cloud_send(token, languages, method, &path, body.as_deref())
                };
                match a {
                    Ok(a) => Sent::Answer(a),
                    Err(e) => Sent::Failed(e.words(&words).unwrap_or_default()),
                }
            }
            KnobSource::File(file) if method == "GET" => match std::fs::read(file) {
                Ok(body) => Sent::Answer(tafs_core::signin::Answer { status: 200, body }),
                Err(_) => Sent::Failed(String::new()),
            },
            _ => Sent::Nothing,
        })
        .await
        .unwrap_or(Sent::Failed(String::new()))
    }

    /// tafs cloud's words for a refusal.
    pub(crate) fn refused(&self, a: &tafs_core::signin::Answer) -> String {
        if (200..300).contains(&a.status) {
            return tafs_core::signin::plain_words(&self.w, 200).unwrap_or_default();
        }
        tafs_core::signin::refusal(a, Some(&self.w))
            .words(&self.w)
            .unwrap_or_default()
    }

    async fn read_once(&self, version: Option<i64>) -> Result<Knob, (u16, String)> {
        match self
            .send("GET", knob::path(&self.taf.id, version), None)
            .await
        {
            Sent::Answer(a) if a.status == 200 => {
                knob::read(&a.body).ok_or_else(|| (200, self.refused(&a)))
            }
            Sent::Answer(a) => Err((a.status, self.refused(&a))),
            Sent::Nothing => Err((0, String::new())),
            Sent::Failed(why) => Err((0, why)),
        }
    }

    /// `loadKnob`: the dial, and when a version waits, that version's dial; one installed or
    /// replaced while reading (404) is read again as it is now.
    pub(crate) async fn load(&self) {
        let read = match self.read_once(None).await {
            Ok(k) => match k.pending.as_ref().map(|p| p.version) {
                Some(v) if !matches!(self.source, KnobSource::File(_)) => {
                    match self.read_once(Some(v)).await {
                        Err((404, _)) => self.read_once(None).await,
                        other => other,
                    }
                }
                _ => Ok(k),
            },
            Err(e) => Err(e),
        };
        match read {
            Ok(k) => {
                if let (Some(made), Some(live)) = (self.changed.get(), k.version) {
                    if live >= made {
                        self.changed.set(None);
                    }
                }
                *self.knob.borrow_mut() = Some(k);
                *self.error.borrow_mut() = None;
            }
            Err((_, why)) => {
                *self.error.borrow_mut() = self
                    .w
                    .get("knob.dial_could_not_read", &[("why", &why)])
                    .ok();
            }
        }
        self.changed_now();
    }

    /// The dial as last read, why it couldn't be read or changed, whether a change is on its
    /// way, the version the last change made, whether every start is paused, its work, and the
    /// taf.
    pub(crate) fn knob(&self) -> Option<Knob> {
        self.knob.borrow().clone()
    }

    pub(crate) fn error(&self) -> Option<String> {
        self.error.borrow().clone()
    }

    pub(crate) fn set_error(&self, why: Option<String>) {
        *self.error.borrow_mut() = why;
        self.changed_now();
    }

    pub(crate) fn busy(&self) -> bool {
        self.busy.get()
    }

    pub(crate) fn set_busy(&self, busy: bool) {
        self.busy.set(busy);
        self.changed_now();
    }

    pub(crate) fn paused(&self) -> bool {
        self.paused.get()
    }

    pub(crate) fn work(&self) -> Option<tafs_core::work::Work> {
        self.work.borrow().clone()
    }

    pub(crate) fn taf(&self) -> &KnobTaf {
        &self.taf
    }

    pub(crate) fn words(&self) -> &Rc<WordBook> {
        &self.w
    }

    pub fn reload(&self) {
        let me = self.clone();
        glib::spawn_future_local(async move { me.load().await });
    }

    /// Its work (`loadWork`), for the replies given up on.
    pub(crate) fn load_work(&self) {
        let me = self.clone();
        glib::spawn_future_local(async move {
            let (source, id) = (me.taf.work.clone(), me.taf.id.clone());
            let read = gio::spawn_blocking(move || -> Option<tafs_core::work::Work> {
                let body = match &source {
                    crate::work_panel::WorkSource::Cloud { token, languages } => {
                        let a = crate::kiff::cloud(token, languages, &tafs_core::work::path(&id))
                            .ok()?;
                        (a.status == 200).then_some(a.body)?
                    }
                    crate::work_panel::WorkSource::File(f) => std::fs::read(f).ok()?,
                    crate::work_panel::WorkSource::None => return None,
                };
                tafs_core::work::read(&body)
            })
            .await
            .ok()
            .flatten();
            *me.work.borrow_mut() = read;
            me.changed_now();
        });
    }

    /// `changeKnob`: the live setting copied, changed, and sent whole; tafs cloud makes the next
    /// version, which waits to be made live. Read again after.
    fn change(&self, edit: impl FnOnce(&mut Setting)) {
        if self.busy.get() || !self.taf.manages {
            return;
        }
        let Some(setting) = self.knob.borrow().as_ref().and_then(|k| k.setting.clone()) else {
            return;
        };
        let mut changed = setting.clone();
        edit(&mut changed);
        if changed == setting {
            return;
        }
        self.busy.set(true);
        self.changed_now();
        let me = self.clone();
        glib::spawn_future_local(async move {
            let sent = me
                .send("PUT", knob::path(&me.taf.id, None), Some(changed.body()))
                .await;
            let why = match sent {
                Sent::Answer(a) if a.status == 200 => match knob::read_change(&a.body) {
                    Some(c) => {
                        me.changed.set(Some(c.version));
                        None
                    }
                    None => Some(me.refused(&a)),
                },
                Sent::Answer(a) => Some(me.refused(&a)),
                Sent::Nothing => None,
                Sent::Failed(why) => Some(why),
            };
            if let Some(why) = why {
                *me.error.borrow_mut() =
                    me.w.get("knob.dial_did_not_change", &[("why", &why)]).ok();
            }
            me.load().await;
            me.busy.set(false);
            me.changed_now();
        });
    }

    /// `adjustKnob`: a stop's next size down or up, made on the newest draft only.
    pub(crate) fn adjust(&self, item: &Item, direction: i64) {
        let choice = {
            let knob = self.knob.borrow();
            let Some(k) = knob.as_ref() else { return };
            if !k.accepts_choices(self.changed.get()) || k.find(&item.id) != Some(item) {
                return;
            }
            match item.size.as_ref().and_then(|s| s.choice(direction)) {
                Some(c) => c.setting.clone(),
                None => return,
            }
        };
        self.change(move |s| *s = choice);
    }

    pub(crate) fn can_move(&self, item: &Item, direction: i64) -> bool {
        let knob = self.knob.borrow();
        knob.as_ref()
            .is_some_and(|k| k.accepts_choices(self.changed.get()))
            && item
                .size
                .as_ref()
                .and_then(|s| s.choice(direction))
                .is_some()
    }

    /// A change as the session that answers with no new version: a follow-up removed, every
    /// start paused or resumed, tafs' own places. `done` reads the answer; its error is said.
    fn act(
        &self,
        method: &'static str,
        path: String,
        body: Option<Vec<u8>>,
        done: impl FnOnce(&Knobs, &tafs_core::signin::Answer) -> Result<(), String> + 'static,
        failed: impl FnOnce(&Knobs, String) -> String + 'static,
    ) {
        if self.busy.get() {
            return;
        }
        self.busy.set(true);
        self.changed_now();
        let me = self.clone();
        glib::spawn_future_local(async move {
            let why = match me.send(method, path, body).await {
                Sent::Answer(a) => done(&me, &a).err(),
                Sent::Nothing => None,
                Sent::Failed(why) => Some(why),
            };
            if let Some(why) = why {
                *me.error.borrow_mut() = Some(failed(&me, why));
            }
            me.load().await;
            me.busy.set(false);
            me.changed_now();
        });
    }

    pub(crate) fn remove_follow_up(&self, id: &str) {
        self.act(
            "DELETE",
            knob::follow_up_path(&self.taf.id, id),
            None,
            |me, a| {
                if a.status == 200 && knob::removed(&a.body) {
                    Ok(())
                } else {
                    Err(me.refused(a))
                }
            },
            |me, why| {
                me.w.get("knob.follow_up_not_removed", &[("why", &why)])
                    .unwrap_or(why)
            },
        );
    }

    /// `setPaused`: no run starts while paused. A 502 while pausing: not every start could be.
    pub(crate) fn set_paused(&self, paused: bool) {
        *self.error.borrow_mut() = None;
        self.act(
            "POST",
            knob::pause_path(&self.taf.id, paused),
            Some(b"{}".to_vec()),
            move |me, a| {
                if a.status == 502 && paused {
                    return Err(me.l("setup.not_every_paused"));
                }
                match (a.status, knob::read_taf(&a.body)) {
                    (200, Some(t)) => {
                        me.paused.set(t.paused);
                        Ok(())
                    }
                    _ => Err(me.refused(a)),
                }
            },
            |_, why| why,
        );
    }

    /// `setResultPlaces`: tafs' own places, live at once.
    fn set_places(&self, places: Vec<String>) {
        if places.is_empty() || !self.taf.manages {
            return;
        }
        self.act(
            "PUT",
            knob::results_path(&self.taf.id),
            Some(knob::results_body(&places)),
            |me, a| match (a.status, knob::read_taf(&a.body)) {
                (200, Some(t)) => {
                    *me.results.borrow_mut() = t.results;
                    Ok(())
                }
                _ => Err(me.refused(a)),
            },
            |me, why| {
                me.w.get("setup.results_not_changed", &[("why", &why)])
                    .unwrap_or(why)
            },
        );
    }

    /// `sendAgain`: a reply given up on, sent once more by its place.
    fn send_again(&self, run: &str, place: &str) {
        if self.sending.borrow().iter().any(|r| r == run) {
            return;
        }
        self.sending.borrow_mut().push(run.to_string());
        self.changed_now();
        let me = self.clone();
        let (run, path) = (
            run.to_string(),
            tafs_core::runs::send_again_path(&self.taf.id, run, place),
        );
        glib::spawn_future_local(async move {
            let sent = me.send("POST", path, Some(b"{}".to_vec())).await;
            if let Sent::Answer(a) = &sent {
                if !(200..300).contains(&a.status) {
                    *me.error.borrow_mut() = Some(me.refused(a));
                }
            } else if let Sent::Failed(why) = sent {
                *me.error.borrow_mut() = Some(why);
            }
            me.sending.borrow_mut().retain(|r| r != &run);
            me.load_work();
        });
    }
}

/// Which of Setup's screens.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    StartsWhen,
    ResultsGoTo,
}

/// The name of a taf's page's navigation, where Setup's screens push.
pub const TAF_NAV: &str = "taf-nav";

/// Opens one of Setup's screens: pushed on the taf's page it's opened from, with Back and its title
/// in the bar, as the iPad's `NavigationLink`; elsewhere, in a sheet with its own navigation.
pub fn open(knobs: Knobs, screen: Screen, parent: &impl IsA<gtk::Widget>) {
    let title = knobs.l(match screen {
        Screen::StartsWhen => "setup.starts_when",
        Screen::ResultsGoTo => "setup.results_go_to",
    });
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let host = parent
        .ancestor(adw::NavigationView::static_type())
        .and_downcast::<adw::NavigationView>()
        .filter(|n| n.widget_name() == TAF_NAV);
    let (nav, dialog) = match &host {
        Some(nav) => (nav.clone(), None),
        None => {
            let nav = adw::NavigationView::new();
            let dialog = adw::Dialog::new();
            dialog.set_child(Some(&nav));
            dialog.set_content_width(600);
            dialog.set_content_height(1000);
            dialog.set_title(&title);
            (nav, Some(dialog))
        }
    };
    let shown = page(&title, &column);
    nav.push(&shown);
    let ctx = Rc::new(Screens {
        knobs: knobs.clone(),
        screen,
        column,
        nav: nav.downgrade(),
        dialog: dialog.as_ref().map(|d| d.downgrade()).unwrap_or_default(),
        slot: crate::approval::refusal_slot(),
        doors: RefCell::new(None),
    });
    let listening = {
        let weak = Rc::downgrade(&ctx);
        knobs.listen(Rc::new(move || {
            if let Some(c) = weak.upgrade() {
                c.draw();
            }
        }))
    };
    // Gone (the sheet closed, or Back past it): it stops listening, and its state goes.
    let keep = RefCell::new(Some(ctx.clone()));
    let done = {
        let knobs = knobs.clone();
        move || {
            if keep.borrow_mut().take().is_some() {
                knobs.unlisten(listening);
            }
        }
    };
    ctx.draw();
    knobs.reload();
    if screen == Screen::ResultsGoTo {
        knobs.load_work();
    }
    match dialog {
        Some(dialog) => {
            dialog.connect_closed(move |_| done());
            dialog.present(Some(parent));
        }
        None => {
            let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::default();
            let id = {
                let handler = handler.clone();
                nav.connect_popped(move |nav, popped| {
                    if popped == &shown {
                        done();
                        if let Some(id) = handler.borrow_mut().take() {
                            nav.disconnect(id);
                        }
                    }
                })
            };
            *handler.borrow_mut() = Some(id);
        }
    }
}

fn page(title: &str, content: &gtk::Box) -> adw::NavigationPage {
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(content));
    scroller.set_vexpand(true);
    let toolbar = adw::ToolbarView::new();
    let bar = adw::HeaderBar::new();
    bar.set_show_end_title_buttons(false);
    bar.set_show_start_title_buttons(false);
    toolbar.add_top_bar(&bar);
    // Back as the window's other screens draw it: round, on the panel's white.
    toolbar.add_css_class("tafs-pushed");
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    adw::NavigationPage::new(&toolbar, title)
}

/// A screen's state while it's open.
struct Screens {
    knobs: Knobs,
    screen: Screen,
    column: gtk::Box,
    nav: glib::WeakRef<adw::NavigationView>,
    dialog: glib::WeakRef<adw::Dialog>,
    /// Where Make it live's tap says this computer can't approve.
    slot: gtk::Box,
    /// The taf's doors, read once Starts when has its dial.
    doors: RefCell<Option<(crate::doors::DoorsModel, u64)>>,
}

impl Drop for Screens {
    fn drop(&mut self) {
        if let Some((model, id)) = self.doors.borrow_mut().take() {
            model.unlisten(id);
        }
    }
}

impl Screens {
    fn l(&self, k: &str) -> String {
        self.knobs.l(k)
    }

    fn draw(self: &Rc<Self>) {
        while let Some(c) = self.column.first_child() {
            self.column.remove(&c);
        }
        let knob = self.knobs.knob.borrow().clone();
        let error = self.knobs.error.borrow().clone();
        match (self.screen, knob) {
            (Screen::StartsWhen, Some(k)) => self.starts_when(&k),
            (Screen::ResultsGoTo, Some(k)) => self.results_go_to(&k),
            (screen, None) => {
                self.column.append(&text(
                    &self.l(if screen == Screen::StartsWhen {
                        "setup.reading_starts"
                    } else {
                        "setup.reading_results"
                    }),
                    15,
                    400,
                    &["tafs-dim"],
                ));
            }
        }
        if let Some(e) = error {
            self.column.append(&text(&e, 13, 400, &["tafs-no"]));
        }
    }

    /// `KnobPendingPanel`: a change waiting as the next version, its words, and Make it live,
    /// which the device checks before the switch: this computer says it can't, and begins
    /// nothing.
    fn pending(self: &Rc<Self>, k: &Knob) {
        if !self.knobs.taf.manages {
            return;
        }
        let Ok(Some((head, lines, button))) = knob::pending_lines(&self.knobs.w, k) else {
            return;
        };
        let p = panel(false);
        p.add_css_class("knocks-edge");
        p.append(&text(&head, 10, 600, &["tafs-mono", "tafs-amber-text"]));
        for line in &lines {
            p.append(&text(line, 13, 400, &["tafs-soft"]));
        }
        let b = primary(&button);
        let (me, version) = (Rc::downgrade(self), k.pending.as_ref().map(|p| p.version));
        b.connect_clicked(move |_| {
            let (Some(c), Some(version)) = (me.upgrade(), version) else {
                return;
            };
            crate::approval::refuse(
                &c.knobs.w,
                tafs_core::sidebar::Mode::Kiff,
                &tafs_core::signed::Act::MakeLive {
                    taf: c.knobs.taf.id.clone(),
                    version,
                },
                &c.slot,
            );
        });
        p.append(&b);
        if let Some(parent) = self.slot.parent() {
            if let Ok(b) = parent.downcast::<gtk::Box>() {
                b.remove(&self.slot);
            }
        }
        p.append(&self.slot);
        self.column.append(&p);
    }

    fn starts_when(self: &Rc<Self>, k: &Knob) {
        let manages = self.knobs.taf.manages;
        let busy = self.knobs.busy.get();
        self.pending(k);
        self.schedule(k, manages, busy);
        self.events(k, manages, busy);
        if manages && !matches!(self.knobs.taf.doors, crate::doors::DoorsSource::None) {
            self.doors();
        }
        self.follow_ups(k, manages, busy);
        if manages && self.knobs.taf.may_use_coach {
            let b = outline(&self.l("setup.change_starts"));
            let me = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(c) = me.upgrade() {
                    c.coach();
                }
            });
            self.column.append(&b);
        }
        // Pause every start.
        if manages && k.can_pause {
            let paused = self.knobs.paused.get();
            let b = outline(&self.l(if paused {
                "setup.resume_every"
            } else {
                "setup.pause_every"
            }));
            if !paused {
                b.add_css_class("tafs-no");
            } else {
                b.add_css_class("tafs-ok");
            }
            b.set_sensitive(!busy);
            let knobs = self.knobs.clone();
            b.connect_clicked(move |_| knobs.set_paused(!paused));
            self.column.append(&b);
        }
    }

    fn coach(&self) {
        if let Some(d) = self.dialog.upgrade() {
            d.close();
        }
        (self.knobs.taf.open_coach)();
    }

    /// Its schedule: each with − and + through tafs cloud's sizes, when it runs next, and what
    /// it does when still working at the next start.
    fn schedule(self: &Rc<Self>, k: &Knob, manages: bool, busy: bool) {
        let schedules = k.wakes_of("schedule");
        let p = panel(false);
        p.append(&eyebrow(&self.l("setup.schedule"), false));
        if schedules.is_empty() {
            p.append(&text(
                &self.l(if self.knobs.taf.works_from_chat {
                    "setup.no_start"
                } else {
                    "setup.no_schedule"
                }),
                15,
                400,
                &["tafs-soft"],
            ));
        }
        for item in &schedules {
            p.append(&text(item.line(), 15, 600, &["tafs-ink"]));
            if let Some(size) = &item.size {
                p.append(&self.stepper(item, &size.stepper_label(), manages && !busy));
            }
        }
        if let Some(next) = &self.knobs.taf.next_run {
            if !self.knobs.paused.get() {
                p.append(&text(next, 13, 400, &["tafs-dim"]));
            }
        }
        if !schedules.is_empty() {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            v.append(&text(&self.l("setup.if_busy"), 13, 400, &["tafs-soft"]));
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            row.add_css_class("tafs-segmented");
            row.set_homogeneous(true);
            let busy_now = k
                .setting
                .as_ref()
                .map(|s| s.busy.clone())
                .unwrap_or_else(|| "skip".into());
            let mut first: Option<gtk::ToggleButton> = None;
            for (value, key) in [("skip", "setup.skip_it"), ("latest", "setup.run_after")] {
                let b = gtk::ToggleButton::with_label(&self.l(key));
                b.add_css_class("segment");
                match &first {
                    Some(f) => b.set_group(Some(f)),
                    None => first = Some(b.clone()),
                }
                b.set_active(busy_now == value);
                b.set_sensitive(manages && !busy && k.setting.is_some());
                let knobs = self.knobs.clone();
                b.connect_toggled(move |b| {
                    if b.is_active() {
                        knobs.change(|s| s.busy = value.into());
                    }
                });
                row.append(&b);
            }
            row.update_property(&[gtk::accessible::Property::Label(
                &self.l("setup.if_still_working"),
            )]);
            v.append(&row);
            p.append(&v);
        }
        self.column.append(&p);
    }

    /// `SizeStepper`: − and + through the stop's choices, its value between them; the three share
    /// the row, as the iPad's outlined buttons and value do.
    fn stepper(self: &Rc<Self>, item: &Item, label: &str, may: bool) -> gtk::Box {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        row.set_homogeneous(true);
        for (direction, icon, words) in [
            (-1, "list-remove-symbolic", "common.less"),
            (1, "list-add-symbolic", "common.more"),
        ] {
            let b = gtk::Button::from_icon_name(icon);
            b.add_css_class("tafs-outline");
            b.set_size_request(44, 36);
            b.set_hexpand(true);
            b.set_sensitive(may && self.knobs.can_move(item, direction));
            b.update_property(&[gtk::accessible::Property::Label(&self.l(words))]);
            let (knobs, item) = (self.knobs.clone(), item.clone());
            b.connect_clicked(move |_| knobs.adjust(&item, direction));
            if direction == 1 {
                let value = text(label, 14, 600, &["tafs-ink"]);
                value.set_hexpand(true);
                value.set_xalign(0.5);
                value.set_justify(gtk::Justification::Center);
                row.append(&value);
            }
            row.append(&b);
        }
        row
    }

    /// App events: each, with Approve while it waits (signed at planque: the iPad disables it
    /// where the device can't approve, as here), or Listening and Stop.
    fn events(self: &Rc<Self>, k: &Knob, manages: bool, busy: bool) {
        let events = k.wakes_of("app_event");
        if events.is_empty() {
            return;
        }
        let p = panel(false);
        p.append(&eyebrow(&self.l("setup.app_events"), false));
        for item in events {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            v.append(&text(&item.label, 15, 600, &["tafs-ink"]));
            if !item.words.is_empty() {
                v.append(&text(&item.words, 13, 400, &["tafs-soft"]));
            }
            let state = item.approval.as_ref().map(|a| a.state.as_str());
            if state == Some("needed") && item.ok.as_deref() == Some("approve") {
                let b = primary(&self.l("setup.approve"));
                // `disabled(!manages || model.approvalBlocked)`: this computer can't approve.
                b.set_sensitive(false);
                v.append(&b);
            } else if state == Some("approved") || item.on {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let listening = text(&self.l("setup.listening"), 13, 600, &["tafs-ok"]);
                listening.set_hexpand(true);
                row.append(&listening);
                if let (true, Some(trigger)) = (manages, &item.trigger) {
                    let b = outline(&self.l("setup.stop"));
                    b.add_css_class("tafs-no");
                    b.set_hexpand(false);
                    b.set_sensitive(!busy && k.setting.is_some());
                    let (knobs, trigger) = (self.knobs.clone(), trigger.clone());
                    b.connect_clicked(move |_| {
                        let trigger = trigger.clone();
                        knobs.change(move |s| s.wakes.retain(|w| *w != trigger));
                    });
                    row.append(&b);
                }
                v.append(&row);
            }
            p.append(&v);
        }
        p.append(&text(&self.l("setup.approve_note"), 12, 400, &["tafs-dim"]));
        self.column.append(&p);
    }

    fn door_model(self: &Rc<Self>) -> crate::doors::DoorsModel {
        if let Some((m, _)) = self.doors.borrow().as_ref() {
            return m.clone();
        }
        let m = crate::doors::DoorsModel::new(
            self.knobs.w.clone(),
            self.knobs.taf.id.clone(),
            self.knobs.taf.doors.clone(),
        );
        let weak = Rc::downgrade(self);
        let id = m.listen(Rc::new(move || {
            if let Some(c) = weak.upgrade() {
                c.draw();
            }
        }));
        *self.doors.borrow_mut() = Some((m.clone(), id));
        m.load();
        m
    }

    /// `DoorsList`: each door, open or closed, with Edit (its own screen), then Add a door.
    fn doors(self: &Rc<Self>) {
        let model = self.door_model();
        let p = panel(false);
        p.append(&eyebrow(&self.l("setup.doors"), false));
        for door in model.doors() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.append(&crate::doors::badge(&door.app, door.open, 30));
            let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
            v.set_hexpand(true);
            v.append(&text(&door.label, 15, 600, &["tafs-ink"]));
            let state = match &door.words {
                Some(w) => w.state.clone(),
                None => self.l(if door.open {
                    "setup.open"
                } else {
                    "door.status.closed"
                }),
            };
            v.append(&text(&state, 13, 400, &["tafs-soft"]));
            row.append(&v);
            let edit = gtk::Button::with_label(&self.l("setup.edit"));
            edit.add_css_class("tafs-link");
            edit.set_valign(gtk::Align::Center);
            let (me, id) = (Rc::downgrade(self), door.id.clone());
            edit.connect_clicked(move |_| {
                if let Some(c) = me.upgrade() {
                    c.edit_door(&id);
                }
            });
            row.append(&edit);
            p.append(&row);
        }
        // `Label(addDoor, systemImage: "plus")`: the + before its words.
        let add = gtk::Button::new();
        let face = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        face.append(&crate::widgets::symbol("list-add-symbolic", 16));
        face.append(&gtk::Label::new(Some(&self.l("setup.add_door"))));
        add.set_child(Some(&face));
        add.add_css_class("tafs-link");
        add.set_halign(gtk::Align::Start);
        add.set_sensitive(model.loaded() && !model.addable().is_empty());
        {
            let me = Rc::downgrade(self);
            add.connect_clicked(move |b| {
                let Some(c) = me.upgrade() else { return };
                let slack = {
                    let (w, name, b) = (c.knobs.w.clone(), c.knobs.taf.name.clone(), b.clone());
                    Rc::new(move |_app: tafs_core::doors::DoorApp| slack_sheet(&w, &name, &b))
                        as Rc<dyn Fn(tafs_core::doors::DoorApp)>
                };
                crate::doors::add_door_sheet(&c.door_model(), Some(slack), Rc::new(|_| {}), b);
            });
        }
        p.append(&add);
        self.column.append(&p);
    }

    /// A door's Edit: its own screen on top (`DoorsView.only`), whose Slack door's removal
    /// stops planque's subscription first.
    fn edit_door(self: &Rc<Self>, id: &str) {
        let Some(nav) = self.nav.upgrade() else {
            return;
        };
        let model = self.door_model();
        let title = model
            .doors()
            .into_iter()
            .find(|d| d.id == id)
            .and_then(|d| self.knobs.w.get("door.named", &[("door", &d.label)]).ok())
            .unwrap_or_else(|| self.l("door.title_plain"));
        let taf = &self.knobs.taf;
        let content = crate::doors::view(
            model,
            crate::doors::DoorsTaf {
                name: taf.name.clone(),
                initial: taf.initial.clone(),
                color: taf.color,
            },
            crate::doors::Opens {
                manages: true,
                my_email: taf.email.clone(),
                focus: Some(id.to_string()),
                removes_slack: true,
                add_slack: Some(Rc::new(|_| {})),
                knob: None,
            },
        );
        content.set_margin_start(16);
        content.set_margin_end(16);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        nav.push(&page(&title, &content));
    }

    /// Follow-ups it set itself: each, its end, and Remove.
    fn follow_ups(self: &Rc<Self>, k: &Knob, manages: bool, busy: bool) {
        let follow_ups = k.wakes_of("follow_up");
        if follow_ups.is_empty() {
            return;
        }
        let p = panel(false);
        p.append(&eyebrow(&self.l("setup.follow_ups"), false));
        for item in follow_ups {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
            v.set_hexpand(true);
            v.append(&text(item.line(), 15, 400, &["tafs-ink"]));
            if let Some(at) = item.expires.as_deref().and_then(knob::expires_words) {
                v.append(&text(&at, 12, 400, &["tafs-dim"]));
            }
            row.append(&v);
            if let (true, Some(id)) = (manages, &item.follow_up) {
                let b = outline(&self.l("setup.remove"));
                b.add_css_class("tafs-no");
                b.set_hexpand(false);
                b.set_valign(gtk::Align::Start);
                b.set_sensitive(!busy);
                let (knobs, id) = (self.knobs.clone(), id.clone());
                b.connect_clicked(move |_| knobs.remove_follow_up(&id));
                row.append(&b);
            }
            p.append(&row);
        }
        self.column.append(&p);
    }

    /// A place's row: its title and line, and its switch.
    fn place_row(
        &self,
        title: &str,
        line: &str,
        on: bool,
        disabled: bool,
        set: impl Fn(bool) + 'static,
    ) -> gtk::Box {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
        v.set_hexpand(true);
        v.append(&text(title, 15, 600, &["tafs-ink"]));
        if !line.is_empty() {
            v.append(&text(line, 13, 400, &["tafs-soft"]));
        }
        row.append(&v);
        let s = gtk::Switch::new();
        s.set_active(on);
        s.set_valign(gtk::Align::Center);
        s.add_css_class("tafs-ok-switch");
        s.set_sensitive(!disabled);
        s.update_property(&[gtk::accessible::Property::Label(title)]);
        s.connect_state_set(move |_, now| {
            if now != on {
                set(now);
            }
            glib::Propagation::Stop
        });
        row.append(&s);
        row
    }

    fn results_go_to(self: &Rc<Self>, k: &Knob) {
        let manages = self.knobs.taf.manages;
        let busy = self.knobs.busy.get();
        let live = knob::live_built_ins(self.knobs.results.borrow().as_deref(), Some(k));
        self.column
            .append(&text(&self.l("setup.at_least_one"), 13, 400, &["tafs-dim"]));
        self.pending(k);
        let p = panel(false);
        p.append(&eyebrow(&self.l("setup.from_tafs"), false));
        for (place, title, line) in
            knob::built_in_rows(&self.knobs.w, self.knobs.taf.email.as_deref()).unwrap_or_default()
        {
            let on = live.as_ref().is_some_and(|l| l.iter().any(|p| p == place));
            let last = on && live.as_ref().map(|l| l.len()).unwrap_or(0) <= 1;
            let (knobs, live) = (self.knobs.clone(), live.clone());
            p.append(&self.place_row(
                &title,
                &line,
                on,
                !manages || busy || live.is_none() || last,
                move |now| {
                    if let Some(sent) = live.as_ref().and_then(|l| knob::places(l, place, now)) {
                        knobs.set_places(sent);
                    }
                },
            ));
        }
        self.column.append(&p);
        let p = panel(false);
        p.append(&eyebrow(&self.l("setup.in_apps"), false));
        for item in k
            .results
            .iter()
            .filter(|r| !knob::BUILT_IN.contains(&r.place.as_deref().unwrap_or("")))
        {
            let toggled = item
                .size
                .as_ref()
                .and_then(|s| s.toggled())
                .map(|c| c.setting.clone());
            let (knobs, choice) = (self.knobs.clone(), toggled.clone());
            p.append(&self.place_row(
                &item.label,
                &item.words,
                item.on,
                !manages || busy || toggled.is_none(),
                move |_| {
                    if let Some(c) = choice.clone() {
                        knobs.change(move |s| *s = c);
                    }
                },
            ));
        }
        if manages && self.knobs.taf.may_use_coach {
            let b = gtk::Button::with_label(&self.l("setup.add_place"));
            b.add_css_class("tafs-link");
            b.set_halign(gtk::Align::Start);
            let me = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(c) = me.upgrade() {
                    c.coach();
                }
            });
            p.append(&b);
        }
        self.column.append(&p);
        self.not_sent(manages);
    }

    /// Replies given up on after their tries, with Send again.
    fn not_sent(&self, manages: bool) {
        let work = self.knobs.work.borrow();
        let unsent: Vec<&tafs_core::work::Request> = work
            .as_ref()
            .map(|w| {
                w.requests
                    .iter()
                    .filter(|r| r.deliveries.iter().any(|d| d.send_again))
                    .collect()
            })
            .unwrap_or_default();
        if unsent.is_empty() {
            return;
        }
        let p = panel(false);
        p.add_css_class("knocks-edge");
        p.append(&eyebrow(&self.l("setup.not_sent"), true));
        for r in unsent {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            v.append(&text(&r.line, 14, 400, &["tafs-ink"]));
            if manages {
                let sending = self.knobs.sending.borrow().contains(&r.id);
                let b = outline(&self.l(if sending {
                    "work.sending_again"
                } else {
                    "setup.send_again"
                }));
                b.set_sensitive(!sending);
                let place = r
                    .deliveries
                    .iter()
                    .find(|d| d.send_again)
                    .map(|d| d.send_place())
                    .unwrap_or_else(|| "door_reply".into());
                let (knobs, run) = (self.knobs.clone(), r.id.clone());
                b.connect_clicked(move |_| knobs.send_again(&run, &place));
                v.append(&b);
            }
            p.append(&v);
        }
        self.column.append(&p);
    }
}

/// `SlackDoorSheet`: a Slack door listens through one of the taf's Slack bot connections. Those
/// are planque's grants, which this computer doesn't read (its only planque call is the grants
/// proof tafs cloud forwards), so it has none to offer: the sheet says to connect it first, as
/// the iPad's does with none, and Cancel.
fn slack_sheet(w: &WordBook, taf: &str, parent: &impl IsA<gtk::Widget>) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let s = crate::sheet::new(&l("door.slack_door"), &l("common.cancel"), None, 480);
    s.column.append(&text(
        &w.get("door.slack.connect_first_in_connect_apps", &[("taf", taf)])
            .unwrap_or_default(),
        15,
        400,
        &["tafs-soft"],
    ));
    s.dialog.present(Some(parent));
}
