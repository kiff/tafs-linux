# A taf's page

The iPad's taf page (`TafDetailView`), beside the sidebar: a taf's row in Your tafs, a line of
the Overview, or a held call opens it. What it says is `tafs-core/src/taf_page.rs`, tested there;
the page is `crates/tafs-app/src/taf.rs`.

## The page

- **The head** (`header`): the avatar (34), the name (20 bold, one line), and its state as a chip
  (`StateChip`): Working in green, Waiting in amber, Stopped and Halted in red, Ready and Paused
  in the soft colour, each on its colour at 15%. When tafs cloud flags what went wrong, the chip
  isn't shown in place of Working.
- **The tabs:** Chat, Activity, Card and Setup, as the system's segmented control.
- **The Card bar** (`CardBar`): a small Card in the taf's colour (the dim colour when it isn't
  live, faded when revoked or not active) with the Card's mark, the Card's figure and words ("€1,960
  left today of €2,000", else its title), and under it its name, its environment when it isn't
  production, and how many Cards ("Refunds · Test · 3 Cards"); a chevron at its end.
- **Its held calls**, under NEEDS YOU in the amber, as Needs you's cards.
- **Activity** (the sample's, `activity(_:)`): All, Done, Held, Refused (Done is the iPad's
  `taf.halt.part_held`), then its lines by day, newest first, each day's heading with what was
  spent, each line as the Record's without its taf. "Nothing yet." with All, else "Nothing here."

`--page taf` opens the sample's Refunds, as `taf.png`.

## Not yet

- **The page opens on Activity**, the tab ported so far; the iPad opens on Chat. Chat, Card and
  Setup are there, dimmed, until each is ported, and the Card bar opens nothing yet.
- **A tafs cloud taf's Activity** is its own paged read (`TafActivityList`: `WorkPanel`,
  `RunsPanel`, the sources' pages, `taf-activity.json`): until it's ported, it shows its held calls.
- **The line under the head:** its version and who it's shared with (`headerLine`, `ShareLine`).
- **The side column** on a wide window, Revoke, Remove, and folding the head while scrolling.
