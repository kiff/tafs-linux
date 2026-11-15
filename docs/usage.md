# Plan & usage

The iPad's Plan & usage (`UsageView`), beside the sidebar: the account's balance, then the credits
its tafs used. Every line is tafs cloud's words, as it writes them: the screen adds nothing but
their order. The reads are `tafs-core/src/usage.rs`, tested there; the screen is
`crates/tafs-app/src/usage.rs`.

## The screen

It reads each time it opens: until then a panel with a spinner, at the screen's full width.

- **The balance** (`BalancePanel`), from `GET /v1/usage/balance`: tafs cloud's `words` (17
  semibold), a bar at `meter_pct` (green, amber when `warning`, red when `waiting`), the `detail`
  (14, soft) and the `warning` (14 semibold, in the state's colour). The panel's edge is red at
  50% while waiting, amber at 50% from the warning. When it couldn't be read, why, in red.
- **Credits** (`CreditsPanel`), from `GET /v1/usage/credits`, unless the balance says
  `credits_panel: false` (no plan and no run lately): CREDITS, Last period and This period, then
  the period's `words` (17 semibold). Choosing Last period reads `?period=previous` the first
  time; until it's read a spinner, then its lines, why it couldn't be read, or "The last
  period's credits couldn't be read." Under the panel:
  - each taf that used credits, most first, as tafs cloud lists them. Signed in with KIFF, a
    taf's row opens its runs; the row for tafs removed since, and the sample's, don't;
  - Coach past its fair use, its own row;
  - Not counted, with each reason (14, soft);
  - the note (13, soft), last.
- **A taf's runs** (`TafCreditsView`), from `GET /v1/usage/credits/tafs/{id}` for the period
  shown: its name as the title and Plan & usage to go back, the taf's `words`, then its runs,
  newest first, each tafs cloud's words (semibold when counted, soft when not) and when it ended
  in this computer's time; More runs reads the next page (`after`) while there is one, none
  twice. When nothing could be read, why, and Try again.

A read that fails keeps what was read before and says why: tafs cloud's own `words` when it
sends them, else "{what} couldn't be read: {why}". A tafs cloud without a read (404) shows
nothing for it. Each read goes to tafs cloud only, as the session (`may_send_to`).

The sample shows the iPad's sample balance (Solo, 48% used) and credits, both periods; its rows
don't open. Signed out, or with `--home` (no session), the screen is empty. `--page usage` opens
it, as `usage.png`.

## Not yet

- **Raise the limit**, which signs with this computer's key, and the read after it.
- **Buying** packs and Solo: the iPad's are the App Store's, and Linux has no store yet.
- **KIFF's usage read**: who bills the plan (`billingPanel`), Ask an admin and its answers
  (`askPanel`), and members' requests for whoever manages billing (`requestsPanel`).
- **A run's request** (`KnobRequestDetail`): a run's row doesn't open yet.
- **Refreshing in place**: the iPad pulls to refresh; here the screen reads again when it opens,
  and a taf's runs have Try again.
