//! `DoorRim`: a taf with a door open has an arc travelling round its avatar's edge, as tafs cloud's
//! Home gives it (`avatar_rim`: `{visible, tint, length}`, `home.ts` › `avatarRim`). Amber while a
//! knock waits at one of its doors, dim while every open door rests, else the taf's own colour; its
//! length grows with the doors open, up to 0.6 of the edge.

use serde_json::Value;

/// The arc's colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tint {
    /// A knock waits at one of its doors: the amber.
    Knock,
    /// Every open door is resting: the dim colour.
    Resting,
    /// The taf's own colour.
    Active,
}

/// A rim to draw: its tint, and how much of the edge it covers, in thousandths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rim {
    pub tint: Tint,
    pub length: u16,
}

/// tafs cloud's `avatar_rim`, when it's to be drawn: `visible`, a length above nothing and at
/// most the whole edge. Anything else, or none sent, is no rim.
pub fn read(v: &Value) -> Option<Rim> {
    let o = v.as_object()?;
    if o.get("visible").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let length = o.get("length").and_then(Value::as_f64)?;
    if !(length > 0.0 && length <= 1.0) {
        return None;
    }
    let tint = match o.get("tint").and_then(Value::as_str) {
        Some("knock") => Tint::Knock,
        Some("resting") => Tint::Resting,
        _ => Tint::Active,
    };
    Some(Rim {
        tint,
        length: (length * 1000.0).round() as u16,
    })
}

/// The point `t` (0 to 1) of the way round a rounded rectangle's edge, `w` by `h` with corners of
/// `r`, clockwise from the top edge's start after its left corner.
pub fn point(w: f64, h: f64, r: f64, t: f64) -> (f64, f64) {
    let r = r.clamp(0.0, w.min(h) / 2.0);
    let (sw, sh) = (w - 2.0 * r, h - 2.0 * r);
    let quarter = std::f64::consts::FRAC_PI_2 * r;
    let total = 2.0 * (sw + sh) + 4.0 * quarter;
    let mut d = t.rem_euclid(1.0) * total;
    // Each stretch: a straight edge, then the corner after it.
    let corner = |cx: f64, cy: f64, from: f64, d: f64| {
        let a = from + d / r.max(f64::EPSILON);
        (cx + r * a.cos(), cy + r * a.sin())
    };
    use std::f64::consts::{FRAC_PI_2, PI};
    if d <= sw {
        return (r + d, 0.0);
    }
    d -= sw;
    if d <= quarter {
        return corner(w - r, r, -FRAC_PI_2, d);
    }
    d -= quarter;
    if d <= sh {
        return (w, r + d);
    }
    d -= sh;
    if d <= quarter {
        return corner(w - r, h - r, 0.0, d);
    }
    d -= quarter;
    if d <= sw {
        return (w - r - d, h);
    }
    d -= sw;
    if d <= quarter {
        return corner(r, h - r, FRAC_PI_2, d);
    }
    d -= quarter;
    if d <= sh {
        return (0.0, h - r - d);
    }
    d -= sh;
    corner(r, r, PI, d.min(quarter))
}

/// The arc as points along the edge, from `phase` round for `length` of it (`PerimeterArc`):
/// past the start, it goes on from the beginning.
pub fn arc(w: f64, h: f64, r: f64, phase: f64, length: f64) -> Vec<(f64, f64)> {
    let steps = ((length * 240.0).ceil() as usize).max(2);
    (0..=steps)
        .map(|i| point(w, h, r, phase + length * i as f64 / steps as f64))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rim_is_drawn_only_as_tafs_cloud_gives_it() {
        let rim = |v: Value| read(&v);
        assert_eq!(
            rim(serde_json::json!({"visible": true, "tint": "knock", "length": 0.34})),
            Some(Rim {
                tint: Tint::Knock,
                length: 340
            })
        );
        assert_eq!(
            rim(serde_json::json!({"visible": true, "tint": "resting", "length": 0.22}))
                .map(|r| r.tint),
            Some(Tint::Resting)
        );
        assert_eq!(
            rim(serde_json::json!({"visible": true, "tint": "active", "length": 0.6}))
                .map(|r| r.tint),
            Some(Tint::Active)
        );
        // Not visible, no length, a length out of range, or not the shape: none.
        assert_eq!(
            rim(serde_json::json!({"visible": false, "tint": "active", "length": 0.2})),
            None
        );
        assert_eq!(
            rim(serde_json::json!({"visible": true, "tint": "active"})),
            None
        );
        assert_eq!(
            rim(serde_json::json!({"visible": true, "length": 0.0})),
            None
        );
        assert_eq!(
            rim(serde_json::json!({"visible": true, "length": 1.5})),
            None
        );
        assert_eq!(rim(serde_json::json!("yes")), None);
        assert_eq!(rim(Value::Null), None);
    }

    #[test]
    fn the_arc_runs_along_the_edge_and_over_its_start() {
        let (w, h, r) = (42.0, 42.0, 14.0);
        let on_edge = |(x, y): (f64, f64)| {
            // Inside the corners' squares, at r from their centre; else on a straight side.
            let cx = x.clamp(r, w - r);
            let cy = y.clamp(r, h - r);
            ((x - cx).hypot(y - cy) - r).abs() < 1e-6
                || ((x - cx).abs() < 1e-9
                    && (y - cy).abs() < 1e-9
                    && (x == 0.0 || x == w || y == 0.0 || y == h))
        };
        for i in 0..=100 {
            let p = point(w, h, r, i as f64 / 100.0);
            assert!(on_edge(p), "{i}: {p:?}");
        }
        // It starts after the top-left corner and comes back to it.
        assert_eq!(point(w, h, r, 0.0), (r, 0.0));
        let end = point(w, h, r, 1.0);
        assert!((end.0 - r).abs() < 1e-6 && end.1.abs() < 1e-6, "{end:?}");
        // Past the start, it goes on from the beginning.
        let a = arc(w, h, r, 0.9, 0.3);
        assert!(a.len() > 2);
        let last = *a.last().unwrap();
        let wrapped = point(w, h, r, 0.2);
        assert!((last.0 - wrapped.0).abs() < 1e-6 && (last.1 - wrapped.1).abs() < 1e-6);
    }
}
