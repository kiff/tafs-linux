//! Account › Language (`LanguagePicker`): the app's own list. This computer's language first, then
//! the app's languages, each named in itself, the one in use checked. A pick applies at once: the
//! window is drawn again in it, with no relaunch, and it's kept for the next launch. The rows are
//! `tafs-core/src/account.rs` › `language_rows`.

use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::account::{language_rows, Action};
use tafs_core::words::WordBook;

use crate::widgets::{one_line, symbol, text};

/// `chosen`: the language picked in the app, none for this computer's. `act` gets the pick.
pub fn page(
    w: Rc<WordBook>,
    chosen: Option<String>,
    back: Rc<dyn Fn()>,
    act: Rc<dyn Fn(Action)>,
) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let back_button = gtk::Button::new();
    back_button.set_child(Some(&symbol("go-previous-symbolic", 20)));
    back_button.add_css_class("tafs-round-back");
    back_button.set_halign(gtk::Align::Start);
    back_button.update_property(&[gtk::accessible::Property::Label(&l("settings.title"))]);
    back_button.connect_clicked(move |_| back());
    column.append(&back_button);
    column.append(&text(&l("settings.language_title"), 34, 700, &["tafs-ink"]));

    let rows = language_rows(&w, chosen.as_deref()).unwrap_or_default();
    // This computer's language in a group of its own, then the app's languages.
    for group in [&rows[..rows.len().min(1)], &rows[rows.len().min(1)..]] {
        if group.is_empty() {
            continue;
        }
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        list.add_css_class("tafs-group");
        for (i, r) in group.iter().enumerate() {
            if i > 0 {
                let line = gtk::Separator::new(gtk::Orientation::Horizontal);
                line.add_css_class("tafs-line");
                list.append(&line);
            }
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            row.set_margin_top(12);
            row.set_margin_bottom(12);
            let name = one_line(&r.name, 17, 400, &["tafs-ink"]);
            name.set_hexpand(true);
            name.set_xalign(0.0);
            row.append(&name);
            if r.picked {
                let check = symbol("tafs-checkmark-symbolic", 17);
                check.add_css_class("tafs-amber-text");
                row.append(&check);
            }
            let button = gtk::Button::new();
            button.add_css_class("tafs-row-button");
            button.set_child(Some(&row));
            button.set_widget_name(&format!("language.{}", r.code.unwrap_or("device")));
            if r.picked {
                button.update_state(&[gtk::accessible::State::Selected(Some(true))]);
            }
            // The one in use does nothing; any other applies at once.
            let (act, code, picked) = (act.clone(), r.code, r.picked);
            button.connect_clicked(move |_| {
                if !picked {
                    act(Action::UseLanguage(code));
                }
            });
            list.append(&button);
        }
        column.append(&list);
    }

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    scroller.upcast()
}
