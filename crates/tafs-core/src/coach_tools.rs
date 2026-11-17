//! Coach's tools still to give (`TafsCloud.CoachTools`, `CoachToolsRow`, `CoachToolsSheet`,
//! `CoachToolsGiven`): what tafs cloud reads from KIFF of Coach's tools (`GET /v1/setup`'s
//! `tools`), as one line above the conversation that opens the step: the step's words, each tool
//! under a few groups, and Coach's Cards in KIFF's words. Giving them is signed, behind one
//! fingerprint on the iPad, which wraps the button in `ApprovalGate`: here the gate's line stands
//! in its place, and nothing is sent. Read as leniently as the iPad's `JSONSerialization` read:
//! a field of another shape is none, an item that isn't an object is left out.

use serde_json::{Map, Value};

use crate::coach::field;
use crate::words::{WordBook, WordsError};

/// `CoachTools.Step.Tool`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Tool {
    pub name: String,
    pub does: String,
    pub why: String,
}

/// `CoachTools.Step.Group`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Group {
    pub name: String,
    pub line: String,
    pub tools: Vec<Tool>,
}

/// `CoachTools.Step`: the step's words, every one tafs cloud's.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Step {
    pub title: String,
    pub intro: String,
    pub groups: Vec<Group>,
    pub details: String,
    pub ask: String,
    pub give: String,
    pub not_now: String,
    pub row: String,
}

/// `CoachTools.CardDraft.Words`: one of Coach's Cards in KIFF's words.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CardWords {
    pub title: String,
    pub lines: Vec<String>,
}

/// `CoachTools.CardDraft`, as far as its words go: the body is the phone's to sign.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CardDraft {
    pub card_title: Option<String>,
    pub cards: Vec<CardWords>,
}

/// `CoachTools`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachTools {
    /// `given`, `to_give`, `taken` or `unknown`.
    pub state: String,
    /// With `taken`: the name a vendor's tool holds.
    pub taken: Option<String>,
    /// The batch's line per tool, when tafs cloud sends the batch.
    pub connect: Option<Vec<String>>,
    pub card: Option<CardDraft>,
    pub step: Option<Step>,
}

/// A line as written, or none where it's missing or isn't text (`Step.text`).
fn said(o: &Map<String, Value>, key: &str) -> String {
    o.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|s| s.as_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default()
}

/// KIFF's words for the Cards, each with a title or lines: one with neither is left out.
fn card_words(v: Option<&Value>) -> Vec<CardWords> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|c| {
                    let c = c.as_object()?;
                    let title = ["title", "card_title", "label"]
                        .iter()
                        .find_map(|k| c.get(*k).and_then(Value::as_str));
                    let lines = strings(c.get("lines"));
                    if title.is_none() && lines.is_empty() {
                        return None;
                    }
                    Some(CardWords {
                        title: title.unwrap_or_default().to_string(),
                        lines,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn step(v: &Value) -> Option<Step> {
    let o = v.as_object()?;
    // `LenientItem`: a group or tool that isn't an object is left out, and so is a tool with no
    // name the app can read.
    let groups = o
        .get("groups")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_object)
                .map(|g| Group {
                    name: said(g, "name"),
                    line: said(g, "line"),
                    tools: g
                        .get("tools")
                        .and_then(Value::as_array)
                        .map(|t| {
                            t.iter()
                                .filter_map(Value::as_object)
                                .map(|t| Tool {
                                    name: said(t, "name"),
                                    does: said(t, "does"),
                                    why: said(t, "why"),
                                })
                                .filter(|t| !t.name.is_empty())
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Step {
        title: said(o, "title"),
        intro: said(o, "intro"),
        groups,
        details: said(o, "details"),
        ask: said(o, "ask"),
        give: said(o, "give"),
        not_now: said(o, "not_now"),
        row: said(o, "row"),
    })
}

/// `CoachStart.parse`'s pair, as far as the tools step reads it: Coach's missing tools and Cards
/// as one pair to sign once, with KIFF's words for the Cards. None unless it's a pair the phone
/// may sign (`together`, `startable`).
fn pair_card(tools: &Map<String, Value>, state: &str) -> Option<CardDraft> {
    let requests = tools.get("requests")?.as_array()?;
    let request = |id: &str| {
        requests.iter().filter_map(Value::as_object).find(|r| {
            r.get("id").and_then(Value::as_str) == Some(id)
                && r.get("method").and_then(Value::as_str).is_some()
                && r.get("path").and_then(Value::as_str).is_some()
                && field(r, "body").is_some()
        })
    };
    let (connect, draft) = (request("connect")?, request("draft")?);
    let s = |r: &Map<String, Value>, k: &str| r.get(k).and_then(Value::as_str).map(str::to_string);
    let gateway = |r: &Map<String, Value>| r.get("gateway").and_then(Value::as_bool) == Some(true);
    let cards = card_words(draft.get("cards"));
    let together = s(tools, "scheme").as_deref() == Some("kiff.requests.v1");
    let startable = state == "to_give"
        && s(draft, "method").as_deref() == Some("POST")
        && s(draft, "path").as_deref() == Some("/v1/me/agent-drafts")
        && !gateway(draft)
        && !cards.is_empty()
        && s(connect, "method").as_deref() == Some("POST")
        && s(connect, "path").as_deref() == Some("/v1/connections/batch")
        && gateway(connect);
    (together && startable).then(|| CardDraft {
        card_title: s(draft, "card_title"),
        cards,
    })
}

impl CoachTools {
    /// `CoachTools.read`: from `GET /v1/setup`'s answer; none from a tafs cloud that sends no
    /// `tools`.
    pub fn read(body: &[u8]) -> Option<CoachTools> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let t = v.as_object()?.get("tools")?.as_object()?;
        let state = t
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let connect = t
            .get("connect")
            .and_then(Value::as_object)
            .filter(|c| field(c, "body").is_some())
            .map(|c| strings(c.get("lines")));
        let mut card = t
            .get("card")
            .and_then(Value::as_object)
            .filter(|c| field(c, "body").is_some())
            .map(|c| CardDraft {
                card_title: c
                    .get("card_title")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                cards: card_words(c.get("cards")),
            });
        if card.is_none() {
            card = pair_card(t, &state);
        }
        Some(CoachTools {
            taken: t.get("taken").and_then(Value::as_str).map(str::to_string),
            connect,
            card,
            step: t.get("coach_tools_step").and_then(step),
            state,
        })
    }

    pub fn given(&self) -> bool {
        self.state == "given"
    }

    pub fn to_give(&self) -> bool {
        self.state == "to_give"
    }
}

/// `CoachToolsSheet.said`: tafs cloud's words where it sends them; the app's own only for a tafs
/// cloud from before them.
fn said_or(server: &str, w: &WordBook, own: &str) -> Result<String, WordsError> {
    if server.trim().is_empty() {
        w.line(own)
    } else {
        Ok(server.to_string())
    }
}

/// `CoachToolsRow`: its words, or none once the tools are given (or before tafs cloud said).
pub fn row(w: &WordBook, tools: Option<&CoachTools>) -> Result<Option<String>, WordsError> {
    match tools {
        Some(t) if !t.given() => {
            let row = t.step.as_ref().map(|s| s.row.as_str()).unwrap_or_default();
            said_or(row, w, "coach.tools.title").map(Some)
        }
        _ => Ok(None),
    }
}

/// One line of the step, in its size, weight and colour.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Said {
    pub words: String,
    pub size: u32,
    pub weight: u16,
    pub soft: bool,
}

fn line(words: &str, size: u32, weight: u16, soft: bool) -> Said {
    Said {
        words: words.to_string(),
        size,
        weight,
        soft,
    }
}

/// `CoachToolsSheet` as drawn: its title, what's above the fold, the fold's words and what's under
/// it (`CoachToolsGiven.stepBody`), the ask, whether the give button's place is shown, and Not now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SheetLines {
    pub title: String,
    pub lines: Vec<Said>,
    /// "Each tool and its limits", which opens the fold; none when nothing is folded.
    pub details: Option<String>,
    pub folded: Vec<Said>,
    pub ask: Option<Said>,
    /// `to_give`: the give button's place, where this computer says why it can't.
    pub gives: bool,
    pub not_now: String,
}

/// `CoachToolsSheet.body` for `tools`.
pub fn sheet(w: &WordBook, tools: &CoachTools) -> Result<SheetLines, WordsError> {
    let s = tools.step.clone().unwrap_or_default();
    let mut out = SheetLines {
        title: said_or(&s.title, w, "coach.tools.title")?,
        lines: vec![],
        details: None,
        folded: vec![],
        ask: None,
        gives: false,
        not_now: said_or(&s.not_now, w, "common.not_now")?,
    };
    match tools.state.as_str() {
        "taken" => {
            out.lines.push(line(
                &match &tools.taken {
                    Some(n) => w.get("coach.taken.named", &[("name", n)])?,
                    None => w.line("coach.taken.unnamed")?,
                },
                15,
                400,
                false,
            ));
        }
        "to_give" => {
            let step = tools.step.as_ref();
            let grouped = step.is_some_and(|s| !s.groups.is_empty());
            if let Some(st) = step.filter(|_| grouped).filter(|s| !s.intro.is_empty()) {
                out.lines.push(line(&st.intro, 15, 400, true));
            }
            // A few groups up front; each tool and the Cards under the fold.
            if let Some(st) = step.filter(|_| grouped) {
                for g in &st.groups {
                    out.lines.push(line(&g.name, 15, 600, false));
                    if !g.line.is_empty() {
                        out.lines.push(line(&g.line, 14, 400, true));
                    }
                }
                if !st.details.is_empty() {
                    out.details = Some(st.details.clone());
                }
            }
            let mut under = vec![];
            if let Some(st) = step.filter(|_| grouped) {
                for g in &st.groups {
                    if !g.tools.is_empty() {
                        under.push(line(&g.name, 13, 600, false));
                    }
                    for t in &g.tools {
                        under.push(line(&t.name, 14, 600, false));
                        if !t.does.is_empty() {
                            under.push(line(&t.does, 13, 400, false));
                        }
                        if !t.why.is_empty() {
                            under.push(line(&t.why, 13, 400, true));
                        }
                    }
                }
            } else if let Some(lines) = &tools.connect {
                under.extend(lines.iter().map(|l| line(l, 14, 400, false)));
            }
            if let Some(card) = &tools.card {
                if let Some(t) = card.card_title.as_ref().filter(|t| !t.is_empty()) {
                    under.push(line(t, 14, 600, false));
                }
                for c in &card.cards {
                    if !c.title.is_empty() {
                        under.push(line(&c.title, 13, 600, false));
                    }
                    under.extend(c.lines.iter().map(|l| line(l, 13, 400, true)));
                }
            }
            // Open where the step has no fold.
            if out.details.is_some() {
                out.folded = under;
            } else {
                out.lines.extend(under);
            }
            if let Some(ask) = step.map(|s| s.ask.as_str()).filter(|a| !a.is_empty()) {
                out.ask = Some(line(ask, 15, 600, false));
            }
            out.gives = true;
        }
        _ => out
            .lines
            .push(line(&w.line("coach.tools_unknown")?, 15, 400, false)),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn book() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    fn contract() -> Vec<u8> {
        let path = format!(
            "{}/../../shared/contracts/setup-coach.json",
            env!("CARGO_MANIFEST_DIR")
        );
        let v: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
        serde_json::to_vec(&v["answer"]).unwrap()
    }

    #[test]
    fn the_contracts_tools_read_as_the_ipad_reads_them() {
        let w = book();
        let t = CoachTools::read(&contract()).unwrap();
        assert!(t.to_give() && !t.given());
        let step = t.step.as_ref().unwrap();
        assert_eq!(step.title, "Give Coach its tools");
        assert_eq!(step.groups[0].name, "Watch your tafs");
        assert_eq!(step.groups[0].tools[0].name, "See your tafs");
        assert_eq!(
            t.connect.as_ref().unwrap()[0],
            "See your tafs, their versions and their runs"
        );
        assert_eq!(
            row(&w, Some(&t)).unwrap().as_deref(),
            Some("Coach's tools are waiting for you · Give them")
        );
        let s = sheet(&w, &t).unwrap();
        assert_eq!(s.title, "Give Coach its tools");
        assert_eq!(
            s.lines[0].words,
            "Coach needs these to look after your tafs."
        );
        assert_eq!(s.lines[1].words, "Watch your tafs");
        assert_eq!(s.details.as_deref(), Some("Each tool and its limits"));
        assert_eq!(s.folded[0].words, "Watch your tafs");
        assert_eq!(s.folded[1].words, "See your tafs");
        assert!(s.gives);
        assert_eq!(s.not_now, "Not now");
    }

    #[test]
    fn nothing_once_given_or_before_tafs_cloud_says() {
        let w = book();
        assert_eq!(row(&w, None).unwrap(), None);
        let given = CoachTools::read(br#"{"setup":null,"tools":{"state":"given"}}"#).unwrap();
        assert!(given.given());
        assert_eq!(row(&w, Some(&given)).unwrap(), None);
        assert_eq!(CoachTools::read(br#"{"setup":null}"#), None);
        assert_eq!(CoachTools::read(br#"{"tools":"to_give"}"#), None);
    }

    #[test]
    fn the_apps_own_words_stand_in_only_for_a_tafs_cloud_without_them() {
        let w = book();
        let t = CoachTools::read(
            br#"{"tools":{"state":"to_give","connect":{"body":{},"lines":["Read a run",3]},
                 "card":{"body":{},"card_title":"Coach's Card",
                         "cards":[{"label":"Watching","lines":["500 reads a day"]},{"x":1},"no"]}}}"#,
        )
        .unwrap();
        assert_eq!(row(&w, Some(&t)).unwrap().as_deref(), Some("Coach's tools"));
        let s = sheet(&w, &t).unwrap();
        assert_eq!(s.title, "Coach's tools");
        assert_eq!(s.details, None);
        let words: Vec<&str> = s.lines.iter().map(|l| l.words.as_str()).collect();
        assert_eq!(
            words,
            ["Read a run", "Coach's Card", "Watching", "500 reads a day"]
        );
        assert_eq!(s.not_now, "Not now");
    }

    #[test]
    fn taken_and_unknown_say_why() {
        let w = book();
        let t = CoachTools::read(br#"{"tools":{"state":"taken","taken":"taf_run"}}"#).unwrap();
        let s = sheet(&w, &t).unwrap();
        assert!(s.lines[0].words.contains("taf_run"));
        assert!(!s.gives);
        let t = CoachTools::read(br#"{"tools":{"state":7}}"#).unwrap();
        assert_eq!(t.state, "unknown");
        let s = sheet(&w, &t).unwrap();
        assert_eq!(
            s.lines[0].words,
            "KIFF didn't answer about Coach's tools. Try again in a moment."
        );
    }

    #[test]
    fn a_pair_to_sign_once_gives_its_cards_words() {
        let pair = json!({"tools": {"state": "to_give", "scheme": "kiff.requests.v1", "requests": [
            {"id": "connect", "method": "POST", "path": "/v1/connections/batch", "gateway": true, "body": {}},
            {"id": "draft", "method": "POST", "path": "/v1/me/agent-drafts", "body": {},
             "card_title": "Coach's Card", "cards": [{"title": "Watching", "lines": ["500 a day"]}]}
        ]}});
        let t = CoachTools::read(pair.to_string().as_bytes()).unwrap();
        let card = t.card.unwrap();
        assert_eq!(card.card_title.as_deref(), Some("Coach's Card"));
        assert_eq!(card.cards[0].lines, ["500 a day"]);
        // Not one the phone may sign: no Cards drawn from it.
        let mut odd = pair.clone();
        odd["tools"]["requests"][0]["path"] = json!("/v1/elsewhere");
        assert_eq!(
            CoachTools::read(odd.to_string().as_bytes()).unwrap().card,
            None
        );
    }

    #[test]
    fn a_lenient_step_leaves_out_what_it_cant_read() {
        let t = CoachTools::read(
            br#"{"tools":{"state":"to_give","coach_tools_step":{"title":5,"groups":[
                "no",{"name":"G","tools":[{"name":""},{"name":"T","does":1},7]}]}}}"#,
        )
        .unwrap();
        let s = t.step.unwrap();
        assert_eq!(s.title, "");
        assert_eq!(s.groups.len(), 1);
        assert_eq!(
            s.groups[0].tools,
            [Tool {
                name: "T".into(),
                ..Default::default()
            }]
        );
    }

    #[test]
    fn giving_them_is_never_sent_unsigned() {
        // The step's two requests are signed: neither can be made as the session.
        assert!(
            crate::signed::Unsigned::new("POST", "/v1/me/agent-drafts".into(), b"{}".to_vec())
                .is_none()
        );
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            let t = CoachTools::read(&contract()).unwrap();
            sheet(&w, &t).unwrap();
            row(&w, Some(&t)).unwrap();
        }
    }
}
