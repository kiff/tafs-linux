//! What Coach's conversation has made so far (`SetupShape`, `SetupShapePanel`): beside the
//! conversation on a wide window, the taf's name, the plan, and the Card KIFF drafted. Read only
//! from what the app already holds: the thread and KIFF's drafts. Nothing here asks a server for
//! more. The tools it connects come from planque's grants, which this computer doesn't read, so
//! that part is never drawn here.

use crate::coach::{Item, Kind, SetupThread, ShownComponent};
use crate::coach_lines::{card_state, CardState, PlanStep};
use crate::coach_send::Conversation;
use crate::words::{WordBook, WordsError};

/// The panel's width beside the conversation (`frame(width: 420)`).
pub const PANEL_WIDTH: i32 = 420;

/// The narrowest the conversation and the panel are drawn side by side (`width >= 780`).
pub const SIDE_BY_SIDE: i32 = 780;

/// `SetupShape.Step.State`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepState {
    Done,
    NeedsYou,
    Now,
    ToDo,
    Failed,
    Skipped,
}

/// `SetupShape.Step`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    pub label: String,
    pub state: StepState,
}

/// `SetupShape.Card`: the taf's Cards as KIFF drafted them, a draft until confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeCard {
    pub title: String,
    pub lines: Vec<String>,
    pub confirmed: bool,
}

/// `SetupShape`, without the tools.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SetupShape {
    /// The taf's name, once Coach hands it over.
    pub name: Option<String>,
    /// The plan's title.
    pub title: Option<String>,
    pub steps: Vec<Step>,
    pub card: Option<ShapeCard>,
}

/// `SetupComponent.needsOwner`: the cards that ask the owner to act.
fn needs_owner(k: Kind) -> bool {
    matches!(
        k,
        Kind::Confirm | Kind::Questions | Kind::Approval | Kind::SignIn | Kind::Drive
    )
}

/// `SetupThread.asks`: the cards that ask the owner to act, in the thread's order.
pub fn asks(thread: &SetupThread) -> Vec<&ShownComponent> {
    thread
        .items()
        .into_iter()
        .filter_map(|i| match i {
            Item::Component(c) if needs_owner(c.component.kind) => Some(c),
            _ => None,
        })
        .collect()
}

/// `wroteSince`: the owner wrote after `at`.
fn wrote_since(thread: &SetupThread, at: chrono::DateTime<chrono::Utc>) -> bool {
    thread
        .messages
        .iter()
        .any(|m| m.from == crate::coach::From::Owner && m.at > at)
}

/// `setupWaiting`: the card waiting for the owner, if any: the latest approval not answered, or a
/// question or sign-in not answered or written past. One answered here isn't.
pub fn waiting(c: &Conversation) -> Option<&ShownComponent> {
    let t = &c.thread;
    asks(t).into_iter().rev().find(|shown| {
        let card = &shown.component;
        if c.answered.contains(&shown.card_ref()) {
            return false;
        }
        match card.kind {
            Kind::Approval => card_state(t, card) == CardState::NeedsYou,
            Kind::SignIn => {
                card_state(t, card) == CardState::NeedsYou && !wrote_since(t, shown.first_at)
            }
            _ => t.answer_to(&card.id).is_none() && !wrote_since(t, shown.first_at),
        }
    })
}

impl SetupShape {
    /// `isEmpty`.
    pub fn is_empty(&self) -> bool {
        self.name.is_none() && self.steps.is_empty() && self.card.is_none()
    }

    /// `SetupShape(thread:grants:drafts:waiting:states:)`, from the conversation as held here.
    pub fn new(w: &WordBook, c: &Conversation) -> Result<SetupShape, WordsError> {
        let thread = &c.thread;
        let shown: Vec<&crate::coach::Component> =
            thread.components.iter().map(|s| &s.component).collect();
        let name = shown
            .iter()
            .rev()
            .find(|s| s.kind == Kind::Result && s.taf.is_some())
            .and_then(|s| s.taf.as_ref())
            .map(|t| t.name.clone());
        let waiting = waiting(c).map(|s| s.component.id.clone());
        let mut title = None;
        let steps = if let Some(plan) = thread.plan() {
            // The plan, as Coach keeps it. Its step at work waits for the owner when a card does.
            let plan = &plan.component;
            title = plan.title.clone();
            let mut waited = false;
            plan.steps
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|step| {
                    let state = match step.status.as_str() {
                        "done" => StepState::Done,
                        "failed" => StepState::Failed,
                        "skipped" => StepState::Skipped,
                        "active" => {
                            let needs_you = waiting.is_some() && !waited;
                            waited = waited || needs_you;
                            if needs_you {
                                StepState::NeedsYou
                            } else {
                                StepState::Now
                            }
                        }
                        _ => StepState::ToDo,
                    };
                    Step {
                        label: step.label,
                        state,
                    }
                })
                .collect()
        } else {
            // No plan shown: what Coach asked of the owner, and where each stands.
            asks(thread)
                .into_iter()
                .map(|ask| {
                    let card = &ask.component;
                    // planque's vendor isn't read here: the card's service stands for it.
                    let vendor = card.service.clone();
                    let label = match card.kind {
                        Kind::Approval => {
                            let v = match &vendor {
                                Some(v) => v.clone(),
                                None => w.line("coach.shape.the_connection")?,
                            };
                            w.get("coach.shape.approve_named", &[("vendor", &v)])?
                        }
                        Kind::SignIn => match &vendor {
                            Some(v) => w.get("coach.sign_in.at", &[("vendor", v)])?,
                            None => w.line("coach.sign_in.at_vendor")?,
                        },
                        _ => match card.text.clone().or(card.title.clone()) {
                            Some(t) => t,
                            None => w.line("needs_you.answer")?,
                        },
                    };
                    if waiting.as_deref() == Some(card.id.as_str()) {
                        return Ok(Step {
                            label,
                            state: StepState::NeedsYou,
                        });
                    }
                    let state = if card.kind == Kind::Confirm {
                        // Answered, or written past: the conversation went on without it.
                        if thread.answer_to(&card.id).is_some() {
                            StepState::Done
                        } else {
                            StepState::Skipped
                        }
                    } else {
                        match card_state(thread, card) {
                            CardState::Done => StepState::Done,
                            CardState::Declined => StepState::Skipped,
                            CardState::Failed(_) => StepState::Failed,
                            CardState::NeedsYou => StepState::ToDo,
                        }
                    };
                    Ok(Step { label, state })
                })
                .collect::<Result<_, WordsError>>()?
        };
        // The latest draft an approval names that KIFF has shown the app, holding tools.
        let card = shown
            .iter()
            .filter(|s| s.kind == Kind::Approval)
            .filter_map(|s| s.draft.as_ref().and_then(|d| c.drafts.get(d)))
            .rfind(|d| d.holds_tools());
        let card = match card {
            Some(d) => Some(ShapeCard {
                title: d.card_title(w)?,
                lines: d.lines(w)?,
                confirmed: d.confirmed(),
            }),
            None => None,
        };
        Ok(SetupShape {
            name,
            title,
            steps,
            card,
        })
    }
}

/// One step as the panel draws it: its mark, its words' weight and colour, struck through when
/// skipped, and "Needs you" under it when it waits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepLine {
    /// The plan's mark (`StepMark`), with its words for a screen reader.
    pub mark: PlanStep,
    pub strong: bool,
    pub dim: bool,
    pub struck: bool,
    pub needs_you: Option<String>,
}

/// The Card as the panel draws it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CardLines {
    pub eyebrow: String,
    pub title: String,
    /// Its lines, or "no actions yet".
    pub lines: Vec<String>,
    pub confirmed: bool,
    /// "A draft until you approve it in the conversation.", while it isn't confirmed.
    pub until: Option<String>,
}

/// `SetupShapePanel`'s words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeLines {
    pub eyebrow: String,
    pub heading: Option<String>,
    /// Nothing made yet: what will show here.
    pub empty: Option<String>,
    /// "Plan · 2 of 4", and each step.
    pub plan: Option<(String, Vec<StepLine>)>,
    pub card: Option<CardLines>,
}

/// `SetupShapePanel.mark`.
fn mark(state: StepState) -> &'static str {
    match state {
        StepState::Done => "done",
        StepState::Failed => "failed",
        StepState::NeedsYou | StepState::Now => "active",
        StepState::ToDo | StepState::Skipped => "pending",
    }
}

/// `SetupShapePanel.body`.
pub fn lines(w: &WordBook, shape: &SetupShape) -> Result<ShapeLines, WordsError> {
    let plan = if shape.steps.is_empty() {
        None
    } else {
        let done = shape
            .steps
            .iter()
            .filter(|s| s.state == StepState::Done)
            .count();
        let progress = w.get(
            "coach.plan.progress",
            &[
                ("done", &done.to_string()),
                ("total", &shape.steps.len().to_string()),
            ],
        )?;
        let steps = shape
            .steps
            .iter()
            .map(|s| {
                let status = mark(s.state);
                Ok(StepLine {
                    mark: crate::coach_lines::plan_step(w, status, &s.label)?,
                    strong: matches!(s.state, StepState::NeedsYou | StepState::Now),
                    dim: matches!(s.state, StepState::ToDo | StepState::Skipped),
                    struck: s.state == StepState::Skipped,
                    needs_you: if s.state == StepState::NeedsYou {
                        Some(w.line("needs_you.title")?)
                    } else {
                        None
                    },
                })
            })
            .collect::<Result<_, WordsError>>()?;
        Some((progress, steps))
    };
    let card = match &shape.card {
        Some(c) => Some(CardLines {
            eyebrow: w.line(if c.confirmed {
                "card.card_confirmed_kiff"
            } else {
                "card.card_draft_kiff"
            })?,
            title: c.title.clone(),
            lines: if c.lines.is_empty() {
                vec![w.line("card.no_actions_short")?]
            } else {
                c.lines.clone()
            },
            confirmed: c.confirmed,
            until: if c.confirmed {
                None
            } else {
                Some(w.line("coach.shape.draft_until_approved")?)
            },
        }),
        None => None,
    };
    Ok(ShapeLines {
        eyebrow: w.line("coach.shape.taking_shape")?,
        heading: shape.name.clone().or(shape.title.clone()),
        empty: if shape.is_empty() {
            Some(w.line("billing.conversation_goes_plan_tools_being")?)
        } else {
            None
        },
        plan,
        card,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn book() -> WordBook {
        WordBook::load(&WordBook::default_root(), "en").unwrap()
    }

    fn at(s: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(s).unwrap().into()
    }

    fn conv(messages: serde_json::Value, components: serde_json::Value) -> Conversation {
        let mut c = Conversation::default();
        let t = SetupThread::read(
            &serde_json::to_vec(&json!({"messages": messages, "components": components})).unwrap(),
        )
        .unwrap();
        c.took(t, at("2026-10-16T13:00:00Z"));
        c
    }

    fn card(id: &str, seq: i64, component: serde_json::Value) -> serde_json::Value {
        json!({"at": format!("2026-10-16T12:00:{:02}.000Z", seq), "session": "s", "run": "r",
               "seq": seq, "component": component, "key": format!("s#{id}")})
    }

    fn plan() -> serde_json::Value {
        card(
            "plan",
            1,
            json!({"type": "steps", "id": "plan", "title": "Set up Refunds", "steps": [
                {"label": "Connect Stripe", "status": "done"},
                {"label": "Approve the Card", "status": "active"},
                {"label": "Try it", "status": "pending"},
                {"label": "Old idea", "status": "skipped"}]}),
        )
    }

    fn approval() -> serde_json::Value {
        card(
            "card",
            5,
            json!({"type": "approval", "id": "card", "title": "Refunds' Card",
                   "draft": "drf_1", "waits": "owner"}),
        )
    }

    #[test]
    fn nothing_made_yet_says_what_will_show() {
        let w = book();
        let s = SetupShape::new(&w, &Conversation::default()).unwrap();
        assert!(s.is_empty());
        let l = lines(&w, &s).unwrap();
        assert_eq!(l.eyebrow, "Taking shape");
        assert_eq!(l.heading, None);
        assert!(l.empty.unwrap().starts_with("As the conversation goes"));
        assert_eq!(l.plan, None);
    }

    #[test]
    fn the_plan_with_its_step_at_work_waiting_for_the_owner() {
        let w = book();
        let c = conv(json!([]), json!([plan(), approval()]));
        assert_eq!(waiting(&c).unwrap().component.id, "card");
        let s = SetupShape::new(&w, &c).unwrap();
        assert_eq!(s.title.as_deref(), Some("Set up Refunds"));
        let states: Vec<StepState> = s.steps.iter().map(|s| s.state).collect();
        assert_eq!(
            states,
            [
                StepState::Done,
                StepState::NeedsYou,
                StepState::ToDo,
                StepState::Skipped
            ]
        );
        let l = lines(&w, &s).unwrap();
        assert_eq!(l.heading.as_deref(), Some("Set up Refunds"));
        let (progress, steps) = l.plan.unwrap();
        assert_eq!(progress, "Plan · 1 of 4");
        assert_eq!(steps[1].needs_you.as_deref(), Some("Needs you"));
        assert!(steps[1].strong && !steps[1].dim);
        assert_eq!(steps[1].mark.status, "active");
        assert!(steps[3].struck && steps[3].dim);
        assert_eq!(steps[3].mark.status, "pending");
        // No Card read: none drawn.
        assert_eq!(l.card, None);
    }

    #[test]
    fn with_no_plan_what_coach_asked_and_where_each_stands() {
        let w = book();
        let c = conv(
            json!([{"id": "o", "at": "2026-10-16T12:00:30.000Z", "from": "owner",
                    "kind": "message", "text": "live", "session": "s"}]),
            json!([
                card(
                    "mode",
                    2,
                    json!({"type": "confirm", "id": "mode", "text": "Test or live?",
                    "options": [{"value": "live", "label": "Live"}], "waits": "owner"})
                ),
                card(
                    "sign",
                    3,
                    json!({"type": "sign_in", "id": "sign", "service": "Stripe", "waits": "owner"})
                ),
                approval(),
            ]),
        );
        let s = SetupShape::new(&w, &c).unwrap();
        let got: Vec<(String, StepState)> =
            s.steps.iter().map(|s| (s.label.clone(), s.state)).collect();
        assert_eq!(
            got,
            [
                // Written past: the conversation went on without it.
                ("Test or live?".to_string(), StepState::Skipped),
                ("Sign in at Stripe".to_string(), StepState::ToDo),
                ("Approve the connection".to_string(), StepState::NeedsYou),
            ]
        );
    }

    #[test]
    fn the_card_kiff_drafted_and_the_name_once_handed_over() {
        let w = book();
        let mut c = conv(
            json!([]),
            json!([
                plan(),
                approval(),
                card(
                    "done",
                    9,
                    json!({"type": "result", "id": "done",
                "text": "Ready.", "taf": {"agent": "refunds-x", "name": "Refunds"}})
                )
            ]),
        );
        let draft = |status: &str| {
            crate::drafts::AgentDraft::read(
                json!({"id": "drf_1", "agent_id": "a", "status": status, "version": "1",
                       "label": "Refunds", "cards": [{"lines": ["Refund up to €5 each"]}]})
                .to_string()
                .as_bytes(),
            )
            .unwrap()
        };
        c.took_draft("drf_1", Ok(draft("draft")));
        let s = SetupShape::new(&w, &c).unwrap();
        assert_eq!(s.name.as_deref(), Some("Refunds"));
        let l = lines(&w, &s).unwrap();
        assert_eq!(l.heading.as_deref(), Some("Refunds"));
        let card = l.card.unwrap();
        assert_eq!(card.eyebrow, "Card · draft · by KIFF");
        assert_eq!(card.title, "Refunds' Card");
        assert_eq!(card.lines, ["Refund up to €5 each"]);
        assert!(card.until.is_some());
        c.took_draft("drf_1", Ok(draft("confirmed")));
        let card = lines(&w, &SetupShape::new(&w, &c).unwrap())
            .unwrap()
            .card
            .unwrap();
        assert_eq!(card.eyebrow, "Card · confirmed by KIFF");
        assert_eq!(card.until, None);
    }

    #[test]
    fn every_shipped_language_has_the_panels_words() {
        for lang in crate::words::SHIPPED {
            let w = WordBook::load(&WordBook::default_root(), lang).unwrap();
            let c = conv(json!([]), json!([plan(), approval()]));
            lines(&w, &SetupShape::new(&w, &c).unwrap()).unwrap();
            lines(&w, &SetupShape::default()).unwrap();
        }
    }
}
