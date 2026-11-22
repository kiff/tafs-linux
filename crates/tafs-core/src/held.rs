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
    /// Each argument as text (`ArgumentText`): a string as it is, a number or a flag written
    /// out, anything else "…".
    #[serde(default)]
    pub arguments: Option<BTreeMap<String, Value>>,
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
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => i.to_string(),
            (None, Some(f)) => plain(f),
            _ => "…".into(),
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
    })
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
        // ICU's own data for these two: Mongolian takes the English "th" and Lao Thai's "ที่".
        "mn" => format!("{d}th"),
        "km" => format!("ទី{d}"),
        "az" => format!("{d}'inci"),
        // ICU's Bengali: English's suffixes by CLDR's Bengali ordinal classes.
        "bn" => {
            let suffix = match crate::locale::ordinal_class(language, n) {
                "one" => "st",
                "two" => "nd",
                "few" => "rd",
                _ => "th",
            };
            format!("{d}{suffix}")
        }
        "lo" => format!("ที่\u{200b}{d}"),
        "ar" | "he" => d,
        _ => format!("{d}."),
    }
}

/// `details`: what a gateway call is on, one line an argument, not the amount, not KIFF's own.
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
        .filter_map(|(key, v)| {
            let k = key.to_lowercase();
            let text = argument_text(v);
            (!shown.contains(&k) && !KIFF_OWN_ARGUMENTS.contains(&k.as_str()) && !text.is_empty())
                .then(|| format!("{key}: {text}"))
        })
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

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(all("bn"), "১st ২nd ৩nd ৭st ১১th ২১th ১০০th ১,০০১th");
    }
}
