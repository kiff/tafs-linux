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
