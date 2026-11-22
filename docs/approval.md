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

At tafs cloud, `signed::signed_at_cloud` names what the device checks or signs before the iPad
sends it: a version switched to or made live, a trial opened (on a KIFF draft the phone signed)
or installed, and a taf removed for good. Every write to tafs cloud (`kiff::cloud_send` and the
rest) refuses those routes by path, whatever built it, read as `signed` reads a route.

The acts drawn this way: People & groups' changes, signing another device out, and the
account's time zone (`docs/account.md`); declining a request (`docs/usage.md`); sharing a taf and taking a share away (`docs/taf.md`); connecting an app
once its taf is picked (nothing is started, so nothing waits at planque), removing an app's
access, cancelling its request or sign-in, clearing a removed taf's connections and signing in
again, which are planque's (`docs/connect-apps.md`); removing a Slack
door, whose subscription at planque the device signs first, making a taf's waiting version live, which the device checks before
tafs cloud's switch begins, and removing a taf for good, whose `DELETE` the approval key signs
(`docs/taf.md`); Done and Skip it on a step a taf handed its owner, whose answer carries the
device key's signature (`docs/needs-you.md`; `signed` lists `POST /v1/me/handoffs/{id}/answer`);
and on the taf's dial, approving a held call once and an app event (`docs/taf.md`; `signed` lists a
held call's answers, `POST /v1/me/card-exceptions/{id}/approve-once|reject|change-card`). Revoking a Card isn't signed (it only takes authority away): it really goes, as
the session, unless KIFF keeps the session view-only. planque's acts
(`Act::at_planque`) have no route here at all: this computer's only planque call is its grants read.
Making a version live and removing a taf (`Act::at_device`) are checked or signed by the device before a call it makes as the
session; this computer never begins either, since a switch stops the taf until it's finished.

Approving an app event's subscription in Starts when is one the iPad disables itself where the
device can't approve (`approvalBlocked`): disabled here too.

A button the iPad itself wraps in `ApprovalGate` is the exception: there, as on the iPad, the
gate's line is drawn in the button's place (`approval::gate`), not the button. Raise the limit is
one (`docs/usage.md`); so is a connection's approval (`docs/connect-apps.md`); an app event's filters before its subscription is signed
(`AppEventApprovalSheet`, `app_event_sheet.rs`) is another: the sheet is drawn whole, its filters
as the iPad's `JSONEncoder` writes them, and the gate's line stands where Approve would. It opens
from the dial, pressing an app event to approve. From Setup › Starts when it doesn't, as on the
iPad while the device can't approve: Approve there is disabled (`approvalBlocked`).
`--app-event item.json` opens it for its screenshot.

Not on Linux: `ReadinessReader` reads again when the app comes back, but this computer's state
never changes. `openTafsSettings` opens tafs' iOS settings, which don't exist here, and
`Unsupported` offers neither button anyway.

## Approving with a passkey

**A Linux computer paired without a security key approves nothing**: it's view-only, and every
approval goes to the phone. Paired with one, it approves with that key (below): a USB security
key (a FIDO2 key with a PIN), as kiff/tafs `docs/computer-passkeys.md` describes for a computer
with no key store of its own. What a passkey approval carries is in `tafs-core/src/passkey.rs`:

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

It's asked only when the person adds the key at pairing or approves with it. Its tests ask with
no key plugged in: where libfido2 is installed it finds none, and where it isn't, it says it's
missing. Either way, no PIN is left held.

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
- **The sheet stays open to try again,** except for a locked key, or once the request has gone
  to KIFF (`KeyError::Sent`). Cancel or closing it approves
  nothing. If the key is waiting for its touch at that moment, the request is stopped at the key
  (`StopKey`, `OpenKey::cancel`, as macOS 12's sheet does), so a touch afterwards signs nothing.
  The attempt then answers `Cancelled`, and its caller sends nothing.
- **The PIN** is taken from the field as bytes and the field emptied at once. It's handed to
  the key once off the main loop, then cleared. As on macOS 12, GTK's own entry held the
  characters while they were typed, and this app can't zero that memory.

It opens when the person adds the key at pairing ("add" mode) and for each approval. For a
screenshot, `--page pin-sheet` shows it with canned answers (five tries left, no key plugged in):
no key is asked there, and nothing is signed.

## Registering the key at pairing, and approving with it

The contract is the PM's decision (i) (kiff/work#17, 22 Nov): a computer whose device key is in
software pairs with `kind: "passkey"` and a **security key's credential**, and KIFF makes its
session `approval_mode: "passkey"`. The device key still grants nothing: only a fresh assertion
from that key approves, one request at a time, with presence and the PIN. Everything below waits
on KIFF's side of it (kiff/cloud#96), and is on (`passkey::ENABLED`).

**At pairing** (`pairing.rs` › `add_key`, `tafs-core/src/pairing_key.rs`):
1. With one key plugged in, the PIN sheet opens in its "add" mode before the code shows: "Add a
   security key", its ask, Add, and "Pair without a security key" in Cancel's place, which pairs
   view-only as today (`words::add_sheet`). No key, or several, pairs view-only too.
2. The key makes a credential for kiff.dev over `kiff.passkey.pair.v1\n<nonce>\n<pub>` (KIFF's
   `PairingCredentialMessage`), so it's made for this pairing and no other: a `webauthn.create`
   client data with that message's hash as its challenge.
3. The credential is read as KIFF reads it (`read_created`): kiff.dev's hash, presence and the PIN,
   an attested credential, not backup-eligible, and exactly an ES256 P-256 key, which becomes SPKI
   DER. Anything KIFF would refuse is never sent.
4. The code names it: `credential: {id, public_key}`, base64url. The check code is over the
   device key and the credential's key (`pairing_key::check_code`, KIFF's `PairingCode`), so the
   six characters the phone shows vouch for both.
5. The claim carries `credential_proof`: the client data and an attestation object of format
   `none` over the key's authenticator data. The key's own attestation is never sent.
6. The session keeps the credential only when KIFF answered `passkey` (`kept_with_key`). Any
   other answer keeps it view-only: the app never claims more than KIFF granted.

**Approving** (`key_approval.rs`, from `approval::refuse`): signed in with such a session, a tap
that would sign asks the key instead, for the acts whose whole request this app writes
(`Act::request`, the iPad's `KIFFClient` bodies: groups and their members, signing a device out,
the time zone, sharing, resuming, discarding a setup, removing a taf, trials, and an API door's
reopening, a key's revoking and the door's removal, at tafs cloud, whose `doorSigned` takes the
passkey header as the device's; then the doors are read again). **An API door's New key stays on
the phone** (the PM's 00:19): its answer is the key, shown once, which this computer can't keep
from a screen recording or a clipboard manager's history.
- The request is the message a device key would sign (`kiff.request.v1`: tenant, method, path
  with its query, the body's hash, `signed_at`), its hash the challenge.
- The PIN sheet asks the key once. Its answer is checked (`passkey::header`'s refusals), and the
  request goes with `KIFF-Passkey-Assertion`, to KIFF, or to tafs cloud for what it takes signed
  (`Act::at_cloud`). `kiff::send_with_key` sends only the act's own request, byte for byte, and a
  call to KIFF only where KIFF takes it signed.
- **Removing a taf for good** goes in the iPad's order (`AppModel.removeTaf`): the key approves,
  then its Cards are revoked at KIFF with the session, one plain call each, then tafs cloud's
  `DELETE` goes with the approval. What didn't go is said in the app's words: Cards KIFF didn't
  revoke, the parts tafs cloud couldn't remove (`removal::after`), or that tafs cloud didn't
  answer (`removal::cloud_did_not_answer`).
- **The sheet names the act** in its lead ("This needs your security key: {act}. Plug it in,
  then touch it and enter its PIN."), `{act}` being the iPad's own Face ID reason for it
  (`approval::refuse_as`, `people::prompt`, `Act::own_prompt`).
- Once sent, the sheet closes, whatever comes back. A refusal shows KIFF's own line where the
  act's line is. A lost answer shows the network's line, and the sheet offers no second PIN try:
  the act may already have reached KIFF, and a new assertion would send it again (a group made
  twice). With no line to show, it's the shared "couldn't sign" line, never a code name.

Still refused here, whatever the key: planque's acts (their own signing), a held call's or a
step's answer (its signature is in its body), Make live (the device checks it first), and Raise
the limit and Decline, whose bodies need what their acts don't carry yet.
