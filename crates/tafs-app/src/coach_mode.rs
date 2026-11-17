//! Coach mode (`CoachMode.swift` › `CoachPanel`, `CoachView`): Coach's conversation slides in
//! from the right over whatever is open, which stays in view behind a light shade. It opens at a
//! third of the window, or the width the person last chose (`tafs-core/src/coach_mode.rs`): its
//! left edge drags it wider, up to full screen, and back; dragged past its narrowest, it closes,
//! as a click outside it, Escape or Back do. ⤢ makes it full screen and brings it back to the
//! width it had. Closing it never ends a conversation: the sidebar's Coach opens it again.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;
use tafs_core::coach_mode::Width;
use tafs_core::words::WordBook;

thread_local! {
    /// A screenshot opens Coach mode full screen, keeping nothing.
    pub static START_FULL: Cell<bool> = const { Cell::new(false) };
}

/// What makes Coach's page with the panel's title row.
pub type MakePage = Rc<dyn Fn(Chrome) -> gtk::Widget>;

/// What the panel's title row holds besides Coach's own: Back, and ⤢ with the state it's in.
pub struct Chrome {
    pub close: Rc<dyn Fn()>,
    pub toggle_full: Rc<dyn Fn()>,
    pub full: bool,
}

/// The shade, the panel over the window, and what makes a new conversation page in it.
pub struct CoachMode {
    overlay: gtk::Overlay,
    scrim: gtk::Box,
    panel: gtk::Box,
    slot: gtk::Box,
    width: Cell<Width>,
    /// How far the edge is being dragged, while it is.
    drag: Cell<f64>,
    /// Makes Coach's page, with the panel's title row.
    make: RefCell<Option<MakePage>>,
}

impl CoachMode {
    /// Over `under`, the window's screens: the returned overlay takes its place.
    pub fn new(w: &WordBook, under: &impl IsA<gtk::Widget>) -> Rc<CoachMode> {
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(under));
        let scrim = gtk::Box::new(gtk::Orientation::Vertical, 0);
        scrim.add_css_class("tafs-scrim");
        scrim.set_visible(false);
        scrim.set_accessible_role(gtk::AccessibleRole::Button);
        scrim.update_property(&[gtk::accessible::Property::Label(
            &w.line("coach.close_coach_mode").unwrap_or_default(),
        )]);
        overlay.add_overlay(&scrim);
        let panel = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        panel.add_css_class("tafs-coach-mode");
        panel.set_visible(false);
        crate::widgets::ground(&panel);
        let handle = gtk::Box::new(gtk::Orientation::Vertical, 0);
        handle.set_size_request(16, -1);
        handle.set_widget_name("coach.width");
        handle.set_cursor_from_name(Some("col-resize"));
        handle.set_accessible_role(gtk::AccessibleRole::Separator);
        handle.update_property(&[
            gtk::accessible::Property::Label(&w.line("coach.coach_mode_width").unwrap_or_default()),
            gtk::accessible::Property::Description(
                &w.line("coach.drag_make_wider_narrower").unwrap_or_default(),
            ),
        ]);
        let capsule = gtk::Box::new(gtk::Orientation::Vertical, 0);
        capsule.add_css_class("tafs-handle");
        capsule.set_size_request(5, 44);
        capsule.set_halign(gtk::Align::Center);
        capsule.set_valign(gtk::Align::Center);
        capsule.set_vexpand(true);
        handle.append(&capsule);
        panel.append(&handle);
        let slot = gtk::Box::new(gtk::Orientation::Vertical, 0);
        slot.set_hexpand(true);
        panel.append(&slot);
        overlay.add_overlay(&panel);
        let mode = Rc::new(CoachMode {
            overlay: overlay.clone(),
            scrim: scrim.clone(),
            panel: panel.clone(),
            slot,
            width: Cell::new(if START_FULL.with(Cell::get) {
                Width {
                    full: true,
                    ..Width::default()
                }
            } else {
                crate::store::coach_width()
            }),
            drag: Cell::new(0.0),
            make: RefCell::new(None),
        });
        {
            // The panel at the window's right, its width as chosen; the shade over all of it.
            let m = Rc::downgrade(&mode);
            overlay.connect_get_child_position(move |o, child| {
                let m = m.upgrade()?;
                let (w, h) = (o.width(), o.height());
                if child == m.panel.upcast_ref::<gtk::Widget>() {
                    let width = (m.width.get().on(f64::from(w), m.drag.get()).round() as i32)
                        .clamp(1, w.max(1));
                    return Some(gtk::gdk::Rectangle::new(w - width, 0, width, h));
                }
                Some(gtk::gdk::Rectangle::new(0, 0, w, h))
            });
        }
        {
            let m = Rc::downgrade(&mode);
            let click = gtk::GestureClick::new();
            click.connect_released(move |_, _, _, _| {
                if let Some(m) = m.upgrade() {
                    if !m.width.get().full {
                        m.close();
                    }
                }
            });
            scrim.add_controller(click);
        }
        {
            let drag = gtk::GestureDrag::new();
            let m = Rc::downgrade(&mode);
            drag.connect_drag_update(move |_, dx, _| {
                if let Some(m) = m.upgrade() {
                    m.drag.set(dx);
                    m.overlay.queue_allocate();
                }
            });
            let m = Rc::downgrade(&mode);
            drag.connect_drag_end(move |_, dx, _| {
                let Some(m) = m.upgrade() else { return };
                m.drag.set(0.0);
                let screen = f64::from(m.overlay.width());
                match m.width.get().dragged(screen, dx) {
                    None => m.close(),
                    Some(chosen) => {
                        let was_full = m.width.get().full;
                        m.keep(chosen);
                        if chosen.full != was_full {
                            m.reopen();
                        }
                    }
                }
            });
            handle.add_controller(drag);
        }
        {
            let keys = gtk::EventControllerKey::new();
            let m = Rc::downgrade(&mode);
            keys.connect_key_pressed(move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape {
                    if let Some(m) = m.upgrade() {
                        m.close();
                        return glib::Propagation::Stop;
                    }
                }
                glib::Propagation::Proceed
            });
            panel.add_controller(keys);
        }
        mode
    }

    /// The overlay, in the window's content.
    pub fn widget(&self) -> gtk::Widget {
        self.overlay.clone().upcast()
    }

    /// What makes Coach's page each time the panel opens.
    pub fn set_page(&self, make: MakePage) {
        *self.make.borrow_mut() = Some(make);
    }

    pub fn is_open(&self) -> bool {
        self.panel.is_visible()
    }

    fn keep(&self, width: Width) {
        self.width.set(width);
        crate::store::set_coach_width(&width);
        self.scrim.set_opacity(if width.full { 0.0 } else { 1.0 });
        self.overlay.queue_allocate();
    }

    /// Opens Coach over what's on screen, its conversation read anew; open already, it stays.
    pub fn open(self: &Rc<Self>) {
        if self.is_open() {
            return;
        }
        self.fill();
        self.scrim
            .set_opacity(if self.width.get().full { 0.0 } else { 1.0 });
        self.scrim.set_visible(true);
        self.panel.set_visible(true);
    }

    /// The page again, with ⤢ in its new state.
    fn reopen(self: &Rc<Self>) {
        if self.is_open() {
            self.fill();
        }
    }

    fn fill(self: &Rc<Self>) {
        while let Some(c) = self.slot.first_child() {
            self.slot.remove(&c);
        }
        let Some(make) = self.make.borrow().clone() else {
            return;
        };
        let (close, toggle) = (Rc::downgrade(self), Rc::downgrade(self));
        let page = make(Chrome {
            close: Rc::new(move || {
                if let Some(m) = close.upgrade() {
                    m.close();
                }
            }),
            toggle_full: Rc::new(move || {
                if let Some(m) = toggle.upgrade() {
                    m.keep(m.width.get().toggled());
                    m.reopen();
                }
            }),
            full: self.width.get().full,
        });
        page.set_hexpand(true);
        self.slot.append(&page);
    }

    /// Closes it: the conversation stays where it is at tafs cloud, and its page goes, so its
    /// reads stop.
    pub fn close(&self) {
        self.scrim.set_visible(false);
        self.panel.set_visible(false);
        while let Some(c) = self.slot.first_child() {
            self.slot.remove(&c);
        }
    }
}

/// The title row's leading buttons: Back, and ⤢ (full screen, or back to its width).
pub fn chrome_buttons(w: &WordBook, chrome: &Chrome) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    b.set_margin_start(10);
    b.set_valign(gtk::Align::Center);
    let back = gtk::Button::new();
    back.add_css_class("tafs-bar-icon");
    back.set_child(Some(&crate::widgets::symbol("go-previous-symbolic", 18)));
    let label = w.line("common.back").unwrap_or_default();
    back.set_tooltip_text(Some(&label));
    back.update_property(&[gtk::accessible::Property::Label(&label)]);
    back.set_widget_name("coach.back");
    let close = chrome.close.clone();
    back.connect_clicked(move |_| close());
    b.append(&back);
    let full = gtk::Button::new();
    full.add_css_class("tafs-bar-icon");
    full.set_child(Some(&crate::widgets::symbol(
        if chrome.full {
            "view-restore-symbolic"
        } else {
            "view-fullscreen-symbolic"
        },
        16,
    )));
    let label = w
        .line(if chrome.full {
            "coach.back_width"
        } else {
            "coach.full_screen"
        })
        .unwrap_or_default();
    full.set_tooltip_text(Some(&label));
    full.update_property(&[gtk::accessible::Property::Label(&label)]);
    full.set_widget_name("coach.full");
    let toggle = chrome.toggle_full.clone();
    full.connect_clicked(move |_| toggle());
    b.append(&full);
    b
}
