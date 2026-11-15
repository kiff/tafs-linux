//! The sample the iPad shows, as far as the screens ported so far draw it: its five tafs
//! (`TafTemplates.all`, in the reference set's order) and the two calls their Cards held
//! (`AppModel.propose`'s `.held`: the migration Deploys' Card always asks about, and Bill pay's
//! €8,400 bill over its €5,000 each). A held call lapses 10 minutes after it was held.

use chrono::{DateTime, Duration, Local};

use crate::money::euros;
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
