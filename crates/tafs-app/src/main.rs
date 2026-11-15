//! tafs for Linux: GTK 4 and libadwaita. The same product, account and words as the iPad app.

mod kiff;
mod shell;
mod sign_in;
mod store;
mod style;
mod welcome;
mod widgets;

use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::language;
use tafs_core::sample;
use tafs_core::session::KeptSession;
use tafs_core::sidebar::{Mode, Selection, SidebarModel};
use tafs_core::words::{WordBook, SHIPPED};

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
    screenshot: Option<String>,
}

const USAGE: &str = "tafs [--lang en|de|fr|es|it|pt|nl|pl] [--theme light|dark] [--sample] [--signed-in] [--view-only]
     [--page welcome|sign-in|overview|needs-you|record|calendar|usage|connect-apps|account] [--screenshot out.png]";

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
    let open_shell: Rc<dyn Fn(Mode, bool)> = {
        let (window, words, page) = (window.clone(), words.clone(), o.page.clone());
        let signed_in = o.signed_in;
        Rc::new(move |mode: Mode, view_only: bool| {
            let model = match mode {
                Mode::Sample => sample::sidebar(&words, chrono::Local::now(), signed_in)
                    .unwrap_or_else(|e| panic!("{e}")),
                _ => kiff_model(),
            };
            let shell = shell::build(
                words.clone(),
                &model,
                view_only,
                selection_of(page.as_deref()),
            );
            window.set_content(Some(&shell.root));
        })
    };

    match (o.page.as_deref(), &kept) {
        _ if o.sample => open_shell(Mode::Sample, false),
        _ if o.view_only => open_shell(Mode::Kiff, true),
        (Some("welcome") | Some("sign-in"), _) | (_, None) => {
            let welcome = welcome::build(&words);
            window.set_content(Some(&welcome.root));
            let present: Rc<dyn Fn(Option<tafs_core::signin::DeviceStart>)> = {
                let (window, words, languages, open_shell) = (
                    window.clone(),
                    words.clone(),
                    languages.clone(),
                    open_shell.clone(),
                );
                Rc::new(move |preview| {
                    let open_shell = open_shell.clone();
                    let done: Rc<dyn Fn(KeptSession)> = Rc::new(move |kept: KeptSession| {
                        open_shell(Mode::Kiff, kept.approval_mode.is_view_only())
                    });
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
                    .connect_clicked(move |_| open_shell(Mode::Sample, false));
            }
            if o.page.as_deref() == Some("sign-in") {
                // A screenshot shows a code without reaching KIFF.
                let preview = screenshot.then(|| sign_in::preview(lang));
                present(preview);
            }
        }
        (_, Some(kept)) => open_shell(Mode::Kiff, kept.approval_mode.is_view_only()),
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
