//! A taf's page folds its head while the person scrolls down, so the chat has the window, and
//! brings it back on the way up (the iPad's `TafDetailView.folded`, `fold(_:)`): the line under
//! the name (its Files, its versions) and the Card bar. Only the person's own scrolling folds it
//! (the iPad's drag, here a wheel, a touchpad or a finger), never the chat following an answer as
//! it's written. The tabs and the name stay.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use gtk::prelude::*;

/// How far a scroll or a drag goes before it folds or unfolds: the iPad's 24 points.
const AFTER: f64 = 24.0;

/// The page's folding: the parts it hides, and whether they're hidden.
#[derive(Default)]
pub struct Fold {
    folded: Cell<bool>,
    parts: RefCell<Vec<gtk::Revealer>>,
}

thread_local! {
    /// `--page taf-folded:<id>`: the page opens folded, each time it's drawn, for the screenshot.
    pub static OPEN_FOLDED: Cell<bool> = const { Cell::new(false) };
}

impl Fold {
    pub fn new() -> Rc<Fold> {
        Rc::new(Fold::default())
    }

    /// `part` in a revealer that folds it away upwards, in 0.2 s (none with the desktop's
    /// animations off).
    pub fn part(&self, part: &impl IsA<gtk::Widget>) -> gtk::Revealer {
        let r = gtk::Revealer::new();
        r.set_transition_type(gtk::RevealerTransitionType::SlideUp);
        r.set_transition_duration(200);
        r.set_child(Some(part));
        r.set_reveal_child(!self.folded.get());
        self.parts.borrow_mut().push(r.clone());
        r
    }

    /// Folds going down, unfolds going up; nothing when it already is.
    pub fn set(&self, down: bool) {
        if self.folded.replace(down) == down {
            return;
        }
        for r in self.parts.borrow().iter() {
            r.set_reveal_child(!down);
        }
    }

    /// Folds as the person scrolls `scroller`: a wheel's notch at once, a touchpad's or a finger's
    /// movement once it has gone 24 pixels one way.
    pub fn watch(self: &Rc<Self>, scroller: &gtk::ScrolledWindow) {
        let wheel = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::VERTICAL);
        // Before the scroller takes it, and left for it to scroll.
        wheel.set_propagation_phase(gtk::PropagationPhase::Capture);
        let gone = Rc::new(Cell::new(0.0_f64));
        {
            let gone = gone.clone();
            wheel.connect_scroll_begin(move |_| gone.set(0.0));
        }
        {
            let (fold, gone) = (Rc::downgrade(self), gone.clone());
            wheel.connect_scroll(move |c, _, dy| {
                if let Some(fold) = fold.upgrade() {
                    let pixels = c.unit() == gtk::gdk::ScrollUnit::Surface;
                    if let Some(down) = moved(&gone, dy, pixels) {
                        fold.set(down);
                    }
                }
                gtk::glib::Propagation::Proceed
            });
        }
        scroller.add_controller(wheel);
        // A finger on a touchscreen: the iPad's drag, its content moving up as the page goes down.
        let drag = gtk::GestureDrag::new();
        drag.set_touch_only(true);
        drag.set_propagation_phase(gtk::PropagationPhase::Capture);
        {
            let fold = Rc::downgrade(self);
            drag.connect_drag_update(move |_, _, dy| {
                if let (Some(fold), Some(down)) = (fold.upgrade(), dragged(dy)) {
                    fold.set(down);
                }
            });
        }
        scroller.add_controller(drag);
    }

    /// Watches the first scroller in `pane`, once it's built (the chat's, which it makes itself).
    pub fn watch_in(self: &Rc<Self>, pane: &impl IsA<gtk::Widget>) {
        if let Some(s) = first_scroller(pane.upcast_ref()) {
            self.watch(&s);
        }
    }
}

/// A wheel or touchpad's step: a notch (`pixels` false) says which way at once; pixels add up,
/// and say once they've gone 24 one way, a turn the other way starting again.
fn moved(gone: &Cell<f64>, dy: f64, pixels: bool) -> Option<bool> {
    if dy == 0.0 {
        return None;
    }
    if !pixels {
        return Some(dy > 0.0);
    }
    let before = gone.get();
    let now = if before * dy < 0.0 { dy } else { before + dy };
    gone.set(now);
    (now.abs() > AFTER).then(|| {
        gone.set(0.0);
        now > 0.0
    })
}

/// A finger's drag `dy` from where it touched: past 24 one way says which, as the iPad's
/// `abs(drag.translation.height) > 24`, `fold(drag.translation.height < 0)`.
fn dragged(dy: f64) -> Option<bool> {
    (dy.abs() > AFTER).then_some(dy < 0.0)
}

fn first_scroller(w: &gtk::Widget) -> Option<gtk::ScrolledWindow> {
    if let Some(s) = w.downcast_ref::<gtk::ScrolledWindow>() {
        return Some(s.clone());
    }
    let mut c = w.first_child();
    while let Some(child) = c {
        if let Some(s) = first_scroller(&child) {
            return Some(s);
        }
        c = child.next_sibling();
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_notch_says_at_once_and_pixels_after_24() {
        let gone = Cell::new(0.0);
        assert_eq!(moved(&gone, 1.0, false), Some(true));
        assert_eq!(moved(&gone, -1.0, false), Some(false));
        assert_eq!(moved(&gone, 0.0, true), None);
        // A touchpad: 10 + 10 isn't yet 24; 10 more is.
        assert_eq!(moved(&gone, 10.0, true), None);
        assert_eq!(moved(&gone, 10.0, true), None);
        assert_eq!(moved(&gone, 10.0, true), Some(true));
        // Turning back starts again from nothing.
        assert_eq!(moved(&gone, 20.0, true), None);
        assert_eq!(moved(&gone, -20.0, true), None);
        assert_eq!(moved(&gone, -10.0, true), Some(false));
    }

    #[test]
    fn a_drag_folds_as_the_ipads() {
        assert_eq!(dragged(-25.0), Some(true));
        assert_eq!(dragged(25.0), Some(false));
        assert_eq!(dragged(-24.0), None);
        assert_eq!(dragged(10.0), None);
    }
}
