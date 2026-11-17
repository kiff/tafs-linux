//! A tafs cloud taf's Setup tab (`TafDetailView.setup(_:)`): tafs cloud's read of it
//! (`GET /v1/tafs/{id}/setup`, `TafsCloud.SetupView`) and what the taf remembers
//! (`GET /v1/tafs/{id}/memory`, `TafsCloud.Note`), each decoded as the iPad's decoders do.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer};
use serde_json::Value;

fn time<'de, D: Deserializer<'de>>(d: D) -> Result<DateTime<Utc>, D::Error> {
    let s = String::deserialize(d)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|t| t.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}

/// `WorkflowStep`, every field required but `app` and `grant`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct WorkflowStep {
    pub number: i64,
    pub label: String,
    pub words: String,
    #[serde(default)]
    pub app: Option<String>,
    #[serde(default)]
    pub grant: Option<String>,
    pub running: bool,
}

/// `ConnectedAction`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConnectedAction {
    pub name: String,
    pub title: String,
    pub limits: Vec<String>,
    pub steps: Vec<i64>,
    pub step_words: String,
}

/// `ConnectedApp.Stats`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Stats {
    pub calls: i64,
    pub done: i64,
    pub held: i64,
    pub words: String,
    pub complete: bool,
    pub activity_query: String,
}

/// `ConnectedApp`: one of the taf's connected apps, as tafs cloud says it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ConnectedApp {
    pub id: String,
    pub name: String,
    pub status: String,
    pub status_words: String,
    pub reconnect: bool,
    pub action_count: i64,
    pub action_words: String,
    pub actions: Vec<ConnectedAction>,
    pub steps: Vec<i64>,
    pub workflow_steps: Vec<WorkflowStep>,
    pub running: bool,
    pub stats: Stats,
    #[serde(default)]
    pub workflow_words: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Workflow {
    pub title: String,
    pub summary: String,
    pub steps: Vec<WorkflowStep>,
    #[serde(default)]
    pub running_step: Option<i64>,
}

/// What the taf remembers, summed up (`SetupView.Memory`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Memory {
    pub count: i64,
    pub title: String,
    #[serde(default)]
    pub latest: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Versions {
    pub title: String,
    pub summary: String,
    #[serde(default)]
    pub live: Option<i64>,
    pub earlier: i64,
}

/// The taf's own Connect again (`ConnectAgain`), read leniently: one without its connection or
/// words reads as none.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConnectAgain {
    pub grant: String,
    pub app: String,
    pub title: String,
    pub words: String,
    pub done_words: String,
}

impl ConnectAgain {
    pub fn usable(&self) -> bool {
        !self.grant.is_empty() && !self.title.is_empty() && !self.words.is_empty()
    }
}

impl<'de> Deserialize<'de> for ConnectAgain {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<ConnectAgain, D::Error> {
        let v = Value::deserialize(d)?;
        let text = |k: &str| {
            v.get(k)
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string()
        };
        Ok(ConnectAgain {
            grant: text("grant"),
            app: text("app"),
            title: text("title"),
            words: text("words"),
            done_words: text("done_words"),
        })
    }
}

/// tafs cloud's Setup read (`SetupView`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SetupView {
    pub connections: Vec<ConnectedApp>,
    pub workflow: Workflow,
    pub memory: Memory,
    pub versions: Versions,
    #[serde(default)]
    pub connections_error: Option<String>,
    #[serde(default)]
    pub card_unit_exceptions: Option<Vec<crate::waits::CardUnitException>>,
    #[serde(default)]
    pub connect_again: Option<ConnectAgain>,
}

/// The Setup read; None when it can't be read as the iPad reads it.
pub fn read(body: &[u8]) -> Option<SetupView> {
    serde_json::from_slice(body).ok()
}

/// `setupView`'s path.
pub fn path(taf: &str) -> String {
    format!("/v1/tafs/{}/setup", crate::work::segment(taf))
}

/// A note the taf keeps (`TafsCloud.Note`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Note {
    pub id: String,
    pub path: String,
    pub content: String,
    #[serde(deserialize_with = "time")]
    pub updated_at: DateTime<Utc>,
}

impl Note {
    /// The note's name, from its path: "/customers/acme.md" reads "customers · acme".
    pub fn title(&self) -> String {
        self.path
            .split('/')
            .filter(|p| !p.is_empty())
            .map(|p| p.replace(".md", "").replace('-', " "))
            .collect::<Vec<_>>()
            .join(" · ")
    }
}

/// `memory`'s answer, `{"notes": […]}`: every note whole, or none read.
pub fn read_notes(body: &[u8]) -> Option<Vec<Note>> {
    #[derive(Deserialize)]
    struct Answer {
        notes: Vec<Note>,
    }
    serde_json::from_slice::<Answer>(body).ok().map(|a| a.notes)
}

/// `memory(_:all:)`'s path: the latest notes, or all of them.
pub fn memory_path(taf: &str, all: bool) -> String {
    format!(
        "/v1/tafs/{}/memory{}",
        crate::work::segment(taf),
        if all { "?all=true" } else { "" }
    )
}

/// `forget`'s path: one note, `DELETE`d.
pub fn forget_path(taf: &str, note: &str) -> String {
    format!(
        "/v1/tafs/{}/memory/{}",
        crate::work::segment(taf),
        crate::work::segment(note)
    )
}

/// One node of `ConnectedToolsRing` per app: the connection that does the work, with any other
/// sign-in to the same app inside its sheet (`ConnectedToolsRing.Node`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub app: ConnectedApp,
    pub others: Vec<ConnectedApp>,
}

/// `ConnectedToolsRing.nodes`: in tafs cloud's order of each app's first connection; the one shown
/// is the first with actions, else one to sign in again, else the first.
pub fn nodes(apps: &[ConnectedApp]) -> Vec<Node> {
    let mut order: Vec<&str> = Vec::new();
    for a in apps {
        if !order.contains(&a.name.as_str()) {
            order.push(&a.name);
        }
    }
    order
        .into_iter()
        .map(|name| {
            let all: Vec<&ConnectedApp> = apps.iter().filter(|a| a.name == name).collect();
            let main = all
                .iter()
                .find(|a| a.action_count > 0)
                .or_else(|| all.iter().find(|a| a.reconnect))
                .unwrap_or(&all[0]);
            Node {
                app: (*main).clone(),
                others: all
                    .iter()
                    .filter(|a| a.id != main.id)
                    .map(|a| (*a).clone())
                    .collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_note_s_title_comes_from_its_path() {
        let n = Note {
            id: "n1".into(),
            path: "/customers/acme-corp.md".into(),
            content: "Pays by card".into(),
            updated_at: Utc::now(),
        };
        assert_eq!(n.title(), "customers · acme corp");
        assert!(read_notes(br#"{"notes":[{"id":"n1","path":"/a.md","content":"x"}]}"#).is_none());
        assert_eq!(
            read_notes(
                br#"{"notes":[{"id":"n1","path":"/a.md","content":"x","updated_at":"2026-11-16T09:00:00.120Z"}]}"#
            )
            .unwrap()
            .len(),
            1
        );
        assert_eq!(memory_path("t 1", true), "/v1/tafs/t%201/memory?all=true");
        assert_eq!(forget_path("t", "n/1"), "/v1/tafs/t/memory/n%2F1");
        assert_eq!(path("t"), "/v1/tafs/t/setup");
    }

    #[test]
    fn connect_again_without_its_words_is_none() {
        let a: ConnectAgain =
            serde_json::from_str(r#"{"grant":" g ","title":"T","words":7}"#).unwrap();
        assert_eq!(a.grant, "g");
        assert!(!a.usable());
    }

    #[test]
    fn one_node_per_app() {
        let base: ConnectedApp = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../../shared/contracts/taf-setup-connected.json"
            ))
            .unwrap()["answer"]["connections"][0]
                .clone(),
        )
        .unwrap();
        let mut idle = base.clone();
        idle.id = "grt_2".into();
        idle.action_count = 0;
        let mut again = base.clone();
        again.id = "grt_3".into();
        again.action_count = 0;
        again.reconnect = true;
        let mut other = base.clone();
        other.id = "grt_stripe".into();
        other.name = "Stripe".into();
        let n = nodes(&[idle.clone(), other.clone(), again.clone(), base.clone()]);
        assert_eq!(n.len(), 2);
        assert_eq!(n[0].app.id, "grt_gmail", "the one with actions");
        assert_eq!(
            n[0].others
                .iter()
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>(),
            ["grt_2", "grt_3"]
        );
        assert_eq!(n[1].app.id, "grt_stripe");
        let n = nodes(&[idle.clone(), again.clone()]);
        assert_eq!(n[0].app.id, "grt_3", "else one to sign in again");
        assert_eq!(nodes(&[idle]).len(), 1);
    }
}
