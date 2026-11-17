# Approving on this computer

The iPad's approval key signs only after Face ID or Touch ID (`Biometry.swift` ›
`ApprovalReadiness`). Each state has its own words: none enrolled, no passcode, turned off for
tafs, locked or not available now, and no Face ID or Touch ID at all. `tafs-core/approval.rs`
ports each of them with its words, `path`, and when Open Settings and Check again help, and
tests them in every shipped language.

This computer has no Face ID or Touch ID, and its key is in software, so it is always
`Unsupported` (`THIS_COMPUTER`). It can never approve, and checking again can't change that.

## In an approval's place (`ApprovalSetupView.swift`)

- **`ApprovalGate`** (`approval::gate`, app `approval::gate`): signed in with KIFF, while the
  device can't approve, `ApprovalNotReady` stands where the approval's button would be. The
  sample signs nothing for real, so it keeps the button.
- **`ApprovalNotReady`** (`approval::not_ready`): the state's short line (14, ink). Where it
  applies, it adds the note that a new face or fingerprint ends the approval key (12, dim), Open
  Settings, and Check again. For this computer, that's the line alone: "This device can't
  approve. Approve on another device signed in to your account."
- **`ApprovalNotice`** and `approvalBlocked` (`approval::notice`, `approval::approval_blocked`):
  the same words for a button that stays where it is, disabled.

Linux draws it in place of "Did this happen?"'s No and Yes (`docs/needs-you.md`).

## A view-only computer

Linux never signs. For each act the iPad signs, this computer draws the iPad's button, and the
sheet the iPad opens before signing, with its fields. The tap that would sign shows
`ApprovalNotReady`'s words where the iPad says why an act didn't go, and nothing is sent
(`tafs-core/src/signed.rs` › `tap`, app `approval::refuse`). The sample, which the iPad never
signs for, does nothing on that tap.

`signed::signed` says which calls KIFF takes only signed, as the iPad's `KIFFClient` lists them:
- `signsSetup`: a taf's draft, alert targets, keys, the time zone, deleting the account, raising
  the limit, an approval window, groups and sharing, declining a request, "Did this happen?",
  resuming, and confirming a draft or ending its trial;
- `grantsAuthority`;
- the session's approval key and its relink;
- what the iPad signs beforehand: a device signed out, a computer's pairing, a passkey.

A call this computer makes as the session is a `signed::Unsigned`, which can't be made for any of
them. The route is matched as KIFF matches it, without its query, fragment or a trailing `/`, and
a path that isn't plain (`//`, `/./`, `/../`) counts as signed, so no spelling of a signed route
can be made unsigned. Each signed act (`signed::Act`) names the call the iPad would make, and the tests check
that none of them can be made.

The acts drawn this way: People & groups' changes, signing another device out, and the
account's time zone (`docs/account.md`); declining a request (`docs/usage.md`); connecting an app
once its taf is picked (nothing is started, so nothing waits at planque), removing an app's
access, cancelling its request or sign-in, clearing a removed taf's connections and signing in
again, which are planque's (`docs/connect-apps.md`).

A button the iPad itself wraps in `ApprovalGate` is the exception: there, as on the iPad, the
gate's line is drawn in the button's place (`approval::gate`), not the button. Raise the limit is
one (`docs/usage.md`); so is a connection's approval (`docs/connect-apps.md`).

Not on Linux: `ReadinessReader` reads again when the app comes back, but this computer's state
never changes. `openTafsSettings` opens tafs' iOS settings, which don't exist here, and
`Unsupported` offers neither button anyway.

## Approving with a passkey (switched off)

**The first release approves nothing on Linux.** A Linux computer is view-only: it's added with
the phone, and every approval goes to the phone. Approving here with a USB security key (a FIDO2
key with a PIN), as kiff/tafs `docs/computer-passkeys.md` describes for a computer with no key
store of its own, waits on a contract decision. The parts that don't depend on it are in
`tafs-core/src/passkey.rs`, switched off (`ENABLED` is false), so nothing asks a key for
anything:

- **The message** a device key would sign. One request is `kiff.request.v1`: the tenant, the
  method, the path with its query, the body's SHA-256 in hex, and `signed_at`. Two requests
  signed together are `kiff.requests.v1`. The drive doc's test vector is a test here.
- **The challenge**, `base64url(sha256(message))`.
- **The client data**, `{"type":"webauthn.get","challenge":…,"origin":"https://kiff.dev","crossOrigin":false}`,
  these exact bytes, in this order.
- **The checks before sending**, refused in this order:
  - authenticator data that can't be read;
  - a relying-party hash that isn't `sha256("kiff.dev")`;
  - no user presence, or no user verification;
  - a credential other than the one asked for;
  - a passkey that is backup eligible or backed up.
- **The header**: `KIFF-Passkey-Assertion: t=<signed_at>,a=<base64url JSON>`. Two requests
  signed together add `,with=sha256:<the other's hash>`.
- **`signed_at`**: RFC 3339 with milliseconds, in UTC, and never the same twice.

The tests sign as a key would (`authenticatorData ‖ sha256(clientData)`, P-256) and verify each
header as KIFF does.

The key's side without the key is in `tafs-core/src/security_key.rs`, ported from macOS 12's
`SecurityKey.swift`. The app will ask the key through libfido2 with exactly these, as macOS 12
does:

- **What's asked of the key** (`AssertionAsk`, `CredentialAsk`): a 32-byte client data hash and
  a credential id of 1 to 1023 bytes, refused before the key is asked. A new credential is
  kiff.dev ("KIFF"), ES256 only, not discoverable, with a user id of 1 to 64 bytes. Presence is
  asked, and `uv` is left out because the PIN verifies the person (some CTAP2.0 keys refuse
  both). The UV flag is still required in the answer. The key waits two minutes for its touch.
- **Why the key gave no answer** (`Failure::of`, libfido2's codes from `fido/err.h` 1.17.0):
  - a wrong PIN, with the tries left;
  - blocked (including a wrong PIN with no tries left), or paused after three in a row;
  - no PIN set;
  - another key;
  - not touched in time;
  - denied;
  - unplugged;
  - anything else, by its code.

  The app adds no key, several keys, a U2F-only key, and a PIN missing, too short, too long or
  already used.
- **The PIN** (`Pin`): taken from the bytes typed, which are cleared whether it's taken or
  refused. It needs at least 4 characters and at most 63 bytes, with no NUL. It's handed to the
  key once as a C string, then cleared, even if the call panics. It's also cleared when dropped,
  and it never prints.
- **A new credential's attestation object**: `fmt`, `attStmt` and `authData`, in CTAP2's
  canonical order.

The key itself is `crates/tafs-app/src/fido_key.rs`, ported from macOS 12's `FidoKey.swift`.
It asks the one key plugged in through libfido2, with exactly those asks:
- **It loads the system's `libfido2.so.1` at runtime**, the first time a key is asked
  (Debian and Ubuntu's `libfido2-1`, Fedora's `libfido2`, 1.14 or later for the raw
  authenticator data). macOS 12 builds the library from pinned source instead. This way the app
  builds and runs without it, and a computer without it says so (`Missing::Library`,
  `Missing::TooOld`).
- **It starts libfido2 once**, with no U2F fallback. Its log goes to a handler that says
  nothing, set after `fido_init` (which sets its own when `FIDO_DEBUG` is set), so the frames
  sent to the key are never printed. The environment is left as it is, since changing it while
  other threads read it isn't safe.
- **It opens one key only**, with a PIN set. No key, several keys, a U2F-only key or a key
  without a PIN is refused before anything is asked of it. The key waits two minutes for its
  touch.
- **It asks only two things of the key:** `fido_dev_get_assert` and `fido_dev_make_cred`. It
  reads the retry count after a wrong PIN and for the PIN sheet. It never resets the key, and
  never sets or changes its PIN.
- **The PIN goes to the key once**, and is cleared whether or not the key was reached.
- **A request can be cancelled at the key** (`fido_dev_cancel`) from another thread. An
  `OpenKey` is one request, as macOS 12's `run` makes a new one each time. Once cancelled, it
  stays cancelled.
- **The answer is raw:** the authenticator data, the signature and the credential used, or the
  attestation object built from libfido2's three parts. `passkey::refusal` checks it whatever
  libfido2 says.

Nothing calls it while it's switched off. Its tests ask with no key plugged in: where libfido2
is installed it finds none, and where it isn't, it says it's missing. Either way, no PIN is left
held.

**The PIN sheet** (`crates/tafs-app/src/pin_sheet.rs`) is ported from macOS 12's `PINSheet.swift`,
with its words from `SecurityKeyWords.swift` (`security_key::words`). It shows:
- the title ("Approve with your security key", 15 semibold) and what it asks (14, soft);
- the tries left, read from the key before the PIN is asked (13, dim);
- PIN, then the field; "Touch your security key." in amber while the key waits;
- each failure in its own shared `approval.security_key.*` line (13, red);
- Cancel and Approve.

How it behaves:
- **Each failure has its own line:** no key, several keys, a key that can't check a PIN, no PIN
  set, a PIN that's too short or too long, a wrong PIN with the tries left in each language's
  plural, locked, three in a row, another key, not touched in time, not allowed, unplugged.
- **Anything else** (an answer KIFF would refuse, a libfido2 that's missing) says the key
  couldn't sign, never that the key declined.
- **The sheet stays open to try again,** except for a locked key. Cancel or closing it approves
  nothing. If the key is waiting for its touch at that moment, the request is stopped at the key
  (`StopKey`, `OpenKey::cancel`, as macOS 12's sheet does), so a touch afterwards signs nothing.
  The attempt then answers `Cancelled`, and its caller sends nothing.
- **The PIN** is taken from the field as bytes and the field emptied at once. It's handed to
  the key once off the main loop, then cleared. As on macOS 12, GTK's own entry held the
  characters while they were typed, and this app can't zero that memory.

It's switched off: only `--page pin-sheet` shows it, with canned answers (five tries left, no key
plugged in). No key is asked, and nothing is signed.

Next come registering the key through pairing, with S1's `credential_proof`, and the approval
route.
