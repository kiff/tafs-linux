//! Account › Alerts (`NotificationSettingsView`): what an alert looks like, alerts on this
//! computer and their sound, and the line under them. The iPad's "What alerts you" is a phone's
//! push choices, read with its push token: a computer has none, so the iPad draws none of it
//! there either. Haptics are a phone's.

use crate::words::{WordBook, WordsError};

/// An alert as it shows (`LockScreenAlert`): its title and line, with sound or silent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Example {
    pub title: String,
    pub line: String,
    pub loud: bool,
    /// What a screen reader says: "With sound: title. line".
    pub spoken: String,
}

/// The screen's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AlertsLines {
    pub title: String,
    pub examples: Vec<Example>,
    /// "Alerts on this computer".
    pub on_this: String,
    pub sound: String,
    pub sound_words: String,
    /// Under the two: who alerts go to, or what the switch does.
    pub foot: String,
}

/// The lines, for someone who may answer held calls or not (`canAnswerHolds`; unknown reads as
/// may, as the iPad's `== false`).
pub fn lines(w: &WordBook, can_answer_holds: Option<bool>) -> Result<AlertsLines, WordsError> {
    let device = w.line("device.linux")?;
    let example = |title: &str, line: &str, loud: bool| -> Result<Example, WordsError> {
        let (title, line) = (w.line(title)?, w.line(line)?);
        let how = w.line(if loud {
            "alert.with_sound"
        } else {
            "alert.silent"
        })?;
        Ok(Example {
            spoken: format!("{how}: {title}. {line}"),
            title,
            line,
            loud,
        })
    };
    Ok(AlertsLines {
        title: w.line("settings.alerts")?,
        examples: vec![
            example(
                "settings.chargebacks_needs_you",
                "settings.dispute_over_limit",
                true,
            )?,
            example("settings.refunds_answered_maria", "alert.by_email", false)?,
        ],
        on_this: w.get("alert.on_this_device", &[("device", &device)])?,
        sound: w.line("alert.sound")?,
        sound_words: w.line("alert.sound_words")?,
        // The iPad's "…tells you, even with tafs closed" is a phone's push: this computer reads
        // held calls only while tafs runs, so it says what the switch does instead.
        foot: if can_answer_holds == Some(false) {
            w.line("alert.foot.cant_answer_holds")?
        } else {
            w.line("alert.prompt")?
        },
    })
}

/// Whether a fresh hold is announced (`phoneAlerts.enabled`), and with sound: no alerts, no
/// sound, its choice kept for when they're back.
pub fn announce(alerts_on: bool, sound: bool) -> Option<bool> {
    alerts_on.then_some(sound)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    #[test]
    fn the_screen_says_what_this_computer_does() {
        let w = words();
        let l = lines(&w, None).unwrap();
        assert_eq!(l.title, "Alerts");
        assert_eq!(l.on_this, "Alerts on this computer");
        assert_eq!(l.sound_words, "For a hold or a halt only");
        // Never "even with tafs closed": the computer reads only while tafs runs.
        assert_eq!(l.foot, "Get an alert when a taf needs you");
        assert_eq!(l.examples.len(), 2);
        assert!(l.examples[0].loud && !l.examples[1].loud);
        assert_eq!(l.examples[1].title, "Refunds answered maria@…");
        assert_eq!(
            l.examples[1].spoken,
            "Silent: Refunds answered maria@…. By email"
        );
        assert_eq!(lines(&w, Some(true)).unwrap().foot, l.foot);
    }

    #[test]
    fn a_role_that_cant_answer_is_told_so() {
        let l = lines(&words(), Some(false)).unwrap();
        assert!(l.foot.contains("Your role on this account can't"));
    }

    #[test]
    fn no_alerts_no_sound() {
        assert_eq!(announce(true, true), Some(true));
        assert_eq!(announce(true, false), Some(false));
        assert_eq!(announce(false, true), None);
    }
}
