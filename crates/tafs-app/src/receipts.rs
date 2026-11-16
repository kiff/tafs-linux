//! `ExportReceiptsRow` (`RecordSearchViews.swift`): under the Record's search box, for whoever
//! sees every taf's Record. "Export receipts" opens the last 7, 30 and 90 days; one reads tafs
//! cloud's export, and its JSON and CSV are saved, two files, in the folder the person picks.
//! The iPad hands them to its share sheet; a computer saves them where it's told.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gio, glib};
use tafs_core::receipts::{self, Export};
use tafs_core::words::WordBook;

use crate::widgets::{outline, text};

/// Where the export is read: tafs cloud, as the session. Copied into the read's thread.
#[derive(Clone)]
pub struct ReceiptsSource {
    pub token: String,
    pub languages: Vec<&'static str>,
    /// `--receipts`'s answer, for the screenshots: every export reads it, nothing is sent and
    /// nothing is saved.
    pub canned: Option<Vec<u8>>,
    /// `--page record-export`: the days open when the Record opens.
    pub open: bool,
}

/// The export (`exportReceipts`): tafs cloud's answer, or why not, in plain words.
fn export(w: &WordBook, source: &ReceiptsSource, from: &str, to: &str) -> Result<Export, String> {
    if let Some(body) = &source.canned {
        return Ok(Export::read(body, from, to));
    }
    let a = crate::kiff::receipts(&source.token, &source.languages, &receipts::path(from, to))
        .map_err(|e| e.words(w).unwrap_or_default())?;
    if !(200..300).contains(&a.status) {
        return Err(tafs_core::signin::refusal(&a, Some(w))
            .words(w)
            .unwrap_or_default());
    }
    Ok(Export::read(&a.body, from, to))
}

/// Both files into `dir`, the JSON as tafs cloud sent it.
fn save(dir: &std::path::Path, e: &Export) -> std::io::Result<()> {
    let (json, csv) = e.file_names();
    std::fs::write(dir.join(json), &e.json)?;
    std::fs::write(dir.join(csv), e.csv.as_bytes())
}

/// The row: the button (15 semibold, amber; dim with a spinner while preparing), the days
/// under it while open, how many calls have no receipt yet (13, soft), and why it wasn't
/// exported (13, red).
pub fn row(w: Rc<WordBook>, source: Rc<ReceiptsSource>) -> gtk::Box {
    let Ok(lines) = receipts::row(&w) else {
        return gtk::Box::new(gtk::Orientation::Vertical, 0);
    };
    let b = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let button = gtk::Button::new();
    button.add_css_class("tafs-plain");
    button.set_widget_name("export-receipts");
    let face = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    face.set_size_request(-1, 44);
    let spinner = gtk::Spinner::new();
    spinner.set_visible(false);
    face.append(&spinner);
    let label = text(&lines.export, 15, 600, &["tafs-amber-text"]);
    label.set_valign(gtk::Align::Center);
    label.set_hexpand(true);
    label.set_wrap(false);
    face.append(&label);
    button.set_child(Some(&face));
    b.append(&button);

    let days = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    days.set_visible(source.open);
    b.append(&days);
    let missing = text("", 13, 400, &["tafs-soft"]);
    missing.set_visible(false);
    b.append(&missing);
    let why = text("", 13, 400, &["tafs-no"]);
    why.set_visible(false);
    b.append(&why);

    let preparing = Rc::new(Cell::new(false));
    let set_preparing = {
        let (button, spinner, label, preparing, days, lines) = (
            button.clone(),
            spinner.clone(),
            label.clone(),
            preparing.clone(),
            days.clone(),
            lines.clone(),
        );
        let open = Rc::new(RefCell::new(source.open));
        let set: Rc<dyn Fn(bool, Option<bool>)> =
            Rc::new(move |now: bool, opened: Option<bool>| {
                preparing.set(now);
                if let Some(o) = opened {
                    *open.borrow_mut() = o;
                }
                button.set_sensitive(!now);
                spinner.set_visible(now);
                if now {
                    spinner.start();
                } else {
                    spinner.stop();
                }
                label.set_label(if now { &lines.preparing } else { &lines.export });
                label.remove_css_class(if now { "tafs-amber-text" } else { "tafs-dim" });
                label.add_css_class(if now { "tafs-dim" } else { "tafs-amber-text" });
                days.set_visible(*open.borrow() && !now);
            });
        set
    };
    {
        let (days, set) = (days.clone(), set_preparing.clone());
        button.connect_clicked(move |_| set(false, Some(!days.is_visible())));
    }

    for (count, words) in lines.days {
        let d = outline(&words);
        d.set_hexpand(false);
        let (w, source, preparing, set, missing, why) = (
            w.clone(),
            source.clone(),
            preparing.clone(),
            set_preparing.clone(),
            missing.clone(),
            why.clone(),
        );
        d.connect_clicked(move |d| {
            if preparing.get() {
                return;
            }
            let today = tafs_core::account_time::day(&chrono::Utc::now());
            let (from, to) = receipts::range(count, today);
            set(true, None);
            missing.set_visible(false);
            why.set_visible(false);
            let window = d.root().and_then(|r| r.downcast::<gtk::Window>().ok());
            let (w, source, set, missing, why) = (
                w.clone(),
                source.clone(),
                set.clone(),
                missing.clone(),
                why.clone(),
            );
            glib::spawn_future_local(async move {
                let words: WordBook = (*w).clone();
                let s: ReceiptsSource = (*source).clone();
                let got = gio::spawn_blocking(move || export(&words, &s, &from, &to))
                    .await
                    .unwrap_or_else(|_| Err(String::new()));
                let say_why = |reason: &str| {
                    why.set_label(&receipts::not_exported(&w, reason).unwrap_or_default());
                    why.set_visible(true);
                };
                let e = match got {
                    Ok(e) => e,
                    Err(reason) => {
                        set(false, None);
                        say_why(&reason);
                        return;
                    }
                };
                set(false, Some(false));
                if let Ok(Some(m)) = receipts::missing(&w, e.missing) {
                    missing.set_label(&m);
                    missing.set_visible(true);
                }
                // A screenshot's export is never saved.
                if source.canned.is_some() {
                    return;
                }
                // Where to save them; closing the dialog saves nothing, as closing the share sheet.
                let dialog = gtk::FileDialog::new();
                dialog.set_title(&receipts::row(&w).map(|l| l.export).unwrap_or_default());
                if let Ok(dir) = dialog.select_folder_future(window.as_ref()).await {
                    if let Some(path) = dir.path() {
                        if let Err(err) = save(&path, &e) {
                            say_why(&err.to_string());
                        }
                    }
                }
            });
        });
        days.append(&d);
    }
    b
}
