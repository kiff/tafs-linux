//! What this person may do with each taf (`KIFFClient.Access`, `GET /v1/me/access`), who a taf
//! is shared with (`KIFFClient.Share`, `GET /v1/agents/{agent}/grants`), and a tafs cloud taf's
//! last run's report (`TafsCloud.Run`, `Report`; `GET /v1/tafs/{id}/runs`, `…/runs/{session}`),
//! for the Overview, a taf's Card tab and who issued its Cards.

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};
use serde_json::Value;

use crate::people::{Group, Person};
use crate::words::{WordBook, WordsError};

/// What a person may do with a taf: use it, see everyone's work with it, change it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Access {
    pub use_: bool,
    pub oversee: bool,
    pub manage: bool,
    pub groups: Option<Vec<String>>,
}

fn flag(v: &Value, key: &str) -> Option<bool> {
    v.get(key)?.as_bool()
}

/// `GET /v1/me/access`: each KIFF agent's access, as strictly as Swift's synthesized decoder
/// (`use`, `oversee` and `manage` required); none when any can't be read.
pub fn read_access(body: &[u8]) -> Option<BTreeMap<String, Access>> {
    let v: Value = serde_json::from_slice(body).ok()?;
    v.get("agents")?
        .as_object()?
        .iter()
        .map(|(agent, a)| {
            let groups = match a.get("groups") {
                None | Some(Value::Null) => None,
                Some(g) => Some(
                    g.as_array()?
                        .iter()
                        .map(|s| s.as_str().map(str::to_string))
                        .collect::<Option<Vec<_>>>()?,
                ),
            };
            Some((
                agent.clone(),
                Access {
                    use_: flag(a, "use")?,
                    oversee: flag(a, "oversee")?,
                    manage: flag(a, "manage")?,
                    groups,
                },
            ))
        })
        .collect()
}

pub const ACCESS_PATH: &str = "/v1/me/access";

/// `AppModel.manages`: the access read says so; nothing read, no.
pub fn manages(access: Option<&Access>) -> bool {
    access.is_some_and(|a| a.manage)
}

/// Who a taf is shared with: a group, or one person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Share {
    pub group: Option<String>,
    pub subject: Option<String>,
}

fn optional(v: &Value, key: &str) -> Result<Option<String>, ()> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(()),
    }
}

/// `GET /v1/agents/{agent}/grants`.
pub fn read_shares(body: &[u8]) -> Option<Vec<Share>> {
    let v: Value = serde_json::from_slice(body).ok()?;
    v.get("grants")?
        .as_array()?
        .iter()
        .map(|s| {
            s.is_object().then_some(())?;
            Some(Share {
                group: optional(s, "group").ok()?,
                subject: optional(s, "subject").ok()?,
            })
        })
        .collect()
}

fn escaped(v: &str) -> String {
    url::form_urlencoded::byte_serialize(v.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

pub fn shares_path(agent: &str) -> String {
    format!("/v1/agents/{}/grants", escaped(agent))
}

/// The Overview's "Support · 3 people" (`usedBy`): each group's or person's name, then how many
/// people; "Not shared" with none.
pub fn used_by(
    w: &WordBook,
    shares: &[Share],
    groups: &[Group],
    people: &[Person],
) -> Result<String, WordsError> {
    if shares.is_empty() {
        return w.line("people.not_shared");
    }
    let mut who = std::collections::BTreeSet::new();
    let mut names = Vec::new();
    for s in shares {
        if let Some(group) = s
            .group
            .as_ref()
            .and_then(|g| groups.iter().find(|x| &x.id == g))
        {
            names.push(group.shown_name(w)?);
            who.extend(group.members.iter().map(|m| m.subject.clone()));
        } else if let Some(subject) = &s.subject {
            names.push(
                people
                    .iter()
                    .find(|p| &p.subject == subject)
                    .map(|p| p.label().to_string())
                    .unwrap_or_else(|| subject.clone()),
            );
            who.insert(subject.clone());
        }
    }
    Ok(format!(
        "{} · {}",
        names.join(", "),
        w.count("people.count", who.len() as i64, &[])?
    ))
}

/// One run (`TafsCloud.Run`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub session: Option<String>,
    pub at: DateTime<FixedOffset>,
    pub error: Option<String>,
}

/// `GET /v1/tafs/{id}/runs`, strictly: a run without its date fails the read.
pub fn read_runs(body: &[u8]) -> Option<Vec<Run>> {
    let v: Value = serde_json::from_slice(body).ok()?;
    v.get("runs")?
        .as_array()?
        .iter()
        .map(|r| {
            Some(Run {
                session: optional(r, "session").ok()?,
                at: DateTime::parse_from_rfc3339(r.get("at")?.as_str()?).ok()?,
                error: optional(r, "error").ok()?,
            })
        })
        .collect()
}

/// `GET /v1/tafs/{id}/runs/{session}`: the report's status and text.
pub fn read_report(body: &[u8]) -> Option<(String, String)> {
    let v: Value = serde_json::from_slice(body).ok()?;
    let r = v.get("report")?;
    Some((
        r.get("status")?.as_str()?.to_string(),
        r.get("text")?.as_str()?.to_string(),
    ))
}

pub fn runs_path(taf: &str) -> String {
    format!("/v1/tafs/{}/runs", escaped(taf))
}

pub fn report_path(taf: &str, session: &str) -> String {
    format!("/v1/tafs/{}/runs/{}", escaped(taf), escaped(session))
}

/// A taf's last run as the Overview reads it (`RunReport`): its report's text (empty when there's
/// none) and what went wrong.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunReport {
    pub text: String,
    pub error: Option<String>,
}

/// `loadReport`'s choice: the latest run with a session has a report to read (`Ok`); with none,
/// the first run's error stands (`Err`).
pub fn latest_session(runs: &[Run]) -> Result<String, RunReport> {
    match runs.iter().find_map(|r| r.session.clone()) {
        Some(s) => Ok(s),
        None => Err(RunReport {
            text: String::new(),
            error: runs.first().and_then(|r| r.error.clone()),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_reads_strictly() {
        let a = read_access(
            br#"{"agents":{"a1":{"use":true,"oversee":false,"manage":true},"a2":{"use":true,"oversee":false,"manage":false,"groups":["g"]}}}"#,
        )
        .unwrap();
        assert!(manages(a.get("a1")));
        assert!(!manages(a.get("a2")));
        assert!(!manages(a.get("a3")));
        assert!(read_access(br#"{"agents":{"a1":{"use":true}}}"#).is_none());
    }

    #[test]
    fn runs_choose_the_latest_with_a_session() {
        let runs = read_runs(
            br#"{"runs":[{"session":null,"at":"2026-10-16T12:00:00Z","error":"x"},{"session":"s1","at":"2026-10-16T11:00:00Z","error":null}]}"#,
        )
        .unwrap();
        assert_eq!(latest_session(&runs), Ok("s1".into()));
        let none =
            read_runs(br#"{"runs":[{"at":"2026-10-16T12:00:00Z","error":"stopped"}]}"#).unwrap();
        assert_eq!(
            latest_session(&none),
            Err(RunReport {
                text: String::new(),
                error: Some("stopped".into())
            })
        );
        assert!(read_runs(br#"{"runs":[{"session":"s"}]}"#).is_none());
        assert_eq!(report_path("t 1", "s/2"), "/v1/tafs/t%201/runs/s%2F2");
    }
}
