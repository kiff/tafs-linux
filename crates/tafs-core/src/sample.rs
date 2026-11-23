//! The sample the iPad shows, as far as the screens ported so far draw it: its five tafs
//! (`TafTemplates.all`, in the reference set's order) and the three calls their Cards held
//! (`AppModel.propose`'s `.held`: the migration Deploys' Card always asks about, Late parcels'
//! reship and Support replies' promise, each of which always waits for the owner). No sample
//! moves money. A held call lapses 10 minutes after it was held.

use chrono::{DateTime, Duration, Local};

use crate::card::{CallsTotal, Card, Window};
use crate::money::Money;
use crate::overview::OverviewTaf;
use crate::sidebar::{initial, relevant_now, HeldCall, Mode, SidebarModel, TafRow};
use crate::words::{WordBook, WordsError};

/// The sample's tafs: key in the sample's words, colour (`colorIndex`).
pub const TAFS: [(&str, i64); 5] = [
    ("deploys", 1),
    ("parcels", 3),
    ("support", 4),
    ("leads", 0),
    ("access", 2),
];

pub fn held(w: &WordBook, held_at: DateTime<Local>) -> Result<Vec<HeldCall>, WordsError> {
    let lapses = held_at + Duration::minutes(10);
    let call = |id: &str,
                taf: &str,
                summary: String,
                amount: Option<Money>,
                context: String,
                reason: String,
                next: String| HeldCall {
        id: id.into(),
        taf: taf.into(),
        summary,
        amount,
        context,
        reason,
        held_at,
        expires_at: lapses,
        kiff_id: None,
        proposal_id: None,
        approvable_once: true,
        accounting_version: None,
        next_steps: vec![next],
        details: vec![],
        taf_about: None,
        taf_reason: None,
        needs_release: false,
    };
    let always_asks = |rule: &str| -> Result<String, WordsError> {
        w.get("card.check.always_asks", &[("rule", &w.line(rule)?)])
    };
    Ok(vec![
        call(
            "sample-deploys-migration",
            "deploys",
            w.line("sample.deploys.migration")?,
            None,
            w.line("sample.deploys.migration_why")?,
            always_asks("sample.deploys.rule.migrations")?,
            w.line("sample.deploys.next_step")?,
        ),
        call(
            "sample-parcels-reship",
            "parcels",
            w.line("sample.parcels.reship_5098")?,
            None,
            w.line("sample.parcels.reship_5098_why")?,
            always_asks("sample.parcels.rule.reships")?,
            w.line("sample.parcels.next_step")?,
        ),
        call(
            "sample-support-promise",
            "support",
            w.line("sample.support.promise_7802")?,
            None,
            w.line("sample.support.promise_7802_why")?,
            always_asks("sample.support.rule.promises")?,
            w.line("sample.support.next_step")?,
        ),
    ])
}

/// The sample's lines of what each taf did last, for a taf no call waits on (its newest line in
/// the sample's Record).
fn last_done(w: &WordBook, key: &str) -> Result<String, WordsError> {
    w.line(match key {
        "leads" => "sample.leads.confirm_nord",
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
            tone: Default::default(),
            rim: None,
            trial: None,
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
        account_halt: None,
        shared: Default::default(),
        others: vec![],
        setting_up: vec![],
        plan: None,
        brief_tab: None,
        finish: Default::default(),
        unreachable: None,
    })
}

/// Each sample taf's Card in front, as the sample's Record has used it (`TafCard.merged`: the
/// first rule with a count; what its allowed calls drew). No sample's Card moves money.
pub fn card(w: &WordBook, key: &str) -> Result<Card, WordsError> {
    let day = || Window::new(w, "calendar_day", None, None);
    let calls = |limit: i64, used: i64| -> Result<Card, WordsError> {
        Ok(Card {
            calls: Some(CallsTotal {
                limit,
                used: Some(used),
                window: day()?,
            }),
            total: None,
        })
    };
    match key {
        // Deploys: its deploy, 1 of 10 today.
        "deploys" => calls(10, 1),
        // Late parcels: order #5120's delay message, 1 of 100 today.
        "parcels" => calls(100, 1),
        // Support replies: ticket #7781's reply and the day's earlier ones, 12 of 40 today, as its
        // live Card on the Card tab counts.
        "support" => calls(40, 12),
        // Lead follow-up: Studio Kaya's reply and Nord Atelier's call, 2 of 50 today.
        "leads" => calls(50, 2),
        // Access: Ana's role, 1 of 40 today.
        _ => calls(40, 1),
    }
}

/// The Overview's tafs over the sample, in the list's order: a call waits on three of them (Support replies, Late parcels and Deploys).
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
            // The sample's labels are the iPad's screenshots' only.
            labels: vec![],
            rim: None,
        });
    }
    Ok(out)
}

/// The sample's Record, newest first, as `record.png`: each taf's start (its Card issued by
/// You), then the scripts' first two rounds (`AppModel.propose`): what its Card allowed, done
/// within it (`amountLine`), and what it held.
pub fn record(w: &WordBook, now: DateTime<Local>) -> Result<Vec<crate::record::Entry>, WordsError> {
    use crate::record::{Entry, Kind};
    let at = now.with_timezone(&chrono::Utc);
    // No amount: "Within the Card", capitalised as a line of its own.
    let within = || -> Result<String, WordsError> {
        let tail = w.line("card.within_card")?;
        let mut c = tail.chars();
        Ok(c.next()
            .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
            .unwrap_or_default())
    };
    let entry = |taf: &str, kind: Kind, title: String, detail: String| Entry {
        id: format!("sample-{taf}-{title}"),
        taf: taf.into(),
        kind,
        action: None,
        title,
        detail,
        at,
        made_at: None,
        owner_approved: false,
        amount: None,
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
        report: Default::default(),
    };
    let held = held(w, now)?;
    let l = |k: &str| w.line(k);
    // The scripts' second round, then their first, each in the sample's starting order (Lead
    // follow-up, Deploys, Access, Late parcels, Support replies).
    let mut out = vec![
        entry(
            "leads",
            Kind::Done,
            l("sample.leads.confirm_nord")?,
            within()?,
        ),
        entry(
            "deploys",
            Kind::Held,
            w.get("record.held", &[("summary", &held[0].summary)])?,
            held[0].reason.clone(),
        ),
        entry(
            "access",
            Kind::Done,
            l("sample.access.deactivate_tom")?,
            within()?,
        ),
        entry(
            "parcels",
            Kind::Held,
            w.get("record.held", &[("summary", &held[1].summary)])?,
            held[1].reason.clone(),
        ),
        entry(
            "support",
            Kind::Held,
            w.get("record.held", &[("summary", &held[2].summary)])?,
            held[2].reason.clone(),
        ),
        entry(
            "leads",
            Kind::Done,
            l("sample.leads.reply_kaya")?,
            within()?,
        ),
        entry(
            "deploys",
            Kind::Done,
            l("sample.deploys.deploy_api")?,
            within()?,
        ),
        entry(
            "access",
            Kind::Done,
            l("sample.access.sales_role")?,
            within()?,
        ),
        entry(
            "parcels",
            Kind::Done,
            l("sample.parcels.tell_5120")?,
            within()?,
        ),
        entry(
            "support",
            Kind::Done,
            l("sample.support.reply_7781")?,
            within()?,
        ),
    ];
    let you = l("common.you")?;
    for key in ["support", "parcels", "access", "deploys", "leads"] {
        let name = l(&format!("sample.{key}.name"))?;
        out.push(entry(
            key,
            Kind::Started,
            w.get("record.started", &[("taf", &name)])?,
            w.get("record.card_issued_by", &[("owner", &you)])?,
        ));
    }
    Ok(out)
}

/// A sample taf's Card lines, as its template has them (`TafTemplates`): counts a day, what
/// always asks, and what is never done. No sample's Card moves money.
pub fn rules(w: &WordBook, key: &str) -> Result<Vec<crate::cards::Rule>, WordsError> {
    use crate::cards::Rule;
    let day = || Window::new(w, "calendar_day", None, None);
    let rule = |label: &str| -> Result<Rule, WordsError> { Ok(Rule::new(w.line(label)?, day()?)) };
    let count = |label: &str, n: i64| -> Result<Rule, WordsError> {
        let mut r = rule(label)?;
        r.count_per_window = Some(n);
        Ok(r)
    };
    let asks = |label: &str| -> Result<Rule, WordsError> {
        let mut r = rule(label)?;
        r.always_ask = true;
        Ok(r)
    };
    let never = |label: &str| -> Result<Rule, WordsError> {
        let mut r = rule(label)?;
        r.never = true;
        r.holds = false;
        Ok(r)
    };
    Ok(match key {
        // Up to 40 replies a day, as kiff.dev/tafs's example Card says.
        "support" => vec![
            count("sample.support.rule.replies", 40)?,
            asks("sample.support.rule.promises")?,
            never("sample.support.rule.customer_data")?,
        ],
        "parcels" => vec![
            count("sample.parcels.rule.delay_messages", 100)?,
            asks("sample.parcels.rule.reships")?,
            never("sample.parcels.rule.address_changes")?,
            never("sample.parcels.rule.refunds")?,
        ],
        "access" => vec![
            count("sample.access.rule.approved_roles", 40)?,
            count("sample.access.rule.deactivations", 20)?,
            asks("sample.access.rule.elevated")?,
            never("sample.access.rule.admin_roles")?,
            never("sample.access.rule.deleting_users")?,
        ],
        "deploys" => vec![
            count("sample.deploys.rule.deploys", 10)?,
            count("sample.deploys.rule.rollbacks", 20)?,
            asks("sample.deploys.rule.migrations")?,
            never("sample.deploys.rule.destroy_data")?,
        ],
        _ => vec![
            count("sample.leads.rule.replies", 50)?,
            asks("sample.leads.rule.discounts")?,
        ],
    })
}

/// A sample taf's Cards. As the app has them: its one Card, merged from its template's lines,
/// with what the sample's Record used (`TafCard.merged`). For a screenshot of Support replies'
/// Card tab (`screenshot`), the `-screen card` list (`Screenshots.cards`): its live Card, named
/// for its helpdesk, with its replies counted against the day's limit.
pub fn full_cards(
    w: &WordBook,
    key: &str,
    screenshot: bool,
) -> Result<Vec<crate::cards::FullCard>, WordsError> {
    use crate::cards::FullCard;
    let day = || Window::new(w, "calendar_day", None, None);
    let shown = screenshot && key == "support";
    Ok(vec![FullCard {
        extra: Default::default(),
        id: format!("sample-{key}"),
        title: if shown {
            w.line("sample.card.refunds_stripe")?
        } else {
            w.line(&format!("sample.{key}.name"))?
        },
        environment: String::new(),
        live: true,
        active: true,
        expires: None,
        rules: rules(w, key)?,
        totals: if shown {
            Card {
                total: None,
                calls: Some(CallsTotal {
                    limit: 40,
                    used: Some(12),
                    window: day()?,
                }),
            }
        } else {
            card(w, key)?
        },
    }])
}

/// A sample taf's notes (`sampleNotes`): the kind of thing a real one writes down as it learns,
/// an hour, a day and three days old, newest first.
pub fn notes(
    w: &WordBook,
    key: &str,
    now: DateTime<Local>,
) -> Result<Vec<crate::setup::Note>, WordsError> {
    let at = now.with_timezone(&chrono::Utc);
    [
        ("/how-you-like-it.md", "sample.memory.explain", 3_600),
        ("/customers/regulars.md", "sample.memory.regulars", 86_400),
        ("/from-the-owner.md", "sample.memory.ask_first", 3 * 86_400),
    ]
    .into_iter()
    .enumerate()
    .map(|(i, (path, words, ago))| {
        Ok(crate::setup::Note {
            id: format!("sample-{key}-{i}"),
            path: path.into(),
            content: w.line(words)?,
            updated_at: at - Duration::seconds(ago),
        })
    })
    .collect()
}

#[cfg(test)]
mod notes_tests {
    use super::*;

    fn now() -> DateTime<Local> {
        chrono::TimeZone::with_ymd_and_hms(&Local, 2026, 11, 23, 9, 0, 0).unwrap()
    }

    #[test]
    fn a_sample_taf_has_three_notes_newest_first() {
        let n = notes(
            &WordBook::load(&WordBook::default_root(), "en").unwrap(),
            "support",
            now(),
        )
        .unwrap();
        assert_eq!(n.len(), 3);
        assert_eq!(n[0].title(), "how you like it");
        assert_eq!(n[1].title(), "customers · regulars");
        assert!(n.windows(2).all(|p| p[0].updated_at > p[1].updated_at));
        assert!(n.iter().all(|x| x.id.starts_with("sample-support-")));
    }
}
