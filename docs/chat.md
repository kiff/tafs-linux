# A taf's chat

The iPad's taf chat (`TafChatView`), on a taf's page's Chat tab, for a taf tafs cloud runs, signed in
with KIFF, and the sample's ([below](#the-samples-chat)). The page opens on it, as the iPad's does. What it reads,
holds and draws is `tafs-core/src/taf_chat.rs`, tested there and in every language at the pin
(`tests/at_the_pin.rs`); the pane is `crates/tafs-app/src/taf_chat.rs`. What it shares with Coach's
conversation ([coach.md](coach.md)) is shared code: the routes, bodies and answers of what's sent
(`tafs-core/src/coach_send.rs`: `messages_path`, `steer`, `stop`, `sent`, `steered`, `Refusal`,
`merge`, `stamp`, `steered_into`), the owner's bubble, an answer's words, the dots, the times between
parts and the message box (`crates/tafs-app/src/chat.rs`). Nothing it sends is signed: each goes to
tafs cloud as the session (`kiff::cloud_send`).

## What's read

`GET /v1/tafs/{id}/conversation` as the session, with no proof and no session: the taf's last runs
and the person's live chat (`conversation`). It's decoded as the iPad's `thread(_:)`:

- `messages` strictly: one the app can't read fails the read, and the chat says "The thread couldn't
  be loaded. …" above the box. Each message's marks (`work`, `door`, `delivered`, `counted`,
  `refused`, `answers`, `id`) as leniently as Coach's.
- `working` (`{session, run?}`) strictly when it's there.
- `steering`, `stopped` and `capped` leniently: one it can't read is none. `stopped` names any stop;
  an older tafs cloud's `capped` stands in for it.
- `components` must be a list; of its cards, only an `ask_coach` card with words is kept (`AskCoachCard`),
  and one it can't read is left out. A taf's other cards aren't drawn in its chat, as on the iPad.

The thread is read when the Chat tab first shows. After that, plain reads are the fallback when
nothing streams (`followChat`; kiff/tafs `docs/scale.md`, `tafs-core/src/follow.rs`). While the taf
owes an answer (the owner's message is the thread's last) or works, the thread is read 2.5 s after
the owner wrote, then further apart while it doesn't change (5, 10, then 30 s; back to 2.5 s once it
does), each wait a random ±20%, for two minutes after each message of the owner's. An answer that
streams reads the thread itself, so nothing else is read meanwhile. Nothing is read while the window
is minimised or suspended, and it's read at once when it's seen again. One read at a time, and none
once the pane is gone.
What the chat holds (the thread, what was sent and not sent, the stop) is kept for each taf while the
app runs, and signing out forgets it, with the box's words.

## What's sent

All at tafs cloud, as the session, unsigned; a 401 on any of them signs this computer out.

**One message, one id** (`client_message_id`, kiff/tafs `docs/api.md`): each message composed here,
in a taf's chat and in Coach, gets its own v4 UUID. It goes on the stream, on the plain send that
takes its place when the stream fails (the same body), and on a steer. A Not sent row keeps it only
where tafs cloud may have taken the message: nothing answered (a dropped connection), a 408, a 5xx,
or a 2xx whose answer couldn't be read. Retry then goes under the same id, and tafs cloud answers a
repeat with the run it already started, never a second one. After any other answer (a 409, a 400, a
429), Retry is a new message with a new id (`retry_keeps_id`). Edit and a new message get a new id. An improvement's first
message (`/improve`) and a trial's chat send none, as on Windows. A 409 `dedup_key_conflict` is Not
sent with `error.sent_with_other_text`. The rules are `tafs-core/src/coach_send.rs` ›
`new_message_id`, `message_body`, `steer_body`, with tests in both chats.

- **A message** (`deliverChat`): `POST /v1/tafs/{id}/messages` with `{"text"}` alone: the taf's live
  chat (`ChatTarget.live`). Trimmed, at most 2,000 characters ("That's longer than 2000 characters…"
  above the box, the words left in it), one at a time. It shows at once, faded, stamped no earlier
  than the thread's latest, until tafs cloud's copy takes its place, by the id the send answered, else
  by its words (`SentMessages.merge`). The taf is then awaited (the dots, and Stop) until tafs cloud
  says it's at work or it answers, two minutes at most, and the thread is read again.
- **While the taf works**, Send writes to its run: `POST /v1/tafs/{id}/steer` with `{"run", "text",
  "hold": true}`. Taken, it shows under the thread with "Rae reads this at its next step" ("…when it
  carries on" while the run waits); kept, tafs cloud's held message shows with "Waits until it's done,
  then goes as your next message". `held_full` says tafs cloud's error above the box and leaves the
  words in it. With no run yet (the first message still on its way), the words stay in the box.
- **Stop** (Send's place while the taf works and the box is empty; Ctrl+.): `POST /v1/tafs/{id}/stop`
  with the session at work, else the thread's latest. Over here at once: Stop and the dots go, though
  tafs cloud may say it works until its next step. Stopped needs no line; "It didn't stop: …" when it
  didn't.
- **Refused:** 429 without tafs cloud's words keeps the message as Not sent with "That's a lot of
  messages in a few minutes. Give Rae a moment, then try again.", gone once a message goes through;
  `ai_consent_required` puts the AI-data consent's screen up and the words go back in the box; a
  credits code says tafs cloud's words above the box and the words go back; anything else keeps the
  message as **Not sent** with tafs cloud's words, Edit (back in the box) and Retry.
- **Refused for a halt** (`halted: "taf"` or `"account"`): "Not sent · " and tafs cloud's words, then,
  where the iPad offers Resume, why this computer can't (`ApprovalGate`'s line): resuming is signed,
  so it's done on the phone. Retry sends it again once it's resumed. An older tafs cloud's halt, known
  by its words, says "Rae is halted" and "Nothing reaches it until you resume it…", with the same
  line.
- **What a run never read:** tafs cloud's `steering.not_sent`, and what a run that's over listed
  unread ("You stopped it before it read this.", "It wasn't read before the work ended."), each shown
  once as Not sent. A Retry kept again still owes it. This computer keeps which were dealt with in
  memory only, so a restart shows them again.
- **An owner's message's menu** (a right click or a long press): Send again, Edit and send, Copy.
  A message not sent has its own: Copy, and Delete, which takes it out of the thread unsent
  (`discardUnsentChat`).
- `--chat <answer.json>` (with `--home`) draws a chat from a file: nothing is read or sent.

- **The answer as it's written** (`openStream`, `followChatStream`): a message goes streamed first,
  `POST /v1/tafs/{id}/messages/stream` to tafs cloud's stream host, as Coach's does
  ([coach.md](coach.md)), and plainly when the stream can't open; a trial's chat sends plainly. While
  it streams, the taf's words show as they come, each part up to a tool call kept as it was
  written (once: words the thread already holds show there), with `LiveWork` just before the
  first of them: the live line and the steps done. The words being written now show at an even
  pace, not in the bursts they arrive in, catching up within a few frames, and whatever they're in
  the middle of is closed so no Markdown symbol shows on the way: an open bold or code, a link
  whose address isn't in yet (its words). With animations off, all of it at once (`SteadyMarkdown`,
  `chat::steady_answer_words`, `coach_lines::healed`; Coach's answer streams the same way). A
  message the run took, or a run that said
  something, is read from the thread. At its end the thread is read, then the live answer goes.
  Stopped at its own cap, the chat says why. A run that ended with `ERROR` says what went through
  before it stopped (`failedWords`, in tafs cloud's words when it gave some), each call with what
  it moved where the Record has it (", €150.00", by KIFF's key for the call), with Retry, which
  sends its message again. Stop stops the streamed message's own chat first.
- **A run picked back up** (`resumeChat`): when the chat opens, and at each read after, a run
  tafs cloud says is at work while nothing here follows it (the owner left the chat or the app and
  came back) is followed from its first step, `GET /v1/tafs/{id}/runs/{run}/stream?after=0` at the
  stream host, the second of the two routes the session may take there. Once per run: one whose
  stream can't open is left to the reads; never one the owner stopped, while a message is on its
  way, or in a trial's chat. Its answer sits under the owner's last message, which Retry sends
  again should the run fail.

## What's drawn

- **The pin** (`pinNext`, `pinned`, `PinnedTurn`; `tafs-core/src/pin.rs`, shared with Coach): a
  message sent goes to the top of the thread at once and stays there while the taf works and its
  answer grows below it, the work block under it and the words under that. The room under it is
  the view less its turn, so nothing moves as the answer grows; it stays once the run ends, until
  the next message is pinned. What's pinned is the message just sent, matched by its words
  (`matchesSent`) among what's new since Send: never a reply that came first, nor an older copy of
  the same words. When tafs cloud's copy takes its place under another id, the pin moves to it
  without the view moving (the 859 test); gone with no copy, the view goes to the latest. One
  written to the run and one tafs cloud holds are pinned the same way. A message that couldn't go
  lets the pin go, and the chat goes to its end, as Android's does (the iPad is to follow). The
  owner scrolling (wheel, touchpad, the scrollbar) hands the view back; ↓ lets the pin go and
  follows again. `tools/pin-check.sh` checks it in the window, as the iPad's `ChatPinUITests`:
  at the top within a second of Send, still there while it works (the work block under it) and
  writes, and unmoved when the stored copy replaces it.

In the iPad's order, in a column at most 720 wide:

- **The empty chat:** "Ask Rae what it did, or tell it what to do differently…".
- **The thread** by time, each item once (`chatItems`), with a time where parts are an hour or more
  apart, and at the top:
  - a run's kickoff as a small note, "Run started 08:00 · …", two lines at most;
  - the owner's words on the right, on the brand sweep, with "Answered the card above" under words
    that answered a card; a message by a door with its stamp above it ("BY EMAIL · ana@…"), and no
    menu;
  - what the taf did before writing its words, folded under them as "Worked for 12 s ›"
    (`WorkSummary`, `WorkLog`), which opens its steps (`StepList`): each in the taf's words ("Reading
    its notes", "Checking what waits for you", "Asking KIFF to run …", the tool's name in KIFF in
    words, since this computer reads no connection's title for it from planque), tries of one step
    after a failed one as one row ("2 tries"), a failed step left out once something worked, a step
    not made "not made · read first";
  - the taf's words full width, its Markdown as formatting, with tafs cloud's lines around them:
    where the run's result went above (`delivered`), what it counted under (`counted`), each 13 soft
    as written; a door's answer with how it went back under it ("↩ SENT BACK BY EMAIL · SAME
    THREAD", red when it wasn't); a connection refused, never folded (`RefusedLines`): tafs cloud's
    words in the failure colour, then why and what next;
  - **its held calls,** as Needs you draws them, placed after the words of the turn that asks for
    them (`heldAfterItsWords`), and waiting for those words at most 45 s after they were held
    (`withheldUntilWords`);
  - **a setting it hands to Coach** (`AskCoachCardView`): its words, and Ask Coach for someone who
    may use Coach and manages the taf, which opens Coach on this taf with the card's words in the
    box, nothing sent (`askCoachAbout`, [Coach on a taf](coach.md)).
- Then the messages not sent, the dots while the taf works, what was written to its run and not
  read yet, why the latest run stopped at a cap ("Rae stopped: this task would cost more than its
  limit…", on the amber at 10%, `capWords`: `cost_cap`, `turn_cap`, `max_token`, `refused`) once
  nothing is at work, and what tafs cloud holds to go next.
- **Where it opens, and ↓:** opened, the chat lands on the taf's latest held call (`chatWaiting`), in
  the middle, unless it's among the last three items, which the newest shows; else at the newest.
  It follows what arrives while its newest is in view. Read back, ↓ over its foot goes to the
  newest, with the amber dot when something arrived below. While the held call is out of view, the
  Needs you pill names it ("Needs you · " and its summary) and shows it (`NeedsYouPill`).
- **The box:** "Message Rae", or "Write the next message" while it works; Send on the brand sweep, its
  label "Send to Rae while it works" then; Stop in the ink. Ctrl+Enter sends.

## No mic

The iPad's mic dictates on the device only (`requiresOnDeviceRecognition`), and a Linux desktop
has no on-device speech recognition, so there's no mic here; nothing of tafs sends a voice off the
device.

## Files

Attaching photos and files (`AttachViews.swift`, `tafs-core/attach.rs`, app `attach.rs`), in a
taf's chat and in Coach's alike:

- **The paperclip** at the start of the box: Choose photos, or Choose file, with this computer's
  file chooser. There's no camera to take a photo with. It holds five files a message
  (`FilePrep.perMessage`), and says so when it's full.
- **Each file is made ready as the iPad makes it** (`FilePrep`). A photo is redrawn as a JPEG on
  white, its long edge at most 2,576 px, so nothing but its pixels goes. Any other kind tafs takes
  (PDF, text, CSV, Markdown, Word, Excel) is checked against its limit before it's read (10 MB a
  photo, 25 MB a PDF, 2 MB the rest); one over it, or of a kind tafs doesn't take, says why in the
  tray and isn't added.
- **Up as soon as it's picked**: declared to tafs cloud as the session (`POST
  /v1/tafs/{id}/attachments`, its kind, name, size and SHA-256), whose answer says where its bytes
  go. They go straight to that storage address, with only the headers tafs cloud gave: no
  session, HTTPS only, no credentials in the address, no redirect followed. Nothing is signed.
- **The tray** above the box (`FileTray`, `FileChip`): each file's icon or thumbnail, its name,
  and Uploading, Ready, or why it didn't go (in red), with its remove control. Send waits while a
  file is still going up ("Send once the files are up"); files ready to go make Send sendable with no
  words.
- **With the next message**, by their ids (`attachments`); a message the send didn't take puts its
  files back in the tray.
- **In the thread** (`MessageFiles`): a message's files under it at the end, each its icon and
  name. A message of files alone has no bubble. A tap asks tafs cloud for that file (`GET
  /v1/tafs/{id}/attachments/{file}`, only the person's own), reads its bytes from the storage
  address it answers (HTTPS only, at most 25 MB), saves them in this app's cache
  (`~/.cache/tafs/opened/{id}/`) under the file's name with the extension of its kind, so the
  desktop never takes it for a program, and opens it with the desktop's app for it, as the iPad's
  Quick Look. One that can't be opened says why in the tray.

`--attach crates/tafs-app/fixtures/attach-tray.json` (synthetic) fills the tray as a screenshot
draws it: CI's `taf-chat-attach` and `coach-attach`. `taf-chat.json` sends one message with a
photo.

## Reporting an answer

Each of the taf's answers that tafs cloud has an id for (`canReport`: never the owner's words, a
kickoff, or one on its way) has a menu, on a right click or a long press, with **Report this answer**
(`reportable`). Coach's answers have the same ([coach.md](coach.md)). It opens a sheet
(`ReportAnswerSheet`, `crates/tafs-app/src/report_sheet.rs`, `tafs-core/src/report.rs`): Cancel,
"Report this answer", Send; "What's wrong with it?" and the four reasons (Offensive, Harmful, Wrong,
Other), the one chosen checked in the amber; "Add a note (optional)", with "A note is at most 1,000
characters." under one longer; "A person at KIFF reads your report, with the answer as it was
written." Send is there once a reason is chosen and the note fits, and sends, as the session,
unsigned, `POST /v1/tafs/{id}/messages/{message}/report` with `{"reason", "note"?, "session"}`: the
note trimmed and left out when blank, the answer's chat as `session` (Coach's by Coach's taf). The
answer, `{"report": {"id", "at", "new"}}`, is read as strictly as the iPad's `AnswerReport`. Sent, the
sheet says "Thanks. Someone at KIFF will read your report.", or "You've already reported this
answer." when `new` is false, with Done. Not sent, it says "Not sent: " and why, and stays open to send
again: tafs cloud's words when its refusal has them, "This answer isn't on tafs cloud any more." for a
404, else "It didn't reach KIFF. Try again.". `--report` (with `--chat`) opens it over the chat's last
answer that can be reported; its Send sends nothing.

## A held call in a streamed answer

As the iPad's `streamingItems`, `anchorHeld` and `placed` (kiff/tafs#206):

- **While it streams,** a call KIFF holds shows in the answer where it was made, after the words
  before it and those that ask for it, not by the time KIFF held it. It's known by KIFF's decision
  id, which the stream's `held` names, against the cards' (`heldKeys`).
- **KIFF is read again for this taf** (its `refresh`) when a call is held and when the stream ends,
  so the card is there without the window being drawn again: its pin and scroll stay.
- **Home is read again too** (`refresh`), when a call is held and when a stream ends, in a taf's
  chat and in Coach: the sidebar's rows, Needs you's count and NEEDS YOU are drawn anew in place,
  what's chosen stays chosen, and the screen beside it isn't drawn again (`shell.rs` ›
  `redraw_sidebar`, `main.rs` › `reread_home`).
- **Once the answer is recorded,** each held call stays after the recorded words it came after, or
  after the owner's message when it came first (`anchors`, kept while the app runs).
- **Answered,** its card becomes what became of it, in the same row (both are keyed by its Record
  line's id, `heldKey`: `recordID(proposalID ?? kiffID)`): in the answer where it was made while
  that answer streams, else by when it was answered.
- The rules are `tafs-core/src/taf_chat.rs` › `items_of`, `placed`, `anchor_held`, with tests.

## A door's knocks

As the iPad's `.knock` items and `KnockCard` in TafChatView: where Home says tafs cloud offers doors
(`doors_offered`), someone who wrote through one of the taf's doors shows in its chat at the
knock's own time, so a reply newer than it comes after it (`Item::Knock`, `Line::Knock`); a door's
line about its day's messages shows there too, with OK. The card is the Activity's
([taf.md](taf.md#its-knocks)), eight lines of the message, with Do it, Pair and Ignore, unsigned,
as the session (`POST /v1/tafs/{id}/doors/knocks/{knock}`).

- One doors model for each taf (`doors(for:)`, `crates/tafs-app/src/doors.rs` › `shared`), shared
  by its chat, its Activity's knocks and its doors' screen: an answer in one shows in all.
- The doors are read when the chat is first read, and again each time its thread's messages change
  (`.task(id: chats[tafID]?.count)`); this computer has no push, so a new knock arrives when the
  thread's messages next change, or when the chat is opened again. A knock doesn't count as the taf's words for a held card waiting for them.
- A trial's chat has none. The rules are `tafs-core/src/taf_chat.rs` › `items_of`, with tests; the
  screenshot is `taf-chat-knocks` (`--doors crates/tafs-app/fixtures/taf-doors.json` with the
  chat's fixture).

## What became of its calls

As the iPad's `OutcomeLine` in a taf's chat (`chatItems`' outcomes, `outcomeText`, `outcomeTone`,
`outcomeMark`), from the Record (KIFF's activity, `GET /v1/me/activity?limit=100`, this taf's
lines): since the chat's first message, each call approved or refused, and each call the owner
approved that wasn't done, is on its way, or whose outcome is unknown, as one line with its mark.

- "Refund order 4410: approved by you, done" (green tick) when KIFF says the reader answered it
  (`answer.you`), else "…: approved, done"; "…: refused, not sent" (grey cross); "…: sent, waiting
  for the app" (amber clock); "…: outcome unknown" (amber question mark); "…: {why}" for one not
  done (red cross). This computer answers no held call, so there's no line of its own.
- It goes where it was answered (`answer.at`), between the taf asking and what it did next; with
  no answer's time, where its card was (`made_at`, else when KIFF recorded it), after the words
  that ask for it.
- The page opens with the Record as Home read it, with its held cards; KIFF's activity is read
  again here, with its held calls, when a call is held, when a stream ends, and when a run seen at
  work here is over (`loadChat`'s `refresh`). A read that fails changes nothing; a 401
  signs this computer out.
- The rules are `tafs-core/src/taf_chat.rs` › `outcomes`, `outcome_line`, `call_amounts`,
  `items_of`, with tests. The screenshot is `taf-chat-outcomes` (`--activity
  crates/tafs-app/fixtures/taf-chat-activity.json`, synthetic).

## The sample's chat

As the iPad's sample (`loadChat`'s hello, `loadSample`, `sampleReply`, `deliverChatOnce`'s sample
branch), each sample taf's page has its Chat, and nothing is read or sent
(`tafs-core/src/sample_chat.rs`, `ChatSource::Sample`):

- **It starts** with the taf's hello: "Hi, I'm Deploys. I'm on it: {what it's doing now}. Ask me
  what I did, or tell me what to do differently. I stay inside my Card either way." Support replies
  also has the owner's "How did this morning go?" and its answer. Its held call is there as its
  card, where it was held, and the chat is kept while the app runs (the iPad keeps none across
  launches either).
- **Each message is answered** by the first of `sampleReply`'s rules that matches:
  - "stop", "pause" or "hold off" anywhere: "Paused. Nothing more goes out until you tell me to
    carry on.", and the taf is paused;
  - "go", "carry on", "resume" or "continue", the whole message or its first words: "Back to
    work.", and a paused taf carries on;
  - "more", "raise", "limit", "bigger" or "increase" anywhere: "I can't give myself more room: only
    you can, by changing my Card. …";
  - anything else: "Lately:" with its last three Record lines, then how many things wait on the
    owner in Needs you, then "(Sample reply. Signed in with KIFF, your taf answers for real.)".
- **The answer streams** as a run's does, at the iPad's pace (a beat is 0.35 s): what it's
  thinking, its notes read (`memory`), then its words, a twelfth of a beat each; then it's kept
  with what it worked on ("Worked for 2 s"). A second message while it writes stays in the box.
  There are no files, no Stop and no steering in the sample, as on the iPad.
- **Paused or carrying on** shows at once on the sidebar's row ("Paused by you", "Back to work");
  the taf's page shows its state when it's opened again.
- `--page taf-sample-chat` opens Support replies on its chat, CI's `taf-sample-chat`; a
  screenshot answers at once. `--page taf` stays on Activity, as the iPad's `-screen taf`.

## Not yet

- `SteadyMarkdown`.
- Keeping across launches
  which unread rows were dealt with.

## Screenshots

`tools/screenshots.sh` saves `taf-chat` (`crates/tafs-app/fixtures/taf-chat.json`: a scheduled run's
report, a question and its answer, a setting handed to Coach, a message by email and its answer, a
connection refused, a message a stopped run never read, and the cost cap) and `taf-chat-working`
(`taf-chat-working.json`: at work, with Stop, the dots, a message the run hasn't read and one tafs
cloud holds), `taf-chat-stream` (`--chat-stream taf-chat-stream.txt`, a stream's `event:`/`data:`
lines taken after `taf-chat.json`'s thread: the answer's words so far, its step running and the one
done before) and `taf-report` (Report this answer over the chat). Both are synthetic, shaped as tafs cloud's `coach-conversation.json` and the iPad's
`TafsCloud.Thread`: there's no contract for a taf's own thread at the pin.
