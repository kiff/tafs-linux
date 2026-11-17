//! A taf's page beside the sidebar (`TafDetailView`): its head, its tabs, its Card bar, and its
//! Activity (the sample's, `activity(_:)`; a tafs cloud taf's own list comes with its read).

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::card::Card;
use tafs_core::needs_you::CardLines;
use tafs_core::record::{Entry, Tone};
use tafs_core::taf_page::{
    activity, card_bar, shows_chip, ActivityFilter, CardMeta, ChipTone, TafState,
};
use tafs_core::words::{upper, WordBook};

use crate::widgets::{avatar, eyebrow, one_line, symbol, text};

/// Draws the filters and the days again, after a choice.
type Redraw = Rc<dyn Fn()>;

/// What a taf's page draws.
pub struct TafPage {
    pub key: String,
    pub name: String,
    pub initial: String,
    pub color: i64,
    pub state: TafState,
    pub attention: bool,
    pub card: Option<Card>,
    pub card_title: String,
    pub card_environment: String,
    pub card_live: bool,
    pub card_active: bool,
    pub card_count: usize,
    pub revoked: bool,
    /// Its held calls' cards, from Needs you.
    pub held: Vec<CardLines>,
    /// How its chat reads its held calls again from KIFF: a tafs cloud taf's.
    pub held_reread: Option<crate::taf_chat::HeldReread>,
    /// The sample draws its Activity from the Record; a tafs cloud taf's is its own read.
    pub sample: bool,
    /// This person manages the taf (`manages`, KIFF's access read).
    pub manages: bool,
    /// Its Cards whole, for the Card tab, and who issued them (`issuer(of:)`).
    pub cards: Vec<tafs_core::cards::FullCard>,
    pub issued_by: String,
    /// Opens on the Card tab (`-screen card`).
    pub opens_on_card: bool,
    /// Opens on the Setup tab (`--page taf-setup:<id>`).
    pub opens_on_setup: bool,
    /// Its doors open over the page (`DoorsView`, `--page doors:<id>`), or one door's screen
    /// (`--page doors:<id>:<door>`), for a screenshot.
    pub opens_doors: Option<Option<String>>,
    /// Setup's screen open over its tab (`-screen starts`, `-screen results`), for a screenshot.
    pub opens_setup_screen: Option<crate::starts_when::Screen>,
    /// This person's sign-in email: their own key at a door reads "You", and Email me's line.
    pub my_email: Option<String>,
    /// KIFF keeps this session view-only (`approval_mode: view`): Revoke's line in its place.
    pub view_only: bool,
    /// Its Cards' ids at KIFF, revoked by Revoke.
    pub card_ids: Vec<String>,
    /// Where Setup's screens read and change its dial (`GET …/knob`), and what they say of it
    /// from Home: no schedule (`worksFromChat`), its next run, paused, and where tafs cloud
    /// delivers its results itself.
    pub knob: crate::starts_when::KnobSource,
    pub works_from_chat: bool,
    pub next_run_line: Option<String>,
    pub paused: bool,
    pub results: Option<Vec<serde_json::Value>>,
    /// tafs cloud's id for it, and where its Activity is read from.
    pub cloud_id: Option<String>,
    pub source: ActivitySource,
    /// Its job in words (`taf.job`), and where its Setup tab is read.
    pub job: String,
    pub setup: crate::setup_pane::SetupSource,
    /// Where its doors are read and its knocks answered (`TafKnocksPanel`), where tafs cloud
    /// offers doors.
    pub doors: crate::knocks::DoorsSource,
    /// Its Card unit requests as Home sent them, drawn until its Activity's own are read, and
    /// whether this person may use Coach (their Coach button, dimmed).
    pub card_units: Vec<tafs_core::waits::CardUnitException>,
    /// The steps it handed its owner, as Home sent them (with the vendor's link when tafs cloud
    /// vouched for it): a request's step is drawn from here where it's listed.
    pub steps: Vec<tafs_core::waits::YourStep>,
    pub may_use_coach: bool,
    /// Where its work is read (`WorkPanel`), and its health as Home read it.
    pub work: crate::work_panel::WorkSource,
    pub health: Option<tafs_core::home::Health>,
    /// Where its runs are read and Run now sent, and `RunsPanel`'s line (`runs::next_line`).
    pub runs: crate::runs_panel::RunsSource,
    pub runs_line: Option<(String, bool)>,
    /// tafs cloud's words for the zone its times are in, and its schedule's when another
    /// (`timeZoneWords`, `scheduleTimeZoneWords`): signed in with KIFF.
    pub time_zone_words: Option<String>,
    pub schedule_time_zone_words: Option<String>,
    /// Where its chat is read and what's written goes (`TafChatView`): a tafs cloud taf's, signed
    /// in with KIFF.
    pub chat: crate::taf_chat::ChatSource,
}

/// Where a tafs cloud taf's Activity pages come from.
#[derive(Clone)]
pub enum ActivitySource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A contract file (`--home`'s screenshots): `taf-activity.json` beside it.
    File(std::path::PathBuf),
    None,
}

pub fn page(
    w: Rc<WordBook>,
    t: &TafPage,
    entries: Rc<Vec<Entry>>,
    halt_row: Option<gtk::Button>,
    open_usage: Rc<dyn Fn()>,
    open_coach: Rc<dyn Fn()>,
    open_connect: Rc<dyn Fn()>,
) -> gtk::Widget {
    // Coach from a tafs cloud taf's page is Coach on this taf (`improveTaf`, `askCoachAbout`): a
    // new conversation whose first message goes with the taf's context, the words in the box.
    // How Activity is narrowed to an app's calls, once it's drawn.
    let selects_apps: Rc<RefCell<Option<Selects>>> = Rc::new(RefCell::new(None));
    let open_coach: Rc<dyn Fn()> = match (&t.cloud_id, t.sample) {
        (Some(id), false) if t.manages && t.may_use_coach => {
            let (id, name) = (id.clone(), t.name.clone());
            Rc::new(move || crate::coach_start::improve(&id, &name, None))
        }
        _ => open_coach,
    };
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 12);
    outer.set_margin_start(16);
    outer.set_margin_end(16);
    outer.set_margin_top(12);

    // One row: avatar, name and state.
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    head.append(&avatar(&t.initial, t.color, 34));
    let name = one_line(&t.name, 20, 700, &["tafs-ink"]);
    name.set_hexpand(true);
    head.append(&name);
    if shows_chip(t.state, t.attention) {
        let chip = gtk::Label::new(Some(&t.state.label(&w).unwrap_or_default()));
        chip.add_css_class("tafs-state-chip");
        chip.add_css_class(match t.state.tone() {
            ChipTone::Ok => "ok",
            ChipTone::Amber => "amber",
            ChipTone::No => "no",
            ChipTone::Soft => "soft",
        });
        chip.set_valign(gtk::Align::Center);
        head.append(&chip);
    }
    outer.append(&head);
    // Its versions, and its trial while one is open, for the person who manages a tafs cloud taf.
    if let (false, true, Some(id)) = (t.sample, t.manages, &t.cloud_id) {
        outer.append(&crate::versions::link(
            w.clone(),
            crate::versions::VersionsTaf {
                id: id.clone(),
                name: t.name.clone(),
                chat: t.chat.clone(),
            },
        ));
    }

    // Chat, Activity, Card, Setup, as the system's segmented control. Chat is a tafs cloud taf's,
    // signed in with KIFF, and the page opens on it (the sample's chat is there, dimmed); Setup too
    // (the sample's isn't ported).
    let has_chat = !t.sample && t.cloud_id.is_some() && !t.chat.is_none();
    let has_setup = !t.sample
        && t.cloud_id.is_some()
        && !matches!(t.setup, crate::setup_pane::SetupSource::None);
    let tabs = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    tabs.add_css_class("tafs-segmented");
    tabs.set_homogeneous(true);
    let mut first: Option<gtk::ToggleButton> = None;
    let mut buttons = Vec::new();
    for (key, enabled) in [
        ("taf.section.chat", has_chat),
        ("taf.section.activity", true),
        ("taf.section.card", true),
        ("taf.section.setup", has_setup),
    ] {
        let b = gtk::ToggleButton::with_label(&w.line(key).unwrap_or_default());
        b.add_css_class("segment");
        if let Some(f) = &first {
            b.set_group(Some(f));
        } else {
            first = Some(b.clone());
        }
        b.set_sensitive(enabled);
        tabs.append(&b);
        buttons.push(b);
    }
    let (chat_tab, activity_tab, card_tab, setup_tab) = (
        buttons[0].clone(),
        buttons[1].clone(),
        buttons[2].clone(),
        buttons[3].clone(),
    );
    outer.append(&tabs);

    // The Card in front, one line above the activity.
    let mut card_bar_box: Option<gtk::Box> = None;
    if let Some(card) = &t.card {
        let meta = CardMeta {
            title: &t.card_title,
            environment: &t.card_environment,
            live: t.card_live,
            active: t.card_active,
        };
        if let Ok(lines) = card_bar(&w, card, &meta, t.card_count, t.revoked) {
            // It opens the Card tab, and isn't there on it.
            let bar = card_bar_widget(&lines, t.color);
            let click = gtk::GestureClick::new();
            let opens = card_tab.clone();
            click.connect_released(move |_, _, _, _| opens.set_active(true));
            bar.add_controller(click);
            bar.set_cursor_from_name(Some("pointer"));
            outer.append(&bar);
            card_bar_box = Some(bar);
        }
    }

    let body = gtk::Box::new(gtk::Orientation::Vertical, 16);
    body.set_margin_top(4);
    body.set_margin_bottom(12);
    // `TafActivityList.parts`: for whoever manages the taf, its doors' knocks come first.
    if let (false, true, Some(id)) = (t.sample, t.manages, &t.cloud_id) {
        body.append(&crate::knocks::panel_for(
            w.clone(),
            id.clone(),
            Some(t.name.clone()),
            t.doors.clone(),
        ));
    }
    // Its held calls: here, or in the side column beside it on a wide window.
    let held_here = held_box(&w, &t.held);
    body.append(&held_here);
    if !t.sample {
        if let Some(id) = &t.cloud_id {
            // `TafActivityList.parts`: its held calls, its Card unit requests, its work, then the
            // list. The requests are its Activity's last page's, once read.
            let units = gtk::Box::new(gtk::Orientation::Vertical, 14);
            let units_of = Units {
                w: w.clone(),
                home: t.card_units.clone(),
                discuss: t.may_use_coach && t.manages,
                into: units.clone(),
            };
            units_of.draw(None, None);
            body.append(&units);
            body.append(&crate::work_panel::panel_for(
                w.clone(),
                id.clone(),
                t.work.clone(),
                t.health.clone(),
                Rc::new(crate::work_panel::Opens {
                    usage: open_usage.clone(),
                    coach: {
                        let open = open_coach.clone();
                        Rc::new(move |say: String| {
                            crate::coach::set_draft(&say);
                            open();
                        })
                    },
                    manages: t.manages,
                    may_use_coach: t.may_use_coach,
                    steps: t.steps.clone(),
                }),
            ));
            // Its runs: when it runs next and Run now, then the runs tafs cloud kept, read again
            // once Run now started one.
            let (kept, reload) = crate::runs_panel::retained_runs(
                w.clone(),
                id.clone(),
                t.runs.clone(),
                t.manages,
                tafs_core::runs::offers_ask_coach(t.sample, t.may_use_coach, t.manages)
                    .then(|| open_coach.clone()),
            );
            if let Some(line) = &t.runs_line {
                if !matches!(t.runs, crate::runs_panel::RunsSource::None) {
                    body.append(&crate::runs_panel::runs_panel(
                        w.clone(),
                        id.clone(),
                        t.runs.clone(),
                        line.clone(),
                        reload,
                    ));
                }
            }
            body.append(&kept);
            let rows = Rc::new(RowOpens {
                taf: t.key.clone(),
                manages: t.manages,
                may_use_coach: t.may_use_coach,
                coach: open_coach.clone(),
                connect: open_connect.clone(),
                receipt: match &t.source {
                    ActivitySource::Cloud { token, languages } => {
                        Some(Rc::new(crate::receipt_sheet::ReceiptSource {
                            token: token.clone(),
                            languages: languages.clone(),
                            canned: None,
                            open: None,
                        }))
                    }
                    _ => None,
                },
            });
            let (list, select) =
                cloud_activity(w.clone(), id.clone(), t.source.clone(), units_of, rows);
            *selects_apps.borrow_mut() = Some(select);
            body.append(&list);
        }
    }
    if t.sample {
        let filters = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let days_box = gtk::Box::new(gtk::Orientation::Vertical, 10);
        body.append(&filters);
        body.append(&days_box);
        let chosen = Rc::new(RefCell::new(ActivityFilter::All));
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
        let key = t.key.clone();
        *draw.borrow_mut() = Some(Rc::new({
            let (w, filters, days_box, chosen, redraw) = (
                w.clone(),
                filters.clone(),
                days_box.clone(),
                chosen.clone(),
                redraw.clone(),
            );
            move || {
                while let Some(c) = filters.first_child() {
                    filters.remove(&c);
                }
                while let Some(c) = days_box.first_child() {
                    days_box.remove(&c);
                }
                let now_filter = *chosen.borrow();
                for f in ActivityFilter::ALL {
                    let b = gtk::Button::with_label(&f.title(&w).unwrap_or_default());
                    b.add_css_class("tafs-chip-button");
                    if f == now_filter {
                        b.add_css_class("selected");
                    }
                    let (chosen, redraw) = (chosen.clone(), redraw.clone());
                    b.connect_clicked(move |_| {
                        *chosen.borrow_mut() = f;
                        redraw();
                    });
                    filters.append(&b);
                }
                let days = activity(&w, &entries, &key, now_filter, chrono::Local::now())
                    .unwrap_or_default();
                if days.is_empty() {
                    let words = w
                        .line(if now_filter == ActivityFilter::All {
                            "common.nothing_yet"
                        } else {
                            "record.nothing_here"
                        })
                        .unwrap_or_default();
                    days_box.append(&text(&words, 15, 400, &["tafs-dim"]));
                }
                for day in days {
                    let h = gtk::Label::new(Some(&upper(&day.title)));
                    h.set_xalign(0.0);
                    h.add_css_class("tafs-day");
                    h.set_margin_top(6);
                    days_box.append(&h);
                    for e in day.lines {
                        days_box.append(&line(&w, e));
                    }
                }
            }
        }));
        redraw();
    }
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&body));
    scroller.set_vexpand(true);
    let card_scroller = gtk::ScrolledWindow::new();
    card_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    let card = card_pane(&w, t, &open_coach, &open_usage);
    // `HaltRow(tafID:framed:)`, as on the iPad: after the Card's own lines and How this is
    // counted, before Changes to how it runs, Usage and Revoke.
    if let Some(row) = halt_row {
        let next = ["taf.changes", "taf.usage"].iter().find_map(|n| {
            let mut c = card.first_child();
            while let Some(w) = c {
                if w.widget_name() == *n && w.is_visible() {
                    return Some(w);
                }
                c = w.next_sibling();
            }
            None
        });
        match next.and_then(|n| n.prev_sibling()) {
            Some(before) => card.insert_child_after(&row, Some(&before)),
            None => card.append(&row),
        }
    }
    card_scroller.set_child(Some(&card));
    card_scroller.set_vexpand(true);
    let panes = gtk::Stack::new();
    panes.add_named(&scroller, Some("activity"));
    panes.add_named(&card_scroller, Some("card"));
    // Its dial, shared by Setup's screens and its doors (`knobs[tafID]`), signed in with KIFF.
    let dial_knobs = match (
        &t.cloud_id,
        matches!(t.knob, crate::starts_when::KnobSource::None),
    ) {
        (Some(id), false) => Some({
            crate::starts_when::Knobs::new(
                w.clone(),
                crate::starts_when::KnobTaf {
                    id: id.clone(),
                    name: t.name.clone(),
                    initial: t.initial.clone(),
                    color: t.color,
                    manages: t.manages,
                    may_use_coach: t.may_use_coach,
                    works_from_chat: t.works_from_chat,
                    next_run: t.next_run_line.clone(),
                    paused: t.paused,
                    results: t.results.clone(),
                    email: t.my_email.clone(),
                    doors: t.doors.clone(),
                    work: t.work.clone(),
                    runs: t.runs.clone(),
                    open_coach: open_coach.clone(),
                },
                t.knob.clone(),
            )
        }),
        _ => None,
    };
    // Setup: read when it's first shown.
    if let (true, Some(id)) = (has_setup, &t.cloud_id) {
        let setup_scroller = gtk::ScrolledWindow::new();
        setup_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
        setup_scroller.set_vexpand(true);
        // At the page's own edges, with the tabs, as the Card tab and the iPad's Setup.
        let column = gtk::Box::new(gtk::Orientation::Vertical, 0);
        setup_scroller.set_child(Some(&column));
        panes.add_named(&setup_scroller, Some("setup"));
        let made = std::cell::Cell::new(false);
        let knobs = dial_knobs.clone();
        // An app's calls: Activity, narrowed to them (`activitySelection`).
        let open_activity: Rc<dyn Fn(String)> = {
            let (tab, selects) = (activity_tab.downgrade(), selects_apps.clone());
            Rc::new(move |query: String| {
                if let Some(select) = selects.borrow().clone() {
                    select(query);
                }
                if let Some(t) = tab.upgrade() {
                    t.set_active(true);
                }
            })
        };
        let (w, taf, source) = (
            w.clone(),
            Rc::new(crate::setup_pane::SetupTaf {
                id: id.clone(),
                name: t.name.clone(),
                initial: t.initial.clone(),
                color: t.color,
                job: t.job.clone(),
                manages: t.manages,
                discuss: t.may_use_coach && t.manages,
                may_use_coach: t.may_use_coach,
                knobs,
                open_coach: open_coach.clone(),
                open_connect: open_connect.clone(),
                open_activity,
            }),
            t.setup.clone(),
        );
        if let (Some(screen), Some(knobs)) = (t.opens_setup_screen, taf.knobs.clone()) {
            let once = std::cell::Cell::new(Some(knobs));
            outer.connect_map(move |o| {
                if let Some(knobs) = once.take() {
                    crate::starts_when::open(knobs, screen, o);
                }
            });
        }
        let panes = panes.clone();
        setup_tab.connect_toggled(move |b| {
            if b.is_active() {
                if !made.replace(true) {
                    column.append(&crate::setup_pane::pane(
                        w.clone(),
                        taf.clone(),
                        source.clone(),
                    ));
                }
                panes.set_visible_child_name("setup");
            }
        });
    }
    // Chat: read when it's first shown, and kept while the app runs.
    if let (true, Some(id)) = (has_chat, &t.cloud_id) {
        let holder = gtk::Box::new(gtk::Orientation::Vertical, 0);
        holder.set_vexpand(true);
        panes.add_named(&holder, Some("chat"));
        let made = std::cell::Cell::new(false);
        let (w, source) = (w.clone(), t.chat.clone());
        let taf = RefCell::new(Some(crate::taf_chat::ChatTaf {
            id: id.clone(),
            name: t.name.clone(),
            ask_coach: t.may_use_coach && t.manages,
            held: t.held.clone(),
            trial: false,
            reread: t.held_reread.clone(),
        }));
        let panes = panes.clone();
        chat_tab.connect_toggled(move |b| {
            if b.is_active() {
                if !made.replace(true) {
                    if let Some(taf) = taf.borrow_mut().take() {
                        holder.append(&crate::taf_chat::pane(w.clone(), taf, source.clone()));
                    }
                }
                panes.set_visible_child_name("chat");
            }
        });
    }
    // `sideColumn`: beside the chat or the activity, on a wide window, the calls waiting and the
    // Cards, 360 wide; the page keeps an iPhone's width (430) for the chat beside it, in a window
    // wider than tall (the iPad in landscape). Without it, the Card bar above, and the held calls in the activity.
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    panes.set_hexpand(true);
    row.append(&panes);
    let side_scroller = gtk::ScrolledWindow::new();
    side_scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    side_scroller.set_size_request(SIDE_WIDTH, -1);
    // 360 wide, never more: what's in it wanting room mustn't take it from the chat or the
    // activity.
    side_scroller.set_hexpand(false);
    side_scroller.set_child(Some(&side(&w, t, &card_tab)));
    side_scroller.set_visible(false);
    side_scroller.set_widget_name("taf.side");
    row.append(&side_scroller);
    outer.append(&row);
    {
        let (side, bar, held, card_tab, setup_tab) = (
            side_scroller.clone(),
            card_bar_box.clone(),
            held_here.clone(),
            card_tab.clone(),
            setup_tab.clone(),
        );
        let has_cards = !t.cards.is_empty();
        outer.add_tick_callback(move |o, _| {
            let width = o.width();
            // "Only in landscape": the iPad's page is wider than tall exactly when the iPad is
            // in landscape. Here that's the window's shape, as a 4:3 window less the sidebar
            // leaves a page about as tall as it's wide.
            let landscape = o
                .root()
                .map(|r| r.width() > r.height())
                .unwrap_or(width > o.height());
            let section_lt_card = !card_tab.is_active() && !setup_tab.is_active();
            let wide = has_cards
                && section_lt_card
                && landscape
                && width - SIDE_WIDTH - 32 >= CHAT_AT_LEAST;
            if side.is_visible() != wide {
                side.set_visible(wide);
            }
            if let Some(bar) = &bar {
                // Above the chat or the activity only (`section < 2`), never the Card or Setup.
                let shown = !wide && section_lt_card;
                if bar.is_visible() != shown {
                    bar.set_visible(shown);
                }
            }
            if held.is_visible() == wide {
                held.set_visible(!wide);
            }
            gtk::glib::ControlFlow::Continue
        });
    }
    {
        let panes = panes.clone();
        activity_tab.connect_toggled(move |b| {
            if b.is_active() {
                panes.set_visible_child_name("activity");
            }
        });
    }
    {
        let panes = panes.clone();
        card_tab.connect_toggled(move |b| {
            if b.is_active() {
                panes.set_visible_child_name("card");
            }
        });
    }
    if t.opens_on_setup && has_setup {
        setup_tab.set_active(true);
    } else if t.opens_on_card {
        card_tab.set_active(true);
    } else if has_chat {
        // The iPad opens a taf's page on its chat.
        chat_tab.set_active(true);
    } else {
        activity_tab.set_active(true);
    }
    // `DoorsView`, the doors' own dial over the door it's turned to: the iPad's page has no tab
    // for it at the pin, so a screenshot opens it over the page.
    if let (Some(focus), Some(id)) = (&t.opens_doors, &t.cloud_id) {
        if !matches!(t.doors, crate::knocks::DoorsSource::None) {
            let model = crate::doors::DoorsModel::new(w.clone(), id.clone(), t.doors.clone());
            let taf = crate::doors::DoorsTaf {
                name: t.name.clone(),
                initial: t.initial.clone(),
                color: t.color,
            };
            let opens = crate::doors::Opens {
                manages: t.manages,
                my_email: t.my_email.clone(),
                focus: focus.clone(),
                knob: dial_knobs
                    .clone()
                    .map(|k| (k, crate::dial::held_faces(&t.held))),
                ..Default::default()
            };
            let once = std::cell::Cell::new(Some((model, taf, opens)));
            outer.connect_map(move |o| {
                if let Some((model, taf, opens)) = once.take() {
                    crate::doors::sheet(model, taf, opens, o);
                }
            });
        }
    }
    crate::widgets::ground(&outer);
    // Setup's screens push here, beside the sidebar, with Back and their title in the bar
    // (`PanelLink`'s `NavigationLink`), as the iPad's do.
    let nav = adw::NavigationView::new();
    nav.set_widget_name(crate::starts_when::TAF_NAV);
    nav.add(&adw::NavigationPage::with_tag(&outer, &t.name, ROOT));
    nav.upcast()
}

/// The tag of a taf's page in its navigation.
const ROOT: &str = "taf";

/// The column of a taf's page, under its navigation: where the window puts what it adds.
pub fn column(page: &gtk::Widget) -> Option<gtk::Box> {
    use adw::prelude::NavigationPageExt;
    page.downcast_ref::<adw::NavigationView>()?
        .find_page(ROOT)?
        .child()?
        .downcast()
        .ok()
}

/// The Card tab (`card(_:)`): the Cards as a wallet stack, the front Card's lines, and Revoke.
fn card_pane(
    w: &Rc<WordBook>,
    t: &TafPage,
    open_coach: &Rc<dyn Fn()>,
    open_usage: &Rc<dyn Fn()>,
) -> gtk::Box {
    // `offersReconnect`: signed in, for whoever manages the taf and may use Coach.
    let reconnect: Option<Rc<dyn Fn()>> =
        (!t.sample && t.manages && t.may_use_coach).then(|| open_coach.clone());
    let pane = gtk::Box::new(gtk::Orientation::Vertical, 16);
    pane.set_margin_top(12);
    pane.set_margin_bottom(12);
    // The zone its times are in, then its schedule's when it runs in another (amber).
    if let Some(words) = &t.time_zone_words {
        pane.append(&text(words, 13, 400, &["tafs-soft"]));
    }
    if let Some(words) = &t.schedule_time_zone_words {
        pane.append(&text(words, 13, 400, &["tafs-amber-text"]));
    }
    let front: Rc<RefCell<Option<String>>> = Rc::new(RefCell::new(None));
    let stack_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    stack_box.set_halign(gtk::Align::Center);
    let lines_box = gtk::Box::new(gtk::Orientation::Vertical, 0);
    pane.append(&stack_box);
    pane.append(&lines_box);
    let cards = Rc::new(t.cards.clone());
    let (color, issued_by, revoked) = (t.color, t.issued_by.clone(), t.revoked);
    let draw: Rc<RefCell<Option<Redraw>>> = Rc::new(RefCell::new(None));
    *draw.borrow_mut() = Some(Rc::new({
        let (w, stack_box, lines_box, cards, front, draw, reconnect) = (
            w.clone(),
            stack_box.clone(),
            lines_box.clone(),
            cards.clone(),
            front.clone(),
            draw.clone(),
            reconnect.clone(),
        );
        move || {
            while let Some(c) = stack_box.first_child() {
                stack_box.remove(&c);
            }
            while let Some(c) = lines_box.first_child() {
                lines_box.remove(&c);
            }
            let order = tafs_core::cards::stack(&cards, front.borrow().as_deref());
            let overlay = gtk::Overlay::new();
            let n = order.len() as i32;
            let base = gtk::Box::new(gtk::Orientation::Vertical, 0);
            base.set_size_request(FACE_WIDTH, FACE_HEIGHT + (n - 1).max(0) * EDGE);
            overlay.set_child(Some(&base));
            for (i, (index, depth)) in order.iter().enumerate() {
                let card = &cards[*index];
                let face = face(&w, card, color, &issued_by, revoked, *depth);
                face.set_margin_top(i as i32 * EDGE);
                face.set_valign(gtk::Align::Start);
                // Tapping a Card behind brings it forward.
                let click = gtk::GestureClick::new();
                let (front, draw, id) = (front.clone(), draw.clone(), card.id.clone());
                click.connect_released(move |_, _, _, _| {
                    *front.borrow_mut() = Some(id.clone());
                    let f = draw.borrow().clone();
                    if let Some(f) = f {
                        f();
                    }
                });
                face.add_controller(click);
                overlay.add_overlay(&face);
            }
            stack_box.append(&overlay);
            // All of the front Card's limits: the face shows the main one.
            let shown = order.last().map(|(i, _)| &cards[*i]);
            if let Some(card) = shown {
                if !card.rules.is_empty() {
                    let panel = crate::widgets::panel(false);
                    for rule in &card.rules {
                        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                        let left = gtk::Box::new(gtk::Orientation::Vertical, 2);
                        left.append(&text(&rule.label, 15, 600, &["tafs-ink"]));
                        left.append(&text(
                            &rule.outside(&w).unwrap_or_default(),
                            12,
                            400,
                            &["tafs-dim"],
                        ));
                        row.append(&left);
                        let summary = text(
                            &rule.summary(&w).unwrap_or_default(),
                            13,
                            400,
                            &["tafs-mono", if rule.never { "tafs-no" } else { "tafs-ink" }],
                        );
                        summary.set_hexpand(true);
                        summary.set_xalign(1.0);
                        summary.set_justify(gtk::Justification::Right);
                        row.append(&summary);
                        panel.append(&row);
                    }
                    lines_box.append(&panel);
                }
                card_own_lines(&w, card, &lines_box, reconnect.as_ref());
            }
        }
    }));
    let f = draw.borrow().clone();
    if let Some(f) = f {
        f();
    }
    // Changes to how it runs: its versions, for whoever manages a tafs cloud taf.
    if let (false, true, Some(id)) = (t.sample, t.manages, &t.cloud_id) {
        pane.append(&crate::versions::changes_row(
            w.clone(),
            crate::versions::VersionsTaf {
                id: id.clone(),
                name: t.name.clone(),
                chat: t.chat.clone(),
            },
            t.setup.clone(),
        ));
    }
    // Its Usage row: its credits this period and its runs, in Plan & usage (`TafCreditsView`), for
    // whoever manages a tafs cloud taf, signed in.
    if let (false, true, Some(id)) = (t.sample, t.manages, &t.cloud_id) {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let icon = crate::widgets::symbol(crate::widgets::icon_for("chart.bar"), 15);
        icon.add_css_class("tafs-amber-text");
        row.append(&icon);
        let label = text(
            &w.line("usage.label").unwrap_or_default(),
            15,
            600,
            &["tafs-amber-text"],
        );
        label.set_hexpand(true);
        row.append(&label);
        let chevron = crate::widgets::symbol("go-next-symbolic", 13);
        chevron.add_css_class("tafs-dim");
        row.append(&chevron);
        let b = gtk::Button::new();
        b.set_child(Some(&row));
        b.add_css_class("tafs-panel-link");
        b.set_widget_name("taf.usage");
        let (open, id) = (open_usage.clone(), id.clone());
        b.connect_clicked(move |_| {
            crate::usage::open_runs_of(&id);
            open();
        });
        pane.append(&b);
    }
    // Revoke, for the sample or whoever manages the taf (`manages`, from KIFF's access read). A
    // view-only session (KIFF's `view`) isn't offered it, on every device alike: the shared line
    // says where it's done. Otherwise its question, then each Card revoked at KIFF as the
    // session, unsigned. The sample's is the sample's own, there, dimmed.
    if t.state != TafState::Revoked && (t.sample || t.manages) {
        if !t.sample && t.view_only {
            pane.append(&text(
                &w.line("taf.revoke_needs_approver").unwrap_or_default(),
                13,
                400,
                &["tafs-soft"],
            ));
        } else {
            let revoke = crate::widgets::outline(&w.line("taf.revoke").unwrap_or_default());
            revoke.add_css_class("refuse");
            revoke.set_sensitive(!t.sample);
            let why = text("", 13, 400, &["tafs-no"]);
            why.set_visible(false);
            let (w, name, ids, source, why2) = (
                w.clone(),
                t.name.clone(),
                t.card_ids.clone(),
                t.setup.clone(),
                why.clone(),
            );
            revoke.connect_clicked(move |b| {
                ask_revoke(&w, &name, ids.clone(), source.clone(), b, &why2)
            });
            pane.append(&revoke);
            pane.append(&why);
        }
    }
    pane
}

/// Revoke's question; its yes revokes each of the taf's Cards at KIFF as the session
/// (`POST /v1/me/mandates/{id}/revoke`, unsigned), then the button goes. What KIFF refused is said
/// under it.
fn ask_revoke(
    w: &Rc<WordBook>,
    name: &str,
    ids: Vec<String>,
    source: crate::setup_pane::SetupSource,
    button: &gtk::Button,
    why: &gtk::Label,
) {
    use adw::prelude::*;
    let l = |k: &str| w.line(k).unwrap_or_default();
    let ask = adw::AlertDialog::new(
        Some(
            &w.get("taf.revoke_title", &[("taf", name)])
                .unwrap_or_default(),
        ),
        Some(&l("taf.revoke_body")),
    );
    ask.add_response("cancel", &l("common.cancel"));
    ask.add_response("revoke", &l("taf.revoke_and_stop"));
    ask.set_response_appearance("revoke", adw::ResponseAppearance::Destructive);
    ask.set_default_response(Some("cancel"));
    ask.set_close_response("cancel");
    let (w, b, why) = (w.clone(), button.downgrade(), why.downgrade());
    ask.connect_response(None, move |_, r| {
        if r != "revoke" {
            return;
        }
        let (w, ids, source, b, why) = (
            w.clone(),
            ids.clone(),
            source.clone(),
            b.clone(),
            why.clone(),
        );
        gtk::glib::spawn_future_local(async move {
            let words: WordBook = (*w).clone();
            let failed = gtk::gio::spawn_blocking(move || -> Option<String> {
                let crate::setup_pane::SetupSource::Cloud { token, languages } = &source else {
                    // A screenshot's Cards: nothing is sent.
                    return None;
                };
                for id in &ids {
                    let call = tafs_core::signed::Unsigned::new(
                        "POST",
                        tafs_core::signed::revoke_path(id),
                        b"{}".to_vec(),
                    )?;
                    match crate::kiff::kiff_send(token, languages, &call) {
                        Ok(a) if (200..300).contains(&a.status) => {}
                        Ok(a) => {
                            return Some(
                                tafs_core::signin::refusal(&a, Some(&words))
                                    .words(&words)
                                    .unwrap_or_default(),
                            )
                        }
                        Err(e) => return Some(e.words(&words).unwrap_or_default()),
                    }
                }
                None
            })
            .await
            .unwrap_or(None);
            match failed {
                Some(reason) => {
                    if let Some(why) = why.upgrade() {
                        why.set_label(
                            &w.get("card.revoke_failed", &[("why", &reason)])
                                .unwrap_or(reason),
                        );
                        why.set_visible(true);
                    }
                }
                None => {
                    if let Some(b) = b.upgrade() {
                        b.set_visible(false);
                    }
                }
            }
        });
    });
    ask.present(Some(button));
}

/// The front Card's lines in tafs cloud's and KIFF's own words, under its limits (`card(_:)`):
/// its totals for each item apart; How this is counted, opening each tool's lines; what's
/// pending, for a Card that counts for real; and why its total has no currency, with Reconnect it
/// in Coach (`offersReconnect`).
fn card_own_lines(
    w: &Rc<WordBook>,
    card: &tafs_core::cards::FullCard,
    into: &gtk::Box,
    reconnect: Option<&Rc<dyn Fn()>>,
) {
    let lines = gtk::Box::new(gtk::Orientation::Vertical, 12);
    lines.set_margin_top(16);
    for line in &card.extra.each_item {
        lines.append(&text(line, 14, 400, &["tafs-ink"]));
    }
    if !card.extra.counted.is_empty() {
        let title = w.line("card.how_counted").unwrap_or_default();
        let p = crate::widgets::panel(false);
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let icon = crate::widgets::symbol("view-list-symbolic", 16);
        icon.add_css_class("tafs-amber-text");
        h.append(&icon);
        let label = text(&title, 15, 600, &["tafs-amber-text"]);
        label.set_hexpand(true);
        h.append(&label);
        let chevron = crate::widgets::symbol("go-next-symbolic", 13);
        chevron.add_css_class("tafs-dim");
        h.append(&chevron);
        p.append(&h);
        let b = gtk::Button::new();
        b.set_child(Some(&p));
        b.add_css_class("tafs-plain");
        b.set_widget_name("card-counted");
        let counted = card.extra.counted.clone();
        b.connect_clicked(move |b| counted_sheet(&title, &counted, b));
        lines.append(&b);
    }
    for line in card.pending_shown() {
        lines.append(&text(line, 13, 400, &["tafs-soft"]));
    }
    // Its total has no currency because a tool it grants isn't connected any more: KIFF's words,
    // and for whoever manages the taf and may use Coach, signed in, Coach to connect it again.
    if let Some(words) = &card.extra.unit_words {
        let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
        v.append(&text(words, 13, 400, &["tafs-soft"]));
        if let Some(open) = reconnect {
            let b =
                gtk::Button::with_label(&w.line("connect.reconnect_in_coach").unwrap_or_default());
            b.add_css_class("tafs-link");
            b.set_halign(gtk::Align::Start);
            b.set_widget_name("card-reconnect");
            let open = open.clone();
            b.connect_clicked(move |_| open());
            v.append(&b);
        }
        lines.append(&v);
    }
    if lines.first_child().is_some() {
        into.append(&lines);
    }
}

/// Card › How this is counted (`CountedView`): each tool's title, then KIFF's lines for how it's
/// counted, as the owner approved them. The app adds no words of its own.
fn counted_sheet(
    title: &str,
    counted: &[tafs_core::cards::Counted],
    parent: &impl IsA<gtk::Widget>,
) {
    use adw::prelude::*;
    let column = gtk::Box::new(gtk::Orientation::Vertical, 16);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(16);
    column.set_margin_bottom(20);
    for tool in counted {
        column.append(&eyebrow(&tool.title, false));
        let p = crate::widgets::panel(false);
        for line in &tool.lines {
            p.append(&text(line, 15, 400, &["tafs-ink"]));
        }
        column.append(&p);
    }
    // Pushed on the taf's page with Back and its title in the bar, as the iPad's `PanelLink`;
    // a sheet only off a taf's page.
    if let Some(nav) = crate::starts_when::taf_nav(parent) {
        nav.push(&crate::starts_when::page(title, &column));
        return;
    }
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&column));
    scroller.set_vexpand(true);
    let header = adw::HeaderBar::new();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    let dialog = adw::Dialog::new();
    dialog.set_title(title);
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(520);
    dialog.set_content_height(520);
    dialog.present(Some(parent));
}

/// A Card's face's size: a payment card's shape, at most 300 wide.
const FACE_WIDTH: i32 = 300;
const FACE_HEIGHT: i32 = 189;
/// How much of a Card behind shows: `WalletLayout` places each 26 below the one before (the
/// iPad's `CardStack.edge`, 40, isn't what it draws).
const EDGE: i32 = 26;

/// `TafCardFace`: its title and environment, its figure and words, its facts, who issued it and
/// until when, white on the taf's colour (the dim colour when it isn't live).
/// The side column's width, and the chat's least width beside it (`sideWidth`, `chatAtLeast`).
const SIDE_WIDTH: i32 = 360;
const CHAT_AT_LEAST: i32 = 430;

/// A taf's held calls under NEEDS YOU in amber; nothing when none waits.
fn held_box(w: &Rc<WordBook>, held: &[tafs_core::needs_you::CardLines]) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 16);
    if !held.is_empty() {
        b.append(&eyebrow(
            &w.line("needs_you.title").unwrap_or_default(),
            true,
        ));
        for card in held {
            b.append(&crate::needs_you::held_card(w, card));
        }
    }
    b
}

/// `side`: the calls waiting, then CARDS with their stack, and All N Cards, which opens the Card
/// tab.
fn side(w: &Rc<WordBook>, t: &TafPage, card_tab: &gtk::ToggleButton) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 16);
    v.set_margin_top(4);
    v.set_margin_bottom(12);
    v.append(&held_box(w, &t.held));
    let n = t.cards.len() as i64;
    v.append(&eyebrow(
        &w.count("card.heading", n, &[]).unwrap_or_default(),
        false,
    ));
    let order = tafs_core::cards::stack(&t.cards, None);
    let overlay = gtk::Overlay::new();
    let base = gtk::Box::new(gtk::Orientation::Vertical, 0);
    base.set_size_request(
        FACE_WIDTH,
        FACE_HEIGHT + (order.len() as i32 - 1).max(0) * EDGE,
    );
    overlay.set_child(Some(&base));
    for (i, (index, depth)) in order.iter().enumerate() {
        let f = face(
            w,
            &t.cards[*index],
            t.color,
            &t.issued_by,
            t.revoked,
            *depth,
        );
        f.set_margin_top(i as i32 * EDGE);
        f.set_valign(gtk::Align::Start);
        overlay.add_overlay(&f);
    }
    overlay.set_halign(gtk::Align::Center);
    v.append(&overlay);
    let all = crate::widgets::outline(&w.count("card.all_of", n, &[]).unwrap_or_default());
    let tab = card_tab.clone();
    all.connect_clicked(move |_| tab.set_active(true));
    v.append(&all);
    v
}

fn face(
    w: &WordBook,
    card: &tafs_core::cards::FullCard,
    color: i64,
    issued_by: &str,
    revoked: bool,
    depth: usize,
) -> gtk::Overlay {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    v.add_css_class("tafs-face");
    v.add_css_class(&if card.live {
        crate::style::avatar_class(color)
    } else {
        "dimfill".to_string()
    });
    if revoked || !card.active {
        v.add_css_class("faded");
    }
    v.set_size_request(FACE_WIDTH, FACE_HEIGHT);
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = one_line(&card.title, 12, 600, &["tafs-mono"]);
    title.set_hexpand(true);
    top.append(&title);
    if let Some(env) = card.environment_label() {
        let pill = gtk::Label::new(Some(&env));
        pill.add_css_class("tafs-env-pill");
        top.append(&pill);
    }
    v.append(&top);
    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    v.append(&spacer);
    if let Ok(Some((figure, words))) = card.totals.hero(w) {
        let h = gtk::Box::new(gtk::Orientation::Vertical, 2);
        h.append(&one_line(&figure, 34, 700, &[]));
        h.append(&one_line(&words, 12, 500, &["tafs-face-words"]));
        v.append(&h);
    }
    for fact in card.facts(w).unwrap_or_default() {
        // One line, shrunk to 80% before it's cut (`minimumScaleFactor(0.8)`): a monospaced
        // character is 0.6 of its size, and the face has 268 across inside its padding.
        let fits = (f64::from(FACE_WIDTH - 32) / (0.6 * 11.0)) as usize;
        let size = if fact.chars().count() > fits { 9 } else { 11 };
        v.append(&one_line(
            &fact,
            size,
            400,
            &["tafs-mono", "tafs-face-words"],
        ));
    }
    if card.rules.is_empty() {
        v.append(&one_line(
            &w.line("card.no_actions_short").unwrap_or_default(),
            12,
            400,
            &["tafs-mono"],
        ));
    }
    let spacer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    spacer.set_vexpand(true);
    v.append(&spacer);
    let foot = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let by = gtk::Box::new(gtk::Orientation::Vertical, 1);
    by.append(&one_line(
        &w.line("card.issued_by").unwrap_or_default(),
        10,
        400,
        &["tafs-face-caption"],
    ));
    by.append(&one_line(issued_by, 12, 600, &[]));
    by.set_hexpand(true);
    foot.append(&by);
    let (caption, date) = card.ends(w, revoked).unwrap_or_default();
    let ends = gtk::Box::new(gtk::Orientation::Vertical, 1);
    let c = one_line(&caption, 10, 400, &["tafs-face-caption"]);
    c.set_xalign(1.0);
    ends.append(&c);
    let d = one_line(&date, 12, 600, &[]);
    d.set_xalign(1.0);
    ends.append(&d);
    foot.append(&ends);
    v.append(&foot);
    // The ring at its top corner, and each Card behind a shade darker.
    let o = gtk::Overlay::new();
    o.set_child(Some(&v));
    let ring = gtk::DrawingArea::new();
    ring.set_can_target(false);
    ring.set_draw_func(|_, cr, width, _| {
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.18);
        cr.set_line_width(1.0);
        cr.arc(
            width as f64 - 65.0 + 35.0,
            65.0 - 45.0,
            65.0,
            0.0,
            std::f64::consts::TAU,
        );
        let _ = cr.stroke();
    });
    o.add_overlay(&ring);
    let shade = tafs_core::cards::shade(depth);
    if shade > 0.0 {
        let dim = gtk::Box::new(gtk::Orientation::Vertical, 0);
        dim.set_can_target(false);
        dim.add_css_class("tafs-face-shade");
        dim.set_opacity(shade);
        o.add_overlay(&dim);
    }
    o.set_overflow(gtk::Overflow::Hidden);
    o.add_css_class("tafs-face-clip");
    o.set_size_request(FACE_WIDTH, FACE_HEIGHT);
    o.set_halign(gtk::Align::Center);
    let spoken = w
        .get("taf.show_card", &[("title", &card.title)])
        .unwrap_or_else(|_| card.title.clone());
    o.update_property(&[gtk::accessible::Property::Label(&spoken)]);
    o
}

/// `CardBar`: a small Card in the taf's colour with the Card's mark, its figure and words, its
/// name and how many, and a chevron.
fn card_bar_widget(lines: &tafs_core::taf_page::CardBarLines, color: i64) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    b.add_css_class("tafs-card-bar");
    let face = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    face.add_css_class("tafs-card-mini");
    face.add_css_class(&if lines.live {
        crate::style::avatar_class(color)
    } else {
        "dimfill".to_string()
    });
    if lines.dim {
        face.add_css_class("faded");
    }
    face.set_size_request(46, 29);
    face.set_valign(gtk::Align::Center);
    // Its own size, never stretched by the mark inside it.
    face.set_hexpand(false);
    let mark = symbol("tafs-creditcard-symbolic", 13);
    mark.set_size_request(46, 29);
    mark.set_halign(gtk::Align::Center);
    face.append(&mark);
    b.append(&face);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 1);
    v.set_hexpand(true);
    v.set_valign(gtk::Align::Center);
    let l = text(&lines.line, 15, 600, &["tafs-ink"]);
    l.set_lines(2);
    v.append(&l);
    let d = text(&lines.details, 12, 400, &["tafs-dim"]);
    d.set_lines(2);
    v.append(&d);
    b.append(&v);
    let chevron = symbol("go-next-symbolic", 13);
    chevron.add_css_class("tafs-dim");
    b.append(&chevron);
    b
}

/// `RecordRow` without its taf: the kind's dot, the title with its amount, the time, the line.
fn line(w: &WordBook, e: &Entry) -> gtk::Box {
    let r = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    r.add_css_class("tafs-record-row");
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.add_css_class("tafs-kind-dot");
    dot.add_css_class(match e.kind.tone() {
        Tone::Ok => "ok",
        Tone::Amber => "amber",
        Tone::No => "no",
        Tone::Dim => "dim",
    });
    dot.set_size_request(9, 9);
    dot.set_valign(gtk::Align::Start);
    dot.set_margin_top(6);
    r.append(&dot);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
    v.set_hexpand(true);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let title = e.title_with_amount(w).unwrap_or_else(|_| e.title.clone());
    let t = one_line(&title, 14, 600, &["tafs-ink"]);
    t.set_hexpand(true);
    head.append(&t);
    // `zonedShortTime`: with the account zone's name when it isn't this computer's.
    let time = gtk::Label::new(Some(&tafs_core::account_time::zoned_short_time(&e.at)));
    time.add_css_class("tafs-mono");
    time.add_css_class("tafs-dim");
    time.add_css_class("tafs-time");
    head.append(&time);
    v.append(&head);
    // The sample's held lines still wait.
    let detail = e.row_detail(w, true).unwrap_or_else(|_| e.detail.clone());
    v.append(&text(&detail, 13, 400, &["tafs-soft"]));
    r.append(&v);
    r.update_property(&[gtk::accessible::Property::Label(&format!(
        "{title}, {detail}"
    ))]);
    r
}

/// `TafActivityList`'s list: tafs cloud's filters, its day groups, each call's line, and its foot,
/// a page at a time.
/// `TafActivityList`'s `.cardUnits`: the Activity's last page's requests (`card_unit_exceptions`),
/// else Home's for the taf, each as Needs you draws it, then tafs cloud's notice.
struct Units {
    w: Rc<WordBook>,
    home: Vec<tafs_core::waits::CardUnitException>,
    discuss: bool,
    into: gtk::Box,
}

impl Units {
    fn draw(&self, page: Option<&[tafs_core::waits::CardUnitException]>, notice: Option<&str>) {
        while let Some(c) = self.into.first_child() {
            self.into.remove(&c);
        }
        for ask in page.unwrap_or(&self.home) {
            let lines = tafs_core::needs_you::card_unit_lines(ask, self.discuss);
            self.into
                .append(&crate::needs_you::card_unit_panel(&self.w, &lines));
        }
        if let Some(notice) = notice {
            self.into.append(&text(notice, 13, 400, &["tafs-dim"]));
        }
        self.into.set_visible(self.into.first_child().is_some());
    }
}

/// Narrows Activity to one app's calls: its `activity_query`, or empty for every app.
type Selects = Rc<dyn Fn(String)>;

/// The Activity list from tafs cloud, and how it's narrowed to one app's calls (an app's
/// `activity_query`; empty, every app).
fn cloud_activity(
    w: Rc<WordBook>,
    id: String,
    source: ActivitySource,
    units: Units,
    rows: Rc<RowOpens>,
) -> (gtk::Box, Rc<dyn Fn(String)>) {
    use tafs_core::activity::{foot, path, ActivityPage, Foot};
    struct State {
        filter: String,
        /// One app's calls (`selection`), as tafs cloud wrote the query; empty, every app.
        selection: String,
        /// Bumped by each new selection: a read for an older one is dropped.
        generation: u64,
        pages: Vec<ActivityPage>,
        error: Option<String>,
        loading: bool,
    }
    let root = gtk::Box::new(gtk::Orientation::Vertical, 14);
    let state = Rc::new(RefCell::new(State {
        filter: "all".into(),
        selection: String::new(),
        generation: 0,
        pages: vec![],
        error: None,
        loading: false,
    }));
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
    // Reads a page: the first again (a new filter, Try again), or the next.
    let load: Rc<dyn Fn(bool)> = {
        let (w, state, redraw, id, source) = (
            w.clone(),
            state.clone(),
            redraw.clone(),
            id.clone(),
            source.clone(),
        );
        Rc::new(move |more: bool| {
            let (filter, cursor, selection, generation) = {
                let mut s = state.borrow_mut();
                if s.loading {
                    return;
                }
                s.loading = true;
                s.error = None;
                (
                    s.filter.clone(),
                    if more {
                        s.pages.last().and_then(|p| p.next_cursor.clone())
                    } else {
                        None
                    },
                    s.selection.clone(),
                    s.generation,
                )
            };
            let p = path(&id, &filter, cursor.as_deref(), &selection);
            let source = source.clone();
            let words: WordBook = (*w).clone();
            let (state, redraw) = (state.clone(), redraw.clone());
            gtk::glib::spawn_future_local(async move {
                let read = gtk::gio::spawn_blocking(move || -> Result<ActivityPage, String> {
                    let body = match &source {
                        ActivitySource::Cloud { token, languages } => {
                            let a = crate::kiff::taf_activity(token, languages, &p)
                                .map_err(|e| e.words(&words).unwrap_or_default())?;
                            if a.status != 200 {
                                return Err(tafs_core::signin::refusal(&a, Some(&words))
                                    .words(&words)
                                    .unwrap_or_default());
                            }
                            a.body
                        }
                        ActivitySource::File(file) => {
                            let v: serde_json::Value = serde_json::from_slice(
                                &std::fs::read(file).map_err(|e| e.to_string())?,
                            )
                            .map_err(|e| e.to_string())?;
                            serde_json::to_vec(&v["answer"]).map_err(|e| e.to_string())?
                        }
                        ActivitySource::None => return Err(String::new()),
                    };
                    ActivityPage::read(&body).ok_or_else(|| {
                        tafs_core::signin::plain_words(&words, 200).unwrap_or_default()
                    })
                })
                .await
                .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    if s.generation != generation {
                        return;
                    }
                    s.loading = false;
                    match read {
                        Ok(page) => {
                            if !more {
                                s.pages.clear();
                            }
                            s.pages.push(page);
                        }
                        Err(e) => s.error = Some(e),
                    }
                }
                redraw();
            });
        })
    };
    *draw.borrow_mut() = Some(Rc::new({
        let (w, root, state, redraw, load) = (
            w.clone(),
            root.clone(),
            state.clone(),
            redraw.clone(),
            load.clone(),
        );
        move || {
            while let Some(c) = root.first_child() {
                root.remove(&c);
            }
            let s = state.borrow();
            // Narrowed to one app's calls: All apps shows every app's again.
            if !s.selection.is_empty() {
                let all =
                    gtk::Button::with_label(&w.line("taf.activity.all_apps").unwrap_or_default());
                all.add_css_class("tafs-link");
                all.set_halign(gtk::Align::Start);
                all.set_widget_name("taf.activity.all-apps");
                let (state, load, redraw) = (state.clone(), load.clone(), redraw.clone());
                all.connect_clicked(move |_| {
                    {
                        let mut s = state.borrow_mut();
                        s.selection.clear();
                        s.generation += 1;
                        s.pages.clear();
                        s.loading = false;
                    }
                    redraw();
                    load(false);
                });
                root.append(&all);
            }
            // tafs cloud's filters, from its first page.
            let filters = gtk::Box::new(gtk::Orientation::Horizontal, 6);
            for choice in s
                .pages
                .first()
                .map(|p| p.filters.clone())
                .unwrap_or_default()
            {
                let b = gtk::Button::with_label(&choice.title);
                b.add_css_class("tafs-filter-bordered");
                if choice.id == s.filter {
                    b.add_css_class("selected");
                }
                let (state, load, redraw) = (state.clone(), load.clone(), redraw.clone());
                b.connect_clicked(move |_| {
                    state.borrow_mut().filter = choice.id.clone();
                    redraw();
                    load(false);
                });
                filters.append(&b);
            }
            root.append(&filters);
            for page in &s.pages {
                for group in &page.groups {
                    if !group.title.is_empty() {
                        root.append(&text(&group.title, 12, 600, &["tafs-dim"]));
                    }
                    for e in &group.entries {
                        root.append(&activity_row(&w, e, &rows));
                    }
                }
            }
            let last = s.pages.last();
            units.draw(
                last.and_then(|p| p.card_unit_exceptions.as_deref()),
                last.and_then(|p| p.card_unit_exceptions_notice.as_deref()),
            );
            match foot(&s.pages, s.error.as_deref()) {
                Foot::Error(e) => {
                    root.append(&text(&e, 15, 400, &["tafs-no"]));
                    let b =
                        crate::widgets::outline(&w.line("common.try_again").unwrap_or_default());
                    let (load, more) = (load.clone(), !s.pages.is_empty());
                    b.connect_clicked(move |_| load(more));
                    root.append(&b);
                }
                Foot::More(_) => {
                    // The next page loads once the spinner comes into view.
                    let spinner = gtk::Spinner::new();
                    spinner.start();
                    let load = load.clone();
                    spinner.connect_map(move |_| load(true));
                    root.append(&spinner);
                }
                _ if s.loading => {
                    let spinner = gtk::Spinner::new();
                    spinner.start();
                    spinner.set_halign(gtk::Align::Start);
                    root.append(&spinner);
                }
                Foot::End(words) => root.append(&text(&words, 15, 400, &["tafs-dim"])),
                Foot::NothingHere => root.append(&text(
                    &w.line("record.nothing_here").unwrap_or_default(),
                    15,
                    400,
                    &["tafs-dim"],
                )),
                Foot::Nothing => {}
            }
        }
    }));
    redraw();
    load(false);
    let select: Rc<dyn Fn(String)> = Rc::new(move |selection: String| {
        {
            let mut s = state.borrow_mut();
            if s.selection == selection && !s.pages.is_empty() {
                return;
            }
            s.selection = selection;
            s.generation += 1;
            s.pages.clear();
            // A read under way is for the last selection: it's dropped, and this one read.
            s.loading = false;
        }
        redraw();
        load(false);
    });
    (root, select)
}

/// What a row on a tafs cloud taf's Activity opens: its receipt, and its next step's button.
pub struct RowOpens {
    /// The taf's key, the Record line's taf.
    pub taf: String,
    pub manages: bool,
    pub may_use_coach: bool,
    /// Coach on the taf (its words in the box), and Connect apps.
    pub coach: Rc<dyn Fn()>,
    pub connect: Rc<dyn Fn()>,
    /// Where its receipt is read, signed in with KIFF; none for a screenshot's.
    pub receipt: Option<Rc<crate::receipt_sheet::ReceiptSource>>,
}

/// `SourceActivityRow` on a taf's Activity: the row opens its receipt (`ReceiptView`), and its next
/// step is a button for whoever manages the taf (`NextStepRow`): Connect it again opens Connect
/// apps, Ask Coach opens Coach with its words in the box, nothing sent; anyone else, and a kind
/// the app doesn't know, reads its words.
fn activity_row(
    w: &Rc<WordBook>,
    e: &tafs_core::activity::ActivityEntry,
    opens: &Rc<RowOpens>,
) -> gtk::Box {
    use tafs_core::activity::{next_action, next_has_button, NextAction};
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let head = source_row(&tafs_core::activity::ActivityEntry {
        next: None,
        ..e.clone()
    });
    let b = gtk::Button::new();
    b.set_child(Some(&head));
    b.add_css_class("tafs-plain");
    b.update_property(&[gtk::accessible::Property::Description(
        &w.line("record.shows_receipt").unwrap_or_default(),
    )]);
    {
        let (w, e, opens) = (w.clone(), e.clone(), opens.clone());
        b.connect_clicked(move |b| {
            if let Some(entry) = tafs_core::activity::evidence(&w, &e, &opens.taf) {
                crate::receipt_sheet::present(
                    w.clone(),
                    opens.receipt.clone(),
                    entry,
                    None,
                    Rc::new(|_| {}),
                    b,
                );
            }
        });
    }
    v.append(&b);
    if let Some(next) = &e.next {
        let action = next_action(next, &format!("{}: {}", e.title, e.detail));
        if next_has_button(&action, opens.manages, opens.may_use_coach) {
            let button = crate::widgets::outline(&next.words);
            let opens = opens.clone();
            button.connect_clicked(move |_| match &action {
                NextAction::AskCoach(words) => {
                    crate::coach::set_draft(words);
                    (opens.coach)();
                }
                NextAction::ConnectApps => (opens.connect)(),
                NextAction::Line => {}
            });
            v.append(&button);
        } else {
            v.append(&text(&next.words, 13, 400, &["tafs-soft"]));
        }
    }
    v
}

/// `SourceActivityRow`: its title and detail, its time and amount at its end, and its next step.
pub fn source_row(e: &tafs_core::activity::ActivityEntry) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    row.set_margin_top(5);
    row.set_margin_bottom(5);
    let left = gtk::Box::new(gtk::Orientation::Vertical, 4);
    left.set_hexpand(true);
    left.append(&text(&e.title, 15, 600, &["tafs-ink"]));
    left.append(&text(&e.detail, 13, 400, &["tafs-soft"]));
    row.append(&left);
    let right = gtk::Box::new(gtk::Orientation::Vertical, 2);
    right.set_valign(gtk::Align::Start);
    let time = text(&e.time, 12, 400, &["tafs-dim"]);
    time.set_wrap(false);
    time.set_xalign(1.0);
    right.append(&time);
    if let Some(amount) = &e.amount_text {
        let a = text(amount, 13, 600, &["tafs-ink"]);
        a.set_wrap(false);
        a.set_xalign(1.0);
        right.append(&a);
    }
    row.append(&right);
    v.append(&row);
    // Its next step, in tafs cloud's words: a line until its buttons come with Coach and Connect apps.
    if let Some(next) = &e.next {
        v.append(&text(&next.words, 13, 400, &["tafs-soft"]));
    }
    v.update_property(&[gtk::accessible::Property::Label(&format!(
        "{}, {}, {}",
        e.title, e.detail, e.time
    ))]);
    v
}
