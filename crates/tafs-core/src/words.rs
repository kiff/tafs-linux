//! The words, read from the shared repository at the pin (`shared/app/strings/<lang>/*.json`).
//!
//! The same files the iPad's `Words.swift` is generated from (`app/strings/generate.py`):
//! a key's line, its `{name}` values, its `{name|possessive}` and `{name|this}` (`{name|This}`
//! to open a sentence) by the language's rule, and a count's plural forms on `{count}`. A key that isn't there, or a value a line asks for and
//! isn't given, is an error: there is no English fallback and no `{hole}` on screen.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// The languages tafs ships (`languages.json`'s `shipped`), English first.
pub const SHIPPED: [&str; 8] = ["en", "de", "fr", "es", "it", "pt", "nl", "pl"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WordsError {
    /// The folder of a language couldn't be read.
    Unreadable(String),
    /// No such key at the pin.
    MissingKey(String),
    /// A line asks for a value it wasn't given.
    MissingValue { key: String, value: String },
    /// A count has no form for this number in this language.
    MissingForm { key: String, form: &'static str },
    /// A count read as a line, or a line as a count.
    WrongShape(String),
}

impl fmt::Display for WordsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WordsError::Unreadable(why) => write!(f, "the words couldn't be read: {why}"),
            WordsError::MissingKey(k) => write!(f, "no word for {k} at the pin"),
            WordsError::MissingValue { key, value } => write!(f, "{key} needs {{{value}}}"),
            WordsError::MissingForm { key, form } => write!(f, "{key} has no {form} form"),
            WordsError::WrongShape(k) => write!(f, "{k} isn't that kind of line"),
        }
    }
}

impl std::error::Error for WordsError {}

/// One language's lines.
#[derive(Debug, Clone)]
pub struct WordBook {
    language: String,
    lines: BTreeMap<String, Value>,
}

impl WordBook {
    /// The words' root at the pin: `shared/app/strings`, from `TAFS_STRINGS` or beside the
    /// running program, or the repository's `shared/` when run from a checkout.
    pub fn default_root() -> PathBuf {
        if let Ok(p) = std::env::var("TAFS_STRINGS") {
            return PathBuf::from(p);
        }
        let mut candidates = Vec::new();
        if let Ok(exe) = std::env::current_exe() {
            if let Some(dir) = exe.parent() {
                candidates.push(dir.join("strings"));
                candidates.push(dir.join("../share/tafs/strings"));
            }
        }
        candidates.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../shared/app/strings"));
        candidates.push(PathBuf::from("shared/app/strings"));
        candidates
            .into_iter()
            .find(|p| p.join("en").is_dir())
            .unwrap_or_else(|| PathBuf::from("shared/app/strings"))
    }

    /// Reads `<root>/<language>/*.json`.
    pub fn load(root: &Path, language: &str) -> Result<WordBook, WordsError> {
        let folder = root.join(language);
        let entries = std::fs::read_dir(&folder)
            .map_err(|e| WordsError::Unreadable(format!("{}: {e}", folder.display())))?;
        let mut lines = BTreeMap::new();
        let mut names: Vec<PathBuf> = entries
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .collect();
        names.sort();
        for path in names {
            let text = std::fs::read_to_string(&path)
                .map_err(|e| WordsError::Unreadable(format!("{}: {e}", path.display())))?;
            let map: BTreeMap<String, Value> = serde_json::from_str(&text)
                .map_err(|e| WordsError::Unreadable(format!("{}: {e}", path.display())))?;
            lines.extend(map);
        }
        if lines.is_empty() {
            return Err(WordsError::Unreadable(format!(
                "{} has no words",
                folder.display()
            )));
        }
        Ok(WordBook {
            language: language.to_string(),
            lines,
        })
    }

    pub fn language(&self) -> &str {
        &self.language
    }

    /// Every key in this language.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.lines.keys().map(String::as_str)
    }

    /// The raw line, as the file has it: a string, or a count's forms.
    pub fn raw(&self, key: &str) -> Option<&Value> {
        self.lines.get(key)
    }

    /// A line, its values filled.
    pub fn get(&self, key: &str, values: &[(&str, &str)]) -> Result<String, WordsError> {
        match self.lines.get(key) {
            Some(Value::String(s)) => fill(key, s, values, &self.language),
            Some(_) => Err(WordsError::WrongShape(key.to_string())),
            None => Err(WordsError::MissingKey(key.to_string())),
        }
    }

    /// A line with no values.
    pub fn line(&self, key: &str) -> Result<String, WordsError> {
        self.get(key, &[])
    }

    /// A count: the form the language gives `n`, with `{count}` and its values filled.
    pub fn count(&self, key: &str, n: i64, values: &[(&str, &str)]) -> Result<String, WordsError> {
        let forms = match self.lines.get(key) {
            Some(Value::Object(o)) => o,
            Some(_) => return Err(WordsError::WrongShape(key.to_string())),
            None => return Err(WordsError::MissingKey(key.to_string())),
        };
        let form = plural_form(&self.language, n);
        let line = forms
            .get(form)
            .and_then(Value::as_str)
            .ok_or(WordsError::MissingForm {
                key: key.to_string(),
                form,
            })?;
        let number = n.to_string();
        let mut all: Vec<(&str, &str)> = vec![("count", number.as_str())];
        all.extend_from_slice(values);
        fill(key, line, &all, &self.language)
    }
}

/// The CLDR plural form of a whole number in a shipped language: the forms the files have.
pub fn plural_form(language: &str, n: i64) -> &'static str {
    let n = n.unsigned_abs();
    // The forms the files carry (test_strings.py's NEEDS): French and Portuguese put 0 and 1 in
    // "one"; Polish has one, few and many for whole numbers; the rest one and other.
    match language {
        "fr" | "pt" => {
            if n <= 1 {
                "one"
            } else {
                "other"
            }
        }
        "pl" => {
            let (m10, m100) = (n % 10, n % 100);
            if n == 1 {
                "one"
            } else if (2..=4).contains(&m10) && !(12..=14).contains(&m100) {
                "few"
            } else {
                "many"
            }
        }
        _ => {
            if n == 1 {
                "one"
            } else {
                "other"
            }
        }
    }
}

/// A name's possessive by the language's rule (`POSSESSIVE` in `generate.py`); the name as it
/// is in a language with none.
pub fn possessive(language: &str, name: &str) -> String {
    let rule: Option<(&str, &str, &str)> = match language {
        "en" => Some(("s", "'", "'s")),
        "de" => Some(("sßxz", "'", "s")),
        _ => None,
    };
    let Some((ends, after, other)) = rule else {
        return name.to_string();
    };
    match name.chars().last() {
        None => String::new(),
        Some(last) => {
            let lower: String = last.to_lowercase().collect();
            let suffix = if ends.contains(lower.as_str()) {
                after
            } else {
                other
            };
            format!("{name}{suffix}")
        }
    }
}

/// "This name" by the language's rule (`THIS` in `generate.py`): "ce Mac", "cet appareil";
/// `capital` opens a sentence ("Cet appareil"). The name as it is in a language with none: its
/// lines write their own word. The rule is right for the device kinds the apps name: it takes
/// every h as mute and y as a vowel.
pub fn demonstrative(language: &str, name: &str, capital: bool) -> String {
    let rule: Option<(&str, &str, &str)> = match language {
        "fr" => Some(("aeiouyhàâäéèêëîïôöùûüœæ", "cet ", "ce ")),
        _ => None,
    };
    let Some((starts, before, other)) = rule else {
        return name.to_string();
    };
    let Some(first) = name.chars().next() else {
        return String::new();
    };
    let lower: String = first.to_lowercase().collect();
    let word = if starts.contains(lower.as_str()) {
        before
    } else {
        other
    };
    if capital {
        let mut chars = word.chars();
        let head: String = chars
            .next()
            .map(|c| c.to_uppercase().collect())
            .unwrap_or_default();
        format!("{head}{}{name}", chars.as_str())
    } else {
        format!("{word}{name}")
    }
}

/// How a placeholder asks for its value: as it is, its possessive, or "this" before it.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Form {
    Plain,
    Possessive,
    This,
    ThisCapital,
}

/// Fills `{name}`, `{name|possessive}`, `{name|this}` and `{name|This}`. A form it doesn't know
/// is a missing value, so a line never reaches the screen with a `{hole}` in it.
fn fill(
    key: &str,
    line: &str,
    values: &[(&str, &str)],
    language: &str,
) -> Result<String, WordsError> {
    let mut out = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            rest = "";
            break;
        };
        let hole = &after[..close];
        let (name, form) = match hole.split_once('|') {
            Some((n, "possessive")) => (n, Form::Possessive),
            Some((n, "this")) => (n, Form::This),
            Some((n, "This")) => (n, Form::ThisCapital),
            _ => (hole, Form::Plain),
        };
        let value = values
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| *v)
            .ok_or_else(|| WordsError::MissingValue {
                key: key.to_string(),
                value: name.to_string(),
            })?;
        match form {
            Form::Plain => out.push_str(value),
            Form::Possessive => out.push_str(&possessive(language, value)),
            Form::This => out.push_str(&demonstrative(language, value, false)),
            Form::ThisCapital => out.push_str(&demonstrative(language, value, true)),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// The placeholders a line names, `{name}`, `{name|possessive}` or `{name|this}`, in order.
pub fn placeholders(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else { break };
        let name = after[..close].split('|').next().unwrap_or("").to_string();
        if !out.contains(&name) {
            out.push(name);
        }
        rest = &after[close + 1..];
    }
    out
}

/// `Words.list`: names joined as the language lists them (`ListFormatter`, CLDR's "and" list):
/// "Refunds", "Refunds and Chargebacks", "A, B, and C" in English; "A, B et C" in French.
pub fn list(language: &str, names: &[String]) -> String {
    let and = match language {
        "fr" => "et",
        "de" => "und",
        "es" => "y",
        "it" | "pt" => "e",
        "nl" => "en",
        "pl" => "i",
        _ => "and",
    };
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [a, b] => format!("{a} {and} {b}"),
        [rest @ .., last] => {
            // English alone puts a comma before its "and".
            let comma = if language == "en" { "," } else { "" };
            format!("{}{comma} {and} {last}", rest.join(", "))
        }
    }
}

/// Text in capitals by the language's own rules, as the iPad's `Eyebrow` (`uppercased()`).
pub fn upper(text: &str) -> String {
    text.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn possessive_follows_the_language() {
        assert_eq!(possessive("en", "Refunds"), "Refunds'");
        assert_eq!(possessive("en", "Rae"), "Rae's");
        assert_eq!(possessive("de", "Klaus"), "Klaus'");
        assert_eq!(possessive("de", "Rae"), "Raes");
        assert_eq!(possessive("fr", "Rae"), "Rae");
    }

    #[test]
    fn this_follows_the_language_both_ways() {
        let fr: Vec<String> = [
            "Mac",
            "PC",
            "PC Windows",
            "appareil",
            "iPad",
            "écran",
            "hôte",
        ]
        .iter()
        .map(|n| demonstrative("fr", n, false))
        .collect();
        assert_eq!(
            fr,
            [
                "ce Mac",
                "ce PC",
                "ce PC Windows",
                "cet appareil",
                "cet iPad",
                "cet écran",
                "cet hôte"
            ]
        );
        assert_eq!(demonstrative("fr", "Mac", true), "Ce Mac");
        assert_eq!(demonstrative("fr", "appareil", true), "Cet appareil");
        assert_eq!(demonstrative("en", "computer", false), "computer");
        assert_eq!(demonstrative("fr", "", true), "");
    }

    #[test]
    fn fill_gives_this_and_refuses_a_form_it_doesnt_know() {
        let pc = [("device", "PC")];
        assert_eq!(
            fill("k", "Alertes sur {device|this}", &pc, "fr").unwrap(),
            "Alertes sur ce PC"
        );
        assert_eq!(
            fill("k", "{device|This} est ajouté.", &pc, "fr").unwrap(),
            "Ce PC est ajouté."
        );
        assert_eq!(
            fill(
                "k",
                "{device|This} est prêt.",
                &[("device", "appareil")],
                "fr"
            )
            .unwrap(),
            "Cet appareil est prêt."
        );
        assert!(fill("k", "{device|that}", &pc, "fr").is_err());
        assert_eq!(
            placeholders("sur {device|this} et {device|This}"),
            ["device"]
        );
    }

    #[test]
    fn plural_forms() {
        assert_eq!(plural_form("en", 1), "one");
        assert_eq!(plural_form("en", 0), "other");
        assert_eq!(plural_form("fr", 0), "one");
        assert_eq!(plural_form("fr", 2), "other");
        assert_eq!(plural_form("pl", 1), "one");
        assert_eq!(plural_form("pl", 3), "few");
        assert_eq!(plural_form("pl", 13), "many");
        assert_eq!(plural_form("pl", 22), "few");
        assert_eq!(plural_form("pl", 5), "many");
    }

    #[test]
    fn fill_refuses_a_missing_value() {
        let err = fill("k", "Hi {name}", &[], "en").unwrap_err();
        assert_eq!(
            err,
            WordsError::MissingValue {
                key: "k".into(),
                value: "name".into()
            }
        );
        assert_eq!(
            fill("k", "{taf|possessive} Card", &[("taf", "Refunds")], "en").unwrap(),
            "Refunds' Card"
        );
    }
}
