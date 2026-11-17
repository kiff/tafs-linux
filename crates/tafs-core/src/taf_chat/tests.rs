use super::*;
use crate::coach_send::{message_body, sent, steer_body, steered};

fn en() -> WordBook {
    WordBook::load(&WordBook::default_root(), "en").unwrap()
}

fn at(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

fn fixture(name: &str) -> Vec<u8> {
    let path = format!("{}/../tafs-app/fixtures/{name}", env!("CARGO_MANIFEST_DIR"));
    let file: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    serde_json::to_vec(&file["answer"]).unwrap()
}

fn thread(json: &str) -> Option<TafThread> {
    TafThread::read(json.as_bytes())
}

fn owner(text: &str, at_: &str, session: &str) -> String {
    format!(
        r#"{{"at":"{at_}","from":"owner","kind":"message","text":"{text}","session":"{session}"}}"#
    )
}

fn taf(text: &str, at_: &str, session: &str) -> String {
    format!(
        r#"{{"at":"{at_}","from":"taf","kind":"message","text":"{text}","session":"{session}"}}"#
    )
}

fn chat_of(json: &str) -> TafChat {
    let mut c = TafChat::new("taf-1");
    c.took(&en(), thread(json).unwrap(), at("2026-11-16T10:00:00Z"));
    c
}

#[test]
fn the_conversation_is_the_live_chat_s() {
    assert_eq!(conversation_path("taf 1"), "/v1/tafs/taf%201/conversation");
    // A taf's message goes into its live chat: no session, no new chat.
    let body: Value =
        serde_json::from_slice(&message_body("Hi", &ChatTarget::default(), &[])).unwrap();
    assert_eq!(body, serde_json::json!({"text": "Hi"}));
    // Files go with it by their ids (`MessageBody.attachments`), none when there are none.
    let file = crate::coach::Attachment {
        id: "att_1".into(),
        kind: "image".into(),
        media_type: "image/jpeg".into(),
        name: "receipt.jpg".into(),
        size: 10,
    };
    let body: Value =
        serde_json::from_slice(&message_body("", &ChatTarget::default(), &[file])).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"text": "", "attachments": ["att_1"]})
    );
}

#[test]
fn the_fixture_reads_as_the_ipad_reads_it() {
    let t = TafThread::read(&fixture("taf-chat.json")).unwrap();
    // A message sent with a file: the file goes with it, by its id, name and kind.
    let sent = t
        .messages
        .iter()
        .find(|m| !m.attachments.is_empty())
        .unwrap();
    assert_eq!(sent.attachments[0].id, "att_label");
    assert_eq!(sent.attachments[0].media_type, "image/jpeg");
    assert_eq!(t.messages.len(), 8);
    assert_eq!(t.working, None);
    assert_eq!(
        t.capped,
        Some(Capped {
            run: "run7".into(),
            stop: "cost_cap".into()
        })
    );
    // Only the ask_coach card: a taf's other cards aren't drawn.
    assert_eq!(t.ask_coach.len(), 1);
    assert_eq!(t.ask_coach[0].id, "raise-limit");
    assert_eq!(t.ask_coach[0].session, "sesn_9");
    let s = t.steering.unwrap();
    assert_eq!(s.not_sent.len(), 1);
    assert!(s.over());
    let w = TafThread::read(&fixture("taf-chat-working.json")).unwrap();
    assert_eq!(w.working.as_deref(), Some("sesn_9"));
    assert_eq!(w.working_run.as_deref(), Some("run8"));
    assert_eq!(
        w.steering.unwrap().held.unwrap().text,
        "Then tell me the total"
    );
}

#[test]
fn the_messages_and_working_are_strict() {
    let m = owner("Hi", "2026-11-16T09:00:00Z", "s");
    assert!(thread(&format!(r#"{{"messages":[{m}]}}"#)).is_some());
    // No messages, or one it can't read: no thread.
    assert!(thread("{}").is_none());
    assert!(
        thread(r#"{"messages":[{"at":"2026-11-16T09:00:00Z","from":"owner","text":"Hi"}]}"#)
            .is_none()
    );
    // `working` is read whole when it's there.
    assert!(thread(&format!(
        r#"{{"messages":[{m}],"working":{{"session":1}}}}"#
    ))
    .is_none());
    assert!(thread(&format!(
        r#"{{"messages":[{m}],"working":{{"session":"s","run":2}}}}"#
    ))
    .is_none());
    assert!(thread(&format!(r#"{{"messages":[{m}],"working":null}}"#))
        .unwrap()
        .working
        .is_none());
    let w = thread(&format!(
        r#"{{"messages":[{m}],"working":{{"session":"s"}}}}"#
    ))
    .unwrap();
    assert_eq!((w.working.as_deref(), w.working_run), (Some("s"), None));
}

#[test]
fn the_steering_the_stop_and_the_cards_are_lenient() {
    let m = owner("Hi", "2026-11-16T09:00:00Z", "s");
    // A steering or a stop it can't read is none, never the thread.
    let t = thread(&format!(
        r#"{{"messages":[{m}],"steering":{{"run":"r"}},"stopped":{{"run":"r"}},"capped":7}}"#
    ))
    .unwrap();
    assert_eq!((t.steering, t.capped), (None, None));
    // `stopped` names any stop; `capped` stands in for it from an older tafs cloud.
    let t = thread(&format!(
        r#"{{"messages":[{m}],"stopped":{{"run":"r","stop":"max_token"}},"capped":{{"run":"r","stop":"cost_cap"}}}}"#
    ))
    .unwrap();
    assert_eq!(t.capped.unwrap().stop, "max_token");
    let t = thread(&format!(
        r#"{{"messages":[{m}],"stopped":{{"run":"r"}},"capped":{{"run":"r","stop":"turn_cap"}}}}"#
    ))
    .unwrap();
    assert_eq!(t.capped.unwrap().stop, "turn_cap");
    // Each card leniently: another kind, blank words or a time it can't read leave it out.
    let card = |kind: &str, text: &str, at_: &str| {
        format!(
            r#"{{"at":"{at_}","session":"s","component":{{"type":"{kind}","id":"c","text":"{text}"}}}}"#
        )
    };
    let t = thread(&format!(
        r#"{{"messages":[{m}],"components":[{},{},{},{},{{"at":"2026-11-16T09:00:00Z"}}]}}"#,
        card("ask_coach", "Raise it", "2026-11-16T09:00:01Z"),
        card("ask_coach", "  ", "2026-11-16T09:00:01Z"),
        card("confirm", "Go?", "2026-11-16T09:00:01Z"),
        card("ask_coach", "Raise it", "yesterday"),
    ))
    .unwrap();
    assert_eq!(t.ask_coach.len(), 1);
    assert_eq!(t.ask_coach[0].text, "Raise it");
    // A card with no session is the thread's own; one whose session isn't words, none.
    let t = thread(&format!(
        r#"{{"messages":[{m}],"components":[{{"at":"2026-11-16T09:00:01Z","component":{{"type":"ask_coach","id":"c","text":"x"}}}},{{"at":"2026-11-16T09:00:01Z","session":3,"component":{{"type":"ask_coach","id":"d","text":"x"}}}}]}}"#
    ))
    .unwrap();
    assert_eq!(t.ask_coach.len(), 1);
    assert_eq!(t.ask_coach[0].session, "");
    // `components` itself must be a list.
    assert!(thread(&format!(r#"{{"messages":[{m}],"components":{{}}}}"#)).is_none());
}

#[test]
fn a_cap_shows_only_once_nothing_is_at_work_and_a_stop_is_over_at_once() {
    let w = en();
    let mut c = chat_of(std::str::from_utf8(&fixture("taf-chat.json")).unwrap());
    assert_eq!(c.capped.as_deref(), Some("cost_cap"));
    let lines = lines(
        &w,
        &c,
        &Whose {
            name: Some("Rae"),
            ask_coach: false,
        },
        &[],
        at("2026-11-16T10:00:00Z"),
    )
    .unwrap();
    assert!(lines
        .capped
        .unwrap()
        .starts_with("Rae stopped: this task would cost more"));
    // At work: no cap.
    c.took(
        &w,
        TafThread::read(&fixture("taf-chat-working.json")).unwrap(),
        at("2026-11-16T10:03:00Z"),
    );
    assert_eq!(c.capped, None);
    assert!(c.thinking());
    // Stopped here: over at once, though tafs cloud still says it works.
    assert_eq!(c.stop().as_deref(), Some("sesn_9"));
    c.stopped("sesn_9");
    assert!(!c.thinking());
    assert_eq!(c.ended, Some(Ended::Stopped));
    // Once tafs cloud no longer has it at work, the stop is done with.
    let mut idle = TafThread::read(&fixture("taf-chat-working.json")).unwrap();
    idle.working = None;
    c.took(&w, idle, at("2026-11-16T10:04:00Z"));
    assert_eq!(c.stopped, None);
}

#[test]
fn stop_takes_the_session_at_work_else_the_thread_s_latest() {
    let mut c = chat_of(&format!(
        r#"{{"messages":[{},{}]}}"#,
        owner("Hi", "2026-11-16T09:00:00Z", "s1"),
        taf("Hello", "2026-11-16T09:00:05Z", "s2")
    ));
    c.awaiting = Some(at("2026-11-16T09:59:00Z"));
    assert_eq!(c.stop().as_deref(), Some("s2"));
    assert_eq!(c.awaiting, None);
    assert_eq!(TafChat::default().stop(), None);
}

#[test]
fn what_a_run_never_read_shows_once_as_not_sent() {
    let w = en();
    let mut c = chat_of(std::str::from_utf8(&fixture("taf-chat.json")).unwrap());
    assert_eq!(c.unsent.len(), 1);
    assert_eq!(c.unsent[0].text, "And skip 4412 too");
    assert_eq!(c.unsent[0].why, "You stopped it before it read this.");
    // Read again: not again.
    c.took(
        &w,
        TafThread::read(&fixture("taf-chat.json")).unwrap(),
        at("2026-11-16T10:01:00Z"),
    );
    assert_eq!(c.unsent.len(), 1);
    // What a run that's over listed unread, in the app's words: stopped, or ended.
    let m = owner("Go", "2026-11-16T09:00:00Z", "s");
    let unread = |status: &str| {
        chat_of(&format!(
            r#"{{"messages":[{m}],"steering":{{"run":"r","status":"{status}","untaken":[{{"text":"and more","at":"2026-11-16T09:00:01Z"}}]}}}}"#
        ))
        .unwrap_why()
    };
    assert_eq!(
        unread("CANCELLED"),
        vec!["You stopped it before it read this."]
    );
    assert_eq!(
        unread("COMPLETED"),
        vec!["It wasn't read before the work ended."]
    );
    assert!(unread("RUNNING").is_empty());
}

trait Whys {
    fn unwrap_why(&self) -> Vec<String>;
}

impl Whys for TafChat {
    fn unwrap_why(&self) -> Vec<String> {
        self.unsent.iter().map(|u| u.why.clone()).collect()
    }
}

#[test]
fn send_goes_now_or_to_the_run_at_work() {
    let w = en();
    let now = at("2026-11-16T10:05:00Z");
    let mut c = chat_of(std::str::from_utf8(&fixture("taf-chat.json")).unwrap());
    assert_eq!(c.submit(&w, "   ", now).unwrap(), Submit::Stay);
    // Too long: it stays in the box, with why.
    let long = "a".repeat(CHAT_LIMIT + 1);
    assert_eq!(c.submit(&w, &long, now).unwrap(), Submit::Stay);
    assert_eq!(
        c.error.as_deref(),
        Some("That's longer than 2000 characters. Say it in fewer words.")
    );
    let Submit::Send(d) = c.submit(&w, " Refund 4410 ", now).unwrap() else {
        panic!("a message")
    };
    assert_eq!(d.text, "Refund 4410");
    assert_eq!(d.target, ChatTarget::default());
    assert_eq!(c.error, None);
    // Shown at once, after the thread's latest, faded until tafs cloud's copy comes.
    let mine = c.thread.messages.last().unwrap();
    assert_eq!((mine.session.as_str(), mine.at), (PENDING, now));
    // One at a time: while it goes, words go to the run, and with no run yet they stay.
    assert!(c.sending);
    assert_eq!(c.submit(&w, "more", now).unwrap(), Submit::Stay);
    c.delivered(
        &d,
        &sent(br#"{"session":"sesn_9","new_session":false,"message":{"id":"run9.in"}}"#).unwrap(),
        now,
    );
    assert_eq!(c.awaiting, Some(now));
    assert!(c.thinking());
    // tafs cloud's copy takes its place by its id, whatever its time.
    let mut t = TafThread::read(&fixture("taf-chat.json")).unwrap();
    t.messages.push(ChatMessage {
        at: at("2026-11-16T10:04:59Z"),
        server_id: Some("run9.in".into()),
        session: "sesn_9".into(),
        ..mine_like("Refund 4410")
    });
    t.working = Some("sesn_9".into());
    t.working_run = Some("run9".into());
    c.took(&w, t, now);
    assert_eq!(
        c.thread
            .messages
            .iter()
            .filter(|m| m.text == "Refund 4410")
            .count(),
        1
    );
    // At work: Send writes to its run.
    assert_eq!(
        c.submit(&w, "and 4411", now).unwrap(),
        Submit::Steer {
            run: "run9".into(),
            text: "and 4411".into()
        }
    );
}

fn mine_like(text: &str) -> ChatMessage {
    ChatMessage {
        at: at("2026-11-16T10:00:00Z"),
        from: From::Owner,
        kind: MessageKind::Message,
        text: text.into(),
        session: PENDING.into(),
        door: None,
        work: None,
        attachments: vec![],
        server_id: None,
        answers: None,
        delivered: None,
        counted: None,
        refused: None,
    }
}

#[test]
fn words_written_to_the_run_show_at_once() {
    let w = en();
    let now = at("2026-11-16T10:05:00Z");
    let mut c = chat_of(std::str::from_utf8(&fixture("taf-chat-working.json")).unwrap());
    let body: Value = serde_json::from_slice(&steer_body("run8", "and 4411")).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"run": "run8", "text": "and 4411", "hold": true})
    );
    c.steered(
        "run8",
        "and 4411",
        &steered(br#"{"status":"RUNNING"}"#).unwrap(),
        now,
    );
    let l = lines(
        &w,
        &c,
        &Whose {
            name: Some("Rae"),
            ask_coach: false,
        },
        &[],
        now,
    )
    .unwrap();
    assert_eq!(
        l.steering,
        vec![
            (
                "Leave out orders from Spain".to_string(),
                "Rae reads this at its next step".to_string()
            ),
            (
                "and 4411".to_string(),
                "Rae reads this at its next step".to_string()
            ),
        ]
    );
    assert_eq!(
        l.queued,
        Some((
            "Then tell me the total".into(),
            "Waits until it's done, then goes as your next message".into()
        ))
    );
    // Kept, and sent at once as the next message: awaited.
    c.awaiting = None;
    c.steered(
        "run8",
        "x",
        &steered(br#"{"sent":{"run":"run10","session":"sesn_9"},"held":{"text":"x","at":"2026-11-16T10:05:00Z"}}"#).unwrap(),
        now,
    );
    assert_eq!(c.awaiting, Some(now));
    assert_eq!(
        c.thread
            .steering
            .as_ref()
            .unwrap()
            .held
            .as_ref()
            .unwrap()
            .text,
        "x"
    );
    // A run that's paused reads it when it carries on; with no name, "It".
    c.thread.steering.as_mut().unwrap().status = "PAUSED".into();
    let l = lines(
        &w,
        &c,
        &Whose {
            name: None,
            ask_coach: false,
        },
        &[],
        now,
    )
    .unwrap();
    assert_eq!(l.steering[0].1, "It reads this when it carries on");
}

#[test]
fn a_refusal_shows_as_the_ipad_shows_it() {
    let w = en();
    let r = |status: u16, body: &str| Refusal::read(status, body.as_bytes());
    // Too many too fast, with no words of tafs cloud's: the app's, kept as not sent.
    assert_eq!(
        refused(&w, &r(429, "{}"), "Rae", false).unwrap(),
        SendRefused::NotSent {
            why: "That's a lot of messages in a few minutes. Give Rae a moment, then try again."
                .into(),
            halted: None,
            busy: true
        }
    );
    // A refusal with words says them, as they are: a halt too.
    assert_eq!(
        refused(&w, &r(429, r#"{"words":"Slow down"}"#), "Rae", false).unwrap(),
        SendRefused::NotSent {
            why: "Slow down".into(),
            halted: None,
            busy: false
        }
    );
    assert_eq!(
        refused(
            &w,
            &r(
                409,
                r#"{"error":"halted","words":"Rae is halted. Resume it to send.","halted":"taf"}"#
            ),
            "Rae",
            false
        )
        .unwrap(),
        SendRefused::NotSent {
            why: "Rae is halted. Resume it to send.".into(),
            halted: Some("taf".into()),
            busy: false
        }
    );
    assert_eq!(
        refused(
            &w,
            &r(403, r#"{"code":"ai_consent_required"}"#),
            "Rae",
            false
        )
        .unwrap(),
        SendRefused::Consent
    );
    assert_eq!(
        refused(
            &w,
            &r(
                409,
                r#"{"code":"credits_used_up","words":"Your credits are used up."}"#
            ),
            "Rae",
            true
        )
        .unwrap(),
        SendRefused::Box("Your credits are used up.".into())
    );
    // More than tafs cloud holds: its error as it is, the words back in the box.
    assert_eq!(
        refused(
            &w,
            &r(
                409,
                r#"{"code":"held_full","error":"Up to 10 messages can wait."}"#
            ),
            "Rae",
            true
        )
        .unwrap(),
        SendRefused::Box("Up to 10 messages can wait.".into())
    );
    // On a message, held_full is any refusal.
    assert_eq!(
        refused(
            &w,
            &r(
                409,
                r#"{"code":"held_full","error":"Up to 10 messages can wait."}"#
            ),
            "Rae",
            false
        )
        .unwrap(),
        SendRefused::NotSent {
            why: "Up to 10 messages can wait.".into(),
            halted: None,
            busy: false
        }
    );
    assert_eq!(
        refused(&w, &r(500, "nope"), "Rae", false).unwrap(),
        SendRefused::NotSent {
            why: "tafs cloud answered 500".into(),
            halted: None,
            busy: false
        }
    );
}

#[test]
fn not_sent_rows_and_a_halt_this_computer_cant_resume() {
    let w = en();
    let now = at("2026-11-16T10:05:00Z");
    let mut c = chat_of(&format!(
        r#"{{"messages":[{}]}}"#,
        taf("Hello", "2026-11-16T09:00:00Z", "s")
    ));
    let d = c.deliver("Refund 4410", now).unwrap();
    c.not_sent(
        &d,
        "Rae is halted. Resume it to send.",
        Some("taf".into()),
        false,
        now,
    );
    assert!(!c.sending);
    assert!(c.thread.messages.iter().all(|m| m.session != PENDING));
    let d = c.deliver("Refund 4411", now).unwrap();
    c.not_sent(&d, "Rae is halted", None, false, now);
    let d = c.deliver("Refund 4412", now).unwrap();
    c.not_sent(&d, "It didn't reach tafs cloud.", None, false, now);
    let whose = Whose {
        name: Some("Rae"),
        ask_coach: false,
    };
    let l = lines(&w, &c, &whose, &[], now).unwrap();
    // A halt tafs cloud names: its words, and in Resume's place why this computer can't.
    let halt = &l.unsent[0];
    assert_eq!(halt.note, "Not sent · Rae is halted. Resume it to send.");
    assert!(halt.halted && halt.cant.is_some() && halt.edit.is_none());
    assert_eq!(halt.retry.as_deref(), Some("Retry"));
    // An older tafs cloud's halt, by its words: that it's halted, and nothing reaches it.
    let old = &l.unsent[1];
    assert_eq!(old.note, "Rae is halted");
    assert!(old.resume_note.is_some() && old.cant.is_some() && old.retry.is_none());
    let plain = &l.unsent[2];
    assert_eq!(plain.note, "Not sent · It didn't reach tafs cloud.");
    assert_eq!(
        (plain.edit.as_deref(), plain.retry.as_deref()),
        (Some("Edit"), Some("Retry"))
    );
    assert!(plain.cant.is_none());
    // Each has its menu, Copy and Delete.
    assert!(l
        .unsent
        .iter()
        .all(|u| (u.copy.as_str(), u.delete.as_str()) == ("Copy", "Delete")));
    // Edit takes it out, its words back in the box.
    assert_eq!(c.edit(c.unsent[2].id).as_deref(), Some("Refund 4412"));
    assert_eq!(c.unsent.len(), 2);
    // Delete takes one out, not sent; the others stay in their order.
    let gone = c.unsent[1].id;
    c.discard(gone);
    assert_eq!(c.unsent.len(), 1);
    assert_eq!(c.unsent[0].text, "Refund 4410");
    // Retry takes it out of its place; not taken, it goes back there.
    let (i, u) = c.retry(c.unsent[0].id).unwrap();
    assert!(c.unsent.is_empty());
    c.retry_back(i, u);
    assert_eq!(c.unsent[0].text, "Refund 4410");
}

#[test]
fn a_message_that_goes_takes_a_too_fast_one_away() {
    let now = at("2026-11-16T10:05:00Z");
    let mut c = chat_of(&format!(
        r#"{{"messages":[{}]}}"#,
        taf("Hello", "2026-11-16T09:00:00Z", "s")
    ));
    let d = c.deliver("one", now).unwrap();
    c.not_sent(&d, "too many", None, true, now);
    let d = c.deliver("two", now).unwrap();
    c.delivered(
        &d,
        &sent(br#"{"session":"s","new_session":false}"#).unwrap(),
        now,
    );
    assert!(c.unsent.is_empty());
    // A retry kept again still owes its steering row.
    let mut c = chat_of(std::str::from_utf8(&fixture("taf-chat.json")).unwrap());
    let (_, u) = c.retry(c.unsent[0].id).unwrap();
    let left = c.unsent.len();
    c.steer_not_sent(&u.text, "again", None, false, now);
    c.retry_kept(left, &u);
    assert_eq!(c.unsent[0].settle, u.settle);
}

#[test]
fn the_wait_ends_with_an_answer_work_or_two_minutes() {
    let w = en();
    let since = at("2026-11-16T09:00:00Z");
    let json = format!(
        r#"{{"messages":[{}]}}"#,
        owner("Hi", "2026-11-16T09:00:00Z", "s")
    );
    let mut c = chat_of(&json);
    c.awaiting = Some(since);
    c.took(&w, thread(&json).unwrap(), at("2026-11-16T09:01:00Z"));
    assert!(c.awaiting.is_some());
    c.took(&w, thread(&json).unwrap(), at("2026-11-16T09:02:01Z"));
    assert!(c.awaiting.is_none());
    c.awaiting = Some(since);
    let answered = format!(
        r#"{{"messages":[{},{}]}}"#,
        owner("Hi", "2026-11-16T09:00:00Z", "s"),
        taf("Hello", "2026-11-16T09:00:03Z", "s")
    );
    c.took(&w, thread(&answered).unwrap(), at("2026-11-16T09:00:10Z"));
    assert!(c.awaiting.is_none());
}

#[test]
fn reads_follow_each_message_thirty_times() {
    let mut c = chat_of(&format!(
        r#"{{"messages":[{}]}}"#,
        owner("Hi", "2026-11-16T09:00:00Z", "s")
    ));
    let mut p = Polls::default();
    assert!(c.awaiting_reply());
    for _ in 0..POLL_BUDGET {
        assert!(p.read_now(&c));
    }
    assert!(!p.read_now(&c));
    // Another message: thirty more.
    c.thread.messages.push(mine_like("again"));
    assert!(p.read_now(&c));
    // Answered and idle: no reads.
    let c = chat_of(&format!(
        r#"{{"messages":[{},{}]}}"#,
        owner("Hi", "2026-11-16T09:00:00Z", "s"),
        taf("Hello", "2026-11-16T09:00:03Z", "s")
    ));
    assert!(!Polls::default().read_now(&c));
    // Nothing of the owner's to follow: no reads, as the iPad's.
    let c = chat_of(r#"{"messages":[],"working":{"session":"s","run":"r"}}"#);
    assert!(c.thinking());
    assert!(!Polls::default().read_now(&c));
}

#[test]
fn a_held_card_goes_after_the_words_that_ask_for_it_and_waits_for_them() {
    let json = format!(
        r#"{{"messages":[{},{},{}]}}"#,
        owner("Refund 4410", "2026-11-16T09:00:00Z", "s"),
        taf("It waits for your approval.", "2026-11-16T09:00:30Z", "s"),
        taf("Anything else?", "2026-11-16T09:00:31Z", "s")
    );
    let t = thread(&json).unwrap();
    let held = vec![("call-1".to_string(), at("2026-11-16T09:00:10Z"))];
    let ids: Vec<String> = items(&t, &held, at("2026-11-16T09:01:00Z"))
        .iter()
        .map(Item::id)
        .collect();
    assert_eq!(ids.len(), 4);
    assert!(ids[..3].iter().all(|i| i.starts_with("m|")), "{ids:?}");
    assert_eq!(ids.last().unwrap(), "k|call-1", "past that turn's words");
    // Only past that turn's words: another session's after them stay below the card.
    let json = format!(
        r#"{{"messages":[{},{},{}]}}"#,
        owner("Refund 4410", "2026-11-16T09:00:00Z", "s"),
        taf("It waits for you.", "2026-11-16T09:00:30Z", "s"),
        taf("Scheduled report", "2026-11-16T09:00:40Z", "sched")
    );
    let t = thread(&json).unwrap();
    let held = vec![("call-1".to_string(), at("2026-11-16T09:00:10Z"))];
    let it = items(&t, &held, at("2026-11-16T09:10:00Z"));
    assert!(matches!(it[2], Item::Held { .. }));
    assert!(matches!(it[3], Item::Message(m) if m.session == "sched"));
    // Held after the owner's last message, with no words since: hidden for 45 s, then shown.
    let json = format!(
        r#"{{"messages":[{}]}}"#,
        owner("Refund 4410", "2026-11-16T09:00:00Z", "s")
    );
    let t = thread(&json).unwrap();
    let held = vec![("call-1".to_string(), at("2026-11-16T09:00:10Z"))];
    assert_eq!(items(&t, &held, at("2026-11-16T09:00:54Z")).len(), 1);
    assert_eq!(items(&t, &held, at("2026-11-16T09:00:55Z")).len(), 2);
    // The same key twice: once.
    let held = vec![
        ("call-1".to_string(), at("2026-11-16T09:00:10Z")),
        ("call-1".to_string(), at("2026-11-16T09:00:11Z")),
    ];
    assert_eq!(items(&t, &held, at("2026-11-16T10:00:00Z")).len(), 2);
}

#[test]
fn the_chat_draws_the_ipad_s_lines() {
    let w = en();
    let now = at("2026-11-16T10:00:00Z");
    let c = chat_of(std::str::from_utf8(&fixture("taf-chat.json")).unwrap());
    let l = lines(
        &w,
        &c,
        &Whose {
            name: Some("Rae"),
            ask_coach: true,
        },
        &[],
        now,
    )
    .unwrap();
    assert_eq!(l.empty, None);
    assert_eq!(l.thinking, None);
    assert_eq!(l.menu.0, "Send again");
    let mut kinds = l.lines.iter();
    assert!(matches!(kinds.next(), Some(Line::Gap(_))));
    assert!(
        matches!(kinds.next(), Some(Line::Kickoff(k)) if k.ends_with("· Work the refund queue: every open refund request since your last run."))
    );
    match kinds.next() {
        Some(Line::Taf {
            delivered,
            counted,
            refused,
            stamp,
            ..
        }) => {
            assert_eq!(
                delivered.as_deref(),
                Some("Sent to Google Sheets · Refunds 2026")
            );
            assert_eq!(
                counted.as_deref(),
                Some("From the Record — done: 3 · waiting for you: 1")
            );
            assert_eq!((refused, stamp), (&None, &None));
        }
        other => panic!("{other:?}"),
    }
    // An hour and more later: a time.
    assert!(matches!(kinds.next(), Some(Line::Gap(_))));
    assert!(matches!(
        kinds.next(),
        Some(Line::Owner {
            menu: true,
            stamp: None,
            pending: false,
            ..
        })
    ));
    assert!(matches!(kinds.next(), Some(Line::Taf { .. })));
    assert_eq!(
        kinds.next(),
        Some(&Line::AskCoach {
            text: "Raise Rae's limit for one refund to €200, so orders like 4410 go through without waiting.".into(),
            ask: Some("Ask Coach".into())
        })
    );
    // A door's message: by its door, with no menu; its answer, how it went back.
    assert_eq!(
        kinds.next(),
        Some(&Line::Owner {
            text: "Please refund order 4415, it arrived broken.".into(),
            answered: None,
            stamp: Some("BY EMAIL · ana@example.com".into()),
            menu: false,
            pending: false,
            files: vec![]
        })
    );
    assert!(
        matches!(kinds.next(), Some(Line::Taf { stamp: Some((s, false)), .. }) if s == "↩ SENT BACK BY EMAIL · SAME THREAD")
    );
    assert!(matches!(kinds.next(), Some(Line::Owner { .. })));
    match kinds.next() {
        Some(Line::Taf {
            refused: Some(r), ..
        }) => assert_eq!(r.lines().len(), 3),
        other => panic!("{other:?}"),
    }
    assert_eq!(kinds.next(), None);
    assert_eq!(l.unsent.len(), 1);
    // Someone who can't change the taf reads the card alone.
    let l = lines(
        &w,
        &c,
        &Whose {
            name: Some("Rae"),
            ask_coach: false,
        },
        &[],
        now,
    )
    .unwrap();
    assert!(l
        .lines
        .iter()
        .any(|x| matches!(x, Line::AskCoach { ask: None, .. })));
}

#[test]
fn an_empty_chat_says_what_it_s_for_and_the_box_says_who() {
    let w = en();
    let now = at("2026-11-16T10:00:00Z");
    let c = chat_of(r#"{"messages":[]}"#);
    let l = lines(
        &w,
        &c,
        &Whose {
            name: Some("Rae"),
            ask_coach: false,
        },
        &[],
        now,
    )
    .unwrap();
    assert!(l.empty.unwrap().starts_with("Ask Rae what it did"));
    let l = lines(
        &w,
        &c,
        &Whose {
            name: None,
            ask_coach: false,
        },
        &[],
        now,
    )
    .unwrap();
    assert!(l.empty.unwrap().starts_with("Ask it what it did"));
    let k = composer(&w, Some("Rae"), false).unwrap();
    assert_eq!(
        (k.placeholder.as_str(), k.send.as_str(), k.stop.as_str()),
        ("Message Rae", "Send", "Stop")
    );
    let k = composer(&w, Some("Rae"), true).unwrap();
    assert_eq!(
        (k.placeholder.as_str(), k.send.as_str()),
        ("Write the next message", "Send to Rae while it works")
    );
    assert_eq!(
        composer(&w, None, false).unwrap().placeholder,
        "Message your taf"
    );
}

#[test]
fn every_cap_has_its_words_and_door_stamps_say_how_it_went() {
    let w = en();
    for stop in CAP_STOPS {
        assert!(cap_words(&w, stop, "Rae")
            .unwrap()
            .unwrap()
            .starts_with("Rae stopped:"));
    }
    assert_eq!(cap_words(&w, "end_turn", "Rae").unwrap(), None);
    let mark = |reply: Option<&str>, why: Option<&str>| crate::coach::DoorMark {
        app: "whatsapp".into(),
        from: None,
        subject: None,
        reply: reply.map(str::to_string),
        why: why.map(str::to_string),
    };
    assert_eq!(
        door_stamp(&w, &mark(None, None), true).unwrap().0,
        "BY WHATSAPP"
    );
    assert_eq!(
        door_stamp(&w, &mark(None, None), false).unwrap().0,
        "GOES BACK BY WHATSAPP WHEN DONE"
    );
    assert_eq!(
        door_stamp(&w, &mark(Some("not_sent"), Some("No number")), false).unwrap(),
        ("NOT SENT BACK · No number".into(), true)
    );
    assert!(
        halted_words("Halted: resume first")
            && halted_words("Rae is halted")
            && !halted_words("Not halting")
    );
}

#[test]
fn a_taf_s_answer_carries_its_work_folded_in_its_words() {
    let w = en();
    let c = chat_of(std::str::from_utf8(&fixture("taf-chat.json")).unwrap());
    let l = lines(
        &w,
        &c,
        &Whose {
            name: Some("Rae"),
            ask_coach: false,
        },
        &[],
        at("2026-11-16T10:00:00Z"),
    )
    .unwrap();
    let works: Vec<&WorkLines> = l
        .lines
        .iter()
        .filter_map(|x| match x {
            Line::Taf {
                work: Some(work), ..
            } => Some(work),
            _ => None,
        })
        .collect();
    assert_eq!(works.len(), 2);
    let report = works[0];
    assert_eq!(report.summary, "Worked for 12 s");
    let rows: Vec<(&str, Option<&str>)> = report
        .steps
        .iter()
        .map(|r| (r.words.as_str(), r.note.as_deref()))
        .collect();
    // Tries of one step after a failed one are one row; the failure is left out once it worked.
    assert_eq!(
        rows,
        vec![
            ("Reading its notes", None),
            // A connected tool's title is planque's, which this computer doesn't read: its name
            // in KIFF in words, as `KIFFMapping.toolWords` gives it.
            ("Asking KIFF to run list payments k3f9", None),
            ("Asking KIFF to run create refund k3f9", Some("2 tries")),
            ("Checking what waits for you", None),
        ]
    );
    assert!(report
        .steps
        .iter()
        .all(|r| r.status == crate::coach_lines::StepStatus::Done));
    assert_eq!(report.accessible, "Worked for 12 s, 4 steps");
    // The owner's messages and a message without work have none.
    assert!(l
        .lines
        .iter()
        .any(|x| matches!(x, Line::Taf { work: None, .. })));
}

#[test]
fn only_the_taf_s_answers_with_an_id_can_be_reported() {
    let w = en();
    let c = chat_of(std::str::from_utf8(&fixture("taf-chat.json")).unwrap());
    let l = lines(
        &w,
        &c,
        &Whose {
            name: Some("Rae"),
            ask_coach: false,
        },
        &[],
        at("2026-11-16T10:00:00Z"),
    )
    .unwrap();
    let reported: Vec<crate::report::Reported> = l
        .lines
        .iter()
        .filter_map(|x| match x {
            Line::Taf { report, .. } => report.clone(),
            _ => None,
        })
        .collect();
    let ids: Vec<&str> = reported.iter().map(|r| r.message.as_str()).collect();
    assert_eq!(ids, ["run4.3", "run5.2", "door1.2", "run7.4"]);
    assert_eq!(reported[1].session, "sesn_9");
}
