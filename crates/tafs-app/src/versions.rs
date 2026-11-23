//! A taf's versions and its trial (`VersionsPanel`, `TrialStartSheet`, `TrialScreen`): the link
//! under a tafs cloud taf's name ("Version 2 · live", and "Trying v3 · Resume" while a trial is
//! open), the versions in a sheet, newest first, and the open trial in a sheet of its own, with
//! its own chat (`tafs-core/versions.rs`). What the phone signs is drawn up to the signature:
//! Install this version, Finish and Go back say this computer can't approve. With a security key,
//! Start trying and a trial's Install go from here: the key approves KIFF's confirm of the trial
//! Card's draft (`Act::ConfirmDraft`), and tafs cloud's own calls go as the session; without one,
//! they stand disabled with why (`ApprovalNotice`). Cancelling a switch, and chatting with the
//! trial, are sent.

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
    /// `--page versions:<key>` or `trial:<key>`: that sheet, open over the page; `try:<key>`,
    /// the versions with the first one's Try it open over them.
    pub static OPEN: RefCell<Option<&'static str>> = const { RefCell::new(None) };
    /// `--trial-draft answer.json`: KIFF's draft a trial's Card is, for the screenshots: the
    /// start sheet draws it; nothing is read or sent.
    pub static DRAFT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
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
        if matches!(OPEN.with(|o| *o.borrow()), Some("versions" | "try")) {
            // `try`: kept, so the versions' first Try it opens over them once they're drawn.
            OPEN.with(|o| {
                let mut o = o.borrow_mut();
                if *o == Some("versions") {
                    *o = None;
                }
            });
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

/// The draft a trial's Card is, as read here: tafs cloud's id for it and KIFF's draft.
type Shown = Rc<RefCell<Option<tafs_core::drafts::AgentDraft>>>;

impl Opener {
    fn l(&self, key: &str) -> String {
        self.w.line(key).unwrap_or_default()
    }

    /// The session and languages, signed in with KIFF; none for a screenshot's files.
    fn session(&self) -> Option<(String, Vec<&'static str>)> {
        match &self.source {
            Source::Cloud {
                token, languages, ..
            } => Some((token.clone(), languages.clone())),
            Source::File { .. } => None,
        }
    }

    /// A change at tafs cloud as the session (`try`, `PUT …/trial`, `install`), off the main
    /// thread: its answer's body, or why it didn't go, in tafs cloud's words or the app's.
    async fn send(
        &self,
        method: &'static str,
        path: String,
        body: Option<Vec<u8>>,
    ) -> Result<Vec<u8>, String> {
        let Some((token, languages)) = self.session() else {
            return Err(String::new());
        };
        let words = (*self.w).clone();
        gio::spawn_blocking(move || {
            let a = crate::kiff::cloud_send(&token, &languages, method, &path, body.as_deref())
                .map_err(|e| e.words(&words).unwrap_or_default())?;
            if (200..300).contains(&a.status) {
                Ok(a.body)
            } else {
                Err(tafs_core::signin::refusal(&a, Some(&words))
                    .words(&words)
                    .unwrap_or_default())
            }
        })
        .await
        .unwrap_or_else(|_| Err(String::new()))
    }

    /// `kiff.draft(id:)`: KIFF's draft as the session, unsigned.
    async fn draft(&self, id: &str) -> Result<tafs_core::drafts::AgentDraft, String> {
        let Some((token, languages)) = self.session() else {
            return Err(String::new());
        };
        let (words, path) = ((*self.w).clone(), tafs_core::drafts::draft_path(id));
        gio::spawn_blocking(move || {
            let a = crate::kiff::kiff_get(&token, &languages, &path)
                .map_err(|e| e.words(&words).unwrap_or_default())?;
            if !(200..300).contains(&a.status) {
                return Err(tafs_core::signin::refusal(&a, Some(&words))
                    .words(&words)
                    .unwrap_or_default());
            }
            tafs_core::drafts::AgentDraft::read(&a.body)
                .ok_or_else(|| tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
        })
        .await
        .unwrap_or_else(|_| Err(String::new()))
    }

    /// Whether this computer's security key approves a trial Card's confirm here.
    fn key_confirms(&self) -> bool {
        let kept = crate::store::load(chrono::Utc::now().timestamp());
        self.session().is_some()
            && crate::key_approval::can_approve(
                kept.as_ref(),
                &Act::ConfirmDraft {
                    draft: "d".into(),
                    version: "v".into(),
                    trial: true,
                },
            )
    }

    /// A draft's Cards, as the owner reads them before the key (`DraftCardsView`): each Card's
    /// lines in a panel, under "Its Card".
    fn draw_card(&self, into: &gtk::Box, draft: &tafs_core::drafts::AgentDraft) {
        while let Some(c) = into.first_child() {
            into.remove(&c);
        }
        into.append(&eyebrow(&self.l("card.its_card"), false));
        for lines in draft.card_lines(&self.w).unwrap_or_default() {
            let p = panel(false);
            for line in lines {
                p.append(&text(&line, 14, 400, &["tafs-ink"]));
            }
            into.append(&p);
        }
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
                        if OPEN.with(|o| *o.borrow()) == Some("try") {
                            OPEN.with(|o| *o.borrow_mut() = None);
                            let t = t.clone();
                            glib::idle_add_local_once(move || t.emit_clicked());
                        }
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

    /// `TrialStartSheet`: what trying the version means, its Card as KIFF's draft says it, and
    /// Start trying. With a security key, Start trying reads the draft again (changed since it
    /// was shown, it's shown again to read first), the key approves KIFF's confirm of it as a
    /// trial, and once KIFF says it's trying, tafs cloud opens the trial and its chat opens.
    /// Without one, it stands disabled with why.
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
        // Its Card, then Start trying, with the key.
        self.trial_card(v, &sheet, over);
        // Why it can't go from here, unless this computer's security key approves it.
        if let (false, Ok(Some(n))) = (
            self.key_confirms(),
            tafs_core::approval::notice(&self.w, Mode::Kiff, tafs_core::approval::THIS_COMPUTER),
        ) {
            sheet.column.append(&crate::approval::not_ready(&n));
        }
        sheet.dialog.present(Some(over));
    }

    /// The trial's Card in the start sheet, read when it opens (`trialCard`: tafs cloud's draft
    /// for it, then KIFF's), and Start trying wired to the key.
    fn trial_card(self: &Rc<Self>, v: &Version, sheet: &crate::sheet::Sheet, over: &gtk::Button) {
        if self.session().is_none() {
            // A screenshot's draft, from its file: drawn, and Start trying stays as it is.
            let file = DRAFT.with(|d| d.borrow().clone());
            if let Some(d) = file
                .and_then(|p| std::fs::read(p).ok())
                .and_then(|b| tafs_core::drafts::AgentDraft::read(&b))
            {
                let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
                self.draw_card(&card, &d);
                sheet.column.append(&card);
            }
            return;
        }
        let card = gtk::Box::new(gtk::Orientation::Vertical, 8);
        card.append(
            &gtk::Spinner::builder()
                .spinning(true)
                .halign(gtk::Align::Start)
                .build(),
        );
        sheet.column.append(&card);
        let error = text("", 13, 400, &["tafs-no"]);
        error.set_visible(false);
        sheet.column.append(&error);
        let slot = crate::approval::refusal_slot();
        sheet.column.append(&slot);
        let shown: Shown = Rc::default();
        let n = v.n;
        let start = sheet.confirm.clone();
        {
            let (me, card, error, shown, start) = (
                self.clone(),
                card.clone(),
                error.clone(),
                shown.clone(),
                start.clone(),
            );
            glib::spawn_future_local(async move {
                let read = match me
                    .send("POST", versions::try_path(&me.taf.id, n), None)
                    .await
                {
                    Ok(body) => match versions::read_try(&body) {
                        Some(id) => me.draft(&id).await,
                        None => Err(tafs_core::signin::plain_words(&me.w, 200).unwrap_or_default()),
                    },
                    Err(why) => Err(why),
                };
                match read {
                    Ok(d) if d.cards.is_some() => {
                        me.draw_card(&card, &d);
                        *shown.borrow_mut() = Some(d);
                        if let Some(start) = &start {
                            start.set_sensitive(me.key_confirms());
                        }
                    }
                    other => {
                        while let Some(c) = card.first_child() {
                            card.remove(&c);
                        }
                        let why = other.err().unwrap_or_default();
                        error.set_label(&if why.is_empty() {
                            me.l("version.trial_card_unreadable")
                        } else {
                            me.w.get(
                                "version.trial.card_not_read",
                                &[("n", &n.to_string()), ("why", &why)],
                            )
                            .unwrap_or_default()
                        });
                        error.set_visible(true);
                    }
                }
            });
        }
        let Some(start) = start else { return };
        let (me, dialog, over) = (self.clone(), sheet.dialog.clone(), over.clone());
        let label = start.label().unwrap_or_default();
        start.connect_clicked(move |b| {
            let Some(was) = shown.borrow().clone() else {
                return;
            };
            b.set_sensitive(false);
            b.set_label(&me.l("common.starting"));
            let (me, b, card, error, slot, shown, dialog, over, label) = (
                me.clone(),
                b.clone(),
                card.clone(),
                error.clone(),
                slot.clone(),
                shown.clone(),
                dialog.clone(),
                over.clone(),
                label.clone(),
            );
            glib::spawn_future_local(async move {
                let back = |b: &gtk::Button| {
                    b.set_label(&label);
                    b.set_sensitive(true);
                };
                error.set_visible(false);
                // What the owner read is what the key approves: changed since, read it again.
                let now = match me.draft(&was.id).await {
                    Ok(d) => d,
                    Err(why) => {
                        error.set_label(
                            &me.w
                                .get(
                                    "version.trial.cant_try",
                                    &[("n", &n.to_string()), ("why", &why)],
                                )
                                .unwrap_or_default(),
                        );
                        error.set_visible(true);
                        return back(&b);
                    }
                };
                if now.version != was.version {
                    me.draw_card(&card, &now);
                    *shown.borrow_mut() = Some(now);
                    error.set_label(&me.l("version.trial.changed"));
                    error.set_visible(true);
                    return back(&b);
                }
                let act = Act::ConfirmDraft {
                    draft: was.id.clone(),
                    version: was.version.clone(),
                    trial: true,
                };
                let what =
                    me.w.get(
                        "version.trial.prompt",
                        &[("n", &n.to_string()), ("taf", &me.taf.name)],
                    )
                    .ok();
                let opened: Rc<dyn Fn()> = {
                    let (me, b, error, dialog, over, draft) = (
                        me.clone(),
                        b.clone(),
                        error.clone(),
                        dialog.clone(),
                        over.clone(),
                        was.id.clone(),
                    );
                    Rc::new(move || {
                        let (me, b, error, dialog, over, draft) = (
                            me.clone(),
                            b.clone(),
                            error.clone(),
                            dialog.clone(),
                            over.clone(),
                            draft.clone(),
                        );
                        glib::spawn_future_local(async move {
                            me.opened(n, &draft, &b, &error, &dialog, &over).await;
                        });
                    })
                };
                crate::approval::refuse_then(
                    &me.w,
                    Mode::Kiff,
                    &act,
                    what,
                    vec![],
                    &slot,
                    Some(opened),
                );
                back(&b);
            });
        });
    }

    /// The key's confirm went: only a KIFF that runs trials answers "trying", and anything else
    /// is never taken for a trial. Then tafs cloud opens it, and its chat opens.
    async fn opened(
        self: &Rc<Self>,
        n: i64,
        draft: &str,
        b: &gtk::Button,
        error: &gtk::Label,
        dialog: &adw::Dialog,
        over: &gtk::Button,
    ) {
        let say = |why: String| {
            error.set_label(&why);
            error.set_visible(true);
        };
        let trying = match self.draft(draft).await {
            Ok(d) if d.status == "trying" => true,
            Ok(d) if d.status == "confirmed" => {
                say(self.l("version.not_a_trial.confirmed"));
                false
            }
            Ok(d) => {
                say(self
                    .w
                    .get("version.not_a_trial.other", &[("status", &d.status)])
                    .unwrap_or_default());
                false
            }
            Err(why) => {
                say(self
                    .w
                    .get(
                        "version.trial.not_started",
                        &[("n", &n.to_string()), ("why", &why)],
                    )
                    .unwrap_or_default());
                false
            }
        };
        if !trying {
            b.set_sensitive(false);
            return;
        }
        let opened = self
            .send(
                "PUT",
                versions::trial_path(&self.taf.id),
                Some(versions::open_trial_body(n, draft)),
            )
            .await
            .and_then(|body| {
                versions::read_opened(&body)
                    .ok_or_else(|| tafs_core::signin::plain_words(&self.w, 200).unwrap_or_default())
            });
        match opened {
            Ok(t) => {
                dialog.close();
                self.trial(&t, over);
            }
            Err(why) => say(self
                .w
                .get(
                    "version.trial.not_started",
                    &[("n", &n.to_string()), ("why", &why)],
                )
                .unwrap_or_default()),
        }
    }

    /// `installTrial`: asked first, then KIFF's draft read; not confirmed live yet, the key
    /// approves KIFF's confirm of it; then tafs cloud installs it (the version goes live, the
    /// trial ends) and the sheet closes.
    fn install(
        self: &Rc<Self>,
        t: &OpenTrial,
        l: &versions::TrialLines,
        b: &gtk::Button,
        slot: &gtk::Box,
        dialog: &adw::Dialog,
    ) {
        let ask = adw::AlertDialog::new(Some(&l.install_title), Some(&l.install_body));
        ask.add_response("cancel", &self.l("common.cancel"));
        ask.add_response("install", &l.install);
        ask.set_response_appearance("install", adw::ResponseAppearance::Suggested);
        ask.set_default_response(Some("cancel"));
        ask.set_close_response("cancel");
        let (me, t, slot, dialog) = (self.clone(), t.clone(), slot.clone(), dialog.clone());
        ask.connect_response(None, move |_, response| {
            if response != "install" {
                return;
            }
            let (me, t, slot, dialog) = (me.clone(), t.clone(), slot.clone(), dialog.clone());
            glib::spawn_future_local(async move {
                let unfinished = |why: String| {
                    me.w.get(
                        "version.trial.install_unfinished",
                        &[("n", &t.n.to_string()), ("why", &why)],
                    )
                    .unwrap_or_default()
                };
                let installed: Rc<dyn Fn()> = {
                    let (me, t, slot, dialog) =
                        (me.clone(), t.clone(), slot.clone(), dialog.clone());
                    Rc::new(move || {
                        let (me, t, slot, dialog) =
                            (me.clone(), t.clone(), slot.clone(), dialog.clone());
                        glib::spawn_future_local(async move {
                            match me
                                .send("POST", versions::install_path(&me.taf.id), None)
                                .await
                            {
                                Ok(_) => {
                                    dialog.close();
                                }
                                Err(why) => {
                                    slot.append(&text(
                                        &me.w
                                            .get(
                                                "version.trial.install_unfinished",
                                                &[("n", &t.n.to_string()), ("why", &why)],
                                            )
                                            .unwrap_or_default(),
                                        13,
                                        400,
                                        &["tafs-no"],
                                    ));
                                    slot.set_visible(true);
                                }
                            }
                        });
                    })
                };
                match me.draft(&t.draft).await {
                    Ok(d) if d.confirmed() => installed(),
                    Ok(d) => {
                        let what =
                            me.w.get(
                                "version.prompt.install",
                                &[("n", &t.n.to_string()), ("taf", &me.taf.name)],
                            )
                            .ok();
                        crate::approval::refuse_then(
                            &me.w,
                            Mode::Kiff,
                            &Act::ConfirmDraft {
                                draft: d.id.clone(),
                                version: d.version.clone(),
                                trial: false,
                            },
                            what,
                            vec![],
                            &slot,
                            Some(installed),
                        );
                    }
                    Err(why) => {
                        slot.append(&text(&unfinished(why), 13, 400, &["tafs-no"]));
                        slot.set_visible(true);
                    }
                }
            });
        });
        ask.present(Some(b));
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
        // Install: with a security key, from here (asked, then the key confirms the draft live,
        // then tafs cloud installs it); without one, disabled, with why under the bar.
        let install = outline(&l.install);
        install.set_sensitive(self.key_confirms());
        {
            let (me, t, l, slot, dialog) = (
                self.clone(),
                t.clone(),
                l.clone(),
                slot.clone(),
                sheet.dialog.clone(),
            );
            install.connect_clicked(move |b| me.install(&t, &l, b, &slot, &dialog));
        }
        buttons.append(&install);
        bar.append(&buttons);
        // Why it can't go from here, unless this computer's security key approves it.
        if let (false, Ok(Some(n))) = (
            self.key_confirms(),
            tafs_core::approval::notice(&self.w, Mode::Kiff, tafs_core::approval::THIS_COMPUTER),
        ) {
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
