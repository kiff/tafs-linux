# Coach

The iPad's Coach conversation (`SetupChatView` at a regular width) in Coach mode over the app
(below), with its message box. Its Coach button opens it, signed in with KIFF by someone who may use Coach; the sample
has no Coach, so there it still opens its title alone. What it reads and how it's decoded is
`tafs-core/src/coach.rs`; the device's proof is `tafs-core/src/planque.rs`; what's sent, and how the
conversation holds it while it goes, is `tafs-core/src/coach_send.rs`; what it draws is
`tafs-core/src/coach_lines.rs`, tested there and in every language at the pin
(`tests/at_the_pin.rs`); the page is `crates/tafs-app/src/coach.rs`, with what it draws alike with a
taf's chat ([chat.md](chat.md)) in `crates/tafs-app/src/chat.rs`. Nothing it sends is signed:
each goes to tafs cloud as the session (`kiff::cloud_send`, which reaches tafs cloud only).

## What's read

In the iPad's order (`loadSetup`, `loadSetupThread`), at tafs cloud as the session:

1. **Coach's taf:** `GET /v1/setup`'s `setup.id` (`setupRead`, contract `setup-coach.json`), read
   as strictly as the iPad's `CloudTaf`. No Coach yet: its start (`introBefore`), board 9 and
   "Coach mode", with why this computer can't in Start's place: starting Coach is signed, so it's
   done on the phone. The same answer's `tools` is Coach's tools as KIFF has them (below). Linux sends no `X-Time-Zone` with it: on a new account that header sets the account's zone, and this computer changes nothing.
2. **Its newest conversation:** `GET /v1/tafs/{id}/chats` (contract `coach-chats.json`), the chat
   with the latest `updated`. Without the list, the conversation reads without a session, as on
   the iPad.
3. **This device's proof** (below), when it can be made.
4. **The conversation:** `GET /v1/tafs/{id}/conversation/{session}` (contract
   `coach-conversation.json`) with the proof in `Planque-Device-Signature`. `?can=` goes only
   with a proof and steps this device signs beyond connecting and cards; this computer signs none,
   so it is never sent.
5. **KIFF's drafts** (`readCardDraft`): after each read, every draft an approval or a sign-in
   names that isn't read as confirmed, `GET /v1/me/agent-drafts/{id}` at KIFF as the session, an
   unsigned read (`tafs-core/src/drafts.rs` › `draft_read`; confirming a draft is signed, on the
   phone). It's decoded as `AgentDraft` (as strict as the iPad's synthesized decoder, an unknown
   field left alone). A read KIFF refuses keeps KIFF's reason for the card (`setupDraftErrors`),
   and a draft read before stays drawn.

The answer is decoded as `setupThread(decoding:)`: the messages strictly (one the app can't read
fails the read, and the screen says "The conversation couldn't be loaded: …"); each card, coach
line and drive's end leniently (one it can't read is left out, never the thread); a mark it
can't read on a message leaves the message plain; an answer it can't read leaves its card
unanswered. A card's own fields are as strict as the iPad's synthesized decoder, with two marks
read leniently: `signs` (each field) and `wider` (another shape is no mark; blank words are none).
Cards fold by tafs cloud's key, else their conversation and id (`SetupThread.fold`).

## What's sent

All at tafs cloud, as the session, unsigned. A 401 on any of them, or on a read, signs this computer
out, as every other read does.

- **A message** (`deliverSetup`): `POST /v1/tafs/{id}/messages` with `{"text", "session"?,
  "new_chat"?, "new_chat_after_handover": true}` (`ChatTarget`: the chat shown, a new one, or the
  live one; Coach always follows a new chat tafs cloud starts after a hand-over,
  `followNewSetupChat`). Trimmed, at most 2,000 characters, one at a time. It shows at once, stamped
  no earlier than the thread's latest (`SentMessages.stamp`), and stays until tafs cloud's copy takes
  its place, found by the id the send answered (`Sent.message.id`), else by its words, for two
  minutes at most (`SentMessages.merge`). Then Coach is awaited (the dots) until it's at work or
  answers, two minutes at most (`settleSetupAwaiting`), and the thread is read again.
- **While Coach works**, Send writes to its run (`steer`): `POST /v1/tafs/{id}/steer` with
  `{"run", "text", "hold": true}`; taken, it shows under the thread as unread; kept, tafs cloud's
  held message shows with "Waits until it's done…" (`QueuedBubble`). `held_full` and the credits'
  refusals leave the words in the box with tafs cloud's words.
- **Stop** (Send's place while Coach works and the box is empty; Ctrl+. as the iPad's ⌘.):
  `POST /v1/tafs/{id}/stop` with `{"session"}` (the chat at work, else the thread's), then
  "Stopped" under the thread, or "It didn't stop: …".
- **A card's answer** (`answer`): `POST /v1/tafs/{id}/components/{component}/answer` with
  `{"answer", "session"?}`: a confirm's option value, or a questions card's answers
  (`{"<question>": {"value"} | {"values"} | {"own"}}`, `QuestionAnswers`). The card goes quiet at
  once; a 409 (it doesn't wait any more) reads the thread again; any other failure says "Your answer
  didn't reach Setup" under the card (`CardNote`).
- **Refused:** `ai_consent_required` puts the AI-data consent's screen up (`ai_consent::required`,
  from the consent in the refusal) and the words go back in the box, or the card stays as chosen;
  a credits code (409 `credits_used_up`, `limit_reached`, `no_plan`) says tafs cloud's words above
  the box and the words go back; anything else keeps the message in the thread as **Not sent** with
  why, Edit (back to the box) and Retry (`UnsentBubble`); a send refused because the account is
  halted offers Resume, which opens Halt all. tafs cloud's `steering.not_sent` shows the same way,
  once. A message not sent has a menu (a right click or a long press): Copy, the message as it was
  written, and Delete, which takes it out of the thread unsent; an answer's card can then be
  answered again (`discardUnsent`).
- **Reporting Coach's answer:** an answer of Coach's with tafs cloud's id has Report this answer in
  its menu (a right click or a long press), which sends `POST /v1/tafs/{setup}/messages/{id}/report`
  with the answer's chat, as a taf's answer does ([chat.md](chat.md#reporting-an-answer)).
- **Reading again** (`followSetup`): while the page is open, every second while Coach works, else
  every three, for 100 reads after the owner last wrote; one read at a time, and none once the page
  is closed. Linux has no stream: there's no live work, no reveal and no pin of a sent message.
- **New conversation** (the pencil in the bar, `newSetupChat`): an empty chat, and the next
  message starts it (`new_chat: true`); the chat tafs cloud answers is the one shown from then on.
  One that's empty already stays, and shows its top.
- **Previous conversations** (the bar's menu, `SetupChatsList`): Coach's chats, newest first, read
  again as it opens, each by what the owner asked and when it last changed, the one shown on the
  raised colour; one opens it alone, and messages go on in it (`openSetupChat`). Archive asks first
  ("Archive this conversation?") and then `POST /v1/tafs/{id}/chats/{session}/archive`; the one
  shown gives way to a new conversation. Archived (`GET /v1/tafs/{id}/chats/archived`) opens one,
  or puts it back (`POST …/unarchive`). A refusal is said at the top of the list.
- **The box** keeps what's typed while the app runs (`setupDraft`); signing out clears it. Ctrl+Enter
  sends (the iPad's ⌘↩). A screenshot's conversation (`--coach`) sends nothing: Send, Stop and the
  cards' answers do nothing there.

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

- **The title**, inline, as the iPad's navigation bar, on Coach's own wash, with New conversation
  and the menu at its trailing edge.
- **The empty chat:** board 7 while the account has no taf or one, "Ask anything about your tafs"
  and what Coach does.
- **The plan** (`SetupPlan`) pinned above the thread: "Plan · 1 of 3" and the step it's at,
  opening to every step with its mark.
- **The thread** in the order already drawn (`SteadyOrder`, iPad tafs#1241, Android's
  `Chat.steady`): what's on screen keeps its place while Coach answers, new items go after it, and
  the owner's message sent from here gives its place to tafs cloud's copy (another id and time, the
  same words), so neither it nor a card moves when the copy lands. Another conversation, or a new
  one once its chat is known, starts in the thread's own order (`SetupThread.items`): a card that waits after what Coach
  wrote with it, up to the owner's next message or its answer. A time where parts are an hour or
  more apart, and at the top. The owner's words on the right, on the brand sweep, an answer to a
  card in their own words ("Approve", "€50") and none at all when the card shows it, and
  "Answered the card above" under words that answered a card. Coach's words full width, its
  Markdown as formatting, under "Worked for 1 min 1 s ›", which opens its steps in Coach's words.
- **Each card by its kind:** a notice in its tone; a confirm's question and choices, answered here; a
  questions card's questions and choices, picked here with "1 of 2 answered", Something else… and
  its own-answer field, Send answers once each has an answer, and Chat about this, which puts the
  card away ("Show them again"); an approval; a sign-in; a result (set up, setting up, wasn't set up, a plan needed with
  Choose a plan, or the button that opens the taf once it's on the list; an improvement's new
  version, with See version once it's made, which opens the taf it improves (`improvedTaf`: by its
  id at tafs cloud, else its agent) on its Versions, or that it's made, in words, for a taf not
  listed here); a drive's ask; a drive's end ("What I did": the task, each step with what it
  touched, and why it ended, in tafs cloud's words or the app's for its reason; its steps are
  read with `try?`, so one it can't read leaves them all out, as on the iPad); a dial card's title. An answered card folds to one line
  (`OutcomeLine`); an approval's opens again on a click, and Fold puts it back. An approval whose
  Card KIFF took the signature for (`signed`: confirmed or claimed) names that Card on its line,
  with the Card's lines under it.
- **Approvals and sign-ins:** the eyebrow in the card's state, the card's words (tafs cloud's
  line when the connection is already done, else Coach's title and words for it, since this
  computer reads no grant from planque), the taf's Card as KIFF drafted it (its title, or tafs
  cloud's words when it's the one approved next, then its lines in monospace: KIFF's own, else
  `DraftCard.lines` in the app's words, two grants that read alike each adding their action),
  or, while it isn't read, "Reading KIFF's words…", or KIFF's reason with Try again, which reads
  it again; tafs cloud's line for a Card approved next on a card with no draft, then
  **tafs cloud's `wider.words`** in the amber (14 medium, as `WiderLine`), above where the buttons
  go, then why this computer can't approve (`ApprovalGate`'s line) in the buttons' place.
- **After the thread:** "While you were away" and how the tasks Coach handed out went, the dots
  while Coach is at work, and what was written to its run that it hasn't read yet.
- **Where it opens, and ↓:** opened, the conversation lands on the card waiting for the owner
  (`setupWaiting`), in the middle, else at the latest; it follows what arrives while its latest is
  in view. Read back, ↓ over its foot goes to the latest, with the amber dot when something
  arrived below ("Jump to the latest, something new"). While the card waiting is out of view, the
  Needs you bar over the foot names it ("Needs you · approve Stripe", "Needs you · sign in at
  Gmail", "Needs you · let Coach drive", "Needs you · a question": the app is the card's own
  service, since planque's grants aren't read here), and shows it (`NeedsYouBar`,
  `crates/tafs-app/src/chat.rs` › `Jump`).

## Coach mode

Coach slides in from the right over whatever is open (`CoachPanel`, `crates/tafs-app/src/coach_mode.rs`,
`tafs-core/src/coach_mode.rs`), which stays in view under a light shade; anything that opens Coach
(the sidebar's Coach, the brief's next move, Connect apps' Ask Coach) opens it there. It opens at a
third of the window, or the width last chosen, kept in the user's config folder (`coach.width`,
`coach.full`). Its left edge drags it wider, up to full screen within 40 px of the window's edge,
or narrower, never under a third; dragged more than 60 px past that, it closes. Its title row has
Back and ⤢ (Full screen, or Back to its width) before the title, as the iPad's `CoachView`. A click
on the shade (not when full screen), Escape and Back close it. Closing never ends a conversation:
its page goes, its reads stop, and opening it again reads it anew, the words not sent kept.

Not on Linux: the swipe of the conversation to the right that closes it on a touch screen (a drag
there selects text here), and the phone's `PhoneCoachPanel` (a compact width, which this computer's
sidebar layout doesn't have). The iPad slides it in and out; here it appears and goes.

## Coach's tools

While `GET /v1/setup`'s `tools` says they're still to give (`to_give`, `taken` or `unknown`),
one line above the conversation says so in tafs cloud's words (`CoachToolsRow`, the step's
`row`, else "Coach's tools"), and opens the step (`CoachToolsSheet`,
`tafs-core/src/coach_tools.rs`): its title, its intro, a few groups up front, "Each tool and its
limits", which unfolds each tool with what it does and why and Coach's Cards in KIFF's words, then
the ask. Every word is tafs cloud's or KIFF's; the app's own stand in only for a tafs cloud that
sends none. Taken, it says which name another tool in KIFF holds; unknown, that KIFF didn't
answer. Once given, there's no line.

Giving them is the one confirmation, signed on the phone behind Face ID, and the iPad wraps its
button in `ApprovalGate`: here the gate's line stands in the button's place, and nothing is sent
(the connections batch and Coach's Cards' draft are never built or sent from this computer).
Not now closes the step. The iPad also opens the step at the end of a first Start; this computer
starts nothing, so it opens only from the line. `tools` is read as leniently as the iPad's
`JSONSerialization` read: a field of another shape is none, a group or a tool that isn't an
object is left out, and a tool with no name too.

## Taking shape

On a window wide enough (the conversation's page at least 780 px wide, `shaping`), what the
conversation has made so far stands beside it, 420 px wide on Coach mode's deeper blue
(`SetupShapePanel`, `tafs-core/src/setup_shape.rs`), and the plan moves from above the thread
into it. Narrower, the conversation is alone. It reads only what this computer already holds:
"Taking shape", the taf's name once a result hands it over (else the plan's title); the plan's
steps with their marks, the step at work as "Needs you" while a card waits for the owner
(`setupWaiting`), a skipped step struck through; with no plan, each card that asked the owner
something and where it stands; then the Card KIFF drafted for the latest approval that names one,
dashed until KIFF confirms it, with "A draft until you approve it in the conversation." With
nothing made yet, what will show there.

The iPad also lists the apps being connected, from planque's grants: this computer doesn't read
them, so that part isn't drawn. The iPad shows the panel only once the phone is enrolled with
planque (`planqueReady`), which is how it starts Coach; this computer starts nothing, and shows the
panel beside any conversation tafs cloud has for it.

## The phone's, never this computer's

Approve, Decline, a sign-in's button and Not now, and a drive's buttons. This computer is
view-only: those cards keep the can't-approve line where their buttons go. Decline and Not now
are unsigned on the iPad, but they stay with the phone too, so the whole card is answered in one
place. `coach_send::answer_value` takes only a confirm's choice and a questions card's answers.
For an approval, a sign-in or a drive it gives none, whatever the text, so nothing here can build
"approved" or "signed in" by mistake. A test holds it to that.

## Not yet

- Setting a chat aside from Needs you; New conversation's choice of a taf to improve
  (`startSwitch`, `improveTaf`).
- The microphone and attachments; the recipes on an empty chat; an improvement's first message
  (`improve`).
- What the iPad reads beside the thread: planque's grant (its words, terms and profile).
- The live stream (`LiveWork`, revealing, pinning a sent message, a run that failed part-way with
  Retry); and a dial card (`KnobDraftCard`).
- A refused connection's lines are decoded but not drawn: the iPad's Coach doesn't draw them.

## Screenshots

`--page coach --coach <answer.json>` (with `--home`) draws a conversation from a file: nothing is
read or sent. `tools/screenshots.sh` saves `coach` (the contract's conversation, Coach at work: the box with
Stop), `coach-wider` (tafs's own contract, `contracts/coach-wider.json`: an approval waiting under
its wider line; the tests read the same contract, so a change in tafs cloud's shape fails them
here), `coach-asks` (`crates/tafs-app/fixtures/coach-asks.json`: Coach at rest with a confirm and a
questions card waiting, a message its run never read as Not sent, and one tafs cloud holds),
`coach-empty` (`crates/tafs-app/fixtures/coach-empty.json` with no taf: board 7 over the empty
chat), `coach-start` (`shared/contracts/setup-start.json`: no Coach yet, board 9) and
`coach-chats` (`--coach-chats shared/contracts/coach-chats.json`: Previous conversations open over
the contract's conversation). `--coach-drafts <answers.json>` gives KIFF's answers to the drafts
by id (`{"status", "answer"}`), never read: `coach-draft`
(`crates/tafs-app/fixtures/coach-card.json` with `coach-drafts.json`: an approval with its Card
read) and `coach-draft-refused` (`coach-drafts-refused.json`: KIFF's refusal, with Try again).
`--coach-setup <answer.json>` gives tafs cloud's `GET /v1/setup` answer for Coach's tools, and
`--page coach-tools` opens their step: `coach-tools` (`shared/contracts/setup-coach.json` over
`coach-asks.json`). `--page coach` opens Coach mode at a third of the window over the Overview, and
`--page coach-full` full screen, keeping nothing: `coach-full` (`coach-card.json` with its draft,
"Taking shape" beside it) and `coach-drive` (`crates/tafs-app/fixtures/coach-drive.json`: a drive's
end and a new ask).
