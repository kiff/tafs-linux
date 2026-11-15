//! A Card's figure, as the iPad draws it (`Model/Models.swift`: `Window`, `CardTotal`,
//! `TafCard.hero`): what's left of its money total, else of its calls. Use KIFF couldn't read
//! shows the limit alone, never as zero left or used.

use crate::money::Money;
use crate::words::{WordBook, WordsError};

/// A Card's window (`Window`): KIFF's name for it, and its words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub raw: String,
    /// Ends a limit: "a day", "in any hour", "in total".
    pub label: String,
    /// The window's use so far: "today", "in the last hour", "so far".
    pub current: String,
}

impl Window {
    /// The window `raw`, in the server's words when it sent them, else the app's (`known`).
    pub fn new(
        w: &WordBook,
        raw: &str,
        each: Option<&str>,
        now: Option<&str>,
    ) -> Result<Window, WordsError> {
        // "day" is how the app kept a window before it kept KIFF's own name.
        let raw = if raw == "day" { "calendar_day" } else { raw };
        let known = match raw {
            "calendar_day" => Some(("card.window.a_day", "card.window.today")),
            "week" => Some(("card.window.a_week", "card.window.this_week")),
            "rolling_24h" => Some(("card.window.any_day", "card.window.last_day")),
            "rolling_7d" => Some(("card.window.any_week", "card.window.last_week")),
            "rolling_1h" => Some(("card.window.any_hour", "card.window.last_hour")),
            "lifetime" => Some(("card.window.in_total", "card.window.so_far")),
            _ => None,
        };
        let label = match (each, known) {
            (Some(e), _) => e.to_string(),
            (None, Some((k, _))) => w.line(k)?,
            (None, None) => raw.to_string(),
        };
        let current = match (now, known) {
            (Some(n), _) => n.to_string(),
            (None, Some((_, k))) => w.line(k)?,
            (None, None) => raw.to_string(),
        };
        Ok(Window {
            raw: raw.to_string(),
            label,
            current,
        })
    }
}

/// A money total for its window (`CardTotal<Money>`): `used` is none when KIFF couldn't read
/// it, never zero. With extra room, KIFF's `extra` and the Card's own `base`, and when the room
/// ends, already in words (`endsWords`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MoneyTotal {
    pub limit: Money,
    pub used: Option<Money>,
    pub window: Window,
    pub extra: Option<Money>,
    pub base: Option<Money>,
    pub room_ends: Option<String>,
}

impl MoneyTotal {
    pub fn new(limit: Money, used: Option<Money>, window: Window) -> MoneyTotal {
        MoneyTotal {
            limit,
            used,
            window,
            extra: None,
            base: None,
            room_ends: None,
        }
    }
}

/// A count of calls (`CardTotal<Int>`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallsTotal {
    pub limit: i64,
    pub used: Option<i64>,
    pub window: Window,
}

/// The Card in front of a taf, as far as its figure goes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Card {
    pub total: Option<MoneyTotal>,
    pub calls: Option<CallsTotal>,
}

impl Card {
    /// `hero`: the face's big figure and the words under it.
    pub fn hero(&self, w: &WordBook) -> Result<Option<(String, String)>, WordsError> {
        let lang = w.language();
        if let Some(t) = &self.total {
            // An amount with no currency says so once, after the words, and its figures stand
            // bare beside it: "30 000 left today of 30 000 · no currency".
            let unitless = !t.limit.has_unit();
            let shown = |m: &Money| -> Result<String, WordsError> {
                if unitless {
                    Ok(m.figure(lang))
                } else {
                    m.formatted(w)
                }
            };
            let note = if unitless {
                w.line("card.hero.no_currency_suffix")?
            } else {
                String::new()
            };
            if let Some(used) = t.used.as_ref().filter(|u| u.same_kind(&t.limit)) {
                let left = Money {
                    minor: (t.limit.minor - used.minor).max(0),
                    ..t.limit.clone()
                };
                // The Card's own total and the extra room apart, as KIFF says them.
                if let (Some(extra), Some(own)) = (&t.extra, &t.base) {
                    let until = match &t.room_ends {
                        Some(ends) => w.get("card.until_ends", &[("ends", ends)])?,
                        None => String::new(),
                    };
                    return Ok(Some((
                        shown(&left)?,
                        w.get(
                            "card.left_extra_room",
                            &[
                                ("current", &t.window.current),
                                ("own", &shown(own)?),
                                ("extra", &shown(extra)?),
                                ("until", &until),
                                ("note", &note),
                            ],
                        )?,
                    )));
                }
                return Ok(Some((
                    shown(&left)?,
                    w.get(
                        "card.hero.left_of_note",
                        &[
                            ("window", &t.window.current),
                            ("limit", &shown(&t.limit)?),
                            ("note", &note),
                        ],
                    )?,
                )));
            }
            return Ok(Some((
                shown(&t.limit)?,
                w.get(
                    "card.hero.use_unavailable_note",
                    &[("label", &t.window.label), ("note", &note)],
                )?,
            )));
        }
        if let Some(c) = &self.calls {
            return Ok(Some(match c.used {
                None => (
                    c.limit.to_string(),
                    w.get(
                        "card.hero.calls_use_unavailable",
                        &[("window", &c.window.label)],
                    )?,
                ),
                Some(used) => {
                    let left = (c.limit - used).max(0);
                    (
                        left.to_string(),
                        w.count(
                            "card.hero.calls_left_of",
                            left,
                            &[
                                ("window", &c.window.current),
                                ("limit", &c.limit.to_string()),
                            ],
                        )?,
                    )
                }
            }));
        }
        Ok(None)
    }

    /// `OverviewView.cardLine`: the hero's figure and words, "9 calls left today of 10".
    pub fn line(&self, w: &WordBook) -> Result<Option<String>, WordsError> {
        Ok(self
            .hero(w)?
            .map(|(figure, words)| format!("{figure} {words}")))
    }
}
