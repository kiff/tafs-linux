//! A halt of one taf or of every taf (`HaltScreen`, `HaltRow`): its own screen beside the
//! sidebar. Halting is one tap with no signature, since it only takes authority away; it reads
//! Halted only once kiff/agents, KIFF and planque all hold it, and Halting… until then. Resuming is
//! signed by the device, which this computer can't: its button is there, dimmed, and says why.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gio, glib};
use tafs_core::halt::{self, Action, Halt, State};
use tafs_core::sidebar::Selection;
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, outline, panel, symbol, text};

/// Where a halt is read and sent: tafs cloud, as the session. Copied into each read's thread.
#[derive(Clone)]
pub struct HaltSource {
    pub token: String,
    pub languages: Vec<&'static str>,
    /// `--halt`'s answer, for the screenshots: every read is it, and nothing is ever sent.
    pub canned: Option<Halt>,
}

/// A taf's halt by tafs cloud's id and its name, or with none every taf's.
#[derive(Clone)]
pub struct Whose {
    pub id: Option<String>,
    pub name: Option<String>,
}

/// One read (`refreshHalt`): none when it didn't come back readable, and the screen keeps what
/// it had.
fn read(source: &HaltSource, id: Option<&str>) -> Option<Halt> {
    if let Some(h) = &source.canned {
        return Some(h.clone());
    }
    crate::kiff::halt(&source.token, &source.languages, id, false)
        .ok()
        .filter(|a| a.status == 200)
        .and_then(|a| halt::read(&a.body))
}

/// The halt (`halt`): what tafs cloud answers, or why it didn't.
fn send(w: &WordBook, source: &HaltSource, id: Option<&str>) -> Result<Halt, String> {
    let all = id.is_none();
    if let Some(h) = &source.canned {
        return Ok(h.clone());
    }
    let why = |e: tafs_core::signin::KiffError| e.words(w).unwrap_or_default();
    let answer = crate::kiff::halt(&source.token, &source.languages, id, true)
        .map_err(|e| halt::not_halted(w, all, &why(e)).unwrap_or_default())?;
    if !(200..300).contains(&answer.status) {
        let e = tafs_core::signin::refusal(&answer, Some(w));
        return Err(halt::not_halted(w, all, &why(e)).unwrap_or_default());
    }
    halt::read(&answer.body).ok_or_else(|| {
        halt::not_halted(
            w,
            all,
            &tafs_core::signin::plain_words(w, 200).unwrap_or_default(),
        )
        .unwrap_or_default()
    })
}

/// The screen's redraw, set once it's made: the reads and the halt call it.
type Redraw = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// The screen (`HaltScreen`).
pub fn page(w: Rc<WordBook>, source: Rc<HaltSource>, whose: Whose) -> gtk::Widget {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.append(&body);
    let problem = text("", 14, 400, &["tafs-no"]);
    problem.set_visible(false);
    column.append(&problem);

    let state: Rc<RefCell<Halt>> = Rc::new(RefCell::new(Halt::running()));
    let busy = Rc::new(Cell::new(false));
    // A new screen stops the last one's reads (`.task(id:)`).
    let alive = Rc::new(Cell::new(true));
    column.connect_destroy({
        let alive = alive.clone();
        move |_| alive.set(false)
    });

    let draw: Redraw = Rc::new(RefCell::new(None));
    let poll: Rc<dyn Fn()> = {
        let (source, whose, state, draw, alive) = (
            source.clone(),
            whose.clone(),
            state.clone(),
            draw.clone(),
            alive.clone(),
        );
        Rc::new(move || {
            // Read now, then again every 2 seconds while it's halting, at most 60 times.
            let (source, whose, state, draw, alive) = (
                source.clone(),
                whose.clone(),
                state.clone(),
                draw.clone(),
                alive.clone(),
            );
            glib::spawn_future_local(async move {
                for n in 0..=halt::POLLS {
                    if n > 0 {
                        if state.borrow().state != State::Halting {
                            break;
                        }
                        glib::timeout_future(halt::POLL).await;
                    }
                    if !alive.get() {
                        return;
                    }
                    let (s, id) = ((*source).clone(), whose.id.clone());
                    let got = gio::spawn_blocking(move || read(&s, id.as_deref()))
                        .await
                        .ok()
                        .flatten();
                    if !alive.get() {
                        return;
                    }
                    if let Some(h) = got {
                        *state.borrow_mut() = h;
                        if let Some(d) = draw.borrow().clone() {
                            d();
                        }
                    }
                }
            });
        })
    };

    let redraw: Rc<dyn Fn()> = {
        let (w, body, state, busy, whose, source, problem, poll, draw2) = (
            w.clone(),
            body.clone(),
            state.clone(),
            busy.clone(),
            whose.clone(),
            source.clone(),
            problem.clone(),
            poll.clone(),
            draw.clone(),
        );
        Rc::new(move || {
            while let Some(c) = body.first_child() {
                body.remove(&c);
            }
            let h = state.borrow().clone();
            let Ok(l) = halt::lines(&w, &h, whose.name.as_deref()) else {
                return;
            };
            // Board 4, every taf halted (`Illustration.allHalted`): Halt all, once it holds.
            if whose.id.is_none() && h.state == State::Halted {
                body.append(&crate::illustrations::all_halted());
            }
            let head = panel(false);
            let name = eyebrow(&l.name, false);
            name.add_css_class(if l.stopped { "tafs-no" } else { "tafs-dim" });
            head.append(&name);
            head.append(&text(&l.headline, 26, 700, &["tafs-ink"]));
            head.append(&text(&l.explanation, 15, 400, &["tafs-soft"]));
            body.append(&head);
            if let Some((who, parts)) = &l.parts {
                let p = panel(false);
                p.append(&eyebrow(who, false));
                for part in parts {
                    let r = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                    let mark = symbol(
                        if part.held {
                            "emblem-ok-symbolic"
                        } else {
                            "content-loading-symbolic"
                        },
                        16,
                    );
                    mark.add_css_class(if part.held { "tafs-ok" } else { "tafs-dim" });
                    r.append(&mark);
                    let t = text(&part.text, 15, 400, &["tafs-ink"]);
                    t.set_hexpand(true);
                    r.append(&t);
                    r.append(&text(&part.status, 13, 400, &["tafs-dim"]));
                    p.append(&r);
                }
                if let Some(with_all) = &l.with_all {
                    p.append(&text(with_all, 13, 400, &["tafs-dim"]));
                }
                body.append(&p);
            }
            let halt_button = |label: &str| {
                let b = outline(label);
                b.add_css_class("tafs-no");
                b.set_sensitive(!busy.get());
                let (w, source, whose, state, busy, problem, poll, draw2) = (
                    w.clone(),
                    source.clone(),
                    whose.clone(),
                    state.clone(),
                    busy.clone(),
                    problem.clone(),
                    poll.clone(),
                    draw2.clone(),
                );
                b.connect_clicked(move |b| {
                    busy.set(true);
                    b.set_sensitive(false);
                    problem.set_visible(false);
                    let (w, source, whose, state, busy, problem, poll, draw2) = (
                        w.clone(),
                        source.clone(),
                        whose.clone(),
                        state.clone(),
                        busy.clone(),
                        problem.clone(),
                        poll.clone(),
                        draw2.clone(),
                    );
                    glib::spawn_future_local(async move {
                        let words: WordBook = (*w).clone();
                        let id = whose.id.clone();
                        let source: HaltSource = (*source).clone();
                        let sent =
                            gio::spawn_blocking(move || send(&words, &source, id.as_deref()))
                                .await
                                .unwrap_or_else(|_| Err(String::new()));
                        busy.set(false);
                        match sent {
                            Ok(h) => *state.borrow_mut() = h,
                            Err(why) => {
                                problem.set_label(&why);
                                problem.set_visible(!why.is_empty());
                            }
                        }
                        if let Some(d) = draw2.borrow().clone() {
                            d();
                        }
                        // While it's halting, read it every 2 seconds until every part holds it.
                        poll();
                    });
                });
                b
            };
            match &l.action {
                Action::Halt(label) => {
                    let label = if busy.get() {
                        w.line("taf.halt.halting").unwrap_or_default()
                    } else {
                        label.clone()
                    };
                    body.append(&halt_button(&label));
                }
                Action::Halting { wait, again } => {
                    let r = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                    let spinner = gtk::Spinner::new();
                    spinner.start();
                    r.append(&spinner);
                    r.append(&text(wait, 14, 400, &["tafs-dim"]));
                    body.append(&r);
                    // Halt again: a part that didn't take it the first time is asked again.
                    body.append(&halt_button(again));
                }
                Action::Resume { button, why } => {
                    let b = outline(button);
                    b.set_sensitive(false);
                    body.append(&b);
                    body.append(&text(why, 13, 400, &["tafs-soft"]));
                }
                Action::None => {}
            }
        })
    };
    *draw.borrow_mut() = Some(redraw.clone());
    redraw();
    poll();

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    crate::widgets::ground(&scroller);
    scroller.upcast()
}

/// `HaltRow`, framed: a hand in red, "Halt", and "Stop it until you resume" or where the halt
/// stands in red, and a chevron; the whole panel opens its screen. Its state is read when it's
/// drawn.
pub fn row(
    w: Rc<WordBook>,
    source: Rc<HaltSource>,
    whose: Whose,
    open: Rc<dyn Fn(Selection)>,
) -> gtk::Button {
    let p = panel(false);
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let hand = symbol("process-stop-symbolic", 16);
    hand.add_css_class("tafs-no");
    h.append(&hand);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    let all = whose.id.is_none();
    let (title, line, stopped) = halt::row(&w, &Halt::running(), all).unwrap_or_default();
    v.append(&text(&title, 15, 600, &["tafs-ink"]));
    let state = text(&line, 13, 400, &["tafs-dim"]);
    v.append(&state);
    h.append(&v);
    let chevron = symbol("go-next-symbolic", 13);
    chevron.add_css_class("tafs-dim");
    h.append(&chevron);
    p.append(&h);
    let _ = stopped;
    let b = gtk::Button::new();
    b.set_child(Some(&p));
    b.add_css_class("tafs-plain");
    b.set_widget_name("taf.halt");
    {
        let id = whose.id.clone();
        b.connect_clicked(move |_| {
            open(match &id {
                Some(id) => Selection::Halt(id.clone()),
                None => Selection::HaltAll,
            })
        });
    }
    let (w2, alive) = (w.clone(), Rc::new(Cell::new(true)));
    b.connect_destroy({
        let alive = alive.clone();
        move |_| alive.set(false)
    });
    glib::spawn_future_local(async move {
        let id = whose.id.clone();
        let source: HaltSource = (*source).clone();
        let got = gio::spawn_blocking(move || read(&source, id.as_deref()))
            .await
            .ok()
            .flatten();
        if let (true, Some(h)) = (alive.get(), got) {
            if let Ok((_, line, stopped)) = halt::row(&w2, &h, all) {
                state.set_label(&line);
                state.remove_css_class("tafs-dim");
                state.add_css_class(if stopped { "tafs-no" } else { "tafs-dim" });
            }
        }
    });
    b
}
