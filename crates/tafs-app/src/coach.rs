//! Coach beside the sidebar (`SetupChatView` at a regular width), read only: Coach's newest
//! conversation as tafs cloud reads it with this computer's proof, drawn as the iPad draws it.
//! The lines are `tafs-core/src/coach_lines.rs`. Nothing here sends: there is no message box, a
//! card's answers are drawn dimmed, and where an approval's or a sign-in's buttons would be, it
//! says this computer can't approve (`ApprovalGate`), under tafs cloud's `wider` line.

use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::coach::{self, SetupThread};
use tafs_core::coach_lines::{
    ApprovalLines, Card, CoachLines, Line, Listed, Mark, Outcome, PlanLines, PlanStep, ResultLines,
    SignInLines, StepStatus, Tone, WorkLines,
};
use tafs_core::session::KeptSession;
use tafs_core::sidebar::{Mode, Selection};
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, outline, panel, primary, symbol, text};

/// Where Coach's conversation is read from.
#[derive(Clone)]
pub enum CoachSource {
    /// A conversation as tafs cloud answers it, from a file: nothing is sent.
    File(std::path::PathBuf),
    /// tafs cloud, as the session, with this device's proof when it has a key and KIFF names
    /// this computer.
    Cloud {
        kept: Box<KeptSession>,
        languages: Vec<&'static str>,
    },
    /// The sample has no Coach: its title alone, as before.
    None,
}

/// The page, and what it reads.
pub fn page(
    w: Rc<WordBook>,
    source: CoachSource,
    listed: Rc<Vec<Listed>>,
    open: Rc<dyn Fn(Selection)>,
) -> gtk::Widget {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.add_css_class("tafs-coach-wash");
    root.set_vexpand(true);
    // The inline title, as the iPad's navigation bar has it.
    let title = text(
        &w.line("coach.title").unwrap_or_default(),
        17,
        600,
        &["tafs-ink"],
    );
    title.set_xalign(0.5);
    title.set_margin_top(14);
    title.set_margin_bottom(12);
    let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bar.add_css_class("tafs-coach-title");
    bar.append(&title);
    root.append(&bar);
    let plan_slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    root.append(&plan_slot);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    column.set_valign(gtk::Align::Start);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    root.append(&scroller);
    let error = text("", 13, 400, &["tafs-no"]);
    error.set_margin_start(16);
    error.set_margin_end(16);
    error.set_margin_bottom(8);
    error.set_visible(false);
    root.append(&error);

    let draw: Rc<dyn Fn(Result<SetupThread, String>)> = {
        let (w, column, plan_slot, error) = (w.clone(), column.clone(), plan_slot.clone(), error);
        Rc::new(move |read| {
            while let Some(c) = column.first_child() {
                column.remove(&c);
            }
            while let Some(c) = plan_slot.first_child() {
                plan_slot.remove(&c);
            }
            let thread = match read {
                Ok(t) => t,
                Err(why) => {
                    error.set_label(&why);
                    error.set_visible(true);
                    return;
                }
            };
            error.set_visible(false);
            let mode = Mode::Kiff;
            match tafs_core::coach_lines::lines(&w, &thread, &listed, mode, chrono::Utc::now()) {
                Ok(lines) => {
                    if let Some(plan) = &lines.plan {
                        let p = plan_panel(plan);
                        p.set_margin_start(12);
                        p.set_margin_end(12);
                        p.set_margin_top(8);
                        plan_slot.append(&p);
                    }
                    draw_thread(&column, &lines, &open);
                }
                Err(e) => {
                    error.set_label(&e.to_string());
                    error.set_visible(true);
                }
            }
        })
    };

    match source {
        CoachSource::None => {}
        CoachSource::File(path) => {
            let read = std::fs::read(&path)
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok())
                .and_then(|b| SetupThread::read(&b));
            match read {
                Some(t) => draw(Ok(t)),
                None => {
                    eprintln!(
                        "{} isn't Coach's conversation as tafs cloud answers it",
                        path.display()
                    );
                    std::process::exit(2);
                }
            }
        }
        CoachSource::Cloud { kept, languages } => {
            let words = (*w).clone();
            gtk::glib::spawn_future_local(async move {
                let read = gtk::gio::spawn_blocking(move || read_cloud(&words, &kept, &languages))
                    .await
                    .unwrap_or_else(|_| Err(String::new()));
                draw(read);
            });
        }
    }
    scroller.add_css_class("tafs-coach-wash");
    root.upcast()
}

/// tafs cloud's words for a refusal, or the app's plain words for what went wrong.
fn refusal(w: &WordBook, a: &tafs_core::signin::Answer) -> String {
    tafs_core::signin::refusal(a, Some(w))
        .words(w)
        .unwrap_or_default()
}

/// `loadSetupThread`, read only: Coach's taf (`GET /v1/setup`), its newest chat, this device's
/// proof, then the conversation with it. Without a key or this computer's id in KIFF, it reads
/// without the proof, as the iPad does; tafs cloud then marks no card.
fn read_cloud(
    w: &WordBook,
    kept: &KeptSession,
    languages: &[&'static str],
) -> Result<SetupThread, String> {
    let why = |why: String| {
        w.get("coach.conversation_not_loaded", &[("why", &why)])
            .unwrap_or(why)
    };
    let plain = || tafs_core::signin::plain_words(w, 200).unwrap_or_default();
    let token = &kept.token;
    let a = crate::kiff::cloud(token, languages, coach::SETUP_PATH)
        .map_err(|e| why(e.words(w).unwrap_or_default()))?;
    if !(200..300).contains(&a.status) {
        return Err(why(refusal(w, &a)));
    }
    let Some(id) = coach::setup_id(&a.body).map_err(|_| why(plain()))? else {
        // No Coach yet: the empty chat's words. Starting it is signed, on the phone.
        return Ok(SetupThread::default());
    };
    // Coach opens on its newest conversation alone; without the list, the thread reads as before.
    let session = crate::kiff::cloud(token, languages, &coach::chats_path(&id))
        .ok()
        .filter(|a| (200..300).contains(&a.status))
        .and_then(|a| coach::chats(&a.body))
        .and_then(|chats| coach::newest(&chats).map(|c| c.session.clone()));
    let proof = proof(kept, languages);
    // This computer signs no step beyond reading: it never asks for `can`.
    let path = coach::conversation_path(&id, session.as_deref(), proof.is_some(), &[]);
    let headers: Vec<(&str, &str)> = proof
        .as_deref()
        .map(|p| vec![(tafs_core::planque::HEADER, p)])
        .unwrap_or_default();
    let a = crate::kiff::cloud_with(token, languages, &path, &headers)
        .map_err(|e| why(e.words(w).unwrap_or_default()))?;
    if !(200..300).contains(&a.status) {
        return Err(why(refusal(w, &a)));
    }
    SetupThread::read(&a.body).ok_or_else(|| why(plain()))
}

/// This device's signed read of its grants (`grantsReadProof`): only for a session with a key and
/// a tenant, once KIFF names this computer. None otherwise.
fn proof(kept: &KeptSession, languages: &[&'static str]) -> Option<String> {
    let tenant = kept.tenant.as_deref().filter(|t| !t.is_empty())?;
    let key = kept.key()?;
    let a = crate::kiff::devices(&kept.token, languages).ok()?;
    if !(200..300).contains(&a.status) {
        return None;
    }
    let device = tafs_core::devices::this_device_id(&a.body)?;
    Some(tafs_core::planque::grants_read_proof(
        &key,
        &device,
        tenant,
        tafs_core::planque::PLANQUE,
    ))
}

fn tone_class(t: Tone) -> &'static str {
    match t {
        Tone::Ink => "tafs-ink",
        Tone::Soft => "tafs-soft",
        Tone::Dim | Tone::Line => "tafs-dim",
        Tone::Ok => "tafs-ok",
        Tone::No => "tafs-no",
        Tone::Amber => "tafs-amber-text",
    }
}

/// A panel with its edge in a state's colour (`Panel(stroke:)`).
fn card_panel(stroke: Tone) -> gtk::Box {
    let p = panel(false);
    match stroke {
        Tone::Amber => p.add_css_class("edge-amber"),
        Tone::Ok => p.add_css_class("edge-ok"),
        Tone::No => p.add_css_class("edge-no"),
        Tone::Ink => p.add_css_class("edge-ink"),
        _ => {}
    }
    p
}

/// `Eyebrow(text:color:)`.
fn eyebrow_in(s: &str, tone: Tone) -> gtk::Label {
    let l = eyebrow(s, tone == Tone::Amber);
    match tone {
        Tone::Ok => l.add_css_class("ok"),
        Tone::No => l.add_css_class("tafs-no"),
        _ => {}
    }
    l
}

fn draw_thread(column: &gtk::Box, lines: &CoachLines, open: &Rc<dyn Fn(Selection)>) {
    if let Some((title, line)) = &lines.starts {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 12);
        v.set_margin_top(16);
        v.append(&text(title, 26, 700, &["tafs-ink"]));
        v.append(&text(line, 15, 400, &["tafs-soft"]));
        column.append(&v);
    }
    for line in &lines.lines {
        column.append(&match line {
            Line::Gap(at) => {
                let l = text(at, 12, 400, &["tafs-dim"]);
                l.set_xalign(0.5);
                l.set_justify(gtk::Justification::Center);
                l.set_margin_top(6);
                l.upcast::<gtk::Widget>()
            }
            Line::Owner {
                text: words,
                answered,
            } => owner(words, answered.as_deref(), false),
            Line::Coach { work, text: words } => coach_words(work.as_ref(), words),
            Line::Card { card_ref, card } => {
                let c = card_view(card, open);
                c.set_widget_name(&format!("card.{card_ref}"));
                c
            }
            Line::Drive { eyebrow: e, task } => {
                let p = panel(false);
                p.append(&eyebrow_in(e, Tone::Amber));
                p.append(&text(task, 17, 600, &["tafs-ink"]));
                p.upcast()
            }
        });
    }
    if let Some((title, said)) = &lines.away {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
        b.add_css_class("tafs-away");
        b.append(&text(title, 14, 600, &["tafs-ink"]));
        for s in said {
            b.append(&text(s, 13, 400, &["tafs-soft"]));
        }
        column.append(&b);
    }
    for said in &lines.coach_lines {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let arrow = text("↳", 11, 400, &["tafs-dim"]);
        arrow.set_valign(gtk::Align::Start);
        arrow.set_accessible_role(gtk::AccessibleRole::Presentation);
        h.append(&arrow);
        let l = text(said, 13, 400, &["tafs-soft"]);
        l.set_hexpand(true);
        h.append(&l);
        column.append(&h);
    }
    // Coach at work: the dots where its words will be.
    if let Some(answering) = &lines.thinking {
        let dots = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        dots.set_margin_top(8);
        dots.set_margin_bottom(8);
        for _ in 0..3 {
            let d = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            d.add_css_class("tafs-typing");
            d.set_valign(gtk::Align::Center);
            dots.append(&d);
        }
        dots.set_accessible_role(gtk::AccessibleRole::Img);
        dots.update_property(&[gtk::accessible::Property::Label(answering)]);
        column.append(&dots);
    }
    for (words, note) in &lines.steering {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
        v.append(&owner(words, None, true));
        let n = text(note, 12, 400, &["tafs-dim"]);
        n.set_xalign(1.0);
        v.append(&n);
        column.append(&v);
    }
}

/// The owner's words on the right, on the brand sweep (`ChatBubble`), and under them that they
/// answered the card above.
fn owner(words: &str, answered: Option<&str>, steered: bool) -> gtk::Widget {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let bubble = text(words, 15, 400, &["tafs-owner-bubble"]);
    if steered {
        bubble.add_css_class("steered");
    }
    bubble.set_halign(gtk::Align::End);
    bubble.set_margin_start(48);
    bubble.set_selectable(true);
    bubble.set_max_width_chars(60);
    v.append(&bubble);
    if let Some(a) = answered {
        let l = text(a, 12, 400, &["tafs-dim"]);
        l.set_xalign(1.0);
        v.append(&l);
    }
    v.upcast()
}

/// Coach's words full width, with no bubble, under its work folded to "Worked for".
fn coach_words(work: Option<&WorkLines>, words: &str) -> gtk::Widget {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    if let Some(work) = work {
        v.append(&work_summary(work));
    }
    let l = text("", 16, 400, &["tafs-ink"]);
    // Its size alone: a weight over the whole would undo the words' own bold.
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrSize::new_size_absolute(
        16 * gtk::pango::SCALE,
    ));
    l.set_attributes(Some(&attrs));
    l.set_markup(&tafs_core::coach_lines::markup(words));
    l.set_selectable(true);
    l.set_margin_top(2);
    l.set_margin_bottom(2);
    v.append(&l);
    v.upcast()
}

/// `WorkSummary`: "Worked for 12 s ›", which opens its steps.
fn work_summary(work: &WorkLines) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let b = gtk::Button::new();
    b.add_css_class("tafs-work");
    b.set_halign(gtk::Align::Start);
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    h.append(&text(&work.summary, 13, 400, &["tafs-dim"]));
    let chevron = symbol("pan-end-symbolic", 11);
    chevron.add_css_class("tafs-dim");
    h.append(&chevron);
    b.set_child(Some(&h));
    b.update_property(&[
        gtk::accessible::Property::Label(&work.accessible),
        gtk::accessible::Property::Description(&work.shows),
    ]);
    v.append(&b);
    let steps = gtk::Box::new(gtk::Orientation::Vertical, 4);
    steps.add_css_class("tafs-step-list");
    for row in &work.steps {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let (icon, class) = match row.status {
            StepStatus::Done => ("tafs-checkmark-symbolic", "tafs-dim"),
            StepStatus::Error => ("dialog-error-symbolic", "tafs-no"),
            StepStatus::NotMade => ("edit-undo-symbolic", "tafs-dim"),
        };
        let mark = symbol(icon, 11);
        mark.add_css_class(class);
        mark.set_size_request(14, -1);
        h.append(&mark);
        h.append(&text(&row.words, 13, 400, &["tafs-soft"]));
        if let Some(note) = &row.note {
            h.append(&text(note, 12, 400, &["tafs-dim"]));
        }
        steps.append(&h);
    }
    steps.set_visible(false);
    v.append(&steps);
    let (shows, hides) = (work.shows.clone(), work.hides.clone());
    b.connect_clicked(move |b| {
        let open = !steps.is_visible();
        steps.set_visible(open);
        chevron.set_icon_name(Some(if open {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        }));
        b.update_property(&[gtk::accessible::Property::Description(if open {
            &hides
        } else {
            &shows
        })]);
    });
    v
}

/// `StepMark`: done, failed, at work or to do.
fn plan_mark(step: &PlanStep) -> gtk::Widget {
    let mark = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    mark.add_css_class("tafs-plan-mark");
    mark.set_halign(gtk::Align::Center);
    mark.set_valign(gtk::Align::Center);
    match step.status.as_str() {
        "done" | "failed" => {
            mark.add_css_class(&step.status);
            let i = symbol(
                if step.status == "done" {
                    "tafs-checkmark-symbolic"
                } else {
                    "window-close-symbolic"
                },
                9,
            );
            i.set_hexpand(true);
            mark.append(&i);
        }
        "active" => mark.add_css_class("active"),
        _ => mark.add_css_class("pending"),
    }
    mark.set_accessible_role(gtk::AccessibleRole::Img);
    mark.update_property(&[gtk::accessible::Property::Label(&step.mark_words)]);
    mark.upcast()
}

/// `SetupPlan`: pinned above the thread, folded to the step in progress until opened.
fn plan_panel(plan: &PlanLines) -> gtk::Box {
    let p = panel(false);
    let b = gtk::Button::new();
    b.add_css_class("tafs-plain");
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let now_mark = if plan.now.status.is_empty() {
        None
    } else {
        Some(plan_mark(&plan.now))
    };
    if let Some(m) = &now_mark {
        h.append(m);
    }
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    v.append(&eyebrow(&plan.progress, false));
    let now = crate::widgets::one_line(&plan.now.label, 14, 600, &["tafs-ink"]);
    v.append(&now);
    h.append(&v);
    let chevron = symbol("pan-down-symbolic", 12);
    chevron.add_css_class("tafs-dim");
    h.append(&chevron);
    b.set_child(Some(&h));
    b.update_property(&[gtk::accessible::Property::Description(&plan.shows)]);
    p.append(&b);
    let steps = gtk::Box::new(gtk::Orientation::Vertical, 8);
    for s in &plan.steps {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(&plan_mark(s));
        let l = text(
            &s.label,
            14,
            if s.status == "active" { 600 } else { 400 },
            &[if s.status == "pending" || s.status == "skipped" {
                "tafs-dim"
            } else {
                "tafs-ink"
            }],
        );
        if s.status == "skipped" {
            let attrs = l.attributes().unwrap_or_default();
            attrs.insert(gtk::pango::AttrInt::new_strikethrough(true));
            l.set_attributes(Some(&attrs));
        }
        row.append(&l);
        steps.append(&row);
    }
    steps.set_visible(false);
    p.append(&steps);
    let (shows, folds) = (plan.shows.clone(), plan.folds.clone());
    b.connect_clicked(move |b| {
        let open = !steps.is_visible();
        steps.set_visible(open);
        now.set_visible(!open);
        if let Some(m) = &now_mark {
            m.set_visible(!open);
        }
        chevron.set_icon_name(Some(if open {
            "pan-up-symbolic"
        } else {
            "pan-down-symbolic"
        }));
        b.update_property(&[gtk::accessible::Property::Description(if open {
            &folds
        } else {
            &shows
        })]);
    });
    p
}

/// `OutcomeLine`: an answered card in one line, its mark in its colour.
fn outcome(o: &Outcome) -> gtk::Box {
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    h.add_css_class("tafs-outcome");
    let (icon, class) = match o.mark {
        Mark::Check => ("emblem-ok-symbolic", tone_class(o.tone)),
        Mark::Minus => ("list-remove-symbolic", tone_class(o.tone)),
        Mark::Below => ("go-down-symbolic", tone_class(o.tone)),
    };
    let m = symbol(icon, 14);
    m.add_css_class(class);
    m.set_valign(gtk::Align::Start);
    m.set_margin_top(1);
    h.append(&m);
    let l = text(&o.text, 14, 400, &["tafs-soft"]);
    l.set_hexpand(true);
    h.append(&l);
    if o.more {
        let c = symbol("pan-down-symbolic", 11);
        c.add_css_class("tafs-dim");
        h.append(&c);
    }
    h
}

/// The answers this computer doesn't send: there, dimmed.
fn dimmed(b: &gtk::Button) {
    b.set_sensitive(false);
}

/// A row of buttons that wraps (`WrapRow`).
fn wrap_row(spacing: u32) -> gtk::FlowBox {
    let f = gtk::FlowBox::new();
    f.set_selection_mode(gtk::SelectionMode::None);
    f.set_column_spacing(spacing);
    f.set_row_spacing(spacing);
    f.set_homogeneous(false);
    f.set_max_children_per_line(12);
    // Filling the row: a row that shrinks to its children misreads its height once they wrap.
    f.set_halign(gtk::Align::Fill);
    f.set_valign(gtk::Align::Start);
    f.set_can_focus(false);
    f
}

fn card_view(card: &Card, open: &Rc<dyn Fn(Selection)>) -> gtk::Widget {
    match card {
        Card::Notice {
            title,
            text: words,
            tone,
        } => {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
            v.add_css_class("tafs-notice");
            match tone {
                Tone::No => v.add_css_class("no"),
                Tone::Amber => v.add_css_class("amber"),
                Tone::Ok => v.add_css_class("ok"),
                _ => {}
            }
            if let Some(t) = title {
                v.append(&text(t, 14, 600, &[tone_class(*tone)]));
            }
            v.append(&text(words, 14, 400, &["tafs-ink"]));
            v.upcast()
        }
        Card::Outcome(o) => outcome(o).upcast(),
        Card::Confirm {
            text: words,
            options,
        } => {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 8);
            v.append(&text(words, 15, 400, &["tafs-ink"]));
            let row = wrap_row(8);
            for o in options {
                let b = gtk::Button::with_label(o);
                b.add_css_class("tafs-capsule");
                dimmed(&b);
                b.set_halign(gtk::Align::Start);
                row.insert(&b, -1);
            }
            v.append(&row);
            v.upcast()
        }
        Card::Questions {
            text: words,
            eyebrow: e,
            count,
            questions,
            something_else,
            send,
            chat_about,
        } => {
            let p = panel(false);
            if let Some(t) = words {
                p.append(&text(t, 15, 400, &["tafs-ink"]));
            }
            let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let eb = eyebrow(e, false);
            eb.set_hexpand(true);
            h.append(&eb);
            h.append(&text(count, 12, 400, &["tafs-mono", "tafs-dim"]));
            p.append(&h);
            for q in questions {
                let v = gtk::Box::new(gtk::Orientation::Vertical, 10);
                v.append(&text(&q.title, 16, 600, &["tafs-ink"]));
                let row = wrap_row(8);
                for (label, detail) in q
                    .options
                    .iter()
                    .chain(std::iter::once(&(something_else.clone(), None)))
                {
                    let b = gtk::Button::new();
                    b.add_css_class("tafs-choice");
                    let inner = gtk::Box::new(gtk::Orientation::Vertical, 2);
                    // One line each in the row: a wrapping label can't tell the row its height.
                    let l = text(label, 15, 400, &[]);
                    l.set_wrap(false);
                    inner.append(&l);
                    if let Some(d) = detail {
                        let l = text(d, 12, 400, &[]);
                        l.set_wrap(false);
                        inner.append(&l);
                    }
                    b.set_child(Some(&inner));
                    dimmed(&b);
                    b.set_halign(gtk::Align::Start);
                    row.insert(&b, -1);
                }
                v.append(&row);
                if let Some(n) = &q.note {
                    v.append(&text(n, 13, 400, &["tafs-soft"]));
                }
                p.append(&v);
            }
            let line = gtk::Separator::new(gtk::Orientation::Horizontal);
            line.add_css_class("tafs-line");
            p.append(&line);
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let s = primary(send);
            s.set_hexpand(false);
            dimmed(&s);
            row.append(&s);
            let chat = gtk::Button::with_label(chat_about);
            chat.add_css_class("tafs-coach-link");
            dimmed(&chat);
            row.append(&chat);
            p.append(&row);
            p.upcast()
        }
        Card::Approval(a) => approval(a),
        Card::SignIn(s) => sign_in(s),
        Card::Result(r) => result(r, open),
        Card::Drive {
            eyebrow: e,
            task,
            note,
            gate,
        } => {
            let p = card_panel(Tone::Amber);
            p.append(&eyebrow_in(e, Tone::Amber));
            p.append(&text(task, 20, 700, &["tafs-ink"]));
            p.append(&text(note, 14, 400, &["tafs-soft"]));
            if let Some(g) = gate {
                p.append(&crate::approval::not_ready(g));
            }
            p.upcast()
        }
        Card::Title(t) => {
            let p = panel(false);
            p.append(&text(t, 15, 600, &["tafs-ink"]));
            p.upcast()
        }
    }
}

/// tafs cloud's `wider` line, above where the buttons go (`WiderLine`): 14 medium, in the amber.
fn wider_line(words: &str) -> gtk::Label {
    let l = text(words, 14, 500, &["tafs-amber-text"]);
    l.set_widget_name("card.wider");
    l
}

/// `SetupApproval`, read only.
fn approval(a: &ApprovalLines) -> gtk::Widget {
    let p = card_panel(a.stroke);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let e = eyebrow_in(&a.eyebrow, a.eyebrow_tone);
    e.set_hexpand(true);
    head.append(&e);
    let fold = a.fold.as_ref().map(|f| {
        let b = gtk::Button::with_label(f);
        b.add_css_class("tafs-plain");
        b.add_css_class("tafs-dim");
        head.append(&b);
        b
    });
    p.append(&head);
    if let Some(done) = &a.done {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let m = symbol("emblem-ok-symbolic", 14);
        m.add_css_class("tafs-ok");
        h.append(&m);
        h.append(&text(done, 14, 600, &["tafs-ok"]));
        p.append(&h);
    }
    if let Some(t) = &a.title {
        p.append(&text(t, 17, 600, &["tafs-ink"]));
    }
    if let Some(t) = &a.text {
        p.append(&text(t, 15, 400, &["tafs-ink"]));
    }
    if let Some(next) = &a.card_next {
        p.append(&text(next, 13, 600, &["tafs-soft"]));
    }
    if let Some(w) = &a.wider {
        p.append(&wider_line(w));
    }
    if let Some(r) = &a.replaced {
        p.append(&text(r, 13, 400, &["tafs-soft"]));
    }
    if let Some(f) = &a.failed {
        p.append(&text(f, 14, 400, &["tafs-no"]));
    }
    if let Some(g) = &a.gate {
        p.append(&crate::approval::not_ready(g));
    }
    let Some(folded) = &a.folded else {
        return p.upcast();
    };
    // Answered, or asked again below: its line, which opens the card; Fold puts it back.
    let stack = gtk::Stack::new();
    stack.set_vhomogeneous(false);
    stack.set_hhomogeneous(false);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(200);
    let line = gtk::Button::new();
    line.add_css_class("tafs-outcome-button");
    line.set_child(Some(&outcome(folded)));
    if let Some(hint) = &folded.hint {
        line.update_property(&[gtk::accessible::Property::Description(hint)]);
    }
    stack.add_named(&line, Some("line"));
    stack.add_named(&p, Some("card"));
    stack.set_visible_child_name("line");
    {
        let stack = stack.clone();
        line.connect_clicked(move |_| stack.set_visible_child_name("card"));
    }
    if let Some(fold) = fold {
        let stack = stack.clone();
        fold.connect_clicked(move |_| stack.set_visible_child_name("line"));
    }
    stack.upcast()
}

/// A vendor's sign-in, read only: its words, the wider line, and why this computer can't do it
/// in its button's place.
fn sign_in(s: &SignInLines) -> gtk::Widget {
    let p = card_panel(s.stroke);
    p.append(&eyebrow_in(&s.eyebrow, s.eyebrow_tone));
    p.append(&text(&s.title, 20, 700, &["tafs-ink"]));
    if let Some(f) = &s.failed {
        p.append(&text(f, 14, 400, &["tafs-no"]));
    }
    p.append(&text(&s.note, 14, 400, &["tafs-soft"]));
    if let Some(w) = &s.wider {
        p.append(&wider_line(w));
    }
    if let Some(g) = &s.gate {
        p.append(&crate::approval::not_ready(g));
    }
    p.upcast()
}

/// A result: the taf set up, or its new version.
fn result(r: &ResultLines, open: &Rc<dyn Fn(Selection)>) -> gtk::Widget {
    let p = card_panel(Tone::Ok);
    p.append(&eyebrow_in(&r.eyebrow, Tone::Ok));
    p.append(&text(&r.text, 15, 400, &["tafs-ink"]));
    if let Some(d) = &r.detail {
        p.append(&text(d, 13, 400, &["tafs-soft"]));
    }
    if let Some((words, choose)) = &r.plan_needed {
        p.append(&text(words, 14, 600, &["tafs-amber-text"]));
        let b = outline(choose);
        let open = open.clone();
        b.connect_clicked(move |_| open(Selection::Usage));
        p.append(&b);
    }
    if let Some((words, key)) = &r.open {
        let b = primary(words);
        let (open, key) = (open.clone(), key.clone());
        b.connect_clicked(move |_| open(Selection::Taf(key.clone())));
        p.append(&b);
    }
    for (words, tone) in &r.lines {
        p.append(&text(words, 13, 400, &[tone_class(*tone)]));
    }
    if let Some(preparing) = &r.preparing {
        let b = primary(preparing);
        dimmed(&b);
        p.append(&b);
    }
    p.upcast()
}
