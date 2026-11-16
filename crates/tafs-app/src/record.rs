//! The Record beside the sidebar (`RecordView`): every call each taf made, by day, newest first,
//! narrowed to one taf and a filter; a day's reads folded into one line.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::record::{days, spent, Entry, Filter, Tone};
use tafs_core::words::{upper, WordBook};

use crate::widgets::{one_line, text};

/// A taf as the chips and rows name it: its key, name and colour.
#[derive(Clone)]
pub struct Taf {
    pub key: String,
    pub name: String,
    pub color: i64,
}

/// What the page reads: the Record, the tafs, and which held calls KIFF still holds.
pub struct RecordData {
    pub entries: Vec<Entry>,
    pub tafs: Vec<Taf>,
    /// `isWaiting`: whether a held line's call is still held.
    pub waiting: Rc<dyn Fn(&Entry) -> bool>,
    /// Signed in with KIFF: the search across the person's tafs.
    pub search: Option<Rc<crate::record_search::SearchSource>>,
}

/// Draws the chips and the days again, after a choice.
type Redraw = Rc<dyn Fn()>;

struct State {
    filter: Filter,
    taf: Option<String>,
    open_reads: BTreeSet<String>,
}

pub fn page(w: Rc<WordBook>, data: Rc<RecordData>) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let l = |k: &str| w.line(k).unwrap_or_default();
    let title = text(&l("record.title"), 34, 700, &["tafs-ink"]);
    title.set_width_chars(16);
    column.append(&title);
    column.append(&text(&l("record.lead"), 14, 400, &["tafs-soft"]));

    let tafs_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let tafs_scroll = gtk::ScrolledWindow::new();
    tafs_scroll.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
    tafs_scroll.set_child(Some(&tafs_row));
    let filters_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 10);
    // Signed in: the search box, and while a search is shown, it stands in for the list.
    if let Some(source) = &data.search {
        let (a, b, c) = (tafs_scroll.clone(), filters_row.clone(), list.clone());
        let showing: Rc<dyn Fn(bool)> = Rc::new(move |searching| {
            a.set_visible(!searching);
            b.set_visible(!searching);
            c.set_visible(!searching);
        });
        let ui = crate::record_search::build(w.clone(), source.clone(), showing);
        column.append(&ui.entry);
        column.append(&ui.results);
    }
    column.append(&tafs_scroll);
    column.append(&filters_row);
    column.append(&list);

    let state = Rc::new(RefCell::new(State {
        filter: Filter::All,
        taf: None,
        open_reads: BTreeSet::new(),
    }));
    let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
    let redraw = {
        let draw = draw.clone();
        move || {
            let f = draw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        }
    };
    let redraw: Rc<dyn Fn()> = Rc::new(redraw);

    *draw.borrow_mut() = Some(Rc::new({
        let (w, data, state, tafs_row, filters_row, list, redraw) = (
            w.clone(),
            data.clone(),
            state.clone(),
            tafs_row.clone(),
            filters_row.clone(),
            list.clone(),
            redraw.clone(),
        );
        move || {
            clear(&tafs_row);
            clear(&filters_row);
            clear(&list);
            let s = state.borrow();
            // "All tafs", then one chip a taf with its colour.
            let all = chip(
                &w.line("record.all_tafs").unwrap_or_default(),
                None,
                s.taf.is_none(),
            );
            {
                let (state, redraw) = (state.clone(), redraw.clone());
                all.connect_clicked(move |_| {
                    state.borrow_mut().taf = None;
                    redraw();
                });
            }
            tafs_row.append(&all);
            for t in &data.tafs {
                let c = chip(&t.name, Some(t.color), s.taf.as_deref() == Some(&t.key));
                let (state, redraw, key) = (state.clone(), redraw.clone(), t.key.clone());
                c.connect_clicked(move |_| {
                    state.borrow_mut().taf = Some(key.clone());
                    redraw();
                });
                tafs_row.append(&c);
            }
            for f in Filter::ALL {
                let c = chip(&f.title(&w).unwrap_or_default(), None, s.filter == f);
                let (state, redraw) = (state.clone(), redraw.clone());
                c.connect_clicked(move |_| {
                    state.borrow_mut().filter = f;
                    redraw();
                });
                filters_row.append(&c);
            }
            let now = chrono::Local::now();
            let shown =
                days(&w, &data.entries, s.filter, s.taf.as_deref(), now).unwrap_or_default();
            for day in &shown {
                let all: Vec<&Entry> = day.lines.iter().chain(day.reads.iter()).copied().collect();
                let heading = match spent(&w, &all).ok().flatten() {
                    Some(sum) => format!("{} · {sum}", day.title),
                    None => day.title.clone(),
                };
                let h = gtk::Label::new(Some(&upper(&heading)));
                h.set_xalign(0.0);
                h.add_css_class("tafs-day");
                h.set_margin_top(6);
                list.append(&h);
                for e in &day.lines {
                    list.append(&row(&w, &data, e));
                }
                if !day.reads.is_empty() {
                    let key = day.title.clone();
                    let open = s.open_reads.contains(&key);
                    if open {
                        for e in &day.reads {
                            list.append(&row(&w, &data, e));
                        }
                    }
                    let words = format!(
                        "· {} · {}",
                        w.count("record.reads", day.reads.len() as i64, &[])
                            .unwrap_or_default(),
                        w.line(if open { "record.hide" } else { "record.show" })
                            .unwrap_or_default()
                    );
                    let b = gtk::Button::with_label(&words);
                    b.add_css_class("tafs-reads");
                    b.set_halign(gtk::Align::Start);
                    let (state, redraw) = (state.clone(), redraw.clone());
                    b.connect_clicked(move |_| {
                        let mut s = state.borrow_mut();
                        if !s.open_reads.remove(&key) {
                            s.open_reads.insert(key.clone());
                        }
                        drop(s);
                        redraw();
                    });
                    list.append(&b);
                }
            }
            if shown.is_empty() {
                let words = w
                    .line(if s.filter == Filter::All && s.taf.is_none() {
                        "common.nothing_yet"
                    } else {
                        "record.nothing_here"
                    })
                    .unwrap_or_default();
                let e = text(&words, 14, 400, &["tafs-dim"]);
                e.set_margin_top(8);
                list.append(&e);
            }
        }
    }));
    redraw();

    column.set_valign(gtk::Align::Start);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    crate::widgets::ground(&scroller);
    scroller.upcast()
}

fn clear(b: &gtk::Box) {
    while let Some(c) = b.first_child() {
        b.remove(&c);
    }
}

/// A chip; the chosen one inverts: ink fill, the background's colour for its words.
fn chip(title: &str, dot: Option<i64>, selected: bool) -> gtk::Button {
    let b = gtk::Button::new();
    let inner = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    if let Some(color) = dot {
        let d = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        d.add_css_class("tafs-taf-dot");
        d.add_css_class(&crate::style::avatar_class(color));
        d.set_size_request(8, 8);
        d.set_valign(gtk::Align::Center);
        inner.append(&d);
    }
    inner.append(&gtk::Label::new(Some(title)));
    b.set_child(Some(&inner));
    b.add_css_class("tafs-chip-button");
    if selected {
        b.add_css_class("selected");
    }
    b.update_state(&[gtk::accessible::State::Selected(Some(selected))]);
    b
}

/// `RecordRow`: the kind's dot, "Taf · title · amount", its time, and what became of it.
fn row(w: &WordBook, data: &RecordData, e: &Entry) -> gtk::Box {
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
    let named = match data.tafs.iter().find(|t| t.key == e.taf) {
        Some(t) => {
            let d = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            d.add_css_class("tafs-taf-dot");
            d.add_css_class(&crate::style::avatar_class(t.color));
            d.set_size_request(8, 8);
            d.set_valign(gtk::Align::Center);
            head.append(&d);
            format!("{} · {title}", t.name)
        }
        None => title,
    };
    let t = one_line(&named, 14, 600, &["tafs-ink"]);
    t.set_hexpand(true);
    head.append(&t);
    // `zonedShortTime`: with the account zone's name when it isn't this computer's.
    let time = gtk::Label::new(Some(&tafs_core::account_time::zoned_short_time(&e.at)));
    time.add_css_class("tafs-mono");
    time.add_css_class("tafs-dim");
    time.add_css_class("tafs-time");
    head.append(&time);
    v.append(&head);
    let detail = e
        .row_detail(w, (data.waiting)(e))
        .unwrap_or_else(|_| e.detail.clone());
    v.append(&text(&detail, 13, 400, &["tafs-soft"]));
    r.append(&v);
    r.update_property(&[gtk::accessible::Property::Label(&format!(
        "{named}, {detail}"
    ))]);
    r
}
