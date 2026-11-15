//! Account beside the sidebar (`SettingsView`): its sections as the iPad's grouped list.

use std::rc::Rc;

use adw::prelude::*;
use tafs_core::account::{AccountLines, Action, Row, Screen};

use crate::widgets::text;

/// What Account's rows do: the app's own actions, and the screens they open.
pub struct Actions {
    pub act: Rc<dyn Fn(Action)>,
    pub open: Rc<dyn Fn(Screen, String)>,
}

pub fn page(a: &AccountLines, actions: &Actions) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
    column.set_margin_start(24);
    column.set_margin_end(24);
    column.set_margin_top(32);
    column.set_margin_bottom(24);
    let title = text(&a.title, 34, 700, &["tafs-ink"]);
    title.set_width_chars(16);
    title.set_margin_bottom(12);
    column.append(&title);
    for section in &a.sections {
        if let Some(header) = &section.header {
            let h = text(header, 17, 600, &["tafs-dim"]);
            h.set_margin_start(20);
            h.set_margin_top(20);
            h.set_margin_bottom(8);
            column.append(&h);
        } else {
            let gap = gtk::Box::new(gtk::Orientation::Vertical, 0);
            gap.set_size_request(-1, 28);
            column.append(&gap);
        }
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("tafs-group");
        let mut opens = Vec::new();
        for row in &section.rows {
            let r = row_widget(row, actions);
            if let Row::Opens { title, screen, .. } = row {
                opens.push((r.clone(), *screen, title.clone()));
            }
            list.append(&r);
        }
        let open = actions.open.clone();
        list.connect_row_activated(move |_, row| {
            if let Some((_, screen, title)) = opens.iter().find(|(r, _, _)| r == row) {
                open(*screen, title.clone());
            }
        });
        column.append(&list);
    }
    column.set_valign(gtk::Align::Start);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    crate::widgets::ground(&scroller);
    scroller.upcast()
}

fn line(label: &str, value: Option<&str>, chevron: bool) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let l = text(label, 17, 400, &["tafs-ink"]);
    l.set_hexpand(true);
    b.append(&l);
    if let Some(v) = value {
        let v = text(v, 17, 400, &["tafs-dim"]);
        v.set_xalign(1.0);
        b.append(&v);
    }
    if chevron {
        let c = gtk::Image::from_icon_name("go-next-symbolic");
        c.add_css_class("tafs-dim");
        b.append(&c);
    }
    b
}

fn row_widget(row: &Row, actions: &Actions) -> gtk::ListBoxRow {
    let r = gtk::ListBoxRow::new();
    r.set_activatable(false);
    match row {
        Row::Value(label, value) => r.set_child(Some(&line(label, Some(value), false))),
        Row::Text(words) => r.set_child(Some(&text(words, 15, 400, &["tafs-soft"]))),
        Row::Problem(words) => r.set_child(Some(&text(words, 15, 400, &["tafs-no"]))),
        Row::Opens {
            title,
            value,
            screen,
        } => {
            r.set_child(Some(&line(title, value.as_deref(), true)));
            r.set_activatable(true);
            let _ = screen;
        }
        Row::Link(words, url) => {
            let b = row_button(words);
            b.add_css_class("amber");
            let url = url.to_string();
            b.connect_clicked(move |b| {
                let window = b.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                gtk::UriLauncher::new(&url).launch(
                    window.as_ref(),
                    gtk::gio::Cancellable::NONE,
                    |_| {},
                );
            });
            r.set_child(Some(&b));
        }
        Row::Action(words, action) | Row::Destructive(words, action) => {
            let b = row_button(words);
            b.add_css_class(if matches!(row, Row::Destructive(..)) {
                "no"
            } else {
                "amber-text"
            });
            // Linux has no per-app language setting to open: the app speaks the desktop's language.
            if *action == Action::ChangeLanguage {
                b.set_sensitive(false);
            }
            let (act, action) = (actions.act.clone(), *action);
            b.connect_clicked(move |_| act(action));
            r.set_child(Some(&b));
        }
        Row::Theme {
            light,
            dark,
            system,
        } => {
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            b.add_css_class("tafs-segmented");
            b.set_homogeneous(true);
            b.set_hexpand(true);
            let chosen = crate::store::theme();
            let mut first: Option<gtk::ToggleButton> = None;
            for (choice, words) in [("light", light), ("dark", dark), ("system", system)] {
                let t = gtk::ToggleButton::with_label(words);
                t.add_css_class("segment");
                if let Some(f) = &first {
                    t.set_group(Some(f));
                } else {
                    first = Some(t.clone());
                }
                t.set_active(chosen == choice);
                t.connect_toggled(move |t| {
                    if t.is_active() {
                        crate::store::set_theme(choice);
                        adw::StyleManager::default().set_color_scheme(scheme(choice));
                    }
                });
                b.append(&t);
            }
            r.set_child(Some(&b));
        }
    }
    r
}

/// A row that is a button: its words from the leading edge, as the iPad's list draws them.
fn row_button(words: &str) -> gtk::Button {
    let b = gtk::Button::new();
    let l = gtk::Label::new(Some(words));
    l.set_xalign(0.0);
    l.set_hexpand(true);
    b.set_child(Some(&l));
    b.set_hexpand(true);
    b.add_css_class("tafs-row-button");
    b
}

/// The appearance a choice asks for.
pub fn scheme(choice: &str) -> adw::ColorScheme {
    match choice {
        "light" => adw::ColorScheme::ForceLight,
        "dark" => adw::ColorScheme::ForceDark,
        _ => adw::ColorScheme::Default,
    }
}
