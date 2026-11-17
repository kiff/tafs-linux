//! The calendar (`CalendarView`, `CalendarModel.swift`): the tafs' scheduled work still to come,
//! as tafs cloud reads it (`TafsCloud.CalendarRead`, `GET /v1/calendar`), in Day, Week and Month.
//! Every occurrence is tafs cloud's, placed by the day and minute it gives in the account's zone:
//! the app converts no zone. Dates are YYYY-MM-DD on the Gregorian calendar, weeks from Monday.

use chrono::{Datelike, Months, NaiveDate};
use serde_json::Value;

use crate::words::{WordBook, WordsError};

/// One occurrence (`CalendarEvent`): a schedule's firing, a follow-up, or a run's next try.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub taf: String,
    pub name: String,
    pub color: i64,
    /// `schedule`, `follow_up` or `retry`.
    pub kind: String,
    pub at: String,
    pub day: String,
    pub time: String,
    pub minute: i64,
    pub words: String,
    /// How many firings it stands for (at least one).
    pub count: i64,
}

impl Event {
    fn read(v: &Value) -> Option<Event> {
        let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        Some(Event {
            taf: text("taf")?,
            at: text("at")?,
            day: text("day")?,
            time: text("time")?,
            minute: v.get("minute")?.as_i64()?,
            name: text("name").unwrap_or_default(),
            color: v.get("color").and_then(Value::as_i64).unwrap_or(0),
            kind: text("kind").unwrap_or_else(|| "schedule".into()),
            words: text("words").unwrap_or_default(),
            count: v.get("count").and_then(Value::as_i64).unwrap_or(1).max(1),
        })
    }
}

/// tafs cloud's read of a range (`CalendarRead`): lenient, an event it can't read left out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalendarRead {
    pub time_zone: String,
    pub from: String,
    pub to: String,
    pub events: Vec<Event>,
    pub days: Vec<String>,
    /// tafs cloud's line for the range ("3 to come", "Halted: nothing runs until it's resumed").
    pub words: String,
    pub more: bool,
    pub halted: bool,
}

impl CalendarRead {
    /// An empty read of a range (the sample's, as the iPad's screenshot).
    pub fn empty(from: &str, to: &str) -> CalendarRead {
        CalendarRead {
            time_zone: "UTC".into(),
            from: from.into(),
            to: to.into(),
            events: Vec::new(),
            days: Vec::new(),
            words: String::new(),
            more: false,
            halted: false,
        }
    }

    pub fn read(body: &[u8]) -> Option<CalendarRead> {
        let v: Value = serde_json::from_slice(body).ok()?;
        if !v.is_object() {
            return None;
        }
        let text = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        let flag = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
        Some(CalendarRead {
            time_zone: text("time_zone").unwrap_or_else(|| "UTC".into()),
            from: text("from").unwrap_or_default(),
            to: text("to").unwrap_or_default(),
            events: v
                .get("events")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(Event::read).collect())
                .unwrap_or_default(),
            // A list of strings, else none (`try? decode([String])`).
            days: v
                .get("days")
                .and_then(Value::as_array)
                .and_then(|a| {
                    a.iter()
                        .map(|d| d.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()
                })
                .unwrap_or_default(),
            words: text("words").unwrap_or_default(),
            more: flag("more"),
            halted: flag("halted"),
        })
    }
}

/// `GET /v1/calendar?from=…&to=…` on tafs cloud.
pub fn path(from: &str, to: &str) -> String {
    format!("/v1/calendar?from={from}&to={to}")
}

/// The calendar's three views; Week is the one it opens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Day,
    Week,
    Month,
}

impl Mode {
    pub const ALL: [Mode; 3] = [Mode::Day, Mode::Week, Mode::Month];
    pub fn title(self, w: &WordBook) -> Result<String, WordsError> {
        w.line(match self {
            Mode::Day => "calendar.mode.day",
            Mode::Week => "calendar.mode.week",
            Mode::Month => "calendar.mode.month",
        })
    }
}

/// What the calendar shows: its view and the selected date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct State {
    pub mode: Mode,
    pub selected: String,
}

/// `CalendarLayout.of`: wide (a month navigator, the calendar beside it, an agenda under it) only
/// with room for the navigator and a landscape tablet's height; compact otherwise.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Layout {
    Wide,
    Compact,
}

pub fn layout(width: f64, height: f64) -> Layout {
    layout_in(width, height, width > height)
}

/// `layout` where "the screen lies on its side" is the window's shape, not the page's: a 4:3
/// window less the sidebar leaves a page about as tall as it's wide, where the iPad in landscape
/// is always wider.
pub fn layout_in(width: f64, height: f64, landscape: bool) -> Layout {
    if width >= 700.0 && landscape && height >= 600.0 {
        Layout::Wide
    } else {
        Layout::Compact
    }
}

fn parts(day: &str) -> Option<(i32, u32, u32)> {
    let p: Vec<i64> = day.split('-').filter_map(|x| x.parse().ok()).collect();
    if p.len() != 3 || !(1..=12).contains(&p[1]) || !(1..=31).contains(&p[2]) {
        return None;
    }
    Some((p[0] as i32, p[1] as u32, p[2] as u32))
}

fn date(day: &str) -> Option<NaiveDate> {
    let (y, m, d) = parts(day)?;
    NaiveDate::from_ymd_opt(y, m, d)
}

fn string(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Today as the account's clock shows it.
pub fn today() -> String {
    string(crate::account_time::day(&chrono::Utc::now()))
}

pub fn plus_days(day: &str, n: i64) -> String {
    date(day)
        .and_then(|d| d.checked_add_signed(chrono::Duration::days(n)))
        .map(string)
        .unwrap_or_else(|| day.to_string())
}

/// The same day of the month `n` months on, on the month's last day when it has fewer.
pub fn plus_months(day: &str, n: i32) -> String {
    let Some(d) = date(day) else {
        return day.to_string();
    };
    let out = if n >= 0 {
        d.checked_add_months(Months::new(n as u32))
    } else {
        d.checked_sub_months(Months::new(n.unsigned_abs()))
    };
    out.map(string).unwrap_or_else(|| day.to_string())
}

/// 0 for Monday to 6 for Sunday.
pub fn weekday_index(day: &str) -> usize {
    date(day)
        .map(|d| d.weekday().num_days_from_monday() as usize)
        .unwrap_or(0)
}

pub fn week_start(day: &str) -> String {
    plus_days(day, -(weekday_index(day) as i64))
}

pub fn week(day: &str) -> Vec<String> {
    let start = week_start(day);
    (0..7).map(|i| plus_days(&start, i)).collect()
}

pub fn month_start(day: &str) -> String {
    match parts(day) {
        Some((y, m, _)) => format!("{y:04}-{m:02}-01"),
        None => day.to_string(),
    }
}

/// Six weeks from the Monday on or before the month's first day.
pub fn month_grid(day: &str) -> Vec<String> {
    let start = week_start(&month_start(day));
    (0..42).map(|i| plus_days(&start, i)).collect()
}

pub fn same_month(a: &str, b: &str) -> bool {
    a.get(..7) == b.get(..7)
}

pub fn day_number(day: &str) -> u32 {
    parts(day).map(|p| p.2).unwrap_or(0)
}

/// The dates one read covers: the selected month's grid.
pub fn range(selected: &str) -> (String, String) {
    let grid = month_grid(selected);
    (grid[0].clone(), grid[41].clone())
}

/// The read when it answers the selected date's range; none while that range is being read.
pub fn current<'a>(read: Option<&'a CalendarRead>, selected: &str) -> Option<&'a CalendarRead> {
    let (from, to) = range(selected);
    read.filter(|r| r.from == from && r.to == to)
}

/// The dates a view shows.
pub fn shown(state: &State) -> Vec<String> {
    match state.mode {
        Mode::Day => vec![state.selected.clone()],
        Mode::Week => week(&state.selected),
        Mode::Month => month_grid(&state.selected)
            .into_iter()
            .filter(|d| same_month(d, &state.selected))
            .collect(),
    }
}

/// The compact strip's days: in Week the seven its heading names; in Day the month's weeks.
pub fn strip_days(state: &State) -> Vec<String> {
    match state.mode {
        Mode::Week => week(&state.selected),
        _ => month_grid(&state.selected),
    }
}

/// The previous or next day, week or month.
pub fn step(state: &State, n: i32) -> State {
    State {
        mode: state.mode,
        selected: match state.mode {
            Mode::Day => plus_days(&state.selected, n.into()),
            Mode::Week => plus_days(&state.selected, 7 * i64::from(n)),
            Mode::Month => plus_months(&state.selected, n),
        },
    }
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];
const WEEKDAYS: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];

pub fn weekday_name(w: &WordBook, i: usize) -> Result<String, WordsError> {
    w.line(&format!("calendar.weekday.{}", WEEKDAYS[i.min(6)]))
}

fn month_name(w: &WordBook, m: u32) -> Result<String, WordsError> {
    w.line(&format!(
        "calendar.month_name.{}",
        MONTHS[(m.clamp(1, 12) - 1) as usize]
    ))
}

fn month_title(w: &WordBook, m: u32) -> Result<String, WordsError> {
    w.line(&format!(
        "calendar.month_title.{}",
        MONTHS[(m.clamp(1, 12) - 1) as usize]
    ))
}

/// The view's title in the app's language's order: "Sun 15 November 2026", "9 – 15 November
/// 2026", "November 2026".
pub fn title(w: &WordBook, state: &State) -> Result<String, WordsError> {
    let Some((y, m, d)) = parts(&state.selected) else {
        return Ok(state.selected.clone());
    };
    match state.mode {
        Mode::Day => w.get(
            "calendar.title.day",
            &[
                ("weekday", &weekday_name(w, weekday_index(&state.selected))?),
                ("day", &d.to_string()),
                ("month", &month_name(w, m)?),
                ("year", &y.to_string()),
            ],
        ),
        Mode::Week => {
            let days = week(&state.selected);
            let (Some(a), Some(b)) = (parts(&days[0]), parts(&days[6])) else {
                return Ok(state.selected.clone());
            };
            if a.0 != b.0 {
                return w.get(
                    "calendar.title.week_across_years",
                    &[
                        ("from_day", &a.2.to_string()),
                        ("from_month", &month_name(w, a.1)?),
                        ("from_year", &a.0.to_string()),
                        ("to_day", &b.2.to_string()),
                        ("to_month", &month_name(w, b.1)?),
                        ("to_year", &b.0.to_string()),
                    ],
                );
            }
            if a.1 != b.1 {
                return w.get(
                    "calendar.title.week_across_months",
                    &[
                        ("from_day", &a.2.to_string()),
                        ("from_month", &month_name(w, a.1)?),
                        ("to_day", &b.2.to_string()),
                        ("to_month", &month_name(w, b.1)?),
                        ("year", &b.0.to_string()),
                    ],
                );
            }
            w.get(
                "calendar.title.week",
                &[
                    ("from_day", &a.2.to_string()),
                    ("to_day", &b.2.to_string()),
                    ("month", &month_name(w, b.1)?),
                    ("year", &b.0.to_string()),
                ],
            )
        }
        Mode::Month => w.get(
            "calendar.title.month",
            &[("month", &month_title(w, m)?), ("year", &y.to_string())],
        ),
    }
}

/// A day as a heading: "Thu 5 November".
pub fn day_heading(w: &WordBook, day: &str) -> Result<String, WordsError> {
    let Some((_, m, d)) = parts(day) else {
        return Ok(day.to_string());
    };
    w.get(
        "calendar.day_heading",
        &[
            ("weekday", &weekday_name(w, weekday_index(day))?),
            ("day", &d.to_string()),
            ("month", &month_name(w, m)?),
        ],
    )
}

/// A day's events in time order (minute, then name, then instant).
pub fn events_on<'a>(events: &'a [Event], day: &str) -> Vec<&'a Event> {
    let mut on: Vec<&Event> = events.iter().filter(|e| e.day == day).collect();
    on.sort_by(|a, b| (a.minute, &a.name, &a.at).cmp(&(b.minute, &b.name, &b.at)));
    on
}

/// The agenda: the events on the dates, by day, days with none left out.
pub fn agenda<'a>(events: &'a [Event], days: &[String]) -> Vec<(String, Vec<&'a Event>)> {
    days.iter()
        .filter_map(|d| {
            let on = events_on(events, d);
            (!on.is_empty()).then(|| (d.clone(), on))
        })
        .collect()
}

/// One hour's slot: each taf with work in it once, with how many times, in order of its first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Slot {
    pub taf: String,
    pub name: String,
    pub color: i64,
    pub count: i64,
    pub first: String,
}

/// A day's work by hour (0 to 23).
pub fn slots(events: &[Event], day: &str) -> std::collections::BTreeMap<i64, Vec<Slot>> {
    let mut out: std::collections::BTreeMap<i64, Vec<Slot>> = Default::default();
    for e in events_on(events, day) {
        let hour = (e.minute / 60).clamp(0, 23);
        let row = out.entry(hour).or_default();
        match row.iter_mut().find(|s| s.taf == e.taf) {
            Some(s) => s.count = s.count.saturating_add(e.count),
            None => row.push(Slot {
                taf: e.taf.clone(),
                name: e.name.clone(),
                color: e.color,
                count: e.count,
                first: e.time.clone(),
            }),
        }
    }
    out
}

/// The first hour with work among the dates; 8 when there's none.
pub fn first_hour(events: &[Event], days: &[String]) -> i64 {
    events
        .iter()
        .filter(|e| days.contains(&e.day))
        .map(|e| (e.minute / 60).clamp(0, 23))
        .min()
        .unwrap_or(8)
}

/// How many firings a day has.
pub fn count(events: &[Event], day: &str) -> i64 {
    events
        .iter()
        .filter(|e| e.day == day)
        .fold(0i64, |n, e| n.saturating_add(e.count))
}

/// The colours of the tafs with work on a day, at most three.
pub fn marks(events: &[Event], day: &str) -> Vec<i64> {
    let (mut seen, mut colors) = (Vec::new(), Vec::new());
    for e in events_on(events, day) {
        if !seen.contains(&e.taf) {
            seen.push(e.taf.clone());
            colors.push(e.color);
            if colors.len() == 3 {
                break;
            }
        }
    }
    colors
}

/// The date picker's pill: the date in the language's medium style ("Nov 15, 2026").
pub fn medium_date(language: &str, day: &str) -> String {
    match date(day) {
        Some(d) => crate::home::long_date(language, d),
        None => day.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(taf: &str, day: &str, minute: i64, count: i64) -> Event {
        Event {
            taf: taf.into(),
            name: taf.to_uppercase(),
            color: 1,
            kind: "schedule".into(),
            at: format!("{day}T00:00:00Z"),
            day: day.into(),
            time: format!("{:02}:{:02}", minute / 60, minute % 60),
            minute,
            words: String::new(),
            count,
        }
    }

    #[test]
    fn dates_count_from_monday() {
        assert_eq!(weekday_index("2026-11-15"), 6);
        assert_eq!(week_start("2026-11-15"), "2026-11-09");
        assert_eq!(
            range("2026-11-15"),
            ("2026-10-26".into(), "2026-12-06".into())
        );
        assert_eq!(plus_months("2026-01-31", 1), "2026-02-28");
        assert_eq!(plus_months("2026-03-31", -1), "2026-02-28");
        assert!(same_month("2026-11-01", "2026-11-30"));
        let s = State {
            mode: Mode::Week,
            selected: "2026-11-15".into(),
        };
        assert_eq!(step(&s, 1).selected, "2026-11-22");
        assert_eq!(
            shown(&State {
                mode: Mode::Month,
                selected: "2026-02-10".into()
            })
            .len(),
            28
        );
        assert_eq!(
            strip_days(&State {
                mode: Mode::Day,
                selected: "2026-11-15".into()
            })
            .len(),
            42
        );
    }

    #[test]
    fn the_read_is_lenient() {
        let r = CalendarRead::read(
            br#"{"from":"a","to":"b","events":[{"taf":"t","at":"x","day":"2026-11-15","time":"09:00","minute":540,"count":0},{"taf":"bad"}],"days":[1],"halted":true}"#,
        )
        .unwrap();
        assert_eq!(r.events.len(), 1);
        assert_eq!(r.events[0].count, 1);
        assert_eq!(r.events[0].kind, "schedule");
        assert!(r.days.is_empty());
        assert!(r.halted);
        assert_eq!(r.time_zone, "UTC");
    }

    #[test]
    fn slots_marks_and_agenda() {
        let events = vec![
            event("a", "2026-11-15", 9 * 60 + 30, 2),
            event("a", "2026-11-15", 9 * 60 + 45, 1),
            event("b", "2026-11-15", 7 * 60, 1),
            event("c", "2026-11-16", 60, 1),
        ];
        let s = slots(&events, "2026-11-15");
        assert_eq!(s[&9].len(), 1);
        assert_eq!(s[&9][0].count, 3);
        assert_eq!(s[&9][0].first, "09:30");
        assert_eq!(first_hour(&events, &["2026-11-15".into()]), 7);
        assert_eq!(first_hour(&events, &["2026-11-20".into()]), 8);
        assert_eq!(count(&events, "2026-11-15"), 4);
        // tafs cloud's counts are summed without overflowing, however large.
        let huge = vec![
            event("a", "2026-11-15", 0, i64::MAX),
            event("a", "2026-11-15", 1, 5),
        ];
        assert_eq!(count(&huge, "2026-11-15"), i64::MAX);
        assert_eq!(slots(&huge, "2026-11-15")[&0][0].count, i64::MAX);
        assert_eq!(marks(&events, "2026-11-15").len(), 2);
        let a = agenda(&events, &["2026-11-14".into(), "2026-11-15".into()]);
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].1[0].taf, "b");
    }

    #[test]
    fn layouts() {
        assert_eq!(layout(1000.0, 790.0), Layout::Wide);
        assert_eq!(layout(1000.0, 1300.0), Layout::Compact);
        assert_eq!(layout(600.0, 500.0), Layout::Compact);
        // A 4:3 window less the sidebar: the page about square, the window on its side.
        assert_eq!(layout_in(904.0, 900.0, true), Layout::Wide);
        assert_eq!(layout_in(904.0, 900.0, false), Layout::Compact);
        assert_eq!(layout_in(904.0, 560.0, true), Layout::Compact);
    }
}
