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
    /// This person manages the taf (`manages`, KIFF's access read).
    pub manages: bool,
    /// Its Cards whole, for the Card tab, and who issued them (`issuer(of:)`).
    pub cards: Vec<tafs_core::cards::FullCard>,
    pub issued_by: String,
    /// Opens on the Card tab (`-screen card`).
    pub opens_on_card: bool,
    /// tafs cloud's id for it, and where its Activity is read from.
    pub cloud_id: Option<String>,
    pub source: ActivitySource,
    /// Where its work is read (`WorkPanel`), and its health as Home read it.
    pub work: crate::work_panel::WorkSource,
    pub health: Option<tafs_core::home::Health>,
    /// Where its runs are read and Run now sent, and `RunsPanel`'s line (`runs::next_line`).
    pub runs: crate::runs_panel::RunsSource,
    pub runs_line: Option<(String, bool)>,
}

/// Where a tafs cloud taf's Activity pages come from.
#[derive(Clone)]
pub enum ActivitySource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A contract file (`--home`'s screenshots): `taf-activity.json` beside it.
    File(std::path::PathBuf),
    None,
}

pub fn page(
    w: Rc<WordBook>,
    t: &TafPage,
    entries: Rc<Vec<Entry>>,
    halt_row: Option<gtk::Button>,
    open_usage: Rc<dyn Fn()>,
) -> gtk::Widget {
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

    // Chat, Activity, Card, Setup, as the system's segmented control. Activity and Card are the
    // tabs ported so far: Chat and Setup are there, dimmed, until theirs are.
    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tabs.add_css_class("tafs-segmented");
    tabs.set_homogeneous(true);
    let mut first: Option<gtk::ToggleButton> = None;
    let mut buttons = Vec::new();
    for (key, enabled) in [
        ("taf.section.chat", false),
        ("taf.section.activity", true),
        ("taf.section.card", true),
        ("taf.section.setup", false),
    ] {
        let b = gtk::ToggleButton::with_label(&w.line(key).unwrap_or_default());
        b.add_css_class("segment");
        if let Some(f) = &first {
            b.set_group(Some(f));
        } else {
            first = Some(b.clone());
        }
        b.set_sensitive(enabled);
        tabs.append(&b);
        buttons.push(b);
    }
    let (activity_tab, card_tab) = (buttons[1].clone(), buttons[2].clone());
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
            // It opens the Card tab, and isn't there on it.
            let bar = card_bar_widget(&lines, t.color);
            let click = gtk::GestureClick::new();
            let opens = card_tab.clone();
            click.connect_released(move |_, _, _, _| opens.set_active(true));
            bar.add_controller(click);
            bar.set_cursor_from_name(Some("pointer"));
            outer.append(&bar);
            let bar2 = bar.clone();
            card_tab.connect_toggled(move |b| bar2.set_visible(!b.is_active()));
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
    if !t.sample {
        if let Some(id) = &t.cloud_id {
            // `TafActivityList.parts`: its held calls, its work, then the list.
            body.append(&crate::work_panel::panel_for(
                w.clone(),
                id.clone(),
                t.work.clone(),
                t.health.clone(),
                Rc::new(crate::work_panel::Opens { usage: open_usage }),
            ));
            // Its runs: when it runs next and Run now, then the runs tafs cloud kept, read again
            // once Run now started one.
            let (kept, reload) =
                crate::runs_panel::retained_runs(w.clone(), id.clone(), t.runs.clone());
            if let Some(line) = &t.runs_line {
                if !matches!(t.runs, crate::runs_panel::RunsSource::None) {
                    body.append(&crate::runs_panel::runs_panel(
                        w.clone(),
                        id.clone(),
                        t.runs.clone(),
                        line.clone(),
                        reload,
                    ));
                }
            }
            body.append(&kept);
            body.append(&cloud_activity(w.clone(), id.clone(), t.source.clone()));
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
    let card_scroller = gtk::ScrolledWindow::new();
    card_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    let card = card_pane(&w, t);
    // `HaltRow(tafID:framed:)`, at the Card's foot as on the iPad.
    if let Some(row) = halt_row {
        row.set_margin_start(16);
        row.set_margin_end(16);
        card.append(&row);
    }
    card_scroller.set_child(Some(&card));
    card_scroller.set_vexpand(true);
    let panes = gtk::Stack::new();
    panes.add_named(&scroller, Some("activity"));
    panes.add_named(&card_scroller, Some("card"));
    outer.append(&panes);
    {
        let panes = panes.clone();
        activity_tab.connect_toggled(move |b| {
            if b.is_active() {
                panes.set_visible_child_name("activity");
            }
        });
    }
    {
        let panes = panes.clone();
        card_tab.connect_toggled(move |b| {
            if b.is_active() {
                panes.set_visible_child_name("card");
            }
        });
    }
    if t.opens_on_card {
        card_tab.set_active(true);
    } else {
        activity_tab.set_active(true);
    }
    crate::widgets::ground(&outer);
    outer.upcast()
}

/// The Card tab (`card(_:)`): the Cards as a wallet stack, the front Card's lines, and Revoke.
fn card_pane(w: &Rc<WordBook>, t: &TafPage) -> gtk::Box {
    let pane = gtk::Box::new(gtk::Orientation::Vertical, 16);
    pane.set_margin_top(12);
    pane.set_margin_bottom(12);
    let front: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let stack_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    stack_box.set_halign(gtk::Align::Center);
    let lines_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    pane.append(&stack_box);
    pane.append(&lines_box);
    let cards = Rc::new(t.cards.clone());
    let (color, issued_by, revoked) = (t.color, t.issued_by.clone(), t.revoked);
    let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
    *draw.borrow_mut() = Some(Rc::new({
        let (w, stack_box, lines_box, cards, front, draw) = (
            w.clone(),
            stack_box.clone(),
            lines_box.clone(),
            cards.clone(),
            front.clone(),
            draw.clone(),
        );
        move || {
            while let Some(c) = stack_box.first_child() {
                stack_box.remove(&c);
            }
            while let Some(c) = lines_box.first_child() {
                lines_box.remove(&c);
            }
            let order = tafs_core::cards::stack(&cards, front.borrow().as_deref());
            let overlay = gtk::Overlay::new();
            let n = order.len() as i32;
            let base = gtk::Box::new(gtk::Orientation::Vertical, 0);
            base.set_size_request(FACE_WIDTH, FACE_HEIGHT + (n - 1).max(0) * EDGE);
            overlay.set_child(Some(&base));
            for (i, (index, depth)) in order.iter().enumerate() {
                let card = &cards[*index];
                let face = face(&w, card, color, &issued_by, revoked, *depth);
                face.set_margin_top(i as i32 * EDGE);
                face.set_valign(gtk::Align::Start);
                // Tapping a Card behind brings it forward.
                let click = gtk::GestureClick::new();
                let (front, draw, id) = (front.clone(), draw.clone(), card.id.clone());
                click.connect_released(move |_, _, _, _| {
                    *front.borrow_mut() = Some(id.clone());
                    let f = draw.borrow().clone();
                    if let Some(f) = f {
                        f();
                    }
                });
                face.add_controller(click);
                overlay.add_overlay(&face);
            }
            stack_box.append(&overlay);
            // All of the front Card's limits: the face shows the main one.
            let shown = order.last().map(|(i, _)| &cards[*i]);
            if let Some(card) = shown {
                if !card.rules.is_empty() {
                    let panel = crate::widgets::panel(false);
                    for rule in &card.rules {
                        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                        let left = gtk::Box::new(gtk::Orientation::Vertical, 2);
                        left.append(&text(&rule.label, 15, 600, &["tafs-ink"]));
                        left.append(&text(
                            &rule.outside(&w).unwrap_or_default(),
                            12,
                            400,
                            &["tafs-dim"],
                        ));
                        row.append(&left);
                        let summary = text(
                            &rule.summary(&w).unwrap_or_default(),
                            13,
                            400,
                            &["tafs-mono", if rule.never { "tafs-no" } else { "tafs-ink" }],
                        );
                        summary.set_hexpand(true);
                        summary.set_xalign(1.0);
                        summary.set_justify(gtk::Justification::Right);
                        row.append(&summary);
                        panel.append(&row);
                    }
                    lines_box.append(&panel);
                }
            }
        }
    }));
    let f = draw.borrow().clone();
    if let Some(f) = f {
        f();
    }
    // Revoke, for the sample or whoever manages the taf (`manages`, from KIFF's access read):
    // there, dimmed, until revoking is ported with approvals.
    if t.state != TafState::Revoked && (t.sample || t.manages) {
        let revoke = crate::widgets::outline(&w.line("taf.revoke").unwrap_or_default());
        revoke.add_css_class("refuse");
        revoke.set_sensitive(false);
        pane.append(&revoke);
    }
    pane
}

/// A Card's face's size: a payment card's shape, at most 300 wide.
const FACE_WIDTH: i32 = 300;
const FACE_HEIGHT: i32 = 189;
/// How much of a Card behind shows: `WalletLayout` places each 26 below the one before (the
/// iPad's `CardStack.edge`, 40, isn't what it draws).
const EDGE: i32 = 26;

/// `TafCardFace`: its title and environment, its figure and words, its facts, who issued it and
/// until when, white on the taf's colour (the dim colour when it isn't live).
fn face(
    w: &WordBook,
    card: &tafs_core::cards::FullCard,
    color: i64,
    issued_by: &str,
    revoked: bool,
    depth: usize,
) -> gtk::Overlay {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    v.add_css_class("tafs-face");
    v.add_css_class(&if card.live {
        crate::style::avatar_class(color)
    } else {
        "dimfill".to_string()
    });
    if revoked || !card.active {
        v.add_css_class("faded");
    }
    v.set_size_request(FACE_WIDTH, FACE_HEIGHT);
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = one_line(&card.title, 12, 600, &["tafs-mono"]);
    title.set_hexpand(true);
    top.append(&title);
    if let Some(env) = card.environment_label() {
        let pill = gtk::Label::new(Some(&env));
        pill.add_css_class("tafs-env-pill");
        top.append(&pill);
    }
    v.append(&top);
    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    v.append(&spacer);
    if let Ok(Some((figure, words))) = card.totals.hero(w) {
        let h = gtk::Box::new(gtk::Orientation::Vertical, 2);
        h.append(&one_line(&figure, 34, 700, &[]));
        h.append(&one_line(&words, 12, 500, &["tafs-face-words"]));
        v.append(&h);
    }
    for fact in card.facts(w).unwrap_or_default() {
        // One line, shrunk to 80% before it's cut (`minimumScaleFactor(0.8)`): a monospaced
        // character is 0.6 of its size, and the face has 268 across inside its padding.
        let fits = (f64::from(FACE_WIDTH - 32) / (0.6 * 11.0)) as usize;
        let size = if fact.chars().count() > fits { 9 } else { 11 };
        v.append(&one_line(
            &fact,
            size,
            400,
            &["tafs-mono", "tafs-face-words"],
        ));
    }
    if card.rules.is_empty() {
        v.append(&one_line(
            &w.line("card.no_actions_short").unwrap_or_default(),
            12,
            400,
            &["tafs-mono"],
        ));
    }
    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    v.append(&spacer);
    let foot = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let by = gtk::Box::new(gtk::Orientation::Vertical, 1);
    by.append(&one_line(
        &w.line("card.issued_by").unwrap_or_default(),
        10,
        400,
        &["tafs-face-caption"],
    ));
    by.append(&one_line(issued_by, 12, 600, &[]));
    by.set_hexpand(true);
    foot.append(&by);
    let (caption, date) = card.ends(w, revoked).unwrap_or_default();
    let ends = gtk::Box::new(gtk::Orientation::Vertical, 1);
    let c = one_line(&caption, 10, 400, &["tafs-face-caption"]);
    c.set_xalign(1.0);
    ends.append(&c);
    let d = one_line(&date, 12, 600, &[]);
    d.set_xalign(1.0);
    ends.append(&d);
    foot.append(&ends);
    v.append(&foot);
    // The ring at its top corner, and each Card behind a shade darker.
    let o = gtk::Overlay::new();
    o.set_child(Some(&v));
    let ring = gtk::DrawingArea::new();
    ring.set_can_target(false);
    ring.set_draw_func(|_, cr, width, _| {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
        cr.set_line_width(1.0);
        cr.arc(
            width as f64 - 65.0 + 35.0,
            65.0 - 45.0,
            65.0,
            0.0,
            std::f64::consts::TAU,
        );
        let _ = cr.stroke();
    });
    o.add_overlay(&ring);
    let shade = tafs_core::cards::shade(depth);
    if shade > 0.0 {
        let dim = gtk::Box::new(gtk::Orientation::Vertical, 0);
        dim.set_can_target(false);
        dim.add_css_class("tafs-face-shade");
        dim.set_opacity(shade);
        o.add_overlay(&dim);
    }
    o.set_overflow(gtk::Overflow::Hidden);
    o.add_css_class("tafs-face-clip");
    o.set_size_request(FACE_WIDTH, FACE_HEIGHT);
    o.set_halign(gtk::Align::Center);
    let spoken = w
        .get("taf.show_card", &[("title", &card.title)])
        .unwrap_or_else(|_| card.title.clone());
    o.update_property(&[gtk::accessible::Property::Label(&spoken)]);
    o
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
    // `zonedShortTime`: with the account zone's name when it isn't this computer's.
    let time = gtk::Label::new(Some(&tafs_core::account_time::zoned_short_time(&e.at)));
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

/// `TafActivityList`'s list: tafs cloud's filters, its day groups, each call's line, and its foot,
/// a page at a time.
fn cloud_activity(w: Rc<WordBook>, id: String, source: ActivitySource) -> gtk::Box {
    use tafs_core::activity::{foot, path, ActivityPage, Foot};
    struct State {
        filter: String,
        pages: Vec<ActivityPage>,
        error: Option<String>,
        loading: bool,
    }
    let root = gtk::Box::new(gtk::Orientation::Vertical, 14);
    let state = Rc::new(RefCell::new(State {
        filter: "all".into(),
        pages: vec![],
        error: None,
        loading: false,
    }));
    let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
    let redraw: Redraw = {
        let draw = draw.clone();
        Rc::new(move || {
            let f = draw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        })
    };
    // Reads a page: the first again (a new filter, Try again), or the next.
    let load: Rc<dyn Fn(bool)> = {
        let (w, state, redraw, id, source) = (
            w.clone(),
            state.clone(),
            redraw.clone(),
            id.clone(),
            source.clone(),
        );
        Rc::new(move |more: bool| {
            let (filter, cursor) = {
                let mut s = state.borrow_mut();
                if s.loading {
                    return;
                }
                s.loading = true;
                s.error = None;
                (
                    s.filter.clone(),
                    if more {
                        s.pages.last().and_then(|p| p.next_cursor.clone())
                    } else {
                        None
                    },
                )
            };
            let p = path(&id, &filter, cursor.as_deref());
            let source = source.clone();
            let words: WordBook = (*w).clone();
            let (state, redraw) = (state.clone(), redraw.clone());
            gtk::glib::spawn_future_local(async move {
                let read = gtk::gio::spawn_blocking(move || -> Result<ActivityPage, String> {
                    let body = match &source {
                        ActivitySource::Cloud { token, languages } => {
                            let a = crate::kiff::taf_activity(token, languages, &p)
                                .map_err(|e| e.words(&words).unwrap_or_default())?;
                            if a.status != 200 {
                                return Err(tafs_core::signin::refusal(&a, Some(&words))
                                    .words(&words)
                                    .unwrap_or_default());
                            }
                            a.body
                        }
                        ActivitySource::File(file) => {
                            let v: serde_json::Value = serde_json::from_slice(
                                &std::fs::read(file).map_err(|e| e.to_string())?,
                            )
                            .map_err(|e| e.to_string())?;
                            serde_json::to_vec(&v["answer"]).map_err(|e| e.to_string())?
                        }
                        ActivitySource::None => return Err(String::new()),
                    };
                    ActivityPage::read(&body).ok_or_else(|| {
                        tafs_core::signin::plain_words(&words, 200).unwrap_or_default()
                    })
                })
                .await
                .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    s.loading = false;
                    match read {
                        Ok(page) => {
                            if !more {
                                s.pages.clear();
                            }
                            s.pages.push(page);
                        }
                        Err(e) => s.error = Some(e),
                    }
                }
                redraw();
            });
        })
    };
    *draw.borrow_mut() = Some(Rc::new({
        let (w, root, state, redraw, load) = (
            w.clone(),
            root.clone(),
            state.clone(),
            redraw.clone(),
            load.clone(),
        );
        move || {
            while let Some(c) = root.first_child() {
                root.remove(&c);
            }
            let s = state.borrow();
            // tafs cloud's filters, from its first page.
            let filters = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            for choice in s
                .pages
                .first()
                .map(|p| p.filters.clone())
                .unwrap_or_default()
            {
                let b = gtk::Button::with_label(&choice.title);
                b.add_css_class("tafs-filter-bordered");
                if choice.id == s.filter {
                    b.add_css_class("selected");
                }
                let (state, load, redraw) = (state.clone(), load.clone(), redraw.clone());
                b.connect_clicked(move |_| {
                    state.borrow_mut().filter = choice.id.clone();
                    redraw();
                    load(false);
                });
                filters.append(&b);
            }
            root.append(&filters);
            for page in &s.pages {
                for group in &page.groups {
                    if !group.title.is_empty() {
                        root.append(&text(&group.title, 12, 600, &["tafs-dim"]));
                    }
                    for e in &group.entries {
                        root.append(&source_row(e));
                    }
                }
            }
            if let Some(notice) = s
                .pages
                .last()
                .and_then(|p| p.card_unit_exceptions_notice.clone())
            {
                root.append(&text(&notice, 13, 400, &["tafs-dim"]));
            }
            match foot(&s.pages, s.error.as_deref()) {
                Foot::Error(e) => {
                    root.append(&text(&e, 15, 400, &["tafs-no"]));
                    let b =
                        crate::widgets::outline(&w.line("common.try_again").unwrap_or_default());
                    let (load, more) = (load.clone(), !s.pages.is_empty());
                    b.connect_clicked(move |_| load(more));
                    root.append(&b);
                }
                Foot::More(_) => {
                    // The next page loads once the spinner comes into view.
                    let spinner = gtk::Spinner::new();
                    spinner.start();
                    let load = load.clone();
                    spinner.connect_map(move |_| load(true));
                    root.append(&spinner);
                }
                _ if s.loading => {
                    let spinner = gtk::Spinner::new();
                    spinner.start();
                    spinner.set_halign(gtk::Align::Start);
                    root.append(&spinner);
                }
                Foot::End(words) => root.append(&text(&words, 15, 400, &["tafs-dim"])),
                Foot::NothingHere => root.append(&text(
                    &w.line("record.nothing_here").unwrap_or_default(),
                    15,
                    400,
                    &["tafs-dim"],
                )),
                Foot::Nothing => {}
            }
        }
    }));
    redraw();
    load(false);
    root
}

/// `SourceActivityRow`: its title and detail, its time and amount at its end, and its next step.
pub fn source_row(e: &tafs_core::activity::ActivityEntry) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_margin_top(5);
    row.set_margin_bottom(5);
    let left = gtk::Box::new(gtk::Orientation::Vertical, 4);
    left.set_hexpand(true);
    left.append(&text(&e.title, 15, 600, &["tafs-ink"]));
    left.append(&text(&e.detail, 13, 400, &["tafs-soft"]));
    row.append(&left);
    let right = gtk::Box::new(gtk::Orientation::Vertical, 2);
    right.set_valign(gtk::Align::Start);
    let time = text(&e.time, 12, 400, &["tafs-dim"]);
    time.set_wrap(false);
    time.set_xalign(1.0);
    right.append(&time);
    if let Some(amount) = &e.amount_text {
        let a = text(amount, 13, 600, &["tafs-ink"]);
        a.set_wrap(false);
        a.set_xalign(1.0);
        right.append(&a);
    }
    row.append(&right);
    v.append(&row);
    // Its next step, in tafs cloud's words: a line until its buttons come with Coach and Connect apps.
    if let Some(next) = &e.next {
        v.append(&text(&next.words, 13, 400, &["tafs-soft"]));
    }
    v.update_property(&[gtk::accessible::Property::Label(&format!(
        "{}, {}, {}",
        e.title, e.detail, e.time
    ))]);
    v
}
