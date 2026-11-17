//! The window once signed in, or in the sample: the iPad's one sidebar for the whole app
//! (`HomeSplitView` with `tab` set, an iPad's full width) and the chosen screen beside it.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::account::AccountLines;
use tafs_core::needs_you::NeedsYouLines;
use tafs_core::overview::OverviewLines;
use tafs_core::sidebar::{Selection, SidebarLines, SidebarModel};
use tafs_core::words::WordBook;

use crate::widgets::{avatar, eyebrow, icon_for, mascot, one_line, outline, symbol, text};

/// What opens a screen beside the sidebar.
type Opener = Rc<dyn Fn(Selection)>;

/// The sidebar's width: the iPad's at full width.
pub const SIDEBAR_WIDTH: i32 = 320;

pub struct Shell {
    pub root: gtk::Box,
}

/// The screens ported so far, as they read now, and what Account's actions do.
pub struct Screens {
    pub overview: OverviewLines,
    pub needs_you: NeedsYouLines,
    pub account: AccountLines,
    /// What Account reads when it opens (Coach's suggestions, the AI-data consent), signed in.
    pub account_reads: Option<Rc<crate::ai_consent::Reads>>,
    pub record: Rc<crate::record::RecordData>,
    /// Each taf's page, by its key.
    pub tafs: Rc<Vec<crate::taf::TafPage>>,
    pub actions: Rc<dyn Fn(tafs_core::account::Action)>,
    /// Where Plan & usage reads, and tafs cloud's tafs by id for their runs' title.
    pub usage: crate::usage::UsageSource,
    pub cloud_names: crate::usage::Names,
    /// Where Connect apps reads.
    pub connect: crate::connect::ConnectSource,
    /// Where Account › Devices reads.
    pub devices: crate::devices::DevicesSource,
    /// Account › Alerts: signed in with KIFF, whether this person may answer held calls.
    pub alerts: crate::alerts::AlertsSource,
    /// Where People & groups reads, and what it knows besides.
    pub people: crate::people::PeopleSource,
    pub people_known: Rc<crate::people::Known>,
    /// Where the time zones are read.
    pub zones: crate::zones::ZonesSource,
    /// Where a halt is read and sent, signed in with KIFF: the halt screens and the taf page's row.
    pub halt: Option<Rc<crate::halt::HaltSource>>,
    /// Where Export my chats reads, signed in with KIFF from a kept session.
    pub export: Option<Rc<crate::kiff_account::ExportSource>>,
    /// The jobs' gallery, signed in with KIFF: "Explore jobs" and, with tafs, the strip.
    pub gallery: Option<Rc<crate::recipes::GalleryPane>>,
    /// Home with no taf, when tafs cloud lists its recipes (`homeListsRecipes`): they take the
    /// Overview's place.
    pub recipes: Option<Rc<crate::recipes::RecipesPane>>,
    /// Where the calendar reads, and the state a screenshot opens it in.
    pub calendar: crate::calendar::CalendarSource,
    pub calendar_first: tafs_core::calendar::State,
    /// Where Coach's conversation is read, and the account's tafs a result can open.
    pub coach: crate::coach::CoachSource,
    pub coach_listed: Rc<Vec<tafs_core::coach_lines::Listed>>,
    /// "Your brief", in Needs you's place once tafs cloud sends one.
    pub brief: Option<Rc<crate::brief::BriefPane>>,
}

/// Builds the shell. `view_only` shows the notice that this computer can't approve yet.
pub fn build(
    w: Rc<WordBook>,
    model: &SidebarModel,
    screens: Screens,
    view_only: Option<ViewOnly>,
    first: Selection,
) -> Shell {
    let Screens {
        overview,
        needs_you,
        account,
        account_reads,
        record,
        tafs,
        actions,
        usage,
        cloud_names,
        connect,
        devices,
        alerts,
        calendar,
        calendar_first,
        people,
        people_known,
        zones,
        recipes,
        gallery,
        halt,
        export,
        coach,
        coach_listed,
        brief,
    } = screens;
    let entries = Rc::new(record.entries.clone());
    let lines = model.lines(&w).unwrap_or_else(|e| panic!("{e}"));
    let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    crate::widgets::ground(&root);

    let detail = gtk::Box::new(gtk::Orientation::Vertical, 0);
    detail.set_hexpand(true);
    crate::widgets::ground(&detail);
    if let Some(why) = view_only {
        detail.append(&view_only_notice(&w, why));
    }
    let stack = gtk::Stack::new();
    stack.set_vexpand(true);
    stack.set_hexpand(true);
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    stack.set_transition_duration(250);
    detail.append(&stack);

    let selection: Rc<RefCell<Option<Selection>>> = Rc::new(RefCell::new(None));
    let rows: Rc<RefCell<Vec<(gtk::ListBoxRow, Selection)>>> = Rc::new(RefCell::new(Vec::new()));
    let has_tafs = !model.tafs.is_empty();
    let (sidebar, list, account_icon, account_button, coach_button, plan_button) =
        sidebar(&w, &lines, &rows, gallery.as_ref(), has_tafs);
    root.append(&sidebar);
    root.append(&detail);

    // What a screen opens goes through the sidebar's own choice, once it exists.
    let holder: Rc<RefCell<Option<Opener>>> = Rc::new(RefCell::new(None));
    let open: Rc<dyn Fn(Selection)> = {
        let holder = holder.clone();
        Rc::new(move |s| {
            let f = holder.borrow().clone();
            if let Some(f) = f {
                f(s);
            }
        })
    };
    let overview = Rc::new(overview);
    let needs_you = Rc::new(needs_you);
    let account = Rc::new(account);
    let select: Rc<dyn Fn(Selection)> = {
        let (open, overview, needs_you, account, actions, record) = (
            open.clone(),
            overview.clone(),
            needs_you.clone(),
            account.clone(),
            actions.clone(),
            record.clone(),
        );
        let (tafs, entries) = (tafs.clone(), entries.clone());
        let (w, stack, selection, rows, list, account_icon, lines) = (
            w.clone(),
            stack.clone(),
            selection.clone(),
            rows.clone(),
            list.clone(),
            account_icon.clone(),
            lines.clone(),
        );
        let model_tafs: Vec<(String, String)> = model
            .tafs
            .iter()
            .map(|(t, _)| (t.key.clone(), t.name.clone()))
            .collect();
        Rc::new(move |s: Selection| {
            if selection.borrow().as_ref() == Some(&s) {
                return;
            }
            *selection.borrow_mut() = Some(s.clone());
            // The list shows what's chosen: a row of it, or none for Account at the foot.
            match rows.borrow().iter().find(|(_, sel)| *sel == s) {
                Some((row, _)) => list.select_row(Some(row)),
                None => list.unselect_all(),
            }
            // Account, and the screens it opens.
            let account_open = matches!(
                s,
                Selection::Account
                    | Selection::Devices
                    | Selection::People
                    | Selection::KiffAccount
                    | Selection::TimeZone
                    | Selection::Alerts
                    | Selection::Language
            );
            account_icon.set_icon_name(Some(icon_for(if account_open {
                "person.crop.circle.fill"
            } else {
                "person.crop.circle"
            })));
            if account_open {
                account_icon.add_css_class("open");
            } else {
                account_icon.remove_css_class("open");
            }
            let title = title_of(&w, &lines, &model_tafs, &s);
            let page = match s {
                Selection::Overview => match &recipes {
                    Some(pane) => crate::recipes::page(pane.clone()),
                    None => crate::overview::page(&overview, open.clone()),
                },
                // "Your brief" in its place once there is one; Needs you, as today, otherwise.
                Selection::NeedsYou => {
                    let list: Rc<dyn Fn() -> gtk::Widget> = {
                        let (w, needs_you, open) = (w.clone(), needs_you.clone(), open.clone());
                        Rc::new(move || crate::needs_you::page(w.clone(), &needs_you, open.clone()))
                    };
                    match &brief {
                        Some(pane) => {
                            crate::brief::page(w.clone(), pane.clone(), open.clone(), list)
                        }
                        None => list(),
                    }
                }
                Selection::Record => {
                    let open = open.clone();
                    crate::record::page(
                        w.clone(),
                        record.clone(),
                        Rc::new(move |key: String| open(Selection::Taf(key))),
                    )
                }
                Selection::Calendar => {
                    // A taf's work opens its page, by tafs cloud's id.
                    let known: Vec<String> =
                        tafs.iter().filter_map(|t| t.cloud_id.clone()).collect();
                    let by_cloud: Vec<(String, String)> = tafs
                        .iter()
                        .filter_map(|t| Some((t.cloud_id.clone()?, t.key.clone())))
                        .collect();
                    let open = open.clone();
                    crate::calendar::page(
                        w.clone(),
                        calendar.clone(),
                        Rc::new(crate::calendar::Opening {
                            known,
                            open: Rc::new(move |cloud: String| {
                                if let Some((_, key)) = by_cloud.iter().find(|(c, _)| *c == cloud) {
                                    open(Selection::Taf(key.clone()));
                                }
                            }),
                        }),
                        calendar_first.clone(),
                    )
                }
                Selection::KiffAccount => {
                    let open = open.clone();
                    let to_zone = open.clone();
                    crate::kiff_account::page(
                        w.clone(),
                        people_known.account.clone(),
                        Rc::new(move || open(Selection::Account)),
                        Rc::new(move || to_zone(Selection::TimeZone)),
                        export.clone(),
                    )
                }
                Selection::TimeZone => {
                    let open = open.clone();
                    crate::zones::page(
                        w.clone(),
                        zones.clone(),
                        people_known
                            .account
                            .as_ref()
                            .map(|a| a.time_zone.clone().unwrap_or_else(|| "UTC".into())),
                        Rc::new(move || open(Selection::Account)),
                    )
                }
                Selection::Language => {
                    let open = open.clone();
                    crate::language_picker::page(
                        w.clone(),
                        crate::store::language().map(str::to_string),
                        Rc::new(move || open(Selection::Account)),
                        actions.clone(),
                    )
                }
                Selection::People => {
                    let open = open.clone();
                    crate::people::page(
                        w.clone(),
                        people.clone(),
                        people_known.clone(),
                        Rc::new(move || open(Selection::Account)),
                    )
                }
                Selection::Alerts => {
                    let open = open.clone();
                    crate::alerts::page(
                        w.clone(),
                        alerts.clone(),
                        Rc::new(move || open(Selection::Account)),
                    )
                }
                Selection::Devices => {
                    let open = open.clone();
                    crate::devices::page(
                        w.clone(),
                        devices.clone(),
                        Rc::new(move || open(Selection::Account)),
                    )
                }
                Selection::ConnectApps => {
                    let open = open.clone();
                    crate::connect::page(
                        w.clone(),
                        connect.clone(),
                        Rc::new(move || open(Selection::Coach)),
                    )
                }
                Selection::Usage => {
                    crate::usage::page(w.clone(), usage.clone(), cloud_names.clone())
                }
                // Coach, read only, signed in with KIFF; the sample has none, so its title alone.
                Selection::Coach if !matches!(coach, crate::coach::CoachSource::None) => {
                    crate::coach::page(w.clone(), coach.clone(), coach_listed.clone(), open.clone())
                }
                // A halt: every taf's, or one taf's by tafs cloud's id.
                Selection::HaltAll | Selection::Halt(_) => match &halt {
                    Some(source) => {
                        let id = match &s {
                            Selection::Halt(id) => Some(id.clone()),
                            _ => None,
                        };
                        let name = id.as_ref().and_then(|id| {
                            tafs.iter()
                                .find(|t| t.cloud_id.as_deref() == Some(id.as_str()))
                                .map(|t| t.name.clone())
                        });
                        crate::halt::page(
                            w.clone(),
                            source.clone(),
                            crate::halt::Whose { id, name },
                        )
                    }
                    None => screen_page(&title).upcast::<gtk::Widget>(),
                },
                // A taf, or a held call on its taf's page.
                Selection::Taf(key) | Selection::Held { taf: key, .. } => {
                    match tafs.iter().find(|t| t.key == *key) {
                        Some(t) => {
                            // Halting is a job of its own too: its row opens its screen, for a
                            // tafs cloud taf this person manages that isn't revoked.
                            let halt_row = match (&halt, &t.cloud_id) {
                                (Some(source), Some(id)) if t.manages && !t.revoked => {
                                    Some(crate::halt::row(
                                        w.clone(),
                                        source.clone(),
                                        crate::halt::Whose {
                                            id: Some(id.clone()),
                                            name: Some(t.name.clone()),
                                        },
                                        open.clone(),
                                    ))
                                }
                                _ => None,
                            };
                            let (o1, o2, o3) = (open.clone(), open.clone(), open.clone());
                            crate::taf::page(
                                w.clone(),
                                t,
                                entries.clone(),
                                halt_row,
                                Rc::new(move || o1(Selection::Usage)),
                                Rc::new(move || o2(Selection::Coach)),
                                Rc::new(move || o3(Selection::ConnectApps)),
                            )
                        }
                        None => screen_page(&title).upcast::<gtk::Widget>(),
                    }
                }
                Selection::Account => {
                    let open = open.clone();
                    crate::account::page(
                        &account,
                        &crate::account::Actions {
                            export: export.clone(),
                            reads: account_reads.clone(),
                            words: w.clone(),
                            act: actions.clone(),
                            open: Rc::new(move |screen, _title| {
                                open(match screen {
                                    tafs_core::account::Screen::Devices => Selection::Devices,
                                    tafs_core::account::Screen::People => Selection::People,
                                    tafs_core::account::Screen::KiffAccount => {
                                        Selection::KiffAccount
                                    }
                                    tafs_core::account::Screen::TimeZone => Selection::TimeZone,
                                    tafs_core::account::Screen::Language => Selection::Language,
                                    tafs_core::account::Screen::ConnectApps => {
                                        Selection::ConnectApps
                                    }
                                    tafs_core::account::Screen::Usage => Selection::Usage,
                                    tafs_core::account::Screen::Alerts => Selection::Alerts,
                                })
                            }),
                        },
                    )
                }
                _ => screen_page(&title).upcast::<gtk::Widget>(),
            };
            stack.add_child(&page);
            stack.set_visible_child(&page);
            // Each pick starts its own screen: nothing kept from the last (`.id(selection)`).
            let old: Vec<gtk::Widget> = {
                let mut v = Vec::new();
                let mut c = stack.first_child();
                while let Some(child) = c {
                    c = child.next_sibling();
                    if child != page {
                        v.push(child);
                    }
                }
                v
            };
            let stack2 = stack.clone();
            gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(300), move || {
                for child in old {
                    if child.parent().is_some() && stack2.visible_child().as_ref() != Some(&child) {
                        stack2.remove(&child);
                    }
                }
            });
        })
    };

    {
        let (select, rows) = (select.clone(), rows.clone());
        list.connect_row_activated(move |_, row| {
            if let Some((_, s)) = rows.borrow().iter().find(|(r, _)| r == row) {
                select(s.clone());
            }
        });
    }
    {
        let select = select.clone();
        account_button.connect_clicked(move |_| select(Selection::Account));
    }
    // Coach opens its conversation, read only; in the sample, its title.
    if let Some(coach) = coach_button {
        let select = select.clone();
        coach.connect_clicked(move |_| select(Selection::Coach));
    }
    if let Some(plan) = plan_button {
        let select = select.clone();
        plan.connect_clicked(move |_| select(Selection::Usage));
    }
    *holder.borrow_mut() = Some(select.clone());
    select(first);
    Shell { root }
}

/// The sidebar: its list (the brand, the screens, NEEDS YOU, YOUR TAFS) and its foot.
fn sidebar(
    w: &WordBook,
    lines: &SidebarLines,
    rows: &Rc<RefCell<Vec<(gtk::ListBoxRow, Selection)>>>,
    gallery: Option<&Rc<crate::recipes::GalleryPane>>,
    has_tafs: bool,
) -> (
    gtk::Box,
    gtk::ListBox,
    gtk::Image,
    gtk::Button,
    Option<gtk::Button>,
    Option<gtk::Button>,
) {
    let pane = gtk::Box::new(gtk::Orientation::Vertical, 0);
    pane.add_css_class("tafs-sidebar");
    pane.set_size_request(SIDEBAR_WIDTH, -1);
    pane.set_hexpand(false);

    let list = gtk::ListBox::new();
    list.set_selection_mode(gtk::SelectionMode::Single);
    list.set_activate_on_single_click(true);
    list.set_margin_top(24);
    list.set_margin_bottom(12);
    let add = |row: gtk::ListBoxRow, s: Option<Selection>| {
        if let Some(s) = s {
            rows.borrow_mut().push((row.clone(), s));
        } else {
            row.set_selectable(false);
            row.set_activatable(false);
            row.add_css_class("header");
        }
        list.append(&row);
    };

    // The mascot and "tafs": the way to the Overview, and what opens first.
    let brand = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    brand.append(&mascot(34));
    let name = gtk::Label::new(Some(&lines.brand));
    name.add_css_class("brand");
    brand.append(&name);
    brand.set_margin_top(4);
    brand.set_margin_bottom(4);
    let row = sidebar_row(&brand, 52);
    // The mark is the list's header: no fill when the Overview is open (`listRowBackground(.clear)`).
    row.add_css_class("brand-row");
    row.update_property(&[gtk::accessible::Property::Label(&lines.brand)]);
    add(row, Some(Selection::Overview));
    add(gap(16), None);

    // The app's screens, each an item, with Needs you's count as a plain grey number.
    for item in &lines.screens {
        let b = gtk::Box::new(gtk::Orientation::Horizontal, 14);
        let icon = symbol(icon_for(item.symbol), 20);
        // A halt's hand is red; every other screen's icon is ink.
        icon.add_css_class(if item.symbol.starts_with("hand.") {
            "tafs-no"
        } else {
            "tafs-ink"
        });
        b.append(&icon);
        let title = gtk::Label::new(Some(&item.title));
        title.add_css_class("row-title");
        title.set_xalign(0.0);
        // Halt all's state, in red under its title, while it isn't running.
        match &item.line {
            Some(line) => {
                let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
                v.set_hexpand(true);
                v.set_valign(gtk::Align::Center);
                v.append(&title);
                v.append(&text(line, 13, 400, &["tafs-no"]));
                b.append(&v);
            }
            None => {
                title.set_hexpand(true);
                b.append(&title);
            }
        }
        if let Some(badge) = &item.badge {
            let count = gtk::Label::new(Some(badge));
            count.add_css_class("row-badge");
            b.append(&count);
        }
        let row = sidebar_row(&b, 52);
        row.update_property(&[gtk::accessible::Property::Label(&item.accessible)]);
        add(row, Some(item.selection.clone()));
    }

    // NEEDS YOU: each held call, its taf and summary, why and when it lapses.
    if let Some(label) = &lines.held_label {
        add(section(label, true), None);
        for held in &lines.held {
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            dot.add_css_class("tafs-dot");
            dot.set_size_request(8, 8);
            dot.set_valign(gtk::Align::Center);
            b.append(&dot);
            let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
            v.set_hexpand(true);
            v.set_valign(gtk::Align::Center);
            v.append(&one_line(&held.title, 14, 600, &["held-title"]));
            v.append(&one_line(&held.line, 12, 400, &["held-line"]));
            b.append(&v);
            let row = sidebar_row(&b, 56);
            row.update_property(&[gtk::accessible::Property::Label(&format!(
                "{}, {}",
                held.title, held.line
            ))]);
            add(row, Some(held.selection.clone()));
        }
    }

    // A taf's row (`tafLinks`, `tafRow`): its avatar, name and what it's doing, opening its page.
    let taf_row = |taf: &tafs_core::sidebar::TafRow| {
        let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        b.append(&avatar(&taf.initial, taf.color_index, 34));
        let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
        v.set_hexpand(true);
        v.set_valign(gtk::Align::Center);
        v.append(&one_line(&taf.name, 15, 600, &["row-title"]));
        v.append(&one_line(
            &taf.now,
            13,
            400,
            &["taf-now", tone_class(taf.tone)],
        ));
        b.append(&v);
        b.set_margin_top(2);
        b.set_margin_bottom(2);
        let row = sidebar_row(&b, 60);
        row.update_property(&[gtk::accessible::Property::Label(&format!(
            "{}, {}",
            taf.name, taf.now
        ))]);
        row
    };

    // YOUR TAFS: those relevant now; all of them are on the Overview.
    if let Some(label) = &lines.your_tafs_label {
        add(section(label, false), None);
        for taf in &lines.your_tafs {
            add(taf_row(taf), Some(Selection::Taf(taf.key.clone())));
        }
        // A taf being set up, among the tafs, by its name, with no state. Its line is Finish setting
        // up's words, from tafs cloud; finishing it opens Coach, which isn't on this computer, so
        // the row opens nothing. Discard, for the person whose setup it was, is signed, so it's
        // there, dimmed.
        for (taf, discard) in &lines.setting_up {
            let b = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            b.append(&avatar(&taf.initial, taf.color_index, 34));
            let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
            v.set_hexpand(true);
            v.set_valign(gtk::Align::Center);
            v.append(&one_line(&taf.name, 15, 600, &["row-title"]));
            if !taf.now.is_empty() {
                v.append(&one_line(
                    &taf.now,
                    13,
                    400,
                    &["taf-now", tone_class(taf.tone)],
                ));
            }
            b.append(&v);
            b.set_margin_top(2);
            b.set_margin_bottom(2);
            let row = sidebar_row(&b, 60);
            row.set_widget_name("home.setting-up");
            row.update_property(&[gtk::accessible::Property::Label(&if taf.now.is_empty() {
                taf.name.clone()
            } else {
                format!("{}, {}", taf.name, taf.now)
            })]);
            add(row, None);
            if let Some(words) = discard {
                let d = gtk::Button::with_label(words);
                d.add_css_class("tafs-outline");
                d.add_css_class("tafs-soft");
                d.set_halign(gtk::Align::End);
                d.set_hexpand(false);
                d.set_sensitive(false);
                d.set_widget_name("home.discard-setup");
                add(sidebar_row(&d, 0), None);
            }
        }
        if let Some(none) = &lines.no_tafs {
            let l = text(none, 13, 400, &["tafs-dim"]);
            let row = sidebar_row(&l, 0);
            add(row, None);
        }
    }
    // SHARED WITH YOU: the tafs shared with this person to use, relevant now.
    if let Some(label) = &lines.shared_label {
        add(section(label, false), None);
        for taf in &lines.shared {
            add(taf_row(taf), Some(Selection::Taf(taf.key.clone())));
        }
    }

    // The jobs, to browse with nothing created: with or without tafs, once tafs cloud sends the
    // row's words (`ExploreRow`).
    if let Some(pane) = gallery {
        if let Some(words) = pane.gallery.explore() {
            let b = gtk::Button::new();
            let h = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            h.append(&symbol("view-grid-symbolic", 15));
            h.append(&one_line(words, 14, 600, &["tafs-ink"]));
            b.set_child(Some(&h));
            b.add_css_class("tafs-plain");
            b.set_widget_name("home.explore");
            let pane = pane.clone();
            b.connect_clicked(move |b| crate::recipes::gallery(&pane, b));
            add(sidebar_row(&b, 44), None);
        }
        // A few jobs the account doesn't run yet, where tafs cloud shows them (`RecipeStrip`):
        // each opens its sheet; "See all jobs" opens them all.
        let strip = pane.gallery.strip_recipes();
        if has_tafs && !strip.is_empty() {
            if !pane.gallery.strip.title.trim().is_empty() {
                add(section(&pane.gallery.strip.title, false), None);
            }
            for recipe in strip {
                let h = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
                bar.set_size_request(4, 28);
                bar.set_valign(gtk::Align::Center);
                bar.add_css_class("tafs-strip-bar");
                if let Some(color) = crate::recipes::strip_tint(pane, recipe) {
                    bar.set_widget_name(&crate::style::tint_name("strip"));
                    crate::style::tint_bar(&bar, &color);
                }
                h.append(&bar);
                let lead = text(recipe.lead(), 14, 400, &["tafs-ink"]);
                lead.set_lines(2);
                lead.set_ellipsize(gtk::pango::EllipsizeMode::End);
                h.append(&lead);
                let b = gtk::Button::new();
                b.set_child(Some(&h));
                b.add_css_class("tafs-plain");
                let (pane, recipe) = (pane.clone(), recipe.clone());
                b.connect_clicked(move |b| crate::recipes::strip_sheet(&pane, &recipe, b));
                add(sidebar_row(&b, 40), None);
            }
            if let Some(words) = pane.gallery.see_all() {
                let b = gtk::Button::new();
                b.set_child(Some(&one_line(words, 14, 600, &["tafs-ink"])));
                b.add_css_class("tafs-plain");
                b.set_widget_name("home.recipe-strip.all");
                let pane = pane.clone();
                b.connect_clicked(move |b| crate::recipes::gallery(&pane, b));
                add(sidebar_row(&b, 44), None);
            }
        }
    }

    // OTHER AGENTS WITH CARDS: agents made through KIFF's API, not run by tafs, apart.
    if let Some(label) = &lines.others_label {
        add(section(label, false), None);
        for taf in &lines.others {
            add(taf_row(taf), Some(Selection::Taf(taf.key.clone())));
        }
    }

    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&list));
    pane.append(&scroller);

    // Coach, then the account just under it: always in sight.
    let foot = gtk::Box::new(gtk::Orientation::Vertical, 8);
    foot.set_margin_start(16);
    foot.set_margin_end(16);
    foot.set_margin_top(12);
    foot.set_margin_bottom(12);
    // The plan, only when it needs the owner (`PlanLine`): tafs cloud's words, then Plan & usage,
    // which it opens.
    let mut plan_button = None;
    if let Some(plan) = &lines.plan {
        let class = match plan.tone {
            tafs_core::sidebar::Tone::No => "tafs-no",
            _ => "tafs-amber-text",
        };
        let b = gtk::Button::new();
        b.add_css_class("tafs-plain");
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let dot = gtk::Label::new(Some("●"));
        dot.add_css_class(class);
        dot.add_css_class("tafs-plan-dot");
        dot.set_valign(gtk::Align::Start);
        h.append(&dot);
        let words = text(&plan.words, 13, 400, &[class]);
        words.set_hexpand(true);
        h.append(&words);
        let title = one_line(&plan.title, 12, 600, &["tafs-amber-text"]);
        title.set_valign(gtk::Align::Start);
        h.append(&title);
        b.set_child(Some(&h));
        b.set_widget_name("plan-line");
        b.update_property(&[gtk::accessible::Property::Label(&plan.accessible)]);
        foot.append(&b);
        plan_button = Some(b);
    }
    let mut coach_button = None;
    if let Some(coach) = &lines.coach {
        let b = gtk::Button::new();
        b.add_css_class("tafs-coach");
        let inner = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        inner.set_halign(gtk::Align::Center);
        inner.append(&symbol(icon_for("sparkle"), 17));
        inner.append(&one_line(coach, 15, 600, &[]));
        b.set_child(Some(&inner));
        b.update_property(&[gtk::accessible::Property::Label(
            &w.line("coach.coach_mode").unwrap_or_default(),
        )]);
        foot.append(&b);
        coach_button = Some(b);
    }
    let account = gtk::Button::new();
    account.add_css_class("tafs-account");
    let inner = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    let icon = symbol(icon_for("person.crop.circle"), 20);
    inner.append(&icon);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 1);
    v.set_valign(gtk::Align::Center);
    v.append(&one_line(&lines.account, 14, 600, &["account-title"]));
    if let Some(label) = &lines.account_label {
        v.append(&one_line(label, 12, 400, &["account-label"]));
    }
    inner.append(&v);
    account.set_child(Some(&inner));
    account.update_property(&[gtk::accessible::Property::Label(&lines.account_accessible)]);
    foot.append(&account);
    pane.append(&foot);
    (pane, list, icon, account, coach_button, plan_button)
}

/// A row's line colour: dim, or what went wrong in the amber, or red when it's failing.
fn tone_class(tone: tafs_core::sidebar::Tone) -> &'static str {
    match tone {
        tafs_core::sidebar::Tone::Dim => "dim",
        tafs_core::sidebar::Tone::Amber => "amber",
        tafs_core::sidebar::Tone::No => "no",
    }
}

fn sidebar_row(child: &impl IsA<gtk::Widget>, min_height: i32) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    row.set_child(Some(child));
    if min_height > 0 {
        row.set_size_request(-1, min_height);
    }
    row
}

/// A section's label, as the iPad's list header: the eyebrow, with room above it.
fn section(label: &str, amber: bool) -> gtk::ListBoxRow {
    let e = eyebrow(label, amber);
    e.set_margin_top(20);
    e.set_margin_bottom(8);
    let row = sidebar_row(&e, 0);
    row.set_selectable(false);
    row
}

fn gap(h: i32) -> gtk::ListBoxRow {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
    b.set_size_request(-1, h);
    sidebar_row(&b, 0)
}

/// The words of what's open beside the sidebar.
fn title_of(
    w: &WordBook,
    lines: &SidebarLines,
    tafs: &[(String, String)],
    s: &Selection,
) -> String {
    let l = |k: &str| w.line(k).unwrap_or_default();
    match s {
        Selection::Overview => l("home.overview.title"),
        Selection::Account => lines.account.clone(),
        Selection::Coach => l("coach.coach_mode"),
        Selection::Devices => l("settings.devices.title"),
        Selection::People => l("people.title"),
        Selection::KiffAccount => l("settings.kiff_account"),
        Selection::TimeZone => l("settings.time_zone_title"),
        Selection::Language => l("settings.language_title"),
        Selection::Alerts => l("settings.alerts"),
        Selection::Titled(title) => title.clone(),
        Selection::Halt(_) => l("taf.halt.title"),
        Selection::Taf(key) | Selection::Held { taf: key, .. } => tafs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, n)| n.clone())
            .unwrap_or_default(),
        other => lines
            .screens
            .iter()
            .find(|i| &i.selection == other)
            .map(|i| i.title.clone())
            .unwrap_or_default(),
    }
}

/// A screen not ported yet: its title alone, where the iPad's large title is.
fn screen_page(title: &str) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 0);
    b.set_margin_start(24);
    b.set_margin_end(24);
    b.set_margin_top(32);
    let t = text(title, 34, 700, &["tafs-ink"]);
    t.set_width_chars(16);
    b.append(&t);
    crate::widgets::ground(&b);
    b
}

/// Why this computer is view-only, for the notice beside the sidebar.
pub enum ViewOnly {
    /// Signed in in the browser: it can be added with the phone. The function opens the pairing.
    CanPair(Rc<dyn Fn()>),
    /// Added with the phone already: its key is in software, so it still approves nothing, and
    /// approvals go to the phone. No button: pairing again can't lift it (as tafs-macos
    /// #90 does on macOS 12).
    Paired,
}

/// This computer is view-only (the pairing words): before it's paired, "It can't approve until you
/// add it with your phone." and Add this computer; once paired, "This computer can't sign
/// approvals itself. Approve on your phone, or with a passkey." and nothing to press.
fn view_only_notice(w: &WordBook, why: ViewOnly) -> gtk::Box {
    let device = w.line("device.linux").unwrap_or_default();
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 16);
    b.add_css_class("tafs-notice");
    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
    v.set_hexpand(true);
    v.append(&text(
        &w.line("pair.view_only.title").unwrap_or_default(),
        15,
        600,
        &["tafs-ink"],
    ));
    let lead_key = match why {
        ViewOnly::CanPair(_) => "pair.view_only.lead",
        ViewOnly::Paired => "pair.added_passkey",
    };
    let lead = text(
        &w.get(lead_key, &[("device", &device)]).unwrap_or_default(),
        13,
        400,
        &["tafs-soft"],
    );
    // A width it never wraps below: the window's least size is measured at its least width.
    lead.set_width_chars(40);
    v.append(&lead);
    b.append(&v);
    let ViewOnly::CanPair(add_this) = why else {
        return b;
    };
    let add = outline(
        &w.get("pair.view_only.add", &[("device", &device)])
            .unwrap_or_default(),
    );
    add.set_hexpand(false);
    add.set_valign(gtk::Align::Center);
    add.connect_clicked(move |_| add_this());
    b.append(&add);
    b
}
