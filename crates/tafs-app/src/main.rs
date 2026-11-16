//! tafs for Linux: GTK 4 and libadwaita. The same product, account and words as the iPad app.

mod account;
mod ai_consent;
mod alerts;
mod approval;
mod brief;
mod calendar;
mod chat;
mod coach;
mod connect;
mod connected;
mod devices;
mod fido_key;
mod halt;
mod illustrations;
mod kiff;
mod kiff_account;
mod knocks;
mod language_picker;
mod needs_you;
mod overview;
mod pairing;
mod people;
mod pin_sheet;
mod receipt_sheet;
mod receipts;
mod recipes;
mod record;
mod record_search;
mod report_sheet;
mod runs_panel;
mod setup_pane;
mod sheet;
mod shell;
mod sign_in;
mod store;
mod style;
mod taf;
mod taf_chat;
mod usage;
mod welcome;
mod widgets;
mod work_panel;
mod zones;

use std::cell::{Cell, RefCell};
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
    /// What the Record's lines acted on (`nameRecordLines`): the call refs of each taf with a
    /// line KIFF hashed a resource for, by KIFF's key for the call.
    call_refs: std::collections::BTreeMap<String, String>,
    /// "Did this happen?": the money calls KIFF can't prove (`unproven`).
    unproven: Vec<tafs_core::unproven::Unproven>,
    /// The recipes Home lists with no taf (`recipes`); none read, none listed.
    recipes: Option<tafs_core::recipes::Recipes>,
    /// Every ready-made job (`gallery`); none read, no Explore row and no strip.
    gallery: Option<tafs_core::recipes::Gallery>,
    /// The account's halt (`refreshHalt(nil)`); none read, running.
    account_halt: Option<tafs_core::halt::Halt>,
    /// What this person may do with each taf, by KIFF agent (`GET /v1/me/access`).
    access: std::collections::BTreeMap<String, tafs_core::access::Access>,
    /// Who each taf this person manages is shared with, in the Overview's words, by KIFF agent.
    used_by: std::collections::BTreeMap<String, String>,
    /// Each tafs cloud taf's last run, by its id.
    reports: std::collections::BTreeMap<String, tafs_core::access::RunReport>,
    /// The balance, read with each refresh for the sidebar's plan line (`refreshBalance`).
    balance: Option<tafs_core::usage::Balance>,
    /// "Your brief" (`GET /v1/brief`), read with the list: none (a 404, an error, nothing in
    /// it) keeps Needs you as today.
    brief: Option<tafs_core::brief::Brief>,
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
    /// With `--home`: tafs cloud's recipes answer from a file, for Home with no taf.
    recipes: Option<String>,
    /// With `--home`: tafs cloud's gallery answer from a file, for the Explore row and the strip.
    gallery: Option<String>,
    /// With `--home`: a halt as tafs cloud answers it, for the halt screens' screenshots.
    halt: Option<String>,
    /// With `--home`: a Record search page (and a case) as tafs cloud answers them.
    record_search: Option<String>,
    /// With `--home`: what this person may do with each taf, as tafs cloud answers
    /// `GET /v1/me/access`.
    access: Option<String>,
    /// With `--home`: a receipts export as tafs cloud answers it, never saved.
    receipts: Option<String>,
    /// With `--home`: KIFF's activity, `{"calls": […]}`, for the Record.
    activity: Option<String>,
    /// With `--activity`: one taf's call refs as tafs cloud answers `GET /v1/tafs/{id}/calls`,
    /// for every taf's lines.
    calls: Option<String>,
    /// With `--home`: a receipt as KIFF answers `GET /v1/receipts/{trace_id}`, never sent.
    receipt: Option<String>,
    /// With `--home`: a taf's work as tafs cloud answers `GET /v1/tafs/{id}/work`, never sent.
    work: Option<String>,
    /// With `--home`: a taf's doors as tafs cloud answers `GET /v1/tafs/{id}/doors`; its knocks'
    /// answers send nothing.
    doors: Option<String>,
    /// With `--home`: Account's reads, `{"suggestions": …, "consent": …}`, each as tafs cloud
    /// answers it; its changes send nothing.
    account_reads: Option<String>,
    /// With `--home`: the KIFF account as `GET /v1/me` answers it, for screens drawn by who may
    /// do what.
    me: Option<String>,
    /// With `--home`: a taf's notes as tafs cloud answers `GET /v1/tafs/{id}/memory`, for See all.
    memory: Option<String>,
    /// With `--home`: a page of a taf's runs as tafs cloud answers `GET /v1/tafs/{id}/work/runs`.
    runs: Option<String>,
    /// With `--home`: the balance as tafs cloud answers `GET /v1/usage/balance`.
    balance: Option<String>,
    /// With `--runs`: a run's own page as tafs cloud answers `GET /v1/tafs/{id}/requests/{run}`.
    request: Option<String>,
    /// With `--home`: Coach's conversation as tafs cloud answers
    /// `GET /v1/tafs/{id}/conversation/{session}`, read from a file: nothing is sent.
    coach: Option<String>,
    /// With `--coach`: Coach's chats as tafs cloud answers them, opening Previous conversations.
    coach_chats: Option<String>,
    /// With `--home`: a taf's chat as tafs cloud answers `GET /v1/tafs/{id}/conversation`, read
    /// from a file: nothing is sent.
    chat: Option<String>,
    /// With `--chat`: Report this answer open over its last answer that can be reported.
    report: bool,
    /// With `--home`: "Your brief" as tafs cloud answers `GET /v1/brief`; every read is it, and
    /// Later sends nothing.
    brief: Option<String>,
    /// With `--brief`: a mission's log as tafs cloud answers `GET /v1/missions/{id}/log`, for
    /// every mission's Evidence.
    mission_log: Option<String>,
    /// `--size WxH`: the window's size, for a screenshot of a narrower layout.
    size: Option<(i32, i32)>,
    screenshot: Option<String>,
}

const USAGE: &str = "tafs [--lang en|de|fr|es|it|pt|nl|pl] [--theme light|dark] [--sample] [--signed-in] [--view-only [--paired]]
     [--home contract.json] [--usage answers.json] [--people answers.json] [--unproven answers.json] [--recipes answers.json] [--gallery answers.json] [--halt answer.json] [--record-search answers.json] [--access answer.json] [--receipts answer.json] [--activity answer.json] [--calls answer.json] [--receipt answer.json] [--work answer.json] [--doors answer.json] [--account-reads answers.json] [--me answer.json] [--memory answer.json] [--runs answer.json] [--request answer.json] [--balance answer.json] [--coach answer.json] [--coach-chats answer.json] [--chat answer.json [--report]] [--brief answer.json] [--mission-log answer.json] [--size WxH] [--page welcome|sign-in|pair|recipe:<id>|gallery|halt-all|halt:<id>|record-search:<q>|record-export|receipt:<proposal id>|run:<taf id>|pin-sheet|coach|overview|overview-label:<id>|needs-you|brief-answer:<decision id>|mission-log:<mission id>|record|calendar|calendar-week|usage|connect-apps|account|kiff-account|time-zone|time-zone-use|alerts|devices|devices-sign-out|people|people-rename|people-add|taf|taf-card|taf-setup|consent|connected|sign-in-code] [--screenshot out.png]";|     [--home contract.json] [--usage answers.json] [--people answers.json] [--unproven answers.json] [--recipes answers.json] [--gallery answers.json] [--halt answer.json] [--record-search answers.json] [--access answer.json] [--receipts answer.json] [--activity answer.json] [--calls answer.json] [--receipt answer.json] [--work answer.json] [--doors answer.json] [--account-reads answers.json] [--me answer.json] [--memory answer.json] [--runs answer.json] [--request answer.json] [--balance answer.json] [--coach answer.json] [--coach-chats answer.json] [--chat answer.json [--report]] [--size WxH] [--page welcome

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
            "--recipes" => o.recipes = Some(value()?),
            "--gallery" => o.gallery = Some(value()?),
            "--halt" => o.halt = Some(value()?),
            "--record-search" => o.record_search = Some(value()?),
            "--access" => o.access = Some(value()?),
            "--receipts" => o.receipts = Some(value()?),
            "--activity" => o.activity = Some(value()?),
            "--calls" => o.calls = Some(value()?),
            "--receipt" => o.receipt = Some(value()?),
            "--work" => o.work = Some(value()?),
            "--doors" => o.doors = Some(value()?),
            "--account-reads" => o.account_reads = Some(value()?),
            "--me" => o.me = Some(value()?),
            "--memory" => o.memory = Some(value()?),
            "--runs" => o.runs = Some(value()?),
            "--balance" => o.balance = Some(value()?),
            "--request" => o.request = Some(value()?),
            "--coach" => o.coach = Some(value()?),
            "--coach-chats" => o.coach_chats = Some(value()?),
            "--chat" => o.chat = Some(value()?),
            "--report" => o.report = true,
            "--brief" => o.brief = Some(value()?),
            "--mission-log" => o.mission_log = Some(value()?),
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
        Some(other) if other.starts_with("brief-answer:") => Selection::NeedsYou,
        Some(other) if other.starts_with("mission-log:") => Selection::NeedsYou,
        Some("record") => Selection::Record,
        Some("calendar") | Some("calendar-week") => Selection::Calendar,
        Some("usage") => Selection::Usage,
        Some("connect-apps") => Selection::ConnectApps,
        Some("account") => Selection::Account,
        Some("devices") | Some("devices-sign-out") => Selection::Devices,
        Some("people") | Some("people-rename") | Some("people-add") => Selection::People,
        Some("kiff-account") | Some("delete-account") => Selection::KiffAccount,
        Some("time-zone") | Some("time-zone-use") => Selection::TimeZone,
        Some("alerts") => Selection::Alerts,
        Some("language") => Selection::Language,
        Some("coach") => Selection::Coach,
        // The sample's Refunds, as the reference set's taf.png opens it.
        Some("taf") | Some("taf-card") => Selection::Taf("refunds".into()),
        // With `--home`, the contract's first taf.
        Some(other) if other.starts_with("taf:") => Selection::Taf(other["taf:".len()..].into()),
        Some(other) if other.starts_with("run:") => Selection::Taf(other["run:".len()..].into()),
        Some("halt-all") => Selection::HaltAll,
        Some(other) if other.starts_with("record-search:") => Selection::Record,
        Some("record-export") => Selection::Record,
        Some(other) if other.starts_with("receipt:") => Selection::Record,
        Some(other) if other.starts_with("halt:") => Selection::Halt(other["halt:".len()..].into()),
        Some(other) if other.starts_with("taf-card:") => {
            Selection::Taf(other["taf-card:".len()..].into())
        }
        Some(other) if other.starts_with("taf-setup:") => {
            Selection::Taf(other["taf-setup:".len()..].into())
        }
        _ => Selection::Overview,
    }
}

thread_local! {
    /// A language was just picked in Account › Language: the window drawn again in it opens on
    /// Account, where the person was, as the iPad's picker goes back to it.
    static REDRAWN_FOR_LANGUAGE: Cell<bool> = const { Cell::new(false) };
}

fn activate(app: &adw::Application, o: &Options) {
    let preferred = language::preferred_from_env();
    let redrawn = REDRAWN_FOR_LANGUAGE.with(|r| r.replace(false));
    // `--lang`, else the language picked in Account (a screenshot never reads it), else this
    // computer's.
    let lang: &'static str = match &o.lang {
        Some(l) => SHIPPED
            .iter()
            .find(|s| **s == l.as_str())
            .copied()
            .unwrap_or("en"),
        None => o
            .screenshot
            .is_none()
            .then(store::language)
            .flatten()
            .unwrap_or_else(|| language::app_language(&preferred)),
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
        first: RefCell::new(if redrawn {
            Selection::Account
        } else {
            selection_of(o.page.as_deref())
        }),
        kept: RefCell::new(None),
        sign_out_error: RefCell::new(None),
        cloud: RefCell::new(None),
        held_seen: RefCell::new(None),
        held_watch: Cell::new(0),
        problem: RefCell::new(None),
        home_file: o.home.as_ref().map(std::path::PathBuf::from),
        work_file: o.work.as_ref().map(std::path::PathBuf::from),
        doors_file: o.doors.as_ref().map(std::path::PathBuf::from),
        account_reads_file: o.account_reads.as_ref().map(std::path::PathBuf::from),
        memory_file: o.memory.as_ref().map(std::path::PathBuf::from),
        runs_file: o.runs.as_ref().map(std::path::PathBuf::from),
        request_file: o.request.as_ref().map(std::path::PathBuf::from),
        coach_file: o.coach.as_ref().map(std::path::PathBuf::from),
        coach_chats: o.coach_chats.as_ref().map(std::path::PathBuf::from),
        chat_file: o.chat.as_ref().map(std::path::PathBuf::from),
        open_report: o.report,
        brief_file: o.brief.as_ref().map(std::path::PathBuf::from),
        mission_log_file: o.mission_log.as_ref().map(std::path::PathBuf::from),
        open_run: o.page.as_deref().is_some_and(|p| p.starts_with("run:")),
        usage_file: o.usage.as_ref().map(std::path::PathBuf::from),
        people_file: o.people.as_ref().map(std::path::PathBuf::from),
        halt_file: halt_file(o.halt.as_deref()),
        record_search_file: record_search_file(o.record_search.as_deref()),
        receipts_file: o.receipts.as_deref().map(|p| match std::fs::read(p) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("{p} can't be read");
                std::process::exit(2);
            }
        }),
        receipts_open: o.page.as_deref() == Some("record-export"),
        receipt_file: o.receipt.as_deref().map(|p| match std::fs::read(p) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("{p} can't be read");
                std::process::exit(2);
            }
        }),
        receipt_open: o
            .page
            .as_deref()
            .and_then(|p| p.strip_prefix("receipt:"))
            .map(tafs_core::record::record_id),
        record_query: o
            .page
            .as_deref()
            .and_then(|p| p.strip_prefix("record-search:"))
            .map(str::to_string),
        calendar_day: o.page.as_deref() == Some("calendar"),
        screenshot: o.screenshot.is_some(),
        on_card: o
            .page
            .as_deref()
            .is_some_and(|p| p == "taf-card" || p.starts_with("taf-card:")),
        on_setup: o
            .page
            .as_deref()
            .is_some_and(|p| p.starts_with("taf-setup:")),
    });
    if let Some(id) = o
        .page
        .as_deref()
        .and_then(|p| p.strip_prefix("overview-label:"))
    {
        overview::CHOSEN.with(|c| *c.borrow_mut() = Some(id.to_string()));
    }
    if let Some(id) = o
        .page
        .as_deref()
        .and_then(|p| p.strip_prefix("mission-log:"))
    {
        brief::LOG.with(|l| *l.borrow_mut() = Some(id.to_string()));
    }
    if let Some(id) = o
        .page
        .as_deref()
        .and_then(|p| p.strip_prefix("brief-answer:"))
    {
        brief::ANSWERED.with(|a| *a.borrow_mut() = Some(id.to_string()));
    }
    if o.page.as_deref() == Some("delete-account") {
        kiff_account::OPEN_DELETE.with(|d| d.set(true));
    }
    match o.page.as_deref() {
        Some("people-rename") => people::OPEN.with(|p| p.set(Some("rename"))),
        Some("people-add") => people::OPEN.with(|p| p.set(Some("add"))),
        Some("devices-sign-out") => devices::OPEN_SIGN_OUT.with(|d| d.set(true)),
        Some("time-zone-use") => zones::TAP_USE_HERE.with(|t| t.set(true)),
        _ => {}
    }
    // A hold's notification, clicked (`app.open-hold`): Needs you, read again. A screenshot
    // sends none.
    if !screenshot {
        let action = gio::SimpleAction::new("open-hold", Some(glib::VariantTy::STRING));
        let me = Rc::downgrade(&app_state);
        action.connect_activate(move |_, _| {
            if let Some(me) = me.upgrade() {
                me.open_hold();
            }
        });
        app.add_action(&action);
    }

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
            account: o.me.as_deref().map(|path| {
                std::fs::read(path)
                    .ok()
                    .and_then(|body| {
                        Account::read(&tafs_core::signin::Answer { status: 200, body }).ok()
                    })
                    .unwrap_or_else(|| {
                        eprintln!("{path} isn't KIFF's account as GET /v1/me answers it");
                        std::process::exit(2);
                    })
            }),
            activity: activity_file(o.activity.as_deref()),
            call_refs: calls_file(o.calls.as_deref()),
            unproven: unproven_file(o.unproven.as_deref()),
            recipes: recipes_file(o.recipes.as_deref()),
            gallery: gallery_file(o.gallery.as_deref()),
            account_halt: o.halt.as_deref().map(|p| halt_file(Some(p)).unwrap()),
            access: access_file(o.access.as_deref()),
            used_by: Default::default(),
            reports: Default::default(),
            balance: balance_file(o.balance.as_deref()),
            brief: o.brief.as_deref().map(|path| {
                std::fs::read(path)
                    .ok()
                    .and_then(|body| brief::read_answer(&body, &app_state.words))
                    .unwrap_or_else(|| {
                        eprintln!("{path} isn't a brief as tafs cloud answers GET /v1/brief");
                        std::process::exit(2);
                    })
            }),
        }),
        None => {
            eprintln!("{path} isn't Home as tafs cloud answers it");
            std::process::exit(2);
        }
    });
    match (o.page.as_deref(), kept) {
        _ if o.sample => app_state.open_shell(Mode::Sample, false, None),
        _ if from_file.is_some() => {
            app_state.open_shell(Mode::Kiff, o.view_only, from_file.clone());
            // `--page connected`: the celebration after signing in, over the shell.
            if o.page.as_deref() == Some("connected") {
                connected::present(&app_state.window, &app_state.words);
            }
            // `--page consent`: the AI-data consent's screen, as after sign-in, from `--account-reads`.
            if o.page.as_deref() == Some("consent") {
                if let Some(file) = &app_state.account_reads_file {
                    ai_consent::ask_after_sign_in(
                        app_state.words.clone(),
                        ai_consent::Source::File(file.clone()),
                        app_state.window.clone().upcast(),
                    );
                }
            }
            // `--page gallery`: the jobs' gallery, browsing, for its screenshot.
            if o.page.as_deref() == Some("gallery") {
                if let Some(g) = from_file.as_deref().and_then(|c| c.gallery.clone()) {
                    let pane = recipes::GalleryPane {
                        labels: from_file
                            .as_deref()
                            .and_then(|c| c.recipes.as_ref())
                            .and_then(|r| r.labels.clone()),
                        gallery: g,
                        may_use_coach: true,
                    };
                    recipes::gallery(&pane, &window);
                }
            }
            // `--page recipe:<id>`: Home's recipe opens its sheet, for its screenshot.
            if let (Some(id), Some(c)) = (
                o.page.as_deref().and_then(|p| p.strip_prefix("recipe:")),
                from_file.as_deref(),
            ) {
                if let Some(r) = c.recipes.as_ref() {
                    if let Some(recipe) = r.recipes.iter().find(|x| x.id == id) {
                        let pane = recipes::RecipesPane {
                            recipes: r.clone(),
                            may_use_coach: true,
                            hand_words: None,
                            sample_label: None,
                        };
                        recipes::sheet(&pane, recipe, &window);
                    }
                }
            }
        }
        _ if o.view_only => app_state.open_shell(Mode::Kiff, true, None),
        (Some("welcome") | Some("sign-in") | Some("sign-in-code") | Some("pair"), _)
        | (_, None) => {
            app_state.show_welcome();
            // `--page sign-in-code`: Continue already tapped, the code on screen.
            if o.page.as_deref() == Some("sign-in-code") {
                sign_in::CONTINUED.with(|c| c.set(true));
            }
            if matches!(o.page.as_deref(), Some("sign-in") | Some("sign-in-code")) {
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
    // `--page pin-sheet`: the security key's PIN sheet, for its screenshot. Its answers are
    // canned (five tries left; no key plugged in): no key is asked, nothing is signed.
    if o.page.as_deref() == Some("pin-sheet") {
        use tafs_core::security_key::Failure;
        pin_sheet::present(
            app_state.words.clone(),
            &window,
            std::sync::Arc::new(|_| Err(fido_key::KeyError::Key(Failure::NoKey))),
            std::sync::Arc::new(|| Ok(5)),
            std::sync::Arc::new(|| {}),
            Rc::new(|_| {}),
        );
    }
    if let Some(path) = o.screenshot.clone() {
        // Saved 1.5 s after the window shows, once it's drawn. On a busy machine the first frame
        // can come later, with no renderer or nothing drawn yet: tried again every half second,
        // for up to 10 s more, before it gives up.
        let (window, app) = (window.clone(), app.clone());
        let tries = std::cell::Cell::new(0);
        glib::timeout_add_local(std::time::Duration::from_millis(500), move || {
            tries.set(tries.get() + 1);
            if tries.get() < 3 {
                return glib::ControlFlow::Continue;
            }
            match save_screenshot(window.upcast_ref(), &path) {
                Ok(()) => println!("saved {path}"),
                Err(e) if tries.get() < 23 => {
                    eprintln!("screenshot: {e}; trying again");
                    return glib::ControlFlow::Continue;
                }
                Err(e) => {
                    eprintln!("screenshot: {e}");
                    std::process::exit(1);
                }
            }
            app.quit();
            glib::ControlFlow::Break
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
    /// The held calls already known, by KIFF's id: notified, or waiting when the account was
    /// read. None until the account is read, and after signing out.
    held_seen: RefCell<Option<std::collections::BTreeSet<String>>>,
    /// The minute's read of held calls in force: a new account read, or signing out, ends the
    /// last one.
    held_watch: Cell<u64>,
    /// The welcome's line for why it's there, while it's open.
    problem: RefCell<Option<gtk::Label>>,
    /// `--home`'s contract file, for the screenshots' tafs cloud reads.
    home_file: Option<std::path::PathBuf>,
    /// `--work`'s answer: the screenshots' work, never sent.
    work_file: Option<std::path::PathBuf>,
    /// `--doors`: a taf's doors as tafs cloud answers them, for its knocks' screenshot.
    doors_file: Option<std::path::PathBuf>,
    /// `--account-reads`: Account's reads for its screenshots.
    account_reads_file: Option<std::path::PathBuf>,
    /// `--memory`: a taf's notes for its Setup screenshots.
    memory_file: Option<std::path::PathBuf>,
    /// `--runs`'s page: the screenshots' runs, never sent.
    runs_file: Option<std::path::PathBuf>,
    /// `--request`'s page: the screenshots' run, never sent.
    request_file: Option<std::path::PathBuf>,
    /// `--coach`'s answer: the screenshots' Coach conversation, never sent.
    coach_file: Option<std::path::PathBuf>,
    /// `--coach-chats`: Previous conversations open over it, with these chats.
    coach_chats: Option<std::path::PathBuf>,
    /// `--chat`: a taf's chat from a file, never sent.
    chat_file: Option<std::path::PathBuf>,
    /// `--report`: Report this answer open over the chat.
    open_report: bool,
    /// `--brief`'s answer: the screenshots' brief, every read of it, never sent.
    brief_file: Option<std::path::PathBuf>,
    /// `--mission-log`'s answer: every mission's log, never sent.
    mission_log_file: Option<std::path::PathBuf>,
    /// `--page run:<taf id>`: that taf's first kept run open, for a screenshot.
    open_run: bool,
    /// `--usage`'s file of synthetic answers.
    usage_file: Option<std::path::PathBuf>,
    /// `--halt`'s answer: the screenshots' halt, never sent to tafs cloud.
    halt_file: Option<tafs_core::halt::Halt>,
    /// `--page record-search:<q>`: what's typed in the Record's box, for a screenshot.
    record_query: Option<String>,
    /// `--record-search`'s answers: the screenshots' search page and case, never sent.
    record_search_file: Option<(
        tafs_core::record_search::SearchPage,
        Option<tafs_core::record_search::RecordCase>,
    )>,
    /// `--receipts`'s answer: the screenshots' export, never sent or saved.
    receipts_file: Option<Vec<u8>>,
    /// `--page record-export`: the export's days open, for a screenshot.
    receipts_open: bool,
    /// `--receipt`'s answer: the screenshots' receipt, never sent.
    receipt_file: Option<Vec<u8>>,
    /// `--page receipt:<proposal id>`: that line's receipt open, for a screenshot.
    receipt_open: Option<String>,
    /// `--people`'s file of synthetic answers.
    people_file: Option<std::path::PathBuf>,
    /// A screenshot: the sample's Refunds has the reference set's three Cards.
    screenshot: bool,
    /// `--page taf-card`: the taf opens on its Card tab.
    on_card: bool,
    /// `--page taf-setup:<id>`: a taf's page opens on Setup.
    on_setup: bool,
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
        // Signed out: the consent's screen, if it's open, goes with the account.
        ai_consent::forget();
        // Signed out: nothing written to Coach for the last account stays.
        coach::forget();
        // And nothing written to a taf.
        taf_chat::forget();
        // Signed out: no account, so this computer's zone.
        tafs_core::account_time::use_zone(None);
        // Signed out: the minute's read ends, and the last account's notifications never stay.
        self.held_watch.set(self.held_watch.get() + 1);
        if let (Some(seen), Some(app)) = (self.held_seen.take(), self.window.application()) {
            for hold in seen {
                app.withdraw_notification(&tafs_core::hold_alert::tag(&hold));
            }
        }
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
        let done: Rc<dyn Fn(KeptSession)> = Rc::new(move |kept| me.connected(kept));
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
        let done: Rc<dyn Fn(KeptSession)> = Rc::new(move |kept| me.connected(kept));
        sign_in::present(
            &self.window,
            self.window.upcast_ref(),
            self.words.clone(),
            self.languages.clone(),
            done,
            preview,
        );
    }

    /// Signed in just now, by KIFF or by pairing: the account opens, and the celebration shows
    /// over it, once (`justConnected`, `ConnectedView`).
    fn connected(self: &Rc<Self>, kept: KeptSession) {
        self.open_account(kept);
        connected::present(&self.window, &self.words);
    }

    /// Signed in: the shell at once, then the account (`GET /v1/me`), Home and the held calls,
    /// drawn again once read. An answer that can't be read leaves it as it was.
    fn open_account(self: &Rc<Self>, kept: KeptSession) {
        // A new account's consent is its own: Not now from the last one doesn't carry over.
        ai_consent::forget();
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
                let call_refs = call_refs(&token, &languages, &home, &activity);
                let (access, used_by, reports) = taf_reads(&words, &token, &languages, &home);
                // `loadRecipes`: read with the rest; one that can't be read lists none.
                let recipes = kiff::recipes(&token, &languages)
                    .ok()
                    .filter(|a| a.status == 200)
                    .and_then(|a| tafs_core::recipes::Recipes::read(&a.body));
                // The account's halt, for the sidebar's Halt all: none read, it reads running.
                let account_halt = kiff::halt(&token, &languages, None, false)
                    .ok()
                    .filter(|a| a.status == 200)
                    .and_then(|a| tafs_core::halt::read(&a.body));
                // `loadGallery`: a read that fails leaves none.
                let gallery = kiff::gallery(&token, &languages)
                    .ok()
                    .filter(|a| a.status == 200)
                    .and_then(|a| tafs_core::recipes::Gallery::read(&a.body));
                // The balance alone, for the plan's line: quietly, a failure shows none.
                let balance = kiff::cloud(&token, &languages, tafs_core::usage::BALANCE_PATH)
                    .ok()
                    .filter(|a| a.status == 200)
                    .and_then(|a| tafs_core::usage::Balance::read(&a.body));
                // Your brief, read with the list: tafs cloud answering `/v1/brief` is what turns
                // it on (a 404, an error or a brief with nothing in it keeps Needs you as today).
                let brief = kiff::cloud(&token, &languages, &tafs_core::brief::Brief::path(None))
                    .ok()
                    .filter(|a| a.status == 200)
                    .and_then(|a| brief::read_answer(&a.body, &words));
                Read::Cloud(Box::new(Cloud {
                    brief,
                    balance,
                    home,
                    held,
                    account,
                    activity,
                    call_refs,
                    unproven,
                    recipes,
                    gallery,
                    account_halt,
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
                    let cloud = Rc::new(*cloud);
                    me.open_shell(Mode::Kiff, view_only, Some(cloud.clone()));
                    // `loadAIConsent`: not given, its screen, unless Not now was answered.
                    if let Some(kept) = me.kept.borrow().as_ref() {
                        ai_consent::ask_after_sign_in(
                            me.words.clone(),
                            ai_consent::Source::Cloud {
                                token: kept.token.clone(),
                                languages: me.languages.clone(),
                            },
                            me.window.clone().upcast(),
                        );
                    }
                    me.watch_holds(&cloud);
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
        signed_out: Rc<dyn Fn()>,
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
        // Where a tafs cloud taf's work is read: tafs cloud as the session; a screenshot's from
        // `--work`, never sent.
        let work = match (self.kept.borrow().as_ref(), &self.work_file) {
            (_, Some(file)) => work_panel::WorkSource::File(file.clone()),
            (Some(kept), None) => work_panel::WorkSource::Cloud {
                token: kept.token.clone(),
                languages: self.languages.clone(),
            },
            _ => work_panel::WorkSource::None,
        };
        // Where its Setup is read and a note forgotten: tafs cloud as the session; a screenshot's
        // from the contract beside `--home`, and `--memory`'s notes.
        let setup = match (self.kept.borrow().as_ref(), &self.home_file) {
            (Some(kept), _) => setup_pane::SetupSource::Cloud {
                token: kept.token.clone(),
                languages: self.languages.clone(),
            },
            (None, Some(home)) => setup_pane::SetupSource::File {
                setup: home.with_file_name("taf-setup.json"),
                notes: self.memory_file.clone(),
            },
            _ => setup_pane::SetupSource::None,
        };
        // Where its doors are read and its knocks answered: tafs cloud as the session; a
        // screenshot's from `--doors`, and its answers send nothing.
        let doors = match (self.kept.borrow().as_ref(), &self.doors_file) {
            (_, Some(file)) => knocks::DoorsSource::File(file.clone()),
            (Some(kept), None) => knocks::DoorsSource::Cloud {
                token: kept.token.clone(),
                languages: self.languages.clone(),
            },
            _ => knocks::DoorsSource::None,
        };
        // Where its runs are read and Run now sent: tafs cloud as the session; a screenshot's
        // first page from `--runs`, and its Run now sends nothing.
        let runs = match (self.kept.borrow().as_ref(), &self.runs_file) {
            (_, Some(file)) => runs_panel::RunsSource::File {
                page: file.clone(),
                request: self.request_file.clone(),
                open: self.open_run,
            },
            (Some(kept), None) => runs_panel::RunsSource::Cloud {
                token: kept.token.clone(),
                languages: self.languages.clone(),
            },
            _ => runs_panel::RunsSource::None,
        };
        // Where its chat is read and what's written goes: tafs cloud as the session, signed in with
        // KIFF; a screenshot's from `--chat`, never sent.
        let chat = match (mode, self.kept.borrow().as_ref(), &self.chat_file) {
            (Mode::Kiff, _, Some(file)) => taf_chat::ChatSource::File {
                thread: file.clone(),
                report: self.open_report,
            },
            (Mode::Kiff, Some(kept), None) => taf_chat::ChatSource::Cloud {
                token: kept.token.clone(),
                languages: self.languages.clone(),
                signed_out,
            },
            _ => taf_chat::ChatSource::None,
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
                        opens_on_setup: self.on_setup,
                        cloud_id: None,
                        source: taf::ActivitySource::None,
                        work: work_panel::WorkSource::None,
                        job: String::new(),
                        setup: setup_pane::SetupSource::None,
                        doors: knocks::DoorsSource::None,
                        card_units: vec![],
                        may_use_coach: false,
                        health: None,
                        runs: runs_panel::RunsSource::None,
                        runs_line: None,
                        time_zone_words: None,
                        schedule_time_zone_words: None,
                        chat: taf_chat::ChatSource::None,
                    }
                })
                .collect(),
            // Its tafs, and the agents not run by tafs (`otherAgents`), whose rows open a page too.
            (_, Some(c)) => c
                .home
                .tafs
                .iter()
                .chain(c.home.other_agents())
                .filter_map(|h| {
                    let cards = tafs_core::home::full_cards(w, h, w.language()).ok()?;
                    let health = tafs_core::home::Health::read(&h.health);
                    let line = tafs_core::runs::next_line(w, h).ok();
                    // Its job in words, as tafs cloud says it; else described by its Cards
                    // (`KIFFMapping.taf`).
                    let job = h.job_summary.clone().unwrap_or_else(|| {
                        w.count("taf.works_under_cards", h.card.ids.len() as i64, &[])
                            .unwrap_or_default()
                    });
                    let zones = (
                        h.time_zone_words.clone(),
                        h.schedule_time_zone_words.clone(),
                        job,
                    );
                    Some((
                        tafs_core::home::taf(w, h).ok()?,
                        cards,
                        (health, line, zones),
                    ))
                })
                .map(|(t, cards, extra)| {
                    (
                        tafs_core::access::manages(c.access.get(&t.agent)),
                        t,
                        cards,
                        extra,
                    )
                })
                .map(
                    |(manages, t, cards, (health, runs_line, zones))| taf::TafPage {
                        time_zone_words: zones.0,
                        schedule_time_zone_words: zones.1,
                        job: zones.2,
                        setup: setup.clone(),
                        work: work.clone(),
                        // Its knocks, where tafs cloud offers doors (`doorsOffered`); a
                        // screenshot's `--doors` stands for the offer.
                        doors: if self.doors_file.is_some()
                            || c.home.doors_offered.as_ref().is_some_and(|d| !d.is_empty())
                        {
                            doors.clone()
                        } else {
                            knocks::DoorsSource::None
                        },
                        // `model.cardUnitExceptions.filter { $0.taf == cloud id }`.
                        card_units: c
                            .home
                            .card_unit_exceptions
                            .iter()
                            .flatten()
                            .filter(|a| t.cloud && a.taf == t.key)
                            .cloned()
                            .collect(),
                        may_use_coach: model.may_use_coach,
                        health,
                        runs: runs.clone(),
                        runs_line,
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
                        opens_on_setup: self.on_setup,
                        source: source.clone(),
                        chat: chat.clone(),
                    },
                )
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
                tafs_core::record::naming(
                    tafs_core::record::record(&self.words, &c.activity, &taf_of)
                        .unwrap_or_default(),
                    &c.call_refs,
                )
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
        // Signed in: the search across the person's tafs, at tafs cloud as the session; a
        // screenshot's answers from `--record-search`, never sent.
        let on_home: Vec<String> = cloud
            .map(|c| c.home.tafs.iter().filter_map(|t| t.id.clone()).collect())
            .unwrap_or_default();
        let search = match (mode, self.kept.borrow().as_ref(), &self.record_search_file) {
            (Mode::Kiff, _, Some(canned)) => Some(Rc::new(record_search::SearchSource {
                token: String::new(),
                languages: vec![],
                canned: Some(canned.clone()),
                initial: self.record_query.clone(),
                on_home: on_home.clone(),
            })),
            (Mode::Kiff, Some(kept), None) => Some(Rc::new(record_search::SearchSource {
                token: kept.token.clone(),
                languages: self.languages.clone(),
                canned: None,
                initial: None,
                on_home,
            })),
            _ => None,
        };
        // Signed in, for whoever sees every taf's Record (`seesEveryTaf`): the receipts
        // export; a screenshot's from `--receipts`, never sent or saved.
        let sees_every =
            cloud.is_some_and(|c| tafs_core::receipts::sees_every_taf(&c.home.tafs, &c.access));
        let receipts = match (
            mode,
            sees_every,
            self.kept.borrow().as_ref(),
            &self.receipts_file,
        ) {
            (Mode::Kiff, true, _, Some(canned)) => Some(Rc::new(receipts::ReceiptsSource {
                token: String::new(),
                languages: vec![],
                canned: Some(canned.clone()),
                open: self.receipts_open,
            })),
            (Mode::Kiff, true, Some(kept), None) => Some(Rc::new(receipts::ReceiptsSource {
                token: kept.token.clone(),
                languages: self.languages.clone(),
                canned: None,
                open: false,
            })),
            _ => None,
        };
        // Signed in with KIFF: a line's receipt, read as the session; a screenshot's from
        // `--receipt`, never sent.
        let receipt = match (mode, self.kept.borrow().as_ref(), &self.receipt_file) {
            (Mode::Kiff, _, Some(canned)) => Some(Rc::new(receipt_sheet::ReceiptSource {
                token: String::new(),
                languages: vec![],
                canned: Some(canned.clone()),
                open: self.receipt_open.clone(),
            })),
            (Mode::Kiff, Some(kept), None) => Some(Rc::new(receipt_sheet::ReceiptSource {
                token: kept.token.clone(),
                languages: self.languages.clone(),
                canned: None,
                open: None,
            })),
            _ => None,
        };
        record::RecordData {
            entries,
            tafs,
            waiting: Rc::new(move |e| !kiff || still_held.contains(&e.id)),
            search,
            receipts,
            receipt,
        }
    }

    /// While tafs runs, KIFF's held calls read once a minute, as Windows does (a computer gets no
    /// push): each call held since the last read is notified (`HoldNotification`), and a call no
    /// longer held loses its notification (`removeAlerts(forHold:)`). What waited when the account
    /// was read isn't announced: it's in Needs you. Only for someone who answers held calls
    /// (`canAnswerHolds`). A 401 signs this computer out, as any read.
    fn watch_holds(self: &Rc<Self>, cloud: &Cloud) {
        use tafs_core::hold_alert;
        let watch = self.held_watch.get() + 1;
        self.held_watch.set(watch);
        // Notifications already shown for holds of this account stay theirs.
        let mut seen = self.held_seen.take().unwrap_or_default();
        seen.extend(cloud.held.iter().map(|h| h.id.clone()));
        *self.held_seen.borrow_mut() = Some(seen);
        let notifies = hold_alert::notifies(
            cloud
                .account
                .as_ref()
                .and_then(|a| a.capabilities.as_deref()),
        );
        let Some(token) = self.kept.borrow().as_ref().map(|k| k.token.clone()) else {
            return;
        };
        // Names for the alerts: `others` whole, as it repeats the tafs being set up.
        let tafs: Vec<(String, String)> = cloud
            .home
            .tafs
            .iter()
            .chain(&cloud.home.others)
            .map(|t| (t.kiff_agent.clone(), t.name.clone()))
            .collect();
        let (me, languages) = (self.clone(), self.languages.clone());
        glib::spawn_future_local(async move {
            loop {
                glib::timeout_future(hold_alert::PERIOD).await;
                if me.held_watch.get() != watch {
                    return;
                }
                let (token, languages) = (token.clone(), languages.clone());
                let read = gio::spawn_blocking(move || kiff::held(&token, &languages).ok())
                    .await
                    .ok()
                    .flatten();
                if me.held_watch.get() != watch {
                    return;
                }
                let Some(answer) = read else { continue };
                if answer.status == 401 {
                    me.signed_out_by_kiff();
                    return;
                }
                // A read that fails changes nothing: the next minute reads again.
                let Some(holds) = (answer.status == 200)
                    .then(|| tafs_core::held::read_held(&answer.body))
                    .flatten()
                else {
                    continue;
                };
                let Some(app) = me.window.application() else {
                    continue;
                };
                let seen = me.held_seen.borrow().clone();
                for hold in hold_alert::gone(seen.as_ref(), &holds) {
                    app.withdraw_notification(&hold_alert::tag(&hold));
                }
                // Account › Alerts: off, nothing new is announced; on, with its sound or not.
                let announce =
                    tafs_core::alerts::announce(store::alerts_on(), store::alert_sound());
                if let (true, Some(sound)) = (notifies, announce) {
                    let mut rang = false;
                    for hold in hold_alert::fresh(seen.as_ref(), &holds) {
                        let named = hold_alert::named(
                            &me.words,
                            hold,
                            tafs.iter().map(|(a, n)| (a.as_str(), n.as_str())),
                        )
                        .ok()
                        .flatten();
                        let Ok(lines) = hold_alert::lines(&me.words, hold, named.as_deref()) else {
                            continue;
                        };
                        let n = gio::Notification::new(&lines.title);
                        n.set_body(Some(&lines.body));
                        n.set_default_action_and_target_value(
                            "app.open-hold",
                            Some(&lines.hold.to_variant()),
                        );
                        app.send_notification(Some(&hold_alert::tag(&lines.hold)), &n);
                        // A notification here has no sound of its own: the desktop's alert
                        // sound, once a read.
                        if sound && !rang {
                            if let Some(d) = gtk::gdk::Display::default() {
                                d.beep();
                            }
                            rang = true;
                        }
                    }
                }
                *me.held_seen.borrow_mut() = Some(holds.iter().map(|h| h.id.clone()).collect());
            }
        });
    }

    /// A hold's notification, clicked: the account read again, opening on Needs you
    /// (`PushDelegate.open`).
    fn open_hold(self: &Rc<Self>) {
        self.window.present();
        let Some(kept) = self.kept.borrow().clone() else {
            return;
        };
        *self.first.borrow_mut() = Selection::NeedsYou;
        self.open_account(kept);
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
                let (model, mut tafs) = home_model(
                    words,
                    &c.home,
                    &c.held,
                    &c.activity,
                    &c.access,
                    c.balance.as_ref(),
                    at,
                )
                .unwrap_or_else(|e| fail(e));
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
        // tafs cloud's line for a Home with no taf, under Your tafs (`recipes.homeWords`).
        model.home_words = cloud
            .as_ref()
            .and_then(|c| c.recipes.as_ref())
            .and_then(|r| r.home_words())
            .map(str::to_string);
        // Needs you's row says tafs cloud's name for the brief once there is one
        // (`needsYouTabTitle`).
        if let (Mode::Kiff, Some(b)) = (mode, cloud.as_ref().and_then(|c| c.brief.as_ref())) {
            model.brief_tab = Some(b.words.tab.clone()).filter(|t| !t.is_empty());
        }
        let account = cloud.as_ref().and_then(|c| c.account.clone());
        // `showsAccountHalt`: signed in with KIFF, to someone who manages Cards, while a tafs
        // cloud taf runs, and whenever the account is halted or halting, since that row is
        // where it's resumed from.
        if let (Mode::Kiff, Some(c)) = (mode, cloud.as_deref()) {
            let h = c
                .account_halt
                .clone()
                .unwrap_or_else(tafs_core::halt::Halt::running);
            let manages_cards = account.as_ref().is_none_or(|a| a.allows("cards.manage"));
            let cloud_tafs = c
                .home
                .tafs
                .iter()
                .chain(c.home.other_agents())
                .any(|t| t.id.is_some());
            if manages_cards && (cloud_tafs || h.state != tafs_core::halt::State::Running) {
                model.account_halt = Some(h);
            }
        }
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
        // Home's labels, as chips over the grid and on each taf's line (docs/labels.md); none
        // sent, none drawn. The sample's are the iPad's screenshots' only.
        let overview = match (mode, cloud.as_deref()) {
            (Mode::Kiff, Some(c)) => tafs_core::overview::with_labels(
                words,
                overview,
                c.home.labels(),
                c.home.labels_all().as_deref(),
                c.brief.as_ref(),
            )
            .unwrap_or_else(|e| fail(e)),
            _ => overview,
        };
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
                            .chain(c.home.other_agents())
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
            tafs_core::account::terms_url(words.language()),
            tafs_core::account::privacy_url(words.language()),
            env!("CARGO_PKG_VERSION"),
        )
        .unwrap_or_else(|e| fail(e));
        let me = self.clone();
        let actions: Rc<dyn Fn(AccountAction)> =
            Rc::new(move |action| me.act(action, mode, view_only));
        let record = Rc::new(self.record_data(mode, &model, cloud.as_deref(), at));
        // tafs cloud ended the session: signed out, as every read does.
        let signed_out: Rc<dyn Fn()> = {
            let me = Rc::downgrade(self);
            Rc::new(move || {
                if let Some(me) = me.upgrade() {
                    me.signed_out_by_kiff();
                }
            })
        };
        let tafs =
            Rc::new(self.taf_pages(mode, &model, cloud.as_deref(), may_answer, at, signed_out));
        // Your brief, signed in with KIFF once tafs cloud sends one: read again for a label at
        // tafs cloud as the session, or a screenshot's from `--brief`, never sent.
        let brief = match (
            mode,
            cloud.as_deref(),
            self.kept.borrow().as_ref(),
            &self.brief_file,
        ) {
            (Mode::Kiff, Some(c), kept, file) => c.brief.clone().and_then(|b| {
                let source = match (file, kept) {
                    (Some(f), _) => brief::BriefSource::File(f.clone()),
                    (None, Some(kept)) => brief::BriefSource::Cloud {
                        token: kept.token.clone(),
                        languages: self.languages.clone(),
                    },
                    (None, None) => return None,
                };
                // What Needs you holds that the brief can match by its `ref`, and what it can't
                // carry (`briefNeedsYouLine`).
                let mut items: Vec<String> = model
                    .held
                    .iter()
                    .filter_map(|h| h.kiff_id.as_ref().map(|id| format!("hold:{id}")))
                    .collect();
                items.extend(
                    c.home
                        .your_steps
                        .iter()
                        .flatten()
                        .map(|s| format!("step:{}", s.handoff)),
                );
                let others = model.held.iter().filter(|h| h.kiff_id.is_none()).count()
                    + c.home.card_unit_exceptions.as_ref().map_or(0, Vec::len)
                    + if may_answer { c.unproven.len() } else { 0 }
                    + c.home.coach_waiting.as_ref().map_or(0, Vec::len);
                Some(Rc::new(brief::BriefPane {
                    brief: b,
                    source,
                    may_use_coach: model.may_use_coach,
                    tafs: tafs
                        .iter()
                        .filter_map(|t| {
                            Some(brief::BriefTaf {
                                cloud_id: t.cloud_id.clone()?,
                                key: t.key.clone(),
                                name: t.name.clone(),
                                initial: t.initial.clone(),
                                color: t.color,
                            })
                        })
                        .collect(),
                    needs_you_items: items,
                    needs_you_others: others as i64,
                    log_file: self.mission_log_file.clone(),
                }))
            }),
            _ => None,
        };
        let screens = shell::Screens {
            brief,
            overview,
            needs_you,
            account: account_lines,
            // Signed in: Account's reads at tafs cloud as the session; a screenshot's from
            // `--account-reads`, never sent.
            account_reads: match (mode, self.kept.borrow().as_ref(), &self.account_reads_file) {
                (Mode::Kiff, _, Some(file)) => Some(ai_consent::Source::File(file.clone())),
                (Mode::Kiff, Some(kept), None) => Some(ai_consent::Source::Cloud {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                }),
                _ => None,
            }
            .map(|source| {
                Rc::new(ai_consent::Reads {
                    source,
                    may_use_coach: model.may_use_coach,
                    manages_cards: account.as_ref().is_none_or(|a| a.allows("cards.manage")),
                })
            }),
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
            // A halt is read and sent as the session, signed in with KIFF.
            halt: match (mode, self.kept.borrow().as_ref(), &self.halt_file) {
                // `--halt`: the screenshots' answer, never sent to tafs cloud.
                (Mode::Kiff, _, Some(h)) => Some(Rc::new(halt::HaltSource {
                    token: String::new(),
                    languages: vec![],
                    canned: Some(h.clone()),
                })),
                (Mode::Kiff, Some(kept), None) => Some(Rc::new(halt::HaltSource {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                    canned: None,
                })),
                _ => None,
            },
            // Export my chats, as the session: a kept one only, never a screenshot's.
            export: match (mode, self.kept.borrow().as_ref(), self.screenshot) {
                (Mode::Kiff, Some(kept), false) => Some(Rc::new(kiff_account::ExportSource {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                })),
                _ => None,
            },
            // The gallery, signed in with KIFF once read: Explore jobs, and with tafs the strip.
            gallery: match (mode, cloud.as_deref()) {
                (Mode::Kiff, Some(c)) => c.gallery.clone().map(|gallery| {
                    Rc::new(recipes::GalleryPane {
                        gallery,
                        labels: c.recipes.as_ref().and_then(|r| r.labels.clone()),
                        may_use_coach: model.may_use_coach,
                    })
                }),
                _ => None,
            },
            // `homeListsRecipes`: signed in with KIFF, Home read, no taf, and tafs cloud lists them.
            recipes: match (mode, cloud.as_deref()) {
                (Mode::Kiff, Some(c)) if model.tafs.is_empty() => c
                    .recipes
                    .clone()
                    .filter(|r| r.home_lists_recipes())
                    .map(|recipes| {
                        Rc::new(recipes::RecipesPane {
                            recipes,
                            may_use_coach: model.may_use_coach,
                            hand_words: None,
                            sample_label: None,
                        })
                    }),
                _ => None,
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
            alerts: alerts::AlertsSource {
                kiff: (mode == Mode::Kiff).then(|| {
                    cloud
                        .as_deref()
                        .and_then(|c| c.account.as_ref())
                        .map(|a| tafs_core::hold_alert::notifies(a.capabilities.as_deref()))
                }),
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
                    connectable: tafs
                        .iter()
                        .filter(|t| t.manages)
                        .filter_map(|t| t.cloud_id.clone())
                        .collect(),
                },
                _ => connect::ConnectSource::None,
            },
            // Coach, signed in with KIFF: its conversation read as the session with this device's
            // proof, or a screenshot's from `--coach`, never sent. The sample has no Coach.
            coach: match (mode, self.kept.borrow().as_ref(), &self.coach_file) {
                (Mode::Kiff, _, Some(f)) => coach::CoachSource::File {
                    thread: f.clone(),
                    chats: self.coach_chats.clone(),
                },
                (Mode::Kiff, Some(kept), None) => coach::CoachSource::Cloud {
                    kept: Box::new(kept.clone()),
                    languages: self.languages.clone(),
                    // tafs cloud ended the session: signed out, as every read does.
                    signed_out: {
                        let me = Rc::downgrade(self);
                        Rc::new(move || {
                            if let Some(me) = me.upgrade() {
                                me.signed_out_by_kiff();
                            }
                        })
                    },
                },
                _ => coach::CoachSource::None,
            },
            // The account's tafs by KIFF agent: a result's button opens the one it set up.
            coach_listed: Rc::new(
                cloud
                    .as_deref()
                    .map(|c| {
                        c.home
                            .tafs
                            .iter()
                            .filter_map(|h| tafs_core::home::taf(words, h).ok())
                            .map(|t| tafs_core::coach_lines::Listed {
                                agent: t.agent,
                                key: t.key,
                                name: t.name,
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            ),
            cloud_names: Rc::new(
                tafs.iter()
                    .filter_map(|t| Some((t.cloud_id.clone()?, t.name.clone(), t.manages)))
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

    /// A language picked in Account › Language (`AppLanguage.choose`): kept for the next launch,
    /// and applied at once, the window drawn again in it, on Account. Every read is made again,
    /// asking in the new language first. This window's minute reads stop with it.
    fn use_language(self: &Rc<Self>, code: Option<&'static str>) {
        if store::language() == code {
            return;
        }
        store::set_language(code);
        let Some(app) = self.window.application() else {
            return;
        };
        self.held_watch.set(self.held_watch.get() + 1);
        REDRAWN_FOR_LANGUAGE.with(|r| r.set(true));
        app.activate();
        self.window.close();
    }

    /// Account's actions: sign in from the sample, clear the sample, or sign out of KIFF.
    fn act(self: &Rc<Self>, action: AccountAction, mode: Mode, view_only: bool) {
        match action {
            AccountAction::SignIn => self.present_sign_in(None),
            // The sample is forgotten, and the welcome opens.
            AccountAction::ClearSample => self.show_welcome(),
            // Account opens its picker itself.
            AccountAction::ChangeLanguage => {}
            AccountAction::UseLanguage(code) => self.use_language(code),
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

/// The account's sidebar and Overview from Home, KIFF's held calls and its activity: the tafs a
/// call waits on first, then the one with the newest call in the Record, then tafs cloud's order
/// (`listedTafs`, `byRecent`); Your tafs those relevant now, and tafs cloud's count of what waits.
fn home_model(
    w: &WordBook,
    h: &Home,
    exceptions: &[CardException],
    activity: &[tafs_core::record::ActivityCall],
    access: &std::collections::BTreeMap<String, tafs_core::access::Access>,
    balance: Option<&tafs_core::usage::Balance>,
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
    // Its newest call in the Record, by its KIFF agent.
    let latest = |t: &tafs_core::home::Taf| {
        activity
            .iter()
            .filter(|c| c.actor_id.as_deref() == Some(t.agent.as_str()))
            .map(|c| c.created_at)
            .max()
    };
    let ordered = tafs_core::sidebar::by_recent(&tafs, waits, latest);
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
    // `sharedWithMe`: KIFF's answer says use without manage; no answer yet is not "shared".
    let shared = tafs
        .iter()
        .filter(|t| access.get(&t.agent).is_some_and(|a| !a.manage))
        .map(|t| t.key.clone())
        .collect();
    // Agents made through KIFF's API, not run by tafs: their Cards, apart (`otherAgents`).
    let others = h
        .other_agents()
        .iter()
        .filter_map(|o| tafs_core::home::taf(w, o).ok())
        .map(|t| tafs_core::home::row(&t))
        .collect();
    let model = SidebarModel {
        list_loaded: true,
        plan: balance.and_then(|b| b.home_line.clone().map(|l| (l, b.plan_tone.clone()))),
        brief_tab: None,
        tafs: rows,
        held,
        cloud_needs_you: h.needs_you.as_ref().map(|n| n.count),
        shared,
        others,
        setting_up: h
            .setting_up()
            .iter()
            .map(|s| (s.row(), s.discard.as_ref().map(|d| d.button.clone())))
            .collect(),
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
        account_halt: None,
        shared: Default::default(),
        others: vec![],
        setting_up: vec![],
        plan: None,
        brief_tab: None,
    }
}

/// `--recipes`'s file, as tafs cloud answers `GET /v1/recipes` (a contract file's `answer`, or the
/// answer itself). None given, none listed; a file that can't be read stops the app.
fn recipes_file(path: Option<&str>) -> Option<tafs_core::recipes::Recipes> {
    let path = path?;
    let body = std::fs::read(path).ok();
    let answer = body.as_deref().and_then(|b| {
        let v: serde_json::Value = serde_json::from_slice(b).ok()?;
        let a = v.get("answer").cloned().unwrap_or(v);
        serde_json::to_vec(&a).ok()
    });
    match answer.and_then(|a| tafs_core::recipes::Recipes::read(&a)) {
        Some(r) => Some(r),
        None => {
            eprintln!("{path} isn't the recipes as tafs cloud answers them");
            std::process::exit(2);
        }
    }
}

/// `--record-search`'s file: `{ "page": …, "case": … }`, the search page and a case as tafs cloud
/// answers them (a contract file's `answer`, or the answers themselves). A file that can't be
/// read stops the app.
fn record_search_file(
    path: Option<&str>,
) -> Option<(
    tafs_core::record_search::SearchPage,
    Option<tafs_core::record_search::RecordCase>,
)> {
    use tafs_core::record_search::{RecordCase, SearchPage};
    let path = path?;
    let read = std::fs::read(path).ok().and_then(|b| {
        let v: serde_json::Value = serde_json::from_slice(&b).ok()?;
        let a = v.get("answer").cloned().unwrap_or(v);
        let page = SearchPage::read(&serde_json::to_vec(a.get("page")?).ok()?)?;
        let case = a
            .get("case")
            .and_then(|c| RecordCase::read(&serde_json::to_vec(c).ok()?));
        Some((page, case))
    });
    match read {
        Some(r) => Some(r),
        None => {
            eprintln!("{path} isn't a Record search as tafs cloud answers it");
            std::process::exit(2);
        }
    }
}

/// `--halt`'s file, as tafs cloud answers a halt (`{ "halt": … }`, a contract file's `answer`, or
/// the answer itself). None given, none; a file that can't be read stops the app.
fn halt_file(path: Option<&str>) -> Option<tafs_core::halt::Halt> {
    let path = path?;
    let answer = std::fs::read(path).ok().and_then(|b| {
        let v: serde_json::Value = serde_json::from_slice(&b).ok()?;
        serde_json::to_vec(&v.get("answer").cloned().unwrap_or(v)).ok()
    });
    match answer.and_then(|a| tafs_core::halt::read(&a)) {
        Some(h) => Some(h),
        None => {
            eprintln!("{path} isn't a halt as tafs cloud answers it");
            std::process::exit(2);
        }
    }
}

/// `--gallery`'s file, as tafs cloud answers `GET /v1/recipes/gallery` (a contract file's
/// `answer`, or the answer itself). None given, none; a file that can't be read stops the app.
fn gallery_file(path: Option<&str>) -> Option<tafs_core::recipes::Gallery> {
    let path = path?;
    let answer = std::fs::read(path).ok().and_then(|b| {
        let v: serde_json::Value = serde_json::from_slice(&b).ok()?;
        serde_json::to_vec(&v.get("answer").cloned().unwrap_or(v)).ok()
    });
    match answer.and_then(|a| tafs_core::recipes::Gallery::read(&a)) {
        Some(g) => Some(g),
        None => {
            eprintln!("{path} isn't the gallery as tafs cloud answers it");
            std::process::exit(2);
        }
    }
}

/// `--balance`'s answer (a contract file's `answer`, or the answer itself): the screenshots'
/// plan line. None given, none shown; a file that can't be read stops the app.
fn balance_file(path: Option<&str>) -> Option<tafs_core::usage::Balance> {
    let path = path?;
    let read = std::fs::read(path).ok().and_then(|b| {
        let v: serde_json::Value = serde_json::from_slice(&b).ok()?;
        let a = v.get("answer").cloned().unwrap_or(v);
        tafs_core::usage::Balance::read(&serde_json::to_vec(&a).ok()?)
    });
    match read {
        Some(b) => Some(b),
        None => {
            eprintln!("{path} isn't a balance as tafs cloud answers it");
            std::process::exit(2);
        }
    }
}

/// `nameRecordLines`: the call refs of each tafs cloud taf with a Record line KIFF hashed a
/// resource for, read as the session. Best effort: a taf whose calls can't be read keeps its
/// lines without what they acted on.
fn call_refs(
    token: &str,
    languages: &[&str],
    home: &Home,
    activity: &[tafs_core::record::ActivityCall],
) -> std::collections::BTreeMap<String, String> {
    let agents = tafs_core::record::agents_to_name(activity);
    let mut refs = std::collections::BTreeMap::new();
    for t in &home.tafs {
        let Some(id) = t.id.as_deref().filter(|_| agents.contains(&t.kiff_agent)) else {
            continue;
        };
        let read = kiff::cloud(token, languages, &tafs_core::record::calls_path(id))
            .ok()
            .filter(|a| a.status == 200)
            .and_then(|a| tafs_core::record::read_calls(&a.body));
        for r in read.unwrap_or_default() {
            refs.insert(r.invocation, r.value);
        }
    }
    refs
}

/// `--calls`' answer: one taf's call refs, for the screenshots' Record.
fn calls_file(path: Option<&str>) -> std::collections::BTreeMap<String, String> {
    let Some(path) = path else {
        return Default::default();
    };
    match std::fs::read(path)
        .ok()
        .and_then(|b| tafs_core::record::read_calls(&b))
    {
        Some(calls) => calls.into_iter().map(|r| (r.invocation, r.value)).collect(),
        None => {
            eprintln!("{path} isn't a taf's calls as tafs cloud answers it");
            std::process::exit(2);
        }
    }
}

/// `--activity`'s answer: KIFF's activity for the screenshots' Record.
fn activity_file(path: Option<&str>) -> Vec<tafs_core::record::ActivityCall> {
    let Some(path) = path else {
        return vec![];
    };
    match std::fs::read(path)
        .ok()
        .and_then(|b| tafs_core::record::read_activity(&b))
    {
        Some(calls) => calls,
        None => {
            eprintln!("{path} isn't KIFF's activity");
            std::process::exit(2);
        }
    }
}

/// `--access`'s file, as tafs cloud answers `GET /v1/me/access` (a contract file's `answer`, or
/// the answer itself). None given, none read; a file that can't be read stops the app.
fn access_file(
    path: Option<&str>,
) -> std::collections::BTreeMap<String, tafs_core::access::Access> {
    let Some(path) = path else {
        return Default::default();
    };
    let read = std::fs::read(path).ok().and_then(|b| {
        let v: serde_json::Value = serde_json::from_slice(&b).ok()?;
        let a = v.get("answer").cloned().unwrap_or(v);
        tafs_core::access::read_access(&serde_json::to_vec(&a).ok()?)
    });
    match read {
        Some(a) => a,
        None => {
            eprintln!("{path} isn't access as tafs cloud answers it");
            std::process::exit(2);
        }
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
