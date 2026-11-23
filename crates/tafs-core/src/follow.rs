//! How a thread is followed by reading it again, when nothing streams (docs/scale.md › The
//! target, in kiff/tafs): the reads are a fallback, so they back off while nothing changes, each
//! wait gets a random ±20% so computers don't line up on a shared clock, and they end a while
//! after the owner last wrote. The app ticks a local timer, which reads nothing; `Follow` says
//! when a tick should read.

use std::time::Duration;

/// A taf's chat: 2.5 s, then 5, 10 and 30 while nothing changes.
pub const CHAT: [Duration; 4] = [
    Duration::from_millis(2_500),
    Duration::from_secs(5),
    Duration::from_secs(10),
    Duration::from_secs(30),
];

/// Coach's thread: 3 s, then 6, 10, 15 and 30 while nothing changes, never 1 s.
pub const COACH: [Duration; 5] = [
    Duration::from_secs(3),
    Duration::from_secs(6),
    Duration::from_secs(10),
    Duration::from_secs(15),
    Duration::from_secs(30),
];

/// How long a taf's chat is followed after the owner last wrote (`chatPollBudget`'s 30 reads of
/// 4 s).
pub const CHAT_WINDOW: Duration = Duration::from_secs(120);

/// How long Coach's thread is followed after the owner last wrote (`setupPollBudget`'s 100 reads
/// of 3 s).
pub const COACH_WINDOW: Duration = Duration::from_secs(300);

/// The local tick the app checks `Follow` on. It reads nothing by itself.
pub const TICK: Duration = Duration::from_millis(500);

/// `d`, made from 80% to 120% of itself by `r`, a random number from 0 to 1.
pub fn jitter(d: Duration, r: f64) -> Duration {
    d.mul_f64(0.8 + 0.4 * r.clamp(0.0, 1.0))
}

/// One thread's reads: when the next is due, and how many in a row changed nothing.
#[derive(Debug, Clone)]
pub struct Follow<T> {
    ladder: &'static [Duration],
    window: Duration,
    /// The owner's last message, by id: the window starts again at each.
    owner: Option<String>,
    started: bool,
    /// Time since the owner last wrote, as the ticks count it.
    since: Duration,
    /// When, on that count, the next read is due.
    due: Duration,
    /// Reads in a row that found the thread as it was.
    quiet: usize,
    seen: Option<T>,
}

impl<T: PartialEq + Clone> Follow<T> {
    pub fn new(ladder: &'static [Duration], window: Duration) -> Self {
        Follow {
            ladder,
            window,
            owner: None,
            started: false,
            since: Duration::ZERO,
            due: Duration::ZERO,
            quiet: 0,
            seen: None,
        }
    }

    fn step(&self) -> Duration {
        self.ladder[self.quiet.min(self.ladder.len().saturating_sub(1))]
    }

    /// `dt` has passed since the last tick, and the owner's last message is `owner`: whether to
    /// read now. A new message of the owner's starts the window again, its first read one step
    /// away; past the window, nothing is read.
    pub fn tick(&mut self, dt: Duration, owner: Option<String>, r: f64) -> bool {
        if !self.started || owner != self.owner {
            self.started = true;
            self.owner = owner;
            self.since = Duration::ZERO;
            self.quiet = 0;
            self.due = jitter(self.step(), r);
            return false;
        }
        self.since += dt;
        self.since <= self.window && self.since >= self.due
    }

    /// The thread as a read found it: the next read is one step further while it's unchanged,
    /// and back at the first step once it moved.
    pub fn read(&mut self, thread: &T, r: f64) {
        if self.seen.as_ref() == Some(thread) {
            self.quiet += 1;
        } else {
            self.quiet = 0;
            self.seen = Some(thread.clone());
        }
        self.due = self.since + jitter(self.step(), r);
    }

    /// The window was seen again after it wasn't: read at the next tick, from the first step.
    pub fn seen_again(&mut self) {
        self.quiet = 0;
        self.due = self.since;
    }

    /// The wait the next read is at now, unjittered (for tests and the docs).
    pub fn step_now(&self) -> Duration {
        self.step()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ticks `secs` seconds at `TICK`, with no jitter (r = 0.5), counting the reads, each finding
    /// `thread`.
    fn run(f: &mut Follow<u32>, owner: &str, secs: u64, thread: u32) -> Vec<f64> {
        let mut at = Duration::ZERO;
        let mut reads = vec![];
        while at < Duration::from_secs(secs) {
            at += TICK;
            if f.tick(TICK, Some(owner.into()), 0.5) {
                reads.push(at.as_secs_f64());
                f.read(&thread, 0.5);
            }
        }
        reads
    }

    #[test]
    fn jitter_is_within_twenty_percent() {
        let d = Duration::from_secs(10);
        assert_eq!(jitter(d, 0.0), Duration::from_secs(8));
        assert_eq!(jitter(d, 1.0), Duration::from_secs(12));
        assert_eq!(jitter(d, 0.5), d);
        assert_eq!(jitter(d, 7.0), Duration::from_secs(12));
    }

    #[test]
    fn a_quiet_chat_backs_off_and_stops_after_its_window() {
        let mut f = Follow::new(&CHAT, CHAT_WINDOW);
        f.tick(TICK, Some("m1".into()), 0.5);
        let reads = run(&mut f, "m1", 600, 7);
        // The first read takes the thread in; then 5, 10 and 30 s apart while nothing changes,
        // and none after two minutes: 7 reads where the old schedule made 30.
        assert_eq!(reads, [2.5, 5.0, 10.0, 20.0, 50.0, 80.0, 110.0]);
    }

    #[test]
    fn a_thread_that_moves_is_read_at_the_first_step_again() {
        let mut f = Follow::new(&COACH, COACH_WINDOW);
        f.tick(TICK, None, 0.5);
        let mut at = Duration::ZERO;
        let mut reads = vec![];
        let mut thread = 0;
        while at < Duration::from_secs(40) {
            at += TICK;
            if f.tick(TICK, None, 0.5) {
                reads.push(at.as_secs_f64());
                // It moves at each of the first three reads, then stays.
                if reads.len() <= 3 {
                    thread += 1;
                }
                f.read(&thread, 0.5);
            }
        }
        assert_eq!(reads, [3.0, 6.0, 9.0, 12.0, 18.0, 28.0]);
    }

    #[test]
    fn coach_never_reads_every_second() {
        assert!(COACH.iter().all(|d| *d >= Duration::from_secs(3)));
        assert!(CHAT.iter().all(|d| *d >= Duration::from_millis(2_500)));
    }

    #[test]
    fn a_new_message_starts_the_window_again() {
        let mut f = Follow::new(&CHAT, CHAT_WINDOW);
        f.tick(TICK, Some("m1".into()), 0.5);
        assert!(!run(&mut f, "m1", 600, 1).is_empty());
        assert!(run(&mut f, "m1", 60, 1).is_empty(), "past its window");
        // The owner writes again: followed again, from the first step.
        assert!(!f.tick(TICK, Some("m2".into()), 0.5));
        assert_eq!(f.step_now(), CHAT[0]);
        assert_eq!(run(&mut f, "m2", 10, 1)[0], 2.5);
    }

    #[test]
    fn seen_again_reads_at_the_next_tick() {
        let mut f = Follow::<u32>::new(&COACH, COACH_WINDOW);
        f.tick(TICK, None, 0.5);
        assert!(!f.tick(TICK, None, 0.5));
        f.seen_again();
        assert!(f.tick(TICK, None, 0.5));
    }

    #[test]
    fn the_jitter_spreads_the_reads() {
        let mut early = Follow::<u32>::new(&CHAT, CHAT_WINDOW);
        let mut late = Follow::<u32>::new(&CHAT, CHAT_WINDOW);
        early.tick(TICK, Some("m".into()), 0.0);
        late.tick(TICK, Some("m".into()), 1.0);
        let (mut a, mut b) = (None, None);
        for n in 1..=10 {
            if a.is_none() && early.tick(TICK, Some("m".into()), 0.0) {
                a = Some(n);
            }
            if b.is_none() && late.tick(TICK, Some("m".into()), 1.0) {
                b = Some(n);
            }
        }
        // 2 s and 3 s: the first read lands at a different tick.
        assert_eq!((a, b), (Some(4), Some(6)));
    }
}
