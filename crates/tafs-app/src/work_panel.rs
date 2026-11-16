//! A tafs cloud taf's work (`WorkPanel`, `TafDetailView.swift`), on its Activity: its line, its
//! health with the fixes this computer can act on, its oldest waiting case, then its requests and
//! skipped firings, newest first, each in tafs cloud's words. Read once as the session
//! (`GET /v1/tafs/{id}/work`); nothing is sent. The lines are `tafs-core/src/work.rs`.

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

/// What the panel's fixes open: Plan & usage, for a `credits` fix.
pub struct Opens {
    pub usage: Rc<dyn Fn()>,
}

/// `HealthLines`: each line in the health's colour, with the fixes it can act on. Coach isn't on
/// this computer, so a `coach` fix draws no button, as the iPad's without `mayUseCoach`.
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
            if fix.kind == "credits" {
                let button = outline(&fix.words);
                button.set_hexpand(false);
                button.set_halign(gtk::Align::Start);
                let opens = opens.clone();
                button.connect_clicked(move |_| (opens.usage)());
                b.append(&button);
            }
        }
    }
    Some(b)
}

fn draw(
    w: &WordBook,
    root: &gtk::Box,
    read: Result<work::Work, String>,
    home_health: Option<&Health>,
    opens: &Rc<Opens>,
) {
    let p = panel(false);
    p.append(&eyebrow(&w.line("taf.its_work").unwrap_or_default(), false));
    match read {
        Ok(work) => {
            let view = work::panel(&work, home_health);
            if let Some(line) = &view.line {
                p.append(&text(line, 15, 400, &["tafs-ink"]));
            }
            if let Some(h) = view.health.as_ref().and_then(|h| health_lines(h, opens)) {
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
            for row in &view.rows {
                match row {
                    Row::Skipped(s) => p.append(&text(&s.words, 13, 400, &["tafs-dim"])),
                    Row::Request(r) => {
                        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
                        let class = match work::tone(&r.state) {
                            Tone::Amber => "tafs-amber-text",
                            Tone::No => "tafs-no",
                            Tone::Soft => "tafs-soft",
                        };
                        v.append(&text(&r.line, 14, 400, &[class]));
                        if let Some(used) = &r.credits {
                            v.append(&text(used, 12, 400, &["tafs-dim"]));
                        }
                        // Send again is for whoever manages the taf: not ported yet.
                        for d in &r.deliveries {
                            let class = if d.send_again {
                                "tafs-amber-text"
                            } else {
                                "tafs-dim"
                            };
                            v.append(&text(&d.line, 12, 400, &[class]));
                        }
                        p.append(&v);
                    }
                }
            }
        }
        Err(why) => p.append(&text(&why, 13, 400, &["tafs-no"])),
    }
    root.append(&p);
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
    let r = root.clone();
    gtk::glib::spawn_future_local(async move {
        let words: WordBook = (*w).clone();
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
        draw(&w, &r, read, home_health.as_ref(), &opens);
    });
    root
}
