//! "A new version of tafs is out", tell only (`tafs_core::update`): one line at the sidebar's
//! foot that opens the new release's page in the browser. The app checks at start and once a
//! day while it stays open; it downloads and replaces nothing.

use std::cell::RefCell;
use std::time::Duration;

use gtk::glib;
use gtk::prelude::*;
use tafs_core::update::{self, NewVersion};
use tafs_core::words::WordBook;

use crate::widgets::{symbol, text};

thread_local! {
    /// The newer release found, once one is: it stays until a later check finds a newer one.
    static FOUND: RefCell<Option<NewVersion>> = const { RefCell::new(None) };
    /// The line in the sidebar now on screen; a sidebar drawn again makes its own.
    static LINE: RefCell<Option<gtk::Button>> = const { RefCell::new(None) };
}

/// The line, for the sidebar's foot: shown only once a newer release is found.
pub fn line(w: &WordBook) -> gtk::Button {
    let words = w.line("update.available").unwrap_or_default();
    let b = gtk::Button::new();
    b.add_css_class("tafs-plain");
    b.set_widget_name("new-version");
    let h = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let arrow = symbol("tafs-arrow-up-symbolic", 13);
    arrow.add_css_class("tafs-amber-text");
    arrow.set_valign(gtk::Align::Start);
    h.append(&arrow);
    let label = text(&words, 12, 600, &["tafs-amber-text"]);
    label.set_hexpand(true);
    h.append(&label);
    b.set_child(Some(&h));
    b.update_property(&[gtk::accessible::Property::Label(&words)]);
    b.connect_clicked(|b| {
        let Some(found) = FOUND.with(|f| f.borrow().clone()) else {
            return;
        };
        let window = b.root().and_downcast::<gtk::Window>();
        gtk::UriLauncher::new(&found.url).launch(
            window.as_ref(),
            gtk::gio::Cancellable::NONE,
            |_| {},
        );
    });
    let found = FOUND.with(|f| f.borrow().clone());
    show_on(&b, found.as_ref());
    LINE.with(|l| *l.borrow_mut() = Some(b.clone()));
    b
}

fn show_on(b: &gtk::Button, found: Option<&NewVersion>) {
    b.set_visible(found.is_some());
    b.set_tooltip_text(found.map(|f| f.url.as_str()));
}

/// A newer release found: the line shows it, now and in any sidebar drawn after.
pub fn found(new: NewVersion) {
    FOUND.with(|f| *f.borrow_mut() = Some(new.clone()));
    if let Some(b) = LINE.with(|l| l.borrow().clone()) {
        show_on(&b, Some(&new));
    }
}

/// The checks: one now, then one a day while the app stays open. A check that can't read
/// (offline, refused, a rate limit) changes nothing and says nothing.
pub fn start() {
    check();
    glib::timeout_add_seconds_local(update::EVERY_SECONDS, || {
        check();
        glib::ControlFlow::Continue
    });
}

fn check() {
    glib::spawn_future_local(async {
        let read = gtk::gio::spawn_blocking(|| update::check(read_latest)).await;
        if let Ok(Some(new)) = read {
            found(new);
        }
    });
}

/// GitHub's answer, read with no token and no cookie: only the app's User-Agent says who asks.
fn read_latest(url: &str) -> Result<serde_json::Value, ()> {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(15))
        .timeout(Duration::from_secs(30))
        .user_agent(update::USER_AGENT)
        .try_proxy_from_env(true)
        .build()
        .get(url)
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|_| ())?
        .into_json()
        .map_err(|_| ())
}
