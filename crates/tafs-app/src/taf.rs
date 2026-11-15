//! A taf's page beside the sidebar (`TafDetailView`): its head, its tabs, its Card bar, and its
//! Activity (the sample's, `activity(_:)`; a tafs cloud taf's own list comes with its read).

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::card::Card;
use tafs_core::needs_you::CardLines;
use tafs_core::record::{Entry, Tone};
use tafs_core::taf_page::{
    activity, card_bar, shows_chip, ActivityFilter, CardMeta, ChipTone, TafState,
};
use tafs_core::words::{upper, WordBook};

use crate::widgets::{avatar, eyebrow, one_line, symbol, text};

/// Draws the filters and the days again, after a choice.
type Redraw = Rc<dyn Fn()>;

/// What a taf's page draws.
pub struct TafPage {
    pub key: String,
    pub name: String,
    pub initial: String,
    pub color: i64,
    pub state: TafState,
    pub attention: bool,
    pub card: Option<Card>,
    pub card_title: String,
    pub card_environment: String,
    pub card_live: bool,
    pub card_active: bool,
    pub card_count: usize,
    pub revoked: bool,
    /// Its held calls' cards, from Needs you.
    pub held: Vec<CardLines>,
    /// The sample draws its Activity from the Record; a tafs cloud taf's is its own read.
    pub sample: bool,
}

pub fn page(w: Rc<WordBook>, t: &TafPage, entries: Rc<Vec<Entry>>) -> gtk::Widget {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 12);
    outer.set_margin_start(16);
    outer.set_margin_end(16);
    outer.set_margin_top(12);

    // One row: avatar, name and state.
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    head.append(&avatar(&t.initial, t.color, 34));
    let name = one_line(&t.name, 20, 700, &["tafs-ink"]);
    name.set_hexpand(true);
    head.append(&name);
    if shows_chip(t.state, t.attention) {
        let chip = gtk::Label::new(Some(&t.state.label(&w).unwrap_or_default()));
        chip.add_css_class("tafs-state-chip");
        chip.add_css_class(match t.state.tone() {
            ChipTone::Ok => "ok",
            ChipTone::Amber => "amber",
            ChipTone::No => "no",
            ChipTone::Soft => "soft",
        });
        chip.set_valign(gtk::Align::Center);
        head.append(&chip);
    }
    outer.append(&head);

    // Chat, Activity, Card, Setup, as the system's segmented control. Activity is the tab ported
    // so far: the others are there, dimmed, until theirs are.
    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tabs.add_css_class("tafs-segmented");
    tabs.set_homogeneous(true);
    let mut first: Option<gtk::ToggleButton> = None;
    for (key, enabled) in [
        ("taf.section.chat", false),
        ("taf.section.activity", true),
        ("taf.section.card", false),
        ("taf.section.setup", false),
    ] {
        let b = gtk::ToggleButton::with_label(&w.line(key).unwrap_or_default());
        b.add_css_class("segment");
        if let Some(f) = &first {
            b.set_group(Some(f));
        } else {
            first = Some(b.clone());
        }
        b.set_active(enabled);
        b.set_sensitive(enabled);
        tabs.append(&b);
    }
    outer.append(&tabs);

    // The Card in front, one line above the activity.
    if let Some(card) = &t.card {
        let meta = CardMeta {
            title: &t.card_title,
            environment: &t.card_environment,
            live: t.card_live,
            active: t.card_active,
        };
        if let Ok(lines) = card_bar(&w, card, &meta, t.card_count, t.revoked) {
            outer.append(&card_bar_widget(&lines, t.color));
        }
    }

    let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
    body.set_margin_top(4);
    body.set_margin_bottom(12);
    if !t.held.is_empty() {
        body.append(&eyebrow(
            &w.line("needs_you.title").unwrap_or_default(),
            true,
        ));
        for card in &t.held {
            body.append(&crate::needs_you::held_card(&w, card));
        }
    }
    if t.sample {
        let filters = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let days_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        body.append(&filters);
        body.append(&days_box);
        let chosen = Rc::new(RefCell::new(ActivityFilter::All));
        let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
        let redraw: Rc<dyn Fn()> = {
            let draw = draw.clone();
            Rc::new(move || {
                let f = draw.borrow().clone();
                if let Some(f) = f {
                    f();
                }
            })
        };
        let key = t.key.clone();
        *draw.borrow_mut() = Some(Rc::new({
            let (w, filters, days_box, chosen, redraw) = (
                w.clone(),
                filters.clone(),
                days_box.clone(),
                chosen.clone(),
                redraw.clone(),
            );
            move || {
                while let Some(c) = filters.first_child() {
                    filters.remove(&c);
                }
                while let Some(c) = days_box.first_child() {
                    days_box.remove(&c);
                }
                let now_filter = *chosen.borrow();
                for f in ActivityFilter::ALL {
                    let b = gtk::Button::with_label(&f.title(&w).unwrap_or_default());
                    b.add_css_class("tafs-chip-button");
                    if f == now_filter {
                        b.add_css_class("selected");
                    }
                    let (chosen, redraw) = (chosen.clone(), redraw.clone());
                    b.connect_clicked(move |_| {
                        *chosen.borrow_mut() = f;
                        redraw();
                    });
                    filters.append(&b);
                }
                let days = activity(&w, &entries, &key, now_filter, chrono::Local::now())
                    .unwrap_or_default();
                if days.is_empty() {
                    let words = w
                        .line(if now_filter == ActivityFilter::All {
                            "common.nothing_yet"
                        } else {
                            "record.nothing_here"
                        })
                        .unwrap_or_default();
                    days_box.append(&text(&words, 15, 400, &["tafs-dim"]));
                }
                for day in days {
                    let h = gtk::Label::new(Some(&upper(&day.title)));
                    h.set_xalign(0.0);
                    h.add_css_class("tafs-day");
                    h.set_margin_top(6);
                    days_box.append(&h);
                    for e in day.lines {
                        days_box.append(&line(&w, e));
                    }
                }
            }
        }));
        redraw();
    }
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&body));
    scroller.set_vexpand(true);
    outer.append(&scroller);
    crate::widgets::ground(&outer);
    outer.upcast()
}

/// `CardBar`: a small Card in the taf's colour with the Card's mark, its figure and words, its
/// name and how many, and a chevron.
fn card_bar_widget(lines: &tafs_core::taf_page::CardBarLines, color: i64) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    b.add_css_class("tafs-card-bar");
    let face = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    face.add_css_class("tafs-card-mini");
    face.add_css_class(&if lines.live {
        crate::style::avatar_class(color)
    } else {
        "dimfill".to_string()
    });
    if lines.dim {
        face.add_css_class("faded");
    }
    face.set_size_request(46, 29);
    face.set_valign(gtk::Align::Center);
    // Its own size, never stretched by the mark inside it.
    face.set_hexpand(false);
    let mark = symbol("tafs-creditcard-symbolic", 13);
    mark.set_size_request(46, 29);
    mark.set_halign(gtk::Align::Center);
    face.append(&mark);
    b.append(&face);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 1);
    v.set_hexpand(true);
    v.set_valign(gtk::Align::Center);
    let l = text(&lines.line, 15, 600, &["tafs-ink"]);
    l.set_lines(2);
    v.append(&l);
    let d = text(&lines.details, 12, 400, &["tafs-dim"]);
    d.set_lines(2);
    v.append(&d);
    b.append(&v);
    let chevron = symbol("go-next-symbolic", 13);
    chevron.add_css_class("tafs-dim");
    b.append(&chevron);
    b
}

/// `RecordRow` without its taf: the kind's dot, the title with its amount, the time, the line.
fn line(w: &WordBook, e: &Entry) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    r.add_css_class("tafs-record-row");
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.add_css_class("tafs-kind-dot");
    dot.add_css_class(match e.kind.tone() {
        Tone::Ok => "ok",
        Tone::Amber => "amber",
        Tone::No => "no",
        Tone::Dim => "dim",
    });
    dot.set_size_request(9, 9);
    dot.set_valign(gtk::Align::Start);
    dot.set_margin_top(6);
    r.append(&dot);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
    v.set_hexpand(true);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = e.title_with_amount(w).unwrap_or_else(|_| e.title.clone());
    let t = one_line(&title, 14, 600, &["tafs-ink"]);
    t.set_hexpand(true);
    head.append(&t);
    let time = gtk::Label::new(Some(
        &e.at
            .with_timezone(&chrono::Local)
            .format("%H:%M")
            .to_string(),
    ));
    time.add_css_class("tafs-mono");
    time.add_css_class("tafs-dim");
    time.add_css_class("tafs-time");
    head.append(&time);
    v.append(&head);
    // The sample's held lines still wait.
    let detail = e.row_detail(w, true).unwrap_or_else(|_| e.detail.clone());
    v.append(&text(&detail, 13, 400, &["tafs-soft"]));
    r.append(&v);
    r.update_property(&[gtk::accessible::Property::Label(&format!(
        "{title}, {detail}"
    ))]);
    r
}
