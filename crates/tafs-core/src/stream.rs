//! tafs cloud's stream of a run as it answers (`streamMessage`, `resumeRun`): server-sent events,
//! each a name and a line of JSON, read into the iPad's `StreamEvent`; and the answer they build
//! while it's written (`LiveAnswer`): its words so far, its tool calls as steps, and the live line.
//! Both routes are the session's (`POST /v1/tafs/{id}/messages/stream`,
//! `GET /v1/tafs/{id}/runs/{run}/stream?after=`), never signed.

use serde_json::{Map, Value};

use crate::words::{WordBook, WordsError};

/// `POST /v1/tafs/{id}/messages/stream` (`streamMessage`): the message, with its run streamed.
pub fn message_path(taf: &str) -> String {
    format!("/v1/tafs/{}/messages/stream", segment(taf))
}

/// `GET /v1/tafs/{id}/runs/{run}/stream?after=` (`resumeRun`): a run of the person's picked
/// back up, from its `after`th step.
pub fn resume_path(taf: &str, run: &str, after: usize) -> String {
    format!(
        "/v1/tafs/{}/runs/{}/stream?after={after}",
        segment(taf),
        segment(run)
    )
}

/// Where the apps reach tafs cloud, as they share it (`app/hosts.json` at the pin): compiled in,
/// never copied here.
const HOSTS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../shared/app/hosts.json"
));

/// Where a run streams from (`TafsCloud.defaultStreamURL`, the shared `tafs_cloud_stream`):
/// tafs cloud's function itself, as a run streams for minutes, longer than a CDN waits between
/// bytes. None when the shared file doesn't give an HTTPS origin: then nothing streams.
pub fn stream_host() -> Option<&'static str> {
    static HOST: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();
    HOST.get_or_init(|| origin_host(HOSTS, "tafs_cloud_stream"))
        .as_deref()
}

/// The host of `key` in a hosts file, when it's an HTTPS origin and nothing more.
fn origin_host(hosts: &str, key: &str) -> Option<String> {
    let v: Value = serde_json::from_str(hosts).ok()?;
    let u = url::Url::parse(v.get(key)?.as_str()?).ok()?;
    let bare = u.scheme() == "https"
        && u.username().is_empty()
        && u.password().is_none()
        && u.port().is_none()
        && u.path() == "/"
        && u.query().is_none()
        && u.fragment().is_none();
    bare.then(|| u.host_str().map(str::to_string)).flatten()
}

/// A stream route's address at the stream host, when there is one.
pub fn stream_url(path: &str) -> Option<String> {
    stream_host().map(|host| format!("https://{host}{path}"))
}

/// Whether the session may go to `url` with `method`: HTTPS to the stream host, with no user,
/// password or port, on one of the two stream routes and nothing else. No other route of the
/// session's, and nothing the device signs, is ever sent there.
pub fn may_stream_to(method: &str, url: &str) -> bool {
    stream_host().is_some_and(|host| may_stream_at(host, method, url))
}

fn may_stream_at(host: &str, method: &str, url: &str) -> bool {
    let Ok(u) = url::Url::parse(url) else {
        return false;
    };
    if u.scheme() != "https"
        || !u.username().is_empty()
        || u.password().is_some()
        || u.port().is_some()
        || u.host_str() != Some(host)
        || u.fragment().is_some()
    {
        return false;
    }
    let Some(parts) = u.path_segments().map(|p| p.collect::<Vec<_>>()) else {
        return false;
    };
    match (method, parts.as_slice(), u.query()) {
        ("POST", ["v1", "tafs", taf, "messages", "stream"], None) => !taf.is_empty(),
        ("GET", ["v1", "tafs", taf, "runs", run, "stream"], Some(q)) => {
            !taf.is_empty()
                && !run.is_empty()
                && q.strip_prefix("after=")
                    .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        }
        _ => false,
    }
}

fn segment(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes())
        .collect::<String>()
        .replace('+', "%20")
}

/// A call a run made that acted and went through (`DoneCall`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoneCall {
    pub tool: String,
    /// What it acted on (a payment's id), when KIFF named it.
    pub resource: Option<String>,
    /// KIFF's key for the call, its Record line's.
    pub invocation: Option<String>,
}

/// What a streamed run does, as tafs cloud relays it (`StreamEvent`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The chat the message went into, first, and the run that answers it; `message` is tafs
    /// cloud's id for the owner's message.
    Session {
        session: String,
        new_session: bool,
        run: Option<String>,
        message: Option<String>,
    },
    /// More of the answer, as it's written.
    Text(String),
    /// A tool the agent calls, by name, with what it's doing in a few words when tafs cloud says;
    /// `call` is the call's id, which its `Step` names when it ends.
    Activity {
        tool: String,
        step: Option<String>,
        call: Option<String>,
    },
    /// How a call ended: "done", "error" or "not_made".
    Step { call: String, status: String },
    /// More of the summary of what it's thinking: kept, never drawn.
    Thinking(String),
    /// The one line to show while it works: `thinking`, `working` with its words,
    /// `waiting_for_you` or `done`.
    State {
        state: String,
        words: Option<String>,
    },
    /// A run picked back up wrote a message: the conversation has it.
    Said,
    /// A run picked back up went on after a pause: what it did before is the conversation's.
    Part,
    /// The agent showed a component: the conversation has it.
    Component,
    /// The run stopped: its status, why, what went through before it stopped, and tafs cloud's
    /// own code and words for a failure it names.
    End {
        status: String,
        stop: String,
        done: Vec<DoneCall>,
        code: Option<String>,
        words: Option<String>,
    },
    /// The stream broke on tafs cloud's side (`error`): the conversation has the rest.
    Broken,
    /// The run read a message the owner wrote to it while it went.
    Taken,
    /// KIFF holds the call just made, at this point of the answer.
    Held {
        exception: String,
        invocation: String,
    },
}

/// One event from its name and its line of JSON (`streamEvent`); none when it isn't one the app
/// knows, or lacks what it needs.
pub fn event(name: &str, data: &str) -> Option<Event> {
    let v: Value = serde_json::from_str(data).unwrap_or(Value::Null);
    let empty = Map::new();
    let o = v.as_object().unwrap_or(&empty);
    let text = |k: &str| o.get(k).and_then(Value::as_str).map(str::to_string);
    let said = |k: &str| text(k).filter(|s| !s.trim().is_empty());
    Some(match name {
        "session" => Event::Session {
            session: text("session")?,
            new_session: o
                .get("new_session")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            run: text("run"),
            message: o
                .get("message")
                .and_then(|m| m.get("id"))
                .and_then(Value::as_str)
                .map(str::to_string),
        },
        "taken" => Event::Taken,
        "text" => Event::Text(text("text")?),
        "activity" => Event::Activity {
            tool: text("tool")?,
            step: text("step"),
            call: text("call_id"),
        },
        "step" => Event::Step {
            call: text("call_id")?,
            status: text("status").unwrap_or_else(|| "done".into()),
        },
        "thinking" => Event::Thinking(text("text")?),
        "state" => Event::State {
            state: text("state")?,
            words: text("words").filter(|w| !w.trim().is_empty()),
        },
        "said" => Event::Said,
        "part" => Event::Part,
        "component" => Event::Component,
        "end" => Event::End {
            status: text("status").unwrap_or_default(),
            stop: text("stop").unwrap_or_default(),
            done: o
                .get("done")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(|c| {
                            Some(DoneCall {
                                tool: c.get("tool")?.as_str()?.to_string(),
                                resource: c
                                    .get("resource")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                                invocation: c
                                    .get("invocation")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            code: said("code"),
            words: said("words"),
        },
        "error" => Event::Broken,
        "held" => Event::Held {
            exception: text("exception").unwrap_or_default(),
            invocation: text("invocation").unwrap_or_default(),
        },
        _ => return None,
    })
}

/// Reads the stream's lines as they come (`events`): `event:` names what the next `data:` line
/// is, "message" until one does.
#[derive(Debug, Clone)]
pub struct Reader {
    name: String,
}

impl Default for Reader {
    fn default() -> Reader {
        Reader {
            name: "message".into(),
        }
    }
}

impl Reader {
    /// One line of the stream (its line break taken off): the event it completes, if any.
    pub fn line(&mut self, line: &str) -> Option<Event> {
        if let Some(name) = line.strip_prefix("event:") {
            self.name = name.trim().to_string();
            None
        } else {
            line.strip_prefix("data:")
                .and_then(|data| event(&self.name, data))
        }
    }
}

/// tafs cloud's live line: what the run does now (`LiveState`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveState {
    pub state: String,
    pub words: Option<String>,
}

impl LiveState {
    /// The line in words (`line`): the step's or the note's own while working, else what the
    /// state is; none at `done`. A state the app doesn't know reads as thinking.
    pub fn line(&self, w: &WordBook) -> Result<Option<String>, WordsError> {
        Ok(match self.state.as_str() {
            "done" => None,
            "working" => Some(match &self.words {
                Some(words) => format!("{words}…"),
                None => w.line("chat.working")?,
            }),
            "waiting_for_you" => Some(w.line("chat.waiting_for_you")?),
            _ => Some(w.line("chat.thinking")?),
        })
    }
}

/// How a streamed tool call went (`LiveStep.Status`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepStatus {
    Running,
    Done,
    Error,
    Held,
    /// Refused back unmade: an act sent beside a read; not a failure.
    NotMade,
}

impl StepStatus {
    /// A `step` event's status.
    pub fn from_stream(s: &str) -> StepStatus {
        match s {
            "error" => StepStatus::Error,
            "not_made" => StepStatus::NotMade,
            _ => StepStatus::Done,
        }
    }
}

/// One tool call of an answer as it streams (`LiveStep`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiveStep {
    pub call: Option<String>,
    pub tool: String,
    /// What it's doing, in the owner's words; none shows the tool's own.
    pub words: Option<String>,
    pub status: StepStatus,
}

/// A finished piece of a taf's streamed answer (`ReplyPart`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplyPart {
    Text(String),
    /// KIFF held the call made here.
    Held {
        exception: String,
        invocation: String,
    },
}

/// The answer as it's written (`LiveAnswer`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LiveAnswer {
    /// The part being written now.
    pub text: String,
    pub activity: Option<String>,
    /// The chat the message went into: what Stop stops.
    pub session: Option<String>,
    /// The run answering it: what a message written meanwhile goes to.
    pub run: Option<String>,
    /// The answer so far, in the order it came: its words up to each tool call, and where KIFF
    /// held a call. It only grows while it streams.
    pub parts: Vec<ReplyPart>,
    /// When it started, as seconds since the epoch: the time its words are shown under.
    pub started: i64,
    /// What it's thinking now: kept, never drawn.
    pub thinking: String,
    /// tafs cloud's live line, once its stream sends one.
    pub state: Option<LiveState>,
    /// The tools it called in this answer, in order, and how each went.
    pub steps: Vec<LiveStep>,
}

impl LiveAnswer {
    pub fn new(started: i64) -> LiveAnswer {
        LiveAnswer {
            started,
            ..LiveAnswer::default()
        }
    }

    /// The live line (`line(current:)`): tafs cloud's while its stream sends one, else the step
    /// in progress in its words, else "Writing…" or "Thinking…".
    pub fn line(&self, w: &WordBook, current: Option<&str>) -> Result<String, WordsError> {
        if let Some(s) = &self.state {
            if let Some(l) = s.line(w)? {
                return Ok(l);
            }
        }
        if let Some(c) = current {
            return Ok(c.to_string());
        }
        w.line(if self.text.is_empty() {
            "chat.thinking"
        } else {
            "chat.writing"
        })
    }

    /// A tool call begins (`began`): shown as a step; `show` draws in the chat itself. A call
    /// sent again (a run picked back up replays its steps) keeps the status it has.
    pub fn began(&mut self, tool: &str, words: Option<&str>, call: Option<&str>) {
        if tool == "show" {
            return;
        }
        if let Some(c) = call {
            if self.steps.iter().any(|s| s.call.as_deref() == Some(c)) {
                return;
            }
        }
        self.steps.push(LiveStep {
            call: call.map(str::to_string),
            tool: tool.to_string(),
            words: words.map(str::to_string),
            status: StepStatus::Running,
        });
    }

    /// A run went on after a pause (`newPart`): what follows starts afresh.
    pub fn new_part(&mut self) {
        self.text.clear();
        self.parts.clear();
        self.steps.clear();
        self.activity = None;
        self.thinking.clear();
        self.state = None;
    }

    /// A call ended (`ended`): the step it began, or the latest still running when the stream
    /// didn't say which (a held call).
    pub fn ended(&mut self, call: Option<&str>, status: StepStatus) {
        let i = match call {
            Some(c) => self
                .steps
                .iter()
                .rposition(|s| s.call.as_deref() == Some(c)),
            None => None,
        }
        .or_else(|| {
            self.steps
                .iter()
                .rposition(|s| s.status == StepStatus::Running)
        });
        if let Some(i) = i {
            self.steps[i].status = status;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_session_goes_to_the_stream_host_for_its_two_routes_only() {
        let host = stream_host().expect("the shared file names the stream's origin");
        let url = |p: &str| stream_url(p).unwrap();
        assert!(may_stream_to("POST", &url(&message_path("taf 1"))));
        assert!(may_stream_to("GET", &url(&resume_path("t", "r/1", 3))));
        // The other method of each, any other route, a query or none where it doesn't belong.
        assert!(!may_stream_to("GET", &url(&message_path("t"))));
        assert!(!may_stream_to("POST", &url(&resume_path("t", "r", 0))));
        assert!(!may_stream_to("POST", &url("/v1/tafs/t/messages")));
        assert!(!may_stream_to("POST", &url("/v1/me/devices/stream")));
        assert!(!may_stream_to(
            "POST",
            &url("/v1/tafs/t/messages/stream?x=1")
        ));
        assert!(!may_stream_to("GET", &url("/v1/tafs/t/runs/r/stream")));
        assert!(!may_stream_to(
            "GET",
            &url("/v1/tafs/t/runs/r/stream?after=1&x=2")
        ));
        assert!(!may_stream_to("POST", &url("/v1/tafs//messages/stream")));
        assert!(!may_stream_to("POST", &url("/v1/tafs/t/messages/stream/")));
        // Another host, another scheme, a port or a user.
        let path = message_path("t");
        assert!(!may_stream_to(
            "POST",
            &format!("https://cloud.tafs.kiff.dev{path}")
        ));
        assert!(!may_stream_to("POST", &format!("http://{host}{path}")));
        assert!(!may_stream_to(
            "POST",
            &format!("https://{host}:8443{path}")
        ));
        assert!(!may_stream_to("POST", &format!("https://a@{host}{path}")));
        assert!(!may_stream_to(
            "POST",
            &format!("https://{host}.evil.dev{path}")
        ));
    }

    #[test]
    fn the_hosts_come_from_the_shared_file_as_bare_https_origins() {
        // tafs cloud's own address here is the shared one.
        assert_eq!(
            origin_host(HOSTS, "tafs_cloud").map(|h| format!("https://{h}")),
            Some(crate::home::TAFS_CLOUD.to_string())
        );
        let one = |url: &str| origin_host(&format!(r#"{{"s":"{url}"}}"#), "s");
        assert_eq!(one("https://a.example"), Some("a.example".into()));
        assert_eq!(one("https://a.example/"), Some("a.example".into()));
        for not in [
            "http://a.example",
            "https://a.example:444",
            "https://u@a.example",
            "https://a.example/v1",
            "https://a.example?x=1",
            "https://a.example#x",
            "a.example",
        ] {
            assert_eq!(one(not), None, "{not}");
        }
        assert_eq!(origin_host("{}", "s"), None);
        assert_eq!(origin_host("not json", "s"), None);
        // With no stream host, nothing is ever sent for a stream.
        assert!(!may_stream_at(
            "",
            "POST",
            "https:///v1/tafs/t/messages/stream"
        ));
    }

    fn book() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    #[test]
    fn the_events_read_as_the_ipad_reads_them() {
        assert_eq!(
            event(
                "session",
                r#"{"session":"s1","new_session":true,"run":"r1","message":{"id":"m1"}}"#
            ),
            Some(Event::Session {
                session: "s1".into(),
                new_session: true,
                run: Some("r1".into()),
                message: Some("m1".into())
            })
        );
        assert_eq!(event("session", "{}"), None);
        assert_eq!(
            event("text", r#"{"text":"Hi"}"#),
            Some(Event::Text("Hi".into()))
        );
        assert_eq!(
            event(
                "activity",
                r#"{"tool":"stripe_refund","step":"Refunding","call_id":"c1"}"#
            ),
            Some(Event::Activity {
                tool: "stripe_refund".into(),
                step: Some("Refunding".into()),
                call: Some("c1".into())
            })
        );
        assert_eq!(
            event("step", r#"{"call_id":"c1"}"#),
            Some(Event::Step {
                call: "c1".into(),
                status: "done".into()
            })
        );
        assert_eq!(
            event("state", r#"{"state":"working","words":"  "}"#),
            Some(Event::State {
                state: "working".into(),
                words: None
            })
        );
        assert_eq!(
            event(
                "end",
                r#"{"status":"failed","stop":"error","done":[{"tool":"refund","resource":"pi_1"},{"x":1}],"code":"ai_unavailable","words":" "}"#
            ),
            Some(Event::End {
                status: "failed".into(),
                stop: "error".into(),
                done: vec![DoneCall {
                    tool: "refund".into(),
                    resource: Some("pi_1".into()),
                    invocation: None
                }],
                code: Some("ai_unavailable".into()),
                words: None
            })
        );
        assert_eq!(event("error", ""), Some(Event::Broken));
        assert_eq!(
            event("held", r#"{"exception":"e1","invocation":"i1"}"#),
            Some(Event::Held {
                exception: "e1".into(),
                invocation: "i1".into()
            })
        );
        assert_eq!(event("ping", "{}"), None);
    }

    #[test]
    fn the_reader_names_each_data_line_by_the_event_before_it() {
        let mut r = Reader::default();
        assert_eq!(r.line(": keep-alive"), None);
        assert_eq!(r.line("event: text"), None);
        assert_eq!(
            r.line(r#"data: {"text":"a"}"#),
            Some(Event::Text("a".into()))
        );
        assert_eq!(
            r.line(r#"data:{"text":"b"}"#),
            Some(Event::Text("b".into()))
        );
        assert_eq!(r.line(""), None);
        assert_eq!(r.line("event:said"), None);
        assert_eq!(r.line("data: {}"), Some(Event::Said));
    }

    #[test]
    fn the_live_answer_keeps_its_steps_and_its_line() {
        let w = book();
        let mut a = LiveAnswer::new(0);
        assert_eq!(a.line(&w, None).unwrap(), "Thinking…");
        a.text.push_str("Refund");
        assert_eq!(a.line(&w, None).unwrap(), "Writing…");
        assert_eq!(a.line(&w, Some("Refunding…")).unwrap(), "Refunding…");
        a.began("show", None, Some("c0"));
        a.began("refund", Some("Refunding"), Some("c1"));
        a.began("refund", Some("again"), Some("c1"));
        a.began("read", None, None);
        assert_eq!(a.steps.len(), 2);
        a.ended(Some("c1"), StepStatus::Done);
        a.ended(None, StepStatus::Held);
        assert_eq!(
            a.steps.iter().map(|s| s.status).collect::<Vec<_>>(),
            [StepStatus::Done, StepStatus::Held]
        );
        a.state = Some(LiveState {
            state: "working".into(),
            words: Some("Checking Stripe".into()),
        });
        assert_eq!(a.line(&w, None).unwrap(), "Checking Stripe…");
        a.state = Some(LiveState {
            state: "done".into(),
            words: None,
        });
        assert_eq!(a.line(&w, None).unwrap(), "Writing…");
        a.new_part();
        assert!(a.steps.is_empty() && a.text.is_empty() && a.state.is_none());
        assert_eq!(StepStatus::from_stream("not_made"), StepStatus::NotMade);
        assert_eq!(message_path("taf 1"), "/v1/tafs/taf%201/messages/stream");
        assert_eq!(
            resume_path("t", "r/1", 3),
            "/v1/tafs/t/runs/r%2F1/stream?after=3"
        );
    }
}
