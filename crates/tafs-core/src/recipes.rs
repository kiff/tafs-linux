//! The recipes Home lists with no taf yet (`TafsCloud.Recipes`, `GET /v1/recipes`): tafs cloud's
//! line, "Say it in your own words", a category per chip and each recipe led by the problem it
//! ends. Every word is tafs cloud's, in the person's language; the app has none of its own here.
//!
//! Read exactly as the iPad's decoders read it: what a `try` there requires, this requires (a
//! category's or a recipe's id and title, and the kind of `category`, `line`, `say` and `shown`),
//! and what a `try?` there forgives (a colour, an app, a headline, the Card, `home`, `labels`),
//! this forgives.

use serde_json::{Map, Value};

/// A colour as tafs cloud sends it, `#RRGGBB` for light and for dark (`Tint`). Either one not
/// readable, there's no colour: the app never guesses one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Tint {
    pub light: u32,
    pub dark: u32,
}

impl Tint {
    /// `#RRGGBB`, and only that.
    pub fn hex(s: &str) -> Option<u32> {
        let digits = s.strip_prefix('#')?;
        if s.chars().count() != 7 || !digits.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(digits, 16).ok()
    }

    fn read(v: &Value) -> Option<Tint> {
        Some(Tint {
            light: Tint::hex(v.get("light")?.as_str()?)?,
            dark: Tint::hex(v.get("dark")?.as_str()?)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    pub id: String,
    pub title: String,
    pub color: Option<Tint>,
}

/// One of the apps a recipe's taf uses, with its logo's path on tafs cloud when there is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct App {
    pub name: String,
    pub logo: Option<String>,
}

/// The Card a recipe's taf would have: what it does on its own, asks first, and never does.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Card {
    pub allows: Vec<String>,
    pub holds: Vec<String>,
    pub never: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub id: String,
    pub title: String,
    pub category: String,
    pub line: String,
    /// What handing it to Coach puts in Coach's message box.
    pub say: String,
    pub headline: String,
    pub today: String,
    pub card: Card,
    pub apps: Vec<App>,
    pub connected: bool,
    pub running: bool,
    pub proved: String,
    pub sample: String,
}

impl Recipe {
    /// `lead`: the problem it ends; its title from a tafs cloud from before headlines.
    pub fn lead(&self) -> &str {
        if self.headline.trim().is_empty() {
            &self.title
        } else {
            &self.headline
        }
    }
}

/// "Say it in your own words" and its line (`Home.OwnWords`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnWords {
    pub title: String,
    pub line: String,
}

/// What Home shows with no taf (`Recipes.Home`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct HomeWords {
    pub shown: bool,
    pub words: String,
    /// None when tafs cloud sent none, or one with no title.
    pub own_words: Option<OwnWords>,
}

/// The recipe sheet's words (`Labels`): its three zones and its button.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Labels {
    pub allows: String,
    pub holds: String,
    pub never: String,
    pub hand: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipes {
    pub categories: Vec<Category>,
    pub recipes: Vec<Recipe>,
    pub shown: bool,
    pub home: Option<HomeWords>,
    pub labels: Option<Labels>,
}

/// Where an app's logo loads from (`TafsCloud.logoURL`): only ever tafs cloud, at
/// `/v1/logos/{key}` with a key of 32 lowercase hex digits. Anything else is no logo, and the
/// app's name stands alone.
pub fn logo_url(path: Option<&str>) -> Option<String> {
    let key = path?.strip_prefix("/v1/logos/")?;
    if key.len() != 32
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    Some(format!("{}/v1/logos/{key}", crate::home::TAFS_CLOUD))
}

/// One zone of a recipe's Card on its sheet (`Zone`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Zone {
    pub id: &'static str,
    pub title: String,
    pub lines: Vec<String>,
}

// `try c.decode(String.self)`: there, and a string.
fn required_str(o: &Map<String, Value>, k: &str) -> Option<String> {
    o.get(k)?.as_str().map(str::to_string)
}

// `try c.decodeIfPresent(String.self) ?? ""`: absent or null is "", any other kind fails.
fn strict_str(o: &Map<String, Value>, k: &str) -> Result<String, ()> {
    match o.get(k) {
        None | Some(Value::Null) => Ok(String::new()),
        Some(Value::String(s)) => Ok(s.clone()),
        Some(_) => Err(()),
    }
}

// `(try? c.decodeIfPresent(String.self)) ?? ""`: anything but a string is "".
fn lenient_str(o: &Map<String, Value>, k: &str) -> String {
    o.get(k).and_then(Value::as_str).unwrap_or("").to_string()
}

fn lenient_bool(o: &Map<String, Value>, k: &str) -> bool {
    o.get(k).and_then(Value::as_bool).unwrap_or(false)
}

// `(try? c.decodeIfPresent([String].self)) ?? []`: a list of strings, else none.
fn lenient_strings(o: &Map<String, Value>, k: &str) -> Vec<String> {
    o.get(k)
        .and_then(Value::as_array)
        .and_then(|a| a.iter().map(|v| v.as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

fn category(v: &Value) -> Option<Category> {
    let o = v.as_object()?;
    Some(Category {
        id: required_str(o, "id")?,
        title: required_str(o, "title")?,
        color: o.get("color").and_then(Tint::read),
    })
}

fn recipe(v: &Value) -> Option<Recipe> {
    let o = v.as_object()?;
    let card = o
        .get("card")
        .and_then(Value::as_object)
        .map(|c| Card {
            allows: lenient_strings(c, "allows"),
            holds: lenient_strings(c, "holds"),
            never: lenient_strings(c, "never"),
        })
        .unwrap_or_default();
    // `LenientItem<App>`: an app without its name is left out.
    let apps = o
        .get("apps")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|app| {
                    let app = app.as_object()?;
                    Some(App {
                        name: required_str(app, "name")?,
                        logo: app.get("logo").and_then(Value::as_str).map(str::to_string),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    Some(Recipe {
        id: required_str(o, "id")?,
        title: required_str(o, "title")?,
        category: strict_str(o, "category").ok()?,
        line: strict_str(o, "line").ok()?,
        say: strict_str(o, "say").ok()?,
        headline: lenient_str(o, "headline"),
        today: lenient_str(o, "today"),
        card,
        apps,
        connected: lenient_bool(o, "connected"),
        running: lenient_bool(o, "running"),
        proved: lenient_str(o, "proved").trim().to_string(),
        sample: lenient_str(o, "sample").trim().to_string(),
    })
}

fn home_words(v: &Value) -> Option<HomeWords> {
    let o = v.as_object()?;
    let own_words = o.get("own_words").and_then(Value::as_object).and_then(|w| {
        let title = lenient_str(w, "title").trim().to_string();
        // A row with no title isn't one.
        (!title.is_empty()).then(|| OwnWords {
            title,
            line: lenient_str(w, "line").trim().to_string(),
        })
    });
    Some(HomeWords {
        shown: lenient_bool(o, "shown"),
        words: lenient_str(o, "words"),
        own_words,
    })
}

fn labels(v: &Value) -> Option<Labels> {
    let o = v.as_object()?;
    Some(Labels {
        allows: lenient_str(o, "allows"),
        holds: lenient_str(o, "holds"),
        never: lenient_str(o, "never"),
        hand: lenient_str(o, "hand"),
    })
}

// `try c.decodeIfPresent([T].self) ?? []`: absent or null is none; anything else must all read.
fn strict_list<T>(
    o: &Map<String, Value>,
    k: &str,
    read: fn(&Value) -> Option<T>,
) -> Option<Vec<T>> {
    match o.get(k) {
        None | Some(Value::Null) => Some(vec![]),
        Some(Value::Array(a)) => a.iter().map(read).collect(),
        Some(_) => None,
    }
}

impl Recipes {
    /// `GET /v1/recipes`, read as the iPad reads it; none when it can't be (the iPad keeps what
    /// it had, and Home shows no recipes).
    pub fn read(body: &[u8]) -> Option<Recipes> {
        let v: Value = serde_json::from_slice(body).ok()?;
        let o = v.as_object()?;
        Some(Recipes {
            categories: strict_list(o, "categories", category)?,
            recipes: strict_list(o, "recipes", recipe)?,
            shown: match o.get("shown") {
                None | Some(Value::Null) => true,
                Some(Value::Bool(b)) => *b,
                Some(_) => return None,
            },
            home: o.get("home").and_then(home_words),
            labels: o.get("labels").and_then(labels),
        })
    }

    /// `byCategory`: each category in tafs cloud's order with its recipes; one with none isn't
    /// shown.
    pub fn by_category(&self) -> Vec<(&Category, Vec<&Recipe>)> {
        self.categories
            .iter()
            .map(|c| {
                (
                    c,
                    self.recipes
                        .iter()
                        .filter(|r| r.category == c.id)
                        .collect::<Vec<_>>(),
                )
            })
            .filter(|(_, r)| !r.is_empty())
            .collect()
    }

    /// `groups(picked:)`: narrowed to `picked` when it's listed; else all of them.
    pub fn groups(&self, picked: Option<&str>) -> Vec<(&Category, Vec<&Recipe>)> {
        let all = self.by_category();
        match picked {
            Some(p) if all.iter().any(|(c, _)| c.id == p) => {
                all.into_iter().filter(|(c, _)| c.id == p).collect()
            }
            _ => all,
        }
    }

    /// `homeListsRecipes`: tafs cloud says Home lists them, the sheet's words came, and there are
    /// some to list.
    pub fn home_lists_recipes(&self) -> bool {
        self.home.as_ref().is_some_and(|h| h.shown)
            && self.labels.is_some()
            && !self.by_category().is_empty()
    }

    /// `homeWords`: tafs cloud's line for Home with no taf; none when it sent none.
    pub fn home_words(&self) -> Option<&str> {
        self.home
            .as_ref()
            .map(|h| h.words.trim())
            .filter(|w| !w.is_empty())
    }

    /// `homeOwnWords`.
    pub fn home_own_words(&self) -> Option<&OwnWords> {
        self.home.as_ref().and_then(|h| h.own_words.as_ref())
    }

    /// `zones(of:)`: does on its own, asks first, never does, titled by `labels`; a zone the Card
    /// leaves empty isn't one.
    pub fn zones(&self, r: &Recipe) -> Vec<Zone> {
        let l = self.labels.clone().unwrap_or_default();
        [
            ("allows", l.allows, r.card.allows.clone()),
            ("holds", l.holds, r.card.holds.clone()),
            ("never", l.never, r.card.never.clone()),
        ]
        .into_iter()
        .filter(|(_, _, lines)| !lines.is_empty())
        .map(|(id, title, lines)| Zone { id, title, lines })
        .collect()
    }

    /// `handWords`: the sheet's button that hands the recipe to Coach; none when none came.
    pub fn hand_words(&self) -> Option<&str> {
        self.labels
            .as_ref()
            .map(|l| l.hand.trim())
            .filter(|w| !w.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_logo_loads_only_from_tafs_cloud_by_its_key() {
        let key = "0123456789abcdef0123456789abcdef";
        assert_eq!(
            logo_url(Some(&format!("/v1/logos/{key}"))),
            Some(format!("{}/v1/logos/{key}", crate::home::TAFS_CLOUD))
        );
        for bad in [
            None,
            Some("/v1/logos/stripe"),
            Some("/v1/logos/0123456789ABCDEF0123456789ABCDEF"),
            Some("/v1/logos/0123456789abcdef0123456789abcde"),
            Some("/v1/logos/0123456789abcdef0123456789abcdef0"),
            Some("/v1/logos/0123456789abcdef0123456789abcdeg"),
            Some("https://evil.example/v1/logos/0123456789abcdef0123456789abcdef"),
            Some("/v1/logos/../0123456789abcdef0123456789abcd"),
            Some("v1/logos/0123456789abcdef0123456789abcdef"),
        ] {
            assert_eq!(logo_url(bad), None, "{bad:?}");
        }
    }
}
