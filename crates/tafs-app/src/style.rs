//! The iPad's look as GTK CSS: `Theme.swift`'s colours, light and dark, and the shared
//! components' shapes (`Components.swift`: `Panel`, `PrimaryLook`, `OutlineLook`, `Eyebrow`,
//! `TafAvatar`). The appearance follows the desktop's setting, or `--theme`.

use tafs_core::theme::{taf_color, Tone, BRAND, PALETTE, TAF_COLORS};

/// The colours as `@define-color`, for one appearance.
fn colours(dark: bool) -> String {
    let mut out = String::new();
    for Tone {
        name,
        light,
        dark: d,
    } in PALETTE
    {
        let c = if dark { d } else { light };
        out.push_str(&format!("@define-color tafs_{name} {};\n", c.css()));
    }
    // The selected sidebar row: the system list's grey fill (iOS `systemFill` light, `secondarySystemFill` dark).
    out.push_str(if dark {
        "@define-color tafs_selected rgba(120, 120, 128, 0.32);\n"
    } else {
        "@define-color tafs_selected rgba(120, 120, 128, 0.16);\n"
    });
    out
}

const SHAPES: &str = r#"
window, .tafs-ground { background-color: @tafs_background; color: @tafs_ink; }
.tafs-raised { background-color: @tafs_raised; }
.tafs-ink { color: @tafs_ink; }
.tafs-soft { color: @tafs_soft; }
.tafs-dim { color: @tafs_dim; }
.tafs-no { color: @tafs_no; }
.tafs-amber-text { color: @tafs_amber_text; }

/* Eyebrow: capitals, monospaced 12, tracked 0.7. */
.tafs-eyebrow { font-family: monospace; font-size: 12px; letter-spacing: 0.7px; color: @tafs_dim; }
.tafs-eyebrow.amber { color: @tafs_amber; }

/* Panel: the panel colour, a 1-point edge, corners of 18, padding 16. */
.tafs-panel { background-color: @tafs_panel; border: 1px solid @tafs_line; border-radius: 18px; padding: 16px; }
.tafs-panel.amber-edge { border-color: alpha(@tafs_amber, 0.4); }

/* PrimaryLook: the brand sweep, white 16 semibold, 54 high, corners of 16. */
button.tafs-primary {
  background-image: linear-gradient(135deg, BRAND0, BRAND1, BRAND2);
  color: @tafs_amber_ink; font-size: 16px; font-weight: 600;
  min-height: 54px; border-radius: 16px; padding: 0 18px; border: none; box-shadow: none;
}
button.tafs-primary:hover { background-image: linear-gradient(135deg, BRAND0, BRAND1, BRAND2); }
button.tafs-primary:active { opacity: 0.8; }
button.tafs-primary:disabled { background-image: none; background-color: alpha(@tafs_dim, 0.22); color: @tafs_soft; }

/* OutlineLook: ink 15 medium, 48 high, corners of 14, its edge the ink at 40%. */
button.tafs-outline {
  background: none; color: @tafs_ink; font-size: 15px; font-weight: 500;
  min-height: 48px; border-radius: 14px; border: 1px solid alpha(@tafs_ink, 0.4); box-shadow: none;
}
button.tafs-outline:hover { background-color: alpha(@tafs_ink, 0.04); }
button.tafs-outline:active { background-color: alpha(@tafs_ink, 0.08); }
button.tafs-outline:disabled { opacity: 0.4; }

button.tafs-link { background: none; box-shadow: none; border: none; color: @tafs_amber_text; font-size: 13px; font-weight: 600; padding: 2px 4px; min-height: 0; }

/* The sidebar: the raised colour, rows 15 semibold in the ink, the chosen row a rounded grey fill. */
.tafs-sidebar { background-color: @tafs_raised; }
.tafs-sidebar list { background: none; }
.tafs-sidebar row { border-radius: 10px; padding: 0 12px; margin: 0 12px; background: none; color: @tafs_ink; outline: none; }
.tafs-sidebar row:hover { background-color: alpha(@tafs_ink, 0.04); }
.tafs-sidebar row:selected, .tafs-sidebar row:selected:hover { background-color: @tafs_selected; color: @tafs_ink; }
.tafs-sidebar row.header, .tafs-sidebar row.header:hover,
.tafs-sidebar row.brand-row, .tafs-sidebar row.brand-row:hover, .tafs-sidebar row.brand-row:selected { background: none; }
.tafs-sidebar .row-title { font-size: 15px; font-weight: 600; color: @tafs_ink; }
.tafs-sidebar .row-badge { font-size: 15px; color: @tafs_dim; }
.tafs-sidebar .held-title { font-size: 14px; font-weight: 600; color: @tafs_ink; }
.tafs-sidebar .held-line { font-size: 12px; color: @tafs_soft; }
.tafs-sidebar .taf-now { font-size: 13px; color: @tafs_dim; }
.tafs-sidebar .taf-now.amber { color: @tafs_amber_text; }
.tafs-sidebar .taf-now.no { color: @tafs_no; }
.tafs-sidebar .brand { font-size: 22px; font-weight: 700; color: @tafs_ink; }
.tafs-dot { background-color: @tafs_amber; border-radius: 4px; min-width: 8px; min-height: 8px; }

/* Coach: the ink button, the dark ground's colour on it, 44 high, corners of 14. */
button.tafs-coach { background-color: @tafs_ink; background-image: none; color: @tafs_on_fill; font-size: 15px; font-weight: 600;
  min-height: 44px; border-radius: 14px; border: none; box-shadow: none; }
button.tafs-coach:hover { background-color: alpha(@tafs_ink, 0.92); }
button.tafs-coach.driving { background-color: @tafs_amber; }
button.tafs-account { background: none; box-shadow: none; border: none; padding: 0; min-height: 44px; }
button.tafs-account:hover { background-color: alpha(@tafs_ink, 0.04); }
.tafs-account .account-title { font-size: 14px; font-weight: 600; color: @tafs_ink; }
.tafs-account .account-label { font-size: 12px; color: @tafs_dim; }
.tafs-account image { color: @tafs_soft; }
.tafs-account image.open { color: @tafs_amber_text; }

/* TafAvatar: the initial in white on the taf's colour, corners of 30% of its size. */
.tafs-avatar { color: white; font-weight: 700; }
AVATARS

/* The welcome's points: a 9-point dot in its colour. */
.tafs-point { border-radius: 5px; min-width: 9px; min-height: 9px; }
.tafs-point.ok { background-color: @tafs_ok; }
.tafs-point.amber { background-color: @tafs_amber; }
.tafs-point.no { background-color: @tafs_no; }

/* The Overview: a held call on the raised colour, corners of 12; a line between the grid's rows. */
button.tafs-held-row { background-color: @tafs_raised; background-image: none; border-radius: 12px; padding: 12px; border: none; box-shadow: none; min-height: 0; }
button.tafs-held-row:hover { background-color: shade(@tafs_raised, 0.97); }
separator.tafs-line { background-color: @tafs_line; min-height: 1px; }

/* A held call: the reason as an amber chip, the countdown monospaced 12, the taf's words on a wash. */
.tafs-chip { font-size: 12px; font-weight: 500; color: @tafs_amber_text; background-color: alpha(@tafs_amber, 0.14); border-radius: 999px; padding: 5px 10px; }
.tafs-countdown { font-size: 12px; }
.tafs-taf-says { background-color: alpha(@tafs_soft, 0.08); border-radius: 10px; padding: 10px; }
.tafs-taf-says .taf-about { font-size: 15px; font-weight: 600; }
.tafs-taf-says .taf-reason { font-size: 14px; }
.tafs-details title, .tafs-details > title > label { color: @tafs_blue; font-size: 13px; font-weight: 600; }
button.tafs-outline.refuse { color: @tafs_no; border-color: alpha(@tafs_no, 0.4); }

/* Account: the iPad's grouped list, a panel with corners of 26, rows 17 in the ink, a line between. */
list.tafs-group { background-color: @tafs_panel; border-radius: 26px; padding: 0 20px; }
list.tafs-group > row { padding: 14px 0; min-height: 24px; background: none; border-bottom: 1px solid @tafs_line; }
list.tafs-group > row:last-child { border-bottom: none; }
list.tafs-group > row:hover { background: none; }
button.tafs-row-button { background: none; box-shadow: none; border: none; padding: 0; min-height: 0; font-size: 17px; font-weight: 400; }
button.tafs-row-button > label { margin: 0; }
button.tafs-row-button.amber { color: @tafs_amber; }
button.tafs-row-button.amber-text { color: @tafs_amber_text; }
button.tafs-row-button.no { color: @tafs_no; }
button.tafs-row-button:disabled { opacity: 0.4; }
/* The system's segmented control: a capsule track in the system grey, the chosen segment a capsule on it. */
.tafs-segmented { background-color: alpha(#767680, 0.12); border-radius: 17px; padding: 2px; }
.tafs-segmented button.segment { background: none; background-image: none; box-shadow: none; border: none; border-radius: 15px; min-height: 28px; font-size: 13px; font-weight: 500; color: @tafs_ink; }
.tafs-segmented button.segment:checked { background-color: @tafs_panel; box-shadow: 0 1px 3px alpha(black, 0.12); }

/* The Record: chips (the chosen one inverts), day headings, rows on the panel with corners of 14. */
button.tafs-chip-button { background-color: @tafs_panel; background-image: none; color: @tafs_soft; border: 1px solid @tafs_line; border-radius: 999px; padding: 5px 11px; min-height: 0; font-size: 13px; font-weight: 400; box-shadow: none; }
button.tafs-chip-button.selected { background-color: @tafs_ink; color: @tafs_on_fill; border-color: @tafs_ink; }
.tafs-taf-dot { border-radius: 4px; min-width: 8px; min-height: 8px; }
.tafs-day { font-family: monospace; font-size: 11px; font-weight: 500; color: @tafs_dim; }
.tafs-record-row { background-color: @tafs_panel; border: 1px solid @tafs_line; border-radius: 14px; padding: 14px; }
.tafs-kind-dot { border-radius: 5px; min-width: 9px; min-height: 9px; }
.tafs-kind-dot.ok { background-color: @tafs_ok; }
.tafs-kind-dot.amber { background-color: @tafs_amber; }
.tafs-kind-dot.no { background-color: @tafs_no; }
.tafs-kind-dot.dim { background-color: @tafs_dim; }
.tafs-time { font-size: 12px; }
button.tafs-reads { background: none; box-shadow: none; border: none; color: @tafs_ok; font-size: 13px; font-weight: 400; padding: 0 4px; min-height: 0; }

/* A taf's page: the state chip (its colour on its colour at 15%), the Card bar, the small Card. */
.tafs-state-chip { font-size: 12px; font-weight: 500; border-radius: 999px; padding: 4px 10px; }
.tafs-state-chip.ok { color: @tafs_ok; background-color: alpha(@tafs_ok, 0.15); }
.tafs-state-chip.amber { color: @tafs_amber; background-color: alpha(@tafs_amber, 0.15); }
.tafs-state-chip.no { color: @tafs_no; background-color: alpha(@tafs_no, 0.15); }
.tafs-state-chip.soft { color: @tafs_soft; background-color: alpha(@tafs_soft, 0.15); }
.tafs-card-bar { background-color: @tafs_panel; border: 1px solid @tafs_line; border-radius: 14px; padding: 9px 14px; }
.tafs-card-mini { border-radius: 5px; color: alpha(white, 0.9); }
.tafs-card-mini.dimfill { background-color: @tafs_dim; }
.tafs-card-mini.faded { opacity: 0.4; }
.tafs-segmented button.segment:disabled { opacity: 0.4; }

/* tafs cloud's Activity filters: the system's bordered buttons, the chosen one in the amber as text. */
button.tafs-filter-bordered { background-color: alpha(@tafs_soft, 0.12); background-image: none; color: @tafs_soft; border: none; border-radius: 8px; padding: 5px 12px; min-height: 0; font-size: 15px; font-weight: 400; box-shadow: none; }
button.tafs-filter-bordered.selected { background-color: alpha(@tafs_amber_text, 0.15); color: @tafs_amber_text; }

/* A Card's face: white on its colour, corners of 18; each Card behind a shade darker. */
.tafs-face { color: white; border-radius: 18px; padding: 16px; }
.tafs-face.dimfill { background-color: @tafs_dim; }
.tafs-face.faded { opacity: 0.4; }
.tafs-face-words { opacity: 0.9; }
.tafs-face-caption { opacity: 0.75; }
.tafs-face-shade { background-color: black; border-radius: 18px; }
.tafs-face-clip { border-radius: 18px; box-shadow: 0 -2px 4px alpha(black, 0.16); }
.tafs-env-pill { font-family: monospace; font-size: 10px; font-weight: 600; letter-spacing: 0.6px; background-color: alpha(white, 0.22); border-radius: 999px; padding: 2px 6px; color: white; }

/* Plan & usage: a panel's edge in its tone at 50%, the meter's fill, a panel that opens. */
.tafs-panel.edge-no { border-color: alpha(@tafs_no, 0.5); }
.tafs-panel.edge-amber { border-color: alpha(@tafs_amber, 0.5); }
.tafs-meter.ok { color: @tafs_ok; }
.tafs-meter.amber { color: @tafs_amber_text; }
.tafs-meter.no { color: @tafs_no; }
button.tafs-panel-link { background-color: @tafs_panel; background-image: none; border: 1px solid @tafs_line; border-radius: 18px; padding: 16px; box-shadow: none; }
button.tafs-back { background: none; border: none; box-shadow: none; color: @tafs_amber_text; padding: 4px 8px; }

/* Connect apps: the inset list's group, its buttons as rows, the search, a Connect as wide as its words. */
.tafs-group { background-color: @tafs_panel; border-radius: 18px; padding: 0 16px; }
button.tafs-list-button { background-color: @tafs_panel; background-image: none; border: none; border-radius: 18px; padding: 16px; box-shadow: none; color: @tafs_amber_text; font-size: 17px; font-weight: 400; }
button.tafs-list-button label { font-size: 17px; }
button.tafs-list-button > label { margin: 0; }
entry.tafs-search, .tafs-search { border-radius: 999px; min-height: 36px; }
button.tafs-outline.compact { min-height: 30px; padding: 8px 16px; }
button.tafs-list-button label { margin: 0; }
.tafs-ok { color: @tafs_ok; }

/* Devices: the round back button, a text button in red. */
button.tafs-round-back { background-color: @tafs_panel; border-radius: 999px; min-width: 44px; min-height: 44px; padding: 0; box-shadow: none; border: none; color: @tafs_ink; }
button.tafs-text-button { background: none; border: none; box-shadow: none; padding: 4px 0; font-size: 17px; }
button.tafs-text-button.refuse { color: @tafs_no; }

/* The calendar. */
button.tafs-chevron { background: none; border: none; box-shadow: none; color: @tafs_amber_text; padding: 4px; min-width: 0; min-height: 0; }
button.tafs-today-pill { background: none; box-shadow: none; border: 1px solid alpha(@tafs_ink, 0.4); border-radius: 999px; min-height: 40px; padding: 0 16px; color: @tafs_ink; font-size: 15px; font-weight: 500; }
button.tafs-today-pill.at-today { color: @tafs_soft; border-color: alpha(@tafs_ink, 0.2); }
menubutton.tafs-date-pill > button { background-color: alpha(@tafs_ink, 0.06); background-image: none; border: none; box-shadow: none; border-radius: 999px; padding: 6px 14px; color: @tafs_ink; font-size: 17px; }
.tafs-navigator { background-color: @tafs_panel; border-radius: 14px; padding: 14px; }
button.tafs-day-cell { background: none; box-shadow: none; border: 1.5px solid transparent; border-radius: 8px; padding: 2px; min-height: 0; min-width: 0; }
button.tafs-day-cell.selected { background-color: alpha(@tafs_amber, 0.18); }
button.tafs-day-cell.today { border-color: @tafs_amber; }
.tafs-out-of-month { color: alpha(@tafs_dim, 0.6); }
.tafs-mark { min-width: 5px; min-height: 5px; border-radius: 999px; }
.tafs-mark.on-amber { background-color: @tafs_amber_ink; }
button.tafs-strip-cell { background-color: @tafs_panel; background-image: none; box-shadow: none; border: 1.5px solid transparent; border-radius: 12px; padding: 0; min-width: 0; }
button.tafs-strip-cell.selected { background-color: @tafs_amber; }
button.tafs-strip-cell.today { border-color: @tafs_amber; }
.tafs-amber-ink { color: @tafs_amber_ink; }
button.tafs-plain { background: none; border: none; box-shadow: none; padding: 0; min-height: 0; min-width: 0; }
.tafs-hour { border-top: 1px solid @tafs_line; }
.tafs-slot { border-radius: 6px; padding: 3px 6px; }
.tafs-dot7 { min-width: 7px; min-height: 7px; border-radius: 999px; }
.tafs-dot9 { min-width: 9px; min-height: 9px; border-radius: 999px; }
.tafs-event { background-color: @tafs_panel; border-radius: 10px; padding: 10px; }

.tafs-person-badge { background-color: @tafs_raised; border-radius: 999px; color: @tafs_ink; font-size: 14px; font-weight: 600; }

.tafs-code { font-family: monospace; font-size: 34px; font-weight: 600; color: @tafs_ink; }
.tafs-mono { font-family: monospace; }
.tafs-notice { background-color: @tafs_panel; border-bottom: 1px solid @tafs_line; padding: 12px 24px; }
spinner { color: @tafs_amber; }
headerbar { background-color: @tafs_background; box-shadow: none; color: @tafs_ink; }
headerbar button { color: @tafs_amber_text; }
link, label link { color: @tafs_amber_text; }
"#;

/// The style sheet for one appearance.
pub fn css(dark: bool) -> String {
    let mut avatars = String::new();
    for (i, hex) in TAF_COLORS.iter().enumerate() {
        avatars.push_str(&format!(
            ".tafs-avatar.c{i}, .tafs-taf-dot.c{i}, .tafs-card-mini.c{i}, .tafs-face.c{i}, .tafs-mark.c{i}, .tafs-dot7.c{i}, .tafs-dot9.c{i} {{ background-color: #{hex:06x}; }}\n.tafs-slot.c{i} {{ background-color: alpha(#{hex:06x}, 0.16); }}\n"
        ));
    }
    let shapes = SHAPES
        .replace("BRAND0", &format!("#{:06x}", BRAND[0]))
        .replace("BRAND1", &format!("#{:06x}", BRAND[1]))
        .replace("BRAND2", &format!("#{:06x}", BRAND[2]))
        .replace("AVATARS", &avatars);
    let dark_controls = if dark {
        // In dark, the track at 24% and the chosen segment the system's #636366, so it stands out.
        ".tafs-segmented { background-color: alpha(#767680, 0.24); }\n.tafs-segmented button.segment:checked { background-color: #636366; }\n"
    } else {
        ""
    };
    colours(dark) + &shapes + dark_controls
}

/// The avatar's colour class for a taf's colour index (`Theme.tafColor`).
pub fn avatar_class(index: i64) -> String {
    let hex = taf_color(index);
    let i = TAF_COLORS.iter().position(|c| *c == hex).unwrap_or(0);
    format!("c{i}")
}

/// Loads the style sheet, and loads it again when the appearance changes.
pub fn install(display: &gtk::gdk::Display) {
    let provider = gtk::CssProvider::new();
    let manager = adw::StyleManager::default();
    provider.load_from_string(&css(manager.is_dark()));
    gtk::style_context_add_provider_for_display(
        display,
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    manager.connect_dark_notify(move |m| provider.load_from_string(&css(m.is_dark())));
    gtk::IconTheme::for_display(display).add_resource_path("/dev/kiff/tafs/icons");
}
