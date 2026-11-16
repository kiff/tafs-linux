//! The sign-in's drawings (`SignInArt`), in shapes only, never a letter, a number or a word
//! (rule 14), and the celebration after signing in either way (`ConnectedView`): the computer and
//! the phone, joined; `sign_in.connected` and `sign_in.connected_line`; Let's go.

use std::f64::consts::PI;
use std::rc::Rc;

use adw::prelude::*;
use gtk::cairo;
use tafs_core::words::WordBook;

use crate::widgets::{primary, text};

fn rgb(cr: &cairo::Context, hex: u32, alpha: f64) {
    let (r, g, b) = (
        ((hex >> 16) & 0xFF) as f64 / 255.0,
        ((hex >> 8) & 0xFF) as f64 / 255.0,
        (hex & 0xFF) as f64 / 255.0,
    );
    cr.set_source_rgba(r, g, b, alpha);
}

/// A palette colour in the appearance on screen.
fn tone(name: &str) -> u32 {
    let t = tafs_core::theme::tone(name).expect("the palette has it");
    if adw::StyleManager::default().is_dark() {
        t.dark.0
    } else {
        t.light.0
    }
}

fn rounded(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64) {
    let r = r.min(w / 2.0).min(h / 2.0);
    cr.new_sub_path();
    cr.arc(x + w - r, y + r, r, -PI / 2.0, 0.0);
    cr.arc(x + w - r, y + h - r, r, 0.0, PI / 2.0);
    cr.arc(x + r, y + h - r, r, PI / 2.0, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * PI / 2.0);
    cr.close_path();
}

/// The app's mark as a gradient square: #B35C00, #E8003A, #2A64F5 from the top leading corner.
fn mark(cr: &cairo::Context, x: f64, y: f64, side: f64, r: f64) {
    let g = cairo::LinearGradient::new(x, y, x + side, y + side);
    g.add_color_stop_rgb(0.0, 0xB3 as f64 / 255.0, 0x5C as f64 / 255.0, 0.0);
    g.add_color_stop_rgb(0.5, 0xE8 as f64 / 255.0, 0.0, 0x3A as f64 / 255.0);
    g.add_color_stop_rgb(
        1.0,
        0x2A as f64 / 255.0,
        0x64 as f64 / 255.0,
        0xF5 as f64 / 255.0,
    );
    rounded(cr, x, y, side, side, r);
    let _ = cr.set_source(&g);
    let _ = cr.fill();
}

fn capsule(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, hex: u32, alpha: f64) {
    rounded(cr, x, y, w, h, h / 2.0);
    rgb(cr, hex, alpha);
    let _ = cr.fill();
}

/// A soft shadow under a rounded shape: a few widening outlines at low opacity.
fn shadow(cr: &cairo::Context, x: f64, y: f64, w: f64, h: f64, r: f64, lift: f64) {
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
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.025);
        let _ = cr.fill();
    }
}

fn area(width: i32, height: i32, draw: impl Fn(&cairo::Context) + 'static) -> gtk::DrawingArea {
    let a = gtk::DrawingArea::new();
    a.set_content_width(width);
    a.set_content_height(height);
    a.set_halign(gtk::Align::Center);
    // A drawing, not words: nothing for a screen reader to read.
    a.update_state(&[gtk::accessible::State::Hidden(true)]);
    a.set_draw_func(move |_, cr, _, _| draw(cr));
    a
}

/// `SignInArt.kiff`: KIFF's sign-in, a card with the app's mark, two lines and its button, under a
/// lock, on a soft amber circle.
pub fn kiff_art() -> gtk::DrawingArea {
    area(200, 200, |cr| {
        let (ink, amber) = (tone("ink"), tone("amber"));
        // The circle, 190 across.
        cr.arc(100.0, 100.0, 95.0, 0.0, 2.0 * PI);
        rgb(cr, amber, 0.16);
        let _ = cr.fill();
        // The card, 122 × 156, from the top trailing corner moved 34 in and 20 down.
        let (x, y, w, h) = (200.0 - 122.0 - 34.0, 20.0, 122.0, 156.0);
        shadow(cr, x, y, w, h, 22.0, 10.0);
        rounded(cr, x, y, w, h, 22.0);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let _ = cr.fill();
        mark(cr, x + 16.0, y + 16.0, 34.0, 10.0);
        capsule(cr, x + 16.0, y + 60.0, 78.0, 8.0, 0x1B1726, 0.85);
        capsule(cr, x + 16.0, y + 78.0, 58.0, 8.0, 0x1B1726, 0.18);
        capsule(
            cr,
            x + 16.0,
            y + h - 16.0 - 26.0,
            w - 32.0,
            26.0,
            amber,
            1.0,
        );
        // The lock, on a white circle at the top trailing corner.
        let (cx, cy) = (175.0, 25.0);
        shadow(cr, cx - 25.0, cy - 25.0, 50.0, 50.0, 25.0, 6.0);
        cr.arc(cx, cy, 25.0, 0.0, 2.0 * PI);
        cr.set_source_rgb(1.0, 1.0, 1.0);
        let _ = cr.fill();
        let _ = ink;
        rgb(cr, 0x1B1726, 1.0);
        cr.set_line_width(2.0);
        rounded(cr, cx - 8.0, cy - 2.0, 16.0, 12.0, 2.5);
        let _ = cr.stroke();
        cr.new_sub_path();
        cr.arc(cx, cy - 4.0, 4.5, PI, 0.0);
        cr.line_to(cx + 4.5, cy - 2.0);
        cr.move_to(cx - 4.5, cy - 2.0);
        cr.line_to(cx - 4.5, cy - 4.0);
        let _ = cr.stroke();
    })
}

/// `SignInArt.connected`: the computer and the phone, a check between them.
pub fn connected_art() -> gtk::DrawingArea {
    area(360, 170, |cr| {
        let (ink, check) = (tone("ink"), tone("amber_text"));
        // The computer, 200 × 136, and the phone, 80 × 156, 40 apart, on one baseline.
        let base = 165.0;
        let (cx, cy, cw, ch) = (20.0, base - 136.0, 200.0, 136.0);
        let (px, py, pw, ph) = (cx + cw + 40.0, base - 156.0, 80.0, 156.0);
        for (x, y, w, h, r) in [(cx, cy, cw, ch, 16.0), (px, py, pw, ph, 22.0)] {
            rounded(cr, x, y, w, h, r);
            cr.set_source_rgb(1.0, 1.0, 1.0);
            let _ = cr.fill_preserve();
            rgb(cr, ink, 1.0);
            cr.set_line_width(3.0);
            let _ = cr.stroke();
        }
        mark(cr, cx + 16.0, cy + 16.0, 30.0, 9.0);
        capsule(cr, cx + 16.0, cy + 54.0, 110.0, 8.0, 0x1B1726, 0.8);
        capsule(cr, cx + 16.0, cy + 70.0, 80.0, 8.0, 0x1B1726, 0.18);
        // The check, 52 across, 60 right of the middle.
        let (kx, ky) = (180.0 + 60.0, base - 78.0);
        for i in 1..=5 {
            cr.arc(kx, ky + 8.0, 26.0 + i as f64 * 2.0, 0.0, 2.0 * PI);
            rgb(cr, tone("amber"), 0.05);
            let _ = cr.fill();
        }
        cr.arc(kx, ky, 26.0, 0.0, 2.0 * PI);
        rgb(cr, check, 1.0);
        let _ = cr.fill();
        cr.set_source_rgb(1.0, 1.0, 1.0);
        cr.set_line_width(4.0);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);
        cr.move_to(kx - 10.0, ky + 1.0);
        cr.line_to(kx - 3.0, ky + 8.0);
        cr.line_to(kx + 11.0, ky - 8.0);
        let _ = cr.stroke();
    })
}

/// `ConnectedView`, over the window after signing in either way: the drawing, the two lines (30
/// bold), Let's go. Closing it is Let's go.
pub fn present(parent: &impl IsA<gtk::Widget>, w: &Rc<WordBook>) {
    let column = gtk::Box::new(gtk::Orientation::Vertical, 32);
    column.set_margin_start(32);
    column.set_margin_end(32);
    column.set_margin_top(48);
    column.set_margin_bottom(48);
    column.set_valign(gtk::Align::Center);
    column.append(&connected_art());
    let lines = gtk::Box::new(gtk::Orientation::Vertical, 6);
    for key in ["sign_in.connected", "sign_in.connected_line"] {
        let l = text(&w.line(key).unwrap_or_default(), 30, 700, &["tafs-ink"]);
        l.set_xalign(0.5);
        l.set_justify(gtk::Justification::Center);
        lines.append(&l);
    }
    column.append(&lines);
    let go = primary(&w.line("sign_in.lets_go").unwrap_or_default());
    go.set_widget_name("signIn.letsGo");
    go.set_halign(gtk::Align::Center);
    go.set_size_request(320, -1);
    go.set_hexpand(false);
    column.append(&go);
    // The screen's own background, edge to edge, the drawing and the words in its middle.
    let screen = gtk::Box::new(gtk::Orientation::Vertical, 0);
    screen.set_vexpand(true);
    screen.append(&column);
    column.set_vexpand(true);
    crate::widgets::ground(&screen);
    let dialog = adw::Dialog::new();
    dialog.set_child(Some(&screen));
    dialog.set_content_width(640);
    dialog.set_content_height(560);
    {
        let dialog = dialog.clone();
        go.connect_clicked(move |_| {
            dialog.close();
        });
    }
    dialog.present(Some(parent));
}
