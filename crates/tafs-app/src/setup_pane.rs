//! A tafs cloud taf's Setup tab (`TafDetailView.setup(_:)`), signed in with KIFF: its Card unit
//! requests, its own Connect again (signed: drawn up to the signature), its job with Improve in
//! Coach, Starts when, its connected apps (`ConnectedToolsRing` and `ConnectedAppSheet`),
//! Results go to, what it remembers (`MemorySummaryPanel`, and `MemoryPanel` behind See all, where
//! a note is forgotten), and Remove for good, dimmed. tafs cloud's Setup read
//! (`GET /v1/tafs/{id}/setup`) is made without planque's grants proof, which this computer can't
//! sign, so tafs cloud sends why it left the connections out. The reads are
//! `tafs-core/src/setup.rs`; Starts when and Results go to are `starts_when.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::setup::{self, Note, SetupView};
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, outline, panel, text};

/// Something to call again later: the Setup's read, a sheet's redraw.
type Later = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// Where a taf's Setup and notes are read, and a note forgotten.
#[derive(Clone)]
pub enum SetupSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answers: the Setup read, and the notes (`--memory`) when given. Nothing is
    /// sent, and Forget changes nothing.
    File {
        setup: std::path::PathBuf,
        notes: Option<std::path::PathBuf>,
    },
    None,
}

/// The taf the pane is for: tafs cloud's id, its job as Home says it, whether this person
/// manages it, and the Card unit requests' Coach button (`mayUseCoach` and `manages`).
pub struct SetupTaf {
    pub id: String,
    pub name: String,
    pub initial: String,
    pub color: i64,
    pub job: String,
    pub manages: bool,
    pub discuss: bool,
    pub may_use_coach: bool,
    /// Starts when's and Results go to's dial, signed in with KIFF.
    pub knobs: Option<crate::starts_when::Knobs>,
    /// Coach (Improve in Coach, Choose actions), Connect apps (the ring's +), and the taf's
    /// Activity (an app's calls).
    pub open_coach: Rc<dyn Fn()>,
    pub open_connect: Rc<dyn Fn()>,
    pub open_activity: Rc<dyn Fn()>,
}

fn file_answer(path: &std::path::Path) -> Option<Vec<u8>> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    serde_json::to_vec(v.get("answer").unwrap_or(&v)).ok()
}

async fn read_setup(w: &WordBook, source: &SetupSource, id: &str) -> Result<SetupView, String> {
    let (words, source, id) = (w.clone(), source.clone(), id.to_string());
    gio::spawn_blocking(move || {
        let body = match &source {
            SetupSource::Cloud { token, languages } => {
                let a = crate::kiff::cloud(token, languages, &setup::path(&id))
                    .map_err(|e| e.words(&words).unwrap_or_default())?;
                if a.status != 200 {
                    return Err(tafs_core::signin::refusal(&a, Some(&words))
                        .words(&words)
                        .unwrap_or_default());
                }
                a.body
            }
            SetupSource::File { setup, .. } => file_answer(setup).ok_or_default()?,
            SetupSource::None => return Err(String::new()),
        };
        setup::read(&body)
            .ok_or_else(|| tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
    })
    .await
    .unwrap_or_else(|_| Err(String::new()))
}

trait OkOrDefault<T> {
    fn ok_or_default(self) -> Result<T, String>;
}

impl<T> OkOrDefault<T> for Option<T> {
    fn ok_or_default(self) -> Result<T, String> {
        self.ok_or_else(String::new)
    }
}

async fn read_notes(w: &WordBook, source: &SetupSource, id: &str) -> Result<Vec<Note>, String> {
    let (words, source, id) = (w.clone(), source.clone(), id.to_string());
    let read = gio::spawn_blocking(move || {
        let body = match &source {
            SetupSource::Cloud { token, languages } => {
                let a = crate::kiff::cloud(token, languages, &setup::memory_path(&id, true))
                    .map_err(|e| e.words(&words).unwrap_or_default())?;
                if a.status != 200 {
                    return Err(tafs_core::signin::refusal(&a, Some(&words))
                        .words(&words)
                        .unwrap_or_default());
                }
                a.body
            }
            SetupSource::File {
                notes: Some(notes), ..
            } => file_answer(notes).ok_or_default()?,
            _ => return Err(String::new()),
        };
        setup::read_notes(&body)
            .ok_or_else(|| tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
    })
    .await
    .unwrap_or_else(|_| Err(String::new()));
    read.map_err(|why| {
        w.get("taf.memory.not_loaded", &[("why", &why)])
            .unwrap_or(why)
    })
}

/// The Setup tab's pane: read when it's first shown, and again by Try again.
pub fn pane(w: Rc<WordBook>, taf: Rc<SetupTaf>, source: SetupSource) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 16);
    root.set_margin_top(4);
    root.set_margin_bottom(12);
    let load: Later = Rc::new(RefCell::new(None));
    let load_now: Rc<dyn Fn()> = {
        let (w, taf, source, root, load) = (
            w.clone(),
            taf.clone(),
            source.clone(),
            root.clone(),
            load.clone(),
        );
        Rc::new(move || {
            while let Some(c) = root.first_child() {
                root.remove(&c);
            }
            let spinner = gtk::Spinner::new();
            spinner.start();
            spinner.set_halign(gtk::Align::Start);
            root.append(&spinner);
            let (w, taf, source, root, load) = (
                w.clone(),
                taf.clone(),
                source.clone(),
                root.clone(),
                load.clone(),
            );
            glib::spawn_future_local(async move {
                let read = read_setup(&w, &source, &taf.id).await;
                while let Some(c) = root.first_child() {
                    root.remove(&c);
                }
                draw(&w, &taf, &source, &root, read, &load);
            });
        })
    };
    *load.borrow_mut() = Some(load_now.clone());
    load_now();
    root
}

fn draw(
    w: &Rc<WordBook>,
    taf: &Rc<SetupTaf>,
    source: &SetupSource,
    root: &gtk::Box,
    read: Result<SetupView, String>,
    load: &Later,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let view = match read {
        Ok(view) => Some(view),
        Err(why) => {
            // Why it couldn't be read, and Try again (the tools panel's, `tafSetupErrors`).
            root.append(&text(&why, 14, 400, &["tafs-no"]));
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
            root.append(&again);
            None
        }
    };
    // Its Card unit requests first, as Needs you draws them.
    for ask in view
        .as_ref()
        .and_then(|v| v.card_unit_exceptions.as_ref())
        .into_iter()
        .flatten()
    {
        let lines = tafs_core::needs_you::card_unit_lines(ask, taf.discuss);
        root.append(&crate::needs_you::card_unit_panel(&lines));
    }
    // Its own Connect again, first: that same connection signed in again, behind the device's
    // approval, which this computer can't give.
    if let Some(again) = view
        .as_ref()
        .and_then(|v| v.connect_again.as_ref())
        .filter(|a| a.usable() && taf.manages)
    {
        let p = panel(false);
        p.add_css_class("knocks-edge");
        p.append(&text(&again.words, 14, 600, &["tafs-amber-text"]));
        let b = crate::widgets::primary(&again.title);
        let slot = crate::approval::refusal_slot();
        let (w2, grant, slot2) = (w.clone(), again.grant.clone(), slot.clone());
        b.connect_clicked(move |_| {
            crate::approval::refuse(
                &w2,
                tafs_core::sidebar::Mode::Kiff,
                &tafs_core::signed::Act::ConnectAgain {
                    grant: grant.clone(),
                },
                &slot2,
            )
        });
        p.append(&b);
        p.append(&slot);
        root.append(&p);
    }
    // The job, in Home's words, and Improve in Coach for whoever manages it and may use Coach.
    let job = panel(false);
    job.append(&eyebrow(&l("setup.job"), false));
    job.append(&text(&taf.job, 15, 400, &["tafs-ink"]));
    if taf.manages && taf.may_use_coach {
        let b = gtk::Button::with_label(&l("setup.improve_in_coach"));
        b.add_css_class("tafs-link");
        b.add_css_class("tafs-big-link");
        b.set_halign(gtk::Align::Start);
        let open = taf.open_coach.clone();
        b.connect_clicked(move |_| open());
        job.append(&b);
    }
    root.append(&job);
    if let Some(knobs) = &taf.knobs {
        root.append(&screen_link(
            &l("setup.starts_when"),
            "tafs-clock-symbolic",
            knobs,
            crate::starts_when::Screen::StartsWhen,
        ));
    }
    root.append(&tools_ring(w, taf, view.as_ref(), load));
    if let Some(knobs) = &taf.knobs {
        root.append(&screen_link(
            &l("setup.results_go_to"),
            "mail-send-symbolic",
            knobs,
            crate::starts_when::Screen::ResultsGoTo,
        ));
    }
    // What it remembers (`MemorySummaryPanel`): tafs cloud's line for it, its latest note, See all.
    if let Some(view) = &view {
        let memory = panel(false);
        memory.append(&eyebrow(&l("taf.memory.title"), false));
        memory.append(&text(&view.memory.title, 15, 600, &["tafs-ink"]));
        if let Some(latest) = view.memory.latest.as_ref().filter(|s| !s.is_empty()) {
            let line = text(latest, 14, 400, &["tafs-soft"]);
            line.set_lines(2);
            line.set_ellipsize(gtk::pango::EllipsizeMode::End);
            memory.append(&line);
        }
        let all = gtk::Button::with_label(&l("taf.memory.see_all"));
        all.add_css_class("tafs-link");
        all.set_halign(gtk::Align::Start);
        {
            let (w, taf, source, load) = (w.clone(), taf.clone(), source.clone(), load.clone());
            all.connect_clicked(move |b| notes_sheet(&w, &taf, &source, b.upcast_ref(), &load));
        }
        memory.append(&all);
        root.append(&memory);
    }
    // Remove for good, for whoever manages it: signed, so it's there, dimmed.
    if taf.manages {
        let remove = outline(&l("setup.remove_for_good"));
        remove.add_css_class("tafs-no");
        remove.set_sensitive(false);
        root.append(&remove);
    }
}

/// `PanelLink`: a row that opens one of Setup's screens, its mark, words and caret.
fn screen_link(
    words: &str,
    icon: &str,
    knobs: &crate::starts_when::Knobs,
    screen: crate::starts_when::Screen,
) -> gtk::Button {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.append(&crate::widgets::symbol(icon, 16));
    let label = text(words, 15, 600, &["tafs-amber-text"]);
    label.set_hexpand(true);
    row.append(&label);
    row.append(&crate::widgets::symbol("go-next-symbolic", 14));
    let b = gtk::Button::new();
    b.set_child(Some(&row));
    b.add_css_class("tafs-panel-link");
    let knobs = knobs.clone();
    b.connect_clicked(move |b| crate::starts_when::open(knobs.clone(), screen, b));
    b
}

/// `ConnectedToolsRing`: the taf in the middle, each app it's connected to around it (one node
/// per app, `nodes`), a + to connect one for whoever manages it; then its workflow, folded. Why
/// the connections couldn't be read, with Try again. The ring's lines are shapes; its words are
/// buttons over it.
fn tools_ring(
    w: &Rc<WordBook>,
    taf: &Rc<SetupTaf>,
    view: Option<&SetupView>,
    load: &Later,
) -> gtk::Box {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let p = panel(false);
    p.append(&eyebrow(&l("connect.connected_tools"), false));
    if let Some(error) = view.and_then(|v| v.connections_error.clone()) {
        p.append(&text(&error, 13, 400, &["tafs-no"]));
        let again = gtk::Button::with_label(&l("common.try_again"));
        again.add_css_class("tafs-link");
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
    let Some(view) = view else {
        return p;
    };
    let nodes = setup::nodes(&view.connections);
    let count = nodes.len() + usize::from(taf.manages);
    let fixed = gtk::Fixed::new();
    let area = gtk::DrawingArea::new();
    area.set_content_height(280);
    area.set_hexpand(true);
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&area));
    overlay.add_overlay(&fixed);
    let running: Vec<bool> = nodes.iter().map(|n| n.app.running).collect();
    area.set_draw_func(move |_, cr, width, height| {
        let (cx, cy) = (width as f64 / 2.0, height as f64 / 2.0);
        let r = (width.min(height) as f64) / 2.0 - 36.0;
        for (i, running) in running.iter().enumerate() {
            let a = -std::f64::consts::FRAC_PI_2
                + i as f64 * 2.0 * std::f64::consts::PI / count.max(1) as f64;
            cr.move_to(cx, cy);
            cr.line_to(cx + a.cos() * r, cy + a.sin() * r);
            let (hex, alpha) = line_colour();
            crate::illustrations::rgb(cr, hex, alpha);
            cr.set_line_width(1.0);
            cr.set_dash(&[4.0, 4.0], 0.0);
            let _ = cr.stroke();
            cr.set_dash(&[], 0.0);
            if *running {
                cr.new_sub_path();
                cr.arc(cx, cy, r, a, a + 0.1 * 2.0 * std::f64::consts::PI);
                crate::illustrations::rgb(cr, crate::illustrations::tone("ok"), 1.0);
                cr.set_line_width(3.0);
                cr.set_line_cap(gtk::cairo::LineCap::Round);
                let _ = cr.stroke();
            }
        }
    });
    area.update_state(&[gtk::accessible::State::Hidden(true)]);
    let mut widgets: Vec<gtk::Widget> = Vec::new();
    for node in &nodes {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
        let name = text(&node.app.name, 13, 600, &["tafs-ink"]);
        name.set_xalign(0.5);
        name.set_justify(gtk::Justification::Center);
        name.set_lines(2);
        name.set_ellipsize(gtk::pango::EllipsizeMode::End);
        v.append(&name);
        let actions = text(
            &node.app.action_words,
            11,
            400,
            &[if node.app.running {
                "tafs-ok"
            } else {
                "tafs-soft"
            }],
        );
        actions.set_xalign(0.5);
        actions.set_justify(gtk::Justification::Center);
        actions.set_lines(2);
        actions.set_ellipsize(gtk::pango::EllipsizeMode::End);
        v.append(&actions);
        let b = gtk::Button::new();
        b.set_child(Some(&v));
        b.add_css_class("tafs-ring-node");
        b.set_size_request(88, -1);
        let (w, taf, node, load) = (w.clone(), taf.clone(), node.clone(), load.clone());
        b.connect_clicked(move |b| app_sheet(&w, &taf, &node, b.upcast_ref(), &load));
        widgets.push(b.upcast());
    }
    if taf.manages {
        let plus = gtk::Button::from_icon_name("list-add-symbolic");
        plus.add_css_class("tafs-ring-plus");
        plus.set_size_request(46, 46);
        plus.update_property(&[gtk::accessible::Property::Label(&l(
            "connect.connect_an_app",
        ))]);
        let open = taf.open_connect.clone();
        plus.connect_clicked(move |_| open());
        widgets.push(plus.upcast());
    }
    let centre = crate::widgets::avatar(&taf.initial, taf.color, 54);
    let job = gtk::Button::new();
    job.set_child(Some(&centre));
    job.add_css_class("tafs-plain");
    job.update_property(&[gtk::accessible::Property::Label(
        &w.get("taf.job_label", &[("taf", &taf.name)])
            .unwrap_or_default(),
    )]);
    {
        let (w, taf) = (w.clone(), taf.clone());
        job.connect_clicked(move |b| {
            let a = adw::AlertDialog::new(Some(&taf.name), Some(&taf.job));
            a.add_response("done", &w.line("common.done").unwrap_or_default());
            a.present(Some(b));
        });
    }
    for x in &widgets {
        fixed.put(x, 0.0, 0.0);
    }
    fixed.put(&job, 0.0, 0.0);
    {
        let (fixed, widgets, job) = (fixed.downgrade(), widgets.clone(), job.downgrade());
        area.connect_resize(move |_, width, height| {
            let (Some(fixed), Some(job)) = (fixed.upgrade(), job.upgrade()) else {
                return;
            };
            let (cx, cy) = (width as f64 / 2.0, height as f64 / 2.0);
            let r = (width.min(height) as f64) / 2.0 - 36.0;
            let put = |x: &gtk::Widget, px: f64, py: f64| {
                let (_, nat) = x.preferred_size();
                fixed.move_(
                    x,
                    px - nat.width() as f64 / 2.0,
                    py - nat.height() as f64 / 2.0,
                );
            };
            for (i, x) in widgets.iter().enumerate() {
                let a = -std::f64::consts::FRAC_PI_2
                    + i as f64 * 2.0 * std::f64::consts::PI / count.max(1) as f64;
                put(x, cx + a.cos() * r, cy + a.sin() * r);
            }
            put(job.upcast_ref(), cx, cy);
        });
    }
    p.append(&overlay);
    // The workflow, folded: its title and summary; open, each step and Change with Coach.
    let head = gtk::Box::new(gtk::Orientation::Vertical, 2);
    head.append(&text(&view.workflow.title, 15, 400, &["tafs-ink"]));
    head.append(&text(&view.workflow.summary, 13, 400, &["tafs-soft"]));
    let expander = gtk::Expander::new(None);
    expander.set_label_widget(Some(&head));
    let steps = gtk::Box::new(gtk::Orientation::Vertical, 8);
    steps.set_margin_top(8);
    for step in &view.workflow.steps {
        steps.append(&step_row(step));
    }
    if taf.manages {
        let b = gtk::Button::with_label(&l("taf.workflow.change_with_coach"));
        b.add_css_class("tafs-link");
        b.set_halign(gtk::Align::Start);
        let open = taf.open_coach.clone();
        b.connect_clicked(move |_| open());
        steps.append(&b);
    }
    expander.set_child(Some(&steps));
    p.append(&expander);
    p
}

fn line_colour() -> (u32, f64) {
    let t = tafs_core::theme::tone("line").expect("the palette has it");
    let c = if adw::StyleManager::default().is_dark() {
        t.dark
    } else {
        t.light
    };
    (c.0, c.1 as f64)
}

/// A workflow step: its number (green while it runs), its label, words and app.
fn step_row(step: &setup::WorkflowStep) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let n = text(
        &step.number.to_string(),
        15,
        400,
        &[if step.running { "tafs-ok" } else { "tafs-soft" }],
    );
    n.set_valign(gtk::Align::Start);
    row.append(&n);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
    v.set_hexpand(true);
    v.append(&text(&step.label, 15, 600, &["tafs-ink"]));
    if !step.words.is_empty() {
        v.append(&text(&step.words, 13, 400, &["tafs-soft"]));
    }
    if let Some(app) = &step.app {
        v.append(&text(app, 12, 400, &["tafs-amber-text"]));
    }
    row.append(&v);
    row
}

/// `ConnectedAppSheet`: a connected app on this taf: where it stands, what it may do, its part of
/// the workflow, its calls (opening the taf's Activity), its other sign-ins. Signing in again
/// and removing access are signed at planque: drawn up to the signature, after Remove access's
/// question. Add actions opens Coach.
fn app_sheet(
    w: &Rc<WordBook>,
    taf: &Rc<SetupTaf>,
    node: &setup::Node,
    parent: &gtk::Widget,
    load: &Later,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let app = &node.app;
    let s = crate::sheet::new(&app.name, &l("common.done"), None, 520);
    let slot = crate::approval::refusal_slot();
    let section = |title: Option<&str>| {
        if let Some(t) = title {
            s.column.append(&crate::sheet::header(t));
        }
        let b = panel(false);
        s.column.append(&b);
        b
    };
    let refuse = |act: tafs_core::signed::Act| {
        let (w, slot) = (w.clone(), slot.clone());
        move || {
            crate::approval::refuse(&w, tafs_core::sidebar::Mode::Kiff, &act, &slot);
        }
    };
    let first = section(None);
    first.append(&text(
        &app.status_words,
        15,
        400,
        &[if app.reconnect {
            "tafs-amber-text"
        } else {
            "tafs-soft"
        }],
    ));
    if app.reconnect && taf.manages {
        let b = gtk::Button::with_label(&l("connect.sign_in_again"));
        b.add_css_class("tafs-link");
        b.set_halign(gtk::Align::Start);
        let go = refuse(tafs_core::signed::Act::ConnectAgain {
            grant: app.id.clone(),
        });
        b.connect_clicked(move |_| go());
        first.append(&b);
    }
    let may = section(Some(&l("taf.app.what_it_may_do")));
    for action in &app.actions {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 5);
        v.append(&text(&action.title, 16, 600, &["tafs-ink"]));
        for line in &action.limits {
            v.append(&text(line, 13, 400, &["tafs-soft"]));
        }
        if !action.step_words.is_empty() {
            v.append(&text(&action.step_words, 13, 400, &["tafs-dim"]));
        }
        may.append(&v);
    }
    let flow = section(Some(&l("taf.workflow.title")));
    if let Some(words) = &app.workflow_words {
        flow.append(&text(words, 15, 400, &["tafs-dim"]));
    }
    for step in &app.workflow_steps {
        let line = if step.words.is_empty() {
            format!("{} · {}", step.number, step.label)
        } else {
            format!("{} · {}\n{}", step.number, step.label, step.words)
        };
        flow.append(&text(&line, 15, 400, &["tafs-ink"]));
    }
    let stats = section(None);
    let calls = gtk::Button::with_label(&app.stats.words);
    calls.add_css_class("tafs-link");
    calls.set_halign(gtk::Align::Start);
    calls.set_sensitive(!app.actions.is_empty());
    {
        let (open, d) = (taf.open_activity.clone(), s.dialog.downgrade());
        calls.connect_clicked(move |_| {
            if let Some(d) = d.upgrade() {
                d.close();
            }
            open();
        });
    }
    stats.append(&calls);
    if !node.others.is_empty() {
        let others = section(Some(&l("connect.other_connections")));
        for other in &node.others {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let words = text(
                &other.status_words,
                15,
                400,
                &[if other.reconnect {
                    "tafs-amber-text"
                } else {
                    "tafs-soft"
                }],
            );
            words.set_hexpand(true);
            row.append(&words);
            if taf.manages {
                let b = gtk::Button::with_label(&l("connect.disconnect_button"));
                b.add_css_class("tafs-link");
                b.add_css_class("tafs-no");
                let ask = disconnect_question(w, &app.name, other.id.clone(), slot.clone());
                b.connect_clicked(move |b| ask(b.upcast_ref()));
                row.append(&b);
            }
            others.append(&row);
        }
    }
    if taf.manages {
        let acts = section(None);
        let add = gtk::Button::with_label(&l("taf.app.add_actions"));
        add.add_css_class("tafs-link");
        add.set_halign(gtk::Align::Start);
        {
            let (open, d) = (taf.open_coach.clone(), s.dialog.downgrade());
            add.connect_clicked(move |_| {
                if let Some(d) = d.upgrade() {
                    d.close();
                }
                open();
            });
        }
        acts.append(&add);
        let remove = gtk::Button::with_label(&l("connect.disconnect_button"));
        remove.add_css_class("tafs-link");
        remove.add_css_class("tafs-no");
        remove.set_halign(gtk::Align::Start);
        let ask = disconnect_question(w, &app.name, app.id.clone(), slot.clone());
        remove.connect_clicked(move |b| ask(b.upcast_ref()));
        acts.append(&remove);
    }
    s.column.append(&slot);
    {
        let load = load.clone();
        s.dialog.connect_closed(move |_| {
            let f = load.borrow().clone();
            if let Some(f) = f {
                f();
            }
        });
    }
    s.dialog.present(Some(parent));
}

/// Remove access's question; its yes would end the connection at planque, signed by the device:
/// this computer says it can't approve, and the connection stays.
fn disconnect_question(
    w: &Rc<WordBook>,
    app: &str,
    grant: String,
    slot: gtk::Box,
) -> impl Fn(&gtk::Widget) {
    let w = w.clone();
    let app = app.to_string();
    move |parent| {
        let l = |k: &str| w.line(k).unwrap_or_default();
        let ask = adw::AlertDialog::new(
            Some(
                &w.get("connect.disconnect_app_question", &[("app", &app)])
                    .unwrap_or_default(),
            ),
            Some(&l("connect.disconnect_body")),
        );
        ask.add_response("cancel", &l("common.cancel"));
        ask.add_response("remove", &l("connect.disconnect_button"));
        ask.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        ask.set_default_response(Some("cancel"));
        ask.set_close_response("cancel");
        let (w, grant, slot) = (w.clone(), grant.clone(), slot.clone());
        ask.connect_response(None, move |_, r| {
            if r == "remove" {
                crate::approval::refuse(
                    &w,
                    tafs_core::sidebar::Mode::Kiff,
                    &tafs_core::signed::Act::Disconnect {
                        grant: grant.clone(),
                    },
                    &slot,
                );
            }
        });
        ask.present(Some(parent));
    }
}

/// `MemoryPanel(loadAll: true)` behind See all: every note, newest first, each with Forget, which
/// asks first; then what the notes are for. The Setup read again once it's closed.
fn notes_sheet(
    w: &Rc<WordBook>,
    taf: &Rc<SetupTaf>,
    source: &SetupSource,
    parent: &gtk::Widget,
    load: &Later,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(12);
    column.set_margin_bottom(20);
    let p = panel(false);
    column.append(&p);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_title(&l("taf.memory.title"));
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(560);
    dialog.set_content_height(600);
    {
        let load = load.clone();
        dialog.connect_closed(move |_| {
            let f = load.borrow().clone();
            if let Some(f) = f {
                f();
            }
        });
    }
    let notes: Rc<RefCell<Option<Vec<Note>>>> = Rc::new(RefCell::new(None));
    let error: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let redraw: Later = Rc::new(RefCell::new(None));
    let draw: Rc<dyn Fn()> = {
        let (w, taf, source, p, notes, error, redraw, dialog) = (
            w.clone(),
            taf.clone(),
            source.clone(),
            p.clone(),
            notes.clone(),
            error.clone(),
            redraw.clone(),
            dialog.clone(),
        );
        Rc::new(move || {
            let l = |k: &str| w.line(k).unwrap_or_default();
            while let Some(c) = p.first_child() {
                p.remove(&c);
            }
            p.append(&eyebrow(&l("taf.memory.title"), false));
            let read = notes.borrow().clone();
            match &read {
                Some(list) => {
                    if list.is_empty() {
                        p.append(&text(&l("taf.memory.empty"), 14, 400, &["tafs-soft"]));
                    }
                    for note in list {
                        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                        let v = gtk::Box::new(gtk::Orientation::Vertical, 3);
                        v.set_hexpand(true);
                        v.append(&text(&note.content, 15, 400, &["tafs-ink"]));
                        let when = note
                            .updated_at
                            .with_timezone(&chrono::Local)
                            .format("%H:%M")
                            .to_string();
                        v.append(&text(
                            &format!("{} · {when}", note.title()),
                            12,
                            400,
                            &["tafs-soft"],
                        ));
                        row.append(&v);
                        let forget = gtk::Button::with_label(&l("taf.memory.forget"));
                        forget.add_css_class("tafs-plain");
                        forget.add_css_class("tafs-soft");
                        forget.set_valign(gtk::Align::Start);
                        forget.update_property(&[gtk::accessible::Property::Label(
                            &w.get("taf.memory.forget_label", &[("note", &note.content)])
                                .unwrap_or_default(),
                        )]);
                        let (w, taf, source, notes, error, redraw, dialog, note) = (
                            w.clone(),
                            taf.clone(),
                            source.clone(),
                            notes.clone(),
                            error.clone(),
                            redraw.clone(),
                            dialog.clone(),
                            note.clone(),
                        );
                        forget.connect_clicked(move |_| {
                            ask_forget(&w, &taf, &source, &note, &notes, &error, &redraw, &dialog)
                        });
                        row.append(&forget);
                        p.append(&row);
                    }
                    p.append(&text(&l("taf.memory.explains"), 12, 400, &["tafs-soft"]));
                }
                None if error.borrow().is_none() => {
                    let s = gtk::Spinner::new();
                    s.start();
                    s.set_halign(gtk::Align::Start);
                    p.append(&s);
                }
                None => {}
            }
            if let Some(why) = error.borrow().clone() {
                p.append(&text(&why, 13, 400, &["tafs-no"]));
            }
        })
    };
    *redraw.borrow_mut() = Some(draw.clone());
    draw();
    {
        let (w, taf, source, notes, error, draw) = (
            w.clone(),
            taf.clone(),
            source.clone(),
            notes.clone(),
            error.clone(),
            draw.clone(),
        );
        glib::spawn_future_local(async move {
            match read_notes(&w, &source, &taf.id).await {
                Ok(list) => *notes.borrow_mut() = Some(list),
                Err(why) => *error.borrow_mut() = Some(why),
            }
            draw();
        });
    }
    {
        // Breaks the sheet's own cycle once it's gone.
        let redraw = redraw.clone();
        dialog.connect_closed(move |_| {
            redraw.borrow_mut().take();
        });
    }
    dialog.present(Some(parent));
}

/// Forget asks first (`forgetTitle`, the note, Forget in red, Cancel); its yes takes the note away
/// at once and puts it back, with why, when tafs cloud refuses. A note already gone counts as
/// forgotten.
#[allow(clippy::too_many_arguments)]
fn ask_forget(
    w: &Rc<WordBook>,
    taf: &Rc<SetupTaf>,
    source: &SetupSource,
    note: &Note,
    notes: &Rc<RefCell<Option<Vec<Note>>>>,
    error: &Rc<RefCell<Option<String>>>,
    redraw: &Later,
    parent: &adw::Dialog,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let ask = adw::AlertDialog::new(
        Some(&l("taf.memory.forget_title")),
        Some(
            &w.get("taf.memory.forget_body", &[("note", &note.content)])
                .unwrap_or_default(),
        ),
    );
    ask.add_response("cancel", &l("common.cancel"));
    ask.add_response("forget", &l("taf.memory.forget"));
    ask.set_response_appearance("forget", adw::ResponseAppearance::Destructive);
    ask.set_default_response(Some("cancel"));
    ask.set_close_response("cancel");
    let (w, taf, source, note, notes, error, redraw) = (
        w.clone(),
        taf.clone(),
        source.clone(),
        note.clone(),
        notes.clone(),
        error.clone(),
        redraw.clone(),
    );
    ask.connect_response(None, move |_, response| {
        if response != "forget" {
            return;
        }
        let again = || {
            let f = redraw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        };
        if let Some(list) = notes.borrow_mut().as_mut() {
            list.retain(|n| n.id != note.id);
        }
        *error.borrow_mut() = None;
        again();
        let (w, taf, source, note, notes, error, redraw) = (
            w.clone(),
            taf.clone(),
            source.clone(),
            note.clone(),
            notes.clone(),
            error.clone(),
            redraw.clone(),
        );
        glib::spawn_future_local(async move {
            let (words, src, id, nid) = (
                (*w).clone(),
                source.clone(),
                taf.id.clone(),
                note.id.clone(),
            );
            let sent = gio::spawn_blocking(move || -> Result<(), String> {
                match &src {
                    SetupSource::Cloud { token, languages } => {
                        let a = crate::kiff::cloud_send(
                            token,
                            languages,
                            "DELETE",
                            &setup::forget_path(&id, &nid),
                            None,
                        )
                        .map_err(|e| e.words(&words).unwrap_or_default())?;
                        match a.status {
                            200 | 204 | 404 => Ok(()),
                            _ => Err(tafs_core::signin::refusal(&a, Some(&words))
                                .words(&words)
                                .unwrap_or_default()),
                        }
                    }
                    _ => Ok(()),
                }
            })
            .await
            .unwrap_or_else(|_| Err(String::new()));
            if let Err(why) = sent {
                // Put back, newest first.
                if let Some(list) = notes.borrow_mut().as_mut() {
                    list.push(note.clone());
                    list.sort_by_key(|n| std::cmp::Reverse(n.updated_at));
                }
                *error.borrow_mut() = Some(
                    w.get("taf.memory.not_forgotten", &[("why", &why)])
                        .unwrap_or(why),
                );
            }
            let f = redraw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        });
    });
    ask.present(Some(parent));
}
