//! Connect apps (`ConnectAppsView`, `ConnectColumns`, `ConnectRow`): what tafs cloud offers to
//! connect (`TafsCloud.ConnectApps`, `GET /v1/connect/apps`), the person's own first, in tafs
//! cloud's order and words. Read as strictly as the iPad's decoder: an app it can't read fails
//! the read, as on the iPad; its words are lenient.

use serde_json::Value;

use crate::words::{list, WordBook, WordsError};

/// One app of the catalog (`ConnectApps.App`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub entry: String,
    pub vendor: String,
    pub title: String,
    pub name: Option<String>,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub logo: Option<String>,
    /// oauth or key.
    pub how: String,
    /// connected, available, needs_reconnect, needs_setup or signed_in.
    pub state: String,
    pub grant: Option<String>,
    pub line: Option<String>,
    pub method: Option<String>,
    pub connected_flag: bool,
    pub setup_words: Option<String>,
    pub whose: Option<String>,
    /// tafs cloud's names of the tafs that use it; none when it sends none.
    pub used_by: Option<Vec<String>>,
    /// The tafs that need it (`needed_by`), by tafs cloud's id and name: "Which taf?" lists only
    /// these, and none means Ask Coach. None from a tafs cloud before it, or a list it can't read.
    pub needed_by: Option<Vec<UsedBy>>,
    /// Its live connections made under a taf that doesn't need it (`misplaced`).
    pub misplaced: Vec<Misplaced>,
}

/// A taf by tafs cloud's id and name (`ConnectApps.UsedBy`): each field a string where it's there,
/// else empty.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct UsedBy {
    pub taf: String,
    pub name: String,
}

impl UsedBy {
    fn read(v: &Value) -> Result<UsedBy, ()> {
        if !v.is_object() {
            return Err(());
        }
        Ok(UsedBy {
            taf: strict_text(v, "taf")?.unwrap_or_default(),
            name: strict_text(v, "name")?.unwrap_or_default(),
        })
    }
}

/// A live connection of an app made under a taf that doesn't need it (`ConnectApps.Misplaced`):
/// its grant, and that taf.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Misplaced {
    pub grant: String,
    pub taf: String,
    pub name: String,
}

impl Misplaced {
    fn read(v: &Value) -> Result<Misplaced, ()> {
        Ok(Misplaced {
            grant: v
                .get("grant")
                .and_then(Value::as_str)
                .ok_or(())?
                .to_string(),
            taf: strict_text(v, "taf")?.unwrap_or_default(),
            name: strict_text(v, "name")?.unwrap_or_default(),
        })
    }
}

/// A removed taf's live connection, with the live tafs whose Cards still use it
/// (`ConnectApps.RemovedGrant`): Clear keeps those.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemovedGrant {
    pub grant: String,
    pub used_by: Vec<UsedBy>,
}

/// A taf removed since, as tafs cloud names it from KIFF's archived agents
/// (`ConnectApps.RemovedTaf`). Its live connections come only with this device's signed grants
/// read: none means no Clear.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemovedTaf {
    pub agent: String,
    pub removed_at: Option<String>,
    pub grants: Option<Vec<RemovedGrant>>,
}

impl RemovedTaf {
    fn read(t: &Value) -> Result<RemovedTaf, ()> {
        fn list<T>(
            v: &Value,
            key: &str,
            each: fn(&Value) -> Result<T, ()>,
        ) -> Result<Option<Vec<T>>, ()> {
            match v.get(key) {
                None | Some(Value::Null) => Ok(None),
                Some(Value::Array(a)) => a.iter().map(each).collect::<Result<_, _>>().map(Some),
                Some(_) => Err(()),
            }
        }
        fn grant(g: &Value) -> Result<RemovedGrant, ()> {
            Ok(RemovedGrant {
                grant: g
                    .get("grant")
                    .and_then(Value::as_str)
                    .ok_or(())?
                    .to_string(),
                used_by: list(g, "used_by", UsedBy::read)?.unwrap_or_default(),
            })
        }
        Ok(RemovedTaf {
            agent: t
                .get("agent")
                .and_then(Value::as_str)
                .ok_or(())?
                .to_string(),
            removed_at: strict_text(t, "removed_at")?,
            grants: list(t, "grants", grant)?,
        })
    }
}

/// A string where it's there; a value of another kind fails the read (`decodeIfPresent`).
fn strict_text(v: &Value, key: &str) -> Result<Option<String>, ()> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(()),
    }
}

/// tafs cloud's words: trimmed, none when blank or not a string.
fn words(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

impl App {
    fn read(v: &Value) -> Result<App, ()> {
        Ok(App {
            entry: v
                .get("entry")
                .and_then(Value::as_str)
                .ok_or(())?
                .to_string(),
            vendor: strict_text(v, "vendor")?.unwrap_or_default(),
            title: strict_text(v, "title")?.unwrap_or_default(),
            icon: strict_text(v, "icon")?,
            logo: v.get("logo").and_then(Value::as_str).map(str::to_string),
            how: strict_text(v, "how")?.unwrap_or_else(|| "oauth".into()),
            state: strict_text(v, "state")?.unwrap_or_else(|| "available".into()),
            grant: strict_text(v, "grant")?,
            name: words(v, "name"),
            subtitle: words(v, "subtitle"),
            line: words(v, "line"),
            whose: words(v, "whose"),
            method: words(v, "method"),
            setup_words: words(v, "words"),
            connected_flag: v.get("connected").and_then(Value::as_bool).unwrap_or(false),
            // [{taf, name}], by tafs cloud's names; anything else reads as none sent.
            used_by: v.get("used_by").and_then(Value::as_array).and_then(|a| {
                let named: Option<Vec<Option<String>>> = a
                    .iter()
                    .map(|n| {
                        let o = n.as_object()?;
                        match o.get("name") {
                            None | Some(Value::Null) => Some(None),
                            Some(Value::String(s)) => Some(Some(s.trim().to_string())),
                            Some(_) => None,
                        }
                    })
                    .collect();
                named.map(|n| n.into_iter().flatten().filter(|s| !s.is_empty()).collect())
            }),
            // A list it can't read is none sent: every taf you manage, never a guess from half a
            // list. One without its taf's id is left out.
            needed_by: v
                .get("needed_by")
                .and_then(Value::as_array)
                .and_then(|a| {
                    a.iter()
                        .map(UsedBy::read)
                        .collect::<Result<Vec<_>, _>>()
                        .ok()
                })
                .map(|n| n.into_iter().filter(|u| !u.taf.is_empty()).collect()),
            misplaced: v
                .get("misplaced")
                .and_then(Value::as_array)
                .and_then(|a| {
                    a.iter()
                        .map(Misplaced::read)
                        .collect::<Result<Vec<_>, _>>()
                        .ok()
                })
                .unwrap_or_default(),
        })
    }

    /// `displayName`: tafs cloud's `name`, else its `title`.
    pub fn display_name(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.title)
    }
    pub fn connected(&self) -> bool {
        self.state == "connected" || self.connected_flag
    }
    /// Connects with a key, by `method` where tafs cloud sends it, else `how`.
    pub fn takes_key(&self) -> bool {
        self.method.as_deref().unwrap_or(&self.how) == "key"
    }
    pub fn needs_reconnect(&self) -> bool {
        self.state == "needs_reconnect"
    }
    /// Not set up for anyone to connect yet: no Connect, only tafs cloud's words.
    pub fn needs_setup(&self) -> bool {
        self.state == "needs_setup"
    }
    /// Connected, signed in, or to sign in again: the person's own, under Yours.
    pub fn yours(&self) -> bool {
        self.connected() || self.needs_reconnect() || self.state == "signed_in"
    }
}

/// A page of Connect apps (`TafsCloud.ConnectApps`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConnectApps {
    pub apps: Vec<App>,
    /// The tafs removed since, newest first: a connection whose `request.agent` is one is a
    /// removed taf's.
    pub removed_tafs: Vec<RemovedTaf>,
    /// Coach can draft a vendor that isn't listed.
    pub by_key: bool,
    pub provider: Option<String>,
    pub next_cursor: Option<String>,
}

impl ConnectApps {
    /// tafs cloud's answer; none when any app, or a field of the page, can't be read.
    pub fn read(body: &[u8]) -> Option<ConnectApps> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let apps = match v.get("apps") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(a)) => a.iter().map(App::read).collect::<Result<_, _>>().ok()?,
            Some(_) => return None,
        };
        let by_key = match v.get("by_key") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => return None,
        };
        // `removed_tafs` must read too, as on the iPad (`RemovedTaf`, `RemovedGrant`, `UsedBy`).
        let removed_tafs = match v.get("removed_tafs") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(a)) => a
                .iter()
                .map(RemovedTaf::read)
                .collect::<Result<_, _>>()
                .ok()?,
            Some(_) => return None,
        };
        Some(ConnectApps {
            apps,
            removed_tafs,
            by_key,
            provider: strict_text(&v, "provider").ok()?,
            next_cursor: strict_text(&v, "next_cursor").ok()?,
        })
    }

    /// `loadMoreConnectApps`: the next page's apps added after those shown, none twice; `by_key`
    /// from either; the next page's cursor.
    pub fn add(&mut self, page: ConnectApps) {
        let seen: std::collections::BTreeSet<String> =
            self.apps.iter().map(|a| a.entry.clone()).collect();
        self.apps
            .extend(page.apps.into_iter().filter(|a| !seen.contains(&a.entry)));
        self.by_key = self.by_key || page.by_key;
        // The removed tafs stay as the first page read them, with its proof: a further page only
        // adds apps.
        self.provider = page.provider;
        self.next_cursor = page.next_cursor;
    }
}

/// `GET /v1/connect/apps` with what the person typed (trimmed, none when blank), the provider and
/// the cursor, each escaped; a cursor's `+` stays a value, never a space.
pub fn path(q: &str, provider: Option<&str>, cursor: Option<&str>) -> String {
    let escaped = |v: &str| {
        url::form_urlencoded::byte_serialize(v.as_bytes())
            .collect::<String>()
            .replace('+', "%20")
    };
    let mut items = Vec::new();
    let q = q.trim();
    if !q.is_empty() {
        items.push(format!("q={}", escaped(q)));
    }
    if let Some(p) = provider {
        items.push(format!("provider={}", escaped(p)));
    }
    if let Some(c) = cursor {
        items.push(format!("cursor={}", escaped(c)));
    }
    if items.is_empty() {
        "/v1/connect/apps".into()
    } else {
        format!("/v1/connect/apps?{}", items.join("&"))
    }
}

/// What a row says (`ConnectRow`): how it connects, whose it is, where it stands, which tafs use
/// it, tafs cloud's line, and tafs cloud's words for an app to set up.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub how: String,
    pub whose: Option<String>,
    pub status: Option<String>,
    pub used_by: Option<String>,
    pub line: Option<String>,
    pub setup: Option<String>,
}

impl Row {
    /// `used_here`: the tafs this computer's read of the connections names, until tafs cloud
    /// sends `used_by` (none read on Linux yet).
    pub fn new(w: &WordBook, app: &App, used_here: &[String]) -> Result<Row, WordsError> {
        let names = app.used_by.as_deref().unwrap_or(used_here);
        Ok(Row {
            how: w.line(if app.takes_key() {
                "connect.how.key"
            } else {
                "connect.how.sign_in"
            })?,
            whose: app.whose.clone(),
            status: if app.connected() {
                Some(w.line("connect.status.connected")?)
            } else if app.needs_reconnect() {
                Some(w.line("connect.sign_in_again")?)
            } else {
                None
            },
            used_by: if app.yours() && !names.is_empty() {
                Some(w.get(
                    "connect.apps.used_by",
                    &[("tafs", &list(w.language(), names))],
                )?)
            } else {
                None
            },
            line: app.line.clone(),
            setup: app
                .needs_setup()
                .then(|| app.setup_words.clone().unwrap_or_default()),
        })
    }

    /// The narrow window's second line: an app to set up says tafs cloud's words; else tafs
    /// cloud's line; else how, whose, where it stands and who uses it.
    pub fn second(&self) -> String {
        if let Some(s) = &self.setup {
            return s.clone();
        }
        if let Some(l) = &self.line {
            return l.clone();
        }
        [
            Some(self.how.clone()),
            self.whose.clone(),
            self.status.clone(),
            self.used_by.clone(),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
    }
}

/// `ConnectRow.split`: the person's own first, then the rest, each in tafs cloud's order. An app
/// under Yours isn't under Discover again in another of its catalog forms.
pub fn split(apps: &[App]) -> (Vec<&App>, Vec<&App>) {
    let yours: Vec<&App> = apps.iter().filter(|a| a.yours()).collect();
    let had: std::collections::BTreeSet<String> = yours.iter().flat_map(|a| keys(a)).collect();
    let discover = apps
        .iter()
        .filter(|a| !a.yours() && keys(a).is_disjoint(&had))
        .collect();
    (yours, discover)
}

/// `ConnectRow.keys`: the names one app goes by across the catalog's forms: its entry's last part
/// without its version, and its vendor, folded.
pub fn keys(app: &App) -> std::collections::BTreeSet<String> {
    let last = app.entry.rsplit(':').next().unwrap_or(&app.entry);
    // Without a trailing "@<digits>".
    let slug = match last.rfind('@') {
        Some(i) if i + 1 < last.len() && last[i + 1..].chars().all(|c| c.is_ascii_digit()) => {
            &last[..i]
        }
        _ => last,
    };
    [slug, app.vendor.as_str()]
        .iter()
        .map(|n| fold(n))
        .filter(|n| !n.is_empty())
        .collect()
}

fn fold(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && !".-_".contains(*c))
        .collect()
}

/// `ConnectRow.saysMakeATaf`: "Make one first" over Discover only, where there are apps to
/// connect, the list wasn't opened from a taf, and no taf can have them.
pub fn says_make_a_taf(discover: usize, for_taf: bool, connectable: usize) -> bool {
    discover > 0 && !for_taf && connectable == 0
}

/// Whom a Connect is for (`ConnectRow.ConnectFor`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectFor {
    /// The taf the list was opened from, by tafs cloud's id.
    Taf(String),
    /// The person picks, in "Which taf is this for?".
    Ask,
    /// No taf needs it: Coach sets one up.
    AskCoach,
    /// No taf can have it: Connect is off, and "Make one first" says why.
    NoTaf,
}

/// `ConnectRow.connectFor`: a connection goes only to a taf the person named, among those that
/// need the app when tafs cloud says (`needed`): the taf the list was opened from, else their
/// pick, asked even when one taf is all there is. With `needed_by`, none, or a taf opened from
/// that doesn't need it, is Ask Coach. Without it, every taf the person manages. Never a guess.
pub fn connect_for(opened_from: Option<&str>, connectable: &[String], needed: bool) -> ConnectFor {
    if !needed {
        if let Some(t) = opened_from {
            return ConnectFor::Taf(t.to_string());
        }
        return if connectable.is_empty() {
            ConnectFor::NoTaf
        } else {
            ConnectFor::Ask
        };
    }
    if let Some(t) = opened_from {
        return if connectable.iter().any(|c| c == t) {
            ConnectFor::Taf(t.to_string())
        } else {
            ConnectFor::AskCoach
        };
    }
    if connectable.is_empty() {
        ConnectFor::AskCoach
    } else {
        ConnectFor::Ask
    }
}

/// A taf an app can be connected for (`connectableTafs`): on tafs cloud, managed by this person.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Connectable {
    /// tafs cloud's id.
    pub id: String,
    pub name: String,
}

/// `connectableTafs(for:)`: of the tafs an app can be connected for, the ones tafs cloud says need
/// it (`needed_by`), in its order. A tafs cloud that doesn't say leaves every one.
pub fn connectable_for(tafs: &[Connectable], needed: Option<&[UsedBy]>) -> Vec<Connectable> {
    match needed {
        None => tafs.to_vec(),
        Some(needed) => needed
            .iter()
            .filter_map(|n| tafs.iter().find(|t| t.id == n.taf).cloned())
            .collect(),
    }
}

/// The app's name in "Which taf?" and in why it couldn't be connected: tafs cloud's `name`, else
/// its vendor, else its title.
pub fn called(app: &App) -> &str {
    app.name.as_deref().unwrap_or(if app.vendor.is_empty() {
        &app.title
    } else {
        &app.vendor
    })
}

/// `startConnect`: `POST /v1/connect/apps/{entry}` (escaped as `TafsCloud.escaped`: the path's
/// characters but `/`) with the taf's tafs cloud id. planque makes the request for that taf's
/// KIFF agent; nothing is approved or signed by it.
pub fn start_path(entry: &str) -> String {
    format!("/v1/connect/apps/{}", path_escaped(entry))
}

pub fn start_body(taf: &str) -> Vec<u8> {
    serde_json::json!({ "taf": taf }).to_string().into_bytes()
}

/// `addingPercentEncoding(withAllowedCharacters: .urlPathAllowed` without `/`): letters, digits
/// and `-._~!$&'()*+,;=:@` stay; anything else is %XX.
pub fn path_escaped(v: &str) -> String {
    v.bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~!$&'()*+,;=:@".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect()
}

/// What starting a connection answers (`TafsCloud.ConnectStart`): the request to approve, and
/// whether it's approved with a key the owner gives. `grant` is planque's id, or its grant as
/// tafs cloud passes it on, read for its id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConnectStart {
    pub grant: String,
    pub needs_key: bool,
}

impl ConnectStart {
    pub fn read(body: &[u8]) -> Option<ConnectStart> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let grant = match v.get("grant")? {
            Value::String(s) => s.clone(),
            g => g.get("id")?.as_str()?.to_string(),
        };
        let needs_key = match v.get("needs_key") {
            None | Some(Value::Null) => false,
            Some(Value::Bool(b)) => *b,
            Some(_) => return None,
        };
        Some(ConnectStart { grant, needs_key })
    }
}

/// tafs cloud's code for a taf that doesn't use the app: its words send the person to Coach.
pub const NOT_NEEDED: &str = "not_needed";

/// `ConnectColumns`: the table's four widths in `total`: App, How it connects, Whose, Status.
/// The App column has its room first; the other three give way together, each in proportion to
/// what it can give, down to their least, so App keeps its own least.
pub const SPACING: f64 = 12.0;
pub const APP_LEAST: f64 = 200.0;
pub const WIDE: [f64; 3] = [150.0, 130.0, 230.0];
pub const LEAST: [f64; 3] = [96.0, 84.0, 180.0];

pub fn widths(total: f64) -> [f64; 4] {
    let room = (total - SPACING * 3.0).max(0.0);
    let wide_sum: f64 = WIDE.iter().sum();
    if room - wide_sum >= APP_LEAST {
        return [room - wide_sum, WIDE[0], WIDE[1], WIDE[2]];
    }
    let least_sum: f64 = LEAST.iter().sum();
    let give = wide_sum - least_sum;
    let need = give.min(APP_LEAST - (room - wide_sum));
    let mut others = [0.0; 3];
    for i in 0..3 {
        others[i] = if give > 0.0 {
            WIDE[i] - (WIDE[i] - LEAST[i]) * need / give
        } else {
            WIDE[i]
        };
    }
    let app = (room - others.iter().sum::<f64>()).max(0.0);
    [app, others[0], others[1], others[2]]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(entry: &str, vendor: &str, state: &str) -> App {
        App::read(&serde_json::json!({"entry": entry, "vendor": vendor, "state": state})).unwrap()
    }

    #[test]
    fn one_unreadable_app_fails_the_read_as_on_the_ipad() {
        assert!(ConnectApps::read(br#"{"apps":[{"entry":"a"}],"by_key":true}"#).is_some());
        assert!(ConnectApps::read(br#"{"apps":[{"entry":"a"},{"vendor":"x"}]}"#).is_none());
        assert!(ConnectApps::read(br#"{"apps":[{"entry":"a","state":3}]}"#).is_none());
        assert!(ConnectApps::read(br#"{"removed_tafs":[{"agent":"a","grants":[{"grant":"g","used_by":[{"name":"R"}]}]}]}"#).is_some());
        assert!(ConnectApps::read(
            br#"{"removed_tafs":[{"agent":"a","grants":[{"used_by":[]}]}]}"#
        )
        .is_none());
        // Words are lenient: a blank or odd one reads as none.
        let page = ConnectApps::read(br#"{"apps":[{"entry":"a","name":"  ","line":5}]}"#).unwrap();
        assert_eq!(page.apps[0].name, None);
        assert_eq!(page.apps[0].line, None);
    }

    #[test]
    fn used_by_reads_names_or_nothing() {
        let a =
            App::read(&serde_json::json!({"entry":"a","used_by":[{"taf":"t","name":" Refunds "}]}))
                .unwrap();
        assert_eq!(a.used_by, Some(vec!["Refunds".to_string()]));
        // Plain names aren't tafs cloud's form: none sent, as the iPad's decoder reads them.
        let a = App::read(&serde_json::json!({"entry":"a","used_by":["Refunds"]})).unwrap();
        assert_eq!(a.used_by, None);
    }

    #[test]
    fn yours_first_and_never_twice() {
        let apps = vec![
            app("connector:composio:stripe", "Stripe", "connected"),
            app("stripe@3", "Stripe", "available"),
            app("qonto@1", "Qonto", "available"),
            app("x", "Gmail", "needs_reconnect"),
        ];
        let (yours, discover) = split(&apps);
        assert_eq!(
            yours.iter().map(|a| a.entry.as_str()).collect::<Vec<_>>(),
            ["connector:composio:stripe", "x"]
        );
        assert_eq!(
            discover
                .iter()
                .map(|a| a.entry.as_str())
                .collect::<Vec<_>>(),
            ["qonto@1"]
        );
        assert_eq!(
            keys(&app(
                "connector:composio:google-sheets",
                "Google Sheets",
                "available"
            )),
            ["googlesheets".to_string()].into_iter().collect()
        );
    }

    #[test]
    fn more_apps_are_added_once() {
        let mut shown = ConnectApps {
            removed_tafs: vec![RemovedTaf {
                agent: "old".into(),
                ..RemovedTaf::default()
            }],
            apps: vec![app("a", "", "available")],
            by_key: true,
            provider: Some("p".into()),
            next_cursor: Some("c".into()),
        };
        shown.add(ConnectApps {
            removed_tafs: vec![],
            apps: vec![app("a", "", "available"), app("b", "", "available")],
            by_key: false,
            provider: Some("p".into()),
            next_cursor: None,
        });
        assert_eq!(shown.apps.len(), 2);
        assert!(shown.by_key);
        assert_eq!(shown.next_cursor, None);
        // The removed tafs stay as the first page read them.
        assert_eq!(shown.removed_tafs.len(), 1);
    }

    #[test]
    fn needed_by_and_misplaced_read_leniently() {
        let a = App::read(&serde_json::json!({"entry": "a", "needed_by": [{"taf": "t1", "name": "Refunds"}, {"name": "No id"}], "misplaced": [{"grant": "g", "name": "Bills"}]})).unwrap();
        assert_eq!(
            a.needed_by,
            Some(vec![UsedBy {
                taf: "t1".into(),
                name: "Refunds".into()
            }])
        );
        assert_eq!(a.misplaced[0].grant, "g");
        // A list it can't read is none sent, never half a list.
        let a = App::read(&serde_json::json!({"entry": "a", "needed_by": [{"taf": 1}], "misplaced": [{"taf": "t"}]})).unwrap();
        assert_eq!(a.needed_by, None);
        assert!(a.misplaced.is_empty());
        // An empty list is tafs cloud saying no taf needs it.
        let a = App::read(&serde_json::json!({"entry": "a", "needed_by": []})).unwrap();
        assert_eq!(a.needed_by, Some(vec![]));
    }

    #[test]
    fn the_contracts_read() {
        for name in [
            "connect-apps",
            "connect-apps-removed",
            "connect-apps-next",
            "connect-apps-search",
        ] {
            let file = format!(
                "{}/../../shared/contracts/{name}.json",
                env!("CARGO_MANIFEST_DIR")
            );
            let v: Value = serde_json::from_slice(&std::fs::read(file).unwrap()).unwrap();
            let page = ConnectApps::read(v["answer"].to_string().as_bytes()).unwrap();
            assert!(!page.apps.is_empty(), "{name}");
            assert_eq!(page.removed_tafs[0].agent, "old-refunds", "{name}");
        }
    }

    #[test]
    fn whom_a_connect_is_for_never_a_guess() {
        let one = vec!["t1".to_string()];
        let two = vec!["t1".to_string(), "t2".to_string()];
        assert_eq!(
            connect_for(None, &one, false),
            ConnectFor::Ask,
            "one taf is still asked"
        );
        assert_eq!(connect_for(None, &two, false), ConnectFor::Ask);
        assert_eq!(
            connect_for(Some("t2"), &two, false),
            ConnectFor::Taf("t2".into())
        );
        assert_eq!(connect_for(None, &[], false), ConnectFor::NoTaf);
        assert_eq!(connect_for(None, &one, true), ConnectFor::Ask);
        assert_eq!(connect_for(None, &[], true), ConnectFor::AskCoach);
        assert_eq!(
            connect_for(Some("t2"), &two, true),
            ConnectFor::Taf("t2".into())
        );
        assert_eq!(
            connect_for(Some("t9"), &one, true),
            ConnectFor::AskCoach,
            "Stripe under a Gmail taf"
        );
        let tafs = vec![
            Connectable {
                id: "t1".into(),
                name: "Refunds".into(),
            },
            Connectable {
                id: "t2".into(),
                name: "Bills".into(),
            },
        ];
        assert_eq!(connectable_for(&tafs, None), tafs);
        let needed = [
            UsedBy {
                taf: "t2".into(),
                name: "".into(),
            },
            UsedBy {
                taf: "t9".into(),
                name: "".into(),
            },
        ];
        assert_eq!(connectable_for(&tafs, Some(&needed)), vec![tafs[1].clone()]);
        assert!(connectable_for(&tafs, Some(&[])).is_empty());
    }

    #[test]
    fn starting_a_connection() {
        assert_eq!(
            start_path("connector:p:gmail"),
            "/v1/connect/apps/connector:p:gmail"
        );
        assert_eq!(start_path("a/b c"), "/v1/connect/apps/a%2Fb%20c");
        assert_eq!(start_body("t1"), br#"{"taf":"t1"}"#);
        assert_eq!(
            ConnectStart::read(br#"{"grant":"g1","needs_key":true}"#),
            Some(ConnectStart {
                grant: "g1".into(),
                needs_key: true
            })
        );
        assert_eq!(
            ConnectStart::read(br#"{"grant":{"id":"g2","status":"pending"}}"#),
            Some(ConnectStart {
                grant: "g2".into(),
                needs_key: false
            })
        );
        assert_eq!(ConnectStart::read(br#"{"grant":5}"#), None);
        assert_eq!(
            ConnectStart::read(br#"{"grant":"g","needs_key":"yes"}"#),
            None
        );
        // Starting signs nothing: KIFF never takes it, and it's tafs cloud's, as the session.
        assert!(!crate::signed::signed("POST", &start_path("x")));
        let a = App::read(
            &serde_json::json!({"entry": "a", "vendor": "Stripe", "title": "Stripe payments"}),
        )
        .unwrap();
        assert_eq!(called(&a), "Stripe");
        let a = App::read(&serde_json::json!({"entry": "a", "title": "Stripe payments"})).unwrap();
        assert_eq!(called(&a), "Stripe payments");
    }

    #[test]
    fn paths() {
        assert_eq!(path("  ", None, None), "/v1/connect/apps");
        assert_eq!(
            path(" google sheets ", Some("composio"), Some("a+b/c")),
            "/v1/connect/apps?q=google%20sheets&provider=composio&cursor=a%2Bb%2Fc"
        );
    }

    #[test]
    fn columns_give_way_as_the_ipads() {
        assert_eq!(widths(1000.0), [1000.0 - 36.0 - 510.0, 150.0, 130.0, 230.0]);
        // Too narrow for App's least at the wide widths: the three give way in proportion.
        let w = widths(600.0);
        assert!((w[0] - 200.0).abs() < 1e-9);
        assert!(w[1] < 150.0 && w[1] >= 96.0);
        // Narrower still: the three at their least, App takes what's left.
        let w = widths(400.0);
        assert_eq!([w[1], w[2], w[3]], LEAST);
        assert!((w[0] - (400.0 - 36.0 - 360.0)).abs() < 1e-9);
    }

    #[test]
    fn make_a_taf_first() {
        assert!(says_make_a_taf(4, false, 0));
        assert!(!says_make_a_taf(0, false, 0));
        assert!(!says_make_a_taf(4, true, 0));
        assert!(!says_make_a_taf(4, false, 1));
    }
}
