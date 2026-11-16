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

## The Overview

What the mascot opens, and what opens first (`OverviewView` in `ShareViews.swift`, with
`showsCalendar` off: Calendar is a sidebar item). The logic is `tafs-core/src/overview.rs`; the
page is `crates/tafs-app/src/overview.rs`.

- **The title** at 30 bold and "Today, across your tafs" at 14, dim.
- **NEEDS YOU**, whenever anything waits (`needsYouCount > 0`): the amber eyebrow, then each held
  call on the raised colour with corners of 12: an amber dot, "Deploys · Run migration
  0042_orders_index" (14 semibold, one line) and "answer by 03:02" (12, soft) at its end. A call
  opens its taf. When Needs you counts more than the calls listed (`showsRest`), one more row says
  so in tafs cloud's words (or "3 need you."), with a chevron, and opens Needs you.
- **One grid**, four equal columns, so each heading sits over its column: TAF, NOW, CARD, LAST
  RESULT (11 bold, dim), a line between rows, 16 apart across and 12 down.
  - **TAF:** the avatar (30) and the name (15 semibold), with who uses it under it for whoever
    manages it, once read.
  - **NOW:** what it's doing (13, ink).
  - **CARD:** the Card in front's figure and words (`cardLine`, `TafCard.hero`): "9 calls left
    today of 10", "€23,760 left this week of €25,000"; a use KIFF couldn't read shows the limit
    and "use unavailable", never zero. "–" with no Card.
  - **LAST RESULT** (13, soft, two lines at most, `lastResult`): the first line of its latest
    run's report with its `**` and `#` dropped, else that run's error, else "Last ran at 09:00",
    else "Nothing yet"; "–" for a taf tafs cloud doesn't run, as the sample's.
  - Every cell of a line opens its taf; a screen reader reads the line as one ("Deploys, Waiting
    for you · …, 9 calls left today of 10, –"), and Enter on it opens it.
  - With no tafs, "No tafs yet." across the four columns.
- **"Each line opens the taf."** under it, 12, dim.
- The column is at most 980 wide, centred, with 24 around it; the cream reaches both edges.

## Home from tafs cloud

Signed in, the window opens at once, then reads `GET /v1/home` on tafs cloud
(`https://cloud.tafs.kiff.dev`) as the session, in the app's language (`Accept-Language`), and
draws the sidebar and the Overview again from it (`TafsCloud.home`). The read is
`tafs-core/src/home.rs`, tested against the pinned `contracts/home.json` and `home-paused.json`.

- **As strict as the iPad's decoder:** a field `HomeTaf` requires and an answer lacks, or a time
  that isn't one, fails the read, and the screen stays as it was. A taf's health, its work line,
  its last change and Home's headline are read leniently, as on the iPad: an odd one never fails
  it. An answer that isn't a 200 leaves the screen as it was too.
- **The token goes only to tafs cloud's exact host** (`session::may_send_to`), with no
  redirect followed.
- **Each taf** (`KIFFMapping.taf`): what it's doing in the app's words for its state (Waiting for
  you, Paused, Halted · resume it from Halt, Works from its chat, Acting under KIFF, Card not
  active), or, while it works, tafs cloud's "Next run Fri 12:30" (else "Next run 12:30", else
  Running). A setup that stopped partway says tafs cloud's words instead.
- **What went wrong** (`healthNow`): tafs cloud's work line when it flags it, or when an idle taf
  isn't healthy. The sidebar says it where the next run would be, in the amber, red when its
  health is failing; the Overview keeps what it's doing.
- **Its Card** (`shownCards.first`): KIFF's first Card as tafs cloud read it, its money in its own
  currency and decimals, in the app's language (`Money.figure`), and a bare number saying it has
  no currency; else the Card merged from its lines, from an older tafs cloud.
- **Your tafs:** those relevant now (a call of its waits, or it ran or changed in the last 24
  hours). With tafs but none relevant now, there's no Your tafs: they're all on the Overview.
- **Needs you's count** is tafs cloud's, and so are its words.
- **Last result:** the first line of its last run's report (`GET /v1/tafs/{id}/runs`, then the
  latest run with a session, `GET /v1/tafs/{id}/runs/{session}`), else that run's error, else
  why the runs couldn't be read, else when it last ran ("Last ran at 09:00"), else "Nothing yet".
- **Who uses each taf** this person manages (`manages`, from KIFF's `GET /v1/me/access`): its
  shares (`GET /v1/agents/{agent}/grants`) named by group (KIFF's `GET /v1/groups`) or person
  (`GET /v1/tenant/members`), with how many people ("Support, lee@example.test · 3 people"), or
  "Not shared". Someone who only uses a taf doesn't see who else does.

These reads follow Home's after signing in or at launch, each on its own: one that fails leaves
its part out.

`--home shared/contracts/home.json` reads a contract instead (CI's `overview-home`).

## The sample

`--sample` (and Try it with sample data) draws the iPad's five sample tafs, each with its Card as
the sample's Record used it (Deploys 1 of 10 deploys today, Bill pay €1,240 of €25,000 this week,
Refunds €40 of €2,000 today, Chargebacks €35 of €500 this week, Access 1 of 40 roles today), and
the two calls their Cards held (`TafTemplates`, `AppModel.propose`): Deploys' migration, which its Card always asks
about, and Bill pay's €8,400 bill, over €5,000 each. A held call lapses 10 minutes after it was
held. `--signed-in` draws it with the account's items, as the reference set's `overview.png`.

## The view-only notice

When KIFF says the session is view-only, the screen beside the sidebar opens with "View only":
"This computer shows your tafs, Needs you and the Record. It can't approve until you add it with
your phone." and **Add this computer**, which opens the [pairing](pairing.md).

Once the computer is paired, it's still view-only (its key is in software, and KIFF lets a
software key approve nothing), so pairing again can't lift it. The notice then says "This computer
can't sign approvals itself. Approve on your phone, or with a passkey." (`pair.added_passkey`) and
has no button, as tafs-macos #90 does on macOS 12. The kept session says it was
paired (`KeptSession.paired`); a session kept before that reads as not paired.

## The order

The sidebar's tafs and the Overview's rows are in the iPad's order (`listedTafs`, `byRecent`,
`tafs-core/src/sidebar.rs` › `by_recent`):
- a taf a call waits on comes first;
- then the one used most recently, by its newest call in the Record (KIFF's activity, by its
  agent);
- then the tafs with nothing yet, in tafs cloud's order.
The iPad settles this when the list loads and when the app comes back, so it doesn't move while
the owner looks at it. This computer reads the account afresh each time it opens, so each read
settles it.

## Shared with you, and other agents

As the iPad's sidebar (`HomeSplitView`, `tafSections`, `otherAgents`):
- **Your tafs** is the tafs this person manages that are relevant now.
- **Shared with you** comes under it: the tafs KIFF says they may use but not manage
  (`sharedWithMe`, from `GET /v1/me/access`), relevant now. A taf whose answer isn't in yet counts
  as theirs, so nothing moves into "Shared with you" and back. Someone with only shared tafs sees
  just those, with no Your tafs.
- **Tafs being set up** (Home's `setting_up`) come after Your tafs' rows, and Your tafs shows
  for them even when no other taf is relevant now. Each is its name and colour, with no state.
  - Its line is tafs cloud's Finish setting up words, in the amber, when it sends a chat to
    finish in (`finish`: its taf, session and words). Finishing opens Coach, which isn't on this
    computer, so the row opens nothing.
  - Discard, for the person whose setup it was, is tafs cloud's button word, under the row on the
    right. It's signed, so it's there, dimmed.
  - Read as the iPad's own decoder reads it: whatever is missing or the wrong kind reads as
    empty, an item without a name is skipped, and none of it fails Home.
- **Other agents with Cards** comes last, after the recipes strip: every agent made through KIFF's
  API that tafs doesn't run (Home's `others`), apart. Each opens its page. When Home sends
  `setting_up`, `others` only repeats the tafs being set up for older apps, and it isn't drawn
  (`otherAgents`).
- The screenshots (synthetic) are CI's `sidebar-others` (`--home
  crates/tafs-app/fixtures/home-others.json`, which sends no `setting_up`) and
  `sidebar-setting-up` (`home-setting-up.json`: one with Finish and Discard, one plain, and two
  items skipped).

## The plan's line

At the sidebar's foot, above Account (`PlanLine`): the plan, only when it needs the owner. That's
tafs cloud's `home_line` from the balance (`GET /v1/usage/balance`), which is read with the
account, as the iPad reads it with each refresh. The line shows a dot and the words, red while
the plan waits on the owner (`plan_tone` `waiting`), else the amber. Then Plan & usage (12
semibold, amber), and a click opens it. A balance that can't be read, or with no `home_line`,
shows nothing. The sample has none. The screenshot (`--balance
crates/tafs-app/fixtures/balance-home-line.json`, synthetic) is CI's `sidebar-plan`.

## Not yet

- **What each row opens:** each screen but the Overview shows its title until it's ported.
- **Reading Home again:** it's read once, after signing in or at launch. A 401 there signs this
  computer out ([sign-in](sign-in.md)).
- **Its order, in part:** a connection its vendor signed out doesn't count as waiting yet (planque's
  grants aren't read), and a taf's chat doesn't count as being at work (chats aren't on Linux).
- **Held calls, steps and knocks** from KIFF: until they're read, Your tafs is what ran or changed.
- **The account's name** under Account, and its zone for times: from KIFF's account read.
- **A room's end on a later day** shows its date as 2026-11-16 00:00, not in the language's words
  yet.
- **Finish setting up** and **+ New**, both with Coach; **Discard** itself, which is signed.
- **The Unreachable line** at the foot: Linux reads the account once and doesn't try again by
  itself yet.
- **Coach mode:** its button opens its title for now.
