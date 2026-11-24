use super::*;
use serde_json::json;

fn call(outcomes: &[&str]) -> HeldCall {
    let w = crate::words::WordBook::load(&crate::words::WordBook::default_root(), "en").unwrap();
    let e: crate::held::CardException = serde_json::from_value(json!({
        "id": "exc_1", "subject": "agt_refunds", "action": "refund", "entity_id": "ch_1",
        "status": "held", "created_at": "2026-11-23T10:00:00Z", "expires_at": null, "accounting_version": 7,
        "breaches": outcomes.iter().map(|o| json!({"mandate_id": "m1", "outcome": o})).collect::<Vec<_>>(),
    }))
    .unwrap();
    crate::held::held(&w, &e, "taf-1").unwrap()
}

#[test]
fn offered_only_on_a_hold_a_card_change_can_lift() {
    assert!(allowable(&call(&["per_action_exceeded"])));
    assert!(allowable(&call(&["limit_reached", "resource_not_on_card"])));
    // A "never", or any outcome this app doesn't know, among them: no offer.
    assert!(!allowable(&call(&["per_action_exceeded", "never"])));
    assert!(!allowable(&call(&["unit_mismatch"])));
    assert!(!allowable(&call(&[])));
    // A sign-in fill, Coach's install, or a call not from KIFF: never.
    let mut c = call(&["per_action_exceeded"]);
    c.needs_release = true;
    assert!(!allowable(&c));
    let mut c = call(&["per_action_exceeded"]);
    c.action = "version_install".into();
    assert!(!allowable(&c));
    let mut c = call(&["per_action_exceeded"]);
    c.kiff_id = None;
    assert!(!allowable(&c));

    // Live only when KIFF names `holds.allow`; a Cards manager, or a KIFF that sends no list.
    let caps = |c: &[&str]| c.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    let both = caps(&["holds.allow", "cards.manage"]);
    let c = call(&["per_action_exceeded"]);
    assert!(offers(true, Some(&both), true, &c));
    assert!(!offers(true, None, true, &c));
    assert!(!offers(true, Some(&caps(&["cards.manage"])), true, &c));
    assert!(!offers(true, Some(&caps(&["holds.allow"])), true, &c));
    assert!(!offers(false, Some(&both), true, &c));
    assert!(!offers(true, Some(&both), false, &c));
    assert!(manages_cards(None));
}

#[test]
fn tafs_clouds_answer_read_strictly() {
    assert_eq!(
        offer_path("t 1", "exc/1"),
        "/v1/tafs/t%201/holds/exc%2F1/allow"
    );
    assert_eq!(offer_body(false), b"{}");
    assert_eq!(offer_body(true), br#"{"round":true}"#);
    let a = Answer::read(
        json!({"offer": {
            "changes": [{"mandate_id": "m1", "kind": "per_action", "before": 15000, "after": 32000,
                "unit": "EUR", "words_before": "Refunds up to €150 each", "words_after": "Refunds up to €320 each",
                "rounded": false}],
            "round": {"after": 35000, "words": "€350"},
            "would": {"calls": 4, "words": "With this Card, 4 more refunds would have gone out on their own (€1,180)"},
            "draft": "drf_1", "confirm_key": "critical", "send_held": true}})
        .to_string()
        .as_bytes(),
    )
    .unwrap();
    let o = a.offer.unwrap();
    assert_eq!(o.draft, "drf_1");
    assert_eq!(o.round.as_deref(), Some("€350"));
    assert_eq!(o.changes[0].after, Some(32000));
    assert_eq!(o.send_held, Some(true));
    let none =
        Answer::read(br#"{"none":"no_change","words":"Nothing narrow to change."}"#).unwrap();
    assert!(none.offer.is_none());
    assert_eq!(none.words.as_deref(), Some("Nothing narrow to change."));
    // A field of another shape fails the read; so does an offer with no draft.
    assert_eq!(Answer::read(br#"{"none":3}"#), None);
    assert_eq!(Answer::read(br#"{"offer":{"changes":[],"draft":1}}"#), None);
    assert_eq!(Answer::read(br#"{"offer":{"changes":[]}}"#), None);
}

fn per_action_offer(after: i64) -> Offer {
    Offer {
        changes: vec![Change {
            mandate_id: "m1".into(),
            kind: "per_action".into(),
            before: Some(15000),
            after: Some(after),
            unit: Some("EUR".into()),
            window: None,
            parameter: None,
            value: None,
            words_before: None,
            words_after: None,
            rounded: None,
        }],
        would: None,
        draft: "drf_1".into(),
        send_held: Some(true),
        round: None,
    }
}

/// The Card in force: refunds up to €150 each, €600 a day, on customers c1 and c2.
fn current() -> serde_json::Value {
    json!({
        "grants": [
            {"action": "refund", "per_action_max": [{"quantity": "sum(amount)", "max": 15000, "unit": "EUR", "scale": 2}],
             "resources": {"parameter": "customer", "values": ["c2", "c1"]}},
            {"action": "lookup"}
        ],
        "aggregates": [{"quantity": "sum(amount)", "limit": 60000, "window": "day", "unit": "EUR", "scale": 2}],
        "on_exceed": "owner", "subject": "agt_refunds", "domain": "Payments",
        "valid_from": "2026-11-01T00:00:00Z"
    })
}

/// KIFF's draft: one Card revising `m1`, built from the Card in force with `edit` applied.
fn draft_with(
    edit: impl FnOnce(&mut serde_json::Value),
    top: impl FnOnce(&mut serde_json::Value),
) -> AgentDraft {
    let mut card = current();
    edit(&mut card);
    card["revises"] = json!("m1");
    card["current"] = current();
    let mut d = json!({"id": "drf_1", "agent_id": "agt_refunds", "status": "draft", "version": "v2",
        "from_hold": "exc_1", "cards": [card]});
    top(&mut d);
    AgentDraft::read(d.to_string().as_bytes()).expect("a draft")
}

fn cap(card: &mut serde_json::Value, max: i64) {
    card["grants"][0]["per_action_max"][0]["max"] = json!(max);
}

fn ok(d: &AgentDraft, o: &Offer) -> bool {
    matches(d, o, "agt_refunds", "exc_1", "refund")
}

#[test]
fn a_draft_is_signed_only_when_its_exactly_what_was_offered() {
    let o = per_action_offer(32000);
    // The one offered change: the refund cap from €150 to €320. Written in another order, with
    // the same instant at another offset: still the same terms.
    let d = draft_with(
        |c| {
            cap(c, 32000);
            c["grants"][0]["resources"]["values"] = json!(["c1", "c2"]);
            c["valid_from"] = json!("2026-11-01T01:00:00+01:00");
            c["domain"] = json!("payments");
        },
        |_| {},
    );
    assert!(ok(&d, &o));

    // Not the figure offered, not the agent, not the held call, or not the action held.
    assert!(!ok(&draft_with(|c| cap(c, 35000), |_| {}), &o));
    assert!(!matches(&d, &o, "agt_other", "exc_1", "refund"));
    assert!(!matches(&d, &o, "", "exc_1", "refund"));
    assert!(!matches(&d, &o, "agt_refunds", "exc_9", "refund"));
    assert!(!matches(&d, &o, "agt_refunds", "exc_1", "lookup"));
    // A draft that doesn't say which hold it's from is still checked on everything else.
    assert!(ok(
        &draft_with(
            |c| cap(c, 32000),
            |d| {
                d.as_object_mut().unwrap().remove("from_hold");
            }
        ),
        &o
    ));

    // Anything else changed too: a total, a scope, a state, a next step, the policy, a holder,
    // the domain, a date, another grant, a cap's scale.
    for edit in [
        (|c: &mut serde_json::Value| c["aggregates"][0]["limit"] = json!(90000))
            as fn(&mut serde_json::Value),
        |c| c["grants"][0]["resources"]["values"] = json!(["c1", "c2", "c3"]),
        |c| c["grants"][0]["entity_states"] = json!(["open"]),
        |c| c["grants"][0]["next_steps"] = json!(["notify"]),
        |c| c["on_exceed"] = json!("refuse"),
        |c| c["subject"] = json!("agt_other"),
        |c| c["domain"] = json!("Other"),
        |c| c["valid_until"] = json!("2027-01-01T00:00:00Z"),
        |c| c["grants"][1]["per_action_max"] = json!([{"quantity": "count", "max": 5}]),
        |c| c["grants"][0]["per_action_max"][0]["scale"] = json!(0),
    ] {
        let d = draft_with(
            |c| {
                cap(c, 32000);
                edit(c);
            },
            |_| {},
        );
        assert!(!ok(&d, &o), "{:?}", d.cards);
    }

    // A draft that retires a Card, sets a hold's wait, revokes, or revises nothing named.
    assert!(!ok(
        &draft_with(|c| cap(c, 32000), |d| d["retires"] = json!([{"id": "m9"}])),
        &o
    ));
    assert!(!ok(
        &draft_with(
            |c| {
                cap(c, 32000);
                c["hold_expiry"] = json!("PT1H");
            },
            |_| {}
        ),
        &o
    ));
    assert!(!ok(
        &draft_with(
            |c| {
                cap(c, 32000);
                c["revoked_at"] = json!("2026-11-23T10:00:00Z");
            },
            |_| {}
        ),
        &o
    ));
    assert!(!ok(
        &draft_with(
            |c| cap(c, 32000),
            |d| d["cards"][0]["revises"] = json!("m2")
        ),
        &o
    ));
    // A Card that revises nothing in force (a new Card), or the offer's Card left unchanged.
    let mut new_card = draft_with(|c| cap(c, 32000), |_| {});
    new_card.cards.as_mut().unwrap()[0].revises = None;
    assert!(!ok(&new_card, &o));
    assert!(!ok(&draft_with(|_| {}, |_| {}), &o));
    // No Cards at all, or an offer that names none.
    let mut empty = draft_with(|c| cap(c, 32000), |_| {});
    empty.cards = None;
    assert!(!ok(&empty, &o));
    let mut nothing = per_action_offer(32000);
    nothing.changes.clear();
    assert!(!ok(&draft_with(|c| cap(c, 32000), |_| {}), &nothing));
    // A kind the contract doesn't name applies in no way.
    let mut unknown = per_action_offer(32000);
    unknown.changes[0].kind = "widen".into();
    assert!(!ok(&draft_with(|c| cap(c, 32000), |_| {}), &unknown));
    // The same Card twice.
    let twice = draft_with(
        |c| cap(c, 32000),
        |d| {
            let card = d["cards"][0].clone();
            d["cards"].as_array_mut().unwrap().push(card);
        },
    );
    assert!(!ok(&twice, &o));
}

#[test]
fn a_card_marked_unchanged_must_read_as_in_force() {
    let o = per_action_offer(32000);
    let with_unchanged = |edit: fn(&mut serde_json::Value)| {
        draft_with(
            |c| cap(c, 32000),
            |d| {
                let mut other = current();
                edit(&mut other);
                other["revises"] = json!("m7");
                other["current"] = current();
                other["unchanged"] = json!(true);
                d["cards"].as_array_mut().unwrap().push(other);
            },
        )
    };
    assert!(ok(&with_unchanged(|_| {}), &o));
    assert!(!ok(
        &with_unchanged(|c| c["on_exceed"] = json!("refuse")),
        &o
    ));
}

#[test]
fn a_total_a_count_and_a_scope_each_move_as_offered() {
    let change = |kind: &str, before: Option<i64>, after: Option<i64>| Change {
        mandate_id: "m1".into(),
        kind: kind.into(),
        before,
        after,
        unit: Some("EUR".into()),
        window: Some("day".into()),
        parameter: None,
        value: None,
        words_before: None,
        words_after: None,
        rounded: None,
    };
    let offer = |c: Change| Offer {
        changes: vec![c],
        ..per_action_offer(0)
    };
    // A day's total from €600 to €750, and not in another window.
    let total = offer(change("total", Some(60000), Some(75000)));
    assert!(ok(
        &draft_with(|c| c["aggregates"][0]["limit"] = json!(75000), |_| {}),
        &total
    ));
    let mut weekly = total.clone();
    weekly.changes[0].window = Some("week".into());
    assert!(!ok(
        &draft_with(|c| c["aggregates"][0]["limit"] = json!(75000), |_| {}),
        &weekly
    ));
    // A count: "each call asks you first" (0 a day) becomes 1 a day.
    let mut count = change("count", Some(0), Some(1));
    count.unit = None;
    let counted = |c: &mut serde_json::Value| {
        c["aggregates"] = json!([{"quantity": "count", "limit": 0, "window": "day"}]);
    };
    let d = draft_with(
        |c| {
            counted(c);
            c["aggregates"][0]["limit"] = json!(1);
        },
        |d| counted(&mut d["cards"][0]["current"]),
    );
    assert!(ok(&d, &offer(count)));
    // A scope: exactly the one value added, on the held action's parameter.
    let mut scope = change("resource", None, None);
    scope.parameter = Some("customer".into());
    scope.value = Some("c3".into());
    let scope = offer(scope);
    assert!(ok(
        &draft_with(
            |c| c["grants"][0]["resources"]["values"] = json!(["c1", "c2", "c3"]),
            |_| {}
        ),
        &scope
    ));
    assert!(!ok(
        &draft_with(
            |c| c["grants"][0]["resources"]["values"] = json!(["c1", "c2", "c3", "c4"]),
            |_| {}
        ),
        &scope
    ));
    // Both of a hold's breaches in one draft: the cap and the total.
    let mut both = per_action_offer(32000);
    both.changes.push(change("total", Some(60000), Some(75000)));
    let d = draft_with(
        |c| {
            cap(c, 32000);
            c["aggregates"][0]["limit"] = json!(75000);
        },
        |_| {},
    );
    assert!(ok(&d, &both));
    assert!(!ok(&draft_with(|c| cap(c, 32000), |_| {}), &both));
}

#[test]
fn a_draft_naming_only_the_same_first_holder_keeps_the_cards_holders() {
    let o = per_action_offer(32000);
    let shared = |c: &mut serde_json::Value| c["subjects"] = json!(["agt_refunds", "agt_second"]);
    // The Card in force is held by two; the draft names only the first: the same holders.
    let d = draft_with(
        |c| {
            cap(c, 32000);
            c.as_object_mut().unwrap().remove("subjects");
        },
        |d| shared(&mut d["cards"][0]["current"]),
    );
    assert!(ok(&d, &o));
    // Naming another first holder is a change.
    let d = draft_with(
        |c| {
            cap(c, 32000);
            c["subject"] = json!("agt_second");
        },
        |d| shared(&mut d["cards"][0]["current"]),
    );
    assert!(!ok(&d, &o));
}
