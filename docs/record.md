# The Record

The iPad's Record (`RecordView`), beside the sidebar: every call each taf made, by day, newest
first. What it says is `tafs-core/src/record.rs`, tested there; the page is
`crates/tafs-app/src/record.rs`.

## The screen

- **"Record"** at 34 bold, and "Everything your tafs did, were refused, or asked you. Newest
  first." (14, soft).
- **The tafs' chips:** "All tafs", then one a taf with its colour's dot, in the tafs' own order
  (the sample's as it made them, else tafs cloud's). The chosen chip inverts: the ink's fill, the
  background's colour for its words.
- **The filters:** All, Money moved (an amount, done or approved), Stopped (refused, not done,
  revoked), Waiting (held, sending, unknown).
- **Each day**, newest first: its heading ("TODAY · NOV 15", "YESTERDAY · NOV 14", else
  "FRIDAY · NOV 13"), monospaced 11 in capitals, with the money that moved that day ("€640
  spent", a sum for each currency). The day and month are the app's language's own
  (`.dateTime.day().month(.abbreviated)`, `weekday(.wide)`), in the account's time zone
  ([app.md](app.md#time)). A line's time has the zone's short name after it when the account's
  clock isn't this computer's ("14:05 JST").
- **Each line** (`RecordRow`), on the panel with corners of 14: a dot in its kind's colour (green
  done, amber waiting, red stopped), the taf's colour and "Chargebacks · Accept dispute on #3342
  · €35" (14 semibold), its time (12, monospaced), and under it what became of it (13, soft):
  what it acted on, then who decided and what the tool did when KIFF says, else its own line. A
  held line KIFF no longer holds says "no longer waiting · nothing sent". A screen reader reads
  the line as one.
- **A day's reads** fold into one line, "· 3 reads of data, within its Card · Show" in green,
  with All; Hide folds them again.
- **Nothing:** "Nothing yet." with All and every taf, else "Nothing here."

## KIFF's activity

Signed in, the app reads `GET /v1/me/activity?limit=100` at KIFF as the session (`activity`),
with the account, Home and the held calls. Each call by a taf Home lists (by the KIFF agent it
was made as) is one line (`KIFFMapping.record`):

- **Allowed:** what the tool did (`executed`): done within its Card, or approved by you then
  done; refused by the app, saying it still counts against today's total (money, an amount, or
  why); not sent, nothing reached the app; sent, waiting for its answer; the answer lost; or not
  recorded. A connection that refused it before sending says so.
- **Held** (`owner_required`, `approval_required`): Waiting for you.
- **Anything else:** refused, "Not sent · " and KIFF's reason when it gives one.
- **Its title:** the tool's title as KIFF gives it, else the action in words; with its amount
  after it, as money in its currency, or the value and its unit as written.
- **Who decided** (once KIFF sends an answer or a receipt): approved or refused by you, by "an
  admin" (KIFF's role for whoever answered), refused by its Card, Card issued; then done, nothing
  sent, not done, nothing sent yet, sent with no answer yet, or result unknown.
- A call by an agent the app doesn't list isn't shown. An activity read that fails leaves the
  Record empty; a 401 signs this computer out ([sign-in](sign-in.md)).

## The sample

`--sample`'s Record is the iPad's, as `record.png`: each taf started ("Card issued by You"), then
the scripts' first two rounds, newest first: what each Card allowed ("€40, within the Card",
"Within the Card") and the two calls it held ("Held: Pay Atlas Freight", "Over €5,000 each").

## Not yet

- **A line's receipt** (`ReceiptView`): a line opens nothing yet.
- **Search** (`RecordSearchBox`, `RecordResults`, a case's sheet) and **Export receipts**.
- **What a line acted on** (`naming`, the runs' call refs): KIFF's hash alone names nothing.
- **"Signed on this computer"** and the receipt's signed mark under a line.
- **Which vendor refused** a call, by its connection (`vendors`).
