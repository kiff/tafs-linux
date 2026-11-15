//! Needs you beside the sidebar (`NeedsYouView`): each held call's card (`HeldCallCard`), or
//! "Nothing is waiting." with its hint.

use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::held::remaining;
use tafs_core::needs_you::{Answers, CardLines, NeedsYouLines};
use tafs_core::signin::is_kiff_link;
use tafs_core::words::WordBook;

use crate::widgets::{avatar, outline, panel, primary, text};

pub fn page(w: Rc<WordBook>, n: &NeedsYouLines) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let title = text(&n.title, 34, 700, &["tafs-ink"]);
    title.set_width_chars(16);
    column.append(&title);
    if let Some((nothing, hint)) = &n.empty {
        let p = panel(false);
        p.append(&text(nothing, 15, 600, &["tafs-ink"]));
        p.append(&text(hint, 14, 400, &["tafs-soft"]));
        column.append(&p);
    }
    for card in &n.cards {
        column.append(&held_card(&w, card));
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
