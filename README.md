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
| `shared-pin.json` | The commit of the shared repository the words and contracts are read at. |
| `docs/` | One page per area. The [port checklist](docs/port-checklist.md) says where each iPad screen stands. |

## Build and run

Needs Rust (stable), GTK 4.14 or later and libadwaita 1.5 or later (on Ubuntu 24.04:
`sudo apt install libgtk-4-dev libadwaita-1-dev`), and git.

```
tools/sync.sh                   # the words at the pin, into shared/
cargo test -p tafs-core         # the core's tests
cargo run -p tafs               # the app
cargo run -p tafs -- --sample   # over the iPad's sample
```
