# Halting

The iPad's `HaltScreen` and `HaltRow` (`HaltScreen.swift`), and `TafsCloud.Halt`, on Linux. The
read and the lines are `tafs-core/src/halt.rs`, tested against canned answers; the screen and the
row are `crates/tafs-app/src/halt.rs`.

## What it does

A halt of one taf or, with none, of every taf. **Halting is one tap, with no signature**: it
only takes authority away. `POST` to tafs cloud's `/v1/tafs/{id}/halt` (a taf, by tafs cloud's
id as one path segment) or `/v1/halt` (every taf), as the session, with `{}`. tafs cloud halts it
in kiff/agents and KIFF at once. It reads **Halted** only once all three parts hold it:

- kiff/agents: no run starts;
- KIFF's gateway: it refuses the calls;
- planque: it sends nothing to the apps.

Until then it reads **Halting…**. While it's halting, the screen reads it again every 2 seconds,
at most 60 times (`GET` the same path). A read that can't be read keeps what was there.

**Resuming gives authority back**, so it's signed by the device (`tafs.resume.v1`). This
computer's key is in software and signs nothing, so its button is there, dimmed: "Resume Rae", or
"Resume all your tafs" (the iPad's prompt words, as Linux has no biometry to name), with "This
device can't approve. Approve on another device signed in to your account." under it. A taf
halted with the whole account has no button: it resumes with the account.

## The screen

Beside the sidebar:

- for Halt all once every part holds it, board 4 (cards at rest under a pause, in shapes:
  [illustrations](app.md#illustrations));
- a panel with the taf's name (or "All your tafs") as an eyebrow, dim while running and red
  otherwise; the headline (Running, Halting…, Halted; 26 bold); and what it means (15, soft):
  "Halting stops it/your tafs until you resume…", "Nothing new starts…", or "Since 14:05.
  Resuming starts only new work." in the account's zone;
- once it's halting or halted, **Who holds the halt**: each part with a check (green) or dots, and
  Done or Waiting; "Halted with all your tafs…" for a taf halted with the account;
- **Halt now** (or **Halt all now**) in red, while it runs; while it's halting, a spinner,
  "Halting… waiting for every part to confirm.", and **Halt again**, which asks the parts that
  didn't take it; halted, the dimmed resume;
- why it couldn't halt, in red: "tafs couldn't halt it. {why}", KIFF's or tafs cloud's words.

## Where it opens

- **Halt all**, in the sidebar after the screens (`showsAccountHalt`): signed in with KIFF, for
  someone who manages Cards (`cards.manage`, none said counting as allowed), while a tafs cloud
  taf runs, and whenever the account is halted or halting, since that's where it's resumed from.
  Its hand is red; while it isn't running, its state is in red under it. It's read with the
  account.
- **Halt**, at the foot of a taf's Card (`HaltRow`, framed): for a tafs cloud taf this person
  manages that isn't revoked. It reads its own state when it's drawn.

## Screenshots

`--halt crates/tafs-app/fixtures/halt-halting.json` or `halt-halted.json` (synthetic), with
`--home` and `--page halt-all` or `--page halt:<id>`, draws the screens from that answer: every
read is the file's, and **nothing is ever sent**. CI's `halt-all`, `halt-all-halted` and `halt-one`.

## Not yet

- **Resuming** from this computer: with passkey approvals.
- The list read again after a halt (`refresh()`): the sidebar's row reads its new state at the
  next read of the account; the screen itself is current.
