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

## Search (signed in)

The iPad's `RecordSearchBox`, `RecordResults`, `RecordHitRow` and `RecordCaseSheet`
(`RecordSearchViews.swift`) with `TafsCloud+Record.swift`'s search and case, on Linux:
`tafs-core/src/record_search.rs` (tested against the pinned `taf-activity.json` entry) and
`crates/tafs-app/src/record_search.rs`.

- **The box**, under the lead, signed in with KIFF: tafs cloud's words as its placeholder (read once,
  `GET /v1/record?q=&limit=1`'s `hint`; the app's "Search by order, invoice, amount or action"
  until they come). What's typed is searched 0.4 s after the last key (`recordTypingPause`), Enter
  searches at once, the same words aren't searched again, and an emptied box shows the Record at
  once.
- **A search** (`GET /v1/record?q=`, what's typed percent-encoded but its letters and digits)
  stands in for the chips and the days. Its pages, in tafs cloud's groups (or all as one untitled
  group): each group's title (12 semibold, dim), then each call under its taf's name (11, mono,
  upper case), drawn as its taf's Activity draws it (`SourceActivityRow`), and what it's about
  (amber, with ›), which opens its case. "Nothing matched", in tafs cloud's words, when every page
  is empty; its error in red; More while a next page waits (a cursor tafs cloud refuses, 409,
  searches again from the top); its end words after the last.
- **A case** (`GET /v1/record/cases/{about}`, one escaped segment), over the window: what it's
  about (24 bold), the step that didn't go through in red, in tafs cloud's words, then every call
  about it, oldest first; Done at the top.

Read as the iPad's decoders: a hit needs its taf's id and an Activity entry that reads; the
words are forgiven; a blank `about` opens no case. `--home shared/contracts/home.json
--record-search crates/tafs-app/fixtures/record-search.json --page record-search:1042` (synthetic)
draws a search, CI's `record-search`; nothing is sent.

A call from a taf that isn't on Home (tafs cloud's id isn't one of Home's tafs) is drawn as the
iPad's plain panel: its title (15 semibold), detail (13, soft), amount (14) and time (12, dim), with no
next step to take from here. In the screenshot, Sam's "Send a reply" is one.

## Exporting receipts (signed in)

The iPad's `ExportReceiptsRow` (`RecordSearchViews.swift`), with `exportReceipts` from
`TafsCloud+Record.swift` and `AppModel`, is ported to `tafs-core/src/receipts.rs` and
`crates/tafs-app/src/receipts.rs`. It sits under the box, for whoever sees every taf's Record
(`seesEveryTaf`): at least one of their tafs is on tafs cloud, and they oversee or manage each.
A taf whose access wasn't read counts as not seen. tafs cloud still decides, with a 403 in its
words.

- **Export receipts** (15 semibold, amber) opens Last 7 days, Last 30 days and Last 90 days. Each
  range ends today and counts today, in the account's calendar days.
- **One of them** reads `GET /v1/record/receipts?from=&to=`. While it reads, "Preparing receipts…"
  shows, dim, with a spinner. Up to 32 MB is read, and a longer answer is refused rather than
  saved cut short.
- **The files:** `receipts-{from}-to-{to}.json` is tafs cloud's answer byte for byte (KIFF's
  receipts as issued), and `.csv` is its `csv`. "{count} calls have no receipt yet." follows
  when some are `missing`. "They couldn't be exported. {why}" shows in red when the read fails.
- **Where Linux differs:** the iPad hands the two files to its share sheet. Linux asks for a
  folder and saves both files there. Closing the dialog saves nothing, as closing the share
  sheet does. A day in the answer that isn't YYYY-MM-DD never names a file: the asked day does.
  If saving fails, the system's reason follows "They couldn't be exported.", as the iPad shows
  its own. The files are written with mode 0600, readable by the person alone. Saving the same
  days in the same folder replaces the earlier files.

To draw the row open (CI's `record-export`; nothing is sent or saved), run with:
- `--home shared/contracts/home.json`;
- `--access crates/tafs-app/fixtures/access.json` (Rae, overseen);
- `--record-search crates/tafs-app/fixtures/record-search.json`;
- `--receipts crates/tafs-app/fixtures/receipts.json`;
- `--page record-export`.

All of these fixtures are synthetic.

## Checking a receipt on this computer

`tafs-core/src/receipt_check.rs` is the iPad's `KIFF/ReceiptCheck.swift` (kiff/tafs#167): a KIFF
receipt (`GET /v1/receipts/{trace_id}`) is checked here as KIFF's own verification does.
- **The signed digest** is keccak256 of the ASCII hex of the SHA-256 of `causal_chain`. The chain
  is taken from the answer byte for byte as KIFF wrote it (`raw_value`), never decoded and
  written again: Go escapes `<`, `>` and `&` and writes `1e+21`, which a re-encoding would change.
- **The signer** is recovered from the 65-byte signature (secp256k1, `k256`) as an Ethereum
  address. It must be the one the receipt declares and KIFF's signer
  (`0x2f297affd7b62ef712a1e3158421ccb4bf2efcbe`).
  - A high S is recovered too, as libsecp256k1 does, so a signature in the upper half of the
    curve order still checks.
- **A receipt that names a top-level key twice is refused as unreadable.** A decoder keeps the
  last copy of a key, and the chain's scan the first, so two `causal_chain` or `attestation` keys
  could show one set of steps while another is checked. Refusing it keeps what's shown and what's
  checked the same bytes.
- **The outcome:** no attestation yet is "not signed yet", never a failure. Anything else that
  doesn't check out says why, in the shared `receipt.check.*` words.

Nothing draws it yet. `ReceiptView`, a Record line's receipt, comes next and uses it.
