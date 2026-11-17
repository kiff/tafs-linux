//! The iPad's look as GTK CSS: `Theme.swift`'s colours, light and dark, and the shared
//! components' shapes (`Components.swift`: `Panel`, `PrimaryLook`, `OutlineLook`, `Eyebrow`,
//! `TafAvatar`). The appearance follows the desktop's setting, or `--theme`.

use gtk::prelude::*;
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
.tafs-eyebrow.tafs-no { color: @tafs_no; }

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
/* OutlineButtonStyle(color: Theme.no): a halt's button, its words and edge in red. */
button.tafs-outline.tafs-no { color: @tafs_no; border-color: alpha(@tafs_no, 0.5); }
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

/* Coach (`SetupChatView`): its own wash; the owner's words on the brand sweep, corners of 18; cards'
   edges in their state's colour; an answered card's line on the raised colour at 60%. */
.tafs-coach-wash { background-color: @tafs_coach_wash; }
/* "Taking shape" beside the conversation: Coach mode's blue a shade deeper; the Card on the panel,
   corners of 14, its edge dashed until KIFF confirms it, then the ok colour. */
.tafs-coach-panel { background-color: @tafs_coach_panel; }
.tafs-shape-card { background-color: @tafs_panel; border: 1px dashed alpha(@tafs_ink, 0.25); border-radius: 14px; padding: 14px; }
.tafs-shape-card.confirmed { border: 1px solid @tafs_ok; }
.tafs-coach-text { color: @tafs_coach; }
.tafs-coach-title { border-bottom: 1px solid @tafs_line; }
.tafs-owner-bubble { background-image: linear-gradient(135deg, BRAND0, BRAND1, BRAND2); color: white; border-radius: 18px; padding: 10px 14px; }
.tafs-owner-bubble.steered { opacity: 0.8; }
.tafs-panel.edge-amber { border-color: @tafs_amber; }
.tafs-panel.edge-ok { border-color: @tafs_ok; }
.tafs-panel.edge-no { border-color: @tafs_no; }
.tafs-panel.edge-ink { border-color: @tafs_ink; }
.tafs-eyebrow.ok { color: @tafs_ok; }
.tafs-outcome { background-color: alpha(@tafs_raised, 0.6); border-radius: 12px; padding: 10px 12px; }
button.tafs-outcome-button { background: none; border: none; box-shadow: none; padding: 0; min-height: 0; }
button.tafs-outcome-button:hover .tafs-outcome { background-color: @tafs_raised; }
.tafs-notice { border-radius: 14px; padding: 12px; background-color: alpha(@tafs_soft, 0.10); }
.tafs-notice.no { background-color: alpha(@tafs_no, 0.10); }
.tafs-notice.amber { background-color: alpha(@tafs_amber, 0.10); }
.tafs-notice.ok { background-color: alpha(@tafs_ok, 0.10); }
.tafs-away { background-color: @tafs_raised; border-radius: 14px; padding: 12px; }
.tafs-card-words { background-color: @tafs_raised; border-radius: 12px; padding: 12px; }
button.tafs-capsule { background-color: @tafs_panel; background-image: none; color: @tafs_ink; border: 1px solid alpha(@tafs_ink, 0.18);
  border-radius: 999px; padding: 9px 14px; min-height: 0; box-shadow: none; font-size: 14px; font-weight: 400; }
button.tafs-choice { background-color: @tafs_panel; background-image: none; color: @tafs_ink; border: 1px solid alpha(@tafs_ink, 0.18);
  border-radius: 22px; padding: 10px 16px; min-height: 24px; box-shadow: none; }
button.tafs-capsule:disabled, button.tafs-choice:disabled { opacity: 0.45; }
button.tafs-coach-link { background: none; border: none; box-shadow: none; color: @tafs_coach; font-size: 15px; font-weight: 600; padding: 0; min-height: 0; }
button.tafs-coach-link:disabled { opacity: 0.45; }
button.tafs-work { background: none; border: none; box-shadow: none; padding: 0; min-height: 0; color: @tafs_dim; font-size: 13px; }
.tafs-step-list { border-left: 1px solid @tafs_line; padding-left: 6px; margin-left: 4px; }
.tafs-plan-mark { border-radius: 999px; min-width: 18px; min-height: 18px; }
.tafs-plan-mark.done { background-color: @tafs_ok; color: @tafs_on_fill; }
.tafs-plan-mark.failed { background-color: @tafs_no; color: @tafs_on_fill; }
.tafs-plan-mark.active { border: 2.5px solid @tafs_amber; min-width: 13px; min-height: 13px; }
.tafs-plan-mark.pending { border: 1.5px solid alpha(@tafs_ink, 0.25); min-width: 15px; min-height: 15px; }
.tafs-typing { background-color: @tafs_dim; border-radius: 999px; min-width: 7px; min-height: 7px; }
/* A choice picked: the coach colour, the words on it semibold (`QuestionsCard.choice`). */
button.tafs-choice.on { background-color: @tafs_coach; border-color: @tafs_coach; color: @tafs_on_fill; }
button.tafs-choice .detail { opacity: 0.8; }
button.tafs-coach-link.small { font-size: 13px; }
button.tafs-coach-link:hover { opacity: 0.8; }
entry.tafs-own-answer { background-color: @tafs_panel; border: 1px solid @tafs_coach; border-radius: 12px;
  padding: 6px 14px; min-height: 32px; font-size: 15px; color: @tafs_ink; box-shadow: none; outline: none; }
/* A message not sent, or held to go later: its bubble at 60%. */
.tafs-unsent .tafs-owner-bubble { opacity: 0.6; }
button.tafs-unsent-act { background: none; border: none; box-shadow: none; padding: 0; min-height: 0; font-size: 14px; color: @tafs_soft; }
button.tafs-unsent-act.strong { color: @tafs_ink; font-weight: 600; }
/* The composer: the bar on the background with a soft shadow above it (`barBackground`), the field on the panel with a 1-point edge, corners of 20;
   Send the coach colour, Stop the ink, each a 40-point circle. */
.tafs-composer-bar { background-color: @tafs_background; box-shadow: 0 -2px 6px alpha(@tafs_ink, 0.06); }
.tafs-composer { background-color: @tafs_panel; border: 1px solid @tafs_line; border-radius: 20px; }
textview.tafs-composer-text, textview.tafs-composer-text text { background: none; color: @tafs_ink; font-size: 16px; }
.tafs-composer scrolledwindow { background: none; }
button.tafs-send, button.tafs-stop { border-radius: 999px; min-width: 40px; min-height: 40px; padding: 0; border: none; box-shadow: none;
  background-image: none; color: @tafs_on_fill; }
button.tafs-send { background-color: @tafs_coach; }
button.tafs-send:disabled { background-color: alpha(@tafs_coach, 0.4); color: @tafs_on_fill; }
button.tafs-stop { background-color: @tafs_ink; }
/* A taf's chat: Send on the brand sweep (`Theme.brand`), white; a door's stamp in the blue, monospaced. */
button.tafs-send.brand { background-color: transparent; background-image: linear-gradient(135deg, BRAND0, BRAND1, BRAND2); color: white; }
button.tafs-send.brand:disabled { opacity: 0.4; color: white; }
.tafs-blue { color: @tafs_blue; }
.tafs-mono { font-family: monospace; }
/* Coach's bar: New conversation and its menu, plain icons in the ink. */
button.tafs-bar-icon, menubutton.tafs-bar-icon > button { background: none; border: none; box-shadow: none; color: @tafs_ink; min-width: 36px; min-height: 36px; padding: 0; }
button.tafs-bar-icon:hover, menubutton.tafs-bar-icon > button:hover { background-color: alpha(@tafs_ink, 0.06); }
/* Previous conversations: each chat on the panel, the one shown on the raised colour. */
.tafs-chat-line { background-color: @tafs_panel; border-radius: 12px; padding: 0 14px 0 0; }
.tafs-chat-line.shown { background-color: @tafs_raised; }
button.tafs-chat-row { background: none; border: none; box-shadow: none; padding: 10px 14px; border-radius: 12px; }
button.tafs-chat-row:hover { background-color: alpha(@tafs_ink, 0.04); }

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
button.tafs-chip-button:checked { background-color: @tafs_ink; color: @tafs_on_fill; border-color: @tafs_ink; }
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
/* A Record line's receipt: checked, its edge in ok at 40%; the timeline's ring and its line. */
/* Account › Alerts: an alert as it shows. */
.tafs-alert-example { background-color: @tafs_raised; border-radius: 18px; padding: 12px; }
/* A kept run: a link to its own page, its lines where the panel's are. */
button.tafs-run-row { padding: 0; margin: 0; min-height: 0; border-radius: 8px; }
button.tafs-run-row:hover { background-color: alpha(@tafs_ink, 0.04); }
.tafs-panel.edge-ok { border-color: alpha(@tafs_ok, 0.4); }
.tafs-step-ring { min-width: 10px; min-height: 10px; border-radius: 999px; border: 2px solid @tafs_dim; background-color: @tafs_panel; }
.tafs-step-rule { min-width: 2px; background-color: @tafs_line; }
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
/* Account: the chevron at the foot while there's more below, over a fade to the background. */
.tafs-more-below { background-image: linear-gradient(to bottom, alpha(@tafs_background, 0), @tafs_background); }
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

button.tafs-row-button { background: none; border: none; box-shadow: none; padding: 12px 0; font-size: 17px; font-weight: 400; }
button.tafs-row-button.tafs-amber-text { color: @tafs_amber_text; }
button.tafs-row-button.tafs-no { color: @tafs_no; }
button.tafs-link.tafs-no { color: @tafs_no; }
.tafs-strip-bar { background-color: @tafs_line; border-radius: 999px; }

.tafs-code { font-family: monospace; font-size: 34px; font-weight: 600; color: @tafs_ink; }
.tafs-mono { font-family: monospace; }
.tafs-notice { background-color: @tafs_panel; border-bottom: 1px solid @tafs_line; padding: 12px 24px; }
spinner { color: @tafs_amber; }
headerbar { background-color: @tafs_background; box-shadow: none; color: @tafs_ink; }
headerbar button { color: @tafs_amber_text; }
link, label link { color: @tafs_amber_text; }
/* A taf's knocks: the panel's edge in amber at 45%; a stranger's knock on amber at 7%, a
   paired sender's calmly; Do it in amber, Pair outlined, Ignore plain. */
.tafs-panel.knocks-edge { border-color: alpha(@tafs_amber, 0.45); }
.tafs-knock { padding: 12px; border-radius: 16px; background-color: alpha(@tafs_amber, 0.07); border: 1px solid alpha(@tafs_amber, 0.45); }
.tafs-knock.paired { background-color: @tafs_panel; border-color: alpha(@tafs_ink, 0.2); }
button.tafs-knock-do { background: @tafs_amber; color: white; font-size: 14px; font-weight: 600; border-radius: 999px; padding: 8px 14px; border: none; box-shadow: none; min-height: 0; }
button.tafs-knock-pair { background: none; color: @tafs_ink; font-size: 14px; font-weight: 500; border-radius: 999px; padding: 8px 12px; border: 1px solid alpha(@tafs_ink, 0.2); box-shadow: none; min-height: 0; }
button.tafs-knock-ignore { background: none; color: @tafs_soft; font-size: 14px; font-weight: 500; padding: 8px 12px; border: none; box-shadow: none; min-height: 0; }
.tafs-do-it-field { background-color: @tafs_panel; border: 1px solid @tafs_line; border-radius: 12px; padding: 6px; font-size: 15px; }
/* The sign-in's phone half: the QR on its white card, 20 round, with a soft shadow. */
.tafs-qr-card { background-color: white; border-radius: 20px; padding: 16px; box-shadow: 0 10px 18px alpha(black, 0.14); }
/* Your brief: an urgent decision's edge in red at 50%, its badge white on red; Coach's pick on its
   wash, corners of 12; why it can't wait on red at 8%, corners of 10; the next move's edge in amber
   at 35%; a case's 7-point dot in its state's colour. */
.tafs-panel.edge-move { border-color: alpha(@tafs_amber, 0.35); }
.tafs-urgent { background-color: @tafs_no; color: @tafs_on_fill; border-radius: 999px; padding: 4px 7px; }
.tafs-recommends { background-color: @tafs_coach_wash; border-radius: 12px; padding: 12px; }
.tafs-cant-wait { background-color: alpha(@tafs_no, 0.08); border-radius: 10px; padding: 10px; }
.tafs-case-dot { border-radius: 999px; min-width: 7px; min-height: 7px; }
.tafs-case-dot.ok { background-color: @tafs_ok; }
.tafs-case-dot.amber { background-color: @tafs_amber; }
.tafs-case-dot.dim { background-color: @tafs_dim; }
/* OutlineButtonStyle(color:) in the amber as text, and in Coach's colour. */
button.tafs-outline.amber { color: @tafs_amber_text; border-color: alpha(@tafs_amber_text, 0.5); }
button.tafs-outline.coach { color: @tafs_coach; border-color: alpha(@tafs_coach, 0.5); }
/* A label chip (`LabelChip`): its name and count on the panel with a 1-point edge, a capsule 32
   high; inverted when chosen; an amber dot while one of its tafs waits. */
button.tafs-label-chip { background-color: @tafs_panel; background-image: none; color: @tafs_ink; border: 1px solid @tafs_line;
  border-radius: 999px; padding: 0 12px; min-height: 32px; box-shadow: none; }
button.tafs-label-chip.selected { background-color: @tafs_ink; color: @tafs_on_fill; border-color: @tafs_ink; }
/* A mission's log (`LogRow`): on the panel with a 1-point edge, corners of 12, a 3-point edge in its
   kind's colour; the kind in that colour on it at 12%; evidence in Coach's colour on its wash. */
.tafs-log-row { background-color: @tafs_panel; border: 1px solid @tafs_line; border-radius: 12px; }
.tafs-log-edge { min-width: 3px; }
.tafs-log-edge.ok { background-color: @tafs_ok; }
.tafs-log-edge.blue { background-color: @tafs_blue; }
.tafs-log-edge.amber { background-color: @tafs_amber; }
.tafs-log-edge.ink { background-color: @tafs_ink; }
.tafs-log-edge.dim { background-color: @tafs_dim; }
.tafs-log-edge.line { background-color: @tafs_line; }
.tafs-log-kind { border-radius: 999px; padding: 2px 6px; }
.tafs-log-kind.ok { color: @tafs_ok; background-color: alpha(@tafs_ok, 0.12); }
.tafs-log-kind.blue { color: @tafs_blue; background-color: alpha(@tafs_blue, 0.12); }
.tafs-log-kind.amber { color: @tafs_amber; background-color: alpha(@tafs_amber, 0.12); }
.tafs-log-kind.ink { color: @tafs_ink; background-color: alpha(@tafs_ink, 0.12); }
.tafs-log-kind.dim { color: @tafs_dim; background-color: alpha(@tafs_dim, 0.12); }
.tafs-evidence { color: @tafs_coach; background-color: @tafs_coach_wash; border-radius: 999px; padding: 2px 6px; }
/* A taf's labels on its line (`TafLabels`): 11 medium, soft, on the raised colour. */
.tafs-taf-label { color: @tafs_soft; background-color: @tafs_raised; border-radius: 999px; padding: 2px 7px; }
.tafs-chip-dot { background-color: @tafs_amber; border-radius: 999px; min-width: 6px; min-height: 6px; }
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

thread_local! {
    static TINTED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// A name for a widget a tint's rule selects: the app's own, never an id tafs cloud sent (one with
/// a space or a brace would break the rule, or add another to the app's style sheet).
pub fn tint_name(prefix: &'static str) -> String {
    let n = TINTED.with(|c| {
        c.set(c.get() + 1);
        c.get()
    });
    format!("tint-{prefix}-{n}")
}

/// Whether a widget's name may go in a selector as it is: letters, digits, `-` and `_` only.
fn plain_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// A rule for one named widget, at the app's priority: the colour tafs cloud sends for a thing
/// (a recipe's category) has no class of its own. `color` is `#RRGGBB`, read and checked, and the
/// name is `tint_name`'s; any other name gets no rule.
fn name_rule(widget: &impl IsA<gtk::Widget>, rule: &str) {
    if !plain_name(&widget.widget_name()) {
        return;
    }
    let provider = gtk::CssProvider::new();
    provider.load_from_string(rule);
    gtk::style_context_add_provider_for_display(
        &widget.display(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );
}

/// A category's chip in its colour (`RecipeHome.chip`): its words and edge in it, and its fill
/// once picked.
pub fn tint_chip(chip: &gtk::ToggleButton, color: &str) {
    let name = chip.widget_name();
    name_rule(
        chip,
        &format!(
            "button#{name} {{ color: {color}; border-color: {color}; }} \
             button#{name}:checked {{ background-color: {color}; color: @tafs_on_fill; border-color: {color}; }}"
        ),
    );
}

/// A recipe's card edge in its category's colour (`RecipeRow`'s 4-wide leading bar).
pub fn tint_edge(card: &gtk::Box, color: &str) {
    let name = card.widget_name();
    name_rule(
        card,
        &format!("#{name} {{ box-shadow: inset 4px 0 0 {color}; }}"),
    );
}

/// A strip job's 4-wide bar in its category's colour.
pub fn tint_bar(bar: &gtk::Box, color: &str) {
    let name = bar.widget_name();
    name_rule(bar, &format!("#{name} {{ background-color: {color}; }}"));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tints_rule_names_only_the_apps_own_widgets() {
        let a = tint_name("chip");
        let b = tint_name("chip");
        assert_ne!(a, b);
        assert!(plain_name(&a));
        // An id as tafs cloud might send one never makes a selector.
        for id in ["money } * { color: red", "a b", "x{", "", "é"] {
            assert!(!plain_name(id), "{id}");
        }
    }
}
