//! Every screen's lines at the pin, in every shipped language: a key the screens use that isn't
//! there, or a value a line asks for that isn't given, fails here before any screen.

use chrono::{Local, TimeZone};
use tafs_core::sample;
use tafs_core::signin::{device_name, KiffError};
use tafs_core::words::{demonstrative, placeholders, WordBook, SHIPPED};

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
    ("card.rule.never", &[]),
    ("card.rule.always_asks", &[]),
    ("card.rule.up_to_each", &["amount"]),
    ("card.rule.at_most_each", &["amount"]),
    ("card.rule.at_most_each_refused_above", &["amount"]),
    ("card.asks_you", &["formatted", "sum_label"]),
    ("card.window.in_total_window", &["window"]),
    ("card.extra_room", &["extra", "window"]),
    ("card.extra_room_until", &["extra", "ends"]),
    ("card.outside_held", &[]),
    ("card.outside_refused", &[]),
    ("card.issued_by", &[]),
    ("card.ends", &[]),
    ("card.until", &[]),
    ("card.ended", &[]),
    ("card.revoked", &[]),
    ("card.no_end_date", &[]),
    ("card.no_actions_short", &[]),
    ("sample.card.refunds_stripe", &[]),
    ("sample.card.read_stripe", &[]),
    ("sample.refunds_stripe_test", &[]),
    ("sample.list_charges", &[]),
    ("sample.look_up_charge", &[]),
    ("taf.revoke", &[]),
    ("people.not_shared", &[]),
    ("report.runs_not_loaded", &["why"]),
    ("settings.find_time_zone", &[]),
    ("settings.account_id", &[]),
    ("settings.time_zone.uses_this", &[]),
    ("export.row", &[]),
    ("export.own_chats_each_taf_coach", &[]),
    ("settings.delete_account.row", &[]),
    ("sign_in.deletes_every_taf_runs_chats", &[]),
    ("settings.use_device_zone", &["device", "identifier"]),
    ("people.add_group", &[]),
    ("people.new_group", &[]),
    ("people.everyone_group", &[]),
    ("people.manager", &[]),
    ("people.member", &[]),
    ("people.uses_none", &[]),
    ("people.everyone_uses_none", &[]),
    ("people.no_groups", &[]),
    ("people.uses", &["tafs"]),
    ("people.add_someone_to", &["name"]),
    ("people.group_actions", &["group"]),
    ("people.not_read", &["why"]),
    ("calendar.today", &[]),
    ("calendar.reading", &[]),
    ("calendar.agenda", &[]),
    ("calendar.view_picker", &[]),
    ("calendar.choose_date", &[]),
    ("calendar.mode.day", &[]),
    ("calendar.mode.week", &[]),
    ("calendar.mode.month", &[]),
    ("calendar.previous_day", &[]),
    ("calendar.next_day", &[]),
    ("calendar.previous_week", &[]),
    ("calendar.next_week", &[]),
    ("calendar.previous_month", &[]),
    ("calendar.next_month", &[]),
    ("calendar.nothing_on_day", &[]),
    ("calendar.nothing_this_week", &[]),
    ("calendar.nothing_these_days", &[]),
    ("calendar.weekday.mon", &[]),
    ("calendar.weekday.tue", &[]),
    ("calendar.weekday.wed", &[]),
    ("calendar.weekday.thu", &[]),
    ("calendar.weekday.fri", &[]),
    ("calendar.weekday.sat", &[]),
    ("calendar.weekday.sun", &[]),
    ("calendar.month_name.january", &[]),
    ("calendar.month_name.february", &[]),
    ("calendar.month_name.march", &[]),
    ("calendar.month_name.april", &[]),
    ("calendar.month_name.may", &[]),
    ("calendar.month_name.june", &[]),
    ("calendar.month_name.july", &[]),
    ("calendar.month_name.august", &[]),
    ("calendar.month_name.september", &[]),
    ("calendar.month_name.october", &[]),
    ("calendar.month_name.november", &[]),
    ("calendar.month_name.december", &[]),
    ("calendar.month_title.january", &[]),
    ("calendar.month_title.february", &[]),
    ("calendar.month_title.march", &[]),
    ("calendar.month_title.april", &[]),
    ("calendar.month_title.may", &[]),
    ("calendar.month_title.june", &[]),
    ("calendar.month_title.july", &[]),
    ("calendar.month_title.august", &[]),
    ("calendar.month_title.september", &[]),
    ("calendar.month_title.october", &[]),
    ("calendar.month_title.november", &[]),
    ("calendar.month_title.december", &[]),
    ("calendar.slot_at", &["name", "first"]),
    ("home.calendar_not_read", &["why"]),
    ("settings.devices.unnamed", &[]),
    ("settings.devices.this_device_title", &[]),
    ("settings.devices.sign_out", &[]),
    ("settings.devices.not_used", &[]),
    ("settings.devices.note_device", &["device"]),
    ("settings.devices.last_used", &["ago"]),
    ("settings.devices.not_loaded", &["why"]),
    ("settings.devices.this_device_name", &["device"]),
    ("connect.apps.search", &[]),
    ("connect.apps.yours", &[]),
    ("connect.apps.discover", &[]),
    ("connect.apps.column_app", &[]),
    ("connect.apps.column_how", &[]),
    ("connect.apps.column_whose", &[]),
    ("connect.apps.column_status", &[]),
    ("connect.apps.connect", &[]),
    ("connect.apps.not_listed", &[]),
    ("connect.apps.used_by", &["tafs"]),
    ("connect.apps_connect_taf_make_one", &[]),
    ("connect.apps_not_read", &["why"]),
    ("connect.more_apps", &[]),
    ("connect.more_apps_could_not_listed", &["why"]),
    ("connect.how.key", &[]),
    ("connect.how.sign_in", &[]),
    ("connect.status.connected", &[]),
    ("connect.sign_in_again", &[]),
    ("common.loading", &[]),
    ("billing.by_apple", &[]),
    ("billing.by_google_play", &[]),
    ("billing.by_kiff", &[]),
    ("billing.by_agreement", &[]),
    ("billing.ask.asked_raise", &[]),
    ("billing.ask.asked_pack", &[]),
    ("billing.ask.raised", &[]),
    ("billing.ask.added_pack", &[]),
    ("billing.ask.declined", &[]),
    ("billing.ask.declined_note", &["note"]),
    ("billing.ask.expired", &[]),
    ("billing.ask.someone_raise", &[]),
    ("billing.ask.someone_pack", &["pack"]),
    ("billing.plan_could_not_read_kiff", &["why"]),
    ("usage.admin_answers", &["detail"]),
    ("usage.company_billed_kiff_solo_bought", &["time"]),
    ("usage.pack_credits", &[]),
    ("usage.raise_limit_button", &[]),
    ("usage.requests.title", &[]),
    ("usage.requests.waiting", &[]),
    ("usage.requests.withdraw", &[]),
    ("usage.requests.ask_admin", &[]),
    ("usage.requests.decline", &[]),
    ("usage.credits", &[]),
    ("usage.last_period", &[]),
    ("usage.this_period", &[]),
    ("usage.last_period_credits_could_not", &[]),
    ("usage.more_runs", &[]),
    ("usage.runs_could_not_read_pull", &[]),
    ("usage.the_balance", &[]),
    ("work.its_runs", &[]),
    ("sample.credits.balance_detail", &[]),
    ("sample.credits.balance_words", &[]),
    ("sample.credits.invoices_last", &[]),
    ("sample.credits.invoices_this", &[]),
    ("sample.credits.last_period", &[]),
    ("sample.credits.not_counted", &[]),
    ("sample.credits.not_counted_coach", &[]),
    ("sample.credits.not_counted_failed", &[]),
    ("sample.credits.not_counted_trials", &[]),
    ("sample.credits.note", &[]),
    ("sample.credits.plan_line", &[]),
    ("sample.credits.refunds_last", &[]),
    ("sample.credits.refunds_this", &[]),
    ("sample.credits.this_period", &[]),
    ("common.could_not_read_why", &["what", "why"]),
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

/// The pinned `generate.py`'s `THIS` table, as `(language, starts, before, other)`; none when the pin
/// doesn't carry it. Its one line reads `THIS = {"fr": ("…", "cet ", "ce ")}`: every quoted string, in
/// fours.
fn pinned_this_rules() -> Option<Vec<(String, String, String, String)>> {
    let source = std::fs::read_to_string(WordBook::default_root().join("generate.py")).ok()?;
    let line = source.lines().find(|l| l.starts_with("THIS = {"))?;
    let quoted: Vec<String> = line
        .split('"')
        .skip(1)
        .step_by(2)
        .map(str::to_string)
        .collect();
    assert!(
        !quoted.is_empty() && quoted.len().is_multiple_of(4),
        "the pinned THIS line doesn't read as rules: {line}"
    );
    Some(
        quoted
            .chunks(4)
            .map(|r| (r[0].clone(), r[1].clone(), r[2].clone(), r[3].clone()))
            .collect(),
    )
}

#[test]
fn the_this_rule_is_the_pinned_generators() {
    // `demonstrative` copies the shared generate.py's THIS table: once the pin carries it, each
    // language's letters and words must be this app's, letter by letter, so the copy can't drift.
    // Before the pin carries it, no line at the pin may use the form.
    let Some(rules) = pinned_this_rules() else {
        for lang in SHIPPED {
            let w = book(lang);
            for key in w.keys() {
                let raw = w.raw(key).map(|v| v.to_string()).unwrap_or_default();
                assert!(
                    !raw.contains("|this}") && !raw.contains("|This}"),
                    "{lang} {key} uses |this before the pin has its rule"
                );
            }
        }
        return;
    };
    for (lang, starts, before, other) in rules {
        for letter in "abcdefghijklmnopqrstuvwxyzàâäçéèêëîïôöùûüœæ"
            .chars()
            .chain(starts.chars())
        {
            let name = format!("{letter}x");
            let word = if starts.contains(letter) {
                &before
            } else {
                &other
            };
            assert_eq!(
                demonstrative(&lang, &name, false),
                format!("{word}{name}"),
                "{lang} {name}"
            );
        }
    }
}

#[test]
fn every_line_at_the_pin_fills_with_no_hole_left() {
    // Every line in every language, each value given: a form this app doesn't know
    // (`{device|that}`) fails here, never on screen.
    for lang in SHIPPED {
        let w = book(lang);
        let keys: Vec<String> = w.keys().map(str::to_string).collect();
        for key in &keys {
            let texts: Vec<String> = match w.raw(key) {
                Some(serde_json::Value::Object(forms)) => forms
                    .values()
                    .filter_map(|v| v.as_str().map(str::to_string))
                    .collect(),
                Some(v) => vec![v.as_str().unwrap_or_default().to_string()],
                None => vec![],
            };
            let mut names: Vec<String> = texts.iter().flat_map(|t| placeholders(t)).collect();
            names.sort();
            names.dedup();
            let given: Vec<(&str, &str)> = names
                .iter()
                .filter(|n| n.as_str() != "count")
                .map(|n| (n.as_str(), "Mac"))
                .collect();
            let filled: Vec<String> = if w.raw(key).is_some_and(|v| v.is_object()) {
                (0..=25)
                    .filter_map(|n| match w.count(key, n, &given) {
                        Ok(text) => Some(text),
                        // A language needn't carry every form for every number.
                        Err(tafs_core::words::WordsError::MissingForm { .. }) => None,
                        Err(e) => panic!("{lang} {key} ({n}): {e:?}"),
                    })
                    .collect()
            } else {
                vec![w
                    .get(key, &given)
                    .unwrap_or_else(|e| panic!("{lang} {key}: {e:?}"))]
            };
            for text in filled {
                assert!(!text.contains('{'), "{lang} {key} left a hole: {text}");
            }
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
    // Coach's suggestions and the consent are read when Account opens: placed, not yet drawn.
    let slots: Vec<Option<tafs_core::account::Slot>> =
        kiff.sections.iter().map(|s| s.slot).collect();
    assert_eq!(slots[3], Some(tafs_core::account::Slot::Suggestions));
    assert_eq!(slots[7], Some(tafs_core::account::Slot::Consent));
    assert_eq!(slots.iter().flatten().count(), 2);
    assert_eq!(
        headers,
        [
            Some("Your account"),
            Some("People & apps"),
            Some("Plan"),
            Some("Coach"),
            None,
            Some("Theme"),
            Some("Language"),
            None,
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
        matches!(&kiff.sections[8].rows[0], Row::Opens { value: Some(v), .. } if v == "Europe/Paris")
    );
    let last = &kiff.sections.last().unwrap().rows;
    assert!(matches!(&last[0], Row::Destructive(t, _) if t == "Sign out"));
    assert_eq!(last[1], Row::Problem("Not signed out".into()));

    // In a language kiff.dev has the pages in, the legal rows are plain.
    let fr = book("fr");
    let fr_lines = lines(&fr, Mode::Sample, None, None, "t", "p", "0.1.0").unwrap();
    let legal = fr_lines
        .sections
        .iter()
        .find(|s| s.rows.iter().any(|r| matches!(r, Row::Link(..))))
        .unwrap();
    assert!(matches!(&legal.rows[0], Row::Link(t, _) if t == &fr.line("settings.terms").unwrap()));
    assert!(
        matches!(&legal.rows[1], Row::Link(t, _) if t == &fr.line("settings.privacy_policy").unwrap())
    );
}

#[test]
fn the_legal_pages_open_in_the_apps_language() {
    use tafs_core::account::{legal_label, privacy_url, terms_url, LEGAL_LANGUAGES};
    for lang in ["de", "es", "fr", "it", "nl", "pl", "pt"] {
        assert!(LEGAL_LANGUAGES.contains(&lang));
        assert_eq!(
            terms_url(lang),
            format!("https://kiff.dev/{lang}/tafs/terms")
        );
        assert_eq!(
            privacy_url(lang),
            format!("https://kiff.dev/{lang}/tafs/privacy")
        );
        let w = book(lang);
        assert_eq!(
            legal_label(&w, "settings.terms").unwrap(),
            w.line("settings.terms").unwrap()
        );
    }
    // English, and any language kiff.dev lacks, open the English pages.
    for lang in ["en", "ja", ""] {
        assert_eq!(terms_url(lang), "https://kiff.dev/tafs/terms");
        assert_eq!(privacy_url(lang), "https://kiff.dev/tafs/privacy");
    }
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

#[test]
fn refunds_card_tab_is_the_reference_sets() {
    use tafs_core::cards::stack;
    let w = book("en");
    let cards = sample::full_cards(&w, "refunds", true).unwrap();
    let titles: Vec<&str> = cards.iter().map(|c| c.title.as_str()).collect();
    assert_eq!(
        titles,
        ["Refunds · Stripe", "Read Stripe", "Refunds · Stripe test"]
    );
    // The furthest back on top: the test Card, then Read Stripe, then Refunds · Stripe in front.
    let order: Vec<&str> = stack(&cards, None)
        .iter()
        .map(|(i, _)| cards[*i].title.as_str())
        .collect();
    assert_eq!(
        order,
        ["Refunds · Stripe test", "Read Stripe", "Refunds · Stripe"]
    );
    assert_eq!(cards[2].environment_label().as_deref(), Some("TEST"));
    let front = &cards[0];
    assert_eq!(
        front.totals.hero(&w).unwrap(),
        Some(("€1,520".into(), "left today of €2,000".into()))
    );
    assert_eq!(
        front.facts(&w).unwrap(),
        ["up to €150 each; more asks you first", "2 tools"]
    );
    assert_eq!(
        front.ends(&w, false).unwrap(),
        ("Ends".into(), "No end date".into())
    );
    let lines: Vec<(String, String, String)> = front
        .rules
        .iter()
        .map(|r| {
            (
                r.label.clone(),
                r.outside(&w).unwrap(),
                r.summary(&w).unwrap(),
            )
        })
        .collect();
    assert_eq!(
        lines,
        [
            (
                "Refunds".into(),
                "Outside: held for you".into(),
                "up to €150 each; more asks you first · €2,000 in total a day, then it asks you"
                    .into()
            ),
            (
                "Credits".into(),
                "Outside: held for you".into(),
                "up to €100 each; more asks you first · €1,000 in total a day, then it asks you"
                    .into()
            ),
        ]
    );
    assert_eq!(
        cards[1].totals.hero(&w).unwrap(),
        Some(("180".into(), "calls left today of 200".into()))
    );
    // The app's own sample: Refunds' one Card, its payouts never.
    let own = sample::full_cards(&w, "refunds", false).unwrap();
    assert_eq!(own.len(), 1);
    assert_eq!(own[0].rules[2].summary(&w).unwrap(), "never");
    assert_eq!(own[0].rules[2].outside(&w).unwrap(), "Outside: refused");
    let deploys = sample::full_cards(&w, "deploys", false).unwrap();
    let summaries: Vec<String> = deploys[0]
        .rules
        .iter()
        .map(|r| r.summary(&w).unwrap())
        .collect();
    assert_eq!(
        summaries,
        [
            "10 calls a day, then it asks you",
            "20 calls a day, then it asks you",
            "always asks you",
            "never"
        ]
    );
}

#[test]
fn plan_and_usage_is_the_reference_sets() {
    use tafs_core::usage::{sample_balance, sample_credits};
    let w = book("en");
    let b = sample_balance(&w).unwrap().balance;
    assert_eq!(b.words, "Solo · 260 of 500 credits left this period");
    assert_eq!(b.meter_pct, Some(48));
    assert_eq!(
        b.detail.as_deref(),
        Some("Then 900 in packs, which never expire")
    );
    assert!(b.credits_panel && !b.waiting() && b.raise.is_none() && b.buy.is_none());
    let c = sample_credits(&w, false).unwrap();
    assert_eq!(c.words, "This period: 1,240 credits used · ends on 30 Nov");
    let tafs: Vec<&str> = c.tafs.iter().map(|t| t.words.as_str()).collect();
    assert_eq!(
        tafs,
        [
            "Refunds · 820 credits · 41 runs",
            "Overdue invoices · 420 credits · 28 runs"
        ]
    );
    let not = c.shows_not_counted().unwrap();
    assert_eq!(not.words, "Not counted: 9 runs");
    let lines: Vec<&str> = not.lines.iter().map(|l| l.words.as_str()).collect();
    assert_eq!(
        lines,
        [
            "Coach, within its fair use · 6 runs",
            "Trials of a new version · 2 runs",
            "Failed on our side · 1 run"
        ]
    );
    assert_eq!(
        c.note.as_deref(),
        Some("Each run counts once, when it ends, in whole credits.")
    );
    let last = sample_credits(&w, true).unwrap();
    assert_eq!(
        last.words,
        "Last period: 2,310 credits used · 1 Oct to 31 Oct"
    );
    assert!(last.shows_not_counted().is_none());
    // Every language has the sample's lines.
    for lang in SHIPPED {
        let w = book(lang);
        sample_balance(&w).unwrap();
        sample_credits(&w, false).unwrap();
        sample_credits(&w, true).unwrap();
    }
}

#[test]
fn kiffs_usage_lines_at_the_pin() {
    use tafs_core::usage::{
        asked_line, billing_line, last_answer, request_line, solo_still_renews, KiffUsage,
    };
    for lang in SHIPPED {
        let w = book(lang);
        let u = KiffUsage::read(
            br#"{"plan":"solo","billing":{"source":"stripe","apple_solo_until":"2026-12-03T10:00:00Z"},"requests":[{"id":"r","asked_by":"u","kind":"pack","pack":"rayo","status":"declined","note":"Not now","created_at":"2026-11-01T00:00:00Z","expires_at":"2026-11-30T00:00:00Z"}]}"#,
        )
        .unwrap();
        let r = &u.requests.as_ref().unwrap()[0];
        assert!(billing_line(&w, &u).unwrap().is_some());
        assert!(solo_still_renews(&w, &u).unwrap().unwrap().contains("2026"));
        assert!(last_answer(&w, &u, false)
            .unwrap()
            .unwrap()
            .contains("Not now"));
        assert_eq!(last_answer(&w, &u, true).unwrap(), None);
        asked_line(&w, r).unwrap();
        // A pack that isn't on sale reads as its id, capitalised.
        assert!(request_line(&w, r, &u).unwrap().contains("Rayo"));
    }
    let w = book("en");
    let u = KiffUsage::read(br#"{"billing":{"source":"apple"}}"#).unwrap();
    assert_eq!(
        billing_line(&w, &u).unwrap().as_deref(),
        Some("Billed by Apple")
    );
    assert_eq!(solo_still_renews(&w, &u).unwrap(), None);
    let u = KiffUsage::read(br#"{"billing":{"source":""}}"#).unwrap();
    assert_eq!(billing_line(&w, &u).unwrap(), None);
}

#[test]
fn connect_apps_is_the_reference_sets() {
    use tafs_core::connect::{split, ConnectApps, Row};
    let file = WordBook::default_root().join("../../ios/Tafs/Samples/connect-apps.json");
    let apps = ConnectApps::read(&std::fs::read(file).expect("run tools/sync.sh first")).unwrap();
    let (yours, discover) = split(&apps.apps);
    let names = |a: &[&tafs_core::connect::App]| -> Vec<String> {
        a.iter().map(|a| a.display_name().to_string()).collect()
    };
    assert_eq!(names(&yours), ["Gmail", "Stripe"]);
    assert_eq!(
        names(&discover),
        ["Google Sheets", "Slack", "Qonto", "HubSpot"]
    );
    assert!(apps.by_key);
    let w = book("en");
    let gmail = Row::new(&w, yours[0], &[]).unwrap();
    assert_eq!(
        (
            gmail.how.as_str(),
            gmail.whose.as_deref(),
            gmail.status.as_deref()
        ),
        ("Sign in", Some("Yours"), Some("Connected"))
    );
    // The sample's `used_by` is plain names, which tafs cloud's form isn't: no "Used by", as on
    // the iPad's reference picture.
    assert_eq!(gmail.used_by, None);
    assert_eq!(Row::new(&w, discover[2], &[]).unwrap().how, "With a key");
    // A taf's names, listed as each language lists them.
    use tafs_core::words::list;
    let names: Vec<String> = ["Refunds", "Chargebacks", "Deploys"]
        .map(String::from)
        .to_vec();
    assert_eq!(list("en", &names[..2]), "Refunds and Chargebacks");
    assert_eq!(list("en", &names), "Refunds, Chargebacks, and Deploys");
    assert_eq!(list("fr", &names), "Refunds, Chargebacks et Deploys");
    assert_eq!(list("de", &names[..1]), "Refunds");
    let used = Row::new(
        &w,
        &tafs_core::connect::ConnectApps::read(
            br#"{"apps":[{"entry":"a","state":"connected","used_by":[{"taf":"t1","name":"Refunds"},{"taf":"t2","name":"Chargebacks"}]}]}"#,
        )
        .unwrap()
        .apps[0],
        &[],
    )
    .unwrap();
    assert_eq!(
        used.used_by.as_deref(),
        Some("Used by Refunds and Chargebacks")
    );
}

#[test]
fn devices_is_the_reference_sets() {
    use tafs_core::devices::{last_used, sample};
    let now = chrono::Utc::now();
    for lang in SHIPPED {
        let w = book(lang);
        let devices = sample(&w, now).unwrap();
        assert!(devices[0].this_device && !devices[1].this_device && !devices[2].this_device);
        for d in &devices {
            last_used(&w, d.last_used, now).unwrap();
        }
    }
    let w = book("en");
    let devices = sample(&w, now).unwrap();
    let lines: Vec<String> = devices
        .iter()
        .map(|d| last_used(&w, d.last_used, now).unwrap())
        .collect();
    assert_eq!(
        lines,
        [
            "Last used in 0 seconds",
            "Last used 3 hours ago",
            "Last used 1 day ago"
        ]
    );
    assert_eq!(devices[1].name.as_deref(), Some("tafs on iPhone"));
    assert_eq!(last_used(&w, None, now).unwrap(), "Not used yet");
}

#[test]
fn calendar_titles_are_the_reference_sets() {
    use tafs_core::calendar::{day_heading, title, Mode, State};
    let w = book("en");
    let s = |mode, selected: &str| State {
        mode,
        selected: selected.into(),
    };
    assert_eq!(
        title(&w, &s(Mode::Day, "2026-11-15")).unwrap(),
        "Sun 15 November 2026"
    );
    assert_eq!(
        title(&w, &s(Mode::Week, "2026-11-15")).unwrap(),
        "9 – 15 November 2026"
    );
    assert_eq!(
        title(&w, &s(Mode::Week, "2026-12-01")).unwrap(),
        "30 November – 6 December 2026"
    );
    assert_eq!(
        title(&w, &s(Mode::Week, "2026-12-30")).unwrap(),
        "28 December 2026 – 3 January 2027"
    );
    assert_eq!(
        title(&w, &s(Mode::Month, "2026-11-15")).unwrap(),
        "November 2026"
    );
    assert_eq!(day_heading(&w, "2026-11-05").unwrap(), "Thu 5 November");
    // Every title, heading and count in every language.
    for lang in SHIPPED {
        let w = book(lang);
        for mode in Mode::ALL {
            for day in ["2026-11-15", "2026-12-01", "2026-12-30"] {
                title(&w, &s(mode, day)).unwrap();
            }
            mode.title(&w).unwrap();
        }
        day_heading(&w, "2026-11-05").unwrap();
        w.count("calendar.day_scheduled", 3, &[("day", "x")])
            .unwrap();
        w.count(
            "calendar.slot_times",
            2,
            &[("name", "x"), ("first", "09:00")],
        )
        .unwrap();
        w.count("calendar.times", 2, &[]).unwrap();
    }
}

#[test]
fn people_lines_at_the_pin() {
    use tafs_core::account::Account;
    use tafs_core::people::{account_people, uses, Group};
    let account = |name: &str, email: &str| Account {
        tenant_id: "t".into(),
        slug: "acme".into(),
        display_name: Some(name.into()),
        email: Some(email.into()),
        time_zone: None,
        capabilities: None,
        approval_mode: None,
    };
    let group = |everyone| Group {
        id: "g".into(),
        name: "Finance".into(),
        everyone,
        members: vec![],
        agents: vec![],
    };
    let w = book("en");
    assert_eq!(
        account_people(&w, &account("Acme", "rae@acme.test"), 3).unwrap(),
        "Acme · 3 people"
    );
    // A Solo account, named after its email: the count alone.
    assert_eq!(
        account_people(&w, &account("rae@acme.test", "rae@acme.test"), 1).unwrap(),
        "1 person"
    );
    let names: Vec<String> = ["Refunds", "Invoices"].map(String::from).to_vec();
    assert_eq!(
        uses(&w, &group(false), &names).unwrap(),
        "Uses Invoices and Refunds"
    );
    assert_eq!(uses(&w, &group(false), &[]).unwrap(), "Uses no taf yet");
    for lang in SHIPPED {
        let w = book(lang);
        account_people(&w, &account("Acme", "rae@acme.test"), 2).unwrap();
        uses(&w, &group(true), &[]).unwrap();
        group(true).shown_name(&w).unwrap();
    }
}

#[test]
fn kiff_account_lines_at_the_pin() {
    use tafs_core::account::{kiff_account_lines, Account};
    let account = Account {
        tenant_id: "t".into(),
        slug: "acme".into(),
        display_name: Some("Acme".into()),
        email: Some("rae@acme.test".into()),
        time_zone: Some("Europe/Paris".into()),
        capabilities: None,
        approval_mode: None,
    };
    let w = book("en");
    let lines = kiff_account_lines(&w, &account, "America/New_York").unwrap();
    let rows: Vec<(&str, &str)> = lines
        .rows
        .iter()
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(
        rows,
        [
            ("KIFF account", "Acme"),
            ("Account id", "acme"),
            ("Signed in as", "rae@acme.test")
        ]
    );
    assert_eq!(lines.zone, "Europe/Paris");
    assert_eq!(
        lines.use_this_zone.as_deref(),
        Some("Use this computer's: America/New_York")
    );
    // The same zone here: no line to use it. No zone from KIFF: UTC.
    assert_eq!(
        kiff_account_lines(&w, &account, "Europe/Paris")
            .unwrap()
            .use_this_zone,
        None
    );
    let utc = Account {
        time_zone: None,
        ..account.clone()
    };
    assert_eq!(kiff_account_lines(&w, &utc, "UTC").unwrap().zone, "UTC");
    for lang in SHIPPED {
        kiff_account_lines(&book(lang), &account, "Asia/Tokyo").unwrap();
    }
    assert!(!tafs_core::account_time::local_identifier().is_empty());
}

#[test]
fn who_uses_a_taf_at_the_pin() {
    use tafs_core::access::{used_by, Share};
    use tafs_core::people::{Group, Member, Person};
    let group = Group {
        id: "g".into(),
        name: "Support".into(),
        everyone: false,
        members: vec![
            Member {
                subject: "a".into(),
                role: "member".into(),
            },
            Member {
                subject: "b".into(),
                role: "manager".into(),
            },
        ],
        agents: vec![],
    };
    let person = Person {
        subject: "c".into(),
        role: "member".into(),
        status: "active".into(),
        email: Some("lee@example.test".into()),
        you: false,
    };
    let shares = [
        Share {
            group: Some("g".into()),
            subject: None,
        },
        Share {
            group: None,
            subject: Some("c".into()),
        },
        Share {
            group: None,
            subject: Some("a".into()),
        },
    ];
    let w = book("en");
    // People counted once, however they're shared with.
    assert_eq!(
        used_by(
            &w,
            &shares,
            std::slice::from_ref(&group),
            std::slice::from_ref(&person)
        )
        .unwrap(),
        "Support, lee@example.test, a · 3 people"
    );
    assert_eq!(used_by(&w, &[], &[], &[]).unwrap(), "Not shared");
    for lang in SHIPPED {
        used_by(&book(lang), &shares, std::slice::from_ref(&group), &[]).unwrap();
    }
}

#[test]
fn coachs_thread_reads_in_every_language() {
    use serde_json::{json, Value};
    use tafs_core::coach::SetupThread;
    use tafs_core::coach_lines::{lines, Line, Listed};
    let root = env!("CARGO_MANIFEST_DIR");
    let read = |path: &str| -> Value {
        let file: Value =
            serde_json::from_slice(&std::fs::read(format!("{root}/{path}")).unwrap()).unwrap();
        file["answer"].clone()
    };
    let mut every = read("../../shared/contracts/coach-conversation.json");
    // Every kind of card, waiting, so each one's words are read.
    let card = |c: Value| json!({"at": "2026-10-16T12:02:00Z", "session": "sess1", "run": "run3", "component": c});
    let more = [
        json!({"type": "steps", "id": "plan", "steps": [{"label": "Connect Stripe", "status": "done"}, {"label": "Draft its Card", "status": "active"}]}),
        json!({"type": "notice", "id": "n", "title": "Heads up", "text": "Stripe is in test mode.", "tone": "warning"}),
        json!({"type": "confirm", "id": "c", "text": "Test mode?", "options": [{"value": "y", "label": "Yes"}]}),
        json!({"type": "sign_in", "id": "s", "service": "Google Sheets", "waits": "owner", "wider": {"words": "wider"}}),
        json!({"type": "approval", "id": "a", "grant": "grt_9", "title": "Connect Google Sheets", "waits": "owner", "signs": {"card": false, "words": "Its Card: you approve it next"}, "wider": {"words": "wider"}}),
        json!({"type": "result", "id": "v", "text": "A new version.", "taf": {"agent": "rae", "name": "Rae", "improves": "taf-aaaa0000000000000000", "note": "Faster."}}),
        json!({"type": "drive", "id": "d", "text": "Open Connect apps"}),
        json!({"type": "knob", "id": "k", "title": "Rae's dial"}),
    ];
    for c in more {
        every["components"].as_array_mut().unwrap().push(card(c));
    }
    every["steering"] = json!({"run": "run3", "status": "RUNNING", "untaken": [{"text": "And chargebacks", "at": "2026-10-16T12:03:00Z"}], "held": {"text": "Then Shippo", "at": "2026-10-16T12:03:30Z"}});
    every["coach_lines"]
        .as_array_mut()
        .unwrap()
        .push(json!({"at": "2026-10-16T12:04:00Z", "taf": "Rae", "said": "Done.", "away": true}));
    let threads = [
        every,
        read("../../shared/contracts/coach-wider.json"),
        json!({"messages": []}),
    ];
    let listed = [Listed {
        agent: "refunds-x".into(),
        key: "taf-1".into(),
        name: "Refunds".into(),
    }];
    let now = chrono::Utc::now();
    for lang in SHIPPED {
        let w = book(lang);
        for t in &threads {
            let thread = SetupThread::read(&serde_json::to_vec(t).unwrap()).unwrap();
            let l = lines(&w, &thread, &listed, tafs_core::sidebar::Mode::Kiff, now)
                .unwrap_or_else(|e| panic!("{lang}: {e}"));
            assert!(l.starts.is_some() || !l.lines.is_empty(), "{lang}");
            // A hole is a brace before a name; the lines' own braces have a space after them.
            for line in &l.lines {
                let shown = format!("{line:?}");
                let hole = shown
                    .match_indices('{')
                    .any(|(i, _)| shown[i + 1..].starts_with(|c: char| c.is_ascii_alphabetic()));
                assert!(!hole, "{lang}: {shown}");
            }
            assert!(l
                .lines
                .iter()
                .all(|x| !matches!(x, Line::Gap(g) if g.is_empty())));
        }
    }
}

#[test]
fn what_is_sent_to_coach_reads_in_every_language() {
    use chrono::{DateTime, Utc};
    use tafs_core::coach::SetupThread;
    use tafs_core::coach_lines::{answered_of, composer, ended, unsent};
    use tafs_core::coach_send::{Conversation, Ended, Refusal, Submit};
    let at = DateTime::parse_from_rfc3339("2026-10-16T12:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let mut c = Conversation::default();
    c.setup = Some("taf-c".into());
    c.chat = Some("sess1".into());
    c.took(SetupThread::default(), at);
    let Submit::Send(d) = c.submit("Hi", at) else {
        panic!("not sent")
    };
    c.not_sent(&d, "offline", Some("account".into()), at);
    let Submit::Send(d) = c.submit("[questions limits] {}", at) else {
        panic!("not sent")
    };
    c.not_sent(&d, "offline", None, at);
    for lang in SHIPPED {
        let w = book(lang);
        for thinking in [false, true] {
            let l = composer(&w, thinking).unwrap_or_else(|e| panic!("{lang}: {e}"));
            assert!(!l.placeholder.is_empty() && !l.send.is_empty() && !l.stop.is_empty());
        }
        let u = unsent(&w, &c.thread, &c.unsent).unwrap_or_else(|e| panic!("{lang}: {e}"));
        assert!(u[0].resume.is_some() && u[0].edit.is_none(), "{lang}");
        assert!(u[1].edit.is_some() && u[1].resume.is_none(), "{lang}");
        assert!(ended(&w, Some(Ended::Stopped)).unwrap().is_some());
        assert!(!answered_of(&w, 1, 2).unwrap().contains('{'), "{lang}");
        Refusal::read(409, b"{}").words(&w).unwrap();
        for key in [
            "coach.answer_did_not_reach_setup",
            "coach.conversation_not_loaded",
            "work.did_not_stop",
        ] {
            assert!(w.get(key, &[("why", "x")]).is_ok(), "{lang}: {key}");
        }
    }
}
