//! The iPad's shared pieces (`Components.swift`, `Theme.swift`) as GTK widgets.

use gtk::prelude::*;

/// A label in a size and weight, with style classes (`scaledFont`).
pub fn text(s: &str, size: u32, weight: u16, classes: &[&str]) -> gtk::Label {
    let l = gtk::Label::new(Some(s));
    l.set_xalign(0.0);
    l.set_wrap(true);
    l.set_wrap_mode(gtk::pango::WrapMode::WordChar);
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrSize::new_size_absolute(
        size as i32 * gtk::pango::SCALE,
    ));
    attrs.insert(gtk::pango::AttrInt::new_weight(weight_of(weight)));
    l.set_attributes(Some(&attrs));
    for c in classes {
        l.add_css_class(c);
    }
    l
}

/// One line, cut short with an ellipsis (`oneLineUnlessLarge`, `lineLimit(1)`).
pub fn one_line(s: &str, size: u32, weight: u16, classes: &[&str]) -> gtk::Label {
    let l = text(s, size, weight, classes);
    l.set_wrap(false);
    l.set_ellipsize(gtk::pango::EllipsizeMode::End);
    l.set_single_line_mode(true);
    l
}

fn weight_of(w: u16) -> gtk::pango::Weight {
    match w {
        700.. => gtk::pango::Weight::Bold,
        600.. => gtk::pango::Weight::Semibold,
        500.. => gtk::pango::Weight::Medium,
        _ => gtk::pango::Weight::Normal,
    }
}

/// `Eyebrow`: in capitals, monospaced 12, tracked, in the dim colour or the amber.
pub fn eyebrow(s: &str, amber: bool) -> gtk::Label {
    let l = gtk::Label::new(Some(&tafs_core::words::upper(s)));
    l.set_xalign(0.0);
    l.add_css_class("tafs-eyebrow");
    plain_if_ideographic(&l, s);
    if amber {
        l.add_css_class("amber");
    }
    l
}

/// A caption set monospaced and spaced out takes the plain face with no spacing when its text is in
/// Hangul, kana, Han or Arabic (`Eyebrow.ideographic`), which spacing pulls apart.
pub fn plain_if_ideographic(l: &gtk::Label, text: &str) {
    if tafs_core::words::ideographic(text) {
        l.add_css_class("ideographic");
    }
}

/// `PrimaryButtonStyle`.
pub fn primary(label: &str) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.add_css_class("tafs-primary");
    b.set_hexpand(true);
    b
}

/// `OutlineButtonStyle`.
pub fn outline(label: &str) -> gtk::Button {
    let b = gtk::Button::with_label(label);
    b.add_css_class("tafs-outline");
    b.set_hexpand(true);
    b
}

/// `Panel`, its edge in the line colour or the amber at 40%.
pub fn panel(amber: bool) -> gtk::Box {
    let b = gtk::Box::new(gtk::Orientation::Vertical, 12);
    b.add_css_class("tafs-panel");
    if amber {
        b.add_css_class("amber-edge");
    }
    b
}

/// `Mascot`.
pub fn mascot(size: i32) -> gtk::Image {
    let i = gtk::Image::from_resource("/dev/kiff/tafs/mascot.png");
    i.set_pixel_size(size);
    i.set_size_request(size, size);
    i.set_accessible_role(gtk::AccessibleRole::Presentation);
    i
}

/// `TafAvatar` with its `DoorRim`: while a door is open, an arc travels round the avatar's edge, 4
/// out from it, once every 4.5 s (still with the desktop's animations off), in the amber while a
/// knock waits, dim while every open door rests, else the taf's colour. No rim: the avatar alone.
pub fn taf_avatar(
    initial: &str,
    color_index: i64,
    size: i32,
    rim: Option<tafs_core::rim::Rim>,
) -> gtk::Widget {
    use tafs_core::rim::Tint;
    let a = avatar(initial, color_index, size);
    let Some(rim) = rim else {
        return a.upcast();
    };
    let o = gtk::Overlay::new();
    let area = gtk::DrawingArea::new();
    area.set_content_width(size + 8);
    area.set_content_height(size + 8);
    area.set_accessible_role(gtk::AccessibleRole::Presentation);
    o.set_child(Some(&area));
    o.add_overlay(&a);
    let phase = std::rc::Rc::new(std::cell::Cell::new(0.0_f64));
    {
        let phase = phase.clone();
        area.set_draw_func(move |_, cr, w, h| {
            let hex = match rim.tint {
                Tint::Knock => tone_hex("amber"),
                Tint::Resting => tone_hex("dim"),
                Tint::Active => tafs_core::theme::taf_color(color_index),
            };
            crate::illustrations::rgb(cr, hex, 1.0);
            cr.set_line_width(2.0);
            cr.set_line_cap(gtk::cairo::LineCap::Round);
            cr.set_line_join(gtk::cairo::LineJoin::Round);
            // The edge 1 in from the area's, so the 2-wide line stays inside it.
            let radius = size as f64 * 0.3 + 4.0 - 1.0;
            let points = tafs_core::rim::arc(
                w as f64 - 2.0,
                h as f64 - 2.0,
                radius,
                phase.get(),
                rim.length as f64 / 1000.0,
            );
            if let Some((x, y)) = points.first() {
                cr.move_to(x + 1.0, y + 1.0);
                for (x, y) in &points[1..] {
                    cr.line_to(x + 1.0, y + 1.0);
                }
                let _ = cr.stroke();
            }
        });
    }
    if gtk::Settings::default().is_some_and(|s| s.is_gtk_enable_animations()) {
        let start = std::cell::Cell::new(None::<i64>);
        area.add_tick_callback(move |area, clock| {
            let now = clock.frame_time();
            let from = start.get().unwrap_or(now);
            start.set(Some(from));
            phase.set(((now - from) as f64 / 1_000_000.0 / 4.5).rem_euclid(1.0));
            area.queue_draw();
            gtk::glib::ControlFlow::Continue
        });
    }
    o.set_valign(gtk::Align::Center);
    o.set_halign(gtk::Align::Center);
    o.upcast()
}

/// A palette colour as this appearance draws it.
fn tone_hex(name: &str) -> u32 {
    let t = tafs_core::theme::tone(name).expect("the palette has it");
    if adw::StyleManager::default().is_dark() {
        t.dark.0
    } else {
        t.light.0
    }
}

/// `TafAvatar`: the initial in white on the taf's colour, corners of 30%.
pub fn avatar(initial: &str, color_index: i64, size: i32) -> gtk::Label {
    let l = gtk::Label::new(Some(initial));
    l.add_css_class("tafs-avatar");
    l.add_css_class(&crate::style::avatar_class(color_index));
    l.set_size_request(size, size);
    l.set_valign(gtk::Align::Center);
    l.set_halign(gtk::Align::Center);
    let attrs = gtk::pango::AttrList::new();
    attrs.insert(gtk::pango::AttrSize::new_size_absolute(
        size * gtk::pango::SCALE / 2,
    ));
    attrs.insert(gtk::pango::AttrInt::new_weight(gtk::pango::Weight::Bold));
    l.set_attributes(Some(&attrs));
    let provider = gtk::CssProvider::new();
    provider.load_from_string(&format!(
        "label {{ border-radius: {}px; }}",
        (size as f32 * 0.3).round()
    ));
    #[allow(deprecated)]
    l.style_context()
        .add_provider(&provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
    l.set_accessible_role(gtk::AccessibleRole::Presentation);
    l
}

/// A symbol at a size (the iPad's SF Symbols, drawn from the app's own icons).
pub fn symbol(name: &str, size: i32) -> gtk::Image {
    let i = gtk::Image::from_icon_name(name);
    i.set_pixel_size(size);
    i
}

/// The icon the app draws for one of the iPad's SF Symbols.
pub fn icon_for(sf: &str) -> &'static str {
    match sf {
        "bell" => "tafs-bell-symbolic",
        "list.bullet" => "tafs-list-symbolic",
        "calendar" => "tafs-calendar-symbolic",
        "chart.bar" => "tafs-chart-bar-symbolic",
        "square.stack.3d.up" => "tafs-stack-symbolic",
        "app.connected.to.app.below.fill" => "tafs-apps-connected-symbolic",
        "person.crop.circle" => "tafs-person-circle-symbolic",
        "person.crop.circle.fill" => "tafs-person-circle-fill-symbolic",
        "sparkle" => "tafs-sparkle-symbolic",
        "sparkles" => "tafs-sparkles-symbolic",
        "clock" => "tafs-clock-symbolic",
        "paperplane" => "tafs-paperplane-symbolic",
        "bolt.fill" => "tafs-bolt-symbolic",
        "doc.on.doc" => "tafs-copy-symbolic",
        "checkmark" => "tafs-checkmark-symbolic",
        "creditcard" => "tafs-creditcard-symbolic",
        "person.2" => "system-users-symbolic",
        "link" => "insert-link-symbolic",
        "doc.text" => "x-office-document-symbolic",
        "speaker.wave.2" | "speaker.wave.2.fill" => "audio-volume-high-symbolic",
        "bell.slash" => "notifications-disabled-symbolic",
        // A halt's hand: the desktop's stop sign, in red where it's drawn.
        // The iPad's raised hand: open while every taf runs, filled once one is halted.
        "hand.raised" => "tafs-hand-raised-symbolic",
        "hand.raised.fill" => "tafs-hand-raised-fill-symbolic",
        _ => "image-missing-symbolic",
    }
}

/// A screen's ground (`screenBackground`).
pub fn ground<W: IsA<gtk::Widget>>(w: &W) {
    w.add_css_class("tafs-ground");
}

/// Pull to refresh (`.refreshable`): `refresh` runs when `scroller` is pulled past its top (a
/// touchpad's or a touch screen's pull, as GTK reports it), and on Ctrl+R or F5 while the focus is
/// in `host`, a computer's way to ask: only there (`Local`), so the sidebar and the screen beside
/// it never compete for the keys. Whether a read is already on its way is `refresh`'s to say.
pub fn refreshable<W: IsA<gtk::Widget>>(
    scroller: &gtk::ScrolledWindow,
    host: &W,
    refresh: std::rc::Rc<dyn Fn()>,
) {
    {
        let refresh = refresh.clone();
        scroller.connect_edge_overshot(move |_, edge| {
            if edge == gtk::PositionType::Top {
                refresh();
            }
        });
    }
    let keys = gtk::ShortcutController::new();
    keys.set_scope(gtk::ShortcutScope::Local);
    for trigger in ["<Control>r", "F5"] {
        let refresh = refresh.clone();
        keys.add_shortcut(gtk::Shortcut::new(
            gtk::ShortcutTrigger::parse_string(trigger),
            Some(gtk::CallbackAction::new(move |_, _| {
                refresh();
                gtk::glib::Propagation::Stop
            })),
        ));
    }
    host.add_controller(keys);
}

/// Writes a file the person saved from tafs (their chats, their receipts) readable by them alone
/// (0600), whatever the umask: other accounts on a shared computer can't read it. A file already
/// there by that name is replaced, and keeps the same mode.
pub fn write_private(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    // `mode` applies only when the file is made: one there already is set to 0600 too.
    f.set_permissions(std::fs::Permissions::from_mode(0o600))?;
    f.write_all(bytes)
}

#[cfg(test)]
mod tests {
    /// One layout on every app (kiff/tafs `docs/parity.md`): a page's column is set only through
    /// `page_column`, so no source but this one sets a page cap of its own, and each page uses it.
    #[test]
    fn every_page_column_comes_from_page_column() {
        use super::PageColumn;
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        // Not pages: Account's sheets, a taf's chat column and a file's preview keep their own.
        let not_pages = ["widgets.rs", "sheet.rs", "taf_chat.rs", "files.rs"];
        let mut own = Vec::new();
        let mut pages = Vec::new();
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let src = std::fs::read_to_string(&path).unwrap();
            if !not_pages.contains(&name.as_str()) {
                for cap in ["720", "980", "640", "1100", "1180"] {
                    if src.contains(&format!("set_maximum_size({cap})")) {
                        own.push(name.clone());
                    }
                }
            }
            if name != "widgets.rs" && src.contains("page_column(") {
                pages.push(name);
            }
        }
        assert_eq!(own, Vec::<String>::new());
        pages.sort();
        assert_eq!(
            pages,
            [
                "account.rs",
                "alerts.rs",
                "brief.rs",
                "calendar.rs",
                "chrome.rs",
                "connect.rs",
                "connections.rs",
                "devices.rs",
                "kiff_account.rs",
                "language_picker.rs",
                "needs_you.rs",
                "overview.rs",
                "people.rs",
                "recipes.rs",
                "usage.rs"
            ]
        );
        assert_eq!(PageColumn::Reading.width(), 720);
        assert_eq!(PageColumn::List.width(), 980);
    }

    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn a_saved_file_is_the_persons_alone() {
        let dir = std::env::temp_dir().join(format!("tafs-private-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("tafs-chats.txt");
        std::fs::write(&path, b"old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        super::write_private(&path, b"new").unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"new");
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

/// The most a page's column takes on a wide window, the same on every app (kiff/tafs
/// `docs/parity.md`, "One layout on every app"): the iPad's and the Mac's caps. A page sets its
/// width only through `page_column`, never a number of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageColumn {
    /// Account and its pages, Devices, People, Alerts, Chrome, Needs you, Connect apps, a
    /// connection, Plan & usage.
    Reading = 720,
    /// Overview and an app's connections.
    List = 980,
    /// The recipe gallery.
    Gallery = 640,
    /// The Brief.
    Brief = 1100,
    /// Calendar.
    Calendar = 1180,
}

impl PageColumn {
    /// Its cap, in pixels.
    pub const fn width(self) -> i32 {
        self as i32
    }
}

/// The page's column: at most `column`'s width, centred in the pane, its own rows aligned left.
/// A pane narrower than the cap is unchanged. Put it in the page's scroller.
pub fn page_column<W: IsA<gtk::Widget>>(child: &W, column: PageColumn) -> adw::Clamp {
    let clamp = adw::Clamp::new();
    clamp.set_maximum_size(column.width());
    clamp.set_tightening_threshold(column.width());
    clamp.set_valign(gtk::Align::Start);
    clamp.set_child(Some(child));
    clamp
}
