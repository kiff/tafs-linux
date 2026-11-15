//! tafs for Linux: GTK 4 and libadwaita. The same product, account and words as the iPad app.

mod account;
mod kiff;
mod needs_you;
mod overview;
mod record;
mod shell;
mod sign_in;
mod store;
mod style;
mod taf;
mod usage;
mod welcome;
mod widgets;

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
    home: Option<String>,
    /// Synthetic answers for Plan & usage signed in with KIFF, for its screenshots.
    usage: Option<String>,
    screenshot: Option<String>,
}

const USAGE: &str = "tafs [--lang en|de|fr|es|it|pt|nl|pl] [--theme light|dark] [--sample] [--signed-in] [--view-only]
     [--home contract.json] [--usage answers.json] [--page welcome|sign-in|overview|needs-you|record|calendar|usage|connect-apps|account|taf|taf-card] [--screenshot out.png]";

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
            "--sample" => o.sample = true,
            "--signed-in" => o.signed_in = true,
            "--view-only" => o.view_only = true,
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
        Some("calendar") => Selection::Calendar,
        Some("usage") => Selection::Usage,
        Some("connect-apps") => Selection::ConnectApps,
        Some("account") => Selection::Account,
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
        .default_width(WIDTH)
        .default_height(HEIGHT)
        .title(words.line("app.name").unwrap_or_default())
        .build();
    window.set_size_request(900, 600);

    let app_state = Rc::new(App {
        window: window.clone(),
        words,
        languages,
        signed_in: o.signed_in,
        first: RefCell::new(selection_of(o.page.as_deref())),
        kept: RefCell::new(None),
        sign_out_error: RefCell::new(None),
        cloud: RefCell::new(None),
        problem: RefCell::new(None),
        home_file: o.home.as_ref().map(std::path::PathBuf::from),
        usage_file: o.usage.as_ref().map(std::path::PathBuf::from),
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
        (Some("welcome") | Some("sign-in"), _) | (_, None) => {
            app_state.show_welcome();
            if o.page.as_deref() == Some("sign-in") {
                // A screenshot shows a code without reaching KIFF.
                app_state.present_sign_in(screenshot.then(|| sign_in::preview(lang)));
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
    /// A screenshot: the sample's Refunds has the reference set's three Cards.
    screenshot: bool,
    /// `--page taf-card`: the taf opens on its Card tab.
    on_card: bool,
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
                Read::Cloud(Box::new(Cloud {
                    home,
                    held,
                    account,
                    activity,
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
                .map(|(t, cards)| taf::TafPage {
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
                    cards,
                    issued_by: issuer.clone(),
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
        let at = chrono::Local::now();
        let fail = |e: tafs_core::words::WordsError| -> ! { panic!("{e}") };
        let (mut model, tafs, cloud_words) = match (mode, &cloud) {
            (Mode::Sample, _) => (
                sample::sidebar(words, at, self.signed_in).unwrap_or_else(|e| fail(e)),
                sample::overview(words, at).unwrap_or_else(|e| fail(e)),
                None,
            ),
            (_, Some(c)) => {
                let (model, tafs) =
                    home_model(words, &c.home, &c.held, at).unwrap_or_else(|e| fail(e));
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
            cloud_names: Rc::new(
                tafs.iter()
                    .filter_map(|t| Some((t.cloud_id.clone()?, t.name.clone())))
                    .collect(),
            ),
            tafs,
            actions,
        };
        let shell = shell::build(
            words.clone(),
            &model,
            screens,
            view_only,
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
