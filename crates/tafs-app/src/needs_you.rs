//! Needs you beside the sidebar (`NeedsYouView`): the tafs' steps, Coach's waits, "Did this
//! happen?" (`DidThisHappenCard`), and each held call's card (`HeldCallCard`), or "Nothing is
//! waiting." with its hint.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::{AdwDialogExt, AlertDialogExt};
use gtk::prelude::*;
use gtk::{gio, glib};
use tafs_core::held::remaining;
use tafs_core::needs_you::{Answers, CardLines, NeedsYouLines};
use tafs_core::session::KeptSession;
use tafs_core::sidebar::Mode;
use tafs_core::signin::is_kiff_link;
use tafs_core::words::WordBook;

use crate::widgets::{avatar, outline, panel, primary, text};

pub fn page(
    w: Rc<WordBook>,
    n: &NeedsYouLines,
    open: Rc<dyn Fn(tafs_core::sidebar::Selection)>,
    aside: Option<Rc<SetAsideSource>>,
) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let title = text(&n.title, 34, 700, &["tafs-ink"]);
    title.set_width_chars(16);
    column.append(&title);
    // In the iPad's order: the steps the tafs handed the person, then Coach's chats waiting on them.
    for step in &n.steps {
        column.append(&step_card(&w, step));
    }
    // "Set all aside" above them, then each chat: what's set aside goes once tafs cloud says so.
    let rows: Rc<RefCell<Vec<(String, gtk::Box)>>> = Rc::default();
    if let Some(clear) = &n.set_all_aside {
        let b = outline(&clear.button);
        b.add_css_class("tafs-amber-text");
        b.set_widget_name("needs-you.coach-waiting.set-all-aside");
        let (w, clear, aside, rows) = (w.clone(), clear.clone(), aside.clone(), rows.clone());
        b.connect_clicked(move |b| ask_set_all_aside(&w, &clear, aside.clone(), rows.clone(), b));
        column.append(&b);
        tap_for_screenshot("set-all-aside", &b);
    }
    for waiting in &n.coach {
        let card = coach_card(&w, waiting, n.opens_coach, aside.clone());
        rows.borrow_mut()
            .push((waiting.session.clone(), card.clone()));
        column.append(&card);
    }
    // Money calls KIFF can't prove: the person says on the phone whether they happened.
    for happened in &n.happened {
        column.append(&happened_card(happened));
    }
    // A hold opened from its notification is brought into view (`requestedHold`): its card, or
    // its Card unit request.
    let mut by_id: Vec<(String, gtk::Widget)> = Vec::new();
    for ask in &n.card_units {
        let panel = card_unit_panel(&w, ask);
        by_id.push((ask.id.clone(), panel.clone().upcast()));
        column.append(&panel);
    }
    if let Some((nothing, hint)) = &n.empty {
        // Board 6, nothing waiting (`Illustration.nothingWaiting`), above its words.
        column.append(&crate::illustrations::nothing_waiting());
        let p = panel(false);
        p.append(&text(nothing, 15, 600, &["tafs-ink"]));
        p.append(&text(hint, 14, 400, &["tafs-soft"]));
        column.append(&p);
    }
    for card in &n.cards {
        let c = held_card(&w, card);
        by_id.push((card.id.clone(), c.clone().upcast()));
        column.append(&c);
    }
    // A knock at a taf's door waits for the owner too: its row opens the taf, on its Activity,
    // where the knocks are answered.
    for knock in &n.knocks {
        column.append(&knock_row(knock, open.clone()));
    }
    // The pane's full width, 16 from each edge, as the iPad's.
    column.set_valign(gtk::Align::Start);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    crate::widgets::ground(&scroller);
    if let Some(card) = REQUESTED
        .with(|r| r.borrow().clone())
        .and_then(|id| by_id.into_iter().find(|(i, _)| *i == id))
        .map(|(_, c)| c)
    {
        scroll_to(&scroller, &column, card);
    }
    scroller.upcast()
}

thread_local! {
    /// The hold whose notification was clicked (`requestedHold`), until a Needs you that lists it
    /// is on screen: its id as KIFF keeps it.
    static REQUESTED: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A hold's notification clicked: Needs you brings it into view once it lists it.
pub fn request_hold(id: &str) {
    REQUESTED.with(|r| *r.borrow_mut() = Some(id.to_string()));
}

/// `card` at the top of `scroller`, once it's on screen; the request is then done. A page
/// replaced before then leaves it for the next.
fn scroll_to(scroller: &gtk::ScrolledWindow, column: &gtk::Box, card: gtk::Widget) {
    let (scroller, column) = (scroller.clone(), column.clone());
    glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
        if !card.is_mapped() || scroller.vadjustment().upper() <= 0.0 {
            // Not drawn yet, or no longer: a replaced page is never mapped again.
            return if scroller.parent().is_some() {
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            };
        }
        if let Some(bounds) = card.compute_bounds(&column) {
            scroller.vadjustment().set_value(f64::from(bounds.y()));
        }
        REQUESTED.with(|r| *r.borrow_mut() = None);
        glib::ControlFlow::Break
    });
}

/// `YourStepCard`: tafs cloud's title, the taf, what and why, the vendor's page when tafs cloud
/// vouched for its link (it opens in the browser, with where it really goes under it), and Done or
/// Skip it: the answer is signed by the device key, so their tap says this computer can't approve,
/// under them, and sends nothing.
fn step_card(w: &Rc<WordBook>, step: &tafs_core::waits::YourStep) -> gtk::Box {
    let p = panel(true);
    p.set_spacing(10);
    if !step.title.is_empty() {
        p.append(&crate::widgets::eyebrow(&step.title, false));
    }
    if let Some((_, name)) = step.taf.as_ref().filter(|(_, n)| !n.is_empty()) {
        p.append(&text(name, 14, 600, &["tafs-ink"]));
    }
    p.append(&text(&step.what, 16, 600, &["tafs-ink"]));
    if !step.why.is_empty() {
        p.append(&text(&step.why, 14, 400, &["tafs-soft"]));
    }
    if let Some((url, host, words)) = step.open_link() {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let open = outline(&words);
        open.connect_clicked(move |b| {
            let launcher = gtk::UriLauncher::new(&url);
            let window = b.root().and_downcast::<gtk::Window>();
            launcher.launch(window.as_ref(), gtk::gio::Cancellable::NONE, |_| {});
        });
        v.append(&open);
        v.append(&text(&host, 12, 400, &["tafs-mono", "tafs-dim"]));
        p.append(&v);
    }
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_homogeneous(true);
    let slot = crate::approval::refusal_slot();
    let done = crate::widgets::primary(&step.done);
    row.append(&done);
    let skip = outline(&step.skip);
    row.append(&skip);
    for b in [&done, &skip] {
        let (w, slot, handoff) = (w.clone(), slot.clone(), step.handoff.clone());
        b.connect_clicked(move |_| {
            crate::approval::refuse(
                &w,
                tafs_core::sidebar::Mode::Kiff,
                &tafs_core::signed::Act::AnswerStep {
                    handoff: handoff.clone(),
                },
                &slot,
            )
        });
    }
    p.append(&row);
    p.append(&slot);
    p
}

/// Where Coach's waiting chats are set aside, signed in with KIFF: the session, and Home read
/// again once tafs cloud took it.
pub struct SetAsideSource {
    pub kept: KeptSession,
    pub languages: Vec<&'static str>,
    pub refresh: Rc<dyn Fn()>,
    /// tafs cloud ended the session: signed out, as every read does.
    pub signed_out: Rc<dyn Fn()>,
}

/// `setAsideCoach`: one waiting chat, or every one with none, set aside at tafs cloud. Nothing is
/// signed; this device's signed read of its connections goes with it when it has one. `done` has
/// the chats tafs cloud ended, or why not in its words; Home is read again once it took it.
fn set_aside(
    w: &Rc<WordBook>,
    source: &Rc<SetAsideSource>,
    session: Option<String>,
    done: impl FnOnce(Result<Vec<String>, String>) + 'static,
) {
    let (words, source) = ((**w).clone(), source.clone());
    let (kept, languages) = (source.kept.clone(), source.languages.clone());
    glib::spawn_future_local(async move {
        let sent = gio::spawn_blocking(move || -> Result<Vec<String>, Option<String>> {
            let proof = crate::coach::this_device(&kept, &languages)
                .and_then(|d| crate::coach::proof(&kept, &d));
            let path = tafs_core::waits::set_aside_path(session.as_deref());
            let a = crate::kiff::set_aside(&kept.token, &languages, &path, proof.as_deref())
                .map_err(|e| Some(e.words(&words).unwrap_or_default()))?;
            match a.status {
                401 => Err(None),
                200..=299 => tafs_core::waits::read_set_aside(&a.body)
                    .ok_or_else(|| tafs_core::signin::plain_words(&words, 200).ok()),
                _ => Err(Some(
                    tafs_core::signin::refusal(&a, Some(&words))
                        .words(&words)
                        .unwrap_or_default(),
                )),
            }
        })
        .await
        .unwrap_or(Err(Some(String::new())));
        match sent {
            Err(None) => (source.signed_out)(),
            Err(Some(why)) => done(Err(why)),
            Ok(ended) => {
                done(Ok(ended));
                (source.refresh)();
            }
        }
    });
}

/// "Set all aside"'s one ask (`SetAllAsideButton`): tafs cloud's title, its words, and its yes
/// (destructive) and no, else Cancel. Yes sets every listed chat aside.
fn ask_set_all_aside(
    w: &Rc<WordBook>,
    clear: &tafs_core::waits::CoachWaitingClear,
    aside: Option<Rc<SetAsideSource>>,
    rows: Rc<RefCell<Vec<(String, gtk::Box)>>>,
    button: &gtk::Button,
) {
    let ask = adw::AlertDialog::new(Some(&clear.title), None);
    if !clear.text.is_empty() {
        ask.set_body(&clear.text);
    }
    let no = if clear.no.is_empty() {
        w.line("common.cancel").unwrap_or_default()
    } else {
        clear.no.clone()
    };
    ask.add_response("no", &no);
    ask.add_response("yes", &clear.yes);
    ask.set_response_appearance("yes", adw::ResponseAppearance::Destructive);
    ask.set_default_response(Some("no"));
    ask.set_close_response("no");
    let (w, b) = (w.clone(), button.clone());
    ask.connect_response(None, move |_, answer| {
        // A screenshot's chats: nothing is sent.
        let (true, Some(source)) = (answer == "yes", aside.clone()) else {
            return;
        };
        b.set_sensitive(false);
        for (_, row) in rows.borrow().iter() {
            row.set_sensitive(false);
        }
        let (b, rows) = (b.clone(), rows.clone());
        set_aside(&w, &source, None, move |r| {
            b.set_sensitive(true);
            for (session, row) in rows.borrow().iter() {
                row.set_sensitive(true);
                if r.as_ref().is_ok_and(|ended| ended.contains(session)) {
                    row.set_visible(false);
                }
            }
            match r {
                Ok(_) => b.set_visible(false),
                Err(why) => b.set_tooltip_text(Some(&why)),
            }
        });
    });
    ask.present(Some(button));
}

/// `CoachWaitingCard`: tafs cloud's line with Coach's mark, then Open Coach, which opens Coach on
/// that chat, for someone who may use Coach; and Set aside, when tafs cloud sent its words, which
/// sets the chat aside at tafs cloud: the row goes once tafs cloud says so, or says why it stays.
fn coach_card(
    w: &Rc<WordBook>,
    waiting: &tafs_core::waits::CoachWaiting,
    opens_coach: bool,
    aside: Option<Rc<SetAsideSource>>,
) -> gtk::Box {
    let p = panel(true);
    p.set_widget_name(&format!("needs-you.coach-waiting.{}", waiting.session));
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let mark = crate::widgets::symbol("tafs-sparkle-symbolic", 15);
    mark.add_css_class("tafs-amber-text");
    line.append(&mark);
    let words = text(&waiting.line, 14, 600, &["tafs-ink"]);
    words.set_hexpand(true);
    line.append(&words);
    p.append(&line);
    // Its way in only for someone who may use Coach: else the line alone.
    if opens_coach {
        let open = crate::widgets::primary(&waiting.open);
        let session = waiting.session.clone();
        open.connect_clicked(move |_| crate::coach_start::finish(&session));
        p.append(&open);
    }
    if !waiting.set_aside.is_empty() && !waiting.session.is_empty() {
        let why = text("", 13, 400, &["tafs-no"]);
        why.set_visible(false);
        let b = outline(&waiting.set_aside);
        b.add_css_class("tafs-soft");
        let (w, session, card, why2) = (w.clone(), waiting.session.clone(), p.clone(), why.clone());
        b.connect_clicked(move |b| {
            // A screenshot's chat: nothing is sent.
            let Some(source) = aside.clone() else {
                return;
            };
            b.set_sensitive(false);
            let (b, card, why, session2) = (b.clone(), card.clone(), why2.clone(), session.clone());
            set_aside(&w, &source, Some(session.clone()), move |r| {
                b.set_sensitive(true);
                match r {
                    Ok(ended) if ended.contains(&session2) => card.set_visible(false),
                    Ok(_) => {}
                    Err(words) => {
                        why.set_label(&words);
                        why.set_visible(!words.is_empty());
                    }
                }
            });
        });
        p.append(&b);
        p.append(&why);
    }
    p
}

/// `DidThisHappenCard`: an amber-edged panel with "Did this happen?" in amber, KIFF's words for
/// the call (or its title and amount), why KIFF asks, and where No and Yes would be, that this
/// computer can't approve (`ApprovalGate`, one for both): both answers are signed like an approval.
fn happened_card(h: &tafs_core::unproven::HappenedLines) -> gtk::Box {
    let p = panel(true);
    p.set_widget_name(&format!("happened.{}", h.proposal));
    p.append(&text(&h.title, 13, 600, &["tafs-amber-text"]));
    p.append(&text(&h.headline, 15, 600, &["tafs-ink"]));
    if let Some(amount) = &h.amount {
        p.append(&text(amount, 15, 400, &["tafs-ink"]));
    }
    if let Some(why) = &h.why {
        p.append(&text(why, 13, 400, &["tafs-soft"]));
    }
    p.append(&crate::approval::not_ready(&h.can_not_answer));
    p
}

/// The knocked row (`NeedsYouView`): an amber-edged panel with the taf's avatar (32), its name
/// (14 semibold), how many knocks (14, soft) and a chevron; it opens the taf's page, whose
/// Activity opens first.
fn knock_row(
    k: &tafs_core::needs_you::KnockLines,
    open: Rc<dyn Fn(tafs_core::sidebar::Selection)>,
) -> gtk::Button {
    let p = panel(true);
    let line = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    line.append(&avatar(&k.initial, k.color, 32));
    let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
    words.set_hexpand(true);
    words.append(&text(&k.name, 14, 600, &["tafs-ink"]));
    words.append(&text(&k.line, 14, 400, &["tafs-soft"]));
    line.append(&words);
    let chevron = crate::widgets::symbol("go-next-symbolic", 13);
    chevron.add_css_class("tafs-dim");
    line.append(&chevron);
    p.append(&line);
    let button = gtk::Button::new();
    button.set_child(Some(&p));
    button.add_css_class("tafs-plain");
    button.set_widget_name(&format!("knocks.{}", k.key));
    button.set_tooltip_text(Some(&k.hint));
    button.update_property(&[gtk::accessible::Property::Description(&k.hint)]);
    let key = k.key.clone();
    button.connect_clicked(move |_| open(tafs_core::sidebar::Selection::Taf(key.clone())));
    button
}

/// `CardUnitExceptionPanel`: tafs cloud's title (16 semibold), words (14, soft) and history (12,
/// dim); its Coach button for someone who may use Coach and manages the taf, which opens Coach in
/// a new conversation with tafs cloud's words in its box, nothing sent; and Dismiss in red, signed
/// like a refusal, so its tap says this computer can't approve, under it, and sends nothing.
pub fn card_unit_panel(w: &Rc<WordBook>, ask: &tafs_core::needs_you::CardUnitLines) -> gtk::Box {
    let p = panel(false);
    p.set_widget_name(&format!("card-unit.{}", ask.id));
    p.append(&text(&ask.title, 16, 600, &["tafs-ink"]));
    p.append(&text(&ask.words, 14, 400, &["tafs-soft"]));
    p.append(&text(&ask.history, 12, 400, &["tafs-dim"]));
    if let Some(discuss) = &ask.discuss {
        let b = gtk::Button::with_label(discuss);
        b.add_css_class("tafs-link");
        b.set_halign(gtk::Align::Start);
        // `discussCardUnit`: Coach on its taf, its words in the box.
        let (taf, say) = (ask.taf.clone(), ask.coach_text.clone());
        b.connect_clicked(move |_| crate::coach_start::improve(&taf, Some(&say)));
        p.append(&b);
    }
    let slot = crate::approval::refusal_slot();
    let dismiss = gtk::Button::with_label(&ask.dismiss);
    dismiss.add_css_class("tafs-link");
    dismiss.add_css_class("tafs-no");
    dismiss.set_halign(gtk::Align::Start);
    dismiss.set_widget_name(&format!("card-unit.dismiss.{}", ask.id));
    {
        let (w, slot, exception) = (w.clone(), slot.clone(), ask.id.clone());
        dismiss.connect_clicked(move |_| {
            crate::approval::refuse(
                &w,
                Mode::Kiff,
                &tafs_core::signed::Act::DismissCardUnit {
                    exception: exception.clone(),
                },
                &slot,
            )
        });
    }
    p.append(&dismiss);
    p.append(&slot);
    tap_for_screenshot("dismiss", &dismiss);
    p
}

/// `HeldCallCard`: an amber-edged panel with the taf, its countdown, the amount, what it asks,
/// why it's held, the taf's own words, its details, and what can be done.
pub fn held_card(w: &Rc<WordBook>, c: &CardLines) -> gtk::Box {
    let p = panel(true);
    let face = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    if let Some((initial, color)) = &c.avatar {
        head.append(&avatar(initial, *color, 32));
    }
    let holder = text(&c.holder, 14, 600, &["tafs-ink"]);
    holder.set_hexpand(true);
    head.append(&holder);
    let left = gtk::Label::new(Some(&c.remaining));
    left.add_css_class("tafs-mono");
    left.add_css_class("tafs-amber-text");
    left.add_css_class("tafs-countdown");
    head.append(&left);
    // Counted down every second, wherever the card is; it stops once the card is gone.
    {
        let (w, expires, weak) = (w.clone(), c.expires_at, left.downgrade());
        gtk::glib::timeout_add_seconds_local(1, move || {
            let Some(left) = weak.upgrade() else {
                return gtk::glib::ControlFlow::Break;
            };
            if let Ok(words) = remaining(&w, expires, chrono::Local::now()) {
                left.set_label(&words);
            }
            gtk::glib::ControlFlow::Continue
        });
    }
    face.append(&head);
    if let Some(amount) = &c.amount {
        face.append(&text(amount, 44, 700, &["tafs-ink"]));
    }
    face.append(&text(&c.summary, 16, 600, &["tafs-ink"]));
    if !c.context.is_empty() {
        face.append(&text(&c.context, 14, 400, &["tafs-soft"]));
    }
    let chip = gtk::Label::new(Some(&c.reason));
    chip.add_css_class("tafs-chip");
    chip.set_halign(gtk::Align::Start);
    chip.set_wrap(true);
    chip.set_xalign(0.0);
    face.append(&chip);
    // One sentence for a screen reader: the taf, what it asks, when it lapses.
    face.update_property(&[gtk::accessible::Property::Label(&c.sentence)]);
    p.append(&face);

    // The taf's own words, drawn verbatim under their own heading, never in place of KIFF's.
    if let Some((heading, lines)) = &c.taf_says {
        let b = gtk::Box::new(gtk::Orientation::Vertical, 4);
        b.add_css_class("tafs-taf-says");
        b.append(&text(heading, 12, 600, &["tafs-soft"]));
        for (i, line) in lines.iter().enumerate() {
            let l = gtk::Label::new(Some(line));
            l.set_use_markup(false);
            l.set_xalign(0.0);
            l.set_wrap(true);
            l.add_css_class("tafs-ink");
            l.add_css_class(if i == 0 { "taf-about" } else { "taf-reason" });
            b.append(&l);
        }
        p.append(&b);
    }
    if let Some((heading, lines)) = &c.details {
        let e = gtk::Expander::new(Some(heading));
        e.add_css_class("tafs-details");
        let b = gtk::Box::new(gtk::Orientation::Vertical, 4);
        b.set_margin_top(4);
        for line in lines {
            let l = gtk::Label::new(Some(line));
            l.set_xalign(0.0);
            l.set_selectable(true);
            l.set_wrap(true);
            l.add_css_class("tafs-mono");
            l.add_css_class("tafs-soft");
            b.append(&l);
        }
        e.set_child(Some(&b));
        p.append(&e);
    }

    match &c.answers {
        Answers::Expired(line) => p.append(&text(line, 14, 400, &["tafs-soft"])),
        Answers::SomeoneAnswers { nothing_sent, line } => {
            p.append(&text(nothing_sent, 14, 400, &["tafs-soft"]));
            p.append(&text(line, 13, 400, &["tafs-soft"]));
        }
        Answers::InKiff {
            nothing_sent,
            line,
            button,
            page,
        } => {
            p.append(&text(nothing_sent, 14, 400, &["tafs-soft"]));
            p.append(&text(line, 13, 400, &["tafs-soft"]));
            let b = primary(button);
            let page = page.clone();
            b.connect_clicked(move |b| {
                if is_kiff_link(&page) {
                    let window = b.root().and_then(|r| r.downcast::<gtk::Window>().ok());
                    gtk::UriLauncher::new(&page).launch(
                        window.as_ref(),
                        gtk::gio::Cancellable::NONE,
                        |_| {},
                    );
                }
            });
            p.append(&b);
        }
        Answers::Here {
            nothing_sent,
            approve,
            over_total,
            next_steps,
            refuse,
        } => {
            p.append(&text(nothing_sent, 14, 400, &["tafs-soft"]));
            // The sample's answers come with the sample's Card checks: until then they're there, dimmed.
            if let Some(approve) = approve {
                let b = primary(approve);
                b.set_sensitive(false);
                p.append(&b);
            }
            if let Some(line) = over_total {
                p.append(&text(line, 13, 400, &["tafs-soft"]));
            }
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row.set_homogeneous(true);
            for step in next_steps {
                row.append(&outline(step));
            }
            let r = outline(refuse);
            r.add_css_class("refuse");
            row.append(&r);
            row.set_sensitive(false);
            p.append(&row);
        }
    }
    p
}

thread_local! {
    /// What a screenshot taps once Needs you is drawn: "Set all aside" (`set-all-aside`), or the
    /// first Card unit request's Dismiss (`dismiss`).
    pub static OPEN: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

/// Taps `b` once it's drawn, when a screenshot asked for `which`: the one on screen, since the
/// page can be drawn again before then.
fn tap_for_screenshot(which: &'static str, b: &gtk::Button) {
    if OPEN.with(|o| o.get()) == Some(which) {
        let b = b.clone();
        // Brought into view first, as the person would scroll to it.
        glib::timeout_add_local(std::time::Duration::from_millis(150), move || {
            if OPEN.with(|o| o.get()) != Some(which) || b.root().is_none() {
                return glib::ControlFlow::Break;
            }
            if !b.is_mapped() {
                return glib::ControlFlow::Continue;
            }
            OPEN.with(|o| o.set(None));
            b.grab_focus();
            b.emit_clicked();
            // What the tap drew under it, in view too: the card's foot at the screen's foot.
            let b = b.clone();
            glib::timeout_add_local_once(std::time::Duration::from_millis(450), move || {
                let (Some(card), Some(sw)) =
                    (b.parent(), b.ancestor(gtk::ScrolledWindow::static_type()))
                else {
                    return;
                };
                let sw = sw
                    .downcast::<gtk::ScrolledWindow>()
                    .expect("a scrolled window");
                // Measured on what scrolls (the column in the viewport), not on the viewport.
                let content = sw.child().and_then(|v| v.first_child());
                let Some(bounds) = content.and_then(|c| card.compute_bounds(&c)) else {
                    return;
                };
                let adj = sw.vadjustment();
                let foot = f64::from(bounds.y() + bounds.height()) + 16.0;
                adj.set_value((foot - adj.page_size()).max(adj.value()));
            });
            glib::ControlFlow::Break
        });
    }
}
