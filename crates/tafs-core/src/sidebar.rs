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
    /// Account › KIFF account.
    KiffAccount,
    /// Account › Time zone.
    TimeZone,
    /// Account › Alerts.
    Alerts,
    /// Coach mode, from its button at the foot.
    Coach,
    /// A screen Account opens, by its title, until it's ported.
    Titled(String),
    /// A taf, by its key.
    Taf(String),
    /// Halt all: every taf at once, its own screen (`HomeSelection.haltAll`).
    HaltAll,
    /// A taf's halt, by its key (tafs cloud's id), its own screen (`HaltRow`).
    Halt(String),
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
    /// A line under the title, in red: Halt all's state while it isn't running.
    pub line: Option<String>,
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
    /// The account's halt, where it's offered (`showsAccountHalt`): its row, and its state.
    pub account_halt: Option<crate::halt::Halt>,
    /// The tafs shared with this person to use, not theirs to manage (`sharedWithMe`), by key:
    /// KIFF's answer says use without manage; a taf whose answer isn't in yet counts as theirs.
    pub shared: std::collections::BTreeSet<String>,
    /// Agents made through KIFF's API, not run by tafs (`otherAgents`): their Cards, apart.
    pub others: Vec<TafRow>,
    /// tafs cloud's line for the plan, only when it needs the owner, and its tone (the balance
    /// read's `home_line`, `plan_tone`).
    pub plan: Option<(String, String)>,
}

/// The plan's line at the sidebar's foot (`PlanLine`): tafs cloud's words, in red while it
/// waits on the owner, else the amber; then Plan & usage, which it opens.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanLine {
    pub words: String,
    pub tone: Tone,
    pub title: String,
    /// What a screen reader says: the words, then where it goes.
    pub accessible: String,
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
    /// SHARED WITH YOU, with the tafs shared with this person that are relevant now.
    pub shared_label: Option<String>,
    pub shared: Vec<TafRow>,
    /// OTHER AGENTS WITH CARDS, with every one of them.
    pub others_label: Option<String>,
    pub others: Vec<TafRow>,
    /// The plan, only when it needs the owner, signed in with KIFF.
    pub plan: Option<PlanLine>,
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
            line: None,
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
        // Every taf at once, from its own screen (`haltAllRow`): its state in red under it while
        // it isn't running.
        if let Some(h) = &self.account_halt {
            let running = h.state == crate::halt::State::Running;
            let mut halt = item(
                Selection::HaltAll,
                w.line("taf.halt.title_all")?,
                if running {
                    "hand.raised"
                } else {
                    "hand.raised.fill"
                },
            );
            if !running {
                let state = crate::halt::headline(w, h)?;
                halt.accessible = format!("{}, {}", halt.title, state);
                halt.line = Some(state);
            }
            screens.push(halt);
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

        // As one sidebar for the whole app, only the tafs relevant now: all of them are on the
        // Overview. Theirs, then the ones shared with them (`tafSections`); someone with only
        // shared tafs sees just those.
        let relevant = |shared: bool| -> Vec<TafRow> {
            self.tafs
                .iter()
                .filter(|(t, r)| *r && self.shared.contains(&t.key) == shared)
                .map(|(t, _)| t.clone())
                .collect()
        };
        let (relevant, shared) = (relevant(false), relevant(true));
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
            shared_label: (!shared.is_empty())
                .then(|| w.line("home.shared_you").map(|s| upper(&s)))
                .transpose()?,
            shared,
            others_label: (!self.others.is_empty())
                .then(|| w.line("home.other_agents").map(|s| upper(&s)))
                .transpose()?,
            others: self.others.clone(),
            plan: match (&self.plan, self.mode) {
                (Some((words, tone)), Mode::Kiff) => {
                    let title = w.line("usage.title")?;
                    Some(PlanLine {
                        accessible: format!(
                            "{words}. {}",
                            w.get("home.opens_plan", &[("title", &title)])?
                        ),
                        words: words.clone(),
                        // `UsageView.color(planTone, calm: amberText)`.
                        tone: if tone == "waiting" {
                            Tone::No
                        } else {
                            Tone::Amber
                        },
                        title,
                    })
                }
                _ => None,
            },
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
        line: None,
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

/// The tafs as listed (`listedTafs`, `byRecent`): a taf waiting on the owner first, then the
/// one used or at work most recently (`latest`, its newest Record entry), so the taf the owner
/// just worked with is at the top. Tafs with nothing yet keep tafs cloud's order, after the rest.
/// The iPad settles this when the list loads and when the app comes back; this computer reads
/// the account afresh each time, so each read settles it.
pub fn by_recent<T>(
    tafs: &[T],
    waiting: impl Fn(&T) -> bool,
    latest: impl Fn(&T) -> Option<chrono::DateTime<chrono::Utc>>,
) -> Vec<&T> {
    let mut listed: Vec<(usize, &T)> = tafs.iter().enumerate().collect();
    // A stable sort, as the iPad's tie on tafs cloud's order (`a.offset < b.offset`).
    listed.sort_by(|(_, a), (_, b)| {
        waiting(b)
            .cmp(&waiting(a))
            .then_with(|| latest(b).cmp(&latest(a)))
    });
    listed.into_iter().map(|(_, t)| t).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(key: &str) -> TafRow {
        TafRow {
            key: key.into(),
            name: key.to_uppercase(),
            initial: initial(key),
            color_index: 0,
            now: "Working".into(),
            badge: None,
            tone: Tone::default(),
        }
    }

    fn signed_in(tafs: Vec<(TafRow, bool)>) -> SidebarModel {
        SidebarModel {
            mode: Mode::Kiff,
            screenshot_signed_in: false,
            may_use_coach: false,
            held: vec![],
            tafs,
            list_loaded: true,
            home_words: None,
            account_name: None,
            cloud_needs_you: None,
            account_halt: None,
            shared: Default::default(),
            others: vec![],
            plan: None,
        }
    }

    #[test]
    fn shared_tafs_and_other_agents_have_their_own_sections() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let mut m = signed_in(vec![(row("a"), true), (row("b"), true), (row("c"), false)]);
        m.shared = ["b".to_string(), "c".to_string()].into();
        m.others = vec![row("x")];
        let l = m.lines(&w).unwrap();
        let keys = |rows: &[TafRow]| rows.iter().map(|r| r.key.clone()).collect::<Vec<_>>();
        // Theirs, then the shared ones relevant now (c isn't); every other agent.
        assert_eq!(keys(&l.your_tafs), ["a"]);
        assert_eq!(l.shared_label.as_deref(), Some("SHARED WITH YOU"));
        assert_eq!(keys(&l.shared), ["b"]);
        assert_eq!(l.others_label.as_deref(), Some("OTHER AGENTS WITH CARDS"));
        assert_eq!(keys(&l.others), ["x"]);
        // Someone with only shared tafs sees just those: no Your tafs.
        let mut only = signed_in(vec![(row("b"), true)]);
        only.shared = ["b".to_string()].into();
        let l = only.lines(&w).unwrap();
        assert_eq!(l.your_tafs_label, None);
        assert_eq!(keys(&l.shared), ["b"]);
        assert_eq!(l.others_label, None);
        // None shared and no other agents: neither section.
        let l = signed_in(vec![(row("a"), true)]).lines(&w).unwrap();
        assert_eq!((l.shared_label, l.others_label), (None, None));
    }

    #[test]
    fn the_plan_line_shows_only_when_tafs_cloud_sends_one() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let mut m = signed_in(vec![]);
        assert_eq!(m.lines(&w).unwrap().plan, None);
        m.plan = Some(("Your trial ends tomorrow".into(), "warning".into()));
        let p = m.lines(&w).unwrap().plan.unwrap();
        assert_eq!(p.words, "Your trial ends tomorrow");
        assert_eq!(p.tone, Tone::Amber);
        assert_eq!(p.title, "Plan & usage");
        assert_eq!(
            p.accessible,
            "Your trial ends tomorrow. Opens Plan & usage."
        );
        // Waiting on the owner: red. Any other tone: the amber.
        m.plan = Some(("Tasks paused".into(), "waiting".into()));
        assert_eq!(m.lines(&w).unwrap().plan.unwrap().tone, Tone::No);
        m.plan = Some(("x".into(), "ok".into()));
        assert_eq!(m.lines(&w).unwrap().plan.unwrap().tone, Tone::Amber);
        // The sample has no plan line.
        m.mode = Mode::Sample;
        assert_eq!(m.lines(&w).unwrap().plan, None);
    }

    #[test]
    fn the_list_is_waiting_then_most_recent_then_tafs_cloud_s_order() {
        use chrono::TimeZone;
        let at = |h| Some(chrono::Utc.with_ymd_and_hms(2026, 11, 16, h, 0, 0).unwrap());
        // (key, waiting, latest)
        let tafs = [
            ("a", false, None),
            ("b", false, at(3)),
            ("c", true, at(1)),
            ("d", false, at(5)),
            ("e", false, None),
            ("f", true, None),
        ];
        let listed: Vec<&str> = by_recent(&tafs, |t| t.1, |t| t.2)
            .iter()
            .map(|t| t.0)
            .collect();
        assert_eq!(listed, ["c", "f", "d", "b", "a", "e"]);
    }
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
