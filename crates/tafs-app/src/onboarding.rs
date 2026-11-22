//! The three cards on how tafs works (`OnboardingCards`), after signing in, before the first Home,
//! and again from How tafs works (`HowTafsWorksView`, in Account). Each draws the app's own
//! components with the sample's words, so the person knows them a minute later and reads them in
//! their language.

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use tafs_core::words::WordBook;

use crate::widgets::{avatar, eyebrow, outline, panel, primary, text};

/// The cards, in order (`OnboardingPage`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Tafs,
    Card,
    Control,
}

pub const PAGES: [Page; 3] = [Page::Tafs, Page::Card, Page::Control];

impl Page {
    fn title(self, w: &WordBook) -> String {
        w.line(match self {
            Page::Tafs => "onboarding.tafs_title",
            Page::Card => "onboarding.card_title",
            Page::Control => "onboarding.control_title",
        })
        .unwrap_or_else(|e| panic!("{e}"))
    }

    /// The last card names this computer's way to approve: on the phone
    /// (`onboarding.control_lead_desktop`, in every shipped language since tafs#1710). A line
    /// missing at the pin leaves the card its title and picture alone: never another device's
    /// words, and never English.
    fn lead(self, w: &WordBook) -> Option<String> {
        match self {
            Page::Tafs => w.line("onboarding.tafs_lead").ok(),
            Page::Card => w.line("onboarding.card_lead").ok(),
            Page::Control => w.line("onboarding.control_lead_desktop").ok(),
        }
    }

    fn index(self) -> usize {
        PAGES.iter().position(|p| *p == self).unwrap_or(0)
    }
}

/// Whether the cards were seen on this computer (Get started or Skip): kept in the user's config.
fn seen_path() -> std::path::PathBuf {
    gtk::glib::user_config_dir()
        .join("tafs")
        .join("onboarding-cards-seen")
}

pub fn seen() -> bool {
    seen_path().exists()
}

fn mark_seen() {
    let p = seen_path();
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(p, "1");
}

/// The cards over the window from `start`. `first`: after signing in, so finishing marks them
/// seen; from How tafs works, they only close.
pub fn present(window: &impl IsA<gtk::Widget>, w: Rc<WordBook>, start: Page, first: bool) {
    let dialog = adw::Dialog::new();
    dialog.set_content_width(520);
    dialog.set_content_height(720);
    let page = Rc::new(Cell::new(start));
    let root = gtk::Box::new(gtk::Orientation::Vertical, 24);
    root.set_margin_start(24);
    root.set_margin_end(24);
    root.set_margin_top(16);
    root.set_margin_bottom(24);
    // The ground fills the dialog to its edges, margins and all.
    let frame = gtk::Box::new(gtk::Orientation::Vertical, 0);
    frame.append(&root);
    crate::widgets::ground(&frame);

    let skip = gtk::Button::with_label(&w.line("onboarding.skip").unwrap_or_default());
    skip.add_css_class("tafs-link");
    skip.set_halign(gtk::Align::End);
    root.append(&skip);

    let picture = gtk::Box::new(gtk::Orientation::Vertical, 0);
    picture.set_vexpand(true);
    picture.set_valign(gtk::Align::Center);
    // A picture of the app, not a control: what it says is in the title and the lead.
    picture.set_can_target(false);
    picture.set_can_focus(false);
    root.append(&picture);

    let title = text("", 30, 700, &["tafs-ink"]);
    title.set_accessible_role(gtk::AccessibleRole::Heading);
    let lead = text("", 16, 400, &["tafs-soft"]);
    let words = gtk::Box::new(gtk::Orientation::Vertical, 12);
    words.append(&title);
    words.append(&lead);
    root.append(&words);

    let dots = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    dots.set_halign(gtk::Align::Center);
    dots.set_accessible_role(gtk::AccessibleRole::Presentation);
    let dot_widgets: Vec<gtk::Box> = PAGES
        .iter()
        .map(|_| {
            let d = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            d.add_css_class("tafs-point");
            d.set_size_request(8, 8);
            dots.append(&d);
            d
        })
        .collect();
    root.append(&dots);

    let next = primary("");
    root.append(&next);

    let show: Rc<dyn Fn()> = {
        let (w, page, picture, title, lead, next) = (
            w.clone(),
            page.clone(),
            picture.clone(),
            title.clone(),
            lead.clone(),
            next.clone(),
        );
        Rc::new(move || {
            let p = page.get();
            while let Some(c) = picture.first_child() {
                picture.remove(&c);
            }
            picture.append(&picture_of(&w, p));
            title.set_text(&p.title(&w));
            let line = p.lead(&w);
            lead.set_visible(line.is_some());
            lead.set_text(&line.unwrap_or_default());
            for (i, d) in dot_widgets.iter().enumerate() {
                if i == p.index() {
                    d.add_css_class("amber");
                    d.remove_css_class("dim");
                } else {
                    d.add_css_class("dim");
                    d.remove_css_class("amber");
                }
            }
            let last = p.index() + 1 == PAGES.len();
            next.set_label(
                &w.line(if last {
                    "onboarding.get_started"
                } else {
                    "common.next"
                })
                .unwrap_or_default(),
            );
        })
    };
    show();

    let finish: Rc<dyn Fn()> = {
        let dialog = dialog.clone();
        Rc::new(move || {
            if first {
                mark_seen();
            }
            dialog.close();
        })
    };
    {
        let finish = finish.clone();
        skip.connect_clicked(move |_| finish());
    }
    {
        let (page, show) = (page.clone(), show.clone());
        next.connect_clicked(move |_| match PAGES.get(page.get().index() + 1) {
            Some(p) => {
                page.set(*p);
                show();
            }
            None => finish(),
        });
    }
    // Closing it any other way (Escape) is Skip.
    if first {
        dialog.connect_closed(|_| mark_seen());
    }
    dialog.set_child(Some(&frame));
    dialog.present(Some(window));
}

/// Each card's picture, drawn by the app's own components with the sample's words.
fn picture_of(w: &WordBook, page: Page) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_else(|e| panic!("{e}"));
    match page {
        // Home's taf rows: the sample's tafs that never move money, each with its avatar, name
        // and job, as Home lists them.
        Page::Tafs => {
            let list = gtk::Box::new(gtk::Orientation::Vertical, 10);
            list.set_size_request(-1, -1);
            list.set_halign(gtk::Align::Fill);
            for (i, key) in ["support", "parcels", "leads"].iter().enumerate() {
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                row.add_css_class("tafs-raised-row");
                let name = l(&format!("sample.{key}.name"));
                row.append(&avatar(&tafs_core::sidebar::initial(&name), i as i64, 36));
                let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
                v.append(&text(&name, 15, 600, &["tafs-ink"]));
                let job = text(&l(&format!("sample.{key}.job")), 13, 400, &["tafs-soft"]);
                job.set_wrap(false);
                job.set_ellipsize(gtk::pango::EllipsizeMode::End);
                v.append(&job);
                v.set_hexpand(true);
                row.append(&v);
                list.append(&row);
            }
            list.upcast()
        }
        // A taf's Card as its Card tab draws it: the sample's Support replies Card, 12 of 40
        // replies used today, issued by you.
        Page::Card => {
            let cards =
                tafs_core::sample::full_cards(w, "support", true).unwrap_or_else(|e| panic!("{e}"));
            let face = crate::taf::face(w, &cards[0], 0, &l("common.you"), false, 0);
            face.set_halign(gtk::Align::Center);
            face.upcast()
        }
        // Needs you's card: the sample's Deploys asks, nothing is sent until you approve, and
        // the three answers.
        Page::Control => {
            let p = panel(true);
            p.append(&eyebrow(&l("sample.deploys.name"), false));
            p.append(&text(
                &l("sample.deploys.deploy_api"),
                16,
                600,
                &["tafs-ink"],
            ));
            p.append(&text(
                &l("sample.deploys.deploy_api_why"),
                14,
                400,
                &["tafs-soft"],
            ));
            p.append(&text(&l("needs_you.nothing_sent"), 13, 400, &["tafs-dim"]));
            p.append(&primary(&l("needs_you.approve_once")));
            p.append(&outline(&l("sample.deploys.next_step")));
            let refuse = outline(&l("needs_you.refuse"));
            refuse.add_css_class("refuse");
            p.append(&refuse);
            p.upcast()
        }
    }
}

/// How tafs works (Account): the three cards again, each opening where it's tapped, and the
/// privacy and terms pages, KIFF's and tafs's, each titled as its page is.
pub fn how_it_works(window: &impl IsA<gtk::Widget>, w: Rc<WordBook>) {
    let l = |k: &str| w.line(k).unwrap_or_else(|e| panic!("{e}"));
    let dialog = adw::Dialog::new();
    dialog.set_title(&l("onboarding.how_it_works"));
    dialog.set_content_width(480);
    dialog.set_content_height(560);
    let root = gtk::Box::new(gtk::Orientation::Vertical, 18);
    root.set_margin_start(20);
    root.set_margin_end(20);
    root.set_margin_top(12);
    root.set_margin_bottom(20);
    let cards = gtk::ListBox::new();
    cards.add_css_class("tafs-group");
    cards.set_selection_mode(gtk::SelectionMode::None);
    for page in PAGES {
        let row = adw::ActionRow::builder()
            .title(page.title(&w))
            .activatable(true)
            .build();
        row.add_suffix(&gtk::Image::from_icon_name("go-next-symbolic"));
        let (w, window) = (w.clone(), window.clone().upcast::<gtk::Widget>());
        row.connect_activated(move |_| present(&window, w.clone(), page, false));
        cards.append(&row);
    }
    root.append(&cards);
    let legal = gtk::ListBox::new();
    legal.add_css_class("tafs-group");
    legal.set_selection_mode(gtk::SelectionMode::None);
    let lang = w.language();
    for (key, url) in [
        (
            "onboarding.row_kiff_privacy",
            tafs_core::onboarding::kiff_page(lang, "privacy"),
        ),
        (
            "onboarding.row_tafs_privacy",
            tafs_core::account::privacy_url(lang),
        ),
        (
            "onboarding.row_kiff_terms",
            tafs_core::onboarding::kiff_page(lang, "terms"),
        ),
        (
            "onboarding.row_tafs_terms",
            tafs_core::account::terms_url(lang),
        ),
    ] {
        let title = tafs_core::account::legal_label(&w, key).unwrap_or_else(|e| panic!("{e}"));
        let row = adw::ActionRow::builder()
            .title(title)
            .activatable(true)
            .build();
        row.add_css_class("amber");
        row.connect_activated(move |r| {
            let window = r.root().and_then(|r| r.downcast::<gtk::Window>().ok());
            gtk::UriLauncher::new(&url).launch(
                window.as_ref(),
                gtk::gio::Cancellable::NONE,
                |_| {},
            );
        });
        legal.append(&row);
    }
    root.append(&legal);
    let header = adw::HeaderBar::new();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&root));
    crate::widgets::ground(&toolbar);
    dialog.set_child(Some(&toolbar));
    dialog.present(Some(window));
}
