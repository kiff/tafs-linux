//! Account beside the sidebar (`SettingsView`): its sections as the iPad's grouped list.

use std::rc::Rc;

use adw::prelude::*;
use tafs_core::account::{AccountLines, Action, Row, Screen, Slot};

use crate::widgets::text;

/// What Account's rows do: the app's own actions, and the screens they open.
pub struct Actions {
    /// What Account reads when it opens, signed in.
    pub reads: Option<Rc<crate::ai_consent::Reads>>,
    pub words: Rc<tafs_core::words::WordBook>,
    pub act: Rc<dyn Fn(Action)>,
    pub open: Rc<dyn Fn(Screen, String)>,
    /// Where Export my chats reads, signed in with KIFF from a kept session; none dims it.
    pub export: Option<Rc<crate::kiff_account::ExportSource>>,
}

pub fn page(a: &AccountLines, actions: &Actions) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
    column.set_margin_start(24);
    column.set_margin_end(24);
    column.set_margin_top(12);
    column.set_margin_bottom(24);
    // Its title in the bar's place, small and centred (`inlineTitle`), as Calendar's and Plan &
    // usage's.
    let title = text(&a.title, 17, 600, &["tafs-ink"]);
    title.set_halign(gtk::Align::Center);
    title.set_margin_bottom(20);
    column.append(&title);
    for section in &a.sections {
        let head: gtk::Widget = if let Some(header) = &section.header {
            let h = text(header, 17, 600, &["tafs-dim"]);
            h.set_margin_start(20);
            h.set_margin_top(20);
            h.set_margin_bottom(8);
            h.upcast()
        } else {
            let gap = gtk::Box::new(gtk::Orientation::Vertical, 0);
            gap.set_size_request(-1, 28);
            gap.upcast()
        };
        column.append(&head);
        let list = gtk::ListBox::new();
        list.set_selection_mode(gtk::SelectionMode::None);
        list.add_css_class("tafs-group");
        // Export my chats (`ExportChatsSection`): its row, and what it exports under it.
        if section.slot == Some(Slot::Export) {
            column.append(&crate::kiff_account::export_section(
                &actions.words,
                actions.export.clone(),
            ));
            let footer = text(
                &actions
                    .words
                    .line("export.own_chats_each_taf_coach")
                    .unwrap_or_default(),
                13,
                400,
                &["tafs-dim"],
            );
            footer.set_margin_start(20);
            footer.set_margin_end(20);
            footer.set_margin_top(8);
            column.append(&footer);
            continue;
        }
        // A part read when Account opens: hidden, header and all, until it's read; never drawn
        // when it can't be, or isn't this person's.
        if let Some(slot) = section.slot {
            head.set_visible(false);
            list.set_visible(false);
            column.append(&list);
            let Some(reads) = actions.reads.clone() else {
                continue;
            };
            let row = gtk::ListBoxRow::new();
            row.set_activatable(false);
            let into = gtk::Box::new(gtk::Orientation::Vertical, 8);
            row.set_child(Some(&into));
            list.append(&row);
            let shown: Rc<dyn Fn()> = {
                let (head, list) = (head.clone(), list.clone());
                Rc::new(move || {
                    head.set_visible(true);
                    list.set_visible(true);
                })
            };
            match slot {
                Slot::Suggestions if reads.may_use_coach => crate::ai_consent::suggestions(
                    actions.words.clone(),
                    reads.source.clone(),
                    reads.manages_cards,
                    into,
                    shown,
                ),
                Slot::Suggestions => {}
                Slot::Consent => crate::ai_consent::consent_row(
                    actions.words.clone(),
                    reads.source.clone(),
                    into,
                    shown,
                ),
                Slot::Export => {}
            }
            continue;
        }
        let mut opens = Vec::new();
        for row in &section.rows {
            let r = row_widget(row, actions);
            match row {
                Row::Opens { title, screen, .. } => opens.push((r.clone(), *screen, title.clone())),
                Row::Plan { title, .. } => opens.push((r.clone(), Screen::Usage, title.clone())),
                _ => {}
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
    // Until the list's end is in sight, a chevron at the foot says there's more, over a fade to
    // the background; it takes no taps.
    let hint = gtk::Box::new(gtk::Orientation::Vertical, 0);
    hint.add_css_class("tafs-more-below");
    hint.set_valign(gtk::Align::End);
    hint.set_can_target(false);
    let chevron = gtk::Image::from_icon_name("pan-down-symbolic");
    chevron.add_css_class("tafs-dim");
    chevron.set_margin_top(16);
    chevron.set_margin_bottom(8);
    hint.append(&chevron);
    hint.set_accessible_role(gtk::AccessibleRole::Presentation);
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&scroller));
    overlay.add_overlay(&hint);
    let adjustment = scroller.vadjustment();
    let at_end = {
        let hint = hint.clone();
        move |a: &gtk::Adjustment| {
            hint.set_visible(a.value() + a.page_size() < a.upper() - 1.0);
        }
    };
    at_end(&adjustment);
    adjustment.connect_value_changed(at_end.clone());
    adjustment.connect_changed(at_end);
    overlay.upcast()
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
        Row::Plan { line, tone, title } => {
            // `PlanRow`: tafs cloud's line in its tone's colour, and Plan & usage under it.
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
            v.set_hexpand(true);
            v.append(&text(
                line,
                16,
                600,
                &[match tone {
                    tafs_core::usage::Tone::No => "tafs-no",
                    tafs_core::usage::Tone::Amber => "tafs-amber-text",
                    tafs_core::usage::Tone::Calm => "tafs-ink",
                }],
            ));
            v.append(&text(title, 13, 400, &["tafs-amber-text"]));
            b.append(&v);
            let c = gtk::Image::from_icon_name("go-next-symbolic");
            c.add_css_class("tafs-dim");
            c.set_valign(gtk::Align::Center);
            b.append(&c);
            r.set_child(Some(&b));
            r.set_activatable(true);
            r.set_widget_name("plan-card");
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
            // Change language opens Account › Language (`LanguagePicker`), with the chevron of a
            // row that opens a screen.
            if *action == Action::ChangeLanguage {
                if let Some(l) = b.child() {
                    let face = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    b.set_child(None::<&gtk::Widget>);
                    face.append(&l);
                    let c = gtk::Image::from_icon_name("go-next-symbolic");
                    c.add_css_class("tafs-dim");
                    face.append(&c);
                    b.set_child(Some(&face));
                }
            }
            let (act, open, action, title) = (
                actions.act.clone(),
                actions.open.clone(),
                *action,
                words.clone(),
            );
            b.connect_clicked(move |_| {
                if action == Action::ChangeLanguage {
                    open(Screen::Language, title.clone());
                } else {
                    act(action);
                }
            });
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
pub fn row_button(words: &str) -> gtk::Button {
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
