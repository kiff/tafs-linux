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
| `--page …` | `welcome`, `sign-in`, or a screen beside the sidebar: `needs-you`, `record`, `calendar`, `usage`, `connect-apps`, `account`, `taf` (the sample's Refunds), or `taf:<id>` (a taf by its key) |
| `--screenshot out.png` | save the window (1280×800, at 2×) as a PNG and quit. It never reads or writes the keyring, and its sign-in shows a code without reaching KIFF |

CI builds the app and saves the welcome, the sign-in, the Overview beside the sidebar (with the
account's items), the Overview from the `home.json` contract, Needs you, the Record, Refunds'
page and its Card tab, Plan & usage and Account over the sample, Plan & usage signed in with
KIFF from synthetic answers (managing billing, and as a member), Connect apps, Devices, a tafs cloud taf's page and its Card tab from
the contracts, the sidebar over the plain sample, and the view-only notice in en, fr and pt,
light and dark: 108 pictures, as the `screenshots`
artifact of the run. The job fails if any is missing.
