//! An amount as the app's language writes it (`Money.figure`): "€2,000" in English, never the
//! computer's region. The iPad formats with `Locale(identifier: <language>)`; these are that
//! locale's whole-number figures for the euro, the only currency the screens ported so far show.

/// Whole euros in a shipped language.
pub fn euros(whole: i64, language: &str) -> String {
    const NBSP: &str = "\u{a0}";
    const NNBSP: &str = "\u{202f}";
    let n = whole.unsigned_abs();
    let sign = if whole < 0 { "-" } else { "" };
    match language {
        "de" | "it" => format!("{sign}{}{NBSP}€", group(n, ".", 1)),
        "fr" => format!("{sign}{}{NBSP}€", group(n, NNBSP, 1)),
        "es" => format!("{sign}{}{NBSP}€", group(n, ".", 2)),
        "pl" => format!("{sign}{}{NBSP}€", group(n, NBSP, 2)),
        "pt" | "nl" => format!("€{NBSP}{sign}{}", group(n, ".", 1)),
        _ => format!("{sign}€{}", group(n, ",", 1)),
    }
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
}
