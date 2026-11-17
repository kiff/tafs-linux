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
