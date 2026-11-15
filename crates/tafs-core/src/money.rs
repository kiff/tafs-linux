//! An amount as the app's language writes it (`Money.figure`, `Money.formatted`): "€2,000" in
//! English, never the computer's region. The iPad formats with `Locale(identifier: <language>)`;
//! these are that locale's figures (CLDR): its grouping, decimal mark and currency pattern.
//!
//! An amount carries the unit and decimal places KIFF declares for it (`KIFFMapping.money`);
//! with no unit it's a bare number, which never stands alone where money would.

use crate::words::{WordBook, WordsError};

const NBSP: &str = "\u{a0}";
const NNBSP: &str = "\u{202f}";

/// An amount in minor units of `unit`, at `scale` (`Money`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Money {
    pub minor: i64,
    pub unit: Option<String>,
    pub scale: u32,
}

/// The most decimal places an amount may declare: 10^18 is the largest power of ten an i64 holds.
/// A server-sent scale past it makes the amount unreadable, never a figure (or a crash).
pub const MAX_SCALE: u32 = 18;

impl Money {
    pub fn eur_whole(euros: i64) -> Money {
        Money {
            minor: euros * 100,
            unit: Some("EUR".into()),
            scale: 2,
        }
    }

    /// `KIFFMapping.money`: no unit, or an empty one, is a bare number. A scale past
    /// `MAX_SCALE` can't be read: none.
    pub fn new(minor: i64, unit: Option<&str>, scale: u32) -> Option<Money> {
        if scale > MAX_SCALE {
            return None;
        }
        Some(match unit {
            Some(u) if !u.is_empty() => Money {
                minor,
                unit: Some(u.to_string()),
                scale,
            },
            _ => Money {
                minor,
                unit: None,
                scale: 0,
            },
        })
    }

    pub fn has_unit(&self) -> bool {
        self.unit.is_some()
    }

    /// The same unit and scale: the two can be added or taken apart.
    pub fn same_kind(&self, other: &Money) -> bool {
        self.unit == other.unit && self.scale == other.scale
    }

    /// `figure`: with its unit, the decimals shown only when it isn't whole.
    pub fn figure(&self, language: &str) -> String {
        let Some(unit) = &self.unit else {
            return number(self.minor, 0, language);
        };
        // Built only through `new`, which refuses a scale past `MAX_SCALE`; and if one ever got
        // here, "–" rather than a wrong figure or a panic.
        let Some(power) = 10i64.checked_pow(self.scale) else {
            return "–".into();
        };
        let n = if self.minor % power == 0 {
            number(self.minor / power, 0, language)
        } else {
            number(self.minor, self.scale, language)
        };
        let upper = unit.to_uppercase();
        if !is_iso_currency(&upper) {
            // Not a currency: the number and its unit (`numberStyle = .decimal`).
            return format!("{n} {unit}");
        }
        let symbol = symbol(&upper, language);
        match language {
            "en" => {
                // A letter symbol stands apart: "CHF 30", "$30".
                if symbol.chars().last().is_some_and(char::is_alphabetic) {
                    format!("{symbol}{NBSP}{n}")
                } else {
                    format!("{symbol}{n}")
                }
            }
            "pt" | "nl" => format!("{symbol}{NBSP}{n}"),
            _ => format!("{n}{NBSP}{symbol}"),
        }
    }

    /// `formatted`: the figure, or for a bare number, the figure saying it has no currency.
    pub fn formatted(&self, w: &WordBook) -> Result<String, WordsError> {
        let figure = self.figure(w.language());
        if self.has_unit() {
            Ok(figure)
        } else {
            w.get("card.no_currency", &[("amount", &figure)])
        }
    }
}

/// Whole euros in a shipped language.
pub fn euros(whole: i64, language: &str) -> String {
    Money::eur_whole(whole).figure(language)
}

/// A number in minor units at `decimals` places, as the language writes it.
fn number(minor: i64, decimals: u32, language: &str) -> String {
    let sign = if minor < 0 { "-" } else { "" };
    let n = minor.unsigned_abs();
    let Some(power) = 10u64.checked_pow(decimals) else {
        return "–".into();
    };
    let (whole, fraction) = (n / power, n % power);
    let (group_sep, min, decimal) = match language {
        "en" => (",", 1, "."),
        "fr" => (NNBSP, 1, ","),
        "es" => (".", 2, ","),
        "pl" => (NBSP, 2, ","),
        _ => (".", 1, ","),
    };
    let mut out = format!("{sign}{}", group(whole, group_sep, min));
    if decimals > 0 {
        out.push_str(decimal);
        out.push_str(&format!("{fraction:0width$}", width = decimals as usize));
    }
    out
}

/// Thousands, grouped once the number has `min` more digits than a group (CLDR's
/// minimumGroupingDigits: Spanish and Polish write 5000, but 25.000).
fn group(n: u64, sep: &str, min: usize) -> String {
    let digits = n.to_string();
    if digits.len() < 3 + min {
        return digits;
    }
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push_str(sep);
        }
        out.push(c);
    }
    out
}

/// The currencies tafs's amounts come in so far, by their ISO code; any other three-letter code
/// is a currency written by its code.
fn is_iso_currency(code: &str) -> bool {
    code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase())
}

/// A currency's symbol in a language (CLDR's standard symbol).
fn symbol(code: &str, language: &str) -> String {
    let s = match (code, language) {
        ("EUR", _) => "€",
        ("USD", "en") | ("USD", "de") => "$",
        ("USD", "fr") => "$US",
        ("USD", "es") | ("USD", "pt") | ("USD", "nl") => "US$",
        ("USD", _) => "USD",
        ("GBP", "fr") => "£GB",
        ("GBP", "es") | ("GBP", "pl") => "GBP",
        ("GBP", _) => "£",
        _ => code,
    };
    s.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_language_writes_its_own() {
        assert_eq!(euros(5_000, "en"), "€5,000");
        assert_eq!(euros(25_000, "en"), "€25,000");
        assert_eq!(euros(40, "en"), "€40");
        assert_eq!(euros(5_000, "de"), "5.000\u{a0}€");
        assert_eq!(euros(5_000, "fr"), "5\u{202f}000\u{a0}€");
        assert_eq!(euros(5_000, "es"), "5000\u{a0}€");
        assert_eq!(euros(25_000, "es"), "25.000\u{a0}€");
        assert_eq!(euros(5_000, "pl"), "5000\u{a0}€");
        assert_eq!(euros(25_000, "pl"), "25\u{a0}000\u{a0}€");
        assert_eq!(euros(5_000, "pt"), "€\u{a0}5.000");
        assert_eq!(euros(1_000_000, "en"), "€1,000,000");
    }

    #[test]
    fn decimals_show_only_when_not_whole() {
        let m = |minor, unit, scale| Money::new(minor, unit, scale).unwrap();
        assert_eq!(m(1050, Some("EUR"), 2).figure("en"), "€10.50");
        assert_eq!(m(1050, Some("EUR"), 2).figure("de"), "10,50\u{a0}€");
        assert_eq!(m(30000, Some("USD"), 2).figure("en"), "$300");
        assert_eq!(m(30000, Some("USD"), 2).figure("fr"), "300\u{a0}$US");
        assert_eq!(m(3000, Some("CHF"), 2).figure("en"), "CHF\u{a0}30");
        assert_eq!(m(30000, None, 0).figure("en"), "30,000");
        assert_eq!(m(5, Some("credits"), 0).figure("en"), "5 credits");
    }

    #[test]
    fn a_scale_past_eighteen_is_unreadable_never_a_crash() {
        assert!(Money::new(1, Some("EUR"), 18).is_some());
        assert_eq!(Money::new(1, Some("EUR"), 19), None);
        assert_eq!(Money::new(1, Some("EUR"), 64), None);
        assert_eq!(Money::new(1, None, 64), None);
        // Even built by hand, a figure never panics.
        let odd = Money {
            minor: 5,
            unit: Some("EUR".into()),
            scale: 64,
        };
        assert_eq!(odd.figure("en"), "–");
    }
}
