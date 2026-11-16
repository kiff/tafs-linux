# The Linux app (`crates/tafs-app`)

GTK 4 and libadwaita, in Rust. It builds with plain `cargo build -p tafs`, once the words are in
`shared/` (`tools/sync.sh`): without them it stops before any screen, never showing a line in
another language or a `{hole}`.

## The window

- **Signed out:** the welcome ([sign-in](sign-in.md)), and Sign in with KIFF over it.
- **Signed in, or in the sample:** one sidebar for the whole app, as on the iPad
  ([the sidebar](home.md)), and the chosen screen beside it: the Overview first, and
  [Needs you](needs-you.md), [the Record](record.md), [a taf's page](taf.md), and
  [Account](account.md) from the sidebar's foot. Screens not
  ported yet show their title only, in the [port checklist](port-checklist.md)'s order.

The colours are the iPad's `Theme.swift`, light and dark, and follow the desktop's appearance
(libadwaita's `StyleManager`), or the theme chosen in Account. The type is the desktop's own font; the iPad's rounded titles are
its bold. The SF Symbols are the app's own symbolic icons, drawn for the nearest look
(`assets/icons`), and the mascot is the iPad's.

## Language

The app speaks the first of the person's languages that tafs ships (`LANGUAGE`, then `LC_ALL`,
`LC_MESSAGES`, `LANG`), otherwise English, as the iPad's `screenLanguages`. Every line comes from
the words at the pin.

## Time

Every time the app draws is in the account's time zone (`AccountTime`,
`tafs-core/src/account_time.rs`): signed in with KIFF, the zone `GET /v1/me` names (UTC when it
names none, or one this computer doesn't know); in the sample, and before the account is read,
this computer's. The day a line falls on ("Today", a Record day) is the account's day. Where the
iPad adds the zone's short name (`zonedShortTime`: a held call's "answer by", a Record line's
time), so does Linux, when the account's clock isn't this computer's at that moment ("14:05 JST").

## Options

| Option | What |
| :- | :- |
| `--lang fr` | speak a shipped language |
| `--theme light` or `dark` | force the appearance |
| `--sample` | the iPad's sample, as Try it with sample data opens it |
| `--signed-in` | with `--sample`: the account's items too, as the iPad's reference set draws them |
| `--home contract.json` | the account's screens from a contract file's Home (`shared/contracts/home.json`), in place of tafs cloud |
| `--view-only` | a signed-in session KIFF says is view-only, with no tafs read |
| `--unproven answers.json` | with `--home`: KIFF's "Did this happen?" answer from a file, in place of KIFF |
| `--recipes answers.json` | with `--home`: tafs cloud's recipes answer from a file, for Home with no taf ([recipes](recipes.md)) |
| `--gallery answers.json` | with `--home`: tafs cloud's gallery answer from a file, for Explore jobs and the strip |
| `--halt answer.json` | with `--home`: a halt as tafs cloud answers it; every read is it and nothing is sent ([halting](halt.md)) |
| `--record-search answers.json` | with `--home`: a Record search page and case as tafs cloud answers them; nothing is sent |
| `--access answer.json` | with `--home`: what this person may do with each taf, as tafs cloud answers `GET /v1/me/access` |
| `--receipts answer.json` | with `--home`: a receipts export as tafs cloud answers it; nothing is sent or saved |
| `--activity answer.json` | with `--home`: KIFF's activity (`{"calls": […]}`) for the Record |
| `--receipt answer.json` | with `--home`: a receipt as KIFF answers `GET /v1/receipts/{trace_id}`; nothing is sent |
| `--paired` | with `--view-only`: a computer already added with the phone, whose notice says approvals go to the phone |
| `--page …` | `welcome`, `sign-in`, `pair` (adding this computer with a phone), `recipe:<id>` (with `--home` and `--recipes`: that recipe's sheet), `gallery` (with `--home` and `--gallery`: the gallery), `halt-all` or `halt:<id>` (a halt's screen), `record-search:<q>` (the Record with that search), `record-export` (the Record with the receipts' days open), `receipt:<proposal id>` (with `--activity` and `--receipt`: that line's receipt), `delete-account` (with `--people`: the KIFF account's Delete account sheet), `pin-sheet` (the security key's PIN sheet, with canned answers; no key is asked), or a screen beside the sidebar: `needs-you`, `record`, `calendar`, `usage`, `connect-apps`, `account`, `taf` (the sample's Refunds), or `taf:<id>` (a taf by its key) |
| `--screenshot out.png` | save the window (1280×800, at 2×) as a PNG and quit. It never reads or writes the keyring, and its sign-in and pairing show a code without reaching KIFF |

CI builds the app and saves the welcome, the sign-in, the pairing, the Overview beside the sidebar (with the
account's items), the Overview from the `home.json` contract, Needs you (and with synthetic steps and Coach's waits, and with "Did this happen?"), the Record, Refunds'
page and its Card tab, Plan & usage and Account over the sample, Plan & usage signed in with
KIFF from synthetic answers (managing billing, and as a member), Connect apps, Devices, People & groups, the KIFF account and its time zone picker (from synthetic answers), the
calendar's Day and Week (and Day at an upright iPad's size, `--size 1032x1376`), a tafs cloud
taf's page and its Card tab from the contracts, the sidebar over the plain sample, and Home with no taf and a recipe's sheet, the strip and the gallery, the halt screens, a Record search, the receipts export open, a Record line's receipt, the Delete account sheet, the security key's PIN sheet, the
view-only notice (and once paired) in en, fr and pt, light and dark: 234 pictures, as the `screenshots`
artifact of the run. The job fails if any is missing.

## The AppImage

`tools/appimage.sh` packages tafs as `tafs-<version>-x86_64.AppImage`, the AppImage for Ubuntu. It
contains:
- the release binary;
- the words at the pin, in `usr/share/tafs/strings`, where the app looks beside its binary;
- the desktop entry (`packaging/dev.kiff.tafs.desktop`) and its icon (the mascot, 192×192);
- Adwaita's symbolic icons, which the screens take from the desktop's theme;
- GTK 4 and libadwaita, with what they load, bundled by linuxdeploy and its GTK plugin. The
  script downloads both at pinned versions and refuses them unless their SHA-256 matches.

libfido2 isn't bundled. A security key uses the system's `libfido2.so.1` when it's installed.

**What it runs on:** it's built on Ubuntu 24.04, whose glibc (2.39) is its floor, so it runs on
Ubuntu 24.04 and newer, and other distributions as new. Ubuntu 22.04 has GTK 4.6 and
libadwaita 1.1, and the app needs GTK 4.14 and libadwaita 1.5. Reaching 22.04 would mean
building those two from source on 22.04, or shipping a Flatpak.

**In CI** (`.github/workflows/appimage.yml`):
- A pull request that changes the packaging builds the AppImage, runs its `--help` (which loads
  every bundled library and starts nothing), and keeps the file as the run's `appimage`
  artifact.
- Run by hand with a version (`v` and three numbers, as `v0.1.0`: anything else is refused before a tag is made), it also attaches the file to a **draft** release.
  Publishing it is the owner's yes, never the workflow's.
