//! "Allow this from now on" (`AllowSheetView`, kiff/tafs `docs/allow-from-now-on.md`), over a held
//! call's card. Opening it asks tafs cloud for its offer (`POST …/holds/{exception}/allow`, as the
//! session), then reads KIFF's draft here (`GET /v1/me/agent-drafts/{id}`) and checks it is exactly
//! what was offered (`tafs_core::allow::matches`). It shows tafs cloud's change before and after,
//! KIFF's own lines for each Card that changes, what it would have done, and the held call's
//! switch. One approval with this computer's security key confirms the draft at the version read,
//! with the held call when the switch is on (`signed::Act::AllowFromNowOn`). A `none` shows tafs
//! cloud's words with nothing to sign; a draft that isn't what was offered says so, and nothing is
//! signed. Round up asks tafs cloud for the rounded draft, checked the same way: the app never
//! rounds.

use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use tafs_core::allow::{self, Answer};
use tafs_core::drafts::AgentDraft;
use tafs_core::signed::Act;
use tafs_core::words::WordBook;

use crate::widgets::{outline, primary, text};

/// The held call the sheet is open on, as its card knows it.
#[derive(Clone)]
pub struct Target {
    /// Its taf, by tafs cloud's id.
    pub taf: String,
    /// KIFF's id for the hold.
    pub exception: String,
    pub agent: String,
    pub action: String,
    /// The hold's accounting version, as the card showed it: what "Let this one go out too" names.
    pub accounting_version: Option<i64>,
}

#[derive(Default)]
struct State {
    answer: Option<Answer>,
    draft: Option<AgentDraft>,
    matches: bool,
    busy: bool,
    error: Option<String>,
    /// "Let this one go out too": the held call goes with the same signature.
    send_held: bool,
    /// The rounded figure was asked for: the sheet no longer offers to round.
    rounded: bool,
}

/// The sheet's draw, set once it exists, so a read or a switch can draw it again.
type Redraw = Rc<RefCell<Option<Rc<dyn Fn()>>>>;

/// What became of the sheet: the Card changed, and whether the held call went with it.
pub type Done = Rc<dyn Fn(bool)>;

/// tafs cloud's offer, then KIFF's draft read here and whether it's what was offered. Blocking:
/// off the main loop. Why it couldn't be read, in KIFF's or tafs cloud's words, or the app's.
fn read(
    w: &WordBook,
    token: &str,
    languages: &[&'static str],
    t: &Target,
    round: bool,
) -> Result<(Answer, Option<AgentDraft>, bool), String> {
    let why = |s: String| w.get("version.card_not_read", &[("why", &s)]).unwrap_or(s);
    let a = crate::kiff::cloud_send(
        token,
        languages,
        "POST",
        &allow::offer_path(&t.taf, &t.exception),
        Some(&allow::offer_body(round)),
    )
    .map_err(|e| why(e.words(w).unwrap_or_default()))?;
    if !(200..300).contains(&a.status) {
        return Err(why(tafs_core::signin::refusal(&a, Some(w))
            .words(w)
            .unwrap_or_default()));
    }
    let plain = || why(tafs_core::signin::plain_words(w, 200).unwrap_or_default());
    let answer = Answer::read(&a.body).ok_or_else(plain)?;
    let Some(offer) = answer.offer.clone().filter(|o| !o.draft.is_empty()) else {
        return Ok((answer, None, false));
    };
    let call = tafs_core::drafts::draft_read(&offer.draft).ok_or_else(plain)?;
    let d = crate::kiff::kiff_get(token, languages, call.path())
        .map_err(|e| why(e.words(w).unwrap_or_default()))?;
    if !(200..300).contains(&d.status) {
        return Err(why(tafs_core::signin::refusal(&d, Some(w))
            .words(w)
            .unwrap_or_default()));
    }
    let draft = AgentDraft::read(&d.body).ok_or_else(plain)?;
    // Signed only when every key of its Cards is one the check reads, and it's what was offered.
    let matches = allow::signable(&d.body, &draft, &offer, &t.agent, &t.exception, &t.action);
    Ok((answer, Some(draft), matches))
}

/// Opens the sheet over `over` and asks tafs cloud for the offer now, never before the tap.
pub fn open(
    w: Rc<WordBook>,
    over: &impl IsA<gtk::Widget>,
    token: String,
    languages: Vec<&'static str>,
    target: Target,
    done: Done,
) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let title = l("needs_you.allow_title");
    let sheet = crate::sheet::new(&title, &l("common.cancel"), None, 520);
    sheet.dialog.set_widget_name("allow.sheet");
    let state = Rc::new(RefCell::new(State {
        busy: true,
        send_held: true,
        ..State::default()
    }));
    // Why a confirm didn't go, under the sheet's content; it stays across redraws.
    let refused = crate::approval::refusal_slot();
    let redraw: Redraw = Rc::new(RefCell::new(None));
    let load: Rc<dyn Fn(bool)> = {
        let (w, state, redraw, token, languages, target) = (
            w.clone(),
            state.clone(),
            redraw.clone(),
            token.clone(),
            languages.clone(),
            target.clone(),
        );
        Rc::new(move |round: bool| {
            {
                let mut s = state.borrow_mut();
                s.busy = true;
                s.error = None;
            }
            if let Some(r) = redraw.borrow().clone() {
                r();
            }
            let (words, token, languages, target) = (
                (*w).clone(),
                token.clone(),
                languages.clone(),
                target.clone(),
            );
            let (state, redraw) = (state.clone(), redraw.clone());
            gtk::glib::spawn_future_local(async move {
                let got = gtk::gio::spawn_blocking(move || {
                    read(&words, &token, &languages, &target, round)
                })
                .await
                .unwrap_or_else(|_| Err(String::new()));
                {
                    let mut s = state.borrow_mut();
                    s.busy = false;
                    match got {
                        Ok((answer, draft, matches)) => {
                            // Rounding up keeps the person's switch; the first offer sets it.
                            if !round {
                                s.send_held = answer
                                    .offer
                                    .as_ref()
                                    .and_then(|o| o.send_held)
                                    .unwrap_or(true);
                            }
                            s.answer = Some(answer);
                            s.draft = draft;
                            s.matches = matches;
                            s.rounded = round;
                            s.error = None;
                        }
                        Err(e) => s.error = Some(e),
                    }
                }
                if let Some(r) = redraw.borrow().clone() {
                    r();
                }
            });
        })
    };
    let draw: Rc<dyn Fn()> = {
        let (w, state, column, refused, load, dialog, target, done) = (
            w.clone(),
            state.clone(),
            sheet.column.clone(),
            refused.clone(),
            load.clone(),
            sheet.dialog.downgrade(),
            target.clone(),
            done.clone(),
        );
        let redraw = redraw.clone();
        Rc::new(move || {
            while let Some(c) = column.first_child() {
                column.remove(&c);
            }
            let l = |k: &str| w.line(k).unwrap_or_default();
            let s = state.borrow();
            let heading = text(&l("needs_you.allow_title"), 22, 700, &["tafs-ink"]);
            heading.update_property(&[gtk::accessible::Property::Label(&l(
                "needs_you.allow_title",
            ))]);
            column.append(&heading);
            match (&s.answer, s.busy) {
                (Some(answer), _) => match &answer.offer {
                    Some(offer) => {
                        for c in &offer.changes {
                            let pair = gtk::Box::new(gtk::Orientation::Vertical, 2);
                            pair.append(&text(
                                c.words_before.as_deref().unwrap_or_default(),
                                14,
                                400,
                                &["tafs-soft"],
                            ));
                            pair.append(&text(
                                &format!("→ {}", c.words_after.as_deref().unwrap_or_default()),
                                16,
                                600,
                                &["tafs-ink"],
                            ));
                            column.append(&pair);
                        }
                        // KIFF's own lines for each Card that changes, as it keeps them: what's
                        // signed.
                        if let Some(d) = &s.draft {
                            let cards = d.cards.as_deref().unwrap_or_default();
                            let lines = d.card_lines(&w).unwrap_or_default();
                            for (card, lines) in cards.iter().zip(lines) {
                                if card.unchanged == Some(true) && card.current.is_some() {
                                    continue;
                                }
                                for line in lines {
                                    column.append(&text(&line, 13, 400, &["tafs-dim"]));
                                }
                            }
                        }
                        if let Some(would) = offer.would.as_deref().filter(|w| !w.is_empty()) {
                            column.append(&text(would, 14, 400, &["tafs-ink"]));
                        }
                        if !s.matches {
                            // A draft that couldn't be read says so in the error line alone.
                            if s.draft.is_some() || (!s.busy && s.error.is_none()) {
                                let no =
                                    text(&l("needs_you.allow_not_offered"), 14, 400, &["tafs-no"]);
                                no.set_widget_name("allow.not-offered");
                                column.append(&no);
                            }
                        } else {
                            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                            let label =
                                text(&l("needs_you.allow_send_this_too"), 15, 400, &["tafs-ink"]);
                            label.set_hexpand(true);
                            row.append(&label);
                            let switch = gtk::Switch::new();
                            switch.set_active(s.send_held);
                            switch.set_sensitive(!s.busy);
                            switch.set_valign(gtk::Align::Center);
                            switch.set_widget_name("allow.send-held");
                            switch.update_property(&[gtk::accessible::Property::Label(&l(
                                "needs_you.allow_send_this_too",
                            ))]);
                            {
                                let (state, redraw) = (state.clone(), redraw.clone());
                                switch.connect_active_notify(move |sw| {
                                    let changed = {
                                        let mut s = state.borrow_mut();
                                        let was = s.send_held;
                                        s.send_held = sw.is_active();
                                        was != s.send_held
                                    };
                                    if changed {
                                        if let Some(r) = redraw.borrow().clone() {
                                            gtk::glib::idle_add_local_once(move || r());
                                        }
                                    }
                                });
                            }
                            row.append(&switch);
                            column.append(&row);
                            if !s.send_held {
                                column.append(&text(
                                    &l("needs_you.allow_switch_off"),
                                    13,
                                    400,
                                    &["tafs-soft"],
                                ));
                            }
                            if let Some(figure) = offer
                                .round
                                .as_deref()
                                .filter(|f| !f.is_empty() && !s.rounded)
                            {
                                let round = outline(
                                    &w.get("needs_you.allow_round_up", &[("amount", figure)])
                                        .unwrap_or_default(),
                                );
                                round.set_widget_name("allow.round");
                                round.set_sensitive(!s.busy);
                                let load = load.clone();
                                round.connect_clicked(move |_| load(true));
                                column.append(&round);
                            }
                            column.append(&confirm(
                                &w,
                                &s,
                                &target,
                                &refused,
                                &state,
                                &redraw,
                                dialog.clone(),
                                done.clone(),
                            ));
                        }
                    }
                    // No offer, whatever its reason: tafs cloud's words, nothing to sign.
                    None => {
                        let none = text(
                            answer.words.as_deref().unwrap_or_default(),
                            15,
                            400,
                            &["tafs-soft"],
                        );
                        none.set_widget_name("allow.none");
                        column.append(&none);
                    }
                },
                (None, true) => {
                    let spinner = gtk::Spinner::new();
                    spinner.start();
                    spinner.set_halign(gtk::Align::Center);
                    column.append(&spinner);
                }
                (None, false) => {}
            }
            if let Some(e) = &s.error {
                column.append(&text(e, 13, 400, &["tafs-no"]));
            }
            if refused.parent().is_some() {
                refused.unparent();
            }
            column.append(&refused);
        })
    };
    *redraw.borrow_mut() = Some(draw.clone());
    draw();
    sheet.dialog.present(Some(over));
    load(false);
}

/// The one signature: Approve, asking this computer's security key to confirm KIFF's draft at the
/// version read, with the held call when the switch is on. Without the key, the gate's words, and
/// nothing is signed.
#[allow(clippy::too_many_arguments)]
fn confirm(
    w: &Rc<WordBook>,
    s: &State,
    target: &Target,
    refused: &gtk::Box,
    state: &Rc<RefCell<State>>,
    redraw: &Redraw,
    dialog: gtk::glib::WeakRef<adw::Dialog>,
    done: Done,
) -> gtk::Widget {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let button = primary(&if s.busy {
        l("approval.approving")
    } else {
        l("approval.approve")
    });
    button.set_widget_name("allow.confirm");
    let Some(draft) = s.draft.as_ref().filter(|d| !d.confirmed()) else {
        button.set_sensitive(false);
        return button.upcast();
    };
    let act = |send: bool| Act::AllowFromNowOn {
        draft: draft.id.clone(),
        version: draft.version.clone(),
        hold: if send {
            target
                .accounting_version
                .map(|v| (target.exception.clone(), v))
        } else {
            None
        },
    };
    let kept = crate::store::load(chrono::Utc::now().timestamp());
    if !crate::key_approval::can_approve(kept.as_ref(), &act(false)) {
        button.set_sensitive(false);
        return crate::approval::gate(w, tafs_core::sidebar::Mode::Kiff, &button);
    }
    button.set_sensitive(!s.busy);
    let (w, state, redraw, refused, target) = (
        w.clone(),
        state.clone(),
        redraw.clone(),
        refused.clone(),
        target.clone(),
    );
    let (draft_id, version) = (draft.id.clone(), draft.version.clone());
    button.connect_clicked(move |_| {
        let send = state.borrow().send_held;
        // The call goes only as the person saw it: without its version, nothing is signed.
        if send && target.accounting_version.is_none() {
            state.borrow_mut().error =
                Some(w.line("needs_you.answer_not_taken").unwrap_or_default());
            if let Some(r) = redraw.borrow().clone() {
                r();
            }
            return;
        }
        let act = Act::AllowFromNowOn {
            draft: draft_id.clone(),
            version: version.clone(),
            hold: send.then(|| {
                (
                    target.exception.clone(),
                    target.accounting_version.unwrap_or_default(),
                )
            }),
        };
        let (dialog, done) = (dialog.clone(), done.clone());
        crate::approval::refuse_then(
            &w,
            tafs_core::sidebar::Mode::Kiff,
            &act,
            w.line("needs_you.allow_title").ok(),
            vec![],
            &refused,
            Some(Rc::new(move || {
                if let Some(d) = dialog.upgrade() {
                    d.close();
                }
                done(send);
            })),
        );
    });
    button.upcast()
}
