//! Account › Alerts (`NotificationSettingsView`): what an alert looks like, then, signed in with
//! KIFF, alerts on this computer and their sound, kept across launches, with the line under
//! them. The lines are `tafs-core/src/alerts.rs`. Alerts here are the held calls this computer
//! reads once a minute while tafs runs (`main.rs` › `watch_holds`).

use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::alerts::{lines, Example};
use tafs_core::words::WordBook;

use crate::widgets::{icon_for, mascot, one_line, symbol, text};

/// Signed in with KIFF: whether this person may answer held calls (`canAnswerHolds`), when
/// it's known. The sample has no alerts to turn on, as the iPad's.
#[derive(Clone)]
pub struct AlertsSource {
    pub kiff: Option<Option<bool>>,
}

/// An alert as it shows (`LockScreenAlert`): the mascot, its title and line, and whether it
/// makes a sound.
fn example(e: &Example) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    r.add_css_class("tafs-alert-example");
    let m = mascot(30);
    m.set_valign(gtk::Align::Start);
    r.append(&m);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let t = one_line(&e.title, 14, 600, &["tafs-ink"]);
    t.set_hexpand(true);
    head.append(&t);
    let i = gtk::Image::from_icon_name(icon_for(if e.loud {
        "speaker.wave.2.fill"
    } else {
        "bell.slash"
    }));
    i.set_pixel_size(12);
    i.add_css_class("tafs-dim");
    head.append(&i);
    v.append(&head);
    let line = text(&e.line, 13, 400, &["tafs-soft"]);
    line.set_lines(2);
    line.set_ellipsize(gtk::pango::EllipsizeMode::End);
    v.append(&line);
    r.append(&v);
    r.update_property(&[gtk::accessible::Property::Label(&e.spoken)]);
    r
}

/// A switch's row: its words (and a dim line under them) and the switch.
fn switch_row(
    title: &str,
    under: Option<&str>,
    on: bool,
    sensitive: bool,
) -> (gtk::Box, gtk::Switch) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_margin_top(12);
    row.set_margin_bottom(12);
    let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
    words.set_hexpand(true);
    words.set_valign(gtk::Align::Center);
    words.append(&text(title, 15, 400, &["tafs-ink"]));
    if let Some(u) = under {
        words.append(&text(u, 13, 400, &["tafs-dim"]));
    }
    row.append(&words);
    let s = gtk::Switch::new();
    s.set_active(on);
    s.set_sensitive(sensitive);
    s.set_valign(gtk::Align::Center);
    s.update_property(&[gtk::accessible::Property::Label(title)]);
    row.append(&s);
    (row, s)
}

pub fn page(w: Rc<WordBook>, source: AlertsSource, back: Rc<dyn Fn()>) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let lines = lines(&w, source.kiff.flatten()).ok();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    // Back to Account, as the iPad's navigation bar's round back button.
    let back_button = gtk::Button::new();
    back_button.set_child(Some(&symbol("go-previous-symbolic", 20)));
    back_button.add_css_class("tafs-round-back");
    back_button.set_halign(gtk::Align::Start);
    back_button.set_tooltip_text(Some(&l("settings.title")));
    back_button.update_property(&[gtk::accessible::Property::Label(&l("settings.title"))]);
    back_button.connect_clicked(move |_| back());
    column.append(&back_button);
    let Some(lines) = lines else {
        return column.upcast();
    };
    let title = text(&lines.title, 34, 700, &["tafs-ink"]);
    title.set_width_chars(12);
    column.append(&title);
    let examples = gtk::Box::new(gtk::Orientation::Vertical, 10);
    examples.set_margin_top(8);
    examples.set_margin_bottom(8);
    for e in &lines.examples {
        examples.append(&example(e));
    }
    column.append(&examples);
    // Alerts come from KIFF's held calls, which a signed-in account has.
    if source.kiff.is_some() {
        let on = crate::store::alerts_on();
        let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
        group.add_css_class("tafs-group");
        let (row, alerts) = switch_row(&lines.on_this, None, on, true);
        group.append(&row);
        let line = gtk::Separator::new(gtk::Orientation::Horizontal);
        line.add_css_class("tafs-line");
        group.append(&line);
        // No alerts, no sound: off and greyed while alerts are off here, its choice kept for
        // when they're back.
        let (row, sound) = switch_row(
            &lines.sound,
            Some(&lines.sound_words),
            on && crate::store::alert_sound(),
            on,
        );
        group.append(&row);
        column.append(&group);
        {
            let sound = sound.clone();
            alerts.connect_active_notify(move |s| {
                let on = s.is_active();
                crate::store::set_alerts_on(on);
                sound.set_sensitive(on);
                sound.set_active(on && crate::store::alert_sound());
            });
        }
        sound.connect_active_notify(move |s| {
            // Greyed while alerts are off: only the person's own choice is kept.
            if s.is_sensitive() {
                crate::store::set_alert_sound(s.is_active());
            }
        });
        let foot = text(&lines.foot, 13, 400, &["tafs-dim"]);
        foot.set_margin_start(12);
        foot.set_margin_end(12);
        column.append(&foot);
    }
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    crate::widgets::ground(&scroller);
    scroller.upcast()
}
