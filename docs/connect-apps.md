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
- **Each row** (`cells`): the name (16 semibold, one line, soft for an app not
  set up yet) and what it is (13, soft, two lines); how it connects ("Sign in", "With a key", by
  `method`, else `how`); whose it is, in tafs cloud's words; where it stands: "Connected" in green
  or "Sign in again" in amber, with "Used by Refunds and Chargebacks" under it when tafs cloud
  names the tafs, or tafs cloud's words for an app to set up. At the end, a ✓ for a connected
  app; else Connect, or Sign in again, as wide as its words; nothing for an app to set up.
- **Over Discover**, "Apps connect for a taf. Make one first…" when no taf can have the apps.
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
`--page connect-apps` opens it, as `connect-apps.png`.

## Not yet

- **Connecting**: Connect stays dimmed. It needs a taf the person manages (`connectableTafs`), and
  who manages a taf isn't read yet; the approval sheet (`ConnectApprovalSheet`), signed with this
  computer's key, and the vendor's sign-in come with it, with "Which taf?".
- **A connected app's detail** (`AppConnectionsView`, `ConnectionDetailView`): what it can read
  and do, which tafs use it, Disconnect, and removed tafs' connections. A connected row doesn't
  open yet, so it has no chevron.
- **Logos** (`logoURL`), read from tafs cloud only: none is drawn yet, as on the iPad for an app
  tafs cloud sends no logo for.
- **This computer's read of the connections**, which names the tafs using an app when tafs cloud
  sends no `used_by`, and the signed grants read that lets Clear end removed tafs' connections.
- **A narrow window's one-line rows** (the iPhone's `second` line): Linux always draws the table.
