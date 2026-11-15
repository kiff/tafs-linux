//! Account › Devices (`DevicesView`): the devices signed in as this person, each with when it was
//! last used, "This device" for this computer, and Sign out for the others. The lines are
//! `tafs-core/src/devices.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::devices::{last_used, Device};
use tafs_core::words::WordBook;

use crate::widgets::{one_line, symbol, text};

/// Where Devices is read from.
#[derive(Clone)]
pub enum DevicesSource {
    /// The sample: the reference set's three in a screenshot, else none, as the iPad's sample.
    Sample {
        screenshot: bool,
    },
    /// KIFF, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    None,
}

#[derive(Default)]
struct State {
    devices: Vec<Device>,
    error: Option<String>,
}

pub fn page(w: Rc<WordBook>, source: DevicesSource, back: Rc<dyn Fn()>) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    // Back to Account, as the iPad's navigation bar's round back button.
    let back_button = gtk::Button::new();
    back_button.set_child(Some(&symbol("go-previous-symbolic", 20)));
    back_button.add_css_class("tafs-round-back");
    back_button.set_halign(gtk::Align::Start);
    back_button.set_tooltip_text(Some(&l("settings.title")));
    back_button.update_property(&[gtk::accessible::Property::Label(&l("settings.title"))]);
    back_button.connect_clicked(move |_| back());
    column.append(&back_button);
    let title = text(&l("settings.devices.title"), 34, 700, &["tafs-ink"]);
    title.set_width_chars(12);
    column.append(&title);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.append(&list);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));

    let state = Rc::new(RefCell::new(State::default()));
    let draw: Rc<dyn Fn()> = {
        let (w, list, state) = (w.clone(), list.clone(), state.clone());
        Rc::new(move || {
            while let Some(c) = list.first_child() {
                list.remove(&c);
            }
            let s = state.borrow();
            let l = |k: &str| w.line(k).unwrap_or_default();
            if let Some(why) = &s.error {
                let p = crate::widgets::panel(false);
                p.append(&text(why, 14, 400, &["tafs-no"]));
                list.append(&p);
            }
            let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
            group.add_css_class("tafs-group");
            let now = chrono::Utc::now();
            for (i, d) in s.devices.iter().enumerate() {
                if i > 0 {
                    let line = gtk::Separator::new(gtk::Orientation::Horizontal);
                    line.add_css_class("tafs-line");
                    group.append(&line);
                }
                let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                row.set_margin_top(12);
                row.set_margin_bottom(12);
                let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
                words.set_hexpand(true);
                let name = d
                    .name
                    .clone()
                    .unwrap_or_else(|| l("settings.devices.unnamed"));
                words.append(&one_line(&name, 15, 600, &["tafs-ink"]));
                words.append(&one_line(
                    &last_used(&w, d.last_used, now).unwrap_or_default(),
                    13,
                    400,
                    &["tafs-dim"],
                ));
                row.append(&words);
                if d.this_device {
                    let t = one_line(
                        &l("settings.devices.this_device_title"),
                        13,
                        400,
                        &["tafs-dim"],
                    );
                    t.set_valign(gtk::Align::Center);
                    row.append(&t);
                } else {
                    // Signing another device out is an approval, signed by an approval key:
                    // there, dimmed, until this computer can approve.
                    let out = gtk::Button::with_label(&l("settings.devices.sign_out"));
                    out.add_css_class("tafs-text-button");
                    out.add_css_class("refuse");
                    out.set_valign(gtk::Align::Center);
                    out.set_sensitive(false);
                    row.append(&out);
                }
                group.append(&row);
            }
            if !s.devices.is_empty() {
                list.append(&group);
            }
            let device = l("device.linux");
            let note = text(
                &w.get("settings.devices.note_device", &[("device", &device)])
                    .unwrap_or_default(),
                13,
                400,
                &["tafs-dim"],
            );
            note.set_margin_start(12);
            note.set_margin_end(12);
            list.append(&note);
        })
    };
    match &source {
        DevicesSource::Sample { screenshot } => {
            if *screenshot {
                state.borrow_mut().devices =
                    tafs_core::devices::sample(&w, chrono::Utc::now()).unwrap_or_default();
            }
        }
        DevicesSource::None => {}
        DevicesSource::Cloud { token, languages } => {
            let (words, token, languages) = ((*w).clone(), token.clone(), languages.clone());
            let (state, draw) = (state.clone(), draw.clone());
            gtk::glib::spawn_future_local(async move {
                let read = gtk::gio::spawn_blocking(move || {
                    let why = |why: String| {
                        words
                            .get("settings.devices.not_loaded", &[("why", &why)])
                            .unwrap_or(why)
                    };
                    let a = crate::kiff::devices(&token, &languages)
                        .map_err(|e| why(e.words(&words).unwrap_or_default()))?;
                    if !(200..300).contains(&a.status) {
                        return Err(why(tafs_core::signin::refusal(&a, Some(&words))
                            .words(&words)
                            .unwrap_or_default()));
                    }
                    Device::read_all(&a.body).ok_or_else(|| {
                        why(tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
                    })
                })
                .await
                .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    match read {
                        Ok(d) => {
                            s.devices = d;
                            s.error = None;
                        }
                        Err(e) => s.error = Some(e),
                    }
                }
                draw();
            });
        }
    }
    draw();
    crate::widgets::ground(&scroller);
    scroller.upcast()
}
