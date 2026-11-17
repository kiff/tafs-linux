# A taf's page

The iPad's taf page (`TafDetailView`), beside the sidebar: a taf's row in Your tafs, a line of
the Overview, or a held call opens it. What it says is `tafs-core/src/taf_page.rs`, tested there;
the page is `crates/tafs-app/src/taf.rs`.

## The page

- **The head** (`header`): the avatar (34), the name (20 bold, one line), and its state as a chip
  (`StateChip`): Working in green, Waiting in amber, Stopped and Halted in red, Ready and Paused
  in the soft colour, each on its colour at 15%. When tafs cloud flags what went wrong, the chip
  isn't shown in place of Working.
- **The tabs:** Chat, Activity, Card and Setup, as the system's segmented control. A tafs cloud
  taf's page, signed in with KIFF, opens on its chat ([chat.md](chat.md)); without it, on Activity.
- **The Card bar** (`CardBar`): a small Card in the taf's colour (the dim colour when it isn't
  live, faded when revoked or not active) with the Card's mark, the Card's figure and words ("€1,960
  left today of €2,000", else its title), and under it its name, its environment when it isn't
  production, and how many Cards ("Refunds · Test · 3 Cards"); a chevron at its end.
- **Its held calls**, under NEEDS YOU in the amber, as Needs you's cards.
- **Activity** (the sample's, `activity(_:)`): All, Done, Held, Refused (Done is the iPad's
  `taf.halt.part_held`), then its lines by day, newest first, each day's heading with what was
  spent, each line as the Record's without its taf. "Nothing yet." with All, else "Nothing here."

- **Card** (`card(_:)`): the taf's Cards as a wallet stack (`CardStack`), the front one whole and
  each one behind showing its top 26 (`WalletLayout`), a shade darker the deeper it is; tapping one
  brings it forward. Each face (`TafCardFace`, 300×189, corners of 18) is white on the taf's
  colour, the dim colour when it isn't live, faded when revoked or not active: its title
  (monospaced 12 semibold) and its environment as a pill when it isn't production, the main
  limit's figure (34 bold) and words, its facts (monospaced 11, one line, smaller before it's
  cut), "No actions" when it has no limits, then "Issued by" who and when it ends ("Ends" or
  "Until", "No end date"). Under the stack, every limit of the front Card (`CardLines`): its
  label (15 semibold), what happens outside it (12, dim), and its summary (monospaced 13, at the
  end, red when never allowed). Then Revoke, in red, dimmed until it's ported, in the sample and
  for a taf this person manages (`manages`, KIFF's `GET /v1/me/access`). The Card bar opens this
  tab, and isn't shown on it.
- **Who issued a Card** (`issuer(of:)`): "You" in the sample and for a taf this person manages;
  for any other KIFF taf, the account's name, else "The account".

`--page taf` opens the sample's Refunds, as `taf.png`; `--page taf-card` opens it on its Card
tab, with the reference set's three Cards, as `taf-card.png`; `--page taf-card:<id>` opens a taf
of `--home` on it.

## A tafs cloud taf's Activity

A taf tafs cloud runs draws its own Activity (`TafActivityList`), a page at a time from
`GET /v1/tafs/{id}/activity?filter=…&cursor=…` on tafs cloud as the session (`activityPage`), all
in tafs cloud's words (`tafs-core/src/activity.rs`):

- its held calls first, as Needs you's cards;
- its Card unit requests (`.cardUnits`), each as Needs you draws it (`CardUnitExceptionPanel`,
  its Coach button and Dismiss dimmed): the last page's own (`card_unit_exceptions`) once read,
  else Home's for the taf; then tafs cloud's notice (`card_unit_exceptions_notice`), 13, dim. A
  page that sends them reads as strictly as Home: one without what the iPad needs, and the page
  isn't read;
- tafs cloud's filters, from its first page, as bordered buttons (the chosen one in the amber as
  text); choosing one reads its first page again;
- each day group's title (12 semibold, dim), then each call (`SourceActivityRow`): its title (15
  semibold) and detail (13, soft), its time (12, dim) and amount (13 semibold) at its end, and
  under it its next step in tafs cloud's words;
- at the foot: the next page as the list reaches its end (`next_cursor`), tafs cloud's end words,
  "Nothing here." when no page has an entry, or what went wrong and Try again.

Read as strictly as the iPad's decoder, with one exception: a call's `held`, which the iPad's
`ActivityCall` requires and tafs cloud's `taf-activity.json` doesn't send, reads as not held when
it's absent. `--home shared/contracts/home.json --page taf:<id>` reads `taf-activity.json` beside it
(CI's `taf-home`). With `--home crates/tafs-app/fixtures/home-waits.json`, its own
`taf-activity.json` (the contract's, with Home's request as the page's own, synthetic) draws one
(CI's `taf-units`).

## Its knocks

For whoever manages a tafs cloud taf, and only where Home says tafs cloud offers doors
(`doors_offered`), its doors' knocks come first on its Activity, before its held calls
(`TafKnocksPanel`, `crates/tafs-app/src/knocks.rs`, `tafs-core/src/doors.rs`). The doors are read as
the session when the Activity opens (`GET /v1/tafs/{id}/doors`), tried again quietly after 1 s and
3 s; nothing is drawn until they're read, and nothing when no message waits. Read as the iPad's
decoders read them: every door and app whole, or none; a knock without its id, door, app, sender
or time, none; the rest of a knock when it can be.

- **Knocks · waiting on you** (amber eyebrow, the panel's edge in amber at 45%): each open knock
  from someone not paired, then **Paired**, a paired sender's message kept from its knock. The
  door's line about its day's messages (`rest`) isn't one of them, here or on the doors' screen
  (as the iPad filters it); `KnockCard` draws it with OK (an Ignore) where it's given one.
- **Each knock** (`KnockCard`), on amber at 7% with an amber edge, a paired sender's on the panel
  with a quiet edge: "● KNOCK · BY EMAIL" or "PAIRED · BY EMAIL" (10 semibold, monospaced), with
  "· DIDN'T CHECK OUT" when the sender check failed; for a paired sender "Paired · Do it or
  Ignore this message"; "Ana Ruiz (ana@example.com) wrote about "…". Nothing was done." (14
  semibold); the message (14, soft, three lines, selectable); its attachments, not opened; why
  the sender can't be trusted, when it can't.
- **Its answers**, unsigned, `POST /v1/tafs/{id}/doors/knocks/{knock}` as the session: **Do it**
  (amber, not for a sender who failed the check) opens "Tell Rae what to do" with the owner's
  words (Do what it asks, within your Card) and Send to Rae; **Pair Ana** or Pair them, for a
  sender who passed the check and isn't paired yet, which reads the doors again, the message
  still waiting; **Ignore**. A knock answered goes away. Why one wasn't sent shows under the
  panel, or in Do it while it's open; a 429 without tafs cloud's words is the app's own line.

`--doors <file>` with `--home` draws a taf's doors from a file, as if tafs cloud offered them, and
its answers send nothing: `crates/tafs-app/fixtures/taf-doors.json` (synthetic), with
`access-manage.json` so the taf is managed, CI's `taf-knocks`.

## Its doors

`DoorsView` (`crates/tafs-app/src/doors.rs`, `tafs-core/src/doors.rs`), with one `DoorsModel` per
view that every panel and sheet on it listens to. At the pin the iPad's page has no tab for it (its
tabs are Chat, Activity, Card and Setup): one door's screen opens from Setup › Starts when › Doors ›
Edit (`only`), and the whole of it, with the doors' own dial, from nowhere. Linux draws both, in a
sheet over the page.

- **The doors' dial** (`DoorDial`, without the taf's own dial): DOORS · TURN TO PICK, PRESS TO
  OPEN; the taf's avatar in the middle on a knob with grip ticks, in a glow that breathes while a
  door is open; each door's arc on the ring, lit in the taf's colour while open, grey while resting,
  dashed while closed; a pointer at the top; each door's badge on the ring (`DoorBadge`: "@" for
  email, "W" for WhatsApp, else its initial; filled in its colour while open) with an amber dot
  while a knock waits there, its name outside in capitals, and a last dashed "+" stop, ADD. Under
  the avatar, "EMAIL · OPEN" or ADD A DOOR, in the taf's colour unless the door is closed. Under the
  dial, tafs cloud's tap words for the door, else "Press Rae to close Email", or "Press Rae to add
  a door". The ring is Cairo shapes; every word and letter on it is a label over it (rule 14).
- **Turning it:** dragging turns it (past 6 points), a tap on a stop turns the short way to it,
  the arrow keys a stop at a time, each springing to its stop (`DialMath`, its tests ported);
  with animations off it jumps. A tap on the taf, Enter or Space presses: for whoever manages it,
  the door at the top opens or closes, or Add a door opens. It's a slider to a screen reader, with
  "Email door, open, 1 of 3".
- **The door at the top** (or the screen's own door): tafs cloud's line that its settings are
  below; its badge, EMAIL DOOR and where it stands (tafs cloud's words, else Open, Closed or
  Resting until tomorrow), with a switch in green for whoever manages it; its address (monospaced,
  selectable), Copy address for email ("Copied" for 1.4 s); a meter of today's messages, in the
  taf's colour or grey while resting, and the day's lines. Then PAIRED · CAN GIVE WORK with each key
  ("You" only on this person's own sign-in email; a name that is only the address isn't said
  twice; Unpair for one the owner paired), how it knows who wrote, Pair someone; or, for a door
  that can't tell, why no one can give work. Remove door in red.
- **With no door:** for whoever manages it, ADD A DOOR, what a door is, and Add a door (none left:
  "No more doors can be added here."); for anyone else, that someone who manages it can add one.
  A door removed from its own screen: "This door was removed."
- **Its knocks:** Paired, then Knocks · waiting on you, each `KnockCard` as on the Activity; why
  a change failed, in red; the footer.
- **Add a door** (`AddDoorSheet`): its title and Done, a search, the apps that can tell who's
  writing, then those that can't, each with its badge and how; "No app called "…" here yet." or
  none left; replies only go to who wrote.
- **Pair someone** (`PairSheet`): what pairing does, the sender (email address, phone number, Slack
  member ID or the door's own word for it), a name (optional), Pair while there's a sender, Cancel.
- **The owner's changes, as the session, unsigned** (`kiff::cloud_send`): adding a door
  (`POST /v1/tafs/{id}/doors {"app"}`), opening or closing it (`PATCH …/doors/{door} {"open"}`),
  pairing (`POST …/keys {"id", "name"?}`, the sender trimmed), unpairing (`DELETE …/keys/{sender}`),
  each answered with the door (`{"door"}`), which replaces the one shown; removing it
  (`DELETE …/doors/{door}`, answered `{"removed"}`), after its question. A change that fails says
  why in red and leaves the doors as they were.
- **WhatsApp:** its signup comes back to the iPhone's own address, which this computer has none
  of, so Add says "WhatsApp doors are added from tafs on an iPhone for now; once added, they show
  here." and sends nothing.
- **A Slack door's removal**, from one door's screen: planque's subscription is stopped first,
  signed by the device; this computer can't, so Remove's yes shows the can't-approve line and the
  door stays ([approval](approval.md)). Adding one through Slack's own sheet is Starts when's.

`--page doors:<id>` with `--doors` and `--home` opens the doors over the taf's page, and
`--page doors:<id>:<door>` one door's screen, from a file, sending nothing:
`crates/tafs-app/fixtures/taf-doors-dial.json` (synthetic, from the iPad's `DoorsModel.sample` and
`DoorsTests`), with `access-manage.json` and `me-manager.json`, whose email is the owner's key.
CI's `doors` and `doors-door`.

## Its work

Above its Activity, after its held calls, a tafs cloud taf's work (`WorkPanel`,
`crates/tafs-app/src/work_panel.rs`, `tafs-core/src/work.rs`). It's read once as the session
(`GET /v1/tafs/{id}/work`), and nothing is drawn until it's read, as the iPad's. Under "Its work":
- **its line** (15, ink), unless its health's first line already says it (`repeats(workLine:)`);
- **its health** (`HealthLines`): the work's, else Home's. Each line is amber, or red when it's
  failing. A `credits` fix opens Plan & usage. A `coach` fix draws no button: Coach isn't on this
  computer, as the iPad's without `mayUseCoach`;
- **its oldest waiting case** in tafs cloud's words, amber and semibold once past the promise;
- **its requests and skipped firings,** newest first. A request's line is amber when it needs
  you, red when it failed, else soft, with what it used ("34 credits") and where its result went
  under it. A skipped firing is dim: nothing ran, which isn't a failure.
- **What went wrong reading it** in red, in its place.

Read as leniently as the iPad's `TafWork`: a part that isn't one reads as none, and a list with
one item it can't read reads as empty. The screenshot (`--work
crates/tafs-app/fixtures/taf-work.json`, synthetic: no contract for it at the pin) is CI's
`taf-work`.

## Its runs

After its work, a tafs cloud taf's runs (`crates/tafs-app/src/runs_panel.rs`,
`tafs-core/src/runs.rs`):
- **Runs** (`RunsPanel`): it works from its chat (no schedule, no Run now); else Paused, or tafs
  cloud's words for its next run ("Next run Fri 12:30"), or "Next run at 12:30", or On its
  schedule.
  - **Run now** (`POST /v1/tafs/{id}/run` as the session, no signature) turns to "Starting a
    run…" on the tap, so a second tap can't send one. A run it started shows among the runs
    it kept, read again. If its check started no run, tafs cloud's words say why. If it
    didn't start, "The run didn't start." and why, in red.
- **The runs tafs cloud kept** (`RetainedRuns`): its heading, the first page's 3 runs (`GET
  /v1/tafs/{id}/work/runs?limit=3`) with the firings that started no run in their places, each
  in tafs cloud's words.
  - Under them, its "more" line (`more_title`) opens every run in a sheet, 20 a page. The next
    page is read as the list reaches its end, then tafs cloud's end words.
  - What went wrong reading is shown with Try again.
- **A run's own page** (`KnobRequestDetail`): each kept run, on the page and in the sheet,
  opens it over the window (`GET /v1/tafs/{id}/requests/{run}`), titled Request:
  - where it came from (`from`), and its line;
  - each step with its mark: done (ok), next (a clock) or what went wrong (a warning), both
    amber; its line under it;
  - for whoever manages the taf, Try the reply now on each delivery that can be sent again
    (`POST …/requests/{run}/deliveries/{place}/send`, unsigned). It turns to "Sending again…",
    and the page is read again. If it fails, "It wasn't sent again." and why.
  - Ask Coach isn't here: Coach isn't on this computer.
  - What went wrong reading it shows with Try again.
- **Read as strictly as the iPad's `RunsPage`:** a page without its title, or with a run it
  can't read, isn't one.
- The screenshot (`--runs crates/tafs-app/fixtures/taf-runs-page.json`, synthetic) is CI's
  `taf-work`, with the work.

## A Card's own lines

On the Card tab, as the iPad's `card(_:)`, all from Home as tafs cloud read it from KIFF:
- **At the top,** tafs cloud's words for the zone its times are in (`time_zone_words`, 13, soft).
  When its schedule runs in another zone, those words follow (`schedule_time_zone_words`, amber).
- **Under the front Card's limits:**
  - its totals for each item apart (`each_item`, 14, ink), blank ones dropped;
  - **How this is counted** (`counted`), a row that opens each tool's title over KIFF's lines,
    as the owner approved them (`CountedView`). The app adds no words of its own;
  - what of its totals is from calls the tool refused (`pending_text`, the money total's then
    the count's, 13, soft). This shows only for a live, active Card (`countsForReal`), since a
    test Card's totals aren't what the taf may do for real;
  - why its total has no currency (`unit_words`, 13, soft). Its Connect again opens Coach,
    which isn't on this computer.
- The screenshot (`--home crates/tafs-app/fixtures/home-card-lines.json`, synthetic) is CI's
  `taf-card-lines`.

## Its Setup

A tafs cloud taf's Setup tab, signed in with KIFF (`TafDetailView.setup(_:)`,
`crates/tafs-app/src/setup_pane.rs`, `tafs-core/src/setup.rs`), read when the tab is first shown
(`GET /v1/tafs/{id}/setup`, as strictly as the iPad's `SetupView`). Linux reads it without
planque's grants proof, which this computer can't sign, so tafs cloud sends `connections_error` in
place of the connections. Until it's read, a spinner; when it can't be, why and Try again. Then, in
the iPad's order:

- **Its Card unit requests** (`card_unit_exceptions`), as Needs you draws them.
- **The job:** THE JOB, and its job as tafs cloud says it (`job_summary`), else "Works under {n}
  Cards". Improve in Coach is Coach's, which isn't on this computer.
- **What it remembers** (`MemorySummaryPanel`): tafs cloud's line for its notes (15 semibold), its
  latest (14, soft, two lines), and See all.
  - **See all** opens every note (`MemoryPanel`, `GET /v1/tafs/{id}/memory?all=true`), newest
    first: its words (15), its name from its path and time (12, soft), and Forget; then what the
    notes are for. With none, "Nothing yet…". Why they couldn't be loaded, in red. The Setup is
    read again once it's closed.
  - **Forget** asks first ("Forget this note?", the note, that it can't be undone, Forget in red,
    Cancel). Its yes takes the note away at once and `DELETE`s it at tafs cloud as the session,
    unsigned; a note already gone (404) counts as forgotten. A refusal puts it back, with
    "It wasn't forgotten. {why}".
- **Remove for good**, for whoever manages it: signed, so it's there, dimmed.

`--page taf-setup:<id>` with `--home shared/contracts/home.json` reads the contract's
`taf-setup.json` beside it, and `--memory crates/tafs-app/fixtures/taf-memory.json` (synthetic)
for See all; Forget sends nothing. CI's `taf-setup`.

## Not yet

- **The sample's Chat**, there, dimmed. A tafs cloud taf's page opens on its chat ([chat.md](chat.md)).
- **On the Setup tab:** Starts when, Results go to, the connected apps' ring, its own Connect
  again (signed), Improve in Coach, and the sample's Setup (there, dimmed).
- **On the Card tab:** Connect again (Coach), the halt row,
  Versions, Usage; Revoke itself (dimmed).
- **Above a tafs cloud taf's Activity:** the next step's buttons (Connect it
  again, Ask Coach), shown as their words for now; a call's receipt; "All apps" for a selection.
- **The line under the head:** its version and who it's shared with (`headerLine`, `ShareLine`).
- **In its work:** Answer within (`PromiseRow`, whoever manages the taf), Send again, and a step
  it handed you as its card (`YourStepCard`): its line for now.
- **The side column** on a wide window, Remove, and folding the head while scrolling.
