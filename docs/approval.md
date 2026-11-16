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
