//! Coach mode's width (`CoachMode.swift` › `CoachPanel`): on a wide window, Coach slides in from
//! the right over whatever is open, which stays in view. It opens at a third of the window, or the
//! width last chosen: its left edge drags it wider, up to full screen, and back, never narrower
//! than a third. Dragged past that, it closes. ⤢ makes it full screen and brings it back to the
//! width it had. The choice is kept for the next time (`coach.width`, `coach.full`).

/// The narrowest it's kept, as a share of the window (`narrowest`).
pub const NARROWEST: f64 = 1.0 / 3.0;

/// Dragged this far under the narrowest, it closes.
const CLOSES_UNDER: f64 = 60.0;

/// Dragged to within this of the window's width, it's full screen.
const FULL_WITHIN: f64 = 40.0;

/// The width chosen: a share of the window, and whether it's full screen.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Width {
    pub share: f64,
    pub full: bool,
}

impl Default for Width {
    fn default() -> Width {
        Width {
            share: NARROWEST,
            full: false,
        }
    }
}

impl Width {
    /// Its width in a window `screen` wide, while its edge is dragged `drag` to the right
    /// (`min(screen, max(0, chosen - drag))`).
    pub fn on(&self, screen: f64, drag: f64) -> f64 {
        let chosen = if self.full {
            screen
        } else {
            screen * self.share
        };
        (chosen - drag).max(0.0).min(screen)
    }

    /// The drag ended `dx` to the right: the width it keeps, or none when it closes.
    pub fn dragged(&self, screen: f64, dx: f64) -> Option<Width> {
        let chosen = self.on(screen, 0.0) - dx;
        if chosen < screen * NARROWEST - CLOSES_UNDER {
            return None;
        }
        let full = chosen >= screen - FULL_WITHIN;
        Some(Width {
            share: if full || screen <= 0.0 {
                self.share
            } else {
                (chosen / screen).clamp(NARROWEST, 1.0)
            },
            full,
        })
    }

    /// ⤢: full screen, or back to the width it had.
    pub fn toggled(&self) -> Width {
        Width {
            full: !self.full,
            ..*self
        }
    }

    /// As kept between launches: "0.3333 full" or "0.5".
    pub fn kept(&self) -> String {
        if self.full {
            format!("{:.4} full", self.share)
        } else {
            format!("{:.4}", self.share)
        }
    }

    /// What was kept, or the default for anything else.
    pub fn read(kept: &str) -> Width {
        let mut parts = kept.split_whitespace();
        let share = parts
            .next()
            .and_then(|s| s.parse::<f64>().ok())
            .filter(|s| s.is_finite())
            .map(|s| s.clamp(NARROWEST, 1.0));
        let full = parts.next() == Some("full");
        match share {
            Some(share) if parts.next().is_none() => Width { share, full },
            _ => Width::default(),
        }
    }
}

/// A swipe of the conversation to the right closes Coach mode (`CoachPanel`'s
/// `DragGesture(minimumDistance: 30)`): over 140 points across and under 60 up or down, so a
/// scroll never does.
pub fn swipe_closes(dx: f64, dy: f64) -> bool {
    dx > 140.0 && dy.abs() < 60.0
}

/// Whether a touch's drag is a swipe across yet, not a scroll: 30 points to the right, more across
/// than up or down. Until it is, the conversation scrolls.
pub fn swiping(dx: f64, dy: f64) -> bool {
    dx >= 30.0 && dx > dy.abs() * 2.0 && dy.abs() < 60.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_swipe_to_the_right_closes_it_and_a_scroll_never_does() {
        assert!(swipe_closes(141.0, 0.0));
        assert!(swipe_closes(200.0, -59.0));
        assert!(!swipe_closes(140.0, 0.0));
        assert!(!swipe_closes(300.0, 60.0));
        assert!(!swipe_closes(-300.0, 0.0));
        // Taken as a swipe only once it's clearly across.
        assert!(!swiping(20.0, 0.0));
        assert!(swiping(30.0, 5.0));
        assert!(!swiping(40.0, 30.0));
        assert!(!swiping(-40.0, 0.0));
    }

    #[test]
    fn it_opens_at_a_third_and_follows_its_edge() {
        let w = Width::default();
        assert!((w.on(1200.0, 0.0) - 400.0).abs() < 1e-9);
        // Dragged left, wider; right, narrower; never past the window or under nothing.
        assert!((w.on(1200.0, -200.0) - 600.0).abs() < 1e-9);
        assert!((w.on(1200.0, 100.0) - 300.0).abs() < 1e-9);
        assert_eq!(w.on(1200.0, -5000.0), 1200.0);
        assert_eq!(w.on(1200.0, 5000.0), 0.0);
    }

    #[test]
    fn a_drag_keeps_its_width_goes_full_or_closes() {
        let w = Width::default();
        let wider = w.dragged(1200.0, -200.0).unwrap();
        assert!((wider.share - 0.5).abs() < 1e-9);
        assert!(!wider.full);
        // A little narrower than a third stays at a third.
        let narrow = w.dragged(1200.0, 50.0).unwrap();
        assert!((narrow.share - NARROWEST).abs() < 1e-9);
        // Past the narrowest by more than 60, it closes.
        assert_eq!(w.dragged(1200.0, 61.0), None);
        // Near the window's width, full screen, keeping the share it had.
        let full = w.dragged(1200.0, -780.0).unwrap();
        assert!(full.full);
        assert!((full.share - NARROWEST).abs() < 1e-9);
        // Full, dragged back: a share again.
        let back = full.dragged(1200.0, 300.0).unwrap();
        assert!(!back.full);
        assert!((back.share - 0.75).abs() < 1e-9);
    }

    #[test]
    fn full_screen_and_back_to_its_width() {
        let w = Width {
            share: 0.5,
            full: false,
        };
        let full = w.toggled();
        assert!(full.full);
        assert_eq!(full.on(1000.0, 0.0), 1000.0);
        assert_eq!(full.toggled(), w);
    }

    #[test]
    fn its_width_is_kept_and_read_back() {
        for w in [
            Width::default(),
            Width {
                share: 0.5,
                full: true,
            },
        ] {
            let r = Width::read(&w.kept());
            assert!((r.share - w.share).abs() < 1e-3);
            assert_eq!(r.full, w.full);
        }
        // Anything else is the default; a share out of bounds is held to them.
        for bad in ["", "wide", "NaN", "0.5 full extra", "inf"] {
            assert_eq!(Width::read(bad), Width::default(), "{bad}");
        }
        assert_eq!(Width::read("0.1").share, NARROWEST);
        assert_eq!(Width::read("3").share, 1.0);
    }
}
