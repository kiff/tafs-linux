# Coach

The iPad's Coach conversation (`SetupChatView` at a regular width), beside the sidebar, **read
only**. Its Coach button opens it, signed in with KIFF by someone who may use Coach; the sample has
no Coach, so there it still opens its title alone. What it reads and how it's decoded is
`tafs-core/src/coach.rs`; the device's proof is `tafs-core/src/planque.rs`; what it draws is
`tafs-core/src/coach_lines.rs`, tested there and in every language at the pin
(`tests/at_the_pin.rs`); the page is `crates/tafs-app/src/coach.rs`.

## What's read

In the iPad's order (`loadSetup`, `loadSetupThread`), at tafs cloud as the session:

1. **Coach's taf:** `GET /v1/setup`'s `setup.id` (`setupRead`, contract `setup-coach.json`), read
   as strictly as the iPad's `CloudTaf`. No Coach yet: the empty chat ("Ask anything about your
   tafs"); starting Coach is signed, so it's done on the phone. Linux sends no `X-Time-Zone` with
   it: on a new account that header sets the account's zone, and this computer changes nothing.
2. **Its newest conversation:** `GET /v1/tafs/{id}/chats` (contract `coach-chats.json`), the chat
   with the latest `updated`. Without the list, the conversation reads without a session, as on
   the iPad.
3. **This device's proof** (below), when it can be made.
4. **The conversation:** `GET /v1/tafs/{id}/conversation/{session}` (contract
   `coach-conversation.json`) with the proof in `Planque-Device-Signature`. `?can=` goes only
   with a proof and steps this device signs beyond connecting and cards; this computer signs none,
   so it is never sent.

The answer is decoded as `setupThread(decoding:)`: the messages strictly (one the app can't read
fails the read, and the screen says "The conversation couldn't be loaded: …"); each card, coach
line and drive's end leniently (one it can't read is left out, never the thread); a mark it
can't read on a message leaves the message plain; an answer it can't read leaves its card
unanswered. A card's own fields are as strict as the iPad's synthesized decoder, with two marks
read leniently: `signs` (each field) and `wider` (another shape is no mark; blank words are none).
Cards fold by tafs cloud's key, else their conversation and id (`SetupThread.fold`).

## The proof

`grantsReadProof`: this device signs its own read of the account's grants at planque,
`GET /v1/accounts/{tenant}/grants` (the tenant escaped as one path segment), as planque's
`device.Message`:

```
planque.request.v1
https://planque.kiff.dev
{tenant}
GET
/v1/accounts/{tenant}/grants
{SHA-256 of the empty body, lowercase hex}
{signed at}
```

The signing time is `SigningTime.next`: RFC 3339 in UTC with milliseconds, always later than the
last this app signed with. The device key signs it with no prompt (ECDSA P-256 over SHA-256, DER,
base64: what KIFF and planque verify), and the header reads `t={signed at},v1={signature},d={device}`,
`device` being this computer's id in KIFF: the first device `GET /v1/me/devices` marks
`this_device` whose id starts `dev_` (`thisDeviceID`). tafs cloud forwards it to planque, and only
then marks each card waiting with what its sign-in covers (`signs`) and, where an app's sign-in
can't be narrowed to the job, `wider: {words}`.

It's made only for a kept KIFF session with its tenant and its device key (`KeptSession.device_key`),
once KIFF names this computer; without any of these, the conversation is read without it, as the
iPad does, and tafs cloud marks no card. The token, the key and the signature are never logged.

## What's drawn

- **The title**, inline, as the iPad's navigation bar, on Coach's own wash.
- **The plan** (`SetupPlan`) pinned above the thread: "Plan · 1 of 3" and the step it's at,
  opening to every step with its mark.
- **The thread** in the iPad's order (`SetupThread.items`): a card that waits after what Coach
  wrote with it, up to the owner's next message or its answer. A time where parts are an hour or
  more apart, and at the top. The owner's words on the right, on the brand sweep, an answer to a
  card in their own words ("Approve", "€50") and none at all when the card shows it, and
  "Answered the card above" under words that answered a card. Coach's words full width, its
  Markdown as formatting, under "Worked for 1 min 1 s ›", which opens its steps in Coach's words.
- **Each card by its kind:** a notice in its tone; a confirm's question and choices; a questions
  card's questions and choices, "0 of 1 answered", Something else…, Send answers and Chat about
  this; an approval; a sign-in; a result (set up, setting up, wasn't set up, a plan needed with
  Choose a plan, or the button that opens the taf once it's on the list; an improvement's new
  version); a drive's ask; a dial card's title. An answered card folds to one line
  (`OutcomeLine`); an approval's opens again on a click, and Fold puts it back.
- **Approvals and sign-ins:** the eyebrow in the card's state, the card's words (tafs cloud's
  line when the connection is already done, else Coach's title and words for it, since this
  computer reads no grant from planque), tafs cloud's line for a Card approved next, then
  **tafs cloud's `wider.words`** in the amber (14 medium, as `WiderLine`), above where the buttons
  go, then why this computer can't approve (`ApprovalGate`'s line) in the buttons' place.
- **After the thread:** "While you were away" and how the tasks Coach handed out went, the dots
  while Coach is at work, and what was written to its run that it hasn't read yet.

## Not yet

- **Sending anything:** the message box, the recipes on an empty chat, and every answer. A
  confirm's choices, a questions card's choices, Send answers and Chat about this are there,
  dimmed.
- **Approving:** Approve, Decline, a sign-in's button and Not now, and a drive's Approve. This
  computer can't approve; the phone does.
- What the iPad reads beside the thread: planque's grant (its words, terms and profile) and KIFF's
  draft of a Card on an approval.
- The live stream (`LiveWork`, revealing, pinning a sent message), the Needs you bar and ↓, New
  conversation, Previous conversations and archiving, Coach's tools row, "Taking shape"
  (`SetupShapePanel`), a dial card (`KnobDraftCard`), a drive's steps and why it ended, a result's
  See version, and reading again while Coach works.
- A refused connection's lines are decoded but not drawn: the iPad's Coach doesn't draw them.

## Screenshots

`--page coach --coach <answer.json>` (with `--home`) draws a conversation from a file: nothing is
read or sent. `tools/screenshots.sh` saves `coach` (the contract's conversation) and `coach-wider`
(`crates/tafs-app/fixtures/coach-wider.json`: the same chat with an approval waiting under its
wider line).
