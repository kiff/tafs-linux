//! What stands where an approval's button would while this computer can't approve
//! (`ApprovalSetupView.swift` › `ApprovalGate`, `ApprovalNotReady`). It never can: it has no
//! Face ID or Touch ID (`tafs_core::approval::THIS_COMPUTER`), so signed in with KIFF the
//! words stand in the button's place; the sample, which signs nothing, keeps the button, and its
//! tap says it isn't in the sample.

use gtk::prelude::*;
use tafs_core::approval::{self, Gate, NotReady};
use tafs_core::security_key::words;
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
/// `slot` says it can't approve here. In the sample, which signs nothing, `slot` says it isn't in
/// the sample (`sample.not_in_sample`).
///
/// With this computer's security key on (`key_approval`), signed in with a session that approves
/// with it, the tap asks the key instead, and `slot` says why when KIFF or the key refused.
pub fn refuse(w: &WordBook, mode: Mode, act: &tafs_core::signed::Act, slot: &gtk::Box) {
    refuse_as(w, mode, act, None, slot)
}

/// `refuse`, with the act named as the iPad names it to Face ID (`what`: "Sign the iPad out of
/// your account"): the security key's sheet says it in its lead.
pub fn refuse_as(
    w: &WordBook,
    mode: Mode,
    act: &tafs_core::signed::Act,
    what: Option<String>,
    slot: &gtk::Box,
) {
    refuse_with(w, mode, act, what, vec![], slot)
}

/// `refuse_as`, revoking `cards` at KIFF once the key approved and before the act goes: a taf
/// removed for good (`AppModel.removeTaf`).
pub fn refuse_with(
    w: &WordBook,
    mode: Mode,
    act: &tafs_core::signed::Act,
    what: Option<String>,
    cards: Vec<String>,
    slot: &gtk::Box,
) {
    refuse_then(w, mode, act, what, cards, slot, None)
}

/// `refuse_with`, and `then` once the security key approved and the act went: what's shown is
/// read again (a door reopened, a key revoked, a door removed).
pub fn refuse_then(
    w: &WordBook,
    mode: Mode,
    act: &tafs_core::signed::Act,
    what: Option<String>,
    cards: Vec<String>,
    slot: &gtk::Box,
    then: Option<std::rc::Rc<dyn Fn()>>,
) {
    while let Some(c) = slot.first_child() {
        slot.remove(&c);
    }
    if mode == Mode::Kiff {
        let kept = crate::store::load(chrono::Utc::now().timestamp());
        // Approved with the key (the act's own request or answer), or, for Disconnect and Clear,
        // the key's touch and PIN and then the device key at planque (the owner's rule: never on the
        // device key alone).
        let approves = crate::key_approval::can_approve(kept.as_ref(), act);
        let ends = crate::key_approval::can_end(kept.as_ref(), act);
        if let (true, Some(kept)) = (approves || ends, kept) {
            let slot2 = slot.clone();
            let languages =
                tafs_core::language::screen_languages(&tafs_core::language::preferred_from_env());
            let done: crate::key_approval::Approved =
                std::rc::Rc::new(move |outcome| match outcome {
                    Some(Err(why)) => {
                        slot2.append(&text(&why, 13, 400, &["tafs-no"]));
                        slot2.set_visible(true);
                    }
                    Some(Ok(())) => {
                        slot2.set_visible(false);
                        if let Some(then) = &then {
                            then();
                        }
                    }
                    None => slot2.set_visible(false),
                });
            let what = what.or_else(|| act.own_prompt(w));
            let w = std::rc::Rc::new(w.clone());
            if approves {
                crate::key_approval::approve(
                    w,
                    slot,
                    kept,
                    languages,
                    act,
                    what.as_deref(),
                    cards,
                    done,
                );
            } else {
                crate::key_approval::end_with_presence(
                    w,
                    slot,
                    kept,
                    languages,
                    act,
                    what.as_deref(),
                    done,
                );
            }
            return;
        }
    }
    match tafs_core::signed::tap(w, mode, act) {
        Ok(Some(n)) => {
            slot.append(&not_ready(&n));
            // With the key on, signed in with KIFF: no key with this computer says what one is,
            // and wherever this computer can't approve (no key, or an act only the phone does),
            // "Approve on your phone instead". Nothing is handed over: the act
            // is done again on the phone, and the button puts this away.
            if tafs_core::passkey::ENABLED && mode == Mode::Kiff {
                let kept = crate::store::load(chrono::Utc::now().timestamp());
                let has_key = kept.as_ref().is_some_and(|k| k.security_key.is_some());
                if let (false, Ok(line)) = (has_key, words::none_paired(w)) {
                    slot.append(&text(&line, 12, 400, &["tafs-dim"]));
                }
                if let Ok(label) = words::on_phone(w) {
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                    let on_phone = outline(&label);
                    on_phone.set_hexpand(false);
                    on_phone.set_widget_name("approval.on-phone");
                    let s = slot.clone();
                    on_phone.connect_clicked(move |_| s.set_visible(false));
                    row.append(&on_phone);
                    slot.append(&row);
                }
            }
            slot.set_visible(true);
            // Measured as it was added, a wrapped line can keep a one-line height: every part of
            // it is measured again once drawn (an idle came too soon in a door's sheet), so the
            // sheet or screen around it makes room.
            let slot = slot.clone();
            gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(150), move || {
                fn resize(w: &gtk::Widget) {
                    w.queue_resize();
                    let mut c = w.first_child();
                    while let Some(child) = c {
                        resize(&child);
                        c = child.next_sibling();
                    }
                }
                resize(slot.upcast_ref());
            });
        }
        _ => slot.set_visible(false),
    }
}
