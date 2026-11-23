//! A taf's versions and its trial (`VersionsPanel`, `TrialViews.swift`, `TafsCloud`'s Versions and
//! Trying a version): read from tafs cloud as the session (`GET /v1/tafs/{id}/versions`,
//! `GET /v1/tafs/{id}/trial`), drawn as the iPad draws them. What changes a version or a trial is
//! signed by the phone, so this computer draws it up to the signature and sends nothing: a switch
//! (`Install this version`, `Finish`, `Draft its Card again`, `Stop it and install`), a trial's start
//! (`POST …/versions/{n}/try` makes a KIFF draft only the phone confirms), its Install and its Go
//! back (KIFF's trial draft ended with the setup signer). Cancelling a switch, reading the trial
//! and chatting with it are the session's.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};

use crate::access::escaped;
use crate::words::{WordBook, WordsError};

fn time<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
    let s = String::deserialize(d)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

fn optional_time<'de, D: Deserializer<'de>>(d: D) -> Result<Option<DateTime<Utc>>, D::Error> {
    match Option::<String>::deserialize(d)? {
        None => Ok(None),
        Some(s) => DateTime::parse_from_rfc3339(&s)
            .map(|t| Some(t.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
    }
}

/// One version of a taf (`TafsCloud.Version`), kept like an app build.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Version {
    pub n: i64,
    pub note: Option<String>,
    pub by: Option<String>,
    /// Who made it, in tafs cloud's words; none for one kiff/agents wrote itself.
    #[serde(default)]
    pub by_words: Option<String>,
    #[serde(deserialize_with = "time")]
    pub created: DateTime<Utc>,
    pub live: bool,
    /// The KIFF draft its Card revision waits on, while unconfirmed.
    pub draft: Option<String>,
    /// Its trials that ended, oldest first.
    #[serde(default)]
    pub trials: Option<Vec<TrialSummary>>,
}

/// A switch to version `n` under way: nothing of the taf runs until it ends.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Switching {
    pub n: i64,
    pub draft: Option<String>,
    #[serde(deserialize_with = "time")]
    pub at: DateTime<Utc>,
}

/// The taf's versions, oldest first, and a switch under way (`VersionState`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct VersionState {
    pub versions: Vec<Version>,
    pub switching: Option<Switching>,
}

/// The taf's open trial (`OpenTrial`): one per taf.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct OpenTrial {
    pub id: String,
    pub n: i64,
    pub draft: String,
    pub session: Option<String>,
    #[serde(deserialize_with = "time")]
    pub started: DateTime<Utc>,
    #[serde(deserialize_with = "time")]
    pub used: DateTime<Utc>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub changed: Option<String>,
    #[serde(default, rename = "try")]
    pub try_with: Option<String>,
    #[serde(default, deserialize_with = "optional_time")]
    pub ends: Option<DateTime<Utc>>,
}

/// What a version keeps of a trial once it ended.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct TrialSummary {
    #[serde(deserialize_with = "time")]
    pub started: DateTime<Utc>,
    #[serde(deserialize_with = "time")]
    pub ended: DateTime<Utc>,
    /// installed, back, idle or replaced.
    pub why: String,
    pub calls: i64,
    pub held: i64,
    pub refused: i64,
}

pub fn versions_path(taf: &str) -> String {
    format!("/v1/tafs/{}/versions", escaped(taf))
}

/// `cancelSwitch`: the switch under way dropped, as the session.
pub fn cancel_switch_path(taf: &str) -> String {
    format!("/v1/tafs/{}/switching", escaped(taf))
}

pub fn trial_path(taf: &str) -> String {
    format!("/v1/tafs/{}/trial", escaped(taf))
}

/// `trialDraft`: the KIFF draft tafs cloud prepares as version n's trial Card, as the session.
pub fn try_path(taf: &str, n: i64) -> String {
    format!("/v1/tafs/{}/versions/{n}/try", escaped(taf))
}

/// `trialDraft`'s answer: the draft's id; none when it isn't one.
pub fn read_try(body: &[u8]) -> Option<String> {
    #[derive(Deserialize)]
    struct Answer {
        draft: String,
    }
    serde_json::from_slice::<Answer>(body)
        .ok()
        .map(|a| a.draft)
        .filter(|d| !d.trim().is_empty())
}

/// `openTrial(_:version:draft:)`'s body: the version and the draft KIFF now has trying. tafs
/// cloud opens the trial only once KIFF says so (409 before that).
pub fn open_trial_body(n: i64, draft: &str) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({ "n": n, "draft": draft })).unwrap_or_default()
}

/// `openTrial`'s answer, `{"trial": …}`: the trial now open.
pub fn read_opened(body: &[u8]) -> Option<OpenTrial> {
    #[derive(Deserialize)]
    struct Answer {
        trial: OpenTrial,
    }
    serde_json::from_slice::<Answer>(body).ok().map(|a| a.trial)
}

/// `installTrial`: the version tried goes live and the trial ends, once KIFF has its draft
/// confirmed live, as the session.
pub fn install_path(taf: &str) -> String {
    format!("/v1/tafs/{}/trial/install", escaped(taf))
}

/// The trial's chat, read as a taf's thread is.
pub fn trial_conversation_path(taf: &str) -> String {
    format!("/v1/tafs/{}/trial/conversation", escaped(taf))
}

/// One message to the trial, in its own chat.
pub fn trial_messages_path(taf: &str) -> String {
    format!("/v1/tafs/{}/trial/messages", escaped(taf))
}

/// `versionState`'s answer; none when it can't be read as the iPad reads it.
pub fn read_versions(body: &[u8]) -> Option<VersionState> {
    serde_json::from_slice(body).ok()
}

/// `openTrial(_:)`'s answer: the open trial, or none open. None when it can't be read.
pub fn read_trial(body: &[u8]) -> Option<Option<OpenTrial>> {
    #[derive(Deserialize)]
    struct Answer {
        trial: Option<OpenTrial>,
    }
    serde_json::from_slice::<Answer>(body).ok().map(|a| a.trial)
}

/// `trialMessage`'s answer, `{session, run}`, as a message taken: its chat, and no id of its own
/// (the thread read next names it).
pub fn trial_sent(body: &[u8]) -> Option<crate::coach_send::Sent> {
    #[derive(Deserialize)]
    struct Answer {
        session: String,
        #[allow(dead_code)]
        run: String,
    }
    let a: Answer = serde_json::from_slice(body).ok()?;
    Some(crate::coach_send::Sent {
        session: a.session,
        new_session: false,
        message: None,
    })
}

/// A day as the iPad's abbreviated date writes it, in the account's zone.
fn date(w: &WordBook, at: &DateTime<Utc>) -> String {
    crate::record::short_date(w.language(), crate::account_time::day(at))
}

/// What a version's row offers (`VersionsPanel.row`): nothing for the live one; Resume the trial
/// for the one being tried; else Try it, and Install this version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Offers {
    None,
    Resume(String),
    TryAndInstall {
        try_it: String,
        /// One trial per taf: another waits for the open one to end.
        try_open: bool,
        install: String,
        /// One switch at a time, and none while a trial is open.
        install_open: bool,
    },
}

/// One version's row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub n: i64,
    pub number: String,
    /// "Live now" (green) or "Trying" (amber).
    pub live: Option<String>,
    pub trying: Option<String>,
    pub note: Option<String>,
    pub byline: String,
    /// What each ended trial came to, one line each.
    pub tried: Vec<String>,
    pub offers: Offers,
}

/// A switch that didn't finish: its title, why the taf waits, Finish and Cancel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchLines {
    pub n: i64,
    pub title: String,
    pub waits: String,
    /// Finish, when its version is listed.
    pub finish: Option<String>,
    pub cancel: String,
}

/// `VersionsPanel`'s lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionsLines {
    pub title: String,
    pub intro: String,
    pub switching: Option<SwitchLines>,
    /// "One version so far…" when tafs cloud lists none.
    pub first_only: Option<String>,
    /// Newest first.
    pub rows: Vec<Row>,
}

/// `VersionsPanel.byline`: who made it, when, and whether it brings a new Card.
pub fn byline(w: &WordBook, v: &Version) -> Result<String, WordsError> {
    let when = format!(
        "{} {}",
        date(w, &v.created),
        crate::account_time::short_time(&v.created)
    );
    let mut parts: Vec<String> = Vec::new();
    if let Some(by) = v.by_words.as_ref().filter(|b| !b.is_empty()) {
        parts.push(by.clone());
    }
    parts.push(when);
    if v.draft.is_some() {
        parts.push(w.line("version.brings_new_card")?);
    }
    Ok(parts.join(" · "))
}

/// `VersionsPanel.tried`: "Tried 21 Oct · 3 calls, 1 held · went back".
pub fn tried(w: &WordBook, t: &TrialSummary) -> Result<String, WordsError> {
    let mut counts = vec![w.count("version.tried.calls", t.calls, &[])?];
    if t.held > 0 {
        counts.push(w.count("version.tried.held", t.held, &[])?);
    }
    if t.refused > 0 {
        counts.push(w.count("version.tried.refused", t.refused, &[])?);
    }
    let how = w.line(match t.why.as_str() {
        "installed" => "version.tried.installed",
        "back" => "version.tried.went_back",
        "idle" => "version.tried.ended_unused",
        _ => "version.tried.ended",
    })?;
    Ok([
        w.get("version.tried.on", &[("date", &date(w, &t.started))])?,
        counts.join(", "),
        how,
    ]
    .join(" · "))
}

/// The panel, from tafs cloud's versions and the open trial.
pub fn lines(
    w: &WordBook,
    state: &VersionState,
    trial: Option<&OpenTrial>,
) -> Result<VersionsLines, WordsError> {
    let switching = match &state.switching {
        Some(sw) => Some(SwitchLines {
            n: sw.n,
            title: w.get(
                "version.installing_number",
                &[("version", &sw.n.to_string())],
            )?,
            waits: w.line("version.doesnt_run_until")?,
            finish: state
                .versions
                .iter()
                .any(|v| v.n == sw.n)
                .then(|| w.line("version.finish"))
                .transpose()?,
            cancel: w.line("common.cancel")?,
        }),
        None => None,
    };
    let mut rows = Vec::new();
    for v in state.versions.iter().rev() {
        let trying = trial.is_some_and(|t| t.n == v.n);
        let offers = if v.live {
            Offers::None
        } else if trying {
            Offers::Resume(w.line("version.resume_trial")?)
        } else {
            Offers::TryAndInstall {
                try_it: w.line("version.try_it")?,
                try_open: trial.is_none(),
                install: w.line("version.install_this")?,
                install_open: trial.is_none()
                    && state.switching.as_ref().is_none_or(|sw| sw.n == v.n),
            }
        };
        rows.push(Row {
            n: v.n,
            number: w.get("version.number", &[("version", &v.n.to_string())])?,
            live: v.live.then(|| w.line("version.live_now")).transpose()?,
            trying: (!v.live && trying)
                .then(|| w.line("version.trying"))
                .transpose()?,
            note: v.note.clone().filter(|n| !n.is_empty()),
            byline: byline(w, v)?,
            tried: v
                .trials
                .iter()
                .flatten()
                .map(|t| tried(w, t))
                .collect::<Result<_, _>>()?,
            offers,
        });
    }
    Ok(VersionsLines {
        title: w.line("version.title")?,
        intro: w.line("taf.try_version_first_you_chat")?,
        switching,
        first_only: state
            .versions
            .is_empty()
            .then(|| w.line("version.first_only"))
            .transpose()?,
        rows,
    })
}

/// The link on a taf's page to its versions: "Version 3 · live", else "Versions"; and, with a
/// trial open, "Trying v4 · Resume".
pub fn link(
    w: &WordBook,
    state: Option<&VersionState>,
    trial: Option<&OpenTrial>,
) -> Result<(String, Option<String>), WordsError> {
    let live = state.and_then(|s| s.versions.iter().find(|v| v.live).map(|v| v.n));
    let title = match live {
        Some(n) => w.get("version.live_number", &[("version", &n.to_string())])?,
        None => w.line("version.title")?,
    };
    let resume = trial
        .map(|t| w.get("taf.trying_resume", &[("n", &t.n.to_string())]))
        .transpose()?;
    Ok((title, resume))
}

/// `TrialStartSheet`: its title, and what trying a version means. Its Card can't be shown here:
/// reading it makes the KIFF draft the phone would confirm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartLines {
    pub title: String,
    pub heading: String,
    pub while_trying: String,
    pub lines: Vec<String>,
    pub start: String,
}

pub fn start_lines(w: &WordBook, v: &Version) -> Result<StartLines, WordsError> {
    let n = v.n.to_string();
    Ok(StartLines {
        title: w.line("version.try_it")?,
        heading: match v.note.as_ref().filter(|n| !n.is_empty()) {
            Some(note) => w.get(
                "version.try_number_note",
                &[("version", &n), ("note", note)],
            )?,
            None => w.get("version.try_number", &[("version", &n)])?,
        },
        while_trying: w.line("version.while_trying")?,
        lines: vec![
            w.get("version.while_you_try.chat", &[("n", &n)])?,
            w.line("version.while_you_try.live_keeps_working")?,
            w.line("version.while_you_try.counts_from_zero")?,
            w.line("version.while_you_try.ends")?,
        ],
        start: w.line("version.start_trying")?,
    })
}

/// `TrialScreen`'s bar and guide: "Trying version 4", "not live", Go back and Install with their
/// asks, and, before its first message, what the trial is and what to try.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrialLines {
    pub trying: String,
    pub not_live: String,
    pub go_back: String,
    pub go_back_title: String,
    pub go_back_body: String,
    pub install: String,
    pub install_title: String,
    pub install_body: String,
    pub guide: Vec<String>,
    pub guide_foot: String,
}

pub fn trial_lines(w: &WordBook, t: &OpenTrial, name: &str) -> Result<TrialLines, WordsError> {
    let n = t.n.to_string();
    let mut guide = Vec::new();
    if let Some(changed) = t.changed.as_ref().filter(|c| !c.is_empty()) {
        guide.push(w.get("version.guide.whats_new", &[("changed", changed)])?);
    }
    guide.push(match t.try_with.as_ref().filter(|c| !c.is_empty()) {
        Some(with) => w.get("version.guide.try_with", &[("try_with", with)])?,
        None => w.get("version.guide.give_task", &[("name", name)])?,
    });
    let mut foot = vec![w.get("version.guide.keeps_working", &[("name", name)])?];
    if let Some(ends) = &t.ends {
        foot.push(w.get("version.guide.trial_ends", &[("date", &date(w, ends))])?);
    }
    foot.push(w.line("version.guide.install_or_back")?);
    Ok(TrialLines {
        trying: w.get("version.trial.trying", &[("n", &n)])?,
        not_live: w.line("version.not_live_tag")?,
        go_back: w.line("version.go_back")?,
        go_back_title: w.line("version.go_back_title")?,
        go_back_body: w.get("version.go_back_body", &[("version", &n), ("taf", name)])?,
        install: w.line("version.install")?,
        install_title: w.get("version.install_title", &[("version", &n)])?,
        install_body: w.get("version.install_body", &[("version", &n)])?,
        guide,
        guide_foot: foot.join(" "),
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_trial_is_tried_opened_and_installed_at_tafs_clouds_own_paths() {
        assert_eq!(try_path("taf 1", 3), "/v1/tafs/taf%201/versions/3/try");
        assert_eq!(read_try(br#"{"draft":"drf_9"}"#), Some("drf_9".into()));
        assert_eq!(read_try(br#"{"draft":" "}"#), None);
        assert_eq!(read_try(b"{}"), None);
        assert_eq!(
            open_trial_body(3, "drf_9"),
            br#"{"draft":"drf_9","n":3}"#.to_vec()
        );
        let opened = read_opened(
            br#"{"trial":{"id":"tr1","n":3,"draft":"drf_9","started":"2026-11-23T04:00:00Z","used":"2026-11-23T04:00:00Z"}}"#,
        )
        .unwrap();
        assert_eq!((opened.n, opened.draft.as_str()), (3, "drf_9"));
        assert_eq!(read_opened(b"{}"), None);
        assert_eq!(install_path("t1"), "/v1/tafs/t1/trial/install");
        // None of them is a route the device signs: tafs cloud checks KIFF's draft instead.
        for (m, p) in [
            ("POST", try_path("t1", 3)),
            ("PUT", trial_path("t1")),
            ("POST", install_path("t1")),
        ] {
            assert!(!crate::signed::signed_at_cloud(m, &p), "{m} {p}");
        }
    }

    use super::*;

    fn en() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    const STATE: &str = r#"{"versions":[
        {"n":1,"note":null,"by":"setup","created":"2026-11-01T09:00:00Z","live":false,"draft":null,
         "trials":[{"started":"2026-11-02T09:00:00Z","ended":"2026-11-03T09:00:00Z","why":"back","calls":3,"held":1,"refused":0}]},
        {"n":2,"note":"Refunds up to €200","by":"auth0|rae","by_words":"By you","created":"2026-11-10T14:05:00Z","live":true,"draft":null},
        {"n":3,"note":"Asks before shipping","by":"auth0|rae","by_words":"By Coach mode","created":"2026-11-16T08:30:00.000Z","live":false,"draft":"drf_3"}],
        "switching":null}"#;

    #[test]
    fn versions_read_as_the_ipad_draws_them() {
        let w = en();
        let s = read_versions(STATE.as_bytes()).unwrap();
        let l = lines(&w, &s, None).unwrap();
        assert_eq!(l.rows.len(), 3);
        // Newest first; the live one marked, and offering nothing.
        assert_eq!(l.rows[0].number, "Version 3");
        assert_eq!(l.rows[1].live.as_deref(), Some("Live now"));
        assert_eq!(l.rows[1].offers, Offers::None);
        assert!(l.rows[0].byline.starts_with("By Coach mode · "));
        assert!(l.rows[0].byline.ends_with(" · brings a new Card"));
        assert_eq!(l.rows[2].tried.len(), 1);
        assert!(l.rows[2].tried[0].starts_with("Tried "));
        assert!(l.rows[2].tried[0].ends_with(" · 3 calls, 1 held · went back"));
        assert!(matches!(
            l.rows[0].offers,
            Offers::TryAndInstall {
                try_open: true,
                install_open: true,
                ..
            }
        ));
        assert_eq!(
            link(&w, Some(&s), None).unwrap(),
            ("Version 2 · live".into(), None)
        );
    }

    #[test]
    fn an_open_trial_closes_the_others() {
        let w = en();
        let s = read_versions(STATE.as_bytes()).unwrap();
        let t = read_trial(
            br#"{"trial":{"id":"tr1","n":3,"draft":"drf_t","session":"sess_t","started":"2026-11-16T09:00:00Z","used":"2026-11-16T10:00:00Z","changed":"It asks before shipping","try":"Ship order 4410"}}"#,
        )
        .unwrap()
        .unwrap();
        let l = lines(&w, &s, Some(&t)).unwrap();
        assert_eq!(l.rows[0].trying.as_deref(), Some("Trying"));
        assert_eq!(l.rows[0].offers, Offers::Resume("Resume the trial".into()));
        assert!(matches!(
            l.rows[2].offers,
            Offers::TryAndInstall {
                try_open: false,
                install_open: false,
                ..
            }
        ));
        assert_eq!(
            link(&w, Some(&s), Some(&t)).unwrap().1.as_deref(),
            Some("Trying v3 · Resume")
        );
        let g = trial_lines(&w, &t, "Rae").unwrap();
        assert_eq!(g.trying, "Trying version 3");
        assert_eq!(
            g.guide,
            [
                "What's new: It asks before shipping",
                "Try it with: Ship order 4410"
            ]
        );
        assert_eq!(read_trial(br#"{"trial":null}"#), Some(None));
        assert_eq!(read_trial(b"[]"), None);
    }

    #[test]
    fn a_switch_under_way_waits() {
        let w = en();
        let mut s = read_versions(STATE.as_bytes()).unwrap();
        s.switching = Some(Switching {
            n: 3,
            draft: Some("drf_3".into()),
            at: Utc::now(),
        });
        let l = lines(&w, &s, None).unwrap();
        let sw = l.switching.unwrap();
        assert_eq!(sw.title, "Installing version 3");
        assert_eq!(sw.finish.as_deref(), Some("Finish"));
        // Only the version being switched to may be installed meanwhile.
        assert!(matches!(
            l.rows[2].offers,
            Offers::TryAndInstall {
                install_open: false,
                ..
            }
        ));
        assert!(matches!(
            l.rows[0].offers,
            Offers::TryAndInstall {
                install_open: true,
                ..
            }
        ));
        let none = read_versions(br#"{"versions":[],"switching":null}"#).unwrap();
        assert!(lines(&w, &none, None).unwrap().first_only.is_some());
        assert_eq!(cancel_switch_path("t 1"), "/v1/tafs/t%201/switching");
        for p in [trial_conversation_path("t1"), trial_messages_path("t1")] {
            assert!(!crate::signed::signed_at_cloud("POST", &p), "{p}");
        }
    }
}
