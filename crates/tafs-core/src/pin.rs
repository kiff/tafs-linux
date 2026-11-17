//! The chat pin (`TafChatView`'s and `SetupChatView`'s `pinNext`, `pinned`, `PinnedTurn`): a sent
//! message goes to the top of the thread at once and stays there while the answer grows below it,
//! in a taf's chat and in Coach (the iPad's 845 and 859 tests, `ChatPinUITests`). The room under
//! it is the view less its turn, so nothing moves as the answer grows; the owner scrolling hands
//! control back; ↓ (Jump to the latest) lets it go. What's drawn is the app's: this is the rules.

use std::collections::HashSet;

/// `SetupThread.matchesSent`: tafs cloud's copy of a message is the words sent, or ends with them
/// after a blank line (what it put before them, an answer to a card).
pub fn matches_sent(stored: &str, sent: &str) -> bool {
    stored == sent || stored.ends_with(&format!("\n\n{sent}"))
}

/// Something in the thread the pin can hold: the owner's message, one written to the run, or one
/// tafs cloud holds; by its id, with its words. A message not sent is one too, to know it: it lets
/// the pin go.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub id: String,
    pub words: String,
    /// The owner's message in the thread (`m|`): its copy can take its place under another id.
    pub message: bool,
    /// A message that couldn't go (Not sent, with Retry).
    pub unsent: bool,
}

/// What the view does after a draw.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Move {
    /// Nothing pinned: as before (the latest while following, else where it was).
    None,
    /// Pin this to the top: a message just sent (`animated`) or its copy (not).
    Pin { id: String, animated: bool },
    /// The pinned one, back at the top, unless the owner scrolled since.
    Repin { id: String },
    /// The pinned message is gone and nothing took its place: to the latest, following.
    ToLatest,
}

/// The pin's state (`pinNext`, `pinText`, `sentBefore`, `pinned`, `pinnedWords`, `touched`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pin {
    /// What the owner just sent, trimmed, and what the thread held then.
    next: Option<(String, HashSet<String>)>,
    pinned: Option<String>,
    words: Option<String>,
    /// The owner scrolled since the pin.
    pub touched: bool,
}

impl Pin {
    /// Send: the next message the owner sends goes to the top. The earlier pin goes now.
    pub fn sent(&mut self, words: &str, before: impl IntoIterator<Item = String>) {
        self.next = Some((words.trim().to_string(), before.into_iter().collect()));
        self.pinned = None;
        self.words = None;
        self.touched = false;
    }

    /// The words didn't go (not tried): nothing new is pinned.
    pub fn cancel(&mut self) {
        self.next = None;
    }

    /// What's pinned now.
    pub fn pinned(&self) -> Option<&str> {
        self.pinned.as_deref()
    }

    /// ↓, or the owner reached the latest by hand: the pin goes, and the thread follows again.
    pub fn release(&mut self) {
        self.pinned = None;
        self.words = None;
    }

    /// The owner scrolled: the pin stays, but the view is theirs.
    pub fn touch(&mut self) {
        if self.pinned.is_some() {
            self.touched = true;
        }
    }

    /// A draw with these candidates, in the thread's order: what the view does
    /// (`onChange(of: items)`, `pinBubble`, `onChange(of: unsent)`).
    pub fn drawn(&mut self, candidates: &[Candidate]) -> Move {
        let ids: HashSet<&str> = candidates.iter().map(|c| c.id.as_str()).collect();
        // The pinned message gone: its copy, by its words, takes the pin, or it goes to the latest.
        if let Some(p) = self.pinned.clone() {
            if !ids.contains(p.as_str()) {
                let copy = self.words.as_deref().and_then(|w| {
                    candidates
                        .iter()
                        .rev()
                        .find(|c| c.message && matches_sent(&c.words, w))
                });
                match copy {
                    Some(c) => self.pinned = Some(c.id.clone()),
                    None => {
                        self.release();
                        return Move::ToLatest;
                    }
                }
            }
        }
        // What the owner just sent, new since the send: never a reply that came first, nor an
        // older copy of the same words.
        if let Some((words, before)) = &self.next {
            if let Some(c) = candidates
                .iter()
                .rev()
                .find(|c| !before.contains(&c.id) && matches_sent(&c.words, words))
            {
                // It couldn't go: the pin goes, and the chat to its end, as Android's does (and
                // the iPad is to).
                if c.unsent {
                    self.next = None;
                    self.release();
                    return Move::ToLatest;
                }
                let id = c.id.clone();
                self.words = c.message.then(|| words.clone());
                self.next = None;
                self.pinned = Some(id.clone());
                self.touched = false;
                return Move::Pin { id, animated: true };
            }
        }
        match &self.pinned {
            Some(id) if !self.touched => Move::Repin { id: id.clone() },
            _ => Move::None,
        }
    }
}

/// `PinnedTurn.room`: the room under the pinned message's turn, so it sits at the top and the
/// answer grows into the room. A whole view until the turn is laid out.
pub fn room(view: f64, top: Option<f64>, foot: Option<f64>) -> f64 {
    match (top, foot) {
        (Some(top), Some(foot)) if foot >= top => (view - (foot - top)).max(0.0),
        _ => view.max(0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, words: &str) -> Candidate {
        Candidate {
            id: id.into(),
            words: words.into(),
            message: true,
            unsent: false,
        }
    }

    fn ids(c: &[Candidate]) -> Vec<String> {
        c.iter().map(|c| c.id.clone()).collect()
    }

    #[test]
    fn a_sent_message_goes_to_the_top_at_once_and_stays_while_the_answer_grows() {
        let mut pin = Pin::default();
        let thread = vec![m("m|1", "Hi")];
        pin.sent("  Which refunds went out?  ", ids(&thread));
        // Shown at once, before any answer (845: within a moment of Send).
        let shown = vec![m("m|1", "Hi"), m("m|local", "Which refunds went out?")];
        assert_eq!(
            pin.drawn(&shown),
            Move::Pin {
                id: "m|local".into(),
                animated: true
            }
        );
        // While it works and writes: back at the top on every draw.
        assert_eq!(
            pin.drawn(&shown),
            Move::Repin {
                id: "m|local".into()
            }
        );
        // tafs cloud's copy takes its place under another id (859): the pin moves to it.
        let copied = vec![m("m|1", "Hi"), m("m|srv", "Which refunds went out?")];
        assert_eq!(pin.drawn(&copied), Move::Repin { id: "m|srv".into() });
        assert_eq!(pin.pinned(), Some("m|srv"));
    }

    #[test]
    fn neither_a_reply_first_nor_an_older_copy_of_the_same_words_is_pinned() {
        let mut pin = Pin::default();
        let thread = vec![m("m|1", "Again"), m("m|2", "Done.")];
        pin.sent("Again", ids(&thread));
        // Only what was there before: nothing pinned yet.
        assert_eq!(pin.drawn(&thread), Move::None);
        let mut later = thread.clone();
        later.push(m("m|3", "Again"));
        assert_eq!(
            pin.drawn(&later),
            Move::Pin {
                id: "m|3".into(),
                animated: true
            }
        );
        // An answer to a card stored with what came before it still matches.
        assert!(matches_sent("Answer to card c1\n\nUp to €5", "Up to €5"));
        assert!(!matches_sent("Up to €5 each", "Up to €5"));
    }

    #[test]
    fn scrolling_hands_control_back_and_down_lets_go() {
        let mut pin = Pin::default();
        pin.sent("Hi", Vec::new());
        let shown = vec![m("m|a", "Hi")];
        pin.drawn(&shown);
        pin.touch();
        // The owner's view: no repin while the answer grows.
        assert_eq!(pin.drawn(&shown), Move::None);
        assert_eq!(pin.pinned(), Some("m|a"));
        pin.release();
        assert_eq!(pin.drawn(&shown), Move::None);
        assert_eq!(pin.pinned(), None);
    }

    #[test]
    fn a_pinned_message_gone_with_no_copy_goes_to_the_latest() {
        let mut pin = Pin::default();
        pin.sent("Hi", Vec::new());
        pin.drawn(&[m("m|a", "Hi")]);
        assert_eq!(pin.drawn(&[m("m|b", "Other")]), Move::ToLatest);
        assert_eq!(pin.pinned(), None);
    }

    #[test]
    fn a_message_written_to_the_run_is_what_goes_to_the_top() {
        let mut pin = Pin::default();
        pin.sent("Skip 4412", vec!["m|1".into()]);
        let steer = Candidate {
            id: "steer|0".into(),
            words: "Skip 4412".into(),
            message: false,
            unsent: false,
        };
        assert_eq!(
            pin.drawn(&[m("m|1", "Hi"), steer.clone()]),
            Move::Pin {
                id: "steer|0".into(),
                animated: true
            }
        );
        // It has no copy: gone, the view goes to the latest.
        assert_eq!(pin.drawn(&[m("m|1", "Hi")]), Move::ToLatest);
    }

    #[test]
    fn a_message_that_couldnt_go_lets_the_pin_go() {
        let unsent = Candidate {
            id: "u|7".into(),
            words: "Skip 4412".into(),
            message: false,
            unsent: true,
        };
        // Refused before any row showed: nothing is pinned, and the chat goes to its end.
        let mut pin = Pin::default();
        pin.sent("Skip 4412", vec!["m|1".into()]);
        assert_eq!(pin.drawn(&[m("m|1", "Hi"), unsent.clone()]), Move::ToLatest);
        assert_eq!(pin.pinned(), None);
        assert_eq!(pin.drawn(&[m("m|1", "Hi"), unsent.clone()]), Move::None);
        // Shown first, then not sent: its row gives way to Not sent, which doesn't take the pin.
        let mut pin = Pin::default();
        pin.sent("Skip 4412", vec!["m|1".into()]);
        pin.drawn(&[m("m|1", "Hi"), m("m|local", "Skip 4412")]);
        assert_eq!(pin.drawn(&[m("m|1", "Hi"), unsent]), Move::ToLatest);
        assert_eq!(pin.pinned(), None);
    }

    #[test]
    fn the_room_under_the_turn_is_the_view_less_the_turn() {
        assert_eq!(room(800.0, None, None), 800.0);
        assert_eq!(room(800.0, Some(1000.0), Some(1300.0)), 500.0);
        // As the answer grows, the room shrinks by as much: nothing moves.
        assert_eq!(room(800.0, Some(1000.0), Some(1500.0)), 300.0);
        assert_eq!(room(800.0, Some(1000.0), Some(2000.0)), 0.0);
    }
}
