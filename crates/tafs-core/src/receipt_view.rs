//! A Record line's receipt (`RecordView.swift` › `ReceiptView`): the line's state, its title with
//! the amount, its facts, the steps KIFF signed, and KIFF's receipt checked on this computer
//! (`receipt_check`). The steps are read from the same bytes the check verified
//! (`receipt_check::raw_value`), never from a second decode of the whole answer.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::receipt_check::{self, Check};
use crate::record::{Amount, Entry, Kind};
use crate::words::{WordBook, WordsError};

/// `GET /v1/receipts/{trace_id}` at KIFF, the id escaped as one segment.
pub fn path(trace_id: &str) -> String {
    format!("/v1/receipts/{}", crate::access::escaped(trace_id))
}

/// One step KIFF signed (`Step`): its words, when, its hash shortened, and what it names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub words: String,
    pub at: Option<DateTime<Utc>>,
    pub hash: String,
    pub kind: String,
    /// What the step says the call acted on, as KIFF hashed it.
    pub on_hash: Option<String>,
    /// The amount the decision step signed, when it has one.
    pub amount: Option<Amount>,
    /// A `hold_settled` step's answer, as KIFF records it ("reject" for a refusal).
    pub answer: Option<String>,
}

/// The receipt's steps, as KIFF signed them. None when the answer has no single chain
/// (`raw_value` refuses a key written twice).
pub fn steps(raw: &[u8]) -> Vec<Step> {
    let Some(chain) = receipt_check::raw_value("causal_chain", raw) else {
        return vec![];
    };
    let Ok(Value::Array(list)) = serde_json::from_slice::<Value>(chain) else {
        return vec![];
    };
    list.iter()
        .filter_map(Value::as_object)
        .map(|s| {
            let text = |k: &str| s.get(k).and_then(Value::as_str);
            let kind = text("kind").unwrap_or_default().to_string();
            let message = text("message").filter(|m| !m.is_empty());
            let hash = text("data_hash").unwrap_or_default();
            let data = s.get("data").and_then(Value::as_object);
            let amount = data
                .and_then(|d| d.get("amount"))
                .and_then(Value::as_object)
                .and_then(|a| {
                    Some(Amount {
                        value: a.get("value")?.as_str()?.to_string(),
                        unit: a.get("unit").and_then(Value::as_str).map(str::to_string),
                    })
                });
            Step {
                words: message
                    .map(str::to_string)
                    .unwrap_or_else(|| kind.replace('_', " ")),
                at: text("occurred_at")
                    .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
                    .map(|t| t.with_timezone(&Utc)),
                hash: short(hash),
                on_hash: data
                    .and_then(|d| d.get("on_hash"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                amount,
                answer: data
                    .and_then(|d| d.get("answer"))
                    .and_then(Value::as_str)
                    .map(str::to_string),
                kind,
            }
        })
        .collect()
}

/// A hash shortened as the sheet shows it: its first and last four, longer than eight.
fn short(hash: &str) -> String {
    let n = hash.chars().count();
    if n > 8 {
        let head: String = hash.chars().take(4).collect();
        let tail: String = hash.chars().skip(n - 4).collect();
        format!("{head}…{tail}")
    } else {
        hash.to_string()
    }
}

/// `KIFFMapping.isOn`: the value KIFF's hash names.
pub fn is_on(value: &str, hash: &str) -> bool {
    let digest: String = Sha256::digest(value.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    hash == format!("sha256:{digest}")
}

/// The signed chain names what the call acted on, and the line's value isn't it.
pub fn on_mismatch(entry: &Entry, steps: &[Step]) -> bool {
    match (
        &entry.on_value,
        steps.iter().find_map(|s| s.on_hash.as_ref()),
    ) {
        (Some(value), Some(signed)) => !is_on(value, signed),
        _ => false,
    }
}

/// The signed decision step names the amount, and the line's isn't it.
pub fn amount_mismatch(entry: &Entry, steps: &[Step]) -> bool {
    match (&entry.amount, steps.iter().find_map(|s| s.amount.as_ref())) {
        (Some(line), Some(signed)) => {
            line.value != signed.value
                || line.unit.as_deref().unwrap_or("") != signed.unit.as_deref().unwrap_or("")
        }
        _ => false,
    }
}

/// The signed chain has the answer (its `hold_settled` step), and the line's isn't it: KIFF
/// records a refusal as "reject".
pub fn answer_mismatch(entry: &Entry, steps: &[Step]) -> bool {
    let signed = steps
        .iter()
        .find(|s| s.kind == "hold_settled")
        .and_then(|s| s.answer.as_deref());
    match (&entry.answer, signed) {
        (Some(line), Some(signed)) => (line.decision == "refused") != (signed == "reject"),
        _ => false,
    }
}

/// A string field at the receipt's top level, read from its own bytes.
pub fn field(raw: &[u8], key: &str) -> Option<String> {
    let bytes = receipt_check::raw_value(key, raw)?;
    serde_json::from_slice::<String>(bytes).ok()
}

/// A string field of the receipt's attestation.
pub fn attestation(raw: &[u8], key: &str) -> Option<String> {
    let bytes = receipt_check::raw_value("attestation", raw)?;
    let v: Value = serde_json::from_slice(bytes).ok()?;
    v.get(key)?.as_str().map(str::to_string)
}

/// The line's state, beside its coloured dot (`stateLabel`).
pub fn state_label(w: &WordBook, e: &Entry) -> Result<String, WordsError> {
    if let Some(answer) = &e.answer {
        let who = e.answerer(w)?;
        return w.get(
            if answer.decision == "refused" {
                "receipt.refused_by"
            } else {
                "receipt.approved_once_by"
            },
            &[("who", &who)],
        );
    }
    w.line(match e.kind {
        Kind::Done => "receipt.label.done",
        Kind::Approved => "receipt.label.approved",
        Kind::Refused => "record.filter.refused",
        Kind::Held => "receipt.label.held",
        Kind::Unknown => "receipt.label.unknown",
        Kind::NotDone => "receipt.label.not_done",
        Kind::Sending => "receipt.label.sending",
        Kind::Revoked => "card.card_revoked",
        Kind::Started => "receipt.label.started",
        Kind::InTafs => "receipt.label.in_tafs",
    })
}

/// The Outcome fact (`outcome`).
pub fn outcome(w: &WordBook, e: &Entry) -> Result<String, WordsError> {
    match e.kind {
        Kind::Done | Kind::Approved => match e.amount_text(w)? {
            Some(amount) if e.amount.is_some() => {
                w.get("receipt.outcome.moved", &[("amount", &amount)])
            }
            _ => w.line("receipt.outcome.done"),
        },
        Kind::Refused => match &e.answer {
            Some(a) if a.decision == "refused" => {
                w.get("receipt.outcome.refused_by", &[("who", &e.answerer(w)?)])
            }
            _ => w.line("receipt.outcome.card_refused"),
        },
        Kind::Held => w.line("receipt.outcome.waiting"),
        _ => Ok(e.detail.clone()),
    }
}

/// The decision, first of the receipt's facts (`decisionWords`).
pub fn decision_words(w: &WordBook, e: &Entry) -> Result<String, WordsError> {
    if let Some(answer) = &e.answer {
        return w.get(
            if answer.decision == "refused" {
                "receipt.decision.held_refused_by"
            } else {
                "receipt.decision.held_approved_once_by"
            },
            &[("who", &e.answerer(w)?)],
        );
    }
    w.line(match e.kind {
        Kind::Refused => "receipt.decision.refused_by_card",
        // Still waiting: the Card held it, it didn't allow it.
        Kind::Held => "receipt.decision.held_by_card",
        _ if e.owner_approved => "receipt.decision.held_approved_by_you",
        _ => "receipt.decision.allowed_by_card",
    })
}

/// The Decided by fact (`decidedBy`). `kiff`: signed in with KIFF (else the sample's Card).
pub fn decided_by(w: &WordBook, e: &Entry, kiff: bool) -> Result<String, WordsError> {
    if let Some(answer) = &e.answer {
        let mut how = if answer.you == Some(true) {
            w.line("receipt.decided_by.you")?
        } else {
            let who = e.answerer(w)?;
            let mut c = who.chars();
            match c.next() {
                Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                None => who,
            }
        };
        if answer.signed == Some(true) {
            let device = match answer.device.as_deref().filter(|d| !d.is_empty()) {
                Some(d) => d.to_string(),
                None => w.line("receipt.decided_by.your_device")?,
            };
            how = w.get(
                "receipt.decided_by.signed_on",
                &[("who", &how), ("device", &device)],
            )?;
        }
        return Ok(how);
    }
    w.line(match e.kind {
        Kind::Approved => "receipt.decided_by.you_approving_once",
        _ if e.owner_approved => "receipt.decided_by.you_approving_once",
        Kind::Revoked => "receipt.decided_by.you_revoking",
        Kind::Started => "receipt.decided_by.you_issuing",
        _ if kiff => "receipt.decided_by.kiff",
        _ => "receipt.decided_by.card",
    })
}

/// What the receipt panel says (`receiptPanel`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Panel {
    /// Being read and checked: "Checking KIFF's receipt on this computer…".
    Checking,
    /// It couldn't be read from KIFF: why, and Try again.
    NotLoaded(String),
    /// Checked: signed by KIFF, unchanged since.
    Checked { signer: String },
    /// KIFF signs in its next sweep. `late`: ten minutes on, with the id to tell support.
    NotSignedYet { late: bool },
    /// It doesn't check out, and why: a mismatch with the line, a changed chain, or the check's
    /// own reason.
    Failed(String),
}

/// The panel for a check of the receipt that was read, and the line it's for.
pub fn panel(
    w: &WordBook,
    e: &Entry,
    check: &Check,
    steps: &[Step],
    now: DateTime<Utc>,
) -> Result<Panel, WordsError> {
    Ok(match check {
        Check::Verified { .. } if on_mismatch(e, steps) => {
            Panel::Failed(w.line("receipt.check.payment_mismatch")?)
        }
        Check::Verified { .. } if amount_mismatch(e, steps) => {
            Panel::Failed(w.line("receipt.check.amount_mismatch")?)
        }
        Check::Verified { .. } if answer_mismatch(e, steps) => {
            Panel::Failed(w.line("receipt.check.answer_mismatch")?)
        }
        Check::Verified { signer } => Panel::Checked {
            signer: signer.clone(),
        },
        Check::NotSignedYet => Panel::NotSignedYet {
            late: now - e.at > chrono::Duration::minutes(10),
        },
        Check::Failed(_) => Panel::Failed(w.line("receipt.changed")?),
    })
}

/// The full receipt's lines, in mono under "Show the full receipt" (`fullReceipt`): the receipt
/// and its signer, each step with when and its hash, and the signature.
pub fn full_receipt(
    trace_id: &str,
    signer: Option<&str>,
    steps: &[Step],
    signature: Option<&str>,
    time: impl Fn(DateTime<Utc>) -> String,
) -> Vec<String> {
    let mut out = vec![];
    let mut head = vec![format!("receipt {trace_id}")];
    if let Some(s) = signer {
        let n = s.chars().count();
        let first: String = s.chars().take(6).collect();
        let last: String = s.chars().skip(n.saturating_sub(4)).collect();
        head.push(format!("signer {first}…{last}"));
    }
    out.push(head.join(" · "));
    for (i, step) in steps.iter().enumerate() {
        let at = step.at.map(&time).unwrap_or_default();
        out.push(format!(
            "{}  {} · {at}\n   hash {}",
            i + 1,
            step.words,
            step.hash
        ));
    }
    if let Some(sig) = signature {
        let n = sig.chars().count();
        let first: String = sig.chars().take(4).collect();
        let last: String = sig.chars().skip(n.saturating_sub(4)).collect();
        out.push(format!(
            "✎  trace {trace_id} · secp256k1 + keccak256\n   sig 0x{first}…{last}"
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{Answer, Receipt};
    use chrono::TimeZone;

    fn words() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    fn entry(kind: Kind) -> Entry {
        Entry {
            id: "e".into(),
            taf: "rae".into(),
            kind,
            title: "Refund a payment".into(),
            detail: "Done, within its Card".into(),
            at: Utc.with_ymd_and_hms(2026, 10, 21, 0, 41, 3).unwrap(),
            owner_approved: false,
            amount: None,
            read_only: false,
            on_arg: None,
            on_value: None,
            answer: None,
            receipt: Some(Receipt {
                trace_id: "01KTRACE".into(),
                status: "signed".into(),
            }),
            proposal: None,
            reason: None,
        }
    }

    const RAW: &str = r#"{"trace_id":"01KTRACE","issued_at":"2026-10-21T00:41:03.12Z","causal_chain":[{"sequence":1,"kind":"decision","occurred_at":"2026-10-21T00:41:03.12Z","message":"held: over \u003c€100\u003e","data_hash":"0123456789abcdef","data":{"amount":{"value":"150.00","unit":"EUR"},"on_hash":"sha256:2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"}},{"sequence":2,"kind":"hold_settled","occurred_at":"2026-10-21T00:41:04.12Z","data_hash":"bb","data":{"answer":"approved_once"}}],"attestation":{"algorithm":"secp256k1+keccak256","signature":"79917d81aabbccdd"}}"#;

    #[test]
    fn the_steps_are_read_from_the_checked_bytes() {
        let s = steps(RAW.as_bytes());
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].words, "held: over <€100>");
        assert_eq!(s[0].hash, "0123…cdef");
        assert_eq!(s[0].amount.as_ref().unwrap().value, "150.00");
        assert_eq!(s[1].words, "hold settled");
        assert_eq!(s[1].hash, "bb");
        assert_eq!(s[1].answer.as_deref(), Some("approved_once"));
        assert_eq!(
            s[1].at,
            Some(
                Utc.with_ymd_and_hms(2026, 10, 21, 0, 41, 4).unwrap()
                    + chrono::Duration::milliseconds(120)
            )
        );
        // A chain written twice: no steps, as the check refuses it.
        let twice = RAW.replacen(r#""attestation""#, r#""causal_chain":[],"attestation""#, 1);
        assert!(steps(twice.as_bytes()).is_empty());
        assert!(steps(b"not json").is_empty());
        assert_eq!(
            field(RAW.as_bytes(), "issued_at").as_deref(),
            Some("2026-10-21T00:41:03.12Z")
        );
        assert_eq!(
            attestation(RAW.as_bytes(), "signature").as_deref(),
            Some("79917d81aabbccdd")
        );
        assert_eq!(path("01K/TRACE"), "/v1/receipts/01K%2FTRACE");
    }

    #[test]
    fn what_the_line_says_is_held_to_what_kiff_signed() {
        let s = steps(RAW.as_bytes());
        let mut e = entry(Kind::Done);
        assert!(!on_mismatch(&e, &s) && !amount_mismatch(&e, &s) && !answer_mismatch(&e, &s));
        // "foo" is what the signed hash names.
        e.on_value = Some("foo".into());
        assert!(!on_mismatch(&e, &s));
        e.on_value = Some("bar".into());
        assert!(on_mismatch(&e, &s));
        e.amount = Some(Amount {
            value: "150.00".into(),
            unit: Some("EUR".into()),
        });
        assert!(!amount_mismatch(&e, &s));
        e.amount = Some(Amount {
            value: "950.00".into(),
            unit: Some("EUR".into()),
        });
        assert!(amount_mismatch(&e, &s));
        e.answer = Some(Answer {
            decision: "approved_once".into(),
            by: None,
            you: Some(true),
            signed: None,
            device: None,
        });
        assert!(!answer_mismatch(&e, &s));
        e.answer.as_mut().unwrap().decision = "refused".into();
        assert!(answer_mismatch(&e, &s));
        assert!(is_on(
            "foo",
            "sha256:2c26b46b68ffc68ff99b453c1d30413413422d706483bfa0f98a5e886266e7ae"
        ));
    }

    #[test]
    fn the_panel_says_what_the_check_found() {
        let w = words();
        let s = steps(RAW.as_bytes());
        let e = entry(Kind::Done);
        let now = e.at + chrono::Duration::minutes(1);
        let ok = Check::Verified {
            signer: "0xabc".into(),
        };
        assert_eq!(
            panel(&w, &e, &ok, &s, now).unwrap(),
            Panel::Checked {
                signer: "0xabc".into()
            }
        );
        let mut wrong = e.clone();
        wrong.amount = Some(Amount {
            value: "950.00".into(),
            unit: Some("EUR".into()),
        });
        assert_eq!(
            panel(&w, &wrong, &ok, &s, now).unwrap(),
            Panel::Failed("The amount on this line doesn't match KIFF's receipt.".into())
        );
        assert_eq!(
            panel(&w, &e, &Check::NotSignedYet, &s, now).unwrap(),
            Panel::NotSignedYet { late: false }
        );
        assert_eq!(
            panel(
                &w,
                &e,
                &Check::NotSignedYet,
                &s,
                e.at + chrono::Duration::minutes(11)
            )
            .unwrap(),
            Panel::NotSignedYet { late: true }
        );
        assert!(matches!(
            panel(
                &w,
                &e,
                &Check::Failed(receipt_check::Failure::NotKiff),
                &s,
                now
            )
            .unwrap(),
            Panel::Failed(_)
        ));
    }

    #[test]
    fn the_facts_read_as_the_ipads() {
        let w = words();
        let mut e = entry(Kind::Done);
        assert_eq!(state_label(&w, &e).unwrap(), "Done within the Card");
        assert_eq!(outcome(&w, &e).unwrap(), "Done.");
        assert_eq!(decision_words(&w, &e).unwrap(), "allowed by the Card");
        assert_eq!(decided_by(&w, &e, true).unwrap(), "KIFF, against the Card");
        assert_eq!(decided_by(&w, &e, false).unwrap(), "The Card");
        e.amount = Some(Amount {
            value: "150.00".into(),
            unit: Some("EUR".into()),
        });
        assert_eq!(outcome(&w, &e).unwrap(), "Done. €150 moved.");
        e.answer = Some(Answer {
            decision: "approved_once".into(),
            by: None,
            you: Some(true),
            signed: Some(true),
            device: Some("iPhone".into()),
        });
        assert_eq!(state_label(&w, &e).unwrap(), "Approved once by you");
        assert_eq!(decided_by(&w, &e, true).unwrap(), "You, signed on iPhone");
        assert_eq!(
            decision_words(&w, &e).unwrap(),
            "held → approved once by you"
        );
        let r = entry(Kind::Refused);
        assert_eq!(outcome(&w, &r).unwrap(), "Not sent. Its Card refused it.");
        assert_eq!(decision_words(&w, &r).unwrap(), "refused by the Card");
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            for kind in [
                Kind::Done,
                Kind::Approved,
                Kind::Refused,
                Kind::Held,
                Kind::Unknown,
                Kind::Revoked,
                Kind::Started,
                Kind::NotDone,
                Kind::Sending,
                Kind::InTafs,
            ] {
                let e = entry(kind);
                state_label(&w, &e).unwrap();
                outcome(&w, &e).unwrap();
                decision_words(&w, &e).unwrap();
                decided_by(&w, &e, true).unwrap();
            }
        }
    }

    #[test]
    fn the_full_receipt_lists_every_step() {
        let s = steps(RAW.as_bytes());
        let lines = full_receipt(
            "01KTRACE",
            Some("0x2c7536e3605d9c16a7a3d7b1898e529396a65c23"),
            &s,
            Some("79917d81aabbccdd"),
            |t| t.format("%H:%M:%S").to_string(),
        );
        assert_eq!(lines[0], "receipt 01KTRACE · signer 0x2c75…5c23");
        assert_eq!(
            lines[1],
            "1  held: over <€100> · 00:41:03\n   hash 0123…cdef"
        );
        assert_eq!(
            lines[3],
            "✎  trace 01KTRACE · secp256k1 + keccak256\n   sig 0x7991…ccdd"
        );
    }
}
