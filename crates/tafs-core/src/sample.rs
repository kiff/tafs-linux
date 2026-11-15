//! The sample the iPad shows, as far as the screens ported so far draw it: its five tafs
//! (`TafTemplates.all`, in the reference set's order) and the two calls their Cards held
//! (`AppModel.propose`'s `.held`: the migration Deploys' Card always asks about, and Bill pay's
//! €8,400 bill over its €5,000 each). A held call lapses 10 minutes after it was held.

use chrono::{DateTime, Duration, Local};

use crate::card::{CallsTotal, Card, EuroTotal, Window};
use crate::money::euros;
use crate::overview::OverviewTaf;
use crate::sidebar::{initial, relevant_now, HeldCall, Mode, SidebarModel, TafRow};
use crate::words::{WordBook, WordsError};

/// The sample's tafs: key in the sample's words, colour (`colorIndex`).
pub const TAFS: [(&str, i64); 5] = [
    ("deploys", 1),
    ("bill_pay", 3),
    ("refunds", 4),
    ("chargebacks", 0),
    ("access", 2),
];

pub fn held(w: &WordBook, held_at: DateTime<Local>) -> Result<Vec<HeldCall>, WordsError> {
    let lapses = held_at + Duration::minutes(10);
    Ok(vec![
        HeldCall {
            id: "sample-deploys-migration".into(),
            taf: "deploys".into(),
            summary: w.line("sample.deploys.migration")?,
            reason: w.get(
                "card.check.always_asks",
                &[("rule", &w.line("sample.deploys.rule.migrations")?)],
            )?,
            expires_at: lapses,
        },
        HeldCall {
            id: "sample-bill-pay-atlas".into(),
            taf: "bill_pay".into(),
            summary: w.line("sample.bill_pay.pay_atlas")?,
            reason: w.get(
                "card.check.over_each",
                &[("amount", &euros(5_000, w.language()))],
            )?,
            expires_at: lapses,
        },
    ])
}

/// The sample's lines of what each taf did last, for a taf no call waits on (its newest line in
/// the sample's Record).
fn last_done(w: &WordBook, key: &str) -> Result<String, WordsError> {
    w.line(match key {
        "refunds" => "sample.refunds.credit_2290",
        "chargebacks" => "sample.chargebacks.accept_3342",
        _ => "sample.access.deactivate_tom",
    })
}

/// The sidebar over the sample, as the reference set draws it (`-signedIn` for the account's
/// items, `screenshot_signed_in`).
pub fn sidebar(
    w: &WordBook,
    now: DateTime<Local>,
    screenshot_signed_in: bool,
) -> Result<SidebarModel, WordsError> {
    let held = held(w, now)?;
    let mut tafs = Vec::new();
    for (key, color) in TAFS {
        let name = w.line(&format!("sample.{key}.name"))?;
        let waiting = held.iter().find(|h| h.taf == key);
        let now_line = match waiting {
            Some(call) => w.get("taf.now.waiting_on", &[("summary", &call.summary)])?,
            None => last_done(w, key)?,
        };
        let row = TafRow {
            key: key.into(),
            initial: initial(&name),
            name,
            color_index: color,
            now: now_line,
            badge: None,
        };
        tafs.push((
            row,
            relevant_now(waiting.is_some(), false, false, None, None, now),
        ));
    }
    Ok(SidebarModel {
        mode: Mode::Sample,
        screenshot_signed_in,
        may_use_coach: true,
        held,
        tafs,
        list_loaded: true,
        home_words: None,
        account_name: None,
        cloud_needs_you: None,
    })
}

/// Each sample taf's Card in front, as the sample's Record has used it (`TafCard.merged`: the
/// first rule with a money total, else the first with a count; what its allowed calls drew).
pub fn card(w: &WordBook, key: &str) -> Result<Card, WordsError> {
    let day = || Window::new(w, "calendar_day", None, None);
    let week = || Window::new(w, "week", None, None);
    Ok(match key {
        // Deploys: its deploy, 1 of 10 today.
        "deploys" => Card {
            calls: Some(CallsTotal {
                limit: 10,
                used: Some(1),
                window: day()?,
            }),
            total: None,
        },
        // Bill pay: Northwind's €1,240 of €25,000 this week.
        "bill_pay" => Card {
            total: Some(EuroTotal {
                limit: 25_000,
                used: Some(1_240),
                window: week()?,
            }),
            calls: None,
        },
        // Refunds: #4471's €40 of €2,000 today.
        "refunds" => Card {
            total: Some(EuroTotal {
                limit: 2_000,
                used: Some(40),
                window: day()?,
            }),
            calls: None,
        },
        // Chargebacks: #3342's €35 accepted, of €500 this week.
        "chargebacks" => Card {
            total: Some(EuroTotal {
                limit: 500,
                used: Some(35),
                window: week()?,
            }),
            calls: None,
        },
        // Access: Ana's role, 1 of 40 today.
        _ => Card {
            calls: Some(CallsTotal {
                limit: 40,
                used: Some(1),
                window: day()?,
            }),
            total: None,
        },
    })
}

/// The Overview's tafs over the sample, in the list's order: a call waits on the first two.
pub fn overview(w: &WordBook, now: DateTime<Local>) -> Result<Vec<OverviewTaf>, WordsError> {
    let model = sidebar(w, now, false)?;
    let mut out = Vec::new();
    for (row, _) in model.tafs {
        out.push(OverviewTaf {
            card: Some(card(w, &row.key)?),
            key: row.key,
            name: row.name,
            initial: row.initial,
            color_index: row.color_index,
            now: row.now,
            // tafs cloud doesn't run the sample's tafs: no last result ("–").
            cloud: None,
            used_by: String::new(),
        });
    }
    Ok(out)
}
