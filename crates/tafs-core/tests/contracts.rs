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
