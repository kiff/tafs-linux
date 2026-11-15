//! tafs for Linux: GTK 4 and libadwaita. The same product, account and words as the iPad app.

mod kiff;
mod overview;
mod shell;
mod sign_in;
mod store;
mod style;
mod welcome;
mod widgets;

use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::home::Home;
use tafs_core::language;
use tafs_core::sample;
use tafs_core::session::KeptSession;
use tafs_core::sidebar::{Mode, Selection, SidebarModel};
use tafs_core::words::{WordBook, SHIPPED};

/// Opens the shell: over the sample or the account, view-only or not, with Home once read.
type OpenShell = dyn Fn(Mode, bool, Option<Rc<Home>>);

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
    screenshot: Option<String>,
}

const USAGE: &str = "tafs [--lang en|de|fr|es|it|pt|nl|pl] [--theme light|dark] [--sample] [--signed-in] [--view-only]
     [--home contract.json] [--page welcome|sign-in|overview|needs-you|record|calendar|usage|connect-apps|account] [--screenshot out.png]";

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

    let manager = adw::StyleManager::default();
    manager.set_color_scheme(match o.theme.as_deref() {
        Some("dark") => adw::ColorScheme::ForceDark,
        Some("light") => adw::ColorScheme::ForceLight,
        _ => adw::ColorScheme::Default,
    });
    let display = gtk::gdk::Display::default().expect("a display");
    style::install(&display);

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .default_width(WIDTH)
        .default_height(HEIGHT)
        .title(words.line("app.name").unwrap_or_default())
        .build();
    window.set_size_request(900, 600);
    let screenshot = o.screenshot.is_some();

    // What opens: the sample when asked, else the kept session, else the welcome. A screenshot
    // never reads or writes the keyring.
    let now = chrono::Utc::now().timestamp();
    let kept = if screenshot || o.sample {
        None
    } else {
        store::load(now)
    };
    // The shell: over the sample, or the account's with Home as tafs cloud last sent it (none
    // read yet: no tafs listed).
    let open_shell: Rc<OpenShell> = {
        let (window, words, page) = (window.clone(), words.clone(), o.page.clone());
        let signed_in = o.signed_in;
        Rc::new(move |mode: Mode, view_only: bool, home: Option<Rc<Home>>| {
            let at = chrono::Local::now();
            let fail = |e: tafs_core::words::WordsError| -> ! { panic!("{e}") };
            let (model, tafs, cloud_words) = match (mode, &home) {
                (Mode::Sample, _) => (
                    sample::sidebar(&words, at, signed_in).unwrap_or_else(|e| fail(e)),
                    sample::overview(&words, at).unwrap_or_else(|e| fail(e)),
                    None,
                ),
                (_, Some(h)) => {
                    let (model, tafs) = home_model(&words, h, at).unwrap_or_else(|e| fail(e));
                    (model, tafs, h.needs_you.as_ref().map(|n| n.words.clone()))
                }
                (_, None) => (kiff_model(), vec![], None),
            };
            let count = model.needs_you_count();
            let needs_you = match cloud_words {
                Some(words) => words,
                None => {
                    tafs_core::overview::needs_you_words(&words, count).unwrap_or_else(|e| fail(e))
                }
            };
            let overview =
                tafs_core::overview::lines(&words, &tafs, &model.held, count, &needs_you)
                    .unwrap_or_else(|e| fail(e));
            let shell = shell::build(
                words.clone(),
                &model,
                overview,
                view_only,
                selection_of(page.as_deref()),
            );
            window.set_content(Some(&shell.root));
        })
    };
    // Signed in: the shell at once, then Home from tafs cloud, drawn again once it's read. An
    // answer that can't be read, or isn't a 200, leaves it as it was.
    let open_account: Rc<dyn Fn(KeptSession)> = {
        let (open_shell, languages) = (open_shell.clone(), languages.clone());
        Rc::new(move |kept: KeptSession| {
            let view_only = kept.approval_mode.is_view_only();
            open_shell(Mode::Kiff, view_only, None);
            let (open_shell, languages, token) =
                (open_shell.clone(), languages.clone(), kept.token.clone());
            glib::spawn_future_local(async move {
                let answer = gio::spawn_blocking(move || kiff::home(&token, &languages))
                    .await
                    .ok()
                    .and_then(Result::ok);
                if let Some(home) = answer
                    .filter(|a| a.status == 200)
                    .and_then(|a| Home::read(&a.body))
                {
                    open_shell(Mode::Kiff, view_only, Some(Rc::new(home)));
                }
            });
        })
    };

    let from_file = o.home.as_deref().map(|path| match read_contract(path) {
        Some(h) => Rc::new(h),
        None => {
            eprintln!("{path} isn't Home as tafs cloud answers it");
            std::process::exit(2);
        }
    });
    match (o.page.as_deref(), &kept) {
        _ if o.sample => open_shell(Mode::Sample, false, None),
        _ if from_file.is_some() => open_shell(Mode::Kiff, o.view_only, from_file.clone()),
        _ if o.view_only => open_shell(Mode::Kiff, true, None),
        (Some("welcome") | Some("sign-in"), _) | (_, None) => {
            let welcome = welcome::build(&words);
            window.set_content(Some(&welcome.root));
            let present: Rc<dyn Fn(Option<tafs_core::signin::DeviceStart>)> = {
                let (window, words, languages, open_account) = (
                    window.clone(),
                    words.clone(),
                    languages.clone(),
                    open_account.clone(),
                );
                Rc::new(move |preview| {
                    let open_account = open_account.clone();
                    let done: Rc<dyn Fn(KeptSession)> =
                        Rc::new(move |kept: KeptSession| open_account(kept));
                    sign_in::present(
                        &window,
                        window.upcast_ref(),
                        words.clone(),
                        languages.clone(),
                        done,
                        preview,
                    );
                })
            };
            {
                let present = present.clone();
                welcome.sign_in.connect_clicked(move |_| present(None));
            }
            {
                let open_shell = open_shell.clone();
                welcome
                    .sample
                    .connect_clicked(move |_| open_shell(Mode::Sample, false, None));
            }
            if o.page.as_deref() == Some("sign-in") {
                // A screenshot shows a code without reaching KIFF.
                let preview = screenshot.then(|| sign_in::preview(lang));
                present(preview);
            }
        }
        (_, Some(kept)) => open_account(kept.clone()),
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

/// A contract file's answer (`shared/contracts/home.json`), as tafs cloud would send it.
fn read_contract(path: &str) -> Option<Home> {
    let file: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Home::read(&serde_json::to_vec(file.get("answer")?).ok()?)
}

/// The account's sidebar and Overview from Home: its tafs in tafs cloud's order, Your tafs
/// those relevant now, and tafs cloud's count of what waits.
fn home_model(
    w: &WordBook,
    h: &Home,
    at: chrono::DateTime<chrono::Local>,
) -> Result<(SidebarModel, Vec<tafs_core::overview::OverviewTaf>), tafs_core::words::WordsError> {
    let mut rows = Vec::new();
    let mut overview = Vec::new();
    for t in &h.tafs {
        let taf = tafs_core::home::taf(w, t)?;
        rows.push((
            tafs_core::home::row(&taf),
            tafs_core::home::relevant(&taf, false, at),
        ));
        overview.push(tafs_core::home::overview_taf(&taf));
    }
    let model = SidebarModel {
        list_loaded: true,
        tafs: rows,
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
        may_use_coach: true,
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
