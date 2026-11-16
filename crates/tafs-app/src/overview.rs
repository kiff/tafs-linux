//! The Overview beside the sidebar (`OverviewView`): the title, its drawing, NEEDS YOU's calls,
//! and one grid of the tafs, each line opening its taf.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::overview::OverviewLines;
use tafs_core::sidebar::Selection;

use crate::widgets::{avatar, eyebrow, one_line, symbol, text};

thread_local! {
    /// `--page overview-label:<id>`: the Overview opens filtered to that label, for a screenshot.
    pub static CHOSEN: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The column's widest, from the leading edge of a centred column (`frame(maxWidth: 980)`).
const MAX_WIDTH: i32 = 980;

pub fn page(o: &OverviewLines, open: Rc<dyn Fn(Selection)>) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 20);
    column.set_margin_start(24);
    column.set_margin_end(24);
    column.set_margin_top(24);
    column.set_margin_bottom(24);

    let head = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let title = text(&o.title, 30, 700, &["tafs-ink"]);
    title.set_width_chars(16);
    head.append(&title);
    head.append(&text(&o.subtitle, 14, 400, &["tafs-dim"]));
    column.append(&head);
    // Board 10, the Overview's header (`Illustration.overview`); with no taf yet, board 2
    // (`Illustration.noTafs`) in its place, above the grid's "No tafs yet".
    column.append(&if o.empty.is_some() {
        crate::illustrations::no_tafs()
    } else {
        crate::illustrations::overview()
    });

    if let Some(label) = &o.needs_you_label {
        let section = gtk::Box::new(gtk::Orientation::Vertical, 8);
        section.append(&eyebrow(label, true));
        for held in &o.held {
            let row = held_row(&held.title, Some(&held.answer_by), false);
            let (open, sel) = (open.clone(), held.selection.clone());
            row.connect_clicked(move |_| open(sel.clone()));
            row.update_property(&[gtk::accessible::Property::Label(&format!(
                "{}, {}",
                held.title, held.answer_by
            ))]);
            section.append(&row);
        }
        // What else waits: tafs cloud's count, in its words, opening Needs you.
        if let Some(rest) = &o.rest {
            let row = held_row(rest, None, true);
            let open = open.clone();
            row.connect_clicked(move |_| open(Selection::NeedsYou));
            section.append(&row);
        }
        column.append(&section);
    }

    // The labels' chips, filtering the grid under them without reordering it; the list opens on
    // All every time.
    let listing = Rc::new(Listing {
        o: o.clone(),
        open: open.clone(),
        list: gtk::Box::new(gtk::Orientation::Vertical, 20),
        shown: RefCell::new((CHOSEN.with(|c| c.borrow().clone()), false)),
    });
    column.append(&listing.list);
    listing.draw();
    column.append(&text(&o.each_line_opens, 12, 400, &["tafs-dim"]));

    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(MAX_WIDTH);
    clamp.set_tightening_threshold(MAX_WIDTH);
    clamp.set_child(Some(&column));
    clamp.set_valign(gtk::Align::Start);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&clamp));
    scroller.set_vexpand(true);
    crate::widgets::ground(&scroller);
    scroller.upcast()
}

/// The chips and the grid under them, drawn again when a chip is tapped: the label chosen, and
/// whether "+N" showed them all.
struct Listing {
    o: OverviewLines,
    open: Rc<dyn Fn(Selection)>,
    list: gtk::Box,
    shown: RefCell<(Option<String>, bool)>,
}

impl Listing {
    fn draw(self: &Rc<Self>) {
        while let Some(c) = self.list.first_child() {
            self.list.remove(&c);
        }
        let (selected, expanded) = self.shown.borrow().clone();
        if !self.o.labels.is_empty() {
            let (choose, expand) = (self.clone(), self.clone());
            self.list.append(&crate::brief::chips_row(
                &tafs_core::brief::chips(
                    &self.o.labels,
                    selected.as_deref(),
                    &self.o.labels_all,
                    expanded,
                ),
                Rc::new(move |label| {
                    choose.shown.borrow_mut().0 = label;
                    choose.draw();
                }),
                Rc::new(move || {
                    expand.shown.borrow_mut().1 = true;
                    expand.draw();
                }),
            ));
        }
        self.list
            .append(&table(&self.o, &self.open, selected.as_deref()));
    }
}

/// A held call, or what else waits: an amber dot, its words, and its time or a chevron, on the
/// raised colour with corners of 12.
fn held_row(title: &str, trailing: Option<&str>, chevron: bool) -> gtk::Button {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.add_css_class("tafs-dot");
    dot.set_size_request(8, 8);
    dot.set_valign(gtk::Align::Center);
    b.append(&dot);
    let t = one_line(title, 14, 600, &["tafs-ink"]);
    t.set_hexpand(true);
    b.append(&t);
    if let Some(time) = trailing {
        b.append(&one_line(time, 12, 400, &["tafs-soft"]));
    }
    if chevron {
        let c = symbol("go-next-symbolic", 12);
        c.add_css_class("tafs-soft");
        b.append(&c);
    }
    let button = gtk::Button::new();
    button.set_child(Some(&b));
    button.add_css_class("tafs-held-row");
    button
}

/// One grid, so each heading sits over its column: TAF, NOW, CARD, LAST RESULT, a line between
/// rows, each line opening its taf.
fn table(o: &OverviewLines, open: &Rc<dyn Fn(Selection)>, label: Option<&str>) -> gtk::Grid {
    let grid = gtk::Grid::new();
    grid.set_column_homogeneous(true);
    grid.set_column_spacing(16);
    grid.set_row_spacing(12);
    for (i, heading) in o.columns.iter().enumerate() {
        let h = text(heading, 11, 700, &["tafs-dim"]);
        h.set_wrap(false);
        grid.attach(&h, i as i32, 0, 1, 1);
    }
    let mut y = 1;
    let rows = o
        .rows
        .iter()
        .filter(|r| tafs_core::brief::listed(&o.labels, label, &r.label_ids));
    for row in rows {
        let line = gtk::Separator::new(gtk::Orientation::Horizontal);
        line.add_css_class("tafs-line");
        grid.attach(&line, 0, y, 4, 1);
        y += 1;

        let taf = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        taf.append(&avatar(&row.initial, row.color_index, 30));
        let names = gtk::Box::new(gtk::Orientation::Vertical, 2);
        names.set_valign(gtk::Align::Center);
        names.append(&one_line(&row.name, 15, 600, &["tafs-ink"]));
        if !row.used_by.is_empty() {
            names.append(&one_line(&row.used_by, 12, 400, &["tafs-dim"]));
        }
        // Its labels, as small grey chips: two, then "+N" (`TafLabels`).
        if !row.labels.is_empty() {
            let chips = gtk::Box::new(gtk::Orientation::Horizontal, 4);
            for words in &row.labels {
                chips.append(&one_line(words, 11, 500, &["tafs-taf-label"]));
            }
            names.append(&chips);
        }
        taf.append(&names);
        taf.set_valign(gtk::Align::Start);
        let now = text(&row.now, 13, 400, &["tafs-ink"]);
        let card = text(&row.card, 13, 400, &["tafs-ink"]);
        let last = text(&row.last_result, 13, 400, &["tafs-soft"]);
        last.set_lines(2);
        last.set_ellipsize(gtk::pango::EllipsizeMode::End);
        for (x, cell) in [
            taf.upcast_ref::<gtk::Widget>(),
            now.upcast_ref(),
            card.upcast_ref(),
            last.upcast_ref(),
        ]
        .into_iter()
        .enumerate()
        {
            cell.set_valign(gtk::Align::Start);
            cell.set_cursor_from_name(Some("pointer"));
            let click = gtk::GestureClick::new();
            let (open, sel) = (open.clone(), row.selection.clone());
            click.connect_released(move |_, _, _, _| open(sel.clone()));
            cell.add_controller(click);
            grid.attach(cell, x as i32, y, 1, 1);
        }
        // The whole line is read as one, and opens with the keyboard from its name.
        taf.set_focusable(true);
        taf.update_property(&[gtk::accessible::Property::Label(&row.accessible)]);
        let key = gtk::EventControllerKey::new();
        let (open2, sel2) = (open.clone(), row.selection.clone());
        key.connect_key_pressed(move |_, k, _, _| {
            if matches!(
                k,
                gtk::gdk::Key::Return | gtk::gdk::Key::space | gtk::gdk::Key::KP_Enter
            ) {
                open2(sel2.clone());
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });
        taf.add_controller(key);
        for cell in [&now, &card, &last] {
            cell.set_accessible_role(gtk::AccessibleRole::Presentation);
        }
        y += 1;
    }
    if let Some(empty) = &o.empty {
        grid.attach(&text(empty, 14, 400, &["tafs-dim"]), 0, y, 4, 1);
    }
    grid
}
