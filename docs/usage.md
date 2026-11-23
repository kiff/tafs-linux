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
  Signed in with KIFF, tafs cloud's Raise offer (`raise.words`) goes under it, behind
  `ApprovalGate` as on the iPad: while this computer can't approve, the gate's line ("This device
  can't approve. Approve on another device signed in to your account.") is drawn in Raise's
  place, and nothing is sent. The same for Raise the limit on a member's request.
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

  When KIFF's read fails, why, once, in red, after the credits.

## Asking, withdrawing, answering

The iPad doesn't sign a member's asks, so they work here, at KIFF as the session:
- **Ask an admin** opens `AskAdminView`: Cancel, "Ask an admin", what asking does (for a raise or
  a pack), WHICH PACK on Solo (the packs on sale, the First month pack first while tafs cloud
  says it can be bought), WHY (OPTIONAL) with "What the credits are for", and Ask. Ask works once
  a pack is chosen where one is asked for, and says "Asking…" while it's sent.
  - It sends `POST /v1/me/pack-requests` with the reason (its first 500 characters, trimmed),
    and `kind: raise` or the pack's id.
  - Asked, the sheet closes and the screen reads again.
  - Not asked, "The request didn't go. {why}" in red in the sheet, and in the plan's message on
    the screen.
- **Withdraw** sends `POST /v1/me/pack-requests/{id}/withdraw`, then the screen reads again.
  When it fails, the plan's message says "The request wasn't withdrawn. {why}".

The plan's message (`planMessage`) is a panel after the balance, in the soft colour.

An admin's answers are signed (`docs/approval.md` › A view-only computer), and nothing is sent:
- **Raise the limit** on a member's raise request: its tap says this computer can't approve, in
  the plan's message's place.
- **Decline** opens `DeclineRequestView`: Cancel, "Decline", its question ("Decline the request
  to raise the spend limit?", "Decline the request for {pack}?"), NOTE (OPTIONAL) with "Why, for
  them", and Decline in red. Its tap says this computer can't approve, above the button where the
  iPad says why it wasn't declined, and the sheet stays open.

Each send is an unsigned call (`tafs_core::signed::Unsigned`), which can't be one KIFF takes only
signed. A screenshot's account (`--usage`) sends nothing. `--page usage-ask` opens Ask an
admin's sheet (CI's `usage-ask`, the member's file), and `--page usage-decline` opens the first
request's Decline with Decline tapped (`usage-decline`).
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
  is one, none twice. When nothing could be read, why, and Try again. Each taf's runs are kept
  as last read while Plan & usage is open (`tafCredits`, by taf and period): opened again, they're
  read again, and where that read fails, what the last one gave stays, with why under its runs.
- **A run's own page** (`CreditsRunRow`): signed in with KIFF, a run tafs cloud says `opens`
  is a link, with a chevron, to the run's own page (`KnobRequestDetail`, [the taf's
  page](taf.md)), read as the session. Try the reply now is there for whoever manages the taf,
  and Ask Coach for someone who may use Coach and manages it (`offersAskCoach`), opening Coach as
  it was (`openCoach`). A run that doesn't open, and the screenshots' file, stay plain lines.

Pulled past its top (`.refreshable`: a touchpad's or a touch screen's pull, as GTK reports it),
or on Ctrl+R or F5 while the focus is in it, the screen reads again in place (`refreshUsage`): the balance, the credits and
KIFF's usage, and the period before once it was asked for. A taf's runs read their credits and
newest runs again the same way, and keep Try again while nothing is read, for a mouse. What was
read stays in sight meanwhile, and a pull while a read is on its way asks nothing more.

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

- **Buying** packs and Solo, and approving a member's pack (which buys it): the iPad's are the
  App Store's. They're held for the owner's money question.
