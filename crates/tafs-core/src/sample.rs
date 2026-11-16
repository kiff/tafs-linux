//! The sample the iPad shows, as far as the screens ported so far draw it: its five tafs
//! (`TafTemplates.all`, in the reference set's order) and the two calls their Cards held
//! (`AppModel.propose`'s `.held`: the migration Deploys' Card always asks about, and Bill pay's
//! €8,400 bill over its €5,000 each). A held call lapses 10 minutes after it was held.

use chrono::{DateTime, Duration, Local};

use crate::card::{CallsTotal, Card, MoneyTotal, Window};
use crate::money::euros;
use crate::money::Money;
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
        expires_at: lapses,
        kiff_id: None,
        proposal_id: None,
        approvable_once: true,
        accounting_version: None,
        next_steps: vec![next],
        details: vec![],
        taf_about: None,
        taf_reason: None,
    };
    Ok(vec![
        call(
            "sample-deploys-migration",
            "deploys",
            w.line("sample.deploys.migration")?,
            None,
            w.line("sample.deploys.migration_why")?,
            w.get(
                "card.check.always_asks",
                &[("rule", &w.line("sample.deploys.rule.migrations")?)],
            )?,
            w.line("sample.deploys.next_step")?,
        ),
        call(
            "sample-bill-pay-atlas",
            "bill_pay",
            w.line("sample.bill_pay.pay_atlas")?,
            Some(Money::eur_whole(8_400)),
            w.line("sample.bill_pay.pay_atlas_why")?,
            w.get(
                "card.check.over_each",
                &[("amount", &euros(5_000, w.language()))],
            )?,
            w.line("sample.bill_pay.next_step")?,
        ),
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
            tone: Default::default(),
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
        plan: None,
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
            total: Some(MoneyTotal::new(
                Money::eur_whole(25_000),
                Some(Money::eur_whole(1_240)),
                week()?,
            )),
            calls: None,
        },
        // Refunds: #4471's €40 of €2,000 today.
        "refunds" => Card {
            total: Some(MoneyTotal::new(
                Money::eur_whole(2_000),
                Some(Money::eur_whole(40)),
                day()?,
            )),
            calls: None,
        },
        // Chargebacks: #3342's €35 accepted, of €500 this week.
        "chargebacks" => Card {
            total: Some(MoneyTotal::new(
                Money::eur_whole(500),
                Some(Money::eur_whole(35)),
                week()?,
            )),
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

/// The sample's Record, newest first, as `record.png`: each taf's start (its Card issued by
/// You), then the scripts' first two rounds (`AppModel.propose`): what its Card allowed, done
/// within it (`amountLine`), and what it held.
pub fn record(w: &WordBook, now: DateTime<Local>) -> Result<Vec<crate::record::Entry>, WordsError> {
    use crate::record::{Entry, Kind};
    let at = now.with_timezone(&chrono::Utc);
    let lang = w.language();
    let within = |amount: Option<i64>| -> Result<String, WordsError> {
        let tail = w.line("card.within_card")?;
        Ok(match amount {
            Some(euros_) => format!("{}, {tail}", euros(euros_, lang)),
            None => {
                let mut c = tail.chars();
                c.next()
                    .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                    .unwrap_or_default()
            }
        })
    };
    let entry = |taf: &str, kind: Kind, title: String, detail: String| Entry {
        id: format!("sample-{taf}-{title}"),
        taf: taf.into(),
        kind,
        title,
        detail,
        at,
        owner_approved: false,
        amount: None,
        read_only: false,
        on_arg: None,
        on_value: None,
        answer: None,
        receipt: None,
        proposal: None,
        reason: None,
    };
    let held = held(w, now)?;
    let l = |k: &str| w.line(k);
    let mut out = vec![
        entry(
            "chargebacks",
            Kind::Done,
            l("sample.chargebacks.accept_3342")?,
            within(Some(35))?,
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
            within(None)?,
        ),
        entry(
            "bill_pay",
            Kind::Held,
            w.get("record.held", &[("summary", &held[1].summary)])?,
            held[1].reason.clone(),
        ),
        entry(
            "refunds",
            Kind::Done,
            l("sample.refunds.credit_2290")?,
            within(Some(25))?,
        ),
        entry(
            "chargebacks",
            Kind::Done,
            l("sample.chargebacks.answer_3318")?,
            within(Some(120))?,
        ),
        entry(
            "deploys",
            Kind::Done,
            l("sample.deploys.deploy_api")?,
            within(None)?,
        ),
        entry(
            "access",
            Kind::Done,
            l("sample.access.sales_role")?,
            within(None)?,
        ),
        entry(
            "bill_pay",
            Kind::Done,
            l("sample.bill_pay.pay_northwind")?,
            within(Some(1_240))?,
        ),
        entry(
            "refunds",
            Kind::Done,
            l("sample.refunds.refund_4471")?,
            within(Some(40))?,
        ),
    ];
    let you = l("common.you")?;
    for key in ["refunds", "bill_pay", "access", "deploys", "chargebacks"] {
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

/// A sample taf's Card lines, as its template has them (`TafTemplates`).
pub fn rules(w: &WordBook, key: &str) -> Result<Vec<crate::cards::Rule>, WordsError> {
    use crate::cards::Rule;
    let day = || Window::new(w, "calendar_day", None, None);
    let week = || Window::new(w, "week", None, None);
    let rule = |label: &str, window: Window| -> Result<Rule, WordsError> {
        Ok(Rule::new(w.line(label)?, window))
    };
    let eur = |e: i64| Some(Money::eur_whole(e));
    Ok(match key {
        "refunds" => {
            let mut a = rule("sample.refunds.rule.refunds", day()?)?;
            a.per_action = eur(150);
            a.per_window = eur(2_000);
            let mut b = rule("sample.refunds.rule.credits", day()?)?;
            b.per_action = eur(100);
            b.per_window = eur(1_000);
            let mut c = rule("sample.refunds.rule.payouts", day()?)?;
            c.never = true;
            c.holds = false;
            vec![a, b, c]
        }
        "bill_pay" => {
            let mut a = rule("sample.bill_pay.rule.bills", week()?)?;
            a.per_action = eur(5_000);
            a.per_window = eur(25_000);
            let mut b = rule("sample.bill_pay.rule.wires", day()?)?;
            b.always_ask = true;
            let mut c = rule("sample.bill_pay.rule.bank_details", day()?)?;
            c.never = true;
            c.holds = false;
            let mut d = rule("sample.bill_pay.rule.new_payees", day()?)?;
            d.never = true;
            d.holds = false;
            vec![a, b, c, d]
        }
        "access" => {
            let mut a = rule("sample.access.rule.approved_roles", day()?)?;
            a.count_per_window = Some(40);
            let mut b = rule("sample.access.rule.deactivations", day()?)?;
            b.count_per_window = Some(20);
            let mut c = rule("sample.access.rule.elevated", day()?)?;
            c.always_ask = true;
            let mut d = rule("sample.access.rule.admin_roles", day()?)?;
            d.never = true;
            d.holds = false;
            let mut e = rule("sample.access.rule.deleting_users", day()?)?;
            e.never = true;
            e.holds = false;
            vec![a, b, c, d, e]
        }
        "deploys" => {
            let mut a = rule("sample.deploys.rule.deploys", day()?)?;
            a.count_per_window = Some(10);
            let mut b = rule("sample.deploys.rule.rollbacks", day()?)?;
            b.count_per_window = Some(20);
            let mut c = rule("sample.deploys.rule.migrations", day()?)?;
            c.always_ask = true;
            let mut d = rule("sample.deploys.rule.destroy_data", day()?)?;
            d.never = true;
            d.holds = false;
            vec![a, b, c, d]
        }
        _ => {
            let mut a = rule("sample.chargebacks.rule.evidence", day()?)?;
            a.per_action = eur(500);
            a.count_per_window = Some(50);
            let mut b = rule("sample.chargebacks.rule.accepting_loss", week()?)?;
            b.per_action = eur(100);
            b.per_window = eur(500);
            vec![a, b]
        }
    })
}

/// A sample taf's Cards. As the app has them: its one Card, merged from its template's lines,
/// with what the sample's Record used (`TafCard.merged`). For a screenshot of Refunds' Card tab
/// (`screenshot`), the three `-screen card` lists (`Screenshots.cards`): a live one with its
/// total, a read-only one with its calls, and a test one.
pub fn full_cards(
    w: &WordBook,
    key: &str,
    screenshot: bool,
) -> Result<Vec<crate::cards::FullCard>, WordsError> {
    use crate::cards::{FullCard, Rule};
    let day = || Window::new(w, "calendar_day", None, None);
    if !(screenshot && key == "refunds") {
        let name = w.line(&format!("sample.{key}.name"))?;
        return Ok(vec![FullCard {
            extra: Default::default(),
            id: format!("sample-{key}"),
            title: name,
            environment: String::new(),
            live: true,
            active: true,
            expires: None,
            rules: rules(w, key)?,
            totals: card(w, key)?,
        }]);
    }
    let refunds = FullCard {
        extra: Default::default(),
        id: "sample-refunds".into(),
        title: w.line("sample.card.refunds_stripe")?,
        environment: String::new(),
        live: true,
        active: true,
        expires: None,
        rules: rules(w, "refunds")?.into_iter().take(2).collect(),
        totals: Card {
            total: Some(MoneyTotal::new(
                Money::eur_whole(2_000),
                Some(Money::eur_whole(480)),
                day()?,
            )),
            calls: None,
        },
    };
    let mut charges = Rule::new(w.line("sample.list_charges")?, day()?);
    charges.count_per_window = Some(200);
    let reads = FullCard {
        extra: Default::default(),
        id: "sample-reads".into(),
        title: w.line("sample.card.read_stripe")?,
        environment: String::new(),
        live: true,
        active: true,
        expires: None,
        rules: vec![charges, Rule::new(w.line("sample.look_up_charge")?, day()?)],
        totals: Card {
            total: None,
            calls: Some(CallsTotal {
                limit: 200,
                used: Some(20),
                window: day()?,
            }),
        },
    };
    let test = FullCard {
        extra: Default::default(),
        id: "sample-test".into(),
        title: w.line("sample.refunds_stripe_test")?,
        environment: "test".into(),
        live: false,
        ..refunds.clone()
    };
    Ok(vec![refunds, reads, test])
}
