use super::*;
use p256::ecdsa::signature::Verifier;
use p256::pkcs8::DecodePublicKey;

const T: &str = "t_acme";

fn card_ask(draft: &str, version: &str, message: String) -> Ask {
    Ask {
        id: "ask_1".into(),
        step: "cards".into(),
        message,
        reason: "Approve the Card".into(),
        kiff: Some(AskDraft {
            draft: draft.into(),
            version: version.into(),
        }),
    }
}

#[test]
fn only_the_cards_step_for_the_drawn_draft_and_version_is_signed_here() {
    let good = card_ask("drf_1", "v7", cards_message(T, "drf_1", "v7"));
    assert!(signs_here(&good, "drf_1", "v7", T));
    // The connection's step, whatever it says, is the phone's.
    let connect = Ask {
        step: "connect".into(),
        ..good.clone()
    };
    assert!(!signs_here(&connect, "drf_1", "v7", T));
    // Another draft, another version, another account, or a message this computer didn't build.
    for (ask, draft, shown, tenant) in [
        (good.clone(), "drf_2", "v7", T),
        (good.clone(), "drf_1", "v8", T),
        (good.clone(), "drf_1", "v7", "t_other"),
        (good.clone(), "drf_1", "v7", ""),
        (
            card_ask("drf_1", "v7", cards_message(T, "drf_1", "v8")),
            "drf_1",
            "v7",
            T,
        ),
        (
            card_ask(
                "drf_1",
                "v7",
                format!("{}|x", cards_message(T, "drf_1", "v7")),
            ),
            "drf_1",
            "v7",
            T,
        ),
        (
            card_ask("drf_9", "v7", cards_message(T, "drf_1", "v7")),
            "drf_1",
            "v7",
            T,
        ),
        (
            card_ask("drf_1", "v9", cards_message(T, "drf_1", "v7")),
            "drf_1",
            "v7",
            T,
        ),
        (
            Ask {
                kiff: None,
                ..good.clone()
            },
            "drf_1",
            "v7",
            T,
        ),
    ] {
        assert!(
            !signs_here(&ask, draft, shown, tenant),
            "{ask:?} {draft} {shown} {tenant}"
        );
    }
    assert_eq!(
        cards_message(T, "drf_1", "v7"),
        "tafs.cards.v1|t_acme|drf_1|v7"
    );
}

#[test]
fn approve_is_offered_only_once_the_connection_is_made_and_the_card_is_next() {
    assert!(offered(true, true, Some("drf_1"), Some("v7")));
    for (connected, next, draft, shown) in [
        (false, true, Some("drf_1"), Some("v7")),
        (true, false, Some("drf_1"), Some("v7")),
        (true, true, None, Some("v7")),
        (true, true, Some("drf_1"), None),
        (true, true, Some(""), Some("v7")),
        (true, true, Some("drf_1"), Some("")),
    ] {
        assert!(!offered(connected, next, draft, shown));
    }
}

#[test]
fn tafs_clouds_answers_are_read_leniently_and_never_as_a_signature() {
    let read = |s: &str| Decision::read(s.as_bytes());
    assert_eq!(
        read(r#"{"outcome":"answered","answer":"approved","note":"Done"}"#),
        Decision::Answered {
            note: Some("Done".into())
        }
    );
    assert_eq!(
        read(r#"{"outcome":"read_again"}"#),
        Decision::ReadAgain { note: None }
    );
    let sign = read(
        r#"{"outcome":"sign","session":"s_1","ask":{"id":"a1","step":"cards","message":"tafs.cards.v1|t_acme|d|v","reason":"Approve","kiff":{"draft":"d","version":"v"}}}"#,
    );
    let Decision::Sign { ask, session } = sign else {
        panic!("{sign:?}");
    };
    assert_eq!(session.as_deref(), Some("s_1"));
    assert!(signs_here(&ask, "d", "v", T));
    // A connection's ask reads, and is then refused by `signs_here`.
    assert!(matches!(
        read(r#"{"outcome":"sign","ask":{"id":"a1","step":"connect","message":"m","reason":"r"}}"#),
        Decision::Sign { .. }
    ));
    // Anything unreadable signs nothing.
    for bad in [
        "",
        "[]",
        r#"{"outcome":"sign"}"#,
        r#"{"outcome":"sign","ask":{"step":"cards","message":"m"}}"#,
        r#"{"outcome":"sign","ask":{"id":"a","step":"cards","message":"m","kiff":{"draft":"d"}}}"#,
        r#"{"outcome":"sign","ask":{"id":"a","step":"cards","message":"m","kiff":"d"}}"#,
        r#"{"outcome":"sign_in"}"#,
        r#"{"outcome":"later"}"#,
    ] {
        assert_eq!(read(bad), Decision::Unknown, "{bad}");
    }
}

#[test]
fn the_ask_and_the_report_go_where_tafs_cloud_takes_them() {
    assert_eq!(
        ask_path("taf 1", "c/2"),
        "/v1/tafs/taf%201/components/c%2F2/approval"
    );
    assert_eq!(
        report_path("taf1", "c2", "ask 3"),
        "/v1/tafs/taf1/components/c2/approval/ask%203"
    );
    let body: Value = serde_json::from_slice(&ask_body("g1", "drf_1", "v7", Some("s_1"))).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"grant":"g1","draft":"drf_1","shown_draft":"v7","session":"s_1"})
    );
    let body: Value = serde_json::from_slice(&ask_body("g1", "drf_1", "v7", None)).unwrap();
    assert!(body.get("session").is_none());
    // What came of it, never an answer: confirmed, or failed with why (trimmed, at most 500).
    assert_eq!(
        report_body(&Ok(()), Some("s_1")),
        br#"{"result":"confirmed","session":"s_1"}"#
    );
    assert_eq!(
        report_body(&Err("  KIFF said no.  ".into()), None),
        br#"{"result":"failed","why":"KIFF said no."}"#
    );
    let long: Value = serde_json::from_slice(&report_body(&Err("x".repeat(900)), None)).unwrap();
    assert_eq!(long["why"].as_str().unwrap().len(), 500);
}

#[test]
fn the_report_is_signed_over_exactly_the_bytes_sent() {
    let key = DeviceKey::generate();
    let path = report_path("taf1", "c2", "ask3");
    let body = report_body(&Ok(()), Some("s_1"));
    let at = "2026-11-24T05:00:00.000Z";
    let header = report_signature(&key, T, &path, &body, at);
    let (t, v1) = header.split_once(",v1=").unwrap();
    assert_eq!(t, format!("t={at}"));
    use base64::Engine;
    let sig = base64::engine::general_purpose::STANDARD
        .decode(v1)
        .unwrap();
    let sig = p256::ecdsa::Signature::from_der(&sig).unwrap();
    let vk = p256::ecdsa::VerifyingKey::from_public_key_der(&key.public_spki_der()).unwrap();
    // KIFF's `RequestMessage` for this very request.
    let message = crate::passkey::request_v1(T, "POST", &path, &body, at);
    assert!(vk.verify(&message, &sig).is_ok());
    // Another body, path or time doesn't hold.
    let other = report_body(&Err("no".into()), Some("s_1"));
    for m in [
        crate::passkey::request_v1(T, "POST", &path, &other, at),
        crate::passkey::request_v1(T, "POST", &report_path("taf1", "c2", "ask4"), &body, at),
        crate::passkey::request_v1(T, "POST", &path, &body, "2026-11-24T05:00:00.001Z"),
    ] {
        assert!(vk.verify(&m, &sig).is_err());
    }
}
