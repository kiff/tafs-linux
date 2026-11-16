//! What waits for the owner at a taf's doors, on its Activity (`TafKnocksPanel`,
//! `TafPresentationViews.swift`): strangers' knocks (Do it, Pair them, Ignore), then a paired
//! sender's message kept from its knock, apart and calmly. Each knock is `KnockCard`
//! (`DoorsView.swift`) and Do it opens `DoItSheet`. The doors are read as the session when the
//! Activity opens (`GET /v1/tafs/{id}/doors`), tried again quietly after 1 s and 3 s (`DoorsModel`);
//! an answer is `POST /v1/tafs/{id}/doors/knocks/{knock}`, unsigned. The lines are
//! `tafs-core/src/doors.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::doors::{self, Doors, Knock};
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, panel, primary, text};

/// Where a taf's doors are read and its knocks answered.
#[derive(Clone)]
pub enum DoorsSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answer (`--doors`): read, and its answers send nothing.
    File(std::path::PathBuf),
    None,
}

/// The panel's state (`DoorsModel`): the doors once read, why an answer or a read failed, and
/// whether an answer is on its way.
#[derive(Default)]
struct State {
    doors: Option<Doors>,
    error: Option<String>,
    busy: bool,
}

type Redraw = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

#[derive(Clone)]
struct Knocks {
    w: Rc<WordBook>,
    id: String,
    taf: Option<String>,
    source: DoorsSource,
    state: Rc<RefCell<State>>,
    redraw: Redraw,
}

impl Knocks {
    fn redraw(&self) {
        let f = self.redraw.borrow().clone();
        if let Some(f) = f {
            f();
        }
    }

    /// `DoorsModel.load`: read, trying again quietly when a read fails; only a read that keeps
    /// failing says so.
    fn load(&self) {
        let me = self.clone();
        glib::spawn_future_local(async move {
            let (source, id) = (me.source.clone(), me.id.clone());
            let read = gio::spawn_blocking(move || -> Option<Doors> {
                for (attempt, wait) in [0u64, 1, 3].into_iter().enumerate() {
                    if attempt > 0 {
                        std::thread::sleep(std::time::Duration::from_secs(wait));
                    }
                    let body = match &source {
                        DoorsSource::Cloud { token, languages } => {
                            match crate::kiff::cloud(token, languages, &doors::path(&id)) {
                                Ok(a) if a.status == 200 => a.body,
                                _ => continue,
                            }
                        }
                        DoorsSource::File(file) => std::fs::read(file).ok()?,
                        DoorsSource::None => return None,
                    };
                    if let Some(d) = doors::read(&body) {
                        return Some(d);
                    }
                }
                None
            })
            .await
            .ok()
            .flatten();
            {
                let mut s = me.state.borrow_mut();
                match read {
                    Some(d) => {
                        s.doors = Some(d);
                        s.error = None;
                    }
                    None => s.error = me.w.line("door.not_read").ok(),
                }
            }
            me.redraw();
        });
    }

    /// `DoorsModel.answer`: "do" (with the owner's words), "pair" or "ignore". Pairing keeps the
    /// message waiting, so the doors are read again; anything else takes the knock away.
    fn answer(&self, knock: &Knock, action: &'static str, say: Option<String>, done: Rc<dyn Fn()>) {
        if self.state.borrow().busy {
            return;
        }
        self.state.borrow_mut().busy = true;
        self.redraw();
        let me = self.clone();
        let knock_id = knock.id.clone();
        glib::spawn_future_local(async move {
            let (words, source, id, k) = (
                (*me.w).clone(),
                me.source.clone(),
                me.id.clone(),
                knock_id.clone(),
            );
            let sent = gio::spawn_blocking(move || -> Result<bool, String> {
                match &source {
                    DoorsSource::Cloud { token, languages } => {
                        let a = crate::kiff::answer_knock(
                            token,
                            languages,
                            &doors::knock_path(&id, &k),
                            &doors::answer_body(action, say.as_deref()),
                        )
                        .map_err(|e| e.words(&words).unwrap_or_default())?;
                        if a.status == 200 && doors::answered(&a.body) {
                            return Ok(true);
                        }
                        // A 429 without tafs cloud's words: the app's own line, as the chat says it.
                        let cloud_words = serde_json::from_slice::<serde_json::Value>(&a.body)
                            .ok()
                            .and_then(|v| {
                                v.get("words").and_then(|w| w.as_str()).map(String::from)
                            });
                        if a.status == 429 && cloud_words.is_none() {
                            return Err(words.line("door.knock.too_many").unwrap_or_default());
                        }
                        Err(tafs_core::signin::refusal(&a, Some(&words))
                            .words(&words)
                            .unwrap_or_default())
                    }
                    // A screenshot's knocks: nothing is sent, and nothing changes.
                    _ => Ok(false),
                }
            })
            .await
            .unwrap_or_else(|_| Err(String::new()));
            let reload = {
                let mut s = me.state.borrow_mut();
                s.busy = false;
                match sent {
                    Ok(true) => {
                        s.error = None;
                        if action == "pair" {
                            true
                        } else {
                            if let Some(d) = s.doors.as_mut() {
                                d.knocks.retain(|k| k.id != knock_id);
                            }
                            false
                        }
                    }
                    Ok(false) => false,
                    Err(why) => {
                        s.error = Some(why);
                        false
                    }
                }
            };
            me.redraw();
            done();
            if reload {
                me.load();
            }
        });
    }
}

/// `KnockCard` for a message (the panel draws no door lines): who wrote, what, and the owner's
/// answers.
fn knock_card(k: &Knocks, knock: &Knock, busy: bool, parent: &gtk::Box) -> gtk::Box {
    let w = &k.w;
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("tafs-knock");
    if knock.paired {
        card.add_css_class("paired");
    }
    let Ok(lines) = doors::knock_lines(w, knock) else {
        return card;
    };
    card.append(&text(
        &lines.head,
        10,
        600,
        &[
            "tafs-mono",
            if knock.paired {
                "tafs-soft"
            } else {
                "tafs-amber-text"
            },
        ],
    ));
    if let Some(line) = &lines.paired_line {
        card.append(&text(line, 12, 400, &["tafs-dim"]));
    }
    card.append(&text(&lines.wrote, 14, 600, &["tafs-ink"]));
    let body = text(&lines.text, 14, 400, &["tafs-soft"]);
    body.set_selectable(true);
    // `lineLimit(3)` on the Activity.
    body.set_lines(3);
    body.set_ellipsize(gtk::pango::EllipsizeMode::End);
    card.append(&body);
    if let Some(a) = &lines.attachments {
        card.append(&text(a, 12, 400, &["tafs-dim"]));
    }
    if let Some(note) = &lines.proof_note {
        card.append(&text(note, 12, 400, &["tafs-dim"]));
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    if let Some(label) = &lines.do_it {
        let b = gtk::Button::with_label(label);
        b.add_css_class("tafs-knock-do");
        b.set_sensitive(!busy);
        let (k, knock, parent) = (k.clone(), knock.clone(), parent.clone());
        b.connect_clicked(move |_| do_it_sheet(&k, &knock, &parent));
        row.append(&b);
    }
    if let Some(label) = &lines.pair {
        let b = gtk::Button::with_label(label);
        b.add_css_class("tafs-knock-pair");
        b.set_sensitive(!busy);
        let (k, knock) = (k.clone(), knock.clone());
        b.connect_clicked(move |_| k.answer(&knock, "pair", None, Rc::new(|| {})));
        row.append(&b);
    }
    let ignore = gtk::Button::with_label(&lines.ignore);
    ignore.add_css_class("tafs-knock-ignore");
    ignore.set_sensitive(!busy);
    {
        let (k, knock) = (k.clone(), knock.clone());
        ignore.connect_clicked(move |_| k.answer(&knock, "ignore", None, Rc::new(|| {})));
    }
    row.append(&ignore);
    card.append(&row);
    card
}

/// `DoItSheet`: what the owner tells the taf to do about a knock; their words are the instruction.
fn do_it_sheet(k: &Knocks, knock: &Knock, parent: &gtk::Box) {
    let Ok(lines) = doors::do_it_lines(&k.w, knock, k.taf.as_deref()) else {
        return;
    };
    // `onAppear`: no error from before.
    k.state.borrow_mut().error = None;
    k.redraw();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(8);
    column.set_margin_bottom(20);
    column.append(&text(&lines.title, 22, 700, &["tafs-ink"]));
    column.append(&text(&lines.intro, 14, 400, &["tafs-soft"]));
    let say = gtk::TextView::new();
    say.set_wrap_mode(gtk::WrapMode::WordChar);
    say.buffer().set_text(&lines.default_say);
    say.add_css_class("tafs-do-it-field");
    say.set_size_request(-1, 110);
    say.update_property(&[gtk::accessible::Property::Label(&lines.placeholder)]);
    column.append(&say);
    let why = text("", 14, 400, &["tafs-no"]);
    why.set_visible(false);
    column.append(&why);
    let send = primary(&lines.send);
    column.append(&send);
    let cancel = gtk::Button::with_label(&lines.cancel);
    cancel.add_css_class("flat");
    column.append(&cancel);

    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&column));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(480);

    // Send is there while the words aren't empty and nothing is on its way.
    let words_of = |say: &gtk::TextView| {
        let b = say.buffer();
        b.text(&b.start_iter(), &b.end_iter(), false)
            .trim()
            .to_string()
    };
    let refresh: Rc<dyn Fn()> = {
        let (send, say, state, lines) = (send.clone(), say.clone(), k.state.clone(), lines.clone());
        Rc::new(move || {
            let busy = state.borrow().busy;
            send.set_label(if busy { &lines.sending } else { &lines.send });
            send.set_sensitive(!busy && !words_of(&say).is_empty());
        })
    };
    refresh();
    {
        let refresh = refresh.clone();
        say.buffer().connect_changed(move |_| refresh());
    }
    {
        let (k, knock, say, dialog, why, refresh) = (
            k.clone(),
            knock.clone(),
            say.clone(),
            dialog.clone(),
            why.clone(),
            refresh.clone(),
        );
        send.connect_clicked(move |_| {
            let (state, dialog, why, again) = (
                k.state.clone(),
                dialog.clone(),
                why.clone(),
                refresh.clone(),
            );
            // Why it wasn't sent, here where the owner is, not on the panel behind the sheet.
            let done: Rc<dyn Fn()> = Rc::new(move || {
                again();
                match state.borrow().error.clone() {
                    None => {
                        dialog.close();
                    }
                    Some(e) => {
                        why.set_label(&e);
                        why.set_visible(true);
                    }
                }
            });
            k.answer(&knock, "do", Some(words_of(&say)), done);
            refresh();
        });
    }
    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog.close();
        });
    }
    dialog.present(Some(parent));
}

/// The panel for the taf `id` (tafs cloud's), `taf` its name for Do it; nothing until its doors
/// are read, and nothing when no message waits.
pub fn panel_for(
    w: Rc<WordBook>,
    id: String,
    taf: Option<String>,
    source: DoorsSource,
) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 14);
    if matches!(source, DoorsSource::None) {
        return root;
    }
    let k = Knocks {
        w,
        id,
        taf,
        source,
        state: Rc::new(RefCell::new(State::default())),
        redraw: Rc::new(RefCell::new(None)),
    };
    let draw: Rc<dyn Fn()> = {
        let (k, root) = (k.clone(), root.clone());
        Rc::new(move || {
            while let Some(c) = root.first_child() {
                root.remove(&c);
            }
            let s = k.state.borrow();
            let Some(d) = &s.doors else {
                return;
            };
            let (waiting, paired) = doors::panel(d);
            if !waiting.is_empty() {
                let p = panel(false);
                p.add_css_class("knocks-edge");
                p.append(&eyebrow(
                    &k.w.line("door.knocks_waiting").unwrap_or_default(),
                    true,
                ));
                for knock in &waiting {
                    p.append(&knock_card(&k, knock, s.busy, &root));
                }
                root.append(&p);
            }
            if !paired.is_empty() {
                let p = panel(false);
                p.append(&eyebrow(
                    &k.w.line("door.paired").unwrap_or_default(),
                    false,
                ));
                for knock in &paired {
                    p.append(&knock_card(&k, knock, s.busy, &root));
                }
                root.append(&p);
            }
            if !waiting.is_empty() || !paired.is_empty() {
                if let Some(e) = &s.error {
                    root.append(&text(e, 13, 400, &["tafs-no"]));
                }
            }
            root.set_visible(root.first_child().is_some());
        })
    };
    *k.redraw.borrow_mut() = Some(draw);
    root.set_visible(false);
    k.load();
    root
}
