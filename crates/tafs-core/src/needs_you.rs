//! Needs you (`NeedsYouView`) and each held call's card (`HeldCallCard`), as far as this app
//! draws them: the calls KIFF holds and the sample's.

use chrono::{DateTime, Local};

use crate::held::{remaining, HeldCall};
use crate::words::{WordBook, WordsError};

/// KIFF's web app, where a held call can be answered today (`KIFFClient.webURL`).
pub const KIFF_WEB: &str = "https://app.kiff.dev";

/// `answerPage`: the held call's page in KIFF, in the app's language.
pub fn answer_page(kiff_id: &str, language: &str) -> String {
    let mut url = url::Url::parse(KIFF_WEB).expect("KIFF's web app");
    url.path_segments_mut()
        .expect("a base")
        .push("needs-you")
        .push(kiff_id);
    url.query_pairs_mut().append_pair("lang", language);
    url.to_string()
}

/// What a card offers under its face.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Answers {
    /// It lapsed: "Expired. Nobody answered in time…".
    Expired(String),
    /// Someone else on the account answers it (`mayAnswerHolds` false).
    SomeoneAnswers { nothing_sent: String, line: String },
    /// A call from KIFF, while this computer doesn't sign answers: answer it in KIFF.
    InKiff {
        nothing_sent: String,
        line: String,
        button: String,
        page: String,
    },
    /// The sample's, answered here: Approve once (when it can be), its next steps, Refuse.
    Here {
        nothing_sent: String,
        approve: Option<String>,
        over_total: Option<String>,
        next_steps: Vec<String>,
        refuse: String,
    },
}

/// One card's lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLines {
    pub id: String,
    /// The taf's name, else "A taf".
    pub holder: String,
    /// Its avatar, when the taf is listed: its initial and colour.
    pub avatar: Option<(String, i64)>,
    /// "9:37 left", or "expired", counted down to `expires_at`.
    pub remaining: String,
    pub expires_at: DateTime<Local>,
    pub amount: Option<String>,
    pub summary: String,
    pub context: String,
    pub reason: String,
    /// "The taf says", with its lines, when it said anything.
    pub taf_says: Option<(String, Vec<String>)>,
    /// "Details", with its lines.
    pub details: Option<(String, Vec<String>)>,
    pub answers: Answers,
    /// What a screen reader says for the card's face (`sentence`).
    pub sentence: String,
}

/// The screen's lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NeedsYouLines {
    pub title: String,
    pub cards: Vec<CardLines>,
    /// Steps the tafs handed the person (`yourSteps`), from Home.
    pub steps: Vec<crate::waits::YourStep>,
    /// Coach's chats waiting on the person (`coachWaiting`), the usable ones.
    pub coach: Vec<crate::waits::CoachWaiting>,
    /// "Set all aside" above two or more of them (`setAllAside`).
    pub set_all_aside: Option<crate::waits::CoachWaitingClear>,
    /// Coach's way in, for someone who may use Coach (`mayUseCoach`): else the line alone.
    pub opens_coach: bool,
    /// "Did this happen?": money calls KIFF can't prove, after Coach's waits (`unproven`).
    pub happened: Vec<crate::unproven::HappenedLines>,
    /// With nothing waiting: "Nothing is waiting." and its hint.
    pub empty: Option<(String, String)>,
}

/// A taf as the cards name it: its key, name, initial and colour.
pub struct Holder<'a> {
    pub key: &'a str,
    pub name: &'a str,
    pub initial: &'a str,
    pub color: i64,
}

pub fn lines(
    w: &WordBook,
    held: &[HeldCall],
    tafs: &[Holder],
    may_answer: bool,
    now: DateTime<Local>,
) -> Result<NeedsYouLines, WordsError> {
    let mut cards = Vec::new();
    for call in held {
        let taf = tafs.iter().find(|t| t.key == call.taf);
        let holder = match taf {
            Some(t) => t.name.to_string(),
            None => w.line("needs_you.a_taf")?,
        };
        let amount = match &call.amount {
            Some(a) => Some(a.formatted(w)?),
            None => None,
        };
        let nothing_sent = w.line("needs_you.nothing_sent")?;
        let answers = if now >= call.expires_at {
            Answers::Expired(w.line("needs_you.expired_line")?)
        } else if !may_answer {
            Answers::SomeoneAnswers {
                nothing_sent,
                line: w.line("needs_you.someone_answers")?,
            }
        } else if let Some(id) = &call.kiff_id {
            Answers::InKiff {
                nothing_sent,
                line: w.line("needs_you.answer_in_kiff_until")?,
                button: w.line("needs_you.answer_in_kiff_button")?,
                page: answer_page(id, w.language()),
            }
        } else {
            Answers::Here {
                nothing_sent,
                approve: call
                    .approvable_once
                    .then(|| w.line("needs_you.approve_once"))
                    .transpose()?,
                over_total: (!call.approvable_once)
                    .then(|| w.line("needs_you.over_total"))
                    .transpose()?,
                next_steps: call.next_steps.clone(),
                refuse: w.line("needs_you.refuse")?,
            }
        };
        let says: Vec<String> = [call.taf_about.clone(), call.taf_reason.clone()]
            .into_iter()
            .flatten()
            .collect();
        let what = [Some(call.summary.clone()), amount.clone()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(", ");
        cards.push(CardLines {
            id: call.id.clone(),
            avatar: taf.map(|t| (t.initial.to_string(), t.color)),
            remaining: remaining(w, call.expires_at, now)?,
            expires_at: call.expires_at,
            sentence: w.get(
                "needs_you.asks_lapses",
                &[
                    ("taf", &holder),
                    ("what", &what),
                    ("reason", &call.reason),
                    ("time", &crate::account_time::short_time(&call.expires_at)),
                    ("nothing_sent", &w.line("needs_you.nothing_sent")?),
                ],
            )?,
            holder,
            amount,
            summary: call.summary.clone(),
            context: call.context.clone(),
            reason: call.reason.clone(),
            taf_says: (!says.is_empty())
                .then(|| w.line("needs_you.taf_says").map(|t| (t, says)))
                .transpose()?,
            details: (!call.details.is_empty())
                .then(|| w.line("common.details").map(|t| (t, call.details.clone())))
                .transpose()?,
            answers,
        });
    }
    Ok(NeedsYouLines {
        title: w.line("needs_you.title")?,
        steps: Vec::new(),
        coach: Vec::new(),
        set_all_aside: None,
        opens_coach: false,
        happened: vec![],
        empty: held
            .is_empty()
            .then(|| -> Result<_, WordsError> {
                Ok((
                    w.line("needs_you.nothing_waiting")?,
                    w.line("needs_you.empty_hint")?,
                ))
            })
            .transpose()?,
        cards,
    })
}

/// Home's steps and Coach's waits beside the held calls: Coach's only when usable, "Set all aside"
/// only with two or more; "Nothing is waiting." only when none of them waits either.
pub fn with_waits(
    mut lines: NeedsYouLines,
    home: &crate::home::Home,
    may_use_coach: bool,
) -> NeedsYouLines {
    lines.opens_coach = may_use_coach;
    lines.steps = home.your_steps.clone().unwrap_or_default();
    lines.coach = home
        .coach_waiting
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.usable())
        .collect();
    lines.set_all_aside = home
        .coach_waiting_clear
        .clone()
        .filter(|c| c.usable() && lines.coach.len() > 1);
    if !lines.steps.is_empty() || !lines.coach.is_empty() {
        lines.empty = None;
    }
    lines
}

/// KIFF's "Did this happen?" calls beside the rest (`NeedsYouView`): after Coach's waits, before
/// the held calls. Read only for someone who may answer holds (`loadUnproven`); "Nothing is
/// waiting." only when none of them waits either.
pub fn with_unproven(
    mut lines: NeedsYouLines,
    w: &WordBook,
    calls: &[crate::unproven::Unproven],
) -> Result<NeedsYouLines, WordsError> {
    lines.happened = crate::unproven::lines(w, calls)?;
    if !lines.happened.is_empty() {
        lines.empty = None;
    }
    Ok(lines)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_answer_page_is_kiffs() {
        assert_eq!(
            answer_page("exc_1", "fr"),
            "https://app.kiff.dev/needs-you/exc_1?lang=fr"
        );
        assert_eq!(
            answer_page("a/b", "en"),
            "https://app.kiff.dev/needs-you/a%2Fb?lang=en"
        );
    }
}
