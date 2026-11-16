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

Next come the key itself over CTAP2 (HID) with its PIN sheet, then registering it through
pairing, with S1's `credential_proof`.
