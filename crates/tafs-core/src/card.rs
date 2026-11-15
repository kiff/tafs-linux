//! A Card's figure, as the iPad draws it (`Model/Models.swift`: `Window`, `CardTotal`,
//! `TafCard.hero`): what's left of its money total, else of its calls. Use KIFF couldn't read
//! shows the limit alone, never as zero left or used.

use crate::money::euros;
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

/// A money total in whole euros, the sample's (`CardTotal<Money>`): `used` is none when KIFF
/// couldn't read it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EuroTotal {
    pub limit: i64,
    pub used: Option<i64>,
    pub window: Window,
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
    pub total: Option<EuroTotal>,
    pub calls: Option<CallsTotal>,
}

impl Card {
    /// `hero`: the face's big figure and the words under it.
    pub fn hero(&self, w: &WordBook) -> Result<Option<(String, String)>, WordsError> {
        let lang = w.language();
        if let Some(t) = &self.total {
            let limit = euros(t.limit, lang);
            return Ok(Some(match t.used {
                Some(used) => (
                    euros((t.limit - used).max(0), lang),
                    w.get(
                        "card.hero.left_of_note",
                        &[
                            ("window", &t.window.current),
                            ("limit", &limit),
                            ("note", ""),
                        ],
                    )?,
                ),
                None => (
                    limit,
                    w.get(
                        "card.hero.use_unavailable_note",
                        &[("label", &t.window.label), ("note", "")],
                    )?,
                ),
            }));
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
