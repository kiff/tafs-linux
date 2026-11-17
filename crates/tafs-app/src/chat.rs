//! What Coach's conversation and a taf's chat draw alike (`ChatBubble`, `WorkSummary`,
//! `StepList`, `TypingBubble`, the message bar's `composer`), and how either sends to tafs cloud:
//! as the session, unsigned (`kiff::cloud_send`, which reaches tafs cloud only).

use adw::prelude::*;
use gtk::glib;
use tafs_core::coach_lines::{StepStatus, WorkLines};
use tafs_core::signin::Answer;
use tafs_core::words::WordBook;

use crate::widgets::{symbol, text};

/// What a send to tafs cloud answered: its status and body, or why it didn't get there.
pub type Sent = Result<Answer, String>;

/// A send to tafs cloud as the session, off the main thread.
pub async fn post(
    w: &WordBook,
    token: &str,
    languages: &[&'static str],
    path: String,
    body: Vec<u8>,
) -> Sent {
    let (words, token, languages) = (w.clone(), token.to_string(), languages.to_vec());
    gtk::gio::spawn_blocking(move || {
        crate::kiff::cloud_send(&token, &languages, "POST", &path, Some(&body))
            .map_err(|e| e.words(&words).unwrap_or_default())
    })
    .await
    .unwrap_or_else(|_| Err(String::new()))
}

/// The message box (`composer`) and its button, Send or Stop.
pub struct Composer {
    pub bar: gtk::Box,
    /// The row under the words: the paperclip at its start, Send or Stop at its end.
    pub row: gtk::Box,
    pub view: gtk::TextView,
    pub placeholder: gtk::Label,
    pub send: gtk::Button,
    pub stop: gtk::Button,
}

/// Which chat's box: its widgets' names, and Send's colour (Coach's own, or the brand sweep).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Whose {
    Coach,
    Taf,
}

/// The message box: the words, what it says while empty, and Send, or Stop while the chat's agent
/// works.
pub fn composer(whose: Whose) -> Composer {
    let name = match whose {
        Whose::Coach => "coach",
        Whose::Taf => "chat",
    };
    let bar = gtk::Box::new(gtk::Orientation::Vertical, 0);
    bar.add_css_class("tafs-composer-bar");
    let field = gtk::Box::new(gtk::Orientation::Vertical, 4);
    field.add_css_class("tafs-composer");
    field.set_margin_start(12);
    field.set_margin_end(12);
    field.set_margin_top(10);
    field.set_margin_bottom(10);
    let view = gtk::TextView::new();
    view.add_css_class("tafs-composer-text");
    view.set_wrap_mode(gtk::WrapMode::WordChar);
    view.set_accepts_tab(false);
    view.set_top_margin(10);
    view.set_bottom_margin(4);
    view.set_left_margin(14);
    view.set_right_margin(14);
    view.set_widget_name(&format!("{name}.composer"));
    let placeholder = text("", 16, 400, &["tafs-dim"]);
    placeholder.set_can_target(false);
    placeholder.set_halign(gtk::Align::Start);
    placeholder.set_valign(gtk::Align::Start);
    placeholder.set_margin_start(14);
    placeholder.set_margin_top(10);
    placeholder.set_accessible_role(gtk::AccessibleRole::Presentation);
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_hscrollbar_policy(gtk::PolicyType::Never);
    // One line high, growing with the words up to about six lines, then scrolling.
    scroll.set_propagate_natural_height(true);
    scroll.set_min_content_height(30);
    scroll.set_max_content_height(160);
    scroll.set_vscrollbar_policy(gtk::PolicyType::External);
    scroll.set_child(Some(&view));
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&scroll));
    overlay.add_overlay(&placeholder);
    field.append(&overlay);
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
    row.set_margin_start(8);
    row.set_margin_end(8);
    row.set_margin_bottom(8);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    row.append(&spacer);
    let send = gtk::Button::new();
    send.add_css_class("tafs-send");
    if whose == Whose::Taf {
        send.add_css_class("brand");
    }
    send.set_child(Some(&symbol("go-up-symbolic", 16)));
    send.set_widget_name(&format!("{name}.send"));
    let stop = gtk::Button::new();
    stop.add_css_class("tafs-stop");
    stop.set_child(Some(&symbol("media-playback-stop-symbolic", 14)));
    stop.set_widget_name(&format!("{name}.stop"));
    row.append(&send);
    row.append(&stop);
    field.append(&row);
    bar.append(&field);
    Composer {
        bar,
        row,
        view,
        placeholder,
        send,
        stop,
    }
}

impl Composer {
    pub fn draft(&self) -> String {
        let b = self.view.buffer();
        b.text(&b.start_iter(), &b.end_iter(), false).to_string()
    }

    pub fn set_draft(&self, words: &str) {
        self.view.buffer().set_text(words);
    }

    /// Its words and its button: Stop while the agent works and nothing is written, else Send,
    /// dimmed while there's nothing to send. With the chat's files (`FileTray`): files ready to go
    /// make Send sendable without words; while one is still going up, or files wait for the agent
    /// to finish, Send waits, its words `held`.
    pub fn show_with(
        &self,
        placeholder: &str,
        send: &str,
        stop: &str,
        thinking: bool,
        files: Option<(bool, Option<&str>)>,
    ) {
        let draft = self.draft();
        let (ready, held) = files.unwrap_or((false, None));
        let empty = draft.trim().is_empty() && !ready;
        self.placeholder.set_label(placeholder);
        self.placeholder.set_visible(draft.is_empty());
        self.view
            .update_property(&[gtk::accessible::Property::Placeholder(placeholder)]);
        self.stop.set_visible(thinking && empty);
        self.stop
            .update_property(&[gtk::accessible::Property::Label(stop)]);
        self.send.set_visible(!(thinking && empty));
        self.send.set_sensitive(!empty && held.is_none());
        self.send
            .update_property(&[gtk::accessible::Property::Label(held.unwrap_or(send))]);
    }

    /// ⌘↩ sends and ⌘. stops, as on the iPad's keyboard: Ctrl here.
    pub fn keys(&self, send: impl Fn() + 'static, stop: impl Fn() + 'static) {
        let keys = gtk::EventControllerKey::new();
        let (send_button, stop_button) = (self.send.clone(), self.stop.clone());
        keys.connect_key_pressed(move |_, key, _, state| {
            if !state.contains(gtk::gdk::ModifierType::CONTROL_MASK) {
                return glib::Propagation::Proceed;
            }
            match key {
                gtk::gdk::Key::Return | gtk::gdk::Key::KP_Enter => {
                    if send_button.is_visible() && send_button.is_sensitive() {
                        send();
                    }
                    glib::Propagation::Stop
                }
                gtk::gdk::Key::period => {
                    if stop_button.is_visible() {
                        stop();
                    }
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        });
        self.view.add_controller(keys);
    }
}

/// The owner's words on the right, on the brand sweep (`ChatBubble`), and under them that they
/// answered the card above.
pub fn owner(words: &str, answered: Option<&str>, steered: bool) -> gtk::Widget {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 4);
    let bubble = text(words, 15, 400, &["tafs-owner-bubble"]);
    if steered {
        bubble.add_css_class("steered");
    }
    bubble.set_halign(gtk::Align::End);
    bubble.set_margin_start(48);
    bubble.set_selectable(true);
    bubble.set_max_width_chars(60);
    v.append(&bubble);
    if let Some(a) = answered {
        let l = text(a, 12, 400, &["tafs-dim"]);
        l.set_xalign(1.0);
        v.append(&l);
    }
    v.upcast()
}

/// The agent's words full width, with no bubble, its Markdown as formatting, under its work
/// folded to "Worked for".
/// What a sent file's chip does when tapped: opens it, by the file and its chip.
pub type OpenFile = std::rc::Rc<dyn Fn(&tafs_core::coach::Attachment, &gtk::Button)>;

/// `MessageFiles`: the files sent with a message, under it at the end, each its icon and name on
/// the panel colour; a tap opens it full size.
pub fn message_files(
    w: &tafs_core::words::WordBook,
    files: &[tafs_core::coach::Attachment],
    open: OpenFile,
) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    v.set_halign(gtk::Align::End);
    for f in files {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.append(&crate::attach::file_icon(&f.media_type, None));
        // Its whole name up to 32 characters, then cut with "…".
        let name = crate::widgets::one_line(&f.name, 14, 500, &["tafs-ink"]);
        let shown = f.name.chars().count().min(32) as i32;
        name.set_width_chars(shown);
        name.set_max_width_chars(shown);
        row.append(&name);
        let b = gtk::Button::new();
        b.set_child(Some(&row));
        b.add_css_class("tafs-message-file");
        b.set_widget_name("chat.message-file");
        let label = w
            .get("chat.file.open", &[("name", &f.name)])
            .unwrap_or_default();
        b.update_property(&[gtk::accessible::Property::Label(&label)]);
        b.set_tooltip_text(Some(&label));
        let (open, f) = (open.clone(), f.clone());
        b.connect_clicked(move |b| open(&f, b));
        v.append(&b);
    }
    v
}

pub fn answer(work: Option<&WorkLines>, words: &str) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    if let Some(work) = work {
        v.append(&work_summary(work));
    }
    v.append(&answer_words(words));
    v
}

/// The answer's words alone: Markdown as Pango markup, 16, selectable.
pub fn answer_words(words: &str) -> gtk::Label {
    let l = text("", 16, 400, &["tafs-ink"]);
    // Its size alone: a weight over the whole would undo the words' own bold.
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrSize::new_size_absolute(
        16 * gtk::pango::SCALE,
    ));
    l.set_attributes(Some(&attrs));
    l.set_markup(&tafs_core::coach_lines::markup(words));
    l.set_selectable(true);
    l.set_margin_top(2);
    l.set_margin_bottom(2);
    l
}

/// The dots where the agent's words will be (`TypingBubble`, full width).
pub fn dots(answering: &str) -> gtk::Box {
    let dots = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    dots.set_margin_top(8);
    dots.set_margin_bottom(8);
    for _ in 0..3 {
        let d = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        d.add_css_class("tafs-typing");
        d.set_valign(gtk::Align::Center);
        dots.append(&d);
    }
    dots.set_accessible_role(gtk::AccessibleRole::Img);
    dots.update_property(&[gtk::accessible::Property::Label(answering)]);
    dots
}

/// A time between parts of the thread, centred and dim.
pub fn gap(at: &str) -> gtk::Widget {
    let l = text(at, 12, 400, &["tafs-dim"]);
    l.set_xalign(0.5);
    l.set_justify(gtk::Justification::Center);
    l.set_margin_top(6);
    l.upcast()
}

/// `WorkSummary`: "Worked for 12 s ›", which opens its steps (`StepList`).
pub fn work_summary(work: &WorkLines) -> gtk::Box {
    let v = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let b = gtk::Button::new();
    b.add_css_class("tafs-work");
    b.set_halign(gtk::Align::Start);
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    h.append(&text(&work.summary, 13, 400, &["tafs-dim"]));
    let chevron = symbol("pan-end-symbolic", 11);
    chevron.add_css_class("tafs-dim");
    h.append(&chevron);
    b.set_child(Some(&h));
    b.update_property(&[
        gtk::accessible::Property::Label(&work.accessible),
        gtk::accessible::Property::Description(&work.shows),
    ]);
    v.append(&b);
    let steps = gtk::Box::new(gtk::Orientation::Vertical, 4);
    steps.add_css_class("tafs-step-list");
    for row in &work.steps {
        let h = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        let (icon, class) = match row.status {
            StepStatus::Done => ("tafs-checkmark-symbolic", "tafs-dim"),
            StepStatus::Error => ("dialog-error-symbolic", "tafs-no"),
            StepStatus::NotMade => ("edit-undo-symbolic", "tafs-dim"),
        };
        let mark = symbol(icon, 11);
        mark.add_css_class(class);
        mark.set_size_request(14, -1);
        h.append(&mark);
        h.append(&text(&row.words, 13, 400, &["tafs-soft"]));
        if let Some(note) = &row.note {
            h.append(&text(note, 12, 400, &["tafs-dim"]));
        }
        steps.append(&h);
    }
    steps.set_visible(false);
    v.append(&steps);
    let (shows, hides) = (work.shows.clone(), work.hides.clone());
    b.connect_clicked(move |b| {
        let open = !steps.is_visible();
        steps.set_visible(open);
        chevron.set_icon_name(Some(if open {
            "pan-down-symbolic"
        } else {
            "pan-end-symbolic"
        }));
        b.update_property(&[gtk::accessible::Property::Description(if open {
            &hides
        } else {
            &shows
        })]);
    });
    v
}

/// A message's menu, on a right click or a long press: each item's words and what it does.
pub fn menu_on(widget: &impl IsA<gtk::Widget>, items: Vec<(String, std::rc::Rc<dyn Fn()>)>) {
    let widget = widget.as_ref().clone();
    let open = std::rc::Rc::new(move |x: f64, y: f64, at: &gtk::Widget| {
        let list = gtk::Box::new(gtk::Orientation::Vertical, 0);
        let popover = gtk::Popover::new();
        for (words, act) in &items {
            let b = gtk::Button::with_label(words);
            b.add_css_class("flat");
            let (act, p) = (act.clone(), popover.clone());
            b.connect_clicked(move |_| {
                p.popdown();
                act();
            });
            list.append(&b);
        }
        popover.set_child(Some(&list));
        popover.set_parent(at);
        popover.set_has_arrow(true);
        popover.set_pointing_to(Some(&gtk::gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.connect_closed(|p| {
            let p = p.clone();
            glib::idle_add_local_once(move || p.unparent());
        });
        popover.popup();
    });
    // Before a selectable label's own menu: this is the message's.
    let click = gtk::GestureClick::new();
    click.set_button(gtk::gdk::BUTTON_SECONDARY);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let (open, at) = (open.clone(), widget.clone());
        click.connect_pressed(move |g, _, x, y| {
            g.set_state(gtk::EventSequenceState::Claimed);
            open(x, y, &at);
        });
    }
    widget.add_controller(click);
    let press = gtk::GestureLongPress::new();
    press.set_propagation_phase(gtk::PropagationPhase::Capture);
    {
        let at = widget.clone();
        press.connect_pressed(move |_, x, y| open(x, y, &at));
    }
    widget.add_controller(press);
}

thread_local! {
    /// For a screenshot (`--page coach-read-back`): once landed, the thread is read back to its
    /// top, so ↓ and the bar show.
    pub static READ_BACK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Over a thread, at its foot: the pill for what waits for the owner while it's out of view
/// (`NeedsYouPill`), and ↓ back to the latest while the thread doesn't follow, with a dot when
/// something arrived below (`unseen`). The thread follows what arrives while its latest is in
/// view (`following`); scrolling up stops that, and reaching the foot again or ↓ starts it.
pub struct Jump {
    pub overlay: gtk::Overlay,
    scroller: gtk::ScrolledWindow,
    down: gtk::Button,
    dot: gtk::Box,
    pill: gtk::Button,
    pill_words: gtk::Label,
    /// The thread follows what arrives (`following`).
    pub following: std::cell::Cell<bool>,
    unseen: std::cell::Cell<bool>,
    /// Opened, it has landed: on what waits, or at the latest.
    landed: std::cell::Cell<bool>,
    /// A draw is putting the thread back: the scroll it makes isn't the owner's.
    held: std::cell::Cell<bool>,
    waiting: std::cell::RefCell<Option<gtk::Widget>>,
    latest: (String, String),
}

impl Jump {
    pub fn new(w: &WordBook, scroller: &gtk::ScrolledWindow) -> std::rc::Rc<Jump> {
        let overlay = gtk::Overlay::new();
        overlay.set_child(Some(scroller));
        overlay.set_vexpand(true);
        let foot = gtk::Box::new(gtk::Orientation::Vertical, 8);
        // As wide as what it shows, so the rest of the thread under it still takes a click.
        foot.set_valign(gtk::Align::End);
        foot.set_halign(gtk::Align::Center);
        foot.set_margin_bottom(10);
        let pill = gtk::Button::new();
        pill.add_css_class("tafs-needs-pill");
        pill.set_margin_start(16);
        pill.set_margin_end(16);
        pill.set_halign(gtk::Align::Center);
        pill.set_widget_name("needs-you-pill");
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let mark = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        mark.add_css_class("tafs-dot");
        mark.set_valign(gtk::Align::Center);
        row.append(&mark);
        let pill_words = text("", 14, 600, &["tafs-ink"]);
        pill_words.set_ellipsize(gtk::pango::EllipsizeMode::End);
        pill_words.set_hexpand(true);
        row.append(&pill_words);
        let up = symbol("pan-up-symbolic", 13);
        up.add_css_class("tafs-dim");
        row.append(&up);
        pill.set_child(Some(&row));
        if let Ok(hint) = w.line("coach.shows_it") {
            pill.update_property(&[gtk::accessible::Property::Description(&hint)]);
        }
        pill.set_visible(false);
        foot.append(&pill);
        let down = gtk::Button::new();
        down.add_css_class("tafs-jump");
        down.set_halign(gtk::Align::Center);
        down.set_widget_name("jump-latest");
        let over = gtk::Overlay::new();
        let arrow = symbol("go-down-symbolic", 15);
        arrow.add_css_class("tafs-ink");
        over.set_child(Some(&arrow));
        let dot = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        dot.add_css_class("tafs-jump-dot");
        dot.set_halign(gtk::Align::End);
        dot.set_valign(gtk::Align::Start);
        dot.set_visible(false);
        over.add_overlay(&dot);
        down.set_child(Some(&over));
        down.set_visible(false);
        foot.append(&down);
        overlay.add_overlay(&foot);
        let j = std::rc::Rc::new(Jump {
            overlay,
            scroller: scroller.clone(),
            down,
            dot,
            pill,
            pill_words,
            following: std::cell::Cell::new(true),
            unseen: std::cell::Cell::new(false),
            landed: std::cell::Cell::new(false),
            held: std::cell::Cell::new(false),
            waiting: std::cell::RefCell::new(None),
            latest: (
                w.line("chat.jump_to_latest").unwrap_or_default(),
                w.line("chat.jump_latest_something_new").unwrap_or_default(),
            ),
        });
        {
            let jw = std::rc::Rc::downgrade(&j);
            j.down.connect_clicked(move |_| {
                if let Some(j) = jw.upgrade() {
                    j.to_latest();
                }
            });
        }
        {
            let jw = std::rc::Rc::downgrade(&j);
            j.pill.connect_clicked(move |_| {
                if let Some(j) = jw.upgrade() {
                    let target = j.waiting.borrow().clone();
                    if let Some(t) = target {
                        j.to(&t);
                    }
                }
            });
        }
        // Scrolled, by the owner or by the thread: it follows while its latest is in view.
        {
            let jw = std::rc::Rc::downgrade(&j);
            scroller.vadjustment().connect_value_changed(move |_| {
                if let Some(j) = jw.upgrade() {
                    if !j.held.get() {
                        let bottom = j.at_bottom();
                        j.following.set(bottom);
                        if bottom {
                            j.unseen.set(false);
                        }
                    }
                    j.refresh();
                }
            });
        }
        {
            let jw = std::rc::Rc::downgrade(&j);
            scroller.vadjustment().connect_changed(move |_| {
                if let Some(j) = jw.upgrade() {
                    j.refresh();
                }
            });
        }
        j.refresh();
        j
    }

    /// The thread's latest is in view.
    pub fn at_bottom(&self) -> bool {
        let adj = self.scroller.vadjustment();
        adj.value() + adj.page_size() >= adj.upper() - 24.0
    }

    /// ↓: to the latest, following again.
    pub fn to_latest(&self) {
        self.following.set(true);
        self.unseen.set(false);
        let adj = self.scroller.vadjustment();
        adj.set_value(adj.upper() - adj.page_size());
        self.refresh();
    }

    /// The pill: what waits, in the middle of the view, the thread no longer following.
    pub fn to(&self, target: &gtk::Widget) {
        let Some(child) = self.scroller.child() else {
            return;
        };
        let Some(at) = target.compute_point(&child, &gtk::graphene::Point::new(0.0, 0.0)) else {
            return;
        };
        let adj = self.scroller.vadjustment();
        let middle = f64::from(at.y()) + f64::from(target.height()) / 2.0;
        let value = (middle - adj.page_size() / 2.0).clamp(0.0, adj.upper() - adj.page_size());
        adj.set_value(value);
        self.following.set(self.at_bottom());
        self.refresh();
    }

    /// What waits for the owner now, and the pill's words for it; none takes the pill away.
    pub fn waiting(&self, waits: Option<(&str, gtk::Widget)>) {
        match waits {
            Some((words, widget)) => {
                self.pill_words.set_label(words);
                self.pill
                    .update_property(&[gtk::accessible::Property::Label(words)]);
                *self.waiting.borrow_mut() = Some(widget);
            }
            None => *self.waiting.borrow_mut() = None,
        }
        self.refresh();
    }

    /// Something new arrived below while the thread didn't follow.
    pub fn arrived(&self) {
        if !self.following.get() {
            self.unseen.set(true);
            self.refresh();
        }
    }

    /// A draw is about to take the thread down and put it back: until `place`, a scroll isn't the
    /// owner's.
    pub fn hold(&self) {
        self.held.set(true);
    }

    /// Where a draw puts the view: opened, it lands once the thread is in (`ready`) on what waits
    /// (in the middle, unless it's among the last few items, which the latest shows) or at the
    /// latest; then the latest while following, else where it was.
    pub fn put(&self, keep: f64, ready: bool, near_end: bool) {
        let value = self.place(keep, ready, near_end);
        self.scroller.vadjustment().set_value(value);
        self.refresh();
    }

    fn place(&self, keep: f64, ready: bool, near_end: bool) -> f64 {
        self.held.set(false);
        let adj = self.scroller.vadjustment();
        let latest = adj.upper() - adj.page_size();
        if !self.landed.get() && !ready {
            self.following.set(true);
            return latest;
        }
        if !self.landed.replace(true) {
            if READ_BACK.with(|r| r.replace(false)) {
                self.following.set(false);
                return 0.0;
            }
            if let (Some(target), false) = (self.waiting.borrow().clone(), near_end) {
                if let Some(child) = self.scroller.child() {
                    if let Some(at) =
                        target.compute_point(&child, &gtk::graphene::Point::new(0.0, 0.0))
                    {
                        let middle = f64::from(at.y()) + f64::from(target.height()) / 2.0;
                        self.following.set(false);
                        return (middle - adj.page_size() / 2.0).clamp(0.0, latest.max(0.0));
                    }
                }
            }
            self.following.set(true);
            return latest;
        }
        if self.following.get() {
            latest
        } else {
            keep
        }
    }

    /// Opened again (another conversation): it lands again.
    pub fn reland(&self) {
        self.landed.set(false);
        self.following.set(true);
        self.unseen.set(false);
    }

    /// The pill while what waits is out of view; ↓ while the latest is out of view and the thread
    /// doesn't follow, with its dot for something new.
    fn refresh(&self) {
        let adj = self.scroller.vadjustment();
        let waits_out_of_view = self.waiting.borrow().as_ref().is_some_and(|t| {
            let Some(child) = self.scroller.child() else {
                return false;
            };
            let Some(at) = t.compute_point(&child, &gtk::graphene::Point::new(0.0, 0.0)) else {
                return false;
            };
            let (top, h) = (f64::from(at.y()), f64::from(t.height()));
            h > 0.0 && (top + h <= adj.value() || top >= adj.value() + adj.page_size())
        });
        self.pill.set_visible(waits_out_of_view);
        let down = !self.at_bottom() && !self.following.get();
        self.down.set_visible(down);
        self.dot.set_visible(down && self.unseen.get());
        let label = if self.unseen.get() {
            &self.latest.1
        } else {
            &self.latest.0
        };
        self.down
            .update_property(&[gtk::accessible::Property::Label(label)]);
        self.down.set_tooltip_text(Some(label));
    }
}
