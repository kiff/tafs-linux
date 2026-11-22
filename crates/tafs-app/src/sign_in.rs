//! Sign in (`SignInView`, the approved redesign): two halves, as on an iPad or a Mac. On the
//! left, "Sign in with KIFF" with Continue: this computer shows a code, the person approves it in
//! KIFF in their browser, and this computer receives a session. On the right, "Sign in with your
//! phone": the pairing QR at once (`PairSignInView(embedded: true)`, `pairing.rs`). A narrow window
//! stacks them. Shown as the iPad's sheet: a dialog over the welcome, with Cancel. Either way ends
//! on the celebration (`connected.rs`). Every word is a shared key; the drawings carry no text.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::keys::DeviceKey;
use tafs_core::session::KeptSession;
use tafs_core::signin::{self, Cancelled, Clock, DeviceStart, KiffError, Stopped};
use tafs_core::words::WordBook;

use crate::kiff::Kiff;
use crate::widgets::{eyebrow, icon_for, outline, panel, primary, symbol, text};

thread_local! {
    /// A screenshot's sign-in with Continue already tapped (`--page sign-in-code`).
    pub static CONTINUED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// What the person reads while it starts, waits, or after it stopped.
struct Ui {
    code_box: gtk::Box,
    code: gtk::Label,
    link: gtk::Label,
    copy: gtk::Button,
    copy_label: gtk::Label,
    copy_icon: gtk::Image,
    starting: gtk::Spinner,
    problem: gtk::Label,
    try_again: gtk::Button,
}

/// One sign-in at a time: its cancel flag, and the nudge that asks KIFF again at once.
#[derive(Default)]
struct Run {
    cancelled: Arc<AtomicBool>,
    nudged: Arc<AtomicBool>,
}

/// `finishSignIn`'s wait: the interval in slices of half a second, cut short when the person comes
/// back to tafs from the browser (`nudgeSignIn`), or stopped when the sign-in closes.
struct Waiting {
    started: Instant,
    cancelled: Arc<AtomicBool>,
    nudged: Arc<AtomicBool>,
}

impl Clock for Waiting {
    fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
    fn wait(&self, d: Duration) -> Result<(), Cancelled> {
        let slice = Duration::from_millis(500);
        let mut waited = Duration::ZERO;
        while waited < d && !self.nudged.swap(false, Ordering::SeqCst) {
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

/// The sign-in. `done` gets the kept session once KIFF hands it over, or the phone pairs this
/// computer, and the keyring holds it. `preview`: a screenshot's, which reaches nothing: Continue
/// shows the code given, and the phone's half a code made here, never claimed.
/// A session KIFF just handed over, kept (on a worker thread): a session it replaces signs itself
/// out at KIFF first, best effort, so its device doesn't stay there out of the app's reach; then
/// the new one is kept before the window moves on, so a relaunch opens signed in. A keyring that
/// refuses ends the new session at KIFF at once: it would be gone at the next launch while its
/// device stayed at KIFF.
pub fn keep(
    token: &tafs_core::signin::DeviceToken,
    key: &DeviceKey,
) -> Result<KeptSession, KiffError> {
    let now = chrono::Utc::now().timestamp();
    if let Some(old) = crate::store::load(now) {
        if old.token != token.access_token {
            let _ = Kiff::new(vec![]).sign_out(&old.token);
        }
    }
    let kept = KeptSession::from_token(token, key, now);
    if let Err(refused) = crate::store::save(&kept, now) {
        eprintln!("the keyring refused the session: {}", refused.0);
        let _ = Kiff::new(vec![]).sign_out(&token.access_token);
        return Err(KiffError::Unreadable);
    }
    Ok(kept)
}

pub fn present(
    parent: &impl IsA<gtk::Widget>,
    window: &gtk::Window,
    w: Rc<WordBook>,
    languages: Vec<&'static str>,
    done: Rc<dyn Fn(KeptSession)>,
    preview: Option<DeviceStart>,
) {
    let l = {
        let w = w.clone();
        move |k: &str| w.line(k).unwrap_or_else(|e| panic!("{e}"))
    };
    let device = l("device.linux");

    // The usual way (`kiffHalf`): board 1 (first run), "Sign in with KIFF", Continue; then the
    // code.
    let half = gtk::Box::new(gtk::Orientation::Vertical, 24);
    half.set_margin_start(32);
    half.set_margin_end(32);
    half.set_margin_top(32);
    half.set_margin_bottom(32);
    half.append(&crate::illustrations::first_run());
    let title = text(&l("sign_in.with_kiff"), 30, 700, &["tafs-ink"]);
    title.set_xalign(0.5);
    title.set_justify(gtk::Justification::Center);
    half.append(&title);
    let continue_button = primary(&l("common.continue"));
    continue_button.set_widget_name("signIn.continue");
    continue_button.set_halign(gtk::Align::Center);
    continue_button.set_hexpand(false);
    continue_button.set_size_request(320, -1);
    half.append(&continue_button);
    // Shown once Continue is tapped, at most 420 wide.
    let content = gtk::Box::new(gtk::Orientation::Vertical, 16);
    content.set_halign(gtk::Align::Center);
    content.set_size_request(380, -1);
    content.set_visible(false);
    content.append(&text(
        &w.get("sign_in.lead", &[("device", &device)]).unwrap(),
        15,
        400,
        &["tafs-soft"],
    ));

    // The code, the button that opens KIFF, the link for another browser and the wait.
    let code_box = gtk::Box::new(gtk::Orientation::Vertical, 20);
    let card = panel(true);
    card.append(&eyebrow(&l("sign_in.your_code"), false));
    let code = gtk::Label::new(None);
    code.set_xalign(0.0);
    code.set_selectable(true);
    code.add_css_class("tafs-code");
    card.append(&code);
    card.append(&text(&l("sign_in.check_code"), 13, 400, &["tafs-dim"]));
    code_box.append(&card);
    let open = primary(&l("sign_in.open_kiff"));
    code_box.append(&open);
    let other = gtk::Box::new(gtk::Orientation::Vertical, 6);
    other.append(&text(&l("sign_in.other_browser"), 13, 400, &["tafs-dim"]));
    let link_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let link = gtk::Label::new(None);
    link.set_xalign(0.0);
    link.set_hexpand(true);
    link.set_selectable(true);
    link.set_wrap(true);
    link.set_wrap_mode(gtk::pango::WrapMode::Char);
    link.set_lines(2);
    link.set_ellipsize(gtk::pango::EllipsizeMode::Middle);
    link.add_css_class("tafs-mono");
    link.add_css_class("tafs-soft");
    link_row.append(&link);
    let copy = gtk::Button::new();
    copy.add_css_class("tafs-link");
    let copy_inner = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    let copy_icon = symbol(icon_for("doc.on.doc"), 13);
    let copy_label = gtk::Label::new(Some(&l("common.copy")));
    copy_inner.append(&copy_icon);
    copy_inner.append(&copy_label);
    copy.set_child(Some(&copy_inner));
    copy.set_valign(gtk::Align::Start);
    link_row.append(&copy);
    other.append(&link_row);
    code_box.append(&other);
    let wait_row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let spinner = gtk::Spinner::new();
    spinner.start();
    wait_row.append(&spinner);
    wait_row.append(&text(&l("sign_in.waiting"), 14, 400, &["tafs-soft"]));
    code_box.append(&wait_row);
    content.append(&code_box);

    let starting = gtk::Spinner::new();
    starting.set_halign(gtk::Align::Center);
    content.append(&starting);
    half.append(&content);

    let problem = text("", 14, 400, &["tafs-no"]);
    problem.set_xalign(0.5);
    problem.set_justify(gtk::Justification::Center);
    half.append(&problem);
    let try_again = outline(&l("common.try_again"));
    try_again.set_halign(gtk::Align::Center);
    try_again.set_hexpand(false);
    half.append(&try_again);
    let terms = agreement(&w);
    terms.set_xalign(0.5);
    terms.set_justify(gtk::Justification::Center);
    terms.set_halign(gtk::Align::Center);
    terms.set_size_request(380, -1);
    half.append(&terms);

    // The other way (`phoneHalf`): this computer's pairing QR, scanned and approved from the
    // person's phone, on the raised colour.
    let dialog = adw::Dialog::new();
    let finished: Rc<dyn Fn(KeptSession)> = {
        let (dialog, done) = (dialog.clone(), done.clone());
        Rc::new(move |kept| {
            dialog.close();
            done(kept);
        })
    };
    let phone = Rc::new(crate::pairing::body(
        w.clone(),
        languages.clone(),
        None,
        finished,
        preview.is_some(),
        true,
    ));
    let phone_half = gtk::Box::new(gtk::Orientation::Vertical, 0);
    phone_half.add_css_class("tafs-raised");
    phone_half.append(&phone.root);

    // Side by side, each half as wide as the other; a narrow window stacks them.
    let halves = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    halves.set_homogeneous(true);
    half.set_valign(gtk::Align::Start);
    halves.append(&half);
    halves.append(&phone_half);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&halves));
    scroller.set_vexpand(true);
    let bin = adw::BreakpointBin::new();
    bin.set_size_request(320, 400);
    bin.set_child(Some(&scroller));
    let narrow = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        // Below both halves' least (each 380 wide inside 32 on each side), with room to spare.
        920.0,
        adw::LengthUnit::Px,
    ));
    narrow.add_setter(
        &halves,
        "orientation",
        Some(&gtk::Orientation::Vertical.to_value()),
    );
    narrow.add_setter(&halves, "homogeneous", Some(&false.to_value()));
    bin.add_breakpoint(narrow);

    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    let cancel = gtk::Button::with_label(&l("common.cancel"));
    cancel.add_css_class("flat");
    header.pack_start(&cancel);
    // The sheet has no title of its own: the screen's large title says it.
    header.set_title_widget(Some(&gtk::Label::new(None)));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&bin));
    crate::widgets::ground(&toolbar);

    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(1000);
    dialog.set_content_height(720);
    dialog.set_title(&l("sign_in.with_kiff"));

    let ui = Rc::new(Ui {
        code_box,
        code,
        link,
        copy,
        copy_label,
        copy_icon,
        starting,
        problem,
        try_again,
    });
    let run: Rc<RefCell<Run>> = Rc::new(RefCell::new(Run::default()));
    let current: Rc<RefCell<Option<DeviceStart>>> = Rc::new(RefCell::new(None));

    let begin: Rc<dyn Fn()> = {
        let (ui, run, current, w, languages, done, dialog) = (
            ui.clone(),
            run.clone(),
            current.clone(),
            w.clone(),
            languages.clone(),
            done.clone(),
            dialog.clone(),
        );
        Rc::new(move || {
            // A new start: nothing shown from the last, and the last one stopped.
            run.borrow().cancelled.store(true, Ordering::SeqCst);
            let fresh = Run::default();
            let (cancelled, nudged) = (fresh.cancelled.clone(), fresh.nudged.clone());
            *run.borrow_mut() = fresh;
            *current.borrow_mut() = None;
            show(&ui, &w, None, None);
            let (ui, current, w, languages, done, dialog) = (
                ui.clone(),
                current.clone(),
                w.clone(),
                languages.clone(),
                done.clone(),
                dialog.clone(),
            );
            glib::spawn_future_local(async move {
                let language = w.language().to_string();
                let name = signin::device_name(&w).unwrap_or_else(|e| panic!("{e}"));
                let key = Arc::new(DeviceKey::generate());
                let body = signin::start_body(&key.public_spki_b64(), &name);
                let words: WordBook = (*w).clone();
                let langs = languages.clone();
                let started = gio::spawn_blocking(move || {
                    signin::start(&Kiff::new(langs), &body, &language, Some(&words))
                })
                .await
                .unwrap_or(Err(KiffError::Unreadable));
                if cancelled.load(Ordering::SeqCst) {
                    return;
                }
                let start = match started {
                    Ok(s) => s,
                    Err(e) => return show(&ui, &w, None, Some(e.words(&w).unwrap_or_default())),
                };
                *current.borrow_mut() = Some(start.clone());
                show(&ui, &w, Some(&start), None);
                let clock = Waiting {
                    started: Instant::now(),
                    cancelled: cancelled.clone(),
                    nudged,
                };
                let words: WordBook = (*w).clone();
                let langs = languages.clone();
                let token_key = key.clone();
                let gone = cancelled.clone();
                let finished = gio::spawn_blocking(move || {
                    let token = signin::finish(&Kiff::new(langs), &clock, &start, Some(&words))?;
                    // Closed while KIFF handed it over: nobody is waiting for it here, so it's
                    // never kept, and it ends at KIFF rather than staying there unseen.
                    if gone.load(Ordering::SeqCst) {
                        let _ = Kiff::new(vec![]).sign_out(&token.access_token);
                        return Err(Stopped::Cancelled);
                    }
                    keep(&token, &token_key).map_err(Stopped::Failed)
                })
                .await
                .unwrap_or(Err(Stopped::Failed(KiffError::Unreadable)));
                if cancelled.load(Ordering::SeqCst) {
                    return;
                }
                match finished {
                    Ok(kept) => {
                        dialog.close();
                        done(kept);
                    }
                    Err(Stopped::Cancelled) => {}
                    Err(Stopped::Failed(e)) => {
                        *current.borrow_mut() = None;
                        show(&ui, &w, None, Some(e.words(&w).unwrap_or_default()));
                    }
                }
            });
        })
    };

    {
        let (current, window) = (current.clone(), window.clone());
        open.connect_clicked(move |_| {
            if let Some(start) = current.borrow().as_ref() {
                if signin::is_kiff_link(&start.verification_uri_complete) {
                    gtk::UriLauncher::new(&start.verification_uri_complete).launch(
                        Some(&window),
                        gio::Cancellable::NONE,
                        |_| {},
                    );
                }
            }
        });
    }
    {
        let (current, ui, w) = (current.clone(), ui.clone(), w.clone());
        ui.clone().copy.connect_clicked(move |b| {
            if let Some(start) = current.borrow().as_ref() {
                b.clipboard().set_text(&start.verification_uri_complete);
                ui.copy_label
                    .set_label(&w.line("common.copied").unwrap_or_default());
                ui.copy_icon.set_icon_name(Some(icon_for("checkmark")));
            }
        });
    }
    {
        let begin = begin.clone();
        ui.try_again.connect_clicked(move |_| begin());
    }
    {
        let dialog = dialog.clone();
        cancel.connect_clicked(move |_| {
            dialog.close();
        });
    }
    {
        // Closing stops both ways: this sign-in's polling and the pairing's claims, as the iPad's
        // `task?.cancel()`.
        let (run, phone) = (run.clone(), phone.clone());
        dialog.connect_closed(move |_| {
            run.borrow().cancelled.store(true, Ordering::SeqCst);
            phone.stop();
        });
    }
    // Back from approving in the browser: ask KIFF at once.
    {
        let run = run.clone();
        window.connect_is_active_notify(move |win| {
            if win.is_active() {
                run.borrow().nudged.store(true, Ordering::SeqCst);
            }
        });
    }

    // One way at a time: starting the phone's again stops KIFF's code and puts Continue back.
    {
        let (run, current, ui, w, content, continue_button) = (
            run.clone(),
            current.clone(),
            ui.clone(),
            w.clone(),
            content.clone(),
            continue_button.clone(),
        );
        phone.connect_restarted(move || {
            run.borrow().cancelled.store(true, Ordering::SeqCst);
            *current.borrow_mut() = None;
            show(&ui, &w, None, None);
            ui.starting.set_visible(false);
            content.set_visible(false);
            continue_button.set_visible(true);
        });
    }

    // Continue: the phone's way stops, its claim with it, then the code is asked for and shown
    // (`begin`).
    {
        let (content, current, ui, w, begin, phone) = (
            content.clone(),
            current.clone(),
            ui.clone(),
            w.clone(),
            begin.clone(),
            phone.clone(),
        );
        continue_button.connect_clicked(move |b| {
            phone.pause();
            b.set_visible(false);
            content.set_visible(true);
            match &preview {
                // A screenshot: a code as KIFF would show it, without reaching KIFF.
                Some(start) => {
                    *current.borrow_mut() = Some(start.clone());
                    show(&ui, &w, Some(start), None);
                }
                None => begin(),
            }
        });
    }
    show(&ui, &w, None, None);
    ui.starting.set_visible(false);
    dialog.present(Some(parent));
    if CONTINUED.with(|c| c.replace(false)) {
        continue_button.emit_clicked();
    }
}

/// The code a screenshot shows, shaped as KIFF's answer.
pub fn preview(language: &str) -> DeviceStart {
    serde_json::from_value::<DeviceStart>(serde_json::json!({
        "device_code": "",
        "user_code": "WDJB-MJHT",
        "verification_uri": "https://app.kiff.dev/device",
        "verification_uri_complete": "https://app.kiff.dev/device?code=WDJB-MJHT",
        "expires_in": 600,
        "interval": 5
    }))
    .expect("a start")
    .in_app_language(language)
}

/// What shows: the code once KIFF answered, a spinner before it, the problem and Try again after.
fn show(ui: &Ui, w: &WordBook, start: Option<&DeviceStart>, problem: Option<String>) {
    ui.code_box.set_visible(start.is_some());
    if let Some(start) = start {
        ui.code.set_label(&start.user_code);
        ui.link.set_label(&start.verification_uri_complete);
    }
    ui.copy_label
        .set_label(&w.line("common.copy").unwrap_or_default());
    ui.copy_icon.set_icon_name(Some(icon_for("doc.on.doc")));
    let waiting_for_kiff = start.is_none() && problem.is_none();
    ui.starting.set_visible(waiting_for_kiff);
    ui.starting.set_spinning(waiting_for_kiff);
    ui.problem.set_visible(problem.is_some());
    ui.problem.set_label(problem.as_deref().unwrap_or(""));
    ui.try_again.set_visible(problem.is_some());
}

/// "By signing in, you agree to KIFF's Terms … and its Privacy policy.", the two names linked
/// wherever the language puts them, to the pages in the app's language where kiff.dev has them,
/// else in English, saying so (`Legal.agreement`, `Legal.label`).
pub fn agreement(w: &WordBook) -> gtk::Label {
    let label_of = |key: &str| {
        tafs_core::account::legal_label(w, key).unwrap_or_else(|_| w.line(key).unwrap_or_default())
    };
    let line = w
        .get("sign_in.agree", &[("terms", "\u{1}"), ("privacy", "\u{2}")])
        .unwrap_or_default();
    let mut markup = String::new();
    for c in line.chars() {
        match c {
            '\u{1}' => markup.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                tafs_core::account::terms_url(w.language()),
                glib::markup_escape_text(&label_of("settings.terms"))
            )),
            '\u{2}' => markup.push_str(&format!(
                "<a href=\"{}\">{}</a>",
                tafs_core::account::privacy_url(w.language()),
                glib::markup_escape_text(&label_of("settings.privacy_policy"))
            )),
            c => markup.push_str(&glib::markup_escape_text(&c.to_string())),
        }
    }
    let label = gtk::Label::new(None);
    label.set_markup(&format!("<span font=\"12px\">{markup}</span>"));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label.add_css_class("tafs-dim");
    label
}
