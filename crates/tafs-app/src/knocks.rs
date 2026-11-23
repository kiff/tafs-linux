//! What waits for the owner at a taf's doors, on its Activity (`TafKnocksPanel`,
//! `TafPresentationViews.swift`): strangers' knocks (Do it, Pair them, Ignore), then a paired
//! sender's message kept from its knock, apart and calmly. Each knock is `KnockCard`
//! (`DoorsView.swift`), on the Activity and on the doors' own screen (`doors.rs`), and Do it opens
//! `DoItSheet`. The doors are read as the session when the Activity opens (`GET /v1/tafs/{id}/doors`),
//! tried again quietly after 1 s and 3 s (`DoorsModel`, `doors.rs`); an answer is
//! `POST /v1/tafs/{id}/doors/knocks/{knock}`, unsigned. The lines are `tafs-core/src/doors.rs`.

use std::rc::Rc;

use adw::prelude::*;
use tafs_core::doors::{self, Knock};
use tafs_core::words::WordBook;

use crate::doors::DoorsModel;
pub use crate::doors::DoorsSource;
use crate::widgets::{eyebrow, outline, panel, primary, text};

/// `KnockCard`: who wrote, what, and the owner's three answers; or the door's line about its
/// day's messages, with OK. `taf` names it in Do it; `compact` keeps a message to three lines
/// (eight otherwise).
pub(crate) fn knock_card(
    model: &DoorsModel,
    taf: Option<&str>,
    knock: &Knock,
    busy: bool,
    compact: bool,
    parent: &impl IsA<gtk::Widget>,
) -> gtk::Box {
    let w = &model.w;
    let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
    card.add_css_class("tafs-knock");
    if knock.paired {
        card.add_css_class("paired");
    }
    let Ok(lines) = doors::knock_lines(w, knock) else {
        return card;
    };
    let parent: gtk::Widget = parent.clone().upcast();
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
    if let Ok(Some(rest)) = doors::rest_line(w, knock) {
        card.append(&text(&rest, 14, 400, &["tafs-ink"]));
        let ok = outline(&w.line("common.ok").unwrap_or_default());
        ok.set_sensitive(!busy);
        let (m, knock) = (model.clone(), knock.clone());
        ok.connect_clicked(move |_| m.answer(&knock, "ignore", None, Rc::new(|| {})));
        card.append(&ok);
        return card;
    }
    card.append(&text(&lines.wrote, 14, 600, &["tafs-ink"]));
    let body = text(&lines.text, 14, 400, &["tafs-soft"]);
    body.set_selectable(true);
    body.set_lines(if compact { 3 } else { 8 });
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
        let (m, taf, knock, parent) = (
            model.clone(),
            taf.map(str::to_string),
            knock.clone(),
            parent.clone(),
        );
        b.connect_clicked(move |_| do_it_sheet(&m, taf.as_deref(), &knock, &parent));
        row.append(&b);
    }
    if let Some(label) = &lines.pair {
        let b = gtk::Button::with_label(label);
        b.add_css_class("tafs-knock-pair");
        b.set_sensitive(!busy);
        let (m, knock) = (model.clone(), knock.clone());
        b.connect_clicked(move |_| m.answer(&knock, "pair", None, Rc::new(|| {})));
        row.append(&b);
    }
    let ignore = gtk::Button::with_label(&lines.ignore);
    ignore.add_css_class("tafs-knock-ignore");
    ignore.set_sensitive(!busy);
    {
        let (m, knock) = (model.clone(), knock.clone());
        ignore.connect_clicked(move |_| m.answer(&knock, "ignore", None, Rc::new(|| {})));
    }
    row.append(&ignore);
    card.append(&row);
    card
}

/// `DoItSheet`: what the owner tells the taf to do about a knock; their words are the instruction.
fn do_it_sheet(k: &DoorsModel, taf: Option<&str>, knock: &Knock, parent: &gtk::Widget) {
    let Ok(lines) = doors::do_it_lines(&k.w, knock, taf) else {
        return;
    };
    // `onAppear`: no error from before.
    k.state.borrow_mut().error = None;
    k.changed();
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
    let model = crate::doors::shared(w, id, source);
    let draw: Rc<dyn Fn()> = {
        let (m, root) = (model.clone(), root.downgrade());
        Rc::new(move || {
            let Some(root) = root.upgrade() else { return };
            while let Some(c) = root.first_child() {
                root.remove(&c);
            }
            let s = m.state.borrow();
            let Some(d) = &s.doors else {
                return;
            };
            let (waiting, paired) = doors::panel(d);
            if !waiting.is_empty() {
                let p = panel(false);
                p.add_css_class("knocks-edge");
                p.append(&eyebrow(
                    &m.w.line("door.knocks_waiting").unwrap_or_default(),
                    true,
                ));
                for knock in &waiting {
                    p.append(&knock_card(&m, taf.as_deref(), knock, s.busy, true, &root));
                }
                root.append(&p);
            }
            if !paired.is_empty() {
                let p = panel(false);
                p.append(&eyebrow(
                    &m.w.line("door.paired").unwrap_or_default(),
                    false,
                ));
                for knock in &paired {
                    p.append(&knock_card(&m, taf.as_deref(), knock, s.busy, true, &root));
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
    let listening = model.listen(draw);
    {
        let model = model.clone();
        root.connect_destroy(move |_| model.unlisten(listening));
    }
    root.set_visible(false);
    model.load();
    root
}
