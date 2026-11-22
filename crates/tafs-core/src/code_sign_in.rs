//! Account › Sign-in (the iPad's `CodeSignInRow`, tafs#1711): whether the person signs in with a code KIFF emails them,
//! as KIFF keeps it (`GET /v1/me/sign-in`). Off for everyone until they turn it on. Turning it off goes from here, as the
//! session; turning it on needs a recent sign-in, which a device's session never passes, so it opens KIFF's own sign-in
//! page and the setting is read again on the way back. KIFF emails the person about either change.

use serde::Deserialize;

use crate::words::{WordBook, WordsError};

/// KIFF's route for it.
pub const PATH: &str = "/v1/me/sign-in";

/// KIFF's page where it's turned on, behind a recent sign-in.
pub fn turn_on_page() -> String {
    format!("{}/dashboard/sign-in", crate::needs_you::KIFF_WEB)
}

/// The setting (`KIFFClient.CodeSignIn`): on, and whether KIFF can send codes at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub struct CodeSignIn {
    pub email_code: bool,
    pub available: bool,
}

pub fn read(body: &[u8]) -> Option<CodeSignIn> {
    serde_json::from_slice(body).ok()
}

/// Turning it off: the one change sent from here.
pub fn off_body() -> Vec<u8> {
    br#"{"email_code":false}"#.to_vec()
}

/// What the row offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offer {
    /// It's on: Turn off.
    TurnOff(String),
    /// It's off and KIFF can send codes: Turn on, and that it opens KIFF's page.
    TurnOn { button: String, note: String },
    /// KIFF can't send codes yet.
    NotAvailable(String),
}

/// The row's lines (`CodeSignInRow`): the section's title, the label and its value, the help, and what it offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub title: String,
    pub label: String,
    pub value: String,
    pub help: String,
    pub offer: Offer,
}

pub fn lines(w: &WordBook, s: CodeSignIn) -> Result<Lines, WordsError> {
    let l = |k: &str| w.line(k);
    Ok(Lines {
        title: l("settings.sign_in_code.title")?,
        label: l("settings.sign_in_code.label")?,
        value: l(if s.email_code {
            "settings.sign_in_code.on"
        } else {
            "settings.sign_in_code.off"
        })?,
        help: l("settings.sign_in_code.help")?,
        offer: if s.email_code {
            Offer::TurnOff(l("settings.sign_in_code.turn_off")?)
        } else if s.available {
            Offer::TurnOn {
                button: l("settings.sign_in_code.turn_on")?,
                note: l("settings.sign_in_code.turn_on_in_browser")?,
            }
        } else {
            Offer::NotAvailable(l("settings.sign_in_code.not_available")?)
        },
    })
}

/// Why it couldn't be read, or didn't change.
pub fn not_loaded(w: &WordBook, why: &str) -> Result<String, WordsError> {
    w.get("settings.sign_in_code.not_loaded", &[("why", why)])
}

pub fn not_changed(w: &WordBook, why: &str) -> Result<String, WordsError> {
    w.get("settings.sign_in_code.not_changed", &[("why", why)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_setting_reads_and_offers_what_it_can() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let on = read(br#"{"email_code":true,"available":true}"#).unwrap();
        let l = lines(&w, on).unwrap();
        assert_eq!(l.value, "On");
        assert_eq!(l.offer, Offer::TurnOff("Turn off".into()));
        let off = read(br#"{"email_code":false,"available":true}"#).unwrap();
        assert!(matches!(
            lines(&w, off).unwrap().offer,
            Offer::TurnOn { .. }
        ));
        let none = read(br#"{"email_code":false,"available":false}"#).unwrap();
        assert!(matches!(
            lines(&w, none).unwrap().offer,
            Offer::NotAvailable(_)
        ));
        assert!(read(b"{}").is_none());
        assert_eq!(off_body(), br#"{"email_code":false}"#);
        assert_eq!(turn_on_page(), "https://app.kiff.dev/dashboard/sign-in");
        // Turning it off is the session's own call; KIFF doesn't take it signed.
        assert!(crate::signed::Unsigned::new("PUT", PATH.into(), off_body()).is_some());
    }

    #[test]
    fn every_shipped_language_has_the_row() {
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            for s in [(true, true), (false, true), (false, false)] {
                lines(
                    &w,
                    CodeSignIn {
                        email_code: s.0,
                        available: s.1,
                    },
                )
                .unwrap_or_else(|e| panic!("{lang}: {e:?}"));
            }
            assert!(!not_loaded(&w, "x").unwrap().contains('{'), "{lang}");
            assert!(!not_changed(&w, "x").unwrap().contains('{'), "{lang}");
        }
    }
}
