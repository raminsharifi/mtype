# mtype

A fast, offline typing test for the terminal, inspired by
[Monkeytype](https://github.com/monkeytypegame/monkeytype). It includes local
history, adaptive practice, replay, a browser-based growth dashboard, and no
account system.

```text
  30

  the quick brown fox jumps over the lazy dog and then keeps
  going while the timer counts down and your wpm and accuracy
  update as you type each word in the test

  tab restart    esc config    ctrl+c quit
```

## Highlights

- Time, words, quote, zen, custom-text, and adaptive-practice modes
- Live WPM and accuracy with detailed results and animated replay
- Local personal bests, streaks, activity, and word-level analytics
- Wrong-word and slow-word practice backed by local SQLite data
- Animated private dashboard through `mtype stats serve`
- Lazy.nvim-inspired tabbed config workspace
- All Monkeytype English word lists bundled for offline use
- Themes, funboxes, presets, portable backup, and custom content

## Install

Download the latest binary from [GitHub Releases](https://github.com/raminsharifi/mtype/releases/latest),
or install from source:

```sh
cargo install --git https://github.com/raminsharifi/mtype
```

Prebuilt binaries are available for macOS Apple Silicon and Linux x86_64. See
the [installation guide](docs/installation.md) for platform-specific steps.

## Quick start

```sh
mtype
mtype --time 60 --punctuation
mtype --mode words --words 50
mtype practice missed --words 25
mtype stats serve
```

Press Esc during a test to open config. Use Tab to change sections, Up and Down
to move, Enter to open or apply a setting, and Esc to close. Config changes stay
open so you can adjust several settings in one visit.

## Documentation

- [Installation](docs/installation.md)
- [Complete CLI reference](docs/cli-reference.md)
- [Tests, modes, and keyboard controls](docs/usage.md)
- [Configuration workspace](docs/configuration.md)
- [Scoring, metrics, and results](docs/scoring-and-results.md)
- [Stats, dashboard, and adaptive practice](docs/stats-and-practice.md)
- [Local data, privacy, backup, and reset](docs/data-and-privacy.md)
- [Languages, quotes, and themes](docs/custom-content.md)
- [Troubleshooting](docs/troubleshooting.md)
- [Development](docs/development.md)

Start with the [documentation index](docs/README.md) for a guided path.

## Offline by default

Tests, saved results, analytics, practice, config, and the browser dashboard all
run locally. `mtype sync` is the only optional network feature; it downloads
extra content for later offline use.

## Credits and license

mtype is derived from Monkeytype and is licensed under GPL-3.0-or-later. See
[LICENSE](LICENSE) and [NOTICE.md](NOTICE.md). mtype is not affiliated with,
endorsed by, or sponsored by Monkeytype.
