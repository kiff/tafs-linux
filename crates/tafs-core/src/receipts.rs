//! The Record's receipts export (`RecordSearchViews.swift` › `ExportReceiptsRow`,
//! `TafsCloud+Record.swift` › `exportReceipts`, `AppModel.exportReceipts`): the last 7, 30 or 90
//! days of receipts, as tafs cloud's JSON and its CSV, two files. Only for whoever sees every
//! taf's Record (`seesEveryTaf`); tafs cloud still decides.

use std::collections::BTreeMap;

use chrono::{Duration, NaiveDate};
use serde_json::Value;

use crate::access::Access;
use crate::home::HomeTaf;
use crate::words::{WordBook, WordsError};

/// The day ranges offered.
pub const DAYS: [i64; 3] = [7, 30, 90];

/// `seesEveryTaf`: the person's tafs on tafs cloud, at least one, each one they oversee or
/// manage. A taf whose access wasn't read counts as not seen.
pub fn sees_every_taf(tafs: &[HomeTaf], access: &BTreeMap<String, Access>) -> bool {
    let on_cloud: Vec<&HomeTaf> = tafs.iter().filter(|t| t.id.is_some()).collect();
    !on_cloud.is_empty()
        && on_cloud.iter().all(|t| {
            access
                .get(&t.kiff_agent)
                .is_some_and(|a| a.oversee || a.manage)
        })
}

/// The last `count` days, today among them, as the account's calendar days (YYYY-MM-DD).
pub fn range(count: i64, today: NaiveDate) -> (String, String) {
    let from = today - Duration::days(count - 1);
    (day(from), day(today))
}

fn day(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// `exportReceipts`'s path.
pub fn path(from: &str, to: &str) -> String {
    format!("/v1/record/receipts?from={from}&to={to}")
}

/// What tafs cloud answered (`ReceiptsExport`): its days, the answer as it came, its CSV, and how
/// many calls have no receipt yet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Export {
    pub from: String,
    pub to: String,
    pub json: Vec<u8>,
    pub csv: String,
    pub missing: i64,
}

impl Export {
    /// Read as the iPad reads it: anything it can't read is the asked days, no CSV and none
    /// missing; the JSON file is the answer as it came. A day that isn't one (YYYY-MM-DD) is
    /// the asked day: it names the files, so it never takes a path with it.
    pub fn read(body: &[u8], from: &str, to: &str) -> Export {
        let v: Value = serde_json::from_slice(body).unwrap_or(Value::Null);
        let o = v.as_object();
        let text = |k: &str| o.and_then(|o| o.get(k)).and_then(Value::as_str);
        let a_day = |k: &str, asked: &str| {
            text(k)
                .filter(|d| NaiveDate::parse_from_str(d, "%Y-%m-%d").is_ok() && d.len() == 10)
                .unwrap_or(asked)
                .to_string()
        };
        Export {
            from: a_day("from", from),
            to: a_day("to", to),
            json: body.to_vec(),
            csv: text("csv").unwrap_or_default().to_string(),
            missing: o
                .and_then(|o| o.get("missing"))
                .and_then(|m| m.as_i64().or_else(|| m.as_f64().map(|f| f as i64)))
                .unwrap_or(0),
        }
    }

    /// The two files' names: `receipts-{from}-to-{to}.json` and `.csv`.
    pub fn file_names(&self) -> (String, String) {
        let stem = format!("receipts-{}-to-{}", self.from, self.to);
        (format!("{stem}.json"), format!("{stem}.csv"))
    }
}

/// The row's words (`ExportReceiptsRow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowLines {
    pub export: String,
    pub preparing: String,
    /// "Last 7 days", "Last 30 days", "Last 90 days", with their counts.
    pub days: Vec<(i64, String)>,
}

pub fn row(w: &WordBook) -> Result<RowLines, WordsError> {
    Ok(RowLines {
        export: w.line("record.export_receipts")?,
        preparing: w.line("record.export_preparing")?,
        days: DAYS
            .iter()
            .map(|&n| Ok((n, w.count("record.export_last_days", n, &[])?)))
            .collect::<Result<_, WordsError>>()?,
    })
}

/// "{count} calls have no receipt yet.", once exported with some missing.
pub fn missing(w: &WordBook, n: i64) -> Result<Option<String>, WordsError> {
    if n > 0 {
        Ok(Some(w.count("record.export_missing", n, &[])?))
    } else {
        Ok(None)
    }
}

/// "They couldn't be exported. {why}".
pub fn not_exported(w: &WordBook, why: &str) -> Result<String, WordsError> {
    w.get("record.export_not_exported", &[("why", why)])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(lang: &str) -> WordBook {
        WordBook::load(&WordBook::default_root(), lang).unwrap()
    }

    fn day(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn the_last_days_count_today() {
        assert_eq!(
            range(7, day("2026-11-16")),
            ("2026-11-10".into(), "2026-11-16".into())
        );
        assert_eq!(
            range(30, day("2026-03-01")),
            ("2026-01-31".into(), "2026-03-01".into())
        );
        assert_eq!(range(90, day("2026-11-16")).0, "2026-08-19");
        assert_eq!(
            path("2026-11-10", "2026-11-16"),
            "/v1/record/receipts?from=2026-11-10&to=2026-11-16"
        );
    }

    #[test]
    fn an_export_reads_as_the_ipads() {
        let body =
            br#"{"from":"2026-11-10","to":"2026-11-16","csv":"a,b\n","missing":2,"receipts":[]}"#;
        let e = Export::read(body, "x", "y");
        assert_eq!(e.from, "2026-11-10");
        assert_eq!(e.to, "2026-11-16");
        assert_eq!(e.csv, "a,b\n");
        assert_eq!(e.missing, 2);
        assert_eq!(e.json, body.to_vec());
        assert_eq!(
            e.file_names(),
            (
                "receipts-2026-11-10-to-2026-11-16.json".into(),
                "receipts-2026-11-10-to-2026-11-16.csv".into()
            )
        );
        // Nothing readable: the asked days, no CSV, none missing.
        let e = Export::read(b"not json", "2026-11-10", "2026-11-16");
        assert_eq!((e.csv.as_str(), e.missing), ("", 0));
        assert_eq!(e.from, "2026-11-10");
        // A day that isn't one never names a file.
        let e = Export::read(
            br#"{"from":"../../x","to":"2026-11-16/.."}"#,
            "2026-11-10",
            "2026-11-16",
        );
        assert_eq!(e.file_names().0, "receipts-2026-11-10-to-2026-11-16.json");
    }

    /// The iPad's own: today is the account's day, and the last 7 days are it and the six before.
    #[test]
    fn the_range_counts_the_accounts_today() {
        use chrono::TimeZone;
        // 23:30 UTC on 10 Nov is 13:30 on 11 Nov in Kiritimati (UTC+14).
        let now = chrono::Utc
            .with_ymd_and_hms(2026, 11, 10, 23, 30, 0)
            .unwrap();
        let today = now
            .with_timezone(&chrono_tz::Pacific::Kiritimati)
            .date_naive();
        assert_eq!(range(7, today), ("2026-11-05".into(), "2026-11-11".into()));
        assert_eq!(range(90, today).0, "2026-08-14");
        assert_eq!(DAYS, [7, 30, 90], "inside tafs cloud's 92");
    }

    /// The iPad's test body: spaced, with 150.50, so a re-encoded copy would differ.
    #[test]
    fn the_json_file_is_the_answer_byte_for_byte() {
        let body = br#"{"from": "2026-11-05", "to": "2026-11-11", "time_zone": "Europe/Paris", "receipts": [{"trace_id": "t1", "sig": "0xab", "amount": 150.50}], "csv": "day,taf\n2026-11-05,Rae\n", "missing": 2}"#;
        let e = Export::read(body, "2026-11-05", "2026-11-11");
        assert_eq!(e.json, body.to_vec());
        assert_eq!(e.csv, "day,taf\n2026-11-05,Rae\n");
        assert_eq!(e.missing, 2);
        assert_eq!(
            e.file_names(),
            (
                "receipts-2026-11-05-to-2026-11-11.json".into(),
                "receipts-2026-11-05-to-2026-11-11.csv".into()
            )
        );
    }

    fn taf(id: Option<&str>, agent: &str) -> HomeTaf {
        let contract: Value =
            serde_json::from_slice(include_bytes!("../../../shared/contracts/home.json")).unwrap();
        let home = crate::home::Home::read(contract["answer"].to_string().as_bytes()).unwrap();
        HomeTaf {
            id: id.map(str::to_string),
            kiff_agent: agent.to_string(),
            ..home.tafs[0].clone()
        }
    }

    fn access(oversee: bool, manage: bool) -> Access {
        Access {
            use_: true,
            oversee,
            manage,
            groups: None,
        }
    }

    #[test]
    fn only_whoever_sees_every_taf_exports() {
        let tafs = [
            taf(Some("t1"), "a1"),
            taf(Some("t2"), "a2"),
            taf(None, "a3"),
        ];
        let mut a = BTreeMap::new();
        a.insert("a1".to_string(), access(true, false));
        assert!(!sees_every_taf(&tafs, &a), "a2's access wasn't read");
        a.insert("a2".to_string(), access(false, true));
        assert!(sees_every_taf(&tafs, &a), "a3 isn't on tafs cloud");
        a.insert("a2".to_string(), access(false, false));
        assert!(!sees_every_taf(&tafs, &a));
        assert!(!sees_every_taf(&[], &a));
        assert!(!sees_every_taf(&[taf(None, "a3")], &a));
    }

    #[test]
    fn the_words_read_in_every_language() {
        let en = words("en");
        let r = row(&en).unwrap();
        assert_eq!(r.export, "Export receipts");
        assert_eq!(r.days[0], (7, "Last 7 days".into()));
        assert_eq!(missing(&en, 0).unwrap(), None);
        assert_eq!(
            missing(&en, 1).unwrap().unwrap(),
            "1 call has no receipt yet."
        );
        assert_eq!(
            not_exported(&en, "KIFF said no.").unwrap(),
            "They couldn't be exported. KIFF said no."
        );
        for lang in crate::words::SHIPPED {
            let w = words(lang);
            row(&w).unwrap();
            for n in [1, 2, 5, 22] {
                missing(&w, n).unwrap();
            }
            not_exported(&w, "x").unwrap();
        }
    }
}
