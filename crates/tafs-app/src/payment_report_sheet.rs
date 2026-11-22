//! "Report this payment" (`ReportPaymentSheet.swift`): the sheet a payment's receipt opens when
//! tafs cloud offers it. Its title, what reporting does with the entry's own taf, amount and date,
//! "Report and halt {taf}" and Cancel. One tap reports it: KIFF marks its receipt and tafs cloud
//! halts the taf, as the halt button does. No note, no reason and no signature, since it only
//! takes authority away; resuming the taf stays behind the phone.
//!
//! Once tafs cloud answered: "Reported. {taf} is halted…" and Done, the receipt's button gone and
//! the taf's halt row showing its answer. Anything else: "Couldn't send the report. Nothing
//! changed." and the sheet stays to try again. Signed out of KIFF (a screenshot's) nothing is sent.

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::halt::Halt;
use tafs_core::payment_report::{self, Lines};
use tafs_core::record::Entry;
use tafs_core::words::WordBook;

use crate::receipt_sheet::ReceiptSource;
use crate::widgets::{outline, primary, text};

/// Heard once tafs cloud answered: the taf's halt as the answer gave it.
pub type Reported = Rc<dyn Fn(Halt)>;

/// What sending came to: the halt, or the sheet says it failed. With no session nothing is sent.
fn send(send_as: Option<(String, Vec<&'static str>)>, trace: &str) -> Option<Halt> {
    let (token, languages) = send_as?;
    let a = crate::kiff::report_payment(&token, &languages, trace).ok()?;
    if !(200..300).contains(&a.status) {
        return None;
    }
    payment_report::read(&a.body).map(|r| r.halt)
}

/// The sheet over `parent` for `e`, whose taf is `taf_name`.
pub fn present(
    w: Rc<WordBook>,
    source: Option<Rc<ReceiptSource>>,
    e: Entry,
    taf_name: Option<String>,
    parent: &impl IsA<gtk::Widget>,
    reported: Reported,
) {
    let (Some(trace), Ok(l)) = (
        e.report.trace_id.clone(),
        payment_report::lines(&w, &e, taf_name.as_deref()),
    ) else {
        return;
    };
    let Lines {
        title,
        intro,
        submit,
        cancel,
        failed,
        done,
        done_button,
        ..
    } = l;
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(20);
    column.set_margin_bottom(20);
    column.append(&text(&title, 30, 700, &["tafs-ink"]));
    let said = text(&intro, 16, 400, &["tafs-ink"]);
    column.append(&said);
    let problem = text(&failed, 14, 600, &["tafs-no"]);
    problem.set_visible(false);
    column.append(&problem);
    let report = primary(&submit);
    column.append(&report);
    let close = outline(&cancel);
    column.append(&close);

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    // As tall as what it says, as a sheet sized to its content.
    scroller.set_propagate_natural_height(true);
    crate::widgets::ground(&scroller);
    let dialog = adw::Dialog::new();
    dialog.set_child(Some(&scroller));
    dialog.set_content_width(480);
    dialog.set_title(&title);
    {
        let dialog = dialog.clone();
        close.connect_clicked(move |_| {
            dialog.close();
        });
    }
    let sending = Rc::new(Cell::new(false));
    {
        let (said, problem, report2, close, sending) = (
            said.clone(),
            problem.clone(),
            report.clone(),
            close.clone(),
            sending.clone(),
        );
        report.connect_clicked(move |_| {
            if sending.replace(true) {
                return;
            }
            report2.set_sensitive(false);
            problem.set_visible(false);
            // What crosses to the thread: the session's token and languages, none for a
            // screenshot's.
            let send_as = source
                .as_deref()
                .filter(|s| s.canned.is_none() && !s.token.is_empty())
                .map(|s| (s.token.clone(), s.languages.clone()));
            let trace = trace.clone();
            let (said, problem, report2, close, sending, reported, done, done_button) = (
                said.clone(),
                problem.clone(),
                report2.clone(),
                close.clone(),
                sending.clone(),
                reported.clone(),
                done.clone(),
                done_button.clone(),
            );
            glib::spawn_future_local(async move {
                let answer = gio::spawn_blocking(move || send(send_as, &trace))
                    .await
                    .ok()
                    .flatten();
                sending.set(false);
                match answer {
                    Some(halt) => {
                        said.set_label(&done);
                        report2.set_visible(false);
                        close.set_label(&done_button);
                        close.remove_css_class("tafs-outline");
                        close.add_css_class("tafs-primary");
                        reported(halt);
                    }
                    None => {
                        problem.set_visible(true);
                        report2.set_sensitive(true);
                    }
                }
            });
        });
    }
    dialog.present(Some(parent));
}
