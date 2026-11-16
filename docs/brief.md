# Your brief

The iPad's "Your brief" (`BriefView`, Missions design v2.1 §7), beside the sidebar in Needs
you's place once tafs cloud sends one. What it reads is `tafs-core/src/brief.rs`, as the iPad's
`TafsCloud+Brief.swift` and its model (`AppModel`: `loadBrief`, `briefDecisions`,
`needsYouBeyondBrief`, `destination(of:answer:)`, `laterBrief`), tested there; the page is
`crates/tafs-app/src/brief.rs`.

## When it shows

Signed in with KIFF, `GET /v1/brief` is read with the account's other reads, at tafs cloud as the
session. Its answer is the switch: a 404, an error, an answer that isn't an object, or a brief
with nothing in it (no decision, nothing done, no next move) keeps Needs you exactly as today.
Once there is one, Needs you's row in the sidebar says tafs cloud's name for it (`words.tab`,
"Your brief", `needsYouTabTitle`) with Needs you's count, and opens the brief. The sample has no
brief: the iPad's is for its screenshots only.

## How it's read

As leniently as the iPad: the brief fails only when it isn't an object; a part the app can't
read is left out, never the brief with it.

- **Decisions** (`question`, `answers` with `recommended` and `decline`, `why`, `meanwhile`,
  `deadline`, `urgent`, `later`, `cant_wait` or the route's `now_why`, `note`, `ref`): one with
  no id, no question or no answer is left out. Coach's pick is drawn as recommended only with its
  `why`.
- **`more`** only with a count above 0 and its words; **`done`** groups only with a case (a case
  needs a name or its words); **`next_move`** only with its words, Start and Dismiss; **`rest`**
  only with its words; **`credits`** its `words`.
- **`labels`**: each with an id and a name; `waiting` a count (a yes from an earlier draft reads
  as one).
- **The headings** (`words`): tafs cloud's when sent; one it didn't send, or sent blank, is the
  shared catalog's (`brief.*`: "Your brief", "Decisions", "Done since the last brief", "Next
  move", "Coach recommends", "Meanwhile:", "Evidence", "Urgent"), so a heading is never blank.
  `when` and `all` have no shared word: tafs cloud's or nothing.

## The screen

- **The title** at 34 bold; when it was written ("Morning brief · 08:00 · next at 18:00", 13
  soft, after a clock); then **the label chips** (`LabelChips`): All (tafs cloud's word, else
  "All", `home.labels_all`), two labels with their counts and an amber dot while one of their tafs
  waits, then "+N", which shows them all. The chosen chip is inverted; tapped again, it's All. A
  chip reads the brief again for that label (`GET /v1/brief?label=`), and what was put aside here
  comes back; a read that fails or finds nothing is Needs you, as on the iPad.
- **Two columns** on a wide pane, decisions beside Done, the next move and the rest under Done;
  one column when the pane is under 640 pixels, in the iPhone's order (decisions, next move, Done, the rest).
- **Each decision** (`DecisionCard`): its taf's avatar and name (found by tafs cloud's id), and at
  the end the red Urgent badge with its bolt, or the deadline (12 monospaced, amber); the
  question (17 semibold); Coach's pick and its why on Coach's wash; "Meanwhile:" in bold, then
  what happens; why it can't wait, in red on red at 8%; its answers; its note (12 dim). An urgent
  decision has a red edge and never a Later, whatever was sent. Under the decisions, "1 more
  decision" (13 soft) when tafs cloud counts more than it lists.
- **The answers:** the recommended one filled with the spark, the others outlined (a decline in
  red), and Later beside them.
- **Nothing left to decide:** board 3 (a calm sun over a clear page, in shapes:
  [illustrations](app.md#illustrations)) where the decisions go.
- **The next move:** an amber-edged panel, NEXT MOVE with the spark, its words, Start (amber) and
  Dismiss. Start opens Coach with the move in its box, for whoever may use Coach; either puts it
  away.
- **Done:** each mission's cases (a 7-point dot, green once closed, amber while waiting, else
  dim; the name 14 semibold, its words 13, green once closed) with what it's worth (15
  semibold), a line between missions.
- **The rest:** tafs cloud's line (14 ink) and, whenever Needs you holds more than the brief
  shows, Needs you's own count ("2 need you.", `needs_you.count`), matched item by item
  (`hold:<id>`, `step:<handoff>`) and counting what the brief can't carry (Coach's waits, Card
  unit requests, "Did this happen?"). It opens Needs you over the brief, with the way back.
- **The credits line**, 12 dim, centred, at the foot.

## Answering

A recommendation is shown, never acted on, and nothing is signed from the brief: an answer opens
where its own answer is given (`destination(of:answer:)`).

- **A held call or a step** (`ref.kind` `hold`, `step`): answered on the iPad with a signature
  (KIFF's card behind Face ID; a step's Done or Skip it). This computer can't, so its buttons are
  drawn as the iPad's, and tapped, `ApprovalGate`'s words go under the answers: "This device
  can't approve. Approve on another device signed in to your account."
- **A taf whose scheduled check keeps failing** (`check`): its one answer, "Open the taf", opens
  that taf's page, found by tafs cloud's id (`ref.taf`). A taf the app doesn't list opens Needs
  you.
- **Anything else** (a connection, a proposal, a ref with no id): Needs you.
- **Later** (`POST /v1/brief/decisions/{id}/later`, the id as written, ":" kept): the card goes
  only once tafs cloud has moved it; else it stays, to answer or try again. It signs nothing.

## Not yet

- Done's **Evidence ›**, which opens the mission's log (`MissionLogView`).
- The brief's alert (`brief_kind`), and reading it again when one comes: this computer has no
  push.

## Screenshots

`--brief answer.json` (with `--home`) reads the brief from a file, tafs cloud's answer or a
fixture's `{about, answer}`: every read is it, a label's too, and Later sends nothing.
`--page brief-answer:<decision id>` opens it with that decision's signed answer tapped. CI's
`brief`, `brief-answer` and `brief-clear` are `crates/tafs-app/fixtures/brief.json` and
`brief-clear.json` over `home-missions.json`'s four tafs.
