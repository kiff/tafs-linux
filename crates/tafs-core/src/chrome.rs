//! Account › Chrome (`ChromeViews.swift`, kiff/tafs `docs/chrome.md`): the Chromes paired to the account, and Unpair.
//! It shows only once planque offers Chromes and this person may connect tools: planque answers 404 while it doesn't,
//! and 403 for someone who may only view; either way the row isn't drawn, as the iPad's while `chromes` is nil.
//!
//! This computer lists them and unpairs one with its device key (planque's safe routes, `planque::Safe`). Pairing a
//! Chrome binds a phone's approval key the extension then trusts, so it stays on the phone: here, Pair a Chrome says so,
//! with "Approve on your phone instead".

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::words::{WordBook, WordsError};

/// One Chrome paired to the account (`PlanqueClient.Chrome`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chrome {
    pub id: String,
    pub name: String,
    pub browser: Option<String>,
    pub os: Option<String>,
    pub paired_at: Option<DateTime<Utc>>,
    /// The sites of its live connections, each once, in planque's order ("Your Chrome" 3 and 4):
    /// planque decides which are live (asked for or approved, not ended or failed), never this app.
    /// None from a planque that doesn't list them.
    pub sites: Vec<String>,
}

/// A connection a Chrome runs, as planque lists it (`connections`).
#[derive(Deserialize)]
struct Connection {
    #[serde(default)]
    sites: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct Read {
    chrome_id: String,
    name: String,
    #[serde(default)]
    browser: Option<String>,
    #[serde(default)]
    os: Option<String>,
    #[serde(default)]
    paired_at: Option<String>,
    #[serde(default)]
    connections: Option<Vec<Connection>>,
}

/// What planque's list answered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Listed {
    /// The Chromes, maybe none.
    Chromes(Vec<Chrome>),
    /// planque doesn't offer Chromes (404), or this person may only view (403): Account › Chrome isn't drawn.
    Hidden,
}

/// planque's answer to `GET …/chromes`, by its status: 2xx the list (none when it can't be read, as the iPad's decoder
/// fails it), 404 or 403 hidden; anything else is `None`, a read that failed.
pub fn listed(status: u16, body: &[u8]) -> Option<Listed> {
    match status {
        200..=299 => {
            #[derive(Deserialize)]
            struct List {
                chromes: Vec<Read>,
            }
            let list: List = serde_json::from_slice(body).ok()?;
            Some(Listed::Chromes(
                list.chromes
                    .into_iter()
                    .map(|r| Chrome {
                        id: r.chrome_id,
                        name: r.name,
                        browser: r.browser.filter(|b| !b.is_empty()),
                        os: r.os.filter(|o| !o.is_empty()),
                        paired_at: r
                            .paired_at
                            .and_then(|t| DateTime::parse_from_rfc3339(&t).ok())
                            .map(|t| t.with_timezone(&Utc)),
                        sites: {
                            let mut seen: Vec<String> = Vec::new();
                            for s in r
                                .connections
                                .into_iter()
                                .flatten()
                                .flat_map(|c| c.sites.unwrap_or_default())
                            {
                                if !s.trim().is_empty() && !seen.contains(&s) {
                                    seen.push(s);
                                }
                            }
                            seen
                        },
                    })
                    .collect(),
            ))
        }
        403 | 404 => Some(Listed::Hidden),
        _ => None,
    }
}

/// A Chrome's line under its name: "Chrome 157 on Windows", when planque knows both.
pub fn browser_on(w: &WordBook, c: &Chrome) -> Result<Option<String>, WordsError> {
    match (&c.browser, &c.os) {
        (Some(b), Some(o)) => Ok(Some(
            w.get("chrome.browser_on", &[("browser", b), ("system", o)])?,
        )),
        _ => Ok(None),
    }
}

/// The screen's lines (`ChromesView`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub title: String,
    pub none: String,
    pub footer: String,
    pub pair: String,
    pub profile_note: String,
    pub remove: String,
}

pub fn lines(w: &WordBook) -> Result<Lines, WordsError> {
    Ok(Lines {
        title: w.line("chrome.title")?,
        none: w.line("chrome.none")?,
        footer: w.line("chrome.footer")?,
        pair: w.line("chrome.pair")?,
        profile_note: w.line("chrome.profile_note")?,
        remove: w.line("chrome.remove")?,
    })
}

/// Unpair's question: "Unpair {name}".
pub fn remove_question(w: &WordBook, c: &Chrome) -> Result<String, WordsError> {
    w.get("chrome.remove_named", &[("name", &c.name)])
}

/// What it's used for, under its name ("Your Chrome" 4): "Used for wikipedia.org, shop.example";
/// none when it runs no connection.
pub fn used_for(w: &WordBook, c: &Chrome) -> Result<Option<String>, WordsError> {
    if c.sites.is_empty() {
        return Ok(None);
    }
    w.get("chrome.used_for", &[("sites", &c.sites.join(", "))])
        .map(Some)
}

/// What Unpair ends, in its question ("Your Chrome" 3): the connections that use it, which come
/// back only by connecting them again; none when it runs no connection.
pub fn unpair_ends(w: &WordBook, c: &Chrome) -> Result<Option<String>, WordsError> {
    if c.sites.is_empty() {
        return Ok(None);
    }
    w.get("chrome.unpair_ends", &[("sites", &c.sites.join(", "))])
        .map(Some)
}

/// Why the list couldn't be read.
pub fn not_loaded(w: &WordBook, why: &str) -> Result<String, WordsError> {
    w.get("chrome.not_loaded", &[("why", why)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_chrome_says_what_it_is_used_for_and_what_unpair_ends() {
        let Some(Listed::Chromes(c)) = listed(
            200,
            br#"{"chromes":[{"chrome_id":"chr_1","name":"Work","connections":[{"grant":"g1","name":"Rae","sites":["wikipedia.org","shop.example"]},{"grant":"g2","name":"Bo","sites":["wikipedia.org"]},{"grant":"g3","name":"Al","sites":null}]},{"chrome_id":"chr_2","name":"Home","connections":[]},{"chrome_id":"chr_3","name":"Old"}]}"#,
        ) else {
            panic!("a list");
        };
        // Each site once, in planque's order.
        assert_eq!(c[0].sites, ["wikipedia.org", "shop.example"]);
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        assert_eq!(
            used_for(&w, &c[0]).unwrap().as_deref(),
            Some("Used for wikipedia.org, shop.example")
        );
        assert_eq!(
            unpair_ends(&w, &c[0]).unwrap().as_deref(),
            Some("Unpairing ends the connections that use it: wikipedia.org, shop.example. They come back only when you connect them again.")
        );
        // None live, or a planque that doesn't list them: nothing said.
        for chrome in &c[1..] {
            assert!(chrome.sites.is_empty());
            assert_eq!(used_for(&w, chrome).unwrap(), None);
            assert_eq!(unpair_ends(&w, chrome).unwrap(), None);
        }
    }

    #[test]
    fn the_list_reads_and_a_403_or_404_hides_it() {
        let Some(Listed::Chromes(c)) = listed(
            200,
            br#"{"chromes":[{"chrome_id":"chr_1","name":"Work laptop","browser":"Chrome 157","os":"Windows","paired_at":"2026-11-22T16:30:00Z"},{"chrome_id":"chr_2","name":"Home","browser":"","os":"macOS"}]}"#,
        ) else {
            panic!("a list");
        };
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].id, "chr_1");
        assert!(c[0].paired_at.is_some());
        assert_eq!(c[1].browser, None);
        assert_eq!(listed(404, b""), Some(Listed::Hidden));
        assert_eq!(listed(403, b"{}"), Some(Listed::Hidden));
        assert_eq!(listed(500, b""), None);
        assert_eq!(listed(200, b"{}"), None);
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        assert_eq!(
            browser_on(&w, &c[0]).unwrap().as_deref(),
            Some("Chrome 157 on Windows")
        );
        assert_eq!(browser_on(&w, &c[1]).unwrap(), None);
        assert_eq!(remove_question(&w, &c[0]).unwrap(), "Unpair Work laptop");
    }

    #[test]
    fn every_shipped_language_has_the_screen() {
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            lines(&w).unwrap_or_else(|e| panic!("{lang}: {e:?}"));
            assert!(!not_loaded(&w, "x").unwrap().contains('{'), "{lang}");
        }
    }
}
