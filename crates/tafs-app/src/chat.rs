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
