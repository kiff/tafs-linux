//! The celebration after signing in either way (`ConnectedView`): its drawing
//! (`SignInArt.connected`) in shapes only, never a letter, a number or a word (rule 14), the
//! computer and the phone joined; `sign_in.connected` and `sign_in.connected_line`; Let's go.

use std::f64::consts::PI;
use std::rc::Rc;

use adw::prelude::*;
use gtk::cairo;
use tafs_core::words::WordBook;

use crate::illustrations::{area, capsule, mark, rgb, rounded, tone};
use crate::widgets::{primary, text};

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
