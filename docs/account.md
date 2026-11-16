# Account

The iPad's Account (`SettingsView` in `SignInView.swift`), beside the sidebar, opened from
Account at its foot. What it says is `tafs-core/src/account.rs`, tested there; the page is
`crates/tafs-app/src/account.rs`.

## The screen

"Account" at 34 bold, then the iPad's grouped list: each section's header (17 semibold, dim), and
its rows on the panel colour with corners of 26, 17 in the ink, a line between them. In the iPad's
order:

- **Your account:** Mode (Signed in with KIFF, Sample data, Signed out); for a KIFF account,
  KIFF account with its name, and Signed in as with the email, said once (`AccountLines`): none
  when the account is named after it, as a Solo account is.
- **People & apps** (KIFF): People & groups, Devices, Connect apps.
- **Plan** (KIFF, `PlanRow`): once tafs cloud's balance is read with its `plan_line`, that line
  (16 semibold, in `plan_tone`'s colour: amber for a warning, red while waiting) with Plan & usage
  under it (13, amber); else Plan & usage alone. Either opens Plan & usage. It's the balance read
  with Home, for the sidebar's plan line.
- **Coach** (KIFF, for someone who may use Coach, once read): Show suggestions, a switch, with
  tafs cloud's words for what it does now under it (13, soft), read when Account opens
  (`GET /v1/me/suggestions`). It works for someone who manages Cards (`cards.manage`, none listed
  counting as allowed), else it's dimmed. A change is `PUT /v1/me/suggestions` with `show`,
  unsigned, as the session; the switch stays dimmed while it's on its way, and a refusal says
  "It didn't change. {why}" in red, in tafs cloud's words. A setting that can't be read draws no
  section. The recipes aren't read again at once, as the iPad's `loadRecipes` does: Home's are
  read with the next refresh.
- **Alerts:** opens Account › Alerts (below).
- **Theme:** Light, Dark or System, as the system's segmented control (a capsule track in the
  system grey, the chosen segment on it; in dark, #636366). The choice is kept across launches in
  the user's config folder (`tafs/theme`), as the iPad's `tafs.theme`, and the window follows it at
  once.
- **Language:** the app's language by its own name ("English", "Français"), and Change language.
- **The AI-data consent** (KIFF, once read, `GET /v1/me/ai-consent`; see below).
- **Time zone** (KIFF): the account's, from KIFF, else UTC.
- **Export my chats** (KIFF, `ExportChatsSection`), with what it exports under it: the same row
  as the KIFF account's (below).
- **How it works:** "Every action a taf takes goes through KIFF, … Your answers are signed on this
  computer."
- **Legal:** Terms and Privacy policy, tafs's own pages, saying "(in English)" outside English.
- **About:** Version, and "They taf. You kiff."
- **Account actions:** in the sample, Sign in with KIFF and Clear the sample; signed in, Sign out,
  and under it why signing out didn't work.

Until the list's end is in sight, a chevron at its foot, over a fade to the background, says
there's more below, as the iPad's does. It takes no taps.

## KIFF's account

Signed in, the app reads `GET /v1/me` at KIFF as the session (`me`), with Home and the held calls:

- **Its name** (`name`): its display name, else its slug. It shows under Account at the sidebar's
  foot (`accountLabel`), and a screen reader hears "Account, Acme".
- **`approval_mode`:** what KIFF checks (contract §1.2). The view-only notice follows KIFF's word
  once it's read; until then, the one the session was handed.
- **Its capabilities:** `holds.answer` decides whether held calls say someone else answers them
  (none said: allowed, as the iPad's `allows`); Coach shows for `members.manage` until Coach's own
  access is read (`mayUseCoach`, which is off with nothing read).

## The AI-data consent

Every word is tafs cloud's (`AIConsent.words`), read as strictly as the iPad's decoder; one
without its title, its lines, Allow or Not now is no screen, and draws no row
(`crates/tafs-app/src/ai_consent.rs`, `tafs-core/src/ai_consent.rs`, tested against the pinned
`ai-consent.json`).

- **Account's row** (`AIConsentRow`): once given, tafs cloud's words for it ("AI use: allowed")
  and Withdraw in red, when it sends Withdraw's words; not given, its title in the amber, which
  opens the screen. Why an answer didn't go, in red, under it.
- **The screen** (`AIConsentView`), over the window: tafs cloud's sparkles mark, its title (24
  bold), its lines (16), why Allow didn't go, Allow and Not now.
  - **Allow** is `POST /v1/me/ai-consent` with the version of the words on the screen. tafs cloud
    refuses an older screen's yes (409) with the new words, which show in its place with why.
  - **Not now** (or closing it) puts it off for this session: it isn't put up again by a read,
    only from Account.
- **After sign-in** (`loadAIConsent`), the consent is read with the account; not given, the
  screen comes up over the window, unless Not now was answered this session. A new account's
  consent is its own, and an account that goes (signed out here or by KIFF) closes the screen if
  it's open, so its Allow is never answered for the account that left.
- **Withdraw** is `DELETE /v1/me/ai-consent`. Each is unsigned, at tafs cloud as the session.

A turn tafs cloud refuses for want of the consent (`ai_consent_required`) puts the screen up on
the iPad; Linux sends no model turns yet (no chat or Coach), so nothing here asks for it.

The screenshots read `--account-reads` (`{"suggestions": …, "consent": …}`, each as tafs cloud
answers it; nothing is sent): `account-kiff` with `--me crates/tafs-app/fixtures/me-manager.json`
(an account that may use Coach and manages Cards) and `account-reads.json` (consent given), and
`consent` (`--page consent`, the screen as after sign-in, `account-reads-not-given.json`).
`account-kiff` also reads `--balance crates/tafs-app/fixtures/balance-home-line.json`, for the
Plan row's line.

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

## Against the iPad's, line by line

Each section and row of `SettingsView` is here, in its order, with `AccountLines` and `PlanRow`.
What differs:
- **The theme's segments** have their words alone: `ThemePicker`'s labels carry SF Symbols,
  which this computer doesn't have.
- **How it works** says "Your answers are signed on this computer.", the shared line with this
  computer's name in it. This computer only views and signs nothing, so the line isn't true
  here; the shared strings have no other line for a computer that only views yet.
- **Under Account in the sidebar** is the account's name (`accountLabel`), as the iPad's; its
  email is on Account itself, in Signed in as.

## Time zone

Account › Time zone (`TimeZonePicker`, `crates/tafs-app/src/zones.rs`, `tafs-core/src/zones.rs`),
from Account's row or the KIFF account's: "Use this computer's: {identifier}", then tafs cloud's
zones (`GET /v1/time-zones?q=…`, every character of what was typed escaped but a letter or a
digit) for what the person types in "Find a time zone", read again a moment after the last key,
only the newest search's answer kept; the account's zone checked. Why the zones couldn't be read,
in red, under the first row. Read as strictly as the iPad's decoder: a zone without its `id` or
`title` fails the read.

**Choosing a zone** and **using this computer's** are the owner's act, which the iPad signs
(`setAccountTimeZone`: `tafs.time-zone.v1` behind Face ID, then `PUT /v1/me/time-zone` as a
signed setup call). The rows are buttons, as the iPad's are, and so is Use this computer's.
Their tap is the tap that would sign. Under the first row, where the iPad says why the zone
didn't change, it says "This device can't approve. Approve on another device signed in to your
account." Nothing is sent (`docs/approval.md` › A view-only computer). `--page time-zone-use`
taps Use this computer's (CI's `time-zone-use`).

## KIFF account

Account › KIFF account (`AccountView`, `crates/tafs-app/src/kiff_account.rs`): the round back
button, then the account's name, its id when it isn't its name, and who is signed in (as
`AccountLines`); its time zone (`time_zone` from `GET /v1/me`, UTC when KIFF sends none) with,
when this computer is in another zone, "Use this computer's: {identifier}" (this computer's zone
by `TZ`, else `/etc/localtime`), and "Holds' deadlines, Activity, the Record and what your tafs
write use this time zone."; Export my chats with what it exports; and Delete account with what it
deletes, for whoever may (`account.delete`). The time zone row opens its picker. Using
this computer's zone is signed: its tap says this computer can't approve, under it, and sends
nothing.

Export my chats (`ExportChatsView.swift`, kiff/tafs#424) reads `GET /v1/me/chats/export` as the
session, signed in with KIFF from a kept session. While it reads, "Preparing your chats…" shows
with a spinner.
- **The two files** are split as `splitExport` does: `tafs-chats.json` is the answer without
  `readable`, its keys sorted and pretty-printed, and `tafs-chats.txt` is `readable`. They're
  saved in a folder the person picks. The iPad hands them to its share sheet instead, and
  closing the dialog saves nothing, as closing the sheet does.
- **Saving again:** "Ready: tafs-chats.json, tafs-chats.txt. Share again" then saves them again.
  These are the iPad's words.
- **When it fails,** why shows in red and the row says Try again:
  - an answer that wasn't read, or isn't what tafs cloud sends: "Your chats weren't exported.
    Check your connection and try again.";
  - files that couldn't be saved: "…this computer couldn't save the files."
- **The answer can be up to 32 MB**, as for receipts.
- **The files are the person's alone:** they're written with mode 0600, whatever the umask, so
  other accounts on a shared computer can't read them. Their names are fixed, so an export
  saved in the same folder replaces the last one there.
- **A screenshot's account sends nothing**, so its row stays dimmed.

Delete account opens its sheet (`DeleteAccountView`), with Cancel at the top:
- the warning ("This deletes your KIFF account. It can't be undone.", 15 semibold);
- What goes, each line with its symbol: tafs, runs, chats and notes; app sign-ins; Cards and
  keys; KIFF records, after 30 days.

Deleting is confirmed by the device, which this computer can't do. So where Delete account would
be, the sheet says "This device can't approve. Approve on another device signed in to your
account." (`ApprovalGate`, `docs/approval.md`). The iPad's footer ("You confirm with Face ID…")
goes with the button, so it's left out. `--page delete-account` opens the sheet, CI's
`delete-account`.

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

**Every change is signed on the iPad** (`changePeople`, behind Face ID, each a call KIFF takes
only signed). This computer only views, so it draws each one as the iPad does, up to the
signature, and sends nothing (`tafs-core/src/signed.rs`, `docs/approval.md` › A view-only
computer):
- **+ Group** opens `GroupNameSheet` with Cancel, "New group", and Create, which works once a
  name is typed (trimmed). Under the field: "Add its people next, then share a taf with it from
  the taf's page."
- **A group's actions** (the ⋯ button, for whoever manages people, never on Everyone):
  - **Rename** opens the same sheet, "Rename Finance", with its name in the field, Save, and "Only
    the name changes: its people and tafs stay.";
  - **Delete** asks "Delete Finance?", with what it does and Delete Finance in red.
- **A person's menu** (the iPad's context menu and swipe: a right click or a long press here):
  - Make the manager, or Make a member, for whoever manages people;
  - Remove from Finance, in red, where the iPad's `mayRemove` lets them.
- **Add someone to Finance** opens `AddToGroupSheet`, "Add to Finance": As the group's manager,
  with what a manager does (for whoever manages people), then THE ACCOUNT'S PEOPLE not in it yet,
  each with their badge; "Everyone on the account is here. …" when no one is left.

Create, Save, Delete, each menu item and each person in the add sheet is the tap that would
sign. It says "This device can't approve. Approve on another device signed in to your account."
In a sheet, the words go under its content and the sheet stays open. On the screen, they go
where the iPad says why a change didn't go, above the groups.

`--page people-rename` opens the rename sheet over Finance with Save tapped (CI's
`people-rename`), and `--page people-add` opens the add sheet (`people-add`).

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

**Signing another device out** is an approval the iPad signs with its approval key
(`removeDevice`). Sign out asks first, as the iPad's confirmation does: "Sign tafs on iPhone
out?" ("Sign this device out?" for one with no name), Sign out in red, and Cancel. Its Sign out
is the tap that would sign. Above the list, where the iPad says why a device wasn't signed out,
it says "This device can't approve. Approve on another device signed in to your account." Nothing
is sent (`docs/approval.md` › A view-only computer). The sample signs nothing, so there it does
nothing. `--page devices-sign-out` opens the question over the sample's iPhone (CI's
`devices-sign-out`).

Not on Linux: **Add a device**, since a phone pairs a computer (`PairComputerView`), never the
computer itself; and **the account's passkeys** (`DevicesView`'s passkeys section), which the
iPad draws only signed in with KIFF and which aren't ported yet.
## Language

**Change language** opens **Account › Language** (`LanguagePicker`, `language_picker.rs`):
"This computer's language" first, then the app's 8 languages in tafs's order, each named in
itself ("Français", "Polski"), the one in use checked (`tafs-core/account.rs` ›
`language_rows`). The one in use does nothing. Any other applies at once: it's kept in the
config folder (`~/.config/tafs/language`), and the window is drawn again in it, on Account,
with no relaunch. Every read is made again, asking tafs cloud and KIFF in the new language
first. This computer's language removes the pick, and tafs speaks the desktop's again
(`LANGUAGE`, `LANG`). `--lang` still wins, and a screenshot never reads the pick.

## Alerts

Account › Alerts (`NotificationSettingsView`, `crates/tafs-app/src/alerts.rs`,
`tafs-core/src/alerts.rs`), with the round back to Account.
- **What an alert looks like** (`LockScreenAlert`): "Chargebacks needs you" with sound and
  "Refunds answered maria@…" silent, each with the mascot, on the raised colour.
- **Signed in with KIFF:**
  - **Alerts on this computer** turns the held calls' alerts on or off (`main.rs` ›
    `watch_holds`). Off, nothing new is announced; what's already shown stays.
  - **Sound** ("For a hold or a halt only") is off and greyed while alerts are off, and its
    choice is kept for when they're back.
  - Both are kept across launches in the user's config folder (`tafs/alerts`,
    `tafs/alert-sound`), on unless turned off. A notification here has no sound of its own, so
    with Sound on, a read that announces a call plays the desktop's alert sound once.
  - **Under them:** for a role that can't answer held calls, "Alerts go to the people who may
    answer held calls. Your role on this account can't."; else "Get an alert when a taf needs
    you". The iPad's "…this device tells you, even with tafs closed" is a phone's push. This
    computer reads only while tafs runs, so it isn't said here.
- **Not on Linux:** What alerts you and the tafs that alert (a phone's push choices, read with
  its push token, which a computer hasn't; the iPad draws none of it without one), and Haptics.
- **The sample** shows the two examples alone, as the iPad's.
