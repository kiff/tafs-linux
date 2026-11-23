//! Files (kiff/tafs `docs/files.md`, `Model/Files.swift`, `KIFF/TafsCloud+Files.swift`): what the account's
//! tafs delivered in their runs, kept 30 days from the run and seen by everyone on the account. tafs cloud lists them
//! (`GET /v1/files`), gives a file's start as plain text (`GET /v1/files/{id}`), a URL to its bytes for a minute
//! (`…/download`), and deletes it for everyone (`DELETE`). Its own switch alone decides: a 404 draws nothing.
//!
//! The app only draws tafs cloud's facts: it groups and orders them, counts the days to when tafs cloud says a file
//! goes, and cleans what it draws (a name is never a path; no control or direction character).

use chrono::{DateTime, NaiveDate, Utc};
use serde::Deserialize;

use crate::words::{WordBook, WordsError};

/// How long a file is kept from its run (the owner's pick): tafs cloud's rule, shown here, never applied here.
pub const KEPT_DAYS: i64 = 30;

/// At most this many pages are read at once: a month of an account's files, well past what a screen shows.
pub const PAGES: usize = 10;

/// A file a taf delivered in a run, as the screens draw it (`DeliveredFile`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveredFile {
    pub id: String,
    /// The name alone, cleaned (`FileText.name`).
    pub name: String,
    /// Its type, as its extension says it (else its media type's), upper case: "CSV", "PDF".
    pub kind: String,
    pub bytes: i64,
    /// The taf that made it: tafs cloud's id, and its name as tafs has it (empty once the taf is gone).
    pub taf: String,
    pub taf_name: String,
    /// The run it came from, and when it was delivered.
    pub run: String,
    pub run_at: DateTime<Utc>,
    pub chat: Option<String>,
    /// When it goes, as tafs cloud says.
    pub goes_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
struct Read {
    id: String,
    taf: String,
    run: String,
    #[serde(default)]
    chat: Option<String>,
    name: String,
    media_type: String,
    size: i64,
    created: String,
    expires: String,
    #[serde(default)]
    taf_name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct PageRead {
    files: Vec<Read>,
    #[serde(default)]
    next: Option<String>,
}

fn time(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|t| t.with_timezone(&Utc))
}

impl DeliveredFile {
    fn from_read(r: Read) -> Option<DeliveredFile> {
        let clean = name(&r.name, &r.id);
        let ext = std::path::Path::new(&clean)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_string();
        let kind = if ext.is_empty() {
            r.media_type.rsplit('/').next().unwrap_or("").to_string()
        } else {
            ext
        };
        Some(DeliveredFile {
            id: r.id,
            name: clean,
            kind: kind.to_uppercase(),
            bytes: r.size,
            taf: r.taf,
            taf_name: r.taf_name.unwrap_or_default(),
            run: r.run,
            run_at: time(&r.created)?,
            chat: r.chat,
            goes_at: time(&r.expires)?,
        })
    }
}

/// A page of the list, and the cursor to the next: none when it can't be read, as the iPad's decoder fails it.
pub fn read_page(body: &[u8]) -> Option<(Vec<DeliveredFile>, Option<String>)> {
    let p: PageRead = serde_json::from_slice(body).ok()?;
    let files = p
        .files
        .into_iter()
        .map(DeliveredFile::from_read)
        .collect::<Option<Vec<_>>>()?;
    Some((files, p.next))
}

/// `GET /v1/files`, the page after `after`.
pub fn list_path(after: Option<&str>) -> String {
    match after {
        Some(a) => format!("/v1/files?after={}", crate::access::escaped(a)),
        None => "/v1/files".into(),
    }
}

/// `GET /v1/files/{id}` (its start), and `DELETE` (for everyone).
pub fn file_path(id: &str) -> String {
    format!("/v1/files/{}", crate::access::escaped(id))
}

/// `GET /v1/files/{id}/download`: a URL to its bytes for a minute.
pub fn download_path(id: &str) -> String {
    format!("{}/download", file_path(id))
}

/// A file's start as tafs cloud cleaned it, and whether there's more (`FilePreviewText`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreviewText {
    pub text: String,
    pub truncated: bool,
}

/// The start from `GET /v1/files/{id}`, cleaned again here (`FileText.preview`); none when tafs cloud gave none.
pub fn read_preview(body: &[u8]) -> Option<Option<PreviewText>> {
    #[derive(Deserialize)]
    struct R {
        #[serde(default)]
        preview: Option<String>,
        #[serde(default)]
        preview_truncated: Option<bool>,
    }
    let r: R = serde_json::from_slice(body).ok()?;
    Some(r.preview.map(|p| PreviewText {
        text: preview(&p),
        truncated: r.preview_truncated.unwrap_or(false),
    }))
}

/// The URL to a file's bytes: only `https`, with no user in it. It's read with no token: the session goes only to
/// KIFF's and tafs cloud's own hosts.
pub fn read_download(body: &[u8]) -> Option<url::Url> {
    #[derive(Deserialize)]
    struct R {
        url: String,
    }
    let r: R = serde_json::from_slice(body).ok()?;
    let u = url::Url::parse(&r.url).ok()?;
    (u.scheme() == "https" && u.username().is_empty() && u.password().is_none()).then_some(u)
}

/// The types tafs cloud gives a text start for: their bytes are never drawn in its place.
pub const TEXT_TYPES: [&str; 4] = ["CSV", "MD", "TXT", "JSON"];

pub fn has_text_start(f: &DeliveredFile) -> bool {
    TEXT_TYPES.contains(&f.kind.as_str())
}

fn is_direction(c: char) -> bool {
    matches!(c as u32, 0x202A..=0x202E | 0x2066..=0x2069 | 0x200E | 0x200F | 0x061C)
}

/// The name alone (`FileText.name`): what follows the last `/` or `\`, control and direction characters taken out;
/// `fallback` if nothing's left, or only "." or "..".
pub fn name(raw: &str, fallback: &str) -> String {
    let last = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let kept: String = last
        .chars()
        .filter(|c| !is_direction(*c) && !c.is_control())
        .collect();
    let clean = kept.trim();
    if clean.is_empty() || clean == "." || clean == ".." {
        fallback.to_string()
    } else {
        clean.to_string()
    }
}

/// A preview's text (`FileText.preview`): lines and tabs kept, every other control or direction character taken out,
/// Windows' line ends made one.
pub fn preview(raw: &str) -> String {
    raw.replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|c| *c == '\n' || *c == '\t' || (!is_direction(*c) && !c.is_control()))
        .collect()
}

/// A group of the list (`FilesGrouping.Group`): its day, taf or run, and its files in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub id: String,
    pub files: Vec<DeliveredFile>,
}

/// Newest first; a run's files by name, so the order never moves between reads.
pub fn newest_first(files: &[DeliveredFile]) -> Vec<DeliveredFile> {
    let mut f = files.to_vec();
    f.sort_by(|a, b| b.run_at.cmp(&a.run_at).then_with(|| a.name.cmp(&b.name)));
    f
}

fn grouped(files: Vec<DeliveredFile>, key: impl Fn(&DeliveredFile) -> String) -> Vec<Group> {
    let mut out: Vec<Group> = vec![];
    for f in files {
        let k = key(&f);
        match out.iter_mut().find(|g| g.id == k) {
            Some(g) => g.files.push(f),
            None => out.push(Group {
                id: k,
                files: vec![f],
            }),
        }
    }
    out
}

/// Under the day of their run in the account's zone (`byDay`).
pub fn by_day(files: &[DeliveredFile]) -> Vec<Group> {
    grouped(newest_first(files), |f| {
        crate::account_time::day(&f.run_at).to_string()
    })
}

/// Each taf with its files, the taf with the newest run first (`byTaf`).
pub fn by_taf(files: &[DeliveredFile]) -> Vec<Group> {
    grouped(newest_first(files), |f| f.taf.clone())
}

/// Each run with its files, the newest run first (`byRun`).
pub fn by_run(files: &[DeliveredFile]) -> Vec<Group> {
    grouped(newest_first(files), |f| f.run.clone())
}

/// Whole days left before it goes, never below none: 0 is today (`daysLeft`).
pub fn days_left(f: &DeliveredFile, now: DateTime<Utc>) -> i64 {
    ((f.goes_at - now).num_seconds() / 86_400).max(0)
}

/// When it goes: today, tomorrow, or in so many days (`FileDays.goes`).
pub fn goes(w: &WordBook, days: i64) -> Result<String, WordsError> {
    match days {
        0 => w.line("files.goes_today"),
        1 => w.line("files.goes_tomorrow"),
        n => w.count("files.goes_in_days", n, &[]),
    }
}

/// A group's day, as the iPad heads it: the weekday, the day and the month, in the account's zone.
pub fn day_title(language: &str, day: NaiveDate) -> String {
    crate::locale::weekday_day_month(language, day)
}

/// The byte units in each shipped language, as CLDR writes them short (from ICU 74's `MeasureUnit`, as the iPad's
/// `ByteCountFormatter` words a size): the byte by plural form, then kB, MB and GB.
type Units = (
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static str,
    &'static str,
    &'static str,
);
const UNITS: &[Units] = &[
    ("en", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    ("de", &[("other", "{0} Byte")], "{0} kB", "{0} MB", "{0} GB"),
    ("fr", &[("other", "{0} o")], "{0} ko", "{0} Mo", "{0} Go"),
    ("es", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    ("it", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    (
        "pt",
        &[("other", "{0} bytes")],
        "{0} kB",
        "{0} MB",
        "{0} GB",
    ),
    ("nl", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    ("pl", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    ("ko", &[("other", "{0}byte")], "{0}kB", "{0}MB", "{0}GB"),
    ("ja", &[("other", "{0} byte")], "{0} KB", "{0} MB", "{0} GB"),
    (
        "et",
        &[("one", "{0} bait"), ("other", "{0} baiti")],
        "{0} kB",
        "{0} MB",
        "{0} GB",
    ),
    ("lt", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    ("lv", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    ("cs", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    ("sk", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    (
        "sl",
        &[
            ("one", "{0} bajt"),
            ("two", "{0} bajta"),
            ("few", "{0} bajti"),
            ("other", "{0} bajtov"),
        ],
        "{0} kB",
        "{0} MB",
        "{0} GB",
    ),
    ("hu", &[("other", "{0} bájt")], "{0} kB", "{0} MB", "{0} GB"),
    ("ro", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    ("bg", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    (
        "hr",
        &[
            ("one", "{0} bajt"),
            ("few", "{0} bajta"),
            ("other", "{0} bajtova"),
        ],
        "{0} kB",
        "{0} MB",
        "{0} GB",
    ),
    ("tr", &[("other", "{0} bayt")], "{0} kB", "{0} MB", "{0} GB"),
    ("id", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    ("vi", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    ("th", &[("other", "{0} ไบต์")], "{0} kB", "{0} MB", "{0} GB"),
    ("zh", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    (
        "ar",
        &[("other", "{0} بايت")],
        "{0} كيلوبايت",
        "{0} م.ب",
        "{0} غ.ب",
    ),
    ("sr", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    ("sq", &[("other", "{0} bajt")], "{0} kB", "{0} MB", "{0} GB"),
    (
        "ka",
        &[("other", "{0} ბაიტი")],
        "{0} კბაიტი",
        "{0} MB",
        "{0} GB",
    ),
    (
        "hy",
        &[("one", "{0} Բայթ"), ("other", "{0} Բ")],
        "{0} կԲ",
        "{0} ՄԲ",
        "{0} ԳԲ",
    ),
    ("az", &[("other", "{0} bayt")], "{0} kB", "{0} MB", "{0} GB"),
    ("bn", &[("other", "{0} বাইট")], "{0} kB", "{0} MB", "{0} GB"),
    ("he", &[("other", "{0} בייט")], "{0} kB", "{0} MB", "{0} GB"),
    ("ca", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    (
        "mk",
        &[("one", "{0} бајт"), ("other", "{0} бајти")],
        "{0} kB",
        "{0} MB",
        "{0} GB",
    ),
    ("mn", &[("other", "{0} байт")], "{0} кБ", "{0} МБ", "{0} ГБ"),
    ("ne", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    ("km", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    ("lo", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
    ("tk", &[("other", "{0} B")], "{0} kB", "{0} MB", "{0} GB"),
    ("hi", &[("other", "{0} बाइट")], "{0} kB", "{0} MB", "{0} GB"),
    ("ur", &[("other", "{0} byte")], "{0} kB", "{0} MB", "{0} GB"),
];

/// A size as the language words it (`FileSize.words`): bytes below a kilobyte; whole kilobytes; megabytes with one
/// decimal, gigabytes with two, each only when it isn't whole. Decimal units, as the iPad's file style counts them.
pub fn size(language: &str, bytes: i64) -> String {
    let base = crate::locale::base(language);
    let u = UNITS.iter().find(|u| u.0 == base).unwrap_or(&UNITS[0]);
    let bytes = bytes.max(0);
    // A count at `decimals`, rounded, its decimals shown only when they aren't zero.
    let shown = |value: f64, decimals: u32| -> String {
        let scale = 10i64.pow(decimals);
        let minor = (value * scale as f64).round() as i64;
        if minor % scale == 0 {
            crate::locale::number(minor / scale, 0, language)
        } else {
            crate::locale::number(minor, decimals, language)
        }
    };
    let (n, pattern) = if bytes < 1_000 {
        let form = crate::locale::plural_form(language, bytes);
        let p =
            u.1.iter()
                .find(|(f, _)| *f == form)
                .or_else(|| u.1.iter().find(|(f, _)| *f == "other"))
                .map(|(_, p)| *p)
                .unwrap_or("{0}");
        (crate::locale::number(bytes, 0, language), p)
    } else if bytes < 1_000_000 {
        (shown(bytes as f64 / 1e3, 0), u.2)
    } else if bytes < 1_000_000_000 {
        (shown(bytes as f64 / 1e6, 1), u.3)
    } else {
        (shown(bytes as f64 / 1e9, 2), u.4)
    };
    pattern.replace("{0}", &n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    const LIST: &[u8] = include_bytes!("../../../shared/contracts/files-list.json");

    fn answer(contract: &[u8]) -> Vec<u8> {
        let v: serde_json::Value = serde_json::from_slice(contract).unwrap();
        serde_json::to_vec(&v["answer"]).unwrap()
    }

    #[test]
    fn the_contract_reads_as_the_ipad_reads_it() {
        let (files, next) = read_page(&answer(LIST)).unwrap();
        assert_eq!(next, None);
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "notes.md");
        assert_eq!(files[0].kind, "MD");
        assert_eq!(files[0].taf_name, "Order rows");
        assert_eq!(files[1].chat.as_deref(), Some("chat-1"));
        assert_eq!(files[1].kind, "CSV");
        // A list that can't be read is none, not half a list.
        assert!(read_page(br#"{"files":[{"id":"x"}]}"#).is_none());
    }

    #[test]
    fn a_name_is_never_a_path_and_a_preview_keeps_only_lines_and_tabs() {
        assert_eq!(name("../../etc/passwd", "f1"), "passwd");
        assert_eq!(name("a\\b\\report.csv", "f1"), "report.csv");
        assert_eq!(name("..", "f1"), "f1");
        assert_eq!(name("re\u{202e}port\u{7}.pdf", "f1"), "report.pdf");
        assert_eq!(preview("a\r\nb\tc\u{2066}\u{1b}d"), "a\nb\tcd");
    }

    #[test]
    fn the_download_is_https_only_and_its_preview_is_cleaned() {
        let d = std::fs::read("../../shared/contracts/files-download.json").unwrap();
        assert!(read_download(&answer(&d)).is_some());
        assert!(read_download(br#"{"url":"http://s3.test/x"}"#).is_none());
        assert!(read_download(br#"{"url":"https://a:b@s3.test/x"}"#).is_none());
        let f = std::fs::read("../../shared/contracts/files-file.json").unwrap();
        let p = read_preview(&answer(&f)).unwrap().unwrap();
        assert_eq!(p.text, "name,amount\nAna,12.50\n");
        assert!(!p.truncated);
        assert_eq!(read_preview(b"{}"), Some(None));
        assert_eq!(download_path("a b"), "/v1/files/a%20b/download");
        assert_eq!(list_path(Some("c/1")), "/v1/files?after=c%2F1");
    }

    fn file(id: &str, taf: &str, run: &str, at: i64, name: &str) -> DeliveredFile {
        let t = Utc.timestamp_opt(at, 0).unwrap();
        DeliveredFile {
            id: id.into(),
            name: name.into(),
            kind: "CSV".into(),
            bytes: 1,
            taf: taf.into(),
            taf_name: String::new(),
            run: run.into(),
            run_at: t,
            chat: None,
            goes_at: t + chrono::Duration::days(KEPT_DAYS),
        }
    }

    #[test]
    fn groups_keep_the_newest_first_and_a_runs_files_by_name() {
        let all = vec![
            file("1", "a", "r1", 100, "b.csv"),
            file("2", "b", "r2", 300, "x.csv"),
            file("3", "a", "r1", 100, "a.csv"),
        ];
        let runs = by_run(&all);
        assert_eq!(runs[0].id, "r2");
        assert_eq!(
            runs[1]
                .files
                .iter()
                .map(|f| f.id.as_str())
                .collect::<Vec<_>>(),
            ["3", "1"]
        );
        assert_eq!(by_taf(&all)[0].id, "b");
        let f = &all[0];
        assert_eq!(days_left(f, f.run_at), KEPT_DAYS);
        assert_eq!(days_left(f, f.goes_at + chrono::Duration::days(3)), 0);
    }

    #[test]
    fn sizes_read_as_each_language_writes_them() {
        assert_eq!(size("en", 22), "22 byte");
        assert_eq!(size("en", 14_000), "14 kB");
        assert_eq!(size("en", 1_500_000), "1.5 MB");
        assert_eq!(size("en", 2_000_000), "2 MB");
        assert_eq!(size("fr", 86_000), "86 ko");
        assert_eq!(size("de", 1_250_000), "1,3 MB");
        assert_eq!(size("ko", 14_000), "14kB");
        assert_eq!(size("sl", 5), "5 bajtov");
        assert_eq!(size("sl", 1), "1 bajt");
        assert_eq!(size("pt-BR", 8), "8 bytes");
        assert_eq!(size("ar", 14_000), "14 كيلوبايت");
        for lang in crate::words::SHIPPED {
            for n in [0, 1, 2, 3, 5, 22, 999, 14_000, 2_500_000, 3_200_000_000] {
                let s = size(lang, n);
                assert!(!s.contains('{') && !s.is_empty(), "{lang} {n}: {s}");
            }
        }
        // Turkmen, Hindi and Urdu, in CLDR 45's short units (Urdu's is the Latin "byte", for one
        // too: its short unit has only "other").
        assert_eq!(size("tk", 512), "512 B");
        assert_eq!(size("hi", 512), "512 बाइट");
        assert_eq!(size("ur", 1), "1 byte");
        assert_eq!(size("ur", 512), "512 byte");
    }

    #[test]
    fn when_it_goes_in_every_language() {
        let en = WordBook::load(&WordBook::default_root(), "en").unwrap();
        assert_eq!(goes(&en, 0).unwrap(), "goes today");
        assert_eq!(goes(&en, 1).unwrap(), "goes tomorrow");
        assert_eq!(goes(&en, 12).unwrap(), "goes in 12 days");
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            for n in [0, 1, 2, 3, 5, 11, 21, 29] {
                assert!(!goes(&w, n).unwrap().contains('{'), "{lang} {n}");
            }
        }
    }
}
