//! tafs for Linux: GTK 4 and libadwaita. The same product, account and words as the iPad app.

mod account;
mod calendar;
mod connect;
mod devices;
mod kiff;
mod kiff_account;
mod needs_you;
mod overview;
mod pairing;
mod people;
mod record;
mod shell;
mod sign_in;
mod store;
mod style;
mod taf;
mod usage;
mod welcome;
mod widgets;
mod zones;

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::account::{Account, Action as AccountAction};
use tafs_core::held::CardException;
use tafs_core::home::Home;
use tafs_core::language;
use tafs_core::sample;
use tafs_core::session::{ApprovalMode, KeptSession};
use tafs_core::sidebar::{Mode, Selection, SidebarModel};
use tafs_core::words::{WordBook, SHIPPED};

/// What the account's screens are drawn from: Home from tafs cloud, and KIFF's held calls.
struct Cloud {
    home: Home,
    held: Vec<CardException>,
    /// The KIFF account (`GET /v1/me`), once read.
    account: Option<Account>,
    /// KIFF's activity, for the Record.
    activity: Vec<tafs_core::record::ActivityCall>,
    /// "Did this happen?": the money calls KIFF can't prove (`unproven`).
    unproven: Vec<tafs_core::unproven::Unproven>,
    /// What this person may do with each taf, by KIFF agent (`GET /v1/me/access`).
    access: std::collections::BTreeMap<String, tafs_core::access::Access>,
    /// Who each taf this person manages is shared with, in the Overview's words, by KIFF agent.
    used_by: std::collections::BTreeMap<String, String>,
    /// Each tafs cloud taf's last run, by its id.
    reports: std::collections::BTreeMap<String, tafs_core::access::RunReport>,
}

/// The window's size, a laptop's, as the screenshots are saved.
const WIDTH: i32 = 1280;
const HEIGHT: i32 = 800;

#[derive(Debug, Default, Clone)]
struct Options {
    lang: Option<String>,
    theme: Option<String>,
    page: Option<String>,
    sample: bool,
    signed_in: bool,
    view_only: bool,
    /// With `--view-only`: a computer already added with the phone.
    paired: bool,
    home: Option<String>,
    /// Synthetic answers for Plan & usage signed in with KIFF, for its screenshots.
    usage: Option<String>,
    /// Synthetic answers for People & groups signed in with KIFF, for its screenshot.
    people: Option<String>,
    /// With `--home`: KIFF's "Did this happen?" answer from a file, for its screenshot.
    unproven: Option<String>,
    /// `--size WxH`: the window's size, for a screenshot of a narrower layout.
    size: Option<(i32, i32)>,
    screenshot: Option<String>,
}

const USAGE: &str = "tafs [--lang en|de|fr|es|it|pt|nl|pl] [--theme light|dark] [--sample] [--signed-in] [--view-only [--paired]]
     [--home contract.json] [--usage answers.json] [--people answers.json] [--unproven answers.json] [--size WxH] [--page welcome|sign-in|pair|overview|needs-you|record|calendar|calendar-week|usage|connect-apps|account|kiff-account|time-zone|devices|people|taf|taf-card] [--screenshot out.png]";

fn options() -> Result<Options, String> {
    let mut o = Options::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--lang" => o.lang = Some(value()?),
            "--theme" => o.theme = Some(value()?),
            "--page" => o.page = Some(value()?),
            "--screenshot" => o.screenshot = Some(value()?),
            "--home" => o.home = Some(value()?),
            "--usage" => o.usage = Some(value()?),
            "--people" => o.people = Some(value()?),
            "--unproven" => o.unproven = Some(value()?),
            "--size" => {
                let v = value()?;
                let (w, h) = v.split_once('x').ok_or("--size takes WxH")?;
                o.size = Some((
                    w.parse().map_err(|_| "--size takes WxH")?,
                    h.parse().map_err(|_| "--size takes WxH")?,
                ));
            }
            "--sample" => o.sample = true,
            "--signed-in" => o.signed_in = true,
            "--view-only" => o.view_only = true,
            "--paired" => o.paired = true,
            "--help" | "-h" => return Err(USAGE.into()),
            other => return Err(format!("{other}?\n{USAGE}")),
        }
    }
    if let Some(l) = &o.lang {
        if !SHIPPED.contains(&l.as_str()) {
            return Err(format!("tafs doesn't speak {l}"));
        }
    }
    Ok(o)
}

fn main() -> glib::ExitCode {
    let options = match options() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return glib::ExitCode::FAILURE;
        }
    };
    gio::resources_register_include!("tafs.gresource").expect("the app's resources");
    let app = adw::Application::builder()
        .application_id("dev.kiff.tafs")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| activate(app, &options));
    // GTK reads no arguments of ours.
    app.run_with_args::<&str>(&[])
}

/// The page a screenshot or `--page` opens beside the sidebar.
fn selection_of(page: Option<&str>) -> Selection {
    match page {
        Some("needs-you") => Selection::NeedsYou,
        Some("record") => Selection::Record,
        Some("calendar") | Some("calendar-week") => Selection::Calendar,
        Some("usage") => Selection::Usage,
        Some("connect-apps") => Selection::ConnectApps,
        Some("account") => Selection::Account,
        Some("devices") => Selection::Devices,
        Some("people") => Selection::People,
        Some("kiff-account") => Selection::KiffAccount,
        Some("time-zone") => Selection::TimeZone,
        // The sample's Refunds, as the reference set's taf.png opens it.
        Some("taf") | Some("taf-card") => Selection::Taf("refunds".into()),
        // With `--home`, the contract's first taf.
        Some(other) if other.starts_with("taf:") => Selection::Taf(other["taf:".len()..].into()),
        Some(other) if other.starts_with("taf-card:") => {
            Selection::Taf(other["taf-card:".len()..].into())
        }
        _ => Selection::Overview,
    }
}

fn activate(app: &adw::Application, o: &Options) {
    let preferred = language::preferred_from_env();
    let lang: &'static str = match &o.lang {
        Some(l) => SHIPPED
            .iter()
            .find(|s| **s == l.as_str())
            .copied()
            .unwrap_or("en"),
        None => language::app_language(&preferred),
    };
    let mut languages = vec![lang];
    languages.extend(
        language::screen_languages(&preferred)
            .into_iter()
            .filter(|l| *l != lang),
    );
    let words = match WordBook::load(&WordBook::default_root(), lang) {
        Ok(w) => Rc::new(w),
        Err(e) => {
            // No words, no screen: never a line in another language or a {hole}.
            eprintln!("{e}. Run tools/sync.sh: the words at the pin.");
            app.quit();
            return;
        }
    };
    let screenshot = o.screenshot.is_some();
    // The appearance: `--theme`, else Account's choice (a screenshot never reads it), else the desktop's.
    let theme = o.theme.clone().unwrap_or_else(|| {
        if screenshot {
            "system".into()
        } else {
            store::theme()
        }
    });
    adw::StyleManager::default().set_color_scheme(account::scheme(&theme));
    let display = gtk::gdk::Display::default().expect("a display");
    style::install(&display);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(o.size.map_or(WIDTH, |s| s.0))
        .default_height(o.size.map_or(HEIGHT, |s| s.1))
        .title(words.line("app.name").unwrap_or_default())
        .build();
    window.set_size_request(900, 600);

    let app_state = Rc::new(App {
        window: window.clone(),
        words,
        languages,
        signed_in: o.signed_in,
        paired: o.paired,
        first: RefCell::new(selection_of(o.page.as_deref())),
        kept: RefCell::new(None),
        sign_out_error: RefCell::new(None),
        cloud: RefCell::new(None),
        problem: RefCell::new(None),
        home_file: o.home.as_ref().map(std::path::PathBuf::from),
        usage_file: o.usage.as_ref().map(std::path::PathBuf::from),
        people_file: o.people.as_ref().map(std::path::PathBuf::from),
        calendar_day: o.page.as_deref() == Some("calendar"),
        screenshot: o.screenshot.is_some(),
        on_card: o
            .page
            .as_deref()
            .is_some_and(|p| p == "taf-card" || p.starts_with("taf-card:")),
    });

    // What opens: the sample when asked, else the kept session, else the welcome. A screenshot
    // never reads or writes the keyring.
    let now = chrono::Utc::now().timestamp();
    let kept = if screenshot || o.sample {
        None
    } else {
        store::load(now)
    };
    let from_file = o.home.as_deref().map(|path| match read_contract(path) {
        Some(home) => Rc::new(Cloud {
            home,
            held: vec![],
            account: None,
            activity: vec![],
            unproven: unproven_file(o.unproven.as_deref()),
            access: Default::default(),
            used_by: Default::default(),
            reports: Default::default(),
        }),
        None => {
            eprintln!("{path} isn't Home as tafs cloud answers it");
            std::process::exit(2);
        }
    });
    match (o.page.as_deref(), kept) {
        _ if o.sample => app_state.open_shell(Mode::Sample, false, None),
        _ if from_file.is_some() => app_state.open_shell(Mode::Kiff, o.view_only, from_file),
        _ if o.view_only => app_state.open_shell(Mode::Kiff, true, None),
        (Some("welcome") | Some("sign-in") | Some("pair"), _) | (_, None) => {
            app_state.show_welcome();
            if o.page.as_deref() == Some("sign-in") {
                // A screenshot shows a code without reaching KIFF.
                app_state.present_sign_in(screenshot.then(|| sign_in::preview(lang)));
            }
            if o.page.as_deref() == Some("pair") {
                // A screenshot shows a code made here, never claimed.
                app_state.present_pairing(screenshot);
            }
        }
        (_, Some(kept)) => app_state.open_account(kept),
    }

    window.present();
    if let Some(path) = o.screenshot.clone() {
        let (window, app) = (window.clone(), app.clone());
        glib::timeout_add_local_once(std::time::Duration::from_millis(1500), move || {
            match save_screenshot(window.upcast_ref(), &path) {
                Ok(()) => println!("saved {path}"),
                Err(e) => {
                    eprintln!("screenshot: {e}");
                    std::process::exit(1);
                }
            }
            app.quit();
        });
    }
}

/// The window and what it shows: the welcome, or the shell over the sample or the account.
struct App {
    window: adw::ApplicationWindow,
    words: Rc<WordBook>,
    languages: Vec<&'static str>,
    /// `--signed-in`: the sample with the account's items, as the reference set draws it.
    /// `--paired`: the view-only notice of a computer already added with the phone.
    paired: bool,
    signed_in: bool,
    /// What opens beside the sidebar.
    first: RefCell<Selection>,
    /// The session signed in, for signing out.
    kept: RefCell<Option<KeptSession>>,
    /// Why signing out didn't work, shown in Account.
    sign_out_error: RefCell<Option<String>>,
    /// What was last read for the account.
    cloud: RefCell<Option<Rc<Cloud>>>,
    /// The welcome's line for why it's there, while it's open.
    problem: RefCell<Option<gtk::Label>>,
    /// `--home`'s contract file, for the screenshots' tafs cloud reads.
    home_file: Option<std::path::PathBuf>,
    /// `--usage`'s file of synthetic answers.
    usage_file: Option<std::path::PathBuf>,
    /// `--people`'s file of synthetic answers.
    people_file: Option<std::path::PathBuf>,
    /// A screenshot: the sample's Refunds has the reference set's three Cards.
    screenshot: bool,
    /// `--page taf-card`: the taf opens on its Card tab.
    on_card: bool,
    /// `--page calendar`: the calendar opens on Day, as the reference set's `calendar.png`.
    calendar_day: bool,
}

/// What reading the account came to.
enum Read {
    Cloud(Box<Cloud>),
    /// KIFF answered 401: the session is over.
    SignedOut,
    /// Nothing that could be read: the screen stays as it was.
    Nothing,
}

impl App {
    /// A line on the welcome, in red: why it's there.
    fn welcome_problem(&self, words: &str) {
        if let Some(label) = self.problem.borrow().as_ref() {
            label.set_label(words);
            label.set_visible(true);
        }
    }

    /// The welcome (`OnboardingView`), with nothing kept.
    fn show_welcome(self: &Rc<Self>) {
        // Signed out: no account, so this computer's zone.
        tafs_core::account_time::use_zone(None);
        *self.kept.borrow_mut() = None;
        *self.cloud.borrow_mut() = None;
        *self.sign_out_error.borrow_mut() = None;
        *self.first.borrow_mut() = Selection::Overview;
        let welcome = welcome::build(&self.words);
        self.window.set_content(Some(&welcome.root));
        *self.problem.borrow_mut() = Some(welcome.problem.clone());
        let me = self.clone();
        welcome
            .sign_in
            .connect_clicked(move |_| me.present_sign_in(None));
        let me = self.clone();
        welcome
            .sample
            .connect_clicked(move |_| me.open_shell(Mode::Sample, false, None));
        let me = self.clone();
        welcome
            .pair
            .connect_clicked(move |_| me.present_pairing(false));
    }

    /// Adding this computer with a phone, over what's open. A session kept here (view-only) is
    /// the one the new one replaces. `preview`: a code made here, for a screenshot.
    fn present_pairing(self: &Rc<Self>, preview: bool) {
        let me = self.clone();
        let done: Rc<dyn Fn(KeptSession)> = Rc::new(move |kept| me.open_account(kept));
        pairing::present(
            &self.window,
            self.words.clone(),
            self.languages.clone(),
            self.kept.borrow().clone(),
            done,
            preview,
        );
    }

    /// Sign in with KIFF over what's open.
    fn present_sign_in(self: &Rc<Self>, preview: Option<tafs_core::signin::DeviceStart>) {
        let me = self.clone();
        let done: Rc<dyn Fn(KeptSession)> = Rc::new(move |kept| me.open_account(kept));
        sign_in::present(
            &self.window,
            self.window.upcast_ref(),
            self.words.clone(),
            self.languages.clone(),
            done,
            preview,
        );
    }

    /// Signed in: the shell at once, then the account (`GET /v1/me`), Home and the held calls,
    /// drawn again once read. An answer that can't be read leaves it as it was.
    fn open_account(self: &Rc<Self>, kept: KeptSession) {
        let view_only = kept.approval_mode.is_view_only();
        *self.kept.borrow_mut() = Some(kept.clone());
        self.open_shell(Mode::Kiff, view_only, None);
        let (me, languages, token) = (self.clone(), self.languages.clone(), kept.token.clone());
        let words: WordBook = (*self.words).clone();
        glib::spawn_future_local(async move {
            let read = gio::spawn_blocking(move || {
                let signed_out = |a: &tafs_core::signin::Answer| a.status == 401;
                let me = kiff::me(&token, &languages).ok();
                let home = kiff::home(&token, &languages).ok();
                let held = kiff::held(&token, &languages).ok();
                let activity = kiff::activity(&token, &languages).ok();
                // KIFF ended the session (a 401 on any of them: the computer removed in Devices,
                // the account gone): this computer is signed out (`forgetSession`).
                if [&me, &home, &held, &activity]
                    .into_iter()
                    .flatten()
                    .any(signed_out)
                {
                    return Read::SignedOut;
                }
                let account = me.and_then(|a| Account::read(&a).ok());
                // "Did this happen?", only for someone who may answer holds (`loadUnproven`).
                // None read, none shown; a 401 there signs this computer out as anywhere.
                let unproven = if account.as_ref().is_none_or(|a| a.allows("holds.answer")) {
                    match kiff::did_this_happen(&token, &languages) {
                        Ok(a) if signed_out(&a) => return Read::SignedOut,
                        Ok(a) if a.status == 200 => {
                            tafs_core::unproven::read_unproven(&a.body).unwrap_or_default()
                        }
                        _ => vec![],
                    }
                } else {
                    vec![]
                };
                let Some(home) = home
                    .filter(|a| a.status == 200)
                    .and_then(|a| Home::read(&a.body))
                else {
                    return Read::Nothing;
                };
                // KIFF's held calls: none read, none shown, and Home still is.
                let held = held
                    .filter(|a| a.status == 200)
                    .and_then(|a| tafs_core::held::read_held(&a.body))
                    .unwrap_or_default();
                // KIFF's activity: none read, an empty Record, and the rest still is.
                let activity = activity
                    .filter(|a| a.status == 200)
                    .and_then(|a| tafs_core::record::read_activity(&a.body))
                    .unwrap_or_default();
                let (access, used_by, reports) = taf_reads(&words, &token, &languages, &home);
                Read::Cloud(Box::new(Cloud {
                    home,
                    held,
                    account,
                    activity,
                    unproven,
                    access,
                    used_by,
                    reports,
                }))
            })
            .await
            .unwrap_or(Read::Nothing);
            match read {
                Read::Cloud(cloud) => {
                    // KIFF's word for how the session approves is what counts (contract §1.2).
                    let view_only = match cloud
                        .account
                        .as_ref()
                        .and_then(|a| a.approval_mode.as_deref())
                    {
                        Some(word) => ApprovalMode::from_kiff(Some(word)).is_view_only(),
                        None => view_only,
                    };
                    me.open_shell(Mode::Kiff, view_only, Some(Rc::new(*cloud)));
                }
                Read::SignedOut => me.signed_out_by_kiff(),
                Read::Nothing => {}
            }
        });
    }

    /// Each taf's page: the sample's tafs with their Cards and states, else Home's.
    fn taf_pages(
        &self,
        mode: Mode,
        model: &SidebarModel,
        cloud: Option<&Cloud>,
        may_answer: bool,
        at: chrono::DateTime<chrono::Local>,
    ) -> Vec<taf::TafPage> {
        let w = &self.words;
        let holders: Vec<tafs_core::needs_you::Holder> = model
            .tafs
            .iter()
            .map(|(t, _)| tafs_core::needs_you::Holder {
                key: &t.key,
                name: &t.name,
                initial: &t.initial,
                color: t.color_index,
            })
            .collect();
        // A taf's held calls, as Needs you draws them.
        let held_of = |key: &str| -> Vec<tafs_core::needs_you::CardLines> {
            let calls: Vec<_> = model
                .held
                .iter()
                .filter(|c| c.taf == key)
                .cloned()
                .collect();
            tafs_core::needs_you::lines(w, &calls, &holders, may_answer, at)
                .map(|l| l.cards)
                .unwrap_or_default()
        };
        // Where a tafs cloud taf's Activity is read: tafs cloud as the session, else the contract
        // beside `--home`'s.
        let source = match (self.kept.borrow().as_ref(), &self.home_file) {
            (Some(kept), _) => taf::ActivitySource::Cloud {
                token: kept.token.clone(),
                languages: self.languages.clone(),
            },
            (None, Some(home)) => {
                taf::ActivitySource::File(home.with_file_name("taf-activity.json"))
            }
            _ => taf::ActivitySource::None,
        };
        // `issuer(of:)`: who manages a taf isn't read yet, so a KIFF taf's Cards are the account's.
        let issuer = cloud
            .and_then(|c| c.account.as_ref())
            .map(|a| a.name())
            .or_else(|| w.line("card.issued_by_account").ok())
            .unwrap_or_default();
        match (mode, cloud) {
            (Mode::Sample, _) => model
                .tafs
                .iter()
                .map(|(t, _)| {
                    let waits = model.held.iter().any(|c| c.taf == t.key);
                    taf::TafPage {
                        key: t.key.clone(),
                        name: t.name.clone(),
                        initial: t.initial.clone(),
                        color: t.color_index,
                        state: if waits {
                            tafs_core::taf_page::TafState::Waiting
                        } else {
                            tafs_core::taf_page::TafState::Working
                        },
                        attention: false,
                        card: sample::card(w, &t.key).ok(),
                        card_title: t.name.clone(),
                        card_environment: String::new(),
                        card_live: true,
                        card_active: true,
                        card_count: 1,
                        revoked: false,
                        held: held_of(&t.key),
                        sample: true,
                        manages: false,
                        cards: sample::full_cards(w, &t.key, self.screenshot).unwrap_or_default(),
                        issued_by: w.line("common.you").unwrap_or_default(),
                        opens_on_card: self.on_card,
                        cloud_id: None,
                        source: taf::ActivitySource::None,
                    }
                })
                .collect(),
            (_, Some(c)) => c
                .home
                .tafs
                .iter()
                .filter_map(|h| {
                    let cards = tafs_core::home::full_cards(w, h, w.language()).ok()?;
                    Some((tafs_core::home::taf(w, h).ok()?, cards))
                })
                .map(|(t, cards)| (tafs_core::access::manages(c.access.get(&t.agent)), t, cards))
                .map(|(manages, t, cards)| taf::TafPage {
                    held: held_of(&t.key),
                    initial: tafs_core::sidebar::initial(&t.name),
                    cloud_id: t.cloud.then(|| t.key.clone()),
                    key: t.key,
                    name: t.name,
                    color: t.color_index,
                    state: t.state,
                    attention: t.attention,
                    card: t.card,
                    card_title: t.card_title,
                    card_environment: t.card_environment,
                    card_live: t.card_live,
                    card_active: t.card_active,
                    card_count: t.card_count,
                    revoked: t.revoked,
                    sample: false,
                    manages,
                    cards,
                    // `issuer(of:)`: whoever manages the taf sees who issued it; else the account.
                    issued_by: if manages {
                        w.line("common.you").unwrap_or_default()
                    } else {
                        issuer.clone()
                    },
                    opens_on_card: self.on_card,
                    source: source.clone(),
                })
                .collect(),
            _ => vec![],
        }
    }

    /// The Record's lines: the sample's, or KIFF's activity for the tafs Home lists, by the
    /// agent each call was made as. Its chips are every taf.
    fn record_data(
        &self,
        mode: Mode,
        model: &SidebarModel,
        cloud: Option<&Cloud>,
        at: chrono::DateTime<chrono::Local>,
    ) -> record::RecordData {
        // The chips in the tafs' own order (`model.tafs`): the sample's as it made them
        // (`TafTemplates.all`), else tafs cloud's, not the sidebar's waiting-first order.
        let mut tafs: Vec<record::Taf> = model
            .tafs
            .iter()
            .map(|(t, _)| record::Taf {
                key: t.key.clone(),
                name: t.name.clone(),
                color: t.color_index,
            })
            .collect();
        let order: Vec<String> = match (mode, cloud) {
            (Mode::Sample, _) => ["refunds", "bill_pay", "access", "deploys", "chargebacks"]
                .map(String::from)
                .to_vec(),
            (_, Some(c)) => c
                .home
                .tafs
                .iter()
                .map(|t| {
                    t.id.clone()
                        .unwrap_or_else(|| format!("agent:{}", t.kiff_agent))
                })
                .collect(),
            _ => vec![],
        };
        tafs.sort_by_key(|t| order.iter().position(|k| *k == t.key).unwrap_or(usize::MAX));
        let entries = match (mode, cloud) {
            (Mode::Sample, _) => sample::record(&self.words, at).unwrap_or_default(),
            (_, Some(c)) => {
                let agents: Vec<(String, String)> = c
                    .home
                    .tafs
                    .iter()
                    .map(|t| {
                        (
                            t.kiff_agent.clone(),
                            t.id.clone()
                                .unwrap_or_else(|| format!("agent:{}", t.kiff_agent)),
                        )
                    })
                    .collect();
                let taf_of = |actor: &str| {
                    agents
                        .iter()
                        .find(|(a, _)| a == actor)
                        .map(|(_, k)| k.clone())
                };
                tafs_core::record::record(&self.words, &c.activity, &taf_of).unwrap_or_default()
            }
            _ => vec![],
        };
        // `isWaiting`: the sample's held lines wait; KIFF's while KIFF still holds their call.
        let still_held: Vec<String> = model
            .held
            .iter()
            .filter_map(|c| c.proposal_id.clone().or_else(|| c.kiff_id.clone()))
            .map(|id| tafs_core::record::record_id(&id))
            .collect();
        let kiff = mode == Mode::Kiff;
        record::RecordData {
            entries,
            tafs,
            waiting: Rc::new(move |e| !kiff || still_held.contains(&e.id)),
        }
    }

    /// KIFF ended the session: forgotten here with its key, and the welcome says so.
    fn signed_out_by_kiff(self: &Rc<Self>) {
        forget_kept();
        self.show_welcome();
        if let Ok(words) = self.words.line("error.signed_out") {
            self.welcome_problem(&words);
        }
    }

    /// The shell: over the sample, or the account's with what's been read.
    fn open_shell(self: &Rc<Self>, mode: Mode, view_only: bool, cloud: Option<Rc<Cloud>>) {
        let words = &self.words;
        *self.cloud.borrow_mut() = cloud.clone();
        // `AccountTime.use`: signed in with KIFF, every time in the account's zone (UTC when
        // KIFF says none); the sample, or no account read yet, this computer's.
        let kiff_account = cloud.as_ref().and_then(|c| c.account.as_ref());
        tafs_core::account_time::use_zone(match (mode, kiff_account) {
            (Mode::Kiff, Some(a)) => Some(a.time_zone.as_deref().unwrap_or("UTC")),
            _ => None,
        });
        let at = chrono::Local::now();
        let fail = |e: tafs_core::words::WordsError| -> ! { panic!("{e}") };
        let (mut model, tafs, cloud_words) = match (mode, &cloud) {
            (Mode::Sample, _) => (
                sample::sidebar(words, at, self.signed_in).unwrap_or_else(|e| fail(e)),
                sample::overview(words, at).unwrap_or_else(|e| fail(e)),
                None,
            ),
            (_, Some(c)) => {
                let (model, mut tafs) =
                    home_model(words, &c.home, &c.held, at).unwrap_or_else(|e| fail(e));
                // Who uses each taf this person manages, and each tafs cloud taf's last run.
                for o in tafs.iter_mut() {
                    let agent = c
                        .home
                        .tafs
                        .iter()
                        .filter_map(|h| tafs_core::home::taf(words, h).ok())
                        .find(|t| t.key == o.key)
                        .map(|t| t.agent);
                    if let Some(u) = agent.and_then(|a| c.used_by.get(&a)) {
                        o.used_by = u.clone();
                    }
                    if let (Some(run), Some(r)) = (o.cloud.as_mut(), c.reports.get(&o.key)) {
                        run.report = Some(r.text.clone());
                        run.error = r.error.clone();
                    }
                }
                (
                    model,
                    tafs,
                    c.home.needs_you.as_ref().map(|n| n.words.clone()),
                )
            }
            (_, None) => (kiff_model(), vec![], None),
        };
        let account = cloud.as_ref().and_then(|c| c.account.clone());
        if let Some(a) = &account {
            model.account_name = Some(a.name());
            // `mayUseCoach`: Coach's access, until that's read, the account's members.manage.
            model.may_use_coach = a
                .capabilities
                .as_ref()
                .is_some_and(|c| c.iter().any(|x| x == "members.manage"));
        }
        let count = model.needs_you_count();
        let needs_you_words = match cloud_words {
            Some(words) => words,
            None => tafs_core::overview::needs_you_words(words, count).unwrap_or_else(|e| fail(e)),
        };
        let overview =
            tafs_core::overview::lines(words, &tafs, &model.held, count, &needs_you_words)
                .unwrap_or_else(|e| fail(e));
        let holders: Vec<tafs_core::needs_you::Holder> = model
            .tafs
            .iter()
            .map(|(t, _)| tafs_core::needs_you::Holder {
                key: &t.key,
                name: &t.name,
                initial: &t.initial,
                color: t.color_index,
            })
            .collect();
        let may_answer = account.as_ref().is_none_or(|a| a.allows("holds.answer"));
        let needs_you = tafs_core::needs_you::lines(words, &model.held, &holders, may_answer, at)
            .unwrap_or_else(|e| fail(e));
        // Home's steps and Coach's waits, signed in.
        let needs_you = match &cloud {
            Some(c) => tafs_core::needs_you::with_unproven(
                tafs_core::needs_you::with_waits(
                    needs_you,
                    &c.home,
                    model.may_use_coach,
                    // `canDiscuss`'s `manages`: the tafs cloud taf's KIFF agent, as access reads it.
                    &|taf: &str| {
                        c.home
                            .tafs
                            .iter()
                            .chain(&c.home.others)
                            .find(|t| t.id.as_deref() == Some(taf))
                            .is_some_and(|t| {
                                tafs_core::access::manages(c.access.get(&t.kiff_agent))
                            })
                    },
                ),
                words,
                if may_answer { &c.unproven } else { &[] },
            )
            .unwrap_or_else(|e| fail(e)),
            None => needs_you,
        };
        // Knocks at the tafs' doors, in the tafs' own order, after the held calls.
        let needs_you = match &cloud {
            Some(c) => {
                let knocks = c.home.knocks();
                let knocked: Vec<tafs_core::needs_you::Knocked> = model
                    .tafs
                    .iter()
                    .map(|(t, _)| tafs_core::needs_you::Knocked {
                        key: &t.key,
                        name: &t.name,
                        initial: &t.initial,
                        color: t.color_index,
                        knocks: knocks.get(&t.key).copied().unwrap_or(0),
                    })
                    .collect();
                tafs_core::needs_you::with_knocks(needs_you, words, &knocked)
                    .unwrap_or_else(|e| fail(e))
            }
            None => needs_you,
        };
        let account_lines = tafs_core::account::lines(
            words,
            mode,
            account.as_ref(),
            self.sign_out_error.borrow().as_deref(),
            sign_in::TERMS,
            sign_in::PRIVACY,
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap_or_else(|e| fail(e));
        let me = self.clone();
        let actions: Rc<dyn Fn(AccountAction)> =
            Rc::new(move |action| me.act(action, mode, view_only));
        let record = Rc::new(self.record_data(mode, &model, cloud.as_deref(), at));
        let tafs = Rc::new(self.taf_pages(mode, &model, cloud.as_deref(), may_answer, at));
        let screens = shell::Screens {
            overview,
            needs_you,
            account: account_lines,
            record,
            usage: match (mode, self.kept.borrow().as_ref()) {
                (Mode::Sample, _) => usage::UsageSource::Sample,
                (Mode::Kiff, Some(kept)) => usage::UsageSource::Cloud {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                    manages_billing: cloud
                        .as_deref()
                        .and_then(|c| c.account.as_ref())
                        .is_none_or(|a| a.allows("billing.manage")),
                },
                _ => match &self.usage_file {
                    Some(f) => usage::UsageSource::File(f.clone()),
                    None => usage::UsageSource::None,
                },
            },
            zones: match (mode, self.kept.borrow().as_ref(), &self.people_file) {
                (Mode::Kiff, Some(kept), _) => zones::ZonesSource::Cloud {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                },
                (Mode::Kiff, None, Some(f)) => zones::ZonesSource::File(f.clone()),
                _ => zones::ZonesSource::None,
            },
            people: match (mode, self.kept.borrow().as_ref(), &self.people_file) {
                (Mode::Kiff, Some(kept), _) => people::PeopleSource::Cloud {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                },
                (Mode::Kiff, None, Some(f)) => people::PeopleSource::File(f.clone()),
                _ => people::PeopleSource::None,
            },
            people_known: Rc::new(people::Known {
                // The account read; for a screenshot, the one `--people`'s file gives.
                account: cloud
                    .as_deref()
                    .and_then(|c| c.account.clone())
                    .or_else(|| {
                        let f = self.people_file.as_ref()?;
                        let v: serde_json::Value =
                            serde_json::from_slice(&std::fs::read(f).ok()?).ok()?;
                        serde_json::from_value(v["account"].clone()).ok()
                    }),
                agents: cloud
                    .as_deref()
                    .map(|c| {
                        c.home
                            .tafs
                            .iter()
                            .filter_map(|h| tafs_core::home::taf(words, h).ok())
                            .map(|t| (t.agent, t.name))
                            .collect()
                    })
                    .unwrap_or_default(),
            }),
            calendar: match (mode, self.kept.borrow().as_ref()) {
                (Mode::Kiff, Some(kept)) => calendar::CalendarSource::Cloud {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                },
                _ => calendar::CalendarSource::Empty,
            },
            // Week first; a screenshot's `--page calendar` opens Day, `calendar-week` Week.
            calendar_first: tafs_core::calendar::State {
                mode: if self.calendar_day {
                    tafs_core::calendar::Mode::Day
                } else {
                    tafs_core::calendar::Mode::Week
                },
                selected: tafs_core::calendar::today(),
            },
            devices: match (mode, self.kept.borrow().as_ref()) {
                (Mode::Sample, _) => devices::DevicesSource::Sample {
                    screenshot: self.screenshot,
                },
                (Mode::Kiff, Some(kept)) => devices::DevicesSource::Cloud {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                },
                _ => devices::DevicesSource::None,
            },
            connect: match (mode, self.kept.borrow().as_ref()) {
                (Mode::Sample, _) => connect::ConnectSource::Sample {
                    screenshot: self.screenshot,
                },
                (Mode::Kiff, Some(kept)) => connect::ConnectSource::Cloud {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                },
                _ => connect::ConnectSource::None,
            },
            cloud_names: Rc::new(
                tafs.iter()
                    .filter_map(|t| Some((t.cloud_id.clone()?, t.name.clone())))
                    .collect(),
            ),
            tafs,
            actions,
        };
        // View-only: before it's paired, the notice's "Add this computer" pairs it, replacing the
        // session it has; once paired, the notice says approvals go to the phone.
        let paired = self.paired || self.kept.borrow().as_ref().is_some_and(|kept| kept.paired);
        let add = view_only.then(|| {
            if paired {
                shell::ViewOnly::Paired
            } else {
                let me = self.clone();
                shell::ViewOnly::CanPair(Rc::new(move || me.present_pairing(false)))
            }
        });
        let shell = shell::build(
            words.clone(),
            &model,
            screens,
            add,
            self.first.borrow().clone(),
        );
        self.window.set_content(Some(&shell.root));
    }

    /// Account's actions: sign in from the sample, clear the sample, or sign out of KIFF.
    fn act(self: &Rc<Self>, action: AccountAction, mode: Mode, view_only: bool) {
        match action {
            AccountAction::SignIn => self.present_sign_in(None),
            // The sample is forgotten, and the welcome opens.
            AccountAction::ClearSample => self.show_welcome(),
            AccountAction::ChangeLanguage => {}
            AccountAction::SignOut => {
                let Ok((title, body, confirm)) = tafs_core::account::sign_out_question(&self.words)
                else {
                    return;
                };
                let dialog = adw::AlertDialog::new(Some(&title), Some(&body));
                dialog.add_response(
                    "cancel",
                    &self.words.line("common.cancel").unwrap_or_default(),
                );
                dialog.add_response("sign-out", &confirm);
                dialog.set_response_appearance("sign-out", adw::ResponseAppearance::Destructive);
                dialog.set_default_response(Some("cancel"));
                dialog.set_close_response("cancel");
                let me = self.clone();
                dialog.connect_response(None, move |_, response| {
                    if response == "sign-out" {
                        me.sign_out(mode, view_only);
                    }
                });
                dialog.present(Some(&self.window));
            }
        }
    }

    /// `AppModel.signOut`: KIFF ends the session (a 401 means it already had), then this
    /// computer forgets it and its key, and the welcome opens. When KIFF can't be reached, the
    /// session stays, and Account says why.
    fn sign_out(self: &Rc<Self>, mode: Mode, view_only: bool) {
        let Some(kept) = self.kept.borrow().clone() else {
            self.show_welcome();
            return;
        };
        *self.sign_out_error.borrow_mut() = None;
        let me = self.clone();
        glib::spawn_future_local(async move {
            let token = kept.token.clone();
            let result = gio::spawn_blocking(move || {
                tafs_core::account::signed_out(kiff::Kiff::new(vec![]).sign_out(&token))
            })
            .await
            .unwrap_or(Err(tafs_core::signin::KiffError::Unreadable));
            match result {
                Ok(()) => {
                    forget_kept();
                    me.show_welcome();
                }
                Err(e) => {
                    *me.sign_out_error.borrow_mut() =
                        tafs_core::account::not_signed_out(&me.words, &e).ok();
                    *me.first.borrow_mut() = Selection::Account;
                    let cloud = me.cloud.borrow().clone();
                    me.open_shell(mode, view_only, cloud);
                }
            }
        });
    }
}

/// Forgets the kept session and its key, tried twice. A keyring that still refuses is said in
/// the log by its fixed word; the token it keeps is one KIFF has ended, so the next launch's
/// reads get a 401 and forget it again.
fn forget_kept() {
    if let Err(first) = store::clear() {
        if let Err(again) = store::clear() {
            eprintln!(
                "the keyring kept the ended session: {} ({})",
                again.0, first.0
            );
        }
    }
}

/// A contract file's answer (`shared/contracts/home.json`), as tafs cloud would send it.
fn read_contract(path: &str) -> Option<Home> {
    let file: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Home::read(&serde_json::to_vec(file.get("answer")?).ok()?)
}

/// The account's sidebar and Overview from Home and KIFF's held calls: the tafs a call waits on
/// first, then in tafs cloud's order (`waitingFirst`), Your tafs those relevant now, and tafs
/// cloud's count of what waits.
fn home_model(
    w: &WordBook,
    h: &Home,
    exceptions: &[CardException],
    at: chrono::DateTime<chrono::Local>,
) -> Result<(SidebarModel, Vec<tafs_core::overview::OverviewTaf>), tafs_core::words::WordsError> {
    let tafs: Vec<tafs_core::home::Taf> = h
        .tafs
        .iter()
        .map(|t| tafs_core::home::taf(w, t))
        .collect::<Result<_, _>>()?;
    // Each held call on its taf, by the KIFF agent it's held for.
    let mut held = Vec::new();
    for e in exceptions {
        let key = tafs
            .iter()
            .find(|t| t.agent == e.subject)
            .map(|t| t.key.clone())
            .unwrap_or_else(|| format!("agent:{}", e.subject));
        held.push(tafs_core::held::held(w, e, &key)?);
    }
    let waits = |t: &tafs_core::home::Taf| held.iter().any(|c| c.taf == t.key);
    let ordered: Vec<&tafs_core::home::Taf> = tafs
        .iter()
        .filter(|t| waits(t))
        .chain(tafs.iter().filter(|t| !waits(t)))
        .collect();
    let rows = ordered
        .iter()
        .map(|t| {
            (
                tafs_core::home::row(t),
                tafs_core::home::relevant(t, waits(t), at),
            )
        })
        .collect();
    let overview = ordered
        .iter()
        .map(|t| tafs_core::home::overview_taf(t))
        .collect();
    let model = SidebarModel {
        list_loaded: true,
        tafs: rows,
        held,
        cloud_needs_you: h.needs_you.as_ref().map(|n| n.count),
        ..kiff_model()
    };
    Ok((model, overview))
}

/// The reads the Overview and a taf's Card tab add to Home's (`refreshAccess`, `shares`,
/// `loadReport`): this person's access to each taf; for each taf they manage, who it's shared
/// with; and each tafs cloud taf's last run. Each stands alone: one that fails leaves its part out.
type TafReads = (
    std::collections::BTreeMap<String, tafs_core::access::Access>,
    std::collections::BTreeMap<String, String>,
    std::collections::BTreeMap<String, tafs_core::access::RunReport>,
);

fn taf_reads(w: &WordBook, token: &str, languages: &[&'static str], home: &Home) -> TafReads {
    use tafs_core::access::*;
    let ok = |a: Result<tafs_core::signin::Answer, _>| {
        a.ok().filter(|a| a.status == 200).map(|a| a.body)
    };
    let tafs: Vec<tafs_core::home::Taf> = home
        .tafs
        .iter()
        .filter_map(|h| tafs_core::home::taf(w, h).ok())
        .collect();
    let access = ok(kiff::kiff_get(token, languages, ACCESS_PATH))
        .and_then(|b| read_access(&b))
        .unwrap_or_default();
    let managed: Vec<&tafs_core::home::Taf> = tafs
        .iter()
        .filter(|t| manages(access.get(&t.agent)))
        .collect();
    let mut used = std::collections::BTreeMap::new();
    if !managed.is_empty() {
        let groups = ok(kiff::kiff_get(
            token,
            languages,
            tafs_core::people::GROUPS_PATH,
        ))
        .and_then(|b| tafs_core::people::Group::read_all(&b))
        .unwrap_or_default();
        let people = ok(kiff::kiff_get(
            token,
            languages,
            tafs_core::people::PEOPLE_PATH,
        ))
        .and_then(|b| tafs_core::people::Person::read_all(&b))
        .unwrap_or_default();
        let (groups, people) = tafs_core::people::arrange(groups, people);
        for t in managed {
            if let Some(shares) = ok(kiff::kiff_get(token, languages, &shares_path(&t.agent)))
                .and_then(|b| read_shares(&b))
            {
                if let Ok(line) = used_by(w, &shares, &groups, &people) {
                    used.insert(t.agent.clone(), line);
                }
            }
        }
    }
    let mut reports = std::collections::BTreeMap::new();
    for t in tafs.iter().filter(|t| t.cloud) {
        let not_loaded = |why: String| RunReport {
            text: String::new(),
            error: Some(
                w.get("report.runs_not_loaded", &[("why", &why)])
                    .unwrap_or(why),
            ),
        };
        let read = || -> Result<RunReport, String> {
            let a = kiff::cloud(token, languages, &runs_path(&t.key))
                .map_err(|e| e.words(w).unwrap_or_default())?;
            if a.status != 200 {
                return Err(tafs_core::signin::refusal(&a, Some(w))
                    .words(w)
                    .unwrap_or_default());
            }
            let runs = read_runs(&a.body)
                .ok_or_else(|| tafs_core::signin::plain_words(w, 200).unwrap_or_default())?;
            let session = match latest_session(&runs) {
                Ok(s) => s,
                Err(none) => return Ok(none),
            };
            let a = kiff::cloud(token, languages, &report_path(&t.key, &session))
                .map_err(|e| e.words(w).unwrap_or_default())?;
            if a.status != 200 {
                return Err(tafs_core::signin::refusal(&a, Some(w))
                    .words(w)
                    .unwrap_or_default());
            }
            let (_, text) = read_report(&a.body)
                .ok_or_else(|| tafs_core::signin::plain_words(w, 200).unwrap_or_default())?;
            Ok(RunReport { text, error: None })
        };
        reports.insert(t.key.clone(), read().unwrap_or_else(not_loaded));
    }
    (access, used, reports)
}

/// The signed-in account's sidebar until Home is read from tafs cloud: no tafs listed yet.
fn kiff_model() -> SidebarModel {
    SidebarModel {
        mode: Mode::Kiff,
        screenshot_signed_in: false,
        may_use_coach: false,
        held: vec![],
        tafs: vec![],
        list_loaded: false,
        home_words: None,
        account_name: None,
        cloud_needs_you: None,
    }
}

/// `--unproven`'s file, as KIFF answers `GET /v1/me/did-this-happen`. None given, none waiting;
/// a file that can't be read stops the app, as a broken `--home` does.
fn unproven_file(path: Option<&str>) -> Vec<tafs_core::unproven::Unproven> {
    let Some(path) = path else {
        return vec![];
    };
    match std::fs::read(path)
        .ok()
        .and_then(|b| tafs_core::unproven::read_unproven(&b))
    {
        Some(calls) => calls,
        None => {
            eprintln!("{path} isn't \"Did this happen?\" as KIFF answers it");
            std::process::exit(2);
        }
    }
}

/// Saves the window, dialogs over it included, at twice its size, as a PNG.
fn save_screenshot(window: &gtk::Window, path: &str) -> Result<(), String> {
    let (w, h) = (window.width() as f32, window.height() as f32);
    let paintable = gtk::WidgetPaintable::new(Some(window));
    let snapshot = gtk::Snapshot::new();
    snapshot.scale(2.0, 2.0);
    paintable.snapshot(&snapshot, w as f64, h as f64);
    let node = snapshot.to_node().ok_or("nothing drawn")?;
    let renderer = window
        .native()
        .and_then(|n| n.renderer())
        .ok_or("no renderer")?;
    let bounds = gtk::graphene::Rect::new(0.0, 0.0, w * 2.0, h * 2.0);
    let texture = renderer.render_texture(&node, Some(&bounds));
    texture.save_to_png(path).map_err(|e| e.to_string())
}
