//! "Your brief" beside the sidebar (`BriefView`, Missions design v2.1 §7), in Needs you's place
//! once tafs cloud sends one: decisions first, each with Coach's recommendation and what happens
//! meanwhile, then one next move, then what got done, case by case, and the rest in one line that
//! opens Needs you. Two columns on a wide pane, decisions beside Done. Every word, count and
//! order is tafs cloud's (`tafs-core/src/brief.rs`); a recommendation is shown, never acted on.
//!
//! This computer is view-only: an answer the iPad gives with a signature (a held call on KIFF's
//! card, a step's Done or Skip it) is drawn as the iPad draws it, and tapped, `ApprovalGate`'s
//! words say why it's answered on the phone. A taf whose check keeps failing opens its page
//! ("Open the taf"). Later moves a decision to the next brief at tafs cloud; nothing is signed.

use std::cell::RefCell;
use std::collections::BTreeSet;
use std::rc::Rc;

use adw::prelude::*;
use tafs_core::brief::{self, Brief, ChipTap, Decision, Destination};
use tafs_core::sidebar::{Mode, Selection};
use tafs_core::words::WordBook;

use crate::widgets::{avatar, eyebrow, icon_for, one_line, outline, panel, primary, symbol, text};

/// Where the brief is read, filtered to a label, and where Later is sent.
#[derive(Clone)]
pub enum BriefSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answer (`--brief`): every read is it, and Later sends nothing.
    File(std::path::PathBuf),
}

/// One of the account's tafs, as a decision names it by tafs cloud's id.
#[derive(Clone)]
pub struct BriefTaf {
    pub cloud_id: String,
    pub key: String,
    pub name: String,
    pub initial: String,
    pub color: i64,
}

/// What the brief is drawn from, once read with the account.
pub struct BriefPane {
    pub brief: Brief,
    pub source: BriefSource,
    /// `mayUseCoach`: the next move's Start opens Coach with it.
    pub may_use_coach: bool,
    pub tafs: Vec<BriefTaf>,
    /// Needs you's items the brief can match by its `ref` (`hold:<id>`, `step:<handoff>`), and
    /// how many it can't carry.
    pub needs_you_items: Vec<String>,
    pub needs_you_others: i64,
}

/// The page's state (`AppModel`'s brief, `briefLabel`, `briefSetAside`, `nextMoveDismissed`).
struct State {
    /// None once a read for a label finds nothing: Needs you, as today.
    brief: Option<Brief>,
    label: Option<String>,
    set_aside: BTreeSet<String>,
    next_move_dismissed: bool,
    /// "+N" tapped: every label shown.
    expanded: bool,
    /// A narrow pane: one column.
    compact: bool,
    /// The decisions whose signed answer was tapped: `ApprovalGate`'s words under them.
    gated: BTreeSet<String>,
    /// Needs you, opened from the rest's line, over the brief.
    needs_you_open: bool,
}

struct Page {
    w: Rc<WordBook>,
    pane: Rc<BriefPane>,
    open: Rc<dyn Fn(Selection)>,
    /// Needs you's own page, for the rest's line and a brief with nothing left.
    needs_you: Rc<dyn Fn() -> gtk::Widget>,
    state: RefCell<State>,
    root: gtk::Box,
}

thread_local! {
    /// `--page brief-answer:<decision id>`: that decision's signed answer already tapped, its
    /// `ApprovalGate` words showing, for a screenshot.
    pub static ANSWERED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Below this width the pane is an iPhone's: one column.
const COMPACT_BELOW: f64 = 640.0;

pub fn page(
    w: Rc<WordBook>,
    pane: Rc<BriefPane>,
    open: Rc<dyn Fn(Selection)>,
    needs_you: Rc<dyn Fn() -> gtk::Widget>,
) -> gtk::Widget {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.set_vexpand(true);
    crate::widgets::ground(&root);
    let page = Rc::new(Page {
        state: RefCell::new(State {
            brief: Some(pane.brief.clone()),
            label: None,
            set_aside: BTreeSet::new(),
            next_move_dismissed: false,
            expanded: false,
            compact: false,
            gated: ANSWERED.with(|a| a.borrow().iter().cloned().collect()),
            needs_you_open: false,
        }),
        w,
        pane,
        open,
        needs_you,
        root: root.clone(),
    });
    page.draw();
    let bin = adw::BreakpointBin::new();
    bin.set_size_request(320, 240);
    bin.set_child(Some(&root));
    let narrow = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        COMPACT_BELOW,
        adw::LengthUnit::Px,
    ));
    for compact in [true, false] {
        let p = page.clone();
        let set = move |_: &adw::Breakpoint| {
            p.state.borrow_mut().compact = compact;
            p.draw();
        };
        if compact {
            narrow.connect_apply(set);
        } else {
            narrow.connect_unapply(set);
        }
    }
    bin.add_breakpoint(narrow);
    bin.upcast()
}

impl Page {
    fn draw(self: &Rc<Self>) {
        while let Some(c) = self.root.first_child() {
            self.root.remove(&c);
        }
        let s = self.state.borrow();
        // No brief left (a label with nothing in it): Needs you, as today.
        let Some(b) = s.brief.as_ref() else {
            self.root.append(&(self.needs_you)());
            return;
        };
        if s.needs_you_open {
            // Needs you over the brief, with the way back to it.
            let back = gtk::Button::new();
            back.set_child(Some(&symbol("go-previous-symbolic", 20)));
            back.add_css_class("tafs-round-back");
            back.set_halign(gtk::Align::Start);
            back.set_margin_start(16);
            back.set_margin_top(12);
            back.set_tooltip_text(Some(&b.words.title));
            back.update_property(&[gtk::accessible::Property::Label(&b.words.title)]);
            let p = self.clone();
            back.connect_clicked(move |_| {
                p.state.borrow_mut().needs_you_open = false;
                p.draw();
            });
            self.root.append(&back);
            self.root.append(&(self.needs_you)());
            return;
        }
        let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
        column.set_margin_start(16);
        column.set_margin_end(16);
        column.set_margin_top(12);
        column.set_margin_bottom(12);
        column.append(&self.header(b, &s));
        let decisions = gtk::Box::new(gtk::Orientation::Vertical, 14);
        self.decisions(b, &s, &decisions);
        if s.compact {
            column.append(&decisions);
            self.next_move(b, &s, &column);
            self.done(b, &column);
            self.rest(b, &s, &column);
        } else {
            let two = gtk::Box::new(gtk::Orientation::Horizontal, 20);
            two.set_homogeneous(true);
            decisions.set_valign(gtk::Align::Start);
            two.append(&decisions);
            let right = gtk::Box::new(gtk::Orientation::Vertical, 14);
            right.set_valign(gtk::Align::Start);
            self.done(b, &right);
            // Under Done, so it stays in view beside the decisions (the iPad mockup).
            self.next_move(b, &s, &right);
            self.rest(b, &s, &right);
            two.append(&right);
            column.append(&two);
        }
        if let Some(credits) = &b.credits {
            let c = text(credits, 12, 400, &["tafs-dim"]);
            c.set_xalign(0.5);
            c.set_justify(gtk::Justification::Center);
            column.append(&c);
        }
        column.set_valign(gtk::Align::Start);
        let scroller = gtk::ScrolledWindow::new();
        scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        scroller.set_child(Some(&column));
        scroller.set_vexpand(true);
        crate::widgets::ground(&scroller);
        self.root.append(&scroller);
    }

    /// The title, when it was written, and the label chips filtering it.
    fn header(self: &Rc<Self>, b: &Brief, s: &State) -> gtk::Box {
        let head = gtk::Box::new(gtk::Orientation::Vertical, 8);
        let title = text(&b.words.title, 34, 700, &["tafs-ink"]);
        title.set_width_chars(16);
        head.append(&title);
        if !b.words.when.is_empty() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let clock = symbol(icon_for("clock"), 13);
            clock.add_css_class("tafs-soft");
            clock.set_accessible_role(gtk::AccessibleRole::Presentation);
            row.append(&clock);
            row.append(&text(&b.words.when, 13, 400, &["tafs-soft"]));
            head.append(&row);
        }
        if !b.labels.is_empty() {
            // The brief's own word for All, else the app's (`labelsAllWords`).
            let all = brief::all_words(&self.w, None, Some(b)).unwrap_or_default();
            let p = self.clone();
            let expand = self.clone();
            head.append(&chips_row(
                &brief::chips(&b.labels, s.label.as_deref(), &all, s.expanded),
                Rc::new(move |label| p.filter(label)),
                Rc::new(move || {
                    expand.state.borrow_mut().expanded = true;
                    expand.draw();
                }),
            ));
        }
        head
    }

    /// `filterBrief`: the brief read again for the label chosen; what was set aside here comes
    /// back. A read that fails, or finds nothing, is Needs you, as on the iPad.
    fn filter(self: &Rc<Self>, label: Option<String>) {
        {
            let mut s = self.state.borrow_mut();
            s.label = label.clone();
            s.set_aside.clear();
            s.gated.clear();
        }
        let (p, source, words) = (self.clone(), self.pane.source.clone(), (*self.w).clone());
        gtk::glib::spawn_future_local(async move {
            let read = gtk::gio::spawn_blocking(move || {
                let body = match &source {
                    BriefSource::Cloud { token, languages } => {
                        crate::kiff::cloud(token, languages, &Brief::path(label.as_deref()))
                            .ok()
                            .filter(|a| a.status == 200)?
                            .body
                    }
                    // A fixture is the same answer for every label: nothing is sent.
                    BriefSource::File(path) => std::fs::read(path).ok()?,
                };
                read_answer(&body, &words)
            })
            .await
            .ok()
            .flatten();
            p.state.borrow_mut().brief = read;
            p.draw();
        });
    }

    /// The decisions still to answer here, or board 3 where they go when there's none.
    fn decisions(self: &Rc<Self>, b: &Brief, s: &State, into: &gtk::Box) {
        let open: Vec<&Decision> = b.open_decisions(&s.set_aside).collect();
        if open.is_empty() {
            // Board 3, nothing to decide (`Illustration.briefClear`): a calm sun where the
            // decisions go.
            let board = crate::illustrations::brief_clear();
            board.set_margin_top(8);
            board.set_margin_bottom(8);
            into.append(&board);
            return;
        }
        into.append(&eyebrow(&b.words.decisions, false));
        for d in open {
            into.append(&self.decision(b, d, s.gated.contains(&d.id)));
        }
        if let Some(more) = &b.more {
            into.append(&text(&more.words, 13, 400, &["tafs-soft"]));
        }
    }

    /// `DecisionCard`: its taf, the question, Coach's recommendation with its why, what happens
    /// meanwhile, why it can't wait, its answers, and its note.
    fn decision(self: &Rc<Self>, b: &Brief, d: &Decision, gated: bool) -> gtk::Box {
        let words = &b.words;
        let p = panel(false);
        if d.urgent {
            p.add_css_class("edge-no");
        }
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let taf = self.pane.tafs.iter().find(|t| t.cloud_id == d.r#ref.taf);
        if let Some(t) = taf {
            head.append(&avatar(&t.initial, t.color, 26));
            head.append(&one_line(&t.name, 14, 600, &["tafs-ink"]));
        }
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_hexpand(true);
        head.append(&spacer);
        if d.urgent {
            // tafs cloud's word for it; the bolt alone where it sent none.
            let badge = gtk::Box::new(gtk::Orientation::Horizontal, 3);
            badge.add_css_class("tafs-urgent");
            badge.set_valign(gtk::Align::Center);
            let bolt = symbol(icon_for("bolt.fill"), 10);
            bolt.set_accessible_role(gtk::AccessibleRole::Presentation);
            badge.append(&bolt);
            if !words.urgent.is_empty() {
                badge.append(&one_line(&words.urgent, 11, 600, &[]));
            }
            head.append(&badge);
        } else if let Some(deadline) = &d.deadline {
            let l = one_line(deadline, 12, 400, &["tafs-amber-text", "tafs-mono"]);
            head.append(&l);
        }
        // Nothing in it, no line: a taf the app doesn't list, and nothing urgent or due.
        if taf.is_some() || d.urgent || d.deadline.is_some() {
            p.append(&head);
        }
        p.append(&text(&d.question, 17, 600, &["tafs-ink"]));
        if let (Some(pick), Some(why)) = (d.recommended(), &d.why) {
            let r = gtk::Box::new(gtk::Orientation::Vertical, 4);
            r.add_css_class("tafs-recommends");
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            let spark = symbol(icon_for("sparkle"), 13);
            spark.add_css_class("tafs-coach-text");
            spark.set_accessible_role(gtk::AccessibleRole::Presentation);
            line.append(&spark);
            line.append(&text(
                &format!("{}: {}", words.recommends, pick.words),
                13,
                600,
                &["tafs-coach-text"],
            ));
            r.append(&line);
            r.append(&text(why, 14, 400, &["tafs-ink"]));
            p.append(&r);
        }
        if !d.meanwhile.is_empty() {
            // "Meanwhile:" in bold, then what happens.
            let m = text(
                &format!("{} {}", words.meanwhile, d.meanwhile),
                14,
                400,
                &["tafs-ink"],
            );
            if let Some(attrs) = m.attributes() {
                let mut bold = gtk::pango::AttrInt::new_weight(gtk::pango::Weight::Bold);
                bold.set_start_index(0);
                bold.set_end_index(words.meanwhile.len() as u32);
                attrs.insert(bold);
                m.set_attributes(Some(&attrs));
            }
            p.append(&m);
        }
        if let Some(why) = &d.cant_wait {
            let c = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            c.add_css_class("tafs-cant-wait");
            let clock = symbol(icon_for("clock"), 13);
            clock.add_css_class("tafs-no");
            clock.set_valign(gtk::Align::Start);
            clock.set_accessible_role(gtk::AccessibleRole::Presentation);
            c.append(&clock);
            let l = text(why, 13, 400, &["tafs-no"]);
            l.set_hexpand(true);
            c.append(&l);
            p.append(&c);
        }
        self.answers(d, &p);
        if gated {
            // `ApprovalGate`'s words: this computer can't approve, so the phone answers.
            if let Ok(tafs_core::approval::Gate::NotReady(n)) =
                tafs_core::approval::gate(&self.w, Mode::Kiff, tafs_core::approval::THIS_COMPUTER)
            {
                p.append(&crate::approval::not_ready(&n));
            }
        }
        if let Some(note) = &d.note {
            p.append(&text(note, 12, 400, &["tafs-dim"]));
        }
        p
    }

    /// The recommended answer filled, the others beside Later, a decline in red. An urgent
    /// decision can't be put off: no Later, whatever was sent.
    fn answers(self: &Rc<Self>, d: &Decision, into: &gtk::Box) {
        if let Some(pick) = d.recommended() {
            let b = primary(&pick.words);
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.set_halign(gtk::Align::Center);
            let spark = symbol(icon_for("sparkle"), 15);
            spark.set_accessible_role(gtk::AccessibleRole::Presentation);
            row.append(&spark);
            row.append(&gtk::Label::new(Some(&pick.words)));
            b.set_child(Some(&row));
            b.update_property(&[gtk::accessible::Property::Label(&pick.words)]);
            self.on_answer(&b, d);
            into.append(&b);
        }
        let others = d.others();
        let later = d.later_shown();
        if others.is_empty() && later.is_none() {
            return;
        }
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        for a in others {
            let b = outline(&a.words);
            if a.decline {
                b.add_css_class("tafs-no");
            }
            self.on_answer(&b, d);
            row.append(&b);
        }
        if let Some(later) = later {
            let b = outline(later);
            let (p, d) = (self.clone(), d.clone());
            b.connect_clicked(move |button| p.later(&d, button));
            row.append(&b);
        }
        into.append(&row);
    }

    /// An answer opens where its own answer is given (`answerBrief`): signed on the iPad, so here
    /// `ApprovalGate`'s words under the card; a failing check's taf opens its page; anything else
    /// opens Needs you.
    fn on_answer(self: &Rc<Self>, button: &gtk::Button, d: &Decision) {
        let (p, d) = (self.clone(), d.clone());
        button.connect_clicked(move |_| match Destination::of(&d) {
            dest if dest.signed() => {
                p.state.borrow_mut().gated.insert(d.id.clone());
                p.draw();
            }
            Destination::Taf(cloud) => {
                match p.pane.tafs.iter().find(|t| t.cloud_id == cloud) {
                    Some(t) => (p.open)(Selection::Taf(t.key.clone())),
                    // A taf the app doesn't list opens Needs you instead.
                    None => p.open_needs_you(),
                }
            }
            _ => p.open_needs_you(),
        });
    }

    fn open_needs_you(self: &Rc<Self>) {
        self.state.borrow_mut().needs_you_open = true;
        self.draw();
    }

    /// `laterBrief`: set aside only once tafs cloud has moved it; else it stays, to answer or try
    /// again. A fixture's sends nothing and sets it aside.
    fn later(self: &Rc<Self>, d: &Decision, button: &gtk::Button) {
        let Some(path) = brief::later_path(d) else {
            return;
        };
        let id = d.id.clone();
        match self.pane.source.clone() {
            BriefSource::File(_) => {
                self.state.borrow_mut().set_aside.insert(id);
                self.draw();
            }
            BriefSource::Cloud { token, languages } => {
                button.set_sensitive(false);
                let (p, button) = (self.clone(), button.clone());
                gtk::glib::spawn_future_local(async move {
                    let moved = gtk::gio::spawn_blocking(move || {
                        crate::kiff::cloud_send(&token, &languages, "POST", &path, Some(b"{}"))
                            .is_ok_and(|a| (200..300).contains(&a.status))
                    })
                    .await
                    .unwrap_or(false);
                    if moved {
                        p.state.borrow_mut().set_aside.insert(id);
                        p.draw();
                    } else {
                        button.set_sensitive(true);
                    }
                });
            }
        }
    }

    /// One next move: Start opens Coach with it, for whoever may use Coach; Dismiss puts it away.
    fn next_move(self: &Rc<Self>, b: &Brief, s: &State, into: &gtk::Box) {
        let Some(m) = &b.next_move else {
            return;
        };
        if s.next_move_dismissed {
            return;
        }
        let p = panel(false);
        p.add_css_class("edge-move");
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let spark = symbol(icon_for("sparkle"), 12);
        spark.add_css_class("tafs-amber-text");
        spark.set_accessible_role(gtk::AccessibleRole::Presentation);
        head.append(&spark);
        let e = eyebrow(&b.words.next_move, false);
        e.add_css_class("tafs-amber-text");
        head.append(&e);
        p.append(&head);
        p.append(&text(&m.words, 15, 600, &["tafs-ink"]));
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let start = outline(&m.start);
        start.add_css_class("amber");
        {
            let (page, words) = (self.clone(), m.words.clone());
            start.connect_clicked(move |_| {
                page.state.borrow_mut().next_move_dismissed = true;
                if page.pane.may_use_coach {
                    crate::coach::set_draft(&words);
                    (page.open)(Selection::Coach);
                } else {
                    page.draw();
                }
            });
        }
        row.append(&start);
        let dismiss = outline(&m.dismiss);
        {
            let page = self.clone();
            dismiss.connect_clicked(move |_| {
                page.state.borrow_mut().next_move_dismissed = true;
                page.draw();
            });
        }
        row.append(&dismiss);
        p.append(&row);
        into.append(&p);
    }

    /// Done, case by case: green once closed, amber while it waits, with what it's worth.
    fn done(self: &Rc<Self>, b: &Brief, into: &gtk::Box) {
        if b.done.is_empty() {
            return;
        }
        into.append(&eyebrow(&b.words.done, false));
        let p = panel(false);
        for (i, group) in b.done.iter().enumerate() {
            if i > 0 {
                let line = gtk::Separator::new(gtk::Orientation::Horizontal);
                line.add_css_class("tafs-line");
                p.append(&line);
            }
            for c in &group.cases {
                p.append(&case_row(c));
            }
            if let Some(value) = &group.value {
                p.append(&text(value, 15, 600, &["tafs-ink"]));
            }
        }
        into.append(&p);
    }

    /// The things that need nothing now, in one line, opening Needs you; with its own count
    /// whenever Needs you holds more than the brief's decisions (`briefNeedsYouLine`).
    fn rest(self: &Rc<Self>, b: &Brief, s: &State, into: &gtk::Box) {
        let open: Vec<&Decision> = b.open_decisions(&s.set_aside).collect();
        let count = brief::needs_you_line(
            &self.w,
            &self.pane.needs_you_items,
            self.pane.needs_you_others,
            &open,
        )
        .ok()
        .flatten();
        if b.rest.is_none() && count.is_none() {
            return;
        }
        let row = gtk::Button::new();
        row.add_css_class("tafs-panel-link");
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
        v.set_hexpand(true);
        if let Some(rest) = &b.rest {
            v.append(&text(&rest.words, 14, 400, &["tafs-ink"]));
        }
        if let Some(count) = &count {
            v.append(&text(
                count,
                13,
                600,
                &[if b.rest.is_none() {
                    "tafs-ink"
                } else {
                    "tafs-dim"
                }],
            ));
        }
        h.append(&v);
        let chevron = symbol("go-next-symbolic", 13);
        chevron.add_css_class("tafs-dim");
        h.append(&chevron);
        row.set_child(Some(&h));
        let p = self.clone();
        row.connect_clicked(move |_| p.open_needs_you());
        into.append(&row);
    }
}

/// tafs cloud's answer, or a fixture's (`{answer}` or the answer itself), read as the iPad reads
/// it; none when it isn't a brief or has nothing in it.
pub fn read_answer(body: &[u8], w: &WordBook) -> Option<Brief> {
    let v: serde_json::Value = serde_json::from_slice(body).ok()?;
    let answer = match v.get("answer") {
        Some(a) if v.get("about").is_some() => serde_json::to_vec(a).ok()?,
        _ => body.to_vec(),
    };
    Brief::read(&answer, w).ok().flatten()?.shown()
}

/// `CaseRow`: a dot in its state's colour, its name and what was done, as tafs cloud says it.
fn case_row(c: &brief::Case) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.add_css_class("tafs-case-dot");
    dot.add_css_class(if c.closed() {
        "ok"
    } else if c.waiting() {
        "amber"
    } else {
        "dim"
    });
    dot.set_valign(gtk::Align::Start);
    dot.set_margin_top(6);
    row.append(&dot);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    if !c.name.is_empty() {
        v.append(&text(&c.name, 14, 600, &["tafs-ink"]));
    }
    v.append(&text(
        &c.words,
        13,
        400,
        &[if c.closed() { "tafs-ok" } else { "tafs-soft" }],
    ));
    row.append(&v);
    // Read as one.
    let label = [c.name.as_str(), c.words.as_str()]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    row.set_accessible_role(gtk::AccessibleRole::Group);
    row.update_property(&[gtk::accessible::Property::Label(&label)]);
    row
}

/// `LabelChips`: All, then the labels, then "+N", in a row that scrolls sideways; a chip's tap
/// filters (`choose`) or shows them all (`expand`).
pub fn chips_row(
    chips: &[brief::Chip],
    choose: Rc<dyn Fn(Option<String>)>,
    expand: Rc<dyn Fn()>,
) -> gtk::ScrolledWindow {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.set_margin_top(2);
    row.set_margin_bottom(2);
    for c in chips {
        let b = gtk::Button::new();
        b.add_css_class("tafs-label-chip");
        if c.selected {
            b.add_css_class("selected");
        }
        let inner = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        if c.waiting {
            let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            dot.add_css_class("tafs-chip-dot");
            dot.set_valign(gtk::Align::Center);
            inner.append(&dot);
        }
        inner.append(&one_line(&c.words, 13, 500, &[]));
        if let Some(n) = c.count {
            let count = one_line(&n.to_string(), 12, 400, &[]);
            count.set_opacity(0.7);
            inner.append(&count);
        }
        b.set_child(Some(&inner));
        let label = match c.count {
            Some(n) => format!("{}, {n}", c.words),
            None => c.words.clone(),
        };
        b.update_property(&[gtk::accessible::Property::Label(&label)]);
        if c.selected {
            b.update_state(&[gtk::accessible::State::Selected(Some(true))]);
        }
        match &c.tap {
            ChipTap::Choose(label) => {
                let (choose, label) = (choose.clone(), label.clone());
                b.connect_clicked(move |_| choose(label.clone()));
            }
            ChipTap::Expand => {
                let expand = expand.clone();
                b.connect_clicked(move |_| expand());
            }
        }
        row.append(&b);
    }
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Never);
    scroller.set_child(Some(&row));
    scroller.set_propagate_natural_height(true);
    scroller
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").expect("the words at the pin")
    }

    fn fixture(name: &str) -> Vec<u8> {
        std::fs::read(format!("{}/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))).unwrap()
    }

    /// The screenshots' briefs read as tafs cloud's answer, inside `{about, answer}` or not.
    #[test]
    fn the_fixtures_read_as_tafs_clouds_answer() {
        let w = en();
        let b = read_answer(&fixture("brief.json"), &w).expect("a brief");
        let kinds: Vec<_> = b.decisions.iter().map(|d| d.r#ref.kind.as_str()).collect();
        assert_eq!(kinds, ["check", "hold", "step"]);
        assert_eq!(
            Destination::of(&b.decisions[0]),
            Destination::Taf("taf-aaaa0000000000000000".into())
        );
        assert!(b.decisions[0].urgent && b.decisions[0].later_shown().is_none());
        assert!(Destination::of(&b.decisions[1]).signed());
        assert_eq!(b.labels.len(), 3);
        let v: serde_json::Value = serde_json::from_slice(&fixture("brief.json")).unwrap();
        let bare = serde_json::to_vec(&v["answer"]).unwrap();
        assert_eq!(read_answer(&bare, &w), Some(b));
        let clear = read_answer(&fixture("brief-clear.json"), &w).expect("done, so a brief");
        assert!(clear.decisions.is_empty(), "board 3 where the decisions go");
        assert_eq!(read_answer(b"{}", &w), None, "nothing in it: Needs you");
        assert_eq!(read_answer(b"[]", &w), None);
    }
}
