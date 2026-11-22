//! The Overview: today across the tafs, one line each (`OverviewView` in `ShareViews.swift`),
//! as the whole app's sidebar opens it (`showsCalendar` off: Calendar is a sidebar item).

use crate::card::Card;
use crate::sidebar::{HeldCall, Selection};
use crate::words::{upper, WordBook, WordsError};

/// One taf as the Overview reads it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewTaf {
    pub key: String,
    pub name: String,
    pub initial: String,
    pub color_index: i64,
    /// What it's doing now (`taf.now`).
    pub now: String,
    /// The Card in front (`shownCards.first`).
    pub card: Option<Card>,
    /// tafs cloud runs it (`cloudTaf(for:)`): its last result is read; the sample's aren't.
    pub cloud: Option<CloudRun>,
    /// "Support · 3 people": who it's shared with, for whoever manages it, once read.
    pub used_by: String,
    /// Its labels by id, as Home says (`labels(of:)`).
    pub labels: Vec<String>,
}

/// What tafs cloud says of a taf's last run.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CloudRun {
    /// Its latest run's report, as text, once read.
    pub report: Option<String>,
    pub error: Option<String>,
    /// When it last ran, as a time of day already in the account's zone.
    pub last_run: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewLines {
    pub title: String,
    pub subtitle: String,
    /// NEEDS YOU, when anything waits (`needsYouCount > 0`).
    pub needs_you_label: Option<String>,
    pub held: Vec<HeldRow>,
    /// What else waits, beyond the held calls listed: tafs cloud's count in its words.
    pub rest: Option<String>,
    pub columns: [String; 4],
    pub rows: Vec<OverviewRow>,
    /// "No tafs yet.", with none listed; while the list is read, or when it couldn't be, that
    /// (`Words.Home.loading`, `notLoaded`).
    pub empty: Option<String>,
    /// Where the account's list stands (`loadingList`, `listUnread`).
    pub list: ListState,
    pub each_line_opens: String,
    /// The label chips over the grid (`listLabels`): Home's labels in order, none with no
    /// labelled taf; and the first chip's word (`labelsAllWords`).
    pub labels: Vec<crate::brief::Label>,
    pub labels_all: String,
    /// The first Home (the PM's (b), kiff/work#20, 22 Nov, as the iPad's `HomeView` draws it):
    /// with no taf, for someone who may use Coach, tafs cloud's own empty line (else "No tafs
    /// yet" and what Coach does), then Ask Coach and How tafs works. None otherwise.
    pub first_home: Option<(String, Option<String>)>,
}

/// Where the account's list stands: being read, read, or not read (`loadingList`, `listUnread`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ListState {
    Loading,
    #[default]
    Read,
    Unread,
}

/// The board over an Overview with no taf (`HomeSplitView`'s pane with nothing chosen): board 2
/// once the list is read and empty, else the mascot (72) while it's read or when it couldn't be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Board {
    Overview,
    NoTafs,
    Mascot,
}

impl OverviewLines {
    /// The first Home, when the list is read and holds no taf and the person may use Coach
    /// (`Onboarding.showsFirstHome`): its lines (`onboarding::first_home_lines`), drawn in the
    /// empty line's place.
    pub fn with_first_home(
        mut self,
        w: &WordBook,
        may_use_coach: bool,
        cloud_words: Option<&str>,
    ) -> Result<OverviewLines, WordsError> {
        if may_use_coach && self.list == ListState::Read && self.rows.is_empty() {
            self.first_home = Some(crate::onboarding::first_home_lines(w, cloud_words)?);
        }
        Ok(self)
    }

    /// The lines as the list stands: with no taf, "Loading your tafs…" while it's read and
    /// "Couldn't load your tafs." when it couldn't be, in place of "No tafs yet.".
    pub fn with_list(mut self, w: &WordBook, list: ListState) -> Result<OverviewLines, WordsError> {
        self.list = list;
        if self.empty.is_some() {
            self.empty = Some(w.line(match list {
                ListState::Loading => "home.loading",
                ListState::Read => "home.no_tafs",
                ListState::Unread => "home.not_loaded",
            })?);
        }
        Ok(self)
    }

    /// What's drawn under the title.
    pub fn board(&self) -> Board {
        match (self.empty.is_some(), self.list) {
            (false, _) => Board::Overview,
            (true, ListState::Read) => Board::NoTafs,
            (true, _) => Board::Mascot,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldRow {
    pub selection: Selection,
    /// "Deploys · Run migration 0042_orders_index"
    pub title: String,
    /// "answer by 03:02"
    pub answer_by: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverviewRow {
    pub selection: Selection,
    pub name: String,
    pub initial: String,
    pub color_index: i64,
    pub used_by: String,
    pub now: String,
    /// "9 calls left today of 10", else "–".
    pub card: String,
    pub last_result: String,
    /// What a screen reader says for the whole line.
    pub accessible: String,
    /// Its labels by id, to filter by, and as its small grey chips: two, then "+N" (`TafLabels`).
    pub label_ids: Vec<String>,
    pub labels: Vec<String>,
}

/// `OverviewView.showsRest`: Needs you counts more than the held calls listed here.
pub fn shows_rest(count: i64, held: usize) -> bool {
    count > held as i64
}

/// `lastResult`: the first line of its last run's report, its Markdown's `**` and `#` dropped,
/// else that run's error, else when it last ran, else "Nothing yet". "–" for a taf tafs cloud
/// doesn't run.
pub fn last_result(w: &WordBook, cloud: Option<&CloudRun>) -> Result<String, WordsError> {
    let Some(cloud) = cloud else {
        return Ok("–".into());
    };
    if let Some(report) = &cloud.report {
        let first = report.split('\n').map(str::trim).find(|l| !l.is_empty());
        if let Some(first) = first {
            return Ok(first.replace("**", "").replace('#', "").trim().to_string());
        }
        if let Some(error) = &cloud.error {
            return Ok(error.clone());
        }
    }
    match &cloud.last_run {
        Some(time) => w.get("people.last_ran", &[("time", time)]),
        None => w.line("overview.nothing_yet"),
    }
}

/// The Overview's lines. `needs_you_count` is the sidebar's (`needsYouCount`), and
/// `needs_you_words` Home's line for it (`needsYouWords`).
pub fn lines(
    w: &WordBook,
    tafs: &[OverviewTaf],
    held: &[HeldCall],
    needs_you_count: i64,
    needs_you_words: &str,
) -> Result<OverviewLines, WordsError> {
    let mut held_rows = Vec::new();
    for call in held {
        let Some(taf) = tafs.iter().find(|t| t.key == call.taf) else {
            continue;
        };
        held_rows.push(HeldRow {
            selection: Selection::Held {
                call: call.id.clone(),
                taf: taf.key.clone(),
            },
            title: format!("{} · {}", taf.name, call.summary),
            answer_by: w.get(
                "needs_you.answer_by",
                &[(
                    "time",
                    &crate::account_time::zoned_short_time(&call.expires_at),
                )],
            )?,
        });
    }
    let mut rows = Vec::new();
    for t in tafs {
        let card = match &t.card {
            Some(c) => c.line(w)?,
            None => None,
        };
        let last = last_result(w, t.cloud.as_ref())?;
        let accessible = [
            Some(t.name.clone()),
            Some(t.now.clone()),
            card.clone(),
            Some(last.clone()),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(", ");
        rows.push(OverviewRow {
            selection: Selection::Taf(t.key.clone()),
            name: t.name.clone(),
            initial: t.initial.clone(),
            color_index: t.color_index,
            used_by: t.used_by.clone(),
            now: t.now.clone(),
            card: card.unwrap_or_else(|| "–".into()),
            last_result: last,
            accessible,
            label_ids: t.labels.clone(),
            labels: vec![],
        });
    }
    Ok(OverviewLines {
        title: w.line("home.overview.title")?,
        subtitle: w.line("home.overview.subtitle")?,
        needs_you_label: (needs_you_count > 0)
            .then(|| w.line("needs_you.title").map(|s| upper(&s)))
            .transpose()?,
        held: if needs_you_count > 0 {
            held_rows
        } else {
            vec![]
        },
        rest: (needs_you_count > 0 && shows_rest(needs_you_count, held.len()))
            .then(|| needs_you_words.to_string()),
        columns: [
            w.line("home.overview.column_taf")?,
            w.line("home.overview.column_now")?,
            w.line("home.overview.column_card")?,
            w.line("home.overview.column_last_result")?,
        ],
        empty: tafs
            .is_empty()
            .then(|| w.line("home.no_tafs"))
            .transpose()?,
        list: ListState::Read,
        rows,
        each_line_opens: w.line("home.overview.each_line_opens")?,
        labels: vec![],
        labels_all: w.line("home.labels_all")?,
        first_home: None,
    })
}

/// The labels' chips over the grid, and each row's (`listLabels`, `labelsAllWords`,
/// `TafLabels`): Home's labels and its word for All, else the brief's, else the app's.
pub fn with_labels(
    w: &WordBook,
    mut o: OverviewLines,
    labels: Vec<crate::brief::Label>,
    home_all: Option<&str>,
    brief: Option<&crate::brief::Brief>,
) -> Result<OverviewLines, WordsError> {
    for row in o.rows.iter_mut() {
        row.labels = crate::brief::taf_chips(&crate::brief::names_of(&labels, &row.label_ids));
    }
    o.labels_all = crate::brief::all_words(w, home_all, brief)?;
    o.labels = labels;
    Ok(o)
}

/// `needsYouWords` when tafs cloud sent none: the count in the app's words.
pub fn needs_you_words(w: &WordBook, count: i64) -> Result<String, WordsError> {
    if count == 0 {
        w.line("needs_you.nothing")
    } else {
        w.count("needs_you.count", count, &[])
    }
}

#[cfg(test)]
mod list_tests {
    use super::*;

    #[test]
    fn with_no_taf_the_overview_says_where_the_list_stands() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let o = lines(&w, &[], &[], 0, "").unwrap();
        assert_eq!(o.board(), Board::NoTafs);
        let loading = o.clone().with_list(&w, ListState::Loading).unwrap();
        assert_eq!(loading.empty.as_deref(), Some("Loading your tafs…"));
        assert_eq!(loading.board(), Board::Mascot);
        let unread = o.clone().with_list(&w, ListState::Unread).unwrap();
        assert_eq!(unread.empty.as_deref(), Some("Couldn't load your tafs."));
        assert_eq!(unread.board(), Board::Mascot);
        let read = o.with_list(&w, ListState::Read).unwrap();
        assert_eq!(read.empty.as_deref(), Some("No tafs yet."));
    }

    #[test]
    fn the_first_home_is_for_someone_who_may_use_coach_with_no_taf() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let read = |may| {
            lines(&w, &[], &[], 0, "")
                .unwrap()
                .with_list(&w, ListState::Read)
                .unwrap()
                .with_first_home(&w, may, None)
                .unwrap()
                .first_home
        };
        let (title, lead) = read(true).unwrap();
        assert_eq!(title, w.line("home.no_tafs").unwrap());
        assert!(lead.unwrap().starts_with("Tell Coach"));
        assert_eq!(read(false), None);
        // tafs cloud's own line comes first, with no lead of the app's.
        let o = lines(&w, &[], &[], 0, "")
            .unwrap()
            .with_list(&w, ListState::Read)
            .unwrap()
            .with_first_home(&w, true, Some("Your first taf starts here."))
            .unwrap();
        assert_eq!(
            o.first_home,
            Some(("Your first taf starts here.".into(), None))
        );
        // Not while the list is read.
        let loading = lines(&w, &[], &[], 0, "")
            .unwrap()
            .with_list(&w, ListState::Loading)
            .unwrap()
            .with_first_home(&w, true, None)
            .unwrap();
        assert_eq!(loading.first_home, None);
    }
}
