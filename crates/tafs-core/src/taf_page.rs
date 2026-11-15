//! A taf's page (`TafDetailView`): its head (`header`, `StateChip`), its tabs, its Card bar
//! (`CardBar`) and its Activity as the sample draws it (`activity(_:)`, `ActivityFilter`).

use chrono::{DateTime, Local};

use crate::card::Card;
use crate::record::{day_title, spent, Entry, Kind};
use crate::words::{WordBook, WordsError};

/// A taf's state (`TafState`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TafState {
    Working,
    Waiting,
    Idle,
    Paused,
    Revoked,
    Halted,
}

/// The chip's colour (`StateChip.color`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChipTone {
    Ok,
    Amber,
    No,
    Soft,
}

impl TafState {
    /// `KIFFMapping.taf`'s state from tafs cloud's word.
    pub fn from_cloud(state: &str, unscheduled: bool) -> TafState {
        match state {
            "waiting" => TafState::Waiting,
            "paused" => TafState::Paused,
            "halted" => TafState::Halted,
            "working" if unscheduled => TafState::Idle,
            "working" => TafState::Working,
            "idle" => TafState::Idle,
            _ => TafState::Revoked,
        }
    }

    /// `label`.
    pub fn label(self, w: &WordBook) -> Result<String, WordsError> {
        w.line(match self {
            TafState::Working => "taf.state.working",
            TafState::Waiting => "taf.state.waiting",
            TafState::Idle => "taf.state.ready",
            TafState::Paused => "taf.state.paused",
            TafState::Revoked => "taf.state.stopped",
            TafState::Halted => "taf.state.halted",
        })
    }

    pub fn tone(self) -> ChipTone {
        match self {
            TafState::Working => ChipTone::Ok,
            TafState::Waiting => ChipTone::Amber,
            TafState::Revoked | TafState::Halted => ChipTone::No,
            TafState::Idle | TafState::Paused => ChipTone::Soft,
        }
    }
}

/// `header`: the state chip shows unless what went wrong is said in place of "Working".
pub fn shows_chip(state: TafState, attention: bool) -> bool {
    !(attention && state == TafState::Working)
}

/// The Activity's filters (`ActivityFilter`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityFilter {
    All,
    Done,
    Held,
    Refused,
}

impl ActivityFilter {
    pub const ALL: [ActivityFilter; 4] = [
        ActivityFilter::All,
        ActivityFilter::Done,
        ActivityFilter::Held,
        ActivityFilter::Refused,
    ];

    /// `title`: "Done" is the iPad's `Words.Taf.Halt.partHeld`.
    pub fn title(self, w: &WordBook) -> Result<String, WordsError> {
        w.line(match self {
            ActivityFilter::All => "record.filter.all",
            ActivityFilter::Done => "taf.halt.part_held",
            ActivityFilter::Held => "record.filter.held",
            ActivityFilter::Refused => "record.filter.refused",
        })
    }

    pub fn keeps(self, e: &Entry) -> bool {
        match self {
            ActivityFilter::All => true,
            ActivityFilter::Done => matches!(e.kind, Kind::Done | Kind::Approved),
            ActivityFilter::Held => e.kind == Kind::Held,
            ActivityFilter::Refused => {
                matches!(e.kind, Kind::Refused | Kind::NotDone | Kind::Revoked)
            }
        }
    }
}

/// One day of a taf's Activity: its heading and its lines, newest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityDay<'a> {
    pub title: String,
    pub lines: Vec<&'a Entry>,
}

/// `activity(_:)`: the taf's lines, filtered, by day, newest first; each day's heading with what
/// was spent. Reads aren't folded here.
pub fn activity<'a>(
    w: &WordBook,
    entries: &'a [Entry],
    taf: &str,
    filter: ActivityFilter,
    now: DateTime<Local>,
) -> Result<Vec<ActivityDay<'a>>, WordsError> {
    let mut kept: Vec<&Entry> = entries
        .iter()
        .filter(|e| e.taf == taf && filter.keeps(e))
        .collect();
    kept.sort_by_key(|e| std::cmp::Reverse(e.at));
    let mut out: Vec<(chrono::NaiveDate, Vec<&Entry>)> = Vec::new();
    for e in kept {
        let day = e.at.with_timezone(&Local).date_naive();
        match out.last_mut() {
            Some((d, lines)) if *d == day => lines.push(e),
            _ => out.push((day, vec![e])),
        }
    }
    out.into_iter()
        .map(|(day, lines)| {
            let title = day_title(w, day, now.date_naive())?;
            let title = match spent(w, &lines)? {
                Some(sum) => format!("{title} · {sum}"),
                None => title,
            };
            Ok(ActivityDay { title, lines })
        })
        .collect()
}

/// `CardBar`'s two lines: the hero's figure and words, else the Card's title; then its title, its
/// environment when it isn't production, and how many Cards there are. Dimmed when revoked or
/// not active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardBarLines {
    pub line: String,
    pub details: String,
    pub dim: bool,
    /// Its colour: the taf's when live, else the dim colour (`TafCardFace.fill`).
    pub live: bool,
}

pub struct CardMeta<'a> {
    pub title: &'a str,
    pub environment: &'a str,
    pub live: bool,
    pub active: bool,
}

pub fn card_bar(
    w: &WordBook,
    card: &Card,
    meta: &CardMeta,
    count: usize,
    revoked: bool,
) -> Result<CardBarLines, WordsError> {
    let line = match card.hero(w)? {
        Some((figure, words)) => format!("{figure} {words}"),
        None => meta.title.to_string(),
    };
    let mut parts = vec![meta.title.to_string()];
    // `environmentLabel`, capitalised: none for a live Card or every environment.
    if !meta.live && !meta.environment.is_empty() {
        let upper = meta.environment.to_uppercase();
        let mut c = upper.chars();
        let cap = c
            .next()
            .map(|f| f.to_string() + &c.as_str().to_lowercase())
            .unwrap_or_default();
        parts.push(cap);
    }
    if count > 1 {
        parts.push(w.count("card.count", count as i64, &[])?);
    }
    Ok(CardBarLines {
        line,
        details: parts.join(" · "),
        dim: revoked || !meta.active,
        live: meta.live,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states_from_tafs_cloud() {
        assert_eq!(TafState::from_cloud("working", true), TafState::Idle);
        assert_eq!(TafState::from_cloud("working", false), TafState::Working);
        assert_eq!(TafState::from_cloud("stopped", false), TafState::Revoked);
        assert!(!shows_chip(TafState::Working, true));
        assert!(shows_chip(TafState::Paused, true));
    }
}
