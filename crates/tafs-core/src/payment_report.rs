//! "Report this payment" (`ReportPaymentSheet.swift`, `ReceiptView.offersReport`, kiff/tafs
//! `docs/record.md`): on a payment the person didn't expect, one tap reports it. KIFF marks its
//! receipt and tafs cloud halts its taf, as the halt button does. It takes no note and no reason,
//! and signs nothing, since it only takes authority away; resuming the taf stays behind the phone.
//!
//! tafs cloud decides who is offered it (an owner or admin, a payment with a receipt): an
//! Activity entry carries `report` only then, and `reported_at` once it's reported.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::halt::Halt;
use crate::record::Entry;
use crate::words::{WordBook, WordsError};

/// tafs cloud's offer on an Activity entry (`ReportOffer`): the payment's receipt id, for the
/// report, and when it was made (the sheet's date).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Offer {
    pub trace_id: String,
    pub at: String,
}

/// What a Record line knows of reporting it (`reportTraceID`, `reportMadeAt`, `reportedAt`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Reportable {
    /// Its receipt's id, only where tafs cloud offers the report.
    pub trace_id: Option<String>,
    /// When the payment was made, as the offer says.
    pub made_at: Option<DateTime<Utc>>,
    /// When it was reported: the button is gone.
    pub reported_at: Option<DateTime<Utc>>,
}

impl Reportable {
    /// From an Activity entry's `report` and `reported_at`. A time that can't be read is none, as
    /// the iPad's `TafsCloud.isoDate`; an offer with no readable date still stands.
    pub fn from_cloud(offer: Option<&Offer>, reported_at: Option<&str>) -> Reportable {
        Reportable {
            trace_id: offer.map(|o| o.trace_id.clone()),
            made_at: offer.and_then(|o| iso_date(&o.at)),
            reported_at: reported_at.and_then(iso_date),
        }
    }
}

/// `TafsCloud.isoDate`: an ISO 8601 time, with or without fractions of a second.
pub fn iso_date(text: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

/// `offersReport`: the button shows where tafs cloud sent an offer and the payment isn't
/// reported yet, here or just now.
pub fn offers_report(e: &Entry, reported_now: bool) -> bool {
    e.report.trace_id.is_some() && e.report.reported_at.is_none() && !reported_now
}

/// `POST /v1/record/{trace_id}/report` at tafs cloud, the id escaped as one segment.
pub fn path(trace_id: &str) -> String {
    format!("/v1/record/{}/report", crate::access::escaped(trace_id))
}

/// What reporting answers (`PaymentReport`): when KIFF marked it, the taf it halted, and that
/// taf's halt. Reporting again answers the same.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Answer {
    pub reported_at: String,
    #[serde(default)]
    pub taf: Option<String>,
    pub halt: Halt,
}

/// tafs cloud's answer, or none when it can't be read.
pub fn read(body: &[u8]) -> Option<Answer> {
    serde_json::from_slice(body).ok()
}

/// The sheet's lines (`ReportPaymentSheet.body`), with the entry's own taf, amount and date.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lines {
    pub title: String,
    /// "Report a payment you didn't expect: {taf} paid {amount} on {date}. We halt {taf}…"
    pub intro: String,
    /// "Report and halt {taf}".
    pub submit: String,
    pub cancel: String,
    /// "Couldn't send the report. Nothing changed. Try again."
    pub failed: String,
    /// "Reported. {taf} is halted…", once tafs cloud answered.
    pub done: String,
    pub done_button: String,
    /// The receipt's button: "Report this payment".
    pub button: String,
}

pub fn lines(w: &WordBook, e: &Entry, taf_name: Option<&str>) -> Result<Lines, WordsError> {
    let taf = taf_name.unwrap_or("–");
    let at = e.report.made_at.unwrap_or(e.at);
    let local = crate::account_time::local(&at).date();
    let date = format!(
        "{}{}",
        crate::locale::long_date(w.language(), local),
        crate::account_time::label(&at)
    );
    let amount = e.amount_text(w)?.unwrap_or_default();
    Ok(Lines {
        title: w.line("record.report.title")?,
        intro: w.get(
            "record.report.intro",
            &[("taf", taf), ("amount", &amount), ("date", &date)],
        )?,
        submit: w.get("record.report.submit", &[("taf", taf)])?,
        cancel: w.line("common.cancel")?,
        failed: w.line("record.report.failed")?,
        done: w.get("record.report.done", &[("taf", taf)])?,
        done_button: w.line("common.done")?,
        button: w.line("record.report.button")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Amount, Kind};
    use chrono::TimeZone;

    fn entry() -> Entry {
        Entry {
            id: "e".into(),
            taf: "rae".into(),
            kind: Kind::Done,
            title: "Refund a payment".into(),
            detail: "Done".into(),
            at: Utc.with_ymd_and_hms(2026, 10, 21, 9, 0, 0).unwrap(),
            made_at: None,
            owner_approved: false,
            amount: Some(Amount {
                value: "150.00".into(),
                unit: Some("EUR".into()),
            }),
            read_only: false,
            entity_id: None,
            on_hash: None,
            on_arg: None,
            on_value: None,
            answer: None,
            receipt: None,
            proposal: None,
            reason: None,
            refused_by_tool: false,
            refused_by: None,
            report: Reportable::default(),
        }
    }

    #[test]
    fn the_button_shows_only_on_an_offer_not_yet_reported() {
        let mut e = entry();
        assert!(!offers_report(&e, false));
        e.report = Reportable::from_cloud(
            Some(&Offer {
                trace_id: "01K/TRACE".into(),
                at: "2026-10-20T08:00:00.123Z".into(),
            }),
            None,
        );
        assert!(offers_report(&e, false));
        assert!(!offers_report(&e, true));
        assert_eq!(
            e.report.made_at,
            Some(
                Utc.with_ymd_and_hms(2026, 10, 20, 8, 0, 0).unwrap()
                    + chrono::Duration::milliseconds(123)
            )
        );
        e.report.reported_at = iso_date("2026-10-21T10:00:00Z");
        assert!(!offers_report(&e, false));
        // A date that can't be read is none; the offer stands.
        let odd = Reportable::from_cloud(
            Some(&Offer {
                trace_id: "t".into(),
                at: "yesterday".into(),
            }),
            Some("not a time"),
        );
        assert_eq!(odd.trace_id.as_deref(), Some("t"));
        assert_eq!((odd.made_at, odd.reported_at), (None, None));
    }

    #[test]
    fn the_report_goes_to_the_receipts_own_path() {
        assert_eq!(path("01K/TRACE"), "/v1/record/01K%2FTRACE/report");
        assert!(!crate::signed::signed_at_cloud("POST", &path("t")));
    }

    #[test]
    fn the_answer_carries_the_tafs_halt() {
        let a = read(br#"{"reported_at":"2026-10-21T10:00:00Z","taf":"t1","halt":{"state":"halting","runtime":true,"kiff":false,"planque":false,"account":false}}"#).unwrap();
        assert_eq!(a.taf.as_deref(), Some("t1"));
        assert_eq!(a.halt.state, crate::halt::State::Halting);
        assert!(read(b"{}").is_none());
    }

    #[test]
    fn the_sheet_says_the_tafs_name_amount_and_the_payments_date() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let mut e = entry();
        e.report.made_at = iso_date("2026-10-20T08:00:00Z");
        let l = lines(&w, &e, Some("Rae")).unwrap();
        assert_eq!(l.title, "Report this payment");
        assert!(
            l.intro.starts_with(
                "Report a payment you didn't expect: Rae paid €150 on October 20, 2026"
            ),
            "{}",
            l.intro
        );
        assert_eq!(l.submit, "Report and halt Rae");
        assert!(l.done.starts_with("Reported. Rae is halted"));
        // No name known: a dash, never a hole.
        assert!(lines(&w, &e, None).unwrap().submit.ends_with('–'));
    }

    #[test]
    fn every_shipped_language_has_the_sheet() {
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            let l = lines(&w, &entry(), Some("Rae")).unwrap_or_else(|e| panic!("{lang}: {e:?}"));
            assert!(!l.intro.contains('{'), "{lang}");
        }
    }
}
