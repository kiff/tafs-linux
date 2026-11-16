//! The security key's PIN sheet, for each approval this computer makes with its key (tafs-macos
//! `Sources/App/approvals/PINSheet.swift`, kiff/tafs `docs/computer-passkeys.md`): its title, what
//! it asks, the tries left when the key says, the PIN field, then "Touch your security key." while
//! the key waits; each failure in its own shared line (`security_key::words`), and the sheet stays
//! open to try again, but for a locked key. Switched off with the rest (`passkey::ENABLED`): only
//! `--page pin-sheet` shows it, and it asks no key there.
//!
//! **The PIN** is taken from the field as UTF-8 bytes and the field emptied at once. The bytes
//! become one `security_key::Pin`, which clears them, is handed to the key once, then cleared.
//! What's left, as on macOS 12: GTK's own entry held the characters while they were typed, and
//! its text is copied once to be read; this app can't zero that memory, and empties the field.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::security_key::{words, Failure, Pin};
use tafs_core::words::WordBook;

use crate::fido_key::KeyError;
use crate::widgets::{eyebrow, outline, primary, text};

/// Asks the key with the PIN; `Ok` once the key approved.
pub type Attempt = Arc<dyn Fn(&mut Pin) -> Result<(), KeyError> + Send + Sync>;
/// Reads the PIN tries left, before the PIN is asked.
pub type ReadTries = Arc<dyn Fn() -> Result<i32, KeyError> + Send + Sync>;
/// Hears once: none once approved, or why the sheet closed.
pub type Done = Rc<dyn Fn(Option<KeyError>)>;
/// Stops the request at the key (`OpenKey::cancel`, as macOS 12's sheet does), from the main
/// loop while `Attempt` waits on its thread.
pub type StopKey = Arc<dyn Fn() + Send + Sync>;

/// The sheet closing (Cancel, Escape, or its window closed): while the key waits for its touch,
/// the request is stopped at the key, so a touch after the sheet said "cancelled" signs nothing.
/// The attempt then answers `Cancelled`, and its caller sends nothing.
struct Closing {
    touching: Rc<Cell<bool>>,
    stop: StopKey,
}

impl Closing {
    fn closed(&self) {
        if self.touching.get() {
            (self.stop)();
        }
    }
}

/// The line for why the key gave no approval, and whether the sheet stays open. A cancel says
/// nothing; anything that isn't the key's own failure is the key failing to sign.
fn said(w: &WordBook, e: KeyError) -> Option<(String, bool)> {
    match e {
        KeyError::Cancelled => None,
        KeyError::Key(f) => Some((
            words::line(w, f).unwrap_or_default(),
            words::may_try_again(Some(f)),
        )),
        KeyError::Missing(_) | KeyError::Unreadable => {
            Some((words::failed(w).unwrap_or_default(), true))
        }
    }
}

/// The sheet over `parent`.
pub fn present(
    w: Rc<WordBook>,
    parent: &impl IsA<gtk::Widget>,
    attempt: Attempt,
    read_tries: ReadTries,
    stop: StopKey,
    done: Done,
) {
    let Ok(l) = words::sheet(&w) else {
        return;
    };
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(20);
    column.set_margin_bottom(20);
    let title = text(&l.title, 15, 600, &["tafs-ink"]);
    title.set_xalign(0.5);
    column.append(&title);
    column.append(&text(&l.enter_pin, 14, 400, &["tafs-soft"]));
    let tries = text("", 13, 400, &["tafs-dim"]);
    tries.set_visible(false);
    column.append(&tries);
    column.append(&eyebrow(&l.pin, false));
    let field = gtk::PasswordEntry::new();
    field.set_placeholder_text(Some(&l.pin));
    field.set_widget_name("security-key-pin");
    column.append(&field);
    let touch = text(&l.touch, 14, 600, &["tafs-amber-text"]);
    touch.set_visible(false);
    column.append(&touch);
    let message = text("", 13, 400, &["tafs-no"]);
    message.set_visible(false);
    column.append(&message);
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    buttons.set_halign(gtk::Align::End);
    let cancel = outline(&l.cancel);
    cancel.set_hexpand(false);
    let approve = primary(&l.approve);
    approve.set_hexpand(false);
    buttons.append(&cancel);
    buttons.append(&approve);
    column.append(&buttons);
    // The ground fills the whole sheet, its margins included.
    let sheet = gtk::Box::new(gtk::Orientation::Vertical, 0);
    sheet.append(&column);
    crate::widgets::ground(&sheet);

    let dialog = adw::Dialog::new();
    dialog.set_child(Some(&sheet));
    dialog.set_content_width(440);
    dialog.set_title(&l.title);
    dialog.set_default_widget(Some(&approve));
    field.set_activates_default(true);

    // Heard once: the first of approved, cancelled or closed.
    let done: Rc<RefCell<Option<Done>>> = Rc::new(RefCell::new(Some(done)));
    let finish = {
        let done = done.clone();
        Rc::new(move |e: Option<KeyError>| {
            if let Some(d) = done.borrow_mut().take() {
                d(e);
            }
        })
    };
    let locked = Rc::new(Cell::new(false));
    let touching = Rc::new(Cell::new(false));
    let show = {
        let (w, message, locked, field, approve) = (
            w.clone(),
            message.clone(),
            locked.clone(),
            field.clone(),
            approve.clone(),
        );
        Rc::new(move |e: KeyError| {
            let Some((line, again)) = said(&w, e) else {
                return;
            };
            message.set_label(&line);
            message.set_visible(true);
            if !again {
                locked.set(true);
                field.set_sensitive(false);
                approve.set_sensitive(false);
            }
        })
    };

    // The tries left, as the key says them; a key that can't be read now says nothing yet.
    {
        let (w, tries, show, read_tries) =
            (w.clone(), tries.clone(), show.clone(), read_tries.clone());
        glib::spawn_future_local(async move {
            let read = gio::spawn_blocking(move || read_tries())
                .await
                .unwrap_or(Err(KeyError::Unreadable));
            match read {
                Ok(n) => {
                    tries.set_label(&words::tries_left(&w, n).unwrap_or_default());
                    tries.set_visible(true);
                }
                Err(KeyError::Key(Failure::PinBlocked)) => show(KeyError::Key(Failure::PinBlocked)),
                Err(_) => {}
            }
        });
    }

    // Takes the typed PIN and asks the key with it.
    {
        let (field, touch, message, show, finish, dialog, locked, touching, approve2) = (
            field.clone(),
            touch.clone(),
            message.clone(),
            show.clone(),
            finish.clone(),
            dialog.clone(),
            locked.clone(),
            touching.clone(),
            approve.clone(),
        );
        approve.connect_clicked(move |_| {
            if touching.get() || locked.get() {
                return;
            }
            let mut bytes = field.text().as_bytes().to_vec();
            field.set_text("");
            let mut pin = match Pin::new(&mut bytes) {
                Ok(p) => p,
                Err(f) => return show(KeyError::Key(f)),
            };
            message.set_visible(false);
            touching.set(true);
            touch.set_visible(true);
            field.set_sensitive(false);
            approve2.set_sensitive(false);
            let attempt = attempt.clone();
            let (field, touch, show, finish, dialog, touching, approve2, locked) = (
                field.clone(),
                touch.clone(),
                show.clone(),
                finish.clone(),
                dialog.clone(),
                touching.clone(),
                approve2.clone(),
                locked.clone(),
            );
            glib::spawn_future_local(async move {
                // Handed to the key once; cleared whatever the key answered.
                let answer = gio::spawn_blocking(move || {
                    let a = attempt(&mut pin);
                    pin.release();
                    a
                })
                .await
                .unwrap_or(Err(KeyError::Unreadable));
                touching.set(false);
                touch.set_visible(false);
                match answer {
                    Ok(()) => {
                        finish(None);
                        dialog.close();
                    }
                    Err(e) => {
                        if !locked.get() {
                            field.set_sensitive(true);
                            approve2.set_sensitive(true);
                        }
                        show(e);
                        field.grab_focus();
                    }
                }
            });
        });
    }

    // Cancel, or the sheet closed: nothing is approved, and the field is emptied.
    {
        let (dialog, field) = (dialog.clone(), field.clone());
        cancel.connect_clicked(move |_| {
            field.set_text("");
            dialog.close();
        });
    }
    {
        let (field, finish) = (field.clone(), finish.clone());
        let closing = Closing {
            touching: touching.clone(),
            stop,
        };
        dialog.connect_closed(move |_| {
            field.set_text("");
            closing.closed();
            finish(Some(KeyError::Cancelled));
        });
    }
    dialog.present(Some(parent));
    field.grab_focus();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Condvar, Mutex};

    /// A stand-in attempt that waits for the key's touch until it's stopped: closing the sheet
    /// while it waits stops it, and it answers `Cancelled`, never an approval.
    #[test]
    fn closing_while_the_key_waits_stops_it_at_the_key() {
        let stopped = Arc::new((Mutex::new(false), Condvar::new()));
        let s = stopped.clone();
        let attempt = std::thread::spawn(move || {
            let (lock, wake) = &*s;
            let mut stop = lock.lock().unwrap();
            while !*stop {
                let (next, timeout) = wake
                    .wait_timeout(stop, std::time::Duration::from_secs(5))
                    .unwrap();
                stop = next;
                if timeout.timed_out() {
                    // The touch came: without the stop, the key would sign.
                    return Ok::<(), KeyError>(());
                }
            }
            Err(KeyError::Cancelled)
        });
        let s = stopped.clone();
        let closing = Closing {
            touching: Rc::new(Cell::new(true)),
            stop: Arc::new(move || {
                let (lock, wake) = &*s;
                *lock.lock().unwrap() = true;
                wake.notify_all();
            }),
        };
        closing.closed();
        assert_eq!(attempt.join().unwrap(), Err(KeyError::Cancelled));
    }

    /// Closed before any PIN went to the key: there's nothing to stop.
    #[test]
    fn closing_before_the_key_is_asked_stops_nothing() {
        let calls = Arc::new(Mutex::new(0));
        let c = calls.clone();
        let closing = Closing {
            touching: Rc::new(Cell::new(false)),
            stop: Arc::new(move || *c.lock().unwrap() += 1),
        };
        closing.closed();
        assert_eq!(*calls.lock().unwrap(), 0);
    }
}
