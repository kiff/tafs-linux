# Signing this computer in

The iPad's sign-in, on Linux: its welcome (`OnboardingView`), and Sign in with KIFF (`SignInView`)
over it as a dialog, the iPad's sheet. The flow is KIFF's device authorization, the one the phones
use (KIFF's desktop client contract §1.1). The logic is in `tafs-core/src/signin.rs` and tested
there; the screen is `crates/tafs-app/src/sign_in.rs`.

## The welcome

The mascot (96), "tafs" (48 bold), the lead (16, soft), and its three points: Inside the Card,
Outside it, Revoked, each with a dot in green, amber and red. At the foot, Sign in with KIFF (the
brand's button) and Try it with sample data (the outlined one), which opens the
[sidebar](home.md) over the sample. Under them, tafs-macos's line: "Already use tafs on your
phone? Add this computer with it.", which opens the [pairing](pairing.md).

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

Under them, "Sign in with your other phone" (`PairSignInView`): this sign-in stops (its wait for the
browser ends, as the iPad's `task?.cancel()`), and this computer's [pairing](pairing.md) opens: the
QR and its check code, for the person's phone to scan and approve. Then the
terms line, Terms and Privacy policy linked in the app's language (`kiff.dev/<lang>/tafs/terms`,
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
