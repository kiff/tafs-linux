//! Every screen's lines at the pin, in every shipped language: a key the screens use that isn't
//! there, or a value a line asks for that isn't given, fails here before any screen.

use chrono::{Local, TimeZone};
use tafs_core::sample;
use tafs_core::signin::{device_name, KiffError};
use tafs_core::words::{placeholders, WordBook, SHIPPED};

fn book(lang: &str) -> WordBook {
    WordBook::load(&WordBook::default_root(), lang)
        .expect("run tools/sync.sh first: the words at the pin")
}

/// The lines the sign-in, the welcome and the sidebar read, with the values each fills.
const LINES: &[(&str, &[&str])] = &[
    ("app.name", &[]),
    ("sign_in.welcome.title", &[]),
    ("sign_in.welcome.lead", &[]),
    ("sign_in.welcome.inside_lead", &[]),
    ("sign_in.welcome.inside_rest", &[]),
    ("sign_in.welcome.outside_lead", &[]),
    ("sign_in.welcome.outside_rest", &[]),
    ("sign_in.welcome.revoked_lead", &[]),
    ("sign_in.welcome.revoked_rest", &[]),
    ("sign_in.with_kiff", &[]),
    ("sample.try_it", &[]),
    ("sign_in.pair_start", &[]),
    ("sign_in.lead", &["device"]),
    ("sign_in.your_code", &[]),
    ("sign_in.check_code", &[]),
    ("sign_in.open_kiff", &[]),
    ("sign_in.other_browser", &[]),
    ("sign_in.waiting", &[]),
    ("sign_in.agree", &["terms", "privacy"]),
    ("settings.terms", &[]),
    ("settings.privacy_policy", &[]),
    ("settings.legal_in_english", &["label"]),
    ("common.copy", &[]),
    ("common.copied", &[]),
    ("common.try_again", &[]),
    ("common.cancel", &[]),
    ("device.linux", &[]),
    ("pair.device_name", &["device"]),
    ("pair.view_only.title", &[]),
    ("pair.view_only.lead", &["device"]),
    ("pair.view_only.add", &["device"]),
    ("home.overview.title", &[]),
    ("home.overview.subtitle", &[]),
    ("home.overview.column_taf", &[]),
    ("home.overview.column_now", &[]),
    ("home.overview.column_card", &[]),
    ("home.overview.column_last_result", &[]),
    ("home.overview.each_line_opens", &[]),
    ("home.no_tafs", &[]),
    ("needs_you.answer_by", &["time"]),
    ("needs_you.nothing", &[]),
    ("needs_you.nothing_waiting", &[]),
    ("needs_you.empty_hint", &[]),
    ("needs_you.expired_line", &[]),
    ("needs_you.nothing_sent", &[]),
    ("needs_you.left", &["time"]),
    ("needs_you.expired", &[]),
    ("needs_you.answer_in_kiff_until", &[]),
    ("needs_you.answer_in_kiff_button", &[]),
    ("needs_you.a_taf", &[]),
    ("needs_you.someone_answers", &[]),
    (
        "needs_you.asks_lapses",
        &["taf", "what", "reason", "time", "nothing_sent"],
    ),
    ("needs_you.taf_says", &[]),
    ("needs_you.approve_once", &[]),
    ("needs_you.over_total", &[]),
    ("needs_you.refuse", &[]),
    ("common.details", &[]),
    ("card.held.each_call_asks", &[]),
    ("card.held.for_each", &["name"]),
    ("card.held.for_item", &["item"]),
    ("card.held.over_one_call", &["amount", "cap"]),
    ("card.held.over_total", &["amount", "cap"]),
    ("card.over_limit_calls_would", &["limit", "nth"]),
    ("card.outcome.over_one_action", &[]),
    ("card.outcome.over_total", &[]),
    ("card.outcome.not_a_resource", &[]),
    ("card.outcome.outside", &[]),
    ("card.no_currency", &["amount"]),
    (
        "card.left_extra_room",
        &["current", "own", "extra", "until", "note"],
    ),
    ("card.until_ends", &["ends"]),
    ("people.last_ran", &["time"]),
    ("overview.nothing_yet", &[]),
    ("card.hero.left_of_note", &["window", "limit", "note"]),
    ("card.hero.use_unavailable_note", &["label", "note"]),
    ("card.hero.calls_use_unavailable", &["window"]),
    ("needs_you.title", &[]),
    ("needs_you.tab_label", &["n"]),
    ("record.title", &[]),
    ("calendar.heading", &[]),
    ("usage.title", &[]),
    ("connect.apps.title", &[]),
    ("home.your_tafs", &[]),
    ("home.reason_answer_by", &["why", "time"]),
    ("home.connect_tool_set_up_your", &[]),
    ("home.tafs_shared_you_show", &[]),
    ("coach.title", &[]),
    ("coach.coach_mode", &[]),
    ("settings.title", &[]),
    ("settings.account_named", &["account"]),
    ("taf.now.waiting_on", &["summary"]),
    ("card.check.always_asks", &["rule"]),
    ("card.check.over_each", &["amount"]),
    ("error.sign_in_code_expired", &[]),
    ("error.offline", &[]),
    ("error.kiff_answered", &["status"]),
];

#[test]
fn every_line_is_there_in_every_language_with_the_values_english_has() {
    for lang in SHIPPED {
        let w = book(lang);
        for (key, values) in LINES {
            let raw = w.raw(key).unwrap_or_else(|| panic!("{lang} has no {key}"));
            let line = raw
                .as_str()
                .unwrap_or_else(|| panic!("{lang} {key} is a count"));
            let mut asked = placeholders(line);
            asked.sort();
            let mut given: Vec<String> = values.iter().map(|s| s.to_string()).collect();
            given.sort();
            assert_eq!(asked, given, "{lang} {key}");
            let filled: Vec<(&str, &str)> = values.iter().map(|v| (*v, "x")).collect();
            w.get(key, &filled).unwrap();
        }
    }
}

#[test]
fn the_device_is_named_in_each_language() {
    assert_eq!(device_name(&book("en")).unwrap(), "tafs on this computer");
    for lang in SHIPPED {
        let name = device_name(&book(lang)).unwrap();
        assert!((1..=64).contains(&name.chars().count()), "{lang}: {name}");
        assert!(!name.chars().any(char::is_control));
    }
}

#[test]
fn errors_read_in_each_language() {
    for lang in SHIPPED {
        let w = book(lang);
        for e in [
            KiffError::Expired,
            KiffError::Offline,
            KiffError::Server {
                status: 503,
                message: "x".into(),
            },
            KiffError::Unreadable,
        ] {
            assert!(!e.words(&w).unwrap().is_empty());
        }
    }
}

#[test]
fn the_sample_sidebar_is_the_reference_sets() {
    let w = book("en");
    let now = Local.with_ymd_and_hms(2026, 11, 15, 2, 52, 0).unwrap();
    let lines = sample::sidebar(&w, now, true).unwrap().lines(&w).unwrap();
    let titles: Vec<&str> = lines.screens.iter().map(|s| s.title.as_str()).collect();
    assert_eq!(
        titles,
        [
            "Needs you",
            "Record",
            "Calendar",
            "Plan & usage",
            "Connect apps"
        ]
    );
    assert_eq!(lines.screens[0].badge.as_deref(), Some("2"));
    assert_eq!(lines.screens[0].accessible, "Needs you, 2");
    assert_eq!(lines.held_label.as_deref(), Some("NEEDS YOU"));
    assert_eq!(
        lines.held[0].title,
        "Deploys: Run migration 0042_orders_index"
    );
    assert_eq!(
        lines.held[0].line,
        "Schema migrations: always asks you · answer by 03:02"
    );
    assert_eq!(lines.held[1].title, "Bill pay: Pay Atlas Freight");
    assert_eq!(lines.held[1].line, "Over €5,000 each · answer by 03:02");
    assert_eq!(lines.your_tafs_label.as_deref(), Some("YOUR TAFS"));
    let rows: Vec<(&str, &str)> = lines
        .your_tafs
        .iter()
        .map(|t| (t.name.as_str(), t.now.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            (
                "Deploys",
                "Waiting for you · Run migration 0042_orders_index"
            ),
            ("Bill pay", "Waiting for you · Pay Atlas Freight")
        ]
    );
    assert_eq!(lines.coach.as_deref(), Some("Coach"));
    assert_eq!(lines.account, "Account");
    assert_eq!(lines.no_tafs, None);
}

#[test]
fn the_plain_sample_has_no_account_items() {
    // Try it with sample data, not the reference set's: Needs you and Record only, no Coach.
    let w = book("en");
    let lines = sample::sidebar(&w, Local::now(), false)
        .unwrap()
        .lines(&w)
        .unwrap();
    assert_eq!(lines.screens.len(), 2);
    assert_eq!(lines.coach, None);
}

#[test]
fn the_sidebar_reads_in_every_language() {
    for lang in SHIPPED {
        let w = book(lang);
        let lines = sample::sidebar(&w, Local::now(), true)
            .unwrap()
            .lines(&w)
            .unwrap();
        assert_eq!(lines.screens.len(), 5, "{lang}");
        assert_eq!(lines.held.len(), 2, "{lang}");
    }
}

#[test]
fn the_sample_overview_is_the_reference_sets() {
    use tafs_core::overview;
    let w = book("en");
    let now = Local.with_ymd_and_hms(2026, 11, 15, 2, 52, 0).unwrap();
    let model = sample::sidebar(&w, now, true).unwrap();
    let tafs = sample::overview(&w, now).unwrap();
    let words = overview::needs_you_words(&w, model.needs_you_count()).unwrap();
    let o = overview::lines(&w, &tafs, &model.held, model.needs_you_count(), &words).unwrap();
    assert_eq!(o.title, "Overview");
    assert_eq!(o.subtitle, "Today, across your tafs");
    assert_eq!(o.needs_you_label.as_deref(), Some("NEEDS YOU"));
    let held: Vec<(&str, &str)> = o
        .held
        .iter()
        .map(|h| (h.title.as_str(), h.answer_by.as_str()))
        .collect();
    assert_eq!(
        held,
        [
            (
                "Deploys · Run migration 0042_orders_index",
                "answer by 03:02"
            ),
            ("Bill pay · Pay Atlas Freight", "answer by 03:02")
        ]
    );
    assert_eq!(o.rest, None);
    assert_eq!(o.columns, ["TAF", "NOW", "CARD", "LAST RESULT"]);
    let rows: Vec<(&str, &str, &str, &str)> = o
        .rows
        .iter()
        .map(|r| {
            (
                r.name.as_str(),
                r.now.as_str(),
                r.card.as_str(),
                r.last_result.as_str(),
            )
        })
        .collect();
    assert_eq!(
        rows,
        [
            (
                "Deploys",
                "Waiting for you · Run migration 0042_orders_index",
                "9 calls left today of 10",
                "–"
            ),
            (
                "Bill pay",
                "Waiting for you · Pay Atlas Freight",
                "€23,760 left this week of €25,000",
                "–"
            ),
            (
                "Refunds",
                "Credit on account #2290",
                "€1,960 left today of €2,000",
                "–"
            ),
            (
                "Chargebacks",
                "Accept dispute on #3342",
                "€465 left this week of €500",
                "–"
            ),
            (
                "Access",
                "Deactivate Tom's accounts",
                "39 calls left today of 40",
                "–"
            ),
        ]
    );
    assert_eq!(o.each_line_opens, "Each line opens the taf.");
    assert_eq!(o.empty, None);
}

#[test]
fn the_overview_reads_in_every_language_and_with_no_tafs() {
    use tafs_core::overview;
    for lang in SHIPPED {
        let w = book(lang);
        let tafs = sample::overview(&w, Local::now()).unwrap();
        let o = overview::lines(&w, &tafs, &[], 0, "").unwrap();
        assert_eq!(o.rows.len(), 5, "{lang}");
        assert_eq!(o.needs_you_label, None);
        let none =
            overview::lines(&w, &[], &[], 3, &overview::needs_you_words(&w, 3).unwrap()).unwrap();
        assert!(none.empty.is_some());
        assert!(
            none.rest.is_some(),
            "{lang}: what else waits opens Needs you"
        );
    }
}

#[test]
fn last_result_reads_the_report_first() {
    use tafs_core::overview::{last_result, CloudRun};
    let w = book("en");
    assert_eq!(last_result(&w, None).unwrap(), "–");
    let report = CloudRun {
        report: Some("\n  ## **Refunded** 3 orders\nmore".into()),
        error: None,
        last_run: Some("09:00".into()),
    };
    assert_eq!(last_result(&w, Some(&report)).unwrap(), "Refunded 3 orders");
    let failed = CloudRun {
        report: Some("  \n".into()),
        error: Some("Stripe timed out".into()),
        last_run: None,
    };
    assert_eq!(last_result(&w, Some(&failed)).unwrap(), "Stripe timed out");
    let ran = CloudRun {
        report: None,
        error: None,
        last_run: Some("09:00".into()),
    };
    assert_eq!(last_result(&w, Some(&ran)).unwrap(), "Last ran at 09:00");
    assert_eq!(
        last_result(&w, Some(&CloudRun::default())).unwrap(),
        "Nothing yet"
    );
}
