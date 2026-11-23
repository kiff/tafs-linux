//! "Allow this from now on" on a held call (kiff/tafs `docs/allow-from-now-on.md`, the iPad's
//! `HoldAllow`, `AllowSheetView`). Whether to offer it is the app's own rule, from the held call KIFF
//! sent and the account's capabilities, with no request. What it offers is tafs cloud's
//! (`POST /v1/tafs/{id}/holds/{exception}/allow`). What's signed is KIFF's draft, which this
//! computer reads itself (`GET /v1/me/agent-drafts/{id}`) and signs only when it's exactly what
//! was offered (`matches`): one confirm, with this computer's security key, naming the held call
//! when it goes too (`signed::Act::AllowFromNowOn`).

use std::collections::BTreeSet;

use serde_json::Value;

use crate::drafts::{AgentDraft, DraftCard};
use crate::held::HeldCall;

/// The breaches a Card change can lift. A "never" is refused, not held, so it never shows here.
pub const LIFTABLE: [&str; 3] = [
    "per_action_exceeded",
    "limit_reached",
    "resource_not_on_card",
];

/// `allowable`: a hold from KIFF whose every breach a Card change can lift. Never a sign-in fill
/// (a Card that let fills go on their own would allow them without a release), and never Coach's
/// install of a version (approving that is installing it).
pub fn allowable(call: &HeldCall) -> bool {
    !call.needs_release
        && call.kiff_id.is_some()
        && call.action != "version_install"
        && !call.breach_outcomes.is_empty()
        && call
            .breach_outcomes
            .iter()
            .all(|o| LIFTABLE.contains(&o.as_str()))
}

/// `allowIsLive`: the account's capabilities name `holds.allow`. A list without it, or no list at
/// all, shows no button, so nobody taps into a route that isn't there yet.
pub fn is_live(capabilities: Option<&[String]>) -> bool {
    capabilities.is_some_and(|c| c.iter().any(|x| x == "holds.allow"))
}

/// `managesCards`: `cards.manage`, or a KIFF that sends no list.
pub fn manages_cards(capabilities: Option<&[String]>) -> bool {
    capabilities.is_none_or(|c| c.iter().any(|x| x == "cards.manage"))
}

/// `offersAllow`, given what the screen knows: signed in with KIFF, live, a Cards manager, the
/// call's taf one of tafs cloud's, and the call allowable.
pub fn offers(
    kiff: bool,
    capabilities: Option<&[String]>,
    cloud_taf: bool,
    call: &HeldCall,
) -> bool {
    kiff && is_live(capabilities) && manages_cards(capabilities) && cloud_taf && allowable(call)
}

/// tafs cloud's offer route for a held call of taf `taf` (tafs cloud's id).
pub fn offer_path(taf: &str, exception: &str) -> String {
    format!(
        "/v1/tafs/{}/holds/{}/allow",
        crate::access::escaped(taf),
        crate::access::escaped(exception)
    )
}

/// The offer's body: `{}` for the exact figure, `{"round":true}` for the rounded one.
pub fn offer_body(round: bool) -> Vec<u8> {
    if round {
        br#"{"round":true}"#.to_vec()
    } else {
        b"{}".to_vec()
    }
}

/// `AllowOffer.Change`: one limit the offer changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Change {
    pub mandate_id: String,
    pub kind: String,
    pub before: Option<i64>,
    pub after: Option<i64>,
    pub unit: Option<String>,
    pub window: Option<String>,
    /// For a `resource` change: the parameter whose scope grows, and the one value added.
    pub parameter: Option<String>,
    pub value: Option<String>,
    pub words_before: Option<String>,
    pub words_after: Option<String>,
    pub rounded: Option<bool>,
}

/// `AllowOffer`: the narrowest change, what it would have let go, and KIFF's draft for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub changes: Vec<Change>,
    /// `would.words`, left out when KIFF's replay couldn't be read.
    pub would: Option<String>,
    pub draft: String,
    pub send_held: Option<bool>,
    /// `round.words`: the rounded figure, named before any tap. Absent when rounding changes
    /// nothing. The app never rounds.
    pub round: Option<String>,
}

/// `AllowAnswer`: the offer, or why there's none in tafs cloud's words. A `none` this app
/// doesn't know reads as any other: its words, and nothing to sign.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub offer: Option<Offer>,
    pub none: Option<String>,
    pub words: Option<String>,
}

type Read<T> = Result<T, ()>;

fn opt_text(o: &serde_json::Map<String, Value>, k: &str) -> Read<Option<String>> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        _ => Err(()),
    }
}

fn opt_int(o: &serde_json::Map<String, Value>, k: &str) -> Read<Option<i64>> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(v) => v.as_i64().map(Some).ok_or(()),
    }
}

fn opt_bool(o: &serde_json::Map<String, Value>, k: &str) -> Read<Option<bool>> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Bool(b)) => Ok(Some(*b)),
        _ => Err(()),
    }
}

fn opt_object<'a>(
    o: &'a serde_json::Map<String, Value>,
    k: &str,
) -> Read<Option<&'a serde_json::Map<String, Value>>> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(m)) => Ok(Some(m)),
        _ => Err(()),
    }
}

fn change(v: &Value) -> Read<Change> {
    let o = v.as_object().ok_or(())?;
    Ok(Change {
        mandate_id: opt_text(o, "mandate_id")?.ok_or(())?,
        kind: opt_text(o, "kind")?.ok_or(())?,
        before: opt_int(o, "before")?,
        after: opt_int(o, "after")?,
        unit: opt_text(o, "unit")?,
        window: opt_text(o, "window")?,
        parameter: opt_text(o, "parameter")?,
        value: opt_text(o, "value")?,
        words_before: opt_text(o, "words_before")?,
        words_after: opt_text(o, "words_after")?,
        rounded: opt_bool(o, "rounded")?,
    })
}

fn offer(o: &serde_json::Map<String, Value>) -> Read<Offer> {
    let changes = o
        .get("changes")
        .and_then(Value::as_array)
        .ok_or(())?
        .iter()
        .map(change)
        .collect::<Read<Vec<_>>>()?;
    let would = match opt_object(o, "would")? {
        Some(w) => {
            opt_int(w, "calls")?;
            opt_text(w, "words")?
        }
        None => None,
    };
    let round = match opt_object(o, "round")? {
        Some(r) => {
            opt_int(r, "after")?;
            opt_text(r, "words")?
        }
        None => None,
    };
    opt_text(o, "confirm_key")?;
    Ok(Offer {
        changes,
        would,
        draft: opt_text(o, "draft")?.ok_or(())?,
        send_held: opt_bool(o, "send_held")?,
        round,
    })
}

impl Answer {
    /// tafs cloud's answer, read as strictly as the iPad's decoder: a field of another shape
    /// fails the read; one missing or null is none.
    pub fn read(body: &[u8]) -> Option<Answer> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let o = v.as_object()?;
        Some(Answer {
            offer: match opt_object(o, "offer").ok()? {
                Some(m) => Some(offer(m).ok()?),
                None => None,
            },
            none: opt_text(o, "none").ok()?,
            words: opt_text(o, "words").ok()?,
        })
    }
}

/// Every term of a Card that decides what its taf may do, in an order of its own, as KIFF writes
/// them in a draft's Card and in the Card in force (`HoldAllow.Terms`). KIFF's lines and a grant's
/// title are words, not terms. What a draft may leave out reads as KIFF fills it: a total's
/// measure (attempts for a count, effects for a sum), and the Card's holders when the draft names
/// only the same first holder.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Terms {
    grants: Vec<TGrant>,
    totals: Vec<TTotal>,
    on_exceed: Option<String>,
    holders: Vec<String>,
    domain: String,
    environment: String,
    location: String,
    valid_from: String,
    valid_until: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TCap {
    quantity: String,
    unit: Option<String>,
    max: i64,
    scale: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TGrant {
    action: String,
    caps: Vec<TCap>,
    /// The scope's parameter, and its values sorted; none for a grant with no scope.
    scope_of: Option<String>,
    scope: Vec<String>,
    states: Vec<String>,
    next_steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct TTotal {
    quantity: String,
    window: String,
    per: String,
    measure: String,
    unit: Option<String>,
    limit: i64,
    scale: Option<i64>,
}

/// A time as KIFF writes it, as the instant it names, so the same moment written with another
/// offset or precision is the same term. None is "". Text that can't be read as a time is kept
/// behind a mark no instant has, so it equals only the same text, and never a time.
fn instant(time: Option<&str>) -> String {
    match time {
        None | Some("") => String::new(),
        Some(t) => match chrono::DateTime::parse_from_rfc3339(t) {
            Ok(d) => format!("@{}", d.timestamp_millis()),
            Err(_) => format!("?{t}"),
        },
    }
}

fn holders(subject: Option<&String>, subjects: Option<&Vec<String>>) -> Vec<String> {
    match subjects {
        Some(s) if !s.is_empty() => s.clone(),
        _ => subject.map(|s| vec![s.clone()]).unwrap_or_default(),
    }
}

fn grants(card: &DraftCard) -> Vec<TGrant> {
    card.grants
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|g| TGrant {
            action: g.action.clone(),
            caps: g
                .per_action_max
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|c| TCap {
                    quantity: c.quantity.clone(),
                    unit: c.unit.clone(),
                    max: c.max,
                    scale: c.scale,
                })
                .collect(),
            scope_of: g.resources.as_ref().map(|r| r.parameter.clone()),
            scope: g
                .resources
                .as_ref()
                .and_then(|r| r.values.clone())
                .unwrap_or_default(),
            states: g.entity_states.clone().unwrap_or_default(),
            next_steps: g.next_steps.clone().unwrap_or_default(),
        })
        .collect()
}

fn totals(card: &DraftCard) -> Vec<TTotal> {
    card.aggregates
        .as_deref()
        .unwrap_or_default()
        .iter()
        .map(|a| {
            // Left out, KIFF counts attempts for a count and effects for a sum.
            let measure = match a.measure.as_deref() {
                Some(m) if !m.is_empty() => m.to_string(),
                _ if a.quantity == "count" => "attempts".into(),
                _ => "effects".into(),
            };
            TTotal {
                quantity: a.quantity.clone(),
                window: a.window.clone(),
                per: a.per.clone().unwrap_or_default(),
                measure,
                unit: a.unit.clone(),
                limit: a.limit,
                scale: a.scale,
            }
        })
        .collect()
}

impl Terms {
    #[allow(clippy::too_many_arguments)]
    fn new(
        grants: Vec<TGrant>,
        totals: Vec<TTotal>,
        on_exceed: Option<String>,
        holders: Vec<String>,
        domain: Option<&str>,
        environment: Option<&str>,
        location: Option<&str>,
        valid_from: String,
        valid_until: String,
    ) -> Terms {
        let mut grants: Vec<TGrant> = grants
            .into_iter()
            .map(|mut g| {
                g.caps.sort();
                g.scope.sort();
                g
            })
            .collect();
        grants.sort();
        let mut totals = totals;
        totals.sort();
        Terms {
            grants,
            totals,
            on_exceed,
            holders,
            domain: domain.unwrap_or_default().to_lowercase(),
            environment: environment.unwrap_or_default().to_lowercase(),
            location: location.unwrap_or_default().to_string(),
            valid_from,
            valid_until,
        }
    }

    /// The Card in force (`current`).
    pub fn in_force(current: &DraftCard) -> Terms {
        Terms::new(
            grants(current),
            totals(current),
            current.on_exceed.clone(),
            holders(current.subject.as_ref(), current.subjects.as_ref()),
            current.domain.as_deref(),
            current.environment.as_deref(),
            current.location.as_deref(),
            instant(current.valid_from.as_deref()),
            instant(current.valid_until.as_deref()),
        )
    }

    /// A draft's Card, read over the Card in force it revises: a draft that names no holders keeps
    /// the Card's, when its first holder is the same.
    pub fn of_draft(card: &DraftCard, current: &DraftCard) -> Terms {
        let in_force_holders = holders(current.subject.as_ref(), current.subjects.as_ref());
        let mut h = holders(card.subject.as_ref(), card.subjects.as_ref());
        if card.subjects.as_ref().is_none_or(|s| s.is_empty())
            && card.subject.is_some()
            && card.subject.as_ref() == in_force_holders.first()
        {
            h = in_force_holders;
        }
        Terms::new(
            grants(card),
            totals(card),
            card.on_exceed.clone(),
            h,
            card.domain.as_deref(),
            card.environment.as_deref(),
            card.location.as_deref(),
            instant(card.valid_from.as_deref()),
            instant(card.valid_until.as_deref()),
        )
    }

    fn with(&self, grants: Option<Vec<TGrant>>, totals: Option<Vec<TTotal>>) -> Terms {
        Terms::new(
            grants.unwrap_or_else(|| self.grants.clone()),
            totals.unwrap_or_else(|| self.totals.clone()),
            self.on_exceed.clone(),
            self.holders.clone(),
            Some(&self.domain),
            Some(&self.environment),
            Some(&self.location),
            self.valid_from.clone(),
            self.valid_until.clone(),
        )
    }
}

/// Every way `change` can apply to `terms`: one limit of its kind, at its figure before and in its
/// unit, moved to its figure after; or one value added to the scope of the held call's `action`. A
/// per-call cap is the held `action`'s. A kind the contract doesn't name applies in no way.
pub fn applied(terms: &Terms, change: &Change, action: &str) -> Vec<Terms> {
    let mut all = vec![];
    match change.kind.as_str() {
        "per_action" => {
            let (Some(before), Some(after)) = (change.before, change.after) else {
                return all;
            };
            for (g, grant) in terms.grants.iter().enumerate() {
                if grant.action != action {
                    continue;
                }
                for (c, cap) in grant.caps.iter().enumerate() {
                    if cap.max == before && cap.unit == change.unit {
                        let mut grants = terms.grants.clone();
                        grants[g].caps[c].max = after;
                        all.push(terms.with(Some(grants), None));
                    }
                }
            }
        }
        "total" | "count" => {
            let (Some(before), Some(after)) = (change.before, change.after) else {
                return all;
            };
            for (t, total) in terms.totals.iter().enumerate() {
                if total.limit == before
                    && total.unit == change.unit
                    && change.window.as_ref().is_none_or(|w| *w == total.window)
                {
                    let mut totals = terms.totals.clone();
                    totals[t].limit = after;
                    all.push(terms.with(None, Some(totals)));
                }
            }
        }
        "resource" => {
            // Exactly the values in force plus this one, on the held action's scope for this
            // parameter.
            let (Some(parameter), Some(value)) = (
                change.parameter.as_deref().filter(|p| !p.is_empty()),
                change.value.as_deref().filter(|v| !v.is_empty()),
            ) else {
                return all;
            };
            for (g, grant) in terms.grants.iter().enumerate() {
                if grant.action == action
                    && grant.scope_of.as_deref() == Some(parameter)
                    && !grant.scope.iter().any(|s| s == value)
                {
                    let mut grants = terms.grants.clone();
                    grants[g].scope.push(value.to_string());
                    all.push(terms.with(Some(grants), None));
                }
            }
        }
        _ => {}
    }
    all
}

/// `HoldAllow.matches`: this computer signs what the person saw. KIFF's draft is for `agent` (and,
/// when KIFF says, made from held call `hold`). It retires no Card and adds none. Each Card it
/// changes is one the offer names, and reads exactly as that Card in force with the offered change
/// made (`applied`); every other term is the same. A Card marked unchanged must read as in force
/// too. Anything else isn't what was offered, and nothing is signed.
pub fn matches(draft: &AgentDraft, offer: &Offer, agent: &str, hold: &str, action: &str) -> bool {
    if agent.is_empty() || draft.agent_id != agent {
        return false;
    }
    if draft.from_hold.as_deref().is_some_and(|f| f != hold) {
        return false;
    }
    if !draft.retires.as_deref().unwrap_or_default().is_empty() {
        return false;
    }
    let named: BTreeSet<&str> = offer
        .changes
        .iter()
        .map(|c| c.mandate_id.as_str())
        .collect();
    let Some(cards) = draft.cards.as_deref() else {
        return false;
    };
    if named.is_empty() {
        return false;
    }
    let mut changed: BTreeSet<&str> = BTreeSet::new();
    for card in cards {
        let (Some(revises), Some(current)) = (card.revises.as_deref(), card.current.as_deref())
        else {
            return false;
        };
        // A draft that sets how long a hold waits, or revokes, changes more than a limit.
        if card.hold_expiry.as_deref().is_some_and(|h| !h.is_empty()) || card.revoked_at.is_some() {
            return false;
        }
        let in_force = Terms::in_force(current);
        let terms = Terms::of_draft(card, current);
        if card.unchanged == Some(true) {
            if terms != in_force {
                return false;
            }
            continue;
        }
        if !named.contains(revises) || changed.contains(revises) {
            return false;
        }
        let mut expected: BTreeSet<Terms> = BTreeSet::from([in_force]);
        for change in offer.changes.iter().filter(|c| c.mandate_id == revises) {
            expected = expected
                .iter()
                .flat_map(|t| applied(t, change, action))
                .collect();
        }
        if !expected.contains(&terms) {
            return false;
        }
        changed.insert(revises);
    }
    changed == named
}

#[cfg(test)]
mod tests;
