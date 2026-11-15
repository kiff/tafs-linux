//! The sidebar for the whole app: the iPad's `HomeSplitView` on a full-width iPad (`tab` set),
//! and what it draws from `AppModel` (`relevantNow`, `needsYouCount`, `accountLabel`,
//! `coachAvailable`). The app only places what this says.

use chrono::{DateTime, Local};

use crate::words::{upper, WordBook, WordsError};

/// What the sidebar opens beside it (`HomeSelection`, the cases a sidebar row picks).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Selection {
    /// Today across the tafs: the mascot and "tafs" open it, and it's what opens first.
    Overview,
    NeedsYou,
    Record,
    Calendar,
    Usage,
    ConnectApps,
    Account,
    /// Account › Devices.
    Devices,
    /// Account › People & groups.
    People,
    /// Coach mode, from its button at the foot.
    Coach,
    /// A screen Account opens, by its title, until it's ported.
    Titled(String),
    /// A taf, by its key.
    Taf(String),
    /// A held call, on its taf's page.
    Held {
        call: String,
        taf: String,
    },
}

/// How the app is signed in (`AppModel.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Kiff,
    Sample,
    SignedOut,
}

/// One of the app's screens as a sidebar item (`screenItem`): its words, its symbol, its count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScreenItem {
    pub selection: Selection,
    pub title: String,
    /// The iPad's SF Symbol, which the app draws with its nearest icon.
    pub symbol: &'static str,
    /// Needs you's count, a plain number at the row's end (`.badge`): none at zero.
    pub badge: Option<String>,
    /// What a screen reader says: "Needs you, 2" with a count.
    pub accessible: String,
}

pub use crate::held::HeldCall;

/// A taf, as the sidebar's row shows it (`tafRow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TafRow {
    pub key: String,
    pub name: String,
    pub initial: String,
    pub color_index: i64,
    /// What it's doing now, in the dim colour (`taf.now`).
    pub now: String,
    /// A small amber badge after the name (`taf.badge`), when it has one.
    pub badge: Option<String>,
    /// The line's colour: dim for what it's doing, amber or red for what went wrong.
    pub tone: Tone,
}

/// A row's line colour (`tafRow`): dim, the amber as text, or red when it's failing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tone {
    #[default]
    Dim,
    Amber,
    No,
}

/// The sidebar's state, from the app's model.
#[derive(Debug, Clone)]
pub struct SidebarModel {
    pub mode: Mode,
    /// The reference set's sample drawn as a signed-in account (`Screenshots.signedIn`).
    pub screenshot_signed_in: bool,
    /// `mayUseCoach`: KIFF's capabilities, none read yet counting as allowed (`allows`).
    pub may_use_coach: bool,
    /// The held calls, newest first.
    pub held: Vec<HeldCall>,
    /// Every taf, in the list's order (`listedTafs`), and whether each is relevant now.
    pub tafs: Vec<(TafRow, bool)>,
    /// Home has been read from tafs cloud, and read cleanly (`listLoaded && !listUnread`).
    pub list_loaded: bool,
    /// tafs cloud's line for a Home with no taf (`recipes.homeWords`).
    pub home_words: Option<String>,
    /// The account's name, once KIFF says it (`kiffAccount.name`).
    pub account_name: Option<String>,
    /// Home's count from tafs cloud (`homeNeedsYou`); none counts the held calls here.
    pub cloud_needs_you: Option<i64>,
}

/// Everything the sidebar draws, top to foot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SidebarLines {
    pub brand: String,
    pub screens: Vec<ScreenItem>,
    /// NEEDS YOU, in the amber, when calls are held.
    pub held_label: Option<String>,
    pub held: Vec<HeldLine>,
    /// YOUR TAFS, with its rows, when the section shows.
    pub your_tafs_label: Option<String>,
    pub your_tafs: Vec<TafRow>,
    /// The line under Your tafs with no taf at all.
    pub no_tafs: Option<String>,
    /// Coach's button, for those who may use it.
    pub coach: Option<String>,
    pub account: String,
    pub account_label: Option<String>,
    pub account_accessible: String,
}

/// A held call's two lines (`heldRow`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeldLine {
    pub selection: Selection,
    pub title: String,
    pub line: String,
}

impl SidebarModel {
    /// `signedInLayout`: KIFF, or its screenshot over the sample.
    pub fn signed_in_layout(&self) -> bool {
        self.mode == Mode::Kiff || self.screenshot_signed_in
    }

    /// `needsYouCount`: tafs cloud's count, or the app's own count of held calls.
    pub fn needs_you_count(&self) -> i64 {
        self.cloud_needs_you
            .map(|n| n.max(0))
            .unwrap_or(self.held.len() as i64)
    }

    /// `coachAvailable || Screenshots.signedIn`: signed in to KIFF and allowed to use Coach.
    pub fn coach_available(&self) -> bool {
        (self.mode == Mode::Kiff && self.may_use_coach) || self.screenshot_signed_in
    }

    /// `accountLabel`: the account's name, in KIFF only.
    pub fn account_label(&self) -> Option<&str> {
        if self.mode == Mode::Kiff {
            self.account_name.as_deref()
        } else {
            None
        }
    }

    pub fn lines(&self, w: &WordBook) -> Result<SidebarLines, WordsError> {
        let count = self.needs_you_count();
        let needs_you = w.line("needs_you.title")?;
        let mut screens = vec![ScreenItem {
            selection: Selection::NeedsYou,
            title: needs_you.clone(),
            symbol: "bell",
            badge: (count > 0).then(|| count.to_string()),
            accessible: if count > 0 {
                w.get("needs_you.tab_label", &[("n", &count.to_string())])?
            } else {
                needs_you
            },
        }];
        screens.push(item(
            Selection::Record,
            w.line("record.title")?,
            "list.bullet",
        ));
        if self.signed_in_layout() {
            screens.push(item(
                Selection::Calendar,
                w.line("calendar.heading")?,
                "calendar",
            ));
            screens.push(item(Selection::Usage, w.line("usage.title")?, "chart.bar"));
            screens.push(item(
                Selection::ConnectApps,
                w.line("connect.apps.title")?,
                "app.connected.to.app.below.fill",
            ));
        }

        let mut held = Vec::new();
        for call in &self.held {
            // A call whose taf isn't listed isn't drawn (`if let taf = model.taf(call.tafID)`).
            let Some((taf, _)) = self.tafs.iter().find(|(t, _)| t.key == call.taf) else {
                continue;
            };
            held.push(HeldLine {
                selection: Selection::Held {
                    call: call.id.clone(),
                    taf: taf.key.clone(),
                },
                title: format!("{}: {}", taf.name, call.summary),
                line: w.get(
                    "home.reason_answer_by",
                    &[
                        ("why", &call.reason),
                        (
                            "time",
                            &crate::account_time::zoned_short_time(&call.expires_at),
                        ),
                    ],
                )?,
            });
        }
        let held_label =
            (!held.is_empty()).then(|| upper(&w.line("needs_you.title").unwrap_or_default()));

        // As one sidebar for the whole app, only the tafs relevant now: all of them are on the Overview.
        let relevant: Vec<TafRow> = self
            .tafs
            .iter()
            .filter(|(_, r)| *r)
            .map(|(t, _)| t.clone())
            .collect();
        let shows = !relevant.is_empty() || self.tafs.is_empty();
        let no_tafs = if self.mode == Mode::Kiff && self.list_loaded && self.tafs.is_empty() {
            Some(match &self.home_words {
                Some(words) => words.clone(),
                None => w.line(if self.may_use_coach {
                    "home.connect_tool_set_up_your"
                } else {
                    "home.tafs_shared_you_show"
                })?,
            })
        } else {
            None
        };

        let account = w.line("settings.title")?;
        let account_accessible = match self.account_label() {
            Some(name) => w.get("settings.account_named", &[("account", name)])?,
            None => account.clone(),
        };
        Ok(SidebarLines {
            brand: w.line("app.name")?,
            screens,
            held_label,
            held,
            your_tafs_label: shows
                .then(|| w.line("home.your_tafs").map(|s| upper(&s)))
                .transpose()?,
            your_tafs: if shows { relevant } else { vec![] },
            no_tafs: if shows { no_tafs } else { None },
            coach: self
                .coach_available()
                .then(|| w.line("coach.title"))
                .transpose()?,
            account,
            account_label: self.account_label().map(str::to_string),
            account_accessible,
        })
    }
}

fn item(selection: Selection, title: String, symbol: &'static str) -> ScreenItem {
    ScreenItem {
        accessible: title.clone(),
        selection,
        title,
        symbol,
        badge: None,
    }
}

/// `shortTime`: the time of day as 14:05, in the account's zone (`account_time`).
pub fn short_time(at: DateTime<Local>) -> String {
    crate::account_time::short_time(&at)
}

/// `AppModel.relevantNow`: a call of its waits, a step or a knock is open, or it ran or changed
/// in the last 24 hours.
pub fn relevant_now(
    held: bool,
    step: bool,
    knock: bool,
    last_run: Option<DateTime<Local>>,
    last_change: Option<DateTime<Local>>,
    now: DateTime<Local>,
) -> bool {
    if held || step || knock {
        return true;
    }
    [last_run, last_change]
        .into_iter()
        .flatten()
        .any(|t| (now - t) < chrono::Duration::hours(24))
}

/// The first letter of a name, for its avatar (`Taf.initial`).
pub fn initial(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, TimeZone};

    fn at(h: u32, m: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 11, 15, h, m, 0).unwrap()
    }

    #[test]
    fn relevant_now_is_a_wait_or_the_last_day() {
        let now = at(12, 0);
        assert!(relevant_now(true, false, false, None, None, now));
        assert!(relevant_now(false, true, false, None, None, now));
        assert!(relevant_now(false, false, true, None, None, now));
        assert!(relevant_now(
            false,
            false,
            false,
            Some(now - Duration::hours(23)),
            None,
            now
        ));
        assert!(relevant_now(
            false,
            false,
            false,
            None,
            Some(now - Duration::hours(1)),
            now
        ));
        assert!(!relevant_now(
            false,
            false,
            false,
            Some(now - Duration::hours(24)),
            None,
            now
        ));
        assert!(!relevant_now(false, false, false, None, None, now));
    }

    #[test]
    fn short_time_is_24_hour() {
        assert_eq!(short_time(at(3, 2)), "03:02");
        assert_eq!(initial("bill pay"), "B");
    }
}
