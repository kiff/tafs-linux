//! The picture a paired Chrome kept just before a taf's step acted, from a Record line's receipt (the iPhone's
//! `ChromeShotView`, tafs#1725; Android's tafs#1734): "See the page before this step", one per picture kept for the
//! line's operation, each opening its step's picture, when it was taken, and that it's kept for 30 days. Both reads are
//! planque's safe routes, signed by this computer's device key (`planque::Safe::{Shots, Shot}`); no session token goes.
//! None kept, a 404 or a failed list: no link. A picture that doesn't check out, or doesn't decode: "The picture
//! couldn't be loaded", never a blank.

use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};
use tafs_core::chrome_shots::{self, Read, ShotRef};
use tafs_core::planque::Safe;
use tafs_core::session::KeptSession;
use tafs_core::words::WordBook;

use crate::widgets::text;

/// Where the pictures are read.
#[derive(Clone)]
pub enum ShotSource {
    /// planque, signed by this computer's device key.
    Cloud {
        kept: KeptSession,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answers (`--chrome-shots`: `{"list": …, "shot": …}`). Nothing is sent.
    File(std::path::PathBuf),
}

impl ShotSource {
    /// Signed in with KIFF, this computer's session (a screenshot's file in its place); none in the
    /// sample.
    pub fn cloud(languages: Vec<&'static str>) -> Option<ShotSource> {
        if let Some(path) = FILE.with(|f| f.borrow().clone()) {
            return Some(ShotSource::File(path));
        }
        let kept = crate::store::load(chrono::Utc::now().timestamp())?;
        Some(ShotSource::Cloud { kept, languages })
    }

    /// A screenshot's file (`--chrome-shots`), whatever the session: nothing is read from it but the file.
    pub fn file() -> Option<ShotSource> {
        FILE.with(|f| f.borrow().clone()).map(ShotSource::File)
    }

    fn file_part(path: &std::path::Path, part: &str) -> Option<Vec<u8>> {
        let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        serde_json::to_vec(&v[part]).ok()
    }

    /// The pictures kept for an operation: none on any failure, so the receipt shows no link.
    fn list(&self, op: &str) -> Vec<ShotRef> {
        match self {
            ShotSource::Cloud { kept, languages } => {
                let Some(device) = crate::coach::this_device(kept, languages) else {
                    return vec![];
                };
                crate::kiff::planque_safe(kept, &device, languages, Safe::Shots(op))
                    .map(|a| chrome_shots::read_list(a.status, &a.body))
                    .unwrap_or_default()
            }
            ShotSource::File(path) => Self::file_part(path, "list")
                .map(|b| chrome_shots::read_list(200, &b))
                .unwrap_or_default(),
        }
    }

    /// One step's picture.
    fn shot(&self, step: &str) -> Read {
        match self {
            ShotSource::Cloud { kept, languages } => {
                let Some(device) = crate::coach::this_device(kept, languages) else {
                    return Read::Failed;
                };
                match crate::kiff::planque_safe(kept, &device, languages, Safe::Shot(step)) {
                    Ok(a) => chrome_shots::read(a.status, &a.body, step),
                    Err(_) => Read::Failed,
                }
            }
            ShotSource::File(path) => Self::file_part(path, "shot")
                .map(|b| chrome_shots::read(200, &b, step))
                .unwrap_or(Read::Failed),
        }
    }
}

thread_local! {
    /// `--chrome-shots <file>`: a screenshot's answers, in place of planque.
    pub static FILE: std::cell::RefCell<Option<std::path::PathBuf>> =
        const { std::cell::RefCell::new(None) };
    /// `--page taf-receipt-shot:…`: the first picture's sheet opened once the links are drawn, for the screenshot.
    pub static OPEN_FIRST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The receipt's links, one per picture kept for `op`, into `into` once read; none when there are none.
pub fn links(w: Rc<WordBook>, source: ShotSource, op: String, into: &gtk::Box) {
    let into = into.downgrade();
    glib::spawn_future_local(async move {
        let s2 = source.clone();
        let shots = gio::spawn_blocking(move || s2.list(&op))
            .await
            .unwrap_or_default();
        let Some(into) = into.upgrade() else { return };
        let open_first = OPEN_FIRST.with(|o| o.replace(false));
        for (i, shot) in shots.into_iter().enumerate() {
            let b = gtk::Button::with_label(&w.line("chrome.shot_open").unwrap_or_default());
            b.add_css_class("tafs-link");
            b.set_halign(gtk::Align::Start);
            b.set_widget_name("receipt.shot-open");
            let (w2, s, step) = (w.clone(), source.clone(), shot.step.clone());
            b.connect_clicked(move |b| sheet(w2.clone(), s.clone(), step.clone(), b));
            into.append(&b);
            if open_first && i == 0 {
                let b = b.clone();
                glib::idle_add_local_once(move || b.emit_clicked());
            }
        }
    });
}

/// `ChromeShotView`: "Before this step", the picture, when it was taken (in the account's zone) and that it's kept for
/// 30 days; or that none is kept, or that it couldn't be loaded.
fn sheet(w: Rc<WordBook>, source: ShotSource, step: String, parent: &gtk::Button) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(20);
    column.set_margin_bottom(20);
    column.append(&text(&l("chrome.shot_title"), 26, 700, &["tafs-ink"]));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let spinner = gtk::Spinner::new();
    spinner.start();
    spinner.set_size_request(-1, 160);
    body.append(&spinner);
    column.append(&body);
    let done = gtk::Button::with_label(&l("common.done"));
    done.add_css_class("flat");
    column.append(&done);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_propagate_natural_height(true);
    scroller.set_child(Some(&column));
    crate::widgets::ground(&scroller);
    let dialog = adw::Dialog::new();
    dialog.set_title(&l("chrome.shot_title"));
    dialog.set_child(Some(&scroller));
    dialog.set_content_width(620);
    // Room for the title, a picture at its tallest (340) and its two lines: sized once, so the
    // sheet doesn't open around the spinner and scroll once the picture comes.
    dialog.set_content_height(600);
    {
        let d = dialog.downgrade();
        done.connect_clicked(move |_| {
            if let Some(d) = d.upgrade() {
                d.close();
            }
        });
    }
    dialog.present(Some(parent));
    let body = body.downgrade();
    glib::spawn_future_local(async move {
        // Read and decoded off the main loop, so a large picture never holds the window.
        let (read, decoded) = gio::spawn_blocking(move || {
            let read = source.shot(&step);
            let decoded = match &read {
                Read::Shown(shot) => chrome_shots::pixels(shot),
                _ => None,
            };
            (read, decoded)
        })
        .await
        .unwrap_or((Read::Failed, None));
        let Some(body) = body.upgrade() else { return };
        while let Some(c) = body.first_child() {
            body.remove(&c);
        }
        let failed = || {
            text(
                &w.line("chrome.shot_failed").unwrap_or_default(),
                15,
                400,
                &["tafs-no"],
            )
        };
        match read {
            Read::Shown(shot) => match decoded {
                Some((width, height, rgba)) => {
                    let bytes = glib::Bytes::from_owned(rgba);
                    let texture = gdk::MemoryTexture::new(
                        width as i32,
                        height as i32,
                        gdk::MemoryFormat::R8g8b8a8,
                        &bytes,
                        width as usize * 4,
                    );
                    let picture = gtk::Picture::for_paintable(&texture);
                    picture.set_can_shrink(true);
                    picture.set_content_fit(gtk::ContentFit::Contain);
                    // The sheet's width (620 less its margins), at the picture's own shape, so its
                    // edge is the picture's; never taller than 340, so the whole sheet stays in view.
                    let shown_w = 580.0_f64.min(width as f64);
                    let shown_h = (shown_w * height as f64 / width.max(1) as f64).min(340.0);
                    let shown_w = shown_h * width as f64 / height.max(1) as f64;
                    picture.set_size_request(shown_w as i32, shown_h as i32);
                    picture.set_halign(gtk::Align::Start);
                    picture.add_css_class("tafs-shot");
                    picture.set_widget_name("chrome-shot-image");
                    picture.update_property(&[gtk::accessible::Property::Label(
                        &w.line("chrome.shot_title").unwrap_or_default(),
                    )]);
                    body.append(&picture);
                    let local = tafs_core::account_time::local(&shot.taken_at);
                    body.append(&text(
                        &tafs_core::locale::date_time(w.language(), local),
                        13,
                        400,
                        &["tafs-dim"],
                    ));
                    body.append(&text(
                        &w.line("chrome.shot_kept").unwrap_or_default(),
                        13,
                        400,
                        &["tafs-dim"],
                    ));
                }
                None => body.append(&failed()),
            },
            Read::None => body.append(&text(
                &w.line("chrome.shot_none").unwrap_or_default(),
                15,
                400,
                &["tafs-dim"],
            )),
            Read::Failed => body.append(&failed()),
        }
    });
}
