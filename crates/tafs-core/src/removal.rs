//! Removing a taf for good (`AppModel.removeTaf`, `TafsCloud.Removal`): approved first, then its
//! Cards revoked at KIFF (they're KIFF's, so the device revokes them, one plain call each), then
//! tafs cloud's `DELETE` with that approval. tafs cloud ends its connections, schedule, runs and
//! chats, memory, key and name, and says which it did.

use serde::Deserialize;

use crate::words::{WordBook, WordsError};

/// tafs cloud's answer (`Removal`): whether every part went, which did, which didn't.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Removal {
    pub done: bool,
    pub removed: Vec<String>,
    pub failed: Vec<String>,
}

pub fn read(body: &[u8]) -> Option<Removal> {
    serde_json::from_slice(body).ok()
}

/// What became of its Cards (`removalCards`'s first part): all revoked, or how many KIFF didn't.
pub fn cards(w: &WordBook, cards_left: usize) -> Result<String, WordsError> {
    if cards_left == 0 {
        w.line("taf.remove.cards_revoked")
    } else {
        w.get(
            "card.kiff_did_not_revoke_cards",
            &[("cards_left", &cards_left.to_string())],
        )
    }
}

/// A part that didn't go, in the app's words (`removalStep`); one it doesn't know, as sent.
fn step(w: &WordBook, step: &str) -> String {
    let key = match step {
        "connections" => "taf.remove.step_connections",
        "schedule" => "taf.remove.step_schedule",
        "history" => "taf.remove.step_history",
        "memory" => "taf.remove.step_memory",
        "key" => "taf.remove.step_key",
        "name" => "taf.remove.step_name",
        _ => return step.to_string(),
    };
    w.line(key).unwrap_or_else(|_| step.to_string())
}

/// The line once tafs cloud answered: none when every part went; else what wasn't finished,
/// after what became of its Cards and its connections.
pub fn after(
    w: &WordBook,
    cards_left: usize,
    removal: &Removal,
) -> Result<Option<String>, WordsError> {
    if removal.done {
        return Ok(None);
    }
    let mut c = cards(w, cards_left)?;
    if removal.removed.iter().any(|r| r == "connections") {
        c.push_str(&w.line("connect.connections_disconnected")?);
    }
    let failed: Vec<String> = removal.failed.iter().map(|s| step(w, s)).collect();
    Ok(Some(w.get(
        "taf.remove.not_finished_after",
        &[("cards", &c), ("failed", &failed.join(", "))],
    )?))
}

/// The line when tafs cloud didn't answer the removal (`cloudDidNotAnswer`): what became of its
/// Cards, and why.
pub fn cloud_did_not_answer(
    w: &WordBook,
    cards_left: usize,
    why: &str,
) -> Result<String, WordsError> {
    w.get(
        "taf.remove.cloud_did_not_answer",
        &[("cards", &cards(w, cards_left)?), ("why", why)],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn en() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    #[test]
    fn a_removal_all_done_says_nothing_more() {
        let r = read(br#"{"done":true,"removed":["connections","schedule"],"failed":[]}"#).unwrap();
        assert_eq!(after(&en(), 0, &r).unwrap(), None);
        assert!(read(b"{}").is_none());
    }

    #[test]
    fn a_part_left_is_named_after_the_cards_and_connections() {
        let r = Removal {
            done: false,
            removed: vec!["connections".into()],
            failed: vec!["memory".into(), "odd".into()],
        };
        assert_eq!(
            after(&en(), 0, &r).unwrap().unwrap(),
            "Its Cards are revoked in KIFF. Its access to apps is removed. Not finished: its memory, odd couldn't be removed. tafs tries again each time it refreshes."
        );
        let kept = after(
            &en(),
            2,
            &Removal {
                done: false,
                removed: vec![],
                failed: vec!["key".into()],
            },
        )
        .unwrap()
        .unwrap();
        assert!(
            kept.starts_with("KIFF didn't revoke 2 of its Cards"),
            "{kept}"
        );
    }

    #[test]
    fn no_answer_says_what_became_of_the_cards() {
        assert!(cloud_did_not_answer(&en(), 0, "offline")
            .unwrap()
            .starts_with("Its Cards are revoked in KIFF. tafs cloud didn't answer (offline)."));
    }

    #[test]
    fn every_shipped_language_has_the_lines() {
        let r = Removal {
            done: false,
            removed: vec!["connections".into()],
            failed: vec!["schedule".into(), "history".into(), "name".into()],
        };
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            for left in [0, 1, 3] {
                let l = after(&w, left, &r).unwrap().unwrap();
                assert!(!l.contains('{'), "{lang}: {l}");
                assert!(!cloud_did_not_answer(&w, left, "x").unwrap().contains('{'));
            }
        }
    }
}
