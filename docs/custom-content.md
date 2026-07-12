# Bundled and custom languages, quotes, themes, and funboxes

[Documentation index](README.md) · [Configuration](configuration.md) · [Data paths](data-and-privacy.md)

## Bundled language lists

All 13 lists below are embedded in the executable and work without a network
connection:

| Name | Purpose |
| --- | --- |
| `english` | Base frequency-ordered English list (200 words) |
| `english_1k` | 1,000-word English list |
| `english_5k` | 5,000-word English list |
| `english_10k` | 10,000-word English list |
| `english_25k` | Extended English list with more than 20,000 entries |
| `english_450k` | Very large English list with more than 400,000 entries |
| `english_commonly_misspelled` | Common spelling traps |
| `english_contractions` | English contractions |
| `english_doubleletter` | Double-letter words |
| `english_legal` | Legal vocabulary |
| `english_medical` | Medical vocabulary |
| `english_old` | Older English vocabulary |
| `english_shakespearean` | Shakespearean vocabulary |

Large embedded JSON lists are parsed once per process and cached. Select a list
under Test → Language or temporarily:

```sh
mtype --language english_10k --mode words --words 50
```

## Language JSON format

Synced language files use the Monkeytype language shape:

```json
{
  "name": "example",
  "noLazyMode": false,
  "orderedByFrequency": true,
  "rightToLeft": false,
  "words": ["alpha", "beta", "gamma"]
}
```

| Field | Required | Use in mtype |
| --- | --- | --- |
| `name` | Yes | Descriptive metadata |
| `noLazyMode` | No, defaults false | Parsed metadata; lazy comparison is controlled by config |
| `orderedByFrequency` | No, defaults false | Parsed metadata; Zipf funbox relies on list order being meaningful |
| `rightToLeft` | No, defaults false | Parsed metadata; terminal RTL layout is not specially implemented |
| `words` | Yes | Word pool |

The filename, not the internal name, is used for selection:

```text
<data directory>/languages/spanish.json
```

is selected with `--language spanish`.

## Sync languages

```sh
mtype sync language spanish
mtype sync language code_rust
```

Sync downloads a named JSON file from the Monkeytype repository into
`languages/`. It is the only step requiring network access; generation uses the
local copy afterward.

The downloader requests:

```text
https://raw.githubusercontent.com/monkeytypegame/monkeytype/master/frontend/static/languages/<name>.json
```

It has a 20-second timeout and validates the response as a language before
writing. Names highlighted by the CLI error/help path are:

- `english_1k`, `english_5k`, `english_10k`;
- `spanish`, `french`, `german`, `italian`, `portuguese`, `dutch`, `russian`;
- `code_python`, `code_javascript`, `code_rust`, `code_c`.

If a CLI-requested language is unavailable, mtype prints a note and falls back
to `english`. The config language list includes embedded names and valid local
filenames discovered in the data directory.

## Quotes

English quotes are embedded. Additional quote collections can be synced:

```sh
mtype sync quotes french
```

Quote downloads use:

```text
https://raw.githubusercontent.com/monkeytypegame/monkeytype/master/frontend/static/quotes/<language>.json
```

The response is validated as a quote collection before it replaces the local
file. `sync` accepts `language`, `languages`, or `lang` for word lists and
`quotes` or `quote` for quote collections.

Quote JSON shape:

```json
{
  "language": "example",
  "groups": [[0, 100], [101, 300], [301, 600], [601, 10000]],
  "quotes": [
    {
      "text": "A complete example quote.",
      "source": "Example source",
      "length": 25,
      "id": 1
    }
  ]
}
```

| Field | Meaning |
| --- | --- |
| `language` | Collection metadata |
| `groups` | Inclusive `[minimum, maximum]` length ranges for short, medium, long, thicc |
| `quotes[].text` | Ordered target text |
| `quotes[].source` | Optional display source; missing values default empty |
| `quotes[].length` | Numeric value used for band selection |
| `quotes[].id` | ID used by `--quote-id` and repeat quotes |

Quote text normalization replaces Unicode ellipsis with three periods, converts
newlines to spaces, collapses repeated spaces, and trims the result before
splitting on spaces.

Sized word-list suffixes matching `_<digits>k` are removed when resolving quote
language, so `english_1k` uses `english` quotes. If no requested collection is
available, mtype falls back to embedded English quotes.

## Bundled terminal themes

| Theme | General palette |
| --- | --- |
| `serika_dark` | Monkeytype dark charcoal and yellow |
| `dracula` | Dark purple |
| `nord` | Cool blue-gray |
| `catppuccin` | Mauve dark palette |
| `gruvbox_dark` | Warm retro dark palette |
| `tokyo_night` | Dark blue palette |
| `rose_pine` | Muted rose dark palette |
| `solarized_dark` | Solarized dark palette |

Select under Appearance → Theme. Theme changes apply immediately without
restarting the current test.

## Custom theme format

Create:

```text
<data directory>/themes/my_theme.toml
```

Complete example:

```toml
bg = "#202124"
main = "#e2b714"
sub = "#646669"
sub_alt = "#191a1c"
text = "#d1d0c5"
error = "#ca4754"
caret = "#e2b714"
error_extra = "#7e2a33"
colorful_error = "#ca4754"
colorful_error_extra = "#7e2a33"
```

### Required fields

| Field | Role |
| --- | --- |
| `bg` | Main terminal background |
| `main` | Accent, active selections, net graph, headline metrics |
| `sub` | Muted text, inactive labels, raw graph |
| `sub_alt` | Config/dialog surface background |
| `text` | Main target/result text |
| `error` | Incorrect and missed characters |

### Optional fields and fallback

| Field | Fallback |
| --- | --- |
| `caret` | `main` |
| `error_extra` | `error` |
| `colorful_error` | `error` |
| `colorful_error_extra` | `error_extra` when present, otherwise `error` |

Colors accept `#rrggbb`; the parser also accepts short `#rgb` and ignores an
alpha pair in an eight-digit string. An invalid color becomes terminal reset.
An unreadable or invalid theme file causes the named theme to fall back to
`serika_dark`.

The theme name is the filename stem. Custom names are discovered at config-open
time, merged with bundled names, deduplicated, and sorted.

## Funboxes

mtype supports 20 terminal-feasible Monkeytype funboxes. They fall into word
generators, text transforms, and behavior changes.

### Generator funboxes

The first active generator funbox replaces the dictionary source. Later active
transform funboxes still process the generated value.

| Name | Generated target |
| --- | --- |
| `58008` | Random 1–7 digit number, first digit nonzero |
| `gibberish` | Random 1–7 lowercase ASCII letters |
| `ascii` | Random 1–10 printable ASCII characters from code points 33–126 |
| `specials` | Random 1–7 punctuation/symbol characters |
| `binary` | Exactly eight bits for a random value 0–255 |
| `hexadecimal` | `0x` followed by 1–4 random two-digit hex bytes |
| `IPv4` | Four random decimal octets separated by periods |
| `IPv6` | Eight random hexadecimal 16-bit groups separated by colons |

### Transform funboxes

Transforms apply in the order stored in config.

| Name | Effect |
| --- | --- |
| `capitals` | Capitalize the first character |
| `rAnDoMcAsE` | Independently choose upper/lower case for each character |
| `sPoNgEcAsE` | Alternate lower and upper case by character index |
| `ALL_CAPS` | Convert the word to uppercase |
| `rot13` | Apply ROT13 to ASCII letters |
| `backwards` | Reverse Unicode character order |
| `ddoouubblleedd` | Duplicate every character |
| `instant_messaging` | Lowercase and remove parentheses, periods, quotes, exclamation, and question marks |
| `underscore_spaces` | Append `_` to every nonfinal generated word |
| `morse` | Convert supported letters/digits to Morse codes separated by `/` |

### Behavior funboxes

| Name | Effect |
| --- | --- |
| `zipf` | Select frequency-ordered pool indexes with a Zipf-like distribution instead of uniform random |
| `no_quit` | Block Tab restart and Esc leaving/restart while a test is running |

Unknown funbox names in manually edited config are ignored. Clear all removes
every stored funbox. Generator-only pool logic such as punctuation and number
injection is skipped when a generator funbox supplies the base word, but active
text transforms still run.

Purely visual/audio or deeper-engine Monkeytype funboxes are intentionally not
implemented, including mirror, nausea, TTS, no-space, read-ahead, memory,
plus-one, weakspot, pseudolang, and polyglot.

## Content fallback and validation

- Empty language pools cannot generate normal random targets.
- Practice with no analytics falls back to the selected language pool.
- Missing CLI language names warn and fall back to base English.
- Missing quote collections fall back to embedded English quotes.
- Invalid local JSON fails to load as that content; keep valid upstream shapes.
- Stop and restart mtype after changing files so discovery and parse caches are
  refreshed.
