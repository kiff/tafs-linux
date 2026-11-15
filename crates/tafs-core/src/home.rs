//! Home from tafs cloud (`GET /v1/home`, `TafsCloud.home`), read as the iPad reads it
//! (`TafsCloud.Home`, `HomeTaf`), and each taf as the screen shows it (`KIFFMapping.taf`).
//!
//! As strict as the iPad's decoder: a field it requires and an answer lacks fails the read, and
//! the screen stays as it was. The parts the iPad reads leniently (a taf's health, its work line,
//! its last change, Home's headline) never fail it.

use std::collections::BTreeMap;

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::card::{CallsTotal, Card, MoneyTotal, Window};
use crate::money::Money;
use crate::overview::{CloudRun, OverviewTaf};
use crate::sidebar::{initial, relevant_now, TafRow, Tone};
use crate::words::{WordBook, WordsError};

/// tafs cloud's API.
pub const TAFS_CLOUD: &str = "https://cloud.tafs.kiff.dev";

/// An ISO time as tafs cloud writes it, with or without fractions of a second; anything else
/// fails the read, as the iPad's date decoding does.
fn time<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    match s {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|t| Some(t.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
    }
}

/// `last_change`: any shape, read as a time only when it's one (`HomeTaf.time`).
fn lenient_time(v: &Value) -> Option<DateTime<Utc>> {
    v.as_str()
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|t| t.with_timezone(&Utc))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Amount {
    pub minor: i64,
    pub unit: Option<String>,
    pub scale: u32,
}

impl Amount {
    /// `KIFFMapping.money`; none when its scale can't be read.
    pub fn money(&self) -> Option<Money> {
        Money::new(self.minor, self.unit.as_deref(), self.scale)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WindowWords {
    pub each: String,
    pub now: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Rule {
    pub action: String,
    pub label: String,
    pub outside: String,
    pub per_action: Option<Amount>,
    pub per_window: Option<Amount>,
    pub count_per_window: Option<i64>,
    pub window: String,
    #[serde(default)]
    pub window_words: Option<WindowWords>,
    #[serde(default)]
    pub extra_room: Option<Amount>,
    #[serde(default)]
    pub base_limit: Option<Amount>,
    #[serde(default, deserialize_with = "time")]
    pub room_window_end: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(bound(deserialize = "V: Deserialize<'de>"))]
pub struct Total<V> {
    pub limit: V,
    pub used: Option<V>,
    pub window: String,
    #[serde(default)]
    pub window_words: Option<WindowWords>,
    #[serde(default)]
    pub extra_room: Option<V>,
    #[serde(default)]
    pub base_limit: Option<V>,
    #[serde(default, deserialize_with = "time")]
    pub room_window_end: Option<DateTime<Utc>>,
}

/// One KIFF Card on its own (`CardView`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CardView {
    pub id: String,
    pub title: String,
    pub environment: String,
    pub live: bool,
    pub active: bool,
    #[serde(deserialize_with = "time")]
    pub expires: Option<DateTime<Utc>>,
    pub rules: Vec<Rule>,
    pub total: Option<Total<Amount>>,
    pub calls: Option<Total<i64>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CardSummary {
    pub ids: Vec<String>,
    pub active: bool,
    #[serde(deserialize_with = "time")]
    pub expires: Option<DateTime<Utc>>,
    pub tools: Vec<String>,
    pub rules: Vec<Rule>,
    pub used: BTreeMap<String, Amount>,
    pub usage_unknown: Vec<String>,
    #[serde(default)]
    pub cards: Option<Vec<CardView>>,
}

/// A taf's health (`Health`): never fails the read it's on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Health {
    pub level: String,
    pub lines: Vec<String>,
}

impl Health {
    fn read(v: &Value) -> Option<Health> {
        if v.is_null() {
            return None;
        }
        let Some(o) = v.as_object() else {
            return Some(Health {
                level: "healthy".into(),
                lines: vec![],
            });
        };
        let level = o
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("healthy")
            .to_string();
        let lines = o
            .get("lines")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(Value::as_str)
                    .filter(|l| !l.trim().is_empty())
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Some(Health { level, lines })
    }

    /// Something to show: a level other than healthy, with a line to say why.
    pub fn needs_attention(&self) -> bool {
        self.level != "healthy" && !self.lines.is_empty()
    }

    pub fn failing(&self) -> bool {
        self.level == "failing"
    }
}

/// Where its work stands (`TafWork.Summary`): never fails the read it's on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Work {
    pub status: String,
    pub line: String,
    pub attention: bool,
}

impl Work {
    fn read(v: &Value) -> Option<Work> {
        if v.is_null() {
            return None;
        }
        let Some(o) = v.as_object() else {
            return Some(Work {
                status: String::new(),
                line: String::new(),
                attention: false,
            });
        };
        Some(Work {
            status: o
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("idle")
                .to_string(),
            line: o
                .get("line")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            attention: o.get("attention").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}

/// A taf as tafs cloud lists it (`HomeTaf`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HomeTaf {
    /// tafs cloud's id; none for an agent tafs doesn't run.
    pub id: Option<String>,
    pub name: String,
    pub job: Option<String>,
    #[serde(default)]
    pub unscheduled: Option<bool>,
    #[serde(default)]
    pub job_summary: Option<String>,
    pub kiff_agent: String,
    pub status: Option<String>,
    #[serde(deserialize_with = "time")]
    pub next_run: Option<DateTime<Utc>>,
    #[serde(deserialize_with = "time")]
    pub last_run: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_change: Value,
    pub color: i64,
    /// working, paused, waiting, halted, idle (an agent tafs doesn't run) or stopped.
    pub state: String,
    /// Its calls held for the owner.
    pub waiting: i64,
    pub card: CardSummary,
    #[serde(default)]
    pub health: Value,
    #[serde(default)]
    pub work: Value,
    #[serde(default)]
    pub next_run_words: Option<String>,
    #[serde(default)]
    pub finish: Value,
}

/// tafs cloud's count of what waits, with its words (`NeedsYou`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NeedsYou {
    pub count: i64,
    pub words: String,
}

/// Home (`TafsCloud.Home`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Home {
    pub tafs: Vec<HomeTaf>,
    pub others: Vec<HomeTaf>,
    pub removing: Vec<Value>,
    #[serde(default)]
    pub needs_you: Option<NeedsYou>,
    #[serde(default)]
    pub headline: Value,
    #[serde(default)]
    pub setting_up: Option<Vec<Value>>,
}

impl Home {
    /// Home's answer, read as the iPad reads it; `None` when it can't be, and the screen stays
    /// as it was.
    pub fn read(body: &[u8]) -> Option<Home> {
        serde_json::from_slice(body).ok()
    }

    /// `headlineWords`: tafs cloud's words for Home's line; none when it sent none the app can draw.
    pub fn headline_words(&self) -> Option<String> {
        let w = self.headline.get("words")?.as_str()?.trim();
        (!w.is_empty()).then(|| w.to_string())
    }
}

/// A taf as the screen shows it (`KIFFMapping.taf`'s `Taf`, the parts drawn so far).
#[derive(Debug, Clone, PartialEq)]
pub struct Taf {
    pub key: String,
    /// Its holder in KIFF (`kiff_agent`): what KIFF's held calls name.
    pub agent: String,
    pub name: String,
    pub color_index: i64,
    /// What it's doing, in the app's words or tafs cloud's (`taf.now`).
    pub now: String,
    /// What went wrong last, before its next run (`healthNow`), and whether it's failing.
    pub health_now: Option<String>,
    pub failing: bool,
    /// The Card in front (`shownCards.first`).
    pub card: Option<Card>,
    pub last_run: Option<DateTime<Utc>>,
    pub last_change: Option<DateTime<Utc>>,
    /// tafs cloud runs it.
    pub cloud: bool,
}

fn window(w: &WordBook, raw: &str, words: Option<&WindowWords>) -> Result<Window, WordsError> {
    Window::new(
        w,
        raw,
        words.map(|x| x.each.as_str()),
        words.map(|x| x.now.as_str()),
    )
}

/// `KIFFMapping.taf`: its state in the app's words, its Card's lines and use as tafs cloud read
/// them from KIFF. Nothing is worked out here.
pub fn taf(w: &WordBook, h: &HomeTaf) -> Result<Taf, WordsError> {
    let now = match h.state.as_str() {
        "waiting" => w.line("taf.now.waiting_on_you")?,
        "paused" => w.line("taf.state.paused")?,
        "halted" => w.line("taf.now.halted")?,
        "working" if h.unscheduled == Some(true) => w.line("taf.now.works_from_chat")?,
        // tafs cloud's words when it sent them ("Next run Mon 08:00"): the app's own only from a
        // tafs cloud that sends none.
        "working" => match (&h.next_run_words, &h.next_run) {
            (Some(words), _) => words.clone(),
            (None, Some(t)) => w.get(
                "taf.now.next_run",
                &[("time", &t.with_timezone(&Local).format("%H:%M").to_string())],
            )?,
            (None, None) => w.line("taf.now.running")?,
        },
        "idle" => w.line("taf.now.acting_under_kiff")?,
        _ => w.line("taf.now.card_not_active")?,
    };
    let health = Health::read(&h.health);
    let work = Work::read(&h.work);
    // What went wrong last: tafs cloud flags it, and an idle taf that isn't healthy says it too.
    let health_now = work.as_ref().and_then(|work| {
        let says = work.attention
            || (work.status == "idle" && health.as_ref().is_some_and(Health::needs_attention));
        (says && !work.line.trim().is_empty()).then(|| work.line.clone())
    });
    // Its setup stopped partway: tafs cloud's words in place of what it's doing.
    let finish_words = h.finish.as_object().and_then(|f| {
        let text = |k: &str| {
            f.get(k)
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
        };
        (!text("taf").is_empty() && !text("session").is_empty() && !text("words").is_empty())
            .then(|| text("words").to_string())
    });
    Ok(Taf {
        key: h
            .id
            .clone()
            .unwrap_or_else(|| format!("agent:{}", h.kiff_agent)),
        agent: h.kiff_agent.clone(),
        name: h.name.clone(),
        color_index: h.color,
        now: finish_words.unwrap_or(now),
        health_now,
        failing: health.as_ref().is_some_and(Health::failing),
        card: shown_card(w, &h.card)?,
        last_run: h.last_run,
        last_change: lenient_time(&h.last_change),
        cloud: h.id.is_some(),
    })
}

/// `shownCards.first`: KIFF's first Card, in tafs cloud's order; else the one merged from the
/// taf's lines (`TafCard.merged`), from an older tafs cloud.
fn shown_card(w: &WordBook, c: &CardSummary) -> Result<Option<Card>, WordsError> {
    if let Some(first) = c.cards.as_ref().and_then(|cards| cards.first()) {
        // A limit whose scale can't be read is no figure; a use that can't be, use unavailable.
        let total = match first
            .total
            .as_ref()
            .and_then(|t| t.limit.money().map(|limit| (t, limit)))
        {
            Some((t, limit)) => {
                let mut m = MoneyTotal::new(
                    limit,
                    t.used.as_ref().and_then(Amount::money),
                    window(w, &t.window, t.window_words.as_ref())?,
                );
                m.extra = t.extra_room.as_ref().and_then(Amount::money);
                m.base = t.base_limit.as_ref().and_then(Amount::money);
                m.room_ends = t.room_window_end.map(ends_words);
                Some(m)
            }
            None => None,
        };
        let calls = match &first.calls {
            Some(t) => Some(CallsTotal {
                limit: t.limit,
                used: t.used,
                window: window(w, &t.window, t.window_words.as_ref())?,
            }),
            None => None,
        };
        return Ok(Some(Card { total, calls }));
    }
    // Merged: the first rule with a money total, else the first with a count.
    if let Some(rule) = c.rules.iter().find(|r| r.per_window.is_some()) {
        let Some(limit) = rule.per_window.as_ref().and_then(Amount::money) else {
            return Ok(Some(Card::default()));
        };
        let used = match c.used.get(&rule.action) {
            Some(a) => a.money(),
            None if c.usage_unknown.contains(&rule.action) => None,
            None => Some(Money {
                minor: 0,
                ..limit.clone()
            }),
        };
        let window = window(w, &rule.window, rule.window_words.as_ref())?;
        return Ok(Some(Card {
            total: Some(MoneyTotal::new(limit, used, window)),
            calls: None,
        }));
    }
    if let Some(rule) = c.rules.iter().find(|r| r.count_per_window.is_some()) {
        // No count is none used, unless KIFF couldn't read it.
        let used = if c.usage_unknown.contains(&rule.action) {
            None
        } else {
            Some(0)
        };
        let window = window(w, &rule.window, rule.window_words.as_ref())?;
        return Ok(Some(Card {
            total: None,
            calls: Some(CallsTotal {
                limit: rule.count_per_window.unwrap(),
                used,
                window,
            }),
        }));
    }
    Ok(Some(Card::default()))
}

/// `endsWords`: the time alone today, else its date with it, here (until the account's zone
/// is read).
fn ends_words(t: DateTime<Utc>) -> String {
    let local = t.with_timezone(&Local);
    if local.date_naive() == Local::now().date_naive() {
        local.format("%H:%M").to_string()
    } else {
        local.format("%Y-%m-%d %H:%M").to_string()
    }
}

/// The sidebar's row for a taf (`tafRow`): what went wrong in place of what it's doing.
pub fn row(t: &Taf) -> TafRow {
    TafRow {
        key: t.key.clone(),
        name: t.name.clone(),
        initial: initial(&t.name),
        color_index: t.color_index,
        now: t.health_now.clone().unwrap_or_else(|| t.now.clone()),
        badge: None,
        tone: match (&t.health_now, t.failing) {
            (None, _) => Tone::Dim,
            (Some(_), true) => Tone::No,
            (Some(_), false) => Tone::Amber,
        },
    }
}

/// `relevantNow` for a taf from Home: a call of its waits, or it ran or changed in the last 24
/// hours. Steps and knocks come with their reads.
pub fn relevant(t: &Taf, held: bool, now: DateTime<Local>) -> bool {
    relevant_now(
        held,
        false,
        false,
        t.last_run.map(|x| x.with_timezone(&Local)),
        t.last_change.map(|x| x.with_timezone(&Local)),
        now,
    )
}

/// The Overview's line for a taf from Home.
pub fn overview_taf(t: &Taf) -> OverviewTaf {
    OverviewTaf {
        key: t.key.clone(),
        name: t.name.clone(),
        initial: initial(&t.name),
        color_index: t.color_index,
        // The Overview keeps what it's doing; the sidebar says what went wrong.
        now: t.now.clone(),
        card: t
            .card
            .clone()
            .filter(|c| c.total.is_some() || c.calls.is_some()),
        cloud: t.cloud.then(|| CloudRun {
            report: None,
            error: None,
            last_run: t
                .last_run
                .map(|x| x.with_timezone(&Local).format("%H:%M").to_string()),
        }),
        used_by: String::new(),
    }
}
