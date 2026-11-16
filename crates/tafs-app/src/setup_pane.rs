//! A tafs cloud taf's Setup tab (`TafDetailView.setup(_:)`), signed in with KIFF: its Card unit
//! requests, its job, what it remembers (`MemorySummaryPanel`, and `MemoryPanel` behind See all,
//! where a note is forgotten), and Remove for good, dimmed. tafs cloud's Setup read
//! (`GET /v1/tafs/{id}/setup`) is made without planque's grants proof, which this computer can't
//! sign, so tafs cloud leaves the connections out. The reads are `tafs-core/src/setup.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::setup::{self, Note, SetupView};
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, outline, panel, text};

/// Something to call again later: the Setup's read, a sheet's redraw.
type Later = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// Where a taf's Setup and notes are read, and a note forgotten.
#[derive(Clone)]
pub enum SetupSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answers: the Setup read, and the notes (`--memory`) when given. Nothing is
    /// sent, and Forget changes nothing.
    File {
        setup: std::path::PathBuf,
        notes: Option<std::path::PathBuf>,
    },
    None,
}

/// The taf the pane is for: tafs cloud's id, its job as Home says it, whether this person
/// manages it, and the Card unit requests' Coach button (`mayUseCoach` and `manages`).
pub struct SetupTaf {
    pub id: String,
    pub job: String,
    pub manages: bool,
    pub discuss: bool,
}

fn file_answer(path: &std::path::Path) -> Option<Vec<u8>> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok()
}

async fn read_setup(w: &WordBook, source: &SetupSource, id: &str) -> Result<SetupView, String> {
    let (words, source, id) = (w.clone(), source.clone(), id.to_string());
    gio::spawn_blocking(move || {
        let body = match &source {
            SetupSource::Cloud { token, languages } => {
                let a = crate::kiff::cloud(token, languages, &setup::path(&id))
                    .map_err(|e| e.words(&words).unwrap_or_default())?;
                if a.status != 200 {
                    return Err(tafs_core::signin::refusal(&a, Some(&words))
                        .words(&words)
                        .unwrap_or_default());
                }
                a.body
            }
            SetupSource::File { setup, .. } => file_answer(setup).ok_or_default()?,
            SetupSource::None => return Err(String::new()),
        };
        setup::read(&body)
            .ok_or_else(|| tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
    })
    .await
    .unwrap_or_else(|_| Err(String::new()))
}

trait OkOrDefault<T> {
    fn ok_or_default(self) -> Result<T, String>;
}

impl<T> OkOrDefault<T> for Option<T> {
    fn ok_or_default(self) -> Result<T, String> {
        self.ok_or_else(String::new)
    }
}

async fn read_notes(w: &WordBook, source: &SetupSource, id: &str) -> Result<Vec<Note>, String> {
    let (words, source, id) = (w.clone(), source.clone(), id.to_string());
    let read = gio::spawn_blocking(move || {
        let body = match &source {
            SetupSource::Cloud { token, languages } => {
                let a = crate::kiff::cloud(token, languages, &setup::memory_path(&id, true))
                    .map_err(|e| e.words(&words).unwrap_or_default())?;
                if a.status != 200 {
                    return Err(tafs_core::signin::refusal(&a, Some(&words))
                        .words(&words)
                        .unwrap_or_default());
                }
                a.body
            }
            SetupSource::File {
                notes: Some(notes), ..
            } => file_answer(notes).ok_or_default()?,
            _ => return Err(String::new()),
        };
        setup::read_notes(&body)
            .ok_or_else(|| tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
    })
    .await
    .unwrap_or_else(|_| Err(String::new()));
    read.map_err(|why| {
        w.get("taf.memory.not_loaded", &[("why", &why)])
            .unwrap_or(why)
    })
}

/// The Setup tab's pane: read when it's first shown, and again by Try again.
pub fn pane(w: Rc<WordBook>, taf: Rc<SetupTaf>, source: SetupSource) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 16);
    root.set_margin_top(4);
    root.set_margin_bottom(12);
    let load: Later = Rc::new(RefCell::new(None));
    let load_now: Rc<dyn Fn()> = {
        let (w, taf, source, root, load) = (
            w.clone(),
            taf.clone(),
            source.clone(),
            root.clone(),
            load.clone(),
        );
        Rc::new(move || {
            while let Some(c) = root.first_child() {
                root.remove(&c);
            }
            let spinner = gtk::Spinner::new();
            spinner.start();
            spinner.set_halign(gtk::Align::Start);
            root.append(&spinner);
            let (w, taf, source, root, load) = (
                w.clone(),
                taf.clone(),
                source.clone(),
                root.clone(),
                load.clone(),
            );
            glib::spawn_future_local(async move {
                let read = read_setup(&w, &source, &taf.id).await;
                while let Some(c) = root.first_child() {
                    root.remove(&c);
                }
                draw(&w, &taf, &source, &root, read, &load);
            });
        })
    };
    *load.borrow_mut() = Some(load_now.clone());
    load_now();
    root
}

fn draw(
    w: &Rc<WordBook>,
    taf: &Rc<SetupTaf>,
    source: &SetupSource,
    root: &gtk::Box,
    read: Result<SetupView, String>,
    load: &Later,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let view = match read {
        Ok(view) => Some(view),
        Err(why) => {
            // Why it couldn't be read, and Try again (the tools panel's, `tafSetupErrors`).
            root.append(&text(&why, 14, 400, &["tafs-no"]));
            let again = outline(&l("common.try_again"));
            again.set_hexpand(false);
            again.set_halign(gtk::Align::Start);
            let load = load.clone();
            again.connect_clicked(move |_| {
                let f = load.borrow().clone();
                if let Some(f) = f {
                    f();
                }
            });
            root.append(&again);
            None
        }
    };
    // Its Card unit requests first, as Needs you draws them.
    for ask in view
        .as_ref()
        .and_then(|v| v.card_unit_exceptions.as_ref())
        .into_iter()
        .flatten()
    {
        let lines = tafs_core::needs_you::card_unit_lines(ask, taf.discuss);
        root.append(&crate::needs_you::card_unit_panel(&lines));
    }
    // The job, in Home's words. Improve in Coach is Coach's, which isn't on this computer.
    let job = panel(false);
    job.append(&eyebrow(&l("setup.job"), false));
    job.append(&text(&taf.job, 15, 400, &["tafs-ink"]));
    root.append(&job);
    // What it remembers (`MemorySummaryPanel`): tafs cloud's line for it, its latest note, See all.
    if let Some(view) = &view {
        let memory = panel(false);
        memory.append(&eyebrow(&l("taf.memory.title"), false));
        memory.append(&text(&view.memory.title, 15, 600, &["tafs-ink"]));
        if let Some(latest) = view.memory.latest.as_ref().filter(|s| !s.is_empty()) {
            let line = text(latest, 14, 400, &["tafs-soft"]);
            line.set_lines(2);
            line.set_ellipsize(gtk::pango::EllipsizeMode::End);
            memory.append(&line);
        }
        let all = gtk::Button::with_label(&l("taf.memory.see_all"));
        all.add_css_class("tafs-link");
        all.set_halign(gtk::Align::Start);
        {
            let (w, taf, source, load) = (w.clone(), taf.clone(), source.clone(), load.clone());
            all.connect_clicked(move |b| notes_sheet(&w, &taf, &source, b.upcast_ref(), &load));
        }
        memory.append(&all);
        root.append(&memory);
    }
    // Remove for good, for whoever manages it: signed, so it's there, dimmed.
    if taf.manages {
        let remove = outline(&l("setup.remove_for_good"));
        remove.add_css_class("tafs-no");
        remove.set_sensitive(false);
        root.append(&remove);
    }
}

/// `MemoryPanel(loadAll: true)` behind See all: every note, newest first, each with Forget, which
/// asks first; then what the notes are for. The Setup read again once it's closed.
fn notes_sheet(
    w: &Rc<WordBook>,
    taf: &Rc<SetupTaf>,
    source: &SetupSource,
    parent: &gtk::Widget,
    load: &Later,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(12);
    column.set_margin_bottom(20);
    let p = panel(false);
    column.append(&p);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_title(&l("taf.memory.title"));
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(560);
    dialog.set_content_height(600);
    {
        let load = load.clone();
        dialog.connect_closed(move |_| {
            let f = load.borrow().clone();
            if let Some(f) = f {
                f();
            }
        });
    }
    let notes: Rc<RefCell<Option<Vec<Note>>>> = Rc::new(RefCell::new(None));
    let error: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let redraw: Later = Rc::new(RefCell::new(None));
    let draw: Rc<dyn Fn()> = {
        let (w, taf, source, p, notes, error, redraw, dialog) = (
            w.clone(),
            taf.clone(),
            source.clone(),
            p.clone(),
            notes.clone(),
            error.clone(),
            redraw.clone(),
            dialog.clone(),
        );
        Rc::new(move || {
            let l = |k: &str| w.line(k).unwrap_or_default();
            while let Some(c) = p.first_child() {
                p.remove(&c);
            }
            p.append(&eyebrow(&l("taf.memory.title"), false));
            let read = notes.borrow().clone();
            match &read {
                Some(list) => {
                    if list.is_empty() {
                        p.append(&text(&l("taf.memory.empty"), 14, 400, &["tafs-soft"]));
                    }
                    for note in list {
                        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                        let v = gtk::Box::new(gtk::Orientation::Vertical, 3);
                        v.set_hexpand(true);
                        v.append(&text(&note.content, 15, 400, &["tafs-ink"]));
                        let when = note
                            .updated_at
                            .with_timezone(&chrono::Local)
                            .format("%H:%M")
                            .to_string();
                        v.append(&text(
                            &format!("{} · {when}", note.title()),
                            12,
                            400,
                            &["tafs-soft"],
                        ));
                        row.append(&v);
                        let forget = gtk::Button::with_label(&l("taf.memory.forget"));
                        forget.add_css_class("tafs-plain");
                        forget.add_css_class("tafs-soft");
                        forget.set_valign(gtk::Align::Start);
                        forget.update_property(&[gtk::accessible::Property::Label(
                            &w.get("taf.memory.forget_label", &[("note", &note.content)])
                                .unwrap_or_default(),
                        )]);
                        let (w, taf, source, notes, error, redraw, dialog, note) = (
                            w.clone(),
                            taf.clone(),
                            source.clone(),
                            notes.clone(),
                            error.clone(),
                            redraw.clone(),
                            dialog.clone(),
                            note.clone(),
                        );
                        forget.connect_clicked(move |_| {
                            ask_forget(&w, &taf, &source, &note, &notes, &error, &redraw, &dialog)
                        });
                        row.append(&forget);
                        p.append(&row);
                    }
                    p.append(&text(&l("taf.memory.explains"), 12, 400, &["tafs-soft"]));
                }
                None if error.borrow().is_none() => {
                    let s = gtk::Spinner::new();
                    s.start();
                    s.set_halign(gtk::Align::Start);
                    p.append(&s);
                }
                None => {}
            }
            if let Some(why) = error.borrow().clone() {
                p.append(&text(&why, 13, 400, &["tafs-no"]));
            }
        })
    };
    *redraw.borrow_mut() = Some(draw.clone());
    draw();
    {
        let (w, taf, source, notes, error, draw) = (
            w.clone(),
            taf.clone(),
            source.clone(),
            notes.clone(),
            error.clone(),
            draw.clone(),
        );
        glib::spawn_future_local(async move {
            match read_notes(&w, &source, &taf.id).await {
                Ok(list) => *notes.borrow_mut() = Some(list),
                Err(why) => *error.borrow_mut() = Some(why),
            }
            draw();
        });
    }
    {
        // Breaks the sheet's own cycle once it's gone.
        let redraw = redraw.clone();
        dialog.connect_closed(move |_| {
            redraw.borrow_mut().take();
        });
    }
    dialog.present(Some(parent));
}

/// Forget asks first (`forgetTitle`, the note, Forget in red, Cancel); its yes takes the note away
/// at once and puts it back, with why, when tafs cloud refuses. A note already gone counts as
/// forgotten.
#[allow(clippy::too_many_arguments)]
fn ask_forget(
    w: &Rc<WordBook>,
    taf: &Rc<SetupTaf>,
    source: &SetupSource,
    note: &Note,
    notes: &Rc<RefCell<Option<Vec<Note>>>>,
    error: &Rc<RefCell<Option<String>>>,
    redraw: &Later,
    parent: &adw::Dialog,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let ask = adw::AlertDialog::new(
        Some(&l("taf.memory.forget_title")),
        Some(
            &w.get("taf.memory.forget_body", &[("note", &note.content)])
                .unwrap_or_default(),
        ),
    );
    ask.add_response("cancel", &l("common.cancel"));
    ask.add_response("forget", &l("taf.memory.forget"));
    ask.set_response_appearance("forget", adw::ResponseAppearance::Destructive);
    ask.set_default_response(Some("cancel"));
    ask.set_close_response("cancel");
    let (w, taf, source, note, notes, error, redraw) = (
        w.clone(),
        taf.clone(),
        source.clone(),
        note.clone(),
        notes.clone(),
        error.clone(),
        redraw.clone(),
    );
    ask.connect_response(None, move |_, response| {
        if response != "forget" {
            return;
        }
        let again = || {
            let f = redraw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        };
        if let Some(list) = notes.borrow_mut().as_mut() {
            list.retain(|n| n.id != note.id);
        }
        *error.borrow_mut() = None;
        again();
        let (w, taf, source, note, notes, error, redraw) = (
            w.clone(),
            taf.clone(),
            source.clone(),
            note.clone(),
            notes.clone(),
            error.clone(),
            redraw.clone(),
        );
        glib::spawn_future_local(async move {
            let (words, src, id, nid) = (
                (*w).clone(),
                source.clone(),
                taf.id.clone(),
                note.id.clone(),
            );
            let sent = gio::spawn_blocking(move || -> Result<(), String> {
                match &src {
                    SetupSource::Cloud { token, languages } => {
                        let a = crate::kiff::cloud_send(
                            token,
                            languages,
                            "DELETE",
                            &setup::forget_path(&id, &nid),
                            None,
                        )
                        .map_err(|e| e.words(&words).unwrap_or_default())?;
                        match a.status {
                            200 | 204 | 404 => Ok(()),
                            _ => Err(tafs_core::signin::refusal(&a, Some(&words))
                                .words(&words)
                                .unwrap_or_default()),
                        }
                    }
                    _ => Ok(()),
                }
            })
            .await
            .unwrap_or_else(|_| Err(String::new()));
            if let Err(why) = sent {
                // Put back, newest first.
                if let Some(list) = notes.borrow_mut().as_mut() {
                    list.push(note.clone());
                    list.sort_by_key(|n| std::cmp::Reverse(n.updated_at));
                }
                *error.borrow_mut() = Some(
                    w.get("taf.memory.not_forgotten", &[("why", &why)])
                        .unwrap_or(why),
                );
            }
            let f = redraw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        });
    });
    ask.present(Some(parent));
}
