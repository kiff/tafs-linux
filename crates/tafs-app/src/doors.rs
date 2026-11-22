//! A taf's doors (`DoorsView.swift`, `DoorsModel`): where work reaches it from outside the app.
//! `DoorsView` is the doors' own dial (`DoorDial`) over the door it's turned to, or one door's
//! screen (`only`, Setup › Starts when › Edit): its address, who may give it work (its keys), its
//! day, and the knocks waiting. Adding a door (`AddDoorSheet`), opening and closing one, pairing a
//! sender (`PairSheet`), unpairing and removing a door are the owner's session's, so they really
//! go to tafs cloud (`kiff::cloud_send`); a Slack door's subscription at planque is signed, so its
//! removal stops at the signature. The lines are `tafs-core/src/doors.rs`.

use std::cell::{Cell, RefCell};
use std::f64::consts::PI;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{cairo, gio, glib};
use tafs_core::doors::{self, dial, BadgeColor, Door, DoorApp, Doors, Key, Knock};
use tafs_core::words::WordBook;

use crate::widgets::{eyebrow, outline, panel, primary, text};

/// Where a taf's doors are read and changed.
#[derive(Clone)]
pub enum DoorsSource {
    /// tafs cloud, as the session.
    Cloud {
        token: String,
        languages: Vec<&'static str>,
    },
    /// A screenshot's answer (`--doors`): read, and nothing a change would send is sent.
    File(std::path::PathBuf),
    None,
}

/// `DoorsModel`'s state: the doors once read (`loaded`), why a change or a read failed, and
/// whether a change is on its way.
#[derive(Default)]
pub struct DoorsState {
    pub doors: Option<Doors>,
    pub error: Option<String>,
    pub busy: bool,
}

type Listeners = Rc<RefCell<Vec<(u64, Rc<dyn Fn()>)>>>;

/// One taf's doors (`DoorsModel`), shared by the views that draw them: each listens, and draws
/// again when they change.
#[derive(Clone)]
pub struct DoorsModel {
    pub w: Rc<WordBook>,
    /// tafs cloud's id for the taf.
    pub id: String,
    pub source: DoorsSource,
    pub state: Rc<RefCell<DoorsState>>,
    listeners: Listeners,
    next: Rc<Cell<u64>>,
}

/// What a change answered: the door it's now, nothing to show (a screenshot's), or why not.
enum Changed {
    Door(Box<Door>),
    Nothing,
    Failed(String),
}

impl DoorsModel {
    pub fn new(w: Rc<WordBook>, id: String, source: DoorsSource) -> DoorsModel {
        DoorsModel {
            w,
            id,
            source,
            state: Rc::new(RefCell::new(DoorsState::default())),
            listeners: Rc::new(RefCell::new(Vec::new())),
            next: Rc::new(Cell::new(0)),
        }
    }

    /// Calls `f` each time the doors change; `unlisten` with what it returns stops it.
    pub fn listen(&self, f: Rc<dyn Fn()>) -> u64 {
        let id = self.next.get() + 1;
        self.next.set(id);
        self.listeners.borrow_mut().push((id, f));
        id
    }

    pub fn unlisten(&self, id: u64) {
        self.listeners.borrow_mut().retain(|(i, _)| *i != id);
    }

    pub fn changed(&self) {
        let all: Vec<Rc<dyn Fn()>> = self
            .listeners
            .borrow()
            .iter()
            .map(|(_, f)| f.clone())
            .collect();
        for f in all {
            f();
        }
    }

    pub fn loaded(&self) -> bool {
        self.state.borrow().doors.is_some()
    }

    pub fn doors(&self) -> Vec<Door> {
        self.state
            .borrow()
            .doors
            .as_ref()
            .map(|d| d.doors.clone())
            .unwrap_or_default()
    }

    pub fn open_knocks(&self) -> Vec<Knock> {
        self.state
            .borrow()
            .doors
            .as_ref()
            .map(Doors::open_knocks)
            .unwrap_or_default()
    }

    pub fn addable(&self) -> Vec<DoorApp> {
        self.state
            .borrow()
            .doors
            .as_ref()
            .map(Doors::addable)
            .unwrap_or_default()
    }

    /// `DoorsModel.load`: read, trying again quietly after 1 s and 3 s when a read fails; only a
    /// read that keeps failing says so.
    pub fn load(&self) {
        let me = self.clone();
        glib::spawn_future_local(async move {
            let (source, id) = (me.source.clone(), me.id.clone());
            let read = gio::spawn_blocking(move || -> Option<Doors> {
                for (attempt, wait) in [0u64, 1, 3].into_iter().enumerate() {
                    if attempt > 0 {
                        std::thread::sleep(std::time::Duration::from_secs(wait));
                    }
                    let body = match &source {
                        DoorsSource::Cloud { token, languages } => {
                            match crate::kiff::cloud(token, languages, &doors::path(&id)) {
                                Ok(a) if a.status == 200 => a.body,
                                _ => continue,
                            }
                        }
                        DoorsSource::File(file) => std::fs::read(file).ok()?,
                        DoorsSource::None => return None,
                    };
                    if let Some(d) = doors::read(&body) {
                        return Some(d);
                    }
                }
                None
            })
            .await
            .ok()
            .flatten();
            {
                let mut s = me.state.borrow_mut();
                match read {
                    Some(d) => {
                        s.doors = Some(d);
                        s.error = None;
                    }
                    None => s.error = me.w.line("door.not_read").ok(),
                }
            }
            me.changed();
        });
    }

    /// Why tafs cloud didn't take a change: its words, a 429 without them in the app's own, as
    /// the chat says it.
    fn refused(words: &WordBook, a: &tafs_core::signin::Answer, too_many: bool) -> String {
        let cloud_words = serde_json::from_slice::<serde_json::Value>(&a.body)
            .ok()
            .and_then(|v| v.get("words").and_then(|w| w.as_str()).map(String::from));
        if too_many && a.status == 429 && cloud_words.is_none() {
            return words.line("door.knock.too_many").unwrap_or_default();
        }
        if (200..300).contains(&a.status) {
            // Taken, but not as the app reads it.
            return tafs_core::signin::plain_words(words, 200).unwrap_or_default();
        }
        tafs_core::signin::refusal(a, Some(words))
            .words(words)
            .unwrap_or_default()
    }

    /// `DoorsModel.answer`: "do" (with the owner's words), "pair" or "ignore". Pairing keeps the
    /// message waiting, so the doors are read again; anything else takes the knock away.
    pub fn answer(
        &self,
        knock: &Knock,
        action: &'static str,
        say: Option<String>,
        done: Rc<dyn Fn()>,
    ) {
        if self.state.borrow().busy {
            return;
        }
        self.state.borrow_mut().busy = true;
        self.changed();
        let me = self.clone();
        let knock_id = knock.id.clone();
        glib::spawn_future_local(async move {
            let (words, source, id, k) = (
                (*me.w).clone(),
                me.source.clone(),
                me.id.clone(),
                knock_id.clone(),
            );
            let sent = gio::spawn_blocking(move || -> Result<bool, String> {
                match &source {
                    DoorsSource::Cloud { token, languages } => {
                        let a = crate::kiff::answer_knock(
                            token,
                            languages,
                            &doors::knock_path(&id, &k),
                            &doors::answer_body(action, say.as_deref()),
                        )
                        .map_err(|e| e.words(&words).unwrap_or_default())?;
                        if a.status == 200 && doors::answered(&a.body) {
                            return Ok(true);
                        }
                        Err(DoorsModel::refused(&words, &a, true))
                    }
                    // A screenshot's knocks: nothing is sent, and nothing changes.
                    _ => Ok(false),
                }
            })
            .await
            .unwrap_or_else(|_| Err(String::new()));
            let reload = {
                let mut s = me.state.borrow_mut();
                s.busy = false;
                match sent {
                    Ok(true) => {
                        s.error = None;
                        if action == "pair" {
                            true
                        } else {
                            if let Some(d) = s.doors.as_mut() {
                                d.knocks.retain(|k| k.id != knock_id);
                            }
                            false
                        }
                    }
                    Ok(false) => false,
                    Err(why) => {
                        s.error = Some(why);
                        false
                    }
                }
            };
            me.changed();
            done();
            if reload {
                me.load();
            }
        });
    }

    /// `DoorsModel.work`: a change on tafs cloud that answers with the door, which replaces the
    /// one shown. A change that fails says why; one that works leaves an earlier error as it was.
    fn work(
        &self,
        method: &'static str,
        path: String,
        body: Option<Vec<u8>>,
        done: Rc<dyn Fn(Option<Door>)>,
    ) {
        self.state.borrow_mut().busy = true;
        self.changed();
        let me = self.clone();
        glib::spawn_future_local(async move {
            let (words, source) = ((*me.w).clone(), me.source.clone());
            let sent = gio::spawn_blocking(move || match &source {
                DoorsSource::Cloud { token, languages } => {
                    match crate::kiff::cloud_send(token, languages, method, &path, body.as_deref())
                    {
                        Err(e) => Changed::Failed(e.words(&words).unwrap_or_default()),
                        Ok(a) => {
                            if method == "DELETE" && path.split('/').count() == 6 {
                                // `removeDoor`: `{"removed": …}`, no door.
                                if a.status == 200 && doors::removed(&a.body) {
                                    Changed::Nothing
                                } else {
                                    Changed::Failed(DoorsModel::refused(&words, &a, false))
                                }
                            } else {
                                match (a.status, doors::read_door(&a.body)) {
                                    (200 | 201, Some(d)) => Changed::Door(Box::new(d)),
                                    _ => Changed::Failed(DoorsModel::refused(&words, &a, false)),
                                }
                            }
                        }
                    }
                }
                // A screenshot's doors: nothing is sent, and nothing changes.
                _ => Changed::Nothing,
            })
            .await
            .unwrap_or(Changed::Failed(String::new()));
            let door = {
                let mut s = me.state.borrow_mut();
                s.busy = false;
                match sent {
                    Changed::Door(d) => {
                        if let Some(all) = s.doors.as_mut() {
                            all.put((*d).clone());
                        }
                        Some(*d)
                    }
                    Changed::Nothing => None,
                    Changed::Failed(why) => {
                        s.error = Some(why);
                        None
                    }
                }
            };
            me.changed();
            done(door);
        });
    }

    /// `DoorsModel.add`: a door turned on at once. WhatsApp's starts with its signup, whose page
    /// comes back to the iPhone's own address, which this computer has none of: it says where
    /// it's added instead, and sends nothing.
    pub fn add(&self, app: &DoorApp, done: Rc<dyn Fn(bool)>) {
        if app.kind == "whatsapp" {
            self.state.borrow_mut().error = self.w.line("door.whatsapp_elsewhere").ok();
            self.changed();
            done(false);
            return;
        }
        self.work(
            "POST",
            doors::path(&self.id),
            Some(doors::add_body(&app.id, &[])),
            Rc::new(move |d| done(d.is_some())),
        );
    }

    pub fn set_open(&self, door: &Door, open: bool) {
        // An `api` door opens again only signed (`reopen`): pausing it is always safe.
        if doors::api::only_signed(door, "PATCH", open) {
            return;
        }
        self.work(
            "PATCH",
            doors::door_path(&self.id, &door.id),
            Some(doors::open_body(open)),
            Rc::new(|_| {}),
        );
    }

    /// `DoorsModel.remove`: the door goes once tafs cloud has taken it away.
    pub fn remove(&self, door: &Door, done: Rc<dyn Fn()>) {
        // An `api` door is never removed unsigned (`removeAPI`).
        if doors::api::only_signed(door, "DELETE", false) {
            return;
        }
        let me = self.clone();
        let gone = door.id.clone();
        let state = self.state.clone();
        let was_error = self.state.borrow().error.clone();
        self.work(
            "DELETE",
            doors::door_path(&self.id, &door.id),
            None,
            Rc::new(move |_| {
                let took = {
                    let s = state.borrow();
                    s.error == was_error && matches!(me.source, DoorsSource::Cloud { .. })
                };
                if took {
                    if let Some(d) = state.borrow_mut().doors.as_mut() {
                        d.doors.retain(|x| x.id != gone);
                    }
                    me.changed();
                }
                done();
            }),
        );
    }

    /// `DoorsModel.pair`: the sender trimmed; nothing when that leaves nothing.
    pub fn pair(&self, door: &Door, sender: &str, name: &str, done: Rc<dyn Fn()>) {
        if doors::api::only_signed(door, "POST", false) {
            return;
        }
        let Some(body) = doors::pair_body(sender, name) else {
            return;
        };
        self.work(
            "POST",
            doors::keys_path(&self.id, &door.id),
            Some(body),
            Rc::new(move |_| done()),
        );
    }

    pub fn unpair(&self, door: &Door, key: &Key) {
        if doors::api::only_signed(door, "DELETE", false) {
            return;
        }
        self.work(
            "DELETE",
            doors::key_path(&self.id, &door.id, &key.id),
            None,
            Rc::new(|_| {}),
        );
    }
}

/// The taf the doors are for, as its page draws it.
#[derive(Clone)]
pub struct DoorsTaf {
    pub name: String,
    pub initial: String,
    pub color: i64,
}

/// How `DoorsView` is opened.
#[derive(Clone, Default)]
pub struct Opens {
    /// Whether this person manages the taf: only then are its doors' controls shown.
    pub manages: bool,
    /// This person's sign-in: their own key, paired for them, reads "You".
    pub my_email: Option<String>,
    /// One door's screen (`only`): its settings, no dial.
    pub focus: Option<String>,
    /// Removing a Slack door takes planque's subscription back first (`slack(add:remove:)`).
    pub removes_slack: bool,
    /// Adding a Slack door opens its own sheet.
    pub add_slack: Option<Rc<dyn Fn(DoorApp)>>,
    /// The taf's dial, as tafs cloud last sent it, and the held calls its face can show: the
    /// dial stands in the doors' own (`knob(_:held:press:approve:)`).
    pub knob: Option<(crate::starts_when::Knobs, Vec<tafs_core::knob::HeldFace>)>,
}

// Colours, in the appearance on screen.

fn palette(name: &str) -> (u32, f64) {
    let t = tafs_core::theme::tone(name).expect("the palette has it");
    let c = if adw::StyleManager::default().is_dark() {
        t.dark
    } else {
        t.light
    };
    (c.0, c.1 as f64)
}

fn set(cr: &cairo::Context, (hex, alpha): (u32, f64), times: f64) {
    crate::illustrations::rgb(cr, hex, alpha * times);
}

fn badge_hex(app: &str) -> u32 {
    match doors::color(app) {
        BadgeColor::Sky => palette("sky").0,
        BadgeColor::Ok => palette("ok").0,
        BadgeColor::Taf(i) => tafs_core::theme::taf_color(i),
    }
}

fn hex_attr(label: &gtk::Label, hex: u32) {
    let attrs = label.attributes().unwrap_or_default();
    let c = |shift: u32| (((hex >> shift) & 0xFF) * 257) as u16;
    attrs.insert(gtk::pango::AttrColor::new_foreground(c(16), c(8), c(0)));
    label.set_attributes(Some(&attrs));
}

fn sized(s: &str, size: f64, weight: gtk::pango::Weight, mono: bool) -> gtk::Label {
    let l = gtk::Label::new(Some(s));
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrSize::new_size_absolute(
        (size * gtk::pango::SCALE as f64) as i32,
    ));
    attrs.insert(gtk::pango::AttrInt::new_weight(weight));
    if mono {
        attrs.insert(gtk::pango::AttrString::new_family("monospace"));
    }
    l.set_attributes(Some(&attrs));
    l
}

/// The circle a door's badge is drawn as, at (`x`, `y`) its centre: filled while open.
fn badge_shape(cr: &cairo::Context, app: &str, open: bool, x: f64, y: f64, size: f64) {
    cr.new_sub_path();
    cr.arc(x, y, size / 2.0, 0.0, 2.0 * PI);
    if open {
        crate::illustrations::rgb(cr, badge_hex(app), 1.0);
        let _ = cr.fill();
    } else {
        set(cr, palette("panel"), 1.0);
        let _ = cr.fill_preserve();
        set(cr, palette("ink"), 0.25);
        cr.set_line_width(1.5);
        let _ = cr.stroke();
    }
}

/// `DoorBadge`: a door's app as a small round badge, filled while open, its letter on it.
pub fn badge(app: &str, open: bool, size: i32) -> gtk::Widget {
    let o = gtk::Overlay::new();
    let (app_s, s) = (app.to_string(), size as f64);
    let area = gtk::DrawingArea::new();
    area.set_content_width(size);
    area.set_content_height(size);
    area.set_draw_func(move |_, cr, _, _| badge_shape(cr, &app_s, open, s / 2.0, s / 2.0, s));
    o.set_child(Some(&area));
    let mark = sized(
        &doors::mark(app),
        s * 0.46,
        gtk::pango::Weight::Heavy,
        false,
    );
    if open {
        hex_attr(&mark, 0xFFFFFF);
    } else {
        mark.add_css_class("tafs-dim");
    }
    o.add_overlay(&mark);
    o.set_halign(gtk::Align::Start);
    o.set_valign(gtk::Align::Center);
    o.update_state(&[gtk::accessible::State::Hidden(true)]);
    o.upcast()
}

/// A meter's capsule (`Meter`), its fill in `hex`.
fn meter(fraction: f64, hex: u32) -> gtk::DrawingArea {
    let bar = gtk::DrawingArea::new();
    bar.set_content_height(5);
    bar.set_hexpand(true);
    let fraction = fraction.clamp(0.0, 1.0);
    bar.set_draw_func(move |_, cr, width, height| {
        let (wd, h) = (width as f64, height as f64);
        crate::illustrations::rounded(cr, 0.0, 0.0, wd, h, h / 2.0);
        set(cr, palette("ink"), 0.08);
        let _ = cr.fill();
        if fraction > 0.0 {
            crate::illustrations::rounded(cr, 0.0, 0.0, (wd * fraction).max(h), h, h / 2.0);
            crate::illustrations::rgb(cr, hex, 1.0);
            let _ = cr.fill();
        }
    });
    bar
}

/// The doors' own dial (`DoorDial`): the taf in the middle, its doors on a ring that turns, a
/// pointer at the top. Each stop is a door; the last is always "Add a door". Turned by dragging,
/// by tapping a stop, or with the arrow keys; pressing the taf opens or closes the door at the
/// top (`press`). The ring is shapes; its words are labels over it.
struct Dial {
    root: gtk::Overlay,
    area: gtk::DrawingArea,
    fixed: gtk::Fixed,
    w: Rc<WordBook>,
    taf: DoorsTaf,
    doors: RefCell<Vec<Door>>,
    knocks: RefCell<Vec<Knock>>,
    selected: Rc<Cell<usize>>,
    rotation: Cell<f64>,
    breathe: Cell<f64>,
    drag: Cell<Option<(f64, f64)>>,
    labels: RefCell<Vec<(gtk::Label, gtk::Label)>>,
    avatar: gtk::Label,
    center: gtk::Label,
    turning: RefCell<Option<adw::SpringAnimation>>,
    breathing: RefCell<Option<adw::TimedAnimation>>,
    /// The stop at the top changed.
    picked: RefCell<Option<Rc<dyn Fn()>>>,
    press: RefCell<Option<Rc<dyn Fn()>>>,
}

impl Dial {
    fn new(w: Rc<WordBook>, taf: DoorsTaf, selected: Rc<Cell<usize>>) -> Rc<Dial> {
        let root = gtk::Overlay::builder()
            .accessible_role(gtk::AccessibleRole::Slider)
            .focusable(true)
            .build();
        root.set_size_request(-1, 300);
        root.set_hexpand(true);
        let area = gtk::DrawingArea::new();
        area.set_hexpand(true);
        area.set_vexpand(true);
        root.set_child(Some(&area));
        let fixed = gtk::Fixed::new();
        fixed.set_can_target(false);
        root.add_overlay(&fixed);
        let avatar = crate::widgets::avatar(&taf.initial, taf.color, 56);
        fixed.put(&avatar, 0.0, 0.0);
        let center = sized("", 10.0, gtk::pango::Weight::Semibold, true);
        fixed.put(&center, 0.0, 0.0);
        let d = Rc::new(Dial {
            root,
            area,
            fixed,
            w,
            taf,
            doors: RefCell::new(Vec::new()),
            knocks: RefCell::new(Vec::new()),
            rotation: Cell::new(selected.get() as f64 * dial::step(1)),
            selected,
            breathe: Cell::new(0.0),
            drag: Cell::new(None),
            labels: RefCell::new(Vec::new()),
            avatar,
            center,
            turning: RefCell::new(None),
            breathing: RefCell::new(None),
            picked: RefCell::new(None),
            press: RefCell::new(None),
        });
        {
            let weak = Rc::downgrade(&d);
            d.area.set_draw_func(move |_, cr, width, height| {
                if let Some(d) = weak.upgrade() {
                    d.draw(cr, width as f64, height as f64);
                }
            });
        }
        {
            let weak = Rc::downgrade(&d);
            d.area.connect_resize(move |_, _, _| {
                if let Some(d) = weak.upgrade() {
                    d.place();
                }
            });
        }
        // Drawn again in the other appearance.
        {
            let area = d.area.downgrade();
            let id = adw::StyleManager::default().connect_dark_notify(move |_| {
                if let Some(a) = area.upgrade() {
                    a.queue_draw();
                }
            });
            let id = RefCell::new(Some(id));
            d.area.connect_destroy(move |_| {
                if let Some(id) = id.borrow_mut().take() {
                    adw::StyleManager::default().disconnect(id);
                }
            });
        }
        d.gestures();
        d.keys();
        d.breathes();
        d
    }

    fn stops(&self) -> usize {
        self.doors.borrow().len() + 1
    }

    fn geometry(&self) -> (f64, f64, f64) {
        let (w, h) = (self.area.width() as f64, self.area.height() as f64);
        let size = w.min(h);
        (w / 2.0, h / 2.0 + 6.0, size * 0.36)
    }

    fn any_open(&self) -> bool {
        self.doors.borrow().iter().any(|d| d.open)
    }

    fn draw(&self, cr: &cairo::Context, _w: f64, _h: f64) {
        let (cx, cy, r) = self.geometry();
        if r <= 0.0 {
            return;
        }
        let taf = tafs_core::theme::taf_color(self.taf.color);
        let stops = self.stops();
        let step = dial::step(stops);
        let rotation = self.rotation.get();
        // The taf's glow: it breathes while a door is open.
        let glow = if self.any_open() {
            0.12 + 0.10 * self.breathe.get()
        } else {
            0.04
        };
        let g = cairo::RadialGradient::new(cx, cy, 10.0, cx, cy, r * 1.3);
        let (tr, tg, tb) = (
            ((taf >> 16) & 0xFF) as f64 / 255.0,
            ((taf >> 8) & 0xFF) as f64 / 255.0,
            (taf & 0xFF) as f64 / 255.0,
        );
        g.add_color_stop_rgba(0.0, tr, tg, tb, glow);
        g.add_color_stop_rgba(1.0, tr, tg, tb, 0.0);
        cr.arc(cx, cy, r * 1.3, 0.0, 2.0 * PI);
        let _ = cr.set_source(&g);
        let _ = cr.fill();
        // The knob, with grip ticks that turn with it.
        let knob = r - 14.0;
        let lg = cairo::LinearGradient::new(cx - knob, cy - knob, cx + knob, cy + knob);
        for (at, name) in [(0.0, "panel"), (1.0, "raised")] {
            let (hex, _) = palette(name);
            lg.add_color_stop_rgb(
                at,
                ((hex >> 16) & 0xFF) as f64 / 255.0,
                ((hex >> 8) & 0xFF) as f64 / 255.0,
                (hex & 0xFF) as f64 / 255.0,
            );
        }
        cr.new_sub_path();
        cr.arc(cx, cy, knob, 0.0, 2.0 * PI);
        let _ = cr.set_source(&lg);
        let _ = cr.fill_preserve();
        set(cr, palette("line"), 1.0);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
        for i in 0..60 {
            let long = i % 5 == 0;
            let a = ((i as f64) * 6.0 - rotation - 90.0) * PI / 180.0;
            let len = if long { 8.0 } else { 4.0 };
            let mid = r - 26.0;
            cr.move_to(
                cx + a.cos() * (mid - len / 2.0),
                cy + a.sin() * (mid - len / 2.0),
            );
            cr.line_to(
                cx + a.cos() * (mid + len / 2.0),
                cy + a.sin() * (mid + len / 2.0),
            );
            set(cr, palette("ink"), if long { 0.16 } else { 0.07 });
            cr.set_line_width(1.0);
            cr.set_line_cap(cairo::LineCap::Round);
            let _ = cr.stroke();
        }
        cr.new_sub_path();
        cr.arc(cx, cy, r, 0.0, 2.0 * PI);
        set(cr, palette("line"), 1.0);
        cr.set_line_width(1.0);
        let _ = cr.stroke();
        // Each door's arc: lit while open, dashed while closed, grey while resting.
        let span = (step * 0.76).min(56.0);
        for (i, door) in self.doors.borrow().iter().enumerate() {
            let a = dial::angle(i, stops, rotation);
            cr.new_sub_path();
            cr.arc(
                cx,
                cy,
                r,
                (a - span / 2.0) * PI / 180.0,
                (a + span / 2.0) * PI / 180.0,
            );
            cr.set_line_cap(cairo::LineCap::Round);
            if door.open {
                if door.resting {
                    set(cr, palette("dim"), 1.0);
                } else {
                    crate::illustrations::rgb(cr, taf, 1.0);
                }
                cr.set_line_width(4.0);
                cr.set_dash(&[], 0.0);
            } else {
                set(cr, palette("ink"), 0.22);
                cr.set_line_width(2.0);
                cr.set_dash(&[3.0, 5.0], 0.0);
            }
            let _ = cr.stroke();
            cr.set_dash(&[], 0.0);
        }
        // The pointer.
        let ty = cy - r - 20.0;
        cr.move_to(cx, ty + 4.5);
        cr.line_to(cx - 6.0, ty - 4.5);
        cr.line_to(cx + 6.0, ty - 4.5);
        cr.close_path();
        set(cr, palette("ink"), 1.0);
        let _ = cr.fill();
        // The stops: each door's badge on the ring, or the dashed circle to add one.
        let doors = self.doors.borrow();
        let knocks = self.knocks.borrow();
        for i in 0..stops {
            let a = dial::angle(i, stops, rotation) * PI / 180.0;
            let (x, y) = (cx + a.cos() * r, cy + a.sin() * r);
            let size = if i == self.selected.get() { 30.0 } else { 24.0 };
            match doors.get(i) {
                Some(door) => {
                    badge_shape(cr, &door.app, door.open, x, y, size);
                    let knocking = knocks.iter().any(|k| k.door == door.id && !k.is_line());
                    if knocking {
                        cr.new_sub_path();
                        cr.arc(
                            x + size / 2.0 - 1.5,
                            y - size / 2.0 + 1.5,
                            4.5,
                            0.0,
                            2.0 * PI,
                        );
                        set(cr, palette("amber"), 1.0);
                        let _ = cr.fill();
                    }
                }
                None => {
                    cr.new_sub_path();
                    cr.arc(x, y, size / 2.0, 0.0, 2.0 * PI);
                    set(cr, palette("panel"), 1.0);
                    let _ = cr.fill_preserve();
                    crate::illustrations::rgb(cr, taf, 1.0);
                    cr.set_line_width(1.5);
                    cr.set_dash(&[3.0, 3.0], 0.0);
                    let _ = cr.stroke();
                    cr.set_dash(&[], 0.0);
                }
            }
        }
    }

    /// Puts the labels where the ring has its stops now: each badge's letter on it, each stop's
    /// name outside the ring, the taf and its line in the middle.
    fn place(&self) {
        let (cx, cy, r) = self.geometry();
        let stops = self.stops();
        let rotation = self.rotation.get();
        let put = |w: &gtk::Widget, x: f64, y: f64| {
            let (_, nat) = w.preferred_size();
            self.fixed.move_(
                w,
                x - nat.width() as f64 / 2.0,
                y - nat.height() as f64 / 2.0,
            );
        };
        for (i, (mark, name)) in self.labels.borrow().iter().enumerate() {
            let a = dial::angle(i, stops, rotation) * PI / 180.0;
            put(mark.upcast_ref(), cx + a.cos() * r, cy + a.sin() * r);
            put(
                name.upcast_ref(),
                cx + a.cos() * (r + 40.0),
                cy + a.sin() * (r + 30.0),
            );
            let selected = i == self.selected.get();
            name.set_opacity(if selected && (a.sin() + 1.0).abs() < 0.05 {
                0.0
            } else {
                1.0
            });
        }
        put(self.avatar.upcast_ref(), cx, cy - 6.0);
        put(self.center.upcast_ref(), cx, cy + 38.0);
    }

    /// The labels and words again, for the doors as they are and the stop at the top.
    fn words(&self) {
        let doors = self.doors.borrow().clone();
        let selected = self.selected.get();
        let Ok(lines) = doors::dial_lines(&self.w, &doors, selected, &self.taf.name) else {
            return;
        };
        let taf = tafs_core::theme::taf_color(self.taf.color);
        for (mark, name) in self.labels.borrow_mut().drain(..) {
            self.fixed.remove(&mark);
            self.fixed.remove(&name);
        }
        let mut labels = Vec::new();
        for (i, stop) in lines.stops.iter().enumerate() {
            let selected = i == selected;
            let size = if selected { 30.0 } else { 24.0 };
            let mark = match doors.get(i) {
                Some(d) => {
                    let m = sized(
                        &doors::mark(&d.app),
                        size * 0.46,
                        gtk::pango::Weight::Heavy,
                        false,
                    );
                    if d.open {
                        hex_attr(&m, 0xFFFFFF);
                    } else {
                        m.add_css_class("tafs-dim");
                    }
                    m
                }
                None => {
                    let m = sized("+", 14.0, gtk::pango::Weight::Bold, false);
                    hex_attr(&m, taf);
                    m
                }
            };
            let name = sized(
                stop,
                if selected { 11.0 } else { 9.5 },
                if selected {
                    gtk::pango::Weight::Semibold
                } else {
                    gtk::pango::Weight::Normal
                },
                true,
            );
            name.add_css_class(if selected { "tafs-ink" } else { "tafs-dim" });
            self.fixed.put(&mark, 0.0, 0.0);
            self.fixed.put(&name, 0.0, 0.0);
            labels.push((mark, name));
        }
        *self.labels.borrow_mut() = labels;
        self.center.set_label(&lines.center);
        self.center.remove_css_class("tafs-dim");
        let attrs = gtk::pango::AttrList::new();
        attrs.insert(gtk::pango::AttrSize::new_size_absolute(
            10 * gtk::pango::SCALE,
        ));
        attrs.insert(gtk::pango::AttrInt::new_weight(
            gtk::pango::Weight::Semibold,
        ));
        attrs.insert(gtk::pango::AttrString::new_family("monospace"));
        self.center.set_attributes(Some(&attrs));
        if lines.center_lit {
            hex_attr(&self.center, taf);
        } else {
            self.center.add_css_class("tafs-dim");
        }
        self.root.update_property(&[
            gtk::accessible::Property::Label(&lines.label),
            gtk::accessible::Property::ValueText(&lines.value),
            gtk::accessible::Property::Description(&lines.hint_spoken),
        ]);
        self.place();
        self.area.queue_draw();
    }

    /// New doors, or knocks: the stop at the top kept where it can be.
    fn set(self: &Rc<Self>, doors: Vec<Door>, knocks: Vec<Knock>) {
        let stops_before = self.stops();
        *self.doors.borrow_mut() = doors;
        *self.knocks.borrow_mut() = knocks;
        if self.stops() != stops_before {
            let at = self.selected.get().min(self.stops() - 1);
            self.snap(at as i64, false);
        }
        self.words();
    }

    fn turned(&self, now: usize) {
        if now != self.selected.get() {
            self.selected.set(now);
            self.words();
            let f = self.picked.borrow().clone();
            if let Some(f) = f {
                f();
            }
        } else {
            self.place();
            self.area.queue_draw();
        }
    }

    /// Turns to stop `raw` (any turn), springing there unless `animate` is false or motion is
    /// reduced.
    fn snap(self: &Rc<Self>, raw: i64, animate: bool) {
        let target = raw as f64 * dial::step(self.stops());
        if let Some(a) = self.turning.borrow_mut().take() {
            a.pause();
        }
        let now = dial::stop(target, self.stops());
        let moving = gtk::Settings::default()
            .map(|s| s.is_gtk_enable_animations())
            .unwrap_or(true);
        if !animate || !moving || !self.area.is_mapped() {
            self.rotation.set(target);
            self.turned(now);
            return;
        }
        let weak = Rc::downgrade(self);
        let anim = adw::SpringAnimation::new(
            &self.area,
            self.rotation.get(),
            target,
            adw::SpringParams::new(0.78, 1.0, 322.0),
            adw::CallbackAnimationTarget::new(move |v| {
                if let Some(d) = weak.upgrade() {
                    d.rotation.set(v);
                    d.place();
                    d.area.queue_draw();
                }
            }),
        );
        self.turning.replace(Some(anim.clone()));
        self.turned(now);
        anim.play();
    }

    /// `turn(to:)`: the shortest way to stop `i`.
    fn turn_to(self: &Rc<Self>, i: usize) {
        let current = (self.rotation.get() / dial::step(self.stops())).round() as i64;
        self.snap(
            current + dial::shortest(current, i as i64, self.stops()),
            true,
        );
    }

    fn gestures(self: &Rc<Self>) {
        let drag = gtk::GestureDrag::new();
        {
            let weak = Rc::downgrade(self);
            drag.connect_drag_begin(move |_, _, _| {
                if let Some(d) = weak.upgrade() {
                    d.drag.set(None);
                }
            });
        }
        {
            let weak = Rc::downgrade(self);
            drag.connect_drag_update(move |g, dx, dy| {
                let Some(d) = weak.upgrade() else { return };
                let Some((sx, sy)) = g.start_point() else {
                    return;
                };
                if d.drag.get().is_none() && dx.hypot(dy) < 6.0 {
                    return;
                }
                let (cx, cy, _) = d.geometry();
                let angle = |x: f64, y: f64| (y - cy).atan2(x - cx) * 180.0 / PI;
                if d.drag.get().is_none() {
                    if let Some(a) = d.turning.borrow_mut().take() {
                        a.pause();
                    }
                    d.drag.set(Some((angle(sx, sy), d.rotation.get())));
                }
                let Some((start, rotation)) = d.drag.get() else {
                    return;
                };
                d.rotation
                    .set(rotation - dial::delta(start, angle(sx + dx, sy + dy)));
                let now = dial::stop(d.rotation.get(), d.stops());
                d.turned(now);
            });
        }
        {
            let weak = Rc::downgrade(self);
            drag.connect_drag_end(move |g, _, _| {
                let Some(d) = weak.upgrade() else { return };
                if d.drag.take().is_some() {
                    // `settle`.
                    let raw = (d.rotation.get() / dial::step(d.stops())).round() as i64;
                    d.snap(raw, true);
                    return;
                }
                // Not a turn: a tap, on the taf or on a stop.
                let Some((x, y)) = g.start_point() else {
                    return;
                };
                d.root.grab_focus();
                let (cx, cy, r) = d.geometry();
                if (x - cx).hypot(y - (cy - 6.0)) <= 28.0 {
                    let f = d.press.borrow().clone();
                    if let Some(f) = f {
                        f();
                    }
                    return;
                }
                let stops = d.stops();
                for i in 0..stops {
                    let a = dial::angle(i, stops, d.rotation.get()) * PI / 180.0;
                    if (x - (cx + a.cos() * r)).hypot(y - (cy + a.sin() * r)) <= 18.0 {
                        d.turn_to(i);
                        return;
                    }
                }
            });
        }
        self.root.add_controller(drag);
    }

    /// The adjustable action: up or right a stop on, down or left one back; Enter or Space
    /// presses.
    fn keys(self: &Rc<Self>) {
        let keys = gtk::EventControllerKey::new();
        let weak = Rc::downgrade(self);
        keys.connect_key_pressed(move |_, key, _, _| {
            let Some(d) = weak.upgrade() else {
                return glib::Propagation::Proceed;
            };
            let current = (d.rotation.get() / dial::step(d.stops())).round() as i64;
            use gtk::gdk::Key as K;
            match key {
                K::Up | K::Right => d.snap(current + 1, true),
                K::Down | K::Left => d.snap(current - 1, true),
                K::Return | K::KP_Enter | K::space => {
                    let f = d.press.borrow().clone();
                    if let Some(f) = f {
                        f();
                    }
                }
                _ => return glib::Propagation::Proceed,
            }
            glib::Propagation::Stop
        });
        self.root.add_controller(keys);
    }

    /// The glow breathes over 2.4 s each way while it's on screen and motion isn't reduced.
    fn breathes(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        let target = adw::CallbackAnimationTarget::new(move |v| {
            if let Some(d) = weak.upgrade() {
                d.breathe.set(v);
                if d.any_open() {
                    d.area.queue_draw();
                }
            }
        });
        let a = adw::TimedAnimation::new(&self.area, 0.0, 1.0, 2400, target);
        a.set_repeat_count(0);
        a.set_alternate(true);
        a.set_easing(adw::Easing::EaseInOutCubic);
        {
            let a = a.downgrade();
            self.area.connect_map(move |_| {
                if let Some(a) = a.upgrade() {
                    a.play();
                }
            });
        }
        {
            let a = a.downgrade();
            self.area.connect_unmap(move |_| {
                if let Some(a) = a.upgrade() {
                    a.reset();
                }
            });
        }
        self.breathing.replace(Some(a));
    }
}

thread_local! {
    /// A screenshot's button pressed once the door is drawn (`--page doors:<taf>:<door>:<name>`):
    /// `door.api.new_key`, `door.api.instructions`.
    pub static PRESS: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// The screenshot's button, pressed once it's drawn: found by its name under `under`.
fn press_for_screenshot(under: &gtk::Widget) {
    let Some(name) = PRESS.with(|p| p.borrow().clone()) else {
        return;
    };
    fn find(w: &gtk::Widget, name: &str) -> Option<gtk::Button> {
        if w.widget_name() == name {
            if let Ok(b) = w.clone().downcast::<gtk::Button>() {
                return Some(b);
            }
        }
        let mut c = w.first_child();
        while let Some(child) = c {
            if let Some(b) = find(&child, name) {
                return Some(b);
            }
            c = child.next_sibling();
        }
        None
    }
    if let Some(b) = find(under, &name) {
        PRESS.with(|p| *p.borrow_mut() = None);
        glib::idle_add_local_once(move || b.emit_clicked());
    }
}

/// `DoorsView`: the doors' dial over the door it's turned to, or one door's screen (`focus`).
/// Read when it's shown.
pub fn view(model: DoorsModel, taf: DoorsTaf, opens: Opens) -> gtk::Box {
    let w = model.w.clone();
    let root = gtk::Box::new(gtk::Orientation::Vertical, 16);
    let selected = Rc::new(Cell::new(0usize));
    let slot = crate::approval::refusal_slot();
    let details = gtk::Box::new(gtk::Orientation::Vertical, 16);
    let dial = if opens.focus.is_none() && opens.knob.is_none() {
        let p = panel(false);
        p.append(&eyebrow(
            &w.line("door.doors_turn_pick_press_open")
                .unwrap_or_default(),
            false,
        ));
        let d = Dial::new(w.clone(), taf.clone(), selected.clone());
        p.append(&d.root);
        let hint = text("", 12, 400, &["tafs-dim"]);
        hint.set_xalign(0.5);
        hint.set_justify(gtk::Justification::Center);
        p.append(&hint);
        root.append(&p);
        Some((d, hint))
    } else {
        None
    };
    let taf_dial = opens.knob.clone().filter(|_| opens.focus.is_none());
    if taf_dial.is_none() {
        root.append(&details);
    }
    let ctx = Rc::new(Ctx {
        model: model.clone(),
        taf,
        opens,
        selected,
        slot,
        details,
        root: root.downgrade(),
        dial: dial.as_ref().map(|(d, _)| d.clone()),
        hint: dial.as_ref().map(|(_, h)| h.clone()),
        copied: Rc::new(Cell::new(false)),
    });
    if let Some((d, _)) = &dial {
        let weak = Rc::downgrade(&ctx);
        *d.picked.borrow_mut() = Some(Rc::new(move || {
            if let Some(c) = weak.upgrade() {
                c.draw();
            }
        }));
        let weak = Rc::downgrade(&ctx);
        *d.press.borrow_mut() = Some(Rc::new(move || {
            if let Some(c) = weak.upgrade() {
                c.press();
            }
        }));
    }
    let listening = {
        let weak = Rc::downgrade(&ctx);
        model.listen(Rc::new(move || {
            if let Some(c) = weak.upgrade() {
                if let Some(d) = &c.dial {
                    d.set(c.model.doors(), c.model.open_knocks());
                }
                c.draw();
            }
        }))
    };
    // The view keeps its own state while it's there, and stops listening once it's gone.
    // SAFETY: only ever read back as the type it's stored as, and only here.
    unsafe { root.set_data("doors-view", ctx.clone()) };
    {
        let model = model.clone();
        root.connect_destroy(move |_| model.unlisten(listening));
    }
    if let Some(d) = &ctx.dial {
        d.set(Vec::new(), Vec::new());
    }
    // The taf's dial in the doors' own dial's place (`TafDialPanel`), the door's panels under it;
    // a door's stop on When picks that door.
    if let Some((knobs, held)) = taf_dial {
        let weak = Rc::downgrade(&ctx);
        root.append(&crate::dial::panel(
            knobs,
            model.clone(),
            ctx.opens.manages,
            held,
            ctx.details.clone(),
            Rc::new(move |door: &str| {
                if let Some(c) = weak.upgrade() {
                    if let Some(i) = c.model.doors().iter().position(|d| d.id == door) {
                        c.selected.set(i);
                        c.draw();
                    }
                }
            }),
        ));
    }
    ctx.draw();
    model.load();
    root
}

/// What `DoorsView` draws from.
struct Ctx {
    model: DoorsModel,
    taf: DoorsTaf,
    opens: Opens,
    selected: Rc<Cell<usize>>,
    slot: gtk::Box,
    details: gtk::Box,
    /// The view itself, which keeps this: sheets open over it.
    root: glib::WeakRef<gtk::Box>,
    dial: Option<Rc<Dial>>,
    hint: Option<gtk::Label>,
    copied: Rc<Cell<bool>>,
}

impl Ctx {
    fn l(&self, k: &str) -> String {
        self.model.w.line(k).unwrap_or_default()
    }

    fn door(&self) -> Option<Door> {
        let doors = self.model.doors();
        match &self.opens.focus {
            Some(id) => doors.into_iter().find(|d| &d.id == id),
            None => doors.into_iter().nth(self.selected.get()),
        }
    }

    /// `press`: for whoever manages it, Add a door at the last stop, else the door opened or
    /// closed.
    fn press(self: &Rc<Self>) {
        if !self.opens.manages {
            return;
        }
        match self.door() {
            None => self.add(),
            Some(door) => self.model.set_open(&door, !door.open),
        }
    }

    fn add(self: &Rc<Self>) {
        let weak = Rc::downgrade(self);
        add_door_sheet(
            &self.model,
            self.opens.add_slack.clone(),
            Rc::new(move |added| {
                if let (true, Some(c)) = (added, weak.upgrade()) {
                    let n = c.model.doors().len();
                    if let Some(d) = &c.dial {
                        d.snap(n.saturating_sub(1) as i64, true);
                    } else {
                        c.selected.set(n.saturating_sub(1));
                    }
                    c.draw();
                }
            }),
            &self.parent(),
        );
    }

    /// What a sheet opens over.
    fn parent(&self) -> gtk::Widget {
        match self.root.upgrade() {
            Some(r) => r.upcast(),
            None => self.details.clone().upcast(),
        }
    }

    fn draw(self: &Rc<Self>) {
        let w = &self.model.w;
        let name = self.taf.name.as_str();
        while let Some(c) = self.details.first_child() {
            self.details.remove(&c);
        }
        let door = self.door();
        if let Some(hint) = &self.hint {
            if let Some(d) = &self.dial {
                if let Ok(lines) =
                    doors::dial_lines(w, &d.doors.borrow(), self.selected.get(), name)
                {
                    hint.set_label(&lines.hint);
                }
            }
        }
        // tafs cloud's line saying the door's settings are below; the taf's dial says its own.
        let shows_dial = self.opens.knob.is_some() && self.opens.focus.is_none();
        if let Some(settings) = door
            .as_ref()
            .and_then(|d| d.words.as_ref())
            .filter(|_| !shows_dial)
        {
            let l = text(&settings.settings, 12, 400, &["tafs-dim"]);
            l.set_xalign(0.5);
            l.set_justify(gtk::Justification::Center);
            self.details.append(&l);
        }
        match &door {
            Some(door) => self.door_panels(door),
            None if self.opens.focus.is_some() => {
                self.details
                    .append(&text(&self.l("door.door_removed"), 15, 400, &["tafs-soft"]));
            }
            None if self.opens.manages && !shows_dial => self.details.append(&self.add_panel()),
            None if self.opens.manages => {}
            None => self.details.append(&text(
                &w.get("door.no_door_yet", &[("taf", name)])
                    .unwrap_or_default(),
                15,
                400,
                &["tafs-soft"],
            )),
        }
        self.details.append(&self.slot);
        // Strangers' knocks wait on the owner; a paired sender's message, kept from its knock,
        // waits for Do it or Ignore, apart and calmly.
        let open = self.model.open_knocks();
        let busy = self.model.state.borrow().busy;
        let paired: Vec<&Knock> = open.iter().filter(|k| !k.is_line() && k.paired).collect();
        let waiting: Vec<&Knock> = open.iter().filter(|k| !k.is_line() && !k.paired).collect();
        if !paired.is_empty() {
            let p = panel(false);
            p.append(&eyebrow(&self.l("door.paired"), false));
            for k in paired {
                p.append(&crate::knocks::knock_card(
                    &self.model,
                    Some(name),
                    k,
                    busy,
                    true,
                    &self.parent(),
                ));
            }
            self.details.append(&p);
        }
        if !waiting.is_empty() {
            let p = panel(false);
            p.add_css_class("knocks-edge");
            p.append(&eyebrow(&self.l("door.knocks_waiting"), true));
            for k in waiting {
                p.append(&crate::knocks::knock_card(
                    &self.model,
                    Some(name),
                    k,
                    busy,
                    true,
                    &self.parent(),
                ));
            }
            self.details.append(&p);
        }
        if let Some(e) = self.model.state.borrow().error.clone() {
            self.details.append(&text(&e, 13, 400, &["tafs-no"]));
        }
        self.details.append(&text(
            &w.get("door.footer", &[("taf", name)]).unwrap_or_default(),
            13,
            400,
            &["tafs-dim"],
        ));
    }

    /// `addPanel`: what a door is, and Add a door, once the doors are read and one can be.
    fn add_panel(self: &Rc<Self>) -> gtk::Box {
        let p = panel(false);
        p.append(&eyebrow(&self.l("door.add_door"), false));
        p.append(&text(
            &self
                .model
                .w
                .get("door.add_intro", &[("taf", &self.taf.name)])
                .unwrap_or_default(),
            15,
            400,
            &["tafs-ink"],
        ));
        let add = primary(&self.l("door.add_door"));
        let none = self.model.loaded() && self.model.addable().is_empty();
        add.set_sensitive(self.model.loaded() && !none);
        {
            let weak = Rc::downgrade(self);
            add.connect_clicked(move |_| {
                if let Some(c) = weak.upgrade() {
                    c.add();
                }
            });
        }
        p.append(&add);
        if none {
            p.append(&text(&self.l("door.no_more"), 13, 400, &["tafs-dim"]));
        }
        p
    }

    /// `panel(_:)`: the door, its address and its day; then who may give it work, and Remove.
    fn door_panels(self: &Rc<Self>, door: &Door) {
        if doors::api::is_api(door) {
            return self.api_panels(door);
        }
        let w = &self.model.w;
        let manages = self.opens.manages;
        let busy = self.model.state.borrow().busy;
        let Ok(lines) = doors::door_lines(
            w,
            door,
            &self.taf.name,
            manages,
            self.opens.my_email.as_deref(),
        ) else {
            return;
        };
        let p = panel(false);
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        head.append(&badge(&door.app, door.open, 34));
        let names = gtk::Box::new(gtk::Orientation::Vertical, 2);
        names.set_hexpand(true);
        names.set_valign(gtk::Align::Center);
        names.append(&eyebrow(&lines.named, false));
        names.append(&text(&lines.state, 13, 500, &["tafs-soft"]));
        head.append(&names);
        if manages {
            let s = gtk::Switch::new();
            s.set_active(door.open);
            s.set_valign(gtk::Align::Center);
            s.set_sensitive(!busy);
            s.add_css_class("tafs-ok-switch");
            s.update_property(&[gtk::accessible::Property::Label(
                &self.l("door.status.open"),
            )]);
            let (model, d) = (self.model.clone(), door.clone());
            s.connect_state_set(move |_, on| {
                if on != d.open {
                    model.set_open(&d, on);
                }
                glib::Propagation::Stop
            });
            head.append(&s);
        }
        p.append(&head);
        let address = text(&lines.address, 15, 500, &["tafs-ink", "tafs-mono"]);
        address.set_selectable(true);
        p.append(&address);
        if let Some(copy) = &lines.copy {
            let copied_words = self.l("common.copied");
            let b = outline(if self.copied.get() {
                &copied_words
            } else {
                copy
            });
            let (addr, copied, copy, done) = (
                door.address.clone(),
                self.copied.clone(),
                copy.clone(),
                self.l("common.copied"),
            );
            b.connect_clicked(move |b| {
                b.clipboard().set_text(&addr);
                copied.set(true);
                b.set_label(&done);
                let (b, copied, copy) = (b.downgrade(), copied.clone(), copy.clone());
                glib::timeout_add_local_once(std::time::Duration::from_millis(1400), move || {
                    copied.set(false);
                    if let Some(b) = b.upgrade() {
                        b.set_label(&copy);
                    }
                });
            });
            p.append(&b);
        }
        let hex = if lines.resting {
            palette("dim").0
        } else {
            tafs_core::theme::taf_color(self.taf.color)
        };
        p.append(&meter(lines.fraction, hex));
        p.append(&text(&lines.day, 12, 400, &["tafs-dim"]));
        if let Some(others) = &lines.others {
            p.append(&text(others, 12, 400, &["tafs-dim"]));
        }
        self.details.append(&p);

        let k = panel(false);
        k.append(&eyebrow(&lines.keys_title, false));
        if lines.holds_keys {
            for (row, key) in lines.keys.iter().zip(door.keys.iter()) {
                let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
                let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
                v.set_hexpand(true);
                v.append(&text(
                    row.name.as_deref().unwrap_or(&row.id),
                    15,
                    600,
                    &["tafs-ink"],
                ));
                if row.name.is_some() {
                    v.append(&text(&row.id, 12, 400, &["tafs-dim", "tafs-mono"]));
                }
                line.append(&v);
                if row.unpair {
                    let b = gtk::Button::with_label(&self.l("door.unpair"));
                    b.add_css_class("tafs-knock-ignore");
                    b.set_valign(gtk::Align::Center);
                    b.set_sensitive(!busy);
                    let (model, d, key) = (self.model.clone(), door.clone(), key.clone());
                    b.connect_clicked(move |_| model.unpair(&d, &key));
                    line.append(&b);
                }
                k.append(&line);
            }
            k.append(&text(&lines.keys_foot, 12, 400, &["tafs-dim"]));
            if manages {
                let b = outline(&self.l("door.pair_someone"));
                let (model, d, root) = (self.model.clone(), door.clone(), self.parent());
                b.connect_clicked(move |_| pair_sheet(&model, &d, &root));
                k.append(&b);
            }
        } else {
            k.append(&text(&lines.keys_foot, 15, 400, &["tafs-ink"]));
        }
        if manages {
            let b = gtk::Button::with_label(&self.l("door.remove_door"));
            b.add_css_class("tafs-link");
            b.add_css_class("tafs-no");
            b.set_halign(gtk::Align::Start);
            let (me, d) = (Rc::downgrade(self), door.clone());
            b.connect_clicked(move |_| {
                if let Some(c) = me.upgrade() {
                    c.confirm_remove(&d);
                }
            });
            k.append(&b);
        }
        self.details.append(&k);
    }

    /// `APIDoorPanel`: a door "From another application". Its kind and name, the open switch,
    /// today's requests against the limit and the last request in tafs cloud's words; then its
    /// keys (made, last used, last four: never a key), New key, the API instructions and Remove.
    /// Pausing goes from here, unsigned. Opening it again, a new key, revoking one and removing
    /// the door are signed: this computer says it can't, with "Approve on your phone instead",
    /// and sends nothing (`signed::Act::at_api_door`).
    fn api_panels(self: &Rc<Self>, door: &Door) {
        use doors::api;
        use tafs_core::signed::Act;
        let w = self.model.w.clone();
        let manages = self.opens.manages;
        let busy = self.model.state.borrow().busy;
        let refuse = {
            let (w, slot) = (w.clone(), self.slot.clone());
            Rc::new(move |act: Act, what: Option<String>| {
                crate::approval::refuse_as(&w, tafs_core::sidebar::Mode::Kiff, &act, what, &slot);
            })
        };
        let (taf_id, door_id) = (self.model.id.clone(), door.id.clone());

        let p = panel(false);
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        head.append(&badge(&door.app, door.open, 34));
        let names = gtk::Box::new(gtk::Orientation::Vertical, 2);
        names.set_hexpand(true);
        names.set_valign(gtk::Align::Center);
        names.append(&eyebrow(&self.l("door.api.kind"), false));
        names.append(&text(&door.label, 17, 600, &["tafs-ink"]));
        head.append(&names);
        if manages {
            let s = gtk::Switch::new();
            s.set_active(door.open);
            s.set_valign(gtk::Align::Center);
            s.set_sensitive(!busy);
            s.add_css_class("tafs-ok-switch");
            s.set_widget_name("door.api.open");
            s.update_property(&[gtk::accessible::Property::Label(
                &self.l("door.status.open"),
            )]);
            let (model, d, refuse, w) =
                (self.model.clone(), door.clone(), refuse.clone(), w.clone());
            let (taf_id, door_id) = (taf_id.clone(), door_id.clone());
            s.connect_state_set(move |s, on| {
                if on == d.open {
                    return glib::Propagation::Stop;
                }
                if on {
                    // Opening it again is signed: it stays paused here.
                    s.set_active(false);
                    refuse(
                        Act::ReopenDoor {
                            taf: taf_id.clone(),
                            door: door_id.clone(),
                        },
                        w.get("door.api.open_reason", &[("app", &d.label)]).ok(),
                    );
                } else {
                    model.set_open(&d, false);
                }
                glib::Propagation::Stop
            });
            head.append(&s);
        }
        p.append(&head);
        if !door.open {
            p.append(&text(
                &self.l("door.api.paused"),
                14,
                400,
                &["tafs-amber-text"],
            ));
        }
        let today = api::today(door);
        p.append(&eyebrow(&self.l("door.api.today"), false));
        p.append(&meter(
            today as f64 / door.limit.max(1) as f64,
            tafs_core::theme::taf_color(self.taf.color),
        ));
        let limit = tafs_core::locale::number(door.limit, 0, w.language());
        if let Ok(of) = w.count("door.api.of_limit", today, &[("limit", &limit)]) {
            p.append(&text(&of, 13, 500, &["tafs-soft"]));
        }
        // Only in tafs cloud's words: none is drawn until they're sent.
        if let Ok(Some(last)) = api::last_request(&w, door) {
            let l = text(&last, 13, 400, &["tafs-soft"]);
            l.set_widget_name("door.api.last_request");
            p.append(&l);
        }
        self.details.append(&p);

        let k = panel(false);
        k.append(&eyebrow(&self.l("door.api.keys"), false));
        for key in &door.keys {
            let line = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
            v.set_hexpand(true);
            v.append(&text(
                key.name.as_deref().unwrap_or(&key.id),
                15,
                600,
                &["tafs-ink"],
            ));
            if let Ok(l) = api::key_line(&w, key) {
                v.append(&text(&l, 12, 400, &["tafs-dim"]));
            }
            line.append(&v);
            if manages {
                let b = gtk::Button::with_label(&self.l("taf.revoke"));
                b.add_css_class("tafs-link");
                b.add_css_class("tafs-no");
                b.set_valign(gtk::Align::Start);
                b.set_sensitive(!busy);
                b.set_widget_name(&format!("door.api.revoke.{}", key.id));
                let (me, key, refuse) = (Rc::downgrade(self), key.clone(), refuse.clone());
                let (taf_id, door_id) = (taf_id.clone(), door_id.clone());
                b.connect_clicked(move |_| {
                    let Some(c) = me.upgrade() else { return };
                    let name = api::name(&key);
                    let w = &c.model.w;
                    let ask = adw::AlertDialog::new(
                        Some(
                            &w.get("door.api.revoke_title", &[("key", &name)])
                                .unwrap_or_default(),
                        ),
                        Some(&c.l("door.api.revoke_body")),
                    );
                    ask.add_response("cancel", &c.l("common.cancel"));
                    ask.add_response("revoke", &c.l("taf.revoke"));
                    ask.set_response_appearance("revoke", adw::ResponseAppearance::Destructive);
                    ask.set_default_response(Some("cancel"));
                    ask.set_close_response("cancel");
                    let (refuse, taf_id, door_id, key_id) = (
                        refuse.clone(),
                        taf_id.clone(),
                        door_id.clone(),
                        key.id.clone(),
                    );
                    let what = w.get("door.api.revoke_reason", &[("key", &name)]).ok();
                    ask.connect_response(None, move |_, answer| {
                        if answer == "revoke" {
                            refuse(
                                Act::RevokeDoorKey {
                                    taf: taf_id.clone(),
                                    door: door_id.clone(),
                                    key: key_id.clone(),
                                },
                                what.clone(),
                            );
                        }
                    });
                    ask.present(Some(&c.parent()));
                });
                line.append(&b);
            }
            k.append(&line);
        }
        if manages {
            let b = outline(&self.l("door.api.new_key"));
            b.set_widget_name("door.api.new_key");
            b.set_sensitive(!busy && door.keys.len() < api::MAX_KEYS);
            let (refuse, w2, label) = (refuse.clone(), w.clone(), door.label.clone());
            let (taf_id, door_id) = (taf_id.clone(), door_id.clone());
            b.connect_clicked(move |_| {
                refuse(
                    Act::NewDoorKey {
                        taf: taf_id.clone(),
                        door: door_id.clone(),
                    },
                    w2.get("door.api.new_key_reason", &[("app", &label)]).ok(),
                );
            });
            k.append(&b);
        }
        k.append(&text(&self.l("door.api.two_keys"), 12, 400, &["tafs-dim"]));
        let boundary = w
            .get("door.api.boundary", &[("taf", &self.taf.name)])
            .unwrap_or_default();
        k.append(&text(&boundary, 13, 400, &["tafs-soft"]));
        let instructions = gtk::Button::new();
        instructions.add_css_class("flat");
        instructions.set_widget_name("door.api.instructions");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let title = text(&self.l("door.api.instructions"), 15, 500, &["tafs-ink"]);
        title.set_hexpand(true);
        row.append(&title);
        let chevron =
            gtk::Image::from_icon_name(if tafs_core::locale::right_to_left(w.language()) {
                "go-previous-symbolic"
            } else {
                "go-next-symbolic"
            });
        chevron.add_css_class("tafs-dim");
        row.append(&chevron);
        instructions.set_child(Some(&row));
        {
            let (w, d, taf, parent) = (
                w.clone(),
                door.clone(),
                self.taf.name.clone(),
                self.parent(),
            );
            instructions.connect_clicked(move |_| api_instructions(&w, &d, &taf, &parent));
        }
        k.append(&instructions);
        if manages {
            let b = gtk::Button::with_label(&self.l("door.remove_door"));
            b.add_css_class("tafs-link");
            b.add_css_class("tafs-no");
            b.set_halign(gtk::Align::Start);
            let (me, d) = (Rc::downgrade(self), door.clone());
            b.connect_clicked(move |_| {
                if let Some(c) = me.upgrade() {
                    c.confirm_remove(&d);
                }
            });
            k.append(&b);
        }
        self.details.append(&k);
        press_for_screenshot(self.details.upcast_ref());
    }

    /// Remove's question: what goes, and Remove in red. A Slack door's subscription at planque
    /// goes first, signed by the device: this computer can't, so it says so and the door stays.
    fn confirm_remove(self: &Rc<Self>, door: &Door) {
        let w = &self.model.w;
        let ask = adw::AlertDialog::new(
            Some(
                &w.get("door.remove_title", &[("door", &door.label.to_lowercase())])
                    .unwrap_or_default(),
            ),
            Some(
                &w.get("door.remove_body", &[("taf", &self.taf.name)])
                    .unwrap_or_default(),
            ),
        );
        ask.add_response("cancel", &self.l("common.cancel"));
        ask.add_response("remove", &self.l("door.remove"));
        ask.set_response_appearance("remove", adw::ResponseAppearance::Destructive);
        ask.set_default_response(Some("cancel"));
        ask.set_close_response("cancel");
        let (me, d) = (Rc::downgrade(self), door.clone());
        ask.connect_response(None, move |_, response| {
            let Some(c) = me.upgrade() else { return };
            if response != "remove" {
                return;
            }
            if doors::api::is_api(&d) {
                crate::approval::refuse(
                    &c.model.w,
                    tafs_core::sidebar::Mode::Kiff,
                    &tafs_core::signed::Act::RemoveApiDoor {
                        taf: c.model.id.clone(),
                        door: d.id.clone(),
                    },
                    &c.slot,
                );
                return;
            }
            if d.kind == "slack" && c.opens.removes_slack {
                if let (Some(grant), Some(trigger)) = (&d.grant, &d.trigger) {
                    crate::approval::refuse(
                        &c.model.w,
                        tafs_core::sidebar::Mode::Kiff,
                        &tafs_core::signed::Act::StopSlackSubscription {
                            grant: grant.clone(),
                            trigger: trigger.clone(),
                        },
                        &c.slot,
                    );
                    return;
                }
            }
            let weak = Rc::downgrade(&c);
            c.model.remove(
                &d,
                Rc::new(move || {
                    if let Some(c) = weak.upgrade() {
                        let n = c.model.doors().len();
                        c.selected.set(c.selected.get().min(n));
                        c.draw();
                    }
                }),
            );
        });
        ask.present(Some(&self.parent()));
    }
}

/// `AddDoorSheet`: the app a new door opens on, those that can tell who's writing first,
/// narrowed by a search. Done closes it.
pub fn add_door_sheet(
    model: &DoorsModel,
    add_slack: Option<Rc<dyn Fn(DoorApp)>>,
    done: Rc<dyn Fn(bool)>,
    parent: &impl IsA<gtk::Widget>,
) {
    let w = model.w.clone();
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(20);
    column.set_margin_bottom(20);
    let head = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    let title = text(&l("door.add_door"), 24, 700, &["tafs-ink"]);
    title.set_hexpand(true);
    head.append(&title);
    let close = gtk::Button::with_label(&l("common.done"));
    close.add_css_class("tafs-unsent-act");
    close.set_valign(gtk::Align::Center);
    head.append(&close);
    column.append(&head);
    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some(&l("connect.apps.search")));
    search.add_css_class("tafs-search");
    column.append(&search);
    let list = gtk::Box::new(gtk::Orientation::Vertical, 8);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_propagate_natural_height(true);
    scroller.set_max_content_height(520);
    scroller.set_child(Some(&list));
    column.append(&scroller);
    crate::widgets::ground(&column);
    let dialog = adw::Dialog::new();
    dialog.set_title(&l("door.add_door"));
    dialog.set_child(Some(&column));
    dialog.set_content_width(480);
    {
        let d = dialog.downgrade();
        close.connect_clicked(move |_| {
            if let Some(d) = d.upgrade() {
                d.close();
            }
        });
    }
    let redraw: Rc<dyn Fn()> = {
        let (w, model, list, search, dialog) = (
            w.clone(),
            model.clone(),
            list.clone(),
            search.clone(),
            dialog.clone(),
        );
        Rc::new(move || {
            let l = |k: &str| w.line(k).unwrap_or_default();
            while let Some(c) = list.first_child() {
                list.remove(&c);
            }
            let query = search.text().to_string();
            let addable = model.addable();
            let busy = model.state.borrow().busy;
            let (proven, rest) = doors::add_sections(&addable, &query);
            for (title, apps) in [
                (l("door.proven_section"), proven.clone()),
                (l("door.unproven_section"), rest.clone()),
            ] {
                if apps.is_empty() {
                    continue;
                }
                let e = eyebrow(&title, false);
                e.set_margin_top(6);
                list.append(&e);
                for app in apps {
                    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                    row.append(&badge(&app.id, true, 32));
                    let v = gtk::Box::new(gtk::Orientation::Vertical, 2);
                    v.set_hexpand(true);
                    v.append(&text(&app.label, 15, 600, &["tafs-ink"]));
                    v.append(&text(&app.how, 12, 400, &["tafs-dim"]));
                    row.append(&v);
                    let b = gtk::Button::new();
                    b.set_child(Some(&row));
                    b.add_css_class("tafs-app-row");
                    b.set_sensitive(!busy);
                    let (model, add_slack, done, dialog) = (
                        model.clone(),
                        add_slack.clone(),
                        done.clone(),
                        dialog.clone(),
                    );
                    b.connect_clicked(move |_| {
                        if let (true, Some(open)) = (app.kind == "slack", &add_slack) {
                            dialog.close();
                            open(app.clone());
                            return;
                        }
                        let (done, dialog) = (done.clone(), dialog.clone());
                        model.add(
                            &app,
                            Rc::new(move |added| {
                                done(added);
                                if added {
                                    dialog.close();
                                }
                            }),
                        );
                    });
                    list.append(&b);
                }
            }
            if proven.is_empty() && rest.is_empty() {
                let none = if query.is_empty() {
                    l("door.no_more")
                } else {
                    w.get("door.no_app_called", &[("query", &query)])
                        .unwrap_or_default()
                };
                list.append(&text(&none, 13, 400, &["tafs-dim"]));
            }
            // Why an app wasn't added, here where the owner is.
            if let Some(e) = model.state.borrow().error.clone() {
                list.append(&text(&e, 13, 400, &["tafs-no"]));
            }
            let foot = text(&l("door.replies_only"), 12, 400, &["tafs-dim"]);
            foot.set_margin_top(6);
            list.append(&foot);
        })
    };
    {
        let redraw = redraw.clone();
        search.connect_search_changed(move |_| redraw());
    }
    let listening = model.listen(redraw.clone());
    {
        let model = model.clone();
        dialog.connect_closed(move |_| model.unlisten(listening));
    }
    redraw();
    dialog.present(Some(parent));
}

/// `PairSheet`: a sender paired with a door gives the taf work from now on. Pair is there while
/// the sender isn't empty and nothing is on its way, and closes the sheet once taken.
/// `APIInstructionsView`: where to send work, the example request (reading the key from the
/// caller's own secret), that the key isn't here, and what a door's key can't do.
fn api_instructions(w: &WordBook, door: &Door, taf: &str, parent: &gtk::Widget) {
    let l = |k: &str| w.line(k).unwrap_or_default();
    let endpoint = doors::api::endpoint(door);
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(20);
    column.set_margin_bottom(20);
    column.append(&text(&l("door.api.instructions"), 24, 700, &["tafs-ink"]));
    column.append(&eyebrow(&l("door.api.where"), false));
    let address = text(&endpoint, 14, 400, &["tafs-ink", "tafs-mono"]);
    address.set_selectable(true);
    address.set_widget_name("door.api.endpoint");
    // An address reads left to right in every language.
    address.set_direction(gtk::TextDirection::Ltr);
    address.set_xalign(0.0);
    column.append(&address);
    column.append(&eyebrow(&l("door.api.example"), false));
    // Code, as written: monospaced, selectable, scrolled sideways rather than wrapped.
    let code = gtk::Label::new(Some(&doors::api::example(&endpoint)));
    code.add_css_class("tafs-mono");
    code.add_css_class("tafs-ink");
    code.set_selectable(true);
    code.set_xalign(0.0);
    code.set_direction(gtk::TextDirection::Ltr);
    code.set_margin_start(12);
    code.set_margin_end(12);
    code.set_margin_top(12);
    code.set_margin_bottom(12);
    code.set_widget_name("door.api.example");
    let scroller = gtk::ScrolledWindow::new();
    // Code starts at the left in every language, scrolled from its start.
    scroller.set_direction(gtk::TextDirection::Ltr);
    scroller.set_vscrollbar_policy(gtk::PolicyType::Never);
    // Its own viewport, left to right too: one made for it would take the language's direction
    // and start scrolled to the end.
    let viewport = gtk::Viewport::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    viewport.set_direction(gtk::TextDirection::Ltr);
    viewport.set_child(Some(&code));
    scroller.set_child(Some(&viewport));
    scroller.add_css_class("tafs-raised");
    scroller.add_css_class("tafs-code-block");
    column.append(&scroller);
    column.append(&text(&l("door.api.key_not_here"), 14, 400, &["tafs-soft"]));
    column.append(&text(
        &w.get("door.api.boundary", &[("taf", taf)])
            .unwrap_or_default(),
        14,
        400,
        &["tafs-soft"],
    ));
    let done = gtk::Button::with_label(&l("common.done"));
    done.add_css_class("flat");
    column.append(&done);
    crate::widgets::ground(&column);
    let dialog = adw::Dialog::new();
    dialog.set_title(&l("door.api.instructions"));
    dialog.set_child(Some(&column));
    dialog.set_content_width(520);
    // Done takes the focus, so the address isn't shown selected.
    dialog.set_focus(Some(&done));
    {
        let d = dialog.downgrade();
        done.connect_clicked(move |_| {
            if let Some(d) = d.upgrade() {
                d.close();
            }
        });
    }
    dialog.present(Some(parent));
}

fn pair_sheet(model: &DoorsModel, door: &Door, parent: &gtk::Widget) {
    let w = model.w.clone();
    let l = |k: &str| w.line(k).unwrap_or_default();
    let column = gtk::Box::new(gtk::Orientation::Vertical, 14);
    column.set_margin_start(20);
    column.set_margin_end(20);
    column.set_margin_top(20);
    column.set_margin_bottom(20);
    column.append(&text(&l("door.pair_someone"), 24, 700, &["tafs-ink"]));
    column.append(&text(
        &w.get("door.pair_intro", &[("door", &door.label.to_lowercase())])
            .unwrap_or_default(),
        14,
        400,
        &["tafs-soft"],
    ));
    let sender = gtk::Entry::new();
    sender.set_placeholder_text(Some(&doors::pair_hint(&w, door).unwrap_or_default()));
    sender.set_input_hints(gtk::InputHints::NO_SPELLCHECK);
    column.append(&sender);
    let name = gtk::Entry::new();
    name.set_placeholder_text(Some(&l("door.name_optional")));
    column.append(&name);
    let pair = primary(&l("door.pair"));
    column.append(&pair);
    let cancel = gtk::Button::with_label(&l("common.cancel"));
    cancel.add_css_class("flat");
    column.append(&cancel);
    crate::widgets::ground(&column);
    let dialog = adw::Dialog::new();
    dialog.set_title(&l("door.pair_someone"));
    dialog.set_child(Some(&column));
    dialog.set_content_width(440);
    let refresh: Rc<dyn Fn()> = {
        let (pair, sender, state) = (pair.clone(), sender.clone(), model.state.clone());
        Rc::new(move || {
            pair.set_sensitive(!sender.text().trim().is_empty() && !state.borrow().busy);
        })
    };
    refresh();
    {
        let refresh = refresh.clone();
        sender.connect_changed(move |_| refresh());
    }
    let listening = model.listen(refresh.clone());
    {
        let model = model.clone();
        dialog.connect_closed(move |_| model.unlisten(listening));
    }
    {
        let (model, door, dialog) = (model.clone(), door.clone(), dialog.clone());
        pair.connect_clicked(move |_| {
            let (state, dialog) = (model.state.clone(), dialog.clone());
            model.pair(
                &door,
                &sender.text(),
                &name.text(),
                Rc::new(move || {
                    if state.borrow().error.is_none() {
                        dialog.close();
                    }
                }),
            );
        });
    }
    {
        let d = dialog.downgrade();
        cancel.connect_clicked(move |_| {
            if let Some(d) = d.upgrade() {
                d.close();
            }
        });
    }
    dialog.present(Some(parent));
}

/// `DoorsView` as a sheet over the taf's page: the doors' dial, or one door (`focus`), titled as
/// the iPad's navigation title is.
pub fn sheet(model: DoorsModel, taf: DoorsTaf, opens: Opens, parent: &impl IsA<gtk::Widget>) {
    let title_of = {
        let (model, focus) = (model.clone(), opens.focus.clone());
        move || {
            let w = &model.w;
            match &focus {
                Some(id) => model
                    .doors()
                    .into_iter()
                    .find(|d| &d.id == id)
                    .and_then(|d| w.get("door.named", &[("door", &d.label)]).ok())
                    .unwrap_or_else(|| w.line("door.title_plain").unwrap_or_default()),
                None => w.line("setup.doors").unwrap_or_default(),
            }
        }
    };
    let title = title_of();
    let dialog = adw::Dialog::new();
    {
        // Named by its door once the doors are read.
        let d = dialog.downgrade();
        let listening = model.listen(Rc::new(move || {
            if let Some(d) = d.upgrade() {
                d.set_title(&title_of());
            }
        }));
        let model = model.clone();
        dialog.connect_closed(move |_| model.unlisten(listening));
    }
    let content = view(model, taf, opens);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.set_margin_top(12);
    content.set_margin_bottom(12);
    let scroller = gtk::ScrolledWindow::new();
    scroller.set_hscrollbar_policy(gtk::PolicyType::Never);
    scroller.set_child(Some(&content));
    scroller.set_vexpand(true);
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&adw::HeaderBar::new());
    toolbar.set_content(Some(&scroller));
    crate::widgets::ground(&toolbar);
    dialog.set_title(&title);
    dialog.set_child(Some(&toolbar));
    dialog.set_content_width(560);
    dialog.set_content_height(1000);
    dialog.present(Some(parent));
}
