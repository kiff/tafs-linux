//! KIFF's draft of a taf's Card (`KIFFClient.AgentDraft`, `DraftCard`), as an approval card in
//! Coach names it (`component.draft`): read as the session with `GET /v1/me/agent-drafts/{id}`
//! (`draft(id:)`), never confirmed here (confirming is signed, on the phone). Each Card's lines
//! are KIFF's own when it sends them, else the app's words for its grants and totals
//! (`DraftCard.lines(alike:)`). Decoded as strictly as Swift's synthesized `Decodable`: a field of
//! another shape fails the read, one that's missing or null is none.

use std::collections::HashSet;

use serde_json::{Map, Value};

use crate::card::Window;
use crate::coach::{array_if, bool_if, field, int_if, object, strings, text, text_if, Read};
use crate::held::{name as tool_name, tool_words};
use crate::money::Money;
use crate::signed::Unsigned;
use crate::words::{WordBook, WordsError};

/// `draft(id:)`'s path.
pub fn draft_path(id: &str) -> String {
    format!("/v1/me/agent-drafts/{}", crate::access::escaped(id))
}

/// `draft(id:)` as the session: a read, never one KIFF takes only signed.
pub fn draft_read(id: &str) -> Option<Unsigned> {
    Unsigned::new("GET", draft_path(id), Vec::new())
}

/// `DraftCard.Grant.PerAction`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerAction {
    pub quantity: String,
    pub max: i64,
    pub unit: Option<String>,
    pub scale: Option<i64>,
}

/// `DraftCard.Grant.Resources`: what a grant may act on, only these values of one parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resources {
    pub parameter: String,
    pub values: Option<Vec<String>>,
}

/// `DraftCard.Grant`: one action, its title and its caps; and its other terms, kept so a draft
/// can be compared with the Card in force (`allow::matches`): its scope, the states it applies
/// in, and the owner's other answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    pub action: String,
    pub title: Option<String>,
    pub per_action_max: Option<Vec<PerAction>>,
    pub resources: Option<Resources>,
    pub entity_states: Option<Vec<String>>,
    pub next_steps: Option<Vec<String>>,
}

/// `DraftCard.Aggregate`: a total over a window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Aggregate {
    pub quantity: String,
    pub limit: i64,
    pub window: String,
    pub unit: Option<String>,
    pub scale: Option<i64>,
    /// What the total counts (attempts or effects; KIFF's default when left out), and the
    /// parameter it's kept for each value of, when it's per item.
    pub measure: Option<String>,
    pub per: Option<String>,
}

/// `DraftCard`: its lines, and every term `allow::matches` compares with the Card in force.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DraftCard {
    pub grants: Option<Vec<Grant>>,
    pub aggregates: Option<Vec<Aggregate>>,
    pub on_exceed: Option<String>,
    /// KIFF's own words for the Card (`lines`), shown as they come.
    pub kiff_lines: Option<Vec<String>>,
    /// A revision draft's Card: the id of the Card in force it revises, and that Card's terms
    /// now (`current`: only its terms are read there).
    pub revises: Option<String>,
    pub current: Option<Box<DraftCard>>,
    /// `current`'s own id, when KIFF sends it: the Card in force it describes, which must be the
    /// one `revises` names (`allow::matches`). None from a KIFF that doesn't send it.
    pub id: Option<String>,
    /// KIFF says the Card it revises keeps its full terms.
    pub unchanged: Option<bool>,
    /// Who holds it, where it applies, and when it starts and ends, as KIFF writes them.
    pub subject: Option<String>,
    pub subjects: Option<Vec<String>>,
    pub domain: Option<String>,
    pub environment: Option<String>,
    pub location: Option<String>,
    pub valid_from: Option<String>,
    pub valid_until: Option<String>,
    /// A draft's alone: how long a hold waits, and a revocation.
    pub hold_expiry: Option<String>,
    pub revoked_at: Option<String>,
}

/// `AgentDraft`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentDraft {
    pub id: String,
    pub agent_id: String,
    /// draft, confirming, confirmed, rolling_back, failed, discarding, discarded, claimed.
    pub status: String,
    pub version: String,
    pub reason: Option<String>,
    pub cards: Option<Vec<DraftCard>>,
    /// An agent that holds no tools: Setup.
    pub no_tools: Option<bool>,
    /// The name it was submitted with, such as the taf's.
    pub label: Option<String>,
    /// The Cards it retires, by id.
    pub retires: Option<Vec<String>>,
    /// The held call it was made from (`allow-draft`), when KIFF says.
    pub from_hold: Option<String>,
}

fn money(minor: i64, unit: Option<&str>, scale: Option<i64>) -> Money {
    // `KIFFMapping.money`: a unit's scale as declared, or none. A scale no amount can have
    // reads as the bare number rather than a wrong figure.
    u32::try_from(scale.unwrap_or(0))
        .ok()
        .and_then(|s| Money::new(minor, unit, s))
        .or_else(|| Money::new(minor, None, 0))
        .expect("a bare number is always an amount")
}

/// `KIFFMapping.isSum`.
fn is_sum(quantity: &str) -> bool {
    quantity.to_lowercase().starts_with("sum(")
}

fn int_of(v: &Value) -> Read<i64> {
    v.as_i64().ok_or(())
}

fn per_action(v: &Value) -> Read<PerAction> {
    let o = object(v)?;
    Ok(PerAction {
        quantity: text(o, "quantity")?,
        max: int_of(field(o, "max").ok_or(())?)?,
        unit: text_if(o, "unit")?,
        scale: int_if(o, "scale")?,
    })
}

fn grant(v: &Value) -> Read<Grant> {
    let o = object(v)?;
    let resources = match field(o, "resources") {
        Some(r) => {
            let r = object(r)?;
            Some(Resources {
                parameter: text(r, "parameter")?,
                values: array_if(r, "values", strings)?,
            })
        }
        None => None,
    };
    Ok(Grant {
        action: text(o, "action")?,
        title: text_if(o, "title")?,
        per_action_max: array_if(o, "per_action_max", per_action)?,
        resources,
        entity_states: array_if(o, "entity_states", strings)?,
        next_steps: array_if(o, "next_steps", strings)?,
    })
}

fn aggregate(v: &Value) -> Read<Aggregate> {
    let o = object(v)?;
    Ok(Aggregate {
        quantity: text(o, "quantity")?,
        limit: int_of(field(o, "limit").ok_or(())?)?,
        window: text(o, "window")?,
        unit: text_if(o, "unit")?,
        scale: int_if(o, "scale")?,
        measure: text_if(o, "measure")?,
        per: text_if(o, "per")?,
    })
}

/// A Card's terms (`DraftCard.Terms`, and a `DraftCard`'s own): its grants, totals, words, who
/// holds it, where it applies and when.
fn terms(o: &Map<String, Value>) -> Read<DraftCard> {
    Ok(DraftCard {
        grants: array_if(o, "grants", grant)?,
        aggregates: array_if(o, "aggregates", aggregate)?,
        on_exceed: text_if(o, "on_exceed")?,
        kiff_lines: array_if(o, "lines", strings)?,
        subject: text_if(o, "subject")?,
        subjects: array_if(o, "subjects", strings)?,
        domain: text_if(o, "domain")?,
        environment: text_if(o, "environment")?,
        location: text_if(o, "location")?,
        valid_from: text_if(o, "valid_from")?,
        valid_until: text_if(o, "valid_until")?,
        ..DraftCard::default()
    })
}

fn draft_card(v: &Value) -> Read<DraftCard> {
    let o = object(v)?;
    let current = match field(o, "current") {
        Some(c) => {
            let c = object(c)?;
            Some(Box::new(DraftCard {
                id: text_if(c, "id")?,
                ..terms(c)?
            }))
        }
        None => None,
    };
    Ok(DraftCard {
        revises: text_if(o, "revises")?,
        current,
        unchanged: bool_if(o, "unchanged")?,
        hold_expiry: text_if(o, "hold_expiry")?,
        revoked_at: text_if(o, "revoked_at")?,
        ..terms(o)?
    })
}

impl AgentDraft {
    /// KIFF's answer to `draft(id:)`; none when it isn't one.
    pub fn read(body: &[u8]) -> Option<AgentDraft> {
        let v: Value = serde_json::from_slice(body).ok()?;
        Self::read_value(&v).ok()
    }

    fn read_value(v: &Value) -> Read<AgentDraft> {
        let o = object(v)?;
        text_if(o, "claim_secret")?;
        array_if(o, "card_ids", strings)?;
        let from_hold = text_if(o, "from_hold")?;
        let retires = array_if(o, "retires", |r| {
            let r = object(r)?;
            if let Some(c) = field(r, "current") {
                terms(object(c)?)?;
            }
            text(r, "id")
        })?;
        Ok(AgentDraft {
            retires,
            from_hold,
            id: text(o, "id")?,
            agent_id: text(o, "agent_id")?,
            status: text(o, "status")?,
            version: text(o, "version")?,
            reason: text_if(o, "reason")?,
            cards: array_if(o, "cards", draft_card)?,
            no_tools: bool_if(o, "no_tools")?,
            label: text_if(o, "label")?,
        })
    }

    pub fn confirmed(&self) -> bool {
        self.status == "confirmed"
    }

    /// KIFF took the owner's signature for it: confirmed, or confirmed and its key claimed since.
    pub fn signed(&self) -> bool {
        self.status == "confirmed" || self.status == "claimed"
    }

    /// An agent that holds tools: Setup's own draft holds none.
    pub fn holds_tools(&self) -> bool {
        self.no_tools != Some(true)
    }

    /// `cardLines`: each Card's lines, two grants that read alike adding their action.
    pub fn card_lines(&self, w: &WordBook) -> Result<Vec<Vec<String>>, WordsError> {
        let all = self.cards.as_deref().unwrap_or_default();
        let grants: Vec<Grant> = all
            .iter()
            .flat_map(|c| c.grants.clone().unwrap_or_default())
            .collect();
        let alike = alike(&grants);
        all.iter().map(|c| c.lines(w, &alike)).collect()
    }

    /// `lines`: every Card's lines in one list.
    pub fn lines(&self, w: &WordBook) -> Result<Vec<String>, WordsError> {
        Ok(self.card_lines(w)?.into_iter().flatten().collect())
    }

    /// `cardTitle`: "Refunds assistant's Card", never its agent id.
    pub fn card_title(&self, w: &WordBook) -> Result<String, WordsError> {
        let name = self.label.as_deref().unwrap_or("").trim();
        if name.is_empty() {
            w.line("card.the_tafs_card")
        } else {
            w.get("coach.card_of", &[("owner", name)])
        }
    }
}

/// `DraftCard.alike`: the names, ignoring case, that more than one of these grants reads as.
pub fn alike(grants: &[Grant]) -> HashSet<String> {
    let mut seen = HashSet::new();
    let mut twice = HashSet::new();
    for g in grants {
        let name = tool_name(&g.action, g.title.as_deref()).to_lowercase();
        if !seen.insert(name.clone()) {
            twice.insert(name);
        }
    }
    twice
}

impl DraftCard {
    /// `asksPastLimits`: past a limit, the call asks the owner first.
    pub fn asks_past_limits(&self) -> bool {
        self.on_exceed.as_deref() == Some("owner")
    }

    /// `lines(alike:)`: KIFF's own lines when sent; else each action by its title with its caps,
    /// then each total in its own window.
    pub fn lines(&self, w: &WordBook, alike: &HashSet<String>) -> Result<Vec<String>, WordsError> {
        if let Some(k) = self.kiff_lines.as_ref().filter(|k| !k.is_empty()) {
            return Ok(k.clone());
        }
        let mut out = Vec::new();
        for g in self.grants.as_deref().unwrap_or_default() {
            let name = tool_name(&g.action, g.title.as_deref());
            let caps = g
                .per_action_max
                .as_deref()
                .unwrap_or_default()
                .iter()
                .map(|cap| {
                    let amount = if is_sum(&cap.quantity) {
                        money(cap.max, cap.unit.as_deref(), cap.scale).formatted(w)?
                    } else {
                        cap.max.to_string()
                    };
                    w.get("card.line.up_to_each", &[("amount", &amount)])
                })
                .collect::<Result<Vec<_>, _>>()?;
            let action = if alike.contains(&name.to_lowercase()) {
                format!("{name} · {}", tool_words(&g.action))
            } else {
                name
            };
            let more = if self.asks_past_limits() {
                w.line("card.line.more_asks")?
            } else {
                String::new()
            };
            out.push(if caps.is_empty() {
                action
            } else {
                format!("{action}: {}{more}", caps.join(", "))
            });
        }
        let then = if self.asks_past_limits() {
            w.line("card.line.then_asks")?
        } else {
            String::new()
        };
        for a in self.aggregates.as_deref().unwrap_or_default() {
            // KIFF's own window, exactly: never a lifetime or hourly total read as a day's.
            let window = Window::new(w, &a.window, None, None)?;
            if is_sum(&a.quantity) {
                let sum_label = if window.raw == "calendar_day" {
                    w.get("card.window.in_total_window", &[("window", &window.label)])?
                } else {
                    window.label.clone()
                };
                out.push(format!(
                    "{} {sum_label}{then}",
                    money(a.limit, a.unit.as_deref(), a.scale).formatted(w)?
                ));
            } else if self.asks_past_limits() && a.limit == 0 {
                out.push(w.line("card.held.each_call_asks")?);
            } else {
                out.push(w.count(
                    "card.line.calls_in_window",
                    a.limit,
                    &[("window", &window.label), ("then", &then)],
                )?);
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(lang: &str) -> WordBook {
        WordBook::load(&WordBook::default_root(), lang).unwrap()
    }

    const DRAFT: &str = r#"{
        "id": "drf_s1", "agent_id": "refunds-acme", "status": "draft", "version": "v3",
        "label": "Refunds assistant", "future_field": {"anything": 1},
        "cards": [{
            "grants": [
                {"action": "stripe__create_refund", "title": "Refund a payment",
                 "per_action_max": [{"quantity": "sum(amount)", "max": 10000, "unit": "EUR", "scale": 2}]},
                {"action": "stripe__list_charges"}
            ],
            "aggregates": [
                {"quantity": "sum(amount)", "limit": 100000, "window": "calendar_day", "unit": "EUR", "scale": 2},
                {"quantity": "count", "limit": 20, "window": "calendar_day"}
            ]
        }]
    }"#;

    #[test]
    fn a_draft_reads_with_its_lines_and_title() {
        let w = words("en");
        let d = AgentDraft::read(DRAFT.as_bytes()).unwrap();
        assert_eq!(d.id, "drf_s1");
        assert!(!d.confirmed() && !d.signed() && d.holds_tools());
        assert_eq!(d.card_title(&w).unwrap(), "Refunds assistant's Card");
        assert_eq!(
            d.lines(&w).unwrap(),
            vec![
                "Refund a payment: up to €100 each",
                "list charges",
                "€1,000 in total a day",
                "20 calls a day",
            ]
        );
    }

    #[test]
    fn kiffs_own_lines_win_and_past_limits_asks() {
        let w = words("en");
        let own = r#"{"id":"d","agent_id":"a","status":"confirmed","version":"1",
            "cards":[{"lines":["Refund up to €50"],"grants":[{"action":"x"}]}]}"#;
        let d = AgentDraft::read(own.as_bytes()).unwrap();
        assert!(d.confirmed() && d.signed());
        assert_eq!(d.lines(&w).unwrap(), vec!["Refund up to €50"]);
        let asks = r#"{"id":"d","agent_id":"a","status":"claimed","version":"1","cards":[{
            "on_exceed":"owner",
            "grants":[{"action":"pay","title":"Pay","per_action_max":[{"quantity":"count","max":3}]}],
            "aggregates":[{"quantity":"count","limit":0,"window":"calendar_day"},
                          {"quantity":"count","limit":5,"window":"rolling_1h"}]}]}"#;
        let d = AgentDraft::read(asks.as_bytes()).unwrap();
        assert!(d.signed() && !d.confirmed());
        assert_eq!(
            d.lines(&w).unwrap(),
            vec![
                "Pay: up to 3 each; more asks you first",
                "Each call asks you first",
                "5 calls in any hour, then it asks you",
            ]
        );
    }

    #[test]
    fn two_grants_that_read_alike_add_their_action() {
        let w = words("en");
        let body = r#"{"id":"d","agent_id":"a","status":"draft","version":"1","cards":[
            {"grants":[{"action":"stripe__write_refunds","title":"Write Stripe API data"}]},
            {"grants":[{"action":"stripe__write_customers","title":"write stripe api data"}]}]}"#;
        let d = AgentDraft::read(body.as_bytes()).unwrap();
        assert_eq!(
            d.card_lines(&w).unwrap(),
            vec![
                vec!["Write Stripe API data · write refunds"],
                vec!["write stripe api data · write customers"],
            ]
        );
    }

    #[test]
    fn a_title_with_no_label_is_the_tafs_card() {
        let w = words("en");
        let d = AgentDraft::read(
            br#"{"id":"d","agent_id":"a","status":"draft","version":"1","label":"  "}"#,
        )
        .unwrap();
        assert_eq!(d.card_title(&w).unwrap(), "The taf's Card");
        assert_eq!(d.lines(&w).unwrap(), Vec::<String>::new());
    }

    #[test]
    fn a_field_of_another_shape_fails_the_read_as_swift_does() {
        // Missing what's required.
        assert!(AgentDraft::read(br#"{"id":"d","status":"draft","version":"1"}"#).is_none());
        // A number where text goes, nested or not.
        for bad in [
            r#"{"id":"d","agent_id":"a","status":"draft","version":1}"#,
            r#"{"id":"d","agent_id":"a","status":"draft","version":"1","label":3}"#,
            r#"{"id":"d","agent_id":"a","status":"draft","version":"1","cards":[{"grants":[{"title":"x"}]}]}"#,
            r#"{"id":"d","agent_id":"a","status":"draft","version":"1","cards":[{"aggregates":[{"quantity":"count","limit":"5","window":"week"}]}]}"#,
            r#"{"id":"d","agent_id":"a","status":"draft","version":"1","retires":[{"current":{}}]}"#,
            r#"{"id":"d","agent_id":"a","status":"draft","version":"1","no_tools":"yes"}"#,
        ] {
            assert!(AgentDraft::read(bad.as_bytes()).is_none(), "{bad}");
        }
        // Null is none.
        let d = AgentDraft::read(
            br#"{"id":"d","agent_id":"a","status":"draft","version":"1","cards":null,"label":null}"#,
        )
        .unwrap();
        assert_eq!(d.cards, None);
    }

    #[test]
    fn the_read_is_unsigned_and_confirming_never_is() {
        let read = draft_read("drf s/1").unwrap();
        assert_eq!(read.method(), "GET");
        assert_eq!(read.path(), "/v1/me/agent-drafts/drf%20s%2F1");
        assert!(read.body().is_empty());
        for (method, path) in [
            ("POST", "/v1/me/agent-drafts/drf_s1/confirm"),
            ("POST", "/v1/me/agent-drafts/drf_s1/trial"),
            ("POST", "/v1/me/agent-drafts"),
        ] {
            assert!(Unsigned::new(method, path.into(), vec![]).is_none());
        }
    }

    #[test]
    fn every_shipped_language_words_the_lines() {
        for lang in crate::words::SHIPPED {
            let w = words(lang);
            let d = AgentDraft::read(DRAFT.as_bytes()).unwrap();
            assert_eq!(d.lines(&w).unwrap().len(), 4);
            assert!(!d.card_title(&w).unwrap().is_empty());
        }
    }
}
