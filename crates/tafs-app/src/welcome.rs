//! The welcome, before anyone is signed in (`OnboardingView`): its drawing, what tafs is, Sign in
//! with KIFF, and the sample.

use adw::prelude::*;
use tafs_core::words::WordBook;

use crate::widgets::{outline, primary, text};

pub struct Welcome {
    pub root: gtk::Box,
    /// Why the welcome is open, when it's not the first time: KIFF ended the session.
    pub problem: gtk::Label,
    pub sign_in: gtk::Button,
    pub sample: gtk::Button,
    /// "Already use tafs on your phone? Add this computer with it." (tafs-macos's welcome).
    pub pair: gtk::Button,
}

pub fn build(w: &WordBook) -> Welcome {
    let l = |k: &str| w.line(k).unwrap_or_else(|e| panic!("{e}"));
    let root = gtk::Box::new(gtk::Orientation::Vertical, 24);
    root.set_margin_start(24);
    root.set_margin_end(24);
    root.set_margin_top(48);
    root.set_margin_bottom(24);
    root.set_vexpand(true);
    crate::widgets::ground(&root);

    // Board 1, first run (`Illustration.firstRun`): a card opening, above the words. It gets
    // smaller in a short window rather than push the buttons off it.
    root.append(&crate::illustrations::first_run().shrinking());

    let head = gtk::Box::new(gtk::Orientation::Vertical, 12);
    head.append(&text(&l("sign_in.welcome.title"), 48, 700, &["tafs-ink"]));
    head.append(&text(&l("sign_in.welcome.lead"), 16, 400, &["tafs-soft"]));
    root.append(&head);

    let points = gtk::Box::new(gtk::Orientation::Vertical, 10);
    for (lead, rest, tone) in [
        (
            "sign_in.welcome.inside_lead",
            "sign_in.welcome.inside_rest",
            "ok",
        ),
        (
            "sign_in.welcome.outside_lead",
            "sign_in.welcome.outside_rest",
            "amber",
        ),
        (
            "sign_in.welcome.revoked_lead",
            "sign_in.welcome.revoked_rest",
            "no",
        ),
    ] {
        points.append(&point(&l(lead), &l(rest), tone));
    }
    root.append(&points);

    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    root.append(&spacer);

    let problem = text("", 14, 400, &["tafs-no"]);
    problem.set_visible(false);
    root.append(&problem);

    let buttons = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let sign_in = primary(&l("sign_in.with_kiff"));
    let sample = outline(&l("sample.try_it"));
    let device = l("device.linux");
    let pair = gtk::Button::with_label(
        &w.get("pair.from_sign_in", &[("device", &device)])
            .unwrap_or_else(|e| panic!("{e}")),
    );
    pair.add_css_class("tafs-link");
    pair.set_halign(gtk::Align::Start);
    if let Some(label) = pair.child().and_downcast::<gtk::Label>() {
        label.set_wrap(true);
        label.set_xalign(0.0);
    }
    buttons.append(&sign_in);
    buttons.append(&sample);
    buttons.append(&pair);
    root.append(&buttons);
    Welcome {
        problem,
        root,
        sign_in,
        sample,
        pair,
    }
}

/// A dot in its colour, the lead in bold ink and the rest in the soft colour, at 15.
fn point(lead: &str, rest: &str, tone: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.add_css_class("tafs-point");
    dot.add_css_class(tone);
    dot.set_valign(gtk::Align::Center);
    dot.set_size_request(9, 9);
    dot.set_accessible_role(gtk::AccessibleRole::Presentation);
    row.append(&dot);
    let label = gtk::Label::new(None);
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.set_hexpand(true);
    // The lead in bold ink, the rest in the soft colour: the theme's, again when it changes.
    let (lead, rest) = (lead.to_string(), rest.to_string());
    let draw = {
        let label = label.clone();
        move |dark: bool| {
            let tone = |n: &str| {
                let t = tafs_core::theme::tone(n).expect("a palette colour");
                (if dark { t.dark } else { t.light }).css()
            };
            label.set_markup(&format!(
                "<span font=\"15px\" weight=\"bold\" foreground=\"{}\">{}</span><span font=\"15px\" foreground=\"{}\"> {}</span>",
                tone("ink"),
                gtk::glib::markup_escape_text(&lead),
                tone("soft"),
                gtk::glib::markup_escape_text(&rest)
            ));
        }
    };
    let manager = adw::StyleManager::default();
    draw(manager.is_dark());
    manager.connect_dark_notify(move |m| draw(m.is_dark()));
    row.append(&label);
    row
}
