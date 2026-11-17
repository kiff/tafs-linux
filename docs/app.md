# The Linux app (`crates/tafs-app`)

GTK 4 and libadwaita, in Rust. It builds with plain `cargo build -p tafs`, once the words are in
`shared/` (`tools/sync.sh`): without them it stops before any screen, never showing a line in
another language or a `{hole}`.

## The window

- **Signed out:** the welcome ([sign-in](sign-in.md)), and Sign in with KIFF over it.
- **Signed in, or in the sample:** one sidebar for the whole app, as on the iPad
  ([the sidebar](home.md)), and the chosen screen beside it: the Overview first, and
  [Needs you](needs-you.md) (or [Your brief](brief.md) in its place, once tafs cloud sends
  one), [the Record](record.md), [a taf's page](taf.md) with [its chat](chat.md), and
  [Coach](coach.md) and [Account](account.md) from the sidebar's foot. Screens not
  ported yet show their title only, in the [port checklist](port-checklist.md)'s order.

The colours are the iPad's `Theme.swift`, light and dark, and follow the desktop's appearance
(libadwaita's `StyleManager`), or the theme chosen in Account. The type is the desktop's own font; the iPad's rounded titles are
its bold. The SF Symbols are the app's own symbolic icons, drawn for the nearest look
(`assets/icons`), and the mascot is the iPad's.

## Illustrations

The iPad's approved first-time and empty-state drawings (`Illustration`, one per board), drawn in
Cairo shapes (`crates/tafs-app/src/illustrations.rs`), never a letter, a number or a word (rule
14): the screen's own words say what each means. Each is drawn on its board's grid in the
palette's colours for the appearance on screen, again when it changes, and scaled down to the
width it's given, never wider than the iPad shows it and never out of proportion. A screen reader
skips them. The sign-in's drawings (`connected.rs`) are made of the same pieces.

| Board | What it shows | Where |
| :- | :- | :- |
| 1, first run | A card opening, sparkles round it | The welcome, above "tafs", in the mascot's place; it gets smaller in a short window rather than push the buttons off |
| 2, no tafs yet | An empty card's outline, a plus in it | The Overview with no taf (and no recipes listed), above the grid's "No tafs yet", in board 10's place; while the list is read, or when it couldn't be, the mascot (72) in its place, over "Loading your tafs…" or "Couldn't load your tafs." |
| 3, the brief with nothing to decide | A calm sun over a clear page | Your brief with no decision left, where the decisions go |
| 4, every taf halted | Cards at rest under a pause | Halt all, once every part holds it, above its panel |
| 6, nothing waiting | An empty tray under a calm check | Needs you with nothing waiting, above "Nothing is waiting." |
| 7, Coach with no taf or one | Its spark beside one card, room for more | Coach's empty chat, at its leading edge above "Ask anything about your tafs", while the account has no taf or one |
| 8, a new taf | Words in a bubble becoming a card | + New's gallery, small, beside "Say it in your own words" |
| 9, Coach starting | Its spark in rings filling, its first steps under it | Coach before it's started, above "Coach mode" (a wide window, as an iPad's or a Mac's) |
| 10, the overview | A ring, a gentle line and cards in a row | The Overview, under its title |

Not drawn yet, as their screens aren't on Linux: 5 (a key's
receipt, after connecting an app), 11 and 12 (a watch's).

Every screen with an empty state on the iPad fills the pane beside the sidebar, at the window's
full height, with its board where one exists: the Overview with no taf (board 2, or the mascot
while the list is read or when it couldn't be), Needs you with nothing waiting (6), your brief
with nothing to decide (3), Halt all once held (4), Coach's empty chat (7) and its start (9), and
+ New's gallery (8). Home with no taf and recipes listed takes the pane with them, at most 720
wide. Boards 5 (a key's receipt after connecting an app, which this computer can't sign), 11 and
12 (a watch's) have no screen here.

The colours are the palette's by the iPad's own names: `Theme.amber` is `amber`, `amberText`
`amber_text`, `amberInk` `amber_ink`, `ink`, `panel`, `onFill` `on_fill`, `ok`, `blue`, `pink`;
`Theme.brand` is the mark's sweep (`theme::BRAND`), and the shadows are black.

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
| `--coach-setup answer.json` | with `--home`: Coach as tafs cloud answers `GET /v1/setup`, for Finish setting up |
| `--unreachable network\|error` | with `--home`: the sidebar as after Home couldn't be read, the Unreachable line at its foot; nothing is read again |
| `--halt answer.json` | with `--home`: a halt as tafs cloud answers it; every read is it and nothing is sent ([halting](halt.md)) |
| `--record-search answers.json` | with `--home`: a Record search page and case as tafs cloud answers them; nothing is sent |
| `--access answer.json` | with `--home`: what this person may do with each taf, as tafs cloud answers `GET /v1/me/access` |
| `--receipts answer.json` | with `--home`: a receipts export as tafs cloud answers it; nothing is sent or saved |
| `--activity answer.json` | with `--home`: KIFF's activity (`{"calls": […]}`) for the Record |
| `--work answer.json` | with `--home`: a taf's work as tafs cloud answers `GET /v1/tafs/{id}/work`; nothing is sent |
| `--balance answer.json` | with `--home`: the balance as tafs cloud answers `GET /v1/usage/balance`, for the sidebar's plan line |
| `--runs answer.json` | with `--home`: a page of a taf's runs as tafs cloud answers `GET /v1/tafs/{id}/work/runs`; every page reads it, and Run now sends nothing |
| `--request answer.json` | with `--runs`: a run's own page as tafs cloud answers `GET /v1/tafs/{id}/requests/{run}`; Try the reply now sends nothing |
| `--receipt answer.json` | with `--home`: a receipt as KIFF answers `GET /v1/receipts/{trace_id}`; nothing is sent |
| `--coach answer.json` | with `--home`: Coach's conversation as tafs cloud answers `GET /v1/tafs/{id}/conversation/{session}`; nothing is read or sent ([Coach](coach.md)) |
| `--chat answer.json` | with `--home`: a taf's chat as tafs cloud answers `GET /v1/tafs/{id}/conversation`, on its page's Chat tab; nothing is read or sent ([A taf's chat](chat.md)). With `--report`, Report this answer opens over its last answer |
| `--brief answer.json` | with `--home`: "Your brief" as tafs cloud answers `GET /v1/brief`, in Needs you's place; every read is it, a label's too, and Later sends nothing ([Your brief](brief.md)) |
| `--mission-log answer.json` | with `--brief`: a mission's log as tafs cloud answers `GET /v1/missions/{id}/log`, for every mission's Evidence |
| `--connect answers.json` | with `--home`: Connect apps' first page as tafs cloud answers `GET /v1/connect/apps` (`answer`); nothing is sent ([Connect apps](connect-apps.md)) |
| `--knob answer.json` | with `--home`: a taf's knob as tafs cloud answers `GET /v1/tafs/{id}/knob`, for Starts when, Results go to and the dial; nothing is sent ([A taf's page](taf.md)) |
| `--setup answer.json` | with `--home`: a taf's Setup as tafs cloud answers `GET /v1/tafs/{id}/setup`, in place of the contract's `taf-setup.json` ([A taf's page](taf.md)) |
| `--taf-activity answer.json` | with `--home`: a taf's Activity as tafs cloud answers `GET /v1/tafs/{id}/activity` ([A taf's page](taf.md)) |
| `--app-event item.json` | a dial's stop with its approval, as tafs cloud sends it in the knob's `wakes`: its filters' sheet opens over the window, signed in; its Approve is the gate's line, so nothing is signed or subscribed |
| `--paired` | with `--view-only`: a computer already added with the phone, whose notice says approvals go to the phone |
| `--page …` | `welcome`, `sign-in`, `pair` (adding this computer with a phone), `recipe:<id>` (with `--home` and `--recipes`: that recipe's sheet), `gallery` (with `--home` and `--gallery`: the gallery), `new-taf` (the gallery as + New opens it), `coach-hand:<recipe id>` (with `--recipes` and `--coach`: Coach with that recipe's words in its box), `halt-all` or `halt:<id>` (a halt's screen), `record-search:<q>` (the Record with that search), `record-export` (the Record with the receipts' days open), `receipt:<proposal id>` (with `--activity` and `--receipt`: that line's receipt), `run:<taf id>` (with `--runs` and `--request`: that taf's first kept run's own page), `brief-answer:<decision id>` (with `--brief`: Your brief with that decision's signed answer tapped), `overview-label:<id>` (the Overview filtered to Home's label by its id), `mission-log:<mission id>` (with `--brief` and `--mission-log`: that mission's log over the brief), `delete-account` (with `--people`: the KIFF account's Delete account sheet), `pin-sheet` (the security key's PIN sheet, with canned answers; no key is asked), or a screen beside the sidebar: `needs-you`, `share:<key>` or `share-step2:<key>` (with `--people`'s `grants`: Share's sheet over that taf's page, or its second step with Share tapped), `record`, `calendar`, `usage`, `connect-apps` (and `connect-app`, `connect-connection`, `connect-remove` over the sample; `connect-which-taf`, `connect-approve` with `--connect`), `account`, `alerts`, `taf` (the sample's Refunds), `taf:<id>` (a taf by its key), or `coach` (with `--coach`: Coach's conversation), `coach-tools` (Coach mode with its tools step open), `coach-full` (Coach mode full screen) |
| `--screenshot out.png` | save the window (1280×800, at 2×) as a PNG and quit. It never reads or writes the keyring, and its sign-in and pairing show a code without reaching KIFF |

CI builds the app and saves the welcome, the sign-in, the pairing, the Overview beside the sidebar (with the
account's items), the Overview from the `home.json` contract, Needs you (and with synthetic steps and Coach's waits, and with "Did this happen?", and a held call from KIFF, and Set all aside's ask and a Card unit request's Dismiss tapped, and a hold's notification clicked), Resume tapped on a halted taf, Discard tapped on a taf being set up, the Record, Refunds'
page and its Card tab, Plan & usage and Account over the sample, Plan & usage signed in with
KIFF from synthetic answers (managing billing, and as a member), Connect apps, Devices, People & groups, Share on a taf's page and its two steps, the KIFF account and its time zone picker (from synthetic answers), Alerts, the
calendar's Day and Week (and Day at an upright iPad's size, `--size 1032x1376`), a tafs cloud
taf's page and its Card tab from the contracts (and with its Card's own lines) (and with its work and runs, and a run's own page), the sidebar over the plain sample (and with an agent not run by tafs, with a taf being set up, with the plan's line, and with the Unreachable line), and Home with no taf and a recipe's sheet, the strip and the gallery (and as + New opens it, and Coach with a job handed to it), the halt screens, a Record search, the receipts export open, a Record line's receipt, the Delete account sheet, the security key's PIN sheet,
an app event's filters before signing, Coach's conversation from the contract (in Coach mode over the Overview, with its tools step open, full screen, and beside what it has made so far; and with an approval waiting under tafs cloud's wider line, with a confirm and a questions card waiting and a message not sent (landing on the card, and read back to its top with ↓), with Previous conversations open, empty, before it's started, and on a taf to improve), a taf's chat (and while it works, with Report this answer open, and with files in its tray, as in Coach's), a taf's versions (and with a trial open, and the trial itself), Your brief (and with a signed answer tapped, with nothing to decide, and a mission's log), the Overview with Home's label chips (and filtered to one), the
view-only notice (and once paired), and the boards' empty states (the Overview with no taf, and before its tafs could be read, Needs you with nothing waiting, Halt all once halted, the brief with nothing to decide) in en, fr and pt, light and dark: 726 pictures, as the `screenshots`
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
  every bundled library and starts nothing), and starts it on a clean Ubuntu 24.04 (below). The
  file isn't kept: the repository is public, and a run's artifact would be a copy anyone signed in
  to GitHub could download.
- Run by hand with a version (`v` and three numbers, as `v0.1.0`: anything else is refused before
  a tag is made), one job builds it, checks it starts, starts it on a clean Ubuntu 24.04, and only
  then attaches it with its `SHA256SUMS` to a **draft** release, its only copy. Publishing it is
  the owner's yes, never the workflow's.
- Before a draft is published, `.github/workflows/verify-release.yml`, run by hand with its tag,
  downloads the draft's own AppImage and `SHA256SUMS`, checks one against the other, and starts
  that file on a clean Ubuntu 24.04. Its three screenshots are the run's only artifact. Nothing
  is created, edited or published. It takes `tools/check-appimage.sh` from the branch it's run
  on (main), so a draft is checked with today's check, not the one it was built with.

**On a clean Ubuntu 24.04** (`tools/check-appimage.sh`, in an `ubuntu:24.04` container): the
container has what every Ubuntu 24.04 desktop has and the AppImage counts on, since linuxdeploy
leaves those base libraries out on purpose (fribidi, HarfBuzz, fontconfig, FreeType, GL/EGL/GL ES, X11,
xkbcommon, the Wayland client, GL ES), plus a virtual display (`xvfb`) and one font. It never has GTK,
libadwaita or a MIME database (`shared-mime-info`): the check fails if one is installed. It then
fails on:
- a library the app finds in neither the AppImage nor that base (a bare container without the
  base stops at `libfribidi.so.0`);
- an image loader that can't find its own libraries, or a `loaders.cache` line naming one that
  isn't in the AppImage. GTK opens the gdk-pixbuf loaders at run time, with no
  `LD_LIBRARY_PATH`, and the SVG one draws every icon. The GTK plugin copies them from the build
  system with no RUNPATH, so `tools/appimage.sh` points each at the AppImage's `usr/lib`
  (`patchelf`) before the AppImage is made: the SVG loader never depends on the system's librsvg;
- an icon GTK can't load, in any of the three screens it saves: the first screen, the sign-in,
  and the sample signed in, whose sidebar draws the app's own icons. gdk-pixbuf tells an image's
  format by the MIME database (through GIO) before it chooses a loader, so the AppImage brings
  its own compiled `usr/share/mime/mime.cache`. Without it, the v1.0.0 draft drew every SVG icon
  as a missing-image placeholder ("Unrecognized image file format");
- a GL library GTK's own renderer can't open: the first screen is drawn again with GTK's default
  renderer, not the software one the others use. The desktop base includes GL ES (`libgles2`),
  as a desktop has: linuxdeploy leaves the GL libraries out, since they're the GPU driver's. On
  a system without it (a server image, as WSL's Ubuntu), GTK said "Couldn't open
  libGLESv2.so.2" and no window opened.

**Signing.** The draft job signs `SHA256SUMS` with tafs for Linux's OpenPGP key
(`tools/sign.sh`): an armored detached signature, `SHA256SUMS.asc`, attached beside it. Whoever
downloads checks the signature with the public key, then the AppImage against `SHA256SUMS`:

```
gpg --verify SHA256SUMS.asc SHA256SUMS && sha256sum -c SHA256SUMS
```

- The key is two secrets in the `linux-signing` environment, `TAFS_SIGNING_KEY` (armored) and
  `TAFS_SIGNING_PASSPHRASE`. Only the signing step reads them. They're imported into a keyring
  made for the step and removed after it, from stdin, never a file or the command line.
- **While the environment holds no key, the draft goes out unsigned** and the step says so.
  Adding the key is a value in the environment, not a change here; its public half then goes in
  `packaging/` and on the download page.
- A pull request that changes the packaging runs the same step with a throwaway key made in the
  job, so the signing is known to work before the real key exists.
