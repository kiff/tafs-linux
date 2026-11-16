//! What stands where an approval's button would while this computer can't approve
//! (`ApprovalSetupView.swift` › `ApprovalGate`, `ApprovalNotReady`). It never can: it has no
//! Face ID or Touch ID (`tafs_core::approval::THIS_COMPUTER`), so signed in with KIFF the
//! words stand in the button's place; the sample, which signs nothing, keeps the button.

use gtk::prelude::*;
use tafs_core::approval::{self, Gate, NotReady};
use tafs_core::sidebar::Mode;
use tafs_core::words::WordBook;

use crate::widgets::{outline, text};

/// `ApprovalNotReady`: the state's line (14, ink), the new face or fingerprint note (12, dim)
/// where it applies, and Check again unless the device can never approve. Open Settings isn't
/// drawn: it opens tafs' own iOS settings, which this computer has none of. This computer's
/// state has neither, so in practice it's the line alone.
pub fn not_ready(n: &NotReady) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 8);
    b.set_hexpand(true);
    let line = text(&n.line, 14, 400, &["tafs-ink"]);
    line.set_widget_name("approvalNotReady");
    b.append(&line);
    if let Some(note) = &n.note {
        b.append(&text(note, 12, 400, &["tafs-dim"]));
    }
    if let Some(again) = &n.check_again {
        // Reading this computer again can't change it: there, and nothing more.
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        row.append(&outline(again));
        b.append(&row);
    }
    b
}

/// `ApprovalGate`: the button, or while this device can't approve, why in its place.
pub fn gate(w: &WordBook, mode: Mode, button: &impl IsA<gtk::Widget>) -> gtk::Widget {
    match approval::gate(w, mode, approval::THIS_COMPUTER) {
        Ok(Gate::NotReady(n)) => not_ready(&n).upcast(),
        _ => button.clone().upcast(),
    }
}

/// Where the can't-approve words go once a tap would have signed: hidden until then.
pub fn refusal_slot() -> gtk::Box {
    let slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
    slot.set_widget_name("cantSign");
    slot.set_visible(false);
    slot
}

/// The tap that would sign `act` (`tafs_core::signed::tap`): this computer sends nothing, and
/// `slot` says it can't approve here. In the sample, which signs nothing, the tap does nothing.
pub fn refuse(w: &WordBook, mode: Mode, act: &tafs_core::signed::Act, slot: &gtk::Box) {
    while let Some(c) = slot.first_child() {
        slot.remove(&c);
    }
    match tafs_core::signed::tap(w, mode, act) {
        Ok(Some(n)) => {
            slot.append(&not_ready(&n));
            slot.set_visible(true);
        }
        _ => slot.set_visible(false),
    }
}
