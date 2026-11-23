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
    /// What of its use is from calls the tool refused, in KIFF's words.
    #[serde(default)]
    pub pending_text: Option<String>,
    /// Why its amount has no currency, in KIFF's words.
    #[serde(default)]
    pub unit_words: Option<String>,
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
    /// How this is counted, each tool's lines as the owner approved them; none from KIFF.
    #[serde(default)]
    pub counted: Option<Vec<CountedTool>>,
    /// Its totals for each item apart, in tafs cloud's words.
    #[serde(default)]
    pub each_item: Option<Vec<String>>,
}

/// One tool of a Card, and how KIFF counts it (`Counted`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CountedTool {
    pub tool: String,
    pub title: String,
    pub lines: Vec<String>,
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
    /// The fixes tafs cloud decides for its lines (`Health.Fix`).
    pub fixes: Vec<Fix>,
}

/// A way to fix what one line says (`Health.Fix`): Coach opened with `say` in its box
/// (`coach`), or Plan & usage (`credits`). `line` is the line's index; -1 when it isn't one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Fix {
    pub line: i64,
    pub kind: String,
    pub words: String,
    pub say: Option<String>,
}

impl Fix {
    fn read(v: &Value) -> Option<Fix> {
        let o = v.as_object()?;
        Some(Fix {
            line: o.get("line").and_then(Value::as_i64).unwrap_or(-1),
            kind: o
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            words: o
                .get("words")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            say: o.get("say").and_then(Value::as_str).map(str::to_string),
        })
    }
}

impl Health {
    /// The fix kinds the app can act on (`knownKinds`).
    pub const KNOWN_KINDS: [&'static str; 2] = ["coach", "credits"];

    /// The fixes for one line, of the kinds the app can act on, in tafs cloud's order
    /// (`fixes(for:)`).
    pub fn fixes_for(&self, line: usize) -> Vec<&Fix> {
        self.fixes
            .iter()
            .filter(|f| {
                f.line == line as i64
                    && Self::KNOWN_KINDS.contains(&f.kind.as_str())
                    && !f.words.is_empty()
                    && (f.kind != "coach" || !f.say.as_deref().unwrap_or("").is_empty())
            })
            .collect()
    }

    /// Whether a work's line is this health's first reason (`repeats(workLine:)`), so the page
    /// says it once.
    pub fn repeats(&self, work_line: &str) -> bool {
        let line = work_line.trim();
        match self.lines.first() {
            Some(first) if self.needs_attention() && !line.is_empty() => first.starts_with(line),
            _ => false,
        }
    }

    pub fn read(v: &Value) -> Option<Health> {
        if v.is_null() {
            return None;
        }
        let Some(o) = v.as_object() else {
            return Some(Health {
                level: "healthy".into(),
                ..Default::default()
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
        let fixes = o
            .get("fixes")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Fix::read).collect())
            .unwrap_or_default();
        Some(Health {
            level,
            lines,
            fixes,
        })
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
    /// Whether it matters now, by tafs cloud's rule (`relevant_now`): the sidebar's lists read
    /// it. None from a tafs cloud that doesn't send it.
    #[serde(default)]
    pub relevant_now: Option<bool>,
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
    /// The zone its times are in, in tafs cloud's words ("Times in UTC"), and, when its schedule
    /// runs in another, that (`timeZoneWords`, `scheduleTimeZoneWords`).
    #[serde(default)]
    pub time_zone_words: Option<String>,
    #[serde(default)]
    pub schedule_time_zone_words: Option<String>,
    #[serde(default)]
    pub finish: Value,
    /// Its doors, for its avatar's rim and Needs you's knocks; absent when it has none.
    #[serde(default)]
    pub doors: Option<Vec<DoorLight>>,
    /// The rim round its avatar while a door is open (`avatar_rim`, `rim::read`); another shape
    /// is none.
    #[serde(default)]
    pub avatar_rim: Value,
    /// Its open trial (`trial`, `OpenTrial`), for whoever manages it; another shape is none.
    #[serde(default)]
    pub trial: Value,
    /// Where tafs cloud delivers its results itself (`door_reply`, `app`, `owner_email`), live.
    #[serde(default)]
    pub results: Option<Vec<Value>>,
    /// Its labels by id (docs/labels.md), only on a taf that has any; another shape is none
    /// (`LabelIDs`).
    #[serde(default)]
    pub labels: Value,
}

impl HomeTaf {
    /// Where Finish setting up opens, for an agent whose setup stopped partway (`finish`).
    pub fn finish_chat(&self) -> Option<FinishChat> {
        FinishChat::read(&self.finish)
    }

    /// Its key in the app: tafs cloud's id, else its KIFF agent's.
    pub fn key(&self) -> String {
        self.id
            .clone()
            .unwrap_or_else(|| format!("agent:{}", self.kiff_agent))
    }
}

impl Home {
    /// Knocks waiting at each taf's doors, by its key (`knocks(_:)`), Home's tafs and then the
    /// others'.
    pub fn knocks(&self) -> std::collections::BTreeMap<String, i64> {
        self.tafs
            .iter()
            .chain(&self.others)
            .map(|t| (t.key(), t.doors.iter().flatten().map(|d| d.knocks).sum()))
            .collect()
    }
}

/// tafs cloud's words for Discard (`SettingUp.Discard`), each trimmed; one the app can draw has a
/// button and its yes (`usable`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiscardWords {
    pub button: String,
    pub title: String,
    pub text: String,
    pub yes: String,
    pub no: String,
}

/// Where Finish setting up opens (`HomeTaf.Finish`): Coach's taf and the chat where the setup
/// stopped, each trimmed; one the app can open has both and its words (`usable`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct FinishChat {
    pub taf: String,
    pub session: String,
    pub words: String,
}

impl FinishChat {
    /// Read as `Finish.init(from:)`: anything not an object, or a field missing or not a string,
    /// reads as empty.
    pub fn read(v: &Value) -> Option<FinishChat> {
        let f = v.as_object()?;
        let text = |k: &str| {
            f.get(k)
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .to_string()
        };
        Some(FinishChat {
            taf: text("taf"),
            session: text("session"),
            words: text("words"),
        })
    }

    /// `usable`: a chat and its words.
    pub fn usable(&self) -> bool {
        !self.taf.is_empty() && !self.session.is_empty() && !self.words.is_empty()
    }

    /// `canFinishSetup`: usable, for someone who may use Coach, signed in, and only in Coach's
    /// own chat (`coach`, Coach's id at tafs cloud, once read).
    pub fn can_finish(&self, kiff: bool, may_use_coach: bool, coach: Option<&str>) -> bool {
        kiff && may_use_coach && self.usable() && coach == Some(self.taf.as_str())
    }
}

/// A taf whose setup stopped partway (`TafsCloud.Home.SettingUp`), read as the iPad's own
/// `init(from:)` reads it: whatever is missing or of the wrong kind reads as empty, so an item never
/// fails Home, and one without a name is skipped.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SettingUp {
    pub name: String,
    pub kiff_agent: String,
    pub color: i64,
    /// Finish setting up's words (`finish.words`), when tafs cloud sends a chat to open it in
    /// (`Finish.usable`: its taf, session and words).
    pub finish: Option<String>,
    /// Discard, for the person whose setup it was.
    pub discard: Option<DiscardWords>,
    /// Where Finish setting up opens, as sent (`finish`); none when it isn't an object.
    pub chat: Option<FinishChat>,
}

impl SettingUp {
    fn read(v: &Value) -> SettingUp {
        let text = |o: &Value, k: &str| -> String {
            o.get(k)
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string()
        };
        let chat = v.get("finish").and_then(FinishChat::read);
        let finish = chat
            .as_ref()
            .filter(|f| f.usable())
            .map(|f| f.words.clone());
        let discard = v
            .get("discard")
            .and_then(|d| d.get("words"))
            .filter(|w| w.is_object())
            .map(|w| DiscardWords {
                button: text(w, "button"),
                title: text(w, "title"),
                text: text(w, "text"),
                yes: text(w, "yes"),
                no: text(w, "no"),
            })
            .filter(|d| !d.button.is_empty() && !d.yes.is_empty());
        SettingUp {
            // The name and agent untrimmed, as the iPad keeps them.
            name: v
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            kiff_agent: v
                .get("kiff_agent")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            color: v.get("color").and_then(Value::as_i64).unwrap_or(0),
            finish,
            discard,
            chat,
        }
    }

    /// Its key in the app, by its KIFF agent, else its name (`holderID`).
    pub fn key(&self) -> String {
        if self.kiff_agent.is_empty() {
            format!("setting-up:{}", self.name)
        } else {
            format!("agent:{}", self.kiff_agent)
        }
    }

    /// Its row among the tafs (`tafRow`): its name and colour, no state, and Finish setting up's
    /// words in the amber when tafs cloud sends them.
    pub fn row(&self) -> TafRow {
        TafRow {
            key: self.key(),
            name: self.name.clone(),
            initial: initial(&self.name),
            color_index: self.color,
            now: self.finish.clone().unwrap_or_default(),
            badge: None,
            tone: Tone::Amber,
            rim: None,
            trial: None,
        }
    }
}

impl Home {
    /// The tafs being set up (`settingUp`), those with a name; none when Home sends none.
    pub fn setting_up(&self) -> Vec<SettingUp> {
        self.setting_up
            .iter()
            .flatten()
            .map(SettingUp::read)
            .filter(|s| !s.name.is_empty())
            .collect()
    }

    /// `otherAgents`: with `setting_up` sent, `others` only repeats the tafs being set up for older
    /// apps and isn't drawn; without it, every one of them.
    pub fn other_agents(&self) -> &[HomeTaf] {
        if self.setting_up.is_some() {
            &[]
        } else {
            &self.others
        }
    }
}

/// One of a taf's doors (`DoorLight`): lit while open, amber while a knock waits, grey while it
/// rests. What's missing or the wrong kind reads as nothing to show, so a door never stops Home;
/// a door that isn't an object does, as the iPad's keyed decoder does.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DoorLight {
    pub app: String,
    pub open: bool,
    pub resting: bool,
    pub knocks: i64,
}

impl<'de> Deserialize<'de> for DoorLight {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Map::<String, Value>::deserialize(d)?;
        Ok(DoorLight {
            app: v
                .get("app")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            open: v.get("open").and_then(Value::as_bool).unwrap_or(false),
            resting: v.get("resting").and_then(Value::as_bool).unwrap_or(false),
            // Swift's `Int`: a whole number only.
            knocks: v.get("knocks").and_then(Value::as_i64).unwrap_or(0),
        })
    }
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
    /// The doors tafs cloud offers here (`doors_offered`); absent when none, and no taf's
    /// knocks are read.
    #[serde(default)]
    pub doors_offered: Option<Vec<String>>,
    /// Card unit requests (`card_unit_exceptions`).
    #[serde(default)]
    pub card_unit_exceptions: Option<Vec<crate::waits::CardUnitException>>,
    /// Steps its tafs handed the person (`your_steps`).
    #[serde(default)]
    pub your_steps: Option<Vec<crate::waits::YourStep>>,
    /// Coach's chats waiting on the person (`coach_waiting`), oldest first.
    #[serde(default)]
    pub coach_waiting: Option<Vec<crate::waits::CoachWaiting>>,
    /// "Set all aside" (`coach_waiting_clear`).
    #[serde(default)]
    pub coach_waiting_clear: Option<crate::waits::CoachWaitingClear>,
    /// The account's labels, in the order to show them (docs/labels.md); absent with no labelled
    /// taf, and no chips are drawn. Read leniently (`LabelList`): never fails Home.
    #[serde(default)]
    pub labels: Value,
    /// The words for the labels' first chip (`labels_words.all`).
    #[serde(default)]
    pub labels_words: Value,
}

impl Home {
    /// Home's answer, read as the iPad reads it; `None` when it can't be, and the screen stays
    /// as it was.
    pub fn read(body: &[u8]) -> Option<Home> {
        serde_json::from_slice(body).ok()
    }

    /// `homeLabels`: the account's labels, in order; none with no labelled taf.
    pub fn labels(&self) -> Vec<crate::brief::Label> {
        crate::brief::label_list(&self.labels).unwrap_or_default()
    }

    /// `homeLabelsAll`: Home's word for the first chip, when it sent one.
    pub fn labels_all(&self) -> Option<String> {
        crate::brief::labels_all(&self.labels_words)
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
    /// tafs cloud's `relevant_now`, when it sent it.
    pub relevant_now: Option<bool>,
    /// tafs cloud runs it.
    pub cloud: bool,
    /// Its state (`TafState`), and whether tafs cloud flags what went wrong (`attention`).
    pub state: crate::taf_page::TafState,
    pub attention: bool,
    /// The Card in front's title, environment, whether it's live and active; how many Cards; and
    /// whether the taf's Card is revoked.
    pub card_title: String,
    pub card_environment: String,
    pub card_live: bool,
    pub card_active: bool,
    pub card_count: usize,
    pub revoked: bool,
    /// Its Cards' ids at KIFF (`kiffCardIDs`), which Revoke revokes.
    pub card_ids: Vec<String>,
    /// Knocks waiting at its doors (`knocks(_:)`: their sum), for Needs you's row.
    pub knocks: i64,
    /// A door with a knock waiting (`doors.contains { $0.knocks > 0 }`), for `relevantNow`.
    pub knocked: bool,
    /// Its labels by id (`CloudTaf.labels`).
    pub labels: Vec<String>,
    /// The rim round its avatar while a door is open (`DoorRim`).
    pub rim: Option<crate::rim::Rim>,
    /// Its open trial, as Home sends it (`setTrial`): its own row under the taf.
    pub trial: Option<crate::versions::OpenTrial>,
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
                &[("time", &crate::account_time::short_time(t))],
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
    // The Card in front: KIFF's first, else the one merged from the taf's lines, named after it.
    let front = h.card.cards.as_ref().and_then(|c| c.first());
    Ok(Taf {
        key: h.key(),
        agent: h.kiff_agent.clone(),
        name: h.name.clone(),
        color_index: h.color,
        now: finish_words.unwrap_or(now),
        health_now,
        failing: health.as_ref().is_some_and(Health::failing),
        card: shown_card(w, &h.card)?,
        last_run: h.last_run,
        last_change: lenient_time(&h.last_change),
        relevant_now: h.relevant_now,
        knocks: h.doors.iter().flatten().map(|d| d.knocks).sum(),
        knocked: h.doors.iter().flatten().any(|d| d.knocks > 0),
        rim: crate::rim::read(&h.avatar_rim),
        trial: serde_json::from_value(h.trial.clone()).ok(),
        cloud: h.id.is_some(),
        state: crate::taf_page::TafState::from_cloud(&h.state, h.unscheduled == Some(true)),
        attention: Work::read(&h.work).is_some_and(|w| w.attention && !w.line.trim().is_empty()),
        card_title: front
            .map(|c| c.title.clone())
            .unwrap_or_else(|| h.name.clone()),
        card_environment: front.map(|c| c.environment.clone()).unwrap_or_default(),
        card_live: front.is_none_or(|c| c.live),
        card_active: front.map_or(h.card.active, |c| c.active),
        card_count: h
            .card
            .cards
            .as_ref()
            .filter(|c| !c.is_empty())
            .map_or(1, Vec::len),
        revoked: !h.card.active,
        card_ids: h.card.ids.clone(),
        // Only a tafs cloud taf carries labels (`cloudTaf(for:)?.labels`).
        labels: if h.id.is_some() {
            crate::brief::label_ids(&h.labels)
        } else {
            vec![]
        },
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

/// `endsWords`: the time alone today, else its date with it, in the account's zone.
fn ends_words(t: DateTime<Utc>) -> String {
    use crate::account_time::{day, local, short_time};
    if day(&t) == day(&Utc::now()) {
        short_time(&t)
    } else {
        local(&t).format("%Y-%m-%d %H:%M").to_string()
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
        rim: t.rim,
        trial: None,
        tone: match (&t.health_now, t.failing) {
            (None, _) => Tone::Dim,
            (Some(_), true) => Tone::No,
            (Some(_), false) => Tone::Amber,
        },
    }
}

/// An open trial's row under its taf (`trialTaf`): "Refunds · v2", the trial's badge (tafs
/// cloud's label, else "Trial"), and "Trying version 2 · ends 29 Nov if unused" (tafs cloud's
/// title where it sent one). None without an open trial.
pub fn trial_row(w: &WordBook, t: &Taf) -> Result<Option<TafRow>, WordsError> {
    let Some(trial) = &t.trial else {
        return Ok(None);
    };
    let n = trial.n.to_string();
    let title = match &trial.title {
        Some(title) if !title.trim().is_empty() => title.clone(),
        _ => w.get("version.trial.trying", &[("n", &n)])?,
    };
    let ends = match trial.ends {
        Some(at) => Some(w.get(
            "version.trial.ends_if_unused",
            &[(
                "date",
                &crate::locale::short_date(w.language(), crate::account_time::day(&at)),
            )],
        )?),
        None => None,
    };
    let badge = match &trial.label {
        Some(l) if !l.trim().is_empty() => l.clone(),
        _ => w.line("version.trial.badge")?,
    };
    let mut r = row(t);
    r.name = w.get("version.trial.chat_name", &[("taf", &t.name), ("n", &n)])?;
    r.now = [Some(title), ends]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ");
    r.badge = Some(badge);
    r.tone = Tone::Dim;
    r.rim = None;
    r.trial = None;
    Ok(Some(r))
}

/// Whether a taf from Home is listed as relevant now: tafs cloud's `relevant_now` whenever it sent
/// it (the apps never work the rule out themselves); only from a tafs cloud that doesn't, the
/// old rule here (`relevantNow`): a call of its waits, a knock at its doors, or it ran or changed
/// in the last 24 hours.
pub fn relevant(t: &Taf, held: bool, now: DateTime<Local>) -> bool {
    if let Some(flag) = t.relevant_now {
        return flag;
    }
    relevant_now(
        held,
        false,
        t.knocked,
        t.last_run.map(|x| x.with_timezone(&Local)),
        t.last_change.map(|x| x.with_timezone(&Local)),
        now,
    )
}

/// The Overview's line for a taf from Home.
pub fn overview_taf(t: &Taf) -> OverviewTaf {
    OverviewTaf {
        rim: t.rim,
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
            last_run: t.last_run.map(|x| crate::account_time::short_time(&x)),
        }),
        used_by: String::new(),
        labels: t.labels.clone(),
    }
}

/// A taf's Cards whole (`shownCards`): KIFF's, each on its own, in tafs cloud's order; else the
/// one merged from the taf's lines, named after it.
pub fn full_cards(
    w: &WordBook,
    h: &HomeTaf,
    language: &str,
) -> Result<Vec<crate::cards::FullCard>, WordsError> {
    use crate::cards::{FullCard, Rule as CardRule};
    let rule = |r: &Rule| -> Result<CardRule, WordsError> {
        let mut out = CardRule::new(
            r.label.clone(),
            window(w, &r.window, r.window_words.as_ref())?,
        );
        out.per_action = r.per_action.as_ref().and_then(Amount::money);
        out.per_window = r.per_window.as_ref().and_then(Amount::money);
        out.count_per_window = r.count_per_window;
        out.holds = r.outside != "refuse";
        out.extra_room = r.extra_room.as_ref().and_then(Amount::money);
        out.base_limit = r.base_limit.as_ref().and_then(Amount::money);
        out.room_ends = r.room_window_end.map(ends_words);
        Ok(out)
    };
    let date =
        |t: Option<DateTime<Utc>>| t.map(|t| long_date(language, crate::account_time::day(&t)));
    if let Some(cards) = h.card.cards.as_ref().filter(|c| !c.is_empty()) {
        return cards
            .iter()
            .map(|c| {
                let one = CardSummary {
                    cards: Some(vec![c.clone()]),
                    ..h.card.clone()
                };
                Ok(FullCard {
                    extra: crate::cards::CardExtra {
                        each_item: c
                            .each_item
                            .iter()
                            .flatten()
                            .filter(|l| !l.trim().is_empty())
                            .cloned()
                            .collect(),
                        counted: c
                            .counted
                            .iter()
                            .flatten()
                            .map(|t| crate::cards::Counted {
                                tool: t.tool.clone(),
                                title: t.title.clone(),
                                lines: t.lines.clone(),
                            })
                            .collect(),
                        pending: [
                            c.total.as_ref().and_then(|t| t.pending_text.clone()),
                            c.calls.as_ref().and_then(|t| t.pending_text.clone()),
                        ]
                        .into_iter()
                        .flatten()
                        .collect(),
                        unit_words: c.total.as_ref().and_then(|t| t.unit_words.clone()),
                    },
                    id: c.id.clone(),
                    title: c.title.clone(),
                    environment: c.environment.clone(),
                    live: c.live,
                    active: c.active,
                    expires: date(c.expires),
                    rules: c.rules.iter().map(rule).collect::<Result<_, _>>()?,
                    totals: shown_card(w, &one)?.unwrap_or_default(),
                })
            })
            .collect();
    }
    Ok(vec![FullCard {
        extra: Default::default(),
        id: "merged".into(),
        title: h.name.clone(),
        environment: String::new(),
        live: true,
        active: h.card.active,
        expires: date(h.card.expires),
        rules: h.card.rules.iter().map(rule).collect::<Result<_, _>>()?,
        totals: shown_card(
            w,
            &CardSummary {
                cards: None,
                ..h.card.clone()
            },
        )?
        .unwrap_or_default(),
    }])
}

/// `shown(date: .abbreviated, time: .omitted)` in a shipped language: "Dec 31, 2026".
pub fn long_date(language: &str, day: chrono::NaiveDate) -> String {
    use chrono::Datelike;
    let short = crate::record::short_date(language, day);
    match language {
        "en" => format!("{short}, {}", day.year()),
        _ => format!("{short} {}", day.year()),
    }
}

#[cfg(test)]
mod finish_tests {
    use super::*;

    #[test]
    fn finish_setting_up_opens_only_coach_s_own_chat() {
        let f = FinishChat::read(&serde_json::json!({
            "taf": " setup-coach ", "session": "chat_1", "words": "Needs you to finish setting up"
        }))
        .unwrap();
        assert_eq!(
            (f.taf.as_str(), f.session.as_str()),
            ("setup-coach", "chat_1")
        );
        assert!(f.usable());
        assert!(f.can_finish(true, true, Some("setup-coach")));
        // Another Coach's chat, Coach not read, the sample, or someone who may not use Coach.
        assert!(!f.can_finish(true, true, Some("other")));
        assert!(!f.can_finish(true, true, None));
        assert!(!f.can_finish(false, true, Some("setup-coach")));
        assert!(!f.can_finish(true, false, Some("setup-coach")));
        // Lenient as the iPad's: a field of another kind reads as empty, and isn't usable.
        let half =
            FinishChat::read(&serde_json::json!({"taf": 3, "session": "chat_1", "words": "w"}))
                .unwrap();
        assert_eq!(half.taf, "");
        assert!(!half.usable());
        assert_eq!(FinishChat::read(&serde_json::json!("chat_1")), None);
        // A setting-up item keeps where it opens, and its words only when usable.
        let s = SettingUp::read(&serde_json::json!({
            "name": "Invoice chaser", "finish": {"taf": "c", "session": "", "words": "Finish"}
        }));
        assert_eq!(s.finish, None);
        assert_eq!(s.chat.map(|c| c.taf), Some("c".to_string()));
    }
}
