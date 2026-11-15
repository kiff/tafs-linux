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
    ("record.title", &[]),
    ("record.lead", &[]),
    ("record.all_tafs", &[]),
    ("record.filter.all", &[]),
    ("record.filter.money_moved", &[]),
    ("record.filter.stopped", &[]),
    ("record.filter.waiting", &[]),
    ("record.hide", &[]),
    ("record.show", &[]),
    ("record.nothing_here", &[]),
    ("common.nothing_yet", &[]),
    ("record.held", &["summary"]),
    ("record.started", &["taf"]),
    ("record.card_issued_by", &["owner"]),
    ("card.within_card", &[]),
    ("common.you", &[]),
    ("calendar.today_date", &["date"]),
    ("calendar.yesterday_date", &["date"]),
    ("record.approved_by_you", &[]),
    ("record.allowed_by_card", &[]),
    ("record.approved_then_done", &[]),
    ("record.done_within_card", &[]),
    ("record.tool_refused_money", &["who"]),
    ("record.tool_refused_amount", &["who"]),
    ("record.tool_refused_counts_why", &["who", "why"]),
    ("record.not_sent_nothing_reached_why", &["allowed", "why"]),
    ("record.sent_waiting", &["allowed"]),
    ("record.answer_lost", &["allowed"]),
    ("record.not_recorded", &["allowed"]),
    ("record.the_tool", &[]),
    ("record.waiting_for_you", &[]),
    ("record.not_sent_because", &["reason"]),
    ("record.not_sent", &[]),
    ("record.connection_refused_before_sending", &[]),
    ("record.no_longer_waiting", &[]),
    ("record.nothing_sent", &[]),
    ("record.on.it", &[]),
    ("taf.section.chat", &[]),
    ("taf.section.activity", &[]),
    ("taf.section.card", &[]),
    ("taf.section.setup", &[]),
    ("taf.state.working", &[]),
    ("taf.state.waiting", &[]),
    ("taf.state.ready", &[]),
    ("taf.state.paused", &[]),
    ("taf.state.stopped", &[]),
    ("taf.state.halted", &[]),
    ("taf.halt.part_held", &[]),
    ("record.filter.held", &[]),
    ("record.filter.refused", &[]),
    ("settings.your_account", &[]),
    ("settings.mode", &[]),
    ("settings.mode_kiff", &[]),
    ("settings.mode_sample", &[]),
    ("settings.mode_signed_out", &[]),
    ("settings.kiff_account", &[]),
    ("settings.signed_in_as", &[]),
    ("settings.people_apps", &[]),
    ("people.title", &[]),
    ("settings.devices.title", &[]),
    ("settings.plan", &[]),
    ("settings.alerts", &[]),
    ("settings.theme_title", &[]),
    ("settings.theme.light", &[]),
    ("settings.theme.dark", &[]),
    ("settings.theme.system", &[]),
    ("settings.language_title", &[]),
    ("settings.change_language", &[]),
    ("settings.time_zone_title", &[]),
    ("sign_in.how_works", &[]),
    ("sign_in.every_action_taf_takes_goes", &["device"]),
    ("settings.legal", &[]),
    ("settings.about", &[]),
    ("settings.version", &[]),
    ("home.tagline", &[]),
    ("settings.account_actions", &[]),
    ("settings.sign_out", &[]),
    ("settings.clear_sample", &[]),
    ("settings.sign_out_of_kiff.title", &[]),
    ("settings.sign_out_of_kiff.body_device", &["device"]),
    ("sign_in.not_signed_out_device", &["why", "device"]),
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
    ("error.signed_out", &[]),
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

#[test]
fn account_is_the_ipads_in_its_order() {
    use tafs_core::account::{lines, Account, Row};
    use tafs_core::sidebar::Mode;
    let w = book("en");
    let sample = lines(
        &w,
        Mode::Sample,
        None,
        None,
        "https://kiff.dev/tafs/terms",
        "https://kiff.dev/tafs/privacy",
        "0.1.0",
    )
    .unwrap();
    let headers: Vec<Option<&str>> = sample
        .sections
        .iter()
        .map(|s| s.header.as_deref())
        .collect();
    assert_eq!(
        headers,
        [
            Some("Your account"),
            None,
            Some("Theme"),
            Some("Language"),
            Some("How it works"),
            Some("Legal"),
            Some("About"),
            Some("Account actions")
        ]
    );
    assert_eq!(
        sample.sections[0].rows[0],
        Row::Value("Mode".into(), "Sample data".into())
    );
    let actions: Vec<&Row> = sample.sections.last().unwrap().rows.iter().collect();
    assert!(matches!(actions[0], Row::Action(t, _) if t == "Sign in with KIFF"));
    assert!(matches!(actions[1], Row::Destructive(t, _) if t == "Clear the sample"));
    assert!(
        matches!(&sample.sections[4].rows[0], Row::Text(t) if t.ends_with("Your answers are signed on this computer."))
    );

    let acme = Account {
        tenant_id: "t1".into(),
        slug: "acme".into(),
        display_name: Some("Acme".into()),
        email: Some("rae@acme.test".into()),
        time_zone: Some("Europe/Paris".into()),
        capabilities: None,
        approval_mode: Some("view".into()),
    };
    let kiff = lines(
        &w,
        Mode::Kiff,
        Some(&acme),
        Some("Not signed out"),
        "t",
        "p",
        "0.1.0",
    )
    .unwrap();
    let headers: Vec<Option<&str>> = kiff.sections.iter().map(|s| s.header.as_deref()).collect();
    assert_eq!(
        headers,
        [
            Some("Your account"),
            Some("People & apps"),
            Some("Plan"),
            None,
            Some("Theme"),
            Some("Language"),
            None,
            Some("How it works"),
            Some("Legal"),
            Some("About"),
            Some("Account actions")
        ]
    );
    assert!(
        matches!(&kiff.sections[0].rows[1], Row::Opens { title, value: Some(v), .. } if title == "KIFF account" && v == "Acme")
    );
    assert_eq!(
        kiff.sections[0].rows[2],
        Row::Value("Signed in as".into(), "rae@acme.test".into())
    );
    assert!(
        matches!(&kiff.sections[6].rows[0], Row::Opens { value: Some(v), .. } if v == "Europe/Paris")
    );
    let last = &kiff.sections.last().unwrap().rows;
    assert!(matches!(&last[0], Row::Destructive(t, _) if t == "Sign out"));
    assert_eq!(last[1], Row::Problem("Not signed out".into()));

    // Outside English, the legal rows say the pages are in English.
    let fr = book("fr");
    let fr_lines = lines(&fr, Mode::Sample, None, None, "t", "p", "0.1.0").unwrap();
    let legal = fr_lines
        .sections
        .iter()
        .find(|s| s.rows.iter().any(|r| matches!(r, Row::Link(..))))
        .unwrap();
    assert!(matches!(&legal.rows[0], Row::Link(t, _) if t != &fr.line("settings.terms").unwrap()));
}

#[test]
fn the_sample_record_is_the_reference_sets() {
    use tafs_core::record::{days, Filter, Kind};
    let w = book("en");
    let now = Local.with_ymd_and_hms(2026, 11, 15, 2, 55, 0).unwrap();
    let record = sample::record(&w, now).unwrap();
    let d = days(&w, &record, Filter::All, None, now).unwrap();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].title, "Today · Nov 15");
    let names = |key: &str| w.line(&format!("sample.{key}.name")).unwrap();
    let lines: Vec<(String, String)> = d[0]
        .lines
        .iter()
        .map(|e| {
            (
                format!("{} · {}", names(&e.taf), e.title_with_amount(&w).unwrap()),
                e.row_detail(&w, true).unwrap(),
            )
        })
        .collect();
    let expect = [
        (
            "Chargebacks · Accept dispute on #3342",
            "€35, within the Card",
        ),
        (
            "Deploys · Held: Run migration 0042_orders_index",
            "Schema migrations: always asks you",
        ),
        ("Access · Deactivate Tom's accounts", "Within the Card"),
        ("Bill pay · Held: Pay Atlas Freight", "Over €5,000 each"),
        ("Refunds · Credit on account #2290", "€25, within the Card"),
        (
            "Chargebacks · Answer dispute on #3318",
            "€120, within the Card",
        ),
        ("Deploys · Deploy api v2.14.1", "Within the Card"),
        ("Access · Give Ana the Sales role", "Within the Card"),
        (
            "Bill pay · Pay Northwind Packaging",
            "€1,240, within the Card",
        ),
        ("Refunds · Refund order #4471", "€40, within the Card"),
        ("Refunds · Refunds started", "Card issued by You"),
        ("Bill pay · Bill pay started", "Card issued by You"),
        ("Access · Access started", "Card issued by You"),
        ("Deploys · Deploys started", "Card issued by You"),
        ("Chargebacks · Chargebacks started", "Card issued by You"),
    ];
    let got: Vec<(&str, &str)> = lines
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(got, expect);
    // Waiting keeps the held ones; Stopped none; Money moved none (the sample's lines carry no KIFF amount).
    assert_eq!(
        days(&w, &record, Filter::Waiting, None, now).unwrap()[0]
            .lines
            .len(),
        2
    );
    assert!(days(&w, &record, Filter::Stopped, None, now)
        .unwrap()
        .is_empty());
    assert!(days(&w, &record, Filter::MoneyMoved, None, now)
        .unwrap()
        .is_empty());
    assert_eq!(
        days(&w, &record, Filter::All, Some("refunds"), now).unwrap()[0]
            .lines
            .len(),
        3
    );
    assert!(record.iter().filter(|e| e.kind == Kind::Started).count() == 5);
}

#[test]
fn kiffs_activity_reads_as_the_ipad_shows_it() {
    use tafs_core::record::{read_activity, record, spent, Kind};
    let w = book("en");
    let body = br#"{"calls":[
      {"proposal_id":"p1","action_name":"stripe_create_refund","title":"Refund a payment","actor_id":"rae","outcome":"allowed","created_at":"2026-11-15T10:00:00Z","held":false,
       "tool_outcome":{"status":"done"},"amount":{"value":"150.00","unit":"EUR"}},
      {"proposal_id":"p2","action_name":"refund","actor_id":"rae","outcome":"allowed","created_at":"2026-11-15T10:01:00Z","held":true,
       "tool_outcome":{"status":"refused_by_tool","message":"card_declined"},"amount":{"value":"20","unit":"EUR"}},
      {"proposal_id":"p3","action_name":"refund","actor_id":"rae","outcome":"owner_required","created_at":"2026-11-15T10:02:00Z","held":true},
      {"proposal_id":"p4","action_name":"refund","actor_id":"rae","outcome":"denied","created_at":"2026-11-15T10:03:00Z","held":false,"reason":"over the limit"},
      {"proposal_id":"p5","action_name":"list_charges","actor_id":"someone-else","outcome":"allowed","created_at":"2026-11-15T10:04:00Z","held":false,"tool_outcome":{"status":"done"}}
    ]}"#;
    let calls = read_activity(body).expect("KIFF's activity reads");
    let entries = record(&w, &calls, &|actor| {
        (actor == "rae").then(|| "taf-rae".to_string())
    })
    .unwrap();
    assert_eq!(
        entries.len(),
        4,
        "a call by an agent the app doesn't show isn't listed"
    );
    assert_eq!(
        (entries[0].kind, entries[0].detail.as_str()),
        (Kind::Done, "Done, within its Card")
    );
    assert_eq!(
        entries[0].title_with_amount(&w).unwrap(),
        "Refund a payment · €150"
    );
    assert_eq!(entries[1].kind, Kind::NotDone);
    assert_eq!(
        entries[1].detail,
        "The app refused it. No money moved, but it still counts against today's total."
    );
    assert_eq!(
        (entries[2].kind, entries[2].detail.as_str()),
        (Kind::Held, "Waiting for you")
    );
    assert_eq!(
        (entries[3].kind, entries[3].detail.as_str()),
        (Kind::Refused, "Not sent · over the limit")
    );
    let all: Vec<&_> = entries.iter().collect();
    assert_eq!(spent(&w, &all).unwrap().as_deref(), Some("€150 spent"));
    // A held call KIFF no longer holds.
    assert_eq!(
        entries[2].row_detail(&w, false).unwrap(),
        "no longer waiting · nothing sent"
    );
}

#[test]
fn the_sample_refunds_page_is_the_reference_sets() {
    use tafs_core::taf_page::{activity, card_bar, ActivityFilter, CardMeta, TafState};
    let w = book("en");
    let now = Local.with_ymd_and_hms(2026, 11, 15, 2, 56, 0).unwrap();
    let record = sample::record(&w, now).unwrap();
    let days = activity(&w, &record, "refunds", ActivityFilter::All, now).unwrap();
    assert_eq!(days.len(), 1);
    assert_eq!(days[0].title, "Today · Nov 15");
    let lines: Vec<(String, String)> = days[0]
        .lines
        .iter()
        .map(|e| {
            (
                e.title_with_amount(&w).unwrap(),
                e.row_detail(&w, true).unwrap(),
            )
        })
        .collect();
    let got: Vec<(&str, &str)> = lines
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(
        got,
        [
            ("Credit on account #2290", "€25, within the Card"),
            ("Refund order #4471", "€40, within the Card"),
            ("Refunds started", "Card issued by You"),
        ]
    );
    let titles: Vec<String> = ActivityFilter::ALL
        .iter()
        .map(|f| f.title(&w).unwrap())
        .collect();
    assert_eq!(titles, ["All", "Done", "Held", "Refused"]);
    assert_eq!(
        activity(&w, &record, "refunds", ActivityFilter::Done, now).unwrap()[0]
            .lines
            .len(),
        2
    );
    assert!(activity(&w, &record, "refunds", ActivityFilter::Held, now)
        .unwrap()
        .is_empty());
    assert_eq!(
        activity(&w, &record, "deploys", ActivityFilter::Held, now).unwrap()[0]
            .lines
            .len(),
        1
    );
    let card = sample::card(&w, "refunds").unwrap();
    let meta = CardMeta {
        title: "Refunds",
        environment: "",
        live: true,
        active: true,
    };
    let bar = card_bar(&w, &card, &meta, 1, false).unwrap();
    assert_eq!(
        (bar.line.as_str(), bar.details.as_str()),
        ("€1,960 left today of €2,000", "Refunds")
    );
    let test_card = CardMeta {
        title: "Refunds",
        environment: "test",
        live: false,
        active: true,
    };
    assert_eq!(
        card_bar(&w, &card, &test_card, 3, false).unwrap().details,
        "Refunds · Test · 3 Cards"
    );
    assert_eq!(TafState::Working.label(&w).unwrap(), "Working");
}
