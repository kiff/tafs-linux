//! A taf's Cards as the Card tab draws them (`card(_:)`, `TafCardFace`, `CardStack`,
//! `CardLines`, `EnvironmentPill`): each Card whole, with its lines (`CardRule.summary`) and the
//! face's small lines (`TafCard.facts`).

use crate::card::{Card, Window};
use crate::money::Money;
use crate::words::{WordBook, WordsError};

/// One line of a Card (`CardRule`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    pub label: String,
    pub per_action: Option<Money>,
    pub per_window: Option<Money>,
    pub count_per_window: Option<i64>,
    pub window: Window,
    /// Never allowed, whatever the amount.
    pub never: bool,
    /// Allowed only with the owner's answer.
    pub always_ask: bool,
    /// Outside its limits it waits for the owner; else it's refused.
    pub holds: bool,
    pub extra_room: Option<Money>,
    pub base_limit: Option<Money>,
    /// When the extra room ends, in words.
    pub room_ends: Option<String>,
}

impl Rule {
    pub fn new(label: String, window: Window) -> Rule {
        Rule {
            label,
            per_action: None,
            per_window: None,
            count_per_window: None,
            window,
            never: false,
            always_ask: false,
            holds: true,
            extra_room: None,
            base_limit: None,
            room_ends: None,
        }
    }

    /// `ownTotal`: the Card's own total, without the extra room in it.
    fn own_total(&self) -> Option<&Money> {
        if self.extra_room.is_some() {
            self.base_limit.as_ref().or(self.per_window.as_ref())
        } else {
            self.per_window.as_ref()
        }
    }

    /// `sumLabel`: "in total a day" for a day's, any other window's own words.
    fn sum_label(&self, w: &WordBook) -> Result<String, WordsError> {
        if self.window.raw == "calendar_day" {
            w.get(
                "card.window.in_total_window",
                &[("window", &self.window.label)],
            )
        } else {
            Ok(self.window.label.clone())
        }
    }

    /// `roomWords`: "+€30 extra room until 00:00".
    fn room_words(&self, w: &WordBook) -> Result<Option<String>, WordsError> {
        let (Some(extra), Some(_)) = (&self.extra_room, &self.base_limit) else {
            return Ok(None);
        };
        let extra = extra.formatted(w)?;
        Ok(Some(match &self.room_ends {
            Some(ends) => w.get(
                "card.extra_room_until",
                &[("extra", &extra), ("ends", ends)],
            )?,
            None => w.get(
                "card.extra_room",
                &[("extra", &extra), ("window", &self.window.current)],
            )?,
        }))
    }

    /// `summary`: the line's limits in the words KIFF's Cards use.
    pub fn summary(&self, w: &WordBook) -> Result<String, WordsError> {
        if self.never {
            return w.line("card.rule.never");
        }
        if self.always_ask {
            return w.line("card.rule.always_asks");
        }
        let mut parts = Vec::new();
        if self.holds {
            if let Some(each) = &self.per_action {
                parts.push(w.get("card.rule.up_to_each", &[("amount", &each.formatted(w)?)])?);
            }
            if let Some(own) = self.own_total() {
                parts.push(w.get(
                    "card.asks_you",
                    &[
                        ("formatted", &own.formatted(w)?),
                        ("sum_label", &self.sum_label(w)?),
                    ],
                )?);
            }
            if let Some(room) = self.room_words(w)? {
                parts.push(room);
            }
            if let Some(n) = self.count_per_window {
                parts.push(if n == 0 {
                    w.line("card.held.each_call_asks")?
                } else {
                    w.count(
                        "card.rule.calls_then_asks",
                        n,
                        &[("window", &self.window.label)],
                    )?
                });
            }
            return Ok(parts.join(" · "));
        }
        if let Some(each) = &self.per_action {
            parts.push(w.get("card.rule.at_most_each", &[("amount", &each.formatted(w)?)])?);
        }
        if let Some(own) = self.own_total() {
            parts.push(if own.has_unit() {
                format!("{} {}", own.formatted(w)?, self.window.label)
            } else {
                format!("{} {}", own.formatted(w)?, self.sum_label(w)?)
            });
        }
        if let Some(room) = self.room_words(w)? {
            parts.push(room);
        }
        if let Some(n) = self.count_per_window {
            parts.push(w.count("card.calls_window", n, &[("window", &self.window.label)])?);
        }
        Ok(parts.join(" · "))
    }

    /// `CardLines`' second line: what happens outside it.
    pub fn outside(&self, w: &WordBook) -> Result<String, WordsError> {
        w.line(if self.holds {
            "card.outside_held"
        } else {
            "card.outside_refused"
        })
    }
}

/// One Card whole (`TafCard`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FullCard {
    pub id: String,
    pub title: String,
    pub environment: String,
    pub live: bool,
    pub active: bool,
    /// When it ends, already in words; none for a Card with no end.
    pub expires: Option<String>,
    pub rules: Vec<Rule>,
    /// Its totals, for its figure.
    pub totals: Card,
}

impl FullCard {
    /// `environmentLabel`: the environment it's scoped to in capitals, none when live or every one.
    pub fn environment_label(&self) -> Option<String> {
        (!self.live && !self.environment.is_empty()).then(|| self.environment.to_uppercase())
    }

    /// `facts`: the face's small lines under the figure.
    pub fn facts(&self, w: &WordBook) -> Result<Vec<String>, WordsError> {
        let mut out = Vec::new();
        if self.totals.hero(w)?.is_none() {
            for r in self.rules.iter().take(2) {
                out.push(format!("{} {}", r.label.to_lowercase(), r.summary(w)?));
            }
            if self.rules.len() > 2 {
                out.push(w.count("card.more", self.rules.len() as i64 - 2, &[])?);
            }
            return Ok(out);
        }
        if let Some(rule) = self.rules.iter().find(|r| r.per_action.is_some()) {
            let each = rule
                .per_action
                .as_ref()
                .expect("found by it")
                .formatted(w)?;
            out.push(if rule.holds {
                w.get("card.rule.up_to_each", &[("amount", &each)])?
            } else {
                w.get("card.rule.at_most_each_refused_above", &[("amount", &each)])?
            });
        }
        if self.rules.len() > 1 {
            out.push(w.count("card.tools", self.rules.len() as i64, &[])?);
        }
        Ok(out)
    }

    /// The face's foot: its caption ("Ends", "Until", "Ended", "Revoked") and its date.
    pub fn ends(&self, w: &WordBook, revoked: bool) -> Result<(String, String), WordsError> {
        let caption = w.line(if revoked {
            "card.revoked"
        } else if !self.active {
            "card.ended"
        } else if self.expires.is_none() {
            "card.ends"
        } else {
            "card.until"
        })?;
        let date = match &self.expires {
            Some(d) => d.clone(),
            None => w.line("card.no_end_date")?,
        };
        Ok((caption, date))
    }
}

/// `CardStack`: the Card in front last, the others behind it, furthest back first (the reverse of
/// their order), each with its depth behind the front one.
pub fn stack(cards: &[FullCard], front: Option<&str>) -> Vec<(usize, usize)> {
    if cards.is_empty() {
        return vec![];
    }
    let shown = cards
        .iter()
        .position(|c| Some(c.id.as_str()) == front)
        .unwrap_or(0);
    let mut order: Vec<usize> = (0..cards.len()).filter(|i| *i != shown).rev().collect();
    order.push(shown);
    let n = order.len();
    order
        .into_iter()
        .enumerate()
        .map(|(i, card)| (card, n - 1 - i))
        .collect()
}

/// `CardStack.shade`: how much darker a Card `depth` behind the front one is drawn.
pub fn shade(depth: usize) -> f64 {
    if depth == 0 {
        0.0
    } else {
        (0.12 + 0.08 * (depth as f64 - 1.0)).min(0.32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stack_puts_the_front_card_last() {
        let card = |id: &str| FullCard {
            id: id.into(),
            title: id.into(),
            environment: String::new(),
            live: true,
            active: true,
            expires: None,
            rules: vec![],
            totals: Card::default(),
        };
        let cards = [card("a"), card("b"), card("c")];
        assert_eq!(stack(&cards, None), [(2, 2), (1, 1), (0, 0)]);
        assert_eq!(stack(&cards, Some("b")), [(2, 2), (0, 1), (1, 0)]);
        assert_eq!(shade(0), 0.0);
        assert!((shade(1) - 0.12).abs() < 1e-9);
        assert!((shade(9) - 0.32).abs() < 1e-9);
    }
}
