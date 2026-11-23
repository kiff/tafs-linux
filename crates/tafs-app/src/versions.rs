//! A taf's versions and its trial (`VersionsPanel`, `TrialStartSheet`, `TrialScreen`): the link
//! under a tafs cloud taf's name ("Version 2 · live", and "Trying v3 · Resume" while a trial is
//! open), the versions in a sheet, newest first, and the open trial in a sheet of its own, with
//! its own chat (`tafs-core/versions.rs`). What the phone signs is drawn up to the signature:
//! Install this version, Finish and Go back say this computer can't approve, and Start trying
//! and Install stand disabled with why (`ApprovalNotice`). Cancelling a switch, and chatting with
//! the trial, are sent.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gio, glib};
use tafs_core::sidebar::Mode;
use tafs_core::signed::Act;
use tafs_core::versions::{self, Offers, OpenTrial, Version, VersionState};
use tafs_core::words::WordBook;

use crate::taf_chat::ChatSource;
use crate::widgets::{eyebrow, outline, panel, symbol, text};

thread_local! {
    /// `--versions answer.json` and `--trial answer.json`: tafs cloud's answers for the
    /// screenshots; nothing is read or sent.
    pub static FILES: RefCell<Option<(PathBuf, Option<PathBuf>)>> = const { RefCell::new(None) };
    /// `--page versions:<key>` or `trial:<key>`: that sheet, open over the page.
    pub static OPEN: RefCell<Option<&'static str>> = const { RefCell::new(None) };
}

/// The taf the versions are of: tafs cloud's id, its name, and where its chat reads.
#[derive(Clone)]
pub struct VersionsTaf {
    pub id: String,
    pub name: String,
    pub chat: ChatSource,
}

/// Where the versions are read: tafs cloud as the session, or a screenshot's files.
#[derive(Clone)]
enum Source {
    Cloud {
        token: String,
        languages: Vec<&'static str>,
        signed_out: Rc<dyn Fn()>,
    },
    File {
        versions: PathBuf,
        trial: Option<PathBuf>,
    },
}

impl Source {
    fn of(chat: &ChatSource) -> Option<Source> {
        if let Some((versions, trial)) = FILES.with(|f| f.borrow().clone()) {
            return Some(Source::File { versions, trial });
        }
        match chat {
            ChatSource::Cloud {
                token,
                languages,
                signed_out,
            } => Some(Source::Cloud {
                token: token.clone(),
                languages: languages.clone(),
                signed_out: signed_out.clone(),
            }),
            _ => None,
        }
    }
}

/// What a read came to: the versions and the open trial, why they couldn't be read, or the
/// session over.
enum Read {
    Got(Box<(VersionState, Option<OpenTrial>)>),
    Failed(String),
    SignedOut,
}

/// `loadVersions` and `loadTrial`, at once.
async fn read(w: &WordBook, source: &Source, taf: &str) -> Read {
    // What the read needs, without the session's ending (which stays on this thread).
    enum At {
        Cloud(String, Vec<&'static str>),
        File(PathBuf, Option<PathBuf>),
    }
    let at = match source {
        Source::Cloud {
            token, languages, ..
        } => At::Cloud(token.clone(), languages.clone()),
        Source::File { versions, trial } => At::File(versions.clone(), trial.clone()),
    };
    let (w, taf) = (w.clone(), taf.to_string());
    gio::spawn_blocking(move || match &at {
        At::File(versions, trial) => {
            let state = std::fs::read(versions)
                .ok()
                .and_then(|b| versions::read_versions(&b));
            let trial = trial
                .as_ref()
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| versions::read_trial(&b))
                .flatten();
            match state {
                Some(s) => Read::Got(Box::new((s, trial))),
                None => Read::Failed(String::new()),
            }
        }
        At::Cloud(token, languages) => {
            let get = |path: String| crate::kiff::cloud(token, languages, &path);
            let unreadable = || tafs_core::signin::plain_words(&w, 200).unwrap_or_default();
            let v = match get(versions::versions_path(&taf)) {
                Ok(a) if a.status == 401 => return Read::SignedOut,
                Ok(a) if a.status == 200 => match versions::read_versions(&a.body) {
                    Some(s) => s,
                    None => return Read::Failed(unreadable()),
                },
                Ok(a) => {
                    return Read::Failed(
                        tafs_core::signin::refusal(&a, Some(&w))
                            .words(&w)
                            .unwrap_or_default(),
                    )
                }
                Err(e) => return Read::Failed(e.words(&w).unwrap_or_default()),
            };
            // No trial read is no trial shown.
            let trial = match get(versions::trial_path(&taf)) {
                Ok(a) if a.status == 200 => versions::read_trial(&a.body).flatten(),
                _ => None,
            };
            Read::Got(Box::new((v, trial)))
        }
    })
    .await
    .unwrap_or(Read::Failed(String::new()))
}

/// `VersionsScreen` pushed on `nav`: Coach's panel's own navigation, when a new version's card
/// opens it there, as the iPad's (with Back and its title in the bar). False when the versions
/// can't be read from here.
pub fn push(w: Rc<WordBook>, taf: VersionsTaf, nav: &adw::NavigationView) -> bool {
    let Some(source) = Source::of(&taf.chat) else {
        return false;
    };
    let me = Rc::new(Opener { w, source, taf });
    let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
    me.fill(&body);
    nav.push(&crate::starts_when::page(
        &me.l("version.title"),
        &column_of(&body),
    ));
    true
}

/// A pushed screen's column, at the panel's margins.
fn column_of(body: &gtk::Box) -> gtk::Box {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 12);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    column.append(body);
    column
}

/// The link under a tafs cloud taf's name: its versions, and its trial while one is open. Drawn
/// once both are read; nothing while they're read, and nothing when they can't be.
pub fn link(w: Rc<WordBook>, taf: VersionsTaf) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 14);
    row.set_widget_name("taf.versions");
    let Some(source) = Source::of(&taf.chat) else {
        return row;
    };
    let (row2, w2) = (row.clone(), w.clone());
    glib::spawn_future_local(async move {
        let (state, trial) = match read(&w2, &source, &taf.id).await {
            Read::Got(got) => *got,
            Read::SignedOut => {
                if let Source::Cloud { signed_out, .. } = &source {
                    signed_out();
                }
                return;
            }
            Read::Failed(_) => return,
        };
        let Ok((title, resume)) = versions::link(&w2, Some(&state), trial.as_ref()) else {
            return;
        };
        let b = gtk::Button::new();
        b.add_css_class("tafs-plain");
        let inner = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let clock = symbol("document-open-recent-symbolic", 13);
        clock.add_css_class("tafs-soft");
        inner.append(&clock);
        inner.append(&text(&title, 13, 600, &["tafs-soft"]));
        let chevron = symbol("go-next-symbolic", 11);
        chevron.add_css_class("tafs-dim");
        inner.append(&chevron);
        b.set_child(Some(&inner));
        let open = Rc::new(Opener {
            w: w2.clone(),
            source: source.clone(),
            taf: taf.clone(),
        });
        {
            let open = open.clone();
            b.connect_clicked(move |b| open.versions(b));
        }
        row2.append(&b);
        if let (Some(words), Some(t)) = (resume, trial) {
            let r = gtk::Button::with_label(&words);
            r.add_css_class("tafs-link");
            r.set_widget_name("taf.trial");
            let (open, t2) = (open.clone(), t.clone());
            r.connect_clicked(move |b| open.trial(&t2, b));
            row2.append(&r);
            if OPEN.with(|o| *o.borrow()) == Some("trial") {
                OPEN.with(|o| *o.borrow_mut() = None);
                glib::idle_add_local_once(move || r.emit_clicked());
            }
        }
        if OPEN.with(|o| *o.borrow()) == Some("versions") {
            OPEN.with(|o| *o.borrow_mut() = None);
            glib::idle_add_local_once(move || b.emit_clicked());
        }
    });
    row
}

/// What opens the sheets, for one taf.
/// The Card tab's "Changes to how it runs" (`PanelLink` to `VersionsScreen`), for whoever
/// manages a tafs cloud taf: tafs cloud's title for the versions, else the app's, and its
/// summary ("v1 live · 0 earlier") once Setup's read says it. It opens the versions, as the
/// version line under the taf's name does.
pub fn changes_row(
    w: Rc<WordBook>,
    taf: VersionsTaf,
    setup: crate::setup_pane::SetupSource,
) -> gtk::Button {
    let b = gtk::Button::new();
    b.add_css_class("tafs-panel-link");
    b.set_widget_name("taf.changes");
    let Some(source) = Source::of(&taf.chat) else {
        b.set_visible(false);
        return b;
    };
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let words = gtk::Box::new(gtk::Orientation::Vertical, 4);
    words.set_hexpand(true);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let icon = symbol(crate::widgets::icon_for("square.stack.3d.up"), 15);
    icon.add_css_class("tafs-amber-text");
    head.append(&icon);
    let title = text(
        &w.line("version.changes_row").unwrap_or_default(),
        15,
        600,
        &["tafs-amber-text"],
    );
    head.append(&title);
    words.append(&head);
    let summary = text("", 13, 400, &["tafs-soft"]);
    summary.set_visible(false);
    words.append(&summary);
    row.append(&words);
    let chevron = symbol("go-next-symbolic", 13);
    chevron.add_css_class("tafs-dim");
    row.append(&chevron);
    b.set_child(Some(&row));
    let open = Rc::new(Opener {
        w: w.clone(),
        source,
        taf: taf.clone(),
    });
    b.connect_clicked(move |b| open.versions(b));
    let id = taf.id.clone();
    glib::spawn_future_local(async move {
        if let Some(v) = crate::setup_pane::versions_of(&w, &setup, &id).await {
            if !v.title.trim().is_empty() {
                title.set_label(&v.title);
            }
            if !v.summary.trim().is_empty() {
                summary.set_label(&v.summary);
                summary.set_visible(true);
            }
        }
    });
    b
}

#[derive(Clone)]
struct Opener {
    w: Rc<WordBook>,
    source: Source,
    taf: VersionsTaf,
}

impl Opener {
    fn l(&self, key: &str) -> String {
        self.w.line(key).unwrap_or_default()
    }

    /// `VersionsScreen`: the panel, read when it opens and again after a change. Pushed on the
    /// taf's page, with Back and its title in the bar, as the iPad's; else in a sheet.
    fn versions(&self, over: &gtk::Button) {
        let body = gtk::Box::new(gtk::Orientation::Vertical, 12);
        let me = Rc::new(self.clone());
        me.fill(&body);
        if let Some(nav) = crate::starts_when::taf_nav(over) {
            nav.push(&crate::starts_when::page(
                &self.l("version.title"),
                &column_of(&body),
            ));
            return;
        }
        let sheet = crate::sheet::new(&self.l("version.title"), &self.l("common.done"), None, 560);
        sheet.column.append(&body);
        sheet.dialog.present(Some(over));
    }

    fn fill(self: &Rc<Self>, body: &gtk::Box) {
        let (me, body) = (self.clone(), body.clone());
        glib::spawn_future_local(async move {
            let read = read(&me.w, &me.source, &me.taf.id).await;
            while let Some(c) = body.first_child() {
                body.remove(&c);
            }
            match read {
                Read::Got(got) => me.panel(&body, &got.0, got.1.as_ref()),
                Read::Failed(why) => body.append(&text(&why, 13, 400, &["tafs-no"])),
                Read::SignedOut => {
                    if let Source::Cloud { signed_out, .. } = &me.source {
                        signed_out();
                    }
                }
            }
        });
    }

    fn panel(self: &Rc<Self>, body: &gtk::Box, state: &VersionState, trial: Option<&OpenTrial>) {
        let Ok(l) = versions::lines(&self.w, state, trial) else {
            return;
        };
        let p = panel(false);
        p.append(&eyebrow(&l.title, false));
        p.append(&text(&l.intro, 13, 400, &["tafs-soft"]));
        let slot = crate::approval::refusal_slot();
        if let Some(sw) = &l.switching {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            v.append(&text(&sw.title, 15, 600, &["tafs-amber-text"]));
            v.append(&text(&sw.waits, 13, 400, &["tafs-soft"]));
            let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            buttons.set_homogeneous(true);
            if let Some(finish) = &sw.finish {
                let b = outline(finish);
                self.refuses(&b, version_act(&self.taf.id, sw.n), &slot);
                buttons.append(&b);
            }
            let cancel = outline(&sw.cancel);
            cancel.add_css_class("tafs-no");
            let (me, body2) = (self.clone(), body.clone());
            cancel.connect_clicked(move |b| me.cancel_switch(b, &body2));
            buttons.append(&cancel);
            v.append(&buttons);
            p.append(&v);
        }
        if let Some(first) = &l.first_only {
            p.append(&text(first, 14, 400, &["tafs-soft"]));
        }
        for row in &l.rows {
            let r = gtk::Box::new(gtk::Orientation::Vertical, 6);
            r.set_widget_name(&format!("version.{}", row.n));
            let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            head.append(&text(&row.number, 15, 600, &["tafs-ink"]));
            if let Some(live) = &row.live {
                head.append(&tag(live, "ok"));
            } else if let Some(trying) = &row.trying {
                head.append(&tag(trying, "amber"));
            }
            r.append(&head);
            if let Some(note) = &row.note {
                r.append(&text(note, 14, 400, &["tafs-ink"]));
            }
            r.append(&text(&row.byline, 12, 400, &["tafs-dim"]));
            for t in &row.tried {
                r.append(&text(t, 12, 400, &["tafs-soft"]));
            }
            let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            buttons.set_homogeneous(true);
            match &row.offers {
                Offers::None => {}
                Offers::Resume(words) => {
                    if let Some(t) = trial {
                        let b = outline(words);
                        let (me, t) = (self.clone(), t.clone());
                        b.connect_clicked(move |b| me.trial(&t, b));
                        buttons.append(&b);
                    }
                }
                Offers::TryAndInstall {
                    try_it,
                    try_open,
                    install,
                    install_open,
                } => {
                    let t = outline(try_it);
                    t.set_sensitive(*try_open);
                    if let Some(v) = state.versions.iter().find(|v| v.n == row.n) {
                        let (me, v) = (self.clone(), v.clone());
                        t.connect_clicked(move |b| me.start(&v, b));
                    }
                    buttons.append(&t);
                    let i = outline(install);
                    i.set_sensitive(*install_open);
                    self.refuses(&i, version_act(&self.taf.id, row.n), &slot);
                    buttons.append(&i);
                }
            }
            if buttons.first_child().is_some() {
                r.append(&buttons);
            }
            p.append(&r);
        }
        p.append(&slot);
        body.append(&p);
    }

    /// A tap that would sign `act`: this computer says it can't approve, in `slot`.
    fn refuses(&self, b: &gtk::Button, act: Act, slot: &gtk::Box) {
        let (w, slot) = (self.w.clone(), slot.clone());
        b.connect_clicked(move |_| crate::approval::refuse(&w, Mode::Kiff, &act, &slot));
    }

    /// `cancelSwitch`: the switch dropped at tafs cloud, as the session, then read again.
    fn cancel_switch(self: &Rc<Self>, b: &gtk::Button, body: &gtk::Box) {
        let Source::Cloud {
            token, languages, ..
        } = self.source.clone()
        else {
            return;
        };
        b.set_sensitive(false);
        let (me, body, taf) = (self.clone(), body.clone(), self.taf.id.clone());
        glib::spawn_future_local(async move {
            let _ = gio::spawn_blocking(move || {
                crate::kiff::cloud_send(
                    &token,
                    &languages,
                    "DELETE",
                    &versions::cancel_switch_path(&taf),
                    None,
                )
            })
            .await;
            me.fill(&body);
        });
    }

    /// `TrialStartSheet`: what trying the version means, and Start trying, disabled with why:
    /// its Card is the KIFF draft the phone confirms, which this computer doesn't ask for.
    fn start(self: &Rc<Self>, v: &Version, over: &gtk::Button) {
        let Ok(l) = versions::start_lines(&self.w, v) else {
            return;
        };
        let sheet = crate::sheet::new(&l.title, &self.l("common.cancel"), Some(&l.start), 520);
        if let Some(start) = &sheet.confirm {
            start.set_sensitive(false);
        }
        sheet
            .column
            .append(&text(&l.heading, 17, 600, &["tafs-ink"]));
        let p = panel(false);
        p.append(&eyebrow(&l.while_trying, false));
        for line in &l.lines {
            p.append(&text(line, 14, 400, &["tafs-ink"]));
        }
        sheet.column.append(&p);
        if let Ok(Some(n)) =
            tafs_core::approval::notice(&self.w, Mode::Kiff, tafs_core::approval::THIS_COMPUTER)
        {
            sheet.column.append(&crate::approval::not_ready(&n));
        }
        sheet.dialog.present(Some(over));
    }

    /// `TrialScreen`: the bar (trying, not live, Go back and Install with their asks), what the
    /// trial is before its first message, and the trial's own chat.
    fn trial(self: &Rc<Self>, t: &OpenTrial, over: &gtk::Button) {
        let Ok(l) = versions::trial_lines(&self.w, t, &self.taf.name) else {
            return;
        };
        let sheet = crate::sheet::new(
            &self
                .w
                .get(
                    "version.trial.chat_name",
                    &[("taf", &self.taf.name), ("n", &t.n.to_string())],
                )
                .unwrap_or_default(),
            &self.l("common.done"),
            None,
            720,
        );
        let bar = gtk::Box::new(gtk::Orientation::Vertical, 8);
        bar.add_css_class("tafs-trial-bar");
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        head.append(&text(&l.trying, 15, 600, &["tafs-ink"]));
        head.append(&tag(&l.not_live, "amber"));
        bar.append(&head);
        let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        buttons.set_homogeneous(true);
        let slot = crate::approval::refusal_slot();
        let back = outline(&l.go_back);
        back.add_css_class("tafs-no");
        {
            let (w, slot, title, body, yes) = (
                self.w.clone(),
                slot.clone(),
                l.go_back_title.clone(),
                l.go_back_body.clone(),
                l.go_back.clone(),
            );
            let act = Act::EndTrial {
                draft: t.draft.clone(),
            };
            back.connect_clicked(move |b| {
                ask(&w, b, &title, &body, &yes, true, act.clone(), &slot)
            });
        }
        buttons.append(&back);
        // Install waits on the approval key: disabled here, with why under the bar.
        let install = outline(&l.install);
        install.set_sensitive(false);
        buttons.append(&install);
        bar.append(&buttons);
        if let Ok(Some(n)) =
            tafs_core::approval::notice(&self.w, Mode::Kiff, tafs_core::approval::THIS_COMPUTER)
        {
            bar.append(&crate::approval::not_ready(&n));
        }
        bar.append(&slot);
        sheet.column.append(&bar);
        let guide = gtk::Box::new(gtk::Orientation::Vertical, 8);
        for line in &l.guide {
            guide.append(&text(line, 14, 400, &["tafs-ink"]));
        }
        guide.append(&text(&l.guide_foot, 13, 400, &["tafs-soft"]));
        sheet.column.append(&guide);
        let chat = crate::taf_chat::pane(
            self.w.clone(),
            crate::taf_chat::ChatTaf {
                id: self.taf.id.clone(),
                name: self.taf.name.clone(),
                ask_coach: false,
                held: vec![],
                trial: true,
                reread: None,
                record: vec![],
                doors: crate::doors::DoorsSource::None,
            },
            self.taf.chat.clone(),
        );
        chat.set_size_request(-1, 420);
        sheet.column.append(&chat);
        sheet.dialog.present(Some(over));
    }
}

/// Install this version, or Finish: the switch the device begins and signs.
fn version_act(taf: &str, n: i64) -> Act {
    Act::MakeLive {
        taf: taf.to_string(),
        version: n,
    }
}

/// A tag after a version's number: "Live now" in green, "Trying" or "not live" in amber.
fn tag(words: &str, tone: &str) -> gtk::Label {
    let l = gtk::Label::new(Some(words));
    l.add_css_class("tafs-version-tag");
    l.add_css_class(tone);
    l.set_valign(gtk::Align::Center);
    l
}

/// An ask before a signed act (`confirmationDialog`): its title, its words, and its yes; the yes
/// says this computer can't approve, in `slot`, and nothing is sent.
#[allow(clippy::too_many_arguments)]
fn ask(
    w: &Rc<WordBook>,
    over: &gtk::Button,
    title: &str,
    body: &str,
    yes: &str,
    destructive: bool,
    act: Act,
    slot: &gtk::Box,
) {
    let a = adw::AlertDialog::new(Some(title), Some(body));
    a.add_response("no", &w.line("common.cancel").unwrap_or_default());
    a.add_response("yes", yes);
    if destructive {
        a.set_response_appearance("yes", adw::ResponseAppearance::Destructive);
    }
    a.set_default_response(Some("no"));
    a.set_close_response("no");
    let (w, slot) = (w.clone(), slot.clone());
    a.connect_response(None, move |_, r| {
        if r == "yes" {
            crate::approval::refuse(&w, Mode::Kiff, &act, &slot);
        }
    });
    a.present(Some(over));
}
