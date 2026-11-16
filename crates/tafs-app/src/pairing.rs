//! Adding this computer with a phone (tafs-macos `SignInView.pairing`, `SignInModel.pair`, and
//! `QRCode`): the QR code, where to scan it on the phone, the 6-character check code, and that
//! this computer is waiting. Shown as the sign-in is: a dialog over what's open, with Cancel.
//!
//! - **The keyring first.** Before a code is shown, the keyring is asked to take a session
//!   (`store::check_writable`); if it refuses, the pairing stops there, with no code to approve.
//! - **Each code is new**: a new device key and a new nonce. KIFF refuses a pairing for a key that
//!   is already a device's, so a view-only computer pairs with a key of its own, never its old one.
//! - **The new session is kept first**, then a session it replaces is signed out at KIFF. If the
//!   pairing doesn't happen (the code runs out, KIFF refuses, the person cancels), nothing is kept
//!   and nothing is signed out.
//! - **Cancel** stops the claims; a session handed over to a closed pairing ends at KIFF at once.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::keys::DeviceKey;
use tafs_core::pairing::{self, QrModules};
use tafs_core::session::KeptSession;
use tafs_core::signin::{self, Cancelled, Clock, KiffError, Stopped};
use tafs_core::words::WordBook;

use crate::kiff::Kiff;
use crate::widgets::{outline, primary, text};

/// The QR code's side on screen, as tafs-macos draws it, and its white margin.
const SIDE: i32 = 220;
const MARGIN: i32 = 12;

/// The claims' wait: the interval in slices of half a second, stopped when the pairing closes.
struct Waiting {
    started: Instant,
    cancelled: Arc<AtomicBool>,
}

impl Clock for Waiting {
    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
    fn wait(&self, d: Duration) -> Result<(), Cancelled> {
        let slice = Duration::from_millis(500);
        let mut waited = Duration::ZERO;
        while waited < d {
            if self.cancelled.load(Ordering::SeqCst) {
                return Err(Cancelled);
            }
            let step = slice.min(d - waited);
            std::thread::sleep(step);
            waited += step;
        }
        if self.cancelled.load(Ordering::SeqCst) {
            return Err(Cancelled);
        }
        Ok(())
    }
}

/// A code on screen: the QR code's text and the check code.
#[derive(Clone)]
pub struct Code {
    pub text: String,
    pub check: String,
}

/// What the pairing shows: its code, a code that ran out, or why it didn't work.
enum Step {
    Starting,
    Code(Code),
    Expired,
    Failed(String),
}

struct Ui {
    code_box: gtk::Box,
    qr: gtk::DrawingArea,
    modules: Rc<RefCell<Option<QrModules>>>,
    check: gtk::Label,
    starting: gtk::Spinner,
    note: gtk::Label,
    again: gtk::Button,
    /// The sign-in's half: "It will ask whether this computer shows {code}." (`pairCheck`).
    check_line: Option<gtk::Label>,
}

/// A QR code drawn as squares, crisp at any scale: black modules on white, with a white margin
/// (the quiet zone) around them.
fn qr_area(modules: Rc<RefCell<Option<QrModules>>>) -> gtk::DrawingArea {
    let area = gtk::DrawingArea::new();
    area.set_content_width(SIDE + 2 * MARGIN);
    area.set_content_height(SIDE + 2 * MARGIN);
    area.set_halign(gtk::Align::Center);
    area.set_draw_func(move |_, cr, width, height| {
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let r = 12.0;
        let (w, h) = (f64::from(width), f64::from(height));
        cr.new_sub_path();
        cr.arc(w - r, r, r, -std::f64::consts::FRAC_PI_2, 0.0);
        cr.arc(w - r, h - r, r, 0.0, std::f64::consts::FRAC_PI_2);
        cr.arc(
            r,
            h - r,
            r,
            std::f64::consts::FRAC_PI_2,
            std::f64::consts::PI,
        );
        cr.arc(r, r, r, std::f64::consts::PI, 1.5 * std::f64::consts::PI);
        cr.close_path();
        let _ = cr.fill();
        let Some(m) = modules.borrow().clone() else {
            return;
        };
        let side = f64::from(width.min(height) - 2 * MARGIN);
        let cell = (side / m.width as f64).floor().max(1.0);
        let drawn = cell * m.width as f64;
        let x0 = ((w - drawn) / 2.0).round();
        let y0 = ((h - drawn) / 2.0).round();
        cr.set_source_rgb(0.0, 0.0, 0.0);
        for y in 0..m.width {
            for x in 0..m.width {
                if m.is_dark(x, y) {
                    cr.rectangle(x0 + x as f64 * cell, y0 + y as f64 * cell, cell, cell);
                }
            }
        }
        let _ = cr.fill();
    });
    area
}

fn show(ui: &Ui, w: &WordBook, step: &Step) {
    let device = w.line("device.linux").unwrap_or_default();
    ui.code_box.set_visible(matches!(step, Step::Code(_)));
    ui.starting.set_visible(matches!(step, Step::Starting));
    ui.starting.set_spinning(matches!(step, Step::Starting));
    ui.note
        .set_visible(matches!(step, Step::Expired | Step::Failed(_)));
    ui.again
        .set_visible(matches!(step, Step::Expired | Step::Failed(_)));
    ui.note.remove_css_class("tafs-no");
    ui.note.remove_css_class("tafs-dim");
    match step {
        Step::Starting => {}
        Step::Code(code) => {
            *ui.modules.borrow_mut() = pairing::qr(&code.text);
            ui.qr.queue_draw();
            ui.check.set_label(&code.check);
            if let Some(line) = &ui.check_line {
                line.set_label(
                    &w.get(
                        "sign_in.pair_check",
                        &[("device", &device), ("code", &code.check)],
                    )
                    .unwrap_or_default(),
                );
            }
        }
        Step::Expired => {
            ui.note
                .set_label(&w.line("pair.code_expired").unwrap_or_default());
            ui.note.add_css_class("tafs-dim");
            ui.again
                .set_label(&w.line("pair.new_code").unwrap_or_default());
        }
        Step::Failed(why) => {
            ui.note.set_label(
                &w.get("pair.failed", &[("device", &device), ("why", why)])
                    .unwrap_or_default(),
            );
            ui.note.add_css_class("tafs-no");
            ui.again
                .set_label(&w.line("common.try_again").unwrap_or_default());
        }
    }
}

/// Why a pairing didn't work, as `pair.failed` names it: KIFF's own code for a refusal, else the
/// app's line for what happened.
fn why(e: &KiffError, w: &WordBook) -> String {
    let why = match e {
        KiffError::Server { message, .. } => message.clone(),
        other => other.words(w).unwrap_or_default(),
    };
    // `pair.failed` ends the sentence itself.
    why.trim_end_matches('.').to_string()
}

/// A pairing on screen: what it draws, and how to stop it (closing what holds it).
pub struct Pairing {
    pub root: gtk::Box,
    attempt: Rc<RefCell<Arc<AtomicBool>>>,
}

impl Pairing {
    /// Stops its claims: a session KIFF hands over after this ends at KIFF at once.
    pub fn stop(&self) {
        self.attempt.borrow().store(true, Ordering::SeqCst);
    }
}

/// The pairing, started. Alone (`present`): the title, the QR, where to scan it, the check code,
/// the wait. `embedded`: the sign-in's right half (`PairSignInView(embedded: true)`), the QR first,
/// "Sign in with your phone", "Scan with tafs", then where to scan it and the check code in its
/// panel. `done` gets the kept session; whoever holds the pairing closes.
pub fn body(
    w: Rc<WordBook>,
    languages: Vec<&'static str>,
    replaced: Option<KeptSession>,
    done: Rc<dyn Fn(KeptSession)>,
    preview: bool,
    embedded: bool,
) -> Pairing {
    let l = {
        let w = w.clone();
        move |k: &str| w.line(k).unwrap_or_else(|e| panic!("{e}"))
    };
    let device = l("device.linux");

    let content = gtk::Box::new(gtk::Orientation::Vertical, if embedded { 22 } else { 18 });
    content.set_margin_start(32);
    content.set_margin_end(32);
    content.set_margin_top(if embedded { 32 } else { 12 });
    content.set_margin_bottom(32);
    let centred = |label: gtk::Label| {
        label.set_xalign(0.5);
        label.set_justify(gtk::Justification::Center);
        label
    };
    let steps = || {
        w.get(
            "pair.steps",
            &[
                ("account", &l("settings.title")),
                ("devices", &l("settings.devices.title")),
                ("pair", &l("settings.devices.pair")),
            ],
        )
        .unwrap()
    };

    let modules: Rc<RefCell<Option<QrModules>>> = Rc::new(RefCell::new(None));
    let qr = qr_area(modules.clone());
    qr.update_property(&[gtk::accessible::Property::Label(
        &w.get("pair.title", &[("device", &device)]).unwrap(),
    )]);
    let check = gtk::Label::new(None);
    check.set_selectable(true);
    check.add_css_class("tafs-code");
    let wait_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let spinner = gtk::Spinner::new();
    spinner.start();
    wait_row.append(&spinner);
    let code_box = gtk::Box::new(gtk::Orientation::Vertical, 18);
    let starting = gtk::Spinner::new();
    starting.set_halign(gtk::Align::Center);
    let check_line_label = text("", 13, 400, &["tafs-dim"]);

    if embedded {
        // The QR first, on its white card, then the half's title and how to scan it.
        let card = gtk::Box::new(gtk::Orientation::Vertical, 0);
        card.add_css_class("tafs-qr-card");
        card.set_halign(gtk::Align::Center);
        card.append(&qr);
        code_box.append(&card);
        content.append(&code_box);
        starting.set_size_request(-1, SIDE + 2 * MARGIN);
        content.append(&starting);
        content.append(&centred(text(
            &l("sign_in.with_phone"),
            30,
            700,
            &["tafs-ink"],
        )));
        let scan = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        scan.set_halign(gtk::Align::Center);
        scan.append(&crate::widgets::symbol("tafs-viewfinder-symbolic", 17));
        scan.append(&text(&l("sign_in.scan_with_tafs"), 17, 400, &["tafs-soft"]));
        content.append(&scan);
        // Once there's a code: where to scan it, then the check code in its panel, and the wait.
        let lines = gtk::Box::new(gtk::Orientation::Vertical, 14);
        lines.set_halign(gtk::Align::Center);
        lines.set_size_request(380, -1);
        lines.append(&text(
            &w.get(
                "sign_in.pair_how",
                &[
                    ("account", &l("settings.title")),
                    ("devices", &l("settings.devices.title")),
                    ("add", &l("settings.devices.pair")),
                ],
            )
            .unwrap(),
            14,
            400,
            &["tafs-dim"],
        ));
        let p = crate::widgets::panel(true);
        p.append(&crate::widgets::eyebrow(&l("sign_in.your_code"), false));
        check.set_xalign(0.0);
        p.append(&check);
        p.append(&check_line_label);
        lines.append(&p);
        wait_row.append(&text(&l("sign_in.pair_waiting"), 14, 400, &["tafs-soft"]));
        lines.append(&wait_row);
        // Shown with the QR, once there's a code.
        code_box
            .bind_property("visible", &lines, "visible")
            .sync_create()
            .build();
        content.append(&lines);
    } else {
        let title = text(
            &w.get("pair.title", &[("device", &device)]).unwrap(),
            24,
            700,
            &["tafs-ink"],
        );
        title.set_xalign(0.5);
        title.set_justify(gtk::Justification::Center);
        content.append(&title);
        code_box.append(&qr);
        code_box.append(&centred(text(&steps(), 14, 400, &["tafs-dim"])));
        code_box.append(&centred(text(
            &w.get("pair.check_code_lead", &[("device", &device)])
                .unwrap(),
            14,
            400,
            &["tafs-ink"],
        )));
        check.set_halign(gtk::Align::Center);
        check.add_css_class("tafs-check-code");
        code_box.append(&check);
        wait_row.set_halign(gtk::Align::Center);
        wait_row.append(&text(&l("pair.waiting"), 14, 400, &["tafs-soft"]));
        code_box.append(&wait_row);
        content.append(&code_box);
        content.append(&starting);
    }
    let note = centred(text("", 14, 400, &[]));
    content.append(&note);
    let again = if embedded { outline("") } else { primary("") };
    again.set_halign(gtk::Align::Center);
    again.set_hexpand(false);
    content.append(&again);

    let ui = Rc::new(Ui {
        code_box,
        qr,
        modules,
        check,
        starting,
        note,
        again,
        check_line: embedded.then_some(check_line_label),
    });
    // The attempt on screen: a new one stops the last, and closing stops it.
    let attempt: Rc<RefCell<Arc<AtomicBool>>> =
        Rc::new(RefCell::new(Arc::new(AtomicBool::new(false))));

    let begin: Rc<dyn Fn()> = {
        let (ui, attempt, w, languages, done, replaced) = (
            ui.clone(),
            attempt.clone(),
            w.clone(),
            languages.clone(),
            done.clone(),
            replaced.clone(),
        );
        Rc::new(move || {
            attempt.borrow().store(true, Ordering::SeqCst);
            let cancelled = Arc::new(AtomicBool::new(false));
            *attempt.borrow_mut() = cancelled.clone();
            show(&ui, &w, &Step::Starting);
            let (ui, w, languages, done, replaced) = (
                ui.clone(),
                w.clone(),
                languages.clone(),
                done.clone(),
                replaced.clone(),
            );
            glib::spawn_future_local(async move {
                // The keyring takes a session, or no code is shown.
                let writable = gio::spawn_blocking(|| {
                    crate::store::check_writable(chrono::Utc::now().timestamp())
                })
                .await
                .unwrap_or(Err(crate::store::StoreError("keyring")));
                if cancelled.load(Ordering::SeqCst) {
                    return;
                }
                if let Err(refused) = writable {
                    eprintln!("the keyring refused before pairing: {}", refused.0);
                    return show(&ui, &w, &Step::Failed(refused.0.to_string()));
                }
                let name = signin::device_name(&w).unwrap_or_else(|e| panic!("{e}"));
                let key = Arc::new(DeviceKey::generate());
                let nonce = pairing::nonce();
                let payload = pairing::Payload::new(&key, &name, &nonce, chrono::Utc::now());
                let code = Code {
                    text: payload.text(),
                    check: pairing::check_code(&key.public_spki_der()),
                };
                show(&ui, &w, &Step::Code(code));
                let body = pairing::claim_body(
                    &nonce,
                    &payload.pub_key,
                    &pairing::claim_proof(&key, &nonce),
                );
                let clock = Waiting {
                    started: Instant::now(),
                    cancelled: cancelled.clone(),
                };
                let words: WordBook = (*w).clone();
                let gone = cancelled.clone();
                let finished = gio::spawn_blocking(move || {
                    let token = pairing::wait(&Kiff::new(languages), &clock, &body, Some(&words))?;
                    // Closed while KIFF handed it over: nobody holds it here, so it ends at KIFF.
                    if gone.load(Ordering::SeqCst) {
                        let _ = Kiff::new(vec![]).sign_out(&token.access_token);
                        return Err(Stopped::Cancelled);
                    }
                    let now = chrono::Utc::now().timestamp();
                    // The new session and its key are kept first, view-only: a software key
                    // approves nothing (`pairing::kept`).
                    let kept = pairing::kept(&token, &key, now);
                    if let Err(refused) = crate::store::save(&kept, now) {
                        eprintln!("the keyring refused the session: {}", refused.0);
                        let _ = Kiff::new(vec![]).sign_out(&token.access_token);
                        return Err(Stopped::Failed(KiffError::Server {
                            status: 0,
                            message: refused.0.to_string(),
                        }));
                    }
                    // Then the view-only session it replaces ends at KIFF, with its own key.
                    if let Some(old) = replaced {
                        if old.token != kept.token {
                            let _ = Kiff::new(vec![]).sign_out(&old.token);
                        }
                    }
                    Ok::<KeptSession, Stopped>(kept)
                })
                .await
                .unwrap_or(Err(Stopped::Failed(KiffError::Unreadable)));
                if cancelled.load(Ordering::SeqCst) {
                    return;
                }
                match finished {
                    Ok(kept) => done(kept),
                    Err(Stopped::Cancelled) => {}
                    Err(Stopped::Failed(KiffError::Expired)) => show(&ui, &w, &Step::Expired),
                    Err(Stopped::Failed(e)) => show(&ui, &w, &Step::Failed(why(&e, &w))),
                }
            });
        })
    };
    {
        let begin = begin.clone();
        ui.again.connect_clicked(move |_| begin());
    }
    if preview {
        // A screenshot: a code made here, never claimed.
        let key = DeviceKey::generate();
        let name = signin::device_name(&w).unwrap_or_default();
        let payload = pairing::Payload::new(&key, &name, &pairing::nonce(), chrono::Utc::now());
        show(
            &ui,
            &w,
            &Step::Code(Code {
                text: payload.text(),
                check: pairing::check_code(&key.public_spki_der()),
            }),
        );
    } else {
        begin();
    }
    Pairing {
        root: content,
        attempt,
    }
}

/// The pairing alone, as a dialog over what's open, with Cancel: from the welcome, or to pair a
/// view-only computer again.
pub fn present(
    parent: &impl IsA<gtk::Widget>,
    w: Rc<WordBook>,
    languages: Vec<&'static str>,
    replaced: Option<KeptSession>,
    done: Rc<dyn Fn(KeptSession)>,
    preview: bool,
) {
    let device = w.line("device.linux").unwrap_or_default();
    let dialog = adw::Dialog::new();
    let close_and_done: Rc<dyn Fn(KeptSession)> = {
        let dialog = dialog.clone();
        Rc::new(move |kept| {
            dialog.close();
            done(kept);
        })
    };
    let pairing = Rc::new(body(
        w.clone(),
        languages,
        replaced,
        close_and_done,
        preview,
        false,
    ));
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&pairing.root));
    scroller.set_vexpand(true);

    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    let cancel = gtk::Button::with_label(&w.line("common.cancel").unwrap_or_default());
    cancel.add_css_class("flat");
    header.pack_start(&cancel);
    header.set_title_widget(Some(&gtk::Label::new(None)));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);

    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(520);
    dialog.set_content_height(680);
    dialog.set_title(
        &w.get("pair.title", &[("device", &device)])
            .unwrap_or_default(),
    );
    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog.close();
        });
    }
    {
        let pairing = pairing.clone();
        dialog.connect_closed(move |_| pairing.stop());
    }
    dialog.present(Some(parent));
}
