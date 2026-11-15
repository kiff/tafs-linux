# Adding this computer with a phone

How a Linux computer gets a session that may ask for approvals: a phone that already approves
signs this computer's key. The iPad has no computer side to port (it has the phone's side,
`PairComputerView`, not for Linux); this is tafs-macos's pairing (`docs/pairing.md`,
`Messages.swift`, `Signing.swift`, `Client.swift`, `SignInModel.pair`, `SignInView.pairing`,
`QRCode.swift`), after its #25 and #30, against KIFF's desktop-client contract §5. The logic is
`tafs-core/src/pairing.rs`, tested there; the screen is `crates/tafs-app/src/pairing.rs`.

## Where it opens

- **The welcome**, under its buttons: "Already use tafs on your phone? Add this computer with it."
- **The view-only notice** beside the sidebar: **Add this computer**. The pairing replaces the
  view-only session (below). Once paired, the notice has no button and says approvals go to the
  phone ([home](home.md)).
- `--page pair` opens it over the welcome; with `--screenshot` its code is made here and never
  claimed.

## The code (`pairing::Payload`, `check_code`)

1. **The keyring first.** Before a code shows, the keyring is asked to take a session
   (`store::check_writable`, macOS #30's `SecretStore.checkWritable`): the session kept is written
   over itself and read back, or, with none, an empty item is written, read back and deleted. If
   it refuses, the pairing stops there: "This computer wasn't added: Secret Service. Show a new
   code and try again.", with no code and nothing for the phone to approve.
2. **A new device key and a new nonce** (32 random bytes, base64url) for each code. KIFF refuses a
   pairing for a key that's already a device's, so a view-only computer never pairs its old key.
3. **The QR code's text**, `kiff.pair.v1`, keys sorted, slashes as they are:
   `{"exp","key_store":"software","kind":"passkey","name","nonce","platform":"linux","pub","v"}`.
   `pub` is base64url (no padding) of the key's SPKI DER; `exp` is RFC 3339 UTC to the second, 5
   minutes ahead; `name` is "tafs on this computer" in the person's language. Linux's key is in
   software, so the code says so and carries **no approval key**: KIFF never lets that key approve.
4. **The QR code** is the text as bytes (byte mode), error correction M, black on white with a
   white margin, drawn as squares so it stays sharp at any scale. A test reads it back from the
   image, as a phone's camera would.
5. **The check code**: the first 6 characters of base32 (RFC 4648, no padding) of SHA-256 of the
   key's SPKI DER. The phone shows the same 6 before it approves.

## The claim (`pairing::claim`, `wait`)

`POST /v1/pair/claim` with `{"nonce","pub","proof"}`; `proof` is the key's signature (ECDSA
P-256, DER, standard base64) over `kiff.pair.claim.v1\n{nonce}\n{sha256 hex of the SPKI DER}`.

- It waits 3 s before the first claim and between claims. 400 `authorization_pending` claims
  again; 400 `slow_down` adds 5 s to the wait for good; past the code's 5 minutes it ends as
  expired without claiming again (never more than 10).
- Any other answer ends the wait: 400 `expired_token` is "This code expired." with **Show a new
  code**; anything else is "This computer wasn't added: {why}. Show a new code and try again."
  with **Try again**, where why is KIFF's own code (`invalid_proof`) or the app's line for what
  happened (offline).
- The answer is the session (`access_token`, `expires_in`, `tenant`, `subject`, `device_id`). It
  is kept **view-only** whatever the answer says (`pairing::kept`): this computer's key is in
  software, and KIFF lets a software key approve nothing. KIFF answers `view` for a software key;
  this keeps it so whatever the answer says. It is also kept as paired
  (`KeptSession.paired`), so the notice says approvals go to the phone, with no button. The view-only notice is then drawn from KIFF's `/v1/me` (`approval_mode`), as for any
  session.

## The session

- **The new session and its key are kept first**, read back from the keyring. If the keyring
  refuses then, the new session is signed out at KIFF and the pairing says why.
- **Then** a view-only session it replaces is signed out at KIFF, with its own token.
- If the pairing doesn't happen (the code runs out, KIFF refuses, the person cancels), nothing is
  kept and nothing is signed out: the computer stays as it was.
- **Cancel** stops the claims. A session handed over to a pairing that closed is signed out at
  KIFF at once, never kept.
- No request, answer, token or signature is logged.

## The screen

| Step | What it shows |
|---|---|
| The code | "Add this computer with your phone" (24 bold), the QR code (220, on white, rounded), "On your phone, open tafs, then Account, Devices, Add a device. Scan this code.", "Your phone asks if your computer shows this code:", the check code (mono, bold), and "Waiting for your phone…" |
| It ran out | "This code expired." and **Show a new code** |
| It didn't work | "This computer wasn't added: {why}. Show a new code and try again." in red, and **Try again** |

Cancel is at the top left, as the sign-in's.

## The network

The app's HTTP goes through the proxy the desktop names (`HTTPS_PROXY`) and trusts this
computer's certificate authorities, so a network that requires a proxy reaches KIFF too.

## Tests

`cargo test -p tafs-core pairing`: base32 against RFC 4648's vectors, the check code, the code's
exact text, the nonce, the proof verified with the public key, the claim's body, every answer the
contract names, a paired session kept view-only whatever the answer says, the wait's pace, its end with the code, and a cancel. No test reaches the network.
