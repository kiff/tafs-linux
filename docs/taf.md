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
- **The side column** (`sideColumn`), on Chat and Activity when the page is wider than tall and
  keeps 430 for the chat beside a 360-wide column: the held calls, CARDS (or CARD) with their stack,
  and All of the Card (or All N Cards), which opens the Card tab. Meanwhile the Card bar and the
  held calls in the activity step aside, as on the iPad in landscape.
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

`--page taf` opens the sample's Support replies, as `taf.png`; `--page taf-card` opens it on its
Card tab, with its one Card named for its helpdesk and its replies counted, as `taf-card.png`; `--page taf-card:<id>` opens a taf
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
  semibold) and detail (13, soft), its time (12, dim) and amount (13 semibold) at its end. The row
  opens its receipt (`ReceiptView`, the Record's, on the call as the Record's line with tafs
  cloud's kind, title and detail: `activity::evidence`). Under it its next step (`NextStepRow`):
  for whoever manages the taf, signed in, a button in tafs cloud's words: Connect it again opens
  Connect apps (signing that connection in again where its vendor signed it out needs planque's
  grants, which this computer doesn't read), Ask Coach opens Coach on this taf with tafs cloud's
  message, else the row's own words, in the box, nothing sent (for whoever may use Coach); anyone else, and a
  kind the app doesn't know, reads the words alone (`activity::next_action`);
- at the foot: the next page as the list reaches its end (`next_cursor`), tafs cloud's end words,
  "Nothing here." when no page has an entry, or what went wrong and Try again.
- `--taf-activity crates/tafs-app/fixtures/taf-activity-next.json` (synthetic: the contract's page
  with two not-done calls and their next steps) in the contract's place: CI's `taf-next`.

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

The same knocks show in its chat at their times ([chat.md](chat.md#a-doors-knocks)), from one doors
model for the taf, shared by its chat, this panel and its doors' screen.

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
- **A door "From another application"** (`kind: "api"`, `APIDoorPanel`): FROM ANOTHER APPLICATION
  over its name, the switch; while paused, "Requests are refused until you open it…" in amber;
  TODAY, a meter in the taf's colour and "37 of 100"; "Last request {time} · {status}" only in
  tafs cloud's words (none drawn until it sends them, never its raw `status`), the time in the
  account's zone (`locale::date_time`, the iPad's `.abbreviated` date and `.shortened` time). Then
  KEYS, each with its name and "Made … · last used … · ends …iXrW" (never a key), Revoke in red;
  New key (off at two keys); the two-keys note and what a door's key can't do; **API
  instructions**, a dialog with where to send work (the whole address on tafs cloud), the example
  request reading `$TAFS_API_KEY` (monospaced, left to right in every language, scrolled sideways)
  and that the key isn't here; Remove door.
  - **Pausing it** is the one change sent from here (`PATCH …/doors/{door} {"open": false}`).
    Opening it again, New key, Revoke (after its question) and Remove (after its) are signed at tafs
    cloud by the phone's approval key: this computer shows the can't-approve line with "Approve on
    your phone instead" and sends nothing (`signed::Act::at_api_door`). Another door's changes go
    unsigned at the same paths, so the path can't tell them apart: `DoorsModel` asks
    `doors::api::only_signed` before every call and drops these.
  - `--page doors:<taf>:<door>:<button>` presses a button once the door is drawn, for the
    screenshots: `crates/tafs-app/fixtures/taf-doors-api.json` (tafs cloud's contract) and
    `taf-doors-api-paused.json` (synthetic: paused, one key), CI's `doors-api`,
    `doors-api-new-key` and `doors-api-instructions`.

### The taf's dial

Where the taf's dial was read, it stands in the doors' own dial (`TafDialPanel`, `TafKnobView`,
`crates/tafs-app/src/dial.rs`, `tafs-core/src/knob.rs` › `nav`, `face_lines`), the door's panels
under it, as on the iPad's Doors tab:

- DRAFT · V8 · NOT LIVE while a version waits; Ask Coach or find anything (opens Coach); the
  service menu, "Showing All services", with This taf and each connection's stops ("Notion · 2
  stops"), a new choice starting from the job with every ring unturned.
- **The dial**, on its 360 grid: today's work round the rim (done green, working blue, retrying
  amber, needing you red), When, What and Where as bands in their own colours inside it, in tafs
  cloud's order, each showing six stops at a time (`page`), a door's stop with its arc (amber while
  a knock waits, grey while resting, dashed while closed) and its light; the pointer at the chosen
  ring's top stop, or at the rim; the face with its kicker, its big word (the stop's state, its
  size, its label; the call's amount; else the taf's why) and its hint. Choosing a stop turns its
  ring the short way to bring it to the top (`nearestTurn`) and dims the other rings; a door's
  stop on When picks that door for the panels below. The rings are shapes; the stops' labels are
  buttons over them, upright (the iPad turns each with its ring).
- **The face pressed:** a held call that may be approved once (signed: the can't-approve line);
  Coach for an "Add a door" stop; else, for whoever manages the taf, the chosen stop's act as the
  session: a door opened or closed (`PATCH …/doors/{door}`), a run started (`POST …/run`), a
  follow-up removed; an app event to approve opens its filters (`AppEventApprovalSheet`), where
  Approve, signed at planque, stands as the gate's line ([approval](approval.md)). With nothing
  chosen, it walks to the first stop.
- Under it, the stop's words or the dial's caption, the size in force, Needs you while a call may
  be approved once (its face), ‹ Why back to the job; **the remote** (Back, Lower, the power
  button to pause or resume every start, Raise, Next), under the dial here since a sheet that
  scrolls has no foot to keep it on; the waiting version's lines and Make it live (refused, as in
  Starts when); tafs cloud's line for today's work, opening its requests (`KnobRequestsSheet`),
  the one still asking for attention opened at once on its own page (`KnobRequestDetail`).

`--knob crates/tafs-app/fixtures/taf-knob-dial.json` (synthetic: the iPad's `Screenshots.knob`
with four services) with `--page doors:<id>` is CI's `taf-dial`.

`--page doors:<id>` with `--doors` and `--home` opens the doors over the taf's page, and
`--page doors:<id>:<door>` one door's screen, from a file, sending nothing:
`crates/tafs-app/fixtures/taf-doors-dial.json` (synthetic, from the iPad's `DoorsModel.sample` and
`DoorsTests`), with `access-manage.json` and `me-manager.json`, whose email is the owner's key.
CI's `doors` and `doors-door`.

## Its work

Above its Activity, after its held calls, a tafs cloud taf's work (`WorkPanel`,
`crates/tafs-app/src/work_panel.rs`, `tafs-core/src/work.rs`). It's read as the session
(`GET /v1/tafs/{id}/work`), and again after a change; nothing is drawn until it's read, as the
iPad's. Under "Its work":
- **its line** (15, ink), unless its health's first line already says it (`repeats(workLine:)`);
- **its health** (`HealthLines`): the work's, else Home's. Each line is amber, or red when it's
  failing. A `credits` fix opens Plan & usage. A `coach` fix, for whoever may use Coach and
  manages the taf, opens Coach with tafs cloud's words in its box, nothing sent;
- **its oldest waiting case** in tafs cloud's words, amber and semibold once past the promise;
- **Answer within** (`PromiseRow`), for whoever manages the taf: the promise ("4 hours", "1 day")
  or Not set, and a menu of Not set and 1, 2, 4, 8, 24, 48 and 168 hours; a choice is
  `PUT /v1/tafs/{id}/promise {"hours"}` (null clears it) as the session, then the work is read
  again; "It couldn't be set. …" when it wasn't. "You're told when a case waits longer." under it;
- **its requests and skipped firings,** newest first. A request's line is amber when it needs
  you, red when it failed, else soft, with what it used ("34 credits") and where its result went
  under it. A step the run handed its owner (`your_step`) stands in its line's place as its card
  (`YourStepCard`, [Needs you](needs-you.md)), without the taf's name: Home's copy of it where
  Home lists it, which carries the vendor's link when tafs cloud vouched for it, else the
  request's own. Done and Skip it are signed: their tap says this computer can't approve. A door reply given up on offers **Send again** to whoever manages the taf
  (`POST …/requests/{run}/deliveries/{place}/send`, unsigned; "Sending again…" while it goes,
  "It wasn't sent again. …" when it didn't). A skipped firing is dim: nothing ran, which isn't a
  failure.
- **What went wrong reading it** in red, in its place.

Read as leniently as the iPad's `TafWork`: a part that isn't one reads as none, and a list with
one item it can't read reads as empty. The screenshot (`--work
crates/tafs-app/fixtures/taf-work.json`, synthetic: no contract for it at the pin) is CI's
`taf-work`, and with `access-manage.json`, Answer within and Send again, `taf-work-manage`; a
screenshot's changes send nothing.

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
  - Ask Coach, for someone who may use Coach and manages the taf (`offersAskCoach`), and in the
    sample: the sheets close, and Coach opens on this taf ([Coach on a taf](coach.md)).
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
  - why its total has no currency (`unit_words`, 13, soft), with **Reconnect it in Coach** for
    whoever manages the taf and may use Coach, signed in (`offersReconnect`), which opens Coach.
- **Revoke**, for whoever manages the taf: a view-only session (KIFF's `approval_mode: view`)
  gets "To revoke this Card, use a device that can approve." in its place, as on every device;
  otherwise its question (Revoke and stop in red), whose yes revokes each of its Cards at KIFF as
  the session (`POST /v1/me/mandates/{id}/revoke`, unsigned: it takes authority away), then the
  button goes; KIFF's refusal is said under it. The sample's is there, dimmed.
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
- **Its own Connect again** (`connect_again`, for whoever manages it): tafs cloud's words in amber,
  and its button in tafs cloud's words. Signing that connection in again is approved at planque,
  signed by the device: the tap shows the can't-approve line and sends nothing ([approval](approval.md)).
- **The job:** THE JOB, and its job as tafs cloud says it (`job_summary`), else "Works under {n}
  Cards"; **Improve in Coach** for whoever manages it and may use Coach, which opens Coach on a new
  conversation on this taf ([Coach on a taf](coach.md)).
- **Starts when** and, after the connected apps, **Results go to**: rows that open their screens
  (below).
- **Connected apps** (`ConnectedToolsRing`): why tafs cloud left the connections out (this
  computer reads Setup without planque's grants proof, so it says so), with Try again; the taf in
  the middle (its job in an alert), one node per app around it (`setup::nodes`: the connection with
  actions, else one to sign in again, else the first; its name and actions, green while it runs),
  dashed lines to it and a green arc while it runs, and + for whoever manages it, opening Connect
  apps. Under it the workflow, folded: its title and summary, each step (green while it runs) and
  Change the workflow with Coach. A node opens its sheet (`ConnectedAppSheet`): where it stands
  (Sign in again: signed, refused here), what it may do, its workflow, its calls (opening the
  Activity narrowed to them: tafs cloud's `activity_query` after the filter, as it wrote it, left
  out unless it's a plain query; "All apps" over the list shows every app's again), other
  connections, Add actions with Coach, and Remove access, whose question's yes is
  signed at planque and refused here.
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
- **Remove for good**, for whoever manages it: tapped, it reads what goes with the taf
  (`GET /v1/tafs/{id}/removal`, ending nothing), then asks ("Remove Rae for good?", what always
  goes and tafs cloud's words, Remove in red). Its yes is signed by the approval key: this
  computer shows the can't-approve line under the button, and nothing is removed.

`--page taf-setup:<id>` with `--home shared/contracts/home.json` reads the contract's
`taf-setup.json` beside it (`--setup` another, such as `taf-setup-reconnect.json`), and `--memory
crates/tafs-app/fixtures/taf-memory.json` (synthetic) for See all; Forget sends nothing. CI's
`taf-setup` and `taf-setup-apps`.

### Starts when and Results go to

`TafSetupViews.swift` (`crates/tafs-app/src/starts_when.rs`, `tafs-core/src/knob.rs`), each a sheet
with its own navigation, over the dial as tafs cloud sends it (`GET /v1/tafs/{id}/knob`, and when a
version waits, `?version=` that one; a 404 then reads it again as it is now). Read as strictly as
the iPad's `Knob`: a field in the wrong shape and the dial isn't read ("The dial couldn't be read:
…"); one missing takes the iPad's default. Until it's read, "Reading what starts it…" or "Reading
where its results go…". Pulled past its top (`.refreshable`: a touchpad's or a touch screen's
pull, as GTK reports it), or on Ctrl+R or F5 while the focus is in it, a screen reads its dial again (`loadKnob`), and
Results go to its work too (`loadWork`, whose failed read keeps what was read); what was read stays in sight meanwhile, and a pull while
one is on its way asks nothing more.

- **The waiting version** (`KnobPendingPanel`, for whoever manages it): DRAFT · V8 · NOT LIVE in
  amber, tafs cloud's lines for it, and Make 2 changes live with your passcode (what the iPad names
  on a device with no Face ID or Touch ID). Making it live begins tafs cloud's switch, which stops
  the taf until it's finished, after the device's own check: this computer begins nothing, and the
  tap shows the can't-approve line.
- **Starts when** (`StartsWhenView`): its schedule (each line, − and + through tafs cloud's sizes,
  held while a newer version than the one shown waits, `acceptsChoices`; the next run and the
  account's zone; Skip it or Run after while still working); app events (Approve, disabled as the
  iPad disables it where the device can't approve; or Listening and Stop); its doors (each with
  Edit, opening the door's screen on top, [its doors](#its-doors), and Add a door…, whose Slack door
  opens Slack's sheet: planque's Slack connections aren't read here, so it says to connect the bot
  first); follow-ups it set itself, with their end and Remove; Change when it starts with Coach;
  Pause every start, or Resume ("Not every start could be paused. Try again." on a 502).
- **Results go to** (`ResultsGoToView`): at least one place stays on; tafs' own places (Reply to
  the sender, Alert our phones, Email me with the person's email), switched from the live taf's
  `results` and never a waiting version's, the last one on disabled (`liveBuiltIns`, `places`);
  each place in a connected app, through its other choice; Add a place from a connected app… with
  Coach; replies not sent, with Send again.
- **As the session, unsigned:** a dial change (`PUT …/knob`, the live `setting` copied, changed and
  sent whole, `when: {busy}`; it waits as the next version), tafs' own places (`PUT …/results
  {"places"}`, live at once), a follow-up removed (`DELETE …/follow-ups/{id}`), every start paused
  or resumed (`POST …/pause`, `…/resume`), a reply sent again. Each failure says why in red.

`--page starts:<id>` and `--page results:<id>` (the iPad's `-screen starts`, `-screen results`) open
them over Setup, with `--knob crates/tafs-app/fixtures/taf-knob-setup.json` (synthetic, the iPad's
`Screenshots.setupKnob` with a version waiting), `--doors` and `--work`; nothing is sent. CI's
`taf-starts` and `taf-results`.

## Sharing it

Under the head row, for whoever manages the taf, signed in with KIFF (`headerLine`'s `ShareLine`):
**Share** (a capsule with the share mark, 15 semibold) and who it's shared with beside it (14,
soft): "Shared with Finance and sam@example.test", "Shared with everyone" for KIFF's default group
alone, "Not shared" (the reader is never named), "Sharing…" until read. The shares
(`GET /v1/agents/{agent}/grants`), groups and people are read as the session, and again when the
sheet closes (`share.rs`, `tafs-core/share.rs`).

Share opens its sheet (`ShareTafSheet`), "Share {taf}", in two steps:
1. **Who uses it**: each group, its people under it, managers first ("Maria (manager), Ana"), and
   each person with their badge, each picked with a circle; "Choose someone on the account." under
   them. Cancel at the start; Next once read and something changed.
2. **What it means**: "Who gets {taf}" (or "{taf} won't be shared"), then each person's own chats,
   who gets it (a group with its people, everyone, a person) and who stops using it. Back at the
   start. **Share** at the end is signed on the iPad (one Face ID for every grant added and taken
   away, `signsSetup`'s grants route), so its tap says this computer can't approve, and nothing is
   sent.

The line sits under the head row from `shell.rs`, as the taf page's own file belongs to another
port. Screenshots: `--home shared/contracts/home.json --access
crates/tafs-app/fixtures/access-manage.json --people crates/tafs-app/fixtures/people-kiff.json`
with `--page share:taf-aaaa0000000000000000` (step one, CI's `share`) or `share-step2:…` (step two
with a change picked and Share tapped, CI's `share-step2`).

## Its Usage

On the Card tab, for whoever manages a tafs cloud taf, signed in: Usage, in amber with a chart and a
chevron, opens Plan & usage on this taf's credits this period and its runs (`TafCreditsView`,
[usage](usage.md)); Back there returns to Plan & usage.

## Its versions and its trial

For the person who manages a tafs cloud taf, under its name (`VersionsPanel`, `TrialViews.swift`;
app `versions.rs`, `tafs-core/versions.rs`): "Version 2 · live" (else "Versions") with a clock and a
chevron, and, while a trial is open, "Trying v3 · Resume" in amber. Both are read from tafs cloud
as the session when the page opens (`GET /v1/tafs/{id}/versions`, `GET /v1/tafs/{id}/trial`);
nothing shows until they're read, or when they can't be. A 401 signs this computer out.

- **Versions**, in a sheet: the eyebrow and what versions are; a switch under way, with Finish
  and Cancel; then each version, newest first: its number, "Live now" (green) or "Trying"
  (amber), its note, who made it and when, "brings a new Card" when it does, and what each ended
  trial came to ("Tried Nov 2 · 3 calls, 1 held · went back"). The live one offers nothing; the
  one being tried offers Resume the trial; the rest offer Try it and Install this version, both
  closed while a trial is open, and Install closed while another version's switch is under way.
- **What's sent:** Cancel on a switch (`DELETE /v1/tafs/{id}/switching`), and the trial's chat.
- **What's drawn up to the signature:**
  - Install this version, Finish: the switch the device begins and signs (`Act::MakeLive`); the tap
    shows the can't-approve line, and nothing is sent.
  - Try it opens what trying it means ("While you try it" and its four lines), then its Card, as
    the owner reads it before approving (`trialCard`): tafs cloud prepares it as a KIFF draft
    (`POST /v1/tafs/{id}/versions/{n}/try`, as the session) and KIFF's draft is read and drawn
    under "Its Card"; one that can't be read says why. **With a security key**, Start trying
    (`startTrying`) reads the draft again (changed since it was shown, it's drawn again with "What
    you're asked to approve changed…", to read first), the key approves KIFF's
    `POST /v1/me/agent-drafts/{id}/confirm` with `{"as":"trial","version":…}`, exactly the version
    read (`Act::ConfirmDraft`, the sheet saying "Try version 3 of Rae, with its own Card"), and only
    once KIFF's draft says `trying` does tafs cloud open the trial (`PUT /v1/tafs/{id}/trial`
    `{n, draft}`, as the session), whose sheet opens. A KIFF that answers anything else starts no
    trial, and says so. Without a key, Start trying stays disabled with why (`ApprovalNotice`).
- **The trial** (`TrialScreen`), in a sheet named "Rae · v3": its bar on the amber (Trying version
  3, "not live"), Go back with its ask, whose yes shows the can't-approve line (the trial's KIFF
  draft is ended with the device's key first, `Act::EndTrial`); Install, **with a security key**:
  asked first ("Install version 3?"), then KIFF's draft is read and, not confirmed live yet, the
  key approves its confirm (`{"version":…}`, "Install version 3 of Rae, with its Card"), then
  tafs cloud installs it as the session (`POST /v1/tafs/{id}/trial/install`), the version going
  live and the trial ending; without a key, disabled with why; what the trial is, in tafs cloud's words (what's new, what to try it
  with, that the live taf keeps working, when it ends unused); and the trial's own chat
  (`/v1/tafs/{id}/trial/conversation`, `/v1/tafs/{id}/trial/messages`, the words alone, no
  files), kept apart from the taf's.
- `--versions` (with `--trial`) reads them from files (synthetic: no contract at the pin);
  `--page versions:<key>` or `trial:<key>` opens the sheet, and `try:<key>` the versions with the
  first Try it open over them; `--trial-draft` gives KIFF's draft its Card is, from a file
  (synthetic, `crates/tafs-app/fixtures/trial-draft.json`). CI's `taf-versions`,
  `taf-versions-try`, `taf-versions-trial` and `taf-trial`.

## Not yet

- **The sample's Chat**, there, dimmed. A tafs cloud taf's page opens on its chat ([chat.md](chat.md)).
- **On the Setup tab:** the sample's Setup (there, dimmed).
- **In the trial's sheet:** its guide stays above the chat once messages are in (the iPad shows it
  before the first only), and the chat's box is under it, a scroll down.
- Remove, and folding the head while scrolling.
