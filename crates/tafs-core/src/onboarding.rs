//! The first run after one sign-in (kiff/tafs `docs/onboarding.md`, the iPad's `Onboarding`):
//! the welcome with one tick for the terms, KIFF's sign-in in the browser, the three cards on how
//! tafs works, and the first Home. On Linux it's on: there's no build without it.

use std::collections::BTreeMap;
use std::path::Path;

use crate::words::{WordBook, WordsError};

/// The age every country reads (`MinimumAge.standard`).
pub const STANDARD_AGE: u32 = 16;

/// legal-i18n's table of the age of digital consent where the law asks more than 16
/// (`app/strings/ages.json`, the iPad's generated `MinimumAge`), by region.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MinimumAge {
    default: u32,
    by_region: BTreeMap<String, u32>,
}

impl MinimumAge {
    /// Read from the words' root (`ages.json` beside the languages).
    pub fn load(root: &Path) -> MinimumAge {
        let read = || -> Option<MinimumAge> {
            let text = std::fs::read_to_string(root.join("ages.json")).ok()?;
            let map: BTreeMap<String, u32> = serde_json::from_str(&text).ok()?;
            let default = *map.get("default")?;
            let by_region = map.into_iter().filter(|(k, _)| k != "default").collect();
            Some(MinimumAge { default, by_region })
        };
        read().unwrap_or(MinimumAge {
            default: STANDARD_AGE,
            by_region: BTreeMap::new(),
        })
    }

    /// The age in a region, or the default where the table has none or no region is known.
    pub fn of(&self, region: Option<&str>) -> u32 {
        region
            .and_then(|r| self.by_region.get(&r.to_uppercase()))
            .copied()
            .unwrap_or(self.default)
    }
}

/// This computer's region, from the locale the person set (`LC_ALL`, `LC_MESSAGES`, `LANG`):
/// "de_AT.UTF-8" is AT. None for "C", "POSIX" or a locale with no region.
pub fn region_from_env() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|v| std::env::var(v).ok())
        .find(|v| !v.is_empty())
        .and_then(|v| region_of(&v))
}

/// A locale's region: "pt_BR.UTF-8" is BR, "zh-Hant-TW" is TW.
pub fn region_of(locale: &str) -> Option<String> {
    let tag = locale.split(['.', '@']).next()?;
    tag.split(['_', '-'])
        .skip(1)
        .find(|p| p.len() == 2 && p.chars().all(|c| c.is_ascii_alphabetic()))
        .map(str::to_uppercase)
}

/// The welcome's age line for the person's country: its own age where the law asks more than 16,
/// else the line every country reads (`Onboarding.ageLine`).
pub fn age_line(
    w: &WordBook,
    ages: &MinimumAge,
    region: Option<&str>,
) -> Result<String, WordsError> {
    let age = ages.of(region);
    if age > STANDARD_AGE {
        w.get("onboarding.age_n", &[("age", &age.to_string())])
    } else {
        w.line("onboarding.age")
    }
}

/// The welcome's one tick (`ConsentTick`): Continue waits on it. Trying the sample data needs no
/// account, so it never waits.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ConsentTick {
    agreed: bool,
}

impl ConsentTick {
    pub fn toggle(&mut self) {
        self.agreed = !self.agreed;
    }
    pub fn agreed(&self) -> bool {
        self.agreed
    }
    pub fn can_continue(&self) -> bool {
        self.agreed
    }
}

/// Whether the three cards follow the celebration after signing in: once per computer, until
/// Get started or Skip (`Onboarding.showsCards`).
pub fn shows_cards(seen: bool) -> bool {
    !seen
}

/// The first Home's words, for someone who may use Coach: tafs cloud's own empty line when it
/// sends one, else the app's "No tafs yet" and what Coach does (`firstHomeLines`).
pub fn first_home_lines(
    w: &WordBook,
    cloud: Option<&str>,
) -> Result<(String, Option<String>), WordsError> {
    match cloud.filter(|c| !c.is_empty()) {
        Some(c) => Ok((c.to_string(), None)),
        None => Ok((
            w.line("home.no_tafs")?,
            Some(w.line("onboarding.home_lead")?),
        )),
    }
}

/// KIFF's own Terms and Privacy, beside tafs's (`Legal.kiffPage`): in the app's language where
/// kiff.dev serves it, else in English at the unprefixed path.
pub fn kiff_page(language: &str, name: &str) -> String {
    let code = crate::language::kiff_code(language);
    if crate::account::LEGAL_LANGUAGES.contains(&code) {
        format!("https://kiff.dev/{code}/{name}")
    } else {
        format!("https://kiff.dev/{name}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(lang: &str) -> WordBook {
        WordBook::load(&WordBook::default_root(), lang).unwrap()
    }

    #[test]
    fn the_age_line_follows_the_country() {
        let ages = MinimumAge::load(&WordBook::default_root());
        assert_eq!(ages.of(Some("IN")), 18);
        assert_eq!(ages.of(Some("br")), 18);
        assert_eq!(ages.of(Some("FR")), 16);
        assert_eq!(ages.of(None), 16);
        let en = words("en");
        assert_eq!(
            age_line(&en, &ages, Some("ID")).unwrap(),
            "You must be at least 18."
        );
        assert!(age_line(&en, &ages, Some("US"))
            .unwrap()
            .starts_with("You must be at least 16, or"));
    }

    #[test]
    fn a_locale_names_its_region() {
        assert_eq!(region_of("de_AT.UTF-8").as_deref(), Some("AT"));
        assert_eq!(region_of("pt_BR").as_deref(), Some("BR"));
        assert_eq!(region_of("zh-Hant-TW").as_deref(), Some("TW"));
        assert_eq!(region_of("C.UTF-8"), None);
        assert_eq!(region_of("fr"), None);
    }

    #[test]
    fn continue_waits_on_the_tick() {
        let mut t = ConsentTick::default();
        assert!(!t.can_continue());
        t.toggle();
        assert!(t.can_continue());
        t.toggle();
        assert!(!t.agreed());
    }

    #[test]
    fn the_first_home_takes_tafs_clouds_line_first() {
        let en = words("en");
        assert_eq!(
            first_home_lines(&en, Some("Nothing here")).unwrap(),
            ("Nothing here".into(), None)
        );
        let (title, lead) = first_home_lines(&en, None).unwrap();
        assert_eq!(title, en.line("home.no_tafs").unwrap());
        assert!(lead.unwrap().starts_with("Tell Coach"));
        assert!(shows_cards(false));
        assert!(!shows_cards(true));
    }

    #[test]
    fn kiffs_own_pages() {
        assert_eq!(kiff_page("de", "terms"), "https://kiff.dev/de/terms");
        assert_eq!(
            kiff_page("zh-Hant", "privacy"),
            "https://kiff.dev/zh/privacy"
        );
        assert_eq!(kiff_page("en", "terms"), "https://kiff.dev/terms");
        assert_eq!(kiff_page("ar", "terms"), "https://kiff.dev/ar/terms");
        assert_eq!(kiff_page("sr-Latn", "terms"), "https://kiff.dev/sr/terms");
        // A language kiff.dev doesn't serve: the English page.
        assert_eq!(kiff_page("el", "terms"), "https://kiff.dev/terms");
    }

    #[test]
    fn every_language_has_the_welcomes_words() {
        for lang in crate::words::SHIPPED {
            let w = words(lang);
            for k in [
                "onboarding.age",
                "onboarding.kiff_terms",
                "onboarding.tafs_terms",
                "onboarding.kiff_privacy",
                "onboarding.tafs_privacy",
                "onboarding.tafs_title",
                "onboarding.tafs_lead",
                "onboarding.card_title",
                "onboarding.card_lead",
                "onboarding.control_title",
                "onboarding.get_started",
                "onboarding.skip",
                "onboarding.how_it_works",
                "onboarding.terms_changed",
                "onboarding.home_lead",
            ] {
                w.line(k).unwrap_or_else(|e| panic!("{lang}: {e}"));
            }
            w.get("onboarding.age_n", &[("age", "18")]).unwrap();
            w.get(
                "onboarding.consent_tick",
                &[("kiff_terms", "a"), ("tafs_terms", "b")],
            )
            .unwrap();
            w.get(
                "onboarding.privacy_links",
                &[("kiff_privacy", "a"), ("tafs_privacy", "b")],
            )
            .unwrap();
        }
    }
}
