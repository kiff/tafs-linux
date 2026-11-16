//! Account's Coach section (`SettingsView`'s "Show suggestions") and the AI-data consent
//! (`AIConsentRow`, `AIConsentView`), read from tafs cloud as the session when Account opens, and
//! changed there unsigned. The consent's screen is also put up after sign-in while it isn't given
//! (`loadAIConsent`), unless Not now was answered this session. The reads are
//! `tafs-core/src/ai_consent.rs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::ai_consent::{self, AIConsent};
use tafs_core::words::WordBook;

use crate::widgets::{outline, primary, symbol, text};

/// Where they're read and changed.
#[derive(Clone)]
pub enum Source {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answers (`--account-reads`: `{"suggestions": …, "consent": …}`, each as
    /// tafs cloud answers it). Nothing is sent, and a change changes nothing.
    File(std::path::PathBuf),
}

/// What Account reads when it opens, and who may change it: Coach's section is for someone who
/// may use Coach (`mayUseCoach`), its switch for someone who manages Cards (`canManageCards`).
pub struct Reads {
    pub source: Source,
    pub may_use_coach: bool,
    pub manages_cards: bool,
}

thread_local! {
    /// Not now was answered this session (`aiConsentPutOff`): the screen isn't put up again by a
    /// read, only from Account.
    static PUT_OFF: Cell<bool> = const { Cell::new(false) };
    /// The consent's screen while it's open: it goes with the account it was read for.
    static OPEN: RefCell<Option<glib::WeakRef<adw::Dialog>>> = const { RefCell::new(None) };
}

/// A new account's consent is its own, and an account that goes takes its screen with it: nothing
/// of the last one's stays (`forgetAIConsent`), and an open screen closes, so its Allow can't be
/// answered for the account that left.
pub fn forget() {
    if let Some(open) = OPEN
        .with(|o| o.borrow_mut().take())
        .and_then(|w| w.upgrade())
    {
        open.close();
    }
    PUT_OFF.with(|p| p.set(false));
}

/// tafs cloud's words when it sent them (`CloudError.words`), else its error, else the app's line
/// for the status.
fn why(w: &WordBook, a: &tafs_core::signin::Answer) -> String {
    serde_json::from_slice::<serde_json::Value>(&a.body)
        .ok()
        .and_then(|v| v.get("words").and_then(|w| w.as_str()).map(String::from))
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            tafs_core::signin::refusal(a, Some(w))
                .words(w)
                .unwrap_or_default()
        })
}

fn file_part(path: &std::path::Path, key: &str) -> Option<Vec<u8>> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    serde_json::to_vec(v.get(key)?).ok()
}

/// A read: the answer's body when tafs cloud answered 200; none otherwise, and the part isn't drawn.
async fn get(source: &Source, path: &'static str, key: &'static str) -> Option<Vec<u8>> {
    let source = source.clone();
    gio::spawn_blocking(move || match &source {
        Source::Cloud { token, languages } => crate::kiff::cloud(token, languages, path)
            .ok()
            .filter(|a| a.status == 200)
            .map(|a| a.body),
        Source::File(p) => file_part(p, key),
    })
    .await
    .ok()
    .flatten()
}

/// A change: the answer, or why it didn't go. A screenshot's sends nothing (`Ok(None)`).
async fn send(
    w: &WordBook,
    source: &Source,
    method: &'static str,
    path: &'static str,
    body: Option<Vec<u8>>,
) -> Result<Option<tafs_core::signin::Answer>, String> {
    let (source, words) = (source.clone(), w.clone());
    gio::spawn_blocking(move || match &source {
        Source::Cloud { token, languages } => {
            crate::kiff::cloud_send(token, languages, method, path, body.as_deref())
                .map(Some)
                .map_err(|e| e.words(&words).unwrap_or_default())
        }
        Source::File(_) => Ok(None),
    })
    .await
    .unwrap_or_else(|_| Err(String::new()))
}

/// The Coach section's switch (`Toggle`): "Show suggestions" with tafs cloud's words under it,
/// enabled for someone who manages Cards, and why a change didn't go. Filled into `into` once the
/// setting is read; `shown` is called then, so the section appears.
pub fn suggestions(
    w: Rc<WordBook>,
    source: Source,
    enabled: bool,
    into: gtk::Box,
    shown: Rc<dyn Fn()>,
) {
    glib::spawn_future_local(async move {
        let Some(read) = get(&source, ai_consent::SUGGESTIONS_PATH, "suggestions")
            .await
            .and_then(|b| ai_consent::read_suggestions(&b))
        else {
            return;
        };
        let state = Rc::new(RefCell::new(read));
        let busy = Rc::new(Cell::new(false));
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let labels = gtk::Box::new(gtk::Orientation::Vertical, 2);
        labels.set_hexpand(true);
        labels.append(&text(
            &w.line("settings.show_suggestions").unwrap_or_default(),
            17,
            400,
            &["tafs-ink"],
        ));
        let words = text("", 13, 400, &["tafs-soft"]);
        labels.append(&words);
        row.append(&labels);
        let switch = gtk::Switch::new();
        switch.set_valign(gtk::Align::Center);
        switch.update_property(&[gtk::accessible::Property::Label(
            &w.line("settings.show_suggestions").unwrap_or_default(),
        )]);
        row.append(&switch);
        let problem = text("", 13, 400, &["tafs-no"]);
        problem.set_visible(false);
        let column = gtk::Box::new(gtk::Orientation::Vertical, 6);
        column.append(&row);
        column.append(&problem);
        into.append(&column);
        let draw = {
            let (state, busy, switch, words) =
                (state.clone(), busy.clone(), switch.clone(), words.clone());
            move || {
                let s = state.borrow();
                switch.set_active(s.show);
                words.set_label(&s.words);
                words.set_visible(!s.words.is_empty());
                switch.set_sensitive(enabled && !busy.get());
            }
        };
        draw();
        let draw = Rc::new(draw);
        switch.connect_state_set(move |_, on| {
            if busy.get() || on == state.borrow().show {
                return glib::Propagation::Proceed;
            }
            busy.set(true);
            problem.set_visible(false);
            draw();
            let (w, source, state, busy, problem, draw) = (
                w.clone(),
                source.clone(),
                state.clone(),
                busy.clone(),
                problem.clone(),
                draw.clone(),
            );
            glib::spawn_future_local(async move {
                let sent = send(
                    &w,
                    &source,
                    "PUT",
                    ai_consent::SUGGESTIONS_PATH,
                    Some(ai_consent::suggestions_body(on)),
                )
                .await;
                busy.set(false);
                match sent {
                    Ok(Some(a)) if a.status == 200 => {
                        if let Some(now) = ai_consent::read_suggestions(&a.body) {
                            *state.borrow_mut() = now;
                        }
                    }
                    Ok(Some(a)) => {
                        problem.set_label(
                            &w.get("settings.suggestions_not_changed", &[("why", &why(&w, &a))])
                                .unwrap_or_default(),
                        );
                        problem.set_visible(true);
                    }
                    Ok(None) => {}
                    Err(e) => {
                        problem.set_label(
                            &w.get("settings.suggestions_not_changed", &[("why", &e)])
                                .unwrap_or_default(),
                        );
                        problem.set_visible(true);
                    }
                }
                draw();
            });
            glib::Propagation::Stop
        });
        shown();
    });
}

/// The consent's state while Account shows it: what tafs cloud last said, an answer on its way,
/// and why one didn't go.
struct Consent {
    w: Rc<WordBook>,
    source: Source,
    now: RefCell<AIConsent>,
    busy: Cell<bool>,
    error: RefCell<Option<String>>,
    redraw: RefCell<Option<Rc<dyn Fn()>>>,
}

impl Consent {
    fn redraw(&self) {
        let f = self.redraw.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }
}

/// `AIConsentRow`: once given, tafs cloud's words for it, with Withdraw; not given, its title in
/// the amber, which opens the screen; why an answer didn't go. Filled into `into` once read.
pub fn consent_row(w: Rc<WordBook>, source: Source, into: gtk::Box, shown: Rc<dyn Fn()>) {
    glib::spawn_future_local(async move {
        let Some(read) = get(&source, ai_consent::CONSENT_PATH, "consent")
            .await
            .and_then(|b| ai_consent::read_consent(&b))
            .filter(AIConsent::drawable)
        else {
            return;
        };
        let c = Rc::new(Consent {
            w,
            source,
            now: RefCell::new(read),
            busy: Cell::new(false),
            error: RefCell::new(None),
            redraw: RefCell::new(None),
        });
        let draw: Rc<dyn Fn()> = {
            let (c2, into) = (Rc::downgrade(&c), into.clone());
            Rc::new(move || {
                let Some(c) = c2.upgrade() else { return };
                while let Some(child) = into.first_child() {
                    into.remove(&child);
                }
                let now = c.now.borrow().clone();
                if now.given {
                    let given = text(&now.words.given, 17, 400, &["tafs-ink"]);
                    given.set_widget_name("settings.ai-consent");
                    into.append(&given);
                    if let Some(words) = now.withdraw() {
                        let b = crate::account::row_button(words);
                        b.add_css_class("no");
                        b.set_sensitive(!c.busy.get());
                        let c = c.clone();
                        b.connect_clicked(move |_| withdraw(&c));
                        into.append(&b);
                    }
                } else {
                    let b = crate::account::row_button(&now.words.title);
                    b.add_css_class("amber-text");
                    b.set_widget_name("settings.ai-consent");
                    let c = c.clone();
                    b.connect_clicked(move |b| present(&c, b.upcast_ref()));
                    into.append(&b);
                }
                let error = c.error.borrow().clone();
                if let Some(why) = error {
                    into.append(&text(&why, 13, 400, &["tafs-no"]));
                }
            })
        };
        *c.redraw.borrow_mut() = Some(draw.clone());
        draw();
        shown();
    });
}

/// Withdraw: from then on, nothing this person writes reaches the model.
fn withdraw(c: &Rc<Consent>) {
    c.busy.set(true);
    c.redraw();
    let c = c.clone();
    glib::spawn_future_local(async move {
        let sent = send(&c.w, &c.source, "DELETE", ai_consent::CONSENT_PATH, None).await;
        c.busy.set(false);
        match sent {
            Ok(Some(a)) if a.status == 200 => {
                if let Some(now) = ai_consent::read_consent(&a.body) {
                    *c.now.borrow_mut() = now;
                }
                *c.error.borrow_mut() = None;
                PUT_OFF.with(|p| p.set(true));
            }
            Ok(Some(a)) => *c.error.borrow_mut() = Some(why(&c.w, &a)),
            Ok(None) => {}
            Err(e) => *c.error.borrow_mut() = Some(e),
        }
        c.redraw();
    });
}

/// After sign-in (`loadAIConsent`): the consent, read; not given, its screen over the window, unless
/// Not now was answered this session.
pub fn ask_after_sign_in(w: Rc<WordBook>, source: Source, window: gtk::Widget) {
    if PUT_OFF.with(Cell::get) {
        return;
    }
    glib::spawn_future_local(async move {
        let Some(read) = get(&source, ai_consent::CONSENT_PATH, "consent")
            .await
            .and_then(|b| ai_consent::read_consent(&b))
            .filter(|c| c.drawable() && !c.given)
        else {
            return;
        };
        let c = Rc::new(Consent {
            w,
            source,
            now: RefCell::new(read),
            busy: Cell::new(false),
            error: RefCell::new(None),
            redraw: RefCell::new(None),
        });
        present(&c, &window);
    });
}

/// A turn refused for want of the consent (`consentRequired`, tafs cloud's `ai_consent_required`
/// with the consent in its refusal): its screen, now, whatever was put off before. A refusal whose
/// consent can't be drawn puts nothing up.
pub fn required(w: Rc<WordBook>, source: Source, refusal: &[u8], window: gtk::Widget) {
    let Some(read) = ai_consent::read_consent(refusal).filter(AIConsent::drawable) else {
        return;
    };
    // Already up: the one screen stays.
    if OPEN.with(|o| {
        o.borrow()
            .as_ref()
            .and_then(|w| w.upgrade())
            .is_some_and(|d| d.parent().is_some())
    }) {
        return;
    }
    let c = Rc::new(Consent {
        w,
        source,
        now: RefCell::new(read),
        busy: Cell::new(false),
        error: RefCell::new(None),
        redraw: RefCell::new(None),
    });
    present(&c, &window);
}

/// `AIConsentView`: tafs cloud's words, as sent, then Allow and Not now. Nothing the person writes
/// reaches the model before Allow; Not now closes it for this session.
fn present(c: &Rc<Consent>, parent: &gtk::Widget) {
    *c.error.borrow_mut() = None;
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(24);
    column.set_margin_end(24);
    column.set_margin_top(12);
    column.set_margin_bottom(24);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_propagate_natural_height(true);
    scroller.set_child(Some(&column));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(560);
    let fill: Fill = Rc::new(RefCell::new(None));
    let fill_now: Rc<dyn Fn()> = {
        let (c, column, dialog, fill) = (c.clone(), column.clone(), dialog.clone(), fill.clone());
        Rc::new(move || {
            while let Some(child) = column.first_child() {
                column.remove(&child);
            }
            let now = c.now.borrow().clone();
            let mark = symbol("tafs-sparkles-symbolic", 28);
            mark.add_css_class("tafs-amber-text");
            mark.set_halign(gtk::Align::Start);
            mark.update_state(&[gtk::accessible::State::Hidden(true)]);
            column.append(&mark);
            let title = text(&now.words.title, 24, 700, &["tafs-ink"]);
            title.set_widget_name("consent.title");
            column.append(&title);
            let lines = gtk::Box::new(gtk::Orientation::Vertical, 12);
            for line in &now.words.lines {
                lines.append(&text(line, 16, 400, &["tafs-ink"]));
            }
            column.append(&lines);
            let error = c.error.borrow().clone();
            if let Some(why) = error {
                column.append(&text(&why, 14, 400, &["tafs-no"]));
            }
            let allow_button = primary(&now.words.allow);
            allow_button.set_widget_name("consent.allow");
            allow_button.set_sensitive(!c.busy.get());
            {
                let (c, dialog, fill) = (c.clone(), dialog.clone(), fill.clone());
                allow_button.connect_clicked(move |_| allow(&c, &dialog, &fill));
            }
            column.append(&allow_button);
            let not_now = outline(&now.words.not_now);
            not_now.set_widget_name("consent.not-now");
            not_now.set_sensitive(!c.busy.get());
            {
                let (c, dialog) = (c.clone(), dialog.clone());
                not_now.connect_clicked(move |_| {
                    PUT_OFF.with(|p| p.set(true));
                    *c.error.borrow_mut() = None;
                    dialog.close();
                });
            }
            column.append(&not_now);
        })
    };
    *fill.borrow_mut() = Some(fill_now.clone());
    fill_now();
    // Closing it is Not now, as the iPad's sheet dismissed.
    {
        let (c, fill) = (c.clone(), fill.clone());
        dialog.connect_closed(move |_| {
            // Breaks the screen's own cycle once it's gone.
            fill.borrow_mut().take();
            if !c.now.borrow().given {
                PUT_OFF.with(|p| p.set(true));
            }
            c.redraw();
        });
    }
    OPEN.with(|o| *o.borrow_mut() = Some(dialog.downgrade()));
    dialog.present(Some(parent));
}

type Fill = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

fn refill(fill: &Fill) {
    let f = fill.borrow().clone();
    if let Some(f) = f {
        f();
    }
}

/// Allow, for the version of the words on the screen. An older screen's yes is refused (409) with
/// the new words, which then show in its place.
fn allow(c: &Rc<Consent>, dialog: &adw::Dialog, fill: &Fill) {
    c.busy.set(true);
    refill(fill);
    let version = c.now.borrow().version.clone();
    let (c, dialog, fill) = (c.clone(), dialog.clone(), fill.clone());
    glib::spawn_future_local(async move {
        let sent = send(
            &c.w,
            &c.source,
            "POST",
            ai_consent::CONSENT_PATH,
            Some(ai_consent::allow_body(&version)),
        )
        .await;
        c.busy.set(false);
        match sent {
            Ok(Some(a)) if a.status == 200 => {
                if let Some(given) = ai_consent::read_consent(&a.body) {
                    *c.now.borrow_mut() = given;
                }
                *c.error.borrow_mut() = None;
                c.redraw();
                dialog.close();
                return;
            }
            Ok(Some(a)) if a.status == 409 => {
                *c.error.borrow_mut() = Some(why(&c.w, &a));
                let newer = match ai_consent::read_consent(&a.body) {
                    Some(n) => Some(n),
                    None => get(&c.source, ai_consent::CONSENT_PATH, "consent")
                        .await
                        .and_then(|b| ai_consent::read_consent(&b)),
                };
                if let Some(n) = newer.filter(AIConsent::drawable) {
                    *c.now.borrow_mut() = n;
                }
            }
            Ok(Some(a)) => *c.error.borrow_mut() = Some(why(&c.w, &a)),
            // A screenshot's Allow sends nothing and leaves the screen as it is.
            Ok(None) => {}
            Err(e) => *c.error.borrow_mut() = Some(e),
        }
        refill(&fill);
        c.redraw();
    });
}
