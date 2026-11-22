//! The calendar beside the sidebar (`CalendarView`): the tafs' scheduled work still to come, in
//! Day, Week and Month, read from tafs cloud for the selected month's six weeks. Wide (a month
//! navigator, the calendar beside it, the agenda under them) or compact (a week strip, the
//! selected day under it), by the iPad's own rule on the page's size. A taf's work opens the taf.
//! The lines are `tafs-core/src/calendar.rs`.

use std::cell::RefCell;
use std::rc::Rc;

use gtk::prelude::*;
use tafs_core::calendar::{
    self as cal, agenda, current, day_heading, events_on, first_hour, layout, marks, path, range,
    slots, step, strip_days, title, today, week, weekday_index, weekday_name, CalendarRead, Event,
    Layout, Mode, State,
};
use tafs_core::words::WordBook;

use crate::style::avatar_class;
use crate::widgets::{eyebrow, one_line, symbol, text};

/// Where the calendar reads.
#[derive(Clone)]
pub enum CalendarSource {
    /// No account to read (the sample, signed out): an empty range, as the iPad's screenshot.
    Empty,
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
}

/// What a taf's work opens: the taf's page, by tafs cloud's id, when this person has it listed.
pub struct Opening {
    pub known: Vec<String>,
    pub open: Rc<dyn Fn(String)>,
}

struct Data {
    state: State,
    read: Option<CalendarRead>,
    error: Option<String>,
    /// The range being read, so an older answer never replaces a newer one.
    reading: Option<(String, String)>,
    drawn: Option<Layout>,
}

type Redraw = Rc<dyn Fn()>;

const HOUR_HEIGHT: i32 = 44;

pub fn page(
    w: Rc<WordBook>,
    source: CalendarSource,
    opening: Rc<Opening>,
    first: State,
) -> gtk::Widget {
    let outer = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let heading = one_line(
        &w.line("calendar.heading").unwrap_or_default(),
        17,
        600,
        &["tafs-ink"],
    );
    heading.set_halign(gtk::Align::Center);
    heading.set_margin_top(12);
    outer.append(&heading);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 18);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_vexpand(true);
    scroller.set_child(Some(&column));
    outer.append(&scroller);

    let data = Rc::new(RefCell::new(Data {
        state: first,
        read: None,
        error: None,
        reading: None,
        drawn: None,
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
    // Reads the selected month's range, when it isn't the one read or being read.
    let load: Rc<dyn Fn(bool)> = {
        let (w, source, data, redraw) = (w.clone(), source.clone(), data.clone(), redraw.clone());
        Rc::new(move |again: bool| {
            let (from, to) = range(&data.borrow().state.selected);
            {
                let mut d = data.borrow_mut();
                let have = d
                    .read
                    .as_ref()
                    .is_some_and(|r| r.from == from && r.to == to);
                if !again && (have || d.reading == Some((from.clone(), to.clone()))) {
                    return;
                }
                d.error = None;
                d.reading = Some((from.clone(), to.clone()));
            }
            let CalendarSource::Cloud { token, languages } = &source else {
                let mut d = data.borrow_mut();
                d.read = Some(CalendarRead::empty(&from, &to));
                d.reading = None;
                return;
            };
            let (words, token, languages) = ((*w).clone(), token.clone(), languages.clone());
            let (data, redraw) = (data.clone(), redraw.clone());
            let p = path(&from, &to);
            gtk::glib::spawn_future_local(async move {
                let read = gtk::gio::spawn_blocking(move || {
                    let why = |why: String| {
                        words
                            .get("home.calendar_not_read", &[("why", &why)])
                            .unwrap_or(why)
                    };
                    let a = crate::kiff::cloud(&token, &languages, &p)
                        .map_err(|e| why(e.words(&words).unwrap_or_default()))?;
                    if !(200..300).contains(&a.status) {
                        return Err(why(tafs_core::signin::refusal(&a, Some(&words))
                            .words(&words)
                            .unwrap_or_default()));
                    }
                    CalendarRead::read(&a.body).ok_or_else(|| {
                        why(tafs_core::signin::plain_words(&words, 200).unwrap_or_default())
                    })
                })
                .await
                .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut d = data.borrow_mut();
                    if d.reading != Some((from.clone(), to.clone())) {
                        return;
                    }
                    d.reading = None;
                    match read {
                        Ok(mut r) => {
                            // The range asked for, so `current` matches it.
                            if r.from.is_empty() {
                                r.from = from;
                                r.to = to;
                            }
                            d.read = Some(r);
                        }
                        Err(e) => d.error = Some(e),
                    }
                }
                redraw();
            });
        })
    };
    // Shows another view or date, and reads its range when it's another month's.
    let show: Rc<dyn Fn(State)> = {
        let (data, load, redraw) = (data.clone(), load.clone(), redraw.clone());
        Rc::new(move |s: State| {
            data.borrow_mut().state = s;
            load(false);
            redraw();
        })
    };
    *draw.borrow_mut() = Some(Rc::new({
        let (w, column, outer, data, show, load, opening) = (
            w.clone(),
            column.clone(),
            outer.clone(),
            data.clone(),
            show.clone(),
            load.clone(),
            opening.clone(),
        );
        move || {
            while let Some(c) = column.first_child() {
                column.remove(&c);
            }
            let (width, height) = (outer.width(), outer.height());
            // On its side: the window's shape (`layout_in`).
            let landscape = outer
                .root()
                .map(|r| r.width() > r.height())
                .unwrap_or(width > height);
            let how = if width > 0 {
                tafs_core::calendar::layout_in(f64::from(width), f64::from(height), landscape)
            } else {
                Layout::Wide
            };
            data.borrow_mut().drawn = Some(how);
            let d = data.borrow();
            let ctx = Ctx {
                w: &w,
                state: &d.state,
                read: current(d.read.as_ref(), &d.state.selected),
                error: d.error.as_deref(),
                today: today(),
                show: &show,
                load: &load,
                opening: &opening,
            };
            let pad = if how == Layout::Wide { 24 } else { 16 };
            column.set_margin_start(pad);
            column.set_margin_end(pad);
            column.set_margin_top(pad);
            column.set_margin_bottom(pad);
            match how {
                Layout::Wide => {
                    let top = gtk::Box::new(gtk::Orientation::Horizontal, 24);
                    let nav = navigator(&ctx);
                    nav.set_size_request(280, -1);
                    nav.set_valign(gtk::Align::Start);
                    top.append(&nav);
                    let right = gtk::Box::new(gtk::Orientation::Vertical, 14);
                    right.set_hexpand(true);
                    header(&ctx, &right, false);
                    match ctx.state.mode {
                        Mode::Day => right
                            .append(&time_grid(&ctx, std::slice::from_ref(&ctx.state.selected))),
                        Mode::Week => right.append(&time_grid(&ctx, &week(&ctx.state.selected))),
                        Mode::Month => right.append(&month_cells(&ctx, true)),
                    }
                    top.append(&right);
                    column.append(&top);
                    column.append(&agenda_list(&ctx));
                }
                Layout::Compact => {
                    let v = gtk::Box::new(gtk::Orientation::Vertical, 14);
                    header(&ctx, &v, true);
                    match ctx.state.mode {
                        Mode::Day => {
                            v.append(&week_strip(&ctx));
                            v.append(&time_grid(&ctx, std::slice::from_ref(&ctx.state.selected)));
                        }
                        Mode::Week => {
                            v.append(&week_strip(&ctx));
                            v.append(&day_list(&ctx, &ctx.state.selected));
                        }
                        Mode::Month => {
                            v.append(&month_cells(&ctx, false));
                            v.append(&day_list(&ctx, &ctx.state.selected));
                        }
                    }
                    column.append(&v);
                }
            }
        }
    }));
    // Draws again when the page's size changes the layout (`GeometryReader`).
    {
        let (data, redraw) = (data.clone(), redraw.clone());
        outer.add_tick_callback(move |widget, _| {
            let (wd, h) = (widget.width(), widget.height());
            if wd > 0 {
                let how = layout(f64::from(wd), f64::from(h));
                if data.borrow().drawn != Some(how) {
                    redraw();
                }
            }
            gtk::glib::ControlFlow::Continue
        });
    }
    // Pull to refresh (`.refreshable`): the range read again. Not while a read is on its way.
    let refresh: Rc<dyn Fn()> = {
        let (data, load, redraw) = (data.clone(), load.clone(), redraw.clone());
        Rc::new(move || {
            if data.borrow().reading.is_some() {
                return;
            }
            load(true);
            redraw();
        })
    };
    crate::widgets::refreshable(&scroller, &outer, refresh);
    load(false);
    redraw();
    crate::widgets::ground(&outer);
    outer.upcast()
}

struct Ctx<'a> {
    w: &'a WordBook,
    state: &'a State,
    read: Option<&'a CalendarRead>,
    error: Option<&'a str>,
    today: String,
    show: &'a Rc<dyn Fn(State)>,
    load: &'a Rc<dyn Fn(bool)>,
    opening: &'a Rc<Opening>,
}

impl Ctx<'_> {
    fn l(&self, k: &str) -> String {
        self.w.line(k).unwrap_or_default()
    }
    fn events(&self) -> &[Event] {
        self.read.map(|r| r.events.as_slice()).unwrap_or(&[])
    }
    fn select(&self, day: &str) -> impl Fn() + 'static {
        let (show, mode, day) = (self.show.clone(), self.state.mode, day.to_string());
        move || {
            show(State {
                mode,
                selected: day.clone(),
            })
        }
    }
}

fn chevron(name: &str, label: &str, then: impl Fn() + 'static) -> gtk::Button {
    let b = gtk::Button::new();
    b.set_child(Some(&symbol(name, 17)));
    b.add_css_class("tafs-chevron");
    b.update_property(&[gtk::accessible::Property::Label(label)]);
    b.set_tooltip_text(Some(label));
    b.connect_clicked(move |_| then());
    b
}

/// Today, previous and next, the date, and Day / Week / Month; then why it couldn't be read,
/// the reading, or tafs cloud's words when the account is halted or some work was left out.
fn header(ctx: &Ctx, into: &gtk::Box, compact: bool) {
    let title_words = title(ctx.w, ctx.state).unwrap_or_default();
    let t = if compact {
        text(&title_words, 16, 700, &["tafs-ink"])
    } else {
        one_line(&title_words, 20, 700, &["tafs-ink"])
    };
    let nav = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    // Today: a pill as wide as its word; with today shown it does nothing, readable, never faded.
    let at_today = ctx.state.selected == ctx.today;
    let today_pill = gtk::Button::with_label(&ctx.l("calendar.today"));
    today_pill.add_css_class("tafs-today-pill");
    if at_today {
        today_pill.add_css_class("at-today");
    }
    {
        let go = ctx.select(&ctx.today);
        today_pill.connect_clicked(move |_| {
            if !at_today {
                go()
            }
        });
    }
    nav.append(&today_pill);
    let (prev, next) = match ctx.state.mode {
        Mode::Day => ("calendar.previous_day", "calendar.next_day"),
        Mode::Week => ("calendar.previous_week", "calendar.next_week"),
        Mode::Month => ("calendar.previous_month", "calendar.next_month"),
    };
    for (icon, key, n) in [
        ("go-previous-symbolic", prev, -1),
        ("go-next-symbolic", next, 1),
    ] {
        let (show, s) = (ctx.show.clone(), step(ctx.state, n));
        nav.append(&chevron(icon, &ctx.l(key), move || show(s.clone())));
    }
    if !compact {
        nav.append(&t);
    }
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    nav.append(&spacer);
    nav.append(&date_picker(ctx));
    // Day, Week, Month as the segmented control.
    let modes = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    modes.add_css_class("tafs-segmented");
    modes.set_homogeneous(true);
    let mut group: Option<gtk::ToggleButton> = None;
    for m in Mode::ALL {
        let b = gtk::ToggleButton::with_label(&m.title(ctx.w).unwrap_or_default());
        b.add_css_class("segment");
        if let Some(g) = &group {
            b.set_group(Some(g));
        } else {
            group = Some(b.clone());
        }
        b.set_active(m == ctx.state.mode);
        let (show, selected) = (ctx.show.clone(), ctx.state.selected.clone());
        b.connect_toggled(move |b| {
            if b.is_active() {
                let s = State {
                    mode: m,
                    selected: selected.clone(),
                };
                let show = show.clone();
                // After the toggle has settled, never inside its own signal.
                gtk::glib::idle_add_local_once(move || show(s));
            }
        });
        modes.append(&b);
    }
    modes.update_property(&[gtk::accessible::Property::Label(
        &ctx.l("calendar.view_picker"),
    )]);
    if compact {
        into.append(&t);
        into.append(&nav);
        into.append(&modes);
    } else {
        into.append(&nav);
        modes.set_size_request(320, -1);
        modes.set_halign(gtk::Align::Start);
        into.append(&modes);
    }
    if let Some(why) = ctx.error {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let e = text(why, 13, 400, &["tafs-no"]);
        e.set_hexpand(true);
        row.append(&e);
        let again = crate::widgets::outline(&ctx.l("common.try_again"));
        again.set_hexpand(false);
        let load = ctx.load.clone();
        again.connect_clicked(move |_| load(true));
        row.append(&again);
        into.append(&row);
    } else if ctx.read.is_none() {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let spinner = gtk::Spinner::new();
        spinner.start();
        row.append(&spinner);
        row.append(&one_line(
            &ctx.l("calendar.reading"),
            13,
            400,
            &["tafs-dim"],
        ));
        into.append(&row);
    } else if let Some(r) = ctx.read.filter(|r| r.halted || r.more) {
        into.append(&text(
            &r.words,
            13,
            600,
            &[if r.halted { "tafs-no" } else { "tafs-soft" }],
        ));
    }
}

/// The date picker (`DatePicker`, compact): the date as a pill, a month calendar under it.
fn date_picker(ctx: &Ctx) -> gtk::MenuButton {
    let pill = gtk::MenuButton::new();
    // The date alone, as the iPad's compact picker: no arrow.
    pill.set_child(Some(&gtk::Label::new(Some(&cal::medium_date(
        ctx.w.language(),
        &ctx.state.selected,
    )))));
    pill.add_css_class("tafs-date-pill");
    pill.set_always_show_arrow(false);
    pill.update_property(&[gtk::accessible::Property::Label(
        &ctx.l("calendar.choose_date"),
    )]);
    let picker = gtk::Calendar::new();
    if let Ok(d) = chrono::NaiveDate::parse_from_str(&ctx.state.selected, "%Y-%m-%d") {
        use chrono::Datelike;
        if let Ok(dt) =
            gtk::glib::DateTime::from_utc(d.year(), d.month() as i32, d.day() as i32, 12, 0, 0.0)
        {
            picker.select_day(&dt);
        }
    }
    let popover = gtk::Popover::new();
    popover.set_child(Some(&picker));
    pill.set_popover(Some(&popover));
    let (show, mode) = (ctx.show.clone(), ctx.state.mode);
    picker.connect_day_selected(move |c| {
        let d = c.date();
        let day = format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day_of_month());
        popover.popdown();
        let show = show.clone();
        gtk::glib::idle_add_local_once(move || {
            show(State {
                mode,
                selected: day,
            })
        });
    });
    pill
}

/// The navigator, beside the calendar when wide: the month's title, its previous and next, and
/// its days.
fn navigator(ctx: &Ctx) -> gtk::Box {
    let p = gtk::Box::new(gtk::Orientation::Vertical, 10);
    p.add_css_class("tafs-navigator");
    let top = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let month = title(
        ctx.w,
        &State {
            mode: Mode::Month,
            selected: ctx.state.selected.clone(),
        },
    )
    .unwrap_or_default();
    let t = one_line(&month, 15, 600, &["tafs-ink"]);
    t.set_hexpand(true);
    top.append(&t);
    for (icon, key, n) in [
        ("go-previous-symbolic", "calendar.previous_month", -1),
        ("go-next-symbolic", "calendar.next_month", 1),
    ] {
        let go = ctx.select(&cal::plus_months(&ctx.state.selected, n));
        top.append(&chevron(icon, &ctx.l(key), go));
    }
    p.append(&top);
    p.append(&month_cells(ctx, false));
    p
}

/// The month's six weeks: the weekdays, then each day with its number, its tafs' marks, and how
/// many when large; the selected day in the amber at 18%, today ringed in the amber.
fn month_cells(ctx: &Ctx, large: bool) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let grid = gtk::Grid::new();
    grid.set_column_spacing(4);
    grid.set_row_spacing(4);
    grid.set_column_homogeneous(true);
    for i in 0..7 {
        let n = one_line(
            &weekday_name(ctx.w, i).unwrap_or_default(),
            11,
            700,
            &["tafs-dim"],
        );
        n.set_halign(gtk::Align::Center);
        grid.attach(&n, i as i32, 0, 1, 1);
    }
    let events = ctx.events();
    for (i, day) in cal::month_grid(&ctx.state.selected).iter().enumerate() {
        let in_month = cal::same_month(day, &ctx.state.selected);
        let selected = *day == ctx.state.selected;
        let count = cal::count(events, day);
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 3);
        cell.set_valign(gtk::Align::Center);
        let number = one_line(
            &cal::day_number(day).to_string(),
            if large { 15 } else { 13 },
            if selected { 700 } else { 400 },
            &[if in_month {
                "tafs-ink"
            } else {
                "tafs-out-of-month"
            }],
        );
        number.set_halign(gtk::Align::Center);
        cell.append(&number);
        cell.append(&marks_row(&marks(events, day), false));
        if large && count > 0 {
            let c = one_line(&count.to_string(), 11, 400, &["tafs-soft"]);
            c.set_halign(gtk::Align::Center);
            cell.append(&c);
        }
        let b = gtk::Button::new();
        b.set_child(Some(&cell));
        b.add_css_class("tafs-day-cell");
        if selected {
            b.add_css_class("selected");
        }
        if *day == ctx.today {
            b.add_css_class("today");
        }
        b.set_size_request(-1, if large { 64 } else { 38 });
        let heading = day_heading(ctx.w, day).unwrap_or_default();
        let label = if count > 0 {
            ctx.w
                .count("calendar.day_scheduled", count, &[("day", &heading)])
                .unwrap_or(heading)
        } else {
            heading
        };
        b.update_property(&[gtk::accessible::Property::Label(&label)]);
        let go = ctx.select(day);
        b.connect_clicked(move |_| go());
        grid.attach(&b, (i % 7) as i32, (i / 7 + 1) as i32, 1, 1);
    }
    v.append(&grid);
    v
}

/// A day's marks: a dot in each taf's colour, at most three; in the amber's ink when selected.
fn marks_row(colors: &[i64], on_amber: bool) -> gtk::Box {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 2);
    row.set_halign(gtk::Align::Center);
    row.set_size_request(-1, 5);
    for c in colors {
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("tafs-mark");
        if on_amber {
            dot.add_css_class("on-amber");
        } else {
            dot.add_css_class(&avatar_class(*c));
        }
        row.append(&dot);
    }
    row
}

/// The compact strip: in Week the seven days sharing the width; in Day the month's weeks in a
/// row that scrolls, the selected day's week first.
fn week_strip(ctx: &Ctx) -> gtk::Widget {
    let fills = ctx.state.mode == Mode::Week;
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    row.set_homogeneous(fills);
    let days = strip_days(ctx.state);
    let start = cal::week_start(&ctx.state.selected);
    let first_shown = days.iter().position(|d| *d == start).unwrap_or(0);
    for day in &days {
        let selected = *day == ctx.state.selected;
        let cell = gtk::Box::new(gtk::Orientation::Vertical, 4);
        cell.set_valign(gtk::Align::Center);
        let tone = if selected {
            "tafs-amber-ink"
        } else {
            "tafs-dim"
        };
        let wd = one_line(
            &weekday_name(ctx.w, weekday_index(day)).unwrap_or_default(),
            11,
            700,
            &[tone],
        );
        wd.set_halign(gtk::Align::Center);
        cell.append(&wd);
        let n = one_line(
            &cal::day_number(day).to_string(),
            17,
            600,
            &[if selected {
                "tafs-amber-ink"
            } else {
                "tafs-ink"
            }],
        );
        n.set_halign(gtk::Align::Center);
        cell.append(&n);
        cell.append(&marks_row(&marks(ctx.events(), day), selected));
        let b = gtk::Button::new();
        b.set_child(Some(&cell));
        b.add_css_class("tafs-strip-cell");
        if selected {
            b.add_css_class("selected");
        } else if *day == ctx.today {
            b.add_css_class("today");
        }
        b.set_size_request(if fills { -1 } else { 46 }, 66);
        b.update_property(&[gtk::accessible::Property::Label(
            &day_heading(ctx.w, day).unwrap_or_default(),
        )]);
        let go = ctx.select(day);
        b.connect_clicked(move |_| go());
        row.append(&b);
    }
    if fills {
        return row.upcast();
    }
    let s = gtk::ScrolledWindow::new();
    // It scrolls without showing a bar (`showsIndicators: false`).
    s.set_policy(gtk::PolicyType::External, gtk::PolicyType::Never);
    s.set_child(Some(&row));
    // The selected day's week first.
    let adj = s.hadjustment();
    let x = f64::from(first_shown as i32 * (46 + 6));
    gtk::glib::idle_add_local_once(move || adj.set_value(x));
    s.upcast()
}

/// Day and Week's time slots: a heading for each day in Week, then the 24 hours in a view 520
/// high, scrolled to the first hour with work (8 with none); "Nothing scheduled" when none.
fn time_grid(ctx: &Ctx, days: &[String]) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let events = ctx.events();
    let by_day: Vec<_> = days.iter().map(|d| slots(events, d)).collect();
    if days.len() > 1 {
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        head.set_margin_bottom(6);
        let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        spacer.set_size_request(48, 1);
        head.append(&spacer);
        for day in days {
            let col = gtk::Box::new(gtk::Orientation::Vertical, 1);
            let wd = one_line(
                &weekday_name(ctx.w, weekday_index(day)).unwrap_or_default(),
                11,
                700,
                &["tafs-dim"],
            );
            wd.set_halign(gtk::Align::Center);
            col.append(&wd);
            let is_today = *day == ctx.today;
            let n = one_line(
                &cal::day_number(day).to_string(),
                15,
                if is_today { 700 } else { 400 },
                &[if is_today {
                    "tafs-amber-text"
                } else {
                    "tafs-ink"
                }],
            );
            n.set_halign(gtk::Align::Center);
            col.append(&n);
            let b = gtk::Button::new();
            b.set_child(Some(&col));
            b.add_css_class("tafs-plain");
            b.set_hexpand(true);
            b.update_property(&[gtk::accessible::Property::Label(
                &day_heading(ctx.w, day).unwrap_or_default(),
            )]);
            let (show, day) = (ctx.show.clone(), day.clone());
            b.connect_clicked(move |_| {
                show(State {
                    mode: Mode::Day,
                    selected: day.clone(),
                })
            });
            head.append(&b);
        }
        v.append(&head);
    }
    let hours = gtk::Box::new(gtk::Orientation::Vertical, 0);
    for hour in 0..24i64 {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        row.add_css_class("tafs-hour");
        let label = one_line(
            &format!("{hour:02}:00"),
            11,
            400,
            &["tafs-mono", "tafs-dim"],
        );
        label.set_size_request(48, -1);
        label.set_valign(gtk::Align::Start);
        row.append(&label);
        for slots in &by_day {
            let cell = gtk::Box::new(gtk::Orientation::Vertical, 2);
            cell.set_hexpand(true);
            cell.set_size_request(-1, HOUR_HEIGHT);
            cell.set_margin_top(3);
            for slot in slots.get(&hour).into_iter().flatten() {
                cell.append(&slot_chip(ctx, slot, days.len() > 1));
            }
            row.append(&cell);
        }
        hours.append(&row);
    }
    let s = gtk::ScrolledWindow::new();
    s.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
    s.set_min_content_height(520);
    s.set_max_content_height(520);
    s.set_child(Some(&hours));
    let first = first_hour(events, days);
    let adj = s.vadjustment();
    // Each hour's row is its least height and the line above it.
    let y = f64::from(first as i32 * (HOUR_HEIGHT + 3 + 1));
    gtk::glib::idle_add_local_once(move || adj.set_value(y));
    v.append(&s);
    if ctx.read.is_some() && days.iter().all(|d| events_on(events, d).is_empty()) {
        // Centred under the slots, as the iPad's.
        let e = empty(&ctx.l(if days.len() == 1 {
            "calendar.nothing_on_day"
        } else {
            "calendar.nothing_this_week"
        }));
        e.set_halign(gtk::Align::Center);
        v.append(&e);
    }
    v
}

fn slot_chip(ctx: &Ctx, slot: &cal::Slot, compact: bool) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    row.add_css_class("tafs-slot");
    row.add_css_class(&avatar_class(slot.color));
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.add_css_class("tafs-dot7");
    dot.add_css_class(&avatar_class(slot.color));
    dot.set_valign(gtk::Align::Center);
    row.append(&dot);
    let words = if compact {
        slot.name.clone()
    } else {
        format!("{} · {}", slot.first, slot.name)
    };
    let name = one_line(&words, 12, 600, &["tafs-ink"]);
    name.set_hexpand(true);
    row.append(&name);
    if slot.count > 1 {
        row.append(&one_line(
            &ctx.w
                .count("calendar.times", slot.count, &[])
                .unwrap_or_else(|_| format!("×{}", slot.count)),
            11,
            400,
            &["tafs-soft"],
        ));
    }
    let label = if slot.count > 1 {
        ctx.w.count(
            "calendar.slot_times",
            slot.count,
            &[("name", &slot.name), ("first", &slot.first)],
        )
    } else {
        ctx.w.get(
            "calendar.slot_at",
            &[("name", &slot.name), ("first", &slot.first)],
        )
    }
    .unwrap_or_default();
    opening(ctx, &slot.taf, row.upcast(), &label)
}

/// A taf's work opens the taf beside the sidebar; a taf this person no longer has listed opens
/// nothing.
fn opening(ctx: &Ctx, taf: &str, content: gtk::Widget, label: &str) -> gtk::Widget {
    if !ctx.opening.known.iter().any(|k| k == taf) {
        content.update_property(&[gtk::accessible::Property::Label(label)]);
        return content;
    }
    let b = gtk::Button::new();
    b.set_child(Some(&content));
    b.add_css_class("tafs-plain");
    b.update_property(&[gtk::accessible::Property::Label(label)]);
    let (open, taf) = (ctx.opening.open.clone(), taf.to_string());
    b.connect_clicked(move |_| open(taf.clone()));
    b.upcast()
}

/// The selected day's work (compact): its heading, then its events.
fn day_list(ctx: &Ctx, day: &str) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 8);
    v.append(&eyebrow(
        &day_heading(ctx.w, day).unwrap_or_default(),
        false,
    ));
    let on = events_on(ctx.events(), day);
    if ctx.read.is_some() && on.is_empty() {
        v.append(&empty(&ctx.l("calendar.nothing_on_day")));
    }
    for e in on {
        v.append(&event_row(ctx, e));
    }
    v
}

/// The agenda (wide): the shown dates' work by day; tafs cloud's words when nothing is to come.
fn agenda_list(ctx: &Ctx) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 10);
    v.append(&eyebrow(&ctx.l("calendar.agenda"), false));
    let groups = agenda(ctx.events(), &cal::shown(ctx.state));
    if let (Some(r), true) = (ctx.read, groups.is_empty()) {
        v.append(&empty(&if r.words.is_empty() {
            ctx.l("calendar.nothing_these_days")
        } else {
            r.words.clone()
        }));
    }
    for (day, events) in groups {
        let g = gtk::Box::new(gtk::Orientation::Vertical, 6);
        g.append(&one_line(
            &day_heading(ctx.w, &day).unwrap_or_default(),
            14,
            600,
            &[if day == ctx.today {
                "tafs-amber-text"
            } else {
                "tafs-ink"
            }],
        ));
        for e in events {
            g.append(&event_row(ctx, e));
        }
        v.append(&g);
    }
    v
}

fn event_row(ctx: &Ctx, e: &Event) -> gtk::Widget {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.add_css_class("tafs-event");
    let time = one_line(&e.time, 13, 400, &["tafs-mono", "tafs-soft"]);
    time.set_size_request(46, -1);
    row.append(&time);
    let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    dot.add_css_class("tafs-dot9");
    dot.add_css_class(&avatar_class(e.color));
    dot.set_valign(gtk::Align::Center);
    row.append(&dot);
    let words = gtk::Box::new(gtk::Orientation::Vertical, 2);
    words.append(&one_line(&e.name, 14, 600, &["tafs-ink"]));
    words.append(&one_line(&e.words, 12, 400, &["tafs-dim"]));
    words.set_hexpand(true);
    row.append(&words);
    let label = format!("{} {} {}", e.time, e.name, e.words);
    opening(ctx, &e.taf, row.upcast(), &label)
}

fn empty(words: &str) -> gtk::Label {
    let l = text(words, 13, 400, &["tafs-dim"]);
    l.set_margin_top(6);
    l.set_margin_bottom(6);
    l
}
