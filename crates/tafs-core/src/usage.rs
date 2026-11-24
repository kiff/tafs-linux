//! Plan & usage (`UsageView`, `BalancePanel`, `CreditsPanel`, `TafCreditsView`): tafs cloud's
//! balance and the credits the tafs used, read only, each line in tafs cloud's words
//! (`TafsCloud.Balance`, `Credits`, `TafCredits`). Read as leniently as the iPad's decoders:
//! a part that can't be read is left out, never the read, but for the few fields each requires.

use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::signin::Answer;
use crate::words::{WordBook, WordsError};

/// `GET /v1/usage/balance` on tafs cloud.
pub const BALANCE_PATH: &str = "/v1/usage/balance";

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::to_string)
}

/// A string that isn't empty.
fn filled(v: &Value, key: &str) -> Option<String> {
    text(v, key).filter(|t| !t.is_empty())
}

fn int(v: &Value, key: &str) -> Option<i64> {
    v.get(key).and_then(Value::as_i64)
}

fn flag(v: &Value, key: &str) -> Option<bool> {
    v.get(key).and_then(Value::as_bool)
}

/// Raise the limit, as offered (`Balance.Offer`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub words: String,
    pub from_cents: Option<i64>,
}

/// What the store may sell (`Balance.Buy`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Buy {
    pub solo: bool,
    pub packs: bool,
    pub first_month_ready: Option<bool>,
    pub coach: Option<String>,
}

impl Buy {
    fn read(v: &Value) -> Option<Buy> {
        if !v.is_object() {
            return None;
        }
        // `true`, or anything set that isn't `false` or null (a list of products), offers it.
        let offered = |k: &str| match v.get(k) {
            Some(Value::Bool(b)) => *b,
            None | Some(Value::Null) => false,
            Some(_) => true,
        };
        Some(Buy {
            solo: offered("solo"),
            packs: offered("packs"),
            first_month_ready: v
                .get("first_month")
                .filter(|f| f.is_object())
                .map(|f| flag(f, "ready").unwrap_or(false)),
            coach: text(v, "coach").filter(|c| !c.trim().is_empty()),
        })
    }
}

/// The balance's line (`Balance.Line`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceLine {
    /// `ok`, `warning` (from 80%) or `waiting`, `ok` when not sent; one the app doesn't know
    /// draws as `ok` (`tone`).
    pub state: String,
    pub why: Option<String>,
    /// 0 to 100, for a bar.
    pub meter_pct: Option<i64>,
    pub words: String,
    pub detail: Option<String>,
    pub warning: Option<String>,
    pub raise: Option<Offer>,
    pub buy: Option<Buy>,
    pub credits_panel: bool,
}

impl BalanceLine {
    fn read(v: &Value) -> Option<BalanceLine> {
        Some(BalanceLine {
            words: text(v, "words")?,
            state: text(v, "state").unwrap_or_else(|| "ok".into()),
            why: text(v, "why"),
            meter_pct: int(v, "meter_pct").map(|p| p.clamp(0, 100)),
            detail: filled(v, "detail"),
            warning: filled(v, "warning"),
            raise: v.get("raise").and_then(|r| {
                Some(Offer {
                    words: text(r, "words")?,
                    from_cents: int(r, "from_cents"),
                })
            }),
            buy: v.get("buy").and_then(Buy::read),
            credits_panel: flag(v, "credits_panel").unwrap_or(true),
        })
    }

    /// Waiting: no new run starts until the balance changes.
    pub fn waiting(&self) -> bool {
        self.state == "waiting"
    }
}

/// The account's balance now (`TafsCloud.Balance`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Balance {
    pub balance: BalanceLine,
    pub plan_line: Option<String>,
    pub plan_tone: String,
    pub home_line: Option<String>,
}

impl Balance {
    /// tafs cloud's answer; none when `balance` or its `words` can't be read.
    pub fn read(body: &[u8]) -> Option<Balance> {
        let v: Value = serde_json::from_slice(body).ok()?;
        Some(Balance {
            balance: BalanceLine::read(v.get("balance")?)?,
            plan_line: filled(&v, "plan_line"),
            plan_tone: text(&v, "plan_tone").unwrap_or_else(|| "ok".into()),
            home_line: filled(&v, "home_line"),
        })
    }
}

/// The period a read covers (`Credits.Period`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    pub start: DateTime<FixedOffset>,
    pub end: DateTime<FixedOffset>,
    pub current: bool,
}

fn date(v: &Value, key: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(v.get(key)?.as_str()?).ok()
}

/// One taf's part of the period (`Credits.Taf`); no `taf` for the tafs removed since.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TafLine {
    pub taf: Option<String>,
    pub credits: i64,
    pub runs: i64,
    pub words: String,
}

/// Coach past its fair use (`Credits.Coach`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachLine {
    pub credits: i64,
    pub runs: i64,
    pub words: String,
}

/// What wasn't counted, and why (`Credits.NotCounted`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NotCounted {
    pub runs: i64,
    pub words: String,
    pub lines: Vec<NotCountedLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotCountedLine {
    pub why: String,
    pub runs: i64,
    pub words: String,
}

/// The credits the tafs used in a period (`TafsCloud.Credits`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Credits {
    pub period: Option<Period>,
    pub credits: i64,
    pub runs: i64,
    pub words: String,
    pub tafs: Vec<TafLine>,
    pub coach: Option<CoachLine>,
    pub not_counted: Option<NotCounted>,
    pub note: Option<String>,
}

/// Each element of a list that can be read; the others left out (`LenientItem`).
fn each<T>(v: &Value, key: &str, read: impl Fn(&Value) -> Option<T>) -> Vec<T> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(read).collect())
        .unwrap_or_default()
}

impl Credits {
    /// tafs cloud's answer; none without its `credits` and `words`.
    pub fn read(body: &[u8]) -> Option<Credits> {
        let v: Value = serde_json::from_slice(body).ok()?;
        Some(Credits {
            period: v.get("period").and_then(|p| {
                Some(Period {
                    start: date(p, "start")?,
                    end: date(p, "end")?,
                    current: flag(p, "current").unwrap_or(true),
                })
            }),
            credits: int(&v, "credits")?,
            runs: int(&v, "runs").unwrap_or(0),
            words: text(&v, "words")?,
            tafs: each(&v, "tafs", |t| {
                Some(TafLine {
                    taf: text(t, "taf"),
                    credits: int(t, "credits")?,
                    runs: int(t, "runs")?,
                    words: text(t, "words")?,
                })
            }),
            coach: v.get("coach").and_then(|c| {
                Some(CoachLine {
                    credits: int(c, "credits")?,
                    runs: int(c, "runs")?,
                    words: text(c, "words")?,
                })
            }),
            not_counted: v
                .get("not_counted")
                .filter(|n| n.is_object())
                .map(|n| NotCounted {
                    runs: int(n, "runs").unwrap_or(0),
                    words: text(n, "words").unwrap_or_default(),
                    lines: each(n, "lines", |l| {
                        Some(NotCountedLine {
                            why: text(l, "why")?,
                            runs: int(l, "runs")?,
                            words: text(l, "words")?,
                        })
                    }),
                }),
            note: filled(&v, "note"),
        })
    }

    /// Whether the not-counted panel shows: its words or a line.
    pub fn shows_not_counted(&self) -> Option<&NotCounted> {
        self.not_counted
            .as_ref()
            .filter(|n| !n.words.is_empty() || !n.lines.is_empty())
    }
}

/// One run of a taf's credits (`TafCredits.Run`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub run: String,
    pub at: DateTime<FixedOffset>,
    pub credits: i64,
    pub counted: bool,
    pub why: Option<String>,
    pub words: String,
    pub opens: bool,
}

/// One taf's credits in a period, with its runs, newest first (`TafsCloud.TafCredits`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TafCredits {
    pub credits: i64,
    pub runs: i64,
    pub words: String,
    pub items: Vec<Run>,
    pub next: Option<String>,
}

impl TafCredits {
    pub fn read(body: &[u8]) -> Option<TafCredits> {
        let v: Value = serde_json::from_slice(body).ok()?;
        Some(TafCredits {
            credits: int(&v, "credits")?,
            runs: int(&v, "runs").unwrap_or(0),
            words: text(&v, "words")?,
            items: each(&v, "items", |r| {
                Some(Run {
                    run: text(r, "run")?,
                    at: date(r, "at")?,
                    credits: int(r, "credits")?,
                    counted: flag(r, "counted")?,
                    why: text(r, "why"),
                    words: text(r, "words")?,
                    opens: flag(r, "opens").unwrap_or(false),
                })
            }),
            next: filled(&v, "next"),
        })
    }

    /// The next page added after the runs already read, none twice (`readTafCredits(more:)`).
    pub fn add(&mut self, page: TafCredits) {
        let seen: std::collections::BTreeSet<String> =
            self.items.iter().map(|r| r.run.clone()).collect();
        self.items
            .extend(page.items.into_iter().filter(|r| !seen.contains(&r.run)));
        self.next = page.next;
    }
}

fn escaped(v: &str) -> String {
    url::form_urlencoded::byte_serialize(v.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

/// `GET /v1/usage/credits`, or the period before's.
pub fn credits_path(previous: bool) -> String {
    format!(
        "/v1/usage/credits{}",
        if previous { "?period=previous" } else { "" }
    )
}

/// `GET /v1/usage/credits/tafs/{id}`, with the period before and the page after, each escaped.
pub fn taf_credits_path(taf: &str, previous: bool, after: Option<&str>) -> String {
    let mut query = Vec::new();
    if previous {
        query.push("period=previous".to_string());
    }
    if let Some(a) = after {
        query.push(format!("after={}", escaped(a)));
    }
    let mut p = format!("/v1/usage/credits/tafs/{}", escaped(taf));
    if !query.is_empty() {
        p.push('?');
        p.push_str(&query.join("&"));
    }
    p
}

/// What a tafs cloud read came to: read, none from a tafs cloud without it (404), or why not
/// (`creditsWords`): tafs cloud's own words when it gives them, else "{what} couldn't be read".
pub fn answered<T>(
    w: &WordBook,
    answer: &Answer,
    what: &str,
    read: impl Fn(&[u8]) -> Option<T>,
) -> Result<Option<T>, String> {
    if answer.status == 404 {
        return Ok(None);
    }
    if (200..300).contains(&answer.status) {
        return read(&answer.body)
            .map(Some)
            .ok_or_else(|| crate::signin::plain_words(w, 200).unwrap_or_default());
    }
    let fields: Option<Value> = serde_json::from_slice(&answer.body).ok();
    if let Some(words) = fields.as_ref().and_then(|f| filled(f, "words")) {
        return Err(words);
    }
    let why = crate::signin::refusal(answer, Some(w))
        .words(w)
        .unwrap_or_default();
    Err(w
        .get(
            "common.could_not_read_why",
            &[("what", what), ("why", &why)],
        )
        .unwrap_or(why))
}

/// tafs cloud's tone as a colour (`UsageView.color`): waiting in red, warning in amber, else
/// the calm colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    No,
    Amber,
    Calm,
}

pub fn tone(state: &str) -> Tone {
    match state {
        "waiting" => Tone::No,
        "warning" => Tone::Amber,
        _ => Tone::Calm,
    }
}

/// The sample's balance (`sampleBalance`): Solo, about half the period's credits left.
pub fn sample_balance(w: &WordBook) -> Result<Balance, WordsError> {
    Ok(Balance {
        balance: BalanceLine {
            state: "ok".into(),
            why: None,
            meter_pct: Some(48),
            words: w.line("sample.credits.balance_words")?,
            detail: Some(w.line("sample.credits.balance_detail")?),
            warning: None,
            raise: None,
            buy: None,
            credits_panel: true,
        },
        plan_line: Some(w.line("sample.credits.plan_line")?),
        plan_tone: "ok".into(),
        home_line: None,
    })
}

/// The sample's credits (`sampleCredits`), this period or the one before.
pub fn sample_credits(w: &WordBook, previous: bool) -> Result<Credits, WordsError> {
    let l = |k: &str| w.line(&format!("sample.credits.{k}"));
    let taf = |id: &str, credits, runs, k: &str| -> Result<TafLine, WordsError> {
        Ok(TafLine {
            taf: Some(id.into()),
            credits,
            runs,
            words: l(k)?,
        })
    };
    if previous {
        return Ok(Credits {
            period: None,
            credits: 2310,
            runs: 126,
            words: l("last_period")?,
            tafs: vec![
                taf("sample-support", 1480, 74, "refunds_last")?,
                taf("sample-parcels", 830, 52, "invoices_last")?,
            ],
            coach: None,
            not_counted: None,
            note: Some(l("note")?),
        });
    }
    let line = |why: &str, runs, k: &str| -> Result<NotCountedLine, WordsError> {
        Ok(NotCountedLine {
            why: why.into(),
            runs,
            words: l(k)?,
        })
    };
    Ok(Credits {
        period: None,
        credits: 1240,
        runs: 69,
        words: l("this_period")?,
        tafs: vec![
            taf("sample-support", 820, 41, "refunds_this")?,
            taf("sample-parcels", 420, 28, "invoices_this")?,
        ],
        coach: None,
        not_counted: Some(NotCounted {
            runs: 9,
            words: l("not_counted")?,
            lines: vec![
                line("coach", 6, "not_counted_coach")?,
                line("trial", 2, "not_counted_trials")?,
                line("failed", 1, "not_counted_failed")?,
            ],
        }),
        note: Some(l("note")?),
    })
}

/// Who bills the plan (`KIFFClient.Usage.Billing`): `apple`, `google`, `stripe` (KIFF),
/// `contract`, or none.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Billing {
    pub source: String,
    /// After a move from Solo in the App Store to Company: when Solo still renews with Apple.
    pub apple_solo_until: Option<DateTime<FixedOffset>>,
}

/// A pack of credits as KIFF sells it (`Usage.Store.Pack`): only its id and KIFF's line are
/// drawn here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pack {
    pub id: String,
    pub name: String,
    pub line: String,
    pub once: bool,
}

/// What the app sells (`Usage.Store`); its products are the App Store's.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Store {
    pub packs: Vec<Pack>,
}

impl Store {
    /// `offered(firstMonth:)`: the First month pack first, only while it can be bought, then
    /// the packs that can be bought again.
    pub fn offered(&self, first_month: bool) -> Vec<&Pack> {
        let mut listed: Vec<&Pack> = Vec::new();
        if first_month {
            listed.extend(self.packs.iter().find(|p| p.once));
        }
        listed.extend(self.packs.iter().filter(|p| !p.once));
        listed
    }
}

/// A member's request to the account's admins (`KIFFClient.PackRequest`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackRequest {
    pub id: String,
    pub asked_by: String,
    /// `raise` or `pack`; `pack` when KIFF doesn't say.
    pub kind: String,
    pub pack: Option<String>,
    pub reason: String,
    /// open, approved, declined, withdrawn or expired.
    pub status: String,
    pub created_at: DateTime<FixedOffset>,
    pub expires_at: DateTime<FixedOffset>,
    pub note: Option<String>,
}

/// KIFF's usage read for the account (`KIFFClient.Usage`, `GET /v1/me/usage`): its plan, who
/// bills it, the store and members' requests. Every part reads on its own.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KiffUsage {
    /// `solo`, `team` (Company), `company` (an agreement), `internal`, or none.
    pub plan: Option<String>,
    pub billing: Option<Billing>,
    pub store: Option<Store>,
    /// Every open request for someone who manages billing; this person's own otherwise.
    pub requests: Option<Vec<PackRequest>>,
}

/// `GET /v1/me/usage` at KIFF.
pub const KIFF_USAGE_PATH: &str = "/v1/me/usage";

impl KiffUsage {
    /// KIFF's answer: none only when it isn't an object.
    pub fn read(body: &[u8]) -> Option<KiffUsage> {
        let v: Value = serde_json::from_slice(body).ok()?;
        if !v.is_object() {
            return None;
        }
        Some(KiffUsage {
            plan: text(&v, "plan"),
            billing: v.get("billing").filter(|b| b.is_object()).map(|b| Billing {
                source: text(b, "source").unwrap_or_default(),
                apple_solo_until: date(b, "apple_solo_until"),
            }),
            store: v.get("store").and_then(|s| {
                // `solo_product` is required, as on the iPad.
                text(s, "solo_product")?;
                Some(Store {
                    packs: each(s, "packs", |p| {
                        text(p, "apple_product")?;
                        Some(Pack {
                            id: text(p, "id")?,
                            name: text(p, "name")?,
                            line: text(p, "line")?,
                            once: flag(p, "once").unwrap_or(false),
                        })
                    }),
                })
            }),
            requests: v.get("requests").and_then(Value::as_array).map(|_| {
                each(&v, "requests", |r| {
                    // `reason` and `note` must be strings when they're there.
                    let optional = |k: &str| match r.get(k) {
                        None | Some(Value::Null) => Some(None),
                        Some(Value::String(t)) => Some(Some(t.clone())),
                        Some(_) => None,
                    };
                    Some(PackRequest {
                        id: text(r, "id")?,
                        asked_by: text(r, "asked_by")?,
                        kind: text(r, "kind").unwrap_or_else(|| "pack".into()),
                        pack: text(r, "pack"),
                        reason: optional("reason")?.unwrap_or_default(),
                        status: text(r, "status")?,
                        created_at: date(r, "created_at")?,
                        expires_at: date(r, "expires_at")?,
                        note: optional("note")?,
                    })
                })
            }),
        })
    }
}

/// `UsageView.billingLine`: who bills the plan, in one line; none when no plan is billed.
pub fn billing_line(w: &WordBook, u: &KiffUsage) -> Result<Option<String>, WordsError> {
    let key = match u.billing.as_ref().map(|b| b.source.as_str()) {
        Some("apple") => "billing.by_apple",
        Some("google") => "billing.by_google_play",
        Some("stripe") => "billing.by_kiff",
        Some("contract") => "billing.by_agreement",
        _ => return Ok(None),
    };
    w.line(key).map(Some)
}

/// `UsageView.soloStillRenews`: after a move to Company, while Solo still renews with Apple.
pub fn solo_still_renews(w: &WordBook, u: &KiffUsage) -> Result<Option<String>, WordsError> {
    let Some(b) = &u.billing else { return Ok(None) };
    let (true, Some(until)) = (b.source == "stripe", b.apple_solo_until) else {
        return Ok(None);
    };
    let day = crate::account_time::day(&until);
    w.get(
        "usage.company_billed_kiff_solo_bought",
        &[("time", &long_date(w.language(), day))],
    )
    .map(Some)
}

/// `UsageView.asks`: what someone who doesn't manage billing asks an admin for: a raise on
/// Company (`team`), a pack on Solo while packs are on sale; nothing on others. `true` is a raise.
pub fn asks(u: &KiffUsage) -> Option<bool> {
    match u.plan.as_deref() {
        Some("team") => Some(true),
        Some("solo") => match &u.store {
            Some(s) if !s.offered(false).is_empty() => Some(false),
            _ => None,
        },
        _ => None,
    }
}

/// `UsageView.waiting`: the member's own request still open.
pub fn waiting(u: &KiffUsage, manages_billing: bool) -> Option<&PackRequest> {
    if manages_billing {
        return None;
    }
    u.requests.as_ref()?.iter().find(|r| r.status == "open")
}

/// `UsageView.askedLine`: the member's request while it waits.
pub fn asked_line(w: &WordBook, r: &PackRequest) -> Result<String, WordsError> {
    w.line(if r.kind == "raise" {
        "billing.ask.asked_raise"
    } else {
        "billing.ask.asked_pack"
    })
}

/// `UsageView.lastAnswer`: the member's latest answered request.
pub fn last_answer(
    w: &WordBook,
    u: &KiffUsage,
    manages_billing: bool,
) -> Result<Option<String>, WordsError> {
    if manages_billing {
        return Ok(None);
    }
    let Some(r) = u.requests.as_ref().and_then(|r| r.first()) else {
        return Ok(None);
    };
    let raise = r.kind == "raise";
    Ok(Some(match r.status.as_str() {
        "approved" => w.line(if raise {
            "billing.ask.raised"
        } else {
            "billing.ask.added_pack"
        })?,
        "declined" => match r.note.as_deref() {
            Some(note) if !note.is_empty() => {
                w.get("billing.ask.declined_note", &[("note", note)])?
            }
            _ => w.line("billing.ask.declined")?,
        },
        "expired" => w.line("billing.ask.expired")?,
        _ => return Ok(None),
    }))
}

/// `UsageView.openRequests`: the open requests, for someone who manages billing.
pub fn open_requests(u: &KiffUsage, manages_billing: bool) -> Vec<&PackRequest> {
    if !manages_billing {
        return Vec::new();
    }
    u.requests
        .iter()
        .flatten()
        .filter(|r| r.status == "open")
        .collect()
}

/// `UsageView.packLine`: a pack as KIFF sells it; "a pack of credits" without one, its id
/// capitalised when it isn't on sale.
pub fn pack_line(w: &WordBook, id: Option<&str>, u: &KiffUsage) -> Result<String, WordsError> {
    let Some(id) = id else {
        return w.line("usage.pack_credits");
    };
    Ok(u.store
        .as_ref()
        .and_then(|s| s.packs.iter().find(|p| p.id == id))
        .map(|p| p.line.clone())
        .unwrap_or_else(|| capitalized(id)))
}

/// Swift's `capitalized`: each word's first letter up, the rest down.
fn capitalized(s: &str) -> String {
    let mut out = String::new();
    let mut start = true;
    for c in s.chars() {
        if c.is_alphanumeric() {
            if start {
                out.extend(c.to_uppercase());
            } else {
                out.extend(c.to_lowercase());
            }
            start = false;
        } else {
            out.push(c);
            start = true;
        }
    }
    out
}

/// `UsageView.requestLine`: an open request, as someone who manages billing reads it.
pub fn request_line(w: &WordBook, r: &PackRequest, u: &KiffUsage) -> Result<String, WordsError> {
    if r.kind == "raise" {
        return w.line("billing.ask.someone_raise");
    }
    w.get(
        "billing.ask.someone_pack",
        &[("pack", &pack_line(w, r.pack.as_deref(), u)?)],
    )
}

/// What a member asks for (`askAdmin`), unsigned as the session: a raise, or a pack by its id,
/// with the reason they gave (its first 500 characters, trimmed). None when it asks for nothing.
pub fn ask_request(
    raise: bool,
    pack: Option<&str>,
    reason: &str,
) -> Option<crate::signed::Unsigned> {
    let reason: String = reason.chars().take(500).collect();
    let mut body = serde_json::Map::new();
    body.insert("reason".into(), Value::String(reason.trim().to_string()));
    if raise {
        body.insert("kind".into(), Value::String("raise".into()));
    } else {
        body.insert("pack".into(), Value::String(pack?.to_string()));
    }
    crate::signed::Unsigned::new(
        "POST",
        PACK_REQUESTS_PATH.into(),
        serde_json::to_vec(&body).ok()?,
    )
}

/// `POST /v1/me/pack-requests` at KIFF.
pub const PACK_REQUESTS_PATH: &str = "/v1/me/pack-requests";

/// The member takes their request back (`withdrawRequest`), unsigned as the session.
pub fn withdraw_request(id: &str) -> Option<crate::signed::Unsigned> {
    crate::signed::Unsigned::new(
        "POST",
        format!("{PACK_REQUESTS_PATH}/{}/withdraw", escaped(id)),
        b"{}".to_vec(),
    )
}

/// What the security key's sheet says before Raise the limit's touch: tafs cloud's offer (the new
/// limit, "Raise the limit to €300") and the limit it's raised from ("The spend limit is €200
/// now."), `from_cents` in euros. None without the limit it's raised from: nothing is raised then.
pub fn raise_lines(w: &WordBook, offer: &Offer) -> Result<Option<Vec<String>>, WordsError> {
    let Some(from) = offer
        .from_cents
        .and_then(|c| crate::money::Money::new(c, Some("EUR"), 2))
    else {
        return Ok(None);
    };
    let amount = from.formatted(w)?;
    Ok(Some(vec![
        offer.words.clone(),
        w.get("usage.raise_limit.from", &[("amount", &amount)])?,
    ]))
}

/// Declining a member's request (`declineRequest`) with the admin's note to them, which the
/// device key signs once the security key checked the person (`device_signed`).
pub fn decline_act(r: &PackRequest, note: &str) -> crate::signed::Act {
    crate::signed::Act::DeclineRequest {
        request: r.id.clone(),
        note: note.trim().to_string(),
    }
}

/// `offeredPacks`: the packs a member may ask for, the First month pack first while tafs cloud
/// says it can be bought.
pub fn offered_packs<'a>(u: &'a KiffUsage, balance: Option<&Balance>) -> Vec<&'a Pack> {
    let first_month = balance
        .and_then(|b| b.balance.buy.as_ref())
        .and_then(|b| b.first_month_ready)
        == Some(true);
    u.store
        .as_ref()
        .map(|s| s.offered(first_month))
        .unwrap_or_default()
}

/// `AskAdminView`'s words: Ask an admin, what asking does (for a raise or a pack), Which pack,
/// Why (optional) and its hint, Ask (Asking… while it's sent), and Cancel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AskSheet {
    pub title: String,
    pub line: String,
    pub which_pack: String,
    pub why: String,
    pub hint: String,
    pub ask: String,
    pub asking: String,
    pub cancel: String,
}

pub fn ask_sheet(w: &WordBook, raise: bool) -> Result<AskSheet, WordsError> {
    Ok(AskSheet {
        title: w.line("usage.requests.ask_admin")?,
        line: w.line(if raise {
            "billing.ask.line_raise"
        } else {
            "billing.ask.line_pack"
        })?,
        which_pack: w.line("usage.requests.which_pack")?,
        why: w.line("usage.requests.why_optional")?,
        hint: w.line("billing.what_credits")?,
        ask: w.line("usage.requests.ask")?,
        asking: w.line("usage.requests.asking")?,
        cancel: w.line("common.cancel")?,
    })
}

/// Whether Ask may be tapped: the plan asks for something, a pack is chosen when it's a pack,
/// and nothing is on its way.
pub fn can_ask(raise: Option<bool>, pack: Option<&str>, sending: bool) -> bool {
    match raise {
        None => false,
        Some(true) => !sending,
        Some(false) => pack.is_some() && !sending,
    }
}

/// `DeclineRequestView`'s words: its question, Note (optional) and its hint, Decline (Declining…
/// while it's sent), and Cancel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclineSheet {
    pub title: String,
    pub question: String,
    pub note: String,
    pub hint: String,
    pub decline: String,
    pub cancel: String,
}

pub fn decline_sheet(
    w: &WordBook,
    r: &PackRequest,
    u: &KiffUsage,
) -> Result<DeclineSheet, WordsError> {
    Ok(DeclineSheet {
        title: w.line("coach.approval.decline")?,
        question: if r.kind == "raise" {
            w.line("billing.request.decline_raise_title")?
        } else {
            w.get(
                "billing.request.decline_pack_title",
                &[("pack", &pack_line(w, r.pack.as_deref(), u)?)],
            )?
        },
        note: w.line("usage.requests.note_optional")?,
        hint: w.line("usage.requests.note_hint")?,
        decline: w.line("coach.approval.decline")?,
        cancel: w.line("common.cancel")?,
    })
}

/// Why asking didn't go (`notSent`), or withdrawing didn't (`notWithdrawn`).
pub fn not_sent(w: &WordBook, why: &str) -> Result<String, WordsError> {
    w.get("billing.request.not_sent", &[("why", why)])
}

pub fn not_withdrawn(w: &WordBook, why: &str) -> Result<String, WordsError> {
    w.get("billing.request.not_withdrawn", &[("why", why)])
}

/// `Date.shown(date: .long)`: the long date in each shipped language ("December 31, 2026",
/// "31 décembre 2026").
pub fn long_date(language: &str, day: chrono::NaiveDate) -> String {
    crate::locale::long_date(language, day)
}

/// Where Company (its id `team`) is bought on the web: KIFF's checkout, which signs the person
/// in first.
pub const TEAM_CHECKOUT: &str = "https://kiff.dev/dashboard/checkout";

/// Where a computer buys: Solo and credit packs only in the tafs app on the
/// phone, through its store, and Company on kiff.dev. kiff.dev sells neither Solo nor credit
/// packs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buying {
    /// "Buy Solo or credits in the tafs app on your phone."
    pub on_phone: String,
    /// The web plan's link (`billing.team_on_web`), to `TEAM_CHECKOUT`.
    pub team: String,
}

/// Shown where the iPad sells (`BalancePanel`'s Buy credits and Solo): signed in with KIFF, when
/// tafs cloud's balance offers packs or Solo. Nothing is bought on this computer.
pub fn buying(w: &WordBook, buy: Option<&Buy>) -> Result<Option<Buying>, WordsError> {
    if !buy.is_some_and(|b| b.packs || b.solo) {
        return Ok(None);
    }
    Ok(Some(Buying {
        on_phone: w.line("billing.buy_on_phone")?,
        team: w.line("billing.team_on_web")?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_balance_reads_leniently() {
        let b = Balance::read(
            br#"{"balance":{"words":"Solo","meter_pct":140,"detail":"","state":"warning","raise":{"nope":1},"buy":{"solo":["x"],"packs":false,"coach":"  "}},"plan_tone":null}"#,
        )
        .unwrap();
        assert_eq!(b.balance.meter_pct, Some(100));
        assert_eq!(b.balance.detail, None);
        assert_eq!(b.balance.raise, None);
        let buy = b.balance.buy.unwrap();
        assert!(buy.solo && !buy.packs && buy.coach.is_none());
        assert_eq!(b.plan_tone, "ok");
        assert!(b.balance.credits_panel);
        assert_eq!(Balance::read(br#"{"balance":{"state":"ok"}}"#), None);
    }

    #[test]
    fn credits_leave_out_what_cant_be_read() {
        let c = Credits::read(
            br#"{"credits":5,"words":"W","period":{"start":"x","end":"y"},"tafs":[{"taf":"a","credits":1,"runs":1,"words":"A"},{"credits":"bad"}],"not_counted":{"lines":[{"why":"trial","runs":2,"words":"T"},{}]},"note":""}"#,
        )
        .unwrap();
        assert_eq!(c.period, None);
        assert_eq!(c.tafs.len(), 1);
        assert_eq!(c.not_counted.as_ref().unwrap().lines.len(), 1);
        assert!(c.shows_not_counted().is_some());
        assert_eq!(c.note, None);
        assert_eq!(Credits::read(br#"{"words":"W"}"#), None);
    }

    #[test]
    fn a_tafs_runs_add_a_page_once() {
        let page = |runs: &[&str], next: Option<&str>| TafCredits {
            credits: 1,
            runs: 1,
            words: "w".into(),
            items: runs
                .iter()
                .map(|r| Run {
                    run: r.to_string(),
                    at: DateTime::parse_from_rfc3339("2026-11-15T10:00:00Z").unwrap(),
                    credits: 1,
                    counted: true,
                    why: None,
                    words: r.to_string(),
                    opens: false,
                })
                .collect(),
            next: next.map(str::to_string),
        };
        let mut known = page(&["a", "b"], Some("c1"));
        known.add(page(&["b", "c"], None));
        let runs: Vec<_> = known.items.iter().map(|r| r.run.as_str()).collect();
        assert_eq!(runs, ["a", "b", "c"]);
        assert_eq!(known.next, None);
    }

    #[test]
    fn paths_escape_their_parts() {
        assert_eq!(credits_path(true), "/v1/usage/credits?period=previous");
        assert_eq!(
            taf_credits_path("a/b c", true, Some("x&y=z")),
            "/v1/usage/credits/tafs/a%2Fb%20c?period=previous&after=x%26y%3Dz"
        );
        assert_eq!(
            taf_credits_path("t", false, None),
            "/v1/usage/credits/tafs/t"
        );
    }

    #[test]
    fn tones() {
        assert_eq!(tone("waiting"), Tone::No);
        assert_eq!(tone("warning"), Tone::Amber);
        assert_eq!(tone("anything"), Tone::Calm);
    }

    #[test]
    fn kiffs_usage_reads_each_part_alone() {
        let u = KiffUsage::read(
            br#"{"plan":"solo","billing":{"source":"stripe","apple_solo_until":"2026-12-03T10:00:00Z"},"store":{"solo_product":"s","packs":[{"id":"rayo","name":"Rayo","line":"Rayo, 1,000 credits","apple_product":"p"},{"id":"bad"}]},"requests":[{"id":"r1","asked_by":"u","status":"open","created_at":"2026-11-01T00:00:00Z","expires_at":"2026-11-30T00:00:00Z"},{"id":"r2"}]}"#,
        )
        .unwrap();
        assert_eq!(u.store.as_ref().unwrap().packs.len(), 1);
        let requests = u.requests.as_ref().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].kind, "pack");
        assert_eq!(asks(&u), Some(false));
        assert!(waiting(&u, false).is_some() && waiting(&u, true).is_none());
        assert_eq!(open_requests(&u, true).len(), 1);
        assert!(open_requests(&u, false).is_empty());
        // A store without its Solo product isn't read; the rest still is.
        let u = KiffUsage::read(br#"{"plan":"team","store":{"packs":[]},"billing":"x"}"#).unwrap();
        assert_eq!(u.store, None);
        assert_eq!(u.billing, None);
        assert_eq!(asks(&u), Some(true));
        assert_eq!(KiffUsage::read(b"[]"), None);
    }

    #[test]
    fn the_first_month_pack_is_listed_first_and_only_while_offered() {
        let pack = |id: &str, once| Pack {
            id: id.into(),
            name: id.into(),
            line: id.into(),
            once,
        };
        let s = Store {
            packs: vec![pack("a", false), pack("first", true), pack("b", false)],
        };
        let ids = |f| {
            s.offered(f)
                .iter()
                .map(|p| p.id.clone())
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(true), ["first", "a", "b"]);
        assert_eq!(ids(false), ["a", "b"]);
    }

    #[test]
    fn capitalized_is_swifts() {
        assert_eq!(capitalized("rayo"), "Rayo");
        assert_eq!(capitalized("big-PACK two"), "Big-Pack Two");
    }

    #[test]
    fn long_dates() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 12, 3).unwrap();
        assert_eq!(long_date("en", d), "December 3, 2026");
        assert_eq!(long_date("fr", d), "3 décembre 2026");
        assert_eq!(long_date("de", d), "3. Dezember 2026");
        assert_eq!(long_date("pt", d), "3 de dezembro de 2026");
        assert_eq!(long_date("pl", d), "3 grudnia 2026");
    }

    #[test]
    fn asking_an_admin_and_withdrawing_are_sent_unsigned() {
        let raise = ask_request(true, None, "  For the month end  ").unwrap();
        assert_eq!(
            (raise.method(), raise.path()),
            ("POST", "/v1/me/pack-requests")
        );
        let body: Value = serde_json::from_slice(raise.body()).unwrap();
        assert_eq!(body["kind"], "raise");
        assert_eq!(body["reason"], "For the month end");
        assert!(body.get("pack").is_none());
        let pack = ask_request(false, Some("rayo"), &"x".repeat(600)).unwrap();
        let body: Value = serde_json::from_slice(pack.body()).unwrap();
        assert_eq!(body["pack"], "rayo");
        assert_eq!(body["reason"].as_str().unwrap().len(), 500);
        assert!(body.get("kind").is_none());
        // A pack request names its pack.
        assert_eq!(ask_request(false, None, ""), None);
        let w = withdraw_request("r 1").unwrap();
        assert_eq!(
            (w.method(), w.path(), w.body()),
            ("POST", "/v1/me/pack-requests/r%201/withdraw", &b"{}"[..])
        );
        assert!(can_ask(Some(true), None, false));
        assert!(!can_ask(Some(true), None, true));
        assert!(!can_ask(Some(false), None, false));
        assert!(can_ask(Some(false), Some("rayo"), false));
        assert!(!can_ask(None, Some("rayo"), false));
    }

    #[test]
    fn raising_and_declining_are_signed() {
        let u = KiffUsage::read(
            br#"{"plan":"solo","store":{"solo_product":"s","packs":[{"id":"rayo","name":"Rayo","line":"Rayo, 1,000 credits","apple_product":"p"}]},"requests":[{"id":"r1","asked_by":"u","kind":"pack","pack":"rayo","status":"open","created_at":"2026-11-01T00:00:00Z","expires_at":"2026-11-30T00:00:00Z"}]}"#,
        )
        .unwrap();
        let r = &u.requests.as_ref().unwrap()[0];
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let d = decline_sheet(&w, r, &u).unwrap();
        assert_eq!(d.question, "Decline the request for Rayo, 1,000 credits?");
        assert_eq!(
            (d.title.as_str(), d.decline.as_str()),
            ("Decline", "Decline")
        );
        for act in [
            decline_act(r, ""),
            crate::signed::Act::RaiseLimit { from_cents: 1 },
        ] {
            let (method, path) = act.route();
            assert!(crate::signed::Unsigned::new(method, path, vec![]).is_none());
        }
        let a = ask_sheet(&w, false).unwrap();
        assert_eq!(a.title, "Ask an admin");
        assert_eq!(a.hint, "What the credits are for");
        assert_eq!(offered_packs(&u, None).len(), 1);
    }

    #[test]
    fn raising_shows_the_new_limit_and_the_one_it_is_raised_from() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let offer = Offer {
            words: "Raise the limit to €300".into(),
            from_cents: Some(20_000),
        };
        assert_eq!(
            raise_lines(&w, &offer).unwrap(),
            Some(vec![
                "Raise the limit to €300".to_string(),
                "The spend limit is €200 now.".to_string()
            ])
        );
        // No limit to raise from: nothing to raise.
        let none = Offer {
            from_cents: None,
            ..offer
        };
        assert_eq!(raise_lines(&w, &none).unwrap(), None);
    }

    #[test]
    fn a_computer_says_where_to_buy_never_sells() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let offer = |solo, packs| Buy {
            solo,
            packs,
            ..Buy::default()
        };
        let b = buying(&w, Some(&offer(true, false))).unwrap().unwrap();
        assert_eq!(
            b.on_phone,
            "Buy Solo or credits in the tafs app on your phone."
        );
        // The web plan's link, in the shared words (Company on kiff.dev).
        assert_eq!(b.team, w.line("billing.team_on_web").unwrap());
        assert!(buying(&w, Some(&offer(false, true))).unwrap().is_some());
        // Nothing offered, or no offer at all: no lines.
        assert_eq!(buying(&w, Some(&offer(false, false))).unwrap(), None);
        assert_eq!(buying(&w, None).unwrap(), None);
        assert_eq!(TEAM_CHECKOUT, "https://kiff.dev/dashboard/checkout");
        // Every shipped language has both lines.
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            assert!(
                buying(&w, Some(&offer(true, true))).unwrap().is_some(),
                "{lang}"
            );
        }
    }
}
