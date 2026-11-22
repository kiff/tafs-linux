//! A taf's doors and the knocks at them (`TafsCloud+Doors.swift`, `DoorsModel`), read from
//! tafs cloud as the session (`GET /v1/tafs/{id}/doors`), and the lines a knock is drawn with on
//! the taf's Activity (`TafKnocksPanel`, `KnockCard`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::words::{WordBook, WordsError};

/// A paired sender (`Door.Key`): its id; the rest read when it can be.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Key {
    pub id: String,
    pub name: Option<String>,
    pub auto: bool,
    /// An `api` door's key: its last four, when it was made and last used. Never the key.
    pub last4: Option<String>,
    pub made_at: Option<String>,
    pub last_used_at: Option<String>,
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Key, D::Error> {
        let v = Value::deserialize(d)?;
        let id = v
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| serde::de::Error::custom("a key without its id"))?;
        Ok(Key {
            id: id.to_string(),
            name: v.get("name").and_then(Value::as_str).map(str::to_string),
            auto: v.get("auto").and_then(Value::as_bool).unwrap_or(false),
            last4: v.get("last4").and_then(Value::as_str).map(str::to_string),
            made_at: v.get("made_at").and_then(Value::as_str).map(str::to_string),
            last_used_at: v
                .get("last_used_at")
                .and_then(Value::as_str)
                .map(str::to_string),
        })
    }
}

/// tafs cloud's words for a door (`Door.Words`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DoorWords {
    pub state: String,
    pub tap: String,
    pub settings: String,
}

/// One door (`TafsCloud.Door`), decoded as the iPad's synthesized decoder does: every field it
/// needs, or the doors aren't read.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Door {
    pub id: String,
    pub app: String,
    pub kind: String,
    pub label: String,
    pub open: bool,
    pub address: String,
    pub proof: String,
    pub how: String,
    pub keys: Vec<Key>,
    pub today: i64,
    pub limit: i64,
    pub resting: bool,
    #[serde(default)]
    pub others_today: Option<i64>,
    #[serde(default)]
    pub others_limit: Option<i64>,
    #[serde(default)]
    pub words: Option<DoorWords>,
    #[serde(default)]
    pub grant: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
    /// An `api` door's counts (`today` the requests today), and its last request.
    #[serde(default)]
    pub counts: Option<Counts>,
    #[serde(default)]
    pub last_request: Option<LastRequest>,
}

/// An `api` door's counts (`Door.Counts`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Counts {
    #[serde(default)]
    pub today: Option<i64>,
}

/// An `api` door's last request (`Door.LastRequest`): its state in tafs cloud's words, and when.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LastRequest {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub at: Option<String>,
    #[serde(default)]
    pub words: Option<String>,
}

/// The `api` door's lines that aren't words (`APIDoorText`): dates, a key's name, the endpoint and
/// the example.
pub mod api {
    use super::{Door, Key};
    use crate::words::{WordBook, WordsError};

    /// A door holds at most two live keys, so one can be replaced without stopping the other.
    pub const MAX_KEYS: usize = 2;

    /// A door "From another application".
    pub fn is_api(door: &Door) -> bool {
        door.kind == "api"
    }

    /// A change tafs cloud takes on an `api` door only signed: opening it again, its keys, its
    /// removal. Pausing (`PATCH` with `open: false`) is the one this computer sends. The doors
    /// screen asks before each call, and refuses these (`signed::Act::at_api_door`).
    pub fn only_signed(door: &Door, method: &str, opens: bool) -> bool {
        is_api(door) && !(method == "PATCH" && !opens)
    }

    /// A door's address on tafs cloud, whole.
    pub fn endpoint(door: &Door) -> String {
        if door.address.starts_with("https://") {
            door.address.clone()
        } else {
            format!("{}{}", crate::home::TAFS_CLOUD, door.address)
        }
    }

    /// Today's requests: the counts' when sent, else the door's own.
    pub fn today(door: &Door) -> i64 {
        door.counts
            .as_ref()
            .and_then(|c| c.today)
            .unwrap_or(door.today)
    }

    /// The last request's state in tafs cloud's words, trimmed; none until it sends them, never
    /// the raw `status`.
    pub fn last_request_words(door: &Door) -> Option<String> {
        let words = door.last_request.as_ref()?.words.as_deref()?.trim();
        (!words.is_empty()).then(|| words.to_string())
    }

    /// An ISO time from tafs cloud, in the account's zone: its date and time, abbreviated; none
    /// when it isn't one.
    pub fn when(language: &str, iso: Option<&str>) -> Option<String> {
        let at = chrono::DateTime::parse_from_rfc3339(iso?).ok()?;
        Some(crate::locale::date_time(
            language,
            crate::account_time::local(&at),
        ))
    }

    /// "Last request {time} · {status}", in tafs cloud's words only.
    pub fn last_request(w: &WordBook, door: &Door) -> Result<Option<String>, WordsError> {
        let Some(words) = last_request_words(door) else {
            return Ok(None);
        };
        let at = door.last_request.as_ref().and_then(|l| l.at.as_deref());
        let time = when(w.language(), at)
            .or_else(|| at.map(str::to_string))
            .unwrap_or_default();
        Ok(Some(w.get(
            "door.api.last_request",
            &[("time", &time), ("status", &words)],
        )?))
    }

    /// What a key is called: its name, else its last four, else its id.
    pub fn name(key: &Key) -> String {
        key.name
            .clone()
            .or_else(|| key.last4.as_ref().map(|l| format!("…{l}")))
            .unwrap_or_else(|| key.id.clone())
    }

    /// The key's line: made, last used or not yet, and its last four. Never the key.
    pub fn key_line(w: &WordBook, key: &Key) -> Result<String, WordsError> {
        let made = when(w.language(), key.made_at.as_deref()).unwrap_or_default();
        let last4 = key.last4.clone().unwrap_or_default();
        match when(w.language(), key.last_used_at.as_deref()) {
            Some(used) => w.get(
                "door.api.key_line",
                &[("made", &made), ("used", &used), ("last4", &last4)],
            ),
            None => w.get(
                "door.api.key_never_used",
                &[("made", &made), ("last4", &last4)],
            ),
        }
    }

    /// An example request. It reads the key from the caller's own secret, `$TAFS_API_KEY`: the key
    /// is never written in it.
    pub fn example(endpoint: &str) -> String {
        format!(
            "curl -X POST {endpoint} \\\n  -H \"Authorization: Bearer $TAFS_API_KEY\" \\\n  -H \"Idempotency-Key: <your request id>\" \\\n  -H \"Content-Type: application/json\" \\\n  -d '{{\"input\": {{}}}}'"
        )
    }
}

/// An app a door can open on (`DoorApp`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DoorApp {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub proof: String,
    pub how: String,
}

/// Someone without a key wrote (`TafsCloud.Knock`): the owner decides.
#[derive(Debug, Clone, PartialEq)]
pub struct Knock {
    pub id: String,
    pub door: String,
    pub app: String,
    pub from: String,
    pub name: Option<String>,
    /// "pass", "fail" (the sender check failed) or "none" (the door can't tell).
    pub proof: String,
    pub subject: Option<String>,
    pub text: String,
    pub attachments: i64,
    pub at: DateTime<Utc>,
    pub state: String,
    /// Not a message: the line saying the door had its day's messages.
    pub rest: Option<String>,
    pub dropped: i64,
    /// Its sender was paired from it; the message still waits.
    pub paired: bool,
}

/// An ISO time as tafs cloud writes it, with or without fractions of a second.
fn time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

impl<'de> Deserialize<'de> for Knock {
    /// `Knock.init(from:)`: its id, door, app, sender and time, or it isn't read; the rest
    /// read when it can be.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Knock, D::Error> {
        let v = Value::deserialize(d)?;
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        let need =
            |k: &str| s(k).ok_or_else(|| serde::de::Error::custom(format!("a knock without {k}")));
        let n = |k: &str| v.get(k).and_then(Value::as_i64).unwrap_or(0);
        Ok(Knock {
            id: need("id")?,
            door: need("door")?,
            app: need("app")?,
            from: need("from")?,
            name: s("name"),
            proof: s("proof").unwrap_or_else(|| "none".into()),
            subject: s("subject"),
            text: s("text").unwrap_or_default(),
            attachments: n("attachments"),
            at: s("at")
                .as_deref()
                .and_then(time)
                .ok_or_else(|| serde::de::Error::custom("a knock without its time"))?,
            state: s("state").unwrap_or_else(|| "open".into()),
            rest: s("rest"),
            dropped: n("dropped"),
            paired: v.get("paired").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}

impl Knock {
    /// `isLine`: the door's line about its day's messages, not a message.
    pub fn is_line(&self) -> bool {
        self.rest.is_some()
    }

    /// `who`: "Ana (ana@example.com)", or the sender alone.
    pub fn who(&self) -> String {
        match &self.name {
            Some(n) => format!("{n} ({})", self.from),
            None => self.from.clone(),
        }
    }
}

/// tafs cloud's answer (`TafsCloud.Doors`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Doors {
    pub doors: Vec<Door>,
    pub apps: Vec<DoorApp>,
    pub knocks: Vec<Knock>,
}

/// The doors, as the iPad decodes them; `None` when they can't be.
pub fn read(body: &[u8]) -> Option<Doors> {
    serde_json::from_slice(body).ok()
}

/// `TafsCloud.doors`' path.
pub fn path(taf: &str) -> String {
    format!("/v1/tafs/{}/doors", crate::work::segment(taf))
}

/// `answerKnock`'s path.
pub fn knock_path(taf: &str, knock: &str) -> String {
    format!(
        "/v1/tafs/{}/doors/knocks/{}",
        crate::work::segment(taf),
        crate::work::segment(knock)
    )
}

/// `answerKnock`'s body: "do" (with the owner's words, when they wrote some), "pair" or "ignore".
pub fn answer_body(action: &str, say: Option<&str>) -> Vec<u8> {
    let mut body = serde_json::Map::new();
    body.insert("action".into(), Value::from(action));
    if let Some(say) = say.filter(|s| !s.is_empty()) {
        body.insert("say".into(), Value::from(say));
    }
    Value::Object(body).to_string().into_bytes()
}

/// `answerKnock`'s answer, `{"state": …}`: whether it was taken.
pub fn answered(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("state").map(Value::is_string))
        .unwrap_or(false)
}

/// `DoorNames.label`: an app's name as the owner reads it.
pub fn label(app: &str) -> String {
    match app {
        "email" => "email".into(),
        "whatsapp" => "WhatsApp".into(),
        _ => {
            let mut c = app.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        }
    }
}

/// `TafKnocksPanel`: the open knocks that are messages, strangers' first, then paired senders'.
pub fn panel(doors: &Doors) -> (Vec<Knock>, Vec<Knock>) {
    let open = doors
        .knocks
        .iter()
        .filter(|k| k.state == "open" && !k.is_line());
    let (paired, waiting): (Vec<Knock>, Vec<Knock>) = open.cloned().partition(|k| k.paired);
    (waiting, paired)
}

/// A knock's lines and buttons, as `KnockCard` draws a message (the panel draws no door lines).
#[derive(Debug, Clone, PartialEq)]
pub struct KnockLines {
    /// "● KNOCK · BY EMAIL", with "· DIDN'T CHECK OUT" when the sender check failed.
    pub head: String,
    /// "Paired · Do it or Ignore this message", for a paired sender's.
    pub paired_line: Option<String>,
    /// "Ana (ana@example.com) wrote about "Refund". Nothing was done."
    pub wrote: String,
    pub text: String,
    pub attachments: Option<String>,
    /// Why the sender can't be trusted, when it can't.
    pub proof_note: Option<String>,
    /// Do it, unless the sender check failed.
    pub do_it: Option<String>,
    /// "Pair Ana" or "Pair them", for a sender who passed the check and isn't paired yet.
    pub pair: Option<String>,
    pub ignore: String,
}

pub fn knock_lines(w: &WordBook, k: &Knock) -> Result<KnockLines, WordsError> {
    let app = crate::words::upper(&label(&k.app));
    let mut head = if k.paired {
        w.get("door.knock.paired_by", &[("app", &app)])?
    } else {
        w.get("door.knock.by", &[("app", &app)])?
    };
    if k.proof == "fail" {
        head = format!("{head} · {}", w.line("door.knock.didnt_check_out")?);
    }
    let who = k.who();
    Ok(KnockLines {
        head,
        paired_line: if k.paired {
            Some(w.line("door.knock.paired_line")?)
        } else {
            None
        },
        wrote: match &k.subject {
            Some(subject) => w.get(
                "door.knock.wrote_about",
                &[("who", &who), ("subject", subject)],
            )?,
            None => w.get("door.knock.wrote", &[("who", &who)])?,
        },
        text: k.text.clone(),
        attachments: if k.attachments > 0 {
            Some(w.count("door.knock.attachments", k.attachments, &[])?)
        } else {
            None
        },
        proof_note: match k.proof.as_str() {
            "fail" => Some(w.get("door.knock.proof_failed", &[("from", &k.from)])?),
            "none" => Some(w.line("door.knock.proof_none")?),
            _ => None,
        },
        do_it: if k.proof != "fail" {
            Some(w.line("door.do_it_button")?)
        } else {
            None
        },
        pair: if k.proof == "pass" && !k.paired {
            Some(match k.name.as_deref().and_then(|n| n.split(' ').next()) {
                Some(first) => w.get("door.knock.pair", &[("name", first)])?,
                None => w.line("door.knock.pair_them")?,
            })
        } else {
            None
        },
        ignore: w.line("door.ignore")?,
    })
}

/// `DoItSheet`'s words for a taf, by its name (`taf?.name ?? Words.Chat.yourTaf`).
#[derive(Debug, Clone, PartialEq)]
pub struct DoItLines {
    pub title: String,
    pub intro: String,
    pub placeholder: String,
    pub default_say: String,
    pub send: String,
    pub sending: String,
    pub cancel: String,
}

pub fn do_it_lines(w: &WordBook, k: &Knock, taf: Option<&str>) -> Result<DoItLines, WordsError> {
    let your_taf = w.line("chat.your_taf")?;
    let it = w.line("chat.it_fallback")?;
    let who = k.who();
    Ok(DoItLines {
        title: w.get("door.do_it.title", &[("taf", taf.unwrap_or(&your_taf))])?,
        intro: w.get(
            "door.do_it.intro",
            &[("who", &who), ("taf", taf.unwrap_or(&it))],
        )?,
        placeholder: w.line("door.what_do")?,
        default_say: w.line("door.do_it.default")?,
        send: w.get("door.send_to", &[("taf", taf.unwrap_or(&your_taf))])?,
        sending: w.line("common.sending")?,
        cancel: w.line("common.cancel")?,
    })
}

impl Door {
    /// `holdsKeys`: it can tell who wrote, so it holds keys.
    pub fn holds_keys(&self) -> bool {
        self.proof == "proven"
    }
}

impl Doors {
    /// `openKnocks`.
    pub fn open_knocks(&self) -> Vec<Knock> {
        self.knocks
            .iter()
            .filter(|k| k.state == "open")
            .cloned()
            .collect()
    }

    /// `addable`: the apps that can still be turned on, one door of each kind per taf.
    pub fn addable(&self) -> Vec<DoorApp> {
        self.apps
            .iter()
            .filter(|a| !self.doors.iter().any(|d| d.kind == a.kind))
            .cloned()
            .collect()
    }

    /// A change that answered with the door: it replaces the one shown, or comes last.
    pub fn put(&mut self, door: Door) {
        match self.doors.iter().position(|d| d.id == door.id) {
            Some(i) => self.doors[i] = door,
            None => self.doors.push(door),
        }
    }
}

/// A door's own path (`setDoorOpen`, `removeDoor`).
pub fn door_path(taf: &str, door: &str) -> String {
    format!("{}/{}", path(taf), crate::work::segment(door))
}

/// `pair`'s path.
pub fn keys_path(taf: &str, door: &str) -> String {
    format!("{}/keys", door_path(taf, door))
}

/// `unpair`'s path.
pub fn key_path(taf: &str, door: &str, sender: &str) -> String {
    format!("{}/{}", keys_path(taf, door), crate::work::segment(sender))
}

/// `addDoor`'s body: the app, and what else its kind needs (a Slack door's connection).
pub fn add_body(app: &str, extra: &[(&str, &str)]) -> Vec<u8> {
    let mut body = serde_json::Map::new();
    for (k, v) in extra {
        body.insert((*k).into(), Value::from(*v));
    }
    body.insert("app".into(), Value::from(app));
    Value::Object(body).to_string().into_bytes()
}

/// `setDoorOpen`'s body.
pub fn open_body(open: bool) -> Vec<u8> {
    serde_json::json!({ "open": open }).to_string().into_bytes()
}

/// `DoorsModel.pair`: the sender trimmed, none when that leaves nothing; the name only when
/// one was written.
pub fn pair_body(sender: &str, name: &str) -> Option<Vec<u8>> {
    let sender = sender.trim();
    if sender.is_empty() {
        return None;
    }
    let mut body = serde_json::Map::new();
    body.insert("id".into(), Value::from(sender));
    if !name.is_empty() {
        body.insert("name".into(), Value::from(name));
    }
    Some(Value::Object(body).to_string().into_bytes())
}

/// `DoorResponse`: the door a change answers with.
pub fn read_door(body: &[u8]) -> Option<Door> {
    #[derive(Deserialize)]
    struct DoorResponse {
        door: Door,
    }
    serde_json::from_slice::<DoorResponse>(body)
        .ok()
        .map(|r| r.door)
}

/// `removeDoor`'s answer, `{"removed": …}`: whether it was taken.
pub fn removed(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("removed").map(Value::is_string))
        .unwrap_or(false)
}

/// `DoorNames.mark`: the letter on a door's badge.
pub fn mark(app: &str) -> String {
    match app {
        "email" => "@".into(),
        "whatsapp" => "W".into(),
        _ => app
            .chars()
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default(),
    }
}

/// A badge's colour (`DoorNames.color`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BadgeColor {
    Sky,
    Ok,
    /// `Theme.tafColor` of this index.
    Taf(i64),
}

/// `DoorNames.color`: the same colour on every launch, from the name's letters, not its hash.
pub fn color(app: &str) -> BadgeColor {
    match app {
        "email" => BadgeColor::Sky,
        "whatsapp" => BadgeColor::Ok,
        _ => BadgeColor::Taf(app.chars().map(|c| c as i64).sum()),
    }
}

/// The dial's geometry, apart from drawing it (`DialMath`).
pub mod dial {
    /// The turn between two stops, in degrees.
    pub fn step(stops: usize) -> f64 {
        360.0 / stops.max(1) as f64
    }

    /// Stop `index`'s angle on screen, in degrees (0 is 3 o'clock, clockwise): the top when
    /// the dial is turned to it.
    pub fn angle(index: usize, stops: usize, rotation: f64) -> f64 {
        -90.0 + index as f64 * step(stops) - rotation
    }

    /// The stop at the top for a turn.
    pub fn stop(rotation: f64, stops: usize) -> usize {
        let stops = stops.max(1) as i64;
        let n = (rotation / step(stops as usize)).round() as i64;
        n.rem_euclid(stops) as usize
    }

    /// The turn between two angles, the short way round, in -180...180.
    pub fn delta(from: f64, to: f64) -> f64 {
        let mut d = (to - from) % 360.0;
        if d > 180.0 {
            d -= 360.0;
        }
        if d < -180.0 {
            d += 360.0;
        }
        d
    }

    /// Steps from stop `from` (any turn) to stop `to`, the short way round.
    pub fn shortest(from: i64, to: i64, stops: usize) -> i64 {
        let stops = stops.max(1) as i64;
        let mut d = (to - from).rem_euclid(stops);
        if d as f64 > stops as f64 / 2.0 {
            d -= stops;
        }
        d
    }
}

/// What the doors' own dial says (`DoorDial`, `DoorsView.hint`) with `selected` at the top.
#[derive(Debug, Clone, PartialEq)]
pub struct DialLines {
    /// Each stop's name outside the ring, in capitals; the last is "ADD".
    pub stops: Vec<String>,
    /// Under the taf: "EMAIL · OPEN", or "ADD A DOOR".
    pub center: String,
    /// The centre line in the taf's colour (an open door, or Add), else dim.
    pub center_lit: bool,
    /// Under the dial: tafs cloud's tap words, or what pressing does.
    pub hint: String,
    pub label: String,
    pub value: String,
    pub hint_spoken: String,
}

pub fn dial_lines(
    w: &WordBook,
    doors: &[Door],
    selected: usize,
    taf: &str,
) -> Result<DialLines, WordsError> {
    let stops = doors.len() + 1;
    let n = stops.to_string();
    let door = doors.get(selected);
    let status = |d: &Door| {
        w.line(if d.open {
            "door.status.open"
        } else {
            "door.status.closed"
        })
    };
    let mut names: Vec<String> = doors
        .iter()
        .map(|d| crate::words::upper(&d.label))
        .collect();
    names.push(crate::words::upper(&w.line("door.add_short")?));
    Ok(DialLines {
        stops: names,
        center: match door {
            // In the app's language, as the door's status is.
            Some(d) => format!(
                "{} · {}",
                crate::words::upper(&d.label),
                crate::words::upper(&status(d)?)
            ),
            None => w.line("door.dial.add_stamp")?,
        },
        center_lit: door.map(|d| d.open).unwrap_or(true),
        hint: match door {
            None => w.get("door.press_add_door", &[("taf", taf)])?,
            Some(d) => match &d.words {
                Some(words) => words.tap.clone(),
                None => w.get(
                    if d.open {
                        "door.press_close"
                    } else {
                        "door.press_open"
                    },
                    &[("taf", taf), ("door", &d.label)],
                )?,
            },
        },
        label: w.line("door.door_dial")?,
        value: match door {
            None => w.get("door.dial.add_position", &[("stops", &n), ("stops_2", &n)])?,
            Some(d) => w.get(
                if d.open {
                    "door.dial.position_open"
                } else {
                    "door.dial.position_closed"
                },
                &[
                    ("door", &d.label),
                    ("position", &(selected + 1).to_string()),
                    ("stops", &n),
                ],
            )?,
        },
        hint_spoken: w.line("door.swipe_up_down_pick_door")?,
    })
}

/// A paired sender's row (`DoorsView.panel`'s keys).
#[derive(Debug, Clone, PartialEq)]
pub struct KeyRow {
    pub id: String,
    /// "You" on this person's own key, else its name unless that is only its address.
    pub name: Option<String>,
    /// Unpair, for a key the owner paired, to whoever manages the taf.
    pub unpair: bool,
}

/// One door's panels as `DoorsView.panel` draws them.
#[derive(Debug, Clone, PartialEq)]
pub struct DoorLines {
    pub named: String,
    pub state: String,
    pub address: String,
    /// Copy address, for an email door.
    pub copy: Option<String>,
    /// The meter's fill, and whether it rests (dim) rather than in the taf's colour.
    pub fraction: f64,
    pub resting: bool,
    pub day: String,
    pub others: Option<String>,
    pub keys_title: String,
    pub keys: Vec<KeyRow>,
    /// Under the keys: how it knows who wrote; or, holding none, why no one can give work.
    pub keys_foot: String,
    pub holds_keys: bool,
}

pub fn door_lines(
    w: &WordBook,
    door: &Door,
    taf: &str,
    manages: bool,
    my_email: Option<&str>,
) -> Result<DoorLines, WordsError> {
    let holds = door.holds_keys();
    let mine = |k: &Key| {
        k.auto
            && my_email
                .map(|e| e.to_lowercase() == k.id.to_lowercase())
                .unwrap_or(false)
    };
    let you = w.line("door.key.you")?;
    Ok(DoorLines {
        named: w.get("door.named", &[("door", &door.label)])?,
        state: match &door.words {
            Some(words) => words.state.clone(),
            None => w.line(match (door.open, door.resting) {
                (true, true) => "door.status.resting",
                (true, false) => "door.status.open",
                (false, _) => "door.status.closed",
            })?,
        },
        address: door.address.clone(),
        copy: if door.kind == "email" {
            Some(w.line("door.copy_address")?)
        } else {
            None
        },
        fraction: door.today as f64 / door.limit.max(1) as f64,
        resting: door.resting,
        day: if door.resting {
            let limit = door.limit.to_string();
            w.get(
                "door.paired_senders_today_more_dropped",
                &[("limit", &limit), ("limit_2", &limit)],
            )?
        } else {
            w.get(
                "door.paired_senders_today_text_only",
                &[
                    ("today", &door.today.to_string()),
                    ("limit", &door.limit.to_string()),
                ],
            )?
        },
        others: match (door.others_today, door.others_limit) {
            (Some(o), Some(c)) => Some(w.get(
                "door.anyone_else_today",
                &[("others", &o.to_string()), ("cap", &c.to_string())],
            )?),
            _ => None,
        },
        keys_title: w.line(if holds {
            "door.key.paired"
        } else {
            "door.key.no_one_paired"
        })?,
        keys: if holds {
            door.keys
                .iter()
                .map(|k| KeyRow {
                    id: k.id.clone(),
                    // The address once, when the name is only the address.
                    name: if mine(k) {
                        Some(you.clone())
                    } else {
                        k.name
                            .clone()
                            .filter(|n| n.to_lowercase() != k.id.to_lowercase())
                    },
                    unpair: !k.auto && manages,
                })
                .collect()
        } else {
            Vec::new()
        },
        keys_foot: if holds {
            w.get("door.key.foot", &[("how", &door.how)])?
        } else {
            w.get("door.key.none", &[("how", &door.how), ("taf", taf)])?
        },
        holds_keys: holds,
    })
}

/// The door's day line in a knock's place (`KnockCard`'s `rest`): its messages from the owner
/// and paired senders ("keys") or from anyone else, and how many more were dropped.
pub fn rest_line(w: &WordBook, k: &Knock) -> Result<Option<String>, WordsError> {
    match k.rest.as_deref() {
        None => Ok(None),
        Some("keys") => Ok(Some(w.count("door.knock.rest_keys", k.dropped, &[])?)),
        Some(_) => Ok(Some(w.count("door.knock.rest_others", k.dropped, &[])?)),
    }
}

/// `AddDoorSheet`'s sections: the apps that can tell who's writing, then the rest, each
/// narrowed by the search (`localizedCaseInsensitiveContains`).
pub fn add_sections(addable: &[DoorApp], query: &str) -> (Vec<DoorApp>, Vec<DoorApp>) {
    let q = query.to_lowercase();
    let matches = |a: &&DoorApp| q.is_empty() || a.label.to_lowercase().contains(&q);
    let (proven, rest): (Vec<DoorApp>, Vec<DoorApp>) = addable
        .iter()
        .filter(matches)
        .cloned()
        .partition(|a| a.proof == "proven");
    (proven, rest)
}

/// `PairSheet`'s field: what to write, by the door's kind.
pub fn pair_hint(w: &WordBook, door: &Door) -> Result<String, WordsError> {
    match door.kind.as_str() {
        "email" => w.line("door.pair_hint.email"),
        "whatsapp" => w.line("door.pair_hint.phone"),
        "slack" => w.line("door.pair_hint.slack"),
        _ => w.get("door.pair_hint.other", &[("door", &door.label)]),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    const DOORS: &str = r#"{
      "doors": [{"id":"d1","app":"email","kind":"email","label":"Email","open":true,
        "address":"rae@doors.example","proof":"proven","how":"It checks the sender",
        "keys":[{"id":"me@example.com","auto":true},{"id":"x","name":7}],
        "today":2,"limit":50,"resting":false}],
      "apps": [{"id":"email","label":"Email","kind":"email","proof":"proven","how":"It checks the sender"}],
      "knocks": [
        {"id":"k1","door":"d1","app":"email","from":"ana@example.com","name":"Ana Ruiz","proof":"pass",
         "subject":"Refund","text":"Can I get $15 back?","attachments":2,"at":"2026-11-16T09:00:00.120Z"},
        {"id":"k2","door":"d1","app":"email","from":"bo@example.com","proof":"fail","text":"Hi",
         "at":"2026-11-16T09:05:00Z","attachments":"many"},
        {"id":"k3","door":"d1","app":"whatsapp","from":"+33600000000","proof":"none","text":"Order?",
         "at":"2026-11-16T09:06:00Z","paired":true},
        {"id":"k4","door":"d1","app":"email","from":"door","at":"2026-11-16T09:07:00Z","rest":"others","dropped":3},
        {"id":"k5","door":"d1","app":"email","from":"cy@example.com","at":"2026-11-16T08:00:00Z","state":"done"}
      ]
    }"#;

    #[test]
    fn doors_read_as_the_ipad_decodes_them() {
        let d = read(DOORS.as_bytes()).unwrap();
        assert_eq!(d.doors[0].keys[1].name, None);
        assert_eq!(d.knocks.len(), 5);
        assert_eq!(d.knocks[1].attachments, 0);
        assert_eq!(d.knocks[2].proof, "none");
        assert_eq!(d.knocks[3].state, "open");
        // A door without what the iPad needs: none read.
        assert!(read(
            DOORS
                .replace(r#""resting":false"#, r#""resting":"no""#)
                .as_bytes()
        )
        .is_none());
        // A knock without its sender: none read.
        assert!(read(DOORS.replace(r#""from":"bo@example.com","#, "").as_bytes()).is_none());
    }

    #[test]
    fn the_panel_shows_open_messages_strangers_first() {
        let (waiting, paired) = panel(&read(DOORS.as_bytes()).unwrap());
        assert_eq!(
            waiting.iter().map(|k| k.id.as_str()).collect::<Vec<_>>(),
            ["k1", "k2"]
        );
        assert_eq!(
            paired.iter().map(|k| k.id.as_str()).collect::<Vec<_>>(),
            ["k3"]
        );
    }

    #[test]
    fn a_knock_s_lines() {
        let w = words();
        let d = read(DOORS.as_bytes()).unwrap();
        let a = knock_lines(&w, &d.knocks[0]).unwrap();
        assert_eq!(a.head, "● KNOCK · BY EMAIL");
        assert_eq!(
            a.wrote,
            "Ana Ruiz (ana@example.com) wrote about \"Refund\". Nothing was done."
        );
        assert_eq!(
            a.attachments.as_deref(),
            Some("It had 2 attachments, not opened.")
        );
        assert_eq!(a.proof_note, None);
        assert_eq!(a.do_it.as_deref(), Some("Do it"));
        assert_eq!(a.pair.as_deref(), Some("Pair Ana"));
        let b = knock_lines(&w, &d.knocks[1]).unwrap();
        assert_eq!(b.head, "● KNOCK · BY EMAIL · DIDN'T CHECK OUT");
        assert_eq!(b.wrote, "bo@example.com wrote. Nothing was done.");
        assert!(b
            .proof_note
            .unwrap()
            .starts_with("It says it's from bo@example.com"));
        assert_eq!((b.do_it, b.pair), (None, None));
        let c = knock_lines(&w, &d.knocks[2]).unwrap();
        assert_eq!(c.head, "PAIRED · BY WHATSAPP");
        assert_eq!(
            c.paired_line.as_deref(),
            Some("Paired · Do it or Ignore this message")
        );
        assert_eq!(
            c.proof_note.as_deref(),
            Some("This door can't prove who wrote it.")
        );
        assert_eq!(c.pair, None);
    }

    #[test]
    fn do_it_names_the_taf_or_not() {
        let w = words();
        let k = &read(DOORS.as_bytes()).unwrap().knocks[0];
        let named = do_it_lines(&w, k, Some("Rae")).unwrap();
        assert_eq!(named.title, "Tell Rae what to do");
        assert_eq!(named.send, "Send to Rae");
        assert!(named.intro.contains("Rae acts on what you say"));
        let unnamed = do_it_lines(&w, k, None).unwrap();
        assert_eq!(unnamed.title, "Tell your taf what to do");
    }

    #[test]
    fn answers_and_paths() {
        assert_eq!(path("taf a"), "/v1/tafs/taf%20a/doors");
        assert_eq!(knock_path("t", "k/1"), "/v1/tafs/t/doors/knocks/k%2F1");
        assert_eq!(answer_body("ignore", None), br#"{"action":"ignore"}"#);
        assert_eq!(answer_body("do", Some("")), br#"{"action":"do"}"#);
        assert_eq!(
            answer_body("do", Some("Refund it")),
            br#"{"action":"do","say":"Refund it"}"#
        );
        assert!(answered(br#"{"state":"done"}"#));
        assert!(!answered(br#"{}"#));
        assert_eq!(label("slack"), "Slack");
    }

    #[test]
    fn the_dial_lands_on_the_door_at_the_top() {
        // `DoorsTests.testTheDialLandsOnTheDoorAtTheTop`.
        assert_eq!(dial::stop(0.0, 4), 0);
        assert_eq!(dial::stop(90.0, 4), 1);
        assert_eq!(dial::stop(130.0, 4), 1);
        assert_eq!(dial::stop(-90.0, 4), 3);
        assert_eq!(dial::stop(361.0, 4), 0);
        assert_eq!(dial::angle(1, 4, 90.0), -90.0);
        assert_eq!(dial::delta(170.0, -170.0), 20.0);
        assert_eq!(dial::shortest(0, 3, 4), -1);
        assert_eq!(dial::shortest(5, 2, 4), 1);
        assert_eq!(dial::step(0), 360.0);
    }

    #[test]
    fn what_can_be_added_and_what_is_open() {
        let mut d = read(DOORS.as_bytes()).unwrap();
        assert!(d.addable().is_empty());
        d.apps.push(DoorApp {
            id: "whatsapp".into(),
            label: "WhatsApp".into(),
            kind: "whatsapp".into(),
            proof: "proven".into(),
            how: "WhatsApp gives the number".into(),
        });
        d.apps.push(DoorApp {
            id: "sms".into(),
            label: "Text message".into(),
            kind: "sms".into(),
            proof: "none".into(),
            how: "Anyone".into(),
        });
        assert_eq!(
            d.addable()
                .iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
            ["whatsapp", "sms"]
        );
        let (proven, rest) = add_sections(&d.addable(), "");
        assert_eq!((proven.len(), rest.len()), (1, 1));
        let (proven, rest) = add_sections(&d.addable(), "TEXT");
        assert_eq!((proven.len(), rest[0].id.as_str()), (0, "sms"));
        assert_eq!(
            d.open_knocks()
                .iter()
                .map(|k| k.id.as_str())
                .collect::<Vec<_>>(),
            ["k1", "k2", "k3", "k4"]
        );
        // A change's door replaces the one shown, or comes last.
        let mut changed = d.doors[0].clone();
        changed.open = false;
        d.put(changed);
        assert_eq!((d.doors.len(), d.doors[0].open), (1, false));
        let mut other = d.doors[0].clone();
        other.id = "d2".into();
        d.put(other);
        assert_eq!(d.doors[1].id, "d2");
    }

    #[test]
    fn the_owners_changes_and_their_answers() {
        assert_eq!(door_path("t 1", "d/1"), "/v1/tafs/t%201/doors/d%2F1");
        assert_eq!(keys_path("t", "d"), "/v1/tafs/t/doors/d/keys");
        assert_eq!(
            key_path("t", "d", "ana@example.com"),
            "/v1/tafs/t/doors/d/keys/ana%40example.com"
        );
        assert_eq!(add_body("email", &[]), br#"{"app":"email"}"#);
        assert_eq!(
            add_body(
                "slack",
                &[("grant", "g1"), ("channel", "C1"), ("trigger", "tr")]
            ),
            br#"{"app":"slack","channel":"C1","grant":"g1","trigger":"tr"}"#
        );
        assert_eq!(open_body(false), br#"{"open":false}"#);
        assert_eq!(pair_body("  ", "Ana"), None);
        assert_eq!(
            pair_body(" ana@example.com\n", "").unwrap(),
            br#"{"id":"ana@example.com"}"#
        );
        assert_eq!(
            pair_body("ana@example.com", "Ana").unwrap(),
            br#"{"id":"ana@example.com","name":"Ana"}"#
        );
        let door = DOORS
            .split_once(r#""doors": ["#)
            .unwrap()
            .1
            .split_once(
                r#"],
      "apps""#,
            )
            .unwrap()
            .0;
        assert_eq!(
            read_door(format!(r#"{{"door":{door}}}"#).as_bytes())
                .unwrap()
                .address,
            "rae@doors.example"
        );
        assert!(read_door(br#"{"door":{"id":"d1"}}"#).is_none());
        assert!(removed(br#"{"removed":"d1"}"#));
        assert!(!removed(br#"{"removed":true}"#));
    }

    #[test]
    fn a_badge_s_mark_and_colour() {
        assert_eq!(
            (mark("email"), mark("whatsapp"), mark("slack"), mark("")),
            ("@".into(), "W".into(), "S".into(), String::new())
        );
        assert_eq!(color("email"), BadgeColor::Sky);
        assert_eq!(color("whatsapp"), BadgeColor::Ok);
        // "slack": 115 + 108 + 97 + 99 + 107.
        assert_eq!(color("slack"), BadgeColor::Taf(526));
    }

    #[test]
    fn the_dial_s_words() {
        let w = words();
        let d = read(DOORS.as_bytes()).unwrap();
        let on = dial_lines(&w, &d.doors, 0, "Rae").unwrap();
        assert_eq!(on.stops, ["EMAIL", "ADD"]);
        assert_eq!(on.center, "EMAIL · OPEN");
        assert!(on.center_lit);
        assert_eq!(on.hint, "Press Rae to close Email");
        assert_eq!(on.value, "Email door, open, 1 of 2");
        let add = dial_lines(&w, &d.doors, 1, "Rae").unwrap();
        assert_eq!(add.center, "ADD A DOOR");
        assert!(add.center_lit);
        assert_eq!(add.hint, "Press Rae to add a door");
        assert_eq!(add.value, "Add a door, 2 of 2");
        let mut closed = d.doors.clone();
        closed[0].open = false;
        closed[0].words = Some(DoorWords {
            state: "Closed since 9:00".into(),
            tap: "Tap to open the email door.".into(),
            settings: "Below.".into(),
        });
        let c = dial_lines(&w, &closed, 0, "Rae").unwrap();
        assert_eq!(c.center, "EMAIL · CLOSED");
        assert!(!c.center_lit);
        assert_eq!(c.hint, "Tap to open the email door.");
    }

    #[test]
    fn a_door_s_panels() {
        let w = words();
        let mut door = read(DOORS.as_bytes()).unwrap().doors[0].clone();
        door.keys.push(Key {
            id: "Ana@Example.com".into(),
            name: Some("ana@example.com".into()),
            ..Key::default()
        });
        door.keys.push(Key {
            id: "bo@example.com".into(),
            name: Some("Bo".into()),
            ..Key::default()
        });
        let l = door_lines(&w, &door, "Rae", true, Some("ME@example.com")).unwrap();
        assert_eq!(l.named, "Email door");
        assert_eq!(l.state, "Open");
        assert_eq!(l.copy.as_deref(), Some("Copy address"));
        assert_eq!(l.fraction, 2.0 / 50.0);
        assert_eq!(
            l.day,
            "2 of 50 from you and paired senders today. Text only: attachments aren't opened, and the reply says so."
        );
        assert_eq!(l.others, None);
        assert_eq!(l.keys_title, "Paired · can give work");
        // "You" only on this person's own key; a name that is only the address isn't said twice.
        assert_eq!(l.keys[0].name.as_deref(), Some("You"));
        assert!(!l.keys[0].unpair);
        assert_eq!(l.keys[2].name, None);
        assert!(l.keys[2].unpair);
        assert_eq!(l.keys[3].name.as_deref(), Some("Bo"));
        assert!(l
            .keys_foot
            .starts_with("It checks the sender. A message counts"));
        // Someone else's sign-in: their key isn't "You"; someone who doesn't manage can't unpair.
        let other = door_lines(&w, &door, "Rae", false, Some("x@example.com")).unwrap();
        assert_eq!(other.keys[0].name, None);
        assert!(other.keys.iter().all(|k| !k.unpair));
        door.resting = true;
        door.others_today = Some(10);
        door.others_limit = Some(10);
        door.proof = "none".into();
        let r = door_lines(&w, &door, "Rae", true, None).unwrap();
        assert_eq!(r.state, "Resting until tomorrow");
        assert_eq!(
            r.day,
            "50 of 50 from you and paired senders today. More are dropped until tomorrow."
        );
        assert_eq!(
            r.others.as_deref(),
            Some("10 of 10 from anyone else today.")
        );
        assert_eq!(r.keys_title, "No one is paired here");
        assert!(r.keys.is_empty());
        assert!(r.keys_foot.contains("so no one here can give Rae work"));
        assert_eq!(r.copy, Some("Copy address".into()));
        door.kind = "whatsapp".into();
        assert_eq!(door_lines(&w, &door, "Rae", true, None).unwrap().copy, None);
        assert_eq!(
            pair_hint(&w, &door).unwrap(),
            "Phone number, with its country code"
        );
        door.kind = "telegram".into();
        door.label = "Telegram".into();
        assert_eq!(pair_hint(&w, &door).unwrap(), "Who, as Telegram names them");
    }

    #[test]
    fn the_door_s_day_line() {
        let w = words();
        let d = read(DOORS.as_bytes()).unwrap();
        assert_eq!(rest_line(&w, &d.knocks[0]).unwrap(), None);
        assert_eq!(
            rest_line(&w, &d.knocks[3]).unwrap().unwrap(),
            "This door had its messages from people who aren't paired for today. 3 more were dropped."
        );
        let mut keys = d.knocks[3].clone();
        keys.rest = Some("keys".into());
        keys.dropped = 1;
        assert_eq!(
            rest_line(&w, &keys).unwrap().unwrap(),
            "This door had its messages from you and paired senders for today. 1 more was dropped."
        );
    }

    #[test]
    fn every_shipped_language_draws_the_doors() {
        let d = read(DOORS.as_bytes()).unwrap();
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            dial_lines(&w, &d.doors, 0, "Rae").unwrap();
            dial_lines(&w, &d.doors, 1, "Rae").unwrap();
            door_lines(&w, &d.doors[0], "Rae", true, None).unwrap();
            rest_line(&w, &d.knocks[3]).unwrap();
            pair_hint(&w, &d.doors[0]).unwrap();
        }
    }

    #[test]
    fn an_api_door_reads_its_keys_counts_and_last_request() {
        let body = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tafs-app/fixtures/taf-doors-api.json"
        ))
        .unwrap();
        let d = read(&body).unwrap();
        let door = &d.doors[0];
        assert_eq!(door.kind, "api");
        assert_eq!(door.keys.len(), 2);
        assert_eq!(door.keys[0].last4.as_deref(), Some("iXrW"));
        assert_eq!(door.keys[1].last_used_at, None);
        assert_eq!(api::today(door), 37);
        assert_eq!(
            api::endpoint(door),
            "https://cloud.tafs.kiff.dev/v1/doors/api/door_ord7/requests"
        );
        let w = words();
        let last = api::last_request(&w, door).unwrap().unwrap();
        assert!(
            last.starts_with("Last request ") && last.ends_with(" · Done"),
            "{last}"
        );
        assert!(last.contains("2026"), "{last}");
        let used = api::key_line(&w, &door.keys[0]).unwrap();
        assert!(
            used.contains("last used") && used.ends_with("ends …iXrW"),
            "{used}"
        );
        let never = api::key_line(&w, &door.keys[1]).unwrap();
        assert!(
            never.contains("not used yet") && never.ends_with("…Qp0z"),
            "{never}"
        );
        assert_eq!(api::name(&door.keys[0]), "ERP production");
        // Only pausing goes unsigned; another door's changes all do.
        assert!(!api::only_signed(door, "PATCH", false));
        assert!(api::only_signed(door, "PATCH", true));
        assert!(api::only_signed(door, "POST", false));
        assert!(api::only_signed(door, "DELETE", false));
        let mut email = door.clone();
        email.kind = "email".into();
        assert!(!api::only_signed(&email, "DELETE", false));
    }

    #[test]
    fn an_api_door_never_words_a_state_itself() {
        let mut door = read(
            &std::fs::read(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../tafs-app/fixtures/taf-doors-api.json"
            ))
            .unwrap(),
        )
        .unwrap()
        .doors
        .remove(0);
        door.last_request = Some(LastRequest {
            status: Some("done".into()),
            at: Some("2026-11-19T09:42:15Z".into()),
            words: Some("  ".into()),
        });
        assert_eq!(api::last_request(&words(), &door).unwrap(), None);
        door.counts = None;
        door.today = 4;
        assert_eq!(api::today(&door), 4);
        let k = Key {
            id: "key_9".into(),
            name: None,
            auto: false,
            last4: Some("AbCd".into()),
            made_at: Some("not a time".into()),
            last_used_at: None,
        };
        assert_eq!(api::name(&k), "…AbCd");
        assert_eq!(api::when("en", k.made_at.as_deref()), None);
        let ex = api::example("https://x/y");
        assert!(ex.starts_with("curl -X POST https://x/y \\\n"));
        assert!(ex.contains("$TAFS_API_KEY") && !ex.contains("key_"));
    }

    #[test]
    fn every_shipped_language_has_an_api_doors_lines() {
        let body = std::fs::read(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tafs-app/fixtures/taf-doors-api.json"
        ))
        .unwrap();
        let door = read(&body).unwrap().doors.remove(0);
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            for line in [
                api::last_request(&w, &door).unwrap().unwrap(),
                api::key_line(&w, &door.keys[0]).unwrap(),
                api::key_line(&w, &door.keys[1]).unwrap(),
            ] {
                assert!(!line.contains('{'), "{lang}: {line}");
            }
        }
    }
}
