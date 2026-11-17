//! An app's connections, as Connect apps shows them (`AppConnectionsView`, `ConnectionDetailView`):
//! planque's grants (`PlanqueClient.Grant`), read as strictly as the iPad's decoder, grouped by the
//! taf each was set up for, with the words for removing access, clearing a removed taf's
//! connections and fixing one.
//!
//! This computer reads no grant from planque: the iPad reads them only once it is set up with
//! planque (`planqueReady`: enrolled, a signed act), which a computer that only views never is. So
//! signed in with KIFF an app's connections say so (`readsConnections` is false), and only the
//! sample's screenshot draws connections, from the iPad's own sample (`Samples/grants.json`).

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use crate::coach::{array, array_if, bool_if, field, object, strings, text, text_if, Read};
use crate::connect::{App, RemovedGrant, RemovedTaf};
use crate::words::{list, WordBook, WordsError};

/// One tool a connection asks for (`Grant.Request.Tool`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    pub tool: String,
    pub name: Option<String>,
    pub title: Option<String>,
    pub read_only: Option<bool>,
    /// Setup's own read, never the taf's.
    pub setup_reads: Option<bool>,
}

impl Tool {
    fn read(v: &Value) -> Read<Tool> {
        let o = object(v)?;
        Ok(Tool {
            tool: text(o, "tool")?,
            name: text_if(o, "name")?,
            title: text_if(o, "title")?,
            read_only: bool_if(o, "read_only")?,
            setup_reads: bool_if(o, "setup_reads")?,
        })
    }

    /// One of the taf's own tools, not Setup's read.
    pub fn tafs(&self) -> bool {
        self.setup_reads != Some(true)
    }

    /// Its name in KIFF: the Card's action.
    pub fn action(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.tool)
    }
}

/// A managed provider's connection (`Grant.Request.Connector`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Connector {
    pub provider: Option<String>,
    pub toolkit: Option<String>,
}

/// What the connection asks for (`Grant.Request`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub server: String,
    pub agent: String,
    pub tools: Vec<Tool>,
    pub connector: Option<Connector>,
    /// `api_key` for a vendor reached with a key the owner gives.
    pub kind: Option<String>,
}

impl Request {
    fn read(v: &Value) -> Read<Request> {
        let o = object(v)?;
        let connector = match field(o, "connector") {
            None => None,
            Some(c) => {
                let c = object(c)?;
                Some(Connector {
                    provider: text_if(c, "provider")?,
                    toolkit: text_if(c, "toolkit")?,
                })
            }
        };
        Ok(Request {
            server: text(o, "server")?,
            agent: text(o, "agent")?,
            tools: array(o, "tools", Tool::read)?,
            connector,
            kind: text_if(o, "kind")?,
        })
    }
}

/// A connection's health for people (`Grant.Health`), read leniently: anything but its fields'
/// own kinds reads as empty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Health {
    pub state: String,
    pub ok: bool,
    pub words: String,
}

impl Health {
    fn read(v: &Value) -> Health {
        Health {
            state: v
                .get("state")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            ok: v.get("ok").and_then(Value::as_bool).unwrap_or(false),
            words: v
                .get("words")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .into(),
        }
    }
}

/// One thing the owner can do with a connection now, as planque offers it (`Grant.Action`), read
/// leniently: `sign_in_again` (`approve` or `key`), `remove` (`delete`, with what stops),
/// `remove_unfinished` (`delete`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Action {
    pub id: String,
    pub words: String,
    pub how: String,
    pub stops: Option<String>,
}

pub const SIGN_IN_AGAIN: &str = "sign_in_again";
pub const REMOVE: &str = "remove";
pub const REMOVE_UNFINISHED: &str = "remove_unfinished";

impl Action {
    fn read(v: &Value) -> Action {
        let t = |k: &str| -> String {
            v.get(k)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let stops = t("stops");
        Action {
            id: t("id"),
            words: t("words"),
            how: t("how"),
            stops: (!stops.is_empty()).then_some(stops),
        }
    }

    /// One this app knows how to take, with its words: anything else isn't drawn.
    pub fn known(&self) -> bool {
        if self.words.is_empty() {
            return false;
        }
        match self.id.as_str() {
            SIGN_IN_AGAIN => self.how == "approve" || self.how == "key",
            REMOVE | REMOVE_UNFINISHED => self.how == "delete",
            _ => false,
        }
    }

    pub fn removes(&self) -> bool {
        self.id == REMOVE || self.id == REMOVE_UNFINISHED
    }
}

/// When planque changed it, in the owner's words (`Grant.Event`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub at: String,
    pub what: String,
}

/// A connection at planque (`PlanqueClient.Grant`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grant {
    pub id: String,
    /// pending, approved, signed_in, listed, connected, failed, disconnected or needs_reconnect.
    pub status: String,
    pub status_words: Option<String>,
    /// What the owner reads before approving.
    pub text: String,
    pub terms: Option<Vec<String>>,
    pub digest: String,
    pub request: Request,
    pub account: Option<String>,
    pub title: Option<String>,
    /// The tafs that use it, by their names (planque's rows).
    pub used_by: Option<Vec<String>>,
    pub health: Option<Health>,
    pub actions: Option<Vec<Action>>,
    pub tools: Option<Vec<String>>,
    pub reason: Option<String>,
    pub events: Vec<Event>,
    pub signed_in: Option<bool>,
    pub held_until: Option<String>,
    pub vendor_name: Option<String>,
    pub entry: Option<String>,
    pub provider: Option<String>,
    pub agent_name: Option<String>,
    /// Whether `digest` is the hash of the request as planque sent it: set only by a read from
    /// planque, which this computer never makes, so never here.
    pub digest_matches: bool,
}

/// `.whitespaces` (no newlines), as `trimmingCharacters(in: .whitespaces)`.
fn trim_spaces(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() && c != '\n' && c != '\r')
}

impl Grant {
    fn read(v: &Value) -> Read<Grant> {
        let o: &Map<String, Value> = object(v)?;
        let health = field(o, "health").map(Health::read);
        let actions = match field(o, "actions") {
            None => None,
            Some(Value::Array(a)) => Some(a.iter().map(Action::read).collect()),
            Some(_) => return Err(()),
        };
        Ok(Grant {
            id: text(o, "id")?,
            status: text(o, "status")?,
            status_words: text_if(o, "status_words")?,
            text: text(o, "text")?,
            terms: array_if(o, "terms", strings)?,
            digest: text(o, "digest")?,
            request: Request::read(field(o, "request").ok_or(())?)?,
            account: text_if(o, "account")?,
            title: text_if(o, "title")?,
            used_by: array_if(o, "used_by", strings)?,
            health,
            actions,
            tools: array_if(o, "tools", strings)?,
            reason: text_if(o, "reason")?,
            events: array(o, "events", |e| {
                let e = object(e)?;
                Ok(Event {
                    at: text(e, "at")?,
                    what: text(e, "what")?,
                })
            })?,
            signed_in: bool_if(o, "signed_in")?,
            held_until: text_if(o, "held_until")?,
            vendor_name: text_if(o, "vendor")?,
            entry: text_if(o, "entry")?,
            provider: text_if(o, "provider")?,
            agent_name: text_if(o, "agent_name")?,
            digest_matches: false,
        })
    }

    /// planque's `{"grants": [...]}`: none when one grant can't be read, as on the iPad.
    pub fn read_all(body: &[u8]) -> Option<Vec<Grant>> {
        let v: Value = serde_json::from_slice(body).ok()?;
        array(object(&v).ok()?, "grants", Grant::read).ok()
    }

    /// A list of grants as the iPad's sample keeps them (`SampleScreens.grants`).
    pub fn read_list(body: &[u8]) -> Option<Vec<Grant>> {
        let v: Value = serde_json::from_slice(body).ok()?;
        v.as_array()?
            .iter()
            .map(Grant::read)
            .collect::<Read<_>>()
            .ok()
    }

    /// The vendor's name as planque gives it, else its server.
    pub fn vendor(&self) -> &str {
        self.vendor_name.as_deref().unwrap_or(&self.request.server)
    }

    /// planque draws this connection's row (its health and actions): it opens its detail.
    pub fn row_from_planque(&self) -> bool {
        self.health.as_ref().is_some_and(|h| !h.words.is_empty()) && self.actions.is_some()
    }

    /// What its row is called: planque's `title`, else the account it names (never a managed
    /// provider's id), else none (the row then says the app).
    pub fn name(&self) -> Option<String> {
        if let Some(t) = self
            .title
            .as_deref()
            .map(trim_spaces)
            .filter(|t| !t.is_empty())
        {
            return Some(t.to_string());
        }
        if self.request.connector.is_none() {
            if let Some(a) = self
                .account
                .as_deref()
                .map(trim_spaces)
                .filter(|a| !a.is_empty())
            {
                return Some(a.to_string());
            }
        }
        None
    }

    pub fn connected(&self) -> bool {
        self.status == "connected"
    }
    /// Signed in, with the vendor's tools listed for Setup to choose from: a sign-in the
    /// person declined to finish is one.
    pub fn listed(&self) -> bool {
        self.status == "listed"
    }
    pub fn needs_reconnect(&self) -> bool {
        self.status == "needs_reconnect"
    }
    pub fn takes_key(&self) -> bool {
        self.request.kind.as_deref() == Some("api_key")
    }
    /// A managed provider's connection, connected: removed and connected again, never signed in
    /// again in place.
    pub fn signs_in_through_provider(&self) -> bool {
        self.request.connector.is_some() && self.connected() && !self.takes_key()
    }
    /// Approved, waiting on a sign-in at the vendor that hasn't come back.
    pub fn waiting_sign_in(&self) -> bool {
        self.status == "approved" && self.signed_in != Some(true)
    }
    /// A sign-in planque holds that no taf uses yet.
    pub fn holds_sign_in(&self) -> bool {
        self.listed() || (self.status == "pending" && self.signed_in == Some(true))
    }
    /// A request nobody answered yet, holding no sign-in: cancelled, not removed.
    pub fn pending_request(&self) -> bool {
        self.status == "pending" && !self.holds_sign_in()
    }
    /// Ended with Cancel, not Remove access.
    pub fn cancellable(&self) -> bool {
        self.waiting_sign_in() || self.pending_request()
    }
    /// The actions planque offers that this app knows.
    pub fn known_actions(&self) -> Vec<&Action> {
        self.actions
            .iter()
            .flatten()
            .filter(|a| a.known())
            .collect()
    }
}

/// `AppModel.connections(of:)`: the account's live connections to an app: by the entry planque
/// names for each, or the grant tafs cloud named for it, never one disconnected or failed.
pub fn connections_of<'a>(app: &App, grants: &'a [Grant]) -> Vec<&'a Grant> {
    grants
        .iter()
        .filter(|g| {
            (g.entry.as_deref() == Some(app.entry.as_str())
                || app.grant.as_deref() == Some(g.id.as_str()))
                && !["disconnected", "failed"].contains(&g.status.as_str())
        })
        .collect()
}

/// `AppModel.opensConnections`: a connected app always opens its detail; so does one this device
/// holds a connection to that isn't connected (a declined sign-in, `listed`), so its Disconnect
/// is in reach.
pub fn opens_connections(app: &App, grants: &[Grant]) -> bool {
    app.connected() || !connections_of(app, grants).is_empty()
}

/// The connections of one taf (`AppModel.ConnectionGroup`), by its KIFF agent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group<'a> {
    pub agent: String,
    pub taf: String,
    pub grants: Vec<&'a Grant>,
}

/// An app's connections grouped (`connectionGroups`): by the taf each was set up for, those
/// needing attention first, then by name; a removed taf's, as tafs cloud names them; and the
/// rest, whose taf the app can't name.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Groups<'a> {
    pub tafs: Vec<Group<'a>>,
    pub removed: Vec<&'a Grant>,
    pub others: Vec<&'a Grant>,
}

pub fn groups<'a>(
    grants: &[&'a Grant],
    taf_of: &dyn Fn(&Grant) -> Option<String>,
    removed_agents: &BTreeSet<String>,
    attention: &dyn Fn(&Grant) -> bool,
) -> Groups<'a> {
    let mut out = Groups::default();
    for g in grants {
        let Some(name) = taf_of(g) else {
            if removed_agents.contains(&g.request.agent) {
                out.removed.push(g);
            } else {
                out.others.push(g);
            }
            continue;
        };
        match out.tafs.iter_mut().find(|x| x.agent == g.request.agent) {
            Some(group) => group.grants.push(g),
            None => out.tafs.push(Group {
                agent: g.request.agent.clone(),
                taf: name,
                grants: vec![g],
            }),
        }
    }
    let needs: BTreeSet<String> = out
        .tafs
        .iter()
        .filter(|x| x.grants.iter().any(|g| attention(g)))
        .map(|x| x.agent.clone())
        .collect();
    out.tafs.sort_by(|a, b| {
        needs
            .contains(&b.agent)
            .cmp(&needs.contains(&a.agent))
            .then_with(|| a.taf.to_lowercase().cmp(&b.taf.to_lowercase()))
            .then_with(|| a.agent.cmp(&b.agent))
    });
    out
}

/// What drew attention to a connection, short of its last failure (which needs the record's
/// tools, not read here): a sign-in that waits, or one its vendor signed out.
pub fn attention(g: &Grant) -> bool {
    g.waiting_sign_in() || g.needs_reconnect()
}

/// `removedListed`: what tafs cloud says of each removed taf's connection, by its grant; the
/// first wins.
pub fn removed_listed(removed: &[RemovedTaf]) -> BTreeMap<String, RemovedGrant> {
    let mut out = BTreeMap::new();
    for g in removed.iter().flat_map(|t| t.grants.iter().flatten()) {
        out.entry(g.grant.clone()).or_insert_with(|| g.clone());
    }
    out
}

/// The removed taf's connections Clear ends: only one tafs cloud lists with nobody using it.
pub fn clearable(removed: &[&Grant], listed: &BTreeMap<String, RemovedGrant>) -> Vec<String> {
    removed
        .iter()
        .filter(|g| listed.get(&g.id).is_some_and(|l| l.used_by.is_empty()))
        .map(|g| g.id.clone())
        .collect()
}

/// `toolVendors`: the vendor of each connected tool, by its name in KIFF, the first connection's
/// where two name the same tool: what a call the vendor refused names in the Record.
pub fn tool_vendors(grants: &[Grant]) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for g in grants.iter().filter(|g| g.connected()) {
        for t in g.tools.iter().flatten() {
            out.entry(t.clone())
                .or_insert_with(|| g.vendor().to_string());
        }
    }
    out
}

/// `ConnectApprovalSheet.host(of:)`: the host a key is used with, without its scheme or path;
/// the server as written when it has none.
pub fn host(server: &str) -> String {
    url::Url::parse(server)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| server.to_string())
}

// MARK: Words.

/// `DisconnectWords.showsFooter`: only to whoever manages Cards, and only when a connection here
/// is a taf's.
pub fn shows_footer(manages: bool, tafs: &[Group]) -> bool {
    manages && !tafs.is_empty()
}

/// `DisconnectWords.title`: the question, naming the one taf it ends and the other tafs it
/// stops too.
pub fn disconnect_title(
    w: &WordBook,
    vendor: &str,
    taf: Option<&str>,
    also_stops: &[String],
) -> Result<String, WordsError> {
    let asked = match taf {
        Some(t) => w.get(
            "connect.disconnect.title_from",
            &[("vendor", vendor), ("taf", t)],
        )?,
        None => w.get("connect.disconnect.title", &[("vendor", vendor)])?,
    };
    if also_stops.is_empty() {
        return Ok(asked);
    }
    Ok(format!(
        "{asked} {}",
        w.count(
            "connect.disconnect.also_stops",
            also_stops.len() as i64,
            &[("names", &list(w.language(), also_stops))],
        )?
    ))
}

/// `DisconnectWords.message`: what removing access ends. A key is deleted from tafs once no taf
/// uses it; a sign-in is revoked at the vendor.
pub fn disconnect_message(
    w: &WordBook,
    vendor: &str,
    taf: Option<&str>,
    also_stops: &[String],
    key: bool,
) -> Result<String, WordsError> {
    let others = match also_stops {
        [] => String::new(),
        [one] => format!(
            " {}",
            w.get("connect.disconnect.also_card", &[("owner", one)])?
        ),
        many => format!(
            " {}",
            w.get(
                "connect.disconnect.also_cards",
                &[("names", &list(w.language(), many))],
            )?
        ),
    };
    let first = match taf {
        Some(t) => w.get(
            "connect.disconnect.tools_stop",
            &[("owner", t), ("vendor", vendor)],
        )?,
        None => w.get("connect.disconnect.no_taf_uses", &[("vendor", vendor)])?,
    };
    let rest = if key {
        format!(
            " {}",
            w.get("connect.disconnect.rest_key", &[("vendor", vendor)])?
        )
    } else {
        format!(
            "{}{}",
            w.get(
                "connect.other_connections_stay_connected_no",
                &[("vendor", vendor), ("vendor_2", vendor)],
            )?,
            w.line("connect.connecting_again_asks_you_approve")?
        )
    };
    Ok(format!("{first}{others}{rest}"))
}

/// `ConnectionGroupWords.usedBy`: under a removed taf's connection Clear keeps, the tafs whose
/// Cards still use it, each once; none when there's none.
pub fn kept_by(w: &WordBook, names: &[String]) -> Result<Option<String>, WordsError> {
    let mut seen = BTreeSet::new();
    let n: Vec<String> = names
        .iter()
        .filter(|s| !trim_spaces(s).is_empty() && seen.insert((*s).clone()))
        .cloned()
        .collect();
    let joined = list(w.language(), &n);
    Ok(match n.len() {
        0 => None,
        1 => Some(w.get("connect.used_by.one", &[("name", &joined)])?),
        _ => Some(w.get("connect.used_by.many", &[("names", &joined)])?),
    })
}

/// `ConnectionFixWords.endWord`: "Cancel the request" for one nobody answered, Cancel for a
/// sign-in that waits, else Remove access.
pub fn end_word(w: &WordBook, g: &Grant) -> Result<String, WordsError> {
    w.line(if g.pending_request() {
        "connect.cancel_request.button"
    } else if g.waiting_sign_in() {
        "common.cancel"
    } else {
        "connect.disconnect_button"
    })
}

/// `ConnectionFixWords.cancelTitle`: what's cancelled, asked first.
pub fn cancel_title(w: &WordBook, g: &Grant) -> Result<String, WordsError> {
    let key = if g.pending_request() {
        "connect.cancel_request.title"
    } else {
        "connect.cancel_sign_in.title"
    };
    w.get(key, &[("vendor", g.vendor())])
}

/// `ConnectionFixWords.cancelWord`: the confirmation's own button for it.
pub fn cancel_word(w: &WordBook, g: &Grant) -> Result<String, WordsError> {
    w.line(if g.pending_request() {
        "connect.cancel_request.button"
    } else {
        "connect.cancel_sign_in.button"
    })
}

/// A connection's status line: planque's words, else "Waiting for the sign-in at …" while one
/// waits.
pub fn status_line(w: &WordBook, g: &Grant) -> Result<Option<String>, WordsError> {
    if let Some(s) = g.status_words.as_ref().filter(|s| !s.is_empty()) {
        return Ok(Some(s.clone()));
    }
    if g.waiting_sign_in() {
        return Ok(Some(w.get(
            "connect.status.waiting_sign_in",
            &[("vendor", g.vendor())],
        )?));
    }
    Ok(None)
}

/// "Used by Refunds, Chargebacks" under planque's row: its names joined as planque's rows join
/// them.
pub fn row_used_by(w: &WordBook, g: &Grant) -> Result<Option<String>, WordsError> {
    match g.used_by.as_ref().filter(|u| !u.is_empty()) {
        Some(u) => Ok(Some(
            w.get("connect.apps.used_by", &[("tafs", &u.join(", "))])?,
        )),
        None => Ok(None),
    }
}

/// `misplacedNote`: a connection made under a taf that doesn't need the app.
pub fn misplaced(w: &WordBook, app: &App, g: &Grant) -> Result<Option<String>, WordsError> {
    match app.misplaced.iter().find(|m| m.grant == g.id) {
        Some(m) => Ok(Some(w.get(
            "connect.misplaced",
            &[("taf", &m.name), ("app", app.display_name())],
        )?)),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connect::ConnectApps;

    fn words(lang: &str) -> WordBook {
        WordBook::load(&WordBook::default_root(), lang).unwrap()
    }

    fn sample() -> Vec<Grant> {
        let file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tafs-app/fixtures/grants.json"
        );
        Grant::read_list(&std::fs::read(file).unwrap()).unwrap()
    }

    fn grant(json: serde_json::Value) -> Grant {
        let mut base = serde_json::json!({
            "id": "g1", "status": "connected", "text": "Read mail", "digest": "d",
            "request": {"server": "https://mail.example", "agent": "a1", "tools": [{"tool": "fetch"}]},
            "events": []
        });
        for (k, v) in json.as_object().unwrap() {
            base[k] = v.clone();
        }
        Grant::read(&base).unwrap()
    }

    fn app(json: serde_json::Value) -> App {
        let body = serde_json::json!({ "apps": [json] }).to_string();
        ConnectApps::read(body.as_bytes()).unwrap().apps.remove(0)
    }

    #[test]
    fn the_ipads_sample_reads() {
        let g = sample();
        assert_eq!(g.len(), 3);
        assert_eq!(g[0].name().as_deref(), Some("Stripe · Demo Shop"));
        assert!(g[0].takes_key() && g[0].row_from_planque() && g[0].connected());
        assert!(!g[0].digest_matches);
        assert_eq!(
            g[2].request.connector.as_ref().unwrap().toolkit.as_deref(),
            Some("gmail")
        );
        assert!(g[2].signs_in_through_provider());
        assert_eq!(g[0].known_actions().len(), 1);
        assert!(g[0].known_actions()[0].removes());
    }

    #[test]
    fn a_grant_reads_as_strictly_as_the_ipads_decoder() {
        let ok = br#"{"grants":[{"id":"g","status":"pending","text":"t","digest":"d","request":{"server":"s","agent":"a","tools":[]},"events":[]}]}"#;
        assert_eq!(Grant::read_all(ok).unwrap().len(), 1);
        // A required field missing, or one of another kind, fails the whole read.
        for bad in [
            r#"{"grants":[{"id":"g","status":"pending","text":"t","request":{"server":"s","agent":"a","tools":[]},"events":[]}]}"#,
            r#"{"grants":[{"id":"g","status":"pending","text":"t","digest":"d","request":{"server":"s","agent":"a","tools":[]},"events":[],"signed_in":"yes"}]}"#,
            r#"{"grants":[{"id":"g","status":"pending","text":"t","digest":"d","request":{"server":"s","agent":"a","tools":[{"name":"x"}]},"events":[]}]}"#,
            r#"{"grants":[{"id":"g","status":"pending","text":"t","digest":"d","request":{"server":"s","agent":"a","tools":[]},"events":[],"actions":{}}]}"#,
            r#"{"grants":{}}"#,
        ] {
            assert!(Grant::read_all(bad.as_bytes()).is_none(), "{bad}");
        }
        // Health and actions are read leniently.
        let g = grant(
            serde_json::json!({"health": "odd", "actions": [5, {"id": "remove", "words": " Remove ", "how": "delete", "stops": "  "}]}),
        );
        assert_eq!(g.health, Some(Health::default()));
        assert!(!g.row_from_planque(), "no health words, no planque row");
        let a = g.actions.as_ref().unwrap();
        assert_eq!(a[0], Action::default());
        assert_eq!((a[1].words.as_str(), a[1].stops.clone()), ("Remove", None));
        assert!(a[1].known() && !a[0].known());
        assert!(!Action {
            id: "remove".into(),
            words: "x".into(),
            how: "approve".into(),
            stops: None
        }
        .known());
        assert!(Action {
            id: "sign_in_again".into(),
            words: "x".into(),
            how: "key".into(),
            stops: None
        }
        .known());
    }

    #[test]
    fn statuses_as_the_ipad_reads_them() {
        let pending = grant(serde_json::json!({"status": "pending"}));
        assert!(pending.pending_request() && pending.cancellable());
        let held = grant(serde_json::json!({"status": "pending", "signed_in": true}));
        assert!(held.holds_sign_in() && !held.pending_request() && !held.cancellable());
        let listed = grant(serde_json::json!({"status": "listed"}));
        assert!(listed.listed() && listed.holds_sign_in() && !listed.cancellable());
        let waiting = grant(serde_json::json!({"status": "approved"}));
        assert!(waiting.waiting_sign_in() && waiting.cancellable());
        // The name: planque's title, else the account, never a provider's id.
        assert_eq!(
            grant(serde_json::json!({"account": " Shop "}))
                .name()
                .as_deref(),
            Some("Shop")
        );
        assert_eq!(
            grant(serde_json::json!({"account": "ca_1", "request": {"server": "s", "agent": "a", "tools": [], "connector": {}}})).name(),
            None
        );
        assert_eq!(
            grant(serde_json::json!({})).vendor(),
            "https://mail.example"
        );
        assert_eq!(
            grant(serde_json::json!({"vendor": "Gmail"})).vendor(),
            "Gmail"
        );
    }

    #[test]
    fn a_declined_sign_in_opens_the_app_so_disconnect_is_in_reach() {
        let gmail = app(serde_json::json!({"entry": "connector:p:gmail", "state": "available"}));
        assert!(!opens_connections(&gmail, &[]));
        // A sign-in the person declined to finish: listed, not connected. The app opens.
        let listed = grant(serde_json::json!({"status": "listed", "entry": "connector:p:gmail"}));
        assert!(opens_connections(&gmail, std::slice::from_ref(&listed)));
        // A disconnected or failed one doesn't.
        for status in ["disconnected", "failed"] {
            let g = grant(serde_json::json!({"status": status, "entry": "connector:p:gmail"}));
            assert!(!opens_connections(&gmail, &[g]));
        }
        // Matched by the grant tafs cloud names too.
        let named = app(serde_json::json!({"entry": "x", "grant": "g1"}));
        assert_eq!(
            connections_of(&named, std::slice::from_ref(&listed)).len(),
            1
        );
        // A connected app always opens, with no connection read.
        let connected = app(serde_json::json!({"entry": "y", "state": "connected"}));
        assert!(opens_connections(&connected, &[]));
    }

    #[test]
    fn grouped_by_taf_attention_first_then_removed_then_others() {
        let g = |id: &str, agent: &str, status: &str| {
            grant(
                serde_json::json!({"id": id, "status": status, "request": {"server": "s", "agent": agent, "tools": []}}),
            )
        };
        let all = [
            g("1", "zed", "connected"),
            g("2", "amy", "connected"),
            g("3", "old", "connected"),
            g("4", "gone", "connected"),
            g("5", "zed2", "needs_reconnect"),
            g("6", "amy", "connected"),
        ];
        let refs: Vec<&Grant> = all.iter().collect();
        let names = |g: &Grant| match g.request.agent.as_str() {
            "zed" => Some("Zed".to_string()),
            "amy" => Some("amy".to_string()),
            "zed2" => Some("Zoe".to_string()),
            _ => None,
        };
        let removed: BTreeSet<String> = ["old".to_string()].into();
        let out = groups(&refs, &names, &removed, &attention);
        assert_eq!(
            out.tafs
                .iter()
                .map(|t| (t.taf.as_str(), t.grants.len()))
                .collect::<Vec<_>>(),
            [("Zoe", 1), ("amy", 2), ("Zed", 1)]
        );
        assert_eq!(
            out.removed
                .iter()
                .map(|g| g.id.as_str())
                .collect::<Vec<_>>(),
            ["3"]
        );
        assert_eq!(
            out.others.iter().map(|g| g.id.as_str()).collect::<Vec<_>>(),
            ["4"]
        );
        assert!(shows_footer(true, &out.tafs));
        assert!(!shows_footer(false, &out.tafs));
        assert!(!shows_footer(true, &[]));
    }

    #[test]
    fn clear_ends_only_what_tafs_cloud_lists_with_nobody_using_it() {
        let file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../shared/contracts/connect-apps-removed.json"
        );
        let v: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
        let page = ConnectApps::read(v["answer"].to_string().as_bytes()).unwrap();
        let listed = removed_listed(&page.removed_tafs);
        assert_eq!(listed.len(), 2);
        let g = |id: &str| {
            grant(
                serde_json::json!({"id": id, "request": {"server": "s", "agent": "old-refunds", "tools": []}}),
            )
        };
        let (a, b, c) = (g("g-old-stripe"), g("g-old-sheets"), g("g-unlisted"));
        assert_eq!(clearable(&[&a, &b, &c], &listed), ["g-old-sheets"]);
        let w = words("en");
        let kept = &listed["g-old-stripe"].used_by;
        let names: Vec<String> = kept.iter().map(|u| u.name.clone()).collect();
        assert_eq!(
            kept_by(&w, &names).unwrap(),
            Some(
                w.get("connect.used_by.one", &[("name", "Chargebacks")])
                    .unwrap()
            )
        );
        assert_eq!(kept_by(&w, &["".into(), " ".into()]).unwrap(), None);
        assert_eq!(
            kept_by(&w, &["Rae".into(), "Sam".into(), "Rae".into()])
                .unwrap()
                .as_deref(),
            Some("Kept: the Cards of Rae and Sam use it")
        );
    }

    #[test]
    fn disconnects_words() {
        let w = words("en");
        assert_eq!(
            disconnect_title(&w, "Stripe", Some("Refunds"), &[]).unwrap(),
            "Remove access to Stripe for Refunds?"
        );
        assert_eq!(
            disconnect_title(&w, "Stripe", None, &["Chargebacks".into()]).unwrap(),
            "Remove access to Stripe? Chargebacks stops too."
        );
        let sign_in = disconnect_message(&w, "Gmail", Some("Refunds"), &[], false).unwrap();
        assert!(sign_in.starts_with(
            &w.get(
                "connect.disconnect.tools_stop",
                &[("owner", "Refunds"), ("vendor", "Gmail")]
            )
            .unwrap()
        ));
        assert!(sign_in.ends_with("Connecting again asks you to approve and sign in again."));
        assert!(sign_in.contains(" Other tafs that use Gmail keep their access. If no other taf uses this sign-in, it's revoked at Gmail."));
        let key = disconnect_message(&w, "Stripe", None, &["A".into(), "B".into()], true).unwrap();
        assert_eq!(
            key,
            "No taf uses Stripe now. The Cards of A and B also use these tools, so they stop too. Other tafs that use Stripe keep their access. The key is deleted from tafs once no taf uses it. Using Stripe again asks you to approve and paste a key again."
        );
        for lang in crate::words::SHIPPED {
            let w = words(lang);
            disconnect_message(&w, "Stripe", Some("Refunds"), &["A".into()], false).unwrap();
            disconnect_title(&w, "Stripe", Some("Refunds"), &["A".into(), "B".into()]).unwrap();
        }
    }

    #[test]
    fn fixing_words() {
        let w = words("en");
        let pending = grant(serde_json::json!({"status": "pending", "vendor": "Gmail"}));
        let waiting = grant(serde_json::json!({"status": "approved", "vendor": "Gmail"}));
        let connected = grant(serde_json::json!({"vendor": "Gmail"}));
        assert_eq!(end_word(&w, &pending).unwrap(), "Cancel the request");
        assert_eq!(
            end_word(&w, &waiting).unwrap(),
            w.line("common.cancel").unwrap()
        );
        assert_eq!(end_word(&w, &connected).unwrap(), "Remove access");
        assert_eq!(
            cancel_title(&w, &pending).unwrap(),
            "Cancel the request for Gmail?"
        );
        assert_eq!(
            cancel_title(&w, &waiting).unwrap(),
            "Cancel the sign-in to Gmail?"
        );
        assert_eq!(cancel_word(&w, &waiting).unwrap(), "Cancel the sign-in");
        assert_eq!(
            status_line(&w, &waiting).unwrap().as_deref(),
            Some("Waiting for the sign-in at Gmail")
        );
        assert_eq!(status_line(&w, &connected).unwrap(), None);
        let g = grant(serde_json::json!({"used_by": ["Refunds", "Chargebacks"]}));
        assert_eq!(
            row_used_by(&w, &g).unwrap().as_deref(),
            Some("Used by Refunds, Chargebacks")
        );
        let a = app(
            serde_json::json!({"entry": "e", "name": "Gmail", "misplaced": [{"grant": "g1", "taf": "t", "name": "Bills"}]}),
        );
        assert_eq!(
            misplaced(&w, &a, &g).unwrap().as_deref(),
            Some("Made for Bills, which doesn't use Gmail.")
        );
    }

    /// Connect apps' connections screens send nothing: removing, clearing and signing in again are
    /// signed, so their taps only say this computer can't approve, and nothing reaches planque.
    #[test]
    fn the_connections_screens_send_nothing() {
        let file = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../tafs-app/src/connections.rs"
        );
        let source = std::fs::read_to_string(file).unwrap();
        for call in ["kiff::", "cloud_send", "kiff_send", "planque::", "ureq"] {
            assert!(!source.contains(call), "{call}");
        }
        // Every signed act goes to `approval::refuse`, never anywhere else.
        assert!(source.contains("crate::approval::refuse("));
        // Connect apps sends nothing: its reads, and Connect's tap once a taf is picked says this
        // computer can't approve. No start is made that only a signature here could finish.
        let file = concat!(env!("CARGO_MANIFEST_DIR"), "/../tafs-app/src/connect.rs");
        let source = std::fs::read_to_string(file).unwrap();
        for call in [
            "cloud_send",
            "kiff_send",
            "start_path",
            "start_body",
            "ConnectStart",
        ] {
            assert!(!source.contains(call), "{call}");
        }
        assert!(source.contains("Act::Connect {"));
        assert!(source.contains("crate::approval::refuse("));
    }

    #[test]
    fn a_keys_host() {
        assert_eq!(host("https://api.stripe.com/v1"), "api.stripe.com");
        assert_eq!(host("stripe"), "stripe");
    }

    #[test]
    fn a_refused_calls_vendor_by_its_connected_tool() {
        let g = |id: &str, status: &str, vendor: &str, tools: &[&str]| {
            grant(serde_json::json!({"id": id, "status": status, "vendor": vendor, "tools": tools}))
        };
        let v = tool_vendors(&[
            g("1", "connected", "Stripe", &["create_refund"]),
            g("2", "connected", "Other", &["create_refund", "list"]),
            g("3", "pending", "Gmail", &["send"]),
        ]);
        assert_eq!(v.get("create_refund").map(String::as_str), Some("Stripe"));
        assert_eq!(v.get("list").map(String::as_str), Some("Other"));
        assert_eq!(v.get("send"), None);
    }
}
