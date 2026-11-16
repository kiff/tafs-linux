//! The Record's search, signed in with KIFF (`RecordSearchBox`, `RecordResults`, `RecordHitRow`,
//! `RecordCaseSheet`): a box with tafs cloud's words as its placeholder; what's typed is searched
//! once the person stops typing (0.4 s), Enter searches at once, and an emptied box shows the Record
//! again. While a search is shown, it stands in for the list.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::record_search::{self, RecordCase, RecordHit, Search, SearchPage};
use tafs_core::words::WordBook;

use crate::widgets::{outline, panel, text};

/// Where the Record is searched: tafs cloud, as the session.
#[derive(Clone)]
pub struct SearchSource {
    pub token: String,
    pub languages: Vec<&'static str>,
    /// `--record-search`'s answers, for the screenshots: every search reads the page, every case
    /// the case, and nothing is sent.
    pub canned: Option<(SearchPage, Option<RecordCase>)>,
    /// `--page record-search:<q>`: what's in the box when the Record opens, for a screenshot.
    pub initial: Option<String>,
}

/// One page (`searchRecord`): the page, or why not, in plain words.
fn page_of(
    w: &WordBook,
    source: &SearchSource,
    q: &str,
    cursor: Option<&str>,
    limit: Option<u32>,
) -> Result<SearchPage, (u16, String)> {
    if let Some((page, _)) = &source.canned {
        return Ok(page.clone());
    }
    let path = record_search::search_path(q, cursor, limit);
    let a = crate::kiff::cloud(&source.token, &source.languages, &path)
        .map_err(|e| (0, e.words(w).unwrap_or_default()))?;
    if a.status != 200 {
        let e = tafs_core::signin::refusal(&a, Some(w));
        return Err((a.status, e.words(w).unwrap_or_default()));
    }
    SearchPage::read(&a.body).ok_or_else(|| {
        (
            200,
            tafs_core::signin::plain_words(w, 200).unwrap_or_default(),
        )
    })
}

fn case_of(w: &WordBook, source: &SearchSource, about: &str) -> Result<RecordCase, String> {
    if let Some((_, case)) = &source.canned {
        return case.clone().ok_or_else(String::new);
    }
    let a = crate::kiff::cloud(
        &source.token,
        &source.languages,
        &record_search::case_path(about),
    )
    .map_err(|e| e.words(w).unwrap_or_default())?;
    if a.status != 200 {
        return Err(tafs_core::signin::refusal(&a, Some(w))
            .words(w)
            .unwrap_or_default());
    }
    RecordCase::read(&a.body)
        .ok_or_else(|| tafs_core::signin::plain_words(w, 200).unwrap_or_default())
}

/// A search's run, by itself too: More continues it, and a refused cursor starts it again.
type Run = Rc<dyn Fn(String, bool)>;
/// The run, held weakly by itself, to search again from the top.
type Again = Rc<RefCell<Option<std::rc::Weak<dyn Fn(String, bool)>>>>;
/// The results' redraw, set once it's made.
type Redraw = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// The search and what shows it: the box, and where results go.
pub struct SearchUi {
    pub entry: gtk::SearchEntry,
    pub results: gtk::Box,
}

/// The box and the results' place. `showing` is told whether a search stands in for the list.
pub fn build(w: Rc<WordBook>, source: Rc<SearchSource>, showing: Rc<dyn Fn(bool)>) -> SearchUi {
    let entry = gtk::SearchEntry::new();
    entry.set_placeholder_text(Some(&w.line("record.search_hint").unwrap_or_default()));
    // What's typed is searched once the person stops typing.
    entry.set_search_delay(record_search::TYPING_PAUSE.as_millis() as u32);
    entry.add_css_class("tafs-record-search");
    entry.set_widget_name("record-search");
    let results = gtk::Box::new(gtk::Orientation::Vertical, 10);
    results.set_visible(false);

    let search: Rc<RefCell<Option<Search>>> = Rc::new(RefCell::new(None));
    // Each search's own number: a late page of an earlier one is dropped.
    let generation = Rc::new(Cell::new(0u64));

    let draw: Redraw = Rc::new(RefCell::new(None));
    // Weak: the run holds this, so a strong one would keep both alive for good.
    let again: Again = Rc::new(RefCell::new(None));
    let run: Run = {
        let (w, source, search, generation, draw, again) = (
            w.clone(),
            source.clone(),
            search.clone(),
            generation.clone(),
            draw.clone(),
            again.clone(),
        );
        Rc::new(move |asked: String, more: bool| {
            let cursor = {
                let s = search.borrow();
                let current = s.as_ref().filter(|s| s.q == asked);
                if more {
                    match current {
                        Some(c) if !c.loading && c.next_cursor().is_some() => {
                            c.next_cursor().map(str::to_string)
                        }
                        _ => return,
                    }
                } else {
                    None
                }
            };
            {
                let mut s = search.borrow_mut();
                let mut started = match (more, s.take()) {
                    (true, Some(c)) => c,
                    _ => Search {
                        q: asked.clone(),
                        ..Default::default()
                    },
                };
                started.loading = true;
                started.error = None;
                *s = Some(started);
            }
            generation.set(generation.get() + 1);
            let mine = generation.get();
            if let Some(d) = draw.borrow().clone() {
                d();
            }
            let (w, source, search, generation, draw, asked2, again) = (
                w.clone(),
                source.clone(),
                search.clone(),
                generation.clone(),
                draw.clone(),
                asked.clone(),
                again.clone(),
            );
            glib::spawn_future_local(async move {
                let words: WordBook = (*w).clone();
                let src: SearchSource = (*source).clone();
                let (q, c) = (asked2.clone(), cursor.clone());
                let got =
                    gio::spawn_blocking(move || page_of(&words, &src, &q, c.as_deref(), None))
                        .await
                        .unwrap_or_else(|_| Err((0, String::new())));
                if generation.get() != mine {
                    return;
                }
                // A cursor tafs cloud refuses (409: a call made since) searches again from the top.
                if matches!(&got, Err((409, _))) && cursor.is_some() {
                    if let Some(s) = search.borrow_mut().as_mut() {
                        s.loading = false;
                    }
                    let again = again.borrow().as_ref().and_then(std::rc::Weak::upgrade);
                    if let Some(again) = again {
                        again(asked2, false);
                    }
                    return;
                }
                {
                    let mut s = search.borrow_mut();
                    let Some(now) = s.as_mut().filter(|s| s.q == asked2) else {
                        return;
                    };
                    now.loading = false;
                    match got {
                        Ok(page) => {
                            if cursor.is_none() {
                                now.pages.clear();
                            }
                            now.pages.push(page);
                        }
                        Err((_, why)) => now.error = Some(why),
                    }
                }
                if let Some(d) = draw.borrow().clone() {
                    d();
                }
            });
        })
    };
    *again.borrow_mut() = Some(Rc::downgrade(&run));

    // The results, drawn again whenever the search changes.
    {
        let (w, source, search, results2, run) = (
            w.clone(),
            source.clone(),
            search.clone(),
            results.clone(),
            run.clone(),
        );
        *draw.borrow_mut() = Some(Rc::new(move || {
            while let Some(c) = results2.first_child() {
                results2.remove(&c);
            }
            let s = search.borrow();
            let Some(s) = s.as_ref() else {
                return;
            };
            for page in &s.pages {
                for group in page.shown_groups() {
                    if !group.title.is_empty() {
                        let t = text(&group.title, 12, 600, &["tafs-dim"]);
                        t.set_margin_top(4);
                        results2.append(&t);
                    }
                    for hit in &group.entries {
                        results2.append(&hit_row(&w, &source, hit, true));
                    }
                }
            }
            if let Some(none) = s.none_words() {
                results2.append(&text(none, 14, 400, &["tafs-soft"]));
            }
            if let Some(e) = &s.error {
                results2.append(&text(e, 13, 400, &["tafs-no"]));
            }
            if s.loading {
                let spinner = gtk::Spinner::new();
                spinner.start();
                results2.append(&spinner);
            } else if s.next_cursor().is_some() {
                let more = outline(&w.line("common.more").unwrap_or_default());
                let (run, q) = (run.clone(), s.q.clone());
                more.connect_clicked(move |_| run(q.clone(), true));
                results2.append(&more);
            } else if let Some(end) = s.end_words() {
                results2.append(&text(end, 13, 400, &["tafs-dim"]));
            }
        }));
    }

    // Typing: once the person stops, what's typed; emptied, the Record again at once.
    {
        let (run, search, results, showing) = (
            run.clone(),
            search.clone(),
            results.clone(),
            showing.clone(),
        );
        entry.connect_search_changed(move |e| match record_search::asked(&e.text()) {
            Some(asked) => {
                if search.borrow().as_ref().map(|s| s.q.as_str()) == Some(asked.as_str()) {
                    return;
                }
                results.set_visible(true);
                showing(true);
                run(asked, false);
            }
            None => {
                *search.borrow_mut() = None;
                results.set_visible(false);
                showing(false);
            }
        });
    }
    // Enter: what's typed, at once.
    {
        let (run, results, showing) = (run.clone(), results.clone(), showing.clone());
        entry.connect_activate(move |e| {
            if let Some(asked) = record_search::asked(&e.text()) {
                results.set_visible(true);
                showing(true);
                run(asked, false);
            }
        });
    }
    // The box's words, read once from tafs cloud; the app's own stand in until they come.
    {
        let (w, source, entry) = (w.clone(), source.clone(), entry.clone());
        glib::spawn_future_local(async move {
            let words: WordBook = (*w).clone();
            let src: SearchSource = (*source).clone();
            let hint = gio::spawn_blocking(move || page_of(&words, &src, "", None, Some(1)))
                .await
                .ok()
                .and_then(Result::ok)
                .map(|p| p.hint)
                .filter(|h| !h.trim().is_empty());
            if let Some(hint) = hint {
                entry.set_placeholder_text(Some(&hint));
            }
        });
    }
    if let Some(q) = &source.initial {
        entry.set_text(q);
    }
    SearchUi { entry, results }
}

/// `RecordHitRow`: its taf's name (11, mono, upper case), the call as its taf's Activity draws it,
/// and what it's about, which opens the case (amber, with ›); off in a case.
fn hit_row(
    w: &Rc<WordBook>,
    source: &Rc<SearchSource>,
    hit: &RecordHit,
    opens_case: bool,
) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    let name = text(
        &hit.taf_name.to_uppercase(),
        11,
        500,
        &["tafs-dim", "tafs-mono"],
    );
    v.append(&name);
    let p = panel(false);
    p.append(&crate::taf::source_row(&hit.entry));
    v.append(&p);
    if let (true, Some(about)) = (opens_case, &hit.about) {
        let b = gtk::Button::with_label(&format!("{about} ›"));
        b.add_css_class("tafs-link");
        b.set_halign(gtk::Align::Start);
        let (w, source, about) = (w.clone(), source.clone(), about.clone());
        b.connect_clicked(move |b| case_sheet(&w, &source, &about, b));
        v.append(&b);
    }
    v
}

/// `RecordCaseSheet`: what it's about (24 bold), the step that didn't go through in red, then every
/// call about it, oldest first, each as its taf's Activity draws it; Done at the top.
fn case_sheet(
    w: &Rc<WordBook>,
    source: &Rc<SearchSource>,
    about: &str,
    parent: &impl IsA<gtk::Widget>,
) {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(16);
    column.set_margin_bottom(16);
    column.append(&text(about, 24, 700, &["tafs-ink"]));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 10);
    let spinner = gtk::Spinner::new();
    spinner.start();
    body.append(&spinner);
    column.append(&body);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    let header = adw::HeaderBar::new();
    header.set_show_end_title_buttons(false);
    header.set_show_start_title_buttons(false);
    let done = gtk::Button::with_label(&w.line("common.done").unwrap_or_default());
    done.add_css_class("flat");
    header.pack_start(&done);
    header.set_title_widget(Some(&gtk::Label::new(None)));
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(600);
    dialog.set_content_height(640);
    dialog.set_title(about);
    {
        let dialog = dialog.clone();
        done.connect_clicked(move |_| {
            dialog.close();
        });
    }
    dialog.present(Some(parent));
    let (w, source, about) = (w.clone(), source.clone(), about.to_string());
    glib::spawn_future_local(async move {
        let words: WordBook = (*w).clone();
        let src: SearchSource = (*source).clone();
        let a = about.clone();
        let read = gio::spawn_blocking(move || case_of(&words, &src, &a))
            .await
            .unwrap_or_else(|_| Err(String::new()));
        while let Some(c) = body.first_child() {
            body.remove(&c);
        }
        match read {
            Ok(case) => {
                if let Some(words) = case.failed_words() {
                    body.append(&text(words, 15, 600, &["tafs-no"]));
                }
                for step in &case.steps {
                    body.append(&hit_row(&w, &source, step, false));
                }
            }
            Err(why) => body.append(&text(&why, 14, 400, &["tafs-no"])),
        }
    });
}
