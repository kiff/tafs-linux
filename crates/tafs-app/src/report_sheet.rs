//! Report this answer (`ReportAnswerSheet`), from an answer's menu in a taf's chat or in Coach's: a
//! reason, an optional note, and Send, at tafs cloud as the session, unsigned. Sent, it says a person
//! at KIFF will read it; not sent, it says why and stays open to send again. The words and the
//! request are `tafs-core/src/report.rs`.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;
use tafs_core::report::{self, Outcome, Reason, Reported};
use tafs_core::words::WordBook;

use crate::widgets::{primary, symbol, text};

/// Where a report goes: tafs cloud as the session (`signed_out` when it says the session is over),
/// or nowhere, for a screenshot's chat read from a file.
#[derive(Clone)]
pub enum ReportTo {
    Cloud {
        token: String,
        languages: Vec<&'static str>,
        signed_out: Rc<dyn Fn()>,
    },
    Nowhere,
}

/// The sheet over `over`'s window, for an answer of the taf `taf` (Coach's own taf for Coach's).
pub fn open(w: Rc<WordBook>, over: &gtk::Widget, taf: String, answer: Reported, to: ReportTo) {
    let Ok(lines) = report::sheet(&w) else {
        return;
    };
    let reason: Rc<Cell<Option<Reason>>> = Rc::new(Cell::new(None));
    let sending = Rc::new(Cell::new(false));

    let dialog = adw::Dialog::new();
    dialog.set_content_width(480);
    dialog.set_title(&lines.title);
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    let cancel = gtk::Button::with_label(&lines.cancel);
    cancel.add_css_class("tafs-unsent-act");
    cancel.set_margin_start(8);
    header.pack_start(&cancel);
    let send = gtk::Button::with_label(&lines.send);
    send.add_css_class("tafs-unsent-act");
    send.add_css_class("strong");
    send.set_widget_name("report-send");
    send.set_margin_end(8);
    header.pack_end(&send);

    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(20);
    column.append(&text(&lines.what_wrong, 13, 400, &["tafs-dim"]));
    let reasons = gtk::ListBox::new();
    reasons.add_css_class("boxed-list");
    reasons.set_selection_mode(gtk::SelectionMode::None);
    let marks: Rc<RefCell<Vec<(Reason, gtk::Image)>>> = Rc::new(RefCell::new(Vec::new()));
    for (r, title) in &lines.reasons {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_margin_start(14);
        row.set_margin_end(14);
        row.set_margin_top(12);
        row.set_margin_bottom(12);
        let l = text(title, 15, 400, &["tafs-ink"]);
        l.set_hexpand(true);
        row.append(&l);
        let mark = symbol("tafs-checkmark-symbolic", 14);
        mark.add_css_class("tafs-amber-text");
        mark.set_visible(false);
        row.append(&mark);
        let list_row = gtk::ListBoxRow::new();
        list_row.set_child(Some(&row));
        list_row.set_activatable(true);
        list_row.set_widget_name(&format!("report-reason-{}", r.raw()));
        list_row.update_property(&[gtk::accessible::Property::Label(title)]);
        reasons.append(&list_row);
        marks.borrow_mut().push((*r, mark));
    }
    column.append(&reasons);

    let note = gtk::TextView::new();
    note.add_css_class("tafs-composer-text");
    note.set_wrap_mode(gtk::WrapMode::WordChar);
    note.set_accepts_tab(false);
    note.set_top_margin(10);
    note.set_bottom_margin(10);
    note.set_left_margin(12);
    note.set_right_margin(12);
    note.set_widget_name("report-note");
    note.update_property(&[gtk::accessible::Property::Placeholder(&lines.note_hint)]);
    let hint = text(&lines.note_hint, 15, 400, &["tafs-dim"]);
    hint.set_can_target(false);
    hint.set_halign(gtk::Align::Start);
    hint.set_valign(gtk::Align::Start);
    hint.set_margin_start(12);
    hint.set_margin_top(10);
    hint.set_accessible_role(gtk::AccessibleRole::Presentation);
    let note_scroll = gtk::ScrolledWindow::new();
    note_scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    note_scroll.set_min_content_height(88);
    note_scroll.set_max_content_height(200);
    note_scroll.set_propagate_natural_height(true);
    note_scroll.set_child(Some(&note));
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&note_scroll));
    overlay.add_overlay(&hint);
    let field = gtk::Box::new(gtk::Orientation::Vertical, 0);
    field.add_css_class("tafs-composer");
    field.set_margin_top(8);
    field.append(&overlay);
    column.append(&field);
    let too_long = text(&lines.note_too_long, 13, 400, &["tafs-no"]);
    too_long.set_visible(false);
    column.append(&too_long);
    column.append(&text(&lines.footer, 13, 400, &["tafs-dim"]));
    let failed = text("", 14, 400, &["tafs-no"]);
    failed.set_widget_name("report-failed");
    failed.set_visible(false);
    column.append(&failed);

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_propagate_natural_height(true);
    scroller.set_max_content_height(560);
    scroller.set_child(Some(&column));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    dialog.set_child(Some(&toolbar));

    let words_of = |v: &gtk::TextView| {
        let b = v.buffer();
        b.text(&b.start_iter(), &b.end_iter(), false).to_string()
    };
    // Send once a reason is chosen, nothing is on its way, and the note fits.
    let refresh: Rc<dyn Fn()> = {
        let (send, note, hint, too_long, reason, sending, marks) = (
            send.clone(),
            note.clone(),
            hint.clone(),
            too_long.clone(),
            reason.clone(),
            sending.clone(),
            marks.clone(),
        );
        Rc::new(move || {
            let words = words_of(&note);
            hint.set_visible(words.is_empty());
            too_long.set_visible(words.chars().count() > report::NOTE_LIMIT);
            send.set_sensitive(report::can_send(reason.get(), sending.get(), &words));
            for (r, mark) in marks.borrow().iter() {
                mark.set_visible(reason.get() == Some(*r));
            }
        })
    };
    refresh();
    {
        let (reason, refresh) = (reason.clone(), refresh.clone());
        reasons.connect_row_activated(move |_, row| {
            if let Some(r) = Reason::ALL.get(row.index() as usize) {
                reason.set(Some(*r));
                refresh();
            }
        });
    }
    {
        let refresh = refresh.clone();
        note.buffer().connect_changed(move |_| refresh());
    }
    {
        let d = dialog.downgrade();
        cancel.connect_clicked(move |_| {
            if let Some(d) = d.upgrade() {
                d.close();
            }
        });
    }
    {
        let (w, d, toolbar, header, failed, refresh) = (
            w.clone(),
            dialog.downgrade(),
            toolbar.clone(),
            header.clone(),
            failed.clone(),
            refresh.clone(),
        );
        let done_words = lines.done.clone();
        send.connect_clicked(move |_| {
            let Some(r) = reason.get() else { return };
            if sending.get() {
                return;
            }
            let ReportTo::Cloud {
                token,
                languages,
                signed_out,
            } = to.clone()
            else {
                // A screenshot's chat sends nothing.
                return;
            };
            sending.set(true);
            failed.set_visible(false);
            refresh();
            let body = report::body(r, &words_of(&note), Some(&answer.session));
            let path = report::path(&taf, &answer.message);
            let (w, d, toolbar, header, failed, refresh, sending, done_words) = (
                w.clone(),
                d.clone(),
                toolbar.clone(),
                header.clone(),
                failed.clone(),
                refresh.clone(),
                sending.clone(),
                done_words.clone(),
            );
            glib::spawn_future_local(async move {
                let sent = crate::chat::post(&w, &token, &languages, path, body).await;
                sending.set(false);
                let outcome = match sent {
                    Ok(a) if a.status == 401 => {
                        if let Some(d) = d.upgrade() {
                            d.close();
                        }
                        return signed_out();
                    }
                    Ok(a) => report::outcome(&w, a.status, &a.body),
                    Err(_) => report::unreached(&w),
                };
                let Ok(outcome) = outcome else { return };
                let said = report::said(&w, &outcome).unwrap_or_default();
                match outcome {
                    Outcome::Failed(_) => {
                        failed.set_label(&said);
                        failed.set_visible(true);
                        refresh();
                    }
                    Outcome::Sent | Outcome::Already => {
                        toolbar.set_content(Some(&thanks(&said, &done_words, &d)));
                        header.set_visible(false);
                    }
                }
            });
        });
    }
    dialog.present(Some(over));
}

/// Done: the mark, what the sheet says, and Done.
fn thanks(said: &str, done: &str, dialog: &glib::WeakRef<adw::Dialog>) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 18);
    v.set_margin_start(24);
    v.set_margin_end(24);
    v.set_margin_top(48);
    v.set_margin_bottom(24);
    let mark = symbol("emblem-ok-symbolic", 40);
    mark.add_css_class("tafs-amber-text");
    v.append(&mark);
    let l = text(said, 17, 600, &["tafs-ink"]);
    l.set_xalign(0.5);
    l.set_justify(gtk::Justification::Center);
    l.set_widget_name("report-done");
    v.append(&l);
    let b = primary(done);
    b.set_margin_top(24);
    let d = dialog.clone();
    b.connect_clicked(move |_| {
        if let Some(d) = d.upgrade() {
            d.close();
        }
    });
    v.append(&b);
    v
}
