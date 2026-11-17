# A taf's chat

The iPad's taf chat (`TafChatView`), on a taf's page's Chat tab, for a taf tafs cloud runs, signed in
with KIFF. The page opens on it, as the iPad's does; the sample's Chat is there, dimmed. What it reads,
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

The thread is read when the Chat tab first shows, then every 4 s while the taf owes an answer (the
owner's message is the thread's last) or works, at most 30 times for each message of the owner's
(`followChat`'s plain reads), except while an answer streams, which reads it itself. One read at a
time, and none once the pane is gone.
What the chat holds (the thread, what was sent and not sent, the stop) is kept for each taf while the
app runs, and signing out forgets it, with the box's words.

## What's sent

All at tafs cloud, as the session, unsigned; a 401 on any of them signs this computer out.

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
  first of them: the live line and the steps done. A message the run took, or a run that said
  something, is read from the thread. At its end the thread is read, then the live answer goes.
  Stopped at its own cap, the chat says why. A run that ended with `ERROR` says what went through
  before it stopped (`failedWords`, in tafs cloud's words when it gave some), with Retry, which
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
  without the view moving (the 859 test); gone with no copy, the view goes to the latest. A
  message not sent, one written to the run and one tafs cloud holds are pinned the same way. The
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

## Not yet

- `SteadyMarkdown`.
- **A held call made in a streamed answer** is placed where Needs you's list places it, not where
  it was made in the answer, and KIFF isn't read again at once when it's held or the run ends;
  what went through before a failure has no amount from the Record.
- What became of a held call the owner answered (`OutcomeLine` from the Record: KIFF's answer time
  isn't read here), and a held call's place from the stream (`anchors`).
- A door's knocks among the messages (they're on the Activity tab), and keeping across launches
  which unread rows were dealt with.
- The sample's chat (`sampleReply`).

## Screenshots

`tools/screenshots.sh` saves `taf-chat` (`crates/tafs-app/fixtures/taf-chat.json`: a scheduled run's
report, a question and its answer, a setting handed to Coach, a message by email and its answer, a
connection refused, a message a stopped run never read, and the cost cap) and `taf-chat-working`
(`taf-chat-working.json`: at work, with Stop, the dots, a message the run hasn't read and one tafs
cloud holds), `taf-chat-stream` (`--chat-stream taf-chat-stream.txt`, a stream's `event:`/`data:`
lines taken after `taf-chat.json`'s thread: the answer's words so far, its step running and the one
done before) and `taf-report` (Report this answer over the chat). Both are synthetic, shaped as tafs cloud's `coach-conversation.json` and the iPad's
`TafsCloud.Thread`: there's no contract for a taf's own thread at the pin.
