# Words and contracts

Both come from kiff/tafs at the commit in `shared-pin.json`. `tools/sync.sh` puts them in
`shared/` (`shared/app/strings`, `shared/contracts`), never in git, with one of the iPad's own
samples: `ios/Tafs/Samples/connect-apps.json`, the reference set's Connect apps.

`WordBook::load(root, language)` reads `app/strings/<language>/*.json`, the files the iPad's
`Words.swift` is generated from:

- `get(key, values)` fills `{name}` and `{name|possessive}` (English and German have a rule; the
  others keep the name), and `{name|this}`, or `{name|This}` to open a sentence: French "ce PC",
  "cet appareil" (`THIS` in kiff/tafs's `generate.py`; the others keep the name). A form it
  doesn't know is a missing value.
- `count(key, n, values)` picks the form the language gives `n` (Polish: one, few, many) and fills
  `{count}`.
- A missing key, or a value a line asks for and isn't given, is an error: no English fallback, no
  `{hole}` on screen.

`tests/at_the_pin.rs` checks every line the ported screens read, in all 40 languages, with the
values English has. It also fills every line at the pin, in every language, and fails if a `{`
is left, so a form this app doesn't read yet fails CI before the pin moves. A new key goes into kiff/tafs first, then the pin moves, in its own PR.

## Languages

The app speaks every language tafs ships (`languages.json`'s `shipped`: 40, with pt-BR,
zh-Hant and sr-Latn; Arabic and Hebrew right to left), `words::SHIPPED`. Greek waits until tafs
ships it.

- **A variant** (`variants`: pt-BR over pt) reads as its base's lines with its own on top, as
  `generate.py` builds it (`WordBook::load`).
- **Which one:** the first of the person's languages (`LANGUAGE`, `LC_ALL`, `LC_MESSAGES`, `LANG`)
  the app speaks, by its words' folder: `pt_BR` is pt-BR, `zh_TW` or `zh-Hant` is zh-Hant,
  Serbian only in Latin script (`sr_RS@latin`, `sr-Latn`), Simplified Chinese not at all
  (`language::words_language`). KIFF and kiff.dev know it by its
  code, pt and zh (`language::kiff_code`, as the iPad's `screenLanguages`).
- **Dates, weekdays, relative times, "and" lists, plural forms and numbers** come from CLDR,
  through ICU4X compiled into the app (`tafs-core/src/locale.rs`): the data the iPad's formatters
  read. Arabic writes Latin digits and the Gregorian calendar (`IOS_LOCALE`'s `nu-latn`); Thai
  counts years in the Buddhist era, and Nepali and Bengali write their own digits, as CLDR does. A plural form is kept to those
  `generate.py`'s `PLURALS` gives the language (Vietnamese has only `other`).
- **Ordinals and currency symbols** (`held::ordinal`, `money::symbol`) are ICU's
  digits-ordinal rules and standard symbols, written out: ICU4X has neither.
- **Terms and Privacy** open in the 38 languages kiff.dev serves them in, Traditional Chinese
  under `/zh/` and pt-BR under `/pt/` (`Legal.translated`); any other opens the English page
  and says so.

## Right to left

Arabic sets GTK's default direction to right to left before a widget is made, so every box,
list and row is mirrored, and the arrows take Adwaita's `-rtl` icons, as `chevron.forward` does on
the iPad. The dials and the drawn boards keep their screen angles (kiff/tafs
`docs/languages.md`, "Direction").

- **Each value** filled into a right-to-left line is put in a first-strong isolate (U+2068 …
  U+2069) with any direction character of its own taken out (`words::isolated`, the iPad's
  `Words.isolated`), so a name in Latin letters takes its own direction without turning the
  sentence around.
- **Captions** set monospaced and spaced out (the eyebrows, the Record's days, a Card's
  environment, the dial's kicker) take the plain face with no spacing when written in Hangul,
  kana, Han or Arabic (`words::ideographic`, `Eyebrow.ideographic`).
