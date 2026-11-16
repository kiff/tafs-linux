//! A taf's doors and the knocks at them (`TafsCloud+Doors.swift`, `DoorsModel`), read from
//! tafs cloud as the session (`GET /v1/tafs/{id}/doors`), and the lines a knock is drawn with on
//! the taf's Activity (`TafKnocksPanel`, `KnockCard`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

use crate::words::{WordBook, WordsError};

/// A paired sender (`Door.Key`): its id; the rest read when it can be.
#[derive(Debug, Clone, PartialEq)]
pub struct Key {
    pub id: String,
    pub name: Option<String>,
    pub auto: bool,
}

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Key, D::Error> {
        let v = Value::deserialize(d)?;
        let id = v
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| serde::de::Error::custom("a key without its id"))?;
        Ok(Key {
            id: id.to_string(),
            name: v.get("name").and_then(Value::as_str).map(str::to_string),
            auto: v.get("auto").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}

/// tafs cloud's words for a door (`Door.Words`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DoorWords {
    pub state: String,
    pub tap: String,
    pub settings: String,
}

/// One door (`TafsCloud.Door`), decoded as the iPad's synthesized decoder does: every field it
/// needs, or the doors aren't read.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Door {
    pub id: String,
    pub app: String,
    pub kind: String,
    pub label: String,
    pub open: bool,
    pub address: String,
    pub proof: String,
    pub how: String,
    pub keys: Vec<Key>,
    pub today: i64,
    pub limit: i64,
    pub resting: bool,
    #[serde(default)]
    pub others_today: Option<i64>,
    #[serde(default)]
    pub others_limit: Option<i64>,
    #[serde(default)]
    pub words: Option<DoorWords>,
    #[serde(default)]
    pub grant: Option<String>,
    #[serde(default)]
    pub channel: Option<String>,
    #[serde(default)]
    pub trigger: Option<String>,
}

/// An app a door can open on (`DoorApp`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct DoorApp {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub proof: String,
    pub how: String,
}

/// Someone without a key wrote (`TafsCloud.Knock`): the owner decides.
#[derive(Debug, Clone, PartialEq)]
pub struct Knock {
    pub id: String,
    pub door: String,
    pub app: String,
    pub from: String,
    pub name: Option<String>,
    /// "pass", "fail" (the sender check failed) or "none" (the door can't tell).
    pub proof: String,
    pub subject: Option<String>,
    pub text: String,
    pub attachments: i64,
    pub at: DateTime<Utc>,
    pub state: String,
    /// Not a message: the line saying the door had its day's messages.
    pub rest: Option<String>,
    pub dropped: i64,
    /// Its sender was paired from it; the message still waits.
    pub paired: bool,
}

/// An ISO time as tafs cloud writes it, with or without fractions of a second.
fn time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

impl<'de> Deserialize<'de> for Knock {
    /// `Knock.init(from:)`: its id, door, app, sender and time, or it isn't read; the rest
    /// read when it can be.
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Knock, D::Error> {
        let v = Value::deserialize(d)?;
        let s = |k: &str| v.get(k).and_then(Value::as_str).map(str::to_string);
        let need =
            |k: &str| s(k).ok_or_else(|| serde::de::Error::custom(format!("a knock without {k}")));
        let n = |k: &str| v.get(k).and_then(Value::as_i64).unwrap_or(0);
        Ok(Knock {
            id: need("id")?,
            door: need("door")?,
            app: need("app")?,
            from: need("from")?,
            name: s("name"),
            proof: s("proof").unwrap_or_else(|| "none".into()),
            subject: s("subject"),
            text: s("text").unwrap_or_default(),
            attachments: n("attachments"),
            at: s("at")
                .as_deref()
                .and_then(time)
                .ok_or_else(|| serde::de::Error::custom("a knock without its time"))?,
            state: s("state").unwrap_or_else(|| "open".into()),
            rest: s("rest"),
            dropped: n("dropped"),
            paired: v.get("paired").and_then(Value::as_bool).unwrap_or(false),
        })
    }
}

impl Knock {
    /// `isLine`: the door's line about its day's messages, not a message.
    pub fn is_line(&self) -> bool {
        self.rest.is_some()
    }

    /// `who`: "Ana (ana@example.com)", or the sender alone.
    pub fn who(&self) -> String {
        match &self.name {
            Some(n) => format!("{n} ({})", self.from),
            None => self.from.clone(),
        }
    }
}

/// tafs cloud's answer (`TafsCloud.Doors`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Doors {
    pub doors: Vec<Door>,
    pub apps: Vec<DoorApp>,
    pub knocks: Vec<Knock>,
}

/// The doors, as the iPad decodes them; `None` when they can't be.
pub fn read(body: &[u8]) -> Option<Doors> {
    serde_json::from_slice(body).ok()
}

/// `TafsCloud.doors`' path.
pub fn path(taf: &str) -> String {
    format!("/v1/tafs/{}/doors", crate::work::segment(taf))
}

/// `answerKnock`'s path.
pub fn knock_path(taf: &str, knock: &str) -> String {
    format!(
        "/v1/tafs/{}/doors/knocks/{}",
        crate::work::segment(taf),
        crate::work::segment(knock)
    )
}

/// `answerKnock`'s body: "do" (with the owner's words, when they wrote some), "pair" or "ignore".
pub fn answer_body(action: &str, say: Option<&str>) -> Vec<u8> {
    let mut body = serde_json::Map::new();
    body.insert("action".into(), Value::from(action));
    if let Some(say) = say.filter(|s| !s.is_empty()) {
        body.insert("say".into(), Value::from(say));
    }
    Value::Object(body).to_string().into_bytes()
}

/// `answerKnock`'s answer, `{"state": …}`: whether it was taken.
pub fn answered(body: &[u8]) -> bool {
    serde_json::from_slice::<Value>(body)
        .ok()
        .and_then(|v| v.get("state").map(Value::is_string))
        .unwrap_or(false)
}

/// `DoorNames.label`: an app's name as the owner reads it.
pub fn label(app: &str) -> String {
    match app {
        "email" => "email".into(),
        "whatsapp" => "WhatsApp".into(),
        _ => {
            let mut c = app.chars();
            match c.next() {
                Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                None => String::new(),
            }
        }
    }
}

/// `TafKnocksPanel`: the open knocks that are messages, strangers' first, then paired senders'.
pub fn panel(doors: &Doors) -> (Vec<Knock>, Vec<Knock>) {
    let open = doors
        .knocks
        .iter()
        .filter(|k| k.state == "open" && !k.is_line());
    let (paired, waiting): (Vec<Knock>, Vec<Knock>) = open.cloned().partition(|k| k.paired);
    (waiting, paired)
}

/// A knock's lines and buttons, as `KnockCard` draws a message (the panel draws no door lines).
#[derive(Debug, Clone, PartialEq)]
pub struct KnockLines {
    /// "● KNOCK · BY EMAIL", with "· DIDN'T CHECK OUT" when the sender check failed.
    pub head: String,
    /// "Paired · Do it or Ignore this message", for a paired sender's.
    pub paired_line: Option<String>,
    /// "Ana (ana@example.com) wrote about "Refund". Nothing was done."
    pub wrote: String,
    pub text: String,
    pub attachments: Option<String>,
    /// Why the sender can't be trusted, when it can't.
    pub proof_note: Option<String>,
    /// Do it, unless the sender check failed.
    pub do_it: Option<String>,
    /// "Pair Ana" or "Pair them", for a sender who passed the check and isn't paired yet.
    pub pair: Option<String>,
    pub ignore: String,
}

pub fn knock_lines(w: &WordBook, k: &Knock) -> Result<KnockLines, WordsError> {
    let app = crate::words::upper(&label(&k.app));
    let mut head = if k.paired {
        w.get("door.knock.paired_by", &[("app", &app)])?
    } else {
        w.get("door.knock.by", &[("app", &app)])?
    };
    if k.proof == "fail" {
        head = format!("{head} · {}", w.line("door.knock.didnt_check_out")?);
    }
    let who = k.who();
    Ok(KnockLines {
        head,
        paired_line: if k.paired {
            Some(w.line("door.knock.paired_line")?)
        } else {
            None
        },
        wrote: match &k.subject {
            Some(subject) => w.get(
                "door.knock.wrote_about",
                &[("who", &who), ("subject", subject)],
            )?,
            None => w.get("door.knock.wrote", &[("who", &who)])?,
        },
        text: k.text.clone(),
        attachments: if k.attachments > 0 {
            Some(w.count("door.knock.attachments", k.attachments, &[])?)
        } else {
            None
        },
        proof_note: match k.proof.as_str() {
            "fail" => Some(w.get("door.knock.proof_failed", &[("from", &k.from)])?),
            "none" => Some(w.line("door.knock.proof_none")?),
            _ => None,
        },
        do_it: if k.proof != "fail" {
            Some(w.line("door.do_it_button")?)
        } else {
            None
        },
        pair: if k.proof == "pass" && !k.paired {
            Some(match k.name.as_deref().and_then(|n| n.split(' ').next()) {
                Some(first) => w.get("door.knock.pair", &[("name", first)])?,
                None => w.line("door.knock.pair_them")?,
            })
        } else {
            None
        },
        ignore: w.line("door.ignore")?,
    })
}

/// `DoItSheet`'s words for a taf, by its name (`taf?.name ?? Words.Chat.yourTaf`).
#[derive(Debug, Clone, PartialEq)]
pub struct DoItLines {
    pub title: String,
    pub intro: String,
    pub placeholder: String,
    pub default_say: String,
    pub send: String,
    pub sending: String,
    pub cancel: String,
}

pub fn do_it_lines(w: &WordBook, k: &Knock, taf: Option<&str>) -> Result<DoItLines, WordsError> {
    let your_taf = w.line("chat.your_taf")?;
    let it = w.line("chat.it_fallback")?;
    let who = k.who();
    Ok(DoItLines {
        title: w.get("door.do_it.title", &[("taf", taf.unwrap_or(&your_taf))])?,
        intro: w.get(
            "door.do_it.intro",
            &[("who", &who), ("taf", taf.unwrap_or(&it))],
        )?,
        placeholder: w.line("door.what_do")?,
        default_say: w.line("door.do_it.default")?,
        send: w.get("door.send_to", &[("taf", taf.unwrap_or(&your_taf))])?,
        sending: w.line("common.sending")?,
        cancel: w.line("common.cancel")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    const DOORS: &str = r#"{
      "doors": [{"id":"d1","app":"email","kind":"email","label":"Email","open":true,
        "address":"rae@doors.example","proof":"proven","how":"It checks the sender",
        "keys":[{"id":"me@example.com","auto":true},{"id":"x","name":7}],
        "today":2,"limit":50,"resting":false}],
      "apps": [{"id":"email","label":"Email","kind":"email","proof":"proven","how":"It checks the sender"}],
      "knocks": [
        {"id":"k1","door":"d1","app":"email","from":"ana@example.com","name":"Ana Ruiz","proof":"pass",
         "subject":"Refund","text":"Can I get $15 back?","attachments":2,"at":"2026-11-16T09:00:00.120Z"},
        {"id":"k2","door":"d1","app":"email","from":"bo@example.com","proof":"fail","text":"Hi",
         "at":"2026-11-16T09:05:00Z","attachments":"many"},
        {"id":"k3","door":"d1","app":"whatsapp","from":"+33600000000","proof":"none","text":"Order?",
         "at":"2026-11-16T09:06:00Z","paired":true},
        {"id":"k4","door":"d1","app":"email","from":"door","at":"2026-11-16T09:07:00Z","rest":"others","dropped":3},
        {"id":"k5","door":"d1","app":"email","from":"cy@example.com","at":"2026-11-16T08:00:00Z","state":"done"}
      ]
    }"#;

    #[test]
    fn doors_read_as_the_ipad_decodes_them() {
        let d = read(DOORS.as_bytes()).unwrap();
        assert_eq!(d.doors[0].keys[1].name, None);
        assert_eq!(d.knocks.len(), 5);
        assert_eq!(d.knocks[1].attachments, 0);
        assert_eq!(d.knocks[2].proof, "none");
        assert_eq!(d.knocks[3].state, "open");
        // A door without what the iPad needs: none read.
        assert!(read(
            DOORS
                .replace(r#""resting":false"#, r#""resting":"no""#)
                .as_bytes()
        )
        .is_none());
        // A knock without its sender: none read.
        assert!(read(DOORS.replace(r#""from":"bo@example.com","#, "").as_bytes()).is_none());
    }

    #[test]
    fn the_panel_shows_open_messages_strangers_first() {
        let (waiting, paired) = panel(&read(DOORS.as_bytes()).unwrap());
        assert_eq!(
            waiting.iter().map(|k| k.id.as_str()).collect::<Vec<_>>(),
            ["k1", "k2"]
        );
        assert_eq!(
            paired.iter().map(|k| k.id.as_str()).collect::<Vec<_>>(),
            ["k3"]
        );
    }

    #[test]
    fn a_knock_s_lines() {
        let w = words();
        let d = read(DOORS.as_bytes()).unwrap();
        let a = knock_lines(&w, &d.knocks[0]).unwrap();
        assert_eq!(a.head, "● KNOCK · BY EMAIL");
        assert_eq!(
            a.wrote,
            "Ana Ruiz (ana@example.com) wrote about \"Refund\". Nothing was done."
        );
        assert_eq!(
            a.attachments.as_deref(),
            Some("It had 2 attachments, not opened.")
        );
        assert_eq!(a.proof_note, None);
        assert_eq!(a.do_it.as_deref(), Some("Do it"));
        assert_eq!(a.pair.as_deref(), Some("Pair Ana"));
        let b = knock_lines(&w, &d.knocks[1]).unwrap();
        assert_eq!(b.head, "● KNOCK · BY EMAIL · DIDN'T CHECK OUT");
        assert_eq!(b.wrote, "bo@example.com wrote. Nothing was done.");
        assert!(b
            .proof_note
            .unwrap()
            .starts_with("It says it's from bo@example.com"));
        assert_eq!((b.do_it, b.pair), (None, None));
        let c = knock_lines(&w, &d.knocks[2]).unwrap();
        assert_eq!(c.head, "PAIRED · BY WHATSAPP");
        assert_eq!(
            c.paired_line.as_deref(),
            Some("Paired · Do it or Ignore this message")
        );
        assert_eq!(
            c.proof_note.as_deref(),
            Some("This door can't prove who wrote it.")
        );
        assert_eq!(c.pair, None);
    }

    #[test]
    fn do_it_names_the_taf_or_not() {
        let w = words();
        let k = &read(DOORS.as_bytes()).unwrap().knocks[0];
        let named = do_it_lines(&w, k, Some("Rae")).unwrap();
        assert_eq!(named.title, "Tell Rae what to do");
        assert_eq!(named.send, "Send to Rae");
        assert!(named.intro.contains("Rae acts on what you say"));
        let unnamed = do_it_lines(&w, k, None).unwrap();
        assert_eq!(unnamed.title, "Tell your taf what to do");
    }

    #[test]
    fn answers_and_paths() {
        assert_eq!(path("taf a"), "/v1/tafs/taf%20a/doors");
        assert_eq!(knock_path("t", "k/1"), "/v1/tafs/t/doors/knocks/k%2F1");
        assert_eq!(answer_body("ignore", None), br#"{"action":"ignore"}"#);
        assert_eq!(answer_body("do", Some("")), br#"{"action":"do"}"#);
        assert_eq!(
            answer_body("do", Some("Refund it")),
            br#"{"action":"do","say":"Refund it"}"#
        );
        assert!(answered(br#"{"state":"done"}"#));
        assert!(!answered(br#"{}"#));
        assert_eq!(label("slack"), "Slack");
    }
}
