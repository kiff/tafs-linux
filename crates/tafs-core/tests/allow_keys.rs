//! "Allow this from now on", its second half (kiff/tafs `docs/allow-from-now-on.md`): a draft Card
//! key the check doesn't read is never signed. The shared vector (`vectors/agent-draft-keys.json`,
//! which the iPad's, Android's and tafs cloud's tests read too), over KIFF's own draft pinned in
//! `contracts/kiff-agent-draft-from-hold.json`.

use serde_json::Value;
use tafs_core::allow::{self, Answer};
use tafs_core::drafts::AgentDraft;

fn shared(path: &str) -> Value {
    let full = format!("{}/../../shared/{path}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_slice(&std::fs::read(&full).unwrap_or_else(|e| panic!("{full}: {e}"))).unwrap()
}

/// One RFC 6901 pointer's value set in a JSON document, its last key made when it isn't there.
fn set(doc: &mut Value, path: &[&str], value: Value) {
    let Some((key, rest)) = path.split_first() else {
        *doc = value;
        return;
    };
    let next = match doc {
        Value::Array(list) => &mut list[key.parse::<usize>().unwrap()],
        _ => {
            if !doc.is_object() {
                *doc = Value::Object(Default::default());
            }
            doc.as_object_mut()
                .unwrap()
                .entry(key.to_string())
                .or_insert(Value::Null)
        }
    };
    set(next, rest, value);
}

#[test]
fn a_draft_card_key_the_check_doesnt_read_is_never_signed() {
    let vector = shared("vectors/agent-draft-keys.json");
    // This computer's keys are the vector's, object by object.
    let keys = vector["keys"].as_object().unwrap();
    assert_eq!(keys.len(), allow::KEYS.len());
    for (kind, list) in allow::KEYS {
        let mut ours: Vec<&str> = list.to_vec();
        let mut theirs: Vec<&str> = keys[*kind]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k.as_str().unwrap())
            .collect();
        ours.sort_unstable();
        theirs.sort_unstable();
        assert_eq!(ours, theirs, "{kind}");
    }
    let contract = shared(vector["contract"].as_str().unwrap());
    let cases = vector["cases"].as_array().unwrap();
    assert!(cases.len() > 10);
    for c in cases {
        let mut draft = contract["answer"].clone();
        for (pointer, value) in c["set"].as_object().unwrap() {
            let path: Vec<&str> = pointer.split('/').skip(1).collect();
            set(&mut draft, &path, value.clone());
        }
        let body = serde_json::to_vec(&draft).unwrap();
        assert_eq!(
            allow::keys_known(&body),
            c["ok"].as_bool().unwrap(),
            "{}",
            c["name"]
        );
    }
}

#[test]
fn kiffs_pinned_draft_is_what_its_offer_says_keys_and_all() {
    let body =
        serde_json::to_vec(&shared("contracts/kiff-agent-draft-from-hold.json")["answer"]).unwrap();
    let draft = AgentDraft::read(&body).expect("KIFF's draft reads");
    let offered = r#"{"offer":{"changes":[{"mandate_id":"rae-refund","kind":"per_action","before":15000,"after":32000,"unit":"EUR","words_before":"up to €150 each","words_after":"up to €320 each","rounded":false}],"draft":"drf_fixed","confirm_key":"critical","send_held":true}}"#;
    let offer = Answer::read(offered.as_bytes())
        .and_then(|a| a.offer)
        .expect("the offer reads");
    assert!(allow::keys_known(&body));
    assert!(allow::signable(
        &body,
        &draft,
        &offer,
        "rae",
        "exc_fixed",
        "refund"
    ));
    // The same draft with a key KIFF adds later on its Card: what was offered, but never signed.
    let mut later = shared("contracts/kiff-agent-draft-from-hold.json")["answer"].clone();
    later["cards"][0]["room_grants"] = serde_json::json!(["rae"]);
    let later = serde_json::to_vec(&later).unwrap();
    let read = AgentDraft::read(&later).unwrap();
    assert!(allow::matches(&read, &offer, "rae", "exc_fixed", "refund"));
    assert!(!allow::signable(
        &later,
        &read,
        &offer,
        "rae",
        "exc_fixed",
        "refund"
    ));
}
