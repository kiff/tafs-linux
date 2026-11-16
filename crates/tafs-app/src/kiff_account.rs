//! Account › KIFF account (`AccountView`): the account's name, its id when it isn't its name, who
//! is signed in; its time zone, with this computer's to use when it's elsewhere; Export my chats;
//! and Delete account for whoever may. Setting the zone, exporting and deleting are acts this
//! computer can't take yet: there, dimmed. The lines are `tafs-core/src/account.rs`.

use std::rc::Rc;

use adw::prelude::*;
use tafs_core::account::{kiff_account_lines, Account};
use tafs_core::words::WordBook;

use crate::widgets::{one_line, symbol, text};

fn group() -> gtk::Box {
    let g = gtk::Box::new(gtk::Orientation::Vertical, 0);
    g.add_css_class("tafs-group");
    g
}

fn line() -> gtk::Separator {
    let s = gtk::Separator::new(gtk::Orientation::Horizontal);
    s.add_css_class("tafs-line");
    s
}

/// `LabeledContent`: the label, and its value at the end in the soft colour.
fn labeled(label: &str, value: &str) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.set_margin_top(12);
    row.set_margin_bottom(12);
    let l = one_line(label, 17, 400, &["tafs-ink"]);
    l.set_hexpand(true);
    row.append(&l);
    row.append(&one_line(value, 17, 400, &["tafs-soft"]));
    row
}

fn footer(words: &str) -> gtk::Label {
    let f = text(words, 13, 400, &["tafs-dim"]);
    f.set_margin_start(12);
    f.set_margin_end(12);
    f
}

/// A button as a list row, in the amber or in red; dimmed while it can't act.
fn row_button(words: &str, class: &str) -> gtk::Button {
    let b = gtk::Button::with_label(words);
    b.add_css_class("tafs-row-button");
    b.add_css_class(class);
    if let Some(l) = b.child() {
        l.set_halign(gtk::Align::Start);
    }
    b.set_sensitive(false);
    b
}

thread_local! {
    /// `--page delete-account`: the sheet opens once with the page, for its screenshot.
    pub static OPEN_DELETE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// `DeleteAccountView`, its own sheet: the warning (15 semibold), What goes (the tafs, the
/// sign-ins, Cards and keys, KIFF's records, each with its symbol, 14), and where Delete account
/// would be, why this computer can't (`ApprovalGate`): deleting is confirmed by the device, which
/// this computer can't. Cancel at the top.
fn delete_sheet(w: &WordBook, parent: &impl IsA<gtk::Widget>) {
    let Ok(d) = tafs_core::account::delete_lines(w) else {
        return;
    };
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(16);
    column.set_margin_bottom(16);
    let warning = group();
    let t = text(&d.warning, 15, 600, &["tafs-ink"]);
    t.set_margin_start(16);
    t.set_margin_end(16);
    t.set_margin_top(12);
    t.set_margin_bottom(12);
    warning.append(&t);
    column.append(&warning);
    let heading = text(&d.what_goes.to_uppercase(), 13, 400, &["tafs-dim"]);
    heading.set_margin_start(16);
    column.append(&heading);
    let goes = group();
    for (n, (sf, words)) in d.goes.iter().enumerate() {
        if n > 0 {
            goes.append(&line());
        }
        let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        r.set_margin_start(16);
        r.set_margin_end(16);
        r.set_margin_top(11);
        r.set_margin_bottom(11);
        let icon = symbol(crate::widgets::icon_for(sf), 16);
        icon.add_css_class("tafs-amber-text");
        r.append(&icon);
        r.append(&text(words, 14, 400, &["tafs-ink"]));
        goes.append(&r);
    }
    column.append(&goes);
    let delete = gtk::Button::with_label(&d.title);
    column.append(&crate::approval::gate(
        w,
        tafs_core::sidebar::Mode::Kiff,
        &delete,
    ));

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    let cancel = gtk::Button::with_label(&d.cancel);
    cancel.add_css_class("flat");
    header.pack_start(&cancel);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(560);
    dialog.set_content_height(520);
    dialog.set_title(&d.title);
    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog.close();
        });
    }
    dialog.present(Some(parent));
}

pub fn page(
    w: Rc<WordBook>,
    account: Option<Account>,
    back: Rc<dyn Fn()>,
    open_zone: Rc<dyn Fn()>,
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
    let title = text(&l("settings.kiff_account"), 34, 700, &["tafs-ink"]);
    title.set_width_chars(12);
    column.append(&title);
    if let Some(account) = &account {
        let here = tafs_core::account_time::local_identifier();
        if let Ok(lines) = kiff_account_lines(&w, account, &here) {
            let g = group();
            for (i, (label, value)) in lines.rows.iter().enumerate() {
                if i > 0 {
                    g.append(&line());
                }
                g.append(&labeled(label, value));
            }
            column.append(&g);
            // A Card's day ends at midnight here, and every time the person reads is in it.
            let g = group();
            // The zone opens its picker (`NavigationLink`), with a chevron.
            let zone_row = labeled(&l("settings.time_zone_title"), &lines.zone);
            let chevron = symbol("go-next-symbolic", 13);
            chevron.add_css_class("tafs-dim");
            zone_row.append(&chevron);
            let zone_button = gtk::Button::new();
            zone_button.set_child(Some(&zone_row));
            zone_button.add_css_class("tafs-plain");
            zone_button.connect_clicked(move |_| open_zone());
            g.append(&zone_button);
            if let Some(use_this) = &lines.use_this_zone {
                g.append(&line());
                g.append(&row_button(use_this, "tafs-amber-text"));
            }
            column.append(&g);
            column.append(&footer(&l("settings.time_zone.uses_this")));
            // Their own copy, for their rights of access and portability.
            let g = group();
            g.append(&row_button(&l("export.row"), "tafs-amber-text"));
            column.append(&g);
            column.append(&footer(&l("export.own_chats_each_taf_coach")));
            if account.allows("account.delete") {
                let g = group();
                let delete = row_button(&l("settings.delete_account.row"), "tafs-no");
                // Its sheet opens: what goes, and why this computer can't delete it.
                delete.set_sensitive(true);
                let w = w.clone();
                {
                    let w = w.clone();
                    delete.connect_clicked(move |b| delete_sheet(&w, b));
                }
                if OPEN_DELETE.with(|o| o.replace(false)) {
                    delete.connect_map(move |b| delete_sheet(&w, b));
                }
                g.append(&delete);
                column.append(&g);
                column.append(&footer(&l("sign_in.deletes_every_taf_runs_chats")));
            }
        }
    }
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    crate::widgets::ground(&scroller);
    scroller.upcast()
}
