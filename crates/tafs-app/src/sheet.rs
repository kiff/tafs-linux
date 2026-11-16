//! A sheet over the window, as the iPad's `.sheet` with a `NavigationStack`: its title in the bar,
//! Cancel at the start, the confirming button (when it has one) at the end, and its content under
//! them, scrolled.

use adw::prelude::*;

pub struct Sheet {
    pub dialog: adw::Dialog,
    /// The confirming button at the bar's end (Create, Save).
    pub confirm: Option<gtk::Button>,
    /// Where the sheet's content goes.
    pub column: gtk::Box,
}

pub fn new(title: &str, cancel: &str, confirm: Option<&str>, width: i32) -> Sheet {
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    let cancel = gtk::Button::with_label(cancel);
    cancel.add_css_class("tafs-unsent-act");
    cancel.set_margin_start(8);
    header.pack_start(&cancel);
    let confirm = confirm.map(|words| {
        let b = gtk::Button::with_label(words);
        b.add_css_class("tafs-unsent-act");
        b.add_css_class("strong");
        b.set_margin_end(8);
        header.pack_end(&b);
        b
    });
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(20);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_propagate_natural_height(true);
    scroller.set_max_content_height(560);
    scroller.set_child(Some(&column));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    // It grows when a line comes under its content, as the can't-approve words do, at its width.
    dialog.set_follows_content_size(true);
    toolbar.set_size_request(width, -1);
    dialog.set_title(title);
    dialog.set_child(Some(&toolbar));
    {
        let d = dialog.downgrade();
        cancel.connect_clicked(move |_| {
            if let Some(d) = d.upgrade() {
                d.close();
            }
        });
    }
    Sheet {
        dialog,
        confirm,
        column,
    }
}

/// An inset group's header over it (13, dim, upper case), as the iPad's list draws a section's.
pub fn header(words: &str) -> gtk::Label {
    let h = crate::widgets::text(&words.to_uppercase(), 13, 400, &["tafs-dim"]);
    h.set_margin_start(16);
    h.set_margin_top(8);
    h
}

/// An inset group's footer under it (13, dim).
pub fn footer(words: &str) -> gtk::Label {
    let f = crate::widgets::text(words, 13, 400, &["tafs-dim"]);
    f.set_margin_start(16);
    f.set_margin_end(16);
    f
}
