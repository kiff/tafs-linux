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

/// A tag's words' folder, if the app speaks it: "pt_BR" is "pt-BR", "zh_TW" and "zh-Hant" are
/// "zh-Hant", "de_AT" is "de". Simplified Chinese isn't spoken.
pub fn words_language(tag: &str) -> Option<&'static str> {
    // glibc writes Latin Serbian as a modifier, `sr_RS@latin`.
    let latin_modifier = tag.contains("@latin");
    let tag = tag.split(['.', '@']).next().unwrap_or("");
    let parts: Vec<&str> = tag.split(['-', '_']).filter(|p| !p.is_empty()).collect();
    let code = parts.first()?.to_lowercase();
    let find = |c: &str| SHIPPED.iter().find(|s| **s == c).copied();
    match code.as_str() {
        "zh" => is_traditional_chinese(&parts)
            .then(|| find("zh-Hant"))
            .flatten(),
        // Serbian ships in Latin script: a Cyrillic tag isn't the language the app ships
        // (`isLatinScript`).
        "sr" => (latin_modifier || parts.iter().skip(1).any(|p| p.eq_ignore_ascii_case("latn")))
            .then(|| find("sr-Latn"))
            .flatten(),
        "pt" if parts.iter().skip(1).any(|p| p.eq_ignore_ascii_case("BR")) => {
            find("pt-BR").or_else(|| find("pt"))
        }
        other => find(other),
    }
}

/// `isTraditionalChinese`: the script says so, or no script and a region that writes it.
fn is_traditional_chinese(parts: &[&str]) -> bool {
    let rest = &parts[1.min(parts.len())..];
    if let Some(script) = rest.iter().find(|p| p.len() == 4) {
        return script.eq_ignore_ascii_case("hant");
    }
    rest.iter()
        .find(|p| p.len() == 2)
        .is_some_and(|r| ["TW", "HK", "MO"].contains(&r.to_uppercase().as_str()))
}

/// The code KIFF and kiff.dev know a words' language by (`screenLanguages`' codes): "pt-BR" is
/// "pt", "zh-Hant" is "zh".
pub fn kiff_code(language: &str) -> &str {
    crate::locale::base(language)
}

/// Each language by its code as KIFF knows it, once, at most four (`screenLanguages`).
pub fn screen_languages(preferred: &[String]) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = Vec::new();
    for tag in preferred {
        if let Some(lang) = words_language(tag) {
            let code = kiff_code(lang);
            if !out.contains(&code) {
                out.push(code);
            }
        }
        if out.len() == 4 {
            break;
        }
    }
    out
}

/// The one language the app's screens are in (`Words.locale`), by its words' folder.
pub fn app_language(preferred: &[String]) -> &'static str {
    preferred
        .iter()
        .find_map(|t| words_language(t))
        .unwrap_or("en")
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
        assert_eq!(app_language(&v(&["pt_BR.UTF-8"])), "pt-BR");
        assert_eq!(app_language(&v(&["pt_PT.UTF-8"])), "pt");
        assert_eq!(app_language(&v(&["ja_JP", "fr_FR.UTF-8"])), "ja");
        assert_eq!(app_language(&v(&["zh_TW.UTF-8"])), "zh-Hant");
        assert_eq!(app_language(&v(&["zh-Hant-HK"])), "zh-Hant");
        assert_eq!(app_language(&v(&["zh_CN", "ar_EG"])), "ar");
        assert_eq!(app_language(&v(&["sr_RS", "fr"])), "fr");
        assert_eq!(app_language(&v(&["sr_RS@latin"])), "sr-Latn");
        assert_eq!(app_language(&v(&["sr-Latn-RS"])), "sr-Latn");
        assert_eq!(app_language(&v(&["he_IL.UTF-8"])), "he");
        assert_eq!(screen_languages(&v(&["sr_RS@latin"])), vec!["sr"]);
        assert_eq!(app_language(&v(&["C.UTF-8"])), "en");
        assert_eq!(app_language(&v(&[])), "en");
        assert_eq!(
            screen_languages(&v(&["de", "de_AT", "fr", "es", "it", "nl"])),
            vec!["de", "fr", "es", "it"]
        );
        assert_eq!(
            screen_languages(&v(&["pt_BR", "pt_PT", "zh_TW"])),
            vec!["pt", "zh"]
        );
    }

    #[test]
    fn accept_language_ends_with_english() {
        assert_eq!(accept_language(&["fr"]), "fr, en;q=0.9");
        assert_eq!(accept_language(&["en"]), "en");
    }
}
