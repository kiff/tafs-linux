use super::*;

const SETUP: &str = include_str!("../../../tafs-app/fixtures/taf-knob-setup.json");

fn words(lang: &str) -> WordBook {
    WordBook::load(&WordBook::default_root(), lang).unwrap()
}

#[test]
fn the_dial_reads_as_the_ipad_decodes_it() {
    let k = read(SETUP.as_bytes()).unwrap();
    assert!(k.can_pause);
    assert_eq!(k.version, Some(7));
    assert_eq!(
        k.pending,
        Some(Pending {
            version: 8,
            changes: 2
        })
    );
    assert_eq!(k.wakes.len(), 5);
    assert_eq!(k.wakes_of("app_event").len(), 2);
    let schedule = &k.wakes[0];
    assert!(schedule.on, "a stop that doesn't say is on");
    let size = schedule.size.as_ref().unwrap();
    assert_eq!(size.stepper_label(), "Every 4 hours · Every 4 hours");
    assert_eq!(size.choice(-1).unwrap().label, "Every 2 hours");
    assert_eq!(size.choice(1).unwrap().label, "Every 8 hours");
    assert_eq!(size.choice(2), None);
    let event = &k.wakes[1];
    assert!(!event.on);
    assert_eq!(event.approval.as_ref().unwrap().state, "needed");
    assert_eq!(k.wakes[4].follow_up.as_deref(), Some("fu-1"));
    // `busy` from `when.busy`, else "skip".
    assert_eq!(k.setting.as_ref().unwrap().busy, "skip");
    assert!(k.setting.as_ref().unwrap().sends_to("notion"));
    assert!(!k.setting.as_ref().unwrap().sends_to("owner_email"));
    assert_eq!(k.rim.total(), 13);
    // Empty is a dial with nothing on it; nulls take their defaults.
    let empty = read(br#"{"wakes":null,"rim":null,"can_pause":null}"#).unwrap();
    assert!(empty.can_pause && empty.wakes.is_empty());
    // A field in the wrong shape: the dial isn't read.
    assert!(read(br#"{"version":"7"}"#).is_none());
    assert!(read(br#"{"wakes":[{"label":"No id"}]}"#).is_none());
    assert!(read(br#"{"wakes":[{"id":"a","on":"yes"}]}"#).is_none());
    assert!(read(br#"{"pending":{"version":8}}"#).is_none());
    assert!(read(
        br#"{"wakes":[{"id":"a","approval":{"state":"needed","grant":"g","event":"E"}}]}"#
    )
    .is_none());
}

#[test]
fn a_setting_goes_back_as_put_takes_it() {
    let k = read(SETUP.as_bytes()).unwrap();
    let mut s = k.setting.clone().unwrap();
    s.busy = "latest".into();
    let sent: Value = serde_json::from_slice(&s.body()).unwrap();
    assert_eq!(sent["when"]["busy"], "latest");
    assert_eq!(sent["wakes"][0]["every"], "4h");
    assert_eq!(sent["results"][2]["page"], "p1");
    assert!(sent.get("busy").is_none());
    assert!(sent.get("what").is_none());
    let direct: Setting = serde_json::from_str(r#"{"busy":"latest"}"#).unwrap();
    assert_eq!((direct.busy.as_str(), direct.wakes.len()), ("latest", 0));
}

#[test]
fn choices_wait_for_the_newest_draft() {
    let mut k = read(SETUP.as_bytes()).unwrap();
    // Version 8 waits and the dial shows version 7: no choice on it.
    assert!(!k.accepts_choices(None));
    k.shown = Some(8);
    assert!(k.accepts_choices(None));
    assert!(!k.accepts_choices(Some(9)));
    k.pending = None;
    k.draft = None;
    k.shown = None;
    assert!(k.accepts_choices(None));
    assert!(k.accepts_choices(Some(7)));
    assert!(!k.accepts_choices(Some(8)));
}

#[test]
fn tafs_own_places() {
    let k = read(SETUP.as_bytes()).unwrap();
    let live = [
        Value::from("app"),
        serde_json::json!({"place": "door_reply"}),
    ];
    assert_eq!(
        live_built_ins(Some(&live), Some(&k)).unwrap(),
        ["door_reply", "app"]
    );
    // Without the live taf's, only a dial showing the live version says.
    assert_eq!(live_built_ins(None, Some(&k)), None);
    let mut live_dial = k.clone();
    live_dial.pending = None;
    assert_eq!(
        live_built_ins(None, Some(&live_dial)).unwrap(),
        ["door_reply", "app"]
    );
    assert_eq!(live_built_ins(None, None), None);
    let on = vec!["door_reply".to_string(), "app".to_string()];
    assert_eq!(
        places(&on, "owner_email", true).unwrap(),
        ["door_reply", "app", "owner_email"]
    );
    assert_eq!(places(&on, "door_reply", false).unwrap(), ["app"]);
    assert_eq!(places(&["app".to_string()], "app", false), None);
    let notion = k.results[3].size.as_ref().unwrap();
    assert_eq!(notion.toggled().unwrap().label, "Off");
    assert_eq!(k.results[1].size, None);
}

#[test]
fn paths_bodies_and_answers() {
    assert_eq!(path("t 1", None), "/v1/tafs/t%201/knob");
    assert_eq!(path("t", Some(8)), "/v1/tafs/t/knob?version=8");
    assert_eq!(results_path("t"), "/v1/tafs/t/results");
    assert_eq!(
        results_body(&["door_reply".into(), "app".into()]),
        br#"{"places":["door_reply","app"]}"#
    );
    assert_eq!(follow_up_path("t", "fu/1"), "/v1/tafs/t/follow-ups/fu%2F1");
    assert!(removed(br#"{"removed":"fu-1"}"#));
    assert!(!removed(br#"{}"#));
    assert_eq!(pause_path("t", true), "/v1/tafs/t/pause");
    assert_eq!(pause_path("t", false), "/v1/tafs/t/resume");
    assert_eq!(
        read_change(br#"{"version":8,"words":["Runs every 2 hours"]}"#)
            .unwrap()
            .version,
        8
    );
    assert!(read_change(br#"{"words":[]}"#).is_none());
    let t = read_taf(br#"{"taf":{"status":"paused","results":["app"]}}"#).unwrap();
    assert!(t.paused);
    assert_eq!(t.results.unwrap(), [Value::from("app")]);
    assert!(!read_taf(br#"{"taf":{"status":"active"}}"#).unwrap().paused);
    assert!(read_taf(br#"{"taf":{"results":"app"}}"#).is_none());
    assert!(read_taf(br#"{}"#).is_none());
}

#[test]
fn the_pending_panel_and_the_places_in_words() {
    let w = words("en");
    let k = read(SETUP.as_bytes()).unwrap();
    let (head, lines, button) = pending_lines(&w, &k).unwrap().unwrap();
    assert_eq!(head, "DRAFT · V8 · NOT LIVE");
    assert_eq!(lines.len(), 2);
    assert_eq!(button, "Make 2 changes live with your passcode");
    let rows = built_in_rows(&w, Some("rae@acme.example")).unwrap();
    assert_eq!(
        rows[2].2,
        "Its answer to rae@acme.example; not when you wrote the message"
    );
    for lang in crate::words::SHIPPED {
        let w = words(lang);
        pending_lines(&w, &k).unwrap().unwrap();
        built_in_rows(&w, None).unwrap();
    }
}

fn nav_knob() -> Knob {
    read(br#"{
      "rings": [{"key":"what","name":"Does"},{"key":"wakes","name":"Starts"},{"key":"what","name":"x"}],
      "connections": [{"grant":"g_log","name":"Notion","wakes":0,"results":1}],
      "wakes": [{"id":"daily","label":"Daily"}],
      "what": [{"id":"step1","label":"Check","step":1},{"id":"step2","label":"Log","step":2,"grant":"g_log"}],
      "results": [{"id":"log","label":"Log","grant":"g_log"}]
    }"#)
    .unwrap()
}

#[test]
fn the_remote_walks_the_stops_a_filter_shows() {
    let k = nav_knob();
    // tafs cloud's order, each once; Results left out as it named none.
    assert_eq!(k.ordered_rings(), [Ring::What, Ring::Wakes]);
    let w = words("en");
    assert_eq!(k.title(&w, Ring::What).unwrap(), "Does");
    assert_eq!(k.title(&w, Ring::Results).unwrap(), "Where");
    let own = nav::stops(&k, &nav::Filter::Taf);
    assert_eq!(
        own.iter().map(|s| s.item.id.as_str()).collect::<Vec<_>>(),
        ["step1", "daily"]
    );
    let log = nav::stops(&k, &nav::Filter::Connection("g_log".into()));
    assert_eq!(
        log.iter().map(|s| s.item.id.as_str()).collect::<Vec<_>>(),
        ["step2"]
    );
    assert!(nav::stops(&k, &nav::Filter::Connection("missing".into())).is_empty());
    let all = nav::stops(&k, &nav::Filter::All);
    assert_eq!(nav::next(&all, None, None, 1).unwrap().item.id, "step1");
    assert_eq!(nav::next(&all, None, None, -1).unwrap().item.id, "daily");
    assert_eq!(
        nav::next(&all, Some(Ring::What), Some("step2"), 1)
            .unwrap()
            .item
            .id,
        "daily"
    );
    assert_eq!(
        nav::next(&all, Some(Ring::Wakes), Some("daily"), 1)
            .unwrap()
            .item
            .id,
        "step1"
    );
    assert_eq!(
        nav::next(&all, Some(Ring::What), Some("step1"), -1)
            .unwrap()
            .item
            .id,
        "daily"
    );
    assert!(nav::next(&[], None, None, 1).is_none());
    // `KnobNavigationTests`' measures.
    assert_eq!(nav::nearest_turn(0, 3, 4), -1);
    assert_eq!(nav::nearest_turn(5, 1, 4), 5);
    assert!((nav::label_width(90.0, 6, 100.0, 12.0) - 78.0).abs() < 1e-9);
    assert_eq!(nav::label_width(180.0, 6, 100.0, 12.0), 100.0);
    assert_eq!(nav::label_width(90.0, 1, 100.0, 12.0), 100.0);
    assert_eq!(nav::label_width(2.0, 6, 100.0, 12.0), 0.0);
    // `testLowerHalfLabelsFlipAfterTheRingTurns`.
    for (at, flip) in [
        (0.0, 0.0),
        (90.0, 0.0),
        (180.0, 180.0),
        (270.0, 0.0),
        (360.0, 0.0),
        (-180.0, 180.0),
        // The bottom stop moves to the pointer after half a turn and no longer flips.
        (180.0 - 2.0 * 90.0, 0.0),
        (180.0 + 4.0 * 360.0, 180.0),
    ] {
        assert_eq!(nav::label_flip(at), flip, "at {at}");
    }
    let items: Vec<Item> = (0..10)
        .map(|i| Item {
            id: format!("i{i}"),
            ..k.wakes[0].clone()
        })
        .collect();
    assert_eq!(nav::page(&items, Some("i5"), 4)[0].id, "i4");
    assert_eq!(nav::page(&items, Some("i9"), 4).len(), 2);
    assert_eq!(nav::page(&items, None, 4)[0].id, "i0");
    assert!(nav::page(&items, None, 0).is_empty());
}

#[test]
fn the_face_says_the_stop_the_call_or_the_job() {
    let w = words("en");
    let k = read(SETUP.as_bytes()).unwrap();
    let all = nav::Filter::All;
    let o = face_lines(&w, &k, &Face::Overview, &all, &[]).unwrap();
    assert_eq!((o.kicker.as_str(), o.big.as_str()), ("WHY", "Refund"));
    assert_eq!(o.hint, "Tap a ring");
    let s = face_lines(
        &w,
        &k,
        &Face::Chosen(Ring::Wakes, "schedule_1".into()),
        &all,
        &[],
    )
    .unwrap();
    assert_eq!(s.kicker, "WHEN · EVERY 4 HOURS");
    assert_eq!(s.big, "Every 4 hours");
    assert_eq!(s.hint, "Use \u{2212} and +");
    assert_eq!(s.size.as_deref(), Some("Every 4 hours · Every 4 hours"));
    assert_eq!(s.tint, Tint::Ring(Ring::Wakes));
    let e = face_lines(
        &w,
        &k,
        &Face::Chosen(Ring::Wakes, "event_1".into()),
        &all,
        &[],
    )
    .unwrap();
    assert_eq!(e.hint, "Approve with your passcode");
    let d = face_lines(
        &w,
        &k,
        &Face::Chosen(Ring::Wakes, "door_email".into()),
        &all,
        &[],
    )
    .unwrap();
    assert_eq!(
        (d.big.as_str(), d.hint.as_str()),
        ("Open", "Ask Coach to change")
    );
    let held = HeldFace {
        id: "h1".into(),
        summary: "Refund €150".into(),
        amount: Some("€150".into()),
        reason: "Over its €100 limit".into(),
        approvable_once: true,
        accounting_version: Some(1),
    };
    let h = face_lines(
        &w,
        &k,
        &Face::Held("h1".into()),
        &all,
        std::slice::from_ref(&held),
    )
    .unwrap();
    assert_eq!((h.kicker.as_str(), h.big.as_str()), ("REFUND €150", "€150"));
    assert_eq!(h.hint, "Tap to approve once");
    assert_eq!(h.tint, Tint::Held);
    // A stop the filter hides: the job.
    let hidden = face_lines(
        &w,
        &k,
        &Face::Chosen(Ring::Wakes, "schedule_1".into()),
        &nav::Filter::Connection("none".into()),
        &[],
    )
    .unwrap();
    assert_eq!(hidden.kicker, "WHY");
    for lang in crate::words::SHIPPED {
        let w = words(lang);
        face_lines(&w, &k, &Face::Overview, &all, &[]).unwrap();
        filter_choices(&w, &nav_knob()).unwrap();
    }
}

#[test]
fn the_menu_and_the_rim() {
    let w = words("en");
    let k = nav_knob();
    let choices = filter_choices(&w, &k).unwrap();
    // Results is left out of this dial's rings, so its stop there doesn't count.
    assert_eq!(choices[2].1, "Notion · 1 stop");
    assert_eq!(
        filter_name(&w, &k, &nav::Filter::Connection("g_log".into())).unwrap(),
        "Notion"
    );
    assert_eq!(
        filter_name(&w, &k, &nav::Filter::Connection("x".into())).unwrap(),
        "All services"
    );
    let arcs = rim_arcs(&Rim {
        done: 3,
        working: 1,
        retrying: 0,
        needs_you: 0,
        line: None,
    });
    assert_eq!(arcs.len(), 2);
    assert_eq!(arcs[1].2, "blue");
    assert!((arcs[0].1 - (0.75 - 1.2 / 360.0)).abs() < 1e-9);
    assert!(rim_arcs(&Rim::default()).is_empty());
}
