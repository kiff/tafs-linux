# Plan & usage

The iPad's Plan & usage (`UsageView`), beside the sidebar: the account's balance, who bills the
plan and members' requests, then the credits its tafs used. Every line is tafs cloud's or KIFF's
words, as they write them, or the lines both apps share: the screen adds nothing but their order. The reads are `tafs-core/src/usage.rs`, tested there; the screen is
`crates/tafs-app/src/usage.rs`.

## The screen

It reads each time it opens: until then a panel with a spinner, at the screen's full width.

- **The balance** (`BalancePanel`), from `GET /v1/usage/balance`: tafs cloud's `words` (17
  semibold), a bar at `meter_pct` (green, amber when `warning`, red when `waiting`), the `detail`
  (14, soft) and the `warning` (14 semibold, in the state's colour). The panel's edge is red at
  50% while waiting, amber at 50% from the warning. When it couldn't be read, why, in red.
  Signed in with KIFF, tafs cloud's Raise offer (`raise.words`) is under it, dimmed.
- **KIFF's plan** (`GET /v1/me/usage` at KIFF), signed in with KIFF, in the iPad's order:
  - who bills it (`billingPanel`): "Billed by Apple", "Billed by Google Play", "Billed by KIFF"
    or "Managed by your company's agreement", none when no plan is billed; after a move from
    Solo in the App Store to Company, the line that Solo still renews with Apple until its long
    date. Manage and Cancel Solo open Apple's subscriptions page, which only an Apple device
    has: Linux shows the lines alone;
  - for someone without `billing.manage` (`askPanel`): their open request ("You asked an admin
    to raise the spend limit. An admin answers.") with Withdraw; else their last answer (raised,
    pack added, declined with its note, or ended with the period) and Ask an admin, where the
    plan lets them ask (a raise on Company, a pack on Solo while packs are on sale), in the
    primary style while the balance waits;
  - for someone who manages billing (`requestsPanel`): REQUESTS in the amber, each open request
    ("Someone asks to raise the spend limit.", "Someone asks for {pack}.") with its reason, Raise
    the limit where tafs cloud offers it, and Decline.

  The buttons are there, dimmed, until asking and answering are ported. When KIFF's read fails,
  why, once, in red, after the credits.
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
  (in the account's time zone); More runs reads the next page (`after`) while there
  is one, none twice. When nothing could be read, why, and Try again.

A read that fails keeps what was read before and says why: tafs cloud's own `words` when it
sends them, else "{what} couldn't be read: {why}". A tafs cloud without a read (404) shows
nothing for it. Each read goes to tafs cloud only, as the session (`may_send_to`).

The sample shows the iPad's sample balance (Solo, 48% used) and credits, both periods; its rows
don't open, and KIFF's panels aren't drawn. Signed out, or with `--home` (no session), the
screen is empty. `--page usage` opens it, as `usage.png`. `--usage <file>` with `--home` draws
it signed in with KIFF from a file of synthetic answers (`balance`, `credits`, `usage`,
`manages_billing`): `crates/tafs-app/fixtures/usage-kiff.json` (someone who manages billing) and
`usage-kiff-member.json` (a member), CI's `usage-kiff` and `usage-kiff-member`.

## Not yet

- **Raise the limit**, which signs with this computer's key, and the read after it; declining
  a request, signed too.
- **Asking an admin** (`AskAdminView`) and withdrawing a request.
- **Buying** packs and Solo, and approving a member's pack (which buys it): the iPad's are the
  App Store's, and Linux has no store yet.
- **A run's request** (`KnobRequestDetail`): a run's row doesn't open yet.
- **Refreshing in place**: the iPad pulls to refresh; here the screen reads again when it opens,
  and a taf's runs have Try again.
