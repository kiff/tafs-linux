# tafs for Linux

The tafs desktop app for Linux: the same product, account and words as the iPad app, ported from
it screen by screen. Native: Rust · GTK 4 · libadwaita.

It draws what KIFF and tafs cloud send. Nothing in this first version runs actions on the computer.

## Layout

| Folder | What |
| :- | :- |
| `crates/tafs-core` | Portable logic: words, KIFF's sign-in, the kept session, the device key, the theme, the sidebar. No UI. |
| `crates/tafs-app` | The GTK app (`tafs`). It places what the core says. |
| `tools/sync.sh` | The words and contracts at the pin, into `shared/`. |
| `tools/screenshots.sh` | Every screen built so far in en, fr and pt, light and dark. |
| `tools/pin-check.sh` | The chat pin in the window, in a taf's chat and in Coach (the iPad's `ChatPinUITests`), under a display. |
| `shared-pin.json` | The commit of the shared repository the words and contracts are read at. |
| `docs/` | One page per area. The [port checklist](docs/port-checklist.md) says where each iPad screen stands. |

## Install

Download `tafs-vX.Y.Z-x86_64.AppImage`, `SHA256SUMS` and `SHA256SUMS.asc` from the
[releases](https://github.com/kiff/tafs-linux/releases) page, into one folder. Runs on Ubuntu 24.04
and newer, x86_64.

Check them before running anything. `SHA256SUMS` is signed with tafs for Linux's release key,
[`packaging/tafs-release-key.asc`](packaging/tafs-release-key.asc):

- **Key:** OpenPGP ed25519, signing only, expires 2028-11-16.
  UID `tafs for Linux (release signing) <support@kiff.dev>`.
- **Fingerprint:** `73BC 9445 0F18 1366 FE44  3DCB 2845 162C 93DB BCA1`

```
gpg --import tafs-release-key.asc
gpg --fingerprint support@kiff.dev                      # must show the fingerprint above
gpg --verify SHA256SUMS.asc SHA256SUMS                   # "Good signature"
sha256sum -c SHA256SUMS                                  # the AppImage: OK
chmod +x tafs-v*-x86_64.AppImage && ./tafs-v*-x86_64.AppImage
```

`gpg` says the key isn't certified with a trusted signature unless you've signed it yourself:
that's expected. What counts is the fingerprint, compared with the one here.

## Build and run

Needs Rust (stable), GTK 4.14 or later and libadwaita 1.5 or later (on Ubuntu 24.04:
`sudo apt install libgtk-4-dev libadwaita-1-dev`), and git.

```
tools/sync.sh                   # the words at the pin, into shared/
cargo test -p tafs-core         # the core's tests
cargo run -p tafs               # the app
cargo run -p tafs -- --sample   # over the iPad's sample
```
