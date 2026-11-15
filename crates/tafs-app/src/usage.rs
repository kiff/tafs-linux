//! Plan & usage beside the sidebar (`UsageView`): tafs cloud's balance (`BalancePanel`), then the
//! credits the tafs used (`CreditsPanel`), this period or the one before, each taf's row opening
//! its runs (`TafCreditsView`, `CreditsRunRow`). Every line is tafs cloud's words, as it writes
//! them: the screen adds nothing but their order. The reads are in `tafs-core/src/usage.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::usage::{
    answered, credits_path, sample_balance, sample_credits, taf_credits_path, tone, Balance,
    Credits, TafCredits, Tone, BALANCE_PATH,
};
use tafs_core::usage::{
    asked_line, asks, billing_line, last_answer, open_requests, request_line, solo_still_renews,
    waiting, KiffUsage,
};
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, one_line, outline, panel, symbol, text};

/// Where Plan & usage is read from.
#[derive(Clone)]
pub enum UsageSource {
    /// The sample's words (`sampleBalance`, `sampleCredits`).
    Sample,
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
        /// `managesBilling`: KIFF's `billing.manage`, none said counting as allowed.
        manages_billing: bool,
    },
    /// A file of synthetic answers (`--usage`, the screenshots signed in with KIFF): its
    /// `balance`, `credits` and `usage`, and whether the person manages billing.
    File(std::path::PathBuf),
    /// Nothing to read (signed out, or a screenshot of the contracts): the screen is empty.
    None,
}

/// A tafs cloud taf's name, by its id, for its runs' title.
pub type Names = Rc<Vec<(String, String)>>;

/// What has been read, each read on its own: one that fails keeps what was read and says why.
#[derive(Default)]
struct State {
    /// Read on this screen: an earlier read isn't shown as now.
    read: bool,
    balance: Option<Balance>,
    balance_error: Option<String>,
    credits: Option<Credits>,
    credits_error: Option<String>,
    previous: Option<Credits>,
    previous_error: Option<String>,
    /// The period before was asked for and its read is over, whatever it gave.
    previous_read: bool,
    /// Which period the Credits panel shows.
    showing_previous: bool,
    /// KIFF's usage read, and why it couldn't be read.
    usage: Option<KiffUsage>,
    usage_error: Option<String>,
}

type Redraw = Rc<dyn Fn()>;

/// A read's words when KIFF or tafs cloud couldn't be reached at all.
fn unreached(w: &WordBook, what: &str, e: &tafs_core::signin::KiffError) -> String {
    let why = e.words(w).unwrap_or_default();
    w.get(
        "common.could_not_read_why",
        &[("what", what), ("why", &why)],
    )
    .unwrap_or(why)
}

/// Reads tafs cloud and returns what it came to.
fn cloud_read<T: Send + 'static>(
    w: &WordBook,
    source: &UsageSource,
    path: &str,
    what: &str,
    read: fn(&[u8]) -> Option<T>,
) -> Result<Option<T>, String> {
    let UsageSource::Cloud {
        token, languages, ..
    } = source
    else {
        return Ok(None);
    };
    match crate::kiff::cloud(token, languages, path) {
        Ok(a) => answered(w, &a, what, read),
        Err(e) => Err(unreached(w, what, &e)),
    }
}

pub fn page(w: Rc<WordBook>, source: UsageSource, names: Names) -> gtk::Widget {
    // The screen, and one taf's runs over it, with a way back.
    let screens = gtk::Stack::new();
    screens.set_transition_type(gtk::StackTransitionType::SlideLeftRight);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let title = one_line(
        &w.line("usage.title").unwrap_or_default(),
        17,
        600,
        &["tafs-ink"],
    );
    title.set_halign(gtk::Align::Center);
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_margin_top(12);
    outer.append(&title);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    outer.append(&scroller);
    screens.add_named(&outer, Some("usage"));

    let state = Rc::new(RefCell::new(State::default()));
    let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
    let redraw: Redraw = {
        let draw = draw.clone();
        Rc::new(move || {
            let f = draw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        })
    };
    // Reads the period before: when it's first chosen.
    let read_previous: Rc<dyn Fn()> = {
        let (w, source, state, redraw) = (w.clone(), source.clone(), state.clone(), redraw.clone());
        Rc::new(move || {
            match source {
                UsageSource::Sample => {
                    let mut s = state.borrow_mut();
                    s.previous = sample_credits(&w, true).ok();
                    s.previous_read = true;
                    return;
                }
                UsageSource::None | UsageSource::File(_) => {
                    state.borrow_mut().previous_read = true;
                    return;
                }
                UsageSource::Cloud { .. } => {}
            }
            state.borrow_mut().previous_read = false;
            let (words, source, state, redraw) =
                ((*w).clone(), source.clone(), state.clone(), redraw.clone());
            gtk::glib::spawn_future_local(async move {
                let what = words.line("usage.credits").unwrap_or_default();
                let read = gtk::gio::spawn_blocking(move || {
                    cloud_read(&words, &source, &credits_path(true), &what, Credits::read)
                })
                .await
                .unwrap_or(Ok(None));
                {
                    let mut s = state.borrow_mut();
                    match read {
                        Ok(c) => {
                            s.previous = c;
                            s.previous_error = None;
                        }
                        Err(e) => s.previous_error = Some(e),
                    }
                    s.previous_read = true;
                }
                redraw();
            });
        })
    };
    let open_runs: Rc<dyn Fn(String, bool)> = {
        let (w, source, names, screens) =
            (w.clone(), source.clone(), names.clone(), screens.clone());
        Rc::new(move |taf: String, previous: bool| {
            let back_to = screens.clone();
            let runs = runs_page(
                w.clone(),
                source.clone(),
                &names,
                taf,
                previous,
                Rc::new(move || {
                    back_to.set_visible_child_name("usage");
                }),
            );
            if let Some(old) = screens.child_by_name("runs") {
                screens.remove(&old);
            }
            screens.add_named(&runs, Some("runs"));
            screens.set_visible_child_name("runs");
        })
    };
    *draw.borrow_mut() = Some(Rc::new({
        let (w, column, state, read_previous, redraw, source) = (
            w.clone(),
            column.clone(),
            state.clone(),
            read_previous.clone(),
            redraw.clone(),
            source.clone(),
        );
        move || {
            while let Some(c) = column.first_child() {
                column.remove(&c);
            }
            let s = state.borrow();
            if !s.read {
                // Loading draws the screen at its full width, never a narrow strip.
                let p = panel(false);
                let spinner = gtk::Spinner::new();
                spinner.start();
                spinner.set_halign(gtk::Align::Start);
                p.append(&spinner);
                column.append(&p);
                return;
            }
            balance_panel(&column, &s, kiff_billing(&source).is_some());
            if let (Some(manages_billing), Some(u)) = (kiff_billing(&source), &s.usage) {
                billing_panel(&w, &column, u);
                ask_panel(&w, &column, u, manages_billing, &s);
                requests_panel(&w, &column, u, manages_billing, &s);
            }
            // Left out where tafs cloud says so: no plan and no run lately.
            let shows_credits = s.balance.as_ref().is_none_or(|b| b.balance.credits_panel);
            if shows_credits {
                let kiff = kiff_billing(&source).is_some();
                credits_panel(
                    &w,
                    &column,
                    &s,
                    kiff,
                    &state,
                    &read_previous,
                    &redraw,
                    &open_runs,
                );
            }
            // KIFF's read failed: buying and requests wait on it, said once.
            if let (Some(_), None, Some(why)) = (kiff_billing(&source), &s.usage, &s.usage_error) {
                let p = panel(false);
                p.append(&text(why, 14, 400, &["tafs-no"]));
                column.append(&p);
            }
        }
    }));
    // Read on this screen (`.task { await model.refreshUsage() }`).
    match &source {
        UsageSource::Sample => {
            let mut s = state.borrow_mut();
            s.balance = sample_balance(&w).ok();
            s.credits = sample_credits(&w, false).ok();
            s.read = true;
        }
        UsageSource::None => state.borrow_mut().read = true,
        UsageSource::File(path) => {
            let v: serde_json::Value = std::fs::read(path)
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
            let part = |k: &str| serde_json::to_vec(&v[k]).unwrap_or_default();
            let mut s = state.borrow_mut();
            s.balance = Balance::read(&part("balance"));
            s.credits = Credits::read(&part("credits"));
            s.usage = KiffUsage::read(&part("usage"));
            s.read = true;
        }
        UsageSource::Cloud { .. } => {
            let (words, source, state, redraw) =
                ((*w).clone(), source.clone(), state.clone(), redraw.clone());
            gtk::glib::spawn_future_local(async move {
                let read = gtk::gio::spawn_blocking(move || {
                    let what = words.line("usage.the_balance").unwrap_or_default();
                    let b = cloud_read(&words, &source, BALANCE_PATH, &what, Balance::read);
                    let what = words.line("usage.credits").unwrap_or_default();
                    let c = cloud_read(&words, &source, &credits_path(false), &what, Credits::read);
                    // KIFF's read, alongside: a failure says so once, under the credits.
                    let u = match &source {
                        UsageSource::Cloud {
                            token, languages, ..
                        } => match crate::kiff::usage(token, languages) {
                            Ok(a) if (200..300).contains(&a.status) => KiffUsage::read(&a.body)
                                .ok_or_else(|| {
                                    tafs_core::signin::plain_words(&words, 200).unwrap_or_default()
                                }),
                            Ok(a) => Err(tafs_core::signin::refusal(&a, Some(&words))
                                .words(&words)
                                .unwrap_or_default()),
                            Err(e) => Err(e.words(&words).unwrap_or_default()),
                        }
                        .map_err(|why| {
                            words
                                .get("billing.plan_could_not_read_kiff", &[("why", &why)])
                                .unwrap_or(why)
                        }),
                        _ => Ok(KiffUsage::default()),
                    };
                    (b, c, u)
                })
                .await;
                {
                    let mut s = state.borrow_mut();
                    if let Ok((b, c, u)) = read {
                        match u {
                            Ok(u) => s.usage = Some(u),
                            Err(e) => s.usage_error = Some(e),
                        }
                        match b {
                            Ok(b) => s.balance = b,
                            Err(e) => s.balance_error = Some(e),
                        }
                        match c {
                            Ok(c) => s.credits = c,
                            Err(e) => s.credits_error = Some(e),
                        }
                    }
                    s.read = true;
                }
                redraw();
            });
        }
    }
    redraw();
    crate::widgets::ground(&screens);
    screens.upcast()
}

/// `BalancePanel`: the words, a bar at `meter_pct`, the detail and the warning; else why it
/// couldn't be read. Signed in with KIFF, tafs cloud's Raise offer under it, dimmed until
/// raising (a signed call) is ported. Buying waits for a store.
fn balance_panel(column: &gtk::Box, s: &State, kiff: bool) {
    let Some(b) = &s.balance else {
        if let Some(why) = &s.balance_error {
            let p = panel(false);
            p.append(&text(why, 14, 400, &["tafs-no"]));
            column.append(&p);
        }
        return;
    };
    let line = &b.balance;
    let p = panel(false);
    p.set_spacing(10);
    if line.waiting() {
        p.add_css_class("edge-no");
    } else if line.state == "warning" {
        p.add_css_class("edge-amber");
    }
    // A refresh that failed keeps what was read and says why.
    if let Some(why) = &s.balance_error {
        p.append(&text(why, 13, 400, &["tafs-no"]));
    }
    p.append(&text(&line.words, 17, 600, &["tafs-ink"]));
    if let Some(pct) = line.meter_pct {
        p.append(&meter(pct as f64 / 100.0, tone(&line.state)));
    }
    if let Some(detail) = &line.detail {
        p.append(&text(detail, 14, 400, &["tafs-soft"]));
    }
    if let Some(warning) = &line.warning {
        p.append(&text(
            warning,
            14,
            600,
            &[tone_class(tone(&line.state), "tafs-ink")],
        ));
    }
    column.append(&p);
    if let (true, Some(raise)) = (kiff, &line.raise) {
        let b = crate::widgets::primary(&raise.words);
        b.set_sensitive(false);
        column.append(&b);
    }
}

/// The colour of tafs cloud's tone (`UsageView.color`), with its calm colour.
fn tone_class(t: Tone, calm: &'static str) -> &'static str {
    match t {
        Tone::No => "tafs-no",
        Tone::Amber => "tafs-amber-text",
        Tone::Calm => calm,
    }
}

/// `Meter`: a capsule 5 high, the ink at 10% under the part used, in the tone's colour (the
/// calm one green).
fn meter(fraction: f64, t: Tone) -> gtk::Widget {
    let bar = gtk::DrawingArea::new();
    bar.set_content_height(5);
    bar.set_hexpand(true);
    bar.add_css_class("tafs-meter");
    bar.add_css_class(match t {
        Tone::No => "no",
        Tone::Amber => "amber",
        Tone::Calm => "ok",
    });
    let fraction = fraction.clamp(0.0, 1.0);
    bar.set_draw_func(move |area, cr, width, height| {
        let (wd, h) = (width as f64, height as f64);
        let capsule = |cr: &gtk::cairo::Context, len: f64| {
            let r = h / 2.0;
            if len <= 0.0 {
                return;
            }
            let r = r.min(len / 2.0);
            cr.new_sub_path();
            cr.arc(
                len - r,
                r,
                r,
                -std::f64::consts::FRAC_PI_2,
                std::f64::consts::FRAC_PI_2,
            );
            cr.arc(
                r,
                h - r,
                r,
                std::f64::consts::FRAC_PI_2,
                3.0 * std::f64::consts::FRAC_PI_2,
            );
            cr.close_path();
        };
        let ink = area.color();
        // The track: the ink colour of the page at 10%, from the parent's colour.
        let track = area.parent().map(|p| p.color()).unwrap_or(ink);
        cr.set_source_rgba(
            track.red() as f64,
            track.green() as f64,
            track.blue() as f64,
            0.1,
        );
        capsule(cr, wd);
        let _ = cr.fill();
        cr.set_source_rgba(ink.red() as f64, ink.green() as f64, ink.blue() as f64, 1.0);
        capsule(cr, wd * fraction);
        let _ = cr.fill();
    });
    bar.upcast()
}

/// `CreditsPanel`: this period or the one before; the period's words, each taf that used credits
/// (opening its runs, at KIFF), Coach past its fair use, what wasn't counted, then the note.
#[allow(clippy::too_many_arguments)]
fn credits_panel(
    w: &WordBook,
    column: &gtk::Box,
    s: &State,
    kiff: bool,
    state: &Rc<RefCell<State>>,
    read_previous: &Rc<dyn Fn()>,
    redraw: &Redraw,
    open_runs: &Rc<dyn Fn(String, bool)>,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let previous = s.showing_previous;
    let shown = if previous { &s.previous } else { &s.credits };
    let error = if previous {
        &s.previous_error
    } else {
        &s.credits_error
    };
    // The picker: Last period, then This period, as the iPad's.
    let picker = || {
        let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        tabs.add_css_class("tafs-segmented");
        tabs.set_homogeneous(true);
        let last = gtk::ToggleButton::with_label(&l("usage.last_period"));
        let this = gtk::ToggleButton::with_label(&l("usage.this_period"));
        this.set_group(Some(&last));
        for b in [&last, &this] {
            b.add_css_class("segment");
            tabs.append(b);
        }
        last.set_active(previous);
        this.set_active(!previous);
        let (state, read_previous, redraw) = (state.clone(), read_previous.clone(), redraw.clone());
        last.connect_toggled(move |b| {
            let wants = b.is_active();
            let first = {
                let mut s = state.borrow_mut();
                if s.showing_previous == wants {
                    return;
                }
                s.showing_previous = wants;
                wants && s.previous.is_none()
            };
            if first {
                read_previous();
            }
            // After the toggle has settled, never inside its own signal.
            let redraw = redraw.clone();
            gtk::glib::idle_add_local_once(move || redraw());
        });
        tabs
    };
    let eyebrow_line = || eyebrow(&l("usage.credits"), false);
    let Some(c) = shown else {
        if previous {
            let p = panel(false);
            p.append(&eyebrow_line());
            p.append(&picker());
            if let Some(why) = error {
                p.append(&text(why, 14, 400, &["tafs-no"]));
            } else if s.previous_read {
                p.append(&text(
                    &l("usage.last_period_credits_could_not"),
                    14,
                    400,
                    &["tafs-soft"],
                ));
            } else {
                let spinner = gtk::Spinner::new();
                spinner.start();
                spinner.set_halign(gtk::Align::Start);
                p.append(&spinner);
            }
            column.append(&p);
        } else if let Some(why) = error {
            // This period couldn't be read: tafs cloud's words.
            let p = panel(false);
            p.append(&eyebrow_line());
            p.append(&text(why, 14, 400, &["tafs-no"]));
            column.append(&p);
        }
        return;
    };
    let p = panel(false);
    p.append(&eyebrow_line());
    p.append(&picker());
    // A refresh that failed keeps what was read and says why.
    if let Some(why) = error {
        p.append(&text(why, 13, 400, &["tafs-no"]));
    }
    p.append(&text(&c.words, 17, 600, &["tafs-ink"]));
    column.append(&p);
    for t in &c.tafs {
        // A taf's row opens its runs at KIFF; the row for tafs removed since, and the sample's, don't.
        match (&t.taf, kiff) {
            (Some(id), true) => {
                let row = gtk::Button::new();
                row.add_css_class("tafs-panel-link");
                let h = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                let words = text(&t.words, 15, 400, &["tafs-ink"]);
                words.set_hexpand(true);
                h.append(&words);
                let chevron = symbol("go-next-symbolic", 13);
                chevron.add_css_class("tafs-dim");
                h.append(&chevron);
                row.set_child(Some(&h));
                let (open_runs, id) = (open_runs.clone(), id.clone());
                row.connect_clicked(move |_| open_runs(id.clone(), previous));
                column.append(&row);
            }
            _ => {
                let p = panel(false);
                p.append(&text(&t.words, 15, 400, &["tafs-ink"]));
                column.append(&p);
            }
        }
    }
    // Coach past its fair use: its own row, which doesn't open.
    if let Some(coach) = &c.coach {
        let p = panel(false);
        p.append(&text(&coach.words, 15, 400, &["tafs-ink"]));
        column.append(&p);
    }
    if let Some(not) = c.shows_not_counted() {
        let p = panel(false);
        p.set_spacing(12);
        if !not.words.is_empty() {
            p.append(&text(&not.words, 15, 600, &["tafs-ink"]));
        }
        for line in &not.lines {
            p.append(&text(&line.words, 14, 400, &["tafs-soft"]));
        }
        column.append(&p);
    }
    // The note comes last, as the contract has it.
    if let Some(note) = &c.note {
        let n = text(note, 13, 400, &["tafs-soft"]);
        n.set_margin_start(4);
        n.set_margin_end(4);
        column.append(&n);
    }
}

/// `TafCreditsView`: one taf's credits in a period, then its runs, newest first, More runs while
/// there are more. Its title is the taf's name, else Credits; Back returns to Plan & usage.
fn runs_page(
    w: Rc<WordBook>,
    source: UsageSource,
    names: &Names,
    taf: String,
    previous: bool,
    back: Rc<dyn Fn()>,
) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_margin_top(12);
    let head = gtk::CenterBox::new();
    let back_button = gtk::Button::new();
    back_button.add_css_class("tafs-back");
    let back_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    back_row.append(&symbol("go-previous-symbolic", 17));
    back_row.append(&one_line(&l("usage.title"), 17, 400, &[]));
    back_button.set_child(Some(&back_row));
    back_button.connect_clicked(move |_| back());
    back_button.set_margin_start(8);
    head.set_start_widget(Some(&back_button));
    let name = names
        .iter()
        .find(|(id, _)| *id == taf)
        .map(|(_, n)| n.clone())
        .unwrap_or_else(|| l("usage.credits"));
    head.set_center_widget(Some(&one_line(&name, 17, 600, &["tafs-ink"])));
    outer.append(&head);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    outer.append(&scroller);

    #[derive(Default)]
    struct Runs {
        read: bool,
        mine: Option<TafCredits>,
        error: Option<String>,
        loading: bool,
    }
    let state = Rc::new(RefCell::new(Runs::default()));
    let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
    let redraw: Redraw = {
        let draw = draw.clone();
        Rc::new(move || {
            let f = draw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        })
    };
    // Reads its credits and newest runs; with `more`, the next page after the runs read.
    let load: Rc<dyn Fn(bool)> = {
        let (w, source, state, redraw, taf) = (
            w.clone(),
            source.clone(),
            state.clone(),
            redraw.clone(),
            taf.clone(),
        );
        Rc::new(move |more: bool| {
            let after = {
                let mut s = state.borrow_mut();
                if s.loading {
                    return;
                }
                let after = if more {
                    match s.mine.as_ref().and_then(|m| m.next.clone()) {
                        Some(a) => Some(a),
                        None => return,
                    }
                } else {
                    None
                };
                s.loading = true;
                after
            };
            let path = taf_credits_path(&taf, previous, after.as_deref());
            let (words, source, state, redraw) =
                ((*w).clone(), source.clone(), state.clone(), redraw.clone());
            gtk::glib::spawn_future_local(async move {
                let what = words
                    .line(if more {
                        "usage.more_runs"
                    } else {
                        "work.its_runs"
                    })
                    .unwrap_or_default();
                let read = gtk::gio::spawn_blocking(move || {
                    cloud_read(&words, &source, &path, &what, TafCredits::read)
                })
                .await
                .unwrap_or(Ok(None));
                {
                    let mut s = state.borrow_mut();
                    s.loading = false;
                    s.read = true;
                    match read {
                        Ok(Some(page)) => {
                            s.error = None;
                            match (&mut s.mine, more) {
                                (Some(known), true) => known.add(page),
                                _ => s.mine = Some(page),
                            }
                        }
                        // A tafs cloud without the read.
                        Ok(None) => s.error = None,
                        Err(e) => s.error = Some(e),
                    }
                }
                redraw();
            });
        })
    };
    *draw.borrow_mut() = Some(Rc::new({
        let (w, column, state, load) = (w.clone(), column.clone(), state.clone(), load.clone());
        move || {
            while let Some(c) = column.first_child() {
                column.remove(&c);
            }
            let s = state.borrow();
            let l = |k: &str| w.line(k).unwrap_or_default();
            if !s.read {
                let p = panel(false);
                let spinner = gtk::Spinner::new();
                spinner.start();
                spinner.set_halign(gtk::Align::Start);
                p.append(&spinner);
                column.append(&p);
                return;
            }
            let Some(mine) = &s.mine else {
                let p = panel(false);
                p.append(&text(
                    &s.error
                        .clone()
                        .unwrap_or_else(|| l("usage.runs_could_not_read_pull")),
                    14,
                    400,
                    &["tafs-soft"],
                ));
                // There's no pull on a desktop: the page reads again on Try again.
                let again = outline(&l("common.try_again"));
                let load = load.clone();
                again.connect_clicked(move |_| load(false));
                p.append(&again);
                column.append(&p);
                return;
            };
            let p = panel(false);
            p.append(&text(&mine.words, 17, 600, &["tafs-ink"]));
            column.append(&p);
            if !mine.items.is_empty() {
                let p = panel(false);
                for run in &mine.items {
                    // `CreditsRunRow`: tafs cloud's words, semibold when counted, and when it
                    // ended, in the account's time zone. A run's request opens with its port.
                    let row = gtk::Box::new(gtk::Orientation::Vertical, 2);
                    row.append(&text(
                        &run.words,
                        14,
                        if run.counted { 600 } else { 400 },
                        &[if run.counted { "tafs-ink" } else { "tafs-soft" }],
                    ));
                    row.append(&one_line(
                        &tafs_core::account_time::short_time(&run.at),
                        13,
                        400,
                        &["tafs-dim"],
                    ));
                    p.append(&row);
                }
                if mine.next.is_some() {
                    let more = outline(&l("usage.more_runs"));
                    let load = load.clone();
                    more.connect_clicked(move |_| load(true));
                    p.append(&more);
                }
                if let Some(why) = &s.error {
                    p.append(&text(why, 13, 400, &["tafs-no"]));
                }
                column.append(&p);
            }
        }
    }));
    redraw();
    load(false);
    crate::widgets::ground(&outer);
    outer.upcast()
}

/// `billingPanel`: who bills the plan, in one line; after a move from Solo in the App Store to
/// Company, the line that Solo still renews with Apple. Manage and Cancel Solo open Apple's
/// own subscriptions page, which only an Apple device has: Linux shows their lines alone.
fn billing_panel(w: &WordBook, column: &gtk::Box, u: &KiffUsage) {
    let Some(line) = billing_line(w, u).ok().flatten() else {
        return;
    };
    let p = panel(false);
    p.set_spacing(10);
    p.append(&text(&line, 15, 400, &["tafs-ink"]));
    if let Some(renews) = solo_still_renews(w, u).ok().flatten() {
        p.append(&text(&renews, 14, 400, &["tafs-soft"]));
    }
    column.append(&p);
}

/// `askPanel`, for someone who doesn't manage billing: their request waiting, with Withdraw;
/// else the last answer, and Ask an admin where the plan lets them ask. The buttons are there,
/// dimmed, until asking is ported.
fn ask_panel(w: &WordBook, column: &gtk::Box, u: &KiffUsage, manages_billing: bool, s: &State) {
    if manages_billing {
        return;
    }
    let l = |k: &str| w.line(k).unwrap_or_default();
    if let Some(mine) = waiting(u, false) {
        let p = panel(false);
        p.append(&eyebrow(&l("usage.requests.waiting"), false));
        let detail = asked_line(w, mine).unwrap_or_default();
        p.append(&text(
            &w.get("usage.admin_answers", &[("detail", &detail)])
                .unwrap_or(detail),
            15,
            400,
            &["tafs-ink"],
        ));
        let withdraw = outline(&l("usage.requests.withdraw"));
        withdraw.set_sensitive(false);
        p.append(&withdraw);
        column.append(&p);
        return;
    }
    if let Some(answer) = last_answer(w, u, false).ok().flatten() {
        let p = panel(false);
        p.append(&text(&answer, 14, 400, &["tafs-soft"]));
        column.append(&p);
    }
    if asks(u).is_some() {
        // In the primary style while the balance waits on it.
        let waits = s.balance.as_ref().is_some_and(|b| b.balance.waiting());
        let ask = if waits {
            crate::widgets::primary(&l("usage.requests.ask_admin"))
        } else {
            outline(&l("usage.requests.ask_admin"))
        };
        ask.set_sensitive(false);
        column.append(&ask);
    }
}

/// `requestsPanel`, for someone who manages billing: members' open requests, in the amber,
/// each its line and reason, then Raise the limit (where tafs cloud offers it) and Decline,
/// dimmed until answering is ported. Approving a pack buys it in the App Store: none here.
fn requests_panel(
    w: &WordBook,
    column: &gtk::Box,
    u: &KiffUsage,
    manages_billing: bool,
    s: &State,
) {
    let open = open_requests(u, manages_billing);
    if open.is_empty() {
        return;
    }
    let l = |k: &str| w.line(k).unwrap_or_default();
    let p = panel(false);
    p.add_css_class("edge-amber");
    p.append(&eyebrow(&l("usage.requests.title"), true));
    for r in open {
        let one = gtk::Box::new(gtk::Orientation::Vertical, 8);
        one.append(&text(
            &request_line(w, r, u).unwrap_or_default(),
            15,
            400,
            &["tafs-ink"],
        ));
        if !r.reason.is_empty() {
            one.append(&text(&r.reason, 14, 400, &["tafs-soft"]));
        }
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let offers_raise = s
            .balance
            .as_ref()
            .is_some_and(|b| b.balance.raise.is_some());
        if r.kind == "raise" && offers_raise {
            let raise = crate::widgets::primary(&l("usage.raise_limit_button"));
            raise.set_sensitive(false);
            buttons.append(&raise);
        }
        let decline = outline(&l("usage.requests.decline"));
        decline.set_sensitive(false);
        buttons.append(&decline);
        one.append(&buttons);
        p.append(&one);
    }
    column.append(&p);
}

/// Signed in with KIFF (or its synthetic file): whether the person manages billing. None for
/// the sample and signed out, which draw no KIFF panel.
fn kiff_billing(source: &UsageSource) -> Option<bool> {
    match source {
        UsageSource::Cloud {
            manages_billing, ..
        } => Some(*manages_billing),
        UsageSource::File(path) => Some(
            std::fs::read(path)
                .ok()
                .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                .and_then(|v| v["manages_billing"].as_bool())
                .unwrap_or(true),
        ),
        _ => None,
    }
}
