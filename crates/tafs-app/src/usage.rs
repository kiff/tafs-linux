//! Plan & usage beside the sidebar (`UsageView`): tafs cloud's balance (`BalancePanel`), then the
//! credits the tafs used (`CreditsPanel`), this period or the one before, each taf's row opening
//! its runs (`TafCreditsView`, `CreditsRunRow`). Every line is tafs cloud's words, as it writes
//! them: the screen adds nothing but their order. The reads are in `tafs-core/src/usage.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use tafs_core::approval::NotReady;
use tafs_core::sidebar::Mode;
use tafs_core::signed::Act;
use tafs_core::usage::{
    answered, credits_path, sample_balance, sample_credits, taf_credits_path, tone, Balance,
    Credits, TafCredits, Tone, BALANCE_PATH,
};
use tafs_core::usage::{
    ask_request, ask_sheet, asked_line, asks, billing_line, can_ask, decline_act, decline_sheet,
    last_answer, not_sent, not_withdrawn, offered_packs, open_requests, request_line,
    solo_still_renews, waiting, withdraw_request, KiffUsage, PackRequest,
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
        /// `mayUseCoach`: a run's page offers Ask Coach only to someone who may, and who manages
        /// its taf (`KnobRequestDetail.offersAskCoach`).
        may_use_coach: bool,
    },
    /// A file of synthetic answers (`--usage`, the screenshots signed in with KIFF): its
    /// `balance`, `credits` and `usage`, and whether the person manages billing.
    File(std::path::PathBuf),
    /// Nothing to read (signed out, or a screenshot of the contracts): the screen is empty.
    None,
}

/// A tafs cloud taf's name, by its id, for its runs' title, and whether this person manages it
/// (a run's own page offers Try the reply now to whoever does).
pub type Names = Rc<Vec<(String, String, bool)>>;

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
    /// `planMessage`: why asking or withdrawing didn't go.
    plan_message: Option<String>,
    /// Where a tap would have signed, this computer's words: under the balance's Raise (the iPad's
    /// `why`), and where the plan's message goes (answering a raise request).
    raise_refused: Option<NotReady>,
    plan_refused: Option<NotReady>,
    /// `refreshUsage` is on its way: a pull asks nothing more.
    refreshing: bool,
}

thread_local! {
    /// `--page usage-ask` or `usage-decline`: Ask an admin's sheet, or the first request's
    /// Decline, opens once the page is read, for its screenshot.
    pub static OPEN: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
    /// A taf's runs to open once Plan & usage is drawn: its Usage row on its Card tab.
    static RUNS_OF: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// A taf's Usage row (`TafCreditsView` from its page): Plan & usage opens on its runs, this period.
pub fn open_runs_of(taf: &str) {
    RUNS_OF.with(|r| *r.borrow_mut() = Some(taf.to_string()));
}

/// What the screen's buttons act on: the words, what was read, and reading again.
#[derive(Clone)]
struct Ctx {
    w: Rc<WordBook>,
    state: Rc<RefCell<State>>,
    redraw: Redraw,
    source: UsageSource,
    /// `refreshUsage`: KIFF's usage and the balance, read again.
    reload: Rc<dyn Fn()>,
}

impl Ctx {
    /// Signed in with KIFF, or a screenshot's account: the iPad signs only there.
    fn mode(&self) -> Mode {
        if kiff_billing(&self.source).is_some() {
            Mode::Kiff
        } else {
            Mode::SignedOut
        }
    }

    /// The tap that would sign `act`: what this computer says, kept with `put`, and nothing sent.
    fn refuse(&self, act: &Act, put: fn(&mut State, Option<NotReady>)) {
        let n = tafs_core::signed::tap(&self.w, self.mode(), act)
            .ok()
            .flatten();
        put(&mut self.state.borrow_mut(), n);
        let redraw = self.redraw.clone();
        gtk::glib::idle_add_local_once(move || redraw());
    }

    /// Raise the limit (`raiseLimit`) from the limit tafs cloud offered raising. With this
    /// computer's security key, its sheet shows the new limit and the one it's raised from, then
    /// the key's touch and PIN check the person, then the device key signs, every time; once
    /// raised, the balance is read again. Else what this computer says, kept with `put`.
    fn raise(
        &self,
        offer: &tafs_core::usage::Offer,
        over: &gtk::Widget,
        put: fn(&mut State, Option<NotReady>),
    ) {
        let act = Act::RaiseLimit {
            from_cents: offer.from_cents.unwrap_or_default(),
        };
        let kept = crate::store::load(chrono::Utc::now().timestamp());
        let signs = self.mode() == Mode::Kiff && crate::key_approval::can_sign(kept.as_ref(), &act);
        let (true, Some(kept)) = (signs, kept) else {
            return self.refuse(&act, put);
        };
        let said = move |ctx: &Ctx, line: String| {
            put(
                &mut ctx.state.borrow_mut(),
                Some(NotReady {
                    line,
                    note: None,
                    open_settings: None,
                    check_again: None,
                }),
            );
            let redraw = ctx.redraw.clone();
            gtk::glib::idle_add_local_once(move || redraw());
        };
        // No limit to raise from: nothing is raised (`noRaise`).
        let Ok(Some(details)) = tafs_core::usage::raise_lines(&self.w, offer) else {
            let line = self.w.line("card.limit_can_not_raised_right");
            return said(self, line.unwrap_or_default());
        };
        put(&mut self.state.borrow_mut(), None);
        let ctx = self.clone();
        let done: crate::key_approval::Approved = Rc::new(move |outcome| match outcome {
            Some(Ok(())) => (ctx.reload)(),
            // `limit_changed`: the balance is read again, which offers the raise from the limit
            // it is now; the line says so.
            Some(Err(line)) => {
                said(&ctx, line);
                (ctx.reload)();
            }
            None => {}
        });
        let languages =
            tafs_core::language::screen_languages(&tafs_core::language::preferred_from_env());
        crate::key_approval::sign_with_presence(
            self.w.clone(),
            over,
            kept,
            languages,
            &act,
            act.own_prompt(&self.w).as_deref(),
            details,
            done,
        );
    }
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
    // Each taf's runs as last read, by taf and period (`tafCredits`), kept while the screen is.
    let kept_runs: KeptRuns = Rc::new(RefCell::new(std::collections::BTreeMap::new()));
    let open_runs: Rc<dyn Fn(String, bool)> = {
        let (w, source, names, screens, kept_runs) = (
            w.clone(),
            source.clone(),
            names.clone(),
            screens.clone(),
            kept_runs.clone(),
        );
        Rc::new(move |taf: String, previous: bool| {
            let back_to = screens.clone();
            let runs = runs_page(
                w.clone(),
                source.clone(),
                &names,
                kept_runs.clone(),
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
    // Opened from a taf's Usage row: on its runs at once.
    if let Some(taf) = RUNS_OF.with(|r| r.borrow_mut().take()) {
        let open = open_runs.clone();
        gtk::glib::idle_add_local_once(move || open(taf, false));
    }
    // `refreshUsage`: the balance, the credits and KIFF's usage, read as the session.
    let reload: Rc<dyn Fn()> = {
        let (w, source, state, redraw) = (w.clone(), source.clone(), state.clone(), redraw.clone());
        Rc::new(move || {
            if !matches!(source, UsageSource::Cloud { .. }) {
                return;
            }
            state.borrow_mut().refreshing = true;
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
                        // A read that works says nothing of the last one's failure.
                        match u {
                            Ok(u) => {
                                s.usage = Some(u);
                                s.usage_error = None;
                            }
                            Err(e) => s.usage_error = Some(e),
                        }
                        match b {
                            Ok(b) => {
                                s.balance = b;
                                s.balance_error = None;
                            }
                            Err(e) => s.balance_error = Some(e),
                        }
                        match c {
                            Ok(c) => {
                                s.credits = c;
                                s.credits_error = None;
                            }
                            Err(e) => s.credits_error = Some(e),
                        }
                    }
                    s.read = true;
                    s.refreshing = false;
                }
                redraw();
            });
        })
    };
    // Pull to refresh (`.refreshable { await model.refreshUsage() }`): the balance, the credits
    // and KIFF's usage read again, and the period before too once it was asked for. What was read
    // stays in sight meanwhile; not while a read is on its way.
    let refresh: Rc<dyn Fn()> = {
        let (state, reload, read_previous) = (state.clone(), reload.clone(), read_previous.clone());
        Rc::new(move || {
            let again_previous = {
                let s = state.borrow();
                if s.refreshing || !s.read {
                    return;
                }
                s.previous.is_some() || s.previous_error.is_some()
            };
            reload();
            if again_previous {
                read_previous();
            }
        })
    };
    crate::widgets::refreshable(&scroller, &outer, refresh);
    let ctx = Ctx {
        w: w.clone(),
        state: state.clone(),
        redraw: redraw.clone(),
        source: source.clone(),
        reload: reload.clone(),
    };
    *draw.borrow_mut() = Some(Rc::new({
        let (w, column, state, read_previous, redraw, source, ctx) = (
            w.clone(),
            column.clone(),
            state.clone(),
            read_previous.clone(),
            redraw.clone(),
            source.clone(),
            ctx.clone(),
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
            balance_panel(&ctx, &column, &s, kiff_billing(&source).is_some());
            // `planMessage`: why asking or withdrawing didn't go; or this computer's words.
            if let Some(message) = &s.plan_message {
                let p = panel(false);
                p.append(&text(message, 14, 400, &["tafs-soft"]));
                column.append(&p);
            }
            if let Some(n) = &s.plan_refused {
                let p = panel(false);
                p.append(&crate::approval::not_ready(n));
                column.append(&p);
            }
            if let (Some(manages_billing), Some(u)) = (kiff_billing(&source), &s.usage) {
                billing_panel(&w, &column, u);
                ask_panel(&ctx, &column, u, manages_billing, &s);
                requests_panel(&ctx, &column, u, manages_billing, &s);
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
        UsageSource::Cloud { .. } => {}
    }
    reload();
    open_sheet(&ctx, &column);
    redraw();
    crate::widgets::ground(&screens);
    screens.upcast()
}

/// `BalancePanel`: the words, a bar at `meter_pct`, the detail and the warning; else why it
/// couldn't be read. Signed in with KIFF, tafs cloud's Raise offer under it: raising is signed,
/// so its tap says this computer can't approve, under it where the iPad says why it wasn't
/// raised, and sends nothing. Where the iPad sells credits and Solo, where to buy them.
fn balance_panel(ctx: &Ctx, column: &gtk::Box, s: &State, kiff: bool) {
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
        // `ApprovalGate`, as the iPad's: with this computer's security key, Raise; without one,
        // why this computer can't approve, in its place (`raise_or_gate`).
        let b = crate::widgets::primary(&raise.words);
        b.set_widget_name("raise-limit");
        let (ctx2, offer) = (ctx.clone(), raise.clone());
        b.connect_clicked(move |b| ctx2.raise(&offer, b.upcast_ref(), |s, n| s.raise_refused = n));
        column.append(&raise_or_gate(ctx, &b, raise));
        if let Some(n) = &s.raise_refused {
            column.append(&crate::approval::not_ready(n));
        }
    }
    // Where the iPad sells credits and Solo: on a computer, where to buy them (ruling A).
    if kiff {
        if let Ok(Some(b)) = tafs_core::usage::buying(&ctx.w, line.buy.as_ref()) {
            column.append(&buying_panel(&b));
        }
    }
}

/// Where to buy, on a computer: Solo and credits in the tafs app on the phone, and Company on
/// kiff.dev, opened in the browser. Nothing is bought here.
fn buying_panel(b: &tafs_core::usage::Buying) -> gtk::Box {
    let p = panel(false);
    p.set_spacing(10);
    p.set_widget_name("usage.buying");
    p.append(&text(&b.on_phone, 15, 400, &["tafs-ink"]));
    let team = outline(&b.team);
    team.set_widget_name("usage.team-on-web");
    team.set_halign(gtk::Align::Start);
    team.set_tooltip_text(Some(tafs_core::usage::TEAM_CHECKOUT));
    team.connect_clicked(|b| {
        let window = b.root().and_then(|r| r.downcast::<gtk::Window>().ok());
        gtk::UriLauncher::new(tafs_core::usage::TEAM_CHECKOUT).launch(
            window.as_ref(),
            gtk::gio::Cancellable::NONE,
            |_| {},
        );
    });
    p.append(&team);
    p
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

/// Each taf's credits and runs as last read, by its tafs cloud id and whether it's the period
/// before (`AppModel.tafCredits`, `CreditsKey`).
type KeptRuns = Rc<RefCell<std::collections::BTreeMap<(String, bool), TafCredits>>>;

/// `TafCreditsView`: one taf's credits in a period, then its runs, newest first, More runs while
/// there are more. Its title is the taf's name, else Credits; Back returns to Plan & usage.
/// Opened again, it reads them again; until that read is over, a spinner, then what the last read
/// gave where this one failed, with why under its runs (`tafCreditsError`).
fn runs_page(
    w: Rc<WordBook>,
    source: UsageSource,
    names: &Names,
    kept_runs: KeptRuns,
    taf: String,
    previous: bool,
    back: Rc<dyn Fn()>,
) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    outer.set_margin_top(12);
    let head = gtk::CenterBox::new();
    let back_button = gtk::Button::new();
    // Round, its words for the screen reader and the pointer, as the iPad's Back.
    back_button.add_css_class("tafs-round-back");
    back_button.set_child(Some(&symbol("go-previous-symbolic", 20)));
    back_button.set_tooltip_text(Some(&l("usage.title")));
    back_button.update_property(&[gtk::accessible::Property::Label(&l("usage.title"))]);
    back_button.connect_clicked(move |_| back());
    back_button.set_margin_start(8);
    head.set_start_widget(Some(&back_button));
    let name = names
        .iter()
        .find(|(id, _, _)| *id == taf)
        .map(|(_, n, _)| n.clone())
        .unwrap_or_else(|| l("usage.credits"));
    // A run's own page (`KnobRequestDetail`), read where its credits were: as the session.
    let opening = match &source {
        UsageSource::Cloud {
            token,
            languages,
            may_use_coach,
            ..
        } => {
            let manages = names.iter().any(|(id, _, m)| *id == taf && *m);
            Some(crate::runs_panel::Opening {
                w: w.clone(),
                id: taf.clone(),
                source: crate::runs_panel::RunsSource::Cloud {
                    token: token.clone(),
                    languages: languages.clone(),
                },
                manages,
                // Ask Coach on a run's page opened from here, as on a taf's page and its dial:
                // for someone who may use Coach and manages the taf (`offersAskCoach`); it opens
                // Coach as it was (`openCoach`).
                coach: tafs_core::runs::offers_ask_coach(false, *may_use_coach, manages)
                    .then(|| Rc::new(crate::coach_start::as_it_was) as Rc<dyn Fn()>),
            })
        }
        _ => None,
    };
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
    let state = Rc::new(RefCell::new(Runs {
        mine: kept_runs.borrow().get(&(taf.clone(), previous)).cloned(),
        ..Runs::default()
    }));
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
        let (w, source, state, redraw, taf, kept_runs) = (
            w.clone(),
            source.clone(),
            state.clone(),
            redraw.clone(),
            taf.clone(),
            kept_runs.clone(),
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
            let (kept_runs, key) = (kept_runs.clone(), (taf.clone(), previous));
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
                            if let Some(mine) = &s.mine {
                                kept_runs.borrow_mut().insert(key, mine.clone());
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
                // A pull reads again (its words say so), and Try again for a mouse.
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
                    // ended, in the account's time zone. While tafs cloud says it `opens`, it
                    // opens the run's own page, with a chevron.
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
                    match (&opening, run.opens) {
                        (Some(opening), true) => {
                            row.set_hexpand(true);
                            let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                            h.append(&row);
                            let chevron = symbol("go-next-symbolic", 13);
                            chevron.add_css_class("tafs-dim");
                            chevron.set_valign(gtk::Align::Center);
                            h.append(&chevron);
                            let b = gtk::Button::new();
                            b.add_css_class("flat");
                            b.add_css_class("tafs-run-row");
                            b.set_child(Some(&h));
                            b.update_property(&[gtk::accessible::Property::Label(&run.words)]);
                            let (opening, id) = (opening.clone(), run.run.clone());
                            b.connect_clicked(move |b| {
                                crate::runs_panel::request_sheet(opening.clone(), id.clone(), b)
                            });
                            p.append(&b);
                        }
                        _ => p.append(&row),
                    }
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
    // Pull to refresh (`.refreshable`): its credits and newest runs read again, what was read in
    // sight meanwhile. Not while a read is on its way.
    {
        let load = load.clone();
        crate::widgets::refreshable(&scroller, &outer, Rc::new(move || load(false)));
    }
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
/// else the last answer, and Ask an admin where the plan lets them ask. Both are unsigned, as the
/// session: Withdraw takes the request back, and Ask an admin opens its sheet.
fn ask_panel(ctx: &Ctx, column: &gtk::Box, u: &KiffUsage, manages_billing: bool, s: &State) {
    if manages_billing {
        return;
    }
    let w = &ctx.w;
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
        withdraw.set_widget_name("usage.withdraw");
        let (ctx, id) = (ctx.clone(), mine.id.clone());
        withdraw.connect_clicked(move |b| {
            b.set_sensitive(false);
            let reload = ctx.reload.clone();
            send(
                &ctx,
                withdraw_request(&id),
                not_withdrawn,
                Rc::new(move |_| reload()),
            );
        });
        p.append(&withdraw);
        column.append(&p);
        return;
    }
    if let Some(answer) = last_answer(w, u, false).ok().flatten() {
        let p = panel(false);
        p.append(&text(&answer, 14, 400, &["tafs-soft"]));
        column.append(&p);
    }
    if let Some(raise) = asks(u) {
        // In the primary style while the balance waits on it.
        let waits = s.balance.as_ref().is_some_and(|b| b.balance.waiting());
        let ask = if waits {
            crate::widgets::primary(&l("usage.requests.ask_admin"))
        } else {
            outline(&l("usage.requests.ask_admin"))
        };
        ask.set_widget_name("usage.ask-admin");
        let ctx = ctx.clone();
        ask.connect_clicked(move |b| ask_admin(&ctx, raise, b.upcast_ref()));
        column.append(&ask);
    }
}

/// Sends `call` as the session, unsigned (asking an admin, withdrawing): done says whether it went.
/// When it didn't, why, in the plan's message (`failed`'s words). A screenshot's account sends
/// nothing.
fn send(
    ctx: &Ctx,
    call: Option<tafs_core::signed::Unsigned>,
    failed: fn(&WordBook, &str) -> Result<String, tafs_core::words::WordsError>,
    done: Rc<dyn Fn(bool)>,
) {
    let (
        Some(call),
        UsageSource::Cloud {
            token, languages, ..
        },
    ) = (call, ctx.source.clone())
    else {
        return done(false);
    };
    ctx.state.borrow_mut().plan_message = None;
    let (words, ctx) = ((*ctx.w).clone(), ctx.clone());
    gtk::glib::spawn_future_local(async move {
        let why = gtk::gio::spawn_blocking(move || {
            match crate::kiff::kiff_send(&token, &languages, &call) {
                Ok(a) if (200..300).contains(&a.status) => None,
                Ok(a) => Some(
                    tafs_core::signin::refusal(&a, Some(&words))
                        .words(&words)
                        .unwrap_or_default(),
                ),
                Err(e) => Some(e.words(&words).unwrap_or_default()),
            }
        })
        .await
        .unwrap_or_else(|_| Some(String::new()));
        if let Some(why) = &why {
            ctx.state.borrow_mut().plan_message = failed(&ctx.w, why).ok();
            (ctx.redraw)();
        }
        done(why.is_none());
    });
}

/// `AskAdminView`, its own sheet: what asking does, Which pack (on Solo), Why (optional), why it
/// didn't go, and Ask, unsigned as the session. Asked, the sheet closes and the screen reads again.
fn ask_admin(ctx: &Ctx, raise: bool, over: &gtk::Widget) {
    let w = &ctx.w;
    let Ok(lines) = ask_sheet(w, raise) else {
        return;
    };
    let packs: Vec<(String, String)> = {
        let s = ctx.state.borrow();
        s.usage
            .as_ref()
            .map(|u| {
                offered_packs(u, s.balance.as_ref())
                    .into_iter()
                    .map(|p| (p.id.clone(), p.line.clone()))
                    .collect()
            })
            .unwrap_or_default()
    };
    let sheet = crate::sheet::new(&lines.title, &lines.cancel, None, 520);
    sheet
        .column
        .append(&text(&lines.line, 15, 400, &["tafs-soft"]));
    let chosen: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let ask = crate::widgets::primary(&lines.ask);
    ask.set_widget_name("usage.ask");
    let sending = Rc::new(std::cell::Cell::new(false));
    let refresh: Rc<dyn Fn()> = {
        let (ask, chosen, sending) = (ask.clone(), chosen.clone(), sending.clone());
        Rc::new(move || {
            ask.set_sensitive(can_ask(
                Some(raise),
                chosen.borrow().as_deref(),
                sending.get(),
            ))
        })
    };
    if !raise {
        let p = panel(false);
        p.append(&eyebrow(&lines.which_pack, false));
        let mut first: Option<gtk::CheckButton> = None;
        for (id, line) in &packs {
            let c = gtk::CheckButton::with_label(line);
            c.set_widget_name(&format!("usage.pack.{id}"));
            match &first {
                Some(f) => c.set_group(Some(f)),
                None => first = Some(c.clone()),
            }
            let (chosen, refresh, id) = (chosen.clone(), refresh.clone(), id.clone());
            c.connect_toggled(move |c| {
                if c.is_active() {
                    *chosen.borrow_mut() = Some(id.clone());
                    refresh();
                }
            });
            p.append(&c);
        }
        sheet.column.append(&p);
    }
    let p = panel(false);
    p.append(&eyebrow(&lines.why, false));
    let reason = gtk::Entry::new();
    reason.set_placeholder_text(Some(&lines.hint));
    reason.set_widget_name("usage.reason");
    p.append(&reason);
    sheet.column.append(&p);
    let failed = text("", 14, 400, &["tafs-no"]);
    failed.set_visible(false);
    sheet.column.append(&failed);
    sheet.column.append(&ask);
    refresh();
    {
        let (ctx, dialog, chosen, sending, refresh, failed, asking, words) = (
            ctx.clone(),
            sheet.dialog.downgrade(),
            chosen.clone(),
            sending.clone(),
            refresh.clone(),
            failed.clone(),
            lines.asking.clone(),
            lines.ask.clone(),
        );
        ask.connect_clicked(move |b| {
            let call = ask_request(raise, chosen.borrow().as_deref(), &reason.text());
            if call.is_none() || sending.get() {
                return;
            }
            sending.set(true);
            b.set_label(&asking);
            refresh();
            let (ctx2, dialog, sending, refresh, failed, b, words) = (
                ctx.clone(),
                dialog.clone(),
                sending.clone(),
                refresh.clone(),
                failed.clone(),
                b.clone(),
                words.clone(),
            );
            send(
                &ctx,
                call,
                not_sent,
                Rc::new(move |asked| {
                    sending.set(false);
                    b.set_label(&words);
                    refresh();
                    if asked {
                        if let Some(d) = dialog.upgrade() {
                            d.close();
                        }
                        (ctx2.reload)();
                    } else if let Some(m) = ctx2.state.borrow().plan_message.clone() {
                        failed.set_label(&m);
                        failed.set_visible(true);
                    }
                }),
            );
        });
    }
    sheet.dialog.present(Some(over));
}

/// `DeclineRequestView`, its own sheet: the question, Note (optional), and Decline in red. Declining
/// is signed by the device key after the security key's touch and PIN, over the note as typed;
/// declined, the sheet closes and the requests are read again; not, why, where the iPad says it.
/// Without a key, its tap says this computer can't approve, sends nothing, and the sheet stays.
fn decline(ctx: &Ctx, r: &PackRequest, u: &KiffUsage, over: &gtk::Widget) {
    let w = &ctx.w;
    let Ok(lines) = decline_sheet(w, r, u) else {
        return;
    };
    let sheet = crate::sheet::new(&lines.title, &lines.cancel, None, 520);
    sheet
        .column
        .append(&text(&lines.question, 16, 600, &["tafs-ink"]));
    let p = panel(false);
    p.append(&eyebrow(&lines.note, false));
    let note = gtk::Entry::new();
    note.set_placeholder_text(Some(&lines.hint));
    note.set_widget_name("usage.note");
    p.append(&note);
    sheet.column.append(&p);
    let refused = crate::approval::refusal_slot();
    sheet.column.append(&refused);
    let b = outline(&lines.decline);
    b.add_css_class("refuse");
    b.set_widget_name("usage.decline");
    // The act is the note as it stands at the tap: what the key's touch then signs.
    let (w2, mode, r2) = (w.clone(), ctx.mode(), r.clone());
    // Declined: the sheet closes and the requests are read again.
    let (dialog, reload) = (sheet.dialog.clone(), ctx.reload.clone());
    let then: Rc<dyn Fn()> = Rc::new(move || {
        dialog.close();
        reload();
    });
    b.connect_clicked(move |_| {
        let act = decline_act(&r2, &note.text());
        crate::approval::refuse_then(&w2, mode, &act, None, vec![], &refused, Some(then.clone()))
    });
    sheet.column.append(&b);
    sheet.dialog.present(Some(over));
    if OPEN.with(|o| o.get()) == Some("decline-tapped") {
        OPEN.with(|o| o.set(None));
        gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(400), move || {
            b.emit_clicked()
        });
    }
}

/// A screenshot's sheet, once the page is read: Ask an admin's, or the first request's Decline
/// with Decline tapped.
fn open_sheet(ctx: &Ctx, column: &gtk::Box) {
    let Some(which) = OPEN.with(|o| o.get()) else {
        return;
    };
    let (ctx, column) = (ctx.clone(), column.clone());
    gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
        let s = ctx.state.borrow();
        let Some(u) = s.usage.clone() else { return };
        drop(s);
        if which == "ask" {
            OPEN.with(|o| o.set(None));
            if let Some(raise) = asks(&u) {
                ask_admin(&ctx, raise, column.upcast_ref());
            }
        } else if let Some(r) = u
            .requests
            .as_ref()
            .and_then(|r| r.iter().find(|r| r.status == "open"))
        {
            decline(&ctx, r, &u, column.upcast_ref());
        }
    });
}

/// Raise the limit's place (`ApprovalGate`): the button while this computer's security key can
/// sign it (`key_approval::can_sign`: the key's touch and PIN, then the device key), else why this
/// computer can't approve, as before. The key is never assumed: without one, nothing is offered.
fn raise_or_gate(ctx: &Ctx, button: &gtk::Button, offer: &tafs_core::usage::Offer) -> gtk::Widget {
    let act = Act::RaiseLimit {
        from_cents: offer.from_cents.unwrap_or_default(),
    };
    let kept = crate::store::load(chrono::Utc::now().timestamp());
    if ctx.mode() == Mode::Kiff && crate::key_approval::can_sign(kept.as_ref(), &act) {
        return button.clone().upcast();
    }
    crate::approval::gate(&ctx.w, ctx.mode(), button)
}

/// `requestsPanel`, for someone who manages billing: members' open requests, in the amber,
/// each its line and reason, then Raise the limit (where tafs cloud offers it) and Decline, which
/// opens its sheet. Both are signed by the device key after the security key's touch and PIN
/// (`Ctx::raise`, `decline`); without a key, their tap says this computer can't approve, and sends
/// nothing. Approving a pack buys it in the phone's store: here, where to buy it (ruling A).
fn requests_panel(ctx: &Ctx, column: &gtk::Box, u: &KiffUsage, manages_billing: bool, s: &State) {
    let open = open_requests(u, manages_billing);
    if open.is_empty() {
        return;
    }
    let w = &ctx.w;
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
        let offer = s.balance.as_ref().and_then(|b| b.balance.raise.clone());
        if let (true, Some(offer)) = (r.kind == "raise", offer) {
            // `answerRaise`: raising answers every open request for one.
            let raise = crate::widgets::primary(&l("usage.raise_limit_button"));
            raise.set_widget_name(&format!("usage.raise.{}", r.id));
            let (ctx2, shown) = (ctx.clone(), offer.clone());
            raise.connect_clicked(move |b| {
                ctx2.raise(&offer, b.upcast_ref(), |s, n| s.plan_refused = n)
            });
            // `ApprovalGate`, as the iPad's: with the security key, Raise; without, why in its place.
            buttons.append(&raise_or_gate(ctx, &raise, &shown));
        }
        let packs_sell = s
            .balance
            .as_ref()
            .and_then(|b| b.balance.buy.as_ref())
            .is_some_and(|b| b.packs);
        if r.kind != "raise" && packs_sell {
            // The iPad's Approve buys the pack in its store: on a computer, while packs sell,
            // where to buy it.
            one.append(&text(&l("billing.buy_on_phone"), 14, 400, &["tafs-soft"]));
        }
        let decline_button = outline(&l("usage.requests.decline"));
        decline_button.set_widget_name(&format!("usage.decline.{}", r.id));
        let (ctx, r, u) = (ctx.clone(), r.clone(), u.clone());
        decline_button.connect_clicked(move |b| decline(&ctx, &r, &u, b.upcast_ref()));
        buttons.append(&decline_button);
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
