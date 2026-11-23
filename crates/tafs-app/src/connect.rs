//! Connect apps beside the sidebar (`ConnectAppsView`): what tafs cloud offers to connect, the
//! person's own first (Yours) with a ✓, then the rest (Discover), searched as the person types,
//! as the iPad's table on a wide window: App, How it connects, Whose, Status (`ConnectColumns`).
//! Connect asks "Which taf?"; picking one says this computer can't approve, and nothing is
//! started (the iPad starts a connection only to approve it). An app with connections opens them
//! (`connections.rs`). The lines are `tafs-core/src/connect.rs` and `connections.rs`.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use adw::prelude::*;
use tafs_core::connect::{
    called, connect_for, connectable_for, path, says_make_a_taf, split, App, ConnectApps,
    ConnectFor, Connectable, Row,
};
use tafs_core::connections::{connections_of, opens_connections, Grant};
use tafs_core::session::KeptSession;
use tafs_core::sidebar::Mode;
use tafs_core::signed::Act;
use tafs_core::words::WordBook;

use crate::widgets::{one_line, outline, symbol, text};

/// Where Connect apps is read from.
#[derive(Clone)]
pub enum ConnectSource {
    /// The sample: the reference set's apps and connections in a screenshot
    /// (`SampleScreens.connectApps`, `.grants`), else nothing, as the iPad's sample. `tafs` are
    /// the sample's tafs by key and name, which its connections are made for.
    Sample {
        screenshot: bool,
        tafs: Vec<(String, String)>,
    },
    /// tafs cloud, as the session, with this computer's signed grants read where it can make one,
    /// and the tafs an app can be connected for (`connectableTafs`).
    Cloud {
        kept: Box<KeptSession>,
        languages: Vec<&'static str>,
        connectable: Vec<Connectable>,
        manages: bool,
        /// Each KIFF agent's taf by name (`holderIDs`), revoked ones left out.
        holders: Vec<(String, String)>,
        /// What each taf's live Card draws on (`tafsUsing`).
        using: Vec<tafs_core::connections::Using>,
        /// The Record's lines, for a connection's last failure (`lastFailure`).
        record: Vec<tafs_core::record::Entry>,
    },
    /// A screenshot's answers (`--connect`): `answer` as tafs cloud's first page. A fixture's
    /// read never sends.
    File {
        path: PathBuf,
        connectable: Vec<Connectable>,
        manages: bool,
    },
    /// Signed out: nothing read.
    None,
}

/// The sample's connections' KIFF agents, and the sample taf each is for, by its template
/// (`SampleScreens.grantAgents`).
const SAMPLE_AGENTS: [(&str, &str); 2] = [("sample-support", "support"), ("sample-leads", "leads")];

thread_local! {
    /// A screenshot's page: `connect-app` opens the app with the most connections,
    /// `connect-connection` its first connection's detail, `connect-remove` that connection's
    /// Remove access question, `connect-which-taf` "Which taf?" for the first app it asks for,
    /// `connect-approve` that app's first taf picked.
    pub static OPEN: std::cell::Cell<Option<&'static str>> = const { std::cell::Cell::new(None) };
}

impl ConnectSource {
    pub(crate) fn mode(&self) -> Mode {
        match self {
            ConnectSource::Sample { .. } => Mode::Sample,
            ConnectSource::Cloud { .. } | ConnectSource::File { .. } => Mode::Kiff,
            ConnectSource::None => Mode::SignedOut,
        }
    }

    /// `connectableTafs`: the tafs an app can be connected for; the sample's none.
    fn connectable(&self) -> &[Connectable] {
        match self {
            ConnectSource::Cloud { connectable, .. } | ConnectSource::File { connectable, .. } => {
                connectable
            }
            _ => &[],
        }
    }

    /// `canManageCards`: the account's capabilities, as read; the sample's account manages.
    fn manages(&self) -> bool {
        match self {
            ConnectSource::Cloud { manages, .. } | ConnectSource::File { manages, .. } => *manages,
            _ => true,
        }
    }
}

/// What every screen of Connect apps shares: the words, where it reads, the account's
/// connections as this computer has them, and the tafs they're for.
pub(crate) struct Ctx {
    pub w: Rc<WordBook>,
    pub source: ConnectSource,
    /// The account's connections: the sample's in a screenshot; signed in with KIFF, planque's
    /// as this computer reads them (`refreshConnections`), none until read.
    pub grants: RefCell<Vec<Grant>>,
    /// Whether they were read (`readsConnections`): the sample's are; planque's once a read went.
    pub read: std::cell::Cell<bool>,
    /// Each KIFF agent's taf by name (`holderIDs`), revoked ones left out.
    pub holders: Vec<(String, String)>,
    /// What each taf's live Card draws on (`tafsUsing`), for what removing a connection stops.
    pub using: Vec<tafs_core::connections::Using>,
    /// The Record's lines, for a connection's last failure (`lastFailure`).
    pub record: Vec<tafs_core::record::Entry>,
    /// Reads the connections again (`refreshConnections`), after an act the key approved.
    pub reread: RefCell<Option<Rc<dyn Fn()>>>,
}

impl Ctx {
    pub(crate) fn mode(&self) -> Mode {
        self.source.mode()
    }

    pub(crate) fn manages(&self) -> bool {
        self.source.manages()
    }

    /// `tafOf`: the taf a connection was made for, by its name.
    pub(crate) fn taf_of(&self, g: &Grant) -> Option<String> {
        self.holders
            .iter()
            .find(|(agent, _)| *agent == g.request.agent)
            .map(|(_, name)| name.clone())
    }

    /// `lastFailure`: a connection's last failed call, its own taf's first.
    pub(crate) fn last_failure(&self, g: &Grant) -> Option<String> {
        let own = self
            .using
            .iter()
            .find(|u| u.agent == g.request.agent)
            .map(|u| u.key.as_str());
        tafs_core::connections::last_failure(&self.w, g, &self.record, own)
            .ok()
            .flatten()
    }

    /// What draws a connection to the top of its app's list (`connectionGroups`' attention): a
    /// sign-in that waits, one its vendor signed out, or a last failure.
    pub(crate) fn attention(&self, g: &Grant) -> bool {
        tafs_core::connections::attention(g) || self.last_failure(g).is_some()
    }

    /// The tafs whose connections to an app this computer names (`connectionGroups(of:).tafs`),
    /// for "Used by" while tafs cloud sends no `used_by`.
    fn used_here(&self, app: &App, removed: &std::collections::BTreeSet<String>) -> Vec<String> {
        let grants = self.grants.borrow();
        let conns = connections_of(app, &grants);
        tafs_core::connections::groups(&conns, &|g| self.taf_of(g), removed, &|g| self.attention(g))
            .tafs
            .into_iter()
            .map(|g| g.taf)
            .collect()
    }
}

#[derive(Default)]
struct State {
    apps: Option<ConnectApps>,
    error: Option<String>,
    query: String,
    loading_more: bool,
    /// Each read's number: only the newest search may replace the page.
    request: u64,
    /// A narrow window: one-line rows and no table (`sizeClass != .regular`).
    narrow: bool,
}

type Redraw = Rc<dyn Fn()>;

/// The reference set's apps, from the iPad's own sample at the pin.
fn sample_apps() -> Option<ConnectApps> {
    let file = WordBook::default_root().join("../../ios/Tafs/Samples/connect-apps.json");
    ConnectApps::read(&std::fs::read(file).ok()?)
}

/// The reference set's connections (`Samples/grants.json`, kept as a fixture).
fn sample_grants() -> Vec<Grant> {
    let file = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/grants.json");
    std::fs::read(file)
        .ok()
        .and_then(|b| Grant::read_list(&b))
        .unwrap_or_default()
}

/// A screenshot's file: its `answer`.
fn file_part(path: &PathBuf, key: &str) -> Option<Vec<u8>> {
    let v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
    Some(v.get(key)?.to_string().into_bytes())
}

/// This computer's signed read of the account's grants (`grantsReadProof`), made as the iPad
/// sends it with Connect apps' reads: tafs cloud then says which tafs need an app.
/// None without a key, a tenant, or KIFF's name for this computer. Never logged.
fn proof(
    kept: &KeptSession,
    languages: &[&'static str],
    device: &Mutex<Option<String>>,
) -> Option<String> {
    let known = device.lock().ok()?.clone();
    let id = match known {
        Some(id) => id,
        None => {
            let id = crate::coach::this_device(kept, languages)?;
            if let Ok(mut d) = device.lock() {
                *d = Some(id.clone());
            }
            id
        }
    };
    crate::coach::proof(kept, &id)
}

/// `refreshConnections`: the account's connections from planque, read with this computer's
/// device key on planque's safe route (`GET …/grants`, as Account › Chrome and a taf's steps read
/// it), then the screen drawn again with them. Reading signs and grants nothing. Signed in with
/// KIFF only; a read that fails leaves the connections as they were, as the iPad's.
pub(crate) fn read_connections(
    ctx: &Rc<Ctx>,
    device: &Arc<Mutex<Option<String>>>,
    redraw: Rc<dyn Fn()>,
) {
    let ConnectSource::Cloud {
        kept, languages, ..
    } = &ctx.source
    else {
        return;
    };
    let (kept, languages, device, ctx) =
        (kept.clone(), languages.clone(), device.clone(), ctx.clone());
    gtk::glib::spawn_future_local(async move {
        let read = gtk::gio::spawn_blocking(move || {
            let known = device.lock().ok()?.clone();
            let id = match known {
                Some(id) => id,
                None => {
                    let id = crate::coach::this_device(&kept, &languages)?;
                    if let Ok(mut d) = device.lock() {
                        *d = Some(id.clone());
                    }
                    id
                }
            };
            let a =
                crate::kiff::planque_safe(&kept, &id, &languages, tafs_core::planque::Safe::Grants)
                    .ok()?;
            (200..300)
                .contains(&a.status)
                .then(|| Grant::read_all(&a.body))
                .flatten()
        })
        .await
        .ok()
        .flatten();
        if let Some(grants) = read {
            *ctx.grants.borrow_mut() = grants;
            ctx.read.set(true);
            redraw();
        }
    });
}

/// `loadProfileWords`: KIFF's words for a connection's profile, read as the session and checked
/// against the profile shown (`tafs_core::profile_words::checked`); why not, in the iPad's words.
pub(crate) fn read_profile_words(
    w: &WordBook,
    token: &str,
    languages: &[&'static str],
    profile: &serde_json::Value,
) -> Result<tafs_core::profile_words::ProfileWords, String> {
    use tafs_core::profile_words::{body, checked, Failure};
    let not_read = |why: String| {
        w.get("connect.profile_words_not_read", &[("why", &why)])
            .unwrap_or(why)
    };
    let body = body(profile).ok_or_else(|| Failure::OtherProfile.words(w).unwrap_or_default())?;
    let a = crate::kiff::profile_words(token, languages, body)
        .map_err(|e| not_read(e.words(w).unwrap_or_default()))?;
    if !(200..300).contains(&a.status) {
        return Err(not_read(
            tafs_core::signin::refusal(&a, Some(w))
                .words(w)
                .unwrap_or_default(),
        ));
    }
    checked(&a.body, profile).map_err(|f| f.words(w).unwrap_or_default())
}

/// Reads a page from tafs cloud: why not, in the iPad's words, when it can't.
fn read_page(
    w: &WordBook,
    kept: &KeptSession,
    languages: &[&'static str],
    device: &Mutex<Option<String>>,
    p: &str,
    more: bool,
) -> Result<ConnectApps, String> {
    let why = |why: String| {
        let key = if more {
            "connect.more_apps_could_not_listed"
        } else {
            "connect.apps_not_read"
        };
        w.get(key, &[("why", &why)]).unwrap_or(why)
    };
    let proof = proof(kept, languages, device);
    let headers: Vec<(&str, &str)> = proof
        .as_deref()
        .map(|p| vec![(tafs_core::planque::HEADER, p)])
        .unwrap_or_default();
    let answer = crate::kiff::cloud_with(&kept.token, languages, p, &headers)
        .map_err(|e| why(e.words(w).unwrap_or_default()))?;
    if !(200..300).contains(&answer.status) {
        return Err(why(tafs_core::signin::refusal(&answer, Some(w))
            .words(w)
            .unwrap_or_default()));
    }
    ConnectApps::read(&answer.body)
        .ok_or_else(|| why(tafs_core::signin::plain_words(w, 200).unwrap_or_default()))
}

/// The page's screens: the list, an app's connections over it, and one connection's detail over
/// that, each with a way back.
pub(crate) struct Screens {
    pub stack: gtk::Stack,
}

impl Screens {
    /// Shows `page` as `name`, replacing the one there.
    pub(crate) fn show(&self, name: &str, page: &gtk::Widget) {
        if let Some(old) = self.stack.child_by_name(name) {
            self.stack.remove(&old);
        }
        self.stack.add_named(page, Some(name));
        self.stack.set_visible_child_name(name);
    }

    pub(crate) fn back(&self, name: &str) {
        self.stack.set_visible_child_name(name);
    }
}

pub fn page(w: Rc<WordBook>, source: ConnectSource, open_coach: Rc<dyn Fn()>) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    // The account's connections, and the tafs they're for: the sample's in a screenshot.
    let (grants, holders) = match &source {
        ConnectSource::Sample {
            screenshot: true,
            tafs,
        } => (
            sample_grants(),
            SAMPLE_AGENTS
                .iter()
                .filter_map(|(agent, key)| {
                    tafs.iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, name)| (agent.to_string(), name.clone()))
                })
                .collect(),
        ),
        ConnectSource::Cloud { holders, .. } => (vec![], holders.clone()),
        _ => (vec![], vec![]),
    };
    let ctx = Rc::new(Ctx {
        w: w.clone(),
        source: source.clone(),
        read: std::cell::Cell::new(matches!(source, ConnectSource::Sample { .. })),
        grants: RefCell::new(grants),
        holders,
        using: match &source {
            ConnectSource::Cloud { using, .. } => using.clone(),
            _ => vec![],
        },
        record: match &source {
            ConnectSource::Cloud { record, .. } => record.clone(),
            _ => vec![],
        },
        reread: RefCell::new(None),
    });
    let device: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let screens = Rc::new(Screens {
        stack: gtk::Stack::new(),
    });
    screens
        .stack
        .set_transition_type(gtk::StackTransitionType::SlideLeftRight);

    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.set_margin_start(16);
    column.set_margin_end(16);
    column.set_margin_top(12);
    column.set_margin_bottom(12);
    // The title, and the search over the list (`.searchable`).
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let title = text(&l("connect.apps.title"), 34, 700, &["tafs-ink"]);
    title.set_hexpand(true);
    title.set_width_chars(12);
    head.append(&title);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&l("connect.apps.search")));
    search.set_valign(gtk::Align::Center);
    search.set_width_chars(22);
    search.add_css_class("tafs-search");
    head.append(&search);
    column.append(&head);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 10);
    column.append(&list);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    outer.append(&scroller);
    crate::widgets::ground(&outer);
    screens.stack.add_named(&outer, Some("apps"));

    let state = Rc::new(RefCell::new(State::default()));
    let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
    let redraw: Redraw = {
        let draw = draw.clone();
        Rc::new(move || {
            let f = draw.borrow().clone();
            if let Some(f) = f {
                f();
            }
        })
    };
    // An app's connections, over the list.
    let open_app: Rc<dyn Fn(App)> = {
        let (ctx, screens, state) = (ctx.clone(), screens.clone(), state.clone());
        Rc::new(move |app: App| {
            let removed = state
                .borrow()
                .apps
                .as_ref()
                .map(|a| a.removed_tafs.clone())
                .unwrap_or_default();
            let page = crate::connections::app_page(ctx.clone(), screens.clone(), app, removed);
            screens.show("app", &page);
        })
    };
    // Where Connect's tap says this computer can't approve, over the list, once a taf is
    // picked: hidden until then.
    let refused = crate::approval::refusal_slot();
    let refused_panel = crate::widgets::panel(false);
    refused_panel.append(&refused);
    refused_panel.set_visible(false);
    column.insert_child_after(&refused_panel, Some(&head));
    // Connect for one taf, once picked: the iPad starts the connection only to approve it on its
    // sheet behind Face ID. This computer can't approve, so it says so and starts nothing: a
    // request left at planque could never be approved from here.
    let connect_for_taf: Rc<dyn Fn(App, String)> = {
        let (ctx, refused, refused_panel) = (ctx.clone(), refused.clone(), refused_panel.clone());
        Rc::new(move |app: App, taf: String| {
            let act = Act::Connect {
                entry: app.entry.clone(),
                taf,
            };
            crate::approval::refuse(&ctx.w, ctx.mode(), &act, &refused);
            refused_panel.set_visible(refused.get_visible());
        })
    };
    // "Which taf is this for?": the tafs that can have the app, then Cancel.
    let which_taf: Rc<dyn Fn(App, gtk::Widget)> = {
        let (ctx, connect_for_taf) = (ctx.clone(), connect_for_taf.clone());
        Rc::new(move |app: App, over: gtk::Widget| {
            let w = &ctx.w;
            let tafs = connectable_for(ctx.source.connectable(), app.needed_by.as_deref());
            let ask = adw::AlertDialog::new(
                Some(&w.line("connect.which_taf").unwrap_or_default()),
                Some(
                    &w.get("connect.one_taf", &[("app", called(&app))])
                        .unwrap_or_default(),
                ),
            );
            for (i, t) in tafs.iter().enumerate() {
                ask.add_response(&format!("t{i}"), &t.name);
            }
            ask.add_response("cancel", &w.line("common.cancel").unwrap_or_default());
            ask.set_close_response("cancel");
            let connect_for_taf = connect_for_taf.clone();
            ask.connect_response(None, move |_, r| {
                if let Some(i) = r.strip_prefix('t').and_then(|i| i.parse::<usize>().ok()) {
                    if let Some(t) = tafs.get(i) {
                        connect_for_taf(app.clone(), t.id.clone());
                    }
                }
            });
            ask.present(Some(&over));
        })
    };
    // Connect: the taf first, the person's pick (`ConnectRow.connectFor`); Ask Coach where no
    // taf needs it.
    let connect: Rc<dyn Fn(App, gtk::Widget)> = {
        let (ctx, which_taf, open_coach) = (ctx.clone(), which_taf.clone(), open_coach.clone());
        Rc::new(move |app: App, over: gtk::Widget| match route(&ctx, &app) {
            ConnectFor::Taf(_) | ConnectFor::Ask => which_taf(app, over),
            ConnectFor::AskCoach => open_coach(),
            ConnectFor::NoTaf => {}
        })
    };
    // Reads the first page for what was typed, or the next page for the same search.
    let load: Rc<dyn Fn(bool)> = {
        let (w, ctx, state, redraw, device) = (
            w.clone(),
            ctx.clone(),
            state.clone(),
            redraw.clone(),
            device.clone(),
        );
        Rc::new(move |more: bool| {
            let (kept, languages) = match &ctx.source {
                ConnectSource::Cloud {
                    kept, languages, ..
                } => (kept.clone(), languages.clone()),
                ConnectSource::File { path, .. } => {
                    let mut s = state.borrow_mut();
                    if s.query.is_empty() {
                        s.apps = file_part(path, "answer").and_then(|b| ConnectApps::read(&b));
                    }
                    drop(s);
                    redraw();
                    return;
                }
                // The sample lists the reference set's apps, unsearched.
                ConnectSource::Sample {
                    screenshot: true, ..
                } => {
                    let mut s = state.borrow_mut();
                    if s.query.is_empty() {
                        s.apps = sample_apps();
                    }
                    drop(s);
                    redraw();
                    return;
                }
                _ => {
                    redraw();
                    return;
                }
            };
            let (request, p) = {
                let mut s = state.borrow_mut();
                if more {
                    let Some(shown) = &s.apps else { return };
                    let Some(cursor) = shown.next_cursor.clone() else {
                        return;
                    };
                    if s.loading_more {
                        return;
                    }
                    let p = path(&s.query, shown.provider.as_deref(), Some(&cursor));
                    s.loading_more = true;
                    (s.request, p)
                } else {
                    s.request += 1;
                    // A new search drops the last one's pages at once.
                    s.apps = None;
                    s.error = None;
                    (s.request, path(&s.query, None, None))
                }
            };
            let words = (*w).clone();
            let (state, redraw, device) = (state.clone(), redraw.clone(), device.clone());
            gtk::glib::spawn_future_local(async move {
                let read = gtk::gio::spawn_blocking(move || {
                    read_page(&words, &kept, &languages, &device, &p, more)
                })
                .await
                .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    if more {
                        s.loading_more = false;
                    }
                    // Only the newest search's answer is kept.
                    if s.request != request {
                        return;
                    }
                    match read {
                        Ok(page) => match (&mut s.apps, more) {
                            (Some(shown), true) => shown.add(page),
                            _ => s.apps = Some(page),
                        },
                        Err(e) => s.error = Some(e),
                    }
                }
                redraw();
            });
        })
    };
    // As the person types, a moment after the last key (`search-changed` waits for it).
    {
        let (state, load) = (state.clone(), load.clone());
        search.connect_search_changed(move |e| {
            state.borrow_mut().query = e.text().to_string();
            load(false);
        });
    }
    *draw.borrow_mut() = Some(Rc::new({
        let (w, ctx, list, state, load, open_coach, connect, open_app) = (
            w.clone(),
            ctx.clone(),
            list.clone(),
            state.clone(),
            load.clone(),
            open_coach.clone(),
            connect.clone(),
            open_app.clone(),
        );
        move || {
            while let Some(c) = list.first_child() {
                list.remove(&c);
            }
            let s = state.borrow();
            let l = |k: &str| w.line(k).unwrap_or_default();
            if let Some(why) = &s.error {
                list.append(&text(why, 14, 400, &["tafs-no"]));
            }
            let Some(apps) = &s.apps else {
                if s.error.is_none() {
                    let spinner = gtk::Spinner::new();
                    spinner.start();
                    spinner.set_halign(gtk::Align::Center);
                    spinner.set_margin_top(24);
                    list.append(&spinner);
                }
                return;
            };
            let removed: std::collections::BTreeSet<String> =
                apps.removed_tafs.iter().map(|t| t.agent.clone()).collect();
            let row_for = |app: &App| {
                let used = ctx.used_here(app, &removed);
                row(&ctx, app, &used, s.narrow, &connect, &open_app)
            };
            let (yours, discover) = split(&apps.apps);
            if !yours.is_empty() {
                list.append(&section_header(&l("connect.apps.yours")));
                let group = group();
                // The table's header on a wide window only (`if table { columns }`).
                if !s.narrow {
                    group.append(&columns(&w));
                }
                for (i, app) in yours.iter().enumerate() {
                    if i > 0 || !s.narrow {
                        group.append(&separator());
                    }
                    group.append(&row_for(app));
                }
                list.append(&group);
            }
            if !discover.is_empty() {
                list.append(&section_header(&l("connect.apps.discover")));
                let group = group();
                let mut first = true;
                // Connecting needs a taf: said over the apps to connect, never over Yours.
                if says_make_a_taf(discover.len(), false, ctx.source.connectable().len()) {
                    let note = text(
                        &l("connect.apps_connect_taf_make_one"),
                        13,
                        400,
                        &["tafs-dim"],
                    );
                    note.set_margin_top(14);
                    note.set_margin_bottom(14);
                    group.append(&note);
                    first = false;
                }
                if yours.is_empty() && !s.narrow {
                    if !first {
                        group.append(&separator());
                    }
                    group.append(&columns(&w));
                    first = false;
                }
                for app in &discover {
                    if !first {
                        group.append(&separator());
                    }
                    first = false;
                    group.append(&row_for(app));
                }
                list.append(&group);
            }
            if apps.next_cursor.is_some() {
                let more = gtk::Button::with_label(&if s.loading_more {
                    l("common.loading")
                } else {
                    l("connect.more_apps")
                });
                more.add_css_class("tafs-list-button");
                if let Some(label) = more.child() {
                    label.set_halign(gtk::Align::Start);
                }
                more.set_sensitive(!s.loading_more);
                let load = load.clone();
                more.connect_clicked(move |_| load(true));
                list.append(&more);
            }
            if apps.by_key {
                // An app nobody listed goes to Coach.
                let coach = gtk::Button::with_label(&l("connect.apps.not_listed"));
                coach.add_css_class("tafs-list-button");
                if let Some(label) = coach.child() {
                    label.set_halign(gtk::Align::Start);
                }
                let open_coach = open_coach.clone();
                coach.connect_clicked(move |_| open_coach());
                list.append(&coach);
            }
        }
    }));
    redraw();
    load(false);
    read_connections(&ctx, &device, redraw.clone());
    *ctx.reread.borrow_mut() = {
        let (weak, device, redraw) = (Rc::downgrade(&ctx), device.clone(), redraw.clone());
        Some(Rc::new(move || {
            if let Some(ctx) = weak.upgrade() {
                read_connections(&ctx, &device, redraw.clone());
            }
        }))
    };
    // A screenshot's page, once the list is read.
    if let Some(open) = OPEN.with(|o| o.take()) {
        let apps = state.borrow().apps.clone();
        if let Some(apps) = apps {
            match open {
                // `-screen connect-app`: the app with the most connections, the first of equals.
                "connect-app" | "connect-connection" | "connect-remove" => {
                    let mut best: Option<&App> = None;
                    for a in &apps.apps {
                        let grants = ctx.grants.borrow();
                        let n = connections_of(a, &grants).len();
                        if best.is_none_or(|b| n > connections_of(b, &grants).len()) {
                            best = Some(a);
                        }
                    }
                    if let Some(app) = best.cloned() {
                        crate::connections::OPEN.with(|o| o.set(Some(open)));
                        open_app(app);
                    }
                }
                "connect-which-taf" | "connect-approve" => {
                    let asks = apps
                        .apps
                        .iter()
                        .find(|a| route(&ctx, a) == ConnectFor::Ask)
                        .cloned();
                    if let Some(app) = asks {
                        let over: gtk::Widget = outer.clone().upcast();
                        if open == "connect-which-taf" {
                            let which_taf = which_taf.clone();
                            gtk::glib::idle_add_local_once(move || which_taf(app, over));
                        } else {
                            let tafs =
                                connectable_for(ctx.source.connectable(), app.needed_by.as_deref());
                            if let Some(t) = tafs.first() {
                                let (connect_for_taf, id) = (connect_for_taf.clone(), t.id.clone());
                                gtk::glib::idle_add_local_once(move || connect_for_taf(app, id));
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    // Below the table's least width (App's and the three others' least, with their spacing,
    // inside the page's margins), each app is one row with a second line, as on an iPhone.
    let least = tafs_core::connect::APP_LEAST
        + tafs_core::connect::LEAST.iter().sum::<f64>()
        + tafs_core::connect::SPACING * 3.0
        + 32.0;
    let bin = adw::BreakpointBin::new();
    bin.set_size_request(320, 240);
    bin.set_child(Some(&screens.stack));
    let narrow = adw::Breakpoint::new(adw::BreakpointCondition::new_length(
        adw::BreakpointConditionLengthType::MaxWidth,
        least,
        adw::LengthUnit::Px,
    ));
    {
        let (state, redraw) = (state.clone(), redraw.clone());
        narrow.connect_apply(move |_| {
            state.borrow_mut().narrow = true;
            redraw();
        });
    }
    {
        let (state, redraw) = (state.clone(), redraw.clone());
        narrow.connect_unapply(move |_| {
            state.borrow_mut().narrow = false;
            redraw();
        });
    }
    bin.add_breakpoint(narrow);
    bin.upcast()
}

/// Whom an app's Connect is for (`route`): never a taf the list was opened from, as Connect apps
/// here is always the sidebar's.
fn route(ctx: &Ctx, app: &App) -> ConnectFor {
    let tafs: Vec<String> = connectable_for(ctx.source.connectable(), app.needed_by.as_deref())
        .into_iter()
        .map(|t| t.id)
        .collect();
    connect_for(None, &tafs, app.needed_by.is_some())
}

/// A section's header, as the iPad's inset list draws it: 17 semibold, soft, inset.
pub(crate) fn section_header(words: &str) -> gtk::Label {
    let h = one_line(words, 17, 600, &["tafs-soft"]);
    h.set_margin_start(12);
    h.set_margin_top(10);
    h
}

/// The inset list's group: a white panel with corners of 18.
pub(crate) fn group() -> gtk::Box {
    let g = gtk::Box::new(gtk::Orientation::Vertical, 0);
    g.add_css_class("tafs-group");
    g
}

pub(crate) fn separator() -> gtk::Separator {
    let s = gtk::Separator::new(gtk::Orientation::Horizontal);
    s.add_css_class("tafs-line");
    s
}

/// `ConnectColumns`: four cells laid out at the iPad's widths for the row's width, each given
/// the row's height and centred in it.
fn table_row(cells: [gtk::Widget; 4]) -> gtk::Widget {
    let row: columns_row::ColumnsRow = gtk::glib::Object::new();
    for c in &cells {
        c.set_valign(gtk::Align::Center);
        c.set_parent(&row);
    }
    row.upcast()
}

mod columns_row {
    use gtk::glib;
    use gtk::prelude::*;
    use gtk::subclass::prelude::*;
    use tafs_core::connect::{widths, APP_LEAST, SPACING, WIDE};

    fn ideal() -> f64 {
        APP_LEAST + WIDE.iter().sum::<f64>() + SPACING * 3.0
    }

    fn children(w: &gtk::Widget) -> Vec<gtk::Widget> {
        let mut v = Vec::new();
        let mut c = w.first_child();
        while let Some(child) = c {
            c = child.next_sibling();
            v.push(child);
        }
        v
    }

    #[derive(Default)]
    pub struct Imp;

    #[glib::object_subclass]
    impl ObjectSubclass for Imp {
        const NAME: &'static str = "TafsConnectColumns";
        type Type = ColumnsRow;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Imp {
        fn dispose(&self) {
            for c in children(self.obj().upcast_ref()) {
                c.unparent();
            }
        }
    }

    impl WidgetImpl for Imp {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            if orientation == gtk::Orientation::Horizontal {
                return (0, ideal() as i32, -1, -1);
            }
            let total = if for_size > 0 {
                f64::from(for_size)
            } else {
                ideal()
            };
            let mut h = (0, 0);
            for (c, width) in children(self.obj().upcast_ref()).iter().zip(widths(total)) {
                let (min, nat, _, _) = c.measure(gtk::Orientation::Vertical, width as i32);
                h = (h.0.max(min), h.1.max(nat));
            }
            (h.0, h.1, -1, -1)
        }

        fn size_allocate(&self, width: i32, height: i32, _baseline: i32) {
            let mut x = 0.0;
            for (c, w) in children(self.obj().upcast_ref())
                .iter()
                .zip(widths(f64::from(width)))
            {
                c.size_allocate(&gtk::Allocation::new(x as i32, 0, w as i32, height), -1);
                x += w + SPACING;
            }
        }
    }

    glib::wrapper! {
        pub struct ColumnsRow(ObjectSubclass<Imp>) @extends gtk::Widget;
    }
}

/// The table's header (`columns`): 12 semibold, dim.
fn columns(w: &WordBook) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let cell = |k: &str| one_line(&l(k), 12, 600, &["tafs-dim"]).upcast::<gtk::Widget>();
    let r = table_row([
        cell("connect.apps.column_app"),
        cell("connect.apps.column_how"),
        cell("connect.apps.column_whose"),
        cell("connect.apps.column_status"),
    ]);
    r.set_margin_top(14);
    r.set_margin_bottom(14);
    r.update_state(&[gtk::accessible::State::Hidden(true)]);
    r
}

/// One app (`row`, `cells`): its name and what it is, how it connects, whose it is, and where it
/// stands with who uses it, then its ✓ or Connect. An app with connections opens them, with a
/// chevron: a connected one always, and one this computer holds a connection to that isn't
/// connected (a declined sign-in), its Connect still beside it.
fn row(
    ctx: &Ctx,
    app: &App,
    used_here: &[String],
    narrow: bool,
    connect: &Rc<dyn Fn(App, gtk::Widget)>,
    open_app: &Rc<dyn Fn(App)>,
) -> gtk::Widget {
    let w = &ctx.w;
    let said = Row::new(w, app, used_here).unwrap_or_else(|_| Row {
        how: String::new(),
        whose: None,
        status: None,
        used_by: None,
        line: None,
        setup: None,
    });
    let opens = opens_connections(app, &ctx.grants.borrow());
    // The logo, from tafs cloud only, beside the name (`name`). The name on one line, cut at its
    // end only as a last resort; what it is under it, in two.
    let named = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    if let Some(url) = tafs_core::recipes::logo_url(app.logo.as_deref()) {
        let logo = crate::recipes::logo(url, 24);
        logo.set_valign(gtk::Align::Center);
        named.append(&logo);
    }
    let name = gtk::Box::new(gtk::Orientation::Vertical, 1);
    name.set_hexpand(true);
    named.append(&name);
    name.append(&one_line(
        app.display_name(),
        16,
        600,
        &[if app.needs_setup() {
            "tafs-soft"
        } else {
            "tafs-ink"
        }],
    ));
    if let Some(sub) = &app.subtitle {
        let s = text(sub, 13, 400, &["tafs-soft"]);
        s.set_lines(2);
        s.set_ellipsize(gtk::pango::EllipsizeMode::End);
        name.append(&s);
    }
    let end = trailing(ctx, app, connect);
    let chevron = opens.then(|| {
        let c = symbol("go-next-symbolic", 13);
        c.add_css_class("tafs-dim");
        c.set_valign(gtk::Align::Center);
        c
    });
    // A narrow window (the iPhone's): the name, and how it connects, whose it is and where it
    // stands in one line under it, then its ✓ or Connect.
    let r: gtk::Widget = if narrow {
        let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
        v.set_hexpand(true);
        v.set_valign(gtk::Align::Center);
        v.append(&named);
        v.append(&text(
            &said.second(),
            13,
            400,
            &[if app.needs_reconnect() {
                "tafs-amber-text"
            } else {
                "tafs-dim"
            }],
        ));
        r.append(&v);
        if let Some(t) = end {
            t.set_valign(gtk::Align::Center);
            r.append(&t);
        }
        if let Some(c) = &chevron {
            r.append(c);
        }
        r.upcast()
    } else {
        let how = one_line(&said.how, 14, 400, &["tafs-soft"]);
        let whose = one_line(said.whose.as_deref().unwrap_or(""), 14, 400, &["tafs-soft"]);
        let status = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        if let Some(setup) = said.setup.as_ref().filter(|s| !s.is_empty()) {
            let t = text(setup, 13, 400, &["tafs-soft"]);
            t.set_lines(2);
            t.set_ellipsize(gtk::pango::EllipsizeMode::End);
            t.set_hexpand(true);
            status.append(&t);
        } else {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
            v.set_hexpand(true);
            if let Some(st) = &said.status {
                v.append(&one_line(
                    st,
                    14,
                    600,
                    &[if app.connected() {
                        "tafs-ok"
                    } else {
                        "tafs-amber-text"
                    }],
                ));
                if let Some(used) = &said.used_by {
                    let u = text(used, 12, 400, &["tafs-dim"]);
                    u.set_lines(2);
                    u.set_ellipsize(gtk::pango::EllipsizeMode::End);
                    v.append(&u);
                }
            }
            status.append(&v);
        }
        if let Some(t) = end {
            status.append(&t);
        }
        if let Some(c) = &chevron {
            status.append(c);
        }
        table_row([
            named.upcast(),
            how.upcast(),
            whose.upcast(),
            status.upcast(),
        ])
    };
    r.set_size_request(-1, 52);
    r.set_margin_top(10);
    r.set_margin_bottom(10);
    if opens {
        // The row opens its connections; a Connect beside it takes its own click.
        r.set_cursor_from_name(Some("pointer"));
        let click = gtk::GestureClick::new();
        let (open_app, opened) = (open_app.clone(), app.clone());
        click.connect_released(move |g, _, _, _| {
            g.set_state(gtk::EventSequenceState::Claimed);
            open_app(opened.clone());
        });
        r.add_controller(click);
        r.update_property(&[gtk::accessible::Property::Label(app.display_name())]);
    }
    r
}

/// `trailing`: a ✓ for a connected app; else Connect (or Sign in again, or Ask Coach where no taf
/// needs it) as wide as its words, off while no taf can have it;
/// nothing for an app not set up yet.
fn trailing(ctx: &Ctx, app: &App, connect: &Rc<dyn Fn(App, gtk::Widget)>) -> Option<gtk::Widget> {
    let w = &ctx.w;
    if app.needs_setup() {
        None
    } else if app.connected() {
        let check = symbol("tafs-checkmark-symbolic", 17);
        check.add_css_class("tafs-ok");
        check.update_property(&[gtk::accessible::Property::Label(
            &w.line("connect.status.connected").unwrap_or_default(),
        )]);
        Some(check.upcast())
    } else {
        let route = route(ctx, app);
        let label = w
            .line(if route == ConnectFor::AskCoach {
                "coach.ask"
            } else if app.needs_reconnect() {
                "connect.sign_in_again"
            } else {
                "connect.apps.connect"
            })
            .unwrap_or_default();
        let b = outline(&label);
        b.set_hexpand(false);
        b.add_css_class("compact");
        b.set_valign(gtk::Align::Center);
        b.set_sensitive(route != ConnectFor::NoTaf);
        let (connect, app) = (connect.clone(), app.clone());
        b.connect_clicked(move |b| connect(app.clone(), b.clone().upcast()));
        Some(b.upcast())
    }
}
