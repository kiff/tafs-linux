//! An app event's filters before its subscription is signed (`AppEventApprovalSheet`): the
//! dial's stop as tafs cloud sends it (`TafsCloud.Knob.Item`, with its `approval`), and the
//! filters drawn whole, as the iPad's `JSONEncoder` writes them (pretty printed, keys sorted,
//! slashes as they are). The sheet's Approve is `ApprovalGate`-wrapped on the iPad: on this
//! computer the gate's line stands in its place, so nothing is ever signed or subscribed.

use serde_json::{Map, Value};

/// `Knob.Item.Approval`: every field the iPad's synthesized decoder requires, and the
/// subscription once there is one.
#[derive(Debug, Clone, PartialEq)]
pub struct Approval {
    pub state: String,
    pub grant: String,
    pub event: String,
    /// The filters, as tafs cloud sent them (`[String: JSONValue]`).
    pub config: Map<String, Value>,
    pub subscription: Option<String>,
}

/// A stop of the dial (`Knob.Item`), as far as this sheet draws it: its id, kind, label, words
/// and approval. It's read as strictly as the iPad's `init(from:)`: `id` must be text, and a
/// field there of the wrong kind fails the stop (`decodeIfPresent` throws), whether this sheet
/// draws it or not; a field that's absent or null is none.
#[derive(Debug, Clone, PartialEq)]
pub struct Item {
    pub id: String,
    pub kind: Option<String>,
    pub label: String,
    pub words: String,
    pub on: bool,
    pub ok: Option<String>,
    pub approval: Option<Approval>,
}

/// A present, non-null field: `decodeIfPresent` reads null as absent.
fn field<'a>(o: &'a Map<String, Value>, k: &str) -> Option<&'a Value> {
    o.get(k).filter(|v| !v.is_null())
}

fn opt_text(o: &Map<String, Value>, k: &str) -> Result<Option<String>, ()> {
    match field(o, k) {
        None => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(()),
    }
}

/// An `Int`: a JSON number with no fraction that fits.
fn opt_int(o: &Map<String, Value>, k: &str) -> Result<Option<i64>, ()> {
    match field(o, k) {
        None => Ok(None),
        Some(v) => v
            .as_i64()
            .or_else(|| {
                v.as_f64()
                    .filter(|f| f.fract() == 0.0 && f.abs() < 9.2e18)
                    .map(|f| f as i64)
            })
            .map(Some)
            .ok_or(()),
    }
}

fn opt_array<'a>(o: &'a Map<String, Value>, k: &str) -> Result<Option<&'a Vec<Value>>, ()> {
    match field(o, k) {
        None => Ok(None),
        Some(Value::Array(a)) => Ok(Some(a)),
        Some(_) => Err(()),
    }
}

/// `Knob.Setting`'s decoder: `wakes`, `results` and `what` arrays, `busy` text (or `when`'s),
/// `why` text, each when present.
fn setting(v: &Value) -> Result<(), ()> {
    let o = v.as_object().ok_or(())?;
    opt_array(o, "wakes")?;
    if opt_text(o, "busy")?.is_none() {
        if let Some(when) = field(o, "when") {
            // `When` requires its `busy`.
            when.as_object()
                .and_then(|w| w.get("busy"))
                .and_then(Value::as_str)
                .ok_or(())?;
        }
    }
    opt_array(o, "results")?;
    opt_array(o, "what")?;
    opt_text(o, "why")?;
    Ok(())
}

/// `Knob.Item.Size`'s synthesized decoder: its label, index and choices, each choice with its
/// label and setting.
fn size(v: &Value) -> Result<(), ()> {
    let o = v.as_object().ok_or(())?;
    o.get("label").and_then(Value::as_str).ok_or(())?;
    opt_int(o, "index")?.ok_or(())?;
    let choices = o.get("choices").and_then(Value::as_array).ok_or(())?;
    for c in choices {
        let c = c.as_object().ok_or(())?;
        c.get("label").and_then(Value::as_str).ok_or(())?;
        setting(c.get("setting").ok_or(())?)?;
    }
    Ok(())
}

impl Approval {
    fn read(v: &Value) -> Result<Approval, ()> {
        let o = v.as_object().ok_or(())?;
        let text = |k: &str| {
            o.get(k)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or(())
        };
        Ok(Approval {
            state: text("state")?,
            grant: text("grant")?,
            event: text("event")?,
            config: o
                .get("config")
                .and_then(Value::as_object)
                .cloned()
                .ok_or(())?,
            subscription: opt_text(o, "subscription")?,
        })
    }
}

impl From<&crate::knob::Item> for Item {
    /// A stop of the dial as the dial read it, for its sheet.
    fn from(i: &crate::knob::Item) -> Item {
        Item {
            id: i.id.clone(),
            kind: i.kind.clone(),
            label: i.label.clone(),
            words: i.words.clone(),
            on: i.on,
            ok: i.ok.clone(),
            approval: i.approval.as_ref().map(|a| Approval {
                state: a.state.clone(),
                grant: a.grant.clone(),
                event: a.event.clone(),
                config: a.config.clone(),
                subscription: a.subscription.clone(),
            }),
        }
    }
}

impl Item {
    /// One stop, or none when the iPad's decoder would throw on it.
    pub fn read(v: &Value) -> Option<Item> {
        let o = v.as_object()?;
        let item = (|| -> Result<Item, ()> {
            let id = o.get("id").and_then(Value::as_str).ok_or(())?.to_string();
            let kind = opt_text(o, "kind")?;
            opt_text(o, "place")?;
            let label = opt_text(o, "label")?.unwrap_or_default();
            let words = opt_text(o, "words")?.unwrap_or_default();
            let on = match field(o, "on") {
                None => true,
                Some(Value::Bool(b)) => *b,
                Some(_) => return Err(()),
            };
            for k in [
                "state",
                "tap",
                "expires",
                "door",
                "grant",
                "action",
                "look",
                "follow_up",
            ] {
                opt_text(o, k)?;
            }
            // `trigger` is any JSON at all.
            opt_int(o, "step")?;
            let ok = opt_text(o, "ok")?;
            let approval = field(o, "approval").map(Approval::read).transpose()?;
            if let Some(s) = field(o, "size") {
                size(s)?;
            }
            Ok(Item {
                id,
                kind,
                label,
                words,
                on,
                ok,
                approval,
            })
        })();
        item.ok()
    }

    /// The bytes as they came, read as one stop.
    pub fn parse(body: &[u8]) -> Option<Item> {
        Item::read(&serde_json::from_slice(body).ok()?)
    }
}

/// `AppEventApprovalSheet.filters`: the whole of the approval's `config`, as the iPad's
/// `JSONEncoder` with `.prettyPrinted, .sortedKeys, .withoutEscapingSlashes` writes it; nothing
/// without an approval.
pub fn filters(item: &Item) -> String {
    let Some(a) = &item.approval else {
        return String::new();
    };
    let mut out = String::new();
    write_object(&a.config, 0, &mut out);
    out
}

fn indent(n: usize, out: &mut String) {
    for _ in 0..n {
        out.push_str("  ");
    }
}

fn write_object(o: &Map<String, Value>, depth: usize, out: &mut String) {
    out.push_str("{\n");
    let mut keys: Vec<&String> = o.keys().collect();
    keys.sort_by(|a, b| key_order(a, b));
    for (i, k) in keys.iter().enumerate() {
        indent(depth + 1, out);
        write_string(k, out);
        out.push_str(" : ");
        write_value(&o[*k], depth + 1, out);
        if i + 1 < keys.len() {
            out.push(',');
        }
        out.push('\n');
    }
    if keys.is_empty() {
        out.push('\n');
    }
    indent(depth, out);
    out.push('}');
}

fn write_value(v: &Value, depth: usize, out: &mut String) {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        // `JSONValue` keeps every number as a `Double`.
        Value::Number(n) => out.push_str(&number(n.as_f64().unwrap_or(0.0))),
        Value::String(s) => write_string(s, out),
        Value::Array(a) => {
            out.push_str("[\n");
            for (i, e) in a.iter().enumerate() {
                indent(depth + 1, out);
                write_value(e, depth + 1, out);
                if i + 1 < a.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            if a.is_empty() {
                out.push('\n');
            }
            indent(depth, out);
            out.push(']');
        }
        Value::Object(o) => write_object(o, depth, out),
    }
}

/// A string as `JSONEncoder` escapes it: quotes, backslashes and control characters; `/` as it
/// is (`.withoutEscapingSlashes`); everything else as UTF-8.
fn write_string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A `Double` as `JSONEncoder` writes it: Swift's shortest form, with no `.0` on a whole
/// number, and an exponent (`1e+16`, `1e-05`) where Swift's `description` takes one.
pub fn number(f: f64) -> String {
    if !f.is_finite() {
        return "0".into();
    }
    if f == 0.0 {
        return if f.is_sign_negative() { "-0" } else { "0" }.into();
    }
    // Rust's `{:e}` is the shortest form that reads back: digits and a decimal exponent.
    let e = format!("{f:e}");
    let (mantissa, exp) = e.split_once('e').unwrap_or((&e, "0"));
    let exp: i32 = exp.parse().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let sign = if negative { "-" } else { "" };
    if !(-4..16).contains(&exp) {
        let (first, rest) = digits.split_at(1);
        let m = if rest.is_empty() {
            first.to_string()
        } else {
            format!("{first}.{rest}")
        };
        let es = if exp < 0 { '-' } else { '+' };
        return format!("{sign}{m}e{es}{:02}", exp.abs());
    }
    let point = exp + 1;
    let body = if point <= 0 {
        format!("0.{}{digits}", "0".repeat((-point) as usize))
    } else if point as usize >= digits.len() {
        format!("{digits}{}", "0".repeat(point as usize - digits.len()))
    } else {
        let (a, b) = digits.split_at(point as usize);
        format!("{a}.{b}")
    };
    format!("{sign}{body}")
}

/// `.sortedKeys`' order: as Foundation compares keys for it (`.numeric`, `.caseInsensitive`,
/// `.forcedOrdering`): letters without regard to case, runs of digits by their value, and two
/// keys that still tie by their characters.
fn key_order(a: &str, b: &str) -> std::cmp::Ordering {
    use std::cmp::Ordering;
    let (mut x, mut y) = (a.chars().peekable(), b.chars().peekable());
    loop {
        match (x.peek().copied(), y.peek().copied()) {
            (None, None) => return a.cmp(b),
            (None, Some(_)) => return Ordering::Less,
            (Some(_), None) => return Ordering::Greater,
            (Some(c), Some(d)) if c.is_ascii_digit() && d.is_ascii_digit() => {
                let run = |it: &mut std::iter::Peekable<std::str::Chars>| {
                    let mut s = String::new();
                    while let Some(c) = it.peek().copied().filter(char::is_ascii_digit) {
                        s.push(c);
                        it.next();
                    }
                    s
                };
                let (m, n) = (run(&mut x), run(&mut y));
                let (m, n) = (m.trim_start_matches('0'), n.trim_start_matches('0'));
                let o = m.len().cmp(&n.len()).then_with(|| m.cmp(n));
                if o != Ordering::Equal {
                    return o;
                }
            }
            (Some(c), Some(d)) => {
                let o = c.to_lowercase().cmp(d.to_lowercase());
                if o != Ordering::Equal {
                    return o;
                }
                x.next();
                y.next();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The iPad's own test's stop (`AppEventApprovalTests.needed`).
    const NEEDED: &str = r#"{"id":"trg_event_1","kind":"app_event","label":"Support inbox","words":"Listen for new tickets in queue one","on":false,"ok":"approve","approval":{"state":"needed","grant":"g_9","event":"NEW_TICKET","config":{"queue":"one","include_archived":false,"filters":{"tags":["paid","urgent"],"minimum":12.5}}}}"#;

    #[test]
    fn a_stop_with_its_approval_is_read() {
        let item = Item::parse(NEEDED.as_bytes()).unwrap();
        assert_eq!(item.label, "Support inbox");
        assert!(!item.on);
        assert_eq!(item.ok.as_deref(), Some("approve"));
        let a = item.approval.as_ref().unwrap();
        assert_eq!((a.state.as_str(), a.grant.as_str()), ("needed", "g_9"));
        assert_eq!(a.event, "NEW_TICKET");
        assert_eq!(a.subscription, None);
    }

    #[test]
    fn the_dials_stop_is_the_sheets() {
        let knob: crate::knob::Item = serde_json::from_str(NEEDED).unwrap();
        assert_eq!(Item::from(&knob), Item::parse(NEEDED.as_bytes()).unwrap());
    }

    #[test]
    fn the_filters_are_drawn_as_the_ipads_encoder_writes_them() {
        let item = Item::parse(NEEDED.as_bytes()).unwrap();
        assert_eq!(
            filters(&item),
            "{\n  \"filters\" : {\n    \"minimum\" : 12.5,\n    \"tags\" : [\n      \"paid\",\n      \"urgent\"\n    ]\n  },\n  \"include_archived\" : false,\n  \"queue\" : \"one\"\n}"
        );
        // No approval, no filters.
        let bare = Item::parse(br#"{"id":"x"}"#).unwrap();
        assert_eq!(filters(&bare), "");
        assert_eq!(bare.label, "");
        assert!(bare.on);
    }

    #[test]
    fn slashes_stay_and_the_rest_is_escaped() {
        let item = Item::parse(
            br#"{"id":"x","approval":{"state":"needed","grant":"g","event":"E","config":{"url":"https://a.example/b","q":"say \"hi\"\n","e":{},"l":[],"n":null}}}"#,
        )
        .unwrap();
        assert_eq!(
            filters(&item),
            "{\n  \"e\" : {\n\n  },\n  \"l\" : [\n\n  ],\n  \"n\" : null,\n  \"q\" : \"say \\\"hi\\\"\\n\",\n  \"url\" : \"https://a.example/b\"\n}"
        );
    }

    #[test]
    fn numbers_are_doubles_as_swift_writes_them() {
        for (f, s) in [
            (7.0, "7"),
            (12.5, "12.5"),
            (-3.0, "-3"),
            (0.1, "0.1"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (1e15, "1000000000000000"),
            (1e16, "1e+16"),
            (1.5e20, "1.5e+20"),
            (12345678901234567890.0, "1.2345678901234567e+19"),
        ] {
            assert_eq!(number(f), s, "{f}");
        }
    }

    #[test]
    fn keys_sort_as_foundation_sorts_them() {
        let mut keys = vec!["b", "A", "a10", "a2", "B"];
        keys.sort_by(|a, b| key_order(a, b));
        assert_eq!(keys, vec!["A", "a2", "a10", "B", "b"]);
    }

    #[test]
    fn a_field_of_the_wrong_kind_fails_the_stop_as_the_ipad_s_decoder() {
        assert!(Item::parse(br#"{"label":"no id"}"#).is_none());
        assert!(Item::parse(br#"{"id":"x","on":"yes"}"#).is_none());
        assert!(Item::parse(br#"{"id":"x","step":1.5}"#).is_none());
        assert!(Item::parse(br#"{"id":"x","door":3}"#).is_none());
        // An approval without its config, or with one that isn't an object.
        assert!(Item::parse(
            br#"{"id":"x","approval":{"state":"needed","grant":"g","event":"E"}}"#
        )
        .is_none());
        assert!(Item::parse(
            br#"{"id":"x","approval":{"state":"needed","grant":"g","event":"E","config":[]}}"#
        )
        .is_none());
        assert!(Item::parse(
            br#"{"id":"x","size":{"label":"Size","index":0,"choices":[{"label":"S"}]}}"#
        )
        .is_none());
        // Null reads as absent, and any trigger at all is taken.
        let ok = Item::parse(
            br#"{"id":"x","label":null,"trigger":[1,{"a":true}],"step":2,"size":{"label":"Size","index":0,"choices":[{"label":"S","setting":{"when":{"busy":"skip"}}}]}}"#,
        )
        .unwrap();
        assert_eq!(ok.label, "");
    }
}
