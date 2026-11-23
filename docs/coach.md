# Coach

The iPad's Coach conversation (`SetupChatView` at a regular width) in Coach mode over the app
(below), with its message box. Its Coach button opens it, signed in with KIFF by someone who may use Coach; the sample
has no Coach to start, so there it opens Coach's intro (`introBefore`: board 9, "Coach mode", what
Coach does, and Start, which only signed in with KIFF can press; `coach::sample_intro`, CI's
`sample-coach`). What it reads and how it's decoded is
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
- **Reading again** (`followSetup`): the fallback when nothing streams (kiff/tafs
  `docs/scale.md`, `tafs-core/src/follow.rs`). While the page is open and no answer streams, the
  thread is read 3 s after the owner last wrote, then further apart while it doesn't change (6,
  10, 15, then 30 s; back to 3 s once it does), each wait a random ±20%, for five minutes after
  the owner last wrote. Never every second. Nothing is read while the window is minimised or
  suspended, and it's read at once when it's seen again. One read at a time, and none once the
  page is closed.
- **The answer as it's written** (`openStream`, `followSetupStream`): a message goes to Coach
  streamed, `POST /v1/tafs/{setup}/messages/stream` as the session, unsigned. It goes to tafs
  cloud's stream host (`TafsCloud.defaultStreamURL`, the function itself, as a run streams for
  longer than a CDN waits between bytes), which takes the session for the two stream routes only
  and nothing else (`stream::may_stream_to`). Each read waits up to fifteen minutes. When the
  stream can't open (no network to it, the route missing or failing), the message is sent plainly;
  a refusal tafs cloud means (a 4xx other than 404 or 408) reads as the plain send's would. While
  it streams, `LiveWork` stands in for the dots: the live line (tafs cloud's, else the step
  running, else Writing… or Thinking…) and the steps done under it, just before the first item
  the answer recorded, else under the rest. Coach's words show under it as they're written,
  unless the thread already holds them (`alreadyRecorded`), and the cards that come with them
  wait until they're written (`textBeforeCards`). At each tool call, card or message the thread
  is read and the live words start again; after a pause the steps start again. At its end the
  thread is read, then the live answer goes. A run that ended with `ERROR` shows "It stopped part
  way…" with Retry, which sends its message again (`retrySetupRun`); one cancelled shows Stopped.
  A stream that drops leaves Coach at work, and the reads show the rest. A new or
  another conversation, or leaving the page, stops following it (the run goes on at tafs cloud).
  Not ported: the reveal of an answer letter by letter, which the iPad does only with no stream,
  and Home's read at the stream's end (Linux reads Home when it opens).
- **The pin**: a message sent to Coach goes to the top and stays while its answer grows below,
  as in a taf's chat ([chat.md](chat.md), `tafs-core/src/pin.rs`, `tools/pin-check.sh`).
- **New conversation** (the pencil in the bar, `newSetupChat`): an empty chat, and the next
  message starts it (`new_chat: true`); the chat tafs cloud answers is the one shown from then on.
  One that's empty already stays, and shows its top.
- **Previous conversations** (the bar's menu, `SetupChatsList`): Coach's chats, newest first, read
  again as it opens, each by what the owner asked and when it last changed, the one shown on the
  raised colour; one opens it alone, and messages go on in it (`openSetupChat`). Archive asks first
  ("Archive this conversation?") and then `POST /v1/tafs/{id}/chats/{session}/archive`; the one
  shown gives way to a new conversation. Archived (`GET /v1/tafs/{id}/chats/archived`) opens one,
  or puts it back (`POST …/unarchive`). A refusal is said at the top of the list.
- **Coach on a taf** (`improveTaf`, `askCoachAbout`): from a tafs cloud taf's page (Improve in
  Coach, a next step's Ask Coach, its work's fix, Reconnect it in Coach), its chat's Ask Coach card
  and a Card unit request's Discuss, Coach opens on a new conversation on that taf, with the words
  in the box where there are some, nothing sent. Only for someone who may use Coach and manages
  the taf, as the iPad's `askCoachAbout` and `canDiscuss`: anyone else reads the card or the line
  without its button, and the page's Coach is Coach on no taf. Its first message goes to
  `POST /v1/tafs/{taf}/improve` with `{"text"}` alone (`improve`: tafs cloud adds the taf's
  context, so it takes no files: the words go alone, the files in the tray wait for the next
  message with `coach.improve_no_files` under them, and the paperclip isn't offered meanwhile, as
  the iPad's `improvementHoldsFiles`), the dots from the send; then the conversation
  goes on in the chat tafs cloud answered with, as any other. A new conversation, or another
  reopened, is on no taf. Opened over the page of a taf this person manages (`tafOnScreen`), an
  empty new conversation offers it: the switch with New taf chosen and "Improve Rae", which turns
  it into one on that taf. Empty, it says what it's about as a choice at its top (`startSwitch`: New
  taf starts a new conversation; "Improve Rae" is chosen), then "Improve Rae" and "Say what should
  change, or ask how it's going…", and three first messages that send as they are
  (`improveExamples`: How is it going?, Why was something refused or held?, It can't find the work
  it should do), in place of board 7 and "Ask anything about your tafs".
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
  and what Coach does; then tafs cloud's recipes, by category, as the account's read had them
  (kiff/tafs#476): each its name (`lead`) and line, a tap puts its words in the box to edit and
  send, never sending. Not on a conversation that improves a taf.
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
  version, with See version once it's made, which opens the Versions of the taf it improves
  (`improvedTaf`: by its id at tafs cloud, else its agent), pushed in Coach's panel with Back and
  its title in the bar, as the iPad's (Coach's page is in the panel's own navigation); or that
  it's made, in words, for a taf not listed here); a drive's ask; a drive's end ("What I did": the task, each step with what it
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

On a touch screen, a swipe of the conversation to the right closes it, as the iPad's (over 140 px
across and under 60 up or down, `coach_mode::swipe_closes`). It's taken from the conversation's
scroll only once it's clearly across, and a mouse's drag still selects text. It slides in from the
right and out again, as the iPad's.

Not on Linux: the phone's `PhoneCoachPanel` (a compact width, which this computer's sidebar layout
doesn't have).

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
something and where it stands; then **the tools it connects** (`tools`): each vendor the
conversation's cards name, once, in the order they came, from planque's grants read once with this
computer's device key while the panel shows (`chrome::grants`): Connected in green, Signed in or
Waiting for you in amber, Not connected in red with planque's words for why, its tools once
connected, and "No taf sees your sign-ins."; then the Card KIFF drafted for the latest approval that names one,
dashed until KIFF confirms it, with "A draft until you approve it in the conversation." With
nothing made yet, what will show there.

A grants read that fails leaves the tools out, as before. The iPad shows the panel only once the phone is enrolled with
planque (`planqueReady`), which is how it starts Coach; this computer starts nothing, and shows the
panel beside any conversation tafs cloud has for it.

## The phone's, never this computer's

Approve, a sign-in's button, and a drive's Approve are the phone's. Approving a card ends in tafs
cloud's report signed by the device behind Face ID, and its other steps grant access (planque's
approve, tools and Cards together), which a computer never does (the PM's 19:52 (b), with qa-kiff:
a security-key report for a `cards`-only ask is a design question first). Signing in at a vendor
grants access, and a drive runs on the phone for KIFF's critical key. Those cards keep the
can't-approve line where those buttons go.

Their refusals are answered here, as on the iPad, where they sit outside `ApprovalGate`: Decline
on an approval, Not now on a sign-in (in tafs cloud's words when it gives some) and Not now on a
drive (which declines it), each `answerCard` as data, unsigned. `coach_send::answer_value` builds
only those for these cards: "declined" for an approval or a drive, "not now" for a sign-in, and
nothing else whatever the text, so nothing here can build "approved" or "signed in". A test
holds it to that.

## An answer with no stream

When a message couldn't stream (the stream didn't open, so it was sent plainly), Coach's answer
that comes after the page opened is revealed letter by letter as it arrives (`RevealingBubble`):
about 400 characters a second, two seconds at most, from where it stood across redraws, with what
it's in the middle of closed on the way. The cards that come with it wait until it's whole
(`textBeforeCards`). With animations off it shows at once and holds nothing.

## A dial card

`KnobDraftCard`: "{taf} will work like this", then the dial version Coach drafted, read from tafs
cloud with the taf's dial (`GET …/knob`, as the session, again once the card's version is newer):
each connection's rings apart under its name, When, What and Where in tafs cloud's words, or the
draft's own words when it has no ring. Change goes back to the message box. Confirm makes that
version live, which the device checks before it goes, so the gate's line stands in its place,
with "One confirmation sets all three rings." "This dial is live." once it is, and "A newer dial
replaced this one." once another is newer.

## Not yet

- No mic: a Linux desktop has no on-device speech recognition, as in a taf's chat.
- What the iPad reads beside the thread: planque's grant (its words, terms and profile).
- A refused connection's lines are decoded but not drawn: the iPad's Coach doesn't draw them.

## Screenshots

`--page coach --coach <answer.json>` (with `--home`) draws a conversation from a file: nothing is
read or sent. `tools/screenshots.sh` saves `coach` (the contract's conversation, Coach at work: the box with
Stop), `coach-wider` (tafs's own contract, `contracts/coach-wider.json`: an approval waiting under
its wider line; the tests read the same contract, so a change in tafs cloud's shape fails them
here), `coach-asks` (`crates/tafs-app/fixtures/coach-asks.json`: Coach at rest with a confirm and a
questions card waiting, a message its run never read as Not sent, and one tafs cloud holds),
`coach-stream` (`--coach-stream crates/tafs-app/fixtures/coach-stream.txt`: a stream's
`event:`/`data:` lines taken after the contract's conversation, as the app takes them, so an answer
shows mid-stream: its live line, the steps done and its words so far),
`coach-empty` (`crates/tafs-app/fixtures/coach-empty.json` with no taf: board 7 over the empty
chat), `coach-recipes` (the same, with `crates/tafs-app/fixtures/recipes.json`'s recipes under it),
`coach-start` (`shared/contracts/setup-start.json`: no Coach yet, board 9) and
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
