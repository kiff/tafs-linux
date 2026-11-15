# The sidebar

The iPad's one sidebar for the whole app (`HomeSplitView` with `tab` set, a full-width iPad).
What it says is `tafs-core/src/sidebar.rs`, tested there; the app places it
(`crates/tafs-app/src/shell.rs`).

From the top:

- **The mascot (34) and "tafs" (22 bold):** the way to the Overview, and what opens first. No
  fill when the Overview is open (the iPad's `listRowBackground(.clear)`).
- **The app's screens**, in the iPad's order: Needs you, Record, then, for a signed-in account,
  Calendar, Plan & usage and Connect apps (`signedInLayout`). The sample without the account shows
  only Needs you and Record, as the iPad does.
  - Each row: its symbol (20, ink) and its title (15 semibold, ink), 52 apart.
  - **Needs you's count** at its row's end is a plain grey number, none at zero (`.badge`):
    tafs cloud's count when it sends one, else the held calls (`needsYouCount`). A screen reader
    hears "Needs you, 2".
- **NEEDS YOU**, when calls are held: the eyebrow in the amber, then each call: an 8-point amber
  dot, "Deploys: Run migration 0042_orders_index" (14 semibold) and "Schema migrations: always
  asks you · answer by 03:02" (12, soft), each on one line, cut short. Times are 24-hour, this
  computer's until the account's zone is read (`shortTime`). A call whose taf isn't listed isn't
  drawn. A row opens its taf.
- **YOUR TAFS** (the eyebrow: monospaced 12, tracked, in capitals by the language's rules): the
  tafs relevant now (`relevantNow`): a call of its waits, a step or a knock is open, or it ran or
  changed in the last 24 hours. Each row: the avatar (34, its colour, corners of 30%), the name (15
  semibold) and what it's doing (13, dim), one line each. The section shows with tafs relevant
  now, or with no tafs at all; then, once Home is read, tafs cloud's line, else "Connect an app to
  set up your first taf." (or where shared tafs show, for someone who can't use Coach).
- **At the foot, always in sight:** Coach, the ink button (light in dark, with the dark ground's
  colour on it), 44 high, corners of 14, with the sparkle, for a signed-in account that may use
  Coach; and Account under it, a person in a circle (soft) with the account's name once KIFF
  says it. While Account is open its circle is filled, in the amber.

The pane is the iPad's raised colour, 320 wide. The chosen row is a rounded grey fill (the system
list's), never a side marker.

## The sample

`--sample` (and Try it with sample data) draws the iPad's five sample tafs and the two calls their
Cards held (`TafTemplates`, `AppModel.propose`): Deploys' migration, which its Card always asks
about, and Bill pay's €8,400 bill, over €5,000 each. A held call lapses 10 minutes after it was
held. `--signed-in` draws it with the account's items, as the reference set's `overview.png`.

## The view-only notice

When KIFF says the session is view-only, the screen beside the sidebar opens with "View only":
"This computer shows your tafs, Needs you and the Record. It can't approve until you add it with
your phone." and Add this computer (dimmed: pairing comes in its own change).

## Not yet

- **What each row opens:** each screen shows its title until it's ported (the Overview next).
- **Home from tafs cloud** (`GET /v1/home`): until it's read, a signed-in account lists no tafs and
  no line under Your tafs.
- **Shared with you, Other agents, tafs being set up** (and Discard), the recipes strip and
  Explore, **+ New** and its gallery, the halt-all row: with Home's read and their screens.
- **The Unreachable and Plan lines** at the foot.
- **Coach mode:** its button opens its title for now.
