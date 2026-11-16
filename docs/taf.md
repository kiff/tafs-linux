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
(CI's `taf-home`).

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

## Not yet

- **The page opens on Activity**; the iPad opens on Chat. Chat and Setup are there, dimmed, until
  each is ported.
- **On the Card tab:** Connect again (Coach), the halt row,
  Versions, Usage; Revoke itself (dimmed).
- **Above a tafs cloud taf's Activity:** its doors' knocks, Card unit requests; and the next step's buttons (Connect it
  again, Ask Coach), shown as their words for now; a call's receipt; "All apps" for a selection.
- **The line under the head:** its version and who it's shared with (`headerLine`, `ShareLine`).
- **In its work:** Answer within (`PromiseRow`, whoever manages the taf), Send again, and a step
  it handed you as its card (`YourStepCard`): its line for now.
- **The side column** on a wide window, Remove, and folding the head while scrolling.
