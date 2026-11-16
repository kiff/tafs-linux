//! Whether this device can approve (`Biometry.swift` › `ApprovalReadiness`) and what's said in an
//! approval's place while it can't (`ApprovalSetupView.swift` › `ApprovalGate`, `ApprovalNotice`,
//! `ApprovalNotReady`). The iPad's approval key signs only after Face ID or Touch ID; this
//! computer has neither, so it is always `Unsupported`: its words say so, and there is nothing
//! to open or check again.

use crate::sidebar::Mode;
use crate::words::{WordBook, WordsError};

/// Whether this device can approve, each state with its own words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Readiness {
    Ready,
    /// No face or fingerprint enrolled: the person adds one.
    NoneEnrolled,
    /// No passcode on the device, which Face ID and Touch ID need first.
    NoPasscode,
    /// Face ID or Touch ID is there but turned off for tafs: its own settings turn it on.
    OffForTafs,
    /// Working but not right now: locked out after tries, or not connected.
    Unavailable {
        locked_out: bool,
    },
    /// No Face ID or Touch ID on this device: approvals need another one.
    Unsupported,
}

/// This computer: no Face ID or Touch ID, so it can never approve. Read again changes nothing,
/// so there is no Check again.
pub const THIS_COMPUTER: Readiness = Readiness::Unsupported;

impl Readiness {
    pub fn is_ready(self) -> bool {
        self == Readiness::Ready
    }

    /// The setup screen's words for the state: `device` the device's kind, `kind` "Face ID" or
    /// "Touch ID" (none when the device doesn't say).
    pub fn words(
        self,
        w: &WordBook,
        device: &str,
        kind: Option<&str>,
    ) -> Result<String, WordsError> {
        let either = either(w, kind)?;
        let path = path(w, kind)?;
        match self {
            Readiness::Ready => w.get(
                "approval.ready_approve",
                &[("device", device), ("either", &either)],
            ),
            Readiness::NoneEnrolled => w.get(
                "approval.approving_asks_none_set_up",
                &[("either", &either), ("device", device), ("kind", &path)],
            ),
            Readiness::NoPasscode => w.get(
                "approval.has_no_passcode_needs_one",
                &[
                    ("device", device),
                    ("either", &either),
                    ("kind", &path),
                    ("either_2", &either),
                ],
            ),
            Readiness::OffForTafs => w.get(
                "approval.turned_off_tafs_open_settings",
                &[("either", &either), ("either_2", &either)],
            ),
            Readiness::Unavailable { locked_out: true } => w.get(
                "approval.locked_after_too_many_tries",
                &[("either", &either), ("device", device)],
            ),
            Readiness::Unavailable { locked_out: false } => {
                w.get("approval.can_not_used_right_try", &[("either", &either)])
            }
            Readiness::Unsupported => w.get("approval.has_no_face_id_touch", &[("device", device)]),
        }
    }

    /// A line for an approval's place when it can't be made here.
    pub fn short(self, w: &WordBook, kind: Option<&str>) -> Result<String, WordsError> {
        let either = either(w, kind)?;
        match self {
            Readiness::Ready => Ok(String::new()),
            Readiness::NoneEnrolled => w.get(
                "approval.approving_needs_none_set_up",
                &[("either", &either), ("kind", &path(w, kind)?)],
            ),
            Readiness::NoPasscode => w.get(
                "approval.approving_needs_passcode_set_passcode",
                &[
                    ("either", &either),
                    ("kind", &path(w, kind)?),
                    ("either_2", &either),
                ],
            ),
            Readiness::OffForTafs => {
                w.get("approval.turned_off_tafs_turn_tafs", &[("either", &either)])
            }
            Readiness::Unavailable { locked_out: true } => {
                w.get("approval.locked_unlock_device", &[("either", &either)])
            }
            Readiness::Unavailable { locked_out: false } => {
                w.get("approval.can_not_used_right_try", &[("either", &either)])
            }
            Readiness::Unsupported => w.line("approval.device_can_not_approve_approve"),
        }
    }

    /// Open Settings helps only when it's turned off for tafs: adding a face, a fingerprint or
    /// a passcode isn't in tafs' own settings, so those states give the path in words.
    pub fn opens_settings(self) -> bool {
        self == Readiness::OffForTafs
    }

    /// Check again, unless the device can never approve.
    pub fn checks_again(self) -> bool {
        self != Readiness::Unsupported
    }

    /// Whether to say that a new face or fingerprint ends the approval key.
    pub fn notes_new_biometrics(self) -> bool {
        matches!(self, Readiness::NoneEnrolled | Readiness::NoPasscode)
    }
}

fn either(w: &WordBook, kind: Option<&str>) -> Result<String, WordsError> {
    match kind {
        Some(k) => Ok(k.to_string()),
        None => w.line("approval.face_id_touch_id"),
    }
}

/// Where the person sets it up, in words (`ApprovalReadiness.path`).
pub fn path(w: &WordBook, kind: Option<&str>) -> Result<String, WordsError> {
    w.line(match kind {
        Some("Touch ID") => "approval.settings_path.touch_id",
        Some("Face ID") => "approval.settings_path.face_id",
        _ => "approval.settings_face_id_passcode_touch",
    })
}

/// `ApprovalNotReady`: why this device can't approve, and what to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotReady {
    /// `short`, 14 in ink.
    pub line: String,
    /// The new face or fingerprint note, 12 dim.
    pub note: Option<String>,
    /// Open Settings, where tafs' settings help.
    pub open_settings: Option<String>,
    /// Check again, unless the device can never approve.
    pub check_again: Option<String>,
}

pub fn not_ready(w: &WordBook, r: Readiness, kind: Option<&str>) -> Result<NotReady, WordsError> {
    Ok(NotReady {
        line: r.short(w, kind)?,
        note: if r.notes_new_biometrics() {
            Some(w.line("approval.adding_face_fingerprint_later_ends")?)
        } else {
            None
        },
        open_settings: if r.opens_settings() {
            Some(w.line("voice.open_settings")?)
        } else {
            None
        },
        check_again: if r.checks_again() {
            Some(w.line("approval.check_again")?)
        } else {
            None
        },
    })
}

/// What stands where an approval's button would (`ApprovalGate`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Gate {
    /// The button. In the sample nothing is signed for real, so it always shows.
    Button,
    /// The device can't approve: why, and what to do, in the button's place.
    NotReady(NotReady),
}

pub fn gate(w: &WordBook, mode: Mode, r: Readiness) -> Result<Gate, WordsError> {
    if mode != Mode::Kiff || r.is_ready() {
        Ok(Gate::Button)
    } else {
        Ok(Gate::NotReady(not_ready(w, r, None)?))
    }
}

/// `ApprovalNotice`: the same words for an approval whose button stays where it is, disabled
/// (`approval_blocked`); nothing while the device can approve, or in the sample.
pub fn notice(w: &WordBook, mode: Mode, r: Readiness) -> Result<Option<NotReady>, WordsError> {
    match gate(w, mode, r)? {
        Gate::Button => Ok(None),
        Gate::NotReady(n) => Ok(Some(n)),
    }
}

/// `approvalBlocked`: this device can't approve now, so an approval's button is disabled. The
/// sample signs nothing for real, so nothing is blocked there.
pub fn approval_blocked(mode: Mode, r: Readiness) -> bool {
    mode == Mode::Kiff && !r.is_ready()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(lang: &str) -> WordBook {
        WordBook::load(&WordBook::default_root(), lang).unwrap()
    }

    const ALL: [Readiness; 7] = [
        Readiness::Ready,
        Readiness::NoneEnrolled,
        Readiness::NoPasscode,
        Readiness::OffForTafs,
        Readiness::Unavailable { locked_out: true },
        Readiness::Unavailable { locked_out: false },
        Readiness::Unsupported,
    ];

    #[test]
    fn this_computer_can_never_approve() {
        let w = words("en");
        assert!(!THIS_COMPUTER.is_ready());
        assert_eq!(
            not_ready(&w, THIS_COMPUTER, None).unwrap(),
            NotReady {
                line: "This device can't approve. Approve on another device signed in to your account."
                    .into(),
                note: None,
                open_settings: None,
                check_again: None,
            }
        );
        assert!(approval_blocked(Mode::Kiff, THIS_COMPUTER));
    }

    #[test]
    fn the_sample_shows_the_button() {
        let w = words("en");
        for mode in [Mode::Sample, Mode::SignedOut] {
            assert_eq!(gate(&w, mode, THIS_COMPUTER).unwrap(), Gate::Button);
            assert_eq!(notice(&w, mode, THIS_COMPUTER).unwrap(), None);
            assert!(!approval_blocked(mode, THIS_COMPUTER));
        }
        assert_eq!(
            gate(&w, Mode::Kiff, Readiness::Ready).unwrap(),
            Gate::Button
        );
        assert!(matches!(
            gate(&w, Mode::Kiff, THIS_COMPUTER).unwrap(),
            Gate::NotReady(_)
        ));
    }

    #[test]
    fn each_state_has_its_own_words() {
        let w = words("en");
        let n = |r| not_ready(&w, r, Some("Face ID")).unwrap();
        assert_eq!(
            n(Readiness::NoneEnrolled).line,
            "Approving needs Face ID, and none is set up. Set it up in Settings → Face ID & Passcode."
        );
        assert!(n(Readiness::NoneEnrolled).note.is_some());
        assert!(n(Readiness::NoPasscode).note.is_some());
        assert_eq!(
            n(Readiness::OffForTafs).open_settings.as_deref(),
            Some("Open Settings")
        );
        assert_eq!(
            n(Readiness::OffForTafs).check_again.as_deref(),
            Some("Check again")
        );
        assert_eq!(n(Readiness::NoneEnrolled).open_settings, None);
        assert_eq!(
            n(Readiness::Unavailable { locked_out: true }).line,
            "Face ID is locked after too many tries. Unlock this device with its passcode, then try again."
        );
        assert_eq!(
            Readiness::Unsupported.words(&w, "iPad", None).unwrap(),
            "This iPad has no Face ID or Touch ID, so it can't approve. Approve on another iPhone, iPad or Android phone signed in to your account."
        );
        // Not saying which: both, and both paths.
        assert_eq!(
            Readiness::NoneEnrolled.short(&w, None).unwrap(),
            "Approving needs Face ID or Touch ID, and none is set up. Set it up in Settings → Face ID & Passcode (Touch ID & Passcode on a device with Touch ID)."
        );
    }

    #[test]
    fn every_state_reads_in_every_language() {
        for lang in crate::words::SHIPPED {
            let w = words(lang);
            for r in ALL {
                for kind in [None, Some("Face ID"), Some("Touch ID")] {
                    r.words(&w, "iPad", kind).unwrap();
                    not_ready(&w, r, kind).unwrap();
                }
            }
        }
    }
}
