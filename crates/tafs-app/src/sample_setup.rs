//! The sample's Setup tab (`TafDetailView.setup` over the sample): its job, what it remembers
//! (`MemoryPanel`, the sample's own notes, each forgotten here at once, nothing sent), and Pause or
//! Resume (`togglePause`), which the sidebar's line says at once.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use tafs_core::setup::Note;
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, outline, panel, text};

/// The notes drawn again, once there's something to draw them with.
type Later = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

thread_local! {
    /// Each sample taf's notes as forgetting left them, by key, while the app runs.
    static NOTES: RefCell<std::collections::HashMap<String, Vec<Note>>> = RefCell::default();
}

/// The Setup tab of the sample taf `key`.
pub fn pane(w: Rc<WordBook>, key: &str) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 16);
    let job = panel(false);
    job.append(&eyebrow(&l("setup.job"), false));
    job.append(&text(
        &w.line(&format!("sample.{key}.job")).unwrap_or_default(),
        15,
        400,
        &["tafs-ink"],
    ));
    root.append(&job);
    let memory = panel(false);
    root.append(&memory);
    let draw: Later = Rc::default();
    {
        let (w, key, memory, again) = (w.clone(), key.to_string(), memory.clone(), draw.clone());
        *draw.borrow_mut() = Some(Rc::new(move || {
            draw_notes(&w, &key, &memory, &again);
        }));
    }
    if let Some(d) = draw.borrow().clone() {
        d();
    }
    let label = |paused: bool| l(if paused { "taf.resume" } else { "taf.pause" });
    let toggle = outline(&label(crate::taf_chat::sample_paused_now(key)));
    toggle.set_halign(gtk::Align::Fill);
    {
        let (key, words) = (key.to_string(), (l("taf.resume"), l("taf.pause")));
        toggle.connect_clicked(move |b| {
            let paused = crate::taf_chat::toggle_sample_pause(&key);
            b.set_label(if paused { &words.0 } else { &words.1 });
        });
    }
    root.append(&toggle);
    root.upcast()
}

fn draw_notes(w: &Rc<WordBook>, key: &str, p: &gtk::Box, again: &Later) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    while let Some(c) = p.first_child() {
        p.remove(&c);
    }
    p.append(&eyebrow(&l("taf.memory.title"), false));
    let notes = NOTES.with(|n| {
        n.borrow_mut()
            .entry(key.to_string())
            .or_insert_with(|| {
                tafs_core::sample::notes(w, key, chrono::Local::now()).unwrap_or_default()
            })
            .clone()
    });
    if notes.is_empty() {
        p.append(&text(&l("taf.memory.empty"), 14, 400, &["tafs-soft"]));
    }
    for note in notes {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let v = gtk::Box::new(gtk::Orientation::Vertical, 3);
        v.set_hexpand(true);
        v.append(&text(&note.content, 15, 400, &["tafs-ink"]));
        let when = tafs_core::account_time::short_time(&note.updated_at);
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
        let (w2, key, again) = (w.clone(), key.to_string(), again.clone());
        forget.connect_clicked(move |b| ask_forget(&w2, &key, &note, &again, b.upcast_ref()));
        row.append(&forget);
        p.append(&row);
    }
    p.append(&text(&l("taf.memory.explains"), 12, 400, &["tafs-soft"]));
}

/// Forgetting can't be undone, so it's asked first (the 755 retest), as a real taf's note is.
fn ask_forget(w: &Rc<WordBook>, key: &str, note: &Note, again: &Later, over: &gtk::Widget) {
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
    let (key, id, again) = (key.to_string(), note.id.clone(), again.clone());
    ask.connect_response(None, move |_, response| {
        if response != "forget" {
            return;
        }
        NOTES.with(|n| {
            if let Some(list) = n.borrow_mut().get_mut(&key) {
                list.retain(|x| x.id != id);
            }
        });
        if let Some(d) = again.borrow().clone() {
            d();
        }
    });
    ask.present(Some(over));
}
