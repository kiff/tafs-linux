//! The window's first size: sized to the screen near the iPad's landscape 4:3, as the iPad app is
//! seen, with 900×600 as its minimum only. A screenshot keeps its own fixed size.

/// The least the window can be made.
pub const MIN: (i32, i32) = (900, 600);

/// How much of the screen's height the window takes when it opens, and the most of its width.
const OF_HEIGHT: f64 = 0.85;
const OF_WIDTH: f64 = 0.9;

/// The window's first size on a screen of `width`×`height` (its work area, in the desktop's
/// pixels): 4:3, 85% of the screen's height, narrower when the screen is narrower than that, and
/// never under the minimum (a screen smaller than it gets the minimum).
pub fn first_size(width: i32, height: i32) -> (i32, i32) {
    let (w, h) = (f64::from(width), f64::from(height));
    let mut hh = h * OF_HEIGHT;
    let mut ww = hh * 4.0 / 3.0;
    if ww > w * OF_WIDTH {
        ww = w * OF_WIDTH;
        hh = ww * 3.0 / 4.0;
    }
    (
        (ww.round() as i32).max(MIN.0),
        (hh.round() as i32).max(MIN.1),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_opens_near_4_3_and_never_under_the_minimum() {
        // A 1080p laptop: 85% of its height, 4:3.
        assert_eq!(first_size(1920, 1080), (1224, 918));
        // A 1440p desktop.
        assert_eq!(first_size(2560, 1440), (1632, 1224));
        // A tall, narrow screen: the width bounds it, still 4:3.
        assert_eq!(first_size(1200, 1600), (1080, 810));
        // A small screen: the minimum.
        assert_eq!(first_size(1024, 640), (900, 600));
        // A 1366×768 laptop: 85% of its height is narrower than the minimum, so the minimum
        // width, at that height.
        assert_eq!(first_size(1366, 768), (900, 653));
        for (w, h) in [(1280, 800), (1920, 1200), (3840, 2160)] {
            let (a, b) = first_size(w, h);
            assert!(a >= MIN.0 && b >= MIN.1);
            // 4:3, unless a side is at its minimum.
            let four_three = (f64::from(a) / f64::from(b) - 4.0 / 3.0).abs() < 0.01;
            assert!(four_three || a == MIN.0 || b == MIN.1, "{w}x{h}: {a}x{b}");
        }
    }
}
