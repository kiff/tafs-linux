//! Account › Sign-in (the iPad's `CodeSignInRow`, tafs#1711): whether the person signs in with a code KIFF emails
//! them. Read from KIFF when Account opens (`GET /v1/me/sign-in`), as the session; drawn once read, the section left out
//! when it can't be. Turn off goes from here (`PUT … {"email_code": false}`, unsigned). Turn on needs a recent sign-in,
//! which this computer's session never passes, so it opens KIFF's own sign-in page in the browser, and the setting is
//! read again when this window is in front again. KIFF emails the person about either change.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gio, glib};
use tafs_core::code_sign_in::{self, CodeSignIn, Offer};
use tafs_core::words::WordBook;

use crate::ai_consent::Source;
use crate::widgets::text;

thread_local! {
    /// `--page account-sign-in`: the row's button focused once drawn, so Account scrolls to it.
    pub static FOCUS: Cell<bool> = const { Cell::new(false) };
}

struct Row {
    w: Rc<WordBook>,
    source: Source,
    into: gtk::Box,
    now: RefCell<Option<CodeSignIn>>,
    busy: Cell<bool>,
    /// Turn on opened KIFF's page, and the read waits for this window to be in front again:
    /// a second tap meanwhile opens the page again but waits once, so it's read once.
    waiting: Cell<bool>,
    error: RefCell<Option<String>>,
}

/// KIFF's answer, or why it couldn't be read.
fn get(source: &Source, w: &WordBook) -> Result<CodeSignIn, String> {
    match source {
        Source::Cloud { token, languages } => {
            let a = crate::kiff::kiff_get(token, languages, code_sign_in::PATH)
                .map_err(|e| e.words(w).unwrap_or_default())?;
            if a.status != 200 {
                return Err(tafs_core::signin::refusal(&a, Some(w))
                    .words(w)
                    .unwrap_or_default());
            }
            code_sign_in::read(&a.body)
                .ok_or_else(|| tafs_core::signin::plain_words(w, 200).unwrap_or_default())
        }
        Source::File(path) => crate::ai_consent::file_part(path, "sign_in")
            .and_then(|b| code_sign_in::read(&b))
            .ok_or_else(String::new),
    }
}

/// Whether signing in with a code is available to this account (How tafs works' code section,
/// the iPad's `codeSignIn?.available == true`). A failed read is no.
pub fn available(source: &Source, w: &WordBook) -> bool {
    get(source, w).is_ok_and(|c| c.available)
}

/// Turning it off, as the session: the new setting, or why it didn't change. A screenshot's sends nothing.
fn turn_off(source: &Source, w: &WordBook) -> Result<Option<CodeSignIn>, String> {
    let Source::Cloud { token, languages } = source else {
        return Ok(None);
    };
    let call = tafs_core::signed::Unsigned::new(
        "PUT",
        code_sign_in::PATH.into(),
        code_sign_in::off_body(),
    )
    .ok_or_else(String::new)?;
    let a = crate::kiff::kiff_send(token, languages, &call)
        .map_err(|e| e.words(w).unwrap_or_default())?;
    if !(200..300).contains(&a.status) {
        return Err(tafs_core::signin::refusal(&a, Some(w))
            .words(w)
            .unwrap_or_default());
    }
    Ok(code_sign_in::read(&a.body))
}

/// The slot's row, read from KIFF: shown (`shown`) once read; never when it can't be.
pub fn row(w: Rc<WordBook>, source: Source, into: gtk::Box, shown: Rc<dyn Fn()>) {
    let r = Rc::new(Row {
        w,
        source,
        into,
        now: RefCell::new(None),
        busy: Cell::new(false),
        waiting: Cell::new(false),
        error: RefCell::new(None),
    });
    read(&r, Some(shown));
}

fn read(r: &Rc<Row>, shown: Option<Rc<dyn Fn()>>) {
    let r = r.clone();
    glib::spawn_future_local(async move {
        let (source, words) = (r.source.clone(), (*r.w).clone());
        let got = gio::spawn_blocking(move || get(&source, &words))
            .await
            .unwrap_or_else(|_| Err(String::new()));
        match got {
            Ok(s) => {
                *r.now.borrow_mut() = Some(s);
                *r.error.borrow_mut() = None;
                if let Some(shown) = shown {
                    shown();
                }
            }
            // Read again after Turn on: why, under what's shown. First read: the section stays out.
            Err(why) if r.now.borrow().is_some() => {
                *r.error.borrow_mut() = code_sign_in::not_loaded(&r.w, &why).ok();
            }
            Err(_) => return,
        }
        draw(&r);
    });
}

fn draw(r: &Rc<Row>) {
    while let Some(c) = r.into.first_child() {
        r.into.remove(&c);
    }
    let Some(now) = *r.now.borrow() else { return };
    let Ok(l) = code_sign_in::lines(&r.w, now) else {
        return;
    };
    let value = crate::account::line(&l.label, Some(&l.value), false);
    value.set_widget_name("settings.sign-in-code");
    r.into.append(&value);
    r.into.append(&text(&l.help, 13, 400, &["tafs-soft"]));
    match &l.offer {
        Offer::TurnOff(words) => {
            let b = crate::account::row_button(words);
            b.set_widget_name("settings.sign-in-code.off");
            b.add_css_class("amber-text");
            b.set_sensitive(!r.busy.get());
            let r2 = r.clone();
            b.connect_clicked(move |_| off(&r2));
            r.into.append(&b);
        }
        Offer::TurnOn { button, note } => {
            let b = crate::account::row_button(button);
            b.set_widget_name("settings.sign-in-code.on");
            b.add_css_class("amber-text");
            b.set_sensitive(!r.busy.get());
            let r2 = r.clone();
            b.connect_clicked(move |b| on(&r2, b));
            r.into.append(&b);
            r.into.append(&text(note, 13, 400, &["tafs-soft"]));
        }
        Offer::NotAvailable(words) => r.into.append(&text(words, 13, 400, &["tafs-soft"])),
    }
    if let Some(why) = r.error.borrow().as_deref() {
        r.into.append(&text(why, 13, 400, &["tafs-no"]));
    }
    if FOCUS.with(|f| f.replace(false)) {
        let into = r.into.clone();
        glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
            // Account's scroller brought to the row, its top a little under the window's.
            let Some(scroller) = into
                .ancestor(gtk::ScrolledWindow::static_type())
                .and_downcast::<gtk::ScrolledWindow>()
            else {
                return;
            };
            let Some(child) = scroller.child() else {
                return;
            };
            if let Some(p) = into.compute_point(&child, &gtk::graphene::Point::new(0.0, 0.0)) {
                scroller.vadjustment().set_value(f64::from(p.y()) - 140.0);
            }
        });
    }
}

fn off(r: &Rc<Row>) {
    if r.busy.replace(true) {
        return;
    }
    draw(r);
    let r = r.clone();
    glib::spawn_future_local(async move {
        let (source, words) = (r.source.clone(), (*r.w).clone());
        let sent = gio::spawn_blocking(move || turn_off(&source, &words))
            .await
            .unwrap_or_else(|_| Err(String::new()));
        r.busy.set(false);
        match sent {
            Ok(Some(s)) => {
                *r.now.borrow_mut() = Some(s);
                *r.error.borrow_mut() = None;
            }
            // Taken but not read back: read it again.
            Ok(None) if matches!(r.source, Source::Cloud { .. }) => return read(&r, None),
            Ok(None) => {}
            Err(why) => *r.error.borrow_mut() = code_sign_in::not_changed(&r.w, &why).ok(),
        }
        draw(&r);
    });
}

/// KIFF's own page, in the browser; the setting read again when this window is in front again, whichever way it went.
fn on(r: &Rc<Row>, from: &gtk::Button) {
    let window = from.root().and_downcast::<gtk::Window>();
    let page = code_sign_in::turn_on_page();
    if !tafs_core::signin::is_kiff_link(&page) {
        return;
    }
    gtk::UriLauncher::new(&page).launch(window.as_ref(), gio::Cancellable::NONE, |_| {});
    let Some(window) = window else { return };
    if r.waiting.replace(true) {
        return;
    }
    // Once: the first time the window is active again after it went to the browser.
    let left = Rc::new(Cell::new(false));
    let handler: Rc<RefCell<Option<glib::SignalHandlerId>>> = Rc::default();
    let (r2, h2) = (r.clone(), handler.clone());
    let id = window.connect_is_active_notify(move |win| {
        if !win.is_active() {
            left.set(true);
        } else if left.get() {
            if let Some(id) = h2.borrow_mut().take() {
                win.disconnect(id);
            }
            r2.waiting.set(false);
            read(&r2, None);
        }
    });
    *handler.borrow_mut() = Some(id);
}
