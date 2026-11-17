//! The taf's dial (`TafDialPanel.swift`, `TafKnobView.swift`, `TafKnobRemote.swift`): When, What
//! and Where around its Why, today's work on the rim, and the remote under it (Back, Lower, the
//! power button, Raise, Next). Every stop, word and size is tafs cloud's (`GET …/knob`,
//! `tafs-core/src/knob.rs`). On the iPad it stands in the doors' own dial on the Doors tab when the
//! dial was read; here, likewise, in `DoorsView`.
//!
//! What it does is the session's at tafs cloud: a size moved (`PUT …/knob`, the next version),
//! every start paused or resumed, a door opened or closed, a run started, a follow-up removed.
//! Making the waiting version live, approving an app event and approving a held call once are
//! signed: drawn up to the signature, their tap says this computer can't approve.
//!
//! The rings, the rim, the pointer and the face are Cairo shapes; every word on them is a label
//! over the drawing, upright (the iPad turns a stop's label with its ring).

use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{cairo, glib};
use tafs_core::knob::{self, nav, Face, HeldFace, Item, Knob, Ring, Tint};

use crate::starts_when::Knobs;
use crate::widgets::{primary, text};

/// The design's measures, on its 360 grid (`TafKnobView.Grid`).
const SIZE: f64 = 360.0;
const FACE: f64 = 64.0;
const BANDS: [(f64, f64); 3] = [(68.0, 104.0), (108.0, 140.0), (144.0, 170.0)];
const RIM: (f64, f64) = (174.0, 178.0);
/// A crowded ring shows this many stops at a time.
const CAPACITY: usize = 6;

fn palette(name: &str) -> (u32, f64) {
    let t = tafs_core::theme::tone(name).expect("the palette has it");
    let c = if adw::StyleManager::default().is_dark() {
        t.dark
    } else {
        t.light
    };
    (c.0, c.1 as f64)
}

fn rgba(cr: &cairo::Context, (hex, alpha): (u32, f64), times: f64) {
    crate::illustrations::rgb(cr, hex, alpha * times);
}

fn coloured(label: &gtk::Label, hex: u32, alpha: f64) {
    let attrs = label.attributes().unwrap_or_default();
    let c = |shift: u32| (((hex >> shift) & 0xFF) * 257) as u16;
    attrs.insert(gtk::pango::AttrColor::new_foreground(c(16), c(8), c(0)));
    attrs.insert(gtk::pango::AttrInt::new_foreground_alpha(
        (alpha.clamp(0.0, 1.0) * 65535.0) as u16,
    ));
    label.set_attributes(Some(&attrs));
}

fn sized(s: &str, size: f64, weight: gtk::pango::Weight) -> gtk::Label {
    let l = gtk::Label::new(Some(s));
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrSize::new_size_absolute(
        (size * gtk::pango::SCALE as f64) as i32,
    ));
    attrs.insert(gtk::pango::AttrInt::new_weight(weight));
    l.set_attributes(Some(&attrs));
    l
}

/// What the dial draws from: the shared dial, the face, the filter, each ring's turn, and the
/// held calls the face can show.
struct Dial {
    knobs: Knobs,
    doors: crate::doors::DoorsModel,
    manages: bool,
    held: Vec<HeldFace>,
    face: RefCell<Face>,
    filter: RefCell<nav::Filter>,
    turns: RefCell<std::collections::HashMap<Ring, i64>>,
    area: gtk::DrawingArea,
    fixed: gtk::Fixed,
    column: gtk::Box,
    details: gtk::Box,
    remote: gtk::Box,
    slot: gtk::Box,
    /// A door's stop on When picks that door for the panels below.
    picks_door: Rc<dyn Fn(&str)>,
}

impl Dial {
    fn l(&self, k: &str) -> String {
        self.knobs.words().line(k).unwrap_or_default()
    }

    fn knob(&self) -> Option<Knob> {
        self.knobs.knob()
    }

    fn chosen(&self) -> Option<(Ring, String)> {
        match &*self.face.borrow() {
            Face::Chosen(r, id) => Some((*r, id.clone())),
            _ => None,
        }
    }

    fn visible(&self, knob: &Knob, ring: Ring) -> Vec<Item> {
        let items = nav::items(knob, ring, &self.filter.borrow());
        let chosen = self.chosen();
        let selected = chosen
            .as_ref()
            .filter(|(r, _)| *r == ring)
            .map(|(_, id)| id.as_str());
        nav::page(&items, selected, CAPACITY)
    }

    fn radii(knob: &Knob, ring: Ring) -> (f64, f64) {
        let i = knob
            .ordered_rings()
            .iter()
            .position(|r| *r == ring)
            .unwrap_or(0);
        BANDS[i.min(BANDS.len() - 1)]
    }

    /// The scale and centre of the 360 grid in the area as it is.
    fn grid(&self) -> (f64, f64, f64) {
        let (w, h) = (self.area.width() as f64, self.area.height() as f64);
        let k = w.min(h) / SIZE;
        (k, w / 2.0, h / 2.0)
    }

    /// A stop's angle on screen in degrees, 0 at the top, clockwise.
    fn angle(&self, ring: Ring, index: usize, count: usize) -> f64 {
        let step = 360.0 / count.max(1) as f64;
        index as f64 * step - *self.turns.borrow().get(&ring).unwrap_or(&0) as f64 * step
    }

    fn choose(self: &Rc<Self>, ring: Ring, id: &str) {
        *self.face.borrow_mut() = Face::Chosen(ring, id.to_string());
        self.align();
        if ring == Ring::Wakes {
            let door = self.knob().and_then(|k| {
                k.wakes
                    .iter()
                    .find(|w| w.id == id)
                    .and_then(|w| w.door.clone())
            });
            if let Some(door) = door {
                (self.picks_door)(&door);
            }
        }
        self.draw_all();
    }

    /// `alignSelection`: the chosen ring turns its stop to the top, the short way.
    fn align(&self) {
        let (Some(knob), Some((ring, id))) = (self.knob(), self.chosen()) else {
            return;
        };
        let items = self.visible(&knob, ring);
        if let Some(i) = items.iter().position(|it| it.id == id) {
            let mut turns = self.turns.borrow_mut();
            let now = *turns.get(&ring).unwrap_or(&0);
            turns.insert(ring, nav::nearest_turn(now, i as i64, items.len() as i64));
        }
    }

    fn walk(self: &Rc<Self>, direction: i64) {
        let Some(knob) = self.knob() else { return };
        let stops = nav::stops(&knob, &self.filter.borrow());
        let chosen = self.chosen();
        if let Some(stop) = nav::next(
            &stops,
            chosen.as_ref().map(|c| c.0),
            chosen.as_ref().map(|c| c.1.as_str()),
            direction,
        ) {
            self.choose(stop.ring, &stop.item.id);
        }
    }

    /// The face pressed: a held call approved once (signed), Coach, or the chosen stop's act.
    fn press_face(self: &Rc<Self>) {
        let knob = self.knob();
        let face = self.face.borrow().clone();
        if let Face::Held(id) = &face {
            if let Some(call) = self.held.iter().find(|h| &h.id == id) {
                if call.approvable_once && self.manages && !self.knobs.busy() {
                    crate::approval::refuse(
                        self.knobs.words(),
                        tafs_core::sidebar::Mode::Kiff,
                        &tafs_core::signed::Act::ApproveOnce { call: id.clone() },
                        &self.slot,
                    );
                }
            }
            return;
        }
        let item = knob
            .as_ref()
            .and_then(|k| knob::chosen_item(k, &face, &self.filter.borrow()));
        if item.as_ref().and_then(|i| i.ok.as_deref()) == Some("coach") {
            (self.knobs.taf().open_coach)();
            return;
        }
        match (self.chosen(), item) {
            (Some(_), Some(item)) if item.ok.is_some() && self.manages && !self.knobs.busy() => {
                self.press(&item)
            }
            (None, _) => self.walk(1),
            _ => {}
        }
    }

    /// `pressKnob`: what a stop's face does, as the session. An app event to approve opens its
    /// filters first (`AppEventApprovalSheet`), where approving, signed, stands as the gate's line.
    fn press(self: &Rc<Self>, item: &Item) {
        let taf = self.knobs.taf().id.clone();
        match item.ok.as_deref() {
            Some("approve") => {
                crate::app_event_sheet::present(
                    self.knobs.words(),
                    tafs_core::sidebar::Mode::Kiff,
                    &tafs_core::app_event::Item::from(item),
                    &self.slot,
                );
            }
            Some("door") => {
                if let Some(door) = &item.door {
                    let on = item.on;
                    let path = tafs_core::doors::door_path(&taf, door);
                    let fails = if on {
                        "door.not_closed"
                    } else {
                        "door.not_opened"
                    };
                    self.send("PATCH", path, Some(tafs_core::doors::open_body(!on)), fails);
                }
            }
            Some("run") => {
                self.send(
                    "POST",
                    format!("/v1/tafs/{}/run", tafs_core::work::segment(&taf)),
                    Some(b"{}".to_vec()),
                    "",
                );
            }
            Some("remove_follow_up") => {
                if let Some(id) = &item.follow_up {
                    self.knobs.remove_follow_up(id);
                }
            }
            _ => {}
        }
    }

    /// A press that answers with no new version: sent as the session, then the dial and the
    /// doors read again; what failed said in tafs cloud's words, or with `fails`' line.
    fn send(
        self: &Rc<Self>,
        method: &'static str,
        path: String,
        body: Option<Vec<u8>>,
        fails: &'static str,
    ) {
        let me = self.clone();
        self.knobs.set_busy(true);
        glib::spawn_future_local(async move {
            let why = match me.knobs.send(method, path, body).await {
                crate::starts_when::Sent::Answer(a) if (200..300).contains(&a.status) => None,
                crate::starts_when::Sent::Answer(a) => Some(me.knobs.refused(&a)),
                crate::starts_when::Sent::Nothing => None,
                crate::starts_when::Sent::Failed(why) => Some(why),
            };
            if let Some(why) = why {
                let words = if fails.is_empty() {
                    why
                } else {
                    me.knobs.words().get(fails, &[("why", &why)]).unwrap_or(why)
                };
                me.knobs.set_error(Some(words));
            }
            me.knobs.load().await;
            me.doors.load();
            me.knobs.set_busy(false);
        });
    }

    /// The labels over the drawing: each visible stop's, the face's, and the empty rings' dashes.
    fn draw(self: &Rc<Self>) {
        self.area.queue_draw();
        while let Some(c) = self.fixed.first_child() {
            self.fixed.remove(&c);
        }
        let Some(knob) = self.knob() else {
            return;
        };
        let w = self.knobs.words().clone();
        let (k, cx, cy) = self.grid();
        let chosen = self.chosen();
        let put = |x: &gtk::Widget, px: f64, py: f64| {
            self.fixed.put(x, 0.0, 0.0);
            let (_, nat) = x.preferred_size();
            self.fixed.move_(
                x,
                px - nat.width() as f64 / 2.0,
                py - nat.height() as f64 / 2.0,
            );
        };
        for ring in knob.ordered_rings() {
            let (inner, outer) = Self::radii(&knob, ring);
            let mid = (inner + outer) / 2.0 * k;
            let items = self.visible(&knob, ring);
            let on = chosen.as_ref().is_some_and(|c| c.0 == ring);
            let dim = chosen.is_some() && !on;
            if items.is_empty() {
                let dash = sized("—", 14.0, gtk::pango::Weight::Normal);
                coloured(&dash, ring.color(), 0.4);
                dash.update_property(&[gtk::accessible::Property::Label(
                    &(knob.title(&w, ring).unwrap_or_default() + &self.l("knob.no_stops")),
                )]);
                put(dash.upcast_ref(), cx, cy - mid);
            }
            let width = nav::label_width(mid, items.len(), 100.0 * k, 12.0 * k);
            for (i, item) in items.iter().enumerate() {
                let a = self.angle(ring, i, items.len()) * PI / 180.0;
                let picked = on && chosen.as_ref().is_some_and(|c| c.1 == item.id);
                let label = sized(
                    &item.ring_label(),
                    12.0,
                    if picked {
                        gtk::pango::Weight::Bold
                    } else {
                        gtk::pango::Weight::Medium
                    },
                );
                label.set_ellipsize(gtk::pango::EllipsizeMode::End);
                label.set_max_width_chars(1);
                label.set_width_request(width.max(12.0) as i32);
                if picked {
                    coloured(&label, ring.color(), 1.0);
                } else if on {
                    label.add_css_class("tafs-ink");
                } else {
                    coloured(&label, ring.color(), if item.on { 0.85 } else { 0.45 });
                }
                let b = gtk::Button::new();
                b.set_child(Some(&label));
                b.add_css_class("tafs-plain");
                b.set_opacity(if dim { 0.4 } else { 1.0 });
                b.update_property(&[gtk::accessible::Property::Label(
                    &[
                        Some(item.ring_label()),
                        knob.title(&w, ring).ok(),
                        item.state.clone(),
                        Some(item.words.clone()),
                    ]
                    .into_iter()
                    .flatten()
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join(", "),
                )]);
                let (me, id) = (Rc::downgrade(self), item.id.clone());
                b.connect_clicked(move |_| {
                    if let Some(d) = me.upgrade() {
                        d.choose(ring, &id);
                    }
                });
                put(b.upcast_ref(), cx + a.sin() * mid, cy - a.cos() * mid);
            }
        }
        // The face: its kicker, its big word, its hint, as one button.
        let Ok(lines) = knob::face_lines(
            &w,
            &knob,
            &self.face.borrow(),
            &self.filter.borrow(),
            &self.held,
        ) else {
            return;
        };
        let v = gtk::Box::new(gtk::Orientation::Vertical, (4.0 * k) as i32);
        v.set_width_request((FACE * 2.0 * k - 20.0 * k) as i32);
        v.set_valign(gtk::Align::Center);
        let kicker = sized(&lines.kicker, 10.0, gtk::pango::Weight::Bold);
        kicker.add_css_class("tafs-dim");
        kicker.set_ellipsize(gtk::pango::EllipsizeMode::End);
        kicker.set_max_width_chars(1);
        v.append(&kicker);
        let big = sized(
            &lines.big,
            if lines.big.chars().count() > 5 {
                24.0
            } else {
                32.0
            },
            gtk::pango::Weight::Heavy,
        );
        big.set_wrap(true);
        big.set_lines(3);
        big.set_ellipsize(gtk::pango::EllipsizeMode::End);
        big.set_justify(gtk::Justification::Center);
        big.set_max_width_chars(1);
        match lines.tint {
            Tint::Held => big.add_css_class("tafs-amber-text"),
            Tint::Ring(r) => coloured(&big, r.color(), 1.0),
            Tint::Ink => big.add_css_class("tafs-ink"),
        }
        v.append(&big);
        let hint = sized(&lines.hint, 11.0, gtk::pango::Weight::Normal);
        hint.add_css_class("tafs-dim");
        hint.set_ellipsize(gtk::pango::EllipsizeMode::End);
        hint.set_max_width_chars(1);
        v.append(&hint);
        let face = gtk::Button::new();
        face.set_child(Some(&v));
        face.add_css_class("tafs-plain");
        face.set_size_request((FACE * 2.0 * k) as i32, (FACE * 2.0 * k) as i32);
        face.update_property(&[gtk::accessible::Property::Label(
            &[&lines.kicker, &lines.big, &lines.hint]
                .into_iter()
                .filter(|s| !s.is_empty())
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        )]);
        let me = Rc::downgrade(self);
        face.connect_clicked(move |_| {
            if let Some(d) = me.upgrade() {
                d.press_face();
            }
        });
        put(face.upcast_ref(), cx, cy);
    }

    /// The rim, the bands, the doors' arcs and lights, the pointer and the face's circle.
    fn paint(&self, cr: &cairo::Context) {
        let Some(knob) = self.knob() else { return };
        let (k, cx, cy) = self.grid();
        if k <= 0.0 {
            return;
        }
        let chosen = self.chosen();
        // Today's work around the rim.
        let rim_mid = (RIM.0 + RIM.1) / 2.0 * k;
        let rim_w = (RIM.1 - RIM.0) * k;
        cr.set_line_width(rim_w);
        cr.new_sub_path();
        cr.arc(cx, cy, rim_mid, 0.0, 2.0 * PI);
        rgba(cr, palette("line"), 1.0);
        let _ = cr.stroke();
        for (from, to, tone) in knob::rim_arcs(&knob.rim) {
            cr.new_sub_path();
            cr.arc(
                cx,
                cy,
                rim_mid,
                -PI / 2.0 + from * 2.0 * PI,
                -PI / 2.0 + to * 2.0 * PI,
            );
            rgba(cr, palette(tone), 1.0);
            let _ = cr.stroke();
        }
        for ring in knob.ordered_rings() {
            let (inner, outer) = Self::radii(&knob, ring);
            let on = chosen.as_ref().is_some_and(|c| c.0 == ring);
            let dim = if chosen.is_some() && !on { 0.4 } else { 1.0 };
            cr.set_line_width((outer - inner) * k);
            cr.new_sub_path();
            cr.arc(cx, cy, (inner + outer) / 2.0 * k, 0.0, 2.0 * PI);
            crate::illustrations::rgb(cr, ring.color(), if on { 0.16 } else { 0.07 } * dim);
            let _ = cr.stroke();
            let items = self.visible(&knob, ring);
            let n = items.len().max(1) as f64;
            for (i, item) in items.iter().enumerate() {
                let (Some(_), Some(look)) = (&item.door, &item.look) else {
                    continue;
                };
                // A door's arc, just inside the band's edge: amber while a knock waits, grey
                // while resting, dashed while closed.
                let (hex, alpha) = match look.as_str() {
                    "knock" => (palette("amber").0, 1.0),
                    "resting" => palette("dim"),
                    "open" => (ring.color(), 0.85),
                    _ => (ring.color(), 0.3),
                };
                let start = self.angle(ring, i, items.len()) - 180.0 / n;
                let a0 = (start + 0.015 * 360.0 - 90.0) * PI / 180.0;
                let a1 = (start + (1.0 / n - 0.015).max(0.015) * 360.0 - 90.0) * PI / 180.0;
                cr.set_line_width(2.0 * k);
                if look == "closed" {
                    cr.set_dash(&[3.0 * k, 4.0 * k], 0.0);
                }
                cr.new_sub_path();
                cr.arc(cx, cy, (outer - 2.0) * k, a0, a1);
                crate::illustrations::rgb(cr, hex, alpha * dim);
                let _ = cr.stroke();
                cr.set_dash(&[], 0.0);
                // Its light, under its label.
                let a = self.angle(ring, i, items.len()) * PI / 180.0;
                let r = ((inner + outer) / 2.0 - 10.0) * k;
                cr.new_sub_path();
                cr.arc(cx + a.sin() * r, cy - a.cos() * r, 2.5 * k, 0.0, 2.0 * PI);
                let light = if look == "closed" { 0.2 } else { 0.9 };
                crate::illustrations::rgb(cr, hex, light * dim);
                let _ = cr.fill();
            }
        }
        // The pointer: at the chosen ring's top stop, or at the rim.
        let r = match &chosen {
            Some((ring, _)) => Self::radii(&knob, *ring).1,
            None => RIM.1,
        };
        let ty = cy - (r + 7.0) * k;
        cr.move_to(cx, ty + 5.5 * k);
        cr.line_to(cx - 7.0 * k, ty - 5.5 * k);
        cr.line_to(cx + 7.0 * k, ty - 5.5 * k);
        cr.close_path();
        match &chosen {
            Some((ring, _)) => crate::illustrations::rgb(cr, ring.color(), 1.0),
            None => rgba(cr, palette("ink"), 1.0),
        }
        let _ = cr.fill();
        // The face's circle, its edge and glow in its tint.
        let lines = knob::face_lines(
            self.knobs.words(),
            &knob,
            &self.face.borrow(),
            &self.filter.borrow(),
            &self.held,
        );
        let tint = match lines.map(|l| l.tint) {
            Ok(Tint::Held) => palette("amber_text").0,
            Ok(Tint::Ring(r)) => r.color(),
            _ => palette("ink").0,
        };
        for i in 1..=6 {
            cr.new_sub_path();
            cr.arc(cx, cy, FACE * k + i as f64 * 2.0 * k, 0.0, 2.0 * PI);
            crate::illustrations::rgb(cr, tint, 0.18 / 6.0);
            let _ = cr.fill();
        }
        cr.new_sub_path();
        cr.arc(cx, cy, FACE * k, 0.0, 2.0 * PI);
        rgba(cr, palette("panel"), 1.0);
        let _ = cr.fill_preserve();
        cr.set_line_width(1.0);
        crate::illustrations::rgb(cr, tint, 0.4);
        let _ = cr.stroke();
    }

    /// Everything around the dial: the pending line, Coach's capsule, the service menu, the note,
    /// the size, Needs you, Back to Why, the waiting version, the requests' line, the details;
    /// and the remote.
    fn draw_all(self: &Rc<Self>) {
        while let Some(c) = self.column.first_child() {
            self.column.remove(&c);
        }
        let Some(knob) = self.knob() else {
            self.column.append(&text(
                &self.l("setup.reading_starts"),
                15,
                400,
                &["tafs-dim"],
            ));
            self.column.append(&self.details);
            return;
        };
        let w = self.knobs.words().clone();
        // A face whose stop the dial no longer has goes back to the job.
        if let Some((ring, id)) = self.chosen() {
            if !nav::items(&knob, ring, &self.filter.borrow())
                .iter()
                .any(|i| i.id == id)
            {
                *self.face.borrow_mut() = Face::Overview;
            }
        }
        // `showHeld`: a call no longer to approve once leaves the face.
        let gone = matches!(&*self.face.borrow(), Face::Held(id)
            if !self.held.iter().any(|h| &h.id == id && h.approvable_once));
        if gone {
            *self.face.borrow_mut() = Face::Overview;
        }
        if let Some(p) = &knob.pending {
            let l = text(
                &w.get("knob.not_live", &[("version", &p.version.to_string())])
                    .unwrap_or_default(),
                10,
                600,
                &["tafs-mono", "tafs-amber-text"],
            );
            self.column.append(&l);
        }
        // Ask Coach or find anything.
        let search = gtk::Button::new();
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.append(&crate::widgets::symbol("system-search-symbolic", 14));
        let ask = text(&self.l("knob.search"), 14, 400, &["tafs-dim"]);
        ask.set_wrap(false);
        ask.set_ellipsize(gtk::pango::EllipsizeMode::End);
        row.append(&ask);
        search.set_child(Some(&row));
        search.add_css_class("tafs-search-capsule");
        {
            let open = self.knobs.taf().open_coach.clone();
            search.connect_clicked(move |_| open());
        }
        self.column.append(&search);
        self.column.append(&self.service_menu(&knob));
        self.column.append(&self.area_box());
        let lines = knob::face_lines(
            &w,
            &knob,
            &self.face.borrow(),
            &self.filter.borrow(),
            &self.held,
        )
        .ok();
        if let Some(lines) = &lines {
            let note = text(&lines.note, 15, 400, &["tafs-soft"]);
            note.set_xalign(0.5);
            note.set_justify(gtk::Justification::Center);
            self.column.append(&note);
            if let Some(size) = &lines.size {
                let s = text(size, 12, 500, &["tafs-mono", "tafs-dim"]);
                s.set_xalign(0.5);
                self.column.append(&s);
            }
        }
        // The remote, in sight under the dial: the iPad keeps it above the tab bar, and a sheet
        // that scrolls has no foot to keep it on.
        if let Some(parent) = self.remote.parent() {
            if let Ok(b) = parent.downcast::<gtk::Box>() {
                b.remove(&self.remote);
            }
        }
        self.column.append(&self.remote);
        if let Some(call) = self.held.iter().find(|h| h.approvable_once) {
            let b = gtk::Button::with_label(&self.l("needs_you.title"));
            b.add_css_class("tafs-link");
            b.set_halign(gtk::Align::Center);
            let (me, id) = (Rc::downgrade(self), call.id.clone());
            b.connect_clicked(move |_| {
                if let Some(d) = me.upgrade() {
                    *d.face.borrow_mut() = Face::Held(id.clone());
                    d.draw_all();
                }
            });
            self.column.append(&b);
        }
        if *self.face.borrow() != Face::Overview {
            let b = gtk::Button::with_label(&self.l("knob.back_to_why"));
            b.add_css_class("tafs-link");
            b.add_css_class("tafs-dim");
            b.set_halign(gtk::Align::Center);
            let me = Rc::downgrade(self);
            b.connect_clicked(move |_| {
                if let Some(d) = me.upgrade() {
                    *d.face.borrow_mut() = Face::Overview;
                    d.draw_all();
                }
            });
            self.column.append(&b);
        }
        // The waiting version, and Make it live, which the device checks first.
        if let (Some(p), true) = (&knob.pending, self.manages) {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            for line in knob
                .draft
                .as_ref()
                .map(|d| d.words.clone())
                .unwrap_or_default()
            {
                v.append(&text(&line, 13, 400, &["tafs-soft"]));
            }
            if let Ok(Some((_, _, button))) = knob::pending_lines(&w, &knob) {
                let b = primary(&button);
                b.set_sensitive(!self.knobs.busy());
                let (w, taf, version, slot) = (
                    w.clone(),
                    self.knobs.taf().id.clone(),
                    p.version,
                    self.slot.clone(),
                );
                b.connect_clicked(move |_| {
                    crate::approval::refuse(
                        &w,
                        tafs_core::sidebar::Mode::Kiff,
                        &tafs_core::signed::Act::MakeLive {
                            taf: taf.clone(),
                            version,
                        },
                        &slot,
                    )
                });
                v.append(&b);
            }
            self.column.append(&v);
        }
        if let Some(parent) = self.slot.parent() {
            if let Ok(b) = parent.downcast::<gtk::Box>() {
                b.remove(&self.slot);
            }
        }
        self.column.append(&self.slot);
        // Today's work in tafs cloud's line, opening its requests.
        let requests = gtk::Button::new();
        let r = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let today = text(
            &knob.rim.line.clone().unwrap_or_else(|| {
                w.get("knob.done_today", &[("done", &knob.rim.done.to_string())])
                    .unwrap_or_default()
            }),
            12,
            400,
            &["tafs-dim"],
        );
        today.set_wrap(false);
        r.append(&today);
        r.append(&crate::widgets::symbol("go-next-symbolic", 10));
        requests.set_child(Some(&r));
        requests.add_css_class("tafs-plain");
        requests.set_halign(gtk::Align::Center);
        requests.set_size_request(-1, 44);
        {
            let me = Rc::downgrade(self);
            requests.connect_clicked(move |b| {
                if let Some(d) = me.upgrade() {
                    d.requests_sheet(b.upcast_ref());
                }
            });
        }
        self.column.append(&requests);
        if let Some(parent) = self.details.parent() {
            if let Ok(b) = parent.downcast::<gtk::Box>() {
                b.remove(&self.details);
            }
        }
        self.column.append(&self.details);
        self.draw_remote(&knob);
        self.draw();
    }

    fn area_box(&self) -> gtk::Overlay {
        if let Some(parent) = self.area.parent() {
            if let Some(o) = parent.downcast_ref::<gtk::Overlay>() {
                if let Some(p) = o.parent() {
                    if let Ok(b) = p.downcast::<gtk::Box>() {
                        b.remove(o);
                    }
                }
                return o.clone();
            }
        }
        let o = gtk::Overlay::new();
        o.set_child(Some(&self.area));
        o.add_overlay(&self.fixed);
        o
    }

    /// The service menu: "Showing All services", then each choice.
    fn service_menu(self: &Rc<Self>, knob: &Knob) -> gtk::MenuButton {
        let w = self.knobs.words();
        let name = knob::filter_name(w, knob, &self.filter.borrow()).unwrap_or_default();
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let title = text(
            &w.get("knob.showing_filter", &[("filter", &name)])
                .unwrap_or_default(),
            15,
            600,
            &["tafs-ink"],
        );
        title.set_hexpand(true);
        row.append(&title);
        row.append(&crate::widgets::symbol("pan-down-symbolic", 12));
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let popover = gtk::Popover::new();
        popover.set_child(Some(&list));
        for (filter, words) in knob::filter_choices(w, knob).unwrap_or_default() {
            let b = gtk::Button::with_label(&words);
            b.add_css_class("flat");
            let (me, pop) = (Rc::downgrade(self), popover.downgrade());
            b.connect_clicked(move |_| {
                if let Some(p) = pop.upgrade() {
                    p.popdown();
                }
                if let Some(d) = me.upgrade() {
                    // A new filter starts from the job, every ring unturned.
                    *d.filter.borrow_mut() = filter.clone();
                    d.turns.borrow_mut().clear();
                    *d.face.borrow_mut() = Face::Overview;
                    d.draw_all();
                }
            });
            list.append(&b);
        }
        let menu = gtk::MenuButton::new();
        menu.set_child(Some(&row));
        menu.set_popover(Some(&popover));
        menu.add_css_class("tafs-service-menu");
        menu.update_property(&[gtk::accessible::Property::Label(
            &w.get("knob.showing_filter_label", &[("filter", &name)])
                .unwrap_or_default(),
        )]);
        menu
    }

    /// `TafKnobRemote`: Back, Lower, the power button, Raise, Next, each named under it.
    fn draw_remote(self: &Rc<Self>, knob: &Knob) {
        while let Some(c) = self.remote.first_child() {
            self.remote.remove(&c);
        }
        if let Some(e) = self.knobs.error() {
            let l = text(&e, 13, 400, &["tafs-no"]);
            self.remote.append(&l);
        }
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_homogeneous(true);
        let busy = self.knobs.busy();
        let stops = nav::stops(knob, &self.filter.borrow());
        let chosen = knob::chosen_item(knob, &self.face.borrow(), &self.filter.borrow());
        let can = |d: i64| {
            self.manages && !busy && chosen.as_ref().is_some_and(|i| self.knobs.can_move(i, d))
        };
        let paused = self.knobs.paused();
        let buttons: [(String, &str, bool, u8); 5] = [
            (
                self.l("common.back"),
                "media-skip-backward-symbolic",
                !stops.is_empty(),
                0,
            ),
            (self.l("knob.lower"), "list-remove-symbolic", can(-1), 1),
            (
                self.l(if paused { "knob.resume" } else { "knob.pause" }),
                "system-shutdown-symbolic",
                self.manages && !busy && knob.can_pause,
                2,
            ),
            (self.l("knob.raise"), "list-add-symbolic", can(1), 3),
            (
                self.l("common.next"),
                "media-skip-forward-symbolic",
                !stops.is_empty(),
                4,
            ),
        ];
        for (title, icon, enabled, which) in buttons {
            let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
            let b = gtk::Button::from_icon_name(icon);
            b.add_css_class(if which == 2 {
                "tafs-remote-power"
            } else {
                "tafs-remote"
            });
            if which == 2 && paused {
                b.add_css_class("paused");
            }
            b.set_size_request(-1, 52);
            b.set_sensitive(enabled);
            b.update_property(&[gtk::accessible::Property::Label(&title)]);
            let (me, item) = (Rc::downgrade(self), chosen.clone());
            b.connect_clicked(move |_| {
                let Some(d) = me.upgrade() else { return };
                match which {
                    0 => d.walk(-1),
                    4 => d.walk(1),
                    1 | 3 => {
                        if let Some(item) = &item {
                            d.knobs.adjust(item, if which == 1 { -1 } else { 1 });
                        }
                    }
                    _ => d.knobs.set_paused(!d.knobs.paused()),
                }
            });
            v.append(&b);
            let name = text(
                &tafs_core::words::upper(&title),
                9,
                600,
                &["tafs-mono", "tafs-dim"],
            );
            name.set_xalign(0.5);
            name.set_wrap(false);
            name.set_ellipsize(gtk::pango::EllipsizeMode::End);
            v.append(&name);
            row.append(&v);
        }
        self.remote.append(&row);
    }

    /// `KnobRequestsSheet`: its work's requests, each opening its own page; the one asking for
    /// attention, else the newest, opened at once.
    fn requests_sheet(self: &Rc<Self>, parent: &gtk::Widget) {
        let w = self.knobs.words().clone();
        let s = crate::sheet::new(
            &w.line("taf.its_work").unwrap_or_default(),
            &w.line("common.done").unwrap_or_default(),
            None,
            520,
        );
        let open = crate::runs_panel::Opening {
            w: w.clone(),
            id: self.knobs.taf().id.clone(),
            source: self.knobs.taf().runs.clone(),
            manages: self.manages,
            coach: self
                .knobs
                .taf()
                .may_use_coach
                .then(|| self.knobs.taf().open_coach.clone()),
        };
        let list = s.column.clone();
        let knobs = self.knobs.clone();
        let opened = Rc::new(Cell::new(false));
        let draw: Rc<dyn Fn()> = {
            let (list, knobs, open, opened, dialog) = (
                list.clone(),
                knobs.clone(),
                open.clone(),
                opened.clone(),
                s.dialog.downgrade(),
            );
            Rc::new(move || {
                while let Some(c) = list.first_child() {
                    list.remove(&c);
                }
                let Some(work) = knobs.work() else { return };
                for r in &work.requests {
                    let b = gtk::Button::new();
                    b.set_child(Some(&text(&r.line, 14, 400, &["tafs-ink"])));
                    b.add_css_class("tafs-run-row");
                    let (open, run) = (open.clone(), r.id.clone());
                    b.connect_clicked(move |b| {
                        crate::runs_panel::request_sheet(open.clone(), run.clone(), b)
                    });
                    list.append(&b);
                }
                if work.requests.is_empty() {
                    list.append(&text(
                        &knobs.words().line("knob.no_requests").unwrap_or_default(),
                        15,
                        400,
                        &["tafs-dim"],
                    ));
                }
                // `focusedRequest`: the work still asking for attention, else the newest.
                if !opened.replace(true) {
                    let focused = work
                        .requests
                        .iter()
                        .find(|r| r.state != "done")
                        .or(work.requests.first());
                    if let (Some(r), Some(d)) = (focused, dialog.upgrade()) {
                        crate::runs_panel::request_sheet(open.clone(), r.id.clone(), &d);
                    }
                }
            })
        };
        let listening = self.knobs.listen(draw.clone());
        {
            let knobs = self.knobs.clone();
            s.dialog.connect_closed(move |_| knobs.unlisten(listening));
        }
        s.dialog.present(Some(parent));
        self.knobs.load_work();
        draw();
    }
}

/// `TafDialPanel` with `TafKnobView` and the remote: the dial over `details` (the doors' panels),
/// the remote at the foot. Read when it's shown.
pub fn panel(
    knobs: Knobs,
    doors: crate::doors::DoorsModel,
    manages: bool,
    held: Vec<HeldFace>,
    details: gtk::Box,
    picks_door: Rc<dyn Fn(&str)>,
) -> gtk::Box {
    let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    root.append(&column);
    let remote = gtk::Box::new(gtk::Orientation::Vertical, 8);
    remote.set_margin_top(4);
    let area = gtk::DrawingArea::new();
    area.set_content_height(SIZE as i32);
    area.set_hexpand(true);
    area.update_state(&[gtk::accessible::State::Hidden(true)]);
    let fixed = gtk::Fixed::new();
    // `showHeld`: a call that may be approved once is on the face first.
    let face = held
        .iter()
        .find(|h| h.approvable_once)
        .map(|h| Face::Held(h.id.clone()))
        .unwrap_or_default();
    let d = Rc::new(Dial {
        knobs: knobs.clone(),
        doors,
        manages,
        held,
        face: RefCell::new(face),
        filter: RefCell::new(nav::Filter::All),
        turns: RefCell::new(std::collections::HashMap::new()),
        area: area.clone(),
        fixed,
        column,
        details,
        remote,
        slot: crate::approval::refusal_slot(),
        picks_door,
    });
    {
        let weak = Rc::downgrade(&d);
        area.set_draw_func(move |_, cr, _, _| {
            if let Some(d) = weak.upgrade() {
                d.paint(cr);
            }
        });
    }
    {
        let weak = Rc::downgrade(&d);
        area.connect_resize(move |_, _, _| {
            if let Some(d) = weak.upgrade() {
                // Placed again for the new size, once this allocation is done.
                glib::idle_add_local_once(move || d.draw());
            }
        });
    }
    {
        let a = area.downgrade();
        let id = adw::StyleManager::default().connect_dark_notify(move |_| {
            if let Some(a) = a.upgrade() {
                a.queue_draw();
            }
        });
        let id = RefCell::new(Some(id));
        area.connect_destroy(move |_| {
            if let Some(id) = id.borrow_mut().take() {
                adw::StyleManager::default().disconnect(id);
            }
        });
    }
    let listening = {
        let weak = Rc::downgrade(&d);
        knobs.listen(Rc::new(move || {
            if let Some(d) = weak.upgrade() {
                d.draw_all();
            }
        }))
    };
    // SAFETY: only ever read back as the type it's stored as, and only here.
    unsafe { root.set_data("dial", d.clone()) };
    {
        let knobs = knobs.clone();
        root.connect_destroy(move |_| knobs.unlisten(listening));
    }
    d.draw_all();
    knobs.reload();
    root
}

/// The taf's held calls as its dial's face shows them.
pub fn held_faces(held: &[tafs_core::needs_you::CardLines]) -> Vec<HeldFace> {
    held.iter()
        .map(|c| HeldFace {
            id: c.id.clone(),
            summary: c.summary.clone(),
            amount: c.amount.clone(),
            reason: c.reason.clone(),
            approvable_once: c.approvable_once,
        })
        .collect()
}
