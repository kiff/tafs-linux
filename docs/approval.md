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

Linux draws it in place of:
- "Did this happen?"'s No and Yes (`docs/needs-you.md`);
- the balance's Raise offer and a member's raise request's Raise the limit (`docs/usage.md`).

Not on Linux: `ReadinessReader` reads again when the app comes back, but this computer's state
never changes. `openTafsSettings` opens tafs' iOS settings, which don't exist here, and
`Unsupported` offers neither button anyway.

## Approving with a passkey (switched off)

Linux will approve with a USB security key (a FIDO2 key with a PIN), as kiff/tafs
`docs/computer-passkeys.md` says for a computer with no key store of its own. The first part is
in `tafs-core/src/passkey.rs`, switched off (`ENABLED` is false), so nothing asks a key for
anything yet:

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
- **It starts libfido2 once**, with `FIDO_DEBUG` cleared (it would print the frames sent to the
  key) and no U2F fallback.
- **It opens one key only**, with a PIN set. No key, several keys, a U2F-only key or a key
  without a PIN is refused before anything is asked of it. The key waits two minutes for its
  touch.
- **It asks only two things of the key:** `fido_dev_get_assert` and `fido_dev_make_cred`. It
  reads the retry count after a wrong PIN and for the PIN sheet. It never resets the key, and
  never sets or changes its PIN.
- **The PIN goes to the key once**, and is cleared whether or not the key was reached.
- **A request can be cancelled at the key** (`fido_dev_cancel`) from another thread.
- **The answer is raw:** the authenticator data, the signature and the credential used, or the
  attestation object built from libfido2's three parts. `passkey::refusal` checks it whatever
  libfido2 says.

Nothing calls it while it's switched off. Its tests ask with no key plugged in: where libfido2
is installed it finds none, and where it isn't, it says it's missing. Either way, no PIN is left
held.

Next come the PIN sheet (its `approval.security_key.*` words, once they're at the pin), then
registering the key through pairing, with S1's `credential_proof`.
