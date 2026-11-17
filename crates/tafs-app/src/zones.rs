//! Account › Time zone (`TimeZonePicker`): this computer's zone to use, then tafs cloud's zones for
//! what the person types, a moment after the last key, the account's own checked. Choosing one is
//! the owner's act, signed: Use this computer's and each zone are there, and their tap says this
//! computer can't approve, under the first row where the iPad says why the zone didn't change,
//! and sends nothing. The lines are `tafs-core/src/zones.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::sidebar::Mode;
use tafs_core::words::WordBook;
use tafs_core::zones::{choose, path, read, Zone};

use crate::widgets::{one_line, symbol, text};

/// Where the zones are read.
#[derive(Clone)]
pub enum ZonesSource {
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A file of synthetic answers (`--people`'s `zones`, for the screenshot).
    File(std::path::PathBuf),
    /// The sample and signed out: no zones.
    None,
}

thread_local! {
    /// `--page time-zone-use`: Use this computer's is tapped once the page shows, for its
    /// screenshot.
    pub static TAP_USE_HERE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[derive(Default)]
struct State {
    zones: Vec<Zone>,
    error: Option<String>,
    query: String,
    request: u64,
}

fn fetch(w: &WordBook, source: &ZonesSource, q: &str) -> Result<Vec<Zone>, String> {
    match source {
        ZonesSource::Cloud { token, languages } => {
            let a = crate::kiff::cloud(token, languages, &path(q))
                .map_err(|e| e.words(w).unwrap_or_default())?;
            if !(200..300).contains(&a.status) {
                return Err(tafs_core::signin::refusal(&a, Some(w))
                    .words(w)
                    .unwrap_or_default());
            }
            read(&a.body).ok_or_else(|| tafs_core::signin::plain_words(w, 200).unwrap_or_default())
        }
        ZonesSource::File(f) => {
            let v: serde_json::Value = std::fs::read(f)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            let all =
                read(&serde_json::to_vec(&v["zones"]).unwrap_or_default()).unwrap_or_default();
            let q = q.to_lowercase();
            Ok(all
                .into_iter()
                .filter(|z| {
                    q.is_empty()
                        || z.title.to_lowercase().contains(&q)
                        || z.id.to_lowercase().contains(&q)
                })
                .collect())
        }
        ZonesSource::None => Ok(vec![]),
    }
}

pub fn page(
    w: Rc<WordBook>,
    source: ZonesSource,
    current: Option<String>,
    back: Rc<dyn Fn()>,
) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let back_button = gtk::Button::new();
    back_button.set_child(Some(&symbol("go-previous-symbolic", 20)));
    back_button.add_css_class("tafs-round-back");
    back_button.set_halign(gtk::Align::Start);
    back_button.update_property(&[gtk::accessible::Property::Label(&l("settings.title"))]);
    back_button.connect_clicked(move |_| back());
    column.append(&back_button);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = text(&l("settings.time_zone_title"), 34, 700, &["tafs-ink"]);
    title.set_hexpand(true);
    title.set_width_chars(10);
    head.append(&title);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&l("settings.find_time_zone")));
    search.set_valign(gtk::Align::Center);
    search.set_width_chars(22);
    search.add_css_class("tafs-search");
    head.append(&search);
    column.append(&head);
    // This computer's zone, to use: the owner's act, signed.
    let here = tafs_core::account_time::local_identifier();
    let device = l("device.linux");
    let first = gtk::Box::new(gtk::Orientation::Vertical, 0);
    first.add_css_class("tafs-group");
    let use_here = gtk::Button::with_label(
        &w.get(
            "settings.use_device_zone",
            &[("device", &device), ("identifier", &here)],
        )
        .unwrap_or_default(),
    );
    use_here.add_css_class("tafs-row-button");
    use_here.add_css_class("amber");
    if let Some(c) = use_here.child() {
        c.set_halign(gtk::Align::Start);
    }
    use_here.set_widget_name("zones.use-here");
    first.append(&use_here);
    let error = text("", 13, 400, &["tafs-no"]);
    error.set_visible(false);
    error.set_margin_bottom(10);
    first.append(&error);
    // The iPad's `timeZoneError` spot: once a tap would have signed, this computer's words.
    let mode = match source {
        ZonesSource::None => Mode::SignedOut,
        _ => Mode::Kiff,
    };
    let refused = crate::approval::refusal_slot();
    refused.set_margin_bottom(10);
    first.append(&refused);
    {
        let (w, refused, here) = (w.clone(), refused.clone(), here.clone());
        use_here
            .connect_clicked(move |_| crate::approval::refuse(&w, mode, &choose(&here), &refused));
    }
    if TAP_USE_HERE.with(|t| t.replace(false)) {
        use_here.connect_map(|b| b.emit_clicked());
    }
    column.append(&first);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
    list.add_css_class("tafs-group");
    // Its own section, set apart from the first as the iPad's are (36 in all).
    list.set_margin_top(36 - column.spacing());
    column.append(&list);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));

    let state = Rc::new(RefCell::new(State::default()));
    let draw: Rc<dyn Fn()> = {
        let (w, list, state, error, refused) = (
            w.clone(),
            list.clone(),
            state.clone(),
            error.clone(),
            refused.clone(),
        );
        Rc::new(move || {
            while let Some(c) = list.first_child() {
                list.remove(&c);
            }
            let s = state.borrow();
            error.set_visible(s.error.is_some());
            error.set_label(s.error.as_deref().unwrap_or(""));
            list.set_visible(!s.zones.is_empty());
            for (i, z) in s.zones.iter().enumerate() {
                if i > 0 {
                    let line = gtk::Separator::new(gtk::Orientation::Horizontal);
                    line.add_css_class("tafs-line");
                    list.append(&line);
                }
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                row.set_margin_top(12);
                row.set_margin_bottom(12);
                // In the list's tint, as the iPad draws a row that is a button.
                let t = one_line(&z.title, 17, 400, &["tafs-amber"]);
                t.set_hexpand(true);
                row.append(&t);
                if current.as_deref() == Some(z.id.as_str()) {
                    let check = symbol("tafs-checkmark-symbolic", 17);
                    check.add_css_class("tafs-amber");
                    row.append(&check);
                }
                // A zone is a button: choosing it would sign.
                let b = gtk::Button::new();
                b.set_child(Some(&row));
                b.add_css_class("tafs-plain");
                b.set_widget_name(&format!("zones.{}", z.id));
                let (w, refused, id) = (w.clone(), refused.clone(), z.id.clone());
                b.connect_clicked(move |_| {
                    crate::approval::refuse(&w, mode, &choose(&id), &refused)
                });
                list.append(&b);
            }
        })
    };
    // Reads the zones for what was typed; only the newest search's answer is kept.
    let load: Rc<dyn Fn()> = {
        let (w, source, state, draw) = (w.clone(), source.clone(), state.clone(), draw.clone());
        Rc::new(move || {
            let (request, q) = {
                let mut s = state.borrow_mut();
                s.request += 1;
                (s.request, s.query.clone())
            };
            let (words, source, state, draw) =
                ((*w).clone(), source.clone(), state.clone(), draw.clone());
            gtk::glib::spawn_future_local(async move {
                let got = gtk::gio::spawn_blocking(move || fetch(&words, &source, &q))
                    .await
                    .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    if s.request != request {
                        return;
                    }
                    match got {
                        Ok(z) => {
                            s.zones = z;
                            s.error = None;
                        }
                        Err(e) => s.error = Some(e),
                    }
                }
                draw();
            });
        })
    };
    {
        let (state, load) = (state.clone(), load.clone());
        search.connect_search_changed(move |e| {
            state.borrow_mut().query = e.text().to_string();
            load();
        });
    }
    load();
    draw();
    crate::widgets::ground(&scroller);
    scroller.upcast()
}
