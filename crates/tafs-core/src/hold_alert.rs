//! A held call's notification while tafs runs (`HoldNotification.swift` › `words`, `fill`, `time`;
//! `PushDelegate.swift`'s `removeAlerts(forHold:)`). The iPad gets tafs cloud's push and fills it
//! in from KIFF; a computer gets no push, so while tafs runs it reads KIFF's held calls once a
//! minute, as Windows does, and notifies each call held since the last read. Clicking one opens
//! Needs you.

use std::collections::BTreeSet;

use sha2::{Digest, Sha256};

use crate::held::{self, CardException};
use crate::words::{WordBook, WordsError};

/// How often the held calls are read while tafs runs.
pub const PERIOD: std::time::Duration = std::time::Duration::from_secs(60);

/// A hold's notification: its two lines, and the hold it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub title: String,
    pub body: String,
    pub hold: String,
}

/// "Rae needs you" / "Refund order 42, €150.00 · Over €50 each. Answer by 14:05. Nothing is sent
/// until you approve.": the taf's name when it's known, the deadline in the account's zone (with
/// the zone's short name when this computer's clock differs then).
pub fn lines(w: &WordBook, hold: &CardException, taf: Option<&str>) -> Result<Lines, WordsError> {
    let call = held::held(w, hold, "")?;
    let amount = match &call.amount {
        Some(m) => Some(m.formatted(w)?),
        None => None,
    };
    let what = [Some(call.summary.clone()), amount]
        .into_iter()
        .flatten()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    Ok(Lines {
        title: match taf {
            Some(t) => w.get("alert.taf_needs_you", &[("taf", t)])?,
            None => w.line("alert.needs_you")?,
        },
        body: w.get(
            "alert.answer_nothing_sent_until_you",
            &[
                ("what", &what),
                ("why", &call.reason),
                (
                    "zone",
                    &crate::account_time::zoned_short_time(&call.expires_at),
                ),
            ],
        )?,
        hold: hold.id.clone(),
    })
}

/// The taf a notification names (`fill`): the taf whose KIFF agent held it, Coach for Coach's own
/// agent (`setup`), none for an agent no taf has ("A taf needs you").
pub fn named<'a>(
    w: &WordBook,
    hold: &CardException,
    tafs: impl IntoIterator<Item = (&'a str, &'a str)>,
) -> Result<Option<String>, WordsError> {
    if let Some((_, name)) = tafs.into_iter().find(|(agent, _)| *agent == hold.subject) {
        return Ok(Some(name.to_string()));
    }
    if hold.subject == "setup" {
        return Ok(Some(w.line("coach.title")?));
    }
    Ok(None)
}

/// Whether this session is notified at all (`canAnswerHolds`): only someone who answers held
/// calls; none listed allows.
pub fn notifies(capabilities: Option<&[String]>) -> bool {
    capabilities.is_none_or(|c| c.iter().any(|x| x == "holds.answer"))
}

/// The calls held since the last read: their ids weren't seen. A session's first read (nothing
/// seen yet) notifies nothing, so opening tafs doesn't announce what already waits in Needs you.
pub fn fresh<'a>(
    seen: Option<&BTreeSet<String>>,
    holds: &'a [CardException],
) -> Vec<&'a CardException> {
    match seen {
        None => vec![],
        Some(seen) => holds.iter().filter(|h| !seen.contains(&h.id)).collect(),
    }
}

/// The holds whose notifications go (`removeAlerts(forHold:)`): seen before and no longer held,
/// answered elsewhere or lapsed.
pub fn gone(seen: Option<&BTreeSet<String>>, holds: &[CardException]) -> Vec<String> {
    let Some(seen) = seen else {
        return vec![];
    };
    let now: BTreeSet<&str> = holds.iter().map(|h| h.id.as_str()).collect();
    seen.iter()
        .filter(|id| !now.contains(id.as_str()))
        .cloned()
        .collect()
}

/// A hold's notification id: its id's SHA-256 in hex, so any id KIFF sends is a safe one.
pub fn tag(hold: &str) -> String {
    let digest = Sha256::digest(hold.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("hold-{hex}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(lang: &str) -> WordBook {
        WordBook::load(&WordBook::default_root(), lang).unwrap()
    }

    /// The held call `contracts.rs` reads, and a second one.
    fn holds() -> Vec<CardException> {
        let one = r#"{"id":"exc_1","subject":"rae","action":"stripe_create_refund","entity_id":"tc_9","status":"held",
            "breaches":[{"outcome":"per_action_exceeded","reason":"over","limit":10000,"quantity":"sum(amount)","unit":"EUR","scale":2}],
            "parameters":{"amount":15000},"parameter_units":{"amount":{"unit":"EUR","scale":2}},"reason":null,"units_unavailable":false,
            "next_steps":[],"created_at":"2026-11-15T12:00:00Z","expires_at":"2026-11-15T12:10:00.000Z","accounting_version":3,
            "title":"Refund a payment","arguments":{"amount":15000,"payment_intent":"pi_123","kiff_about":"Order 1042"},
            "taf_about":"Order 1042","taf_reason":"Box arrived crushed","can_approve_once":true}"#;
        let two = one.replace("exc_1", "exc_2");
        held::read_held(format!(r#"{{"exceptions":[{one},{two}]}}"#).as_bytes()).unwrap()
    }

    #[test]
    fn only_a_call_held_since_the_last_read_is_notified() {
        let h = holds();
        assert!(!h.is_empty());
        // The first read notifies nothing and removes nothing.
        assert!(fresh(None, &h).is_empty());
        assert!(gone(None, &h).is_empty());
        let mut seen: BTreeSet<String> = h.iter().map(|x| x.id.clone()).collect();
        assert!(fresh(Some(&seen), &h).is_empty());
        let first = h[0].id.clone();
        seen.remove(&first);
        seen.insert("answered".into());
        assert_eq!(
            fresh(Some(&seen), &h)
                .iter()
                .map(|x| x.id.clone())
                .collect::<Vec<_>>(),
            vec![first]
        );
        assert_eq!(gone(Some(&seen), &h), vec!["answered".to_string()]);
    }

    #[test]
    fn a_hold_is_worded_as_the_ipads() {
        let w = words("en");
        let h = &holds()[0];
        let l = lines(&w, h, Some("Rae")).unwrap();
        assert_eq!(l.title, "Rae needs you");
        assert!(
            l.body.ends_with("Nothing is sent until you approve."),
            "{}",
            l.body
        );
        assert!(
            l.body.starts_with(
                "Refund a payment, €150 · €150 is over the €100 limit for one call. Answer by "
            ),
            "{}",
            l.body
        );
        assert_eq!(l.hold, h.id);
        assert_eq!(lines(&w, h, None).unwrap().title, "A taf needs you");
        // Named: the taf whose agent held it, else Coach for `setup`, else none.
        let tafs = [(h.subject.as_str(), "Rae")];
        assert_eq!(named(&w, h, tafs).unwrap().as_deref(), Some("Rae"));
        assert_eq!(named(&w, h, []).unwrap(), None);
        let coach = CardException {
            subject: "setup".into(),
            ..h.clone()
        };
        assert_eq!(named(&w, &coach, []).unwrap().as_deref(), Some("Coach"));
        for lang in crate::words::SHIPPED {
            lines(&words(lang), h, Some("Rae")).unwrap();
            lines(&words(lang), h, None).unwrap();
        }
    }

    #[test]
    fn who_is_notified_and_the_tag() {
        assert!(notifies(None));
        assert!(notifies(Some(&["holds.answer".to_string()])));
        assert!(!notifies(Some(&["cards.manage".to_string()])));
        let t = tag("../x y");
        assert_eq!(t.len(), 5 + 64);
        assert!(t[5..].chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(tag("a"), tag("b"));
    }
}
