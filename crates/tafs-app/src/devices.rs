//! Account › Devices (`DevicesView`): the devices signed in as this person, each with when it was
//! last used, "This device" for this computer, and Sign out for the others. Signing another device
//! out is an approval the iPad signs: Sign out asks its question, as the iPad's, and its Sign out
//! says this computer can't approve, and sends nothing. The lines are `tafs-core/src/devices.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use tafs_core::devices::{last_used, sign_out_question, Device, Passkey, PASSKEYS_PATH};
use tafs_core::sidebar::Mode;
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

thread_local! {
    /// `--page devices-sign-out`: the question for the first other device opens, for its
    /// screenshot.
    pub static OPEN_SIGN_OUT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[derive(Default)]
struct State {
    devices: Vec<Device>,
    error: Option<String>,
    /// The account's passkeys, signed in with KIFF: none before they're read.
    passkeys: Option<Vec<Passkey>>,
    passkeys_error: Option<String>,
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
    // Where the iPad says why a device wasn't signed out (`devicesError`): here, that this
    // computer can't approve, once Sign out would have signed.
    let mode = match source {
        DevicesSource::Cloud { .. } => Mode::Kiff,
        DevicesSource::Sample { .. } => Mode::Sample,
        DevicesSource::None => Mode::SignedOut,
    };
    let refused = crate::approval::refusal_slot();
    let refused_panel = crate::widgets::panel(false);
    refused_panel.append(&refused);
    refused_panel.set_visible(false);
    column.append(&refused_panel);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.append(&list);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));

    let state = Rc::new(RefCell::new(State::default()));
    let draw: Rc<dyn Fn()> = {
        let (w, list, state, refused, refused_panel) = (
            w.clone(),
            list.clone(),
            state.clone(),
            refused.clone(),
            refused_panel.clone(),
        );
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
                } else if let Ok(Some(q)) = sign_out_question(&w, d) {
                    // Signing another device out is an approval, signed by an approval key: its
                    // question, then this computer's words where it would sign.
                    let out = gtk::Button::with_label(&l("settings.devices.sign_out"));
                    out.add_css_class("tafs-text-button");
                    out.add_css_class("refuse");
                    out.set_valign(gtk::Align::Center);
                    out.set_widget_name(&format!("devices.sign-out.{}", d.id));
                    let (w, refused, refused_panel) =
                        (w.clone(), refused.clone(), refused_panel.clone());
                    let ask: Rc<dyn Fn(&gtk::Widget)> = Rc::new(move |over: &gtk::Widget| {
                        sign_out(&w, mode, &q, over, &refused, &refused_panel)
                    });
                    {
                        let ask = ask.clone();
                        out.connect_clicked(move |b| ask(b.upcast_ref()));
                    }
                    if OPEN_SIGN_OUT.with(|o| o.replace(false)) {
                        out.connect_map(move |b| ask(b.upcast_ref()));
                    }
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
            // PASSKEYS (signed in with KIFF): each by its name, a computer's own saying it goes with
            // its computer. Remove takes the approval key and Add is an iPhone's, so on a computer
            // the iPad lists them only, as here.
            if s.passkeys.is_some() || s.passkeys_error.is_some() {
                list.append(&crate::sheet::header(&l("settings.devices.passkeys_title")));
                if let Some(why) = &s.passkeys_error {
                    let t = text(why, 14, 400, &["tafs-no"]);
                    t.set_margin_start(12);
                    list.append(&t);
                }
                let keys = s.passkeys.as_deref().unwrap_or_default();
                if !keys.is_empty() {
                    let group = gtk::Box::new(gtk::Orientation::Vertical, 0);
                    group.add_css_class("tafs-group");
                    group.set_widget_name("devices.passkeys");
                    for (i, k) in keys.iter().enumerate() {
                        if i > 0 {
                            let line = gtk::Separator::new(gtk::Orientation::Horizontal);
                            line.add_css_class("tafs-line");
                            group.append(&line);
                        }
                        let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
                        row.set_margin_top(12);
                        row.set_margin_bottom(12);
                        let name = k
                            .name
                            .clone()
                            .unwrap_or_else(|| l("settings.devices.unnamed"));
                        row.append(&one_line(&name, 15, 600, &["tafs-ink"]));
                        if !k.removable() {
                            row.append(&one_line(
                                &l("settings.devices.passkey_with_computer"),
                                13,
                                400,
                                &["tafs-dim"],
                            ));
                        }
                        group.append(&row);
                    }
                    list.append(&group);
                }
            }
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
                    let devices = Device::read_all(&a.body).ok_or_else(|| {
                        why(tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
                    })?;
                    // `refreshPasskeys`: the account's passkeys, read as the session; why not,
                    // under their heading.
                    let not_loaded = |why: String| {
                        words
                            .get("settings.devices.passkeys_not_loaded", &[("why", &why)])
                            .unwrap_or(why)
                    };
                    let passkeys = match crate::kiff::kiff_get(&token, &languages, PASSKEYS_PATH) {
                        Ok(a) if (200..300).contains(&a.status) => Passkey::read_all(&a.body)
                            .ok_or_else(|| {
                                not_loaded(
                                    tafs_core::signin::plain_words(&words, 200).unwrap_or_default(),
                                )
                            }),
                        Ok(a) => Err(not_loaded(
                            tafs_core::signin::refusal(&a, Some(&words))
                                .words(&words)
                                .unwrap_or_default(),
                        )),
                        Err(e) => Err(not_loaded(e.words(&words).unwrap_or_default())),
                    };
                    Ok((devices, passkeys))
                })
                .await
                .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    match read {
                        Ok((d, passkeys)) => {
                            s.devices = d;
                            s.error = None;
                            match passkeys {
                                Ok(p) => {
                                    s.passkeys = Some(p);
                                    s.passkeys_error = None;
                                }
                                Err(e) => s.passkeys_error = Some(e),
                            }
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

/// The question before another device is signed out: "Sign Rae's iPhone out?", Sign out in red,
/// and Cancel. Its Sign out would sign: this computer says it can't approve, and sends nothing.
fn sign_out(
    w: &WordBook,
    mode: Mode,
    q: &tafs_core::devices::SignOutQuestion,
    over: &gtk::Widget,
    refused: &gtk::Box,
    refused_panel: &gtk::Box,
) {
    let ask = adw::AlertDialog::new(Some(&q.title), None);
    ask.add_response("cancel", &q.cancel);
    ask.add_response("sign-out", &q.button);
    ask.set_response_appearance("sign-out", adw::ResponseAppearance::Destructive);
    ask.set_default_response(Some("cancel"));
    ask.set_close_response("cancel");
    let (w, act, refused, refused_panel) = (
        w.clone(),
        q.act.clone(),
        refused.clone(),
        refused_panel.clone(),
    );
    ask.connect_response(None, move |_, response| {
        if response == "sign-out" {
            crate::approval::refuse(&w, mode, &act, &refused);
            refused_panel.set_visible(refused.get_visible());
        }
    });
    ask.present(Some(over));
}
