//! The Record (`RecordView`): every call each taf made, by day, newest first. KIFF's activity
//! (`GET /v1/me/activity`, `KIFFClient.ActivityCall`) as the iPad shows it (`KIFFMapping.record`,
//! `executed`, `counted`), and the Record's own rules (`RecordFilter`, `RecordEntry`'s lines,
//! `RecordView.spent`, `dayTitle`).

use chrono::{DateTime, Local, Utc};
use serde::{Deserialize, Deserializer};
use sha2::{Digest, Sha256};

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
pub struct ToolOutcome {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub refused_by: Option<String>,
    #[serde(default)]
    pub code: Option<String>,
}

/// The money a call moves, as KIFF writes it: a decimal string and its unit.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Amount {
    pub value: String,
    #[serde(default)]
    pub unit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Answer {
    pub decision: String,
    #[serde(default)]
    pub by: Option<String>,
    #[serde(default)]
    pub you: Option<bool>,
    /// Signed by the answerer's device, and which (`signed`, `device`).
    #[serde(default)]
    pub signed: Option<bool>,
    #[serde(default)]
    pub device: Option<String>,
    /// When it was answered (`at`): where its outcome goes in its taf's chat.
    #[serde(default, deserialize_with = "optional_time")]
    pub at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Receipt {
    pub trace_id: String,
    pub status: String,
}

/// One call as KIFF's activity lists it (`ActivityCall`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ActivityCall {
    pub proposal_id: String,
    pub action_name: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub actor_id: Option<String>,
    pub outcome: String,
    #[serde(deserialize_with = "time")]
    pub created_at: DateTime<Utc>,
    /// Required by the iPad's decoder, but tafs cloud's Activity entries (`taf-activity.json`)
    /// send none: absent reads as not held, so that answer reads.
    #[serde(default)]
    pub held: bool,
    #[serde(default)]
    pub tool_outcome: Option<ToolOutcome>,
    #[serde(default)]
    pub amount: Option<Amount>,
    #[serde(default)]
    pub read_only: Option<bool>,
    #[serde(default)]
    pub entity_id: Option<String>,
    #[serde(default)]
    pub on_hash: Option<String>,
    #[serde(default)]
    pub on_arg: Option<String>,
    #[serde(default)]
    pub answer: Option<Answer>,
    #[serde(default)]
    pub receipt: Option<Receipt>,
    #[serde(default)]
    pub reasons: Option<Vec<String>>,
    #[serde(default, deserialize_with = "optional_time")]
    pub made_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

/// `TafsCloud.CallRef`: what one of a taf's calls through KIFF acted on (kiff/tafs#167),
/// KIFF's key for the call and the value of the argument its Card scopes on.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct CallRef {
    pub invocation: String,
    pub value: String,
}

/// `TafsCloud.calls`: tafs cloud's `GET /v1/tafs/{id}/calls`, decoded as the iPad does (every
/// ref whole, or none).
pub fn read_calls(body: &[u8]) -> Option<Vec<CallRef>> {
    #[derive(Deserialize)]
    struct Response {
        calls: Vec<CallRef>,
    }
    serde_json::from_slice::<Response>(body)
        .ok()
        .map(|r| r.calls)
}

/// `calls`' path for a taf, by its tafs cloud id.
pub fn calls_path(id: &str) -> String {
    format!("/v1/tafs/{}/calls", crate::work::segment(id))
}

/// `nameRecordLines`: the tafs, by KIFF agent, with a call KIFF hashed a resource for: only
/// theirs are read.
pub fn agents_to_name(calls: &[ActivityCall]) -> std::collections::BTreeSet<String> {
    calls
        .iter()
        .filter(|c| c.on_hash.is_some())
        .filter_map(|c| c.actor_id.clone())
        .collect()
}

/// `KIFFMapping.naming`: the Record's lines with what each acted on, from the tafs' call refs
/// by KIFF's key; only a value whose hash is KIFF's, so a line never names a payment KIFF didn't.
pub fn naming(
    mut record: Vec<Entry>,
    refs: &std::collections::BTreeMap<String, String>,
) -> Vec<Entry> {
    for e in &mut record {
        if let (Some(key), Some(hash)) = (&e.entity_id, &e.on_hash) {
            if let Some(value) = refs.get(key) {
                if crate::receipt_view::is_on(value, hash) {
                    e.on_value = Some(value.clone());
                }
            }
        }
    }
    record
}

/// `activity`: KIFF's answer, `{"calls": […]}`. None when it can't be read.
pub fn read_activity(body: &[u8]) -> Option<Vec<ActivityCall>> {
    #[derive(Deserialize)]
    struct Response {
        calls: Vec<ActivityCall>,
    }
    serde_json::from_slice::<Response>(body)
        .ok()
        .map(|r| r.calls)
}

/// What became of a call (`RecordEntry.Kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Done,
    Approved,
    Refused,
    Held,
    Unknown,
    Revoked,
    Started,
    NotDone,
    Sending,
    InTafs,
}

/// A kind's colour (`Kind.color`): the theme's ok, amber, no, or dim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Ok,
    Amber,
    No,
    Dim,
}

impl Kind {
    pub fn tone(self) -> Tone {
        match self {
            Kind::Done | Kind::Approved | Kind::Started => Tone::Ok,
            Kind::Held | Kind::Unknown | Kind::Sending => Tone::Amber,
            Kind::Refused | Kind::Revoked | Kind::NotDone => Tone::No,
            Kind::InTafs => Tone::Dim,
        }
    }
}

/// One line of the Record (`RecordEntry`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// `recordID`: the same on every refresh.
    pub id: String,
    /// The taf, by its key; KIFF's agent for a call from KIFF.
    pub taf: String,
    pub kind: Kind,
    /// KIFF's name for the call's tool (`action`): what a connection's last failure is found by.
    pub action: Option<String>,
    pub title: String,
    pub detail: String,
    pub at: DateTime<Utc>,
    /// When the call was made (`madeAt`), when KIFF says.
    pub made_at: Option<DateTime<Utc>>,
    pub owner_approved: bool,
    pub amount: Option<Amount>,
    pub read_only: bool,
    /// KIFF's key for the call (`entityID`), which tafs cloud's call refs are kept by.
    pub entity_id: Option<String>,
    /// What KIFF hashed the call acted on (`onHash`).
    pub on_hash: Option<String>,
    pub on_arg: Option<String>,
    /// What it acted on, shown only when its hash is KIFF's (`naming`): none from KIFF alone.
    pub on_value: Option<String>,
    pub answer: Option<Answer>,
    pub receipt: Option<Receipt>,
    /// KIFF's proposal id, for whether a held call still waits.
    pub proposal: Option<String>,
    /// KIFF's words for why (`reason`), never its codes.
    pub reason: Option<String>,
    /// KIFF allowed the call and the vendor refused it, and it still counts against the Card's
    /// totals (`refusedByTool`).
    pub refused_by_tool: bool,
    /// Who refused it at the tool (`refusedBy`): "the connection" for one the connection refused
    /// before sending, else the vendor as connected, when known.
    pub refused_by: Option<String>,
    /// "Report this payment": where tafs cloud offers it, and whether it's reported.
    pub report: crate::payment_report::Reportable,
}

/// `heldKey`: a held call's key in its taf's chat, the id its Record line has
/// (`recordID(proposalID ?? kiffID)`), so its card and what became of it are one row; a call with
/// neither keeps its own id.
pub fn held_key(proposal_id: Option<&str>, kiff_id: Option<&str>, id: &str) -> String {
    match proposal_id.or(kiff_id) {
        Some(p) => record_id(p),
        None => id.to_string(),
    }
}

/// `recordEntry(forCall:)`: the Record's line for one of a taf's calls, by KIFF's key for it.
pub fn entry_for_call<'a>(entries: &'a [Entry], key: &str) -> Option<&'a Entry> {
    entries.iter().find(|e| e.entity_id.as_deref() == Some(key))
}

/// `recordID`: KIFF's call id as a stable key.
pub fn record_id(proposal_id: &str) -> String {
    let digest = Sha256::digest(format!("tafs.record|{proposal_id}").as_bytes());
    digest.iter().take(16).map(|b| format!("{b:02x}")).collect()
}

/// What a write's amount counts on the Card's totals (`counted`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Counted {
    Money,
    Amount,
    Nothing,
}

pub fn counted(c: &ActivityCall) -> Counted {
    match (&c.amount, c.read_only) {
        (Some(a), r) if r != Some(true) => {
            if is_currency(a.unit.as_deref()) {
                Counted::Money
            } else {
                Counted::Amount
            }
        }
        _ => Counted::Nothing,
    }
}

/// `isCurrency`: an ISO 4217 code: three letters, upper-case once read.
pub fn is_currency(unit: Option<&str>) -> bool {
    unit.is_some_and(|u| u.len() == 3 && u.chars().all(|c| c.is_ascii_alphabetic()))
}

/// `executed`: what the tool did with an allowed call.
pub fn executed(
    w: &WordBook,
    o: Option<&ToolOutcome>,
    approved: bool,
    vendor: Option<&str>,
    counts: Counted,
) -> Result<(Kind, String), WordsError> {
    let why = o
        .and_then(|o| o.message.as_deref())
        .filter(|m| !m.is_empty())
        .map(|m| format!(": {m}"))
        .unwrap_or_default();
    let allowed = w.line(if approved {
        "record.approved_by_you"
    } else {
        "record.allowed_by_card"
    })?;
    Ok(match o.map(|o| o.status.as_str()) {
        Some("done") => {
            if approved {
                (Kind::Approved, w.line("record.approved_then_done")?)
            } else {
                (Kind::Done, w.line("record.done_within_card")?)
            }
        }
        Some("refused_by_tool") => {
            let who = match vendor {
                Some(v) => v.to_string(),
                None => w.line("record.the_tool")?,
            };
            match counts {
                Counted::Money => (
                    Kind::NotDone,
                    w.get("record.tool_refused_money", &[("who", &who)])?,
                ),
                Counted::Amount => (
                    Kind::NotDone,
                    w.get("record.tool_refused_amount", &[("who", &who)])?,
                ),
                Counted::Nothing => (
                    Kind::NotDone,
                    w.get(
                        "record.tool_refused_counts_why",
                        &[("who", &who), ("why", &why)],
                    )?,
                ),
            }
        }
        Some("not_sent") => (
            Kind::NotDone,
            w.get(
                "record.not_sent_nothing_reached_why",
                &[("allowed", &allowed), ("why", &why)],
            )?,
        ),
        Some("sending") => (
            Kind::Sending,
            w.get("record.sent_waiting", &[("allowed", &allowed)])?,
        ),
        Some("unknown") => (
            Kind::Unknown,
            w.get("record.answer_lost", &[("allowed", &allowed)])?,
        ),
        // not_recorded, none, or one this app doesn't know yet.
        _ => (
            Kind::Unknown,
            w.get("record.not_recorded", &[("allowed", &allowed)])?,
        ),
    })
}

/// `KIFFMapping.record`: one entry a call of a holder the app shows, by its taf's key
/// (`taf_of` from KIFF's actor id). `vendors` names the vendor each connected tool is from, by
/// its name in KIFF (`toolVendors`), for a call the vendor refused.
pub fn record(
    w: &WordBook,
    calls: &[ActivityCall],
    taf_of: &dyn Fn(&str) -> Option<String>,
    vendors: &std::collections::BTreeMap<String, String>,
) -> Result<Vec<Entry>, WordsError> {
    let mut out = Vec::new();
    for c in calls {
        let Some(taf) = c.actor_id.as_deref().and_then(taf_of) else {
            continue;
        };
        // Approved once only when KIFF carries the owner's approval as the call's answer (kiff/tafs#2225): a held
        // call that went through because its Card changed has none, and is the Card's, as KIFF's own trail says.
        let approved_once = c.held && c.answer.as_ref().is_some_and(|a| a.decision != "refused");
        let (kind, mut detail) = match c.outcome.as_str() {
            "allowed" => executed(
                w,
                c.tool_outcome.as_ref(),
                approved_once,
                vendors.get(&c.action_name).map(String::as_str),
                counted(c),
            )?,
            "owner_required" | "approval_required" => {
                (Kind::Held, w.line("record.waiting_for_you")?)
            }
            // KIFF's own words for why, when it has them.
            _ => match &c.reason {
                Some(r) => (
                    Kind::Refused,
                    w.get("record.not_sent_because", &[("reason", r)])?,
                ),
                None => (Kind::Refused, w.line("record.not_sent")?),
            },
        };
        let tool = c.tool_outcome.as_ref();
        let refused_by_tool =
            c.outcome == "allowed" && tool.is_some_and(|t| t.status == "refused_by_tool");
        if refused_by_tool
            && tool
                .is_some_and(|t| t.refused_by.as_deref() == Some("connection") && t.code.is_some())
        {
            detail = match &c.detail {
                Some(d) => d.clone(),
                None => w.line("record.connection_refused_before_sending")?,
            };
        }
        out.push(Entry {
            id: record_id(&c.proposal_id),
            taf,
            kind,
            action: Some(c.action_name.clone()),
            title: crate::held::name(&c.action_name, c.title.as_deref()),
            detail,
            at: c.created_at,
            made_at: c.made_at,
            owner_approved: c.outcome == "allowed" && approved_once,
            amount: c.amount.clone(),
            read_only: c.read_only == Some(true),
            entity_id: c.entity_id.clone(),
            on_hash: c.on_hash.clone(),
            on_arg: c.on_arg.clone(),
            on_value: None,
            answer: c.answer.clone(),
            receipt: c.receipt.clone(),
            proposal: Some(c.proposal_id.clone()),
            reason: c.reason.clone().filter(|r| !r.trim().is_empty()),
            refused_by_tool,
            refused_by: if !refused_by_tool {
                None
            } else if tool.is_some_and(|t| t.refused_by.as_deref() == Some("connection")) {
                Some(w.line("record.the_connection")?)
            } else {
                vendors.get(&c.action_name).cloned()
            },
            report: Default::default(),
        });
    }
    Ok(out)
}

/// The Record's filters (`RecordFilter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    MoneyMoved,
    Stopped,
    Waiting,
}

impl Filter {
    pub const ALL: [Filter; 4] = [
        Filter::All,
        Filter::MoneyMoved,
        Filter::Stopped,
        Filter::Waiting,
    ];

    pub fn title(self, w: &WordBook) -> Result<String, WordsError> {
        w.line(match self {
            Filter::All => "record.filter.all",
            Filter::MoneyMoved => "record.filter.money_moved",
            Filter::Stopped => "record.filter.stopped",
            Filter::Waiting => "record.filter.waiting",
        })
    }

    pub fn keeps(self, e: &Entry) -> bool {
        match self {
            Filter::All => true,
            Filter::MoneyMoved => {
                e.amount.is_some() && matches!(e.kind, Kind::Done | Kind::Approved)
            }
            Filter::Stopped => matches!(e.kind, Kind::Refused | Kind::NotDone | Kind::Revoked),
            Filter::Waiting => matches!(e.kind, Kind::Held | Kind::Sending | Kind::Unknown),
        }
    }
}

impl Entry {
    /// `money`: the amount as money, when its unit is a currency and its value a decimal.
    pub fn money(&self) -> Option<Money> {
        let a = self.amount.as_ref()?;
        let unit = a.unit.as_deref()?;
        if !(unit.len() == 3 && unit.chars().all(|c| c.is_ascii_uppercase())) {
            return None;
        }
        let (minor, scale) = decimal(&a.value)?;
        Money::new(minor, Some(unit), scale)
    }

    /// `amountText`: "€150.00" as money everywhere, the unit as written when it isn't a currency.
    pub fn amount_text(&self, w: &WordBook) -> Result<Option<String>, WordsError> {
        let Some(a) = &self.amount else {
            return Ok(None);
        };
        if decimal(&a.value).is_none() {
            return Ok(None);
        }
        match a.unit.as_deref().filter(|u| !u.is_empty()) {
            None => Ok(Some(a.value.clone())),
            Some(unit) => match self.money() {
                Some(m) => m.formatted(w).map(Some),
                None => Ok(Some(format!("{} {unit}", a.value))),
            },
        }
    }

    /// `titleWithAmount`: "Save a dispute response · $800".
    pub fn title_with_amount(&self, w: &WordBook) -> Result<String, WordsError> {
        Ok(match self.amount_text(w)? {
            Some(m) => format!("{} · {m}", self.title),
            None => self.title.clone(),
        })
    }

    /// `onText`: "payment …QbstR", when tafs has the value KIFF's hash names.
    pub fn on_text(&self, w: &WordBook) -> Result<Option<String>, WordsError> {
        let Some(value) = &self.on_value else {
            return Ok(None);
        };
        let noun = match &self.on_arg {
            Some(a) => a.clone(),
            None => w.line("record.on.it")?,
        }
        .replace('_', " ");
        let n = value.chars().count();
        Ok(Some(if n > 6 {
            let tail: String = value.chars().skip(n - 5).collect();
            format!("{noun} …{tail}")
        } else {
            format!("{noun} {value}")
        }))
    }

    /// `answerer`: "you", or KIFF's role for them.
    pub fn answerer(&self, w: &WordBook) -> Result<String, WordsError> {
        let Some(answer) = &self.answer else {
            return w.line("record.answerer.someone_else");
        };
        if answer.you == Some(true) {
            return w.line("record.answerer.you");
        }
        match answer.by.as_deref().filter(|r| !r.is_empty()) {
            None => w.line("record.answerer.someone_else"),
            Some(
                role
                @ ("admin" | "approver" | "editor" | "member" | "manager" | "owner" | "viewer"),
            ) => w.line(&format!("people.role_phrase.{role}")),
            Some(other) => w.get(
                "people.role_phrase.other",
                &[("role", &other.replace('_', " "))],
            ),
        }
    }

    /// `decidedLine`.
    pub fn decided_line(&self, w: &WordBook) -> Result<String, WordsError> {
        if let Some(answer) = &self.answer {
            let who = self.answerer(w)?;
            return w.get(
                if answer.decision == "refused" {
                    "record.decided.refused_by"
                } else {
                    "record.decided.approved_by"
                },
                &[("who", &who)],
            );
        }
        w.line(match self.kind {
            Kind::Refused => "record.decided.refused_by_card",
            Kind::Held => "record.decided.waiting_for_you",
            Kind::Revoked => "card.card_revoked",
            Kind::Started => "record.decided.card_issued",
            _ if self.owner_approved => "record.decided.approved_by_you",
            _ => "record.decided.within_card",
        })
    }

    /// `resultLine`.
    pub fn result_line(&self, w: &WordBook) -> Result<String, WordsError> {
        if self.kind == Kind::InTafs {
            return Ok(String::new());
        }
        w.line(match self.kind {
            Kind::Done | Kind::Approved => "record.result.done",
            Kind::Refused | Kind::Revoked => "record.nothing_sent",
            Kind::NotDone => "record.result.not_done",
            Kind::Held => "record.result.nothing_sent_yet",
            Kind::Sending => "record.result.no_answer_yet",
            Kind::Unknown => "record.result.unknown",
            _ => "record.result.started",
        })
    }

    /// `RecordRow.detail`: what it acted on, then who decided and what the tool did, else its
    /// own line. Held, and KIFF no longer holds it: no longer waiting, nothing sent.
    pub fn row_detail(&self, w: &WordBook, waiting: bool) -> Result<String, WordsError> {
        let mut parts = Vec::new();
        if let Some(on) = self.on_text(w)? {
            parts.push(on);
        }
        if self.kind == Kind::Held && !waiting {
            parts.push(w.line("record.no_longer_waiting")?);
            parts.push(w.line("record.nothing_sent")?);
        } else if self.receipt.is_some() || self.answer.is_some() {
            parts.push(self.decided_line(w)?);
            parts.push(self.result_line(w)?);
        } else {
            parts.push(self.detail.clone());
        }
        Ok(parts.join(" · "))
    }
}

/// A decimal string as minor units and its scale: "150.00" → (15000, 2). None when it isn't one.
pub fn decimal(value: &str) -> Option<(i64, u32)> {
    let (sign, digits) = match value.strip_prefix('-') {
        Some(rest) => (-1, rest),
        None => (1, value),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole.chars().all(|c| c.is_ascii_digit())
        || !fraction.chars().all(|c| c.is_ascii_digit())
    {
        return None;
    }
    let scale = fraction.len() as u32;
    if scale > crate::money::MAX_SCALE {
        return None;
    }
    let minor: i64 = format!("{whole}{fraction}").parse().ok()?;
    Some((sign * minor, scale))
}

/// `RecordView.spent`: "€640 spent", a sum for each currency, of the money that moved.
pub fn spent(w: &WordBook, entries: &[&Entry]) -> Result<Option<String>, WordsError> {
    let mut sums: Vec<Money> = Vec::new();
    for e in entries.iter().filter(|e| Filter::MoneyMoved.keeps(e)) {
        let Some(m) = e.money() else { continue };
        match sums.iter_mut().find(|s| s.unit == m.unit) {
            Some(sum) => {
                let scale = sum.scale.max(m.scale);
                let at = |x: &Money| x.minor.checked_mul(10i64.checked_pow(scale - x.scale)?);
                let (Some(a), Some(b)) = (at(sum), at(&m)) else {
                    continue;
                };
                let Some(total) = a.checked_add(b) else {
                    continue;
                };
                *sum = Money {
                    minor: total,
                    unit: m.unit.clone(),
                    scale,
                };
            }
            None => sums.push(m),
        }
    }
    if sums.is_empty() {
        return Ok(None);
    }
    let figures = sums
        .iter()
        .map(|m| m.formatted(w))
        .collect::<Result<Vec<_>, _>>()?;
    // The iPad's own words: " spent", in every language.
    Ok(Some(figures.join(" + ") + " spent"))
}

/// One day of the Record: its title, its lines, and its reads apart, to fold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day<'a> {
    pub title: String,
    pub lines: Vec<&'a Entry>,
    pub reads: Vec<&'a Entry>,
}

/// `days`: the Record by day, newest first, filtered; with All, a day's reads apart.
pub fn days<'a>(
    w: &WordBook,
    entries: &'a [Entry],
    filter: Filter,
    taf: Option<&str>,
    now: DateTime<Local>,
) -> Result<Vec<Day<'a>>, WordsError> {
    let mut kept: Vec<&Entry> = entries
        .iter()
        .filter(|e| filter.keeps(e) && taf.is_none_or(|t| e.taf == t))
        .collect();
    kept.sort_by_key(|e| std::cmp::Reverse(e.at));
    let mut out: Vec<(chrono::NaiveDate, Day)> = Vec::new();
    for e in kept {
        let day = crate::account_time::day(&e.at);
        if out.last().map(|(d, _)| *d) != Some(day) {
            out.push((
                day,
                Day {
                    title: day_title(w, day, crate::account_time::day(&now))?,
                    lines: vec![],
                    reads: vec![],
                },
            ));
        }
        let last = &mut out.last_mut().expect("a day").1;
        if e.read_only && filter == Filter::All {
            last.reads.push(e);
        } else {
            last.lines.push(e);
        }
    }
    Ok(out.into_iter().map(|(_, d)| d).collect())
}

/// `dayTitle`: "Today · Nov 15", "Yesterday · Nov 14", else "Friday · Nov 13", in the app's
/// language (its locale's abbreviated month and day, and wide weekday).
pub fn day_title(
    w: &WordBook,
    day: chrono::NaiveDate,
    today: chrono::NaiveDate,
) -> Result<String, WordsError> {
    let date = short_date(w.language(), day);
    if day == today {
        return w.get("calendar.today_date", &[("date", &date)]);
    }
    if Some(day) == today.pred_opt() {
        return w.get("calendar.yesterday_date", &[("date", &date)]);
    }
    Ok(format!("{} · {date}", weekday(w.language(), day)))
}

/// `.dateTime.day().month(.abbreviated)` in a shipped language.
pub fn short_date(language: &str, day: chrono::NaiveDate) -> String {
    crate::locale::short_date(language, day)
}

/// `.dateTime.weekday(.wide)` in a shipped language.
pub fn weekday(language: &str, day: chrono::NaiveDate) -> String {
    crate::locale::weekday(language, day)
}

#[cfg(test)]
mod tests {
    fn named(entity: Option<&str>, hash: Option<&str>) -> Entry {
        let mut e = crate::sample::record(
            &WordBook::load(&WordBook::default_root(), "en").unwrap(),
            chrono::Local::now(),
        )
        .unwrap()
        .remove(0);
        e.entity_id = entity.map(String::from);
        e.on_hash = hash.map(String::from);
        e.on_arg = Some("payment_intent".into());
        e.on_value = None;
        e
    }

    #[test]
    fn a_line_names_only_the_value_kiff_hashed() {
        let hash = format!(
            "sha256:{}",
            Sha256::digest(b"pi_3PqFixture0QbstR")
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>()
        );
        let refs: std::collections::BTreeMap<String, String> = [
            ("inv_1".to_string(), "pi_3PqFixture0QbstR".to_string()),
            ("inv_2".to_string(), "pi_NOT_KIFFS".to_string()),
        ]
        .into();
        let out = naming(
            vec![
                named(Some("inv_1"), Some(&hash)),
                named(Some("inv_2"), Some(&hash)),
                named(Some("inv_3"), Some(&hash)),
                named(Some("inv_1"), None),
            ],
            &refs,
        );
        assert_eq!(out[0].on_value.as_deref(), Some("pi_3PqFixture0QbstR"));
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        assert_eq!(
            out[0].on_text(&w).unwrap().as_deref(),
            Some("payment intent …QbstR")
        );
        assert!(out[1..].iter().all(|e| e.on_value.is_none()));
    }

    #[test]
    fn a_taf_s_calls_read_whole_or_not_at_all() {
        let refs = read_calls(br#"{"calls":[{"invocation":"inv_1","value":"pi_1"}]}"#).unwrap();
        assert_eq!(
            refs,
            vec![CallRef {
                invocation: "inv_1".into(),
                value: "pi_1".into()
            }]
        );
        assert!(read_calls(br#"{"calls":[{"invocation":"inv_1"}]}"#).is_none());
        assert!(read_calls(br#"{}"#).is_none());
        assert_eq!(calls_path("taf a/b"), "/v1/tafs/taf%20a%2Fb/calls");
    }

    #[test]
    fn only_tafs_with_a_hashed_call_are_read() {
        let calls = read_activity(
            br#"{"calls":[
            {"proposal_id":"p1","action_name":"a","actor_id":"rae","outcome":"allowed","created_at":"2026-11-16T03:41:03Z","on_hash":"sha256:00"},
            {"proposal_id":"p2","action_name":"a","actor_id":"bo","outcome":"allowed","created_at":"2026-11-16T03:41:03Z"}]}"#,
        )
        .unwrap();
        assert_eq!(
            agents_to_name(&calls).into_iter().collect::<Vec<_>>(),
            vec!["rae"]
        );
    }

    use super::*;

    #[test]
    fn decimals_read_as_kiff_writes_them() {
        assert_eq!(decimal("150.00"), Some((15000, 2)));
        assert_eq!(decimal("40"), Some((40, 0)));
        assert_eq!(decimal("-1.5"), Some((-15, 1)));
        assert_eq!(decimal("1e5"), None);
        assert_eq!(decimal(""), None);
        assert_eq!(decimal("1.0000000000000000000"), None);
    }

    #[test]
    fn dates_in_each_language() {
        let d = chrono::NaiveDate::from_ymd_opt(2026, 11, 13).unwrap();
        assert_eq!(short_date("en", d), "Nov 13");
        assert_eq!(short_date("de", d), "13. Nov.");
        assert_eq!(short_date("fr", d), "13 nov.");
        assert_eq!(weekday("en", d), "Friday");
        assert_eq!(weekday("fr", d), "vendredi");
    }

    #[test]
    fn a_refused_call_names_its_vendor_by_its_connection() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let body = br#"{"calls":[
          {"proposal_id":"p1","action_name":"create_refund","actor_id":"rae","outcome":"allowed","created_at":"2026-11-15T10:00:00Z","held":false,
           "tool_outcome":{"status":"refused_by_tool","message":"card_declined"}},
          {"proposal_id":"p2","action_name":"send","actor_id":"rae","outcome":"allowed","created_at":"2026-11-15T10:01:00Z","held":false,
           "tool_outcome":{"status":"refused_by_tool","refused_by":"connection","code":"x"},"detail":"Gmail refused it before sending"},
          {"proposal_id":"p3","action_name":"other","actor_id":"rae","outcome":"allowed","created_at":"2026-11-15T10:02:00Z","held":false,
           "tool_outcome":{"status":"refused_by_tool"}},
          {"proposal_id":"p4","action_name":"create_refund","actor_id":"rae","outcome":"allowed","created_at":"2026-11-15T10:03:00Z","held":false,
           "tool_outcome":{"status":"done"}}
        ]}"#;
        let calls = read_activity(body).unwrap();
        let vendors: std::collections::BTreeMap<String, String> =
            [("create_refund".to_string(), "Stripe".to_string())].into();
        let e = record(&w, &calls, &|_| Some("rae".into()), &vendors).unwrap();
        assert!(
            e[0].detail.starts_with("Stripe refused it"),
            "{}",
            e[0].detail
        );
        assert_eq!(
            (e[0].refused_by_tool, e[0].refused_by.as_deref()),
            (true, Some("Stripe"))
        );
        assert_eq!(e[1].refused_by.as_deref(), Some("The connection"));
        assert_eq!(e[1].detail, "Gmail refused it before sending");
        // No connection names the tool's vendor: "The app".
        assert!(
            e[2].detail.starts_with("The app refused it"),
            "{}",
            e[2].detail
        );
        assert_eq!(
            (e[2].refused_by_tool, e[2].refused_by.as_deref()),
            (true, None)
        );
        assert_eq!(
            (e[3].refused_by_tool, e[3].refused_by.as_deref()),
            (false, None)
        );
    }

    #[test]
    fn record_ids_are_stable() {
        assert_eq!(record_id("p1"), record_id("p1"));
        assert_ne!(record_id("p1"), record_id("p2"));
        assert_eq!(record_id("p1").len(), 32);
    }
}
