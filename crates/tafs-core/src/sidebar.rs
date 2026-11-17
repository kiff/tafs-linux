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
    /// Account › Language.
    Language,
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

/// Discard on a taf being set up (`DiscardSettingUpButton`): tafs cloud's words, and the KIFF agent
/// it's for, which names the route (`discardPath`). One whose agent is a placeholder
/// (`setting-up:`) or none is drawn, and its yes does nothing, as on the iPad.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Discard {
    pub agent: String,
    pub words: crate::home::DiscardWords,
}

impl Discard {
    /// The signed act its yes would be, when it has a real agent.
    pub fn act(&self) -> Option<crate::signed::Act> {
        (!self.agent.is_empty() && !self.agent.starts_with("setting-up:")).then(|| {
            crate::signed::Act::DiscardSettingUp {
                agent: self.agent.clone(),
            }
        })
    }
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
    /// The tafs being set up (`settingUp`), among Your tafs: each row, and Discard for the person
    /// whose setup it was.
    pub setting_up: Vec<(TafRow, Option<Discard>)>,
    /// tafs cloud's line for the plan, only when it needs the owner, and its tone (the balance
    /// read's `home_line`, `plan_tone`).
    pub plan: Option<(String, String)>,
    /// tafs cloud's name for Needs you's row once there's a brief ("Your brief",
    /// `needsYouTabTitle`); none, Needs you.
    pub brief_tab: Option<String>,
    /// The chat each row whose setup stopped partway opens in Coach (`finishSetup`), by the row's
    /// key: only those `canFinishSetup` allows.
    pub finish: std::collections::BTreeMap<String, String>,
    /// What the last read of Home couldn't do (`unreachable`), and whether the app tries again by
    /// itself (`retryingRefresh`); none once a read goes through.
    pub unreachable: Option<(Unreachable, bool)>,
}

/// Why Home couldn't be read (`refreshWords`): the network, or KIFF or tafs cloud answering with
/// an error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unreachable {
    Network,
    Error,
}

/// The line at the sidebar's foot while KIFF can't be reached (`UnreachableLine`): an amber dot,
/// the words (with "trying again" while the app tries by itself), and Status, which opens KIFF's
/// status page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnreachableLine {
    pub words: String,
    pub status: String,
    pub url: &'static str,
}

/// KIFF's status page, served apart from KIFF, so it answers when KIFF doesn't (`Legal.status`).
pub const STATUS_URL: &str = "https://status.kiff.dev";

/// `UnreachableLine`'s words for what failed: with "trying again" while the app tries again by
/// itself (`statusRetrying`).
pub fn unreachable_line(
    w: &WordBook,
    why: Unreachable,
    retrying: bool,
) -> Result<UnreachableLine, WordsError> {
    let words = w.line(match why {
        Unreachable::Network => "home.cant_reach_kiff",
        Unreachable::Error => "home.couldnt_load_from_kiff",
    })?;
    Ok(UnreachableLine {
        words: if retrying {
            w.get("home.status_retrying", &[("words", &words)])?
        } else {
            words
        },
        status: w.line("home.status")?,
        url: STATUS_URL,
    })
}

/// The waits between the app's own tries after a read that failed, in seconds, then no more until
/// the person comes back (`retryWaits`).
pub const RETRY_WAITS: [u64; 5] = [2, 10, 30, 60, 120];

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
    /// The tafs being set up, after them: no page to open, and Discard when it's offered.
    pub setting_up: Vec<(TafRow, Option<Discard>)>,
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
    /// + New ("Start a new taf"), signed in with KIFF (`canStartTafs`).
    pub start_new: Option<String>,
    /// The chat each row whose setup stopped partway opens in Coach, by the row's key.
    pub finish: std::collections::BTreeMap<String, String>,
    /// KIFF can't be reached just now; nothing when all is well.
    pub unreachable: Option<UnreachableLine>,
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
            title: self.brief_tab.clone().unwrap_or_else(|| needs_you.clone()),
            symbol: "bell",
            badge: (count > 0).then(|| count.to_string()),
            accessible: match (&self.brief_tab, count > 0) {
                (None, true) => w.get("needs_you.tab_label", &[("n", &count.to_string())])?,
                (None, false) => needs_you,
                (Some(tab), true) => format!("{tab}, {count}"),
                (Some(tab), false) => tab.clone(),
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
        let shows = !relevant.is_empty() || !self.setting_up.is_empty() || self.tafs.is_empty();
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
            setting_up: if shows {
                self.setting_up.clone()
            } else {
                vec![]
            },
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
            start_new: (self.mode == Mode::Kiff)
                .then(|| w.line("home.start_new_taf"))
                .transpose()?,
            finish: self.finish.clone(),
            unreachable: self
                .unreachable
                .map(|(why, retrying)| unreachable_line(w, why, retrying))
                .transpose()?,
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
            setting_up: vec![],
            plan: None,
            brief_tab: None,
            finish: Default::default(),
            unreachable: None,
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

    /// Needs you's row says tafs cloud's name for the brief once there is one
    /// (`needsYouTabTitle`), with its count; Needs you otherwise.
    #[test]
    fn needs_you_s_row_is_the_briefs_once_there_is_one() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let mut m = signed_in(vec![]);
        m.cloud_needs_you = Some(2);
        let first = |m: &SidebarModel| m.lines(&w).unwrap().screens[0].clone();
        assert_eq!(first(&m).title, w.line("needs_you.title").unwrap());
        m.brief_tab = Some("Your brief".into());
        let item = first(&m);
        assert_eq!(item.selection, Selection::NeedsYou);
        assert_eq!(item.title, "Your brief");
        assert_eq!(item.badge.as_deref(), Some("2"));
        assert_eq!(item.accessible, "Your brief, 2");
        m.cloud_needs_you = Some(0);
        assert_eq!(first(&m).accessible, "Your brief");
    }

    #[test]
    fn tafs_being_set_up_show_your_tafs_after_its_rows() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        // None relevant now, but one being set up: Your tafs shows, with it.
        let mut m = signed_in(vec![(row("a"), false)]);
        m.setting_up = vec![(
            row("s"),
            Some(Discard {
                agent: "agt_s".into(),
                words: crate::home::DiscardWords {
                    button: "Discard".into(),
                    yes: "Discard".into(),
                    ..Default::default()
                },
            }),
        )];
        let l = m.lines(&w).unwrap();
        assert_eq!(l.your_tafs_label.as_deref(), Some("YOUR TAFS"));
        assert!(l.your_tafs.is_empty());
        assert_eq!(l.setting_up.len(), 1);
        // Only shared tafs and none being set up: no Your tafs, and none drawn.
        let mut only = signed_in(vec![(row("b"), true)]);
        only.shared = ["b".to_string()].into();
        assert!(only.lines(&w).unwrap().setting_up.is_empty());
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

    /// New taf (`canStartTafs`) signed in with KIFF only, and each row's chat to finish in
    /// Coach as the app allows it.
    #[test]
    fn new_and_finish_setting_up_are_signed_in_only() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let mut m = signed_in(vec![]);
        m.finish = [("agent:s".to_string(), "chat_1".to_string())].into();
        let l = m.lines(&w).unwrap();
        assert_eq!(l.start_new.as_deref(), Some("Start a new taf"));
        assert_eq!(l.finish.get("agent:s").map(String::as_str), Some("chat_1"));
        m.mode = Mode::Sample;
        assert_eq!(m.lines(&w).unwrap().start_new, None);
    }

    /// `UnreachableLine`: what failed, "trying again" while the app tries by itself, and Status.
    #[test]
    fn the_unreachable_line_says_what_failed() {
        let w = WordBook::load(&WordBook::default_root(), "en").unwrap();
        let mut m = signed_in(vec![]);
        assert_eq!(m.lines(&w).unwrap().unreachable, None);
        m.unreachable = Some((Unreachable::Network, true));
        let u = m.lines(&w).unwrap().unreachable.unwrap();
        assert_eq!(u.words, "Can't reach KIFF · trying again");
        assert_eq!(
            (u.status.as_str(), u.url),
            ("Status", "https://status.kiff.dev")
        );
        m.unreachable = Some((Unreachable::Error, false));
        assert_eq!(
            m.lines(&w).unwrap().unreachable.unwrap().words,
            "Couldn't load from KIFF"
        );
    }
}
