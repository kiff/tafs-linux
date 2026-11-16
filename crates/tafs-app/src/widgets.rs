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
    if amber {
        l.add_css_class("amber");
    }
    l
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
        "app.connected.to.app.below.fill" => "tafs-apps-connected-symbolic",
        "person.crop.circle" => "tafs-person-circle-symbolic",
        "person.crop.circle.fill" => "tafs-person-circle-fill-symbolic",
        "sparkle" => "tafs-sparkle-symbolic",
        "sparkles" => "tafs-sparkles-symbolic",
        "doc.on.doc" => "tafs-copy-symbolic",
        "checkmark" => "tafs-checkmark-symbolic",
        "creditcard" => "tafs-creditcard-symbolic",
        "person.2" => "system-users-symbolic",
        "link" => "insert-link-symbolic",
        "doc.text" => "x-office-document-symbolic",
        // A halt's hand: the desktop's stop sign, in red where it's drawn.
        "hand.raised" | "hand.raised.fill" => "process-stop-symbolic",
        _ => "image-missing-symbolic",
    }
}

/// A screen's ground (`screenBackground`).
pub fn ground<W: IsA<gtk::Widget>>(w: &W) {
    w.add_css_class("tafs-ground");
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
