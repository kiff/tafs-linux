//! Held calls: KIFF's (`GET /v1/me/card-exceptions?status=held`, `KIFFClient.CardException`),
//! as the iPad shows them (`KIFFMapping.held`, `says`, `toolWords`, `moneyReason`,
//! `countReason`, `reasonBreach`, `details`, `amount`, `tafWords`, `outcomeText`).

use std::collections::BTreeMap;

use chrono::{DateTime, Duration, Local, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::card::Window;
use crate::money::Money;
use crate::words::{WordBook, WordsError};

fn time<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
    let s = String::deserialize(d)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

fn optional_time<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    match Option::<String>::deserialize(d)? {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|t| Some(t.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Breach {
    pub outcome: String,
    pub reason: Option<String>,
    pub limit: Option<i64>,
    pub quantity: Option<String>,
    pub unit: Option<String>,
    pub scale: Option<u32>,
    #[serde(default)]
    pub unit_unavailable: Option<bool>,
    #[serde(default)]
    pub unit_ambiguous: Option<bool>,
    #[serde(default)]
    pub used: Option<i64>,
    #[serde(default)]
    pub draw: Option<i64>,
    #[serde(default)]
    pub window: Option<String>,
    #[serde(default)]
    pub per: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ParameterUnit {
    pub unit: String,
    pub scale: u32,
}

/// One held call as KIFF sends it (`CardException`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CardException {
    pub id: String,
    pub subject: String,
    pub action: String,
    pub entity_id: String,
    pub status: String,
    pub breaches: Vec<Breach>,
    pub parameters: Option<BTreeMap<String, i64>>,
    pub reason: Option<String>,
    pub parameter_units: Option<BTreeMap<String, ParameterUnit>>,
    pub units_unavailable: Option<bool>,
    pub next_steps: Option<Vec<String>>,
    #[serde(deserialize_with = "time")]
    pub created_at: DateTime<Utc>,
    #[serde(deserialize_with = "optional_time")]
    pub expires_at: Option<DateTime<Utc>>,
    pub accounting_version: Option<i64>,
    #[serde(default)]
    pub title: Option<String>,
    /// Each argument as KIFF keeps it, whole: Details writes a list or object in full
    /// (`details`, contracts/held-arguments.json).
    #[serde(default, deserialize_with = "held_arguments")]
    pub arguments: Option<BTreeMap<String, Arg>>,
    #[serde(default)]
    pub proposal_id: Option<String>,
    #[serde(default)]
    pub reason_text: Option<String>,
    #[serde(default)]
    pub can_approve_once: Option<bool>,
    /// The taf's own words: one that isn't text is left out, never the hold.
    #[serde(default)]
    pub taf_about: Value,
    #[serde(default)]
    pub taf_reason: Value,
    /// A Chrome step's own block (`browser`): a sealed fill (`sealed_fill`) or a fill (`fill`)
    /// needs the phone's release beside its answer (kiff/tafs docs/chrome.md, "never an allowing
    /// answer without its release"), which this computer, with no approval key, can't make.
    #[serde(default)]
    pub browser: Value,
}

/// `heldCalls`: KIFF's answer, `{"exceptions": […]}`. None when it can't be read.
pub fn read_held(body: &[u8]) -> Option<Vec<CardException>> {
    #[derive(Deserialize)]
    struct Response {
        exceptions: Vec<CardException>,
    }
    serde_json::from_slice::<Response>(body)
        .ok()
        .map(|r| r.exceptions)
}

/// `ArgumentText`.
pub fn argument_text(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => match (n.as_i64(), n.as_u64(), n.as_f64()) {
            (Some(i), _, _) => i.to_string(),
            (None, Some(u), _) => u.to_string(),
            (None, None, Some(f)) => plain(f),
            _ => n.to_string(),
        },
        _ => "…".into(),
    }
}

/// A decimal written out plainly, as sent: "0.000015", never "1.5e-05" (15 significant digits).
fn plain(d: f64) -> String {
    let s = format!("{:.*}", 15, d);
    let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
    if s.is_empty() || s == "-" {
        "0".into()
    } else {
        s
    }
}

/// A held call as the screen shows it (`HeldCall`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldCall {
    pub id: String,
    /// The taf it's held for, by its key (KIFF's agent for a call from KIFF).
    pub taf: String,
    pub summary: String,
    pub amount: Option<Money>,
    /// What it's for, in the sample's words; none from KIFF.
    pub context: String,
    pub reason: String,
    /// When it was held (`heldAt`): where its card goes in its taf's chat.
    pub held_at: DateTime<Local>,
    pub expires_at: DateTime<Local>,
    /// KIFF's id: a call from KIFF is answered in KIFF until this computer signs answers.
    pub kiff_id: Option<String>,
    /// The proposal KIFF held: what its activity files the call under.
    pub proposal_id: Option<String>,
    pub approvable_once: bool,
    pub accounting_version: Option<i64>,
    pub next_steps: Vec<String>,
    pub details: Vec<String>,
    pub taf_about: Option<String>,
    pub taf_reason: Option<String>,
    /// It fills a sign-in in the person's Chrome (`browser.sealed_fill` or `browser.fill`): only
    /// the phone answers it, with its release; this computer's key never does.
    pub needs_release: bool,
    /// The KIFF agent it's held for, the action it would take, and each breach's outcome as KIFF
    /// names it: what "Allow this from now on" is decided from (`allow::allowable`). Empty for a
    /// sample's.
    pub agent: String,
    pub action: String,
    pub breach_outcomes: Vec<String>,
}

/// The arguments KIFF's gateway adds to every call, never listed under Details.
const KIFF_OWN_ARGUMENTS: [&str; 3] = ["kiff_operation_id", "kiff_about", "kiff_reason"];

/// `KIFFMapping.held`.
pub fn held(w: &WordBook, e: &CardException, taf: &str) -> Result<HeldCall, WordsError> {
    let reason = match e.reason_text.as_deref().filter(|r| !r.is_empty()) {
        Some(r) => r.to_string(),
        None => match money_reason(w, e)? {
            Some(r) => r,
            None => match count_reason(w, e)? {
                Some(r) => r,
                None => match reason_breach(e)
                    .and_then(|b| b.reason.clone())
                    .or_else(|| e.reason.clone())
                {
                    Some(r) => r,
                    None => outcome_text(w, reason_breach(e).map(|b| b.outcome.as_str()))?,
                },
            },
        },
    };
    let name = says(&e.action, e.title.as_deref());
    // A gateway call is filed under its tool call's id: what it's on is in its arguments.
    let summary = if e.arguments.is_none() {
        format!("{name} {}", e.entity_id)
    } else {
        name
    };
    Ok(HeldCall {
        id: e.id.clone(),
        taf: taf.to_string(),
        summary,
        amount: amount(e),
        context: String::new(),
        reason,
        held_at: e.created_at.with_timezone(&Local),
        expires_at: e
            .expires_at
            .unwrap_or(e.created_at + Duration::minutes(10))
            .with_timezone(&Local),
        kiff_id: Some(e.id.clone()),
        proposal_id: e.proposal_id.clone(),
        approvable_once: e.can_approve_once.unwrap_or(true),
        accounting_version: e.accounting_version,
        next_steps: e.next_steps.clone().unwrap_or_default(),
        details: details(e),
        taf_about: taf_words(e.taf_about.as_str(), 120),
        taf_reason: taf_words(e.taf_reason.as_str(), 280),
        needs_release: noted_release(&e.id, needs_release(&e.browser)),
        agent: e.subject.clone(),
        action: e.action.clone(),
        breach_outcomes: e.breaches.iter().map(|b| b.outcome.clone()).collect(),
    })
}

/// The held calls read as fills in the person's Chrome, by KIFF's id: `signed::Act::answer`
/// refuses to sign Approve once or Extra room for one of them, whatever draws the button. Shared,
/// since a read may run off the main loop.
static RELEASE_NEEDED: std::sync::Mutex<std::collections::BTreeSet<String>> =
    std::sync::Mutex::new(std::collections::BTreeSet::new());

/// Notes whether the held call `id` needs the phone's release, and says so back.
fn noted_release(id: &str, needs: bool) -> bool {
    if let Ok(mut set) = RELEASE_NEEDED.lock() {
        if needs {
            set.insert(id.to_string());
        } else {
            set.remove(id);
        }
    }
    needs
}

/// Whether the held call `id` was read as a fill in the person's Chrome. A poisoned record says
/// yes: never signed here on a guess.
pub fn release_needed(id: &str) -> bool {
    RELEASE_NEEDED.lock().map_or(true, |set| set.contains(id))
}

/// A held call's `browser` block asks for a fill in the person's Chrome: a sealed fill (a saved
/// password or username) or a plain one. Either needs the phone's release beside the answer. A
/// block that isn't one this app can read counts too: never allowed here on a guess.
pub fn needs_release(browser: &Value) -> bool {
    match browser {
        Value::Null => false,
        Value::Object(o) => ["sealed_fill", "fill"]
            .iter()
            .any(|k| o.get(*k).is_some_and(|v| !v.is_null())),
        _ => true,
    }
}

/// `name`: its tool's title as KIFF gives it, else the action in words.
pub fn name(action: &str, title: Option<&str>) -> String {
    let t = title.unwrap_or("").trim();
    if t.is_empty() {
        tool_words(action)
    } else {
        t.to_string()
    }
}

/// `says`: a held call's name, with its action in words unless the title names its last word.
pub fn says(action: &str, title: Option<&str>) -> String {
    let name = name(action, title);
    let mut words = tool_words(action);
    let title_words: Vec<String> = name
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    let lower = words.to_lowercase();
    let mut parts: Vec<&str> = lower.split(' ').filter(|s| !s.is_empty()).collect();
    // A last word of four letters or digits the title doesn't have is the grant's own: never shown.
    if title.is_some_and(|t| !t.is_empty()) && parts.len() > 1 {
        let last = *parts.last().unwrap();
        if last.chars().count() == 4
            && !title_words.iter().any(|t| t == last)
            && last.chars().all(char::is_alphanumeric)
        {
            parts.pop();
            words = parts.join(" ");
        }
    }
    let Some(last) = parts.last() else {
        return name;
    };
    if title_words.iter().any(|t| t == last) {
        name
    } else {
        format!("{name} · {words}")
    }
}

/// `toolWords`: past a server's prefix and the grant's own, separators as spaces.
pub fn tool_words(tool: &str) -> String {
    let after = tool.rsplit("__").next().unwrap_or(tool);
    let mut name = after.rsplit('.').next().unwrap_or(after).to_string();
    // A grant's prefix: six or more lower-case letters and digits, with a digit, then "_".
    if let Some(i) = name.find('_') {
        let prefix = &name[..i];
        let is_grant = prefix.len() >= 6
            && prefix
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            && prefix.chars().any(|c| c.is_ascii_digit());
        if is_grant && i + 1 < name.len() {
            name = name[i + 1..].to_string();
        }
    }
    let words: Vec<&str> = name.split(['_', '-']).filter(|s| !s.is_empty()).collect();
    if words.is_empty() {
        tool.to_string()
    } else {
        words.join(" ")
    }
}

/// `reasonBreach`: over both a one-call limit and a total, and not approvable once, the total.
pub fn reason_breach(e: &CardException) -> Option<&Breach> {
    if e.can_approve_once == Some(false) {
        if let Some(total) = e
            .breaches
            .iter()
            .find(|b| b.outcome != "per_action_exceeded")
        {
            return Some(total);
        }
    }
    e.breaches.first()
}

/// `moneyReason`: "€150 is over the €100 limit for one call", when KIFF declares the limit's
/// unit and it's the amount's.
pub fn money_reason(w: &WordBook, e: &CardException) -> Result<Option<String>, WordsError> {
    let Some(b) = reason_breach(e) else {
        return Ok(None);
    };
    let (Some(limit), Some(unit)) = (b.limit, b.unit.as_deref()) else {
        return Ok(None);
    };
    if b.unit_ambiguous == Some(true) {
        return Ok(None);
    }
    let Some(amount) = amount(e) else {
        return Ok(None);
    };
    if amount.unit.as_deref() != Some(unit) {
        return Ok(None);
    }
    let Some(cap) = Money::new(limit, Some(unit), b.scale.unwrap_or(amount.scale)) else {
        return Ok(None);
    };
    let cap = cap.formatted(w)?;
    let shown = amount.formatted(w)?;
    Ok(match b.outcome.as_str() {
        "per_action_exceeded" => Some(w.get(
            "card.held.over_one_call",
            &[("amount", &shown), ("cap", &cap)],
        )?),
        "limit_reached" => {
            Some(w.get("card.held.over_total", &[("amount", &shown), ("cap", &cap)])?)
        }
        _ => None,
    })
}

/// `countReason`: a hold over a count of calls, in the Card's words.
pub fn count_reason(w: &WordBook, e: &CardException) -> Result<Option<String>, WordsError> {
    let Some(b) = reason_breach(e) else {
        return Ok(None);
    };
    let Some(limit) = b.limit else {
        return Ok(None);
    };
    if b.outcome != "limit_reached" || b.quantity.as_deref() != Some("count") {
        return Ok(None);
    }
    if limit == 0 {
        return Ok(Some(w.line("card.held.each_call_asks")?));
    }
    let nth = ordinal(w.language(), b.used.unwrap_or(limit) + b.draw.unwrap_or(1));
    let Some(raw) = &b.window else {
        return Ok(Some(w.get(
            "card.over_limit_calls_would",
            &[("limit", &limit.to_string()), ("nth", &nth)],
        )?));
    };
    let each = match b.per.as_deref().filter(|p| !p.is_empty()) {
        Some(p) => w.get(
            "card.held.for_each",
            &[("name", &p.to_lowercase().replace('_', " "))],
        )?,
        None => String::new(),
    };
    let item = match (
        each.is_empty(),
        b.value.as_deref().filter(|v| !v.is_empty()),
    ) {
        (false, Some(v)) => w.get("card.held.for_item", &[("item", v)])?,
        _ => String::new(),
    };
    let window = Window::new(w, raw, None, None)?;
    Ok(Some(w.count(
        "card.held.count_in_window",
        limit,
        &[
            ("window", &window.label),
            ("each", &each),
            ("nth", &nth),
            ("item", &item),
        ],
    )?))
}

/// `Words.ordinal`: the language's ordinal for a whole number (its locale's `.ordinal` style,
/// ICU's digits-ordinal rules), its digits grouped as the language groups them.
pub fn ordinal(language: &str, n: i64) -> String {
    let d = crate::locale::number(n, 0, language);
    match crate::locale::base(language) {
        "en" => {
            let suffix = match (n % 10, n % 100) {
                (_, 11..=13) => "th",
                (1, _) => "st",
                (2, _) => "nd",
                (3, _) => "rd",
                _ => "th",
            };
            format!("{d}{suffix}")
        }
        "fr" if n == 1 => "1er".into(),
        "fr" | "nl" => format!("{d}e"),
        "es" => format!("{d}.º"),
        "it" | "pt" => format!("{d}º"),
        "ja" | "zh" => format!("第{d}"),
        "ko" => format!("{d}번째"),
        "id" => format!("ke-{d}"),
        "vi" => format!("thứ {d}"),
        "th" => format!("ที่\u{a0}{d}"),
        "ro" => format!("{d}a"),
        "bg" => {
            let suffix = if n % 100 == 0 && n % 1000 != 0 {
                "тен"
            } else if (11..=19).contains(&(n % 100)) {
                "и"
            } else {
                match n % 10 {
                    1 => "ви",
                    2 => "ри",
                    3 | 4 => "ти",
                    _ => "и",
                }
            };
            format!("{d}-{suffix}")
        }
        "ca" => {
            let suffix = match n {
                1 | 3 => "r",
                2 => "n",
                4 => "t",
                _ if n % 100 == 1 && n > 100 => "r",
                _ if n % 100 == 2 && n > 100 => "n",
                _ if n % 10 == 1 && n % 100 != 11 && n > 20 => "r",
                _ if n % 10 == 2 && n % 100 != 12 && n > 20 => "n",
                _ => "è",
            };
            format!("{d}{suffix}")
        }
        // Where ICU falls back to English, each language's own, for the Card's lines, which are
        // all about a call. Mongolian: -р, the usual written ordinal.
        "mn" => format!("{d}-р"),
        // Urdu: کال is feminine, so ویں after a number.
        "ur" => format!("{d}ویں"),
        // Turkmen: -njy for CLDR's Turkmen ordinal "few" (n % 10 = 6 or 9, or n = 10), else -nji.
        "tk" => {
            let few = matches!(n % 10, 6 | 9) || n == 10;
            format!("{d}-{}", if few { "njy" } else { "nji" })
        }
        // Hindi: कॉल is feminine ("यह 6ठी कॉल होती"), so 1ली, 2री, 3री, 4थी, 6ठी, every other
        // वीं; 0 has none.
        "hi" => match n {
            0 => d,
            1 => format!("{d}ली"),
            2 | 3 => format!("{d}री"),
            4 => format!("{d}थी"),
            6 => format!("{d}ठी"),
            _ => format!("{d}वीं"),
        },
        "km" => format!("ទី{d}"),
        "az" => format!("{d}'inci"),
        // Bengali: its own suffix by CLDR's Bengali ordinal classes, where ICU writes English's:
        // one (1, 5, 7–10) ম, two (2, 3) য়, few (4) র্থ, many (6) ষ্ঠ, else তম.
        "bn" => {
            let suffix = match crate::locale::ordinal_class(language, n) {
                "one" => "ম",
                "two" => "য়",
                "few" => "র্থ",
                "many" => "ষ্ঠ",
                _ => "তম",
            };
            format!("{d}{suffix}")
        }
        "lo" => format!("ที่\u{200b}{d}"),
        "ar" | "he" => d,
        _ => format!("{d}."),
    }
}

/// `details`: what a gateway call is on, as KIFF keeps its arguments: one line an argument, and a list
/// or object written out in full below its name, each item (1, 2, ...) or field on its own line, two
/// spaces deeper each level, keys in code point order at every level (contracts/held-arguments.json).
/// Nothing the call carries is ever folded to "…": a held browser call's `steps` are what the owner
/// signs (red-team, 27 Nov). Not the amount, not KIFF's own; an empty string at the top says nothing.
pub fn details(e: &CardException) -> Vec<String> {
    let Some(arguments) = &e.arguments else {
        return vec![];
    };
    let mut shown: Vec<String> = e
        .parameter_units
        .as_ref()
        .map(|u| u.keys().map(|k| k.to_lowercase()).collect())
        .unwrap_or_default();
    shown.extend(
        e.parameters
            .as_ref()
            .map(|p| p.keys().map(|k| k.to_lowercase()).collect::<Vec<_>>())
            .unwrap_or_default(),
    );
    arguments
        .iter()
        .flat_map(|(key, v)| {
            let k = key.to_lowercase();
            if shown.contains(&k)
                || KIFF_OWN_ARGUMENTS.contains(&k.as_str())
                || *v == Arg::Text(String::new())
            {
                return vec![];
            }
            argument_lines(&visible(key), v, 0)
        })
        .collect()
}

/// `argumentLines`: `label`'s `value` as lines. A plain value on its label's line; a list's items
/// (1, 2, ...) and an object's fields below it, two spaces deeper each level, keys in code point
/// order (a `BTreeMap` of UTF-8 keys is in code point order). An empty string below the top is
/// written `""`, so a fill with nothing still shows.
pub fn argument_lines(label: &str, v: &Arg, depth: usize) -> Vec<String> {
    let pad = "  ".repeat(depth);
    match v {
        Arg::List(items) if items.is_empty() => vec![format!("{pad}{label}: []")],
        Arg::List(items) => std::iter::once(format!("{pad}{label}:"))
            .chain(
                items
                    .iter()
                    .enumerate()
                    .flat_map(|(i, x)| argument_lines(&(i + 1).to_string(), x, depth + 1)),
            )
            .collect(),
        Arg::Object(fields) if fields.is_empty() => vec![format!("{pad}{label}: {{}}")],
        Arg::Object(fields) => std::iter::once(format!("{pad}{label}:"))
            .chain(
                fields
                    .iter()
                    .flat_map(|(k, x)| argument_lines(&visible(k), x, depth + 1)),
            )
            .collect(),
        Arg::Text(s) if s.is_empty() => vec![format!("{pad}{label}: \"\"")],
        Arg::Text(s) => vec![format!("{pad}{label}: {}", visible(s))],
        Arg::Null => vec![format!("{pad}{label}: null")],
        Arg::Flag(b) => vec![format!("{pad}{label}: {b}")],
        Arg::Number(n) => vec![format!("{pad}{label}: {}", plain_literal(n))],
    }
}

/// A held call's argument as KIFF keeps it, read from its own bytes: a number keeps the text it
/// was sent with (`1E+400` stays `1E+400`, `0.12345678901234567890` keeps every digit), which no
/// `f64`, and not serde_json's own number, does.
#[derive(Debug, Clone, PartialEq)]
pub enum Arg {
    Text(String),
    Number(String),
    Flag(bool),
    Null,
    List(Vec<Arg>),
    /// Fields in code point order; a key sent twice keeps the last, which is what runs.
    Object(BTreeMap<String, Arg>),
}

/// How deep a held call's arguments are read, as serde_json's own limit: past it, the call has
/// no arguments to show (`arguments_unavailable`), never a stack overflow.
const ARGUMENTS_DEPTH_MAX: usize = 128;

/// `arguments`, read from the bytes KIFF sent (`RawValue`). Anything but an object, or past
/// [`ARGUMENTS_DEPTH_MAX`], is refused as before.
fn held_arguments<'de, D: Deserializer<'de>>(
    d: D,
) -> Result<Option<BTreeMap<String, Arg>>, D::Error> {
    let Some(raw) = Option::<Box<serde_json::value::RawValue>>::deserialize(d)? else {
        return Ok(None);
    };
    match read_arguments(raw.get()) {
        Some(Arg::Object(fields)) => Ok(Some(fields)),
        _ => Err(serde::de::Error::custom(
            "arguments: not an object of JSON values",
        )),
    }
}

/// One JSON value from `text`, numbers kept as their text.
pub fn read_arguments(text: &str) -> Option<Arg> {
    let mut r = ArgReader { s: text, i: 0 };
    let v = r.value(0)?;
    r.space();
    (r.i == text.len()).then_some(v)
}

struct ArgReader<'a> {
    s: &'a str,
    i: usize,
}

impl ArgReader<'_> {
    fn space(&mut self) {
        while matches!(
            self.s.as_bytes().get(self.i),
            Some(b' ' | b'\t' | b'\n' | b'\r')
        ) {
            self.i += 1;
        }
    }

    fn take(&mut self, b: u8) -> Option<()> {
        self.space();
        (self.s.as_bytes().get(self.i) == Some(&b)).then(|| self.i += 1)
    }

    fn word(&mut self, w: &str, v: Arg) -> Option<Arg> {
        self.s[self.i..].starts_with(w).then(|| {
            self.i += w.len();
            v
        })
    }

    /// A string, unescaped by serde_json from its own bytes.
    fn text(&mut self) -> Option<String> {
        self.space();
        let b = self.s.as_bytes();
        let start = self.i;
        if b.get(self.i) != Some(&b'"') {
            return None;
        }
        self.i += 1;
        loop {
            match b.get(self.i)? {
                b'\\' => self.i += 2,
                b'"' => break,
                _ => self.i += 1,
            }
        }
        self.i += 1;
        serde_json::from_str(self.s.get(start..self.i)?).ok()
    }

    fn value(&mut self, depth: usize) -> Option<Arg> {
        if depth > ARGUMENTS_DEPTH_MAX {
            return None;
        }
        self.space();
        match *self.s.as_bytes().get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut fields = BTreeMap::new();
                if self.take(b'}').is_some() {
                    return Some(Arg::Object(fields));
                }
                loop {
                    let k = self.text()?;
                    self.take(b':')?;
                    fields.insert(k, self.value(depth + 1)?);
                    if self.take(b',').is_none() {
                        self.take(b'}')?;
                        return Some(Arg::Object(fields));
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                if self.take(b']').is_some() {
                    return Some(Arg::List(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    if self.take(b',').is_none() {
                        self.take(b']')?;
                        return Some(Arg::List(items));
                    }
                }
            }
            b'"' => self.text().map(Arg::Text),
            b't' => self.word("true", Arg::Flag(true)),
            b'f' => self.word("false", Arg::Flag(false)),
            b'n' => self.word("null", Arg::Null),
            b'-' | b'0'..=b'9' => {
                let start = self.i;
                while matches!(
                    self.s.as_bytes().get(self.i),
                    Some(b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E')
                ) {
                    self.i += 1;
                }
                Some(Arg::Number(self.s[start..self.i].to_string()))
            }
            _ => None,
        }
    }
}

/// The most digits a number is written out with. Past it, the number shows as sent, so an
/// exponent can't make the app write a billion zeros (case 7 of the contract).
const PLAIN_DIGITS_MAX: i128 = 100;

/// A JSON number exactly as sent ([`Arg::Number`] keeps its text), written
/// plainly: no exponent, and no trailing zero after the point ("1.50" is 1.5, "2.0" is 2,
/// "1.5e-05" is 0.000015), and every digit kept, past an f64 or an i64 too. One whose plain form
/// would pass 100 digits (the leading 0 and the zeros after the point counted) is written as sent:
/// `1e99` is written out, `1e100` and `1e-400` stay as they are.
pub fn plain_literal(n: &str) -> String {
    let sent = n;
    let (sign, n) = match n.strip_prefix('-') {
        Some(rest) => ("-", rest),
        None => ("", n),
    };
    let (mant, exp) = n.split_once(['e', 'E']).unwrap_or((n, ""));
    let (whole, frac) = mant.split_once('.').unwrap_or((mant, ""));
    let digits = format!("{whole}{frac}");
    let digits = digits.trim_start_matches('0');
    // A zero is one written digit, whatever its exponent ("0e500", and one past i64 too).
    if digits.is_empty() {
        return "0".into();
    }
    let exp = if exp.is_empty() {
        0
    } else {
        match exp.trim_start_matches('+').parse::<i64>() {
            Ok(e) => e,
            Err(_) => return sent.to_string(),
        }
    };
    // The plain form's digit count, worked out before a zero is written.
    let significant = digits.trim_end_matches('0');
    let s = frac.len() as i128 - exp as i128 - (digits.len() - significant.len()) as i128;
    let count = if s <= 0 {
        significant.len() as i128 - s
    } else {
        (significant.len() as i128).max(s + 1)
    };
    if count > PLAIN_DIGITS_MAX {
        return sent.to_string();
    }
    let scale = frac.len() as i64 - exp;
    let text = if scale <= 0 {
        format!("{digits}{}", "0".repeat((-scale) as usize))
    } else {
        let scale = scale as usize;
        let digits = if digits.len() <= scale {
            format!("{}{digits}", "0".repeat(scale - digits.len() + 1))
        } else {
            digits.to_string()
        };
        let (int, dec) = digits.split_at(digits.len() - scale);
        let dec = dec.trim_end_matches('0');
        if dec.is_empty() {
            int.to_string()
        } else {
            format!("{int}.{dec}")
        }
    };
    format!("{sign}{text}")
}

/// `visible`: a line from KIFF or a taf as Details draws it: no direction mark, so nothing can
/// reorder what's beside it, and any other control character (a line break) as a space, so a
/// value can't draw a line of its own.
pub fn visible(text: &str) -> String {
    text.chars()
        .filter(|c| !crate::files::is_direction(*c))
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect()
}

/// `amount(of:)`: a parameter KIFF declares a unit for, in that unit; else the parameter a
/// breach sums, as a bare number; else none.
pub fn amount(e: &CardException) -> Option<Money> {
    let parameters = e.parameters.as_ref().filter(|p| !p.is_empty())?;
    let units: BTreeMap<String, &ParameterUnit> = e
        .parameter_units
        .iter()
        .flatten()
        .map(|(k, v)| (k.to_lowercase(), v))
        .collect();
    if let Some((key, value)) = parameters
        .iter()
        .find(|(k, _)| units.contains_key(&k.to_lowercase()))
    {
        let unit = units[&key.to_lowercase()];
        // A scale that can't be read: no amount shown, never a wrong one.
        return Money::new(*value, Some(&unit.unit), unit.scale);
    }
    let summed: Vec<String> = e
        .breaches
        .iter()
        .filter_map(|b| b.quantity.as_deref())
        .filter_map(summed)
        .map(|s| s.to_lowercase())
        .collect();
    parameters
        .iter()
        .find(|(k, _)| summed.contains(&k.to_lowercase()))
        .and_then(|(_, v)| Money::new(*v, None, 0))
}

/// `summed`: "sum(amount_cents)" → "amount_cents".
fn summed(quantity: &str) -> Option<String> {
    let lower = quantity.to_lowercase();
    (lower.starts_with("sum(") && quantity.ends_with(')'))
        .then(|| quantity[4..quantity.len() - 1].to_string())
}

/// `outcomeText`.
pub fn outcome_text(w: &WordBook, outcome: Option<&str>) -> Result<String, WordsError> {
    w.line(match outcome {
        Some("per_action_exceeded") => "card.outcome.over_one_action",
        Some("limit_reached") => "card.outcome.over_total",
        Some("resource_not_on_card") => "card.outcome.not_a_resource",
        _ => "card.outcome.outside",
    })
}

/// `tafWords`: one plain line, control characters and line breaks as spaces, runs of space as
/// one, cut to KIFF's cap with "…". None when nothing is left.
pub fn taf_words(text: Option<&str>, max: usize) -> Option<String> {
    let flat: String = text?
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    let line = flat
        .split(' ')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    if line.is_empty() {
        return None;
    }
    if line.chars().count() > max {
        Some(line.chars().take(max - 1).collect::<String>() + "…")
    } else {
        Some(line)
    }
}

/// `left`: "9:37 left", "expired" from the end; minutes and seconds, as the iPad's timer.
pub fn remaining(
    w: &WordBook,
    expires: DateTime<Local>,
    now: DateTime<Local>,
) -> Result<String, WordsError> {
    let seconds = (expires - now).num_seconds();
    if seconds < 1 {
        return w.line("needs_you.expired");
    }
    let time = if seconds >= 3600 {
        format!(
            "{}:{:02}:{:02}",
            seconds / 3600,
            (seconds % 3600) / 60,
            seconds % 60
        )
    } else {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    };
    w.get("needs_you.left", &[("time", &time)])
}

/// `continueHeld`'s path (kiff/tafs#206): the run that paused on a held call goes on now that
/// it's answered, as the session. None for a taf tafs cloud doesn't hold (KIFF's agent alone,
/// or one still being set up): no run of its waits there.
pub fn continue_path(taf: &str, exception: &str) -> Option<String> {
    if taf.is_empty()
        || exception.is_empty()
        || taf.starts_with("agent:")
        || taf.starts_with("setting-up:")
    {
        return None;
    }
    Some(format!(
        "/v1/tafs/{}/holds/{}/continue",
        crate::access::escaped(taf),
        crate::access::escaped(exception)
    ))
}

/// What `continueHeld` answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoneOn {
    /// The run goes on, or waits on another of its held calls (its status).
    Goes(String),
    /// 409 (`not_paused`): the run had already gone on, as KIFF's alert often makes it before
    /// this lands. Nothing to say.
    WentOn,
    /// No run waited on the call any more (404 `run_ended`, kiff/tafs#1812): tafs cloud's words,
    /// in the owner's language, that nothing ran.
    RunEnded(String),
    /// Refused otherwise (an older tafs cloud's bare 404 among them): why is said, in tafs
    /// cloud's words or the app's.
    Refused,
}

/// `continueHeld`'s answer, by its status and body.
pub fn read_gone_on(status: u16, body: &[u8]) -> GoneOn {
    #[derive(Deserialize, Default)]
    struct Answer {
        status: Option<String>,
        code: Option<String>,
        words: Option<String>,
    }
    let a = serde_json::from_slice::<Answer>(body).unwrap_or_default();
    match status {
        200..=299 => GoneOn::Goes(a.status.unwrap_or_default()),
        409 => GoneOn::WentOn,
        404 if a.code.as_deref() == Some("run_ended") => match a.words {
            Some(w) if !w.trim().is_empty() => GoneOn::RunEnded(w.trim().to_string()),
            _ => GoneOn::Refused,
        },
        _ => GoneOn::Refused,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_answered_call_goes_on_at_tafs_clouds_own_path() {
        assert_eq!(
            continue_path("depl_1", "ce_1").as_deref(),
            Some("/v1/tafs/depl_1/holds/ce_1/continue")
        );
        assert_eq!(
            continue_path("depl 1", "ce/1").as_deref(),
            Some("/v1/tafs/depl%201/holds/ce%2F1/continue")
        );
        // KIFF's agent alone, or a taf still being set up: no run at tafs cloud waits there.
        assert_eq!(continue_path("agent:ag_1", "ce_1"), None);
        assert_eq!(continue_path("setting-up:Rae", "ce_1"), None);
        assert_eq!(continue_path("depl_1", ""), None);
        assert!(!crate::signed::signed_at_cloud(
            "POST",
            "/v1/tafs/depl_1/holds/ce_1/continue"
        ));
    }

    #[test]
    fn a_run_that_had_ended_says_nothing_ran_in_tafs_clouds_words() {
        let ended = r#"{"error":"not_found","code":"run_ended","words":"Cette exécution était terminée, donc rien ne s'est passé. "}"#;
        assert_eq!(
            read_gone_on(404, ended.as_bytes()),
            GoneOn::RunEnded("Cette exécution était terminée, donc rien ne s'est passé.".into())
        );
        assert_eq!(
            read_gone_on(200, br#"{"run":"r1","status":"PAUSED"}"#),
            GoneOn::Goes("PAUSED".into())
        );
        assert_eq!(
            read_gone_on(200, br#"{"status":"waiting"}"#),
            GoneOn::Goes("waiting".into())
        );
        // The run had already gone on, as KIFF's alert made it: nothing to say.
        assert_eq!(
            read_gone_on(409, br#"{"error":"not_paused"}"#),
            GoneOn::WentOn
        );
        // Refused otherwise, an older tafs cloud's bare 404 and blank words among them: why is
        // said, never swallowed.
        assert_eq!(
            read_gone_on(404, br#"{"error":"not_found"}"#),
            GoneOn::Refused
        );
        assert_eq!(
            read_gone_on(404, br#"{"code":"run_ended","words":"  "}"#),
            GoneOn::Refused
        );
        assert_eq!(
            read_gone_on(403, br#"{"error":"forbidden"}"#),
            GoneOn::Refused
        );
        assert_eq!(read_gone_on(502, b"bad gateway"), GoneOn::Refused);
    }

    #[test]
    fn tool_words_drop_prefixes() {
        assert_eq!(tool_words("bxkwf7vi_list_charges"), "list charges");
        assert_eq!(tool_words("stripe.create_refund"), "create refund");
        assert_eq!(tool_words("kiff__refund_order"), "refund order");
        assert_eq!(tool_words("refund"), "refund");
    }

    #[test]
    fn says_adds_the_action_unless_the_title_names_it() {
        assert_eq!(
            says("create_refund", Some("Refund a payment")),
            "Refund a payment"
        );
        assert_eq!(
            says("stripe_create_refund", Some("Write Stripe API data")),
            "Write Stripe API data · stripe create refund"
        );
        assert_eq!(
            says("create_refund_repg", Some("Refund a payment")),
            "Refund a payment"
        );
        assert_eq!(says("create_refund", None), "create refund");
    }

    #[test]
    fn ordinals_in_each_language() {
        assert_eq!(ordinal("en", 1), "1st");
        assert_eq!(ordinal("en", 2), "2nd");
        assert_eq!(ordinal("en", 11), "11th");
        assert_eq!(ordinal("en", 21), "21st");
        assert_eq!(ordinal("fr", 1), "1er");
        assert_eq!(ordinal("fr", 2), "2e");
        assert_eq!(ordinal("de", 3), "3.");
    }

    #[test]
    fn taf_words_are_one_plain_line() {
        assert_eq!(
            taf_words(Some("Order\n1042\t \u{7}  please"), 120).as_deref(),
            Some("Order 1042 please")
        );
        assert_eq!(taf_words(Some("  \n "), 120), None);
        assert_eq!(taf_words(Some("abcdef"), 4).as_deref(), Some("abc…"));
    }

    #[test]
    fn arguments_as_text() {
        assert_eq!(argument_text(&Value::from(0.000015)), "0.000015");
        assert_eq!(argument_text(&Value::from(42)), "42");
        assert_eq!(argument_text(&Value::from(true)), "true");
        assert_eq!(argument_text(&serde_json::json!({"a": 1})), "…");
    }

    /// A number under Details exactly as sent, written plainly: no exponent, no trailing zero,
    /// every digit (the vector's cases 5 and 6, kiff/tafs#2254 and the next).
    #[test]
    fn a_number_argument_keeps_every_digit_and_drops_trailing_zeros() {
        let Some(Arg::Object(args)) = read_arguments(
            r#"{"price": 1.50, "rate": 2.0, "share": 0.250, "tiny": 1.5e-05, "id_like": 12345678901234567.89,
                "long": 0.12345678901234567890, "big": 98765432109876543210987, "neg": -0.5, "zero": 0.0, "e": 1E+3}"#,
        ) else {
            panic!("not an object")
        };
        let line = |k: &str| argument_lines(k, &args[k], 0).remove(0);
        assert_eq!(line("price"), "price: 1.5");
        assert_eq!(line("rate"), "rate: 2");
        assert_eq!(line("share"), "share: 0.25");
        assert_eq!(line("tiny"), "tiny: 0.000015");
        assert_eq!(line("id_like"), "id_like: 12345678901234567.89");
        assert_eq!(line("long"), "long: 0.1234567890123456789");
        assert_eq!(line("big"), "big: 98765432109876543210987");
        assert_eq!(line("neg"), "neg: -0.5");
        assert_eq!(line("zero"), "zero: 0");
        assert_eq!(line("e"), "e: 1000");
    }

    /// Case 7: a number whose plain form would pass 100 digits shows as sent, so an exponent
    /// can't make Details write a billion zeros (a taf's `1e999999999`).
    #[test]
    fn a_number_past_a_hundred_digits_shows_as_sent() {
        let Some(Arg::Object(args)) = read_arguments(
            r#"{"huge": 1e400, "small": 1e-400, "big_edge": 1e99, "over": 1e100, "neg": -1E+400,
                "small_edge": 1e-99, "small_over": 1e-100, "zeros": 1.000e-99, "vast": 1e999999999,
                "past_i64": 1e99999999999999999999, "long_edge": 1234567890e90}"#,
        ) else {
            panic!("not an object")
        };
        let line = |k: &str| argument_lines(k, &args[k], 0).remove(0);
        assert_eq!(line("huge"), "huge: 1e400");
        assert_eq!(line("small"), "small: 1e-400");
        assert_eq!(line("big_edge"), format!("big_edge: 1{}", "0".repeat(99)));
        assert_eq!(line("over"), "over: 1e100");
        assert_eq!(line("neg"), "neg: -1E+400");
        assert_eq!(
            line("small_edge"),
            format!("small_edge: 0.{}1", "0".repeat(98))
        );
        assert_eq!(line("small_over"), "small_over: 1e-100");
        assert_eq!(line("zeros"), format!("zeros: 0.{}1", "0".repeat(98)));
        assert_eq!(line("vast"), "vast: 1e999999999");
        assert_eq!(line("past_i64"), "past_i64: 1e99999999999999999999");
        assert_eq!(
            line("long_edge"),
            format!("long_edge: 1234567890{}", "0".repeat(90))
        );
    }

    /// Case 7's edges beyond the vector: a zero is 0 however it's sent, an exponent past i64 is as
    /// sent, and a key sent twice shows the last, which is what runs.
    #[test]
    fn a_held_calls_arguments_are_read_from_their_own_bytes() {
        let Some(Arg::Object(args)) = read_arguments(
            r#"{"zero_far": 0e500, "past": 1000e2147483647, "twice": 1, "twice": "last",
                "nested": {"n": 1E+400, "list": [1.50, -0.0, "\u00e9"]}}"#,
        ) else {
            panic!("not an object")
        };
        let line = |k: &str| argument_lines(k, &args[k], 0);
        assert_eq!(line("zero_far"), ["zero_far: 0"]);
        assert_eq!(plain_literal("0e99999999999999999999"), "0");
        assert_eq!(
            plain_literal("1e99999999999999999999"),
            "1e99999999999999999999"
        );
        assert_eq!(line("past"), ["past: 1000e2147483647"]);
        assert_eq!(line("twice"), ["twice: last"]);
        assert_eq!(
            line("nested"),
            [
                "nested:",
                "  list:",
                "    1: 1.5",
                "    2: 0",
                "    3: é",
                "  n: 1E+400"
            ]
        );
        // Too deep to read is no arguments, never a stack overflow; nor is anything but JSON.
        let deep = format!("{}1{}", "[".repeat(200), "]".repeat(200));
        assert_eq!(read_arguments(&deep), None);
        for bad in [r#"{"a": 1"#, r#"{"a" 1}"#, "{} x", r#"{"a": tru}"#, ""] {
            assert_eq!(read_arguments(bad), None, "{bad}");
        }
    }

    /// A held call's arguments under Details, every case of contracts/held-arguments.json: the
    /// lines every app writes, exactly. Nothing is ever shortened to "…" (red-team, 27 Nov: a held
    /// browse_act's steps showed as "steps: …").
    #[test]
    fn a_held_calls_arguments_are_the_contracts_lines_exactly() {
        let file: Value = serde_json::from_str(include_str!(
            "../../../shared/contracts/held-arguments.json"
        ))
        .unwrap();
        for case in file["cases"].as_array().unwrap() {
            let name = case["name"].as_str().unwrap();
            let held = serde_json::json!({"exceptions": [{"id": "ce_c", "subject": "a", "action": "x",
                "entity_id": "e", "status": "held", "breaches": [], "created_at": "2026-11-27T16:00:00Z",
                "expires_at": null, "accounting_version": 1,
                "arguments": case["arguments"]}]});
            let e = &read_held(held.to_string().as_bytes()).unwrap()[0];
            let want: Vec<String> = case["lines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| l.as_str().unwrap().to_string())
                .collect();
            assert_eq!(details(e), want, "{name}");
        }
    }

    #[test]
    fn ordinals_as_icu_writes_them() {
        let all = |l: &str| {
            [1, 2, 3, 7, 11, 21, 100, 1001]
                .iter()
                .map(|n| ordinal(l, *n))
                .collect::<Vec<_>>()
                .join(" ")
        };
        assert_eq!(all("en"), "1st 2nd 3rd 7th 11th 21st 100th 1,001st");
        assert_eq!(all("bg"), "1-ви 2-ри 3-ти 7-и 11-и 21-ви 100-тен 1001-ви");
        assert_eq!(
            all("ko"),
            "1번째 2번째 3번째 7번째 11번째 21번째 100번째 1,001번째"
        );
        assert_eq!(ordinal("zh-Hant", 3), "第3");
        assert_eq!(ordinal("pt-BR", 3), "3º");
        // Turkmen, Hindi and Urdu in their own words, for a call: ICU writes
        // English's suffixes for Turkmen and Urdu, and Hindi's masculine forms.
        let some = |l: &str| {
            [1, 2, 3, 4, 6, 9, 10, 21]
                .iter()
                .map(|n| ordinal(l, *n))
                .collect::<Vec<_>>()
                .join(" ")
        };
        assert_eq!(
            some("tk"),
            "1-nji 2-nji 3-nji 4-nji 6-njy 9-njy 10-njy 21-nji"
        );
        assert_eq!(some("hi"), "1ली 2री 3री 4थी 6ठी 9वीं 10वीं 21वीं");
        assert_eq!(some("ur"), "1ویں 2ویں 3ویں 4ویں 6ویں 9ویں 10ویں 21ویں");
        assert_eq!(some("bn"), "১ম ২য় ৩য় ৪র্থ ৬ষ্ঠ ৯ম ১০ম ২১তম");
        assert_eq!(some("mn"), "1-р 2-р 3-р 4-р 6-р 9-р 10-р 21-р");
        assert_eq!(ordinal("hi", 0), "0");
        assert_eq!(ordinal("th", 3), "ที่\u{a0}3");
        assert_eq!(ordinal("ro", 3), "3a");
        assert_eq!(ordinal("ar", 3), "3");
        assert_eq!(ordinal("cs", 3), "3.");
        assert_eq!(all("ca"), "1r 2n 3r 7è 11è 21r 100è 1.001r");
        assert_eq!(ordinal("ca", 4), "4t");
        assert_eq!(ordinal("ca", 112), "112è");
        assert_eq!(ordinal("km", 3), "ទី3");
        assert_eq!(ordinal("mk", 3), "3.");
        assert_eq!(ordinal("az", 21), "21'inci");
        assert_eq!(ordinal("sr-Latn", 3), "3.");
        assert_eq!(ordinal("he", 3), "3");
        assert_eq!(all("bn"), "১ম ২য় ৩য় ৭ম ১১তম ২১তম ১০০তম ১,০০১তম");
    }
}
