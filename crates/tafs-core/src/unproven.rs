//! "Did this happen?" (the iPad's `DidThisHappenCard` and `KIFFClient.Unproven`): a money call
//! whose outcome KIFF can't prove, because its answer was lost, or the tool refused it with
//! nothing proving no effect, or it's one of the first outcomes of a money operation on
//! probation. The owner says whether it happened: No gives the Card its room back, Yes keeps it
//! counted.
//!
//! The answer is signed like an approval, and a Linux computer approves nothing (its key is in
//! software). So the card shows the call and why KIFF asks, and in place of No and Yes, that this
//! device can't approve: it's answered on the phone, as tafs-macos #76 does.

use serde::Deserialize;

use crate::money::Money;
use crate::words::{WordBook, WordsError};

/// One call KIFF asks about (`KIFFClient.Unproven`). Only `proposal` must be there: a missing
/// title is empty and a missing status is `unknown`, as the iPad's decoder reads them.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Unproven {
    pub proposal: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    /// `unknown`, `refused_by_tool`, or `probation`.
    #[serde(default)]
    pub status: Option<String>,
    /// In minor units of `unit`, at `scale`.
    #[serde(default)]
    pub amount: Option<i64>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub scale: Option<u32>,
    #[serde(default)]
    pub at: Option<String>,
    /// KIFF's sentence for the call ("Refund a payment: 40.00 EUR."), shown as it is.
    #[serde(default)]
    pub words: Option<String>,
    /// Why KIFF asks, in its words.
    #[serde(default)]
    pub status_words: Option<String>,
}

impl Unproven {
    /// `title ?? ""`.
    pub fn title(&self) -> &str {
        self.title.as_deref().unwrap_or("")
    }

    /// `status ?? "unknown"`.
    pub fn status(&self) -> &str {
        self.status.as_deref().unwrap_or("unknown")
    }

    /// `money`: the amount, in its unit, at its scale (2 when KIFF gives none).
    pub fn money(&self) -> Option<Money> {
        self.amount
            .and_then(|minor| Money::new(minor, self.unit.as_deref(), self.scale.unwrap_or(2)))
    }
}

/// `GET /v1/me/did-this-happen`'s answer (`didThisHappen`): `calls`, none when KIFF sends none.
/// An answer that can't be read is none read: the screen keeps what it had (`loadUnproven`).
pub fn read_unproven(body: &[u8]) -> Option<Vec<Unproven>> {
    #[derive(Deserialize)]
    struct Response {
        #[serde(default)]
        calls: Option<Vec<Unproven>>,
    }
    serde_json::from_slice::<Response>(body)
        .ok()
        .map(|r| r.calls.unwrap_or_default())
}

/// What the card draws (`DidThisHappenCard.body`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HappenedLines {
    /// The call's id, the card's own (`happened.<proposal>`).
    pub proposal: String,
    /// "Did this happen?", in amber.
    pub title: String,
    /// KIFF's words for the call, or before KIFF sent them, its title.
    pub headline: String,
    /// The amount, only when KIFF sent no words for the call.
    pub amount: Option<String>,
    /// Why KIFF asks, in its words, when it said.
    pub why: Option<String>,
    /// In place of No and Yes: this computer can't approve (`ApprovalGate`'s `ApprovalNotReady`).
    pub can_not_answer: crate::approval::NotReady,
}

/// The cards, in KIFF's order (newest first).
pub fn lines(w: &WordBook, calls: &[Unproven]) -> Result<Vec<HappenedLines>, WordsError> {
    let title = w.line("record.did_this_happen.title")?;
    let can_not_answer = crate::approval::not_ready(w, crate::approval::THIS_COMPUTER, None)?;
    calls
        .iter()
        .map(|call| {
            let amount = match (&call.words, call.money()) {
                (None, Some(m)) => Some(m.formatted(w)?),
                _ => None,
            };
            Ok(HappenedLines {
                proposal: call.proposal.clone(),
                title: title.clone(),
                headline: call
                    .words
                    .clone()
                    .unwrap_or_else(|| call.title().to_string()),
                amount,
                why: call.status_words.clone(),
                can_not_answer: can_not_answer.clone(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    #[test]
    fn kiffs_answer_reads_as_the_ipad_reads_it() {
        let body = br#"{"calls":[
            {"proposal":"p_1","agent":"ag_1","tool":"stripe.refund","title":"Refund a payment","status":"unknown",
             "amount":4000,"unit":"EUR","scale":2,"at":"2026-11-15T10:00:00Z",
             "words":"Refund a payment: 40.00 EUR.","status_words":"Its answer was lost."},
            {"proposal":"p_2"}
        ]}"#;
        let calls = read_unproven(body).unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].proposal, "p_1");
        assert_eq!(calls[0].status(), "unknown");
        assert_eq!(calls[0].money(), Money::new(4000, Some("EUR"), 2));
        // Only the proposal must be there: an empty title, status unknown, no amount.
        assert_eq!(calls[1].title(), "");
        assert_eq!(calls[1].status(), "unknown");
        assert_eq!(calls[1].money(), None);
        // No calls, or calls: null, is none waiting.
        assert_eq!(read_unproven(b"{}"), Some(vec![]));
        assert_eq!(read_unproven(br#"{"calls":null}"#), Some(vec![]));
        // A call without its proposal, or no JSON, reads nothing: the screen keeps what it had.
        assert_eq!(read_unproven(br#"{"calls":[{"title":"x"}]}"#), None);
        assert_eq!(read_unproven(b"<html>"), None);
    }

    #[test]
    fn a_scale_kiff_leaves_out_is_two() {
        let call: Unproven =
            serde_json::from_str(r#"{"proposal":"p","amount":1250,"unit":"EUR"}"#).unwrap();
        assert_eq!(call.money(), Money::new(1250, Some("EUR"), 2));
    }

    #[test]
    fn the_card_shows_kiffs_words_and_why_and_that_the_phone_answers() {
        let w = words();
        let calls = read_unproven(
            br#"{"calls":[
                {"proposal":"p_1","title":"Refund a payment","amount":4000,"unit":"EUR","scale":2,
                 "words":"Refund a payment: 40.00 EUR.","status_words":"Its answer was lost."},
                {"proposal":"p_2","title":"Pay a bill","amount":84000,"unit":"EUR","scale":2},
                {"proposal":"p_3","title":"Send an invoice"}
            ]}"#,
        )
        .unwrap();
        let cards = lines(&w, &calls).unwrap();
        assert_eq!(cards.len(), 3);
        // KIFF's words, as they are, and no amount beside them.
        assert_eq!(cards[0].title, "Did this happen?");
        assert_eq!(cards[0].headline, "Refund a payment: 40.00 EUR.");
        assert_eq!(cards[0].amount, None);
        assert_eq!(cards[0].why.as_deref(), Some("Its answer was lost."));
        // Before KIFF sent words: its title and amount, and no why.
        assert_eq!(cards[1].headline, "Pay a bill");
        assert_eq!(
            cards[1].amount,
            Some(
                Money::new(84000, Some("EUR"), 2)
                    .unwrap()
                    .formatted(&w)
                    .unwrap()
            )
        );
        assert_eq!(cards[1].why, None);
        assert_eq!(cards[2].headline, "Send an invoice");
        assert_eq!(cards[2].amount, None);
        // No and Yes are signed like an approval: this device can't, and says so.
        for card in &cards {
            assert_eq!(
                card.can_not_answer.line,
                "This device can't approve. Approve on another device signed in to your account."
            );
        }
        assert_eq!(cards[0].proposal, "p_1");
    }
}
