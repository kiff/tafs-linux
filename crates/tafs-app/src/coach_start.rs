//! How Coach opens from elsewhere in the app, as the iPad's `AppModel` opens it: a new taf
//! (`startNewTaf`: a new conversation, the box as it was), a job handed to Coach
//! (`handRecipeToCoach`: a new conversation with the job's words in the box, nothing sent), and
//! Finish setting up (`finishSetup`: the chat where the setup stopped). Coach's page takes what was
//! asked as it opens (`apply`), and the window opens Coach beside the sidebar (`set_opener`).

use std::cell::RefCell;
use std::rc::Rc;

use tafs_core::coach_send::Conversation;

/// What Coach opens on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// A new conversation, on no taf (`newSetupChat`).
    New,
}

thread_local! {
    /// What Coach opens on next; none, as it was (its newest chat).
    static START: RefCell<Option<Start>> = const { RefCell::new(None) };
    /// Coach opened beside the sidebar, afresh even when it's open already.
    static OPEN: RefCell<Option<Rc<dyn Fn()>>> = const { RefCell::new(None) };
}

/// The window's way to Coach, set once the sidebar is built; none, nothing opens.
pub fn set_opener(open: Option<Rc<dyn Fn()>>) {
    OPEN.with(|o| *o.borrow_mut() = open);
}

/// Coach opened on `start`, with `draft` in its box where one is given; nothing without a way to
/// Coach (signed out, the sample).
fn open(start: Start, draft: Option<&str>) {
    let Some(open) = OPEN.with(|o| o.borrow().clone()) else {
        return;
    };
    if let Some(words) = draft {
        crate::coach::set_draft(words);
    }
    START.with(|s| *s.borrow_mut() = Some(start));
    open();
}

/// `startNewTaf`: Coach in a new conversation, on no taf; what's in its box stays.
pub fn new_taf() {
    open(Start::New, None);
}

/// `handRecipeToCoach`: Coach in a new conversation, `say` in its box for the person to edit and
/// send. Nothing is sent.
pub fn hand(say: &str) {
    open(Start::New, Some(say));
}

/// What was asked, taken once: none after.
pub fn take() -> Option<Start> {
    START.with(|s| s.borrow_mut().take())
}

/// Coach's page as it opens: the conversation it was asked for, if any.
pub fn apply(conv: &mut Conversation) {
    match take() {
        Some(Start::New) => conv.new_chat(),
        None => {}
    }
}

/// Signed out: nothing asked for the last account stays.
pub fn forget() {
    START.with(|s| *s.borrow_mut() = None);
    set_opener(None);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coach_opens_on_what_was_asked_once() {
        let opened = Rc::new(std::cell::Cell::new(0));
        let o = opened.clone();
        set_opener(Some(Rc::new(move || o.set(o.get() + 1))));
        new_taf();
        assert_eq!(opened.get(), 1);
        let mut conv = Conversation::default();
        conv.chat = Some("chat_old".into());
        apply(&mut conv);
        assert!(conv.fresh);
        assert_eq!(conv.chat, None);
        // Taken once: Coach opened again from its button is as it was.
        assert_eq!(take(), None);
        hand("Chase unpaid invoices");
        assert_eq!(opened.get(), 2);
        assert_eq!(take(), Some(Start::New));
        forget();
        new_taf();
        assert_eq!(opened.get(), 2, "signed out, nothing opens");
        assert_eq!(take(), None, "and nothing waits for the next Coach");
    }
}
