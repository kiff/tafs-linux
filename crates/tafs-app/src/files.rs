//! Files (`FilesViews.swift`, kiff/tafs `docs/files.md`): what the account's tafs delivered in their runs, kept 30
//! days from the run, seen by everyone on the account. Account › Files opens the account's, with All, By taf and By
//! run and a chip per taf; a taf's page opens that taf's, by run. A file opens over the list: its start as plain
//! text (for a type that has one), Download, its taf, run, type and size, who sees it and when it goes, and Delete
//! now apart, behind a confirm, for everyone at once.
//!
//! The lines and the order are `tafs-core/src/files.rs`. Download reads the file's bytes only when asked, from the
//! address tafs cloud gives for a minute, with no session (`kiff::download`), and writes them where the person picks,
//! readable by them alone. Linux has no share sheet: the iPad's Share has no counterpart here, and Download is it.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::files::{self, DeliveredFile, PreviewText};
use tafs_core::words::WordBook;

use crate::widgets::{one_line, outline, panel, symbol, text};

/// Where Files is read from.
#[derive(Clone)]
pub enum FilesSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// tafs cloud's contracts beside each other (`--files <files-list.json>`, for the screenshots): the list, and
    /// `files-file.json`'s start for its file. Nothing is downloaded or deleted.
    File(std::path::PathBuf),
    None,
}

fn answer_of(path: &std::path::Path) -> Option<Vec<u8>> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    serde_json::to_vec(&v["answer"]).ok()
}

/// Every page of the account's files (`CloudFilesStore.files`), at most `files::PAGES`; or why not.
fn read_all(w: &WordBook, source: &FilesSource) -> Result<Vec<DeliveredFile>, String> {
    let why = |why: String| {
        w.get(
            "common.could_not_read_why",
            &[
                ("what", &w.line("files.title").unwrap_or_default()),
                ("why", &why),
            ],
        )
        .unwrap_or(why)
    };
    match source {
        FilesSource::Cloud { token, languages } => {
            let mut all = vec![];
            let mut after: Option<String> = None;
            for _ in 0..files::PAGES {
                let a = crate::kiff::cloud(token, languages, &files::list_path(after.as_deref()))
                    .map_err(|e| why(e.words(w).unwrap_or_default()))?;
                if a.status != 200 {
                    return Err(why(tafs_core::signin::refusal(&a, Some(w))
                        .words(w)
                        .unwrap_or_default()));
                }
                let (page, next) = files::read_page(&a.body).ok_or_else(|| {
                    why(tafs_core::signin::plain_words(w, 200).unwrap_or_default())
                })?;
                all.extend(page);
                match next {
                    Some(n) => after = Some(n),
                    None => break,
                }
            }
            Ok(all)
        }
        FilesSource::File(path) => answer_of(path)
            .and_then(|b| files::read_page(&b))
            .map(|(f, _)| f)
            .ok_or_else(|| why(String::new())),
        FilesSource::None => Ok(vec![]),
    }
}

/// A file's start (`readPreview`), only for a type tafs cloud gives one for: `Ok(None)` when it gave none, `Err` when
/// it couldn't be reached. The bytes are never read to draw it.
fn read_preview(source: &FilesSource, id: &str) -> Result<Option<PreviewText>, ()> {
    match source {
        FilesSource::Cloud { token, languages } => {
            let a = crate::kiff::cloud(token, languages, &files::file_path(id)).map_err(|_| ())?;
            if a.status != 200 {
                return Err(());
            }
            files::read_preview(&a.body).ok_or(())
        }
        FilesSource::File(path) => {
            let one = path.with_file_name("files-file.json");
            let body = answer_of(&one).ok_or(())?;
            let v: serde_json::Value = serde_json::from_slice(&body).map_err(|_| ())?;
            if v["id"].as_str() != Some(id) {
                return Ok(None);
            }
            files::read_preview(&body).ok_or(())
        }
        FilesSource::None => Ok(None),
    }
}

/// Why tafs cloud refused, in its words when it gave them, else the app's for its status.
fn refused(w: &WordBook, a: &tafs_core::signin::Answer) -> String {
    tafs_core::signin::refusal(a, Some(w))
        .words(w)
        .unwrap_or_default()
}

/// The file's bytes (`contents(of:)`): the address tafs cloud gives for a minute, read with no session. Or why not:
/// the offline line only when nothing answered, else what tafs cloud or storage said (dev-android's note on #86).
fn read_bytes(w: &WordBook, source: &FilesSource, id: &str) -> Result<Vec<u8>, String> {
    let FilesSource::Cloud { token, languages } = source else {
        return Err(w.line("error.offline").unwrap_or_default());
    };
    let a = crate::kiff::cloud(token, languages, &files::download_path(id))
        .map_err(|e| e.words(w).unwrap_or_default())?;
    if a.status != 200 {
        return Err(refused(w, &a));
    }
    let url = files::read_download(&a.body)
        .ok_or_else(|| tafs_core::signin::plain_words(w, 200).unwrap_or_default())?;
    let got = crate::kiff::download(url.as_str()).map_err(|e| e.words(w).unwrap_or_default())?;
    if got.status != 200 {
        return Err(tafs_core::signin::plain_words(w, got.status).unwrap_or_default());
    }
    Ok(got.body)
}

/// A file that couldn't be written where the person picked: "Download didn't go through.", with no reason after it.
fn not_saved(w: &WordBook) -> String {
    w.get(
        "common.didnt_go_through",
        &[
            ("what", &w.line("files.download").unwrap_or_default()),
            ("why", ""),
        ],
    )
    .map(|l| l.trim().to_string())
    .unwrap_or_default()
}

/// Delete now, for everyone (`deleteFile`): once tafs cloud deleted it, or why not, as `read_bytes` says it.
fn delete(w: &WordBook, source: &FilesSource, id: &str) -> Result<(), String> {
    let FilesSource::Cloud { token, languages } = source else {
        return Err(w.line("error.offline").unwrap_or_default());
    };
    let a = crate::kiff::cloud_send(token, languages, "DELETE", &files::file_path(id), None)
        .map_err(|e| e.words(w).unwrap_or_default())?;
    if (200..300).contains(&a.status) {
        Ok(())
    } else {
        Err(refused(w, &a))
    }
}

type Redraw = Rc<dyn Fn()>;
/// A group's header, from its first file.
type Head = Box<dyn Fn(&files::Group) -> String>;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Grouping {
    All,
    ByTaf,
    ByRun,
}

struct State {
    files: Option<Result<Vec<DeliveredFile>, String>>,
    grouping: Grouping,
    /// A chip's taf, narrowing the account's files to one.
    chip: Option<String>,
    /// A wide window (the iPad's `sizeClass == .regular`): the list 380 wide, the file beside it.
    wide: bool,
    /// The file picked on a wide window; the first shown when none is.
    selected: Option<String>,
}

/// The least width of the page for the file beside the list: the list's 380 and as much again.
const WIDE_FROM: f64 = 760.0;

impl State {
    /// The files shown: the taf's, or the chip's, or all.
    fn mine(&self, taf: Option<&String>) -> Vec<DeliveredFile> {
        let Some(Ok(all)) = &self.files else {
            return vec![];
        };
        all.iter()
            .filter(|f| taf.or(self.chip.as_ref()).is_none_or(|t| &f.taf == t))
            .cloned()
            .collect()
    }

    fn groups(&self, mine: &[DeliveredFile]) -> Vec<files::Group> {
        match self.grouping {
            Grouping::All => files::by_day(mine),
            Grouping::ByTaf => files::by_taf(mine),
            Grouping::ByRun => files::by_run(mine),
        }
    }

    /// What's beside the list on a wide window (`selected ?? shown.first`): the file picked while
    /// it's still there, else the first shown.
    fn beside(&self, taf: Option<&String>) -> Option<DeliveredFile> {
        let Some(Ok(all)) = &self.files else {
            return None;
        };
        let picked = self
            .selected
            .as_ref()
            .and_then(|id| all.iter().find(|f| &f.id == id));
        picked.cloned().or_else(|| {
            let mine = self.mine(taf);
            self.groups(&mine)
                .first()
                .and_then(|g| g.files.first())
                .cloned()
        })
    }
}

/// The file's type, as a small badge: its own extension, never an icon that says more than it knows.
fn badge(kind: &str) -> gtk::Label {
    let b = gtk::Label::new(Some(kind));
    b.add_css_class("tafs-file-badge");
    b.set_size_request(40, 40);
    b.set_valign(gtk::Align::Center);
    b.set_accessible_role(gtk::AccessibleRole::Presentation);
    b
}

/// A file in a list (`FileRow`): its type, its name, the taf with its size, and when it goes.
fn row(w: &WordBook, f: &DeliveredFile, now: chrono::DateTime<chrono::Utc>) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    r.set_margin_top(8);
    r.set_margin_bottom(8);
    r.append(&badge(&f.kind));
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    v.append(&one_line(&f.name, 15, 600, &["tafs-ink"]));
    let size = files::size(w.language(), f.bytes);
    let under = [f.taf_name.as_str(), size.as_str()]
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" · ");
    v.append(&one_line(&under, 13, 400, &["tafs-dim"]));
    v.append(&one_line(
        &files::goes(w, files::days_left(f, now)).unwrap_or_default(),
        12,
        400,
        &["tafs-amber-text"],
    ));
    r.append(&v);
    r
}

/// The account's files (`taf` none), or one taf's by run, as tafs cloud's id.
pub fn page(w: Rc<WordBook>, source: FilesSource, taf: Option<String>) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(16);
    column.set_margin_bottom(16);
    let title = text(&l("files.title"), 34, 700, &["tafs-ink"]);
    title.set_width_chars(12);
    column.append(&title);
    let controls = gtk::Box::new(gtk::Orientation::Vertical, 8);
    column.append(&controls);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.append(&list);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    // On a wide window, the file beside the list (`FilesView`'s `HStack`: the list 380 wide, a
    // divider, the file).
    let beside = gtk::Box::new(gtk::Orientation::Vertical, 0);
    beside.set_margin_start(16);
    beside.set_margin_end(16);
    beside.set_margin_top(16);
    beside.set_margin_bottom(16);
    let beside_scroller = gtk::ScrolledWindow::new();
    beside_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    beside_scroller.set_hexpand(true);
    beside_scroller.set_child(Some(&beside));
    beside_scroller.set_widget_name("files.beside");
    let divider = gtk::Separator::new(gtk::Orientation::Vertical);
    let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    row_box.append(&scroller);
    row_box.append(&divider);
    row_box.append(&beside_scroller);
    crate::widgets::ground(&row_box);
    // Which file is beside the list now, so it's drawn again only when that changes.
    let beside_id: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));

    let state = Rc::new(RefCell::new(State {
        files: None,
        grouping: if taf.is_some() {
            Grouping::ByRun
        } else {
            Grouping::All
        },
        chip: None,
        wide: true,
        selected: None,
    }));
    let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
    let redraw: Rc<dyn Fn()> = {
        let draw = draw.clone();
        Rc::new(move || {
            let f = draw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        })
    };
    let load: Rc<dyn Fn()> = {
        let (w, source, state, redraw) = (w.clone(), source.clone(), state.clone(), redraw.clone());
        Rc::new(move || {
            let (w2, source, state, redraw) =
                ((*w).clone(), source.clone(), state.clone(), redraw.clone());
            glib::spawn_future_local(async move {
                let read = gio::spawn_blocking(move || read_all(&w2, &source))
                    .await
                    .unwrap_or_else(|_| Err(String::new()));
                state.borrow_mut().files = Some(read);
                redraw();
            });
        })
    };
    *draw.borrow_mut() = Some({
        let (w, source, state, controls, list, redraw, load, taf) = (
            w.clone(),
            source.clone(),
            state.clone(),
            controls.clone(),
            list.clone(),
            redraw.clone(),
            load.clone(),
            taf.clone(),
        );
        let (scroller, divider, beside_scroller, beside, beside_id) = (
            scroller.clone(),
            divider.clone(),
            beside_scroller.clone(),
            beside.clone(),
            beside_id.clone(),
        );
        Rc::new(move || {
            for b in [&controls, &list] {
                while let Some(c) = b.first_child() {
                    b.remove(&c);
                }
            }
            let l = |k: &str| w.line(k).unwrap_or_default();
            // The list alone, or 380 wide with the file beside it.
            let wide = state.borrow().wide;
            scroller.set_hexpand(!wide);
            scroller.set_size_request(if wide { 380 } else { -1 }, -1);
            divider.set_visible(wide);
            beside_scroller.set_visible(wide);
            let shown = if wide {
                state.borrow().beside(taf.as_ref())
            } else {
                None
            };
            let shown_id = shown.as_ref().map(|f| f.id.clone());
            if wide && *beside_id.borrow() != shown_id {
                *beside_id.borrow_mut() = shown_id.clone();
                while let Some(c) = beside.first_child() {
                    beside.remove(&c);
                }
                match shown {
                    Some(f) => {
                        // Deleted: nothing picked, and the list read again.
                        let gone: Rc<dyn Fn()> = {
                            let (state, load) = (state.clone(), load.clone());
                            Rc::new(move || {
                                state.borrow_mut().selected = None;
                                load();
                            })
                        };
                        let clamp = adw::Clamp::new();
                        clamp.set_maximum_size(640);
                        clamp.set_child(Some(&detail_column(w.clone(), source.clone(), f, gone)));
                        beside.append(&clamp);
                    }
                    None => {
                        let pick = text(&l("files.pick_one"), 14, 400, &["tafs-soft"]);
                        pick.set_vexpand(true);
                        pick.set_valign(gtk::Align::Center);
                        pick.set_halign(gtk::Align::Center);
                        beside.append(&pick);
                    }
                }
            }
            let s = state.borrow();
            let all = match &s.files {
                None => {
                    let spin = gtk::Spinner::new();
                    spin.start();
                    list.append(&spin);
                    return;
                }
                Some(Err(why)) => {
                    list.append(&text(why, 14, 400, &["tafs-no"]));
                    return;
                }
                Some(Ok(all)) => all,
            };
            let mine = s.mine(taf.as_ref());
            if mine.is_empty() && (taf.is_some() || all.is_empty()) {
                list.append(&text(&l("files.empty"), 14, 400, &["tafs-soft"]));
                return;
            }
            // The account's: All, By taf and By run, and a chip per taf with files.
            if taf.is_none() {
                let seg = gtk::Box::new(gtk::Orientation::Horizontal, 0);
                seg.add_css_class("linked");
                for (g, k) in [
                    (Grouping::All, "files.all"),
                    (Grouping::ByTaf, "files.by_taf"),
                    (Grouping::ByRun, "files.by_run"),
                ] {
                    let b = gtk::ToggleButton::with_label(&l(k));
                    b.set_active(s.grouping == g);
                    let (state, redraw) = (state.clone(), redraw.clone());
                    b.connect_clicked(move |_| {
                        state.borrow_mut().grouping = g;
                        redraw();
                    });
                    seg.append(&b);
                }
                controls.append(&seg);
                let chips = gtk::Box::new(gtk::Orientation::Horizontal, 6);
                let mut tafs: Vec<(Option<String>, String)> = vec![(None, l("files.all_tafs"))];
                for g in files::by_taf(all) {
                    if let Some(f) = g.files.first() {
                        tafs.push((Some(f.taf.clone()), f.taf_name.clone()));
                    }
                }
                for (id, name) in tafs {
                    let b = gtk::Button::with_label(&name);
                    b.add_css_class("tafs-chip-button");
                    if s.chip == id {
                        b.add_css_class("selected");
                    }
                    let (state, redraw) = (state.clone(), redraw.clone());
                    b.connect_clicked(move |_| {
                        state.borrow_mut().chip = id.clone();
                        redraw();
                    });
                    chips.append(&b);
                }
                let chip_scroll = gtk::ScrolledWindow::new();
                chip_scroll.set_vscrollbar_policy(gtk::PolicyType::Never);
                chip_scroll.set_child(Some(&chips));
                controls.append(&chip_scroll);
            }
            let now = chrono::Utc::now();
            let (groups, head): (Vec<files::Group>, Head) = match s.grouping {
                Grouping::All => (
                    files::by_day(&mine),
                    Box::new({
                        let lang = w.language().to_string();
                        move |g: &files::Group| {
                            g.files
                                .first()
                                .map(|f| {
                                    files::day_title(&lang, tafs_core::account_time::day(&f.run_at))
                                })
                                .unwrap_or_default()
                        }
                    }),
                ),
                Grouping::ByTaf => (
                    files::by_taf(&mine),
                    Box::new(|g: &files::Group| {
                        g.files
                            .first()
                            .map(|f| f.taf_name.clone())
                            .unwrap_or_default()
                    }),
                ),
                Grouping::ByRun => (
                    files::by_run(&mine),
                    Box::new({
                        let lang = w.language().to_string();
                        move |g: &files::Group| {
                            g.files
                                .first()
                                .map(|f| {
                                    [
                                        f.taf_name.clone(),
                                        files::day_title(
                                            &lang,
                                            tafs_core::account_time::day(&f.run_at),
                                        ),
                                    ]
                                    .into_iter()
                                    .filter(|s| !s.is_empty())
                                    .collect::<Vec<_>>()
                                    .join(" · ")
                                })
                                .unwrap_or_default()
                        }
                    }),
                ),
            };
            for g in &groups {
                list.append(&crate::widgets::eyebrow(&head(g), false));
                let p = panel(false);
                for f in &g.files {
                    let b = gtk::Button::new();
                    b.add_css_class("tafs-plain");
                    b.set_child(Some(&row(&w, f, now)));
                    b.set_widget_name(&format!("files.file.{}", f.id));
                    if wide && shown_id.as_deref() == Some(f.id.as_str()) {
                        b.add_css_class("tafs-file-picked");
                    }
                    let (w, source, f, load, state, redraw) = (
                        w.clone(),
                        source.clone(),
                        f.clone(),
                        load.clone(),
                        state.clone(),
                        redraw.clone(),
                    );
                    // Wide: the file beside the list. Narrow: its sheet.
                    b.connect_clicked(move |b| {
                        if state.borrow().wide {
                            state.borrow_mut().selected = Some(f.id.clone());
                            redraw();
                        } else {
                            detail(w.clone(), source.clone(), f.clone(), load.clone(), b)
                        }
                    });
                    p.append(&b);
                }
                list.append(&p);
            }
            list.append(&text(&l("files.kept_line"), 12, 400, &["tafs-dim"]));
            // `--page files-file`: the first file opens, for its screenshot (beside the list
            // already, on a wide window).
            if !wide && OPEN_FIRST.with(|o| o.replace(false)) {
                if let Some(f) = groups.first().and_then(|g| g.files.first()).cloned() {
                    let (w, source, load, list) =
                        (w.clone(), source.clone(), load.clone(), list.clone());
                    glib::idle_add_local_once(move || detail(w, source, f, load, &list));
                }
            }
        })
    });
    let bin = adw::BreakpointBin::new();
    bin.set_size_request(320, 240);
    bin.set_child(Some(&row_box));
    let narrow = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        WIDE_FROM,
        adw::LengthUnit::Px,
    ));
    for (signal_wide, apply) in [(false, true), (true, false)] {
        let (state, redraw) = (state.clone(), redraw.clone());
        let set = move |_: &adw::Breakpoint| {
            state.borrow_mut().wide = signal_wide;
            redraw();
        };
        if apply {
            narrow.connect_apply(set);
        } else {
            narrow.connect_unapply(set);
        }
    }
    bin.add_breakpoint(narrow);
    redraw();
    load();
    bin.upcast()
}

thread_local! {
    /// `--page files-file`: the first file's sheet opens once the list is drawn.
    pub static OPEN_FIRST: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// A fact's line: its name (13, dim) and value (14, ink).
fn fact(name: &str, value: &str) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let n = text(name, 13, 400, &["tafs-dim"]);
    n.set_size_request(110, -1);
    n.set_valign(gtk::Align::Start);
    r.append(&n);
    let v = text(value, 14, 400, &["tafs-ink"]);
    v.set_hexpand(true);
    r.append(&v);
    r
}

/// A file's sheet, over a narrow window's list: `deleted` reads the list again once tafs cloud
/// deleted it, and the sheet closes.
fn detail(
    w: Rc<WordBook>,
    source: FilesSource,
    f: DeliveredFile,
    deleted: Rc<dyn Fn()>,
    over: &impl IsA<gtk::Widget>,
) {
    let s = crate::sheet::new(
        &f.name,
        &w.line("common.done").unwrap_or_default(),
        None,
        560,
    );
    let gone: Rc<dyn Fn()> = {
        let dialog = s.dialog.clone();
        Rc::new(move || {
            dialog.close();
            deleted();
        })
    };
    s.column.append(&detail_column(w, source, f, gone));
    s.dialog.present(Some(over));
}

/// A file (`FileDetailView`): its start for a text type, Download, its facts, and Delete now behind a confirm.
/// `deleted` hears once tafs cloud deleted it. In a sheet on a narrow window, beside the list on a wide one.
fn detail_column(
    w: Rc<WordBook>,
    source: FilesSource,
    f: DeliveredFile,
    deleted: Rc<dyn Fn()>,
) -> gtk::Box {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let lang = w.language().to_string();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    head.append(&badge(&f.kind));
    let name = text(&f.name, 20, 700, &["tafs-ink"]);
    name.set_hexpand(true);
    head.append(&name);
    column.append(&head);
    // Its start, drawn as plain text, never as Markdown or HTML; read only for a type tafs cloud gives one for.
    let start = gtk::Box::new(gtk::Orientation::Vertical, 6);
    column.append(&start);
    if files::has_text_start(&f) {
        let spin = gtk::Spinner::new();
        spin.start();
        start.append(&spin);
        let (source2, id, start, w2) = (source.clone(), f.id.clone(), start.clone(), w.clone());
        glib::spawn_future_local(async move {
            let read = gio::spawn_blocking(move || read_preview(&source2, &id))
                .await
                .unwrap_or(Err(()));
            while let Some(c) = start.first_child() {
                start.remove(&c);
            }
            match read {
                Ok(Some(p)) => {
                    let p_box = panel(false);
                    let t = gtk::Label::new(Some(&p.text));
                    t.set_xalign(0.0);
                    t.set_wrap(true);
                    t.set_wrap_mode(gtk::pango::WrapMode::WordChar);
                    t.set_selectable(true);
                    t.add_css_class("monospace");
                    t.add_css_class("tafs-ink");
                    p_box.append(&t);
                    start.append(&p_box);
                    if p.truncated {
                        start.append(&text(
                            &w2.line("files.preview_start").unwrap_or_default(),
                            12,
                            400,
                            &["tafs-soft"],
                        ));
                    }
                }
                // Answered with no start: no preview, as on Android; nothing more read.
                Ok(None) => {}
                // As the iPad's (tafs#1728): "Files couldn't be read: {why}".
                Err(()) => start.append(&text(
                    &w2.get(
                        "common.could_not_read_why",
                        &[
                            ("what", &f_name_of(&w2)),
                            ("why", &w2.line("error.offline").unwrap_or_default()),
                        ],
                    )
                    .unwrap_or_default(),
                    13,
                    400,
                    &["tafs-no"],
                )),
            }
        });
    }
    // Download: the bytes read only now, written where the person picks, readable by them alone.
    let note = text("", 13, 400, &["tafs-ok"]);
    note.set_visible(false);
    // A download or a deletion that didn't go, and why: what tafs cloud said, or the offline line when nothing answered.
    let failed = text(&l("error.offline"), 13, 400, &["tafs-no"]);
    failed.set_visible(false);
    let download = outline(&l("files.download"));
    {
        let (w, source, id, file_name, note, title, failed) = (
            w.clone(),
            source.clone(),
            f.id.clone(),
            f.name.clone(),
            note.clone(),
            l("files.download"),
            failed.clone(),
        );
        let saved = l("files.saved");
        download.connect_clicked(move |b| {
            let window = b.root().and_downcast::<gtk::Window>();
            let dialog = gtk::FileDialog::new();
            dialog.set_title(&title);
            dialog.set_initial_name(Some(&file_name));
            let (w, source, id, note, saved, failed) = (
                w.clone(),
                source.clone(),
                id.clone(),
                note.clone(),
                saved.clone(),
                failed.clone(),
            );
            glib::spawn_future_local(async move {
                let Ok(file) = dialog.save_future(window.as_ref()).await else {
                    return;
                };
                let Some(path) = file.path() else {
                    return;
                };
                let words = (*w).clone();
                let offline = w.line("error.offline").unwrap_or_default();
                let written = gio::spawn_blocking(move || read_bytes(&words, &source, &id))
                    .await
                    .unwrap_or(Err(offline))
                    // Saved on this computer, or "Download didn't go through." in the reader's language: the
                    // system's own reason is in English whatever the app's (desktop-windows' note on #87).
                    .and_then(|b| {
                        crate::widgets::write_private(&path, &b).map_err(|_| not_saved(&w))
                    });
                note.set_label(&saved);
                note.set_visible(written.is_ok());
                if let Err(why) = &written {
                    failed.set_label(why);
                }
                failed.set_visible(written.is_err());
            });
        });
    }
    column.append(&download);
    column.append(&note);
    column.append(&failed);
    let facts = panel(false);
    if !f.taf_name.is_empty() {
        facts.append(&fact(&l("files.made_by"), &f.taf_name));
    }
    let local = tafs_core::account_time::local(&f.run_at);
    facts.append(&fact(
        &l("files.in_run"),
        &format!(
            "{} {}",
            tafs_core::locale::short_date(&lang, local.date()),
            local.format("%H:%M")
        ),
    ));
    facts.append(&fact(&l("files.type"), &f.kind));
    facts.append(&fact(&l("files.size"), &files::size(&lang, f.bytes)));
    facts.append(&fact(&l("files.who_sees"), &l("files.everyone")));
    facts.append(&fact(
        &l("files.goes_away"),
        &w.get(
            "files.goes_on",
            &[(
                "date",
                &tafs_core::locale::short_date(&lang, tafs_core::account_time::day(&f.goes_at)),
            )],
        )
        .unwrap_or_default(),
    ));
    column.append(&facts);
    // Delete now, apart, behind a confirm that says it's gone for everyone, right away.
    let delete_now = outline(&l("files.delete_now"));
    delete_now.add_css_class("tafs-no");
    delete_now.set_margin_top(12);
    {
        let (w, source, f, deleted, failed) = (
            w.clone(),
            source.clone(),
            f.clone(),
            deleted.clone(),
            failed.clone(),
        );
        delete_now.connect_clicked(move |b| {
            let l = |k: &str| w.line(k).unwrap_or_default();
            let ask = adw::AlertDialog::new(
                Some(
                    &w.get("files.delete_question", &[("name", &f.name)])
                        .unwrap_or_default(),
                ),
                Some(&l("files.delete_body")),
            );
            ask.add_response("cancel", &l("common.cancel"));
            ask.add_response("delete", &l("files.delete_now"));
            ask.set_response_appearance("delete", adw::ResponseAppearance::Destructive);
            ask.set_default_response(Some("cancel"));
            ask.set_close_response("cancel");
            let (w, source, id, deleted, failed) = (
                w.clone(),
                source.clone(),
                f.id.clone(),
                deleted.clone(),
                failed.clone(),
            );
            ask.connect_response(None, move |_, r| {
                if r != "delete" {
                    return;
                }
                let (w, source, id, deleted, failed) = (
                    w.clone(),
                    source.clone(),
                    id.clone(),
                    deleted.clone(),
                    failed.clone(),
                );
                glib::spawn_future_local(async move {
                    let words = (*w).clone();
                    let offline = w.line("error.offline").unwrap_or_default();
                    let gone = gio::spawn_blocking(move || delete(&words, &source, &id))
                        .await
                        .unwrap_or(Err(offline));
                    // Gone from the list only once tafs cloud deleted it.
                    match gone {
                        Ok(()) => deleted(),
                        Err(why) => {
                            failed.set_label(&why);
                            failed.set_visible(true);
                        }
                    }
                });
            });
            ask.present(Some(b));
        });
    }
    column.append(&delete_now);
    column
}

fn f_name_of(w: &WordBook) -> String {
    w.line("files.title").unwrap_or_default()
}

/// The Files entry first on a taf's header (`TafFilesEntry`): "Files", with how many it keeps now once read.
pub fn taf_entry(
    w: Rc<WordBook>,
    source: FilesSource,
    taf: String,
    open: Rc<dyn Fn()>,
) -> gtk::Button {
    let b = gtk::Button::new();
    b.add_css_class("tafs-plain");
    b.set_widget_name("taf.files");
    let face = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    face.append(&symbol("folder-symbolic", 14));
    let title = w.line("files.title").unwrap_or_default();
    let label = one_line(&title, 14, 600, &["tafs-ink"]);
    face.append(&label);
    b.set_child(Some(&face));
    b.set_size_request(-1, 44);
    b.connect_clicked(move |_| open());
    let (w2, label2, b2) = ((*w).clone(), label.clone(), b.clone());
    glib::spawn_future_local(async move {
        let words = w2.clone();
        let count = gio::spawn_blocking(move || {
            read_all(&words, &source)
                .map(|all| all.iter().filter(|f| f.taf == taf).count())
                .unwrap_or(0)
        })
        .await
        .unwrap_or(0);
        if count > 0 {
            let said = w2
                .count("files.taf_row", count as i64, &[])
                .unwrap_or_default();
            label2.set_label(&format!("{title} · {count}"));
            b2.update_property(&[gtk::accessible::Property::Label(&format!(
                "{title}, {said}"
            ))]);
        }
    });
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> State {
        let v: serde_json::Value = serde_json::from_slice(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../shared/contracts/files-list.json"
            ))
            .expect("the files contract at the pin"),
        )
        .unwrap();
        let (all, _) = files::read_page(&serde_json::to_vec(&v["answer"]).unwrap()).unwrap();
        State {
            files: Some(Ok(all)),
            grouping: Grouping::All,
            chip: None,
            wide: true,
            selected: None,
        }
    }

    #[test]
    fn beside_the_list_is_the_file_picked_else_the_first_shown() {
        let mut s = state();
        let mine = s.mine(None);
        assert!(mine.len() > 1, "the contract has files to pick from");
        let first = s.groups(&mine)[0].files[0].id.clone();
        assert_eq!(s.beside(None).map(|f| f.id), Some(first.clone()));
        // A file picked stays beside the list.
        let other = mine.iter().find(|f| f.id != first).unwrap().id.clone();
        s.selected = Some(other.clone());
        assert_eq!(s.beside(None).map(|f| f.id), Some(other));
        // Gone from the list (deleted): the first shown again.
        s.selected = Some("fil_gone".into());
        assert_eq!(s.beside(None).map(|f| f.id), Some(first));
        // A taf's own files: its first.
        let taf = mine.last().unwrap().taf.clone();
        let theirs = s.beside(Some(&taf)).unwrap();
        assert_eq!(theirs.taf, taf);
        // Nothing read yet, or a failed read: nothing beside it ("Pick a file to see it here.").
        s.files = None;
        assert_eq!(s.beside(None), None);
        s.files = Some(Err("offline".into()));
        assert_eq!(s.beside(None), None);
    }
}
