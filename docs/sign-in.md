# Signing this computer in

The iPad's sign-in, on Linux: its welcome (`OnboardingView`), and Sign in with KIFF (`SignInView`)
over it as a dialog, the iPad's sheet. The flow is KIFF's device authorization, the one the phones
use (KIFF's desktop client contract §1.1). The logic is in `tafs-core/src/signin.rs` and tested
there; the screen is `crates/tafs-app/src/sign_in.rs`.

## The sign-in: two halves

The approved redesign (tafs#1240's `SignInView` for an iPad or a Mac), a dialog 1000 wide:

- **On the left, the usual way** (`kiffHalf`): board 1 (first run: a card opening, sparkles round
  it, `illustrations.rs`), as on every app's sign-in, "Sign in with KIFF" (30
  bold), and **Continue** (the brand's button, 320 wide). Continue starts the flow below, and its
  code and its steps show in its place, at most 420 wide. The terms line is under it.
- **On the right, the other way** (`phoneHalf`), on the raised colour: this computer's
  [pairing](pairing.md), its QR shown at once, "Sign in with your phone", "Scan with tafs" with
  a viewfinder, where to scan it (`sign_in.pair_how`), the check code in its panel with "It will
  ask whether this computer shows {code}." (`sign_in.pair_check`), and the wait.
- **One way at a time**, as the Mac and Windows apps: Continue stops the phone's half (its claim
  ends, and a session KIFF hands over after that is ended at KIFF) and puts **Show the code**
  (`sign_in.show_code`) where its QR was. Show the code, or a new code or Try again there, stops
  KIFF's code the same way and puts Continue back. So two sign-ins never land together, for two
  accounts, with no sign of which one won. `--page sign-in-code` shows the phone's half put aside.
- **A narrow window** (under 920, both halves' least) stacks them, the phone's under KIFF's, and
  scrolls.
- **Either way ends on the celebration** (`ConnectedView`, `crates/tafs-app/src/connected.rs`),
  over the account just opened: the computer and the phone joined, a check between them; "You're
  connected." and "Let's build great things together." (30 bold); **Let's go**, which closes it.
- **The drawings are shapes only**, drawn here: no letter, number or word (rule 14). Every word is
  a shared key.
- Cancel closes it, and stops both: the browser's wait and the pairing's claims.

`--page sign-in` draws it with nothing reached; `--page sign-in-code` with Continue tapped and a
made-up code; `--size 700x900` narrow; `--page connected` the celebration (CI's `sign-in`,
`sign-in-code`, `sign-in-narrow`, `connected`).

## The welcome

Board 1, first run (a card opening, in shapes: [illustrations](app.md#illustrations)), "tafs" (48 bold), the lead (16, soft), and its three points: Inside the Card,
Outside it, Revoked, each with a dot in green, amber and red. The welcome scrolls in a short
window.

**The one tick** (K5, kiff/tafs `docs/onboarding.md` step 1; `OnboardingView`, `ConsentTick`):
"I agree to KIFF's Terms of Service and the tafs Terms", each name a link to its page in the app's
language (KIFF's at `kiff.dev/<lang>/terms`, tafs's at `kiff.dev/<lang>/tafs/terms`). Under it,
privacy as two links, never ticked, and the age line: this computer's country's own age where the
law asks more than 16 (`onboarding::age_line`, from `ages.json` at the pin and the region of
`LC_ALL`, `LC_MESSAGES` or `LANG`), else the line every country reads. The box is its own control,
so a link opens its page and never ticks it.

At the foot, **Continue** (the brand's button), off until the box is ticked, and Try it with
sample data (the outlined one), which never waits on the tick and opens the [sidebar](home.md)
over the sample. Under them, "Already use tafs on your phone? Add this computer with it.", which
opens the [pairing](pairing.md), also waiting on the tick.

## Signing in inside the app (Continue)

`tafs-core/src/app_signin.rs`, `crates/tafs-app/src/app_sign_in.rs`; KIFF's contract is
kiff/cloud's `api/internal/appsignin/README.md`.

1. **The terms version** in force is read as the welcome shows (`GET /v1/auth/app/terms`), so the
   version sent is the one the person saw. With none, Continue says signing in didn't work, and
   nothing opens: consent is never skipped.
2. **Continue opens KIFF's page in the person's browser:** `/oauth/authorize` with
   `client_id=tafs-app`, `redirect_uri=dev.kiff.tafs:/oauth`, a fresh S256 challenge over a
   43-character verifier, a fresh 256-bit `state`, and `ui_locales` where KIFF's page speaks the
   app's language. There's no private browser on a computer: the browser's own cookies come in,
   and KIFF's rule that the person acts on its page for each request is the guard (kiff/cloud#23).
3. **The way back:** KIFF sends the browser to `dev.kiff.tafs:/oauth?code=…&state=…`. The desktop
   hands it to tafs through the `.desktop` file's `x-scheme-handler/dev.kiff.tafs`. Run plainly,
   tafs is one app on the desktop (`HANDLES_OPEN`), so a second launch hands the link to the window
   that waits for it. **A window started with options** (`--lang fr`, a screenshot) is its own
   (`NON_UNIQUE`) and never hears the link: to try Continue by hand, start tafs with no options.
   GIO hands the link to `open` byte for byte (`app_sign_in` tests it through
   `g_file_new_for_commandline_arg`). Only this attempt's link is taken: `dev.kiff.tafs:/oauth` exactly, its
   `state`, each field once. Any other link is dropped, not even read as a refusal.
   - **An AppImage** has no install step, so on its first run it writes its own entry to
     `~/.local/share/applications/dev.kiff.tafs.desktop` (its `Exec` is the AppImage where it
     is, escaped as the Desktop Entry spec asks; a path with a control character isn't written)
     and asks `xdg-mime` to make it the scheme's handler (`scheme.rs`). An entry a package
     installed is left alone.
4. **The token:** `POST /v1/auth/app/token` with the code, the verifier, the terms version and
   the device exactly as `/v1/auth/device/start` sends it (`start_body`). The session is kept as
   the device sign-in keeps one (`sign_in::keep`), then the celebration, then the three cards.
5. **How Continue ends otherwise:**
   - declined on KIFF's page (`access_denied`): nothing is said;
   - signing in inside the app is off (`unsupported_client`): "Signing in inside the app isn't
     on yet.";
   - the terms changed since the tick (409 `terms_outdated`): the box is unticked, "The Terms
     changed since you ticked the box…", and the new version is read. The code is kept for 50 of
     its 60 seconds, so the new tick sends it again without opening KIFF's page twice;
   - anything else: "Signing in didn't work.", the tick kept, Continue free again.
   - A browser closed with no answer leaves the welcome as it is, Continue free: a computer has
     no "came back from the tab" signal.
6. **The phone's way** carries the same terms version on its claim (`terms_version` on
   `/v1/pair/claim`, `pairing::claim_body_with_terms`). Terms that changed close the pairing and
   untick the box, as Continue does.

## The first Home

Signed in with no taf, for someone who may use Coach, the Overview shows the first Home under
board 2 (the iPad's `HomeView` 137–152, on every desktop):
tafs cloud's own empty line when it sends one, else "No tafs yet" and what Coach does; **Ask
Coach**, a new conversation (`startNewTaf`); and **How tafs works**, the cards and the legal pages
(`overview::with_first_home`, `overview.rs` › `first_home`).

## The three cards

After the celebration, once on this computer (kept in the user's config), and again from
Account › How tafs works (`onboarding.rs`, the iPad's `OnboardingCards`, `HowTafsWorksView`):
Your tafs (the sample's three tafs that move no money, as Home lists them), A Card is your
permission (the sample's Support replies Card, 12 of 40 used today, issued by you, drawn by the
Card tab's own face), You stay in control (Needs you's card over the sample's Deploys). Skip, Next
and Get started; Escape is Skip. The last card names how this computer approves:
`onboarding.control_lead_desktop` (from tafs#1710, in all 40 shipped languages). How tafs works lists the three cards, then, as the
iPad's, "tafs can work in your Chrome" (`help.chrome_*`) once planque lists Chromes for this
person, and "Sign in with a code sent by email" (`help.code_*`) once KIFF says it's available,
each read when the page opens (`chrome::offered`, `code_sign_in::available`; a failed read shows
neither), then "Approve with a security key on this computer" (`help.security_key_title`, `_lead`, `_use`, from
tafs#1848): approving here needs a FIDO2 key with its own PIN, added when this computer is
paired, and without one this computer only views. Then the four legal pages, KIFF's and tafs's,
each titled as its page is. The page scrolls when it's taller than the dialog. `--page how-it-works` with `--chromes` and
`--account-reads` draws both (CI's `how-it-works-help`).

## The flow

1. **Start** (`startDeviceSignIn`): `POST /v1/auth/device/start`, no session:
   - `public_key`: this computer's device key, standard base64 of SPKI DER, P-256;
   - `"approval": "passkey"` in place of an approval key, which Linux doesn't hold, and
     `"key_store": "software"`, since the key isn't in hardware (see [keys](keys.md));
   - `device_name`: "tafs on this computer", in the person's language (`pair.device_name`);
   - `platform: linux`, `kind: other`.
2. **The code:** the answer's link must be https on KIFF's own domain (`kiff.dev` or a host under
   it), on the default port, with no user in it; anything else refuses the whole answer. The link
   gets the app's language (`lang=`) unless it names one (`inAppLanguage`). The device code is never
   printed.
3. **The browser:** Open KIFF to approve opens the link; the person signs in, checks the code and
   approves. Copy puts the link on the clipboard, for another browser.
4. **The session** (`finishSignIn`): `POST /v1/auth/device/token`, waiting the interval (at least
   2 s) before each try, 5 s more on `slow_down`. A dropped connection, a 5xx or a 429 asks again;
   coming back to the window from the browser asks at once (`nudgeSignIn`). The code running out,
   `expired_token`, or another refusal stops it.

The token is handed over once and never printed. Once signed in, the window opens the sidebar,
with the view-only notice when KIFF's `approval_mode` says `view` (or says nothing this app knows).

## What the person reads

| Step | Shows |
| :- | :- |
| Starting | a spinner |
| Waiting | YOUR CODE in its amber-edged panel, Open KIFF to approve, the link with Copy, "Waiting for your approval…" |
| Stopped | why, in plain words by status (`PlainError`): offline, busy, down, or "The sign-in code expired. Start again.", and Try again |

Under them, the terms line, Terms and Privacy policy linked in the app's language (`kiff.dev/<lang>/tafs/terms`,
`/<lang>/tafs/privacy` for de, es, fr, it, nl, pl and pt), and in English, saying "(in English)",
for any other language (`kiff.dev/tafs/terms`, `/tafs/privacy`). Account's Legal rows do the same. Cancel closes it and stops the poll.

## The kept session

Signed in, the session is kept in the person's keyring (the Secret Service: GNOME Keyring, KWallet,
KeePassXC), as the iPad keeps it in the Keychain, and read back before the window moves on. A
keyring that refuses signs the new session out at KIFF (`POST /v1/auth/device/logout`) and says
so, so no device is left at KIFF that this computer forgot.

- **What:** the token, tenant, subject, `approval_mode`, its expiry (90 days unless KIFF says),
  and the device key it's bound to. Never printed.
- **Read:** only for KIFF's exact API host, only until it expires; an unreadable one reads as none.
- **Where its token may go** (`session::may_send_to`): https only, on exactly `api.kiff.dev` or
  `cloud.tafs.kiff.dev`, on the default port, with no user in the link. Redirects aren't followed.
- **`approval_mode`:** `key` and `passkey` as KIFF says them; anything else (none, `device_key`, an
  unknown word) is view.
- **A session this one replaces** signs itself out at KIFF first, best effort, so its device
  doesn't stay there out of reach.
- **A session handed over after the sign-in closed** is never kept: it's signed out at KIFF.
- **KIFF ending the session** (a 401 on the account, Home or the held calls: the computer removed
  in Devices, the account gone): the keyring forgets it and its key, and the welcome opens, saying
  "Signed out of KIFF. Sign in again."

## Tests

`cargo test -p tafs-core` (`signin`, `session`, `keys`, and `tests/at_the_pin.rs` for every
line in the 8 languages): the start's body, the link check, the language, the poll's waits and
slow-down, expiry and refusals, the modes, a relaunch, the exact hosts. Nothing here reaches KIFF.

## Not yet

- **A passkey** that approves for this computer (contract §2.6): its own change, with KIFF's
  Linux origin.
