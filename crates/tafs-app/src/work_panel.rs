//! A tafs cloud taf's work (`WorkPanel`, `TafDetailView.swift`), on its Activity: its line, its
//! health with the fixes this computer can act on, its oldest waiting case, then its requests and
//! skipped firings, newest first, each in tafs cloud's words. Read as the session
//! (`GET /v1/tafs/{id}/work`), and again after a change: for whoever manages the taf, Answer within
//! (`PUT …/promise`) and Send again for a door reply given up on, both unsigned. The lines are
//! `tafs-core/src/work.rs`.

use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::home::Health;
use tafs_core::words::WordBook;
use tafs_core::work::{self, Row, Tone};

use crate::widgets::{eyebrow, outline, panel, text};

/// Where a taf's work is read.
#[derive(Clone)]
pub enum WorkSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answer (`--work`), never sent.
    File(std::path::PathBuf),
    None,
}

/// What the panel opens, and who's looking.
pub struct Opens {
    /// Plan & usage, for a `credits` fix.
    pub usage: Rc<dyn Fn()>,
    /// Coach with words in its box, for a `coach` fix.
    pub coach: Rc<dyn Fn(String)>,
    /// Whoever manages the taf: Answer within and Send again (signed in with KIFF).
    pub manages: bool,
    /// Coach's fix, for whoever may use Coach and manages the taf.
    pub may_use_coach: bool,
    /// Home's steps for this taf, by hand-off: a request's own is drawn from here where listed.
    pub steps: Vec<tafs_core::waits::YourStep>,
}

/// `HealthLines`: each line in the health's colour, with the fixes it can act on: Coach's opens
/// Coach on this taf with tafs cloud's words in the box, nothing sent, for whoever may use Coach
/// and manages it; credits opens Plan & usage.
fn health_lines(h: &Health, opens: &Rc<Opens>) -> Option<gtk::Box> {
    if !h.needs_attention() {
        return None;
    }
    let b = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let class = if h.failing() {
        "tafs-no"
    } else {
        "tafs-amber-text"
    };
    for (i, line) in h.lines.iter().enumerate() {
        b.append(&text(line, 14, 400, &[class]));
        for fix in h.fixes_for(i) {
            let act: Option<Rc<dyn Fn()>> = match fix.kind.as_str() {
                "credits" => {
                    let opens = opens.clone();
                    Some(Rc::new(move || (opens.usage)()))
                }
                "coach" => match fix.say.clone().filter(|s| !s.is_empty()) {
                    Some(say) if opens.may_use_coach && opens.manages => {
                        let opens = opens.clone();
                        Some(Rc::new(move || (opens.coach)(say.clone())))
                    }
                    _ => None,
                },
                _ => None,
            };
            if let Some(act) = act {
                let button = outline(&fix.words);
                button.set_hexpand(false);
                button.set_halign(gtk::Align::Start);
                button.connect_clicked(move |_| act());
                b.append(&button);
            }
        }
    }
    Some(b)
}

/// The panel's state: the taf, where its work is read, the last read, why a change failed
/// (`workErrors`), and the replies being sent again.
struct Panel {
    w: Rc<WordBook>,
    id: String,
    source: WorkSource,
    home_health: Option<Health>,
    opens: Rc<Opens>,
    /// The panel on the page: its buttons keep this, never the other way round.
    root: gtk::glib::WeakRef<gtk::Box>,
    read: std::cell::RefCell<Option<Result<work::Work, String>>>,
    error: std::cell::RefCell<Option<String>>,
    sending: std::cell::RefCell<Vec<String>>,
}

impl Panel {
    fn draw(self: &Rc<Self>) {
        let w = &self.w;
        let Some(root) = self.root.upgrade() else {
            return;
        };
        while let Some(c) = root.first_child() {
            root.remove(&c);
        }
        let read = self.read.borrow().clone();
        let Some(read) = read else { return };
        let p = panel(false);
        p.append(&eyebrow(&w.line("taf.its_work").unwrap_or_default(), false));
        match read {
            Ok(work) => {
                let view = work::panel(&work, self.home_health.as_ref());
                if let Some(line) = &view.line {
                    p.append(&text(line, 15, 400, &["tafs-ink"]));
                }
                if let Some(h) = view
                    .health
                    .as_ref()
                    .and_then(|h| health_lines(h, &self.opens))
                {
                    p.append(&h);
                }
                if let Some(o) = &view.oldest_waiting {
                    p.append(&text(
                        &o.words,
                        14,
                        if o.late { 600 } else { 400 },
                        &[if o.late {
                            "tafs-amber-text"
                        } else {
                            "tafs-soft"
                        }],
                    ));
                }
                if self.opens.manages {
                    p.append(&self.promise_row(work.promise_hours));
                }
                for row in &view.rows {
                    match row {
                        Row::Skipped(s) => p.append(&text(&s.words, 13, 400, &["tafs-dim"])),
                        Row::Request(r) => p.append(&self.request(r)),
                    }
                }
            }
            Err(why) => p.append(&text(&why, 13, 400, &["tafs-no"])),
        }
        if let Some(e) = self.error.borrow().clone() {
            p.append(&text(&e, 13, 400, &["tafs-no"]));
        }
        root.append(&p);
    }

    fn request(self: &Rc<Self>, r: &work::Request) -> gtk::Box {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let class = match work::tone(&r.state) {
            Tone::Amber => "tafs-amber-text",
            Tone::No => "tafs-no",
            Tone::Soft => "tafs-soft",
        };
        // A step the taf handed its owner, in place of its line (`YourStepCard`): Home's, which
        // carries the vendor's link when tafs cloud vouched for it, else the request's own.
        match r.your_step.as_deref() {
            Some(step) => {
                let step = self
                    .opens
                    .steps
                    .iter()
                    .find(|s| s.handoff == step.handoff)
                    .unwrap_or(step);
                v.append(&crate::needs_you::step_card(&self.w, step, true));
            }
            None => v.append(&text(&r.line, 14, 400, &[class])),
        }
        if let Some(used) = &r.credits {
            v.append(&text(used, 12, 400, &["tafs-dim"]));
        }
        for d in &r.deliveries {
            let class = if d.send_again {
                "tafs-amber-text"
            } else {
                "tafs-dim"
            };
            v.append(&text(&d.line, 12, 400, &[class]));
            // A door reply given up on: Send again, for whoever manages the taf.
            if d.send_again && self.opens.manages {
                let sending = self.sending.borrow().contains(&r.id);
                let b = outline(
                    &self
                        .w
                        .line(if sending {
                            "work.sending_again"
                        } else {
                            "work.send_again"
                        })
                        .unwrap_or_default(),
                );
                b.set_sensitive(!sending);
                let (me, run, place) = (self.clone(), r.id.clone(), d.send_place());
                b.connect_clicked(move |_| me.send_again(&run, &place));
                v.append(&b);
            }
        }
        v
    }

    /// `PromiseRow`: Answer within, the promise or Not set, and a menu of whole hours to change
    /// or clear it; the line under it.
    fn promise_row(self: &Rc<Self>, hours: Option<i64>) -> gtk::Box {
        let w = &self.w;
        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = text(
            &w.line("work.answer_within").unwrap_or_default(),
            15,
            400,
            &["tafs-ink"],
        );
        title.set_hexpand(true);
        row.append(&title);
        let value = text(
            &work::promise_value(w, hours).unwrap_or_default(),
            15,
            600,
            &["tafs-amber-text"],
        );
        value.set_wrap(false);
        row.append(&value);
        row.append(&crate::widgets::symbol("pan-down-symbolic", 12));
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let popover = gtk::Popover::new();
        popover.set_child(Some(&list));
        let menu = gtk::MenuButton::new();
        menu.set_child(Some(&row));
        menu.set_popover(Some(&popover));
        menu.add_css_class("tafs-plain");
        menu.set_size_request(-1, 44);
        let choices = std::iter::once(None).chain(work::PROMISE_CHOICES.iter().map(|h| Some(*h)));
        for choice in choices {
            let b = gtk::Button::with_label(&work::promise_value(w, choice).unwrap_or_default());
            b.add_css_class("flat");
            let (me, popover) = (self.clone(), popover.downgrade());
            b.connect_clicked(move |_| {
                if let Some(p) = popover.upgrade() {
                    p.popdown();
                }
                if choice != hours {
                    me.set_promise(choice);
                }
            });
            list.append(&b);
        }
        v.append(&menu);
        v.append(&text(
            &w.line("work.answer_within_line").unwrap_or_default(),
            12,
            400,
            &["tafs-dim"],
        ));
        v
    }

    /// A change as the session, then the work read again, and what failed said after the read.
    fn change(
        self: &Rc<Self>,
        path: String,
        body: Vec<u8>,
        method: &'static str,
        fails: &'static str,
    ) {
        *self.error.borrow_mut() = None;
        let me = self.clone();
        gtk::glib::spawn_future_local(async move {
            let (words, source) = ((*me.w).clone(), me.source.clone());
            let failed = gtk::gio::spawn_blocking(move || -> Option<String> {
                match &source {
                    WorkSource::Cloud { token, languages } => {
                        match crate::kiff::cloud_send(token, languages, method, &path, Some(&body))
                        {
                            Ok(a) if (200..300).contains(&a.status) => None,
                            Ok(a) => Some(
                                tafs_core::signin::refusal(&a, Some(&words))
                                    .words(&words)
                                    .unwrap_or_default(),
                            ),
                            Err(e) => Some(e.words(&words).unwrap_or_default()),
                        }
                    }
                    // A screenshot's work: nothing is sent.
                    _ => None,
                }
            })
            .await
            .unwrap_or(None);
            me.sending.borrow_mut().clear();
            me.load().await;
            if let Some(why) = failed {
                *me.error.borrow_mut() = me.w.get(fails, &[("why", &why)]).ok();
            }
            me.draw();
        });
    }

    fn set_promise(self: &Rc<Self>, hours: Option<i64>) {
        self.change(
            work::promise_path(&self.id),
            work::promise_body(hours),
            "PUT",
            "work.answer_within_not_set",
        );
    }

    /// `sendAgain`: once more by its place; it does no work and signs nothing.
    fn send_again(self: &Rc<Self>, run: &str, place: &str) {
        if self.sending.borrow().iter().any(|r| r == run) {
            return;
        }
        self.sending.borrow_mut().push(run.to_string());
        self.draw();
        self.change(
            tafs_core::runs::send_again_path(&self.id, run, place),
            b"{}".to_vec(),
            "POST",
            "work.not_sent_again",
        );
    }

    async fn load(self: &Rc<Self>) {
        let (words, source, id) = ((*self.w).clone(), self.source.clone(), self.id.clone());
        let read = gtk::gio::spawn_blocking(move || -> Result<work::Work, String> {
            let body = match &source {
                WorkSource::Cloud { token, languages } => {
                    let a = crate::kiff::taf_activity(token, languages, &work::path(&id))
                        .map_err(|e| e.words(&words).unwrap_or_default())?;
                    if a.status != 200 {
                        return Err(tafs_core::signin::refusal(&a, Some(&words))
                            .words(&words)
                            .unwrap_or_default());
                    }
                    a.body
                }
                WorkSource::File(file) => std::fs::read(file).map_err(|e| e.to_string())?,
                WorkSource::None => return Err(String::new()),
            };
            work::read(&body)
                .ok_or_else(|| tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
        })
        .await
        .unwrap_or_else(|_| Err(String::new()));
        *self.read.borrow_mut() = Some(read);
    }
}

/// The panel for the taf `id`, drawn once its work is read; nothing until then, as the iPad's.
pub fn panel_for(
    w: Rc<WordBook>,
    id: String,
    source: WorkSource,
    home_health: Option<Health>,
    opens: Rc<Opens>,
) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
    if matches!(source, WorkSource::None) {
        return root;
    }
    let p = Rc::new(Panel {
        w,
        id,
        source,
        home_health,
        opens,
        root: root.downgrade(),
        read: std::cell::RefCell::new(None),
        error: std::cell::RefCell::new(None),
        sending: std::cell::RefCell::new(Vec::new()),
    });
    gtk::glib::spawn_future_local(async move {
        p.load().await;
        p.draw();
    });
    root
}
