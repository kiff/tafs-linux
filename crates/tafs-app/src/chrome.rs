//! Account › Chrome (`ChromeViews.swift`, kiff/tafs `docs/chrome.md`): the Chromes paired to the account, Unpair, and
//! Pair a Chrome. The list and Unpair go to planque signed by this computer's device key (its safe routes,
//! `planque::Safe`); no session token goes. Account's row shows only once the list reads: planque answers 404 while it
//! doesn't offer Chromes and 403 for someone who may only view, and then there's no row, as the iPad's while `chromes`
//! is nil. Pairing binds a phone's approval key the extension trusts afterwards, so it stays on the phone: Pair a Chrome
//! says this computer can't, with "Approve on your phone instead" (the PM's 20:12).
//!
//! Built to the "Your Chrome" contract (kiff/tafs `docs/chrome.md`, tafs#1744): unpairing a Chrome ends its
//! connections at planque (planque#364), so none is left pointing at a Chrome that's gone. Each Chrome's connections,
//! and naming them in Unpair's question (its rules 3 and 4), follow once planque's connection read names its Chrome
//! and tafs has the words.

use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::chrome::{self, Chrome, Listed};
use tafs_core::planque::Safe;
use tafs_core::session::KeptSession;
use tafs_core::words::WordBook;

use crate::widgets::{one_line, outline, panel, text};

/// Where the Chromes are read.
#[derive(Clone)]
pub enum ChromeSource {
    /// planque, signed by this computer's device key (its id in KIFF read first).
    Cloud {
        kept: KeptSession,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answer (`--chromes`: planque's list). Nothing is sent.
    File(std::path::PathBuf),
    None,
}

/// The list, or why it couldn't be read; `Ok(Listed::Hidden)` when planque doesn't offer it to this person.
fn read(w: &WordBook, source: &ChromeSource) -> Result<Listed, String> {
    let why = |why: String| chrome::not_loaded(w, &why).unwrap_or(why);
    match source {
        ChromeSource::Cloud { kept, languages } => {
            let device = crate::coach::this_device(kept, languages)
                .ok_or_else(|| why(w.line("error.offline").unwrap_or_default()))?;
            let a = crate::kiff::planque_safe(kept, &device, languages, Safe::Chromes)
                .map_err(|e| why(e.words(w).unwrap_or_default()))?;
            chrome::listed(a.status, &a.body).ok_or_else(|| {
                why(tafs_core::signin::refusal(&a, Some(w))
                    .words(w)
                    .unwrap_or_default())
            })
        }
        ChromeSource::File(path) => {
            let v: serde_json::Value = std::fs::read(path)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .ok_or_else(|| why(String::new()))?;
            let body = serde_json::to_vec(&v["answer"]).unwrap_or_default();
            chrome::listed(200, &body).ok_or_else(|| why(String::new()))
        }
        ChromeSource::None => Ok(Listed::Hidden),
    }
}

/// Unpair (`removeChrome`): true once planque took it away.
fn unpair(source: &ChromeSource, id: &str) -> bool {
    let ChromeSource::Cloud { kept, languages } = source else {
        return false;
    };
    let Some(device) = crate::coach::this_device(kept, languages) else {
        return false;
    };
    crate::kiff::planque_safe(kept, &device, languages, Safe::Unpair(id))
        .is_ok_and(|a| (200..300).contains(&a.status))
}

/// Account's Chrome row: hidden until planque lists Chromes for this person.
pub fn show_row_when_offered(w: Rc<WordBook>, source: ChromeSource, row: &gtk::ListBoxRow) {
    row.set_visible(false);
    let row = row.downgrade();
    glib::spawn_future_local(async move {
        let words = (*w).clone();
        let read = gio::spawn_blocking(move || read(&words, &source)).await;
        if let (Ok(Ok(Listed::Chromes(_))), Some(row)) = (read, row.upgrade()) {
            row.set_visible(true);
        }
    });
}

/// `ChromesView`: the Chromes, each with Unpair behind its question; Pair a Chrome; the footers.
pub fn page(w: Rc<WordBook>, source: ChromeSource) -> gtk::Widget {
    let Ok(lines) = chrome::lines(&w) else {
        return gtk::Box::new(gtk::Orientation::Vertical, 0).upcast();
    };
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(16);
    column.set_margin_bottom(16);
    let title = text(&lines.title, 34, 700, &["tafs-ink"]);
    title.set_width_chars(12);
    column.append(&title);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 8);
    column.append(&list);
    column.append(&text(&lines.footer, 12, 400, &["tafs-dim"]));
    // Pair a Chrome: the phone's. This computer says it can't, and offers the phone instead.
    let pair = outline(&lines.pair);
    pair.set_widget_name("chrome.pair");
    pair.set_margin_top(12);
    column.append(&pair);
    let refused = gtk::Box::new(gtk::Orientation::Vertical, 8);
    refused.set_visible(false);
    column.append(&refused);
    {
        let (w, refused) = (w.clone(), refused.clone());
        pair.connect_clicked(move |_| {
            while let Some(c) = refused.first_child() {
                refused.remove(&c);
            }
            if let Ok(n) =
                tafs_core::approval::not_ready(&w, tafs_core::approval::THIS_COMPUTER, None)
            {
                refused.append(&crate::approval::not_ready(&n));
            }
            if let Ok(label) = tafs_core::security_key::words::on_phone(&w) {
                let b = outline(&label);
                b.set_hexpand(false);
                b.set_halign(gtk::Align::Start);
                b.set_widget_name("approval.on-phone");
                let r = refused.clone();
                b.connect_clicked(move |_| r.set_visible(false));
                refused.append(&b);
            }
            refused.set_visible(true);
        });
    }
    column.append(&text(&lines.profile_note, 12, 400, &["tafs-dim"]));

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    crate::widgets::ground(&scroller);

    let load: Rc<dyn Fn()> = {
        let (w, source, list) = (w.clone(), source.clone(), list.clone());
        Rc::new(move || {
            let (w, source, list) = (w.clone(), source.clone(), list.clone());
            glib::spawn_future_local(async move {
                let (words, s2) = ((*w).clone(), source.clone());
                let read = gio::spawn_blocking(move || read(&words, &s2))
                    .await
                    .unwrap_or_else(|_| Err(String::new()));
                while let Some(c) = list.first_child() {
                    list.remove(&c);
                }
                match read {
                    Err(why) => list.append(&text(&why, 14, 400, &["tafs-no"])),
                    Ok(Listed::Hidden) => {}
                    Ok(Listed::Chromes(chromes)) => {
                        let p = panel(false);
                        if chromes.is_empty() {
                            p.append(&text(
                                &chrome::lines(&w).map(|l| l.none).unwrap_or_default(),
                                15,
                                400,
                                &["tafs-dim"],
                            ));
                        }
                        let mut first: Option<gtk::Button> = None;
                        for c in chromes {
                            let row = chrome_row(w.clone(), source.clone(), c, &list);
                            if first.is_none() {
                                first = row.last_child().and_downcast::<gtk::Button>();
                            }
                            p.append(&row);
                        }
                        list.append(&p);
                        // `--page chrome-unpair`: the first Chrome's Unpair question, for its screenshot.
                        if OPEN_UNPAIR.with(|o| o.replace(false)) {
                            if let Some(b) = first {
                                glib::idle_add_local_once(move || b.emit_clicked());
                            }
                        }
                    }
                }
            });
        })
    };
    RELOAD.with(|r| *r.borrow_mut() = Some(Rc::downgrade(&load)));
    load();
    // The page keeps its loader alive.
    keep_alive(&scroller, load);
    scroller.upcast()
}

thread_local! {
    /// `--page chrome-unpair`: the first Chrome's Unpair question opens once the list is drawn.
    pub static OPEN_UNPAIR: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// The open page's loader, read again once a Chrome is unpaired.
    static RELOAD: std::cell::RefCell<Option<std::rc::Weak<dyn Fn()>>> =
        const { std::cell::RefCell::new(None) };
}

/// Ties the loader's life to the page's.
fn keep_alive(page: &gtk::ScrolledWindow, load: Rc<dyn Fn()>) {
    page.connect_destroy(move |_| {
        let _ = &load;
    });
}

/// One Chrome: its name, "{browser} on {system}" when planque knows both, and Unpair in red behind its question.
fn chrome_row(w: Rc<WordBook>, source: ChromeSource, c: Chrome, over: &gtk::Box) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    r.set_margin_top(6);
    r.set_margin_bottom(6);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    v.append(&one_line(&c.name, 15, 600, &["tafs-ink"]));
    if let Ok(Some(on)) = chrome::browser_on(&w, &c) {
        v.append(&one_line(&on, 13, 400, &["tafs-dim"]));
    }
    // What it's used for ("Your Chrome" 4): the sites of the connections planque lists on it.
    if let Ok(Some(used)) = chrome::used_for(&w, &c) {
        v.append(&crate::widgets::text(&used, 13, 400, &["tafs-soft"]));
    }
    r.append(&v);
    let remove = gtk::Button::with_label(&w.line("chrome.remove").unwrap_or_default());
    remove.add_css_class("tafs-link");
    remove.add_css_class("tafs-no");
    remove.set_valign(gtk::Align::Center);
    remove.set_widget_name(&format!("chrome.remove.{}", c.id));
    {
        let over = over.clone();
        remove.connect_clicked(move |_| {
            let l = |k: &str| w.line(k).unwrap_or_default();
            // What unpairing ends ("Your Chrome" 3): the connections that use it.
            let ends = chrome::unpair_ends(&w, &c).ok().flatten();
            let ask = adw::AlertDialog::new(
                Some(&chrome::remove_question(&w, &c).unwrap_or_default()),
                ends.as_deref(),
            );
            ask.add_response("cancel", &l("common.cancel"));
            ask.add_response("remove", &l("chrome.remove"));
            ask.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
            ask.set_default_response(Some("cancel"));
            ask.set_close_response("cancel");
            let (source, id) = (source.clone(), c.id.clone());
            ask.connect_response(None, move |_, answer| {
                if answer != "remove" {
                    return;
                }
                let (source, id) = (source.clone(), id.clone());
                glib::spawn_future_local(async move {
                    let _ = gio::spawn_blocking(move || unpair(&source, &id)).await;
                    // Read again either way: the list says what planque keeps now.
                    let load = RELOAD.with(|r| r.borrow().as_ref().and_then(|w| w.upgrade()));
                    if let Some(load) = load {
                        load();
                    }
                });
            });
            ask.present(Some(&over));
        });
    }
    r.append(&remove);
    r
}
