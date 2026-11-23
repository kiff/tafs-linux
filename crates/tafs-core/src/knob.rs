//! A taf's dial as tafs cloud sends it (`TafsCloud+Knob.swift`, `GET /v1/tafs/{id}/knob`): its
//! purpose, When (`wakes`), What and Where (`results`) around it, today's work, a change waiting to
//! go live, and the live `setting` a change copies. Every label, line and word is tafs cloud's; the
//! app composes none. Read as strictly as the iPad's decoders: a field there in the wrong shape and
//! the dial isn't read; one missing takes the iPad's default.
//!
//! Changes are the session's at tafs cloud: a new dial (`PUT …/knob`, the taf's next version, live
//! only once switched to), tafs' own places for results (`PUT …/results`, live at once), a
//! follow-up removed, every start paused or resumed.

use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use crate::words::{WordBook, WordsError};

/// `decodeIfPresent(…) ?? default`: missing or null takes the default; another shape fails.
fn or_default<'de, D, T>(d: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(d)?.unwrap_or_default())
}

fn on_by_default<'de, D: Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(Option::<bool>::deserialize(d)?.unwrap_or(true))
}

/// `Knob.Does`: the purpose in the face.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Does {
    #[serde(default, deserialize_with = "or_default")]
    pub label: String,
    #[serde(default, deserialize_with = "or_default")]
    pub line: String,
}

/// `Item.Approval`: an app event's subscription at planque, needed or approved.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Approval {
    pub state: String,
    pub grant: String,
    pub event: String,
    pub config: Map<String, Value>,
    #[serde(default)]
    pub subscription: Option<String>,
}

/// One of a stop's sizes, and the whole dial it makes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Choice {
    pub label: String,
    pub setting: Setting,
}

/// `Item.Size`: − and + through the choices tafs cloud offers, `index` the one in force.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Size {
    pub label: String,
    pub index: i64,
    pub choices: Vec<Choice>,
}

impl Size {
    /// `choice(moving:)`: the next choice down (-1) or up (1), if there is one.
    pub fn choice(&self, direction: i64) -> Option<&Choice> {
        let n = self.choices.len() as i64;
        if !(0..n).contains(&self.index) || !(direction == -1 || direction == 1) {
            return None;
        }
        let to = self.index + direction;
        (0..n).contains(&to).then(|| &self.choices[to as usize])
    }

    /// `ResultsGoToView.toggled`: a place's other choice, off to on or on to off, when it has
    /// exactly two.
    pub fn toggled(&self) -> Option<&Choice> {
        if self.choices.len() != 2 || !(0..2).contains(&self.index) {
            return None;
        }
        Some(&self.choices[1 - self.index as usize])
    }

    /// `SizeStepper`'s label: "Every 4 hours · Every 4 hours", or the label alone.
    pub fn stepper_label(&self) -> String {
        match usize::try_from(self.index)
            .ok()
            .and_then(|i| self.choices.get(i))
        {
            Some(c) => format!("{} · {}", self.label, c.label),
            None => self.label.clone(),
        }
    }
}

/// One stop: a trigger on When, a step on What, or a place on Where (`Knob.Item`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Item {
    pub id: String,
    /// run_now, door, schedule, app_event, app_check, follow_up or step.
    #[serde(default)]
    pub kind: Option<String>,
    /// door_reply, app, owner_email or a connected app's action.
    #[serde(default)]
    pub place: Option<String>,
    #[serde(default, deserialize_with = "or_default")]
    pub label: String,
    #[serde(default, deserialize_with = "or_default")]
    pub words: String,
    /// A stop that doesn't say is on.
    #[serde(default = "yes", deserialize_with = "on_by_default")]
    pub on: bool,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub tap: Option<String>,
    #[serde(default)]
    pub expires: Option<String>,
    #[serde(default)]
    pub door: Option<String>,
    /// The trigger as `PUT …/knob` takes it, kept as tafs cloud wrote it.
    #[serde(default)]
    pub trigger: Option<Value>,
    #[serde(default)]
    pub grant: Option<String>,
    #[serde(default)]
    pub step: Option<i64>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub ok: Option<String>,
    #[serde(default)]
    pub look: Option<String>,
    #[serde(default)]
    pub follow_up: Option<String>,
    #[serde(default)]
    pub approval: Option<Approval>,
    #[serde(default)]
    pub size: Option<Size>,
}

fn yes() -> bool {
    true
}

impl Item {
    fn is(&self, kind: &str) -> bool {
        self.kind.as_deref() == Some(kind)
    }

    /// `ringLabel`: "2 · Refund" for a step, else its label.
    pub fn ring_label(&self) -> String {
        match self.step {
            Some(n) => format!("{n} · {}", self.label),
            None => self.label.clone(),
        }
    }

    /// The line a stop is drawn with: its words, else its label.
    pub fn line(&self) -> &str {
        if self.words.is_empty() {
            &self.label
        } else {
            &self.words
        }
    }
}

/// `Knob.Rim`: today's work in the account's zone, and tafs cloud's line for it.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
pub struct Rim {
    #[serde(default, deserialize_with = "or_default")]
    pub done: i64,
    #[serde(default, deserialize_with = "or_default")]
    pub working: i64,
    #[serde(default, deserialize_with = "or_default")]
    pub retrying: i64,
    #[serde(default, deserialize_with = "or_default")]
    pub needs_you: i64,
    #[serde(default)]
    pub line: Option<String>,
}

impl Rim {
    pub fn total(&self) -> i64 {
        self.done + self.working + self.retrying + self.needs_you
    }
}

/// One connection's part of a draft (`Draft.Group`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Group {
    #[serde(default)]
    pub grant: Option<String>,
    pub name: String,
    pub wakes: Vec<String>,
    pub what: Vec<String>,
    pub results: Vec<String>,
}

/// `Knob.Draft`: the newest dial version made after the live one, in tafs cloud's words.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Draft {
    pub version: i64,
    #[serde(default, deserialize_with = "or_default")]
    pub groups: Vec<Group>,
    #[serde(default, deserialize_with = "or_default")]
    pub words: Vec<String>,
    #[serde(default, deserialize_with = "or_default")]
    pub wakes: Vec<String>,
    #[serde(default, deserialize_with = "or_default")]
    pub when: Vec<String>,
    #[serde(default, deserialize_with = "or_default")]
    pub what: Vec<String>,
    #[serde(default, deserialize_with = "or_default")]
    pub results: Vec<String>,
}

/// A connection on the dial (`Knob.Connection`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Connection {
    pub grant: String,
    pub name: String,
    pub wakes: i64,
    pub results: i64,
    #[serde(default)]
    pub icon: Option<String>,
}

/// A ring's name, as tafs cloud names it (`Knob.RingName`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RingName {
    pub key: String,
    pub name: String,
}

/// `Knob.Pending`: the version waiting to go live, and how many changes it makes.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Pending {
    pub version: i64,
    pub changes: i64,
}

/// What `PUT …/knob` takes (`Knob.Setting`): the triggers as tafs cloud wrote them, what a busy
/// taf does, and where results go.
#[derive(Debug, Clone, PartialEq)]
pub struct Setting {
    pub wakes: Vec<Value>,
    pub busy: String,
    pub results: Vec<Value>,
    pub what: Option<Vec<Value>>,
    pub why: Option<String>,
}

impl<'de> Deserialize<'de> for Setting {
    /// `busy`, else `when.busy`, else "skip".
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Setting, D::Error> {
        #[derive(Deserialize)]
        struct When {
            busy: String,
        }
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default, deserialize_with = "or_default")]
            wakes: Vec<Value>,
            #[serde(default)]
            busy: Option<String>,
            #[serde(default)]
            when: Option<When>,
            #[serde(default, deserialize_with = "or_default")]
            results: Vec<Value>,
            #[serde(default)]
            what: Option<Vec<Value>>,
            #[serde(default)]
            why: Option<String>,
        }
        let r = Raw::deserialize(d)?;
        Ok(Setting {
            wakes: r.wakes,
            busy: r
                .busy
                .or(r.when.map(|w| w.busy))
                .unwrap_or_else(|| "skip".into()),
            results: r.results,
            what: r.what,
            why: r.why,
        })
    }
}

impl Setting {
    /// As `PUT` takes it: `when: {busy}`.
    pub fn body(&self) -> Vec<u8> {
        let mut o = Map::new();
        o.insert("wakes".into(), Value::Array(self.wakes.clone()));
        o.insert("when".into(), serde_json::json!({ "busy": self.busy }));
        o.insert("results".into(), Value::Array(self.results.clone()));
        if let Some(what) = &self.what {
            o.insert("what".into(), Value::Array(what.clone()));
        }
        if let Some(why) = &self.why {
            o.insert("why".into(), Value::from(why.as_str()));
        }
        Value::Object(o).to_string().into_bytes()
    }

    /// `sends(to:)`: whether results go to `place`, named alone or with what it needs.
    pub fn sends_to(&self, place: &str) -> bool {
        self.results
            .iter()
            .any(|v| place_of(v).as_deref() == Some(place))
    }
}

/// `Setting.place(of:)`: a place by name ("app"), or with what it needs (`{place: …}`).
pub fn place_of(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("place").and_then(Value::as_str).map(str::to_string),
        _ => None,
    }
}

/// A taf's dial (`TafsCloud.Knob`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Knob {
    /// Older servers didn't send it: Pause stays offered until they do.
    #[serde(default = "yes", deserialize_with = "on_by_default")]
    pub can_pause: bool,
    #[serde(default)]
    pub pending: Option<Pending>,
    #[serde(default)]
    pub version: Option<i64>,
    #[serde(default)]
    pub shown: Option<i64>,
    #[serde(default)]
    pub why: Option<String>,
    #[serde(default, deserialize_with = "or_default")]
    pub what: Vec<Item>,
    #[serde(default, deserialize_with = "or_default")]
    pub connections: Vec<Connection>,
    #[serde(default, deserialize_with = "or_default")]
    pub rings: Vec<RingName>,
    #[serde(default, deserialize_with = "or_default")]
    pub does: Does,
    #[serde(default, deserialize_with = "or_default")]
    pub wakes: Vec<Item>,
    #[serde(default, deserialize_with = "or_default")]
    pub when: Vec<Item>,
    #[serde(default, deserialize_with = "or_default")]
    pub results: Vec<Item>,
    #[serde(default, deserialize_with = "or_default")]
    pub rim: Rim,
    #[serde(default)]
    pub draft: Option<Draft>,
    #[serde(default)]
    pub setting: Option<Setting>,
}

impl Knob {
    /// `acceptsChoices(waitingFor:)`: a choice is made on the newest draft this computer knows of.
    pub fn accepts_choices(&self, changed: Option<i64>) -> bool {
        let pending = self
            .pending
            .as_ref()
            .map(|p| p.version)
            .or(self.draft.as_ref().map(|d| d.version))
            .unwrap_or(0)
            .max(changed.unwrap_or(0));
        pending == 0 || pending <= self.version.unwrap_or(0) || self.shown == Some(pending)
    }

    /// The stops on When of one kind, in tafs cloud's order.
    pub fn wakes_of(&self, kind: &str) -> Vec<&Item> {
        self.wakes.iter().filter(|i| i.is(kind)).collect()
    }

    /// The stop `item` is now, wherever it is on the dial.
    pub fn find(&self, id: &str) -> Option<&Item> {
        self.wakes
            .iter()
            .chain(self.what.iter())
            .chain(self.results.iter())
            .find(|i| i.id == id)
    }
}

/// The dial, as the iPad decodes it; `None` when it can't be.
pub fn read(body: &[u8]) -> Option<Knob> {
    serde_json::from_slice(body).ok()
}

/// `knob(_:version:)`'s path.
pub fn path(taf: &str, version: Option<i64>) -> String {
    let base = format!("/v1/tafs/{}/knob", crate::work::segment(taf));
    match version {
        Some(v) => format!("{base}?version={v}"),
        None => base,
    }
}

/// What a change made (`Knob.Change`): the taf's next version, and its words.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Change {
    pub version: i64,
    #[serde(default, deserialize_with = "or_default")]
    pub words: Vec<String>,
}

pub fn read_change(body: &[u8]) -> Option<Change> {
    serde_json::from_slice(body).ok()
}

/// `putResults`'s path.
pub fn results_path(taf: &str) -> String {
    format!("/v1/tafs/{}/results", crate::work::segment(taf))
}

/// `putResults`'s body.
pub fn results_body(places: &[String]) -> Vec<u8> {
    serde_json::json!({ "places": places })
        .to_string()
        .into_bytes()
}

/// `removeFollowUp`'s path.
pub fn follow_up_path(taf: &str, follow_up: &str) -> String {
    format!(
        "/v1/tafs/{}/follow-ups/{}",
        crate::work::segment(taf),
        crate::work::segment(follow_up)
    )
}

/// `removeFollowUp`'s answer, `{"removed": …}`.
pub fn removed(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("removed").map(Value::is_string))
        .unwrap_or(false)
}

/// `setPaused`'s path: every start paused, or resumed.
pub fn pause_path(taf: &str, paused: bool) -> String {
    format!(
        "/v1/tafs/{}/{}",
        crate::work::segment(taf),
        if paused { "pause" } else { "resume" }
    )
}

/// The taf a change answers with (`{"taf": CloudTaf}`): whether it's paused (`status`), and where
/// tafs cloud delivers its results itself (`results`). `None` when there's no taf in it.
#[derive(Debug, Clone, PartialEq)]
pub struct TafAnswer {
    pub paused: bool,
    pub results: Option<Vec<Value>>,
}

pub fn read_taf(body: &[u8]) -> Option<TafAnswer> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let t = v.get("taf")?.as_object()?;
    Some(TafAnswer {
        paused: t.get("status").and_then(Value::as_str) == Some("paused"),
        results: match t.get("results") {
            None | Some(Value::Null) => None,
            Some(Value::Array(a)) => Some(a.clone()),
            Some(_) => return None,
        },
    })
}

/// tafs' own places, in their order (`ResultsGoToView.builtInOrder`).
pub const BUILT_IN: [&str; 3] = ["door_reply", "app", "owner_email"];

/// `liveBuiltIns`: tafs' own places on, as the live taf has them; without them, the dial's
/// when it shows the live version; `None` when they aren't known.
pub fn live_built_ins(live: Option<&[Value]>, knob: Option<&Knob>) -> Option<Vec<String>> {
    if let Some(live) = live {
        let on: Vec<String> = live.iter().filter_map(place_of).collect();
        return Some(
            BUILT_IN
                .iter()
                .filter(|p| on.iter().any(|o| o == *p))
                .map(|p| p.to_string())
                .collect(),
        );
    }
    let knob = knob?;
    if knob.pending.is_some() {
        return None;
    }
    Some(
        BUILT_IN
            .iter()
            .filter(|p| {
                match knob
                    .results
                    .iter()
                    .find(|r| r.place.as_deref() == Some(**p))
                {
                    Some(r) => r.on,
                    None => knob.setting.as_ref().is_some_and(|s| s.sends_to(p)),
                }
            })
            .map(|p| p.to_string())
            .collect(),
    )
}

/// `ResultsGoToView.places`: what `PUT …/results` is sent when one of tafs' places is switched,
/// the live ones with that one changed; `None` when it would leave none.
pub fn places(live: &[String], switching: &str, on: bool) -> Option<Vec<String>> {
    let mut all: Vec<&str> = live
        .iter()
        .map(String::as_str)
        .filter(|p| *p != switching)
        .collect();
    if on {
        all.push(switching);
    }
    let sent: Vec<String> = BUILT_IN
        .iter()
        .filter(|p| all.contains(p))
        .map(|p| p.to_string())
        .collect();
    (!sent.is_empty()).then_some(sent)
}

/// tafs' own places' rows (`builtIns`): each title and line, in the same words as on Android.
pub fn built_in_rows(
    w: &WordBook,
    email: Option<&str>,
) -> Result<Vec<(&'static str, String, String)>, WordsError> {
    Ok(vec![
        (
            "door_reply",
            w.line("setup.place.reply_title")?,
            w.line("setup.place.reply_words")?,
        ),
        (
            "app",
            w.line("setup.place.app_title")?,
            w.line("setup.place.app_words")?,
        ),
        (
            "owner_email",
            w.line("setup.place.email_title")?,
            match email {
                Some(e) => w.get("setup.place.email_words", &[("email", e)])?,
                None => w.line("setup.place.email_words_yours")?,
            },
        ),
    ])
}

/// `KnobPendingPanel`'s words: "DRAFT · V8 · NOT LIVE", the draft's lines, and its button, which
/// names what the device asks for before it signs (`Biometry.name`): with no Face ID or Touch ID,
/// as on this computer, "your passcode". Its tap says this computer can't approve.
pub fn pending_lines(
    w: &WordBook,
    knob: &Knob,
) -> Result<Option<(String, Vec<String>, String)>, WordsError> {
    let biometry = w.line("approval.your_passcode")?;
    let biometry = biometry.as_str();
    let Some(p) = &knob.pending else {
        return Ok(None);
    };
    Ok(Some((
        w.get("knob.not_live", &[("version", &p.version.to_string())])?,
        knob.draft
            .as_ref()
            .map(|d| d.words.clone())
            .unwrap_or_default(),
        w.count("knob.make_live", p.changes, &[("biometry", biometry)])?,
    )))
}

/// The rings, inside to out (`Knob.Ring`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Ring {
    Wakes,
    What,
    Results,
}

impl Ring {
    pub const ALL: [Ring; 3] = [Ring::Wakes, Ring::What, Ring::Results];

    fn key(self) -> &'static str {
        match self {
            Ring::Wakes => "wakes",
            Ring::What => "what",
            Ring::Results => "results",
        }
    }

    /// `TafKnobView.color`: the same on every appearance.
    pub fn color(self) -> u32 {
        match self {
            Ring::Wakes => 0x0B7DA4,
            Ring::What => 0x2A64F5,
            Ring::Results => 0x7B4DFF,
        }
    }

    /// The dial's own words for it, the same on iPhone, iPad and Android.
    fn own_title(self, w: &WordBook) -> Result<String, WordsError> {
        w.line(match self {
            Ring::Wakes => "knob.ring.when",
            Ring::What => "knob.ring.what",
            Ring::Results => "knob.ring.where",
        })
    }
}

impl Knob {
    /// `orderedRings`: tafs cloud's order, each once; all three when it names none.
    pub fn ordered_rings(&self) -> Vec<Ring> {
        if self.rings.is_empty() {
            return Ring::ALL.to_vec();
        }
        let mut out: Vec<Ring> = Vec::new();
        for name in &self.rings {
            if let Some(r) = Ring::ALL.into_iter().find(|r| r.key() == name.key) {
                if !out.contains(&r) {
                    out.push(r);
                }
            }
        }
        out
    }

    /// `title(_:)`: tafs cloud's name for a ring, else the dial's own.
    pub fn title(&self, w: &WordBook, ring: Ring) -> Result<String, WordsError> {
        match self.rings.iter().find(|r| r.key == ring.key()) {
            Some(r) => Ok(r.name.clone()),
            None => ring.own_title(w),
        }
    }

    pub fn items(&self, ring: Ring) -> &[Item] {
        match ring {
            Ring::Wakes => &self.wakes,
            Ring::What => &self.what,
            Ring::Results => &self.results,
        }
    }
}

/// `KnobNavigation`: presentation only, filtering and walking the stops never changes their
/// meaning.
pub mod nav {
    use super::{Item, Knob, Ring};

    /// Which stops show: all, the taf's own, or one connection's.
    #[derive(Debug, Clone, PartialEq, Eq, Default)]
    pub enum Filter {
        #[default]
        All,
        Taf,
        Connection(String),
    }

    #[derive(Debug, Clone, PartialEq)]
    pub struct Stop {
        pub ring: Ring,
        pub item: Item,
    }

    pub fn items(knob: &Knob, ring: Ring, filter: &Filter) -> Vec<Item> {
        knob.items(ring)
            .iter()
            .filter(|i| match filter {
                Filter::All => true,
                Filter::Taf => i.grant.is_none(),
                Filter::Connection(g) => i.grant.as_deref() == Some(g.as_str()),
            })
            .cloned()
            .collect()
    }

    pub fn stops(knob: &Knob, filter: &Filter) -> Vec<Stop> {
        knob.ordered_rings()
            .into_iter()
            .flat_map(|ring| {
                items(knob, ring, filter)
                    .into_iter()
                    .map(move |item| Stop { ring, item })
            })
            .collect()
    }

    /// The next stop (`direction` 1) or the one before (-1), round the dial; from none, the first
    /// or the last.
    pub fn next(
        stops: &[Stop],
        ring: Option<Ring>,
        id: Option<&str>,
        direction: i64,
    ) -> Option<Stop> {
        if stops.is_empty() {
            return None;
        }
        let n = stops.len() as i64;
        let Some(index) = stops
            .iter()
            .position(|s| Some(s.ring) == ring && Some(s.item.id.as_str()) == id)
        else {
            return if direction < 0 {
                stops.last()
            } else {
                stops.first()
            }
            .cloned();
        };
        Some(stops[((index as i64 + direction % n + n) % n) as usize].clone())
    }

    /// A crowded ring shows `capacity` stops at a time: the page with the chosen one.
    pub fn page(items: &[Item], selected: Option<&str>, capacity: usize) -> Vec<Item> {
        if capacity == 0 {
            return Vec::new();
        }
        let index = items
            .iter()
            .position(|i| Some(i.id.as_str()) == selected)
            .unwrap_or(0);
        let start = index / capacity * capacity;
        items.iter().skip(start).take(capacity).cloned().collect()
    }

    /// `nearestTurn`: the ring's turn that brings stop `index` to the top the short way.
    pub fn nearest_turn(turn: i64, index: i64, stops: i64) -> i64 {
        if stops <= 0 {
            return turn;
        }
        let at = turn.rem_euclid(stops);
        let mut d = index - at;
        if d as f64 > stops as f64 / 2.0 {
            d -= stops;
        }
        if (d as f64) < -(stops as f64) / 2.0 {
            d += stops;
        }
        turn + d
    }

    /// `labelWidth`: the chord inside a stop's share of its ring, leaving a gap between
    /// neighbours.
    pub fn label_width(radius: f64, stops: usize, maximum: f64, gap: f64) -> f64 {
        if stops <= 1 {
            return maximum.max(0.0);
        }
        (2.0 * radius * (std::f64::consts::PI / stops as f64).sin() - gap)
            .min(maximum)
            .max(0.0)
    }
}

/// What the dial's face shows (`TafKnobView.Face`): the job, a chosen stop, or a held call.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Face {
    #[default]
    Overview,
    Chosen(Ring, String),
    Held(String),
}

/// A held call the face can show: what it is, its amount, why, and whether it may be approved
/// once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldFace {
    pub id: String,
    pub summary: String,
    pub amount: Option<String>,
    pub reason: String,
    pub approvable_once: bool,
    /// The hold's `accounting_version`, for Approve once with a security key.
    pub accounting_version: Option<i64>,
}

/// The face's tint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    Held,
    Ring(Ring),
    Ink,
}

/// The face's words, the note under the dial, and the chosen stop's size.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaceLines {
    pub kicker: String,
    pub big: String,
    pub hint: String,
    pub note: String,
    pub size: Option<String>,
    pub tint: Tint,
}

/// The chosen stop, among those the filter shows.
pub fn chosen_item(knob: &Knob, face: &Face, filter: &nav::Filter) -> Option<Item> {
    match face {
        Face::Chosen(ring, id) => nav::items(knob, *ring, filter)
            .into_iter()
            .find(|i| &i.id == id),
        _ => None,
    }
}

/// `TafKnobView`'s `kicker`, `big`, `hint` and `note`. Approving an app event names what the
/// device asks for: "your passcode" on this computer, which can't.
pub fn face_lines(
    w: &WordBook,
    knob: &Knob,
    face: &Face,
    filter: &nav::Filter,
    held: &[HeldFace],
) -> Result<FaceLines, WordsError> {
    let call = match face {
        Face::Held(id) => held.iter().find(|h| &h.id == id),
        _ => None,
    };
    let item = chosen_item(knob, face, filter);
    let size = item.as_ref().and_then(|i| i.size.as_ref()).and_then(|s| {
        usize::try_from(s.index)
            .ok()
            .and_then(|n| s.choices.get(n))
            .map(|c| format!("{} · {}", s.label, c.label))
    });
    if let Some(call) = call {
        return Ok(FaceLines {
            kicker: crate::words::upper(&call.summary),
            big: call.amount.clone().unwrap_or_else(|| call.summary.clone()),
            hint: if call.approvable_once {
                w.line("knob.approve_once")?
            } else {
                String::new()
            },
            note: call.reason.clone(),
            size: None,
            tint: Tint::Held,
        });
    }
    let (ring, item) = match (face, item) {
        (Face::Chosen(ring, _), Some(item)) => (*ring, item),
        _ => {
            return Ok(FaceLines {
                kicker: w.line("knob.why")?,
                big: knob.why.clone().unwrap_or_else(|| knob.does.label.clone()),
                hint: w.line("knob.tap_a_ring")?,
                note: w.line("knob.caption")?,
                size: None,
                tint: Tint::Ink,
            })
        }
    };
    let big = match item.state.as_deref().filter(|s| !s.is_empty()) {
        Some(state) => state.to_string(),
        None => item
            .size
            .as_ref()
            .and_then(|s| usize::try_from(s.index).ok().and_then(|n| s.choices.get(n)))
            .map(|c| c.label.clone())
            .unwrap_or_else(|| item.label.clone()),
    };
    let hint = if item.ok.as_deref() == Some("approve") {
        w.get(
            "approval.approve_with",
            &[("biometry", &w.line("approval.your_passcode")?)],
        )?
    } else if let Some(tap) = &item.tap {
        tap.clone()
    } else if item.size.is_some() {
        w.line("knob.use_size")?
    } else {
        w.line("knob.ask_coach_to_change")?
    };
    Ok(FaceLines {
        kicker: crate::words::upper(&format!("{} · {}", knob.title(w, ring)?, item.ring_label())),
        big,
        hint,
        note: item.words.clone(),
        size,
        tint: Tint::Ring(ring),
    })
}

/// The service menu (`serviceMenu`): "Showing All services", and each choice with its words.
pub fn filter_name(w: &WordBook, knob: &Knob, filter: &nav::Filter) -> Result<String, WordsError> {
    match filter {
        nav::Filter::All => w.line("knob.all_services"),
        nav::Filter::Taf => w.line("knob.this_taf"),
        nav::Filter::Connection(g) => match knob.connections.iter().find(|c| &c.grant == g) {
            Some(c) => Ok(c.name.clone()),
            None => w.line("knob.all_services"),
        },
    }
}

pub fn filter_choices(w: &WordBook, knob: &Knob) -> Result<Vec<(nav::Filter, String)>, WordsError> {
    let mut out = vec![
        (nav::Filter::All, w.line("knob.all_services")?),
        (nav::Filter::Taf, w.line("knob.this_taf")?),
    ];
    for c in &knob.connections {
        let f = nav::Filter::Connection(c.grant.clone());
        let n = nav::stops(knob, &f).len() as i64;
        out.push((f, w.count("knob.with_stops", n, &[("name", &c.name)])?));
    }
    Ok(out)
}

/// The rim's arcs: today's done, working, retrying and needing you, each its share of the
/// circle (0 to 1), with a hair between them.
pub fn rim_arcs(rim: &Rim) -> Vec<(f64, f64, &'static str)> {
    let total = rim.total().max(1) as f64;
    let gap = 1.2 / 360.0;
    let mut from = 0.0;
    let mut out = Vec::new();
    for (n, tone) in [
        (rim.done, "ok"),
        (rim.working, "blue"),
        (rim.retrying, "amber"),
        (rim.needs_you, "no"),
    ] {
        if n > 0 {
            let span = n as f64 / total;
            out.push((from + gap, (from + span - gap).max(from + gap), tone));
            from += span;
        }
    }
    out
}

/// Starts when's line under its schedule: tafs cloud's words for its next run, else "Next run at
/// 14:00" in the account's zone, then the account's zone (UTC when it has none); none while
/// paused or with no next run.
pub fn next_run_line(
    w: &WordBook,
    h: &crate::home::HomeTaf,
    zone: Option<&str>,
) -> Result<Option<String>, WordsError> {
    let Some(next) = &h.next_run else {
        return Ok(None);
    };
    if h.status.as_deref() == Some("paused") {
        return Ok(None);
    }
    let words = match &h.next_run_words {
        Some(words) => words.clone(),
        None => w.get(
            "taf.next_run_at",
            &[("time", &crate::account_time::zoned_short_time(next))],
        )?,
    };
    Ok(Some(format!("{words} · {}", zone.unwrap_or("UTC"))))
}

/// A follow-up's end in the account's zone, with or without fractions of a second; none when
/// it can't be read.
pub fn expires_words(s: &str) -> Option<String> {
    let at = chrono::DateTime::parse_from_rfc3339(s).ok()?;
    Some(
        crate::account_time::local(&at)
            .format("%Y-%m-%d %H:%M")
            .to_string()
            + &crate::account_time::label(&at),
    )
}

#[cfg(test)]
mod tests;
