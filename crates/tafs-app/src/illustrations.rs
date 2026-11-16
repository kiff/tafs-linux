//! The approved first-time and empty-state drawings (`Illustration`), one per board, and the
//! Cairo pieces every drawing in the app is made of (the sign-in's too, `connected.rs`).
//!
//! Rule 14: shapes only, never a letter, a number or a word, so a drawing reads the same in every
//! language; the screen's own words say what it means. Each board is drawn on its own grid, in the
//! palette's colours for the appearance on screen (drawn again when it changes), and scaled down
//! to the width it's given, never wider than the iPad shows it and never out of its proportions.
//! A screen reader skips every one.

use std::cell::RefCell;
use std::f64::consts::PI;

use adw::prelude::*;
use gtk::cairo;
use gtk::glib;
use gtk::subclass::prelude::*;

pub(crate) fn rgb(cr: &cairo::Context, hex: u32, alpha: f64) {
    let (r, g, b) = channels(hex);
    cr.set_source_rgba(r, g, b, alpha);
}

fn channels(hex: u32) -> (f64, f64, f64) {
    (
        ((hex >> 16) & 0xFF) as f64 / 255.0,
        ((hex >> 8) & 0xFF) as f64 / 255.0,
        (hex & 0xFF) as f64 / 255.0,
    )
}

/// A palette colour in the appearance on screen.
pub(crate) fn tone(name: &str) -> u32 {
    let t = tafs_core::theme::tone(name).expect("the palette has it");
    if adw::StyleManager::default().is_dark() {
        t.dark.0
    } else {
        t.light.0
    }
}

pub(crate) fn rounded(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}

/// The brand's sweep (`Theme.brand`) over a square, from its top leading corner; `grey` takes
/// that much of its colour away (`grayscale`), 0 for none.
fn brand(x: f64, y: f64, side: f64, grey: f64) -> cairo::LinearGradient {
    let g = cairo::LinearGradient::new(x, y, x + side, y + side);
    for (i, hex) in tafs_core::theme::BRAND.iter().enumerate() {
        let (r, gr, b) = channels(*hex);
        let luma = 0.2126 * r + 0.7152 * gr + 0.0722 * b;
        let toward = |c: f64| c + (luma - c) * grey;
        g.add_color_stop_rgb(i as f64 / 2.0, toward(r), toward(gr), toward(b));
    }
    g
}

/// The app's mark as a gradient square: #B35C00, #E8003A, #2A64F5 from the top leading corner.
pub(crate) fn mark(cr: &cairo::Context, x: f64, y: f64, side: f64, r: f64) {
    greyed_mark(cr, x, y, side, r, 0.0);
}

fn greyed_mark(cr: &cairo::Context, x: f64, y: f64, side: f64, r: f64, grey: f64) {
    rounded(cr, x, y, side, side, r);
    let _ = cr.set_source(brand(x, y, side, grey));
    let _ = cr.fill();
}

pub(crate) fn capsule(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, hex: u32, alpha: f64) {
    rounded(cr, x, y, w, h, h / 2.0);
    rgb(cr, hex, alpha);
    let _ = cr.fill();
}

/// A shadow in a colour, `alpha` at its darkest: six widening outlines, `lift` below the shape.
fn glow(
    cr: &cairo::Context,
    (x, y, w, h, r): (f64, f64, f64, f64, f64),
    lift: f64,
    hex: u32,
    alpha: f64,
) {
    for i in 1..=6 {
        let s = i as f64 * 2.0;
        rounded(
            cr,
            x - s / 2.0,
            y + lift - s / 2.0,
            w + s,
            h + s,
            r + s / 2.0,
        );
        rgb(cr, hex, alpha / 6.0);
        let _ = cr.fill();
    }
}

pub(crate) fn area(
    width: i32,
    height: i32,
    draw: impl Fn(&cairo::Context) + 'static,
) -> gtk::DrawingArea {
    let a = gtk::DrawingArea::new();
    a.set_content_width(width);
    a.set_content_height(height);
    a.set_halign(gtk::Align::Center);
    // A drawing, not words: nothing for a screen reader to read.
    a.update_state(&[gtk::accessible::State::Hidden(true)]);
    a.set_draw_func(move |_, cr, _, _| draw(cr));
    a
}

// The boards' pieces, on a board's grid.

/// The soft amber glow behind a drawing, in a `size` square at `x`, `y`.
fn halo(cr: &cairo::Context, x: f64, y: f64, size: f64) {
    dot(cr, x, y, size, tone("amber"), 0.16);
}

/// A circle in a `size` square at `x`, `y`.
fn dot(cr: &cairo::Context, x: f64, y: f64, size: f64, hex: u32, alpha: f64) {
    cr.new_sub_path();
    cr.arc(x + size / 2.0, y + size / 2.0, size / 2.0, 0.0, 2.0 * PI);
    rgb(cr, hex, alpha);
    let _ = cr.fill();
}

/// A circle lifted off the screen by a shadow in a colour (`shadow(color:radius:y:)`).
fn lifted_dot(
    cr: &cairo::Context,
    (x, y, size): (f64, f64, f64),
    hex: u32,
    under: (u32, f64),
    lift: f64,
) {
    glow(cr, (x, y, size, size, size / 2.0), lift, under.0, under.1);
    dot(cr, x, y, size, hex, 1.0);
}

/// A line of a card, as a bar, 10 high unless told: never its words.
fn bar(cr: &cairo::Context, x: f64, y: f64, w: f64, alpha: f64) {
    capsule(cr, x, y, w, 10.0, tone("ink"), alpha);
}

/// A card's ground, lifted off the screen.
fn sheet(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    glow(cr, (x, y, w, h, r), 14.0, 0x000000, 0.14);
    rounded(cr, x, y, w, h, r);
    rgb(cr, tone("panel"), 1.0);
    let _ = cr.fill();
}

/// What `draw` draws, at `alpha` as one.
fn faded(cr: &cairo::Context, alpha: f64, draw: impl FnOnce()) {
    cr.push_group();
    draw();
    let _ = cr.pop_group_to_source();
    let _ = cr.paint_with_alpha(alpha);
}

/// What `draw` draws, turned `degrees` clockwise round the middle of `x`, `y`, `w`, `h`.
fn turned(
    cr: &cairo::Context,
    (x, y, w, h): (f64, f64, f64, f64),
    degrees: f64,
    draw: impl FnOnce(),
) {
    let _ = cr.save();
    cr.translate(x + w / 2.0, y + h / 2.0);
    cr.rotate(degrees * PI / 180.0);
    cr.translate(-(x + w / 2.0), -(y + h / 2.0));
    draw();
    let _ = cr.restore();
}

/// A small tilted square, a sparkle.
fn chip(cr: &cairo::Context, (x, y, size): (f64, f64, f64), hex: u32, alpha: f64, degrees: f64) {
    turned(cr, (x, y, size, size), degrees, || {
        rounded(cr, x, y, size, size, 4.0);
        rgb(cr, hex, alpha);
        let _ = cr.fill();
    });
}

/// A check, drawn on a 24 by 24 grid scaled to `size` at `x`, `y`, in the colour on a fill.
fn tick(cr: &cairo::Context, x: f64, y: f64, size: f64, width: f64) {
    let p = |a: f64, b: f64| (x + a * size / 24.0, y + b * size / 24.0);
    let (a, b, c) = (p(5.0, 12.5), p(9.5, 17.0), p(19.0, 7.5));
    cr.move_to(a.0, a.1);
    cr.line_to(b.0, b.1);
    cr.line_to(c.0, c.1);
    rgb(cr, tone("on_fill"), 1.0);
    cr.set_line_width(width);
    cr.set_line_cap(cairo::LineCap::Round);
    cr.set_line_join(cairo::LineJoin::Round);
    let _ = cr.stroke();
}

/// One of the overview's cards at `x`, `y`: a badge and two lines.
fn tile(
    cr: &cairo::Context,
    x: f64,
    y: f64,
    badge: impl FnOnce(f64, f64),
    first: f64,
    second: f64,
) {
    sheet(cr, x, y, 260.0, 150.0, 22.0);
    let (x, y) = (x + 22.0, y + 22.0);
    badge(x, y);
    bar(cr, x, y + 38.0 + 20.0, first, 0.75);
    bar(cr, x, y + 38.0 + 20.0 + 10.0 + 10.0, second, 0.15);
}

/// The overview's gentle rise, on a 374 by 160 grid at `x`, `y`; closed, the area under it.
fn rise(cr: &cairo::Context, x: f64, y: f64, closed: bool) {
    cr.move_to(x, y + 130.0);
    cr.curve_to(x + 50.0, y + 120.0, x + 70.0, y + 80.0, x + 120.0, y + 92.0);
    cr.curve_to(
        x + 170.0,
        y + 104.0,
        x + 200.0,
        y + 40.0,
        x + 250.0,
        y + 60.0,
    );
    cr.curve_to(
        x + 300.0,
        y + 80.0,
        x + 330.0,
        y + 20.0,
        x + 374.0,
        y + 26.0,
    );
    if closed {
        cr.line_to(x + 374.0, y + 160.0);
        cr.line_to(x, y + 160.0);
        cr.close_path();
    }
}

mod imp {
    use super::*;

    pub type Draw = Box<dyn Fn(&cairo::Context)>;

    /// `Board`: a drawing on a `width` by `height` grid, at most `shown` wide.
    #[derive(Default)]
    pub struct Board {
        pub grid: std::cell::Cell<(f64, f64, f64)>,
        pub draw: RefCell<Option<Draw>>,
        /// Whether it gives up height on a short screen (`Board.shrinking`).
        pub shrinks: std::cell::Cell<bool>,
        /// At the leading edge of its row, not in its middle (`Board.leading`).
        pub leading: std::cell::Cell<bool>,
        /// Its redraw when the appearance changes, let go with it.
        pub dark: RefCell<Option<glib::SignalHandlerId>>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Board {
        const NAME: &'static str = "TafsBoard";
        type Type = super::Board;
        type ParentType = gtk::Widget;
    }

    impl ObjectImpl for Board {
        fn dispose(&self) {
            if let Some(id) = self.dark.take() {
                adw::StyleManager::default().disconnect(id);
            }
        }
    }

    impl WidgetImpl for Board {
        fn request_mode(&self) -> gtk::SizeRequestMode {
            gtk::SizeRequestMode::HeightForWidth
        }

        /// As wide as it's shown on the iPad, narrower when there's less room; as high as that
        /// width gives it, or less when it shrinks.
        fn measure(&self, orientation: gtk::Orientation, for_size: i32) -> (i32, i32, i32, i32) {
            let (w, h, shown) = self.grid.get();
            if orientation == gtk::Orientation::Horizontal {
                return (0, shown.ceil() as i32, -1, -1);
            }
            let width = if for_size >= 0 {
                f64::from(for_size).min(shown)
            } else {
                shown
            };
            let height = (width * h / w).ceil() as i32;
            (if self.shrinks.get() { 0 } else { height }, height, -1, -1)
        }

        fn snapshot(&self, snapshot: &gtk::Snapshot) {
            let (w, h, shown) = self.grid.get();
            let obj = self.obj();
            let (room_w, room_h) = (f64::from(obj.width()), f64::from(obj.height()));
            let scale = (room_w / w).min(room_h / h).min(shown / w);
            if scale <= 0.0 {
                return;
            }
            // In the middle of the room it's given. A shadow or a sparkle may reach past the
            // board's edge, as on the iPad, which doesn't clip it: room for that all round.
            let (dw, dh) = (w * scale, h * scale);
            let x = if self.leading.get() {
                0.0
            } else {
                (room_w - dw) / 2.0
            };
            let y = (room_h - dh) / 2.0;
            let spill = 24.0 * scale;
            let cr = snapshot.append_cairo(&gtk::graphene::Rect::new(
                (x - spill) as f32,
                (y - spill) as f32,
                (dw + 2.0 * spill) as f32,
                (dh + 2.0 * spill) as f32,
            ));
            cr.translate(x, y);
            cr.scale(scale, scale);
            if let Some(draw) = self.draw.borrow().as_ref() {
                draw(&cr);
            }
        }
    }
}

glib::wrapper! {
    /// A board (`Board`): its drawing scaled to fit, hidden from a screen reader.
    pub struct Board(ObjectSubclass<imp::Board>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Board {
    /// On a screen that doesn't scroll: the drawing gets smaller when the window is short,
    /// rather than push the screen's words and buttons off it (`OnboardingView`).
    pub fn shrinking(self) -> Self {
        self.imp().shrinks.set(true);
        self
    }

    /// At the leading edge of its row, as a board in a column aligned to its leading edge.
    pub fn leading(self) -> Self {
        self.imp().leading.set(true);
        self
    }
}

/// A drawing on its board's grid (`width` by `height` points), scaled to fit the width it's given
/// and never wider than `shown`, so a narrow window gets a smaller drawing; in the middle of its
/// row.
fn board(width: f64, height: f64, shown: f64, draw: impl Fn(&cairo::Context) + 'static) -> Board {
    let b: Board = glib::Object::new();
    b.imp().grid.set((width, height, shown));
    *b.imp().draw.borrow_mut() = Some(Box::new(draw));
    b.set_halign(gtk::Align::Fill);
    b.set_hexpand(true);
    // A drawing, not words: nothing for a screen reader to read.
    b.update_state(&[gtk::accessible::State::Hidden(true)]);
    // In the colours of the appearance on screen, again when it changes.
    let weak = b.downgrade();
    let id = adw::StyleManager::default().connect_dark_notify(move |_| {
        if let Some(b) = weak.upgrade() {
            b.queue_draw();
        }
    });
    *b.imp().dark.borrow_mut() = Some(id);
    b
}

/// Board 1, first run: a card opening, sparkles round it. The welcome.
pub fn first_run() -> Board {
    board(380.0, 335.0, 200.0, |cr| {
        halo(cr, 40.0, 10.0, 300.0);
        // The card, 200 × 250, its pieces 24 in.
        sheet(cr, 90.0, 40.0, 200.0, 250.0, 22.0);
        let (x, y) = (114.0, 64.0);
        mark(cr, x, y, 52.0, 12.0);
        bar(cr, x, y + 74.0, 120.0, 0.85);
        bar(cr, x, y + 96.0, 90.0, 0.18);
        bar(cr, x, y + 118.0, 140.0, 0.18);
        capsule(cr, x, y + 168.0, 152.0, 40.0, tone("amber"), 1.0);
        let amber_text = tone("amber_text");
        dot(cr, 10.0, 20.0, 18.0, amber_text, 0.6);
        chip(cr, (350.0, 60.0, 16.0), tone("blue"), 0.55, 20.0);
        chip(cr, (30.0, 310.0, 14.0), tone("pink"), 0.5, -15.0);
        dot(cr, 330.0, 320.0, 20.0, amber_text, 0.35);
    })
}

/// Board 2, no tafs yet: an empty card's outline, a plus in it. Home with no taf.
pub fn no_tafs() -> Board {
    board(360.0, 320.0, 200.0, |cr| {
        halo(cr, 30.0, 10.0, 300.0);
        // The outline, 228 × 208, its 4-wide dashes inside its edge.
        rounded(cr, 72.0, 62.0, 224.0, 204.0, 22.0);
        rgb(cr, tone("ink"), 0.35);
        cr.set_line_width(4.0);
        cr.set_dash(&[10.0, 7.0], 0.0);
        let _ = cr.stroke();
        cr.set_dash(&[], 0.0);
        lifted_dot(
            cr,
            (148.0, 128.0, 64.0),
            tone("amber"),
            (tone("amber_text"), 0.35),
            10.0,
        );
        let plus = tone("amber_ink");
        capsule(cr, 177.0, 142.0, 6.0, 36.0, plus, 1.0);
        capsule(cr, 162.0, 157.0, 36.0, 6.0, plus, 1.0);
        for x in [10.0, 230.0] {
            faded(cr, 0.6, || sheet(cr, x, 280.0, 120.0, 28.0, 14.0));
        }
    })
}

/// Board 4, every taf halted: cards at rest under a pause. Halt all, once halted.
pub fn all_halted() -> Board {
    board(330.0, 320.0, 190.0, |cr| {
        let ink = tone("ink");
        dot(cr, 15.0, 10.0, 300.0, ink, 0.06);
        for (x, y, degrees) in [(15.0, 90.0, -6.0), (165.0, 80.0, 5.0)] {
            faded(cr, 0.75, || {
                turned(cr, (x, y, 150.0, 190.0), degrees, || {
                    sheet(cr, x, y, 150.0, 190.0, 22.0)
                })
            });
        }
        // The card in front, 150 × 200, its pieces 20 in, its mark greyed.
        sheet(cr, 90.0, 65.0, 150.0, 200.0, 22.0);
        let (x, y) = (110.0, 85.0);
        greyed_mark(cr, x, y, 40.0, 12.0, 0.7);
        bar(cr, x, y + 58.0, 90.0, 0.3);
        bar(cr, x, y + 78.0, 70.0, 0.15);
        // The pause, on an ink circle.
        lifted_dot(cr, (205.0, 30.0, 76.0), ink, (0x000000, 0.3), 12.0);
        let on_fill = tone("on_fill");
        for x in [229.0, 247.0] {
            rounded(cr, x, 50.0, 10.0, 36.0, 3.0);
            rgb(cr, on_fill, 1.0);
            let _ = cr.fill();
        }
    })
}

/// Board 6, nothing waiting: an empty tray under a calm check. Needs you, empty.
pub fn nothing_waiting() -> Board {
    board(380.0, 325.0, 200.0, |cr| {
        halo(cr, 35.0, 10.0, 300.0);
        sheet(cr, 55.0, 160.0, 260.0, 150.0, 26.0);
        let ok = tone("ok");
        lifted_dot(cr, (145.0, 60.0, 80.0), ok, (ok, 0.35), 12.0);
        tick(cr, 165.0, 80.0, 40.0, 5.0);
        bar(cr, 115.0, 240.0, 140.0, 0.12);
        bar(cr, 140.0, 262.0, 90.0, 0.08);
        dot(cr, 345.0, 90.0, 16.0, tone("amber_text"), 0.4);
        dot(cr, 13.0, 210.0, 12.0, tone("blue"), 0.35);
        dot(cr, 360.0, 260.0, 10.0, tone("pink"), 0.35);
    })
}

/// Board 10, the overview: a ring, a gentle line and cards in a row. The Overview's header.
pub fn overview() -> Board {
    board(860.0, 480.0, 340.0, |cr| {
        let ink = tone("ink");
        // The ring's card: 120 across, two thirds of it in the brand's sweep, a line under it.
        sheet(cr, 20.0, 20.0, 360.0, 260.0, 22.0);
        let (cx, cy) = (200.0, 129.0);
        cr.new_sub_path();
        cr.arc(cx, cy, 60.0, 0.0, 2.0 * PI);
        rgb(cr, ink, 0.07);
        cr.set_line_width(16.0);
        let _ = cr.stroke();
        cr.new_sub_path();
        cr.arc(cx, cy, 60.0, -PI / 2.0, -PI / 2.0 + 0.68 * 2.0 * PI);
        let _ = cr.set_source(brand(cx - 60.0, cy - 60.0, 120.0, 0.0));
        cr.set_line_cap(cairo::LineCap::Round);
        let _ = cr.stroke();
        bar(cr, 130.0, 226.0, 140.0, 0.15);
        // The line's card: a title's bar, the rise and its area, a dot on it.
        sheet(cr, 410.0, 20.0, 430.0, 260.0, 22.0);
        bar(cr, 438.0, 48.0, 120.0, 0.8);
        let (x, y) = (438.0, 82.0);
        let amber = tone("amber");
        rise(cr, x, y, true);
        rgb(cr, amber, 0.12);
        let _ = cr.fill();
        rise(cr, x, y, false);
        rgb(cr, amber, 1.0);
        cr.set_line_width(6.0);
        let _ = cr.stroke();
        dot(cr, x + 240.0, y + 50.0, 20.0, tone("amber_text"), 1.0);
        // Three cards in a row.
        tile(
            cr,
            20.0,
            310.0,
            |x, y| mark(cr, x, y, 38.0, 12.0),
            140.0,
            100.0,
        );
        tile(
            cr,
            300.0,
            310.0,
            |x, y| dot(cr, x, y, 38.0, tone("ok"), 1.0),
            120.0,
            150.0,
        );
        tile(
            cr,
            580.0,
            310.0,
            |x, y| dot(cr, x, y, 38.0, tone("blue"), 0.85),
            130.0,
            90.0,
        );
    })
}

/// Coach's spark: four curved points, on a 100 by 100 grid scaled to `size` at `x`, `y`.
fn spark(cr: &cairo::Context, x: f64, y: f64, size: f64) {
    let p = |a: f64, b: f64| (x + a * size / 100.0, y + b * size / 100.0);
    let curve = |c1: (f64, f64), c2: (f64, f64), to: (f64, f64)| {
        cr.curve_to(c1.0, c1.1, c2.0, c2.1, to.0, to.1);
    };
    let start = p(50.0, 0.0);
    cr.move_to(start.0, start.1);
    curve(p(54.0, 34.0), p(66.0, 46.0), p(100.0, 50.0));
    curve(p(66.0, 54.0), p(54.0, 66.0), p(50.0, 100.0));
    curve(p(46.0, 66.0), p(34.0, 54.0), p(0.0, 50.0));
    curve(p(34.0, 46.0), p(46.0, 34.0), start);
    cr.close_path();
}

/// The spark in the brand's sweep.
fn brand_spark(cr: &cairo::Context, x: f64, y: f64, size: f64) {
    spark(cr, x, y, size);
    let _ = cr.set_source(brand(x, y, size, 0.0));
    let _ = cr.fill();
}

/// A ring round `cx`, `cy`: its track, then `filled` of it from the top, clockwise, in `hex`.
fn ring(
    cr: &cairo::Context,
    (cx, cy, r, width): (f64, f64, f64, f64),
    track: f64,
    filled: f64,
    (hex, alpha): (u32, f64),
) {
    let ink = tone("ink");
    cr.new_sub_path();
    cr.arc(cx, cy, r, 0.0, 2.0 * PI);
    rgb(cr, ink, track);
    cr.set_line_width(width);
    let _ = cr.stroke();
    cr.new_sub_path();
    cr.arc(cx, cy, r, -PI / 2.0, -PI / 2.0 + filled * 2.0 * PI);
    rgb(cr, hex, alpha);
    cr.set_line_cap(cairo::LineCap::Round);
    let _ = cr.stroke();
    cr.set_line_cap(cairo::LineCap::Butt);
}

/// One of Coach's first steps at `x`, `y`, under its rings: a dot and a line, 18 in.
fn step(cr: &cairo::Context, x: f64, y: f64, (hex, alpha): (u32, f64), line: f64) {
    sheet(cr, x, y, 170.0, 110.0, 22.0);
    dot(cr, x + 18.0, y + 18.0, 30.0, hex, alpha);
    bar(cr, x + 18.0, y + 64.0, 110.0, line);
}

/// Board 7, Coach with no taf or one: its spark beside one card, room for more. Coach's empty chat,
/// at its leading edge.
pub fn coach_first() -> Board {
    board(610.0, 280.0, 300.0, |cr| {
        halo(cr, 10.0, 10.0, 260.0);
        brand_spark(cr, 70.0, 70.0, 140.0);
        spark(cr, 215.0, 40.0, 40.0);
        rgb(cr, tone("amber"), 0.6);
        let _ = cr.fill();
        // The card, 150 × 180, its pieces 20 in.
        sheet(cr, 310.0, 80.0, 150.0, 180.0, 22.0);
        let (x, y) = (330.0, 100.0);
        mark(cr, x, y, 40.0, 12.0);
        bar(cr, x, y + 58.0, 90.0, 0.85);
        bar(cr, x, y + 78.0, 70.0, 0.18);
        capsule(cr, x, y + 118.0, 110.0, 30.0, tone("amber"), 0.25);
        // Room for the next: a dashed outline, its 4-wide dashes inside its edge.
        rounded(cr, 482.0, 97.0, 114.0, 154.0, 20.0);
        rgb(cr, tone("ink"), 0.22);
        cr.set_line_width(4.0);
        cr.set_dash(&[10.0, 7.0], 0.0);
        let _ = cr.stroke();
        // The dotted way from the spark to the card.
        cr.move_to(250.0, 170.5);
        cr.line_to(300.0, 170.5);
        rgb(cr, tone("amber"), 0.7);
        cr.set_line_width(5.0);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_dash(&[0.0, 9.0], 0.0);
        let _ = cr.stroke();
        cr.set_dash(&[], 0.0);
        cr.set_line_cap(cairo::LineCap::Butt);
    })
}

/// Board 9, a wide window: Coach starting for the first time, its spark in rings filling. Coach
/// before it's started.
pub fn coach_starting() -> Board {
    board(600.0, 490.0, 300.0, |cr| {
        ring(
            cr,
            (300.0, 160.0, 140.0, 10.0),
            0.07,
            0.625,
            (tone("amber"), 1.0),
        );
        ring(
            cr,
            (300.0, 160.0, 108.0, 8.0),
            0.06,
            0.38,
            (tone("blue"), 0.7),
        );
        brand_spark(cr, 240.0, 100.0, 120.0);
        faded(cr, 0.9, || step(cr, 20.0, 360.0, (tone("ok"), 1.0), 0.15));
        step(cr, 215.0, 360.0, (tone("amber"), 1.0), 0.15);
        faded(cr, 0.55, || {
            step(cr, 410.0, 360.0, (tone("ink"), 0.15), 0.1)
        });
        dot(cr, 100.0, 70.0, 14.0, tone("pink"), 0.4);
        dot(cr, 490.0, 50.0, 18.0, tone("amber_text"), 0.4);
    })
}
