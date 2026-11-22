//! Continue on the welcome (K5, `AppModel.signInInApp`): KIFF's sign-in in the person's browser,
//! the way back at `dev.kiff.tafs:/oauth` (the desktop hands it to the running app, `open`), then
//! the token with the terms version the tick agreed to, kept as today's sign-in keeps a session.
//!
//! One attempt at a time: a new Continue replaces the last one, whose link back is then never
//! used. A code KIFF gave before saying the terms changed is kept for 50 of its 60 seconds, so the
//! new tick sends it again without opening KIFF's page twice (as on Android); past that, nothing is
//! kept.

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gtk::{gio, glib};
pub use tafs_core::app_signin::Outcome;
use tafs_core::app_signin::{self, Attempt, Back};
use tafs_core::keys::DeviceKey;
use tafs_core::session::KeptSession;
use tafs_core::signin::{self, KiffError};
use tafs_core::words::WordBook;

use crate::kiff::Kiff;

/// How long a code is kept after KIFF said the terms changed: 50 of its 60 seconds.
const KEPT_CODE: Duration = Duration::from_secs(50);

/// What the welcome hears back.
pub struct Ends {
    /// Signed in: the session, kept.
    pub signed_in: Rc<dyn Fn(KeptSession)>,
    /// Didn't sign in: how it ended.
    pub ended: Rc<dyn Fn(Outcome)>,
}

struct Waiting {
    attempt: Attempt,
    terms: String,
    words: Rc<WordBook>,
    languages: Vec<&'static str>,
    ends: Rc<Ends>,
}

thread_local! {
    static WAITING: RefCell<Option<Waiting>> = const { RefCell::new(None) };
    /// A code from an attempt KIFF answered `terms_outdated`, with the attempt it belongs to.
    static KEPT: RefCell<Option<(Attempt, String, Instant)>> = const { RefCell::new(None) };
}

/// Continue: opens KIFF's page for a fresh attempt, or, when KIFF just said the terms changed and
/// the code is still good, sends that code again with the new version.
pub fn begin(
    window: &gtk::Window,
    w: Rc<WordBook>,
    languages: Vec<&'static str>,
    terms: String,
    ends: Rc<Ends>,
) {
    let kept = KEPT.with(|k| k.borrow_mut().take());
    if let Some((attempt, code, at)) = kept {
        if at.elapsed() < KEPT_CODE {
            redeem(attempt, code, terms, w, languages, ends);
            return;
        }
    }
    let attempt = Attempt::new();
    let link = attempt.authorize_link(w.language(), None);
    WAITING.with(|wt| {
        *wt.borrow_mut() = Some(Waiting {
            attempt,
            terms,
            words: w,
            languages,
            ends,
        })
    });
    gtk::UriLauncher::new(&link).launch(Some(window), gio::Cancellable::NONE, |_| {});
}

/// The welcome is gone, or the person started over: the attempt's link back is never used.
pub fn forget() {
    WAITING.with(|w| *w.borrow_mut() = None);
    KEPT.with(|k| *k.borrow_mut() = None);
}

/// A link the desktop handed the app. Only this attempt's is taken, once; any other is dropped,
/// not even read as a refusal.
pub fn link(uri: &str) {
    let back = WAITING.with(|w| w.borrow().as_ref().and_then(|w| w.attempt.answer(uri)));
    let Some(back) = back else {
        return;
    };
    let Some(waiting) = WAITING.with(|w| w.borrow_mut().take()) else {
        return;
    };
    match back {
        Back::Refused(error) => (waiting.ends.ended)(app_signin::back_refused(&error)),
        Back::Code(code) => redeem(
            waiting.attempt,
            code,
            waiting.terms,
            waiting.words,
            waiting.languages,
            waiting.ends,
        ),
    }
}

/// The code, with this attempt's verifier and the ticked terms, for a session kept here.
fn redeem(
    attempt: Attempt,
    code: String,
    terms: String,
    w: Rc<WordBook>,
    languages: Vec<&'static str>,
    ends: Rc<Ends>,
) {
    glib::spawn_future_local(async move {
        let name = signin::device_name(&w).unwrap_or_else(|e| panic!("{e}"));
        let words: WordBook = (*w).clone();
        let (a, c) = (attempt.clone(), code.clone());
        let done = gio::spawn_blocking(move || {
            let key = Arc::new(DeviceKey::generate());
            let device = signin::start_body(&key.public_spki_b64(), &name);
            let body = a.token_body(&c, &terms, &device);
            let token = app_signin::token(&Kiff::new(languages), &body, Some(&words))?;
            crate::sign_in::keep(&token, &key).map_err(Outcome::Failed)
        })
        .await
        .unwrap_or(Err(Outcome::Failed(KiffError::Unreadable)));
        match done {
            Ok(kept) => (ends.signed_in)(kept),
            Err(Outcome::TermsChanged) => {
                KEPT.with(|k| *k.borrow_mut() = Some((attempt, code, Instant::now())));
                (ends.ended)(Outcome::TermsChanged)
            }
            Err(other) => (ends.ended)(other),
        }
    });
}

/// The terms version in force, read off the main loop; None when KIFF couldn't be read.
pub async fn terms_version(languages: Vec<&'static str>) -> Option<String> {
    gio::spawn_blocking(move || {
        crate::kiff::app_terms(&languages)
            .ok()
            .and_then(|a| app_signin::terms_version(&a))
    })
    .await
    .ok()
    .flatten()
}

#[cfg(test)]
mod tests {
    use gtk::gio;
    use gtk::prelude::FileExt;
    use tafs_core::app_signin::{Attempt, Back};

    /// The way back as GTK hands it to `open`: the browser runs `tafs <link>`, GApplication makes
    /// the argument a `GFile` (`g_file_new_for_commandline_arg`), and `open` reads its URI. A
    /// `scheme:/path` link with no `//` and a query must come through byte for byte.
    #[test]
    fn gio_hands_the_link_back_as_it_came() {
        let link = "dev.kiff.tafs:/oauth?code=c1-_Z9&state=st8_x-Y";
        let file = gio::File::for_commandline_arg(link);
        assert_eq!(file.uri(), link);
        let a = Attempt::with("v", "st8_x-Y");
        assert_eq!(a.answer(&file.uri()), Some(Back::Code("c1-_Z9".into())));
        // A refusal, with percent-escaped words, comes through too.
        let refused = "dev.kiff.tafs:/oauth?error=access_denied&error_description=The%20person%20declined&state=st8_x-Y";
        let file = gio::File::for_commandline_arg(refused);
        assert_eq!(
            a.answer(&file.uri()),
            Some(Back::Refused("access_denied".into()))
        );
    }
}
