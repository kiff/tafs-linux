//! tafs cloud's answers at the pin (`shared/contracts`), read with this app's own types: a field
//! the iPad requires and an answer lacks fails here, on the pin's move, before any screen.

use std::path::PathBuf;

use chrono::{Local, TimeZone};
use serde_json::Value;
use tafs_core::home::{self, Home};
use tafs_core::overview;
use tafs_core::sidebar::Tone;
use tafs_core::words::WordBook;

fn contract(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../shared/contracts")
        .join(name);
    let file: Value =
        serde_json::from_slice(&std::fs::read(&path).expect("run tools/sync.sh first")).unwrap();
    serde_json::to_vec(&file["answer"]).unwrap()
}

fn en() -> WordBook {
    WordBook::load(&WordBook::default_root(), "en").unwrap()
}

#[test]
fn home_reads_as_the_ipad_reads_it() {
    let w = en();
    let h = Home::read(&contract("home.json")).expect("home.json reads");
    assert_eq!(h.tafs.len(), 1);
    assert_eq!(h.needs_you.as_ref().map(|n| n.count), Some(0));
    assert_eq!(h.headline_words().as_deref(), Some("0 tafs working."));
    let rae = home::taf(&w, &h.tafs[0]).unwrap();
    assert_eq!(rae.name, "Rae");
    // tafs cloud's words for its next run, where it would say Running.
    assert_eq!(rae.now, "Next run Fri 12:30");
    // tafs cloud flags what went wrong: the sidebar says it, in red, since its health is failing.
    assert_eq!(
        rae.health_now.as_deref(),
        Some("Its last 2 checks found work too big for one run, so nothing ran")
    );
    let row = home::row(&rae);
    assert_eq!(
        row.now,
        "Its last 2 checks found work too big for one run, so nothing ran"
    );
    assert_eq!(row.tone, Tone::No);
    // The Card in front: €800 left of its €1,000 today.
    let line = rae.card.as_ref().unwrap().line(&w).unwrap();
    assert_eq!(line.as_deref(), Some("€800 left today of €1,000"));
    // The Overview keeps what it's doing; its last result is when it last ran, else nothing yet.
    let o = overview::lines(&w, &[home::overview_taf(&rae)], &[], 0, "").unwrap();
    assert_eq!(o.rows[0].now, "Next run Fri 12:30");
    assert_eq!(o.rows[0].last_result, "Nothing yet");
    // It changed on 15 Oct: not relevant now on 15 Nov, so not in Your tafs.
    let now = Local.with_ymd_and_hms(2026, 11, 15, 12, 0, 0).unwrap();
    assert!(!home::relevant(&rae, false, now));
    let then = Local.with_ymd_and_hms(2026, 10, 15, 20, 0, 0).unwrap();
    assert!(home::relevant(&rae, false, then));
}

#[test]
fn a_paused_taf_reads_paused() {
    let w = en();
    let h = Home::read(&contract("home-paused.json")).expect("home-paused.json reads");
    let rae = home::taf(&w, &h.tafs[0]).unwrap();
    assert_eq!(rae.now, "Paused");
    // Its work isn't flagged and isn't idle: nothing said in place of Paused.
    assert_eq!(rae.health_now, None);
    assert_eq!(home::row(&rae).tone, Tone::Dim);
}

#[test]
fn a_field_the_ipad_requires_fails_the_read() {
    let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
    answer["tafs"][0].as_object_mut().unwrap().remove("color");
    assert!(Home::read(&serde_json::to_vec(&answer).unwrap()).is_none());
    let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
    answer["tafs"][0]["next_run"] = Value::String("Friday".into());
    assert!(Home::read(&serde_json::to_vec(&answer).unwrap()).is_none());
    // What the iPad reads leniently never fails it.
    let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
    answer["tafs"][0]["health"] = Value::String("odd".into());
    answer["tafs"][0]["work"] = Value::Bool(true);
    answer["tafs"][0]["last_change"] = Value::from(12);
    answer["headline"] = Value::from(3);
    let h = Home::read(&serde_json::to_vec(&answer).unwrap()).expect("lenient parts never fail");
    assert_eq!(h.headline_words(), None);
    assert_eq!(home::taf(&en(), &h.tafs[0]).unwrap().health_now, None);
}

#[test]
fn each_state_in_the_apps_words() {
    let w = en();
    let base: Value = serde_json::from_slice(&contract("home.json")).unwrap();
    let mut t = base["tafs"][0].clone();
    for (state, unscheduled, words, expect) in [
        ("waiting", false, Value::Null, "Waiting for you"),
        ("halted", false, Value::Null, "Halted · resume it from Halt"),
        ("working", true, Value::Null, "Works from its chat"),
        ("working", false, Value::Null, "Next run 12:30"),
        ("idle", false, Value::Null, "Acting under KIFF"),
        ("stopped", false, Value::Null, "Card not active"),
    ] {
        t["state"] = state.into();
        t["unscheduled"] = unscheduled.into();
        t["next_run_words"] = words;
        t["next_run"] = Local
            .with_ymd_and_hms(2026, 11, 15, 12, 30, 0)
            .unwrap()
            .to_rfc3339()
            .into();
        let h: home::HomeTaf = serde_json::from_value(t.clone()).unwrap();
        assert_eq!(home::taf(&w, &h).unwrap().now, expect, "{state}");
    }
    // A setup that stopped partway says tafs cloud's words in place of what it's doing.
    t["finish"] =
        serde_json::json!({ "taf": "t", "session": "s", "words": "Finish setting up Rae" });
    let h: home::HomeTaf = serde_json::from_value(t.clone()).unwrap();
    assert_eq!(home::taf(&w, &h).unwrap().now, "Finish setting up Rae");
}

#[test]
fn a_held_call_from_kiff_reads_as_the_ipad_shows_it() {
    use tafs_core::held::{held, read_held};
    use tafs_core::needs_you::{lines, Answers, Holder};
    let w = en();
    let body = br#"{"exceptions":[{"id":"exc_1","subject":"rae","action":"stripe_create_refund","entity_id":"tc_9","status":"held",
        "breaches":[{"outcome":"per_action_exceeded","reason":"over","limit":10000,"quantity":"sum(amount)","unit":"EUR","scale":2}],
        "parameters":{"amount":15000},"parameter_units":{"amount":{"unit":"EUR","scale":2}},"reason":null,"units_unavailable":false,
        "next_steps":[],"created_at":"2026-11-15T12:00:00Z","expires_at":"2026-11-15T12:10:00.000Z","accounting_version":3,
        "title":"Refund a payment","arguments":{"amount":15000,"payment_intent":"pi_123","kiff_about":"Order 1042"},
        "taf_about":"Order 1042","taf_reason":"Box arrived\ncrushed","can_approve_once":true}]}"#;
    let calls = read_held(body).expect("KIFF's held calls read");
    let call = held(&w, &calls[0], "taf-rae").unwrap();
    assert_eq!(call.summary, "Refund a payment");
    assert_eq!(call.amount.as_ref().unwrap().formatted(&w).unwrap(), "€150");
    assert_eq!(call.reason, "€150 is over the €100 limit for one call");
    assert_eq!(call.details, ["payment_intent: pi_123"]);
    assert_eq!(call.taf_reason.as_deref(), Some("Box arrived crushed"));
    let rae = [Holder {
        key: "taf-rae",
        name: "Rae",
        initial: "R",
        color: 0,
    }];
    let n = lines(
        &w,
        std::slice::from_ref(&call),
        &rae,
        true,
        call.expires_at - chrono::Duration::seconds(577),
    )
    .unwrap();
    assert_eq!(n.cards[0].remaining, "9:37 left");
    assert_eq!(n.cards[0].holder, "Rae");
    match &n.cards[0].answers {
        Answers::InKiff { page, button, .. } => {
            assert_eq!(page, "https://app.kiff.dev/needs-you/exc_1?lang=en");
            assert_eq!(button, "Answer in KIFF");
        }
        other => panic!("{other:?}"),
    }
    // Lapsed: words, not buttons that can't work.
    let late = lines(&w, std::slice::from_ref(&call), &rae, true, call.expires_at).unwrap();
    assert!(matches!(late.cards[0].answers, Answers::Expired(_)));
    assert_eq!(late.cards[0].remaining, "expired");
    // A held call missing a field the iPad requires fails the read.
    assert!(read_held(br#"{"exceptions":[{"id":"x"}]}"#).is_none());
}

#[test]
fn a_count_over_its_limit_names_which_call_this_would_be() {
    use tafs_core::held::{held, read_held};
    let w = en();
    let body = br#"{"exceptions":[{"id":"exc_2","subject":"rae","action":"send_email","entity_id":"e1","status":"held",
        "breaches":[{"outcome":"limit_reached","reason":null,"limit":20,"quantity":"count","unit":null,"scale":null,"used":20,"draw":1,"window":"calendar_day"}],
        "parameters":null,"reason":null,"parameter_units":null,"units_unavailable":null,"next_steps":null,
        "created_at":"2026-11-15T12:00:00Z","expires_at":null,"accounting_version":null}]}"#;
    let calls = read_held(body).unwrap();
    let call = held(&w, &calls[0], "taf-rae").unwrap();
    assert_eq!(call.reason, "20 calls a day: this would be the 21st");
    assert_eq!(call.summary, "send email e1");
    assert_eq!(
        call.expires_at,
        calls[0].created_at.with_timezone(&Local) + chrono::Duration::minutes(10)
    );
}

#[test]
fn a_scale_past_eighteen_is_unreadable_on_home_and_held_calls() {
    use tafs_core::held::{held, read_held};
    let w = en();
    for scale in [19, 64] {
        // Home: the Card's limit can't be read, so no figure, and nothing panics.
        let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
        answer["tafs"][0]["card"]["cards"][0]["total"]["limit"]["scale"] = scale.into();
        answer["tafs"][0]["card"]["cards"][0]["total"]["used"]["scale"] = scale.into();
        let h = Home::read(&serde_json::to_vec(&answer).unwrap()).expect("still reads");
        let rae = home::taf(&w, &h.tafs[0]).unwrap();
        let line = rae.card.as_ref().unwrap().line(&w).unwrap();
        assert!(
            line.as_deref().is_none_or(|l| !l.contains('€')),
            "{scale}: {line:?}"
        );
        // The used alone unreadable: use unavailable, never a figure from it.
        let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
        answer["tafs"][0]["card"]["cards"][0]["total"]["used"]["scale"] = scale.into();
        let h = Home::read(&serde_json::to_vec(&answer).unwrap()).unwrap();
        let line = home::taf(&w, &h.tafs[0])
            .unwrap()
            .card
            .unwrap()
            .line(&w)
            .unwrap()
            .unwrap();
        assert_eq!(line, "€1,000 a day · use unavailable");
        // A held call: no amount, and the reason doesn't use it.
        let body = format!(
            r#"{{"exceptions":[{{"id":"x","subject":"rae","action":"refund","entity_id":"e","status":"held",
            "breaches":[{{"outcome":"per_action_exceeded","reason":"over","limit":100,"quantity":"sum(amount)","unit":"EUR","scale":{scale}}}],
            "parameters":{{"amount":150}},"parameter_units":{{"amount":{{"unit":"EUR","scale":{scale}}}}},"reason":null,
            "units_unavailable":false,"next_steps":[],"created_at":"2026-11-15T12:00:00Z","expires_at":null,"accounting_version":1}}]}}"#
        );
        let calls = read_held(body.as_bytes()).unwrap();
        let call = held(&w, &calls[0], "rae").unwrap();
        assert_eq!(call.amount, None);
        assert_eq!(call.reason, "over");
    }
}

#[test]
fn a_tafs_cloud_tafs_activity_reads_as_sent() {
    use tafs_core::activity::{foot, ActivityPage, Foot};
    let page = ActivityPage::read(&contract("taf-activity.json")).expect("taf-activity.json reads");
    assert_eq!(
        page.filters
            .iter()
            .map(|f| f.title.as_str())
            .collect::<Vec<_>>(),
        ["All", "Done", "Held", "Refused"]
    );
    assert_eq!(page.groups[0].title, "Today");
    let e = &page.groups[0].entries[0];
    assert_eq!(
        (e.title.as_str(), e.detail.as_str(), e.time.as_str()),
        ("Read a case", "Done, within its Card", "12:00")
    );
    assert!(!e.call.held, "absent, read as not held");
    assert_eq!(
        foot(std::slice::from_ref(&page), None),
        Foot::End("Nothing older in this view".into())
    );
    assert_eq!(
        foot(&[], Some("It didn't reach the server.")),
        Foot::Error("It didn't reach the server.".into())
    );
    // A field tafs cloud's page requires and an answer lacks fails the read.
    let mut answer: Value = serde_json::from_slice(&contract("taf-activity.json")).unwrap();
    answer.as_object_mut().unwrap().remove("revision");
    assert!(ActivityPage::read(&serde_json::to_vec(&answer).unwrap()).is_none());
}

#[test]
fn a_tafs_runs_and_its_report_read_as_the_ipad_reads_them() {
    use tafs_core::access::{latest_session, read_report, read_runs};
    let runs = read_runs(&contract("taf-runs.json")).expect("taf-runs.json reads");
    assert_eq!(runs.len(), 2);
    assert_eq!(latest_session(&runs), Ok("sess3".into()));
    assert_eq!(
        runs[1].error.as_deref(),
        Some("The taf stopped before it finished: the app didn't answer.")
    );
    let (status, text) = read_report(&contract("taf-run-report.json")).expect("the report reads");
    assert_eq!(status, "idle");
    // The Overview's last result is the report's first line.
    let cloud = overview::CloudRun {
        report: Some(text),
        error: None,
        last_run: None,
    };
    assert_eq!(
        overview::last_result(&en(), Some(&cloud)).unwrap(),
        "Evidence went on the disputes; one waits for you."
    );
}

#[test]
fn home_with_steps_and_coachs_waits() {
    use tafs_core::needs_you::{with_waits, NeedsYouLines};
    let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
    answer["your_steps"] = serde_json::json!([{"handoff": "h1", "what": "Upload the receipt"}]);
    answer["coach_waiting"] = serde_json::json!([
        {"session": "c1", "words": {"line": "Coach waits", "open": "Open Coach"}},
        {"session": "c2", "words": {"line": "Coach asks", "open": "Open Coach"}},
        {"session": "", "words": {"line": "no chat", "open": "Open Coach"}},
        "not even an object"
    ]);
    answer["coach_waiting_clear"] = serde_json::json!({"words": {"button": "Set all aside", "title": "Set 2 aside?", "yes": "Set aside"}});
    let home = Home::read(&serde_json::to_vec(&answer).unwrap()).expect("Home reads");
    let empty = NeedsYouLines {
        title: String::new(),
        cards: vec![],
        steps: vec![],
        coach: vec![],
        set_all_aside: None,
        opens_coach: false,
        empty: Some(("Nothing is waiting.".into(), String::new())),
    };
    let lines = with_waits(empty, &home, false);
    assert_eq!(lines.steps.len(), 1);
    // Coach's waits that can't be drawn are left out, and never fail Home.
    assert_eq!(lines.coach.len(), 2);
    assert!(lines.set_all_aside.is_some());
    assert_eq!(lines.empty, None);
    // A step without its handoff fails Home, as the iPad's decoder does.
    answer["your_steps"] = serde_json::json!([{"what": "no handoff"}]);
    assert!(Home::read(&serde_json::to_vec(&answer).unwrap()).is_none());
}
