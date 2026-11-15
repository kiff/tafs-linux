# Words and contracts

Both come from kiff/tafs at the commit in `shared-pin.json`. `tools/sync.sh` puts them in
`shared/` (`shared/app/strings`, `shared/contracts`), never in git, with one of the iPad's own
samples: `ios/Tafs/Samples/connect-apps.json`, the reference set's Connect apps.

`WordBook::load(root, language)` reads `app/strings/<language>/*.json`, the files the iPad's
`Words.swift` is generated from:

- `get(key, values)` fills `{name}` and `{name|possessive}` (English and German have a rule; the
  others keep the name).
- `count(key, n, values)` picks the form the language gives `n` (Polish: one, few, many) and fills
  `{count}`.
- A missing key, or a value a line asks for and isn't given, is an error: no English fallback, no
  `{hole}` on screen.

`tests/at_the_pin.rs` checks every line the ported screens read, in all 8 languages, with the
values English has. A new key goes into kiff/tafs first, then the pin moves, in its own PR.
