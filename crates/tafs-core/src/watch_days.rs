//! W11, "From your watch" (`WatchDaysViews.swift`, `Model/WatchDays.swift`, `TafsCloud+WatchDays.swift`): in Coach's
//! Previous conversations, the days the person asked from their watch, newest first, each with how many requests; a day
//! opens its turns, oldest first, read only. Only the person's own. A tafs cloud whose switch is off answers 404, and the
//! section isn't drawn; a day or a turn this app can't read is left out, never the list.

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;
use serde_json::Value;

use crate::words::{WordBook, WordsError};

/// `GET /v1/watch/days`.
pub const DAYS_PATH: &str = "/v1/watch/days";

/// `GET /v1/watch/days/{date}`, the date escaped as one segment.
pub fn day_path(date: &str) -> String {
    format!("{DAYS_PATH}/{}", crate::access::escaped(date))
}

/// A day the person asked from their watch (`WatchDay`): `YYYY-MM-DD` in the account's zone, and how many.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Day {
    pub date: String,
    pub count: i64,
}

/// The days, and which is today in the account's zone, as tafs cloud says (`WatchDayList`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Days {
    pub days: Vec<Day>,
    pub today: Option<String>,
}

/// The days from tafs cloud's answer: a day this app can't read is left out (`LenientItem`); an answer it can't read is
/// none.
pub fn read_days(body: &[u8]) -> Days {
    let Ok(v) = serde_json::from_slice::<Value>(body) else {
        return Days::default();
    };
    Days {
        days: v["days"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|d| serde_json::from_value::<Day>(d.clone()).ok())
                    .collect()
            })
            .unwrap_or_default(),
        today: v["today"].as_str().map(str::to_string),
    }
}

/// The taf that took a request.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Taf {
    pub id: String,
    pub name: String,
}

/// One request said from a watch that day, and where it went (`WatchTurn`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Turn {
    pub id: String,
    /// What the person said, as the watch sent it.
    pub said: String,
    pub said_at: Option<DateTime<Utc>>,
    pub taf: Option<Taf>,
    pub state: String,
    /// What came of it, checked by tafs cloud; none while it's at work.
    pub line: Option<String>,
    /// The state in tafs cloud's words.
    pub words: String,
    /// The request this one replies to, shown under it.
    pub reply_to: Option<String>,
}

impl Turn {
    /// What came of it, as the watch shows it: tafs cloud's line when there is one, else its words for the state.
    pub fn outcome(&self) -> &str {
        self.line.as_deref().unwrap_or(&self.words)
    }
}

/// One day's turns, oldest first as tafs cloud sends them. One this app can't read is left out (it needs an `id` and what
/// was said), and so is a message to a taf (`kind: "taf"`): it's in that taf's own chat, not Coach's.
pub fn read_turns(body: &[u8]) -> Vec<Turn> {
    let Ok(v) = serde_json::from_slice::<Value>(body) else {
        return vec![];
    };
    let Some(list) = v["turns"].as_array() else {
        return vec![];
    };
    list.iter()
        .filter_map(|t| {
            if t["kind"].as_str() == Some("taf") {
                return None;
            }
            let text = |k: &str| t[k].as_str().map(str::to_string);
            Some(Turn {
                id: text("id")?,
                said: text("said")?,
                said_at: t["said_at"]
                    .as_str()
                    .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                    .map(|d| d.with_timezone(&Utc)),
                taf: serde_json::from_value(t["taf"].clone()).ok(),
                state: text("state").unwrap_or_default(),
                line: text("line").filter(|l| !l.is_empty()),
                words: text("words").unwrap_or_default(),
                reply_to: text("reply_to"),
            })
        })
        .collect()
}

/// A day's title (`WatchDaysModel.title`): Today for the account's today as tafs cloud gives it (else this computer's in
/// the account's zone), else the weekday, the day and the month in the language. A date that can't be read is as sent.
pub fn title(
    w: &WordBook,
    date: &str,
    today: Option<&str>,
    now: DateTime<Utc>,
) -> Result<String, WordsError> {
    let Ok(day) = NaiveDate::parse_from_str(date, "%Y-%m-%d") else {
        return Ok(date.to_string());
    };
    let local_today = crate::account_time::day(&now)
        .format("%Y-%m-%d")
        .to_string();
    if date == today.unwrap_or(&local_today) {
        return w.line("calendar.today");
    }
    Ok(crate::locale::weekday_day_month_long(w.language(), day))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn the_days_and_their_turns_read_leniently() {
        let d = read_days(
            br#"{"days":[{"date":"2026-11-22","count":3},{"date":"bad"},{"date":"2026-11-20","count":1}],"today":"2026-11-22"}"#,
        );
        assert_eq!(d.days.len(), 2);
        assert_eq!(d.today.as_deref(), Some("2026-11-22"));
        assert_eq!(read_days(b"not json"), Days::default());
        let t = read_turns(
            r#"{"turns":[{"id":"t1","said":"Refund order 1042","said_at":"2026-11-22T09:12:00Z","taf":{"id":"x","name":"Refunds"},"state":"done","line":"Refunded €150","words":"Done"},{"id":"t2","said":"How are sales?","state":"working","line":"","words":"Working on it","reply_to":"t1"},{"id":"t3","said":"to a taf","kind":"taf","state":"done","words":"Done"},{"said":"no id"}]}"#
                .as_bytes(),
        );
        assert_eq!(t.len(), 2);
        assert_eq!(t[0].outcome(), "Refunded €150");
        assert_eq!(t[0].taf.as_ref().unwrap().name, "Refunds");
        // A blank line is none: the state's words stand.
        assert_eq!(t[1].outcome(), "Working on it");
        assert_eq!(t[1].reply_to.as_deref(), Some("t1"));
        assert_eq!(day_path("2026-11-22"), "/v1/watch/days/2026-11-22");
    }

    #[test]
    fn today_is_tafs_clouds_else_the_date_in_the_language() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let now = Utc.with_ymd_and_hms(2026, 11, 22, 12, 0, 0).unwrap();
        assert_eq!(
            title(&w, "2026-11-22", Some("2026-11-22"), now).unwrap(),
            "Today"
        );
        assert_eq!(
            title(&w, "2026-11-20", Some("2026-11-22"), now).unwrap(),
            "Friday, November 20"
        );
        assert_eq!(title(&w, "odd", None, now).unwrap(), "odd");
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            assert!(
                !title(&w, "2026-11-20", None, now).unwrap().is_empty(),
                "{lang}"
            );
        }
    }
}
