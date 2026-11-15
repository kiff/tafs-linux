//! The language the app speaks (`KIFFClient.screenLanguages`, `appLanguage`): the first of the
//! person's languages tafs ships, by its code as the app ships it ("pt_BR" is "pt"), else English.
//! On Linux the person's languages are `LANGUAGE` (a list), then `LC_ALL`, `LC_MESSAGES`, `LANG`.

use crate::words::SHIPPED;

/// The person's languages, as the environment lists them.
pub fn preferred_from_env() -> Vec<String> {
    let mut out = Vec::new();
    for var in ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"] {
        if let Ok(v) = std::env::var(var) {
            out.extend(v.split(':').filter(|s| !s.is_empty()).map(str::to_string));
        }
    }
    out
}

/// Each language by its shipped code, once, at most four (`screenLanguages`).
pub fn screen_languages(preferred: &[String]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for tag in preferred {
        let code = tag.to_lowercase();
        let code = code.split(['-', '_', '.', '@']).next().unwrap_or("");
        if let Some(shipped) = SHIPPED.iter().find(|s| **s == code) {
            if !out.contains(shipped) {
                out.push(shipped);
            }
        }
        if out.len() == 4 {
            break;
        }
    }
    out
}

/// The one language the app's screens are in (`appLanguage`).
pub fn app_language(preferred: &[String]) -> &'static str {
    screen_languages(preferred).first().copied().unwrap_or("en")
}

/// `Accept-Language` (`acceptLanguage`): the person's first four, English last when missing,
/// each after the first with a falling weight.
pub fn accept_language(preferred: &[&str]) -> String {
    let mut tags: Vec<String> = preferred
        .iter()
        .take(4)
        .map(|t| t.replace('_', "-"))
        .collect();
    if !tags.iter().any(|t| t == "en" || t.starts_with("en-")) {
        tags.push("en".into());
    }
    tags.iter()
        .enumerate()
        .map(|(i, t)| {
            if i == 0 {
                t.clone()
            } else {
                format!("{t};q={:.1}", (1.0 - i as f64 * 0.1).max(0.1))
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: &[&str]) -> Vec<String> {
        x.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn picks_the_first_shipped_language() {
        assert_eq!(app_language(&v(&["pt_BR.UTF-8"])), "pt");
        assert_eq!(app_language(&v(&["ja_JP", "fr_FR.UTF-8"])), "fr");
        assert_eq!(app_language(&v(&["C.UTF-8"])), "en");
        assert_eq!(app_language(&v(&[])), "en");
        assert_eq!(
            screen_languages(&v(&["de", "de_AT", "fr", "es", "it", "nl"])),
            vec!["de", "fr", "es", "it"]
        );
    }

    #[test]
    fn accept_language_ends_with_english() {
        assert_eq!(accept_language(&["fr"]), "fr, en;q=0.9");
        assert_eq!(accept_language(&["en"]), "en");
    }
}
