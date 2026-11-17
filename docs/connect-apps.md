# Connect apps

The iPad's Connect apps (`ConnectAppsView`), beside the sidebar: what tafs cloud offers to connect,
the person's own first, in tafs cloud's order and words. The lines are
`tafs-core/src/connect.rs`, tested there; the screen is `crates/tafs-app/src/connect.rs`.

## The screen

- **The title** and, beside it, the search: as the person types, a moment after the last key, the
  first page is read again for what was typed (`GET /v1/connect/apps?q=…`). Only the newest
  search's answer is kept, and a new search drops the last one's pages at once.
- **Yours** (`ConnectRow.split`): the apps that are connected, signed in, or to sign in again.
  **Discover**: the rest, without an app already under Yours in another of its catalog forms
  (Stripe connected through a provider's entry leaves out `stripe@3`, by `keys`: the entry's last
  part without its version, and the vendor, folded).
- **The table** (`ConnectColumns`), with its header row in each group: App, How it connects, Whose,
  Status. The App column has its room first; How, Whose and Status are 150, 130 and 230 wide and
  give way together, in proportion, down to 96, 84 and 180, so App keeps its least of 200.
- **A narrow window** (the iPhone's rows, below the table's least: 596 with its spacing, inside
  the page's margins): no header row; each app is its name and what it is, then one line (13, dim,
  amber while it needs signing in again): tafs cloud's words for an app to set up, else tafs
  cloud's `line`, else how it connects, whose it is, where it stands and who uses it, joined by
  " · " (`second`). Its ✓ or Connect at the end, as on the table. The window crossing that width
  draws the list again in the other form.
- **Used by**: tafs cloud's `used_by`, else the tafs this computer's connections name
  (`connectionGroups(of:)`); signed in with KIFF it has none (see below), the sample's screenshot
  has the reference set's.
- **Each row** (`cells`): its logo (24) where tafs cloud serves one (`logoURL`: only
  `/v1/logos/` and a 32-character hex key, read once a launch, without the session, at most 1 MB;
  nothing while it loads or when it fails), the name (16 semibold, one line, soft for an app not
  set up yet) and what it is (13, soft, two lines); how it connects ("Sign in", "With a key", by
  `method`, else `how`); whose it is, in tafs cloud's words; where it stands: "Connected" in green
  or "Sign in again" in amber, with "Used by Refunds and Chargebacks" under it when tafs cloud
  names the tafs, or tafs cloud's words for an app to set up. At the end, a ✓ for a connected
  app; else Connect, or Sign in again, as wide as its words, or Ask Coach where tafs cloud says no
  taf needs it (`needed_by` empty); nothing for an app to set up. An app with connections opens
  them, with a chevron (`opensConnections`): a connected one always, and one this computer holds
  a connection to that isn't connected (a sign-in the person declined, `listed`), its Connect
  still beside it, so its Remove access is in reach.
- **Over Discover**, "Apps connect for a taf. Make one first…" when no taf can have the apps
  (`connectableTafs`: a tafs cloud taf this person manages, by KIFF's access read). The sample has
  none.
- **More apps** while tafs cloud has a next page (`next_cursor`): it adds the next page's apps,
  none twice, for the same search and provider.
- **Not listed? Ask Coach** when tafs cloud says Coach can draft an app nobody listed (`by_key`):
  it opens Coach.
- Until the first page is read, a spinner; when it can't be read, why, in red ("The apps couldn't
  be read: …").

Read as strictly as the iPad's decoder: an app without its `entry`, or with a field of the wrong
kind, fails the whole read, as on the iPad; tafs cloud's words (`name`, `subtitle`, `line`,
`whose`, `method`, `words`) are trimmed and read as none when blank. `used_by` is read as tafs
cloud's `[{taf, name}]`; anything else reads as none sent.

The sample shows the iPad reference set's apps (`ios/Tafs/Samples/connect-apps.json` at the pin,
synced into `shared/`) in a screenshot, as the iPad's sample does, and nothing otherwise.
`--page connect-apps` opens it, as `connect-apps.png`; with `--size 900x800`, CI's
`connect-apps-narrow` draws the narrow window's rows.

## Connecting

Connect asks whom it's for (`ConnectRow.connectFor`): never a guess. Where tafs cloud names the
tafs that need the app (`needed_by`), only those, and none means Ask Coach (it opens Coach);
without it, every taf on tafs cloud this person manages. "Which taf is this for?" lists them,
with "{app} is connected for one taf…", even when there's one; Cancel closes it. No taf at all:
Connect is off, and "Make one first" says why.

Picking a taf starts the connection at tafs cloud as the session (`startConnect`,
`POST /v1/connect/apps/{entry}` with `{"taf": id}`, the entry escaped as the iPad's path, with
this computer's signed grants read in `Planque-Device-Signature`, as Connect apps' reads send it).
It approves nothing: planque makes the request for the taf's KIFF agent. Every Connect is off
while it starts. tafs cloud's `not_needed` shows its own words; any other refusal "{vendor}
couldn't be connected: …", in red over the list.

Then its approval opens as a sheet (`ConnectApprovalSheet`): "Connect {vendor}", Cancel. With the
request, its words from the connection server, "This request doesn't match what it shows…" when
its digest isn't checked, then for a key where it goes ("The key goes from this phone…"), and in
the approve button's place, `ApprovalGate`'s line: this computer can't approve. The button is the
iPad's `ApprovalGate`'s own, so the line stands where it would be. Without the request (signed in
with KIFF, always: see below), the sheet says "Reading the request…", as the iPad does until its
read comes.

## An app's connections

`AppConnectionsView`, over the list with Back: "Your {app} connections", then:

- "Start from Connected apps on this computer first.": this computer doesn't read the account's
  connections (`readsConnections`: set up with planque, which is signed, and this computer never
  is). Signed in with KIFF that's all there is.
- A section per taf the connections were made for, those needing attention first (a sign-in that
  waits, one its vendor signed out), then by name (`connectionGroups`). A connection planque draws
  (`rowFromPlanque`: its health and actions) is its name, "Used by …", its health (dim, amber
  when not ok) and a misplaced line (`misplaced`), and opens its detail; any other is its name,
  planque's status words or "Waiting for the sign-in at …", the misplaced line, how to fix it (a
  provider's connection says to remove and connect again; else Replace the key, which opens the
  key's sheet, or Sign in again), and Remove access, or Cancel the request / Cancel while it
  waits.
- From removed tafs (`removed_tafs`): each with its taf's name as planque keeps it, planque's
  words, and "Kept: …'s Card uses it"; Clear these connections where tafs cloud lists one with
  nobody using it (`clearable`), asking "Clear 2 connections from removed tafs?" first.
- Other connections, whose taf this computer can't name.
- The footer, "Removing access takes its tools from that taf only…", to whoever manages Cards
  where a connection here is a taf's.

`ConnectionDetailView`: the connection's name, its health, "Used by …", and the actions planque
offers that the app knows (`known`), Remove in red asking once more in planque's words with what
stops.

Removing access, cancelling, clearing and signing in again are planque's, signed behind Face ID
(`signed::Act::Disconnect`, `ClearRemoved`, `SignInAgain`, `at_planque`). Each asks the iPad's
question (`DisconnectWords`: naming the one taf it ends, what stops, and that a key is deleted
from tafs or a sign-in revoked at the vendor), and the tap that would sign shows this computer
can't approve, under the list or the row; nothing is sent, and this computer can't reach planque
at all (`session::may_send_to`). The sample, which signs nothing, does nothing on that tap.

The sample's screenshot draws the reference set's connections (the iPad's `Samples/grants.json`,
kept as `crates/tafs-app/fixtures/grants.json`): Stripe for Refunds and Chargebacks, Gmail for
Refunds. `--page connect-app` opens the app with the most connections, `connect-connection` its
first connection, `connect-remove` its Remove access question. Signed in with KIFF,
`--connect crates/tafs-app/fixtures/connect-kiff.json` (the contract's first page with Qonto
needed by Rae, and a start) with `--page connect-which-taf` or `connect-approve` draws "Which
taf?" and the approval; a fixture's read never sends.

## Not on Linux

- **planque's read of the connections**: the iPad reads them only once set up with planque, a
  signed act. So signed in with KIFF no connection is listed, the approval stays at "Reading the
  request…", and what needs one (Disconnect's question, Clear, Sign in again, a connection's
  detail) is reached only in the sample's screenshot. tafs cloud's `removed_tafs` and
  `misplaced` are read, and drawn where a connection is.
- **Approving, giving a key, and the receipt**: signed. The key's sheet in tafs cloud's words
  (`ConnectionSheet`), pasting a key, the Card signed with it, its receipt with board 5 (an app
  connected) and its Remove access come only after a signature, so not here.
- **KIFF's words for a connection's profile** (`ProfileWordsView`): read for a request this
  computer never has.
- **A connection's last failure** and **the other tafs whose Card uses it** (`lastFailure`,
  `alsoStops`): they need the Record's and the Cards' actions by tool, which this computer doesn't
  keep; the question names none.
