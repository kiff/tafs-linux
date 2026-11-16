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
fn tafs_being_set_up_read_as_the_ipad_reads_them() {
    let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
    // The contract sends `setting_up`, so `others` isn't drawn.
    assert!(Home::read(&serde_json::to_vec(&answer).unwrap())
        .unwrap()
        .other_agents()
        .is_empty());
    answer["setting_up"] = serde_json::json!([
        {"name": "Invoice chaser", "kiff_agent": "ic", "color": 3,
         "finish": {"taf": "coach", "session": "s1", "words": " Finish it "},
         "discard": {"words": {"button": "Discard", "yes": "Discard", "no": 4}, "cards": ["c1"]}},
        {"name": "Half", "finish": {"taf": "coach", "words": "No session"},
         "discard": {"words": {"button": "Discard"}}},
        {"kiff_agent": "nameless"},
        "not an item",
        {"name": 7}
    ]);
    let h = Home::read(&serde_json::to_vec(&answer).unwrap()).expect("never fails Home");
    let s = h.setting_up();
    assert_eq!(
        s.iter().map(|s| s.name.as_str()).collect::<Vec<_>>(),
        ["Invoice chaser", "Half"]
    );
    assert_eq!(s[0].finish.as_deref(), Some("Finish it"));
    assert_eq!(
        s[0].discard
            .as_ref()
            .map(|d| (d.button.as_str(), d.no.as_str())),
        Some(("Discard", ""))
    );
    assert_eq!(s[0].row().key, "agent:ic");
    assert_eq!((s[1].finish.clone(), s[1].discard.clone()), (None, None));
    assert_eq!(s[1].row().key, "setting-up:Half");
    assert_eq!(s[1].row().now, "");
    // Without `setting_up`, every other agent is drawn.
    answer.as_object_mut().unwrap().remove("setting_up");
    answer["others"] = answer["tafs"].clone();
    let h = Home::read(&serde_json::to_vec(&answer).unwrap()).unwrap();
    assert!(h.setting_up().is_empty());
    assert_eq!(h.other_agents().len(), 1);
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
        happened: vec![],
        card_units: vec![],
        knocks: vec![],
        empty: Some(("Nothing is waiting.".into(), String::new())),
    };
    let lines = with_waits(empty, &home, false, &|_| false);
    assert_eq!(lines.steps.len(), 1);
    // Coach's waits that can't be drawn are left out, and never fail Home.
    assert_eq!(lines.coach.len(), 2);
    assert!(lines.set_all_aside.is_some());
    assert_eq!(lines.empty, None);
    // A step without its handoff fails Home, as the iPad's decoder does.
    answer["your_steps"] = serde_json::json!([{"what": "no handoff"}]);
    assert!(Home::read(&serde_json::to_vec(&answer).unwrap()).is_none());
}

#[test]
fn did_this_happen_beside_the_rest() {
    use tafs_core::needs_you::{with_unproven, NeedsYouLines};
    use tafs_core::unproven::read_unproven;
    let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
    let empty = || NeedsYouLines {
        title: String::new(),
        cards: vec![],
        steps: vec![],
        coach: vec![],
        set_all_aside: None,
        opens_coach: false,
        happened: vec![],
        card_units: vec![],
        knocks: vec![],
        empty: Some(("Nothing is waiting.".into(), String::new())),
    };
    // A call KIFF can't prove waits too: "Nothing is waiting." goes.
    let calls =
        read_unproven(br#"{"calls":[{"proposal":"p_1","title":"Refund a payment"}]}"#).unwrap();
    let lines = with_unproven(empty(), &w, &calls).unwrap();
    assert_eq!(lines.happened.len(), 1);
    assert_eq!(lines.happened[0].headline, "Refund a payment");
    assert_eq!(lines.empty, None);
    // None waiting: nothing drawn, and "Nothing is waiting." stays.
    let lines = with_unproven(empty(), &w, &[]).unwrap();
    assert!(lines.happened.is_empty());
    assert!(lines.empty.is_some());
}

#[test]
fn card_unit_requests_from_home() {
    use tafs_core::needs_you::{with_waits, NeedsYouLines};
    let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
    let ask = serde_json::json!({
        "id": "exc-1", "kind": "card_unit", "taf": "taf-aaaa0000000000000000", "name": "Rae",
        "title": "Rae needs a USD limit", "words": "Its Card only allows EUR.",
        "history": "Tried 4 times", "action_title": "Add a USD limit", "dismiss_title": "Dismiss",
        "can_approve_once": false, "coach_text": "Add a USD limit for Rae."
    });
    answer["card_unit_exceptions"] = serde_json::json!([ask]);
    let home = Home::read(&serde_json::to_vec(&answer).unwrap()).expect("Home reads");
    let empty = || NeedsYouLines {
        title: String::new(),
        cards: vec![],
        steps: vec![],
        coach: vec![],
        set_all_aside: None,
        opens_coach: false,
        happened: vec![],
        card_units: vec![],
        knocks: vec![],
        empty: Some(("Nothing is waiting.".into(), String::new())),
    };
    // Someone who may use Coach and manages the taf: its Coach button; and "Nothing is waiting." goes.
    let lines = with_waits(empty(), &home, true, &|taf| {
        taf == "taf-aaaa0000000000000000"
    });
    assert_eq!(lines.card_units.len(), 1);
    let unit = &lines.card_units[0];
    assert_eq!(unit.title, "Rae needs a USD limit");
    assert_eq!(unit.words, "Its Card only allows EUR.");
    assert_eq!(unit.history, "Tried 4 times");
    assert_eq!(unit.discuss.as_deref(), Some("Add a USD limit"));
    assert_eq!(unit.dismiss, "Dismiss");
    assert_eq!(lines.empty, None);
    // Not managing it, or not using Coach: no Coach button.
    let lines = with_waits(empty(), &home, true, &|_| false);
    assert_eq!(lines.card_units[0].discuss, None);
    let lines = with_waits(empty(), &home, false, &|_| true);
    assert_eq!(lines.card_units[0].discuss, None);
    // Each field is the iPad's `let`: one missing fails Home, as its synthesized decoder does.
    let mut broken = ask.clone();
    broken.as_object_mut().unwrap().remove("history");
    answer["card_unit_exceptions"] = serde_json::json!([broken]);
    assert!(Home::read(&serde_json::to_vec(&answer).unwrap()).is_none());
}

#[test]
fn knocks_at_a_tafs_doors() {
    use tafs_core::needs_you::{with_knocks, Knocked, NeedsYouLines};
    let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
    let mut answer: Value = serde_json::from_slice(&contract("home.json")).unwrap();
    // A door's missing or wrong-kind fields read as nothing to show; the knocks add up.
    answer["tafs"][0]["doors"] = serde_json::json!([
        {"app": "slack", "open": true, "knocks": 2},
        {"app": "email", "knocks": 1, "resting": "not a bool"},
        {"knocks": "three"},
        {}
    ]);
    let home = Home::read(&serde_json::to_vec(&answer).unwrap()).expect("Home reads");
    let doors = home.tafs[0].doors.clone().unwrap();
    assert_eq!(doors.len(), 4);
    assert!(doors[0].open);
    assert!(!doors[1].resting);
    assert_eq!(doors[2].knocks, 0);
    assert_eq!(doors[3].app, "");
    let key = home.tafs[0].key();
    assert_eq!(home.knocks().get(&key), Some(&3));
    // A knock makes the taf relevant now, with nothing else.
    let taf = home::taf(&w, &home.tafs[0]).unwrap();
    assert_eq!(taf.knocks, 3);
    assert!(taf.knocked);
    let long_ago = Local.with_ymd_and_hms(2030, 1, 1, 12, 0, 0).unwrap();
    assert!(home::relevant(&taf, false, long_ago));
    // A door that isn't an object fails Home, as the iPad's keyed decoder does.
    answer["tafs"][0]["doors"] = serde_json::json!(["slack"]);
    assert!(Home::read(&serde_json::to_vec(&answer).unwrap()).is_none());

    let empty = || NeedsYouLines {
        title: String::new(),
        cards: vec![],
        steps: vec![],
        coach: vec![],
        set_all_aside: None,
        opens_coach: false,
        happened: vec![],
        card_units: vec![],
        knocks: vec![],
        empty: Some(("Nothing is waiting.".into(), String::new())),
    };
    let tafs = |rae: i64, sam: i64| {
        vec![
            Knocked {
                key: "t-rae",
                name: "Rae",
                initial: "R",
                color: 1,
                knocks: rae,
            },
            Knocked {
                key: "t-sam",
                name: "Sam",
                initial: "S",
                color: 2,
                knocks: sam,
            },
        ]
    };
    // Only tafs with knocks, in the tafs' order, with the count's form; "Nothing is waiting." goes.
    let lines = with_knocks(empty(), &w, &tafs(1, 4)).unwrap();
    assert_eq!(lines.knocks.len(), 2);
    assert_eq!(lines.knocks[0].key, "t-rae");
    assert_eq!(
        lines.knocks[0].line,
        "Someone knocked at its door. Nothing was done."
    );
    assert_eq!(
        lines.knocks[1].line,
        "4 knocks at its doors. Nothing was done."
    );
    assert_eq!(lines.knocks[1].hint, "Opens Sam, where you answer it");
    assert_eq!(lines.empty, None);
    let lines = with_knocks(empty(), &w, &tafs(0, 0)).unwrap();
    assert!(lines.knocks.is_empty());
    assert!(lines.empty.is_some());
}

#[test]
fn recipes_for_home_with_no_taf() {
    use tafs_core::recipes::{Recipes, Tint};
    // The pinned answer is Home's part, the recipes left out: it reads, and lists nothing.
    let mut answer: Value = serde_json::from_slice(&contract("recipes-home.json")).unwrap();
    let r = Recipes::read(&serde_json::to_vec(&answer).unwrap()).expect("the contract reads");
    assert_eq!(r.home_words(), Some("Pick a job for your first taf."));
    assert_eq!(
        r.home_own_words().unwrap().title,
        "Say it in your own words"
    );
    assert_eq!(r.hand_words(), Some("Hand it to Coach"));
    assert!(!r.home_lists_recipes());

    answer["categories"] = serde_json::json!([
        {"id": "money", "title": "Money", "color": {"light": "#B45309", "dark": "#F59E0B"}},
        {"id": "support", "title": "Support", "color": {"light": "orange", "dark": "#000000"}},
        {"id": "empty", "title": "Nothing in it"}
    ]);
    answer["recipes"] = serde_json::json!([
        {"id": "r1", "title": "Refunds", "category": "money", "headline": "Refunds wait days",
         "line": "Refunds within the Card.", "say": "Set up refunds",
         "card": {"allows": ["Refund up to €50"], "holds": ["Refunds over €50"], "never": [1]},
         "apps": [{"name": "Stripe", "logo": "/v1/logos/stripe"}, {"logo": "/v1/logos/x"}, {"name": "Gmail"}]},
        {"id": "r2", "title": "Inbox", "category": "support", "headline": "  ", "proved": "  Proved.  "}
    ]);
    let r = Recipes::read(&serde_json::to_vec(&answer).unwrap()).expect("reads");
    assert!(r.home_lists_recipes());
    // A colour either half of which isn't #RRGGBB is none.
    assert_eq!(
        r.categories[0].color,
        Some(Tint {
            light: 0xB45309,
            dark: 0xF59E0B
        })
    );
    assert_eq!(r.categories[1].color, None);
    // A category with no recipe isn't listed; tafs cloud's order otherwise.
    let groups = r.by_category();
    assert_eq!(
        groups
            .iter()
            .map(|(c, _)| c.id.as_str())
            .collect::<Vec<_>>(),
        ["money", "support"]
    );
    // Picked narrows; a pick no longer listed narrows nothing.
    assert_eq!(r.groups(Some("support")).len(), 1);
    assert_eq!(r.groups(Some("gone")).len(), 2);
    let refunds = &r.recipes[0];
    assert_eq!(refunds.lead(), "Refunds wait days");
    // An app without its name is left out; a zone the Card leaves empty, or can't read, isn't one.
    assert_eq!(
        refunds
            .apps
            .iter()
            .map(|a| a.name.as_str())
            .collect::<Vec<_>>(),
        ["Stripe", "Gmail"]
    );
    let zones = r.zones(refunds);
    assert_eq!(
        zones.iter().map(|z| z.id).collect::<Vec<_>>(),
        ["allows", "holds"]
    );
    assert_eq!(zones[0].title, "Does on its own");
    // A blank headline leads with the title; words are trimmed.
    assert_eq!(r.recipes[1].lead(), "Inbox");
    assert_eq!(r.recipes[1].proved, "Proved.");

    // What the iPad requires, this requires: a recipe without its title, or a `line` that isn't
    // words, fails the read.
    let mut broken = answer.clone();
    broken["recipes"][0]
        .as_object_mut()
        .unwrap()
        .remove("title");
    assert!(Recipes::read(&serde_json::to_vec(&broken).unwrap()).is_none());
    let mut broken = answer.clone();
    broken["recipes"][1]["line"] = serde_json::json!(3);
    assert!(Recipes::read(&serde_json::to_vec(&broken).unwrap()).is_none());
    // What it forgives, this forgives: `home` or `labels` that can't be read are none.
    let mut odd = answer.clone();
    odd["labels"] = serde_json::json!("labels");
    let r = Recipes::read(&serde_json::to_vec(&odd).unwrap()).unwrap();
    assert!(r.labels.is_none());
    assert!(!r.home_lists_recipes());
}

#[test]
fn the_jobs_gallery() {
    use tafs_core::recipes::{Gallery, Labels};
    let answer = serde_json::json!({
        "categories": [
            {"id": "money", "title": "Money"},
            {"title": "no id"},
            "not even an object"
        ],
        "recipes": [
            {"id": "r1", "title": "Refunds", "category": "money", "headline": "Refunds wait days"},
            {"id": "r2", "title": "Chargebacks", "category": "money"},
            {"id": "r3", "category": "money"},
            {"id": "r4", "title": "Bad line", "category": "money", "line": 4}
        ],
        "strip": {"shown": true, "title": "Jobs to try", "ids": ["r2", "gone", "r1"]},
        "words": {"title": "New taf", "all": "All jobs", "own_words": "Say it in your own words",
                  "explore": "  Explore jobs  ", "see_all": "", "try": "Try it with Coach",
                  "sample": "What it reported"},
        "labels": {"allows": "Does on its own", "hand": "Hand it to Coach"}
    });
    let g = Gallery::read(&serde_json::to_vec(&answer).unwrap()).expect("reads");
    // A category or a recipe that can't be read is left out on its own; the rest stay.
    assert_eq!(g.categories.len(), 1);
    assert_eq!(
        g.recipes.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["r1", "r2"]
    );
    // The strip in tafs cloud's order, an id the gallery doesn't list skipped.
    assert_eq!(
        g.strip_recipes()
            .iter()
            .map(|r| r.id.as_str())
            .collect::<Vec<_>>(),
        ["r2", "r1"]
    );
    // The strip shows every job: no "See all".
    assert_eq!(g.see_all(), None);
    assert_eq!(g.explore(), Some("Explore jobs"));
    assert_eq!(g.title(true), "Explore jobs");
    assert_eq!(g.title(false), "New taf");
    assert_eq!(
        g.hand_words(true, g.labels.as_ref()),
        Some("Try it with Coach")
    );
    assert_eq!(
        g.hand_words(false, g.labels.as_ref()),
        Some("Hand it to Coach")
    );
    assert_eq!(g.sample_label(), Some("What it reported"));
    assert_eq!(g.category_of(&g.recipes[0]).unwrap().title, "Money");
    // As recipes: its own labels, else the fallback's.
    let fallback = Labels {
        hand: "Fallback".into(),
        ..Default::default()
    };
    assert_eq!(
        g.as_recipes(Some(&fallback)).hand_words(),
        Some("Hand it to Coach")
    );
    let mut no_labels = g.clone();
    no_labels.labels = None;
    assert_eq!(
        no_labels.as_recipes(Some(&fallback)).hand_words(),
        Some("Fallback")
    );

    // A strip that hides, or shows fewer than all: "See all jobs", else "All jobs".
    let mut fewer = answer.clone();
    fewer["strip"]["ids"] = serde_json::json!(["r1"]);
    let g = Gallery::read(&serde_json::to_vec(&fewer).unwrap()).unwrap();
    assert_eq!(g.see_all(), Some("All jobs"));
    fewer["words"]["see_all"] = serde_json::json!("See all jobs");
    let g = Gallery::read(&serde_json::to_vec(&fewer).unwrap()).unwrap();
    assert_eq!(g.see_all(), Some("See all jobs"));
    fewer["strip"]["shown"] = serde_json::json!(false);
    let g = Gallery::read(&serde_json::to_vec(&fewer).unwrap()).unwrap();
    assert!(g.strip_recipes().is_empty());

    // Everything forgiven: words and strip that can't be read are empty; no jobs, no Explore row.
    let g = Gallery::read(br#"{"words": 3, "strip": "x", "recipes": "x"}"#).unwrap();
    assert_eq!(g.words.title, "");
    assert!(!g.strip.shown);
    assert_eq!(g.explore(), None);
    assert!(Gallery::read(b"[]").is_none());
}

#[test]
fn a_halt_as_tafs_cloud_answers_it() {
    use tafs_core::halt::{self, Action, State};
    let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
    let read = |v: serde_json::Value| halt::read(&serde_json::to_vec(&v).unwrap());
    // Halting: the parts that hold it, and Halt again.
    let h = read(
        serde_json::json!({"halt": {"state": "halting", "runtime": true, "kiff": true,
        "planque": false, "account": false}}),
    )
    .expect("reads");
    assert_eq!(h.state, State::Halting);
    let l = halt::lines(&w, &h, Some("Rae")).unwrap();
    assert_eq!(l.title, "Halt");
    assert_eq!(l.name, "Rae");
    assert_eq!(l.headline, "Halting…");
    let (who, parts) = l.parts.unwrap();
    assert_eq!(who, "Who holds the halt");
    assert_eq!(
        parts
            .iter()
            .map(|p| (p.held, p.status.as_str()))
            .collect::<Vec<_>>(),
        [(true, "Done"), (true, "Done"), (false, "Waiting")]
    );
    assert!(matches!(l.action, Action::Halting { .. }));

    // Running: no parts, and one tap to halt; the account's says "all".
    let running = halt::Halt::running();
    let l = halt::lines(&w, &running, None).unwrap();
    assert_eq!(l.title, "Halt all");
    assert_eq!(l.name, "All your tafs");
    assert_eq!(
        l.explanation,
        "Halting stops your tafs until you resume. A call already sent may still arrive."
    );
    assert_eq!(l.parts, None);
    assert_eq!(l.action, Action::Halt("Halt all now".into()));
    assert_eq!(
        halt::row(&w, &running, false).unwrap(),
        ("Halt".into(), "Stop it until you resume".into(), false)
    );

    // Halted: resuming is signed, so this computer says why; halted with the account, nothing.
    let halted = read(
        serde_json::json!({"halt": {"state": "halted", "halted_at": "2026-11-16T09:30:00Z",
        "runtime": true, "kiff": true, "planque": true, "account": false, "stopped": 2}}),
    )
    .unwrap();
    assert_eq!(halted.stopped, Some(2));
    let l = halt::lines(&w, &halted, Some("Rae")).unwrap();
    assert_eq!(l.headline, "Halted");
    assert!(l.explanation.starts_with("Since "));
    assert_eq!(
        l.action,
        Action::Resume {
            button: "Resume Rae".into(),
            why: "This device can't approve. Approve on another device signed in to your account."
                .into()
        }
    );
    let mut with_all = halted.clone();
    with_all.account = true;
    let l = halt::lines(&w, &with_all, Some("Rae")).unwrap();
    assert_eq!(l.action, Action::None);
    assert_eq!(
        l.with_all.as_deref(),
        Some("Halted with all your tafs. Resume them all to let it run.")
    );
    assert!(halt::row(&w, &halted, true).unwrap().2);

    // Read as the iPad's synthesized decoder: a state it doesn't know, a missing part, or a time
    // that isn't one fails the read.
    for bad in [
        serde_json::json!({"halt": {"state": "paused", "runtime": true, "kiff": true, "planque": true, "account": false}}),
        serde_json::json!({"halt": {"state": "halted", "runtime": true, "kiff": true, "account": false}}),
        serde_json::json!({"halt": {"state": "halted", "halted_at": "yesterday", "runtime": true, "kiff": true, "planque": true, "account": false}}),
        serde_json::json!({"state": "halted"}),
    ] {
        assert!(read(bad.clone()).is_none(), "{bad}");
    }
    // The paths, a taf's id as one segment.
    assert_eq!(halt::path(None), "/v1/halt");
    assert_eq!(halt::path(Some("taf-1")), "/v1/tafs/taf-1/halt");
    assert_eq!(halt::path(Some("a/b c")), "/v1/tafs/a%2Fb%20c/halt");
    assert_eq!(
        halt::not_halted(&w, false, "KIFF answered 503").unwrap(),
        "tafs couldn't halt it. KIFF answered 503"
    );
}

#[test]
fn the_record_searched() {
    use tafs_core::record_search::{self, RecordCase, Search, SearchPage};
    let activity: Value = serde_json::from_slice(&contract("taf-activity.json")).unwrap();
    let entry = activity["entries"]
        .get(0)
        .cloned()
        .unwrap_or_else(|| activity["groups"][0]["entries"][0].clone());
    let mut found = entry.clone();
    found["taf"] = serde_json::json!({"id": "taf-1", "name": "Rae"});
    found["about"] = serde_json::json!("Order #1042");
    let mut blank_about = found.clone();
    blank_about["about"] = serde_json::json!("  ");
    let page = SearchPage::read(
        &serde_json::to_vec(&serde_json::json!({
            "q": "1042", "hint": "Search an order, an invoice, an amount",
            "entries": [found, blank_about], "next_cursor": "c2", "none_words": "Nothing matched."
        }))
        .unwrap(),
    )
    .expect("reads");
    assert_eq!(page.hint, "Search an order, an invoice, an amount");
    assert_eq!(page.entries[0].taf_id, "taf-1");
    assert_eq!(page.entries[0].taf_name, "Rae");
    assert_eq!(page.entries[0].about.as_deref(), Some("Order #1042"));
    // A blank about opens no case.
    assert_eq!(page.entries[1].about, None);
    // No groups: every entry as one, untitled.
    let shown = page.shown_groups();
    assert_eq!(shown.len(), 1);
    assert_eq!(shown[0].title, "");
    assert_eq!(shown[0].entries.len(), 2);
    // More follows: no end words yet; nothing matched only when every page is empty.
    let s = Search {
        q: "1042".into(),
        pages: vec![page],
        loading: false,
        error: None,
    };
    assert_eq!(s.next_cursor(), Some("c2"));
    assert_eq!(s.end_words(), None);
    assert_eq!(s.none_words(), None);
    let empty =
        SearchPage::read(br#"{"none_words": "Nothing matched.", "end_words": "That's all."}"#)
            .unwrap();
    let s = Search {
        q: "x".into(),
        pages: vec![empty],
        loading: false,
        error: None,
    };
    assert_eq!(s.none_words(), Some("Nothing matched."));
    assert_eq!(s.end_words(), Some("That's all."));

    // Read as the iPad's decoder: a hit without its taf's id, or entries that aren't a list, fail.
    let mut no_taf = entry.clone();
    no_taf["taf"] = serde_json::json!({"name": "Rae"});
    assert!(SearchPage::read(
        &serde_json::to_vec(&serde_json::json!({"entries": [no_taf]})).unwrap()
    )
    .is_none());
    assert!(SearchPage::read(br#"{"entries": "x"}"#).is_none());
    // The words are forgiven.
    assert_eq!(SearchPage::read(br#"{"hint": 3}"#).unwrap().hint, "");

    // A case: about required, failed forgiven, its words only when there are some.
    let mut step = entry.clone();
    step["taf"] = serde_json::json!({"id": "taf-1"});
    let case = RecordCase::read(
        &serde_json::to_vec(&serde_json::json!({
            "about": "Order #1042", "steps": [step],
            "failed": {"id": "e1", "words": "The refund didn't go through.",
                       "next": {"kind": "try_later", "words": "Try again later"}}
        }))
        .unwrap(),
    )
    .unwrap();
    assert_eq!(case.steps[0].taf_name, "");
    assert_eq!(case.failed_words(), Some("The refund didn't go through."));
    assert_eq!(case.failed.unwrap().next.unwrap().kind, "try_later");
    let case = RecordCase::read(br#"{"about": "x", "failed": "not an object"}"#).unwrap();
    assert_eq!(case.failed, None);
    assert!(RecordCase::read(br#"{"steps": []}"#).is_none());

    // The paths: what's typed encoded but its letters and digits; the case's about as one segment.
    assert_eq!(
        record_search::search_path("€40 #1042", None, None),
        "/v1/record?q=%E2%82%AC40%20%231042"
    );
    assert_eq!(
        record_search::search_path("a", Some("c/2"), Some(1)),
        "/v1/record?q=a&cursor=c%2F2&limit=1"
    );
    assert_eq!(
        record_search::case_path("Order #1042/x"),
        "/v1/record/cases/Order%20%231042%2Fx"
    );
    // A dot segment stays a segment: never `/v1/record/cases/..`.
    assert_eq!(record_search::case_path(".."), "/v1/record/cases/%2E%2E");
    assert_eq!(record_search::case_path("."), "/v1/record/cases/%2E");
    assert_eq!(record_search::case_path("v1.2"), "/v1/record/cases/v1%2E2");
    assert_eq!(record_search::asked("  1042 "), Some("1042".into()));
    assert_eq!(record_search::asked("   "), None);
}

/// A Card's lines in tafs cloud's and KIFF's own words (`TafCard`): each item's totals (none
/// blank), how it's counted, what's pending (money then calls; shown only for a live, active
/// Card) and why its total has no currency. The contract's `home.json` sends none of them, so
/// they're added to its Card here; its time zone line is the contract's own.
#[test]
fn a_card_s_own_lines_come_from_home_as_sent() {
    let w = en();
    let mut answer: serde_json::Value = serde_json::from_slice(&contract("home.json")).unwrap();
    assert_eq!(
        tafs_core::home::Home::read(&serde_json::to_vec(&answer).unwrap())
            .unwrap()
            .tafs[0]
            .time_zone_words
            .as_deref(),
        Some("Times in UTC")
    );
    let card = &mut answer["tafs"][0]["card"]["cards"][0];
    card["each_item"] = serde_json::json!(["1 call in any 7 days for each invoice id", "  "]);
    card["counted"] = serde_json::json!([{"tool": "stripe.refund", "title": "Create refund", "lines": ["Counted in EUR, by its amount"]}]);
    card["total"]["pending_text"] = serde_json::json!("€40 refused by Stripe still counts");
    card["total"]["unit_words"] =
        serde_json::json!("Its currency comes from Stripe, which isn't connected.");
    let home = tafs_core::home::Home::read(&serde_json::to_vec(&answer).unwrap()).unwrap();
    let cards = tafs_core::home::full_cards(&w, &home.tafs[0], "en").unwrap();
    let c = &cards[0];
    assert_eq!(
        c.extra.each_item,
        ["1 call in any 7 days for each invoice id"]
    );
    assert_eq!(c.extra.counted[0].title, "Create refund");
    assert_eq!(c.extra.counted[0].lines, ["Counted in EUR, by its amount"]);
    assert_eq!(c.extra.pending, ["€40 refused by Stripe still counts"]);
    assert_eq!(
        c.extra.unit_words.as_deref(),
        Some("Its currency comes from Stripe, which isn't connected.")
    );
    assert_eq!(c.pending_shown().len(), usize::from(c.live && c.active));
}

#[test]
fn every_ai_consent_answer_reads_as_the_ipad_reads_it() {
    use tafs_core::ai_consent::read_consent;
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../shared/contracts/ai-consent.json");
    let file: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let cases = file["cases"].as_array().unwrap();
    assert!(!cases.is_empty());
    for case in cases {
        let body = serde_json::to_vec(&case["body"]).unwrap();
        let c = read_consent(&body).unwrap_or_else(|| panic!("{} reads", case["name"]));
        assert!(c.drawable(), "{}", case["name"]);
        // Given only where tafs cloud says so, with the moment it was given.
        assert_eq!(c.given, c.at.is_some(), "{}", case["name"]);
    }
}
