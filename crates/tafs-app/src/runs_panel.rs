//! A tafs cloud taf's runs, on its Activity after its work (`TafActivityList.parts`):
//! `RunsPanel`, when it runs next and Run now, and `RetainedRuns`, the runs tafs cloud kept,
//! three on the page and the rest a page at a time in a sheet. Every line is tafs cloud's
//! (`tafs-core/src/runs.rs`).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::runs::{self, RunsPage};
use tafs_core::words::WordBook;
use tafs_core::work::Row;

use crate::widgets::{eyebrow, outline, panel, text};

/// A loader, set once it's made, for a button drawn before it.
type Later = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// Where a taf's runs are read, and Run now sent.
#[derive(Clone)]
pub enum RunsSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's first page (`--runs`): every page reads it, and Run now sends nothing.
    File(std::path::PathBuf),
    None,
}

fn plain(w: &WordBook, a: &tafs_core::signin::Answer) -> String {
    tafs_core::signin::refusal(a, Some(w))
        .words(w)
        .unwrap_or_default()
}

/// A page, read off the main loop.
async fn read_page(
    w: &WordBook,
    source: &RunsSource,
    id: &str,
    cursor: Option<String>,
    limit: u32,
) -> Result<RunsPage, String> {
    let (words, source, id) = (w.clone(), source.clone(), id.to_string());
    gio::spawn_blocking(move || -> Result<RunsPage, String> {
        let body = match &source {
            RunsSource::Cloud { token, languages } => {
                let a = crate::kiff::taf_activity(
                    token,
                    languages,
                    &runs::path(&id, cursor.as_deref(), limit),
                )
                .map_err(|e| e.words(&words).unwrap_or_default())?;
                if a.status != 200 {
                    return Err(plain(&words, &a));
                }
                a.body
            }
            RunsSource::File(f) => std::fs::read(f).map_err(|e| e.to_string())?,
            RunsSource::None => return Err(String::new()),
        };
        runs::read_page(&body)
            .ok_or_else(|| tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
    })
    .await
    .unwrap_or_else(|_| Err(String::new()))
}

/// A row: a run's line, what it used, where its result went; or a skipped firing, dim.
fn row(r: &Row) -> gtk::Widget {
    match r {
        Row::Skipped(s) => text(&s.words, 13, 400, &["tafs-dim"]).upcast(),
        Row::Request(r) => {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
            v.append(&text(&r.line, 14, 400, &["tafs-ink"]));
            if let Some(used) = &r.credits {
                v.append(&text(used, 12, 400, &["tafs-dim"]));
            }
            for d in &r.deliveries {
                v.append(&text(&d.line, 12, 400, &["tafs-dim"]));
            }
            v.upcast()
        }
    }
}

/// `RunsPanel`: the line, and Run now with tafs cloud's words when it started no run. `line` is
/// `runs::next_line`'s; with no Run now, it works from its chat.
pub fn runs_panel(
    w: Rc<WordBook>,
    id: String,
    source: RunsSource,
    line: (String, bool),
    after_run: Rc<dyn Fn()>,
) -> gtk::Box {
    let p = panel(false);
    p.append(&eyebrow(&w.line("taf.runs").unwrap_or_default(), false));
    p.append(&text(&line.0, 15, 400, &["tafs-ink"]));
    if !line.1 {
        return p;
    }
    let run = outline(&w.line("taf.run_now").unwrap_or_default());
    run.set_hexpand(false);
    run.set_halign(gtk::Align::Start);
    p.append(&run);
    let said = text("", 13, 400, &["tafs-soft"]);
    said.set_visible(false);
    p.append(&said);
    run.connect_clicked(move |b| {
        // Set on the tap itself, before the request starts, so a second tap can't send one.
        b.set_sensitive(false);
        b.set_label(&w.line("taf.starting_run").unwrap_or_default());
        said.set_visible(false);
        let (w, id, source, b, said, after_run) = (
            w.clone(),
            id.clone(),
            source.clone(),
            b.clone(),
            said.clone(),
            after_run.clone(),
        );
        glib::spawn_future_local(async move {
            let words: WordBook = (*w).clone();
            let done = gio::spawn_blocking(move || -> Result<runs::RunNow, String> {
                match &source {
                    RunsSource::Cloud { token, languages } => {
                        let a = crate::kiff::run_now(token, languages, &id)
                            .map_err(|e| e.words(&words).unwrap_or_default())?;
                        if a.status != 200 && a.status != 201 {
                            return Err(plain(&words, &a));
                        }
                        runs::read_run_now(&a.body).ok_or_else(|| {
                            tafs_core::signin::plain_words(&words, 200).unwrap_or_default()
                        })
                    }
                    // A screenshot's runs send nothing.
                    _ => Ok(runs::RunNow {
                        session: None,
                        words: None,
                    }),
                }
            })
            .await
            .unwrap_or_else(|_| Err(String::new()));
            b.set_sensitive(true);
            b.set_label(&w.line("taf.run_now").unwrap_or_default());
            match done {
                // The run it started shows among the runs it kept.
                Ok(runs::RunNow {
                    session: Some(_), ..
                }) => after_run(),
                // Its check started no run: tafs cloud's words, as they are.
                Ok(runs::RunNow { words: Some(t), .. }) => {
                    said.set_label(&t);
                    said.remove_css_class("tafs-no");
                    said.set_visible(true);
                }
                Ok(_) => {}
                Err(why) => {
                    said.set_label(
                        &w.get("taf.run_not_started", &[("why", &why)])
                            .unwrap_or_default(),
                    );
                    said.add_css_class("tafs-no");
                    said.set_visible(true);
                }
            }
        });
    });
    p
}

/// `RetainedRuns` on the page: tafs cloud's heading, the first page's three runs and skipped
/// firings, what went wrong and Try again, and its "more" line opening every run. `reload` reads
/// the page again (after a Run now).
pub fn retained_runs(w: Rc<WordBook>, id: String, source: RunsSource) -> (gtk::Box, Rc<dyn Fn()>) {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let load: Later = Rc::new(RefCell::new(None));
    let reload: Rc<dyn Fn()> = {
        let (w, id, source, root, load) = (
            w.clone(),
            id.clone(),
            source.clone(),
            root.clone(),
            load.clone(),
        );
        Rc::new(move || {
            let (w, id, source, root, load) = (
                w.clone(),
                id.clone(),
                source.clone(),
                root.clone(),
                load.clone(),
            );
            glib::spawn_future_local(async move {
                let read = read_page(&w, &source, &id, None, runs::PANEL).await;
                while let Some(c) = root.first_child() {
                    root.remove(&c);
                }
                let p = panel(false);
                let l = |k: &str| w.line(k).unwrap_or_default();
                match read {
                    Ok(page) => {
                        p.append(&eyebrow(&page.title, false));
                        for r in page.rows(true) {
                            p.append(&row(&r));
                        }
                        if !page.more_title.is_empty() {
                            let more = gtk::Button::with_label(&page.more_title);
                            more.add_css_class("tafs-link");
                            more.set_halign(gtk::Align::Start);
                            let (w, id, source, title) =
                                (w.clone(), id.clone(), source.clone(), page.title.clone());
                            more.connect_clicked(move |b| {
                                all_runs(w.clone(), id.clone(), source.clone(), &title, b)
                            });
                            p.append(&more);
                        }
                    }
                    Err(why) => {
                        p.append(&eyebrow(&l("taf.its_work"), false));
                        p.append(&text(&why, 14, 400, &["tafs-no"]));
                        let again = outline(&l("common.try_again"));
                        again.set_hexpand(false);
                        again.set_halign(gtk::Align::Start);
                        let load = load.clone();
                        again.connect_clicked(move |_| {
                            let f = load.borrow().clone();
                            if let Some(f) = f {
                                f();
                            }
                        });
                        p.append(&again);
                    }
                }
                root.append(&p);
            });
        })
    };
    *load.borrow_mut() = Some(reload.clone());
    if !matches!(source, RunsSource::None) {
        reload();
    }
    (root, reload)
}

/// Every run, in a sheet titled as the panel: 20 a page, the next read as the list reaches its
/// end, then tafs cloud's end words.
fn all_runs(
    w: Rc<WordBook>,
    id: String,
    source: RunsSource,
    title: &str,
    parent: &impl IsA<gtk::Widget>,
) {
    struct State {
        cursor: Option<String>,
        first: bool,
        loading: bool,
        done: bool,
    }
    let list = gtk::Box::new(gtk::Orientation::Vertical, 14);
    list.set_margin_start(16);
    list.set_margin_end(16);
    list.set_margin_top(16);
    list.set_margin_bottom(16);
    let foot = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.append(&list);
    column.append(&foot);
    let state = Rc::new(RefCell::new(State {
        cursor: None,
        first: true,
        loading: false,
        done: false,
    }));
    let next: Later = Rc::new(RefCell::new(None));
    let more: Rc<dyn Fn()> = {
        let (w, id, source, list, foot, state, next) = (
            w.clone(),
            id.clone(),
            source.clone(),
            list.clone(),
            foot.clone(),
            state.clone(),
            next.clone(),
        );
        Rc::new(move || {
            let cursor = {
                let mut s = state.borrow_mut();
                if s.loading || s.done {
                    return;
                }
                s.loading = true;
                s.cursor.clone()
            };
            while let Some(c) = foot.first_child() {
                foot.remove(&c);
            }
            let spinner = gtk::Spinner::new();
            spinner.start();
            spinner.set_halign(gtk::Align::Start);
            foot.append(&spinner);
            let (w, id, source, list, foot, state, next) = (
                w.clone(),
                id.clone(),
                source.clone(),
                list.clone(),
                foot.clone(),
                state.clone(),
                next.clone(),
            );
            glib::spawn_future_local(async move {
                let read = read_page(&w, &source, &id, cursor, runs::ALL).await;
                while let Some(c) = foot.first_child() {
                    foot.remove(&c);
                }
                let mut s = state.borrow_mut();
                s.loading = false;
                match read {
                    Ok(page) => {
                        for r in page.rows(s.first) {
                            list.append(&row(&r));
                        }
                        s.first = false;
                        // A screenshot's file has one page.
                        s.cursor = page
                            .next_cursor
                            .clone()
                            .filter(|_| matches!(source, RunsSource::Cloud { .. }));
                        if s.cursor.is_none() {
                            s.done = true;
                            if let Some(end) = &page.end_words {
                                foot.append(&text(end, 14, 400, &["tafs-dim"]));
                            }
                        }
                    }
                    Err(why) => {
                        foot.append(&text(&why, 14, 400, &["tafs-no"]));
                        let again = outline(&w.line("common.try_again").unwrap_or_default());
                        again.set_hexpand(false);
                        again.set_halign(gtk::Align::Start);
                        let next = next.clone();
                        again.connect_clicked(move |_| {
                            let f = next.borrow().clone();
                            if let Some(f) = f {
                                f();
                            }
                        });
                        foot.append(&again);
                    }
                }
            });
        })
    };
    *next.borrow_mut() = Some(more.clone());
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    {
        let more = more.clone();
        scroller.connect_edge_reached(move |_, edge| {
            if edge == gtk::PositionType::Bottom {
                more();
            }
        });
    }
    let header = adw::HeaderBar::new();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_title(title);
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(560);
    dialog.set_content_height(680);
    // A page that doesn't fill the sheet reaches no edge: read on while there's room.
    {
        let (more, scroller, state) = (more.clone(), scroller.clone(), state.clone());
        let ticks = Rc::new(Cell::new(0));
        glib::timeout_add_local(std::time::Duration::from_millis(400), move || {
            ticks.set(ticks.get() + 1);
            let s = state.borrow();
            if s.done || ticks.get() > 50 {
                return glib::ControlFlow::Break;
            }
            let room = scroller.vadjustment().upper() <= scroller.vadjustment().page_size();
            drop(s);
            if room {
                more();
            }
            glib::ControlFlow::Continue
        });
    }
    more();
    dialog.present(Some(parent));
}
