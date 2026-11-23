//! Account › Devices (`DevicesView`): the devices signed in as this person, each with when it was
//! last used, "This device" for this computer, and Sign out for the others; then the account's
//! passkeys, each of the person's own with Remove for an owner or admin. Signing a device out and
//! removing a passkey are approvals: each asks its question, as the iPad's, then this computer's
//! security key approves it where the session approves with it, else this computer says it can't
//! approve and sends nothing. The lines are `tafs-core/src/devices.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use tafs_core::devices::{
    last_used, may_remove, remove_question, sign_out_question, Device, Passkey, PASSKEYS_PATH,
};
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

/// The passkeys' read, again: set once the source is known.
type ReadAgain = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

#[derive(Default)]
struct State {
    devices: Vec<Device>,
    error: Option<String>,
    /// The account's passkeys, signed in with KIFF: none before they're read.
    passkeys: Option<Vec<Passkey>>,
    passkeys_error: Option<String>,
    /// The person's role on the account (`member_role`): none read, none.
    role: Option<String>,
    /// This computer's security key approves for the session (`canAddPasskey`'s approval key).
    approves: bool,
    /// Each read of the passkeys takes the next number, and so does a removal: only the newest
    /// read's list is kept, so one started before a Remove never brings the removed one back.
    passkeys_read: u64,
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
    // Why a passkey wasn't removed (`passkeysError`), under Passkeys; and the read of the passkeys
    // again once one was, set below with the source.
    let key_refused = crate::approval::refusal_slot();
    let read_passkeys: ReadAgain = Rc::new(RefCell::new(None));
    let draw: Rc<dyn Fn()> = {
        let (w, list, state, refused, refused_panel, key_refused, read_passkeys) = (
            w.clone(),
            list.clone(),
            state.clone(),
            refused.clone(),
            refused_panel.clone(),
            key_refused.clone(),
            read_passkeys.clone(),
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
            // its computer. Remove, for an owner or admin, on the person's own: approved with this
            // computer's security key. Add is an iPhone's.
            if s.passkeys.is_some() || s.passkeys_error.is_some() {
                list.append(&crate::sheet::header(&l("settings.devices.passkeys_title")));
                if let Some(why) = &s.passkeys_error {
                    let t = text(why, 14, 400, &["tafs-no"]);
                    t.set_margin_start(12);
                    list.append(&t);
                }
                key_refused.set_margin_start(12);
                list.append(&key_refused);
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
                        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                        row.set_margin_top(12);
                        row.set_margin_bottom(12);
                        let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
                        words.set_hexpand(true);
                        let name = k
                            .name
                            .clone()
                            .unwrap_or_else(|| l("settings.devices.unnamed"));
                        words.append(&one_line(&name, 15, 600, &["tafs-ink"]));
                        if !k.removable() {
                            words.append(&one_line(
                                &l("settings.devices.passkey_with_computer"),
                                13,
                                400,
                                &["tafs-dim"],
                            ));
                        }
                        row.append(&words);
                        let question = may_remove(k, s.role.as_deref(), s.approves)
                            .then(|| remove_question(&w, k, keys).ok().flatten())
                            .flatten();
                        if let Some(q) = question {
                            let remove = gtk::Button::with_label(&q.button);
                            remove.add_css_class("tafs-text-button");
                            remove.add_css_class("refuse");
                            remove.set_valign(gtk::Align::Center);
                            if let Some(id) = &k.id {
                                remove.set_widget_name(&format!("devices.passkey.remove.{id}"));
                            }
                            let (w, state, key_refused, read_passkeys) = (
                                w.clone(),
                                state.clone(),
                                key_refused.clone(),
                                read_passkeys.clone(),
                            );
                            remove.connect_clicked(move |b| {
                                remove_passkey(
                                    &w,
                                    mode,
                                    &q,
                                    b.upcast_ref(),
                                    &key_refused,
                                    state.clone(),
                                    read_passkeys.borrow().clone(),
                                )
                            });
                            row.append(&remove);
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
            let again = (
                (*w).clone(),
                token.clone(),
                languages.clone(),
                state.clone(),
                draw.clone(),
            );
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
                    let passkeys = passkeys_read(&token, &languages, &words);
                    // The person's role on the account (`/v1/me`'s `member_role`); none read, no
                    // Remove, as the iPad's `canAddPasskey` with no role.
                    let role = crate::kiff::me(&token, &languages)
                        .ok()
                        .and_then(|a| tafs_core::account::Account::read(&a).ok())
                        .and_then(|a| a.member_role);
                    // The session approves with this computer's security key (`can_approve`).
                    let kept = crate::store::load(chrono::Utc::now().timestamp());
                    let approves = crate::key_approval::can_approve(
                        kept.as_ref(),
                        &tafs_core::signed::Act::RemovePasskey {
                            passkey: String::new(),
                        },
                    );
                    Ok((devices, passkeys, role, approves))
                })
                .await
                .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    match read {
                        Ok((d, passkeys, role, approves)) => {
                            s.devices = d;
                            s.error = None;
                            s.role = role;
                            s.approves = approves;
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
            // `refreshPasskeys`, once a passkey was removed: only the newest read's list is kept.
            let (words, token, languages, state, draw) = again;
            *read_passkeys.borrow_mut() = Some(Rc::new(move || {
                let read = {
                    let mut s = state.borrow_mut();
                    s.passkeys_read += 1;
                    s.passkeys_read
                };
                // What's known now (a removed one gone) shows at once, then the newest read.
                draw();
                let (words, token, languages) = (words.clone(), token.clone(), languages.clone());
                let (state, draw) = (state.clone(), draw.clone());
                gtk::glib::spawn_future_local(async move {
                    let passkeys =
                        gtk::gio::spawn_blocking(move || passkeys_read(&token, &languages, &words))
                            .await;
                    {
                        let mut s = state.borrow_mut();
                        if s.passkeys_read != read {
                            return;
                        }
                        match passkeys {
                            Ok(Ok(p)) => {
                                s.passkeys = Some(p);
                                s.passkeys_error = None;
                            }
                            Ok(Err(e)) => s.passkeys_error = Some(e),
                            Err(_) => return,
                        }
                    }
                    draw();
                });
            }));
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
    let (w, act, prompt, refused, refused_panel) = (
        w.clone(),
        q.act.clone(),
        q.prompt.clone(),
        refused.clone(),
        refused_panel.clone(),
    );
    ask.connect_response(None, move |_, response| {
        if response == "sign-out" {
            crate::approval::refuse_as(&w, mode, &act, Some(prompt.clone()), &refused);
            refused_panel.set_visible(refused.get_visible());
        }
    });
    ask.present(Some(over));
}

/// `refreshPasskeys`: the account's passkeys, read as the session; why not, in the words shown
/// under their heading. Blocking: off the main loop.
fn passkeys_read(
    token: &str,
    languages: &[&str],
    words: &WordBook,
) -> Result<Vec<Passkey>, String> {
    let not_loaded = |why: String| {
        words
            .get("settings.devices.passkeys_not_loaded", &[("why", &why)])
            .unwrap_or(why)
    };
    match crate::kiff::kiff_get(token, languages, PASSKEYS_PATH) {
        Ok(a) if (200..300).contains(&a.status) => Passkey::read_all(&a.body).ok_or_else(|| {
            not_loaded(tafs_core::signin::plain_words(words, 200).unwrap_or_default())
        }),
        Ok(a) => Err(not_loaded(
            tafs_core::signin::refusal(&a, Some(words))
                .words(words)
                .unwrap_or_default(),
        )),
        Err(e) => Err(not_loaded(e.words(words).unwrap_or_default())),
    }
}

/// The question before a passkey is removed: "Remove Rae's iPhone?", saying so when it's the last
/// that can approve, Remove in red, and Cancel. Its Remove asks this computer's security key, which
/// sends exactly the passkey's DELETE; once KIFF took it, it leaves the list, and the list is read
/// again, so a read already on its way can't bring it back. Why not, under Passkeys.
fn remove_passkey(
    w: &WordBook,
    mode: Mode,
    q: &tafs_core::devices::RemoveQuestion,
    over: &gtk::Widget,
    refused: &gtk::Box,
    state: Rc<RefCell<State>>,
    read_again: Option<Rc<dyn Fn()>>,
) {
    let ask = adw::AlertDialog::new(Some(&q.title), q.message.as_deref());
    ask.add_response("cancel", &q.cancel);
    ask.add_response("remove", &q.button);
    ask.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
    ask.set_default_response(Some("cancel"));
    ask.set_close_response("cancel");
    let (w, act, prompt, refused) = (w.clone(), q.act.clone(), q.prompt.clone(), refused.clone());
    ask.connect_response(None, move |_, response| {
        if response != "remove" {
            return;
        }
        let tafs_core::signed::Act::RemovePasskey { passkey } = &act else {
            return;
        };
        // Only drawn where this computer's key approves; if that changed since, its words.
        let kept = crate::store::load(chrono::Utc::now().timestamp())
            .filter(|k| crate::key_approval::can_approve(Some(k), &act));
        let Some(kept) = kept else {
            crate::approval::refuse_as(&w, mode, &act, Some(prompt.clone()), &refused);
            return;
        };
        let (passkey, state, read_again) = (passkey.clone(), state.clone(), read_again.clone());
        let (slot, words, what) = (refused.clone(), w.clone(), prompt.clone());
        while let Some(c) = slot.first_child() {
            slot.remove(&c);
        }
        let done: crate::key_approval::Approved = Rc::new(move |outcome| match outcome {
            // `didntGoThrough`: what, then why in the app's words; never KIFF's code.
            Some(Err(why)) => {
                let line = words
                    .get("common.didnt_go_through", &[("what", &what), ("why", &why)])
                    .unwrap_or(why);
                slot.append(&text(&line, 13, 400, &["tafs-no"]));
                slot.set_visible(true);
            }
            // KIFF took it: it leaves the list, and a read already on its way is dropped.
            Some(Ok(())) => {
                slot.set_visible(false);
                {
                    let mut s = state.borrow_mut();
                    s.passkeys_read += 1;
                    if let Some(keys) = &mut s.passkeys {
                        keys.retain(|k| k.id.as_deref() != Some(passkey.as_str()));
                    }
                }
                if let Some(read_again) = &read_again {
                    read_again();
                }
            }
            // The key's sheet closed: nothing was sent, and nothing is said.
            None => slot.set_visible(false),
        });
        let languages =
            tafs_core::language::screen_languages(&tafs_core::language::preferred_from_env());
        crate::key_approval::approve(
            Rc::new(w.clone()),
            &refused,
            kept,
            languages,
            &act,
            Some(&prompt),
            vec![],
            done,
        );
    });
    ask.present(Some(over));
}
