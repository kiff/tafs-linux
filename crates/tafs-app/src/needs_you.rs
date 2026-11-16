//! Needs you beside the sidebar (`NeedsYouView`): the tafs' steps, Coach's waits, "Did this
//! happen?" (`DidThisHappenCard`), and each held call's card (`HeldCallCard`), or "Nothing is
//! waiting." with its hint.

use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::held::remaining;
use tafs_core::needs_you::{Answers, CardLines, NeedsYouLines};
use tafs_core::signin::is_kiff_link;
use tafs_core::words::WordBook;

use crate::widgets::{avatar, outline, panel, primary, text};

pub fn page(
    w: Rc<WordBook>,
    n: &NeedsYouLines,
    open: Rc<dyn Fn(tafs_core::sidebar::Selection)>,
) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let title = text(&n.title, 34, 700, &["tafs-ink"]);
    title.set_width_chars(16);
    column.append(&title);
    // In the iPad's order: the steps the tafs handed the person, then Coach's chats waiting on them.
    for step in &n.steps {
        column.append(&step_card(step));
    }
    if let Some(clear) = &n.set_all_aside {
        // Setting chats aside is the person's act: there, dimmed, until it's ported.
        let b = outline(&clear.button);
        b.add_css_class("tafs-amber-text");
        b.set_sensitive(false);
        column.append(&b);
    }
    for waiting in &n.coach {
        column.append(&coach_card(waiting, n.opens_coach));
    }
    // Money calls KIFF can't prove: the person says on the phone whether they happened.
    for happened in &n.happened {
        column.append(&happened_card(happened));
    }
    for ask in &n.card_units {
        column.append(&card_unit_panel(ask));
    }
    if let Some((nothing, hint)) = &n.empty {
        let p = panel(false);
        p.append(&text(nothing, 15, 600, &["tafs-ink"]));
        p.append(&text(hint, 14, 400, &["tafs-soft"]));
        column.append(&p);
    }
    for card in &n.cards {
        column.append(&held_card(&w, card));
    }
    // A knock at a taf's door waits for the owner too: its row opens the taf, on its Activity,
    // where the knocks are answered.
    for knock in &n.knocks {
        column.append(&knock_row(knock, open.clone()));
    }
    // The pane's full width, 16 from each edge, as the iPad's.
    column.set_valign(gtk::Align::Start);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    crate::widgets::ground(&scroller);
    scroller.upcast()
}

/// `YourStepCard`: tafs cloud's title, the taf, what and why, the vendor's page when tafs cloud
/// vouched for its link (it opens in the browser, with where it really goes under it), and Done or
/// Skip it, signed: there, dimmed, until this computer can sign an answer.
fn step_card(step: &tafs_core::waits::YourStep) -> gtk::Box {
    let p = panel(true);
    p.set_spacing(10);
    if !step.title.is_empty() {
        p.append(&crate::widgets::eyebrow(&step.title, false));
    }
    if let Some((_, name)) = step.taf.as_ref().filter(|(_, n)| !n.is_empty()) {
        p.append(&text(name, 14, 600, &["tafs-ink"]));
    }
    p.append(&text(&step.what, 16, 600, &["tafs-ink"]));
    if !step.why.is_empty() {
        p.append(&text(&step.why, 14, 400, &["tafs-soft"]));
    }
    if let Some((url, host, words)) = step.open_link() {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let open = outline(&words);
        open.connect_clicked(move |b| {
            let launcher = gtk::UriLauncher::new(&url);
            let window = b.root().and_downcast::<gtk::Window>();
            launcher.launch(window.as_ref(), gtk::gio::Cancellable::NONE, |_| {});
        });
        v.append(&open);
        v.append(&text(&host, 12, 400, &["tafs-mono", "tafs-dim"]));
        p.append(&v);
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_homogeneous(true);
    let done = crate::widgets::primary(&step.done);
    done.set_sensitive(false);
    row.append(&done);
    let skip = outline(&step.skip);
    skip.set_sensitive(false);
    row.append(&skip);
    p.append(&row);
    p
}

/// `CoachWaitingCard`: tafs cloud's line with Coach's mark, then Open Coach and Set aside, both
/// there, dimmed, until Coach and setting aside are ported.
fn coach_card(waiting: &tafs_core::waits::CoachWaiting, opens_coach: bool) -> gtk::Box {
    let p = panel(true);
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let mark = crate::widgets::symbol("tafs-sparkle-symbolic", 15);
    mark.add_css_class("tafs-amber-text");
    line.append(&mark);
    let words = text(&waiting.line, 14, 600, &["tafs-ink"]);
    words.set_hexpand(true);
    line.append(&words);
    p.append(&line);
    // Its way in only for someone who may use Coach: else the line alone.
    if opens_coach {
        let open = crate::widgets::primary(&waiting.open);
        open.set_sensitive(false);
        p.append(&open);
    }
    if !waiting.set_aside.is_empty() {
        let aside = outline(&waiting.set_aside);
        aside.set_sensitive(false);
        p.append(&aside);
    }
    p
}

/// `DidThisHappenCard`: an amber-edged panel with "Did this happen?" in amber, KIFF's words for
/// the call (or its title and amount), why KIFF asks, and where No and Yes would be, that this
/// computer can't approve (`ApprovalGate`, one for both): both answers are signed like an approval.
fn happened_card(h: &tafs_core::unproven::HappenedLines) -> gtk::Box {
    let p = panel(true);
    p.set_widget_name(&format!("happened.{}", h.proposal));
    p.append(&text(&h.title, 13, 600, &["tafs-amber-text"]));
    p.append(&text(&h.headline, 15, 600, &["tafs-ink"]));
    if let Some(amount) = &h.amount {
        p.append(&text(amount, 15, 400, &["tafs-ink"]));
    }
    if let Some(why) = &h.why {
        p.append(&text(why, 13, 400, &["tafs-soft"]));
    }
    p.append(&crate::approval::not_ready(&h.can_not_answer));
    p
}

/// The knocked row (`NeedsYouView`): an amber-edged panel with the taf's avatar (32), its name
/// (14 semibold), how many knocks (14, soft) and a chevron; it opens the taf's page, whose
/// Activity opens first.
fn knock_row(
    k: &tafs_core::needs_you::KnockLines,
    open: Rc<dyn Fn(tafs_core::sidebar::Selection)>,
) -> gtk::Button {
    let p = panel(true);
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    line.append(&avatar(&k.initial, k.color, 32));
    let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
    words.set_hexpand(true);
    words.append(&text(&k.name, 14, 600, &["tafs-ink"]));
    words.append(&text(&k.line, 14, 400, &["tafs-soft"]));
    line.append(&words);
    let chevron = crate::widgets::symbol("go-next-symbolic", 13);
    chevron.add_css_class("tafs-dim");
    line.append(&chevron);
    p.append(&line);
    let button = gtk::Button::new();
    button.set_child(Some(&p));
    button.add_css_class("tafs-plain");
    button.set_widget_name(&format!("knocks.{}", k.key));
    button.set_tooltip_text(Some(&k.hint));
    button.update_property(&[gtk::accessible::Property::Description(&k.hint)]);
    let key = k.key.clone();
    button.connect_clicked(move |_| open(tafs_core::sidebar::Selection::Taf(key.clone())));
    button
}

/// `CardUnitExceptionPanel`: tafs cloud's title (16 semibold), words (14, soft) and history (12,
/// dim); its Coach button for someone who may use Coach and manages the taf; and Dismiss in red,
/// which is signed like a refusal. Both there, dimmed: Coach isn't ported, and this computer can't
/// sign.
pub fn card_unit_panel(ask: &tafs_core::needs_you::CardUnitLines) -> gtk::Box {
    let p = panel(false);
    p.set_widget_name(&format!("card-unit.{}", ask.id));
    p.append(&text(&ask.title, 16, 600, &["tafs-ink"]));
    p.append(&text(&ask.words, 14, 400, &["tafs-soft"]));
    p.append(&text(&ask.history, 12, 400, &["tafs-dim"]));
    if let Some(discuss) = &ask.discuss {
        let b = gtk::Button::with_label(discuss);
        b.add_css_class("tafs-link");
        b.set_halign(gtk::Align::Start);
        b.set_sensitive(false);
        p.append(&b);
    }
    let dismiss = gtk::Button::with_label(&ask.dismiss);
    dismiss.add_css_class("tafs-link");
    dismiss.add_css_class("tafs-no");
    dismiss.set_halign(gtk::Align::Start);
    dismiss.set_sensitive(false);
    p.append(&dismiss);
    p
}

/// `HeldCallCard`: an amber-edged panel with the taf, its countdown, the amount, what it asks,
/// why it's held, the taf's own words, its details, and what can be done.
pub fn held_card(w: &Rc<WordBook>, c: &CardLines) -> gtk::Box {
    let p = panel(true);
    let face = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    if let Some((initial, color)) = &c.avatar {
        head.append(&avatar(initial, *color, 32));
    }
    let holder = text(&c.holder, 14, 600, &["tafs-ink"]);
    holder.set_hexpand(true);
    head.append(&holder);
    let left = gtk::Label::new(Some(&c.remaining));
    left.add_css_class("tafs-mono");
    left.add_css_class("tafs-amber-text");
    left.add_css_class("tafs-countdown");
    head.append(&left);
    // Counted down every second, wherever the card is; it stops once the card is gone.
    {
        let (w, expires, weak) = (w.clone(), c.expires_at, left.downgrade());
        gtk::glib::timeout_add_seconds_local(1, move || {
            let Some(left) = weak.upgrade() else {
                return gtk::glib::ControlFlow::Break;
            };
            if let Ok(words) = remaining(&w, expires, chrono::Local::now()) {
                left.set_label(&words);
            }
            gtk::glib::ControlFlow::Continue
        });
    }
    face.append(&head);
    if let Some(amount) = &c.amount {
        face.append(&text(amount, 44, 700, &["tafs-ink"]));
    }
    face.append(&text(&c.summary, 16, 600, &["tafs-ink"]));
    if !c.context.is_empty() {
        face.append(&text(&c.context, 14, 400, &["tafs-soft"]));
    }
    let chip = gtk::Label::new(Some(&c.reason));
    chip.add_css_class("tafs-chip");
    chip.set_halign(gtk::Align::Start);
    chip.set_wrap(true);
    chip.set_xalign(0.0);
    face.append(&chip);
    // One sentence for a screen reader: the taf, what it asks, when it lapses.
    face.update_property(&[gtk::accessible::Property::Label(&c.sentence)]);
    p.append(&face);

    // The taf's own words, drawn verbatim under their own heading, never in place of KIFF's.
    if let Some((heading, lines)) = &c.taf_says {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 4);
        b.add_css_class("tafs-taf-says");
        b.append(&text(heading, 12, 600, &["tafs-soft"]));
        for (i, line) in lines.iter().enumerate() {
            let l = gtk::Label::new(Some(line));
            l.set_use_markup(false);
            l.set_xalign(0.0);
            l.set_wrap(true);
            l.add_css_class("tafs-ink");
            l.add_css_class(if i == 0 { "taf-about" } else { "taf-reason" });
            b.append(&l);
        }
        p.append(&b);
    }
    if let Some((heading, lines)) = &c.details {
        let e = gtk::Expander::new(Some(heading));
        e.add_css_class("tafs-details");
        let b = gtk::Box::new(gtk::Orientation::Vertical, 4);
        b.set_margin_top(4);
        for line in lines {
            let l = gtk::Label::new(Some(line));
            l.set_xalign(0.0);
            l.set_selectable(true);
            l.set_wrap(true);
            l.add_css_class("tafs-mono");
            l.add_css_class("tafs-soft");
            b.append(&l);
        }
        e.set_child(Some(&b));
        p.append(&e);
    }

    match &c.answers {
        Answers::Expired(line) => p.append(&text(line, 14, 400, &["tafs-soft"])),
        Answers::SomeoneAnswers { nothing_sent, line } => {
            p.append(&text(nothing_sent, 14, 400, &["tafs-soft"]));
            p.append(&text(line, 13, 400, &["tafs-soft"]));
        }
        Answers::InKiff {
            nothing_sent,
            line,
            button,
            page,
        } => {
            p.append(&text(nothing_sent, 14, 400, &["tafs-soft"]));
            p.append(&text(line, 13, 400, &["tafs-soft"]));
            let b = primary(button);
            let page = page.clone();
            b.connect_clicked(move |b| {
                if is_kiff_link(&page) {
                    let window = b.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                    gtk::UriLauncher::new(&page).launch(
                        window.as_ref(),
                        gtk::gio::Cancellable::NONE,
                        |_| {},
                    );
                }
            });
            p.append(&b);
        }
        Answers::Here {
            nothing_sent,
            approve,
            over_total,
            next_steps,
            refuse,
        } => {
            p.append(&text(nothing_sent, 14, 400, &["tafs-soft"]));
            // The sample's answers come with the sample's Card checks: until then they're there, dimmed.
            if let Some(approve) = approve {
                let b = primary(approve);
                b.set_sensitive(false);
                p.append(&b);
            }
            if let Some(line) = over_total {
                p.append(&text(line, 13, 400, &["tafs-soft"]));
            }
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.set_homogeneous(true);
            for step in next_steps {
                row.append(&outline(step));
            }
            let r = outline(refuse);
            r.add_css_class("refuse");
            row.append(&r);
            row.set_sensitive(false);
            p.append(&row);
        }
    }
    p
}
