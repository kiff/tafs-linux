# Account

The iPad's Account (`SettingsView` in `SignInView.swift`), beside the sidebar, opened from
Account at its foot. What it says is `tafs-core/src/account.rs`, tested there; the page is
`crates/tafs-app/src/account.rs`.

## The screen

"Account" at 34 bold, then the iPad's grouped list: each section's header (17 semibold, dim), and
its rows on the panel colour with corners of 26, 17 in the ink, a line between them. In the iPad's
order, the sections whose reads are ported:

- **Your account:** Mode (Signed in with KIFF, Sample data, Signed out); for a KIFF account,
  KIFF account with its name, and Signed in as with the email, said once (`AccountLines`): none
  when the account is named after it, as a Solo account is.
- **People & apps** (KIFF): People & groups, Devices, Connect apps.
- **Plan** (KIFF): Plan & usage, its title alone until the balance is read (`PlanRow`).
- **Alerts.**
- **Theme:** Light, Dark or System, as the system's segmented control (a capsule track in the
  system grey, the chosen segment on it; in dark, #636366). The choice is kept across launches in
  the user's config folder (`tafs/theme`), as the iPad's `tafs.theme`, and the window follows it at
  once.
- **Language:** the app's language by its own name ("English", "Français"), and Change language.
- **Time zone** (KIFF): the account's, from KIFF, else UTC.
- **How it works:** "Every action a taf takes goes through KIFF, … Your answers are signed on this
  computer."
- **Legal:** Terms and Privacy policy, tafs's own pages, saying "(in English)" outside English.
- **About:** Version, and "They taf. You kiff."
- **Account actions:** in the sample, Sign in with KIFF and Clear the sample; signed in, Sign out,
  and under it why signing out didn't work.

## KIFF's account

Signed in, the app reads `GET /v1/me` at KIFF as the session (`me`), with Home and the held calls:

- **Its name** (`name`): its display name, else its slug. It shows under Account at the sidebar's
  foot (`accountLabel`), and a screen reader hears "Account, Acme".
- **`approval_mode`:** what KIFF checks (contract §1.2). The view-only notice follows KIFF's word
  once it's read; until then, the one the session was handed.
- **Its capabilities:** `holds.answer` decides whether held calls say someone else answers them
  (none said: allowed, as the iPad's `allows`); Coach shows for `members.manage` until Coach's own
  access is read (`mayUseCoach`, which is off with nothing read).

## Signing out

Sign out asks first (`SignOutWay.signOut`): "Sign out of KIFF?", "This computer stops working for
your account until you sign in again. Your tafs keep working.", Sign out in red, and Cancel.
Then (`AppModel.signOut`):

- `POST /v1/auth/device/logout` with the session alone. A 401 means KIFF had already ended it.
- Signed out at KIFF, the keyring forgets the session and its key, and the welcome opens. A
  keyring that refuses to forget is tried again; if it still refuses, the token it keeps is one
  KIFF has ended, so the next launch's reads get a 401 and forget it.
- When KIFF can't be reached, nothing is forgotten: Account opens again with "Not signed out at
  KIFF yet: … Try again, or sign this computer out from another device's Devices."

**Clear the sample** forgets the sample and opens the welcome; **Sign in with KIFF** from the
sample opens the sign-in over it.

## Not yet

- **The rows' screens:** Alerts and Time zone open their titles until each is ported. KIFF
  account, People & groups, Devices, Connect apps and Plan & usage open their screens.

## KIFF account

Account › KIFF account (`AccountView`, `crates/tafs-app/src/kiff_account.rs`): the round back
button, then the account's name, its id when it isn't its name, and who is signed in (as
`AccountLines`); its time zone (`time_zone` from `GET /v1/me`, UTC when KIFF sends none) with,
when this computer is in another zone, "Use this computer's: {identifier}" (this computer's zone
by `TZ`, else `/etc/localtime`), and "Holds' deadlines, Activity, the Record and what your tafs
write use this time zone."; Export my chats with what it exports; and Delete account with what it
deletes, for whoever may (`account.delete`). The time zone row doesn't open its picker yet; using
this computer's zone (a signed setup call), exporting and deleting are there, dimmed.

## People & groups

Account › People & groups (`PeopleGroupsView`, `crates/tafs-app/src/people.rs`,
`tafs-core/src/people.rs`): the round back button to Account, "+ Group" for whoever manages
people (`members.manage`), then KIFF's error when it couldn't be read ("People and groups
couldn't be read. …"), the account's line ("Acme · 3 people", the count alone for a Solo
account), and each group (`GET /v1/groups`), KIFF's default one first ("Everyone", in the
person's language) then by name: its name (15 semibold) and the tafs it uses ("Uses Invoices and
Refunds", sorted; "Uses no taf yet"), its actions (rename, delete) for whoever manages people,
its people (`GET /v1/tenant/members`, the active ones), managers first, then by email, each with
the first letter on a raised circle, their email (else KIFF's id), and Manager (amber) or Member;
then "Add someone to Finance" for whoever manages the group, never for Everyone. "No groups yet."
when there are none. It reads again every 20 seconds while it's open.

Read as strictly as the iPad's decoders: a group without its `members` or `agents`, or a person
without `subject`, `role` or `status`, fails the read. The sample has no account: no groups.
`--people <file>` with `--home` draws it from synthetic answers
(`crates/tafs-app/fixtures/people-kiff.json`), CI's `people`.

Not yet: **every change** (a group, its name, deleting it, adding or removing someone, making a
manager), each the person's act signed with an approval key: "+ Group", the group's actions and
Add someone are there, dimmed, and removing or changing someone (the iPad's swipe and context
menu) isn't drawn, until this computer can approve.

## Devices

Account › Devices (`DevicesView`, `crates/tafs-app/src/devices.rs`, `tafs-core/src/devices.rs`):
the round back button to Account, then the devices signed in as this person
(`GET /v1/me/devices` at KIFF), each with its name (KIFF's `display_name`, else `name`, else "A
device"; 15 semibold) and when it was last used (13, dim): "Last used 3 hours ago", in iOS's
relative words for each language (`RelativeDateTimeFormatter`: the largest unit that fits; now
is "in 0 seconds"), or "Not used yet". This computer says "This device"; the others have Sign
out, in red. Under them: "Signing a device out ends its sign-in on this account at once. To
sign out this computer, use Sign out of KIFF." When KIFF can't be read, why, in red, above.

Read as strictly as the iPad's decoder: a device without its `id`, or with a date it can't read,
fails the read. The sample shows the reference set's three (this computer, used now; an iPhone 3
hours ago; a Mac a day ago) in a screenshot, and none otherwise, as the iPad's sample.
`--page devices` opens it, as `devices.png`.

Not yet: **signing another device out**, an approval signed by an approval key (Sign out is
there, dimmed, until this computer can approve). **Add a device** isn't for Linux: a phone
pairs a computer (`PairComputerView`), never the computer itself.
- **Change language:** Linux has no setting of the app's own to open: tafs speaks the desktop's
  language (`LANGUAGE`, `LANG`). The button is there, dimmed.
- **Coach's suggestions, the AI-data consent, Export my chats:** with their reads; the iPad draws
  none until they're read either.
- **The plan's line** (`plan_line`) in Plan, with tafs cloud's balance.
