//! tafs for Linux: GTK 4 and libadwaita. The same product, account and words as the iPad app.

mod account;
mod ai_consent;
mod alerts;
mod allow_sheet;
mod app_event_sheet;
mod app_sign_in;
mod approval;
mod attach;
mod brief;
mod calendar;
mod chat;
mod chrome;
mod chrome_shots;
mod coach;
mod coach_mode;
mod coach_start;
mod code_sign_in;
mod connect;
mod connected;
mod connections;
mod devices;
mod dial;
mod doors;
mod fido_key;
mod files;
mod fold;
mod halt;
mod illustrations;
mod key_approval;
mod kiff;
mod kiff_account;
mod knocks;
mod language_picker;
mod needs_you;
mod onboarding;
mod overview;
mod pairing;
mod payment_report_sheet;
mod people;
mod pin_sheet;
mod receipt_sheet;
mod receipts;
mod recipes;
mod record;
mod record_search;
mod report_sheet;
mod runs_panel;
mod sample_setup;
mod scheme;
mod setup_pane;
mod share;
mod sheet;
mod shell;
mod sign_in;
mod starts_when;
mod store;
mod style;
mod taf;
mod taf_chat;
mod update;
mod usage;
mod versions;
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
    /// The account's connections from planque, as this computer read them.
    grants: Vec<tafs_core::connections::Grant>,
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
    /// Coach's id at tafs cloud (`setupTaf?.id`, `GET /v1/setup`), read with Home when a row's
    /// setup stopped partway: only Coach's own chat can be finished there.
    coach_setup: Option<String>,
}

/// The window's first size on this display's first screen (`window::first_size`), or a laptop's
/// when the display names no screen.
fn first_size(display: &gtk::gdk::Display) -> (i32, i32) {
    let monitor = display
        .monitors()
        .item(0)
        .and_then(|m| m.downcast::<gtk::gdk::Monitor>().ok());
    match monitor {
        Some(m) => {
            let g = m.geometry();
            tafs_core::window::first_size(g.width(), g.height())
        }
        None => (WIDTH, HEIGHT),
    }
}

/// The screenshots' size, a laptop's.
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
    /// tafs cloud's Files contract (`files-list.json`, `files-file.json` beside it), for Files' screenshots.
    files: Option<String>,
    /// planque's Chromes list (`--chromes`), for Account › Chrome's screenshots.
    chromes: Option<String>,
    connect: Option<String>,
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
    /// With `--home`: a taf's knob as tafs cloud answers `GET /v1/tafs/{id}/knob` (Starts when,
    /// Results go to, the dial); its changes send nothing.
    knob: Option<String>,
    /// With `--home`: a taf's Setup as tafs cloud answers `GET /v1/tafs/{id}/setup`, in place of
    /// the contract's `taf-setup.json`.
    setup: Option<String>,
    /// With `--home`: a taf's Activity as tafs cloud answers `GET /v1/tafs/{id}/activity`.
    taf_activity: Option<String>,
    /// With `--home`: Account's reads, `{"suggestions": …, "consent": …}`, each as tafs cloud
    /// answers it; its changes send nothing.
    account_reads: Option<String>,
    /// With `--home`: the KIFF account as `GET /v1/me` answers it, for screens drawn by who may
    /// do what.
    me: Option<String>,
    /// A chat's tray of attached files, for the screenshots.
    attach: Option<String>,
    /// KIFF's held calls (`GET /v1/me/card-exceptions?status=held`), beside `--home`.
    held: Option<String>,
    /// A taf's versions and its open trial, as tafs cloud answers them, for the screenshots.
    versions: Option<String>,
    trial: Option<String>,
    /// `--trial-draft`: KIFF's draft a trial's Card is, from a file, for the start sheet.
    trial_draft: Option<String>,
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
    /// `--watch-days`: "From your watch" in Previous conversations, from this file's answers.
    watch_days: Option<String>,
    /// `--chrome-shots <file>`: a Chrome step's pictures, `{"list": …, "shot": …}`, for a receipt.
    chrome_shots: Option<String>,
    /// With `--coach`: KIFF's answers to the drafts its approvals name, by draft id, each
    /// `{"status": …, "answer": …}`: nothing is read.
    coach_drafts: Option<String>,
    /// Coach as tafs cloud answers `GET /v1/setup`: with `--coach`, for Coach's tools; with
    /// `--home`, for Finish setting up. Nothing is read.
    coach_setup: Option<String>,
    /// With `--coach`: an answer streaming in it, from a stream's events. Nothing is read.
    coach_stream: Option<String>,
    /// With `--chat`: an answer streaming in it, from a stream's events. Nothing is read.
    chat_stream: Option<String>,
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
    /// With `--home`: Home couldn't be read just now (`network` or `error`), and the app tries
    /// again, for the Unreachable line's screenshot. Nothing is read again.
    unreachable: Option<String>,
    /// `--new-version v1.0.2`: a newer published release, as GitHub's answer would say it, for
    /// the sidebar's "A new version of tafs is out" line. Nothing is read from GitHub.
    new_version: Option<String>,
    /// A dial's stop with its approval, as tafs cloud sends it in the knob's `wakes`: its sheet
    /// opens over the window, for its screenshot. Nothing is signed or subscribed.
    app_event: Option<String>,
    /// `--size WxH`: the window's size, for a screenshot of a narrower layout.
    size: Option<(i32, i32)>,
    screenshot: Option<String>,
    /// KIFF's way back after its sign-in page (`dev.kiff.tafs:/oauth?…`), handed over by the
    /// desktop: it goes to the running app's waiting attempt (`app_sign_in::link`).
    link: Option<String>,
}

const USAGE: &str = "tafs [dev.kiff.tafs:/oauth?…] [--lang <a language tafs speaks: en, de, fr, ar, ja, zh-Hant, pt-BR…>] [--theme light|dark] [--sample] [--signed-in] [--view-only [--paired]]
     [--home contract.json] [--usage answers.json] [--people answers.json] [--files files-list.json] [--chromes answer.json] [--connect answers.json] [--knob answer.json] [--setup answer.json] [--taf-activity answer.json] [--unproven answers.json] [--recipes answers.json] [--gallery answers.json] [--halt answer.json] [--record-search answers.json] [--access answer.json] [--receipts answer.json] [--activity answer.json] [--calls answer.json] [--receipt answer.json] [--work answer.json] [--doors answer.json] [--account-reads answers.json] [--me answer.json] [--attach files.json] [--held answers.json] [--versions answer.json [--trial answer.json]] [--memory answer.json] [--runs answer.json] [--request answer.json] [--balance answer.json] [--coach answer.json] [--coach-chats answer.json] [--watch-days answers.json] [--chrome-shots answers.json] [--coach-drafts answers.json] [--chat answer.json [--report] [--chat-stream events.txt [--pin-check]]] [--brief answer.json] [--mission-log answer.json] [--coach-setup answer.json] [--coach-stream events.txt] [--unreachable network|error] [--new-version vX.Y.Z] [--app-event item.json] [--size WxH] [--page welcome|sign-in|pair|recipe:<id>|gallery|new-taf|halt-all|halt:<id>|halt-resume:<id>|record-search:<q>|record-export|receipt:<proposal id>|run:<taf id>|pin-sheet|share:<key>|share-step2:<key>|coach|coach-tools|coach-full|coach-read-back|coach-hand:<id>|coach-improve:<key>|overview|overview-label:<id>|needs-you|needs-you-alerts|needs-you-key|needs-you-key-room|account-sign-in|discard-setup|needs-you-set-all-aside|needs-you-dismiss|needs-you-hold:<id>|versions:<key>|trial:<key>|brief-answer:<decision id>|mission-log:<mission id>|record|calendar|calendar-week|usage|usage-ask|usage-decline|connect-apps|connect-app|connect-connection|connect-remove|connect-which-taf|connect-approve|account|kiff-account|time-zone|time-zone-use|alerts|devices|devices-sign-out|people|people-rename|people-add|taf|taf-card|taf-sample-chat|taf-setup|taf-setup:<id>|starts:<id>|results:<id>|doors:<id>[:<door>]|taf-receipt:<key>:<entry>|taf-receipt-shot:<key>:<entry>|taf-folded:<id>|files|files-file|files-taf:<id>|chrome|chrome-unpair|taf-report:<key>:<entry>|consent|connected|sign-in-code] [--screenshot out.png]";

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
            "--files" => o.files = Some(value()?),
            "--chromes" => o.chromes = Some(value()?),
            "--connect" => o.connect = Some(value()?),
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
            "--knob" => o.knob = Some(value()?),
            "--setup" => o.setup = Some(value()?),
            "--taf-activity" => o.taf_activity = Some(value()?),
            "--account-reads" => o.account_reads = Some(value()?),
            "--me" => o.me = Some(value()?),
            "--attach" => o.attach = Some(value()?),
            "--held" => o.held = Some(value()?),
            "--versions" => o.versions = Some(value()?),
            "--trial" => o.trial = Some(value()?),
            "--trial-draft" => o.trial_draft = Some(value()?),
            "--memory" => o.memory = Some(value()?),
            "--runs" => o.runs = Some(value()?),
            "--balance" => o.balance = Some(value()?),
            "--request" => o.request = Some(value()?),
            "--coach" => o.coach = Some(value()?),
            "--coach-chats" => o.coach_chats = Some(value()?),
            "--watch-days" => o.watch_days = Some(value()?),
            "--chrome-shots" => o.chrome_shots = Some(value()?),
            "--coach-drafts" => o.coach_drafts = Some(value()?),
            "--coach-setup" => o.coach_setup = Some(value()?),
            "--coach-stream" => o.coach_stream = Some(value()?),
            "--chat" => o.chat = Some(value()?),
            "--chat-stream" => o.chat_stream = Some(value()?),
            "--pin-check" => crate::chat::PIN_CHECK.with(|c| c.set(true)),
            "--report" => o.report = true,
            "--brief" => o.brief = Some(value()?),
            "--mission-log" => o.mission_log = Some(value()?),
            "--unreachable" => o.unreachable = Some(value()?),
            "--new-version" => o.new_version = Some(value()?),
            "--app-event" => o.app_event = Some(value()?),
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
            link if is_link(link) => o.link = Some(link.to_string()),
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

/// GTK's own renderer draws with GL through libepoxy, which stops the app outright when the GL
/// library it needs can't be opened ("Couldn't open libGLESv2.so.2", on a system with no GL ES,
/// such as a bare WSL). With no renderer asked for, and EGL or GL ES not there, GTK draws with
/// cairo, in software, instead.
fn renderer_without_gl() -> Option<&'static str> {
    if std::env::var_os("GSK_RENDERER").is_some_and(|r| !r.is_empty()) {
        return None;
    }
    let opens = |name: &str| {
        // SAFETY: only opened to see that it can be, by the system's own loader, and closed again.
        unsafe { libloading::Library::new(name) }.is_ok()
    };
    if opens("libEGL.so.1") && opens("libGLESv2.so.2") {
        None
    } else {
        Some("cairo")
    }
}

fn main() -> glib::ExitCode {
    // Before GTK starts, and before any thread: the renderer it can draw with here.
    if let Some(renderer) = renderer_without_gl() {
        eprintln!("no GL ES here: drawing with {renderer}");
        std::env::set_var("GSK_RENDERER", renderer);
    }
    let options = match options() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("{e}");
            return glib::ExitCode::FAILURE;
        }
    };
    gio::resources_register_include!("tafs.gresource").expect("the app's resources");
    // Run plainly (with no option, or only a link back: KIFF's, or WhatsApp's signup's), tafs is one app on the desktop: a
    // second launch, as the browser makes with `dev.kiff.tafs:/oauth?…`, hands its link to the
    // window that waits for it. With options (a screenshot, the pin check), each run is its own.
    let plain = std::env::args().skip(1).all(|a| is_link(&a));
    let flags = if plain {
        gio::ApplicationFlags::HANDLES_OPEN
    } else {
        gio::ApplicationFlags::NON_UNIQUE
    };
    if plain {
        scheme::register_appimage();
    }
    let app = adw::Application::builder()
        .application_id("dev.kiff.tafs")
        .flags(flags)
        .build();
    let options = Rc::new(options);
    {
        let options = options.clone();
        app.connect_activate(move |app| activate(app, &options));
    }
    app.connect_open(move |app, files, _| {
        // Launched by the link itself: the window first, then the link, which no attempt in this
        // new window waits for, so it's dropped.
        if app.active_window().is_none() {
            activate(app, &options);
        }
        for f in files {
            let uri = f.uri();
            if tafs_core::whatsapp_signup::is_return(&uri) {
                doors::whatsapp_link(&uri);
            } else {
                app_sign_in::link(&uri);
            }
        }
    });
    // GTK reads no arguments of ours, only the link back.
    match options_link() {
        Some(link) => app.run_with_args(&["tafs", link.as_str()]),
        None => app.run_with_args::<&str>(&[]),
    }
}

/// A link back the desktop hands tafs: KIFF's after its sign-in page, or WhatsApp's signup's.
fn is_link(a: &str) -> bool {
    a.starts_with("dev.kiff.tafs:") || tafs_core::whatsapp_signup::is_return(a)
}

/// The link back, if the desktop launched tafs with one.
fn options_link() -> Option<String> {
    std::env::args().skip(1).find(|a| is_link(a))
}

/// The page a screenshot or `--page` opens beside the sidebar.
fn selection_of(page: Option<&str>) -> Selection {
    match page {
        Some(
            "needs-you"
            | "needs-you-alerts"
            | "needs-you-key"
            | "needs-you-key-room"
            | "needs-you-set-all-aside"
            | "needs-you-dismiss",
        ) => Selection::NeedsYou,
        Some(other) if other.starts_with("brief-answer:") => Selection::NeedsYou,
        Some(other) if other.starts_with("needs-you-hold:") => Selection::NeedsYou,
        Some(other) if other.starts_with("versions:") => {
            Selection::Taf(other["versions:".len()..].into())
        }
        Some(other) if other.starts_with("try:") => Selection::Taf(other["try:".len()..].into()),
        Some(other) if other.starts_with("trial:") => {
            Selection::Taf(other["trial:".len()..].into())
        }
        Some(other) if other.starts_with("mission-log:") => Selection::NeedsYou,
        Some("record") => Selection::Record,
        Some("calendar") | Some("calendar-week") => Selection::Calendar,
        Some("usage") | Some("usage-ask") | Some("usage-decline") => Selection::Usage,
        Some("connect-apps")
        | Some("connect-app")
        | Some("connect-connection")
        | Some("connect-remove")
        | Some("connect-which-taf")
        | Some("connect-approve") => Selection::ConnectApps,
        Some("account") => Selection::Account,
        // Account with Sign-in's setting brought into view, for its screenshot.
        Some("account-sign-in") => {
            code_sign_in::FOCUS.with(|f| f.set(true));
            Selection::Account
        }
        Some("devices") | Some("devices-sign-out") => Selection::Devices,
        Some("people") | Some("people-rename") | Some("people-add") => Selection::People,
        Some("files") | Some("files-file") => Selection::Files(None),
        Some("chrome") => Selection::Chrome,
        Some("chrome-unpair") => {
            chrome::OPEN_UNPAIR.with(|o| o.set(true));
            Selection::Chrome
        }
        Some(other) if other.starts_with("files-taf:") => {
            Selection::Files(Some(other["files-taf:".len()..].into()))
        }
        Some("kiff-account") | Some("delete-account") => Selection::KiffAccount,
        Some("time-zone") | Some("time-zone-use") => Selection::TimeZone,
        Some("alerts") => Selection::Alerts,
        Some("language") => Selection::Language,
        Some("coach")
        | Some("coach-tools")
        | Some("coach-full")
        | Some("coach-read-back")
        | Some("coach-watch-day") => Selection::Coach,
        Some(other) if other.starts_with("coach-hand:") || other.starts_with("coach-improve:") => {
            Selection::Coach
        }
        // The sample's Support replies, as the reference set's taf.png opens it.
        Some("taf") | Some("taf-card") | Some("taf-sample-chat") => {
            Selection::Taf("support".into())
        }
        // With `--home`, the contract's first taf.
        Some(other) if other.starts_with("taf:") => Selection::Taf(other["taf:".len()..].into()),
        // A taf's page with its head folded, as after scrolling its chat down.
        Some(other) if other.starts_with("taf-folded:") => {
            fold::OPEN_FOLDED.with(|f| f.set(true));
            Selection::Taf(other["taf-folded:".len()..].into())
        }
        Some(other) if other.starts_with("run:") => Selection::Taf(other["run:".len()..].into()),
        Some(other) if other.starts_with("share:") => {
            Selection::Taf(other["share:".len()..].into())
        }
        Some(other) if other.starts_with("share-step2:") => {
            Selection::Taf(other["share-step2:".len()..].into())
        }
        // `taf-receipt:<key>:<entry>` and `taf-report:<key>:<entry>`: that taf's page.
        Some(other)
            if other.starts_with("taf-receipt:")
                || other.starts_with("taf-receipt-shot:")
                || other.starts_with("taf-report:") =>
        {
            let rest = other.split_once(':').map(|(_, r)| r).unwrap_or_default();
            Selection::Taf(rest.split_once(':').map(|(k, _)| k).unwrap_or(rest).into())
        }
        Some("halt-all") => Selection::HaltAll,
        Some(other) if other.starts_with("record-search:") => Selection::Record,
        Some("record-export") => Selection::Record,
        Some(other) if other.starts_with("receipt:") => Selection::Record,
        Some(other) if other.starts_with("halt:") => Selection::Halt(other["halt:".len()..].into()),
        Some(other) if other.starts_with("halt-resume:") => {
            Selection::Halt(other["halt-resume:".len()..].into())
        }
        Some(other) if other.starts_with("taf-card:") => {
            Selection::Taf(other["taf-card:".len()..].into())
        }
        Some(other) if other.starts_with("taf-setup:") => {
            Selection::Taf(other["taf-setup:".len()..].into())
        }
        Some(other) if other.starts_with("starts:") => {
            Selection::Taf(other["starts:".len()..].into())
        }
        Some(other) if other.starts_with("results:") => {
            Selection::Taf(other["results:".len()..].into())
        }
        Some(other) if other.starts_with("doors:") => {
            let rest = &other["doors:".len()..];
            Selection::Taf(rest.split(':').next().unwrap_or(rest).into())
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
    // KIFF knows a language by its code ("pt-BR" is "pt"), the words' one first.
    let first = language::kiff_code(lang);
    let mut languages = vec![first];
    languages.extend(
        language::screen_languages(&preferred)
            .into_iter()
            .filter(|l| *l != first),
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
    // Right to left for Arabic: GTK mirrors every box, list and arrow, as SwiftUI does on the iPad.
    // The dials and the drawn boards keep their screen angles (kiff/tafs `docs/languages.md`,
    // "Direction"): they're objects, not a leading or trailing edge.
    gtk::Widget::set_default_direction(if tafs_core::locale::right_to_left(lang) {
        gtk::TextDirection::Rtl
    } else {
        gtk::TextDirection::Ltr
    });
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

    // Its first size: a screenshot's own, else sized to the screen near 4:3; 900×600 is the
    // least it can be made, never the size it opens at.
    let (width, height) = match (o.size, screenshot) {
        (Some(s), _) => s,
        (None, true) => (WIDTH, HEIGHT),
        (None, false) => first_size(&display),
    };
    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(width)
        .default_height(height)
        .title(words.line("app.name").unwrap_or_default())
        .build();
    window.set_size_request(tafs_core::window::MIN.0, tafs_core::window::MIN.1);

    let app_state = Rc::new(App {
        window: window.clone(),
        shell: RefCell::new(None),
        unreachable: Cell::new(o.unreachable.as_deref().map(|why| {
            (
                if why == "error" {
                    tafs_core::sidebar::Unreachable::Error
                } else {
                    tafs_core::sidebar::Unreachable::Network
                },
                0,
            )
        })),
        account_reads: Cell::new(0),
        home_rereads: Cell::new(0),
        home_pulling: Cell::new(false),
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
        knob_file: o.knob.as_ref().map(std::path::PathBuf::from),
        setup_file: o.setup.as_ref().map(std::path::PathBuf::from),
        taf_activity_file: o.taf_activity.as_ref().map(std::path::PathBuf::from),
        account_reads_file: o.account_reads.as_ref().map(std::path::PathBuf::from),
        memory_file: o.memory.as_ref().map(std::path::PathBuf::from),
        runs_file: o.runs.as_ref().map(std::path::PathBuf::from),
        request_file: o.request.as_ref().map(std::path::PathBuf::from),
        coach_file: o.coach.as_ref().map(std::path::PathBuf::from),
        coach_chats: o.coach_chats.as_ref().map(std::path::PathBuf::from),
        coach_drafts: o.coach_drafts.as_ref().map(std::path::PathBuf::from),
        coach_setup: o.coach_setup.as_ref().map(std::path::PathBuf::from),
        coach_stream: o.coach_stream.as_ref().map(std::path::PathBuf::from),
        chat_file: o.chat.as_ref().map(std::path::PathBuf::from),
        chat_stream: o.chat_stream.as_ref().map(std::path::PathBuf::from),
        open_report: o.report,
        brief_file: o.brief.as_ref().map(std::path::PathBuf::from),
        mission_log_file: o.mission_log.as_ref().map(std::path::PathBuf::from),
        open_run: o.page.as_deref().is_some_and(|p| p.starts_with("run:")),
        usage_file: o.usage.as_ref().map(std::path::PathBuf::from),
        people_file: o.people.as_ref().map(std::path::PathBuf::from),
        files_file: o.files.as_ref().map(std::path::PathBuf::from),
        chromes_file: o.chromes.as_ref().map(std::path::PathBuf::from),
        connect_file: o.connect.as_ref().map(std::path::PathBuf::from),
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
        on_sample_chat: o.page.as_deref() == Some("taf-sample-chat"),
        on_setup: o.page.as_deref().is_some_and(|p| {
            p.starts_with("taf-setup:") || p.starts_with("starts:") || p.starts_with("results:")
        }),
        on_setup_screen: match o.page.as_deref() {
            Some(p) if p.starts_with("starts:") => Some(starts_when::Screen::StartsWhen),
            Some(p) if p.starts_with("results:") => Some(starts_when::Screen::ResultsGoTo),
            _ => None,
        },
        on_doors: o
            .page
            .as_deref()
            .and_then(|p| p.strip_prefix("doors:"))
            .map(|rest| {
                rest.split_once(':').map(|(_, door)| {
                    // A screenshot's press after the door (`doors:<taf>:<door>:<button>`).
                    match door.split_once(':') {
                        Some((door, press)) => {
                            doors::PRESS.with(|p| *p.borrow_mut() = Some(press.to_string()));
                            door.to_string()
                        }
                        None => door.to_string(),
                    }
                })
            }),
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
    // `--page coach-hand:<recipe id>`: a job handed to Coach from its sheet, its words in Coach's
    // box (`handRecipeToCoach`); nothing is sent.
    if let Some(id) = o
        .page
        .as_deref()
        .and_then(|p| p.strip_prefix("coach-hand:"))
    {
        if let Some(r) = recipes_file(o.recipes.as_deref())
            .and_then(|r| r.recipes.into_iter().find(|x| x.id == id))
        {
            coach::set_draft(&r.say);
        }
    }
    // `--watch-days`: "From your watch" read from a file, for its screenshots.
    if let Some(path) = &o.watch_days {
        coach::WATCH_FILE.with(|f| *f.borrow_mut() = Some(path.into()));
        coach::OPEN_WATCH_DAY.with(|f| f.set(o.page.as_deref() == Some("coach-watch-day")));
    }
    if let Some(path) = &o.chrome_shots {
        chrome_shots::FILE.with(|f| *f.borrow_mut() = Some(path.into()));
    }
    // `--page files-file`: the first file's sheet over Files.
    if o.page.as_deref() == Some("files-file") {
        files::OPEN_FIRST.with(|f| f.set(true));
    }
    // `--page taf-receipt:<key>:<entry>`: that Activity entry's receipt over its taf's page; and
    // `taf-report:…`, "Report this payment" over the receipt.
    if let Some(page) = o.page.as_deref() {
        let rest = page
            .strip_prefix("taf-receipt:")
            .or_else(|| page.strip_prefix("taf-receipt-shot:"))
            .or_else(|| page.strip_prefix("taf-report:"));
        // `taf-receipt-shot:…`: the receipt, and its first Chrome step's picture over it.
        if page.starts_with("taf-receipt-shot:") {
            chrome_shots::OPEN_FIRST.with(|o| o.set(true));
        }
        if let Some((_, entry)) = rest.and_then(|r| r.split_once(':')) {
            taf::open_receipt_on_draw(entry);
            if page.starts_with("taf-report:") {
                receipt_sheet::open_report_next();
            }
        }
    }
    // `--page share:<key>` (and `share-step2:<key>`): Share's sheet over that taf's page.
    if let Some(page) = o.page.as_deref() {
        if let Some(key) = page
            .strip_prefix("share:")
            .or_else(|| page.strip_prefix("share-step2:"))
        {
            share::OPEN.with(|s| *s.borrow_mut() = Some(key.to_string()));
            share::OPEN_STEP2.with(|s| s.set(page.starts_with("share-step2:")));
        }
    }
    // `--versions` (and `--trial`): a taf's versions read from files; `--page versions:<key>` or
    // `trial:<key>` opens its sheet over that taf's page.
    if let Some(path) = &o.versions {
        versions::FILES
            .with(|f| *f.borrow_mut() = Some((path.into(), o.trial.as_ref().map(Into::into))));
    }
    match o.page.as_deref() {
        Some(p) if p.starts_with("versions:") => {
            versions::OPEN.with(|v| *v.borrow_mut() = Some("versions"))
        }
        Some(p) if p.starts_with("trial:") => {
            versions::OPEN.with(|v| *v.borrow_mut() = Some("trial"))
        }
        Some(p) if p.starts_with("try:") => versions::OPEN.with(|v| *v.borrow_mut() = Some("try")),
        _ => {}
    }
    if let Some(path) = &o.trial_draft {
        versions::DRAFT.with(|d| *d.borrow_mut() = Some(path.into()));
    }
    // `--page needs-you-hold:<id>`: as a hold's notification opens it, that hold brought into view.
    if let Some(id) = o
        .page
        .as_deref()
        .and_then(|p| p.strip_prefix("needs-you-hold:"))
    {
        needs_you::request_hold(id);
    }
    if let Some(path) = &o.attach {
        attach::SEED.with(|s| *s.borrow_mut() = Some(path.into()));
    }
    if o.page.as_deref() == Some("delete-account") {
        kiff_account::OPEN_DELETE.with(|d| d.set(true));
    }
    match o.page.as_deref() {
        Some("people-rename") => people::OPEN.with(|p| p.set(Some("rename"))),
        Some(
            open @ ("connect-app" | "connect-connection" | "connect-remove" | "connect-which-taf"
            | "connect-approve"),
        ) => {
            let open: &'static str = match open {
                "connect-app" => "connect-app",
                "connect-connection" => "connect-connection",
                "connect-remove" => "connect-remove",
                "connect-which-taf" => "connect-which-taf",
                _ => "connect-approve",
            };
            connect::OPEN.with(|c| c.set(Some(open)));
        }
        Some("people-add") => people::OPEN.with(|p| p.set(Some("add"))),
        Some("devices-sign-out") => devices::OPEN_SIGN_OUT.with(|d| d.set(true)),
        Some("time-zone-use") => zones::TAP_USE_HERE.with(|t| t.set(true)),
        Some("usage-ask") => usage::OPEN.with(|u| u.set(Some("ask"))),
        Some(p) if p.starts_with("halt-resume:") => halt::RESUME.with(|r| r.set(true)),
        Some("discard-setup") => shell::DISCARD.with(|d| d.set(true)),
        Some("needs-you-set-all-aside") => needs_you::OPEN.with(|a| a.set(Some("set-all-aside"))),
        // As a computer paired with a security key sees a held call: Approve once on it. Nothing
        // is signed in the screenshot: pressing it would still need that session.
        Some("needs-you-key" | "needs-you-key-room") => needs_you::KEY_SHOWN.with(|k| k.set(true)),
        Some("needs-you-alerts") => needs_you::ALERTS_SHOWN.with(|s| s.set(true)),
        Some("needs-you-dismiss") => needs_you::OPEN.with(|a| a.set(Some("dismiss"))),
        Some("coach-tools") => coach::OPEN_TOOLS.with(|t| t.set(true)),
        Some("coach-full") => coach_mode::START_FULL.with(|f| f.set(true)),
        Some("coach-read-back") => chat::READ_BACK.with(|r| r.set(true)),
        Some("usage-decline") => usage::OPEN.with(|u| u.set(Some("decline-tapped"))),
        _ => {}
    }
    // A hold's notification, clicked (`app.open-hold`): Needs you, read again, and the hold
    // brought into view once it's listed. A screenshot
    // sends none.
    if !screenshot {
        let action = gio::SimpleAction::new("open-hold", Some(glib::VariantTy::STRING));
        let me = Rc::downgrade(&app_state);
        action.connect_activate(move |_, hold| {
            if let Some(me) = me.upgrade() {
                if let Some(id) = hold.and_then(|h| h.get::<String>()) {
                    needs_you::request_hold(&id);
                }
                me.open_hold();
            }
        });
        app.add_action(&action);
    }

    // What opens: the sample when asked, else the kept session, else the welcome. A screenshot
    // never reads or writes the keyring.
    let now = chrono::Utc::now().timestamp();
    // "A new version of tafs is out": `--new-version`'s release, else GitHub's, read at start and
    // once a day by the app itself, never by a screenshot, the sample or a fixture.
    match &o.new_version {
        Some(tag) => {
            let page = format!("{}/tag/{tag}", tafs_core::update::RELEASES);
            let answer = serde_json::json!({
                "tag_name": tag, "draft": false, "prerelease": false, "html_url": page,
            });
            if let Some(new) = tafs_core::update::newer(tafs_core::update::VERSION, &answer) {
                update::found(new);
            }
        }
        None if !screenshot && !o.sample && o.home.is_none() => update::start(),
        None => {}
    }
    let kept = if screenshot || o.sample {
        None
    } else {
        store::load(now)
    };
    let from_file = o.home.as_deref().map(|path| match read_contract(path) {
        Some(home) => Rc::new(Cloud {
            home,
            held: held_file(o.held.as_deref()),
            // A screenshot reads no connection from planque.
            grants: vec![],
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
            coach_setup: o.coach_setup.as_deref().map(|path| {
                std::fs::read(path)
                    .ok()
                    .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
                    .and_then(|v| serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok())
                    .and_then(|b| tafs_core::coach::setup_id(&b).ok().flatten())
                    .unwrap_or_else(|| {
                        eprintln!("{path} isn't Coach as tafs cloud answers GET /v1/setup");
                        std::process::exit(2);
                    })
            }),
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
    // `--page coach-improve:<key>`: Coach on a new conversation on that taf (`improveTaf`).
    if let (Some(key), Some(c)) = (
        o.page
            .as_deref()
            .and_then(|p| p.strip_prefix("coach-improve:")),
        &from_file,
    ) {
        if let Some(t) = c.home.tafs.iter().find(|t| t.id.as_deref() == Some(key)) {
            coach_start::ask(coach_start::Start::Improve(key.into(), t.name.clone()));
        }
    }
    match (o.page.as_deref(), kept) {
        _ if o.sample => app_state.open_shell(Mode::Sample, false, None),
        _ if from_file.is_some() => {
            app_state.open_shell(Mode::Kiff, o.view_only, from_file.clone());
            // `--page connected`: the celebration after signing in, over the shell.
            if o.page.as_deref() == Some("connected") {
                connected::present(&app_state.window, &app_state.words);
            }
            // `--page cards:<1|2|3>`: one of the three cards on how tafs works, over the shell.
            if let Some(n) = o.page.as_deref().and_then(|p| p.strip_prefix("cards:")) {
                let page = match n {
                    "2" => onboarding::Page::Card,
                    "3" => onboarding::Page::Control,
                    _ => onboarding::Page::Tafs,
                };
                onboarding::present(&app_state.window, app_state.words.clone(), page, false);
            }
            // `--page how-it-works`: How tafs works, from Account.
            if o.page.as_deref() == Some("how-it-works") {
                onboarding::how_it_works(&app_state.window, app_state.words.clone());
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
            // `--page gallery`: the jobs' gallery, browsing, for its screenshot; `new-taf`, as
            // + New opens it.
            if let Some(mode) = match o.page.as_deref() {
                Some("gallery") => Some(recipes::GalleryMode::Browse),
                Some("new-taf") => Some(recipes::GalleryMode::Create),
                _ => None,
            } {
                if let Some(g) = from_file.as_deref().and_then(|c| c.gallery.clone()) {
                    let pane = recipes::GalleryPane {
                        labels: from_file
                            .as_deref()
                            .and_then(|c| c.recipes.as_ref())
                            .and_then(|r| r.labels.clone()),
                        gallery: g,
                        may_use_coach: true,
                        close: app_state.words.line("common.close").unwrap_or_default(),
                    };
                    recipes::gallery(&pane, mode, &window);
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
        (
            Some("welcome")
            | Some("welcome-ticked")
            | Some("welcome-terms-changed")
            | Some("sign-in")
            | Some("sign-in-code")
            | Some("pair"),
            _,
        )
        | (_, None) => {
            app_state.show_welcome();
            match o.page.as_deref() {
                Some("welcome-ticked") => welcome::show_as(true, false),
                Some("welcome-terms-changed") => welcome::show_as(false, true),
                _ => {}
            }
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
    if matches!(o.page.as_deref(), Some("pin-sheet") | Some("pin-sheet-add")) {
        use tafs_core::security_key::Failure;
        // `pin-sheet-add`: the same sheet adding the key when the computer is paired.
        let w = app_state.words.clone();
        let attempt: pin_sheet::Attempt =
            std::sync::Arc::new(|_| Err(fido_key::KeyError::Key(Failure::NoKey)));
        let tries: pin_sheet::ReadTries = std::sync::Arc::new(|| Ok(5));
        let stop: pin_sheet::StopKey = std::sync::Arc::new(|| {});
        if o.page.as_deref() == Some("pin-sheet-add") {
            pin_sheet::present_add(w, &window, attempt, tries, stop, Rc::new(|_| {}));
        } else {
            // The sample's act, named as its prompt: signing out the sample's other device.
            let act = w
                .get("settings.devices.sign_out_prompt", &[("device", "iPad")])
                .ok();
            pin_sheet::present(
                w,
                &window,
                act.as_deref(),
                attempt,
                tries,
                stop,
                Rc::new(|_| {}),
            );
        }
    }
    // `--app-event`: an app event's filters before signing (`AppEventApprovalSheet`), signed in,
    // for its screenshot: its Approve is the gate's line, so nothing is ever signed.
    if let Some(item) = o
        .app_event
        .as_deref()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| tafs_core::app_event::Item::parse(&b))
    {
        app_event_sheet::present(&app_state.words, Mode::Kiff, &item, &window);
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
    /// The window's shell once drawn: what's open in it, and its Unreachable line.
    shell: RefCell<Option<shell::Shell>>,
    /// Home couldn't be read (`unreachable`): why, and how many times the app has tried again by
    /// itself (`retries`); none once a read goes through.
    unreachable: Cell<Option<(tafs_core::sidebar::Unreachable, usize)>>,
    /// The account's reads in force: a sign-out or another sign-in ends the app's tries.
    account_reads: Cell<u64>,
    /// Each in-place reread of Home, numbered: only the newest's answer is drawn, so two that
    /// overlap (a stream's end, then a held call) can't leave the older one on screen.
    home_rereads: Cell<u64>,
    /// A pull of the sidebar is on its way: another asks nothing more. Any read of Home that
    /// ends lets the next pull through, so it can't stay stuck.
    home_pulling: Cell<bool>,
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
    /// `--knob`'s dial: Setup's screens read it, and their changes send nothing.
    knob_file: Option<std::path::PathBuf>,
    /// `--setup`'s Setup read, in the contract's place beside `--home`.
    setup_file: Option<std::path::PathBuf>,
    /// `--taf-activity`'s page, in the contract's place beside `--home`.
    taf_activity_file: Option<std::path::PathBuf>,
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
    /// `--coach-drafts`: KIFF's answers for the drafts, never read.
    coach_drafts: Option<std::path::PathBuf>,
    /// `--coach-setup`: Coach's tools from tafs cloud's answer, never read.
    coach_setup: Option<std::path::PathBuf>,
    /// `--coach-stream`: an answer streaming in that conversation, from a stream's events.
    coach_stream: Option<std::path::PathBuf>,
    /// `--chat`: a taf's chat from a file, never sent.
    chat_file: Option<std::path::PathBuf>,
    /// `--chat-stream`: an answer streaming in that chat, from a stream's events.
    chat_stream: Option<std::path::PathBuf>,
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
    files_file: Option<std::path::PathBuf>,
    chromes_file: Option<std::path::PathBuf>,
    /// `--connect`'s file of synthetic answers: Connect apps' first page, and a start.
    connect_file: Option<std::path::PathBuf>,
    /// A screenshot: the sample's Refunds has the reference set's three Cards.
    screenshot: bool,
    /// `--page taf-card`: the taf opens on its Card tab.
    on_card: bool,
    /// `--page taf-sample-chat`: the sample's Support replies opens on its chat.
    on_sample_chat: bool,
    /// `--page taf-setup:<id>`: a taf's page opens on Setup.
    on_setup: bool,
    /// `--page starts:<id>` or `results:<id>`: Setup's screen open over its tab.
    on_setup_screen: Option<starts_when::Screen>,
    /// `--page doors:<id>[:<door>]`: a taf's doors open over its page (`DoorsView`), or one door's
    /// screen, for a screenshot.
    on_doors: Option<Option<String>>,
    /// `--page calendar`: the calendar opens on Day, as the reference set's `calendar.png`.
    calendar_day: bool,
}

/// What reading the account came to.
enum Read {
    Cloud(Box<Cloud>),
    /// KIFF answered 401: the session is over.
    SignedOut,
    /// Home couldn't be read: the screen stays as it was, and the sidebar says why.
    Unreachable(tafs_core::sidebar::Unreachable),
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
        coach_start::forget();
        // And the app's tries at reading the last account.
        self.account_reads.set(self.account_reads.get() + 1);
        self.unreachable.set(None);
        // And nothing written to a taf.
        taf_chat::forget();
        crate::doors::forget();
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
        // The welcome scrolls in a short window: the tick and the phone's way stay reachable.
        let scroller = gtk::ScrolledWindow::new();
        scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        scroller.set_child(Some(&welcome.root));
        crate::widgets::ground(&scroller);
        self.window.set_content(Some(&scroller));
        *self.problem.borrow_mut() = Some(welcome.problem.clone());
        // The terms version in force, read as the tick shows: the one sent with the sign-in is
        // the one the person saw. Read again when KIFF says it changed.
        let terms: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
        let read_terms = {
            let (terms, languages) = (terms.clone(), self.languages.clone());
            Rc::new(move || {
                let (terms, languages) = (terms.clone(), languages.clone());
                glib::spawn_future_local(async move {
                    *terms.borrow_mut() = app_sign_in::terms_version(languages).await;
                });
            })
        };
        if !self.screenshot {
            read_terms();
        }
        let ui = Rc::new(welcome);
        // How Continue ends when it doesn't sign in (`continueOutcome`).
        let ended: Rc<dyn Fn(app_sign_in::Outcome)> = {
            let (ui, w, read_terms) = (ui.clone(), self.words.clone(), read_terms.clone());
            Rc::new(move |outcome| {
                ui.sign_in.set_sensitive(ui.tick.is_active());
                let say = |line: Option<String>| {
                    ui.problem.set_visible(line.is_some());
                    ui.problem.set_text(&line.unwrap_or_default());
                };
                match outcome {
                    // Declined on KIFF's page: nothing is said.
                    app_sign_in::Outcome::Declined => say(None),
                    app_sign_in::Outcome::NotOn => say(w.line("sign_in.in_app_not_on").ok()),
                    // Unticked, why, and the new version for the new tick.
                    app_sign_in::Outcome::TermsChanged => {
                        say(None);
                        ui.tick.set_active(false);
                        ui.terms_changed.set_visible(true);
                        read_terms();
                    }
                    app_sign_in::Outcome::Failed(_) => say(w.line("sign_in.didnt_work").ok()),
                }
            })
        };
        let me = self.clone();
        let signed_in: Rc<dyn Fn(KeptSession)> = Rc::new(move |kept| me.connected(kept));
        let ends = Rc::new(app_sign_in::Ends {
            signed_in,
            ended: ended.clone(),
        });
        {
            let (me, ui, terms) = (self.clone(), ui.clone(), terms.clone());
            ui.clone().sign_in.connect_clicked(move |_| {
                ui.problem.set_visible(false);
                ui.terms_changed.set_visible(false);
                // No version, no sign-in: consent is never skipped.
                let Some(version) = terms.borrow().clone() else {
                    ui.problem
                        .set_text(&me.words.line("sign_in.didnt_work").unwrap_or_default());
                    ui.problem.set_visible(true);
                    return;
                };
                app_sign_in::begin(
                    me.window.upcast_ref(),
                    me.words.clone(),
                    me.languages.clone(),
                    version,
                    ends.clone(),
                );
            });
        }
        let me = self.clone();
        ui.sample.connect_clicked(move |_| {
            app_sign_in::forget();
            me.open_shell(Mode::Sample, false, None)
        });
        {
            // "Sign in with your phone": its claim carries the version the tick agreed to; terms
            // that changed close it and untick the box, as Continue does.
            let (me, ui, terms) = (self.clone(), ui.clone(), terms.clone());
            ui.clone().pair.connect_clicked(move |_| {
                let Some(version) = terms.borrow().clone() else {
                    ui.problem
                        .set_text(&me.words.line("sign_in.didnt_work").unwrap_or_default());
                    ui.problem.set_visible(true);
                    return;
                };
                let ended = ended.clone();
                pairing::after_tick(
                    version,
                    Rc::new(move || ended(app_sign_in::Outcome::TermsChanged)),
                );
                me.present_pairing(false);
            });
        }
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
        // Signed in, whichever way: a link back from another attempt is never used.
        app_sign_in::forget();
        self.open_account(kept);
        // The three cards follow the celebration, once on this computer.
        let (window, words) = (self.window.clone(), self.words.clone());
        connected::present_then(&self.window, &self.words, move || {
            if tafs_core::onboarding::shows_cards(onboarding::seen()) {
                onboarding::present(&window, words.clone(), onboarding::Page::Tafs, true);
            }
        });
    }

    /// Signed in: the shell at once, then the account (`GET /v1/me`), Home and the held calls,
    /// drawn again once read. An answer that can't be read leaves it as it was.
    fn open_account(self: &Rc<Self>, kept: KeptSession) {
        // A new account's consent is its own: Not now from the last one doesn't carry over.
        ai_consent::forget();
        let view_only = kept.approval_mode.is_view_only();
        *self.kept.borrow_mut() = Some(kept.clone());
        self.unreachable.set(None);
        self.open_shell(Mode::Kiff, view_only, None);
        self.read_account(kept, view_only);
    }

    /// The account's reads (`refresh`): the account, Home and the held calls, then the shell drawn
    /// again with them. When Home can't be read, the sidebar says so quietly (`UnreachableLine`)
    /// and the app tries again by itself, then less often, and stops after a few tries.
    fn read_account(self: &Rc<Self>, kept: KeptSession, view_only: bool) {
        let generation = self.account_reads.get() + 1;
        self.account_reads.set(generation);
        let (me, languages, reading) = (self.clone(), self.languages.clone(), kept.clone());
        let words: WordBook = (*self.words).clone();
        glib::spawn_future_local(async move {
            let read = gio::spawn_blocking(move || read_cloud(&reading, &languages, &words))
                .await
                .unwrap_or(Read::Unreachable(tafs_core::sidebar::Unreachable::Error));
            me.home_pulling.set(false);
            // Signed out, or another account's reads, since: this one's answer is dropped.
            if me.account_reads.get() != generation {
                return;
            }
            match read {
                Read::Cloud(cloud) => {
                    // Read: nothing to say, and no try waiting. What was open stays open.
                    if me.unreachable.take().is_some() {
                        let open = me
                            .shell
                            .borrow()
                            .as_ref()
                            .and_then(|s| s.selection.borrow().clone());
                        if let Some(open) = open {
                            *me.first.borrow_mut() = open;
                        }
                    }
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
                Read::Unreachable(why) => me.unreachable_now(why, kept, view_only, generation),
            }
        });
    }

    /// Home couldn't be read (`refreshFailed`): said quietly at the sidebar's foot, and read again
    /// a moment later, then less often, and no more after a few tries.
    fn unreachable_now(
        self: &Rc<Self>,
        why: tafs_core::sidebar::Unreachable,
        kept: KeptSession,
        view_only: bool,
        generation: u64,
    ) {
        use tafs_core::sidebar::RETRY_WAITS;
        let tries = self.unreachable.get().map_or(0, |(_, n)| n);
        let retrying = tries < RETRY_WAITS.len();
        self.unreachable
            .set(Some((why, (tries + 1).min(RETRY_WAITS.len()))));
        // Nothing read yet: the Overview said it was loading, and now says it couldn't load,
        // where the person was.
        if tries == 0 && self.cloud.borrow().is_none() {
            let open = self
                .shell
                .borrow()
                .as_ref()
                .and_then(|s| s.selection.borrow().clone());
            if let Some(open) = open {
                *self.first.borrow_mut() = open;
            }
            self.open_shell(Mode::Kiff, view_only, None);
            return self.try_again_later(tries, kept, view_only, generation);
        }
        if let Some(shell) = self.shell.borrow().as_ref() {
            shell.set_unreachable(
                tafs_core::sidebar::unreachable_line(&self.words, why, retrying)
                    .ok()
                    .as_ref(),
            );
        }
        if retrying {
            self.try_again_later(tries, kept, view_only, generation);
        }
    }

    /// The app's own next try, `RETRY_WAITS[tries]` seconds from now (±20%), unless the account's reads
    /// moved on meanwhile.
    fn try_again_later(
        self: &Rc<Self>,
        tries: usize,
        kept: KeptSession,
        view_only: bool,
        generation: u64,
    ) {
        let Some(wait) = tafs_core::sidebar::RETRY_WAITS.get(tries) else {
            return;
        };
        let me = Rc::downgrade(self);
        // A random ±20%, so a blip doesn't bring everyone back in the same second.
        let wait =
            tafs_core::follow::jitter(std::time::Duration::from_secs(*wait), glib::random_double());
        glib::timeout_add_local_once(wait, move || {
            if let Some(me) = me.upgrade() {
                if me.account_reads.get() == generation {
                    me.read_account(kept, view_only);
                }
            }
        });
    }

    /// Each taf's page: the sample's tafs with their Cards and states, else Home's.
    fn taf_pages(
        &self,
        mode: Mode,
        model: &SidebarModel,
        cloud: Option<&Cloud>,
        (may_answer, manages_cards): (bool, bool),
        at: chrono::DateTime<chrono::Local>,
        signed_out: Rc<dyn Fn()>,
    ) -> Vec<taf::TafPage> {
        let w = &self.words;
        // "Allow this from now on" is live where KIFF names `holds.allow` for the account.
        let allow_live = cloud
            .and_then(|c| c.account.as_ref())
            .is_some_and(|a| tafs_core::allow::is_live(a.capabilities.as_deref()));
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
        // The sample's Record, for what each sample taf says it did lately in its chat.
        let sample_record = if mode == Mode::Sample {
            sample::record(w, at).unwrap_or_default()
        } else {
            vec![]
        };
        let held_of = |key: &str| -> Vec<tafs_core::needs_you::CardLines> {
            let calls: Vec<_> = model
                .held
                .iter()
                .filter(|c| c.taf == key)
                .cloned()
                .collect();
            tafs_core::needs_you::lines(
                w,
                &calls,
                &holders,
                may_answer,
                manages_cards,
                allow_live,
                at,
            )
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
            (None, Some(home)) => taf::ActivitySource::File(
                self.taf_activity_file
                    .clone()
                    .unwrap_or_else(|| home.with_file_name("taf-activity.json")),
            ),
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
                setup: self
                    .setup_file
                    .clone()
                    .unwrap_or_else(|| home.with_file_name("taf-setup.json")),
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
        // Where Setup's screens read and change its dial: tafs cloud as the session; a
        // screenshot's from `--knob`, whose changes send nothing.
        let knob = match (self.kept.borrow().as_ref(), &self.knob_file) {
            (_, Some(file)) => starts_when::KnobSource::File(file.clone()),
            (Some(kept), None) => starts_when::KnobSource::Cloud {
                token: kept.token.clone(),
                languages: self.languages.clone(),
            },
            _ => starts_when::KnobSource::None,
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
                stream: self.chat_stream.clone(),
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
                    let paused = taf_chat::sample_paused(&t.key) == Some(true);
                    // Its Record lines' titles, newest first, for what it says it did lately.
                    let mut lines: Vec<&tafs_core::record::Entry> =
                        sample_record.iter().filter(|e| e.taf == t.key).collect();
                    lines.sort_by_key(|e| std::cmp::Reverse(e.at));
                    taf::TafPage {
                        kiff_agent: String::new(),
                        key: t.key.clone(),
                        name: t.name.clone(),
                        initial: t.initial.clone(),
                        color: t.color_index,
                        rim: None,
                        state: if paused {
                            tafs_core::taf_page::TafState::Paused
                        } else if waits {
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
                        held_reread: None,
                        sample: true,
                        manages: false,
                        cards: sample::full_cards(w, &t.key, self.screenshot).unwrap_or_default(),
                        issued_by: w.line("common.you").unwrap_or_default(),
                        opens_on_card: self.on_card,
                        opens_on_activity: self.screenshot && !self.on_sample_chat,
                        opens_on_setup: self.on_setup,
                        opens_doors: None,
                        opens_setup_screen: None,
                        my_email: None,
                        view_only: false,
                        card_ids: vec![],
                        knob: starts_when::KnobSource::None,
                        works_from_chat: false,
                        next_run_line: None,
                        paused,
                        results: None,
                        cloud_id: None,
                        source: taf::ActivitySource::None,
                        work: work_panel::WorkSource::None,
                        job: String::new(),
                        setup: setup_pane::SetupSource::None,
                        doors: knocks::DoorsSource::None,
                        card_units: vec![],
                        steps: vec![],
                        may_use_coach: false,
                        health: None,
                        runs: runs_panel::RunsSource::None,
                        runs_line: None,
                        time_zone_words: None,
                        schedule_time_zone_words: None,
                        // The sample's chat (`sampleReply`), at the iPad's pace: a beat is half
                        // of its 0.7 s, none in a screenshot.
                        chat: taf_chat::ChatSource::Sample {
                            now_line: t.now.clone(),
                            done: lines.iter().map(|e| e.title.clone()).collect(),
                            waiting: model.held.iter().filter(|c| c.taf == t.key).count(),
                            beat_ms: if self.screenshot {
                                0
                            } else {
                                tafs_core::sample_chat::REPLY_MS / 2
                            },
                        },
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
                        (
                            h.unscheduled == Some(true),
                            tafs_core::knob::next_run_line(
                                w,
                                h,
                                c.account.as_ref().and_then(|a| a.time_zone.as_deref()),
                            )
                            .ok()
                            .flatten(),
                            h.status.as_deref() == Some("paused"),
                            h.results.clone(),
                        ),
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
                        kiff_agent: t.agent.clone(),
                        time_zone_words: zones.0,
                        schedule_time_zone_words: zones.1,
                        job: zones.2,
                        knob: knob.clone(),
                        works_from_chat: zones.3 .0,
                        next_run_line: zones.3 .1,
                        paused: zones.3 .2,
                        results: zones.3 .3,
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
                        steps: c
                            .home
                            .your_steps
                            .iter()
                            .flatten()
                            .filter(|s| {
                                t.cloud && s.taf.as_ref().is_some_and(|(id, _)| *id == t.key)
                            })
                            .cloned()
                            .collect(),
                        may_use_coach: model.may_use_coach,
                        health,
                        runs: runs.clone(),
                        runs_line,
                        held: held_of(&t.key),
                        // Read again from KIFF in its chat: a tafs cloud taf's, by its agent.
                        held_reread: t.cloud.then(|| crate::taf_chat::HeldReread {
                            agent: t.agent.clone(),
                            key: t.key.clone(),
                            name: t.name.clone(),
                            initial: tafs_core::sidebar::initial(&t.name),
                            color: t.color_index,
                            may_answer,
                            manages_cards,
                            allow_live,
                        }),
                        initial: tafs_core::sidebar::initial(&t.name),
                        cloud_id: t.cloud.then(|| t.key.clone()),
                        rim: t.rim,
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
                        opens_on_activity: false,
                        opens_on_setup: self.on_setup,
                        opens_doors: self.on_doors.clone(),
                        opens_setup_screen: self.on_setup_screen,
                        my_email: c.account.as_ref().and_then(|a| a.email.clone()),
                        // KIFF's word for how the session approves, else the kept session's.
                        view_only: match c.account.as_ref().and_then(|a| a.approval_mode.as_deref())
                        {
                            Some(word) => ApprovalMode::from_kiff(Some(word)).is_view_only(),
                            None => self
                                .kept
                                .borrow()
                                .as_ref()
                                .is_some_and(|k| k.approval_mode.is_view_only()),
                        },
                        card_ids: t.card_ids.clone(),
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
    /// KIFF's activity as the Record's lines (`KIFFMapping.record`), each by its taf's key in the
    /// Record: tafs cloud's id, else `agent:` and its holder in KIFF.
    fn cloud_record(&self, c: &Cloud) -> Vec<tafs_core::record::Entry> {
        let agents: Vec<(String, String)> = c
            .home
            .tafs
            .iter()
            .map(|t| {
                (
                    t.kiff_agent.clone(),
                    tafs_core::connections::Using::key_of(t),
                )
            })
            .collect();
        let taf_of = |actor: &str| {
            agents
                .iter()
                .find(|(a, _)| a == actor)
                .map(|(_, k)| k.clone())
        };
        // `toolVendors`: the vendor of each connected tool, from the account's connections,
        // which the Record doesn't read from planque: none, so a call a vendor refused says
        // "The app", as the iPad's does without them.
        let vendors = tafs_core::connections::tool_vendors(&[]);
        tafs_core::record::naming(
            tafs_core::record::record(&self.words, &c.activity, &taf_of, &vendors)
                .unwrap_or_default(),
            &c.call_refs,
        )
    }

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
            (Mode::Sample, _) => ["support", "parcels", "access", "deploys", "leads"]
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
            (_, Some(c)) => self.cloud_record(c),
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
                // A random ±20%, so computers don't read KIFF in step (docs/scale.md).
                glib::timeout_future(tafs_core::follow::jitter(
                    hold_alert::PERIOD,
                    glib::random_double(),
                ))
                .await;
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
    /// The sidebar's model from what was read (`HomeSplitView`'s), with the Overview's tafs and
    /// tafs cloud's words for Needs you: for the window, and for the sidebar drawn again in place.
    fn sidebar_model(
        &self,
        mode: Mode,
        cloud: Option<&Rc<Cloud>>,
        at: chrono::DateTime<chrono::Local>,
    ) -> (
        SidebarModel,
        Vec<tafs_core::overview::OverviewTaf>,
        Option<String>,
    ) {
        let words = &self.words;
        let fail = |e: tafs_core::words::WordsError| -> ! { panic!("{e}") };
        let (mut model, tafs, cloud_words) = match (mode, cloud) {
            (Mode::Sample, _) => (
                sample::sidebar(words, at, self.signed_in).unwrap_or_else(|e| fail(e)),
                sample::overview(words, at).unwrap_or_else(|e| fail(e)),
                None,
            ),
            (_, Some(c)) => {
                let (model, mut tafs) = home_model(words, c, at).unwrap_or_else(|e| fail(e));
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
        // A sample taf paused or told to carry on in its chat says so (`togglePause`).
        if mode == Mode::Sample {
            for (row, _) in model.tafs.iter_mut() {
                let key = match taf_chat::sample_paused(&row.key) {
                    Some(true) => "taf.now.paused_by_you",
                    Some(false) => "taf.now.back_to_work",
                    None => continue,
                };
                row.now = words.line(key).unwrap_or_default();
            }
        }
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
        if let (Mode::Kiff, Some(c)) = (mode, cloud.map(|c| &**c)) {
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
        // `canFinishSetup`: the rows whose setup stopped partway that Coach can finish, by key.
        if let (Mode::Kiff, Some(c)) = (mode, cloud.map(|c| &**c)) {
            let coach = c.coach_setup.as_deref();
            let may = model.may_use_coach;
            let setting_up = c
                .home
                .setting_up()
                .into_iter()
                .filter_map(|s| Some((s.key(), s.chat.clone()?)));
            let others = c
                .home
                .other_agents()
                .iter()
                .filter_map(|t| Some((t.key(), t.finish_chat()?)));
            model.finish = setting_up
                .chain(others)
                .filter(|(_, f)| f.can_finish(true, may, coach))
                .map(|(key, f)| (key, f.session))
                .collect();
        }
        // KIFF couldn't be reached at the last read, and whether the app tries again.
        if mode == Mode::Kiff {
            model.unreachable = self
                .unreachable
                .get()
                .map(|(why, tries)| (why, tries < tafs_core::sidebar::RETRY_WAITS.len()));
        }
        (model, tafs, cloud_words)
    }

    /// `refresh`: Home and what goes with it read again as the session, and the sidebar drawn
    /// from it in place; the screen beside it stays as it is. A read that fails changes nothing.
    fn reread_home(self: &Rc<Self>) {
        self.reread_home_as(false);
    }

    /// The sidebar pulled (`refresh()`, not the app's own `retry`): Home read again in place,
    /// and the tries started over. While Home can't be read, the account's reads again from the
    /// first try (an older try waiting is dropped, as their generation moves on); else, read in
    /// place, and a failure says so at the sidebar's foot and tries again, as `refreshFailed`.
    fn pull_home(self: &Rc<Self>) {
        let Some(kept) = self.kept.borrow().clone() else {
            return;
        };
        if self.home_pulling.get() {
            return;
        }
        if let Some((why, _)) = self.unreachable.get() {
            self.home_pulling.set(true);
            self.unreachable.set(Some((why, 0)));
            let view_only = kept.approval_mode.is_view_only();
            return self.read_account(kept, view_only);
        }
        if self.cloud.borrow().is_some() {
            self.home_pulling.set(true);
            self.reread_home_as(true);
        }
    }

    fn reread_home_as(self: &Rc<Self>, pulled: bool) {
        let Some(kept) = self.kept.borrow().clone() else {
            return;
        };
        if self.cloud.borrow().is_none() {
            return;
        }
        let generation = self.account_reads.get();
        let reread = self.home_rereads.get() + 1;
        self.home_rereads.set(reread);
        let (me, languages, reading) = (self.clone(), self.languages.clone(), kept.clone());
        let words: WordBook = (*self.words).clone();
        glib::spawn_future_local(async move {
            let read = gio::spawn_blocking(move || read_cloud(&reading, &languages, &words))
                .await
                .unwrap_or(Read::Unreachable(tafs_core::sidebar::Unreachable::Error));
            me.home_pulling.set(false);
            // Another account's, or an older reread's answer, since: dropped.
            if me.account_reads.get() != generation || me.home_rereads.get() != reread {
                return;
            }
            match read {
                Read::Cloud(cloud) => {
                    let cloud = Rc::new(*cloud);
                    *me.cloud.borrow_mut() = Some(cloud.clone());
                    let (model, _, _) =
                        me.sidebar_model(Mode::Kiff, Some(&cloud), chrono::Local::now());
                    if let Some(shell) = me.shell.borrow().as_ref() {
                        shell.redraw_sidebar(&model);
                    }
                }
                Read::SignedOut => me.signed_out_by_kiff(),
                Read::Unreachable(why) if pulled => {
                    let view_only = kept.approval_mode.is_view_only();
                    let generation = me.account_reads.get();
                    me.unreachable_now(why, kept, view_only, generation);
                }
                Read::Unreachable(_) => {}
            }
        });
    }

    fn open_shell(self: &Rc<Self>, mode: Mode, view_only: bool, cloud: Option<Rc<Cloud>>) {
        let words = &self.words;
        // Signed in with KIFF, a chat's stream ending reads Home again for the sidebar.
        crate::shell::set_home_reread(match (mode, self.kept.borrow().is_some()) {
            (Mode::Kiff, true) => {
                let me = Rc::downgrade(self);
                Some(Rc::new(move || {
                    if let Some(me) = me.upgrade() {
                        me.reread_home();
                    }
                }))
            }
            // The sample reads nothing: its sidebar is drawn again from what its chats changed.
            (Mode::Sample, _) => {
                let me = Rc::downgrade(self);
                Some(Rc::new(move || {
                    if let Some(me) = me.upgrade() {
                        let (model, _, _) =
                            me.sidebar_model(Mode::Sample, None, chrono::Local::now());
                        if let Some(shell) = me.shell.borrow().as_ref() {
                            shell.redraw_sidebar(&model);
                        }
                    }
                }))
            }
            _ => None,
        });
        // And a pull of the sidebar reads it again, the tries started over.
        crate::shell::set_home_pull(match (mode, self.kept.borrow().is_some()) {
            (Mode::Kiff, true) => {
                let me = Rc::downgrade(self);
                Some(Rc::new(move || {
                    if let Some(me) = me.upgrade() {
                        me.pull_home();
                    }
                }))
            }
            _ => None,
        });
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
        let (model, tafs, cloud_words) = self.sidebar_model(mode, cloud.as_ref(), at);
        let account = cloud.as_ref().and_then(|c| c.account.clone());
        let count = model.needs_you_count();
        let needs_you_words = match cloud_words {
            Some(words) => words,
            None => tafs_core::overview::needs_you_words(words, count).unwrap_or_else(|e| fail(e)),
        };
        // Signed in, the list is being read until Home comes, or couldn't be read.
        let list = match (mode, &cloud, self.unreachable.get()) {
            (Mode::Kiff, None, Some(_)) => tafs_core::overview::ListState::Unread,
            (Mode::Kiff, None, None) => tafs_core::overview::ListState::Loading,
            _ => tafs_core::overview::ListState::Read,
        };
        let overview =
            tafs_core::overview::lines(words, &tafs, &model.held, count, &needs_you_words)
                .and_then(|o| o.with_list(words, list))
                .and_then(|o| {
                    if mode == Mode::Kiff {
                        o.with_first_home(words, model.may_use_coach, model.home_words.as_deref())
                    } else {
                        Ok(o)
                    }
                })
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
        let manages_cards = account.as_ref().is_none_or(|a| a.allows("cards.manage"));
        let allow_live = account
            .as_ref()
            .is_some_and(|a| tafs_core::allow::is_live(a.capabilities.as_deref()));
        let needs_you = tafs_core::needs_you::lines(
            words,
            &model.held,
            &holders,
            may_answer,
            manages_cards,
            allow_live,
            at,
        )
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
        let (terms, privacy) = (
            tafs_core::account::terms_url(words.language()),
            tafs_core::account::privacy_url(words.language()),
        );
        let account_lines = tafs_core::account::lines(
            words,
            mode,
            account.as_ref(),
            self.sign_out_error.borrow().as_deref(),
            &terms,
            &privacy,
            env!("CARGO_PKG_VERSION"),
        )
        .and_then(|l| {
            // `PlanRow`: tafs cloud's plan line, once the balance is read.
            tafs_core::account::with_plan(
                l,
                words,
                cloud.as_deref().and_then(|c| c.balance.as_ref()),
            )
        })
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
        // Coach's waiting chats set aside at tafs cloud, from a kept session; then Home again.
        let set_aside = match (mode, self.kept.borrow().as_ref()) {
            (Mode::Kiff, Some(kept)) => Some(Rc::new(needs_you::SetAsideSource {
                kept: kept.clone(),
                languages: self.languages.clone(),
                refresh: {
                    let (me, kept) = (Rc::downgrade(self), kept.clone());
                    Rc::new(move || {
                        if let Some(me) = me.upgrade() {
                            me.read_account(kept.clone(), view_only);
                        }
                    })
                },
                signed_out: signed_out.clone(),
            })),
            _ => None,
        };
        let tafs = Rc::new(self.taf_pages(
            mode,
            &model,
            cloud.as_deref(),
            (may_answer, manages_cards),
            at,
            signed_out,
        ));
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
        // Sharing, signed in with KIFF: read as the session, or a screenshot's from `--people`.
        let share = match (mode, self.kept.borrow().as_ref(), &self.people_file) {
            (Mode::Kiff, Some(kept), _) => Some(share::ShareSource::Cloud {
                token: kept.token.clone(),
                languages: self.languages.clone(),
            }),
            (Mode::Kiff, None, Some(f)) => Some(share::ShareSource::File(f.clone())),
            _ => None,
        };
        let share_agents = Rc::new(
            cloud
                .as_deref()
                .map(|c| {
                    c.home
                        .tafs
                        .iter()
                        .filter_map(|h| tafs_core::home::taf(words, h).ok())
                        .map(|t| (t.key, t.agent))
                        .collect()
                })
                .unwrap_or_default(),
        );
        // Coach's empty chat lists the recipes the account's read had (`model.recipes`).
        crate::coach::set_recipes(match (mode, cloud.as_deref()) {
            (Mode::Kiff, Some(c)) => c.recipes.clone(),
            _ => None,
        });
        let screens = shell::Screens {
            set_aside,
            share,
            share_agents,
            account_email: account.as_ref().and_then(|a| a.email.clone()),
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
                    may_use_coach: model.may_use_coach,
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
                        close: self.words.line("common.close").unwrap_or_default(),
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
            chrome: match (mode, self.kept.borrow().as_ref(), &self.chromes_file) {
                (Mode::Kiff, Some(kept), _) => chrome::ChromeSource::Cloud {
                    kept: kept.clone(),
                    languages: self.languages.clone(),
                },
                (Mode::Kiff, None, Some(f)) => chrome::ChromeSource::File(f.clone()),
                _ => chrome::ChromeSource::None,
            },
            files: match (mode, self.kept.borrow().as_ref(), &self.files_file) {
                (Mode::Kiff, Some(kept), _) => files::FilesSource::Cloud {
                    token: kept.token.clone(),
                    languages: self.languages.clone(),
                },
                (Mode::Kiff, None, Some(f)) => files::FilesSource::File(f.clone()),
                _ => files::FilesSource::None,
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
                device_key: self.kept.borrow().is_some(),
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
            connect: {
                // `connectableTafs`: on tafs cloud and managed by this person, by tafs cloud's id.
                let connectable: Vec<tafs_core::connect::Connectable> = tafs
                    .iter()
                    .filter(|t| t.manages)
                    .filter_map(|t| {
                        Some(tafs_core::connect::Connectable {
                            id: t.cloud_id.clone()?,
                            name: t.name.clone(),
                        })
                    })
                    .collect();
                // `canManageCards`.
                let manages = account.as_ref().is_none_or(|a| a.allows("cards.manage"));
                match (mode, self.kept.borrow().as_ref(), &self.connect_file) {
                    (Mode::Sample, _, _) => connect::ConnectSource::Sample {
                        screenshot: self.screenshot,
                        // The sample's tafs, which its connections are made for.
                        tafs: tafs
                            .iter()
                            .filter(|t| !t.revoked)
                            .map(|t| (t.key.clone(), t.name.clone()))
                            .collect(),
                    },
                    (Mode::Kiff, _, Some(f)) => connect::ConnectSource::File {
                        path: f.clone(),
                        connectable,
                        manages,
                    },
                    (Mode::Kiff, Some(kept), None) => connect::ConnectSource::Cloud {
                        kept: Box::new(kept.clone()),
                        languages: self.languages.clone(),
                        connectable,
                        manages,
                        // `holderIDs`: each taf by its holder in KIFF, which its connections name.
                        holders: cloud
                            .as_deref()
                            .map(|c| {
                                c.home
                                    .tafs
                                    .iter()
                                    .filter(|t| t.card.active)
                                    .map(|t| (t.kiff_agent.clone(), t.name.clone()))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        using: cloud
                            .as_deref()
                            .map(|c| {
                                c.home
                                    .tafs
                                    .iter()
                                    .filter_map(tafs_core::connections::Using::of)
                                    .collect()
                            })
                            .unwrap_or_default(),
                        record: cloud
                            .as_deref()
                            .map(|c| self.cloud_record(c))
                            .unwrap_or_default(),
                    },
                    _ => connect::ConnectSource::None,
                }
            },
            // Coach, signed in with KIFF: its conversation read as the session with this device's
            // proof, or a screenshot's from `--coach`, never sent. The sample has no Coach.
            coach: match (mode, self.kept.borrow().as_ref(), &self.coach_file) {
                (Mode::Kiff, _, Some(f)) => coach::CoachSource::File {
                    thread: f.clone(),
                    chats: self.coach_chats.clone(),
                    drafts: self.coach_drafts.clone(),
                    setup: self.coach_setup.clone(),
                    stream: self.coach_stream.clone(),
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
        *self.shell.borrow_mut() = Some(shell);
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
/// Everything the window reads at tafs cloud and KIFF as the session (`refresh`): Home, KIFF's held
/// calls and activity, the account, and the rest the screens draw. Blocking: off the main loop.
/// This computer's id in KIFF for Home's grants read, read once per session and kept: each Home
/// read after the first adds only planque's grants read. A session that changed reads it again.
fn home_device(kept: &KeptSession, languages: &[&'static str]) -> Option<String> {
    static DEVICE: std::sync::Mutex<Option<(String, String)>> = std::sync::Mutex::new(None);
    if let Some((token, id)) = DEVICE.lock().ok()?.as_ref() {
        if *token == kept.token {
            return Some(id.clone());
        }
    }
    let id = coach::this_device(kept, languages)?;
    if let Ok(mut d) = DEVICE.lock() {
        *d = Some((kept.token.clone(), id.clone()));
    }
    Some(id)
}

fn read_cloud(kept: &KeptSession, languages: &[&'static str], words: &WordBook) -> Read {
    let token = kept.token.as_str();
    let signed_out = |a: &tafs_core::signin::Answer| a.status == 401;
    let me = kiff::me(token, languages).ok();
    let home = kiff::home(token, languages);
    // What a failed read of Home says (`refreshWords`): the network, or an error.
    let home_failed = match &home {
        Err(tafs_core::signin::KiffError::Offline) => {
            Some(tafs_core::sidebar::Unreachable::Network)
        }
        Err(_) => Some(tafs_core::sidebar::Unreachable::Error),
        Ok(a) if a.status != 200 => Some(tafs_core::sidebar::Unreachable::Error),
        Ok(_) => None,
    };
    let home = home.ok();
    let held = kiff::held(token, languages).ok();
    let activity = kiff::activity(token, languages).ok();
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
        match kiff::did_this_happen(token, languages) {
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
        return Read::Unreachable(home_failed.unwrap_or(tafs_core::sidebar::Unreachable::Error));
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
    let call_refs = call_refs(token, languages, &home, &activity);
    let (access, used_by, reports) = taf_reads(words, token, languages, &home);
    // `loadRecipes`: read with the rest; one that can't be read lists none.
    let recipes = kiff::recipes(token, languages)
        .ok()
        .filter(|a| a.status == 200)
        .and_then(|a| tafs_core::recipes::Recipes::read(&a.body));
    // The account's halt, for the sidebar's Halt all: none read, it reads running.
    let account_halt = kiff::halt(token, languages, None, false)
        .ok()
        .filter(|a| a.status == 200)
        .and_then(|a| tafs_core::halt::read(&a.body));
    // `loadGallery`: a read that fails leaves none.
    let gallery = kiff::gallery(token, languages)
        .ok()
        .filter(|a| a.status == 200)
        .and_then(|a| tafs_core::recipes::Gallery::read(&a.body));
    // The balance alone, for the plan's line: quietly, a failure shows none.
    let balance = kiff::cloud(token, languages, tafs_core::usage::BALANCE_PATH)
        .ok()
        .filter(|a| a.status == 200)
        .and_then(|a| tafs_core::usage::Balance::read(&a.body));
    // Your brief, read with the list: tafs cloud answering `/v1/brief` is what turns
    // it on (a 404, an error or a brief with nothing in it keeps Needs you as today).
    let brief = kiff::cloud(token, languages, &tafs_core::brief::Brief::path(None))
        .ok()
        .filter(|a| a.status == 200)
        .and_then(|a| brief::read_answer(&a.body, words));
    // Coach's id, only when a row's setup stopped partway and this person may use
    // Coach (`canFinishSetup`): a read that fails finishes none here.
    let stopped = home.setting_up().iter().any(|s| s.chat.is_some())
        || home
            .other_agents()
            .iter()
            .any(|t| t.finish_chat().is_some());
    let coach_setup = if stopped
        && account.as_ref().is_some_and(|a| {
            a.capabilities
                .as_ref()
                .is_some_and(|c| c.iter().any(|x| x == "members.manage"))
        }) {
        kiff::cloud(token, languages, tafs_core::coach::SETUP_PATH)
            .ok()
            .filter(|a| a.status == 200)
            .and_then(|a| tafs_core::coach::setup_id(&a.body).ok().flatten())
    } else {
        None
    };
    // The account's connections (`refreshConnections`), read with this computer's device key on
    // planque's safe route: a taf whose connection its vendor signed out waits on the owner. One
    // that can't be read gives none, and the rest still is.
    let grants = home_device(kept, languages)
        .and_then(|device| {
            kiff::planque_safe(kept, &device, languages, tafs_core::planque::Safe::Grants).ok()
        })
        .filter(|a| (200..300).contains(&a.status))
        .and_then(|a| tafs_core::connections::Grant::read_all(&a.body))
        .unwrap_or_default();
    Read::Cloud(Box::new(Cloud {
        grants,
        coach_setup,
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
}

fn home_model(
    w: &WordBook,
    c: &Cloud,
    at: chrono::DateTime<chrono::Local>,
) -> Result<(SidebarModel, Vec<tafs_core::overview::OverviewTaf>), tafs_core::words::WordsError> {
    let (h, exceptions, grants, activity, access, balance) = (
        &c.home,
        &c.held,
        &c.grants,
        &c.activity,
        &c.access,
        c.balance.as_ref(),
    );
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
    // `signedOut`: a connection its vendor signed out, for the taf it was made for.
    let signed_out = |t: &tafs_core::home::Taf| {
        grants
            .iter()
            .find(|g| g.request.agent == t.agent && g.needs_reconnect())
    };
    // `waitingOnOwner`: a call held for an answer, or a connection to sign in to again.
    let holds = |t: &tafs_core::home::Taf| held.iter().any(|c| c.taf == t.key);
    let waits = |t: &tafs_core::home::Taf| holds(t) || signed_out(t).is_some();
    // Its newest call in the Record, by its KIFF agent, or its chat's newest message as kept here
    // (`byRecent`: "the latest message in its chat").
    let latest = |t: &tafs_core::home::Taf| {
        activity
            .iter()
            .filter(|c| c.actor_id.as_deref() == Some(t.agent.as_str()))
            .map(|c| c.created_at)
            .chain(taf_chat::last_message_at(&t.key))
            .max()
    };
    let ordered = tafs_core::sidebar::by_recent(&tafs, waits, latest);
    let rows = ordered
        .iter()
        .map(|t| {
            let mut row = tafs_core::home::row(t);
            // Its line says so first, in planque's words, in the amber.
            if let Some(g) = signed_out(t) {
                row.now = tafs_core::connections::signed_out_words(w, g);
                row.tone = tafs_core::sidebar::Tone::Amber;
            }
            // Its open trial, under it, for whoever manages it: its sheet is reached from the
            // taf's Versions, which only they see.
            if access.get(&t.agent).is_some_and(|a| a.manage) {
                row.trial = tafs_core::home::trial_row(w, t)
                    .ok()
                    .flatten()
                    .map(Box::new);
            }
            (row, tafs_core::home::relevant(t, holds(t), at))
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
            .map(|s| {
                let discard = s.discard.as_ref().map(|d| tafs_core::sidebar::Discard {
                    agent: s.kiff_agent.clone(),
                    words: d.clone(),
                });
                (s.row(), discard)
            })
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
        finish: Default::default(),
        unreachable: None,
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
/// KIFF's held calls from a file, as `GET /v1/me/card-exceptions?status=held` answers.
fn held_file(path: Option<&str>) -> Vec<CardException> {
    let Some(path) = path else {
        return vec![];
    };
    match std::fs::read(path)
        .ok()
        .and_then(|b| tafs_core::held::read_held(&b))
    {
        Some(calls) => calls,
        None => {
            eprintln!("{path} isn't KIFF's held calls as it answers them");
            std::process::exit(2);
        }
    }
}

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
