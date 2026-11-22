//! The welcome, before anyone is signed in (`OnboardingView`): its drawing, what tafs is, the one
//! tick for the terms with the privacy links and the age line, Continue (KIFF's sign-in in the
//! browser, `app_sign_in`), the sample, and signing in with the phone.

use adw::prelude::*;
use tafs_core::words::WordBook;

use crate::widgets::{outline, primary, text};

pub struct Welcome {
    pub root: gtk::Box,
    /// Why the welcome is open, when it's not the first time: KIFF ended the session.
    pub problem: gtk::Label,
    /// "The Terms changed since you ticked the box…", once KIFF said so.
    pub terms_changed: gtk::Label,
    /// The one tick: Continue and the phone wait on it; the sample never does.
    pub tick: gtk::CheckButton,
    /// Continue: KIFF's sign-in in the browser. Off until the box is ticked.
    pub sign_in: gtk::Button,
    pub sample: gtk::Button,
    /// "Already use tafs on your phone? Add this computer with it." (tafs-macos's welcome).
    pub pair: gtk::Button,
}

thread_local! {
    /// The welcome on screen's tick and "terms changed" line, for a screenshot that shows them.
    static LAST: std::cell::RefCell<Option<(gtk::CheckButton, gtk::Label)>> =
        const { std::cell::RefCell::new(None) };
}

/// For a screenshot: the welcome ticked (`--page welcome-ticked`), or as KIFF left it when the
/// terms changed (`--page welcome-terms-changed`).
pub fn show_as(ticked: bool, terms_changed: bool) {
    LAST.with(|l| {
        if let Some((tick, changed)) = l.borrow().as_ref() {
            tick.set_active(ticked);
            changed.set_visible(terms_changed);
        }
    });
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

    // One tick for both Terms, each name a link; privacy as links below it, never ticked; and the
    // age line for this computer's country. The box is its own control, so a link opens its page
    // and never ticks it.
    let consent = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let terms_changed = text(
        &l("onboarding.terms_changed"),
        14,
        600,
        &["tafs-amber-text"],
    );
    terms_changed.set_visible(false);
    consent.append(&terms_changed);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let tick = gtk::CheckButton::new();
    tick.set_valign(gtk::Align::Start);
    tick.update_property(&[gtk::accessible::Property::Label(
        &w.get(
            "onboarding.consent_tick",
            &[
                ("kiff_terms", &l("onboarding.kiff_terms")),
                ("tafs_terms", &l("onboarding.tafs_terms")),
            ],
        )
        .unwrap_or_else(|e| panic!("{e}")),
    )]);
    row.append(&tick);
    let language = w.language();
    let agree = linked(
        &w.get(
            "onboarding.consent_tick",
            &[("kiff_terms", "\u{1}"), ("tafs_terms", "\u{2}")],
        )
        .unwrap_or_else(|e| panic!("{e}")),
        &[
            (
                l("onboarding.kiff_terms"),
                tafs_core::onboarding::kiff_page(language, "terms"),
            ),
            (
                l("onboarding.tafs_terms"),
                tafs_core::account::terms_url(language),
            ),
        ],
        14,
        &["tafs-ink"],
    );
    agree.set_hexpand(true);
    row.append(&agree);
    consent.append(&row);
    consent.append(&linked(
        &w.get(
            "onboarding.privacy_links",
            &[("kiff_privacy", "\u{1}"), ("tafs_privacy", "\u{2}")],
        )
        .unwrap_or_else(|e| panic!("{e}")),
        &[
            (
                l("onboarding.kiff_privacy"),
                tafs_core::onboarding::kiff_page(language, "privacy"),
            ),
            (
                l("onboarding.tafs_privacy"),
                tafs_core::account::privacy_url(language),
            ),
        ],
        13,
        &["tafs-soft"],
    ));
    let ages = tafs_core::onboarding::MinimumAge::load(&WordBook::default_root());
    let region = tafs_core::onboarding::region_from_env();
    consent.append(&text(
        &tafs_core::onboarding::age_line(w, &ages, region.as_deref())
            .unwrap_or_else(|e| panic!("{e}")),
        12,
        400,
        &["tafs-dim"],
    ));
    root.append(&consent);

    let problem = text("", 14, 400, &["tafs-no"]);
    problem.set_visible(false);
    root.append(&problem);

    let buttons = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let sign_in = primary(&l("common.continue"));
    sign_in.set_sensitive(false);
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
    // The phone's way waits on the same tick: its claim carries the version the tick agreed to.
    pair.set_sensitive(false);
    {
        let (sign_in, pair) = (sign_in.clone(), pair.clone());
        tick.connect_toggled(move |t| {
            sign_in.set_sensitive(t.is_active());
            pair.set_sensitive(t.is_active());
        });
    }
    buttons.append(&sign_in);
    buttons.append(&sample);
    buttons.append(&pair);
    root.append(&buttons);
    LAST.with(|l| *l.borrow_mut() = Some((tick.clone(), terms_changed.clone())));
    Welcome {
        problem,
        terms_changed,
        tick,
        root,
        sign_in,
        sample,
        pair,
    }
}

/// A line with links where its language puts them (`Legal.linked`): each `\u{1}`, `\u{2}`… in
/// `line` is the link of that number, drawn with its words, in the amber.
pub fn linked(line: &str, links: &[(String, String)], size: u32, classes: &[&str]) -> gtk::Label {
    let mut markup = String::new();
    for c in line.chars() {
        let n = c as usize;
        if (1..=links.len()).contains(&n) {
            let (words, url) = &links[n - 1];
            markup.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                gtk::glib::markup_escape_text(url),
                gtk::glib::markup_escape_text(words)
            ));
        } else {
            markup.push_str(&gtk::glib::markup_escape_text(&c.to_string()));
        }
    }
    let label = text("", size, 400, classes);
    label.set_markup(&format!("<span font=\"{size}px\">{markup}</span>"));
    label
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
