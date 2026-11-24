# Changelog

What changed in each release of tafs for Linux, newest first. Each release's notes, in its eight
languages, are in `packaging/release-notes/` and on its GitHub release page.

## v1.1.0

**Signing in and languages**
- Sign in inside the app, with KIFF's page in the browser and the way back to the app, or with a
  code sent by email (Account › Sign-in). The first run shows how tafs works.
- 43 languages, with Arabic and Hebrew right to left; Brazilian Portuguese opens its own legal pages.

**Approving on this computer**
- A USB security key approves what this computer may: making, renaming and changing groups,
  signing a device out, a held call's Approve once and Extra room, an API door's Reopen, Revoke
  and Remove, a trial's Start trying and Install, removing a taf (its Cards revoked first) or a
  passkey, and ending or clearing a connection.
- On a held call, Allow this from now on (`AllowSheetView`): the taf's Card changed only as tafs cloud
  offers and only if KIFF's draft is exactly that, approved once with the key.
- Raise the limit and Decline a member's request (Plan & usage): the security key checks the person,
  by touch and PIN, for that very request, and only then does this computer's device key sign it.
- Anything that gives a taf access to your apps (connecting, keys, Coach's tools, a subscription,
  a drive) stays on your phone, and the screens say so.

**Connections and doors**
- Connect apps lists the account's connections, read from planque with this computer's device
  key, each with its last failed call, the other tafs whose Card uses it, and what removing it
  stops. A grant's digest is checked as planque computes it, and KIFF's words for a connection's
  profile are shown on its approval.
- Coach's approval cards show the connection they ask for; "Taking shape" lists the tools it
  connects; a step names a connected tool by its connection's title.
- Doors: a WhatsApp door's signup from this computer (a one-use `state`, taken back only for that
  taf within 15 minutes); a Slack door on the taf's own bots and channels; a door "From another
  application", with its keys and instructions (new keys are made on the phone).
- Account › Chrome: the Chromes paired with this account, Unpair, and the picture before a Chrome
  step, from a receipt.

**Coach**
- The dial card, the taf on screen offered to improve, Decline and Not now on its cards, the loop
  card, the days you asked from your watch, an answer revealed when there's no stream, and Coach
  mode's slide and swipe.

**Home, a taf's chat and the rest**
- The sidebar puts first what waits on you, including a connection its vendor signed out; pull to
  refresh (or F5, Ctrl+R) on Home, Plan & usage, a taf's runs and Setup.
- A taf's chat: what became of its calls, their amounts, knocks at its doors, the sample's
  answers, answers shown at an even pace, and messages you dealt with kept dealt with across
  launches.
- Files for the account and each taf; Report this payment; Needs you offers alerts while they're
  off; the Overview's lines; a taf's head that folds.
- Reads back off and wait while a screen isn't seen, so many open apps don't read in step.
- tafs is under the TAFs Functional Source License.

## v1.0.1

- Coach's and a taf's answers appear as they're written.
- Coach on a taf's page; a new version's card opens that taf's Versions inside Coach.
- A taf's Activity shows each app's calls, and Ask Coach on a run.
- A message that didn't send: Copy, Delete, or Retry, which sends that same message.
- Your message stays at the top while the answer comes in; the window opens sized to the screen.
- When a new version is out, a line in the sidebar says so and opens its release page. The app
  downloads nothing.

## v1.0.0

- The first release: tafs on Linux as one AppImage, with your tafs, Needs you, the Record and
  Coach. Approvals stay on your phone.
