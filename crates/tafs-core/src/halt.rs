//! A halt of one taf or, with none, of every taf (`HaltScreen`, `HaltRow`, `TafsCloud.Halt`).
//!
//! Halting is one tap, no signature: it only takes authority away. tafs cloud halts it in
//! kiff/agents and KIFF at once, and it reads Halted only once kiff/agents (no run starts), KIFF's
//! gateway (it refuses the calls) and planque (it sends nothing) all hold it; Halting… until then.
//! Resuming gives authority back, so it is signed by the device (`tafs.resume.v1`): a Linux
//! computer, whose key is in software, can't, and the screen says so.

use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::words::{WordBook, WordsError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Running,
    Halting,
    Halted,
}

/// Where a halt stands (`TafsCloud.Halt`). Read as the iPad's synthesized decoder: `state` one of
/// the three, and the parts' flags, are required; `halted_at` and `stopped` may be absent.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Halt {
    pub state: State,
    #[serde(default, deserialize_with = "time")]
    pub halted_at: Option<DateTime<Utc>>,
    /// kiff/agents holds it: no run starts.
    pub runtime: bool,
    /// KIFF's gateway holds it: it refuses the calls.
    pub kiff: bool,
    /// planque holds it: it sends nothing to the vendor.
    pub planque: bool,
    /// Halted because the whole account is, not on its own.
    pub account: bool,
    /// How many runs this halt stopped.
    #[serde(default)]
    pub stopped: Option<i64>,
}

// An RFC 3339 time, or none; a time that isn't one fails the read, as the iPad's date decoding.
fn time<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    let s: Option<String> = Option::deserialize(d)?;
    s.map(|s| {
        DateTime::parse_from_rfc3339(&s)
            .map(|t| t.with_timezone(&Utc))
            .map_err(serde::de::Error::custom)
    })
    .transpose()
}

impl Halt {
    /// `.running`: what a halt reads as until it's read.
    pub fn running() -> Halt {
        Halt {
            state: State::Running,
            halted_at: None,
            runtime: false,
            kiff: false,
            planque: false,
            account: false,
            stopped: None,
        }
    }
}

/// tafs cloud's answer (`HaltResponse`: `{ "halt": … }`), or none when it can't be read.
pub fn read(body: &[u8]) -> Option<Halt> {
    #[derive(Deserialize)]
    struct Response {
        halt: Halt,
    }
    serde_json::from_slice::<Response>(body)
        .ok()
        .map(|r| r.halt)
}

/// `haltPath`: a taf's halt by tafs cloud's id, or with none the account's. The id is escaped as
/// one path segment.
pub fn path(taf: Option<&str>) -> String {
    match taf {
        Some(id) => format!("/v1/tafs/{}/halt", crate::access::escaped(id)),
        None => "/v1/halt".to_string(),
    }
}

/// The screen's lines (`HaltScreen.body`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HaltLines {
    /// The page's title: "Halt", or "Halt all".
    pub title: String,
    /// The taf's name, or "All your tafs", over the headline: dim while running, red otherwise.
    pub name: String,
    pub stopped: bool,
    /// "Running", "Halting…" or "Halted" (26 bold).
    pub headline: String,
    pub explanation: String,
    /// "Who holds the halt", once it's halting or halted: each part and whether it holds it.
    pub parts: Option<(String, Vec<Part>)>,
    /// "Halted with all your tafs…", for a taf halted with the account.
    pub with_all: Option<String>,
    pub action: Action,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Part {
    pub text: String,
    pub held: bool,
    /// "Done" or "Waiting".
    pub status: String,
}

/// What the screen offers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// "Halt now", or "Halt all now": one tap.
    Halt(String),
    /// Halting… waiting for every part, and "Halt again", which asks the parts that didn't take it.
    Halting { wait: String, again: String },
    /// Resuming is signed by the device: the button's words, and why this computer can't.
    Resume { button: String, why: String },
    /// A taf halted with the whole account resumes with it: nothing here.
    None,
}

/// `HaltScreen.headline`.
pub fn headline(w: &WordBook, h: &Halt) -> Result<String, WordsError> {
    w.line(match h.state {
        State::Running => "taf.now.running",
        State::Halting => "taf.halt.halting",
        State::Halted => "taf.state.halted",
    })
}

/// `HaltScreen.explanation`: the time it was halted at in the account's zone.
pub fn explanation(w: &WordBook, h: &Halt, all: bool) -> Result<String, WordsError> {
    match h.state {
        State::Running => {
            let who = w.line(if all {
                "halt.your_tafs_lower"
            } else {
                "record.on.it"
            })?;
            w.get("halt.halting_stops_no_run_starts", &[("who", &who)])
        }
        State::Halting => w.line("halt.explain.halting"),
        State::Halted => match h.halted_at {
            Some(t) => w.get(
                "halt.explain.halted_since",
                &[("time", &crate::account_time::short_time(&t))],
            ),
            None => w.line("halt.explain.halted"),
        },
    }
}

/// The screen for a taf (its name) or, with none, the account. Resuming is signed by the device;
/// this computer's key is in software, so its button names what it would do ("Resume Rae",
/// "Resume all your tafs": the iPad's prompt words, as Linux has no biometry to name) and the
/// shared line says why it can't.
pub fn lines(w: &WordBook, h: &Halt, taf: Option<&str>) -> Result<HaltLines, WordsError> {
    let all = taf.is_none();
    let parts = if h.state == State::Running {
        None
    } else {
        let part = |key: &str, held: bool| -> Result<Part, WordsError> {
            Ok(Part {
                text: w.line(key)?,
                held,
                status: w.line(if held {
                    "taf.halt.part_held"
                } else {
                    "taf.halt.part_waiting"
                })?,
            })
        };
        Some((
            w.line("taf.halt.who_holds")?,
            vec![
                part("taf.halt.no_run_starts", h.runtime)?,
                part("taf.halt.kiff_refuses", h.kiff)?,
                part("taf.halt.connection_sends_nothing", h.planque)?,
            ],
        ))
    };
    let with_account = h.account && !all;
    let action = match h.state {
        State::Running => Action::Halt(w.line(if all { "halt.all_now" } else { "halt.now" })?),
        State::Halting => Action::Halting {
            wait: w.line("taf.halt.halting_wait")?,
            again: w.line("halt.again")?,
        },
        State::Halted if with_account => Action::None,
        State::Halted => Action::Resume {
            button: match taf {
                Some(name) => w.get("taf.halt.resume_prompt", &[("taf", name)])?,
                None => w.line("taf.halt.resume_all_prompt")?,
            },
            why: w.line("approval.device_can_not_approve_approve")?,
        },
    };
    Ok(HaltLines {
        title: w.line(if all {
            "taf.halt.title_all"
        } else {
            "taf.halt.title"
        })?,
        name: match taf {
            Some(name) => name.to_string(),
            None => w.line("taf.halt.all_tafs")?,
        },
        stopped: h.state != State::Running,
        headline: headline(w, h)?,
        explanation: explanation(w, h, all)?,
        with_all: (h.state != State::Running && with_account)
            .then(|| w.line("taf.halt.halted_with_all"))
            .transpose()?,
        parts,
        action,
    })
}

/// `HaltRow`: its title, and where the halt stands ("Stop it until you resume", else the
/// headline, in red).
pub fn row(w: &WordBook, h: &Halt, all: bool) -> Result<(String, String, bool), WordsError> {
    let title = w.line(if all {
        "taf.halt.title_all"
    } else {
        "taf.halt.title"
    })?;
    Ok(if h.state == State::Running {
        (title, w.line("halt.row_running")?, false)
    } else {
        (title, headline(w, h)?, true)
    })
}

/// "tafs couldn't halt it. {why}", or "… your tafs. {why}".
pub fn not_halted(w: &WordBook, all: bool, why: &str) -> Result<String, WordsError> {
    w.get(
        if all {
            "taf.halt.not_halted_all"
        } else {
            "taf.halt.not_halted"
        },
        &[("why", why)],
    )
}

/// While it's halting, the screen reads it again every 2 seconds, at most 60 times.
pub const POLL: std::time::Duration = std::time::Duration::from_secs(2);
pub const POLLS: usize = 60;
