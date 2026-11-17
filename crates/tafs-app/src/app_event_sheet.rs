//! An app event's filters before its subscription is signed (`AppEventApprovalSheet`): the
//! stop's label (20 bold) and words (15, soft), "Listen only for events matching these
//! filters." (14), and the filters whole (13, monospaced), under the bar's "Approve event" and
//! Cancel. Approve is `ApprovalGate`-wrapped on the iPad, so here, as there while a device can't
//! approve, the gate's line stands in its place: nothing is signed, nothing subscribed.
//!
//! It opens from a taf's Setup › Starts when (the iPad's `StartsWhenView`) and the dial
//! (`TafDialPanel`), wherever they're drawn; `--app-event` opens it for its screenshot.

use std::rc::Rc;

use adw::prelude::*;
use tafs_core::app_event::{filters, Item};
use tafs_core::sidebar::Mode;
use tafs_core::words::WordBook;

use crate::widgets::text;

/// The sheet over `over`, for `item`.
pub fn present(w: &Rc<WordBook>, mode: Mode, item: &Item, over: &impl IsA<gtk::Widget>) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let sheet = crate::sheet::new(&l("knob.approve_event"), &l("common.cancel"), None, 480);
    sheet.column.set_spacing(14);
    sheet.column.set_margin_start(20);
    sheet.column.set_margin_end(20);
    sheet.column.set_margin_top(20);
    sheet
        .column
        .append(&text(&item.label, 20, 700, &["tafs-ink"]));
    sheet
        .column
        .append(&text(&item.words, 15, 400, &["tafs-soft"]));
    sheet.column.append(&text(
        &l("knob.listen_only_events_matching_filters"),
        14,
        400,
        &["tafs-ink"],
    ));
    let f = text(&filters(item), 13, 400, &["tafs-ink", "tafs-mono"]);
    f.set_widget_name("event.filters");
    f.set_selectable(true);
    sheet.column.append(&f);
    // `ApprovalGate { Approve with … }`: this computer can't approve, so its line, not the button.
    let approve = crate::widgets::primary(&l("approval.approve"));
    approve.set_widget_name("event.approve");
    sheet
        .column
        .append(&crate::approval::gate(w, mode, &approve));
    sheet.dialog.present(Some(over));
}
