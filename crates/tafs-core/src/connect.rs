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

/// A list that's absent, null, or each of whose elements reads; anything else fails.
fn list_of(v: &Value, key: &str, reads: fn(&Value) -> bool) -> bool {
    match v.get(key) {
        None | Some(Value::Null) => true,
        Some(Value::Array(a)) => a.iter().all(reads),
        Some(_) => false,
    }
}

/// `RemovedTaf`: its `agent`, `removed_at` a string when there, `grants` each with its `grant`
/// and its `used_by` each with `taf` and `name` strings when there.
fn removed_taf_reads(t: &Value) -> bool {
    fn used_by_reads(u: &Value) -> bool {
        u.is_object() && strict_text(u, "taf").is_ok() && strict_text(u, "name").is_ok()
    }
    fn grant_reads(g: &Value) -> bool {
        g.get("grant").and_then(Value::as_str).is_some() && list_of(g, "used_by", used_by_reads)
    }
    t.get("agent").and_then(Value::as_str).is_some()
        && strict_text(t, "removed_at").is_ok()
        && list_of(t, "grants", grant_reads)
}

/// A page of Connect apps (`TafsCloud.ConnectApps`). Removed tafs' connections need this
/// computer's signed grants read, which Linux doesn't send yet: they're read and not drawn.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConnectApps {
    pub apps: Vec<App>,
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
        if !list_of(&v, "removed_tafs", removed_taf_reads) {
            return None;
        }
        Some(ConnectApps {
            apps,
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
            apps: vec![app("a", "", "available")],
            by_key: true,
            provider: Some("p".into()),
            next_cursor: Some("c".into()),
        };
        shown.add(ConnectApps {
            apps: vec![app("a", "", "available"), app("b", "", "available")],
            by_key: false,
            provider: Some("p".into()),
            next_cursor: None,
        });
        assert_eq!(shown.apps.len(), 2);
        assert!(shown.by_key);
        assert_eq!(shown.next_cursor, None);
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
