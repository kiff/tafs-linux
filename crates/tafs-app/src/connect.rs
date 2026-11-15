//! Connect apps beside the sidebar (`ConnectAppsView`): what tafs cloud offers to connect, the
//! person's own first (Yours) with a ✓, then the rest (Discover), searched as the person types,
//! as the iPad's table on a wide window: App, How it connects, Whose, Status (`ConnectColumns`).
//! The lines are `tafs-core/src/connect.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::connect::{path, says_make_a_taf, split, App, ConnectApps, Row};
use tafs_core::words::WordBook;

use crate::widgets::{one_line, outline, symbol, text};

/// Where Connect apps is read from.
#[derive(Clone)]
pub enum ConnectSource {
    /// The sample: the reference set's apps in a screenshot (`SampleScreens.connectApps`),
    /// else nothing, as the iPad's sample.
    Sample { screenshot: bool },
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// Signed out: nothing read.
    None,
}

#[derive(Default)]
struct State {
    apps: Option<ConnectApps>,
    error: Option<String>,
    query: String,
    loading_more: bool,
    /// Each read's number: only the newest search may replace the page.
    request: u64,
}

type Redraw = Rc<dyn Fn()>;

/// The reference set's apps, from the iPad's own sample at the pin.
fn sample_apps() -> Option<ConnectApps> {
    let file = WordBook::default_root().join("../../ios/Tafs/Samples/connect-apps.json");
    ConnectApps::read(&std::fs::read(file).ok()?)
}

/// Reads a page from tafs cloud: why not, in the iPad's words, when it can't.
fn read_page(
    w: &WordBook,
    token: &str,
    languages: &[&str],
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
    let answer =
        crate::kiff::cloud(token, languages, p).map_err(|e| why(e.words(w).unwrap_or_default()))?;
    if !(200..300).contains(&answer.status) {
        return Err(why(tafs_core::signin::refusal(&answer, Some(w))
            .words(w)
            .unwrap_or_default()));
    }
    ConnectApps::read(&answer.body)
        .ok_or_else(|| why(tafs_core::signin::plain_words(w, 200).unwrap_or_default()))
}

/// `connectableTafs`: the tafs an app can be connected for, on tafs cloud and managed by this
/// person. Who manages a taf isn't read on Linux yet, so none: Connect stays dimmed, as the
/// iPad's when no taf can have it.
fn connectable() -> &'static [String] {
    &[]
}

pub fn page(w: Rc<WordBook>, source: ConnectSource, open_coach: Rc<dyn Fn()>) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
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
    // Reads the first page for what was typed, or the next page for the same search.
    let load: Rc<dyn Fn(bool)> = {
        let (w, source, state, redraw) = (w.clone(), source.clone(), state.clone(), redraw.clone());
        Rc::new(move |more: bool| {
            let ConnectSource::Cloud { token, languages } = &source else {
                // The sample lists the reference set's apps, unsearched.
                if let ConnectSource::Sample { screenshot: true } = source {
                    let mut s = state.borrow_mut();
                    if s.query.is_empty() {
                        s.apps = sample_apps();
                    }
                }
                redraw();
                return;
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
            let (words, token, languages) = ((*w).clone(), token.clone(), languages.clone());
            let (state, redraw) = (state.clone(), redraw.clone());
            gtk::glib::spawn_future_local(async move {
                let read = gtk::gio::spawn_blocking(move || {
                    read_page(&words, &token, &languages, &p, more)
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
        let (w, list, state, load, open_coach) = (
            w.clone(),
            list.clone(),
            state.clone(),
            load.clone(),
            open_coach.clone(),
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
            let (yours, discover) = split(&apps.apps);
            if !yours.is_empty() {
                list.append(&section_header(&l("connect.apps.yours")));
                let group = group();
                group.append(&columns(&w));
                for app in &yours {
                    group.append(&separator());
                    group.append(&row(&w, app));
                }
                list.append(&group);
            }
            if !discover.is_empty() {
                list.append(&section_header(&l("connect.apps.discover")));
                let group = group();
                let mut first = true;
                // Connecting needs a taf: said over the apps to connect, never over Yours.
                if says_make_a_taf(discover.len(), false, connectable().len()) {
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
                if yours.is_empty() {
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
                    group.append(&row(&w, app));
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
    crate::widgets::ground(&outer);
    outer.upcast()
}

/// A section's header, as the iPad's inset list draws it: 17 semibold, soft, inset.
fn section_header(words: &str) -> gtk::Label {
    let h = one_line(words, 17, 600, &["tafs-soft"]);
    h.set_margin_start(12);
    h.set_margin_top(10);
    h
}

/// The inset list's group: a white panel with corners of 18.
fn group() -> gtk::Box {
    let g = gtk::Box::new(gtk::Orientation::Vertical, 0);
    g.add_css_class("tafs-group");
    g
}

fn separator() -> gtk::Separator {
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

/// One app (`cells`): its name and what it is, how it connects, whose it is, and where it stands
/// with who uses it, then its ✓ or Connect. A connected app's detail opens with its port.
fn row(w: &WordBook, app: &App) -> gtk::Widget {
    let said = Row::new(w, app, &[]).unwrap_or_else(|_| Row {
        how: String::new(),
        whose: None,
        status: None,
        used_by: None,
        line: None,
        setup: None,
    });
    // The name on one line, cut at its end only as a last resort; what it is under it, in two.
    let name = gtk::Box::new(gtk::Orientation::Vertical, 1);
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
    // A ✓ for a connected app; else Connect (or Sign in again) as wide as its words, dimmed
    // while no taf can have it; nothing for an app not set up yet.
    if app.needs_setup() {
    } else if app.connected() {
        let check = symbol("tafs-checkmark-symbolic", 17);
        check.add_css_class("tafs-ok");
        check.update_property(&[gtk::accessible::Property::Label(
            &w.line("connect.status.connected").unwrap_or_default(),
        )]);
        status.append(&check);
    } else {
        let label = w
            .line(if app.needs_reconnect() {
                "connect.sign_in_again"
            } else {
                "connect.apps.connect"
            })
            .unwrap_or_default();
        let b = outline(&label);
        b.set_hexpand(false);
        b.add_css_class("compact");
        b.set_valign(gtk::Align::Center);
        b.set_sensitive(!connectable().is_empty());
        status.append(&b);
    }
    let r = table_row([name.upcast(), how.upcast(), whose.upcast(), status.upcast()]);
    r.set_size_request(-1, 52);
    r.set_margin_top(10);
    r.set_margin_bottom(10);
    r
}
