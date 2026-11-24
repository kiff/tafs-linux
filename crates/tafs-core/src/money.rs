//! An amount as the app's language writes it (`Money.figure`, `Money.formatted`): "€2,000" in
//! English, never the computer's region. The iPad formats with `Locale(identifier: <language>)`;
//! these are that locale's figures (CLDR): its grouping, decimal mark and currency pattern.
//!
//! An amount carries the unit and decimal places KIFF declares for it (`KIFFMapping.money`);
//! with no unit it's a bare number, which never stands alone where money would.

use crate::words::{WordBook, WordsError};

const NBSP: &str = "\u{a0}";

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
        // Where CLDR puts the symbol: before the number, tight ("$30", "€5,000") or with a space
        // ("€ 5.000"), or after it with a space ("5.000 €"). A letter symbol before a number
        // always stands apart: "CHF 30".
        match crate::locale::base(language) {
            "en" | "ja" | "ko" | "tr" | "id" | "th" | "zh" | "lo" | "hi" | "ur" => {
                if symbol.chars().last().is_some_and(char::is_alphabetic) {
                    format!("{symbol}{NBSP}{n}")
                } else {
                    format!("{symbol}{n}")
                }
            }
            "pt" | "nl" | "mn" | "ne" => format!("{symbol}{NBSP}{n}"),
            // Khmer writes the symbol right after the number.
            "km" => format!("{n}{symbol}"),
            // Arabic opens with a right-to-left mark, as CLDR writes it; Hebrew marks the number
            // and the symbol each.
            "ar" => format!("\u{200f}{n}{NBSP}{symbol}"),
            "he" => format!("\u{200f}{n}{NBSP}\u{200f}{symbol}"),
            // Bengali: a symbol of letters stands apart, a sign doesn't.
            "bn" => {
                if symbol.chars().next().is_some_and(char::is_alphabetic) {
                    format!("{n}{NBSP}{symbol}")
                } else {
                    format!("{n}{symbol}")
                }
            }
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
    crate::locale::number(minor, decimals, language)
}

/// A currency's minor-unit digits (ISO 4217): what its smallest unit is worth, for an amount sent
/// in it (`raise.from_cents`). Two unless ISO says otherwise.
pub fn minor_digits(code: &str) -> u32 {
    match code {
        "BIF" | "CLP" | "DJF" | "GNF" | "ISK" | "JPY" | "KMF" | "KRW" | "PYG" | "RWF" | "UGX"
        | "UYI" | "VND" | "VUV" | "XAF" | "XOF" | "XPF" => 0,
        "BHD" | "IQD" | "JOD" | "KWD" | "LYD" | "OMR" | "TND" => 3,
        "CLF" | "UYW" => 4,
        _ => 2,
    }
}

/// The currencies tafs's amounts come in so far, by their ISO code; any other three-letter code
/// is a currency written by its code.
pub(crate) fn is_iso_currency(code: &str) -> bool {
    code.len() == 3 && code.chars().all(|c| c.is_ascii_uppercase())
}

/// A currency's symbol in a language (CLDR's standard symbol, as ICU writes it).
fn symbol(code: &str, language: &str) -> String {
    let s = match (code, crate::locale::base(language)) {
        ("EUR", "hu" | "ro") => "EUR",
        ("EUR", _) => "€",
        (
            "USD",
            "en" | "de" | "ja" | "et" | "lv" | "sl" | "tr" | "mn" | "km" | "hy" | "he" | "hi"
            | "ur",
        ) => "$",
        ("USD", "fr") => "$US",
        ("USD", "bg") => "щ.д.",
        (
            "USD",
            "es" | "pt" | "nl" | "ko" | "cs" | "id" | "vi" | "th" | "zh" | "ar" | "mk" | "ne"
            | "lo" | "sr" | "sq" | "ka" | "az" | "bn" | "tk",
        ) => "US$",
        ("USD", _) => "USD",
        ("GBP", "fr") => "£GB",
        ("GBP", "ar") => "UK£",
        ("GBP", "es" | "pl" | "lt" | "sk" | "sl" | "hu" | "ro" | "bg" | "hr" | "mk") => "GBP",
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
        // The new languages, as ICU writes them.
        assert_eq!(euros(5_000, "ja"), "€5,000");
        assert_eq!(euros(5_000, "cs"), "5\u{a0}000\u{a0}€");
        // Hungarian groups from five digits (CLDR minimumGroupingDigits), as Spanish does.
        assert_eq!(euros(5_000, "hu"), "5000\u{a0}EUR");
        assert_eq!(euros(25_000, "hu"), "25\u{a0}000\u{a0}EUR");
        assert_eq!(euros(5_000, "pt-BR"), "€\u{a0}5.000");
        assert_eq!(euros(5_000, "ar"), "\u{200f}5,000\u{a0}€");
        assert_eq!(euros(5_000, "tr"), "€5.000");
        assert_eq!(euros(5_000, "km"), "5,000€");
        assert_eq!(euros(5_000, "mn"), "€\u{a0}5,000");
        assert_eq!(euros(5_000, "lo"), "€5.000");
        assert_eq!(euros(5_000, "ca"), "5.000\u{a0}€");
        // Turkmen after the number; Hindi (in lakhs) and Urdu before it, as CLDR writes them.
        assert_eq!(euros(5_000, "tk"), "5\u{a0}000\u{a0}€");
        assert_eq!(euros(150_000, "hi"), "€1,50,000");
        assert_eq!(euros(150_000, "ur"), "€150,000");
        assert_eq!(euros(5_000, "sr-Latn"), "5.000\u{a0}€");
        assert_eq!(euros(5_000, "he"), "\u{200f}5,000\u{a0}\u{200f}€");
        assert_eq!(euros(5_000, "bn"), "৫,০০০€");
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
