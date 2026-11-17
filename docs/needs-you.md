# Needs you

The iPad's Needs you (`NeedsYouView`) and its held call cards (`HeldCallCard`), beside the
sidebar. What they say is `tafs-core/src/needs_you.rs` and `held.rs`, tested there; the page is
`crates/tafs-app/src/needs_you.rs`.

## The screen

- **"Needs you"** at 34 bold, then each held call's card, 14 apart, the pane's full width with 16
  from each edge.
- **With nothing waiting:** board 6 (an empty tray under a calm check, in shapes:
  [illustrations](app.md#illustrations)), then a panel, "Nothing is waiting." (15 semibold) and "When a taf asks for
  something outside its Card, it shows up here and on your lock screen." (14, soft).

## A held call's card

An amber-edged panel (`Panel(stroke: amber 40%)`):

- **The taf:** its avatar (32) and name (14 semibold), else "A taf"; at the end, the time left,
  counted down every second ("9:37 left", monospaced 12, in the amber as text), then "expired".
- **The amount**, when the call has one, at 44 bold: in its own currency, or saying it has none.
- **What it asks** (16 semibold), and what it's for (14, soft) when the sample says.
- **Why it's held**, as an amber chip (12 medium on the amber at 14%).
- A screen reader reads the face as one sentence: "Rae asks: Refund a payment, €150. €150 is over
  the €100 limit for one call. It lapses at 13:10. Nothing is sent until you approve."
- **The taf says:** what the call is on and why, in the taf's own words, under their own heading
  on a soft wash: drawn verbatim, never as markup or a link, and never in place of KIFF's amount,
  rule or action. One plain line each, control characters and line breaks as spaces, cut to
  KIFF's caps (120 and 280) with "…".
- **Details:** what a call is on, one line an argument, monospaced and selectable, behind a
  disclosure: never the amount again, nor KIFF's own arguments.
- **Under it:**
  - **Expired:** "Expired. Nobody answered in time, so KIFF refused it and nothing was sent."
  - **Someone else answers** (the person can't answer the account's holds): "Nothing is sent until
    you approve." and "Someone who approves on this account answers this. Their answer shows here."
  - **A call from KIFF:** "Nothing is sent until you approve.", the iPad's line for answering in
    KIFF, and **Answer in KIFF**, which opens the call's page (`https://app.kiff.dev/needs-you/<id>?lang=…`).
    The iPad answers a call from KIFF on the device only with the session's key (`canAnswer`), and
    its third answer, Allow this from now on, comes with them. A session with no key, as this
    computer's, gets the line and Answer in KIFF alone, so neither is drawn here.
    `--held` reads KIFF's held calls beside `--home`; CI's `needs-you-kiff` draws one.
  - **The sample's:** "Nothing is sent until you approve.", Approve once (when approving once can
    pass it, else why not), its next steps and Refuse.

## KIFF's held calls

Signed in, with Home, the app reads `GET /v1/me/card-exceptions?status=held` at KIFF as the
session (`heldCalls`), and each call as the iPad shows it (`KIFFMapping.held`):

- **Its name** (`says`): its tool's title, with the action in words unless the title names it;
  a grant's own prefix and suffix never shown (`toolWords`).
- **Why** (in this order): KIFF's own words (`reason_text`); else the money over a limit, written
  as money everywhere else ("€150 is over the €100 limit for one call", "… would go over the €1,000
  total"); else a count ("20 calls a day: this would be the 21st", "Each call asks you first"),
  ordinals in the app's language; else KIFF's reason; else its outcome in words. Over both a
  one-call limit and a total, and not approvable once, the total is the one named.
- **Its amount:** a parameter KIFF declares a unit for, in that unit; else the parameter a breach
  sums, as a bare number.
- **When it lapses:** KIFF's time, else 10 minutes after it was held.
- **Its taf:** the one Home lists for the KIFF agent it's held for, else "A taf".
- Read as strictly as the iPad's decoder: a field it requires and an answer lacks fails the read,
  and no held calls are drawn; Home still is.

The held calls also feed the sidebar's NEEDS YOU and the Overview's, Needs you's count where
tafs cloud sends none, and Your tafs (a taf a call waits on is relevant now, and comes first).

## Steps and Coach's waits

Signed in, Home's (`your_steps`, `coach_waiting`, `coach_waiting_clear`) come before the held
calls, in the iPad's order:

- **A step a taf handed the person** (`YourStepCard`), in the amber: tafs cloud's title, the taf,
  what (16 semibold) and why (14, soft); the vendor's page when tafs cloud vouched for its link
  (an https link whose own host is tafs cloud's `link_host`; anything else is no button), which
  opens in the browser exactly as it was parsed and checked (never the string as sent), with that
  host under it; then Done and Skip it. The answer is signed by the device key
  (`answerHandoff`), so their tap shows the can't-approve line under them and sends nothing
  ([approval](approval.md)).
- **"Set all aside"** above two or more of Coach's waiting chats, when tafs cloud sends its words
  (amber, `SetAllAsideButton`). Tapped, tafs cloud's one ask: its title, its words, its yes (red)
  and its no (else Cancel). Yes sets every listed chat aside.
- **A Coach chat waiting on the person** (`CoachWaitingCard`): tafs cloud's line with Coach's
  mark; Open Coach, for someone who may use Coach, which opens Coach on that chat
  (`openCoachWaiting`); Set aside (soft), when tafs cloud sends its words. One without its chat
  or either line isn't listed.

Setting aside is tafs cloud's "not now", never an approval, and nothing is signed
(`setAsideCoach`): `POST /v1/home/coach-waiting/{session}/set-aside`, or
`/v1/home/coach-waiting/set-aside` for every one, as the session. This device's signed read of its
connections goes with it as `Planque-Device-Signature` when it has one, so tafs cloud can withdraw
a connection the chat asked for. A row goes once tafs cloud names it among the chats it ended
(`set_aside`), and Home is read again; a refusal keeps the row and says why, in tafs cloud's
words; a 401 signs this computer out. While it's on its way, the row's button and Set all aside
send nothing more. The iPad's swipe on the line isn't drawn: the button does the same.

"Nothing is waiting." shows only when no step, Coach chat or held call waits. Read as the iPad
reads Home: a step without its `handoff` fails Home; a Coach wait never does.
`--home crates/tafs-app/fixtures/home-waits.json` (synthetic) draws them, CI's `needs-you-waits`.

## Did this happen?

A money call whose outcome KIFF can't prove (`DidThisHappenCard`): its answer was lost, the tool
refused it with nothing showing it had no effect, or it's one of the first calls of a money
operation on probation. Read from KIFF (`GET /v1/me/did-this-happen`, `tafs-core/unproven.rs`)
only for someone who may answer holds (`holds.answer`), as `loadUnproven`; an answer that can't
be read shows none, and a 401 signs this computer out. They come after Coach's waits and before
the held calls, newest first, and "Nothing is waiting." shows only when none waits either.

Each is an amber-edged panel: "Did this happen?" (13 semibold, amber), KIFF's words for the call
as they are (15 semibold) or, before KIFF sent them, its title and amount; why KIFF asks, in its
words (13, soft). No and Yes are signed like an approval, and this computer approves nothing (its
key is in software), so where they'd be the card says "This device can't approve. Approve on
another device signed in to your account." (`ApprovalGate`'s `ApprovalNotReady`, 14 in ink;
`docs/approval.md`), as tafs-macos #76 does: it's answered on the phone. `--home shared/contracts/home.json --unproven
crates/tafs-app/fixtures/unproven.json` (synthetic) draws two, CI's `needs-you-happened`.

## Card unit requests

A taf asked for more of a Card's unit than the Card allows (`CardUnitExceptionPanel`), from tafs
cloud's Home (`card_unit_exceptions`). Every field is required, as the iPad's synthesized decoder
reads them, so one missing fails Home. They come after "Did this happen?", and "Nothing is
waiting." shows only when none waits either.

Each is a panel: tafs cloud's title (16 semibold), words (14, soft) and history (12, dim); its
Coach button (`action_title`) only for someone who may use Coach and manages the taf (`canDiscuss`:
the access read's `manage` for the taf's KIFF agent); then Dismiss in red. The Coach button opens Coach in a new
conversation with tafs cloud's `coach_text` in its box, for the person to edit and send; nothing
is sent (`discussCardUnit`; the taf Coach would improve isn't carried, as Linux's Coach opens on
none). Dismiss is signed like a refusal (`dismissCardUnit`, KIFF's `reject`), so its tap shows
the can't-approve line under it and sends nothing. The same panel stands on a taf's Activity and
Setup. `--home crates/tafs-app/fixtures/home-waits.json` (synthetic) draws one, CI's
`needs-you-waits`; `needs-you-set-all-aside` and `needs-you-dismiss` tap Set all aside and
Dismiss.

## Knocks

A knock at a taf's door waits for its owner too. Home's tafs carry their doors (`doors`:
`DoorLight`'s app, open, resting and knocks; a field missing or of the wrong kind reads as nothing
to show, and a door that isn't an object fails Home, as the iPad's keyed decoder does). After the
held calls, each taf whose doors' knocks add up to more than none has a row, in the tafs' own
order: an amber-edged panel with its avatar (32), name (14 semibold), "Someone knocked at its door.
Nothing was done." or "{count} knocks at its doors. Nothing was done." (14, soft), and a chevron.
It opens the taf's page, whose Activity opens first, where knocks are answered; a screen reader
hears "Opens {taf}, where you answer it". A knock also makes its taf relevant now in the sidebar
(`relevantNow`). "Nothing is waiting." shows only when none knocks either.
`--home crates/tafs-app/fixtures/home-waits.json` (synthetic) draws one, CI's `needs-you-waits`.

## Notifications for held calls

The iPad's hold notification (`HoldNotification.swift`: `words`, `fill`, `time`), and removing it
once the hold is gone (`PushDelegate`'s `removeAlerts(forHold:)`), are ported to
`tafs-core/src/hold_alert.rs` and `main.rs` › `watch_holds`. The iPad gets tafs cloud's push and
fills it in from KIFF. A computer gets no push, so while tafs runs signed in with KIFF it reads
KIFF's held calls (`GET /v1/me/card-exceptions?status=held`) once a minute, as Windows does
(tafs-windows#69):

- **What's notified:** each call held since the last read, as a desktop notification
  (`gio::Notification`). Its title is "Rae needs you", Coach's own agent reads "Coach needs
  you", and an agent no taf has reads "A taf needs you". Its body is what the call does and its
  amount, why it's held, and when to answer by, in the account's zone with the zone's short name
  when this computer's clock differs, then "Nothing is sent until you approve.".
- **What isn't:** what already waited when the account was read. That's in Needs you, and
  opening tafs doesn't announce it.
- **Who's notified:** only someone who answers held calls (`holds.answer`, as `canAnswerHolds`).
- **Gone:** a hold no longer held (answered elsewhere, or lapsed) loses its notification at the
  next read. Signing out withdraws every one, and ends the reads.
- **Clicking one:** tafs comes to the front and reads the account again, opening on Needs you
  (`PushDelegate.open`), and once Needs you lists that hold, its card (or its Card unit
  request) is brought to the top (`requestedHold`). CI's `needs-you-hold` opens as one would.
- **A read that fails** changes nothing, and the next minute reads again. A 401 signs this
  computer out, as any read does.
- **Each notification's id** is its hold's id hashed (SHA-256), so any id KIFF sends is a safe
  one.

Unlike Windows, the reads don't pause while the window is minimised: a minimised window is when
a notification helps. Needs you itself is read when the account opens and when a notification
is clicked, not every minute.

## Not yet

- **Answering from this computer:** KIFF takes an answer signed by the session's key. Until
  this app signs answers (and approves with a passkey), a call from KIFF is answered in KIFF.
  The sample's answers come with the sample's Card checks: until then they're dimmed.
- **Answering a step** (Done, Skip it: signed), **opening Coach** and **setting chats aside**:
  there, dimmed.
- **Answering a knock** on the taf's Activity, and the doors' lights on its avatar's rim: with
  the Doors screen.
- **Discussing a Card unit request with Coach, and dismissing it:** there, dimmed.
- **Alerts** (`AlertsCard`): the iPad offers them only to a session holding an approval key, and a
  Linux session never does. Held calls are notified (above).
- **Bringing a call into view** from its alert.
